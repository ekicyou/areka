use super::*;

// ------------------------------------------------------------------
// parse_px（寛容 parse・6.1）: 数値→Some(i32)・非数値/空→None・決定論
// ------------------------------------------------------------------

#[test]
fn parse_px_numeric_string_yields_some() {
    assert_eq!(parse_px("123"), Some(123));
    assert_eq!(parse_px("0"), Some(0));
    assert_eq!(parse_px("-5"), Some(-5));
    // 仮想スクリーン絶対座標は大きな値・負値もある（1.7）。
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

#[test]
fn parse_px_is_deterministic() {
    for s in ["777", "-3", "x", ""] {
        assert_eq!(parse_px(s), parse_px(s), "同一入力→同一出力: {s:?}");
    }
}

// ------------------------------------------------------------------
// char_pos_entries（保存 entries・1.6）: WindowPos X/Y の key/値等価
// ------------------------------------------------------------------

#[test]
fn char_pos_entries_builds_windowpos_x_y_pairs() {
    let entries = char_pos_entries(0, PointPx { x: 1486, y: 353 });
    assert_eq!(
        entries,
        vec![
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X
                },
                "1486".to_string()
            ),
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::Y
                },
                "353".to_string()
            ),
        ]
    );
}

#[test]
fn char_pos_entries_carries_scope_and_negative_values() {
    // スコープ別（1.6）＋負値（仮想デスクトップ左／上のモニタ・1.7）。
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
// balloon_offset_entries（保存 entries・2.5）: BalloonOffset X/Y の key/値等価
// ------------------------------------------------------------------

#[test]
fn balloon_offset_entries_builds_balloonoffset_x_y_pairs() {
    let entries = balloon_offset_entries(0, PointPx { x: -400, y: 0 });
    assert_eq!(
        entries,
        vec![
            (
                PersistKey::BalloonOffset {
                    scope: 0,
                    axis: Axis::X
                },
                "-400".to_string()
            ),
            (
                PersistKey::BalloonOffset {
                    scope: 0,
                    axis: Axis::Y
                },
                "0".to_string()
            ),
        ]
    );
}

#[test]
fn balloon_offset_entries_carries_scope() {
    let entries = balloon_offset_entries(2, PointPx { x: 336, y: 12 });
    assert_eq!(
        entries,
        vec![
            (
                PersistKey::BalloonOffset {
                    scope: 2,
                    axis: Axis::X
                },
                "336".to_string()
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

/// 保存した値が sylphya の寛容 parse で読み戻せること（key/値等価の往復・8.1 の前哨）。
/// entries の String は parse_px でそのまま i32 へ戻る（Display ⇄ parse 対称）。
#[test]
fn entries_values_round_trip_through_parse_px() {
    let pos = PointPx { x: -7, y: 1040 };
    for (_, v) in char_pos_entries(0, pos) {
        assert!(parse_px(&v).is_some(), "保存値 {v:?} は parse_px で読める");
    }
    assert_eq!(parse_px(&char_pos_entries(0, pos)[0].1), Some(pos.x));
    assert_eq!(parse_px(&char_pos_entries(0, pos)[1].1), Some(pos.y));
}
