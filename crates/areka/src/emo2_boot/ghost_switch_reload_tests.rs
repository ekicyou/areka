//! ゴースト切替の起こし方に読み直しの由来を通す兄弟テスト
//! （areka-P0-mcp-reload タスク 7.1・design「GhostSwitch の口」・Flow 2・要件 3.2〜3.6・12.2・12.4）。
//!
//! 切替の土台（[`SwitchRig`]）の上で、読み直しの窓口・切替・kanade・偽の SHIORI を本物の経路で通す。
//! 確かめること: 印（窓口自身の予約）があれば `switch_to` は `Reloaded{Ghost}` で起こし、降ろす側に
//! `OnGhostChanging`・`OnClose`、起こす側に `OnBoot`・`OnGhostChanged`・`OnFirstBoot` が無く、情報の通知
//! （`OnInitialize`・`username`・`basewareversion`）はある／印が無い・他人の予約なら今までどおり `OnGhostChanged`。

use areka_kanade::ChangeOrigin;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{GhostSpec, SwitchInFlight, SwitchRequest, SwitchVerdict, request_ghost_switch};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::reload::{
    ReloadInFlight, ReloadOrigin, ReloadRequest, ReloadStage, ReloadVerdict, request_reload,
};
use crate::emo2_boot::reload_args::{DescriptNotes, ReloadTarget};
use crate::emo2_boot::spine::RecordedCall;

/// 呼び出しを「GET 名」「NOTIFY 名」の列にする。
fn ids(calls: &[RecordedCall]) -> Vec<String> {
    calls
        .iter()
        .map(|call| match call {
            RecordedCall::Get { id, .. } => format!("GET {id}"),
            RecordedCall::Notify { id, .. } => format!("NOTIFY {id}"),
            other => format!("{other:?}"),
        })
        .collect()
}

fn named(events: &[CapturedEvent], name: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .count()
}

/// A の台本（`OnGhostChanged` には台本で応える＝`OnBoot` へ続かない）。
fn a_script() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0A\e")
            .get("OnGhostChanged", Ok(Some(r"\0A\e".to_owned())))
            .get("OnGhostChanging", Ok(None))
    }))
}

/// A を起こして定常に着かせる（起動記録を置く＝`OnFirstBoot` を飛ばす）。
fn running_a() -> SwitchRig {
    let mut rig = SwitchRig::new(vec![("A", a_script())]);
    // 起こし直すときの窓の準備が閉包を投函する先。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// A の 2 回目の起動が済み、切替の予約と読み直しの印が下りた（定常到達）。
fn a_rebooted(rig: &SwitchRig) -> bool {
    rig.exit_requested()
        || (rig.calls("A").len() == 2
            && rig.world.get_non_send::<SwitchInFlight>().is_none()
            && rig.world.get_non_send::<ReloadInFlight>().is_none())
}

/// A 自身への知らせなしの切替（出どころ「自動」）を窓口を通さずに頼む。
fn plain_switch_to_a(rig: &mut SwitchRig) -> SwitchVerdict {
    request_ghost_switch(
        &mut rig.world,
        SwitchRequest {
            ghost: GhostSpec::Folder("A".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event: None,
        },
    )
}

/// ghost の読み直しは自分自身への切替の道を通り、降ろす側に `OnGhostChanging`・`OnClose` を、起こす側に
/// 台詞を求める GET を 1 つも送らず、情報の通知（`OnInitialize`・`username`・`basewareversion`）は送る。
/// 終わりの記録は `reload_done` の 1 件で、印は下りる。
///
/// # 非空虚性
/// `switch_to` が由来を上書きしないと、根が `OnGhostChanged` になって赤。`end_switch` が
/// `on_switch_end` を呼ばないと、印が残って待ちが打ち切られ赤。
#[test]
fn a_ghost_reload_reboots_with_no_boot_notice_but_with_the_information_notices() {
    let mut rig = running_a();

    let (flow, events) = capture(|| {
        let verdict = request_reload(
            &mut rig.world,
            ReloadRequest {
                target: ReloadTarget::Ghost,
                origin: ReloadOrigin::Script,
                notes: DescriptNotes::default(),
            },
        );
        (verdict, rig.pump_talking_until(a_rebooted))
    });
    let calls = rig.calls("A");
    let exited = rig.exit_requested();
    assert!(rig.shutdown());

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "終了しない: {events:?}");
    assert_eq!(calls.len(), 2, "A を 1 回起こし直す: {calls:?}");
    let old = ids(&calls[0]);
    for absent in ["GET OnGhostChanging", "NOTIFY OnClose", "GET OnClose"] {
        assert!(
            old.iter().all(|c| c != absent),
            "降ろす側に {absent}: {old:?}"
        );
    }
    let new = ids(&calls[1]);
    for absent in ["GET OnBoot", "GET OnGhostChanged", "GET OnFirstBoot"] {
        assert!(
            new.iter().all(|c| c != absent),
            "起こす側に {absent}: {new:?}"
        );
    }
    assert!(
        new.iter()
            .all(|c| !c.starts_with("GET ") || c == "GET username"),
        "台詞を求める GET が 0 件: {new:?}"
    );
    for present in [
        "NOTIFY OnInitialize",
        "GET username",
        "NOTIFY basewareversion",
    ] {
        assert!(
            new.iter().any(|c| c == present),
            "起こす側に {present}: {new:?}"
        );
    }
    assert_eq!(named(&events, "reload_done"), 1, "{events:?}");
    assert_eq!(named(&events, "reload_failed"), 0, "{events:?}");
}

/// 印が無い自分自身への切替は今までどおり `OnGhostChanged` で起きる（要件 3.6・12.4）。
#[test]
fn a_switch_without_the_mark_boots_with_on_ghost_changed_as_before() {
    let mut rig = running_a();

    let (flow, events) = capture(|| {
        (
            plain_switch_to_a(&mut rig),
            rig.pump_talking_until(a_rebooted),
        )
    });
    let calls = rig.calls("A");
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert_eq!(calls.len(), 2, "{calls:?}");
    let new = ids(&calls[1]);
    assert!(new.iter().any(|c| c == "GET OnGhostChanged"), "{new:?}");
    assert_eq!(named(&events, "reload_done"), 0, "{events:?}");
}

/// 印があっても進行中の予約が他人のもの（鍵が違う）なら由来は上書きされず `OnGhostChanged` で起き、
/// 印は `reload_failed(overtaken)` を残して下りる（design「述語の契約」）。
#[test]
fn a_foreign_switch_under_a_reload_mark_boots_as_before_and_drops_the_mark_as_overtaken() {
    let mut rig = running_a();
    rig.world.insert_non_send(ReloadInFlight {
        target: ReloadTarget::Ghost,
        origin: ReloadOrigin::Script,
        ghost: "A".to_owned(),
        stage: ReloadStage::Switching {
            reserved: rig.root.dir().join("not-a"),
        },
    });

    let (flow, events) = capture(|| {
        (
            plain_switch_to_a(&mut rig),
            rig.pump_talking_until(a_rebooted),
        )
    });
    let calls = rig.calls("A");
    assert!(rig.shutdown());

    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert_eq!(calls.len(), 2, "{calls:?}");
    let new = ids(&calls[1]);
    assert!(new.iter().any(|c| c == "GET OnGhostChanged"), "{new:?}");
    let reasons: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_failed"))
        .map(|e| e.field_str("reason").map(str::to_owned))
        .collect();
    assert_eq!(reasons, [Some("overtaken".to_owned())], "{events:?}");
}

/// shiori の読み直しに応える A の台本（`OnDestroy` の NOTIFY に応える）。
fn a_shiori_script() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        standard_script(r"\0A\e").notify("OnDestroy", Ok(()))
    }))
}

/// `scripts` で組んだ土台の上で A を起こして定常に着かせる（起動記録を置く）。
fn running_a_with(scripts: Vec<(&str, FakeShiori)>) -> SwitchRig {
    let mut rig = SwitchRig::new(scripts);
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.boot("A");
    assert!(rig.wait_steady(), "A が定常に着く");
    rig
}

/// 窓を閉じた回数（`close_windows_for_restart` が進める）。
fn windows_epoch(rig: &SwitchRig) -> Option<u64> {
    rig.world
        .get_resource::<crate::app_exit::WindowsEpoch>()
        .map(|e| e.0)
}

/// shiori の読み直しを台本の出どころで頼む。
fn reload_shiori(rig: &mut SwitchRig) -> ReloadVerdict {
    request_reload(
        &mut rig.world,
        ReloadRequest {
            target: ReloadTarget::Shiori,
            origin: ReloadOrigin::Script,
            notes: DescriptNotes::default(),
        },
    )
}

/// 単位を降ろした印（`GhostSession::shutdown` の ① だけが残す・seriko の join と組）。
fn ticker_closed(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.message().contains("loop ticker を Close"))
        .count()
}

/// shiori の読み直し（要件 4.1〜4.3・12.5）: 窓は閉じられず（`WindowsEpoch` 不変）、単位は降ろされない
/// （seriko・loop ticker は同じもの）。古い SHIORI は `OnDestroy(reload)` の後に unload され、`OnClose`・
/// `OnGhostChanging` を受けない。新しい SHIORI は `OnInitialize(reload)` で起き、台詞を求める GET は 0。
/// 終わりの記録は `reload_done` の 1 件。
///
/// # 非空虚性
/// `switch_to` が分かれ目を通らないと窓が閉じられ（epoch が進み）、単位の ticker が Close されて赤。
#[test]
fn a_shiori_reload_keeps_the_windows_and_rebuilds_only_the_runtime() {
    let mut rig = running_a_with(vec![("A", a_shiori_script())]);
    let epoch = windows_epoch(&rig);

    let (flow, events) = capture(|| {
        let verdict = reload_shiori(&mut rig);
        (verdict, rig.pump_talking_until(a_rebooted))
    });
    let calls = rig.calls("A");
    let exited = rig.exit_requested();
    let epoch_after = windows_epoch(&rig);
    assert!(
        rig.shutdown(),
        "残った単位を降ろせる（seriko の join が戻る）"
    );

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "終了しない: {events:?}");
    assert_eq!(epoch_after, epoch, "窓は閉じられない: {events:?}");
    assert_eq!(ticker_closed(&events), 0, "単位は降ろされない: {events:?}");
    assert_eq!(calls.len(), 2, "SHIORI を 1 回 load し直す: {calls:?}");
    let old = &calls[0];
    let destroy = old.iter().position(|c| {
        matches!(c, RecordedCall::Notify { id, references } if id == "OnDestroy"
            && references.first().map(String::as_str) == Some("reload"))
    });
    let unload = old.iter().position(|c| matches!(c, RecordedCall::Unload));
    assert!(
        matches!((destroy, unload), (Some(d), Some(u)) if d < u),
        "古い SHIORI は OnDestroy(reload) の後に unload: {old:?}"
    );
    let old = ids(old);
    for absent in ["GET OnGhostChanging", "NOTIFY OnClose", "GET OnClose"] {
        assert!(
            old.iter().all(|c| c != absent),
            "古い側に {absent}: {old:?}"
        );
    }
    let new = &calls[1];
    assert!(
        new.iter().any(|c| matches!(
            c,
            RecordedCall::Notify { id, references } if id == "OnInitialize"
                && references.first().map(String::as_str) == Some("reload")
        )),
        "新しい SHIORI は OnInitialize(Ref0=reload): {new:?}"
    );
    let new = ids(new);
    assert!(
        new.iter()
            .all(|c| !c.starts_with("GET ") || c == "GET username"),
        "台詞を求める GET が 0 件: {new:?}"
    );
    assert_eq!(named(&events, "reload_done"), 1, "{events:?}");
    assert_eq!(named(&events, "reload_failed"), 0, "{events:?}");
}

/// shiori の読み直しは窓の組の帳簿と吸着の出どころを残す（areka-P0-window-link-directives 要件 6.3）:
/// 台本の組の入った帳簿と、台本で向きを変えた印の付いた `AnchorSource` が、読み直しの後も同じ値のまま。
///
/// # 非空虚性
/// 読み直しが `close_windows_for_restart` を通ると帳簿が外れて赤。
#[test]
fn a_shiori_reload_keeps_the_sticky_ledger_and_anchor_source() {
    use crate::placement::follow::AnchorSource;
    use crate::placement::resolver::Anchor;
    use crate::placement::sticky_group_ledger::{
        StickyGroupLedger, apply_set_sticky, seed_sticky_descript,
    };

    let mut rig = running_a_with(vec![("A", a_shiori_script())]);
    seed_sticky_descript(&mut rig.world, None);
    assert!(apply_set_sticky(
        &mut rig.world,
        &["0".to_owned(), "1".to_owned()]
    ));
    let ledger = rig.world.resource::<StickyGroupLedger>().clone();
    let source = AnchorSource {
        descript: Anchor::Bottom,
        by_script: true,
    };
    let window = rig.world.spawn(source).id();

    let (flow, events) = capture(|| {
        let verdict = reload_shiori(&mut rig);
        (verdict, rig.pump_talking_until(a_rebooted))
    });
    let ledger_after = rig.world.get_resource::<StickyGroupLedger>().cloned();
    let source_after = rig.world.get::<AnchorSource>(window).copied();
    assert!(
        rig.shutdown(),
        "残った単位を降ろせる（seriko の join が戻る）"
    );

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert_eq!(
        (ledger_after, source_after),
        (Some(ledger), Some(source)),
        "shiori の読み直しで帳簿か吸着の出どころが変わった: {events:?}"
    );
}

/// 組み直しの失敗（要件 4.4）: 新しい SHIORI を起こせなければ既定のゴーストへ戻り、印は
/// `reload_failed(fell_back_to_default)` を残して下りる。
///
/// A の 2 回目の起動の根を実在しない場所へ向けて、組み直しを同期で失敗させる。
///
/// # 非空虚性
/// `BootFailed` を既定へ戻さないと、既定が起きずに待ちが打ち切られ赤。
#[test]
fn a_failed_shiori_rebuild_falls_back_to_the_default_ghost() {
    use crate::boot_resolve::DEFAULT_GHOST_FOLDER;
    use crate::ghost_session::GhostBootInputsSource;

    let mut rig = running_a_with(vec![
        ("A", a_shiori_script()),
        (
            DEFAULT_GHOST_FOLDER,
            FakeShiori::Scripted(Box::new(|| standard_script(r"\0emo2\e"))),
        ),
    ]);
    let original = rig
        .world
        .remove_non_send::<GhostBootInputsSource>()
        .expect("作り口が据わっている");
    rig.world.insert_non_send(GhostBootInputsSource(Box::new(
        move |cfg: &crate::ConfigInputs, origin| {
            let mut inputs = (original.0)(cfg, origin);
            if cfg.ghost_root.ends_with("A") {
                inputs.wiring.ghost_root = cfg.ghost_root.join("実在しない根");
            }
            inputs
        },
    )));

    let (flow, events) = capture(|| {
        let verdict = reload_shiori(&mut rig);
        let done = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls(DEFAULT_GHOST_FOLDER).len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none()
                    && rig.world.get_non_send::<ReloadInFlight>().is_none())
        });
        (verdict, done)
    });
    let exited = rig.exit_requested();
    assert!(rig.shutdown());

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "既定へ戻って続く: {events:?}");
    let reasons: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_failed"))
        .map(|e| e.field_str("reason").map(str::to_owned))
        .collect();
    assert_eq!(
        reasons,
        [Some("fell_back_to_default".to_owned())],
        "{events:?}"
    );
}

/// 組み直しの失敗・切替先が既定（要件 4.4）: 既定のゴーストの新しい SHIORI を起こせなければ、既定へ
/// 戻す試みをせずに致命で終わる（`ghost_switch_fatal` の 1 件・組み直しの後に起こす試みが無い）。
///
/// # 非空虚性
/// 既定の判定を外して `switch_to_default` へ渡すと、既定へ戻した記録 `fell_back_to_default` が残り
/// （または 2 度目の試みの致命が数に乗り）赤。
#[test]
fn a_failed_shiori_rebuild_of_the_default_ghost_is_fatal() {
    use crate::boot_resolve::DEFAULT_GHOST_FOLDER;
    use crate::ghost_session::GhostBootInputsSource;

    let mut rig = SwitchRig::new(vec![(DEFAULT_GHOST_FOLDER, a_shiori_script())]);
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record(DEFAULT_GHOST_FOLDER);
    rig.boot(DEFAULT_GHOST_FOLDER);
    assert!(rig.wait_steady(), "既定が定常に着く");
    let original = rig
        .world
        .remove_non_send::<GhostBootInputsSource>()
        .expect("作り口が据わっている");
    rig.world.insert_non_send(GhostBootInputsSource(Box::new(
        move |cfg: &crate::ConfigInputs, origin| {
            let mut inputs = (original.0)(cfg, origin);
            inputs.wiring.ghost_root = cfg.ghost_root.join("実在しない根");
            inputs
        },
    )));

    let (flow, events) = capture(|| {
        let verdict = reload_shiori(&mut rig);
        (verdict, rig.pump_talking_until(SwitchRig::exit_requested))
    });
    let loads = rig.calls(DEFAULT_GHOST_FOLDER).len();
    let in_flight = (
        rig.world.get_non_send::<SwitchInFlight>().is_some(),
        rig.world.get_non_send::<ReloadInFlight>().is_some(),
    );
    assert!(rig.shutdown());

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    // 作り口は入力を作るたびに偽の SHIORI を 1 つ組む: 最初の起動と組み直しの 2 回（既定へ戻す 3 回目が無い）。
    assert_eq!(loads, 2, "既定へ戻す 2 度目の試みが無い: {events:?}");
    assert_eq!(in_flight, (false, false), "予約と印は下りる: {events:?}");
    assert_eq!(named(&events, "ghost_switch_fatal"), 1, "{events:?}");
    let reasons: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_failed"))
        .filter_map(|e| e.field_str("reason").map(str::to_owned))
        .collect();
    assert!(
        reasons.iter().all(|r| r != "fell_back_to_default"),
        "既定へ戻す試みをしない: {events:?}"
    );
}

/// 組み直しの降ろした直後の記憶を書けない（要件 4.4・write-failure-notice 要件 3.3 と同じ扱い）:
/// 予約と印を下ろし、最初の出所が `MemoryWriteFailed` で終了が指示され、窓は閉じられない。
///
/// アプリの記憶の置き場の位置に普通のファイルを置いて書けなくする（足場は実行系へ置き場を渡さない
/// ＝古い実行系の降ろしの記憶はそこへ届かないので、書けない記録は組み直しの 1 回だけ）。
///
/// # 非空虚性
/// `WriteFailed` で予約を下ろさないと予約と印が残って赤。致命へ渡さないと終了が指示されず、待ちが
/// 打ち切られて赤。
#[test]
fn a_shiori_rebuild_that_cannot_write_the_drop_memory_is_fatal() {
    use crate::app_exit::{ExitOrigin, FirstExit};

    let mut rig = running_a_with(vec![("A", a_shiori_script())]);
    let profile = rig.app_dir();
    if profile.is_dir() {
        std::fs::remove_dir_all(&profile).expect("置き場のフォルダを外す");
    }
    std::fs::write(&profile, "x").expect("置き場の位置にファイルを置く");

    let (flow, events) = capture(|| {
        let verdict = reload_shiori(&mut rig);
        (verdict, rig.pump_talking_until(SwitchRig::exit_requested))
    });
    let origin = rig.world.get_resource::<FirstExit>().map(|f| f.0.clone());
    let in_flight = (
        rig.world.get_non_send::<SwitchInFlight>().is_some(),
        rig.world.get_non_send::<ReloadInFlight>().is_some(),
    );
    assert!(rig.shutdown());

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert!(
        matches!(origin, Some(ExitOrigin::MemoryWriteFailed(_))),
        "最初の出所: {origin:?}"
    );
    assert_eq!(in_flight, (false, false), "予約と印は下りる: {events:?}");
    assert_eq!(
        (
            named(&events, "switch_drop_record_degraded"),
            named(&events, "windows_closed_for_restart"),
            named(&events, "ghost_switch_fatal"),
        ),
        (1, 0, 0),
        "(書けない記録・窓を閉じた・切替の致命): {events:?}"
    );
}

/// 新しい結線が読み直しの前の面の控えを持つか（`Emo2Wiring` が無ければ `None`）。
fn carry_handed_over(rig: &SwitchRig) -> Option<bool> {
    rig.world
        .get_non_send::<crate::emo2_boot::frame::Emo2Wiring>()
        .map(|w| w.shell_carry.is_some())
}

/// 控えを取った記録（取れた・取れなかった）の件数。
fn carry_records(events: &[CapturedEvent]) -> usize {
    named(events, "reload_shell_captured") + named(events, "reload_shell_carry_unavailable")
}

/// ghost の読み直しは窓を閉じる前に表示中の面を控え、起こし直した新しい結線へ渡す（要件 3.8）。
/// 窓口を通さない自分自身への切替は控えない（要件 3.6・今までどおり最初の `\s` まで面を出さない）。
///
/// # 非空虚性
/// `switch_to` が控えを取らない・渡さないと、読み直しの側が `Some(false)` で赤。読み直しでない切替でも
/// 控えると、切替の側が `Some(true)` で赤。
#[test]
fn a_ghost_reload_hands_the_shown_surfaces_to_the_new_windows_but_a_switch_does_not() {
    let mut rig = running_a();
    let (flow, events) = capture(|| {
        let verdict = request_reload(
            &mut rig.world,
            ReloadRequest {
                target: ReloadTarget::Ghost,
                origin: ReloadOrigin::Script,
                notes: DescriptNotes::default(),
            },
        );
        (verdict, rig.pump_talking_until(a_rebooted))
    });
    let reloaded = carry_handed_over(&rig);
    assert!(rig.shutdown());
    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert_eq!(reloaded, Some(true), "{events:?}");
    // 窓の生えない土台では窓の一覧が無いので、控えは空（`reload_shell_carry_unavailable`）になる。
    assert_eq!(carry_records(&events), 1, "{events:?}");

    let mut rig = running_a();
    let (flow, events) = capture(|| {
        (
            plain_switch_to_a(&mut rig),
            rig.pump_talking_until(a_rebooted),
        )
    });
    let switched = carry_handed_over(&rig);
    assert!(rig.shutdown());
    assert_eq!(flow, (SwitchVerdict::Accepted, true), "{events:?}");
    assert_eq!(switched, Some(false), "{events:?}");
    assert_eq!(carry_records(&events), 0, "{events:?}");
}

/// ghost の読み直しで起こし直しに失敗し既定のゴーストへ戻ったときは、控えを既定のゴーストの窓へ渡さない
/// （要件 3.7・3.8: 控えは読み直したゴーストの面と着せ替えで、別のゴーストの窓には効かせない）。
///
/// # 非空虚性
/// `hand_over` を `boot_into` の成否より前（または `switch_to_default` の後）に呼ぶと、既定のゴーストの
/// 結線が控えを持って `Some(true)` で赤。
#[test]
fn a_failed_ghost_reload_hands_no_carry_to_the_default_ghost() {
    use crate::boot_resolve::DEFAULT_GHOST_FOLDER;
    use crate::ghost_session::GhostBootInputsSource;

    let mut rig = running_a_with(vec![
        ("A", a_script()),
        (
            DEFAULT_GHOST_FOLDER,
            FakeShiori::Scripted(Box::new(|| standard_script(r"\0emo2\e"))),
        ),
    ]);
    let original = rig
        .world
        .remove_non_send::<GhostBootInputsSource>()
        .expect("作り口が据わっている");
    rig.world.insert_non_send(GhostBootInputsSource(Box::new(
        move |cfg: &crate::ConfigInputs, origin| {
            let mut inputs = (original.0)(cfg, origin);
            if cfg.ghost_root.ends_with("A") {
                inputs.wiring.ghost_root = cfg.ghost_root.join("実在しない根");
            }
            inputs
        },
    )));

    let (flow, events) = capture(|| {
        let verdict = request_reload(
            &mut rig.world,
            ReloadRequest {
                target: ReloadTarget::Ghost,
                origin: ReloadOrigin::Script,
                notes: DescriptNotes::default(),
            },
        );
        let done = rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (rig.calls(DEFAULT_GHOST_FOLDER).len() == 1
                    && rig.world.get_non_send::<SwitchInFlight>().is_none()
                    && rig.world.get_non_send::<ReloadInFlight>().is_none())
        });
        (verdict, done)
    });
    let exited = rig.exit_requested();
    let carried = carry_handed_over(&rig);
    assert!(rig.shutdown());

    assert_eq!(flow, (ReloadVerdict::Accepted, true), "{events:?}");
    assert!(!exited, "既定へ戻って続く: {events:?}");
    assert_eq!(
        carry_records(&events),
        1,
        "読み直しの前に控えは取る: {events:?}"
    );
    assert_eq!(
        carried,
        Some(false),
        "既定のゴーストの結線は控えを持たない: {events:?}"
    );
    let reasons: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_failed"))
        .map(|e| e.field_str("reason").map(str::to_owned))
        .collect();
    assert_eq!(
        reasons,
        [Some("fell_back_to_default".to_owned())],
        "{events:?}"
    );
}
