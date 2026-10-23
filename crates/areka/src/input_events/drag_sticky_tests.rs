//! ドラッグの受け手から組の連れ動きまでの結合の檻（areka-P0-window-link-directives task 3.2・
//! 要件 2.1〜2.4・4.1〜4.6・4.8・設計「sticky_follow」・設計の決め 4・6・12）。
//!
//! 本番と同じ順に組んだ 2 スコープの窓（[`Rig`]・scope 1 をドラッグし scope 0 が組の相手）へ、
//! 帳簿を 2.1 の `seed_sticky_descript` で直に据え、wintf の配り（`dispatch_drag_events`）で
//! 開始・途中・終わりを通す。台本で向きを変えた窓は `apply_script_alignment` と同じ部品
//! （`Anchored`＋印つきの `AnchorSource`）を直に入れて作る。

use areka_sylphya::{Axis, PersistKey};
use bevy_ecs::prelude::*;
use wintf::ecs::Point;
use wintf::ecs::drag::{DragConfig, DraggingState};

use super::test_support::{Rig, ended, started};
use crate::placement::follow::{AnchorSource, Anchored, MonitorSnapshot, sync_drag_writer};
use crate::placement::persist::{char_pos_from_origin_x, char_pos_to_origin_x, project_restore};
use crate::placement::resolver::{Anchor, PointPx, RectPx, SizePx};
use crate::placement::sticky_group_ledger::seed_sticky_descript;

/// 押した位置の窓の中の物理 px。
const PRESS_LOCAL: Point = Point { x: 20, y: 30 };
/// カーソルの量（横にも縦にも動かす）。
const MOVE_BY: Point = Point { x: 150, y: -80 };

fn add(p: Point, by: Point) -> Point {
    Point {
        x: p.x + by.x,
        y: p.y + by.y,
    }
}

fn sub(a: Point, b: Point) -> Point {
    Point {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

/// 台本で向きを変えた窓にする（descript の向きは作ったときの下のまま）。
fn script_anchor(world: &mut World, window: Entity, anchor: Anchor) {
    world.entity_mut(window).insert((
        Anchored(anchor),
        AnchorSource {
            descript: Anchor::Bottom,
            by_script: true,
        },
    ));
}

/// descript の組 `0,1` を据える（書き手の付け直しも走る）。
fn group(rig: &mut Rig) {
    seed_sticky_descript(&mut rig.world, Some("0,1"));
}

/// ドラッグを始める（`DraggingState.initial_inset` を窓の位置にする）。押した位置を返す。
fn begin(rig: &mut Rig) -> Point {
    let char_w = rig.char_w;
    let start = rig.position_of(char_w);
    let press = add(start, PRESS_LOCAL);
    rig.put(started(char_w, press));
    rig.tick();
    rig.set_initial_inset(char_w, start);
    press
}

/// 途中の 1 回の配り。
fn drag(rig: &mut Rig, cursor: Point) {
    rig.drag_to(cursor);
    rig.tick();
}

fn end(rig: &mut Rig, cursor: Point, cancelled: bool) {
    rig.put(ended(rig.char_w, cursor, cancelled));
    rig.tick();
}

fn size_of(rig: &Rig, window: Entity) -> SizePx {
    let s = rig
        .world
        .get::<wintf::ecs::WindowPos>(window)
        .and_then(|wp| wp.size)
        .expect("WindowPos.size があるはず");
    SizePx {
        w: s.width,
        h: s.height,
    }
}

/// 記憶に書かれたスコープの位置（x・y）。
fn saved_pos(saved: &[(PersistKey, String)], scope: u32) -> Option<PointPx> {
    let read = |want: Axis| {
        saved.iter().find_map(|(key, value)| match key {
            PersistKey::WindowPos { scope: s, axis } if *s == scope && *axis == want => {
                value.parse::<i32>().ok()
            }
            _ => None,
        })
    };
    Some(PointPx {
        x: read(Axis::X)?,
        y: read(Axis::Y)?,
    })
}

/// 位置を descript の向き（下）の形で書いた値。
fn bottom_form(rig: &Rig, window: Entity, pos: Point) -> PointPx {
    char_pos_to_origin_x(
        Anchor::Bottom,
        PointPx { x: pos.x, y: pos.y },
        size_of(rig, window),
    )
}

/// 組の片方のドラッグで、相手とそのバルーンが同じ量だけ動き、終わりに両方の位置が記憶に
/// 書かれる（4.1・4.4・4.5）。どちらも下に吸着しているので量は横だけ（2.1）。
#[test]
fn partner_and_its_balloon_move_by_the_same_amount_and_both_are_saved() {
    let mut rig = Rig::new();
    group(&mut rig);
    let (char_w, partner, pb) = (rig.char_w, rig.partner, rig.partner_balloon);
    let start = rig.position_of(char_w);
    let partner_start = rig.position_of(partner);
    let pb_start = rig.position_of(pb);

    let press = begin(&mut rig);
    drag(&mut rig, add(press, MOVE_BY));
    let moved = sub(rig.position_of(char_w), start);
    assert_eq!(
        moved,
        Point { x: MOVE_BY.x, y: 0 },
        "下に吸着して横にだけ動く"
    );
    assert_eq!(rig.position_of(partner), add(partner_start, moved));
    assert_eq!(
        rig.position_of(pb),
        add(pb_start, moved),
        "相手のバルーンも付いて動く"
    );

    end(&mut rig, add(press, MOVE_BY), false);
    let final_char = rig.position_of(char_w);
    let final_partner = rig.position_of(partner);
    assert_eq!(final_partner, add(partner_start, moved));
    let saved = rig.saved();
    assert_eq!(
        saved_pos(&saved, 1),
        Some(bottom_form(&rig, char_w, final_char))
    );
    assert_eq!(
        saved_pos(&saved, 0),
        Some(bottom_form(&rig, partner, final_partner))
    );
    rig.close();
}

/// 相手が下に吸着していれば、ドラッグした窓が縦にも動いても相手は横にだけ動く（4.2）。
#[test]
fn bottom_partner_moves_only_sideways() {
    let mut rig = Rig::new();
    script_anchor(&mut rig.world, rig.char_w, Anchor::Free);
    group(&mut rig);
    let (char_w, partner) = (rig.char_w, rig.partner);
    let start = rig.position_of(char_w);
    let partner_start = rig.position_of(partner);

    let press = begin(&mut rig);
    drag(&mut rig, add(press, MOVE_BY));
    assert_eq!(
        rig.position_of(char_w),
        add(start, MOVE_BY),
        "吸着なしは縦横に動く"
    );
    assert_eq!(
        rig.position_of(partner),
        Point {
            x: partner_start.x + MOVE_BY.x,
            y: partner_start.y
        }
    );
    end(&mut rig, add(press, MOVE_BY), false);
    rig.close();
}

/// ドラッグした窓が吸着で動かなかった軸では、吸着なしの相手も動かない（4.3）。
#[test]
fn axis_the_dragged_window_could_not_move_does_not_move_the_partner() {
    let mut rig = Rig::new();
    script_anchor(&mut rig.world, rig.partner, Anchor::Free);
    group(&mut rig);
    let partner = rig.partner;
    let partner_start = rig.position_of(partner);

    let press = begin(&mut rig);
    drag(&mut rig, add(press, MOVE_BY));
    assert_eq!(
        rig.position_of(partner),
        Point {
            x: partner_start.x + MOVE_BY.x,
            y: partner_start.y
        },
        "ドラッグした窓は下に吸着して縦に動かないので、相手も縦に動かない"
    );
    end(&mut rig, add(press, MOVE_BY), false);
    rig.close();
}

/// 取り消しで終わると、ドラッグした窓も相手も始めの位置へ戻り、その位置が記憶に書かれる。
#[test]
fn cancel_returns_the_partner_to_its_start() {
    let mut rig = Rig::new();
    group(&mut rig);
    let (char_w, partner) = (rig.char_w, rig.partner);
    let start = rig.position_of(char_w);
    let partner_start = rig.position_of(partner);

    let press = begin(&mut rig);
    drag(&mut rig, add(press, MOVE_BY));
    assert_ne!(
        rig.position_of(partner),
        partner_start,
        "前提: 途中で動いた"
    );
    end(&mut rig, press, true);

    assert_eq!(rig.position_of(char_w), start);
    assert_eq!(rig.position_of(partner), partner_start);
    let saved = rig.saved();
    assert_eq!(
        saved_pos(&saved, 0),
        Some(bottom_form(&rig, partner, partner_start))
    );
    rig.close();
}

/// 終わりに wintf の `DraggingState` が無くても、控えから量を取って相手の最終位置と記憶が書かれる。
#[test]
fn end_without_dragging_state_still_moves_and_saves_the_partner() {
    let mut rig = Rig::new();
    group(&mut rig);
    let (char_w, partner) = (rig.char_w, rig.partner);
    let start = rig.position_of(char_w);
    let partner_start = rig.position_of(partner);

    let press = begin(&mut rig);
    drag(&mut rig, add(press, MOVE_BY));
    let moved = sub(rig.position_of(char_w), start);
    rig.world.entity_mut(char_w).remove::<DraggingState>();
    end(&mut rig, add(press, Point { x: 400, y: 0 }), false);

    let final_partner = rig.position_of(partner);
    assert_eq!(final_partner, add(partner_start, moved));
    let saved = rig.saved();
    assert_eq!(
        saved_pos(&saved, 0),
        Some(bottom_form(&rig, partner, final_partner))
    );
    assert!(saved_pos(&saved, 1).is_some(), "ドラッグした窓も書かれる");
    rig.close();
}

/// ドラッグ中に台本が向きを変えても、そのドラッグの終わりまでは始めの向きで縛り、終わりに
/// 今の向きで当て直す（2.4）。
#[test]
fn anchor_change_mid_drag_binds_by_the_start_anchor_and_reapplies_at_the_end() {
    let mut rig = Rig::new();
    script_anchor(&mut rig.world, rig.char_w, Anchor::Free);
    group(&mut rig);
    let char_w = rig.char_w;
    let start = rig.position_of(char_w);
    let size = size_of(&rig, char_w);

    let press = begin(&mut rig);
    drag(&mut rig, add(press, MOVE_BY));
    script_anchor(&mut rig.world, char_w, Anchor::Bottom);
    let later = Point { x: 200, y: -100 };
    drag(&mut rig, add(press, later));
    assert_eq!(
        rig.position_of(char_w),
        add(start, later),
        "始めの向き（吸着なし）で縛る"
    );

    end(&mut rig, add(press, later), false);
    assert_eq!(
        rig.position_of(char_w).y,
        1200 - size.h,
        "終わりに今の向き（下）で当て直す"
    );
    rig.close();
}

/// 2 つとも吸着なしの組では、どちらの窓も wintf でなく areka が同じ回に書き、どの回でも
/// 位置の関係が変わらない（4.1・設計の決め 12）。
#[test]
fn two_free_windows_in_a_group_are_both_written_by_areka_each_step() {
    let mut rig = Rig::new();
    script_anchor(&mut rig.world, rig.char_w, Anchor::Free);
    script_anchor(&mut rig.world, rig.partner, Anchor::Free);
    group(&mut rig);
    let (char_w, partner) = (rig.char_w, rig.partner);
    for window in [char_w, partner] {
        assert!(
            !rig.world.get::<DragConfig>(window).unwrap().move_window,
            "組に入った吸着なしの窓は wintf に動かさせない"
        );
    }
    let start = rig.position_of(char_w);
    let relation = sub(rig.position_of(partner), start);

    let press = begin(&mut rig);
    for by in [Point { x: 10, y: 5 }, MOVE_BY, Point { x: -30, y: 40 }] {
        drag(&mut rig, add(press, by));
        // 窓の手続きの代わりに窓を動かしていないので、動いていれば areka が書いた。
        assert_eq!(rig.position_of(char_w), add(start, by));
        assert_eq!(
            sub(rig.position_of(partner), rig.position_of(char_w)),
            relation
        );
    }
    end(&mut rig, add(press, MOVE_BY), false);
    assert_eq!(
        sub(rig.position_of(partner), rig.position_of(char_w)),
        relation
    );
    rig.close();
}

/// バルーンのドラッグでは、キャラクターの窓も組の相手も動かない（4.6）。
#[test]
fn balloon_drag_does_not_move_the_partner() {
    let mut rig = Rig::new();
    group(&mut rig);
    let (char_w, partner, balloon) = (rig.char_w, rig.partner, rig.balloon);
    let char_start = rig.position_of(char_w);
    let partner_start = rig.position_of(partner);
    let balloon_start = rig.position_of(balloon);

    let press = add(balloon_start, PRESS_LOCAL);
    rig.put(started(balloon, press));
    rig.tick();
    rig.move_window(balloon, add(balloon_start, MOVE_BY));
    drag(&mut rig, add(press, MOVE_BY));
    rig.put(ended(balloon, add(press, MOVE_BY), false));
    rig.tick();

    assert_eq!(rig.position_of(char_w), char_start);
    assert_eq!(rig.position_of(partner), partner_start);
    rig.close();
}

/// 台本で `free` にした窓をドラッグして離した位置が、次の起動の読み戻し（descript の向き＝下）で
/// 同じ x に出る（4.8）。組に無い吸着なしの窓なので wintf が窓を動かす。
#[test]
fn free_window_drag_restores_to_the_released_x_on_next_boot() {
    let mut rig = Rig::new();
    script_anchor(&mut rig.world, rig.char_w, Anchor::Free);
    sync_drag_writer(&mut rig.world);
    let char_w = rig.char_w;
    assert!(rig.world.get::<DragConfig>(char_w).unwrap().move_window);
    let start = rig.position_of(char_w);
    let size = size_of(&rig, char_w);

    let press = begin(&mut rig);
    rig.move_window(char_w, add(start, MOVE_BY));
    drag(&mut rig, add(press, MOVE_BY));
    end(&mut rig, add(press, MOVE_BY), false);
    let released = rig.position_of(char_w);
    assert_eq!(released, add(start, MOVE_BY));

    let saved = saved_pos(&rig.saved(), 1).expect("離した位置が書かれる");
    let snapshot = MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1200,
        }],
    };
    let restored = project_restore(
        Anchor::Bottom,
        char_pos_from_origin_x(Anchor::Bottom, saved, size),
        size,
        &snapshot,
    );
    assert_eq!(restored.x, released.x, "次の起動で離した x に出る");
    rig.close();
}
