//! `ensure_process_dpi_awareness` の 1 回だけの決まり（要件 2.2・2.3・10.3）。

use super::ensure_process_dpi_awareness;

/// 2 回目からの呼び出しは何もしない＝`SetProcessDpiAwarenessContext` の行を 1 つも残さない。
///
/// 1 回目は同じプロセスの別のテスト（`WinApp::new`）が先に済ませているかもしれないので、
/// 捕まえるのは確実に 2 回目以降になる呼び出しだけにする（順に依らない）。
#[test]
fn second_call_does_nothing() {
    ensure_process_dpi_awareness();
    let ((), events) = log_capture_kit::capture(ensure_process_dpi_awareness);
    let dpi_lines: Vec<_> = events
        .iter()
        .filter(|e| e.message().contains("SetProcessDpiAwarenessContext"))
        .collect();
    assert!(dpi_lines.is_empty(), "second call logged: {dpi_lines:?}");
}
