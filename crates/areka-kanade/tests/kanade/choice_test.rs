//! 選択確定カスケードの統合檻（Req9.1／9.2(a)(b)・Req4.5 — 設計 Testing Strategy
//! 「Integration Tests（kanade 檻 `choice_test.rs`）」の (a)／(b)／DD-12 檻）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、注入
//! `ChoiceWaiting`／`Choice`／`Tick` のみで選択確定の全帰結を観測する。実時間 sleep も実時刻も
//! 用いず、全 join は期限付きである（Req9.1・steering `deterministic-test-coverage-mandate`）。
//! 期待 Reference 構成は必ず `events::on_choice_*` 構築子から `expected_call` で導出し、檻側に
//! ハードコードしない（fixture・assert・実装の三点一正本・Req7.1）。
//!
//! # 檻一覧（本ファイルが単一 pass/fail として固定する 3 群）
//! **群 1（Req9.2(a)・任意名形）**
//! 1. [`on_id_choice_fires_named_event_only_then_resolves_and_starts`]: `On` 始まり ID は任意名
//!    イベント **1 段のみ**を発行し（`OnChoiceSelectEx`／`OnChoiceSelect` を発行しない・裁定 1）、
//!    Value 応答が `ResolveChoice`→`Start`（この順・新 talk_id）を起こす。
//!
//! **群 2（Req9.2(b)・正典形）**
//! 2. [`canonical_choice_cascades_ex_then_select_and_resolves_without_start`]: `OnChoiceSelectEx`
//!    （Ref0=ラベル／Ref1=ID／Ref2 以降＝付随参照列）が先行し、204 で `OnChoiceSelect`（Ref0=ID）
//!    へ前進し、最終段 204 では `ResolveChoice` のみで **StartTalk が生じない**。
//! 3. [`canonical_choice_ex_value_short_circuits_select`]: 先行段が Value なら無印を**発行せず**
//!    `ResolveChoice`→`Start` へ短絡する（Req2.4・裁定 2）。
//!
//! **群 3（Req4.5・DD-12・選択起因の失敗）**
//! 4. [`choice_stage_failure_continues_as_204_without_fault_termination`]: 段 GET の `Failed` が
//!    `Unloading{Fault}` へ**倒れず** 204 相当で次段へ前進し、解決まで到達したうえで、終了は
//!    後から駆動した close によって起きる。
//! 5. [`non_choice_failure_during_choice_wait_still_faults`]: 同じ選択待ち窓でも **choice 起源で
//!    ない** GET の `Failed` は従来どおり `Unloading{Fault}` へ倒れる（DD-12 免除が非 choice 経路へ
//!    漏れていないこと＝既存 `failure_test.rs` の規律が不変であることの境界固定）。
//!
//! # 決定性（Req9.1）と同期イディオム
//! steady_test.rs／mouse_test.rs と同じ枠組み: 挨拶なし boot（`without_boot_greeting`）で
//! `Steady{None}` へ直行させ、Tick1 の `OnSecondChange` Value で steady talk（id=1）を起こし、
//! 保留ハーネス（`spawn_harness_gated`・hold_indices=[0]）でその `TalkDone` を park して active talk
//! 窓を維持する。選択待ち帳簿は「現行 talk と一致する `ChoiceWaiting`」でしか確立しないため、この
//! 窓が全群の前提である。カスケードは kanade drive ループの同期往復（execute-batch/reinject-last）で
//! 1 メッセージ処理内に完結するため、段の途中に Tick も別の選択確定も割り込まない。終了は末尾 talk を
//! quit:true にして駆動し、kanade の期限付き join 成功をもって全記録の確定点とする。
//!
//! # タイムアウトを踏まない構成
//! 注入する `ChoiceWaiting` は `timeout_directive_secs: None`（＝既定値 30_000ms へ委譲）で、起点は
//! `display_end = 1_000ms` ゆえ期限は 31_000ms である。本ファイルが注入する Tick は 1_000ms のみ
//! なので、タイムアウト経路（`OnChoiceTimeout`）には構造上到達しない（タイムアウト自体の檻は 6.2）。

use std::sync::mpsc::Sender;

use areka_kanade::{
    ChoiceInput, CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, MouseButton,
    MouseEventKind, MouseInput, TalkCommand, TalkId, events,
};

use super::common::{
    CallMethod, ChoiceResponse, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT,
    FailKind, FailOn, Fixture, Harness, QuitPolicy, RecordedCall, expected_call, expected_unload,
    join_bounded, spawn_harness_gated, spawn_harness_gated_failing,
};

/// `On` 始まりの任意名選択肢 ID（emo2 `menu.pasta` 実物と同形の日本語イベント名）。
const NAMED_CHOICE_ID: &str = "Onおしゃべり頻度メニュー";

/// 正典形（`On` 始まりでない）の選択肢 ID。
const CANONICAL_CHOICE_ID: &str = "頻度変更";

/// 選択肢の表示ラベル（`OnChoiceSelectEx` Ref0 の供給源）。
const CHOICE_LABEL: &str = "おしゃべり頻度";

/// 選択由来の応答スクリプト（steady／close の fixture 語彙と別文字列にし、到達 StartTalk の
/// 由来を script で識別する）。
const FIXED_CHOICE_SCRIPT: &str = r"\0\s[0]頻度を変えるね\e";

/// 選択確定に付随する参照列（記述順を保存することの観測対象）。
fn choice_references() -> Vec<String> {
    vec!["ref-a".to_string(), "ref-b".to_string()]
}

/// カスケード段の GET が帯びる運行状態（active talk＋選択待ち継続中＝`talking,choosing`）。
fn cascading_snapshot() -> ExecutionSnapshot {
    ExecutionSnapshot {
        talk_active: true,
        choice_active: true,
    }
}

/// 選択確定入力を組む（id／label／references はいずれも不透明転写・scope は Reference 非搬送）。
fn choice_input(id: &str) -> ChoiceInput {
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
fn establish_choice_wait(sender: &Sender<KanadeMsg>, candidates: &[&str]) {
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
            display_end: MonotonicMs(1_000),
            // 未指定＝既定 30_000ms へ委譲（期限 31_000ms・本檻の Tick は 1_000ms のみ）。
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
fn choice_gets(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded.iter().filter(|c| is_choice_get(c)).collect()
}

/// 記録列の選択由来 GET の id 列（発行順）——カスケード段列そのものの観測面。
fn choice_get_ids(recorded: &[RecordedCall]) -> Vec<&str> {
    choice_gets(recorded)
        .into_iter()
        .map(|c| c.id.as_str())
        .collect()
}

/// [`TalkCommand`] 到着順を可読なタグ列へ写す（`Start`／`Resolve`／`Cancel` の順序観測面）。
///
/// `TalkCommand` は `PartialEq` を持たないため、順序の突合は本タグ列で行う（DD-4 の FIFO 契約は
/// 「どの指示がどの順で届いたか」で観測するのが正であり、値の等価性は要らない）。
fn command_tags(commands: &[TalkCommand]) -> Vec<String> {
    commands
        .iter()
        .map(|c| match c {
            TalkCommand::Start(s) => format!("Start({})", s.talk_id.0),
            TalkCommand::ResolveChoice { talk_id, id } => format!("Resolve({},{})", talk_id.0, id),
            TalkCommand::CancelChoice { talk_id } => format!("Cancel({})", talk_id.0),
        })
        .collect()
}

// ============================================================================
// 群 1: Req9.2(a) — `On` 始まり ID は任意名イベント 1 段のみ
// ============================================================================

/// `On` 始まり選択肢 ID の確定が **任意名イベントだけ**を発行し、その Value 応答から
/// `ResolveChoice`→`Start`（新 talk_id）が起きる（Req9.2(a)・2.1・3.3・4.1・4.6・5.1・裁定 1）。
///
/// # 駆動（決定的・sleep なし）
/// active talk 窓（id=1・保留）で `ChoiceWaiting{candidates:[NAMED_CHOICE_ID]}` → `Choice{NAMED}`。
/// mock は任意名 id へ script を注入済みゆえ Value を返し、カスケードは 1 段で短絡する。選択由来
/// talk（id=2）は quit:true で終了系列を駆動し、kanade の期限付き join が全記録の確定点になる。
///
/// # 非空虚性・discriminative 性
/// - 任意名 GET が **events 表導出と完全一致**（id＝選択肢 ID 逐語・Ref0 以降＝付随参照列のみ・
///   ラベルと ID を Reference に載せない・Status: `talking,choosing`）で 1 件だけ記録される。
/// - `OnChoiceSelectEx`／`OnChoiceSelect` が記録列に**一切現れない**ことを明示検証する——先行段を
///   発行する退行（裁定 1 の破れ）はここで落ちる。
/// - `TalkCommand` 到着順が `Start(1)`→`Resolve(1,NAMED)`→`Start(2)` である。解決が起動より後に
///   なる／解決が落ちる／起動が旧 id を再利用する、のいずれの退行でもタグ列が食い違って落ちる。
#[test]
fn on_id_choice_fires_named_event_only_then_resolves_and_starts() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            NAMED_CHOICE_ID,
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // hold_indices=[0]: steady talk（id=1）の TalkDone を park して active talk 窓を維持する。
    // quit_flags: index0（steady talk）=false・index1（選択由来 talk）=true（終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[NAMED_CHOICE_ID]);
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)))
        .expect("send Choice");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade named-choice join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the choice-derived talk (quit:true)");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura named-choice join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 段列は任意名 1 段のみ（Ex／無印は一切発行されない・裁定 1）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec![NAMED_CHOICE_ID],
        "`On` 始まり ID は任意名イベント 1 段のみを発行するはず（Ex／無印の先行なし・裁定 1）: {recorded:?}"
    );

    // (2) 任意名 GET の layout が events 表導出と完全一致する（Ref0 以降＝付随参照列のみ・Req3.3）。
    let expected_named = expected_call(events::on_choice_named(
        NAMED_CHOICE_ID.to_string(),
        &choice_references(),
        &cascading_snapshot(),
    ));
    let named = choice_gets(&recorded);
    assert_eq!(
        *named[0], expected_named,
        "任意名 GET は events 表導出（id 逐語・Ref0..=参照列・Status）と完全一致するはず: {recorded:?}"
    );
    assert_eq!(
        named[0].references,
        choice_references(),
        "任意名イベントの Reference は付随参照列のみ（ラベル・ID を含めない・Req3.3）"
    );
    assert_eq!(
        named[0].status,
        Some("talking,choosing".to_string()),
        "カスケード段の GET は選択待ち継続中の複合 Status を帯びる（Req3.6・6.1・裁定 6）"
    );

    // (3) 応答から解決と起動がこの順で起きる（DD-4・Req5.1／4.1／4.6）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{NAMED_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "解決指示は起動指示の直前に、同一チャンネルの FIFO 順で届くはず: {commands:?}"
    );

    // (4) 起動は新 talk_id・選択由来スクリプト（旧 id を再利用しない・Req4.1）。
    let started: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            TalkCommand::Start(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は steady talk"
    );
    assert_eq!(
        started[1].script, FIXED_CHOICE_SCRIPT,
        "2 本目は選択由来の応答スクリプト（既存の起動棚に載った）"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "選択由来 talk は新 talk_id=2（旧 id=1 を再利用しない）"
    );

    // (5) 終了系列を完走した（末尾 Unload）＝選択処理後も状態整合を保っている。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（選択由来 talk quit:true→Unloading{{Quit}}→Unload）"
    );
}

// ============================================================================
// 群 2: Req9.2(b) — 正典形カスケード（Ex 先行 → 無印 → 解決のみ／Ex Value で短絡）
// ============================================================================

/// 正典形 ID の確定が `OnChoiceSelectEx`（Ref0=ラベル／Ref1=ID／Ref2 以降＝付随参照列）を先行
/// 発行し、204 で `OnChoiceSelect`（Ref0=ID）へ前進し、最終段 204 では `ResolveChoice` のみで
/// **StartTalk を起こさない**（Req9.2(b)・2.2・2.3・3.1・3.2・4.2・5.3・裁定 2／3）。
///
/// # 駆動（決定的・sleep なし）
/// 選択由来 GET へは何も注入しない（＝両段とも 204）。選択処理後に `CloseRequest` を積み、保留を
/// 解放して steady talk の `TalkDone{Ended}` を着弾させると `pending_close` が消化されて close 握手が
/// 走り、別れ talk（quit:true）が終了系列を駆動する。`release_all` は全注入の**後**に呼ぶため、
/// 保留 TalkDone は必ず CloseRequest の後ろに並ぶ（mpsc FIFO・race なし）。
///
/// # 非空虚性・discriminative 性
/// - 選択由来 GET の id 列が `[OnChoiceSelectEx, OnChoiceSelect]` **この順**であること。順序が逆／
///   片方欠落／無印が 204 で更に進む、のいずれの退行でも落ちる。
/// - 両段の記録が events 表導出と完全一致し、Ex の Ref0/Ref1/Ref2.. と無印の Ref0 を実値で明示確認
///   する（Reference 割付の取り違えを直接検出する）。
/// - 到達 StartTalk が steady talk と close talk のちょうど 2 本であること＝最終段 204 が talk を
///   **起こしていない**こと（起こしていれば 3 本になり落ちる）。解決指示はちょうど 1 件である。
#[test]
fn canonical_choice_cascades_ex_then_select_and_resolves_without_start() {
    // quitting: close 握手が別れ talk（Value）を返す。選択由来 GET は未注入＝両段とも 204。
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);

    // quit_flags: index0（steady talk）=false（Ended→pending_close 消化）・index1（close talk）=true。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice");
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");
    // 保留解放は全注入の後——保留 TalkDone は CloseRequest の後ろに並ぶ（FIFO・決定的）。
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade canonical-choice join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close (farewell talk quit:true)");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura canonical-choice join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 段列は Ex 先行 → 無印後続（Req2.2／2.3・裁定 2）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceSelectEx", "OnChoiceSelect"],
        "正典形は Ex を先行させ、204 で無印へ前進するはず: {recorded:?}"
    );

    // (2) Ex の Reference 割付（Ref0=ラベル／Ref1=ID／Ref2 以降＝付随参照列・Req3.1）。
    let gets = choice_gets(&recorded);
    let expected_ex = expected_call(events::on_choice_select_ex(
        CHOICE_LABEL,
        CANONICAL_CHOICE_ID,
        &choice_references(),
        &cascading_snapshot(),
    ));
    assert_eq!(
        *gets[0], expected_ex,
        "Ex GET は events 表導出（Ref0=ラベル／Ref1=ID／Ref2..=参照列・Status）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        gets[0].references.len(),
        4,
        "Ex の Reference は 2＋参照列 2＝4 個"
    );
    assert_eq!(gets[0].references[0], CHOICE_LABEL, "Ref0=表示ラベル");
    assert_eq!(gets[0].references[1], CANONICAL_CHOICE_ID, "Ref1=選択肢 ID");
    assert_eq!(
        gets[0].references[2..],
        choice_references()[..],
        "Ref2 以降＝付随参照列を記述順のまま"
    );

    // (3) 無印の Reference 割付（Ref0=ID のみ・Req3.2）。
    let expected_select = expected_call(events::on_choice_select(
        CANONICAL_CHOICE_ID,
        &cascading_snapshot(),
    ));
    assert_eq!(
        *gets[1], expected_select,
        "無印 GET は events 表導出（Ref0=ID のみ）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        gets[1].references,
        vec![CANONICAL_CHOICE_ID.to_string()],
        "無印の Reference は選択肢 ID 1 個のみ（ラベル・参照列を載せない）"
    );

    // (4) 最終段 204 → 解決のみ・StartTalk なし（Req4.2／5.3・裁定 3）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{CANONICAL_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "解決はちょうど 1 件で、その後に続く起動は close talk だけのはず: {commands:?}"
    );
    let started: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            TalkCommand::Start(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(
        started.len(),
        2,
        "到達 StartTalk は steady talk と close talk の 2 本のみ（選択由来 talk は起きない）: {started:?}"
    );
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は steady talk"
    );
    assert_eq!(
        started[1].script, FIXED_FAREWELL_SCRIPT,
        "2 本目は close talk（別れのスクリプト）＝選択由来 talk ではない"
    );

    // (5) 終了系列を完走した（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（close→別れ talk quit:true→Unloading{{Quit}}→Unload）"
    );
}

/// 正典形の先行段が Value を返したら後続段（無印）を**発行せず**、`ResolveChoice`→`Start` へ短絡する
/// （Req9.2(b) 後段・2.4・4.6・裁定 2）。
///
/// # 非空虚性・discriminative 性
/// [`canonical_choice_cascades_ex_then_select_and_resolves_without_start`] と**同一 ID・同一入力**で
/// 応答だけを Value に替えた対の檻である。`OnChoiceSelect` が記録列に現れないこと（＝Value 後も
/// 後続段を撃つ退行の検出）と、解決→起動が新 talk_id で起きること（＝Value 短絡が起動へ繋がること）を
/// 対で固定する。
#[test]
fn canonical_choice_ex_value_short_circuits_select() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            "OnChoiceSelectEx",
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // quit_flags: index0（steady talk）=false・index1（選択由来 talk）=true（終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade ex-value join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the choice-derived talk (quit:true)");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura ex-value join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) Value 短絡: 段列は Ex 1 段のみ（無印は発行されない・Req2.4）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceSelectEx"],
        "先行段が Value を返したら後続段（無印）を発行しないはず: {recorded:?}"
    );

    // (2) 解決→起動（新 talk_id・Req4.6／5.1）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{CANONICAL_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "Ex の Value は解決→起動をこの順で 1 度ずつ起こすはず: {commands:?}"
    );
    let started: Vec<_> = commands
        .iter()
        .filter_map(|c| match c {
            TalkCommand::Start(s) => Some(s),
            _ => None,
        })
        .collect();
    assert_eq!(
        started[1].script, FIXED_CHOICE_SCRIPT,
        "2 本目は Ex 応答スクリプト由来の talk"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "選択由来 talk は新 talk_id=2"
    );

    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（選択由来 talk quit:true→Unloading{{Quit}}→Unload）"
    );
}

// ============================================================================
// 群 3: Req4.5・DD-12 — 選択起因の失敗は終了系列へ倒れない
// ============================================================================

/// カスケード段 GET の `Failed` が `Unloading{Fault}` へ**倒れず**、204 と同じ扱いで次段へ前進し
/// 解決まで到達する（Req4.5・DD-12・C4 規則 8）。終了は後から駆動した close によってのみ起きる。
///
/// # 駆動（決定的・sleep なし）
/// 失敗注入 mock（`FailOn{ id: "OnChoiceSelectEx", kind: Shiori }`）が **最初の** Ex GET だけを落とし、
/// 以降は良性応答表へ戻る。選択待ち窓は保留 sakura で維持する（`spawn_harness_gated_failing`）。
/// 選択処理の後に `CloseRequest` を積み、保留解放で `TalkDone{Ended}` を着弾させて close 握手を走らせる。
///
/// # 非空虚性・discriminative 性（DD-12 先行アームが無ければ全て落ちる）
/// - `Failed` が横断アームで `Unloading{Fault}` へ倒れていれば、その時点で終了系列（Unload）へ入り
///   **`OnChoiceSelect` は発行されない**。本檻は段列が `[Ex, OnChoiceSelect]` であることを固定する。
/// - 同様に、Fault 終了なら `ResolveChoice` は届かず、close 握手（`OnClose`）も走らない。本檻は解決が
///   ちょうど 1 件届くことと、`OnClose` GET が選択段の**後ろ**に現れることを固定する。
/// - Unload はちょうど 1 件で記録列の末尾（＝終了は driven close 由来であり、選択失敗由来ではない）。
#[test]
fn choice_stage_failure_continues_as_204_without_fault_termination() {
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);

    let (harness, gate) = spawn_harness_gated_failing(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
        FailOn {
            id: "OnChoiceSelectEx",
            kind: FailKind::Shiori,
        },
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice");
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade choice-failed join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close, not via the choice-stage failure");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura choice-failed join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 失敗段は 204 相当で次段へ前進した（Fault へ倒れていれば無印は発行されない）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceSelectEx", "OnChoiceSelect"],
        "段 GET の Failed は 204 と同じ扱いで次段へ前進するはず（DD-12）: {recorded:?}"
    );

    // (2) 会話は止まらず選択は解決された（Req5.3）——解決はちょうど 1 件。
    let resolves = commands
        .iter()
        .filter(|c| matches!(c, TalkCommand::ResolveChoice { .. }))
        .count();
    assert_eq!(
        resolves, 1,
        "失敗を挟んでも選択解決はちょうど 1 回届くはず（Req5.3／5.4）: {commands:?}"
    );

    // (3) 終了は driven close 由来である: OnClose GET が選択段の後ろに現れ、Unload は末尾 1 件。
    let close_get_pos = recorded
        .iter()
        .position(|c| c.method == CallMethod::Get && c.id == "OnClose")
        .expect("driven close の OnClose GET が記録されるはず（Fault 終了なら現れない）");
    let select_pos = recorded
        .iter()
        .position(|c| c.method == CallMethod::Get && c.id == "OnChoiceSelect")
        .expect("無印 GET が記録されるはず");
    assert!(
        select_pos < close_get_pos,
        "close 握手は選択カスケードの後に走るはず（順序の退行検出）: {recorded:?}"
    );
    assert_eq!(
        recorded
            .iter()
            .filter(|c| c.method == CallMethod::Unload)
            .count(),
        1,
        "Unload はちょうど 1 件（選択失敗では終了しない）: {recorded:?}"
    );
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（driven close→別れ talk quit:true 由来）"
    );
}

/// 同じ選択待ち窓でも、**choice 起源でない** GET の `Failed` は従来どおり `Unloading{Fault}` へ
/// 倒れる（DD-12 免除の境界固定・Req6.1 の終了規律が非 choice 経路で不変であることの直接証拠）。
///
/// mod.rs の免除条件は「`Steady` **かつ** 選択帳簿が in-flight（`Cascading`／`TimeoutInFlight`）」で
/// あり、選択待ち（`Waiting`）中に届く失敗は免除の対象外である。本檻はその境界を統合層で固定する
/// ——`failure_test.rs`（帳簿が存在しない経路）だけでは「帳簿があるだけで免除が漏れる」退行を
/// 捕捉できないためである。
///
/// # 駆動（決定的・sleep なし）
/// active talk 窓（id=1・保留）で `ChoiceWaiting` を確立し（帳簿は `Waiting`）、その窓でマウス
/// ダブルクリックを注入する。失敗注入 mock がその GET を落とし、横断アームが `Unloading{Fault}`→
/// best-effort Unload→Stopped→StopSelf を駆動する。
///
/// # 非空虚性・discriminative 性
/// - 免除が非 choice 経路へ漏れていれば kanade は終了せず、（Close も StopSelf も送っていないため）
///   join が期限超過して panic する。
/// - Fault 経路を確かに通った直接証拠として、記録列に `OnMouseDoubleClick` GET が現れたうえで
///   Unload がちょうど 1 件末尾に現れ、close 握手（`OnClose`）は**走っていない**ことを確認する。
#[test]
fn non_choice_failure_during_choice_wait_still_faults() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0]);

    // quit_flags: steady talk（index0）は保留したまま解放しない（終了は Fault 経路が駆動する）。
    let (harness, _gate) = spawn_harness_gated_failing(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false]),
        vec![0],
        FailOn {
            id: "OnMouseDoubleClick",
            kind: FailKind::Shiori,
        },
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);
    // 選択待ち（帳簿 Waiting）の窓で非 choice GET を発行させ、その応答を失敗させる。
    harness
        .sender
        .send(KanadeMsg::Mouse(MouseInput {
            scope: 0,
            x: 10,
            y: 20,
            region: Some("Head".to_string()),
            kind: MouseEventKind::DoubleClick {
                button: MouseButton::Left,
            },
        }))
        .expect("send Mouse during choice wait");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 免除が漏れていなければ、非 choice の Failed は終了系列を駆動して join が成功する。
    join_bounded("kanade non-choice-failed join", DEFAULT_TIMEOUT, kanade).expect(
        "non-choice failure must still drive Unloading{Fault} even while a choice ledger exists",
    );
    drop(sender);
    sakura.join_bounded("mock-sakura non-choice-failed join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 失敗させた GET は現に発行された（空虚な合格でない）。
    assert!(
        recorded
            .iter()
            .any(|c| c.method == CallMethod::Get && c.id == "OnMouseDoubleClick"),
        "失敗注入対象の OnMouseDoubleClick GET が発行されているはず: {recorded:?}"
    );

    // (2) Fault 経路の best-effort Unload が末尾に 1 件（＝終了系列を正規に通った）。
    assert_eq!(
        recorded
            .iter()
            .filter(|c| c.method == CallMethod::Unload)
            .count(),
        1,
        "Fault 経路の終了では Unload が 1 度だけ記録されるはず: {recorded:?}"
    );
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（Unloading{{Fault}}→best-effort Unload→Stopped）"
    );

    // (3) close 握手は走っていない（Fault 終了であり driven close 由来ではない）。
    assert!(
        !recorded.iter().any(|c| c.id == "OnClose"),
        "Fault 終了は close 握手（OnClose）を経ないはず: {recorded:?}"
    );
}
