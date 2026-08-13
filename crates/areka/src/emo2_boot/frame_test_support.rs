use std::collections::BTreeMap;
use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc, Mutex};
use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_text::state::TextLayerConfig;
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
use tracing::field::{Field, Visit};
use tracing_subscriber::prelude::*;
use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use crate::placement::follow::MonitorSnapshot;
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;
use crate::placement::diag::WINDOW_MOVE_RECORD_TAG;
use crate::placement::test_support::LogEvent;
use wintf::ecs::{Point, SizeI, WindowHandle, WindowPos};
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::DPI;
use crate::emo2_boot::assets::{BalloonScopeAssets, BootAssets, LoopTables, ScopeAssets};

use super::*;

/// throwaway な `EmoWorld`（空 shell から build・`plan_attachments` は emo_world を読まない）。
///
/// `EmoWorld::build` は空 `Shell` に対し entity ゼロの空 World を返す寛容契約（COM/GPU 不要）。
fn empty_world() -> EmoWorld {
    EmoWorld::build(&areka_parsers::shell::parse(""))
}

/// 空アトラス（headless 構築・`plan_attachments` は atlas を読まない）。
fn empty_atlas() -> AtlasTable {
    AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
}

/// 合成 `BootAssets`（shell scope 集合と同一の balloon scope 集合を持つ標準形）。
///
/// `plan_attachments` が実際に読むのは `shells[*].scope`／`shells[*].initial_surface_id`／
/// `balloons[*].scope` のみ。残りのフィールド（emo_world／atlas／model／resolver／
/// static_binds）は最小の headless 値で埋める（COM/GPU/fixture 不要の純合成）。
pub(super) fn synth_assets(shells: &[(u32, u32)]) -> BootAssets {
    let balloon_scopes: Vec<u32> = shells.iter().map(|&(scope, _)| scope).collect();
    synth_assets_with_balloons(shells, &balloon_scopes)
}

/// balloon scope 集合を shell と独立に指定できる版（`balloon_index` の `None` 経路検証用）。
pub(super) fn synth_assets_with_balloons(shells: &[(u32, u32)], balloon_scopes: &[u32]) -> BootAssets {
    BootAssets {
        shells: shells
            .iter()
            .map(|&(scope, initial_surface_id)| ScopeAssets {
                scope,
                emo_world: empty_world(),
                atlas: empty_atlas(),
                initial_surface_id,
            })
            .collect(),
        balloons: balloon_scopes
            .iter()
            .map(|&scope| BalloonScopeAssets {
                scope,
                emo_world: empty_world(),
                atlas: empty_atlas(),
                model: areka_parsers::balloon::parse_str("", None),
            })
            .collect(),
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds: BindSet::default(),
        // plan_attachments は bind_resolver を読まない（headless 純合成）＝空表で十分。
        bind_resolver: BindResolver::empty(),
        // plan_attachments は loop_tables を読まない（headless 純合成）＝空表で十分。
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: BTreeMap::new(),
        },
        shell_author_dpi: 96,
        balloon_author_dpi: 96,
    }
}

// ── task 4.2: drain／text フェーズ＋emo2_frame_system の檻 ──────────────────────
//
// ログ発火を目視でなく実行テストで決定論的に檻へ入れるため、adapter.rs（task 2.5）と同じ
// スレッドローカル capture subscriber を単一ファイル境界（frame.rs）内へ最小インライン複製する。
// `EmoPresenter::apply` は未装着 target への `Hide`（reply: None）で `error!(?target_id, ...)` を
// 発火するため（presenter.rs `apply_hide`・reply-less でも log-first）、この ERROR を観測して
// drain の apply 到着順（FIFO）を強く檻に入れられる。

/// イベントの `level`＋各フィールドを 1 行文字列へ整形して共有 Vec へ push する最小 Layer。
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
    fn on_event(
        &self,
        ev: &tracing::Event<'_>,
        _: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let meta = ev.metadata();
        let mut line = format!("level={} target={}", meta.level(), meta.target());
        struct V<'a>(&'a mut String);
        impl Visit for V<'_> {
            fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                let _ = write!(self.0, " {}={:?}", f.name(), v);
            }
        }
        ev.record(&mut V(&mut line));
        self.0.lock().unwrap().push(line);
    }
}

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    tracing::subscriber::with_default(subscriber, f);
    let guard = logs.lock().unwrap();
    guard.clone()
}

/// 捕捉行のうち指定 level（例 `"ERROR"`）の件数を数える。
pub(super) fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|l| l.contains(&needle)).count()
}

/// epoch を確立しない固定クロック（drain 系テストは talk_time を使わない）。
pub(super) fn zero_clock() -> TalkClock {
    TalkClock::new(Arc::new(|| 0.0))
}

/// headless な `Emo2Wiring`（実 `EmoPresenter`／空 `TextLayerRuntime`／合成 `BootAssets`）を
/// 注入 `rx`／`clock` で組む（task 4.1 のゲートテストと同型・COM/GPU 不要）。
pub(super) fn headless_wiring_with(rx: Receiver<PresentCommand>, clock: TalkClock) -> Emo2Wiring {
    Emo2Wiring::new(
        EmoPresenter::new(),
        rx,
        mpsc::channel::<MoveDirective>().1,
        mpsc::channel::<crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))),
        clock,
        synth_assets(&[(0, 0)]),
    )
}

/// 偽 HWND の WindowHandle（実窓なし・headless 決定論シーム・follow.rs の fake_handle 相当）。
fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// resnap 檻の 2 スコープ解決済み配置（both Bottom・初期位置は work_area 下端に整合＝
/// bottom 不変量を満たす: scope0 y=1444−687=757／scope1 y=1444−357=1087）。
fn resnap_placements() -> Vec<ScopePlacement> {
    vec![
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 757 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 732 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            anchor: Anchor::Bottom,
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1087 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1068 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            anchor: Anchor::Bottom,
        },
    ]
}

/// 実 work area（原点非 (0,0)・96 非倍数の合成値・bottom=1444＝配置と整合）。
fn resnap_work_area() -> RectPx {
    RectPx {
        left: 31,
        top: 17,
        right: 2574,
        bottom: 1444,
    }
}

/// spawn_ghost_windows で 2 スコープの窓を組み、各窓へ偽 WindowHandle を付与し
/// MonitorSnapshot を挿入した World を返す（char 窓は spawn が Anchored/WindowPos を付ける）。
pub(super) fn resnap_world() -> (World, GhostWindows) {
    let placements = resnap_placements();
    let mut world = World::new();
    let gw = spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles([(0, "a".to_string()), (1, "b".to_string())]),
    );
    // 偽 WindowHandle 付与（enqueue_window_set_pos が WindowPos を書けるように＝
    // resize_window_to の反映口が成立する条件）。
    let mut raw = 0x100usize;
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).unwrap(),
            gw.balloon_window(scope).unwrap(),
        ] {
            world.entity_mut(e).insert(fake_handle(raw));
            raw += 0x10;
        }
    }
    // MonitorSnapshot（project_anchor Bottom が下端 live 算出に用いる）。
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![resnap_work_area()],
    });
    (world, gw)
}

pub(super) fn size_of(world: &World, e: Entity) -> Option<SizeI> {
    world.get::<WindowPos>(e).and_then(|wp| wp.size)
}
pub(super) fn pos_of(world: &World, e: Entity) -> Option<Point> {
    world.get::<WindowPos>(e).and_then(|wp| wp.position)
}

/// resnap が引く物理寸を target ごとに作り分ける fake（[`PhysicalSizeSource`] の檻用実装）。
///
/// shell（偶数 id）と balloon（奇数 id）で**異なる寸**を返し、問い合わせられた `TargetId` を
/// 記録する。これにより「resnap がどちらを読んだか」が窓ジオメトリと問い合わせ記録の
/// 二重の観測面で判別できる（実 `EmoPresenter` は装着＋`ShowSurface` 完了＝GPU が要り、
/// 未装着だと全 target が `None` に潰れて判別不能になる——それが変異生存の穴だった）。
pub(super) struct FakeSizes {
    shell: (u32, u32),
    balloon: (u32, u32),
    pub(super) queried: std::cell::RefCell<Vec<u32>>,
}

impl FakeSizes {
    pub(super) fn new(shell: (u32, u32), balloon: (u32, u32)) -> Self {
        FakeSizes {
            shell,
            balloon,
            queried: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl PhysicalSizeSource for FakeSizes {
    fn physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
        self.queried.borrow_mut().push(target.0);
        // target_map: shell=2*scope（偶数）／balloon=2*scope+1（奇数）。
        if target.0 % 2 == 0 {
            Some(self.shell)
        } else {
            Some(self.balloon)
        }
    }
}

/// 単一ライター経路を通ったか否かの witness 用 sentinel（実位置と重ならない値）。
pub(super) const WRITER_WITNESS: Offset = Offset { x: -1.0, y: -1.0 };

/// spawn 時 offset 付きの `Arrangement`（実 pipeline の spawn 位置を模す・follow.rs 檻と同型）。
fn arrangement_at(x: f32, y: f32) -> Arrangement {
    Arrangement {
        offset: Offset { x, y },
        ..Default::default()
    }
}

/// entity の `Arrangement.offset` を読む（未付与は panic で検出）。
pub(super) fn arrangement_offset_of(world: &World, entity: Entity) -> Offset {
    world
        .get::<Arrangement>(entity)
        .expect("Arrangement があるはず")
        .offset
}

/// 単一ライター経路を通っていない＝窓書込ゼロ（sentinel が据え置かれている）。
pub(super) fn assert_no_write(world: &World, entity: Entity, what: &str) {
    assert_eq!(
        arrangement_offset_of(world, entity),
        WRITER_WITNESS,
        "{what}: 単一ライター経路を通った痕跡がある（書込ゼロのはず）"
    );
}

/// 全窓の書込 witness を sentinel へ戻す（フェーズ境界で「以降の書込」だけを見るため）。
pub(super) fn reset_write_witness(world: &mut World, gw: &GhostWindows) {
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).expect("char 窓がある"),
            gw.balloon_window(scope).expect("balloon 窓がある"),
        ] {
            world
                .entity_mut(e)
                .insert(arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y));
        }
    }
}

/// resnap 檻の World（2 スコープ・偽 HWND・MonitorSnapshot）へ、書込 witness の
/// `Arrangement`（sentinel）と `DPI`（96＝author_dpi 既定と等倍）を全窓へ付与した DPI 相の檻。
pub(super) fn dpi_world() -> (World, GhostWindows) {
    let (mut world, gw) = resnap_world();
    for scope in gw.scopes().collect::<Vec<_>>() {
        for e in [
            gw.char_window(scope).expect("char 窓がある"),
            gw.balloon_window(scope).expect("balloon 窓がある"),
        ] {
            world.entity_mut(e).insert((
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
                DPI::from_dpi(96, 96),
            ));
        }
    }
    (world, gw)
}

/// 決定論 fake の再スケール報告源（GPU 不要で**二経路の結線**を檻に入れる）。
///
/// `EmoPresenter` の消費規約（`refresh_scale` が再表示成立時に自ら `pending_resize` を
/// take して返す／ゲート不成立なら一切触れない）を写した最小の fake。実 presenter 側の
/// 規約そのものは emo-present in-crate テストが所有し、ここでは **frame 側の結線**
/// （両経路を毎フレーム呼ぶ・`Some` のみ reconcile する）だけを見る。
#[derive(Default)]
pub(super) struct FakeReports {
    /// `refresh_scale_report` が返す報告（target 番号→物理寸・取り出しで消える）。
    pub(super) refresh: BTreeMap<u32, (u32, u32)>,
    /// `take_scale_report` が返す未消費報告（同上）。
    pub(super) pending: BTreeMap<u32, (u32, u32)>,
    /// 呼出記録（`("refresh"|"take", target 番号)`・呼ばれたこと自体の非空虚性検査用）。
    pub(super) calls: Vec<(&'static str, u32)>,
}

impl FakeReports {
    /// 指定種別の呼出だけを target 番号の列として取り出す。
    pub(super) fn calls_of(&self, kind: &str) -> Vec<u32> {
        self.calls
            .iter()
            .filter(|(k, _)| *k == kind)
            .map(|(_, t)| *t)
            .collect()
    }
}

impl ScaleReportSource for FakeReports {
    fn refresh_scale_report(
        &mut self,
        _world: &mut World,
        target: TargetId,
    ) -> Option<(u32, u32)> {
        self.calls.push(("refresh", target.0));
        let report = self.refresh.remove(&target.0);
        if report.is_some() {
            // presenter 規約: 再表示が成立したなら状態照合が積んだ要求は本メソッドが消費する
            // （同一フレームの drain が二度目を拾わない）。
            self.pending.remove(&target.0);
        }
        report
    }

    fn take_scale_report(&mut self, target: TargetId) -> Option<(u32, u32)> {
        self.calls.push(("take", target.0));
        self.pending.remove(&target.0)
    }
}

/// S2 檻のモニタ物理下端。モニタ境界は物理量ゆえ拡大率では動かない（work area だけが動く）。
const S2_MONITOR_BOTTOM: i32 = 1492;
/// S2 檻のタスクバー**論理**高。実機のタスクバーは論理寸で宣言され `dpi/96` 倍で物理へ伸びる。
const S2_TASKBAR_LOGICAL_H: i32 = 48;

/// ゴーストが居るモニタの work area を **DPI 水準の関数**として組む（Req 5.6: 絶対 px を
/// 判定に直書きしない）。
///
/// 拡大率が上がるとタスクバーの物理高が `dpi/96` 倍に伸び、work area 下端がその分だけ
/// 上がる——これが「DPI が変われば接地すべき下端が変わる」ことの機構であり、S2 が落として
/// いるのはまさにこの再導出である。`dpi=96` では `1492−48=1444`＝[`resnap_placements`] の
/// 初期配置がちょうど満たす下端であり、**96 だけが旧 Y と新 Y の自己整合で通過する**
/// （Req 5.1／5.4・診断レポート §1.2「なぜ dpi=96 では隠れるか」）。
pub(super) fn s2_work_area_for_dpi(dpi: u16) -> RectPx {
    let taskbar_px = S2_TASKBAR_LOGICAL_H * dpi as i32 / 96;
    RectPx {
        left: 31,
        top: 17,
        right: 2574,
        bottom: S2_MONITOR_BOTTOM - taskbar_px,
    }
}

/// 合成マルチモニタの隣接モニタ（負座標・3200 超座標を含む非対称レイアウト・task 2.2 と同流儀）。
///
/// ゴースト窓の中心は決してここへ入らない。work area 解決が**帰属**（`Contains`）で決まって
/// いることを [`s2_resolved_work_area`] が毎回自己検査するため、単一モニタで回したときの
/// 「解決するモニタが 1 つしか無いから自明に当たる」退化を排除できる。
fn s2_neighbor_work_area() -> RectPx {
    RectPx {
        left: 2574,
        top: -140,
        right: 3874,
        bottom: 1901,
    }
}

/// 当該 DPI 水準における合成マルチモニタ snapshot（index 0＝ゴーストの居るモニタ）。
pub(super) fn s2_snapshot(dpi: u16) -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![s2_work_area_for_dpi(dpi), s2_neighbor_work_area()],
    }
}

/// 窓の**接地点**（下端中央・物理 px）。伺かの立ち絵は足元中央が原点であり、寸法変動でも
/// 動かない（記憶〈キャラ窓の原点は下端中央〉・[`resize_window_to`] 手順 3b）。
pub(super) fn s2_ground_point(world: &World, e: Entity) -> (i32, i32) {
    let pos = pos_of(world, e).expect("WindowPos.position がある");
    let size = size_of(world, e).expect("WindowPos.size がある");
    (pos.x + size.width / 2, pos.y + size.height)
}

/// 探針の**非退化**自己検査（記憶〈2.2 の空虚性の教訓〉＝不変を主張する檻は、探針が欠陥の
/// 不動点に落ちていないかを自ら確かめる）。
///
/// work area 下端が DPI 水準の間で実際に動かなければ、再射影の欠落はどの探針値でも観測
/// できず、檻は「何も起きないから通る」空虚な緑になる。将来 [`s2_work_area_for_dpi`] が
/// 編集されて退化しても、この assert が先に落ちる。
pub(super) fn s2_assert_work_area_bottom_moves(from_dpi: u16, to_dpi: u16) {
    let from = s2_work_area_for_dpi(from_dpi).bottom;
    let to = s2_work_area_for_dpi(to_dpi).bottom;
    assert_ne!(
        from, to,
        "探針が退化している: dpi {from_dpi}→{to_dpi} で work area 下端が動かない（この合成レイアウトでは S2 を観測できない）"
    );
}

/// 捕捉イベントから窓移動レコード行だけを抜く（他の debug ログは無視）。
pub(super) fn window_move_lines(events: &[LogEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with(WINDOW_MOVE_RECORD_TAG))
        .collect()
}

/// 指定 entity の窓移動レコードから `route=` の語だけを取り出す（出現順）。
pub(super) fn window_move_routes_of(events: &[LogEvent], entity: Entity) -> Vec<String> {
    let key = format!("entity={entity:?} ");
    window_move_lines(events)
        .iter()
        .filter(|line| line.contains(&key))
        .filter_map(|line| {
            line.split_whitespace()
                .find_map(|tok| tok.strip_prefix("route=").map(str::to_string))
        })
        .collect()
}
