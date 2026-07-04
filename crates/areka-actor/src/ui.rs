//! UI ブリッジ層: UI pump 上での drain を担う `UiSender`／`spawn_ui`。
//!
//! `UiSender`・`spawn_ui`・`UiSendError` は後続タスクで `async-channel`（unbounded）と
//! `wintf-winmsg-executor` の `spawn_local` を用いて実装される。純粋層（`spawn`/`reply`）には
//! 依存しない。
