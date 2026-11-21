//! 武装の後に台本が置いた窓の記録の決定論テスト。
//!
//! 記録に積まれるのは武装中の `ScriptPlaced` の差し込み（付け直しを含む）と受け口
//! [`note_script_moved`] の呼び出しだけで、同じ窓は 1 つだけ入る。武装の前・解決の後・
//! 武装していないあいだは積まれない。

use bevy_ecs::prelude::*;

use super::{ChainRealignPending, arm_chain_realign, note_script_moved, realign_chain_once_with};
use crate::chain_finalize::{ChainFinalized, ScriptPlaced};

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
fn insertions_and_calls_while_armed_are_recorded_once_per_window() {
    let (mut world, a, b) = rig();
    arm_chain_realign(&mut world, Vec::new());

    world.entity_mut(a).insert(ScriptPlaced);
    // 同じ窓へ付け直しても（`\![move]` は動かすたびに差し込む）記録は 1 つのまま増えない。
    world.entity_mut(a).insert(ScriptPlaced);
    note_script_moved(&mut world, b);
    note_script_moved(&mut world, b);

    assert_eq!(recorded(&world), Some(vec![a, b]));
}

/// 武装の前から印を持つ窓へ、武装の後に台本がもう一度置いたときも記録する（付け直しで差し込みの口が走る）。
#[test]
fn reinsertion_after_arming_on_a_window_marked_before_is_recorded() {
    let (mut world, a, _) = rig();
    world.entity_mut(a).insert(ScriptPlaced);
    arm_chain_realign(&mut world, Vec::new());

    world.entity_mut(a).insert(ScriptPlaced);

    assert_eq!(recorded(&world), Some(vec![a]));
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
    arm_chain_realign(&mut world, Vec::new());
    assert_eq!(recorded(&world), None, "起動の確定の前は武装しない");
    let a = world.spawn(ScriptPlaced).id();
    world.entity_mut(a).insert(ScriptPlaced);
    note_script_moved(&mut world, a);
    assert_eq!(recorded(&world), None);

    let (mut world, a, b) = rig();
    world.entity_mut(a).insert(ScriptPlaced);
    note_script_moved(&mut world, b);
    assert_eq!(recorded(&world), None);
}

/// 詰め直したあとも、本体のバルーンは押し込みの位置（作業領域の右端の内側）のまま。判定し直したか
/// どうかはここでは見ない（系の檻が見る）。
#[test]
fn realign_keeps_the_balloon_at_the_push_position_after_repacking() {
    use wintf::ecs::{Visual, WindowPos};

    use crate::chain_finalize::{ChainRect, ScopeChainState};
    use crate::follow::MonitorSnapshot;
    use crate::resolver::{PointPx, RectPx, SizePx};
    use crate::shared_test_support::placement_sized;
    use crate::source::GhostTitles;
    use crate::spawn::spawn_ghost_windows;
    use crate::test_support::give_fake_handles;

    let (mut world, _, _) = rig();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        }],
    });
    // 今の位置（拡大率の後・鎖から離れた）と幅。鎖の頭（scope 0）は動かない。
    let now = [(0, 1700, 220), (1, 1000, 200), (2, 100, 200)];
    let placements: Vec<_> = now
        .iter()
        .map(|&(scope, x, w)| {
            placement_sized(
                scope,
                PointPx { x, y: 500 },
                PointPx { x: w, y: 0 },
                true,
                SizePx { w, h: 400 },
                SizePx { w: 300, h: 200 },
            )
        })
        .collect();
    let gw = spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles(now.iter().map(|&(s, _, _)| (s, format!("s{s}")))),
    );
    give_fake_handles(&mut world, &gw);
    let chars: Vec<Entity> = (0..3).map(|s| gw.char_window(s).unwrap()).collect();
    for &c in &chars {
        world.spawn((Visual::default(), ChildOf(c)));
    }
    // 変わる直前は隙間 0 で右から隣り合っていた（付け直す組）。
    let before = [(0, 1700), (1, 1480), (2, 1260)]
        .map(|(s, x)| {
            (
                s,
                ChainRect {
                    x,
                    y: 500,
                    w: 220,
                    h: 400,
                },
            )
        })
        .to_vec();
    arm_chain_realign(&mut world, before);
    assert!(world.get_resource::<ChainRealignPending>().is_some());

    realign_chain_once_with(&mut world, |_| {
        Ok(now
            .iter()
            .zip(&chars)
            .map(|(&(scope, x, width), &c)| {
                (
                    ScopeChainState {
                        scope,
                        current_x: x,
                        width,
                        default_x: None,
                    },
                    (scope, c, PointPx { x, y: 500 }),
                )
            })
            .unzip())
    });

    let pos = |e: Entity| {
        let p = world.get::<WindowPos>(e).unwrap().position.unwrap();
        (p.x, p.y)
    };
    assert_eq!(pos(chars[1]), (1500, 500), "本体は鎖の頭の左へ付け直す");
    assert_eq!(pos(chars[2]), (1300, 500), "相方は本体の左へ付け直す");
    assert_eq!(
        pos(gw.balloon_window(1).unwrap()),
        (1920 - 300, 500),
        "本体のバルーンは押し込みの位置"
    );
}
