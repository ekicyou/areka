//! `parse_zorder_tokens` の決定論檻——受理 3 形・拒否 4 分岐・スコープブロック正規化
//! （要件 1.6／2.1〜2.4／3.4／3.5／6.3／8.1・design「Testing Strategy / Unit」1 と 2・
//! design「File Structure Plan」の本ファイル欄「9 分岐＋正規化＋actor 非依存の決定論テスト」）。
//!
//! 実機・実ディスプレイ・World を一切必要としない純関数の檻である（要件 10.1）。
//! 可変の共有状態を持たないため、単独実行と一括実行で結果が変わらない（要件 10.3）。
//!
//! # 檻の並び
//!
//! - 受理 3 形と拒否 4 分岐＋拒否どうしの優先順・入力の前処理（`t_zgp1`〜`t_zgp14`）
//! - スコープブロック正規化（`t_zgp15`〜`t_zgp22`）——同一スコープの 2 窓を
//!   `[Balloon, Char]` の隣接ブロックへ寄せる調停（要件 2.4）と、それが既存の不変条件
//!   「バルーン窓はキャラ窓の直上」（要件 6.3）を破らないことを押さえる。
//!
//! # 拒否の檻が示すこと
//!
//! 拒否分岐の檻はすべて [`expect_reject`] を通す。この道具は `Ok` を受け取ったら
//! 要素列を添えて落ちるので、「拒否時に要素列を一切返さない」（要件 8.1 の部分適用
//! 禁止）が檻の構造そのもので示される。
//!
//! # 正規化の檻が示すこと
//!
//! 正規化の檻はすべて [`assert_paired_scopes_form_balloon_first_blocks`] を通す。
//! 2 窓そろったどのスコープについても `char_at == balloon_at + 1` を全数で見るので、
//! 「隣接している」と「バルーンが手前」を 1 本の式で同時に押さえる（要件 2.4／6.3）。
//! 加えて、正規化が**動かしてはならないもの**——2 窓そろっていないスコープの要素の
//! 位置と、他スコープどうしの相対順——を `t_zgp20` と `t_zgp22` の 2 方向から挟む。

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

// ---------------------------------------------------------------------------
// スコープブロック正規化（要件 2.4・research R6）
// ---------------------------------------------------------------------------

/// 正規化後の要素列で、2 窓そろったスコープがどれも「バルーンが先の隣接ブロック」に
/// なっていることを確かめる（要件 2.4／6.3・task 1.2 の完了状態）。
///
/// `si == bi + 1` の 1 本で「隣接している」と「バルーンが先である」の両方を見る。
fn assert_paired_scopes_form_balloon_first_blocks(elements: &[GroupElement]) {
    for element in elements {
        let scope = element.scope;
        let (Some(balloon_at), Some(char_at)) = (
            elements.iter().position(|other| *other == b(scope)),
            elements.iter().position(|other| *other == s(scope)),
        ) else {
            // 2 窓そろっていないスコープは寄せる相手が居ない＝不変条件の対象外。
            continue;
        };

        assert_eq!(
            char_at,
            balloon_at + 1,
            "スコープ {scope} の 2 窓がバルーン先頭の隣接ブロックになっていない: {elements:?}"
        );
    }
}

/// 反転指定: 同一スコープのキャラ窓がバルーン窓より手前に並ぶ指定は、既存の不変条件
/// （バルーンはキャラ窓の直上）を優先して `[Balloon, Char]` へ寄せる（要件 2.4）。
/// 寄せ先はそのスコープの要素が最初に現れた位置である。
#[test]
fn t_zgp15_inverted_scope_pair_is_folded_into_balloon_then_char() {
    let (elements, normalizations) = expect_accept(&["s1", "b1"]);

    assert_eq!(
        elements,
        vec![b(1), s(1)],
        "キャラ窓を手前に置く要求は採用しない"
    );
    assert_eq!(
        normalizations,
        vec![Normalization {
            scope: 1,
            reordered: true
        }],
        "指定順を採用しなかったことが記録される"
    );
    assert_paired_scopes_form_balloon_first_blocks(&elements);

    // 寄せ先は「最初に現れた要素の位置」であって「バルーン窓が書かれた位置」ではない。
    // 反転していると両者は食い違うので、間に他スコープを挟んで違いを見えるようにする。
    let (with_neighbour, _) = expect_accept(&["s1", "b2", "b1"]);
    assert_eq!(
        with_neighbour,
        vec![b(1), s(1), b(2)],
        "スコープ 1 のブロックは s1 が書かれた先頭位置に立ち、b2 はその後ろに残る"
    );
}

/// 非隣接指定: 同一スコープの 2 窓の間に他スコープが挟まる指定も、反転と同じ規則で
/// 「先に現れた位置の隣接ブロック」へ寄せる（要件 2.4・research R6 の一元処理）。
#[test]
fn t_zgp16_non_adjacent_scope_pair_is_folded_at_first_appearance() {
    let (elements, normalizations) = expect_accept(&["b1", "s0", "s1", "b0"]);

    assert_eq!(
        elements,
        vec![b(1), s(1), b(0), s(0)],
        "スコープ 1 は位置 0 へ、スコープ 0 は位置 1（=s0 が書かれた位置）へ寄る"
    );
    assert_eq!(
        normalizations,
        vec![
            Normalization {
                scope: 1,
                reordered: true
            },
            Normalization {
                scope: 0,
                reordered: true
            }
        ],
        "記録はスコープが最初に現れた順に並ぶ"
    );
    assert_paired_scopes_form_balloon_first_blocks(&elements);
}

/// 指定どおりで既に `[Balloon, Char]` の隣接ブロックだったスコープは組み替えない。
/// 調停の対象であったことは記録に残し、`reordered: false` で「指定どおり採用した」
/// ことを区別する（要件 2.4 の記録は「採用しなかった旨」を読み取れる形で返す）。
#[test]
fn t_zgp17_already_adjacent_block_is_recorded_as_not_reordered() {
    let (elements, normalizations) =
        expect_accept(&["balloon1", "surface1", "balloon0", "surface0"]);

    assert_eq!(elements, vec![b(1), s(1), b(0), s(0)], "要素列は指定のまま");
    assert_eq!(
        normalizations,
        vec![
            Normalization {
                scope: 1,
                reordered: false
            },
            Normalization {
                scope: 0,
                reordered: false
            }
        ]
    );
    assert_paired_scopes_form_balloon_first_blocks(&elements);
}

/// 数値モードでは正規化が何もしない。展開（⑷）が必ず `[Balloon, Char]` の隣接
/// ブロックを作るうえ、その並びは作者が書いた指定順ではないので、寄せるものも
/// 記録するものも無い（要件 2.4 は「明示モードの指定順が」と明示モードを名指しする）。
#[test]
fn t_zgp18_numeric_mode_normalization_is_a_no_op() {
    for tokens in [["1", "0"].as_slice(), ["2", "0", "1"].as_slice()] {
        let (elements, normalizations) = expect_accept(tokens);

        assert!(
            normalizations.is_empty(),
            "tokens={tokens:?} normalizations={normalizations:?}"
        );
        assert_paired_scopes_form_balloon_first_blocks(&elements);
    }

    let (elements, _) = expect_accept(&["2", "0", "1"]);
    assert_eq!(
        elements,
        vec![b(2), s(2), b(0), s(0), b(1), s(1)],
        "展開順は指定順のまま＝正規化は要素列にも触れない"
    );
}

/// 2 窓そろっていないスコープは書かれた位置のまま残し、記録も出さない。
/// 寄せる相手が居ないので調停そのものが起きないからである。
#[test]
fn t_zgp19_single_window_scope_is_left_in_place_without_record() {
    let (untouched, none) = expect_accept(&["b1", "s0"]);
    assert_eq!(untouched, vec![b(1), s(0)], "どちらのスコープも 1 枚だけ");
    assert!(none.is_empty(), "調停が起きないので記録も出ない: {none:?}");

    // 2 窓そろったスコープと 1 枚だけのスコープが同居する場合。
    let (mixed, normalizations) = expect_accept(&["b1", "s0", "s1"]);
    assert_eq!(
        mixed,
        vec![b(1), s(1), s(0)],
        "スコープ 1 だけが寄り、s0 は書かれた位置のまま後ろに残る"
    );
    assert_eq!(
        normalizations,
        vec![Normalization {
            scope: 1,
            reordered: true
        }],
        "記録は 2 窓そろったスコープの分だけ"
    );
    assert_paired_scopes_form_balloon_first_blocks(&mixed);
}

/// 正規化が動かすのは 1 スコープの 2 窓だけであり、他スコープの要素どうしの
/// 相対順は入力のまま保たれる（要件 2.5 の「属さない窓を動かさない」をグループの
/// 内側にも効かせる）。
#[test]
fn t_zgp20_other_scopes_keep_their_relative_order() {
    let (elements, normalizations) = expect_accept(&["b0", "b2", "s3", "s0"]);

    assert_eq!(
        elements,
        vec![b(0), s(0), b(2), s(3)],
        "b2 と s3 は 1 枚だけのスコープ＝互いの前後関係も書かれた順のまま"
    );
    assert_eq!(
        normalizations,
        vec![Normalization {
            scope: 0,
            reordered: true
        }]
    );
    assert_paired_scopes_form_balloon_first_blocks(&elements);
}

/// 完了状態の総取り: 反転・非隣接・両者の混在・数値モードのいずれを与えても、
/// 正規化後の要素列では 2 窓そろったどのスコープもバルーンが先の隣接ブロックになる。
#[test]
fn t_zgp21_every_paired_scope_is_adjacent_with_balloon_first_after_normalization() {
    for (tokens, expected_len) in [
        (["s1", "b1"].as_slice(), 2),
        (["b1", "s0", "s1", "b0"].as_slice(), 4),
        (["s0", "b2", "b0", "s2"].as_slice(), 4),
        (["b1", "s2", "s1", "b3", "s3", "b2"].as_slice(), 6),
        (["surface0", "b1", "balloon0", "s1"].as_slice(), 4),
        (["3", "1", "2"].as_slice(), 6),
    ] {
        let (elements, _) = expect_accept(tokens);

        assert_paired_scopes_form_balloon_first_blocks(&elements);
        assert_eq!(
            elements.len(),
            expected_len,
            "正規化は窓を増やしも減らしもしない: tokens={tokens:?} elements={elements:?}"
        );

        let unique: std::collections::HashSet<GroupElement> = elements.iter().copied().collect();
        assert_eq!(
            unique.len(),
            elements.len(),
            "正規化は窓を複製しない: tokens={tokens:?} elements={elements:?}"
        );
    }
}

/// 2 窓そろっていないスコープの要素は、**ペアブロックより前に書かれていても前のまま**
/// 残る。正規化が動かすのは 1 スコープの 2 窓だけだからである（要件 2.4 の調停は
/// スコープ内に閉じ、要件 2.5 の「属さない窓を動かさない」を破らない）。
///
/// [`t_zgp20_other_scopes_keep_their_relative_order`] の入力はどれも 1 枚だけの要素が
/// 最初のペアブロックより**後ろ**にあり、「1 枚だけの要素をまとめて末尾へ回す」実装と
/// 区別が付かなかった。ここでは 1 枚だけの要素を先頭に置いて、その差を赤にする。
/// 差は「どちらの窓が手前か」そのものであり、本機能が存在する理由に直結する。
#[test]
fn t_zgp22_single_window_scope_written_before_a_block_stays_before_it() {
    // s3 はペアを組むスコープ 0 のどの要素よりも前に書かれている。
    let (elements, normalizations) = expect_accept(&["s3", "b0", "b2", "s0"]);

    assert_eq!(
        elements,
        vec![s(3), b(0), s(0), b(2)],
        "s3 は先頭のまま・スコープ 0 のブロックは b0 が書かれた位置に立ち・b2 はその後ろ"
    );
    assert_eq!(
        normalizations,
        vec![Normalization {
            scope: 0,
            reordered: true
        }],
        "スコープ 0 は非隣接（間に b2）だったので指定順を採用していない"
    );
    assert_paired_scopes_form_balloon_first_blocks(&elements);

    // 最小形: ペアが既に隣接ブロックでも、前に書かれた 1 枚は前のまま。
    let (minimal, minimal_records) = expect_accept(&["s3", "b0", "s0"]);
    assert_eq!(minimal, vec![s(3), b(0), s(0)]);
    assert_eq!(
        minimal_records,
        vec![Normalization {
            scope: 0,
            reordered: false
        }],
        "指定どおりに採用できた場合も調停の対象であったことは記録に残る"
    );
    assert_paired_scopes_form_balloon_first_blocks(&minimal);
}
