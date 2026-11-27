//! `perf_thread_report` の決定論テスト。
//!
//! 実時間の閾値も `sleep` も合否に使わない——行の組み立て・差分の計算・
//! 名簿外の残りの算出・周期の読み取りはいずれも純関数であり、Win32 を通さずに固定できる。
//! スレッドを 1 本起こすのは終わり際のテストだけ（待ちの上限 `FINAL_WAIT` は合否に使わない）。
//! 点灯していない場（テスト既定）で報告スレッドが起きないこともここで押さえる。

use super::*;
use wintf::ecs::world::thread_registry::{FoldedReading, LiveReading};

/// 行を空白で切り、`名前=値` の**名前**だけを順に取り出す。
/// 判定スクリプト `tools/perf/judge-perf.py` の `parse_fields` と同じ切り方
/// （`=` を持たない語＝固定文言は落ちる）。
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

/// 検査用のスレッド標本（値は全て手で置く）。
fn sample_thread() -> ThreadSample {
    ThreadSample {
        role: "ui".to_owned(),
        name: "main".to_owned(),
        tid: 4321,
        kernel_us: 111_000,
        user_us: 222_000,
    }
}

/// 検査用のプロセス標本（値は全て手で置く）。
fn sample_process() -> ProcessSample {
    ProcessSample {
        wall_ms: 60_012,
        kernel_us: 400_000,
        user_us: 600_000,
        threads: 9,
    }
}

// ---------------------------------------------------------------------------
// 行の語彙
// ---------------------------------------------------------------------------

/// `perf(thread)` の 1 行に同じフィールド名が 2 度出ない。
#[test]
fn format_thread_line_has_no_duplicate_field_names() {
    let line = format_thread_line(3, 180, &sample_thread());
    let names = field_names(&line);
    let mut seen = std::collections::BTreeSet::new();
    for name in &names {
        assert!(
            seen.insert(*name),
            "フィールド名 {name} が重複している: {line}"
        );
    }
    assert_eq!(names.len(), 8, "フィールドの本数が変わった: {line}");
}

/// `perf(process)` の 1 行に同じフィールド名が 2 度出ない。
#[test]
fn format_process_line_has_no_duplicate_field_names() {
    let line = format_process_line(3, 180, &sample_process());
    let names = field_names(&line);
    let mut seen = std::collections::BTreeSet::new();
    for name in &names {
        assert!(
            seen.insert(*name),
            "フィールド名 {name} が重複している: {line}"
        );
    }
    assert_eq!(names.len(), 7, "フィールドの本数が変わった: {line}");
}

/// 固定文言とフィールドの並びを字面で固定する（`perf(thread)`）。
#[test]
fn format_thread_line_is_fixed_in_wording_and_order() {
    let line = format_thread_line(3, 180, &sample_thread());
    assert!(
        line.starts_with("perf(thread): スレッド別 CPU "),
        "固定文言が動いた: {line}"
    );
    assert_eq!(
        field_names(&line),
        vec![
            "snap",
            "t_s",
            "tid",
            "name",
            "role",
            "cpu_us",
            "kernel_us",
            "user_us",
        ]
    );
    assert_eq!(
        line,
        "perf(thread): スレッド別 CPU snap=3 t_s=180 tid=4321 name=main role=ui \
         cpu_us=333000 kernel_us=111000 user_us=222000"
    );
}

/// 固定文言とフィールドの並びを字面で固定する（`perf(process)`）。
#[test]
fn format_process_line_is_fixed_in_wording_and_order() {
    let line = format_process_line(3, 180, &sample_process());
    assert!(
        line.starts_with("perf(process): プロセス CPU "),
        "固定文言が動いた: {line}"
    );
    assert_eq!(
        field_names(&line),
        vec![
            "snap",
            "t_s",
            "wall_ms",
            "cpu_us",
            "kernel_us",
            "user_us",
            "threads",
        ]
    );
    assert_eq!(
        line,
        "perf(process): プロセス CPU snap=3 t_s=180 wall_ms=60012 cpu_us=1000000 \
         kernel_us=400000 user_us=600000 threads=9"
    );
}

/// `cpu_us` はカーネルとユーザーの合計である（両行とも）。
#[test]
fn cpu_us_is_the_sum_of_kernel_and_user() {
    let t = sample_thread();
    let line = format_thread_line(1, 60, &t);
    assert_eq!(
        field_value(&line, "cpu_us").unwrap(),
        (t.kernel_us + t.user_us).to_string()
    );

    let p = sample_process();
    let line = format_process_line(1, 60, &p);
    assert_eq!(
        field_value(&line, "cpu_us").unwrap(),
        (p.kernel_us + p.user_us).to_string()
    );
}

/// 壁時計と CPU 時間は別のフィールド名で出る（混ぜて読めない）。
#[test]
fn wall_clock_and_cpu_time_use_distinct_field_names() {
    let line = format_process_line(1, 60, &sample_process());
    assert_eq!(field_value(&line, "wall_ms").unwrap(), "60012");
    assert_eq!(field_value(&line, "cpu_us").unwrap(), "1000000");
    // スレッド行は CPU 時間しか持たない（壁時計を持たせない＝待ち時間を混ぜない）。
    let line = format_thread_line(1, 60, &sample_thread());
    assert!(field_value(&line, "wall_ms").is_none());
}

/// 名前・役割名の空白は `_` へ潰す——潰さないと値の途中が
/// 判定スクリプトに新しい `名前=` として読まれる。
#[test]
fn whitespace_inside_a_name_is_replaced_so_it_cannot_look_like_a_field() {
    let sample = ThreadSample {
        role: "actor:my actor".to_owned(),
        name: "worker 2\tb=1".to_owned(),
        tid: 7,
        kernel_us: 1,
        user_us: 2,
    };
    let line = format_thread_line(1, 1, &sample);
    assert_eq!(field_value(&line, "name").unwrap(), "worker_2_b=1");
    assert_eq!(field_value(&line, "role").unwrap(), "actor:my_actor");
    // 空白が消えたので `b=` は新しいフィールドとして立ち上がらない。
    assert_eq!(field_names(&line).len(), 8);
    assert!(!field_names(&line).contains(&"b"));
}

/// 空の名前は `-` になる（`name=` が値なしで終わらない）。
#[test]
fn an_empty_name_becomes_a_hyphen() {
    let sample = ThreadSample {
        role: "ui".to_owned(),
        name: String::new(),
        tid: 7,
        kernel_us: 0,
        user_us: 0,
    };
    let line = format_thread_line(1, 1, &sample);
    assert_eq!(field_value(&line, "name").unwrap(), "-");
    assert_eq!(field_names(&line).len(), 8);
}

// ---------------------------------------------------------------------------
// 標本の組み立て
// ---------------------------------------------------------------------------

fn sample(tid: u32, kernel_us: u64, user_us: u64) -> ThreadSample {
    ThreadSample {
        role: "ui".to_owned(),
        name: "t".to_owned(),
        tid,
        kernel_us,
        user_us,
    }
}

// ---------------------------------------------------------------------------
// 名簿外の残り
// ---------------------------------------------------------------------------

/// 名簿外の残り＝プロセス CPU − 名簿の合計。
#[test]
fn compute_unregistered_rest_is_process_minus_registered() {
    let process = ProcessSample {
        wall_ms: 1_000,
        kernel_us: 400_000,
        user_us: 600_000,
        threads: 2,
    };
    let threads = vec![sample(1, 300_000, 200_000), sample(2, 50_000, 150_000)];
    let rest = compute_unregistered_rest(&process, &threads, &[]);
    assert_eq!(rest.kernel_us, 50_000);
    assert_eq!(rest.user_us, 250_000);
    assert_eq!(rest.kernel_us + rest.user_us, 300_000);
}

/// 名簿の合計がプロセスを超えても（読み取り時刻のずれ）負へ回らない。
#[test]
fn compute_unregistered_rest_saturates_at_zero() {
    let process = ProcessSample {
        wall_ms: 1_000,
        kernel_us: 10,
        user_us: 10,
        threads: 1,
    };
    let threads = vec![sample(1, 999, 999)];
    let rest = compute_unregistered_rest(&process, &threads, &[]);
    assert_eq!((rest.kernel_us, rest.user_us), (0, 0));
}

/// 残りの行は固定の役割名と、スレッドを指さない身元で出る。
#[test]
fn compute_unregistered_rest_uses_the_fixed_role_and_placeholder_identity() {
    let process = sample_process();
    let rest = compute_unregistered_rest(&process, &[], &[]);
    assert_eq!(rest.role, ROLE_UNREGISTERED_REST);
    assert_eq!(rest.tid, 0);
    assert_eq!(rest.name, "-");
    // 名簿が空なら残りはプロセスの全量（黙って消えない）。
    assert_eq!(rest.kernel_us, process.kernel_us);
    assert_eq!(rest.user_us, process.user_us);
    let line = format_thread_line(1, 60, &rest);
    assert_eq!(field_value(&line, "role").unwrap(), "unregistered_rest");
    assert_eq!(field_value(&line, "tid").unwrap(), "0");
}

fn sample_folded() -> FoldedSample {
    FoldedSample {
        family: "talk thread-*".to_owned(),
        kernel_us: 1_000,
        user_us: 2_500,
        exited: 7,
    }
}

/// 畳んだ行は今の欄に `exited` を 1 つ足しただけで、欄名は重複しない。
#[test]
fn format_folded_line_has_no_duplicate_field_names_and_is_fixed_in_order() {
    let line = format_folded_line(3, 180, &sample_folded());
    let names = field_names(&line);
    let mut seen = std::collections::BTreeSet::new();
    for name in &names {
        assert!(
            seen.insert(*name),
            "フィールド名 {name} が重複している: {line}"
        );
    }
    assert_eq!(
        names,
        vec![
            "snap",
            "t_s",
            "tid",
            "name",
            "role",
            "cpu_us",
            "kernel_us",
            "user_us",
            "exited",
        ]
    );
    assert_eq!(
        line,
        "perf(thread): スレッド別 CPU snap=3 t_s=180 tid=0 name=- role=talk_thread-* \
         cpu_us=3500 kernel_us=1000 user_us=2500 exited=7"
    );
}

/// 畳んだ合計は残りの算出で、生きた行の合計と並べて両方が引かれ、0 で止まる。
#[test]
fn compute_unregistered_rest_subtracts_live_and_folded_and_saturates() {
    let process = ProcessSample {
        wall_ms: 1_000,
        kernel_us: 10_000,
        user_us: 10_000,
        threads: 1,
    };
    let live = vec![sample(1, 3_000, 4_000)];
    let folded = vec![sample_folded()];
    let rest = compute_unregistered_rest(&process, &live, &folded);
    assert_eq!((rest.kernel_us, rest.user_us), (6_000, 3_500));

    let big = FoldedSample {
        kernel_us: 99_000,
        user_us: 1_000,
        ..sample_folded()
    };
    let rest = compute_unregistered_rest(&process, &live, &[big]);
    assert_eq!((rest.kernel_us, rest.user_us), (0, 5_000));
}

// ---------------------------------------------------------------------------
// 周期の読み取り
// ---------------------------------------------------------------------------

/// 未設定は既定 60 秒・正の整数はその秒数・読めない値は既定へ倒す。
#[test]
fn period_from_env_value_defaults_and_parses() {
    let default = Duration::from_secs(DEFAULT_PERIOD_SEC);
    assert_eq!(period_from_env_value(None), default);
    assert_eq!(period_from_env_value(Some("5")), Duration::from_secs(5));
    assert_eq!(period_from_env_value(Some(" 5 ")), Duration::from_secs(5));
    assert_eq!(
        period_from_env_value(Some("0")),
        default,
        "0 秒は詰まるので既定へ"
    );
    assert_eq!(period_from_env_value(Some("abc")), default);
    assert_eq!(period_from_env_value(Some("")), default);
    assert_eq!(period_from_env_value(Some("-1")), default);
}

/// 環境変数の名前と点灯の的を固定する（本番 env は `AREKA_` 冠）。
#[test]
fn the_period_env_name_and_target_are_fixed() {
    assert_eq!(PERIOD_ENV, "AREKA_PERF_THREAD_REPORT_SEC");
    assert_eq!(PERF_TARGET, "areka::perf");
    assert_eq!(DEFAULT_PERIOD_SEC, 60);
    assert_eq!(THREAD_NAME, "areka-perf-report");
}

// ---------------------------------------------------------------------------
// 既定 OFF
// ---------------------------------------------------------------------------

/// 点灯していなければ報告スレッドを起こさない（費用 0）。
/// テストは subscriber を張らないので `areka::perf` は消灯している。
#[test]
fn start_spawns_nothing_while_the_target_is_off() {
    assert!(!is_enabled(), "テストの場で areka::perf が点いている");
    assert!(
        start().is_none(),
        "消灯しているのに報告スレッドが起きた（既定運転で費用を払っている）"
    );
}

// ---------------------------------------------------------------------------
// 終わり際
// ---------------------------------------------------------------------------

/// 最後の 1 枚の途中で報告スレッドが panic した → 記録は panic の error 1 件だけ（待ちきれなかった、ではない）。
#[test]
fn a_panicked_report_thread_is_recorded_as_a_panic_not_a_timeout() {
    let (stop, stop_rx) = mpsc::channel::<()>();
    let (done_tx, done) = mpsc::channel::<()>();
    let join = thread::spawn(move || {
        let _done = done_tx;
        let _ = stop_rx.recv();
        std::panic::resume_unwind(Box::new("テストの panic"));
    });
    let handle = ReportHandle { stop, done, join };
    let ((), events) = log_capture_kit::capture(|| handle.stop_and_report_final());
    let levels: Vec<_> = events.iter().map(|e| e.level).collect();
    assert_eq!(levels, [tracing::Level::ERROR], "{events:?}");
}

/// 停止を送る前に報告スレッドが panic していた → 記録は同じく panic の error 1 件だけ。
#[test]
fn a_report_thread_that_panicked_before_the_stop_is_recorded_as_a_panic() {
    let (stop, stop_rx) = mpsc::channel::<()>();
    let (done_tx, done) = mpsc::channel::<()>();
    let (gone_tx, gone) = mpsc::channel::<()>();
    let join = thread::spawn(move || {
        let _done = done_tx;
        drop(stop_rx);
        let _ = gone_tx.send(());
        std::panic::resume_unwind(Box::new("テストの panic"));
    });
    gone.recv().expect("停止の受け口が落ちた合図");
    let handle = ReportHandle { stop, done, join };
    let ((), events) = log_capture_kit::capture(|| handle.stop_and_report_final());
    let levels: Vec<_> = events.iter().map(|e| e.level).collect();
    assert_eq!(levels, [tracing::Level::ERROR], "{events:?}");
}

// ---------------------------------------------------------------------------
// 1 枚の組み立て
// ---------------------------------------------------------------------------

fn cpu(kernel_100ns: u64, user_100ns: u64) -> CpuTimes {
    CpuTimes {
        kernel_100ns,
        user_100ns,
    }
}

fn live(tid: u32, role: &str, cpu: windows::core::Result<CpuTimes>) -> LiveReading {
    LiveReading {
        role: role.to_owned(),
        name: None,
        tid,
        cpu,
    }
}

#[test]
fn compose_snapshot_orders_lines_counts_live_items_and_returns_unreadable_tids() {
    let reading = RegistryReading {
        live: vec![
            live(30, "ui", Ok(cpu(100, 200))),
            live(
                20,
                "actor:sakura-talk-2",
                Err(windows::core::Error::from(
                    windows::Win32::Foundation::E_FAIL,
                )),
            ),
            live(10, "watchdog", Ok(cpu(10, 20))),
        ],
        folded: vec![
            FoldedReading {
                family: "actor:sakura-talk-*".to_owned(),
                cpu: cpu(55, 66),
                exited: 3,
            },
            FoldedReading {
                family: "actor:kanade".to_owned(),
                cpu: cpu(19, 29),
                exited: 1,
            },
        ],
    };

    let composed = compose_snapshot(
        7,
        Duration::from_millis(61_500),
        cpu(10_000, 20_000),
        reading,
    );

    assert_eq!(composed.unreadable_tids, vec![20]);
    let lines = &composed.lines;
    assert_eq!(lines.len(), 6);
    assert!(lines[0].starts_with("perf(process):"));
    assert_eq!(field_value(&lines[0], "threads"), Some("3"));
    assert_eq!(field_value(&lines[0], "snap"), Some("7"));
    assert_eq!(field_value(&lines[0], "t_s"), Some("61"));
    assert_eq!(field_value(&lines[0], "wall_ms"), Some("61500"));

    let tid_role: Vec<_> = lines[1..]
        .iter()
        .map(|l| {
            (
                field_value(l, "tid").unwrap(),
                field_value(l, "role").unwrap(),
            )
        })
        .collect();
    assert_eq!(
        tid_role,
        vec![
            ("10", "watchdog"),
            ("30", "ui"),
            ("0", "actor:kanade"),
            ("0", "actor:sakura-talk-*"),
            ("0", "unregistered_rest"),
        ]
    );
    assert_eq!(field_value(&lines[1], "name"), Some("-"));
    assert_eq!(field_value(&lines[3], "kernel_us"), Some("1"));
    assert_eq!(field_value(&lines[3], "user_us"), Some("2"));
    assert_eq!(field_value(&lines[3], "cpu_us"), Some("3"));
    assert_eq!(field_value(&lines[3], "exited"), Some("1"));
    assert_eq!(field_value(&lines[4], "exited"), Some("3"));
    assert_eq!(field_value(&lines[5], "exited"), None);
    // 1000 - (1 + 10 + 1 + 5) / 2000 - (2 + 20 + 2 + 6)
    assert_eq!(field_value(&lines[5], "kernel_us"), Some("983"));
    assert_eq!(field_value(&lines[5], "user_us"), Some("1970"));
}
