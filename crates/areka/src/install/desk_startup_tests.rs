//! 起動の依頼の持ち物と窓口の結線のテスト（startup-install-option・design「Testing Strategy / 結線」・
//! 要件 2.1・2.5・2.7・4.1・4.6・4.7・6.1〜6.5・7.1・7.4・9.2・9.4・9.6・9.7）。`desk_tests.rs` の子
//! （道具立ての `Rig` を借りる）。
//!
//! 確かめること: 定常到達は切替の道筋の通知の受け手（`ghost_switch::on_notice`）から窓口へ届き、
//! 最初の 1 回だけ起動の依頼を積む・`initial_install.nar` が先・readonly では 2 件とも断られて控えが
//! 無い・終了の後始末（`exit_wait::begin_close`）が定常の前なら出さずに warn 1 件。

use crate::cli::InstallArgs;
use crate::install::queued_orders;
use crate::install::startup::{self, INITIAL_INSTALL_FILE, StartupInstall};
use crate::readonly::ReadOnlyRun;

use super::*;

fn args(archives: &[PathBuf], words: bool) -> InstallArgs {
    InstallArgs {
        archives: archives.to_vec(),
        call_ghost: words,
        delete_source: words,
    }
}

/// 起動の依頼の持ち物を据えた Rig（exe の隣の代わりは `exe_dir`）。
fn rig_with(args: InstallArgs, exe_dir: Option<PathBuf>) -> Rig {
    let mut rig = Rig::new();
    startup::install(&mut rig.world, StartupInstall::new_with(args, exe_dir));
    rig
}

fn steady(rig: &mut Rig) {
    on_notice(&mut rig.world, KanadeNotice::Steady);
}

fn not_submitted(events: &[CapturedEvent]) -> Vec<String> {
    events_named(events, "startup_install_not_submitted")
        .into_iter()
        .map(|e| {
            assert_eq!(e.level, Level::WARN);
            e.field_str("reason").unwrap_or_default().to_owned()
        })
        .collect()
}

/// 定常到達を 2 回送っても依頼は 1 件で、書庫の並びが書いた順・出どころは起動の引数。
/// exe の隣に `initial_install.nar` が無ければその依頼は無く、warn 以上の記録も無い。取り出した後に
/// 落としても出さずに終わる記録は出ない（要件 2.1・2.5・4.6・9.2）。
#[test]
fn two_steadies_queue_one_startup_order_in_written_order() {
    let tmp = TempPath::under_target("desk-startup-once");
    let exe = tmp.child("exe");
    std::fs::create_dir_all(&exe).expect("exe の代わり");
    let archives = vec![PathBuf::from(r"C:\b.nar"), PathBuf::from(r"C:\a.nar")];
    let mut rig = rig_with(args(&archives, true), Some(exe));

    let (orders, events) = capture(|| {
        steady(&mut rig);
        steady(&mut rig);
        let orders = queued_orders(&rig.world);
        drop(rig);
        orders
    });
    assert_eq!(
        orders,
        vec![InstallOrder {
            archives,
            origin: InstallOrigin::StartupArgs,
        }]
    );
    let queued = events_named(&events, "install_order_queued");
    assert_eq!(queued.len(), 1, "{events:?}");
    assert_eq!(queued[0].field("origin"), Some("StartupArgs"));
    assert_eq!(queued[0].field("count"), Some("2"));
    assert!(
        !events.iter().any(|e| e.level <= Level::WARN),
        "warn 以上は無い: {events:?}"
    );
}

/// exe の隣の `initial_install.nar`（大小を問わない）は起動の引数の依頼より先に別の依頼で積まれる
/// （要件 4.1・4.7・9.4）。
#[test]
fn initial_install_next_to_exe_is_queued_before_the_args() {
    for name in [INITIAL_INSTALL_FILE, "Initial_Install.NAR"] {
        let tmp = TempPath::under_target("desk-startup-initial");
        let exe = tmp.child("exe");
        std::fs::create_dir_all(&exe).expect("exe の代わり");
        std::fs::write(exe.join(name), b"nar").expect("書庫の代わり");
        let archives = vec![PathBuf::from(r"C:\a.nar")];
        let mut rig = rig_with(args(&archives, false), Some(exe.clone()));

        let ((), events) = capture(|| steady(&mut rig));
        let origins: Vec<_> = queued_orders(&rig.world)
            .into_iter()
            .map(|o| (o.origin, o.archives))
            .collect();
        assert_eq!(
            origins,
            vec![
                (
                    InstallOrigin::InitialInstall,
                    vec![exe.join(INITIAL_INSTALL_FILE)]
                ),
                (InstallOrigin::StartupArgs, archives),
            ],
            "{name}"
        );
        let queued: Vec<_> = events_named(&events, "install_order_queued")
            .iter()
            .map(|e| e.field("origin").unwrap_or_default().to_owned())
            .collect();
        assert_eq!(queued, ["InitialInstall", "StartupArgs"], "{name}");
    }
}

/// readonly の World では 2 件とも断られ `readonly_refused` に `origin` が載り、待ち行列は空で、
/// 終わりの控えが無く（終了の後始末で語つきの控えの warn が出ない）、写しは消えない（要件 6.1〜6.5・
/// 7.4・9.6・9.7）。
#[test]
fn readonly_refuses_both_with_origin_and_keeps_no_followup() {
    let tmp = TempPath::under_target("desk-startup-readonly");
    let exe = tmp.child("exe");
    std::fs::create_dir_all(&exe).expect("exe の代わり");
    let initial = exe.join(INITIAL_INSTALL_FILE);
    std::fs::write(&initial, b"nar").expect("書庫の代わり");
    let copy = tmp.child("a.nar");
    std::fs::write(&copy, b"nar").expect("写し");
    let mut rig = rig_with(args(std::slice::from_ref(&copy), true), Some(exe));
    rig.world.insert_resource(ReadOnlyRun);

    let ((), events) = capture(|| {
        steady(&mut rig);
        let _ = begin_close(&mut rig.world);
    });
    let origins: Vec<_> = events_named(&events, "readonly_refused")
        .iter()
        .map(|e| e.field("origin").unwrap_or_default().to_owned())
        .collect();
    assert_eq!(origins, ["InitialInstall", "StartupArgs"]);
    assert!(queued_orders(&rig.world).is_empty(), "待ち行列に積まない");
    assert!(
        events_named(&events, "startup_followup_dropped").is_empty(),
        "断られたら控えない: {events:?}"
    );
    assert!(initial.is_file() && copy.is_file(), "写しは消えない");
}

/// 定常到達の前に終了の後始末が走ると、出さずに終わる warn（`exit_before_steady`）が 1 件で、
/// その後に定常到達が来ても積まず、World を落としても 2 件目は出ない（要件 2.7）。
#[test]
fn exit_before_steady_warns_once_and_not_again_on_drop() {
    let mut rig = rig_with(args(&[PathBuf::from(r"C:\a.nar")], false), None);
    let (orders, events) = capture(|| {
        let _ = begin_close(&mut rig.world);
        steady(&mut rig);
        let orders = queued_orders(&rig.world);
        drop(rig);
        orders
    });
    assert!(orders.is_empty());
    assert_eq!(not_submitted(&events), ["exit_before_steady"]);
}

/// 定常到達の前に World を落とすと `boot_stopped` の warn が 1 件。入れる物が無ければ出ない
/// （要件 7.1）。
#[test]
fn dropping_before_steady_warns_boot_stopped_only_with_something_to_install() {
    let rig = rig_with(args(&[PathBuf::from(r"C:\a.nar")], false), None);
    let ((), events) = capture(|| drop(rig));
    assert_eq!(not_submitted(&events), ["boot_stopped"]);

    let rig = rig_with(InstallArgs::default(), None);
    let ((), events) = capture(|| drop(rig));
    assert!(not_submitted(&events).is_empty());
}
