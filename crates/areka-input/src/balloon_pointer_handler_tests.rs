// -------------------------------------------------------------------------
// on_balloon_pointer_moved の配線
//
// 合成 `PointerState` を直接ハンドラへ与え、(a) marker 不在の縮退を、(b) 適用アーム
// （Inject／ResetOwnState／Noop）を実 `TextLayerRuntime` で観測する。判断分岐そのもの
// （active×hit×last の全組合せ）は `balloon_pure_core_tests.rs` の `hover_action` のテストが持つ。
//
// `choice_hit_rows` は `present_frame`（GPU）でしか `choice_snapshot` を埋めないため、
// headless では現行 rows が常に空＝hit=None。ゆえに「Some(ordinal) ハイライト追従」はここでは
// 作れない（GPU を使う `balloon_gpu_hit_tests.rs` が見る）。ここはハンドラ経由の実注入として
// Inject(None) 遷移（active かつ hit=None かつ last=Some → ハイライト解除注入）を観測する
// （`apply_hover` が実 runtime へ注入し、BalloonWiring.hover の更新と debug marker を固定する）。
// -------------------------------------------------------------------------

use std::rc::Rc;
use std::sync::mpsc;

use areka_sakura::contract::ActorKey;
use bevy_ecs::world::World;
use wintf::ecs::Point;
use wintf::ecs::pointer::{Phase, PointerState};

use super::test_support::{
    bubble_move, capture_logs, empty_runtime, headless_runtime_wiring, insert_wiring, left_press,
    runtime_with_active_choice,
};
use super::*;
use areka_placement::spawn::BalloonWindowMarker;

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

/// hover 遷移注入（Inject アーム）: choice 表示中・現行 hit=None（headless snapshot 空）・
/// 前回注入 Some(2) → `Inject(None)`。ハンドラが `apply_hover` で実 runtime へ `None` を
/// 注入し、自前状態 `BalloonWiring.hover[scope]` を None へ更新し、`choice_hover_inject` を DEBUG 発火する。
#[test]
fn moved_active_choice_transition_injects_and_updates_own_state() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    let runtime = runtime_with_active_choice("0");
    assert!(
        runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=true（選択肢スパンあり）"
    );
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));

    // 前回注入値 Some(2) を仕込む（遷移検出のため・現行 hit=None ゆえ Inject(None) へ遷移）。
    let _rx = insert_wiring(&mut world);
    world
        .get_non_send_mut::<BalloonWiring>()
        .unwrap()
        .set_hover(0, Some(2));

    let ev = bubble_move(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "moved は常に false（非侵襲）"
        );
    });

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None,
        "Inject(None) 遷移で BalloonWiring.hover[scope] が None へ更新される"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("level=DEBUG") && l.contains("choice_hover_inject")),
        "hover 遷移注入で choice_hover_inject を DEBUG 発火: {logs:?}"
    );
}

/// 消滅時整合（ResetOwnState アーム）: choice 非表示・前回注入 Some(3) → 自前状態のみ
/// None 整合し、**inject はしない**（上流原子性が正本＝choice_hover_inject を発火しない）。
#[test]
fn moved_inactive_with_prior_injection_resets_own_state_without_inject() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    // 選択肢無しの runtime（choice_active=false）。
    let runtime = empty_runtime();
    assert!(
        !runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=false（選択肢スパン無し）"
    );
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));

    let _rx = insert_wiring(&mut world);
    world
        .get_non_send_mut::<BalloonWiring>()
        .unwrap()
        .set_hover(0, Some(3));

    let ev = bubble_move(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "moved は常に false"
        );
    });

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

/// 完全 no-op（NoopInactive アーム）: choice 非表示・前回注入なし → 自前状態を触らず
/// inject もしない（非表示中は hover 追従なし）。
#[test]
fn moved_inactive_no_prior_injection_is_full_noop() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    let runtime = empty_runtime();
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));

    let _rx = insert_wiring(&mut world); // hover 空（未注入）

    let ev = bubble_move(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_moved(&mut world, e, e, &ev),
            "moved は常に false"
        );
    });

    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None,
        "NoopInactive は自前状態を触らない（未注入のまま None）"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_hover_inject")),
        "NoopInactive は inject しない: {logs:?}"
    );
}

// -------------------------------------------------------------------------
// on_balloon_pointer_pressed の配線
//
// 合成 `PointerState` を直接ハンドラへ与え、(a) 非左押下（右/中 down）素通し、(b) marker 不在の縮退、
// (c) 棄却経路（非表示=inactive／非ヒット=no_hit の reason 弁別・零 send）を実 `TextLayerRuntime` で
// 観測する。確定発行のフィールド一致・消滅時の棄却は `balloon_pure_core_tests.rs` の
// `click_selection` のテストが持つ。
//
// `choice_hit_rows` は `present_frame`（GPU）でしか `choice_snapshot` を埋めないため
// headless では現行 rows が常に空＝hit=None。ゆえに「ヒット→send→choice_selected info」は
// ここでは作れない（GPU を使う `balloon_gpu_hit_tests.rs` が見る）。ここはハンドラ経由の棄却・縮退・零 send を観測する。
// -------------------------------------------------------------------------

/// 非左押下は素通し: 右/中ボタン down（left_down=false）は確定でないため `false`・零 send。
#[test]
fn pressed_non_left_button_is_noop_false() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    // RuntimeWiring/BalloonWiring を present にしても、left_down=false なら手前で false 短絡する。
    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    let rx = insert_wiring(&mut world);

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

/// BalloonWindowMarker 不在（理論上不到達の構成異常）: **ERROR** で no-op・`false`（silent 禁止）。
#[test]
fn pressed_missing_marker_errors_and_noop() {
    let mut world = World::new();
    let e = world.spawn_empty().id(); // BalloonWindowMarker を持たない entity
    let ev = left_press(10, 20);

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

/// 非表示中クリックは棄却: choice_active=false の実 runtime へ左クリック → 発行ゼロ・
/// `debug!(choice_click_rejected, reason="inactive")`。Inbox の try_recv は Empty。
#[test]
fn pressed_inactive_choice_rejected_with_reason_inactive() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

    // 選択肢無しの runtime（choice_active=false）。
    let runtime = empty_runtime();
    assert!(
        !runtime.borrow().choice_active(&ActorKey::from("0")),
        "前提: choice_active=false（選択肢スパン無し）"
    );
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    let rx = insert_wiring(&mut world);

    let ev = left_press(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "非表示中クリックは棄却＝false（非発行）"
        );
    });

    assert!(
        rx.try_recv().is_err(),
        "非表示中クリックは send しない（Inbox は Empty）"
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

/// 表示中・非ヒットは棄却: choice_active=true でも headless では choice_hit_rows が空ゆえ
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
        runtime
            .borrow()
            .choice_hit_rows(&ActorKey::from("0"))
            .is_empty(),
        "前提: headless では choice_hit_rows は空（GPU 未実行）＝常に非ヒット"
    );
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    let rx = insert_wiring(&mut world);

    let ev = left_press(10, 20);
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "非ヒットは棄却＝false（非発行）"
        );
    });

    assert!(
        rx.try_recv().is_err(),
        "非ヒットでは send しない（Inbox は Empty）"
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
// 資源縮退・Tunnel 素通しで副作用が無いこと
//
// 縮退の判断分岐（Tunnel 短絡／RuntimeWiring 不在=debug／BalloonWiring 不在=error）を、観測資源を
// 同居させた副作用ゼロの二点——(a) send なし（`ChoiceSelectionInbox.try_recv()==Empty`）、
// (b) `BalloonWiring.hover` 不変——とあわせて、両ハンドラ×{Tunnel, RuntimeWiring 不在, BalloonWiring 不在} で固定する。
//
// スレッド親和（NonSend 借用は Input スケジュール排他システム内）: ここは各ハンドラを単一スレッドの
// `&mut World` 排他呼出で直接叩く——NonSend 資源（`RuntimeWiring`／`BalloonWiring`・`Rc<RefCell>`）の借用が
// 排他システム内でのみ起きるという構造契約は、ハンドラの `&mut World` シグネチャ自体が強制する
// （別途スレッドテストは不要）。
// -------------------------------------------------------------------------

/// 縮退シナリオ（副作用ゼロを確かめる分類）。
#[derive(Clone, Copy)]
enum Degrade {
    /// Tunnel 相（全資源 present でも即 false・一切触れない）。
    Tunnel,
    /// `RuntimeWiring` 不在（`BalloonWiring` は present）＝debug 正常縮退。
    RuntimeAbsent,
    /// `BalloonWiring` 不在（`RuntimeWiring` は present）＝error 構成異常縮退。
    BalloonWiringAbsent,
}

/// moved: Tunnel／RuntimeWiring 不在／BalloonWiring 不在のいずれも副作用ゼロで縮退する。
///
/// 各シナリオで観測資源（RuntimeWiring の active runtime・hover=Some(2) 仕込みの BalloonWiring・Inbox）を
/// 可能な限り同居させ、(1) 戻り値 false、(2) 対応ログレベル（Tunnel=無縮退ログ・RuntimeWiring 不在=DEBUG・
/// BalloonWiring 不在=ERROR）、(3) 副作用ゼロ（choice_hover_inject なし／hover 不変／Inbox Empty）を固定する。
#[test]
fn moved_tunnel_and_resource_degrade_are_side_effect_free() {
    for scenario in [
        Degrade::Tunnel,
        Degrade::RuntimeAbsent,
        Degrade::BalloonWiringAbsent,
    ] {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        // RuntimeWiring（active choice runtime）は Tunnel／BalloonWiringAbsent で present。
        if !matches!(scenario, Degrade::RuntimeAbsent) {
            world.insert_non_send(headless_runtime_wiring(runtime_with_active_choice("0")));
        }

        // BalloonWiring（hover=Some(2) 仕込み）＋Inbox は Tunnel／RuntimeAbsent で present。
        let inbox_rx = match scenario {
            Degrade::Tunnel | Degrade::RuntimeAbsent => {
                let rx = insert_wiring(&mut world);
                world
                    .get_non_send_mut::<BalloonWiring>()
                    .unwrap()
                    .set_hover(0, Some(2));
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

        // ── (2) 縮退時の対応ログレベル─────────────────────────────────
        match scenario {
            Degrade::Tunnel => assert!(
                !logs
                    .iter()
                    .any(|l| l.contains("choice_hover_inject") || l.contains("level=ERROR")),
                "Tunnel は短絡ゆえ inject も error も出さない: {logs:?}"
            ),
            Degrade::RuntimeAbsent => {
                assert!(
                    logs.iter()
                        .any(|l| l.contains("level=DEBUG") && l.contains("choice_moved_no_runtime")),
                    "RuntimeWiring 不在は DEBUG で正常縮退（choice_moved_no_runtime）: {logs:?}"
                );
                assert!(
                    !logs.iter().any(|l| l.contains("level=ERROR")),
                    "RuntimeWiring 不在は構成異常でない＝ERROR を出さない: {logs:?}"
                );
            }
            Degrade::BalloonWiringAbsent => assert!(
                logs.iter()
                    .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
                "BalloonWiring 不在は ERROR の構成異常（balloon_wiring_missing）: {logs:?}"
            ),
        }

        // ── (3) 副作用ゼロ: runtime へ強調を注入しない（全シナリオ共通）───────────
        assert!(
            !logs.iter().any(|l| l.contains("choice_hover_inject")),
            "縮退では runtime へ hover を注入しない（choice_hover_inject なし）: {logs:?}"
        );

        // ── (3) 副作用ゼロ: BalloonWiring.hover 不変＋send なし（present の場合）─────────────
        if let Some(rx) = &inbox_rx {
            assert_eq!(
                world.get_non_send::<BalloonWiring>().unwrap().hover(0),
                Some(2),
                "縮退では BalloonWiring.hover[scope] を動かさない（仕込み Some(2) 不変）"
            );
            assert!(
                rx.try_recv().is_err(),
                "縮退では ChoiceSelection を発行しない（Inbox は Empty）"
            );
        }
    }
}

/// pressed: Tunnel／RuntimeWiring 不在／BalloonWiring 不在のいずれも副作用ゼロで縮退する。
///
/// moved 版と対称。pressed の副作用は発行（send）ゆえ副作用ゼロ観測は choice_selected info の非発火＋
/// present な Inbox の `try_recv()==Empty`＋BalloonWiring.hover 不変で固定する。
#[test]
fn pressed_tunnel_and_resource_degrade_are_side_effect_free() {
    for scenario in [
        Degrade::Tunnel,
        Degrade::RuntimeAbsent,
        Degrade::BalloonWiringAbsent,
    ] {
        let mut world = World::new();
        let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();

        if !matches!(scenario, Degrade::RuntimeAbsent) {
            world.insert_non_send(headless_runtime_wiring(runtime_with_active_choice("0")));
        }

        let inbox_rx = match scenario {
            Degrade::Tunnel | Degrade::RuntimeAbsent => {
                let rx = insert_wiring(&mut world);
                world
                    .get_non_send_mut::<BalloonWiring>()
                    .unwrap()
                    .set_hover(0, Some(2));
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
            _ => left_press(10, 20),
        };

        let logs = capture_logs(|| {
            assert!(
                !on_balloon_pointer_pressed(&mut world, e, e, &ev),
                "縮退時 pressed は常に false（発行なし）"
            );
        });

        // ── (2) 縮退時の対応ログレベル─────────────────────────────────
        match scenario {
            Degrade::Tunnel => {
                assert!(logs.is_empty(), "Tunnel は短絡ゆえ何も記録しない: {logs:?}")
            }
            Degrade::RuntimeAbsent => {
                assert!(
                    logs.iter()
                        .any(|l| l.contains("level=DEBUG")
                            && l.contains("choice_pressed_no_runtime")),
                    "RuntimeWiring 不在は DEBUG で正常縮退（choice_pressed_no_runtime）: {logs:?}"
                );
                assert!(
                    !logs.iter().any(|l| l.contains("level=ERROR")),
                    "RuntimeWiring 不在は構成異常でない＝ERROR を出さない: {logs:?}"
                );
            }
            Degrade::BalloonWiringAbsent => assert!(
                logs.iter()
                    .any(|l| l.contains("level=ERROR") && l.contains("balloon_wiring_missing")),
                "BalloonWiring 不在は ERROR の構成異常（balloon_wiring_missing）: {logs:?}"
            ),
        }

        // ── (3) 副作用ゼロ: 選択発行 info（choice_selected）を出さない（全シナリオ共通）───────
        assert!(
            !logs.iter().any(|l| l.contains("choice_selected")),
            "縮退では選択を発行しない（choice_selected info なし）: {logs:?}"
        );

        // ── (3) 副作用ゼロ: BalloonWiring.hover 不変＋send なし（present の場合）─────────────
        if let Some(rx) = &inbox_rx {
            assert_eq!(
                world.get_non_send::<BalloonWiring>().unwrap().hover(0),
                Some(2),
                "縮退では BalloonWiring.hover[scope] を動かさない（pressed は元来非更新だが回帰固定）"
            );
            assert!(
                rx.try_recv().is_err(),
                "縮退では ChoiceSelection を発行しない（Inbox は Empty）"
            );
        }
    }
}

/// 押下ハンドラの末尾は、中断の入口（`super::user_break::on_left_press`）を必ず通る。
/// GPU 無しではバルーンを出せないので受理までは進まないが、
/// 左ダブルクリックの検出の記録が出ることで、この 1 行の呼び出しを固定する。
#[test]
fn pressed_left_double_click_reaches_the_user_break_entry() {
    use crate::user_break::UserBreakWiring;
    use wintf::ecs::pointer::DoubleClick;

    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let runtime = empty_runtime();
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    let _selection_rx = insert_wiring(&mut world);
    let (_flag_tx, flag_rx) = mpsc::channel();
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel();
    let (kanade_tx, kanade_rx) = mpsc::channel();
    world.insert_non_send(UserBreakWiring::new(flag_rx, lifecycle_tx, kanade_tx));

    let ev = Phase::Bubble(PointerState {
        client_point: Point { x: 10, y: 20 },
        left_down: true,
        double_click: DoubleClick::Left,
        ..Default::default()
    });
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, e, e, &ev),
            "バルーンが出ていないので中断は受理されず、選択も成立しない＝false"
        );
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("level=TRACE") && l.contains("balloon_break_detected")),
        "押下ハンドラの末尾から中断の入口が呼ばれ、左ダブルクリックの検出が記録される: {logs:?}"
    );
    assert!(
        lifecycle_rx.try_recv().is_err() && kanade_rx.try_recv().is_err(),
        "バルーンが出ていないので、どちらの線へも送らない"
    );
}

// -------------------------------------------------------------------------
// アンカーだけのバルーン
//
// 選択肢が 1 つも無くても、アンカーの範囲があれば移動と押下は「押せる範囲がある」として扱う。
// headless では当たりの行が空なので、見分けは「範囲がある」側の腕に入ったかで付ける
// （移動＝強調を外す注入・押下＝棄却の理由が `no_hit`）。
// -------------------------------------------------------------------------

/// アンカーだけの runtime を載せた World（`BalloonWiring` の前回の強調は `last`）。
fn anchor_only_world(last: Option<usize>) -> (World, Entity, Receiver<ChoiceSelection>) {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let runtime = super::test_support::runtime_with_active_anchor("0");
    let actor = ActorKey::from("0");
    assert!(
        !runtime.borrow().choice_active(&actor) && runtime.borrow().hit_active(&actor),
        "前提: 選択肢は無く、押せる範囲（アンカー）だけがある"
    );
    world.insert_non_send(headless_runtime_wiring(runtime));
    let rx = insert_wiring(&mut world);
    world
        .get_non_send_mut::<BalloonWiring>()
        .unwrap()
        .set_hover(0, last);
    (world, e, rx)
}

/// 強調していたアンカーから外れる移動は、強調を外す注入をする。
#[test]
fn moved_off_an_anchor_injects_the_highlight_off() {
    let (mut world, e, _rx) = anchor_only_world(Some(0));
    let logs = capture_logs(|| {
        on_balloon_pointer_moved(&mut world, e, e, &bubble_move(10, 20));
    });
    assert_eq!(
        world.get_non_send::<BalloonWiring>().unwrap().hover(0),
        None
    );
    assert!(
        logs.iter().any(|l| l.contains("choice_hover_inject")),
        "アンカーだけでも強調の注入の腕に入る: {logs:?}"
    );
}

/// アンカーだけのバルーンの押下は「範囲はあるが当たらなかった」と判定される。
#[test]
fn pressed_on_anchor_only_balloon_is_judged_against_the_ranges() {
    let (mut world, e, rx) = anchor_only_world(None);
    let logs = capture_logs(|| {
        assert!(!on_balloon_pointer_pressed(
            &mut world,
            e,
            e,
            &left_press(10, 20)
        ));
    });
    assert!(rx.try_recv().is_err(), "当たっていないので送らない");
    assert!(
        logs.iter()
            .any(|l| l.contains("choice_click_rejected") && l.contains("no_hit")),
        "棄却の理由は no_hit（inactive ではない）: {logs:?}"
    );
}
