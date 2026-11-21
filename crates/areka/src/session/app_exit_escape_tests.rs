use areka_input::pointer_test_support::*;
use areka_input::{on_char_pointer_moved, on_char_pointer_pressed};
use areka_placement::spawn::{CharWindowMarker, GhostWindowMarker};
use areka_runtime::hit_region::HitRegion;
use bevy_ecs::prelude::*;
use wintf::ecs::pointer::DoubleClick;

/// 強制退避の檻: **Ctrl+Shift**+左ダブルクリックは従来どおり全窓を despawn し、
/// kanade へは何も送らない（起動後でも効く脱出口）。
///
/// # 非空虚性
/// Shift の判定を落とせば終了指示が送られて `try_recv` の表明が落ち、窓も残って件数が合わない。
#[test]
fn handler_ctrl_shift_left_double_click_despawns_all_ghost_windows_without_sending() {
    let (mut world, rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    crate::session::app_exit::install_input_hooks(&mut world);
    world.spawn(GhostWindowMarker);
    let w0 = world.spawn(GhostWindowMarker).id();
    let other = world.spawn_empty().id();
    assert_eq!(ghost_count(&mut world), 2);

    let ev = bubble_pointer_mods(10, 20, DoubleClick::Left, true, true);
    assert!(on_char_pointer_pressed(&mut world, w0, w0, &ev));
    assert_eq!(
        ghost_count(&mut world),
        0,
        "Ctrl+Shift は強制退避＝全ゴースト窓が despawn される（R15.2）"
    );
    assert!(
        world
            .get_non_send::<wintf::AppExit>()
            .expect("world_with_wiring が受け口を挿している")
            .is_requested(),
        "強制退避は窓を閉じた上で終了を指示する（要件 3.3）"
    );
    assert!(world.get_entity(other).is_ok(), "無関係 entity は残る");
    assert!(rx.try_recv().is_err(), "強制退避は kanade へ何も送らない");
}

/// 起動失敗時の脱出: `MouseWiring` 不在なら Ctrl+左で全窓 despawn。
///
/// 結線前（ghost boot 失敗・boot 前）は終了指示の送り先が無いため、従来どおりの強制退避で
/// 窓を閉じて抜ける。この経路が残っていることが、終了指示を正規の握手へ配線しても
/// 「起動に失敗したゴーストを閉じられる」ことの担保である。
#[test]
fn escape_works_without_mouse_wiring() {
    let mut world = World::new();
    crate::session::app_exit::install_input_hooks(&mut world);
    world.insert_non_send(wintf::AppExit::new());
    world.spawn(GhostWindowMarker);
    let w = world.spawn(GhostWindowMarker).id();
    assert_eq!(ghost_count(&mut world), 2);

    let ev = bubble_pointer(10, 20, DoubleClick::Left, true);
    assert!(
        on_char_pointer_pressed(&mut world, w, w, &ev),
        "wiring 不在でも暫定退避は機能する"
    );
    assert_eq!(ghost_count(&mut world), 0, "全ゴースト窓 despawn");
}

/// 「強制退避操作でのみ全窓終了が起きる」統合檻:
/// Ctrl+左ダブルクリックの**いずれも** `GhostWindowMarker` 窓を despawn しない（非退避操作）。
/// Ctrl+Shift+左ダブルクリック**のみ**が全窓を despawn する。「でのみ」の排他性を単一
/// pass/fail で固定する（plain-left 単独の
/// `handler_left_double_click_without_ctrl_does_not_despawn_and_sends` に対し、非退避操作
/// 全集合の否定＋退避の肯定を 1 檻へ集約する）。
#[test]
fn only_escape_terminates_ghost_windows() {
    let (mut world, _rx) = world_with_wiring(
        |_, x, y| HitRegion {
            scope: 0,
            region: Some("Head".to_string()),
            surface_point: (x, y),
        },
        stepping_clock(1000, 1000),
    );
    crate::session::app_exit::install_input_hooks(&mut world);
    let w = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    world.spawn(GhostWindowMarker);
    world.spawn(GhostWindowMarker);
    assert_eq!(ghost_count(&mut world), 3);

    let non_escape = [
        bubble_pointer(10, 20, DoubleClick::None, false), // 移動（None）
        bubble_pointer(10, 21, DoubleClick::Middle, false), // 中ボタン dblclick
        bubble_pointer(10, 22, DoubleClick::XButton1, false), // 拡張ボタン dblclick
        bubble_pointer(10, 23, DoubleClick::None, false), // 単発クリック
        bubble_pointer(10, 24, DoubleClick::Left, false), // Ctrl なし左 dblclick
    ];
    assert!(on_char_pointer_moved(&mut world, w, w, &non_escape[0]));
    for ev in &non_escape[1..] {
        on_char_pointer_pressed(&mut world, w, w, ev);
    }
    assert_eq!(
        ghost_count(&mut world),
        3,
        "非退避操作（移動/中/拡張/単発/Ctrl なし左）は 1 つも despawn しない"
    );

    // 結線済みの Ctrl+左は退避ではない——Ctrl は無視され、窓は 1 枚も減らない。
    let wired_ctrl_left = bubble_pointer(10, 20, DoubleClick::Left, true);
    assert!(on_char_pointer_pressed(&mut world, w, w, &wired_ctrl_left));
    assert_eq!(
        ghost_count(&mut world),
        3,
        "結線済みの Ctrl+左は左ダブルクリックとして扱われ、窓を閉じない（要件 5.3）"
    );

    // 強制退避操作（Ctrl+Shift+左）のみが全窓を despawn する。
    let escape = bubble_pointer_mods(10, 20, DoubleClick::Left, true, true);
    assert!(on_char_pointer_pressed(&mut world, w, w, &escape));
    assert_eq!(
        ghost_count(&mut world),
        0,
        "強制退避操作でのみ全ゴースト窓が despawn される"
    );
}

/// 本番の組み立て（`register_systems`）だけで強制退避の口が据わり、結線前の
/// Ctrl+左ダブルクリックが窓を閉じて終了を指示する。
#[test]
fn register_systems_installs_the_escape_hook() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_non_send(wintf::AppExit::new());
    let (_kanade_stop, kanade_stop_rx) = std::sync::mpsc::channel::<areka_kanade::KanadeNotice>();
    crate::ghost_session::register_systems(&mut world, kanade_stop_rx);
    assert!(
        world.contains_resource::<areka_input::hooks::InputHooks>(),
        "register_systems が強制退避の口を据える"
    );

    world.spawn(GhostWindowMarker);
    let w = world.spawn(GhostWindowMarker).id();
    assert_eq!(ghost_count(&mut world), 2);

    let ev = bubble_pointer(10, 20, DoubleClick::Left, true);
    assert!(on_char_pointer_pressed(&mut world, w, w, &ev));
    assert_eq!(
        ghost_count(&mut world),
        0,
        "据わった口で全ゴースト窓が閉じる"
    );
    assert!(
        world
            .get_non_send::<wintf::AppExit>()
            .expect("受け口を挿している")
            .is_requested(),
        "据わった口は終了を指示する"
    );
}
