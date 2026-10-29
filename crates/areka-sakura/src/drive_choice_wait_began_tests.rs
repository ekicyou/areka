use super::test_support::NEG_WINDOW;
use super::*;
use crate::contract::{CueCommand, ScriptOrigin, TalkCue, TalkId};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

// ── areka-P0-balloon-text-bugs task 4.2: 選択待ちの始まりの配り（要件 1.10/1.11/4.1） ──
//
// 再生（talk スレッド）は `ChoiceWaiting` を done の口へ送った**後**に、登録された受け口へ
// `choice_wait_began` を 1 回配る。順が逆だと、押下が kanade の待ちより先に着く窓が開く
// （design.md「順の保証」案 (b)）。檻は done の口と受け口の両方を**同じチャンネル**へ流し、
// talk スレッドの上で起きた順をそのまま観測する（同一スレッドの送信順＝受信順）。

/// done の口と受け口の出来事を 1 本の列に並べる観測用の型。
#[derive(Debug, Clone, PartialEq)]
enum Event {
    Done(TalkDone),
    ChoiceWaiting,
    Cue(CueCommand),
    WaitBegan,
}

impl From<TalkDone> for Event {
    fn from(done: TalkDone) -> Self {
        Event::Done(done)
    }
}

impl From<ChoiceWaiting> for Event {
    fn from(_: ChoiceWaiting) -> Self {
        Event::ChoiceWaiting
    }
}

/// cue と選択待ちの始まりを、done の口と同じチャンネルへ流す受け口。
struct EventSink {
    tx: Sender<Event>,
}

impl CueSink for EventSink {
    fn emit(&mut self, cue: TalkCue) {
        let _ = self.tx.send(Event::Cue(cue.command));
    }

    fn choice_wait_began(&mut self) {
        let _ = self.tx.send(Event::WaitBegan);
    }
}

/// 窓が明けるまでに届いた出来事を全部集める（cue は読み飛ばさない）。
fn drain(rx: &mpsc::Receiver<Event>, window: Duration) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(event) = rx.recv_timeout(window) {
        events.push(event);
    }
    events
}

fn spawn(script: &str, talk_id: u64) -> (TalkHandle, mpsc::Receiver<Event>) {
    let (tx, rx) = mpsc::channel::<Event>();
    let handle = spawn_talk(
        StartTalk {
            epilogue: Vec::new(),
            script: script.to_string(),
            talk_id: TalkId(talk_id),
            origin: ScriptOrigin::GHOST,
            strict: false,
        },
        tx.clone(),
        vec![Box::new(EventSink { tx })],
        SystemVarSnapshot::default(),
    );
    (handle, rx)
}

fn not_cue(event: &Event) -> bool {
    !matches!(event, Event::Cue(_))
}

/// **送った後に 1 回・1 トークに 1 回**: 選択待ちに入った tick で `ChoiceWaiting` が done の口へ
/// 送られ、その**後に** `choice_wait_began` が 1 回届く。待ちが続く間に Tick を重ねても 2 度目は
/// 来ず、解決して終わるまでに通算 1 回。
#[test]
fn choice_wait_began_follows_choice_waiting_once_per_talk() {
    let (handle, rx) = spawn(r"\s[10]hello\_w[100]\q[選択A,targetA]\e", 840);

    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(5.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(50.0)).unwrap();
    let waiting = drain(&rx, NEG_WINDOW);
    let order: Vec<Event> = waiting.iter().filter(|e| not_cue(e)).cloned().collect();
    assert_eq!(
        order,
        vec![Event::ChoiceWaiting, Event::WaitBegan],
        "ChoiceWaiting を done の口へ送った後に choice_wait_began が 1 回だけ届く: {waiting:?}"
    );
    // 選択肢の cue は選択待ちの始まりより前に届いている（始まりは状態の移りの後）。
    let choice_at = waiting
        .iter()
        .position(|e| matches!(e, Event::Cue(CueCommand::Choice { .. })))
        .expect("選択肢の cue が届く");
    let began_at = waiting.iter().position(|e| *e == Event::WaitBegan).unwrap();
    assert!(choice_at < began_at, "選択肢の cue の後に始まりが届く");

    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "targetA".to_string(),
        })
        .unwrap();
    handle.actor.join().expect("body は正常終了する");
    let rest: Vec<Event> = drain(&rx, Duration::from_millis(50))
        .into_iter()
        .filter(not_cue)
        .collect();
    assert_eq!(
        rest,
        vec![Event::Done(TalkDone {
            talk_id: TalkId(840),
            reason: TalkEndReason::Ended,
            quit_reserved: false,
        })],
        "解決の後に始まりは再び配られない（1 トークに 1 回）"
    );
}

/// **最後に `\w` の待ちがあっても区切りに入るまで配らない**: 選択肢の cue が届いた後も、
/// 後ろの待ちが明けて選択待ちに入るまでは `ChoiceWaiting` も始まりも出ない。明けた tick で
/// `ChoiceWaiting` → 始まりの順に 1 回ずつ届く。
#[test]
fn choice_wait_began_waits_for_trailing_wait_to_elapse() {
    let (handle, rx) = spawn(r"\s[10]\q[選択A,targetA]\_w[1000]", 841);

    // 選択肢は at=0 で届くが、後ろの 1 秒の待ちの途中（0.5）ではまだ選択待ちでない。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
    let before = drain(&rx, NEG_WINDOW);
    assert!(
        before
            .iter()
            .any(|e| matches!(e, Event::Cue(CueCommand::Choice { .. }))),
        "選択肢の cue は待ちの前に届いている: {before:?}"
    );
    assert!(
        before.iter().all(|e| matches!(e, Event::Cue(_))),
        "待ちが明けるまでは ChoiceWaiting も始まりも出ない: {before:?}"
    );

    // 待ちが明ける tick で選択待ちに入る。
    handle.inbox.send(SakuraMsg::Tick(2.0)).unwrap();
    let after: Vec<Event> = drain(&rx, NEG_WINDOW).into_iter().filter(not_cue).collect();
    assert_eq!(after, vec![Event::ChoiceWaiting, Event::WaitBegan]);

    handle.inbox.send(SakuraMsg::Close).unwrap();
    handle.actor.join().expect("body は正常終了する");
}

/// **選択肢の無いトークは配らない**: 選択待ちに入らないトークでは始まりは一度も届かない。
#[test]
fn script_without_choices_never_announces_choice_wait() {
    let (handle, rx) = spawn(r"\s[10]hello\e", 842);
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
    handle.actor.join().expect("body は正常終了する");
    let events = drain(&rx, Duration::from_millis(50));
    assert!(
        !events.contains(&Event::WaitBegan),
        "選択待ちに入らないトークは始まりを配らない: {events:?}"
    );
    assert_eq!(
        rx.recv_timeout(Duration::from_millis(10)).unwrap_err(),
        RecvTimeoutError::Disconnected
    );
}
