use super::ViewboxExecutor;
use super::test_support::{
    Rig, build, colored_count, glyph_items, opaque_count, require_segoe_ui_emoji,
};
use crate::draw::{FontCatalog, ResolvedFont};
use crate::region::{ScaleContract, TextRegion};
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
