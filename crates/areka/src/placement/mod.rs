//! 窓配置機構（areka-P0-window-placement）のモジュール土台。
//!
//! ゴースト定義（shell dir＋ghost/shell descript の KV）からキャラ窓・バルーン窓の
//! 初期配置を解決し、窓 entity を組み立てる配置パイプラインの器。
//! 座標単位契約（design 正本 U1〜U5）に従い、配置パイプラインの座標・寸法は
//! **すべて物理 px 単一通貨**とする（論理 DIP・`BoxStyle` は持ち込まない）。
//!
//! 依存方向（design「Architecture Pattern & Boundary Map」の強制規約）:
//! `resolver`（純粋・std のみ）← `config`（areka-parsers のみ）←
//! `measure`（emo-atlas/compose）← `spawn`／`follow`（wintf/bevy_ecs）← main.rs シーム。
//! 左のモジュールは右へ import しない。
//!
//! 本ファイルは task 1（scaffold）時点ではサブモジュール宣言と失敗型
//! [`PlacementError`] のみを持つ。公開面（`prepare_ghost_windows`・`GhostWindows`・
//! `move_window_to` 等の再輸出）は後続タスク（2〜7）で実装される。

pub mod config;
pub mod follow;
pub mod measure;
pub mod resolver;
pub mod source;
pub mod spawn;

use std::path::PathBuf;

use areka_parsers::package::MountError;

/// 配置準備パイプライン（resolve→descript 読込→採寸→解決）の観測可能な失敗。
///
/// design「Error Handling」準拠: 安易な panic 禁止・失敗は `error!`＋`Err`。
/// すべて main.rs シームで捕捉され `spawn_dummy_window` フォールバックへ
/// 落ちる（DD14・log-first）。
#[allow(dead_code)] // scaffold（task 1）: 利用側は後続タスクで実装
#[derive(Debug, thiserror::Error)]
pub enum PlacementError {
    /// ゴーストパッケージのマウント解決（`areka_parsers::package::resolve`）失敗。
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
}
