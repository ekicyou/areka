//! コンポジター初期化 ECS システム
//!
//! - `compositor_init_system`: ウィンドウエンティティへの `WindowD3D11Compositor` 自動作成・管理

use crate::ecs::graphics::compositor::WindowD3D11Compositor;
use crate::ecs::graphics::{GraphicsCore, HasGraphicsResources, format_entity_name};
use crate::ecs::window::{CompositionMode, WindowHandle, WindowPos};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error};

/// HWND を持つウィンドウエンティティに `WindowD3D11Compositor` を自動作成・管理する。
///
/// - 新規ウィンドウ検出（`Without<WindowD3D11Compositor>`）→ 作成
/// - デバイスロスト復旧（`Changed<HasGraphicsResources>` + `!is_valid()`）→ 再作成
/// - リサイズ検出（`cached_size` vs `WindowPos.size`）→ `resize()`
pub fn compositor_init_system(
    core: Res<GraphicsCore>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &crate::ecs::window::Window,
            &WindowHandle,
            &WindowPos,
            &HasGraphicsResources,
            Option<&mut WindowD3D11Compositor>,
            Option<&Name>,
        ),
        Or<(
            Without<WindowD3D11Compositor>,
            Changed<HasGraphicsResources>,
            Changed<WindowPos>,
        )>,
    >,
) {
    // GraphicsCore が無効なら早期リターン
    let Some(dc) = core.device_context() else {
        return;
    };

    for (entity, window, _handle, window_pos, _res, compositor_opt, name) in query.iter_mut() {
        // DComp モードの Window はスキップ
        if window.composition_mode() != CompositionMode::ULW {
            continue;
        }

        let entity_name = format_entity_name(entity, name);

        // WindowPos.size が None または幅/高さ 0 の場合はスキップ
        let Some(size) = window_pos.size else {
            continue;
        };
        let w = size.cx as u32;
        let h = size.cy as u32;
        if w == 0 || h == 0 {
            continue;
        }

        match compositor_opt {
            None => {
                // 新規ウィンドウ: WindowD3D11Compositor 作成
                debug!(
                    entity = %entity_name,
                    width = w,
                    height = h,
                    "[compositor_init_system] Creating WindowD3D11Compositor"
                );
                match WindowD3D11Compositor::new(dc, w, h) {
                    Ok(compositor) => {
                        debug!(
                            entity = %entity_name,
                            "[compositor_init_system] WindowD3D11Compositor created"
                        );
                        commands.entity(entity).insert(compositor);
                    }
                    Err(e) => {
                        error!(
                            entity = %entity_name,
                            error = ?e,
                            "[compositor_init_system] Error creating WindowD3D11Compositor"
                        );
                    }
                }
            }
            Some(mut compositor) => {
                if !compositor.is_valid() {
                    // デバイスロスト復旧: 再作成
                    debug!(
                        entity = %entity_name,
                        "[compositor_init_system] Re-creating WindowD3D11Compositor (device lost)"
                    );
                    let old_generation = compositor.generation();
                    match WindowD3D11Compositor::new(dc, w, h) {
                        Ok(mut new_compositor) => {
                            // 旧 generation を引き継ぎインクリメント
                            let target_gen = old_generation.wrapping_add(1);
                            while new_compositor.generation() < target_gen {
                                new_compositor.increment_generation();
                            }
                            *compositor = new_compositor;
                            debug!(
                                entity = %entity_name,
                                old_generation = old_generation,
                                new_generation = compositor.generation(),
                                "[compositor_init_system] WindowD3D11Compositor re-created"
                            );
                        }
                        Err(e) => {
                            error!(
                                entity = %entity_name,
                                error = ?e,
                                "[compositor_init_system] Re-creation failed, invalidating"
                            );
                            compositor.invalidate();
                        }
                    }
                } else if compositor.cached_size() != (w, h) {
                    // リサイズ検出: resize()
                    debug!(
                        entity = %entity_name,
                        old_size = ?compositor.cached_size(),
                        new_size = ?(w, h),
                        "[compositor_init_system] Resizing WindowD3D11Compositor"
                    );
                    if let Err(e) = compositor.resize(dc, w, h) {
                        error!(
                            entity = %entity_name,
                            error = ?e,
                            "[compositor_init_system] Resize failed, keeping old size"
                        );
                    }
                }
            }
        }
    }
}
