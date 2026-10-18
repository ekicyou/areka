//! `--sstp-listen` の値を MCP の待受の候補にする（要件 8）。

use std::net::Ipv4Addr;
use std::path::PathBuf;

/// MCP の待受の計画。
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum ListenPlan {
    /// `--sstp-listen` が渡されていない（今までどおり `AREKA_MCP_PORT` か既定・8.8）。
    #[default]
    NotGiven,
    /// `none` か値なし（8.4）。
    Off,
    /// 書いた順の番号（127.0.0.1 で試す・8.1〜8.3）。
    Ports(Vec<u16>),
    /// 待ち受け先はあったが全部はねた（8.7）。
    AllRejected,
}

/// はねた待ち受け先（記録の材料）。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ListenNote {
    /// 書式の誤り（8.5）。`reason` は `"port"`（番号が 1〜65535 でない・読めない）か `"format"`。
    Rejected { item: String, reason: &'static str },
    /// 127.0.0.1 以外のアドレス（8.6）。
    NotLoopback { item: String },
}

impl ListenNote {
    /// warn を 1 件出す。
    pub(crate) fn log(&self) {
        match self {
            Self::Rejected { item, reason } => tracing::warn!(
                event = "mcp_listen_rejected",
                item = %item,
                reason = *reason,
                "[cli] --sstp-listen の待ち受け先の書式が正しくないので、この待ち受け先を無視する"
            ),
            Self::NotLoopback { item } => tracing::warn!(
                event = "mcp_listen_not_loopback",
                item = %item,
                "[cli] areka の MCP はこの PC の中（127.0.0.1）でだけ待ち受けるので、この待ち受け先を使わない"
            ),
        }
    }
}

/// 純粋。`value` は `--sstp-listen` の値（None＝値なし）。
pub(crate) fn plan(value: Option<&str>) -> (ListenPlan, Vec<ListenNote>) {
    let Some(value) = value else {
        return (ListenPlan::Off, Vec::new());
    };
    if value.trim().eq_ignore_ascii_case("none") {
        return (ListenPlan::Off, Vec::new());
    }
    let mut ports = Vec::new();
    let mut notes = Vec::new();
    for item in value.split(',').map(str::trim) {
        match endpoint(item) {
            Ok(port) => ports.push(port),
            Err(note) => notes.push(note),
        }
    }
    if ports.is_empty() {
        (ListenPlan::AllRejected, notes)
    } else {
        (ListenPlan::Ports(ports), notes)
    }
}

/// 待ち受け先 1 つ → 番号。
fn endpoint(item: &str) -> Result<u16, ListenNote> {
    let rejected = |reason| ListenNote::Rejected {
        item: item.to_owned(),
        reason,
    };
    if item.is_empty() || item.eq_ignore_ascii_case("none") {
        return Err(rejected("format"));
    }
    let port = if item.starts_with('[') {
        // `[…]:p` の IPv6（`[::1]` を含む）はループバックでも使わない。
        return Err(if item.contains("]:") {
            ListenNote::NotLoopback {
                item: item.to_owned(),
            }
        } else {
            rejected("format")
        });
    } else if let Some((host, port)) = item.split_once(':') {
        if !host.eq_ignore_ascii_case("localhost") && host != "127.0.0.1" {
            return Err(if host.parse::<Ipv4Addr>().is_ok() {
                ListenNote::NotLoopback {
                    item: item.to_owned(),
                }
            } else {
                rejected("format")
            });
        }
        port
    } else {
        item
    };
    match port.parse::<u16>() {
        Ok(p) if p != 0 => Ok(p),
        _ => Err(rejected("port")),
    }
}

/// 純粋。計画 → 試す番号の列（None＝待ち受けない）。`current` は今の候補（`AREKA_MCP_PORT` か既定）。
pub(crate) fn ports_to_try(
    plan: &ListenPlan,
    current: impl FnOnce() -> Vec<u16>,
) -> Option<Vec<u16>> {
    match plan {
        ListenPlan::NotGiven => Some(current()),
        ListenPlan::Ports(ports) => Some(ports.clone()),
        ListenPlan::Off | ListenPlan::AllRejected => None,
    }
}

/// MCP の待受を立てる。`Off` は info・`AllRejected` は error を出して何も立てない（8.4・8.7）。
pub(crate) fn start_mcp(
    plan: &ListenPlan,
    tools: areka_mcp::ToolRegistry,
    bridge_exe: Option<PathBuf>,
) -> Option<areka_mcp::McpServer> {
    match ports_to_try(plan, areka_mcp::read_port_candidates) {
        Some(ports) => Some(areka_mcp::start(&ports, tools, bridge_exe)),
        None if *plan == ListenPlan::Off => {
            tracing::info!(
                event = "mcp_listen_off",
                "[cli] --sstp-listen の指定（none か値なし）により、MCP を待ち受けない"
            );
            None
        }
        None => {
            tracing::error!(
                event = "mcp_listen_all_rejected",
                "[cli] --sstp-listen の待ち受け先がすべて使えないので、MCP を待ち受けない（AREKA_MCP_PORT や既定の候補へは戻らない）"
            );
            None
        }
    }
}

#[cfg(test)]
#[path = "listen_tests.rs"]
mod listen_tests;
