//! 運行表の型の変種の網羅（`schedule_tests.rs` から移した）。
//!
//! wildcard を持たない網羅 match で、変種の削除・改名・形の変更をコンパイルで止める。
//! 変種が増えるたびに腕を足すので、行数の上限に近い `schedule_tests.rs` から切り出してある。

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
