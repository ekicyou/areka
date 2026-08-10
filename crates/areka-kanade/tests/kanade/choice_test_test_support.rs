use super::{
    CallMethod, ChoiceInput, ExecutionSnapshot, KanadeMsg, MonotonicMs, RecordedCall, Sender,
    TalkCommand, TalkId,
};

/// `On` 始まりの任意名選択肢 ID（emo2 `menu.pasta` 実物と同形の日本語イベント名）。
pub(super) const NAMED_CHOICE_ID: &str = "Onおしゃべり頻度メニュー";

/// 正典形（`On` 始まりでない）の選択肢 ID。
pub(super) const CANONICAL_CHOICE_ID: &str = "頻度変更";

/// 選択肢の表示ラベル（`OnChoiceSelectEx` Ref0 の供給源）。
pub(super) const CHOICE_LABEL: &str = "おしゃべり頻度";

/// 選択由来の応答スクリプト（steady／close の fixture 語彙と別文字列にし、到達 StartTalk の
/// 由来を script で識別する）。
pub(super) const FIXED_CHOICE_SCRIPT: &str = r"\0\s[0]頻度を変えるね\e";

/// 選択確定に付随する参照列（記述順を保存することの観測対象）。
pub(super) fn choice_references() -> Vec<String> {
    vec!["ref-a".to_string(), "ref-b".to_string()]
}

/// 注入 `ChoiceWaiting` の表示完了時刻（[`establish_choice_wait`] が投函する値）。
const CHOICE_DISPLAY_END_MS: u64 = 1_000;

/// 選択肢タイムアウトの既定値（`KanadeConfig::choice_timeout_default_ms`・裁定 5・Req7.8）。
///
/// `timeout_directive_secs: None`（未指定）のとき kanade が加算する値の**期待値**である。
/// 檻はこの値を config から読まず独立に置く——実装側の既定値が動けば群 5 の 2 点注入
/// （期限手前で非発火・期限到達で発火）が両方向とも落ちる（値そのものの固定・Req7.8）。
const CHOICE_TIMEOUT_DEFAULT_MS: u64 = 30_000;

/// 注入 `ChoiceWaiting` から導かれる期限（`display_end + 既定値`・DD-8 写像の未指定分岐）。
pub(super) const CHOICE_DEADLINE_MS: u64 = CHOICE_DISPLAY_END_MS + CHOICE_TIMEOUT_DEFAULT_MS;

/// カスケード段・タイムアウト GET が帯びる運行状態（active talk＋選択待ち継続中＝`talking,choosing`）。
///
/// C5 の `choice_active` の源は帳簿の 3 段すべて（`Waiting`／`Cascading`／`TimeoutInFlight`）ゆえ、
/// カスケード段 GET と `OnChoiceTimeout` GET は同一の複合スナップショットを帯びる（裁定 6）。
pub(super) fn cascading_snapshot() -> ExecutionSnapshot {
    ExecutionSnapshot {
        talk_active: true,
        choice_active: true,
    }
}

/// 選択確定入力を組む（id／label／references はいずれも不透明転写・scope は Reference 非搬送）。
pub(super) fn choice_input(id: &str) -> ChoiceInput {
    ChoiceInput {
        id: id.to_string(),
        label: CHOICE_LABEL.to_string(),
        scope: 0,
        references: choice_references(),
    }
}

/// active talk（id=1）の窓で選択待ち帳簿を確立するまでの共通注入列。
///
/// Boot（挨拶なし＝`Steady{None}` 直行）→ Tick1（`OnSecondChange` GET Value → steady talk id=1・
/// TalkDone は保留）→ `ChoiceWaiting{talk_id:1, candidates}`。以降は `Steady{Some(1)}` かつ帳簿
/// `Waiting` であり、`KanadeMsg::Choice` の受領検証（talk_id 一致・候補集合一致）を通過できる。
pub(super) fn establish_choice_wait(sender: &Sender<KanadeMsg>, candidates: &[&str]) {
    sender.send(KanadeMsg::Boot).expect("send Boot");
    sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send Tick 1");
    sender
        .send(KanadeMsg::ChoiceWaiting {
            talk_id: TalkId(1),
            choice_ids: candidates.iter().map(|s| s.to_string()).collect(),
            display_end: MonotonicMs(CHOICE_DISPLAY_END_MS),
            // 未指定＝既定値へ委譲（期限は CHOICE_DEADLINE_MS・DD-8 の未指定分岐）。
            timeout_directive_secs: None,
        })
        .expect("send ChoiceWaiting");
}

/// 選択由来 GET（正典 3 ID ＋ 本檻が用いる任意名 ID）か。
fn is_choice_get(c: &RecordedCall) -> bool {
    c.method == CallMethod::Get
        && matches!(
            c.id.as_str(),
            "OnChoiceSelectEx" | "OnChoiceSelect" | "OnChoiceTimeout"
        )
        || (c.method == CallMethod::Get && c.id == NAMED_CHOICE_ID)
}

/// 記録列から選択由来 GET のみを処理順に抽出する。
pub(super) fn choice_gets(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded.iter().filter(|c| is_choice_get(c)).collect()
}

/// 記録列の選択由来 GET の id 列（発行順）——カスケード段列そのものの観測面。
pub(super) fn choice_get_ids(recorded: &[RecordedCall]) -> Vec<&str> {
    choice_gets(recorded)
        .into_iter()
        .map(|c| c.id.as_str())
        .collect()
}

/// 記録列から周期リクエスト（`OnSecondChange`）のみを処理順に抽出する（GET・NOTIFY 双方）。
pub(super) fn pumps(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded
        .iter()
        .filter(|c| c.id == "OnSecondChange")
        .collect()
}

/// 記録列における最初の一致位置（処理順の前後関係を突合するための索引）。
pub(super) fn position_of(recorded: &[RecordedCall], method: CallMethod, id: &str) -> usize {
    recorded
        .iter()
        .position(|c| c.method == method && c.id == id)
        .unwrap_or_else(|| panic!("{method:?} {id} が記録されているはず: {recorded:?}"))
}

/// [`TalkCommand`] 到着順を可読なタグ列へ写す（`Start`／`Resolve`／`Cancel` の順序観測面）。
///
/// `TalkCommand` は `PartialEq` を持たないため、順序の突合は本タグ列で行う（DD-4 の FIFO 契約は
/// 「どの指示がどの順で届いたか」で観測するのが正であり、値の等価性は要らない）。
pub(super) fn command_tags(commands: &[TalkCommand]) -> Vec<String> {
    commands
        .iter()
        .map(|c| match c {
            TalkCommand::Start(s) => format!("Start({})", s.talk_id.0),
            TalkCommand::ResolveChoice { talk_id, id } => format!("Resolve({},{})", talk_id.0, id),
            TalkCommand::CancelChoice { talk_id } => format!("Cancel({})", talk_id.0),
        })
        .collect()
}
