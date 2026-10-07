//! `script:` の選択肢を受け付けた後の結末のテスト（最上位の `schedule::step` から通す）。

use super::super::test_support::{choice_input_of, config, expect_get_call, steady_with_ledger};
use crate::msg::MonotonicMs;
use crate::schedule::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use crate::schedule::{Action, ActiveTalk, ChoicePhase, Input, Phase, step};
use crate::talk::TalkId;
use tracing::Level;

/// 要件 7.1: `\q[バルーンを閉じる,script:\e]` を選ぶと、SHIORI のイベントも翻訳も起こさず、
/// 待ちを閉じて `\e` を新しいトークとして始める。
#[test]
fn script_choice_starts_new_talk_with_text_after_prefix() {
    let s = steady_with_ledger(TalkId(3), 6, &["script:\\e"], ChoicePhase::Waiting);
    let mut result = None;
    let ev = capture(|| {
        result = Some(step(
            s,
            Input::Choice(choice_input_of("script:\\e", "バルーンを閉じる", &[])),
            &config(),
        ));
    });
    let (next, actions) = result.expect("step を呼んだはず");

    match actions.as_slice() {
        [
            Action::ResolveChoice { talk_id, id },
            Action::StartTalk(start),
        ] => {
            assert_eq!(*talk_id, TalkId(3), "解決するのは元のトーク");
            assert_eq!(id, "script:\\e", "解決の ID は選んだ ID のまま");
            assert_eq!(start.talk_id, TalkId(6), "新しいトークの番号");
            assert_eq!(start.script, "\\e", "台本は script: の後ろそのまま");
        }
        _ => panic!(
            "解決 → 再生開始のちょうど 2 つのはず（数 {}）",
            actions.len()
        ),
    }
    assert!(next.choice.is_none(), "待ちの帳簿は消える");
    assert_eq!(next.next_talk_id, 7, "採番が 1 進む");
    assert_eq!(
        next.choice_prev_talk,
        Some(TalkId(3)),
        "元のトークを 1 世代控える"
    );
    match &next.phase {
        Phase::Steady {
            talk:
                Some(ActiveTalk {
                    talk_id,
                    origin,
                    script,
                }),
        } => {
            assert_eq!(*talk_id, TalkId(6));
            assert_eq!(*origin, "choice_script");
            assert_eq!(script, "\\e");
        }
        _ => panic!("枠は新しいトークのはず"),
    }

    let started = logged_once(&ev, Level::INFO, "choice_script_started");
    assert_eq!(
        started.fields.get("choice_id").map(String::as_str),
        Some("script:\\e")
    );
    assert_eq!(started.fields.get("talk_id").map(String::as_str), Some("6"));
    assert_eq!(
        started.fields.get("prev_talk_id").map(String::as_str),
        Some("3")
    );
    let resolved = logged_once(&ev, Level::INFO, "choice_resolved");
    assert_eq!(resolved.outcome.as_deref(), Some("script"));
    assert!(
        position_of(&ev, "choice_script_started") < position_of(&ev, "choice_resolved"),
        "実行の記録は解決の記録より先に出る"
    );
    assert_not_logged(&ev, "choice_script_unused_args");
}

/// 要件 7.2・2.3・2.4: 正典の入れ子の例 `\q[その１,"script:\q[その２,script:その３はない]"]`。
/// 「その１」を選ぶと選択肢「その２」の台本が始まり、その待ちで「その２」を選ぶと
/// `その３はない` が始まる。
#[test]
fn nested_script_choice_runs_two_stages() {
    let first_id = "script:\\q[その２,script:その３はない]";
    let s = steady_with_ledger(TalkId(3), 6, &[first_id], ChoicePhase::Waiting);
    let (s, actions) = step(
        s,
        Input::Choice(choice_input_of(first_id, "その１", &[])),
        &config(),
    );
    match actions.as_slice() {
        [
            Action::ResolveChoice { talk_id, .. },
            Action::StartTalk(start),
        ] => {
            assert_eq!(*talk_id, TalkId(3));
            assert_eq!(start.talk_id, TalkId(6));
            assert_eq!(
                start.script, "\\q[その２,script:その３はない]",
                "1 段目の台本は選択肢「その２」"
            );
        }
        _ => panic!("1 段目は解決 → 再生開始のはず（数 {}）", actions.len()),
    }

    // 新しいトークの再生が選択肢の待ちに入った知らせ。
    let (s, actions) = step(
        s,
        Input::ChoiceWaiting {
            talk_id: TalkId(6),
            choice_ids: vec!["script:その３はない".to_string()],
            display_end: MonotonicMs(1_000),
            timeout_directive_secs: None,
        },
        &config(),
    );
    assert!(actions.is_empty(), "待ちの知らせは行動を出さない");
    assert!(s.choice.is_some(), "新しいトークの待ちができる");

    let (next, actions) = step(
        s,
        Input::Choice(choice_input_of("script:その３はない", "その２", &[])),
        &config(),
    );
    match actions.as_slice() {
        [
            Action::ResolveChoice { talk_id, id },
            Action::StartTalk(start),
        ] => {
            assert_eq!(*talk_id, TalkId(6), "解決するのは 1 段目で始めたトーク");
            assert_eq!(id, "script:その３はない");
            assert_eq!(start.talk_id, TalkId(7));
            assert_eq!(start.script, "その３はない", "2 段目の台本");
        }
        _ => panic!("2 段目は解決 → 再生開始のはず（数 {}）", actions.len()),
    }
    assert_eq!(next.choice_prev_talk, Some(TalkId(6)));
}

/// 要件 7.3・3.1・3.2・5.3: `script:` の後ろが空なら、トークを始めず待ちだけを閉じる。
#[test]
fn empty_script_choice_only_resolves() {
    let s = steady_with_ledger(TalkId(3), 6, &["script:"], ChoicePhase::Waiting);
    let mut result = None;
    let ev = capture(|| {
        result = Some(step(
            s,
            Input::Choice(choice_input_of("script:", "空", &[])),
            &config(),
        ));
    });
    let (next, actions) = result.expect("step を呼んだはず");

    match actions.as_slice() {
        [Action::ResolveChoice { talk_id, id }] => {
            assert_eq!(*talk_id, TalkId(3));
            assert_eq!(id, "script:");
        }
        _ => panic!("解決ちょうど 1 つのはず（数 {}）", actions.len()),
    }
    assert!(next.choice.is_none(), "待ちの帳簿は消える");
    assert_eq!(next.next_talk_id, 6, "採番は変わらない");
    assert_eq!(next.choice_prev_talk, None, "元のトークを控えない");
    match &next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(*talk_id, TalkId(3), "枠は元のトークのまま"),
        _ => panic!("枠は元のトークのはず"),
    }

    let empty = logged_once(&ev, Level::WARN, "choice_script_empty");
    assert_eq!(
        empty.fields.get("choice_id").map(String::as_str),
        Some("script:")
    );
    let resolved = logged_once(&ev, Level::INFO, "choice_resolved");
    assert_eq!(resolved.outcome.as_deref(), Some("script_empty"));
    assert_not_logged(&ev, "choice_script_started");
}

/// 要件 7.3・3.3: 第 3 引数以降は台本に含めず、数だけを警告に残す。
#[test]
fn script_choice_ignores_extra_arguments_and_warns_count() {
    let s = steady_with_ledger(TalkId(3), 6, &["script:\\![open"], ChoicePhase::Waiting);
    let mut result = None;
    let ev = capture(|| {
        result = Some(step(
            s,
            Input::Choice(choice_input_of(
                "script:\\![open",
                "開く",
                &["file", "notepad.exe"],
            )),
            &config(),
        ));
    });
    let (_, actions) = result.expect("step を呼んだはず");

    match actions.as_slice() {
        [Action::ResolveChoice { .. }, Action::StartTalk(start)] => {
            assert_eq!(
                start.script, "\\![open",
                "台本は第 2 引数の script: の後ろだけ"
            );
        }
        _ => panic!(
            "解決 → 再生開始のちょうど 2 つのはず（数 {}）",
            actions.len()
        ),
    }
    let unused = logged_once(&ev, Level::WARN, "choice_script_unused_args");
    assert_eq!(unused.fields.get("count").map(String::as_str), Some("2"));
}

/// 要件 7.3・1.6: 大文字の `Script:x` は `script:` ではなく、通常の選択肢のイベントへ進む。
#[test]
fn capitalized_script_prefix_goes_to_choice_select_ex() {
    let s = steady_with_ledger(TalkId(3), 6, &["Script:x"], ChoicePhase::Waiting);
    let mut result = None;
    let ev = capture(|| {
        result = Some(step(
            s,
            Input::Choice(choice_input_of("Script:x", "境目", &[])),
            &config(),
        ));
    });
    let (_, actions) = result.expect("step を呼んだはず");

    assert_eq!(actions.len(), 1, "依頼 1 つだけ（解決も再生開始も無い）");
    let (id, _) = expect_get_call(&actions[0]);
    assert_eq!(id, "OnChoiceSelectEx");
    assert_not_logged(&ev, "choice_script_started");
    assert_not_logged(&ev, "choice_script_empty");
    assert_not_logged(&ev, "choice_script_unused_args");
}

/// 捕捉列の中で `event` が最初に現れる位置（無ければ panic）。
fn position_of(events: &[CapturedEvent], event_name: &str) -> usize {
    events
        .iter()
        .position(|e| e.event.as_deref() == Some(event_name))
        .unwrap_or_else(|| panic!("記録 {event_name} が無い"))
}
