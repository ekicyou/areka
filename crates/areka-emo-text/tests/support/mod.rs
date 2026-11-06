//! # support/mod.rs — `tests/` の実 GPU・presenter・読み戻しの共有の部品
//!
//! 各テストは `#[path = "support/mod.rs"] mod support;` で引く（`tests/` の直下に置くと
//! テストの実行ファイルになるので、子のフォルダに置く）。中身:
//!
//! 1. **実 GPU の World と予約スロット**——`GraphicsCore`＋`WucGraphicsResource` を載せた
//!    headless の World（本番の UI スレッドと同じ MTA）と、emo-present `VisualMount` と同型の
//!    予約スロット。
//! 2. **presenter の検体**——surface 1000 ＝ 絵 1 枚の `(EmoWorld, AtlasTable)` と `ShowSurface`。
//! 3. **指令とポンプ**——間隔つきの cue と、積んだメッセージを処理し切って抜ける pump。
//! 4. **読み戻しと画素の数え上げ**——供給面の読み戻し・不透明の数・矩形の中の数え上げ・
//!    選択肢の強調の帯の確かめ・白背景へ合成した PNG の書き出し。

// テストの実行ファイルごとに使う部品が違い、使わない部品が dead_code になるため。
#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use areka_actor::reply_channel;
use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::{EmoPresenter, PresentCommand, PresentOutcome, TargetId};
use areka_emo_text::actor::{ChoiceHitRow, TextLayerRuntime};
use areka_emo_text::place::PlaceKey;
use areka_parsers::shell::{AppendTarget, DefRef, Element, ElementPath, Shell, Surface};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};
use wintf_winmsg_executor::{FilterResult, MessageLoop};

// ══ 実 GPU の World と予約スロット ═══════════════════════════════════════════════════════

/// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World を組む。
/// 本番 UI スレッドは MTA（記憶: areka WUC は MTA スレッドで動く）。
pub fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視——テストスレッド毎）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// emo-present `VisualMount` と同型の予約スロット。返り値は (window, slot)。
pub fn spawn_reserved_slot(world: &mut World) -> (Entity, Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-layer-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();
    (window, slot)
}

// ══ presenter の検体（上流の公開 API で本物を組む） ═══════════════════════════════════════

pub fn elem(path: &str, x: i64, y: i64) -> Element {
    Element {
        layer: 0,
        path: ElementPath::new(path.to_string()),
        x,
        y,
    }
}

pub fn shell_of(surfaces: Vec<Surface>) -> Shell {
    let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
    Shell {
        surfaces,
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions,
    }
}

/// 座標由来のグラデーション（全不透明・`salt` で target ごとに画素を変える）の BGRA 密配列。
pub fn gradient_image(w: u32, h: u32, salt: u8) -> Vec<u8> {
    let mut img: Vec<u8> = Vec::with_capacity((w * 4 * h) as usize);
    for y in 0..h {
        for x in 0..w {
            img.push((x as u8).wrapping_mul(3).wrapping_add(salt));
            img.push((y as u8).wrapping_mul(5).wrapping_add(salt));
            img.push(((x + y) as u8).wrapping_mul(7).wrapping_add(salt));
            img.push(0xFF);
        }
    }
    img
}

/// surface 1000 ＝ 単一 element（`w×h` の絵 `img`）の `(EmoWorld, AtlasTable)`
/// （バルーン枠やシェルの画像に相当する合成入力）。
pub fn build_assets(w: u32, h: u32, img: Vec<u8>) -> (EmoWorld, AtlasTable) {
    let base = Path::new("shell/master");
    let surfaces = vec![Surface {
        targets: vec![AppendTarget::Single(1000)],
        elements: vec![elem("p.png", 0, 0)],
        collisions: Vec::new(),
        animations: Vec::new(),
    }];
    let mut dec = MemoryDecoder::new();
    dec.insert(base.join("p.png"), w, h, w * 4, img, true);
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(
        baked.errors.is_empty(),
        "atlas bake セットアップは失敗しない"
    );
    let mut world = EmoWorld::build(&shell_of(surfaces));
    world.bind_atlas(&baked.table, SetId(0));
    (world, baked.table)
}

/// [`build_assets`] の絵を [`gradient_image`] にしたもの。
pub fn build_target_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
    build_assets(w, h, gradient_image(w, h, salt))
}

/// `ShowSurface`（surface 1000）を適用して表示を確立する（reply Ok を要求）。
pub fn show_surface(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target,
            surface_id: 1000,
            binds: BindSet::default(),
            pattern: areka_emo_compose::PatternState::default(),
            reply: Some(tx),
        },
    );
    let outcome = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("reply（ShowSurface）を受信できない");
    assert!(matches!(outcome, Ok(())), "ShowSurface は Ok: {outcome:?}");
}

// ══ 指令とポンプ ═════════════════════════════════════════════════════════════════════════

/// テスト用 cue。reveal は配送 duration 由来（`interval = duration / N`）ゆえ、Text cue へ
/// `N × interval` を焼き込むと per-glyph の間隔 `interval` を得る。他のコマンドは瞬時
/// （duration=0）。`interval = 0.0` なら全グリフが `at` で即時に見える。
pub fn cue(actor: &str, at: f64, command: CueCommand, interval: f64) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => t.chars().count() as f64 * interval,
        _ => 0.0,
    };
    TalkCue {
        at,
        actor: ActorKey::from(actor),
        command,
        duration,
    }
}

/// 事前 queue 済みメッセージを全て処理し queue が空になった時点で抜ける bounded pump
/// （`WM_QUIT` は posted メッセージが尽きた後にのみ配送されるため、drain タスクは queue 済みの
/// cue を全て処理してから復帰する。決定論・ハングしない）。
pub fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

// ══ 読み戻しと画素の数え上げ ═══════════════════════════════════════════════════════════

/// 装着済み actor の供給面の読み戻し（premultiplied BGRA 密配列）。
pub fn read_back(rt: &TextLayerRuntime, actor: &ActorKey) -> Vec<u8> {
    rt.surface(actor)
        .expect("装着済み actor の供給面")
        .read_back()
        .expect("read_back")
}

/// 装着済み actor の供給面の寸。
pub fn surface_size(rt: &TextLayerRuntime, actor: &ActorKey) -> (u32, u32) {
    rt.surface(actor).expect("装着済み actor の供給面").size()
}

/// 提示済みの場所（箱など）の面の読み戻し。
pub fn read_back_at(rt: &TextLayerRuntime, place: &PlaceKey) -> Vec<u8> {
    rt.surface_at(place)
        .expect("提示済みの場所の面")
        .read_back()
        .expect("read_back")
}

/// 非透明ピクセル数（BGRA 密配列の α ≠ 0）。
pub fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 矩形 `(left, top, right, bottom)` の中で述語に合う画素を数える（矩形は面の内へ切り詰める）。
pub fn count_in_rect(
    bytes: &[u8],
    width: u32,
    height: u32,
    rect: (f32, f32, f32, f32),
    pred: impl Fn(&[u8]) -> bool,
) -> usize {
    let x0 = rect.0.floor().max(0.0) as u32;
    let x1 = (rect.2.ceil().max(0.0) as u32).min(width);
    let y0 = rect.1.floor().max(0.0) as u32;
    let y1 = (rect.3.ceil().max(0.0) as u32).min(height);
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if pred(&bytes[i..i + 4]) {
                n += 1;
            }
        }
    }
    n
}

/// 指定 x 帯 `[x0, x1)` × y 帯 `[y0, y1)` で述語に合う画素が現れた **y の最小/最大**（無ければ `None`）。
pub fn y_span_where(
    bytes: &[u8],
    width: u32,
    x0: u32,
    x1: u32,
    y0: u32,
    y1: u32,
    pred: impl Fn(&[u8]) -> bool,
) -> Option<(u32, u32)> {
    let mut span: Option<(u32, u32)> = None;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if pred(&bytes[i..i + 4]) {
                span = Some(match span {
                    None => (y, y),
                    Some((lo, _)) => (lo, y),
                });
                break;
            }
        }
    }
    span
}

// ══ 選択肢の強調の帯 ═══════════════════════════════════════════════════════════════════

/// SquareFill の塗り帯そのもの（`cursor.brush.color` (105,25,25)・premultiplied α=255 ゆえ
/// BGRA=(25,25,105,255)）。
pub fn is_fill(px: &[u8]) -> bool {
    px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255
}

/// hover 行の**文字だけ**を選ぶ述語（塗り帯を文字と数えないための切り分け）。
///
/// α だけでインクを拾うと不透明な塗り帯の画素にも当たり、返る縦範囲は帯の縦範囲そのものになる
/// ——字が 1 画素も無くても「インクは帯の内」が成り立ってしまう。hover 文字は白
/// （`cursor.font.color` 255,255,255）で、premultiplied では帯の外なら B ＝ G ＝ R ＝ α、帯の上に
/// 載った字は B ＝ 25 ＋ 230 × 被覆率 になる。塗り帯 (105,25,25) の青チャネルは 25 止まり
/// （縁が半透明なら更に小さい）なので、「青チャネル 128 以上」で白文字だけを拾える。
pub fn is_hover_text(px: &[u8]) -> bool {
    px[3] >= 128 && px[0] >= 128
}

/// SquareFill 塗り色を矩形内で数える。
pub fn fill_pixels_in_rect(
    bytes: &[u8],
    width: u32,
    height: u32,
    rect: (f32, f32, f32, f32),
) -> usize {
    count_in_rect(bytes, width, height, rect, is_fill)
}

/// 白文字（≈255,255,255・全チャネル閾値で AA 端を除いた芯）を矩形内で数える。
pub fn white_pixels_in_rect(
    bytes: &[u8],
    width: u32,
    height: u32,
    rect: (f32, f32, f32, f32),
) -> usize {
    count_in_rect(bytes, width, height, rect, |px| {
        px[0] >= 200 && px[1] >= 200 && px[2] >= 200 && px[3] == 255
    })
}

/// ヒット行（窓物理 px）→ 供給面 readback（validrect-local＝canvas-local）矩形へ変換する。
///
/// 供給面は validrect の寸だけを覆い、validrect 原点×k の位置へ mount される
/// （surface.rs「Arrangement offset ＝ validrect 原点×k」）。いっぽう `ChoiceHitRow::rect` は
/// **窓の物理 px** なので、面の中を走査するときは validrect 原点を差し引かなければならない。
pub fn to_canvas_local(r: &ChoiceHitRow, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (
        r.rect.left - ox,
        r.rect.top - oy,
        r.rect.right - ox,
        r.rect.bottom - oy,
    )
}

/// 走査窓を hover 行の帯の**上下**へ伸ばす画素数（上端・下端どちらの判定も窓で切らないため）。
pub const SCAN_MARGIN: u32 = 4;
/// ハイライト矩形の下端から hover 文字のインクがはみ出してよい上限（画素）＝ **0**。
/// 裁定 2026-09-06（第 2 回・R13.3）——**緩めてはならない**（R12.2）。
pub const BAND_OVERHANG_MAX: u32 = 0;

/// 強調の帯の期待値（hover 行の行矩形の上端・帯の丈・行ボックスの中央へ寄せる量）。
pub struct Band {
    pub line_top: u32,
    pub height: u32,
    pub offset: u32,
}

/// hover 行（`row`・canvas-local）の強調の帯を確かめる。走査窓は「hover 行の block 起点−余白
/// 〜 次の行（`next_row`）の block 起点＋余白」＝帯寸に依存しない独立の窓。
///
/// 確かめること: 帯の丈が `band.height`・帯の位置が `line_top + offset` から・読み戻した塗りの
/// 上端がヒット矩形の上端と一致（描画とヒットの単一導出・R13.2）・窓の中に白文字が在り、
/// 窓の上下端に接しない・字の上端は帯の内・下端のはみ出しは [`BAND_OVERHANG_MAX`]。
pub fn assert_hover_band(
    tag: &str,
    hover: &[u8],
    (w, h): (u32, u32),
    row: (f32, f32, f32, f32),
    next_row: (f32, f32, f32, f32),
    band: Band,
) {
    let x0 = row.0.floor().max(0.0) as u32;
    let x1 = (row.2.ceil().max(0.0) as u32).min(w);
    let block_top = row.1.floor().max(0.0) as u32;
    let y0 = block_top.saturating_sub(SCAN_MARGIN);
    let y1 = ((next_row.1.floor().max(0.0) as u32) + SCAN_MARGIN).min(h);
    assert!(
        x0 < x1 && y0 < y1 && y0 < block_top,
        "[{tag}] 走査窓が非退化で、かつ hover 行の block 起点 y{block_top} より上から見ている: x{x0}..{x1} y{y0}..{y1}"
    );
    let fill_span =
        y_span_where(hover, w, x0, x1, y0, y1, is_fill).expect("hover 行に塗り画素が在る");
    assert_eq!(
        fill_span.1 - fill_span.0 + 1,
        band.height,
        "[{tag}] ハイライト帯の丈は行送り {}: 実測 y{}..{}",
        band.height,
        fill_span.0,
        fill_span.1
    );
    // 帯の**絶対位置**（R13.1）: 行矩形の上端から行ボックスの中央へ offset だけ寄る。
    let top = band.line_top + band.offset;
    assert_eq!(
        (fill_span.0, fill_span.1),
        (top, top + band.height - 1),
        "[{tag}] ハイライト帯は行ボックスの中央へ {} 画素寄せて置かれる: 実測 y{}..{}",
        band.offset,
        fill_span.0,
        fill_span.1
    );
    assert_eq!(
        fill_span.0 as f32, row.1,
        "[{tag}] 読み戻した塗りの上端 y{} はヒット矩形の上端 {} と一致する（描画とヒットの単一導出）",
        fill_span.0, row.1
    );
    let text_pixels = count_in_rect(
        hover,
        w,
        h,
        (x0 as f32, y0 as f32, x1 as f32, y1 as f32),
        is_hover_text,
    );
    assert!(
        text_pixels > 0,
        "[{tag}] hover 行の走査窓 x{x0}..{x1} y{y0}..{y1} に白文字の画素が 1 つも無い\
         （字が描かれていないのに帯だけで緑になる取り違えを塞ぐ）"
    );
    let ink_span = y_span_where(hover, w, x0, x1, y0, y1, is_hover_text)
        .expect("画素数が 0 でない以上、白文字の縦範囲は在る");
    assert!(
        ink_span.1 < y1 - 1,
        "[{tag}] hover 文字のインク下端 y{} が走査窓の下端 y{y1} に接している——窓が下端を切っている\
         疑いがあり、はみ出しの測定が信用できない（窓を深くすること・R7.3）",
        ink_span.1
    );
    // 上端側の対の門。これが在って初めて、下の「字の上端が帯の内」が窓の取り方ではなく描画結果を
    // 見た判定になる（字が上へずれたら、まず窓の上端に届いてここが赤くなる）。
    assert!(
        ink_span.0 > y0,
        "[{tag}] hover 文字のインク上端 y{} が走査窓の上端 y{y0} に接している——窓が上端を切っている\
         疑いがあり、上端の判定が信用できない（窓を上へ開けること・R7.3）",
        ink_span.0
    );
    let overhang = ink_span.1.saturating_sub(fill_span.1);
    eprintln!(
        "[{tag}] hover 行の縦範囲（canvas-local）: 窓 x{x0}..{x1} y{y0}..{y1} / 帯 y{}..{}（丈 {}）/ \
         白文字 y{}..{}（{text_pixels} 画素）/ 上の余白 {} 画素（負なら帯の上へはみ出し）/ 下のはみ出し {overhang} 画素",
        fill_span.0,
        fill_span.1,
        fill_span.1 - fill_span.0 + 1,
        ink_span.0,
        ink_span.1,
        ink_span.0 as i64 - fill_span.0 as i64
    );
    assert!(
        ink_span.0 >= fill_span.0,
        "[{tag}] hover 行の文字のインク上端 y{} はハイライト矩形の上端 y{} の内側にある\
         （上へのはみ出し＝**文字の上が切れる**・R3.3/4.2）",
        ink_span.0,
        fill_span.0
    );
    assert!(
        overhang == BAND_OVERHANG_MAX,
        "[{tag}] hover 行の文字のインク下端 y{} がハイライト矩形の下端 y{} から {overhang} 画素はみ出した\
         ＝**文字の下が切れる**（R3.3/4.2）。裁定 2026-09-06（第 2 回・R13.3）が許すのは \
         {BAND_OVERHANG_MAX} 画素である。**帯を広げてはならない**（隣接行の帯と重なり、\
         どの選択肢を指しているかの一意性が壊れる）。**許容幅を緩めてもならない**（R12.2）。\
         この数値を添えて開発者の裁定を仰ぐこと",
        ink_span.1,
        fill_span.1
    );
}

// ══ PNG の書き出し（依存追加なし・viewbox_draw.rs の診断ダンプと同一アルゴリズム） ══════

/// read_back（premultiplied BGRA）を白背景へ合成した PNG を `AREKA_DIAG_OUT`（無指定は
/// `CARGO_TARGET_TMPDIR`＝crate 専用 tmp）の `name` へ保存し、そのパスを返す。
pub fn dump_png_on_white(name: &str, bgra: &[u8], w: u32, h: u32) -> String {
    let out_dir =
        std::env::var("AREKA_DIAG_OUT").unwrap_or_else(|_| env!("CARGO_TARGET_TMPDIR").to_string());
    let png = encode_png_rgba(&composite_on_white(bgra, w, h), w, h);
    let path = format!("{out_dir}/{name}");
    std::fs::write(&path, &png).expect("PNG 書き込み");
    path
}

/// premultiplied BGRA read_back を白背景へ合成した RGBA8 を返す。
fn composite_on_white(bgra: &[u8], w: u32, h: u32) -> Vec<u8> {
    let bg = [255u8, 255, 255];
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for (idx, px) in bgra.chunks_exact(4).enumerate() {
        let (b, g, r, a) = (px[0] as u32, px[1] as u32, px[2] as u32, px[3] as u32);
        let inv = 255 - a;
        let o = idx * 4;
        rgba[o] = (r + bg[0] as u32 * inv / 255).min(255) as u8;
        rgba[o + 1] = (g + bg[1] as u32 * inv / 255).min(255) as u8;
        rgba[o + 2] = (b + bg[2] as u32 * inv / 255).min(255) as u8;
        rgba[o + 3] = 255;
    }
    rgba
}

/// CRC-32（PNG チャンク用・多項式 0xEDB88320）。
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Adler-32（zlib ストリーム末尾）。
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// PNG チャンク（長さ＋種別＋データ＋CRC）を書き足す。
fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut kd = Vec::with_capacity(4 + data.len());
    kd.extend_from_slice(kind);
    kd.extend_from_slice(data);
    out.extend_from_slice(&kd);
    out.extend_from_slice(&crc32(&kd).to_be_bytes());
}

/// RGBA8 を無圧縮 deflate（stored ブロック）で PNG エンコードする（依存追加なし）。
fn encode_png_rgba(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut raw = Vec::with_capacity((h * (1 + w * 4)) as usize);
    for y in 0..h {
        raw.push(0u8); // フィルタ 0
        let row = ((y * w * 4) as usize)..(((y + 1) * w * 4) as usize);
        raw.extend_from_slice(&rgba[row]);
    }
    let mut zlib = vec![0x78u8, 0x01];
    let mut i = 0usize;
    while i < raw.len() {
        let chunk = (raw.len() - i).min(65535);
        let bfinal = if i + chunk >= raw.len() { 1u8 } else { 0u8 };
        zlib.push(bfinal); // BFINAL ＋ BTYPE=00（stored）
        zlib.extend_from_slice(&(chunk as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(chunk as u16)).to_le_bytes());
        zlib.extend_from_slice(&raw[i..i + chunk]);
        i += chunk;
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]); // 8bit・color type 6（RGBA）
    png_chunk(&mut out, b"IHDR", &ihdr);
    png_chunk(&mut out, b"IDAT", &zlib);
    png_chunk(&mut out, b"IEND", &[]);
    out
}
