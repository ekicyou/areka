//! 武装の後に台本が置いた窓の記録（`dpi-realign-remembered-chain` 要件 3.1・8.2）の決定論テスト。
//!
//! 記録に積まれるのは武装中の `ScriptPlaced` の差し込み（付け直しを含む）と受け口
//! [`note_script_moved`] の呼び出しだけで、武装の前・解決の後・武装していないあいだは積まれない。

use bevy_ecs::prelude::*;

use super::{ChainRealignPending, arm_chain_realign, note_script_moved, realign_chain_once_with};
use crate::placement::chain_finalize::{ChainFinalized, ScriptPlaced};

/// 起動の確定が済んだ素の World（武装できる状態）と窓 2 つ。
fn rig() -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(ChainFinalized);
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    (world, a, b)
}

fn recorded(world: &World) -> Option<Vec<Entity>> {
    world
        .get_resource::<ChainRealignPending>()
        .map(|p| p.script_moved.clone())
}

/// 解決する（判定の入力は空・待ちも無いので最初の呼び出しで武装が解ける）。
fn resolve(world: &mut World) {
    realign_chain_once_with(world, |_| Ok((Vec::new(), Vec::new())));
    assert!(world.get_resource::<ChainRealignPending>().is_none());
}

#[test]
fn insertions_and_calls_while_armed_are_recorded_including_reinsert() {
    let (mut world, a, b) = rig();
    arm_chain_realign(&mut world, Vec::new());

    world.entity_mut(a).insert(ScriptPlaced);
    // 同じ窓へ付け直しても（`\![move]` は動かすたびに差し込む）もう一度積まれる。
    world.entity_mut(a).insert(ScriptPlaced);
    note_script_moved(&mut world, b);

    assert_eq!(recorded(&world), Some(vec![a, a, b]));
}

#[test]
fn insertion_before_arming_is_not_recorded() {
    let (mut world, a, b) = rig();
    world.entity_mut(a).insert(ScriptPlaced);
    note_script_moved(&mut world, b);

    arm_chain_realign(&mut world, Vec::new());

    assert_eq!(
        recorded(&world),
        Some(Vec::new()),
        "武装の前の分は持ち越さない"
    );
}

#[test]
fn nothing_is_recorded_after_resolution() {
    let (mut world, a, b) = rig();
    arm_chain_realign(&mut world, Vec::new());
    world.entity_mut(a).insert(ScriptPlaced);
    resolve(&mut world);

    world.entity_mut(b).insert(ScriptPlaced);
    note_script_moved(&mut world, a);

    assert_eq!(recorded(&world), None, "解決の後に記録の器を作り直さない");

    // 次の武装は空の記録から始まる（前の遷移の分も解決の後の分も持ち越さない）。
    arm_chain_realign(&mut world, Vec::new());
    assert_eq!(recorded(&world), Some(Vec::new()));
}

#[test]
fn nothing_is_recorded_while_unarmed() {
    // 起動の確定が済んでいない World（武装そのものが起きない）でも差し込みと受け口は静かに抜ける。
    let mut world = World::new();
    let a = world.spawn(ScriptPlaced).id();
    world.entity_mut(a).insert(ScriptPlaced);
    note_script_moved(&mut world, a);
    assert_eq!(recorded(&world), None);

    let (mut world, a, b) = rig();
    world.entity_mut(a).insert(ScriptPlaced);
    note_script_moved(&mut world, b);
    assert_eq!(recorded(&world), None);
}
