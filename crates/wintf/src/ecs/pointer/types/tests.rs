use super::*;

#[test]
fn test_pointer_buffer_push() {
    let mut buffer = PointerBuffer::new();
    assert!(buffer.is_empty());

    buffer.push(PositionSample {
        x: 10.0,
        y: 20.0,
        timestamp: Instant::now(),
    });
    assert_eq!(buffer.len(), 1);
    assert!(!buffer.is_empty());
}

#[test]
fn test_pointer_buffer_max_samples() {
    let mut buffer = PointerBuffer::new();
    for i in 0..10 {
        buffer.push(PositionSample {
            x: i as f32,
            y: i as f32,
            timestamp: Instant::now(),
        });
    }
    // MAX_SAMPLES (5) を超えないことを確認
    assert_eq!(buffer.len(), PointerBuffer::MAX_SAMPLES);

    // 最新の値が最後に追加されたものであることを確認
    let latest = buffer.latest().unwrap();
    assert_eq!(latest.x, 9.0);
}

#[test]
fn test_velocity_calculation() {
    let mut buffer = PointerBuffer::new();
    let t1 = Instant::now();
    buffer.push(PositionSample {
        x: 0.0,
        y: 0.0,
        timestamp: t1,
    });

    // 1サンプルでは速度は0
    let (vx, vy) = buffer.calculate_velocity();
    assert_eq!(vx, 0.0);
    assert_eq!(vy, 0.0);
}

#[test]
fn test_button_buffer_state() {
    let mut buffer = ButtonBuffer::default();
    assert!(!buffer.down_received);
    assert!(!buffer.up_received);

    buffer.record_down();
    assert!(buffer.down_received);
    assert!(!buffer.up_received);

    buffer.record_up();
    assert!(buffer.down_received);
    assert!(buffer.up_received);

    buffer.reset();
    assert!(!buffer.down_received);
    assert!(!buffer.up_received);
}

#[test]
fn test_wheel_buffer() {
    let mut buffer = WheelBuffer::default();
    assert_eq!(buffer.vertical, 0);
    assert_eq!(buffer.horizontal, 0);

    buffer.add_vertical(120);
    buffer.add_vertical(120);
    assert_eq!(buffer.vertical, 240);

    buffer.add_horizontal(-60);
    assert_eq!(buffer.horizontal, -60);

    buffer.reset();
    assert_eq!(buffer.vertical, 0);
    assert_eq!(buffer.horizontal, 0);
}

#[test]
fn test_cursor_velocity_new() {
    let v = CursorVelocity::new(3.0, 4.0);
    assert_eq!(v.x, 3.0);
    assert_eq!(v.y, 4.0);
    assert_eq!(v.magnitude, 5.0); // 3-4-5 直角三角形
}

#[test]
fn test_pointer_state_default() {
    let state = PointerState::default();
    assert_eq!(state.client_point, PhysicalPoint::default());
    assert_eq!(state.local_point, PhysicalPoint::default());
    assert!(!state.left_down);
    assert!(!state.right_down);
    assert!(!state.middle_down);
    assert!(!state.xbutton1_down);
    assert!(!state.xbutton2_down);
    assert!(!state.shift_down);
    assert!(!state.ctrl_down);
    assert_eq!(state.double_click, DoubleClick::None);
    assert_eq!(state.wheel, WheelDelta::default());
}

#[test]
fn test_pointer_leave_marker() {
    // PointerLeaveはunitスタイルのマーカーコンポーネント
    let leave1 = PointerLeave;
    let leave2 = PointerLeave;
    assert_eq!(leave1, leave2);
}

#[test]
fn test_window_pointer_tracking_default() {
    let tracking = WindowPointerTracking::default();
    assert!(!tracking.0);

    let tracking_enabled = WindowPointerTracking(true);
    assert!(tracking_enabled.0);
}

#[test]
fn test_physical_point_new() {
    let pt = PhysicalPoint::new(100, 200);
    assert_eq!(pt.x, 100);
    assert_eq!(pt.y, 200);
}

#[test]
fn test_double_click_variants() {
    assert_eq!(DoubleClick::default(), DoubleClick::None);

    // 各バリアントが異なることを確認
    assert_ne!(DoubleClick::Left, DoubleClick::Right);
    assert_ne!(DoubleClick::Middle, DoubleClick::XButton1);
    assert_ne!(DoubleClick::XButton1, DoubleClick::XButton2);
}

#[test]
fn test_wheel_delta_default() {
    let delta = WheelDelta::default();
    assert_eq!(delta.vertical, 0);
    assert_eq!(delta.horizontal, 0);
}

#[test]
fn test_pointer_button_enum() {
    // PointerButtonのHashトレイト実装を確認
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(PointerButton::Left);
    set.insert(PointerButton::Right);
    set.insert(PointerButton::Middle);
    set.insert(PointerButton::XButton1);
    set.insert(PointerButton::XButton2);
    assert_eq!(set.len(), 5);
}

#[test]
fn test_velocity_calculation_two_samples_nonzero() {
    // 2 サンプル間で非ゼロ速度を計算（既存テストは 1 サンプルの 0 ケースのみ）
    let mut buffer = PointerBuffer::new();
    let t0 = Instant::now();
    buffer.push(PositionSample {
        x: 0.0,
        y: 0.0,
        timestamp: t0,
    });
    buffer.push(PositionSample {
        x: 100.0,
        y: 50.0,
        timestamp: t0 + std::time::Duration::from_millis(100),
    });

    // dx=100/0.1s=1000px/s, dy=50/0.1s=500px/s（最新2サンプル間）
    let (vx, vy) = buffer.calculate_velocity();
    assert!((vx - 1000.0).abs() < 1.0, "vx ≈ 1000px/s, got {vx}");
    assert!((vy - 500.0).abs() < 1.0, "vy ≈ 500px/s, got {vy}");
}

#[test]
fn test_velocity_calculation_tiny_dt_guards_to_zero() {
    // dt < 0.0001s（同一タイムスタンプ）では 0 を返す（ゼロ除算/発散ガード）
    let mut buffer = PointerBuffer::new();
    let t = Instant::now();
    buffer.push(PositionSample {
        x: 0.0,
        y: 0.0,
        timestamp: t,
    });
    buffer.push(PositionSample {
        x: 100.0,
        y: 100.0,
        timestamp: t, // 同一タイムスタンプ → dt ≈ 0
    });

    let (vx, vy) = buffer.calculate_velocity();
    assert_eq!(vx, 0.0, "微小 dt では vx=0");
    assert_eq!(vy, 0.0, "微小 dt では vy=0");
}

#[test]
fn test_velocity_uses_latest_two_samples_only() {
    // 3 サンプル以上でも「最新 2 サンプル間」のみで計算する
    let mut buffer = PointerBuffer::new();
    let t0 = Instant::now();
    // 古いサンプル（無関係な大ジャンプ）
    buffer.push(PositionSample {
        x: 0.0,
        y: 0.0,
        timestamp: t0,
    });
    buffer.push(PositionSample {
        x: 1000.0,
        y: 0.0,
        timestamp: t0 + std::time::Duration::from_millis(10),
    });
    // 最新 2 サンプル: 1000→1010 を 0.1s で
    buffer.push(PositionSample {
        x: 1010.0,
        y: 0.0,
        timestamp: t0 + std::time::Duration::from_millis(110),
    });

    let (vx, _vy) = buffer.calculate_velocity();
    // 10px/0.1s = 100px/s（最初の大ジャンプは無視される）
    assert!(
        (vx - 100.0).abs() < 1.0,
        "最新2サンプルのみ: vx ≈ 100, got {vx}"
    );
}

#[test]
fn test_pointer_buffer_clear_resets_to_empty() {
    let mut buffer = PointerBuffer::new();
    buffer.push(PositionSample {
        x: 1.0,
        y: 2.0,
        timestamp: Instant::now(),
    });
    assert!(!buffer.is_empty());

    buffer.clear();
    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);
    // クリア後は速度計算も 0（サンプル < 2）
    assert_eq!(buffer.calculate_velocity(), (0.0, 0.0));
}

#[test]
fn test_pointer_buffer_eviction_keeps_newest() {
    // MAX_SAMPLES 超過で先頭（最古）が捨てられ、最新が保持される
    let mut buffer = PointerBuffer::new();
    let t0 = Instant::now();
    for i in 0..(PointerBuffer::MAX_SAMPLES + 2) {
        buffer.push(PositionSample {
            x: i as f32,
            y: 0.0,
            timestamp: t0,
        });
    }
    assert_eq!(buffer.len(), PointerBuffer::MAX_SAMPLES);
    // 最新は最後に push した値
    assert_eq!(
        buffer.latest().unwrap().x,
        (PointerBuffer::MAX_SAMPLES + 1) as f32
    );
}

#[test]
fn test_wheel_buffer_saturates_at_i16_bounds() {
    // saturating_add により i16 範囲を超えても飽和（オーバーフロー panic/ラップなし）
    let mut buffer = WheelBuffer::default();
    buffer.add_vertical(i16::MAX);
    buffer.add_vertical(i16::MAX);
    assert_eq!(buffer.vertical, i16::MAX, "正方向に飽和");

    buffer.add_horizontal(i16::MIN);
    buffer.add_horizontal(i16::MIN);
    assert_eq!(buffer.horizontal, i16::MIN, "負方向に飽和");
}

#[test]
fn test_cursor_velocity_zero_is_zero_magnitude() {
    // (0,0) の magnitude は 0（sqrt(0)）
    let v = CursorVelocity::new(0.0, 0.0);
    assert_eq!(v.magnitude, 0.0);
    // Default も全成分 0
    let d = CursorVelocity::default();
    assert_eq!((d.x, d.y, d.magnitude), (0.0, 0.0, 0.0));
}

#[test]
fn test_hit_test_placeholder_returns_window_entity() {
    // Phase 1 プレースホルダー: 常にウィンドウエンティティを返す
    let mut world = bevy_ecs::world::World::new();
    let window = world.spawn_empty().id();
    let other = world.spawn_empty().id();

    let hit = hit_test_placeholder(&world, window, (123.0, 456.0));
    assert_eq!(hit, Some(window), "常に window_entity を返す");
    assert_ne!(hit, Some(other), "他エンティティは返さない");
}

#[test]
fn test_hit_test_with_local_coords_passes_through_screen_coords() {
    // Phase 1: スクリーン座標をそのままローカル座標として返す
    let mut world = bevy_ecs::world::World::new();
    let window = world.spawn_empty().id();

    let result = hit_test_with_local_coords(&world, window, 300, 400);
    let (entity, local) = result.expect("Phase 1 は常に Some");
    assert_eq!(entity, window);
    assert_eq!(
        local,
        PhysicalPoint::new(300, 400),
        "ローカル = スクリーン座標"
    );
}
