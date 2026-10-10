//! アンカーの知らせの受理と 2 段の送出のテスト（areka-P0-anchor-tag-canon 要件 4.1〜4.5・4.9・4.10）。
//!
//! 受理（[`on_anchor`]）と応答（[`on_anchor_reply`]）を直に呼ぶ単体の判定と、最上位の `step` から
//! 細い腕（知らせの振り分け・応答の先頭の腕・送信の失敗の免除）を 1 回ずつ通す 2 本を置く。

use super::*;
use crate::msg::{KanadeConfig, MonotonicMs, ShioriCall, ShioriFailure};
use crate::schedule::log_capture::{capture, logged_once};
use crate::schedule::{ActiveTalk, Input, step};
use crate::talk::TalkId;
use tracing::Level;

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// 定常（話していない）。
fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 7,
        ..State::initial()
    }
}

/// 定常（話している最中）。
fn steady_talking() -> State {
    State {
        phase: Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id: TalkId(3),
                origin: "OnSecondChange",
                script: String::new(),
            }),
        },
        ..steady_none()
    }
}

fn anchor(id: &str, text: &str, references: &[&str]) -> AnchorInput {
    AnchorInput {
        id: id.to_string(),
        text: text.to_string(),
        scope: 1,
        references: references.iter().map(|s| s.to_string()).collect(),
    }
}

/// 一括が GET 1 本だけであることを確かめ、(イベント名, Reference の並び) を返す。
fn only_get(actions: &[Action]) -> (String, Vec<String>) {
    match actions {
        [Action::ShioriRequest(ShioriCall::Get { id, references, .. })] => {
            (id.as_str().to_string(), references.clone())
        }
        _ => panic!("GET 1 本だけのはず（{} 件）", actions.len()),
    }
}

/// 最終段（覚えている ID つき）。
fn final_of(id: &str) -> AnchorStage {
    AnchorStage::Final { id: id.to_string() }
}

fn failed() -> ShioriOutcome {
    ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string()))
}

// --- 段の決め方 ---

/// 要件 4.1・4.4: `On` で始まる ID だけが同じ名前のイベントになる。`script:` の約束は無い。
#[test]
fn plan_is_named_only_for_on_prefixed_ids() {
    assert_eq!(plan_anchor("OnMenu"), AnchorPlan::Named);
    assert_eq!(plan_anchor("On"), AnchorPlan::Named);
    for id in ["詳細", "on_menu", "ONMENU", "", "script:OnMenu"] {
        assert_eq!(plan_anchor(id), AnchorPlan::Canonical, "{id:?}");
    }
}

// --- 受理 ---

/// 要件 4.1: `On` で始まらない ID は `OnAnchorSelectEx` を 1 本積み、段を覚える。
#[test]
fn canonical_id_queues_select_ex_and_remembers_the_stage() {
    let mut out = None;
    let events = capture(|| {
        out = Some(on_anchor(
            steady_none(),
            anchor("詳細", "くわしく", &["a,b", ""]),
        ));
    });
    let (state, actions) = out.unwrap();
    assert_eq!(
        only_get(&actions),
        (
            "OnAnchorSelectEx".to_string(),
            vec![
                "くわしく".to_string(),
                "詳細".to_string(),
                "a,b".to_string(),
                String::new()
            ]
        )
    );
    assert_eq!(
        state.anchor,
        Some(AnchorStage::SelectEx {
            id: "詳細".to_string()
        })
    );
    assert!(state.choice.is_none(), "選択待ちの帳簿は作らない");
    assert!(matches!(state.phase, Phase::Steady { talk: None }));
    logged_once(&events, Level::INFO, "anchor_accepted");
}

/// 要件 4.4・4.5: `On` で始まる ID はその名前のイベント 1 本（Reference は引数だけ）で、最終段。
#[test]
fn on_prefixed_id_queues_the_named_event_as_the_final_stage() {
    let (state, actions) = on_anchor(
        steady_talking(),
        anchor("Onメニューを開く", "メニュー", &["r0", "r1"]),
    );
    assert_eq!(
        only_get(&actions),
        (
            "Onメニューを開く".to_string(),
            vec!["r0".to_string(), "r1".to_string()]
        )
    );
    assert_eq!(state.anchor, Some(final_of("Onメニューを開く")));
    assert!(
        matches!(state.phase, Phase::Steady { talk: Some(_) }),
        "受理では再生中の台詞に触れない"
    );
}

/// 定常以外では警告だけ残して棄却する（何も積まない・段も覚えない）。
#[test]
fn anchor_outside_steady_is_rejected_with_a_warning() {
    let mut out = None;
    let events = capture(|| {
        out = Some(on_anchor(State::initial(), anchor("詳細", "くわしく", &[])));
    });
    let (state, actions) = out.unwrap();
    assert!(actions.is_empty());
    assert!(state.anchor.is_none());
    assert!(matches!(state.phase, Phase::Idle));
    logged_once(&events, Level::WARN, "anchor_rejected_phase");
}

// --- 応答 ---

/// 要件 4.3・4.8: 台本はどの段でも腕で扱わない（呼び手が既存の応答の腕へ流す）。
#[test]
fn script_reply_is_left_to_the_existing_reply_arm() {
    for stage in [
        AnchorStage::SelectEx {
            id: "詳細".to_string(),
        },
        final_of("詳細"),
    ] {
        let mut state = steady_none();
        let handled = on_anchor_reply(
            &mut state,
            stage,
            &ShioriOutcome::Value("\\0やあ\\e".to_string()),
            "OnAnchorSelectEx",
        );
        assert!(handled.is_none());
        assert!(state.anchor.is_none(), "段の記憶は残さない");
    }
}

/// 要件 4.2: `OnAnchorSelectEx` が 204 なら `OnAnchorSelect`（Reference0＝ID）を積んで最終段へ。
#[test]
fn no_content_at_select_ex_queues_select_and_moves_to_final() {
    let mut state = steady_none();
    let actions = on_anchor_reply(
        &mut state,
        AnchorStage::SelectEx {
            id: "詳細".to_string(),
        },
        &ShioriOutcome::NoContent,
        "OnAnchorSelectEx",
    )
    .expect("204 は腕が捌く");
    assert_eq!(
        only_get(&actions),
        ("OnAnchorSelect".to_string(), vec!["詳細".to_string()])
    );
    assert_eq!(state.anchor, Some(final_of("詳細")));
}

/// 要件 4.9: 最終段の 204 では何もしない（段の記憶も残らない）。
#[test]
fn no_content_at_final_does_nothing() {
    let mut state = steady_talking();
    let actions = on_anchor_reply(
        &mut state,
        final_of("詳細"),
        &ShioriOutcome::NoContent,
        "OnAnchorSelect",
    )
    .expect("204 は腕が捌く");
    assert!(actions.is_empty());
    assert!(state.anchor.is_none());
    assert!(matches!(state.phase, Phase::Steady { talk: Some(_) }));
}

/// 要件 4.10: 送信の失敗と想定外の応答は、記録を残して 204 と同じに進む。
#[test]
fn failure_and_unexpected_reply_advance_like_no_content_with_a_record() {
    for (outcome, level, event) in [
        (failed(), Level::ERROR, "anchor_shiori_failed_as_204"),
        (
            ShioriOutcome::Notified,
            Level::WARN,
            "anchor_unexpected_reply",
        ),
    ] {
        let mut state = steady_none();
        let mut actions = None;
        let events = capture(|| {
            actions = on_anchor_reply(
                &mut state,
                AnchorStage::SelectEx {
                    id: "詳細".to_string(),
                },
                &outcome,
                "OnAnchorSelectEx",
            );
        });
        assert_eq!(
            only_get(&actions.expect("腕が捌く")).0,
            "OnAnchorSelect",
            "{event}"
        );
        assert_eq!(state.anchor, Some(final_of("詳細")));
        logged_once(&events, level, event);
    }
}

/// 最終段の失敗と想定外の応答も、どのアンカーだったか（ID）と段を記録に残して何もしない。
/// 空の ID も正しい ID なので、最終段だからといって ID を空で記録しない。
#[test]
fn failure_and_unexpected_reply_at_final_record_the_id() {
    for id in ["詳細", "Onメニューを開く"] {
        for (outcome, level, event) in [
            (failed(), Level::ERROR, "anchor_shiori_failed_as_204"),
            (
                ShioriOutcome::Notified,
                Level::WARN,
                "anchor_unexpected_reply",
            ),
        ] {
            let mut state = steady_talking();
            let mut actions = None;
            let events = capture(|| {
                actions = on_anchor_reply(&mut state, final_of(id), &outcome, "OnAnchorSelect");
            });
            assert!(actions.expect("腕が捌く").is_empty(), "{event}");
            assert!(state.anchor.is_none());
            let record = logged_once(&events, level, event);
            assert_eq!(
                record.fields.get("id").map(String::as_str),
                Some(id),
                "{event}"
            );
            assert_eq!(
                record.fields.get("stage").map(String::as_str),
                Some("final"),
                "{event}"
            );
        }
    }
}

// --- 最上位の `step` から細い腕を通す ---

/// 知らせの振り分け → 送信の失敗が致命へ倒れない → 続きの `OnAnchorSelect` → 204 で静かに終わる。
#[test]
fn send_failure_through_step_keeps_running_and_falls_back_to_select() {
    let (state, actions) = step(
        steady_none(),
        Input::Anchor(anchor("詳細", "くわしく", &[])),
        &config(),
    );
    assert_eq!(only_get(&actions).0, "OnAnchorSelectEx");

    let (state, actions) = step(
        state,
        Input::ShioriReply {
            outcome: failed(),
            origin: "OnAnchorSelectEx",
        },
        &config(),
    );
    assert!(
        matches!(state.phase, Phase::Steady { talk: None }),
        "アンカーの送信の失敗で終了へ倒れない"
    );
    assert_eq!(
        only_get(&actions),
        ("OnAnchorSelect".to_string(), vec!["詳細".to_string()])
    );

    let (state, actions) = step(
        state,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnAnchorSelect",
        },
        &config(),
    );
    assert!(actions.is_empty());
    assert!(state.anchor.is_none());
    assert!(matches!(state.phase, Phase::Steady { talk: None }));
}

/// 台本の応答は既存の応答の腕が受けて台詞を始め、`OnAnchorSelect` は送らない。
#[test]
fn script_reply_through_step_starts_one_talk_without_select() {
    let (state, _) = step(
        steady_none(),
        Input::Anchor(anchor("詳細", "くわしく", &[])),
        &config(),
    );
    let (state, actions) = crate::schedule::translate_test_support::pass_translate(
        step(
            state,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("\\0やあ\\e".to_string()),
                origin: "OnAnchorSelectEx",
            },
            &config(),
        ),
        &config(),
    );
    assert!(
        matches!(actions.as_slice(), [Action::StartTalk(start)] if start.talk_id == TalkId(7)),
        "台詞の起動が 1 回だけ"
    );
    assert!(state.anchor.is_none());
    assert!(matches!(state.phase, Phase::Steady { talk: Some(_) }));
}

/// 知らせ 1 件と模擬の応答の列を `step` から通し、イベントの列と台詞の起動を数えるテスト。
#[path = "anchor_step_tests.rs"]
mod step_tests;
