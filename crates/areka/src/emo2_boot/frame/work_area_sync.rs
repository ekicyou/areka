//! 作業領域源の実行時同期（設計 C6・要件 5.1／5.4／5.5／5.6／5.7）。
//!
//! 実行時のモニタ表（wintf の `Monitor` component 群）から、配置が読む 2 つの源——
//! 作業領域源 [`MonitorSnapshot`] とモニタ別拡大率表 [`MonitorDpiTable`]——を作り直し、
//! **内容が変わったフレームだけ**差し替える。
//!
//! # なぜ要るか（確定台帳 L3）
//!
//! 作業領域源は起動時に 1 度だけ作られ、以後どのフレームでも作り直されなかった
//! （`main.rs` の起動シームが唯一の構築点）。タスクバーの高さは**論理寸**で宣言され
//! 物理 px では拡大率に比例して伸び縮みするので、拡大率が変われば真の作業領域下端が動く。
//! 起動時の値が焼き付いたままだと、下端吸着のキャラ窓は古い下端へ接地し続ける——実機で
//! 200%→100% の 6 遷移すべてが接地点 −48px の浮きを出した（起動時が 200% だったため、
//! 100%→200% の側だけが偶然一致していた）。
//!
//! # 置き場が拡大率の相より**前**である理由
//!
//! 同期は毎フレームの先頭に置く。拡大率の相（`run_dpi_phase`）は `Changed<DPI>` の窓を
//! 現寸で射影し直すが、その射影が読むのは作業領域源である。同一フレームの先頭で源が
//! 新しくなっていれば、相は**新しい下端へ 1 回で**書く。相の後に同期を置くと、相は旧下端へ
//! 書いてから源が新しくなり、次のフレームで書き直す 2 段書込になる（要件 5.8 が禁じる形）。
//!
//! # ここがしないこと
//!
//! - **窓を動かさない**。同期は資源を差し替えるだけで、窓書込を 1 件も出さない。拡大率が
//!   変わらず作業領域だけが変わった窓の再射影は作業領域変化を契機とする再スナップ
//!   （task 5.2）が持つ。
//! - **保存位置の復元判定に効かない**（要件 5.7）。復元は起動時に 1 度だけ源を読む契約で
//!   あり、拡大率をまたぐ保存位置の追従は行わない（`windowposition-limit` の開発者裁定）。

use bevy_ecs::prelude::*;
use tracing::warn;
use wintf::ecs::window::monitor::Monitor;

use crate::placement::follow::{MonitorDpiTable, MonitorSnapshot, MonitorSources, same_monitors};
use crate::placement::transition_diag::{self, MonitorEntry};
use crate::placement::{WORK_AREA_SYNC_CONTEXT, diag, monitor_records};

/// 作業領域源が実際に差し替わったこと（＝作業領域が動いたフレームであること）。
///
/// 差分の中身（どの作業領域がどう動いたか）は**呼び手が両者を突き合わせて読む**。
/// 作業領域変化を契機とする再スナップ（task 5.2）がこの値を受け取り、動いた作業領域に
/// 属する下端吸着のキャラ窓だけを現寸で射影し直す。
#[derive(Debug, Clone)]
pub(super) struct SnapshotChange {
    /// 差し替え**前**の作業領域源（資源が無かった場合は空）。
    // 消費者は task 5.2（作業領域変化を契機とする再スナップ）。areka は bin crate ゆえ
    // `pub` でも dead_code 免除されない（`placement::diag` と同じ事情）。
    #[allow(dead_code)]
    pub(super) previous: MonitorSnapshot,
    /// 差し替え**後**の作業領域源。
    #[allow(dead_code)]
    pub(super) current: MonitorSnapshot,
}

/// 実行時のモニタ表から 2 源を作り直す（毎フレーム先頭の同期段・本番の入口）。
///
/// 走査するのは World に居る `Monitor` component の全 entity である。表そのものの更新は
/// 表示基盤側（`monitor_systems`）が所有し、ここは**読むだけ**——同じ表を 2 箇所で作ると
/// 二重権威になる。
pub(super) fn sync_monitor_snapshot(world: &mut World) -> Option<SnapshotChange> {
    let mut query = world.query::<&Monitor>();
    let monitors: Vec<Monitor> = query.iter(world).cloned().collect();
    sync_monitor_snapshot_with(world, &monitors)
}

/// 与えられたモニタ表で 2 源を作り直す（決定論テストが合成表を直接渡せる形）。
///
/// 戻り値は**差し替えが起きたか**（`None`＝作り直していない）。`None` を返す腕は 2 つ:
///
/// - モニタ 0 台（列挙異常）: `warn!` の上で現状維持する（要件 5.5）。架空の作業領域を
///   発明しないのと同じ理由で、**空の源で今の源を潰さない**——潰すと下端が引けなくなって
///   全キャラ窓の射影が縮退し、窓が画面外へ出る。
/// - 表の内容が現在の源と同じ（順序の違いは同じと見なす・要件 5.4）: 何もしない。
///
/// 差し替えたときだけ 2 種のログを出す——遷移観測の `kind=snapshot` 行（判定器が読む）と、
/// `[diag.monitor_snapshot]`（人が構成を読む既存の共有ヘルパ・呼出点タグで出所が判る）。
pub(super) fn sync_monitor_snapshot_with(
    world: &mut World,
    monitors: &[Monitor],
) -> Option<SnapshotChange> {
    if monitors.is_empty() {
        warn!(
            "[{WORK_AREA_SYNC_CONTEXT}] モニタ表が空（列挙異常）→ 作業領域源を差し替えず現状維持"
        );
        return None;
    }

    let next = MonitorSources::from_monitors(monitors);
    let previous = world.get_resource::<MonitorSnapshot>().cloned();
    let current_sources = match (previous.clone(), world.get_resource::<MonitorDpiTable>()) {
        (Some(snapshot), Some(dpi_table)) => Some(MonitorSources {
            snapshot,
            dpi_table: dpi_table.clone(),
        }),
        // 片方でも無ければ「同じ」とは言えない（起動シームを通らない経路・資源の欠落）。
        // 値を捏造せず、下で作り直して両方を揃える。
        _ => None,
    };
    if let Some(current_sources) = &current_sources
        && same_monitors(current_sources, &next)
    {
        return None;
    }

    let change = SnapshotChange {
        previous: previous.unwrap_or(MonitorSnapshot {
            work_areas: Vec::new(),
        }),
        current: next.snapshot.clone(),
    };

    // 遷移観測（既定 OFF）。**行の材料を組むより外側に**前置ガードを置く——組んでから捨てる形に
    // すると、観測が消えている運転でも確保が走る（要件 10.4 の定常アロケーション 0 は
    // 「既定運転で新たな確保が起きない」ことを含む）。
    if transition_diag::is_enabled() {
        let record: Vec<MonitorEntry> = next
            .snapshot
            .work_areas
            .iter()
            .zip(next.dpi_table.entries.iter())
            .map(|(work_area, entry)| MonitorEntry {
                dpi: entry.dpi,
                work_area: *work_area,
            })
            .collect();
        transition_diag::log_monitor_snapshot_sync(world, &record);
    }
    world.insert_resource(next.snapshot);
    world.insert_resource(next.dpi_table);
    // 人が読む側の構成ログ（起動時の構築点と同じ共有ヘルパ・呼出点タグだけが違う）。
    diag::log_monitor_snapshot(&monitor_records(monitors), WORK_AREA_SYNC_CONTEXT);

    Some(change)
}
