//! `ranges` の決定論テスト（重なりの勝ち負け・差し替えの後の順・窓の全体・空の文字・取り消し・通し番号）。

use super::*;
use bevy_ecs::world::World;

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> TooltipArea {
    TooltipArea::Rect(Rect {
        left,
        top,
        right,
        bottom,
    })
}

fn range(area: TooltipArea, text: Option<&str>) -> TooltipRange {
    TooltipRange {
        area,
        text: text.map(str::to_owned),
    }
}

fn pt(x: f32, y: f32) -> PointF {
    PointF::new(x, y)
}

fn window() -> Entity {
    World::new().spawn_empty().id()
}

#[test]
fn overlap_later_registration_wins() {
    let w = window();
    let mut t = TooltipRanges::default();
    let first = t.add(w, range(rect(0.0, 0.0, 100.0, 100.0), Some("first")));
    let second = t.add(w, range(rect(50.0, 50.0, 150.0, 150.0), Some("second")));

    // 重なった所は後から登録した方。
    let (id, r) = t.hit(pt(75.0, 75.0)).unwrap();
    assert_eq!(id, second);
    assert_eq!(r.text.as_deref(), Some("second"));
    // 重なっていない所はそれぞれ。
    assert_eq!(t.hit(pt(10.0, 10.0)).unwrap().0, first);
    assert_eq!(t.hit(pt(140.0, 140.0)).unwrap().0, second);
    // どれにも入らない所。
    assert!(t.hit(pt(200.0, 10.0)).is_none());
    assert_eq!(first.window(), w);
}

#[test]
fn rect_includes_left_top_excludes_right_bottom() {
    let w = window();
    let mut t = TooltipRanges::default();
    let id = t.add(w, range(rect(10.0, 20.0, 30.0, 40.0), None));
    assert_eq!(t.hit(pt(10.0, 20.0)).unwrap().0, id);
    assert_eq!(t.hit(pt(29.9, 39.9)).unwrap().0, id);
    assert!(t.hit(pt(30.0, 25.0)).is_none());
    assert!(t.hit(pt(15.0, 40.0)).is_none());
    assert!(t.hit(pt(9.9, 25.0)).is_none());
}

#[test]
fn replace_keeps_position_in_order() {
    let w = window();
    let mut t = TooltipRanges::default();
    let first = t.add(w, range(rect(0.0, 0.0, 100.0, 100.0), Some("first")));
    let second = t.add(w, range(rect(0.0, 0.0, 100.0, 100.0), Some("second")));

    // 先に登録した方を差し替えても、後から登録した方が勝ったまま。
    assert!(t.replace(first, range(rect(0.0, 0.0, 200.0, 200.0), Some("first-2"))));
    assert_eq!(t.hit(pt(50.0, 50.0)).unwrap().0, second);
    // 差し替えた中身は効いている。
    let (id, r) = t.hit(pt(150.0, 150.0)).unwrap();
    assert_eq!(id, first);
    assert_eq!(r.text.as_deref(), Some("first-2"));
    assert_eq!(t.get(first).unwrap().text.as_deref(), Some("first-2"));

    // 後から登録した方を差し替えても、勝ち負けは変わらない。
    assert!(t.replace(
        second,
        range(rect(0.0, 0.0, 100.0, 100.0), Some("second-2"))
    ));
    assert_eq!(t.hit(pt(50.0, 50.0)).unwrap().0, second);
}

#[test]
fn whole_window_hits_everywhere_and_later_rect_wins_over_it() {
    let w = window();
    let mut t = TooltipRanges::default();
    let whole = t.add(w, range(TooltipArea::WholeWindow, Some("whole")));
    assert_eq!(t.hit(pt(0.0, 0.0)).unwrap().0, whole);
    assert_eq!(t.hit(pt(-5.0, 9999.0)).unwrap().0, whole);

    let part = t.add(w, range(rect(10.0, 10.0, 20.0, 20.0), Some("part")));
    assert_eq!(t.hit(pt(15.0, 15.0)).unwrap().0, part);
    assert_eq!(t.hit(pt(50.0, 50.0)).unwrap().0, whole);

    // 窓の全体を後から登録すれば、そちらが勝つ。
    let mut t2 = TooltipRanges::default();
    t2.add(w, range(rect(10.0, 10.0, 20.0, 20.0), None));
    let whole2 = t2.add(w, range(TooltipArea::WholeWindow, None));
    assert_eq!(t2.hit(pt(15.0, 15.0)).unwrap().0, whole2);
}

#[test]
fn empty_text_is_held_as_no_text() {
    let w = window();
    let mut t = TooltipRanges::default();
    let id = t.add(w, range(TooltipArea::WholeWindow, Some("")));
    assert_eq!(t.get(id).unwrap().text, None);
    assert_eq!(t.hit(pt(1.0, 1.0)).unwrap().1.text, None);

    // 差し替えでも同じ。
    assert!(t.replace(id, range(TooltipArea::WholeWindow, Some("x"))));
    assert_eq!(t.get(id).unwrap().text.as_deref(), Some("x"));
    assert!(t.replace(id, range(TooltipArea::WholeWindow, Some(""))));
    assert_eq!(t.get(id).unwrap().text, None);

    // 空白だけの文字は空ではない（預けたまま）。
    assert!(t.replace(id, range(TooltipArea::WholeWindow, Some(" "))));
    assert_eq!(t.get(id).unwrap().text.as_deref(), Some(" "));
}

#[test]
fn removed_handle_never_hits_and_is_gone() {
    let w = window();
    let mut t = TooltipRanges::default();
    let under = t.add(w, range(rect(0.0, 0.0, 100.0, 100.0), None));
    let over = t.add(w, range(rect(0.0, 0.0, 100.0, 100.0), None));

    assert!(t.remove(over));
    assert_eq!(t.hit(pt(50.0, 50.0)).unwrap().0, under);
    assert!(t.get(over).is_none());
    // 取り消した持ち手は、差し替えも取り消しも偽。
    assert!(!t.remove(over));
    assert!(!t.replace(over, range(TooltipArea::WholeWindow, None)));
    assert!(t.hit(pt(500.0, 500.0)).is_none());

    assert!(t.remove(under));
    assert!(t.hit(pt(50.0, 50.0)).is_none());
}

#[test]
fn serial_is_never_reused() {
    let w = window();
    let mut t = TooltipRanges::default();
    let a = t.add(w, range(TooltipArea::WholeWindow, Some("a")));
    assert!(t.remove(a));
    let b = t.add(w, range(TooltipArea::WholeWindow, Some("b")));
    assert_ne!(a, b);
    // 古い持ち手は新しい登録を指さない。
    assert!(t.get(a).is_none());
    assert!(!t.replace(a, range(TooltipArea::WholeWindow, Some("stale"))));
    assert!(!t.remove(a));
    assert_eq!(t.hit(pt(1.0, 1.0)).unwrap().0, b);
    assert_eq!(t.get(b).unwrap().text.as_deref(), Some("b"));

    // 途中を取り消しても、次の番号は前のどれとも重ならない。
    let c = t.add(w, range(TooltipArea::WholeWindow, None));
    let d = t.add(w, range(TooltipArea::WholeWindow, None));
    assert!(t.remove(c));
    let e = t.add(w, range(TooltipArea::WholeWindow, None));
    for old in [a, b, c, d] {
        assert_ne!(e, old);
    }
}

#[test]
fn handle_of_another_window_does_not_match() {
    let mut world = World::new();
    let w1 = world.spawn_empty().id();
    let w2 = world.spawn_empty().id();
    let mut t1 = TooltipRanges::default();
    let mut t2 = TooltipRanges::default();
    let a = t1.add(w1, range(TooltipArea::WholeWindow, Some("w1")));
    let b = t2.add(w2, range(TooltipArea::WholeWindow, Some("w2")));
    assert_eq!(b.window(), w2);
    // 番号が同じでも、別の窓の持ち手では触れない。
    assert!(t1.get(b).is_none());
    assert!(!t1.replace(b, range(TooltipArea::WholeWindow, None)));
    assert!(!t1.remove(b));
    assert_eq!(t1.get(a).unwrap().text.as_deref(), Some("w1"));
}
