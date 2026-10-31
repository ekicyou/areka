//! # 横書きの帯と書体の行間（spec emo-text-line-gap・設計の LineGapBandTests）
//!
//! 書体の行間を持つ書体（游ゴシック）の横書きで、選択肢の帯が字の箱の中央に在るかを測る。
//! 縦書きの前例 `viewbox_draw_vertical_band_readback_tests.rs` と同じく、本番と同じ道
//! （`FontCatalog::pick` → 配置 → `line_bands` → 描く → 読み戻す）で回す。
//!
//! 記号（design.md「Data Models」・image px・行矩形の上端 0）: `B` 字の箱の丈（`line_box_height`）・
//! `t` 字の箱の上端（基線 − ascent）・`E` 帯の厚み・案 2 の寄せ `round(max(0, t) + max(0, B − E) / 2)`。

use super::ViewboxExecutor;
use super::test_support::{Rig, block_axis_ink_span, glyph_items, live_diff_model_font};
use crate::canvas::ContentCanvas;
use crate::choice::{LineBand, line_bands};
use crate::cursor_tag::CursorWarnGuard;
use crate::draw::{
    DWriteMetrics, FontCatalog, ResolvedFont, create_line_layout, create_text_format,
    measure_font_metrics,
};
use crate::layout::{GlyphMetrics, LayoutEngine, PositionedLine, WrapPlan};
use crate::look::{GlyphStyles, StyleId, StyleTable, TextLook};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{TextItem, TextLayerConfig};
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
