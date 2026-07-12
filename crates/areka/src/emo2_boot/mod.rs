//! areka-P0-emo2-boot 統合結線モジュール（M-boot: 「emo2 が起動して喋る」最初の可視結果）。
//!
//! 完成済み 5 トラックのエンジン（seriko／sakura／emo-present／emo-text／actor＋dola）を
//! 束ねて「動くアプリ」にする最後の一結線を所有する。新規機構は作らず、シェルアニメーション
//! 側の表示指令を表示層の指令へ変換するアダプタ 1 個＋各エンジンの結線＋二段の観測に徹する
//! （design.md「変更境界」・R10.4）。
//!
//! 依存方向（レイヤ規律・design.md「依存方向（レイヤ規律）」）:
//! `target_map`（純粋・std のみ）→ `adapter`（seriko/emo-present 型）→ `talk_clock`（sakura 型＋clock）
//! → `assets`（parsers/atlas/compose/seriko/emo-present）→ `frame`（bevy_ecs World・emo-present/emo-text 駆動）
//! → `main.rs`（全結線）。左のモジュールは右を import しない。
//!
//! 本ファイル群は Foundation タスク（tasks.md task 1）の骨格であり、各サブモジュールの
//! 機能実装は後続タスク（2〜6）が担う。

pub mod target_map;
pub mod adapter;
pub mod talk_clock;
pub mod assets;
pub mod frame;

use std::path::PathBuf;

/// 統合結線の構築時（load-time）失敗を観測可能化する誤り型（log-first・R7.3）。
///
/// 各段（mount／shell 読取＋parse／bake／balloon 組立／UI アクター spawn）の失敗を
/// `#[from]` 変換で集約し、呼び手（`wire_emo2_boot`）が `MountError::StartPointMissing` 系は
/// `warn!`・他は `error!` に分類して `LogSink`×2 フォールバック boot へ倒す（design.md
/// 「Error Categories and Responses」）。
///
/// 本タスク（tasks.md task 2.6・`build_boot_assets`）は構築入力の組立が返す load-time
/// バリアント（`Mount`／`Decoder`／`ShellRead`／`ShellEmpty`／`Balloon`）を充填する。
/// UI アクター spawn 失敗（`SpawnUi …`）は frame／wire 実装タスク（task 5.1）の領分ゆえ
/// 本タスクでは追加しない。分類（`MountError::StartPointMissing` 系＝想定内 `warn!`・他＝
/// `error!`）は呼び手（`wire_emo2_boot`・task 5.1）が担い、本型はバリアントを観測可能に
/// するのみ（panic せず `Err` を返す・log-first R7.3）。
///
/// `#[from]` は変換元が `std::error::Error` を実装する型のみに用いる。
/// `areka_parsers::package::MountError` は `std::error::Error` 未実装のため
/// `#[from]`/`#[source]` にせず値保持し `Debug` 表示する（placement `PlacementError`
/// と同流儀・working donor 準拠）。
#[derive(Debug, thiserror::Error)]
pub enum BootWiringError {
    /// ゴーストのマウント解決（`areka_parsers::package::resolve`）失敗。
    ///
    /// `MountError`（`StartPointMissing`／`ShellDirMissing` 等）は `std::error::Error`
    /// 未実装のため `#[from]` にせず値保持する（呼び手が variant 内容で warn/error 分類）。
    #[error("ゴーストのマウント解決に失敗: {0:?}")]
    Mount(areka_parsers::package::MountError),

    /// WIC デコーダ（COM）の生成失敗。呼び出しスレッドの COM 未初期化が主因。
    #[error("WIC デコーダの生成に失敗（COM 未初期化？）")]
    Decoder(#[source] windows::core::Error),

    /// shell ファイル（`surfaces.txt`／`descript.txt`）の読み取り失敗（I/O）。
    #[error("shell ファイルの読み取りに失敗: {path}")]
    ShellRead {
        /// 読み取れなかったファイルのパス。
        path: PathBuf,
        /// 元の I/O エラー。
        #[source]
        source: std::io::Error,
    },

    /// `surfaces.txt` は読めたが surface を 1 つも産まなかった（bake/表示対象なし）。
    #[error("surfaces.txt が surface 定義を産まなかった: {path}")]
    ShellEmpty {
        /// surface 定義が空だった `surfaces.txt` のパス。
        path: PathBuf,
    },

    /// バルーン表示対象（`areka_emo_present::build_balloon_target`）の構築失敗。
    ///
    /// `PresentError` は `thiserror` 派生（`std::error::Error` 実装）ゆえ `#[from]` で畳む。
    #[error("バルーン表示対象の構築に失敗")]
    Balloon(#[from] areka_emo_present::PresentError),
}
