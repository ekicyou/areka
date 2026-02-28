//! cue-system のエラー型定義。

/// cue-system のエラー型（thiserror 2）
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CueSystemError {
    #[error("WaitForChoice に先行する Choice コマンドがありません (actor: {actor})")]
    EmptyChoiceBarrier { actor: String },

    #[error("EntityKey '{key}' に対応するエンティティが見つかりません")]
    EntityNotFound { key: String },

    #[error("CueQueue のキャパシティ上限 ({capacity}) を超過しました")]
    CapacityExceeded { capacity: usize },
}

/// CueSheet の実行結果。Modal Dialog の DialogResult に相当。
///
/// 1 CueSheet = 1 フィーチャー実行単位。
/// 上位層は `CueSheetTracker::result()` を毎フレーム poll して完了を検知する。
#[derive(Clone, Debug)]
pub enum CueSheetResult {
    /// 全演者の CueQueue が消費完了
    Completed,
    /// 外部からキャンセルされた
    Cancelled,
    /// バリアのタイムアウト超過
    Timeout,
    /// 選択肢が選択された
    Choice { id: String },
    /// プロトコル違反等のエラー
    Error(CueSystemError),
}
