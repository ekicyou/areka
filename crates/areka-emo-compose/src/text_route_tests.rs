//! `text_route.rs` の行き先の遷移のテスト。

use std::collections::{BTreeMap, BTreeSet};

use crate::{BoxName, BoxSelect, SurfaceKeyOutcome, TextPlace, TextRoute};

fn name(s: &str) -> BoxName {
    BoxName(s.to_string())
}

fn boxed(s: &str) -> TextPlace {
    TextPlace::Box(name(s))
}

/// 面 0 は箱なし、面 10 は main・sub、面 11 は sub だけ、面 12 は other だけ。
fn table() -> BTreeMap<u32, Vec<BoxName>> {
    BTreeMap::from([
        (10, vec![name("main"), name("sub")]),
        (11, vec![name("sub")]),
        (12, vec![name("other")]),
    ])
}

fn at(surface: u32) -> TextRoute {
    let mut route = TextRoute::default();
    route.follow_surface(&table(), SurfaceKeyOutcome::Show(surface));
    route
}

#[test]
fn default_route_is_hidden_and_balloon() {
    let route = TextRoute::default();
    assert_eq!(route.surface(), None);
    assert_eq!(route.dest(), &TextPlace::Balloon);
}

#[test]
fn surface_without_boxes_defaults_to_balloon() {
    let mut route = TextRoute::default();
    assert!(!route.follow_surface(&table(), SurfaceKeyOutcome::Show(0)));
    assert_eq!(route.surface(), Some(0));
    assert_eq!(route.dest(), &TextPlace::Balloon);
}

#[test]
fn surface_with_boxes_defaults_to_first_box() {
    let mut route = TextRoute::default();
    assert!(route.follow_surface(&table(), SurfaceKeyOutcome::Show(10)));
    assert_eq!(route.surface(), Some(10));
    assert_eq!(route.dest(), &boxed("main"));
}

#[test]
fn empty_table_never_leaves_balloon() {
    let empty = BTreeMap::new();
    let mut route = TextRoute::default();
    assert!(!route.follow_surface(&empty, SurfaceKeyOutcome::Show(10)));
    assert_eq!(route.select_box(&empty, "main"), BoxSelect::Missing);
    route.reset(&empty);
    assert_eq!(route.dest(), &TextPlace::Balloon);
}

#[test]
fn surface_change_keeps_box_present_on_new_surface() {
    let mut route = at(10);
    assert_eq!(route.select_box(&table(), "sub"), BoxSelect::Changed);
    assert!(!route.follow_surface(&table(), SurfaceKeyOutcome::Show(11)));
    assert_eq!(route.surface(), Some(11));
    assert_eq!(route.dest(), &boxed("sub"));
}

#[test]
fn surface_change_falls_back_to_default_when_box_is_missing() {
    let mut route = at(10);
    assert!(route.follow_surface(&table(), SurfaceKeyOutcome::Show(12)));
    assert_eq!(route.dest(), &boxed("other"));
    assert!(route.follow_surface(&table(), SurfaceKeyOutcome::Show(0)));
    assert_eq!(route.dest(), &TextPlace::Balloon);
}

#[test]
fn hide_returns_to_balloon() {
    let mut route = at(10);
    assert!(route.follow_surface(&table(), SurfaceKeyOutcome::Hide));
    assert_eq!(route.surface(), None);
    assert_eq!(route.dest(), &TextPlace::Balloon);
}

#[test]
fn unresolved_surface_changes_nothing() {
    let mut route = at(10);
    route.select_box(&table(), "sub");
    let before = route.clone();
    assert!(!route.follow_surface(&table(), SurfaceKeyOutcome::Unresolved));
    assert_eq!(route, before);

    let mut fresh = TextRoute::default();
    assert!(!fresh.follow_surface(&table(), SurfaceKeyOutcome::Unresolved));
    assert_eq!(fresh, TextRoute::default());
}

#[test]
fn select_box_reports_changed_same_and_missing() {
    let mut route = at(10);
    assert_eq!(route.select_box(&table(), "sub"), BoxSelect::Changed);
    assert_eq!(route.dest(), &boxed("sub"));
    assert_eq!(route.select_box(&table(), "sub"), BoxSelect::Same);
    assert_eq!(route.dest(), &boxed("sub"));
    assert_eq!(route.select_box(&table(), "other"), BoxSelect::Missing);
    assert_eq!(route.dest(), &boxed("sub"));
}

#[test]
fn select_box_on_hidden_scope_is_missing() {
    let mut route = TextRoute::default();
    assert_eq!(route.select_box(&table(), "main"), BoxSelect::Missing);
    assert_eq!(route.dest(), &TextPlace::Balloon);
}

#[test]
fn reset_returns_to_default_of_current_surface() {
    let mut route = at(10);
    route.select_box(&table(), "sub");
    route.reset(&table());
    assert_eq!(route.surface(), Some(10));
    assert_eq!(route.dest(), &boxed("main"));

    let other = BTreeMap::from([(10, vec![name("x")])]);
    route.reset(&other);
    assert_eq!(route.dest(), &boxed("x"));
    route.reset(&BTreeMap::new());
    assert_eq!(route.dest(), &TextPlace::Balloon);
}

#[test]
fn fallback_outcome_answers() {
    let ids = BTreeSet::from([0, 5]);
    let route = at(10);
    assert_eq!(route.fallback_outcome(&BTreeSet::new(), &[0]), None);
    assert_eq!(TextRoute::default().fallback_outcome(&ids, &[0]), None);
    assert_eq!(at(5).fallback_outcome(&ids, &[0]), None);
    assert_eq!(
        route.fallback_outcome(&ids, &[3, 5, 0]),
        Some(SurfaceKeyOutcome::Show(5))
    );
    assert_eq!(
        route.fallback_outcome(&ids, &[3, 4]),
        Some(SurfaceKeyOutcome::Hide)
    );
    assert_eq!(
        route.fallback_outcome(&ids, &[]),
        Some(SurfaceKeyOutcome::Hide)
    );
}

#[test]
fn box_key_is_any_non_integer() {
    for key in ["main", "", "1a", "0x10", " 1"] {
        assert!(TextRoute::is_box_key(key), "{key:?}");
    }
    for key in ["0", "10", "-1", "+3"] {
        assert!(!TextRoute::is_box_key(key), "{key:?}");
    }
}
