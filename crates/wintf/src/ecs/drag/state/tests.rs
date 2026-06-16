use super::*;
use crate::ecs::drag::{WindowDragContext, WindowDragContextResource};

// NOTE: `DRAG_STATE` is a thread_local。テストワーカースレッドは再利用され得るため、
// 各テストは Idle へ強制リセットしてから開始する（前のテストが残した状態への依存を排除）。
// SetCapture/ReleaseCapture は HWND::default()（null）に対し UI スレッド外では実質 no-op の
// ため、状態機械そのものはデバイス非依存に検証できる（capture_guard_panic_safety_test と同じ前提）。

/// thread_local を Idle へ強制的に戻す（CaptureGuard が残っていれば borrow 解放後にドロップ）。
fn force_idle() {
    let _guard = update_drag_state(|state| {
        let old = std::mem::replace(state, DragState::Idle);
        // 旧状態に CaptureGuard があれば取り出して borrow 解放後にドロップさせる
        match old {
            DragState::Preparing { capture_guard, .. }
            | DragState::JustStarted { capture_guard, .. }
            | DragState::Dragging { capture_guard, .. } => Some(capture_guard),
            _ => None,
        }
    });
}

fn null_hwnd() -> HWND {
    HWND::default()
}

fn entity(idx: u32) -> Entity {
    // bevy の World 無しでテスト用 Entity を生成する（index→Entity の決定論的生成）
    Entity::from_raw_u32(idx).expect("valid entity index")
}

/// snapshot のバリアント判別子のみを比較するためのヘルパ。
fn variant_name(s: &DragStateSnapshot) -> &'static str {
    match s {
        DragStateSnapshot::Idle => "Idle",
        DragStateSnapshot::Preparing { .. } => "Preparing",
        DragStateSnapshot::JustStarted { .. } => "JustStarted",
        DragStateSnapshot::Dragging { .. } => "Dragging",
        DragStateSnapshot::JustEnded { .. } => "JustEnded",
    }
}

// --- start_preparing -----------------------------------------------------

/// Idle から start_preparing で Preparing へ遷移し、entity/start_pos を保持する。
#[test]
fn test_start_preparing_from_idle_enters_preparing() {
    force_idle();
    let e = entity(1);
    let pos = PhysicalPoint::new(10, 20);
    start_preparing(e, pos, null_hwnd());

    let snap = snapshot_drag_state();
    match snap {
        DragStateSnapshot::Preparing {
            entity, start_pos, ..
        } => {
            assert_eq!(entity, e);
            assert_eq!(start_pos.x, 10);
            assert_eq!(start_pos.y, 20);
        }
        other => panic!("expected Preparing, got {}", variant_name(&other)),
    }
    force_idle();
}

/// 既に Preparing 中の start_preparing は無視される（複数ボタン同時ドラッグ禁止）。
/// 既存の entity/start_pos が維持されることを確認する。
#[test]
fn test_start_preparing_ignored_when_already_active() {
    force_idle();
    let first = entity(1);
    let second = entity(2);
    start_preparing(first, PhysicalPoint::new(1, 1), null_hwnd());
    // 2 回目（別エンティティ・別座標）は無視されるはず
    start_preparing(second, PhysicalPoint::new(99, 99), null_hwnd());

    let snap = snapshot_drag_state();
    match snap {
        DragStateSnapshot::Preparing {
            entity, start_pos, ..
        } => {
            assert_eq!(entity, first, "最初の press が維持されるべき");
            assert_eq!((start_pos.x, start_pos.y), (1, 1));
        }
        other => panic!("expected Preparing, got {}", variant_name(&other)),
    }
    force_idle();
}

/// JustEnded からの start_preparing は許可される（前ドラッグ終了後の新規ドラッグ）。
#[test]
fn test_start_preparing_allowed_from_just_ended() {
    force_idle();
    let e1 = entity(1);
    start_preparing(e1, PhysicalPoint::new(5, 5), null_hwnd());
    end_dragging(PhysicalPoint::new(5, 5), false); // → JustEnded
    assert!(matches!(
        snapshot_drag_state(),
        DragStateSnapshot::JustEnded { .. }
    ));

    let e2 = entity(2);
    start_preparing(e2, PhysicalPoint::new(7, 8), null_hwnd());
    match snapshot_drag_state() {
        DragStateSnapshot::Preparing { entity, .. } => assert_eq!(entity, e2),
        other => panic!("expected Preparing, got {}", variant_name(&other)),
    }
    force_idle();
}

// --- start_dragging ------------------------------------------------------

/// Preparing → JustStarted 遷移。entity/start_pos は維持、current_pos が反映される。
#[test]
fn test_start_dragging_preparing_to_just_started() {
    force_idle();
    let e = entity(3);
    start_preparing(e, PhysicalPoint::new(10, 10), null_hwnd());
    start_dragging(PhysicalPoint::new(18, 14));

    match snapshot_drag_state() {
        DragStateSnapshot::JustStarted {
            entity,
            start_pos,
            current_pos,
            ..
        } => {
            assert_eq!(entity, e);
            assert_eq!((start_pos.x, start_pos.y), (10, 10));
            assert_eq!((current_pos.x, current_pos.y), (18, 14));
        }
        other => panic!("expected JustStarted, got {}", variant_name(&other)),
    }
    force_idle();
}

/// start_dragging は Preparing 以外では no-op（Idle のまま）。
#[test]
fn test_start_dragging_noop_when_not_preparing() {
    force_idle();
    start_dragging(PhysicalPoint::new(1, 1));
    assert!(
        matches!(snapshot_drag_state(), DragStateSnapshot::Idle),
        "Idle からの start_dragging は何もしないべき"
    );
    force_idle();
}

// --- update_dragging -----------------------------------------------------

/// JustStarted → Dragging 遷移。drag_context=None のとき HWND/位置/move_window/constraint は
/// デフォルト値（null HWND, (0,0), false, None）になる。
#[test]
fn test_update_dragging_just_started_to_dragging_without_context() {
    force_idle();
    let e = entity(4);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    start_dragging(PhysicalPoint::new(6, 6));
    update_dragging(PhysicalPoint::new(6, 6), None);

    match snapshot_drag_state() {
        DragStateSnapshot::Dragging {
            entity,
            current_pos,
            prev_pos,
            move_window,
            constraint,
            initial_window_pos,
            ..
        } => {
            assert_eq!(entity, e);
            assert_eq!((current_pos.x, current_pos.y), (6, 6));
            // 初回 Dragging では prev_pos == current_pos
            assert_eq!((prev_pos.x, prev_pos.y), (6, 6));
            assert!(!move_window, "context 無しでは move_window=false");
            assert!(constraint.is_none());
            assert_eq!((initial_window_pos.x, initial_window_pos.y), (0, 0));
        }
        other => panic!("expected Dragging, got {}", variant_name(&other)),
    }
    force_idle();
}

/// JustStarted → Dragging 遷移で WindowDragContextResource の hwnd/initial_window_pos/
/// move_window/constraint が DragState::Dragging に取り込まれる。
#[test]
fn test_update_dragging_reads_window_drag_context() {
    force_idle();
    let e = entity(5);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    start_dragging(PhysicalPoint::new(3, 3));

    let ctx_res = WindowDragContextResource::new();
    let constraint = DragConstraint {
        min_x: Some(-10),
        max_x: Some(500),
        min_y: None,
        max_y: None,
    };
    ctx_res.set(WindowDragContext {
        hwnd: Some(null_hwnd()),
        initial_window_pos: Some(Point { x: 100, y: 200 }),
        move_window: true,
        constraint: Some(constraint),
    });

    update_dragging(PhysicalPoint::new(3, 3), Some(&ctx_res));

    match snapshot_drag_state() {
        DragStateSnapshot::Dragging {
            move_window,
            initial_window_pos,
            constraint,
            ..
        } => {
            assert!(move_window, "context の move_window=true が反映されるべき");
            assert_eq!((initial_window_pos.x, initial_window_pos.y), (100, 200));
            let c = constraint.expect("constraint が反映されるべき");
            assert_eq!(c.min_x, Some(-10));
            assert_eq!(c.max_x, Some(500));
        }
        other => panic!("expected Dragging, got {}", variant_name(&other)),
    }
    force_idle();
}

/// Dragging → Dragging 更新で current_pos が新値、prev_pos が直前の current_pos になる。
#[test]
fn test_update_dragging_dragging_updates_prev_pos() {
    force_idle();
    let e = entity(6);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    start_dragging(PhysicalPoint::new(10, 10));
    update_dragging(PhysicalPoint::new(10, 10), None); // → Dragging (current=prev=10,10)
    update_dragging(PhysicalPoint::new(25, 30), None); // → current=25,30 / prev=10,10

    match snapshot_drag_state() {
        DragStateSnapshot::Dragging {
            current_pos,
            prev_pos,
            ..
        } => {
            assert_eq!((current_pos.x, current_pos.y), (25, 30));
            assert_eq!(
                (prev_pos.x, prev_pos.y),
                (10, 10),
                "prev_pos は直前の current_pos を保持するべき"
            );
        }
        other => panic!("expected Dragging, got {}", variant_name(&other)),
    }
    force_idle();
}

/// update_dragging は Idle/JustEnded など対象外状態では no-op。
#[test]
fn test_update_dragging_noop_when_idle() {
    force_idle();
    update_dragging(PhysicalPoint::new(5, 5), None);
    assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
    force_idle();
}

// --- end_dragging --------------------------------------------------------

/// Preparing からの end_dragging は JustEnded(cancelled=false) へ。entity を保持。
#[test]
fn test_end_dragging_from_preparing() {
    force_idle();
    let e = entity(7);
    start_preparing(e, PhysicalPoint::new(40, 50), null_hwnd());
    end_dragging(PhysicalPoint::new(41, 52), false);

    match snapshot_drag_state() {
        DragStateSnapshot::JustEnded {
            entity,
            position,
            cancelled,
        } => {
            assert_eq!(entity, e);
            assert_eq!((position.x, position.y), (41, 52));
            assert!(!cancelled);
        }
        other => panic!("expected JustEnded, got {}", variant_name(&other)),
    }
    force_idle();
}

/// Dragging からの end_dragging（cancelled=true 指定）は JustEnded(cancelled=true) へ。
#[test]
fn test_end_dragging_from_dragging_preserves_cancelled_flag() {
    force_idle();
    let e = entity(8);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    start_dragging(PhysicalPoint::new(9, 9));
    update_dragging(PhysicalPoint::new(9, 9), None);
    end_dragging(PhysicalPoint::new(60, 70), true);

    match snapshot_drag_state() {
        DragStateSnapshot::JustEnded {
            entity,
            position,
            cancelled,
        } => {
            assert_eq!(entity, e);
            assert_eq!((position.x, position.y), (60, 70));
            assert!(
                cancelled,
                "end_dragging に渡した cancelled=true が反映されるべき"
            );
        }
        other => panic!("expected JustEnded, got {}", variant_name(&other)),
    }
    force_idle();
}

/// Idle からの end_dragging は no-op（JustEnded を作らない）。
#[test]
fn test_end_dragging_noop_when_idle() {
    force_idle();
    end_dragging(PhysicalPoint::new(1, 1), false);
    assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
    force_idle();
}

// --- cancel_dragging -----------------------------------------------------

/// cancel_dragging は常に JustEnded(cancelled=true, position=start_pos) へ。
#[test]
fn test_cancel_dragging_uses_start_pos_and_sets_cancelled() {
    force_idle();
    let e = entity(9);
    start_preparing(e, PhysicalPoint::new(33, 44), null_hwnd());
    start_dragging(PhysicalPoint::new(80, 90)); // start_pos は 33,44 のまま
    cancel_dragging();

    match snapshot_drag_state() {
        DragStateSnapshot::JustEnded {
            entity,
            position,
            cancelled,
        } => {
            assert_eq!(entity, e);
            assert!(cancelled);
            assert_eq!(
                (position.x, position.y),
                (33, 44),
                "cancel は終了位置に start_pos を使うべき"
            );
        }
        other => panic!("expected JustEnded, got {}", variant_name(&other)),
    }
    force_idle();
}

/// Idle からの cancel_dragging は no-op。
#[test]
fn test_cancel_dragging_noop_when_idle() {
    force_idle();
    cancel_dragging();
    assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
    force_idle();
}

// --- reset_to_idle -------------------------------------------------------

/// reset_to_idle は JustEnded のときのみ Idle に戻す。
#[test]
fn test_reset_to_idle_only_from_just_ended() {
    force_idle();
    let e = entity(10);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    end_dragging(PhysicalPoint::new(0, 0), false); // → JustEnded
    reset_to_idle();
    assert!(matches!(snapshot_drag_state(), DragStateSnapshot::Idle));
    force_idle();
}

/// reset_to_idle は Preparing 等 JustEnded 以外では何もしない。
#[test]
fn test_reset_to_idle_noop_when_preparing() {
    force_idle();
    let e = entity(11);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    reset_to_idle();
    assert!(
        matches!(snapshot_drag_state(), DragStateSnapshot::Preparing { .. }),
        "Preparing は reset_to_idle で変化しないべき"
    );
    force_idle();
}

// --- check_threshold -----------------------------------------------------

/// Preparing 中、ユークリッド距離の二乗が閾値の二乗以上なら true。
#[test]
fn test_check_threshold_true_at_or_beyond_distance() {
    force_idle();
    let e = entity(12);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    // (3,4) は距離 5 = 閾値ちょうど → true（>=）
    assert!(check_threshold(PhysicalPoint::new(3, 4), 5));
    // (10,0) は距離 10 > 5 → true
    assert!(check_threshold(PhysicalPoint::new(10, 0), 5));
    force_idle();
}

/// Preparing 中、距離が閾値未満なら false。
#[test]
fn test_check_threshold_false_below_distance() {
    force_idle();
    let e = entity(13);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    // (3,3) は距離 sqrt(18)≈4.24 < 5 → false
    assert!(!check_threshold(PhysicalPoint::new(3, 3), 5));
    force_idle();
}

/// Preparing 以外の状態では check_threshold は常に false（warn ログのみ）。
#[test]
fn test_check_threshold_false_when_not_preparing() {
    force_idle();
    // Idle 状態
    assert!(!check_threshold(PhysicalPoint::new(100, 100), 1));
    force_idle();
}

/// 負方向の座標差（start_pos > current_pos）でも dx*dx が正に評価され、
/// 距離判定が対称に働く（i32 乗算は符号で結果が変わらないことの特性化）。
#[test]
fn test_check_threshold_symmetric_for_negative_delta() {
    force_idle();
    let e = entity(20);
    // start_pos を正値にし、current_pos を左上へ動かして dx/dy を負にする
    start_preparing(e, PhysicalPoint::new(100, 100), null_hwnd());
    // (97,96) → dx=-3, dy=-4, 距離 5 = 閾値ちょうど → true（>= かつ符号非依存）
    assert!(check_threshold(PhysicalPoint::new(97, 96), 5));
    // (98,98) → dx=-2, dy=-2, 距離 sqrt(8)≈2.83 < 5 → false
    assert!(!check_threshold(PhysicalPoint::new(98, 98), 5));
    force_idle();
}

/// i16 実用座標の極値差（本番入力範囲の上限相当）でも i32 算術が桁あふれせず
/// 正確に評価される安全鎖の特性化。dx=32767, dy=0 → distance_sq=1_073_676_289
/// （< i32::MAX=2_147_483_647）で、閾値 5 を上回り true。
/// （ドキュメントの桁あふれ境界 |dx|>46340 には達しない実用上限を固定）。
#[test]
fn test_check_threshold_i16_extent_delta_no_overflow() {
    force_idle();
    let e = entity(21);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    // i16::MAX 相当の水平デルタ。dx*dx は i32 範囲内で桁あふれせず true。
    assert!(check_threshold(PhysicalPoint::new(32767, 0), 5));
    force_idle();
}

// --- snapshot ------------------------------------------------------------

/// DragState::snapshot が各バリアントを対応する DragStateSnapshot バリアントに写像する。
#[test]
fn test_snapshot_maps_each_variant() {
    force_idle();
    // Idle
    assert_eq!(variant_name(&snapshot_drag_state()), "Idle");

    let e = entity(14);
    // Preparing
    start_preparing(e, PhysicalPoint::new(1, 2), null_hwnd());
    assert_eq!(variant_name(&snapshot_drag_state()), "Preparing");

    // JustStarted
    start_dragging(PhysicalPoint::new(2, 3));
    assert_eq!(variant_name(&snapshot_drag_state()), "JustStarted");

    // Dragging
    update_dragging(PhysicalPoint::new(2, 3), None);
    assert_eq!(variant_name(&snapshot_drag_state()), "Dragging");

    // JustEnded
    end_dragging(PhysicalPoint::new(5, 5), false);
    assert_eq!(variant_name(&snapshot_drag_state()), "JustEnded");

    force_idle();
}

/// read_drag_state クロージャに現在の DragState 参照が渡される。
#[test]
fn test_read_drag_state_observes_current_state() {
    force_idle();
    let observed_idle = read_drag_state(|s| matches!(s, DragState::Idle));
    assert!(observed_idle);

    let e = entity(15);
    start_preparing(e, PhysicalPoint::new(0, 0), null_hwnd());
    let observed_preparing = read_drag_state(|s| matches!(s, DragState::Preparing { .. }));
    assert!(observed_preparing);
    force_idle();
}
