//! 配置準備パイプラインの失敗の型。
//!
//! 配置のモジュールを 1 つも引かない。根（`lib.rs`）が `pub use` で出し直すので、
//! 外の道 `areka_placement::PlacementError` は変わらない。

use std::path::PathBuf;

use areka_parsers::package::MountError;

/// 配置準備パイプライン（resolve→descript 読込→採寸→解決）の観測可能な失敗。
///
/// 安易な panic 禁止・失敗は `error!`＋`Err`。
/// 本番の呼び手は `areka-session` の `ghost_session::prepare_ghost_windows`（`prepare_ghost_windows_for_shell`
/// を呼ぶ）で、失敗はそこで包まれて返る。初回の起動（`main`）では「起動窓を開けない」の告知と
/// 終了コード 1 になり、ゴーストの切り替え（`reopen_ghost_windows`）では切り替えの失敗として
/// 扱われて終了はしない（log-first）。
#[derive(Debug, thiserror::Error)]
pub enum PlacementError {
    /// ゴーストパッケージのマウント解決（`areka_parsers::package::resolve_with_shell`）失敗。
    ///
    /// `MountError` は `std::error::Error` 未実装のため `#[from]`/`#[source]`
    /// にせず値として保持し `Debug` 表示する。
    #[error("ゴーストのマウント解決に失敗: {0:?}")]
    Mount(MountError),

    /// descript.txt の読み取り失敗（I/O エラー）。
    #[error("descript の読み取りに失敗: {path}")]
    DescriptRead {
        /// 読み取れなかった descript.txt のパス。
        path: PathBuf,
        /// 元の I/O エラー。
        source: std::io::Error,
    },

    /// surface 採寸（emo-atlas/compose による原寸合成）失敗。
    #[error("scope {scope} の surface 採寸に失敗: {reason}")]
    Measure {
        /// 採寸対象のスコープ番号。
        scope: usize,
        /// 失敗理由（下流の詳細を文字列化）。
        reason: String,
    },

    /// モニタ列挙が 0 台で primary work area の出所がない。
    /// 架空の既定矩形は発明せず呼び手へ返す（その後の扱いは型の doc のとおり）。
    #[error("モニタ列挙に失敗: {reason}")]
    Monitor {
        /// 失敗理由（列挙結果の状況を文字列化）。
        reason: String,
    },
}
