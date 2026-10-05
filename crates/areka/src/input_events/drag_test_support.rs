//! ドラッグの知らせのテストの土台（areka-P0-mouse-drag-events・design「Testing Strategy > areka」）。
//!
//! 本番と同じ順に組む: 本物の `spawn_ghost_windows`（本体と相方）→ 偽の `WindowHandle` →
//! [`attach_char_pointer_handlers`]。wintf のドラッグの資源・`MonitorSnapshot`・偽の記憶の書き手
//! （`FakePersistIo`）を持つ `PersistWiring`・当たり判定の偽物（`RegionSource::Mock`）と止めた時計を
//! 入れた `MouseWiring` と、その受け口（kanade の受信箱の代わり）を持つ。比べるために、包みを
//! 付けない本 spec の前の組み方（[`Rig::without_wrapper`]）も持つ。
//!
//! 知らせは wndproc の代わりに wintf の運ぶ箱（`DragAccumulatorResource`）へ種を積み、
//! `dispatch_drag_events` で配る（本番と同じ配りの経路で、Tunnel と Bubble の 2 回の呼び出しも通る）。

use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use areka_kanade::{KanadeMsg, MouseInput};
use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope, spawn_sylphya,
};
use bevy_ecs::message::Messages;
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::drag::{
    DragAccumulatorResource, DragEndEvent, DragEvent, DragStartEvent, DragTransition,
    DraggingState, WindowDragContextResource, dispatch_drag_events,
};
use wintf::ecs::{PhysicalPoint, Point, WindowHandle, WindowPos};

use super::super::{MouseWiring, RegionSource, attach_char_pointer_handlers};
use crate::emo2_boot::hit_region::HitRegion;
use crate::placement::follow::{MonitorSnapshot, OffsetBase};
use crate::placement::persist::PersistWiring;
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;

/// 相方（scope 1）。0 以外にして既定値との取り違えを避ける。
pub(super) const SCOPE: usize = 1;
/// work area（物理 px）。キャラ窓は下端 1200 へ接地する。
const WORK_AREA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 1920,
    bottom: 1200,
};
/// 偽の当たり判定が `Head` を返す窓の中の x の上限（これ未満が当たり判定のある位置）。
const HEAD_X_LIMIT: i64 = 100;

/// 当たり判定の偽物: 渡された窓の中の位置 `(x, y)` から区別できる座標 `(2x, 3y)` を返し、
/// `x < HEAD_X_LIMIT` なら `Head`・それ以外は当たり判定なし。受け口の値から、渡した位置が
/// 正しいことを読み戻せる。
pub(super) fn fake_hit(scope: u32, x: i64, y: i64) -> HitRegion {
    HitRegion {
        scope,
        region: (x < HEAD_X_LIMIT).then(|| "Head".to_string()),
        surface_point: (x * 2, y * 3),
    }
}

/// 本体と相方の配置（96 の非倍数・キャラ窓は下端接地）。
fn placements() -> Vec<ScopePlacement> {
    let one = |scope: usize, char_pos: PointPx, char_size: SizePx, balloon_pos: PointPx| {
        let offset = PointPx {
            x: balloon_pos.x - char_pos.x,
            y: balloon_pos.y - char_pos.y,
        };
        ScopePlacement {
            scope,
            char_pos,
            char_size,
            balloon_pos,
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: offset,
            balloon_offset_base: OffsetBase::unpinned(offset),
            balloon_limit: false,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
        }
    };
    vec![
        one(
            0,
            PointPx { x: 1483, y: 513 },
            SizePx { w: 434, h: 687 },
            PointPx { x: 1071, y: 488 },
        ),
        one(
            1,
            PointPx { x: 1049, y: 843 },
            SizePx { w: 278, h: 357 },
            PointPx { x: 1334, y: 824 },
        ),
    ]
}

/// 偽の HWND を持つ `WindowHandle`。
fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// アクターへ渡す口と観測する口が同じストアを指す偽の記憶の書き手。
struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

/// 本番の順で組んだ 2 スコープのゴースト窓と、マウスの知らせの受け口。
pub(super) struct Rig {
    pub(super) world: World,
    /// kanade の受信箱の代わり。
    pub(super) rx: mpsc::Receiver<KanadeMsg>,
    parts: SylphyaParts,
    roots: ScopeRoots,
    store: Arc<FakePersistIo>,
    /// 相方のキャラ窓。
    pub(super) char_w: Entity,
    /// 相方のバルーン窓。
    pub(super) balloon: Entity,
}

impl Rig {
    /// 本番の付け方（`spawn_ghost_windows` → [`attach_char_pointer_handlers`]）。
    pub(super) fn new() -> Self {
        Self::build(true)
    }

    /// 本 spec の前の付け方（`spawn_ghost_windows` だけ＝位置の保存の受け手を包まない）。
    pub(super) fn without_wrapper() -> Self {
        Self::build(false)
    }

    fn build(attach: bool) -> Self {
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
        // `dispatch_drag_events` が引く資源（wintf の `EcsWorld::new` と同じ登録）。
        world.insert_resource(DragAccumulatorResource::new());
        world.insert_resource(WindowDragContextResource::new());
        world.init_resource::<Messages<DragStartEvent>>();
        world.init_resource::<Messages<DragEvent>>();
        world.init_resource::<Messages<DragEndEvent>>();
        world.insert_resource(MonitorSnapshot {
            work_areas: vec![WORK_AREA],
        });
        world.insert_non_send(PersistWiring {
            publisher: parts.publisher.clone(),
        });

        // 本番の順: spawn → 偽の WindowHandle（`enqueue_window_set_pos` が位置を書ける条件）→ attach。
        let gw = spawn_ghost_windows(
            &mut world,
            &placements(),
            &GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]),
        );
        let mut raw = 0x100usize;
        for scope in [0, 1] {
            for e in [
                gw.char_window(scope).unwrap(),
                gw.balloon_window(scope).unwrap(),
            ] {
                world.entity_mut(e).insert(fake_handle(raw));
                raw += 0x10;
            }
        }
        if attach {
            attach_char_pointer_handlers(&mut world);
        }

        // 止めた時計（間引きの時間の窓は開かない）。
        let (tx, rx) = mpsc::channel();
        world.insert_non_send(MouseWiring::with_clock(
            tx,
            RegionSource::Mock(fake_hit),
            Box::new(|| 1_000),
        ));

        Self {
            world,
            rx,
            parts,
            roots,
            store,
            char_w: gw.char_window(SCOPE).unwrap(),
            balloon: gw.balloon_window(SCOPE).unwrap(),
        }
    }

    /// wndproc の代わりに種を積む（配りはしない）。
    pub(super) fn put(&self, transition: DragTransition) {
        self.world
            .resource::<DragAccumulatorResource>()
            .set_transition(transition);
    }

    /// 1 回の画面更新として配る。
    pub(super) fn tick(&mut self) {
        dispatch_drag_events(&mut self.world);
    }

    /// 窓の今の位置（`WindowPos.position`）。
    pub(super) fn position_of(&self, entity: Entity) -> Point {
        self.world
            .get::<WindowPos>(entity)
            .and_then(|wp| wp.position)
            .expect("WindowPos.position があるはず")
    }

    /// 窓の手続きの代わりに窓の位置を直に書く（ドラッグの間の窓の移動）。
    pub(super) fn move_window(&mut self, entity: Entity, to: Point) {
        self.world
            .get_mut::<WindowPos>(entity)
            .expect("WindowPos があるはず")
            .position = Some(to);
    }

    /// 開始の配りで入った `DraggingState.initial_inset` を窓の位置にする（製品と同じ意味の値）。
    ///
    /// 偽の HWND では wintf の開始の腕の枠の座標変換が失敗し、`(0,0)` のまま入る。
    pub(super) fn set_initial_inset(&mut self, entity: Entity, at: Point) {
        self.world
            .get_mut::<DraggingState>(entity)
            .expect("開始が配られて DraggingState があるはず")
            .initial_inset = (at.x as f32, at.y as f32);
    }

    /// 受け口を落とす（以後の送出は失敗する）。
    pub(super) fn drop_receiver(&mut self) {
        self.rx = mpsc::channel().1;
    }

    /// 投函済みの保存が書き終わってから、記憶（Ghost スコープ）に書かれた組を読む。
    pub(super) fn saved(&self) -> Vec<(PersistKey, String)> {
        self.parts
            .publisher
            .barrier()
            .expect("アクターが生きている間は barrier が返るはず");
        load_scope(
            PersistScope::Ghost,
            &self.roots,
            &SharedFakeIo(self.store.clone()),
        )
    }

    /// 受け口に溜まったマウスの知らせを全部取り出す（マウス以外が来たら落とす）。
    pub(super) fn drain(&self) -> Vec<MouseInput> {
        self.rx
            .try_iter()
            .map(|msg| match msg {
                KanadeMsg::Mouse(m) => m,
                _ => panic!("マウス以外の知らせが届いた"),
            })
            .collect()
    }

    pub(super) fn close(self) {
        self.parts.publisher.close();
        let _ = self.parts.handle.join();
    }
}

/// 開始の種（`start` はスクリーン物理 px）。
pub(super) fn started(entity: Entity, start: Point) -> DragTransition {
    DragTransition::Started {
        entity,
        start_pos: PhysicalPoint::new(start.x, start.y),
        timestamp: Instant::now(),
    }
}

/// 終了の種（`end` はスクリーン物理 px）。
pub(super) fn ended(entity: Entity, end: Point, cancelled: bool) -> DragTransition {
    DragTransition::Ended {
        entity,
        end_pos: PhysicalPoint::new(end.x, end.y),
        cancelled,
    }
}
