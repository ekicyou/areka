use super::ViewboxExecutor;
use super::test_support::{
    Rig, block_axis_ink_span, build, geo_model, glyph_items, live_diff_model_font, opaque_count,
};
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, HighlightPaint, Resident, ResidentContent,
};
use crate::choice::{
    ResolvedChoiceStyle, annotate_lines, decorate_canvas, derive_hit_rows, line_bands,
    to_window_physical,
};
use crate::cursor_tag::CursorWarnGuard;
use crate::draw::{DWriteMetrics, FontCatalog, PROBE_MAX_EXTENT, ResolvedFont, create_text_format};
use crate::layout::{GlyphMetrics, LayoutEngine, WrapPlan};
use crate::look::{GlyphStyles, StyleId, StyleTable, TextLook};
use crate::region::{ScaleContract, TextRegion};
use crate::state::{ChoiceSpan, SpanKind, TextItem, TextLayerConfig};
use crate::writing::WritingMode;
use areka_sakura::contract::ActorKey;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_LINE_METRICS, DWRITE_TEXT_METRICS, IDWriteFactory2,
};
use windows::core::HSTRING;
use wintf::com::dwrite::{DWriteFactoryExt, dwrite_create_factory};

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

// ── 縦書きの帯（spec vertical-choice-hover-band）: 実フォントの読み戻し（設計の R） ──

/// R の書体（本番と同じ解決に通す）。ぱすたの `font.name`「游ゴシック Medium」は DirectWrite が
/// 名前を引けず ＭＳ ゴシックへ縮退するので除く（本 spec の外で起票・`research.md` 12 節）。
const R_FONTS: [&str; 1] = ["Yu Gothic UI"];
/// ぱすたの測りと同じ 4 つの選択肢（列ごと）。
const R_CHOICES: [&str; 4] = ["晴れてる", "雨だよ", "曇ってる", "ないしょ"];
/// 記録だけの横倒しの欧文の列（選択肢にしない・合否に入れない）。
const R_LATIN: &str = "Ag";
/// 塗りの色（premultiplied BGRA・α=255）＝ `SquareFill { fill: (105, 25, 25) }`。
const R_FILL_BGRA: [u8; 4] = [25, 25, 105, 255];

/// premultiplied BGRA 密配列の全域から、指定 BGRA を持つ画素の y 範囲（上端, 下端）を返す。
fn bgra_y_span(bytes: &[u8], w: u32, h: u32, target: [u8; 4]) -> Option<(u32, u32)> {
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
    name: &str,
    size: u32,
    mode: WritingMode,
    k: f32,
) -> Vec<String> {
    let em = size as f32;
    let tag = format!("[{name} {size} {mode:?} k={k}]");
    let mut red = Vec::new();
    let model = live_diff_model_font(Some(name), Some(size));
    let font = ResolvedFont::resolve(&model);
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
        "{tag} 行の箱 {line_box:.2}・行送り {pitch}・s {s:.2}・帯 {bands:?}",
        pitch = metrics.line_pitch(em),
        s = line_box - em,
    );
    // 記録だけ: 行の TextLayout（本番と同じ書式・幅 em）の位置と幅、行の高さとベースライン。
    let picked = FontCatalog::new(factory)
        .expect("FontCatalog 生成")
        .pick(&font);
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
            "{tag} TextLayout {word}（{}）: left {:.2}・width {:.2}・行の高さ {:.2}・baseline {:.2}",
            picked.name, tm.left, tm.width, lm[0].height, lm[0].baseline
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
    }
    red
}

/// R（6.1・6.2・6.3・1.1・1.2・1.3・1.4・3.2・4.2）: 縦書きの帯が、Yu Gothic UI（比 1.33）× 大きさ 20・28 ×
/// 右から左・左から右 × 拡大率 1・2 で、乗せた列の字のインクを全幅で覆い、厚みが em × k、
/// 隣の列のインクにかからず、行内は字の範囲で、当たりの四角が塗りと画素まで同じ。
///
/// 赤の確かめは組ごとに集めて最後に 1 度だけ落とす（向きごとにどれが赤かを 1 回の走行で残す）。
/// 測った数は標準出力へ出る（`--nocapture`。赤のときは失敗の出力にも載る）。
#[test]
fn vertical_band_covers_column_ink_readback() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let mut rig = Rig::new();
    let mut red = Vec::new();
    for name in R_FONTS {
        for size in [20u32, 28] {
            for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
                for k in [1.0f32, 2.0] {
                    red.extend(vertical_band_case(&mut rig, &factory, name, size, mode, k));
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

/// 1.2 の記録（設計の行き止まり「大きさの違う列が隣り合う」・議題 1）: `\f[height,40]` の字の列と、
/// その両隣の em 20 の列の**字のインクそのもの**が重なるかを、2 つの向き × 拡大率 1・2 で測る。
/// 帯は見ない（帯の式に依らない数）。合否に入れず、測った数を標準出力へ出す（`research.md` 13 節）。
#[test]
#[ignore = "実 GPU の記録だけ（1.2）: 大きい字の列と隣の列のインクの重なりを測る・合否なし"]
fn vertical_big_glyph_column_ink_overlap_record() {
    let factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory");
    let mut rig = Rig::new();
    let name = R_FONTS[0];
    let default = TextLook {
        name: vec![name.to_owned()],
        height: 20.0,
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
    // 小さい列・大きい列（\f[height,40]）・小さい列。
    let words: [(&str, StyleId); 3] = [
        ("雨だよ", StyleId::DEFAULT),
        ("晴れてる", big),
        ("曇ってる", StyleId::DEFAULT),
    ];
    let mut items = Vec::new();
    let mut ids = Vec::new();
    for (word, id) in words {
        if !items.is_empty() {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        let word_items = glyph_items(word);
        ids.extend(std::iter::repeat_n(id, word_items.len()));
        items.extend(word_items);
    }
    let model = live_diff_model_font(Some(name), Some(20));
    let font = ResolvedFont::resolve(&model);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
            .expect("DWriteMetrics 生成");
        let image = (200u32, 200u32);
        let region = TextRegion::resolve(&model, image, mode);
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
            20.0,
            &metrics,
            WrapPlan::CharByChar,
            styles,
            &ActorKey::from("0"),
            &mut CursorWarnGuard::default(),
        );
        assert_eq!(lines.len(), 3, "3 列に並ぶ");
        let window = LayoutEngine::visible_window(&lines, &region, mode);
        let base = ContentCanvas::from_layout(&lines, &region, mode);
        for k in [1.0f32, 2.0] {
            let tag = format!("[{name} 大きい字 {mode:?} k={k}]");
            let contract = ScaleContract::new(k, None);
            let ink: Vec<(u32, u32)> = (0..lines.len())
                .map(|i| {
                    let only = ContentCanvas {
                        residents: vec![base.residents[i].clone()],
                        size: base.size,
                    };
                    let mut surface = rig.attach(image, k);
                    let (w, h) = surface.size();
                    let mut exec =
                        ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
                    exec.render_styled(
                        &only,
                        &window,
                        &font,
                        mode,
                        &contract,
                        &mut surface,
                        &table,
                    )
                    .expect("大きい字の列の描画に失敗");
                    let px = surface.read_back().expect("read_back 失敗");
                    block_axis_ink_span(&px, w, h, mode).expect("列のインクが在る")
                })
                .collect();
            for (i, line) in lines.iter().enumerate() {
                println!(
                    "{tag} 列 {}: 行矩形 x{}..{}（物理）・インク x{}..{}",
                    words[i].0,
                    line.rect.left * k,
                    line.rect.right * k,
                    ink[i].0,
                    ink[i].1
                );
            }
            for j in [0usize, 2] {
                let (b, n) = (ink[1], ink[j]);
                // 正なら重なる画素の数、0 以下なら間の画素の数（負号つき）。
                let overlap = i64::from(b.1.min(n.1)) - i64::from(b.0.max(n.0)) + 1;
                println!("{tag} 大きい列と {} のインクの重なり {overlap}", words[j].0);
            }
        }
    }
}
