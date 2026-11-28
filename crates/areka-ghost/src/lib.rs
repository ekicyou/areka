//! `areka-ghost` — ⓪ghost 結線層。
//!
//! descript.txt 起点のマウント解決（`areka_parsers::package::resolve_with_shell`）を入力として、
//! shiori 通信層（`shiori-host32-host` 経由の kanade shiori actor）・kanade（③）・
//! sakura（④・per-talk dispatcher 経由）・ticker（時刻供給）を起動順に結線し、
//! 終了時は逆順で統括する。
//!
//! # このクレートの構成
//!
//! - [`runtime`] — boot 手順・`GhostRuntime`・終了統括（`shutdown`／`into_parts`）・起動失敗の器
//!   （`GhostBootError`）。
//! - [`catalog`] — ベースウェアの根の目録（ゴースト・シェル・バルーンの列挙と素性・同梱バルーン）。
//! - [`catalog_roots`] — 根の並び（上ほど先に探す）と、フォルダ名・根の比べ方。
//! - [`config`] — 運行設定の値源解決（`KanadeConfig` 値源・shell descript 読解）。
//! - [`dispatcher`] — 永続的な要求元と一時的な再生アクターの非対称吸収。
//! - [`prop_get_sink`] — `\![get,property,…]` の受け口（値を引いて kanade へ返す）。
//! - [`prop_sink`] — 汎用プロパティ SET キュー（`areka.prop.set`）の受け口。
//! - [`relay`] — 相互依存の循環を解く汎用転送コンポーネント。
//! - [`shiori_inproc`] — x64 SHIORI DLL を同じプロセスに読む接続。
//! - [`shiori_wiring`] — 本番の SHIORI 接続手続き（i686 helper 経由）。
//! - [`sink`] — 本番既定の記録先コンポーネント（`LogSink`）。
//! - [`sylphya_wiring`] — 統一プロパティシステム sylphya の起動と結線。
//! - [`ticker`] — 差し替え可能な時刻供給コンポーネント。
//! - `translate_wiring`（crate 内） — 翻訳（`OnTranslate`）のための写しの源の結線。
//!
//! 公開 facade: `boot`／`boot_with_kanade_stop`／`boot_with_origin`／`boot_with_balloon`／`GhostRuntime`／
//! `GhostBootOptions`／`ShioriWiring`／`TickerMode`／`GhostBootError`／`GhostParts`／
//! `GhostHandles`／`GhostShutdownError`／`SystemVarSource`／`SystemVarWiring` を re-export する。

pub mod catalog;
pub mod catalog_roots;
pub mod config;
pub mod dispatcher;
pub mod prop_get_sink;
pub mod prop_sink;
pub mod relay;
pub mod runtime;
pub mod shiori_inproc;
pub mod shiori_wiring;
pub mod sink;
pub mod sylphya_wiring;
pub mod ticker;
mod translate_wiring;

// ログ捕捉の共有基盤。捕捉窓は crate 内でここ 1 箇所（実体は共有 crate `log-capture-kit`）。
#[cfg(test)]
mod test_log_capture;

// src の単体テストが分け合う道具。
#[cfg(test)]
mod test_support;

pub use runtime::{
    GhostBootError, GhostBootOptions, GhostHandles, GhostParts, GhostRuntime, GhostShutdownError,
    ShioriWiring, SystemVarSource, SystemVarWiring, TickerMode, boot, boot_with_balloon,
    boot_with_kanade_stop, boot_with_origin,
};
// 根の目録（列挙と素性）。
pub use catalog::{
    BalloonEntry, BasewareRoot, GhostEntry, Identity, RootMark, ShellEntry, sakura_name,
};
// 根の並びと、フォルダ名・根の比べ方。
pub use catalog_roots::{
    BasewareRoots, GhostBalloons, Hidden, InstallRoot, StoreKind, Survey, fold_key,
    same_folder_name, same_root,
};
// InProc 結線の connect closure 構成関数。
pub use shiori_inproc::inproc_connect;
