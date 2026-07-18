//! `areka-ghost` — ⓪ghost 結線層。
//!
//! descript.txt 起点のマウント解決（`areka_parsers::package::resolve`）を入力として、
//! shiori 通信層（`shiori-host32-host` 経由の kanade shiori actor）・kanade（③）・
//! sakura（④・per-talk dispatcher 経由）・ticker（時刻供給）を起動順に結線し、
//! 終了時は逆順で統括する（design.md「Architecture」「System Flows」）。
//!
//! # このクレートの構成（design.md「File Structure Plan」）
//!
//! - [`runtime`] — boot 手順・`GhostRuntime`・終了統括・起動失敗の器（`GhostBootError`）。
//!   `boot`／`GhostRuntime`（`kanade()`／`dispatcher()`）は task 3.1 が実装済み。
//!   `shutdown`／`into_parts`（`GhostParts`・`GhostHandles`・`GhostShutdownError` を含む
//!   終了統括一式）は task 3.2 が実装済み。
//! - [`config`] — 運行設定の値源解決（`KanadeConfig` 値源・shell descript 読解）。task 2.3。
//! - [`dispatcher`] — 永続的な要求元と一時的な再生アクターの非対称吸収。task 2.5。
//! - [`ticker`] — 差し替え可能な時刻供給コンポーネント。task 2.2。
//! - [`relay`] — 相互依存の循環を解く汎用転送コンポーネント。task 2.1。
//! - [`shiori_wiring`] — 本番の SHIORI 接続手続き。task 2.6。
//! - [`sink`] — 本番既定の記録先コンポーネント（`LogSink`）。task 2.4。
//!
//! 公開 facade（design.md「File Structure Plan」の `src/lib.rs` 記載どおり）:
//! `boot`／`GhostRuntime`／`GhostBootOptions`／`ShioriWiring`／`TickerMode`／
//! `GhostBootError`／`GhostParts`／`GhostHandles`／`GhostShutdownError`／
//! `SystemVarSource`／`default_system_vars` を re-export する。

pub mod config;
pub mod dispatcher;
pub mod relay;
pub mod runtime;
pub mod shiori_wiring;
pub mod sink;
pub mod ticker;

pub use runtime::{
    GhostBootError, GhostBootOptions, GhostHandles, GhostParts, GhostRuntime, GhostShutdownError,
    ShioriWiring, SystemVarSource, TickerMode, boot, default_system_vars,
};
