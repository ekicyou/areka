//! 運行表の型の変種の網羅（`schedule_tests.rs` から移した）。
//!
//! wildcard を持たない網羅 match で、変種の削除・改名・形の変更をコンパイルで止める。
//! 変種が増えるたびに腕を足すので、行数の上限に近い `schedule_tests.rs` から切り出してある。

use super::tests::{choice_input, mouse_move};
use super::*;
use crate::change::KanadeNotice;

/// Req4.4: 既存 `Action` 5 variant が無改変で、選択系 2 variant が additive に増えたこと。
/// ghost-shell-balloon-switch で運行の通知（`Notice`）の腕を 1 つ足した。
///
/// wildcard なしの網羅 match ゆえ、既存 5 variant のいずれかが消える／改名される／
/// 形が変わるとこのテストはコンパイルできない。
#[test]
fn action_variants_are_existing_five_plus_choice_two() {
    fn tag(action: &Action) -> &'static str {
        match action {
            Action::ShioriRequest(_) => "ShioriRequest",
            Action::ShioriUnload => "ShioriUnload",
            Action::StartTalk(_) => "StartTalk",
            Action::ResourceOutcome { .. } => "ResourceOutcome",
            Action::StopSelf => "StopSelf",
            Action::ResolveChoice { .. } => "ResolveChoice",
            Action::CancelChoice { .. } => "CancelChoice",
            Action::Notice(_) => "Notice",
        }
    }
    assert_eq!(tag(&Action::ShioriUnload), "ShioriUnload");
    assert_eq!(tag(&Action::StopSelf), "StopSelf");
    assert_eq!(
        tag(&Action::ResolveChoice {
            talk_id: TalkId(5),
            id: "OnMenu".to_string(),
        }),
        "ResolveChoice"
    );
    assert_eq!(
        tag(&Action::CancelChoice { talk_id: TalkId(5) }),
        "CancelChoice"
    );
    assert_eq!(tag(&Action::Notice(KanadeNotice::Steady)), "Notice");
}

/// Req4.4: 既存 `Phase` の 11 variant が無改変であること（切替の 4 相を足して 15）。
///
/// 本 match は wildcard を持たないため、variant の削除・改名・形（フィールド構成）の
/// 変更はコンパイルを壊す。DD-3 が要求する「Phase を一切触らない」を構造で固定する
/// （`State.choice` は Phase の外＝`pending_close` と同型に置かれる）。
/// ghost-shell-balloon-switch で切替の 4 相の腕を足した（`schedule_tests.rs` から移した）。
#[test]
fn existing_phase_variants_are_unchanged() {
    fn tag(phase: &Phase) -> &'static str {
        match phase {
            Phase::Idle => "Idle",
            Phase::BootInit => "BootInit",
            Phase::BootPrefetch => "BootPrefetch",
            Phase::BootType => "BootType",
            Phase::BootMain => "BootMain",
            Phase::BootVersion { .. } => "BootVersion",
            Phase::Steady { .. } => "Steady",
            Phase::ClosePending { .. } => "ClosePending",
            Phase::CloseTalkWait { .. } => "CloseTalkWait",
            Phase::ChangePending => "ChangePending",
            Phase::ChangeTalkWait { .. } => "ChangeTalkWait",
            Phase::ChangeClosePending => "ChangeClosePending",
            Phase::ChangeCloseTalkWait { .. } => "ChangeCloseTalkWait",
            Phase::Unloading { .. } => "Unloading",
            Phase::Stopped => "Stopped",
        }
    }
    assert_eq!(tag(&Phase::Idle), "Idle");
    assert_eq!(tag(&Phase::Steady { talk: None }), "Steady");
    assert_eq!(tag(&Phase::Stopped), "Stopped");
    assert_eq!(tag(&Phase::ChangePending), "ChangePending");
    assert_eq!(tag(&Phase::ChangeClosePending), "ChangeClosePending");
}

/// テスト用の選択待ち通知入力（内容は判別に効かない＝写像の存在のみを見る）。
fn choice_waiting_input() -> Input {
    Input::ChoiceWaiting {
        talk_id: TalkId(5),
        choice_ids: vec!["OnMenu".to_string()],
        display_end: MonotonicMs(2_000),
        timeout_directive_secs: None,
    }
}

/// Req4.4: 既存 `Input` 8 variant が無改変で、選択系 2 variant が additive に増えたこと。
/// ghost-shell-balloon-switch で汎用の通知の入口（`RaiseEvent`）の腕を足した（`schedule_tests.rs` から移した）。
#[test]
fn input_variants_are_existing_eight_plus_choice_two() {
    fn tag(input: &Input) -> &'static str {
        match input {
            Input::Boot => "Boot",
            Input::Tick { .. } => "Tick",
            Input::TalkDone(_) => "TalkDone",
            Input::CloseRequest { .. } => "CloseRequest",
            Input::ForceQuit { .. } => "ForceQuit",
            Input::ShioriDown { .. } => "ShioriDown",
            Input::Mouse(_) => "Mouse",
            Input::ShioriReply { .. } => "ShioriReply",
            Input::Choice(_) => "Choice",
            Input::ChoiceWaiting { .. } => "ChoiceWaiting",
            Input::UserBreak { .. } => "UserBreak",
            Input::ChangeGhost(_) => "ChangeGhost",
            Input::RaiseEvent { .. } => "RaiseEvent",
        }
    }
    assert_eq!(tag(&Input::Boot), "Boot");
    assert_eq!(tag(&Input::Mouse(mouse_move())), "Mouse");
    assert_eq!(tag(&Input::Choice(choice_input())), "Choice");
    assert_eq!(tag(&choice_waiting_input()), "ChoiceWaiting");
}
