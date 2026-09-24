use super::*;
use crate::msg::{CloseReason, ShioriCall, ShioriFailure};
use crate::schedule::{ActiveTalk, TermCause};
use crate::talk::TalkId;
use areka_actor::reply_channel;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

fn active_talk() -> ActiveTalk {
    ActiveTalk {
        talk_id: TalkId(7),
        origin: "test",
        script: String::new(),
    }
}

/// 運行フェーズの全種類（会話の有無で形が変わるものは両方）。
fn all_phases() -> Vec<Phase> {
    vec![
        Phase::Idle,
        Phase::BootInit,
        Phase::BootPrefetch,
        Phase::BootType,
        Phase::BootMain,
        Phase::BootVersion { talk: None },
        Phase::BootVersion {
            talk: Some(active_talk()),
        },
        Phase::Steady { talk: None },
        Phase::Steady {
            talk: Some(active_talk()),
        },
        Phase::ClosePending {
            reason: CloseReason::User { scope: 0 },
        },
        Phase::CloseTalkWait {
            talk_id: TalkId(7),
            deadline: None,
        },
        Phase::Unloading {
            cause: TermCause::Quit,
        },
        Phase::Unloading {
            cause: TermCause::Fault(crate::msg::ShioriFault::unknown()),
        },
        Phase::Stopped,
    ]
}

/// 期待値の表。wildcard を置かないので、`Phase` に種類が増えたらここでの判断が
/// コンパイル時に要求される（新しいフェーズを黙って「照会できる」にしない）。
fn expected_queryable(phase: &Phase) -> bool {
    match phase {
        Phase::Steady { .. } => true,
        Phase::Idle
        | Phase::BootInit
        | Phase::BootPrefetch
        | Phase::BootType
        | Phase::BootMain
        | Phase::BootVersion { .. }
        | Phase::ClosePending { .. }
        | Phase::CloseTalkWait { .. }
        | Phase::Unloading { .. }
        | Phase::Stopped => false,
    }
}

fn state_in(phase: Phase) -> State {
    let mut state = State::initial();
    state.phase = phase;
    state
}

/// 会話できる状態（`Steady`）だけが照会できる（要件 3.10・終了中は既定名）。
#[test]
fn only_steady_is_queryable() {
    let phases = all_phases();
    let mut queryable_count = 0;
    for phase in &phases {
        assert_eq!(
            queryable(phase),
            expected_queryable(phase),
            "フェーズごとの照会可否が期待と違う"
        );
        if queryable(phase) {
            queryable_count += 1;
        }
    }
    // 表の両側（真・偽）を実際に踏んでいること: 真は Steady の 2 形だけ。
    assert_eq!(queryable_count, 2, "照会できるのは Steady の 2 形だけ");
    assert_eq!(
        phases.len() - queryable_count,
        12,
        "残り 12 形は照会できない"
    );
}

/// SHIORI 応答から照会結果への写像（`boot.rs` の prefetch 段と同じ 3 分岐＋想定外）。
#[test]
fn shiori_outcome_maps_onto_resource_outcome() {
    assert_eq!(
        outcome_of("readmebutton.caption", ShioriOutcome::Value("x".into())),
        ResourceOutcome::Value("x".into())
    );
    // 空文字の 200 は値のまま返す（既定名へ倒すのは受け手の解釈・要件 3.3）。
    assert_eq!(
        outcome_of("readmebutton.caption", ShioriOutcome::Value(String::new())),
        ResourceOutcome::Value(String::new())
    );
    assert_eq!(
        outcome_of("readmebutton.caption", ShioriOutcome::NoContent),
        ResourceOutcome::NoContent
    );
    let failure = ShioriFailure::Timeout("1000ms".into());
    let reason = failure.to_string();
    assert_eq!(
        outcome_of("readmebutton.caption", ShioriOutcome::Failed(failure)),
        ResourceOutcome::Failed(reason)
    );
    // GET の応答としてあり得ない完了語彙は失敗へ倒す（panic しない）。
    for unexpected in [ShioriOutcome::Notified, ShioriOutcome::Unloaded] {
        assert_eq!(
            outcome_of("readmebutton.caption", unexpected),
            ResourceOutcome::Failed("unexpected".into())
        );
    }
}

/// 会話できない状態では SHIORI へ 1 通も送らず、全件「値なし」を同順・同長で 1 回返す（要件 3.10）。
#[test]
fn non_queryable_phase_answers_no_content_without_touching_shiori() {
    for phase in all_phases().into_iter().filter(|p| !expected_queryable(p)) {
        let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
        let (reply_tx, reply_rx) = reply_channel();
        answer(
            &state_in(phase),
            &shiori_tx,
            vec!["readmebutton.caption", "sakura.popupmenu.visible"],
            reply_tx,
        );
        assert_eq!(
            reply_rx.recv().expect("応答は必ず 1 回返る"),
            vec![
                ("readmebutton.caption", ResourceOutcome::NoContent),
                ("sakura.popupmenu.visible", ResourceOutcome::NoContent),
            ]
        );
        assert!(
            matches!(shiori_rx.try_recv(), Err(TryRecvError::Empty)),
            "会話できない状態では SHIORI へ送らない"
        );
    }
}

/// 会話できる状態では id ごとに往復する。許可されない id はその id だけ失敗になり、
/// 残りは答えが返る（同順・同長）。
#[test]
fn steady_round_trips_each_id_and_rejects_only_the_disallowed_one() {
    let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
    // 届いた GET の id をそのまま値にして返すだけの偽 SHIORI。
    let fake = thread::spawn(move || {
        let mut seen = Vec::new();
        while let Ok(ShioriMsg::Request { call, reply }) = shiori_rx.recv() {
            let ShioriCall::Get { id, status, .. } = call else {
                panic!("リソース照会は GET");
            };
            seen.push((id.as_str().to_string(), status.render()));
            let _ = reply.send(ShioriOutcome::Value(format!("<{}>", id.as_str())));
        }
        seen
    });
    let (reply_tx, reply_rx) = reply_channel();
    answer(
        &state_in(Phase::Steady {
            talk: Some(active_talk()),
        }),
        &shiori_tx,
        vec!["closebutton.caption", "notaresource", "username"],
        reply_tx,
    );
    drop(shiori_tx);
    let replied = reply_rx.recv().expect("応答は必ず 1 回返る");
    assert_eq!(replied.len(), 3, "入力と同じ長さ");
    assert_eq!(
        replied[0],
        (
            "closebutton.caption",
            ResourceOutcome::Value("<closebutton.caption>".into())
        )
    );
    assert_eq!(replied[1].0, "notaresource");
    assert!(
        matches!(&replied[1].1, ResourceOutcome::Failed(why) if why.contains("notaresource")),
        "許可されない id はその id だけ失敗になる: {:?}",
        replied[1].1
    );
    assert_eq!(
        replied[2],
        ("username", ResourceOutcome::Value("<username>".into()))
    );
    // 許可されない id は SHIORI へ出ていない。Status は現在の状態（会話中）から導出される。
    assert_eq!(
        fake.join().expect("偽 SHIORI は panic しない"),
        vec![
            (
                "closebutton.caption".to_string(),
                Some("talking".to_string())
            ),
            ("username".to_string(), Some("talking".to_string())),
        ]
    );
}

/// 受信側が待ちを諦めて受信端を捨てていても落ちない。
#[test]
fn dropped_reply_receiver_does_not_panic() {
    let (shiori_tx, _shiori_rx) = mpsc::channel::<ShioriMsg>();
    let (reply_tx, reply_rx) = reply_channel();
    drop(reply_rx);
    answer(
        &state_in(Phase::Idle),
        &shiori_tx,
        vec!["readmebutton.caption"],
        reply_tx,
    );
}
