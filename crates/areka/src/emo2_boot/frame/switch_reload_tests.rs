//! 差し替えの相の読み直しの分かれ目の決定論テスト（areka-P0-mcp-reload task 7.4・要件 5.2・5.3・
//! 5.4・6.2・6.3・6.4・8.4・design「SkinSwitch の口」・Flow 4）。
//!
//! 土台は親（`switch_finish_tests.rs`）の頼んだ段の組み立てをそのまま使い、読み直しの窓口の印
//! （`ReloadInFlight` の `Skins` の段・列は空・予約は相の鍵）を直に置く。印が無いときの今までどおりの
//! 後始末は親のテストが固定している。判定は集めてから 1 回。

use std::collections::VecDeque;

use areka_kanade::TalkGap;

use super::*;
use crate::emo2_boot::reload::{ReloadInFlight, ReloadOrigin, ReloadStage};
use crate::emo2_boot::reload_args::ReloadTarget;

/// 窓口の印を置く（`reserved` は窓口自身が `accept_skin` で立てた予約の鍵）。
fn plant_reload(world: &mut World, kind: SkinKind, reserved: PathBuf) {
    world.insert_non_send(ReloadInFlight {
        target: match kind {
            SkinKind::Shell => ReloadTarget::Shell,
            SkinKind::Balloon => ReloadTarget::Balloon,
        },
        origin: ReloadOrigin::Script,
        ghost: "A".to_owned(),
        stage: ReloadStage::Skins {
            queue: VecDeque::new(),
            applied: false,
            reserved: Some((kind, reserved)),
        },
    });
}

fn target_dir(kind: SkinKind) -> PathBuf {
    match kind {
        SkinKind::Shell => shell_target().dir,
        SkinKind::Balloon => balloon_target().dir,
    }
}

/// `reload_done`／`reload_failed` の (event, reason) の並び。
fn ends(events: &[CapturedEvent]) -> Vec<(String, Option<String>)> {
    events
        .iter()
        .filter_map(|e| {
            let event = e.field_str("event")?;
            matches!(event, "reload_done" | "reload_failed")
                .then(|| (event.to_owned(), e.field_str("reason").map(str::to_owned)))
        })
        .collect()
}

fn done() -> Vec<(String, Option<String>)> {
    vec![("reload_done".to_owned(), None)]
}

fn failed(reason: &str) -> Vec<(String, Option<String>)> {
    vec![("reload_failed".to_owned(), Some(reason.to_owned()))]
}

/// 読み直しの差し替えが済んだら、記憶（`LastShell`・`LastBalloon`）も知らせ（`OnShellChanged`・
/// `OnBalloonChange`）もバルーンの決め方も替えず、相の終わりで `reload_done` を 1 件残して印を下ろす
/// （要件 5.2・5.3・6.2・6.3・8.4）。差し替えの相の完了の記録はそのまま残る。
#[test]
fn reload_mark_skips_memory_and_events_and_ends_done() {
    let mut got = Vec::new();
    let mut want = Vec::new();
    for kind in [SkinKind::Shell, SkinKind::Balloon] {
        let mut rig = super::rig(kind, true);
        plant_reload(&mut rig.world, kind, target_dir(kind));
        answer(&mut rig, 0, Ok(()));
        answer(&mut rig, 1, Ok(()));
        let events = frame(&mut rig);
        let route = rig
            .world
            .get_resource::<BootContext>()
            .map(|c| c.current.balloon.route);
        let seen = (
            ends(&events),
            count(&events, "skin_switch_done", Level::INFO),
            stage(&rig.world),
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
            raised(&rig),
            route,
        );
        got.push((kind, seen, memory(rig)));
        want.push((
            kind,
            (done(), 1, None, true, Vec::new(), Some(BalloonRoute::Argv)),
            old_memory(),
        ));
    }
    assert_eq!(
        got, want,
        "(種別, (窓口の終わり, 相の完了, 相の印, 窓口の印が消えた, kanade への便り, 決め方), 記憶)"
    );
}

/// 差し替えが済まずに相が終わったら（頼んだ段の失敗・待ちの段の中止）、元のまま記憶も知らせも 0 で、
/// 窓口は `reload_failed(skin_failed)` を 1 度だけ残して印を下ろす（要件 5.4・6.4）。
#[test]
fn phase_end_without_swap_fails_the_reload_once() {
    let mut got = Vec::new();
    let mut want = Vec::new();
    for kind in [SkinKind::Shell, SkinKind::Balloon] {
        // 頼んだ段で置き換えが失敗した。
        let mut rig = super::rig(kind, true);
        plant_reload(&mut rig.world, kind, target_dir(kind));
        answer(&mut rig, 0, Ok(()));
        answer(&mut rig, 1, attach_failure());
        let events = frame(&mut rig);
        let seen = (ends(&events), raised(&rig));
        got.push((kind, "attach", seen, memory(rig)));
        want.push((
            kind,
            "attach",
            (failed("skin_failed"), Vec::new()),
            old_memory(),
        ));

        // 待ちの段で利用者が台詞を中断した。
        let mut rig = super::rig(kind, true);
        plant_reload(&mut rig.world, kind, target_dir(kind));
        let (gap_tx, gap) = reply_channel::<TalkGap>();
        let (_build_tx, build) = mpsc::channel();
        gap_tx.send(TalkGap::CancelledByUser).unwrap();
        rig.world.insert_non_send(SkinSwitchInFlight {
            kind,
            target: match kind {
                SkinKind::Shell => shell_target(),
                SkinKind::Balloon => balloon_target(),
            },
            stage: SkinSwitchStage::Waiting {
                gap: Some(gap),
                gap_result: None,
                build,
                built: None,
            },
        });
        let mut events = frame(&mut rig);
        // 印が下りた後の次のフレームは何も告げない（1 度だけ）。
        events.extend(frame(&mut rig));
        let seen = (ends(&events), raised(&rig));
        got.push((kind, "cancel", seen, memory(rig)));
        want.push((
            kind,
            "cancel",
            (failed("skin_failed"), Vec::new()),
            old_memory(),
        ));
    }
    assert_eq!(
        got, want,
        "(種別, 場面, (窓口の終わり, kanade への便り), 記憶)"
    );
}

/// 窓口の印があっても予約の鍵が違う（他人の切替）なら今までどおり記憶と知らせを出し、窓口は
/// `reload_failed(overtaken)` で印を下ろす。
#[test]
fn foreign_phase_keeps_the_normal_finish_and_overtakes_the_reload() {
    let mut rig = rig(SkinKind::Shell, true);
    plant_reload(
        &mut rig.world,
        SkinKind::Shell,
        PathBuf::from("skins/ghost/A/shell/other"),
    );
    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    let events = frame(&mut rig);
    let seen = (
        ends(&events),
        raised(&rig).into_iter().map(|r| r.0).collect::<Vec<_>>(),
    );
    let memory = memory(rig);
    assert_eq!(
        (
            seen,
            memory
                .iter()
                .any(|(k, v)| *k == PersistKey::LastShell && v == "summer")
        ),
        (
            (failed("overtaken"), vec!["OnShellChanged".to_owned()]),
            true
        ),
        "((窓口の終わり, kanade への便り), LastShell が新しいシェル)"
    );
}

// 常に手前の状態と印の寿命（シェルとバルーンの切り替え・stay-on-top-directives タスク 5.3）。
#[path = "switch_stay_on_top_tests.rs"]
mod stay_on_top_tests;
