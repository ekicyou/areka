//! 起動の依頼の純粋な判断のテスト（design「Testing Strategy / 単体」・要件 3.1・3.2・4.3・4.4・5.1〜5.4・9.3・9.5・10.1）。

use std::path::PathBuf;

use areka_kanade::ChangeOrigin;

use super::*;
use crate::emo2_boot::ghost_switch::{GhostSpec, SwitchRequest};
use crate::install::judge::{FailureWord, InstalledItem};

fn item(kind: &'static str, place: &str) -> InstalledItem {
    InstalledItem {
        kind,
        name: format!("{kind} name"),
        place: place.to_owned(),
    }
}

fn installed(kind: &'static str, place: &str) -> ArchiveEnd {
    ArchiveEnd::Installed(vec![item(kind, place)])
}

fn ghost(folder: &str) -> ArchiveEnd {
    installed("ghost", &format!(r"C:\areka\ghost\{folder}"))
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

/// 枝を 1 つずつ外していく既定の事実（そのままなら `Request`）。
fn facts<'a>(archives: usize, ends: &'a [ArchiveEnd]) -> CallGhostFacts<'a> {
    CallGhostFacts {
        archives,
        ends,
        closing: false,
        switching: false,
        switches_at_submit: 3,
        switches_now: 3,
        ghost_now: Some("emo2"),
    }
}

fn skip(reason: CallGhostSkip) -> CallGhost {
    CallGhost::Skip(reason)
}

fn request(folder: &str) -> CallGhost {
    CallGhost::Request {
        folder: folder.to_owned(),
    }
}

// ---- sources_to_delete ----

#[test]
fn only_installed_of_the_five_ends_is_deleted() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar", r"C:\c.nar", r"C:\d.nar"]);
    let ends = [
        ghost("a"),
        ArchiveEnd::Refused,
        ArchiveEnd::Declined,
        ArchiveEnd::Failed(FailureWord::Extraction),
    ];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

#[test]
fn abandoned_and_unhandled_rest_are_not_deleted() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar", r"C:\c.nar"]);
    let ends = [
        installed("shell", r"C:\areka\ghost\x\shell\s"),
        ArchiveEnd::Abandoned,
    ];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

#[test]
fn nothing_installed_deletes_nothing() {
    let archives = paths(&[r"C:\a.nar", r"C:\b.nar"]);
    let ends = [ArchiveEnd::Refused, ArchiveEnd::Failed(FailureWord::FileIo)];
    assert!(sources_to_delete(&archives, &ends).is_empty());
}

#[test]
fn same_path_in_other_case_is_deleted_once_with_first_spelling() {
    // 片方だけが入った（1 回目は失敗・2 回目が入った）でも、最初の綴りで 1 回だけ。
    let archives = paths(&[r"C:\Dir\A.nar", r"C:\b.nar", r"c:\dir\a.NAR"]);
    let ends = [
        ArchiveEnd::Failed(FailureWord::Unsupported),
        ghost("b"),
        ghost("a"),
    ];
    assert_eq!(
        sources_to_delete(&archives, &ends),
        paths(&[r"C:\Dir\A.nar", r"C:\b.nar"])
    );
}

#[test]
fn same_path_installed_twice_is_deleted_once() {
    let archives = paths(&[r"C:\a.nar", r"C:\A.NAR"]);
    let ends = [ghost("a"), ghost("a")];
    assert_eq!(sources_to_delete(&archives, &ends), paths(&[r"C:\a.nar"]));
}

// ---- decide_call_ghost ----

#[test]
fn all_clear_requests_the_installed_ghost_folder() {
    let ends = [ghost("yame")];
    assert_eq!(decide_call_ghost(&facts(1, &ends)), request("yame"));
}

#[test]
fn abandoned_end_skips() {
    let ends = [ghost("a"), ArchiveEnd::Abandoned];
    assert_eq!(
        decide_call_ghost(&facts(3, &ends)),
        skip(CallGhostSkip::Abandoned)
    );
}

#[test]
fn short_ends_without_abandoned_skips_as_abandoned() {
    let ends = [ghost("a")];
    assert_eq!(
        decide_call_ghost(&facts(2, &ends)),
        skip(CallGhostSkip::Abandoned)
    );
}

#[test]
fn closing_skips() {
    let ends = [ghost("a")];
    let f = CallGhostFacts {
        closing: true,
        ..facts(1, &ends)
    };
    assert_eq!(decide_call_ghost(&f), skip(CallGhostSkip::Closing));
}

#[test]
fn no_ghost_installed_skips_for_shell_balloon_supplement_and_all_failed() {
    let cases: Vec<Vec<ArchiveEnd>> = vec![
        vec![installed("shell", r"C:\areka\ghost\emo2\shell\s")],
        vec![installed("balloon", r"C:\areka\balloon\b")],
        vec![installed("supplement", r"C:\areka\ghost\emo2")],
        vec![
            ArchiveEnd::Refused,
            ArchiveEnd::Declined,
            ArchiveEnd::Failed(FailureWord::InvalidType),
        ],
    ];
    for ends in &cases {
        assert_eq!(
            decide_call_ghost(&facts(ends.len(), ends)),
            skip(CallGhostSkip::NoGhostInstalled),
            "{ends:?}"
        );
    }
}

#[test]
fn ghost_with_bundled_balloon_counts_by_its_first_item() {
    // 同梱バルーンは本体の後ろに並ぶ（本体が先）。
    let ends = [ArchiveEnd::Installed(vec![
        item("ghost", r"C:\areka\ghost\yame"),
        item("balloon", r"C:\areka\balloon\yb"),
    ])];
    assert_eq!(decide_call_ghost(&facts(1, &ends)), request("yame"));
}

#[test]
fn switching_skips() {
    let ends = [ghost("a")];
    let f = CallGhostFacts {
        switching: true,
        ..facts(1, &ends)
    };
    assert_eq!(decide_call_ghost(&f), skip(CallGhostSkip::Switching));
}

#[test]
fn accept_count_grew_skips_as_switched_meanwhile() {
    let ends = [ghost("a")];
    // A -> B は 1 増え、A -> B -> A と戻っても 2 増えるので見分けられる。
    for grown in [4, 5] {
        let f = CallGhostFacts {
            switches_now: grown,
            ..facts(1, &ends)
        };
        assert_eq!(
            decide_call_ghost(&f),
            skip(CallGhostSkip::SwitchedMeanwhile)
        );
    }
}

#[test]
fn unchanged_accept_count_goes_on_to_next_branch() {
    let ends = [ghost("emo2")];
    // 数が同じなら 5 を抜け、6（起動中）へ進む。
    assert_eq!(
        decide_call_ghost(&facts(1, &ends)),
        skip(CallGhostSkip::AlreadyRunning)
    );
}

#[test]
fn running_ghost_skips_regardless_of_case() {
    let ends = [ghost("Emo2")];
    assert_eq!(
        decide_call_ghost(&facts(1, &ends)),
        skip(CallGhostSkip::AlreadyRunning)
    );
}

#[test]
fn no_current_ghost_name_requests() {
    let ends = [ghost("emo2")];
    let f = CallGhostFacts {
        ghost_now: None,
        ..facts(1, &ends)
    };
    assert_eq!(decide_call_ghost(&f), request("emo2"));
}

#[test]
fn target_is_last_installed_ghost() {
    let ends = [
        ghost("a"),
        ArchiveEnd::Failed(FailureWord::Extraction),
        ghost("b"),
        installed("shell", r"C:\areka\ghost\b\shell\s2"),
    ];
    assert_eq!(decide_call_ghost(&facts(4, &ends)), request("b"));
}

// ---- callghost_request ----

#[test]
fn request_matches_the_menu_ghost_frame() {
    assert_eq!(
        callghost_request("yame".to_owned()),
        SwitchRequest {
            ghost: GhostSpec::Folder("yame".to_owned()),
            raise_event: true,
            origin: ChangeOrigin::Manual,
            boot_event: None,
        }
    );
}
