//! `WM_WINDOWPOSCHANGED` の受け手（`window_pos.rs`）の判断の決定論テスト。
//!
//! 道具窓（`DISPATCH_TARGET`・`create_test_window` ほか）は兄弟の
//! `window_pos_transition_tests.rs` のものを借りる（写さない）。

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SendMessageW,
    SetWindowPos, WINDOWPOS, WM_WINDOWPOSCHANGED,
};

use super::window_pos_transition_tests::{DISPATCH_TARGET, create_test_window};
use crate::ecs::layout::{BoxSize, BoxStyle, Dimension};
use crate::ecs::window::{
    DPI, DpiChangeContext, SetWindowPosCommand, WindowHandle, WindowPos, drain_window_pos_commands,
    guarded_set_window_pos,
};
use crate::ecs::world::{EcsWorld, engage_tick_flush_guard};

/// 錠（`engage_tick_flush_guard`）を取ったまま積んだ指令は、`WM_WINDOWPOSCHANGED` を
/// 受けても流れない——受け手の流しは錠を取る手順②だけで、錠を取らない流しを持たない（41 番）。
#[test]
fn windowposchanged_does_not_flush_the_queue_while_the_lock_is_held() {
    let _residue = drain_window_pos_commands();
    let hwnd = create_test_window();

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = world
        .borrow_mut()
        .world_mut()
        .spawn((
            WindowHandle { hwnd },
            WindowPos::default(),
            DPI::from_dpi(96, 96),
        ))
        .id();
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = Some((world.clone(), entity)));

    let remaining = {
        let _lock = engage_tick_flush_guard().expect("錠が既に取られている");
        SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
            hwnd,
            30,
            40,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            None,
        ));
        let wp = WINDOWPOS {
            hwnd,
            x: 10,
            y: 10,
            flags: SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            ..Default::default()
        };
        // SAFETY: Win32 境界。同じスレッドの自分の窓へ同期で送る（`wp` は呼び出しの間生きている）。
        unsafe {
            SendMessageW(
                hwnd,
                WM_WINDOWPOSCHANGED,
                Some(WPARAM(0)),
                Some(LPARAM(&wp as *const WINDOWPOS as isize)),
            );
        }
        drain_window_pos_commands().len()
    };

    // 配送を止めてから窓を壊す（`DestroyWindow` のメッセージを拾わない）。
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = None);
    // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }

    assert_eq!(
        remaining, 1,
        "錠を取ったままなのに WM_WINDOWPOSCHANGED の受け手がキューを流した"
    );
}

// ---------------------------------------------------------------------------
// 判断の本体（42 番）: 照り返しの書き方・DPI の直後の `BoxStyle` の飛ばし
// ---------------------------------------------------------------------------

/// 判断の後に読む値: `WindowPos` の変更の印・位置・`BoxStyle.size`。
struct Observed {
    changed: bool,
    position: Option<crate::ecs::Point>,
    box_size: Option<BoxSize>,
}

/// 箱の様式の最初の大きさ（どの書き込みとも違う値にしておく）。
fn initial_box_size() -> Option<BoxSize> {
    Some(BoxSize {
        width: Some(Dimension::Px(7.0)),
        height: Some(Dimension::Px(7.0)),
    })
}

/// 道具窓を作り、判断の本体まで届く entity（窓のハンドル・窓の位置・箱の様式・DPI 192）を
/// 置いて配送先に据える。印は置いた直後に一度消す（置いたこと自体を変更と数えない）。
fn set_up() -> (Rc<RefCell<EcsWorld>>, Entity, HWND) {
    let _residue = drain_window_pos_commands();
    let _residue = DpiChangeContext::take();
    let hwnd = create_test_window();
    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let entity = {
        let mut borrow = world.borrow_mut();
        let w = borrow.world_mut();
        let entity = w
            .spawn((
                WindowHandle { hwnd },
                WindowPos::default(),
                BoxStyle {
                    size: initial_box_size(),
                    ..Default::default()
                },
            ))
            .id();
        // 窓のハンドルの付け足しの手続きが実の DPI を入れるので、その後で 192 に置き直す。
        w.entity_mut(entity).insert(DPI::from_dpi(192, 192));
        w.clear_trackers();
        entity
    };
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = Some((world.clone(), entity)));
    (world, entity, hwnd)
}

fn observe(world: &Rc<RefCell<EcsWorld>>, entity: Entity) -> Observed {
    let borrow = world.borrow();
    let e = borrow.world().entity(entity);
    let pos = e.get_ref::<WindowPos>().expect("WindowPos");
    Observed {
        changed: pos.is_changed(),
        position: pos.position,
        box_size: e.get::<BoxStyle>().expect("BoxStyle").size,
    }
}

fn tear_down(hwnd: HWND) {
    DISPATCH_TARGET.with(|cell| *cell.borrow_mut() = None);
    let _residue = DpiChangeContext::take();
    // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

const MOVE_FLAGS: SET_WINDOW_POS_FLAGS = SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0);

/// ⑴ 外からの `SetWindowPos`（照り返しでない・DPI の文脈なし）は `WindowPos` に変更の印を立て、
/// `BoxStyle.size` を物理 px ÷ 倍率の論理 px で書く。
#[test]
fn external_move_marks_window_pos_changed_and_writes_logical_box_size() {
    let (world, entity, hwnd) = set_up();
    // SAFETY: Win32 境界。同じスレッドの自分の窓を動かす（受け手へ同期で届く）。
    unsafe { SetWindowPos(hwnd, None, 30, 40, 200, 100, MOVE_FLAGS) }.expect("SetWindowPos");
    let seen = observe(&world, entity);
    tear_down(hwnd);

    assert!(
        seen.changed,
        "外からの動きなのに WindowPos の変更の印が立たない"
    );
    assert_eq!(seen.position, Some(crate::ecs::Point { x: 30, y: 40 }));
    assert_eq!(
        seen.box_size,
        Some(BoxSize {
            width: Some(Dimension::Px(100.0)),
            height: Some(Dimension::Px(50.0)),
        }),
        "BoxStyle.size は物理 200x100 を倍率 2 で割った論理 px になる"
    );
}

/// ⑵ 自分の `guarded_set_window_pos` の照り返し（DPI の文脈なし）は値を書くが印を立てず、
/// `BoxStyle` も書かない（書けば `apply_window_pos_changes` が同じ位置を書き直し続ける）。
#[test]
fn self_echo_writes_window_pos_without_marking_it_changed() {
    let (world, entity, hwnd) = set_up();
    // SAFETY: Win32 境界。同じスレッドの自分の窓を動かす（受け手へ同期で届く）。
    unsafe { guarded_set_window_pos(hwnd, None, 30, 40, 200, 100, MOVE_FLAGS) }
        .expect("guarded_set_window_pos");
    let seen = observe(&world, entity);
    tear_down(hwnd);

    assert_eq!(
        seen.position,
        Some(crate::ecs::Point { x: 30, y: 40 }),
        "照り返しでも値は書く"
    );
    assert!(!seen.changed, "照り返しなのに WindowPos の変更の印が立った");
    assert_eq!(
        seen.box_size,
        initial_box_size(),
        "照り返しで BoxStyle を書いた"
    );
}

/// ⑶ `DpiChangeContext::set` の直後は、照り返しでも外からでも `WindowPos` に印を立て
/// （照り返しで飛ばすと旧い位置が残り、旧モニターへ戻されて `WM_DPICHANGED` が回り続ける）、
/// `BoxStyle` は書かない（古い物理寸を新しい倍率で割った値で寸が縮む・膨らむ）。
#[test]
fn after_a_dpi_change_window_pos_is_marked_and_box_style_is_left_alone() {
    for echo in [true, false] {
        let (world, entity, hwnd) = set_up();
        DpiChangeContext::set(DpiChangeContext::new(
            DPI::from_dpi(192, 192),
            RECT {
                left: 30,
                top: 40,
                right: 230,
                bottom: 140,
            },
        ));
        // SAFETY: Win32 境界。同じスレッドの自分の窓を動かす（受け手へ同期で届く）。
        let result = unsafe {
            if echo {
                guarded_set_window_pos(hwnd, None, 30, 40, 200, 100, MOVE_FLAGS)
            } else {
                SetWindowPos(hwnd, None, 30, 40, 200, 100, MOVE_FLAGS)
            }
        };
        let seen = observe(&world, entity);
        tear_down(hwnd);

        result.expect("SetWindowPos");
        assert!(
            seen.changed,
            "DPI の直後なのに WindowPos の変更の印が立たない（echo={echo}）"
        );
        assert_eq!(
            seen.box_size,
            initial_box_size(),
            "DPI の直後に BoxStyle を書いた（echo={echo}）"
        );
    }
}
