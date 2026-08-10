// -------------------------------------------------------------------------
// on_balloon_pointer_moved 配線層檻（task 4.1・design「配線層 > balloon ハンドラ」／
// Error Handling／System Flows・R1.1/1.2/1.3/1.4/1.6/3.1/3.3/4.1/4.2/8.4）
//
// 合成 `PointerState` を直接ハンドラへ与え、(a) Tunnel 素通し、(b) 資源不在の縮退経路を
// 正しい**ログレベル**（Emo2Wiring 不在=debug／BalloonWiring 不在=error）で、(c) 適用アーム
// （Inject／ResetOwnState／Noop）を実 `TextLayerRuntime` で決定的に檻へ入れる。判断分岐そのもの
// （active×hit×last の全組合せ）は hover_action 純関数檻（task 3.2）が網羅済み。
//
// NOTE（runtime constructibility）: `choice_hit_rows` は `present_frame`（GPU）でしか
// `choice_snapshot` を埋めないため、headless では現行 rows が常に空＝hit=None。ゆえに
// 「Some(ordinal) ハイライト追従」は本層では実演できず hover_action 純関数檻（3.2）＋
// task 7.1 の pass-through 檻に委ねる。本檻はハンドラ経由の実注入として Inject(None) 遷移
// （active かつ hit=None かつ last=Some → ハイライト解除注入）を観測する（inject_choice_hover を
// 実 runtime へ実際に呼び、BalloonWiring.hover の更新と debug marker を固定する）。
// -------------------------------------------------------------------------

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_sakura::contract::ActorKey;
use bevy_ecs::world::World;
use wintf::ecs::Point;
use wintf::ecs::pointer::{Phase, PointerState};

use super::*;
use super::test_support::{capture_logs, headless_emo2_wiring, runtime_with_active_choice};
use crate::placement::spawn::BalloonWindowMarker;

/// Bubble 相の合成 `PointerState`（client 物理 px）を組む（moved ハンドラは double_click/ctrl 非参照）。
fn bubble_move(x: i32, y: i32) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        ..Default::default()
    })
}

/// Tunnel 素通し（R1・donor 同型）: Tunnel 相は資源に一切触れず即 `false`（副作用なし）。
#[test]
fn moved_tunnel_phase_is_noop_false() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    // 資源未挿入でも Tunnel は最初に短絡するため到達しない（副作用なしの担保）。
    let tunnel = Phase::Tunnel(PointerState {
        client_point: Point { x: 10, y: 20 },
        ..Default::default()
    });
    assert!(
        !on_balloon_pointer_moved(&mut world, e, e, &tunnel),
        "Tunnel 相は即 false（伝播続行・非侵襲）"
    );
}

/// Emo2Wiring 不在＝正常縮退（R4.1・donor presenter=None 同型）: **DEBUG** レベルで no-op・`false`。
#[test]
fn moved_emo2_absent_degrades_with_debug_not_error() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let ev = bubble_move(10, 20);

    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "Emo2Wiring 不在は no-op false"
        );
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_moved_no_emo2")),
        "Emo2Wiring 不在は DEBUG で正常縮退する（event=choice_moved_no_emo2）: {logs:?}"
    );
    assert!(
        !logs.iter().any(|l| l.contains("level=ERROR")),
        "Emo2Wiring 不在は構成異常ではない＝ERROR を出さない: {logs:?}"
    );
}

/// BalloonWiring 不在＝構成異常（結線漏れ）: Emo2Wiring は present でも **ERROR** で no-op・`false`。
/// （借用規律の固定順序＝Emo2Wiring→BalloonWiring ゆえ Emo2Wiring present が前提。）
#[test]
fn moved_balloon_wiring_absent_degrades_with_error() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    // Emo2Wiring は present（空 runtime）・BalloonWiring は挿入しない。
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
    let ev = bubble_move(10, 20);

    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "BalloonWiring 不在は no-op false"
        );
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
        "BalloonWiring 不在は ERROR の構成異常（event=balloon_wiring_missing）: {logs:?}"
    );
}

/// BalloonWindowMarker 不在（理論上不到達の構成異常）: **ERROR** で no-op・`false`（silent 禁止）。
#[test]
fn moved_missing_marker_errors_and_noop() {
    let mut world = World::new();
    let e = world.spawn_empty().id(); // BalloonWindowMarker を持たない entity
    let ev = bubble_move(10, 20);

    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "marker 不在は no-op false"
        );
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("level=ERROR") && l.contains("balloon_marker_missing")),
        "marker 不在は ERROR で縮退（silent failure 禁止）: {logs:?}"
    );
}

/// hover 遷移注入（Inject アーム・R1.3）: choice 表示中・現行 hit=None（headless snapshot 空）・
/// 前回注入 Some(2) → `Inject(None)`。ハンドラが実 runtime へ `inject_choice_hover(actor, None)` を
/// 呼び、自前状態 `BalloonWiring.hover[scope]` を None へ更新し、`choice_hover_inject` を DEBUG 発火する。
#[test]
fn moved_active_choice_transition_injects_and_updates_own_state() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    let runtime = runtime_with_active_choice("0");
    assert!(
        runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=true（選択肢スパンあり）"
    );
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

    // 前回注入値 Some(2) を仕込む（遷移検出のため・現行 hit=None ゆえ Inject(None) へ遷移）。
    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let mut bw = BalloonWiring::new(tx);
    bw.set_hover(0, Some(2));
    world.insert_non_send_resource(bw);

    let ev = bubble_move(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "moved は常に false（非侵襲）"
        );
    });

    assert_eq!(
        world
            .get_non_send_resource::<BalloonWiring>()
            .unwrap()
            .hover(0),
        None,
        "Inject(None) 遷移で BalloonWiring.hover[scope] が None へ更新される（⑤）"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_hover_inject")),
        "hover 遷移注入で choice_hover_inject を DEBUG 発火（DD-CI-7）: {logs:?}"
    );
    // 実 runtime へ inject 済みでも借用/poison を残さない（try_borrow_mut が成功する）。
    assert!(
        runtime.try_borrow_mut().is_ok(),
        "inject 後も runtime に借用/poison を残さない"
    );
}

/// 消滅時整合（ResetOwnState アーム・R3.4）: choice 非表示・前回注入 Some(3) → 自前状態のみ
/// None 整合し、**inject はしない**（上流原子性が正本＝choice_hover_inject を発火しない）。
#[test]
fn moved_inactive_with_prior_injection_resets_own_state_without_inject() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    // 選択肢無しの runtime（choice_active=false）。
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    assert!(
        !runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=false（選択肢スパン無し）"
    );
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    let mut bw = BalloonWiring::new(tx);
    bw.set_hover(0, Some(3));
    world.insert_non_send_resource(bw);

    let ev = bubble_move(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "moved は常に false"
        );
    });

    assert_eq!(
        world
            .get_non_send_resource::<BalloonWiring>()
            .unwrap()
            .hover(0),
        None,
        "消滅時は自前状態のみ None 整合（ResetOwnState・Some(3)→None）"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_hover_inject")),
        "ResetOwnState は inject しない（choice_hover_inject を出さない・上流原子性が正本）: {logs:?}"
    );
}

/// 完全 no-op（NoopInactive アーム・R1.4）: choice 非表示・前回注入なし → 自前状態を触らず
/// inject もしない（非表示中は hover 追従なし）。
#[test]
fn moved_inactive_no_prior_injection_is_full_noop() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));

    let (tx, _rx) = mpsc::channel::<ChoiceSelection>();
    world.insert_non_send_resource(BalloonWiring::new(tx)); // hover 空（未注入）

    let ev = bubble_move(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "moved は常に false"
        );
    });

    assert_eq!(
        world
            .get_non_send_resource::<BalloonWiring>()
            .unwrap()
            .hover(0),
        None,
        "NoopInactive は自前状態を触らない（未注入のまま None）"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_hover_inject")),
        "NoopInactive は inject しない: {logs:?}"
    );
}

// -------------------------------------------------------------------------
// on_balloon_pointer_pressed 配線層檻（task 4.2・design「配線層 > balloon ハンドラ」／
// Error Handling／System Flows・R2.1/2.3/2.4/2.5/2.6/3.1/3.2/4.2/5.1/8.4）
//
// 合成 `PointerState` を直接ハンドラへ与え、(a) Tunnel 素通し、(b) 非左押下（右/中 down）素通し、
// (c) 資源不在の縮退経路を正しい**ログレベル**（Emo2Wiring 不在=debug／BalloonWiring 不在=error）で、
// (d) 棄却経路（非表示=inactive／非ヒット=no_hit の reason 弁別・零 send）を実 `TextLayerRuntime` で
// 決定的に檻へ入れる。確定発行のフィールド一致・stale 棄却は click_selection 純関数檻（task 3.3）が
// 網羅済み。
//
// NOTE（runtime constructibility）: `choice_hit_rows` は `present_frame`（GPU）でしか
// `choice_snapshot` を埋めないため headless では現行 rows が常に空＝hit=None。ゆえに
// 「ヒット→send→choice_selected info」の full pass-through は本層では実演できず、design Testing
// Strategy item 6 の設計裁定どおり task 7.1/7.3 の pass-through 檻へ委ねる。send 機構自体は
// task 2.2 の `send_selection`／`ChoiceSelectionInbox` 檻で、`Some` 構成は click_selection 檻
// （3.3）で構造的に網羅済み。本檻はハンドラ経由の棄却・縮退・零 send を観測する。
// -------------------------------------------------------------------------

/// 左シングルクリックの合成 `PointerState`（client 物理 px・left_down=true）を組む。
/// `double_click` フィールドは既定 `None` のまま——押下ハンドラは**参照しない**（DD-CI-9）。
fn bubble_left_press(x: i32, y: i32) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        left_down: true,
        ..Default::default()
    })
}

/// `BalloonWiring` と生存 `Receiver`（零 send 観測用）を組む。
fn wiring_with_inbox() -> (BalloonWiring, Receiver<ChoiceSelection>) {
    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
    (BalloonWiring::new(tx), rx)
}

/// Tunnel 素通し（R5.1・donor 同型）: Tunnel 相は資源に一切触れず即 `false`（副作用なし・零 send）。
#[test]
fn pressed_tunnel_phase_is_noop_false() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    // 資源未挿入でも Tunnel は最初に短絡するため到達しない（副作用なしの担保）。
    let tunnel = Phase::Tunnel(PointerState {
        client_point: Point { x: 10, y: 20 },
        left_down: true,
        ..Default::default()
    });
    assert!(
        !on_balloon_pointer_pressed(&mut world, e, e, &tunnel),
        "Tunnel 相は即 false（伝播続行・非侵襲）"
    );
}

/// 非左押下は素通し（R5.1）: 右/中ボタン down（left_down=false）は確定でないため `false`・零 send。
/// `double_click` を一切参照しないことを、既定 None のまま処理が left_down のみで分岐することで担保する。
#[test]
fn pressed_non_left_button_is_noop_false() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    // Emo2Wiring/BalloonWiring を present にしても、left_down=false なら手前で false 短絡する。
    let runtime = runtime_with_active_choice("0");
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
    let (bw, rx) = wiring_with_inbox();
    world.insert_non_send_resource(bw);

    // 右ボタン down（left_down=false）——確定ではない。
    let right = Phase::Bubble(PointerState {
        client_point: Point { x: 10, y: 20 },
        left_down: false,
        right_down: true,
        ..Default::default()
    });
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &right),
            "非左押下（右 down）は false 素通し"
        );
    });

    assert!(
        rx.try_recv().is_err(),
        "非左押下では send しない（Inbox は Empty）"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_selected")),
        "非左押下では choice_selected を出さない: {logs:?}"
    );
}

/// Emo2Wiring 不在＝正常縮退（R4.1・donor presenter=None 同型）: **DEBUG** で no-op・`false`・零 send。
#[test]
fn pressed_emo2_absent_degrades_with_debug_not_error() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let ev = bubble_left_press(10, 20);

    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "Emo2Wiring 不在は no-op false"
        );
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_pressed_no_emo2")),
        "Emo2Wiring 不在は DEBUG で正常縮退（event=choice_pressed_no_emo2）: {logs:?}"
    );
    assert!(
        !logs.iter().any(|l| l.contains("level=ERROR")),
        "Emo2Wiring 不在は構成異常ではない＝ERROR を出さない: {logs:?}"
    );
}

/// BalloonWiring 不在＝構成異常（結線漏れ）: Emo2Wiring は present でも **ERROR** で no-op・`false`。
/// （借用規律の固定順序＝Emo2Wiring→BalloonWiring ゆえ Emo2Wiring present が前提。）
#[test]
fn pressed_balloon_wiring_absent_degrades_with_error() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
    let ev = bubble_left_press(10, 20);

    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "BalloonWiring 不在は no-op false"
        );
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
        "BalloonWiring 不在は ERROR の構成異常（event=balloon_wiring_missing）: {logs:?}"
    );
}

/// BalloonWindowMarker 不在（理論上不到達の構成異常）: **ERROR** で no-op・`false`（silent 禁止）。
#[test]
fn pressed_missing_marker_errors_and_noop() {
    let mut world = World::new();
    let e = world.spawn_empty().id(); // BalloonWindowMarker を持たない entity
    let ev = bubble_left_press(10, 20);

    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "marker 不在は no-op false"
        );
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("level=ERROR") && l.contains("balloon_marker_missing")),
        "marker 不在は ERROR で縮退（silent failure 禁止）: {logs:?}"
    );
}

/// 非表示中クリックは棄却（R3.1）: choice_active=false の実 runtime へ左クリック → 発行ゼロ・
/// `debug!(choice_click_rejected, reason="inactive")`。Inbox の try_recv は Empty。
#[test]
fn pressed_inactive_choice_rejected_with_reason_inactive() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    // 選択肢無しの runtime（choice_active=false）。
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    assert!(
        !runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=false（選択肢スパン無し）"
    );
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
    let (bw, rx) = wiring_with_inbox();
    world.insert_non_send_resource(bw);

    let ev = bubble_left_press(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "非表示中クリックは棄却＝false（非発行）"
        );
    });

    assert!(
        rx.try_recv().is_err(),
        "非表示中クリックは send しない（Inbox は Empty・R3.1）"
    );
    assert!(
        logs.iter().any(|l| l.contains("level=DEBUG")
            && l.contains("choice_click_rejected")
            && l.contains("inactive")),
        "非表示中は reason=inactive の choice_click_rejected を DEBUG 発火: {logs:?}"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_selected")),
        "棄却では choice_selected を出さない: {logs:?}"
    );
}

/// 表示中・非ヒットは棄却（R2.3）: choice_active=true でも headless では choice_hit_rows が空ゆえ
/// 常に非ヒット → 発行ゼロ・`debug!(choice_click_rejected, reason="no_hit")`。
#[test]
fn pressed_active_non_hit_rejected_with_reason_no_hit() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    let runtime = runtime_with_active_choice("0");
    assert!(
        runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=true（選択肢スパンあり）"
    );
    assert!(
        runtime.borrow().choice_hit_rows(&ActorKey::from("0")).is_empty(),
        "前提: headless では choice_hit_rows は空（GPU 未実行）＝常に非ヒット"
    );
    world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&runtime)));
    let (bw, rx) = wiring_with_inbox();
    world.insert_non_send_resource(bw);

    let ev = bubble_left_press(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "非ヒットは棄却＝false（非発行）"
        );
    });

    assert!(
        rx.try_recv().is_err(),
        "非ヒットでは send しない（Inbox は Empty・R2.3）"
    );
    assert!(
        logs.iter().any(|l| l.contains("level=DEBUG")
            && l.contains("choice_click_rejected")
            && l.contains("no_hit")),
        "表示中・非ヒットは reason=no_hit の choice_click_rejected を DEBUG 発火: {logs:?}"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_selected")),
        "棄却では choice_selected を出さない: {logs:?}"
    );
}

// -------------------------------------------------------------------------
// 資源縮退・Tunnel 素通し 副作用ゼロ回帰檻（task 7.2・design Testing Strategy Integration
// Tests item 3/4・Error Handling／R8.1/8.4）
//
// 判断分岐そのもの（Tunnel 短絡／Emo2Wiring 不在=debug 縮退／BalloonWiring 不在=error 縮退／
// marker 不在=error）の**実行網羅**は task 4.1（moved）／4.2（pressed）の各縮退檻が既に閉じている
// （moved_tunnel_phase_is_noop_false／moved_emo2_absent_degrades_with_debug_not_error／
// moved_balloon_wiring_absent_degrades_with_error ほか・pressed 側も同型）。ただしそれらは
// 「戻り値 false＋ログレベル」までで、**観測資源を同居させた副作用ゼロ三点**——(a) send なし
// （present の `ChoiceSelectionInbox.try_recv()==Empty`）、(b) `BalloonWiring.hover` 不変、
// (c) runtime へ `inject_choice_hover` 未適用（借用/poison を残さず `choice_active` 不変・moved は
// `choice_hover_inject` を出さない）——までは固定していない（Tunnel 檻は資源を一切挿入せず、Emo2 不在
// 檻は BalloonWiring を挿入しないため観測点が無かった）。
//
// 本 7.2 檻はその副作用ゼロ次元を**両ハンドラ×{Tunnel, Emo2Wiring 不在, BalloonWiring 不在}**へ
// first-class に追試する（縮退時に対応ログ＝debug／error が出つつ、いかなる観測可能状態も動かない
// ことを、観測資源を同居させて固定する）。判断分岐の重複網羅ではなく副作用ゼロ観測の追加ゆえ非重複。
//
// 8.4（スレッド親和・NonSend 借用は Input スケジュール排他システム内）: 本檻は各ハンドラを単一スレッドの
// `&mut World` 排他呼出で直接叩く——NonSend 資源（`Emo2Wiring`／`BalloonWiring`・`Rc<RefCell>`）の借用が
// 排他システム内でのみ起きるという構造契約は、ハンドラの `&mut World` シグネチャ自体が強制する
// （別途スレッドテストは不要）。
// -------------------------------------------------------------------------

/// 縮退シナリオ（観測資源を同居させた副作用ゼロ回帰の分類）。
#[derive(Clone, Copy)]
enum Degrade {
    /// Tunnel 相（全資源 present でも即 false・一切触れない）。
    Tunnel,
    /// `Emo2Wiring` 不在（`BalloonWiring` は present）＝debug 正常縮退。
    Emo2Absent,
    /// `BalloonWiring` 不在（`Emo2Wiring` は present）＝error 構成異常縮退。
    BalloonWiringAbsent,
}

/// moved: Tunnel／Emo2Wiring 不在／BalloonWiring 不在のいずれも副作用ゼロで縮退する（8.1/8.4）。
///
/// 各シナリオで観測資源（Emo2Wiring の active runtime・hover=Some(2) 仕込みの BalloonWiring・Inbox）を
/// 可能な限り同居させ、(1) 戻り値 false、(2) 対応ログレベル（Tunnel=無縮退ログ・Emo2 不在=DEBUG・
/// BalloonWiring 不在=ERROR）、(3) 副作用ゼロ三点（choice_hover_inject なし／hover 不変／Inbox Empty／
/// runtime 借用・active 不変）を固定する。判断分岐網羅は 4.1、本檻は副作用ゼロ観測を担う。
#[test]
fn moved_tunnel_and_resource_degrade_are_side_effect_free() {
    for scenario in [Degrade::Tunnel, Degrade::Emo2Absent, Degrade::BalloonWiringAbsent] {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        // Emo2Wiring（active choice runtime）は Tunnel／BalloonWiringAbsent で present。
        let runtime = match scenario {
            Degrade::Tunnel | Degrade::BalloonWiringAbsent => {
                let rt = runtime_with_active_choice("0");
                world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&rt)));
                Some(rt)
            }
            Degrade::Emo2Absent => None,
        };

        // BalloonWiring（hover=Some(2) 仕込み）＋Inbox は Tunnel／Emo2Absent で present。
        let inbox_rx = match scenario {
            Degrade::Tunnel | Degrade::Emo2Absent => {
                let (tx, rx) = mpsc::channel::<ChoiceSelection>();
                let mut bw = BalloonWiring::new(tx);
                bw.set_hover(0, Some(2));
                world.insert_non_send_resource(bw);
                Some(rx)
            }
            Degrade::BalloonWiringAbsent => None,
        };

        // 事象: Tunnel は Tunnel 相（資源 present でも短絡）・それ以外は通常 Bubble move。
        let ev = match scenario {
            Degrade::Tunnel => Phase::Tunnel(PointerState {
                client_point: Point { x: 10, y: 20 },
                ..Default::default()
            }),
            _ => bubble_move(10, 20),
        };

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_moved(&mut world, e, e, &ev),
                "縮退時 moved は常に false（非侵襲）"
            );
        });

        // ── (2) 縮退時の対応ログレベル（8.1 の観測条件）─────────────────────────────────
        match scenario {
            Degrade::Tunnel => assert!(
                !logs
                    .iter()
                    .any(|l| l.contains("choice_hover_inject") || l.contains("level=ERROR")),
                "Tunnel は短絡ゆえ inject も error も出さない: {logs:?}"
            ),
            Degrade::Emo2Absent => {
                assert!(
                    logs.iter()
                        .any(|l| l.contains("level=DEBUG") && l.contains("choice_moved_no_emo2")),
                    "Emo2Wiring 不在は DEBUG で正常縮退（choice_moved_no_emo2）: {logs:?}"
                );
                assert!(
                    !logs.iter().any(|l| l.contains("level=ERROR")),
                    "Emo2Wiring 不在は構成異常でない＝ERROR を出さない: {logs:?}"
                );
            }
            Degrade::BalloonWiringAbsent => assert!(
                logs.iter()
                    .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
                "BalloonWiring 不在は ERROR の構成異常（balloon_wiring_missing）: {logs:?}"
            ),
        }

        // ── (3) 副作用ゼロ①: runtime へ inject_choice_hover 未適用（全シナリオ共通）───────────
        assert!(
            !logs.iter().any(|l| l.contains("choice_hover_inject")),
            "縮退では runtime へ hover を注入しない（choice_hover_inject なし）: {logs:?}"
        );

        // ── (3) 副作用ゼロ②③: BalloonWiring.hover 不変＋send なし（present の場合）─────────────
        if let Some(rx) = &inbox_rx {
            assert_eq!(
                world
                    .get_non_send_resource::<BalloonWiring>()
                    .unwrap()
                    .hover(0),
                Some(2),
                "縮退では BalloonWiring.hover[scope] を動かさない（仕込み Some(2) 不変）"
            );
            assert!(
                rx.try_recv().is_err(),
                "縮退では ChoiceSelection を発行しない（Inbox は Empty）"
            );
        }

        // ── (3) 副作用ゼロ④: runtime 未変更（present の場合）借用/poison を残さず active 不変 ─────
        if let Some(rt) = &runtime {
            assert!(
                rt.try_borrow_mut().is_ok(),
                "縮退では runtime に借用/poison を残さない"
            );
            assert!(
                rt.borrow().choice_active(&ActorKey::from("0")),
                "縮退では runtime の choice 状態を動かさない（active 不変）"
            );
        }
    }
}

/// pressed: Tunnel／Emo2Wiring 不在／BalloonWiring 不在のいずれも副作用ゼロで縮退する（8.1/8.4）。
///
/// moved 版と対称。pressed の副作用は発行（send）ゆえ副作用ゼロ観測は choice_selected info の非発火＋
/// present な Inbox の `try_recv()==Empty`＋BalloonWiring.hover 不変＋runtime 借用/active 不変で固定する。
/// 判断分岐網羅は 4.2、本檻は副作用ゼロ観測を担う。
#[test]
fn pressed_tunnel_and_resource_degrade_are_side_effect_free() {
    for scenario in [Degrade::Tunnel, Degrade::Emo2Absent, Degrade::BalloonWiringAbsent] {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        let runtime = match scenario {
            Degrade::Tunnel | Degrade::BalloonWiringAbsent => {
                let rt = runtime_with_active_choice("0");
                world.insert_non_send_resource(headless_emo2_wiring(Rc::clone(&rt)));
                Some(rt)
            }
            Degrade::Emo2Absent => None,
        };

        let inbox_rx = match scenario {
            Degrade::Tunnel | Degrade::Emo2Absent => {
                let (tx, rx) = mpsc::channel::<ChoiceSelection>();
                let mut bw = BalloonWiring::new(tx);
                bw.set_hover(0, Some(2));
                world.insert_non_send_resource(bw);
                Some(rx)
            }
            Degrade::BalloonWiringAbsent => None,
        };

        // 事象: Tunnel は Tunnel 相（left_down=true でも Tunnel 短絡が先）・それ以外は左シングル押下。
        let ev = match scenario {
            Degrade::Tunnel => Phase::Tunnel(PointerState {
                client_point: Point { x: 10, y: 20 },
                left_down: true,
                ..Default::default()
            }),
            _ => bubble_left_press(10, 20),
        };

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                "縮退時 pressed は常に false（発行なし）"
            );
        });

        // ── (2) 縮退時の対応ログレベル（8.1 の観測条件）─────────────────────────────────
        match scenario {
            Degrade::Tunnel => assert!(
                !logs
                    .iter()
                    .any(|l| l.contains("choice_selected") || l.contains("level=ERROR")),
                "Tunnel は短絡ゆえ choice_selected も error も出さない: {logs:?}"
            ),
            Degrade::Emo2Absent => {
                assert!(
                    logs.iter().any(
                        |l| l.contains("level=DEBUG") && l.contains("choice_pressed_no_emo2")
                    ),
                    "Emo2Wiring 不在は DEBUG で正常縮退（choice_pressed_no_emo2）: {logs:?}"
                );
                assert!(
                    !logs.iter().any(|l| l.contains("level=ERROR")),
                    "Emo2Wiring 不在は構成異常でない＝ERROR を出さない: {logs:?}"
                );
            }
            Degrade::BalloonWiringAbsent => assert!(
                logs.iter()
                    .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
                "BalloonWiring 不在は ERROR の構成異常（balloon_wiring_missing）: {logs:?}"
            ),
        }

        // ── (3) 副作用ゼロ①: 選択発行 info（choice_selected）を出さない（全シナリオ共通）───────
        assert!(
            !logs.iter().any(|l| l.contains("choice_selected")),
            "縮退では選択を発行しない（choice_selected info なし）: {logs:?}"
        );

        // ── (3) 副作用ゼロ②③: BalloonWiring.hover 不変＋send なし（present の場合）─────────────
        if let Some(rx) = &inbox_rx {
            assert_eq!(
                world
                    .get_non_send_resource::<BalloonWiring>()
                    .unwrap()
                    .hover(0),
                Some(2),
                "縮退では BalloonWiring.hover[scope] を動かさない（pressed は元来非更新だが回帰固定）"
            );
            assert!(
                rx.try_recv().is_err(),
                "縮退では ChoiceSelection を発行しない（Inbox は Empty）"
            );
        }

        // ── (3) 副作用ゼロ④: runtime 未変更（present の場合）借用/poison を残さず active 不変 ─────
        if let Some(rt) = &runtime {
            assert!(
                rt.try_borrow_mut().is_ok(),
                "縮退では runtime に借用/poison を残さない"
            );
            assert!(
                rt.borrow().choice_active(&ActorKey::from("0")),
                "縮退では runtime の choice 状態を動かさない（active 不変）"
            );
        }
    }
}
