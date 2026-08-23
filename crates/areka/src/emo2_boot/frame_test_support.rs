use crate::emo2_boot::assets::{BalloonScopeAssets, BootAssets, LoopTables, ScopeAssets};
use crate::placement::diag::WINDOW_MOVE_RECORD_TAG;
use crate::placement::follow::{MonitorDpiTable, MonitorSnapshot, MonitorSources};
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;
use crate::placement::test_support::LogEvent;
use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_text::state::TextLayerConfig;
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
use bevy_ecs::prelude::Entity;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use bevy_ecs::system::SystemState;
use log_capture_kit::{LineFormat, capture_lines};
use std::collections::BTreeMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, mpsc};
use std::time::Instant;
use windows::Win32::Foundation::{HINSTANCE, HWND, RECT};
use wintf::ecs::DPI;
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::window::monitor::Monitor;
use wintf::ecs::window::transition_diag::{self, TickStart};
use wintf::ecs::window::{SetWindowPosCommand, drain_window_pos_commands};
use wintf::ecs::{FrameCount, Point, SizeI, WindowHandle, WindowPos};

use super::work_area_sync::{SnapshotChange, resnap_for_work_area_change, sync_monitor_snapshot};

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
pub(super) fn synth_assets_with_balloons(
    shells: &[(u32, u32)],
    balloon_scopes: &[u32],
) -> BootAssets {
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
// ログ発火を目視でなく実行テストで決定論的に檻へ入れる。捕捉層そのものはここに持たず、
// 硬化機構の唯一の定義元 `log-capture-kit` へ委譲する（行の形も呼出側の判定内容も移行前と
// 1 バイト変わらない）。`EmoPresenter::apply` は未装着 target への `Hide`（reply: None）で
// `error!(?target_id, ...)` を発火するため（presenter.rs `apply_hide`・reply-less でも
// log-first）、この ERROR を観測して drain の apply 到着順（FIFO）を強く檻に入れられる。
//
// 「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」は**誤り**である。差し替わる
// のはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
// interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点を最初に踏んだスレッドの判定が
// 焼き付く（先着が勝つ）。捕捉窓を持たないスレッドの既定は `NoSubscriber` で判定は「不要」ゆえ、
// 先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。
// 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 捕捉窓の内側での interest 再計算 ⑶ 番兵イベントに
// よる空振り検出 の 3 点でこれを塞ぐ（機序の逐条解説は `log_capture_kit` の crate doc と
// 同 crate の `src/probe.rs`）。

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
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
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
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
///
/// # なぜ `balloon_limit` を無効にしてあるか（areka-P0-windowposition-limit task 3.3）
///
/// 本 fixture のキャラ窓は下端接地で背が高く（scope0 は 687 論理 px）、192 水準では
/// 高さ 1374 px となって work area（top=17）をほぼ埋める。バルーンはキャラ窓の
/// **上** 25 px にある（`balloon_offset.y = −25`）ため、192 水準の追従位置は
/// work area の上辺を 20 px はみ出す——`limit=1` なら runtime 関門
/// （`enqueue_window_set_pos` 内）が正しく上辺へ引き戻す。
///
/// その補正は本 fixture を使う檻の**主題ではない**。ここが固定しているのは DPI 相の
/// 再射影と窓相対の追従 offset 契約（`balloon_pos − char_pos ≡ offset`）であり、
/// 補正が混ざると恒等式が成立しなくなって主題が読めなくなる（設計 DD6: 恒等式は
/// 「resolver／追従計算の出力時点の事後条件」であって表示位置の恒久不変量ではない）。
/// ゆえに limit を無効にして関門を素通しさせる。limit の挙動そのものは
/// `placement/follow_balloon_limit_tests.rs` が所有し、DPI 起因の寸法変更経路
/// （`resize_window_keep_position`）もそこで補正が掛かることを固定している。
fn resnap_placements() -> Vec<ScopePlacement> {
    vec![
        ScopePlacement {
            scope: 0,
            char_pos: PointPx { x: 1483, y: 757 },
            char_size: SizePx { w: 434, h: 687 },
            balloon_pos: PointPx { x: 1071, y: 732 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: -412, y: -25 },
            // windowposition-limit: **無効**（task 3.3 の追随）。理由は下の注記を参照。
            balloon_limit: false,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
        },
        ScopePlacement {
            scope: 1,
            char_pos: PointPx { x: 1049, y: 1087 },
            char_size: SizePx { w: 278, h: 357 },
            balloon_pos: PointPx { x: 1334, y: 1068 },
            balloon_size: SizePx { w: 223, h: 158 },
            balloon_offset: PointPx { x: 285, y: -19 },
            // windowposition-limit: **無効**（task 3.3 の追随）。理由は下の注記を参照。
            balloon_limit: false,
            anchor: Anchor::Bottom,
            balloon_keyword_base: None,
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
    fn refresh_scale_report(&mut self, _world: &mut World, target: TargetId) -> Option<(u32, u32)> {
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
pub(super) fn s2_neighbor_work_area() -> RectPx {
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

/// 当該 DPI 水準における合成マルチモニタの**実行時のモニタ表**（`Monitor` 2 台）。
///
/// [`s2_snapshot`] と同一レイアウトであり、[`MonitorSources::from_monitors`] へ通すと
/// 同じ作業領域集合になる——同期段の「表と源が一致していれば作り直さない」を、偶然の
/// 一致ではなく**同じ列から作られたこと**として組める。
pub(super) fn s2_monitors(dpi: u16) -> Vec<Monitor> {
    let neighbor = s2_neighbor_work_area();
    vec![
        harness_monitor(
            1,
            s2_work_area_for_dpi(dpi),
            S2_MONITOR_BOTTOM,
            u32::from(dpi),
        ),
        harness_monitor(2, neighbor, neighbor.bottom, u32::from(dpi)),
    ]
}

/// 当該 DPI 水準の 2 源（作業領域源＋モニタ別拡大率表）を、本番と同じ構築関数で作る。
pub(super) fn s2_sources(dpi: u16) -> MonitorSources {
    MonitorSources::from_monitors(&s2_monitors(dpi))
}

/// タスクバーを隠したときの、ゴーストが居るモニタの作業領域（下端＝モニタ物理下端）。
///
/// **拡大率を一切変えずに作業領域だけが動く**構成変更の代表例である（タスクバーの自動的に
/// 隠す設定・位置の変更・多段の表示切替）。この向きの変化では窓の `DPI` が 1 つも変わらない
/// ので `Changed<DPI>` は立たず、拡大率の相は何もしない——だから作業領域変化を契機とする
/// 再スナップが要る（task 5.2・要件 5.1／5.2）。
pub(super) fn s2_taskbar_hidden_work_area(dpi: u16) -> RectPx {
    RectPx {
        bottom: S2_MONITOR_BOTTOM,
        ..s2_work_area_for_dpi(dpi)
    }
}

/// ゴーストが居るモニタ（index 0）の作業領域だけを差し替えた実行時のモニタ表。
///
/// 拡大率（`Monitor.dpi`）とモニタ矩形（`bounds`）は [`s2_monitors`] のまま据え置く
/// ——動くのは作業領域だけである。
pub(super) fn s2_monitors_with_work_area(dpi: u16, work_area: RectPx) -> Vec<Monitor> {
    let mut monitors = s2_monitors(dpi);
    monitors[0].work_area = RECT {
        left: work_area.left,
        top: work_area.top,
        right: work_area.right,
        bottom: work_area.bottom,
    };
    monitors
}

/// 隣接モニタ（index 1・ゴーストが決して居ない側）の作業領域だけを差し替えた表。
///
/// 表そのものは変わる（同期段は作り直す）が、ゴースト窓の接地点に関わる作業領域は 1px も
/// 動かない——「表が変わっただけでは窓を書かない」を、同期段が実際に発火した状態で問う
/// ための構成である（要件 5.2 の「変化が無ければ再射影そのものを行わない」の窓ごとの側）。
pub(super) fn s2_monitors_with_neighbor_work_area(dpi: u16, work_area: RectPx) -> Vec<Monitor> {
    let mut monitors = s2_monitors(dpi);
    monitors[1].work_area = RECT {
        left: work_area.left,
        top: work_area.top,
        right: work_area.right,
        bottom: work_area.bottom,
    };
    monitors
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

// ── task 3.3: 多フレーム駆動ハーネス（要件 7.2／7.5／7.6／7.7）────────────────
//
// 遷移は 1 フレームでは終わらない——作業領域源の差替・窓の拡大率の変化・報告寸の landing・
// 連鎖の解き直しが**別々のフレーム**に落ちる。本節はそれを決定論で並べるための駆動口を
// 与える（実 GPU も実高 DPI モニタも要らない＝要件 7.2）。

/// 常時テストは純 x64（要件 7.5）。i686 で本ハーネスを組もうとしたらビルドで止める
/// ——「32bit でも走ってしまった」を実行時の観察に委ねない。
const _: () = assert!(
    cfg!(target_pointer_width = "64"),
    "多フレーム駆動ハーネスの常時テストは x64 のみで走る（要件 7.5）"
);

/// scope ごとに物理寸を作り分ける [`PhysicalSizeSource`] の fake。
///
/// [`FakeSizes`] は shell／balloon の 2 値しか返せず、「全スコープの寸が窓と一致しているか」を
/// 扱う連鎖・再スナップの檻には粒度が足りない。値 `None` は「表示未成立（初回 `ShowSurface`
/// 前）」を表す。
pub(super) struct PerTargetSizes(BTreeMap<u32, Option<(u32, u32)>>);

impl PerTargetSizes {
    /// scope → shell 物理寸（`None` は未表示）を与える。
    pub(super) fn new<I: IntoIterator<Item = (usize, Option<(u32, u32)>)>>(entries: I) -> Self {
        Self(
            entries
                .into_iter()
                .map(|(scope, size)| (shell_target(scope as u32).0, size))
                .collect(),
        )
    }
}

impl PhysicalSizeSource for PerTargetSizes {
    fn physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
        self.0.get(&target.0).copied().flatten()
    }
}

/// spawn 時の実寸（[`resnap_placements`] と一致）。窓の `WindowPos.size` と揃える基準値。
pub(super) const SPAWN_SIZE_0: (u32, u32) = (434, 687);
/// 同上（後続スコープ）。
pub(super) const SPAWN_SIZE_1: (u32, u32) = (278, 357);

/// 全スコープが表示成立し、寸が窓と一致している状態の fake。
pub(super) fn settled_sizes() -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, Some(SPAWN_SIZE_1))])
}

/// **単一スレッド実行器**を明示した空スケジュール（要件 7.6）。
///
/// ログ捕捉（`log-capture-kit` の捕捉窓＝**呼出スレッド局所**の dispatcher 差替）に依る
/// 検証を既定の多スレッド実行器で回すと、システムがワーカースレッドへ載って 1 行も捕捉
/// できず、檻が空虚に緑になる。捕捉に依る検証はここから組んだスケジュールで走らせ、
/// 「捕捉できる実行形態」を呼び出し側の字面に残す。
pub(super) fn single_threaded_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule
}

/// 合成モニタ 1 台（実行時のモニタ表＝`Monitor` entity 群の要素）。
fn harness_monitor(handle: isize, work_area: RectPx, monitor_bottom: i32, dpi: u32) -> Monitor {
    Monitor {
        handle,
        bounds: RECT {
            left: work_area.left,
            top: work_area.top,
            right: work_area.right,
            bottom: monitor_bottom,
        },
        work_area: RECT {
            left: work_area.left,
            top: work_area.top,
            right: work_area.right,
            bottom: work_area.bottom,
        },
        dpi,
        is_primary: handle == 1,
    }
}

/// 多フレーム駆動ハーネス。
///
/// [`dpi_world`] の檻（2 スコープ・偽 `WindowHandle`・書込 witness・`DPI` 96）を土台に、
/// **フレームを進めながら 3 つの源を差し替える**口を足したもの:
///
/// | 源 | 差替口 | 実体 |
/// |---|---|---|
/// | 作業領域源 | [`set_work_area_source`](Self::set_work_area_source) | `MonitorSnapshot` 資源 |
/// | モニタ別拡大率表 | [`set_monitor_dpi_table`](Self::set_monitor_dpi_table) | `MonitorDpiTable` 資源 |
/// | 実行時のモニタ表 | [`set_monitor_table`](Self::set_monitor_table) | `Monitor` entity 群 |
/// | 窓の拡大率 | [`set_window_dpi`](Self::set_window_dpi)／[`set_scope_dpi`](Self::set_scope_dpi) | 各窓の `DPI` |
///
/// 前 2 者は同期段（[`run_work_area_sync`](Self::run_work_area_sync)）が作り直す側であり、
/// 3 つ目はその入力である。**独立に**差し替えられることが要件 5.3 の観測（接地点と作業領域
/// 下端の差）の前提である——同じ源から両方を引いたら差は定義上つねに 0 になる。
///
/// # テスト間で状態を残さない（要件 7.7）
///
/// 後始末は **[`FrameHarness::new`] と `Drop` の両側**にある——窓書込キューの取り出し
/// （`drain_window_pos_commands`）と観測の写しの初期化（`transition_diag::reset_for_test`）を
/// 冒頭と末尾の双方で行う。
///
/// `new` の側が要るのは、ハーネスを使わない経路が残した汚れも掬うためである（この向きだけを
/// 残留の検査が固定している）。`Drop` の側が要るのは、ハーネスを使ったテストの**次に走るのが
/// ハーネスでない**場合に、キューと写しが汚れたまま渡ってしまうためである——この向きは
/// areka 側に現在読み手が居ないので**どのテストでも赤にできない**（task 3.3 のレビューで確認）。
/// 検査できない残留を「隔離済み」と言わないために、両側で閉じる。
/// なお `Drop` はスコープ末尾で走るので、ハーネスを生かしたまま次を組む形（残留の検査そのもの）
/// は壊れない。
///
/// スコープを跨ぐ資源はほかに World 単位の `FrameCount`／`TickStart` があるが、これらは
/// ハーネスが**自前の `World`** に持つので、そもそもテスト間で共有されない。
///
pub(super) struct FrameHarness {
    /// 駆動対象の World（2 スコープぶんのゴースト窓・作業領域源・モニタ表を持つ）。
    pub(super) world: World,
    /// 窓 entity の正本（`GhostWindows` 資源と同じ内容の写し）。
    ghost_windows: GhostWindows,
    /// スコープ番号（昇順）。
    scopes: Vec<usize>,
    /// 進めたフレーム番号（`new` 直後は 0＝tick 前）。
    frame: u32,
    /// DPI 相の永続 `SystemState`（run を跨いで `last_run` を保つ＝毎 run 全窓誤マッチの回避）。
    dpi_state: Option<SystemState<DpiChangedQuery>>,
    /// 実行時のモニタ表として spawn した entity 群（差替時に消す対象）。
    monitors: Vec<Entity>,
}

impl FrameHarness {
    /// 残留の無い状態から多フレーム駆動を始める（要件 7.7）。
    pub(super) fn new() -> Self {
        // 前のテストが残した窓書込指令を捨てる（**実行はしない**＝実 `SetWindowPos` を呼ばない）。
        let _residue = drain_window_pos_commands();
        // 観測の写し（直近 tick のフレーム番号・flush 区間の起点）を初期化する。
        transition_diag::reset_for_test();

        let (world, ghost_windows) = dpi_world();
        let scopes: Vec<usize> = ghost_windows.scopes().collect();
        FrameHarness {
            world,
            ghost_windows,
            scopes,
            frame: 0,
            dpi_state: None,
            monitors: Vec::new(),
        }
    }

    /// 直近に進めたフレーム番号（`new` 直後は 0）。
    pub(super) fn frame(&self) -> u32 {
        self.frame
    }

    /// 駆動対象のスコープ（昇順）。
    pub(super) fn scopes(&self) -> &[usize] {
        &self.scopes
    }

    /// 当該スコープのキャラ窓。
    pub(super) fn char_window(&self, scope: usize) -> Entity {
        self.ghost_windows
            .char_window(scope)
            .expect("char 窓がある")
    }

    /// 当該スコープのバルーン窓。
    pub(super) fn balloon_window(&self, scope: usize) -> Entity {
        self.ghost_windows
            .balloon_window(scope)
            .expect("balloon 窓がある")
    }

    /// フレームを 1 つ進め、新しいフレーム番号を返す。
    ///
    /// 刻印の 2 つの権威を `try_tick_world` と**同一の規律**で進める（D1）: World 資源
    /// （`FrameCount`＋`TickStart`）を更新し、同じ値をスレッド局所の写しへ配る。片方だけ
    /// 進めると、World を借りる観測点と借りない観測点でフレーム系列が割れる。
    pub(super) fn advance_frame(&mut self) -> u32 {
        self.frame += 1;
        let tick_start = Instant::now();
        self.world.insert_resource(FrameCount(self.frame));
        self.world.insert_resource(TickStart(tick_start));
        transition_diag::begin_tick(self.frame, tick_start);
        self.frame
    }

    /// 作業領域源（`MonitorSnapshot` 資源）を差し替える。
    pub(super) fn set_work_area_source(&mut self, snapshot: MonitorSnapshot) {
        self.world.insert_resource(snapshot);
    }

    /// 当該 DPI 水準の合成マルチモニタ（[`s2_snapshot`]）を作業領域源に据える。
    pub(super) fn set_work_area_source_for_dpi(&mut self, dpi: u16) {
        self.set_work_area_source(s2_snapshot(dpi));
    }

    /// 実行時のモニタ表（`Monitor` entity 群）を差し替える（既存の表は消える）。
    pub(super) fn set_monitor_table(&mut self, monitors: Vec<Monitor>) {
        for entity in std::mem::take(&mut self.monitors) {
            self.world.despawn(entity);
        }
        self.monitors = monitors
            .into_iter()
            .map(|monitor| self.world.spawn(monitor).id())
            .collect();
    }

    /// 当該 DPI 水準の合成マルチモニタを実行時のモニタ表に据える（[`s2_snapshot`] と同一
    /// レイアウト・**作業領域源とは独立に**差し替わる）。
    pub(super) fn set_monitor_table_for_dpi(&mut self, dpi: u16) {
        self.set_monitor_table(s2_monitors(dpi));
    }

    /// モニタ別拡大率表（`MonitorDpiTable` 資源）を差し替える（task 5.1 で新設した源）。
    pub(super) fn set_monitor_dpi_table(&mut self, table: MonitorDpiTable) {
        self.world.insert_resource(table);
    }

    /// 当該 DPI 水準の 2 源（作業領域源＋モニタ別拡大率表）を**同時に**据える。
    ///
    /// 起動直後の状態（本番の起動シームが 2 源を同時に挿す形）を 1 行で作るための口。
    /// 片方だけを古くした状態を組みたいときは 2 つの差替口を別々に呼ぶ。
    pub(super) fn set_monitor_sources_for_dpi(&mut self, dpi: u16) {
        let sources = s2_sources(dpi);
        self.set_work_area_source(sources.snapshot);
        self.set_monitor_dpi_table(sources.dpi_table);
    }

    /// 現在の作業領域源（差し替えが起きたかを値で確かめるための読み口）。
    pub(super) fn work_area_source(&self) -> Option<&MonitorSnapshot> {
        self.world.get_resource::<MonitorSnapshot>()
    }

    /// 現在のモニタ別拡大率表（同上）。
    pub(super) fn monitor_dpi_table(&self) -> Option<&MonitorDpiTable> {
        self.world.get_resource::<MonitorDpiTable>()
    }

    /// 作業領域源の同期を 1 回回す（本番の毎フレーム先頭の相と同一の関数）。
    pub(super) fn run_work_area_sync(&mut self) -> Option<SnapshotChange> {
        sync_monitor_snapshot(&mut self.world)
    }

    /// 全スコープの全窓（キャラ・バルーン）の拡大率を差し替える（OS 設定の拡大率変更）。
    pub(super) fn set_window_dpi(&mut self, dpi: u16) {
        for scope in self.scopes.clone() {
            self.set_scope_dpi(scope, dpi);
        }
    }

    /// 当該スコープの 2 窓だけ拡大率を差し替える（別モニタ・段階的な受理の再現）。
    pub(super) fn set_scope_dpi(&mut self, scope: usize, dpi: u16) {
        for entity in [self.char_window(scope), self.balloon_window(scope)] {
            self.world
                .entity_mut(entity)
                .insert(DPI::from_dpi(dpi, dpi));
        }
    }

    /// DPI 相を 1 回回す（`SystemState` はハーネスが run を跨いで保持する）。
    pub(super) fn run_dpi_phase<S: ScaleReportSource>(&mut self, source: &mut S) {
        dpi_phase_with(source, &mut self.dpi_state, &mut self.world);
    }

    /// 作業領域変化を契機とする再スナップを 1 回回す（本番の拡大率の相の直後の段）。
    pub(super) fn run_work_area_resnap(&mut self, change: &SnapshotChange) {
        resnap_for_work_area_change(&mut self.world, change);
    }

    /// 配置に関わる相を**本番と同じ順**で 1 フレームぶん回す
    /// （同期段 → 拡大率の相 → 作業領域再スナップ）。
    ///
    /// 相を 1 つずつ呼ぶ形だと、本番（`emo2_frame_system`）の並びが入れ替わってもテストは
    /// 自分の並びで緑になる。ここに順序を 1 箇所だけ写し、置き場を主題にするテストは
    /// この口から駆動する（本番の字面そのものは本文走査のテストが別に固定する）。
    ///
    /// 戻り値は同期段が作業領域源を差し替えたか（`None`＝表に変化が無かったフレーム）。
    pub(super) fn run_placement_phases<S: ScaleReportSource>(
        &mut self,
        source: &mut S,
    ) -> Option<SnapshotChange> {
        let change = self.run_work_area_sync();
        self.run_dpi_phase(source);
        if let Some(change) = &change {
            self.run_work_area_resnap(change);
        }
        change
    }

    /// 報告寸の突合を 1 回回す（本番の drain 相・可視性の相の後段と同一の関数）。
    ///
    /// 待ち札のある窓へは、この口からも窓書込が出ないことを問う（設計 C5 の「4 点すべて」の
    /// 2 つ目）。会話中の表情差替（`ShowSurface`）が積む窓寸要求は本番ではこの経路で
    /// landing するので、待ち中の表情差替を組むにはこの駆動口が要る。
    pub(super) fn run_reconcile<S: ScaleReportSource>(&mut self, source: &mut S) {
        reconcile_reported_sizes(source, &mut self.world);
    }

    /// 再スナップを 1 回回す。
    pub(super) fn run_resnap<S: PhysicalSizeSource + ?Sized>(&mut self, source: &S) {
        resnap_with(source, &mut self.world);
    }

    /// 連鎖確定を 1 回回す（一度きりの契約は本番同様・`ChainFinalized` で守られる）。
    pub(super) fn run_chain_finalize<S: PhysicalSizeSource + ?Sized>(&mut self, source: &S) {
        finalize_chain_once_with(source, &mut self.world);
    }

    /// 遷移後の連鎖再解決を 1 回回す（本番の連鎖確定の直後の段と同一の関数）。
    ///
    /// 一度きりの契約（遷移 1 回につき武装→解決の 1 往復）は本番同様
    /// `ChainRealignPending` で守られるので、この口を何度呼んでも 2 度目以降は無操作である。
    pub(super) fn run_chain_realign<S: PhysicalSizeSource + ?Sized>(&mut self, source: &S) {
        realign_chain_once_with_source(source, &mut self.world);
    }

    /// 積まれた窓書込指令を**実行せずに**取り出す（D11 の検査口）。
    ///
    /// 取り出す先はスレッド全体で 1 本のキューであり、ハーネスごとに分かれていない。
    /// 同一スレッドで 2 つのハーネスを同時に生かすと、どちらの `drain_writes` も
    /// 両者の書込を取り出す。
    pub(super) fn drain_writes(&mut self) -> Vec<SetWindowPosCommand> {
        drain_window_pos_commands()
    }

    /// 全窓の書込 witness を sentinel へ戻す（フェーズ境界で「以降の書込」だけを見る）。
    pub(super) fn reset_write_witness(&mut self) {
        reset_write_witness(&mut self.world, &self.ghost_windows);
    }

    /// 当該スコープのキャラ窓の接地点（下端中央・物理 px）。
    pub(super) fn ground_point(&self, scope: usize) -> (i32, i32) {
        s2_ground_point(&self.world, self.char_window(scope))
    }
}

impl Drop for FrameHarness {
    /// 次に走るのがハーネスでない場合に備えて、末尾でも閉じる（要件 7.7）。
    ///
    /// `new` 側の後始末だけだと、ハーネスを使ったテストの次にハーネスを使わないテストが
    /// 来たとき、キューと観測の写しが汚れたまま渡る。この向きは areka 側に現在読み手が
    /// 居ないため**どのテストでも赤にできない**——検査できない残留を放置しないために、
    /// 構築と破棄の両側で閉じる。`Drop` はスコープ末尾で走るので、ハーネスを生かしたまま
    /// 次を組む形（残留の検査そのもの）は壊れない。
    fn drop(&mut self) {
        let _residue = drain_window_pos_commands();
        transition_diag::reset_for_test();
    }
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
