use super::ViewboxExecutor;
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, HighlightPaint, Resident, ResidentContent,
};
use crate::draw::ResolvedFont;
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextItem;
use crate::writing::WritingMode;
use super::test_support::{Rig, build, geo_model, glyph_items, opaque_count};

/// canvas の GlyphRun 住人を、内包 run が等価な非 hover の Choice 住人へ写す
/// （`segments` 空・`hovered=None`・`highlight=None`）。transform/effects は不変。
/// 「Choice は GlyphRun と同格の素描画」（R1.4/R9.5）を検証するための等価変換。
fn as_choice_canvas(canvas: &ContentCanvas) -> ContentCanvas {
    let residents = canvas
        .residents
        .iter()
        .map(|r| match &r.content {
            ResidentContent::GlyphRun(run) => Resident {
                content: ResidentContent::Choice(ChoiceLineContent {
                    run: run.clone(),
                    segments: Vec::new(),
                    hovered: None,
                    highlight: None,
                    // 素描画等価の検証ゆえ帯は em ボックス丈（塗りを持たない）。
                    band_extent: run.size.1,
                }),
                transform: r.transform,
                effects: r.effects,
            },
            _ => r.clone(),
        })
        .collect();
    ContentCanvas {
        residents,
        size: canvas.size,
    }
}

/// R1.4/R9.5（ピクセル同一）: 非 hover（`highlight=None`）の Choice 住人は、内包 run が
/// 等価な GlyphRun 住人と**readback バイト完全一致**で描画される。GlyphRun canvas と
/// Choice canvas をそれぞれ独立の executor/供給面へ初回フレーム描画し、read_back を byte 比較する。
/// （Choice アームが run を GlyphRun と別経路で描く・寸を取り違える等の変異はこの檻で赤くなる。）
#[test]
fn choice_resident_renders_pixel_identical_to_glyph_run() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);

    // 2 行（"あい" / "うえお"）＝複数住人。全角混在で実インクを確実に載せる。
    let mut items = glyph_items("あい");
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.extend(glyph_items("うえお"));
    let (glyph_canvas, window) = build(&items, &region, mode, 10.0);
    let choice_canvas = as_choice_canvas(&glyph_canvas);

    // 等価変換の健全性（偽 GO 防止）: Choice canvas は実際に Choice 住人を持ち、
    // 住人数・寸は GlyphRun canvas と一致する。
    assert!(
        choice_canvas
            .residents
            .iter()
            .any(|r| matches!(r.content, ResidentContent::Choice(_))),
        "変換後 canvas は Choice 住人を含む"
    );
    assert_eq!(
        choice_canvas.residents.len(),
        glyph_canvas.residents.len(),
        "住人数は等価"
    );

    // GlyphRun canvas を独立 executor/供給面へ初回描画。
    let mut surface_glyph = rig.attach(image, 1.0);
    let mut exec_glyph = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec_glyph
        .render(&glyph_canvas, &window, &font, mode, &contract, &mut surface_glyph)
        .expect("GlyphRun render 失敗");
    let bytes_glyph = surface_glyph.read_back().expect("read_back 失敗");

    // Choice canvas を別の独立 executor/供給面へ初回描画。
    let mut surface_choice = rig.attach(image, 1.0);
    let mut exec_choice = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec_choice
        .render(&choice_canvas, &window, &font, mode, &contract, &mut surface_choice)
        .expect("Choice render 失敗");
    let bytes_choice = surface_choice.read_back().expect("read_back 失敗");

    assert!(
        opaque_count(&bytes_glyph) > 0,
        "GlyphRun 描画で実インクが載る（空比較で偽 GO にしない）"
    );
    assert_eq!(
        bytes_choice, bytes_glyph,
        "非 hover の Choice 住人は等価な GlyphRun 住人とピクセル同一に描画される（R1.4/R9.5）"
    );
}

/// premultiplied BGRA 密配列で、列帯 `x0..x1`（全 y）に指定 BGRA と完全一致する画素数を数える。
fn count_bgra_in_x_band(bytes: &[u8], w: u32, h: u32, x0: u32, x1: u32, target: [u8; 4]) -> usize {
    let mut n = 0usize;
    for y in 0..h {
        for x in x0..x1.min(w) {
            let o = ((y * w + x) * 4) as usize;
            if bytes[o..o + 4] == target {
                n += 1;
            }
        }
    }
    n
}

/// 観測可能な完了状態（ハイライト描画・R4.2/4.3/4.5/4.6）: hover 中の Choice 行を描画すると
/// hover セグメント矩形内は塗り色（fill）＋切替文字色（白）の画素になり、セグメント外は素描画
/// （塗りなし）になる。さらに hover 解除フレーム（highlight=None・同一 executor＝キャッシュ
/// TextLayout 再利用）は塗り画素ゼロ・文字色も既定へ戻る（DrawingEffect リセット正準列＝
/// キャッシュ層 TextLayout を汚さない・4.5）。
#[test]
fn hover_choice_line_paints_segment_and_resets_on_hover_off() {
    let mut rig = Rig::new();
    let image = (40u32, 20u32);
    let mut surface = rig.attach(image, 1.0);
    let (w, h) = surface.size();
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // 1 行「あい」（全角 2・font 10）＝グリフ位置 0/10・送り 10。GlyphRun canvas をベースに
    // 先頭グリフのみ選択肢セグメント（ordinal 0・inline_range (0,10)＝resident-local）にして
    // hover=Some(0)・fixture 実導出色（fill=(105,25,25)・text=(255,255,255)）を焼く。
    let (base_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
    let fill = (105u8, 25u8, 25u8);
    let text_color = (255u8, 255u8, 255u8);
    let make_choice = |highlight: Option<HighlightPaint>, hovered: Option<usize>| {
        let residents = base_canvas
            .residents
            .iter()
            .map(|r| match &r.content {
                ResidentContent::GlyphRun(run) => Resident {
                    content: ResidentContent::Choice(ChoiceLineContent {
                        run: run.clone(),
                        segments: vec![ChoiceRowSegment {
                            ordinal: 0,
                            inline_range: (0.0, 10.0), // 先頭グリフ（resident-local）。
                        }],
                        hovered,
                        highlight,
                        // 帯は em ボックス丈（10）より**大きい** 13——実フォント
                        // （Yu Gothic UI 比 1.33）の descent 込み帯と同じ関係を檻に持ち込む。
                        // hover 解除フレームの「塗り画素ゼロ」判定が、ダーティ帯の帯超過分
                        // 拡張（expand_overhang_for_band）まで含めて赤くなる。
                        band_extent: 13.0,
                    }),
                    transform: r.transform,
                    effects: r.effects,
                },
                _ => r.clone(),
            })
            .collect();
        ContentCanvas {
            residents,
            size: base_canvas.size,
        }
    };

    // premultiplied BGRA（α=255 ゆえ非乗算）: 塗り＝(b,g,r,a)=(25,25,105,255)・白文字＝(255,255,255,255)。
    let fill_bgra = [25u8, 25, 105, 255];
    let white_bgra = [255u8, 255, 255, 255];

    // ── frame 1: hover 中（highlight=Some）。 ──
    let hover_canvas = make_choice(
        Some(HighlightPaint {
            fill,
            text: text_color,
        }),
        Some(0),
    );
    exec.render(&hover_canvas, &window, &font, mode, &contract, &mut surface)
        .expect("hover render 失敗");
    let hovered_px = surface.read_back().expect("read_back(hover) 失敗");

    // セグメント帯（x 0..10）: 塗り画素あり＋白文字画素あり（矩形塗り＋文字色切替の双方）。
    let seg_fill = count_bgra_in_x_band(&hovered_px, w, h, 0, 10, fill_bgra);
    let seg_white = count_bgra_in_x_band(&hovered_px, w, h, 0, 10, white_bgra);
    assert!(
        seg_fill > 0,
        "hover セグメント矩形内に塗り色（fill）画素が現れる（R4.2）"
    );
    assert!(
        seg_white > 0,
        "hover セグメント範囲の文字が切替色（白）で描かれる（R4.6）"
    );

    // セグメント外（x 10..20＝2 グリフ目）: 塗り画素ゼロ・白文字ゼロ（素描画＝既定色）。
    let out_fill = count_bgra_in_x_band(&hovered_px, w, h, 10, 20, fill_bgra);
    let out_white = count_bgra_in_x_band(&hovered_px, w, h, 10, 20, white_bgra);
    assert_eq!(
        out_fill, 0,
        "hover セグメント外は塗られない（文字幅＝クリック領域幅・行全幅でない・R4.2）"
    );
    assert_eq!(
        out_white, 0,
        "hover セグメント外の文字は切替色にならない（効果範囲限定・R4.6）"
    );

    // ── frame 2: hover 解除（highlight=None・同一 executor＝行 TextLayout はキャッシュ再利用）。 ──
    let plain_canvas = make_choice(None, None);
    exec.render(&plain_canvas, &window, &font, mode, &contract, &mut surface)
        .expect("hover 解除 render 失敗");
    let plain_px = surface.read_back().expect("read_back(hover 解除) 失敗");

    // 塗り画素は全域ゼロ（塗りが残らない）・白文字も全域ゼロ（効果が全範囲 None へリセット
    // され既定黒へ戻る＝キャッシュ層 TextLayout を汚していない・4.5）。
    let all_fill = count_bgra_in_x_band(&plain_px, w, h, 0, w, fill_bgra);
    let all_white = count_bgra_in_x_band(&plain_px, w, h, 0, w, white_bgra);
    assert_eq!(all_fill, 0, "hover 解除フレームは塗り画素ゼロ（素描画）");
    assert_eq!(
        all_white, 0,
        "hover 解除フレームは切替文字色が残らない（DrawingEffect リセット正準列・4.5）"
    );
    // 非退化: 素描画でも content の非透明インクは在る（vacuous な空面一致を排除）。
    assert!(
        opaque_count(&plain_px) > 0,
        "hover 解除でも素のグリフインクは描かれる"
    );
}
