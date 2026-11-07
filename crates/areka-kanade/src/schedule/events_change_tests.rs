//! 切替の 2 イベント・汎用の組み立て・`OnBoot` の Ref6/7・許可表の照合・置き換えの述語のテスト
//! （areka-P0-ghost-shell-balloon-switch 要件 2.1・4.1・6.2・7.2・7.3・11.5・11.7・11.10）。
//!
//! どの組み立ても番号を詰めない（使わない番号は空文字で埋め、位置を保つ）ことを列ごと突き合わせる。

use super::*;
use crate::change::{
    BootOrigin, ChangeOrigin, ChangeRequest, ChangeTarget, ChangedFrom, ShioriMethod,
};
use crate::schedule::translate_test_support::{change_request, config, strings as s};
use crate::status::ExecutionStatus;

fn get_parts(call: ShioriCall) -> (EventId, Vec<String>) {
    match call {
        ShioriCall::Get { id, references, .. } => (id, references),
        ShioriCall::Notify { .. } => panic!("GET のはずが NOTIFY"),
    }
}

#[test]
fn allowed_static_returns_the_table_spelling_and_none_off_the_table() {
    // 表に載る語の全体は許可表の完全一致のテストが固定する。ここは綴りを返す形だけを 1 語で見る。
    assert_eq!(allowed_static("OnGhostChanging"), Some("OnGhostChanging"));
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
            origin,
            ..change_request()
        };
        let (id, refs) = get_parts(on_ghost_changing(&req, &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, EventId::Static("OnGhostChanging"));
        assert_eq!(
            refs,
            s(&[
                "ポスト",
                word,
                "R_POST_and_KOMAINU",
                r"C:\areka\ghost\r_post"
            ])
        );
    }
}

#[test]
fn on_ghost_changing_keeps_empty_sakura_name_in_place() {
    let req = ChangeRequest {
        target: ChangeTarget {
            sakura_name: String::new(),
            ..change_request().target
        },
        ..change_request()
    };
    let (_, refs) = get_parts(on_ghost_changing(&req, &ExecutionSnapshot::INACTIVE));
    assert_eq!(
        refs,
        s(&["", "manual", "R_POST_and_KOMAINU", r"C:\areka\ghost\r_post"])
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
    let mut config = config();
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
    let plain = config();
    let mut changed = config();
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
fn value_replaces_active_talk_except_second_change_and_scaling() {
    for origin in ["OnMouseMove", "OnMouseDoubleClick", "OnGhostChanging"] {
        assert!(value_replaces_active_talk(origin), "{origin} は置き換える");
    }
    for origin in ["OnSecondChange", "OnShellScaling", "OnBalloonScaling"] {
        assert!(
            !value_replaces_active_talk(origin),
            "{origin} は置き換えない"
        );
    }
}
