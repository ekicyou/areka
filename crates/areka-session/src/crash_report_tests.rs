//! 落ちの報告をこのクレートのテストの実行ファイルに据える。中身は `areka-runtime` の 1 つだけを指す。

#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = areka_runtime::crash_report_test_support::install;

/// 落ちの報告がテストより先に据わっている。
#[test]
fn crash_filter_is_installed_before_the_tests_start() {
    assert!(areka_runtime::crash_report_test_support::installed());
}
