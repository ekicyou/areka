//! Claude Desktop 向けの中継（spec: areka-P0-mcp-stdio-bridge）。
//!
//! 標準入出力の 1 行 1 要求を、`127.0.0.1` で待ち受ける areka の MCP の受け口へ届ける。

pub mod cache;
pub mod config;
pub mod errors;
pub mod http;
// 箱の見立て（spec: bridge-store-user-root）。クレートの中だけ（呼び手は `config::Config::from_env`）。
mod redirect;
pub mod scan;
pub mod service;
pub mod stdio;

/// テスト用の偽の受け口。通しのテスト（`tests/`）と同じ 1 つのファイルを部品のテストからも使う。
/// 使い手ごとに使う部品が違うので、使われない項目の警告は止める。
#[cfg(test)]
#[allow(dead_code)]
#[path = "../tests/common/fake_server.rs"]
pub(crate) mod fake_server;

/// 偽の受け口の自己確認。この部品のテストの束でだけ走る（通しのテストの束では読まない）。
#[cfg(test)]
#[path = "../tests/common/fake_server_tests.rs"]
mod fake_server_tests;
