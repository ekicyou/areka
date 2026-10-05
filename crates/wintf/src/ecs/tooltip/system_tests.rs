//! 窓なしの World のテスト（公開の口: 範囲の登録・差し替え）。

use super::*;
// 公開の型が crate::ecs の口から見えることも、ここで確かめる。
use crate::ecs::{
    OnTooltip, PointF, Rect, TooltipArea, TooltipEndReason, TooltipNotice, TooltipOsError,
    TooltipRange, TooltipRangeId, TooltipRegisterError, TooltipSupply, TooltipTurn,
    TooltipTurnToken, Window,
};

fn range(area: TooltipArea, text: Option<&str>) -> TooltipRange {
    TooltipRange {
        area,
        text: text.map(str::to_owned),
    }
}

fn window(world: &mut World) -> Entity {
    world.spawn(Window::default()).id()
}

fn table(world: &World, w: Entity) -> &ranges::TooltipRanges {
    world
        .get::<ranges::TooltipRanges>(w)
        .expect("範囲の表が窓に付いているはず")
}

#[test]
fn register_attaches_table_to_window() {
    let mut world = World::new();
    let w = window(&mut world);
    let r = range(TooltipArea::WholeWindow, Some("説明"));

    let id = register(&mut world, w, r.clone()).expect("窓があるので登録できるはず");

    assert_eq!(id.window(), w);
    assert_eq!(table(&world, w).get(id), Some(&r));
    // 2 つ目は同じ表に足される（表を付け直さない）。
    let id2 = register(&mut world, w, range(TooltipArea::WholeWindow, None)).unwrap();
    assert!(table(&world, w).get(id).is_some());
    assert!(table(&world, w).get(id2).is_some());
}

#[test]
fn register_on_missing_window_is_error() {
    let mut world = World::new();
    let gone = window(&mut world);
    world.despawn(gone);
    let not_window = world.spawn_empty().id();

    for e in [gone, not_window] {
        let got = register(&mut world, e, range(TooltipArea::WholeWindow, Some("x")));
        assert!(
            matches!(got, Err(TooltipRegisterError::NoSuchWindow)),
            "{got:?}"
        );
    }
    assert!(world.get::<ranges::TooltipRanges>(not_window).is_none());
}

#[test]
fn update_replaces_content_and_keeps_order() {
    let mut world = World::new();
    let w = window(&mut world);
    let a = register(&mut world, w, range(TooltipArea::WholeWindow, Some("a"))).unwrap();
    let b = register(&mut world, w, range(TooltipArea::WholeWindow, Some("b"))).unwrap();
    let p = PointF::new(5.0, 5.0);

    // 先に登録した a を差し替えても、重なった所では後の b が勝つまま。
    let a2 = range(TooltipArea::WholeWindow, Some("a2"));
    assert!(update(&mut world, a, a2.clone()));
    assert_eq!(table(&world, w).get(a), Some(&a2));
    assert_eq!(table(&world, w).hit(p).map(|(k, _)| k), Some(b));

    // b を点に当たらない矩形へ差し替えると、a が当たる。
    let off = TooltipArea::Rect(Rect {
        left: 100.0,
        top: 100.0,
        right: 200.0,
        bottom: 200.0,
    });
    assert!(update(&mut world, b, range(off, Some("b2"))));
    assert_eq!(table(&world, w).hit(p).map(|(k, _)| k), Some(a));
}

#[test]
fn update_missing_is_false() {
    let mut world = World::new();
    let w = window(&mut world);
    let a = register(&mut world, w, range(TooltipArea::WholeWindow, Some("a"))).unwrap();
    // 別の窓の持ち手（表の無い窓）・消した窓の持ち手は偽。
    let other = window(&mut world);
    let other_id = {
        let mut t = ranges::TooltipRanges::default();
        t.add(other, range(TooltipArea::WholeWindow, None))
    };
    assert!(!update(
        &mut world,
        other_id,
        range(TooltipArea::WholeWindow, Some("x"))
    ));
    world.despawn(w);
    assert!(!update(
        &mut world,
        a,
        range(TooltipArea::WholeWindow, Some("x"))
    ));
}

#[test]
fn empty_text_is_not_stored() {
    let mut world = World::new();
    let w = window(&mut world);
    let id = register(&mut world, w, range(TooltipArea::WholeWindow, Some(""))).unwrap();
    assert_eq!(table(&world, w).get(id).unwrap().text, None);

    assert!(update(
        &mut world,
        id,
        range(TooltipArea::WholeWindow, Some("x"))
    ));
    assert!(update(
        &mut world,
        id,
        range(TooltipArea::WholeWindow, Some(""))
    ));
    assert_eq!(table(&world, w).get(id).unwrap().text, None);
}

#[test]
fn notice_callback_component_attaches_to_window() {
    fn on(_: &mut World, _: &TooltipNotice) {}
    let mut world = World::new();
    let w = window(&mut world);
    world.entity_mut(w).insert(OnTooltip(on));
    assert!(world.get::<OnTooltip>(w).is_some());

    // 公開の型の名前が crate::ecs から引けること（中身の組み立ては後の段で確かめる）。
    let _: Option<(
        TooltipTurn,
        TooltipSupply,
        TooltipOsError,
        TooltipEndReason,
        TooltipTurnToken,
        TooltipRangeId,
    )> = None;
}
