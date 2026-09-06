//! バルーン画像中央の指定（`centerx`／`centery`）と、**宣言された文字描画開始点**を基点とする
//! 絶対座標を、**3 書字方向すべて**で固定するテスト（タスク 5.2・検証表 V6・V7）。
//!
//! # 兄弟ファイルとの分担（重複させない）
//!
//! - `layout_cursor_tests.rs` — 横書きの行区切り・軸上書き・末尾蒸発・両軸 no-op・縮退 warn-once、
//!   および軸ごと合成（H2）と実効位置（H3）。
//! - `layout_cursor_order_tests.rs` — 書かれた順（H3b・H3c）。
//! - `layout_cursor_wiring_tests.rs` — 横書きの配線（H1・H4・H5・H6・範囲外記録の肯定側）。
//!   同ファイルの `declared_origin_moves_the_absolute_basis_off_the_write_start_corner` は
//!   **横書き 1 方向だけの対照**（H1 の弁別用）として置かれており、V7（3 方向）は本ファイルの
//!   所管である旨が同ファイルの doc に明記されている。本ファイルは横書きぶんを対照の
//!   繰り返しに終わらせず、**3 方向を 1 本の主張に束ねる**形で書く。
//! - `layout_cursor_vertical_tests.rs`／`layout_cursor_vertical_canon_tests.rs` — 縦書き 2 方向の
//!   着地値と正典の記述例（V1〜V5）。中央指定・宣言 `origin` はどちらも扱っていない。
//! - `cursor_tag_tests.rs`／`cursor_tag_resolve_tests.rs` — 解決そのもの（純関数）。**配線を
//!   通した経路**は本ファイルが初めて見る（純関数の檻は `CursorBasis` を手で組むので、
//!   配線が `image_size` に何を渡しているかを一切主張しない）。
//!
//! # 期待値の出どころ
//!
//! すべて design.md の検証表と正典逐語（requirements.md 付録 A）から**式で**導く。実装の
//! 戻り値は書き写さない。
//!
//! - V6: `\_l[centerx,centery]あ` → 3 方向とも **X = 200・Y = 112**（＝バルーン画像原寸
//!   `(400, 224)` の幅／高さの半分。方向に依らない・Requirement 4.4／4.5）。
//! - V7: 宣言 `origin (50, 20)` で `\_l[0,0]あ` → 横書き `(50, 20)`・`vertical_rl` は
//!   **列右端 50**・`vertical_lr` は **列左端 50**（Requirement 2.9／9.6）。
//!
//! # 共通前提（**弁別 fixture**・既定の共通前提を使ってはならない理由）
//!
//! `FixedMetrics`・`font_height = 10`（全角 'あ' の advance 10・`line_pitch = 10 + 行間 2 = 12`）・
//! バルーン画像原寸 `IMAGE = (400, 224)`。**validrect は画像全域ではなく部分矩形
//! `left 30 / top 8 / right 350 / bottom 210`** にしてある。
//!
//! 既存の横書き檻・縦書き檻の共通前提は validrect ＝画像全域（`0/0/400/224`）で、そこでは
//! `image_size.0 / 2` と `right / 2` が同値（どちらも 200）・`image_size.1 / 2` と `bottom / 2` も
//! 同値（どちらも 112）になる。**その fixture で V6 を書くと、`centerx`／`centery` の基準を
//! validrect の辺と取り違えた実装が素通りする。** 実際、本タスク着手時点で配線の
//! `image_size: region.image_size()` を `(region.right(), region.bottom())` に差し替えても
//! ワークスペースのテストは 1 本も赤にならなかった（タスク 4.1／4.4 のレビューが摂動で実証）。
//!
//! 画像原寸は既定と同じ `(400, 224)` に保ち、validrect だけをずらしてあるので、design.md の
//! V6 が定める literal（X = 200・Y = 112）はそのまま使える。**ただし validrect の中央
//! `((30 + 350) / 2, (8 + 210) / 2) = (190, 109)` のように、X だけが偶然一致する候補が生じうる
//! 選び方もある**——本ファイルの矩形は、下の前提の檻が列挙する取り違え候補すべてが
//! **両軸とも**画像中央と相異なるように選んである。
//!
//! 行矩形は `(left, top, right, bottom)`。縦書きでは行＝列であり、`left`／`right` が列の位置
//! （行送り軸）、`top`／`bottom` が列内の字送り範囲（行内軸）になる。

use super::test_support::{IMAGE, inline_positions, model_rect};
use super::{FixedMetrics, GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;

/// 共通前提の文字高さ。
const FONT: f32 = 10.0;
/// 全角 'あ' の行内送り（`FixedMetrics`・前提の檻で実測と突合する）。
const ADVANCE: f32 = 10.0;
/// 行送りピッチ `10 + 行間 2`（前提の檻で実測と突合する）。
const PITCH: f32 = 12.0;

/// 弁別 fixture の validrect（`left` / `top` / `right` / `bottom`・image px）。
const VALID: (f32, f32, f32, f32) = (30.0, 8.0, 350.0, 210.0);

/// design.md 検証表 V6 の期待値そのもの（バルーン画像原寸 `(400, 224)` の半分）。
const IMAGE_CENTER: (f32, f32) = (200.0, 112.0);

/// design.md 検証表 V7 の宣言された文字描画開始点。
const DECLARED_ORIGIN: (f32, f32) = (50.0, 20.0);

/// 3 書字方向（テストの反復用）。
const MODES: [WritingMode; 3] = [
    WritingMode::HorizontalTb,
    WritingMode::VerticalRl,
    WritingMode::VerticalLr,
];

/// **弁別 fixture・`origin` 未宣言**: validrect が画像の部分矩形で、`origin` 成分は
/// 書字開始角へ縮退する（横書き・`vertical_lr` は `(left, top)`・`vertical_rl` は `(right, top)`）。
fn region_undeclared(mode: WritingMode) -> TextRegion {
    let region = TextRegion::resolve(
        &model_rect(
            (None, None),
            (
                Some(VALID.1 as i32),
                Some(VALID.3 as i32),
                Some(VALID.0 as i32),
                Some(VALID.2 as i32),
            ),
        ),
        IMAGE,
        mode,
    );
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        VALID,
        "fixture が意図した validrect になっていること"
    );
    region
}

/// **弁別 fixture・`origin` 宣言あり**: 上と同じ validrect に、文字描画開始点を角と異なる位置
/// `(50, 20)` として宣言したバルーン（V7）。
fn region_declared(mode: WritingMode) -> TextRegion {
    let region = TextRegion::resolve(
        &model_rect(
            (
                Some(DECLARED_ORIGIN.0 as i32),
                Some(DECLARED_ORIGIN.1 as i32),
            ),
            (
                Some(VALID.1 as i32),
                Some(VALID.3 as i32),
                Some(VALID.0 as i32),
                Some(VALID.2 as i32),
            ),
        ),
        IMAGE,
        mode,
    );
    assert_eq!(
        (region.left(), region.top(), region.right(), region.bottom()),
        VALID,
        "fixture が意図した validrect になっていること"
    );
    region
}

/// 指定方向でレイアウトを通す。
fn layout_in(
    items: &[TextItem],
    visible: usize,
    region: &TextRegion,
    mode: WritingMode,
) -> Vec<PositionedLine> {
    LayoutEngine::layout(
        items,
        visible,
        region,
        mode,
        FONT,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// 行矩形を `(left, top, right, bottom)` で取り出す。
fn rect_of(line: &PositionedLine) -> (f32, f32, f32, f32) {
    let r = &line.rect;
    (r.left, r.top, r.right, r.bottom)
}

/// 行の**着地点を画像座標 (x, y) へ戻す**（書字方向ごとの逆写像）。
///
/// 行内軸の座標は先頭グリフの `inline_pos`、行送り軸の座標は行矩形の「書字開始側の辺」から
/// 取る——`finish_line` の列矩形が `vertical_rl` では `left = block_pos − font_height`・
/// `right = block_pos`、`vertical_lr` では `left = block_pos`、横書きでは `top = block_pos` で
/// あることの裏返しである。
///
/// この逆写像だけを主張の根拠にはしない（配線の軸写像と表を共有しうるため）。各テストは
/// **行矩形そのものの literal** も併せて固定し、本関数は「3 方向で同じ画像座標に着く」という
/// 方向非依存の主張にだけ使う。
fn landing_of(line: &PositionedLine, mode: WritingMode) -> (f32, f32) {
    let inline = inline_positions(line)[0];
    match mode {
        WritingMode::HorizontalTb => (inline, line.rect.top),
        WritingMode::VerticalRl => (line.rect.right, inline),
        WritingMode::VerticalLr => (line.rect.left, inline),
    }
}

/// 全角グリフ 1 個（'あ'・advance 10）。
fn glyph() -> TextItem {
    TextItem::Glyph { ch: 'あ' }
}

/// `\_l[centerx,centery]`。
fn cursor_center_both() -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::CenterX,
        y: CursorCoord::CenterY,
    }
}

/// `\_l[x,y]`（両軸とも絶対 px）。
fn cursor_px(x: f32, y: f32) -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Absolute {
            value: x,
            unit: CursorUnit::Px,
        },
        y: CursorCoord::Absolute {
            value: y,
            unit: CursorUnit::Px,
        },
    }
}

// ─────────────────────────────────────────────────────────────────────
// 前提の檻（3.3 の二段構えと同型・4.4 の Suggestion「弁別性の前提そのものを檻に入れる」）
// ─────────────────────────────────────────────────────────────────────

/// 弁別 fixture が「基点の取り違えを弁別できる」形を保っていることを、**性質として**固定する。
///
/// 以降の V6・V7 はすべて「基準を取り違えた実装が赤になる」ことに依存している。その依存の
/// 土台（候補が互いに相異なること）が黙って崩れると、値の逐語は緑のまま主張だけが空洞に
/// なる——3.3 が `centerx` の弁別性を定数 1 行で失った事例と同型である。
///
/// 檻に入れる性質は 2 つ:
///
/// 1. **画像中央 (200, 112) が、取り違えうる候補のどれとも両軸で相異なる。** 列挙した候補は
///    ⑴ validrect の遠辺の半分 `(right/2, bottom/2)`＝**4.1／4.4 が名指しした摂動そのもの**
///    ⑵ validrect の近辺の半分 `(left/2, top/2)` ⑶ validrect の**寸法**の半分
///    `((right−left)/2, (bottom−top)/2)` ⑷ validrect の**中央** `((left+right)/2, (top+bottom)/2)`
///    ⑸ validrect の 4 隅 ⑹ 画像の左上 `(0, 0)` ⑺ 3 方向の書字開始角 ⑻ 宣言された `origin`。
/// 2. **宣言された `origin (50, 20)` が、3 方向の書字開始角のどれとも両軸で相異なる。**
///    ここが潰れると V7 の「宣言値から測る」が「角から測る」と区別できない。
#[test]
fn the_center_and_origin_fixture_keeps_every_basepoint_candidate_apart() {
    let (left, top, right, bottom) = VALID;
    let htb = region_undeclared(WritingMode::HorizontalTb);
    let vrl = region_undeclared(WritingMode::VerticalRl);
    let vlr = region_undeclared(WritingMode::VerticalLr);

    // ⑴ 前提そのもの: 画像原寸と計量。
    assert_eq!(
        htb.image_size(),
        (400.0, 224.0),
        "バルーン画像原寸（V6 の literal 200 / 112 の出どころ）"
    );
    assert_eq!(
        IMAGE_CENTER,
        (htb.image_size().0 / 2.0, htb.image_size().1 / 2.0),
        "V6 の期待値は画像原寸の半分である（design.md 解決表 `centerx` / `centery` の行）"
    );
    assert_eq!(FixedMetrics.advance('あ', FONT), ADVANCE);
    assert_eq!(FixedMetrics.line_pitch(FONT), PITCH);
    assert_ne!(
        PITCH, ADVANCE,
        "列送り幅と字送り幅が同値だと、軸の取り違えが着地に現れない"
    );

    // ⑵ 書字開始角（未宣言）が方向で分かれている＝方向を取り違えた実装が弁別できる。
    assert_eq!(htb.start(), (left, top), "horizontal_tb は (left, top)");
    assert_eq!(vrl.start(), (right, top), "vertical_rl は (right, top)");
    assert_eq!(vlr.start(), (left, top), "vertical_lr は (left, top)");
    assert_ne!(
        vrl.start().0,
        vlr.start().0,
        "2 つの縦書きの書字開始角が行送り軸で相異なる"
    );

    // ⑶ 画像中央が、取り違えうる候補のどれとも**両軸で**相異なる。
    let candidates: [(&str, (f32, f32)); 11] = [
        (
            "validrect の遠辺の半分（4.1/4.4 が名指しした摂動）",
            (right / 2.0, bottom / 2.0),
        ),
        ("validrect の近辺の半分", (left / 2.0, top / 2.0)),
        (
            "validrect の寸法の半分",
            ((right - left) / 2.0, (bottom - top) / 2.0),
        ),
        (
            "validrect の中央",
            ((left + right) / 2.0, (top + bottom) / 2.0),
        ),
        ("validrect の左上", (left, top)),
        ("validrect の右下", (right, bottom)),
        ("validrect の右上", (right, top)),
        ("validrect の左下", (left, bottom)),
        ("画像の左上", (0.0, 0.0)),
        ("vertical_rl の書字開始角", vrl.start()),
        ("宣言された origin", DECLARED_ORIGIN),
    ];
    for (name, cand) in candidates {
        assert_ne!(
            IMAGE_CENTER.0, cand.0,
            "画像中央 X が「{name}」の X と同値では、基準の取り違えが弁別できない"
        );
        assert_ne!(
            IMAGE_CENTER.1, cand.1,
            "画像中央 Y が「{name}」の Y と同値では、基準の取り違えが弁別できない"
        );
    }

    // ⑷ 宣言された origin が 3 方向の書字開始角のどれとも両軸で相異なる（V7 の弁別性）。
    for (name, corner) in [
        ("horizontal_tb", htb.start()),
        ("vertical_rl", vrl.start()),
        ("vertical_lr", vlr.start()),
    ] {
        assert_ne!(
            DECLARED_ORIGIN.0, corner.0,
            "{name}: 宣言 origin.x が書字開始角の X と同値では「宣言値から測る」が主張できない"
        );
        assert_ne!(
            DECLARED_ORIGIN.1, corner.1,
            "{name}: 宣言 origin.y が書字開始角の Y と同値では「宣言値から測る」が主張できない"
        );
    }

    // ⑸ 宣言バルーンでは 3 方向とも start() が宣言値そのもの（角へ寄せない・R2.9）。
    for mode in MODES {
        assert_eq!(
            region_declared(mode).start(),
            DECLARED_ORIGIN,
            "{mode:?}: 宣言された origin 成分は字義どおり（書字開始角へ縮退しない）"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// V6: `\_l[centerx,centery]` は書字方向に依らず画像の幅／高さの半分
// ─────────────────────────────────────────────────────────────────────

/// **V6**: `\_l[centerx,centery]ああ` は 3 書字方向とも画像座標 **(200, 112)** へ着く
/// （Requirement 4.1〜4.4・design.md 解決表「`centerx` on X → `image_size.0 / 2`」
/// 「`centery` on Y → `image_size.1 / 2`」）。
///
/// 手計算（弁別 fixture・画像原寸 `(400, 224)`・font 10・advance 10）:
/// - X ＝ `image_size.0 / 2 = 400 / 2 = 200`（**validrect の右辺 350 の半分 175 ではない**）
/// - Y ＝ `image_size.1 / 2 = 224 / 2 = 112`（**validrect の下辺 210 の半分 105 ではない**）
///
/// 方向ごとの行矩形（`finish_line` の列矩形規約から導く。グリフを **2 個**置くのは、
/// 1 個だけだと `horizontal_tb` と `vertical_lr` の行矩形が同一の 10×10 になってしまい
/// 2 方向を弁別できないため——2 個目の送り先が横書きは +x・`vertical_lr` は +y に分かれる）:
/// - `horizontal_tb`: 行内＝x・行送り＝y → `(200, 112, 200+10+10, 112+10) = (200, 112, 220, 122)`
/// - `vertical_rl`: 行内＝y・行送り＝x・列矩形は `[block−font, block]` → `(190, 112, 200, 132)`
/// - `vertical_lr`: 行内＝y・行送り＝x・列矩形は `[block, block+font]` → `(200, 112, 210, 132)`
///
/// 3 方向で行矩形は**異なる**（方向が本当に別の regime であることの証跡）のに、逆写像した
/// 画像座標は**同一**である——これが「方向に依らない」の観測点そのものである
/// （Requirement 4.4）。
#[test]
fn center_resolves_to_half_the_balloon_image_in_all_three_writing_modes() {
    let items = [cursor_center_both(), glyph(), glyph()];

    let expected_rects = [
        (WritingMode::HorizontalTb, (200.0, 112.0, 220.0, 122.0)),
        (WritingMode::VerticalRl, (190.0, 112.0, 200.0, 132.0)),
        (WritingMode::VerticalLr, (200.0, 112.0, 210.0, 132.0)),
    ];
    let mut landings = Vec::new();
    for (mode, expected) in expected_rects {
        let region = region_undeclared(mode);
        let lines = layout_in(&items, 2, &region, mode);
        assert_eq!(lines.len(), 1, "{mode:?}: 移動 1 回＝実体化は 1 行");
        assert_eq!(
            rect_of(&lines[0]),
            expected,
            "{mode:?}: 画像中央 (200, 112) から 2 グリフぶん伸びた行矩形"
        );
        landings.push((mode, landing_of(&lines[0], mode)));
    }

    for (mode, landing) in &landings {
        assert_eq!(
            *landing, IMAGE_CENTER,
            "{mode:?}: 着地は画像の幅／高さの半分（validrect の辺・寸法・中央のどれでもない）"
        );
    }
    assert!(
        landings.windows(2).all(|w| w[0].1 == w[1].1),
        "3 方向の着地が同一であること＝`centerx` / `centery` は書字方向に依らない（R4.4）"
    );
    assert_eq!(
        expected_rects
            .iter()
            .map(|(_, r)| (r.0.to_bits(), r.1.to_bits(), r.2.to_bits(), r.3.to_bits()))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3,
        "同じ着地点でも行矩形は 3 方向で相異なる（方向の取り違えが行矩形に現れる）"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Requirement 4.5: 両軸に異なる書式が混在するとき、軸ごとに独立に解決される
// ─────────────────────────────────────────────────────────────────────

/// **軸ごと独立の解決（Requirement 4.5・2.2）**: 中央指定と他の書式を両軸に混ぜても、
/// 各軸がそれぞれの基準（画像／文字描画開始点／実効位置）で独立に解決される。
///
/// 3 つの混在形を 3 方向で見る。手計算はいずれも design.md 解決表の
/// `基点 + 値 × 係数` から:
///
/// **⒜ `[あ, \_l[centerx,@1em], あ]`**（X ＝画像・Y ＝実効位置からの相対）
/// - X ＝ `image_size.0 / 2 = 200`（3 方向とも同じ）
/// - Y ＝ `実効位置.y + 1 × font_height(10)`。実効位置は 1 個目のグリフを置いた後の
///   「次に文字が置かれる位置」で、**書字方向で Y の意味が変わる**——横書きの Y は行送り軸
///   （行頭のまま `top = 8`）、縦書きの Y は行内軸（1 文字ぶん進んで `8 + 10 = 18`）。
///   よって Y ＝ 横書き `8 + 10 = 18`・縦書き `18 + 10 = 28`。
///
/// **⒝ `[あ, \_l[@0,centery], あ]`**（X ＝実効位置からの相対 0・Y ＝画像）
/// - Y ＝ `image_size.1 / 2 = 112`（3 方向とも同じ）
/// - X ＝ `実効位置.x`。横書きの X は行内軸（`30 + 10 = 40`）、縦書きの X は行送り軸
///   （列は未送りなので書字開始角のまま＝`vertical_rl` は 350・`vertical_lr` は 30）。
///
/// **⒞ `[\_l[centerx,3lh], あ]`**（X ＝画像・Y ＝**文字描画開始点**からの絶対）
/// - X ＝ 200・Y ＝ `origin.y + 3 × line_pitch(12) = 8 + 36 = 44`（3 方向とも `top = 8`）。
///   1 つのタグの中で**基点が 2 つ**（画像と文字描画開始点）使われることの観測点。
///
/// ⒝ は X の着地が 3 方向で `40 / 350 / 30` と**すべて相異なる**ので、書字方向を取り違えた
/// 実装は必ず赤になる——V6 だけでは「3 方向とも同じ値」ゆえ方向の取り違えを弁別できない
/// （中央指定は定義上 方向非依存だからである）。**中央指定でない軸を混ぜることが、
/// 「方向に依らない」という主張を空洞にしないための条件**である。
#[test]
fn mixed_formats_on_the_two_axes_resolve_independently_in_all_three_writing_modes() {
    // ⒜ centerx（画像）× @1em（実効位置からの相対）。
    let a_items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::CenterX,
            y: CursorCoord::Relative {
                value: 1.0,
                unit: CursorUnit::Em,
            },
        },
        glyph(),
    ];
    for (mode, expected_rect, expected_landing) in [
        (
            WritingMode::HorizontalTb,
            (200.0, 18.0, 210.0, 28.0),
            (200.0, 18.0),
        ),
        (
            WritingMode::VerticalRl,
            (190.0, 28.0, 200.0, 38.0),
            (200.0, 28.0),
        ),
        (
            WritingMode::VerticalLr,
            (200.0, 28.0, 210.0, 38.0),
            (200.0, 28.0),
        ),
    ] {
        let region = region_undeclared(mode);
        let lines = layout_in(&a_items, 2, &region, mode);
        assert_eq!(lines.len(), 2, "{mode:?}: 移動が成立＝\\_l は行の分割点");
        assert_eq!(rect_of(&lines[1]), expected_rect, "{mode:?}: ⒜ の行矩形");
        assert_eq!(
            landing_of(&lines[1], mode),
            expected_landing,
            "{mode:?}: ⒜ X は画像中央 200・Y は実効位置 + font_height(10)"
        );
    }

    // ⒝ @0（実効位置）× centery（画像）。X の着地が 3 方向ですべて相異なる。
    let b_items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Relative {
                value: 0.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::CenterY,
        },
        glyph(),
    ];
    let mut b_landings = Vec::new();
    for (mode, expected_rect, expected_landing) in [
        (
            WritingMode::HorizontalTb,
            (40.0, 112.0, 50.0, 122.0),
            (40.0, 112.0),
        ),
        (
            WritingMode::VerticalRl,
            (340.0, 112.0, 350.0, 122.0),
            (350.0, 112.0),
        ),
        (
            WritingMode::VerticalLr,
            (30.0, 112.0, 40.0, 122.0),
            (30.0, 112.0),
        ),
    ] {
        let region = region_undeclared(mode);
        let lines = layout_in(&b_items, 2, &region, mode);
        assert_eq!(lines.len(), 2);
        assert_eq!(rect_of(&lines[1]), expected_rect, "{mode:?}: ⒝ の行矩形");
        assert_eq!(
            landing_of(&lines[1], mode),
            expected_landing,
            "{mode:?}: ⒝ Y は画像中央 112・X は実効位置（方向で意味が変わる軸）"
        );
        b_landings.push(expected_landing);
    }
    assert!(
        b_landings.iter().all(|l| l.1 == IMAGE_CENTER.1),
        "⒝ の Y は 3 方向とも画像中央 112（centery は方向に依らない）"
    );
    assert_eq!(
        b_landings
            .iter()
            .map(|l| l.0.to_bits())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3,
        "⒝ の X は 3 方向ですべて相異なる（40 / 350 / 30）——方向を取り違えた実装が赤になる"
    );

    // ⒞ centerx（画像）× 3lh（文字描画開始点からの絶対）＝1 つのタグで基点が 2 つ。
    let c_items = [
        TextItem::CursorMove {
            x: CursorCoord::CenterX,
            y: CursorCoord::Absolute {
                value: 3.0,
                unit: CursorUnit::Lh,
            },
        },
        glyph(),
    ];
    for (mode, expected_rect) in [
        (WritingMode::HorizontalTb, (200.0, 44.0, 210.0, 54.0)),
        (WritingMode::VerticalRl, (190.0, 44.0, 200.0, 54.0)),
        (WritingMode::VerticalLr, (200.0, 44.0, 210.0, 54.0)),
    ] {
        let region = region_undeclared(mode);
        let lines = layout_in(&c_items, 1, &region, mode);
        assert_eq!(lines.len(), 1);
        assert_eq!(rect_of(&lines[0]), expected_rect, "{mode:?}: ⒞ の行矩形");
        assert_eq!(
            landing_of(&lines[0], mode),
            (IMAGE_CENTER.0, VALID.1 + 3.0 * PITCH),
            "{mode:?}: ⒞ X は画像中央 200・Y は origin.y(8) + 3 × line_pitch(12) = 44"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// V7: 宣言された文字描画開始点が 3 方向とも絶対座標の基点になる
// ─────────────────────────────────────────────────────────────────────

/// **V7**: 文字描画開始点を角と異なる位置 `(50, 20)` に**宣言**したバルーンでは、`\_l[0,0]` が
/// 3 書字方向とも**宣言値から**測られる（Requirement 2.9・design.md 原点表「宣言された
/// `origin` 成分はそのまま原点になる」）。
///
/// 手計算（宣言 `origin (50, 20)`・validrect `left 30 / top 8 / right 350 / bottom 210`・font 10）:
/// - `horizontal_tb`: 着地 `(50 + 0, 20 + 0) = (50, 20)` → 行矩形 `(50, 20, 60, 30)`
/// - `vertical_rl`: 行送り軸＝x なので `block_pos = 50`。列矩形は `[block − font, block]`
///   ＝ `[40, 50]`＝**列右端 50**（design.md V7 の逐語）。行内軸＝y は `20` から下へ 10。
///   → 行矩形 `(40, 20, 50, 30)`
/// - `vertical_lr`: 列矩形は `[block, block + font]` ＝ `[50, 60]`＝**列左端 50**（同上）。
///   → 行矩形 `(50, 20, 60, 30)`
///
/// 「列右端／列左端」は `finish_line` の列矩形規約（`VerticalRl` は `left = block_pos − font_height`・
/// `VerticalLr` は `left = block_pos`）から導いたものであって、実装の戻り値の書き写しではない。
///
/// **書字開始角を使う実装はここで必ず赤になる**——未宣言なら角は `horizontal_tb`／`vertical_lr`
/// が `(30, 8)`・`vertical_rl` が `(350, 8)` で、宣言値 `(50, 20)` とは両軸で相異なる（前提の檻 ⑷）。
/// とりわけ `vertical_rl` は 50 対 350 と大きく離れるので、「縦書きだけ角へ戻す」実装も弁別できる。
#[test]
fn declared_origin_is_the_absolute_basis_in_all_three_writing_modes() {
    let items = [cursor_px(0.0, 0.0), glyph()];

    for (mode, expected_rect) in [
        (WritingMode::HorizontalTb, (50.0, 20.0, 60.0, 30.0)),
        (WritingMode::VerticalRl, (40.0, 20.0, 50.0, 30.0)),
        (WritingMode::VerticalLr, (50.0, 20.0, 60.0, 30.0)),
    ] {
        let region = region_declared(mode);
        let lines = layout_in(&items, 1, &region, mode);
        assert_eq!(lines.len(), 1, "{mode:?}: 移動 1 回＝実体化は 1 行");
        assert_eq!(
            rect_of(&lines[0]),
            expected_rect,
            "{mode:?}: 宣言された origin (50, 20) から測った \\_l[0,0] の行矩形"
        );
        assert_eq!(
            landing_of(&lines[0], mode),
            DECLARED_ORIGIN,
            "{mode:?}: 着地は宣言値そのもの（書字開始角でも validrect の辺でも画像中央でもない）"
        );
    }

    // 列の辺としての読み（design.md V7 の逐語「vertical_rl は列右端 50・vertical_lr は列左端 50」）。
    let rl = layout_in(
        &items,
        1,
        &region_declared(WritingMode::VerticalRl),
        WritingMode::VerticalRl,
    );
    assert_eq!(
        rl[0].rect.right, DECLARED_ORIGIN.0,
        "vertical_rl: 列の**右端**が宣言 origin.x = 50（列矩形は [block − font, block]）"
    );
    let lr = layout_in(
        &items,
        1,
        &region_declared(WritingMode::VerticalLr),
        WritingMode::VerticalLr,
    );
    assert_eq!(
        lr[0].rect.left, DECLARED_ORIGIN.0,
        "vertical_lr: 列の**左端**が宣言 origin.x = 50（列矩形は [block, block + font]）"
    );
}

/// **V7 の根拠（Requirement 9.6・2.9）**: 宣言バルーンでは**横書きの着地も変わる**。これは
/// 退行ではなく正典追随である。
///
/// - **本仕様より前の着地（現行値）**: `(30, 8)`＝validrect の左上。旧実装は
///   `cursor_to_image_px(x, region.left(), …)` / `region.top()` を基点にしており
///   （タスク 1.2 以前の `layout.rs`）、ukadoc **2.8.80** の「文字描画範囲左上」に従っていた。
///   宣言された `origin` は基点に一切効かなかった。
/// - **本仕様の着地（正典値）**: `(50, 20)`＝宣言された文字描画開始点。ukadoc **2.8.83** が
///   数値座標の原点を「文字描画開始点（`origin` の位置）」へ改めたためである
///   （requirements.md Requirement 2.1／2.9・開発者裁定 2026-09-02 議題 1）。
/// - **なぜ受け入れるか**: 差分が出るのは `origin` を角と異なる位置に**宣言した**バルーンだけで、
///   未宣言バルーンでは `TextRegion::start()` が書字開始角と一致するため着地は 1 ビットも
///   変わらない（Requirement 2.7・`layout_cursor_wiring_tests.rs` の H1 が固定）。emo2 の
///   実バルーンは `origin` 未宣言なので適合結果は不変である（requirements.md の実測）。
///
/// 本檻は「新しい値である」ことと「古い値**ではない**こと」を両方主張する——後者が無いと、
/// 正典追随という主張が「たまたま今の値」と区別できない。
#[test]
fn declared_origin_changes_the_horizontal_landing_as_canon_following() {
    let mode = WritingMode::HorizontalTb;
    let region = region_declared(mode);
    let lines = layout_in(&[cursor_px(0.0, 0.0), glyph()], 1, &region, mode);
    let landing = landing_of(&lines[0], mode);

    assert_eq!(
        landing, DECLARED_ORIGIN,
        "正典値（ukadoc 2.8.83「文字描画開始点」）＝宣言された origin (50, 20)"
    );
    assert_ne!(
        landing,
        (region.left(), region.top()),
        "旧正典（ukadoc 2.8.80「文字描画範囲左上」）の着地 (30, 8) にはならない——\
         この差分が Requirement 9.6 の「正典追随であって退行ではない」の観測点である"
    );

    // 未宣言バルーンでは同じ入力の着地が書字開始角と一致し、本仕様の前後で変わらない
    // （R2.7。差分が出るのは宣言バルーンだけであることの対照）。
    let undeclared = region_undeclared(mode);
    let lines = layout_in(&[cursor_px(0.0, 0.0), glyph()], 1, &undeclared, mode);
    assert_eq!(
        landing_of(&lines[0], mode),
        (undeclared.left(), undeclared.top()),
        "未宣言バルーンの着地は書字開始角＝validrect の左上のままで、新旧の正典で一致する"
    );
}
