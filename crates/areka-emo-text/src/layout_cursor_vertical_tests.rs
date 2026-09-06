//! 縦書き 2 方向（`vertical_rl`／`vertical_lr`）における `\_l` カーソル移動の着地値を
//! 固定するテスト（タスク 1.1 で新設・Requirements 9.2／9.6／2.7）。
//!
//! # 期待値の由来を行ごとに読み分ける（2 段の書き換えは完了している）
//!
//! 縦書き `\_l` の決定論テストは着手時点で 0 本だった。まず現行の着地をそのまま写し取り
//! （タスク 1.1）、以降の是正で動いた値が**どちらの原因で動いたのか**を行ごとに読み分けられる
//! ようにしてある。書き換えは 2 段で進んだ（design.md Testing Strategy の順序）:
//!
//! - **タスク 1.2（原点の切替）＝済**: 絶対座標の原点を validrect の辺から、解決済みの文字描画
//!   開始点（`TextRegion::start()`）へ移した。動いたのは `vertical_rl` の実導出 3 本の X だけで、
//!   `vertical_lr` と「当時は当該軸が動かなかった形」は 1 件も動いていない。
//! - **タスク 4.1（語彙の解禁）＝済**: 負値絶対・百分率・`@` 相対・`centerx`／`centery` が実導出
//!   へ移った。ここで動いたのは「当時は当該軸が動かなかった形」の値だけで、1.2 で書き換えた
//!   3 本には 1 件も触れていない（2 段の差分は交わらない）。
//!
//! 各テストのコメントには次のいずれかを明記してある——**正典値（1.2 の原点切替による）**／
//! **正典値（4.1 の語彙解禁による）**（いずれも書き換え前の現行値を根拠として併記）／
//! **現行値（正典と一致・是正後も不変）**（一度も書き換わっていない）。
//!
//! # 共通前提（design.md Integration Tests）
//!
//! `FixedMetrics`・`font_height = 10`（全角 'あ' の advance 10・`line_pitch = 10 + 行間 2 = 12`）・
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

/// 共通前提の文字高さ（`line_pitch = 10 + 行間 2 = 12`）。
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
// vertical_rl: 1.1 の時点で既に実導出されていた形（非負の数値・em・lh）
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
/// タスク 4.1 で配線した範囲外記録の口（`cursor_tag::note_out_of_range`）が DEBUG を 1 件残すが、
/// **位置はその記録によって動かされない**（記録は観測だけを担い、クランプはしない）。
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
/// 1.1 の時点で既に実導出されていた 4 形式（非負の数値・省略・`em`・`lh`）のうち `em`／`lh` を覆う。
/// 書き換え前の現行値は X = validrect 左辺(0) + 5em(50) = 50 → 列矩形 `[40, 50]` だった。
/// 原点切替後は X = 書字開始角(400) + 5 × font_height(10) = 450 → 列矩形 `[440, 450]`。
/// Y = 2lh = 2 × line_pitch(12) = 24 → 字送り位置 24（原点の Y 成分は不変ゆえ書き換えなし）。
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
        (440.0, 24.0, 450.0, 34.0),
        "正典値（1.2 の原点切替による）: X = 書字開始角(400) + 5×10 = 450（列矩形 [440, 450]）／\
         Y = 書字開始角の Y 成分(0) + 2×12 = 24（切替の前後で同値）。\
         書き換え前の現行値は [40, 50]（原点が validrect 左辺 0 だったため）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![24.0]);
}

// ─────────────────────────────────────────────────────────────────────
// vertical_rl: 4.1 で解禁された形（負値絶対・百分率・`@` 相対）
// ─────────────────────────────────────────────────────────────────────

/// **正典値（4.1 の語彙解禁による）**: `vertical_rl` の負の X は次の列へ進む。
///
/// 正典では `vertical_rl` の負の X が「次の列」＝自動列送りと同値であり、
/// `\_l[-12,0]` は 2 列目 `[378, 388]` を指す（1 列ぶん＝新しい行送り 12）。
///
/// 書き換え前の現行値は 1 列目 `[390, 400]` だった——旧換算が絶対座標の**非負値のみ**を
/// 実導出し、負の X を当該軸不動として読み捨てていたため、列は書字開始位置
/// （`right = 400`）に留まっていた（「`\_l[-12,0]`（2 列目のつもり）」と「X を書かない」が
/// 同じ結果になっていた）。
///
/// タスク 4.1 で非負ゲートを撤去し、解決層の式 1 本（`位置 = 基点 + 値 × 係数`）へ委譲した
/// ので、X = 書字開始角(400) + (−12) = 388＝列の右端になる。**原点の切替（1.2）に由来する
/// 差分ではない**——原点はこのテストでは切替の前後とも 400 である。
#[test]
fn vertical_rl_negative_x_steps_to_the_next_column() {
    let items = [cursor_px(-12.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalRl);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (378.0, 0.0, 388.0, 10.0),
        "正典値（4.1 の語彙解禁による）: X = 書字開始角(400) + (−12) = 388 → 2 列目 [378, 388]。\
         書き換え前の現行値は [390, 400]（負値が読み捨てられ 1 列目に留まっていた）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
}

/// **正典値（4.1 の語彙解禁による）**: `vertical_rl` の百分率は両軸とも文字高さの割合で動く。
///
/// `\_l[50%,50%]` は正典どおり「文字高さの 50%」＝ 5px の移動になる（係数は
/// `font_height / 100 = 0.1`）。移動が成立するので `\_l` は列の分割点になり、列は 2 本になる。
///
/// 書き換え前の現行値は両軸とも不動で、`\_l` が完全な無効果（列の分割点にすらならない）へ
/// 落ち、2 文字が同じ列に連続して並んでいた。**原点の切替（1.2）に由来する差分ではない**
/// ——旧換算が `Percent` を実導出しなかったことだけが原因である。
#[test]
fn vertical_rl_percent_resolves_from_the_font_height() {
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
        2,
        "正典値（4.1 の語彙解禁による）: 両軸とも移動が成立するので `\\_l` は列の分割点になる\
         （書き換え前は完全無効果ゆえ 1 列だった）"
    );
    assert_eq!(rect_of(&lines[0]), (390.0, 0.0, 400.0, 10.0));
    assert_eq!(
        rect_of(&lines[1]),
        (395.0, 5.0, 405.0, 15.0),
        "正典値（4.1 の語彙解禁による）: X = 書字開始角(400) + 50 × (10/100) = 405（列矩形 [395, 405]）／\
         Y = 書字開始角の Y 成分(0) + 50 × (10/100) = 5。\
         書き換え前の現行値は 1 列目 [390, 400] に 2 文字連続だった"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(inline_positions(&lines[1]), vec![5.0]);
}

/// **正典値（4.1 の語彙解禁による）**: 正典の縦書き記述例 `\_l[@-1lh,0]`（1 列ぶん左の列の
/// 先頭へ）が 2 列目へ進む。
///
/// `@` 相対の基点は現在の文字描画位置で、`vertical_rl` の列位置は列の右端（`block_pos`）
/// ＝ 1 文字目を置いた時点の 400。そこから `−1lh = −12` 進んで 388＝2 列目の右端になり、
/// 自動列送り（`block_pos += −1 × line_pitch`）と同じ値へ着地する。
///
/// 書き換え前の現行値は 1 列目 `[390, 400]` だった——旧換算が `@` 相対（`Relative`）を
/// 実導出しなかったため X が不動で、Y = 0 だけが効いて 2 個目のグリフが 1 個目と
/// **同じ列の同じ位置**に置かれていた（2 本の列矩形が完全に一致する＝重なって見える）。
/// **原点の切替（1.2）に由来する差分ではない**。
#[test]
fn vertical_rl_relative_column_step_moves_one_column_left() {
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
        (378.0, 0.0, 388.0, 10.0),
        "正典値（4.1 の語彙解禁による）: X = 現在の列位置(400) + (−1 × 12) = 388 → 2 列目 [378, 388]。\
         書き換え前の現行値は [390, 400]（`@-1lh` が読み捨てられ 2 文字目が 1 文字目に重なっていた）"
    );
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
}

/// **正典値（4.1 の語彙解禁による）**: 正典の縦書き記述例 `\_l[,@1em]`（字送りを 1 文字ぶん
/// 進める）が字送り軸を 1 文字ぶん進める。
///
/// X は省略＝不動（正典の正常形）。Y は `@` 相対で、基点は現在の字送り位置＝1 文字目の
/// 送り終端 10。そこから `1em = 10` 進んで 20＝1 文字目の直後になる。
///
/// 書き換え前の現行値は「両軸とも不動＝完全無効果」で、`\_l` が列の分割点にもならず
/// 2 文字が隙間なく連続していた。**見た目のグリフ位置は書き換えの前後とも 20 だが、
/// 列が分割される点が異なる**（移動が 1 軸でも成立すれば `\_l` は行の分割点になる）。
/// **原点の切替（1.2）に由来する差分ではない**。
#[test]
fn vertical_rl_relative_inline_step_advances_one_character() {
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
        2,
        "正典値（4.1 の語彙解禁による）: Y 軸の移動が成立するので `\\_l` は列の分割点になる\
         （書き換え前は完全無効果ゆえ 1 列だった）"
    );
    assert_eq!(rect_of(&lines[0]), (390.0, 0.0, 400.0, 10.0));
    assert_eq!(
        rect_of(&lines[1]),
        (390.0, 20.0, 400.0, 30.0),
        "正典値（4.1 の語彙解禁による）: Y = 現在の字送り位置(10) + 1×10 = 20（列は不動 [390, 400]）。\
         書き換え前の現行値は 1 列目 [390, 400] に 2 文字連続（字送り 0・10）だった"
    );
    assert_eq!(inline_positions(&lines[1]), vec![20.0]);
}

// ─────────────────────────────────────────────────────────────────────
// vertical_lr: 1.1 の時点で既に実導出されていた形（非負の数値・em・lh）
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
        (50.0, 24.0, 60.0, 34.0),
        "現行値（正典と一致）: X = 5×10 = 50（列矩形 [50, 60]）／Y = 2×12 = 24"
    );
    assert_eq!(inline_positions(&lines[0]), vec![24.0]);
}

/// **現行値（正典と一致・是正後も不変）**: `vertical_lr` の正の X による列送り。
///
/// `vertical_lr` では列が左から右へ進むので、次の列は正の X（`\_l[12,0]`＝ 1lh ぶん右）で
/// 指せる。これは非負の絶対値ゆえ旧換算でも実導出され、2 列目 `[12, 22]` に着地していた。
/// 同じ位置を相対で書いた `\_l[@1lh,0]`（下のテスト）が 4.1 で同じ `[12, 22]` へ着地するように
/// なったことと対をなす（絶対と相対が同じ列を指せる）。
#[test]
fn vertical_lr_positive_x_steps_to_the_next_column_today() {
    let items = [cursor_px(12.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalLr);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (12.0, 0.0, 22.0, 10.0),
        "現行値（正典と一致）: 2 列目 [12, 22]（line_pitch = 12）"
    );
}

// ─────────────────────────────────────────────────────────────────────
// vertical_lr: 4.1 で解禁された形（負値絶対・百分率・`@` 相対）
// ─────────────────────────────────────────────────────────────────────

/// **正典値（4.1 の語彙解禁による）**: `vertical_lr` の負の X は字義どおり文字描画範囲の
/// 外へ出る（内側へ寄せない）。
///
/// `vertical_lr` の書字開始角は `(left, top) = (0, 0)` なので、`\_l[-12,0]` の X は
/// `0 + (−12) = −12`＝文字描画範囲（0〜400）の左外である。正典は**字義どおり用い、内側への
/// 自動的な寄せを行わない**（Requirement 2.6）ので、列矩形は `[-12, -2]` になる。範囲外なので
/// DEBUG が 1 件記録されるが、**位置はその記録によって動かされない**（記録は観測だけを担い、
/// クランプはしない）。
///
/// 書き換え前の現行値は 1 列目 `[0, 10]` だった（旧換算の非負ゲートが負値を読み捨てて
/// いたため列が書字開始位置 0 のままだった）。**原点の切替（1.2）に由来する差分ではない**
/// ——`vertical_lr` の原点は切替の前後とも `(0, 0)` である。
#[test]
fn vertical_lr_negative_x_lands_literally_outside_the_text_area() {
    let items = [cursor_px(-12.0, 0.0), glyph()];
    let lines = layout_for(&items, 1, WritingMode::VerticalLr);

    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (-12.0, 0.0, -2.0, 10.0),
        "正典値（4.1 の語彙解禁による）: X = 書字開始角(0) + (−12) = −12 → 列矩形 [-12, -2]\
         （字義どおり・内側へ寄せない）。書き換え前の現行値は [0, 10]"
    );
}

/// **正典値（4.1 の語彙解禁による）**: `vertical_lr` の百分率は両軸とも文字高さの割合で動く
/// （`vertical_rl` の鏡像）。書き換え前は両軸とも不動で列も分割しなかった。
#[test]
fn vertical_lr_percent_resolves_from_the_font_height() {
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
        2,
        "正典値（4.1 の語彙解禁による）: 両軸とも移動が成立するので `\\_l` は列の分割点になる\
         （書き換え前は完全無効果ゆえ 1 列だった）"
    );
    assert_eq!(rect_of(&lines[0]), (0.0, 0.0, 10.0, 10.0));
    assert_eq!(
        rect_of(&lines[1]),
        (5.0, 5.0, 15.0, 15.0),
        "正典値（4.1 の語彙解禁による）: X = 書字開始角(0) + 50 × (10/100) = 5（列矩形 [5, 15]）／\
         Y = 5。書き換え前の現行値は 1 列目 [0, 10] に 2 文字連続だった"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(inline_positions(&lines[1]), vec![5.0]);
}

/// **正典値（4.1 の語彙解禁による）**: `vertical_lr` の正典記述例の鏡像 `\_l[@1lh,0]`
/// （次の列へ）が 2 列目へ進む。
///
/// 同じ位置を絶対値で書いた `\_l[12,0]`（上のテスト）と同じ 2 列目 `[12, 22]` に着地する
/// ——絶対と相対が同じ位置を指せることが、`@` 相対の解禁の意味そのものである。
/// 書き換え前の現行値は 1 列目 `[0, 10]` で、受理はされるのに動かない形の典型だった。
/// **原点の切替（1.2）に由来する差分ではない**。
#[test]
fn vertical_lr_relative_column_step_moves_to_the_next_column() {
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
        (12.0, 0.0, 22.0, 10.0),
        "正典値（4.1 の語彙解禁による）: X = 現在の列位置(0) + 1 × 12 = 12 → 2 列目 [12, 22]。\
         書き換え前の現行値は [0, 10]（`@1lh` が読み捨てられ 2 文字目が 1 文字目に重なっていた）"
    );
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
}

/// **正典値（4.1 の語彙解禁による）**: `vertical_lr` の `\_l[,@1em]`（字送りを 1 文字ぶん
/// 進める）が字送り軸を 1 文字ぶん進める（`vertical_rl` の鏡像）。
/// 書き換え前は両軸とも不動＝完全な無効果だった。
#[test]
fn vertical_lr_relative_inline_step_advances_one_character() {
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
        2,
        "正典値（4.1 の語彙解禁による）: Y 軸の移動が成立するので `\\_l` は列の分割点になる\
         （書き換え前は完全無効果ゆえ 1 列だった）"
    );
    assert_eq!(rect_of(&lines[0]), (0.0, 0.0, 10.0, 10.0));
    assert_eq!(
        rect_of(&lines[1]),
        (0.0, 20.0, 10.0, 30.0),
        "正典値（4.1 の語彙解禁による）: Y = 現在の字送り位置(10) + 1×10 = 20（列は不動 [0, 10]）。\
         書き換え前の現行値は 1 列目 [0, 10] に 2 文字連続（字送り 0・10）だった"
    );
    assert_eq!(inline_positions(&lines[1]), vec![20.0]);
}
