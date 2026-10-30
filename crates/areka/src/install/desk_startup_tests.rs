//! 起動の依頼の持ち物と窓口の結線のテスト（startup-install-option・design「Testing Strategy / 結線」・
//! 要件 2.1・2.5・2.7・4.1・4.6・4.7・6.1〜6.5・7.1・7.4・9.2・9.4・9.6・9.7）。`desk_tests.rs` の子
//! （道具立ての `Rig` を借りる）。
//!
//! 確かめること: 定常到達は切替の道筋の通知の受け手（`ghost_switch::on_notice`）から窓口へ届き、
//! 最初の 1 回だけ起動の依頼を積む・`initial_install.nar` が先・readonly では 2 件とも断られて控えが
//! 無い・終了の後始末（`exit_wait::begin_close`）が定常の前なら出さずに warn 1 件。
//! 終わりの知らせ（`DeskAsk::OrderDone`）は窓口の頼みの送出端から `drain` の `answer` を通して届け、
//! 消すのは起動の依頼の「入った」写しだけ・終了の後は消さず頼まない・切り替えの受付の数は切替の
//! 入口（`request_ghost_switch`）を通して確かめる（要件 2.8・3.1・3.5・3.6・4.3・5.1・5.3・9.3〜9.5）。

use std::path::Path;

use areka_kanade::ChangeOrigin;

use crate::cli::InstallArgs;
use crate::emo2_boot::ghost_switch::{
    GhostSpec, SwitchAcceptCount, SwitchRequest, SwitchVerdict, request_ghost_switch,
};
use crate::install::judge::{FailureWord, InstalledItem};
use crate::install::procedure::ArchiveEnd;
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

// ---------------------------------------------------------------- 終わりの受け方・消去・切り替え

fn startup_args(archives: &[PathBuf], call_ghost: bool, delete_source: bool) -> InstallArgs {
    InstallArgs {
        archives: archives.to_vec(),
        call_ghost,
        delete_source,
    }
}

/// 「入った」の終わり方（入れた物 1 つ）。
fn installed(kind: &'static str, place: &Path) -> ArchiveEnd {
    ArchiveEnd::Installed(vec![InstalledItem {
        kind,
        name: format!("{kind} name"),
        place: place.display().to_string(),
    }])
}

/// 写しを 1 本置く（`target\` の下）。
fn copy_at(tmp: &TempPath, name: &str) -> PathBuf {
    let path = tmp.child(name);
    std::fs::write(&path, b"nar").expect("写し");
    path
}

/// 終わりの知らせを窓口の頼みの送出端から入れ、取り出しの系で捌く。
fn done(rig: &mut Rig, order: InstallOrder, ends: Vec<ArchiveEnd>) {
    rig.asks
        .send(DeskAsk::OrderDone { order, ends })
        .expect("窓口の頼みの受信端は生きている");
    rig.drain();
}

/// 定常到達で積まれた依頼の先頭を背景のスレッドの代わりへ渡し、その依頼を返す。
fn steady_and_hand(rig: &mut Rig) -> InstallOrder {
    steady(rig);
    rig.drain();
    rig.orders
        .try_recv()
        .expect("起動の依頼が背景のスレッドへ渡る")
}

fn accepted(world: &World) -> u64 {
    world.get_resource::<SwitchAcceptCount>().map_or(0, |c| c.0)
}

fn dropped_origins(events: &[CapturedEvent]) -> Vec<String> {
    events_named(events, "startup_followup_dropped")
        .into_iter()
        .map(|e| {
            assert_eq!(e.level, Level::WARN);
            e.field("origin").unwrap_or_default().to_owned()
        })
        .collect()
}

/// `deletesource` ありなら「入った」写しだけが消え（ほかの終わり方の写しは残る）、なしなら入っても
/// 消えない。初期インストールは語なしでも入ったら消える（要件 3.1・3.5・3.6・4.3・9.3・9.4）。
#[test]
fn order_done_deletes_only_installed_copies_with_deletesource() {
    for delete_source in [true, false] {
        let tmp = TempPath::under_target("desk-startup-delete");
        let a = copy_at(&tmp, "a.nar");
        let b = copy_at(&tmp, "b.nar");
        let c = copy_at(&tmp, "c.nar");
        let mut rig = rig_with(
            startup_args(&[a.clone(), b.clone(), c.clone()], false, delete_source),
            None,
        );
        let order = steady_and_hand(&mut rig);
        let ends = vec![
            installed("supplement", &tmp.child("placed")),
            ArchiveEnd::Failed(FailureWord::Extraction),
            ArchiveEnd::Declined,
        ];
        let ((), events) = capture(|| done(&mut rig, order, ends));
        assert_eq!(a.is_file(), !delete_source, "入った写し: {delete_source}");
        assert!(b.is_file() && c.is_file(), "入らなかった写しは残る");
        assert_eq!(
            events_named(&events, "startup_source_deleted").len(),
            usize::from(delete_source),
            "{events:?}"
        );
        assert_eq!(events_named(&events, "startup_install_done").len(), 1);
    }

    let tmp = TempPath::under_target("desk-startup-delete-initial");
    let exe = tmp.child("exe");
    std::fs::create_dir_all(&exe).expect("exe の代わり");
    let initial = copy_at(&tmp, "exe/initial_install.nar");
    let mut rig = rig_with(InstallArgs::default(), Some(exe));
    let order = steady_and_hand(&mut rig);
    assert_eq!(order.origin, InstallOrigin::InitialInstall);
    done(
        &mut rig,
        order,
        vec![installed("supplement", &tmp.child("placed"))],
    );
    assert!(!initial.exists(), "初期インストールは語なしで消える");
}

/// 起動の依頼でない依頼（メニュー・台本・窓への投げ込み）が「入った」で終わっても、控えがあっても
/// その書庫は消えず控えも残る。初期インストールの依頼はその依頼の書庫だけを消し、控えは起動の引数の
/// 依頼の終わりで使われる（要件 3.1・3.6・4.3・9.3）。
#[test]
fn other_orders_never_delete_and_leave_the_followup() {
    let tmp = TempPath::under_target("desk-startup-others");
    let exe = tmp.child("exe");
    std::fs::create_dir_all(&exe).expect("exe の代わり");
    let initial = copy_at(&tmp, "exe/initial_install.nar");
    let a = copy_at(&tmp, "a.nar");
    let mut rig = rig_with(
        startup_args(std::slice::from_ref(&a), false, true),
        Some(exe),
    );
    let initial_order = steady_and_hand(&mut rig);
    assert_eq!(initial_order.origin, InstallOrigin::InitialInstall);
    let placed = || installed("supplement", &tmp.child("placed"));

    let mut others = Vec::new();
    for (origin, name) in [
        (InstallOrigin::Menu, "menu.nar"),
        (InstallOrigin::Script, "script.nar"),
        (InstallOrigin::WindowDrop, "drop.nar"),
    ] {
        let path = copy_at(&tmp, name);
        let order = InstallOrder {
            archives: vec![path.clone()],
            origin,
        };
        let ((), events) = capture(|| done(&mut rig, order, vec![placed()]));
        assert!(
            events_named(&events, "startup_install_done").is_empty(),
            "{origin:?}: {events:?}"
        );
        others.push(path);
    }
    assert!(
        others.iter().all(|p| p.is_file()),
        "起動の依頼でない書庫は消えない"
    );
    assert!(
        initial.is_file() && a.is_file(),
        "起動の依頼の書庫も消えない"
    );

    done(&mut rig, initial_order, vec![placed()]);
    assert!(!initial.exists(), "初期インストールの書庫は消える");
    assert!(
        a.is_file(),
        "初期インストールの依頼は起動の引数の書庫を消さない"
    );

    let args_order = rig.orders.try_recv().expect("次の依頼が渡る");
    assert_eq!(args_order.origin, InstallOrigin::StartupArgs);
    done(&mut rig, args_order, vec![placed()]);
    assert!(
        !a.exists(),
        "控えは残っていて、起動の引数の依頼の終わりで消す"
    );
    assert!(others.iter().all(|p| p.is_file()));
}

/// 控えを置いた後・終わりを受ける前に終了の後始末を呼ぶと warn 1 件で、写しは消えず切り替えも
/// 頼まない。その後に遅れて終わりが届いても warn は増えない。初期インストールの依頼の終わりが
/// 門の閉じた後に届くと、消さずに warn 1 件（要件 2.8・3.1・5.1・9.7）。
#[test]
fn ends_after_the_exit_neither_delete_nor_switch_and_warn_once() {
    let tmp = TempPath::under_target("desk-startup-closing");
    let root = root_with_ghost(&tmp, "A");
    root_with_ghost(&tmp, "N");
    let a = copy_at(&tmp, "a.nar");
    let mut rig = rig_with(startup_args(std::slice::from_ref(&a), true, true), None);
    rig.world.insert_resource(boot_context(&root, "A"));
    let kanade = rig.put_ghost(root.ghost_dir("A"));
    let order = steady_and_hand(&mut rig);

    let ((), at_close) = capture(|| {
        let _ = begin_close(&mut rig.world);
    });
    assert_eq!(dropped_origins(&at_close), ["StartupArgs"]);
    let ends = vec![installed("ghost", &root.ghost_dir("N"))];
    let ((), late) = capture(|| done(&mut rig, order, ends));
    assert!(
        dropped_origins(&late).is_empty(),
        "遅れた終わりで 2 件目は出ない: {late:?}"
    );
    assert!(events_named(&late, "startup_source_deleted").is_empty());
    assert!(a.is_file(), "写しは消えない");
    assert!(change_requests(&kanade).is_empty(), "切り替えを頼まない");
    assert!(rig.world.get_non_send::<SwitchInFlight>().is_none());

    let tmp = TempPath::under_target("desk-startup-closing-initial");
    let exe = tmp.child("exe");
    std::fs::create_dir_all(&exe).expect("exe の代わり");
    let initial = copy_at(&tmp, "exe/initial_install.nar");
    let mut rig = rig_with(InstallArgs::default(), Some(exe));
    let order = steady_and_hand(&mut rig);
    let ends = vec![installed("supplement", &tmp.child("placed"))];
    let ((), events) = capture(|| {
        let _ = begin_close(&mut rig.world);
        done(&mut rig, order, ends);
    });
    assert_eq!(dropped_origins(&events), ["InitialInstall"]);
    assert!(initial.is_file(), "初期インストールの写しも消えない");
}

/// 切り替えの受付の数は、よそのフォルダへの受理でだけ 1 増え、混み合い・同じフォルダ（大小違い）・
/// 見つからないでは増えない（要件 5.3）。
#[test]
fn switch_accept_count_grows_only_for_another_folder() {
    let tmp = TempPath::under_target("desk-startup-accept-count");
    let root = root_with_ghost(&tmp, "A");
    root_with_ghost(&tmp, "B");
    let mut rig = Rig::new();
    rig.world.insert_resource(boot_context(&root, "A"));
    let _kanade = rig.put_ghost(root.ghost_dir("A"));
    let folder = |name: &str| SwitchRequest {
        ghost: GhostSpec::Folder(name.to_owned()),
        raise_event: true,
        origin: ChangeOrigin::Manual,
        boot_event: None,
    };
    let ask = |rig: &mut Rig, name: &str| {
        let verdict = request_ghost_switch(&mut rig.world, folder(name));
        (verdict, accepted(&rig.world))
    };

    assert_eq!(accepted(&rig.world), 0);
    assert_eq!(ask(&mut rig, "B"), (SwitchVerdict::Accepted, 1));
    assert_eq!(ask(&mut rig, "B"), (SwitchVerdict::Busy, 1));
    rig.world.remove_non_send::<SwitchInFlight>();
    assert_eq!(ask(&mut rig, "a"), (SwitchVerdict::Accepted, 1));
    rig.world.remove_non_send::<SwitchInFlight>();
    assert_eq!(ask(&mut rig, "nobody"), (SwitchVerdict::NotFound, 1));

    // 直にパスで起こしたゴースト（フォルダ名なし）でも、同じ場所への受理は数えない
    rig.world
        .resource_mut::<crate::boot_config::BootContext>()
        .current
        .ghost
        .folder = None;
    assert_eq!(ask(&mut rig, "A"), (SwitchVerdict::Accepted, 1));
    rig.world.remove_non_send::<SwitchInFlight>();
    assert_eq!(ask(&mut rig, "B"), (SwitchVerdict::Accepted, 2));
}

/// `callghost` は最後に入ったゴーストへメニューと同じ形（知らせあり・`manual`）で切り替えを頼み、
/// 受付の数が 1 増える。その後に来たゴースト自身の切り替えは混み合い（Busy）で退く
/// （要件 5.1・5.3・9.5）。
#[test]
fn callghost_switch_makes_the_ghosts_own_switch_busy() {
    let tmp = TempPath::under_target("desk-startup-callghost");
    let root = root_with_ghost(&tmp, "A");
    root_with_ghost(&tmp, "N");
    let a = copy_at(&tmp, "a.nar");
    let mut rig = rig_with(startup_args(std::slice::from_ref(&a), true, true), None);
    rig.world.insert_resource(boot_context(&root, "A"));
    let kanade = rig.put_ghost(root.ghost_dir("A"));
    let order = steady_and_hand(&mut rig);

    let ends = vec![installed("ghost", &root.ghost_dir("N"))];
    let ((), events) = capture(|| done(&mut rig, order, ends));
    let requested = events_named(&events, "startup_callghost_requested");
    assert_eq!(requested.len(), 1, "{events:?}");
    assert_eq!(requested[0].field("verdict"), Some("Accepted"));
    assert_eq!(
        change_requests(&kanade),
        vec![("N".to_owned(), true, "manual")]
    );
    assert_eq!(accepted(&rig.world), 1);
    assert!(!a.exists(), "deletesource で写しが消える");

    let own = SwitchRequest {
        ghost: GhostSpec::Name("A".to_owned()),
        raise_event: true,
        origin: ChangeOrigin::Automatic,
        boot_event: None,
    };
    let (verdict, events) = capture(|| request_ghost_switch(&mut rig.world, own));
    assert_eq!(verdict, SwitchVerdict::Busy);
    assert_eq!(events_named(&events, "ghost_switch_busy").len(), 1);
    assert_eq!(accepted(&rig.world), 1);
}
