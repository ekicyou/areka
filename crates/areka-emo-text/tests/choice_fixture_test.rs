//! # choice_fixture_test — choice-render 検証の test-local 最小適合 fixture（task 11.1・R7.6）
//!
//! design.md「Testing Strategy > Integration Tests #5（7.6）」の正典述語を張る:
//! **実フォント出力の目視確認（既定フォント盲点の回避）＋ test-local 最小 fixture を tests 配下に
//! 自前で用意**する。fixture は `tests/fixtures/emo2-choice/`:
//!
//! - `descript-cursor.txt` — cursor.* 指定バルーン（SquareFill 実導出＝塗り(105,25,25)＋白文字(255,255,255)）。
//! - `descript-plain.txt`  — cursor.* 未指定バルーン（Invert 縮退＝塗り=既定 font.color・文字=255−c）。
//! - `menu.txt`            — 短メニュー台本（4 項目・注入 cue 列の対応表）。
//!
//! 本テストは 2 本:
//! 1. [`fixtures_parse_to_expected_choice_styles`]（純・GPU 不要）: fixture descript を **実 balloon
//!    パーサ**（`areka_parsers::balloon::parse_str`）で parse し、cursor.* 指定→`SquareFill`・
//!    未指定→`Invert` へ解決されることを固定する（fixture が in-code モデルでなく実 parse される
//!    descript であることの証明）。
//! 2. [`real_font_menu_hover_render_dumps_png`]（GPU・headless）: `descript-cursor.txt` を実フォントで
//!    解決し、4 項目メニュー＋ordinal 0 の hover を通し経路（`register_actor`→`apply_cue`→
//!    `inject_choice_hover`→`present_frame`→`read_back`）で描画する。pixel 檻（hover 行のセグメント
//!    矩形へ塗り色＋白文字画素が載る）に加え、read_back を白背景へ合成して **PNG を既知パスへ保存**し、
//!    実フォント出力の目視確認を pixel 檻へ伴わせる（記憶 emo-text-byte-equiv-default-font-blindspot）。

use std::path::PathBuf;

use areka_emo_text::actor::{
    ChoiceHitRow, ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame,
};
use areka_emo_text::choice::ResolvedChoiceStyle;
use areka_emo_text::region::TextRegion;
use areka_emo_text::state::TextLayerConfig;
use areka_emo_text::writing::WritingMode;
use areka_parsers::balloon::{BalloonModel, parse_str};
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::{GraphicsCore, Visual, WucGraphicsResource};

// ── fixture ロード（vertical_fixture_test と同一の読み込み規約：実ファイル＋charset デコード） ──

/// fixture 配置（`tests/fixtures/emo2-choice/`）。
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("emo2-choice")
}

/// fixture ファイルを読み、`charset,UTF-8` 宣言に従いデコードする（parser-foundation の decode 経路）。
fn read_decoded(name: &str) -> String {
    let path = fixture_dir().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "fixture {} の読取に失敗した（task 11.1 の test-local fixture が必要）: {e}",
            path.display()
        )
    });
    decode(&bytes, DefaultEncoding::Utf8)
}

/// cursor.* 指定 fixture を実 balloon パーサで parse した BalloonModel。
fn cursor_model() -> BalloonModel {
    parse_str(&read_decoded("descript-cursor.txt"), None)
}

/// cursor.* 未指定 fixture を実 balloon パーサで parse した BalloonModel。
fn plain_model() -> BalloonModel {
    parse_str(&read_decoded("descript-plain.txt"), None)
}

fn font_color_tuple(model: &BalloonModel) -> (u8, u8, u8) {
    let c = model.font().color();
    (c.r().unwrap_or(0), c.g().unwrap_or(0), c.b().unwrap_or(0))
}

// ══ 開始点の檻: origin 未宣言（正典推奨形）でも書字開始角 (5,5) が変わらない（GPU 不要） ══════

/// 両 fixture の**描画開始点**が `(5, 5)` であることを固定する
/// （spec `areka-P0-balloon-vertical-canon` 要件 3.11／10.9）。
///
/// 同 spec は「origin クランプ正準」を撤去し、あわせて validrect 外の `origin.x,0`／`origin.y,0`
/// 宣言を持っていた出荷／テスト資産を**正典推奨形**（指定せず validrect の定義に任せる）へ
/// 是正した。本 fixture の 2 本もその対象だが、**着地後も開始点を観測する消費者が 1 本も
/// 無かった**（同 spec タスク 4.1 の意味論棚卸しが着手前に記録した既知の穴）。ここで塞ぐ。
///
/// - **撤去前**（宣言 `origin(0,0)` ＋クランプ）: `0` は validrect `[5, …]` の外なので書字開始角 `(5,5)` へ寄っていた。
/// - **撤去後**（宣言なし）: `None` 腕が書字開始角 `(5,5)` へ縮退する。
///
/// つまり両経路が同じ値を与えるため是正は挙動中立である——その主張を反証可能にするのが本檻。
/// もし `origin` 宣言が復活すれば、クランプはもう無いので開始点は字義どおり `(0,0)` へ落ちて赤くなる。
///
/// 開始点が**画像原寸に依存しない**ことも併せて見る（`validrect.left`／`.top` が非負素通しのため）。
/// 本 fixture は画像を持たないので、これは「どの寸法で解決しても同じ」ことの明示でもある。
#[test]
fn choice_fixtures_start_at_writing_corner_regardless_of_image_size() {
    // 画像原寸を 2 通り振る（負値辺 right/bottom は寸法で動くが、開始点は動かない）。
    for image_size in [(200u32, 150u32), (400u32, 224u32)] {
        for (label, model) in [("cursor", cursor_model()), ("plain", plain_model())] {
            // 前提: 正典推奨形＝origin は未宣言のまま（宣言が戻ったら本檻の意味が変わる）。
            assert_eq!(
                (model.origin().x(), model.origin().y()),
                (None, None),
                "{label}: fixture は origin を宣言しない（正典推奨形・要件 10.9）"
            );

            let region = TextRegion::resolve(&model, image_size, WritingMode::HorizontalTb);
            assert_eq!(
                region.start(),
                (5.0, 5.0),
                "{label} / {image_size:?}: 書字開始角＝validrect 左上 (5,5)（要件 3.11）"
            );
            assert_eq!(
                (region.left(), region.top()),
                (5.0, 5.0),
                "{label} / {image_size:?}: validrect の left/top は非負素通し"
            );
        }
    }
}

// ══ テスト 1: fixture descript が実 parse で期待スタイルへ解決される（GPU 不要） ══════════════

/// Observable（R7.6）: test-local fixture の 2 descript を**実 balloon パーサ**で parse すると、
/// cursor.* 指定バルーンは `SquareFill { fill:(105,25,25), text:(255,255,255) }`・cursor.* 未指定
/// バルーンは `Invert` へ解決される。fixture は実フォント（Yu Gothic UI）を font.name に持つ
/// （既定 ＭＳ ゴシック盲点の回避＝実フォント目視確認の前提）。
#[test]
fn fixtures_parse_to_expected_choice_styles() {
    // ── cursor.* 指定 → SquareFill（実導出・9.2/9.3 の cursor_model と同値） ──
    let cursor = cursor_model();
    assert_eq!(
        cursor.font().name(),
        Some("Yu Gothic UI"),
        "cursor fixture は実フォントを指定する（既定 ＭＳ ゴシック盲点の回避・7.6）"
    );
    let cursor_style =
        ResolvedChoiceStyle::resolve(Some(cursor.cursor()), font_color_tuple(&cursor));
    assert_eq!(
        cursor_style,
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        },
        "cursor.* 指定 fixture は SquareFill（塗り(105,25,25)＋白文字(255,255,255)）へ解決される"
    );

    // ── cursor.* 未指定 → Invert（矩形反転縮退・M1 実導出） ──
    let plain = plain_model();
    assert_eq!(
        plain.font().name(),
        Some("Yu Gothic UI"),
        "plain fixture も実フォントを指定する"
    );
    let plain_style = ResolvedChoiceStyle::resolve(Some(plain.cursor()), font_color_tuple(&plain));
    assert_eq!(
        plain_style,
        ResolvedChoiceStyle::Invert,
        "cursor.* 未指定 fixture は Invert（矩形反転縮退）へ解決される"
    );

    // ── ResolvedBalloonText 経由でも同一スタイルを運ぶ（通し経路が読む choice_style） ──
    let image = (200u32, 120u32);
    assert_eq!(
        ResolvedBalloonText::resolve(&cursor, image).choice_style,
        ResolvedChoiceStyle::SquareFill {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        },
        "ResolvedBalloonText.choice_style も cursor fixture では SquareFill"
    );
    assert_eq!(
        ResolvedBalloonText::resolve(&plain, image).choice_style,
        ResolvedChoiceStyle::Invert,
        "ResolvedBalloonText.choice_style も plain fixture では Invert"
    );

    // ── menu.txt の項目数（\q[…]）が注入 cue 列（4 項目）と一致する（台本と注入の対応検証） ──
    let menu = read_decoded("menu.txt");
    let q_count = menu.matches("\\q[").count();
    assert_eq!(
        q_count,
        MENU.len(),
        "menu.txt の \\q 項目数が注入 cue 列（{} 項目）と一致する: {q_count}",
        MENU.len()
    );
}

// ══ テスト 2: 実フォントで短メニュー＋hover を描き、PNG を目視確認用に保存する（GPU・headless） ══

/// 注入する短メニュー（menu.txt と対応・4 項目）。
const MENU: &[(&str, &str, &str)] = &[
    ("OnYes", "はい", "r0"),
    ("OnNo", "いいえ", "r1"),
    ("OnMaybe", "どちらでも", "r2"),
    ("OnLater", "あとで", "r3"),
];

/// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World（headless・MTA）。
fn make_world_with_gpu() -> World {
    // SAFETY: COM の MTA 初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
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

/// emo-present `VisualMount` と同型の予約スロット（(window, slot) を返す）。
fn spawn_reserved_slot(world: &mut World) -> (bevy_ecs::entity::Entity, bevy_ecs::entity::Entity) {
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

fn choice_cue(id: &str, text: &str, references: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::Choice {
            id: id.into(),
            text: text.into(),
            references: vec![references.into()],
        },
        duration: text.chars().count() as f64 * 0.05,
    }
}

fn newline_cue() -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::NewLine { ratio: 1.0 },
        duration: 0.0,
    }
}

/// SquareFill 塗り色（105,25,25・premultiplied α=255 ゆえ BGRA=(25,25,105,255)）を矩形内で数える。
fn fill_pixels_in_rect(bytes: &[u8], width: u32, height: u32, r: &ChoiceHitRow) -> usize {
    let x0 = r.rect.left.floor().max(0.0) as u32;
    let x1 = (r.rect.right.ceil() as u32).min(width);
    let y0 = r.rect.top.floor().max(0.0) as u32;
    let y1 = (r.rect.bottom.ceil() as u32).min(height);
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if bytes[i] == 25 && bytes[i + 1] == 25 && bytes[i + 2] == 105 && bytes[i + 3] == 255 {
                n += 1;
            }
        }
    }
    n
}

/// 白文字（≈255,255,255・全チャネル閾値で AA 端を除いた芯）を矩形内で数える。
fn white_pixels_in_rect(bytes: &[u8], width: u32, height: u32, r: &ChoiceHitRow) -> usize {
    let x0 = r.rect.left.floor().max(0.0) as u32;
    let x1 = (r.rect.right.ceil() as u32).min(width);
    let y0 = r.rect.top.floor().max(0.0) as u32;
    let y1 = (r.rect.bottom.ceil() as u32).min(height);
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * width + x) * 4) as usize;
            if bytes[i] >= 200 && bytes[i + 1] >= 200 && bytes[i + 2] >= 200 && bytes[i + 3] == 255
            {
                n += 1;
            }
        }
    }
    n
}

fn opaque_count(bytes: &[u8]) -> usize {
    bytes.chunks_exact(4).filter(|px| px[3] != 0).count()
}

/// 指定 x 帯 `[x0, x1)` × y 帯 `[y0, y1)` で述語に合う画素が現れた **y の最小/最大**（無ければ `None`）。
fn y_span_where(
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

/// Observable（R7.6）: cursor.* 指定 fixture を**実フォント（Yu Gothic UI）**で解決し、4 項目メニュー
/// ＋ordinal 0 の hover を通し経路で描画すると、(a) hover 行のセグメント矩形へ SquareFill 塗り色
/// (105,25,25)＋白文字画素が載り（pixel 檻）、(b) その read_back を白背景へ合成した PNG が既知パスへ
/// 保存される（実フォント出力の目視確認を pixel 檻へ伴わせる）。
///
/// PNG 出力先は env `AREKA_DIAG_OUT`（無指定は `CARGO_TARGET_TMPDIR`＝crate 専用 tmp）。
#[test]
fn real_font_menu_hover_render_dumps_png() {
    let mut world = make_world_with_gpu();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let actor = ActorKey::from("0");

    let model = cursor_model();
    let image = (200u32, 120u32);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&model, image),
    );

    // 4 項目を NewLine 区切りで別行へ（hover 行が 1 行に限定されることを観測可能にする）。
    for (i, (id, text, refs)) in MENU.iter().enumerate() {
        rt.apply_cue(&choice_cue(id, text, refs));
        if i + 1 < MENU.len() {
            rt.apply_cue(&newline_cue());
        }
    }

    // 全リビール済みフレーム（t 大）。hover 前も選択肢テキストが描かれる。
    present_frame(&mut rt, &mut world, 100.0).expect("ベースライン提示");
    let rows: Vec<ChoiceHitRow> = rt.choice_hit_rows(&actor).to_vec();
    assert_eq!(rows.len(), MENU.len(), "4 選択肢＝4 ヒット行");

    let (w, h) = rt.surface(&actor).expect("供給面").size();
    let base = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");
    assert!(
        opaque_count(&base) > 0,
        "実フォントで選択肢テキストが描画される（非退化）"
    );

    // ── ordinal 0 を hover 注入 → 再提示 → read_back ──
    rt.inject_choice_hover(&actor, Some(0));
    present_frame(&mut rt, &mut world, 100.0).expect("hover 提示");
    let hover = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");

    // pixel 檻: hover 行（ordinal 0）へ SquareFill 塗り色＋白文字が載る。
    assert!(
        fill_pixels_in_rect(&hover, w, h, &rows[0]) > 0,
        "hover 行に SquareFill 塗り色(105,25,25)画素が載る: {:?}",
        rows[0].rect
    );
    assert!(
        white_pixels_in_rect(&hover, w, h, &rows[0]) > 0,
        "hover 行に白文字(255,255,255)画素が載る: {:?}",
        rows[0].rect
    );
    // 非 hover 行（ordinal 1）へは塗り色画素が載らない（ダーティ限定＝hover 行のみハイライト）。
    assert_eq!(
        fill_pixels_in_rect(&hover, w, h, &rows[1]),
        0,
        "非 hover 行には SquareFill 塗り色画素が載らない"
    );

    // ── 回帰檻（実機不具合「hover 文字の下が切れる」）: インクがハイライト矩形から縦にはみ出さない ──
    //
    // 帯（ハイライト矩形＝ヒット矩形のブロック軸寸）を em ボックス丈（font.height）で切ると、
    // DirectWrite が ascent+descent で描く実インクの descent 側が塗りの外へ落ちる（実フォント
    // Yu Gothic UI は行ボックス 1.3301em）。白バルーン＋白 hover 文字では「下が消えた」ように見える。
    // 走査 y 窓は「hover 行の block 起点〜次行の block 起点」＝帯寸に依存しない独立の窓。
    let x0 = rows[0].rect.left.floor().max(0.0) as u32;
    let x1 = (rows[0].rect.right.ceil() as u32).min(w);
    let y0 = rows[0].rect.top.floor().max(0.0) as u32;
    let y1 = (rows[1].rect.top.floor().max(0.0) as u32).min(h);
    assert!(
        x0 < x1 && y0 < y1,
        "走査窓が非退化: x{x0}..{x1} y{y0}..{y1}"
    );
    let fill_span = y_span_where(&hover, w, x0, x1, y0, y1, |px| {
        px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255
    })
    .expect("hover 行に塗り画素が在る");
    let ink_span =
        y_span_where(&hover, w, x0, x1, y0, y1, |px| px[3] != 0).expect("hover 行にインクが在る");
    assert!(
        ink_span.0 >= fill_span.0 && ink_span.1 <= fill_span.1,
        "hover 行のインク縦範囲 y{}..{} がハイライト矩形 y{}..{} の内側に収まる\
         （はみ出し＝**文字の下が切れる**・R3.3/4.2）",
        ink_span.0,
        ink_span.1,
        fill_span.0,
        fill_span.1
    );

    // ── 目視確認: read_back（premultiplied BGRA）を白背景へ合成して PNG を保存する ──
    let out_dir =
        std::env::var("AREKA_DIAG_OUT").unwrap_or_else(|_| env!("CARGO_TARGET_TMPDIR").to_string());
    let rgba = composite_on_white(&hover, w, h);
    let png = encode_png_rgba(&rgba, w, h);
    let path = format!("{out_dir}/choice_menu_hover_realfont.png");
    std::fs::write(&path, &png).expect("PNG 書き込み");
    eprintln!(
        "[choice-fixture] 実フォント目視確認 PNG を保存: {path} ({w}x{h}・font=Yu Gothic UI・\
         4 項目メニュー・ordinal 0 hover＝SquareFill 塗り(105,25,25)＋白文字)"
    );
}

// ── PNG エンコード（依存追加なし・viewbox_draw.rs の診断ダンプと同一アルゴリズム） ──────────

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
