use super::ViewboxExecutor;
use super::test_support::{
    Rig, build, colored_count, glyph_items, opaque_count, require_segoe_ui_emoji,
};
use crate::canvas::{
    ChoiceLineContent, ChoiceRowSegment, ContentCanvas, HighlightPaint, Resident, ResidentContent,
};
use crate::draw::{DWriteMetrics, FontCatalog, ResolvedFont};
use crate::layout::{LayoutEngine, WrapPlan};
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextLayerConfig;
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_DRAW_TEXT_OPTIONS, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_DRAW_TEXT_OPTIONS_NONE,
};

// ════ 色つきの絵文字の読み戻し（要件 1.1・1.2・1.5・1.6・5.1・6.1・6.4・6.7・6.8） ════
//
// 既定フォント（ＭＳ ゴシック）に無い絵文字は OS の代替フォント（Segoe UI Emoji）の色つきの
// 字形で出る。各テストは先頭で代替フォントの実在を判定し、無ければ理由つきで失敗する
// （飛ばして緑にしない）。描画オプションの対照は、行の指紋にオプションが入らないので、
// 描画器と面を別々に作って取る（同じ描画器での描き直しは何も描かず空振りする）。

/// 3 方式（縦書きの向きは判定しない・色と画素の一致だけを見る）。
const MODES: [WritingMode; 3] = [
    WritingMode::HorizontalTb,
    WritingMode::VerticalRl,
    WritingMode::VerticalLr,
];

/// 字の大きさ（絵文字の色が十分な画素数で出る大きさ）と面の寸（3 字が縦横どちらにも収まる）。
const FONT_PX: u32 = 24;
const IMAGE: (u32, u32) = (96, 96);

/// 既定フォント（名前なし＝ＭＳ ゴシック）・文字色 `color`・origin 未指定（方式ごとの書字開始角）・
/// validrect 全域のモデル。
fn model(color: FontColor) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, Some(FONT_PX), color),
        None,
        None,
    )
}

/// 各テストの先頭: リグを作り、Segoe UI Emoji が引けることを判定する（無ければ理由つきで失敗）。
fn rig_with_emoji_font() -> Rig {
    let rig = Rig::new();
    let factory = rig
        .core
        .dwrite_factory()
        .expect("GraphicsCore::dwrite_factory 失敗");
    let fonts = FontCatalog::new(factory).expect("FontCatalog::new 失敗");
    require_segoe_ui_emoji(&fonts);
    rig
}

/// `text` を新しい描画器と新しい面へ 1 度だけ描いて読み戻す。`options` が `Some` なら
/// 最初の描画の前に描画オプションを差し替える（`None` は本番の既定のまま）。
fn render_fresh(
    rig: &mut Rig,
    mode: WritingMode,
    text: &str,
    color: FontColor,
    options: Option<D2D1_DRAW_TEXT_OPTIONS>,
) -> Vec<u8> {
    let model = model(color);
    let font = ResolvedFont::resolve(&model);
    let region = TextRegion::resolve(&model, IMAGE, mode);
    let contract = ScaleContract::new(1.0, None);
    let (canvas, window) = build(&glyph_items(text), &region, mode, FONT_PX as f32);
    let mut surface = rig.attach(IMAGE, 1.0);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    if let Some(options) = options {
        exec.set_text_draw_options_for_test(options);
    }
    exec.render(&canvas, &window, &font, mode, &contract, &mut surface)
        .expect("render 失敗");
    surface.read_back().expect("read_back 失敗")
}

/// 既定の黒（`FontColor` の成分がすべて欠落＝0）。
fn black() -> FontColor {
    FontColor::new(None, None, None)
}

/// 1.1・6.1: 「あ😀い」を既定の黒で描くと色つきの画素が出る。同じ台詞を別の描画器・別の面で
/// `NONE` にして描くと 0（描画オプションが効いている証拠）。両方の面に不透明画素がある
/// （空の面同士の比較にしない）。3 方式。
#[test]
fn emoji_draws_colored_pixels_and_none_option_draws_none() {
    let mut rig = rig_with_emoji_font();
    for mode in MODES {
        let colored = render_fresh(&mut rig, mode, "あ😀い", black(), None);
        let mono = render_fresh(
            &mut rig,
            mode,
            "あ😀い",
            black(),
            Some(D2D1_DRAW_TEXT_OPTIONS_NONE),
        );
        assert!(
            opaque_count(&colored) > 0 && opaque_count(&mono) > 0,
            "{mode:?}: 両方の面に字が載る（colored={} mono={}）",
            opaque_count(&colored),
            opaque_count(&mono)
        );
        assert!(
            colored_count(&colored) > 0,
            "{mode:?}: 既定の描画オプションでは 😀 が色つきの字形で出る（文字色の黒でも透明でもない画素が 1 以上）"
        );
        assert_eq!(
            colored_count(&mono),
            0,
            "{mode:?}: NONE では 😀 も黒の単色で出る（色つきの画素 0）"
        );
    }
}

/// 5.1・6.4: 絵文字を含まない「あiWa。漢！x」は、色つきの描画オプションと `NONE` とで
/// 読み戻しがバイト等価（描画器も面も別・同じ描画器での描き直しは使わない）。3 方式。
#[test]
fn non_emoji_text_is_byte_identical_with_and_without_color_font_option() {
    let mut rig = rig_with_emoji_font();
    for mode in MODES {
        let colored = render_fresh(
            &mut rig,
            mode,
            "あiWa。漢！x",
            black(),
            Some(D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT),
        );
        let mono = render_fresh(
            &mut rig,
            mode,
            "あiWa。漢！x",
            black(),
            Some(D2D1_DRAW_TEXT_OPTIONS_NONE),
        );
        assert!(
            opaque_count(&colored) > 0,
            "{mode:?}: 字が載る（空の面同士の一致にしない）"
        );
        assert!(
            colored == mono,
            "{mode:?}: 絵文字を含まない台詞の画素は描画オプションで変わらない"
        );
    }
}

/// 1.2: 既定フォントが持つ記号「♥」（U+2665）は文字色に従う。赤（R=255）で描くと、
/// premultiplied の赤は B＝G＝0 なので「α≠0 かつ（G>0 または B>0）」の画素が 0。3 方式。
#[test]
fn heart_symbol_follows_text_color() {
    let mut rig = rig_with_emoji_font();
    let red = || FontColor::new(Some(255), Some(0), Some(0));
    for mode in MODES {
        let bytes = render_fresh(&mut rig, mode, "♥", red(), None);
        let not_red = bytes
            .chunks_exact(4)
            .filter(|px| px[3] != 0 && (px[0] > 0 || px[1] > 0))
            .count();
        assert!(opaque_count(&bytes) > 0, "{mode:?}: ♥ が載る");
        assert_eq!(
            not_red, 0,
            "{mode:?}: ♥ は文字色（赤）の単色で出る（赤以外の色の画素 0）"
        );
    }
}

/// 1.3・10.2: 色つきの字形は `\f[color]` の文字色に従わない。「😀」を黒と赤で描いた読み戻しが
/// バイト等価（描画器も面も別）。色つきの画素が在ることも確かめる（単色同士の一致にしない）。3 方式。
#[test]
fn emoji_ignores_text_color() {
    let mut rig = rig_with_emoji_font();
    let red = || FontColor::new(Some(255), Some(0), Some(0));
    for mode in MODES {
        let on_black = render_fresh(&mut rig, mode, "😀", black(), None);
        let on_red = render_fresh(&mut rig, mode, "😀", red(), None);
        assert!(
            colored_count(&on_black) > 0,
            "{mode:?}: 😀 が色つきの字形で出る（色つきの画素が 1 以上）"
        );
        assert!(
            on_black == on_red,
            "{mode:?}: 色つきの字形は文字色（黒／赤）で変わらない"
        );
    }
}

/// 列 `x0..x1`（全 y）の画素を数える（`pred` が真のもの・BGRA 密配列）。
fn count_in_x_band(bytes: &[u8], w: u32, x0: u32, x1: u32, pred: impl Fn(&[u8]) -> bool) -> usize {
    bytes
        .chunks_exact(4)
        .enumerate()
        .filter(|(i, px)| {
            let x = *i as u32 % w;
            x >= x0 && x < x1 && pred(px)
        })
        .count()
}

/// 1.3・4.5・6.3: 選択肢「あ😀い」の全体をホバーして、塗り色と白の切替文字色で描いた
/// フレームと、ホバー無しのフレームを同じ描画器で描く（ホバーの序数は行の指紋に入るので
/// 描き直される）。帯は本番と同じ実測の送り幅（`DWriteMetrics`）で組んだ配置から取る。
/// 😀 は字形そのものに白（歯・目の光）を持つので「白が 0」は求めず、白の画素数が
/// ホバーの有無で変わらないこと（切替文字色で描き直されていない）を見る。横書き 1 方式。
#[test]
fn hovered_choice_keeps_emoji_own_colors() {
    let mut rig = rig_with_emoji_font();
    let mode = WritingMode::HorizontalTb;
    let model = model(black());
    let font = ResolvedFont::resolve(&model);
    let region = TextRegion::resolve(&model, IMAGE, mode);
    let contract = ScaleContract::new(1.0, None);
    let factory = rig
        .core
        .dwrite_factory()
        .expect("GraphicsCore::dwrite_factory 失敗")
        .clone();
    let metrics = DWriteMetrics::new(&factory, &font, mode, &TextLayerConfig::default())
        .expect("DWriteMetrics::new 失敗");
    let items = glyph_items("あ😀い");
    let lines = LayoutEngine::layout(
        &items,
        items.len(),
        &region,
        mode,
        FONT_PX as f32,
        &metrics,
        WrapPlan::CharByChar,
    );
    let base = ContentCanvas::from_layout(&lines, &region, mode);
    let window = LayoutEngine::visible_window(&lines, &region, mode);
    assert_eq!(base.residents.len(), 1, "「あ😀い」は 1 行");

    // 行の全体を選択肢 0 にする（範囲は住人ローカル）。
    let make_choice = |highlight: Option<HighlightPaint>, hovered: Option<usize>| {
        let residents = base
            .residents
            .iter()
            .map(|r| match &r.content {
                ResidentContent::GlyphRun(run) => Resident {
                    content: ResidentContent::Choice(ChoiceLineContent {
                        run: run.clone(),
                        segments: vec![ChoiceRowSegment {
                            ordinal: 0,
                            inline_range: (0.0, run.size.0),
                        }],
                        hovered,
                        highlight,
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
            size: base.size,
        }
    };

    // 3 字の帯（面の x・整数へ丸める）＝住人の平行移動＋配置の位置〜＋送り幅。
    let bands: Vec<(u32, u32)> = match &base.residents[0].content {
        ResidentContent::GlyphRun(run) => {
            let dx = base.residents[0].transform.offset().0;
            run.glyphs
                .iter()
                .map(|g| {
                    let x0 = dx + g.inline_pos;
                    (x0.round() as u32, (x0 + g.advance).round() as u32)
                })
                .collect()
        }
        other => panic!("住人は GlyphRun のはず: {other:?}"),
    };
    assert_eq!(bands.len(), 3, "「あ」「😀」「い」の 3 クラスタ");

    let fill = (105u8, 25u8, 25u8);
    let fill_bgra = [25u8, 25, 105, 255];
    let white_bgra = [255u8, 255, 255, 255];
    let mut surface = rig.attach(IMAGE, 1.0);
    let (w, _) = surface.size();
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    let hover_canvas = make_choice(
        Some(HighlightPaint {
            fill,
            text: (255, 255, 255),
        }),
        Some(0),
    );
    exec.render(&hover_canvas, &window, &font, mode, &contract, &mut surface)
        .expect("hover render 失敗");
    let hovered = surface.read_back().expect("read_back(hover) 失敗");

    exec.render(
        &make_choice(None, None),
        &window,
        &font,
        mode,
        &contract,
        &mut surface,
    )
    .expect("hover 無し render 失敗");
    let plain = surface.read_back().expect("read_back(hover 無し) 失敗");

    let is = |target: [u8; 4]| move |px: &[u8]| px == target;
    let white_in =
        |bytes: &[u8], (x0, x1): (u32, u32)| count_in_x_band(bytes, w, x0, x1, is(white_bgra));

    // 絵文字の帯: 白の数がホバーの有無で同じ・自分の色が出る・ハイライトの塗りが及ぶ。
    let emoji = bands[1];
    assert_eq!(
        white_in(&hovered, emoji),
        white_in(&plain, emoji),
        "😀 の帯の白の画素数はホバーの有無で同じ（切替文字色で描き直されない）"
    );
    let own_color = count_in_x_band(&hovered, w, emoji.0, emoji.1, |px| {
        px[3] != 0 && px != fill_bgra && px != white_bgra
    });
    assert!(
        own_color > 0,
        "ホバー中の 😀 の帯に塗り色でも白でもない画素がある（字形の自分の色）"
    );
    assert!(
        count_in_x_band(&hovered, w, emoji.0, emoji.1, is(fill_bgra)) > 0,
        "ホバーの塗りが 😀 の帯まで及ぶ"
    );

    // 「あ」「い」の帯: 白はホバー中だけ出る（ホバーの範囲が絵文字の前後で途切れない）。
    for (name, band) in [("あ", bands[0]), ("い", bands[2])] {
        assert!(
            white_in(&hovered, band) > 0,
            "ホバー中の「{name}」は切替文字色（白）で出る"
        );
        assert_eq!(
            white_in(&plain, band),
            0,
            "ホバー無しの「{name}」に白は無い"
        );
    }
}
