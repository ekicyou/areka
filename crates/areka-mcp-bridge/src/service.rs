//! 届いた要求への答えを決める判断と、中継を回す入口。

use rmcp::ClientHandler;
use rmcp::model::{ClientCapabilities, ClientConfig, Implementation};

/// areka の側へ立てる rmcp のクライアントの持ち主（設計 D-3）。名乗りは `areka-mcp-bridge` と
/// 中継の版。areka から届く要求・通知には rmcp の既定で応える（areka は送ってこない）。
pub struct BridgeClient;

impl ClientHandler for BridgeClient {
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            ClientCapabilities::default(),
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        )
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
