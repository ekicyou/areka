//! 語彙分類純関数（design C1）の決定論檻（要件 1.1／1.2／1.3／4.1／4.6／7.2）。
//!
//! 行列:
//! - [`classify_x_vocab`]: 数値 × キーワード 3 種（`center`／`top`／`bottom`）×
//!   不正値 × 未指定 × 大文字混在（→ `Invalid`）。
//! - [`classify_limit_vocab`]: `"0"` × `"1"` × 未指定 × 不正値。
//! - 不変量「`x_num` があれば常に `Numeric`」（design C1 Invariants・要件 5.1 の同一性を
//!   型と構造で保証する回帰境界）。
//! - 未指定（`Numeric(None)`／`Value(true)`）と不正値（`Invalid`）の**区別**（要件 7.2）。
//!
//! 本層は分類だけを持ち、警告の発行は呼び出し側（`apply_scope_windowpositions`・
//! scope 文脈付き・task 2.2）が所有する（design C1 Invariants）。ゆえに本檻は
//! `warn` の有無を主張せず、`Invalid` という**警告可能な戻り値**が出ることだけを固定する。

use super::*;

/// 実運用で `x_raw`／`limit_raw` に載りうる「生値」の代表列。
///
/// 数値経路の不変量檻（[`classify_x_vocab_never_consults_raw_when_numeric_parsed`]）が
/// 生値の全種類を舐めるために使う——キーワード・不正値・未指定のどれであっても
/// 数値が読めていれば結果は `Numeric` でなければならない。
const RAW_SAMPLES: [Option<&str>; 7] = [
    None,
    Some("center"),
    Some("top"),
    Some("bottom"),
    Some("Center"),
    Some("garbage"),
    Some(""),
];

// ---------------------------------------------------------------------------
// classify_x_vocab
// ---------------------------------------------------------------------------

/// キーワード 3 種の小文字完全一致（要件 4.1・DD8）。
/// `top` は `center` と同義＝どちらも中央上 [`BalloonXMode::CenterTop`] へ解決する。
#[test]
fn classify_x_vocab_accepts_lowercase_keywords() {
    assert_eq!(
        classify_x_vocab(None, Some("center")),
        XVocab::Keyword(BalloonXMode::CenterTop)
    );
    assert_eq!(
        classify_x_vocab(None, Some("top")),
        XVocab::Keyword(BalloonXMode::CenterTop)
    );
    assert_eq!(
        classify_x_vocab(None, Some("bottom")),
        XVocab::Keyword(BalloonXMode::CenterBottom)
    );
}

/// `Side` は分類結果に現れない（design C1 の型注記）——キーワード語彙は
/// 中央上／中央下の 2 値しか作らず、`Side` は「キーワードでない」ことの表現である。
#[test]
fn classify_x_vocab_keyword_never_yields_side_mode() {
    for raw in ["center", "top", "bottom"] {
        match classify_x_vocab(None, Some(raw)) {
            XVocab::Keyword(mode) => assert_ne!(mode, BalloonXMode::Side, "raw={raw}"),
            other => panic!("raw={raw} はキーワードとして分類されるべき: {other:?}"),
        }
    }
}

/// 大文字混在は**キーワードとして受理しない**（DD8: 小文字の完全一致のみ）。
/// `Invalid` へ落ちることで呼び出し側の warn が焚かれ、作者が綴りに気づける（要件 4.6）。
#[test]
fn classify_x_vocab_rejects_mixed_case_keywords() {
    for raw in [
        "Center", "CENTER", "cEnter", "Top", "TOP", "tOp", "Bottom", "BOTTOM", "bottoM",
    ] {
        assert_eq!(
            classify_x_vocab(None, Some(raw)),
            XVocab::Invalid,
            "raw={raw} は大文字混在ゆえ不正値へ落ちる"
        );
    }
}

/// 未指定は `Numeric(None)`＝「調整なし」（既存 [`to_screen_adjust`] の入力語彙そのまま）。
/// 生値そのものが無い状態であり、警告の対象ではない（要件 1.2 の x 側対応物）。
#[test]
fn classify_x_vocab_unspecified_is_numeric_none() {
    assert_eq!(classify_x_vocab(None, None), XVocab::Numeric(None));
}

/// 数値でもキーワードでもない値は `Invalid`（要件 4.6）。
/// 空文字（`windowposition.x,` のような値なし行が kv 層で作る）も不正値である
/// ——「キーが在るのに読めない」ことと「キーが無い」ことは別事象。
#[test]
fn classify_x_vocab_classifies_non_numeric_non_keyword_as_invalid() {
    for raw in [
        "",         // 値なし（キーは在る）
        "centre",   // 綴り違い
        "center2",  // 前方一致では受理しない
        "topmost",  // 前方一致では受理しない
        "left",     // 別語彙
        "0.5",      // 非整数
        "1,2",      // 複数値
        "true",     // 別ドメインの語彙
        "\u{3042}", // 非 ASCII
    ] {
        assert_eq!(
            classify_x_vocab(None, Some(raw)),
            XVocab::Invalid,
            "raw={raw:?}"
        );
    }
}

/// **未指定と不正値を区別する**（要件 7.2 が明記）。
/// 同じ「数値が読めなかった」状態でも、未指定は無警告で既定へ、不正値は警告付き縮退へ
/// 分岐する——現行の「非数値を無警告で未指定と同一視する」挙動の是正点そのもの（要件 4.6）。
#[test]
fn classify_x_vocab_distinguishes_unspecified_from_invalid() {
    let unspecified = classify_x_vocab(None, None);
    let invalid = classify_x_vocab(None, Some("centre"));
    assert_eq!(unspecified, XVocab::Numeric(None));
    assert_eq!(invalid, XVocab::Invalid);
    assert_ne!(unspecified, invalid);
}

/// 不変量（design C1 Invariants）: `x_num.is_some()` なら**常に** `Numeric`。
///
/// 数値経路は生文字列を一切参照しない——これが要件 5.1（数値指定の変換を本仕様の前後で
/// 同一に保つ）を型と構造で保証する回帰境界である。生値がキーワードでも大文字混在でも
/// 不正値でも空でも、数値が読めていれば結果は数値そのものでなければならない。
#[test]
fn classify_x_vocab_never_consults_raw_when_numeric_parsed() {
    for num in [i32::MIN, -190, -1, 0, 1, 266, i32::MAX] {
        for raw in RAW_SAMPLES {
            assert_eq!(
                classify_x_vocab(Some(num), raw),
                XVocab::Numeric(Some(num)),
                "num={num} raw={raw:?}"
            );
        }
    }
}

/// 生値の前後空白は上流 kv 層（`kv::parse_kv`）で既に除かれている（`parse.rs` の転記層契約）。
/// 本層の `trim` はその冗長化であり、**実際に届く値（trim 済み）に対しては恒等**である
/// ——同じ語彙が空白の有無で別の分類へ落ちない（DD8「trim 済み文字列との完全一致」）。
#[test]
fn classify_x_vocab_trim_is_identity_for_kv_trimmed_values() {
    for raw in ["center", "top", "bottom", "centre", ""] {
        assert_eq!(
            classify_x_vocab(None, Some(raw)),
            classify_x_vocab(None, Some(&format!("  {raw}  "))),
            "raw={raw:?}"
        );
    }
    // 空白のみの値は trim 後に空文字と同値＝不正値。
    assert_eq!(classify_x_vocab(None, Some("   ")), XVocab::Invalid);
}

// ---------------------------------------------------------------------------
// classify_limit_vocab
// ---------------------------------------------------------------------------

/// 値域 0/1 の受理（要件 1.1）。`"0"`＝無効・`"1"`＝有効。
#[test]
fn classify_limit_vocab_accepts_zero_and_one() {
    assert_eq!(classify_limit_vocab(Some("0")), LimitVocab::Value(false));
    assert_eq!(classify_limit_vocab(Some("1")), LimitVocab::Value(true));
}

/// 未指定は正典既定 1（有効）へ（要件 1.2）。警告の対象ではない。
#[test]
fn classify_limit_vocab_unspecified_is_canonical_default_enabled() {
    assert_eq!(classify_limit_vocab(None), LimitVocab::Value(true));
}

/// 0/1 以外は `Invalid`（要件 1.3: 呼び出し側が警告のうえ既定 1 へ縮退する）。
/// 数値として読める `"2"`／`"-1"`／`"01"` も値域外ゆえ不正値であり、
/// 「数値なら通す」という緩い受理を作らない。
#[test]
fn classify_limit_vocab_classifies_out_of_domain_as_invalid() {
    for raw in [
        "", "2", "-1", "01", "1.0", "10", "true", "false", "on", "off", "yes", "ON",
    ] {
        assert_eq!(
            classify_limit_vocab(Some(raw)),
            LimitVocab::Invalid,
            "raw={raw:?}"
        );
    }
}

/// **未指定と不正値を区別する**（要件 7.2）。どちらも最終的には既定 1 へ落ちるが、
/// 不正値だけが警告経路を通る（要件 1.3／6.3: ログの無い縮退経路を作らない）。
#[test]
fn classify_limit_vocab_distinguishes_unspecified_from_invalid() {
    let unspecified = classify_limit_vocab(None);
    let invalid = classify_limit_vocab(Some("2"));
    assert_eq!(unspecified, LimitVocab::Value(true));
    assert_eq!(invalid, LimitVocab::Invalid);
    assert_ne!(unspecified, invalid);
}

/// x 側と同じく、kv 層 trim 済みの値に対して本層の `trim` は恒等（DD8）。
/// 空白のみの値は trim 後に空文字と同値＝不正値。
#[test]
fn classify_limit_vocab_trim_is_identity_for_kv_trimmed_values() {
    for raw in ["0", "1", "2", ""] {
        assert_eq!(
            classify_limit_vocab(Some(raw)),
            classify_limit_vocab(Some(&format!(" {raw} "))),
            "raw={raw:?}"
        );
    }
    assert_eq!(classify_limit_vocab(Some("   ")), LimitVocab::Invalid);
}
