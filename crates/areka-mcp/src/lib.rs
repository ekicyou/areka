//! MCP サーバの受け口（spec: areka-P0-mcp-server-core）。
//!
//! `127.0.0.1` の 1 つのポートで MCP の Streamable HTTP を待ち受け、JSON-RPC の
//! 解釈は公式 SDK（rmcp）に任せる。areka が持つのは待受・`Origin`／`Host` の検査・
//! 振り分け・登録案内（help）・ツールの登録口だけ。
//!
//! # tokio は mcp スレッドに閉じる
//!
//! 非同期ランタイム（tokio の `current_thread` 1 本）は `areka_actor::spawn_actor("mcp", …)`
//! で起こした `mcp` スレッドの中だけで作って回す。公開面に tokio の型は出さず、
//! 呼び出し側（`fn main()`）は同期のまま立てて、取っ手の `Drop` で畳む。

mod check;
mod dispatch;
mod gate;
mod handler;
mod help;
mod port;
mod registry;
mod server;
#[cfg(test)]
mod testkit;
pub mod tools;

// 公開面（rmcp・tokio・hyper の型は出さない）。
// 名乗り・受け口のパス・中継の exe の名前は中継（areka-mcp-bridge）が同じ定数を読む。
// 中継は、areka が居ないときの組み込みの答えの元（サーバーの版・説明の文・`tools::all_specs` の
// 定義の列）もここから読む。
// `request_shape_error` だけは例外で rmcp の型（`JsonRpcError`）を返す（中継が同じ答えを書く）。
pub use dispatch::{MCP_PATH, request_shape_error};
pub use handler::{INSTRUCTIONS, SERVER_NAME, SERVER_VERSION};
pub use help::BRIDGE_EXE_FILE_NAME;
pub use port::{
    DEFAULT_PORTS, FALLBACK_STEPS, PORT_ENV, candidates_from_env_value, read_port_candidates,
};
pub use registry::{ToolContent, ToolFuture, ToolHandler, ToolOutcome, ToolRegistry, ToolSpec};
pub use server::{McpServer, SHUTDOWN_WAIT, start};
