//! 並べ終えた時点の保存（[`persist_unremembered_char_positions`]）の分かれ道のテスト。
//!
//! 窓の一式は本番と同じ `spawn_ghost_windows` で組む（既定の位置の台帳も本番の経路で用意する）。
//! 記憶は実物のアクター（`spawn_sylphya`）と偽の保存先 `FakePersistIo` に置き、`barrier` の後に
//! `load_scope` で読み戻して確かめる。
//!
//! 「書かない」を確かめるときは、先に 1 回書いて中身を入れてから前後を比べる（空どうしの
//! 比較では何も書かなくても恒真で通る。`follow_drag_tests.rs` の
//! `non_dragend_operations_leave_persist_store_byte_invariant` と同じやり方）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use areka_sylphya::persist::FakePersistIo;
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope,
    spawn_sylphya,
};
use bevy_ecs::prelude::*;
use wintf::ecs::drag::DragEndEvent;
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{Point, WindowPos};

use super::{
    PersistWiring, balloon_offset_entries, char_pos_entries, persist_entries,
    persist_unremembered_char_positions,
};
use crate::chain_finalize::AdjacencyMark;
use crate::follow::{Anchored, on_char_drag_end};
use crate::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::shared_test_support::placement_sized;
use crate::source::GhostTitles;
use crate::spawn::{GhostWindows, spawn_ghost_windows};
use crate::test_support::{SharedFakeIo, fake_handle};
use log_capture_kit::{CapturedEvent, ExpectField, capture};

/// 記録の target（ドラッグの確定と同じ）。
const SAVE_TARGET: &str = "areka::persist::save";

/// 3 スコープの既定の配置（左上・大きさ）。幅はどれも 96 の倍数を避け、半分の切り捨てが
/// 効く奇数も混ぜる（下端の中央の x へ直す計算を通らずに緑になるのを防ぐ）。
const SCOPES: [(usize, PointPx, SizePx); 3] = [
    (0, PointPx { x: 1483, y: 356 }, SizePx { w: 434, h: 687 }),
    (1, PointPx { x: 1049, y: 400 }, SizePx { w: 381, h: 643 }),
    (2, PointPx { x: 668, y: 500 }, SizePx { w: 301, h: 543 }),
];

/// 記憶にある体の値（「書き換えない」を噛ませるための、どの書く値とも重ならない値）。
const SENTINEL_POS: PointPx = PointPx { x: 2222, y: 111 };

/// 記憶の実物のアクター（`spawn_sylphya`）と偽の保存先 `FakePersistIo`（Ghost の根は `/g`）。
/// 並べ直し・送り口・止めたバルーンのドラッグの檻（`reset_pos_tests.rs`・`persist_io_wiring_tests.rs`・
/// `balloon_lock_drag_tests.rs`）も使う。
pub(crate) struct Store {
    shared: Arc<FakePersistIo>,
    roots: ScopeRoots,
    parts: SylphyaParts,
}

impl Store {
    pub(crate) fn new() -> Self {
        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo::new(shared.clone())),
            runtime_sink: None,
        });
        Store {
            shared,
            roots,
            parts,
        }
    }

    /// 記憶の送り口を World へ挿す。
    pub(crate) fn wire(&self, world: &mut World) {
        world.insert_non_send(PersistWiring {
            publisher: self.parts.publisher.clone(),
        });
    }

    /// 送ったものの書き込みが済むのを待ってから、記憶を読み戻す。
    pub(crate) fn load(&self) -> HashMap<PersistKey, String> {
        self.parts
            .publisher
            .barrier()
            .expect("barrier はアクターが生きている間は返る");
        load_scope(
            PersistScope::Ghost,
            &self.roots,
            &SharedFakeIo::new(self.shared.clone()),
        )
        .into_iter()
        .collect()
    }

    pub(crate) fn finish(self) {
        self.parts.publisher.close();
        let _ = self.parts.handle.join();
    }
}

pub(super) struct Rig {
    pub(super) world: World,
    store: Store,
}

impl Rig {
    /// 3 スコープの窓の一式と、記憶の送り口（`wired` のときだけ）を持つ World を組む。
    fn new(wired: bool) -> Self {
        let store = Store::new();
        let mut world = World::new();
        let placements: Vec<ScopePlacement> = SCOPES
            .iter()
            .map(|&(scope, char_pos, char_size)| ScopePlacement {
                anchor: Anchor::Bottom,
                ..placement_sized(
                    scope,
                    char_pos,
                    PointPx { x: -200, y: 0 },
                    false,
                    char_size,
                    SizePx { w: 223, h: 158 },
                )
            })
            .collect();
        spawn_ghost_windows(
            &mut world,
            &placements,
            &GhostTitles::from_scope_titles(SCOPES.iter().map(|&(s, _, _)| (s, format!("s{s}")))),
        );
        if wired {
            store.wire(&mut world);
        }
        Rig { world, store }
    }

    fn char_window(&self, scope: usize) -> Entity {
        self.world
            .resource::<GhostWindows>()
            .char_window(scope)
            .expect("char 窓がある")
    }

    /// 記憶に位置があるスコープとして標す（本番の起動の `clear_default_char_pos` と同じ口）。
    fn mark_remembered(&mut self, scope: usize) {
        assert!(
            self.world
                .resource_mut::<GhostWindows>()
                .clear_default_char_pos(scope)
        );
    }

    /// 先に中身を入れる（「書かない」を噛ませるための種）。
    fn seed(&self, entries: Vec<(PersistKey, String)>) {
        persist_entries(&self.world, entries);
    }

    pub(super) fn load(&self) -> HashMap<PersistKey, String> {
        self.store.load()
    }

    pub(super) fn finish(self) {
        self.store.finish();
    }
}

/// 種: スコープ 0 の記憶の位置・全スコープのバルーンの相対位置・起動の記録。
fn seed_entries() -> Vec<(PersistKey, String)> {
    let mut entries = char_pos_entries(0, SENTINEL_POS);
    for &(scope, _, _) in &SCOPES {
        entries.extend(balloon_offset_entries(
            scope as u32,
            PointPx {
                x: -300 - scope as i32,
                y: 7,
            },
        ));
    }
    entries.push((PersistKey::BootCount, "5".to_string()));
    entries
}

/// 読み戻した記憶からスコープの窓の位置（x・y の両方）を取る（並べ直しの檻も使う）。
pub(crate) fn window_pos(
    map: &HashMap<PersistKey, String>,
    scope: u32,
) -> Option<(String, String)> {
    Some((
        map.get(&PersistKey::WindowPos {
            scope,
            axis: Axis::X,
        })?
        .clone(),
        map.get(&PersistKey::WindowPos {
            scope,
            axis: Axis::Y,
        })?
        .clone(),
    ))
}

/// 窓の左上から、記憶に書く値（下端の中央の x・y は左上のまま）を手で出す。
fn expected_saved(scope: usize) -> (String, String) {
    let (_, pos, size) = SCOPES[scope];
    ((pos.x + size.w / 2).to_string(), pos.y.to_string())
}

fn save_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events.iter().filter(|e| e.target == SAVE_TARGET).collect()
}

pub(super) fn find<'a>(
    events: &'a [CapturedEvent],
    needle: &str,
    scope: usize,
) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| {
            e.target == SAVE_TARGET
                && e.message().contains(needle)
                && e.field("scope") == Some(scope.to_string().as_str())
        })
        .collect()
}

/// 記憶に位置が無いスコープ 1・2 だけが書かれ、記憶に位置がある
/// スコープ 0 とほかの鍵（バルーンの相対位置・起動の記録）は前後で同じ。書いた値は窓の左上から
/// 下端の中央の x へ直した値。
#[test]
fn writes_only_unremembered_scopes_and_keeps_other_keys() {
    let mut rig = Rig::new(true);
    rig.seed(seed_entries());
    let mut before = rig.load();
    // 種のスコープ 0 の位置が組 (0,1) の印を連れてくる。
    assert_eq!(
        before.len(),
        2 + 6 + 1 + 1,
        "種が記憶に入っている: {before:?}"
    );
    rig.mark_remembered(0);

    persist_unremembered_char_positions(&rig.world);
    let after = rig.load();

    assert_eq!(window_pos(&after, 1), Some(expected_saved(1)));
    assert_eq!(window_pos(&after, 2), Some(expected_saved(2)));
    // 値の手計算の確かめ（半分の切り捨てを含む）。
    assert_eq!(expected_saved(1), ("1239".to_string(), "400".to_string()));
    assert_eq!(expected_saved(2), ("818".to_string(), "500".to_string()));

    // 書いた鍵はスコープ 1・2 の位置の 4 つと、触れる組 (0,1)・(1,2) の隣り合いの印の 2 つだけ。SCOPES の並びはどの組も離れているので印は `none`。
    // ほかは前と同じ。
    let mut rest = after.clone();
    for scope in [1, 2] {
        for axis in [Axis::X, Axis::Y] {
            rest.remove(&PersistKey::WindowPos { scope, axis });
        }
        assert_eq!(
            rest.remove(&PersistKey::WindowAdjacent { scope })
                .as_deref(),
            Some("none"),
            "スコープ {scope} の隣り合いの印"
        );
    }
    before.remove(&PersistKey::WindowAdjacent { scope: 1 });
    assert_eq!(rest, before, "書いた 6 つの鍵のほかは前後で同じ");
    assert_eq!(
        window_pos(&after, 0),
        Some((SENTINEL_POS.x.to_string(), SENTINEL_POS.y.to_string())),
        "記憶に位置があるスコープ 0 は書き換えない"
    );
    rig.finish();
}

/// 面の `move` のずれが当たったキャラ窓も、ずれを引いた元の位置（既定の位置）として書く
/// （ずれが起動のたびに積もらない）。
#[test]
fn pattern_shift_is_subtracted_before_saving() {
    let mut rig = Rig::new(true);
    rig.world.insert_resource(crate::follow::MonitorSnapshot {
        work_areas: vec![crate::resolver::RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1043,
        }],
    });
    let shifted = rig.char_window(1);
    // 窓へ書く口は実窓の印を要る（偽 HWND）。
    rig.world.entity_mut(shifted).insert(fake_handle(0x200));
    assert!(crate::follow::apply_char_shift(
        &mut rig.world,
        shifted,
        (12, 0),
        areka_emo_compose::ScaleRatio::ONE
    ));
    assert_eq!(
        rig.world.get::<WindowPos>(shifted).unwrap().position,
        Some(Point {
            x: SCOPES[1].1.x + 12,
            y: SCOPES[1].1.y
        })
    );

    persist_unremembered_char_positions(&rig.world);
    let after = rig.load();

    assert_eq!(window_pos(&after, 1), Some(expected_saved(1)));
    rig.finish();
}

/// 1 つのスコープの窓の大きさを読めなくすると、そのスコープは `warn!` を
/// 出して飛ばし、残りのスコープは書く。
#[test]
fn unreadable_size_skips_that_scope_with_warn_and_writes_the_rest() {
    let mut rig = Rig::new(true);
    rig.seed(seed_entries());
    let before = rig.load();
    let unreadable = rig.char_window(1);
    rig.world.get_mut::<WindowPos>(unreadable).unwrap().size = None;

    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let after = rig.load();

    let warns = find(&events, "読めないので飛ばす", 1);
    assert_eq!(warns.len(), 1, "スコープ 1 の warn が 1 行: {events:?}");
    assert_eq!(warns[0].level, tracing::Level::WARN);
    assert!(
        warns[0].expect_field("missing").contains("大きさ"),
        "読めなかったものが載る: {:?}",
        warns[0].fields_map()
    );
    assert_eq!(window_pos(&after, 0), Some(expected_saved(0)));
    assert_eq!(window_pos(&after, 2), Some(expected_saved(2)));
    assert_eq!(
        window_pos(&after, 1),
        window_pos(&before, 1),
        "飛ばしたスコープは書かない"
    );
    rig.finish();
}

/// 記憶の送り口が無ければ panic せず `info!` を 1 行だけ出す（スコープごとの行は
/// 出さない）。`GhostWindows` が無い World でも panic しない（`debug!`）。
#[test]
fn without_persist_wiring_logs_one_info_and_does_not_panic() {
    let rig = Rig::new(false);
    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let saves = save_events(&events);
    assert_eq!(saves.len(), 1, "送り口が無いときは 1 行だけ: {events:?}");
    assert_eq!(saves[0].level, tracing::Level::INFO);
    assert!(saves[0].message().contains("送り口が無い"));
    rig.finish();

    let mut rig = Rig::new(true);
    rig.world.remove_resource::<GhostWindows>();
    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let saves = save_events(&events);
    assert_eq!(
        saves.len(),
        1,
        "GhostWindows が無いときは 1 行だけ: {events:?}"
    );
    assert_eq!(saves[0].level, tracing::Level::DEBUG);
    rig.finish();
}

/// 次の書き込みを失敗させても関数は戻り、記憶は前のまま。そのあとの
/// ドラッグの確定の書き込みは成功する（失敗は送った先の記録に残り、UI の側へは戻らない）。
#[test]
fn commit_failure_keeps_previous_store_and_later_drag_end_write_succeeds() {
    let mut rig = Rig::new(true);
    rig.seed(seed_entries());
    let before = rig.load();

    rig.store.shared.fail_next_commit();
    persist_unremembered_char_positions(&rig.world);
    let after_fail = rig.load();
    assert_eq!(after_fail, before, "失敗した書き込みは記憶を変えない");

    // そのあとのドラッグの確定（本番のハンドラ）で、スコープ 0 の位置を書く。
    let scope0 = rig.char_window(0);
    rig.world.entity_mut(scope0).insert(fake_handle(0x100));
    rig.world.get_mut::<WindowPos>(scope0).unwrap().position = Some(Point { x: 1700, y: 356 });
    let ev = Phase::Bubble(DragEndEvent {
        target: scope0,
        position: Point::new(0, 0),
        cancelled: false,
        timestamp: Instant::now(),
    });
    assert!(!on_char_drag_end(&mut rig.world, scope0, scope0, &ev));
    // 窓へ届ける指令は捨てる（実 SetWindowPos を呼ばない）。
    let _residue = drain_window_pos_commands();

    let after_drag = rig.load();
    assert_eq!(
        window_pos(&after_drag, 0),
        Some(((1700 + 434 / 2).to_string(), "356".to_string())),
        "失敗のあとのドラッグの確定の書き込みは成功する"
    );
    assert_eq!(
        window_pos(&after_drag, 1),
        None,
        "失敗した回の位置は書かれていない"
    );
    rig.finish();
}

/// 書いた行・記憶があるので書かなかった行が target `areka::persist::save` に
/// `info!` で出る。書いた行にはスコープ・窓の左上・記憶に書く値・窓の幅・揃え方が載る。
#[test]
fn written_and_skipped_scopes_are_logged_under_save_target() {
    let mut rig = Rig::new(true);
    rig.mark_remembered(0);

    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));

    let skipped = find(&events, "記憶に位置があるので書かない", 0);
    assert_eq!(skipped.len(), 1, "スコープ 0 の書かない行: {events:?}");
    assert_eq!(skipped[0].level, tracing::Level::INFO);

    for scope in [1, 2] {
        let written = find(&events, "書いた", scope);
        assert_eq!(written.len(), 1, "スコープ {scope} の書いた行: {events:?}");
        let ev = written[0];
        assert_eq!(ev.level, tracing::Level::INFO);
        let (_, pos, size) = SCOPES[scope];
        let (saved_x, saved_y) = expected_saved(scope);
        assert_eq!(ev.expect_field("char_x"), pos.x.to_string());
        assert_eq!(ev.expect_field("char_y"), pos.y.to_string());
        assert_eq!(ev.expect_field("saved_x"), saved_x);
        assert_eq!(ev.expect_field("saved_y"), saved_y);
        assert_eq!(ev.expect_field("char_w"), size.w.to_string());
        assert_eq!(ev.expect_field("anchor"), "Bottom");
    }
    assert!(
        find(&events, "書いた", 0).is_empty(),
        "記憶があるスコープの書いた行は出ない"
    );
    rig.finish();
}

/// 並べ終える前に動かされた（今の位置が既定の位置と違う）スコープは書かず、今の位置と
/// 既定の位置を `info!` に残す。残りは書く。
#[test]
fn scope_moved_before_finalize_is_not_written() {
    let mut rig = Rig::new(true);
    // スコープ 1 にも中身を入れておく（書けば値が変わる＝噛む）。
    let mut seed = seed_entries();
    seed.extend(char_pos_entries(1, SENTINEL_POS));
    rig.seed(seed);
    let before = rig.load();
    let moved = rig.char_window(1);
    rig.world.get_mut::<WindowPos>(moved).unwrap().position = Some(Point { x: 999, y: 400 });
    rig.mark_remembered(0);

    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let after = rig.load();

    assert_eq!(window_pos(&after, 1), window_pos(&before, 1));
    assert_eq!(window_pos(&after, 2), Some(expected_saved(2)));
    let lines = find(&events, "並べ終える前に動かされた", 1);
    assert_eq!(lines.len(), 1, "スコープ 1 の書かない行: {events:?}");
    assert_eq!(lines[0].level, tracing::Level::INFO);
    assert_eq!(lines[0].expect_field("current_x"), "999");
    assert_eq!(lines[0].expect_field("default_x"), "1049");
    rig.finish();
}

/// 縦にだけ動かされたスコープも書かない。連鎖の起点（スコープ 0）は並べ直しが動かさない
/// ので、台本の移動の指示が縦にだけ動かすと x は既定の位置のまま残る。x だけで比べると
/// 書いてしまう。
#[test]
fn scope_moved_only_vertically_before_finalize_is_not_written() {
    let mut rig = Rig::new(true);
    // スコープ 0・1 に中身を入れておく（書けば値が変わる＝噛む）。
    let mut seed = seed_entries();
    seed.extend(char_pos_entries(1, SENTINEL_POS));
    rig.seed(seed);
    let before = rig.load();
    let moved = rig.char_window(0);
    rig.world.get_mut::<WindowPos>(moved).unwrap().position = Some(Point { x: 1483, y: 300 });

    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let after = rig.load();

    assert_eq!(window_pos(&after, 0), window_pos(&before, 0));
    assert_eq!(window_pos(&after, 1), Some(expected_saved(1)));
    assert_eq!(window_pos(&after, 2), Some(expected_saved(2)));
    let lines = find(&events, "並べ終える前に動かされた", 0);
    assert_eq!(lines.len(), 1, "スコープ 0 の書かない行: {events:?}");
    assert_eq!(lines[0].level, tracing::Level::INFO);
    assert_eq!(lines[0].expect_field("current_y"), "300");
    assert_eq!(lines[0].expect_field("default_y"), "356");
    rig.finish();
}

/// 台本で今の向きを `free` にした窓も、書く形は descript の向き（下）で決まり、
/// 下端の中央の x で書かれる（今の向きの形＝左上のままでは書かない）。
#[test]
fn writes_in_descript_form_even_when_current_anchor_is_free() {
    let mut rig = Rig::new(true);
    let free = rig.char_window(1);
    rig.world.entity_mut(free).insert(Anchored(Anchor::Free));

    persist_unremembered_char_positions(&rig.world);
    let after = rig.load();

    assert_eq!(window_pos(&after, 1), Some(expected_saved(1)));
    rig.finish();
}

/// 全スコープが記憶から戻り、スコープ 1・2 が位置だけの古い記憶（印なし）で、スコープ 2 は覚えた
/// 印を持ち、スコープ 1 がスコープ 0 の左に隣り合う rig と、種を入れた直後の記憶。種は印を連れて
/// こないよう送り口へ直接入れる（`persist_restore_group_tests.rs` の寄せた起動の檻も使う）。
pub(super) fn old_memory_rig() -> (Rig, HashMap<PersistKey, String>) {
    let mut rig = Rig::new(true);
    let mut seed = seed_entries();
    for scope in [1, 2] {
        seed.extend(char_pos_entries(scope, SENTINEL_POS));
    }
    rig.store
        .parts
        .publisher
        .persist_put(PersistScope::Ghost, seed);
    let before = rig.load();
    assert!(
        !before
            .keys()
            .any(|k| matches!(k, PersistKey::WindowAdjacent { .. })),
        "種は印の無い古い記憶: {before:?}"
    );
    for scope in [0, 1, 2] {
        rig.mark_remembered(scope);
    }
    assert!(
        rig.world
            .resource_mut::<GhostWindows>()
            .set_remembered_adjacency(2, Some(AdjacencyMark::Separate))
    );
    // スコープ 1 の右端をスコープ 0 の左端（1483）へ付ける＝左に隣り合う。
    let entity = rig.char_window(1);
    rig.world
        .get_mut::<WindowPos>(entity)
        .unwrap()
        .position
        .as_mut()
        .unwrap()
        .x = 1483 - 381;
    (rig, before)
}

/// 印の無い古い記憶のスコープ 1 には、位置を
/// 書かずに画面の並びから決めた印だけが書き足され、`info!` が残る。覚えた印のあるスコープ 2 と
/// スコープ 0（前の窓が無い）には書き足さない。
#[test]
fn old_memory_gets_only_the_adjacency_mark_added() {
    let (rig, before) = old_memory_rig();

    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let mut after = rig.load();

    assert_eq!(
        after
            .remove(&PersistKey::WindowAdjacent { scope: 1 })
            .as_deref(),
        Some("left"),
        "印だけが書き足される"
    );
    assert_eq!(after, before, "印のほか（位置を含む）は前後で同じ");
    let infos = find(&events, "古い記憶に隣り合いの印だけを書き足した", 1);
    assert_eq!(infos.len(), 1, "書き足しの info が 1 行: {events:?}");
    assert_eq!(infos[0].level, tracing::Level::INFO);
    assert_eq!(infos[0].field("mark"), Some("left"));
    assert!(
        find(&events, "古い記憶に隣り合いの印だけを書き足した", 2).is_empty(),
        "覚えた印のあるスコープには書き足さない"
    );
    rig.finish();
}

/// 記憶の位置を寄せて表示した起動では、隣の窓（記憶に位置が
/// 無いスコープ 0）の位置を書く束からも、印の無い古い記憶（スコープ 1）へ印を書き足さない。
/// スコープ 0 の位置は書く。
#[test]
fn clamped_boot_adds_no_mark_to_old_memory_through_the_neighbours_position() {
    let mut rig = Rig::new(true);
    rig.mark_remembered(1);
    rig.mark_remembered(2);
    rig.world
        .insert_resource(super::RestoredPositionsClamped(true));

    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let after = rig.load();

    assert_eq!(
        window_pos(&after, 0),
        Some(expected_saved(0)),
        "スコープ 0 は書く"
    );
    let lines = save_events(&events)
        .into_iter()
        .filter(|e| e.message().contains("古い記憶へ隣り合いの印を書き足さない"))
        .count();
    assert_eq!(lines, 1, "書き足さない決めの記録は 1 行: {events:?}");
    assert_eq!(
        after.get(&PersistKey::WindowAdjacent { scope: 1 }),
        None,
        "寄せた起動で古い記憶へ印が書き足された: {after:?}"
    );
    rig.finish();
}

/// 寄せて表示した起動のあいだは、ドラッグや連れ動きの保存
/// （[`persist_entries`]）の束からも、印の無い古い記憶（スコープ 1）へ印を書き足さず、外した印の
/// スコープ（スコープ 2 は束に触れないので載らない）を 1 行に残す。寄せずに起動した回は、同じ束が
/// 印を書き、行は出ない（飛ばすのは寄せた起動だけ）。
#[test]
fn clamped_boot_drag_save_adds_no_mark_to_old_memory() {
    for (clamped, expected) in [(false, true), (true, false)] {
        let mut rig = Rig::new(true);
        rig.mark_remembered(1);
        rig.mark_remembered(2);
        rig.world
            .insert_resource(super::RestoredPositionsClamped(clamped));

        let ((), events) =
            capture(|| persist_entries(&rig.world, char_pos_entries(0, SENTINEL_POS)));
        let after = rig.load();

        assert!(window_pos(&after, 0).is_some(), "スコープ 0 の位置は書く");
        let lines: Vec<_> = save_events(&events)
            .into_iter()
            .filter(|e| e.message().contains("古い記憶へ隣り合いの印を書き足さない"))
            .map(|e| e.field("scopes"))
            .collect();
        let expected_lines = if clamped { vec![Some("{1}")] } else { vec![] };
        assert_eq!(
            lines, expected_lines,
            "寄せた起動={clamped}: 外した印の記録"
        );
        assert_eq!(
            after.contains_key(&PersistKey::WindowAdjacent { scope: 1 }),
            expected,
            "寄せた起動={clamped}: {after:?}"
        );
        rig.finish();
    }
}

/// 飛ばすのは印の無い古い記憶だけ: 寄せた起動でも、同じ束が位置を書くスコープ 1 の印は
/// 書く（その記憶はもう古くない）。隣の古い記憶のスコープ 2 の印は書かない。
#[test]
fn clamped_boot_keeps_the_mark_of_the_scope_whose_position_is_written() {
    let mut rig = Rig::new(true);
    rig.mark_remembered(1);
    rig.mark_remembered(2);
    rig.world
        .insert_resource(super::RestoredPositionsClamped(true));

    persist_entries(&rig.world, char_pos_entries(1, SENTINEL_POS));
    let after = rig.load();

    assert!(
        after.contains_key(&PersistKey::WindowAdjacent { scope: 1 }),
        "位置を書くスコープの印は書く: {after:?}"
    );
    assert_eq!(
        after.get(&PersistKey::WindowAdjacent { scope: 2 }),
        None,
        "隣の古い記憶へは書かない: {after:?}"
    );
    rig.finish();
}

/// 古い記憶へ印を書き足す道も、印は descript の `defaultx` を鎖の規則に入れて決める: スコープ 1 が
/// スコープ 0 の左に `kero.defaultx` の 36 px だけ空けて並ぶと `left`。
#[test]
fn old_memory_mark_uses_the_defaultx_gap_of_the_descript() {
    let (mut rig, _) = old_memory_rig();
    super::adjacency_tests::with_basis(&mut rig.world, &[("kero.defaultx", "36")]);
    let entity = rig.char_window(1);
    rig.world
        .get_mut::<WindowPos>(entity)
        .unwrap()
        .position
        .as_mut()
        .unwrap()
        .x = 1483 - 381 - 36;

    persist_unremembered_char_positions(&rig.world);
    let after = rig.load();

    assert_eq!(
        after
            .get(&PersistKey::WindowAdjacent { scope: 1 })
            .map(String::as_str),
        Some("left"),
        "{after:?}"
    );
    rig.finish();
}

/// 1 回の並べ終えた時点の保存で、descript の読めない値の `warn!` は 1 度だけ（位置を書くスコープ 0 と
/// 印だけを書き足す古い記憶のスコープ 1 が同じ回にあっても、鎖の見方を組むのは 1 度）。
#[test]
fn one_finalize_save_warns_once_for_an_unreadable_defaultx() {
    let mut rig = Rig::new(true);
    rig.mark_remembered(1);
    rig.mark_remembered(2);
    assert!(
        rig.world
            .resource_mut::<GhostWindows>()
            .set_remembered_adjacency(2, Some(AdjacencyMark::Separate))
    );
    super::adjacency_tests::with_basis(&mut rig.world, &[("kero.defaultx", "abc")]);

    let ((), events) = capture(|| persist_unremembered_char_positions(&rig.world));
    let after = rig.load();

    assert!(window_pos(&after, 0).is_some(), "スコープ 0 は書く");
    assert!(
        after.contains_key(&PersistKey::WindowAdjacent { scope: 1 }),
        "古い記憶のスコープ 1 へ印を書き足す: {after:?}"
    );
    let warns = events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::WARN
                && e.message().contains("配置キーの数値化に失敗")
                && e.field_str("key") == Some("defaultx/defaultleft")
        })
        .count();
    assert_eq!(warns, 1, "{events:?}");
    rig.finish();
}
