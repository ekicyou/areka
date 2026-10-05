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
