use super::*;
use crate::ecs::Monitor;
use crate::ecs::test_support::capture_under_filter;
use windows::Win32::Foundation::RECT;

/// 実モニタ列挙に依存しない合成 `Monitor`（work area はタスクバー分だけ bounds より小さい）。
fn synthetic_monitor() -> Monitor {
    Monitor {
        handle: 0x1234_5678,
        bounds: RECT {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1200,
        },
        work_area: RECT {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1160,
        },
        dpi: 192,
        is_primary: false,
    }
}

/// 要件 1.1: モニタ列挙行は `handle`（識別子）と `work_area`（作業領域矩形）を含む。
///
/// フィールド名は areka `placement::diag` と**共有語彙**（`handle`・`work_area`）である
/// ことが契約——別名（`hmonitor`・`wa` 等）へ変えると診断手順書の grep 突合が壊れる。
/// フィールドを削れば本檻は赤になる。
#[test]
fn enumerated_monitor_line_carries_handle_and_work_area() {
    let monitor = synthetic_monitor();
    let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
        log_enumerated_monitor(&monitor)
    });

    assert!(
        out.contains("[initialize_layout_root] Creating Monitor entity"),
        "モニタ列挙行が出ていない: {out}"
    );
    assert!(
        out.contains("handle="),
        "モニタ識別子フィールド `handle` が無い（要件 1.1）: {out}"
    );
    assert!(
        out.contains("work_area="),
        "work area フィールド `work_area` が無い（要件 1.1）: {out}"
    );
}

/// 要件 1.1: `work_area` は bounds と**別の実値**として復元できる（l,t,r,b 4 成分）。
///
/// bounds を流用した表示なら赤になる（work area の下端 1160 が現れない）。
#[test]
fn enumerated_monitor_work_area_reconstructs_all_four_edges() {
    let monitor = synthetic_monitor();
    let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
        log_enumerated_monitor(&monitor)
    });

    assert!(
        out.contains("work_area=-1920,0,0,1160"),
        "work area の 4 成分（l,t,r,b）が復元できない: {out}"
    );
    assert!(
        out.contains("handle=305419896"),
        "handle の実値が復元できない: {out}"
    );
    // bounds 側は従来どおり残っている（既存フィールドの非退行）。
    assert!(
        out.contains("bounds_bottom=1200"),
        "既存の bounds フィールドが失われている: {out}"
    );
}

// ======================================================================
// S4（診断レポート §2.7・Req 7.1/7.2/7.3/7.5）
//   「識別子が不変で値だけが変わる」表示構成変更でモニタ表が更新されること
// ======================================================================

use crate::ecs::types::{Point, SizeI};
use crate::ecs::{DPI, WindowPos};

/// 檻へ注入する「新しいモニタ列挙結果」。実 OS 列挙（`enumerate_monitors`）の代役。
#[derive(Resource, Clone)]
struct InjectedMonitors(Vec<Monitor>);

/// [`apply_monitor_snapshot`] の戻り値（再導出を駆動した窓の数）を檻へ持ち出す受け皿。
#[derive(Resource, Default)]
struct RedriveCount(usize);

/// 注入されたモニタ表を [`apply_monitor_snapshot`] へ流す檻専用システム。
///
/// 本番の `detect_display_change_system` と**同一の query 構成**であることが重要——
/// ここだけ別配線にすると檻が本番経路を見ていないことになる。
fn apply_injected_monitors(
    mut commands: Commands,
    layout_root: Query<Entity, With<LayoutRoot>>,
    mut existing_monitors: Query<(Entity, &mut Monitor), With<Monitor>>,
    mut windows: Query<(Entity, &WindowPos, &mut DPI)>,
    mut taffy_res: ResMut<TaffyLayoutResource>,
    injected: Res<InjectedMonitors>,
    mut redriven: ResMut<RedriveCount>,
) {
    let root_entity = layout_root.single().expect("檻の LayoutRoot が単一で存在する");
    redriven.0 = apply_monitor_snapshot(
        &mut commands,
        root_entity,
        &mut existing_monitors,
        &mut windows,
        &mut taffy_res,
        injected.0.clone(),
    );
}

/// 直前の [`run_apply`] が再導出を駆動した窓の数。
fn redrive_count(world: &World) -> usize {
    world.resource::<RedriveCount>().0
}

/// 実機セッション②（診断レポート §2.7）と同型の探針: primary モニタの拡大率を
/// 125%（dpi=120）→ 200%（dpi=192）へ変更した状態。
///
/// - `handle` は不変（拡大率変更でモニタ識別子は変わらない）
/// - `bounds` も不変（物理解像度は変わらない）
/// - `work_area` と `dpi` だけが変わる（タスクバーが物理的に太る）
fn probe_monitor_before() -> Monitor {
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

fn probe_monitor_after() -> Monitor {
    Monitor {
        work_area: RECT {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2064,
        },
        dpi: 192,
        ..probe_monitor_before()
    }
}

/// 探針が**不動点でない**ことを檻自身が検査する（[[2.2 の教訓]]・[[3.2 の教訓]]）。
///
/// 「更新される」を主張する檻は、探針の前後が本当に違う値でなければ空虚になる。
/// さらに「`PartialEq` では等価に見える」ことも同時に固定する——これが S4 の
/// 意味論ギャップそのものであり、探針がその穴を確かに踏んでいる証拠になる。
fn assert_probe_is_not_a_fixed_point(before: &Monitor, after: &Monitor) {
    assert_eq!(
        before.handle, after.handle,
        "探針の前提が壊れている: 識別子は不変でなければならない"
    );
    assert_ne!(
        before.work_area.bottom, after.work_area.bottom,
        "探針が不動点: work area が実際に動いていない"
    );
    assert_ne!(before.dpi, after.dpi, "探針が不動点: dpi が実際に動いていない");
    assert_eq!(
        *before, *after,
        "探針が S4 の穴を踏んでいない: PartialEq（同一性）では等価に見えなければならない"
    );
}

/// 檻用 World。LayoutRoot 1 個・`probe_monitor_before()` の Monitor 1 個を持つ。
fn probe_world(injected: Vec<Monitor>) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());
    world.insert_resource(InjectedMonitors(injected));
    world.insert_resource(RedriveCount::default());
    world.spawn(LayoutRoot);
    let monitor_entity = world.spawn(probe_monitor_before()).id();
    (world, monitor_entity)
}

/// 檻の実行。**シングルスレッド実行器を明示**する——既定の多スレッド実行器では
/// システムが別スレッドで走り、`capture_under_filter`（スレッドローカルの dispatcher
/// 差し替え）が 1 行も捕捉できずログ檻が空虚に緑になる。
fn run_apply(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.set_executor(bevy_ecs::schedule::SingleThreadedExecutor::new());
    schedule.add_systems(apply_injected_monitors);
    schedule.run(world);
}

/// 檻用の窓（枠なしゴースト窓相当）: 中心が primary モニタ上に載る位置・寸。
fn spawn_probe_window(world: &mut World, x: i32, y: i32, dpi: u32) -> Entity {
    world
        .spawn((
            WindowPos {
                position: Some(Point { x, y }),
                size: Some(SizeI {
                    width: 400,
                    height: 600,
                }),
                ..Default::default()
            },
            DPI::from_dpi(dpi as u16, dpi as u16),
        ))
        .id()
}

// ---------------------------------------------------------------- 赤証跡

/// **S4 赤証跡（Req 7.5）**: 識別子が不変で値だけが変わったモニタ構成に対して
/// モニタ表が更新されること。
///
/// 是正未投入（`existing_monitor != new_monitor` を更新判定に使う版）では
/// `PartialEq` が `handle` しか見ないため更新分岐が恒偽になり、**赤**になる。
///
/// 是正（`differs_in_value`）投入後は常時走る回帰檻である（タスク 7.1 でゲート解除済み。
/// 赤の実行出力は `diagnosis-report.md` §3.3 に保存してある）。
#[test]
fn s4_red_monitor_table_updates_when_only_values_change() {
    let before = probe_monitor_before();
    let after = probe_monitor_after();
    assert_probe_is_not_a_fixed_point(&before, &after);

    let (mut world, monitor_entity) = probe_world(vec![after.clone()]);
    run_apply(&mut world);

    let stored = world
        .get::<Monitor>(monitor_entity)
        .expect("Monitor エンティティが生存している")
        .clone();

    // 総数や handle 一致で主張しない——**更新後の実値**を見る。
    assert_eq!(
        stored.work_area.bottom, 2064,
        "work area が起動時の値のまま凍結している（S4）: {stored:?}"
    );
    assert_eq!(
        stored.dpi, 192,
        "dpi が起動時の値のまま凍結している（S4）: {stored:?}"
    );
    // 表そのものが作り直されていない（更新であって差し替えではない）。
    assert_eq!(stored.handle, before.handle, "識別子は不変であること");
    assert_eq!(
        world.query::<&Monitor>().iter(&world).count(),
        1,
        "Monitor エンティティが増減している"
    );
}

/// **S4 赤証跡（Req 7.3/7.5）**: モニタ表が更新されたとき、当該モニタ上の窓の
/// `DPI` が `WM_DPICHANGED` 抜きで再導出されること。
///
/// 是正未投入では上流（モニタ表の更新）が恒偽なので当然ここも駆動されず、**赤**になる。
///
/// 是正（`differs_in_value`）投入後は常時走る回帰檻である（タスク 7.1 でゲート解除済み。
/// 赤の実行出力は `diagnosis-report.md` §3.3 に保存してある）。
#[test]
fn s4_red_window_dpi_redriven_without_wm_dpichanged() {
    assert_probe_is_not_a_fixed_point(&probe_monitor_before(), &probe_monitor_after());

    let (mut world, _monitor_entity) = probe_world(vec![probe_monitor_after()]);
    // 中心 (1200, 800) は primary モニタ上。旧 DPI = 120。
    let window = spawn_probe_window(&mut world, 1000, 500, 120);

    run_apply(&mut world);

    let dpi = *world.get::<DPI>(window).expect("窓の DPI が生存している");
    assert_eq!(
        dpi,
        DPI::from_dpi(192, 192),
        "モニタ表が更新されても窓 DPI が再導出されない（S4・Req 7.3）: {dpi:?}"
    );
}

// ------------------------------------------------------ 常時走る随伴檻

/// Req 7.1/7.2 + Req 1.1: 更新が実際に起き、**何が変わったか**がログから読める。
///
/// 更新後の実値（`work_area` 下端・`dpi`）を component とログの両方で固定する。
/// 述語を恒偽に変異させれば component 側が、ログのフィールドを削れば出力側が赤になる。
#[test]
fn value_only_change_updates_monitor_and_reports_old_and_new() {
    assert_probe_is_not_a_fixed_point(&probe_monitor_before(), &probe_monitor_after());

    let (mut world, monitor_entity) = probe_world(vec![probe_monitor_after()]);
    let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
        run_apply(&mut world)
    });

    let stored = world
        .get::<Monitor>(monitor_entity)
        .expect("Monitor エンティティが生存している")
        .clone();
    assert_eq!(stored.work_area.bottom, 2064, "work area が更新されていない");
    assert_eq!(stored.dpi, 192, "dpi が更新されていない");

    assert!(
        out.contains("[detect_display_change_system] Updating Monitor entity"),
        "更新のログが出ていない: {out}"
    );
    assert!(
        out.contains("old_dpi=120") && out.contains("new_dpi=192"),
        "何が変わったか（新旧 dpi）がログから読めない: {out}"
    );
    assert!(
        out.contains("old_work_area=0,0,3840,2100") && out.contains("new_work_area=0,0,3840,2064"),
        "何が変わったか（新旧 work area）がログから読めない: {out}"
    );
}

/// Req 7.3: 更新されたモニタ上の窓は `WM_DPICHANGED` 抜きで DPI が揃う。
///
/// 駆動を消せば赤になる（`redrive_window_dpi_for_updated_monitors` の呼出削除・
/// `updated_monitors` への push 削除のいずれでも）。
#[test]
fn updated_monitor_redrives_window_dpi_and_reports_it() {
    let (mut world, _) = probe_world(vec![probe_monitor_after()]);
    let window = spawn_probe_window(&mut world, 1000, 500, 120);

    let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
        run_apply(&mut world)
    });

    assert_eq!(
        *world.get::<DPI>(window).expect("窓の DPI"),
        DPI::from_dpi(192, 192),
        "窓 DPI が再導出されていない"
    );
    assert!(
        out.contains("Redriving window DPI from updated Monitor"),
        "再導出の観測点が出ていない: {out}"
    );
    assert!(
        out.contains("old_dpi_x=120") && out.contains("new_dpi_x=192"),
        "再導出の新旧 DPI がログから読めない: {out}"
    );
    // 戻り値（駆動した窓の数）も判定語にする——ログだけに頼らない。
    assert_eq!(redrive_count(&world), 1, "駆動した窓の数が 1 でない");
}

/// 非空虚性の対: **値が同一なら更新しない**（無条件更新へ変異させれば赤）。
///
/// これが無いと「常に更新する」実装でも上の檻が緑になってしまう。
#[test]
fn identical_snapshot_updates_nothing() {
    let (mut world, _) = probe_world(vec![probe_monitor_before()]);
    let window = spawn_probe_window(&mut world, 1000, 500, 120);

    let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
        run_apply(&mut world)
    });

    assert!(
        !out.contains("[detect_display_change_system] Updating Monitor entity"),
        "値が同一なのに更新が走っている: {out}"
    );
    assert!(
        !out.contains("Redriving window DPI"),
        "値が同一なのに窓 DPI の再導出が走っている: {out}"
    );
    assert_eq!(
        *world.get::<DPI>(window).expect("窓の DPI"),
        DPI::from_dpi(120, 120),
        "窓 DPI が不用意に書き換わっている"
    );
    assert_eq!(
        redrive_count(&world),
        0,
        "値が同一なのに再導出が駆動されている"
    );
}

/// 再導出は**更新されたモニタ上の窓だけ**が対象（他モニタの窓は触らない）。
#[test]
fn window_outside_updated_monitor_is_not_redriven() {
    let (mut world, _) = probe_world(vec![probe_monitor_after()]);
    // 中心 (-1680, 800) は探針モニタ（bounds 0,0,3840,2160）の外。
    let outside = spawn_probe_window(&mut world, -1880, 500, 96);
    let inside = spawn_probe_window(&mut world, 1000, 500, 120);

    run_apply(&mut world);

    assert_eq!(
        *world.get::<DPI>(outside).expect("窓の DPI"),
        DPI::from_dpi(96, 96),
        "更新モニタ外の窓が書き換わっている"
    );
    assert_eq!(
        *world.get::<DPI>(inside).expect("窓の DPI"),
        DPI::from_dpi(192, 192),
        "更新モニタ内の窓が書き換わっていない（対照が効いていない証拠）"
    );
}

/// **`CW_USEDEFAULT` の窓が本番経路に実在する**（`WindowPos::default()` ＋ `DPI` は
/// `Window` の `on_window_add` フックが揃えて挿入する）。素通しすると
/// `position.x + size.width / 2` が桁溢れし、dev ビルドでは panic で UI スレッドが死ぬ。
///
/// **本檻は「桁溢れしない」ではなく「打ち切られる」を主張する**——桁溢れするコードは
/// dev では panic して赤、release では wrap した中心が偶然どのモニタにも入らず
/// 「DPI は書き換わらないが打ち切りログも出ない」形で赤になる（どちらのプロファイルでも
/// 検出される）。
#[test]
fn window_with_cw_usedefault_is_skipped_before_overflow() {
    let (mut world, _) = probe_world(vec![probe_monitor_after()]);
    // 本番の on_window_add フックが挿入するのと同じ状態（WindowPos::default() ＋ DPI）。
    let window = world.spawn((WindowPos::default(), DPI::default())).id();
    // 探針の前提: 既定値が確かにセンチネルであること（既定値が変わったら檻ごと見直す）。
    let default_pos = WindowPos::default();
    assert_eq!(
        default_pos.position.expect("既定の position").x,
        CW_USEDEFAULT,
        "WindowPos::default() が CW_USEDEFAULT でない＝本檻の前提が崩れている"
    );

    let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
        run_apply(&mut world)
    });

    assert_eq!(
        *world.get::<DPI>(window).expect("窓の DPI"),
        DPI::default(),
        "座標未確定（CW_USEDEFAULT）の窓が書き換わっている"
    );
    assert_eq!(
        redrive_count(&world),
        0,
        "座標未確定の窓を駆動対象に数えている"
    );
    assert!(
        out.contains("Window position/size undetermined, DPI redrive skipped"),
        "CW_USEDEFAULT が「未確定」として打ち切られていない: {out}"
    );
}

/// 位置・寸が未確定の窓は帰属判定できないため打ち切る（正常系・debug 水準）。
#[test]
fn window_without_position_is_skipped_at_debug_level() {
    let (mut world, _) = probe_world(vec![probe_monitor_after()]);
    let window = world
        .spawn((
            WindowPos {
                position: None,
                size: None,
                ..Default::default()
            },
            DPI::from_dpi(120, 120),
        ))
        .id();

    let out = capture_under_filter("info,wintf::ecs::layout=debug", || {
        run_apply(&mut world)
    });

    assert_eq!(
        *world.get::<DPI>(window).expect("窓の DPI"),
        DPI::from_dpi(120, 120),
        "帰属不明の窓が書き換わっている"
    );
    assert!(
        out.contains("Window position/size undetermined, DPI redrive skipped"),
        "打ち切りが観測できない: {out}"
    );
}

#[test]
fn window_center_requires_both_position_and_size() {
    let full = WindowPos {
        position: Some(Point { x: 100, y: 200 }),
        size: Some(SizeI {
            width: 400,
            height: 600,
        }),
        ..Default::default()
    };
    assert_eq!(window_center(&full), Some((300, 500)));

    let no_size = WindowPos {
        size: None,
        ..full
    };
    assert_eq!(window_center(&no_size), None);

    let no_pos = WindowPos {
        position: None,
        ..full
    };
    assert_eq!(window_center(&no_pos), None);
}

/// 「未確定」は `None` だけではない——wintf の正典センチネル `CW_USEDEFAULT` も未確定。
///
/// `CW_USEDEFAULT == i32::MIN` ゆえ素通しは整数桁溢れになる。判定語は同 crate の
/// 既存 3 箇所（`apply_window_pos_changes`／`sync_window_arrangement_from_window_pos`／
/// `WindowPos::to_window_rect`）と同一。
#[test]
fn window_center_treats_cw_usedefault_as_undetermined() {
    // 既定値そのもの（位置・寸ともセンチネル）。
    assert_eq!(window_center(&WindowPos::default()), None);

    // 位置だけセンチネル。
    let pos_sentinel = WindowPos {
        position: Some(Point {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
        }),
        size: Some(SizeI {
            width: 400,
            height: 600,
        }),
        ..Default::default()
    };
    assert_eq!(window_center(&pos_sentinel), None);

    // 寸だけセンチネル。
    let size_sentinel = WindowPos {
        position: Some(Point { x: 100, y: 200 }),
        size: Some(SizeI {
            width: CW_USEDEFAULT,
            height: CW_USEDEFAULT,
        }),
        ..Default::default()
    };
    assert_eq!(window_center(&size_sentinel), None);

    // 探針が退化していないこと: センチネルは実際に i32::MIN であり、
    // 素通しすれば加算が桁溢れする値である。
    assert_eq!(CW_USEDEFAULT, i32::MIN, "センチネルの実値が変わっている");
}

#[test]
fn monitor_containing_uses_half_open_bounds() {
    let m = probe_monitor_before();
    let monitors = [m.clone()];

    assert!(monitor_containing(&monitors, (0, 0)).is_some(), "左上端は含む");
    assert!(
        monitor_containing(&monitors, (3839, 2159)).is_some(),
        "右下端の 1px 内側は含む"
    );
    assert!(
        monitor_containing(&monitors, (3840, 1000)).is_none(),
        "右端は含まない（半開区間）"
    );
    assert!(
        monitor_containing(&monitors, (1000, 2160)).is_none(),
        "下端は含まない（半開区間）"
    );
    assert!(
        monitor_containing(&monitors, (-1, 1000)).is_none(),
        "左外は含まない"
    );
}
