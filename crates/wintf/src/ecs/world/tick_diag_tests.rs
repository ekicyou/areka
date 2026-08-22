//! `tick_diag` の決定論テスト（task 2.2・要件 2.5/2.6/2.12/3.8/6.5/6.8）。
//!
//! 実時間の閾値は 1 つも使わない（要件 6.5）——窓の切れ目は注入した時刻で作り、
//! `sleep` も経過待ちも行わない。UI スレッド CPU の読み出しはハンドル未設定の状態
//! （テスト既定）では引数で与えるので、行の値も決定論的に定まる。

use super::*;

/// 行を空白で切り、`名前=値` の**名前**だけを順に取り出す（`[tick]` のような
/// `=` を持たない語は落とす）。判定スクリプトの `parse_fields` と同じ切り方。
fn field_names(line: &str) -> Vec<&str> {
    line.split(' ')
        .filter_map(|tok| tok.split_once('='))
        .map(|(name, _)| name)
        .collect()
}

/// 行から `名前=値` の値を引く（最初に現れたもの）。
fn field_value<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.split(' ')
        .filter_map(|tok| tok.split_once('='))
        .find(|(n, _)| *n == name)
        .map(|(_, v)| v)
}

/// `mod.rs` の本文から `try_tick_world` の関数本体だけを切り出す。
fn try_tick_world_body(src: &str) -> &str {
    let at = src
        .find("pub fn try_tick_world")
        .expect("mod.rs に try_tick_world が無い");
    let rest = &src[at..];
    let end = rest
        .find("\n    }\n")
        .expect("try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない");
    &rest[..end]
}

/// 検査用の窓集計（値は全て手で置く）。
fn sample_snapshot() -> WindowSnapshot {
    WindowSnapshot {
        frame: 4242,
        t_ms: 1001,
        ticks: 120,
        skipped: 7,
        heartbeat: 3,
        wall_us: 69_360,
        max_us: 1_234,
        ui_cpu_us: 55_000,
        per_schedule_us: [11, 22, 33, 44, 55, 66, 77, 88, 99, 110, 121, 132, 143],
    }
}

/// (要件 2.12) 1 行の中に同じフィールド名が 2 度出ない。
#[test]
fn format_window_line_has_no_duplicate_field_names() {
    let line = format_window_line(&sample_snapshot());
    let names = field_names(&line);
    let mut seen = std::collections::BTreeSet::new();
    for name in &names {
        assert!(
            seen.insert(*name),
            "フィールド名 `{name}` が 1 行に 2 度出ている（要件 2.12）: {line}"
        );
    }
    assert_eq!(
        names.len(),
        9 + SCHEDULE_NAMES.len(),
        "固定 9 フィールド＋13 本のスケジュールで 22 フィールドのはず: {line}"
    );
}

/// (要件 2.5) 行の頭は `[tick] kind=window`（新設の文言・既存行と衝突しない）。
#[test]
fn format_window_line_starts_with_the_tick_window_prefix() {
    let line = format_window_line(&sample_snapshot());
    assert!(
        line.starts_with("[tick] kind=window "),
        "行頭が `[tick] kind=window ` で始まっていない: {line}"
    );
}

/// (要件 2.6) 壁時計（`wall_us`／`max_us`／13 本）と CPU 時間（`ui_cpu_us`）は
/// 別のフィールド名で出る——同じ名前に混ぜない。
#[test]
fn wall_clock_and_cpu_time_use_distinct_field_names() {
    let line = format_window_line(&sample_snapshot());
    assert_eq!(field_value(&line, "wall_us"), Some("69360"));
    assert_eq!(field_value(&line, "max_us"), Some("1234"));
    assert_eq!(field_value(&line, "ui_cpu_us"), Some("55000"));
    assert_eq!(field_value(&line, "frame"), Some("4242"));
    assert_eq!(field_value(&line, "t_ms"), Some("1001"));
    assert_eq!(field_value(&line, "ticks"), Some("120"));
    assert_eq!(field_value(&line, "skipped"), Some("7"));
    assert_eq!(field_value(&line, "heartbeat"), Some("3"));
}

/// (要件 2.5) 13 本のスケジュールは `SCHEDULE_NAMES` の綴りと順序でそのまま並ぶ。
#[test]
fn format_window_line_lists_thirteen_schedules_in_fixed_order() {
    let snap = sample_snapshot();
    let line = format_window_line(&snap);
    let names = field_names(&line);
    let tail: Vec<String> = names[names.len() - SCHEDULE_NAMES.len()..]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let expected: Vec<String> = SCHEDULE_NAMES.iter().map(|n| format!("{n}_us")).collect();
    assert_eq!(
        tail, expected,
        "13 本のフィールドは `<名前>_us` を固定順で並べるべき: {line}"
    );
    for (i, name) in SCHEDULE_NAMES.iter().enumerate() {
        assert_eq!(
            field_value(&line, &format!("{name}_us")),
            Some(snap.per_schedule_us[i].to_string().as_str()),
            "{name}_us の値が配列の {i} 番と一致すべき"
        );
    }
}

/// (要件 2.5) 綴りの権威は 1 つ——`SCHEDULE_NAMES` は `SCHEDULE_LABELS` の小文字。
#[test]
fn schedule_names_are_the_lowercase_of_the_labels() {
    assert_eq!(SCHEDULE_LABELS.len(), SCHEDULE_NAMES.len());
    for (label, name) in SCHEDULE_LABELS.iter().zip(SCHEDULE_NAMES.iter()) {
        assert_eq!(
            &label.to_ascii_lowercase(),
            name,
            "`{label}` の小文字が名前"
        );
    }
}

/// (要件 2.5/3.4) `try_tick_world` が回す 13 本の**並び順**が `SCHEDULE_LABELS` と
/// 一致する（本番本文を読む構造検査）。相別の所要が別のスケジュールの名前で
/// 出れば順位表が嘘になるので、綴りと順序をここで縛る。
#[test]
fn try_tick_world_runs_the_thirteen_labels_in_the_declared_order() {
    let src = include_str!("mod.rs");
    let body = try_tick_world_body(src);
    let needle = "try_run_schedule(";
    let mut found: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find(needle) {
        let after = &rest[at + needle.len()..];
        let label = after
            .split(|c: char| c == ',' || c == ')')
            .next()
            .expect("run_schedule( の直後にラベルが要る")
            .trim();
        found.push(label.to_string());
        rest = after;
    }
    let expected: Vec<String> = SCHEDULE_LABELS.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        found, expected,
        "try_tick_world の 13 本の並びが SCHEDULE_LABELS と一致すべき"
    );
}

/// (要件 3.8) 前置ガードは `try_tick_world` の本文で**時刻取得より前**に在る。
/// OFF の運転で観測のために時刻を取らないことを、本文の並びで固定する。
#[test]
fn try_tick_world_evaluates_the_guard_before_taking_any_instant() {
    let src = include_str!("mod.rs");
    let body = try_tick_world_body(src);
    let guard = body
        .find("is_enabled()")
        .expect("try_tick_world の本文に前置ガードが無い");
    let instant = body
        .find("Instant::now()")
        .expect("try_tick_world の本文に時刻取得が無い（検査が空振りしている）");
    assert!(
        guard < instant,
        "前置ガード（{guard}）は最初の時刻取得（{instant}）より前に在るべき"
    );
}

/// (要件 3.8) 点灯していなければ、スケジュール 1 本の計時は何も記録しない
/// （`lap` を 13 回叩いても回数も所要も 0 のまま＝時刻取得の経路に入らない）。
#[test]
fn phase_timer_records_nothing_when_the_guard_is_off() {
    let mut timer = PhaseTimer::start(false);
    for _ in 0..SCHEDULE_NAMES.len() {
        timer.lap();
    }
    assert_eq!(timer.laps(), 0, "OFF では 1 度も計時しない");
    assert_eq!(
        timer.per_schedule(),
        &[0u64; 13],
        "OFF では 13 本の所要は 0 のまま"
    );
}

/// (要件 2.5) 点灯していれば 13 本ぶんの区間が 1 本ずつ記録される
/// （所要の**値**は実時間なので合否に使わない＝要件 6.5）。
#[test]
fn phase_timer_records_thirteen_laps_when_the_guard_is_on() {
    let mut timer = PhaseTimer::start(true);
    for _ in 0..SCHEDULE_NAMES.len() {
        timer.lap();
    }
    assert_eq!(timer.laps(), SCHEDULE_NAMES.len(), "13 本ぶん区切られる");
    // 14 本目を叩いても配列の外へは書かない（構成が増えても壊れない）。
    timer.lap();
    assert_eq!(timer.laps(), SCHEDULE_NAMES.len() + 1);
}

/// (要件 2.5) 窓は `TICK_DIAG_WINDOW_MS` に満たないうちは閉じない。
#[test]
fn window_does_not_close_before_the_window_length() {
    let t0 = Instant::now();
    let mut diag = TickDiag::default();
    diag.record_run(t0, 1, 500, &[1; 13], false);
    assert!(
        diag.take_window(t0 + Duration::from_millis(999), 0)
            .is_none(),
        "999ms では窓は閉じない"
    );
    assert!(
        diag.take_window(t0 + Duration::from_millis(1000), 0)
            .is_some(),
        "1000ms で窓が閉じる"
    );
}

/// (要件 2.5) 窓が閉じたら集計値が出て、次の窓は 0 から数え直す。
#[test]
fn window_close_reports_counters_and_then_resets_them() {
    let t0 = Instant::now();
    let mut diag = TickDiag::default();
    let per = [1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    diag.record_run(t0, 10, 100, &per, false);
    diag.record_run(t0 + Duration::from_millis(8), 11, 900, &per, true);
    diag.record_skipped(t0 + Duration::from_millis(16));
    diag.record_skipped(t0 + Duration::from_millis(24));

    let closed = t0 + Duration::from_millis(1000);
    let snap = diag
        .take_window(closed, 42_000)
        .expect("1000ms で窓が閉じるべき");
    assert_eq!(snap.frame, 11, "frame は最後に回った番号");
    assert_eq!(snap.t_ms, 1000);
    assert_eq!(snap.ticks, 2);
    assert_eq!(snap.skipped, 2);
    assert_eq!(snap.heartbeat, 1);
    assert_eq!(snap.wall_us, 1000, "wall_us は窓内の合計");
    assert_eq!(snap.max_us, 900, "max_us は窓内の最大");
    assert_eq!(snap.ui_cpu_us, 42_000);
    assert_eq!(snap.per_schedule_us[0], 2, "13 本は窓内で加算される");
    assert_eq!(snap.per_schedule_us[12], 26);

    // 次の窓は 0 から。まだ 1 秒経っていないので閉じない。
    assert!(
        diag.take_window(closed + Duration::from_millis(999), 0)
            .is_none(),
        "窓の起点は閉じた時刻へ進むべき"
    );
    diag.record_run(closed + Duration::from_millis(8), 12, 50, &[0; 13], false);
    let snap2 = diag
        .take_window(closed + Duration::from_millis(1000), 0)
        .expect("2 つ目の窓が閉じるべき");
    assert_eq!(snap2.ticks, 1, "回数は持ち越さない");
    assert_eq!(snap2.skipped, 0, "省略数は持ち越さない");
    assert_eq!(snap2.heartbeat, 0, "心拍数は持ち越さない");
    assert_eq!(snap2.wall_us, 50, "壁時計の合計は持ち越さない");
    assert_eq!(snap2.max_us, 50, "壁時計の最大は持ち越さない");
    assert_eq!(snap2.per_schedule_us, [0; 13], "13 本の合計は持ち越さない");
}

/// (要件 2.5) 1 度も記録が無いうちは窓が開かず、行も出ない。省略だけでも窓は開く。
#[test]
fn window_stays_closed_until_the_first_record() {
    let t0 = Instant::now();
    let mut diag = TickDiag::default();
    assert!(
        diag.take_window(t0 + Duration::from_secs(10), 0).is_none(),
        "記録が 1 件も無ければ窓は開いていない"
    );
    diag.record_skipped(t0);
    assert!(
        diag.take_window(t0 + Duration::from_millis(1000), 0)
            .is_some(),
        "省略だけでも窓は開く（省略率を出すため）"
    );
}

/// (要件 2.5) 観測チャネルの綴りが動かない。
#[test]
fn tick_target_and_window_length_are_fixed() {
    assert_eq!(TICK_TARGET, "wintf::tick");
    assert_eq!(TICK_DIAG_WINDOW_MS, 1000);
}
