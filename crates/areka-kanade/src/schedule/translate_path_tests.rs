//! 翻訳の経路のテスト（areka-P0-translate-pipeline タスク 3.5・要件 1.2〜1.5・3.1・4.6・5.5・6.1・6.2・8.1）。
//!
//! SHIORI の台詞で再生を始める 8 か所の腕（design「Existing Architecture Analysis」の表）を 1 本ずつ、
//! 前提の状態まで実際の入力で `step` を進めてから、元のイベントの台詞の応答を最上位の [`step`] に入れる。
//! どの経路も ⑴ 一括が翻訳の行動 1 つだけになり ⑵ 翻訳の結果で預けた一括が順序どおり最終の台詞で返り
//! ⑶ 翻訳の結果が元の台詞と同じなら、状態と一括が翻訳なしと同じ（[`assert_path`]）。出口の規則
//! （`translate::after` の中の捕まえる呼び出し）を外すと、⑴ でどの経路も赤になる。
//!
//! 「翻訳なし」の比べる相手は [`step_untranslated`]（[`step`] から翻訳の入口と出口だけを除いた写し）で
//! 同じ前提から作る。翻訳の帳簿と元のイベントの控え（`translate`・`reply_source`）は翻訳そのものの
//! 欄なので比べない。
//!
//! あわせて、翻訳しない台詞（areka が作る起動の記録だけの台詞・0 文字の台詞・外から頼まれた
//! `OnTranslate` の応答）、印のイベントの台詞の見張り、控えの書き換え（`OnChoiceTimeout` の
//! Reference0・停止通知の切替の中身の源）、選択の連鎖の後の翻訳の輸送路の失敗（論点 10）を固定する。

use super::events::SourceEvent;
use super::log_capture::{assert_not_logged, capture, logged_once};
use super::steady::test_support::base_state;
use super::talk_gap::Marked;
use super::*;
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget, GapRaise, ShioriMethod};
use crate::msg::{ChoiceInput, MouseButton, MouseEventKind, ShioriFailure, ShioriFaultKind};
use crate::status::ExecutionStatus;
use crate::talk::EpilogueCommand;
use tracing::Level;

/// SHIORI が返す元の台詞。
const ORIGINAL: &str = "\\0元の台詞\\e";
/// 翻訳の結果（最終の台詞）。
const FINAL: &str = "\\0最終の台詞\\e";
/// 前提づくりで始めたトークの最終の台詞（選択待ちの台詞・置き換えられる台詞）。
const SHOWN: &str = "\\0表示した選択肢の台詞\\e";
/// 選択肢の任意名（`On` 始まり＝任意名 1 段）。
const CHOICE_ID: &str = "OnおやつSelect";

fn cfg() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// 初回起動の記録（areka が台詞の後ろに足す部分）。
fn epilogue() -> Vec<EpilogueCommand> {
    vec![EpilogueCommand {
        name: "boot".to_string(),
        tokens: vec!["記録".to_string()],
    }]
}

/// `Steady{talk: None}`（終了の保留なし・控えなし）。ここから実際の入力で前提へ進める。
fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..base_state()
    }
}

// ============================================================
// 駆動と比べる道具
// ============================================================

/// 前提の状態へ進めるための駆動。入力はすべて最上位の [`step`] を通し、最後に送った GET を控える。
struct Drive {
    s: State,
    cfg: KanadeConfig,
    /// 最後に送った GET（ID と Reference）。応答を待っている元のイベント。
    sent: Option<SourceEvent>,
}

impl Drive {
    fn new(s: State, cfg: KanadeConfig) -> Drive {
        Drive { s, cfg, sent: None }
    }

    fn feed(&mut self, input: Input) -> Vec<Action> {
        let s = std::mem::replace(&mut self.s, State::initial());
        let (s, actions) = step(s, input, &self.cfg);
        self.s = s;
        if let Some(sent) = actions.iter().rev().find_map(|a| match a {
            Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => Some(SourceEvent {
                id: id.clone(),
                references: references.clone(),
            }),
            _ => None,
        }) {
            self.sent = Some(sent);
        }
        actions
    }

    fn reply(&mut self, outcome: ShioriOutcome, origin: &'static str) -> Vec<Action> {
        self.feed(Input::ShioriReply { outcome, origin })
    }

    /// 毎秒のポンプの応答で台詞を始め、翻訳の結果 `shown` で再生させる（前提づくり）。
    ///
    /// 前提づくりは出口の規則に頼らない（翻訳の行動が出なければ、その一括の再生の開始をそのまま
    /// 使う）。規則を外したとき、各経路は前提ではなく自分の腕の確かめで赤になる。
    fn start_talk(&mut self, now: u64, shown: &str) -> TalkId {
        self.feed(Input::Tick {
            now: MonotonicMs(now),
        });
        let mut actions = self.reply(ShioriOutcome::Value(ORIGINAL.into()), "OnSecondChange");
        if let [Action::Translate(_)] = actions.as_slice() {
            actions = self.feed(Input::TranslateDone(Ok(shown.to_string())));
        }
        started(&actions).talk_id
    }
}

/// 翻訳なしの `step`（[`step`] から翻訳の入口と出口だけを除いた写し）。比べる相手を作るためだけに使う。
fn step_untranslated(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    let marked_reply = talk_gap::marked_reply(&state, &input);
    let (mut state, actions) = route(state, input, config);
    talk_gap::observe(&mut state, marked_reply);
    balloon_events::settle(&mut state, &actions);
    (state, actions)
}

/// 行動の種類（順序の確かめに使う）。
fn kind(action: &Action) -> String {
    match action {
        Action::StartTalk(_) => "StartTalk".to_string(),
        Action::ShioriRequest(ShioriCall::Get { id, .. }) => format!("Get:{}", id.as_str()),
        Action::ShioriRequest(ShioriCall::Notify { id, .. }) => format!("Notify:{}", id.as_str()),
        Action::ResolveChoice { .. } => "ResolveChoice".to_string(),
        Action::CancelChoice { .. } => "CancelChoice".to_string(),
        Action::ShioriUnload => "Unload".to_string(),
        Action::StopSelf => "StopSelf".to_string(),
        Action::Notice(_) => "Notice".to_string(),
        Action::ResourceOutcome { .. } => "ResourceOutcome".to_string(),
        Action::Translate(_) => "Translate".to_string(),
    }
}

fn kinds(actions: &[Action]) -> Vec<String> {
    actions.iter().map(kind).collect()
}

/// 行動の中身まで含めた綴り（翻訳なしとの比較に使う）。
fn shape(action: &Action) -> String {
    match action {
        Action::StartTalk(t) => format!(
            "StartTalk({}, {:?}, {:?})",
            t.talk_id.0, t.script, t.epilogue
        ),
        Action::ShioriRequest(
            ShioriCall::Get {
                references, status, ..
            }
            | ShioriCall::Notify {
                references, status, ..
            },
        ) => format!("{}({references:?}, {status:?})", kind(action)),
        Action::ResolveChoice { talk_id, id } => format!("ResolveChoice({}, {id})", talk_id.0),
        Action::CancelChoice { talk_id } => format!("CancelChoice({})", talk_id.0),
        Action::Notice(n) => format!("Notice({n:?})"),
        Action::Translate(r) => format!("Translate({:?}, {:?})", r.script, r.source),
        _ => kind(action),
    }
}

fn shapes(actions: &[Action]) -> Vec<String> {
    actions.iter().map(shape).collect()
}

fn active_talk(phase: &Phase) -> Option<&ActiveTalk> {
    match phase {
        Phase::Steady { talk: Some(t) } | Phase::BootVersion { talk: Some(t) } => Some(t),
        _ => None,
    }
}

/// 相の綴り（番号・期限・再生中の台詞まで含める）。
fn phase_shape(phase: &Phase) -> String {
    let label = phase_label(phase);
    match phase {
        Phase::Steady { talk: Some(t) } | Phase::BootVersion { talk: Some(t) } => {
            format!("{label}({}, {}, {:?})", t.talk_id.0, t.origin, t.script)
        }
        Phase::CloseTalkWait { talk_id, deadline }
        | Phase::ChangeTalkWait { talk_id, deadline }
        | Phase::ChangeCloseTalkWait { talk_id, deadline } => {
            format!("{label}({}, {deadline:?})", talk_id.0)
        }
        Phase::ClosePending { reason } => format!("{label}({reason:?})"),
        _ => label.to_string(),
    }
}

fn marked_shape(marked: &Marked) -> String {
    match marked {
        Marked::None => "None".to_string(),
        Marked::AwaitingReply(id) => format!("AwaitingReply({id})"),
        Marked::Talk(t) => format!("Talk({})", t.0),
        Marked::Ended(end) => format!("Ended({end:?})"),
        Marked::BrokenByUser => "BrokenByUser".to_string(),
    }
}

/// 状態の綴り（翻訳の帳簿と元のイベントの控えは除く・外から届いた写しは翻訳と無関係なので除く）。
fn state_shape(s: &State) -> String {
    let choice = s.choice.as_ref().map(|c| {
        let stage = steady::choice_phase_label(&c.phase);
        format!(
            "({}, {:?}, {:?}, {stage})",
            c.talk_id.0, c.candidates, c.deadline
        )
    });
    let change = s
        .change
        .as_ref()
        .map(|c| format!("({:?}, {:?})", c.req, c.script));
    let gap = s
        .talk_gap
        .as_ref()
        .map(|g| format!("({}, {:?})", marked_shape(&g.marked), g.outcome));
    format!(
        "phase={} now={:?} next={} close={:?} choice={choice:?} prev={:?} break={:?} change={change:?} pending_change={:?} gap={gap:?}",
        phase_shape(&s.phase),
        s.last_now,
        s.next_talk_id,
        s.pending_close,
        s.choice_prev_talk.map(|t| t.0),
        s.user_break_talk.map(|note| note.talk_id.0),
        s.pending_change,
    )
}

fn started(actions: &[Action]) -> &StartTalk {
    actions
        .iter()
        .find_map(|a| match a {
            Action::StartTalk(t) => Some(t),
            _ => None,
        })
        .expect("再生の開始があるはず")
}

/// 経路の前提（元のイベントの GET を送り、その応答を待っている）と、応答に付く出所。
type Prepare = fn() -> (Drive, &'static str);

/// 経路 1 本の通過を確かめる（design Testing Strategy の `translate_path_tests.rs` の 1・3 行目）。
///
/// ⑴ 元のイベントの台詞の応答で、一括が翻訳の行動 1 つだけになり、依頼に元の台詞・元のイベント
/// （応答を待っていた GET）・捕まえた時点の Status が載る ⑵ 翻訳の結果で、預けた一括が `expected`
/// の順で返り、再生の開始の台詞と再生中の台詞の控えが最終の台詞になる ⑶ 翻訳の結果が元の台詞と
/// 同じとき、状態と一括が翻訳なし（[`step_untranslated`]）と同じ。
///
/// 返すのは ⑵ の後の駆動と返った一括（経路ごとの控えの確かめに使う）。
fn assert_path(prepare: Prepare, expected: &[&str]) -> (Drive, Vec<Action>) {
    // ⑴ 捕まえる。
    let (mut d, origin) = prepare();
    let sent = d.sent.clone().expect("元のイベントの GET を送ってある");
    let actions = d.reply(ShioriOutcome::Value(ORIGINAL.into()), origin);
    let [Action::Translate(req)] = actions.as_slice() else {
        panic!("一括は翻訳の行動 1 つだけのはず: {:?}", shapes(&actions));
    };
    assert_eq!(req.script, ORIGINAL, "SHIORI が返したまま（展開の前）");
    assert_eq!(req.source, sent, "元のイベント＝応答を待っていた GET");
    assert_eq!(
        req.status,
        ExecutionStatus::derive(&d.s.snapshot()),
        "Status は捕まえた時点（腕が相を決めた後）の状態から"
    );
    assert!(d.s.translate.is_some(), "帳簿を置く");

    // ⑵ 預けた一括を最終の台詞で返す。
    let resumed = d.feed(Input::TranslateDone(Ok(FINAL.into())));
    assert_eq!(kinds(&resumed), expected, "預けた一括を順序どおり返す");
    let start = started(&resumed);
    assert_eq!(start.script, FINAL, "再生の開始の台詞は最終の台詞");
    assert!(d.s.translate.is_none(), "帳簿は空になる");
    if let Some(active) = active_talk(&d.s.phase)
        && active.talk_id == start.talk_id
    {
        assert_eq!(
            active.script, FINAL,
            "再生中の台詞の控えは最終の台詞（要件 6.1）"
        );
    }

    // ⑶ 元の台詞のまま返る翻訳は、翻訳なしと同じ状態と一括になる（要件 5.5）。
    let (mut t, origin) = prepare();
    t.reply(ShioriOutcome::Value(ORIGINAL.into()), origin);
    let same = t.feed(Input::TranslateDone(Ok(ORIGINAL.into())));
    let (b, origin) = prepare();
    let reply = Input::ShioriReply {
        outcome: ShioriOutcome::Value(ORIGINAL.into()),
        origin,
    };
    let (base, base_actions) = step_untranslated(b.s, reply, &b.cfg);
    assert_eq!(shapes(&same), shapes(&base_actions), "一括が翻訳なしと同じ");
    assert_eq!(
        state_shape(&t.s),
        state_shape(&base),
        "状態が翻訳なしと同じ"
    );
    (d, resumed)
}

// ============================================================
// 経路の前提（実際の入力で元のイベントの GET を送るところまで進める）
// ============================================================

/// 経路 1: 初回起動の `OnFirstBoot` を送った（起動の記録を足す構成）。
fn boot_first() -> (Drive, &'static str) {
    let mut cfg = cfg();
    cfg.first_boot_epilogue = epilogue();
    let mut d = Drive::new(State::initial(), cfg);
    d.feed(Input::Boot);
    d.reply(ShioriOutcome::Notified, "OnInitialize");
    d.reply(ShioriOutcome::NoContent, "username");
    (d, "OnFirstBoot")
}

/// 経路 2: 終了の要求で `OnClose` を送った。
fn close_farewell() -> (Drive, &'static str) {
    let mut d = Drive::new(steady_none(), cfg());
    d.feed(Input::CloseRequest {
        reason: CloseReason::User { scope: 0 },
    });
    (d, "OnClose")
}

fn change_req() -> ChangeRequest {
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: "ポスト".to_string(),
            name: "R_POST_and_KOMAINU".to_string(),
            dir: r"C:\areka\ghost\r_post".to_string(),
        },
        origin: ChangeOrigin::Manual,
        raise_event: true,
    }
}

/// 経路 3: 切替の要求で `OnGhostChanging` を送った。
fn ghost_changing() -> (Drive, &'static str) {
    let mut d = Drive::new(steady_none(), cfg());
    d.feed(Input::ChangeGhost(change_req()));
    (d, "OnGhostChanging")
}

/// 経路 3: `OnGhostChanging` が 204 で、続けて切替の `OnClose` を送った。
fn ghost_change_close() -> (Drive, &'static str) {
    let (mut d, origin) = ghost_changing();
    d.reply(ShioriOutcome::NoContent, origin);
    (d, "OnClose")
}

/// 経路 3: `OnGhostChanging` の応答待ちに終了の要求が来て、切替を取りやめた。
fn ghost_change_yielded() -> (Drive, &'static str) {
    let (mut d, origin) = ghost_changing();
    let actions = d.feed(Input::CloseRequest {
        reason: CloseReason::User { scope: 0 },
    });
    assert_eq!(
        kinds(&actions),
        ["Notice"],
        "取りやめの通知だけ（往復なし）"
    );
    (d, origin)
}

/// 経路 4: 毎秒のポンプで `OnSecondChange` を GET した（再生中のトークなし）。
fn steady_pump() -> (Drive, &'static str) {
    let mut d = Drive::new(steady_none(), cfg());
    d.feed(Input::Tick {
        now: MonotonicMs(2_000),
    });
    (d, "OnSecondChange")
}

/// 経路 4: 再生中にダブルクリックの `OnMouseDoubleClick` を送った（応答は今のトークを置き換える）。
fn steady_replace() -> (Drive, &'static str) {
    let mut d = Drive::new(steady_none(), cfg());
    d.start_talk(2_000, SHOWN);
    d.feed(Input::Mouse(crate::msg::MouseInput {
        scope: 0,
        x: 10,
        y: 20,
        region: Some("Head".to_string()),
        kind: MouseEventKind::DoubleClick {
            button: MouseButton::Left,
        },
    }));
    (d, "OnMouseDoubleClick")
}

/// 選択肢を表示したトーク（最終の台詞 [`SHOWN`]）が選択待ち（期限 7,000 ms）になった状態。
fn choosing() -> Drive {
    let mut d = Drive::new(steady_none(), cfg());
    let talk_id = d.start_talk(2_000, SHOWN);
    d.feed(Input::ChoiceWaiting {
        talk_id,
        choice_ids: vec![CHOICE_ID.to_string()],
        display_end: MonotonicMs(2_000),
        timeout_directive_secs: Some(5.0),
    });
    assert!(d.s.choice.is_some(), "選択待ちの帳簿が立つ");
    d
}

/// 経路 5: 選択肢を選び、任意名の選択のイベントを送った。
fn choice_cascade() -> (Drive, &'static str) {
    let mut d = choosing();
    d.feed(Input::Choice(ChoiceInput {
        id: CHOICE_ID.to_string(),
        label: "おやつ".to_string(),
        scope: 0,
        references: Vec::new(),
    }));
    (d, "OnChoiceEvent")
}

/// 経路 5: 選択待ちが期限に達し、`OnChoiceTimeout` を送った。
fn choice_timeout() -> (Drive, &'static str) {
    let mut d = choosing();
    d.feed(Input::Tick {
        now: MonotonicMs(7_000),
    });
    (d, "OnChoiceTimeout")
}

/// 印のイベント `OnShellChanging` を送って台詞の切れ目の見張りを始めた。
fn marked_shell_changing() -> (Drive, &'static str) {
    let mut d = Drive::new(steady_none(), cfg());
    d.feed(Input::AwaitTalkGap {
        raise: Some(GapRaise {
            id: "OnShellChanging".to_string(),
            references: vec!["B".to_string(), "ポスト".to_string()],
            method: ShioriMethod::Get,
        }),
    });
    (d, "OnShellChanging")
}

// ============================================================
// 8 か所の腕を 1 本ずつ（要件 1.2・1.5・3.1・5.5）
// ============================================================

#[test]
fn path_boot_greeting_returns_start_then_version_notice() {
    let (d, resumed) = assert_path(boot_first, &["StartTalk", "Notify:basewareversion"]);
    assert!(matches!(d.s.phase, Phase::BootVersion { talk: Some(_) }));
    // areka が足す起動の記録は翻訳に渡さず、最終の台詞の後ろにそのまま付く（要件 1.4）。
    assert_eq!(
        started(&resumed).epilogue,
        epilogue(),
        "起動の記録は触らない"
    );
}

#[test]
fn path_close_farewell() {
    let (d, _) = assert_path(close_farewell, &["StartTalk"]);
    assert!(matches!(d.s.phase, Phase::CloseTalkWait { .. }));
}

#[test]
fn path_ghost_changing_rewrites_the_change_script() {
    let (mut d, _) = assert_path(ghost_changing, &["StartTalk"]);
    let talk_id = match d.s.phase {
        Phase::ChangeTalkWait { talk_id, .. } => talk_id,
        _ => panic!("OnGhostChanging の台詞の完了待ちのはず"),
    };
    let script = |d: &Drive| d.s.change.as_ref().and_then(|c| c.script.clone());
    assert_eq!(
        script(&d).as_deref(),
        Some(FINAL),
        "切替の台詞の控えは最終の台詞（要件 6.2）"
    );
    // 台詞が終わって降ろすとき、停止通知の切替の中身の源（切替の帳簿の台詞）は最終の台詞のまま。
    d.feed(Input::TalkDone(TalkDone {
        talk_id,
        reason: TalkEndReason::Ended,
        quit_reserved: false,
    }));
    assert!(matches!(d.s.phase, Phase::Unloading { .. }));
    assert_eq!(script(&d).as_deref(), Some(FINAL));
}

#[test]
fn path_ghost_change_close_keeps_the_change_script() {
    let (d, _) = assert_path(ghost_change_close, &["StartTalk"]);
    assert!(matches!(d.s.phase, Phase::ChangeCloseTalkWait { .. }));
    assert_eq!(
        d.s.change.as_ref().map(|c| c.script.clone()),
        Some(None),
        "切替の OnClose の台詞では切替の台詞を書き換えない"
    );
}

#[test]
fn path_yielded_ghost_change_plays_as_a_steady_talk() {
    let (d, _) = assert_path(ghost_change_yielded, &["StartTalk"]);
    assert!(d.s.pending_close.is_some(), "終了の保留は残る");
    assert!(matches!(active_talk(&d.s.phase), Some(t) if t.origin == "OnGhostChanging"));
}

#[test]
fn path_steady_pump() {
    let (d, _) = assert_path(steady_pump, &["StartTalk"]);
    assert!(matches!(active_talk(&d.s.phase), Some(t) if t.origin == "OnSecondChange"));
}

#[test]
fn path_steady_replace() {
    let (d, _) = assert_path(steady_replace, &["StartTalk"]);
    assert!(matches!(active_talk(&d.s.phase), Some(t) if t.origin == "OnMouseDoubleClick"));
}

#[test]
fn path_choice_cascade_returns_resolve_then_start() {
    let (d, _) = assert_path(choice_cascade, &["ResolveChoice", "StartTalk"]);
    assert!(d.s.choice.is_none(), "選択の帳簿は解けている");
}

#[test]
fn path_choice_timeout() {
    let (d, _) = assert_path(choice_timeout, &["StartTalk"]);
    assert!(d.s.choice.is_none());
}

// ============================================================
// 翻訳しない台詞（要件 1.3・3.1・4.6）
// ============================================================

#[test]
fn boot_record_only_talk_made_by_areka_is_not_translated() {
    // OnFirstBoot も OnBoot も 204 → areka が起動の記録だけの台詞（台詞は空）を作る。
    let (mut d, origin) = boot_first();
    d.reply(ShioriOutcome::NoContent, origin);
    let mut actions = Vec::new();
    let ev = capture(|| actions = d.reply(ShioriOutcome::NoContent, "OnBoot"));
    assert_eq!(kinds(&actions), ["StartTalk", "Notify:basewareversion"]);
    let start = started(&actions);
    assert_eq!(start.script, "");
    assert_eq!(start.epilogue, epilogue());
    assert!(d.s.translate.is_none(), "翻訳の帳簿を置かない");
    assert_not_logged(&ev, "translate_begin");
    assert_not_logged(&ev, "translate_skipped_empty");
}

#[test]
fn empty_greeting_is_not_translated() {
    // SHIORI が 200 で 0 文字の台詞を返した（起動の記録は後ろに付く）。
    let (mut d, origin) = boot_first();
    let mut actions = Vec::new();
    let ev = capture(|| actions = d.reply(ShioriOutcome::Value(String::new()), origin));
    assert_eq!(kinds(&actions), ["StartTalk", "Notify:basewareversion"]);
    assert_eq!(started(&actions).script, "");
    assert_eq!(started(&actions).epilogue, epilogue());
    assert!(d.s.translate.is_none());
    logged_once(&ev, Level::TRACE, "translate_skipped_empty");
    assert_not_logged(&ev, "translate_begin");
}

#[test]
fn reply_to_on_translate_asked_through_the_talk_gap_is_not_translated() {
    // 台詞の切れ目の口から頼まれた `OnTranslate`（汎用の入口の側は translate_tests.rs が固定する）。
    let mut d = Drive::new(steady_none(), cfg());
    d.feed(Input::AwaitTalkGap {
        raise: Some(GapRaise {
            id: "OnTranslate".to_string(),
            references: vec![ORIGINAL.to_string()],
            method: ShioriMethod::Get,
        }),
    });
    let mut actions = Vec::new();
    let ev = capture(|| actions = d.reply(ShioriOutcome::Value(ORIGINAL.into()), "OnTranslate"));
    assert_eq!(kinds(&actions), ["StartTalk"]);
    assert_eq!(started(&actions).script, ORIGINAL);
    assert!(
        d.s.translate.is_none(),
        "OnTranslate を再び送らない（要件 4.6）"
    );
    logged_once(&ev, Level::DEBUG, "translate_skipped_self");
}

// ============================================================
// 見張りと控え（要件 5.5・6.1・6.2）
// ============================================================

#[test]
fn marked_talk_is_followed_through_the_translation() {
    // 通過と「翻訳なしと同じ見張り」は assert_path の ⑶ が比べる。
    let (d, _) = assert_path(marked_shell_changing, &["StartTalk"]);
    let talk_id = active_talk(&d.s.phase).expect("印の台詞を再生中").talk_id;
    let watch = d.s.talk_gap.as_ref().expect("見張りが在る");
    assert!(
        matches!(watch.marked, Marked::Talk(t) if t == talk_id),
        "印の台詞を追う"
    );
    assert!(watch.outcome.is_none(), "まだ決まっていない");

    // 捕まえた時点（翻訳の結果の前）で、見張りは既に印の台詞を追っている。
    let (mut d, origin) = marked_shell_changing();
    d.reply(ShioriOutcome::Value(ORIGINAL.into()), origin);
    let waiting = d.s.translate.as_ref().expect("帳簿を置く").talk_id;
    let watch = d.s.talk_gap.as_ref().expect("見張りが在る");
    assert!(matches!(watch.marked, Marked::Talk(t) if t == waiting));
    assert!(watch.outcome.is_none());
}

#[test]
fn choice_timeout_reference0_is_the_final_script() {
    // 選択待ちのトークは元の台詞 ORIGINAL を翻訳した SHOWN で再生している。
    let (d, _) = choice_timeout();
    let sent = d.sent.expect("OnChoiceTimeout を送った");
    assert_eq!(sent.id.as_str(), "OnChoiceTimeout");
    assert_eq!(
        sent.references,
        [SHOWN],
        "Reference0 は最終の台詞（要件 6.1）"
    );
}

// ============================================================
// 選択の連鎖の後の輸送路の失敗（論点 10・要件 4.5）
// ============================================================

#[test]
fn transport_failure_after_a_choice_chain_goes_to_fault() {
    let (mut d, origin) = choice_cascade();
    let actions = d.reply(ShioriOutcome::Value(ORIGINAL.into()), origin);
    assert_eq!(kinds(&actions), ["Translate"]);
    let mut actions = Vec::new();
    let ev = capture(|| {
        actions = d.feed(Input::TranslateDone(Err(ShioriFailure::Ipc(
            "通信が切れた".to_string(),
        ))))
    });
    assert_eq!(
        kinds(&actions),
        ["Unload"],
        "選択の例外（失敗を 204 と同じに扱い選択を解く）に揃えず、預けた一括を捨てて降ろす"
    );
    assert!(
        matches!(
            &d.s.phase,
            Phase::Unloading { cause: TermCause::Fault(fault) }
                if fault.kind == ShioriFaultKind::Disconnected
        ),
        "故障（Unloading の Fault）へ"
    );
    assert!(d.s.choice.is_none(), "選択の帳簿は残らない");
    assert!(d.s.translate.is_none());
    logged_once(&ev, Level::ERROR, "translate_failed");
    assert_not_logged(&ev, "choice_shiori_failed_as_204");
}
