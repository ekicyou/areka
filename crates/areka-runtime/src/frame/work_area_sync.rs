//! 作業領域源の実行時同期。
//!
//! 実行時のモニタ表（wintf の `Monitor` component 群）から、配置が読む 2 つの源——作業領域源 [`MonitorSnapshot`] とモニタ別拡大率表 [`MonitorDpiTable`]——を作り直し、内容が変わったフレームだけ差し替える。
//!
//! # なぜ要るか
//!
//! 作業領域源は起動時に 1 度だけ作られ、以後作り直されなかった（`main.rs` の起動シームが唯一の構築点）。タスクバーの高さは論理寸で宣言され、物理 px では拡大率に比例して伸び縮みするので、拡大率が変われば真の作業領域下端が動く。起動時の値が焼き付いたままだと、下端吸着のキャラ窓は古い下端へ接地し続ける（実機で 200%→100% の 6 遷移すべてが接地点 −48px の浮きを出した。起動時が 200% だったため、100%→200% の側だけが偶然一致していた）。
//!
//! # 置き場が拡大率の相より前である理由
//!
//! 同期は毎フレームの先頭に置く。拡大率の相（`run_dpi_phase`）は `Changed<DPI>` の窓を現寸で射影し直し、その射影は作業領域源を読む。同一フレームの先頭で源が新しくなっていれば、相は新しい下端へ 1 回で書く。相の後に置くと、旧下端へ書いてから次のフレームで書き直す 2 段書込になる。
//!
//! # ここがしないこと
//!
//! - 同期そのものは窓を動かさない。[`sync_monitor_snapshot`] は資源を差し替えるだけで窓書込を出さない。拡大率が変わらず作業領域だけが変わった窓の再射影は、拡大率の相の後に置かれる [`resnap_for_work_area_change`] が持つ。
//! - 保存位置の復元判定に効かない。復元は起動時に 1 度だけ源を読む契約で、拡大率をまたぐ保存位置の追従は行わない。

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use tracing::warn;
use wintf::ecs::WindowPos;
use wintf::ecs::window::monitor::Monitor;

use areka_placement::diag::PlacementRoute;
use areka_placement::dpi_sync::{self, HoldSite};
use areka_placement::follow::{
    Anchored, MonitorDpiTable, MonitorSnapshot, MonitorSources, project_anchor, same_monitors,
};
use areka_placement::resolver::{Anchor, PointPx, SizePx};
use areka_placement::spawn::CharWindowMarker;
use areka_placement::transition_diag::{self, MonitorEntry};
use areka_placement::{WORK_AREA_SYNC_CONTEXT, diag, monitor_records};

use super::dpi::reproject_char_window_at_current_size;

/// 作業領域源が実際に差し替わったこと（＝作業領域が動いたフレームであること）。
///
/// 差分の中身は呼び手が両者を突き合わせて読む。消費者は [`resnap_for_work_area_change`] で、動いた作業領域に属する下端吸着のキャラ窓だけを現寸で射影し直す。
// 公開の `FrameHarness::run_work_area_sync`・`run_placement_phases` が名指すので開ける
#[derive(Debug, Clone)]
pub struct SnapshotChange {
    /// 差し替え**前**の作業領域源（資源が無かった場合は空）。
    pub(super) previous: MonitorSnapshot,
    /// 差し替え**後**の作業領域源。
    pub(super) current: MonitorSnapshot,
}

type MonitorWatchParams = (
    Query<'static, 'static, (), Changed<Monitor>>,
    Query<'static, 'static, (), With<Monitor>>,
    Option<Res<'static, MonitorSnapshot>>,
    Option<Res<'static, MonitorDpiTable>>,
);

/// フレームを跨いで生きるモニターの変化の見張り（初回の同期で World に据える）。
#[derive(Resource)]
struct MonitorWatch {
    params: SystemState<MonitorWatchParams>,
    /// 前に表を集めたときのモニターの数（消えたモニターは `Changed` に出ないので数で見る）。
    /// `None` は一度も集めていない（初回はモニターが 0 台でも必ず集める）。
    seen: Option<usize>,
}

/// 実行時のモニタ表から 2 源を作り直す（毎フレーム先頭の同期段・本番の入口）。
///
/// 走査するのは World に居る `Monitor` component の全 entity。表の更新は表示基盤側（`monitor_systems`）が所有し、ここは読むだけ（同じ表を 2 箇所で作ると二重権威になる）。モニターの component が変わらず、2 源も前の同期の後に入れ直されていないフレームは表を集めずに `None` を返す。
pub(super) fn sync_monitor_snapshot(world: &mut World) -> Option<SnapshotChange> {
    if !world.contains_resource::<MonitorWatch>() {
        let params = SystemState::new(world);
        world.insert_resource(MonitorWatch { params, seen: None });
    }
    world.resource_scope(|world, mut watch: Mut<MonitorWatch>| {
        let seen = watch.seen;
        let unchanged = watch
            .params
            .get(world)
            .is_ok_and(|(changed, all, snapshot, dpi_table)| {
                changed.is_empty()
                    && seen == Some(all.iter().count())
                    && snapshot.is_some_and(|s| !s.is_changed())
                    && dpi_table.is_some_and(|d| !d.is_changed())
            });
        if unchanged {
            return None;
        }
        let monitors: Vec<Monitor> = world.query::<&Monitor>().iter(world).cloned().collect();
        let change = sync_monitor_snapshot_with(world, &monitors);
        // ここで入れ直した 2 源を、次の呼び出しが「外から入れ直された」と数えないため。
        if let Err(error) = watch.params.get(world) {
            warn!(
                %error,
                "[{WORK_AREA_SYNC_CONTEXT}] 見張りの印を進められない（次のフレームで 1 回作り直す）"
            );
        }
        watch.seen = Some(monitors.len());
        change
    })
}

/// 与えられたモニタ表で 2 源を作り直す（決定論テストが合成表を直接渡せる形）。
///
/// 戻り値は**差し替えが起きたか**（`None`＝作り直していない）。`None` を返す腕は 2 つ:
///
/// - モニタ 0 台（列挙異常）: `warn!` の上で現状維持する。架空の作業領域を
///   発明しないのと同じ理由で、**空の源で今の源を潰さない**——潰すと下端が引けなくなって
///   全キャラ窓の射影が縮退し、窓が画面外へ出る。
/// - 表の内容が現在の源と同じ（順序の違いは同じと見なす）: 何もしない。
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

    // 遷移観測（既定 OFF）。行の材料を組むより外側に前置ガードを置く——組んでから捨てる形だと、観測が消えている運転でも確保が走る。
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
// 作業領域変化を契機とする再スナップ
// ---------------------------------------------------------------------------

// 外へはテストのビルド（`test-support`）でだけ開く。crate の中は毎フレームの相が使う（道と名前は同じ）。
#[cfg(feature = "test-support")]
pub use test_open::resnap_for_work_area_change;
#[cfg(not(feature = "test-support"))]
pub(super) use test_open::resnap_for_work_area_change;

mod test_open {
    use super::*;

    /// 作業領域が動いた窓を、現在の寸のまま新しい下端へ射影し直す。
    ///
    /// 呼ぶのは [`sync_monitor_snapshot`] が差し替えを報告したフレームだけ（変化が無ければ呼び手が [`SnapshotChange`] を持たないので本関数は走らない＝変化の無いフレームの窓書込 0 はこの構造が担う）。
    ///
    /// # なぜ拡大率の相の後なのか
    ///
    /// 拡大率も一緒に動いたフレームでは、拡大率の相（`Changed<DPI>` 駆動）が同じ窓を既に新しい下端へ書き終えている。相の前に置くと、旧寸の窓を先に動かしてから相が書き直す 2 段書込になる。後に置けば、書き終えた窓は導出値が現在値と一致して [`resize_window_to`](areka_placement::follow::resize_window_to) のべき等 skip で書込ゼロになる（＝合流）。
    ///
    /// # 対象の選び方（射影 T の値が動くか）
    ///
    /// 窓ごとに、差し替え前後の源で [`project_anchor`] を 1 度ずつ通し、結果が変わる窓だけを再射影する。帰属の規則（どのモニタに属するか）をここで発明せず、本番の射影が使う 1 つの関数に委ねるので二重権威にならない。
    ///
    /// 「作業領域が変わった窓」より狭い側に倒れている: 作業領域の左端だけが動いたような、下端吸着の位置に影響しない変化では射影値が変わらず、再射影の呼び出しごと省ける。
    ///
    /// 対象は下端吸着（`Anchor::Bottom`）のキャラ窓に限る。バルーン窓は位置が従属量なので触らない（キャラ窓が動けば同一の [`resize_window_to`] 呼出の内側で随伴が追従する。窓左上相対・追従 offset は補正しない）。
    ///
    /// # 整合待ちの窓は書かない
    ///
    /// 対象に選ばれても [`DpiSyncHold`](areka_placement::dpi_sync::DpiSyncHold) の付いた窓へは書かない。原則は待ち札のある窓へのすべての窓書込を見送ることで、例外は明示の操作と随伴（正本は `areka_placement` の `deferral_covers_route`）。
    ///
    /// # 見送った窓は取り残されない
    ///
    /// ほかの見送り点は毎フレーム走るが、本関数は作業領域源が差し替わったフレームだけ走るので、見送ったフレームの [`SnapshotChange`] は二度と来ない。それでも取り残されないのは、札を外すのが拡大率の相だけで、その相が `Changed<DPI>` と札を持つ窓の和集合を対象に取るから。解除フレームには当該窓が必ず対象に入り、[`reproject_char_window_at_current_size`] が差し替え済みの作業領域源（同期段は札に関係なく源を差し替える）で接地点を引き直す。見送りは作業領域の変化を捨てず、反映を解除フレームの 1 本へ合流させる（一度書き）。
    ///
    /// # 確保
    ///
    /// 対象の収集で `Vec` を 1 つ使うが、走るのは作業領域が動いたフレームだけで、定常フレームでは到達しない。
    pub fn resnap_for_work_area_change(world: &mut World, change: &SnapshotChange) {
        // World の不変借用（query）を `&mut World` のループへ跨がせないため、先に対象を collect して借用を解放する。
        let mut targets: Vec<Entity> = Vec::new();
        let mut query =
            world.query_filtered::<(Entity, &Anchored, &WindowPos), With<CharWindowMarker>>();
        for (window, &Anchored(anchor), window_pos) in query.iter(world) {
            // 下端吸着だけが対象。Free は位置を一切動かさない契約で、Top／Left／Right は下端以外の辺が原点。
            if !matches!(anchor, Anchor::Bottom) {
                continue;
            }
            // 位置か寸が未確定（窓生成前）の窓は接地点が無く、射影を通すと正常な起動途中に警告が並ぶので対象に入れない。
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
            let before = project_anchor(anchor, raw, size, Some(&change.previous));
            let after = project_anchor(anchor, raw, size, Some(&change.current));
            if before != after {
                targets.push(window);
            }
        }

        for window in targets {
            // 整合ゲート: 待ち札のある窓へはこの経路からも書かない。対象の選定の後に置く——選定前だと、自分の作業領域が動いていない窓にも見送り行が並ぶ。
            if dpi_sync::defers_window_write(world, window, HoldSite::WorkAreaResnap) {
                continue;
            }
            // 同値ならべき等 skip で書込ゼロ（拡大率の相が書き終えた窓）。縮退（破棄済み・寸未確定）は当該関数が log-first で吸収する。
            reproject_char_window_at_current_size(world, window, PlacementRoute::WorkAreaResnap);
        }
    }
}

// 本体のテスト（`crates/areka/src/runtime_tests/`）が使う私有の関数（中で記録を出す相の部品）へそのまま渡す口（`test-support` のときだけ・
// 本番の可視性は変えない）。
#[cfg(feature = "test-support")]
pub mod test_support {
    use bevy_ecs::world::World;
    use wintf::ecs::window::monitor::Monitor;

    use super::SnapshotChange;

    /// [`super::sync_monitor_snapshot_with`] へそのまま渡す。
    pub fn sync_monitor_snapshot_with(
        world: &mut World,
        monitors: &[Monitor],
    ) -> Option<SnapshotChange> {
        super::sync_monitor_snapshot_with(world, monitors)
    }
}
