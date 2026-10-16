//! `viewbox_draw_underline.rs` の兄弟檻（areka-P0-balloon-text-bugs 2.3・要件 3.1／3.2／3.5／3.10・
//! 4.3）。実 DirectWrite（と、ブラシを作る所だけ実 GPU の D2D）で固定する。
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | 横書きの線の上端がベースラインと一致し、太さが書体の `underlineThickness`×大きさ÷upem と一致する（Yu Gothic UI 28・ＭＳ ゴシック 12・Segoe UI 16） | 3.2, 3.10 |
//! | §2 | 集めた後に区間の下線が立てる前の状態へ戻る（区間の外と、もともと立っていた所を含む） | 3.7 の前提 |
//! | §3 | 集めるのが失敗しても戻し、`error!` を残して `Err` | log-first |
//! | §4 | 描画効果のブラシを持ち、無い・写せない効果は既定の文字色の印（`None`） | 3.1 |
//! | §5 | `render_styled` の中で集めるのに失敗したフレームは提示を見送り（面は前のまま）、次のフレームで再試行して塗る。変化の無いフレームは集めない（2.5）。失敗を仕込むのは行の生成のときの集め（可謬・plan より前）で、Phase 1 の `collect_underlines(...)?` は `apply_color_ranges` と同じ BeginDraw 前の `?` の結線を使う（Phase 1 だけの檻は持たない） | 3.1, 4.3 |

use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{D2D1_DEVICE_CONTEXT_OPTIONS_NONE, ID2D1Brush};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_METRICS, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_LINE_METRICS, DWRITE_TEXT_RANGE,
    IDWriteFactory2, IDWriteTextFormat, IDWriteTextLayout,
};
use windows::core::{BOOL, HSTRING, IUnknown, Interface};
use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::com::dwrite::{DWriteFactoryExt, dwrite_create_factory};

use super::super::test_support::{Rig, live_diff_model_font};
use super::{FAIL_RECORDING, collect_underlines};
use crate::TextLayerError;
use crate::draw::{ResolvedFont, create_text_format};
use crate::underline::{ReadingAxis, stroke_rect};
use crate::writing::WritingMode;

/// 実機の書体と大きさ（実機で見た Yu Gothic UI 28・既定の ＭＳ ゴシック 12・欧文の Segoe UI 16）。
const FONTS: &[(&str, u32, &str)] = &[
    ("Yu Gothic UI", 28, "あいうえお"),
    ("ＭＳ ゴシック", 12, "あいうえお"),
    ("Segoe UI", 16, "underline"),
];

/// 位置の比べの許し（f32 の丸めだけ）。
const EPS: f32 = 1e-3;

fn factory() -> IDWriteFactory2 {
    dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED).expect("DWrite factory が作れる")
}

/// 本番と同じ生成経路（`create_text_format`）の横書きの format と行の TextLayout。
fn layout(
    factory: &IDWriteFactory2,
    name: &str,
    height: u32,
    text: &str,
) -> (IDWriteTextFormat, IDWriteTextLayout) {
    let font = ResolvedFont::resolve(&live_diff_model_font(Some(name), Some(height)));
    let format =
        create_text_format(factory, &font, WritingMode::HorizontalTb).expect("TextFormat が作れる");
    let layout = factory
        .create_text_layout(&HSTRING::from(text), &format, 4096.0, 4096.0)
        .expect("行の TextLayout が作れる");
    (format, layout)
}

/// 書体の設計値（システムのフォント集から名前で引く・標準の太さ・幅・字形）。
fn font_metrics(factory: &IDWriteFactory2, family: &str) -> DWRITE_FONT_METRICS {
    unsafe {
        let mut collection = None;
        factory
            .GetSystemFontCollection(&mut collection, false)
            .expect("システムのフォント集");
        let collection = collection.expect("フォント集がある");
        let mut index = 0;
        let mut exists = BOOL(0);
        collection
            .FindFamilyName(&HSTRING::from(family), &mut index, &mut exists)
            .expect("FindFamilyName");
        assert!(exists.as_bool(), "{family} がこの機械に入っている");
        let font = collection
            .GetFontFamily(index)
            .expect("GetFontFamily")
            .GetFirstMatchingFont(
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
            )
            .expect("GetFirstMatchingFont");
        let mut metrics = DWRITE_FONT_METRICS::default();
        font.GetMetrics(&mut metrics);
        metrics
    }
}

/// 1 行目のベースライン（行の上端からの距離・行の上端は 0）。
fn first_baseline(layout: &IDWriteTextLayout) -> f32 {
    let mut lines = [DWRITE_LINE_METRICS::default(); 4];
    let mut count = 0;
    unsafe { layout.GetLineMetrics(Some(&mut lines), &mut count) }.expect("GetLineMetrics");
    assert_eq!(count, 1, "1 行に収まる");
    lines[0].baseline
}

/// 位置ごとの下線の有無。
fn underline_at(layout: &IDWriteTextLayout, position: u32) -> bool {
    let mut has = BOOL(0);
    unsafe { layout.GetUnderline(position, &mut has, None) }.expect("GetUnderline");
    has.as_bool()
}

fn range(start: u32, length: u32) -> DWRITE_TEXT_RANGE {
    DWRITE_TEXT_RANGE {
        startPosition: start,
        length,
    }
}

// ────────────────────────── §1 上端はベースライン・太さは書体の値（3.2・3.10）

/// 3 書体・拡大率 1.0／1.25／1.5 で、横書きの線の上端が `GetLineMetrics` のベースライン
/// （D2D の描画と同じく装置の画素へ寄せた位置＝`round(baseline × k) ÷ k`。字もこの位置に描かれる）
/// と一致し、太さが書体の `underlineThickness`×大きさ÷upem と一致する（拡大率によらない）。
#[test]
fn horizontal_stroke_top_is_the_baseline_and_thickness_is_the_fonts_value() {
    let factory = factory();
    for &(name, height, text) in FONTS {
        let (_format, layout) = layout(&factory, name, height, text);
        let length = text.encode_utf16().count() as u32;
        let baseline = first_baseline(&layout);
        let m = font_metrics(&factory, name);
        let thickness = m.underlineThickness as f32 * height as f32 / m.designUnitsPerEm as f32;
        for k in [1.0f32, 1.25, 1.5] {
            let recorded = collect_underlines(&layout, &[range(0, length)], k, (0.0, 0.0))
                .expect("集められる");
            assert_eq!(recorded.len(), 1, "{name} k={k}: 区間 1 つに線 1 本");
            let raw = recorded[0].raw;
            assert_eq!(raw.reading, ReadingAxis::LeftToRight, "{name}");

            let snapped = (baseline * k + 0.5).floor() / k;
            let rect = stroke_rect(&raw, (0.0, 0.0));
            assert!(
                (rect.top - snapped).abs() < EPS,
                "{name} {height} k={k}: 線の上端 {} が画素へ寄せたベースライン {snapped}（寄せる前 {baseline}）と一致",
                rect.top
            );
            assert!(
                (raw.thickness - thickness).abs() < EPS,
                "{name} {height} k={k}: 太さ {} が書体の値 {thickness} と一致",
                raw.thickness
            );
            assert!(raw.width > 0.0, "{name}: 幅は正");
            assert!(
                recorded[0].brush.is_none(),
                "{name}: 描画効果なし＝既定の文字色"
            );
        }
    }
}

// ────────────────────────── §2 集めた後に区間の下線が戻る

/// 区間に下線を立てて集めた後、各位置の下線は立てる前の状態（もともと立っていた位置は真・
/// ほかは偽）へ戻る。離れた 2 区間はそれぞれ 1 本ずつ集まる。
#[test]
fn collecting_restores_the_underline_state_of_each_range() {
    let factory = factory();
    let (_format, layout) = layout(&factory, "ＭＳ ゴシック", 12, "あいうえお");
    unsafe { layout.SetUnderline(true, range(1, 1)) }.expect("元からの下線");
    let before: Vec<bool> = (0..5).map(|p| underline_at(&layout, p)).collect();
    assert_eq!(before, [false, true, false, false, false]);

    let recorded = collect_underlines(&layout, &[range(0, 2), range(3, 2)], 1.0, (0.0, 0.0))
        .expect("集められる");
    assert_eq!(recorded.len(), 2, "離れた 2 区間で 2 本");

    let after: Vec<bool> = (0..5).map(|p| underline_at(&layout, p)).collect();
    assert_eq!(after, before, "集めた後は立てる前と同じ");
}

// ────────────────────────── §3 失敗しても戻す・error!＋Err

/// 記録の途中で失敗しても区間の下線は戻り、`error!` を残して `Err` を返す。
#[test]
fn a_failed_collection_still_restores_and_logs_an_error() {
    let factory = factory();
    let (_format, layout) = layout(&factory, "ＭＳ ゴシック", 12, "あいう");
    FAIL_RECORDING.set(true);
    let (result, counts) = log_capture_kit::count_levels(|| {
        collect_underlines(&layout, &[range(0, 3)], 1.0, (0.0, 0.0))
    });
    FAIL_RECORDING.set(false);
    assert!(
        matches!(result, Err(TextLayerError::Device { .. })),
        "失敗は Device の Err"
    );
    assert_eq!(counts.error, 1, "error! が 1 行残る");
    assert!(
        (0..3).all(|p| !underline_at(&layout, p)),
        "失敗しても区間の下線は戻る"
    );
}

// ────────────────────────── §4 描画効果 → ブラシ・写せなければ既定の文字色

/// ブラシの描画効果はそのブラシとして持ち、効果の無い区間と、ブラシでない効果の区間は
/// 既定の文字色の印（`None`）になる。
#[test]
fn drawing_effect_becomes_a_brush_or_the_default_color_marker() {
    let rig = Rig::new();
    let dc = rig
        .core
        .d2d_device()
        .expect("D2D デバイス")
        .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        .expect("D2D DC");
    let red = D2D1_COLOR_F {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let brush = dc.create_solid_color_brush(&red, None).expect("ブラシ");

    let factory = factory();
    let (format, layout) = layout(&factory, "ＭＳ ゴシック", 12, "あいう");
    let not_a_brush: IUnknown = format.cast().expect("IUnknown");
    unsafe {
        layout
            .SetDrawingEffect(&brush, range(0, 1))
            .expect("ブラシの効果");
        layout
            .SetDrawingEffect(&not_a_brush, range(2, 1))
            .expect("ブラシでない効果");
    }

    let recorded =
        collect_underlines(&layout, &[range(0, 3)], 1.0, (0.0, 0.0)).expect("集められる");
    assert_eq!(recorded.len(), 3, "効果の切れ目ごとに 1 本");
    let expected: ID2D1Brush = brush.cast().expect("ID2D1Brush");
    assert_eq!(
        recorded[0].brush.as_ref().map(Interface::as_raw),
        Some(expected.as_raw()),
        "ブラシの効果はそのブラシ"
    );
    assert!(recorded[1].brush.is_none(), "効果なし＝既定の文字色");
    assert!(recorded[2].brush.is_none(), "写せない効果＝既定の文字色");
}

// ────────────────────────── §5 描く段: 集めるのに失敗したフレームは見送り、次のフレームで再試行

/// 下線の行を描くフレームで集めるのに失敗すると、`error!` を残して `Err` を返し、そのフレームは
/// 提示しない（面は前のまま）。次のフレームで再試行して塗る。変化の無いフレームは集めない
/// （失敗を仕込んでも `Ok(false)`）。
///
/// 失敗が当たるのは、最初に集める所＝行の生成のとき（行の保持庫・可謬・plan より前）の集めで
/// ある（失敗した行は保持庫に入らないので、次のフレームで行の生成からやり直す）。Phase 1 で描く
/// 行ごとに呼ぶ `collect_underlines(...)?` は、`apply_color_ranges` と同じ BeginDraw 前の `?` の
/// 結線（失敗＝target 未設定のまま見送り）を使うだけなので、Phase 1 の失敗だけを仕込む檻は
/// 別に持たない。
#[test]
fn a_failed_collection_skips_the_frame_and_the_next_frame_retries() {
    use super::super::ViewboxExecutor;
    use super::super::test_support::{build, geo_model, glyph_items, opaque_count};
    use crate::look::{StyleTable, TextLook};
    use crate::region::{ScaleContract, TextRegion};

    let mut rig = Rig::new();
    let image = (60u32, 30u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(12)));
    let region = TextRegion::resolve(&geo_model(Some(12)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let default = &font.looks.default;
    let mut styles = StyleTable::default();
    let under = styles.intern(
        &TextLook {
            underline: true,
            ..default.clone()
        },
        default,
    );
    let (mut canvas, window) = build(&glyph_items("あい"), &region, mode, 12.0);
    if let crate::canvas::ResidentContent::GlyphRun(run) = &mut canvas.residents[0].content {
        for glyph in &mut run.glyphs {
            glyph.style = under;
        }
    }
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    let mut render = |surface: &mut crate::surface::TextSurface| {
        exec.render_styled(&canvas, &window, &font, mode, &contract, surface, &styles)
    };

    FAIL_RECORDING.set(true);
    let (failed, counts) = log_capture_kit::count_levels(|| render(&mut surface));
    FAIL_RECORDING.set(false);
    assert!(
        matches!(failed, Err(TextLayerError::Device { .. })),
        "集めるのに失敗したフレームは Err"
    );
    assert!(counts.error >= 1, "error! が残る");
    assert_eq!(
        opaque_count(&surface.read_back().expect("read_back")),
        0,
        "失敗したフレームは提示しない（面は前のまま）"
    );

    assert!(
        render(&mut surface).expect("次のフレームは再試行して描く"),
        "描いた"
    );
    let painted = surface.read_back().expect("read_back");
    assert!(opaque_count(&painted) > 0, "字と線が描かれた");

    FAIL_RECORDING.set(true);
    let unchanged = render(&mut surface);
    FAIL_RECORDING.set(false);
    assert!(
        matches!(unchanged, Ok(false)),
        "変化の無いフレームは集めない（失敗を仕込んでも Ok(false)）: {unchanged:?}"
    );
    assert_eq!(
        surface.read_back().expect("read_back"),
        painted,
        "面は変わらない"
    );
}
