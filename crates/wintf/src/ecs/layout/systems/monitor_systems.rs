//! Monitor階層管理システム

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info, warn};

use super::super::taffy::TaffyLayoutResource;
use super::super::{
    BoxInset, BoxPosition, BoxSize, BoxStyle, Dimension, LayoutRoot, LengthPercentageAuto, Rect,
};

use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN,
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

/// モニタ列挙 1 台分のログを出力する。
///
/// 実モニタ列挙に依存せず檻に入れられるよう、`initialize_layout_root` の列挙ループから
/// 切り出した出力点（挙動不変の抽出）。
///
/// `handle`（モニタ識別子）と `work_area`（作業領域矩形）は要件 1.1 の必須項目であり、
/// **フィールド名は areka `placement::diag` の共有語彙**（`handle`・`work_area`）に一致させる
/// ——診断手順書の grep 突合が両側のログを同じ語で引けることが契約。
fn log_enumerated_monitor(monitor: &crate::ecs::Monitor) {
    debug!(
        handle = monitor.handle,
        bounds_left = monitor.bounds.left,
        bounds_top = monitor.bounds.top,
        bounds_right = monitor.bounds.right,
        bounds_bottom = monitor.bounds.bottom,
        work_area = format_args!(
            "{},{},{},{}",
            monitor.work_area.left,
            monitor.work_area.top,
            monitor.work_area.right,
            monitor.work_area.bottom
        ),
        dpi = monitor.dpi,
        is_primary = monitor.is_primary,
        "[initialize_layout_root] Creating Monitor entity"
    );
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
    let monitors = crate::ecs::window::monitor::enumerate_monitors();
    debug!(
        count = monitors.len(),
        "[initialize_layout_root] Enumerated monitors"
    );

    // 各Monitorエンティティを生成
    for monitor in monitors {
        let (width, height) = monitor.physical_size();
        let (left, top) = monitor.top_left();

        log_enumerated_monitor(&monitor);

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
///
/// 実モニタ列挙（`enumerate_monitors`）はこの入口だけが行い、反映そのものは
/// [`apply_monitor_snapshot`] が担う（合成モニタ表を注入して檻に入れられるようにするための
/// 挙動不変の抽出・S4 是正の前提）。
pub fn detect_display_change_system(
    mut commands: Commands,
    mut app: ResMut<crate::ecs::App>,
    layout_root: Query<Entity, With<LayoutRoot>>,
    mut existing_monitors: Query<(Entity, &mut crate::ecs::Monitor), With<crate::ecs::Monitor>>,
    mut windows: Query<(Entity, &crate::ecs::WindowPos, &mut crate::ecs::DPI)>,
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
    let new_monitors = crate::ecs::window::monitor::enumerate_monitors();
    debug!(
        count = new_monitors.len(),
        "[detect_display_change_system] Found monitors"
    );

    let windows_redriven = apply_monitor_snapshot(
        &mut commands,
        root_entity,
        &mut existing_monitors,
        &mut windows,
        &mut taffy_res,
        new_monitors,
    );
    debug!(
        windows_redriven = windows_redriven,
        "[detect_display_change_system] Display configuration change applied"
    );

    // フラグをリセット
    app.reset_display_change();
}

/// モニタ列挙結果（実列挙でも合成でもよい）をモニタ表へ反映する。
///
/// 実 OS 列挙から切り離してあるのは、「識別子が不変で値だけが変わるモニタ構成」を
/// 決定論的に注入して檻に入れるため（Req 7.5）。実モニタを持ち出さずに更新分岐の
/// 到達性を実行検証できることが本抽出の目的である。
///
/// 反映後は [`redrive_window_dpi_for_updated_monitors`] が窓の `DPI` を再導出し、
/// `WM_DPICHANGED` の受理有無に依存しない追従駆動路を成立させる（Req 7.3・D14 帰結⑷）。
///
/// # 戻り値
/// `DPI` を実際に書き換えた窓の数（＝再導出を駆動した窓の数）。呼出点の観測値であり、
/// 檻はこれを判定語に用いる（0 なら駆動していない・1 なら 1 窓だけ駆動した）。
pub(crate) fn apply_monitor_snapshot(
    commands: &mut Commands,
    root_entity: Entity,
    existing_monitors: &mut Query<(Entity, &mut crate::ecs::Monitor), With<crate::ecs::Monitor>>,
    windows: &mut Query<(Entity, &crate::ecs::WindowPos, &mut crate::ecs::DPI)>,
    taffy_res: &mut TaffyLayoutResource,
    new_monitors: Vec<crate::ecs::Monitor>,
) -> usize {
    // 既存のMonitorエンティティをマップに変換（handle → entity）
    let mut existing_map: std::collections::HashMap<isize, (Entity, crate::ecs::Monitor)> =
        existing_monitors
            .iter()
            .map(|(e, m)| (m.handle, (e, m.clone())))
            .collect();

    // 値が実際に更新されたモニタ（新しい値）。窓 DPI の再導出駆動に用いる。
    let mut updated_monitors: Vec<crate::ecs::Monitor> = Vec::new();

    // 新規・更新Monitorの処理
    for new_monitor in new_monitors {
        if let Some((entity, existing_monitor)) = existing_map.remove(&new_monitor.handle) {
            // 既存Monitorの更新
            //
            // 判定は **値の変化**（`differs_in_value`）で行う。`!=`（`PartialEq`）は
            // `handle` のみを見る**同一性**の意味論であり、ここは同一 handle で引いた
            // 相手を比べている以上、恒偽になる（診断レポート §2.7 の欠陥 S4・Req 7.2）。
            if existing_monitor.differs_in_value(&new_monitor) {
                debug!(
                    entity = ?entity,
                    handle = new_monitor.handle,
                    old_bounds = format_args!(
                        "{},{},{},{}",
                        existing_monitor.bounds.left,
                        existing_monitor.bounds.top,
                        existing_monitor.bounds.right,
                        existing_monitor.bounds.bottom
                    ),
                    new_bounds = format_args!(
                        "{},{},{},{}",
                        new_monitor.bounds.left,
                        new_monitor.bounds.top,
                        new_monitor.bounds.right,
                        new_monitor.bounds.bottom
                    ),
                    old_work_area = format_args!(
                        "{},{},{},{}",
                        existing_monitor.work_area.left,
                        existing_monitor.work_area.top,
                        existing_monitor.work_area.right,
                        existing_monitor.work_area.bottom
                    ),
                    new_work_area = format_args!(
                        "{},{},{},{}",
                        new_monitor.work_area.left,
                        new_monitor.work_area.top,
                        new_monitor.work_area.right,
                        new_monitor.work_area.bottom
                    ),
                    old_dpi = existing_monitor.dpi,
                    new_dpi = new_monitor.dpi,
                    old_primary = existing_monitor.is_primary,
                    new_primary = new_monitor.is_primary,
                    "[detect_display_change_system] Updating Monitor entity"
                );
                if let Ok((_, mut monitor)) = existing_monitors.get_mut(entity) {
                    *monitor = new_monitor.clone();
                }
                updated_monitors.push(new_monitor);
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

    // モニタ表が実際に更新されたときだけ、当該モニタ上の窓の DPI を再導出する。
    redrive_window_dpi_for_updated_monitors(windows, &updated_monitors)
}

/// 窓矩形（`WindowPos` の位置・寸）の中心点を返す。位置または寸が未確定なら `None`。
///
/// `WindowPos` はクライアント座標のミラーであり、枠なし窓ではウィンドウ座標と一致する。
/// 枠のある窓では枠の分だけずれ得るが、中心がモニタ境界を跨ぐほどの差にはならない
/// （帰属判定の粒度に対して十分）。
///
/// # 「未確定」は 2 通りある
///
/// `Option::None` だけではない——wintf の**正典の未確定表現は `CW_USEDEFAULT`** である。
/// `WindowPos::default()` は `position`／`size` の両方に `CW_USEDEFAULT` を詰めており
/// （`window_pos/mod.rs`）、`Window` component の `on_window_add` フックが `DPI` と
/// 一緒にそれを自動挿入する。実座標が入るのは `WM_WINDOWPOSCHANGED` の書き戻し以降で、
/// それまでの窓は本関数の入力として実在する。
///
/// `CW_USEDEFAULT == i32::MIN` ゆえ、素通しすると `position.x + size.width / 2` が
/// **整数桁溢れ**を起こす（dev ビルドでは panic＝UI スレッド死・release では wrap して
/// 偶然どのモニタ矩形にも入らないだけ）。判定語は同 crate の既存 3 箇所
/// （`graphics/systems/window_pos.rs` の `apply_window_pos_changes`・
/// `layout/systems/window_pos_systems.rs` の `sync_window_arrangement_from_window_pos`・
/// `window_pos/mod.rs` の `to_window_rect`）と揃えてある。
pub(crate) fn window_center(pos: &crate::ecs::WindowPos) -> Option<(i32, i32)> {
    let position = pos.position?;
    let size = pos.size?;

    // CW_USEDEFAULT が含まれる場合は「未確定」（ウィンドウ作成時の初期値）。
    if position.x == CW_USEDEFAULT || size.width == CW_USEDEFAULT {
        return None;
    }

    Some((position.x + size.width / 2, position.y + size.height / 2))
}

/// 中心点を含むモニタを返す（境界矩形は左上を含み右下を含まない半開区間）。
pub(crate) fn monitor_containing(
    monitors: &[crate::ecs::Monitor],
    center: (i32, i32),
) -> Option<&crate::ecs::Monitor> {
    let (x, y) = center;
    monitors.iter().find(|m| {
        x >= m.bounds.left && x < m.bounds.right && y >= m.bounds.top && y < m.bounds.bottom
    })
}

/// 更新されたモニタ上の窓の `DPI` component を、そのモニタの新しい DPI へ揃える（Req 7.3）。
///
/// **これが `WM_DPICHANGED` に依存しない追従駆動路の本体である**（D14 帰結⑷）。
/// `DPI` が変わると `Changed<DPI>` が発火し、下流（wintf の arrangement 更新／areka の
/// DPI 相）が窓寸と位置を再導出する。`WM_DPICHANGED` が 1 件も届かない実機環境
/// （診断レポート §2.7 の実測）でも、モニタ表の更新さえ起きればここから追従が始まる。
///
/// `WM_DPICHANGED` が届く環境では同じ値が既に入っているため差分ゼロ＝書込なしで抜ける
/// （二重駆動しない）。
///
/// # 戻り値
/// `DPI` を実際に書き換えた窓の数。
fn redrive_window_dpi_for_updated_monitors(
    windows: &mut Query<(Entity, &crate::ecs::WindowPos, &mut crate::ecs::DPI)>,
    updated_monitors: &[crate::ecs::Monitor],
) -> usize {
    if updated_monitors.is_empty() {
        return 0;
    }

    let mut rewritten = 0usize;
    for (entity, window_pos, mut dpi) in windows.iter_mut() {
        let Some(center) = window_center(window_pos) else {
            debug!(
                entity = ?entity,
                "[detect_display_change_system] Window position/size undetermined, DPI redrive skipped"
            );
            continue;
        };
        let Some(monitor) = monitor_containing(updated_monitors, center) else {
            // 更新されたモニタの上に無い窓は対象外（正常系・毎回出ると煩いので trace 相当は置かない）
            continue;
        };

        let new_dpi = crate::ecs::DPI::from_dpi(monitor.dpi as u16, monitor.dpi as u16);
        if *dpi == new_dpi {
            continue;
        }

        let old_dpi = *dpi;
        *dpi = new_dpi;
        rewritten += 1;
        debug!(
            entity = ?entity,
            handle = monitor.handle,
            center = format_args!("{},{}", center.0, center.1),
            old_dpi_x = old_dpi.dpi_x,
            old_dpi_y = old_dpi.dpi_y,
            new_dpi_x = new_dpi.dpi_x,
            new_dpi_y = new_dpi.dpi_y,
            "[detect_display_change_system] Redriving window DPI from updated Monitor (no WM_DPICHANGED required)"
        );
    }
    rewritten
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Monitor;
    use crate::ecs::test_support::capture_under_filter;
    use windows::Win32::Foundation::RECT;

    /// 実モニタ列挙に依存しない合成 `Monitor`（work area はタスクバー分だけ bounds より小さい）。
    fn synthetic_monitor() -> Monitor {
        Monitor {
            handle: 0x1234_5678,
            bounds: RECT {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1200,
            },
            work_area: RECT {
                left: -1920,
                top: 0,
                right: 0,
                bottom: 1160,
            },
            dpi: 192,
            is_primary: false,
        }
    }

    /// 要件 1.1: モニタ列挙行は `handle`（識別子）と `work_area`（作業領域矩形）を含む。
    ///
    /// フィールド名は areka `placement::diag` と**共有語彙**（`handle`・`work_area`）である
    /// ことが契約——別名（`hmonitor`・`wa` 等）へ変えると診断手順書の grep 突合が壊れる。
    /// フィールドを削れば本檻は赤になる。
    #[test]
    fn enumerated_monitor_line_carries_handle_and_work_area() {
        let monitor = synthetic_monitor();
        let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
            log_enumerated_monitor(&monitor)
        });

        assert!(
            out.contains("[initialize_layout_root] Creating Monitor entity"),
            "モニタ列挙行が出ていない: {out}"
        );
        assert!(
            out.contains("handle="),
            "モニタ識別子フィールド `handle` が無い（要件 1.1）: {out}"
        );
        assert!(
            out.contains("work_area="),
            "work area フィールド `work_area` が無い（要件 1.1）: {out}"
        );
    }

    /// 要件 1.1: `work_area` は bounds と**別の実値**として復元できる（l,t,r,b 4 成分）。
    ///
    /// bounds を流用した表示なら赤になる（work area の下端 1160 が現れない）。
    #[test]
    fn enumerated_monitor_work_area_reconstructs_all_four_edges() {
        let monitor = synthetic_monitor();
        let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
            log_enumerated_monitor(&monitor)
        });

        assert!(
            out.contains("work_area=-1920,0,0,1160"),
            "work area の 4 成分（l,t,r,b）が復元できない: {out}"
        );
        assert!(
            out.contains("handle=305419896"),
            "handle の実値が復元できない: {out}"
        );
        // bounds 側は従来どおり残っている（既存フィールドの非退行）。
        assert!(
            out.contains("bounds_bottom=1200"),
            "既存の bounds フィールドが失われている: {out}"
        );
    }

    // ======================================================================
    // S4（診断レポート §2.7・Req 7.1/7.2/7.3/7.5）
    //   「識別子が不変で値だけが変わる」表示構成変更でモニタ表が更新されること
    // ======================================================================

    use crate::ecs::types::{Point, SizeI};
    use crate::ecs::{DPI, WindowPos};

    /// 檻へ注入する「新しいモニタ列挙結果」。実 OS 列挙（`enumerate_monitors`）の代役。
    #[derive(Resource, Clone)]
    struct InjectedMonitors(Vec<Monitor>);

    /// [`apply_monitor_snapshot`] の戻り値（再導出を駆動した窓の数）を檻へ持ち出す受け皿。
    #[derive(Resource, Default)]
    struct RedriveCount(usize);

    /// 注入されたモニタ表を [`apply_monitor_snapshot`] へ流す檻専用システム。
    ///
    /// 本番の `detect_display_change_system` と**同一の query 構成**であることが重要——
    /// ここだけ別配線にすると檻が本番経路を見ていないことになる。
    fn apply_injected_monitors(
        mut commands: Commands,
        layout_root: Query<Entity, With<LayoutRoot>>,
        mut existing_monitors: Query<(Entity, &mut Monitor), With<Monitor>>,
        mut windows: Query<(Entity, &WindowPos, &mut DPI)>,
        mut taffy_res: ResMut<TaffyLayoutResource>,
        injected: Res<InjectedMonitors>,
        mut redriven: ResMut<RedriveCount>,
    ) {
        let root_entity = layout_root.single().expect("檻の LayoutRoot が単一で存在する");
        redriven.0 = apply_monitor_snapshot(
            &mut commands,
            root_entity,
            &mut existing_monitors,
            &mut windows,
            &mut taffy_res,
            injected.0.clone(),
        );
    }

    /// 直前の [`run_apply`] が再導出を駆動した窓の数。
    fn redrive_count(world: &World) -> usize {
        world.resource::<RedriveCount>().0
    }

    /// 実機セッション②（診断レポート §2.7）と同型の探針: primary モニタの拡大率を
    /// 125%（dpi=120）→ 200%（dpi=192）へ変更した状態。
    ///
    /// - `handle` は不変（拡大率変更でモニタ識別子は変わらない）
    /// - `bounds` も不変（物理解像度は変わらない）
    /// - `work_area` と `dpi` だけが変わる（タスクバーが物理的に太る）
    fn probe_monitor_before() -> Monitor {
        Monitor {
            handle: 0x0000_ABCD,
            bounds: RECT {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2160,
            },
            work_area: RECT {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2100,
            },
            dpi: 120,
            is_primary: true,
        }
    }

    fn probe_monitor_after() -> Monitor {
        Monitor {
            work_area: RECT {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2064,
            },
            dpi: 192,
            ..probe_monitor_before()
        }
    }

    /// 探針が**不動点でない**ことを檻自身が検査する（[[2.2 の教訓]]・[[3.2 の教訓]]）。
    ///
    /// 「更新される」を主張する檻は、探針の前後が本当に違う値でなければ空虚になる。
    /// さらに「`PartialEq` では等価に見える」ことも同時に固定する——これが S4 の
    /// 意味論ギャップそのものであり、探針がその穴を確かに踏んでいる証拠になる。
    fn assert_probe_is_not_a_fixed_point(before: &Monitor, after: &Monitor) {
        assert_eq!(
            before.handle, after.handle,
            "探針の前提が壊れている: 識別子は不変でなければならない"
        );
        assert_ne!(
            before.work_area.bottom, after.work_area.bottom,
            "探針が不動点: work area が実際に動いていない"
        );
        assert_ne!(before.dpi, after.dpi, "探針が不動点: dpi が実際に動いていない");
        assert_eq!(
            *before, *after,
            "探針が S4 の穴を踏んでいない: PartialEq（同一性）では等価に見えなければならない"
        );
    }

    /// 檻用 World。LayoutRoot 1 個・`probe_monitor_before()` の Monitor 1 個を持つ。
    fn probe_world(injected: Vec<Monitor>) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(TaffyLayoutResource::default());
        world.insert_resource(InjectedMonitors(injected));
        world.insert_resource(RedriveCount::default());
        world.spawn(LayoutRoot);
        let monitor_entity = world.spawn(probe_monitor_before()).id();
        (world, monitor_entity)
    }

    /// 檻の実行。**シングルスレッド実行器を明示**する——既定の多スレッド実行器では
    /// システムが別スレッドで走り、`capture_under_filter`（スレッドローカルの dispatcher
    /// 差し替え）が 1 行も捕捉できずログ檻が空虚に緑になる。
    fn run_apply(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::SingleThreaded);
        schedule.add_systems(apply_injected_monitors);
        schedule.run(world);
    }

    /// 檻用の窓（枠なしゴースト窓相当）: 中心が primary モニタ上に載る位置・寸。
    fn spawn_probe_window(world: &mut World, x: i32, y: i32, dpi: u32) -> Entity {
        world
            .spawn((
                WindowPos {
                    position: Some(Point { x, y }),
                    size: Some(SizeI {
                        width: 400,
                        height: 600,
                    }),
                    ..Default::default()
                },
                DPI::from_dpi(dpi as u16, dpi as u16),
            ))
            .id()
    }

    // ---------------------------------------------------------------- 赤証跡

    /// **S4 赤証跡（Req 7.5）**: 識別子が不変で値だけが変わったモニタ構成に対して
    /// モニタ表が更新されること。
    ///
    /// 是正未投入（`existing_monitor != new_monitor` を更新判定に使う版）では
    /// `PartialEq` が `handle` しか見ないため更新分岐が恒偽になり、**赤**になる。
    ///
    /// 再現: `cargo test -p wintf -- --ignored s4_red_`
    #[test]
    #[ignore = "S4 赤証跡（是正前の失敗を保存する）。再現: cargo test -p wintf -- --ignored s4_red_"]
    fn s4_red_monitor_table_updates_when_only_values_change() {
        let before = probe_monitor_before();
        let after = probe_monitor_after();
        assert_probe_is_not_a_fixed_point(&before, &after);

        let (mut world, monitor_entity) = probe_world(vec![after.clone()]);
        run_apply(&mut world);

        let stored = world
            .get::<Monitor>(monitor_entity)
            .expect("Monitor エンティティが生存している")
            .clone();

        // 総数や handle 一致で主張しない——**更新後の実値**を見る。
        assert_eq!(
            stored.work_area.bottom, 2064,
            "work area が起動時の値のまま凍結している（S4）: {stored:?}"
        );
        assert_eq!(
            stored.dpi, 192,
            "dpi が起動時の値のまま凍結している（S4）: {stored:?}"
        );
        // 表そのものが作り直されていない（更新であって差し替えではない）。
        assert_eq!(stored.handle, before.handle, "識別子は不変であること");
        assert_eq!(
            world.query::<&Monitor>().iter(&world).count(),
            1,
            "Monitor エンティティが増減している"
        );
    }

    /// **S4 赤証跡（Req 7.3/7.5）**: モニタ表が更新されたとき、当該モニタ上の窓の
    /// `DPI` が `WM_DPICHANGED` 抜きで再導出されること。
    ///
    /// 是正未投入では上流（モニタ表の更新）が恒偽なので当然ここも駆動されず、**赤**になる。
    ///
    /// 再現: `cargo test -p wintf -- --ignored s4_red_`
    #[test]
    #[ignore = "S4 赤証跡（是正前の失敗を保存する）。再現: cargo test -p wintf -- --ignored s4_red_"]
    fn s4_red_window_dpi_redriven_without_wm_dpichanged() {
        assert_probe_is_not_a_fixed_point(&probe_monitor_before(), &probe_monitor_after());

        let (mut world, _monitor_entity) = probe_world(vec![probe_monitor_after()]);
        // 中心 (1200, 800) は primary モニタ上。旧 DPI = 120。
        let window = spawn_probe_window(&mut world, 1000, 500, 120);

        run_apply(&mut world);

        let dpi = *world.get::<DPI>(window).expect("窓の DPI が生存している");
        assert_eq!(
            dpi,
            DPI::from_dpi(192, 192),
            "モニタ表が更新されても窓 DPI が再導出されない（S4・Req 7.3）: {dpi:?}"
        );
    }

    // ------------------------------------------------------ 常時走る随伴檻

    /// Req 7.1/7.2 + Req 1.1: 更新が実際に起き、**何が変わったか**がログから読める。
    ///
    /// 更新後の実値（`work_area` 下端・`dpi`）を component とログの両方で固定する。
    /// 述語を恒偽に変異させれば component 側が、ログのフィールドを削れば出力側が赤になる。
    #[test]
    fn value_only_change_updates_monitor_and_reports_old_and_new() {
        assert_probe_is_not_a_fixed_point(&probe_monitor_before(), &probe_monitor_after());

        let (mut world, monitor_entity) = probe_world(vec![probe_monitor_after()]);
        let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
            run_apply(&mut world)
        });

        let stored = world
            .get::<Monitor>(monitor_entity)
            .expect("Monitor エンティティが生存している")
            .clone();
        assert_eq!(stored.work_area.bottom, 2064, "work area が更新されていない");
        assert_eq!(stored.dpi, 192, "dpi が更新されていない");

        assert!(
            out.contains("[detect_display_change_system] Updating Monitor entity"),
            "更新のログが出ていない: {out}"
        );
        assert!(
            out.contains("old_dpi=120") && out.contains("new_dpi=192"),
            "何が変わったか（新旧 dpi）がログから読めない: {out}"
        );
        assert!(
            out.contains("old_work_area=0,0,3840,2100") && out.contains("new_work_area=0,0,3840,2064"),
            "何が変わったか（新旧 work area）がログから読めない: {out}"
        );
    }

    /// Req 7.3: 更新されたモニタ上の窓は `WM_DPICHANGED` 抜きで DPI が揃う。
    ///
    /// 駆動を消せば赤になる（`redrive_window_dpi_for_updated_monitors` の呼出削除・
    /// `updated_monitors` への push 削除のいずれでも）。
    #[test]
    fn updated_monitor_redrives_window_dpi_and_reports_it() {
        let (mut world, _) = probe_world(vec![probe_monitor_after()]);
        let window = spawn_probe_window(&mut world, 1000, 500, 120);

        let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
            run_apply(&mut world)
        });

        assert_eq!(
            *world.get::<DPI>(window).expect("窓の DPI"),
            DPI::from_dpi(192, 192),
            "窓 DPI が再導出されていない"
        );
        assert!(
            out.contains("Redriving window DPI from updated Monitor"),
            "再導出の観測点が出ていない: {out}"
        );
        assert!(
            out.contains("old_dpi_x=120") && out.contains("new_dpi_x=192"),
            "再導出の新旧 DPI がログから読めない: {out}"
        );
        // 戻り値（駆動した窓の数）も判定語にする——ログだけに頼らない。
        assert_eq!(redrive_count(&world), 1, "駆動した窓の数が 1 でない");
    }

    /// 非空虚性の対: **値が同一なら更新しない**（無条件更新へ変異させれば赤）。
    ///
    /// これが無いと「常に更新する」実装でも上の檻が緑になってしまう。
    #[test]
    fn identical_snapshot_updates_nothing() {
        let (mut world, _) = probe_world(vec![probe_monitor_before()]);
        let window = spawn_probe_window(&mut world, 1000, 500, 120);

        let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
            run_apply(&mut world)
        });

        assert!(
            !out.contains("[detect_display_change_system] Updating Monitor entity"),
            "値が同一なのに更新が走っている: {out}"
        );
        assert!(
            !out.contains("Redriving window DPI"),
            "値が同一なのに窓 DPI の再導出が走っている: {out}"
        );
        assert_eq!(
            *world.get::<DPI>(window).expect("窓の DPI"),
            DPI::from_dpi(120, 120),
            "窓 DPI が不用意に書き換わっている"
        );
        assert_eq!(
            redrive_count(&world),
            0,
            "値が同一なのに再導出が駆動されている"
        );
    }

    /// 再導出は**更新されたモニタ上の窓だけ**が対象（他モニタの窓は触らない）。
    #[test]
    fn window_outside_updated_monitor_is_not_redriven() {
        let (mut world, _) = probe_world(vec![probe_monitor_after()]);
        // 中心 (-1680, 800) は探針モニタ（bounds 0,0,3840,2160）の外。
        let outside = spawn_probe_window(&mut world, -1880, 500, 96);
        let inside = spawn_probe_window(&mut world, 1000, 500, 120);

        run_apply(&mut world);

        assert_eq!(
            *world.get::<DPI>(outside).expect("窓の DPI"),
            DPI::from_dpi(96, 96),
            "更新モニタ外の窓が書き換わっている"
        );
        assert_eq!(
            *world.get::<DPI>(inside).expect("窓の DPI"),
            DPI::from_dpi(192, 192),
            "更新モニタ内の窓が書き換わっていない（対照が効いていない証拠）"
        );
    }

    /// **`CW_USEDEFAULT` の窓が本番経路に実在する**（`WindowPos::default()` ＋ `DPI` は
    /// `Window` の `on_window_add` フックが揃えて挿入する）。素通しすると
    /// `position.x + size.width / 2` が桁溢れし、dev ビルドでは panic で UI スレッドが死ぬ。
    ///
    /// **本檻は「桁溢れしない」ではなく「打ち切られる」を主張する**——桁溢れするコードは
    /// dev では panic して赤、release では wrap した中心が偶然どのモニタにも入らず
    /// 「DPI は書き換わらないが打ち切りログも出ない」形で赤になる（どちらのプロファイルでも
    /// 検出される）。
    #[test]
    fn window_with_cw_usedefault_is_skipped_before_overflow() {
        let (mut world, _) = probe_world(vec![probe_monitor_after()]);
        // 本番の on_window_add フックが挿入するのと同じ状態（WindowPos::default() ＋ DPI）。
        let window = world.spawn((WindowPos::default(), DPI::default())).id();
        // 探針の前提: 既定値が確かにセンチネルであること（既定値が変わったら檻ごと見直す）。
        let default_pos = WindowPos::default();
        assert_eq!(
            default_pos.position.expect("既定の position").x,
            CW_USEDEFAULT,
            "WindowPos::default() が CW_USEDEFAULT でない＝本檻の前提が崩れている"
        );

        let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
            run_apply(&mut world)
        });

        assert_eq!(
            *world.get::<DPI>(window).expect("窓の DPI"),
            DPI::default(),
            "座標未確定（CW_USEDEFAULT）の窓が書き換わっている"
        );
        assert_eq!(
            redrive_count(&world),
            0,
            "座標未確定の窓を駆動対象に数えている"
        );
        assert!(
            out.contains("Window position/size undetermined, DPI redrive skipped"),
            "CW_USEDEFAULT が「未確定」として打ち切られていない: {out}"
        );
    }

    /// 位置・寸が未確定の窓は帰属判定できないため打ち切る（正常系・debug 水準）。
    #[test]
    fn window_without_position_is_skipped_at_debug_level() {
        let (mut world, _) = probe_world(vec![probe_monitor_after()]);
        let window = world
            .spawn((
                WindowPos {
                    position: None,
                    size: None,
                    ..Default::default()
                },
                DPI::from_dpi(120, 120),
            ))
            .id();

        let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
            run_apply(&mut world)
        });

        assert_eq!(
            *world.get::<DPI>(window).expect("窓の DPI"),
            DPI::from_dpi(120, 120),
            "帰属不明の窓が書き換わっている"
        );
        assert!(
            out.contains("Window position/size undetermined, DPI redrive skipped"),
            "打ち切りが観測できない: {out}"
        );
    }

    #[test]
    fn window_center_requires_both_position_and_size() {
        let full = WindowPos {
            position: Some(Point { x: 100, y: 200 }),
            size: Some(SizeI {
                width: 400,
                height: 600,
            }),
            ..Default::default()
        };
        assert_eq!(window_center(&full), Some((300, 500)));

        let no_size = WindowPos {
            size: None,
            ..full
        };
        assert_eq!(window_center(&no_size), None);

        let no_pos = WindowPos {
            position: None,
            ..full
        };
        assert_eq!(window_center(&no_pos), None);
    }

    /// 「未確定」は `None` だけではない——wintf の正典センチネル `CW_USEDEFAULT` も未確定。
    ///
    /// `CW_USEDEFAULT == i32::MIN` ゆえ素通しは整数桁溢れになる。判定語は同 crate の
    /// 既存 3 箇所（`apply_window_pos_changes`／`sync_window_arrangement_from_window_pos`／
    /// `WindowPos::to_window_rect`）と同一。
    #[test]
    fn window_center_treats_cw_usedefault_as_undetermined() {
        // 既定値そのもの（位置・寸ともセンチネル）。
        assert_eq!(window_center(&WindowPos::default()), None);

        // 位置だけセンチネル。
        let pos_sentinel = WindowPos {
            position: Some(Point {
                x: CW_USEDEFAULT,
                y: CW_USEDEFAULT,
            }),
            size: Some(SizeI {
                width: 400,
                height: 600,
            }),
            ..Default::default()
        };
        assert_eq!(window_center(&pos_sentinel), None);

        // 寸だけセンチネル。
        let size_sentinel = WindowPos {
            position: Some(Point { x: 100, y: 200 }),
            size: Some(SizeI {
                width: CW_USEDEFAULT,
                height: CW_USEDEFAULT,
            }),
            ..Default::default()
        };
        assert_eq!(window_center(&size_sentinel), None);

        // 探針が退化していないこと: センチネルは実際に i32::MIN であり、
        // 素通しすれば加算が桁溢れする値である。
        assert_eq!(CW_USEDEFAULT, i32::MIN, "センチネルの実値が変わっている");
    }

    #[test]
    fn monitor_containing_uses_half_open_bounds() {
        let m = probe_monitor_before();
        let monitors = [m.clone()];

        assert!(monitor_containing(&monitors, (0, 0)).is_some(), "左上端は含む");
        assert!(
            monitor_containing(&monitors, (3839, 2159)).is_some(),
            "右下端の 1px 内側は含む"
        );
        assert!(
            monitor_containing(&monitors, (3840, 1000)).is_none(),
            "右端は含まない（半開区間）"
        );
        assert!(
            monitor_containing(&monitors, (1000, 2160)).is_none(),
            "下端は含まない（半開区間）"
        );
        assert!(
            monitor_containing(&monitors, (-1, 1000)).is_none(),
            "左外は含まない"
        );
    }
}
