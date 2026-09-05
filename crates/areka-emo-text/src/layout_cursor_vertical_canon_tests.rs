//! 縦書き 2 方向（`vertical_rl`／`vertical_lr`）における `\_l` の**正典の着地**を、
//! 「同じ結果を出す別経路」との一致として固定するテスト（タスク 5.1・検証表 V1〜V5）。
//!
//! # 兄弟ファイルとの分担（重複させない）
//!
//! - `layout_cursor_vertical_tests.rs` — 縦書き `\_l` の**着地値そのもの**（タスク 1.1 が現行値
//!   として置き、1.2 の原点切替と 4.1 の語彙解禁が正典値へ書き換えた 15 本）。値が「どちらの
//!   是正で動いたか」を行ごとに読み分けるための台帳であり、本ファイルはそれを重複させない。
//! - 本ファイル — 値の逐語ではなく**正典が述べている性質**を固定する。すなわち
//!   ⑴ `\_l[0,0]` の着地が文字描画範囲の**内側**であること（境界上＝範囲内・V1 の残り半分）、
//!   ⑵ 列指定が**自動列送りと同値**であること（V2・V5）、
//!   ⑶ 正典の縦書き記述例 `\_l[@-1lh,0]`／`\_l[,@1em]` が「1 列ぶん左の列の先頭へ」「字送りを
//!   1 文字ぶん進める」という**言葉どおり**の位置になること（V3・V4・V5）、
//!   ⑷ Y が字送り方向（上から下）として横書きと同じ向きであること（Requirement 2.5）、
//!   ⑸ 保留改行を挟んだ `@` 相対の基点が**列の進む向き**で動くこと（`block_dir` の檻）。
//!
//! 期待値は design.md の検証表と正典逐語（requirements.md 付録 A）から**式で**導く。実装の
//! 戻り値は書き写さない。性質の主張には、同じ性質を別経路で満たす**参照レイアウト**を隣に
//! 置く（自動列送り／素の改行／`\_l` を含まない文字列）——値の逐語だけでは「言葉どおりか」を
//! 主張できないからである。
//!
//! # 共通前提（design.md Integration Tests）
//!
//! `FixedMetrics`・`font_height = 10`（全角 'あ' の advance 10・`line_pitch = ceil(10 × 1.25) = 13`）・
//! バルーン画像原寸 `IMAGE = (400, 224)`・validrect は画像全域（`0/0/400/224`）・`origin` 未宣言。
//! 未宣言ゆえ書字開始点は書字開始角へ縮退する——`vertical_rl` は `(right, top) = (400, 0)`・
//! `vertical_lr` は `(left, top) = (0, 0)`。折返し閾値は行内軸の遠辺 `bottom = 224`。
//!
//! 行矩形は `(left, top, right, bottom)`。縦書きでは行＝列であり、`left`／`right` が列の位置
//! （行送り軸）、`top`／`bottom` が列内の字送り範囲（行内軸）になる。
//!
//! **この前提が弁別性を保っていること**（2 方向の書字開始角・画像中央・列送り後の現在位置が
//! 互いに相異なること）自体を、最初の 1 本で檻に入れてある——前提が劣化すると、以降の
//! すべての「取り違えを検知する」主張が黙って無力化されるためである。

use super::test_support::{IMAGE, glyphs, inline_positions, model};
use super::{FixedMetrics, GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::region::TextRegion;
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;
use log_capture_kit::capture;

/// 共通前提の文字高さ。
const FONT: f32 = 10.0;
/// 全角 'あ' の行内送り（`FixedMetrics`・前提の檻で実測と突合する）。
const ADVANCE: f32 = 10.0;
/// 行送りピッチ `ceil(10 × 1.25)`（前提の檻で実測と突合する）。
const PITCH: f32 = 13.0;

/// `origin` 未宣言・validrect 全域のバルーン領域。
///
/// `TextRegion::resolve` は未宣言 `origin` について `debug!` を出すので、ログ件数を数える
/// テストでは**必ず捕捉窓の外**でこれを組むこと。
fn region_for(mode: WritingMode) -> TextRegion {
    TextRegion::resolve(&model((None, None), (None, None)), IMAGE, mode)
}

/// 既に組んだ領域でレイアウトを通す（ログ捕捉窓の内側から呼べる形）。
fn layout_with(
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

/// 共通前提でレイアウトを通す。
fn layout_in(items: &[TextItem], visible: usize, mode: WritingMode) -> Vec<PositionedLine> {
    layout_with(items, visible, &region_for(mode), mode)
}

/// 行矩形を `(left, top, right, bottom)` で取り出す。
fn rect_of(line: &PositionedLine) -> (f32, f32, f32, f32) {
    let r = &line.rect;
    (r.left, r.top, r.right, r.bottom)
}

/// 列の位置（行送り軸の 2 辺）だけを取り出す。
fn column_of(line: &PositionedLine) -> (f32, f32) {
    (line.rect.left, line.rect.right)
}

/// 全角グリフ 1 個。
fn glyph() -> TextItem {
    TextItem::Glyph { ch: 'あ' }
}

/// 素の改行 1 個（`ratio = 1.0`）。
fn newline() -> TextItem {
    TextItem::LineBreak { ratio: 1.0 }
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

/// `\_l[,N]`（X 省略・Y は絶対 px）。
fn cursor_y_px(y: f32) -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Omitted,
        y: CursorCoord::Absolute {
            value: y,
            unit: CursorUnit::Px,
        },
    }
}

/// 正典の縦書き記述例 `\_l[@N lh, 0]`（X は列送り相対・Y は列の先頭を指す絶対 0）。
fn cursor_column_step(value: f32) -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Relative {
            value,
            unit: CursorUnit::Lh,
        },
        y: CursorCoord::Absolute {
            value: 0.0,
            unit: CursorUnit::Px,
        },
    }
}

/// 正典の縦書き記述例 `\_l[,@1em]`（X 省略・Y は字送り相対）。
fn cursor_inline_step() -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Omitted,
        y: CursorCoord::Relative {
            value: 1.0,
            unit: CursorUnit::Em,
        },
    }
}

/// `\_l[@0,]`（X は相対 0＝実効位置から動かない・Y は省略）。
///
/// 「実効位置がどこか」だけを問う探針である——結果が実効位置そのものになるので、基点の
/// 計算が 1 だけ違えば着地も 1 だけ違う。
fn cursor_relative_zero_x() -> TextItem {
    TextItem::CursorMove {
        x: CursorCoord::Relative {
            value: 0.0,
            unit: CursorUnit::Px,
        },
        y: CursorCoord::Omitted,
    }
}

// ─────────────────────────────────────────────────────────────────────
// 前提の檻（3.3 の二段構えと同型・4.4 の Suggestion）
// ─────────────────────────────────────────────────────────────────────

/// 共通前提が「取り違えを弁別できる」形を保っていることを、**性質として**固定する。
///
/// 以降のすべてのテストは「方向を取り違えた実装／基点を取り違えた実装が赤になる」ことに
/// 依存している。その依存の土台（候補が互いに相異なること）が黙って崩れると、値の逐語は
/// 緑のまま主張だけが空洞になる——3.3 が `centerx` の弁別性を定数 1 行で失った事例と同型。
/// そこで土台そのものを檻に入れる。
#[test]
fn the_vertical_fixture_keeps_the_directions_and_basepoints_apart() {
    let rl = region_for(WritingMode::VerticalRl);
    let lr = region_for(WritingMode::VerticalLr);

    // ⑴ 共通前提そのもの（design.md Integration Tests の「共通前提」行）。
    assert_eq!(
        (rl.left(), rl.top(), rl.right(), rl.bottom()),
        (0.0, 0.0, 400.0, 224.0),
        "validrect は画像全域"
    );
    assert_eq!(rl.image_size(), (400.0, 224.0), "バルーン画像原寸");
    assert_eq!(
        FixedMetrics.advance('あ', FONT),
        ADVANCE,
        "全角 1 文字の行内送り"
    );
    assert_eq!(
        FixedMetrics.line_pitch(FONT),
        PITCH,
        "行送りピッチ ceil(10 × 1.25)"
    );

    // ⑵ 列送り幅（13）と字送り送り幅（10）が相異なる＝軸を取り違えた実装が弁別できる。
    assert_ne!(
        PITCH, ADVANCE,
        "列送り幅と字送り幅が同値だと、軸の取り違えが着地に現れない"
    );

    // ⑶ 書字開始角が 2 方向で相異なる＝書字方向を取り違えた実装が弁別できる。
    assert_eq!(rl.start(), (400.0, 0.0), "vertical_rl は (right, top)");
    assert_eq!(lr.start(), (0.0, 0.0), "vertical_lr は (left, top)");
    assert_ne!(
        rl.start().0,
        lr.start().0,
        "2 方向の書字開始角が行送り軸で相異なる"
    );

    // ⑷ 画像中央が両方向の書字開始角と**両軸で**相異なる（原点と画像基準の取り違えの弁別）。
    let center = (rl.image_size().0 / 2.0, rl.image_size().1 / 2.0);
    for (name, start) in [("vertical_rl", rl.start()), ("vertical_lr", lr.start())] {
        assert_ne!(center.0, start.0, "{name}: 画像中央 X と書字開始角 X");
        assert_ne!(center.1, start.1, "{name}: 画像中央 Y と書字開始角 Y");
    }

    // ⑸ 1 列送った後の列位置が、1 列目とも・2 方向の相互とも相異なる。
    //    ここが潰れると「`@` 相対の基点＝現在位置」と「基点＝原点」が弁別できなくなる。
    let rl_cols = layout_in(&glyphs(23), 23, WritingMode::VerticalRl);
    let lr_cols = layout_in(&glyphs(23), 23, WritingMode::VerticalLr);
    assert_eq!(rl_cols.len(), 2, "22 文字で列が埋まり 23 文字目が次の列へ");
    assert_eq!(lr_cols.len(), 2);
    assert_ne!(column_of(&rl_cols[0]), column_of(&rl_cols[1]));
    assert_ne!(column_of(&lr_cols[0]), column_of(&lr_cols[1]));
    assert_ne!(
        column_of(&rl_cols[1]),
        column_of(&lr_cols[1]),
        "2 列目が 2 方向で相異なる（方向を取り違えた実装が同じ列矩形を返せない）"
    );
    assert_ne!(
        column_of(&rl_cols[1]).1,
        rl.start().0,
        "vertical_rl: 2 列目の現在位置(387) と原点(400) が相異なる＝`@` 相対の基点が弁別できる"
    );
    assert_ne!(
        column_of(&lr_cols[1]).0,
        lr.start().0,
        "vertical_lr: 2 列目の現在位置(13) と原点(0) が相異なる"
    );
}

// ─────────────────────────────────────────────────────────────────────
// V1: `\_l[0,0]` は 1 列目＝文字描画範囲の内側（境界上は範囲内）
// ─────────────────────────────────────────────────────────────────────

/// **V1 の残り半分**: `vertical_rl` の `\_l[0,0]` は文字描画範囲の**内側**なので、範囲外の
/// DEBUG を 1 件も残さない。
///
/// 解決値は X ＝ 書字開始角(400) + 0 ＝ 400 で、X 軸の範囲は validrect の `[left, right]`
/// ＝ `[0, 400]`。`400 == right` は**閉区間の境界上＝範囲内**なので記録しない（design.md
/// 縮退表の最終行「境界上は記録しない」）。Y ＝ 0 も `[0, 224]` の下端＝境界上で 0 件。
/// 列矩形が `[390, 400]` と範囲の内側へ伸びる形になるが、判定するのは**点**であって
/// 矩形ではない（矩形の可視性は描画側の責務）。
///
/// **同じ観測点で 1 件を出せる対照を隣に置く**（`\_l[10,0]` → X ＝ 410 は右辺 400 の外）。
/// 対照が無ければ「0 件」は経路ごと死んでいても緑になり、境界上が範囲内であるという主張が
/// 空洞になる。対照は同時に、着地が寄せられない（410 のまま）ことも示す（R2.6）。
///
/// Y は両方とも境界上なので、対照の件数は厳密に 1 件（X 軸のみ）になる。
#[test]
fn vertical_rl_zero_zero_is_inside_the_text_area_and_records_no_debug() {
    let mode = WritingMode::VerticalRl;
    // 領域は捕捉窓の外で組む（`TextRegion::resolve` 自身が縮退の `debug!` を出すため）。
    let region = region_for(mode);
    let inside = [cursor_px(0.0, 0.0), glyph()];
    let outside = [cursor_px(10.0, 0.0), glyph()];

    let (lines, events) = capture(|| layout_with(&inside, 1, &region, mode));
    assert_eq!(lines.len(), 1);
    assert_eq!(
        rect_of(&lines[0]),
        (390.0, 0.0, 400.0, 10.0),
        "X = 書字開始角(400) + 0 → 1 列目 [390, 400]"
    );
    assert_eq!(
        inline_positions(&lines[0]),
        vec![0.0],
        "Y = 0 は字送り軸の先頭＝列の上端"
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e.message().starts_with("[note_out_of_range]"))
            .count(),
        0,
        "x = right・y = top はいずれも閉区間の境界上＝範囲内ゆえ記録しない"
    );

    // 対照: 同じ観測点で 1 件を出せることを示す（境界の 1 つ外側）。
    let (lines, events) = capture(|| layout_with(&outside, 1, &region, mode));
    assert_eq!(
        rect_of(&lines[0]),
        (400.0, 0.0, 410.0, 10.0),
        "X = 400 + 10 = 410 が字義どおり着地する（右辺 400 へ寄せない・R2.6）"
    );
    let notes: Vec<_> = events
        .iter()
        .filter(|e| e.message().starts_with("[note_out_of_range]"))
        .collect();
    assert_eq!(notes.len(), 1, "範囲外は X 軸の 1 件だけ（Y は境界上）");
    assert_eq!(notes[0].level, tracing::Level::DEBUG);
    assert_eq!(notes[0].field("axis"), Some("X"));
    assert_eq!(notes[0].field("value"), Some("410.0"));
    assert_eq!(notes[0].field("range_min"), Some("0.0"));
    assert_eq!(notes[0].field("range_max"), Some("400.0"));
}

// ─────────────────────────────────────────────────────────────────────
// V2・V5: 列指定は自動列送りと同値
// ─────────────────────────────────────────────────────────────────────

/// **V2**: `vertical_rl` の `\_l[-13,0]` が着く列は、**折返しによる自動列送り**が着く列と
/// 同じである（正典「次の列へ移るには X にマイナス値を指定する」）。
///
/// 参照は `\_l` を一切含まない 23 文字——行内軸の閾値は遠辺 `bottom = 224` なので 22 文字で
/// 列が埋まり、23 文字目が自動で次の列へ送られる。その列は `block_pos += (−1) × pitch`
/// ＝ 400 − 13 ＝ 387（列の右端）＝列矩形 `[377, 387]`。
/// `\_l[-13,0]` は X ＝ 書字開始角(400) + (−13) ＝ 387 で、同じ列矩形に着く。
///
/// 値の逐語（`[377, 387]`）だけでは「自動列送りと同値」は主張できない。両者を**同じ実行で
/// 突き合わせる**ことで、片方だけが動く変更（列送り幅の取り違え・符号の取り違え）が赤になる。
#[test]
fn vertical_rl_negative_x_matches_the_automatic_column_feed() {
    let mode = WritingMode::VerticalRl;
    let auto = layout_in(&glyphs(23), 23, mode);
    assert_eq!(auto.len(), 2, "22 文字で列が埋まり 23 文字目が次の列へ");
    assert_eq!(
        column_of(&auto[1]),
        (377.0, 387.0),
        "自動列送り: 400 + (−1) × 13 = 387 → 列矩形 [377, 387]"
    );

    let moved = layout_in(&[cursor_px(-13.0, 0.0), glyph()], 1, mode);
    assert_eq!(moved.len(), 1);
    assert_eq!(
        column_of(&moved[0]),
        column_of(&auto[1]),
        "`\\_l[-13,0]` の列は自動列送りの列と同値"
    );
    assert_eq!(rect_of(&moved[0]), (377.0, 0.0, 387.0, 10.0));
    assert_eq!(
        inline_positions(&moved[0]),
        vec![0.0],
        "Y = 0 → 列の先頭（字送り軸の上端）"
    );

    // 弁別: 同じ指定は `vertical_lr` では 2 列目にならない（X = 0 + (−13) = −13＝範囲の外）。
    let mirror = layout_in(
        &[cursor_px(-13.0, 0.0), glyph()],
        1,
        WritingMode::VerticalLr,
    );
    assert_ne!(
        column_of(&mirror[0]),
        column_of(&auto[1]),
        "負の X が次の列を指すのは vertical_rl だけ（方向の取り違えが弁別できる）"
    );
}

/// **V5（V2 の鏡像）**: `vertical_lr` では**正**の X が自動列送りと同値になる。
///
/// `vertical_lr` の列は左から右へ進むので `block_pos += (+1) × pitch` ＝ 0 + 13 ＝ 13
/// （列の左端）＝列矩形 `[13, 23]`。`\_l[13,0]` は X ＝ 書字開始角(0) + 13 ＝ 13 で同値。
#[test]
fn vertical_lr_positive_x_matches_the_automatic_column_feed() {
    let mode = WritingMode::VerticalLr;
    let auto = layout_in(&glyphs(23), 23, mode);
    assert_eq!(auto.len(), 2);
    assert_eq!(
        column_of(&auto[1]),
        (13.0, 23.0),
        "自動列送り: 0 + (+1) × 13 = 13 → 列矩形 [13, 23]"
    );

    let moved = layout_in(&[cursor_px(13.0, 0.0), glyph()], 1, mode);
    assert_eq!(moved.len(), 1);
    assert_eq!(
        column_of(&moved[0]),
        column_of(&auto[1]),
        "`\\_l[13,0]` の列は自動列送りの列と同値"
    );
    assert_eq!(rect_of(&moved[0]), (13.0, 0.0, 23.0, 10.0));
    assert_eq!(inline_positions(&moved[0]), vec![0.0]);

    // 弁別: 同じ指定は `vertical_rl` では 2 列目にならない（X = 400 + 13 = 413）。
    let mirror = layout_in(&[cursor_px(13.0, 0.0), glyph()], 1, WritingMode::VerticalRl);
    assert_ne!(column_of(&mirror[0]), column_of(&auto[1]));
}

// ─────────────────────────────────────────────────────────────────────
// V3・V4・V5: 正典の縦書き記述例を「言葉どおり」の位置として固定する
// ─────────────────────────────────────────────────────────────────────

/// **V3**: 正典の記述例 `\_l[@-1lh,0]`＝「1 列ぶん左の列の先頭へ」。
///
/// 「1 列ぶん左の列の先頭」を値の逐語ではなく**もう 1 回の列送り**として書き、両者が一致する
/// ことを主張する。参照は `[あ, \n, あ, \n, あ]`（列送り 2 回）、被験は
/// `[あ, \n, あ, \_l[@-1lh,0], あ]`（列送り 1 回＋記述例）。3 文字目はどちらも 3 列目
/// ＝ 400 − 13 − 13 ＝ 374（列の右端）＝列矩形 `[364, 374]`・字送り 0（列の先頭）。
///
/// **`\_l` の前に列を 1 つ送っておくことが要点である**——そうしないと現在位置(400) と
/// 原点(400) が同値になり、`@` 相対の基点を原点と取り違えた実装も同じ値を返してしまう
/// （前提の檻 ⑸ が、送った後の現在位置 387 が原点 400 と相異なることを保証している）。
#[test]
fn canon_example_relative_column_step_reaches_the_next_column_head_in_vertical_rl() {
    let mode = WritingMode::VerticalRl;
    let reference = layout_in(&[glyph(), newline(), glyph(), newline(), glyph()], 3, mode);
    assert_eq!(reference.len(), 3);
    assert_eq!(
        rect_of(&reference[2]),
        (364.0, 0.0, 374.0, 10.0),
        "列送り 2 回: 400 − 13 − 13 = 374 → 列矩形 [364, 374]・字送り 0"
    );

    let items = [
        glyph(),
        newline(),
        glyph(),
        cursor_column_step(-1.0),
        glyph(),
    ];
    let lines = layout_in(&items, 3, mode);
    assert_eq!(lines.len(), 3);
    assert_eq!(
        rect_of(&lines[2]),
        rect_of(&reference[2]),
        "`\\_l[@-1lh,0]` は「1 列ぶん左の列の先頭へ」＝もう 1 回の列送りと同じ矩形"
    );
    assert_eq!(
        inline_positions(&lines[2]),
        vec![0.0],
        "Y = 0（絶対）→ 列の先頭。基点は原点の Y 成分 top = 0"
    );
}

/// **V4**: 正典の記述例 `\_l[,@1em]`＝「字送りを 1 文字ぶん進める」。
///
/// 「1 文字ぶん進める」を値の逐語ではなく**もう 1 文字ぶんの字送り**として書き、両者が一致
/// することを主張する。参照は `\_l` を含まない `あああ`——同じ列に字送り 0・10・20 で並ぶ。
/// 被験は `[あ, \_l[,@1em], あ]` で、2 文字目は現在の字送り位置(10) + 1em(10) ＝ 20
/// ＝**参照の 3 文字目が置かれる位置**。X は省略なので列は動かない。
#[test]
fn canon_example_relative_inline_step_advances_one_character_in_vertical_rl() {
    let mode = WritingMode::VerticalRl;
    let reference = layout_in(&glyphs(3), 3, mode);
    assert_eq!(reference.len(), 1);
    assert_eq!(
        inline_positions(&reference[0]),
        vec![0.0, 10.0, 20.0],
        "素の 3 文字は同じ列に 0・10・20 で並ぶ"
    );

    let lines = layout_in(&[glyph(), cursor_inline_step(), glyph()], 2, mode);
    assert_eq!(
        lines.len(),
        2,
        "字送り軸の移動が成立するので `\\_l` は列の分割点になる"
    );
    assert_eq!(
        column_of(&lines[1]),
        column_of(&reference[0]),
        "X 省略＝列は動かない"
    );
    assert_eq!(
        inline_positions(&lines[1]),
        vec![inline_positions(&reference[0])[2]],
        "「字送りを 1 文字ぶん進める」＝3 文字目が置かれる字送り位置(20) と同値"
    );
    assert_eq!(rect_of(&lines[1]), (390.0, 20.0, 400.0, 30.0));
}

/// **V5（V3 の鏡像）**: `vertical_lr` では `\_l[@1lh,0]` が「1 列ぶん右の列の先頭へ」になる。
///
/// 列送り 2 回は 0 + 13 + 13 ＝ 26（列の左端）＝列矩形 `[26, 36]`。`vertical_rl` の対応する
/// 着地 `[364, 374]` とは別の値になる＝方向を取り違えた実装が弁別できる。
#[test]
fn canon_example_relative_column_step_reaches_the_next_column_head_in_vertical_lr() {
    let mode = WritingMode::VerticalLr;
    let reference = layout_in(&[glyph(), newline(), glyph(), newline(), glyph()], 3, mode);
    assert_eq!(reference.len(), 3);
    assert_eq!(
        rect_of(&reference[2]),
        (26.0, 0.0, 36.0, 10.0),
        "列送り 2 回: 0 + 13 + 13 = 26 → 列矩形 [26, 36]"
    );

    let items = [
        glyph(),
        newline(),
        glyph(),
        cursor_column_step(1.0),
        glyph(),
    ];
    let lines = layout_in(&items, 3, mode);
    assert_eq!(lines.len(), 3);
    assert_eq!(
        rect_of(&lines[2]),
        rect_of(&reference[2]),
        "`\\_l[@1lh,0]` は鏡像の記述例＝もう 1 回の列送りと同じ矩形"
    );
    assert_eq!(inline_positions(&lines[2]), vec![0.0]);

    // 弁別: 同じ着地は `vertical_rl` では起きない。
    let mirror = layout_in(&items, 3, WritingMode::VerticalRl);
    assert_ne!(rect_of(&mirror[2]), rect_of(&reference[2]));
}

/// **V5（V4 の鏡像）**: `vertical_lr` の `\_l[,@1em]` も「字送りを 1 文字ぶん進める」。
///
/// 字送り軸は 2 方向で共通（上から下）なので字送り位置は `vertical_rl` と同じ 20 になるが、
/// **列の位置は `[0, 10]` と異なる**（`vertical_rl` は `[390, 400]`）。
#[test]
fn canon_example_relative_inline_step_advances_one_character_in_vertical_lr() {
    let mode = WritingMode::VerticalLr;
    let reference = layout_in(&glyphs(3), 3, mode);
    assert_eq!(inline_positions(&reference[0]), vec![0.0, 10.0, 20.0]);

    let lines = layout_in(&[glyph(), cursor_inline_step(), glyph()], 2, mode);
    assert_eq!(lines.len(), 2);
    assert_eq!(column_of(&lines[1]), column_of(&reference[0]));
    assert_eq!(
        inline_positions(&lines[1]),
        vec![inline_positions(&reference[0])[2]]
    );
    assert_eq!(rect_of(&lines[1]), (0.0, 20.0, 10.0, 30.0));
    assert_ne!(
        column_of(&lines[1]),
        (390.0, 400.0),
        "列の位置は vertical_rl の鏡像であって同値ではない"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Requirement 2.5: Y は字送り方向（上から下）＝横書きと同じ向き
// ─────────────────────────────────────────────────────────────────────

/// **Requirement 2.5**: Y は 3 書字方向とも「上から下」で、同じ Y 値が同じ image 空間の
/// 高さへ着く（正典「Yは字送り方向（上から下）なので横書きと同じ感覚で使える」）。
///
/// 3 方向とも `origin` 未宣言の書字開始角の Y 成分は `top = 0` なので、`\_l[,N]` の解決値
/// `0 + N` がそのまま image 空間の Y になる。横書きでは Y が**行送り軸**・縦書きでは
/// **行内軸**に写るが、行矩形の `top`／`bottom` は 3 方向とも image 空間の上端・下端なので、
/// 同じ 1 本の期待値で 3 方向を比べられる。
///
/// 2 つの値（20 と 50）を並べるのは、**向き**（正値が下）を主張するためである——値を 1 つ
/// しか見なければ、Y を反転した実装も「たまたま原点対称で一致する」形を作りうる。
#[test]
fn absolute_y_is_the_downward_inline_advance_in_all_three_writing_modes() {
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let near = layout_in(&[cursor_y_px(20.0), glyph()], 1, mode);
        let far = layout_in(&[cursor_y_px(50.0), glyph()], 1, mode);
        assert_eq!(near.len(), 1, "{mode:?}");
        assert_eq!(far.len(), 1, "{mode:?}");
        assert_eq!(
            (near[0].rect.top, near[0].rect.bottom),
            (20.0, 30.0),
            "{mode:?}: Y = top(0) + 20 → image 空間の上端 20・下端 20 + 10"
        );
        assert_eq!(
            (far[0].rect.top, far[0].rect.bottom),
            (50.0, 60.0),
            "{mode:?}: Y = top(0) + 50"
        );
        assert!(
            far[0].rect.top > near[0].rect.top,
            "{mode:?}: Y の正値は下方向（3 方向とも同じ向き）"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// 実効位置の仮適用: 保留改行を挟んだ `@` 相対（`block_dir` の檻）
// ─────────────────────────────────────────────────────────────────────

/// **`block_dir` の檻（タスク 4.2 からの拘束力ある申し送り）**: 保留改行を挟んだ `@` 相対の
/// 基点が、**列の進む向き**（`vertical_rl` は左＝負）で動く。
///
/// `\_l` に先立つ改行はその場では実体化されず保留される。`@` 相対の基点は「もし今フラッシュ
/// したら次の文字が置かれる位置」＝保留改行を仮適用した実効位置なので、`vertical_rl` では
/// 行送り軸が **`block_pos += (−1) × pitch × Σratio`** で動く。したがって `\_l[@0,]`
/// （実効位置から動かない）は、`\_l` を書かなかった場合と**寸分違わぬ位置**へ着く
/// ——「改行を挟んだ相対 0 指定が改行を取り消さない」の縦書き版である。
///
/// これを縦書きで守る檻は 5.1 以前に **0 本**だった。`horizontal_tb` と `vertical_lr` は
/// `block_dir = +1` なので符号を落としても差が出ず、V3 は保留改行を挟まないのでこの経路を
/// 通らない。実測: 仮適用の `eff_block += block_dir * pitch * sum` を `1.0 * pitch * sum`
/// に差し替えると本テストだけが赤になる（Σ = 1 で 387 が 413 に、Σ = 2 で 374 が 426 に）。
///
/// Σ を 1 と 2 の 2 通り置くのは、`sum` を落とした実装（`block_dir * pitch` だけ）も
/// 捕まえるためである（Σ = 1 では一致してしまい、Σ = 2 で 374 対 387 の差が出る）。
#[test]
fn pending_newline_moves_the_relative_basepoint_along_the_column_direction_in_vertical_rl() {
    let mode = WritingMode::VerticalRl;

    // Σ = 1.0: 参照（`\_l` なし）と被験（`\_l[@0,]` あり）が同じ列矩形に着く。
    let reference = layout_in(&[glyph(), newline(), glyph()], 2, mode);
    assert_eq!(reference.len(), 2);
    assert_eq!(
        rect_of(&reference[1]),
        (377.0, 0.0, 387.0, 10.0),
        "保留改行 1 回: 400 + (−1) × 13 × 1 = 387 → 列矩形 [377, 387]"
    );
    let probed = layout_in(
        &[glyph(), newline(), cursor_relative_zero_x(), glyph()],
        2,
        mode,
    );
    assert_eq!(probed.len(), 2);
    assert_eq!(
        rect_of(&probed[1]),
        rect_of(&reference[1]),
        "`\\_l[@0,]` の基点は保留改行を仮適用した実効位置＝改行を取り消さない"
    );

    // Σ = 2.0: 保留改行が累算されることまで基点に効く。
    let reference2 = layout_in(&[glyph(), newline(), newline(), glyph()], 2, mode);
    assert_eq!(
        rect_of(&reference2[1]),
        (364.0, 0.0, 374.0, 10.0),
        "保留改行 2 回: 400 + (−1) × 13 × 2 = 374 → 列矩形 [364, 374]"
    );
    let probed2 = layout_in(
        &[
            glyph(),
            newline(),
            newline(),
            cursor_relative_zero_x(),
            glyph(),
        ],
        2,
        mode,
    );
    assert_eq!(
        rect_of(&probed2[1]),
        rect_of(&reference2[1]),
        "Σratio が基点に乗る（`sum` を落とすと 387 になり赤）"
    );
}

/// **鏡像**: `vertical_lr` でも保留改行を挟んだ `@` 相対の基点が列の進む向き（右＝正）で動く。
///
/// `block_dir = +1` なので着地は `vertical_rl` と逆側へ進み、同じ入力が別の列矩形になる
/// （Σ = 1 で `[13, 23]`・Σ = 2 で `[26, 36]`）。
#[test]
fn pending_newline_moves_the_relative_basepoint_along_the_column_direction_in_vertical_lr() {
    let mode = WritingMode::VerticalLr;

    let reference = layout_in(&[glyph(), newline(), glyph()], 2, mode);
    assert_eq!(
        rect_of(&reference[1]),
        (13.0, 0.0, 23.0, 10.0),
        "保留改行 1 回: 0 + (+1) × 13 × 1 = 13 → 列矩形 [13, 23]"
    );
    let probed = layout_in(
        &[glyph(), newline(), cursor_relative_zero_x(), glyph()],
        2,
        mode,
    );
    assert_eq!(rect_of(&probed[1]), rect_of(&reference[1]));

    let reference2 = layout_in(&[glyph(), newline(), newline(), glyph()], 2, mode);
    assert_eq!(
        rect_of(&reference2[1]),
        (26.0, 0.0, 36.0, 10.0),
        "保留改行 2 回: 0 + (+1) × 13 × 2 = 26 → 列矩形 [26, 36]"
    );
    let probed2 = layout_in(
        &[
            glyph(),
            newline(),
            newline(),
            cursor_relative_zero_x(),
            glyph(),
        ],
        2,
        mode,
    );
    assert_eq!(rect_of(&probed2[1]), rect_of(&reference2[1]));

    // 弁別: 同じ入力が `vertical_rl` では別の列矩形になる（方向の取り違えが赤になる）。
    let mirror = layout_in(
        &[glyph(), newline(), cursor_relative_zero_x(), glyph()],
        2,
        WritingMode::VerticalRl,
    );
    assert_ne!(rect_of(&mirror[1]), rect_of(&reference[1]));
}
