//! 本体に頼る切替・読み直し・起動・終わる相の層（runtime-module-untangle）。モジュールの宣言だけ。

/// 本体の口の束の中身と据え付け（`runtime::session_hooks::SessionHooks`）。
pub(crate) mod hooks;

/// 終わる相（停止通知の受け口・`ghost_quit_system`・`wire_kanade_stop`）。
pub(crate) mod ghost_quit;
