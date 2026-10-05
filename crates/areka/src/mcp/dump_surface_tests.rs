//! `dump_surface` の決定論テスト（窓の無い World）。
//!
//! 結線状態の無い空の World で呼ぶと、その場で `NG:This ghost has no window`（isError: true・
//! content は本文 1 つ＝画像なし）を返す（要件 4.5・4.8・7.5）。描画を通る場合は GPU のテストが固定する。

use std::path::PathBuf;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};

use super::*;

#[test]
fn answers_no_window_at_once_with_an_empty_world() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let args = Args {
        scope: Some(0),
        surface: Some(0),
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(args.clone()));

    handle(&mut World::new(), &ghost, args, req.reply);

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text("NG:This ghost has no window".to_string())]
    );
    assert!(answer.outcome.is_error);
}

/// 写しまで済んだ成功は、呼んだスレッドでなく符号化のスレッドで仕上がり、そのスレッドから答える
/// （要件 6.1・タスク 4.3）。仕事は自分の走ったスレッドの名前を答えに書く。
///
/// # 非空虚性
/// 呼んだスレッド（テストのスレッド）で仕上げると、答えの名前がテストの名前になって赤。
/// 答えは上限つきで待つ（届かなければ `None` で赤）。
#[test]
fn success_is_finished_on_the_encoding_thread_and_answered_from_there() {
    let args = Args {
        scope: Some(0),
        surface: None,
        ghost_name: None,
    };
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(args));
    let job: Job = Box::new(|_| outcome::ok(std::thread::current().name().unwrap_or("")));

    reply_elsewhere(TOOL, Step::Encode(0, job), Instant::now(), req.reply);

    assert_eq!(
        pending.wait_answer().map(|a| a.outcome),
        Some(outcome::ok(ENCODE_THREAD))
    );
}

/// 単体の合成の結果が「無い」（スコープのシェルが表示の層に無い）ときは、判断の断りの文言でなく
/// 専用の文言で想定外の失敗を答え、`error!` を 1 件だけ出す（要件 2.1・7.1）。
///
/// # 非空虚性
/// 判断の断りの文言（`NO_SUCH_SCOPE`）のままだと本文が違って赤。`refuse` へ回すと ERROR が 0 件で赤。
#[test]
fn alone_without_composed_result_answers_shell_not_ready_with_one_error() {
    let (step, levels) = log_capture_kit::count_levels(|| alone(0, 0, None));

    let Step::Now(answered) = step else {
        panic!("その場で答える");
    };
    assert_eq!(
        answered.content,
        vec![ToolContent::Text(
            "NG:the shell of this scope is not ready".to_string()
        )]
    );
    assert!(answered.is_error);
    assert_eq!(levels.error, 1);
}

/// 符号化のスレッドの体で出た `error!` の件数（答えと一緒に返す）。5.1 の判定の本体で、
/// 下の較正で「わざと失敗させれば 1・成功なら 0」を確かめる。
fn errors_in_finish(job: Job) -> (ToolOutcome, usize) {
    let (answered, levels) = log_capture_kit::count_levels(|| finish(TOOL, 0, job, Duration::ZERO));
    (answered, levels.error)
}

/// `finish` の較正（要件 3.3・5.2）: panic する仕事は `NG:the encoding thread panicked` と ERROR 1 件、
/// `fail` を呼ぶ仕事は ERROR 1 件、成功する仕事は ERROR 0 件。同じ判定の関数で数える。
///
/// # 非空虚性
/// `finish` が panic を受けないとテストが panic して赤。判定の関数が常に 0 を返すと前の 2 つで赤、
/// 常に 1 を返すと最後で赤。
#[test]
fn finish_calibration_counts_errors_of_failing_jobs_only() {
    let (panicked, errors) = errors_in_finish(Box::new(|_| panic!("わざと")));
    assert_eq!(
        panicked.content,
        vec![ToolContent::Text(
            "NG:the encoding thread panicked".to_string()
        )]
    );
    assert!(panicked.is_error);
    assert_eq!(errors, 1);

    let (failed, errors) = errors_in_finish(Box::new(|_| fail(TOOL, 0, "わざと")));
    assert!(failed.is_error);
    assert_eq!(errors, 1);

    let (ok, errors) = errors_in_finish(Box::new(|_| outcome::ok("撮れた")));
    assert!(!ok.is_error);
    assert_eq!(errors, 0);
}

/// 受け取り口の無い送りは黙って捨てる（要件 3.5・7.1 ⑸）: panic せず ERROR 0 件。
///
/// # 非空虚性
/// 送りの失敗を `expect` すると panic で赤、`fail` へ回すと ERROR 1 件で赤。
#[test]
fn send_back_without_receiver_drops_silently() {
    let (tx, rx) = mpsc::channel();
    drop(rx);

    let ((), levels) = log_capture_kit::count_levels(|| {
        send_back(
            tx,
            finish(TOOL, 0, Box::new(|_| outcome::ok("撮れた")), Duration::ZERO),
        )
    });

    assert_eq!(levels.error, 0);
}

/// 成功の記録の欄（要件 5.3・7.1 ⑺）: `picture` の 1×1 の仕事を UI 時間 1,234 µs で `finish` に通すと、
/// `[mcp] 絵を返す` が 1 件で、`ui_us` が 1234、`encode_us` が数として読める。
///
/// # 非空虚性
/// 記録が無い・2 件・欄の名前が変わる・`ui` を渡し損ねる（0 や別の値）といずれも赤。
#[test]
fn success_record_carries_ui_us_and_encode_us() {
    let Step::Encode(scope, job) = picture(0, 0, "撮れた".to_string(), &[0, 0, 0, 0], 1, 1)
    else {
        panic!("写しまで済んだ成功");
    };

    let (answered, events) =
        log_capture_kit::capture(|| finish(TOOL, scope, job, Duration::from_micros(1234)));

    assert!(!answered.is_error);
    let records: Vec<_> = events
        .iter()
        .filter(|e| e.message() == "[mcp] 絵を返す")
        .collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].field("ui_us"), Some("1234"));
    assert!(
        records[0]
            .field("encode_us")
            .and_then(|v| v.parse::<u64>().ok())
            .is_some()
    );
}
