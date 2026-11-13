// =============================================================================
// 隠れたバルーンの再表示の決定論テスト（areka-P0-balloon-reappear-short-talk・task 2.1）
//
// design.md「ReappearPhaseTests」。本物の文字の層（`TextLayerRuntime`）へ本物の cue
// （`ClearAll`・`Clear`・`Text`）を流し、本物の観測の収集（`collect_observations`）で観測を作り、
// 本物の判定（`decide`）へ渡して、場面ごとに各フレームの行動と遷移の記録を確かめる。
//
// 修正の前から在る口（観測の収集・判定・切替の忘れ・文字の層への cue の投函・表示の合図）だけを
// 使い、観測の中身は `visible` 以外に触れない。だから修正の前の HEAD でもコンパイルが通り、
// 赤の 7 本は「期待した表示が無い」で失敗する。修正の後はこのファイルを変えずに通る。
//
// 可視の扱い: headless の表示層は表示が一度も確立していないので可視にできない。そこで観測の
// `visible` は、前のフレームの行動（`Show` で真・`HideScopes` で偽・`\b[-1]` にあたる外からの
// 非表示はテストが偽にする）からテストが決めて書き込む。文字の側は本番の経路のまま。
//
// 時刻は引数で注入し（`Some(t)`・時刻の無いフレームは `None`）、待ち時間は短い固定値を渡す。
// 実時間の待機は使わない（要件 3.4）。前の台詞は scope 0 に 5 文字を残す。
// =============================================================================

use std::sync::mpsc;

use areka_emo_text::state::TextLayerConfig;
use dola::cue::{CueCommand, TalkCue};

use super::test_support::attach_headless;
use super::*;

/// 待ち時間（秒）。短い固定値を引数で渡す（環境変数・既定値に依らない）。
const TIMEOUT: f64 = 2.0;

/// 前の台詞（5 文字・1 文字 0.25 秒・占有終端 1.25）。
const PREV_TEXT: &str = "あいうえお";
const PREV_END: f64 = 1.25;

/// 1 フレームの結果（行動の列と、遷移の記録の列）。
type FrameOutcome = (Vec<VisibilityAction>, Vec<VisibilityLogEvent>);

// ---------------------------------------------------------------------------
// 1 フレームずつ進める組（表示層・文字の層・world・判定の状態）
// ---------------------------------------------------------------------------

struct Rig {
    presenter: EmoPresenter,
    runtime: Rc<RefCell<TextLayerRuntime>>,
    world: World,
    state: BalloonVisibilityState,
    scopes: Vec<u32>,
    /// テストが決める「現に可視か」（前のフレームの行動から）。
    visible: BTreeMap<u32, bool>,
    /// フレームごとの結果（各テストが最後に 1 回で比べる）。
    outcomes: Vec<FrameOutcome>,
}

impl Rig {
    fn new(scopes: &[u32]) -> Rig {
        let mut presenter = EmoPresenter::new();
        let mut world = World::new();
        world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
        for &scope in scopes {
            attach_headless(&mut presenter, &mut world, scope);
        }
        Rig {
            presenter,
            runtime: Rc::new(RefCell::new(TextLayerRuntime::new(
                TextLayerConfig::default(),
            ))),
            world,
            state: BalloonVisibilityState::default(),
            scopes: scopes.to_vec(),
            visible: scopes.iter().map(|&scope| (scope, false)).collect(),
            outcomes: Vec::new(),
        }
    }

    /// cue を文字の層へ投函し、合図を添えて 1 フレーム回す。
    fn frame(&mut self, now: Option<f64>, signals: &[TalkLifecycleSignal], cues: &[TalkCue]) {
        self.step(now, signals, cues, false);
    }

    /// 文字の層を借りたまま 1 フレーム回す（観測なしのフレーム・要件 1.4 のただし書き）。
    fn frame_unobservable(
        &mut self,
        now: Option<f64>,
        signals: &[TalkLifecycleSignal],
        cues: &[TalkCue],
    ) {
        self.step(now, signals, cues, true);
    }

    fn step(
        &mut self,
        now: Option<f64>,
        signals: &[TalkLifecycleSignal],
        cues: &[TalkCue],
        hold_runtime: bool,
    ) {
        for cue in cues {
            self.runtime.borrow_mut().apply_cue(cue);
        }
        let held = hold_runtime.then(|| self.runtime.borrow_mut());
        let mut observations = collect_observations(
            &self.presenter,
            &self.runtime,
            &mut self.world,
            &self.scopes,
            now,
            signals.to_vec(),
            &mut self.state,
        );
        drop(held);
        for (scope, observed) in &mut observations.scopes {
            observed.visible = self.visible[scope];
        }

        let decision = decide(&mut self.state, &observations, now, TIMEOUT);

        for action in &decision.actions {
            match action {
                VisibilityAction::Show { scope } => {
                    self.visible.insert(*scope, true);
                }
                VisibilityAction::HideScopes { scopes, .. } => {
                    for scope in scopes {
                        self.visible.insert(*scope, false);
                    }
                }
            }
        }
        let transitions = decision
            .logs
            .into_iter()
            .filter(|event| matches!(event, VisibilityLogEvent::Transition { .. }))
            .collect();
        self.outcomes.push((decision.actions, transitions));
    }

    /// `\b[-1]` にあたる外からの非表示（判定は次のフレームの観測で知る）。
    fn hide_externally(&mut self, scope: u32) {
        self.visible.insert(scope, false);
    }

    /// バルーンの切替: 可視の記憶を忘れ、新しいバルーンは隠れたまま。
    fn switch_balloon(&mut self, scope: u32) {
        self.state.forget_scope(scope);
        self.visible.insert(scope, false);
    }
}

// ---------------------------------------------------------------------------
// cue・合図・期待の組み立て
// ---------------------------------------------------------------------------

fn cue(scope: u32, at: f64, command: CueCommand, duration: f64) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope.to_string()),
        command,
        duration,
    }
}

fn text(scope: u32, at: f64, body: &str, duration: f64) -> TalkCue {
    cue(scope, at, CueCommand::Text(body.into()), duration)
}

fn clear_all() -> TalkCue {
    cue(0, 0.0, CueCommand::ClearAll, 0.0)
}

/// 新しい台詞の合図（本番の表示の合図の受け口と同じく、会話開始の後に占有終端を添える）。
fn talk_started(end: f64) -> [TalkLifecycleSignal; 2] {
    [
        TalkLifecycleSignal::TalkStarted { talk_id: None },
        TalkLifecycleSignal::DisplayEndAt(end),
    ]
}

/// トークの終わりの合図（止まった時刻＝占有終端。時間切れの計測はこれが届いてから始まる）。
fn ended(at: f64) -> TalkLifecycleSignal {
    TalkLifecycleSignal::TalkEnded { at: Some(at) }
}

/// 台詞の始まり（全消去の後に scope 0 の文字）。
fn new_talk(body: &str, duration: f64) -> [TalkCue; 2] {
    [clear_all(), text(0, 0.0, body, duration)]
}

/// 何も出ないフレーム。
fn quiet() -> FrameOutcome {
    (Vec::new(), Vec::new())
}

/// 表示が 1 件・その記録が 1 件のフレーム。
fn shown(scope: u32) -> FrameOutcome {
    (
        vec![VisibilityAction::Show { scope }],
        vec![VisibilityLogEvent::Transition {
            scope,
            trigger: VisibilityTrigger::Content,
            visible: true,
        }],
    )
}

/// 非表示が 1 件・scope ごとの記録のフレーム。
fn hidden(scopes: &[u32], trigger: VisibilityTrigger) -> FrameOutcome {
    (
        vec![VisibilityAction::HideScopes {
            scopes: scopes.to_vec(),
            trigger,
        }],
        scopes
            .iter()
            .map(|&scope| VisibilityLogEvent::Transition {
                scope,
                trigger,
                visible: false,
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// 前の台詞の筋書き（隠れるまで）
// ---------------------------------------------------------------------------

/// 前の台詞を表示 → 占有終端の後に満了して隠れる → 何も来ないフレームを重ねる。
fn shown_then_timed_out(rig: &mut Rig) -> Vec<FrameOutcome> {
    rig.frame(
        Some(0.0),
        &talk_started(PREV_END),
        &[clear_all(), text(0, 0.0, PREV_TEXT, PREV_END)],
    );
    // 5 文字が見え、トークの終わりが届いて計測が始まる
    rig.frame(Some(PREV_END), &[ended(PREV_END)], &[]);
    rig.frame(Some(PREV_END + TIMEOUT), &[], &[]); // 満了
    rig.frame(Some(4.0), &[], &[]);
    rig.frame(Some(5.0), &[], &[]);
    vec![
        shown(0),
        quiet(),
        hidden(&[0], VisibilityTrigger::Timeout),
        quiet(),
        quiet(),
    ]
}

/// 前の台詞の 2 文字目で利用者が中断 → 止めた台詞の残りが時刻の進行で見えるようになる。
fn shown_then_broken(rig: &mut Rig) -> Vec<FrameOutcome> {
    rig.frame(
        Some(0.0),
        &talk_started(PREV_END),
        &[clear_all(), text(0, 0.0, PREV_TEXT, PREV_END)],
    );
    rig.frame(Some(0.25), &[TalkLifecycleSignal::UserBreak], &[]);
    rig.frame(Some(0.75), &[], &[]); // 4 文字
    rig.frame(Some(PREV_END), &[], &[]); // 5 文字（掛け金で出ない）
    vec![
        shown(0),
        hidden(&[0], VisibilityTrigger::UserBreak),
        quiet(),
        quiet(),
    ]
}

// ---------------------------------------------------------------------------
// 赤（修正の前に失敗する 7 本）
// ---------------------------------------------------------------------------

/// 時間切れの後の 1 文字は、全消去と同じフレームで現れる（要件 1.1・1.3・1.6・2.3・3.1）。
#[test]
fn one_glyph_talk_reappears_after_timeout() {
    let mut rig = Rig::new(&[0]);
    let mut expected = shown_then_timed_out(&mut rig);

    rig.frame(Some(0.0), &talk_started(0.25), &new_talk("ん", 0.25));
    rig.frame(Some(0.25), &[], &[]);
    expected.extend([shown(0), quiet()]); // 新しい台詞の最初の文字のフレームで表示

    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・満了・隠れたまま・新しい台詞の表示・その後"
    );
}

/// 利用者の中断の後の 1 文字は、次の台詞の最初のフレームで現れる（要件 1.3・2.2・3.1）。
#[test]
fn one_glyph_talk_reappears_after_user_break() {
    let mut rig = Rig::new(&[0]);
    let mut expected = shown_then_broken(&mut rig);

    rig.frame(Some(0.0), &talk_started(0.25), &new_talk("ん", 0.25));
    rig.frame(Some(0.25), &[], &[]);
    expected.extend([shown(0), quiet()]); // 新しい台詞の最初の文字のフレームで表示

    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・中断・残りが見えても出ない・新しい台詞の表示・その後"
    );
}

/// バルーンの切替の後の 1 文字は、次の台詞の最初のフレームで現れる（要件 1.3・2.1・3.1）。
#[test]
fn one_glyph_talk_reappears_after_balloon_switch() {
    let mut rig = Rig::new(&[0]);
    rig.frame(
        Some(0.0),
        &talk_started(PREV_END),
        &[clear_all(), text(0, 0.0, PREV_TEXT, PREV_END)],
    );
    rig.frame(Some(PREV_END), &[], &[]);
    rig.switch_balloon(0);
    rig.frame(Some(1.5), &[], &[]); // 前の文字は見えたまま
    rig.frame(Some(2.0), &[], &[]);
    rig.frame(Some(0.0), &talk_started(0.25), &new_talk("ん", 0.25));
    rig.frame(Some(0.25), &[], &[]);

    let expected = vec![
        shown(0),
        quiet(),
        quiet(),
        quiet(),
        shown(0), // 新しい台詞の最初の文字のフレームで表示
        quiet(),
    ];
    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・切替の後は隠れたまま・新しい台詞の表示・その後"
    );
}

/// 最初のフレームで全文が一度に見える短い台詞（2 文字 ≦ 前の残り 5 文字）も現れる（要件 1.2・3.2）。
#[test]
fn instant_short_talk_reappears_after_timeout() {
    let mut rig = Rig::new(&[0]);
    let mut expected = shown_then_timed_out(&mut rig);

    rig.frame(Some(0.0), &talk_started(0.0), &new_talk("はい", 0.0));
    rig.frame(Some(0.25), &[], &[]);
    expected.extend([shown(0), quiet()]); // 新しい台詞の最初の文字のフレームで表示

    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・満了・隠れたまま・全文が一度に見える台詞の表示・その後"
    );
}

/// 台詞の途中の `\c` とその後の文字が同じフレームに届くと、隠れていた scope だけが現れる
/// （要件 1.5・3.2）。scope 1 は見えたまま何も出ない。
#[test]
fn mid_talk_clear_and_glyph_reappear_hidden_scope() {
    let mut rig = Rig::new(&[0, 1]);
    rig.frame(
        Some(0.0),
        &talk_started(PREV_END),
        &[clear_all(), text(0, 0.0, PREV_TEXT, PREV_END)],
    );
    rig.frame(
        Some(1.25),
        &[TalkLifecycleSignal::DisplayEndAt(2.0)],
        &[text(1, 1.25, "かきく", 0.75)],
    );
    rig.hide_externally(0); // `\b[-1]` にあたる
    rig.frame(Some(1.75), &[], &[]);
    rig.frame(
        Some(2.0),
        &[TalkLifecycleSignal::DisplayEndAt(2.25)],
        &[
            cue(0, 2.0, CueCommand::Clear, 0.0),
            text(0, 2.0, "ん", 0.25),
        ],
    );
    rig.frame(Some(2.25), &[], &[]);

    let expected = vec![
        shown(0),
        shown(1),
        quiet(),
        shown(0), // 新しい台詞の最初の文字のフレームで表示
        quiet(),
    ];
    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): scope 0 の表示・scope 1 の表示・scope 0 が外から隠れる・\\c と文字で scope 0 だけ表示・その後"
    );
}

/// 文字の層を観測できないフレームと時刻の無いフレームを挟むと、次に観測できたフレームで
/// 現れる（要件 1.4 のただし書き・3.3）。観測なしのフレームは何も出さない。
#[test]
fn reappears_on_next_observed_frame_after_unobservable_frames() {
    let mut rig = Rig::new(&[0]);
    let mut expected = shown_then_timed_out(&mut rig);

    rig.frame_unobservable(Some(0.0), &talk_started(0.25), &new_talk("ん", 0.25));
    rig.frame(None, &[], &[]);
    rig.frame(Some(0.1), &[], &[]);
    rig.frame(Some(0.25), &[], &[]);
    expected.extend([quiet(), quiet(), shown(0), quiet()]); // 新しい台詞の最初の文字のフレームで表示

    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・満了・隠れたまま・借用中・時刻なし・次に観測できたフレームで表示・その後"
    );
}

/// 会話開始の合図が先のフレームに取り出され、全消去と文字が次のフレームに入る並びでも、
/// 最初の文字が見えたフレームで現れる（要件 1.3・1.4・3.3）。
///
/// 合図のフレームでは会話の時刻が 0 へ戻り、前の台詞の文字は最初の 1 文字だけが見える数へ
/// 減る。次のフレームの新しい 1 文字と同じ数なので、前の数との比較では増加にならない。
#[test]
fn reappears_when_talk_started_signal_precedes_clear_frame() {
    let mut rig = Rig::new(&[0]);
    let mut expected = shown_then_timed_out(&mut rig);

    rig.frame(Some(0.0), &talk_started(0.0), &[]);
    rig.frame(
        Some(0.1),
        &[TalkLifecycleSignal::DisplayEndAt(0.25)],
        &new_talk("ん", 0.25),
    );
    rig.frame(Some(0.25), &[], &[]);
    expected.extend([quiet(), shown(0), quiet()]); // 新しい台詞の最初の文字のフレームで表示

    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・満了・隠れたまま・合図だけのフレーム・全消去と文字のフレームで表示・その後"
    );
}

// ---------------------------------------------------------------------------
// 守り（修正の前から通る 3 本）
// ---------------------------------------------------------------------------

/// 中断の後、全消去だけが先に文字の層へ入る（合図はまだ・掛け金は掛かったまま）並びでは、
/// 全消去のフレームでは出ず、最初の文字が見えたフレームで現れる（要件 1.3・1.4・2.2・3.3）。
#[test]
fn clear_before_signal_after_user_break_reappears_on_first_glyph() {
    let mut rig = Rig::new(&[0]);
    let mut expected = shown_then_broken(&mut rig);

    rig.frame(Some(0.0), &[], &[clear_all()]);
    rig.frame(Some(0.0), &talk_started(0.25), &[text(0, 0.0, "ん", 0.25)]);
    rig.frame(Some(0.25), &[], &[]);
    expected.extend([quiet(), shown(0), quiet()]);

    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・中断・残りが見えても出ない・全消去だけ・合図と最初の文字で表示・その後"
    );
}

/// 見えているバルーンへ全消去と最初の文字が同じフレームに届いても、隠して出し直さない
/// （要件 2.4・3.3）。
#[test]
fn visible_balloon_keeps_showing_through_clear_and_glyph() {
    let mut rig = Rig::new(&[0]);
    rig.frame(
        Some(0.0),
        &talk_started(PREV_END),
        &[clear_all(), text(0, 0.0, PREV_TEXT, PREV_END)],
    );
    rig.frame(Some(PREV_END), &[], &[]);
    rig.frame(Some(0.0), &talk_started(0.25), &new_talk("ん", 0.25));
    rig.frame(Some(0.25), &[], &[]);

    let expected = vec![shown(0), quiet(), quiet(), quiet()];
    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): 前の台詞の表示・見えたまま・全消去と文字でも行動も記録も無い・その後"
    );
}

/// 全消去の後に scope 0 へ文字が来ない台詞では、見えていた scope 0 を全消去で 1 回だけ隠し、
/// 台詞の間ずっと隠れたまま保つ。scope 1 は見えたまま（要件 2.5・3.3）。
#[test]
fn scope_without_glyph_after_clear_stays_hidden() {
    let mut rig = Rig::new(&[0, 1]);
    rig.frame(
        Some(0.0),
        &talk_started(PREV_END),
        &[clear_all(), text(0, 0.0, PREV_TEXT, PREV_END)],
    );
    rig.frame(
        Some(1.25),
        &[TalkLifecycleSignal::DisplayEndAt(2.0)],
        &[text(1, 1.25, "かきく", 0.75)],
    );
    rig.frame(Some(2.0), &[], &[]);
    rig.frame(
        Some(0.0),
        &talk_started(1.0),
        &[clear_all(), text(1, 0.0, "さしすせ", 1.0)],
    );
    rig.frame(Some(0.25), &[], &[]);
    rig.frame(Some(1.0), &[], &[]);

    let expected = vec![
        shown(0),
        shown(1),
        quiet(),
        hidden(&[0], VisibilityTrigger::Clear),
        quiet(),
        quiet(),
    ];
    assert_eq!(
        rig.outcomes, expected,
        "フレームごとの (行動, 遷移の記録): scope 0 の表示・scope 1 の表示・見えたまま・全消去で scope 0 だけ隠す・台詞の間ずっと出ない"
    );
}
