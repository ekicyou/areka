//! # 縦書きの帯の実フォントの読み戻し（spec vertical-choice-hover-band・設計の R）
//!
//! 帯が乗せた列の字のインクを全幅で覆い、厚みが em × k、塗りの中心が書体から測った em の箱の
//! 中心に在ることを、Yu Gothic UI とぱすたの縦書きのバルーンの `font.name` の候補列（本番と同じ
//! 解決で「游ゴシック」に引ける）で読み戻して見る。`viewbox_draw_choice_hover_tests.rs` が
//! 1,000 行に届くので兄弟のファイルへ分けた（2.2 の `viewbox_draw_vertical_band_scroll_tests.rs` と同じ扱い）。

use super::ViewboxExecutor;
use super::choice_hover_tests::bgra_x_span;
use super::test_support::{Rig, block_axis_ink_span, glyph_items, live_diff_model_font};
use crate::canvas::ContentCanvas;
use crate::choice::{
    ResolvedChoiceStyle, annotate_lines, decorate_canvas, derive_hit_rows, line_bands,
    to_window_physical,
};
use crate::draw::{DWriteMetrics, FontCatalog, PROBE_MAX_EXTENT, ResolvedFont, create_text_format};
use crate::layout::{GlyphMetrics, LayoutEngine, WrapPlan};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{ChoiceSpan, SpanKind, TextItem, TextLayerConfig};
use crate::writing::WritingMode;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_LINE_METRICS, DWRITE_TEXT_METRICS, IDWriteFactory2,
};
use windows::core::HSTRING;
use wintf::com::dwrite::{DWriteFactoryExt, dwrite_create_factory};

/// R の書体（本番と同じ解決に通す）と、引けるはずの書体の名前（違えば赤）。ぱすたの
/// `pasta_tategaki` の `font.name` は候補列のまま書き、`FontCatalog::pick` で先頭の「游ゴシック Medium」が
/// GDI の名前として引け、その家族名「Yu Gothic」になる（emo-text-dwrite-fidelity 要件 3.1・タスク 1.2）。
pub(super) const R_FONTS: [(&str, &str); 2] = [
    ("Yu Gothic UI", "Yu Gothic UI"),
    (
        "游ゴシック Medium,Yu Gothic Medium,游ゴシック,Yu Gothic,BIZ UDPゴシック,メイリオ",
        "Yu Gothic",
    ),
];
/// ぱすたの測りと同じ 4 つの選択肢（列ごと）。
pub(super) const R_CHOICES: [&str; 4] = ["晴れてる", "雨だよ", "曇ってる", "ないしょ"];
/// 記録だけの横倒しの欧文の列（選択肢にしない・合否に入れない）。
pub(super) const R_LATIN: &str = "Ag";
/// 塗りの色（premultiplied BGRA・α=255）＝ `SquareFill { fill: (105, 25, 25) }`。
pub(super) const R_FILL_BGRA: [u8; 4] = [25, 25, 105, 255];

/// premultiplied BGRA 密配列の全域から、指定 BGRA を持つ画素の y 範囲（上端, 下端）を返す。
pub(super) fn bgra_y_span(bytes: &[u8], w: u32, h: u32, target: [u8; 4]) -> Option<(u32, u32)> {
    let rows: Vec<u32> = (0..h)
        .filter(|&y| {
            (0..w).any(|x| {
                let o = ((y * w + x) * 4) as usize;
                bytes[o..o + 4] == target
            })
        })
        .collect();
    Some((*rows.first()?, *rows.last()?))
}

/// 1 つの組（書体 × 大きさ × 向き × 拡大率）を本番と同じ道（配置 → 帯の決め → 塗りの住人へ
/// 焼く → 描く）で読み戻し、赤になった確かめを返す。測った数は標準出力へ出す（`research.md` の材料）。
fn vertical_band_case(
    rig: &mut Rig,
    factory: &IDWriteFactory2,
    (name, expect): (&str, &str),
    size: u32,
    mode: WritingMode,
    k: f32,
) -> Vec<String> {
    let em = size as f32;
    let mut red = Vec::new();
    let model = live_diff_model_font(Some(name), Some(size));
    let font = ResolvedFont::resolve(&model);
    let picked = FontCatalog::new(factory)
        .expect("FontCatalog 生成")
        .pick(&font);
    let tag = format!("[{} {size} {mode:?} k={k}]", picked.name);
    // 5.3: 候補列が本番の解決で引けた書体の名前を固定する（5.1 で確かめた名前）。
    if picked.name != expect {
        red.push(format!(
            "{tag} 5.3: 書体 {name} が {} に引けた（{expect} のはず）",
            picked.name
        ));
    }
    let metrics = DWriteMetrics::new(factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成");
    let line_box = metrics.line_box_height(em);
    // 前置き: 代替（ＭＳ ゴシック・比ちょうど 1）へ縮退すると行の箱 == em。飛ばさず赤にする。
    if line_box <= em {
        red.push(format!(
            "{tag} 前置き: 書体が引けず代替へ縮退した（行の箱 {line_box} ≦ em {em}）。比 1 より大きい実フォントが前提なので赤にする"
        ));
        return red;
    }

    let image = (((em + 2.0) * 6.0) as u32, (em * 5.0) as u32);
    let region = TextRegion::resolve(&model, image, mode);
    let words: Vec<&str> = R_CHOICES.iter().copied().chain([R_LATIN]).collect();
    let mut items = Vec::new();
    let mut spans = Vec::new();
    let mut glyphs = 0usize;
    for (ordinal, word) in words.iter().enumerate() {
        if !items.is_empty() {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        let word_items = glyph_items(word);
        let start = glyphs;
        glyphs += word_items.len();
        items.extend(word_items);
        if ordinal < R_CHOICES.len() {
            spans.push(ChoiceSpan {
                kind: SpanKind::Choice,
                ordinal,
                id: format!("OnVcTest{}", ordinal + 1),
                label: (*word).to_owned(),
                references: Vec::new(),
                glyph_range: start..glyphs,
            });
        }
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
    assert_eq!(
        lines.len(),
        words.len(),
        "{tag} 4 つの選択肢＋\"Ag\" の 5 列"
    );
    let window = LayoutEngine::visible_window(&lines, &region, mode);
    let bands = line_bands(&lines, mode, &metrics);
    let em_left = metrics.vertical_em_box_left(em, mode);
    let segments = annotate_lines(&lines, &spans);
    let base = ContentCanvas::from_layout(&lines, &region, mode);
    let contract = ScaleContract::new(k, None);
    let style = ResolvedChoiceStyle::SquareFill {
        fill: (105, 25, 25),
        text: (255, 255, 255),
    };
    let draw = |rig: &mut Rig, canvas: &ContentCanvas| -> (Vec<u8>, u32, u32, i32) {
        let mut surface = rig.attach(image, k);
        let (w, h) = surface.size();
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
        exec.render(canvas, &window, &font, mode, &contract, &mut surface)
            .expect("縦書きの読み戻しの描画に失敗");
        let px = surface.read_back().expect("read_back 失敗");
        (px, w, h, exec.scroll_state().committed)
    };

    println!(
        "{tag} 行の箱 {line_box:.2}・行送り {pitch}・s {s:.2}・測った em の箱の左 {em_left:.2}・帯 {bands:?}",
        pitch = metrics.line_pitch(em),
        s = line_box - em,
    );
    // 記録だけ: 行の TextLayout（本番と同じ書式・幅 em）の位置と幅、行の高さとベースライン。
    let format = create_text_format(factory, &picked, mode).expect("TextFormat 生成");
    for word in &words {
        let layout = factory
            .create_text_layout(&HSTRING::from(*word), &format, em, PROBE_MAX_EXTENT)
            .expect("CreateTextLayout");
        let mut tm = DWRITE_TEXT_METRICS::default();
        unsafe { layout.GetMetrics(&mut tm) }.expect("GetMetrics");
        let mut lm = [DWRITE_LINE_METRICS::default(); 4];
        let mut count = 0;
        unsafe { layout.GetLineMetrics(Some(&mut lm), &mut count) }.expect("GetLineMetrics");
        println!(
            "{tag} TextLayout {word}: left {:.2}・width {:.2}・行の高さ {:.2}・baseline {:.2}",
            tm.left, tm.width, lm[0].height, lm[0].baseline
        );
    }

    // 乗せていない面: 列ごとのインク（その列の住人だけを描いて読む）。
    let ink: Vec<Option<(u32, u32)>> = (0..lines.len())
        .map(|i| {
            let only = ContentCanvas {
                residents: vec![base.residents[i].clone()],
                size: base.size,
            };
            let (px, w, h, _) = draw(rig, &only);
            block_axis_ink_span(&px, w, h, mode)
        })
        .collect();
    for (i, line) in lines.iter().enumerate() {
        println!(
            "{tag} 列 {}: 行矩形 x{}..{}（物理）・インク {:?}",
            words[i],
            line.rect.left * k,
            line.rect.right * k,
            ink[i]
        );
    }

    // 乗せた面: 4 つの選択肢へ順に乗せる。
    let hit_rows = derive_hit_rows(&lines, &segments, mode, &region, &bands);
    // 行矩形の左辺（論理）から面の物理画素への写像の足し分（領域の原点 × k ＋ committed）。当たりの四角から取る。
    let mut shift = None;
    for seg in &segments {
        let word = words[seg.ordinal];
        let canvas = decorate_canvas(
            base.clone(),
            &segments,
            Some(seg.ordinal),
            style,
            font.color,
            &region,
            mode,
            &bands,
        );
        let (px, w, h, committed) = draw(rig, &canvas);
        let (Some(fx), Some(fy)) = (
            bgra_x_span(&px, w, h, R_FILL_BGRA),
            bgra_y_span(&px, w, h, R_FILL_BGRA),
        ) else {
            red.push(format!("{tag} {word}: 乗せたのに塗りが 1 画素も無い"));
            continue;
        };
        println!(
            "{tag} {word} に乗せた: 塗り x{}..{}・y{}..{}",
            fx.0, fx.1, fy.0, fy.1
        );
        // 1.1・1.2: 乗せた列のインクが塗りの中に在る。
        match ink[seg.line_index] {
            Some(i) if i.0 >= fx.0 && i.1 <= fx.1 => {}
            Some(i) => red.push(format!(
                "{tag} {word} 1.1/1.2: インク x{}..{} が塗り x{}..{} の外へ出る（左 {}・右 {}）",
                i.0,
                i.1,
                fx.0,
                fx.1,
                i64::from(fx.0) - i64::from(i.0),
                i64::from(i.1) - i64::from(fx.1)
            )),
            None => red.push(format!("{tag} {word}: 乗せていない面に列のインクが無い")),
        }
        // 4.2: 塗りのブロック軸の幅が em × 拡大率。
        let width = fx.1 - fx.0 + 1;
        let want = (em * k).round() as u32;
        if width != want {
            red.push(format!(
                "{tag} {word} 4.2: 塗りの幅 {width} が em × k ＝ {want} でない"
            ));
        }
        // 1.4: 隣の列のインクと交わらない（"Ag" の列は記録だけ＝合否に入れない・設計の行き止まり）。
        for j in [seg.line_index.wrapping_sub(1), seg.line_index + 1] {
            if let Some(Some(n)) = ink.get(j)
                && n.0 <= fx.1
                && fx.0 <= n.1
            {
                let line = format!(
                    "{tag} {word} 1.4: 塗り x{}..{} が隣の列 {} のインク x{}..{} にかかる",
                    fx.0, fx.1, words[j], n.0, n.1
                );
                if j < R_CHOICES.len() {
                    red.push(line);
                } else {
                    println!("{line}（記録だけ）");
                }
            }
        }
        // 1.3: 行内の範囲が選択肢の字の範囲。
        let want_y = (
            ((seg.inline_range.0 - region.top()) * k).round() as u32,
            ((seg.inline_range.1 - region.top()) * k).round() as u32 - 1,
        );
        if fy != want_y {
            red.push(format!(
                "{tag} {word} 1.3: 塗り y{}..{} が字の範囲 y{}..{} でない",
                fy.0, fy.1, want_y.0, want_y.1
            ));
        }
        // 6.2: 当たりの四角が塗りと画素まで同じ（同じ帯の列 → derive_hit_rows → to_window_physical）。
        let row = hit_rows
            .iter()
            .find(|r| r.ordinal == seg.ordinal)
            .expect("選択肢ごとに当たりの行が 1 つ");
        let hit = to_window_physical(row, &region, mode, committed, &contract, (0.0, 0.0));
        let painted = (
            fx.0 as f32,
            fy.0 as f32,
            (fx.1 + 1) as f32,
            (fy.1 + 1) as f32,
        );
        if (hit.left, hit.top, hit.right, hit.bottom) != painted {
            red.push(format!(
                "{tag} {word} 6.2: 当たり {hit:?} が塗り x{}..{}・y{}..{} と同じでない",
                painted.0, painted.2, painted.1, painted.3
            ));
        }
        // 5.3（4.2）: 塗りの中心が、測った em の箱（行矩形の左辺 ＋ em の箱の左・厚み em）の中心から
        // 1 物理画素以内。帯の寄せに依らない写像の足し分を当たりの四角から取る。
        let t = hit.left - k * (lines[seg.line_index].rect.left + bands[seg.line_index].offset);
        shift = Some(t);
        let em_centre = t + k * (lines[seg.line_index].rect.left + em_left + em / 2.0);
        let fill_centre = (fx.0 + fx.1 + 1) as f32 / 2.0;
        println!("{tag} {word}: 塗りの中心 {fill_centre:.2}・em の箱の中心 {em_centre:.2}");
        if (fill_centre - em_centre).abs() > 1.0 {
            red.push(format!(
                "{tag} {word} 5.3: 塗りの中心 {fill_centre:.2} が測った em の箱の中心 {em_centre:.2} から 1 物理画素より離れる"
            ));
        }
    }
    // 記録だけ: "Ag" の列のインクの帯（同じ式で置いた所）からのはみ出し（正ならはみ出す画素の数）。
    let ag = R_CHOICES.len();
    if let (Some(t), Some(i)) = (shift, ink[ag]) {
        let l = t + k * (lines[ag].rect.left + bands[ag].offset);
        println!(
            "{tag} Ag（記録だけ）: インク x{}..{}・帯 x{l}..{}・はみ出し 左 {}・右 {}",
            i.0,
            i.1,
            l + k * em - 1.0,
            l - i.0 as f32,
            i.1 as f32 + 1.0 - (l + k * em)
        );
    }
    red
}

/// R（6.1・6.2・6.3・1.1・1.2・1.3・1.4・3.2・4.2）: 縦書きの帯が、Yu Gothic UI（比 1.33）とぱすたの候補列
/// （游ゴシック）× 大きさ 20・28 × 右から左・左から右 × 拡大率 1・2 で、乗せた列の字のインクを全幅で覆い、
/// 厚みが em × k、塗りの中心が測った em の箱の中心から 1 物理画素以内、隣の列のインクにかからず、
/// 行内は字の範囲で、当たりの四角が塗りと画素まで同じ。候補列が引けた書体の名前も固定する。
///
/// 赤の確かめは組ごとに集めて最後に 1 度だけ落とす（向きごとにどれが赤かを 1 回の走行で残す）。
/// 測った数は標準出力へ出る（`--nocapture`。赤のときは失敗の出力にも載る）。
#[test]
fn vertical_band_covers_column_ink_readback() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let mut rig = Rig::new();
    let mut red = Vec::new();
    for font in R_FONTS {
        for size in [20u32, 28] {
            for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
                for k in [1.0f32, 2.0] {
                    red.extend(vertical_band_case(&mut rig, &factory, font, size, mode, k));
                }
            }
        }
    }
    assert!(
        red.is_empty(),
        "縦書きの帯の読み戻しで {} 件が赤:\n{}",
        red.len(),
        red.join("\n")
    );
}
