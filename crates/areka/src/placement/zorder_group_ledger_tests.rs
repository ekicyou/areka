//! `parse_zorder_tokens` の決定論檻——受理 3 形と拒否 4 分岐（要件 1.6／2.1〜2.3／
//! 3.4／3.5／8.1・design「Testing Strategy / Unit」1）。
//!
//! 実機・実ディスプレイ・World を一切必要としない純関数の檻である（要件 10.1）。
//! 可変の共有状態を持たないため、単独実行と一括実行で結果が変わらない（要件 10.3）。
//!
//! # 拒否の檻が示すこと
//!
//! 拒否分岐の檻はすべて [`expect_reject`] を通す。この道具は `Ok` を受け取ったら
//! 要素列を添えて落ちるので、「拒否時に要素列を一切返さない」（要件 8.1 の部分適用
//! 禁止）が檻の構造そのもので示される。

use super::*;

// ---------------------------------------------------------------------------
// 檻の道具
// ---------------------------------------------------------------------------

/// バルーン窓の要素（`balloonN`／`bN`）。
fn b(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Balloon,
    }
}

/// キャラ窓の要素（`surfaceN`／`sN`）。
fn s(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Char,
    }
}

/// 受理を期待し、要素列と正規化の記録を取り出す。
fn expect_accept(tokens: &[&str]) -> (Vec<GroupElement>, Vec<Normalization>) {
    match parse_zorder_tokens(tokens) {
        Ok(parsed) => parsed,
        Err(reject) => {
            panic!("受理されるべき指定が拒否された: tokens={tokens:?} reject={reject:?}")
        }
    }
}

/// 拒否を期待し、拒否理由だけを取り出す。
///
/// `Ok` が返った場合は**要素列と正規化記録を添えて**落とす。要件 8.1 の
/// 「部分的に適用しない」は、拒否経路で要素列が 1 つも外へ出ないことで成立する。
fn expect_reject(tokens: &[&str]) -> ZOrderReject {
    match parse_zorder_tokens(tokens) {
        Ok((elements, normalizations)) => panic!(
            "拒否されるべき指定が受理された: tokens={tokens:?} elements={elements:?} \
             normalizations={normalizations:?}"
        ),
        Err(reject) => reject,
    }
}

// ---------------------------------------------------------------------------
// 受理（3 形）
// ---------------------------------------------------------------------------

/// 受理形①: 数値モード。要素 1 個が「バルーン窓・キャラ窓の 2 枚」へ展開され、
/// バルーンが自スコープのキャラ窓より手前に来る（要件 1.1／1.2）。
#[test]
fn t_zgp1_numeric_mode_expands_each_scope_into_balloon_then_char() {
    let (elements, normalizations) = expect_accept(&["1", "0"]);

    assert_eq!(
        elements,
        vec![b(1), s(1), b(0), s(0)],
        "数値モードの各要素は [Balloon, Char] の順に展開され、左の要素ほど手前に並ぶ"
    );
    assert!(
        normalizations.is_empty(),
        "同一スコープの 2 窓が既に隣接ブロックである以上、調整の記録は出ない: {normalizations:?}"
    );
}

/// 受理形②: 明示モード（完全形 `balloonN`／`surfaceN`）。要素は窓 1 枚単位で、
/// 書かれた順にそのまま並ぶ（要件 2.1）。
#[test]
fn t_zgp2_explicit_long_form_keeps_one_window_per_element() {
    let (elements, _) = expect_accept(&["balloon1", "surface1", "balloon0", "surface0"]);

    assert_eq!(elements, vec![b(1), s(1), b(0), s(0)]);
}

/// 受理形③: 明示モードの省略形（`bN`／`sN`）。完全形と同じ意味に解釈される
/// （要件 2.2）。あわせて数値モードが明示モードの特例であること——3 形が同じ
/// 要素列へ落ちること——を 1 か所で固定する。
#[test]
fn t_zgp3_explicit_short_form_equals_long_form_and_numeric_expansion() {
    let (short_form, _) = expect_accept(&["b1", "s1", "b0", "s0"]);
    let (long_form, _) = expect_accept(&["balloon1", "surface1", "balloon0", "surface0"]);
    let (numeric, _) = expect_accept(&["1", "0"]);

    assert_eq!(
        short_form, long_form,
        "`bN`／`sN` は `balloonN`／`surfaceN` と同義"
    );
    assert_eq!(
        short_form, numeric,
        "数値モードは明示モードの特例（[Balloon, Char] への展開）である"
    );
}

/// 1 つのタグの中で `bN` と `sN` が並ぶことは重複ではない（要件 3.5）。
/// 重複判定が「スコープ」ではなく「窓」を数えていることを示す。
#[test]
fn t_zgp4_balloon_and_char_of_same_scope_are_distinct_windows() {
    let (elements, _) = expect_accept(&["b1", "s1"]);

    assert_eq!(elements, vec![b(1), s(1)]);
}

/// 受理は要素 2 個ちょうどで成立する（要件 1.6 の境界の受理側）。
#[test]
fn t_zgp5_exactly_two_elements_is_accepted() {
    let (numeric, _) = expect_accept(&["0", "1"]);
    assert_eq!(numeric, vec![b(0), s(0), b(1), s(1)]);

    let (explicit, _) = expect_accept(&["b1", "s0"]);
    assert_eq!(explicit, vec![b(1), s(0)]);
}

// ---------------------------------------------------------------------------
// 拒否①: モード混在（要件 2.3）
// ---------------------------------------------------------------------------

/// 数値のみの要素と `b`／`s` を伴う要素が 1 つのタグに混在したら、そのタグによる
/// 変更を一切行わない（要件 2.3）。並び順を入れ替えても同じ判定になる。
#[test]
fn t_zgp6_reject_mode_mixed() {
    for tokens in [
        ["0", "b1"].as_slice(),
        ["b1", "0"].as_slice(),
        ["balloon0", "1", "s2"].as_slice(),
    ] {
        assert_eq!(
            expect_reject(tokens),
            ZOrderReject::ModeMixed,
            "tokens={tokens:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 拒否②: タグ内重複（要件 3.4）
// ---------------------------------------------------------------------------

/// 同じ窓を指す要素が 2 回以上現れたらタグ全体を採用しない（要件 3.4）。
/// 拒否理由は最初に重複した**窓**を伴う。
#[test]
fn t_zgp7_reject_duplicate_window_element() {
    assert_eq!(
        expect_reject(&["b1", "s0", "b1"]),
        ZOrderReject::DuplicateElement { element: b(1) },
        "明示モードの同一窓の再出現"
    );
    assert_eq!(
        expect_reject(&["balloon1", "s1", "b1"]),
        ZOrderReject::DuplicateElement { element: b(1) },
        "完全形と省略形が同じ窓を指すことも重複として数える（要件 2.2）"
    );
    assert_eq!(
        expect_reject(&["0", "1", "0"]),
        ZOrderReject::DuplicateElement { element: b(0) },
        "数値モードは展開後の窓で重複を数える"
    );
}

// ---------------------------------------------------------------------------
// 拒否③: 要素 2 個未満（要件 1.6）
// ---------------------------------------------------------------------------

/// 要素が 2 個に満たないグループはそのタグによる変更を行わない（要件 1.6）。
///
/// 数えるのは**展開前の要素**である。`["0"]` は展開すれば窓 2 枚になるが、
/// スコープ ID が 1 個しか無いので受理しない（要件 1.1 の「2 個以上のスコープ ID」）。
#[test]
fn t_zgp8_reject_too_few_elements_counted_before_expansion() {
    assert_eq!(
        expect_reject(&[]),
        ZOrderReject::TooFewElements { count: 0 },
        "トークンが 1 つも無い指定"
    );
    assert_eq!(
        expect_reject(&["0"]),
        ZOrderReject::TooFewElements { count: 1 },
        "数値モードの 1 要素は展開後に窓 2 枚になるが受理しない"
    );
    assert_eq!(
        expect_reject(&["b0"]),
        ZOrderReject::TooFewElements { count: 1 },
        "明示モードの 1 要素"
    );
}

// ---------------------------------------------------------------------------
// 拒否④: 解釈できないトークン（要件 8.1）
// ---------------------------------------------------------------------------

/// 解釈できないトークンが 1 つでもあれば、そのタグによる変更を一切行わない
/// （要件 8.1）。拒否理由は受け取ったトークンをそのまま伴う。
#[test]
fn t_zgp9_reject_unparsable_token_carries_the_received_token() {
    for (tokens, expected) in [
        (["0", "xyz"].as_slice(), "xyz"),
        (["b", "s1"].as_slice(), "b"),
        (["b1", "surface"].as_slice(), "surface"),
        (["-1", "0"].as_slice(), "-1"),
        (["+1", "0"].as_slice(), "+1"),
        (["0", ""].as_slice(), ""),
        (["4294967296", "0"].as_slice(), "4294967296"),
    ] {
        assert_eq!(
            expect_reject(tokens),
            ZOrderReject::UnparsableToken {
                token: expected.to_owned()
            },
            "tokens={tokens:?}"
        );
    }
}

/// 語彙は小文字ちょうどで一致させる（`windowposition` の語彙分類と同じ流儀）。
/// 大文字混じりは黙って通さず、解釈できないトークンとして記録される。
#[test]
fn t_zgp10_vocabulary_is_case_sensitive() {
    for token in ["Balloon0", "SURFACE0", "B0", "S0"] {
        assert_eq!(
            expect_reject(&[token, "s1"]),
            ZOrderReject::UnparsableToken {
                token: token.to_owned()
            },
            "token={token:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 拒否どうしの優先順
// ---------------------------------------------------------------------------

/// 解釈できないトークンは他のどの拒否よりも先に立つ。トークンを 1 つずつ読む段で
/// 落ちる以上、モード混在も要素数も重複も判定するまでに至らないからである
/// （どの分岐で落ちても「変更を一切行わない」ことは同じ・要件 8.1）。
#[test]
fn t_zgp11_unparsable_token_takes_precedence_over_other_rejects() {
    assert_eq!(
        expect_reject(&["0", "b1", "xyz"]),
        ZOrderReject::UnparsableToken {
            token: "xyz".to_owned()
        },
        "モード混在よりも先に落ちる"
    );
    assert_eq!(
        expect_reject(&["xyz"]),
        ZOrderReject::UnparsableToken {
            token: "xyz".to_owned()
        },
        "要素 2 個未満よりも先に落ちる"
    );
}

/// モード混在は要素の重複よりも先に立つ。混在したタグは要素列そのものを組めないので、
/// 重複を数える意味が無い。
#[test]
fn t_zgp12_mode_mixed_takes_precedence_over_duplicate() {
    assert_eq!(expect_reject(&["0", "0", "b1"]), ZOrderReject::ModeMixed);
}

// ---------------------------------------------------------------------------
// 入力の前処理
// ---------------------------------------------------------------------------

/// トークン前後の空白は上流（さくらスクリプトの引数分割・kv 層）で既に落ちている。
/// 本層の trim はその冗長化であり、実際に届く値に対しては恒等である
/// （`windowposition::classify_x_vocab` と同じ位置づけ）。
#[test]
fn t_zgp13_trim_is_identity_for_upstream_trimmed_tokens() {
    let (padded, _) = expect_accept(&[" b1 ", "\ts0\t"]);
    let (bare, _) = expect_accept(&["b1", "s0"]);

    assert_eq!(padded, bare);
    // 空白だけのトークンは trim 後に空文字と同値＝解釈できないトークン。
    assert_eq!(
        expect_reject(&["   ", "s1"]),
        ZOrderReject::UnparsableToken {
            token: "   ".to_owned()
        },
        "拒否理由は trim 前の受け取ったトークンをそのまま伴う"
    );
}

/// スコープ ID の個数に上限を設けない（要件 3.7）。
#[test]
fn t_zgp14_no_upper_bound_on_element_count() {
    let tokens: Vec<String> = (0..64u32).map(|n| n.to_string()).collect();
    let borrowed: Vec<&str> = tokens.iter().map(String::as_str).collect();

    let (elements, _) = expect_accept(&borrowed);

    assert_eq!(elements.len(), 128, "64 スコープ × 窓 2 枚");
    assert_eq!(elements[0], b(0));
    assert_eq!(elements[127], s(63));
}
