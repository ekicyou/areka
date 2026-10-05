//! `script:` の選択肢を受け付けた後の結末のテスト（最上位の `schedule::step` から通す）。

use super::super::test_support::{choice_input_of, config, steady_with_ledger};
use crate::schedule::log_capture::{assert_not_logged, capture, logged_once};
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
    assert_not_logged(&ev, "choice_script_unused_args");
}
