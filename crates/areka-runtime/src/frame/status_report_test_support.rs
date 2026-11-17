//! `status_report_tests.rs` の檻が `areka-runtime` と本体の両方で使う観測の組み立て（写しを作らない・spec: runtime-crate-split 設計「実装の中の討議の結果」1）。

use super::*;

pub fn obs(scope: u32, visible: Option<bool>, surface_id: Option<u32>) -> BalloonObservation {
    BalloonObservation {
        scope,
        visible,
        surface_id,
        box_showing: false,
    }
}

/// 箱に文字が出ている観測（areka-P0-shell-balloon 要件 5.5・5.6）。
pub fn obs_box(scope: u32, visible: Option<bool>, surface_id: Option<u32>) -> BalloonObservation {
    BalloonObservation {
        box_showing: true,
        ..obs(scope, visible, surface_id)
    }
}

pub fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}

// 本体のテスト（`crates/areka/src/runtime_tests/`）が使う私有の関数（中で記録を出す相の部品）と欄へそのまま渡す口（`test-support` のときだけ・
// 本番の可視性は変えない・spec: runtime-crate-split 設計 D3 ⑷）。
#[cfg(feature = "test-support")]
use bevy_ecs::world::World;
#[cfg(feature = "test-support")]
use std::collections::BTreeMap;

/// [`super::report_observed`] へそのまま渡す。
#[cfg(feature = "test-support")]
pub fn report_observed(
    world: &World,
    ledger: &mut BalloonStatusLedger,
    observed: &[BalloonObservation],
) {
    super::report_observed(world, ledger, observed)
}

/// [`BalloonStatusLedger`] の送った組（`last_sent`）。
#[cfg(feature = "test-support")]
pub fn last_sent(ledger: &mut BalloonStatusLedger) -> &mut Vec<BalloonBinding> {
    &mut ledger.last_sent
}

/// [`BalloonStatusLedger`] の送った段（`last_stage`）。
#[cfg(feature = "test-support")]
pub fn last_stage(ledger: &BalloonStatusLedger) -> &BTreeMap<u32, StageTuple> {
    &ledger.last_stage
}

/// [`BalloonStatusLedger`] の実行系の借用の失敗を記録したか（`runtime_busy_logged`）。
#[cfg(feature = "test-support")]
pub fn runtime_busy_logged(ledger: &BalloonStatusLedger) -> bool {
    ledger.runtime_busy_logged
}
