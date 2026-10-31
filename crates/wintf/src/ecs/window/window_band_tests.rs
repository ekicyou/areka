//! 帯の働きの純関数の全分岐（要件 2.4・2.5・2.10・7.3）と、偽の OS で回す確定段の帯の系の
//! 分岐（要件 2.1・2.3・2.6・2.9・3.1・6.1・6.3・7.3）。

use super::*;
use bevy_ecs::schedule::Schedule;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS;

fn e(index: u32) -> Entity {
    Entity::from_raw_u32(index).expect("有効な entity の番号")
}

fn h(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

/// 順位どおりに奥から手前へ並ぶ（0 が最上なので、順位の大きい窓が先）。
#[test]
fn back_to_front_orders_by_rank_from_back_to_front() {
    let windows = [(e(1), h(0x10)), (e(2), h(0x20)), (e(3), h(0x30))];
    let rank = |w: HWND| match w.0 as isize {
        0x10 => Some(1),
        0x20 => Some(0),
        0x30 => Some(5),
        _ => unreachable!(),
    };
    assert_eq!(
        back_to_front(&windows, rank),
        vec![(e(3), h(0x30)), (e(1), h(0x10)), (e(2), h(0x20))]
    );
}

/// 測れない窓は奥の端へ置き、測れない窓どうしは元の順を保つ。
#[test]
fn back_to_front_puts_unmeasured_windows_at_the_back_in_original_order() {
    let windows = [
        (e(1), h(0x10)),
        (e(2), h(0x20)),
        (e(3), h(0x30)),
        (e(4), h(0x40)),
    ];
    let rank = |w: HWND| match w.0 as isize {
        0x10 => Some(0),
        0x30 => Some(2),
        _ => None,
    };
    assert_eq!(
        back_to_front(&windows, rank),
        vec![
            (e(2), h(0x20)),
            (e(4), h(0x40)),
            (e(3), h(0x30)),
            (e(1), h(0x10)),
        ]
    );
}

/// 同じ順位の窓どうしは元の順を保つ（並べ替えが安定）。空の列は空のまま。
#[test]
fn back_to_front_keeps_original_order_for_equal_ranks_and_empty_input() {
    let windows = [(e(1), h(0x10)), (e(2), h(0x20))];
    assert_eq!(back_to_front(&windows, |_| Some(3)), windows.to_vec());
    assert!(back_to_front(&[], |_| Some(0)).is_empty());
}

/// 3 つの指令の挿入位置（手前へ出すは帯の最上へ上げる側。帯の外だった窓は後で戻す）。
#[test]
fn band_insert_after_maps_each_op_to_its_insert_position() {
    assert_eq!(band_insert_after(BandOp::Enter), HWND_TOPMOST);
    assert_eq!(band_insert_after(BandOp::Leave), HWND_NOTOPMOST);
    assert_eq!(band_insert_after(BandOp::Raise), HWND_TOPMOST);
}

/// 入る・出るは帯の印を確かめ、手前へ出すは確かめない。
#[test]
fn expected_band_checks_enter_and_leave_but_not_raise() {
    assert_eq!(expected_band(BandOp::Enter), Some(true));
    assert_eq!(expected_band(BandOp::Leave), Some(false));
    assert_eq!(expected_band(BandOp::Raise), None);
}

/// 列は積まれた順に指令と窓の列を持つ。
#[test]
fn window_band_queue_keeps_pushed_order() {
    let mut queue = WindowBandQueue::default();
    queue.push(BandOp::Enter, vec![e(1), e(2)]);
    queue.push(BandOp::Raise, vec![e(2)]);
    assert_eq!(
        queue.ops,
        vec![
            (BandOp::Enter, vec![e(1), e(2)]),
            (BandOp::Raise, vec![e(2)])
        ]
    );
}

// ============================================================================
// 偽の OS（`zorder_chain_apply_test_support.rs` の `double` と同じ流儀）
// ============================================================================

pub(super) mod double {
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS;
    use windows::core::HRESULT;

    /// 偽の画面と、走行中に書かれた窓の記録。
    #[derive(Default)]
    pub(crate) struct Script {
        /// 今の重なり（最上 → 奥）。
        pub z: Vec<isize>,
        /// 帯の印（載っていない窓は読めない＝`None`）。
        pub in_band: HashMap<isize, bool>,
        /// 書き込みを失敗させる窓。
        pub write_fails: HashSet<isize>,
        /// 書き込みは通るが帯が変わらない窓。
        pub stuck: HashSet<isize>,
        /// 書いた窓の列（窓, 挿入位置, フラグ）（順序込み）。
        pub writes: Vec<(isize, isize, SET_WINDOW_POS_FLAGS)>,
    }

    thread_local! {
        static SCRIPT: RefCell<Option<Script>> = const { RefCell::new(None) };
    }

    /// 偽の画面を据えて `body` を走らせ、走行後の台本を返す。
    pub(crate) fn with_script<R>(script: Script, body: impl FnOnce() -> R) -> (R, Script) {
        struct Restore;
        impl Drop for Restore {
            fn drop(&mut self) {
                SCRIPT.with(|cell| *cell.borrow_mut() = None);
            }
        }
        SCRIPT.with(|cell| *cell.borrow_mut() = Some(script));
        let _restore = Restore;
        let out = body();
        let used = SCRIPT.with(|cell| cell.borrow_mut().take());
        (out, used.expect("台本は走行中に落ちない"))
    }

    fn with<R>(f: impl FnOnce(&mut Script) -> R) -> Option<R> {
        SCRIPT.with(|cell| cell.borrow_mut().as_mut().map(f))
    }

    fn hwnd(raw: isize) -> HWND {
        HWND(raw as *mut core::ffi::c_void)
    }

    pub(in super::super) fn top() -> Option<Option<HWND>> {
        with(|s| s.z.first().map(|&w| hwnd(w)))
    }

    pub(in super::super) fn next(of: HWND) -> Option<Option<HWND>> {
        with(|s| {
            let at = s.z.iter().position(|&w| w == of.0 as isize)?;
            s.z.get(at + 1).map(|&w| hwnd(w))
        })
    }

    pub(in super::super) fn in_band(of: HWND) -> Option<Option<bool>> {
        with(|s| s.in_band.get(&(of.0 as isize)).copied())
    }

    pub(in super::super) fn write(
        of: HWND,
        insert_after: HWND,
        flags: SET_WINDOW_POS_FLAGS,
    ) -> Option<windows::core::Result<()>> {
        with(|s| {
            let key = of.0 as isize;
            let after = insert_after.0 as isize;
            s.writes.push((key, after, flags));
            if s.write_fails.contains(&key) {
                return Err(windows::core::Error::from(HRESULT(0x8007_0578u32 as i32)));
            }
            if !s.stuck.contains(&key) {
                // HWND_TOPMOST = -1 は帯へ・HWND_NOTOPMOST = -2 は帯の外へ・HWND_TOP は帯を変えない。
                match after {
                    -1 => {
                        s.in_band.insert(key, true);
                    }
                    -2 => {
                        s.in_band.insert(key, false);
                    }
                    _ => {}
                }
            }
            Ok(())
        })
    }
}

use double::{Script, with_script};

// 持ち主の並べ直しを止める（SWP_NOOWNERZORDER）: 実機（task 7.1 の 2 回目）で持ち主の書き込みが
// OS の持ち主と持たれ窓の並べ直しに落とされ、持たれ窓だけが帯へ上がった。
const FLAGS: SET_WINDOW_POS_FLAGS =
    SET_WINDOW_POS_FLAGS(SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOACTIVATE.0 | SWP_NOOWNERZORDER.0);
const TOPMOST: isize = -1;
const NOTOPMOST: isize = -2;

fn spawn_window(world: &mut World, raw: isize) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd: h(raw),
            instance: HINSTANCE::default(),
        })
        .id()
}

fn band_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(apply_window_band);
    schedule
}

/// 偽の画面で 1 巡回し、記録と書いた列を返す。
fn run_once(
    world: &mut World,
    schedule: &mut Schedule,
    script: Script,
) -> (Vec<log_capture_kit::CapturedEvent>, Script) {
    let ((_, events), used) =
        with_script(script, || log_capture_kit::capture(|| schedule.run(world)));
    (events, used)
}

fn queue(world: &mut World, op: BandOp, windows: Vec<Entity>) {
    world.resource_mut::<WindowBandQueue>().push(op, windows);
}

fn new_world() -> World {
    let mut world = World::new();
    world.insert_resource(WindowBandQueue::default());
    world
}

fn with_level<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    level: tracing::Level,
    message: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == level && e.message().contains(message))
        .collect()
}

fn warns_with<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    message: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    with_level(events, tracing::Level::WARN, message)
}

fn applied_line(events: &[log_capture_kit::CapturedEvent]) -> &log_capture_kit::CapturedEvent {
    let lines = with_level(events, tracing::Level::DEBUG, "[window_band] applied");
    assert_eq!(lines.len(), 1, "{events:?}");
    lines[0]
}

/// 今の重なりで奥の窓から手前の窓の順に、位置・大きさ・活性と持ち主の並べ直しを変えないフラグで書く。
/// 当てた結果（指令・当てた数・飛ばした数）が `debug!` に残る。
#[test]
fn enter_writes_from_back_to_front_with_no_move_size_activate_owner_zorder() {
    let mut world = new_world();
    let a = spawn_window(&mut world, 0x10);
    let b = spawn_window(&mut world, 0x20);
    let c = spawn_window(&mut world, 0x30);
    world.clear_trackers();
    queue(&mut world, BandOp::Enter, vec![a, b, c]);
    let script = Script {
        z: vec![0x99, 0x30, 0x10, 0x77, 0x20],
        in_band: [(0x10, false), (0x20, false), (0x30, false)].into(),
        ..Script::default()
    };
    let (events, used) = run_once(&mut world, &mut band_schedule(), script);
    assert_eq!(
        used.writes,
        vec![
            (0x20, TOPMOST, FLAGS),
            (0x10, TOPMOST, FLAGS),
            (0x30, TOPMOST, FLAGS)
        ]
    );
    let applied = applied_line(&events);
    assert_eq!(applied.field("op"), Some("Enter"));
    assert_eq!(applied.field("applied"), Some("3"));
    assert_eq!(applied.field("skipped"), Some("0"));
    assert!(
        events.iter().all(|e| e.level != tracing::Level::WARN),
        "{events:?}"
    );
}

/// 列の指令は積まれた順に当たり、巡の終わりに列は空になる。
#[test]
fn queued_ops_apply_in_pushed_order_and_queue_ends_empty() {
    let mut world = new_world();
    let a = spawn_window(&mut world, 0x10);
    let b = spawn_window(&mut world, 0x20);
    world.clear_trackers();
    queue(&mut world, BandOp::Leave, vec![a]);
    queue(&mut world, BandOp::Raise, vec![b]);
    let script = Script {
        z: vec![0x10, 0x20],
        in_band: [(0x10, true), (0x20, false)].into(),
        ..Script::default()
    };
    let (_, used) = run_once(&mut world, &mut band_schedule(), script);
    assert_eq!(
        used.writes,
        vec![
            (0x10, NOTOPMOST, FLAGS),
            (0x20, TOPMOST, FLAGS),
            (0x20, NOTOPMOST, FLAGS)
        ]
    );
    assert!(world.resource::<WindowBandQueue>().ops.is_empty());
}

/// 居ない entity と HWND の無い窓は飛ばして `debug!` に残し、残りの窓を書く。
#[test]
fn missing_entity_and_window_without_hwnd_are_skipped() {
    let mut world = new_world();
    let a = spawn_window(&mut world, 0x10);
    let gone = world.spawn_empty().id();
    world.despawn(gone);
    let no_hwnd = world.spawn_empty().id();
    world.clear_trackers();
    queue(&mut world, BandOp::Enter, vec![gone, a, no_hwnd]);
    let script = Script {
        z: vec![0x10],
        in_band: [(0x10, false)].into(),
        ..Script::default()
    };
    let (events, used) = run_once(&mut world, &mut band_schedule(), script);
    assert_eq!(used.writes, vec![(0x10, TOPMOST, FLAGS)]);
    let debug = tracing::Level::DEBUG;
    assert_eq!(with_level(&events, debug, "entity not found").len(), 1);
    assert_eq!(with_level(&events, debug, "no HWND yet").len(), 1);
    let applied = applied_line(&events);
    assert_eq!(applied.field("applied"), Some("1"));
    assert_eq!(applied.field("skipped"), Some("2"));
}

/// 順位が測れない窓は奥の端（最初に書く）へ置き、`warn!` に残す。
#[test]
fn unmeasured_window_goes_to_the_back_with_warn() {
    let mut world = new_world();
    let a = spawn_window(&mut world, 0x10);
    let b = spawn_window(&mut world, 0x20);
    world.clear_trackers();
    queue(&mut world, BandOp::Raise, vec![a, b]);
    let script = Script {
        z: vec![0x10],
        ..Script::default()
    };
    let (events, used) = run_once(&mut world, &mut band_schedule(), script);
    // 帯の印が読めない窓は帯の外とみなし、上げてから戻す（2 回とも奥から）。
    assert_eq!(
        used.writes,
        vec![
            (0x20, TOPMOST, FLAGS),
            (0x10, TOPMOST, FLAGS),
            (0x20, NOTOPMOST, FLAGS),
            (0x10, NOTOPMOST, FLAGS)
        ]
    );
    let warns = warns_with(&events, "rank not measured");
    assert_eq!(warns.len(), 1, "{events:?}");
    assert_eq!(warns[0].field_str("hwnd"), Some("0x20"));
}

/// 1 枚の書き込みの失敗の後も残りの窓を書き、失敗を `warn!` に残す。
#[test]
fn write_failure_is_warned_and_remaining_windows_are_written() {
    let mut world = new_world();
    let a = spawn_window(&mut world, 0x10);
    let b = spawn_window(&mut world, 0x20);
    let c = spawn_window(&mut world, 0x30);
    world.clear_trackers();
    queue(&mut world, BandOp::Raise, vec![a, b, c]);
    let script = Script {
        z: vec![0x10, 0x20, 0x30],
        write_fails: [0x20].into(),
        ..Script::default()
    };
    let (events, used) = run_once(&mut world, &mut band_schedule(), script);
    assert_eq!(
        used.writes,
        vec![
            (0x30, TOPMOST, FLAGS),
            (0x20, TOPMOST, FLAGS),
            (0x10, TOPMOST, FLAGS),
            (0x30, NOTOPMOST, FLAGS),
            (0x20, NOTOPMOST, FLAGS),
            (0x10, NOTOPMOST, FLAGS)
        ]
    );
    // 失敗した窓は 2 回とも書いて 2 回とも残す。当てた数は全部の書き込みが通った窓の数。
    let warns = warns_with(&events, "SetWindowPos failed");
    assert_eq!(warns.len(), 2, "{events:?}");
    assert!(warns.iter().all(|w| w.field_str("hwnd") == Some("0x20")));
    assert_eq!(applied_line(&events).field("applied"), Some("2"));
}

/// 入る・出るの後に帯の印を読み直し、合わない窓だけを `warn!` に残す。手前へ出すは読み直さない。
#[test]
fn band_mismatch_after_enter_and_leave_is_warned_but_raise_is_not_checked() {
    for (op, before) in [(BandOp::Enter, false), (BandOp::Leave, true)] {
        let mut world = new_world();
        let a = spawn_window(&mut world, 0x10);
        let b = spawn_window(&mut world, 0x20);
        world.clear_trackers();
        queue(&mut world, op, vec![a, b]);
        let script = Script {
            z: vec![0x10, 0x20],
            in_band: [(0x10, before), (0x20, before)].into(),
            stuck: [0x10].into(),
            ..Script::default()
        };
        let (events, _) = run_once(&mut world, &mut band_schedule(), script);
        let warns = warns_with(&events, "band mismatch");
        assert_eq!(warns.len(), 1, "{op:?}: {events:?}");
        assert_eq!(warns[0].field_str("hwnd"), Some("0x10"));
        assert_eq!(
            warns[0].field("expected"),
            Some(if before { "false" } else { "true" })
        );
    }

    let mut world = new_world();
    let a = spawn_window(&mut world, 0x10);
    world.clear_trackers();
    queue(&mut world, BandOp::Raise, vec![a]);
    let script = Script {
        z: vec![0x10],
        stuck: [0x10].into(),
        ..Script::default()
    };
    let (events, _) = run_once(&mut world, &mut band_schedule(), script);
    assert!(
        warns_with(&events, "band mismatch").is_empty(),
        "{events:?}"
    );
}

/// 手前へ出すは、全部の窓を奥から帯の最上へ上げ、当てる前に帯の外だった窓だけを奥から
/// 帯の外の最上へ戻す（案 A・裏のプロセスでも入力先の窓の手前へ出る）。帯の中の窓は 1 回だけ。
/// 戻した窓は帯の外のまま、帯の中の窓は帯のままで、帯の食い違いの `warn!` は出ない。
#[test]
fn raise_lifts_all_and_returns_only_windows_that_were_outside_the_band() {
    let mut world = new_world();
    let a = spawn_window(&mut world, 0x10);
    let b = spawn_window(&mut world, 0x20);
    let c = spawn_window(&mut world, 0x30);
    world.clear_trackers();
    queue(&mut world, BandOp::Raise, vec![a, b, c]);
    let script = Script {
        z: vec![0x10, 0x20, 0x30],
        in_band: [(0x10, true), (0x20, false), (0x30, false)].into(),
        ..Script::default()
    };
    let (events, used) = run_once(&mut world, &mut band_schedule(), script);
    assert_eq!(
        used.writes,
        vec![
            (0x30, TOPMOST, FLAGS),
            (0x20, TOPMOST, FLAGS),
            (0x10, TOPMOST, FLAGS),
            (0x30, NOTOPMOST, FLAGS),
            (0x20, NOTOPMOST, FLAGS)
        ]
    );
    assert_eq!(
        [0x10, 0x20, 0x30].map(|k| used.in_band.get(&k).copied()),
        [Some(true), Some(false), Some(false)]
    );
    assert!(
        warns_with(&events, "band mismatch").is_empty(),
        "{events:?}"
    );
    let applied = applied_line(&events);
    assert_eq!(applied.field("op"), Some("Raise"));
    assert_eq!(applied.field("applied"), Some("3"));
}

/// この巡に HWND が付いた印のある窓が帯の外なら入るを当て、帯に居れば書かない。
/// 印の無い窓と、前の巡に HWND が付いた窓には書かない。
#[test]
fn stay_on_top_window_that_got_hwnd_this_frame_enters_band_if_outside() {
    let mut world = new_world();
    let mut schedule = band_schedule();
    for raw in [0x10, 0x20] {
        let entity = spawn_window(&mut world, raw);
        world.entity_mut(entity).insert(StayOnTop);
    }
    spawn_window(&mut world, 0x30);
    let script = Script {
        z: vec![0x10, 0x20, 0x30],
        in_band: [(0x10, false), (0x20, true), (0x30, false)].into(),
        ..Script::default()
    };
    let (_, used) = run_once(&mut world, &mut schedule, script);
    assert_eq!(used.writes, vec![(0x10, TOPMOST, FLAGS)]);

    // 次の巡は HWND が付いた巡ではないので、帯の外でも書かない。
    let script = Script {
        z: vec![0x10, 0x20, 0x30],
        in_band: [(0x10, false), (0x20, false), (0x30, false)].into(),
        ..Script::default()
    };
    let (_, used) = run_once(&mut world, &mut schedule, script);
    assert!(used.writes.is_empty(), "{:?}", used.writes);
}

/// 最上から 4096 枚を超えた先に居る窓は測れない窓として奥の端へ置き、`warn!` に残す。
/// 4096 枚目（順位 4095）までに居る窓は測れる。
#[test]
fn window_beyond_rank_walk_limit_is_unmeasured_and_goes_to_the_back() {
    let mut world = new_world();
    let last_in = spawn_window(&mut world, 0x10);
    let beyond = spawn_window(&mut world, 0x20);
    world.clear_trackers();
    queue(&mut world, BandOp::Raise, vec![beyond, last_in]);
    // 0x10 は順位 4095（上限の内の最後）・0x20 は順位 4096（上限の外）。
    let mut z: Vec<isize> = (0..4095).map(|i| 0x10000 + i).collect();
    z.push(0x10);
    z.push(0x20);
    let script = Script {
        z,
        ..Script::default()
    };
    let (events, used) = run_once(&mut world, &mut band_schedule(), script);
    assert_eq!(
        used.writes,
        vec![
            (0x20, TOPMOST, FLAGS),
            (0x10, TOPMOST, FLAGS),
            (0x20, NOTOPMOST, FLAGS),
            (0x10, NOTOPMOST, FLAGS)
        ]
    );
    let warns = warns_with(&events, "rank not measured");
    assert_eq!(warns.len(), 1, "{events:?}");
    assert_eq!(warns[0].field_str("hwnd"), Some("0x20"));
}

/// HWND が付いた巡の保険で帯の印が読めない窓は、帯の外とみなして入るを当てる。
#[test]
fn stay_on_top_window_with_unreadable_band_mark_enters_band() {
    let mut world = new_world();
    let entity = spawn_window(&mut world, 0x10);
    world.entity_mut(entity).insert(StayOnTop);
    let script = Script {
        z: vec![0x10],
        ..Script::default()
    };
    let (_, used) = run_once(&mut world, &mut band_schedule(), script);
    assert_eq!(used.writes, vec![(0x10, TOPMOST, FLAGS)]);
}
