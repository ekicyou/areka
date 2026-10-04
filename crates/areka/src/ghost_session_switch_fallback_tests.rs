//! 切替の失敗方向の統合テスト（areka-P0-ghost-shell-balloon-switch task 9.2・要件 4.10・6.1〜6.5・10.5）。
//!
//! 偽の SHIORI の土台で A から B へ切り替え、B が起きられないときの行き先を本物の経路で通す:
//! ⑴ B の SHIORI が接続に失敗（起こした後に非同期の `Fault`）→ 既定ゴーストが `OnBoot` の
//! Ref6＝`halt`・Ref7＝B の名前で起き、`OnGhostChanged` 0 件・告知なし・終了の指示なし
//! ⑵ 根に既定ゴーストが無い → 終了の指示・最初の出所が「既定ゴーストへ戻せなかった」
//! ⑶ 既定ゴーストも接続に失敗 → 今日の SHIORI の失敗の経路で終了（告知の場面は既存の 1 つ）
//! ⑷ 切替先の起動が同期で失敗 → 入力の段のあと、ゴーストの窓は既定ゴーストの分だけ・窓の束が 1 つ
//! ⑸ 切替先の窓の閉包が着く前に切替先が失敗 → 入力の段のあと、ゴーストの窓は既定ゴーストの分だけ。
//! 窓は作業プールの閉包が `Input` 段の取り出し（`drain_task_pool_commands`）で World へ生やす
//! 実体（`GhostWindowMarker` の付いた entity）で数える（OS の窓はこの World には作られない）。
//! 判定は集めてから 1 回・降ろすのは必ず有界に行う。

use std::collections::BTreeSet;

use areka_kanade::ChangeOrigin;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use shiori_host32_host::ExitKind;
use wintf::ecs::Window;
use wintf::ecs::widget::bitmap_source::systems::drain_task_pool_commands;
use wintf::ecs::widget::bitmap_source::{BoxedCommand, CommandSender};

use super::*;
use crate::alert::AlertScene;
use crate::boot_config::BootContext;
use crate::boot_resolve::{DEFAULT_GHOST_FOLDER, GhostRoute};
use crate::emo2_boot::ghost_switch::{
    GhostSpec, SwitchRequest, SwitchVerdict, request_ghost_switch,
};
use crate::emo2_boot::spine::{ScriptedShioriBackend, spin_wait_until};
use crate::ghost_session::{GhostSlot, open_ghost_windows};
use crate::placement::spawn::{GhostWindowMarker, GhostWindows};

/// B の `descript.txt` の `name`（フォルダ名と違う綴り＝Ref7 がフォルダ名でなく名前を運ぶことを見る）。
pub(super) const B_NAME: &str = "ビー";

/// `folder` の `descript.txt` の `name` を `name` へ書き換える。
fn rename_ghost(rig: &SwitchRig, folder: &str, name: &str) {
    let descript = rig
        .root
        .ghost_dir(folder)
        .join("ghost")
        .join("master")
        .join("descript.txt");
    let text = std::fs::read_to_string(&descript).expect("descript.txt は UTF-8");
    let rewritten: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with("name,") {
                format!("name,{name}")
            } else {
                line.to_owned()
            }
        })
        .collect();
    std::fs::write(&descript, rewritten.join("\r\n")).expect("descript.txt を書き換える");
}

/// A（台本で B への切替を命じる）・B・既定ゴースト（`default`・無ければ根から消す）を据えた土台。
/// A と既定ゴーストは起動記録あり、B の名前は [`B_NAME`]。
pub(super) fn fallback_rig(a: FakeShiori, b: FakeShiori, default: Option<FakeShiori>) -> SwitchRig {
    let with_default = default.is_some();
    let mut scripts = vec![("A", a), ("B", b)];
    scripts.extend(default.map(|d| (DEFAULT_GHOST_FOLDER, d)));
    let mut rig = SwitchRig::new(scripts);
    rig.world.insert_resource(WintfTaskPool::new());
    rig.plant_boot_record("A");
    if with_default {
        rig.plant_boot_record(DEFAULT_GHOST_FOLDER);
    } else {
        std::fs::remove_dir_all(rig.root.ghost_dir(DEFAULT_GHOST_FOLDER))
            .expect("根から既定ゴーストを消す");
    }
    rename_ghost(&rig, "B", B_NAME);
    rig
}

/// 既定ゴーストの偽の SHIORI（標準の台本・`OnGhostChanged` の応答は持たない）。
pub(super) fn default_ghost() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| standard_script("\\0emo2\\e")))
}

/// 既定ゴーストの 1 度目の起動の `OnBoot` の Ref1 以降。
fn default_on_boot_tail(rig: &SwitchRig) -> Option<Vec<String>> {
    rig.calls(DEFAULT_GHOST_FOLDER)
        .first()
        .and_then(|c| get_refs(c, "OnBoot"))
        .map(|refs| refs.get(1..).map(<[String]>::to_vec).unwrap_or_default())
}

/// 既定ゴーストの `OnBoot` に載るはずの Ref1〜7（Ref1〜5 は空・Ref6＝halt・Ref7＝B の名前）。
fn halted_tail() -> Option<Vec<String>> {
    let mut tail = vec![String::new(); 5];
    tail.extend(["halt".to_owned(), B_NAME.to_owned()]);
    Some(tail)
}

/// `event` の記録の件数。
fn count_of(events: &[CapturedEvent], event: &str) -> usize {
    levels_of(events, event).len()
}

/// `run()` の後の組み立て（`main` の `after_run`）を踏み、告知の場面のゴースト名と終了コード 1 の
/// 旗を返す（取り出された単位は置き場へ戻す＝後で有界に降ろす）。
fn after_run_alert(rig: &mut SwitchRig) -> (Option<Option<String>>, bool) {
    let mut after = crate::after_run(&mut rig.world);
    rig.world.insert_non_send(GhostSlot(after.session.take()));
    let scene = after.scene.map(|scene| match scene {
        AlertScene::ShioriFault { ghost_name, .. } => ghost_name,
        other => panic!("SHIORI の失敗の場面でない: {other:?}"),
    });
    (scene, after.fault)
}

/// B の SHIORI が接続に失敗する（起こした後に非同期の `Fault`＝迎え入れ（切替先）の段）→ 既定ゴーストが
/// 通常の起動系列で起き、`OnBoot` の Ref6＝`halt`・Ref7＝B の名前（`descript.txt` の `name`）、
/// `OnGhostChanged` は 0 件（交代は成立していない・A の送り出しの台本は載らない）。告知は出ず
/// （`after_run` の場面なし・終了コード 0・`alert` の記録 0 件）、終了も指示されない。
/// 記録は `error!(ghost_switch_target_fault)` 1 件。起動の文脈の今のゴーストは既定（経路＝既定）。
///
/// # 非空虚性
/// 迎え入れ（切替先）の `Fault` を今日どおり終了へ流すと終了の指示・告知が立って赤。既定へ戻す由来を
/// 「ふつう」にすると `OnBoot` の Ref6/7 が載らず赤。
#[test]
fn target_connect_fail_boots_default_with_halt_and_no_alert() {
    let mut rig = fallback_rig(
        ghost_a(A_TO_B, None),
        FakeShiori::ConnectFail,
        Some(default_ghost()),
    );
    rig.boot("A");
    let steady = rig.wait_steady();
    let (welcomed, events) = capture(|| pump_to_welcomed(&mut rig, DEFAULT_GHOST_FOLDER, 1));

    let default_kinds: Vec<Vec<String>> = rig
        .calls(DEFAULT_GHOST_FOLDER)
        .iter()
        .map(|c| kinds(c))
        .collect();
    let on_boot_tail = default_on_boot_tail(&rig);
    let b_boots = rig.calls("B").len();
    let exit_requested = rig.exit_requested();
    let first_exit = rig.world.contains_resource::<FirstExit>();
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let current = rig
        .world
        .get_resource::<BootContext>()
        .map(|c| (c.current.ghost.route, c.current.ghost.folder.clone()));
    let (scene, fault) = after_run_alert(&mut rig);
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            (steady, welcomed, default_kinds, on_boot_tail, b_boots),
            (exit_requested, first_exit, reserved, current),
            (scene, fault, count_of(&events, "alert")),
            (
                levels_of(&events, "ghost_switch_target_fault"),
                count_of(&events, "ghost_switch_default_fault"),
                count_of(&events, "ghost_switch_fatal"),
            ),
            shutdown_ok,
        ),
        (
            (
                true,
                true,
                vec![vec![
                    "NOTIFY OnInitialize".to_owned(),
                    "GET username".to_owned(),
                    "GET OnBoot".to_owned(),
                    "GET OnTranslate".to_owned(),
                    "NOTIFY basewareversion".to_owned(),
                ]],
                halted_tail(),
                0,
            ),
            (
                false,
                false,
                false,
                Some((GhostRoute::Default, Some(DEFAULT_GHOST_FOLDER.to_owned()))),
            ),
            (None, false, 0),
            (vec![tracing::Level::ERROR], 0, 0),
            true,
        ),
        "切替先の失敗で既定ゴーストへ戻らない（A の定常・既定の定常まで届いた・既定の呼出列・OnBoot の \
         Ref1〜7・B の台本の数／終了の指示・最初の出所・予約・今のゴースト／告知の場面・終了コード 1・\
         alert の件数／切替先の失敗・既定の失敗・致命の記録／降ろせた): {events:?}"
    );
}

/// 根に既定ゴーストが無いまま B が接続に失敗 → 戻す先が無いので致命: 終了が指示され、最初の出所は
/// 「既定ゴーストへ戻せなかった」（理由に落ちた B の名前）。`main` の後始末は既存の SHIORI の失敗の
/// 場面を既定ゴーストの名前で組み、終了コード 1（要件 6.4・10.5）。
///
/// # 非空虚性
/// 既定が目録に無いときに致命へ進まず何もしないと、終了が指示されず期限切れで赤。
#[test]
fn target_fault_without_default_ghost_is_fatal() {
    let mut rig = fallback_rig(ghost_a(A_TO_B, None), FakeShiori::ConnectFail, None);
    rig.boot("A");
    let steady = rig.wait_steady();
    let (exited, events) = capture(|| rig.pump_talking_until(SwitchRig::exit_requested));

    let origin = rig
        .world
        .get_resource::<FirstExit>()
        .map(|first| match &first.0 {
            ExitOrigin::GhostFallbackFailed(f) => Ok((f.kind, f.reason.contains(B_NAME))),
            other => Err(format!("{other:?}")),
        });
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let (scene, fault) = after_run_alert(&mut rig);
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            (steady, exited, origin, reserved),
            (scene, fault),
            (
                levels_of(&events, "ghost_switch_target_fault"),
                levels_of(&events, "ghost_switch_fatal"),
                count_of(&events, "ghost_switch_booted"),
            ),
            shutdown_ok,
        ),
        (
            (
                true,
                true,
                Some(Ok((ShioriFaultKind::Internal, true))),
                false
            ),
            (Some(Some(DEFAULT_GHOST_FOLDER.to_owned())), true),
            (vec![tracing::Level::ERROR], vec![tracing::Level::ERROR], 1),
            true,
        ),
        "既定ゴーストの無い根で致命にならない（A の定常・終了の指示・最初の出所（種類・理由に B の名前）・\
         予約／告知の場面のゴースト名・終了コード 1／切替先の失敗・致命の記録・切替で起こした数／\
         降ろせた): {events:?}"
    );
}

/// 既定ゴーストも接続に失敗 → 戻す試みは 1 回だけ（既定から先へは戻さない）で、今日の SHIORI の失敗の
/// 経路（最初の出所が kanade の停止の `Fault`・告知の場面は既存の 1 つ・終了コード 1）で終える
/// （要件 6.4・6.5・10.5）。
///
/// # 非空虚性
/// 既定の失敗でも既定へ戻す経路へ入ると、既定を起こし直し続けて終了が指示されず（期限切れ）赤。
#[test]
fn default_ghost_fault_after_fallback_exits_through_shiori_fault_path() {
    let mut rig = fallback_rig(
        ghost_a(A_TO_B, None),
        FakeShiori::ConnectFail,
        Some(FakeShiori::ConnectFail),
    );
    rig.boot("A");
    let steady = rig.wait_steady();
    let (exited, events) = capture(|| rig.pump_talking_until(SwitchRig::exit_requested));

    let origin = rig
        .world
        .get_resource::<FirstExit>()
        .map(|first| match &first.0 {
            ExitOrigin::KanadeStopped(KanadeStopCause::Fault(f)) => Ok(f.kind),
            other => Err(format!("{other:?}")),
        });
    let reserved = rig.world.get_non_send::<SwitchInFlight>().is_some();
    let (scene, fault) = after_run_alert(&mut rig);
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            (steady, exited, origin, reserved),
            (scene.is_some(), fault),
            (
                levels_of(&events, "ghost_switch_target_fault"),
                levels_of(&events, "ghost_switch_default_fault"),
                count_of(&events, "ghost_switch_booted"),
                count_of(&events, "ghost_switch_fatal"),
            ),
            shutdown_ok,
        ),
        (
            (true, true, Some(Ok(ShioriFaultKind::ConnectFailed)), false),
            (true, true),
            (
                vec![tracing::Level::ERROR],
                vec![tracing::Level::ERROR],
                2,
                0
            ),
            true,
        ),
        "既定ゴーストの失敗が今日の失敗の経路で終わらない（A の定常・終了の指示・最初の出所・予約／告知の\
         場面がある・終了コード 1／切替先の失敗・既定の失敗・切替で起こした数・致命の記録／降ろせた): \
         {events:?}"
    );
}

// ── 孤児の窓 0（要件 4.10・6.1） ──

/// 台詞を返さない A（`OnBoot` 204＝定常に入ったとき再生中の台詞が無い）。台詞の時計を回さずに
/// 切替の要求が受理されて A が降ろされる（`Input` 段を回さずに切替を進めるため）。
fn quiet_a() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        ScriptedShioriBackend::builder()
            .notify("OnInitialize", Ok(()))
            .get("OnBoot", Ok(None))
            .notify("basewareversion", Ok(()))
            .notify("OnClose", Ok(()))
            .unload(Ok(ExitKind::Clean))
    }))
}

/// 窓を生やす土台: 作業プールと、その閉包を World へ適用する取り出しの系（`Input` 段）を据える。
fn windows_rig(b: FakeShiori) -> SwitchRig {
    let mut rig = fallback_rig(quiet_a(), b, Some(default_ghost()));
    rig.world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain_task_pool_commands);
    rig
}

/// ゴーストの窓の実体の数。
fn window_count(world: &mut World) -> usize {
    world
        .query_filtered::<(), With<GhostWindowMarker>>()
        .iter(world)
        .count()
}

/// ゴーストの窓の題（並べ替え済み）。
fn window_titles(world: &mut World) -> Vec<String> {
    let mut titles: Vec<String> = world
        .query_filtered::<&Window, With<GhostWindowMarker>>()
        .iter(world)
        .map(|w| w.title.clone())
        .collect();
    titles.sort();
    titles
}

/// 窓の束（`GhostWindows`）が指す実体の集まりが、生きているゴーストの窓の実体の集まりとちょうど
/// 一致するか（束が 1 つで、束の外の窓＝孤児が無い）。束が無ければ `false`。
fn one_bundle_covers_all(world: &mut World) -> bool {
    let live: BTreeSet<Entity> = world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(world)
        .collect();
    let Some(bundle) = world.get_resource::<GhostWindows>() else {
        return false;
    };
    let listed: BTreeSet<Entity> = bundle
        .scopes()
        .flat_map(|s| [bundle.char_window(s), bundle.balloon_window(s)])
        .flatten()
        .collect();
    listed == live
}

/// `Input` 段を `done` が真になるまで有界に回す（作業プールの閉包は別スレッドから届く）。
fn run_input_until(rig: &mut SwitchRig, mut done: impl FnMut(&mut World) -> bool) -> bool {
    spin_wait_until(|| {
        rig.world.run_schedule(Input);
        done(&mut rig.world)
    })
}

/// 既定ゴーストの窓の題（emo2 はスコープ 2 つ＝キャラ窓とバルーン窓で 4 枚）。
fn default_titles() -> Vec<String> {
    ["むらさき", "むらさき", "エモ", "エモ"]
        .map(str::to_owned)
        .to_vec()
}

/// メニューからの B への切替（`OnGhostChanging` を送らない＝台詞を回さずに A を降ろす）を入口へ渡す。
fn request_b(rig: &mut SwitchRig) -> SwitchVerdict {
    request_ghost_switch(
        &mut rig.world,
        SwitchRequest {
            ghost: GhostSpec::Folder("B".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Manual,
            boot_event: None,
        },
    )
}

/// 既定ゴーストの定常到達まで通知の相だけを回す（`Input` 段は回さない＝窓の閉包は溜まったまま）。
fn pump_to_default_steady(rig: &mut SwitchRig) -> bool {
    rig.pump_until(|rig| {
        rig.exit_requested()
            || (!rig.calls(DEFAULT_GHOST_FOLDER).is_empty()
                && rig.world.get_non_send::<SwitchInFlight>().is_none())
    })
}

/// 切替先の起動が同期で失敗（起動の結線が成立しない）→ 既定ゴーストへ戻す。A の窓を生やしてから
/// 切り替え、既定ゴーストの定常のあと `Input` 段を回すと、ゴーストの窓は既定ゴーストの 4 枚だけ
/// （A の窓も B の窓も残らない）で、窓の束は 1 つ（束の外の窓 0）。記録は
/// `error!(ghost_switch_boot_failed, stage=boot)` 1 件、既定は `OnBoot` の Ref6/7 つきで起きる。
///
/// # 非空虚性
/// 厳格な入口の代わりに fallback の入口で切替先を起こすと、B が骨格のまま「起きた」ことになり窓が
/// B の題（Bのさくら）で生えて赤。
#[test]
fn sync_target_boot_failure_leaves_only_default_windows() {
    let mut rig = windows_rig(FakeShiori::WiringFail);
    let cfg_a = rig.cfg("A");
    let a_opened = open_ghost_windows(&mut rig.world, &cfg_a).is_ok();
    rig.boot("A");
    let a_windows = run_input_until(&mut rig, |w| window_count(w) > 0);
    let a_titles = window_titles(&mut rig.world);
    let steady = rig.wait_steady();
    let verdict = request_b(&mut rig);
    let (welcomed, events) = capture(|| pump_to_default_steady(&mut rig));
    let spawned = run_input_until(&mut rig, |w| window_count(w) > 0);

    let titles = window_titles(&mut rig.world);
    let one_bundle = one_bundle_covers_all(&mut rig.world);
    let boot_failed_stages: Vec<Option<String>> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("ghost_switch_boot_failed"))
        .map(|e| e.field_str("stage").map(str::to_owned))
        .collect();
    let on_boot_tail = default_on_boot_tail(&rig);
    let exit_requested = rig.exit_requested();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            (a_opened, a_windows, a_titles, steady, verdict, welcomed),
            (spawned, titles, one_bundle),
            (
                boot_failed_stages,
                on_boot_tail,
                exit_requested,
                shutdown_ok
            ),
        ),
        (
            (
                true,
                true,
                ["Aのさくら", "Aのさくら", "エモ", "エモ"]
                    .map(str::to_owned)
                    .to_vec(),
                true,
                SwitchVerdict::Accepted,
                true,
            ),
            (true, default_titles(), true),
            (vec![Some("boot".to_owned())], halted_tail(), false, true),
        ),
        "同期の失敗で既定の窓だけにならない（A の窓・A の題・A の定常・入口の判定・既定の定常まで届いた／\
         窓が生えた・窓の題・束が 1 つ／起動の失敗の段・OnBoot の Ref1〜7・終了の指示・降ろせた): {events:?}"
    );
}

/// 切替先の窓の閉包が `Input` 段に着く前に切替先の SHIORI が失敗（非同期の `Fault`）→ 既定へ戻す。
/// 溜まった 2 つの閉包（B・既定）を投函順に `Input` 段へ通すと、B の閉包は窓を閉じた回数の食い違いで
/// 窓を作らず（`debug!(ghost_windows_stale)` 1 件）、ゴーストの窓は既定ゴーストの 4 枚だけ・束は 1 つ。
///
/// # 非空虚性
/// 着いた閉包が窓を閉じた回数を照合しないと、B の窓 4 枚が既定の窓と並んで生え（8 枚・Bのさくら）赤。
#[test]
fn async_target_fault_before_window_closure_leaves_only_default_windows() {
    let mut rig = windows_rig(FakeShiori::ConnectFail);
    rig.boot("A");
    let steady = rig.wait_steady();
    let verdict = request_b(&mut rig);
    let welcomed = pump_to_default_steady(&mut rig);

    // 溜まった閉包（B の分・既定の分）を取り出し、投函順のまま 1 つのタスクで作業プールへ戻す
    // （別々のタスクだと着く順が揺れる）。そのうえで `Input` 段を回す。
    let mut pending: Vec<BoxedCommand> = Vec::new();
    let both_queued = spin_wait_until(|| {
        pending.extend(rig.world.resource::<WintfTaskPool>().drain_commands());
        pending.len() >= 2
    });
    let queued = pending.len();
    rig.world
        .resource::<WintfTaskPool>()
        .spawn(|tx: CommandSender| async move {
            for cmd in pending {
                let _ = tx.send(cmd);
            }
        });
    let (spawned, events) = capture(|| run_input_until(&mut rig, |w| window_count(w) > 0));

    let titles = window_titles(&mut rig.world);
    let one_bundle = one_bundle_covers_all(&mut rig.world);
    let on_boot_tail = default_on_boot_tail(&rig);
    let exit_requested = rig.exit_requested();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            (steady, verdict, welcomed, both_queued, queued),
            (
                spawned,
                titles,
                one_bundle,
                count_of(&events, "ghost_windows_stale")
            ),
            (on_boot_tail, exit_requested, shutdown_ok),
        ),
        (
            (true, SwitchVerdict::Accepted, true, true, 2),
            (true, default_titles(), true, 1),
            (halted_tail(), false, true),
        ),
        "切替先の閉包が着く前の失敗で孤児の窓が生えた（A の定常・入口の判定・既定の定常まで届いた・\
         閉包が 2 つ溜まった・溜まった数／窓が生えた・窓の題・束が 1 つ・古い閉包の記録／OnBoot の \
         Ref1〜7・終了の指示・降ろせた): {events:?}"
    );
}
