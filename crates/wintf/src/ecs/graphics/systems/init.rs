// ========== ヘルパー関数 ==========

use crate::com::d2d::D2D1DeviceExt;
use crate::com::dcomp::DCompositionDesktopDeviceExt;
use crate::ecs::graphics::{
    DCompGraphicsResource, GraphicsCore, HasGraphicsResources, VisualGraphics, WindowGraphics,
};
use crate::ecs::layout::GlobalArrangement;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info};
use windows::Win32::Foundation::*;

/// エンティティ名をログ用にフォーマットする
///
/// # Arguments
/// * `entity` - エンティティID
/// * `name` - Nameコンポーネント（オプション）
///
/// # Returns
/// Nameがあれば名前をそのまま返し、なければ`Entity(0v1)`形式でEntity IDを返す
pub fn format_entity_name(entity: Entity, name: Option<&Name>) -> String {
    match name {
        Some(n) => n.to_string(),
        None => format!("Entity({:?})", entity),
    }
}

/// GlobalArrangementの境界から物理ピクセルサイズを計算する
///
/// # Arguments
/// * `global_arrangement` - GlobalArrangementコンポーネント
///
/// # Returns
/// * `Some((width, height))` - 有効なサイズ（幅と高さが両方1以上）
/// * `None` - 無効なサイズ（幅または高さが0以下）
///
/// # Requirements
/// - Req 3.1: GlobalArrangement.boundsから計算（物理ピクセルサイズ）
/// - Req 3.2: スケール適用後のサイズ（小数点切り上げ）
/// - Req 3.3: サイズ0の場合はNone
pub fn calculate_surface_size_from_global_arrangement(
    global_arrangement: &GlobalArrangement,
) -> Option<(u32, u32)> {
    let width = global_arrangement.bounds.right - global_arrangement.bounds.left;
    let height = global_arrangement.bounds.bottom - global_arrangement.bounds.top;

    // サイズが0以下の場合はNone
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    // 小数点以下を切り上げて物理ピクセルサイズを計算
    let width_px = width.ceil() as u32;
    let height_px = height.ceil() as u32;

    // 最低1ピクセルを保証
    if width_px == 0 || height_px == 0 {
        return None;
    }

    Some((width_px, height_px))
}

/// HWNDに対してWindowGraphicsリソースを作成する
fn create_window_graphics_for_hwnd(
    graphics: &GraphicsCore,
    dcomp_resource: &DCompGraphicsResource,
    hwnd: HWND,
) -> windows::core::Result<WindowGraphics> {
    use windows::Win32::Graphics::Direct2D::D2D1_DEVICE_CONTEXT_OPTIONS_NONE;

    if !graphics.is_valid() || !dcomp_resource.is_valid() {
        return Err(windows::core::Error::from(E_FAIL));
    }

    // 1. CompositionTarget作成（DCompGraphicsResource から取得）
    let desktop = dcomp_resource
        .desktop()
        .ok_or(windows::core::Error::from(E_FAIL))?;
    let target = desktop.create_target_for_hwnd(hwnd, true)?;

    // 2. DeviceContext作成
    let d2d = graphics
        .d2d_device()
        .ok_or(windows::core::Error::from(E_FAIL))?;
    let device_context = d2d.create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

    Ok(WindowGraphics::new(target, device_context))
}

// ========== 新しい再初期化システム ==========

/// GraphicsCore初期化・再初期化・一括マーキング
///
/// GraphicsCoreが無効な場合に再作成し、HasGraphicsResourcesを持つ全エンティティに
/// Changed<HasGraphicsResources>をトリガーして再初期化を促す。
pub fn init_graphics_core(
    graphics: Option<ResMut<GraphicsCore>>,
    mut query: Query<&mut HasGraphicsResources>,
    mut commands: Commands,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    match graphics {
        Some(mut gc) => {
            if gc.is_valid() {
                return;
            }

            info!(
                frame = frame_count.0,
                "[init_graphics_core] GraphicsCore re-initialization started"
            );
            match GraphicsCore::new() {
                Ok(new_gc) => {
                    *gc = new_gc;
                    info!(
                        frame = frame_count.0,
                        "[init_graphics_core] GraphicsCore re-initialization completed"
                    );

                    let count = query.iter().count();
                    debug!(
                        frame = frame_count.0,
                        count = count,
                        "[init_graphics_core] Triggering set_changed() on entities (re-init)"
                    );
                    for mut res in query.iter_mut() {
                        res.set_changed();
                    }
                }
                Err(e) => {
                    error!(
                        frame = frame_count.0,
                        error = ?e,
                        "[init_graphics_core] GraphicsCore re-initialization failed"
                    );
                }
            }
        }
        None => {
            info!(
                frame = frame_count.0,
                "[init_graphics_core] GraphicsCore initialization started"
            );
            match GraphicsCore::new() {
                Ok(gc) => {
                    info!(
                        frame = frame_count.0,
                        "[init_graphics_core] GraphicsCore initialization completed"
                    );
                    commands.insert_resource(gc);

                    let count = query.iter().count();
                    debug!(
                        frame = frame_count.0,
                        count = count,
                        "[init_graphics_core] Triggering set_changed() on entities (init)"
                    );
                    for mut res in query.iter_mut() {
                        res.set_changed();
                    }
                }
                Err(e) => {
                    error!(
                        frame = frame_count.0,
                        error = ?e,
                        "[init_graphics_core] GraphicsCore initialization failed"
                    );
                }
            }
        }
    }
}

/// WindowGraphics初期化・再初期化
///
/// WindowGraphics初期化・再初期化（DComp モード専用）
///
/// CompositionMode::DComp の Window にのみ WindowGraphics を作成する。
/// DCompGraphicsResource が未初期化の場合はここで遅延初期化する。
pub fn init_window_graphics(
    graphics: Res<GraphicsCore>,
    dcomp_resource: Option<ResMut<DCompGraphicsResource>>,
    mut query: Query<
        (
            Entity,
            &crate::ecs::window::Window,
            &crate::ecs::window::WindowHandle,
            &HasGraphicsResources,
            Option<&mut WindowGraphics>,
            Option<&Name>,
        ),
        Or<(Without<WindowGraphics>, Changed<HasGraphicsResources>)>,
    >,
    mut commands: Commands,
    frame_count: Res<crate::ecs::world::FrameCount>,
) {
    use crate::ecs::window::CompositionMode;

    if !graphics.is_valid() {
        return;
    }

    // DComp モードの Window が存在するか確認
    let has_dcomp_windows = query
        .iter()
        .any(|(_, w, _, _, _, _)| w.composition_mode() == CompositionMode::DComp);

    if !has_dcomp_windows {
        return;
    }

    // DCompGraphicsResource の遅延初期化
    let dcomp_valid = dcomp_resource.as_ref().map_or(false, |r| r.is_valid());
    if !dcomp_valid {
        if let Some(d2d) = graphics.d2d_device() {
            match DCompGraphicsResource::new(d2d) {
                Ok(new_resource) => {
                    info!(
                        frame = frame_count.0,
                        "[init_window_graphics] DCompGraphicsResource lazily initialized"
                    );
                    commands.insert_resource(new_resource);
                    // insert_resource はコマンドキューなのでこのフレームでは使えない
                    // 次フレームで DCompGraphicsResource が利用可能になる
                    return;
                }
                Err(e) => {
                    error!(
                        frame = frame_count.0,
                        error = ?e,
                        "[init_window_graphics] DCompGraphicsResource initialization failed"
                    );
                    return;
                }
            }
        } else {
            return;
        }
    }

    let dcomp_res = dcomp_resource.unwrap();
    for (entity, window, handle, _res, window_graphics, name) in query.iter_mut() {
        // DComp モードのみ処理
        if window.composition_mode() != CompositionMode::DComp {
            continue;
        }

        let entity_name = format_entity_name(entity, name);
        match window_graphics {
            None => {
                debug!(
                    frame = frame_count.0,
                    entity = %entity_name,
                    "[init_window_graphics] WindowGraphics creating new"
                );
                match create_window_graphics_for_hwnd(&graphics, &dcomp_res, handle.hwnd) {
                    Ok(wg) => {
                        debug!(
                            frame = frame_count.0,
                            entity = %entity_name,
                            "[init_window_graphics] WindowGraphics created"
                        );
                        commands.entity(entity).insert(wg);
                    }
                    Err(e) => {
                        error!(
                            frame = frame_count.0,
                            entity = %entity_name,
                            error = ?e,
                            "[init_window_graphics] Error creating WindowGraphics"
                        );
                    }
                }
            }
            Some(mut wg) => {
                // Changed<HasGraphicsResources>でトリガーされ、!is_valid()の場合に再初期化
                if !wg.is_valid() {
                    debug!(
                        frame = frame_count.0,
                        entity = %entity_name,
                        "[init_window_graphics] WindowGraphics re-initializing"
                    );
                    let old_generation = wg.generation();
                    match create_window_graphics_for_hwnd(&graphics, &dcomp_res, handle.hwnd) {
                        Ok(new_wg) => {
                            // 古いgenerationを引き継いでインクリメント
                            let new_generation = old_generation.wrapping_add(1);
                            *wg = new_wg;
                            // generation を手動で設定（newのデフォルトは0なので）
                            while wg.generation() < new_generation {
                                wg.increment_generation();
                            }
                            debug!(
                                frame = frame_count.0,
                                entity = %entity_name,
                                old_generation = old_generation,
                                new_generation = wg.generation(),
                                "[init_window_graphics] WindowGraphics re-initialized"
                            );
                        }
                        Err(e) => {
                            error!(
                                frame = frame_count.0,
                                entity = %entity_name,
                                error = ?e,
                                "[init_window_graphics] Re-initialization error"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Visual初期化・再初期化 (Deprecated: Use Visual component)
///
/// Changed: GraphicsNeedsInitマーカーから、Changed<HasGraphicsResources> + needs_init()に移行
/// 現在はvisual_resource_management_systemが担当
pub fn init_window_visual(
    _graphics: Res<GraphicsCore>,
    _query: Query<
        (
            Entity,
            &WindowGraphics,
            &HasGraphicsResources,
            Option<&mut VisualGraphics>,
        ),
        Or<(Without<VisualGraphics>, Changed<HasGraphicsResources>)>,
    >,
    _commands: Commands,
    _frame_count: Res<crate::ecs::world::FrameCount>,
) {
    // Deprecated: Visual creation is now handled by visual_resource_management_system
}
