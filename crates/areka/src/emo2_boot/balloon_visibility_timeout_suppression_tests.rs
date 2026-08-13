// =============================================================================
// 判断中核 `decide` のタイムアウト・抑止分岐の網羅表（task 6.2）
//
// design.md「Testing Strategy → Unit Tests ①」のうち、タイムアウトと抑止の系統を 1 つの表と
// して並べる。行は「フレームの並び → 各フレームに期待する事象と、そのフレーム終了時点の
// 状態」で、行が欠けていれば表を見ただけで分かる形にしてある。
//
// # 時刻の書き方（Requirements 9.2 / 9.3・task 6.2 の指定）
//
// 表の時刻は**すべて占有終端 [`ANCHOR`] からの相対**で書く。満了予定の初期確立式は
// `満了予定 = 占有終端 + 待ち時間` であって「計測が成り立った最初のフレームの現在時刻 +
// 待ち時間」ではないため（design「BalloonVisibilityState」の不変条件・Requirement 4.1）、
// 計測が成り立つ最初の観測は各行で占有終端から [`LATE`] 秒だけ遅れて訪れるようにしてある。
// 現在時刻を起点に取り違えた実装では満了予定が [`LATE`] 秒ぶん後ろへずれ、
// `ANCHOR + TIMEOUT` の行が満了しなくなって表が赤くなる。
//
// 注入する時刻は観測したい時点そのものだけで、そこで頭打ちにする。実時間の待機も、反復
// 回数だけで待機を有界化する形も一切用いない。
//
// # 既に押さえてある分岐はここで繰り返さない
//
// 列挙のうち次はいずれも `balloon_visibility_tests.rs`（task 3.2）が押さえている。本表は
// その**補集合**——同じ条件式の反対側の枝と、複数の条件が絡む組み合わせ——を並べる:
//
// - 満了境界の直前・直後と、満了予定の起点が占有終端であること
//   → `deadline_is_anchored_on_display_end_not_on_the_first_eligible_frame`
// - 満了時に可視 scope をまとめて畳み、無発話 scope を含めないこと
//   → `timeout_hides_every_visible_scope_and_leaves_the_silent_ones`
// - 同一フレームに表示した scope を満了の対象から外すこと
//   → `a_scope_shown_in_the_same_frame_is_not_hidden_by_the_expired_deadline`
// - 抑止の成立・解除・解除後の取り直しと、抑止中の超過の保留
//   → `suppression_holds_the_expired_deadline_and_release_restarts_the_measurement`
// - ドラッグ・選択肢が単独で抑止になること、不可視 scope への滞在が抑止にならないこと
//   → `drag_and_choice_suppress_but_hover_on_an_invisible_scope_does_not`
// - 抑止条件が 1 つも観測できないフレームを非抑止として扱うこと
//   → `unobservable_suppression_inputs_are_treated_as_not_suppressed`
// - 会話開始による計測の破棄 → `talk_started_discards_the_measurement_and_keeps_the_glyph_history`
// - 占有終端が現在時刻より未来へ動いたときの計測破棄
//   → `display_end_raised_into_the_future_discards_the_deadline`
// - 可視バルーンが無くなったときの計測破棄 → `clearing_every_balloon_discards_the_measurement`
// - 表示終了信号が届かない間の保持 → `no_measurement_starts_while_the_end_of_talk_signal_is_absent`
// - 占有終端・現在時刻が数でない場合の端の値
//   → `a_negative_display_end_is_rounded_up_to_zero_before_anchoring` ほか 2 本
//
// # 本表が新たに押さえる分岐
//
// ⑴ 占有終端は既知の最大より後退しない（重複・後退した信号は計測を動かさない）— 4.1
// ⑵ 占有終端の引き上げが現在時刻を越えないときは計測を破棄しない（破棄の反対側の枝）— 4.1 / 8.2
// ⑶ 会話が変わると表示終了信号の欠落の記録が武装し直る（会話につき 1 回の再武装）— 4.8 / 4.5
// ⑷ 満了予定に達する**前**の抑止は見送りの記録を出さない — 8.3 / 8.6
// ⑸ 抑止が二度成立すれば記録も二度出る（1 回の抑止につき 1 件の再武装）— 8.3
// ⑹ 抑止が解けても計測が成り立たなければ取り直さない（解除エッジの反対側の枝）— 5.3 / 4.8
// ⑺ 複数の抑止条件が同時に成立したら 1 件の記録へ全種別が載る — 5.1 / 5.2 / 5.4 / 8.3
// ⑻ 抑止条件の一部だけが観測不能でも、残りの条件は抑止として効く — 5.5
// ⑼ 本フレームに表示した scope への滞在は抑止になる — 5.2
// ⑽ 本フレームに全消去で消した scope への滞在は抑止にならない — 5.2 / 5.5
// ⑾ 現在時刻が分からないフレームは抑止の持ち越しを止めず、時刻が戻ったフレームで解除
//    エッジが改めて立つ — 4.9 / 5.3
// ⑿ 満了の対象が本フレーム表示分しか無いときは見送り、満了予定を残して次フレームで消す — 4.3
// ⒀ 満了で消した scope の持ち越し可視が偽になる — 8.1
// =============================================================================

use super::test_support::*;
use super::*;

// ---------------------------------------------------------------------------
// 表の座標系
// ---------------------------------------------------------------------------

/// 会話の占有終端（talk 相対秒）。表の時刻・信号・満了予定はすべてこの値からの相対で書く。
///
/// 0 以外の値にしてあるのは、相対で書いた期待値が絶対時刻の偶然の一致で通らないようにする
/// ためである。
const ANCHOR: f64 = 12.0;

/// タイムアウト時間（秒）。既定値の唯一の定義箇所から引く（Requirement 4.2——同じ数値を
/// 複数箇所へ散らさない）。各フレームへは [`Frame::timeout`] で明示に渡す。
const TIMEOUT: f64 = DEFAULT_BALLOON_TIMEOUT_SECS;

/// 計測が成り立つ最初の観測が占有終端から遅れて訪れる量（秒）。
///
/// **0 にしてはならない**——観測はフレーム単位で飛び飛びに入るという前提そのものを表す量で
/// あり、これが 0 だと「占有終端起点」と「最初の観測の現在時刻起点」が同じ値になって、
/// 起点の取り違えを表が検出できなくなる。
const LATE: f64 = 7.0;

/// 満了境界の直前を表す量（秒）。
const EPS: f64 = 0.25;

// ---------------------------------------------------------------------------
// 表の記述語
// ---------------------------------------------------------------------------

/// 抑止条件 1 つ分の観測。
#[derive(Debug, Clone, Copy)]
enum Watch {
    /// 観測できて不成立。
    Off,
    /// 観測できて成立。
    On,
    /// 観測が取れなかった（Requirement 5.5——抑止しない側へ倒す入力）。
    Blind,
}

impl Watch {
    /// 観測値へ落とす。
    fn observed(self) -> Option<bool> {
        match self {
            Watch::Off => Some(false),
            Watch::On => Some(true),
            Watch::Blind => None,
        }
    }
}

/// 表の観測 1 件: `(scope, 可視グリフ数, 現に可視か, ポインタ滞在, 選択肢表示)`。
type Obs = (u32, usize, bool, Watch, Watch);

/// 本フレームに届く表示ライフサイクル信号。
#[derive(Debug, Clone, Copy)]
enum Signal {
    /// 会話の開始。
    TalkStarted,
    /// 占有終端（[`ANCHOR`] からの相対秒）。
    DisplayEnd(f64),
}

/// 本フレームの現在時刻。
#[derive(Debug, Clone, Copy)]
enum Now {
    /// [`ANCHOR`] からの相対秒。
    At(f64),
    /// 現在時刻が分からないフレーム（talk の起点が未確立）。
    Unknown,
}

/// 1 フレームに期待する事象。並びは **`decide` が返すログの並びそのもの**を書く。
///
/// 行動列はここから規則で導く（下記 [`expand`]）——ログと行動で並びの規約が別だからで、
/// 同じ記述から独立に導いて双方を突き合わせる。
#[derive(Debug, Clone, Copy)]
enum Expect {
    /// 当該 scope を表示する（契機＝可視コンテンツの配置）。
    Shown(u32),
    /// 列挙した scope をまとめて非表示にする（契機＝内容の全消去）。
    Cleared(&'static [u32]),
    /// 列挙した scope をまとめて非表示にする（契機＝タイムアウト）。
    TimedOut(&'static [u32]),
    /// 計測を開始した（値は [`ANCHOR`] からの相対秒）。
    Started { display_end: f64, deadline: f64 },
    /// 計測を破棄した（`deadline` は [`ANCHOR`] からの相対秒）。
    Discarded {
        reason: MeasurementDiscardReason,
        deadline: f64,
    },
    /// 抑止の解除に伴い計測をやり直した（値は [`ANCHOR`] からの相対秒）。
    Restarted { now: f64, deadline: f64 },
    /// 抑止のため非表示を見送った（`deadline` は [`ANCHOR`] からの相対秒）。
    Suppressed {
        kinds: SuppressionKinds,
        deadline: f64,
    },
    /// 可視コンテンツが現れたのに表示終了信号が 1 件も届いていない。
    SignalMissing,
}

/// フレーム実行後に確かめる状態。
#[derive(Debug, Clone, Copy)]
enum After {
    /// 満了予定が立っている（[`ANCHOR`] からの相対秒）。
    Deadline(f64),
    /// 満了予定が無い。
    NoDeadline,
    /// 当該 scope の持ち越し可視。
    PrevVisible(u32, bool),
}

/// 表の 1 フレーム。
struct Row {
    /// 本フレームに届く信号（到着順）。
    signals: &'static [Signal],
    /// 本フレームの観測。
    scopes: &'static [Obs],
    /// いずれかのバルーン窓がドラッグ中か（Requirement 5.1）。
    dragging: bool,
    /// 本フレームの現在時刻。
    now: Now,
    /// 期待する事象（ログの並びそのもの）。
    expect: &'static [Expect],
    /// 期待するフレーム終了時点の状態。
    after: &'static [After],
}

/// 表の 1 行（1 つのフレーム列）。
struct Case {
    name: &'static str,
    rows: &'static [Row],
}

// ---------------------------------------------------------------------------
// 期待記述の展開と走査
// ---------------------------------------------------------------------------

/// 期待記述から、そのフレームの行動列とログ列を組み立てる。
///
/// ログは記述の並びをそのまま採る（＝表に書いた並びが期待そのもの）。行動は `decide` の
/// 規約どおり「全消去の非表示 → 表示 → 満了の非表示」へ並べ替える。相対秒はここで
/// [`ANCHOR`] を足して絶対時刻へ直す。
fn expand(expect: &[Expect]) -> (Vec<VisibilityAction>, Vec<VisibilityLogEvent>) {
    let mut cleared: Vec<VisibilityAction> = Vec::new();
    let mut shown: Vec<VisibilityAction> = Vec::new();
    let mut timed_out: Vec<VisibilityAction> = Vec::new();
    let mut logs: Vec<VisibilityLogEvent> = Vec::new();

    for &entry in expect {
        match entry {
            Expect::Shown(scope) => {
                shown.push(VisibilityAction::Show { scope });
                logs.push(VisibilityLogEvent::Transition {
                    scope,
                    trigger: VisibilityTrigger::Content,
                    visible: true,
                });
            }
            Expect::Cleared(scopes) => {
                cleared.push(VisibilityAction::HideScopes {
                    scopes: scopes.to_vec(),
                    trigger: VisibilityTrigger::Clear,
                });
                logs.extend(scopes.iter().map(|&scope| VisibilityLogEvent::Transition {
                    scope,
                    trigger: VisibilityTrigger::Clear,
                    visible: false,
                }));
            }
            Expect::TimedOut(scopes) => {
                timed_out.push(VisibilityAction::HideScopes {
                    scopes: scopes.to_vec(),
                    trigger: VisibilityTrigger::Timeout,
                });
                logs.extend(scopes.iter().map(|&scope| VisibilityLogEvent::Transition {
                    scope,
                    trigger: VisibilityTrigger::Timeout,
                    visible: false,
                }));
            }
            Expect::Started {
                display_end,
                deadline,
            } => logs.push(VisibilityLogEvent::MeasurementStarted {
                display_end: ANCHOR + display_end,
                deadline: ANCHOR + deadline,
            }),
            Expect::Discarded { reason, deadline } => {
                logs.push(VisibilityLogEvent::MeasurementDiscarded {
                    reason,
                    deadline: ANCHOR + deadline,
                })
            }
            Expect::Restarted { now, deadline } => {
                logs.push(VisibilityLogEvent::MeasurementRestarted {
                    now: ANCHOR + now,
                    deadline: ANCHOR + deadline,
                })
            }
            Expect::Suppressed { kinds, deadline } => {
                logs.push(VisibilityLogEvent::TimeoutSuppressed {
                    kinds,
                    deadline: ANCHOR + deadline,
                })
            }
            Expect::SignalMissing => logs.push(VisibilityLogEvent::DisplayEndSignalMissing),
        }
    }

    let actions = cleared
        .into_iter()
        .chain(shown)
        .chain(timed_out)
        .collect::<Vec<_>>();
    (actions, logs)
}

/// 1 行分のフレーム列を流し、食い違いを積む（最初の 1 件で止めない——1 度の実行で赤くなる
/// 行を全て数え上げられるようにするため）。
fn run(case: &Case, failures: &mut Vec<String>) {
    let mut state = BalloonVisibilityState::default();

    for (index, row) in case.rows.iter().enumerate() {
        let entries: Vec<(u32, ScopeObservation)> = row
            .scopes
            .iter()
            .map(|&(scope, glyphs, visible, hover, choice)| {
                (
                    scope,
                    ScopeObservation {
                        hover: hover.observed(),
                        choice_active: choice.observed(),
                        ..seen(glyphs, visible)
                    },
                )
            })
            .collect();

        let mut frame = Frame::new(&entries).timeout(TIMEOUT);
        if let Now::At(rel) = row.now {
            frame = frame.at(ANCHOR + rel);
        }
        if row.dragging {
            frame = frame.dragging();
        }
        for signal in row.signals {
            frame = match *signal {
                Signal::TalkStarted => frame.talk_started(),
                Signal::DisplayEnd(rel) => frame.display_end(ANCHOR + rel),
            };
        }

        let decision = frame.run(&mut state);
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

        for &check in row.after {
            let mismatch = match check {
                After::Deadline(rel) => (state.deadline != Some(ANCHOR + rel)).then(|| {
                    format!(
                        "満了予定が {:?} ではなく {:?}",
                        Some(ANCHOR + rel),
                        state.deadline
                    )
                }),
                After::NoDeadline => state
                    .deadline
                    .map(|deadline| format!("満了予定が残っている（{deadline:?}）")),
                After::PrevVisible(scope, expected) => match state.per_scope.get(&scope) {
                    Some(previous) if previous.prev_visible == expected => None,
                    Some(previous) => Some(format!(
                        "scope {scope} の持ち越し可視が {expected} ではなく {}",
                        previous.prev_visible
                    )),
                    None => Some(format!("scope {scope} の状態が生えていない")),
                },
            };
            if let Some(detail) = mismatch {
                failures.push(format!("{} / フレーム {index}: {detail}", case.name));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 表
// ---------------------------------------------------------------------------

// 表の本体は隣のファイルへ置く。記述語と走査だけで本ファイルが埋まっており、47 フレーム分の
// 表を同居させるとリポジトリの 1 ファイル 1,000 行の上限を越えるためである（design の
// File Structure Plan が「テーマ超過時はさらに分割」と想定している形）。
#[path = "balloon_visibility_timeout_suppression_cases.rs"]
mod cases;

use cases::CASES;

/// 表の全行を流す。1 度の実行で、赤くなった行と該当フレームが全て並ぶ。
#[test]
fn timeout_and_suppression_branches_hold_for_every_case() {
    let mut failures = Vec::new();
    for case in CASES {
        run(case, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "タイムアウト・抑止の分岐が期待と食い違った:\n{}",
        failures.join("\n")
    );
}

/// 較正: 期待記述が行動列・ログ列へ展開される規則そのものを、既知の 1 件で押さえる。
/// ここが壊れると表の全行が「何も比べない」空虚な行になる。
#[test]
fn expectation_expansion_follows_both_ordering_rules() {
    let (actions, logs) = expand(&[
        Expect::TimedOut(&[2]),
        Expect::Shown(1),
        Expect::Cleared(&[0]),
    ]);
    assert_eq!(
        actions,
        vec![
            VisibilityAction::HideScopes {
                scopes: vec![0],
                trigger: VisibilityTrigger::Clear,
            },
            VisibilityAction::Show { scope: 1 },
            VisibilityAction::HideScopes {
                scopes: vec![2],
                trigger: VisibilityTrigger::Timeout,
            },
        ],
        "行動は「全消去の非表示 → 表示 → 満了の非表示」へ並べ替える"
    );
    assert_eq!(
        logs,
        vec![
            VisibilityLogEvent::Transition {
                scope: 2,
                trigger: VisibilityTrigger::Timeout,
                visible: false,
            },
            VisibilityLogEvent::Transition {
                scope: 1,
                trigger: VisibilityTrigger::Content,
                visible: true,
            },
            VisibilityLogEvent::Transition {
                scope: 0,
                trigger: VisibilityTrigger::Clear,
                visible: false,
            },
        ],
        "ログは記述の並びをそのまま採る"
    );
}

/// 較正: 相対秒が [`ANCHOR`] を足して絶対時刻へ直されていること。ここが素通しだと、
/// 表の時刻がすべて 0 起点になって「占有終端相対で書く」という表の前提が失われる。
#[test]
fn expectation_times_are_anchored_on_the_display_end() {
    let (_, logs) = expand(&[Expect::Started {
        display_end: 0.0,
        deadline: TIMEOUT,
    }]);
    assert_eq!(
        logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            display_end: ANCHOR,
            deadline: ANCHOR + TIMEOUT,
        }]
    );
    assert_ne!(
        ANCHOR, 0.0,
        "起点が 0 だと相対と絶対の取り違えが検出できない"
    );
    assert_ne!(
        LATE, 0.0,
        "最初の観測が遅れて訪れないと起点の取り違えを検出できない"
    );
}

/// 較正: 走査が実際に突き合わせていること——期待と異なる結果を与えたら、行動・ログ・状態の
/// 3 つ全てで食い違いが積まれる。これが 0 件なら表は緑を返すだけの飾りである。
#[test]
fn the_table_runner_reports_mismatches_instead_of_passing_silently() {
    let calibration = Case {
        name: "較正",
        rows: &[Row {
            signals: &[Signal::TalkStarted, Signal::DisplayEnd(0.0)],
            scopes: &[(0, 3, false, Watch::Off, Watch::Off)],
            dragging: false,
            now: Now::At(-5.0),
            // 実際には表示が起き、満了予定は立たない。
            expect: &[],
            after: &[After::Deadline(TIMEOUT)],
        }],
    };

    let mut failures = Vec::new();
    run(&calibration, &mut failures);
    assert_eq!(
        failures.len(),
        3,
        "表示が起きるフレームへ「何も起きない」を期待させたのに食い違いが積まれない: {failures:?}"
    );
}
