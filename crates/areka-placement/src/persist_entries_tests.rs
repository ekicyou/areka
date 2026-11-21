use super::*;

// ------------------------------------------------------------------
// parse_px（寛容 parse）: 数値→Some(i32)・非数値/空→None・決定論
// ------------------------------------------------------------------

#[test]
fn parse_px_numeric_string_yields_some() {
    assert_eq!(parse_px("123"), Some(123));
    assert_eq!(parse_px("0"), Some(0));
    assert_eq!(parse_px("-5"), Some(-5));
    // 仮想スクリーン絶対座標は大きな値・負値もある。
    assert_eq!(parse_px("1920"), Some(1920));
    assert_eq!(parse_px("-1080"), Some(-1080));
}

#[test]
fn parse_px_non_numeric_or_empty_yields_none() {
    assert_eq!(parse_px(""), None, "空文字列は値なし");
    assert_eq!(parse_px("abc"), None, "非数値は値なし");
    assert_eq!(parse_px("12.5"), None, "小数は i32 でないため値なし");
    assert_eq!(parse_px("9999999999999999999"), None, "i32 溢れは値なし");
    assert_eq!(parse_px("   "), None, "空白のみは値なし");
}

#[test]
fn parse_px_tolerates_surrounding_whitespace() {
    assert_eq!(parse_px(" 42 "), Some(42));
}

// ------------------------------------------------------------------
// char_pos_entries（保存 entries）: WindowPos X/Y の key/値等価
// ------------------------------------------------------------------

#[test]
fn char_pos_entries_carries_scope_and_negative_values() {
    // スコープ別＋負値（仮想デスクトップ左／上のモニタ）。
    let entries = char_pos_entries(1, PointPx { x: -400, y: -20 });
    assert_eq!(
        entries,
        vec![
            (
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::X
                },
                "-400".to_string()
            ),
            (
                PersistKey::WindowPos {
                    scope: 1,
                    axis: Axis::Y
                },
                "-20".to_string()
            ),
        ]
    );
}

// ------------------------------------------------------------------
// balloon_offset_entries（保存 entries）: BalloonOffset X/Y の key/値等価
// ------------------------------------------------------------------

#[test]
fn balloon_offset_entries_carries_scope() {
    let entries = balloon_offset_entries(2, PointPx { x: -400, y: 12 });
    assert_eq!(
        entries,
        vec![
            (
                PersistKey::BalloonOffset {
                    scope: 2,
                    axis: Axis::X
                },
                "-400".to_string()
            ),
            (
                PersistKey::BalloonOffset {
                    scope: 2,
                    axis: Axis::Y
                },
                "12".to_string()
            ),
        ]
    );
}
