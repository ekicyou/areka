//! com/dcomp.rs — DirectComposition ラッパーのテスト
//!
//! デバイス生成は既存テストの前例（tests/graphics, tests/visual が
//! D3D11/D2D/DComp デバイスをヘッドレスで生成し合格）に従う。
//! HWND を要する API（create_target_for_hwnd / set_root）は
//! ヘッドレス検証不能のため対象外（フラグメントに所見として記録）。

use windows::Win32::Foundation::{HMODULE, RECT};
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::IDXGIDevice4;
use windows::core::Interface;
use windows_numerics::Matrix4x4;
use wintf::com::d2d::d2d_create_device;
use wintf::com::d3d11::d3d11_create_device;
use wintf::com::dcomp::*;

use super::common::with_com_initialized;
use wintf::com::animation::{
    UIAnimationManagerExt, UIAnimationVariableExt, create_animation_manager,
};

/// D3D11 → DXGI → D2D → DComp デバイスチェーンを構築する
/// （src/ecs/graphics の GraphicsCore / DCompGraphicsResource と同経路）
fn make_dcomp_device() -> IDCompositionDevice3 {
    let d3d = d3d11_create_device(
        None,
        D3D_DRIVER_TYPE_HARDWARE,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        None,
        None,
    )
    .expect("d3d11_create_device");
    let dxgi: IDXGIDevice4 = d3d.cast().expect("cast to IDXGIDevice4");
    let d2d = d2d_create_device(&dxgi).expect("d2d_create_device");
    let desktop = dcomp_create_desktop_device(&d2d).expect("dcomp_create_desktop_device");
    desktop.cast().expect("cast to IDCompositionDevice3")
}

// ============================================================
// DCompositionDeviceExt
// ============================================================

#[test]
fn device_creates_visual_and_commit_succeeds() {
    let device = make_dcomp_device();
    let _visual = device.create_visual().expect("create_visual");
    device.commit().expect("commit");
}

#[test]
fn device_creates_surface() {
    let device = make_dcomp_device();
    device
        .create_surface(
            16,
            16,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )
        .expect("create_surface");
}

#[test]
fn device_returns_frame_statistics() {
    let device = make_dcomp_device();
    let stats = device.get_frame_statistics().expect("get_frame_statistics");
    // タイムスタンプ周波数は QueryPerformanceFrequency 由来で常に正
    assert!(stats.timeFrequency > 0, "timeFrequency should be positive");
}

#[test]
fn device_creates_animation_and_transform_objects() {
    let device = make_dcomp_device();
    device.create_animation().expect("create_animation");
    device
        .create_matrix_transform_3d()
        .expect("create_matrix_transform_3d");
    device
        .create_rotate_transform_3d()
        .expect("create_rotate_transform_3d");
    device
        .create_rectangle_clip()
        .expect("create_rectangle_clip");
}

#[test]
fn device_creates_transform_3d_group_from_members() {
    let device = make_dcomp_device();
    let rotate = device.create_rotate_transform_3d().expect("rotate");
    let matrix = device.create_matrix_transform_3d().expect("matrix");

    let members = [
        Some(rotate.cast::<IDCompositionTransform3D>().expect("cast")),
        Some(matrix.cast::<IDCompositionTransform3D>().expect("cast")),
    ];
    device
        .create_transform_3d_group(&members)
        .expect("create_transform_3d_group");
}

// ============================================================
// DCompositionVisualExt
// ============================================================

#[test]
fn visual_property_setters_succeed() {
    let device = make_dcomp_device();
    let visual = device.create_visual().expect("create_visual");

    visual.set_offset_x(10.5).expect("set_offset_x");
    visual.set_offset_y(-20.25).expect("set_offset_y");
    visual.set_opacity(0.0).expect("set_opacity(0.0)");
    visual.set_opacity(1.0).expect("set_opacity(1.0)");
    visual
        .set_backface_visibility(DCOMPOSITION_BACKFACE_VISIBILITY_HIDDEN)
        .expect("set_backface_visibility");
    device.commit().expect("commit");
}

#[test]
fn visual_add_remove_and_remove_all_children_succeed() {
    let device = make_dcomp_device();
    let parent = device.create_visual().expect("parent");
    let child_a = device.create_visual().expect("child_a");
    let child_b = device.create_visual().expect("child_b");

    parent
        .add_visual(&child_a, true, None)
        .expect("add child_a");
    parent
        .add_visual(&child_b, true, &child_a)
        .expect("add child_b above child_a");
    parent.remove_visual(&child_a).expect("remove child_a");
    parent.remove_all_visuals().expect("remove_all_visuals");
    device.commit().expect("commit");
}

/// 親に存在しない Visual の削除はエラーとして伝播する。
#[test]
fn removing_unparented_visual_fails() {
    let device = make_dcomp_device();
    let parent = device.create_visual().expect("parent");
    let stranger = device.create_visual().expect("stranger");
    let result = parent.remove_visual(&stranger);
    assert!(result.is_err(), "removing non-child should fail");
}

#[test]
fn visual_set_content_with_surface_and_none_succeed() {
    let device = make_dcomp_device();
    let visual = device.create_visual().expect("visual");
    let surface = device
        .create_surface(
            16,
            16,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )
        .expect("surface");

    visual.set_content(&surface).expect("set_content(surface)");
    visual.set_content(None).expect("set_content(None)");
}

#[test]
fn visual_set_effect_and_clip_roundtrip_succeed() {
    let device = make_dcomp_device();
    let visual = device.create_visual().expect("visual");

    // Transform3D は IDCompositionEffect を継承する
    let rotate = device.create_rotate_transform_3d().expect("rotate");
    visual
        .set_effect(&rotate.cast::<IDCompositionEffect>().expect("cast"))
        .expect("set_effect");

    let clip = device.create_rectangle_clip().expect("clip");
    visual
        .set_clip_object(&clip.cast::<IDCompositionClip>().expect("cast"))
        .expect("set_clip_object");
    visual.clear_clip().expect("clear_clip");
    device.commit().expect("commit");
}

// ============================================================
// DCompositionSurfaceExt
// ============================================================

#[test]
fn surface_begin_end_draw_returns_dc_and_offset() {
    let device = make_dcomp_device();
    let surface = device
        .create_surface(
            32,
            32,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )
        .expect("surface");

    let (dc, offset) = surface.begin_draw(None).expect("begin_draw");
    // 更新オフセットはサーフェイスアトラス内の非負座標
    assert!(offset.x >= 0 && offset.y >= 0, "offset: {offset:?}");
    // 返却 DC は ID2D1DeviceContext3 として有効（Clear 呼び出し可能）
    unsafe { dc.Clear(None) };
    surface.end_draw().expect("end_draw");
}

/// 部分更新矩形での begin_draw。未描画サーフェイスへの部分更新は
/// E_INVALIDARG となるため、全面描画を 1 回完了させてから検証する。
#[test]
fn surface_begin_draw_with_update_rect_succeeds_after_full_draw() {
    let device = make_dcomp_device();
    let surface = device
        .create_surface(
            32,
            32,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )
        .expect("surface");

    // 初回は全面描画（部分更新の前提条件）
    let (_dc, _offset) = surface.begin_draw(None).expect("initial full begin_draw");
    surface.end_draw().expect("initial end_draw");

    let rect = RECT {
        left: 0,
        top: 0,
        right: 8,
        bottom: 8,
    };
    let (_dc, _offset) = surface.begin_draw(Some(&rect)).expect("begin_draw(rect)");
    surface.end_draw().expect("end_draw");
}

#[test]
fn surface_suspend_and_resume_draw_succeed() {
    let device = make_dcomp_device();
    let surface = device
        .create_surface(
            16,
            16,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )
        .expect("surface");

    let (_dc, _offset) = surface.begin_draw(None).expect("begin_draw");
    surface.suspend_draw().expect("suspend_draw");
    surface.resume_draw().expect("resume_draw");
    surface.end_draw().expect("end_draw");
}

/// 描画中でないサーフェイスへの end_draw はエラーとして伝播する。
#[test]
fn end_draw_without_begin_draw_fails() {
    let device = make_dcomp_device();
    let surface = device
        .create_surface(
            16,
            16,
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_ALPHA_MODE_PREMULTIPLIED,
        )
        .expect("surface");
    assert!(surface.end_draw().is_err());
}

// ============================================================
// DCompositionRotateTransform3DExt / DCompositionMatrixTransform3DExt
// ============================================================

#[test]
fn rotate_transform_scalar_setters_succeed() {
    let device = make_dcomp_device();
    let rotate = device.create_rotate_transform_3d().expect("rotate");
    rotate.set_angle(45.0).expect("set_angle");
    rotate.set_axis_y(0.0).expect("set_axis_y");
    rotate.set_axis_z(1.0).expect("set_axis_z");
}

/// 空の IDCompositionAnimation は E_INVALIDARG で拒否されるため、
/// セグメントを 1 つ追加（AddCubic）してから設定する。
#[test]
fn rotate_transform_accepts_populated_animation_for_angle() {
    let device = make_dcomp_device();
    let rotate = device.create_rotate_transform_3d().expect("rotate");
    let animation = device.create_animation().expect("animation");
    unsafe { animation.AddCubic(0.0, 0.0, 1.0, 0.0, 0.0) }.expect("AddCubic");
    rotate
        .set_angle_animation(&animation)
        .expect("set_angle_animation");
}

/// セグメントを持たないアニメーションは OS 側で E_INVALIDARG として
/// 拒否され、ラッパーがそのまま伝播する（現行挙動の特性化）。
#[test]
fn set_angle_animation_with_empty_animation_fails() {
    let device = make_dcomp_device();
    let rotate = device.create_rotate_transform_3d().expect("rotate");
    let animation = device.create_animation().expect("animation");
    assert!(rotate.set_angle_animation(&animation).is_err());
}

#[test]
fn matrix_transform_accepts_identity_matrix() {
    let device = make_dcomp_device();
    let matrix = device.create_matrix_transform_3d().expect("matrix");
    let identity = Matrix4x4 {
        M11: 1.0,
        M22: 1.0,
        M33: 1.0,
        M44: 1.0,
        ..Default::default()
    };
    matrix.set_matrix(&identity).expect("set_matrix");
}

// ============================================================
// UIAnimationVariableExt::get_curve（IDCompositionAnimation 連携）
// ============================================================

/// Windows Animation Manager の変数カーブを DComp アニメーション
/// オブジェクトへ転写する get_curve の成功経路。
/// （animation_test.rs では DComp デバイスが無いためここで検証）
#[test]
fn animation_variable_get_curve_writes_into_dcomp_animation() {
    with_com_initialized(|| {
        let device = make_dcomp_device();
        let dcomp_animation = device.create_animation().expect("dcomp animation");

        let manager = create_animation_manager().expect("manager");
        let var = manager.create_animation_variable(0.0).expect("variable");
        var.get_curve(&dcomp_animation).expect("get_curve");
    });
}
