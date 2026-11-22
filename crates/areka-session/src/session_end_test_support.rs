//! 終了・切替・起動のテストが共有する値の組み立てと記録の読み。

use std::path::Path;

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_kanade::{ShioriFault, ShioriFaultKind};
use areka_runtime::spine::RecordedCall;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistOutcome, PersistScope, ScopeRoots, save_scope};

use crate::ghost_switch::{PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget};

/// 切り替えの予約（B へ・前は A・段は `stage`）。
pub fn reservation(stage: SwitchStage) -> SwitchInFlight {
    SwitchInFlight {
        target: SwitchTarget {
            dir: "ghost/B".into(),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: "ghost/A".into(),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage,
        boot_event: None,
        copy: crate::switch_copy::SwitchCopy::None,
    }
}

/// ゴースト `ghost` の記憶に最後のバルーンを書く（Ghost スコープ・boot が据える場所と同じ）。
pub fn put_memory(ghost: &Path, folder: &str) {
    assert_eq!(
        save_scope(
            PersistScope::Ghost,
            &ScopeRoots {
                ghost: Some(profile_areka_root(&ghost.join("ghost").join("master"))),
                ..ScopeRoots::default()
            },
            &FsPersistIo,
            vec![(PersistKey::LastBalloon, folder.to_owned())],
        ),
        PersistOutcome::Saved,
        "記憶を書けない: {}",
        ghost.display()
    );
}

/// 呼出列のうち `OnClose` の NOTIFY の Reference0（届いた分だけ）。
pub fn on_close_ref0(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .filter_map(|c| match c {
            RecordedCall::Notify { id, references } if id == "OnClose" => {
                Some(references.first().cloned().unwrap_or_default())
            }
            _ => None,
        })
        .collect()
}

/// 種類 `kind` の SHIORI の失敗。
pub fn fault(kind: ShioriFaultKind) -> ShioriFault {
    ShioriFault {
        kind,
        reason: "テストの失敗".to_owned(),
    }
}

/// `event` の記録の水準（届いた順）。
#[cfg(test)]
pub(crate) fn levels_of(
    events: &[log_capture_kit::CapturedEvent],
    event: &str,
) -> Vec<tracing::Level> {
    crate::boot_resolve::boot_resolve_tests::events_named(events, event)
        .into_iter()
        .map(|e| e.level)
        .collect()
}

/// `event` の記録 1 件目の欄 `name`（文字列の欄は生値、それ以外は Debug 表現）。
#[cfg(test)]
pub(crate) fn event_field(
    events: &[log_capture_kit::CapturedEvent],
    event: &str,
    name: &str,
) -> Option<String> {
    crate::boot_resolve::boot_resolve_tests::events_named(events, event)
        .first()
        .and_then(|e| e.field_str(name).or_else(|| e.field(name)))
        .map(str::to_owned)
}
