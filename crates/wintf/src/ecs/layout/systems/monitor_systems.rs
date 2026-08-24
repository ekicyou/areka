//! Monitor階層管理システム

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info, warn};

use super::super::taffy::TaffyLayoutResource;
use super::super::{
    BoxInset, BoxPosition, BoxSize, BoxStyle, Dimension, LayoutRoot, LengthPercentageAuto, Rect,
};
use crate::ecs::window::transition_diag::{self, MonitorRecord, Stamp, TickStart};
use crate::ecs::world::FrameCount;

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
///
/// # 遷移観測の刻印は World 資源から組む（design D1）
///
/// 本システムは `Update` に登録されており、`Update` は**既定の多スレッド実行器**で回る。
/// ワーカースレッドからは `transition_diag` のスレッド局所ミラーが見えない（`frame=0` の
/// 行が出る）ため、刻印は必ず `Res<FrameCount>`＋`Res<TickStart>` から
/// [`transition_diag::stamp_from_world`] で組む——`transition_diag::stamp()` を呼ぶのは
/// 退行である。
// bevy のシステムは引数がそのまま World への要求（Query／Res）であり、束ねると
// 要求が読めなくなる。刻印の 2 資源（D1）を足したことで 7 を超えるが、まとめない。
#[allow(clippy::too_many_arguments)]
pub fn detect_display_change_system(
    mut commands: Commands,
    mut app: ResMut<crate::ecs::App>,
    layout_root: Query<Entity, With<LayoutRoot>>,
    mut existing_monitors: Query<(Entity, &mut crate::ecs::Monitor), With<crate::ecs::Monitor>>,
    mut windows: Query<(Entity, &crate::ecs::WindowPos, &mut crate::ecs::DPI)>,
    mut taffy_res: ResMut<TaffyLayoutResource>,
    frame: Res<FrameCount>,
    tick_start: Res<TickStart>,
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
        transition_diag::stamp_from_world(&frame, &tick_start),
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
///
/// # `stamp`
/// 値変化更新の `monitor` レコードに載せる刻印。**呼出側が World 資源から組んで渡す**
/// （design D1）——本関数は多スレッド実行器でワーカースレッドに載り得るため、自分で
/// スレッド局所ミラーを読んではならない。
pub(crate) fn apply_monitor_snapshot(
    commands: &mut Commands,
    root_entity: Entity,
    existing_monitors: &mut Query<(Entity, &mut crate::ecs::Monitor), With<crate::ecs::Monitor>>,
    windows: &mut Query<(Entity, &crate::ecs::WindowPos, &mut crate::ecs::DPI)>,
    taffy_res: &mut TaffyLayoutResource,
    new_monitors: Vec<crate::ecs::Monitor>,
    stamp: Stamp,
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

                // 遷移観測（要件 2.3）。上の `debug` 行と同じ事実を、1 回の遷移の
                // 時系列へ並べられる形（刻印つき・単一 target）で出す。既定 OFF ゆえ
                // 前置ガードの内側でだけ組む。刻印は呼出側が World 資源から組んで
                // 渡した値をそのまま使う（D1・本関数は時刻を読まない）。
                if transition_diag::is_enabled() {
                    transition_diag::emit_line(&transition_diag::monitor_line(&MonitorRecord {
                        stamp,
                        entity,
                        old_dpi: existing_monitor.dpi,
                        new_dpi: new_monitor.dpi,
                        old_work_area: existing_monitor.work_area,
                        new_work_area: new_monitor.work_area,
                    }));
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
///
/// # クレート外へ開いてある理由（areka-P0-dpi-transition-atomicity C5・task 5.4）
///
/// 上位クレートの整合ゲート（配置側が「窓の拡大率と帰属モニタの表が揃ったか」を判定する
/// 点）も、帰属を決めるには**同じ中心**を要る。素直に `position + size / 2` と書くと
/// 上の `CW_USEDEFAULT` の扱いが落ちて、窓生成前の窓で整数桁溢れを起こす（dev ビルドでは
/// panic）。中心の求め方は 1 箇所でなければならない。
pub fn window_center(pos: &crate::ecs::WindowPos) -> Option<(i32, i32)> {
    let position = pos.position?;
    let size = pos.size?;

    // CW_USEDEFAULT が含まれる場合は「未確定」（ウィンドウ作成時の初期値）。
    if position.x == CW_USEDEFAULT || size.width == CW_USEDEFAULT {
        return None;
    }

    Some((position.x + size.width / 2, position.y + size.height / 2))
}

/// 帰属判定の入力＝モニタの**境界矩形**（`work_area` ではない）。
///
/// 表示基盤の [`Monitor`](crate::ecs::Monitor) と、上位クレートが持つモニタ別の表とでは
/// 行の型が違う。だが「どのモニタに属するか」の規則は 1 つでなければならない——別々に
/// 書けば、片方だけが半開区間の端や先勝ちの向きを変えたときに静かに食い違う。本トレイトは
/// [`monitor_containing`] を型に依らない形で使えるようにするためだけに在る。
pub trait MonitorBounds {
    /// 境界矩形（画面座標・物理 px）。
    fn monitor_bounds(&self) -> windows::Win32::Foundation::RECT;
}

impl MonitorBounds for crate::ecs::Monitor {
    fn monitor_bounds(&self) -> windows::Win32::Foundation::RECT {
        self.bounds
    }
}

/// 中心点を含むモニタを返す（境界矩形は左上を含み右下を含まない半開区間・昇順先勝ち）。
///
/// # クレート外へ開いてある理由（areka-P0-dpi-transition-atomicity C5・task 5.4）
///
/// 本関数は表示基盤側の DPI 再導出（[`redrive_window_dpi_for_updated_monitors`]）が使う
/// 帰属規則そのものである。上位クレートの整合ゲートは「窓の拡大率と、窓が属するモニタの
/// 表の拡大率が揃ったか」を判定するので、**同じ帰属規則**を使わなければならない——
/// 別規則を置くと、どちらのモニタに属するかの答えが 2 つになり、ゲートが永遠に解けない
/// 窓が生まれ得る。要素の型が違うだけなので [`MonitorBounds`] 越しに受ける
/// （呼出側は自分の行型へ実装を 1 つ書く）。
pub fn monitor_containing<M: MonitorBounds>(monitors: &[M], center: (i32, i32)) -> Option<&M> {
    let (x, y) = center;
    monitors.iter().find(|m| {
        let bounds = m.monitor_bounds();
        x >= bounds.left && x < bounds.right && y >= bounds.top && y < bounds.bottom
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
#[path = "monitor_systems_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "monitor_systems_transition_tests.rs"]
mod monitor_systems_transition_tests;
