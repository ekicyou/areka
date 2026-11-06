//! 吸着の指令（[`super`]）の檻（areka-P0-window-link-directives 要件 1.1〜1.3・1.5〜1.9・
//! 2.4・9.1・design「script_alignment」）。
//!
//! 窓は `spawn_ghost_windows` で本番と同じ部品のまま組み、偽の `WindowHandle` を付けて
//! 単一の窓書込口が `WindowPos` を書けるようにする（実際の画面は使わない）。作業領域の
//! 4 辺は 96 の倍数を避け、隠れた再スケールがあれば一致が崩れる檻にする。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::drag::DragConfig;
use wintf::ecs::{Point, WindowHandle, WindowPos};

use super::super::follow::{AnchorSource, Anchored, MonitorSnapshot, OffsetBase, begin_char_drag};
use super::super::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use super::super::source::GhostTitles;
use super::super::spawn::{GhostWindows, spawn_ghost_windows};
use super::super::test_support::{ExpectField, LogEvent, capture_logs};
use super::*;

/// 作業領域（左 53・上 37・右 1877・下 1043）。
const WA: RectPx = RectPx {
    left: 53,
    top: 37,
    right: 1877,
    bottom: 1043,
};
const CHAR_POS: PointPx = PointPx { x: 731, y: 356 };
const CHAR_SIZE: SizePx = SizePx { w: 434, h: 687 };
// バルーンがどの辺へ沿っても作業領域の中に収まるずらし（limit の寄せと見分けるため）。
const OFFSET: PointPx = PointPx { x: 100, y: 40 };

/// スコープ 0 だけの、descript が下（`bottom`）の窓の一式を持つ World。
fn world() -> World {
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![WA],
    });
    let placement = ScopePlacement {
        scope: 0,
        char_pos: CHAR_POS,
        char_size: CHAR_SIZE,
        balloon_pos: PointPx {
            x: CHAR_POS.x + OFFSET.x,
            y: CHAR_POS.y + OFFSET.y,
        },
        balloon_size: SizePx { w: 223, h: 158 },
        balloon_offset: OFFSET,
        balloon_offset_base: OffsetBase::unpinned(OFFSET),
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    };
    let gw = spawn_ghost_windows(
        &mut world,
        &[placement],
        &GhostTitles::from_scope_titles([(0, "a".to_string())]),
    );
    for (raw, e) in [
        (0x100usize, gw.char_window(0).unwrap()),
        (0x110, gw.balloon_window(0).unwrap()),
    ] {
        world.entity_mut(e).insert(WindowHandle {
            hwnd: HWND(raw as *mut _),
            instance: HINSTANCE::default(),
        });
    }
    world
}

fn char_window(world: &World) -> Entity {
    world.resource::<GhostWindows>().char_window(0).unwrap()
}

fn balloon_window(world: &World) -> Entity {
    world.resource::<GhostWindows>().balloon_window(0).unwrap()
}

fn pos(world: &World, e: Entity) -> Point {
    world.get::<WindowPos>(e).unwrap().position.unwrap()
}

fn anchor(world: &World) -> Anchor {
    world.get::<Anchored>(char_window(world)).unwrap().0
}

fn source(world: &World) -> AnchorSource {
    *world.get::<AnchorSource>(char_window(world)).unwrap()
}

fn move_window(world: &World) -> bool {
    world
        .get::<DragConfig>(char_window(world))
        .unwrap()
        .move_window
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

fn apply(world: &mut World, scope: u32, word: AlignmentWord) -> (bool, Vec<LogEvent>) {
    capture_logs(|| apply_script_alignment(world, scope, &word))
}

/// バルーンは窓の左上からのずらしを保って付いて動く（1.6）。
fn assert_balloon_follows(world: &World) {
    let c = pos(world, char_window(world));
    assert_eq!(
        pos(world, balloon_window(world)),
        Point {
            x: c.x + OFFSET.x,
            y: c.y + OFFSET.y
        },
        "バルーンが窓に付いて動いていない"
    );
}

// ---------------------------------------------------------------------------
// 語の読み
// ---------------------------------------------------------------------------

/// 6 つの語を読み、前後の空白と大文字小文字の揺れを吸収する（1.1〜1.3）。
#[test]
fn parse_reads_six_words_ignoring_case_and_spaces() {
    let cases = [
        ("top", AlignmentWord::Anchor(Anchor::Top)),
        ("bottom", AlignmentWord::Anchor(Anchor::Bottom)),
        ("left", AlignmentWord::Anchor(Anchor::Left)),
        ("right", AlignmentWord::Anchor(Anchor::Right)),
        ("free", AlignmentWord::Anchor(Anchor::Free)),
        ("default", AlignmentWord::Default),
        (" TOP ", AlignmentWord::Anchor(Anchor::Top)),
        ("Bottom", AlignmentWord::Anchor(Anchor::Bottom)),
        ("\tLeFt", AlignmentWord::Anchor(Anchor::Left)),
        ("RIGHT", AlignmentWord::Anchor(Anchor::Right)),
        ("Free ", AlignmentWord::Anchor(Anchor::Free)),
        ("DEFAULT", AlignmentWord::Default),
    ];
    for (raw, want) in cases {
        assert_eq!(parse_alignment_word(&[raw]), want, "{raw:?}");
    }
}

/// 6 つの語のどれでもない値は受け取った値のまま持つ（1.7）。
#[test]
fn parse_keeps_unknown_value_verbatim() {
    assert_eq!(
        parse_alignment_word(&["middle"]),
        AlignmentWord::Unknown("middle".to_string())
    );
    assert_eq!(
        parse_alignment_word(&["toppp"]),
        AlignmentWord::Unknown("toppp".to_string())
    );
}

/// 方向が無い・空なら欠けた値（1.9）。
#[test]
fn parse_missing_or_empty_value_is_missing() {
    assert_eq!(parse_alignment_word(&[]), AlignmentWord::Missing);
    assert_eq!(parse_alignment_word(&[""]), AlignmentWord::Missing);
    assert_eq!(parse_alignment_word(&["  "]), AlignmentWord::Missing);
}

/// 3 つ目以降の引数は読まずに `debug!` に残し、方向は最初の値で決まる。
#[test]
fn parse_extra_args_are_logged_and_ignored() {
    let (word, events) = capture_logs(|| parse_alignment_word(&["left", "x", "y"]));
    assert_eq!(word, AlignmentWord::Anchor(Anchor::Left));
    let e = one(&events, "alignment_extra_args");
    assert!(e.expect_field("extra").contains("\"x\""), "{e:?}");

    let (_, events) = capture_logs(|| parse_alignment_word(&["left"]));
    assert!(events_of(&events, "alignment_extra_args").is_empty());
}

// ---------------------------------------------------------------------------
// 窓への適用
// ---------------------------------------------------------------------------

/// 下から上へ: 上端へ沿い、x は残る。台本の印・記録・`move_window`（2.4）・バルーン（1.6）。
#[test]
fn bottom_to_top_pins_top_edge_keeps_x_and_logs() {
    let mut world = world();
    assert!(!move_window(&world), "前提: 下に吸着した窓は areka が書く");

    let (ok, events) = apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Top));

    assert!(ok);
    assert_eq!(anchor(&world), Anchor::Top);
    assert_eq!(
        source(&world),
        AnchorSource {
            descript: Anchor::Bottom,
            by_script: true
        }
    );
    assert_eq!(pos(&world, char_window(&world)), Point { x: 731, y: 37 });
    assert_balloon_follows(&world);
    assert!(!move_window(&world));
    let e = one(&events, "alignment_changed");
    assert_eq!(e.expect_field("scope"), "0");
    assert_eq!(e.expect_field("from"), "Bottom");
    assert_eq!(e.expect_field("to"), "Top");
    assert_eq!(e.field_str("source"), Some("script"));
}

/// 下から左・右へ: その辺へ沿い、y は残る（1.1）。
#[test]
fn bottom_to_left_and_right_pin_side_edges_keep_y() {
    let mut world = world();
    assert!(apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Left)).0);
    assert_eq!(pos(&world, char_window(&world)), Point { x: 53, y: 356 });
    assert_balloon_follows(&world);

    let mut world = self::world();
    assert!(apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Right)).0);
    assert_eq!(
        pos(&world, char_window(&world)),
        Point {
            x: 1877 - 434,
            y: 356
        }
    );
    assert_balloon_follows(&world);
}

/// `free` は吸着を外して窓をその場に留め、wintf が窓を動かす側へ戻る（1.2・2.4）。
#[test]
fn free_keeps_position_and_hands_drag_to_wintf() {
    let mut world = world();
    let (ok, events) = apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Free));
    assert!(ok);
    assert_eq!(anchor(&world), Anchor::Free);
    assert_eq!(pos(&world, char_window(&world)), Point { x: 731, y: 356 });
    assert_balloon_follows(&world);
    assert!(move_window(&world), "吸着なしで組に無い窓は wintf が動かす");
    assert_eq!(one(&events, "alignment_changed").expect_field("to"), "Free");
}

/// 読めない語は記録して `free` と同じに外す（1.7）。
#[test]
fn unknown_word_is_logged_and_treated_as_free() {
    let mut world = world();
    let (ok, events) = apply(&mut world, 0, AlignmentWord::Unknown("middle".to_string()));
    assert!(ok);
    assert_eq!(anchor(&world), Anchor::Free);
    assert!(source(&world).by_script);
    assert_eq!(pos(&world, char_window(&world)), Point { x: 731, y: 356 });
    assert_eq!(
        one(&events, "alignment_unknown_word").expect_field("word"),
        "middle"
    );
    assert_eq!(one(&events, "alignment_changed").expect_field("to"), "Free");
}

/// `default` は descript の向きへ戻して印を外し、その辺へ沿わせる（1.3・9.1）。
#[test]
fn default_returns_to_descript_anchor_and_clears_mark() {
    let mut world = world();
    assert!(apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Left)).0);
    // 左へ沿った後、上下を動かしておく（default で下端へ戻ることを見分けるため）。
    assert!(apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Top)).0);
    assert_eq!(pos(&world, char_window(&world)), Point { x: 53, y: 37 });

    let (ok, events) = apply(&mut world, 0, AlignmentWord::Default);

    assert!(ok);
    assert_eq!(anchor(&world), Anchor::Bottom);
    assert_eq!(
        source(&world),
        AnchorSource {
            descript: Anchor::Bottom,
            by_script: false
        }
    );
    assert_eq!(
        pos(&world, char_window(&world)),
        Point {
            x: 53,
            y: 1043 - 687
        }
    );
    assert_balloon_follows(&world);
    let e = one(&events, "alignment_changed");
    assert_eq!(e.expect_field("from"), "Top");
    assert_eq!(e.expect_field("to"), "Bottom");
    assert_eq!(e.field_str("source"), Some("descript"));
}

/// 経路は `MoveCue`（明示の操作）なので既定の位置を連れて行かない＝並べ終えた時点の保存は
/// この窓を記憶に書かない（1.6・design 議題 2）。
#[test]
fn alignment_move_does_not_carry_default_position() {
    let mut world = world();
    let before = world.resource::<GhostWindows>().default_char_pos(0);
    assert_eq!(before, Some(CHAR_POS));

    assert!(apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Top)).0);

    assert_eq!(world.resource::<GhostWindows>().default_char_pos(0), before);
    assert_ne!(
        pos(&world, char_window(&world)),
        Point {
            x: CHAR_POS.x,
            y: CHAR_POS.y
        }
    );
}

/// 欠けた値は何も変えずに記録する（1.9）。
#[test]
fn missing_value_changes_nothing_and_logs() {
    let mut world = world();
    let (ok, events) = apply(&mut world, 0, AlignmentWord::Missing);
    assert!(!ok);
    assert_eq!(anchor(&world), Anchor::Bottom);
    assert!(!source(&world).by_script);
    assert_eq!(pos(&world, char_window(&world)), Point { x: 731, y: 356 });
    assert_eq!(
        one(&events, "alignment_skipped").field_str("reason"),
        Some("missing_value")
    );
    assert!(events_of(&events, "alignment_changed").is_empty());
}

/// 窓の無いスコープ・窓の一式が無い・部品が無いは記録して何もしない（1.8）。
#[test]
fn absent_window_is_logged_and_skipped() {
    let mut world = world();
    let (ok, events) = apply(&mut world, 5, AlignmentWord::Anchor(Anchor::Top));
    assert!(!ok);
    assert_eq!(
        one(&events, "alignment_skipped").field_str("reason"),
        Some("no_char_window")
    );
    assert_eq!(anchor(&world), Anchor::Bottom);

    let e = char_window(&world);
    world.entity_mut(e).remove::<AnchorSource>();
    let (ok, events) = apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Top));
    assert!(!ok);
    assert_eq!(
        one(&events, "alignment_skipped").field_str("reason"),
        Some("no_anchor_source")
    );
    assert_eq!(anchor(&world), Anchor::Bottom);

    world.remove_resource::<GhostWindows>();
    let (ok, events) = apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Top));
    assert!(!ok);
    assert_eq!(
        one(&events, "alignment_skipped").field_str("reason"),
        Some("no_ghost_windows")
    );
}

/// 大きさが読めなければ向きだけ変えて記録し、窓は動かさない。
#[test]
fn unknown_size_changes_anchor_only_and_logs() {
    let mut world = world();
    let e = char_window(&world);
    world.get_mut::<WindowPos>(e).unwrap().size = None;

    let (ok, events) = apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Top));

    assert!(ok);
    assert_eq!(anchor(&world), Anchor::Top);
    assert_eq!(pos(&world, e), Point { x: 731, y: 356 });
    assert_eq!(
        one(&events, "alignment_reanchor_skipped").field_str("reason"),
        Some("char_size_unknown")
    );
    one(&events, "alignment_changed");
}

/// ドラッグ中の窓へ向きの指令が来たら、向き・台本の印・`move_window` は変えるが窓は当て直さず
/// （辺へ飛んで次の `OnDrag` で戻る 1 フレームのちらつきを出さない）、そのことを記録する。
/// 当て直しはドラッグの終わり（`finish_char_drag`）が今の向きで行う
/// （`follow_sticky_follow_drag_tests.rs` の `anchor_change_during_drag_keeps_start_until_finish_then_reanchors`）。
#[test]
fn dragging_window_changes_anchor_but_defers_reanchor_and_logs() {
    let mut world = world();
    let e = char_window(&world);
    begin_char_drag(&mut world, e);

    let (ok, events) = apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Free));
    assert!(ok);
    assert!(move_window(&world), "吸着なしで組に無い窓は wintf が動かす");
    let (ok, events2) = apply(&mut world, 0, AlignmentWord::Anchor(Anchor::Top));
    assert!(ok);

    assert_eq!(anchor(&world), Anchor::Top);
    assert_eq!(
        source(&world),
        AnchorSource {
            descript: Anchor::Bottom,
            by_script: true
        }
    );
    assert!(!move_window(&world));
    assert_eq!(
        pos(&world, e),
        Point { x: 731, y: 356 },
        "ドラッグ中は辺へ飛ばない"
    );
    for events in [&events, &events2] {
        assert_eq!(
            one(events, "alignment_reanchor_skipped").field_str("reason"),
            Some("dragging")
        );
        one(events, "alignment_changed");
    }
}
