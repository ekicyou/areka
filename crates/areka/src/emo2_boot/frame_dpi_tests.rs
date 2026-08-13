use wintf::ecs::{Point, SizeI};

use super::*;
use super::test_support::{
    FakeReports,
    assert_no_write,
    dpi_world,
    pos_of,
    reset_write_witness,
    size_of,
};

// ── task 4.2: DPI 追従フェーズ（run_dpi_phase／窓寸 reconcile 二経路）の檻 ──────────
//
// 判断分岐（窓種別の判定・物理寸の算出・反映口の振り分け・エッジ観測の永続性・二経路の
// 責任分界）を GPU 不要で決定論に固定する（design「Testing Strategy」振り分け基準 (a)・D9）。
// GPU readback 檻（実 k 倍表示の寸法・バイト）は emo-present in-crate＝別プロセス側の領分
// （R5.1/R5.3）ゆえここでは組まない——本ファイルへ 2 個目の Compositor を持ち込まない。
//
// 「書込ゼロ」の観測境界は follow.rs task 2.2 の檻と同一手法を用いる: `SetWindowPosCommand`
// の TLS キューは wintf 私有で件数を覗く API が無く `flush()` は偽 HWND へ実 Win32 を撃つため
// 使えない。代わりに **`Arrangement.offset` 同期**（`enqueue_window_set_pos` 内で enqueue と
// 不可分に対で走る）を witness とし、sentinel が据え置かれたまま＝単一ライター経路を一度も
// 通っていない＝窓書込 0 件の決定論的証拠とする。

use areka_emo_compose::ScaleRatio;

use wintf::ecs::DPI;

/// 窓種別の判定（`spawn.rs` の marker から・純関数）: char のみ／balloon のみ／どちらでもない／
/// 両方同居（結線バグ）の 4 分岐を全網羅する。
#[test]
fn classify_ghost_window_covers_all_marker_combinations() {
    assert_eq!(
        classify_ghost_window(Some(3), None),
        GhostWindowClass::Ghost(3, GhostWindowKind::Char),
        "CharWindowMarker のみ → キャラ窓（scope 保持）"
    );
    assert_eq!(
        classify_ghost_window(None, Some(7)),
        GhostWindowClass::Ghost(7, GhostWindowKind::Balloon),
        "BalloonWindowMarker のみ → バルーン窓（scope 保持）"
    );
    assert_eq!(
        classify_ghost_window(None, None),
        GhostWindowClass::NotGhost,
        "どちらの marker も無い窓は DPI 相の対象外"
    );
    assert_eq!(
        classify_ghost_window(Some(0), Some(0)),
        GhostWindowClass::Ambiguous,
        "両 marker 同居は spawn の排他付与に反する結線バグ（縮退させる）"
    );
}

/// 反映口の振り分け（D8）: **char 窓は `resize_window_to`**（アンカー保存＝Bottom 再射影で
/// 位置が動く）・**balloon 窓は `resize_window_keep_position`**（位置維持）。観測可能な差
/// （position が動く／動かない）で振り分けを反証する。
#[test]
fn reconcile_window_size_routes_char_to_anchor_resize_and_balloon_to_keep_position() {
    let (mut world, gw) = dpi_world();
    let char0 = gw.char_window(0).expect("char 窓がある");
    let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");

    // --- balloon を先に見る（char の resize は BalloonFollow で balloon を動かすため）---
    assert_eq!(
        pos_of(&world, balloon0),
        Some(Point { x: 1071, y: 732 }),
        "前提: balloon 初期位置"
    );
    assert!(
        reconcile_window_size(
            &mut world,
            balloon0,
            GhostWindowKind::Balloon,
            (446, 316),
            PlacementRoute::DpiReproject
        ),
        "balloon: 異寸ゆえ書込が成立する"
    );
    assert_eq!(
        size_of(&world, balloon0),
        Some(SizeI::new(446, 316)),
        "balloon: 新物理寸へ更新"
    );
    assert_eq!(
        pos_of(&world, balloon0),
        Some(Point { x: 1071, y: 732 }),
        "balloon: 位置は維持される（resize_window_keep_position＝アンカー再射影しない）"
    );

    // --- char: アンカー保存リサイズ（Bottom 再射影で y と中央 x が動く）---
    assert_eq!(
        pos_of(&world, char0),
        Some(Point { x: 1483, y: 757 }),
        "前提: char 初期位置"
    );
    assert!(
        reconcile_window_size(
            &mut world,
            char0,
            GhostWindowKind::Char,
            (868, 1374),
            PlacementRoute::DpiReproject
        ),
        "char: 異寸ゆえ書込が成立する"
    );
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(868, 1374)),
        "char: 新物理寸へ更新"
    );
    assert_eq!(
        pos_of(&world, char0),
        Some(Point { x: 1266, y: 70 }),
        "char: Bottom 再射影（y=1444−1374=70）＋下端中央保存（x=1483+217−434=1266）"
    );
}

/// 縮退（log-first・panic しない）: i32 域超過・0 寸は窓へ書かない。同寸はべき等 skip で
/// 書込ゼロ（`false` は失敗ではない）。
#[test]
fn reconcile_window_size_guards_and_idempotent_skip_write_nothing() {
    let (mut world, gw) = dpi_world();
    let char0 = gw.char_window(0).expect("char 窓がある");
    let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");

    // i32 域超過（u32 なら表現できるが窓寸に渡せない）→ 書かない。
    assert!(!reconcile_window_size(
        &mut world,
        char0,
        GhostWindowKind::Char,
        (u32::MAX, 687),
        PlacementRoute::DpiReproject
    ));
    // 0 寸（native 0 由来の退化）→ 書かない。
    assert!(!reconcile_window_size(
        &mut world,
        char0,
        GhostWindowKind::Char,
        (0, 687),
        PlacementRoute::DpiReproject
    ));
    assert!(!reconcile_window_size(
        &mut world,
        balloon0,
        GhostWindowKind::Balloon,
        (446, 0),
        PlacementRoute::DpiReproject
    ));
    // 同寸（k 不変で丸め後も同寸）→ べき等 skip（false は失敗でなく「書かなかった」）。
    assert!(!reconcile_window_size(
        &mut world,
        char0,
        GhostWindowKind::Char,
        (434, 687),
        PlacementRoute::DpiReproject
    ));
    assert!(!reconcile_window_size(
        &mut world,
        balloon0,
        GhostWindowKind::Balloon,
        (223, 158),
        PlacementRoute::DpiReproject
    ));

    assert_eq!(size_of(&world, char0), Some(SizeI::new(434, 687)), "char 寸不変");
    assert_eq!(
        size_of(&world, balloon0),
        Some(SizeI::new(223, 158)),
        "balloon 寸不変"
    );
    assert_no_write(&world, char0, "縮退・べき等 skip");
    assert_no_write(&world, balloon0, "縮退・べき等 skip");
}

/// **本 task の到達判定（tasks.md 4.2）**: 窓 DPI を差し替えた次のフェーズ実行で、当該窓の
/// client が `scaled_extent(applied, native)` と一致する。
///
/// 96→192（k=2/1）へ `DPI` を差し替え、presenter が報告する新物理寸として
/// `ScaleRatio::scaled_extent(native)` を与える（実 presenter の報告値はこの丸め権威で
/// 作られる——emo-present in-crate が所有する契約）。`dpi_phase_with` 一回で char 窓の
/// `WindowPos.size` が同一の `scaled_extent` に一致することを反証する（Req3.1/4.1/4.2）。
#[test]
fn dpi_phase_reconciles_changed_window_to_scaled_extent() {
    let (mut world, gw) = dpi_world();
    let char0 = gw.char_window(0).expect("char 窓がある");
    let native = (434u32, 687u32);
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(native.0 as i32, native.1 as i32)),
        "前提: 窓 client は k=1 相当の native 寸"
    );

    // 窓 DPI 192（k = 192/96 = 2/1）へ差し替え → Changed<DPI> 発火。
    world.entity_mut(char0).insert(DPI::from_dpi(192, 192));
    let k = ScaleRatio::new(192, 96).expect("非ゼロ比");
    let scaled = k.scaled_extent(native.0, native.1);

    let mut source = FakeReports::default();
    source.refresh.insert(shell_target(0).0, scaled);
    let mut state = None;
    dpi_phase_with(&mut source, &mut state, &mut world);

    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(scaled.0 as i32, scaled.1 as i32)),
        "DPI 差替後の同一フレームで窓 client＝scaled_extent(applied, native)"
    );
    assert_eq!(scaled, (868, 1374), "k=2/1・native 434×687 の検算値");
    assert!(
        source.calls_of("refresh").contains(&shell_target(0).0),
        "非空虚性: 当該窓の shell target に対し refresh が呼ばれた"
    );
}

/// 二経路の責任分界 (1)（**二重 resize しない**）: `refresh_scale` が再表示に成立して報告を
/// 返した場合、その要求は presenter 自身が消費済みであり、同一フレームの drain 後段の reconcile は
/// **窓へ一切書かない**（sentinel をフェーズ境界で戻して「以降の書込」だけを見る）。
#[test]
fn drain_reconcile_writes_nothing_when_refresh_already_consumed_the_report() {
    let (mut world, gw) = dpi_world();
    let char0 = gw.char_window(0).expect("char 窓がある");
    world.entity_mut(char0).insert(DPI::from_dpi(192, 192));

    let mut source = FakeReports::default();
    // 状態照合が積んだ要求（pending）と、再表示成立で返る報告（refresh）は**同一の 1 件**。
    source.refresh.insert(shell_target(0).0, (868, 1374));
    source.pending.insert(shell_target(0).0, (868, 1374));

    let mut state = None;
    dpi_phase_with(&mut source, &mut state, &mut world);
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(868, 1374)),
        "DPI 相で reconcile 済み（非空虚性の前提）"
    );

    // フェーズ境界: witness を戻し、以降（drain 後段の報告回収）の書込だけを観測する。
    reset_write_witness(&mut world, &gw);
    reconcile_reported_sizes(&mut source, &mut world);

    assert!(
        source.calls_of("take").contains(&shell_target(0).0),
        "非空虚性: drain 後段の報告回収は take を実際に呼んでいる（呼んだ上で None だった）"
    );
    assert_no_write(&world, char0, "drain 後段の報告回収による二重 resize");
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(868, 1374)),
        "窓寸は DPI 相の結果のまま（二重適用なし）"
    );
}

/// 二経路の責任分界 (2)（**取りこぼさない**・design Flow 3 手順 5）: `refresh_scale` の
/// ゲートが不成立で報告が返らなくても、表示成立点が積んだ未消費要求（初回表示の k₀ 補正）は
/// drain 後段の reconcile が同一フレーム内で拾って窓寸へ反映する。
#[test]
fn drain_reconcile_applies_undrained_report_when_refresh_gate_fails() {
    let (mut world, gw) = dpi_world();
    let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");

    let mut source = FakeReports::default();
    // refresh は空（k 不変等でゲート不成立＝報告なし）・pending のみ未消費で残る。
    source.pending.insert(balloon_target(0).0, (279, 198));

    let mut state = None;
    dpi_phase_with(&mut source, &mut state, &mut world);
    assert_no_write(&world, balloon0, "DPI 相はゲート不成立ゆえ書かない");

    reconcile_reported_sizes(&mut source, &mut world);
    assert_eq!(
        size_of(&world, balloon0),
        Some(SizeI::new(279, 198)),
        "未消費の要求を drain 後段の報告回収が拾って窓 client へ反映（取りこぼしなし）"
    );
    assert_eq!(
        pos_of(&world, balloon0),
        Some(Point { x: 1071, y: 732 }),
        "balloon は位置維持（resize_window_keep_position 経路）"
    );
}

/// 初回 run の全窓マッチ（`SystemState::new` 仕様）は churn を生まない: 報告が無ければ
/// 窓書込ゼロ。ただし**全窓に対し refresh が実際に呼ばれている**ことも同時に見る（空虚な
/// 「何も起きなかった」で通さない）。
#[test]
fn dpi_phase_first_run_matches_all_windows_without_churn() {
    let (mut world, gw) = dpi_world();
    let mut source = FakeReports::default(); // 報告なし＝k 差分なし相当
    let mut state = None;

    dpi_phase_with(&mut source, &mut state, &mut world);

    // 初回 run は全窓（2 スコープ×char/balloon＝4 target）へマッチする（非空虚性）。
    let mut refreshed = source.calls_of("refresh");
    refreshed.sort_unstable();
    assert_eq!(
        refreshed,
        vec![
            shell_target(0).0,
            balloon_target(0).0,
            shell_target(1).0,
            balloon_target(1).0
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>(),
        "初回 run は全ゴースト窓へマッチする（SystemState::new 仕様）"
    );
    // 報告が無ければ窓へは一切書かない（べき等 skip と合わせて churn ゼロ）。
    for scope in [0usize, 1] {
        assert_no_write(&world, gw.char_window(scope).unwrap(), "初回 run churn");
        assert_no_write(&world, gw.balloon_window(scope).unwrap(), "初回 run churn");
    }
}

/// `Changed<DPI>` が無いフレームは**仕事をしない**: 2 回目の run では refresh を一度も呼ばず
/// 窓書込もゼロ（永続 `SystemState` が `last_run` を跨いで保つ＝毎フレーム全マッチしない）。
#[test]
fn dpi_phase_without_dpi_change_does_no_work() {
    let (mut world, gw) = dpi_world();
    let mut source = FakeReports::default();
    let mut state = None;

    // 1 回目（初回 run の全マッチを消費）。
    dpi_phase_with(&mut source, &mut state, &mut world);
    assert!(
        !source.calls_of("refresh").is_empty(),
        "非空虚性: 1 回目は実際にマッチしている"
    );

    // 2 回目: DPI を一切触っていない → マッチ 0 件＝refresh 呼出ゼロ・窓書込ゼロ。
    source.calls.clear();
    dpi_phase_with(&mut source, &mut state, &mut world);
    assert!(
        source.calls_of("refresh").is_empty(),
        "Changed<DPI> 無しのフレームは refresh を呼ばない（実質 no-op）: {:?}",
        source.calls
    );
    for scope in [0usize, 1] {
        assert_no_write(&world, gw.char_window(scope).unwrap(), "変化なしフレーム");
        assert_no_write(
            &world,
            gw.balloon_window(scope).unwrap(),
            "変化なしフレーム",
        );
    }

    // 3 回目: 1 窓だけ DPI を差し替える → その窓だけがマッチする（検知が生きている証拠）。
    let char1 = gw.char_window(1).expect("char 窓がある");
    world.entity_mut(char1).insert(DPI::from_dpi(144, 144));
    source.calls.clear();
    dpi_phase_with(&mut source, &mut state, &mut world);
    assert_eq!(
        source.calls_of("refresh"),
        vec![shell_target(1).0],
        "変化した窓の target だけが refresh 対象"
    );
}
