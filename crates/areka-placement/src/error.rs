//! 配置準備パイプラインの失敗の型（spec: placement-crate-split 要件 7.6 ④）。
//!
//! 配置のモジュールを 1 つも引かない。根（`lib.rs`）が `pub use` で出し直すので、
//! 外の道 `areka_placement::PlacementError` は変わらない。

use std::path::PathBuf;

use areka_parsers::package::MountError;

/// 配置準備パイプライン（resolve→descript 読込→採寸→解決）の観測可能な失敗。
///
/// design「Error Handling」準拠: 安易な panic 禁止・失敗は `error!`＋`Err`。
/// すべて main.rs シームを経て呼び手の `main` へ返り、「起動窓を開けない」の告知と
/// 終了コード 1 になる（log-first）。
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

    /// モニタ列挙が 0 台で primary work area の出所がない（2.12 の基準を
    /// 満たせない）。架空の既定矩形は発明せず呼び手へ返す（`main` が「起動窓を開けない」
    /// を告知して終える）。task 6.1 で追加（mod.rs＝準備関数の置き場は本タスク境界内）。
    #[error("モニタ列挙に失敗: {reason}")]
    Monitor {
        /// 失敗理由（列挙結果の状況を文字列化）。
        reason: String,
    },
}
