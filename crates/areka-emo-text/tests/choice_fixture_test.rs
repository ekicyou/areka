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
//!    解決し、4 項目メニュー＋ordinal 1（先頭でない行）の hover を通し経路（`register_actor`→`apply_cue`→
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

/// ヒット行（バルーン窓の物理 px）→ 供給面 readback の座標（validrect-local）へ写す。
///
/// 供給面は validrect の寸だけを覆い、validrect 原点×k の位置へ mount される
/// （surface.rs「Arrangement offset ＝ validrect 原点×k」）。いっぽう `ChoiceHitRow::rect` は
/// **窓の物理 px** なので、面の中を走査するときは validrect 原点を差し引かなければならない。
/// 本 fixture の validrect 原点は (5,5)（`TextRegion::resolve(...).start()`）なので、差し引かずに
/// 使うと走査窓が 5 画素下へずれ、帯の上 5 行が窓から落ちる。
fn to_canvas_local(r: &ChoiceHitRow, ox: f32, oy: f32) -> (f32, f32, f32, f32) {
    (
        r.rect.left - ox,
        r.rect.top - oy,
        r.rect.right - ox,
        r.rect.bottom - oy,
    )
}

fn count_in_rect(
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

/// SquareFill 塗り色（105,25,25・premultiplied α=255 ゆえ BGRA=(25,25,105,255)）を矩形内で数える。
fn fill_pixels_in_rect(bytes: &[u8], width: u32, height: u32, rect: (f32, f32, f32, f32)) -> usize {
    count_in_rect(bytes, width, height, rect, is_fill)
}

/// 白文字（≈255,255,255・全チャネル閾値で AA 端を除いた芯）を矩形内で数える。
fn white_pixels_in_rect(
    bytes: &[u8],
    width: u32,
    height: u32,
    rect: (f32, f32, f32, f32),
) -> usize {
    count_in_rect(bytes, width, height, rect, |px| {
        px[0] >= 200 && px[1] >= 200 && px[2] >= 200 && px[3] == 255
    })
}

/// SquareFill の塗り帯そのもの（premultiplied BGRA=(25,25,105,255)）。
fn is_fill(px: &[u8]) -> bool {
    px[0] == 25 && px[1] == 25 && px[2] == 105 && px[3] == 255
}

/// hover 行の**文字だけ**を選ぶ述語（塗り帯を文字と数えないための切り分け）。
///
/// α だけでインクを拾うと塗り帯の画素にも当たり、返る縦範囲は帯の縦範囲そのものになる——
/// 字が 1 画素も無くても「インクは帯の中」が成り立ってしまう。hover 文字は白
/// （`cursor.font.color` 255,255,255）で premultiplied では B ＝ G ＝ R ＝ α、帯の上に載った字は
/// B ＝ 25 ＋ 230 × 被覆率 になる。塗り帯の青チャネルは 25 止まり（縁が半透明なら更に小さい）
/// なので、「青チャネル 128 以上」で白文字だけを拾える。
fn is_hover_text(px: &[u8]) -> bool {
    px[3] >= 128 && px[0] >= 128
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
/// ＋ordinal 1 の hover を通し経路で描画すると、(a) hover 行のセグメント矩形へ SquareFill 塗り色
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

    // ── ordinal 1（「いいえ」）を hover 注入 → 再提示 → read_back ──
    //
    // ホバーするのを**先頭でない行**にするのは、下の縦範囲の検査で「字の上端が帯の内」を
    // 反証可能にするためである。先頭行は帯の上端が面の縁（y0）にあり、走査窓をそれより上へ
    // 開けようがないので、上端の判定は窓の取り方だけで必ず成り立ってしまう（字がベースラインごと
    // 上へずれても窓に切られて緑のまま）。ordinal 1 なら帯の上端は行送り 1 つぶん下（y22）に
    // あり、窓はその 4 画素上から見られる。
    rt.inject_choice_hover(&actor, Some(1));
    present_frame(&mut rt, &mut world, 100.0).expect("hover 提示");
    let hover = rt
        .surface(&actor)
        .expect("供給面")
        .read_back()
        .expect("read_back");

    // read_back は validrect-local なので、窓物理のヒット矩形から validrect 原点を差し引く。
    let region = TextRegion::resolve(&model, image, WritingMode::HorizontalTb);
    let (ox, oy) = (region.left(), region.top());
    let row0_cl = to_canvas_local(&rows[0], ox, oy);
    let row1_cl = to_canvas_local(&rows[1], ox, oy);
    let row2_cl = to_canvas_local(&rows[2], ox, oy);

    // 画素の検査: hover 行（ordinal 1）へ SquareFill 塗り色＋白文字が載る。
    assert!(
        fill_pixels_in_rect(&hover, w, h, row1_cl) > 0,
        "hover 行に SquareFill 塗り色(105,25,25)画素が載る: canvas-local {row1_cl:?}"
    );
    assert!(
        white_pixels_in_rect(&hover, w, h, row1_cl) > 0,
        "hover 行に白文字(255,255,255)画素が載る: canvas-local {row1_cl:?}"
    );
    // 非 hover 行（ordinal 0＝hover 行の 1 つ上）へは塗り色画素が載らない
    //（ダーティ限定＝hover 行のみハイライト。帯が上の行へ伸びていないことも同時に言える）。
    assert_eq!(
        fill_pixels_in_rect(&hover, w, h, row0_cl),
        0,
        "非 hover 行には SquareFill 塗り色画素が載らない: canvas-local {row0_cl:?}"
    );

    // ── 回帰検査（実機不具合「hover 文字の下が切れる」）: インクがハイライト矩形からほぼはみ出さない ──
    //
    // 帯（ハイライト矩形＝ヒット矩形のブロック軸寸）を em ボックス丈（font.height）で切ると、
    // DirectWrite が ascent+descent で描く実インクの descent 側が塗りの外へ落ちる（実フォント
    // Yu Gothic UI は行ボックス 1.3301em）。白バルーン＋白 hover 文字では「下が消えた」ように見える。
    // 走査 y 窓は「hover 行の block 起点−余白 4 〜 次行の block 起点＋余白 4」＝帯寸に依存しない
    // 独立の窓。窓を帯とぴったり同じにすると、帯からはみ出したインクが窓の外へ落ちて**測れない
    // まま緑になる**（同 spec 要件 7.3）。これは上端にも同じく効く——窓の上端を帯の上端と揃えると
    // 「字の上端は帯の上端以上」が窓の取り方だけで必ず成り立ち、字が上へずれても緑のままになる。
    // ゆえに窓は帯の上下へ 4 画素ずつ開ける。上の行の字は黒（premultiplied の青チャネルは 0）ゆえ
    // 白文字の述語に当たらず、塗り色 (25,25,105,255) でもないので窓へ入っても数えられない。
    // 次行のインクはその行の block 起点から 5〜6 画素下で始まるので、4 画素の余白なら窓へ入らない。
    // 座標は validrect 原点を差し引いた canvas-local。
    //
    // **裁定 2026-09-06（第 2 回・spec `areka-P0-emo-text-line-height-canon` 要件 3.6）**:
    // 行送りが正典（`font.height + 行間 2`）へ確定した後、実フォントの読み戻しで帯の下端から
    // 文字のインクが下へ出た。開発者の裁定は「**2 画素のはみ出しを許容する**」である。ゆえに
    // 下の判定は「上端は帯の内・下端のはみ出しは 2 画素以内」を見る。
    //
    // はみ出しの量はフォント寸で変わる——**この fixture（font 20）では 1 画素**（ordinal 1 の実測:
    // 帯 y22..43 ＝ 丈 22（font.height 20 ＋ 行間 2）に対し白文字のインク y28..44）だが、
    // **正典の 28 では字によって 2 画素**である
    // （閉・も・調・頻・度 が 2 画素、は・い・じ・る・ど・整 が 1 画素。
    // `line_pitch_readback_test.rs` の 3 本目に実測表がある）。同日の第 1 回裁定（1 画素）は
    // この font 20 の fixture だけで測った値であり、28 での測り直しがそれを上書きした。
    //
    // これは**裁定による意味の変更**であって、都合に合わせて許容幅を緩めたものではない。
    // 帯を広げる案は採らない——帯を行送りより広げると隣接する行の帯と重なり、どの選択肢を
    // 指しているかの一意性が壊れる（同 spec design §「帯の防御式を保つ」）。
    // **はみ出しが 3 画素以上になった場合は帯を広げず、画素数を添えて改めて開発者の裁定を仰ぐ。**
    /// 走査窓を hover 行の帯の**上下**へ伸ばす画素数（上端・下端どちらの判定も窓で切らないため）。
    const SCAN_MARGIN: u32 = 4;
    /// ハイライト矩形の下端から hover 文字のインクがはみ出してよい上限（画素）。
    /// 裁定 2026-09-06（第 2 回・要件 3.6）＝ 2 画素を許容する
    /// ——**許容幅の緩和ではなく裁定による意味の変更**。
    /// この fixture（font 20）の実測は 1 画素で上限に余裕があるが、正典の 28 では 2 画素になる。
    const BAND_OVERHANG_MAX: u32 = 2;

    let x0 = row1_cl.0.floor().max(0.0) as u32;
    let x1 = (row1_cl.2.ceil().max(0.0) as u32).min(w);
    let block_top = row1_cl.1.floor().max(0.0) as u32;
    let y0 = block_top.saturating_sub(SCAN_MARGIN);
    let y1 = ((row2_cl.1.floor().max(0.0) as u32) + SCAN_MARGIN).min(h);
    assert!(
        x0 < x1 && y0 < y1 && y0 < block_top,
        "走査窓が非退化で、かつ hover 行の block 起点 y{block_top} より上から見ている: x{x0}..{x1} y{y0}..{y1}"
    );
    let fill_span =
        y_span_where(&hover, w, x0, x1, y0, y1, is_fill).expect("hover 行に塗り画素が在る");
    assert_eq!(
        fill_span.1 - fill_span.0 + 1,
        22,
        "ハイライト矩形の丈は行送り 22（font.height 20 ＋ 行間 2）: 実測 y{}..{}",
        fill_span.0,
        fill_span.1
    );
    // インクは**文字だけ**を拾う（塗り帯を文字と数えると、字が消えても上端の判定が成り立つ）。
    let text_pixels = count_in_rect(
        &hover,
        w,
        h,
        (x0 as f32, y0 as f32, x1 as f32, y1 as f32),
        is_hover_text,
    );
    assert!(
        text_pixels > 0,
        "hover 行の走査窓 x{x0}..{x1} y{y0}..{y1} に白文字の画素が 1 つも無い\
         （字が描かれていないのに帯だけで緑になる取り違えを塞ぐ）"
    );
    let ink_span = y_span_where(&hover, w, x0, x1, y0, y1, is_hover_text)
        .expect("画素数が 0 でない以上、白文字の縦範囲は在る");
    assert!(
        ink_span.1 < y1 - 1,
        "hover 文字のインク下端 y{} が走査窓の下端 y{y1} に接している——窓が下端を切っている疑いがあり、\
         はみ出しの測定が信用できない（窓を深くすること・R7.3）",
        ink_span.1
    );
    // 上端側の対の門。これが在って初めて、下の「字の上端が帯の内」が窓の取り方ではなく描画結果を
    // 見た判定になる（字が上へずれたら、まず窓の上端に届いてここが赤くなる）。
    assert!(
        ink_span.0 > y0,
        "hover 文字のインク上端 y{} が走査窓の上端 y{y0} に接している——窓が上端を切っている疑いがあり、\
         上端の判定が信用できない（窓を上へ開けること・R7.3）",
        ink_span.0
    );
    eprintln!(
        "[choice-fixture] hover 行（ordinal 1「いいえ」）の縦範囲（font.height 20・canvas-local）: 窓 x{x0}..{x1} y{y0}..{y1} / \
         帯 y{}..{}（丈 {}）/ 白文字 y{}..{}（{text_pixels} 画素）/ 上の余白 {} 画素（負なら帯の上へはみ出し）\
         / 下のはみ出し {} 画素",
        fill_span.0,
        fill_span.1,
        fill_span.1 - fill_span.0 + 1,
        ink_span.0,
        ink_span.1,
        ink_span.0 as i64 - fill_span.0 as i64,
        ink_span.1.saturating_sub(fill_span.1)
    );

    assert!(
        ink_span.0 >= fill_span.0,
        "hover 行の文字のインク上端 y{} はハイライト矩形の上端 y{} の内側にある\
         （上へのはみ出し＝**文字の上が切れる**・R3.3/4.2）",
        ink_span.0,
        fill_span.0
    );
    let overhang = ink_span.1.saturating_sub(fill_span.1);
    assert!(
        overhang <= BAND_OVERHANG_MAX,
        "hover 行の文字のインク下端 y{} がハイライト矩形の下端 y{} から {overhang} 画素はみ出した\
         （インク y{}..{}・矩形 y{}..{}）。裁定 2026-09-06（第 2 回・要件 3.6）が許すのは \
         {BAND_OVERHANG_MAX} 画素までである。**帯を広げてはならない**（隣接行の帯と重なり、\
         どの選択肢を指しているかの一意性が壊れる）。この数値を添えて開発者の裁定を仰ぐこと",
        ink_span.1,
        fill_span.1,
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
         4 項目メニュー・ordinal 1「いいえ」hover＝SquareFill 塗り(105,25,25)＋白文字)"
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
