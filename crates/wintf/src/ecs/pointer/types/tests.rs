use super::*;

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
