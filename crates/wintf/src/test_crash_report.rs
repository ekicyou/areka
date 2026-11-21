//! 落ちの報告をこのクレートのテストの実行ファイルに据える。中身は `crash-report-kit` の 1 つだけを指す。

#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = crash_report_kit::install;

/// 落ちの報告（とプロセスの寿命の MTA）がテストより先に据わっている。
#[test]
fn crash_filter_is_installed_before_the_tests_start() {
    assert!(crash_report_kit::installed());
}
