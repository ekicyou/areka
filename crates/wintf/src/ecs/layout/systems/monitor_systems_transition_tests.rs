//! モニタ表の値変化更新（`monitor_systems.rs`）の**遷移観測**に対する決定論的テスト。
//!
//! 固定するのは 3 つである。
//!
//! - **値変化更新の記録**——識別子が不変で値だけが変わったモニタに対して `monitor`
//!   レコードが 1 行出て、新旧の DPI と作業領域が復元できること。既存の `debug` 行・
//!   窓の拡大率再導出（`redrive`）はそのまま残ること。
//! - **無変化では記録しない**——値が動いていないモニタでは 1 行も出ないこと。
//! - **刻印の出所（design D1）**——`detect_display_change_system` は `Update` に載り、
//!   `Update` は**既定の多スレッド実行器**で回る。ワーカースレッドからはスレッド局所
//!   ミラーが見えないため、刻印は World 資源（`FrameCount`＋`TickStart`）から組まねば
//!   ならない。
//!
//! # 多スレッド実行の相ではログ捕捉を根拠にしない（要件 7.6）
//!
//! `capture_under_filter` は `tracing::subscriber::with_default`＝**スレッド局所**の
//! 差替であり、bevy の既定実行器がシステムをワーカースレッドへ載せると 1 行も届かない。
//! そこで多スレッドの相の検証は**ログではなくデータ**で行う——レコード純関数へ渡す
//! `Stamp` そのものを共有資源へ書き出し、`FrameCount` と突き合わせる
//! （雛形＝`transition_diag_tests::world_stamps_match_frame_count_under_the_default_multi_threaded_executor`）。
//! 逆に、行の字面を見る 3 本は**シングルスレッド実行器を明示**して捕捉を成立させる。
//!
//! 「出ない」を見るテストには、同じ捕捉窓の内側に必ず拾えるはずの**対照行**を置く。

use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

use bevy_ecs::schedule::{ExecutorKind, Schedules};
use windows::Win32::Foundation::RECT;

use super::*;
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::transition_diag::{KIND_MONITOR, RECORD_PREFIX_TAG, TRANSITION_TARGET};
use crate::ecs::world::{EcsWorld, Update};
use crate::ecs::{DPI, Monitor, WindowPos};

/// 実機サインオフが用いる directive のうち、本チャネルを点灯させるもの。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::transition=debug";

/// 上に既存のモニタ表更新ログ（`wintf::ecs::layout`）を足したもの。
/// **既存の記録行を変えていない**ことを同じ捕捉窓の中で見るために使う。
const SIGNOFF_WITH_EXISTING_DIRECTIVES: &str =
    "info,wintf::transition=debug,wintf::ecs::layout=debug";

/// 既定水準（観測チャネルを有効化していない通常運転）。
const DEFAULT_DIRECTIVES: &str = "info";

/// 捕捉が生きている証拠として同じ窓の中で必ず出す対照行。
const CONTROL_LINE: &str = "[transition-probe] alive";

/// 逐語比較に使う固定の刻印（純関数は時刻を読まないので任意の値で組める）。
const PROBE_STAMP: Stamp = Stamp {
    frame: 7,
    t_us: 1234,
};

fn emit_control() {
    tracing::info!(target: TRANSITION_TARGET, "{CONTROL_LINE}");
}

fn transition_lines(captured: &str) -> Vec<&str> {
    captured
        .lines()
        .filter(|line| line.contains(RECORD_PREFIX_TAG))
        .collect()
}

fn lines_of_kind<'a>(captured: &'a str, kind: &str) -> Vec<&'a str> {
    let needle = format!("kind={kind} ");
    transition_lines(captured)
        .into_iter()
        .filter(|line| line.contains(&needle))
        .collect()
}

// ---------------------------------------------------------------------------
// 探針のモニタ（識別子は不変・値だけが変わる＝S4 と同型）
// ---------------------------------------------------------------------------

fn monitor_before() -> Monitor {
    Monitor {
        handle: 0x0000_ABCD,
        bounds: RECT {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2160,
        },
        work_area: RECT {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2100,
        },
        dpi: 120,
        is_primary: true,
    }
}

fn monitor_after() -> Monitor {
    Monitor {
        work_area: RECT {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2064,
        },
        dpi: 192,
        ..monitor_before()
    }
}

// ---------------------------------------------------------------------------
// シングルスレッドの相（行の字面を見る）
// ---------------------------------------------------------------------------

/// 檻へ注入する「新しいモニタ列挙結果」。
#[derive(Resource, Clone)]
struct InjectedMonitorTable(Vec<Monitor>);

/// 檻が渡す刻印（本番は `Res<FrameCount>`＋`Res<TickStart>` から組む）。
#[derive(Resource, Clone, Copy)]
struct ProbeStamp(Stamp);

/// [`apply_monitor_snapshot`] の戻り値（再導出を駆動した窓の数）の受け皿。
#[derive(Resource, Default)]
struct RedriveCount(usize);

/// 注入されたモニタ表を [`apply_monitor_snapshot`] へ流す檻専用システム。
///
/// 本番の `detect_display_change_system` と**同一の query 構成**であることが重要
/// ——ここだけ別配線にすると檻が本番経路を見ていないことになる。
#[allow(clippy::too_many_arguments)]
fn apply_injected_with_stamp(
    mut commands: Commands,
    layout_root: Query<Entity, With<LayoutRoot>>,
    mut existing_monitors: Query<(Entity, &mut Monitor), With<Monitor>>,
    mut windows: Query<(Entity, &WindowPos, &mut DPI)>,
    mut taffy_res: ResMut<TaffyLayoutResource>,
    injected: Res<InjectedMonitorTable>,
    stamp: Res<ProbeStamp>,
    mut redriven: ResMut<RedriveCount>,
) {
    let root_entity = layout_root
        .single()
        .expect("檻の LayoutRoot が単一で存在する");
    redriven.0 = apply_monitor_snapshot(
        &mut commands,
        root_entity,
        &mut existing_monitors,
        &mut windows,
        &mut taffy_res,
        injected.0.clone(),
        stamp.0,
    );
}

/// 檻用 World。LayoutRoot 1 個・`monitor_before()` の Monitor 1 個を持つ。
fn probe_world(injected: Vec<Monitor>) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());
    world.insert_resource(InjectedMonitorTable(injected));
    world.insert_resource(ProbeStamp(PROBE_STAMP));
    world.insert_resource(RedriveCount::default());
    world.spawn(LayoutRoot);
    let monitor_entity = world.spawn(monitor_before()).id();
    (world, monitor_entity)
}

/// 檻の実行。**シングルスレッド実行器を明示**する——既定の多スレッド実行器では
/// システムが別スレッドで走り、`capture_under_filter`（スレッドローカルの dispatcher
/// 差し替え）が 1 行も捕捉できずログ檻が空虚に緑になる（要件 7.6）。
fn run_apply(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(apply_injected_with_stamp);
    schedule.run(world);
}

#[test]
fn a_value_change_update_is_recorded_with_the_supplied_stamp() {
    let (mut world, monitor_entity) = probe_world(vec![monitor_after()]);

    let captured = capture_under_filter(SIGNOFF_WITH_EXISTING_DIRECTIVES, || {
        emit_control();
        run_apply(&mut world);
    });

    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );

    let records = lines_of_kind(&captured, KIND_MONITOR);
    assert_eq!(records.len(), 1, "値変化 1 件につき 1 行: {captured}");
    let line = records[0];

    // 刻印は呼出側が渡した値そのもの（本関数は時刻もフレーム番号も自分では読まない）。
    assert!(
        line.contains("[transition] frame=7 t_us=1234 kind=monitor"),
        "接頭語の刻印が渡した値と違う: {line}"
    );
    assert!(
        line.contains(&format!("entity={monitor_entity:?}")),
        "更新したモニタの entity が載る（areka 側レコードとの結合キー）: {line}"
    );
    assert!(
        line.contains("old_dpi=120 new_dpi=192"),
        "新旧の DPI が同一行から復元できない: {line}"
    );
    assert!(
        line.contains("old_wa=0,0,3840,2100 new_wa=0,0,3840,2064"),
        "新旧の作業領域が同一行から復元できない: {line}"
    );

    // 既存の挙動は不変——`debug` 行も表の更新もそのまま。
    assert!(
        captured.contains("[detect_display_change_system] Updating Monitor entity"),
        "既存の更新ログが失われている: {captured}"
    );
    let stored = world
        .get::<Monitor>(monitor_entity)
        .expect("Monitor エンティティが生存している")
        .clone();
    assert_eq!(
        (stored.dpi, stored.work_area.bottom),
        (192, 2064),
        "モニタ表の更新そのものが壊れている: {stored:?}"
    );
}

#[test]
fn an_unchanged_table_records_nothing() {
    // 同じ値を注入する（`differs_in_value` が偽＝更新分岐へ入らない）。
    let (mut world, _monitor_entity) = probe_world(vec![monitor_before()]);

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        run_apply(&mut world);
    });

    assert!(captured.contains(CONTROL_LINE), "捕捉が死んでいる");
    assert!(
        lines_of_kind(&captured, KIND_MONITOR).is_empty(),
        "値が動いていないのに記録が出ている（定常フレームで行が増える）: {captured}"
    );
}

#[test]
fn the_monitor_record_is_silent_under_the_default_directives() {
    let (mut world, _monitor_entity) = probe_world(vec![monitor_after()]);

    let captured = capture_under_filter(DEFAULT_DIRECTIVES, || {
        emit_control();
        run_apply(&mut world);
    });

    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );
    assert!(
        transition_lines(&captured).is_empty(),
        "観測 target を点けない運転では観測行は 1 行も出ない: {captured}"
    );
}

// ---------------------------------------------------------------------------
// 多スレッド実行器のままの相（データで見る・要件 7.6）
// ---------------------------------------------------------------------------

/// 観測点が 1 回の実行で使った刻印。
#[derive(Clone, Copy, Debug)]
struct StampObservation {
    /// World 資源（`FrameCount`＋`TickStart`）から組んだ刻印。
    from_world: Stamp,
    /// スレッド局所ミラーから読んだフレーム番号（World を持つ点は本来これを読まない）。
    from_mirror: u32,
    /// 記録したスレッド。
    thread: ThreadId,
}

/// 観測の集積先（共有参照＋内部可変＝実行器が直列化しない形）。
#[derive(Resource, Clone, Default)]
struct ObservedMonitorStamps(Arc<Mutex<Vec<StampObservation>>>);

/// **本番と同じ資源から刻印を組み、本番と同じ `apply_monitor_snapshot` を呼ぶ**観測系
/// システム。`Update` へ登録して既定の多スレッド実行器のまま回す。
#[allow(clippy::too_many_arguments)]
fn observe_monitor_stamp(
    mut commands: Commands,
    frame: Res<FrameCount>,
    tick_start: Res<TickStart>,
    layout_root: Query<Entity, With<LayoutRoot>>,
    mut existing_monitors: Query<(Entity, &mut Monitor), With<Monitor>>,
    mut windows: Query<(Entity, &WindowPos, &mut DPI)>,
    mut taffy_res: ResMut<TaffyLayoutResource>,
    injected: Res<InjectedMonitorTable>,
    out: Res<ObservedMonitorStamps>,
) {
    // D1: World を借りられる観測点は資源から組む（`transition_diag::stamp()` は退行）。
    let stamp = transition_diag::stamp_from_world(&frame, &tick_start);
    out.0
        .lock()
        .expect("観測バッファの毒化なし")
        .push(StampObservation {
            from_world: stamp,
            from_mirror: transition_diag::current_frame(),
            thread: std::thread::current().id(),
        });

    let Ok(root_entity) = layout_root.single() else {
        return;
    };
    apply_monitor_snapshot(
        &mut commands,
        root_entity,
        &mut existing_monitors,
        &mut windows,
        &mut taffy_res,
        injected.0.clone(),
        stamp,
    );
}

/// 既定の多スレッド実行器のまま `Update` を回し、モニタ表更新の観測点が使う刻印の
/// フレーム番号が `FrameCount` と一致することを、**ログ捕捉に依らず**確かめる。
///
/// # 本テストが示さないこと
///
/// 走らせるのは探針システムであって `detect_display_change_system` ではない（後者は同 tick で
/// `display_configuration_changed()` が偽ゆえ記録点へ到達しない）。したがって本番の
/// `stamp_from_world` を `stamp()` へ差し替える退行は、**本テストでは一度も赤にならない**
/// （実測 8/8 緑）。その性質は
/// [`the_monitor_record_builds_its_stamp_from_world_resources_only`] が単独で担う。
///
/// 実モニタ台数に依存しないよう、探針のモニタ（handle 0xABCD）を 1 台足し、注入表は
/// 「実在モニタは現状のまま＋探針だけ値を変えたもの」にする——実在モニタは
/// `differs_in_value` が偽で無操作、探針だけが値変化更新の分岐へ入る。
#[test]
fn monitor_stamps_match_frame_count_under_the_default_multi_threaded_executor() {
    let mut ecs = EcsWorld::new();

    // 実在モニタ（`EcsWorld::new` の `initialize_layout_root` が列挙済み）をそのまま写す。
    let mut query = ecs.world_mut().query::<&Monitor>();
    let mut injected: Vec<Monitor> = query.iter(ecs.world()).cloned().collect();

    // 探針を 1 台足し、注入表にはその「変化後」を入れる。
    let probe_entity = ecs.world_mut().spawn(monitor_before()).id();
    injected.push(monitor_after());

    let sink = ObservedMonitorStamps::default();
    ecs.world_mut().insert_resource(sink.clone());
    ecs.world_mut()
        .insert_resource(InjectedMonitorTable(injected));
    ecs.add_systems(Update, observe_monitor_stamp);

    // 検証が空振りしていないこと——`Update` は既定の実行器（多スレッド）のままである。
    let kind = ecs
        .world()
        .resource::<Schedules>()
        .get(Update)
        .expect("Update スケジュールは World 構築時に登録済み")
        .get_executor_kind();
    assert_eq!(
        kind,
        ExecutorKind::MultiThreaded,
        "本テストの前提は既定の多スレッド実行器（単スレッドへ落ちていたら検証にならない）"
    );

    assert!(
        ecs.try_tick_world(),
        "システム登録済みの World は tick する"
    );
    let expected = ecs.world().resource::<FrameCount>().0;
    assert_eq!(expected, 1, "1 周で FrameCount は 1 になる");

    let observations = sink.0.lock().expect("観測バッファの毒化なし").clone();
    assert!(
        !observations.is_empty(),
        "観測系システムが 1 度も走っていない（空虚に緑になっている）"
    );
    for observation in &observations {
        assert_eq!(
            observation.from_world.frame, expected,
            "多スレッド実行でもモニタ表更新点の刻印は FrameCount とずれない: {observation:?}"
        );
    }

    // ワーカースレッドに載った観測があれば、そこでは写しが見えない（frame=0）ことを
    // その場の実測で示す＝`stamp()` へ退行したときに何が起きるかの直接証拠。
    let ui_thread = std::thread::current().id();
    for observation in observations.iter().filter(|o| o.thread != ui_thread) {
        assert_eq!(
            observation.from_mirror, 0,
            "ワーカースレッドからは写しが読めない（World を持つ点が stamp() を呼ぶ退行の検出）: {observation:?}"
        );
    }

    // 探針が値変化更新の分岐へ実際に入っていること（入っていなければ上は空虚）。
    let stored = ecs
        .world()
        .get::<Monitor>(probe_entity)
        .expect("探針のモニタが生存している")
        .clone();
    assert_eq!(
        (stored.dpi, stored.work_area.bottom),
        (192, 2064),
        "探針が値変化更新の分岐（＝記録点）を通っていない: {stored:?}"
    );

    transition_diag::reset_for_test();
}

// ---------------------------------------------------------------------------
// 刻印の出所を構造で固定する（D1）
// ---------------------------------------------------------------------------

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// **本テストは補助ではなく、この性質の唯一の担い手である。**
///
/// 上の多スレッドテストは `stamp()` への退行を**原理的に一度も捕まえられない**——
/// 実測でも変異を入れて 8/8 緑のままだった。理由は載るスレッドの運ではなく配線である:
/// 変異が入る行は `detect_display_change_system` の中にあるが、多スレッドテストが走らせる
/// のは探針システム `observe_monitor_stamp`（自分で刻印を組み `apply_monitor_snapshot` を
/// 直に呼ぶ）であり、本番システムのほうは同 tick で
/// `display_configuration_changed()` が偽ゆえ早期 return して記録点に到達しない。
///
/// つまり多スレッドテストが示すのは「モニタ表更新点の**形**で組んだ刻印はワーカースレッド
/// でも `FrameCount` と一致する」までであり、「本番の記録点が**実際にその組み方をしている**」
/// を示すのは本テストだけである。両者は役割が違い、どちらも落とせない。
#[test]
fn the_monitor_record_builds_its_stamp_from_world_resources_only() {
    let code = code_only(include_str!("monitor_systems.rs"));

    assert!(
        code.contains("transition_diag::stamp_from_world(&frame, &tick_start)"),
        "モニタ表更新点の刻印が World 資源から組まれていない（D1 違反＝ワーカースレッドで frame=0 になる）"
    );
    assert_eq!(
        code.matches("transition_diag::stamp()").count(),
        0,
        "World を借りられる観測点がスレッド局所ミラーを読んでいる（D1 違反）"
    );
    assert_eq!(
        code.matches("transition_diag::is_enabled()").count(),
        1,
        "記録点の前置ガードが 1 つでない（既定運転で行の組立の費用を払う形になる）"
    );

    // 走査が中身を見ている対照——落とし過ぎていないこと。
    assert!(
        code.contains("pub fn detect_display_change_system("),
        "説明文を落とす処理が本文まで落としている"
    );
}
