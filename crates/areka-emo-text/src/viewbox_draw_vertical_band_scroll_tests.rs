//! # 縦書きの帯がスクロールに付いて行き、乗せ替えと解除で消し残さない（spec vertical-choice-hover-band・設計の S）
//!
//! 右から左の帯は行矩形の左へ出る（寄せが負）。描き替えの範囲（`expand_overhang_for_band`）が
//! その分を数えないと、乗せる列を移したフレームや解除のフレームで、前の帯の左の端が消えずに残る。
//! 列があふれて可視窓がスクロールした状態（`committed ≠ 0`）で、同じ executor に
//! 列 A に乗せる（1 枚目＝全域）→ 列 B に乗せる → 解除 の 3 フレームを描き、毎フレーム読み戻して見る。

use super::ViewboxExecutor;
use super::test_support::{Rig, glyph_items, live_diff_model_font, opaque_count};
use crate::canvas::ContentCanvas;
use crate::choice::{
    ResolvedChoiceStyle, annotate_lines, decorate_canvas, derive_hit_rows, line_bands,
    to_window_physical,
};
use crate::draw::{DWriteMetrics, ResolvedFont};
use crate::layout::{GlyphMetrics, LayoutEngine, WrapPlan};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{ChoiceSpan, SpanKind, TextItem, TextLayerConfig};
use crate::writing::WritingMode;
use windows::Win32::Graphics::DirectWrite::DWRITE_FACTORY_TYPE_SHARED;
use wintf::com::dwrite::dwrite_create_factory;

/// 列ごとの選択肢（8 列＝可視窓に収まらずスクロールする数・見えるのは列 4〜7）。
///
/// 乗せる列 4・5 は「ないしょ」——字のインクが列の左へほとんど出ない（Yu Gothic UI 28 で
/// 0.69 image px）ので、描き替えの範囲が帯の左の張り出し（寄せ −4）を数えるかどうかだけで
/// 消し残りが決まる。右から左ではその左隣（列 5 には 6・列 4 には 5）が同じフレームで
/// 描き替わらない順に乗せる（隣の範囲に助けられて緑にならないように）。
const WORDS: [&str; 8] = [
    "晴れてる",
    "雨だよ",
    "曇ってる",
    "晴れてる",
    "ないしょ",
    "ないしょ",
    "雨だよ",
    "曇ってる",
];
/// 乗せる順（列 A → 列 B → 解除）。
const HOVERS: [usize; 2] = [5, 4];
/// 塗りの色（premultiplied BGRA・α=255）＝ `SquareFill { fill: (105, 25, 25) }`。
const FILL_BGRA: [u8; 4] = [25, 25, 105, 255];
const EM: u32 = 28;

/// 塗りの色の画素の座標を全部返す。
fn fill_pixels(bytes: &[u8], w: u32, h: u32) -> Vec<(u32, u32)> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let o = ((y * w + x) * 4) as usize;
            bytes[o..o + 4] == FILL_BGRA
        })
        .collect()
}

/// 1 つの組（向き × 拡大率）を走らせ、赤になった確かめを返す。
fn scroll_case(rig: &mut Rig, mode: WritingMode, k: f32) -> Vec<String> {
    let tag = format!("[Yu Gothic UI {EM} {mode:?} k={k}]");
    let mut red = Vec::new();
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let em = EM as f32;
    let model = live_diff_model_font(Some("Yu Gothic UI"), Some(EM));
    let font = ResolvedFont::resolve(&model);
    let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics 生成");
    // 前置き: 代替へ縮退すると行の箱 == em で寄せが 0＝この檻は何も見なくなる。
    if metrics.line_box_height(em) <= em {
        red.push(format!("{tag} 前置き: 書体が引けず行の箱が em 以下"));
        return red;
    }

    // 3 列ほどしか入らない幅に 8 列＝あふれてスクロールする。
    let image = ((em + 2.0) as u32 * 4, EM * 5);
    let region = TextRegion::resolve(&model, image, mode);
    let mut items = Vec::new();
    let mut spans = Vec::new();
    let mut glyphs = 0usize;
    for (ordinal, word) in WORDS.iter().enumerate() {
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
            id: format!("OnVcScroll{ordinal}"),
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
    assert_eq!(lines.len(), WORDS.len(), "{tag} 8 列");
    let window = LayoutEngine::visible_window(&lines, &region, mode);
    let bands = line_bands(&lines, mode, &metrics);
    let segments = annotate_lines(&lines, &spans);
    let hit_rows = derive_hit_rows(&lines, &segments, mode, &region, &bands);
    let base = ContentCanvas::from_layout(&lines, &region, mode);
    let contract = ScaleContract::new(k, None);
    let style = ResolvedChoiceStyle::SquareFill {
        fill: (105, 25, 25),
        text: (255, 255, 255),
    };
    let canvas_for = |hover: Option<usize>| {
        decorate_canvas(
            base.clone(),
            &segments,
            hover,
            style,
            font.color,
            &region,
            mode,
            &bands,
        )
    };

    // 同じ executor・同じ供給面で 3 フレームを続けて描く（差分描画の描き替えの範囲を通す）。
    // 1 枚目（全域を描く）で列 A に乗せ、2 枚目で列 B へ移し、3 枚目で解除する——1 枚目の帯は
    // 描き替えの範囲で切られずに全部描かれるので、2 枚目の範囲が帯の左の張り出しを数えないと
    // そこが消えずに残る。
    let mut surface = rig.attach(image, k);
    let (w, h) = surface.size();
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    for ordinal in HOVERS {
        exec.render(
            &canvas_for(Some(ordinal)),
            &window,
            &font,
            mode,
            &contract,
            &mut surface,
        )
        .expect("乗せた描画に失敗");
        let committed = exec.scroll_state().committed;
        if committed == 0 {
            red.push(format!(
                "{tag} 前置き: 列があふれず可視窓がスクロールしていない"
            ));
            return red;
        }
        let px = surface.read_back().expect("read_back 失敗");
        let row = hit_rows
            .iter()
            .find(|r| r.ordinal == ordinal)
            .expect("選択肢ごとに当たりの行が 1 つ");
        let hit = to_window_physical(row, &region, mode, committed, &contract, (0.0, 0.0));
        let fill = fill_pixels(&px, w, h);
        if fill.is_empty() {
            red.push(format!("{tag} 列 {ordinal} に乗せたのに塗りが無い"));
            continue;
        }
        // 分数の倍率では塗りの端が当たりの四角と 1 画素ずれ得る（丸めの差）ので 1 画素だけ許す。
        let (l, t) = (hit.left.floor() - 1.0, hit.top.floor() - 1.0);
        let (r, b) = (hit.right.ceil() + 1.0, hit.bottom.ceil() + 1.0);
        let outside: Vec<_> = fill
            .iter()
            .filter(|&&(x, y)| {
                let (x, y) = (x as f32, y as f32);
                x < l || x >= r || y < t || y >= b
            })
            .collect();
        if !outside.is_empty() {
            red.push(format!(
                "{tag} 列 {ordinal} に乗せた: 帯 {hit:?} の外に塗りが {} 画素（先頭 {:?}）＝前の帯の消し残り",
                outside.len(),
                outside[0]
            ));
        }
        // 帯は描き替えの範囲で切られずに左右の端まで塗られる（範囲が張り出しを数えないと欠ける）。
        let (x0, x1) = fill
            .iter()
            .fold((u32::MAX, 0), |(a, b), &(x, _)| (a.min(x), b.max(x)));
        // 面の外へ出る分は描かれないので、帯の端は面の内へ収めて比べる。
        let (want0, want1) = (hit.left.max(0.0).ceil(), hit.right.min(w as f32).floor());
        if x0 as f32 > want0 || ((x1 + 1) as f32) < want1 {
            red.push(format!(
                "{tag} 列 {ordinal} に乗せた: 塗り x{x0}..{x1} が帯 {hit:?} の端まで届かない"
            ));
        }
    }

    // 解除: 面の全域で塗りが 0。
    exec.render(
        &canvas_for(None),
        &window,
        &font,
        mode,
        &contract,
        &mut surface,
    )
    .expect("解除の描画に失敗");
    let px = surface.read_back().expect("read_back 失敗");
    let residue = fill_pixels(&px, w, h);
    if !residue.is_empty() {
        red.push(format!(
            "{tag} 解除の後に塗りが {} 画素残った（先頭 {:?}）",
            residue.len(),
            residue[0]
        ));
    }
    if opaque_count(&px) == 0 {
        red.push(format!(
            "{tag} 解除の後に字のインクが無い（空面で緑にしない）"
        ));
    }
    red
}

/// S（3.1・3.2・3.3）: 右から左・左から右 × 拡大率 1・1.5 で、スクロールした可視窓の中の列へ
/// 乗せ替え、解除する。塗りは今乗せている列の帯の中にだけ在り、解除の後は塗りが 0 画素。
#[test]
fn vertical_band_follows_scroll_and_leaves_no_residue_readback() {
    let mut rig = Rig::new();
    let mut red = Vec::new();
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        for k in [1.0f32, 1.5] {
            red.extend(scroll_case(&mut rig, mode, k));
        }
    }
    assert!(
        red.is_empty(),
        "縦書きの帯のスクロールと消し残しの読み戻しで {} 件が赤:\n{}",
        red.len(),
        red.join("\n")
    );
}
