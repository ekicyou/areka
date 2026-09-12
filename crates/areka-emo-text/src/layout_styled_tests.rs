//! 文字ごとの見た目を配置へ通す配管の檻（タスク 5.1・要件 3.3／7.10／11.2／11.5／15.5）。
//!
//! 本ファイルが締めるのは 3 点である——⑴ 送り幅を測る口の「見た目込み」の既定実装が
//! **見た目の大きさ**で測ること（要件 7.10）、⑵ 配置の本体へ番号列を渡したとき、各文字が
//! 自分の大きさの送り幅を得て装飾番号を持ち帰ること（要件 3.3／11.2）、⑶ 番号列を
//! **渡さない**経路の出力が装飾を入れる前と 1 ビットも変わらないこと（要件 14 系の非回帰を
//! 本タスクの範囲で先に固定する）。
//!
//! 行の高さ（行内最大 em）と装飾入りの公開入口 `layout_styled` はタスク 5.2 の担当で、
//! 本ファイルはまだ触れない。

use super::test_support::{IMAGE, model};
use super::{FixedMetrics, GlyphMetrics, LayoutEngine, LineRect, PositionedLine, WrapPlan};
use crate::canvas::{ContentCanvas, ResidentContent};
use crate::look::{GlyphStyles, StyleId, StyleTable, TextLook};
use crate::region::TextRegion;
use crate::state::TextItem;
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
