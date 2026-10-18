//! script 種別のログが `get_log` で読めることの決定論テスト（design Testing Strategy 要件 7.3）。
//!
//! kanade の公開の出し口 [`areka_kanade::script_log::emit`] を 3 つの表示の語で呼び、捕まえた出来事を
//! ログの履歴の下書き（[`crate::log_history::draft`]）に通して、種別・名・表示の語・本文が取れることを見る。

use areka_kanade::script_log::{PLAIN_LABEL, SSTP_SEND_LABEL, TARGET, emit, event_label};
use log_capture_kit::{CapturedEvent, capture};

use crate::log_history::{Draft, FieldText, Kind, TARGET_SCRIPT, draft};

/// 捕捉した出来事を欄の 2 つの形へ写して下書きに通す。
fn draft_of(ev: &CapturedEvent) -> Option<Draft> {
    draft(
        ev.level,
        &ev.target,
        ev.fields.iter().map(|(name, v)| FieldText {
            name,
            debug: &v.debug,
            raw: v.str_raw.as_deref(),
        }),
    )
}

#[test]
fn kanade_target_is_the_log_history_script_target() {
    assert_eq!(TARGET, TARGET_SCRIPT);
}

#[test]
fn kanade_emit_with_three_labels_drafts_as_script_records() {
    let ghost = "Emily/Phase4.5";
    let on_boot = event_label("OnBoot");
    let cases = [
        (SSTP_SEND_LABEL, r"\0\s[0]hello\e"),
        (on_boot.as_str(), r"\0おはよう\e"),
        (PLAIN_LABEL, r"\1選んだ\e"),
    ];
    let ((), events) = capture(|| {
        for (label, script) in cases {
            emit(ghost, label, script);
        }
    });
    let drafts: Vec<Draft> = events
        .iter()
        .map(|ev| draft_of(ev).unwrap_or_else(|| panic!("種別に当たらない: {ev:?}")))
        .collect();
    assert_eq!(drafts.len(), cases.len(), "捕捉した出来事: {events:?}");
    assert_eq!(
        cases.map(|(l, _)| l),
        ["SSTP(Local,Auth)", "Ghost:OnBoot", "Ghost"]
    );
    for (d, (label, script)) in drafts.iter().zip(cases) {
        assert_eq!(
            (d.kind, d.name.as_str(), d.label.as_str(), d.body.as_str()),
            (Kind::Script, ghost, label, script)
        );
    }
}
