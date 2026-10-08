//! 選ぶ画面を起こす動作と「出ている」旗の決定論テスト（要件 1.3・1.4・11.6・design「Testing
//! Strategy / 窓口と入口」のメニューの箇条）。
//!
//! 確かめること: 選ぶ画面は `install-pick` という名のスレッドで出て、出ている間は「インストール…」が
//! 選べず 2 度目の起動は無視されること・スレッドが終わると旗が降りて選べるようになること・抑止の
//! ときはスレッドが起きず `warn!(install_pick_suppressed)` が 1 件で依頼が 0 件であること・取り消しは
//! `info!`、出せなければ `error!` でどちらも要求 0 件であること・メニューの側の依頼が台本の側の
//! 依頼と出どころだけ違う同じ依頼になること。
//!
//! 本物の画面は出さない: 選ぶ画面の代わり（閉包）を差し込む。背景のスレッドは起こさない（窓口の
//! 依頼の送出端を受信端に差し替える）。別のスレッドは `recv_timeout` で待つ（固まらない）。

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::*;
use crate::emo2_boot::spine::{Progress, wait_until};
use crate::install::judge::{ScriptRequest, script_request};
use crate::install::{InstallOrigin, register};
use crate::menu::{Frame, MenuContext, MenuWiring, install_frame, wire_menu};

/// 絶対パスの書庫（fs は読まない＝在る必要はない）。
const ABSOLUTE: &str = r"C:\areka-install-pick-test\in\hana.nar";

/// 別のスレッドを待つ上限（固まったときに赤で止めるためだけの値）。
const BOUND: Duration = Duration::from_secs(10);

/// 窓口とメニュー（「インストール」枠を登記済み）を据えた World と、背景のスレッドの代わり。
fn rig() -> (World, Receiver<InstallOrder>) {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register(&mut world);
    let (worker, orders) = mpsc::channel();
    world.non_send_mut::<InstallDesk>().worker = Some(worker);
    wire_menu(&mut world, mpsc::channel().0);
    install_frame::register(&mut world);
    (world, orders)
}

/// メニューを出したときの「インストール」枠が選べるか（登記が無ければ None）。
fn item_enabled(world: &World) -> Option<bool> {
    world
        .non_send::<MenuWiring>()
        .registry
        .snapshot(world, &MenuContext { scope: 0 })
        .into_iter()
        .find_map(|(frame, item)| (frame == Frame::Install).then_some(item.enabled))
}

/// 選ぶ画面のスレッドの終わりを上限つきで待つ。
/// 見張りのスレッドは生まず、body の終わり（`is_finished`）を目印に待ちの芯で待ってから同じスレッドで
/// `join` する（負荷の下で見張りのスレッドが始まれず文言の無い赤になった・areka-P0-ghost-session-test-load-flake）。
fn join_bounded(handle: JoinHandle<()>) {
    let finished = || u64::from(handle.is_finished());
    if let Err(failure) = wait_until(
        "選ぶ画面のスレッドの終わり",
        Progress::Count(&finished),
        || handle.is_finished(),
    ) {
        panic!("{failure}");
    }
    assert!(
        handle.join().is_ok(),
        "選ぶ画面のスレッドが終わらないか panic した"
    );
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn warns_and_errors(events: &[CapturedEvent]) -> Vec<(Level, Option<&str>)> {
    events
        .iter()
        .filter(|e| e.level <= Level::WARN)
        .map(|e| (e.level, e.field_str("event")))
        .collect()
}

// ---------------------------------------------------------------- 出どころだけが違う

/// メニューの側（選ぶ画面のスレッドが選ばれたパスを生の要求として流す）と台本の側（受け口と同じ
/// `script_request` を通した出どころ「台本」の生の要求）は、取り出しの後に出どころだけが違う同じ
/// 依頼になる（要件 1.1・9.3・11.6）。
#[test]
fn menu_order_is_the_script_order_with_only_the_origin_changed() {
    let (mut world, orders) = rig();
    // 待ち行列に残して読む（背景のスレッドの代わりへ渡さない）。
    world.non_send_mut::<InstallDesk>().busy = true;

    let handle = start_pick(&mut world, false, || Ok(Some(PathBuf::from(ABSOLUTE))))
        .expect("選ぶ画面のスレッドが起きる");
    join_bounded(handle);
    let Ok(ScriptRequest::Path(script_path)) = script_request(&["path", ABSOLUTE]) else {
        panic!("絶対パスは path の腕で通る");
    };
    raw_sender(&world)
        .send(RawInstallRequest {
            path: script_path,
            origin: InstallOrigin::Script,
        })
        .expect("窓口の受信端は生きている");
    drain(&mut world);

    let queue: Vec<InstallOrder> = world
        .non_send::<InstallDesk>()
        .queue
        .iter()
        .cloned()
        .collect();
    let [menu, script] = queue.as_slice() else {
        panic!("依頼はメニューと台本の 2 件のはず: {queue:?}");
    };
    assert_eq!(
        (menu.origin, script.origin),
        (InstallOrigin::Menu, InstallOrigin::Script)
    );
    assert_eq!(
        *menu,
        InstallOrder {
            origin: InstallOrigin::Menu,
            ..script.clone()
        },
        "出どころのほかは同じ"
    );
    assert_eq!(menu.archives, vec![PathBuf::from(ABSOLUTE)], "書庫 1 本");
    assert!(
        orders.try_recv().is_err(),
        "背景のスレッドの代わりへは渡っていない"
    );
}

// ---------------------------------------------------------------- 出ている間

/// 選ぶ画面は `install-pick` のスレッドで出て、出ている間は「インストール…」が選べず、2 度目の
/// 起動は `debug!` を残して無視される。スレッドが終わると旗が降り、また選べる（要件 1.3）。
#[test]
fn picking_disables_the_item_until_the_thread_ends() {
    let (mut world, _orders) = rig();
    let before = item_enabled(&world);
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let handle = start_pick(&mut world, false, move || {
        let _ = entered_tx.send(thread::current().name().map(str::to_owned));
        let _ = release_rx.recv_timeout(BOUND);
        Ok(None)
    })
    .expect("選ぶ画面のスレッドが起きる");
    let thread_name = entered
        .recv_timeout(BOUND)
        .expect("選ぶ画面の代わりが呼ばれる");

    let during = (item_enabled(&world), can_pick(&world));
    let (second_called_tx, second_called) = mpsc::channel();
    let (second, events) = capture(|| {
        start_pick(&mut world, false, move || {
            let _ = second_called_tx.send(());
            Ok(None)
        })
    });

    release.send(()).expect("選ぶ画面の代わりは待っている");
    join_bounded(handle);
    let after = (item_enabled(&world), can_pick(&world));

    assert_eq!(
        (
            before,
            thread_name.as_deref(),
            during,
            second.is_none(),
            second_called.try_recv().is_err(),
            events_named(&events, "install_pick_busy").len(),
            after,
        ),
        (
            Some(true),
            Some("install-pick"),
            (Some(false), false),
            true,
            true,
            1,
            (Some(true), true),
        ),
        "出る前は選べる・スレッドの名・出ている間は選べない・2 度目は起きない・代わりは呼ばれない・\
         無視の記録 1 件・終わった後は選べる: {events:?}"
    );
}

// ---------------------------------------------------------------- 抑止

/// 抑止のときは選ぶ画面のスレッドを起こさず、`warn!(install_pick_suppressed)` を 1 件残して取り消しと
/// 同じに扱う（依頼 0 件・旗は立たない）。
#[test]
fn suppressed_pick_starts_no_thread_and_queues_nothing() {
    let (mut world, orders) = rig();
    let (called_tx, called) = mpsc::channel();
    let ((handle, queued), events) = capture(|| {
        let handle = start_pick(&mut world, true, move || {
            let _ = called_tx.send(());
            Ok(Some(PathBuf::from(ABSOLUTE)))
        });
        drain(&mut world);
        let queued = world.non_send::<InstallDesk>().queue.len();
        (handle, queued)
    });

    assert_eq!(
        (
            handle.is_none(),
            called.try_recv().is_err(),
            warns_and_errors(&events),
            queued,
            orders.try_recv().is_err(),
            events_named(&events, "install_order_queued").len(),
            can_pick(&world),
        ),
        (
            true,
            true,
            vec![(Level::WARN, Some("install_pick_suppressed"))],
            0,
            true,
            0,
            true,
        ),
        "スレッドなし・画面の代わりは呼ばれない・警告 1 件・依頼 0 件・旗は立たない: {events:?}"
    );
}

// ---------------------------------------------------------------- 選ぶ画面の結果

/// 取り消しは `info!(install_pick_cancelled)`、出せなければ `error!(install_pick_failed)` で、
/// どちらも生の要求 0 件。選ばれたら出どころ「メニュー」の生の要求 1 件（要件 1.3・1.4）。
#[test]
fn pick_results_are_logged_and_only_a_choice_is_sent() {
    let cases: [(
        Result<Option<PathBuf>, PickError>,
        Vec<RawInstallRequest>,
        &str,
        Level,
    ); 3] = [
        (Ok(None), vec![], "install_pick_cancelled", Level::INFO),
        (
            Err(PickError::Dialog(0xFFFF)),
            vec![],
            "install_pick_failed",
            Level::ERROR,
        ),
        (
            Ok(Some(PathBuf::from(ABSOLUTE))),
            vec![RawInstallRequest {
                path: PathBuf::from(ABSOLUTE),
                origin: InstallOrigin::Menu,
            }],
            "",
            Level::INFO,
        ),
    ];
    for (result, expected, event, level) in cases {
        let label = format!("{result:?}");
        let (tx, rx) = mpsc::channel();
        let ((), events) = capture(|| send_pick(result, &tx));
        let logged: Vec<Level> = events_named(&events, event)
            .iter()
            .map(|e| e.level)
            .collect();
        assert_eq!(rx.try_iter().collect::<Vec<_>>(), expected, "{label}");
        if event.is_empty() {
            assert!(warns_and_errors(&events).is_empty(), "{label}: {events:?}");
        } else {
            assert_eq!(logged, vec![level], "{label}: {events:?}");
        }
    }
}
