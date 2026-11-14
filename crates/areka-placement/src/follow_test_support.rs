use std::time::Instant;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::SizeI;
use wintf::ecs::drag::{DragEndEvent, DragEvent, DraggingState};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::{Point, WindowHandle, WindowPos};

use super::{MonitorSnapshot, project_anchor};
use crate::resolver::RectPx;
use log_capture_kit::CapturedEvent;

// -------------------------------------------------------------------------
// テストヘルパ（偽装境界: 実 HWND なしの headless World で決定論検証する。
// SetWindowPosCommand は TLS キューへの enqueue のみで flush しないため、
// 偽 HWND に対する実 SetWindowPos は一切呼ばれない——wintf 自身の
// window_pos_systems_test と同じ流儀）
// -------------------------------------------------------------------------

/// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム）。
pub(super) fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// position 初期値付きの WindowPos。
pub(super) fn window_pos_at(x: i32, y: i32) -> WindowPos {
    WindowPos {
        position: Some(Point { x, y }),
        ..Default::default()
    }
}

/// entity の WindowPos.position を読む（未設定は panic で検出）。
pub(super) fn position_of(world: &World, entity: Entity) -> Point {
    world
        .get::<WindowPos>(entity)
        .expect("WindowPos があるはず")
        .position
        .expect("position があるはず")
}

pub(super) fn drag_event(target: Entity) -> DragEvent {
    DragEvent {
        target,
        start_position: Point::new(0, 0),
        position: Point::new(10, 10),
        is_primary: true,
        timestamp: Instant::now(),
    }
}

pub(super) fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RectPx {
    RectPx {
        left,
        top,
        right,
        bottom,
    }
}

/// 単一モニタの合成 snapshot（物理 px・96 の倍数を避けた下端で再スケール檻）。
pub(super) fn single_monitor_snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(0, 0, 1920, 1043)],
    }
}

/// 全 4 辺が 96 の非倍数の単一モニタ snapshot（各アンカー辺再計算の再スケール檻）。
/// left=53・top=37・right=1877・bottom=1043（いずれも 96 で割り切れない・非零原点）。
pub(super) fn odd_edge_snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(53, 37, 1877, 1043)],
    }
}

// -------------------------------------------------------------------------
// bottom 吸着ドラッグ（task 8.2R・4.7・DD15 v2: 単一ライター）
//
// BottomSnap キャラ窓は DragConfig{move_window:false}＝wndproc は窓を動かさず、
// on_char_drag が DraggingState（dispatch_drag_events が挿入）＋DragEvent の
// カーソル座標から生ドラッグ座標を復元し、ポリシー適用済み座標を一度だけ書く。
// headless では DraggingState を注入して実 flow を模し、handler を直接呼ぶ。
// -------------------------------------------------------------------------

/// DraggingState（dispatch_drag_events 挿入の模擬）。wintf の実セマンティクス:
/// `initial_inset`＝ドラッグ開始時の**窓位置**（dispatch.rs が initial_window_pos
/// を転記）・`drag_start_pos`＝開始カーソル（スクリーン物理 px）。
pub(super) fn dragging_state(initial_window: (i32, i32), drag_start: (i32, i32)) -> DraggingState {
    DraggingState {
        drag_start_pos: Point::new(drag_start.0, drag_start.1),
        initial_inset: (initial_window.0 as f32, initial_window.1 as f32),
    }
}

/// カーソル座標付き DragEvent（start_position は DraggingState と同値・実 flow 準拠）。
pub(super) fn drag_event_at(target: Entity, start: (i32, i32), cursor: (i32, i32)) -> DragEvent {
    DragEvent {
        target,
        start_position: Point::new(start.0, start.1),
        position: Point::new(cursor.0, cursor.1),
        is_primary: true,
        timestamp: Instant::now(),
    }
}

/// 最終カーソル座標付き DragEndEvent。
pub(super) fn drag_end_event_at(target: Entity, cursor: (i32, i32)) -> DragEndEvent {
    DragEndEvent {
        target,
        position: Point::new(cursor.0, cursor.1),
        cancelled: false,
        is_primary: true,
        timestamp: Instant::now(),
    }
}

/// position＋size 付きの WindowPos（spawn の `window_pos` と同型）。
pub(super) fn window_pos_sized(x: i32, y: i32, w: i32, h: i32) -> WindowPos {
    WindowPos {
        position: Some(Point { x, y }),
        size: Some(SizeI::new(w, h)),
        ..Default::default()
    }
}

// -------------------------------------------------------------------------
// work_area_for_window_with_origin ／ guard_visibility
// （task 2.2・D6/S3・S3′・Req 3.1/3.2/5.1/5.3/5.6）
//
// 共通規約: 判定は絶対 px の固定値ではなく**交差・不変条件**で書く（Req 5.6）。
// 座標は 96/120/192 の各水準へスケールした合成レイアウト上で構築し、96 の
// 自己整合（k=1 で恒等写像に退化して欠陥を隠す性質・Req 5.1）に依存しない。
// -------------------------------------------------------------------------

use crate::resolver::{Anchor, PointPx, SizePx};

/// DPI 水準（Req 5.1: 96 のほかに 120・192 を必ず含む）。
pub(super) const DPIS: [i32; 3] = [96, 120, 192];

/// 論理基準値 → 各 DPI の物理 px（整数演算のみ・厳密整除を強制。
/// `resolver.rs` の `px()` が donor・Req 5.6）。
pub(super) fn px(logical: i32, dpi: i32) -> i32 {
    assert_eq!(
        (logical * dpi) % 96,
        0,
        "テスト入力は厳密整除になる論理値（4 の倍数）で構築する"
    );
    logical * dpi / 96
}

/// 混在 DPI マルチモニタの合成レイアウト（Req 5.1/5.3）。
///
/// - index 0: 96 水準の左モニタ。**負座標**（`-1920..0`）・上端 40px の
///   非対称 work area（`top = -40`）
/// - index 1: `dpi` 水準の右モニタ。左端に 64 論理 px のタスクバー＝
///   **非対称 work area**（`left = px(64)`）。192 では右端 3840＝**3200 超座標**
///
/// 2 面のあいだ（`0 ..= px(64)`）はどの work area にも属さない帯であり、
/// 最近傍フォールバックの発火面として使う。
pub(super) fn mixed_layout(dpi: i32) -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![left_wa(), right_wa(dpi)],
    }
}

/// 左モニタ（96 水準・負座標）の work area。
pub(super) fn left_wa() -> RectPx {
    rect(-1920, -40, 0, 1000)
}

/// 右モニタ（`dpi` 水準・非対称）の work area。192 で right=3840（>3200）。
pub(super) fn right_wa(dpi: i32) -> RectPx {
    rect(px(64, dpi), 0, px(1920, dpi), px(1040, dpi))
}

/// キャラ窓の寸（論理 300x400）。
pub(super) fn char_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(300, dpi),
        h: px(400, dpi),
    }
}

/// バルーン窓の寸（論理 500x300）。
pub(super) fn balloon_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(500, dpi),
        h: px(300, dpi),
    }
}

pub(super) fn point(x: i32, y: i32) -> PointPx {
    PointPx { x, y }
}

/// 位置＋寸 → 窓矩形（テスト側の独立実装＝実装の `rect_at` を再利用しない）。
pub(super) fn win(pos: PointPx, size: SizePx) -> RectPx {
    rect(pos.x, pos.y, pos.x + size.w, pos.y + size.h)
}

/// 面積を持つ重なりの独立実装（実装の `rects_intersect` とは別式で書く）。
pub(super) fn overlaps(a: RectPx, b: RectPx) -> bool {
    a.left.max(b.left) < a.right.min(b.right) && a.top.max(b.top) < a.bottom.min(b.bottom)
}

/// キャラ窓の Bottom 接地位置（射影 T が出す Y＝`wa.bottom − h`）。
pub(super) fn grounded_y(wa: RectPx, size: SizePx) -> i32 {
    wa.bottom - size.h
}

/// spawn 時 offset 付きの Arrangement（実 pipeline の spawn 位置を模す）。
pub(super) fn arrangement_at(x: f32, y: f32) -> Arrangement {
    Arrangement {
        offset: Offset { x, y },
        ..Default::default()
    }
}

/// entity の Arrangement.offset を読む（未付与は panic で検出）。
pub(super) fn arrangement_offset_of(world: &World, entity: Entity) -> Offset {
    world
        .get::<Arrangement>(entity)
        .expect("Arrangement があるはず")
        .offset
}

/// entity の WindowPos.size を読む（未設定は panic で検出）。
pub(super) fn size_of(world: &World, entity: Entity) -> SizeI {
    world
        .get::<WindowPos>(entity)
        .expect("WindowPos があるはず")
        .size
        .expect("size があるはず")
}

/// 手順書 §3.3 の grep 判定語（**本体の定数とは独立にここへ literal で置く**）。
pub(super) const CLAMP_TAG: &str = "[visibility-guard] ClampX";
/// 同上（最近傍フォールバックの非ドラッグ経路 warn 昇格・Req 3.2）。
pub(super) const NEAREST_TAG: &str = "[visibility-guard] NearestFallback";
/// 同上（work area を解決できず判定不能・Req 3.3）。
pub(super) const UNRESOLVED_TAG: &str = "[visibility-guard] WorkAreaUnresolved";
/// 3 語に共通の接頭辞（「ガードが何かを言った」ことの一括検出）。
/// **どの work area とも交差しない位置に居た窓**を最近傍モニタへ引き寄せたことを表す
/// 判定語（areka-P0-dpi-transition-atomicity 要件 5.5・task 5.1 が新設）。
///
/// [`NEAREST_TAG`] とは観測する矩形が違う——あちらは射影が**決めた位置**、こちらは射影の
/// **入力**である。画面外に居た窓は、決めた位置がモニタ内へ収まってしまえばあちらの腕には
/// 入らない（＝無観測で吸収される）。語が `[visibility-guard] NearestFallback` を
/// **部分文字列として含まない**のは、`expect_one(&events, NEAREST_TAG)` の件数がこの語で
/// 狂わないようにするためである。
pub(super) const OFFSCREEN_PULL_TAG: &str = "[visibility-guard] OffscreenPull";

pub(super) const GUARD_TAG_PREFIX: &str = "[visibility-guard]";

/// 幅広のキャラ窓寸（論理 320×400）。論理 320／32 はいずれも 8 の倍数ゆえ、
/// 96/120/192 のどの水準でも物理 px が偶数＝手順 3b の `w/2` が切り捨てで狂わない。
pub(super) fn wide_char_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(320, dpi),
        h: px(400, dpi),
    }
}

/// 「どの work area にも属さない帯」（`0 ..= px(64)`）より**狭い**新寸。
pub(super) fn narrow_char_size(dpi: i32) -> SizePx {
    SizePx {
        w: px(32, dpi),
        h: px(400, dpi),
    }
}

/// 帯の中で**右モニタが一意に最近傍になる**中心 x（帯の中点 `px(32)` は左右等距離で
/// 先勝ちに依存するため使わない）。
pub(super) fn gap_center_x(dpi: i32) -> i32 {
    px(40, dpi)
}

/// ガードを通さない**素の**射影結果（＝本タスク以前の挙動）。手順 3b と
/// [`project_anchor`] を檻側で独立に再現し、本体の実装を呼び直さない。
pub(super) fn unguarded_projection(dpi: i32, old_pos: PointPx, new: SizePx) -> PointPx {
    let old = wide_char_size(dpi);
    let raw = PointPx {
        x: old_pos.x + old.w / 2 - new.w / 2,
        y: old_pos.y,
    };
    project_anchor(Anchor::Bottom, raw, new, Some(&mixed_layout(dpi)))
}

/// 窓矩形がいずれかの work area と交差するか（檻側の独立実装 [`overlaps`] で判定）。
pub(super) fn visible_in(layout: &MonitorSnapshot, pos: PointPx, size: SizePx) -> bool {
    layout
        .work_areas
        .iter()
        .any(|wa| overlaps(win(pos, size), *wa))
}

/// 現在位置を [`PointPx`] で読む（檻の比較単位を射影の単位へ揃える）。
pub(super) fn point_of(world: &World, entity: Entity) -> PointPx {
    let p = position_of(world, entity);
    PointPx { x: p.x, y: p.y }
}

/// `[visibility-guard]` を名乗るイベントだけを抜く。
pub(super) fn guard_events<'a>(
    events: &'a [CapturedEvent],
    needle: &str,
) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect()
}

// -------------------------------------------------------------------------
// 範囲の中のテストに写してあった組み立て（audit-placement-follow・指摘 16）
// -------------------------------------------------------------------------

use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_emo_compose::ScaleRatio;
use areka_sylphya::persist::{FakePersistIo, FsPersistIo, PersistIo};
use areka_sylphya::{
    PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope, spawn_sylphya,
};
use temp_path_kit::TempPath;
use windows::Win32::Foundation::RECT;
use wintf::ecs::DPI;
use wintf::ecs::window::monitor::Monitor;

use super::super::persist::PersistWiring;
use super::{Anchored, OffsetBase};
use crate::resolver::ScopePlacement;
use crate::source::GhostTitles;
use crate::spawn::spawn_ghost_windows;

/// 補正ログの判定語（本体定数とは独立に檻側へ literal で置く）。
pub(super) const LIMIT_CLAMP_TAG: &str = "[balloon-limit] Clamp";
/// 解決不能（縮退）の判定語。
pub(super) const LIMIT_UNRESOLVED_TAG: &str = "[balloon-limit] Unresolved";
/// 2 段 grep 第 1 段の接頭辞（「関門が何かを言った」ことの一括検出）。
pub(super) const LIMIT_TAG_PREFIX: &str = "[balloon-limit]";
/// 窓移動レコードの判定語（`placement::diag` の手順書語彙）。
pub(super) const WINDOW_MOVE_TAG: &str = "[diag.window_move]";

/// [`odd_edge_snapshot`] の作業領域（全 4 辺が 96 の非倍数・原点非 (0,0)）。
pub(super) fn odd_edge_wa() -> RectPx {
    odd_edge_snapshot().work_areas[0]
}

/// limit の檻のバルーンの寸（223×158）。
pub(super) fn limit_balloon_size() -> SizePx {
    SizePx { w: 223, h: 158 }
}

/// 作業領域内へ収めたときの位置上限（右下側）。檻側の独立計算（実装を呼ばない）。
pub(super) fn max_pos(area: RectPx, size: SizePx) -> PointPx {
    PointPx {
        x: area.right - size.w,
        y: area.bottom - size.h,
    }
}

/// 4 辺内包しているか（檻側の独立実装）。
pub(super) fn contained(pos: PointPx, size: SizePx, area: RectPx) -> bool {
    pos.x >= area.left
        && pos.y >= area.top
        && pos.x + size.w <= area.right
        && pos.y + size.h <= area.bottom
}

/// limit の檻の基準となる 1 scope ぶんの配置（恒等式 `balloon_offset ≡ balloon_pos − char_pos` 成立）。
///
/// - キャラ窓 (1301,411) 434×600 → 矩形は作業領域に完全内包・中心も内包（帰属が最近傍
///   フォールバックへ落ちない＝基準モニタの解決が正常腕で決まる）
/// - バルーン (1601,811) 223×158 → 同じく内包（＝**書込前は補正不要**の状態から始める）
pub(super) fn base_placement(balloon_limit: bool) -> ScopePlacement {
    ScopePlacement {
        scope: 0,
        char_pos: PointPx { x: 1301, y: 411 },
        char_size: SizePx { w: 434, h: 600 },
        balloon_pos: PointPx { x: 1601, y: 811 },
        balloon_size: limit_balloon_size(),
        balloon_offset: PointPx { x: 300, y: 400 },
        balloon_offset_base: OffsetBase::unpinned(PointPx { x: 300, y: 400 }),
        balloon_limit,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

/// 実 spawn で World を組み、偽 `WindowHandle` を付けて単一ライターを通れるようにする。
pub(super) fn world_with(
    placement: ScopePlacement,
    snapshot: MonitorSnapshot,
) -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(snapshot);
    let titles = GhostTitles::from_scope_titles([(0, "むらさき".to_string())]);
    let ghost_windows = spawn_ghost_windows(&mut world, &[placement], &titles);
    let char_window = ghost_windows
        .char_window(0)
        .expect("キャラ窓が spawn される");
    let balloon = ghost_windows
        .balloon_window(0)
        .expect("バルーン窓が spawn される");
    world.entity_mut(char_window).insert(fake_handle(0x1000));
    world.entity_mut(balloon).insert(fake_handle(0x2000));
    (world, char_window, balloon)
}

/// [`base_placement`] を [`odd_edge_snapshot`] の上に実 spawn した World。
pub(super) fn base_world(balloon_limit: bool) -> (World, Entity, Entity) {
    world_with(base_placement(balloon_limit), odd_edge_snapshot())
}

/// `needle` を含むログが 1 件も無いこと。
pub(super) fn assert_no_event(events: &[CapturedEvent], needle: &str) {
    let hits: Vec<&str> = events
        .iter()
        .map(CapturedEvent::message)
        .filter(|m| m.contains(needle))
        .collect();
    assert!(
        hits.is_empty(),
        "`{needle}` を含むログが出ている（出てはならない）: {hits:?}"
    );
}

/// 縦横同じ値の DPI。
pub(super) fn dpi(v: u16) -> DPI {
    DPI::from_dpi(v, v)
}

/// 表示 DPI と作者基準 DPI の比（軸ごとの拡大率・app_scale=1）。
pub(super) fn k_axis(display_dpi: u32, author_dpi: u32) -> ScaleRatio {
    ScaleRatio::new(display_dpi, author_dpi).expect("非ゼロ DPI")
}

/// 「旧矩形は可視・新提案は全 work area 非交差」へ落ちるキャラ窓 World を組む。
///
/// 旧寸 [`wide_char_size`] の窓を、下端中央付替え（`resize_window_to` 手順 3b）後の
/// 中心が帯へ落ちる位置に置く。新寸 [`narrow_char_size`] は帯より狭いので、射影 T が
/// 出す提案矩形は帯へ収まり **どの work area とも交差しない**——S3 が言う
/// 「非ドラッグ要因で不可視へ遷移する」状態そのものを合成する。
pub(super) fn gap_bound_char_world(dpi: i32) -> (World, Entity, PointPx) {
    let old = wide_char_size(dpi);
    let old_pos = PointPx {
        x: gap_center_x(dpi) - old.w / 2,
        y: left_wa().bottom - old.h,
    };
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(old_pos.x, old_pos.y, old.w, old.h),
            Anchored(Anchor::Bottom),
        ))
        .id();
    (world, e, old_pos)
}

/// 行コメントを落とした本文（説明文の中の字面を数えないため）。
pub(crate) fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 実行時のモニタ表 1 台（3200×1800・96・主モニタ。作業領域の下端だけが関心）。
pub(crate) fn monitor_with_work_area_bottom(bottom: i32) -> Monitor {
    Monitor {
        handle: 1,
        bounds: RECT {
            left: 0,
            top: 0,
            right: 3200,
            bottom: 1800,
        },
        work_area: RECT {
            left: 0,
            top: 0,
            right: 3200,
            bottom,
        },
        dpi: 96,
        is_primary: true,
    }
}

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

/// 実 publisher（`spawn_sylphya`）と、同じ所を読み戻す口の組。
pub(super) struct SylphyaRig {
    pub(super) parts: SylphyaParts,
    roots: ScopeRoots,
    reader: Box<dyn PersistIo>,
}

impl SylphyaRig {
    /// 偽の記憶（共有ストア・Ghost の根は `/g`）。
    pub(super) fn fake() -> Self {
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
        Self {
            parts,
            roots,
            reader: Box::new(SharedFakeIo(store)),
        }
    }

    /// 実ファイル（`FsPersistIo`）。`profile_root` は Ghost スコープの保存先。
    pub(super) fn over_fs(profile_root: PathBuf) -> Self {
        let roots = ScopeRoots {
            ghost: Some(profile_root),
            ..ScopeRoots::default()
        };
        let parts = spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(FsPersistIo),
            runtime_sink: None,
        });
        Self {
            parts,
            roots,
            reader: Box::new(FsPersistIo),
        }
    }

    /// UI スレッド常駐の保存投函口を World へ挿す（persist_entries が引く NonSend リソース）。
    pub(super) fn wire(&self, world: &mut World) {
        world.insert_non_send(PersistWiring {
            publisher: self.parts.publisher.clone(),
        });
    }

    /// 投函済みの保存がすべて書き終わってから Ghost スコープを読む（同一送信端 FIFO）。
    pub(super) fn saved(&self) -> Vec<(PersistKey, String)> {
        self.parts
            .publisher
            .barrier()
            .expect("barrier should resolve while actor is alive");
        load_scope(PersistScope::Ghost, &self.roots, self.reader.as_ref())
    }

    /// 正典終了（アクター join）。
    pub(super) fn close(self) {
        self.parts.publisher.close();
        let _ = self.parts.handle.join();
    }
}

/// 最小解決可能ゴースト（`ghost/master/descript.txt`・`shell/master`）と保存先
/// `profile/areka` を一時フォルダに植える。返すのは（窓口・ゴーストの根・保存先）。
/// 窓口が落ちるとフォルダごと消える（panic をまたいでも）。
pub(super) fn plant_minimal_ghost(label: &str) -> (TempPath, PathBuf, PathBuf) {
    use areka_ghost::sylphya_wiring::profile_areka_root;

    let temp = TempPath::new(label);
    let root = temp.path().to_path_buf();
    let ghost_master = root.join("ghost").join("master");
    std::fs::create_dir_all(&ghost_master).expect("create ghost/master");
    std::fs::write(
        ghost_master.join("descript.txt"),
        "charset,UTF-8\nname,テスト\nsakura.name,さくら\n".as_bytes(),
    )
    .expect("write ghost descript");
    std::fs::create_dir_all(root.join("shell").join("master")).expect("create shell/master");
    // FsPersistIo::commit は親ディレクトリを作らないため、書込先を先に用意する。
    let profile_root = profile_areka_root(&ghost_master);
    std::fs::create_dir_all(&profile_root).expect("create profile/areka");
    (temp, root, profile_root)
}

/// 手順書 §3.3 の grep 判定語（`ClampX` で寄せた後も全 work area と非交差・本体とは独立の literal）。
pub(super) const HIDDEN_AFTER_CLAMP_TAG: &str = "[visibility-guard] HiddenAfterClampX";
