//! `parse_zorder_tokens` の決定論檻——受理 3 形・拒否 4 分岐・スコープブロック正規化・
//! 相棒窓の畳み込み（要件 1.6／2.1〜2.4／2.6／3.4／3.5／6.3／8.1・design
//! 「Testing Strategy / Unit」1 と 5・design「ZOrderGroupLedger（既存・畳み込みを追加）」）。
//!
//! 実機・実ディスプレイ・World を一切必要としない純関数の檻である（要件 10.1）。
//! 可変の共有状態を持たないため、単独実行と一括実行で結果が変わらない（要件 10.3）。
//!
//! # 檻の並び
//!
//! - 受理 3 形と拒否 4 分岐＋拒否どうしの優先順・入力の前処理（`t_zgp1`〜`t_zgp14`）
//! - 相棒窓の補いと、補いがスコープどうしの並びに触れないこと（`t_zgp18`〜`t_zgp20`・
//!   `t_zgp22`〜`t_zgp28`）——片方の窓だけを指名されたスコープへ相棒を補う
//!   （`areka-P0-zorder-scope-inversion` 要件 3）。補う向き・記録の欄・要素数の判定との順序・
//!   数値モードの不関与・既存の拒否分岐の不動を押さえる。
//!
//! 同じスコープの 2 窓を隣り合いへ寄せる調停（完了 spec の要件 2.4）は
//! `areka-P0-zorder-scope-inversion` が覆した。書かれた順のまま採る解釈と並びの形の記録は
//! `zorder_group_ledger_written_order_tests.rs` が押さえる。
//!
//! # 拒否の檻が示すこと
//!
//! 拒否分岐の檻はすべて [`expect_reject`] を通す。この道具は `Ok` を受け取ったら
//! 要素列を添えて落ちるので、「拒否時に要素列を一切返さない」（要件 8.1 の部分適用
//! 禁止）が檻の構造そのもので示される。
//!
//! # 補いの檻が示すこと
//!
//! 補いが**動かしてはならないもの**——スコープどうしの相対順（書かれた位置）
//! ——を `t_zgp20`（補うブロックが後ろ）・`t_zgp22`（補うブロックが前）・
//! `t_zgp28`（補うブロックが前後のペアに挟まれる）の 3 方向から挟む。片側だけの入力は
//! 「まとめて端へ回す」実装と区別が付かない（初版 1.2 の教訓）。

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

    // 明示モードの要素 2 個。どちらも片方だけの指名なので、正規化の段で相棒窓が
    // 補われて窓 4 枚になる（要件 2.6）——**受理の可否は補う前の要素数で決まる**。
    let (explicit, _) = expect_accept(&["b1", "s0"]);
    assert_eq!(explicit, vec![b(1), s(1), b(0), s(0)]);
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

// ---------------------------------------------------------------------------
// 相棒窓の補い（`areka-P0-zorder-scope-inversion` 要件 3）
// ---------------------------------------------------------------------------

/// 2 窓そろって書かれたスコープの記録——`order` は書かれた並びの形。
fn written(scope: u32, order: WrittenOrder) -> Normalization {
    Normalization {
        scope,
        order,
        implied_partner: None,
    }
}

/// 片方の窓だけが書かれたスコープの記録——`partner` は**こちらが補った**窓の種別。
fn implied(scope: u32, partner: GroupWindowKind) -> Normalization {
    Normalization {
        scope,
        order: WrittenOrder::Adjacent,
        implied_partner: Some(partner),
    }
}

/// 数値モードでは正規化が何もしない。展開（⑷）が必ず `[Balloon, Char]` の隣接
/// ブロックを作るうえ、その並びは作者が書いた指定順ではないので、寄せるものも
/// 補うものも記録するものも無い（要件 2.4／2.6 はいずれも明示モードを名指しする）。
#[test]
fn t_zgp18_numeric_mode_normalization_is_a_no_op() {
    for tokens in [["1", "0"].as_slice(), ["2", "0", "1"].as_slice()] {
        let (_, normalizations) = expect_accept(tokens);

        assert!(
            normalizations.is_empty(),
            "tokens={tokens:?} normalizations={normalizations:?}"
        );
    }

    let (elements, _) = expect_accept(&["2", "0", "1"]);
    assert_eq!(
        elements,
        vec![b(2), s(2), b(0), s(0), b(1), s(1)],
        "展開順は指定順のまま＝正規化は要素列にも触れない"
    );
}

/// 片方の窓だけが書かれたスコープは、相棒窓を補って隣り合いにする（要件 2.6・
/// `areka-P0-zorder-scope-inversion` 要件 3.1・3.2）。
/// 補ったことは記録に残る——どちらの窓を補ったかまで読み取れる形である。
#[test]
fn t_zgp19_single_window_scope_is_completed_with_its_implied_partner() {
    let (elements, normalizations) = expect_accept(&["b1", "s0"]);
    assert_eq!(
        elements,
        vec![b(1), s(1), b(0), s(0)],
        "b1 の直後に s1 が、s0 の直前に b0 が補われる"
    );
    assert_eq!(
        normalizations,
        vec![
            implied(1, GroupWindowKind::Char),
            implied(0, GroupWindowKind::Balloon)
        ],
        "補った窓の種別が記録から読み取れない"
    );

    // 2 窓そろったスコープと 1 枚だけのスコープが同居する場合。並びの形の記録と
    // 補いの記録が同じ列に並び、どちらが起きたかを欄で弁別できる。
    let (mixed, mixed_records) = expect_accept(&["b1", "s0", "s1"]);
    assert_eq!(
        mixed,
        vec![b(1), b(0), s(0), s(1)],
        "スコープ 1 は書かれた順のまま・s0 のすぐ手前に b0 が補われる"
    );
    assert_eq!(
        mixed_records,
        vec![
            written(1, WrittenOrder::Split),
            implied(0, GroupWindowKind::Balloon)
        ]
    );
}

/// 補いが足すのは相棒 1 枚だけであり、書かれた窓どうしの相対順は書かれたまま保たれる
/// （要件 2.5 の「属さない窓を動かさない」をグループの内側にも効かせる）。
#[test]
fn t_zgp20_other_scopes_keep_their_relative_order() {
    let (elements, normalizations) = expect_accept(&["b0", "b2", "s3", "s0"]);

    assert_eq!(
        elements,
        vec![b(0), b(2), s(2), b(3), s(3), s(0)],
        "b2 と s3 は片方だけの指名＝相棒を補っても書かれた窓の前後は書かれた順のまま"
    );
    assert_eq!(
        normalizations,
        vec![
            written(0, WrittenOrder::Split),
            implied(2, GroupWindowKind::Char),
            implied(3, GroupWindowKind::Balloon)
        ]
    );
}

/// 片方だけ指名されたスコープのブロックは、**ペアブロックより前に書かれていれば
/// 前のまま**残る。畳み込みは相棒を補うだけで、スコープの並びには触らない。
///
/// [`t_zgp20_other_scopes_keep_their_relative_order`] の入力はどれも片方だけの指名が
/// 最初のペアブロックより**後ろ**にあり、「補ったブロックをまとめて末尾へ回す」実装と
/// 区別が付かなかった。ここでは片方だけの指名を先頭に置いて、その差を赤にする。
/// 差は「どちらの窓が手前か」そのものであり、本機能が存在する理由に直結する。
#[test]
fn t_zgp22_single_window_scope_written_before_a_block_stays_before_it() {
    // s3 はペアを組むスコープ 0 のどの要素よりも前に書かれている。
    let (elements, normalizations) = expect_accept(&["s3", "b0", "b2", "s0"]);

    assert_eq!(
        elements,
        vec![b(3), s(3), b(0), b(2), s(2), s(0)],
        "s3 のブロックは先頭のまま・b2 の相棒は b2 のすぐ奥・s0 は書かれた末尾のまま"
    );
    assert_eq!(
        normalizations,
        vec![
            implied(3, GroupWindowKind::Balloon),
            written(0, WrittenOrder::Split),
            implied(2, GroupWindowKind::Char)
        ],
        "スコープ 0 は非隣接（間に b2・s2）として記録される"
    );

    // 最小形: ペアが既に隣接ブロックでも、前に書かれた片方だけの指名は前のまま。
    let (minimal, minimal_records) = expect_accept(&["s3", "b0", "s0"]);
    assert_eq!(minimal, vec![b(3), s(3), b(0), s(0)]);
    assert_eq!(
        minimal_records,
        vec![
            implied(3, GroupWindowKind::Balloon),
            written(0, WrittenOrder::Adjacent)
        ],
        "隣り合いで書かれたスコープも記録に残る"
    );
}

// ---------------------------------------------------------------------------
// 相棒窓の畳み込み（要件 2.6）
// ---------------------------------------------------------------------------

/// `bN` だけが書かれたら、相棒のキャラ窓 `sN` を**直後**へ補う（要件 2.6）。
///
/// 補うのは相棒 1 枚だけであり、他スコープの窓は 1 つも増えも減りもしない。
#[test]
fn t_zgp23_balloon_only_scope_gains_its_char_window_right_below() {
    let (elements, normalizations) = expect_accept(&["b1", "b0"]);

    assert_eq!(
        elements,
        vec![b(1), s(1), b(0), s(0)],
        "指名された b の直後に相棒の s が入る"
    );
    assert_eq!(
        normalizations,
        vec![
            implied(1, GroupWindowKind::Char),
            implied(0, GroupWindowKind::Char)
        ],
        "補ったのはキャラ窓である、と記録から読み取れない"
    );
}

/// `sN` だけが書かれたら、相棒のバルーン窓 `bN` を**直前**へ補う（要件 2.6）。
///
/// 補いは「バルーンがキャラ窓の直上」という既存の不変条件（要件 6.3）に従うので、
/// 指名された窓の**前**に入る。ブロックが立つ位置は `sN` が書かれた位置である。
#[test]
fn t_zgp24_char_only_scope_gains_its_balloon_window_right_above() {
    let (elements, normalizations) = expect_accept(&["s1", "s0"]);

    assert_eq!(
        elements,
        vec![b(1), s(1), b(0), s(0)],
        "指名された s の直前に相棒の b が入る"
    );
    assert_eq!(
        normalizations,
        vec![
            implied(1, GroupWindowKind::Balloon),
            implied(0, GroupWindowKind::Balloon)
        ],
        "補ったのはバルーン窓である、と記録から読み取れない"
    );

    // ブロックの位置は「指名された窓が書かれた位置」＝他スコープとの前後は動かない。
    let (with_pair, _) = expect_accept(&["b0", "s0", "s9"]);
    assert_eq!(
        with_pair,
        vec![b(0), s(0), b(9), s(9)],
        "補ったブロックが前へ繰り上がっている（畳み込みが並びに触っている）"
    );
}

/// 畳み込みは**要素数の判定より後**に置く。要素 1 個のタグが畳み込みで 2 個に
/// 化けて受理されてはならない（要件 1.6 と要件 2.6 の順序関係）。
#[test]
fn t_zgp25_folding_never_rescues_a_one_element_tag() {
    for tokens in [
        ["b0"].as_slice(),
        ["s0"].as_slice(),
        ["balloon2"].as_slice(),
        ["surface7"].as_slice(),
        ["0"].as_slice(),
    ] {
        assert_eq!(
            expect_reject(tokens),
            ZOrderReject::TooFewElements { count: 1 },
            "畳み込み後の窓の枚数で数えている（要件 1.6 が要件 2.6 に負けている）: \
             tokens={tokens:?}"
        );
    }
}

/// 数値モードでは畳み込みが起きない（展開で既に隣接ブロックになっているため）。
/// 記録が 1 件も出ないことを、補いの欄まで含めて押さえる。
#[test]
fn t_zgp26_numeric_mode_never_implies_a_partner() {
    for tokens in [
        ["1", "0"].as_slice(),
        ["0", "1", "2"].as_slice(),
        ["7", "7000000"].as_slice(),
    ] {
        let (elements, normalizations) = expect_accept(tokens);

        assert!(
            normalizations.is_empty(),
            "数値モードで正規化の記録が出ている: tokens={tokens:?} \
             normalizations={normalizations:?}"
        );
        assert_eq!(
            elements.len(),
            tokens.len() * 2,
            "数値モードの展開が窓を増やしすぎている（畳み込みが二重に走っている）: \
             tokens={tokens:?} elements={elements:?}"
        );
    }
}

/// 既存の拒否分岐は畳み込みの導入で 1 つも動かない（判定順の据え置き）。
///
/// 解釈不能 → モード混在 → 要素数 → 数値展開 → タグ内重複 → 正規化 の順である。
/// 畳み込みは最後の段なので、拒否される指定には一切触れない——拒否経路で要素列が
/// 1 つも外へ出ないことは [`expect_reject`] の構造が示す（要件 8.1）。
#[test]
fn t_zgp27_existing_reject_branches_are_unchanged_by_folding() {
    // 片方だけの指名を混ぜても、先に立つ拒否理由が変わらない。
    assert_eq!(
        expect_reject(&["b0", "xyz"]),
        ZOrderReject::UnparsableToken {
            token: "xyz".to_owned()
        },
        "解釈不能は最優先のまま"
    );
    assert_eq!(
        expect_reject(&["b0", "1"]),
        ZOrderReject::ModeMixed,
        "モード混在は要素数・重複より先のまま"
    );
    assert_eq!(
        expect_reject(&["b1", "s0", "b1"]),
        ZOrderReject::DuplicateElement { element: b(1) },
        "タグ内重複は正規化より先のまま"
    );

    // 畳み込みが新しい重複を作らないこと（補うのは不在の相棒だけ）。
    for tokens in [
        ["b1", "s1"].as_slice(),
        ["s1", "b1"].as_slice(),
        ["b1", "s0"].as_slice(),
        ["b0", "b1", "s1"].as_slice(),
    ] {
        let (elements, _) = expect_accept(tokens);
        let unique: std::collections::HashSet<GroupElement> = elements.iter().copied().collect();
        assert_eq!(
            unique.len(),
            elements.len(),
            "畳み込みが既に在る窓をもう 1 枚足している: tokens={tokens:?} elements={elements:?}"
        );
    }
}

/// 片方だけ指名されたスコープは、**前にも後ろにも寄せない**。
///
/// 1 本の入力で両側から挟む——スコープ 5 は前にペアブロック（0）を、後ろにも
/// ペアブロック（1）を持つ。まとめて末尾へ回す実装でも先頭へ繰り上げる実装でも
/// 赤くなる（初版 1.2 の教訓＝片側だけの入力では「動かしてはならないもの」を
/// 守れない）。
#[test]
fn t_zgp28_an_implied_block_is_squeezed_from_both_sides() {
    let (elements, normalizations) = expect_accept(&["b0", "s0", "s5", "b1", "s1"]);

    assert_eq!(
        elements,
        vec![b(0), s(0), b(5), s(5), b(1), s(1)],
        "補ったブロックが端へ寄っている（畳み込みがスコープの並びに触っている）"
    );
    assert_eq!(
        normalizations,
        vec![
            written(0, WrittenOrder::Adjacent),
            implied(5, GroupWindowKind::Balloon),
            written(1, WrittenOrder::Adjacent)
        ],
        "記録もスコープの初出順に並ぶ"
    );

    // 対照: 補いが起きない同形の指定。ブロックの並びは同じである＝上の並びは
    // 「畳み込みが起きたから」動いたのではない、と読める。
    let (control, _) = expect_accept(&["b0", "s0", "b5", "s5", "b1", "s1"]);
    assert_eq!(control, elements, "補いの有無で並びが変わってはならない");
}
