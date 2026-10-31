//! # 横書きの帯と書体の行間（spec emo-text-line-gap・設計の LineGapBandTests）
//!
//! 書体の行間を持つ書体（游ゴシック）の横書きで、選択肢の帯が字の箱の中央に在るかを測る。
//! 縦書きの前例 `viewbox_draw_vertical_band_readback_tests.rs` と同じく、本番と同じ道
//! （`FontCatalog::pick` → 配置 → `line_bands` → 描く → 読み戻す）で回す。
//!
//! 記号（design.md「Data Models」・image px・行矩形の上端 0）: `B` 字の箱の丈（`line_box_height`）・
//! `t` 字の箱の上端（基線 − ascent）・`E` 帯の厚み・案 2 の寄せ `round(max(0, t) + max(0, B − E) / 2)`。

use super::ViewboxExecutor;
use super::choice_hover_tests::bgra_x_span;
use super::test_support::{Rig, block_axis_ink_span, glyph_items, live_diff_model_font};
use super::vertical_band_readback_tests::{R_FILL_BGRA, R_FONTS, bgra_y_span};
use crate::canvas::{ContentCanvas, ResidentContent};
use crate::choice::{
    LineBand, ResolvedChoiceStyle, annotate_lines, decorate_canvas, derive_hit_rows,
    highlight_band_offset, line_bands, to_window_physical,
};
use crate::cursor_tag::CursorWarnGuard;
use crate::draw::{
    DWriteMetrics, FontCatalog, ResolvedFont, create_line_layout, create_text_format,
    measure_font_metrics,
};
use crate::layout::{FixedMetrics, GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::look::{GlyphStyles, StyleId, StyleTable, TextLook};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{ChoiceSpan, SpanKind, TextItem, TextLayerConfig};
use crate::writing::WritingMode;
use areka_sakura::contract::ActorKey;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_LINE_METRICS, IDWriteFactory2,
};
use wintf::com::dwrite::dwrite_create_factory;

/// 測る書体（本番と同じ `FontCatalog::pick` に通す）。先頭が書体の行間を持つ書体。
const GAP_FONTS: [&str; 3] = ["游ゴシック", "Yu Gothic UI", "ＭＳ ゴシック"];
/// 仮名の行と漢字の行（同じ大きさの隣り合う 2 行）。
const GAP_WORDS: [&str; 2] = ["あいうえお", "漢字選択肢"];

/// 案 2 の帯の寄せ（design.md「Data Models」の式・丸め 1 回）。
fn plan2_offset(t: f32, b: f32, e: f32) -> f32 {
    (t.max(0.0) + (b - e).max(0.0) / 2.0).round()
}

/// 書体の字の箱 `(t, ascent + descent)`（image px）。基線は本番と同じ生成口の「あ」1 字の
/// TextLayout から、ascent・descent は書体の寸法から取る（帯の式の値は使わない）。記録の行も出す。
fn char_box(factory: &IDWriteFactory2, picked: &ResolvedFont, em: f32, tag: &str) -> (f32, f32) {
    let mode = WritingMode::HorizontalTb;
    let format = create_text_format(factory, picked, mode).expect("TextFormat 生成");
    let layout = create_line_layout(factory, "あ", &format, em, mode).expect("create_line_layout");
    let mut lm = [DWRITE_LINE_METRICS::default(); 4];
    let mut count = 0;
    unsafe { layout.GetLineMetrics(Some(&mut lm), &mut count) }.expect("GetLineMetrics");
    assert!(count > 0, "{tag} 行が 0 本");
    let fm = measure_font_metrics(factory, &format).expect("書体の寸法");
    let upem = f32::from(fm.designUnitsPerEm);
    let (ascent, descent) = (
        f32::from(fm.ascent) / upem * em,
        f32::from(fm.descent) / upem * em,
    );
    let t = lm[0].baseline - ascent;
    println!(
        "{tag} DirectWrite 行の高さ {:.3}・基線 {:.3}・ascent {ascent:.3}・descent {descent:.3}（upem {}）・\
         字の箱 {t:.3}..{:.3}（丈 {:.3}）・t {t:.4}",
        lm[0].height,
        lm[0].baseline,
        fm.designUnitsPerEm,
        t + ascent + descent,
        ascent + descent,
    );
    (t, ascent + descent)
}

/// 帯 1 つの記録: 物理画素の範囲・インクとの上下の余白（正＝覆う）・字の箱の中心との差（image px）。
/// インクを上下とも覆うかを返す。
fn band_record(
    label: &str,
    line: &PositionedLine,
    offset: f32,
    extent: f32,
    ink: Option<(u32, u32)>,
    box_centre: f32,
    map: impl Fn(f32) -> f32,
) -> bool {
    let (top, bottom) = (
        map(line.rect.top + offset),
        map(line.rect.top + offset + extent),
    );
    let diff = offset + extent / 2.0 - box_centre;
    let Some((i0, i1)) = ink else {
        println!("    {label}: 帯 y{top}..{bottom}・インク無し・中心差 {diff:+.3}");
        return false;
    };
    let (above, below) = (i0 as f32 - top, bottom - (i1 + 1) as f32);
    println!(
        "    {label}: 寄せ {offset}・厚み {extent}・帯 y{top}..{bottom}（物理）・インク y{i0}..{i1}・\
         余白 上 {above}・下 {below}・字の箱の中心との差 {diff:+.3} image px"
    );
    above >= 0.0 && below >= 0.0
}

/// 1 つの組（書体 × 大きさ × 拡大率）を測って標準出力へ出す（合否なし）。
fn record_case(rig: &mut Rig, factory: &IDWriteFactory2, name: &str, size: u32, k: f32) {
    let mode = WritingMode::HorizontalTb;
    let em = size as f32;
    let model = live_diff_model_font(Some(name), Some(size));
    let font = ResolvedFont::resolve(&model);
    let picked = FontCatalog::new(factory)
        .expect("FontCatalog 生成")
        .pick(&font);
    let tag = format!("[{name}→{} {size} k={k}]", picked.name);
    let (t, char_box_height) = char_box(factory, &picked, em, &tag);
    let metrics = DWriteMetrics::new(factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成");
    let b = metrics.line_box_height(em);

    let image = ((em * 7.0) as u32, (em * 4.0) as u32);
    let region = TextRegion::resolve(&model, image, mode);
    let mut items = Vec::new();
    for word in GAP_WORDS {
        if !items.is_empty() {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.extend(glyph_items(word));
    }
    let glyphs = items
        .iter()
        .filter(|i| matches!(i, TextItem::Glyph { .. }))
        .count();
    let lines = LayoutEngine::layout(
        &items,
        glyphs,
        &region,
        mode,
        em,
        &metrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), GAP_WORDS.len(), "{tag} 2 行に並ぶ");
    let window = LayoutEngine::visible_window(&lines, &region, mode);
    let bands = line_bands(&lines, mode, &metrics);
    let base = ContentCanvas::from_layout(&lines, &region, mode);
    let contract = ScaleContract::new(k, None);

    // 行ごとのインク（その行の住人だけを描いて読む）と、面の物理画素への写像の足し分 committed。
    let mut committed = 0;
    let ink: Vec<Option<(u32, u32)>> = (0..lines.len())
        .map(|i| {
            let only = ContentCanvas {
                residents: vec![base.residents[i].clone()],
                size: base.size,
            };
            let mut surface = rig.attach(image, k);
            let (w, h) = surface.size();
            let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
            exec.render(&only, &window, &font, mode, &contract, &mut surface)
                .expect("横書きの描画に失敗");
            committed = exec.scroll_state().committed;
            let px = surface.read_back().expect("read_back 失敗");
            block_axis_ink_span(&px, w, h, mode)
        })
        .collect();
    // ブロック軸の写像（`to_window_physical` と同じ式・面は原点 (0,0) に装着）。
    let map = |y: f32| (y - region.top()) * k + committed as f32;

    let box_centre = t + char_box_height / 2.0;
    let e = bands[0].extent;
    let plan2 = plan2_offset(t, b, e);
    println!(
        "{tag} B {b:.3}・行送り {}・帯の厚み E {e}・今の寄せ {}・案 2 の寄せ {plan2}・字の箱の中心 {box_centre:.3}",
        metrics.line_pitch(em),
        bands[0].offset
    );
    let mut covers = true;
    for (i, line) in lines.iter().enumerate() {
        println!(
            "{tag} 行 {}: 行矩形 y{}..{}（物理）",
            GAP_WORDS[i],
            map(line.rect.top),
            map(line.rect.bottom)
        );
        band_record("今", line, bands[i].offset, e, ink[i], box_centre, map);
        covers &= band_record("案 2", line, plan2, e, ink[i], box_centre, map);
    }
    let not_shifted = (bands[0].offset + e / 2.0 - box_centre).abs() <= 1.0;
    let centred = (plan2 + e / 2.0 - box_centre).abs() <= 1.0;
    let gap = (lines[1].rect.top + plan2) - (lines[0].rect.top + plan2 + e);
    println!(
        "{tag} 判定: ずれていない（中心差 1 image px 以下）={not_shifted}・案 2 で満たせる={}\
         （インクを覆う {covers}・中心差 1 image px 以下 {centred}・隣の行の帯との間 {gap}）",
        covers && centred && gap >= 0.0
    );
    if k == 1.0 && name != GAP_FONTS[2] {
        big_line_overlap_record(factory, name, size, t / em, &tag);
    }
}

/// `\f[height,40]` の行の次に既定の大きさの行が来る組の帯の重なり（今と案 2・image px・正なら重なる）。
/// 記録だけ（設計の「開発者の決め」で範囲外・判定に入れない）。t は大きさに比例させる。
fn big_line_overlap_record(
    factory: &IDWriteFactory2,
    name: &str,
    size: u32,
    t_ratio: f32,
    tag: &str,
) {
    let mode = WritingMode::HorizontalTb;
    let em = size as f32;
    let default = TextLook {
        name: vec![name.to_owned()],
        height: em,
        ..TextLook::ukadoc_default()
    };
    let mut table = StyleTable::default();
    let big = table.intern(
        &TextLook {
            height: 40.0,
            ..default.clone()
        },
        &default,
    );
    let mut items = Vec::new();
    let mut ids = Vec::new();
    for (word, id) in [(GAP_WORDS[0], big), (GAP_WORDS[1], StyleId::DEFAULT)] {
        if !items.is_empty() {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        let word_items = glyph_items(word);
        ids.extend(std::iter::repeat_n(id, word_items.len()));
        items.extend(word_items);
    }
    let model = live_diff_model_font(Some(name), Some(size));
    let font = ResolvedFont::resolve(&model);
    let metrics = DWriteMetrics::new(factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成");
    // 40 の行が折り返さない幅（既定の em の 7 倍では足りない）。
    let region = TextRegion::resolve(&model, (40 * 7, 200), mode);
    let styles = GlyphStyles {
        table: &table,
        ids: &ids,
        default: &default,
        current: &default,
    };
    let lines = LayoutEngine::layout_styled(
        &items,
        ids.len(),
        &region,
        mode,
        em,
        &metrics,
        WrapPlan::CharByChar,
        styles,
        &ActorKey::from("0"),
        &mut CursorWarnGuard::default(),
    );
    assert_eq!(lines.len(), 2, "{tag} 大きい行と既定の行の 2 行");
    let bands = line_bands(&lines, mode, &metrics);
    let plan2: Vec<f32> = lines
        .iter()
        .zip(&bands)
        .map(|(line, band)| {
            let h = line.rect.bottom - line.rect.top;
            plan2_offset(t_ratio * h, metrics.line_box_height(h), band.extent)
        })
        .collect();
    let overlap = |o0: f32, o1: f32| {
        let LineBand { extent, .. } = bands[0];
        (lines[0].rect.top + o0 + extent) - (lines[1].rect.top + o1)
    };
    println!(
        "{tag} \\f[height,40] の行（行矩形 y{}..{}・帯 {:?}）の次の既定の行（y{}..{}・帯 {:?}）: \
         帯の重なり 今 {}・案 2 {}（寄せ {plan2:?}）",
        lines[0].rect.top,
        lines[0].rect.bottom,
        bands[0],
        lines[1].rect.top,
        lines[1].rect.bottom,
        bands[1],
        overlap(bands[0].offset, bands[1].offset),
        overlap(plan2[0], plan2[1]),
    );
}

/// 記録だけの測り（要件 1.1・1.2・1.3・1.5）: {游ゴシック・Yu Gothic UI・ＭＳ ゴシック} × {20, 28} ×
/// k ＝ {1, 2} の横書きで、字の箱・インク・今の帯と案 2 の帯の余白と判定を標準出力へ出す
/// （`research.md` §3 へ写す）。合否は無い。
#[test]
#[ignore = "実 GPU の記録だけ（emo-text-line-gap 1.2）: 横書きの帯と字の箱を測る・合否なし"]
fn horizontal_line_gap_band_record() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let mut rig = Rig::new();
    for name in GAP_FONTS {
        for size in [20u32, 28] {
            for k in [1.0f32, 2.0] {
                record_case(&mut rig, &factory, name, size, k);
            }
        }
    }
}

/// 書体の行間を持つ偽の計測器（游ゴシックの見込みの数: 字の箱の丈 B ＝ 1.287·h・字の箱の上端
/// t ＝ 0.315·h・行送り h ＋ 2）。字幅は `FixedMetrics` と同じ。
struct LineGapMetrics;

impl GlyphMetrics for LineGapMetrics {
    fn advance(&self, text: &str, h: f32) -> f32 {
        FixedMetrics.advance(text, h)
    }
    fn line_pitch(&self, h: f32) -> f32 {
        h + 2.0
    }
    fn line_box_height(&self, h: f32) -> f32 {
        1.287 * h
    }
    fn horizontal_char_box_top(&self, h: f32) -> f32 {
        0.315 * h
    }
}

/// 決定論のテスト（要件 2.1・2.2・2.3・2.5・3.1・3.3・6.1・GPU なし）: 書体の行間を持つ偽の
/// 計測器で、同じ大きさの横書き 2 行（1 行ずつ選択肢／アンカー）の帯を本番と同じ道
/// （配置 → `choice_ranges`（`line_bands`・`derive_hit_rows`）→ `decorate_canvas`）で出し、
/// (a) 帯の中心が字の箱 [t, t ＋ B] の中心から 1 image px 以内、(b) 厚みが em 以上・行送り以下、
/// (c) 2 行の帯が接するだけで重ならない（境の点はたかだか 1 行に当たる）、(d) 当たりのブロック軸の
/// 範囲と焼いた帯が数値まで同じ、(e) アンカーの範囲も選択肢と同じ帯、を確かめる。
///
/// 較正（要件 6.3・research.md §4）: `line_bands` で t を読まないと (a) が赤（em 20 で中心差 −6.17）。
#[test]
fn line_gap_metrics_centre_the_horizontal_band_on_the_char_box() {
    let mode = WritingMode::HorizontalTb;
    let m = LineGapMetrics;
    for size in [20u32, 28] {
        let em = size as f32;
        let (t, b, pitch) = (
            m.horizontal_char_box_top(em),
            m.line_box_height(em),
            m.line_pitch(em),
        );
        assert!(t > 0.0 && b > em, "前提: 書体の行間を持つ（t {t}・B {b}）");
        let model = live_diff_model_font(None, Some(size));
        let region = TextRegion::resolve(&model, ((em * 7.0) as u32, (em * 4.0) as u32), mode);
        let mut items = glyph_items(GAP_WORDS[0]);
        items.push(TextItem::LineBreak { ratio: 1.0 });
        items.extend(glyph_items(GAP_WORDS[1]));
        let lines = LayoutEngine::layout(&items, 10, &region, mode, em, &m, WrapPlan::CharByChar);
        assert_eq!(lines.len(), 2, "em {em}: 2 行に並ぶ");

        let mut rows_by_kind = Vec::new();
        for kind in [SpanKind::Choice, SpanKind::Anchor] {
            let span = |ordinal, glyph_range| ChoiceSpan {
                kind,
                ordinal,
                id: String::new(),
                label: String::new(),
                references: vec![],
                glyph_range,
            };
            let spans = [span(0, 0..5), span(1, 5..10)];
            let ranges = crate::actor::choice_ranges(&lines, &spans, mode, &region, &m);
            let bands = &ranges.bands;
            for (i, band) in bands.iter().enumerate() {
                // (a) 字の箱の中心は帯の式ではなく偽の計測器の t・B から出す。
                let diff = band.offset + band.extent / 2.0 - (t + b / 2.0);
                assert!(
                    diff.abs() <= 1.0,
                    "em {em} {kind:?} 行 {i}: 帯 {band:?} の中心と字の箱の中心の差 {diff:+.3} が 1 image px を超える"
                );
                // (b)
                assert!(
                    band.extent >= em && band.extent <= pitch,
                    "em {em} {kind:?} 行 {i}: 厚み {} が em {em}〜行送り {pitch} の外",
                    band.extent
                );
            }
            // (c) 前の行の帯の遠端と次の行の帯の近端が同じ（接する・重ならない）。
            assert_eq!(
                lines[0].rect.top + bands[0].offset + bands[0].extent,
                lines[1].rect.top + bands[1].offset,
                "em {em} {kind:?}: 2 行の帯が接するだけでない: {bands:?}"
            );
            // (d) 焼いた帯（住人原点 ＋ 寄せ 〜 ＋ 厚み）と当たりのブロック軸の範囲。
            let rows = ranges.hit_rows();
            assert_eq!(rows.len(), 2, "em {em} {kind:?}: 当たりの行 2 つ");
            let out = decorate_canvas(
                ContentCanvas::from_layout(&lines, &region, mode),
                &ranges.segments,
                None,
                ResolvedChoiceStyle::NoMarker,
                (0, 0, 0),
                &region,
                mode,
                bands,
            );
            for (seg, row) in ranges.segments.iter().zip(&rows) {
                let resident = &out.residents[seg.line_index];
                let ResidentContent::Choice(c) = &resident.content else {
                    panic!(
                        "em {em} {kind:?}: 行 {} が Choice 住人でない",
                        seg.line_index
                    );
                };
                let y0 = resident.transform.offset().1 + c.band_offset;
                assert_eq!(
                    (y0, y0 + c.band_extent),
                    (row.rect.top, row.rect.bottom),
                    "em {em} {kind:?}: 行 {} の焼いた帯と当たりのブロック軸の範囲が違う",
                    seg.line_index
                );
            }
            // 境の点（半開区間 `y >= top && y < bottom`）はたかだか 1 行に当たる（要件 3.3）。
            let x = rows[0].rect.left;
            for y in [rows[0].rect.bottom, rows[0].rect.bottom - 0.5] {
                let hits = rows
                    .iter()
                    .filter(|r| {
                        let q = r.rect;
                        x >= q.left && x < q.right && y >= q.top && y < q.bottom
                    })
                    .count();
                assert_eq!(
                    hits, 1,
                    "em {em} {kind:?}: 点 ({x},{y}) が {hits} 行に当たる"
                );
            }
            rows_by_kind.push(rows);
        }
        // (e) アンカーの当たりは選択肢と同じ帯。
        assert_eq!(
            rows_by_kind[0], rows_by_kind[1],
            "em {em}: アンカーと選択肢の帯"
        );
    }
}

/// 実物の読み戻しの 1 組（書体 × 大きさ × 拡大率）。本番と同じ道（配置 → `line_bands` → 塗りの住人へ
/// 焼く → 描く → 読み戻す）で横書きの選択肢 2 行を描き、赤になった確かめを返す。
/// `expect` が `Some` の書体は書体の行間を持つ書体として (a)〜(f) を、`None` の書体は行間 0 の書体と
/// して (g) を確かめる。測った数は標準出力へ出す（`research.md` §5 の材料）。
fn readback_case(
    rig: &mut Rig,
    factory: &IDWriteFactory2,
    name: &str,
    expect: Option<&str>,
    size: u32,
    k: f32,
) -> Vec<String> {
    let mode = WritingMode::HorizontalTb;
    let em = size as f32;
    let mut red = Vec::new();
    let model = live_diff_model_font(Some(name), Some(size));
    let font = ResolvedFont::resolve(&model);
    let picked = FontCatalog::new(factory)
        .expect("FontCatalog 生成")
        .pick(&font);
    let tag = format!("[{} {size} k={k}]", picked.name);
    let metrics = DWriteMetrics::new(factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成");
    // 本番の計測器が測った字の箱の上端（帯の式が読む値）。
    let t = metrics.horizontal_char_box_top(em);
    let b = metrics.line_box_height(em);

    let image = ((em * 7.0) as u32, (em * 4.0) as u32);
    let region = TextRegion::resolve(&model, image, mode);
    let mut items = Vec::new();
    let mut spans = Vec::new();
    let mut glyphs = 0usize;
    for (ordinal, word) in GAP_WORDS.iter().enumerate() {
        if !items.is_empty() {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        let word_items = glyph_items(word);
        let start = glyphs;
        glyphs += word_items.len();
        items.extend(word_items);
        spans.push(ChoiceSpan {
            kind: SpanKind::Choice,
            ordinal,
            id: format!("OnLgTest{}", ordinal + 1),
            label: (*word).to_owned(),
            references: Vec::new(),
            glyph_range: start..glyphs,
        });
    }
    let lines = LayoutEngine::layout(
        &items,
        glyphs,
        &region,
        mode,
        em,
        &metrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), GAP_WORDS.len(), "{tag} 2 行に並ぶ");
    let bands = line_bands(&lines, mode, &metrics);
    println!("{tag} t {t:.4}・B {b:.3}・帯 {bands:?}");

    let Some(expect) = expect else {
        // (g) 行間 0 の書体: 本番の寄せが t ＝ 0 の式の値と等しい（要件 5.1）。
        for (i, band) in bands.iter().enumerate() {
            let want = highlight_band_offset(b, band.extent);
            if band.offset != want {
                red.push(format!(
                    "{tag} 行 {i} (g): 寄せ {} が t ＝ 0 の式の値 {want} でない（t {t:.4}）",
                    band.offset
                ));
            }
        }
        return red;
    };
    // (a) 本番の解決で引けた書体の名前（代わりの書体のまま緑にしない）。
    if picked.name != expect {
        red.push(format!(
            "{tag} (a): 書体 {name} が {} に引けた（{expect} のはず）",
            picked.name
        ));
    }
    // (b) 測った比が正（行間 0 の書体へ縮退していない）。
    if t <= 0.0 {
        red.push(format!("{tag} (b): 字の箱の上端 t {t} が正でない"));
        return red;
    }
    // (d) の基準: 字の箱は帯の式でなく DirectWrite の基線 − ascent から出す。
    let (box_top, box_height) = char_box(factory, &picked, em, &tag);

    let window = LayoutEngine::visible_window(&lines, &region, mode);
    let segments = annotate_lines(&lines, &spans);
    let base = ContentCanvas::from_layout(&lines, &region, mode);
    let contract = ScaleContract::new(k, None);
    let style = ResolvedChoiceStyle::SquareFill {
        fill: (105, 25, 25),
        text: (255, 255, 255),
    };
    let decorate = |hovered: Option<usize>| {
        decorate_canvas(
            base.clone(),
            &segments,
            hovered,
            style,
            font.color,
            &region,
            mode,
            &bands,
        )
    };
    // 乗せていない面: 行ごとのインク（その行の住人だけを描いて読む）。
    let ink: Vec<Option<(u32, u32)>> = (0..lines.len())
        .map(|i| {
            let only = ContentCanvas {
                residents: vec![base.residents[i].clone()],
                size: base.size,
            };
            let mut surface = rig.attach(image, k);
            let (w, h) = surface.size();
            let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
            exec.render(&only, &window, &font, mode, &contract, &mut surface)
                .expect("横書きの描画に失敗");
            let px = surface.read_back().expect("read_back 失敗");
            block_axis_ink_span(&px, w, h, mode)
        })
        .collect();
    let hit_rows = derive_hit_rows(&lines, &segments, mode, &region, &bands);
    let plain = decorate(None);

    for seg in &segments {
        let word = GAP_WORDS[seg.ordinal];
        let line = &lines[seg.line_index];
        let band = bands[seg.line_index];
        // 乗せたフレームと、同じ描き手で hover を外したフレーム（キャッシュを使い回す本番の形）。
        let mut surface = rig.attach(image, k);
        let (w, h) = surface.size();
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
        let hovered = decorate(Some(seg.ordinal));
        exec.render(&hovered, &window, &font, mode, &contract, &mut surface)
            .expect("乗せたフレームの描画に失敗");
        let px = surface.read_back().expect("read_back(乗せた) 失敗");
        let committed = exec.scroll_state().committed;
        exec.render(&plain, &window, &font, mode, &contract, &mut surface)
            .expect("hover を外したフレームの描画に失敗");
        let off = surface.read_back().expect("read_back(外した) 失敗");

        let (Some(fx), Some(fy)) = (
            bgra_x_span(&px, w, h, R_FILL_BGRA),
            bgra_y_span(&px, w, h, R_FILL_BGRA),
        ) else {
            red.push(format!("{tag} {word}: 乗せたのに塗りが 1 画素も無い"));
            continue;
        };
        println!(
            "{tag} {word} に乗せた: 塗り x{}..{}・y{}..{}・インク {:?}",
            fx.0, fx.1, fy.0, fy.1, ink[seg.line_index]
        );
        // (c) 帯の塗りが行のインク（仮名の行・漢字の行）を上下とも覆う。
        match ink[seg.line_index] {
            Some(i) if i.0 >= fy.0 && i.1 <= fy.1 => {}
            Some(i) => red.push(format!(
                "{tag} {word} (c): インク y{}..{} が塗り y{}..{} の外へ出る（上 {}・下 {}）",
                i.0,
                i.1,
                fy.0,
                fy.1,
                i64::from(fy.0) - i64::from(i.0),
                i64::from(i.1) - i64::from(fy.1)
            )),
            None => red.push(format!("{tag} {word}: 乗せていない面に行のインクが無い")),
        }
        // (e) 当たりの行の矩形（物理）が塗りの範囲を含む＝帯の中のインクの上の点が当たる。
        let row = hit_rows
            .iter()
            .find(|r| r.ordinal == seg.ordinal)
            .expect("選択肢ごとに当たりの行が 1 つ");
        let hit = to_window_physical(row, &region, mode, committed, &contract, (0.0, 0.0));
        let (x1, y1) = ((fx.1 + 1) as f32, (fy.1 + 1) as f32);
        let contains = hit.left <= fx.0 as f32
            && hit.top <= fy.0 as f32
            && hit.right >= x1
            && hit.bottom >= y1;
        if !contains {
            red.push(format!(
                "{tag} {word} (e): 当たり {hit:?} が塗り x{}..{x1}・y{}..{y1} を含まない",
                fx.0, fy.0
            ));
        }
        // (d) 塗りの中心と字の箱の中心の差が k 物理画素以内。写像の足し分は当たりの矩形から取る。
        let shift = hit.top - k * (line.rect.top + band.offset);
        let box_centre = shift + k * (line.rect.top + box_top + box_height / 2.0);
        let fill_centre = (fy.0 + fy.1 + 1) as f32 / 2.0;
        println!("{tag} {word}: 塗りの中心 {fill_centre:.2}・字の箱の中心 {box_centre:.2}");
        if (fill_centre - box_centre).abs() > k {
            red.push(format!(
                "{tag} {word} (d): 塗りの中心 {fill_centre:.2} が字の箱の中心 {box_centre:.2} から {k} 物理画素より離れる"
            ));
        }
        // (f) hover を外したフレームに塗りが残らない（ダーティ帯が遠端まで広がる）。
        if let Some(rest) = bgra_y_span(&off, w, h, R_FILL_BGRA) {
            red.push(format!(
                "{tag} {word} (f): hover を外したフレームに塗りが y{}..{} に残る",
                rest.0, rest.1
            ));
        }
    }
    red
}

/// 実物の読み戻し（要件 2.1・2.2・3.2・5.1・6.2・6.4）: 游ゴシック（ぱすたの候補列・`R_FONTS` と
/// 同じ固定で `Yu Gothic` に引ける）20・28 × k ＝ 1・2 の横書きの選択肢 2 行で (a)〜(f) を、行間 0 の
/// 書体（Yu Gothic UI・ＭＳ ゴシック）の同じ組で (g) を確かめる。
///
/// 赤の確かめは組ごとに集めて最後に 1 度だけ落とす。測った数は標準出力へ出る（`--nocapture`）。
#[test]
fn line_gap_band_covers_char_box_readback() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let mut rig = Rig::new();
    let (yu_gothic, expect) = R_FONTS[1];
    let fonts = [
        (yu_gothic, Some(expect)),
        (GAP_FONTS[1], None),
        (GAP_FONTS[2], None),
    ];
    let mut red = Vec::new();
    for (name, expect) in fonts {
        for size in [20u32, 28] {
            for k in [1.0f32, 2.0] {
                red.extend(readback_case(&mut rig, &factory, name, expect, size, k));
            }
        }
    }
    assert!(
        red.is_empty(),
        "横書きの帯の読み戻しで {} 件が赤:\n{}",
        red.len(),
        red.join("\n")
    );
}
