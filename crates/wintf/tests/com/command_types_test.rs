//! com/d2d/command_types.rs — デバイス非依存の純粋構築ロジックテスト
//!
//! COM インターフェイス参照を要求しないコマンド型（パラメータ保持のみの型）の
//! コンストラクタが、引数を無変換でフィールドへ格納することを特性化する。
//! COM 参照を要する型（`dup_com` 経路）は `d2d_ext_test.rs` で実デバイスを
//! 用いて検証する。

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;
use wintf::com::d2d::*;

// ============================================================
// 単純な状態設定コマンド: 引数 → フィールドの素通し格納
// ============================================================

#[test]
fn set_tags_stores_both_tags_verbatim() {
    let cmd = SetTags::new(u64::MAX, 0);
    assert_eq!(cmd.tag1, u64::MAX);
    assert_eq!(cmd.tag2, 0);
}

#[test]
fn set_antialias_mode_stores_mode() {
    let cmd = SetAntialiasMode::new(D2D1_ANTIALIAS_MODE_ALIASED);
    assert_eq!(cmd.0, D2D1_ANTIALIAS_MODE_ALIASED);
}

#[test]
fn set_text_antialias_mode_stores_mode() {
    let cmd = SetTextAntialiasMode::new(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
    assert_eq!(cmd.0, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
}

#[test]
fn set_unit_mode_stores_mode() {
    let cmd = SetUnitMode::new(D2D1_UNIT_MODE_PIXELS);
    assert_eq!(cmd.0, D2D1_UNIT_MODE_PIXELS);
}

/// SetPrimitiveBlend / SetPrimitiveBlend1 / SetPrimitiveBlend2 は
/// ID2D1CommandSink / Sink1 / Sink4 の各世代に対応する独立した型であり、
/// いずれも引数をそのまま保持する。
#[test]
fn set_primitive_blend_family_stores_blend_mode() {
    assert_eq!(
        SetPrimitiveBlend::new(D2D1_PRIMITIVE_BLEND_ADD).0,
        D2D1_PRIMITIVE_BLEND_ADD
    );
    assert_eq!(
        SetPrimitiveBlend1::new(D2D1_PRIMITIVE_BLEND_MIN).0,
        D2D1_PRIMITIVE_BLEND_MIN
    );
    assert_eq!(
        SetPrimitiveBlend2::new(D2D1_PRIMITIVE_BLEND_MAX).0,
        D2D1_PRIMITIVE_BLEND_MAX
    );
}

#[test]
fn set_transform_stores_matrix_components() {
    let m = Matrix3x2 {
        M11: 1.0,
        M12: 2.0,
        M21: 3.0,
        M22: 4.0,
        M31: 5.0,
        M32: 6.0,
    };
    let cmd = SetTransform::new(m);
    assert_eq!(cmd.0.M11, 1.0);
    assert_eq!(cmd.0.M12, 2.0);
    assert_eq!(cmd.0.M21, 3.0);
    assert_eq!(cmd.0.M22, 4.0);
    assert_eq!(cmd.0.M31, 5.0);
    assert_eq!(cmd.0.M32, 6.0);
}

// ============================================================
// Clear: Option<D2D1_COLOR_F>（null ポインタ = None の写し）
// ============================================================

#[test]
fn clear_none_stores_none() {
    let cmd = Clear::new(None);
    assert!(cmd.0.is_none());
}

#[test]
fn clear_some_stores_color_components() {
    let color = D2D1_COLOR_F {
        r: 0.25,
        g: 0.5,
        b: 0.75,
        a: 1.0,
    };
    let cmd = Clear::new(Some(color));
    let stored = cmd.0.expect("color should be stored");
    assert_eq!(stored.r, 0.25);
    assert_eq!(stored.g, 0.5);
    assert_eq!(stored.b, 0.75);
    assert_eq!(stored.a, 1.0);
}

// ============================================================
// PushAxisAlignedClip: 矩形 + アンチエイリアスモード
// ============================================================

#[test]
fn push_axis_aligned_clip_stores_rect_and_mode() {
    let rect = D2D_RECT_F {
        left: 1.0,
        top: 2.0,
        right: 3.0,
        bottom: 4.0,
    };
    let cmd = PushAxisAlignedClip::new(rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    assert_eq!(cmd.cliprect.left, 1.0);
    assert_eq!(cmd.cliprect.top, 2.0);
    assert_eq!(cmd.cliprect.right, 3.0);
    assert_eq!(cmd.cliprect.bottom, 4.0);
    assert_eq!(cmd.antialiasmode, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
}

// ============================================================
// SetTextRenderingParams: COM 参照は Option — None 経路は純粋
// ============================================================

#[test]
fn set_text_rendering_params_none_stores_none() {
    let cmd = SetTextRenderingParams::new(None);
    assert!(cmd.textrenderingparams.is_none());
}

// ============================================================
// DrawCommand enum: 非 COM バリアントの Clone / Debug 健全性
// ============================================================

#[test]
fn draw_command_unit_variants_are_cloneable_and_debuggable() {
    let commands = [
        DrawCommand::BeginDraw,
        DrawCommand::EndDraw,
        DrawCommand::PopAxisAlignedClip,
        DrawCommand::PopLayer,
    ];
    for cmd in &commands {
        let cloned = cmd.clone();
        // Debug 出力がバリアント名を含むことを確認（パニックしないことが主目的）
        let debug = format!("{cloned:?}");
        assert!(!debug.is_empty());
    }
}

#[test]
fn draw_command_parameter_variants_clone_preserves_payload() {
    let cmd = DrawCommand::SetTags(SetTags::new(7, 8));
    let cloned = cmd.clone();
    match cloned {
        DrawCommand::SetTags(tags) => {
            assert_eq!(tags.tag1, 7);
            assert_eq!(tags.tag2, 8);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}
