use super::test_support::{
    headless_runtime_wiring, runtime_with_active_choice, spawn_balloon_leave_child,
};
use super::*;
use areka_runtime::balloon_hover::BalloonHover;
use bevy_ecs::world::World;
use std::sync::mpsc;

/// hover 自前追跡: `set_hover`/`hover` で scope 別の last-injected ordinal を
/// 記録・回収でき、消滅時整合の `None` 上書きが反映される。
#[test]
fn hover_tracks_last_injected_ordinal_per_scope() {
    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let mut wiring = BalloonWiring::new(tx);

    assert_eq!(wiring.hover(0), None, "未注入 scope の hover は None");

    wiring.set_hover(0, Some(2));
    wiring.set_hover(1, Some(5));
    assert_eq!(wiring.hover(0), Some(2), "scope 0 の last-injected を回収");
    assert_eq!(wiring.hover(1), Some(5), "scope 1 は独立に保持");

    // 消滅時整合: None 上書きで自前状態を整える。
    wiring.set_hover(0, None);
    assert_eq!(wiring.hover(0), None, "None 上書きが反映される");
    assert_eq!(wiring.hover(1), Some(5), "他 scope は影響を受けない");
}

// -------------------------------------------------------------------------
// post-spawn 装着・NonSend 結線・スケジュール登録
//
// (1) 装着: bare World → spawn_ghost_windows → attach_char_pointer_handlers（キャラ窓の装着を先に済ませる）
//     → attach_balloon_pointer_handlers で、全バルーン窓に OnPointerMoved＋OnPointerPressed が
//     装着され、キャラ窓のハンドラ集合は不変であること。
// (2) NonSend 結線: wire_balloon_choice 後に BalloonWiring・ChoiceSelectionInbox・BalloonHover・
//     ShellBoxHover が NonSend 資源として存在すること。
// (3) スケジュール登録: register_balloon_leave_system が
//     clear_balloon_hover_on_leave を Input スケジュールへ登録すること（wire_balloon_choice は登録しない）——
//     構造（systems_len）と行動（Input 実行で hover 解除）の双方で固定する。
//     登録漏れは高速離脱時の hover 残置として実機目視でしか検出できないため。
// -------------------------------------------------------------------------

use bevy_ecs::schedule::Schedules;
use wintf::ecs::Input;
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed, PointerEventHandler};

use crate::attach_char_pointer_handlers;
use areka_placement::spawn::test_support::{titles, two_scope_placements};
use areka_placement::spawn::{CharWindowMarker, spawn_ghost_windows};

fn balloon_window_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<BalloonWindowMarker>>()
        .iter(world)
        .collect()
}

fn char_window_entities(world: &mut World) -> Vec<Entity> {
    world
        .query_filtered::<Entity, With<CharWindowMarker>>()
        .iter(world)
        .collect()
}

fn has_pointer_handlers(world: &World, e: Entity) -> bool {
    world.get::<OnPointerMoved>(e).is_some() && world.get::<OnPointerPressed>(e).is_some()
}

/// キャラ窓の受け手が `attach_char_pointer_handlers` の付けたもののままか（関数の番地で比べる）。
fn has_char_handlers(world: &World, e: Entity) -> bool {
    world.get::<OnPointerMoved>(e).is_some_and(|h| {
        h.0 as usize == (crate::on_char_pointer_moved as PointerEventHandler) as usize
    }) && world.get::<OnPointerPressed>(e).is_some_and(|h| {
        h.0 as usize == (crate::on_char_pointer_pressed as PointerEventHandler) as usize
    })
}

/// 全バルーン窓へハンドラが付き、キャラ窓のハンドラ集合は変わらない。
#[test]
fn attach_installs_handlers_on_all_balloon_windows_and_leaves_char_unchanged() {
    let mut world = World::new();
    spawn_ghost_windows(&mut world, &two_scope_placements(), &titles());
    // キャラ窓へ先にハンドラを装着する（「変わらない」ことの比較元を空でなくする）。
    attach_char_pointer_handlers(&mut world);

    let balloons = balloon_window_entities(&mut world);
    let chars = char_window_entities(&mut world);
    assert_eq!(
        balloons.len(),
        2,
        "2 スコープぶんのバルーン窓が spawn される"
    );
    assert_eq!(chars.len(), 2, "2 スコープぶんのキャラ窓が spawn される");

    // 装着前: バルーン窓はハンドラ未装着（spawn.rs はバルーンに付けない）。
    for &e in &balloons {
        assert!(
            !has_pointer_handlers(&world, e),
            "attach 前のバルーン窓はポインタハンドラ未装着"
        );
    }
    // キャラ窓は装着済み。
    for &e in &chars {
        assert!(
            has_char_handlers(&world, e),
            "キャラ窓は attach_char_pointer_handlers でハンドラ装着済み"
        );
    }

    attach_balloon_pointer_handlers(&mut world);

    // 装着後: 全バルーン窓に OnPointerMoved＋OnPointerPressed。
    for &e in &balloons {
        assert!(
            has_pointer_handlers(&world, e),
            "attach 後は全バルーン窓に OnPointerMoved＋OnPointerPressed が装着される"
        );
    }
    // キャラ窓のハンドラ集合は不変。
    for &e in &chars {
        assert!(
            has_char_handlers(&world, e),
            "キャラ窓のハンドラは attach_balloon_pointer_handlers で不変"
        );
    }
}

/// wire_balloon_choice が BalloonWiring・ChoiceSelectionInbox・BalloonHover・ShellBoxHover を NonSend 挿入。
#[test]
fn wire_inserts_its_non_sends() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    assert!(
        world.get_non_send::<BalloonWiring>().is_none(),
        "挿入前は BalloonWiring 不在"
    );
    assert!(
        world.get_non_send::<ChoiceSelectionInbox>().is_none(),
        "挿入前は ChoiceSelectionInbox 不在"
    );

    wire_balloon_choice(&mut world);

    assert!(
        world.get_non_send::<BalloonWiring>().is_some(),
        "wire 後は BalloonWiring が NonSend 挿入されている"
    );
    assert!(
        world.get_non_send::<ChoiceSelectionInbox>().is_some(),
        "wire 後は ChoiceSelectionInbox が NonSend 挿入されている（発行の受け口）"
    );
    assert!(
        world.get_non_send::<BalloonHover>().is_some(),
        "wire 後は BalloonHover が NonSend 挿入されている"
    );
    assert!(
        world
            .get_non_send::<crate::shell_box::ShellBoxHover>()
            .is_some(),
        "wire 後は ShellBoxHover が NonSend 挿入されている"
    );
    assert!(
        !world.resource::<Schedules>().contains(Input),
        "wire は系を登録しない（登録は `register_balloon_leave_system`）"
    );
}

/// 登録専用の関数は単独で呼べ、離脱の系をちょうど 1 つ足し、持ち物は置かない。
#[test]
fn register_leave_system_alone_adds_one_system_without_wiring() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    register_balloon_leave_system(&mut world);

    let input = world
        .resource::<Schedules>()
        .get(Input)
        .map_or(0, |s| s.systems_len());
    assert_eq!(input, 1, "離脱の系の 1 本だけが載る");
    assert!(
        world.get_non_send::<BalloonWiring>().is_none(),
        "登録は持ち物を置かない"
    );
}

/// 登録済みシステムが Input 実行で走り、
/// バルーン所有 leave の hover を解除する（登録が「clear_balloon_hover_on_leave」であることの行動的証明）。
#[test]
fn registered_leave_system_runs_in_input_schedule_and_clears_balloon_hover() {
    let mut world = World::new();
    world.init_resource::<Schedules>();

    // バルーン所有 leave（scope 0）＋表示中 choice の実 runtime。
    spawn_balloon_leave_child(&mut world, 0);
    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));

    // wire で持ち物を挿入、続けて leave system 登録。
    wire_balloon_choice(&mut world);
    register_balloon_leave_system(&mut world);
    // wire が挿入した BalloonWiring に前回注入値 Some(2) を仕込む（解除対象）。
    world
        .get_non_send_mut::<BalloonWiring>()
        .expect("wire で BalloonWiring 挿入済み")
        .set_hover(0, Some(2));
    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(2),
        "前提: hover[0]=Some(2)"
    );

    // Input スケジュールを実行 → 登録済み clear_balloon_hover_on_leave が走り hover を解除する。
    world.run_schedule(Input);

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None,
        "Input 実行で登録済み clear_balloon_hover_on_leave が走り hover を解除する（登録の行動的証明）"
    );
}
