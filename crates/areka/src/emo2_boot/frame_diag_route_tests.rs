use crate::placement::resolver::SizePx;
use wintf::ecs::SizeI;
use wintf::ecs::DPI;

use super::*;
use super::test_support::{
    FakeReports,
    FakeSizes,
    dpi_world,
    resnap_world,
    size_of,
    synth_assets,
    window_move_lines,
    window_move_routes_of,
};

// ── task 1.4 是正: frame 側 route 割当の檻（Req 1.2／2.4・design D13）──────────
//
// `reconcile_window_size` は **2 呼出元の共通末端**である（DPI 相＝`dpi_phase_with`／
// drain 相＝`reconcile_reported_sizes`）。ここへ 1 つの route を貼り付けると、
// `Changed<DPI>` 非依存の drain 相（初回表示の k₀ 補正を含む）まで「DPI 由来」と
// 名乗ってしまい、Req 1.9 の 2 段 grep 突合（セッション②＝ドラッグ禁止・OS 側 DPI 変更のみ）
// に**偽陽性**が混じる。ゆえに route は呼出元ごとに別語でなければならない（D13）。
//
// 観測境界は `placement::test_support::capture_logs`（`pub(crate)`＝本モジュールから到達可能）
// による tracing イベント本体で、レコード書式の権威は `placement::diag` の純関数が持つ。

use crate::placement::diag::DESPAWNED_SKIP_TAG;

use crate::placement::test_support::{LogEvent, capture_logs as capture_diag_logs};

/// 毎フレーム再スナップ（`resnap_from_sizes`）の書込は `Resnap` として記録される。
#[test]
fn resnap_from_sizes_records_the_resnap_route() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).expect("char 窓がある");

    // h 687→700 の異寸（既存檻 `resnap_from_sizes_drives_resize_and_resnap_on_size_change`
    // と同一の注入）→ 書込が起きる。
    let (_, events) = capture_diag_logs(|| {
        resnap_from_sizes(
            &mut world,
            [(0usize, SizePx { w: 434, h: 700 })].into_iter(),
        )
    });

    assert_eq!(
        window_move_routes_of(&events, char0),
        vec!["Resnap"],
        "再スナップ経由の書込が Resnap として記録されない: {:?}",
        window_move_lines(&events)
    );
}

/// **D13 の核心**: DPI 相（`Changed<DPI>` 由来）と drain 相（報告回収・エッジ非依存）は
/// **別々の経路名**で記録される。同一の共通末端を通ることは route の同一性を意味しない。
#[test]
fn dpi_phase_and_drain_phase_record_distinct_routes() {
    let (mut world, gw) = dpi_world();
    let char0 = gw.char_window(0).expect("char 窓がある");

    // --- DPI 相: 窓 DPI を 96→192 へ差し替え（Changed<DPI> エッジ）---
    world.entity_mut(char0).insert(DPI::from_dpi(192, 192));
    let mut source = FakeReports::default();
    source.refresh.insert(shell_target(0).0, (868, 1374));
    let mut state = None;
    let (_, dpi_events) =
        capture_diag_logs(|| dpi_phase_with(&mut source, &mut state, &mut world));
    let dpi_routes = window_move_routes_of(&dpi_events, char0);
    assert_eq!(
        dpi_routes,
        vec!["DpiReproject"],
        "DPI 相の書込が DpiReproject として記録されない: {:?}",
        window_move_lines(&dpi_events)
    );

    // --- drain 相: `Changed<DPI>` を一切動かさずに報告だけを積む（＝表示成立由来）---
    source.pending.insert(shell_target(0).0, (900, 1400));
    let (_, drain_events) =
        capture_diag_logs(|| reconcile_reported_sizes(&mut source, &mut world));
    let drain_routes = window_move_routes_of(&drain_events, char0);
    assert_eq!(
        drain_routes,
        vec!["ReportedSizeReconcile"],
        "drain 相の書込が ReportedSizeReconcile として記録されない: {:?}",
        window_move_lines(&drain_events)
    );

    assert_ne!(
        dpi_routes, drain_routes,
        "2 呼出元が同一の経路名で記録されている（D13 が禁じる偽陽性の源）"
    );
}

/// **完了状態（tasks.md 1.4）**: DPI 変化ゼロの起動で `DpiReproject` レコードが 1 行も出ない。
///
/// 初回表示の k₀ 補正は drain 相（`reconcile_reported_sizes`）で landing する
/// （`frame.rs` の `run_dpi_phase` doc「初回表示の k₀ 補正＝Flow 3 手順 5 はこちらの経路で
/// landing する」）。この走行を `DpiReproject` と名乗らせると、セッション②の受理回数突合が
/// 起動ごとに偽陽性を拾う。
#[test]
fn boot_without_any_dpi_change_emits_no_dpi_reproject_record() {
    let (mut world, gw) = dpi_world(); // DPI は 96 のまま一切触らない
    let char0 = gw.char_window(0).expect("char 窓がある");
    let mut source = FakeReports::default();
    // 初回表示の k₀ 補正相当（refresh 側は空＝再表示ゲート不成立）。
    source.pending.insert(shell_target(0).0, (500, 720));
    let mut state = None;

    let (_, events) = capture_diag_logs(|| {
        dpi_phase_with(&mut source, &mut state, &mut world);
        reconcile_reported_sizes(&mut source, &mut world);
    });

    let lines = window_move_lines(&events);
    assert!(
        !lines.is_empty(),
        "非空虚性: 起動時 k₀ 補正の書込自体は起きている（レコード 0 行では檻が空虚）"
    );
    assert!(
        lines.iter().all(|l| !l.contains("route=DpiReproject")),
        "DPI 変化ゼロの走行で DpiReproject レコードが出ている（Req 1.2 違反・D13）: {lines:?}"
    );
    assert_eq!(
        window_move_routes_of(&events, char0),
        vec!["ReportedSizeReconcile"],
        "k₀ 補正は報告回収経路として記録されるべき: {lines:?}"
    );
}

// ── task 3.2: フレーム層 消費側の存在確認（Req 6.2/6.3・design D8 消費側）────────
//
// 終了処理でゴースト窓が despawn されると `GhostWindowMarker` の `on_remove` hook
// （task 3.1）が `GhostWindows` Resource から scope エントリを落とす。だが消費側は
// **Resource の写しを持って回る**（`reconcile_reported_sizes` は冒頭で `.cloned()` する）
// ため、「レジストリの参照先の窓が既に存在しない」状態（Req 6.3 の If 節そのもの）は
// 構造上あり得る。以下の檻はその陳腐化レジストリを**明示的に組んで**、
//   (1) 破棄済み scope は warn 以上を 1 行も出さずに打ち切られること（Req 6.2）
//   (2) 打ち切りが**他 scope の処理を止めない**こと（Req 6.3）
// を固定する。掃除後の綺麗なレジストリで回しても両者は自明に成立してしまう（＝空虚な檻）
// ので、探針は必ず「破棄済み entity を指すレジストリ」でなければならない
// （tasks.md Implementation Notes 2.2 の空虚性の教訓と同型）。

/// 破棄済み entity を指したままの**陳腐化レジストリ**を作る（Req 6.3 の状態を再現）。
///
/// `spawn_ghost_windows` の戻り値は Resource とは別の写しゆえ、hook による掃除が
/// 済んだ後に写しを挿し直せば「登録はあるが指す先が消えている」状態になる。
/// 掃除が実際に効いていること（前提の非空虚性）も併せて主張する。
fn despawn_scope_and_restore_stale_registry(world: &mut World, gw: &GhostWindows, scope: usize) {
    let char_window = gw.char_window(scope).expect("char 窓がある");
    let balloon_window = gw.balloon_window(scope).expect("balloon 窓がある");
    world.despawn(char_window);
    world.despawn(balloon_window);
    assert!(
        world
            .get_resource::<GhostWindows>()
            .expect("Resource は残る")
            .char_window(scope)
            .is_none(),
        "前提: despawn hook（task 3.1）が scope {scope} をレジストリから落としている"
    );
    // 陳腐化した写しを挿し直す＝消費側の存在確認だけが防波堤になる状態。
    world.insert_resource(gw.clone());
    assert_eq!(
        world
            .get_resource::<GhostWindows>()
            .expect("Resource がある")
            .char_window(scope),
        Some(char_window),
        "前提: レジストリが破棄済み entity を指している（探針が不動点でない）"
    );
}

/// warn 以上のイベントだけを抜く（`tracing::Level` の Ord は ERROR < WARN < INFO < …）。
fn warn_or_above(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .collect()
}

/// 破棄済み打ち切りの debug 行（本文に判定語を含むもの）を抜く。
fn despawn_skip_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(DESPAWNED_SKIP_TAG))
        .collect()
}

/// Req 6.2/6.3（再スナップ相）: 破棄済み scope は debug で打ち切られ、**生存 scope は
/// 処理し切る**。打ち切りは表示側への問い合わせより手前で起きる（＝問い合わせ記録に
/// 破棄済み scope の target が現れない）。
#[test]
fn resnap_skips_despawned_scope_at_debug_and_processes_surviving_scopes() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).expect("char 窓がある");
    despawn_scope_and_restore_stale_registry(&mut world, &gw, 0);

    // shell=434×700（scope1 の初期寸 278×357 と異なる＝生存 scope は駆動される）。
    let fake = FakeSizes::new((434, 700), (223, 158));
    let (_, events) = capture_diag_logs(|| resnap_with(&fake, &mut world));

    assert!(
        warn_or_above(&events).is_empty(),
        "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {:?}",
        warn_or_above(&events)
    );
    assert_eq!(
        fake.queried.borrow().clone(),
        vec![shell_target(1).0],
        "破棄済み scope の target は引かず、生存 scope だけを引く（フレーム層で打ち切っている証跡）"
    );
    assert_eq!(
        size_of(&world, char1),
        Some(SizeI::new(434, 700)),
        "生存 scope は最後まで処理される（Req 6.3「他の scope の処理を継続」）"
    );
    let skips = despawn_skip_lines(&events);
    assert_eq!(skips.len(), 1, "破棄済み scope の打ち切りは 1 行: {events:?}");
    assert_eq!(skips[0].level, tracing::Level::DEBUG);
    assert!(
        skips[0].message().contains("resnap:"),
        "再スナップ相が自分の相を名乗っていない: {:?}",
        skips[0].message()
    );
}

/// Req 6.2/6.3（報告回収相）: 破棄済み scope の char／balloon 両 target は debug で
/// 打ち切られ、生存 scope の報告は反映される。報告は**取り出したうえで**捨てる
/// （窓が無い以上、次フレームへ持ち越しても反映先が無い＝既存契約の維持）。
#[test]
fn drain_reconcile_skips_despawned_scope_at_debug_and_processes_surviving_scopes() {
    let (mut world, gw) = dpi_world();
    let char1 = gw.char_window(1).expect("char 窓がある");
    despawn_scope_and_restore_stale_registry(&mut world, &gw, 0);

    let mut source = FakeReports::default();
    source.pending.insert(shell_target(0).0, (868, 1374)); // 破棄済み scope
    source.pending.insert(balloon_target(0).0, (279, 198)); // 破棄済み scope
    source.pending.insert(shell_target(1).0, (556, 714)); // 生存 scope

    let (_, events) = capture_diag_logs(|| reconcile_reported_sizes(&mut source, &mut world));

    assert!(
        warn_or_above(&events).is_empty(),
        "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {:?}",
        warn_or_above(&events)
    );
    assert!(
        source.calls_of("take").contains(&shell_target(0).0),
        "非空虚性: 破棄済み scope でも報告の取り出し自体は行われている（持ち越さない）"
    );
    assert_eq!(
        size_of(&world, char1),
        Some(SizeI::new(556, 714)),
        "生存 scope は最後まで処理される（Req 6.3「他の scope の処理を継続」）"
    );
    let skips = despawn_skip_lines(&events);
    assert_eq!(
        skips.len(),
        2,
        "破棄済み scope の char／balloon 両 target が打ち切られる: {events:?}"
    );
    assert!(
        skips
            .iter()
            .all(|e| e.level == tracing::Level::DEBUG && e.message().contains("dpi reconcile:")),
        "報告回収相の打ち切りが debug かつ自分の相を名乗っていない: {skips:?}"
    );
}

/// **完了状態（tasks.md 3.2）**: 終了処理でゴースト窓が破棄された後のフレームで、
/// 破棄済み窓に対する**警告以上のログが 1 行も出ない**（DPI 相・報告回収相・再スナップ相の
/// 3 相通し）。窓への書込も 1 件も起きない。
#[test]
fn frame_after_teardown_despawn_emits_no_warning_for_destroyed_windows() {
    let (mut world, gw) = dpi_world();
    despawn_scope_and_restore_stale_registry(&mut world, &gw, 0);
    despawn_scope_and_restore_stale_registry(&mut world, &gw, 1);

    // 終了処理の直前まで積まれていた報告が残っている状況（表示側は窓の破棄を知らない）。
    let mut source = FakeReports::default();
    for scope in [0u32, 1] {
        source.pending.insert(shell_target(scope).0, (868, 1374));
        source.pending.insert(balloon_target(scope).0, (279, 198));
    }
    let fake = FakeSizes::new((434, 700), (223, 158));
    let mut state = None;

    let (_, events) = capture_diag_logs(|| {
        dpi_phase_with(&mut source, &mut state, &mut world);
        reconcile_reported_sizes(&mut source, &mut world);
        resnap_with(&fake, &mut world);
    });

    assert!(
        warn_or_above(&events).is_empty(),
        "破棄済み窓に対する警告以上のログが残っている（完了状態違反）: {:?}",
        warn_or_above(&events)
    );
    // 非空虚性: 消費側が実際に破棄済み scope を踏んでいる（何も起きなかったのではない）。
    // **相ごとに数える**——総数だけを見ると、フレーム層の打ち切りを外しても下流の追従層が
    // 同じ判定語で同数の debug を出すため、総数が偶然一致して檻が空虚になる。
    let skips = despawn_skip_lines(&events);
    assert_eq!(
        skips
            .iter()
            .filter(|e| e.message().contains("dpi reconcile:"))
            .count(),
        4,
        "報告回収相は 2 scope × char/balloon の 4 件を自分の相で打ち切る: {events:?}"
    );
    assert_eq!(
        skips
            .iter()
            .filter(|e| e.message().contains("resnap:"))
            .count(),
        2,
        "再スナップ相は 2 scope を自分の相で打ち切る: {events:?}"
    );
    assert_eq!(
        skips.len(),
        6,
        "打ち切りは上記 2 相の 6 件だけ（下流の追従層まで降りていない）: {events:?}"
    );
    assert!(
        window_move_lines(&events).is_empty(),
        "破棄済み窓へ書込が発生している: {:?}",
        window_move_lines(&events)
    );
}

/// author_dpi の引き当て（取り違え防止）: shell target には shell 宣言・balloon target には
/// balloon 宣言が渡る。両者 `u16` ゆえ取り違えてもコンパイルは通る——**異なる値**で引き当てを
/// 反証する（入れ替えれば必ず落ちる）。未知 target は既定 96 へ縮退する（panic しない）。
#[test]
fn author_dpis_pairs_shell_and_balloon_declarations() {
    let assets = synth_assets(&[(0, 0)]);
    let plan = plan_attachments(&[0usize], &assets);
    let item = &plan.items[0];
    // shell=120（125% 原稿）・balloon=72（意図的に異なる値・入れ替え検出用）。
    let dpis = AuthorDpis {
        shell: 120,
        balloon: 72,
    };

    assert_eq!(
        dpis.for_target(item, item.shell_target),
        120,
        "shell target には shell_author_dpi が渡る"
    );
    assert_eq!(
        dpis.for_target(item, item.balloon_target),
        72,
        "balloon target には balloon_author_dpi が渡る"
    );
    assert_eq!(
        dpis.for_target(item, TargetId(9999)),
        96,
        "当該 scope のいずれの target でもない＝結線バグ → 既定 96 へ縮退（panic しない）"
    );
}
