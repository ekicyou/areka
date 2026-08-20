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
//! - **同期そのものは窓を動かさない**。[`sync_monitor_snapshot`] は資源を差し替えるだけで、
//!   窓書込を 1 件も出さない。拡大率が変わらず作業領域だけが変わった窓の再射影は、同一
//!   フレームの**拡大率の相の後**に置かれる [`resnap_for_work_area_change`]（task 5.2）が
//!   持つ。
//! - **保存位置の復元判定に効かない**（要件 5.7）。復元は起動時に 1 度だけ源を読む契約で
//!   あり、拡大率をまたぐ保存位置の追従は行わない（`windowposition-limit` の開発者裁定）。

use bevy_ecs::prelude::*;
use tracing::warn;
use wintf::ecs::WindowPos;
use wintf::ecs::window::monitor::Monitor;

use crate::placement::diag::PlacementRoute;
use crate::placement::follow::{
    Anchored, MonitorDpiTable, MonitorSnapshot, MonitorSources, project_anchor, same_monitors,
};
use crate::placement::resolver::{Anchor, PointPx, SizePx};
use crate::placement::spawn::CharWindowMarker;
use crate::placement::transition_diag::{self, MonitorEntry};
use crate::placement::{WORK_AREA_SYNC_CONTEXT, diag, monitor_records};

use super::dpi::reproject_char_window_at_current_size;

/// 作業領域源が実際に差し替わったこと（＝作業領域が動いたフレームであること）。
///
/// 差分の中身（どの作業領域がどう動いたか）は**呼び手が両者を突き合わせて読む**。
/// 消費者は [`resnap_for_work_area_change`]（task 5.2）で、動いた作業領域に属する下端吸着の
/// キャラ窓だけを現寸で射影し直す。
#[derive(Debug, Clone)]
pub(super) struct SnapshotChange {
    /// 差し替え**前**の作業領域源（資源が無かった場合は空）。
    pub(super) previous: MonitorSnapshot,
    /// 差し替え**後**の作業領域源。
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

// ---------------------------------------------------------------------------
// 作業領域変化を契機とする再スナップ（task 5.2・設計 C6・要件 5.1／5.2／5.3／5.4／4.7）
// ---------------------------------------------------------------------------

/// 作業領域が動いた窓を、**現在の寸のまま**新しい下端へ射影し直す（設計 C6）。
///
/// 呼ぶのは [`sync_monitor_snapshot`] が差し替えを報告したフレームだけである——変化が
/// 無ければ呼び手が [`SnapshotChange`] を持たないので、本関数は**そもそも走らない**
/// （要件 5.2・4.7 の「変化の無いフレームで窓書込 0」はこの構造が担う）。
///
/// # なぜ拡大率の相の**後**なのか
///
/// 拡大率も一緒に動いたフレームでは、拡大率の相（`Changed<DPI>` 駆動）が同じ窓を既に
/// 新しい下端へ書き終えている。相の**前**に置くと、まだ旧寸のままの窓を先に動かして
/// から相が書き直す 2 段書込になる。後に置けば、相が書き終えた窓は導出値が現在値と
/// 一致して [`resize_window_to`](crate::placement::follow::resize_window_to) の
/// べき等 skip が書込ゼロで抜ける（＝合流）。
///
/// # 対象の選び方——**射影 T の値が動くか**で決める
///
/// 窓ごとに、差し替え**前**の源と**後**の源で [`project_anchor`] を 1 度ずつ通し、結果が
/// 変わる窓だけを再射影する。帰属の規則（どのモニタに属するか）をここで発明しないための
/// 形である——判定は本番の射影がそのまま使う 1 つの関数に委ねてあり、二重権威にならない
/// （帰属規則そのものの共有は task 5.4 の持ち分）。
///
/// この選び方は「作業領域が変わった窓」より狭い側へ倒れている: 作業領域の左端だけが動いた
/// ような、下端吸着の位置に影響しない変化では射影値が変わらず、再射影を行わない。書込が
/// 出ないことは変わらないが、**呼び出しごと省ける**ぶん要件 5.2 に忠実である。
///
/// 対象は**下端吸着（`Anchor::Bottom`）のキャラ窓**に限る（設計 C6）。バルーン窓は位置が
/// 従属量ゆえ触らない——キャラ窓が動けば同一の [`resize_window_to`] 呼出の内側で随伴が
/// 追従する（窓左上相対・追従 offset は補正しない・要件 10.1）。
///
/// # 確保（要件 10.4）
///
/// 対象の収集で `Vec` を 1 つ使うが、走るのは作業領域が動いたフレームだけであり、定常
/// フレームでは本関数に到達しない（＝定常状態の確保は増えない）。
pub(super) fn resnap_for_work_area_change(world: &mut World, change: &SnapshotChange) {
    // World の不変借用（query）を `&mut World` のループへ跨がせないため、先に対象を
    // collect して借用を解放する（`dpi_phase_with` と同じ collect→release→&mut ループ）。
    let mut targets: Vec<Entity> = Vec::new();
    let mut query =
        world.query_filtered::<(Entity, &Anchored, &WindowPos), With<CharWindowMarker>>();
    for (window, &Anchored(anchor), window_pos) in query.iter(world) {
        // 下端吸着だけが対象（設計 C6）。Free は「位置を一切動かさない」契約であり、
        // Top／Left／Right は下端以外の辺が原点なので本タスクの守備範囲外である。
        if !matches!(anchor, Anchor::Bottom) {
            continue;
        }
        // 位置か寸が未確定（窓生成前）の窓は接地点そのものが存在しない。射影値を比べる
        // 材料が無いので対象に入れない——ここで無理に射影を通すと、窓が出来ていないだけの
        // 正常な起動途中に警告が並ぶ。窓が生えた後の変化は次の差し替えで拾える。
        let (Some(position), Some(size)) = (window_pos.position, window_pos.size) else {
            continue;
        };
        let raw = PointPx {
            x: position.x,
            y: position.y,
        };
        let size = SizePx {
            w: size.width,
            h: size.height,
        };
        // 射影 T を 2 つの源で 1 度ずつ通す（本番の射影と同一関数＝規則を二重に持たない）。
        let before = project_anchor(anchor, raw, size, Some(&change.previous));
        let after = project_anchor(anchor, raw, size, Some(&change.current));
        if before != after {
            targets.push(window);
        }
    }

    for window in targets {
        // 現寸のまま射影 T を一度通す。同値ならべき等 skip で書込ゼロ（＝拡大率の相が
        // 既に書き終えた窓）、動くべき窓だけが 1 回書かれる。縮退（破棄済み・寸未確定）は
        // 当該関数が log-first で吸収する。
        reproject_char_window_at_current_size(world, window, PlacementRoute::WorkAreaResnap);
    }
}
