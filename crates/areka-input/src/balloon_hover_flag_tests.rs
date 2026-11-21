// -------------------------------------------------------------------------
// バルーン滞在フラグ
//
// 「ポインタがバルーン窓の上に居る」を選択肢行の追跡（`BalloonWiring.hover`）とは**独立の軸**として
// 記録することを確かめる。観測点は 2 つ——(a) 記録（`on_balloon_pointer_moved` が選択肢の有無に
// 依らず当該 scope を真にする）、(b) 解除（`clear_balloon_hover_on_leave` が `PointerLeave` で偽へ戻す）。
//
// 記録側と解除側の縮退方向は意図的に非対称である（消えないまま固着する側へ倒さない）:
// 記録は上流資源が揃うときだけ行い（抑止を増やす側は保守的）、解除は上流 runtime の可否に依らず行う
// （抑止を解く側は積極的）。この非対称は `moved_without_runtime_wiring_does_not_record_hover`（moved 側）と
// `leave_clears_balloon_hover_even_without_runtime_wiring`（leave 側）が固定する。
// -------------------------------------------------------------------------

use areka_runtime::balloon_hover::BalloonHover;
use std::rc::Rc;

use bevy_ecs::world::World;

use super::test_support::{
    bubble_move, capture_logs, empty_runtime, headless_runtime_wiring, insert_wiring,
    runtime_with_active_choice, spawn_balloon_leave_child,
};
use super::*;
use areka_placement::spawn::BalloonWindowMarker;
use areka_sakura::ActorKey;

/// World 中の `BalloonHover` へ滞在を照会する短縮形。
fn hovered(world: &World, scope: usize) -> bool {
    world
        .get_non_send::<BalloonHover>()
        .expect("BalloonHover はテストの支えが挿入済み")
        .is_balloon_hovered(scope)
}

/// 記録: 選択肢表示中でも**行に当たっていない**移動で滞在は真になる
/// （headless では `choice_hit_rows` が空＝hit なし）。選択肢の追跡軸とは独立であることの表側。
#[test]
fn moved_records_hover_when_choice_active_but_no_row_hit() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let rt = runtime_with_active_choice("0");
    // 前提を自前で主張する（ヘルパが劣化して choice_active=false・行あり になると、このテストは
    // 隣の `moved_records_hover_per_scope_independently` の重複へ静かに退化する）。
    assert!(
        rt.borrow().choice_active(&ActorKey::from("0")),
        "前提: 選択肢は表示中（choice_active=true）"
    );
    assert!(
        rt.borrow().choice_hit_rows(&ActorKey::from("0")).is_empty(),
        "前提: headless では行ジオメトリが空＝どの座標でも hit なし"
    );
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&rt)));
    let _rx = insert_wiring(&mut world);

    on_balloon_pointer_moved(&mut world, e, e, &bubble_move(10, 20));

    assert!(hovered(&world, 0), "行に当たらない移動でも滞在は真");
}

/// per-scope 独立: 選択肢の無いバルーンでも、別 scope の滞在は互いに影響しない。
#[test]
fn moved_records_hover_per_scope_independently() {
    let mut world = World::new();
    let e0 = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let e1 = world.spawn(BalloonWindowMarker { scope: 1 }).id();
    world.insert_non_send(headless_runtime_wiring(empty_runtime()));
    let _rx = insert_wiring(&mut world);

    on_balloon_pointer_moved(&mut world, e0, e0, &bubble_move(1, 1));
    assert!(hovered(&world, 0), "scope 0 の滞在が真");
    assert!(!hovered(&world, 1), "触れていない scope 1 は偽のまま");

    on_balloon_pointer_moved(&mut world, e1, e1, &bubble_move(2, 2));
    assert!(hovered(&world, 0), "scope 0 の滞在は保たれる");
    assert!(hovered(&world, 1), "scope 1 の滞在も真になる");
}

/// 記録は選択肢スナップショットより前（独立の軸の構造的証跡）: runtime が既に可変借用されていて
/// 選択肢側が縮退（error＋no-op）しても、滞在の記録は成立している。
#[test]
fn moved_records_hover_before_choice_snapshot_degrades() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let rt = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&rt)));
    let _rx = insert_wiring(&mut world);

    // 選択肢側スナップショット（try_borrow）を失敗させる（可変借用を保持したままハンドラを呼ぶ）。
    let guard = rt.borrow_mut();
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &bubble_move(10, 20)),
            "runtime 借用失敗は no-op false"
        );
    });
    drop(guard);

    assert!(
        logs.iter()
            .any(|l| l.contains("level=ERROR") && l.contains("balloon_runtime_borrow_failed")),
        "前提: 選択肢側は借用失敗で縮退している: {logs:?}"
    );
    assert!(
        hovered(&world, 0),
        "選択肢側の縮退は滞在の記録を妨げない（独立の軸）"
    );
}

/// 記録側の縮退方向: 観測基盤（`RuntimeWiring`）が無い間は記録しない
/// ＝抑止を増やす側は保守的に倒す。解除側（`leave_clears_balloon_hover_even_without_runtime_wiring`）と非対称であることが意図。
#[test]
fn moved_without_runtime_wiring_does_not_record_hover() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let _rx = insert_wiring(&mut world); // RuntimeWiring は挿入しない

    on_balloon_pointer_moved(&mut world, e, e, &bubble_move(10, 20));

    assert!(!hovered(&world, 0), "観測基盤が無い間は抑止側へ倒さない");
}

/// 解除側の縮退方向: 観測基盤（`RuntimeWiring`）が無くても解除は行う
/// ＝抑止を解く側は積極的に倒す（滞在フラグが固着して恒久抑止になる経路を作らない）。
#[test]
fn leave_clears_balloon_hover_even_without_runtime_wiring() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    let _rx = insert_wiring(&mut world); // RuntimeWiring は挿入しない
    world
        .get_non_send_mut::<BalloonHover>()
        .unwrap()
        .set_balloon_hover(0);

    clear_balloon_hover_on_leave(&mut world);

    assert!(
        !hovered(&world, 0),
        "観測基盤が無くても滞在は解除する（固着させない）"
    );
}

/// 解除は離脱した scope のみ（per-scope 独立の解除側）。
#[test]
fn leave_clears_only_the_leaving_scope() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    // scope 1 のバルーン窓は存在するが離脱していない。
    world.spawn((
        BalloonWindowMarker { scope: 1 },
        wintf::ecs::Window::default(),
    ));
    world.insert_non_send(headless_runtime_wiring(empty_runtime()));
    let _rx = insert_wiring(&mut world);
    {
        let mut bw = world.get_non_send_mut::<BalloonHover>().unwrap();
        bw.set_balloon_hover(0);
        bw.set_balloon_hover(1);
    }

    clear_balloon_hover_on_leave(&mut world);

    assert!(!hovered(&world, 0), "離脱した scope 0 は偽へ");
    assert!(hovered(&world, 1), "離脱していない scope 1 は真のまま");
}

/// log-first（結線漏れ）: `BalloonWiring` 不在のバルーン所有離脱は、スコープの数に依らず ERROR を
/// 1 行だけ出して panic しない。
#[test]
fn leave_without_balloon_wiring_reports_error() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    spawn_balloon_leave_child(&mut world, 1);
    world.insert_non_send(headless_runtime_wiring(runtime_with_active_choice("0")));

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    let missing = logs
        .iter()
        .filter(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing"))
        .count();
    assert_eq!(missing, 1, "結線漏れは 1 巡に 1 行: {logs:?}");
}
