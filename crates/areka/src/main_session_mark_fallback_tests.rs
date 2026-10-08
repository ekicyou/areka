//! 初回の起動が LogSink へ倒れた回の印（areka-P0-session-mark-residue 要件 4.1〜4.4・4.6・7.3）のテスト。
//!
//! `main_session_mark_tests.rs` の子モジュール（1 ファイル 1,000 行の目安で切り出した）。土台と
//! 補助（`first_boot`・`next_boot`・`expected_halted_next` など）は親のものを使う。

use super::*;

/// 初回の起動が LogSink へ倒れた（要件 4.1・4.3・4.6）: argv でなければ帰結を書いた
/// `warn!(session_mark_pinned_by_fallback)` が 1 件残り、置き場の単位の旗と `run()` の後に組む
/// 印の材料の旗が立つ。argv なら同じ事象は `debug!` どまり（旗は立つが判定は argv が先に効く）。
/// 結線が成立した起動では事象は 0 件で旗は偽（要件 4.8）。記録はどれも `boot_first_ghost` を
/// 呼んだスレッドで出るので、同じスレッドの捕捉で数える。
///
/// # 非空虚性
/// 1 行目が同じ捕捉で `warn!` 1 件を数えるので、3 行目の 0 件は捕捉の盲点ではない。
#[test]
fn first_boot_falling_back_pins_mark_with_one_warn() {
    /// （`warn!` の件数・`debug!` の件数・置き場の単位の旗・印の材料の (argv, 旗)・降ろせた）。
    type Row = (usize, usize, Option<bool>, Option<(bool, bool)>, bool);
    fn boot_and_read(rig: &mut SwitchRig, ghost: GhostDecision, argv: bool) -> Row {
        let ((), events) = capture(|| first_boot(rig, ghost, BootOrigin::Plain, argv));
        let count = |level| {
            events
                .iter()
                .filter(|e| {
                    e.level == level
                        && e.field_str("event") == Some("session_mark_pinned_by_fallback")
                })
                .count()
        };
        let slot_flag = rig
            .world
            .non_send::<GhostSlot>()
            .0
            .as_ref()
            .map(GhostSession::logsink_fallback);
        let mut after = after_run(&mut rig.world);
        let mark = after
            .mark
            .as_ref()
            .map(|m| (m.argv_session, m.logsink_fallback));
        let down_ok = down(rig, &mut after);
        (
            count(tracing::Level::WARN),
            count(tracing::Level::DEBUG),
            slot_flag,
            mark,
            down_ok,
        )
    }

    fn memory_a(rig: &SwitchRig) -> GhostDecision {
        GhostDecision {
            route: GhostRoute::Memory,
            dir: rig.root.ghost_dir("A"),
            folder: Some("A".to_owned()),
        }
    }

    let mut fallen = rig_with(FakeShiori::WiringFail);
    let argv = GhostDecision {
        route: GhostRoute::Argv,
        dir: fallen.root.ghost_dir("A"),
        folder: None,
    };
    let fallen_memory = memory_a(&fallen);
    let fallen_plain = boot_and_read(&mut fallen, fallen_memory, false);
    let fallen_argv = boot_and_read(&mut fallen, argv, true);
    // 足場は 1 つのスレッドに 1 つ（`RigPermit`）。先の足場を捨ててから次を作る。
    drop(fallen);
    let mut wired = rig_with(scripted_a());
    let wired_memory = memory_a(&wired);
    let wired_plain = boot_and_read(&mut wired, wired_memory, false);

    assert_eq!(
        vec![fallen_plain, fallen_argv, wired_plain],
        vec![
            (1, 0, Some(true), Some((false, true)), true),
            (0, 1, Some(true), Some((true, true)), true),
            (0, 0, Some(false), Some((false, false)), true),
        ],
        "LogSink へ倒れた初回の起動の記録か旗が崩れた（行: 倒れた・argv で倒れた・結線が成立した／\
         列: warn の件数・debug の件数・置き場の単位の旗・印の材料 (argv, 旗)・降ろせた）"
    );
}

/// `events` のうち `level` の `event` の件数。
fn count_event(
    events: &[log_capture_kit::CapturedEvent],
    level: tracing::Level,
    event: &str,
) -> usize {
    events
        .iter()
        .filter(|e| e.level == level && e.field_str("event") == Some(event))
        .count()
}

/// LogSink へ倒れた回の 1 周で観測したもの（次の起動を除く）。
#[derive(Debug, PartialEq, Eq)]
struct FallenLap {
    /// 初回の起動の `warn!(session_mark_pinned_by_fallback)` の件数。
    pinned_warns: usize,
    /// 置き場の単位に実行系が在るか（倒れた先の成否）。
    runtime: bool,
    /// 置き場の単位の倒れた旗。
    flag: bool,
    /// 後始末の（降ろせた・印の判定）。
    settled: (bool, Option<MarkVerdict>),
    /// `info!(session_mark_kept)` の `reason`（件数を兼ねて全件を並べる）。
    kept_reasons: Vec<String>,
    /// 後始末の後の印。
    mark: Option<String>,
}

/// 記憶の経路で決まった A の初回の起動が LogSink へ倒れ、OS の閉鎖要求できれいに終えて印を始末し、
/// 同じ根で次に起動する（`fn main` と同じ順）。戻りは（1 周の観測・次の起動・次の起動の期待）。
/// 倒れた先が実行系を起こした回は、SHIORI の接続の失敗で kanade の `Fault` の知らせが非同期に
/// 届きうるので、最初の出所の値は判定に使わない（理由は判定の順で `logsink_fallback` に決まる）。
fn fallen_lap(fake: FakeShiori) -> (FallenLap, NextBoot, NextBoot) {
    let mut rig = rig_with(fake);
    let ((), boot_events) = capture(|| first_boot_memory(&mut rig, "A"));
    let (runtime, flag) = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .map(|s| (s.runtime().is_some(), s.logsink_fallback()))
        .expect("置き場に単位が在る");
    quit_app(&mut rig.world, ExitOrigin::OsClose);
    let (settled, settle_events) = capture(|| after_run_and_settle(&mut rig));
    let app = app_dir(&rig);
    let lap = FallenLap {
        pinned_warns: count_event(
            &boot_events,
            tracing::Level::WARN,
            "session_mark_pinned_by_fallback",
        ),
        runtime,
        flag,
        settled,
        kept_reasons: settle_events
            .iter()
            .filter(|e| {
                e.level == tracing::Level::INFO && e.field_str("event") == Some("session_mark_kept")
            })
            .filter_map(|e| e.field_str("reason").map(str::to_owned))
            .collect(),
        mark: read_session_mark(&app),
    };
    // 最後に使ったゴーストの記憶が A でも、印が在る次の起動はそれを読まない。
    plant_last_ghost(&app, "A");
    let next = next_boot(&mut rig);
    let expected = expected_halted_next(&rig, "A");
    (lap, next, expected)
}

/// 倒れた回の 1 周の期待（倒れた先の実行系の有無だけが形で違う）。
fn expected_fallen_lap(runtime: bool) -> FallenLap {
    FallenLap {
        pinned_warns: 1,
        runtime,
        flag: true,
        settled: (true, Some(MarkVerdict::Keep("logsink_fallback"))),
        kept_reasons: vec!["logsink_fallback".to_owned()],
        mark: Some("A".to_owned()),
    }
}

/// 倒れた先の LogSink の起動が成功した（バルーンの根が無い・実行系は起きる・要件 4.1〜4.4・7.3）:
/// 倒れた `warn!` 1 件 → きれいに終えても印＝A が理由 `logsink_fallback` で残る → 次の起動は
/// 既定ゴースト＋Ref6＝halt・Ref7＝A。
#[test]
fn logsink_fallback_that_boots_keeps_mark_and_next_boot_is_default_with_halt() {
    let (lap, next, expected) = fallen_lap(FakeShiori::BalloonMissing);
    assert_eq!(
        lap,
        expected_fallen_lap(true),
        "倒れた先が成功した回の印が崩れた"
    );
    assert_eq!(next, expected, "次の起動");
}

/// 倒れた先の LogSink の起動も失敗した（ゴーストの根が無い・実行系なし・要件 4.1〜4.4・7.3）:
/// 成功した回と同じく印＝A が理由 `logsink_fallback` で残り、次の起動は既定ゴースト＋Ref6/7。
#[test]
fn logsink_fallback_that_fails_keeps_mark_and_next_boot_is_default_with_halt() {
    let (lap, next, expected) = fallen_lap(FakeShiori::WiringFail);
    assert_eq!(
        lap,
        expected_fallen_lap(false),
        "倒れた先も失敗した回の印が崩れた"
    );
    assert_eq!(next, expected, "次の起動");
}

/// LogSink へ倒れた回が OS のセッションの終了で終わる（要件 4.1・6.1）: 受け手は降ろす前に単位の
/// 倒れた旗を読んで印の材料に載せるので、上限の中できれいに降りても（打ち切りなし）印＝A が理由
/// `logsink_fallback` で残る。上限は負荷で揺れないよう大きく取って引数の入口で渡す。
#[test]
fn logsink_fallback_ended_by_os_session_end_keeps_mark() {
    let mut rig = rig_with(FakeShiori::BalloonMissing);
    first_boot_memory(&mut rig, "A");
    let app = app_dir(&rig);
    let ((), events) = capture(|| {
        crate::session_end::end_session_within(&mut rig.world, std::time::Duration::from_secs(60))
    });
    let kept: Vec<String> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("session_mark_kept"))
        .filter_map(|e| e.field_str("reason").map(str::to_owned))
        .collect();
    let done: Vec<(Option<&str>, Option<&str>)> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("os_session_end_done"))
        .map(|e| (e.field("down_ok"), e.field("shiori_cut")))
        .collect();

    assert_eq!(
        (kept, done, read_session_mark(&app), rig.shutdown()),
        (
            vec!["logsink_fallback".to_owned()],
            vec![(Some("true"), Some("false"))],
            Some("A".to_owned()),
            true,
        ),
        "倒れた回のセッションの終了で印が消えたか材料が崩れた（残す理由・所要の (降ろせた, 打ち切り)・印・置き場が空）: {events:?}"
    );
}

/// argv で始まって LogSink へ倒れた（要件 4.6）: 解決は印を読まず、起こしても書かず、きれいに終えても
/// 消さない（前から在った印がそのまま残る）。倒れた記録は `debug!` 1 件どまりで `warn!` は 0 件、
/// 印を残した記録も 0 件。
///
/// # 非空虚性
/// 同じ捕捉で同じ事象の `debug!` を 1 件数えるので、`warn!` の 0 件は捕捉の盲点ではない。
#[test]
fn argv_session_falling_back_neither_reads_writes_nor_clears_mark() {
    let mut rig = rig_with(FakeShiori::BalloonMissing);
    let app = app_dir(&rig);
    write_session_mark(&app, "前の印");
    let argv_dir = rig.root.ghost_dir("A");
    let found = resolve_boot_from(
        Ok((rig.root.dir().to_path_buf(), RootSource::EnvVar)),
        &["areka.exe".to_owned(), argv_dir.display().to_string()],
        &app,
        |n| panic!("無作為の段へ届いてはならない（候補 {n}）"),
    )
    .expect("argv で決まる")
    .3;
    let ghost = GhostDecision {
        route: GhostRoute::Argv,
        dir: argv_dir,
        folder: None,
    };
    let ((), boot_events) = capture(|| first_boot(&mut rig, ghost, BootOrigin::Plain, true));
    let flag = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .map(GhostSession::logsink_fallback);
    let mark_at_boot = read_session_mark(&app);
    quit_app(&mut rig.world, ExitOrigin::OsClose);
    let (settled, settle_events) = capture(|| after_run_and_settle(&mut rig));
    let pinned = "session_mark_pinned_by_fallback";

    assert_eq!(
        (
            found,
            count_event(&boot_events, tracing::Level::WARN, pinned),
            count_event(&boot_events, tracing::Level::DEBUG, pinned),
            flag,
            mark_at_boot,
            settled,
            count_event(&settle_events, tracing::Level::INFO, "session_mark_kept"),
            read_session_mark(&app),
        ),
        (
            None,
            0,
            1,
            Some(true),
            Some("前の印".to_owned()),
            (true, Some(MarkVerdict::Untouched)),
            0,
            Some("前の印".to_owned()),
        ),
        "argv で倒れた回が印に触れたか倒れた warn を出した（解決の戻り・warn・debug・旗・起こした直後・降ろせた＋判定・残した記録・後始末の後）"
    );
}
