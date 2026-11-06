//! 帳簿の World の側の薄い関数の決定論の檻（task 2.1・要件 3.1・3.4〜3.9・6.5・9.2・
//! design「StickyGroupLedger」）。
//!
//! 窓は偽の部品（`CharWindowMarker`＋`Anchored`＋`DragConfig`）だけで組み、記録は
//! `log-capture-kit`（`test_support::capture_logs`）で取る。

use std::collections::BTreeSet;

use wintf::ecs::drag::DragConfig;

use super::super::follow::Anchored;
use super::super::resolver::Anchor;
use super::super::spawn::CharWindowMarker;
use super::super::test_support::{LogEvent, capture_logs};
use super::*;

fn spawn_char(world: &mut World, scope: usize, anchor: Anchor) -> Entity {
    world
        .spawn((
            CharWindowMarker { scope },
            Anchored(anchor),
            DragConfig {
                move_window: anchor.is_free(),
                ..Default::default()
            },
        ))
        .id()
}

fn move_window(world: &World, e: Entity) -> bool {
    world.get::<DragConfig>(e).unwrap().move_window
}

/// 吸着なしの窓 2 つ（スコープ 0・1）と吸着なしの窓 1 つ（スコープ 2）の World。
fn world_with_free_windows() -> (World, [Entity; 3]) {
    let mut world = World::new();
    let e = [
        spawn_char(&mut world, 0, Anchor::Free),
        spawn_char(&mut world, 1, Anchor::Free),
        spawn_char(&mut world, 2, Anchor::Free),
    ];
    (world, e)
}

fn tokens(raw: &[&str]) -> Vec<String> {
    raw.iter().map(|t| t.to_string()).collect()
}

fn members(scopes: &[u32]) -> BTreeSet<u32> {
    scopes.iter().copied().collect()
}

fn groups(world: &World) -> Vec<StickyGroup> {
    world.resource::<StickyGroupLedger>().groups().to_vec()
}

fn events_of<'a>(events: &'a [LogEvent], event: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

fn one<'a>(events: &'a [LogEvent], event: &str) -> &'a LogEvent {
    let hits = events_of(events, event);
    assert_eq!(
        hits.len(),
        1,
        "`{event}` がちょうど 1 件ではない: {events:?}"
    );
    hits[0]
}

/// 台本の組を作ると帳簿に入り、組の一覧が記録され、両方の `move_window` が偽になる（3.1・9.2）。
#[test]
fn set_sticky_adds_group_logs_and_resyncs_writer() {
    let (mut world, e) = world_with_free_windows();
    seed_sticky_descript(&mut world, None);

    let (ok, events) = capture_logs(|| apply_set_sticky(&mut world, &tokens(&["1", " 0"])));
    assert!(ok);
    assert_eq!(
        groups(&world),
        vec![StickyGroup {
            members: members(&[0, 1]),
            source: StickySource::Script,
        }]
    );
    let logged = one(&events, "sticky_groups");
    assert_eq!(logged.level, tracing::Level::INFO);
    assert_eq!(logged.field_str("action"), Some("set"));
    let shown = logged.field("groups").unwrap();
    assert!(shown.contains("Script") && shown.contains("0") && shown.contains("1"));
    assert!(events_of(&events, "sticky_group_rejected").is_empty());
    assert!(events_of(&events, "sticky_member_absent").is_empty());

    assert!(!move_window(&world, e[0]));
    assert!(!move_window(&world, e[1]));
    assert!(move_window(&world, e[2]));
}

/// 重なり・2 未満・重複・数でない引数は帳簿を変えず、出どころ・引数・理由を `warn!` に出す
/// （3.4・3.5・9.2）。拒否のときも組の一覧を記録する。
#[test]
fn set_sticky_rejects_without_change_and_logs_reason() {
    let (mut world, e) = world_with_free_windows();
    seed_sticky_descript(&mut world, Some("0,1"));
    let before = groups(&world);

    let cases: [(&[&str], &str); 4] = [
        (&["1", "2"], "AlreadyGrouped"),
        (&["2"], "TooFewScopes"),
        (&["2", "2"], "DuplicateScope"),
        (&["2", "x"], "UnparsableToken"),
    ];
    for (raw, reason) in cases {
        let (ok, events) = capture_logs(|| apply_set_sticky(&mut world, &tokens(raw)));
        assert!(!ok, "{raw:?}");
        assert_eq!(groups(&world), before, "{raw:?}");
        let rejected = one(&events, "sticky_group_rejected");
        assert_eq!(rejected.level, tracing::Level::WARN);
        assert_eq!(rejected.field_str("source"), Some("script"));
        assert!(rejected.field("tokens").unwrap().contains(raw[0]));
        assert!(
            rejected.field("reason").unwrap().contains(reason),
            "{raw:?}"
        );
        assert_eq!(
            one(&events, "sticky_groups").field_str("action"),
            Some("set")
        );
    }
    // descript の組の窓は areka が書くまま・組の外の窓は wintf が書くまま。
    assert!(!move_window(&world, e[0]));
    assert!(move_window(&world, e[2]));
}

/// 窓の無いスコープを含む組は受け付け、そのスコープを `warn!` に出す（3.6）。
#[test]
fn set_sticky_accepts_absent_scope_with_warning() {
    let (mut world, e) = world_with_free_windows();
    seed_sticky_descript(&mut world, None);

    let (ok, events) = capture_logs(|| apply_set_sticky(&mut world, &tokens(&["2", "5"])));
    assert!(ok);
    assert_eq!(groups(&world)[0].members, members(&[2, 5]));
    let absent = one(&events, "sticky_member_absent");
    assert_eq!(absent.level, tracing::Level::WARN);
    assert_eq!(absent.field("scope"), Some("5"));
    assert_eq!(absent.field_str("when"), Some("set"));
    assert!(!move_window(&world, e[2]));
}

/// 解除は台本の組だけを消し、descript の組を残して記録し、書き手を付け直す（3.7・9.2）。
#[test]
fn reset_sticky_removes_script_groups_only() {
    let mut world = World::new();
    let e: Vec<Entity> = (0..4)
        .map(|s| spawn_char(&mut world, s, Anchor::Free))
        .collect();
    seed_sticky_descript(&mut world, Some("0,1"));
    assert!(apply_set_sticky(&mut world, &tokens(&["2", "3"])));
    assert!(!move_window(&world, e[2]));

    let (ok, events) = capture_logs(|| apply_reset_sticky(&mut world));
    assert!(ok);
    assert_eq!(
        groups(&world),
        vec![StickyGroup {
            members: members(&[0, 1]),
            source: StickySource::Descript,
        }]
    );
    let logged = one(&events, "sticky_groups");
    assert_eq!(logged.field_str("action"), Some("reset"));
    assert!(logged.field("groups").unwrap().contains("Descript"));
    assert!(!move_window(&world, e[0]));
    assert!(move_window(&world, e[2]));
    assert!(move_window(&world, e[3]));
}

/// 起動では帳簿を据え、descript の値を `,` で区切って同じ規則で読む（3.8・9.2）。
#[test]
fn seed_installs_ledger_from_descript_value() {
    let (mut world, e) = world_with_free_windows();
    let ((), events) = capture_logs(|| seed_sticky_descript(&mut world, Some(" 1 , 0 ")));
    assert_eq!(
        groups(&world),
        vec![StickyGroup {
            members: members(&[0, 1]),
            source: StickySource::Descript,
        }]
    );
    assert_eq!(
        one(&events, "sticky_groups").field_str("action"),
        Some("descript_seed")
    );
    assert!(!move_window(&world, e[0]));
    assert!(!move_window(&world, e[1]));
    assert!(move_window(&world, e[2]));
}

/// 値が無い・空なら組なしの帳簿を据える。読めない値は拒否を記録し、組なしの帳簿を据える（3.8）。
#[test]
fn seed_without_value_or_with_bad_value_installs_empty_ledger() {
    for raw in [None, Some(""), Some("  ")] {
        let (mut world, _) = world_with_free_windows();
        let ((), events) = capture_logs(|| seed_sticky_descript(&mut world, raw));
        assert!(groups(&world).is_empty(), "{raw:?}");
        assert!(events_of(&events, "sticky_group_rejected").is_empty());
        assert_eq!(
            one(&events, "sticky_groups").field_str("action"),
            Some("descript_seed")
        );
    }

    let (mut world, _) = world_with_free_windows();
    let ((), events) = capture_logs(|| seed_sticky_descript(&mut world, Some("0")));
    assert!(groups(&world).is_empty());
    let rejected = one(&events, "sticky_group_rejected");
    assert_eq!(rejected.field_str("source"), Some("descript"));
    assert!(rejected.field("reason").unwrap().contains("TooFewScopes"));
}

/// descript の組の窓の無いスコープも受け付けて `warn!` に出す（3.6）。
#[test]
fn seed_accepts_absent_scope_with_warning() {
    let (mut world, _) = world_with_free_windows();
    let ((), events) = capture_logs(|| seed_sticky_descript(&mut world, Some("0,7")));
    assert_eq!(groups(&world)[0].members, members(&[0, 7]));
    let absent = one(&events, "sticky_member_absent");
    assert_eq!(absent.field("scope"), Some("7"));
    assert_eq!(absent.field_str("when"), Some("seed"));
}

/// シェルの切り替えは台本の組を保ち descript の組だけを取り替える。台本の組と重なる
/// descript の組は作らずに拒否を記録する（3.9・6.5・9.2）。
#[test]
fn replace_keeps_script_groups_and_rejects_overlap() {
    let mut world = World::new();
    let e: Vec<Entity> = (0..4)
        .map(|s| spawn_char(&mut world, s, Anchor::Free))
        .collect();
    seed_sticky_descript(&mut world, Some("0,1"));
    assert!(apply_reset_sticky(&mut world));
    assert!(apply_set_sticky(&mut world, &tokens(&["2", "3"])));

    let ((), events) = capture_logs(|| replace_sticky_descript(&mut world, Some("1,0")));
    assert_eq!(
        one(&events, "sticky_groups").field_str("action"),
        Some("descript_replace")
    );
    assert_eq!(groups(&world).len(), 2);

    let ((), events) = capture_logs(|| replace_sticky_descript(&mut world, Some("1,2")));
    assert_eq!(
        groups(&world),
        vec![StickyGroup {
            members: members(&[2, 3]),
            source: StickySource::Script,
        }]
    );
    let rejected = one(&events, "sticky_group_rejected");
    assert_eq!(rejected.field_str("source"), Some("descript"));
    assert!(rejected.field("reason").unwrap().contains("AlreadyGrouped"));
    // 前の descript の組（0・1）は外れたので wintf が書く。
    assert!(move_window(&world, e[0]));
    assert!(move_window(&world, e[1]));
    assert!(!move_window(&world, e[2]));
}

/// 帳簿が無いときの指令と取り替えは `warn!` を残して捨てる（帳簿を作らない）。
#[test]
fn without_ledger_directives_are_dropped_with_warning() {
    let (mut world, _) = world_with_free_windows();
    let (ok, events) = capture_logs(|| apply_set_sticky(&mut world, &tokens(&["0", "1"])));
    assert!(!ok);
    assert_eq!(one(&events, "sticky_skipped").level, tracing::Level::WARN);
    let (ok, events) = capture_logs(|| apply_reset_sticky(&mut world));
    assert!(!ok);
    assert_eq!(one(&events, "sticky_skipped").level, tracing::Level::WARN);
    let ((), events) = capture_logs(|| replace_sticky_descript(&mut world, Some("0,1")));
    assert_eq!(one(&events, "sticky_skipped").level, tracing::Level::WARN);
    assert!(world.get_resource::<StickyGroupLedger>().is_none());
}
