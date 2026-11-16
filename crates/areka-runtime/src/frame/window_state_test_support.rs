//! `window_state_tests.rs` の檻が `areka-runtime` と本体の両方で使う状態と窓と帯の指令の列の組み立て（写しを作らない・spec: runtime-crate-split 設計「実装の中の討議の結果」1）。

use bevy_ecs::world::World;
use std::sync::mpsc::{Sender, channel};

use super::super::test_support::spawn_resnap_windows;
use super::*;

use WindowStateDirective::StayOnTop as Set;

pub const ON: WindowStateDirective = Set { on: true };

pub const OFF: WindowStateDirective = Set { on: false };

/// 受信端を渡した状態と送信端。
pub fn state_with_channel() -> (StayOnTopState, Sender<WindowStateDirective>) {
    let (tx, rx) = channel();
    let state = StayOnTopState {
        rx: Some(rx),
        on: false,
    };
    (state, tx)
}

/// 2 スコープの窓を持つ World と、ゴーストの窓の全部の entity（スコープ順・キャラ→バルーン）。
pub fn world_with_windows() -> (World, Vec<Entity>) {
    let mut world = World::new();
    let gw = spawn_resnap_windows(&mut world);
    let all = gw
        .scopes()
        .flat_map(|s| [gw.char_window(s).unwrap(), gw.balloon_window(s).unwrap()])
        .collect();
    (world, all)
}

/// 期待の帯の指令の列（`Debug` の写し）。
pub fn queue_text(ops: &[(BandOp, &[Entity])]) -> String {
    let mut queue = WindowBandQueue::default();
    for (op, windows) in ops {
        queue.push(*op, windows.to_vec());
    }
    format!("{queue:?}")
}

/// World の帯の指令の列（`Debug` の写し・資源が無ければ `None`）。
pub fn queue_in(world: &World) -> Option<String> {
    world
        .get_resource::<WindowBandQueue>()
        .map(|queue| format!("{queue:?}"))
}

pub fn marked(world: &World, entity: Entity) -> bool {
    world.get::<StayOnTop>(entity).is_some()
}

// 本体のテスト（`crates/areka/src/runtime_tests/`）が使う私有の欄へそのまま渡す口（`test-support` のときだけ・
// 本番の可視性は変えない・spec: runtime-crate-split 設計 D3 ⑷）。
/// [`StayOnTopState`] の常に手前のオンとオフ（`on`）を読む。
#[cfg(feature = "test-support")]
pub fn on(state: &StayOnTopState) -> bool {
    state.on
}

/// [`StayOnTopState`] の常に手前のオンとオフ（`on`）を書く。
#[cfg(feature = "test-support")]
pub fn on_mut(state: &mut StayOnTopState) -> &mut bool {
    &mut state.on
}
