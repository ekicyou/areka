//! 受理された shiori 呼出の記録単位と、`areka_kanade::events::*` からの期待値導出。
//!
//! `common/mod.rs`（1,657 行）から責務単位で切り出した子モジュール（タスク 8.2）。
//! 項目は親のファサードから再輸出されるため、消費側の `super::common::X` は不変である。

use areka_kanade::ShioriCall;

// ============================================================================
// RecordedCall — 受理された shiori 呼出の記録単位
// ============================================================================

/// shiori 呼出の Method 区別（GET / NOTIFY / Unload）。
///
/// [`ShioriMsg::Request`] の [`ShioriCall`] は GET/NOTIFY を型で区別する。
/// [`ShioriMsg::Unload`] は Method を持たないため、記録上は独立の [`Unload`](CallMethod::Unload)
/// として表す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallMethod {
    /// GET 呼出（`ShioriCall::Get`）。
    Get,
    /// NOTIFY 呼出（`ShioriCall::Notify`）。
    Notify,
    /// 正規終了経路（`ShioriMsg::Unload`）。
    Unload,
}

/// 受理された shiori 呼出 1 件の記録（Method・イベント id・References 構成）。
///
/// fixture・検証・実装が単一の正本（events 表）を共有するため、期待値は
/// [`expected_call`] / [`expected_unload`] で `areka_kanade::events::*` から導出する
/// （ハーネス内に期待 References 定数をハードコードしない・Req 7.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCall {
    /// 呼出の Method（GET / NOTIFY / Unload）。
    pub method: CallMethod,
    /// イベント id（`"OnBoot"` 等・Unload では `"Unload"`）。
    pub id: String,
    /// Reference 構成（順序保持）。
    pub references: Vec<String>,
    /// 送出時の `Status` 実行状態集合の wire 値（`None` ⇔ ヘッダ行なし・5.1）。
    ///
    /// `ExecutionStatus::render()` の結果を写す。これにより mock が記録した呼出と
    /// [`expected_call`] 導出の期待値が Status ヘッダまで含めて突合される
    /// （Testing Strategy #15・DD-IT-3/DD-IT-5）。Unload は Status を持たないため `None`。
    pub status: Option<String>,
}

impl RecordedCall {
    /// [`ShioriCall`] を記録単位へ変換する（GET/NOTIFY の別と id・References・Status を写す）。
    pub(super) fn from_call(call: &ShioriCall) -> Self {
        match call {
            ShioriCall::Get {
                id,
                references,
                status,
            } => RecordedCall {
                method: CallMethod::Get,
                id: id.as_str().to_string(),
                references: references.clone(),
                status: status.render(),
            },
            ShioriCall::Notify {
                id,
                references,
                status,
            } => RecordedCall {
                method: CallMethod::Notify,
                id: id.as_str().to_string(),
                references: references.clone(),
                status: status.render(),
            },
        }
    }
}

/// `areka_kanade::events::*` の [`ShioriCall`] を期待 [`RecordedCall`] へ導出する。
///
/// fixture・assert・実装が単一の正本（events 表）を共有するための唯一の経路。
/// テストは `expected_call(events::on_boot(&config))` のように書き、期待 References を
/// ハーネス側にハードコードしない（Req 7.1）。
pub fn expected_call(call: ShioriCall) -> RecordedCall {
    RecordedCall::from_call(&call)
}

/// Unload 呼出の期待 [`RecordedCall`]（`ShioriMsg::Unload` に対応）。
///
/// Unload は events 表の対象外（GET/NOTIFY を持たない正規終了経路）ゆえ、id は
/// `"Unload"`・References は空で固定する。
pub fn expected_unload() -> RecordedCall {
    RecordedCall {
        method: CallMethod::Unload,
        id: "Unload".to_string(),
        references: Vec::new(),
        // Unload は Status ヘッダを持たない正規終了経路（5.1）。
        status: None,
    }
}
