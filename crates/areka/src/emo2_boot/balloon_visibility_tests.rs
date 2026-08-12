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
// タイムアウト計測・抑止（task 3.2）はここでは扱わない。
// 実時間の待機・反復回数のみによる有界化は一切用いない（Requirement 9.2 / 9.3）。
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

/// 会話単位の状態は本タスクでは静止したまま（駆動は task 3.2）。
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
