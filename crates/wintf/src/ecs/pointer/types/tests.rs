use super::*;

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
