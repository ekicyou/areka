//! `AREKA_MCP_PORT` の読み解き（純粋な判断）と環境変数の読み口（設計判断 B-13）。
//!
//! 値を「試す候補の列」へ写す。判断は環境変数を読まない [`candidates_from_env_value`] が持ち
//! （前例 `areka::perf_thread_report::period_from_env_value`）、環境変数を読むのは
//! [`read_port_candidates`] の 1 か所だけ。既定の候補の並びを組むのもこのファイルの 1 か所だけで、
//! 束ねる・飛ばす・記録するのは `server`。

use std::env::VarError;

use tracing::warn;

/// ポートを変える環境変数の名前（要件 2.7・`AREKA_` の冠）。
pub const PORT_ENV: &str = "AREKA_MCP_PORT";

/// 既定の候補の先頭 2 つ（早い者勝ち・SSP と同じ番号＝要件 2.1）。
pub const DEFAULT_PORTS: [u16; 2] = [9801, 9821];

/// 両方とも使用中のとき隣へ逃げる幅（+1〜+9＝要件 2.1）。
pub const FALLBACK_STEPS: u16 = 9;

/// 既定の候補の列: `base + k` を `k` を外側・[`DEFAULT_PORTS`] を内側にして並べた 20 個
/// （9801・9821・9802・9822・…・9810・9830）。
fn default_candidates() -> Vec<u16> {
    (0..=FALLBACK_STEPS)
        .flat_map(|k| DEFAULT_PORTS.map(|base| base + k))
        .collect()
}

/// 値 → 試す候補の列。空の列は「待ち受けない」（値が `0`）。
///
/// 未設定（None）は既定の 20 個・記録なし。1〜65535 は `vec![p]`（前後の空白は許す＝
/// ` 9000 ` → [9000]・隣は足さない）。空・空白だけ・数でない・負・65536 以上・溢れる値は
/// `warn!` を 1 件残して既定の 20 個。`0` は「待ち受けない」の明示の値なので記録しない
/// （`info!` は `start` が出す）。
pub fn candidates_from_env_value(value: Option<&str>) -> Vec<u16> {
    let Some(raw) = value else {
        return default_candidates();
    };
    match raw.trim().parse::<u16>() {
        Ok(0) => Vec::new(),
        Ok(port) => vec![port],
        Err(_) => {
            warn!(
                env = PORT_ENV,
                value = %raw,
                "[mcp] ポートとして読めない値なので既定の候補で待ち受ける（1〜65535 か、待ち受けないなら 0）"
            );
            default_candidates()
        }
    }
}

/// `std::env::var(PORT_ENV)` を読む。非 UTF-8 は `warn!` を 1 件残して既定の 20 個。
pub fn read_port_candidates() -> Vec<u16> {
    candidates_from_var(std::env::var(PORT_ENV))
}

/// 読んだ結果 → 候補の列。実際の環境変数に触れずに非 UTF-8 の経路を踏めるよう、
/// 読む行（[`read_port_candidates`]）と分けてある。
fn candidates_from_var(var: Result<String, VarError>) -> Vec<u16> {
    match var {
        Ok(value) => candidates_from_env_value(Some(&value)),
        Err(VarError::NotPresent) => candidates_from_env_value(None),
        Err(VarError::NotUnicode(_)) => {
            warn!(
                env = PORT_ENV,
                "[mcp] ポートの指定が UTF-8 でないので既定の候補で待ち受ける"
            );
            default_candidates()
        }
    }
}

#[cfg(test)]
#[path = "port_tests.rs"]
mod port_tests;
