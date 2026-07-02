// ========== ヘルパー関数 ==========

use crate::com::d2d::D2D1DeviceExt;
use crate::com::wuc::CompositorDesktopInteropExt;
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::ecs::graphics::{GraphicsCore, HasGraphicsResources, WindowGraphics};
use crate::ecs::layout::GlobalArrangement;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info};
use windows::Win32::Foundation::*;
use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
use windows::core::Interface;

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
    // NOTE(W3a-V): 幅・高さが NaN の場合この比較は false となり素通りするが、
    // 後段の `NaN.ceil() as u32 == 0`（float→int の飽和キャスト）により
    // width_px == 0 ガードで None に収束する。+inf は u32::MAX へ飽和して
    // Some を返し、下流の Surface 作成が Err（error ログ）で完結する
    // （tests/graphics/surface_optimization_test.rs の NaN/inf 特性化テストで固定）。
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
    wuc_resource: &WucGraphicsResource,
    hwnd: HWND,
) -> windows::core::Result<WindowGraphics> {
    use windows::Win32::Graphics::Direct2D::D2D1_DEVICE_CONTEXT_OPTIONS_NONE;

    if !graphics.is_valid() || !wuc_resource.is_valid() {
        return Err(windows::core::Error::from(E_FAIL));
    }

    // 1. DesktopWindowTarget作成（WucGraphicsResource の Compositor から interop 経由で取得）
    let compositor = wuc_resource
        .compositor()
        .ok_or(windows::core::Error::from(E_FAIL))?;
    let desktop_interop = compositor.cast::<ICompositorDesktopInterop>()?;
    let target = desktop_interop.create_desktop_window_target(hwnd, true)?;

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
/// WindowGraphics初期化・再初期化（DComp モードの Window に対し WUC バックエンドで作成）
///
/// CompositionMode::DComp の Window にのみ WindowGraphics を作成する。
/// WucGraphicsResource が未初期化の場合はここで遅延初期化する。
pub fn init_window_graphics(
    graphics: Res<GraphicsCore>,
    wuc_resource: Option<ResMut<WucGraphicsResource>>,
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

    // WucGraphicsResource の遅延初期化
    let wuc_valid = wuc_resource.as_ref().map_or(false, |r| r.is_valid());
    if !wuc_valid {
        if let Some(d2d) = graphics.d2d_device() {
            match WucGraphicsResource::new(d2d) {
                Ok(new_resource) => {
                    info!(
                        frame = frame_count.0,
                        "[init_window_graphics] WucGraphicsResource lazily initialized"
                    );
                    commands.insert_resource(new_resource);
                    // insert_resource はコマンドキューなのでこのフレームでは使えない
                    // 次フレームで WucGraphicsResource が利用可能になる
                    return;
                }
                Err(e) => {
                    error!(
                        frame = frame_count.0,
                        error = ?e,
                        "[init_window_graphics] WucGraphicsResource initialization failed"
                    );
                    return;
                }
            }
        } else {
            return;
        }
    }

    // NOTE(W3a-V): この時点で wuc_valid == true であり、その判定は wuc_resource が
    // Some かつ is_valid() の場合にのみ true になる（!wuc_valid 分岐は全経路 return 済み）。
    // よってこの unwrap は到達可能な全経路で発火しない。
    let wuc_res = wuc_resource.unwrap();
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
                match create_window_graphics_for_hwnd(&graphics, &wuc_res, handle.hwnd) {
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
                    match create_window_graphics_for_hwnd(&graphics, &wuc_res, handle.hwnd) {
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
