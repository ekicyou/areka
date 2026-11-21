// -------------------------------------------------------------------------
// clear_balloon_hover_on_leave 排他システム
//
// bare World で `PointerLeave` マーカー保持 entity を組み、(a) バルーン所有 leave のみ hover を
// 解除する（親チェーン→`BalloonWindowMarker`）、(b) 非バルーン窓の leave は無視、(c) マーカー不在は
// 完全 no-op、(d) RuntimeWiring 不在は DEBUG 縮退（hover 不変）を決定的に観測する。判断分岐そのもの
// （active×hit=None×last の全組合せ）は `balloon_pure_core_tests.rs` の `hover_action` のテストが持つ。
// `PointerLeave` の除去は本システムの責務外（FrameFinalize の機構不変）ゆえ検証しない。
// -------------------------------------------------------------------------

use areka_runtime::balloon_hover::BalloonHover;
use bevy_ecs::hierarchy::ChildOf;
use std::cell::RefCell;
use std::rc::Rc;
use wintf::ecs::Window;
use wintf::ecs::pointer::PointerLeave;

use areka_sakura::contract::ActorKey;
use bevy_ecs::world::World;

use super::test_support::{
    capture_logs, empty_runtime, headless_runtime_wiring, insert_wiring,
    runtime_with_active_choice, spawn_balloon_leave_child,
};
use super::*;
use areka_placement::spawn::BalloonWindowMarker;

/// `hover[scope]=Some(k)`（`ordinal=Some`）を仕込んだ `BalloonWiring` を World へ NonSend 挿入する。
fn insert_wiring_with_hover(world: &mut World, scope: usize, ordinal: Option<usize>) {
    let _rx = insert_wiring(world);
    if let Some(o) = ordinal {
        world
            .get_non_send_mut::<BalloonWiring>()
            .unwrap()
            .set_hover(scope, Some(o));
    }
}

/// バルーン所有 leave→hover 解除（Inject(None) アーム）: choice 表示中・現行 hit=None
/// （窓外離脱ゆえエッジ非採取）・前回注入 Some(2) → `inject_choice_hover(actor, None)`＋自前状態
/// `BalloonWiring.hover[scope]` を None 更新＋`choice_hover_inject` を DEBUG 発火する。
#[test]
fn leave_balloon_owned_clears_hover_via_inject_none() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);

    let runtime = runtime_with_active_choice("0");
    assert!(
        runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=true（選択肢スパンあり）"
    );
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(2));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None,
        "バルーン所有 leave で hover[scope] が None へ解除される（Inject(None)）"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_hover_inject")),
        "離脱 hover 解除注入で choice_hover_inject を DEBUG 発火: {logs:?}"
    );
}

/// アンカーだけのバルーンでも、窓の外へ出たら強調を外す注入をする。
/// 選択肢が無いので、選択肢の有無で見ていれば注入の腕に入らない。
#[test]
fn leave_anchor_only_balloon_clears_hover_via_inject_none() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    let runtime = super::test_support::runtime_with_active_anchor("0");
    assert!(
        !runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: 選択肢は無い"
    );
    world.insert_non_send(headless_runtime_wiring(runtime));
    insert_wiring_with_hover(&mut world, 0, Some(0));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None
    );
    assert!(
        logs.iter().any(|l| l.contains("choice_hover_inject")),
        "アンカーの強調も注入で外す: {logs:?}"
    );
}

/// バルーン所有 leave・非表示（ResetOwnState アーム）: choice 非表示・前回注入 Some(3) →
/// 自前状態のみ None 整合し、**inject はしない**（上流原子性が正本＝choice_hover_inject を出さない）。
#[test]
fn leave_balloon_owned_inactive_resets_own_state_without_inject() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);

    let runtime = empty_runtime();
    assert!(
        !runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=false（選択肢スパン無し）"
    );
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(3));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None,
        "消滅時は自前状態のみ None 整合（ResetOwnState・Some(3)→None）"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_hover_inject")),
        "ResetOwnState は inject しない（choice_hover_inject を出さない・上流原子性が正本）: {logs:?}"
    );
}

/// 非バルーン窓の leave は無視（key assertion）: `BalloonWindowMarker` を持たない窓の子が
/// `PointerLeave` を保持しても、選択肢の強調（`BalloonWiring`）も窓上の滞在（`BalloonHover`）も変化しない。
#[test]
fn leave_non_balloon_window_is_ignored() {
    let mut world = World::new();
    // 非バルーン窓（Window だが BalloonWindowMarker 無し）の子へ PointerLeave。
    let win = world.spawn(Window::default()).id();
    world.spawn((PointerLeave, ChildOf(win)));

    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(5));
    world
        .get_non_send_mut::<BalloonHover>()
        .unwrap()
        .set_balloon_hover(0);

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(5),
        "非バルーン窓の leave は hover を触らない（balloon 所有チェックの key assertion）"
    );
    assert!(
        world
            .get_non_send::<BalloonHover>()
            .unwrap()
            .is_balloon_hovered(0),
        "非バルーン窓の離脱は滞在を触らない"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_hover_inject")),
        "非バルーン leave では注入しない: {logs:?}"
    );
}

/// マーカー不在は完全 no-op: `PointerLeave` を一切持たない World では hover を
/// 触らず、RuntimeWiring にも触れずに即 return する（完全 no-op）。
#[test]
fn leave_no_marker_is_full_noop() {
    let mut world = World::new();
    // バルーン窓はあるが PointerLeave マーカーは一切無い。
    world.spawn((BalloonWindowMarker { scope: 0 }, Window::default()));

    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(1));

    clear_balloon_hover_on_leave(&mut world);

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(1),
        "マーカー不在フレームは完全 no-op（hover 不変）"
    );
}

/// RuntimeWiring 不在＝正常縮退: バルーン所有 leave があっても
/// **DEBUG** で no-op・hover 不変（`event=choice_leave_no_runtime`・ERROR は出さない）。
#[test]
fn leave_runtime_absent_degrades_with_debug_and_leaves_hover() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    // RuntimeWiring は挿入しない。BalloonWiring のみ present。
    insert_wiring_with_hover(&mut world, 0, Some(1));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(1),
        "RuntimeWiring 不在では hover を触らず縮退（no-op）"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_leave_no_runtime")),
        "RuntimeWiring 不在は DEBUG で正常縮退（event=choice_leave_no_runtime）: {logs:?}"
    );
    assert!(
        !logs.iter().any(|l| l.contains("level=ERROR")),
        "RuntimeWiring 不在は構成異常ではない＝ERROR を出さない: {logs:?}"
    );
}

/// 文字の層を借りたまま離脱を 1 巡回し、借用の失敗の ERROR を数える。
fn leave_borrow_failures(world: &mut World, runtime: &RefCell<TextLayerRuntime>) -> usize {
    let _held = runtime.borrow_mut();
    capture_logs(|| clear_balloon_hover_on_leave(world))
        .iter()
        .filter(|l| l.contains("level=ERROR") && l.contains("balloon_runtime_borrow_failed"))
        .count()
}

/// 文字の層を借りられないと ERROR を 1 行残して強調を外さず、滞在は消す。
#[test]
fn leave_borrow_failure_is_recorded_and_skips_the_highlight() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(2));
    world
        .get_non_send_mut::<BalloonHover>()
        .unwrap()
        .set_balloon_hover(0);

    assert_eq!(leave_borrow_failures(&mut world, &runtime), 1);
    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(2),
        "強調は外していない"
    );
    assert!(
        !world
            .get_non_send::<BalloonHover>()
            .unwrap()
            .is_balloon_hovered(0)
    );
}

/// 同じバルーン窓の 2 つの entity の離脱は、スコープ 1 つ分として 1 回だけ処理する。
#[test]
fn leave_of_two_entities_in_one_window_is_handled_once() {
    let mut world = World::new();
    let child = spawn_balloon_leave_child(&mut world, 0);
    let win = world.get::<ChildOf>(child).unwrap().parent();
    world.spawn((PointerLeave, ChildOf(win)));
    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(2));

    assert_eq!(leave_borrow_failures(&mut world, &runtime), 1);
}
