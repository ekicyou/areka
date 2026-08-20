//! 窓ごとの整合ゲート（設計 C5・要件 5.8／4.1／4.6／10.7）。
//!
//! 窓の拡大率（`DPI` component）と、その窓が属するモニタの**表**（[`MonitorDpiTable`]）の
//! 拡大率が揃うまで、当該窓への**窓書込**を見送る。見送りの札は [`DpiSyncHold`] であり、
//! 付け外しは拡大率の相が一元的に行う。
//!
//! # なぜ要るか（要件 5.8）
//!
//! Windows は `WM_DPICHANGED`（窓の拡大率）と `WM_DISPLAYCHANGE`（モニタ表）の到着順を
//! 保証しない。拡大率通知が先に届く順序では、窓は新しい拡大率を持つのに作業領域源は旧水準の
//! ままである。そのまま再射影すると「**新しい寸のまま旧作業領域下端へ接地した**」中間矩形が
//! 1 度書かれ、表が追いついたフレームでもう 1 度書き直される——要件 5.8 が名指しで禁じる
//! 2 段書込である。実機の 12 遷移では表更新が先だった（確定台帳 L2＝経路 (a) は本採取では
//! 未発火）ので、この経路を押さえるのは決定論テストだけである。
//!
//! # 見送るのは窓書込であって描画ではない（設計討議 議題 1）
//!
//! 待ち札のある窓でも `apply_show`（描画）は止めない——発話もアニメも遅らせない。止めるのは
//! 窓の矩形を書く 3 点だけである: 拡大率の相・報告寸の突合・再スナップ（[`HoldSite`] の 3 語）。
//!
//! # 有界（要件 4.4）
//!
//! 窓の中心が属するモニタと OS が拡大率を決めるモニタが食い違う縁の配置では、表が永遠に
//! 追いつかない。ゆえに待ちは [`DPI_SYNC_HOLD_MAX_FRAMES`] フレームで打ち切り、警告の上で
//! **現在の源のまま**進む（ログ無し失敗経路の禁止）。

use bevy_ecs::prelude::*;
use tracing::warn;
use wintf::ecs::layout::systems::monitor_systems::window_center;
use wintf::ecs::{DPI, WindowPos};

use super::follow::MonitorDpiTable;
use super::transition_diag::{
    self, HOLD_DECISION_HOLD, HOLD_DECISION_PROCEED, HOLD_DECISION_PROCEED_AFTER_TIMEOUT,
    HOLD_SITE_DPI, HOLD_SITE_RECONCILE, HOLD_SITE_RESNAP,
};

/// 整合待ちの上限フレーム数（設計 C5・C7）。
///
/// 判定器の許容（`transition_judge_verdict::HOLD_FRAME_ALLOWANCE`）はこの定数を参照する
/// ——待ちの上限は本番の挙動そのものであり、判定語と同じく定義元は 1 つでなければならない。
pub(crate) const DPI_SYNC_HOLD_MAX_FRAMES: u32 = 30;

/// 整合待ちの札（ゴースト窓 entity へ付く）。
///
/// 付いているあいだ、当該窓への窓書込は 3 つの点すべてで見送られる。付け外しは拡大率の相が
/// 一元的に行い、ほかの点は**読むだけ**である（2 箇所で外すと、解除フレームに書込が 2 本出る）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DpiSyncHold {
    /// 待ち始めたフレーム番号（有界の起点）。
    pub(crate) since_frame: u32,
}

/// 整合ゲートの判定（設計 C5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DpiSyncDecision {
    /// 表と一致した（または表を引けない）ので、そのまま処理する。
    Proceed,
    /// 不一致ゆえ当該窓の窓書込を見送る。
    Hold,
    /// 上限フレームを超えたので、警告の上で現在の源のまま処理する。
    ProceedAfterTimeout,
}

impl DpiSyncDecision {
    /// 観測レコードの判定語（定義元は [`transition_diag`] の 1 箇所）。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            DpiSyncDecision::Proceed => HOLD_DECISION_PROCEED,
            DpiSyncDecision::Hold => HOLD_DECISION_HOLD,
            DpiSyncDecision::ProceedAfterTimeout => HOLD_DECISION_PROCEED_AFTER_TIMEOUT,
        }
    }

    /// 窓書込を見送るか。
    pub(crate) fn is_hold(self) -> bool {
        matches!(self, DpiSyncDecision::Hold)
    }
}

/// 判定を下した観測点（設計 C5 の「3 点すべて」＝待ち札の守備範囲そのもの）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HoldSite {
    /// 拡大率の相（札の付け外しを行う唯一の点）。
    Dpi,
    /// 報告寸の突合。
    Reconcile,
    /// 再スナップ。
    Resnap,
}

impl HoldSite {
    /// 観測レコードの観測点語（定義元は [`transition_diag`] の 1 箇所）。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            HoldSite::Dpi => HOLD_SITE_DPI,
            HoldSite::Reconcile => HOLD_SITE_RECONCILE,
            HoldSite::Resnap => HOLD_SITE_RESNAP,
        }
    }
}

/// 整合ゲートの純判定（World も時刻も読まない・設計 C5）。
///
/// - `table_dpi` が `None`（表なし・どのモニタにも属さない）→ [`Proceed`](DpiSyncDecision::Proceed)。
///   表を引けない窓を待たせると、縁に置かれた窓が毎回上限まで待つ。
/// - 一致 → `Proceed`。
/// - 不一致で待ちが上限未満 → [`Hold`](DpiSyncDecision::Hold)。
/// - 不一致で待ちが上限以上 → [`ProceedAfterTimeout`](DpiSyncDecision::ProceedAfterTimeout)。
///
/// `held_since` が `None`（まだ待っていない）なら経過は 0＝これから待ち始める。経過は
/// `wrapping_sub` で取る——フレーム番号は `u32` で周回するので、素の減算では周回した瞬間に
/// 巨大な差になって上限を即座に超える（待ちが 1 度も効かないフレームが 1 周に 1 回生まれる）。
pub(crate) fn dpi_sync_decision(
    window_dpi: u32,
    table_dpi: Option<u32>,
    held_since: Option<u32>,
    now: u32,
) -> DpiSyncDecision {
    let Some(table_dpi) = table_dpi else {
        return DpiSyncDecision::Proceed;
    };
    if table_dpi == window_dpi {
        return DpiSyncDecision::Proceed;
    }
    let waited = now.wrapping_sub(held_since.unwrap_or(now));
    if waited < DPI_SYNC_HOLD_MAX_FRAMES {
        DpiSyncDecision::Hold
    } else {
        DpiSyncDecision::ProceedAfterTimeout
    }
}

/// 待ちの経過を測る現在フレーム。
///
/// 観測レコードの刻印（[`transition_diag::stamp_of`]）ではなく `FrameCount` を直接読む——
/// 刻印は `FrameCount` と `TickStart` の**両方**が揃わなければ `0` を返す仕様であり、片方だけ
/// 欠けた World では `now` が 0 に固着して待ちが永遠に上限へ届かない（有界が壊れる）。
/// `FrameCount` が無い World（tick 前・素の合成 World）で 0 になるのは同じだが、そこでは
/// そもそもフレームが進まないので待ちも進まないのが正しい。
pub(crate) fn current_frame(world: &World) -> u32 {
    world
        .get_resource::<wintf::ecs::FrameCount>()
        .map_or(0, |frame| frame.0)
}

/// 1 つの窓についてゲートを評価した結果（記録の材料をそのまま持つ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DpiSyncOutcome {
    /// 判定。
    pub(crate) decision: DpiSyncDecision,
    /// 窓が持つ拡大率。
    pub(crate) window_dpi: u32,
    /// 帰属モニタの表が持つ拡大率（帰属なし・表なしは `None`）。
    pub(crate) table_dpi: Option<u32>,
    /// 待ち始めたフレーム番号（これから待ち始めるなら `now`）。
    pub(crate) since_frame: u32,
}

/// 窓の拡大率と帰属モニタの表を突き合わせる（World を**読むだけ**）。
///
/// 帰属は窓矩形の中心で決める。中心は表示基盤側の
/// [`window_center`] が求める——`WindowPos` の未確定表現（`CW_USEDEFAULT`）の扱いを含めて
/// 規則を 1 箇所に置くためである（素直に `position + size / 2` と書くと窓生成前の窓で
/// 整数桁溢れを起こす）。中心が求まらない窓・`DPI` を持たない窓・表そのものが無い World は
/// いずれも「表を引けない」＝[`Proceed`](DpiSyncDecision::Proceed) へ倒す。
pub(crate) fn evaluate(world: &World, window: Entity, now: u32) -> DpiSyncOutcome {
    let window_dpi = world.get::<DPI>(window).map_or(0, |dpi| dpi.dpi_x as u32);
    let table_dpi = world
        .get::<WindowPos>(window)
        .and_then(window_center)
        .zip(world.get_resource::<MonitorDpiTable>())
        .and_then(|((cx, cy), table)| table.dpi_for_point(cx, cy));
    let held_since = world
        .get::<DpiSyncHold>(window)
        .map(|hold| hold.since_frame);
    DpiSyncOutcome {
        decision: dpi_sync_decision(window_dpi, table_dpi, held_since, now),
        window_dpi,
        table_dpi,
        since_frame: held_since.unwrap_or(now),
    }
}

/// 拡大率の相のゲート適用——判定し、札を付け外しし、記録を 1 行出す。
///
/// 戻り値は**当該窓の処理を進めてよいか**（`false`＝本フレームは窓書込も再導出も行わない）。
/// 札の付け外しを行うのは本関数だけである（設計 C5「解除は dpi 相が一元的に行う」）。
///
/// 上限超過は `warn!` を出したうえで**進む**——待ち続けると窓が旧寸のまま取り残されるので、
/// 現在の源で 1 回書く方が安全である（ログ無し失敗経路の禁止）。
pub(crate) fn apply_dpi_phase_gate(world: &mut World, window: Entity, now: u32) -> bool {
    let outcome = evaluate(world, window, now);
    match outcome.decision {
        DpiSyncDecision::Hold => {
            if world.get::<DpiSyncHold>(window).is_none() {
                world.entity_mut(window).insert(DpiSyncHold {
                    since_frame: outcome.since_frame,
                });
            }
        }
        DpiSyncDecision::ProceedAfterTimeout => {
            warn!(
                entity = ?window,
                window_dpi = outcome.window_dpi,
                table_dpi = ?outcome.table_dpi,
                since_frame = outcome.since_frame,
                waited = now.wrapping_sub(outcome.since_frame),
                max_frames = DPI_SYNC_HOLD_MAX_FRAMES,
                "dpi sync: 窓の拡大率とモニタ表が上限フレーム揃わない → 警告の上で現在の源のまま進む"
            );
            world.entity_mut(window).remove::<DpiSyncHold>();
        }
        DpiSyncDecision::Proceed => {
            if world.get::<DpiSyncHold>(window).is_some() {
                world.entity_mut(window).remove::<DpiSyncHold>();
            }
        }
    }
    log_hold(world, window, &outcome, HoldSite::Dpi);
    !outcome.decision.is_hold()
}

/// ほかの窓書込点（報告寸の突合・再スナップ）のゲート——**読むだけ**で札は触らない。
///
/// 戻り値は**当該窓の窓書込を見送るか**（`true`＝この点では何も書かない）。見送った要求は
/// 消費せず次フレームへ持ち越す（報告寸は presenter が保持し、再スナップは実表示寸を毎フレーム
/// 読み直す）。解除フレームに拡大率の相が新しい源・新しい寸で 1 本書き、持ち越された要求は
/// べき等 skip で吸収される。
pub(crate) fn defers_window_write(world: &World, window: Entity, site: HoldSite) -> bool {
    let Some(hold) = world.get::<DpiSyncHold>(window) else {
        return false;
    };
    let since_frame = hold.since_frame;
    let outcome = DpiSyncOutcome {
        decision: DpiSyncDecision::Hold,
        window_dpi: world.get::<DPI>(window).map_or(0, |dpi| dpi.dpi_x as u32),
        table_dpi: world
            .get::<WindowPos>(window)
            .and_then(window_center)
            .zip(world.get_resource::<MonitorDpiTable>())
            .and_then(|((cx, cy), table)| table.dpi_for_point(cx, cy)),
        since_frame,
    };
    log_hold(world, window, &outcome, site);
    true
}

/// 整合待ちの記録を 1 行出す（既定 OFF・前置ガードは本関数が持つ）。
///
/// レコードを**組む**より外側でガードするのは、既定運転で確保を 1 バイトも増やさないため
/// である（要件 10.4）。
fn log_hold(world: &World, window: Entity, outcome: &DpiSyncOutcome, site: HoldSite) {
    if !transition_diag::is_enabled() {
        return;
    }
    transition_diag::log_hold(
        world,
        window,
        outcome.window_dpi,
        outcome.table_dpi,
        outcome.since_frame,
        outcome.decision.as_str(),
        site.as_str(),
    );
}

#[cfg(test)]
#[path = "dpi_sync_tests.rs"]
mod dpi_sync_tests;
