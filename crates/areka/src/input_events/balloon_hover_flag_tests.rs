// -------------------------------------------------------------------------
// バルーン滞在フラグ檻（areka-P0-balloon-visibility task 2.2・design「hover 観測フラグ
// （input_events/balloon.rs）」・Requirements 5.2/5.5）
//
// 「ポインタがバルーン窓の上に居る」を選択肢行の追跡（`BalloonWiring.hover`）とは**独立の軸**として
// 記録することを檻へ入れる。観測点は 3 つ——(a) 記録（`on_balloon_pointer_moved` が選択肢の有無に
// 依らず当該 scope を真にする）、(b) 解除（`clear_balloon_hover_on_leave` が `PointerLeave` で偽へ戻す）、
// (c) 掃除口（`clear_balloon_hover`＝非表示遷移時にコントローラが呼ぶ・消費は task 4.4）。
//
// 記録側と解除側の縮退方向は意図的に非対称である（Requirement 5.5「消えないまま固着する側へ倒さない」）:
// 記録は上流資源が揃うときだけ行い（抑止を増やす側は保守的）、解除は上流 runtime の可否に依らず行う
// （抑止を解く側は積極的）。この非対称は下 2 本のテスト（moved 側／leave 側）が明示的に固定する。
// -------------------------------------------------------------------------

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use bevy_ecs::world::World;
use wintf::ecs::Point;
use wintf::ecs::pointer::{Phase, PointerState};

use super::*;
use super::test_support::{
    capture_logs, headless_emo2_wiring, runtime_with_active_choice, spawn_balloon_leave_child,
};
use crate::placement::spawn::BalloonWindowMarker;

/// Bubble 相の合成 `PointerState`（client 物理 px・moved ハンドラは他フィールド非参照）。
fn bubble_move(x: i32, y: i32) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        ..Default::default()
    })
}

/// 空の `BalloonWiring` を World へ NonSend 挿入する（`Receiver` は檻の生存期間だけ保持する）。
fn insert_wiring(world: &mut World) -> mpsc::Receiver<ChoiceSelection> {
    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
    world.insert_non_send(BalloonWiring::new(tx));
    rx
}

/// 選択肢スパンを持たない空 runtime（`choice_active==false`）。
fn empty_runtime() -> Rc<RefCell<TextLayerRuntime>> {
    Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )))
}

/// World 中の `BalloonWiring` へ滞在を照会する短縮形。
fn hovered(world: &World, scope: usize) -> bool {
    world
        .get_non_send::<BalloonWiring>()
        .expect("BalloonWiring は檻が挿入済み")
        .is_balloon_hovered(scope)
}

/// 初期状態は全 scope 偽（未観測の scope を真と誤らない）。
#[test]
fn balloon_hover_defaults_to_false_for_every_scope() {
    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let bw = BalloonWiring::new(tx);
    assert!(!bw.is_balloon_hovered(0), "未観測 scope 0 は偽");
    assert!(!bw.is_balloon_hovered(7), "未観測 scope 7 は偽");
}

/// 記録（Requirement 5.2）: 選択肢が**一つも表示されていない**バルーン窓上の移動でも滞在は真になる。
/// 選択肢行の上に居るときだけ記録する実装をここで落とす。
#[test]
fn moved_over_balloon_records_hover_without_any_choice() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let rt = empty_runtime();
    assert!(
        !rt.borrow().choice_active(&ActorKey::from("0")),
        "前提: 選択肢は非表示（choice_active=false）"
    );
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&rt)));
    let _rx = insert_wiring(&mut world);

    assert!(
        !on_balloon_pointer_moved(&mut world, e, e, &bubble_move(10, 20)),
        "moved は常に false（非侵襲）"
    );

    assert!(
        hovered(&world, 0),
        "バルーン窓上の移動は選択肢の有無に依らず滞在を記録する（5.2）"
    );
}

/// 記録（Requirement 5.2）: 選択肢表示中でも**行に当たっていない**移動で滞在は真になる
/// （headless では `choice_hit_rows` が空＝hit なし）。選択肢の追跡軸とは独立であることの表側。
#[test]
fn moved_records_hover_when_choice_active_but_no_row_hit() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let rt = runtime_with_active_choice("0");
    // 前提を自前で主張する（ヘルパが劣化して choice_active=false・行あり になると、この檻は
    // 隣の `moved_over_balloon_records_hover_without_any_choice` の重複へ静かに退化する）。
    assert!(
        rt.borrow().choice_active(&ActorKey::from("0")),
        "前提: 選択肢は表示中（choice_active=true）"
    );
    assert!(
        rt.borrow().choice_hit_rows(&ActorKey::from("0")).is_empty(),
        "前提: headless では行ジオメトリが空＝どの座標でも hit なし"
    );
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&rt)));
    let _rx = insert_wiring(&mut world);

    on_balloon_pointer_moved(&mut world, e, e, &bubble_move(10, 20));

    assert!(hovered(&world, 0), "行に当たらない移動でも滞在は真（5.2）");
    assert_eq!(
        world
            .get_non_send::<BalloonWiring>()
            .unwrap()
            .hover(0),
        None,
        "滞在の記録は選択肢 hover ordinal を書き換えない（独立の軸）"
    );
}

/// 独立の軸（design「既存の選択肢 hover 機構とは独立の軸として持つ」）: 一方の更新が他方へ漏れない。
/// 選択肢 hover を滞在フラグへ畳み込んだ実装（どちらか一方の map で両方を表す）をここで落とす。
#[test]
fn choice_hover_ordinal_and_balloon_hover_are_independent_axes() {
    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let mut bw = BalloonWiring::new(tx);

    bw.set_hover(0, Some(2));
    assert!(
        !bw.is_balloon_hovered(0),
        "選択肢行の hover 注入は滞在フラグを立てない"
    );

    bw.set_balloon_hover(0);
    assert_eq!(
        bw.hover(0),
        Some(2),
        "滞在の記録は選択肢 hover ordinal を消さない"
    );

    bw.clear_balloon_hover(0);
    assert_eq!(
        bw.hover(0),
        Some(2),
        "滞在の解除は選択肢 hover ordinal を消さない"
    );
}

/// per-scope 独立（Requirement 5.2 は「当該 scope」）: 別 scope の滞在は互いに影響しない。
#[test]
fn moved_records_hover_per_scope_independently() {
    let mut world = World::new();
    let e0 = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let e1 = world.spawn(BalloonWindowMarker { scope: 1 }).id();
    world.insert_non_send(headless_emo2_wiring(empty_runtime()));
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
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&rt)));
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

/// 記録側の縮退方向（Requirement 5.5）: 観測基盤（`Emo2Wiring`）が無い間は記録しない
/// ＝抑止を増やす側は保守的に倒す。解除側（次の檻）と非対称であることが意図。
#[test]
fn moved_without_emo2_wiring_does_not_record_hover() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let _rx = insert_wiring(&mut world); // Emo2Wiring は挿入しない

    on_balloon_pointer_moved(&mut world, e, e, &bubble_move(10, 20));

    assert!(
        !hovered(&world, 0),
        "観測基盤が無い間は抑止側へ倒さない（5.5）"
    );
}

/// 解除（Requirement 5.2 後段）: バルーン所有の `PointerLeave` で当該 scope の滞在が偽へ戻る。
#[test]
fn leave_on_balloon_window_clears_balloon_hover() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    world.insert_non_send(headless_emo2_wiring(empty_runtime()));
    let _rx = insert_wiring(&mut world);
    world
        .get_non_send_mut::<BalloonWiring>()
        .unwrap()
        .set_balloon_hover(0);
    assert!(hovered(&world, 0), "前提: 滞在は真");

    clear_balloon_hover_on_leave(&mut world);

    assert!(!hovered(&world, 0), "離脱で滞在は偽へ戻る（5.2）");
}

/// 解除側の縮退方向（Requirement 5.5）: 観測基盤（`Emo2Wiring`）が無くても解除は行う
/// ＝抑止を解く側は積極的に倒す（滞在フラグが固着して恒久抑止になる経路を作らない）。
#[test]
fn leave_clears_balloon_hover_even_without_emo2_wiring() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    let _rx = insert_wiring(&mut world); // Emo2Wiring は挿入しない
    world
        .get_non_send_mut::<BalloonWiring>()
        .unwrap()
        .set_balloon_hover(0);

    clear_balloon_hover_on_leave(&mut world);

    assert!(
        !hovered(&world, 0),
        "観測基盤が無くても滞在は解除する（固着させない・5.5）"
    );
}

/// 対象選別（key assertion）: 非バルーン窓の離脱では滞在を触らない。
#[test]
fn leave_on_non_balloon_window_keeps_balloon_hover() {
    use bevy_ecs::hierarchy::ChildOf;
    use wintf::ecs::Window;
    use wintf::ecs::pointer::PointerLeave;

    let mut world = World::new();
    let win = world.spawn(Window::default()).id(); // BalloonWindowMarker 無し
    world.spawn((PointerLeave, ChildOf(win)));
    world.insert_non_send(headless_emo2_wiring(empty_runtime()));
    let _rx = insert_wiring(&mut world);
    world
        .get_non_send_mut::<BalloonWiring>()
        .unwrap()
        .set_balloon_hover(0);

    clear_balloon_hover_on_leave(&mut world);

    assert!(
        hovered(&world, 0),
        "非バルーン窓の離脱は滞在を触らない（key assertion）"
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
    world.insert_non_send(headless_emo2_wiring(empty_runtime()));
    let _rx = insert_wiring(&mut world);
    {
        let mut bw = world.get_non_send_mut::<BalloonWiring>().unwrap();
        bw.set_balloon_hover(0);
        bw.set_balloon_hover(1);
    }

    clear_balloon_hover_on_leave(&mut world);

    assert!(!hovered(&world, 0), "離脱した scope 0 は偽へ");
    assert!(hovered(&world, 1), "離脱していない scope 1 は真のまま");
}

/// 掃除口（Requirement 5.5・消費は task 4.4）: 非表示遷移で呼ぶ `clear_balloon_hover` は当該 scope の
/// みを偽にし、再呼出でも壊れない（不可視中は `PointerLeave` が届かないための明示掃除）。
#[test]
fn clear_balloon_hover_clears_that_scope_only_and_is_idempotent() {
    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let mut bw = BalloonWiring::new(tx);
    bw.set_balloon_hover(0);
    bw.set_balloon_hover(1);

    bw.clear_balloon_hover(0);
    assert!(!bw.is_balloon_hovered(0), "掃除した scope 0 は偽");
    assert!(bw.is_balloon_hovered(1), "他 scope は影響を受けない");

    bw.clear_balloon_hover(0);
    assert!(!bw.is_balloon_hovered(0), "再呼出でも偽のまま（冪等）");

    // 掃除後に再びバルーン窓上へ入れば再び真になる（掃除は恒久禁止ではない）。
    bw.set_balloon_hover(0);
    assert!(bw.is_balloon_hovered(0), "掃除後の再滞在は再び真");
}

/// log-first（結線漏れ）: `BalloonWiring` 不在のバルーン所有離脱は ERROR で報告し panic しない。
#[test]
fn leave_without_balloon_wiring_reports_error() {
    let mut world = World::new();
    spawn_balloon_leave_child(&mut world, 0);
    // BalloonWiring も Emo2Wiring も挿入しない。

    let logs = capture_logs(|| clear_balloon_hover_on_leave(&mut world));

    assert!(
        logs.iter()
            .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
        "結線漏れは ERROR で報告する（silent failure 禁止）: {logs:?}"
    );
}
