//! 知らせの経路から保存まで（areka-P0-drag-click-without-move・design C4・T7-1〜T7-6）。
//!
//! wndproc の代わりに `DragAccumulatorResource::set_transition` で種を積み、
//! `dispatch_drag_events` を 1 回の画面更新として呼ぶ。受け手（`on_char_drag_end`・
//! `on_balloon_drag_end`）は `OnDragEnd` で結線し、保存は偽の記憶の書き手
//! （`FakePersistIo` の共有ストア）を `load_scope` で読んで数える。`FlushResult` の形には
//! 触れないので、修正の前の HEAD でもそのままコンパイルできる。
//!
//! - T7-1〜T7-3: 開始の無い終了（動かさないクリック・準備中の取り消し）で保存 0 件・
//!   窓の位置が変わらない・キーワードで決めたバルーンの位置の素材が残る（修正前は赤）。
//! - T7-4・T7-6: 本物のドラッグと閾値を越えた後の取り消しは今どおり 1 件（前後とも緑）。
//! - T7-5: 速いドラッグ（開始と終了が同じ配り）でキャラ窓が行き先へ置かれ 1 件保存
//!   （修正前は赤）。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    Axis, PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope,
    spawn_sylphya,
};
use bevy_ecs::message::Messages;
use bevy_ecs::prelude::*;
use wintf::ecs::drag::{
    DragAccumulatorResource, DragEndEvent, DragEvent, DragStartEvent, DragTransition, OnDragEnd,
    WindowDragContextResource, dispatch_drag_events,
};
use wintf::ecs::{PhysicalPoint, Point, Window, WindowPos};

use super::super::persist::PersistWiring;
use super::test_support::{fake_handle, position_of, rect, window_pos_at, window_pos_sized};
use super::{
    Anchored, BalloonFollow, MonitorSnapshot, OffsetBase, on_balloon_drag_end, on_char_drag_end,
    project_anchor,
};
use crate::placement::config::BalloonXMode;
use crate::placement::resolver::{Anchor, PointPx, SizePx};
use crate::placement::spawn::{BalloonKeywordBase, BalloonWindowMarker, CharWindowMarker};

/// 相方（scope 1）。0 以外にして既定値との取り違えを避ける。
const SCOPE: usize = 1;
/// キャラ窓の寸（emo2 実寸）。work area 下端 1200 → 接地 y = 1200 − 687 = 513。
const CHAR_SIZE: SizePx = SizePx { w: 434, h: 687 };
/// 押す前のキャラ窓の位置（既に接地済み・96 の非倍数）。
const CHAR_START: Point = Point { x: 1203, y: 513 };
/// 押す前のバルーン窓の位置。
const BALLOON_START: Point = Point { x: 1021, y: 541 };
/// 押したときのカーソル（スクリーン物理 px）。
const DRAG_START: (i32, i32) = (1307, 901);
/// 離したときのカーソル。
const CURSOR: (i32, i32) = (1711, 953);
/// キーワードで決めたバルーンの位置の素材（中身の値は判定に使わない）。
const KEYWORD_BASE: BalloonKeywordBase = BalloonKeywordBase {
    mode: BalloonXMode::CenterTop,
    adjust: PointPx { x: 13, y: -7 },
};

/// アクターへ渡す口と観測する口が同じストアを指す偽の IO。
struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// キャラ窓とバルーン窓を入れた World と、偽の記憶の書き手。
struct Rig {
    world: World,
    parts: SylphyaParts,
    roots: ScopeRoots,
    store: Arc<FakePersistIo>,
    char_w: Entity,
    balloon: Entity,
}

impl Rig {
    fn new() -> Self {
        let store = Arc::new(FakePersistIo::new());
        let roots = ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(store.clone())),
            runtime_sink: None,
        });

        // `dispatch_drag_events` が引くリソース（wintf の `EcsWorld::new` と同じ登録）。
        // 開始の腕は `Messages<DragStartEvent>` を `resource_mut` で引くので無いと落ちる。
        let mut world = World::new();
        world.insert_resource(DragAccumulatorResource::new());
        world.insert_resource(WindowDragContextResource::new());
        world.init_resource::<Messages<DragStartEvent>>();
        world.init_resource::<Messages<DragEvent>>();
        world.init_resource::<Messages<DragEndEvent>>();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![rect(0, 0, 1920, 1200)],
        });
        world.insert_non_send(PersistWiring {
            publisher: parts.publisher.clone(),
        });

        // 製品と同じく窓 entity 自身が `Window` を持つ（開始の腕が窓を見つけて
        // 偽の HWND で枠の座標変換を試みる経路を通す）。`WindowPos` は明示する
        // （`on_window_add` は既にあれば足さない）。
        let balloon = world
            .spawn((
                Window::default(),
                fake_handle(0x2000),
                window_pos_at(BALLOON_START.x, BALLOON_START.y),
                BalloonWindowMarker { scope: SCOPE },
                OnDragEnd(on_balloon_drag_end),
            ))
            .id();
        let char_w = world
            .spawn((
                Window::default(),
                fake_handle(0x1000),
                window_pos_sized(CHAR_START.x, CHAR_START.y, CHAR_SIZE.w, CHAR_SIZE.h),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: SCOPE },
                BalloonFollow::new(
                    balloon,
                    OffsetBase::unpinned(PointPx {
                        x: BALLOON_START.x - CHAR_START.x,
                        y: BALLOON_START.y - CHAR_START.y,
                    }),
                ),
                KEYWORD_BASE,
                OnDragEnd(on_char_drag_end),
            ))
            .id();

        Self {
            world,
            parts,
            roots,
            store,
            char_w,
            balloon,
        }
    }

    /// wndproc の代わりに種を積む（配りはしない）。
    fn put(&self, transition: DragTransition) {
        self.world
            .resource::<DragAccumulatorResource>()
            .set_transition(transition);
    }

    /// 1 回の画面更新として配る。
    fn tick(&mut self) {
        dispatch_drag_events(&mut self.world);
    }

    /// 投函済みの保存がすべて書き終わってから Ghost スコープを読む。
    fn saved(&self) -> Vec<(PersistKey, String)> {
        self.parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");
        load_scope(
            PersistScope::Ghost,
            &self.roots,
            &SharedFakeIo(self.store.clone()),
        )
    }

    fn close(self) {
        self.parts.publisher.close();
        let _ = self.parts.handle.join();
    }
}

fn started(entity: Entity) -> DragTransition {
    DragTransition::Started {
        entity,
        start_pos: PhysicalPoint::new(DRAG_START.0, DRAG_START.1),
        timestamp: Instant::now(),
    }
}

fn ended(entity: Entity, cancelled: bool) -> DragTransition {
    DragTransition::Ended {
        entity,
        end_pos: PhysicalPoint::new(CURSOR.0, CURSOR.1),
        cancelled,
    }
}

/// scope の `WindowPos` の組（保存されていなければ空）。
fn window_pos_keys(saved: &[(PersistKey, String)]) -> Vec<(PersistKey, String)> {
    saved
        .iter()
        .filter(|(k, _)| matches!(k, PersistKey::WindowPos { scope, .. } if *scope == SCOPE as u32))
        .cloned()
        .collect()
}

/// scope の `BalloonOffset` の組（保存されていなければ空）。
fn balloon_offset_keys(saved: &[(PersistKey, String)]) -> Vec<(PersistKey, String)> {
    saved
        .iter()
        .filter(
            |(k, _)| matches!(k, PersistKey::BalloonOffset { scope, .. } if *scope == SCOPE as u32),
        )
        .cloned()
        .collect()
}

/// 並びを問わず同じ組か（`load_scope` の並びに寄りかからない）。
fn same_pairs(actual: &[(PersistKey, String)], expected: &[(PersistKey, String)]) -> bool {
    actual.len() == expected.len() && expected.iter().all(|e| actual.contains(e))
}

/// キャラ窓の保存 1 組（原点＝下端中央基準: 左上 x ＋ w/2）。
fn char_pair(top_left: Point) -> Vec<(PersistKey, String)> {
    let scope = SCOPE as u32;
    vec![
        (
            PersistKey::WindowPos {
                scope,
                axis: Axis::X,
            },
            (top_left.x + CHAR_SIZE.w / 2).to_string(),
        ),
        (
            PersistKey::WindowPos {
                scope,
                axis: Axis::Y,
            },
            top_left.y.to_string(),
        ),
    ]
}

/// 開始の腕が入れた `DraggingState` から受け手が復元するキャラ窓の行き先。
///
/// 偽の HWND では開始の腕の枠の座標変換（`client_to_window_coords`）が失敗し、
/// `DraggingState.initial_inset` が `(0,0)` のまま入る。そのため行き先は
/// `(0,0) + (離したカーソル − 押したカーソル)` をアンカー辺へ射影した値になる。
/// これはテストの組み立ての都合（偽の窓の座標変換の失敗）に寄りかかった期待値で、
/// 製品では `initial_inset` に `WindowPos.position` の枠込みの値が入る。
fn destination() -> Point {
    let raw = PointPx {
        x: CURSOR.0 - DRAG_START.0,
        y: CURSOR.1 - DRAG_START.1,
    };
    let snapshot = MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1920, 1200)],
    };
    let p = project_anchor(Anchor::Bottom, raw, CHAR_SIZE, Some(&snapshot));
    Point { x: p.x, y: p.y }
}

/// T7-1（要件 1.1・1.3・1.4・5.1）: キャラ窓を動かさずにクリック（開始の無い終了）
/// → 保存 0 件・窓の位置は押す前のまま。修正前は赤（受け手が今の位置へ縮退して保存する）。
#[test]
fn click_without_move_on_char_persists_nothing_and_keeps_position() {
    let mut rig = Rig::new();

    rig.put(ended(rig.char_w, false));
    rig.tick();

    let saved = rig.saved();
    assert!(
        window_pos_keys(&saved).is_empty(),
        "動かさないクリックでキャラ窓の位置が保存された: {saved:?}"
    );
    assert_eq!(position_of(&rig.world, rig.char_w), CHAR_START);

    rig.close();
}

/// T7-2（要件 1.2・1.4・5.1）: バルーン窓を動かさずにクリック → 相対位置の保存 0 件・
/// キーワードで決めたバルーンの位置の素材が残る・バルーンの位置は押す前のまま。
/// 修正前は赤（相対位置が保存され素材が退役する）。
#[test]
fn click_without_move_on_balloon_persists_nothing_and_keeps_keyword_base() {
    let mut rig = Rig::new();

    rig.put(ended(rig.balloon, false));
    rig.tick();

    let saved = rig.saved();
    assert!(
        balloon_offset_keys(&saved).is_empty(),
        "動かさないクリックでバルーンの相対位置が保存された: {saved:?}"
    );
    assert_eq!(
        rig.world.get::<BalloonKeywordBase>(rig.char_w),
        Some(&KEYWORD_BASE),
        "動かさないクリックでキーワードの素材が退役した"
    );
    assert_eq!(position_of(&rig.world, rig.balloon), BALLOON_START);
    assert_eq!(position_of(&rig.world, rig.char_w), CHAR_START);

    rig.close();
}

/// T7-3（要件 1.5）: 動かし始める前の取り消し（取り消しの印つきの開始の無い終了）を
/// キャラ窓・バルーン窓へ順に配る → どちらも保存 0 件・素材が残る・位置は押す前のまま。
/// 修正前は赤。
#[test]
fn cancel_before_threshold_persists_nothing() {
    let mut rig = Rig::new();

    rig.put(ended(rig.char_w, true));
    rig.tick();
    rig.put(ended(rig.balloon, true));
    rig.tick();

    let saved = rig.saved();
    assert!(
        window_pos_keys(&saved).is_empty(),
        "準備中の取り消しでキャラ窓の位置が保存された: {saved:?}"
    );
    assert!(
        balloon_offset_keys(&saved).is_empty(),
        "準備中の取り消しでバルーンの相対位置が保存された: {saved:?}"
    );
    assert_eq!(
        rig.world.get::<BalloonKeywordBase>(rig.char_w),
        Some(&KEYWORD_BASE),
        "準備中の取り消しでキーワードの素材が退役した"
    );
    assert_eq!(position_of(&rig.world, rig.char_w), CHAR_START);
    assert_eq!(position_of(&rig.world, rig.balloon), BALLOON_START);

    rig.close();
}

/// T7-4（要件 2.1・2.2・5.3）: 本物のドラッグ（開始 → 配る → 終了 → 配る）をキャラ窓、
/// 続いてバルーン窓で行う → キャラ窓の位置 1 組・バルーンの相対位置 1 組・素材が外れる。
/// 修正の前後とも緑。
#[test]
fn real_drag_persists_once_for_char_and_balloon() {
    let mut rig = Rig::new();

    // キャラ窓のドラッグ。
    rig.put(started(rig.char_w));
    rig.tick();
    rig.put(ended(rig.char_w, false));
    rig.tick();
    let char_final = destination();
    assert_eq!(position_of(&rig.world, rig.char_w), char_final);

    // バルーン窓のドラッグ。バルーンは move_window=true ゆえ wndproc が動かした位置を
    // `WindowPos.position` へ置いたものとして明示する。
    rig.put(started(rig.balloon));
    rig.tick();
    let balloon_final = Point { x: 773, y: 409 };
    rig.world
        .get_mut::<WindowPos>(rig.balloon)
        .unwrap()
        .position = Some(balloon_final);
    rig.put(ended(rig.balloon, false));
    rig.tick();

    let saved = rig.saved();
    assert!(
        same_pairs(&window_pos_keys(&saved), &char_pair(char_final)),
        "本物のドラッグでキャラ窓の位置が 1 組保存されていない: {saved:?}"
    );
    let scope = SCOPE as u32;
    assert!(
        same_pairs(
            &balloon_offset_keys(&saved),
            &vec![
                (
                    PersistKey::BalloonOffset {
                        scope,
                        axis: Axis::X
                    },
                    (balloon_final.x - char_final.x).to_string()
                ),
                (
                    PersistKey::BalloonOffset {
                        scope,
                        axis: Axis::Y
                    },
                    (balloon_final.y - char_final.y).to_string()
                ),
            ]
        ),
        "本物のドラッグでバルーンの相対位置が 1 組保存されていない: {saved:?}"
    );
    assert_eq!(
        rig.world.get::<BalloonKeywordBase>(rig.char_w),
        None,
        "本物のドラッグでキーワードの素材が退役していない"
    );

    rig.close();
}

/// T7-5（要件 2.3・5.4）: 速いドラッグ（開始と終了を積んでから配る 1 回）→ キャラ窓が
/// 行き先へ置かれ、その位置が 1 組保存される。行き先の期待値は [`destination`] の
/// とおり偽の窓の座標変換の失敗に寄りかかる。修正前は赤（開始が終了に上書きされて
/// `DraggingState` が入らず、窓は押す前の位置のまま・保存値も押す前の位置）。
#[test]
fn fast_drag_places_window_at_destination_and_persists_once() {
    let mut rig = Rig::new();
    let dest = destination();
    assert_ne!(dest, CHAR_START, "前提: 行き先は押す前の位置と異なる");

    rig.put(started(rig.char_w));
    rig.put(ended(rig.char_w, false));
    rig.tick();

    assert_eq!(
        position_of(&rig.world, rig.char_w),
        dest,
        "速いドラッグでキャラ窓が行き先へ置かれていない"
    );
    let saved = rig.saved();
    assert!(
        same_pairs(&window_pos_keys(&saved), &char_pair(dest)),
        "速いドラッグで行き先の位置が 1 組保存されていない: {saved:?}"
    );

    rig.close();
}

/// T7-6（要件 2.5・5.3）: 閾値を越えた後の取り消し（開始 → 配る → 取り消しの印つきの
/// 終了 → 配る）→ 今どおりその時点の位置を 1 組保存する。修正の前後とも緑。
#[test]
fn cancel_after_threshold_persists_as_today() {
    let mut rig = Rig::new();

    rig.put(started(rig.char_w));
    rig.tick();
    rig.put(ended(rig.char_w, true));
    rig.tick();

    let saved = rig.saved();
    assert!(
        same_pairs(&window_pos_keys(&saved), &char_pair(destination())),
        "閾値を越えた後の取り消しで位置が 1 組保存されていない: {saved:?}"
    );

    rig.close();
}
