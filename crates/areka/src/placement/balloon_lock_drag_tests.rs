//! 止めたバルーンのドラッグの檻（task 5.2・要件 5.6・6.1・6.3・6.4・6.5・6.6・7.4・
//! design「BalloonLock」入口の見張り）。
//!
//! 組み立ては `follow_drag_end_gate_tests.rs` と同じ: wndproc の代わりに
//! `DragAccumulatorResource` へ種を積み、`dispatch_drag_events` を 1 回の画面更新として呼ぶ。
//! バルーンの窓は製品と同じく `DragConfig` と受け手 `OnDrag`・`OnDragEnd` を持ち、
//! wintf がドラッグの始まりに控える `WindowDragContextResource` もそのまま通る。
//! 止めたドラッグでは wintf が窓を動かさないので、バルーンの `WindowPos` は書き換えない。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope, spawn_sylphya,
};
use bevy_ecs::message::Messages;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::drag::{
    DragAccumulatorResource, DragConfig, DragEndEvent, DragEvent, DragStartEvent, DragTransition,
    OnDrag, OnDragEnd, WindowDragContext, WindowDragContextResource, dispatch_drag_events,
};
use wintf::ecs::pointer::Phase;
use wintf::ecs::{PhysicalPoint, Point, SizeI, Window, WindowHandle, WindowPos};

use super::super::follow::{
    Anchored, BalloonFollow, MonitorSnapshot, OffsetBase, on_balloon_drag, on_balloon_drag_end,
    on_char_drag,
};
use super::super::persist::PersistWiring;
use super::super::resolver::{Anchor, PointPx, RectPx};
use super::super::spawn::{BalloonKeywordBase, CharWindowMarker};
use super::super::test_support::{LogEvent, capture_logs};
use super::*;
use crate::placement::config::BalloonXMode;

/// 相方（scope 1）。0 以外にして既定値との取り違えを避ける。
const SCOPE: usize = 1;
const CHAR_START: Point = Point { x: 1203, y: 513 };
const BALLOON_START: Point = Point { x: 1021, y: 541 };
/// 押す前の追従のずれ。窓の今の差（-182, 28）と違う値にして、引き直されたら分かるようにする。
const OFFSET_START: PointPx = PointPx { x: -150, y: 20 };
const CHAR_HWND: usize = 0x1000;
const BALLOON_HWND: usize = 0x2000;
const KEYWORD_BASE: BalloonKeywordBase = BalloonKeywordBase {
    mode: BalloonXMode::CenterTop,
    adjust: PointPx { x: 13, y: -7 },
};

struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

fn window_pos(p: Point, w: i32, h: i32) -> WindowPos {
    WindowPos {
        position: Some(p),
        size: Some(SizeI::new(w, h)),
        ..Default::default()
    }
}

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
        let mut world = World::new();
        world.insert_resource(DragAccumulatorResource::new());
        world.insert_resource(WindowDragContextResource::new());
        world.init_resource::<Messages<DragStartEvent>>();
        world.init_resource::<Messages<DragEvent>>();
        world.init_resource::<Messages<DragEndEvent>>();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![RectPx {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1200,
            }],
        });
        world.insert_non_send(PersistWiring {
            publisher: parts.publisher.clone(),
        });
        // 製品（`spawn.rs`）と同じく、バルーンの窓は `DragConfig::default()` と 2 つの受け手を持つ。
        let balloon = world
            .spawn((
                Window::default(),
                fake_handle(BALLOON_HWND),
                window_pos(BALLOON_START, 223, 158),
                BalloonWindowMarker { scope: SCOPE },
                DragConfig::default(),
                OnDrag(on_balloon_drag),
                OnDragEnd(on_balloon_drag_end),
            ))
            .id();
        let char_w = world
            .spawn((
                Window::default(),
                fake_handle(CHAR_HWND),
                window_pos(CHAR_START, 434, 687),
                Anchored(Anchor::Bottom),
                CharWindowMarker { scope: SCOPE },
                BalloonFollow::new(balloon, OffsetBase::unpinned(OFFSET_START)),
                KEYWORD_BASE,
                OnDrag(on_char_drag),
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

    fn put(&self, transition: DragTransition) {
        self.world
            .resource::<DragAccumulatorResource>()
            .set_transition(transition);
    }

    fn tick(&mut self) {
        dispatch_drag_events(&mut self.world);
    }

    /// 始まり → 配る。
    fn start(&mut self, entity: Entity) {
        self.put(DragTransition::Started {
            entity,
            start_pos: PhysicalPoint::new(1307, 901),
            timestamp: Instant::now(),
        });
        self.tick();
    }

    /// ドラッグ中のカーソルを動かして配る（`OnDrag` が出る）。
    fn drag(&mut self, cursor: Point) {
        let acc = self.world.resource::<DragAccumulatorResource>();
        acc.accumulate_delta(PhysicalPoint::new(1, 1));
        acc.update_position(PhysicalPoint::new(cursor.x, cursor.y));
        self.tick();
    }

    /// 終わり → 配る。
    fn end(&mut self, entity: Entity) {
        self.put(DragTransition::Ended {
            entity,
            end_pos: PhysicalPoint::new(1711, 953),
            cancelled: false,
        });
        self.tick();
    }

    /// 始まり・途中・終わりの 1 回のドラッグ。
    fn full_drag(&mut self, entity: Entity) {
        self.start(entity);
        self.drag(Point { x: 1500, y: 930 });
        self.end(entity);
    }

    /// wintf が窓を動かしたものとしてバルーンの位置を書く（動かせるドラッグの間だけ使う）。
    fn move_balloon(&mut self, to: Point) {
        self.world
            .get_mut::<WindowPos>(self.balloon)
            .unwrap()
            .position = Some(to);
    }

    fn balloon_pos(&self) -> Point {
        self.world
            .get::<WindowPos>(self.balloon)
            .and_then(|wp| wp.position)
            .unwrap()
    }

    fn char_pos(&self) -> Point {
        self.world
            .get::<WindowPos>(self.char_w)
            .and_then(|wp| wp.position)
            .unwrap()
    }

    fn follow(&self) -> BalloonFollow {
        *self.world.get::<BalloonFollow>(self.char_w).unwrap()
    }

    fn keyword_base(&self) -> Option<BalloonKeywordBase> {
        self.world.get::<BalloonKeywordBase>(self.char_w).copied()
    }

    /// 記憶に書かれたこのスコープのバルーンの相対位置。
    fn saved_offsets(&self) -> Vec<(PersistKey, String)> {
        self.parts.publisher.barrier().unwrap();
        load_scope(
            PersistScope::Ghost,
            &self.roots,
            &SharedFakeIo(self.store.clone()),
        )
        .into_iter()
        .filter(
            |(k, _)| matches!(k, PersistKey::BalloonOffset { scope, .. } if *scope == SCOPE as u32),
        )
        .collect()
    }

    /// 終わりの受け手を直に呼ぶ（wintf の控えを手で組む場合）。
    fn call_end(&mut self) {
        let ev = DragEndEvent {
            target: self.balloon,
            position: PhysicalPoint::new(1711, 953),
            cancelled: false,
            is_primary: true,
            timestamp: Instant::now(),
        };
        let balloon = self.balloon;
        on_balloon_drag_end(&mut self.world, balloon, balloon, &Phase::Bubble(ev));
    }

    fn close(self) {
        self.parts.publisher.close();
        let _ = self.parts.handle.join();
    }
}

fn held_logs(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("balloon_drag_held"))
        .collect()
}

fn scopes(raw: &[u32]) -> BTreeSet<u32> {
    raw.iter().copied().collect()
}

/// 止めたドラッグ（6.1・6.6）: 追従のずれ・位置・記憶・キーワードの素材が変わらず、記録が 1 件。
#[test]
fn held_balloon_drag_changes_nothing_and_logs_once() {
    let mut rig = Rig::new();
    seed_balloon_lock(&mut rig.world, &scopes(&[SCOPE as u32]));
    let follow = rig.follow();

    let ((), events) = capture_logs(|| rig.full_drag(rig.balloon));

    assert_eq!(
        rig.follow(),
        follow,
        "止めたドラッグで追従のずれが引き直された"
    );
    assert_eq!(rig.balloon_pos(), BALLOON_START);
    assert!(
        rig.saved_offsets().is_empty(),
        "止めたドラッグで相対位置が記憶された"
    );
    assert_eq!(rig.keyword_base(), Some(KEYWORD_BASE), "素材が退役した");
    let held = held_logs(&events);
    assert_eq!(
        held.len(),
        1,
        "記録は 1 回のドラッグにつき 1 件: {events:?}"
    );
    assert_eq!(held[0].field("scope"), Some("1"));
    assert_eq!(held[0].field("descript"), Some("true"));
    assert_eq!(held[0].field("script"), Some("false"));
    rig.close();
}

/// 止めていないスコープ: 今どおり追従のずれ・記憶・素材が書かれ、止めた記録は出ない。
#[test]
fn unheld_scope_writes_as_today() {
    let mut rig = Rig::new();
    seed_balloon_lock(&mut rig.world, &scopes(&[0]));
    let follow = rig.follow();

    let ((), events) = capture_logs(|| {
        rig.start(rig.balloon);
        rig.move_balloon(Point { x: 773, y: 409 });
        rig.drag(Point { x: 1500, y: 930 });
        rig.end(rig.balloon);
    });

    assert_ne!(
        rig.follow(),
        follow,
        "動かせるドラッグで追従のずれが引き直されていない"
    );
    assert_eq!(
        rig.saved_offsets().len(),
        2,
        "相対位置が 1 組記憶されていない"
    );
    assert_eq!(rig.keyword_base(), None, "素材が退役していない");
    assert!(held_logs(&events).is_empty());
    rig.close();
}

/// 控えが消えた後（`hwnd: None`・`move_window: false`）: 控えの `move_window` を読まず、
/// 窓の今の設定へ縮退する。動かせる窓なら今どおり、止めた窓なら止める。
#[test]
fn cleared_snapshot_falls_back_to_current_setting() {
    let mut rig = Rig::new();
    rig.world.resource::<WindowDragContextResource>().clear();

    let ((), events) = capture_logs(|| rig.call_end());
    assert_eq!(
        rig.saved_offsets().len(),
        2,
        "消えた控えを「止める」と読んだ"
    );
    assert!(held_logs(&events).is_empty());

    rig.world.entity_mut(rig.char_w).insert(KEYWORD_BASE);
    apply_script_balloon_lock(&mut rig.world, SCOPE as u32, true);
    let ((), events) = capture_logs(|| rig.call_end());
    assert_eq!(
        rig.keyword_base(),
        Some(KEYWORD_BASE),
        "止めた窓で素材が退役した"
    );
    assert_eq!(held_logs(&events).len(), 1);
    assert_eq!(held_logs(&events)[0].field("script"), Some("true"));
    rig.close();
}

/// 別の窓の控え: その `move_window` を読まず、この窓の今の設定へ縮退する。
#[test]
fn other_window_snapshot_falls_back_to_current_setting() {
    let mut rig = Rig::new();
    rig.world
        .resource::<WindowDragContextResource>()
        .set(WindowDragContext {
            hwnd: Some(HWND(CHAR_HWND as *mut _)),
            initial_window_pos: None,
            move_window: false,
            constraint: None,
        });

    let ((), events) = capture_logs(|| rig.call_end());

    assert_eq!(rig.saved_offsets().len(), 2, "別の窓の控えで止めた");
    assert_eq!(rig.keyword_base(), None);
    assert!(held_logs(&events).is_empty());
    rig.close();
}

/// 途中の lock（5.6）: そのドラッグは今どおり終わり（記憶が書かれ）、次のドラッグから止まる。
#[test]
fn mid_drag_lock_takes_effect_from_next_drag() {
    let mut rig = Rig::new();
    seed_balloon_lock(&mut rig.world, &BTreeSet::new());

    let ((), events) = capture_logs(|| {
        rig.start(rig.balloon);
        apply_script_balloon_lock(&mut rig.world, SCOPE as u32, true);
        rig.move_balloon(Point { x: 773, y: 409 });
        rig.drag(Point { x: 1500, y: 930 });
        rig.end(rig.balloon);
    });
    let saved = rig.saved_offsets();
    assert_eq!(saved.len(), 2, "途中の lock がそのドラッグに効いた");
    assert_eq!(rig.keyword_base(), None);
    assert!(held_logs(&events).is_empty());

    let follow = rig.follow();
    let ((), events) = capture_logs(|| rig.full_drag(rig.balloon));
    assert_eq!(rig.follow(), follow);
    assert_eq!(
        rig.saved_offsets(),
        saved,
        "次のドラッグで記憶が書き換わった"
    );
    assert_eq!(
        held_logs(&events).len(),
        1,
        "次のドラッグから止まっていない"
    );
    rig.close();
}

/// キャラクターの窓のドラッグ（6.3）: 止めたスコープでもバルーンは付いて動く。
#[test]
fn char_drag_moves_held_balloon_along() {
    let mut rig = Rig::new();
    seed_balloon_lock(&mut rig.world, &scopes(&[SCOPE as u32]));

    rig.start(rig.char_w);
    rig.drag(Point { x: 1500, y: 930 });

    assert_ne!(rig.char_pos(), CHAR_START, "前提: キャラクターの窓が動いた");
    let c = rig.char_pos();
    assert_eq!(
        rig.balloon_pos(),
        Point {
            x: c.x + OFFSET_START.x,
            y: c.y + OFFSET_START.y
        },
        "止めたスコープのバルーンがキャラクターの窓に付いて動いていない"
    );
    rig.close();
}

/// 止めたドラッグを放してもクリックにならない（6.2・design I5）: ドラッグの判定
/// （`enabled`・`left_button`）は落とさず、放しはドラッグの終わりとして配られる。
/// バルーンのクリックは押した時点で決まるので、放しがクリックとして届く道は無い。
#[test]
fn held_drag_release_is_delivered_as_drag_end_not_click() {
    let mut rig = Rig::new();
    seed_balloon_lock(&mut rig.world, &scopes(&[SCOPE as u32]));

    rig.full_drag(rig.balloon);

    let drag = rig.world.get::<DragConfig>(rig.balloon).unwrap();
    assert!(drag.enabled && drag.left_button && !drag.move_window);
    assert_eq!(rig.world.resource::<Messages<DragEndEvent>>().len(), 1);
    rig.close();
}
