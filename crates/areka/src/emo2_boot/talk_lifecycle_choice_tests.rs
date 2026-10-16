// =============================================================================
// BalloonLifecycleSink の「選択肢を出した」の合図の檻（areka-P0-balloon-text-bugs task 3.1・
// 要件 1.2 / 1.3 / 1.4 / 1.8 / 1.13 / 4.1）
//
//   ⑴ 選択肢の合図 2 つでも `ChoiceShown` は 1 回・`TalkStarted` の後で `TalkEnded` の前
//   ⑵ 選択肢の無いトーク・アンカーだけのトークでは送らない
//   ⑶ 複製ごとに「送った」の印が戻る（次のトークでもまた 1 回送る）
//   ⑷ スコープを問わない（別の話し手の選択肢でも同じ 1 回）
// =============================================================================

use super::*;
use crate::emo2_boot::talk_clock::TalkClock;
use areka_ghost::sink::BootCueSink;
use dola::cue::{ActorKey, CueCommand, TalkCue};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

fn cue(actor: &str, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

fn choice(actor: &str, id: &str) -> TalkCue {
    cue(
        actor,
        CueCommand::Choice {
            id: id.into(),
            text: id.into(),
            references: vec![],
        },
    )
}

fn text(actor: &str) -> TalkCue {
    cue(actor, CueCommand::Text("あ".into()))
}

/// 登録の原本を作る（終わりの時刻はこの檻の関心外なので、時計は止めたままでよい）。
fn registered() -> (BalloonLifecycleSink, Receiver<TalkLifecycleSignal>) {
    let (tx, rx) = channel();
    let clock = TalkClock::new(Arc::new(|| 0.0));
    (BalloonLifecycleSink::new(tx, clock), rx)
}

/// 1 トークぶんの受け口へ cue を流して落とし、届いた合図の列を返す。
fn run_talk(
    registered: &BalloonLifecycleSink,
    rx: &Receiver<TalkLifecycleSignal>,
    cues: Vec<TalkCue>,
) -> Vec<TalkLifecycleSignal> {
    let mut sink: Box<dyn BootCueSink> = registered.clone_box();
    for c in cues {
        sink.emit(c);
    }
    drop(sink);
    rx.try_iter().collect()
}

fn count_choice_shown(signals: &[TalkLifecycleSignal]) -> usize {
    signals
        .iter()
        .filter(|s| matches!(s, TalkLifecycleSignal::ChoiceShown))
        .count()
}

fn position(signals: &[TalkLifecycleSignal], pred: impl Fn(&TalkLifecycleSignal) -> bool) -> usize {
    signals
        .iter()
        .position(pred)
        .unwrap_or_else(|| panic!("合図が見つからない: {signals:?}"))
}

/// ⑴ 選択肢の合図 2 つで 1 回だけ、開始の後で終わりの前に送る。
#[test]
fn two_choices_send_choice_shown_once_between_start_and_end() {
    let (registered, rx) = registered();
    let signals = run_talk(
        &registered,
        &rx,
        vec![text("0"), choice("0", "OnA"), choice("0", "OnB")],
    );

    assert_eq!(count_choice_shown(&signals), 1, "{signals:?}");
    let started = position(&signals, |s| {
        matches!(s, TalkLifecycleSignal::TalkStarted { .. })
    });
    let shown = position(&signals, |s| matches!(s, TalkLifecycleSignal::ChoiceShown));
    let ended = position(&signals, |s| {
        matches!(s, TalkLifecycleSignal::TalkEnded { .. })
    });
    assert!(started < shown && shown < ended, "{signals:?}");
}

/// ⑴ 選択肢が台本の先頭の cue でも、開始の合図が先に出る。
#[test]
fn choice_as_first_cue_still_follows_talk_started() {
    let (registered, rx) = registered();
    let signals = run_talk(&registered, &rx, vec![choice("0", "OnA")]);
    assert!(
        matches!(signals[0], TalkLifecycleSignal::TalkStarted { .. }),
        "{signals:?}"
    );
    assert_eq!(count_choice_shown(&signals), 1, "{signals:?}");
}

/// ⑵ 選択肢の無いトークでは送らない。
#[test]
fn talk_without_choices_sends_no_choice_shown() {
    let (registered, rx) = registered();
    let signals = run_talk(&registered, &rx, vec![text("0"), text("1")]);
    assert_eq!(count_choice_shown(&signals), 0, "{signals:?}");
}

/// ⑵ アンカーだけのトークでは送らない（アンカーは選択肢に数えない）。
#[test]
fn anchor_only_talk_sends_no_choice_shown() {
    let (registered, rx) = registered();
    let signals = run_talk(
        &registered,
        &rx,
        vec![
            cue(
                "0",
                CueCommand::AnchorBegin {
                    id: "OnJump".into(),
                    references: vec![],
                },
            ),
            text("0"),
            cue("0", CueCommand::AnchorEnd),
        ],
    );
    assert_eq!(count_choice_shown(&signals), 0, "{signals:?}");
}

/// ⑶ 複製ごとに印が戻る——選択肢つきのトークの次のトークでもまた 1 回、選択肢の無い
/// トークでは 0 回。
#[test]
fn each_clone_resets_the_sent_mark() {
    let (registered, rx) = registered();
    let first = run_talk(&registered, &rx, vec![choice("0", "OnA")]);
    let second = run_talk(&registered, &rx, vec![choice("0", "OnB")]);
    let third = run_talk(&registered, &rx, vec![text("0")]);
    assert_eq!(
        (
            count_choice_shown(&first),
            count_choice_shown(&second),
            count_choice_shown(&third)
        ),
        (1, 1, 0)
    );
}

/// ⑷ スコープを問わない——別の話し手の選択肢が混ざっても 1 回。
#[test]
fn choices_in_any_scope_count_once() {
    let (registered, rx) = registered();
    let signals = run_talk(
        &registered,
        &rx,
        vec![text("1"), choice("1", "OnA"), choice("0", "OnB")],
    );
    assert_eq!(count_choice_shown(&signals), 1, "{signals:?}");
}
