use super::choice_tests::{
    MENU_SCRIPT, StateFixture, occupy, release, spawn_probe_talk, state_fixture,
};
use super::*;
use crate::test_log_capture::capture;
use areka_kanade::{AnchorInput, ChoiceInput};
use dola::time::testing::at_ms;
use tracing::Level;

// ── 押下の選択の取り次ぎ ──
//
// UI の押下の選択は `ChoiceWaiting` と同じ inbox に並び、dispatcher は判断せずに inbox の順のまま
// kanade へ送る（slot と突き合わせない・中身を変えない）。同期 state の檻で、inbox へ積む順を
// テストが決めて、kanade の口に着く順を見る。

/// 中身の改変を見分けられる選択肢の入力（前後空白・空文字の引数・さくらスクリプト風）。
fn dirty_choice_input() -> ChoiceInput {
    ChoiceInput {
        id: "  Onおしゃべり頻度  ".to_string(),
        label: r" \q[もどる,back] ＞＞ ".to_string(),
        scope: 1,
        references: vec!["  前後  ".to_string(), String::new(), r"\e".to_string()],
    }
}

/// 同じくアンカーの入力。
fn dirty_anchor_input() -> AnchorInput {
    AnchorInput {
        id: " https://example.invalid/ ".to_string(),
        text: "  あい ".to_string(),
        scope: 0,
        references: vec![String::new(), "  ".to_string()],
    }
}

/// 稼働中のトークの `ChoiceWaiting` の後に inbox へ積んだ選択は、間に時刻の前進とトークの起動を
/// 挟んでも、kanade の口へ `ChoiceWaiting` → `Choice` → `Anchor` の順で着き、中身は積んだまま
/// （`Start` が slot を差し替えても棄却しない）。
#[test]
fn pressed_selection_queued_after_choice_waiting_reaches_kanade_in_inbox_order() {
    let mut fx = state_fixture();
    let (probe, _obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(941);
    occupy(&mut fx.state, talk_id, probe);

    fx.feed(DispatcherMsg::Tick { now: at_ms(1_000) });
    fx.feed(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
        talk_id,
        choice_ids: vec!["targetA".to_string()],
        display_end: at_ms(350),
        timeout_directive: None,
    }));
    fx.feed(DispatcherMsg::Tick { now: at_ms(1_200) });
    fx.feed(DispatcherMsg::Start(StartTalk::new(
        TalkId(942),
        MENU_SCRIPT,
    )));
    fx.feed(DispatcherMsg::Choice(dirty_choice_input()));
    fx.feed(DispatcherMsg::Anchor(dirty_anchor_input()));

    let got: Vec<KanadeMsg> = fx.kanade_rx.try_iter().collect();
    assert!(
        matches!(
            &got[..],
            [
                KanadeMsg::ChoiceWaiting { talk_id: waited, .. },
                KanadeMsg::Choice(choice),
                KanadeMsg::Anchor(anchor),
            ] if *waited == talk_id
                && *choice == dirty_choice_input()
                && *anchor == dirty_anchor_input()
        ),
        "inbox の順のまま ChoiceWaiting → Choice → Anchor・中身は積んだまま（届いた数 {}）",
        got.len()
    );

    release(&mut fx.state);
}

/// slot が空でも選択肢とアンカーは stale として捨てず、そのまま kanade へ送る（受けるかどうかは
/// kanade が決める）。stale の記録も残さない。
#[test]
fn pressed_selection_with_empty_slot_is_forwarded_without_rejection() {
    let mut fx = state_fixture();
    assert_eq!(fx.state.current_talk_id(), None, "前提: slot は空");

    let events = capture(|| {
        fx.feed(DispatcherMsg::Choice(dirty_choice_input()));
        fx.feed(DispatcherMsg::Anchor(dirty_anchor_input()));
    });

    let got: Vec<KanadeMsg> = fx.kanade_rx.try_iter().collect();
    assert!(
        matches!(
            &got[..],
            [KanadeMsg::Choice(choice), KanadeMsg::Anchor(anchor)]
                if *choice == dirty_choice_input() && *anchor == dirty_anchor_input()
        ),
        "slot が空でも 2 件ともそのまま送る（届いた数 {}）",
        got.len()
    );
    assert!(
        events.iter().all(|e| e.level > Level::INFO),
        "棄却の記録（info 以上）を残さない: {:?}",
        events
            .iter()
            .map(|e| (e.level, e.event.clone(), e.message.clone()))
            .collect::<Vec<_>>()
    );
}

/// kanade の停止後は、選択肢は `choice_forward_failed`・アンカーは `anchor_forward_failed` の
/// `warn!` を 1 行ずつ残し、dispatcher は止まらずに続く（後の Tick も受ける）。
#[test]
fn pressed_selection_forward_failure_after_kanade_stopped_warns_once_and_continues() {
    let StateFixture {
        mut state,
        kanade_rx,
        _self_rx,
    } = state_fixture();
    drop(kanade_rx);

    let events = capture(|| {
        for msg in [
            DispatcherMsg::Choice(dirty_choice_input()),
            DispatcherMsg::Anchor(dirty_anchor_input()),
            DispatcherMsg::Tick { now: at_ms(5_000) },
        ] {
            assert_eq!(
                state.handle(msg),
                ControlFlow::Continue(()),
                "送れなくても dispatcher は止まらない（停止経路は Close のみ）"
            );
        }
    });

    let count = |name: &str| {
        events
            .iter()
            .filter(|e| {
                e.target == "areka_ghost::dispatcher"
                    && e.level == Level::WARN
                    && e.event.as_deref() == Some(name)
            })
            .count()
    };
    assert_eq!(
        (
            count("choice_forward_failed"),
            count("anchor_forward_failed")
        ),
        (1, 1),
        "種類ごとの名前で warn が 1 行ずつ: {:?}",
        events
            .iter()
            .map(|e| (e.level, e.event.clone(), e.message.clone()))
            .collect::<Vec<_>>()
    );
}
