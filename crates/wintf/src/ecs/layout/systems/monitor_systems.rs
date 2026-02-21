//! Monitor階層管理システム

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info, warn};

use super::super::taffy::TaffyLayoutResource;
use super::super::{
    BoxInset, BoxPosition, BoxSize, BoxStyle, Dimension, LayoutRoot, LengthPercentageAuto, Rect,
};

use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// 仮想デスクトップの矩形を取得
///
/// # 戻り値
/// (x, y, width, height) - 仮想デスクトップの左上座標とサイズ
pub fn get_virtual_desktop_bounds() -> (i32, i32, i32, i32) {
    unsafe {
        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        (x, y, width, height)
    }
}

/// LayoutRootとMonitor階層をワールド初期化時に作成する
/// world.rsのEcsWorld::new()から直接呼び出される
pub fn initialize_layout_root(world: &mut World) {
    // 既にLayoutRootが存在する場合はスキップ
    let existing = world
        .query_filtered::<Entity, With<LayoutRoot>>()
        .iter(world)
        .next();
    if existing.is_some() {
        return;
    }

    info!("[initialize_layout_root] Creating LayoutRoot singleton");

    // 仮想デスクトップの矩形を取得
    let (vx, vy, vw, vh) = get_virtual_desktop_bounds();
    debug!(
        x = vx,
        y = vy,
        width = vw,
        height = vh,
        "[initialize_layout_root] Virtual desktop bounds"
    );

    // LayoutRootエンティティを作成（仮想デスクトップ矩形を設定）
    // Note: Arrangement/GlobalArrangementはLayoutRoot::on_addフックで自動挿入される
    let layout_root = world
        .spawn((
            LayoutRoot,
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(vw as f32)),
                    height: Some(Dimension::Px(vh as f32)),
                }),
                position: Some(BoxPosition::Absolute),
                inset: Some(BoxInset(Rect {
                    left: LengthPercentageAuto::Px(vx as f32),
                    top: LengthPercentageAuto::Px(vy as f32),
                    right: LengthPercentageAuto::Auto,
                    bottom: LengthPercentageAuto::Auto,
                })),
                ..Default::default()
            },
        ))
        .id();

    // LayoutRoot用のTaffyノード作成
    {
        let mut taffy_res = world.resource_mut::<TaffyLayoutResource>();
        if let Err(e) = taffy_res.create_node(layout_root) {
            error!(error = ?e, "[initialize_layout_root] Failed to create Taffy node for LayoutRoot");
            return;
        }
    }

    // 全モニターを列挙
    let monitors = crate::ecs::monitor::enumerate_monitors();
    debug!(
        count = monitors.len(),
        "[initialize_layout_root] Enumerated monitors"
    );

    // 各Monitorエンティティを生成
    for monitor in monitors {
        let (width, height) = monitor.physical_size();
        let (left, top) = monitor.top_left();

        debug!(
            bounds_left = monitor.bounds.left,
            bounds_top = monitor.bounds.top,
            bounds_right = monitor.bounds.right,
            bounds_bottom = monitor.bounds.bottom,
            dpi = monitor.dpi,
            is_primary = monitor.is_primary,
            "[initialize_layout_root] Creating Monitor entity"
        );

        // Note: Arrangement/GlobalArrangementはMonitor::on_addフックで自動挿入される
        let monitor_entity = world
            .spawn((
                monitor,
                ChildOf(layout_root),
                BoxStyle {
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(width)),
                        height: Some(Dimension::Px(height)),
                    }),
                    position: Some(BoxPosition::Absolute),
                    inset: Some(BoxInset(Rect {
                        left: LengthPercentageAuto::Px(left),
                        top: LengthPercentageAuto::Px(top),
                        right: LengthPercentageAuto::Auto,
                        bottom: LengthPercentageAuto::Auto,
                    })),
                    ..Default::default()
                },
            ))
            .id();

        // Monitor用のTaffyノード作成
        let mut taffy_res = world.resource_mut::<TaffyLayoutResource>();
        if let Err(e) = taffy_res.create_node(monitor_entity) {
            error!(
                error = ?e,
                "[initialize_layout_root] Failed to create Taffy node for Monitor"
            );
        }
    }
}

/// Monitorの情報が変更された際に、レイアウトコンポーネントを更新
pub fn update_monitor_layout_system(
    mut query: Query<(&crate::ecs::Monitor, &mut BoxStyle), Changed<crate::ecs::Monitor>>,
) {
    for (monitor, mut box_style) in query.iter_mut() {
        let (width, height) = monitor.physical_size();
        let (left, top) = monitor.top_left();

        debug!(
            width = width,
            height = height,
            left = left,
            top = top,
            "[update_monitor_layout_system] Updating Monitor layout"
        );

        box_style.size = Some(BoxSize {
            width: Some(Dimension::Px(width)),
            height: Some(Dimension::Px(height)),
        });
        box_style.inset = Some(BoxInset(Rect {
            left: LengthPercentageAuto::Px(left),
            top: LengthPercentageAuto::Px(top),
            right: LengthPercentageAuto::Auto,
            bottom: LengthPercentageAuto::Auto,
        }));
    }
}

/// ディスプレイ構成変更を検知し、Monitorエンティティを更新
pub fn detect_display_change_system(
    mut commands: Commands,
    mut app: ResMut<crate::ecs::App>,
    layout_root: Query<Entity, With<LayoutRoot>>,
    mut existing_monitors: Query<(Entity, &mut crate::ecs::Monitor), With<crate::ecs::Monitor>>,
    mut taffy_res: ResMut<TaffyLayoutResource>,
) {
    // ディスプレイ構成変更フラグをチェック
    if !app.display_configuration_changed() {
        return;
    }

    info!("[detect_display_change_system] Display configuration changed, updating monitors");

    // LayoutRootを取得
    let Ok(root_entity) = layout_root.single() else {
        warn!("[detect_display_change_system] LayoutRoot not found, skipping");
        app.reset_display_change();
        return;
    };

    // 新しいモニターリストを取得
    let new_monitors = crate::ecs::monitor::enumerate_monitors();
    debug!(
        count = new_monitors.len(),
        "[detect_display_change_system] Found monitors"
    );

    // 既存のMonitorエンティティをマップに変換（handle → entity）
    let mut existing_map: std::collections::HashMap<isize, (Entity, crate::ecs::Monitor)> =
        existing_monitors
            .iter()
            .map(|(e, m)| (m.handle, (e, m.clone())))
            .collect();

    // 新規・更新Monitorの処理
    for new_monitor in new_monitors {
        if let Some((entity, existing_monitor)) = existing_map.remove(&new_monitor.handle) {
            // 既存Monitorの更新
            if existing_monitor != new_monitor {
                debug!(
                    entity = ?entity,
                    "[detect_display_change_system] Updating Monitor entity"
                );
                if let Ok((_, mut monitor)) = existing_monitors.get_mut(entity) {
                    *monitor = new_monitor;
                }
            }
        } else {
            // 新規Monitorの追加
            debug!(
                handle = new_monitor.handle,
                "[detect_display_change_system] Adding new Monitor"
            );

            let (width, height) = new_monitor.physical_size();
            let (left, top) = new_monitor.top_left();

            // Note: Arrangement/GlobalArrangementはMonitor::on_addフックで自動挿入される
            let monitor_entity = commands
                .spawn((
                    new_monitor,
                    ChildOf(root_entity),
                    BoxStyle {
                        size: Some(BoxSize {
                            width: Some(Dimension::Px(width)),
                            height: Some(Dimension::Px(height)),
                        }),
                        position: Some(BoxPosition::Absolute),
                        inset: Some(BoxInset(Rect {
                            left: LengthPercentageAuto::Px(left),
                            top: LengthPercentageAuto::Px(top),
                            right: LengthPercentageAuto::Auto,
                            bottom: LengthPercentageAuto::Auto,
                        })),
                        ..Default::default()
                    },
                ))
                .id();

            if let Err(e) = taffy_res.create_node(monitor_entity) {
                error!(
                    error = ?e,
                    "[detect_display_change_system] Failed to create Taffy node for new Monitor"
                );
            }
        }
    }

    // 削除されたMonitorの処理
    for (entity, monitor) in existing_map.values() {
        debug!(
            entity = ?entity,
            handle = monitor.handle,
            "[detect_display_change_system] Removing Monitor entity"
        );
        if let Err(e) = taffy_res.remove_node(*entity) {
            error!(
                error = ?e,
                "[detect_display_change_system] Failed to remove Taffy node"
            );
        }
        commands.entity(*entity).despawn();
    }

    // フラグをリセット
    app.reset_display_change();
}
