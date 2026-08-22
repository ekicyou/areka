//! tick の門の既定を起動時に上書きする読み口（`AREKA_TICK_GATE`）。
//!
//! # 何のためか
//!
//! tick の門（wintf の `EcsWorld::decide_tick`）は「見た目が 1 画素も変わらない画面更新
//! では 13 本のスケジュールを回さない」仕組みで、実装は無条件に入っているが**既定は
//! 無効**である。有効にするかどうかは、同じ実行体で ON と OFF を交互に走らせて比べて
//! 決める（設計 C16）。その切り替え口がここで、同時に「有効にしたら様子がおかしい」
//! ときに 1 つの環境変数で元へ戻せる安全弁でもある。
//!
//! # 読み方
//!
//! - `AREKA_TICK_GATE=1` / `true` / `on` → 門を**有効**にする
//! - `AREKA_TICK_GATE=0` / `false` / `off` → 門を**無効**にする
//! - 未設定・空 → 何も上書きしない（既定のまま）
//! - 表に無い綴り → 何も上書きしない（既定のまま）＋ `warn!` を 1 行残す
//!
//! 大文字小文字は区別せず、前後の空白は落とす。黙って倒す経路は 1 本も無い——
//! 読み解けたかどうかに関わらず、効いた値を `info!` で 1 行残す（要件 3.7）。

use tracing::{info, warn};
use wintf::ecs::world::EcsWorld;

/// 門の既定を上書きする環境変数の名前（`AREKA_` 冠・本 spec の登記語）。
pub const TICK_GATE_ENV: &str = "AREKA_TICK_GATE";

/// 環境変数の値を読み解いた結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickGateSetting {
    /// 未設定・空・空白のみ——指定なし（既定を動かさない）。
    Unset,
    /// 表に無い綴り——指定として扱わない（既定を動かさず、呼び出し側が `warn!` を残す）。
    Unknown,
    /// 明示の指定。
    Set(bool),
}

/// 環境変数の値を読み解く純関数（環境そのものには触らない）。
pub fn parse_tick_gate(value: Option<&str>) -> TickGateSetting {
    let Some(raw) = value else {
        return TickGateSetting::Unset;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return TickGateSetting::Unset;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" => TickGateSetting::Set(true),
        "0" | "false" | "off" => TickGateSetting::Set(false),
        _ => TickGateSetting::Unknown,
    }
}

/// [`parse_tick_gate`] の答えを「門を上書きするか否か」だけに縮めた形。
///
/// `None` は「上書きしない」であり、「無効にする」ではない（`Unset` と `Unknown` の
/// 違いは呼び出し側のログにだけ現れる）。
pub fn tick_gate_from_env_value(value: Option<&str>) -> Option<bool> {
    match parse_tick_gate(value) {
        TickGateSetting::Set(enabled) => Some(enabled),
        TickGateSetting::Unset | TickGateSetting::Unknown => None,
    }
}

/// 環境変数を 1 度読み、指定があれば門の既定を上書きする（起動時に 1 回）。
///
/// 何もしなかった場合も含め、効いた値を必ず 1 行残す——「設定したのに効いていない」を
/// ログだけで切り分けられるようにするためである。
pub fn apply_from_env(world: &mut EcsWorld) {
    let raw = std::env::var_os(TICK_GATE_ENV);
    // 不正な UTF-16 を含む値は読み解けないので、表に無い綴りと同じ扱いにする。
    let raw_str = raw.as_ref().and_then(|v| v.to_str());
    let setting = match raw.as_ref() {
        Some(_) if raw_str.is_none() => TickGateSetting::Unknown,
        _ => parse_tick_gate(raw_str),
    };

    if let TickGateSetting::Unknown = setting {
        warn!(
            "[tick_gate_config] {TICK_GATE_ENV} の値 `{}` は読み解けません（1|true|on / 0|false|off）。既定のまま続行します",
            raw_str.unwrap_or("<非 UTF-8>")
        );
    }
    if let Some(enabled) = tick_gate_from_env_value(raw_str) {
        world.set_tick_gate(enabled);
    }

    info!(
        "[tick_gate_config] {TICK_GATE_ENV}={} -> gate={}",
        raw_str.unwrap_or("unset"),
        world.tick_gate_enabled()
    );
}

#[cfg(test)]
#[path = "tick_gate_config_tests.rs"]
mod tests;
