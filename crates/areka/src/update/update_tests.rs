//! 依頼の型の綴り（Reference に載る語）の兄弟テスト（areka-P0-network-update）。

use super::{TargetKind, UpdateReason};

#[test]
fn target_kind_spells_ghost_shell_balloon() {
    let spelled: Vec<&str> = [TargetKind::Ghost, TargetKind::Shell, TargetKind::Balloon]
        .into_iter()
        .map(TargetKind::as_ref_str)
        .collect();
    assert_eq!(spelled, ["ghost", "shell", "balloon"]);
}

#[test]
fn update_reason_spells_manual_script() {
    let spelled: Vec<&str> = [UpdateReason::Manual, UpdateReason::Script]
        .into_iter()
        .map(UpdateReason::as_ref_str)
        .collect();
    assert_eq!(spelled, ["manual", "script"]);
}
