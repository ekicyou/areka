//! W3a-T: graphics コンポーネント（components.rs）のギャップテスト
//!
//! - VisualGraphics: Default / new_with_parent / set_parent_visual / invalidate / Debug /
//!   on_remove フック（親 Visual からのデタッチ）
//! - SurfaceGraphics: set_surface / clear（実 WUC CompositionDrawingSurface + SurfaceBrush 使用）
//!
//! Note: WindowGraphics は DesktopWindowTarget（実 HWND 必須）を要するため
//! ヘッドレスでは構築不能。W3a-T 断片の R2.8 所見として記録。
//!
//! WUC 移行: DComp の create_visual/add_visual/remove_visual/create_surface は
//! それぞれ WUC の SpriteVisual・ContainerVisual.Children().Insert/Remove・
//! CompositionDrawingSurface へ対応する。

use bevy_ecs::prelude::*;
use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use windows::UI::Composition::{ContainerVisual, Visual};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::{Interface, Result};
use wintf::ecs::{GraphicsCore, SurfaceGraphics, VisualGraphics, WucGraphicsResource};

fn make_wuc() -> WucGraphicsResource {
    // WucGraphicsResource::new は DQTAT_COM_NONE を使うため COM 初期化済みスレッドを要求する。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let d2d = core.d2d_device().expect("d2d device");
    WucGraphicsResource::new(d2d).expect("WucGraphicsResource 作成失敗")
}

fn make_visual(resource: &WucGraphicsResource) -> Visual {
    let compositor = resource.compositor().expect("compositor");
    compositor
        .CreateSpriteVisual()
        .expect("CreateSpriteVisual")
        .cast()
        .expect("cast to Visual")
}

/// WUC 移行: DComp add_visual 相当。
fn add_child(parent: &Visual, child: &Visual) -> Result<()> {
    parent
        .cast::<ContainerVisual>()?
        .Children()?
        .InsertAtTop(child)?;
    Ok(())
}

/// WUC 移行: DComp remove_visual 相当。
fn remove_child(parent: &Visual, child: &Visual) -> Result<()> {
    parent.cast::<ContainerVisual>()?.Children()?.Remove(child)?;
    Ok(())
}

fn make_surface(
    resource: &WucGraphicsResource,
    w: u32,
    h: u32,
) -> (
    windows::UI::Composition::CompositionDrawingSurface,
    windows::UI::Composition::CompositionSurfaceBrush,
) {
    let compositor = resource.compositor().expect("compositor");
    let gd = resource.graphics_device().expect("graphics_device");
    let surface = gd
        .CreateDrawingSurface(
            Size {
                Width: w as f32,
                Height: h as f32,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )
        .expect("CreateDrawingSurface");
    let brush = compositor
        .CreateSurfaceBrushWithSurface(&surface)
        .expect("CreateSurfaceBrushWithSurface");
    (surface, brush)
}

// ==========================================================================
// VisualGraphics
// ==========================================================================

#[test]
fn visual_graphics_default_is_invalid_and_empty() {
    let vg = VisualGraphics::default();
    assert!(!vg.is_valid(), "default は GPU リソースなし");
    assert!(vg.visual().is_none());
    assert!(vg.parent_visual().is_none());
}

#[test]
fn visual_graphics_new_with_parent_holds_both_references() {
    let resource = make_wuc();
    let visual = make_visual(&resource);
    let parent = make_visual(&resource);

    let vg = VisualGraphics::new_with_parent(visual, Some(parent));
    assert!(vg.is_valid());
    assert!(vg.visual().is_some());
    assert!(vg.parent_visual().is_some(), "親 Visual がキャッシュされる");
}

#[test]
fn visual_graphics_set_parent_visual_updates_cache() {
    let resource = make_wuc();
    let visual = make_visual(&resource);
    let parent = make_visual(&resource);

    let mut vg = VisualGraphics::new(visual);
    assert!(vg.parent_visual().is_none(), "new では親なし");

    vg.set_parent_visual(Some(parent));
    assert!(vg.parent_visual().is_some(), "親を後付け設定できる");

    vg.set_parent_visual(None);
    assert!(vg.parent_visual().is_none(), "None で解除できる");
}

#[test]
fn visual_graphics_invalidate_clears_visual_and_parent() {
    let resource = make_wuc();
    let visual = make_visual(&resource);
    let parent = make_visual(&resource);

    let mut vg = VisualGraphics::new_with_parent(visual, Some(parent));
    vg.invalidate();

    assert!(!vg.is_valid());
    assert!(vg.visual().is_none(), "visual も解放される");
    assert!(vg.parent_visual().is_none(), "parent_visual キャッシュも解放される");
}

#[test]
fn visual_graphics_debug_reports_presence_flags() {
    let resource = make_wuc();
    let visual = make_visual(&resource);

    let vg = VisualGraphics::new(visual);
    let dbg = format!("{:?}", vg);
    assert!(dbg.contains("VisualGraphics"), "Debug 出力に型名: {dbg}");
    assert!(
        dbg.contains("inner: true") && dbg.contains("parent_visual: false"),
        "COM ポインタではなく有無フラグを出力する: {dbg}"
    );
}

#[test]
fn visual_graphics_on_remove_detaches_from_parent_visual() {
    let resource = make_wuc();
    let parent = make_visual(&resource);
    let child = make_visual(&resource);

    // 親子関係を構築（compositor 階層同期と同じ子挿入経路）
    add_child(&parent, &child).expect("add_child");

    let mut world = World::new();
    let entity = world
        .spawn(VisualGraphics::new_with_parent(
            child.clone(),
            Some(parent.clone()),
        ))
        .id();

    // despawn で on_remove フックが parent.Children().Remove(child) を実行する
    world.despawn(entity);

    // フックで既にデタッチ済みなら、再 remove は「非子 Visual の除去」として失敗する
    assert!(
        remove_child(&parent, &child).is_err(),
        "on_remove フックが親 Visual から子をデタッチ済み"
    );
}

#[test]
fn visual_graphics_on_remove_without_parent_is_safe() {
    let resource = make_wuc();
    let visual = make_visual(&resource);

    let mut world = World::new();
    let entity = world.spawn(VisualGraphics::new(visual)).id();

    // parent_visual = None でも on_remove はパニックしない
    world.despawn(entity);
}

#[test]
fn visual_graphics_on_remove_after_invalidate_is_safe() {
    let resource = make_wuc();
    let parent = make_visual(&resource);
    let child = make_visual(&resource);
    add_child(&parent, &child).expect("add_child");

    let mut world = World::new();
    let entity = world
        .spawn(VisualGraphics::new_with_parent(child, Some(parent)))
        .id();

    // デバイスロスト相当: invalidate 後の despawn（inner = None）でもパニックしない
    world
        .get_mut::<VisualGraphics>(entity)
        .unwrap()
        .invalidate();
    world.despawn(entity);
}

// ==========================================================================
// SurfaceGraphics
// ==========================================================================

#[test]
fn surface_graphics_set_surface_replaces_content_in_place() {
    let resource = make_wuc();
    let (surface1, brush1) = make_surface(&resource, 32, 32);
    let (surface2, brush2) = make_surface(&resource, 64, 48);

    let mut sg = SurfaceGraphics::default();
    assert!(!sg.is_valid());

    sg.set_surface(surface1, brush1, (32, 32));
    assert!(sg.is_valid());
    assert_eq!(sg.size, (32, 32));

    // commands.insert() を使わない直接更新（同一フレーム反映の設計）
    sg.set_surface(surface2, brush2, (64, 48));
    assert!(sg.is_valid());
    assert_eq!(sg.size, (64, 48), "サイズも同時に更新される");
}

#[test]
fn surface_graphics_clear_resets_surface_and_size() {
    let resource = make_wuc();
    let (surface, brush) = make_surface(&resource, 32, 32);

    let mut sg = SurfaceGraphics::new(surface, brush, (32, 32));
    assert!(sg.is_valid());
    assert!(sg.surface().is_some());
    assert!(sg.brush().is_some());

    sg.clear();
    assert!(!sg.is_valid(), "クリア後は無効");
    assert!(sg.surface().is_none());
    assert!(sg.brush().is_none());
    assert_eq!(sg.size, (0, 0), "サイズもリセットされる");
}
