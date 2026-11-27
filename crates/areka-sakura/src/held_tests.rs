//! `HeldQueue` の檻。

use super::*;
use dola::time::testing::at_ms;

fn drain(q: &mut HeldQueue<&'static str>, frame_now: TimePoint) -> Vec<&'static str> {
    std::iter::from_fn(|| q.pop_due(frame_now)).collect()
}

#[test]
fn head_waiting_holds_back_later_items_even_when_their_time_has_come() {
    let mut q = HeldQueue::new();
    q.push(Some(at_ms(100)), "a");
    q.push(Some(at_ms(50)), "b");

    assert_eq!(drain(&mut q, at_ms(60)), Vec::<&str>::new());
    assert_eq!(drain(&mut q, at_ms(100)), vec!["a", "b"]);
    assert!(q.is_empty());
}

#[test]
fn due_items_come_out_in_arrival_order() {
    let mut q = HeldQueue::new();
    q.push(Some(at_ms(10)), "a");
    q.push(Some(at_ms(20)), "b");
    q.push(Some(at_ms(30)), "c");
    q.push(Some(at_ms(40)), "d");

    assert_eq!(drain(&mut q, at_ms(30)), vec!["a", "b", "c"]);
    assert_eq!(q.waiting_until(), Some(at_ms(40)));
}

#[test]
fn item_comes_out_at_exactly_its_time() {
    let mut q = HeldQueue::new();
    q.push(Some(at_ms(100)), "a");

    assert_eq!(
        q.pop_due(at_ms(100) - dola::TimeSpan::try_from_ticks(1).unwrap()),
        None
    );
    assert_eq!(q.pop_due(at_ms(100)), Some("a"));
}

#[test]
fn item_without_time_comes_out_in_the_same_call_sequence_as_the_item_before_it() {
    let mut q = HeldQueue::new();
    q.push(Some(at_ms(100)), "a");
    q.push(None, "b");
    q.push(Some(at_ms(200)), "c");

    assert_eq!(drain(&mut q, at_ms(50)), Vec::<&str>::new());
    assert_eq!(drain(&mut q, at_ms(100)), vec!["a", "b"]);
    assert_eq!(q.waiting_until(), Some(at_ms(200)));
}

#[test]
fn item_without_time_at_the_head_comes_out_at_once() {
    let mut q = HeldQueue::new();
    q.push(None, "a");
    q.push(Some(at_ms(100)), "b");

    assert_eq!(q.waiting_until(), None);
    assert_eq!(drain(&mut q, at_ms(0)), vec!["a"]);
    assert_eq!(q.waiting_until(), Some(at_ms(100)));
}

#[test]
fn waiting_until_returns_the_head_time_and_none_when_empty() {
    let mut q = HeldQueue::new();
    assert_eq!(q.waiting_until(), None);
    assert!(q.is_empty());

    q.push(Some(at_ms(100)), "a");
    q.push(Some(at_ms(50)), "b");
    assert_eq!(q.waiting_until(), Some(at_ms(100)));
    assert!(!q.is_empty());

    assert_eq!(drain(&mut q, at_ms(100)), vec!["a", "b"]);
    assert_eq!(q.waiting_until(), None);
    assert!(q.is_empty());
}

#[test]
fn pushing_one_by_one_and_in_bulk_release_the_same_items_in_the_same_order() {
    let items = [
        (Some(at_ms(10)), "a"),
        (None, "b"),
        (Some(at_ms(30)), "c"),
        (Some(at_ms(20)), "d"),
        (None, "e"),
        (Some(at_ms(50)), "f"),
        (None, "g"),
    ];
    for frame in [0, 10, 25, 30, 49, 50, 60] {
        let frame_now = at_ms(frame);

        let mut bulk = HeldQueue::new();
        for (at, item) in items {
            bulk.push(at, item);
        }
        let from_bulk = drain(&mut bulk, frame_now);

        let mut one_by_one = HeldQueue::new();
        let mut from_one_by_one = Vec::new();
        for (at, item) in items {
            one_by_one.push(at, item);
            from_one_by_one.extend(drain(&mut one_by_one, frame_now));
        }

        assert_eq!(from_one_by_one, from_bulk, "frame {frame} ms");
        assert_eq!(
            one_by_one.waiting_until(),
            bulk.waiting_until(),
            "frame {frame} ms"
        );
    }
}
