//! # 行の中の印の実 GPU の読み戻し（script-small-tags 6.4・要件 5.1・5.4）
//!
//! 目印の字（U+FFFC）の 1 字分の枠に、字を描いた後の段で丸（その字の文字色・不透明度 0.64）か
//! 印の絵を塗ることを、実 GPU で描いて読み戻して見る。枠は選択肢の帯と同じ式（帯の太さは行の
//! 太さ・ずれ 0）なので、テストも同じ式で枠を求める。

use std::sync::Arc;

use areka_emo_present::balloon_marker::MarkerImage;

use super::ViewboxExecutor;
use super::test_support::{Rig, build, geo_model, glyph_items};
use crate::canvas::ResidentContent;
use crate::draw::ResolvedFont;
use crate::inline_marker::{InlineMarkerLook, MARKER_GLYPH};
use crate::look::{StyleTable, TextLook};
use crate::region::{ScaleContract, TextRegion};
use crate::writing::WritingMode;

/// 「印＋あ」の 1 行を描いて読み戻し、面の画素と幅、印の枠（物理画素・左上と右下）を返す。
fn shoot(look: InlineMarkerLook) -> (Vec<u8>, u32, (u32, u32, u32, u32)) {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let model = geo_model(Some(20));
    let mut font = ResolvedFont::resolve(&model);
    font.inline_marker = look;
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let default = font.looks.default.clone();
    let mut styles = StyleTable::default();
    // 丸の色はその字の見た目の文字色（`\f[color]` を含む）＝既定と違う色の見た目を印の字へ。
    let red = styles.intern(
        &TextLook {
            color: (200, 0, 0),
            ..default.clone()
        },
        &default,
    );
    let (mut canvas, window) = build(
        &glyph_items(&format!("{MARKER_GLYPH}あ")),
        &region,
        mode,
        20.0,
    );
    let (dx, dy) = canvas.residents[0].transform.offset();
    let ResidentContent::GlyphRun(run) = &mut canvas.residents[0].content else {
        panic!("1 行目はグリフの行");
    };
    run.glyphs[0].style = red;
    let marker = run.glyphs[0].clone();
    let cell = (
        (dx + marker.inline_pos) as u32,
        (dy + window.block_offset) as u32,
        (dx + marker.inline_pos + marker.advance) as u32,
        (dy + window.block_offset + run.size.1) as u32,
    );
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    assert!(
        exec.render_styled(
            &canvas,
            &window,
            &font,
            mode,
            &contract,
            &mut surface,
            &styles
        )
        .expect("描画"),
        "描いた"
    );
    let (w, _) = surface.size();
    (surface.read_back().expect("read_back"), w, cell)
}

fn pixel(bytes: &[u8], w: u32, (x, y): (u32, u32)) -> [u8; 4] {
    let o = ((y * w + x) * 4) as usize;
    [bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]
}

fn near(actual: [u8; 4], expected: [u8; 4]) -> bool {
    actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 2)
}

/// 絵の無いバルーン（既定の丸）: 枠の真ん中の画素は丸の色（文字色 200,0,0 に不透明度 0.64 を
/// 掛けた乗算済み BGRA＝0,0,128,163）、枠の角の画素は地の色（透明）。
#[test]
fn dot_marker_fills_the_cell_center_and_leaves_the_corner_bare() {
    let (px, w, (l, t, r, b)) = shoot(InlineMarkerLook::Dot);
    let center = pixel(&px, w, ((l + r) / 2, (t + b) / 2));
    assert!(
        near(center, [0, 0, 128, 163]),
        "枠 {:?} の真ん中は丸の色: {center:?}",
        (l, t, r, b)
    );
    let corner = pixel(&px, w, (l, t));
    assert_eq!(corner, [0, 0, 0, 0], "枠の角は地の色");
}

/// 印の絵のあるバルーン: 枠の真ん中に絵（緑 4×4）が原寸で置かれ、枠の角は地の色。
#[test]
fn image_marker_is_drawn_at_the_cell_center() {
    let green = Arc::new(MarkerImage {
        width: 4,
        height: 4,
        bgra: [0u8, 255, 0, 255].repeat(16),
    });
    let (px, w, (l, t, r, b)) = shoot(InlineMarkerLook::Image(green));
    let center = pixel(&px, w, ((l + r) / 2, (t + b) / 2));
    assert!(
        near(center, [0, 255, 0, 255]),
        "枠 {:?} の真ん中は絵の色: {center:?}",
        (l, t, r, b)
    );
    assert_eq!(pixel(&px, w, (l, t)), [0, 0, 0, 0], "枠の角は地の色");
}
