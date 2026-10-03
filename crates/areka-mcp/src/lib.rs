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
pub use port::{
    DEFAULT_PORTS, FALLBACK_STEPS, PORT_ENV, candidates_from_env_value, read_port_candidates,
};
pub use registry::{ToolContent, ToolFuture, ToolHandler, ToolOutcome, ToolRegistry, ToolSpec};
pub use server::{McpServer, SHUTDOWN_WAIT, start};
