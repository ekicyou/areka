use super::*;
use crate::schedule::ActiveTalk;
pub(super) use crate::schedule::translate_test_support::config;

/// Steady{talk: None}（pending_close なし）を時刻 500・任意採番で構築する。
pub(super) fn steady_none(next_id: u64) -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: next_id,
        ..State::initial()
    }
}

/// Steady{talk: Some(id)} を構築する。
pub(super) fn steady_some(talk_id: TalkId, next_id: u64) -> State {
    State {
        phase: Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id,
                origin: "OnSecondChange",
                script: String::new(),
            }),
        },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: next_id,
        ..State::initial()
    }
}

// ============================================================
// 選択確定の受領検証とカスケード駆動（C4 規則 1／2／3・DD-4）
// ============================================================

/// 檻用の選択確定入力（id／label／付随参照列を明示して組む・scope は 0 固定）。
pub(super) fn choice_input_of(id: &str, label: &str, references: &[&str]) -> ChoiceInput {
    ChoiceInput {
        id: id.to_string(),
        label: label.to_string(),
        scope: 0,
        references: references.iter().map(|s| s.to_string()).collect(),
    }
}

/// 選択待ち帳簿つきの `Steady{Some(talk_id)}` を構築する（帳簿の talk は現行 talk と一致）。
pub(super) fn steady_with_ledger(
    talk_id: TalkId,
    next_id: u64,
    candidates: &[&str],
    phase: ChoicePhase,
) -> State {
    let mut s = steady_some(talk_id, next_id);
    s.choice = Some(ChoiceState {
        talk_id,
        candidates: candidates.iter().map(|c| c.to_string()).collect(),
        deadline: Some(MonotonicMs(32_000)),
        phase,
    });
    s
}

/// GET Action から (イベント ID の wire 形, Reference 列) を取り出す（GET 以外は panic）。
pub(crate) fn expect_get_call(action: &Action) -> (String, Vec<String>) {
    match action {
        Action::ShioriRequest(ShioriCall::Get { id, references, .. }) => {
            (id.as_str().to_string(), references.clone())
        }
        _ => panic!("expected GET ShioriRequest"),
    }
}

/// 帳簿の段フェーズを取り出す（帳簿不在は panic）。
pub(super) fn expect_ledger(state: &State) -> &ChoiceState {
    state.choice.as_ref().expect("選択待ち帳簿が存在するはず")
}

// ============================================================
// 選択待ち中の実行状態導出（Req6.1〜6.5・C5・裁定 6）
// ============================================================

/// ShioriRequest（GET/NOTIFY 問わず）の共通ヘッダ `Status` の wire 値を取り出す。
pub(super) fn status_wire(action: &Action) -> Option<String> {
    match action {
        Action::ShioriRequest(
            ShioriCall::Get { status, .. } | ShioriCall::Notify { status, .. },
        ) => status.render(),
        _ => panic!("expected ShioriRequest"),
    }
}
