//! 切替の 2 イベント・汎用の組み立て・`OnBoot` の Ref6/7・許可表の照合・置き換えの述語のテスト
//! （areka-P0-ghost-shell-balloon-switch 要件 2.1・4.1・6.2・7.2・7.3・11.5・11.7・11.10）。
//!
//! どの組み立ても番号を詰めない（使わない番号は空文字で埋め、位置を保つ）ことを列ごと突き合わせる。

use super::*;
use crate::change::{
    BootOrigin, ChangeOrigin, ChangeRequest, ChangeTarget, ChangedFrom, ShioriMethod,
};
use crate::status::ExecutionStatus;

fn target() -> ChangeTarget {
    ChangeTarget {
        sakura_name: "かなで".to_string(),
        name: "Kanade Ghost".to_string(),
        dir: r"C:\areka\ghost\kanade".to_string(),
    }
}

fn get_parts(call: ShioriCall) -> (EventId, Vec<String>) {
    match call {
        ShioriCall::Get { id, references, .. } => (id, references),
        ShioriCall::Notify { .. } => panic!("GET のはずが NOTIFY"),
    }
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn allowed_static_returns_the_table_spelling_for_the_two_change_events() {
    // 13 語に、インストール系の 8 語を足して 21 語（areka-P0-ghost-install 要件 2.12・11.9）、
    // 投げ込みの 2 語を足して 23 語（areka-P0-file-drop 要件 7.3・8.8）、
    // ネットワーク更新の 19 語を足して 42 語（areka-P0-network-update 要件 2.15・9.13）。
    assert_eq!(ALLOWED_EVENT_IDS.len(), 42);
    for id in [
        "OnGhostChanging",
        "OnGhostChanged",
        "OnBoot",
        "OnMouseMove",
        "OnInstallBegin",
        "OnInstallComplete",
        "OnInstallCompleteEx",
        "OnInstallCompleteAll",
        "OnInstallFailure",
        "OnInstallRefuse",
        "OnGhostTermsAccept",
        "OnGhostTermsDecline",
        "OnFileDrop2",
        "OnDirectoryDrop",
        "OnUpdateProcessExec",
        "OnUpdateBegin",
        "OnUpdateReady",
        "OnUpdate.OnDownloadBegin",
        "OnUpdate.OnMD5CompareBegin",
        "OnUpdate.OnMD5CompareComplete",
        "OnUpdate.OnMD5CompareFailure",
        "OnUpdateComplete",
        "OnUpdateFailure",
        "OnUpdateOtherBegin",
        "OnUpdateOtherReady",
        "OnUpdateOther.OnDownloadBegin",
        "OnUpdateOther.OnMD5CompareBegin",
        "OnUpdateOther.OnMD5CompareComplete",
        "OnUpdateOther.OnMD5CompareFailure",
        "OnUpdateOtherComplete",
        "OnUpdateOtherFailure",
        "OnUpdateResult",
        "OnUpdateResultEx",
    ] {
        assert_eq!(allowed_static(id), Some(id));
        assert!(is_allowed_event_id(id));
    }
    for id in [
        "OnTalk",
        "OnHour",
        "onghostchanging",
        "OnGhostChangingX",
        "",
        // 送り先を他のゴーストへ移す場面は作らない（要件 2.11）。
        "OnInstallReroute",
        // 更新オプション `checkonly` の系・エクスプローラ・作る側は送らない（要件 2.15）。
        "OnUpdateCheckComplete",
        "OnUpdateCheckFailure",
        "OnUpdateCheckResult",
        "OnUpdateCheckResultEx",
        "OnUpdateResultExplorer",
        "OnUpdatedataCreating",
        "OnUpdatedataCreated",
    ] {
        assert_eq!(allowed_static(id), None, "{id} は許可表に無い");
    }
}

#[test]
fn on_ghost_changing_is_get_with_target_and_origin_in_ref0_to_ref3() {
    for (origin, word) in [
        (ChangeOrigin::Manual, "manual"),
        (ChangeOrigin::Automatic, "automatic"),
    ] {
        let req = ChangeRequest {
            target: target(),
            origin,
            raise_event: true,
        };
        let (id, refs) = get_parts(on_ghost_changing(&req, &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, EventId::Static("OnGhostChanging"));
        assert_eq!(
            refs,
            s(&["かなで", word, "Kanade Ghost", r"C:\areka\ghost\kanade"])
        );
    }
}

#[test]
fn on_ghost_changing_keeps_empty_sakura_name_in_place() {
    let req = ChangeRequest {
        target: ChangeTarget {
            sakura_name: String::new(),
            ..target()
        },
        origin: ChangeOrigin::Manual,
        raise_event: true,
    };
    let (_, refs) = get_parts(on_ghost_changing(&req, &ExecutionSnapshot::INACTIVE));
    assert_eq!(
        refs,
        s(&["", "manual", "Kanade Ghost", r"C:\areka\ghost\kanade"])
    );
}

#[test]
fn on_ghost_changed_is_get_with_ref4_to_ref6_empty_and_shell_folder_in_ref7() {
    let from = ChangedFrom {
        sakura_name: "えも".to_string(),
        script: r"\0じゃあね\e".to_string(),
        name: "emo2".to_string(),
        dir: r"C:\areka\ghost\emo2".to_string(),
    };
    let (id, refs) = get_parts(on_ghost_changed(
        &from,
        "master",
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(id, EventId::Static("OnGhostChanged"));
    assert_eq!(
        refs,
        s(&[
            "えも",
            r"\0じゃあね\e",
            "emo2",
            r"C:\areka\ghost\emo2",
            "",
            "",
            "",
            "master",
        ])
    );
}

#[test]
fn on_ghost_changed_keeps_empty_script_in_ref1() {
    let from = ChangedFrom {
        sakura_name: String::new(),
        script: String::new(),
        name: "emo2".to_string(),
        dir: r"C:\areka\ghost\emo2".to_string(),
    };
    let (_, refs) = get_parts(on_ghost_changed(&from, "red", &ExecutionSnapshot::INACTIVE));
    assert_eq!(
        refs,
        s(&["", "", "emo2", r"C:\areka\ghost\emo2", "", "", "", "red"])
    );
}

#[test]
fn on_boot_after_halt_carries_halt_in_ref6_and_fallen_ghost_in_ref7() {
    let mut config = KanadeConfig::new("master", "1.0.0");
    config.boot_origin = BootOrigin::Halted {
        ghost_name: "Kanade Ghost".to_string(),
    };
    let (id, refs) = get_parts(on_boot(&config, &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, EventId::Static("OnBoot"));
    assert_eq!(
        refs,
        s(&["master", "", "", "", "", "", "halt", "Kanade Ghost"])
    );
}

#[test]
fn on_boot_without_halt_is_ref0_only() {
    let plain = KanadeConfig::new("master", "1.0.0");
    let mut changed = KanadeConfig::new("master", "1.0.0");
    changed.boot_origin = BootOrigin::ChangedFrom(ChangedFrom {
        sakura_name: "えも".to_string(),
        script: String::new(),
        name: "emo2".to_string(),
        dir: r"C:\areka\ghost\emo2".to_string(),
    });
    for config in [plain, changed] {
        let (_, refs) = get_parts(on_boot(&config, &ExecutionSnapshot::INACTIVE));
        assert_eq!(refs, s(&["master"]));
    }
}

#[test]
fn raise_passes_references_verbatim_as_get_or_notify() {
    let snapshot = ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    };
    let refs = s(&["a", "", "c", ""]);

    match raise("OnGhostChanged", refs.clone(), ShioriMethod::Get, &snapshot) {
        ShioriCall::Get {
            id,
            references,
            status,
        } => {
            assert_eq!(id, EventId::Static("OnGhostChanged"));
            assert_eq!(references, refs);
            assert_eq!(status, ExecutionStatus::derive(&snapshot));
        }
        ShioriCall::Notify { .. } => panic!("GET のはずが NOTIFY"),
    }
    match raise("OnBoot", refs.clone(), ShioriMethod::Notify, &snapshot) {
        ShioriCall::Notify {
            id,
            references,
            status,
        } => {
            assert_eq!(id, EventId::Static("OnBoot"));
            assert_eq!(references, refs);
            assert_eq!(status, ExecutionStatus::derive(&snapshot));
        }
        ShioriCall::Get { .. } => panic!("NOTIFY のはずが GET"),
    }
}

#[test]
fn value_replaces_active_talk_except_second_change() {
    for origin in ["OnMouseMove", "OnMouseDoubleClick", "OnGhostChanging"] {
        assert!(value_replaces_active_talk(origin), "{origin} は置き換える");
    }
    assert!(!value_replaces_active_talk("OnSecondChange"));
}
