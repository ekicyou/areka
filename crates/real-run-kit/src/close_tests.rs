//! `close` の兄弟のテスト。終わり方の読み分けを、本物の `quit_app` と同じ形の行の綴りの表で確かめる（5.3・D-5）。

use super::*;

/// 本物の `quit_app` の `info` の行（`tracing-subscriber` の `fmt`・着色なし）の形。
fn exit_line(origin: &str) -> String {
    format!(
        "2026-10-18T00:00:00.000000Z  INFO areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event=\"app_exit\" origin={origin} closed=2\n"
    )
}

#[test]
fn ended_by_reads_the_origin_of_the_exit_line() {
    let memory = r#"MemoryWriteFailed(PersistFailure { scope: Ghost, dir: Some("C:\a b\(x86)\profile\areka"), keys: [], cause: Io("拒否 (5)") })"#;
    let table: [(&str, EndedBy); 5] = [
        ("KanadeStopped(Quit)", EndedBy::UserClose),
        ("KanadeStopped(Fault(Timeout))", EndedBy::UserClose),
        ("OsClose", EndedBy::UserClose),
        ("Smoke", EndedBy::SafetyExit),
        (memory, EndedBy::Other(memory.to_string())),
    ];
    for (origin, want) in table {
        assert_eq!(ended_by(&exit_line(origin)), want, "origin={origin}");
    }
}

#[test]
fn ended_by_without_an_exit_line_is_unlogged() {
    assert_eq!(ended_by(""), EndedBy::Unlogged);
    // 似た名前の行（2 度目の出所・受け口なし）と受け取りの行は終わりの行ではない。
    let others = "x event=\"app_exit_again\" origin=Smoke\n\
                  x event=\"app_exit_unwired\" origin=Smoke\n\
                  x event=\"os_close_request\" scope=0 kind=\"ghost\"\n";
    assert_eq!(ended_by(others), EndedBy::Unlogged);
}

#[test]
fn ended_by_takes_the_first_exit_line() {
    let two = format!("{}{}", exit_line("Smoke"), exit_line("KanadeStopped(Quit)"));
    assert_eq!(ended_by(&two), EndedBy::SafetyExit);
    // 偽の子の形（受け取りの行の後に終わりの行・`origin` が最後の欄）。
    let fake = "a event=\"os_close_request\"\nb event=\"app_exit\" origin=KanadeStopped(Quit)\r\n";
    assert_eq!(ended_by(fake), EndedBy::UserClose);
}
