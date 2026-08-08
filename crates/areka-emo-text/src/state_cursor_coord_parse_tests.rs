use super::*;

// ══ `\_l` 座標語彙のパース（parse_cursor_coord・語彙全形の網羅・タスク 1.1） ══
//
// 不透明転写文字列 → `CursorCoord`。全入力で値を返す純粋・全域関数（パニック／`Result`
// なし）。表現は「後段 `layout.rs::cursor_to_image_px` が bare/em/lh の非負のみ Some・
// %／@／負値／Invalid／Omitted は None」を区別できる語彙に忠実転写する（設計 縮退表・
// R2.1/2.4/6.5）。負の裸数値は Absolute へ忠実転写し（非負ゲートは layout 層の責務）、
// `%` は `unit: Percent` の Absolute、`@` 接頭は Relative variant で保持する。

// ── 空文字列＝当該軸省略（Omitted・正典 R2.4） ──

#[test]
fn parse_empty_is_omitted() {
    assert_eq!(parse_cursor_coord(""), CursorCoord::Omitted);
}

// ── 裸数値＝image px（Absolute Px・R2.1） ──

#[test]
fn parse_bare_number_is_absolute_px() {
    assert_eq!(
        parse_cursor_coord("5"),
        CursorCoord::Absolute {
            value: 5.0,
            unit: CursorUnit::Px
        }
    );
}

#[test]
fn parse_decimal_bare_number_is_absolute_px() {
    assert_eq!(
        parse_cursor_coord("5.0"),
        CursorCoord::Absolute {
            value: 5.0,
            unit: CursorUnit::Px
        }
    );
}

/// 負の裸数値は Absolute へ**忠実転写**する（非負ゲートは layout 層＝
/// `cursor_to_image_px` が None を返す責務。語彙層は負値を Invalid へ写像しない）。
#[test]
fn parse_negative_bare_number_is_absolute_px_preserving_sign() {
    assert_eq!(
        parse_cursor_coord("-3"),
        CursorCoord::Absolute {
            value: -3.0,
            unit: CursorUnit::Px
        }
    );
}

// ── `Nem` / `Nlh`＝Absolute Em/Lh（R2.1） ──

#[test]
fn parse_em_suffix_is_absolute_em() {
    assert_eq!(
        parse_cursor_coord("5em"),
        CursorCoord::Absolute {
            value: 5.0,
            unit: CursorUnit::Em
        }
    );
}

#[test]
fn parse_lh_suffix_is_absolute_lh() {
    assert_eq!(
        parse_cursor_coord("2lh"),
        CursorCoord::Absolute {
            value: 2.0,
            unit: CursorUnit::Lh
        }
    );
}

// ── `N%`＝Absolute Percent（縮退保持: layout が None・R6.5） ──

#[test]
fn parse_percent_suffix_is_absolute_percent() {
    assert_eq!(
        parse_cursor_coord("50%"),
        CursorCoord::Absolute {
            value: 50.0,
            unit: CursorUnit::Percent
        }
    );
}

// ── `@N`＝Relative（`@` 接頭・語彙保持: layout が None・R6.5） ──

#[test]
fn parse_at_prefix_bare_is_relative_px() {
    assert_eq!(
        parse_cursor_coord("@5"),
        CursorCoord::Relative {
            value: 5.0,
            unit: CursorUnit::Px
        }
    );
}

#[test]
fn parse_at_prefix_em_is_relative_em() {
    assert_eq!(
        parse_cursor_coord("@5em"),
        CursorCoord::Relative {
            value: 5.0,
            unit: CursorUnit::Em
        }
    );
}

#[test]
fn parse_at_prefix_percent_is_relative_percent() {
    assert_eq!(
        parse_cursor_coord("@5%"),
        CursorCoord::Relative {
            value: 5.0,
            unit: CursorUnit::Percent
        }
    );
}

/// 語彙全形の網羅（1.3 checklist「CursorCoord の全形」）: Relative × Lh は他の
/// `@` 単位テスト（Px/Em/Percent）で唯一欠けていた組合せ——Absolute×{Px,Em,Lh,Percent}
/// ／Relative×{Px,Em,Lh,Percent} の完全マトリクスを閉じる。
#[test]
fn parse_at_prefix_lh_is_relative_lh() {
    assert_eq!(
        parse_cursor_coord("@2lh"),
        CursorCoord::Relative {
            value: 2.0,
            unit: CursorUnit::Lh
        }
    );
}

/// `@` の負値も語彙は Relative で保持（layout が None＝縮退の判定は下流）。
#[test]
fn parse_at_prefix_negative_is_relative_px_preserving_sign() {
    assert_eq!(
        parse_cursor_coord("@-2"),
        CursorCoord::Relative {
            value: -2.0,
            unit: CursorUnit::Px
        }
    );
}

// ── パース不能＝Invalid（R6.5・状態不変スキップの源） ──

#[test]
fn parse_non_numeric_is_invalid() {
    assert_eq!(parse_cursor_coord("abc"), CursorCoord::Invalid);
}

#[test]
fn parse_bare_unit_without_number_is_invalid() {
    // 数値のない裸単位（"em"／"lh"／"%"）はパース不能。
    assert_eq!(parse_cursor_coord("em"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("lh"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("%"), CursorCoord::Invalid);
}

#[test]
fn parse_lone_at_is_invalid() {
    // `@` のみ（数値本体が空）はパース不能。
    assert_eq!(parse_cursor_coord("@"), CursorCoord::Invalid);
}

#[test]
fn parse_trailing_garbage_is_invalid() {
    // 数値＋未知サフィックス（"5xx"）はパース不能。
    assert_eq!(parse_cursor_coord("5xx"), CursorCoord::Invalid);
}

#[test]
fn parse_at_prefixed_non_numeric_is_invalid() {
    assert_eq!(parse_cursor_coord("@abc"), CursorCoord::Invalid);
}

/// 非有限（NaN／inf）は Invalid へ縮退（layout の換算を汚さない防御）。
#[test]
fn parse_non_finite_is_invalid() {
    assert_eq!(parse_cursor_coord("NaN"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("inf"), CursorCoord::Invalid);
}

/// 全域性: どの `&str` を与えてもパニックせず必ず値を返す（R2.4 決定論・total function）。
#[test]
fn parse_is_total_over_arbitrary_strings() {
    for raw in [
        "", "0", "-0", "5", "-3", "5.0", "5em", "5lh", "50%", "@5", "@5em", "@5%", "@-2",
        "abc", "em", "lh", "%", "@", "5xx", "@abc", "  ", "5 em", "e", "@@5", "1e3", "1.2.3",
    ] {
        // パニックしないことを踏むのが主眼（戻り値の variant は各専用テストで固定）。
        let _ = parse_cursor_coord(raw);
    }
}
