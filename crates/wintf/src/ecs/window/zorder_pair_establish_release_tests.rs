//! 確立系 [`establish_owner_links`](super::establish_owner_links) が、望む鎖が外させる組を
//! 張らないことのテスト（要件 1.3・4.4・6.1・6.6）。
//!
//! 起動時から逆転の計画（`s0,b0,b1,s1`）が公開されている状態でハンドルが付いた巡に、
//! スコープ 0 の組（バルーンのすぐ奥がキャラクターでない）は張らずに `ReleasedByChain` を
//! 記録し、スコープ 1 の組（連続する）は今と同じに張る——を同じ捕捉窓で固定する。
//! 張られたかは OS 側の事実（`GetWindow(GW_OWNER)`）で確かめる。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_OWNER, GetWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
};
use windows::core::w;

use super::{OwnerEstablishFailed, establish_owner_links};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{
    ChainPlan, KeepDirectlyAbove, OwnerLink, ReassertZOrder, WindowHandle, ZOrderChainPlan,
};

const DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";
const RELEASED_SKIP: &str = "[zorder-pair] skip";
const RELEASED_REASON: &str = "reason=ReleasedByChain";
const OWNER_ESTABLISHED_TAG: &str = "[zorder-pair] owner-established";

fn create_hidden_popup() -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで非表示のトップレベル窓を作る。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            w!("wintf-zorder-establish-release"),
            WINDOW_STYLE(WS_POPUP.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a hidden test window")
    }
}

fn owner_of(hwnd: HWND) -> Option<HWND> {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { GetWindow(hwnd, GW_OWNER) }.ok()
}

fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

fn released_skips(out: &str, entity: Entity) -> usize {
    let needle = format!("entity={entity:?} ");
    out.lines()
        .filter(|l| l.contains(RELEASED_SKIP) && l.contains(RELEASED_REASON))
        .filter(|l| l.contains(&needle))
        .count()
}

fn establish_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(establish_owner_links);
    schedule
}

/// 起動時から逆転の計画がある。ハンドルが付いた巡に、外す組は張らず見送りを記録し、
/// 失敗の印も再断行の要求も付けない。連続する組は今と同じに張る。
#[test]
fn a_pair_the_chain_releases_is_not_linked_when_its_handles_arrive() {
    let hwnds: Vec<HWND> = (0..4).map(|_| create_hidden_popup()).collect();
    let mut world = World::new();
    let s0 = spawn_window(&mut world, hwnds[0]);
    let b0 = spawn_window(&mut world, hwnds[1]);
    let s1 = spawn_window(&mut world, hwnds[2]);
    let b1 = spawn_window(&mut world, hwnds[3]);
    world.entity_mut(b0).insert(KeepDirectlyAbove { peer: s0 });
    world.entity_mut(b1).insert(KeepDirectlyAbove { peer: s1 });
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members: vec![s0, b0, b1, s1],
            cross_edges: Vec::new(),
            absent: Vec::new(),
        }),
        dirty: true,
    });

    let mut schedule = establish_schedule();
    let out = capture_under_filter(DIRECTIVES, || schedule.run(&mut world));

    // 外す組（スコープ 0）: 張らない・見送り 1 本・印も要求も無い。
    assert_eq!(released_skips(&out, b0), 1, "外す組の見送りは 1 本\n{out}");
    assert_eq!(owner_of(hwnds[1]), None, "外す組の owner は張られない");
    assert!(world.get::<OwnerLink>(b0).is_none());
    assert!(world.get::<OwnerEstablishFailed>(b0).is_none());
    assert!(world.get::<ReassertZOrder>(b0).is_none());

    // 連続する組（スコープ 1）: 今と同じに張る（要件 4.4）。
    assert_eq!(released_skips(&out, b1), 0, "連続する組は見送らない\n{out}");
    assert_eq!(
        out.lines()
            .filter(|l| l.contains(OWNER_ESTABLISHED_TAG))
            .count(),
        1,
        "張るのは連続する組の 1 本だけ\n{out}"
    );
    assert_eq!(owner_of(hwnds[3]), Some(hwnds[2]));
    assert!(world.get::<OwnerLink>(b1).is_some());
    assert!(world.get::<ReassertZOrder>(b1).is_some());

    for hwnd in hwnds {
        // SAFETY: Win32 境界。作った窓を破棄する。
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}

/// 鎖が解けた（`None`）後に現れたバルーンは、組の結び目を通常どおり張る（要件 6.6）。
#[test]
fn after_the_chain_is_cleared_a_new_balloon_is_linked_as_a_pair() {
    let character_hwnd = create_hidden_popup();
    let balloon_hwnd = create_hidden_popup();
    let mut world = World::new();
    world.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: true,
    });
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });

    let mut schedule = establish_schedule();
    let out = capture_under_filter(DIRECTIVES, || schedule.run(&mut world));

    assert_eq!(released_skips(&out, balloon), 0, "{out}");
    assert_eq!(owner_of(balloon_hwnd), Some(character_hwnd));
    assert!(world.get::<OwnerLink>(balloon).is_some());

    // SAFETY: Win32 境界。作った窓を破棄する。
    unsafe {
        let _ = DestroyWindow(balloon_hwnd);
        let _ = DestroyWindow(character_hwnd);
    }
}
