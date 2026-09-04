//! 縦書き 2 方向（`vertical_rl`／`vertical_lr`）における `\_l` カーソル移動の着地値を
//! 固定するテスト（タスク 1.1 で新設・Requirements 9.2／9.6／2.7）。
//!
//! # 期待値は「現行値」と「正典値」が混在する（行ごとに読み分ける）
//!
//! 縦書き `\_l` の決定論テストは着手時点で 0 本だった。まず現行の着地をそのまま写し取り
//! （タスク 1.1）、以降の是正で動いた値が**どちらの原因で動いたのか**を行ごとに読み分けられる
//! ようにしてある。書き換えは 2 段で進む（design.md Testing Strategy の順序）:
//!
//! - **タスク 1.2（原点の切替）＝済**: 絶対座標の原点を validrect の辺から、解決済みの文字描画
//!   開始点（`TextRegion::start()`）へ移した。動いたのは `vertical_rl` の実導出 3 本の X だけで、
//!   `vertical_lr` と「現行では当該軸が動かない形」は 1 件も動いていない。
//! - **タスク 4.1（語彙の解禁）＝未**: 負値絶対・百分率・`@` 相対・`centerx`／`centery` が実導出
//!   へ移る。ここで動くのは「現行では当該軸が動かない形」の値。
//!
//! 各テストのコメントには次のいずれかを明記してある——**正典値（1.2 の原点切替による）**（書き換え済み・
//! 書き換え前の現行値を根拠として併記）／**現行値（欠陥の証跡）**（4.1 で書き換わる）／
//! **現行値（正典と一致・是正後も不変）**（書き換わらない）。
//!
//! # 共通前提（design.md Integration Tests）
//!
//! `FixedMetrics`・`font_height = 10`（全角 'あ' の advance 10・`line_pitch = ceil(10 × 1.25) = 13`）・
//! バルーン画像原寸 `IMAGE = (400, 224)`・validrect は画像全域（`left/top/right/bottom = 0/0/400/224`）・
//! **`origin` は未宣言**（`\_l` の非回帰対象＝Requirement 2.7 が保護する条件そのもの）。
//! 未宣言ゆえ書字開始点は書字開始角へ縮退する——`vertical_rl` は `(right, top) = (400, 0)`・
//! `vertical_lr` は `(left, top) = (0, 0)`（`region.rs` の書字開始角正準表）。
//!
//! 行矩形は `(left, top, right, bottom)` の 4 つ組で読む。縦書きでは行＝列であり、
//! `left`／`right` が列の位置（行送り軸）、`top`／`bottom` が列内の字送り範囲（行内軸）になる。

use super::test_support::{IMAGE, inline_positions, model};
use super::{FixedMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;

/// 共通前提の文字高さ（`line_pitch = ceil(10 × 1.25) = 13`）。
const FONT: f32 = 10.0;

/// 行矩形を `(left, top, right, bottom)` の 4 つ組で取り出す。
fn rect_of(line: &PositionedLine) -> (f32, f32, f32, f32) {
    let r = &line.rect;
    (r.left, r.top, r.right, r.bottom)
}

/// `origin` 未宣言・validrect 全域・`FixedMetrics`・font 10 でレイアウトを通す。
fn layout_for(items: &[TextItem], visible: usize, mode: WritingMode) -> Vec<PositionedLine> {
    let region = TextRegion::resolve(&model((None, None), (None, None)), IMAGE, mode);
    LayoutEngine::layout(
        items,
        visible,
        &region,
        mode,
        FONT,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// 絶対 px の `\_l[x,y]`。
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

/// 全角グリフ 1 個。
fn glyph() -> TextItem {
    TextItem::Glyph { ch: 'あ' }
}

// ─────────────────────────────────────────────────────────────────────
// vertical_rl: 現行で実導出される形（非負の数値・em・lh）
// ─────────────────────────────────────────────────────────────────────

/// **正典値（1.2 の原点切替による）**: `vertical_rl` の `\_l[0,0]` は 1 列目に着地する。
///
/// 書き換え前の現行値は列矩形 `[-10, 0]`＝**文字描画範囲（0〜400）の外側左方**だった。
/// 絶対座標の原点が validrect の辺（X は `left = 0`）だったため X = 0 が画像左端を指し、
/// `vertical_rl` の列矩形は「列の右端＝行送り位置」から font_height ぶん左へ伸びるので、
/// 範囲外へ落ちていた。
///
/// タスク 1.2 で原点を解決済みの文字描画開始点（`TextRegion::start()`。`origin` 未宣言ゆえ
/// 書字開始角 `(right, top) = (400, 0)` へ縮退）へ切り替えたので、X = 0 は 400 を指し、
/// 列矩形は 1 列目 `[390, 400]` になる——正典（SSP 2.8.83・Requirement 2.3）の着地そのもの。
/// Y は原点の Y 成分が `top = 0` のままで切替の前後が同値ゆえ書き換えていない。
#[test]
fn vertical_rl_zero_zero_lands_on_the_first_column() {
    let items = [cursor_px(0.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalRl);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (390.0, 0.0, 400.0, 10.0),
        "正典値（1.2 の原点切替による）: X = 書字開始角(400) + 0 → 1 列目 [390, 400]。\
         書き換え前の現行値は [-10, 0]（原点が validrect 左辺 0 だったため）"
    );
    assert_eq!(
        inline_positions(&lines[0]),
        vec![0.0],
        "現行値（正典と一致）: Y = 0 は字送り軸の先頭＝列の上端"
    );
}

/// **正典値（1.2 の原点切替による）**: `vertical_rl` の `\_l[10,10]`。
///
/// 書き換え前の現行値は X = validrect 左辺 0 から右へ 10 → 列矩形 `[0, 10]` だった。
/// 原点切替後は X = 書字開始角 400 から右へ 10 → 列の右端 410・列矩形 `[400, 410]`。
///
/// この列矩形は文字描画範囲（0〜400）の**右外**へ出るが、これは「原点＝右上・X 正＝右」という
/// 正典の帰結であって欠陥ではない（`vertical_rl` で意味のある指定は負の X＝次の列方向）。
/// 範囲外の DEBUG 記録口はまだ存在しない（タスク 3.2／4.1 の担当）ので、ここでは位置だけを固定する。
/// Y = 10 は字送り軸の 10 で、原点の Y 成分（`top = 0`）は切替の前後が同値ゆえ書き換えていない。
#[test]
fn vertical_rl_absolute_ten_ten_measures_x_from_text_start() {
    let items = [cursor_px(10.0, 10.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalRl);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (400.0, 10.0, 410.0, 20.0),
        "正典値（1.2 の原点切替による）: 列の右端＝書字開始角(400) + 10 → 列矩形 [400, 410]。\
         書き換え前の現行値は [0, 10]（原点が validrect 左辺 0 だったため）"
    );
    assert_eq!(
        inline_positions(&lines[0]),
        vec![10.0],
        "現行値（正典と一致）: 字送り位置＝書字開始角の Y 成分(0) + 10（切替の前後で同値）"
    );
}

/// **正典値（1.2 の原点切替による）**: `vertical_rl` の `\_l[5em,2lh]`。
///
/// emo2 の適合フィクスチャが実際に使っている形（`menu.pasta` の 3 箇所）と同じ書式で、
/// 現行で実導出される 4 形式（非負の数値・省略・`em`・`lh`）のうち `em`／`lh` を覆う。
/// 書き換え前の現行値は X = validrect 左辺(0) + 5em(50) = 50 → 列矩形 `[40, 50]` だった。
/// 原点切替後は X = 書字開始角(400) + 5 × font_height(10) = 450 → 列矩形 `[440, 450]`。
/// Y = 2lh = 2 × line_pitch(13) = 26 → 字送り位置 26（原点の Y 成分は不変ゆえ書き換えなし）。
#[test]
fn vertical_rl_em_and_lh_units_resolve_from_text_start() {
    let items = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Em,
            },
            y: CursorCoord::Absolute {
                value: 2.0,
                unit: CursorUnit::Lh,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 1, WritingMode::VerticalRl);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (440.0, 26.0, 450.0, 36.0),
        "正典値（1.2 の原点切替による）: X = 書字開始角(400) + 5×10 = 450（列矩形 [440, 450]）／\
         Y = 書字開始角の Y 成分(0) + 2×13 = 26（切替の前後で同値）。\
         書き換え前の現行値は [40, 50]（原点が validrect 左辺 0 だったため）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![26.0]);
}

// ─────────────────────────────────────────────────────────────────────
// vertical_rl: 現行では当該軸が動かない形（負値絶対・百分率・`@` 相対）
// ─────────────────────────────────────────────────────────────────────

/// **現行値（欠陥の証跡）**: `vertical_rl` の負の X は列を動かさない。
///
/// 正典では `vertical_rl` の負の X が「次の列」＝自動列送りと同値であり、
/// `\_l[-13,0]` は 2 列目 `[377, 387]` を指す。現行の換算は絶対座標の**非負値のみ**を
/// 実導出するため、負の X は当該軸不動として読み捨てられる——列は書字開始位置
/// （`right = 400`）に留まり、列矩形は 1 列目 `[390, 400]` のままになる。
///
/// つまり現行では「`\_l[-13,0]`（2 列目のつもり）」と「X を書かない」が同じ結果になる。
/// この行はタスク 4.1（負値の解禁）で `[377, 387]` へ書き換えられる。
#[test]
fn vertical_rl_negative_x_does_not_move_the_column_today() {
    let items = [cursor_px(-13.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalRl);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (390.0, 0.0, 400.0, 10.0),
        "現行値（欠陥の証跡）: 負の X は読み捨てられ、列は書字開始位置(400)のまま＝\
         1 列目 [390, 400]。正典は 2 列目 [377, 387]（タスク 4.1 で書き換わる）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
}

/// **現行値（欠陥の証跡）**: `vertical_rl` の百分率は両軸とも動かず、列も分割しない。
///
/// `\_l[50%,50%]` は正典では「文字高さの 50%」＝ 5px の移動になるが、現行の換算は
/// `Percent` を実導出しないため両軸とも不動となり、`\_l` は完全な無効果へ落ちる
/// （列の分割点にすらならない）。結果、2 文字は同じ列に連続して並ぶ。
/// この行はタスク 4.1（百分率の解禁）で書き換えられる。
#[test]
fn vertical_rl_percent_is_completely_inert_today() {
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Percent,
            },
            y: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Percent,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 2, WritingMode::VerticalRl);

    assert_eq!(
        lines.len(),
        1,
        "現行値（欠陥の証跡）: 両軸とも不動＝完全無効果ゆえ列を分割しない"
    );
    assert_eq!(
        rect_of(&lines[0]),
        (390.0, 0.0, 400.0, 20.0),
        "現行値（欠陥の証跡）: 移動が起きず 1 列目に 2 文字が連続する"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
}

/// **現行値（欠陥の証跡）**: 正典の縦書き記述例 `\_l[@-1lh,0]`（1 列ぶん左の列の先頭へ）が
/// 列を動かさず、2 文字目が 1 文字目に重なる。
///
/// 現行の換算は `@` 相対（`Relative`）を実導出しないため X は不動。Y = 0 は実導出されるので
/// 列内の字送り位置だけが先頭へ戻り、結果として 2 個目のグリフが 1 個目と**同じ列の同じ位置**
/// に置かれる（2 本の列矩形が完全に一致する＝重なって見える）。
/// この行はタスク 4.1（`@` 相対の解禁）で 2 列目 `[377, 387]` へ書き換えられる。
#[test]
fn vertical_rl_relative_column_step_is_inert_and_overlaps_today() {
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Relative {
                value: -1.0,
                unit: CursorUnit::Lh,
            },
            y: CursorCoord::Absolute {
                value: 0.0,
                unit: CursorUnit::Px,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 2, WritingMode::VerticalRl);

    assert_eq!(
        lines.len(),
        2,
        "Y 軸が実導出されるので `\\_l` は列の分割点にはなる"
    );
    assert_eq!(rect_of(&lines[0]), (390.0, 0.0, 400.0, 10.0));
    assert_eq!(
        rect_of(&lines[1]),
        (390.0, 0.0, 400.0, 10.0),
        "現行値（欠陥の証跡）: `@-1lh` が読み捨てられ 2 文字目が 1 文字目に重なる。\
         正典は 2 列目 [377, 387]（タスク 4.1 で書き換わる）"
    );
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
}

/// **現行値（欠陥の証跡）**: 正典の縦書き記述例 `\_l[,@1em]`（字送りを 1 文字ぶん進める）が
/// 完全な無効果になる。
///
/// X は省略＝不動（正典の正常形）、Y は `@` 相対で現行は不動。両軸とも動かないため
/// `\_l` は列の分割点にもならず、2 文字は隙間なく連続する。
/// 正典では 1 文字目の直後（字送り 20）へ進むので見た目の位置は同じになるが、
/// **列が分割される**点が異なる。この行はタスク 4.1 で書き換えられる。
#[test]
fn vertical_rl_relative_inline_step_is_completely_inert_today() {
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Relative {
                value: 1.0,
                unit: CursorUnit::Em,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 2, WritingMode::VerticalRl);

    assert_eq!(
        lines.len(),
        1,
        "現行値（欠陥の証跡）: 両軸とも不動＝完全無効果ゆえ列を分割しない"
    );
    assert_eq!(rect_of(&lines[0]), (390.0, 0.0, 400.0, 20.0));
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
}

// ─────────────────────────────────────────────────────────────────────
// vertical_lr: 現行で実導出される形（非負の数値・em・lh）
// ─────────────────────────────────────────────────────────────────────

/// **現行値（正典と一致・是正後も不変）**: `vertical_lr` の `\_l[0,0]` は 1 列目に着地する。
///
/// `vertical_lr` は書字開始角が `(left, top) = (0, 0)` であり、現行の原点（validrect の
/// `left`／`top`）と一致するため、原点の切替（タスク 1.2）でも値は動かない。
/// Requirement 2.7 が保護する「`vertical_lr` の既存実導出形は不変」の証跡そのものである。
#[test]
fn vertical_lr_zero_zero_lands_on_the_first_column_today() {
    let items = [cursor_px(0.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalLr);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (0.0, 0.0, 10.0, 10.0),
        "現行値（正典と一致）: 1 列目 [0, 10]。書字開始角と現行原点が同一ゆえ 1.2 でも不変"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
}

/// **現行値（正典と一致・是正後も不変）**: `vertical_lr` の `\_l[10,10]`。
#[test]
fn vertical_lr_absolute_ten_ten_today() {
    let items = [cursor_px(10.0, 10.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalLr);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (10.0, 10.0, 20.0, 20.0),
        "現行値（正典と一致）: 列矩形 [10, 20]・字送り 10"
    );
    assert_eq!(inline_positions(&lines[0]), vec![10.0]);
}

/// **現行値（正典と一致・是正後も不変）**: `vertical_lr` の `\_l[5em,2lh]`。
#[test]
fn vertical_lr_em_and_lh_units_today() {
    let items = [
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 5.0,
                unit: CursorUnit::Em,
            },
            y: CursorCoord::Absolute {
                value: 2.0,
                unit: CursorUnit::Lh,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 1, WritingMode::VerticalLr);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (50.0, 26.0, 60.0, 36.0),
        "現行値（正典と一致）: X = 5×10 = 50（列矩形 [50, 60]）／Y = 2×13 = 26"
    );
    assert_eq!(inline_positions(&lines[0]), vec![26.0]);
}

/// **現行値（正典と一致・是正後も不変）**: `vertical_lr` の正の X による列送り。
///
/// `vertical_lr` では列が左から右へ進むので、次の列は正の X（`\_l[13,0]`＝ 1lh ぶん右）で
/// 指せる。これは非負の絶対値ゆえ現行でも実導出され、2 列目 `[13, 23]` に着地する。
/// 同じ位置を相対で書いた `\_l[@1lh,0]` が動かないこと（下のテスト）と対をなす。
#[test]
fn vertical_lr_positive_x_steps_to_the_next_column_today() {
    let items = [cursor_px(13.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalLr);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (13.0, 0.0, 23.0, 10.0),
        "現行値（正典と一致）: 2 列目 [13, 23]（line_pitch = 13）"
    );
}

// ─────────────────────────────────────────────────────────────────────
// vertical_lr: 現行では当該軸が動かない形（負値絶対・百分率・`@` 相対）
// ─────────────────────────────────────────────────────────────────────

/// **現行値（欠陥の証跡）**: `vertical_lr` の負の X は列を動かさない。
///
/// 正典では負の X は字義どおり書字開始点の左（＝文字描画範囲の外）を指し、
/// 範囲外として DEBUG 記録されつつ位置は動かされない。現行は非負ゲートで読み捨てるため、
/// 列は書字開始位置（0）のままで 1 列目 `[0, 10]` に落ちる。タスク 4.1 で書き換えられる。
#[test]
fn vertical_lr_negative_x_does_not_move_the_column_today() {
    let items = [cursor_px(-13.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalLr);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (0.0, 0.0, 10.0, 10.0),
        "現行値（欠陥の証跡）: 負の X は読み捨てられ 1 列目 [0, 10] のまま。\
         正典は字義どおり列矩形 [-13, -3]（タスク 4.1 で書き換わる）"
    );
}

/// **現行値（欠陥の証跡）**: `vertical_lr` の百分率は両軸とも動かず、列も分割しない。
#[test]
fn vertical_lr_percent_is_completely_inert_today() {
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Percent,
            },
            y: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Percent,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 2, WritingMode::VerticalLr);

    assert_eq!(
        lines.len(),
        1,
        "現行値（欠陥の証跡）: 両軸とも不動＝完全無効果ゆえ列を分割しない"
    );
    assert_eq!(rect_of(&lines[0]), (0.0, 0.0, 10.0, 20.0));
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
}

/// **現行値（欠陥の証跡）**: `vertical_lr` の正典記述例の鏡像 `\_l[@1lh,0]`（次の列へ）が
/// 列を動かさず、2 文字目が 1 文字目に重なる。
///
/// 同じ位置を絶対値で書いた `\_l[13,0]`（上のテスト）は 2 列目 `[13, 23]` に着地するのに、
/// 相対で書くと 1 列目 `[0, 10]` のままになる——受理はされるのに動かない形の典型である。
/// タスク 4.1 で `[13, 23]` へ書き換えられる。
#[test]
fn vertical_lr_relative_column_step_is_inert_and_overlaps_today() {
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Relative {
                value: 1.0,
                unit: CursorUnit::Lh,
            },
            y: CursorCoord::Absolute {
                value: 0.0,
                unit: CursorUnit::Px,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 2, WritingMode::VerticalLr);

    assert_eq!(
        lines.len(),
        2,
        "Y 軸が実導出されるので `\\_l` は列の分割点にはなる"
    );
    assert_eq!(rect_of(&lines[0]), (0.0, 0.0, 10.0, 10.0));
    assert_eq!(
        rect_of(&lines[1]),
        (0.0, 0.0, 10.0, 10.0),
        "現行値（欠陥の証跡）: `@1lh` が読み捨てられ 2 文字目が 1 文字目に重なる。\
         正典は 2 列目 [13, 23]（タスク 4.1 で書き換わる）"
    );
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
}

/// **現行値（欠陥の証跡）**: `vertical_lr` の `\_l[,@1em]`（字送りを 1 文字ぶん進める）が
/// 完全な無効果になる（`vertical_rl` の鏡像）。
#[test]
fn vertical_lr_relative_inline_step_is_completely_inert_today() {
    let items = [
        glyph(),
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Relative {
                value: 1.0,
                unit: CursorUnit::Em,
            },
        },
        glyph(),
    ];
    let lines = layout_for(&items, 2, WritingMode::VerticalLr);

    assert_eq!(
        lines.len(),
        1,
        "現行値（欠陥の証跡）: 両軸とも不動＝完全無効果ゆえ列を分割しない"
    );
    assert_eq!(rect_of(&lines[0]), (0.0, 0.0, 10.0, 20.0));
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0]);
}
