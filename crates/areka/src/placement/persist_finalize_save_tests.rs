//! 並べ終えた時点の保存（[`persist_unremembered_char_positions`]）の分かれ道のテスト
//! （areka-P0-char-position-save-on-exit 要件 1.1・1.2・1.4・1.5・1.7・4.1・4.2・4.3・5.4・
//! design Testing Strategy「書く関数の分かれ道」1〜5）。
//!
//! 窓の一式は本番と同じ `spawn_ghost_windows` で組む（既定の位置の台帳も本番の経路で用意する）。
//! 記憶は実物のアクター（`spawn_sylphya`）と偽の保存先 `FakePersistIo` に置き、`barrier` の後に
//! `load_scope` で読み戻して確かめる。
//!
//! 「書かない」を確かめるときは、先に 1 回書いて中身を入れてから前後を比べる（空どうしの
//! 比較では何も書かなくても恒真で通る。`follow_drag_tests.rs` の
//! `non_dragend_operations_leave_persist_store_byte_invariant` と同じやり方）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope,
    spawn_sylphya,
};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::drag::DragEndEvent;
use wintf::ecs::pointer::Phase;
use wintf::ecs::window::drain_window_pos_commands;
use wintf::ecs::{Point, WindowHandle, WindowPos};

use super::{
    PersistWiring, balloon_offset_entries, char_pos_entries, persist_entries,
    persist_unremembered_char_positions,
};
use crate::placement::follow::{OffsetBase, on_char_drag_end};
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};
use crate::placement::test_support::{ExpectField, LogEvent, capture_logs};

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

/// 共有の偽の保存先（アクターへ渡す分と、観測する分が同じストアを指す）。
struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

struct Rig {
    world: World,
    shared: Arc<FakePersistIo>,
    roots: ScopeRoots,
    parts: SylphyaParts,
}

impl Rig {
    /// 3 スコープの窓の一式と、記憶の送り口（`wired` のときだけ）を持つ World を組む。
    fn new(wired: bool) -> Self {
        let shared = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(shared.clone())),
            runtime_sink: None,
        });
        let mut world = World::new();
        let placements: Vec<ScopePlacement> = SCOPES
            .iter()
            .map(|&(scope, char_pos, char_size)| ScopePlacement {
                scope,
                char_pos,
                char_size,
                balloon_pos: PointPx {
                    x: char_pos.x - 200,
                    y: char_pos.y,
                },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset: PointPx { x: -200, y: 0 },
                balloon_offset_base: OffsetBase::unpinned(PointPx { x: -200, y: 0 }),
                balloon_limit: false,
                anchor: Anchor::Bottom,
                balloon_keyword_base: None,
            })
            .collect();
        spawn_ghost_windows(
            &mut world,
            &placements,
            &GhostTitles::from_scope_titles(SCOPES.iter().map(|&(s, _, _)| (s, format!("s{s}")))),
        );
        if wired {
            world.insert_non_send(PersistWiring {
                publisher: parts.publisher.clone(),
            });
        }
        Rig {
            world,
            shared,
            roots,
            parts,
        }
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

    /// 送ったものの書き込みが済むのを待ってから、記憶を読み戻す。
    fn load(&self) -> HashMap<PersistKey, String> {
        self.parts
            .publisher
            .barrier()
            .expect("barrier はアクターが生きている間は返る");
        load_scope(
            PersistScope::Ghost,
            &self.roots,
            &SharedFakeIo(self.shared.clone()),
        )
        .into_iter()
        .collect()
    }

    fn finish(self) {
        self.parts.publisher.close();
        let _ = self.parts.handle.join();
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

fn window_pos(map: &HashMap<PersistKey, String>, scope: u32) -> Option<(String, String)> {
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

fn save_events(events: &[LogEvent]) -> Vec<&LogEvent> {
    events.iter().filter(|e| e.target == SAVE_TARGET).collect()
}

fn find<'a>(events: &'a [LogEvent], needle: &str, scope: usize) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| {
            e.target == SAVE_TARGET
                && e.message().contains(needle)
                && e.field("scope") == Some(scope.to_string().as_str())
        })
        .collect()
}

/// テスト 1（1.1・1.2・1.4・1.5）: 記憶に位置が無いスコープ 1・2 だけが書かれ、記憶に位置がある
/// スコープ 0 とほかの鍵（バルーンの相対位置・起動の記録）は前後で同じ。書いた値は窓の左上から
/// 下端の中央の x へ直した値。
#[test]
fn writes_only_unremembered_scopes_and_keeps_other_keys() {
    let mut rig = Rig::new(true);
    rig.seed(seed_entries());
    let before = rig.load();
    assert_eq!(before.len(), 2 + 6 + 1, "種が記憶に入っている: {before:?}");
    rig.mark_remembered(0);

    persist_unremembered_char_positions(&rig.world);
    let after = rig.load();

    assert_eq!(window_pos(&after, 1), Some(expected_saved(1)));
    assert_eq!(window_pos(&after, 2), Some(expected_saved(2)));
    // 値の手計算の確かめ（半分の切り捨てを含む）。
    assert_eq!(expected_saved(1), ("1239".to_string(), "400".to_string()));
    assert_eq!(expected_saved(2), ("818".to_string(), "500".to_string()));

    // 書いた鍵はスコープ 1・2 の位置の 4 つだけ。ほかは前と同じ。
    let mut rest = after.clone();
    for scope in [1, 2] {
        for axis in [Axis::X, Axis::Y] {
            rest.remove(&PersistKey::WindowPos { scope, axis });
        }
    }
    assert_eq!(rest, before, "書いた 4 つの鍵のほかは前後で同じ");
    assert_eq!(
        window_pos(&after, 0),
        Some((SENTINEL_POS.x.to_string(), SENTINEL_POS.y.to_string())),
        "記憶に位置があるスコープ 0 は書き換えない"
    );
    rig.finish();
}

/// テスト 2（4.2）: 1 つのスコープの窓の大きさを読めなくすると、そのスコープは `warn!` を
/// 出して飛ばし、残りのスコープは書く。
#[test]
fn unreadable_size_skips_that_scope_with_warn_and_writes_the_rest() {
    let mut rig = Rig::new(true);
    rig.seed(seed_entries());
    let before = rig.load();
    let unreadable = rig.char_window(1);
    rig.world.get_mut::<WindowPos>(unreadable).unwrap().size = None;

    let ((), events) = capture_logs(|| persist_unremembered_char_positions(&rig.world));
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

/// テスト 3: 記憶の送り口が無ければ panic せず `info!` を 1 行だけ出す（スコープごとの行は
/// 出さない）。`GhostWindows` が無い World でも panic しない（`debug!`）。
#[test]
fn without_persist_wiring_logs_one_info_and_does_not_panic() {
    let rig = Rig::new(false);
    let ((), events) = capture_logs(|| persist_unremembered_char_positions(&rig.world));
    let saves = save_events(&events);
    assert_eq!(saves.len(), 1, "送り口が無いときは 1 行だけ: {events:?}");
    assert_eq!(saves[0].level, tracing::Level::INFO);
    assert!(saves[0].message().contains("送り口が無い"));
    rig.finish();

    let mut rig = Rig::new(true);
    rig.world.remove_resource::<GhostWindows>();
    let ((), events) = capture_logs(|| persist_unremembered_char_positions(&rig.world));
    let saves = save_events(&events);
    assert_eq!(
        saves.len(),
        1,
        "GhostWindows が無いときは 1 行だけ: {events:?}"
    );
    assert_eq!(saves[0].level, tracing::Level::DEBUG);
    rig.finish();
}

/// テスト 4（4.1・5.4）: 次の書き込みを失敗させても関数は戻り、記憶は前のまま。そのあとの
/// ドラッグの確定の書き込みは成功する（失敗は送った先の記録に残り、UI の側へは戻らない）。
#[test]
fn commit_failure_keeps_previous_store_and_later_drag_end_write_succeeds() {
    let mut rig = Rig::new(true);
    rig.seed(seed_entries());
    let before = rig.load();

    rig.shared.fail_next_commit();
    persist_unremembered_char_positions(&rig.world);
    let after_fail = rig.load();
    assert_eq!(after_fail, before, "失敗した書き込みは記憶を変えない");

    // そのあとのドラッグの確定（本番のハンドラ）で、スコープ 0 の位置を書く。
    let scope0 = rig.char_window(0);
    rig.world.entity_mut(scope0).insert(WindowHandle {
        hwnd: HWND(0x100 as *mut _),
        instance: HINSTANCE::default(),
    });
    rig.world.get_mut::<WindowPos>(scope0).unwrap().position = Some(Point { x: 1700, y: 356 });
    let ev = Phase::Bubble(DragEndEvent {
        target: scope0,
        position: Point::new(0, 0),
        cancelled: false,
        is_primary: true,
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

/// テスト 5（4.3）: 書いた行・記憶があるので書かなかった行が target `areka::persist::save` に
/// `info!` で出る。書いた行にはスコープ・窓の左上・記憶に書く値・窓の幅・揃え方が載る。
#[test]
fn written_and_skipped_scopes_are_logged_under_save_target() {
    let mut rig = Rig::new(true);
    rig.mark_remembered(0);

    let ((), events) = capture_logs(|| persist_unremembered_char_positions(&rig.world));

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

/// 1.7: 並べ終える前に動かされた（今の位置が既定の位置と違う）スコープは書かず、今の位置と
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

    let ((), events) = capture_logs(|| persist_unremembered_char_positions(&rig.world));
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

/// 1.7: 縦にだけ動かされたスコープも書かない。連鎖の起点（スコープ 0）は並べ直しが動かさない
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

    let ((), events) = capture_logs(|| persist_unremembered_char_positions(&rig.world));
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
