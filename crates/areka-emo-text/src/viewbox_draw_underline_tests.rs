//! `viewbox_draw_underline.rs` の兄弟檻（areka-P0-balloon-text-bugs 2.3・要件 3.1／3.2／3.5／3.10・
//! 4.3）。実 DirectWrite（と、ブラシを作る所だけ実 GPU の D2D）で固定する。
//!
//! | § | 内容 | 要件 |
//! |---|---|---|
//! | §1 | 横書きの線の上端がベースラインと一致し、太さが書体の `underlineThickness`×大きさ÷upem と一致する（Yu Gothic UI 28・ＭＳ ゴシック 12・Segoe UI 16） | 3.2, 3.10 |
//! | §2 | 集めた後に区間の下線が立てる前の状態へ戻る（区間の外と、もともと立っていた所を含む） | 3.7 の前提 |
//! | §3 | 集めるのが失敗しても戻し、`error!` を残して `Err` | log-first |
//! | §4 | 描画効果のブラシを持ち、無い・写せない効果は既定の文字色の印（`None`） | 3.1 |
//! | §5 | `render_styled` の中で集めるのに失敗したフレームは提示を見送り（面は前のまま）、次のフレームで再試行して塗る。変化の無いフレームは集めない（2.5）。失敗を仕込むのは行の生成のときの集め（可謬・plan より前）で、Phase 1 の `collect_underlines(...)?` は `apply_color_ranges` と同じ BeginDraw 前の `?` の結線を使う。Phase 1 で戻すのに失敗した行は保持庫から外し、次のフレームで作り直す（再監査 R2-D4） | 3.1, 4.3 |
//! | §6 | 縦書き（2 方向・3 書体・拡大率 1.0／1.25／1.5・整数でない原点）の areka の塗り方（集める → `stroke_rect` → 字の後に `FillRectangle`）が、D2D 自身の下線（`SetUnderline` を焼いた `DrawTextLayout`）と、覆う画素の範囲が同じ・各画素の各成分の差 4/255 以下で一致する（2.6） | 3.4, 4.3 |
//! | §7 | 既定の見た目が下線つきのバルーンでは、飾りの無い行にも線が出る＝全部の字に下線を積んだ行とバイトで一致し、線の無い行とは違う（指摘 6・balloon-font-descript-keys の要件 5.5） | 3.1 |
//! | §8 | 線を面の内側へ寄せた最下行の後に 1 行送ったフレームが、同じ中身を 1 フレームで全部描いた面とバイトで一致する（寄せた線が送られて字の下に残らない・指摘 15・実 GPU） | 3.11, 4.1 |

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

use super::super::test_support::{Rig, default_and_decorated_strokes, live_diff_model_font};
use super::{FAIL_RECORDING, FAIL_RESTORE, collect_underlines};
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
/// 結線（失敗＝target 未設定のまま見送り）を使う。Phase 1 で戻すのに失敗した行の扱いは
/// `a_failed_restore_forgets_the_line_and_the_next_frame_rebuilds_it` が確かめる。
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

/// 再監査 R2-D4: Phase 1 で保持庫の行に下線を立てて集めた後、戻すのに失敗したフレームは、その行を
/// 保持庫から外す（下線が立ったままの行を次のフレームで引き当てて DirectWrite の下線と areka の
/// 線を二重に描かない）。次のフレームは行を作り直す。見るのは外したことと作り直した回数——絵の
/// 比較は、DirectWrite の下線と areka の線が同じ所に重なり二重に描いても変わらないので檻にならない。
#[test]
fn a_failed_restore_forgets_the_line_and_the_next_frame_rebuilds_it() {
    use super::super::ViewboxExecutor;
    use super::super::test_support::{build, geo_model, glyph_items};
    use crate::look::{StyleTable, TextLook};
    use crate::region::{ScaleContract, TextRegion};

    let mut rig = Rig::new();
    let image = (60u32, 30u32);
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
    let render = |exec: &mut ViewboxExecutor, surface: &mut crate::surface::TextSurface| {
        exec.render_styled(&canvas, &window, &font, mode, &contract, surface, &styles)
    };
    let mut surface = rig.attach(image, 1.0);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    render(&mut exec, &mut surface).expect("1 フレーム目");
    assert!(
        exec.line_store.overhang(0).is_some(),
        "前提: 行は保持庫にある"
    );

    // 前の行を忘れさせて、保持庫の行を Phase 1 で引き当てる全域のフレームにする。
    exec.planner.forget_prev();
    FAIL_RESTORE.set(true);
    let failed = render(&mut exec, &mut surface);
    FAIL_RESTORE.set(false);
    assert!(failed.is_err(), "戻せなかったフレームは Err");
    assert!(
        exec.line_store.overhang(0).is_none(),
        "戻せなかった行は保持庫から外す"
    );

    let creations = exec.stats().line_layout_creations;
    assert!(render(&mut exec, &mut surface).expect("次のフレーム"));
    assert_eq!(
        exec.stats().line_layout_creations - creations,
        1,
        "外した行を作り直す"
    );
}

// ────────────────────────── §6 縦書きの写し方は D2D 自身の下線と一致する（実 GPU）

/// 合格の幅: 各画素の各成分の差の上限（design 件 3「確かめ」・実装の前に決めた値・広げない）。
const D2D_MATCH_TOLERANCE: u8 = 4;

/// 1 枚を実 GPU で描いて読み戻す（拡大率 `k` の変換の下・面は `image`×k の物理寸）。
/// 描き方は `render_styled` と同じ: 透明に消し、k の変換を掛けて描く。
fn shoot_d2d(
    rig: &mut Rig,
    image: (u32, u32),
    k: f32,
    draw: impl FnOnce(
        &windows::Win32::Graphics::Direct2D::ID2D1DeviceContext,
        &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
    ),
) -> Vec<u8> {
    use windows::Win32::Graphics::Direct2D::ID2D1Image;
    use windows_numerics::Matrix3x2;

    let mut surface = rig.attach(image, k);
    let dc = rig
        .core
        .d2d_device()
        .expect("D2D デバイス")
        .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        .expect("D2D DC");
    let target = crate::draw::create_d2d_target_bitmap(&dc, surface.back_tex()).expect("target");
    let black = D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let brush = dc.create_solid_color_brush(&black, None).expect("ブラシ");
    unsafe {
        dc.SetTarget(&target);
        dc.BeginDraw();
    }
    dc.clear(None);
    dc.set_transform(&Matrix3x2 {
        M11: k,
        M12: 0.0,
        M21: 0.0,
        M22: k,
        M31: 0.0,
        M32: 0.0,
    });
    draw(&dc, &brush);
    unsafe { dc.EndDraw(None, None) }.expect("EndDraw");
    unsafe { dc.SetTarget(None::<&ID2D1Image>) };
    surface.flip();
    surface.read_back().expect("read_back")
}

/// 縦書きの行の TextLayout（本番の行の保持庫と同じ寸＝行送りの軸は字の大きさ・行内の軸は果てなし）。
fn vertical_layout(
    factory: &IDWriteFactory2,
    name: &str,
    height: u32,
    text: &str,
    mode: WritingMode,
) -> IDWriteTextLayout {
    let font = ResolvedFont::resolve(&live_diff_model_font(Some(name), Some(height)));
    let format = create_text_format(factory, &font, mode).expect("TextFormat が作れる");
    factory
        .create_text_layout(
            &HSTRING::from(text),
            &format,
            height as f32,
            crate::draw::PROBE_MAX_EXTENT,
        )
        .expect("行の TextLayout が作れる")
}

/// 端でない列の縦書きの下線は、areka の塗り方（`collect_underlines` → `stroke_rect` →
/// 字の後に `FillRectangle`＝`render_styled` と同じ道）と、D2D 自身が描いた下線（区間に
/// `SetUnderline(true)` を焼いて `DrawTextLayout`）とで、覆う画素の範囲（α が 0 でない画素の
/// 集まり）が同じで、各画素の各成分の差が [`D2D_MATCH_TOLERANCE`] 以下（要件 3.4・4.3）。
/// 2 方向・3 書体・拡大率 1.0／1.25／1.5・整数でない原点で確かめる。写し方（時計回りの回し方・
/// offset の側）を取り違えると線が列の反対側か字の中に出て、覆う範囲が違って赤になる。
#[test]
fn vertical_strokes_match_d2d_own_underline_drawing() {
    use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
    use windows_numerics::Vector2;

    let factory = factory();
    let mut rig = Rig::new();
    let image = (120u32, 260u32);
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        for &(name, height, text) in FONTS {
            let length = text.encode_utf16().count() as u32;
            for k in [1.0f32, 1.25, 1.5] {
                for origin in [(40.0f32, 10.0f32), (40.3, 10.7)] {
                    let what = format!("{mode:?} {name} {height} k={k} 原点 {origin:?}");
                    let reference = shoot_d2d(&mut rig, image, k, |dc, brush| {
                        let layout = vertical_layout(&factory, name, height, text, mode);
                        unsafe { layout.SetUnderline(true, range(0, length)) }
                            .expect("SetUnderline");
                        dc.draw_text_layout(
                            Vector2 {
                                X: origin.0,
                                Y: origin.1,
                            },
                            &layout,
                            brush,
                            crate::draw::TEXT_DRAW_OPTIONS,
                        );
                    });
                    let painted = shoot_d2d(&mut rig, image, k, |dc, brush| {
                        let layout = vertical_layout(&factory, name, height, text, mode);
                        let recorded = collect_underlines(&layout, &[range(0, length)], k, origin)
                            .expect("集められる");
                        assert!(!recorded.is_empty(), "{what}: 線が集まる");
                        dc.draw_text_layout(
                            Vector2 {
                                X: origin.0,
                                Y: origin.1,
                            },
                            &layout,
                            brush,
                            crate::draw::TEXT_DRAW_OPTIONS,
                        );
                        for r in &recorded {
                            assert_eq!(r.raw.reading, ReadingAxis::TopToBottom, "{what}");
                            let s = stroke_rect(&r.raw, (0.0, 0.0));
                            let rect = D2D_RECT_F {
                                left: s.left,
                                top: s.top,
                                right: s.right,
                                bottom: s.bottom,
                            };
                            dc.fill_rectangle(&rect, brush);
                        }
                    });
                    // 較正: D2D の下線は字だけの面にインクを足している（比べる線がある）。
                    let glyphs_only = shoot_d2d(&mut rig, image, k, |dc, brush| {
                        let layout = vertical_layout(&factory, name, height, text, mode);
                        dc.draw_text_layout(
                            Vector2 {
                                X: origin.0,
                                Y: origin.1,
                            },
                            &layout,
                            brush,
                            crate::draw::TEXT_DRAW_OPTIONS,
                        );
                    });
                    assert_ne!(reference, glyphs_only, "{what}: D2D が下線を描いた（前提）");

                    let covered = |bytes: &[u8]| -> Vec<bool> {
                        bytes.as_chunks::<4>().0.iter().map(|p| p[3] != 0).collect()
                    };
                    let (cov_ref, cov_ours) = (covered(&reference), covered(&painted));
                    let mismatched = cov_ref
                        .iter()
                        .zip(&cov_ours)
                        .filter(|(a, b)| a != b)
                        .count();
                    let worst = reference
                        .iter()
                        .zip(&painted)
                        .map(|(a, b)| a.abs_diff(*b))
                        .max()
                        .unwrap_or(0);
                    assert!(
                        mismatched == 0 && worst <= D2D_MATCH_TOLERANCE,
                        "{what}: D2D 自身の下線と一致しない（覆う範囲の食い違い {mismatched} 画素・\
                         成分の差の最大 {worst}/255・合格は 0 画素かつ {D2D_MATCH_TOLERANCE}/255 以下）"
                    );
                }
            }
        }
    }
}

/// §7（指摘 6）: `font.underline,1` のバルーンでは飾りの無い字にも下線が出る。比べる相手は
/// 既定のままのバルーンで全部の字に下線を積んだ行（同じ線の塗り方）と、線の無い行。
#[test]
fn a_default_underline_is_drawn_on_plain_glyphs() {
    let [by_default, by_decoration, without] =
        default_and_decorated_strokes(|look| look.underline = true);
    assert_ne!(
        by_decoration, without,
        "前提: 積んだ下線は線の無い行と違う絵になる（非退化）"
    );
    assert!(
        by_default == by_decoration,
        "既定の下線は飾りの無い字にも、積んだ下線と同じ線で出る"
    );
}

// ────────────────────────── §8 寄せた最下行の後の送り（実 GPU）

/// §8（指摘 15）: 既定の見た目が下線つきの大きな字（Yu Gothic UI 48）で、2 行目（最下行）の線が
/// 面の下に落ちて内側へ寄せられた面に 3 行目を足して 1 行送る。送ったフレームの面は、同じ中身を
/// 新しい実行部で 1 フレームに描いた面とバイトで一致する（寄せた線が送られて残らない）。
#[test]
fn a_scroll_after_a_pulled_bottom_row_matches_one_frame_drawing() {
    use super::super::ViewboxExecutor;
    use super::super::test_support::{build, opaque_count};
    use crate::look::StyleTable;
    use crate::region::{ScaleContract, TextRegion};
    use crate::state::TextItem;

    let mut rig = Rig::new();
    // 行送り 50（48＋2）。2 行の em の箱は 0〜48・50〜98 で面（高さ 99）に収まり、2 行目の
    // ベースライン（50＋約 51）の線は面の下に落ちて寄せられる。
    let image = (200u32, 99u32);
    let mode = WritingMode::HorizontalTb;
    let model = live_diff_model_font(Some("Yu Gothic UI"), Some(48));
    let mut font = ResolvedFont::resolve(&model);
    font.looks.default.underline = true;
    let region = TextRegion::resolve(&model, image, mode);
    let contract = ScaleContract::new(1.0, None);
    let styles = StyleTable::default();
    let rows = |n: usize| {
        let mut items: Vec<TextItem> = Vec::new();
        for (i, text) in ["あい", "うえ", "おか"].iter().take(n).enumerate() {
            if i > 0 {
                items.push(TextItem::LineBreak { ratio: 1.0 });
            }
            items.extend(super::super::test_support::glyph_items(text));
        }
        build(&items, &region, mode, 48.0)
    };
    let (two, two_window) = rows(2);
    let (three, three_window) = rows(3);
    assert!(
        three_window.block_offset != two_window.block_offset,
        "前提: 3 行目で送りが起きる"
    );
    // 前提: 2 行目（原点 y＝50）の寄せる前の線は面（高さ 99）の下へはみ出す＝寄せられる。
    let (_, row2) = layout(&factory(), "Yu Gothic UI", 48, "うえ");
    let raw = collect_underlines(&row2, &[range(0, 2)], 1.0, (0.0, 50.0)).expect("集められる");
    assert!(
        raw.iter()
            .any(|r| stroke_rect(&r.raw, (0.0, 0.0)).bottom > image.1 as f32),
        "前提: 2 行目の線が面の下に落ちる"
    );

    let mut surface = rig.attach(image, 1.0);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec.render_styled(
        &two,
        &two_window,
        &font,
        mode,
        &contract,
        &mut surface,
        &styles,
    )
    .expect("2 行のフレーム");
    exec.render_styled(
        &three,
        &three_window,
        &font,
        mode,
        &contract,
        &mut surface,
        &styles,
    )
    .expect("送ったフレーム");
    let scrolled = surface.read_back().expect("read_back");

    let mut fresh_surface = rig.attach(image, 1.0);
    let mut fresh = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    fresh
        .render_styled(
            &three,
            &three_window,
            &font,
            mode,
            &contract,
            &mut fresh_surface,
            &styles,
        )
        .expect("1 フレームで全部描く");
    let one_frame = fresh_surface.read_back().expect("read_back");

    assert!(opaque_count(&one_frame) > 0, "前提: 字と線が描かれた");
    let rows_differ =
        super::super::test_support::diff_rows(&scrolled, &one_frame, image.0, image.1);
    assert!(
        rows_differ.is_empty(),
        "送ったフレームは 1 フレームで全部描いた面と一致する（食い違う行の y: {rows_differ:?}）"
    );
}
