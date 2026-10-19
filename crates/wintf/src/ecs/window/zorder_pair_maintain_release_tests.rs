//! 維持系と沈降の観測が、望む鎖が外させる組に触れないことのテスト
//! （要件 4.4・5.1・5.2・5.4・5.5・6.3）。
//!
//! - 外す組に付いた再断行の要求（未適用＝バルーンの再表示の形・検証待ち）は、指令を積まず
//!   `ReleasedByChain` を記録して落とす。実窓の重なりは 1 つも動かない。
//! - 外す組に付いた活性化の目印は、測らずに外して見送りを記録し、指令も要求も生まない。
//! - 連続する組は、同じ鎖の下でも今と同じに是正・観測される。
//!
//! 「指令が出ない」は `guarded_set_window_pos` の実施記録を数えて測り、捕捉が生きている
//! ことを対照の指令 1 本で示す（兄弟の `zorder_pair_maintain_tests.rs` と同じ作法）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD,
    WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use windows::core::w;

use super::IssuedPairFix;
use crate::api::get_window_below;
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_pair::{ExpectedOrder, InsertSpec};
use crate::ecs::window::zorder_pair_sink::{SinkObservationPending, mark_pair_sink_observation};
use crate::ecs::window::{
    ChainPlan, KeepDirectlyAbove, ReassertZOrder, SetWindowPosCommand, WindowHandle,
    ZOrderChainPlan, ZOrderPairStrategy, apply_zorder_pair_maintenance,
};

const DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";
const SKIP_TAG: &str = "[zorder-pair] skip";
const RELEASED_REASON: &str = "reason=ReleasedByChain";
const FIX_TAG: &str = "[zorder-pair] fix";
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";
const SINK_OBSERVED_TAG: &str = "[zorder-pair] sink-observed";
const SET_WINDOW_POS_TAG: &str = "[guarded_set_window_pos] Calling SetWindowPos";

fn create_window(ex_style: u32, style: u32, parent: Option<HWND>) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで 0x0 のテスト窓を作る。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(ex_style),
            w!("Static"),
            w!("wintf-zorder-maintain-release"),
            WINDOW_STYLE(style),
            0,
            0,
            0,
            0,
            parent,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a test window")
    }
}

fn insert_after(hwnd: HWND, after: HWND) {
    // SAFETY: Win32 境界。自プロセス所有の窓の重なりだけを動かす。
    unsafe {
        SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .expect("SetWindowPos should reorder self-owned test windows");
    }
}

/// 可視の親の下に「バルーン／割り込み／キャラ」（手前 → 奥）の子窓 3 枚。
/// バルーンはキャラのすぐ手前に居ない（是正を要する並び）。
struct Trio {
    parent: HWND,
    character: HWND,
    intruder: HWND,
    balloon: HWND,
}

impl Trio {
    fn create() -> Self {
        let parent = create_window(WS_EX_TOOLWINDOW.0, WS_POPUP.0, None);
        // SAFETY: Win32 境界。活性化せずに表示する（子窓を可視と数えるため）。
        unsafe {
            let _ = ShowWindow(parent, SW_SHOWNA);
        }
        let child = || create_window(0, WS_CHILD.0 | WS_VISIBLE.0, Some(parent));
        let (character, intruder, balloon) = (child(), child(), child());
        insert_after(intruder, balloon);
        insert_after(character, intruder);
        assert_eq!(get_window_below(balloon).unwrap(), Some(intruder), "前提");
        Self {
            parent,
            character,
            intruder,
            balloon,
        }
    }
}

impl Drop for Trio {
    fn drop(&mut self) {
        // SAFETY: Win32 境界。作った親を破棄する（子窓は連鎖する）。
        unsafe {
            let _ = DestroyWindow(self.parent);
        }
    }
}

fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

/// 組を宣言し、(バルーン, キャラ) を返す。
fn pair(world: &mut World, balloon: HWND, character: HWND) -> (Entity, Entity) {
    let character = spawn_window(world, character);
    let balloon = spawn_window(world, balloon);
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
    (balloon, character)
}

fn publish(world: &mut World, members: Vec<Entity>) {
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members,
            cross_edges: Vec::new(),
            absent: Vec::new(),
        }),
        dirty: true,
    });
}

fn new_world() -> World {
    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    world
}

fn maintain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_pair_maintenance);
    schedule
}

/// 重なりも位置も動かさない対照の指令を 1 本出す（捕捉が生きていることの証）。
fn issue_control_command(hwnd: HWND) {
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        None,
    ));
    SetWindowPosCommand::flush();
}

fn count(out: &str, tag: &str) -> usize {
    out.lines().filter(|l| l.contains(tag)).count()
}

fn released_skips(out: &str, entity: Entity) -> usize {
    let needle = format!("entity={entity:?} ");
    out.lines()
        .filter(|l| l.contains(SKIP_TAG) && l.contains(RELEASED_REASON) && l.contains(&needle))
        .count()
}

/// 逆転の計画（キャラがバルーンより手前）の下で、バルーンの再表示が挿す未適用の要求は
/// 指令を積まずに見送りで落ち、実窓の重なりは動かない（要件 5.2・5.5・6.3）。
#[test]
fn a_reshow_request_on_a_released_pair_moves_nothing_and_is_dropped() {
    let trio = Trio::create();
    let mut world = new_world();
    let (balloon, character) = pair(&mut world, trio.balloon, trio.character);
    publish(&mut world, vec![character, balloon]);
    // 再表示の挿入元と同じ形（未適用の既定値）。
    world.entity_mut(balloon).insert(ReassertZOrder::default());

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(trio.balloon);
    });

    assert_eq!(released_skips(&out, balloon), 1, "見送りは 1 本\n{out}");
    assert_eq!(count(&out, FIX_TAG), 0, "是正しない\n{out}");
    assert_eq!(count(&out, SET_WINDOW_POS_TAG), 1, "対照の 1 本だけ\n{out}");
    assert!(world.get::<ReassertZOrder>(balloon).is_none());
    assert_eq!(
        get_window_below(trio.balloon).unwrap(),
        Some(trio.intruder),
        "重なりは 1 つも動かない"
    );
}

/// 外す組に付いた検証待ちの要求は、照合せずに見送りで落とし、持ち越しも外す（要件 5.5）。
#[test]
fn a_pending_verification_on_a_released_pair_is_dropped_without_checking() {
    let trio = Trio::create();
    let mut world = new_world();
    let (balloon, character) = pair(&mut world, trio.balloon, trio.character);
    // 非隣接の鎖（間に別の窓が挟まる）。
    let gap = world.spawn_empty().id();
    publish(&mut world, vec![balloon, gap, character]);
    world.entity_mut(balloon).insert((
        ReassertZOrder {
            pending_verify: Some(ExpectedOrder {
                above: trio.balloon,
                below: trio.character,
            }),
        },
        IssuedPairFix {
            insert_after: InsertSpec::After(trio.intruder),
        },
    ));

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(trio.balloon);
    });

    assert_eq!(released_skips(&out, balloon), 1, "見送りは 1 本\n{out}");
    assert_eq!(count(&out, VERIFY_FAILED_TAG), 0, "照合しない\n{out}");
    assert_eq!(count(&out, FIX_TAG), 0, "照合の記録も出さない\n{out}");
    assert_eq!(count(&out, SET_WINDOW_POS_TAG), 1, "対照の 1 本だけ\n{out}");
    assert!(world.get::<ReassertZOrder>(balloon).is_none());
    assert!(world.get::<IssuedPairFix>(balloon).is_none());
}

/// 同じ鎖の下でも、連続する組は今と同じに是正される（要件 4.4）。
#[test]
fn a_pair_the_chain_keeps_contiguous_is_still_fixed() {
    let trio = Trio::create();
    let mut world = new_world();
    let (balloon, character) = pair(&mut world, trio.balloon, trio.character);
    publish(&mut world, vec![balloon, character]);
    world.entity_mut(balloon).insert(ReassertZOrder::default());

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        schedule.run(&mut world);
    });

    assert_eq!(released_skips(&out, balloon), 0, "{out}");
    assert_eq!(count(&out, VERIFY_FAILED_TAG), 0, "{out}");
    assert_eq!(count(&out, SET_WINDOW_POS_TAG), 1, "是正の 1 本\n{out}");
    assert_eq!(
        get_window_below(trio.balloon).unwrap(),
        Some(trio.character)
    );
    assert!(world.get::<ReassertZOrder>(balloon).is_none());
}

/// 活性化の目印（押す・他のアプリを選ぶ）が外す組に付いても、沈降の観測は測らず、
/// 指令も要求も生まない。連続する組の目印は今と同じに測られる（要件 5.1・5.4・4.4）。
#[test]
fn a_sink_mark_on_a_released_pair_is_dropped_unmeasured() {
    let released_trio = Trio::create();
    let kept_trio = Trio::create();
    let mut world = new_world();
    let (b0, s0) = pair(&mut world, released_trio.balloon, released_trio.character);
    let (b1, s1) = pair(&mut world, kept_trio.balloon, kept_trio.character);
    publish(&mut world, vec![s0, b0, b1, s1]);
    // 非活性化の枝と同じ入口で目印を付ける（キャラ窓が非活性化された形）。
    assert_eq!(mark_pair_sink_observation(&mut world, s0), Some(b0));
    assert_eq!(mark_pair_sink_observation(&mut world, s1), Some(b1));

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(released_trio.balloon);
    });

    assert_eq!(released_skips(&out, b0), 1, "外す組は見送り 1 本\n{out}");
    assert_eq!(released_skips(&out, b1), 0, "{out}");
    assert_eq!(
        count(&out, SINK_OBSERVED_TAG),
        1,
        "測るのは連続する組だけ\n{out}"
    );
    assert_eq!(count(&out, SET_WINDOW_POS_TAG), 1, "対照の 1 本だけ\n{out}");
    for balloon in [b0, b1] {
        assert!(world.get::<SinkObservationPending>(balloon).is_none());
        assert!(world.get::<ReassertZOrder>(balloon).is_none());
    }
}
