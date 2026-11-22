//! UI の基盤の初期化の失敗が `error!` に残ることを `main.rs` の本文で固定する。
//!
//! `WinApp::with_exit_policy` を失敗させる口が外に無いので、呼び出しの直後の本文を見る。

use areka_runtime::source_text_test_support::{code_only, index_of};

#[test]
fn winapp_init_failure_is_logged() {
    let code = code_only(include_str!("main.rs"));
    let call = index_of(&code, "WinApp::with_exit_policy(ExitPolicy::Explicit)");
    let after = &code[call..];
    let end = index_of(after, ")?;");
    let stmt = &after[..end];
    assert!(stmt.contains(".inspect_err("), "inspect_err が無い");
    assert!(
        stmt.contains("winapp_init_failed"),
        "winapp_init_failed が無い"
    );
    assert!(stmt.contains("error!"), "error! が無い");
}
