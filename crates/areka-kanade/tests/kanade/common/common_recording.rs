//! 受理された shiori 呼出の記録単位と、`areka_kanade::events::*` からの期待値導出。
//!
//! 項目は親のファサードから再輸出されるため、消費側は `super::common::X` で引く。

use areka_kanade::{
    BalloonBinding, ExecutionSnapshot, ExecutionStatus, KanadeMsg, MonotonicMs, ShioriCall,
    TalkCommand, events,
};

use super::Harness;

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
    pub fn from_call(call: &ShioriCall) -> Self {
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

/// 元のイベントの呼出 `source` が台詞 `script` を返した後に送る `OnTranslate` の期待
/// [`RecordedCall`]（偽の shiori の既定は 204）。`snapshot` は再生を始める時点の状態。
///
/// 期待は `areka_kanade::events::on_translate` から導出する（Reference をハードコードしない）。
/// 展開の口は素通しなので、Reference0 は `script` そのものになる。
pub fn expected_translate(
    script: &str,
    source: &ShioriCall,
    snapshot: &ExecutionSnapshot,
) -> RecordedCall {
    let (ShioriCall::Get { id, references, .. } | ShioriCall::Notify { id, references, .. }) =
        source;
    let source = events::SourceEvent {
        id: id.clone(),
        references: references.clone(),
    };
    expected_call(events::on_translate(
        script,
        &source,
        ExecutionStatus::derive(snapshot),
    ))
}

// ============================================================================
// 檻どうしで写していた小さな道具（1 か所で直せば全部の檻に効く）
// ============================================================================

/// 記録列から周期リクエスト（`OnSecondChange`）のみを処理順に抽出する（GET・NOTIFY 双方）。
pub fn pumps(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded
        .iter()
        .filter(|c| c.id == "OnSecondChange")
        .collect()
}

/// active talk 窓（`OnSecondChange` NOTIFY）**より後**に現れる最初の `OnSecondChange` GET
/// （＝pump 再開・完了後の GET 復帰）を返す。NOTIFY が無ければ `None`。
///
/// `Steady{Some}` 中は NOTIFY のみで GET は出ず、TalkDone 着弾で `Steady{None}` へ復帰した後に初めて
/// GET が現れるため、「NOTIFY より後の最初の GET」が完了後の pump 再開を一意に指す。
pub fn resumed_get_after_active_window(recorded: &[RecordedCall]) -> Option<&RecordedCall> {
    let notify_idx = recorded
        .iter()
        .position(|c| c.method == CallMethod::Notify && c.id == "OnSecondChange")?;
    recorded
        .iter()
        .enumerate()
        .find(|(i, c)| *i > notify_idx && c.method == CallMethod::Get && c.id == "OnSecondChange")
        .map(|(_, c)| c)
}

/// kanade の受信箱へ 1 通送る。
pub fn send(harness: &Harness, msg: KanadeMsg) {
    harness.sender.send(msg).expect("send to kanade inbox");
}

/// kanade の受信箱へ `Tick` を 1 通送る。
pub fn tick(harness: &Harness, now: u64) {
    send(
        harness,
        KanadeMsg::Tick {
            now: MonotonicMs(now),
        },
    );
}

/// 偽の sakura が受けた指示 1 件を比べられる形へ写す（`TalkCommand` は `PartialEq` を持たない）。
///
/// `Start(<talk_id>,<台詞>)`・`Resolve(<talk_id>,<選択 ID>)`・`Cancel(<talk_id>)` の形。
pub fn command_tag(command: &TalkCommand) -> String {
    match command {
        TalkCommand::Start(s) => format!("Start({},{})", s.talk_id.0, s.script),
        TalkCommand::ResolveChoice { talk_id, id } => format!("Resolve({},{id})", talk_id.0),
        TalkCommand::CancelChoice { talk_id } => format!("Cancel({})", talk_id.0),
    }
}

/// キャラとバルーンの結びつけを組む。
pub fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}
