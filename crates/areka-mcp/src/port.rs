//! `AREKA_MCP_PORT` の読み解き（純粋な判断）と環境変数の読み口。
//!
//! 判断は環境変数を読まない [`port_from_env_value`] が持ち（前例
//! `areka::perf_thread_report::period_from_env_value`）、環境変数を読むのは
//! [`read_port_env`] の 1 か所だけ。

use std::env::VarError;

use tracing::warn;

/// ポートを変える環境変数の名前（要件 2.7・`AREKA_` の冠）。
pub const PORT_ENV: &str = "AREKA_MCP_PORT";

/// 既定のポート（要件 2.1・SSP の 9801 とは別の番号）。
pub const DEFAULT_PORT: u16 = 9821;

/// 値 → 待ち受ける番号（Some）／待ち受けない（None・値が `0`）。
///
/// 空・空白だけ・数でない・負・65536 以上・溢れる値は `warn!` を 1 件残して
/// Some([`DEFAULT_PORT`])。前後の空白は許す（` 9000 ` → Some(9000)）。
/// 未設定（None）は Some([`DEFAULT_PORT`])・記録なし。`0` は「待ち受けない」の
/// 明示の値なので記録しない（`info!` は `start` が出す）。
pub fn port_from_env_value(value: Option<&str>) -> Option<u16> {
    let Some(raw) = value else {
        return Some(DEFAULT_PORT);
    };
    match raw.trim().parse::<u16>() {
        Ok(0) => None,
        Ok(port) => Some(port),
        Err(_) => {
            warn!(
                env = PORT_ENV,
                value = %raw,
                "[mcp] ポートとして読めない値なので既定の {DEFAULT_PORT} で待ち受ける（1〜65535 か、待ち受けないなら 0）"
            );
            Some(DEFAULT_PORT)
        }
    }
}

/// `std::env::var(PORT_ENV)` を読む。非 UTF-8 は `warn!` を 1 件残して
/// Some([`DEFAULT_PORT`])（既定で待ち受ける）。
pub fn read_port_env() -> Option<u16> {
    port_from_var(std::env::var(PORT_ENV))
}

/// 読んだ結果 → ポート。実際の環境変数に触れずに非 UTF-8 の経路を踏めるよう、
/// 読む行（[`read_port_env`]）と分けてある。
fn port_from_var(var: Result<String, VarError>) -> Option<u16> {
    match var {
        Ok(value) => port_from_env_value(Some(&value)),
        Err(VarError::NotPresent) => port_from_env_value(None),
        Err(VarError::NotUnicode(_)) => {
            warn!(
                env = PORT_ENV,
                "[mcp] ポートの指定が UTF-8 でないので既定の {DEFAULT_PORT} で待ち受ける"
            );
            Some(DEFAULT_PORT)
        }
    }
}

#[cfg(test)]
#[path = "port_tests.rs"]
mod port_tests;
