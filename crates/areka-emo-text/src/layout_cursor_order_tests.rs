use super::test_support::{IMAGE, inline_positions, model};
use super::{FixedMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;

// ── 書かれた順の適用（DD-11）: `\_l` と `\n` は書かれた順に効く ──
//
// 本ファイルが見るのは**順序だけ**である。主張は「同じ 2 つのタグでも、書いた順が
// 違えば結果が違う」なので、檻は必ず **順方向と逆順を対で**置く——順方向だけを固定
// すると「どちらの順序でも改行を後勝ちにする実装」が素通りしてしまう（4.2 のレビューで
// 得た教訓＝単独ケースの檻をいくら並べても順序は守れない）。
//
// 共通前提は横書きの既存 layout テストと同じ FixedMetrics・font 10（全角 'あ' の
// advance 10・pitch 12＝10 + 行間 2）。origin(0,0)・wordwrappoint 未宣言ゆえ文字描画開始点は (0, 0)・
// 折返し閾値＝画像右辺 400。

/// 横書き・文字描画開始点 (0, 0) の共通前提で layout を回す。
fn layout_h(items: &[TextItem], visible: usize) -> Vec<PositionedLine> {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    LayoutEngine::layout(
        items,
        visible,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// `\n`（ratio 1.0＝1 行ぶんの送り）。
fn newline() -> TextItem {
    TextItem::LineBreak { ratio: 1.0 }
}

/// `\_l[@10,]`（X だけ現在位置から +10px・Y は省略＝動かさない）。
fn cursor_relative_x10() -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Relative {
            value: 10.0,
            unit: CursorUnit::Px,
        },
        y: CursorCoord::Omitted,
    }
}

/// `\_l[,100]`（Y だけ絶対 100px・X は省略＝動かさない）。
fn cursor_absolute_y100() -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Omitted,
        y: CursorCoord::Absolute {
            value: 100.0,
            unit: CursorUnit::Px,
        },
    }
}

/// **正典値（4.3 の書かれた順による・DD-11）**: 検証表 H3b の前者 `あ\_l[@10,]\nあ`。
///
/// 手計算（font 10・pitch 12・文字描画開始点 (0, 0)）:
/// - `あ`: 行内 0 に置かれ、行内位置 10・行送り位置 0。
/// - `\_l[@10,]`: 保留は無いので実効位置は (10, 0)。X = 10 + 10 = 20 を保留する。
/// - `\n`: **保留カーソルがあるので、改行を保留する前に実体化する**——現在行 [あ@0] を
///   確定し、行内位置を 20 にする。そのうえで改行を保留へ積む。
/// - `あ`: フラッシュで改行が効き、行送り 0 + 12 = 12・行内は行頭 0 へ戻る。
///   ゆえに 2 個目は**次行の先頭 (0, 12)**。
///
/// 書き換え前の現行値は **(20, 12)**——改行の到着時にカーソルを保留したままにしていたので、
/// 同一フラッシュの中でカーソルの X=20 が改行の行内リセットに後勝ちしていた。根拠は DD-11
/// （SSP はタグを書かれた順に適用する・設計ディスカッション裁定 2026-09-02）＝Requirement 9.6
/// の「正典追随」であって退行ではない。
///
/// **逆順の対照（不変）**: `あ\n\_l[@10,]あ` は `\n` 到着時に保留カーソルが無いので先行実体化
/// が起きず、従来どおりフラッシュ内で (2) 改行 →(3) カーソル の順に効く。実効位置は改行を
/// 仮適用した (0, 12) なので `@10` は 10 になり、着地は **(10, 12)**。順方向だけを固定すると
/// 「どちらの順序でも改行が後勝ちする」実装が素通りするが、その実装は逆順でも行内 0 を返すので
/// この対照が赤になる。
#[test]
fn written_order_decides_relative_cursor_against_newline() {
    // 順方向 `\_l` → `\n`: 改行が後勝ち（次行の先頭へ）。
    let cursor_then_break = [
        TextItem::Glyph { ch: 'あ' },
        cursor_relative_x10(),
        newline(),
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = layout_h(&cursor_then_break, 2);
    assert_eq!(
        lines.len(),
        2,
        "`\\_l` も `\\n` も行区切りだが行は 2 本のまま"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(
        inline_positions(&lines[1]),
        vec![0.0],
        "正典値（4.3 の書かれた順による・DD-11）: 後に書かれた `\\n` が後勝ちして行頭 0 へ戻る。\
         書き換え前の現行値は 20（保留したままのカーソル X=20 が改行の行内リセットに勝っていた）"
    );
    assert_eq!(
        lines[1].rect.top, 12.0,
        "行送りは改行の送り 12（`\\_l` は Y を動かしていない）"
    );

    // 逆順 `\n` → `\_l`: 従来どおりカーソルが後勝ち（不変）。
    let break_then_cursor = [
        TextItem::Glyph { ch: 'あ' },
        newline(),
        cursor_relative_x10(),
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = layout_h(&break_then_cursor, 2);
    assert_eq!(lines.len(), 2);
    assert_eq!(
        inline_positions(&lines[1]),
        vec![10.0],
        "逆順は不変: `\\n` 到着時に保留カーソルが無いので先行実体化は起きず、実効位置 (0, 12) から\
         `@10` → 10 が改行の行内リセット 0 に後勝ちする"
    );
    assert_eq!(lines[1].rect.top, 12.0);
}

/// **正典値（4.3 の書かれた順による・DD-11）**: 検証表 H3b の後者 `\_l[,100]\nあ`。
///
/// 手計算（pitch 12・文字描画開始点 (0, 0)）:
/// - `\_l[,100]`: Y = 0 + 100 = 100 を保留する（現在行は空）。
/// - `\n`: 保留カーソルがあるので先に実体化する——現在行は空なので**行は作らず**、
///   行送り位置だけを 100 にする。そのうえで改行を保留へ積む。
/// - `あ`: フラッシュで改行が効き、行送り 100 + 12 = **112**。
///
/// 書き換え前の現行値は **100**——改行送り（0 + 12 = 12）にカーソルの Y=100 が後勝ちして
/// 上書きしていた。根拠は DD-11（Requirement 9.6 の正典追随）。
///
/// **逆順の対照（不変）**: `\n\_l[,100]あ` は従来どおり (2) 改行 →(3) カーソル の順で、
/// カーソルの Y=100 が改行送り 12 に後勝ちする＝**100**。順方向と逆順で 112 対 100 と
/// 値が割れることが「書かれた順」の主張そのものである。
#[test]
fn written_order_decides_absolute_cursor_against_newline() {
    // 順方向 `\_l` → `\n`: 改行の送りがカーソルの着地点に**加算**される。
    let cursor_then_break = [
        cursor_absolute_y100(),
        newline(),
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = layout_h(&cursor_then_break, 1);
    assert_eq!(
        lines.len(),
        1,
        "内容の無い先行実体化は行を作らない（末尾規則と同じく空行を出さない）"
    );
    assert_eq!(
        lines[0].rect.top, 112.0,
        "正典値（4.3 の書かれた順による・DD-11）: 先に効いた `\\_l` の 100 へ、後に書かれた\
         `\\n` の送り 12 が乗る。書き換え前の現行値は 100（カーソルが改行送りを上書きしていた）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);

    // 逆順 `\n` → `\_l`: 従来どおりカーソルが改行送りを上書きする（不変）。
    let break_then_cursor = [
        newline(),
        cursor_absolute_y100(),
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = layout_h(&break_then_cursor, 1);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0].rect.top, 100.0,
        "逆順は不変: 改行送り 12 にカーソルの絶対 Y=100 が後勝ちする"
    );
}

/// **正典値（4.3 の書かれた順による・DD-11）**: 混在順 `あ\n\_l[,100]\nあ`（検証表 H3c）。
/// `\_l` の**前**に書かれた改行と**後**に書かれた改行が、どちらも書かれた順に効く。
///
/// 手計算（font 10・pitch 12・文字描画開始点 (0, 0)）:
/// 1. `あ`: 行内 0 に置かれ、行内位置 10・行送り位置 0。
/// 2. `\n`: 保留カーソルが無いので先行実体化は起きず、Σ=1 を保留へ積むだけ。
/// 3. `\_l[,100]`: Y は絶対なので実効位置に依らず 0 + 100 = 100 を保留する。
///    **このとき Σ=1 はまだ保留に残っている**（`\_l` の腕は保留改行を消費しない）。
/// 4. `\n`: 保留カーソルがあるので、この改行を積む前に**保留を完全に実体化する**——
///    (1) 現在行 [あ\0] を確定 →(2) Σ=1 を消費して行送り 0 + 12 = 12・行内は 0 へ →
///    (3) カーソルの Y=100 がそれに後勝ちして行送り 100。そのうえで Σ=1 を積み直す。
/// 5. `あ`: フラッシュで (2) が効き、行送り 100 + 12 = **112**。
///
/// **(2) を走らせないと 124.0 になる**——`\_l` より前に書かれた Σ=1 が (3) を追い越して
/// 保留に残り、4 段目で積み直した改行と合流して Σ=2 になる（100 + 2×12 = 124）。
/// 124 は書かれた順（112）にも旧正典（100）にも一致しない値で、前に書かれた改行が
/// カーソルの**後**に効いてしまっている。
///
/// **4.3 前は 100.0 だった**（先行実体化そのものが無く、フラッシュで (2) 改行送り 24 に
/// カーソルの Y=100 が後勝ちして上書きしていた）。根拠は DD-11（Requirement 3.5/9.6）。
#[test]
fn written_order_applies_newlines_before_and_after_the_cursor() {
    let items = [
        TextItem::Glyph { ch: 'あ' },
        newline(),
        cursor_absolute_y100(),
        newline(),
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = layout_h(&items, 2);
    assert_eq!(lines.len(), 2);
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(lines[0].rect.top, 0.0);
    assert_eq!(
        lines[1].rect.top, 112.0,
        "正典値（4.3 の書かれた順による・DD-11）: 前に書かれた `\\n` が (2) で先に効き、         `\\_l` の 100 が (3) で後勝ちし、後に書かれた `\\n` の送り 12 が最後に乗る。         (2) を走らせないと Σ が漏れて 124.0・4.3 前は 100.0 だった"
    );
    assert_eq!(
        inline_positions(&lines[1]),
        vec![0.0],
        "行内軸は改行の行頭リセット 0（`\\_l` は X を動かしていない）"
    );
}

/// 末尾規則は不変（4.3 が先行実体化を足しても、内容の無い行は 1 本も増えない）。
///
/// `[あ, \_l[,100], \n]`（後続の可視グリフ無し）: `\n` の到着で保留カーソルが実体化され、
/// 現在行 [あ@0] はそこで確定する。行送り位置は 100 になるが、その位置に置かれる文字が
/// 無いので行は作られず、末尾の改行は保留のまま蒸発する（R5.2/5.3）。
///
/// 実体化の位置が走査の末尾からタグの到着時へ前倒しになっても、確定する行の中身
/// （[あ@0]・行内終端 10・行送り 0）は同じなので、この期待値は**書き換えではなく不変**である。
#[test]
fn trailing_cursor_then_newline_creates_no_extra_line() {
    let items = [
        TextItem::Glyph { ch: 'あ' },
        cursor_absolute_y100(),
        newline(),
    ];
    let lines = layout_h(&items, 1);
    assert_eq!(lines.len(), 1, "末尾の `\\_l`／`\\n` は行を作らない");
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(
        lines[0].rect.top, 0.0,
        "確定した行の行送り位置は実体化前の 0"
    );
    assert_eq!(lines[0].rect.right, 10.0, "行内終端は 'あ' の送り終端 10");

    // 混在順の末尾 `\n` →`\_l` →`\n`（3 段の実体化が末尾で走る形）でも同じ——
    // 4 段目の `\n` で (1)(2)(3) が走って行送りは 100 になるが、そこに置かれる文字が
    // 無いので行は増えず、末尾の改行は保留のまま蒸発する。
    let mixed_tail = [
        TextItem::Glyph { ch: 'あ' },
        newline(),
        cursor_absolute_y100(),
        newline(),
    ];
    let lines = layout_h(&mixed_tail, 1);
    assert_eq!(lines.len(), 1, "末尾の `\\n` →`\\_l` →`\\n` も行を作らない");
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(
        lines[0].rect.top, 0.0,
        "確定した行の行送り位置は実体化前の 0"
    );
    assert_eq!(lines[0].rect.right, 10.0);
}
