//! 中継（`areka-mcp-bridge`）と共有する名前がクレートの外から読めること
//! （spec: areka-P0-mcp-stdio-bridge・要件 2.2・8.2）。
//!
//! 統合テストはクレートの外として建つので、ここが建つこと自体が「外から名前で読める」の判定になる。

use std::time::Duration;

#[test]
fn bridge_shared_names_are_readable_from_outside_the_crate() {
    // 綴りは受け口と中継の両方が頼る値なので固定する。
    assert_eq!(areka_mcp::SERVER_NAME, "areka-mcp-server");
    assert_eq!(areka_mcp::MCP_PATH, "/api/mcp/v1");
    assert_eq!(areka_mcp::BRIDGE_EXE_FILE_NAME, "areka-mcp-bridge.exe");

    // 候補の並びとツールの応答の上限は既に公開。中継が読む名前のまま届くことだけを見る。
    let _: (&str, [u16; 2], u16) = (
        areka_mcp::PORT_ENV,
        areka_mcp::DEFAULT_PORTS,
        areka_mcp::FALLBACK_STEPS,
    );
    let _: fn(Option<&str>) -> Vec<u16> = areka_mcp::candidates_from_env_value;
    let _: Duration = areka_mcp::tools::REPLY_WAIT;
}
