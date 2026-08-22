// -------------------------------------------------------------------------
// clear_balloon_hover_on_leave 排他システム檻（task 5・design「clear_balloon_hover_on_leave」／
// R1.3/3.4）
//
// bare World で `PointerLeave` マーカー保持 entity を組み、(a) バルーン所有 leave のみ hover を
// 解除する（親チェーン→`BalloonWindowMarker`）、(b) 非バルーン窓の leave は無視、(c) マーカー不在は
// 完全 no-op、(d) Emo2Wiring 不在は DEBUG 縮退（hover 不変）を決定的に檻へ入れる。判断分岐そのもの
// （active×hit=None×last の全組合せ）は hover_action 純関数檻（task 3.2）が網羅済み。`PointerLeave`
// の除去は本システムの責務外（FrameFinalize の機構不変）ゆえ検証しない。
// -------------------------------------------------------------------------

use bevy_ecs::hierarchy::ChildOf;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use wintf::ecs::Window;
use wintf::ecs::pointer::PointerLeave;

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_sakura::contract::ActorKey;
use bevy_ecs::world::World;

use super::test_support::{
    capture_logs, headless_emo2_wiring, runtime_with_active_choice, spawn_balloon_leave_child,
};
use super::*;
use crate::placement::spawn::BalloonWindowMarker;

/// `hover[scope]=Some(k)`（`ordinal=Some`）を仕込んだ `BalloonWiring` を World へ NonSend 挿入する。
fn insert_wiring_with_hover(world: &mut World, scope: usize, ordinal: Option<usize>) {
    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let mut bw = BalloonWiring::new(tx);
    if let Some(o) = ordinal {
        bw.set_hover(scope, Some(o));
    }
    world.insert_non_send(bw);
}

/// バルーン所有 leave→hover 解除（Inject(None) アーム・R1.3）: choice 表示中・現行 hit=None
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
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(2));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None,
        "バルーン所有 leave で hover[scope] が None へ解除される（Inject(None)・R1.3）"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_hover_inject")),
        "離脱 hover 解除注入で choice_hover_inject を DEBUG 発火（DD-CI-7）: {logs:?}"
    );
    assert!(
        runtime.try_borrow_mut().is_ok(),
        "inject 後も runtime に借用/poison を残さない"
    );
}

/// バルーン所有 leave・非表示（ResetOwnState アーム・R3.4）: choice 非表示・前回注入 Some(3) →
/// 自前状態のみ None 整合し、**inject はしない**（上流原子性が正本＝choice_hover_inject を出さない）。
#[test]
fn leave_balloon_owned_inactive_resets_own_state_without_inject() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);

    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    assert!(
        !runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=false（選択肢スパン無し）"
    );
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(3));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None,
        "消滅時は自前状態のみ None 整合（ResetOwnState・Some(3)→None・R3.4）"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_hover_inject")),
        "ResetOwnState は inject しない（choice_hover_inject を出さない・上流原子性が正本）: {logs:?}"
    );
}

/// 非バルーン窓の leave は無視（key assertion）: `BalloonWindowMarker` を持たない窓の子が
/// `PointerLeave` を保持しても、別 scope のバルーン hover は一切変化しない。
#[test]
fn leave_non_balloon_window_is_ignored() {
    let mut world = World::new();
    // 非バルーン窓（Window だが BalloonWindowMarker 無し）の子へ PointerLeave。
    let win = world.spawn(Window::default()).id();
    world.spawn((PointerLeave, ChildOf(win)));

    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(5));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(5),
        "非バルーン窓の leave は hover を触らない（balloon 所有チェックの key assertion）"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_hover_inject")),
        "非バルーン leave では注入しない: {logs:?}"
    );
}

/// マーカー不在は完全 no-op（R1.4 の離脱版）: `PointerLeave` を一切持たない World では hover を
/// 触らず、Emo2Wiring にも触れずに即 return する（design Risks: 完全 no-op）。
#[test]
fn leave_no_marker_is_full_noop() {
    let mut world = World::new();
    // バルーン窓はあるが PointerLeave マーカーは一切無い。
    world.spawn((BalloonWindowMarker { scope: 0 }, Window::default()));

    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&runtime)));
    insert_wiring_with_hover(&mut world, 0, Some(1));

    clear_balloon_hover_on_leave(&mut world);

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(1),
        "マーカー不在フレームは完全 no-op（hover 不変）"
    );
}

/// Emo2Wiring 不在＝正常縮退（R4.1・donor presenter=None 同型）: バルーン所有 leave があっても
/// **DEBUG** で no-op・hover 不変（`event=choice_leave_no_emo2`・ERROR は出さない）。
#[test]
fn leave_emo2_absent_degrades_with_debug_and_leaves_hover() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    // Emo2Wiring は挿入しない。BalloonWiring のみ present。
    insert_wiring_with_hover(&mut world, 0, Some(1));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        Some(1),
        "Emo2Wiring 不在では hover を触らず縮退（no-op）"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_leave_no_emo2")),
        "Emo2Wiring 不在は DEBUG で正常縮退（event=choice_leave_no_emo2）: {logs:?}"
    );
    assert!(
        !logs.iter().any(|l| l.contains("level=ERROR")),
        "Emo2Wiring 不在は構成異常ではない＝ERROR を出さない: {logs:?}"
    );
}
