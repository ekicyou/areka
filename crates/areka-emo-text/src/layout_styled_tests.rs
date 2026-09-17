//! 文字ごとの見た目を配置へ通す配管の檻（タスク 5.1・要件 3.3／7.10／11.2／11.5／15.5）。
//!
//! 本ファイルが締めるのは 3 点である——⑴ 送り幅を測る口の「見た目込み」の既定実装が
//! **見た目の大きさ**で測ること（要件 7.10）、⑵ 配置の本体へ番号列を渡したとき、各文字が
//! 自分の大きさの送り幅を得て装飾番号を持ち帰ること（要件 3.3／11.2）、⑶ 番号列を
//! **渡さない**経路の出力が装飾を入れる前と 1 ビットも変わらないこと（要件 14 系の非回帰を
//! 本タスクの範囲で先に固定する）。
//!
//! タスク 5.2 が足すのはさらに 4 点である——(1) 行矩形の丈と行送りが**行内最大の em**で
//! 決まること（要件 7.9）、(2) 文字の置かれていない行の 2 つの落としどころ（次に置く文字の
//! 大きさ／スコープの現在の見た目）、(3) 塊先決の折返しでも合計が見た目込みになること
//! （要件 11.2）、(4) 行送りの式が `TextLayerConfig::line_pitch` の 1 点だけを通ること
//! （要件 7.8——値の比較では見張れないので**字面**で固定する）。

use areka_sakura::contract::ActorKey;

use super::test_support::{IMAGE, model};
use super::{
    CursorWarnGuard, FixedMetrics, GlyphMetrics, LayoutEngine, LineRect, PositionedLine, WrapPlan,
};
use crate::canvas::{ContentCanvas, ResidentContent};
use crate::look::{GlyphStyles, StyleId, StyleTable, TextLook};
use crate::region::TextRegion;
use crate::segment::{Segment, SegmentPlan};
use crate::state::{CursorCoord, CursorUnit, TextItem};
use crate::writing::WritingMode;

/// 大きさだけを既定から変えた見た目。
fn look_with_height(height: f32) -> TextLook {
    TextLook {
        height,
        ..TextLook::ukadoc_default()
    }
}

/// グリフ列（1 文字ずつ）。
fn glyph_items(text: &str) -> Vec<TextItem> {
    text.chars().map(|ch| TextItem::Glyph { ch }).collect()
}

/// 行のグリフの `(文字, 行内位置, 送り幅, 装飾番号)` を抜き出す。
fn glyph_tuples(line: &PositionedLine) -> Vec<(char, f32, f32, u32)> {
    line.glyphs
        .iter()
        .map(|g| (g.ch, g.inline_pos, g.advance, g.style.0))
        .collect()
}

/// 番号列の読み口を組む（表・番号列・既定・いま効いている見た目）。
fn styles<'a>(table: &'a StyleTable, ids: &'a [StyleId], default: &'a TextLook) -> GlyphStyles<'a> {
    GlyphStyles {
        table,
        ids,
        default,
        current: default,
    }
}

// ── 要件 7.10: 送り幅を測る口の「見た目込み」の既定実装 ──

/// 既定実装の `advance_styled` は **見た目の大きさ**で `advance` を引く
/// （`FixedMetrics` は全角＝大きさ・半角＝大きさ ÷ 2）。
///
/// 較正（要件 15.5）: 既定実装が見た目を無視して既定の大きさ（`font_height`）で測る誤りに
/// 戻すと、20 の行と 7 の行が 10 になって赤になる。
#[test]
fn advance_styled_default_impl_measures_at_the_look_height() {
    /// `(文字, 見た目の大きさ, 期待する送り幅)`。
    const CASES: &[(char, f32, f32)] = &[
        ('あ', 10.0, 10.0),
        ('あ', 20.0, 20.0),
        ('a', 20.0, 10.0),
        ('あ', 7.0, 7.0),
        ('a', 7.0, 3.5),
    ];
    // 表が空になるとループが恒真で緑になるので母数を先に固定する。
    assert_eq!(CASES.len(), 5, "検証表の母数");

    for &(ch, height, expected) in CASES {
        let look = look_with_height(height);
        assert_eq!(
            FixedMetrics.advance_styled(ch, &look),
            expected,
            "{ch:?} を大きさ {height} で測った送り幅"
        );
    }
}

// ── 要件 3.3／7.10／11.2／11.5: 番号列を渡した配置 ──

/// 番号列を渡すと、番号の付いた文字は**自分の大きさ**の送り幅を得て、装飾番号を持ち帰る。
/// 行内位置は見た目込みの送り幅の累積で進む（＝折返しと当たり判定の入力が装飾込みになる）。
///
/// 較正（要件 15.5）: ⑴ 送り幅を見た目でなく既定の大きさで測る誤りに戻すと送り幅が
/// `[10, 10, 10]`・行内位置が `[0, 10, 20]` になって赤。⑵ 装飾番号を転写し忘れる誤り
/// （すべて 0）に戻すと番号列 `[0, 1, 1]` の断言が赤。
#[test]
fn styled_glyphs_advance_at_their_own_height_and_carry_their_style_id() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(200), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyph_items("あいう");
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);
    assert_ne!(big, StyleId::DEFAULT, "大きさを変えた見た目は既定ではない");
    let ids = [StyleId::DEFAULT, big, big];

    let lines = LayoutEngine::layout_inner(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
        None,
        Some(styles(&table, &ids, &default)),
    );

    assert_eq!(lines.len(), 1, "折返しの起きない幅なので 1 行");
    assert_eq!(
        glyph_tuples(&lines[0]),
        vec![
            ('あ', 0.0, 10.0, 0),  // 既定の見た目＝font_height 10
            ('い', 10.0, 20.0, 1), // 大きさ 20 → 送り幅 20
            ('う', 30.0, 20.0, 1), // 直前が 20 進んだ位置から
        ],
    );
}

/// 見た目込みの送り幅が折返し位置を動かす（要件 11.2——二段構えの意味論はそのまま、
/// 判定に入る幅だけが装飾込みになる）。
///
/// 較正（要件 15.5）: 既定の大きさで測る誤りに戻すと 3 文字とも 1 行に収まって赤になる。
#[test]
fn styled_advance_moves_the_wrap_point() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(45), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyph_items("あいう");
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);
    let ids = [big, big, big];

    let lines = LayoutEngine::layout_inner(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
        None,
        Some(styles(&table, &ids, &default)),
    );

    // 20 × 3 = 60 > 折返し基準 45 → 3 文字目で折り返す（既定の 10 のままなら 30 で 1 行）。
    assert_eq!(lines.len(), 2, "見た目込みの幅で折り返す");
    assert_eq!(lines[0].glyphs.len(), 2);
    assert_eq!(lines[1].glyphs.len(), 1);
}

// ── 番号列を渡さない経路の非回帰 ──

/// 番号列を**渡さない**出力を、装飾を入れる前（2026-09-12 の実測）に採った値へ字義で固定する。
///
/// 従来の入口（[`LayoutEngine::layout`]・[`LayoutEngine::layout_with_cursor_warn`]）が
/// `styles: None` を渡す実装である以上、両者の突き合わせだけでは「引数を足したことで
/// 挙動が動いていない」の証明にならない（同じ経路を 2 度通るだけで恒真）。ここでは
/// **装飾を入れる前の実測値**を期待値として置き、`None` の経路がそこから 1 ビットでも
/// ずれたら赤になるようにする。あわせて、新しい装飾番号の欄が `None` の経路では
/// すべて既定（0）であることも締める。
#[test]
fn layout_without_styles_matches_the_frozen_pre_decoration_output() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(45), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let mut items = glyph_items("あaいうえ");
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyph_items("おか"));

    let lines = LayoutEngine::layout(
        &items,
        8,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );

    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0].rect,
        LineRect {
            left: 0.0,
            top: 0.0,
            right: 45.0,
            bottom: 10.0,
        },
    );
    assert_eq!(
        glyph_tuples(&lines[0]),
        vec![
            ('あ', 0.0, 10.0, 0),
            ('a', 10.0, 5.0, 0),
            ('い', 15.0, 10.0, 0),
            ('う', 25.0, 10.0, 0),
            ('え', 35.0, 10.0, 0),
        ],
    );
    assert_eq!(
        lines[1].rect,
        LineRect {
            left: 0.0,
            top: 12.0,
            right: 20.0,
            bottom: 22.0,
        },
    );
    assert_eq!(
        glyph_tuples(&lines[1]),
        vec![('お', 0.0, 10.0, 0), ('か', 10.0, 10.0, 0)],
    );
}

/// 既定の見た目だけの番号列を渡した出力は、番号列を渡さない出力と**値として**同一
/// （既定の番号の文字は従来どおり `advance` の経路で測られる——要件 14 系の非回帰）。
///
/// 較正（要件 15.5）: 既定の番号のときも見た目の側から測るように変え、かつその見た目が
/// 既定と食い違えば、この同一性が崩れて赤になる。
#[test]
fn all_default_style_ids_produce_the_same_lines_as_no_style_ids() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(45), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let mut items = glyph_items("あaいうえ");
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyph_items("おか"));
    let default = TextLook::ukadoc_default();
    let table = StyleTable::default();
    let ids = [StyleId::DEFAULT; 7];

    let without = LayoutEngine::layout(
        &items,
        8,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    let with_defaults = LayoutEngine::layout_inner(
        &items,
        8,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
        None,
        Some(styles(&table, &ids, &default)),
    );

    assert_eq!(with_defaults, without);
}

// ── 要件 3.3: 行のグリフ列（canvas）への転写 ──

/// [`ContentCanvas::from_layout`] が装飾番号を行のグリフ列へ写す（配管の 3 段目）。
/// 送り幅も見た目込みのまま渡るので、当たり判定の範囲（要件 11.5）の入力が装飾込みになる。
///
/// 較正（要件 15.5）: 転写を落として `StyleId::DEFAULT` を置く誤りにすると `[0, 1, 1]` が
/// `[0, 0, 0]` になって赤になる。
#[test]
fn canvas_from_layout_transcribes_the_style_id() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(200), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = glyph_items("あいう");
    let default = TextLook::ukadoc_default();
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);
    let ids = [StyleId::DEFAULT, big, big];

    let lines = LayoutEngine::layout_inner(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
        None,
        Some(styles(&table, &ids, &default)),
    );
    let canvas = ContentCanvas::from_layout(&lines, &region, WritingMode::HorizontalTb);

    let ResidentContent::GlyphRun(run) = &canvas.residents[0].content else {
        panic!("グリフ行の住人が来るはず");
    };
    assert_eq!(
        run.glyphs
            .iter()
            .map(|g| (g.ch, g.advance, g.style.0))
            .collect::<Vec<_>>(),
        vec![('あ', 10.0, 0), ('い', 20.0, 1), ('う', 20.0, 1)],
    );
}

// ── タスク 5.2 の追加ヘルパ ──

/// 番号列の読み口を組む（スコープの現在の見た目を既定と別に置ける形）。
fn styles_with_current<'a>(
    table: &'a StyleTable,
    ids: &'a [StyleId],
    default: &'a TextLook,
    current: &'a TextLook,
) -> GlyphStyles<'a> {
    GlyphStyles {
        table,
        ids,
        default,
        current,
    }
}

/// 折返し基準 `wordwrappoint.x` だけを与えた横書きの描画範囲。
fn region_h(wrap_x: i32) -> TextRegion {
    TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(wrap_x), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    )
}

/// 番号列つきの配置（横書き・既定の大きさ 10）。
fn layout_styled_h(
    items: &[TextItem],
    region: &TextRegion,
    styles: GlyphStyles<'_>,
    wrap: WrapPlan<'_>,
) -> Vec<PositionedLine> {
    let visible = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    let mut guard = CursorWarnGuard::default();
    LayoutEngine::layout_styled(
        items,
        visible,
        region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        wrap,
        styles,
        &ActorKey::from("0"),
        &mut guard,
    )
}

/// 行矩形を `(left, top, right, bottom)` の組へ落とす。
fn rects(lines: &[PositionedLine]) -> Vec<(f32, f32, f32, f32)> {
    lines
        .iter()
        .map(|l| (l.rect.left, l.rect.top, l.rect.right, l.rect.bottom))
        .collect()
}

// ── 要件 7.9: 行の丈は「その行に置かれた文字のうち最も大きい em」 ──

/// 行矩形の丈は行内の最大の大きさになる（既定 10・途中の 1 文字だけ 20 の行）。
///
/// 較正（要件 15.5）: 行の丈を既定の大きさ（`font_height`）で固定する誤りに戻すと
/// `bottom` が 20 → 10 になって赤になる。
#[test]
fn line_box_height_is_the_largest_em_placed_on_the_line() {
    let region = region_h(200);
    let items = glyph_items("あいう");
    let default = look_with_height(10.0);
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);
    let ids = [StyleId::DEFAULT, big, StyleId::DEFAULT];

    let lines = layout_styled_h(
        &items,
        &region,
        styles(&table, &ids, &default),
        WrapPlan::CharByChar,
    );

    assert_eq!(lines.len(), 1, "折返しの起きない幅なので 1 行");
    assert_eq!(
        lines[0].rect,
        LineRect {
            left: 0.0,
            top: 0.0,
            right: 40.0,  // 10 + 20 + 10
            bottom: 20.0, // 行内最大の em（既定固定なら 10）
        },
    );
}

/// 行送りは**閉じる行**の最大 em から引かれる（大きい行の次は大きく送り、
/// 小さい行の次は小さく送る）。式は `TextLayerConfig::line_pitch`（`em + 2`）の 1 点のまま。
///
/// 較正（要件 15.5）: 行の丈を既定の大きさで固定する誤りに戻すと 2 行目の `top` が
/// 22 → 12・3 行目が 34 → 24 になって赤になる。
#[test]
fn line_pitch_advances_by_the_closing_lines_largest_em() {
    let region = region_h(200);
    let mut items = glyph_items("あ");
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyph_items("い"));
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyph_items("う"));
    let default = look_with_height(10.0);
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);
    // 1 行目だけ 20・2 行目と 3 行目は既定 10。
    let ids = [big, StyleId::DEFAULT, StyleId::DEFAULT];

    let lines = layout_styled_h(
        &items,
        &region,
        styles(&table, &ids, &default),
        WrapPlan::CharByChar,
    );

    assert_eq!(
        rects(&lines),
        vec![
            (0.0, 0.0, 20.0, 20.0),  // 丈 20
            (0.0, 22.0, 10.0, 32.0), // 20 + 2 送った先・丈 10
            (0.0, 34.0, 10.0, 44.0), // 10 + 2 送った先
        ],
        "行送りは閉じる行の最大 em から引く（既定固定なら 12・24）"
    );
}

// ── 要件 7.9: 文字の置かれていない行の 2 つの落としどころ ──

/// 断言 ⑴——**文字の無い行はそのとき効いている大きさ**（＝次に置く文字の大きさ）で送る。
/// 先頭の改行は行を作らないが、送り量はその 1 文字の大きさで決まる。
///
/// 較正（要件 15.5）: 既定の大きさで固定する誤りに戻すと `top` が 22 → 12 になって赤。
#[test]
fn a_line_with_no_glyph_advances_by_the_height_in_effect() {
    let region = region_h(200);
    let items = vec![
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::Glyph { ch: 'あ' },
    ];
    let default = look_with_height(10.0);
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);

    let big_first = layout_styled_h(
        &items,
        &region,
        styles(&table, &[big], &default),
        WrapPlan::CharByChar,
    );
    assert_eq!(
        rects(&big_first),
        vec![(0.0, 22.0, 20.0, 42.0)],
        "改行だけの行は次に置く文字の大きさ 20 で送る"
    );

    // 対照: 同じ並びで次の文字が既定の大きさなら送りも既定のまま（差が見えていることの確認）。
    let default_first = layout_styled_h(
        &items,
        &region,
        styles(&table, &[StyleId::DEFAULT], &default),
        WrapPlan::CharByChar,
    );
    assert_eq!(
        rects(&default_first),
        vec![(0.0, 12.0, 10.0, 22.0)],
        "次に置く文字が既定なら送りも既定のまま"
    );
}

/// 断言 ⑵——**次に置く文字が無いまま閉じる行**はスコープの現在の見た目の大きさで送る。
///
/// 到達する本番の経路は `\_l` の先行実体化（`LineBreak` 腕・DD-11）である: 保留カーソルを
/// 抱えたまま改行が届くと、その場で「行を閉じて保留改行を適用する」——このとき手元に
/// 次の文字は無い。行内軸だけを指定した `\_l[50,]` を使うのは、行送り軸を指定すると
/// 直後の 保留カーソルの適用が送り量を上書きして
/// しまい、この断言が観測できなくなるためである。
///
/// 手計算（現在の見た目 30・既定 10）: `\n` で保留 1.0 →`\_l[50,]` で保留カーソル →2 つ目の
/// `\n` が先行実体化して `block = pitch(30) = 32`（行内は 50 へ）→ 最後の `あ` の直前で
/// 保留 1.0 を `pitch(10) = 12` で適用し `block = 44`・行内は行頭へ戻る。
///
/// 較正（要件 15.5）: 既定の大きさで固定する誤りに戻すと `top` が 44 → 24 になって赤。
#[test]
fn closing_with_no_following_glyph_uses_the_scope_current_look() {
    let region = region_h(200);
    let items = vec![
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::CursorMove {
            x: CursorCoord::Absolute {
                value: 50.0,
                unit: CursorUnit::Px,
            },
            y: CursorCoord::Omitted,
        },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::Glyph { ch: 'あ' },
    ];
    let default = look_with_height(10.0);
    let current = look_with_height(30.0);
    let table = StyleTable::default();

    let lines = layout_styled_h(
        &items,
        &region,
        styles_with_current(&table, &[StyleId::DEFAULT], &default, &current),
        WrapPlan::CharByChar,
    );

    assert_eq!(
        rects(&lines),
        vec![(0.0, 44.0, 10.0, 54.0)],
        "次に置く文字が無い行はスコープの現在の見た目 30 で送る（32 + 12）"
    );
}

// ── 要件 11.2: 塊先決の折返しでも合計が見た目込み ──

/// 塊ごとの送り幅合計も**同じ見た目**で合計する。合計は塊の収まり判定の左辺なので、
/// 旧幅のまま数えると「収まる」と誤判定して折返し位置が動く。
///
/// 手計算（既定 10・全グリフ 20・折返し基準 50・塊 `{0,1}` と `{1,3}`）: 見た目込みでは
/// 2 つ目の塊の合計が 60 で行頭幅 50 も超える＝長大塊として文字単位規則へ縮退し、
/// 3 文字目で折り返して **2 行**になる。旧幅（合計 30）だと残り行幅 30 にちょうど収まる
/// と判定され、塊内は追加判定なしで置かれて **1 行 4 文字**（行内位置 0/20/40/60）になる。
///
/// 較正（要件 15.5）: 区間ごとの合計を旧幅のまま数える誤りに戻すと、行数 2 → 1 で赤になる。
#[test]
fn segment_advance_sum_is_measured_with_the_glyph_looks() {
    let region = region_h(50);
    let items = glyph_items("あいうえ");
    let default = look_with_height(10.0);
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);
    let ids = [big; 4];
    let plan = SegmentPlan::from_segments(vec![
        Segment { start: 0, len: 1 },
        Segment { start: 1, len: 3 },
    ]);

    let lines = layout_styled_h(
        &items,
        &region,
        styles(&table, &ids, &default),
        WrapPlan::Segmented(&plan),
    );

    assert_eq!(
        lines
            .iter()
            .map(|l| l.glyphs.iter().map(|g| g.inline_pos).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![0.0, 20.0], vec![0.0, 20.0]],
        "塊の合計が見た目込み（旧幅なら 1 行 4 文字 0/20/40/60）"
    );
}

// ── 要件 7.8: 行送りの式は 1 点だけを通る（字面で固定する） ──

/// 行送りの式へ届く点が配置層に **1 つしかない**ことを字面で固定する。
///
/// 値の比較では「1 点だけ」は見張れない——装飾のために別の式や係数を足しても、今日の
/// 入力では同じ値が出うるからである。ここでは走査する 3 ファイルの中で
/// `metrics.line_pitch(` の綴りが **1 回だけ**現れ、それが `layout_styled.rs` の
/// `line_pitch_of` であることと、行送りの式そのもの
/// （`line_gap` を足す形）がどこにも写されていないことを判定する。
#[test]
fn the_line_pitch_formula_is_reached_through_a_single_call_site() {
    const SOURCES: &[(&str, &str)] = &[
        ("layout.rs", include_str!("layout.rs")),
        ("layout_styled.rs", include_str!("layout_styled.rs")),
        ("layout_line_ops.rs", include_str!("layout_line_ops.rs")),
    ];
    // 走査面が空になると以降の判定が恒真になるので母数を先に固定する。
    assert_eq!(SOURCES.len(), 3, "走査する配置層のファイル数");

    let calls: usize = SOURCES
        .iter()
        .map(|(_, src)| src.matches("metrics.line_pitch(").count())
        .sum();
    assert_eq!(
        calls, 1,
        "配置層から行送りの式へ届く点は 1 つだけ（装飾のために 2 つ目を足さないこと・要件 7.8）"
    );
    let styled = SOURCES
        .iter()
        .find(|(name, _)| *name == "layout_styled.rs")
        .expect("走査面に layout_styled.rs が要る")
        .1;
    assert!(
        styled.contains("fn line_pitch_of(") && styled.contains("metrics.line_pitch(height)"),
        "その 1 点は `line_pitch_of`——高さを 1 つ受けるだけで係数を持たない"
    );
    for (name, src) in SOURCES {
        assert!(
            !src.contains("line_gap"),
            "{name} が行送りの式（`line_gap` を足す形）を写している——式の正本は state.rs の 1 点"
        );
    }
}

// ── 装飾入りの公開入口 ──

/// **既定だけの番号列**を [`LayoutEngine::layout_styled`] へ渡した出力が、装飾を知らない
/// 従来の入口 [`LayoutEngine::layout`] と一致する（非回帰）。明示の改行を含む台本で、
/// 行分けと行矩形まで含めて同一であることを見る（この台本の 5 文字は 45 に収まるので
/// 自動折返しは起きない——折返し位置は
/// `styled_advance_moves_the_wrap_point` が持つ）。
///
/// 判定しないこと（doc と述語を一致させるための明示）: 既定でない番号の扱いは
/// `styled_glyphs_advance_at_their_own_height_and_carry_their_style_id` 以下が持ち、
/// `\_l` の縮退警告の配線は `layout_cursor_*_tests.rs` が持つ。本述語は**番号列を
/// 丸ごと無視する実装でも緑**である——それが非回帰の檻の役目だからで、装飾が効いている
/// ことの証拠には使えない。
#[test]
fn layout_styled_is_the_public_entry_for_decorated_scripts() {
    let region = region_h(45);
    let mut items = glyph_items("あaいうえ");
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyph_items("おか"));
    let default = look_with_height(10.0);
    let table = StyleTable::default();
    let ids = [StyleId::DEFAULT; 7];

    let via_styled = layout_styled_h(
        &items,
        &region,
        styles(&table, &ids, &default),
        WrapPlan::CharByChar,
    );
    let via_plain = LayoutEngine::layout(
        &items,
        7,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );

    assert_eq!(via_styled, via_plain, "既定だけの番号列は従来の出力と同一");
}

// ── 要件 3.1／7.9: `\_l` の実効位置の先読みも同じ行送り規則に従う ──

/// `\_l` の基点となる実効位置の先読み（保留改行の仮適用）は、フラッシュ側と**同じ**行送り
/// ——閉じる行の丈から引いた値——を使う。先読みだけが既定の大きさの行送りに取り残されると、
/// `\_l` を挟んだ次の行が前の行へ食い込む。
///
/// 台本 `[大きさ 20 の 1 文字, \n, \_l[,@0], 既定の 1 文字]`（既定 10）: `\_l` の行送り軸は
/// 「実効位置から 0px」なので、仮適用で求めた値がそのまま 2 行目の `top` として観測できる。
/// 閉じる行の丈は 20 ゆえ `line_pitch(20) = 22`。
///
/// 較正（要件 15.5）: 仮適用を既定の大きさの行送りへ戻す（`line_pitch_of(metrics,
/// heights.peek())` → `pitch`）と 2 行目の `top` が 22 → 12 になって赤——1 行目が占める
/// 0〜20 へ 10px 食い込む。`\_l` を外した同じ並びは
/// `line_pitch_advances_by_the_closing_lines_largest_em` が 22 で固定している。
#[test]
fn the_cursor_preview_of_a_pending_newline_uses_the_closing_lines_pitch() {
    let region = region_h(200);
    let items = vec![
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::CursorMove {
            x: CursorCoord::Omitted,
            y: CursorCoord::Relative {
                value: 0.0,
                unit: CursorUnit::Px,
            },
        },
        TextItem::Glyph { ch: 'い' },
    ];
    let default = look_with_height(10.0);
    let mut table = StyleTable::default();
    let big = table.intern(&look_with_height(20.0), &default);
    let ids = [big, StyleId::DEFAULT];

    let lines = layout_styled_h(
        &items,
        &region,
        styles(&table, &ids, &default),
        WrapPlan::CharByChar,
    );

    assert_eq!(
        rects(&lines),
        vec![
            (0.0, 0.0, 20.0, 20.0),  // 丈 20
            (0.0, 22.0, 10.0, 32.0), // 先読みも 20 + 2 で送る（既定固定なら 12＝食い込み）
        ],
        r"`\_l` の実効位置の先読みは閉じる行の丈から行送りを引く"
    );
}
