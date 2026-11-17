// =============================================================================
// 判断中核 `decide` のコンテンツ駆動分岐の網羅表（task 6.1）
//
// design.md「Testing Strategy → Unit Tests ①」のうちコンテンツ駆動の系統を、1 つの表として
// 並べる。行は「観測列（フレームの並び）→ 各フレームに期待する遷移」で、行が欠けていれば
// 表を見ただけで分かる形にしてある。時刻は 1 つも注入しない（`step` は現在時刻なしで進める）
// ため、タイムアウトの判定はどの行でも動かない——実時間の待機も反復回数だけの有界化も無い
// （Requirement 9.1 / 9.2 / 9.3）。
//
// 列挙のうち **既に押さえてある分岐はここで繰り返さない**。対応は次のとおり（いずれも
// `balloon_visibility_tests.rs`）:
//
// - 可視コンテンツの有無（文字あり／改行・カーソル移動・待機・消去のみ）
//   → `flat_glyph_count_is_never_a_show_trigger`／`partial_decrease_is_not_a_hide_trigger`
// - scope 別の表示と無発話 scope の非表示
//   → `first_visible_content_shows_only_that_scope`／
//     `scope_switch_adds_show_without_touching_the_shown_scope`
// - 同一 scope の 2 つ目以降で表示指令を再発行しないこと
//   → `additional_content_on_a_visible_scope_does_not_reissue_show`
// - 全消去に連動する全非表示と、その直後の再出現
//   → `descent_to_zero_hides_every_visible_scope`／`descent_to_zero_on_an_invisible_scope_is_silent`／
//     `content_after_a_hide_shows_again_by_the_same_rule`
// - 同一の観測列が同一の遷移列を返すこと → `identical_observation_sequences_yield_identical_decisions`
//
// 本表が新たに押さえるのは次の 4 点である:
//
// ⑴ 明示指令との相互作用のうち**判断中核が表現できる側**——外因の可視性変化は、観測の
//    `visible` が本制御の記憶と食い違うフレームとしてしか現れない。`\b[-1]` で消えたあとの
//    新規コンテンツによる再表示（Requirements 2.1 / 3.2 / 4.7 と同一規則）と、外因で可視に
//    なった scope へコンテンツが置かれても表示を再発行しないこと（Requirement 2.5 / 6.2 の向き）。
//    外因であることの**検出とログ**（`trigger=explicit`）は配線層の担当で、
//    `balloon_visibility_phase_tests.rs::external_hide_is_logged_as_explicit_and_clears_hover`
//    が既に押さえている。
// ⑵ 表示の契機が「可視グリフ数の増加」であってゼロ起点の増加に限らないこと——明示指令で
//    消えたバルーンには内容が残っており、次の文字はゼロからの増加にならない。
// ⑶ 同一フレームに全消去と表示が同居したときの並び——行動は「全消去の非表示 → 表示」、
//    ログは scope 昇順であり、両者は別の規約である（`decide` の並びの定義）。
// ⑷ 内容を一度も置いていない可視 scope を全消去の対象に含めないこと（Requirement 2.2——
//    他 scope の全消去へ連動させない）。
// =============================================================================

use super::test_support::*;
use super::*;

/// 1 フレームに期待する遷移（表の記述語）。
#[derive(Debug, Clone, Copy)]
enum Expected {
    /// 当該 scope を表示する（契機＝可視コンテンツの配置）。
    Shown(u32),
    /// 列挙した scope をまとめて非表示にする（契機＝内容の全消去）。
    Cleared(&'static [u32]),
}

/// 表の 1 フレーム。観測は scope → (可視グリフ数, 現に可視か) で、抑止条件はいずれも
/// 「観測できて不成立」（`seen`）。
struct Row {
    scopes: &'static [(u32, usize, bool)],
    expect: &'static [Expected],
}

/// 表の 1 行（1 つの観測列）。
struct Case {
    name: &'static str,
    rows: &'static [Row],
}

/// 期待記述から、そのフレームの行動列とログ列を組み立てる。
///
/// 行動は記述の順序をそのまま採り（＝表に書いた並びが期待そのもの）、ログは scope 昇順へ
/// 並べ替える。`decide` は 2 つの並びに別々の規約を置いているため、同じ記述から独立に導いて
/// 双方を突き合わせる。
fn expand(expect: &[Expected]) -> (Vec<VisibilityAction>, Vec<VisibilityLogEvent>) {
    let actions = expect
        .iter()
        .map(|&entry| match entry {
            Expected::Shown(scope) => VisibilityAction::Show { scope },
            Expected::Cleared(scopes) => VisibilityAction::HideScopes {
                scopes: scopes.to_vec(),
                trigger: VisibilityTrigger::Clear,
            },
        })
        .collect();

    let mut logs: Vec<(u32, VisibilityLogEvent)> = Vec::new();
    for &entry in expect {
        match entry {
            Expected::Shown(scope) => logs.push((
                scope,
                VisibilityLogEvent::Transition {
                    scope,
                    trigger: VisibilityTrigger::Content,
                    visible: true,
                },
            )),
            Expected::Cleared(scopes) => logs.extend(scopes.iter().map(|&scope| {
                (
                    scope,
                    VisibilityLogEvent::Transition {
                        scope,
                        trigger: VisibilityTrigger::Clear,
                        visible: false,
                    },
                )
            })),
        }
    }
    logs.sort_by_key(|&(scope, _)| scope);

    (actions, logs.into_iter().map(|(_, event)| event).collect())
}

/// 1 行分の観測列を流し、食い違いを積む（最初の 1 件で止めない——1 度の実行で赤くなる行を
/// 全て数え上げられるようにするため）。
fn run(case: &Case, failures: &mut Vec<String>) {
    let mut state = BalloonVisibilityState::default();
    for (index, row) in case.rows.iter().enumerate() {
        let entries: Vec<(u32, ScopeObservation)> = row
            .scopes
            .iter()
            .map(|&(scope, glyphs, visible)| (scope, seen(glyphs, visible)))
            .collect();
        let decision = step(&mut state, &entries);
        let (actions, logs) = expand(row.expect);

        if decision.actions != actions {
            failures.push(format!(
                "{} / フレーム {index}: 行動が {actions:?} ではなく {:?}",
                case.name, decision.actions
            ));
        }
        if decision.logs != logs {
            failures.push(format!(
                "{} / フレーム {index}: ログが {logs:?} ではなく {:?}",
                case.name, decision.logs
            ));
        }
    }
}

const CASES: &[Case] = &[
    // ⑴⑵ 明示指令で消えたあと、残った内容へ更に文字が置かれる。
    //
    // `\b[-1]` は内容を消さずに可視性だけを落とすため、次の文字はゼロからの増加にならない。
    // 表示の契機を「ゼロから 1 文字目が置かれたこと」と取り違えると、明示指令のあとバルーンが
    // 二度と出てこない。
    Case {
        name: "明示指令で消えたあと残った内容へ文字が加わると出直す",
        rows: &[
            // 装着直後（内容なし・不可視）。
            Row {
                scopes: &[(0, 0, false)],
                expect: &[],
            },
            // 最初の可視コンテンツ。
            Row {
                scopes: &[(0, 3, false)],
                expect: &[Expected::Shown(0)],
            },
            // 表示が届いた次のフレーム。
            Row {
                scopes: &[(0, 3, true)],
                expect: &[],
            },
            // 外因の非表示（`\b[-1]`）。内容は残ったまま可視だけが落ちる。
            Row {
                scopes: &[(0, 3, false)],
                expect: &[],
            },
            // 次の文字が置かれた。ゼロ起点でない増加でも表示の契機になる。
            Row {
                scopes: &[(0, 5, false)],
                expect: &[Expected::Shown(0)],
            },
        ],
    },
    // ⑴ 明示指令で消えたあとの全消去は契機にならず、次の会話の内容で出直す。
    Case {
        name: "明示指令で消えたあとの全消去は無音で、次の内容で出直す",
        rows: &[
            Row {
                scopes: &[(0, 4, false)],
                expect: &[Expected::Shown(0)],
            },
            // 外因の非表示。
            Row {
                scopes: &[(0, 4, false)],
                expect: &[],
            },
            // 全消去。既に不可視なので非表示の契機にならない（消す対象が無い）。
            Row {
                scopes: &[(0, 0, false)],
                expect: &[],
            },
            // 次の会話の可視コンテンツ。Requirement 3.2 / 4.7 と同一規則で出直す。
            Row {
                scopes: &[(0, 2, false)],
                expect: &[Expected::Shown(0)],
            },
        ],
    },
    // ⑴ 本制御が発行していない可視化（明示指令・面切替の結果など）を観測したフレーム。
    //
    // 可視かどうかの真実源は観測であり、既に可視である以上コンテンツの増加は契機にならない
    // （Requirement 2.5）。最後の行は、この無音が「そもそも何も起きない scope」で成り立つ
    // 恒真でないことの較正——同じ scope で全消去が確かに成立する。
    Case {
        name: "外から可視になった scope へ内容が置かれても表示を再発行しない",
        rows: &[
            Row {
                scopes: &[(0, 0, false)],
                expect: &[],
            },
            // 外因の可視化（内容はまだ無い）。
            Row {
                scopes: &[(0, 0, true)],
                expect: &[],
            },
            // コンテンツが置かれる。既に可視なので表示は発行しない。
            Row {
                scopes: &[(0, 4, true)],
                expect: &[],
            },
            // 較正: 全消去は可視である以上ここで成立する。
            Row {
                scopes: &[(0, 0, true)],
                expect: &[Expected::Cleared(&[0])],
            },
        ],
    },
    // ⑶ 同一フレームでの全消去と表示。行動は「全消去の非表示 → 表示」の並び、ログは
    // scope 昇順。この行では両者の並びが一致するため、次の行と対で規約の違いが立つ。
    Case {
        name: "同一フレームの全消去と表示（消える側が小さい scope）",
        rows: &[
            Row {
                scopes: &[(0, 3, false), (1, 0, false)],
                expect: &[Expected::Shown(0)],
            },
            Row {
                scopes: &[(0, 3, true), (1, 0, false)],
                expect: &[],
            },
            // scope 0 の内容が消え、同じフレームで scope 1 が喋り出す。
            Row {
                scopes: &[(0, 0, true), (1, 2, false)],
                expect: &[Expected::Cleared(&[0]), Expected::Shown(1)],
            },
        ],
    },
    // ⑶ 消える側が大きい scope の場合。行動は「全消去 → 表示」のまま（scope 1 の非表示が
    // scope 0 の表示より前）だが、ログは scope 昇順なので表示のログが先に来る。
    Case {
        name: "同一フレームの全消去と表示（消える側が大きい scope）",
        rows: &[
            Row {
                scopes: &[(0, 0, false), (1, 3, false)],
                expect: &[Expected::Shown(1)],
            },
            Row {
                scopes: &[(0, 0, false), (1, 3, true)],
                expect: &[],
            },
            Row {
                scopes: &[(0, 2, false), (1, 0, true)],
                expect: &[Expected::Cleared(&[1]), Expected::Shown(0)],
            },
        ],
    },
    // ⑷ 内容を一度も置いていない可視 scope は、他 scope の全消去に連動させない
    //    （Requirement 2.2）。非表示の契機はあくまで**その scope 自身の**ゼロへの下降である。
    Case {
        name: "内容を一度も置いていない可視 scope は全消去の対象に含めない",
        rows: &[
            Row {
                scopes: &[(0, 3, false), (1, 0, false)],
                expect: &[Expected::Shown(0)],
            },
            // scope 1 は外因で可視になったが、内容は 1 つも置かれていない。
            Row {
                scopes: &[(0, 3, true), (1, 0, true)],
                expect: &[],
            },
            // scope 0 の全消去。対象は scope 0 だけ。
            Row {
                scopes: &[(0, 0, true), (1, 0, true)],
                expect: &[Expected::Cleared(&[0])],
            },
        ],
    },
];

/// 表の全行を流す。1 度の実行で、赤くなった行と該当フレームが全て並ぶ。
#[test]
fn content_driven_branches_hold_for_every_case() {
    let mut failures = Vec::new();
    for case in CASES {
        run(case, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "コンテンツ駆動の分岐が期待と食い違った:\n{}",
        failures.join("\n")
    );
}

/// 較正: 期待記述が行動列・ログ列へ展開される規則そのものを、既知の 1 件で押さえる。
/// ここが壊れると表の全行が「何も比べない」空虚な行になる。
#[test]
fn expectation_expansion_follows_both_ordering_rules() {
    let (actions, logs) = expand(&[Expected::Cleared(&[1]), Expected::Shown(0)]);
    assert_eq!(
        actions,
        vec![
            VisibilityAction::HideScopes {
                scopes: vec![1],
                trigger: VisibilityTrigger::Clear,
            },
            VisibilityAction::Show { scope: 0 },
        ],
        "行動は記述の並びをそのまま採る"
    );
    assert_eq!(
        logs,
        vec![
            VisibilityLogEvent::Transition {
                scope: 0,
                trigger: VisibilityTrigger::Content,
                visible: true,
            },
            VisibilityLogEvent::Transition {
                scope: 1,
                trigger: VisibilityTrigger::Clear,
                visible: false,
            },
        ],
        "ログは scope 昇順へ並べ替える"
    );
}

/// 較正: 走査が実際に突き合わせていること——期待と異なる結果を与えたら、行動とログの双方で
/// 食い違いが積まれる。これが 0 件なら表は緑を返すだけの飾りである。
#[test]
fn the_table_runner_reports_mismatches_instead_of_passing_silently() {
    let calibration = Case {
        name: "較正",
        rows: &[Row {
            scopes: &[(0, 3, false)],
            expect: &[],
        }],
    };

    let mut failures = Vec::new();
    run(&calibration, &mut failures);
    assert_eq!(
        failures.len(),
        2,
        "表示が起きるフレームへ「何も起きない」を期待させたのに食い違いが積まれない: {failures:?}"
    );
}
