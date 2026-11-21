use bevy_ecs::prelude::*;
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed};

use areka_placement::spawn::spawn_ghost_windows;
use areka_placement::spawn::test_support::{titles, two_scope_placements};

/// spawn 直後に `crate::attach_char_pointer_handlers` を呼ぶと、キャラ窓は
/// 正規ポインタハンドラ（`OnPointerMoved`＋`OnPointerPressed`）を持ち、キャラ窓のハンドラ装着はバルーン窓に
/// 何も付けない（バルーン窓のハンドラは `crate::balloon::attach_balloon_pointer_handlers`
/// が別に付ける）。ハンドラ実挙動はこの crate の他のテストが見るので、ここは存在の有無
/// だけを固定する。装着の関数は `areka_placement` から見えない（依存の向き）ので、
/// 本テストはこの crate に置く。
#[test]
fn char_windows_get_pointer_handlers_and_balloons_do_not() {
    let mut world = World::new();
    let placements = two_scope_placements();

    let gw = spawn_ghost_windows(&mut world, &placements, &titles());
    crate::attach_char_pointer_handlers(&mut world);

    for p in &placements {
        let char_e = gw.char_window(p.scope).unwrap();
        let balloon_e = gw.balloon_window(p.scope).unwrap();

        assert!(
            world.get::<OnPointerMoved>(char_e).is_some(),
            "scope{}: キャラ窓は OnPointerMoved を持つ",
            p.scope
        );
        assert!(
            world.get::<OnPointerPressed>(char_e).is_some(),
            "scope{}: キャラ窓は OnPointerPressed を持つ",
            p.scope
        );

        assert!(
            world.get::<OnPointerMoved>(balloon_e).is_none(),
            "scope{}: キャラ窓のハンドラ装着がバルーン窓に OnPointerMoved を付けた",
            p.scope
        );
        assert!(
            world.get::<OnPointerPressed>(balloon_e).is_none(),
            "scope{}: キャラ窓のハンドラ装着がバルーン窓に OnPointerPressed を付けた",
            p.scope
        );
    }
}
