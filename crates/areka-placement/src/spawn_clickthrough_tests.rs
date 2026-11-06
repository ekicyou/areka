use bevy_ecs::prelude::*;
use wintf::ecs::Window;

use super::spawn_ghost_windows;
use super::test_support::{
    fake_window_handle, ghost_window_entities, titles, two_scope_placements,
};

// -------------------------------------------------------------------------
// stand-in 即終了（`on_ghost_pressed`）の退役（areka-P0-input-events task 3.2・6.1）
//
// 旧テスト `double_click_left_despawns_all_ghost_windows` /
// `non_left_double_click_and_tunnel_do_not_despawn_ghost_windows` は stand-in
// 即終了（プレーンなダブルクリックで全窓 despawn）の挙動を檻にしていたが、
// 本 task で stand-in を退役し正規ハンドラ（`on_char_pointer_pressed`）へ
// 差し替えたため仕様退役＝除去した（[[obsolete-vs-broken-test-policy]]）。
// 正規ハンドラの挙動檻（プレーン dblclick は despawn せず DoubleClick 送出／
// Ctrl+左 dblclick で暫定退避 despawn）は input_events の task 2.7 檻が所有し、
// task 4.4 で再カバーされる。本 spawn.rs 側は登録の存在
// （`t_i1_all_windows_have_hit_test_none_and_char_has_pointer_handlers`）で足る。
// -------------------------------------------------------------------------

// -------------------------------------------------------------------------
// T-I4: clickthrough 登録 system（6.1・task 5.2）
//
// 実 `ClickThroughRegistryHandle` は wintf 内部（`new` は pub(crate)）でしか
// 構築できないため、headless の「登録呼び出しが発生する」観測は偽装境界
// （`ClickThroughRegistrar` を `FakeRegistrar` へ差し替え）で行う。汎用実装
// `register_ghost_windows_via` が本体の query filter（GhostWindowMarker ×
// Added<WindowHandle>）ごと system として走る＝production 経路そのもの。
// -------------------------------------------------------------------------

use std::cell::RefCell;
use windows::Win32::Foundation::HWND;

use super::{
    ClickThroughRegistrar, register_ghost_windows_click_through, register_ghost_windows_via,
};

/// 登録呼び出しを記録する偽 registrar（NonSend リソースとして挿入）。
#[derive(Default)]
struct FakeRegistrar {
    calls: RefCell<Vec<(Entity, isize)>>,
}

impl ClickThroughRegistrar for FakeRegistrar {
    fn register_window(&self, window: Entity, hwnd: HWND) {
        self.calls.borrow_mut().push((window, hwnd.0 as isize));
    }
}

fn registrar_calls(world: &World) -> Vec<(Entity, isize)> {
    world.non_send::<FakeRegistrar>().calls.borrow().clone()
}

fn register_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(register_ghost_windows_via::<FakeRegistrar>);
    schedule
}

/// T-I4: `GhostWindowMarker` 窓に `WindowHandle` が付いた瞬間（Added）だけ
/// 登録呼び出しが発生し、(Entity, HWND) が正値・再実行で重複登録しない・
/// 後から HWND が付いた窓も追加で 1 回だけ登録される。
#[test]
fn t_i4_register_system_registers_ghost_windows_on_added_window_handle_once() {
    let mut world = World::new();
    let placements = two_scope_placements();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    world.insert_non_send(FakeRegistrar::default());
    let mut schedule = register_schedule();

    // spawn 直後は WindowHandle 不在 → 登録は起きない
    schedule.run(&mut world);
    assert!(registrar_calls(&world).is_empty());

    // scope0 の 2 窓へ HWND 付与（wintf create_windows が付ける状況の模擬）
    let char0 = gw.char_window(0).unwrap();
    let balloon0 = gw.balloon_window(0).unwrap();
    world.entity_mut(char0).insert(fake_window_handle(0x10));
    world.entity_mut(balloon0).insert(fake_window_handle(0x20));

    schedule.run(&mut world);
    let mut calls = registrar_calls(&world);
    calls.sort_by_key(|(_, hwnd)| *hwnd);
    assert_eq!(calls, vec![(char0, 0x10), (balloon0, 0x20)]);

    // 再実行しても重複登録しない（Added は厳密 1 回）
    schedule.run(&mut world);
    assert_eq!(registrar_calls(&world).len(), 2);

    // 後から HWND が付いた scope1 キャラ窓も追加で 1 回だけ登録される
    let char1 = gw.char_window(1).unwrap();
    world.entity_mut(char1).insert(fake_window_handle(0x30));
    schedule.run(&mut world);
    let calls = registrar_calls(&world);
    assert_eq!(calls.len(), 3);
    assert!(calls.contains(&(char1, 0x30)));
}

/// T-I4 補: `GhostWindowMarker` を持たない窓は `WindowHandle` が付いても
/// 登録されない（標的は placement 生成窓のみ・6.1）。
#[test]
fn t_i4_register_system_ignores_non_ghost_windows() {
    let mut world = World::new();
    world.insert_non_send(FakeRegistrar::default());
    let mut schedule = register_schedule();

    world.spawn((Window::default(), fake_window_handle(0x40)));

    schedule.run(&mut world);
    assert!(registrar_calls(&world).is_empty());
}

/// T-I4 補: 実 system（design 正本 signature）は
/// `ClickThroughRegistryHandle` 未挿入の headless World で no-op（panic
/// しない・ごく初期 tick の未挿入への Option 防御＝donor と同じ作法）。
#[test]
fn t_i4_real_register_system_is_noop_without_registry_resource() {
    let mut world = World::new();
    let placements = two_scope_placements();
    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    let char0 = gw.char_window(0).unwrap();
    world.entity_mut(char0).insert(fake_window_handle(0x50));

    let mut schedule = Schedule::default();
    schedule.add_systems(register_ghost_windows_click_through);
    schedule.run(&mut world);

    // no-op で完走（窓はそのまま）
    assert_eq!(ghost_window_entities(&mut world).len(), 4);
}
