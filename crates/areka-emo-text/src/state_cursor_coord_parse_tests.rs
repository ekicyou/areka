use super::*;

// ══ `\_l` 座標語彙のパース（parse_cursor_coord・語彙全形の網羅） ══
//
// 不透明転写文字列 → `CursorCoord`。全入力で値を返す純粋・全域関数（パニック／`Result`
// なし）。後段の解決層（`cursor_tag.rs`）が解決表どおりに分岐できる語彙への忠実転写である
// （`areka-P0-cursor-tag-canon` design.md「語彙 `CursorCoord`」・R1.1/4.1/4.2）。負の裸数値は
// Absolute、`%` は `unit: Percent` の Absolute、`@` 接頭は Relative variant で保持する——
// いずれも解決層が「基点＋値×係数」で**実導出**する形であって縮退ではない。`centerx`／
// `centery` は生文字列の小文字完全一致だけを該当させ、**軸の情報は持たない**（軸取り違えの
// 判定は解決層の責務）。

// ── 空文字列＝当該軸省略（Omitted・正典 付録 A「(省略): 移動しない」・R1.6/5.5） ──

#[test]
fn parse_empty_is_omitted() {
    assert_eq!(parse_cursor_coord(""), CursorCoord::Omitted);
}

// ── 裸数値＝image px（Absolute Px・単位の意味は R1.3・基点は文字描画開始点＝R2.1） ──

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

/// 負の裸数値は Absolute へ**忠実転写**する（負値は解決層が原点から負方向へ実導出する形＝
/// 縮退ではない。語彙層は負値を Invalid へ写像しない）。
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

// ── `Nem` / `Nlh`＝Absolute Em/Lh（1em＝文字高さ・1lh＝行送り・R1.3） ──

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

// ── `N%`＝Absolute Percent（100%＝文字高さ 1 個ぶん・解決層が実導出・R1.3） ──

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

// ── `@N`＝Relative（`@` 接頭＝現在の文字描画位置が基点・解決層が実導出・R3.1） ──

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

/// `@` の負値も語彙は Relative で保持（符号の意味づけは下流の解決層）。
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

// ── `centerx` / `centery`＝バルーン画像中央（正典 付録 A・R4.1/4.2） ──
//
// 正典逐語（requirements.md 付録 A・SSP 2.8.83）: 「Xにcenterx、Yにcenteryと書くと、
// バルーン画像の中央（幅／高さの半分）に移動する。これだけは文字描画開始点ではなく
// バルーン画像そのものが基準。」
// 語彙層は**軸を知らない**——`centerx` が Y 軸に書かれた取り違えの判定は解決層
// （`cursor_tag.rs`）の責務ゆえ、本層はどちらの軸に書かれていても同じ variant を返す。

#[test]
fn parse_centerx_is_center_x() {
    assert_eq!(parse_cursor_coord("centerx"), CursorCoord::CenterX);
}

#[test]
fn parse_centery_is_center_y() {
    assert_eq!(parse_cursor_coord("centery"), CursorCoord::CenterY);
}

/// `@` 接頭は「現在の文字描画位置からの相対座標」＝数値本体を要求する書式であり、正典に
/// `@centerx` という形は無い。中央指定の判定を `@` 剥離の**前**に置くので `@centerx` は
/// 該当せず、`@` を剥がした本体が数値でないため Invalid へ落ちる（design.md「語彙
/// `CursorCoord`」事後条件 `parse_cursor_coord("@centerx") == Invalid`）。
#[test]
fn parse_at_prefixed_center_is_invalid() {
    assert_eq!(parse_cursor_coord("@centerx"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("@centery"), CursorCoord::Invalid);
}

/// 正典は `centerx`／`centery` を小文字で記す。大小文字の扱いは正典沈黙ゆえ
/// `doc/COMPAT_ARCHITECTURE.md` §8 の「小文字の完全一致のみ」の先例に揃える
/// （design.md 事後条件 `parse_cursor_coord("CENTERX") == Invalid`）。
#[test]
fn parse_uppercase_center_is_invalid() {
    assert_eq!(parse_cursor_coord("CENTERX"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("CENTERY"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("CenterX"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("CenterY"), CursorCoord::Invalid);
}

/// 受理するのは**完全一致**だけ（部分一致・前後に何かが付いた形は数値として解釈できない）。
#[test]
fn parse_center_partial_match_is_invalid() {
    assert_eq!(parse_cursor_coord("center"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("centerxx"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("centerx1"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord(" centerx"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("centerx "), CursorCoord::Invalid);
}

// ── パース不能＝Invalid（R1.5・当該軸不動の源） ──

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

/// 非有限（NaN／inf）は Invalid へ縮退（解決層の式を汚さない防御）。
#[test]
fn parse_non_finite_is_invalid() {
    assert_eq!(parse_cursor_coord("NaN"), CursorCoord::Invalid);
    assert_eq!(parse_cursor_coord("inf"), CursorCoord::Invalid);
}

/// 全域性: どの `&str` を与えてもパニックせず必ず値を返す（決定論・total function）。
#[test]
fn parse_is_total_over_arbitrary_strings() {
    for raw in [
        "", "0", "-0", "5", "-3", "5.0", "5em", "5lh", "50%", "@5", "@5em", "@5%", "@-2", "abc",
        "em", "lh", "%", "@", "5xx", "@abc", "  ", "5 em", "e", "@@5", "1e3", "1.2.3", "centerx",
        "centery", "@centerx", "CENTERX", "center", "centerxx",
    ] {
        // パニックしないことを踏むのが主眼（戻り値の variant は各専用テストで固定）。
        let _ = parse_cursor_coord(raw);
    }
}
