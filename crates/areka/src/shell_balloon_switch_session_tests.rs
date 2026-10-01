//! シェル・バルーンの切替とゴーストのセッション（spec: areka-P0-shell-balloon-switch）。
//!
//! 偽の SHIORI を持つゴースト A（`ghost_switch_test_support` の土台）を本番と同じ入口
//! （[`super::boot_ghost`]）で起こし、同じ World の上でセッションの持ち物と降ろし方を判定する。

use std::path::PathBuf;

use super::{GhostSession, GhostSlot};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

/// seriko の送り手の複製は結線ありのセッションだけが持ち（テスト用の組み立ては持たない）、
/// 持ったセッションを降ろしても seriko の join が止まらずに戻る（要件 8.5）。複製を降ろす最初の
/// 段で落とさないと、③ の join が残った送り手を待ち続けて期限（20 秒）で赤になる。
#[test]
fn session_holding_seriko_sink_shuts_down_within_bound() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.boot("A");
    let held = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .map(|session| session.seriko_sink().is_some());
    let shutdown_ok = rig.shutdown();
    let for_test_held = GhostSession::for_test(None, PathBuf::from("for_test"))
        .seriko_sink()
        .is_some();

    assert_eq!(
        (held, shutdown_ok, for_test_held),
        (Some(true), true, false),
        "seriko の送り手の持ち主が違うか、降ろすのが戻らない（結線ありのセッションが持つ・\
         降ろせた・テスト用の組み立ては持たない）"
    );
}
