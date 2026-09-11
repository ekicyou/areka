use super::ViewboxExecutor;
use super::test_support::{Rig, build, geo_model, glyph_items, opaque_count};
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, HighlightPaint, Resident, ResidentContent,
};
use crate::draw::ResolvedFont;
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextItem;
use crate::writing::WritingMode;

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
                    // 素描画等価の検証ゆえ帯は em ボックス丈（塗りを持たない）・寄せなし。
                    band_extent: run.size.1,
                    band_offset: 0.0,
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
        .render(
            &glyph_canvas,
            &window,
            &font,
            mode,
            &contract,
            &mut surface_glyph,
        )
        .expect("GlyphRun render 失敗");
    let bytes_glyph = surface_glyph.read_back().expect("read_back 失敗");

    // Choice canvas を別の独立 executor/供給面へ初回描画。
    let mut surface_choice = rig.attach(image, 1.0);
    let mut exec_choice = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec_choice
        .render(
            &choice_canvas,
            &window,
            &font,
            mode,
            &contract,
            &mut surface_choice,
        )
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
                        // 帯は em ボックス丈（10）より**大きい** 12（font 10 の行送り
                        // 10+2＝帯の頭打ち値）——実フォント（Yu Gothic UI 比 1.33）の
                        // descent 込み帯と同じ関係を檻に持ち込む。hover 解除フレームの
                        // 「塗り画素ゼロ」判定が、ダーティ帯の帯超過分拡張
                        // （expand_overhang_for_band）まで含めて赤くなる。
                        band_extent: 12.0,
                        // 寄せなし（行ボックス丈 ＝ 帯の丈のフォントと同じ関係）——寄せの効き目は
                        // `hover_band_offset_shifts_paint_and_leaves_no_residue_on_hover_off` が見る。
                        band_offset: 0.0,
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

/// premultiplied BGRA 密配列で、列帯 `x0..x1` の行 `y` に指定 BGRA と完全一致する画素数を数える。
fn count_bgra_in_row(bytes: &[u8], w: u32, y: u32, x0: u32, x1: u32, target: [u8; 4]) -> usize {
    let mut n = 0usize;
    for x in x0..x1.min(w) {
        let o = ((y * w + x) * 4) as usize;
        if bytes[o..o + 4] == target {
            n += 1;
        }
    }
    n
}

/// 観測可能な完了状態（R13.1/13.2/13.5）: `band_offset` を持つ Choice 行を描画すると、
/// ハイライトの塗りは行矩形の block 近端から**寄せの分だけ下がって**始まり、寄せの上の行には
/// 1 画素も載らない。さらに hover 解除フレームで塗りが**消し残らない**——ダーティ帯の超過分は
/// `band_offset + band_extent − font_height` で組まれるので、寄せを数えないと帯の下端側が
/// クリップの外へ落ちて塗りが残る（`expand_overhang_for_band` の回帰檻）。
#[test]
fn hover_band_offset_shifts_paint_and_leaves_no_residue_on_hover_off() {
    const BAND_EXTENT: f32 = 12.0;
    const BAND_OFFSET: f32 = 3.0;
    let mut rig = Rig::new();
    let image = (40u32, 20u32);
    let mut surface = rig.attach(image, 1.0);
    let (w, h) = surface.size();
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // 1 行「あい」（全角 2・font 10）。先頭グリフのみ選択肢セグメント（ordinal 0）にして
    // 帯の丈 12・寄せ 3 を焼く——帯は y3..15（em ボックス丈 10 の外へ 5 画素出る）。
    let (base_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
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
                            inline_range: (0.0, 10.0),
                        }],
                        hovered,
                        highlight,
                        band_extent: BAND_EXTENT,
                        band_offset: BAND_OFFSET,
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
    let fill_bgra = [25u8, 25, 105, 255];

    // ── frame 1: hover 中。塗りは y3 から始まり y14 で終わる（寄せ 0 なら y0..11）。 ──
    let hover_canvas = make_choice(
        Some(HighlightPaint {
            fill: (105, 25, 25),
            text: (255, 255, 255),
        }),
        Some(0),
    );
    exec.render(&hover_canvas, &window, &font, mode, &contract, &mut surface)
        .expect("hover render 失敗");
    let hovered_px = surface.read_back().expect("read_back(hover) 失敗");

    let offset_rows = BAND_OFFSET as u32;
    for y in 0..offset_rows {
        assert_eq!(
            count_bgra_in_row(&hovered_px, w, y, 0, 10, fill_bgra),
            0,
            "寄せの上の行 y{y} には塗りが載らない（寄せを配らないとここが塗られる）"
        );
    }
    assert!(
        count_bgra_in_row(&hovered_px, w, offset_rows, 0, 10, fill_bgra) > 0,
        "帯の 1 行目は寄せの位置 y{offset_rows} にある"
    );
    let last_band_row = offset_rows + BAND_EXTENT as u32 - 1;
    assert!(
        last_band_row < h,
        "帯の最終行 y{last_band_row} は供給面（高さ {h}）の内にある"
    );
    assert!(
        count_bgra_in_row(&hovered_px, w, last_band_row, 0, 10, fill_bgra) > 0,
        "帯の最終行は寄せ＋丈−1（y{last_band_row}）——em ボックス丈 10 の外まで塗られる"
    );
    assert_eq!(
        count_bgra_in_row(&hovered_px, w, last_band_row + 1, 0, 10, fill_bgra),
        0,
        "帯の下は塗られない（丈は寄せても変わらない）"
    );

    // ── frame 2: hover 解除（同一 executor＝キャッシュ再利用）。塗りは 1 画素も残らない。 ──
    let plain_canvas = make_choice(None, None);
    exec.render(&plain_canvas, &window, &font, mode, &contract, &mut surface)
        .expect("hover 解除 render 失敗");
    let plain_px = surface.read_back().expect("read_back(hover 解除) 失敗");
    let residue = count_bgra_in_x_band(&plain_px, w, h, 0, w, fill_bgra);
    assert_eq!(
        residue, 0,
        "hover 解除フレームに塗りが {residue} 画素消し残った——ダーティ帯の超過分が \
         band_offset を数えていない（expand_overhang_for_band・R13.5）"
    );
    assert!(
        opaque_count(&plain_px) > 0,
        "hover 解除でも素のグリフインクは描かれる（空面一致で緑にしない）"
    );
}

/// premultiplied BGRA 密配列の全域から、指定 BGRA を持つ画素の x 範囲（左端, 右端）を返す。
fn bgra_x_span(bytes: &[u8], w: u32, h: u32, target: [u8; 4]) -> Option<(u32, u32)> {
    let mut span: Option<(u32, u32)> = None;
    for x in 0..w {
        let hit = (0..h).any(|y| {
            let o = ((y * w + x) * 4) as usize;
            bytes[o..o + 4] == target
        });
        if hit {
            span = Some(match span {
                None => (x, x),
                Some((l, _)) => (l, x),
            });
        }
    }
    span
}

/// 縦書き（R13.7）: 同じ寄せをブロック軸（x）へ適用する。寄せ 0 と寄せ 3 を独立の
/// executor／供給面へ描き、読み戻した塗りの x 範囲が**寄せのぶんだけ右へ平行移動する**ことを
/// 見る（幅は不変＝帯を広げていない）。`highlight_rect` の縦書きアームが寄せを無視すると
/// 平行移動が 0 になって赤くなる。
///
/// 列の書き出し位置そのものは書字方向の縮退規約に依るので、絶対座標ではなく**2 枚の差**を
/// 測る——寄せを配ったかどうかだけを見る形にして、列原点の規約変更に引きずられないようにする。
#[test]
fn vertical_band_offset_shifts_paint_on_block_axis() {
    const BAND_EXTENT: f32 = 12.0;
    const BAND_OFFSET: f32 = 3.0;
    let mut rig = Rig::new();
    let image = (40u32, 40u32);
    let mode = WritingMode::VerticalLr;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let (base_canvas, window) = build(&glyph_items("あい"), &region, mode, 10.0);
    let fill_bgra = [25u8, 25, 105, 255];

    let paint_with_offset = |rig: &mut Rig, band_offset: f32| -> Vec<u8> {
        let residents = base_canvas
            .residents
            .iter()
            .map(|r| match &r.content {
                ResidentContent::GlyphRun(run) => Resident {
                    content: ResidentContent::Choice(ChoiceLineContent {
                        run: run.clone(),
                        segments: vec![ChoiceRowSegment {
                            ordinal: 0,
                            inline_range: (0.0, 10.0),
                        }],
                        hovered: Some(0),
                        highlight: Some(HighlightPaint {
                            fill: (105, 25, 25),
                            text: (255, 255, 255),
                        }),
                        band_extent: BAND_EXTENT,
                        band_offset,
                    }),
                    transform: r.transform,
                    effects: r.effects,
                },
                _ => r.clone(),
            })
            .collect();
        let canvas = ContentCanvas {
            residents,
            size: base_canvas.size,
        };
        let mut surface = rig.attach(image, 1.0);
        let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
        exec.render(&canvas, &window, &font, mode, &contract, &mut surface)
            .expect("縦書き hover render 失敗");
        surface.read_back().expect("read_back 失敗")
    };

    let (w, h) = (image.0, image.1);
    let flat = paint_with_offset(&mut rig, 0.0);
    let shifted = paint_with_offset(&mut rig, BAND_OFFSET);
    let flat_span = bgra_x_span(&flat, w, h, fill_bgra).expect("寄せ 0 でも塗りは在る");
    let shifted_span = bgra_x_span(&shifted, w, h, fill_bgra).expect("寄せ 3 でも塗りは在る");
    assert_eq!(
        shifted_span.0 - flat_span.0,
        BAND_OFFSET as u32,
        "縦書きの帯はブロック軸（x）へ寄せのぶんだけずれる: 寄せ 0 で x{}..{} ／ 寄せ {BAND_OFFSET} で x{}..{}",
        flat_span.0,
        flat_span.1,
        shifted_span.0,
        shifted_span.1
    );
    assert_eq!(
        shifted_span.1 - shifted_span.0,
        flat_span.1 - flat_span.0,
        "寄せても帯の厚みは変わらない（帯を広げない）"
    );
}
