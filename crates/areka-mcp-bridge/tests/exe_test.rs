//! 建てた exe（`CARGO_BIN_EXE_areka-mcp-bridge`）を起こす通しのテスト。

/// 部品のテスト（クレートの中）と同じ偽の受け口。ここで使わない部品の警告は止める。
#[allow(dead_code)]
#[path = "common/fake_server.rs"]
mod fake_server;
