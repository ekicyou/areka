// =============================================================================
// 判断中核 `decide` の決定論テスト（task 3.1・コンテンツ駆動の表示・非表示）
//
// design.md「Testing Strategy → Unit Tests ①」のうち、本タスクが所有する系統:
//   ⑴ 可視コンテンツ判定（グリフあり／改行・カーソル移動・待機・消去のみ）— Requirement 2.3
//   ⑵ scope 別の表示と無発話 scope の非表示 — Requirements 2.1 / 2.2 / 2.4 / 2.7 / 3.3
//   ⑶ 表示指令を再発行しない — Requirement 2.5
//   ⑷ ゼロへの下降に伴う全非表示と、その直後の再出現 — Requirements 3.1 / 3.2 / 3.6 / 4.7
//   ⑸ 遷移の無いフレームは 1 件もログを生成しない — Requirement 8.6
//   ⑹ 同一の観測列は同一の遷移列を返す（走査順に依存しない） — Requirement 9.1 の前提
//
// タイムアウト計測・抑止（task 3.2）の系統:
//   ⑺ 満了予定の起点＝占有終端／満了境界の直前・直後 — Requirements 4.1 / 4.3 / 4.6
//   ⑻ 抑止の成立・解除・解除後の取り直し・抑止中の超過の保留 — Requirements 5.1〜5.3 / 5.6
//   ⑼ 抑止の観測不能は非抑止扱い — Requirement 5.5
//   ⑽ 会話開始・占有終端の引き上げによる計測破棄 — Requirements 4.5 / 8.2
//   ⑾ 表示終了信号が無い間は計測を始めない — Requirement 4.8
//
// 実時間の待機・反復回数のみによる有界化は一切用いない（Requirement 9.2 / 9.3）。時刻は
// 観測したい時点そのものだけを注入し、そこで頭打ちにする。
// =============================================================================

use super::test_support::*;
use super::*;

/// 期待する表示行動。
fn show(scope: u32) -> VisibilityAction {
    VisibilityAction::Show { scope }
}

/// 期待する非表示行動（内容の全消去が契機）。
fn hide_cleared(scopes: &[u32]) -> VisibilityAction {
    VisibilityAction::HideScopes {
        scopes: scopes.to_vec(),
        trigger: VisibilityTrigger::Clear,
    }
}

/// 期待する表示ログ。
fn shown_log(scope: u32) -> VisibilityLogEvent {
    VisibilityLogEvent::Transition {
        scope,
        trigger: VisibilityTrigger::Content,
        visible: true,
    }
}

/// 期待する非表示ログ。
fn hidden_log(scope: u32) -> VisibilityLogEvent {
    VisibilityLogEvent::Transition {
        scope,
        trigger: VisibilityTrigger::Clear,
        visible: false,
    }
}

/// 期待する非表示行動（タイムアウトが契機）。
fn hide_timed_out(scopes: &[u32]) -> VisibilityAction {
    VisibilityAction::HideScopes {
        scopes: scopes.to_vec(),
        trigger: VisibilityTrigger::Timeout,
    }
}

/// 期待するタイムアウト非表示ログ。
fn timed_out_log(scope: u32) -> VisibilityLogEvent {
    VisibilityLogEvent::Transition {
        scope,
        trigger: VisibilityTrigger::Timeout,
        visible: false,
    }
}

// ---------------------------------------------------------------------------
// ⑵ scope 別の表示
// ---------------------------------------------------------------------------

/// 最初の可視コンテンツが置かれた scope **だけ**が表示される（Requirement 2.1 / 2.2）。
#[test]
fn first_visible_content_shows_only_that_scope() {
    let mut state = BalloonVisibilityState::default();

    // 装着直後: 双方とも内容なし・不可視。
    let quiet = step(&mut state, &[(0, seen(0, false)), (1, seen(0, false))]);
    assert!(quiet.actions.is_empty(), "内容が無いフレームで発行が起きた");
    assert!(quiet.logs.is_empty(), "内容が無いフレームでログが出た");

    // scope 0 だけが喋り出した。
    let d = step(&mut state, &[(0, seen(3, false)), (1, seen(0, false))]);
    assert_eq!(d.actions, vec![show(0)]);
    assert_eq!(d.logs, vec![shown_log(0)]);
}

/// 会話中に発話 scope が切り替わっても、先に表示済みの scope は非表示へ戻らない
/// （Requirement 2.4）。切替後も喋らない scope は最後まで出ない（Requirement 2.7 / 3.3）。
#[test]
fn scope_switch_adds_show_without_touching_the_shown_scope() {
    let mut state = BalloonVisibilityState::default();
    let d0 = step(
        &mut state,
        &[
            (0, seen(4, false)),
            (1, seen(0, false)),
            (2, seen(0, false)),
        ],
    );
    assert_eq!(d0.actions, vec![show(0)]);

    // scope 1 が喋り出す。scope 0 は可視のまま据え置き、scope 2 は無発話のまま。
    let d1 = step(
        &mut state,
        &[(0, seen(4, true)), (1, seen(2, false)), (2, seen(0, false))],
    );
    assert_eq!(d1.actions, vec![show(1)]);
    assert_eq!(d1.logs, vec![shown_log(1)]);
}

/// 同一 scope へ 2 つ目以降の可視コンテンツが置かれても表示指令を再発行しない
/// （Requirement 2.5）。既に可視である以上、増加エッジだけでは契機にならない。
#[test]
fn additional_content_on_a_visible_scope_does_not_reissue_show() {
    let mut state = BalloonVisibilityState::default();
    assert_eq!(
        step(&mut state, &[(0, seen(1, false))]).actions,
        vec![show(0)]
    );

    for glyphs in [2usize, 3, 7] {
        let d = step(&mut state, &[(0, seen(glyphs, true))]);
        assert!(
            d.actions.is_empty(),
            "可視 scope への追加コンテンツで再発行が起きた（glyphs={glyphs}）"
        );
        assert!(
            d.logs.is_empty(),
            "遷移が無いのにログが出た（glyphs={glyphs}）"
        );
    }
}

// ---------------------------------------------------------------------------
// ⑴ 可視コンテンツ判定
// ---------------------------------------------------------------------------

/// 可視グリフ数が据え置きのフレームは表示の契機にならない。改行・カーソル移動・待機・
/// 内容消去だけの観測列は可視グリフ数を動かさないため、この規則が Requirement 2.3 を成す。
#[test]
fn flat_glyph_count_is_never_a_show_trigger() {
    let mut state = BalloonVisibilityState::default();

    // 改行・カーソル移動・待機・消去だけが続く不可視の scope（可視グリフ数はゼロのまま）。
    for _ in 0..4 {
        let d = step(&mut state, &[(0, seen(0, false))]);
        assert!(d.actions.is_empty(), "可視コンテンツが無いのに表示が起きた");
        assert!(d.logs.is_empty(), "遷移が無いのにログが出た");
    }

    // 一度喋ったあとで、また据え置きが続く場合も同じ（不可視へ戻されはしない）。
    assert_eq!(
        step(&mut state, &[(0, seen(5, false))]).actions,
        vec![show(0)]
    );
    for _ in 0..3 {
        let d = step(&mut state, &[(0, seen(5, true))]);
        assert!(d.actions.is_empty(), "据え置きのフレームで発行が起きた");
        assert!(d.logs.is_empty(), "据え置きのフレームでログが出た");
    }
}

/// 可視グリフ数の**ゼロ以外への**下降は非表示の契機にならない（部分消去で消さない）。
#[test]
fn partial_decrease_is_not_a_hide_trigger() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(5, false))]);

    let d = step(&mut state, &[(0, seen(3, true))]);
    assert!(d.actions.is_empty(), "ゼロ以外への下降で非表示が起きた");
    assert!(d.logs.is_empty(), "遷移が無いのにログが出た");
}

// ---------------------------------------------------------------------------
// ⑷ ゼロへの下降と再出現
// ---------------------------------------------------------------------------

/// 会話冒頭の全消去で、表示中の全 scope が 1 つの行動へまとめて畳まれる
/// （Requirement 3.1）。どの scope から会話が始まるかを知らずに導ける（Requirement 3.6）。
#[test]
fn descent_to_zero_hides_every_visible_scope() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false)), (1, seen(0, false))]);
    step(&mut state, &[(0, seen(3, true)), (1, seen(2, false))]);

    let d = step(&mut state, &[(0, seen(0, true)), (1, seen(0, true))]);
    assert_eq!(d.actions, vec![hide_cleared(&[0, 1])]);
    assert_eq!(d.logs, vec![hidden_log(0), hidden_log(1)]);
}

/// 既に不可視の scope がゼロへ下降しても遷移ではない——行動もログも生成しない
/// （Requirement 8.6）。
#[test]
fn descent_to_zero_on_an_invisible_scope_is_silent() {
    let mut state = BalloonVisibilityState::default();
    // 可視化を発行しないまま（不可視のまま）内容だけが増減する scope。
    step(&mut state, &[(0, seen(4, true))]);

    let d = step(&mut state, &[(0, seen(0, false))]);
    assert!(d.actions.is_empty(), "不可視 scope へ非表示が発行された");
    assert!(d.logs.is_empty(), "遷移が無いのにログが出た");
}

/// 全非表示の直後に置かれた可視コンテンツは、表示と同一の規則でそのまま出直す
/// （Requirement 3.2）。タイムアウト非表示のあとの再表示も同じ規則である（Requirement 4.7）。
#[test]
fn content_after_a_hide_shows_again_by_the_same_rule() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);
    let cleared = step(&mut state, &[(0, seen(0, true))]);
    assert_eq!(cleared.actions, vec![hide_cleared(&[0])]);

    let d = step(&mut state, &[(0, seen(2, false))]);
    assert_eq!(d.actions, vec![show(0)]);
    assert_eq!(d.logs, vec![shown_log(0)]);
}

// ---------------------------------------------------------------------------
// 観測が取れないフレーム
// ---------------------------------------------------------------------------

/// グリフ数の観測が取れないフレームは、増加とも下降とも読まない。直前に観測できた値は
/// 保持され、次に観測できたフレームの比較相手になる（消す側へ倒れない）。
#[test]
fn unobserved_glyph_count_yields_no_edge_and_preserves_the_last_seen_value() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);

    let blind = step(&mut state, &[(0, unobserved(true))]);
    assert!(blind.actions.is_empty(), "観測不能フレームで発行が起きた");
    assert!(blind.logs.is_empty(), "観測不能フレームでログが出た");

    // 観測が戻ったフレームで、観測不能フレームを挟む前の値と比べてゼロ下降が成立する。
    let d = step(&mut state, &[(0, seen(0, true))]);
    assert_eq!(d.actions, vec![hide_cleared(&[0])]);
}

// ---------------------------------------------------------------------------
// ⑹ 決定論と並び
// ---------------------------------------------------------------------------

/// 観測の与え順が違っても、行動・ログの並びは scope 昇順で一定になる。
#[test]
fn output_order_is_scope_ascending_regardless_of_input_order() {
    let mut state = BalloonVisibilityState::default();
    let d = step(
        &mut state,
        &[
            (2, seen(1, false)),
            (0, seen(1, false)),
            (1, seen(1, false)),
        ],
    );
    assert_eq!(d.actions, vec![show(0), show(1), show(2)]);
    assert_eq!(d.logs, vec![shown_log(0), shown_log(1), shown_log(2)]);
}

/// 同一の観測列に対して常に同一の遷移列を返す（Requirement 9.1 の前提）。
#[test]
fn identical_observation_sequences_yield_identical_decisions() {
    let frames: [&[(u32, ScopeObservation)]; 5] = [
        &[
            (0, seen(0, false)),
            (1, seen(0, false)),
            (2, seen(0, false)),
        ],
        &[
            (0, seen(2, false)),
            (1, seen(0, false)),
            (2, seen(0, false)),
        ],
        &[(0, seen(2, true)), (1, seen(3, false)), (2, seen(0, false))],
        &[(0, seen(2, true)), (1, seen(3, true)), (2, seen(0, false))],
        &[(0, seen(0, true)), (1, seen(0, true)), (2, seen(0, false))],
    ];

    let run = || {
        let mut state = BalloonVisibilityState::default();
        frames
            .iter()
            .map(|frame| step(&mut state, frame))
            .collect::<Vec<_>>()
    };

    let first = run();
    assert_eq!(first, run(), "同一の観測列が異なる遷移列を返した");

    // 較正: 上の一致が「常に空」で成り立つ恒真でないことを、実際の遷移で押さえる。
    assert_eq!(first[1].actions, vec![show(0)]);
    assert_eq!(first[2].actions, vec![show(1)]);
    assert_eq!(first[4].actions, vec![hide_cleared(&[0, 1])]);
}

// ---------------------------------------------------------------------------
// 状態モデル
// ---------------------------------------------------------------------------

/// 前フレームの実可視は、本判断が同じフレームで発行した遷移を反映して持ち越す。
/// これを観測値のまま覚えると、自分が出した表示が次フレームで「外から表示された」と
/// 誤検出される（消費は task 4.4）。
#[test]
fn prev_visible_reflects_the_transitions_this_frame_issued() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(2, false)), (1, seen(0, false))]);
    assert!(
        state.per_scope[&0].prev_visible,
        "発行した表示が持ち越されていない"
    );
    assert!(
        !state.per_scope[&1].prev_visible,
        "無発話 scope が可視として持ち越された"
    );

    step(&mut state, &[(0, seen(0, true)), (1, seen(0, false))]);
    assert!(
        !state.per_scope[&0].prev_visible,
        "発行した非表示が持ち越されていない"
    );

    // 遷移を発行しないフレームでは観測値がそのまま次の比較相手になる。
    step(&mut state, &[(0, seen(0, true)), (1, seen(0, false))]);
    assert!(
        state.per_scope[&0].prev_visible,
        "観測値が次の比較相手として持ち越されていない"
    );
}

/// 会話単位の状態は、時刻を与えないフレームでは静止したまま（タイムアウトの評価が
/// 現在時刻の不明なフレームで動き出さない）。
#[test]
fn conversation_scoped_state_is_untouched_by_content_decisions() {
    let mut state = BalloonVisibilityState::default();
    step(&mut state, &[(0, seen(3, false))]);
    step(&mut state, &[(0, seen(0, true))]);

    assert_eq!(state.display_end, None);
    assert_eq!(state.deadline, None);
    assert!(!state.prev_suppressed);
    assert!(!state.suppress_logged);
    assert!(!state.signal_gap_warned);
}

// ---------------------------------------------------------------------------
// ⑺ 満了予定の起点と満了境界
// ---------------------------------------------------------------------------

/// 会話の占有終端（talk 相対秒）。
const DISPLAY_END: f64 = 10.0;
/// タイムアウト時間（秒）。既定値の唯一の定義箇所から引く（Requirement 4.2——同じ数値を
/// 複数箇所へ散らさない）。各フレームへは [`Frame::timeout`] で明示に渡す。
const TIMEOUT: f64 = DEFAULT_BALLOON_TIMEOUT_SECS;

/// scope 0 が可視・scope 1 は無発話のまま、満了予定 30.0 の計測が立っている状態を作る
/// （占有終端 0.0 ＋ タイムアウト 30 秒）。
fn armed_state() -> BalloonVisibilityState {
    let mut state = BalloonVisibilityState::default();
    let armed = Frame::new(&[(0, seen(2, false)), (1, seen(0, false))])
        .at(0.0)
        .talk_started()
        .display_end(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(armed.actions, vec![show(0)], "前提: 表示が起きていない");
    assert_eq!(
        armed.logs,
        vec![
            shown_log(0),
            VisibilityLogEvent::MeasurementStarted {
                display_end: 0.0,
                deadline: TIMEOUT,
            }
        ],
        "前提: 計測の開始が記録されていない"
    );
    assert_eq!(
        state.deadline,
        Some(TIMEOUT),
        "前提: 満了予定が立っていない"
    );
    state
}

/// 満了予定の起点は**占有終端**であって、計測が成り立った最初のフレームの現在時刻ではない
/// （Requirement 4.1 の正典起点「スクリプトの表示が終わってから」）。観測はフレーム単位で
/// 飛び飛びに入るため、現在時刻を起点に採ると観測の遅れの分だけ満了がずれる。
///
/// 中断による終了（Requirement 4.6）も起点は同じ占有終端であり、中断のみを理由とする即時
/// 非表示の経路は判断中核に存在しない——起点が 1 つしかないことを、この期待値が押さえる。
#[test]
fn deadline_is_anchored_on_display_end_not_on_the_first_eligible_frame() {
    let mut state = BalloonVisibilityState::default();

    // 会話が始まり、占有終端 10.0 秒が届く。scope 0 が喋って可視になる。
    let shown = Frame::new(&[(0, seen(3, false))])
        .at(1.0)
        .talk_started()
        .display_end(DISPLAY_END)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(shown.actions, vec![show(0)]);
    assert_eq!(state.deadline, None, "占有終端より前に計測が始まった");

    // 計測が成り立つ最初の観測が占有終端から 25 秒遅れて訪れる。現在時刻を起点にすると
    // 満了予定は 65.0 になり、以下の 40.0 の期待が成り立たなくなる。
    let armed = Frame::new(&[(0, seen(3, true))])
        .at(35.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(armed.actions.is_empty(), "計測開始と同時に非表示が起きた");
    assert_eq!(
        armed.logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            display_end: DISPLAY_END,
            deadline: DISPLAY_END + TIMEOUT,
        }]
    );
    assert_eq!(state.deadline, Some(DISPLAY_END + TIMEOUT));

    // 満了予定の直前は不成立。
    let before = Frame::new(&[(0, seen(3, true))])
        .at(DISPLAY_END + TIMEOUT - 0.1)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(before.actions.is_empty(), "満了予定の直前で非表示が起きた");
    assert!(before.logs.is_empty(), "遷移が無いのにログが出た");

    // 満了予定に達したフレームで成立する。
    let expired = Frame::new(&[(0, seen(3, true))])
        .at(DISPLAY_END + TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.logs, vec![timed_out_log(0)]);
    assert_eq!(state.deadline, None, "満了後も計測が残っている");

    // 満了後のフレームは無音（同じ非表示を撃ち直さない・Requirement 8.6）。
    let after = Frame::new(&[(0, seen(3, false))])
        .at(DISPLAY_END + TIMEOUT + 1.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(after.actions.is_empty(), "満了後に非表示が再発行された");
    assert!(after.logs.is_empty(), "満了後にログが出た");
}

/// タイムアウトの非表示は、その時点で可視の scope すべてを 1 つの行動へ畳む。無発話のまま
/// 不可視の scope は対象に含めない（Requirement 4.3）。
#[test]
fn timeout_hides_every_visible_scope_and_leaves_the_silent_ones() {
    let mut state = BalloonVisibilityState::default();
    Frame::new(&[
        (0, seen(2, false)),
        (1, seen(0, false)),
        (2, seen(0, false)),
    ])
    .at(0.0)
    .talk_started()
    .display_end(0.0)
    .timeout(TIMEOUT)
    .run(&mut state);
    Frame::new(&[(0, seen(2, true)), (1, seen(4, false)), (2, seen(0, false))])
        .at(1.0)
        .timeout(TIMEOUT)
        .run(&mut state);

    let expired = Frame::new(&[(0, seen(2, true)), (1, seen(4, true)), (2, seen(0, false))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0, 1])]);
    assert_eq!(expired.logs, vec![timed_out_log(0), timed_out_log(1)]);
}

/// 同じフレームで表示した scope は、積み残しの満了予定で消さない（出してすぐ消す行動対を
/// 作らない）。表示を保持する側へ倒す縮退である。
#[test]
fn a_scope_shown_in_the_same_frame_is_not_hidden_by_the_expired_deadline() {
    let mut state = armed_state();

    let mixed = Frame::new(&[(0, seen(2, true)), (1, seen(3, false))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(mixed.actions, vec![show(1), hide_timed_out(&[0])]);
    assert_eq!(mixed.logs, vec![shown_log(1), timed_out_log(0)]);
}

// ---------------------------------------------------------------------------
// ⑻ 抑止の成立・解除・取り直し
// ---------------------------------------------------------------------------

/// 抑止が成立している間は満了予定を過ぎても非表示を保留し（Requirement 5.6）、解除された
/// 時点から既定時間を改めて計り直す（Requirement 5.3——抑止前の残り時間を再開しない）。
/// 抑止のログは 1 回の抑止につき 1 件だけ出す（Requirement 8.3）。
#[test]
fn suppression_holds_the_expired_deadline_and_release_restarts_the_measurement() {
    let mut state = armed_state();

    // ポインタが滞在したまま満了予定に達する。
    let held = Frame::new(&[(0, hovered(seen(2, true))), (1, seen(0, false))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(held.actions.is_empty(), "抑止中に満了で非表示が起きた");
    assert_eq!(
        held.logs,
        vec![VisibilityLogEvent::TimeoutSuppressed {
            kinds: SuppressionKinds {
                hover: true,
                ..SuppressionKinds::default()
            },
            deadline: TIMEOUT,
        }]
    );
    assert_eq!(state.deadline, Some(TIMEOUT), "保留のはずが計測が消えた");

    // 抑止が続く間の超過は無音のまま保留し続ける。
    let still_held = Frame::new(&[(0, hovered(seen(2, true))), (1, seen(0, false))])
        .at(TIMEOUT + 1.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(still_held.actions.is_empty(), "抑止中に非表示が起きた");
    assert!(
        still_held.logs.is_empty(),
        "抑止のログがフレームごとに出ている"
    );

    // 解除。その瞬間に積み残しの満了で消さず、現在時刻を起点に取り直す。
    let released = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(TIMEOUT + 2.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(
        released.actions.is_empty(),
        "解除の瞬間に積み残しの満了で消えた"
    );
    assert_eq!(
        released.logs,
        vec![VisibilityLogEvent::MeasurementRestarted {
            now: TIMEOUT + 2.0,
            deadline: TIMEOUT + 2.0 + TIMEOUT,
        }]
    );

    // 取り直した満了予定の直前・直後。
    let before = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(TIMEOUT + 2.0 + TIMEOUT - 0.1)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(before.actions.is_empty(), "取り直した満了の直前で消えた");

    let expired = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(TIMEOUT + 2.0 + TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
}

/// ドラッグ中と、装着済み scope での選択肢表示中はいずれも抑止になる（Requirement 5.1 / 5.4）。
/// 一方、**不可視**の scope へのポインタ滞在は抑止にしない——不可視の間は離脱の通知が届かず、
/// 滞在の記録が残ったままだと恒久的な抑止に固着するため（Requirement 5.5 の向き）。
#[test]
fn drag_and_choice_suppress_but_hover_on_an_invisible_scope_does_not() {
    let mut dragged = armed_state();
    let held_by_drag = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .dragging()
        .run(&mut dragged);
    assert!(
        held_by_drag.actions.is_empty(),
        "ドラッグ中に非表示が起きた"
    );
    assert_eq!(
        held_by_drag.logs,
        vec![VisibilityLogEvent::TimeoutSuppressed {
            kinds: SuppressionKinds {
                dragging: true,
                ..SuppressionKinds::default()
            },
            deadline: TIMEOUT,
        }]
    );

    // 選択肢は装着済みの scope すべてを見る（不可視の scope の選択肢でも抑止する）。
    let mut choosing_state = armed_state();
    let held_by_choice = Frame::new(&[(0, seen(2, true)), (1, choosing(seen(0, false)))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut choosing_state);
    assert!(
        held_by_choice.actions.is_empty(),
        "選択肢の表示中に非表示が起きた"
    );
    assert_eq!(
        held_by_choice.logs,
        vec![VisibilityLogEvent::TimeoutSuppressed {
            kinds: SuppressionKinds {
                choice: true,
                ..SuppressionKinds::default()
            },
            deadline: TIMEOUT,
        }]
    );

    // 滞在は可視の scope に限って効く。
    let mut hovered_state = armed_state();
    let expired = Frame::new(&[(0, seen(2, true)), (1, hovered(seen(0, false)))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut hovered_state);
    assert_eq!(
        expired.actions,
        vec![hide_timed_out(&[0])],
        "不可視 scope の滞在で満了が抑止された"
    );
}

// ---------------------------------------------------------------------------
// ⑼ 抑止の観測不能
// ---------------------------------------------------------------------------

/// 抑止条件の観測が取れないフレームは、抑止されていないものとして満了させる
/// （Requirement 5.5——消えないまま固着する側へ倒さない）。
#[test]
fn unobservable_suppression_inputs_are_treated_as_not_suppressed() {
    let mut state = armed_state();

    let expired = Frame::new(&[
        (0, suppression_unobserved(seen(2, true))),
        (1, suppression_unobserved(seen(0, false))),
    ])
    .at(TIMEOUT)
    .timeout(TIMEOUT)
    .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
    assert_eq!(expired.logs, vec![timed_out_log(0)]);
}

// ---------------------------------------------------------------------------
// ⑽ 計測の破棄
// ---------------------------------------------------------------------------

/// 会話開始の通知で計測を破棄する（Requirement 4.5）。グリフ数の履歴は残すため、直後の
/// 全消去がゼロへの下降エッジとして成立する（design の Data Models）。
#[test]
fn talk_started_discards_the_measurement_and_keeps_the_glyph_history() {
    let mut state = armed_state();

    let restarted = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(2.0)
        .talk_started()
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(restarted.actions.is_empty(), "会話開始で発行が起きた");
    assert_eq!(
        restarted.logs,
        vec![VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::TalkStarted,
            deadline: TIMEOUT,
        }]
    );
    assert_eq!(state.display_end, None, "占有終端が破棄されていない");
    assert_eq!(state.deadline, None, "満了予定が破棄されていない");

    // 破棄前の満了予定を過ぎても消えない。
    let past = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(TIMEOUT + 1.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(past.actions.is_empty(), "破棄した満了予定で非表示が起きた");

    // グリフ数の履歴が残っているため、全消去はゼロへの下降として成立する。
    let cleared = Frame::new(&[(0, seen(0, true)), (1, seen(0, false))])
        .at(TIMEOUT + 2.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(cleared.actions, vec![hide_cleared(&[0])]);
}

/// 占有終端が現在時刻より未来へ更新されたら満了予定を破棄し、新しい占有終端を起点に立て直す
/// （選択肢のバリア解除後に cue が届く経路）。
#[test]
fn display_end_raised_into_the_future_discards_the_deadline() {
    let mut state = armed_state();

    let raised = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(5.0)
        .display_end(50.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(raised.actions.is_empty(), "占有終端の更新で発行が起きた");
    assert_eq!(
        raised.logs,
        vec![VisibilityLogEvent::MeasurementDiscarded {
            reason: MeasurementDiscardReason::DisplayEndAdvanced,
            deadline: TIMEOUT,
        }]
    );
    assert_eq!(state.deadline, None);

    // 破棄前の満了予定では消えない。
    let old_deadline = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(
        old_deadline.actions.is_empty(),
        "破棄した満了予定で非表示が起きた"
    );

    // 新しい占有終端を起点に立て直る。
    let rearmed = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(50.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        rearmed.logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            display_end: 50.0,
            deadline: 80.0,
        }]
    );

    let expired = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(80.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
}

/// 可視のバルーンが 1 つも無くなったら計測を破棄する（消す対象が無い）。
#[test]
fn clearing_every_balloon_discards_the_measurement() {
    let mut state = armed_state();

    let cleared = Frame::new(&[(0, seen(0, true)), (1, seen(0, false))])
        .at(2.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(cleared.actions, vec![hide_cleared(&[0])]);
    assert_eq!(
        cleared.logs,
        vec![
            hidden_log(0),
            VisibilityLogEvent::MeasurementDiscarded {
                reason: MeasurementDiscardReason::NoVisibleScope,
                deadline: TIMEOUT,
            }
        ]
    );
    assert_eq!(state.deadline, None);

    // 破棄は 1 回だけ（不可視のまま時が進んでもログは出ない・Requirement 8.6）。
    let quiet = Frame::new(&[(0, seen(0, false)), (1, seen(0, false))])
        .at(3.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(quiet.logs.is_empty(), "破棄のログが繰り返された");
}

// ---------------------------------------------------------------------------
// ⑾ 表示終了信号の欠落
// ---------------------------------------------------------------------------

/// 会話の表示終了信号が届かない間は計測を始めず、バルーンを表示したまま保つ。信号が無いまま
/// 可視コンテンツが現れた事実は記録に残すが、会話につき 1 回に留める（Requirement 4.8）。
#[test]
fn no_measurement_starts_while_the_end_of_talk_signal_is_absent() {
    let mut state = BalloonVisibilityState::default();

    let shown = Frame::new(&[(0, seen(3, false)), (1, seen(0, false))])
        .at(0.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(shown.actions, vec![show(0)]);
    assert_eq!(
        shown.logs,
        vec![shown_log(0), VisibilityLogEvent::DisplayEndSignalMissing]
    );

    // 既定時間を大きく超えても保持する（起点不明を理由に任意の時点で消さない）。
    let long_after = Frame::new(&[(0, seen(3, true)), (1, seen(0, false))])
        .at(TIMEOUT * 10.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(
        long_after.actions.is_empty(),
        "起点不明のまま非表示が起きた"
    );
    assert!(long_after.logs.is_empty(), "記録が毎フレーム繰り返された");
    assert_eq!(state.deadline, None);

    // 別の scope が喋り出しても記録は繰り返さない（会話につき 1 回）。
    let second = Frame::new(&[(0, seen(3, true)), (1, seen(2, false))])
        .at(TIMEOUT * 10.0 + 1.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(second.actions, vec![show(1)]);
    assert_eq!(second.logs, vec![shown_log(1)], "記録が 2 回目も出た");
}

// ---------------------------------------------------------------------------
// ⑿ 端の値（占有終端の丸めと、現在時刻が数でない場合）
// ---------------------------------------------------------------------------

/// 負の占有終端は下限 0.0 へ丸めてから起点にする。丸めが無ければ満了予定はその分だけ手前へ
/// ずれ、早く消える側へ倒れる。
///
/// 送出側は全 cue が負時刻の台本でしか負値を出さないが、丸めの有無で満了予定が動く以上、
/// 判断中核の側で押さえる。
#[test]
fn a_negative_display_end_is_rounded_up_to_zero_before_anchoring() {
    let mut state = BalloonVisibilityState::default();

    let armed = Frame::new(&[(0, seen(2, false))])
        .at(0.0)
        .talk_started()
        .display_end(-5.0)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(armed.actions, vec![show(0)]);
    assert_eq!(
        armed.logs,
        vec![
            shown_log(0),
            VisibilityLogEvent::MeasurementStarted {
                display_end: 0.0,
                deadline: TIMEOUT,
            }
        ],
        "負の占有終端がそのまま起点になった"
    );
    assert_eq!(state.display_end, Some(0.0));

    // 丸めが無ければ満了予定は TIMEOUT - 5.0 で、この直前フレームで既に消えている。
    let before = Frame::new(&[(0, seen(2, true))])
        .at(TIMEOUT - 0.1)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(before.actions.is_empty(), "満了予定の直前で非表示が起きた");

    let expired = Frame::new(&[(0, seen(2, true))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
}

/// 数でない占有終端も同じ丸めで 0.0 になる。丸めが無いと以後どの時刻とも比較が成り立たず、
/// 計測が二度と始まらない（消えないまま固着する側へ倒れる）。
///
/// 送出側は非有限値を記録付きで捨てるためここへは届かないが、丸めの担保は判断中核の責任である。
#[test]
fn a_non_number_display_end_is_rounded_up_to_zero_before_anchoring() {
    let mut state = BalloonVisibilityState::default();

    let armed = Frame::new(&[(0, seen(2, false))])
        .at(0.0)
        .talk_started()
        .display_end(f64::NAN)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(
        armed.logs,
        vec![
            shown_log(0),
            VisibilityLogEvent::MeasurementStarted {
                display_end: 0.0,
                deadline: TIMEOUT,
            }
        ],
        "数でない占有終端で計測が立たなくなった"
    );
    assert_eq!(state.display_end, Some(0.0));

    let expired = Frame::new(&[(0, seen(2, true))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
}

/// 現在時刻が数でないフレームでは満了させない（表示を保持する側へ倒す）。計測そのものは
/// 壊さず、時刻が数に戻ったフレームで通常どおり満了する。
#[test]
fn a_non_number_current_time_never_expires_the_measurement() {
    let mut state = armed_state();

    let blind = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(f64::NAN)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert!(blind.actions.is_empty(), "数でない現在時刻で非表示が起きた");
    assert!(blind.logs.is_empty(), "数でない現在時刻でログが出た");
    assert_eq!(state.deadline, Some(TIMEOUT), "満了予定が壊された");

    // 較正: 上の空が「もう消えるものが無い」で成り立つ恒真でないことを押さえる。
    let expired = Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
        .at(TIMEOUT)
        .timeout(TIMEOUT)
        .run(&mut state);
    assert_eq!(expired.actions, vec![hide_timed_out(&[0])]);
}

/// 同一の観測列（時刻・信号・抑止を含む）は同一の遷移列を返す（Requirement 9.1 の前提）。
#[test]
fn identical_timed_observation_sequences_yield_identical_decisions() {
    let run = || {
        let mut state = BalloonVisibilityState::default();
        let mut decisions = Vec::new();
        decisions.push(
            Frame::new(&[(0, seen(0, false)), (1, seen(0, false))])
                .at(0.0)
                .talk_started()
                .display_end(4.0)
                .timeout(TIMEOUT)
                .run(&mut state),
        );
        decisions.push(
            Frame::new(&[(0, seen(2, false)), (1, seen(0, false))])
                .at(1.0)
                .timeout(TIMEOUT)
                .run(&mut state),
        );
        decisions.push(
            Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
                .at(5.0)
                .timeout(TIMEOUT)
                .run(&mut state),
        );
        decisions.push(
            Frame::new(&[(0, hovered(seen(2, true))), (1, seen(0, false))])
                .at(34.0)
                .timeout(TIMEOUT)
                .run(&mut state),
        );
        decisions.push(
            Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
                .at(35.0)
                .timeout(TIMEOUT)
                .run(&mut state),
        );
        decisions.push(
            Frame::new(&[(0, seen(2, true)), (1, seen(0, false))])
                .at(65.0)
                .timeout(TIMEOUT)
                .run(&mut state),
        );
        decisions
    };

    let first = run();
    assert_eq!(first, run(), "同一の観測列が異なる遷移列を返した");

    // 較正: 上の一致が「常に空」で成り立つ恒真でないことを、実際の遷移で押さえる。
    assert_eq!(first[1].actions, vec![show(0)]);
    assert_eq!(
        first[2].logs,
        vec![VisibilityLogEvent::MeasurementStarted {
            display_end: 4.0,
            deadline: 34.0,
        }]
    );
    assert!(matches!(
        first[3].logs.as_slice(),
        [VisibilityLogEvent::TimeoutSuppressed { .. }]
    ));
    assert!(matches!(
        first[4].logs.as_slice(),
        [VisibilityLogEvent::MeasurementRestarted { .. }]
    ));
    assert_eq!(first[5].actions, vec![hide_timed_out(&[0])]);
}
