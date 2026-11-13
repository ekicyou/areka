//! 同じフォルダ名のゴーストが別の根から見えても、今のゴーストとして扱う（areka-P0-baseware-root-list・要件 8.4・8.7）。

use areka_ghost::same_folder_name;

use crate::boot_resolve::{GhostDecision, running_name};

use super::ghost_switch::SwitchTarget;

/// 切替先が今のゴーストと同じフォルダ名（`same_folder_name`）で、場所が違えば、切替先を
/// 今のゴーストへ差し替える（info `ghost_switch_same_folder_kept`）。それ以外は手を触れない。
///
/// 名前とさくら側の名前も今のゴーストの場所の descript から読み直す（場所は今のゴースト・
/// 名前は別の根のゴースト、という食い違いを `OnGhostChanging` の参照に渡さない）。差し替えの後は
/// 今の「自分自身への切替」と同じ振る舞いになる。
pub(crate) fn pin_to_current(target: SwitchTarget, current: &GhostDecision) -> SwitchTarget {
    let Some(folder) = current.folder.as_deref() else {
        return target;
    };
    if !same_folder_name(&target.folder, folder) || target.dir == current.dir {
        return target;
    }
    tracing::info!(
        event = "ghost_switch_same_folder_kept",
        folder = %target.folder,
        seen = %target.dir.display(),
        current = %current.dir.display(),
        "切替先が今のゴーストと同じフォルダ名で別の根から見えている——今のゴーストを選んだものとして扱う"
    );
    SwitchTarget {
        dir: current.dir.clone(),
        folder: folder.to_owned(),
        name: running_name(current),
        sakura_name: areka_ghost::catalog::sakura_name(&current.dir),
    }
}

#[cfg(test)]
#[path = "same_folder_pin_tests.rs"]
mod tests;
