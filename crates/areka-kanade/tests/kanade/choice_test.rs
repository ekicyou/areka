//! 選択確定カスケードの統合檻（Req9.1／9.2(a)(b)(c)(d)・Req4.5・Req6.1・Req7.3〜7.5 —
//! 設計 Testing Strategy 「Integration Tests（kanade 檻 `choice_test.rs`）」の
//! (a)／(b)／(c)／(d)／DD-12 檻）。
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
//! **群 4（Req9.2(c)・Req6.1／6.2／6.4・実行状態表示）**
//! 6. [`choosing_rides_pump_status_while_waiting_then_clears_after_resolution`]: 選択待ち確立後の
//!    周期リクエストが **NOTIFY**（Ref3="0"）で `Status: talking,choosing` を帯び、解決後の周期
//!    リクエストからは `choosing` が消えて `talking` 単独へ戻る。
//!
//! **群 5（Req9.2(d)・Req7.3／7.4／7.5・タイムアウト）**
//! 7. [`choice_timeout_fires_then_204_cancels_and_rejects_later_choice`]: 注入 Tick のみで期限へ
//!    到達し、`OnChoiceTimeout`（Ref0=起動スクリプト）GET が発行され、204 で `TalkCommand::Cancel`
//!    が届き、注入した `TalkDone{Interrupted}` で `Steady{None}` へ復帰し、以降の `Choice` 注入が
//!    棄却される。
//! 8. [`choice_timeout_value_replaces_talk_via_existing_start_path`]: タイムアウト応答が Value なら
//!    既存の起動経路で置換再生される（新 talk_id・解決／解除指示は発行しない）。
//!
//! **群 6（Req9.2(e)・Req1.1／1.3／1.4・一回性と棄却）**
//! 9. [`one_choice_injection_yields_a_single_cascade_and_later_injections_are_rejected`]: 1 回の
//!    選択確定が **高々 1 カスケード・高々 1 選択解決・高々 1 起動要求**しか起こさず、解決後に
//!    到着した同一 ID の遅延注入がイベントも指示も生まないこと。
//! 10. [`choice_outside_the_candidate_set_is_rejected_and_keeps_the_wait_open`]: 候補集合に無い ID の
//!    注入が何も起こさず、**選択待ちを閉じない**（直後の候補内 ID が正常に通る）こと。
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
//! # 期限（deadline）の作り方——実時刻を一切読まない
//! 注入する `ChoiceWaiting` は `timeout_directive_secs: None`（＝既定値
//! [`CHOICE_TIMEOUT_DEFAULT_MS`] へ委譲）で、起点は `display_end = `[`CHOICE_DISPLAY_END_MS`] ゆえ
//! 期限は [`CHOICE_DEADLINE_MS`] である。したがって**注入 Tick の `now` を選ぶだけ**で期限の手前／
//! 到達を作り分けられる（実時間待機も `Instant` 読み取りも一切ない・Req9.1／7.3）。
//! 群 1〜4 は期限より手前の Tick しか注入しないためタイムアウト経路へは構造上到達せず、群 5 が
//! 期限手前 → 期限到達の 2 点を注入してタイムアウトのみを踏む。

use std::sync::mpsc::Sender;

use areka_kanade::{
    ChoiceInput, CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, MouseButton,
    MouseEventKind, MouseInput, TalkCommand, TalkDone, TalkEndReason, TalkId, events,
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

/// 選択待ちの候補集合に**含めない** ID（群 6 の候補照合の観測面・Req1.4）。
///
/// 正典形（`On` 始まりでない）を選ぶ——受理されてしまう退行では `OnChoiceSelectEx` が発行され、
/// [`choice_get_ids`] の突合で直ちに落ちる（任意名形だと GET の id が檻の抽出述語から外れ、
/// 退行が段列の比較をすり抜ける）。
const OUT_OF_CANDIDATE_CHOICE_ID: &str = "候補外選択肢";

/// 選択肢の表示ラベル（`OnChoiceSelectEx` Ref0 の供給源）。
const CHOICE_LABEL: &str = "おしゃべり頻度";

/// 選択由来の応答スクリプト（steady／close の fixture 語彙と別文字列にし、到達 StartTalk の
/// 由来を script で識別する）。
const FIXED_CHOICE_SCRIPT: &str = r"\0\s[0]頻度を変えるね\e";

/// 選択確定に付随する参照列（記述順を保存することの観測対象）。
fn choice_references() -> Vec<String> {
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
const CHOICE_DEADLINE_MS: u64 = CHOICE_DISPLAY_END_MS + CHOICE_TIMEOUT_DEFAULT_MS;

/// カスケード段・タイムアウト GET が帯びる運行状態（active talk＋選択待ち継続中＝`talking,choosing`）。
///
/// C5 の `choice_active` の源は帳簿の 3 段すべて（`Waiting`／`Cascading`／`TimeoutInFlight`）ゆえ、
/// カスケード段 GET と `OnChoiceTimeout` GET は同一の複合スナップショットを帯びる（裁定 6）。
fn cascading_snapshot() -> ExecutionSnapshot {
    ExecutionSnapshot {
        talk_active: true,
        choice_active: true,
    }
}

/// active talk のみ（選択待ちは終了済み）の運行状態＝`talking` 単独。
fn talking_only_snapshot() -> ExecutionSnapshot {
    ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
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

/// 記録列から周期リクエスト（`OnSecondChange`）のみを処理順に抽出する（GET・NOTIFY 双方）。
fn pumps(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded
        .iter()
        .filter(|c| c.id == "OnSecondChange")
        .collect()
}

/// 記録列における最初の一致位置（処理順の前後関係を突合するための索引）。
fn position_of(recorded: &[RecordedCall], method: CallMethod, id: &str) -> usize {
    recorded
        .iter()
        .position(|c| c.method == method && c.id == id)
        .unwrap_or_else(|| panic!("{method:?} {id} が記録されているはず: {recorded:?}"))
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

// ============================================================================
// 群 4: Req9.2(c) — 選択待ち中の周期リクエストに `choosing` が載り、解決後に消える
// ============================================================================

/// 選択待ち確立後の周期リクエストが **NOTIFY**（Ref3="0"）で `Status: talking,choosing` を帯び、
/// 選択解決後の周期リクエストからは `choosing` が消えて `talking` 単独へ戻る
/// （Req9.2(c)・6.1／6.2／6.4・C5・裁定 6）。
///
/// # 駆動（決定的・sleep なし）
/// 挨拶なし boot →`Steady{None}` 直行 → Tick1（`OnSecondChange` GET Value）で steady talk（id=1）を
/// 起こし、保留ハーネス（`hold_indices=[0]`）でその `TalkDone` を park して active talk 窓を保つ。
/// `ChoiceWaiting` を確立してから **Tick2** を注入し（選択待ち中の pump）、`Choice` を注入して
/// 正典形カスケードを 204／204 で走らせ（＝解決のみ・帳簿消去）、続けて **Tick3** を注入する
/// （解決後の pump）。終了は `CloseRequest`＋保留解放（`TalkDone{Ended}`→close 握手→別れ talk
/// quit:true）で駆動する。注入 Tick はいずれも期限 [`CHOICE_DEADLINE_MS`] より手前ゆえ、
/// タイムアウト経路は踏まない。
///
/// # 非空虚性・discriminative 性
/// - 周期リクエストのうち **NOTIFY はちょうど 2 件**（Tick2／Tick3）であり、それぞれの `status` を
///   **wire 実値**（`"talking,choosing"` ／ `"talking"`）で突合する。`choosing` の導出が入って
///   いなければ 1 件目が `"talking"` になり落ち、解決時の帳簿消去が漏れていれば 2 件目が
///   `"talking,choosing"` のまま残って落ちる（両方向の退行を弁別する）。
/// - 連結順序（正典順 `talking,choosing`）も実値突合に含まれる——順序が逆（`choosing,talking`）に
///   なる退行はここで落ちる（Req6.3 の既存規律が複合値でも保たれること）。
/// - 両件が **NOTIFY・Ref3="0"** であること（GET でないこと）を明示確認する＝選択待ち中も slot
///   占有継続として扱われ、応答スクリプトを運べない型で送出される（Req6.4／6.5 の構造充足）。
/// - 記録位置の前後（Tick2 の NOTIFY < カスケード段 GET < Tick3 の NOTIFY）を突合し、2 件目が
///   確かに**解決後**の pump であることを固定する（空虚な合格を防ぐ）。
#[test]
fn choosing_rides_pump_status_while_waiting_then_clears_after_resolution() {
    // quitting: close 握手が別れ talk（Value）を返す。選択由来 GET は未注入＝両段とも 204
    // （＝解決のみ・StartTalk なし）で、解決後の pump を同一 talk 窓のまま観測できる。
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
    // Tick2: 選択待ち中の周期リクエスト（期限 CHOICE_DEADLINE_MS より手前）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(2_000),
        })
        .expect("send Tick 2 (choosing)");
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice");
    // Tick3: 解決後の周期リクエスト（同じ active talk 窓・choosing だけが消えているはず）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(3_000),
        })
        .expect("send Tick 3 (resolved)");
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

    join_bounded("kanade choosing-status join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close (farewell talk quit:true)");
    drop(sender);
    sakura.join_bounded("mock-sakura choosing-status join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 周期リクエストのうち NOTIFY はちょうど 2 件（Tick2／Tick3）——active talk 窓が両 Tick で
    //     維持されている（GET へ落ちていない）ことの直接確認。
    let notifies: Vec<&RecordedCall> = pumps(&recorded)
        .into_iter()
        .filter(|c| c.method == CallMethod::Notify)
        .collect();
    assert_eq!(
        notifies.len(),
        2,
        "active talk 窓で注入した 2 本の Tick は NOTIFY pump を 1 件ずつ発行するはず: {recorded:?}"
    );

    // (2) 選択待ち中の pump: NOTIFY・Ref3="0"・Status は複合値 `talking,choosing`（Req6.1／6.4・裁定 6）。
    let expected_choosing = expected_call(events::on_second_change(
        MonotonicMs(2_000),
        &cascading_snapshot(),
    ));
    assert_eq!(
        *notifies[0], expected_choosing,
        "選択待ち中の pump は events 表導出（NOTIFY・Ref3=0・複合 Status）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        notifies[0].status,
        Some("talking,choosing".to_string()),
        "選択待ち中の Status wire は正典順の複合値 `talking,choosing`（Req6.1／6.3・裁定 6）"
    );
    assert_eq!(
        notifies[0].references[3], "0",
        "選択待ち中も slot 占有継続＝Ref3 は \"0\"（応答スクリプトを再生しない・Req6.4）"
    );

    // (3) 解決後の pump: 同じ active talk 窓のまま `choosing` だけが消える（Req6.2）。
    let expected_resolved = expected_call(events::on_second_change(
        MonotonicMs(3_000),
        &talking_only_snapshot(),
    ));
    assert_eq!(
        *notifies[1], expected_resolved,
        "解決後の pump は events 表導出（NOTIFY・Ref3=0・talking 単独）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        notifies[1].status,
        Some("talking".to_string()),
        "解決後の Status wire から choosing が消え talking 単独へ戻るはず（Req6.2）"
    );
    assert_eq!(
        notifies[1].references[3], "0",
        "解決後も同じ active talk 窓＝Ref3 は \"0\" のまま（talk 軸は不変）"
    );

    // (4) 非空虚性: 2 件目が確かに「解決後」の pump である（カスケード段が両 NOTIFY の間にある）。
    let first_notify_pos = recorded
        .iter()
        .position(|c| *c == expected_choosing)
        .expect("選択待ち中の NOTIFY が記録されているはず");
    let last_notify_pos = recorded
        .iter()
        .position(|c| *c == expected_resolved)
        .expect("解決後の NOTIFY が記録されているはず");
    let ex_pos = position_of(&recorded, CallMethod::Get, "OnChoiceSelectEx");
    let select_pos = position_of(&recorded, CallMethod::Get, "OnChoiceSelect");
    assert!(
        first_notify_pos < ex_pos && ex_pos < select_pos && select_pos < last_notify_pos,
        "記録順は [choosing pump] → Ex → 無印 → [解決後 pump] のはず（2 件目が解決後である証拠）: {recorded:?}"
    );

    // (5) 段列は正典形 2 段のみ（タイムアウト経路を踏んでいないことの裏取り）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceSelectEx", "OnChoiceSelect"],
        "本檻の Tick は期限手前のみ＝OnChoiceTimeout は発行されないはず: {recorded:?}"
    );

    // (6) 終了系列を完走した（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（driven close→別れ talk quit:true 由来）"
    );
}

// ============================================================================
// 群 5: Req9.2(d) — タイムアウト（期限到達・204 解除・以降棄却／Value 置換再生）
// ============================================================================

/// 注入 Tick のみで期限へ到達し、`OnChoiceTimeout`（Ref0=起動スクリプト）GET が発行され、204 で
/// `TalkCommand::CancelChoice` が届き、`TalkDone{Interrupted}` の注入で `Steady{None}` へ復帰し、
/// **解除後に到着する `Choice` が棄却される**（Req9.2(d)・7.3／7.5・C4 規則 5／6・DD-11・F3）。
///
/// # 駆動（決定的・sleep なし・実時刻を読まない）
/// active talk 窓（id=1・保留）で `ChoiceWaiting` を確立すると期限は [`CHOICE_DEADLINE_MS`] になる。
/// 注入する Tick は次の 4 本だけで、いずれも `now` は檻が与える論理値である:
///
/// 1. `Tick(CHOICE_DISPLAY_END_MS)`（[`establish_choice_wait`] 内）: steady talk を起こす。
/// 2. `Tick(CHOICE_DEADLINE_MS - 1_000)`: **期限手前**——タイムアウトは発火せず通常 pump（NOTIFY）。
/// 3. `Tick(CHOICE_DEADLINE_MS)`: **期限到達**——`OnChoiceTimeout` GET を発行し、この周期は pump を
///    発行しない（C4 規則 5）。mock は当該 id へ未注入ゆえ 204 を返し、`CancelChoice` が発行される。
/// 4. `Tick(CHOICE_DEADLINE_MS + 1_000)`: `TalkDone{Interrupted}` 注入後の pump——`Steady{None}` へ
///    復帰していれば **GET**（Ref3="1"・Status ヘッダなし）になる。
///
/// mock sakura は再生層を持たず `CancelChoice` を記録するだけなので、Close funnel の帰結である
/// `TalkDone{Interrupted}` は檻が直接注入する（設計 Testing Strategy (d) の明文どおり）。終了は
/// `Steady{None}` 復帰後の `CloseRequest`（→別れ talk quit:true）で駆動する。保留は解放しない。
///
/// # 非空虚性・discriminative 性
/// - **期限の両側**を注入して既定値 [`CHOICE_TIMEOUT_DEFAULT_MS`] そのものを固定する:
///   手前（2）では発火せず `talking,choosing` の NOTIFY pump が出る・到達（3）で初めて発火する。
///   既定値が大きくなれば (3) でも発火せず `OnChoiceTimeout` が現れない。小さくなれば (2) で
///   発火し、**記録順が `OnChoiceTimeout` → NOTIFY pump へ逆転**したうえ (2) の pump から
///   `choosing` が消えるため、順序突合と Status 突合の両方で落ちる（**両方向**を弁別する。
///   単に「段列と pump 件数」を数えるだけでは早期発火を見逃す——実測で確認済み）。
/// - `OnChoiceTimeout` GET を events 表導出と完全一致で突合し、Ref0 が **当該トークの起動
///   スクリプト**（`FIXED_STEADY_SCRIPT`＝ActiveTalk.script・DD-10）であることを実値で確認する。
///   Ref0 に選択肢 ID やラベルを載せる退行はここで落ちる。
/// - `OnSecondChange` の総数が 3 件（GET 2・NOTIFY 1）であること＝**発火した周期は pump を出して
///   いない**こと（C4 規則 5）。pump を止め損ねる退行は 4 件になって落ちる。
/// - `TalkCommand` 到着順が `Start(1)`→`Cancel(1)`→`Start(2)` であること。`ResolveChoice` が混じる
///   （タイムアウトを解決扱いする退行）／`Cancel` が落ちる（Req7.5 の破れ）のいずれでも落ちる。
/// - 復帰後の pump が `Steady{None}` の GET（Ref3="1"・Status ヘッダなし）であること＝
///   `TalkDone{Interrupted}` が非 quit 扱いで定常復帰する正規到達点（DD-11）を通ったこと。
/// - 選択由来 GET が `OnChoiceTimeout` **ただ 1 件**であること＝解除後に注入した `Choice` 2 本が
///   いずれもカスケードを起こしていない（棄却された）こと。棄却が漏れれば Ex／無印が現れて落ちる。
///   2 本の注入点は意図的に分けてある——1 本目は**解除直後（現行トークはまだ active）**であり
///   解除側の帳簿消去だけで棄却されねばならず、2 本目は `Steady{None}` 復帰後の遅延通知である。
///   1 本目が無いと、解除で帳簿を消し損ねる退行を `TalkDone{Interrupted}` の掃除（C4 規則 7）が
///   覆い隠して素通りする（実測で確認済み）。
#[test]
fn choice_timeout_fires_then_204_cancels_and_rejects_later_choice() {
    // quitting: 復帰後の driven close が別れ talk（Value）を返す。OnChoiceTimeout は未注入＝204。
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);

    // hold_indices=[0]: steady talk（id=1）の TalkDone を park（解放しない——終了は driven close）。
    // quit_flags: index0（steady talk）=false・index1（close talk）=true（終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);

    // (2) 期限手前の Tick——発火しない（既定値の下限を固定する）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS - 1_000),
        })
        .expect("send Tick just before the deadline");
    // (3) 期限到達の Tick——OnChoiceTimeout GET を発行し pump は出さない。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS),
        })
        .expect("send Tick at the deadline");
    // 解除直後（現行トークはまだ active）の選択確定——解除側の掃除点だけで棄却されるはず（Req7.5）。
    // この 1 本目は `TalkDone{Interrupted}` の掃除（C4 規則 7）に**先行する**ため、解除で帳簿を
    // 消し損ねる退行を単独で弁別する（後述の 2 本目だけでは 2 つの掃除点が互いを覆い隠す）。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice right after cancellation (talk still active)");
    // Close funnel の帰結（dispatcher が Close を転送し talk が返す通知）を檻が直接注入する。
    harness
        .sender
        .send(KanadeMsg::TalkDone(TalkDone {
            talk_id: TalkId(1),
            reason: TalkEndReason::Interrupted,
        }))
        .expect("send TalkDone{Interrupted} (Close funnel の完了通知)");
    // (4) 復帰後の pump——Steady{None} なら GET（Ref3=1）になる。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS + 1_000),
        })
        .expect("send Tick after the interrupted recovery");
    // 2 本目の選択確定——`Steady{None}` 復帰後に到着する遅延通知（同じく棄却されるはず・Req7.5）。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(CANONICAL_CHOICE_ID)))
        .expect("send Choice after the interrupted recovery");
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade choice-timeout join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close after the interrupted recovery");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura choice-timeout join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 選択由来 GET は OnChoiceTimeout ただ 1 件（期限手前で非発火・解除後の Choice は棄却）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceTimeout"],
        "期限到達でのみタイムアウトが発火し、解除後の Choice はカスケードを起こさないはず: {recorded:?}"
    );

    // (2) OnChoiceTimeout の layout（Ref0=当該トークの起動スクリプト・Status は複合値・Req3.4／DD-10）。
    let expected_timeout = expected_call(events::on_choice_timeout(
        FIXED_STEADY_SCRIPT,
        &cascading_snapshot(),
    ));
    let timeout_get = choice_gets(&recorded);
    assert_eq!(
        *timeout_get[0], expected_timeout,
        "OnChoiceTimeout は events 表導出（Ref0=起動スクリプト・Status）と一致するはず: {recorded:?}"
    );
    assert_eq!(
        timeout_get[0].references,
        vec![FIXED_STEADY_SCRIPT.to_string()],
        "Ref0 は当該トークの起動スクリプトそのもの（ActiveTalk.script・DD-10）"
    );
    assert_eq!(
        timeout_get[0].status,
        Some("talking,choosing".to_string()),
        "タイムアウト GET も選択待ち継続中（TimeoutInFlight）＝複合 Status を帯びる（裁定 6）"
    );

    // (3) 発火した周期は pump を出していない（C4 規則 5）——OnSecondChange は GET 2・NOTIFY 1 の計 3 件。
    let pump_calls = pumps(&recorded);
    let pump_methods: Vec<&CallMethod> = pump_calls.iter().map(|c| &c.method).collect();
    assert_eq!(
        pump_methods,
        vec![
            &CallMethod::Get,    // Tick(display_end): Steady{None} の pump 問い合わせ
            &CallMethod::Notify, // 期限手前の Tick: active talk 窓の NOTIFY
            &CallMethod::Get,    // 復帰後の Tick: Steady{None} の pump 問い合わせ
        ],
        "期限到達の Tick は pump を発行しない（規則 5）＝周期リクエストは 3 件のみのはず: {recorded:?}"
    );

    // (3') 既定値そのものの固定（Req7.8）: 期限**手前**の Tick では発火していない。
    //      観測面は 2 点——(i) その pump がまだ `talking,choosing` を帯びる（帳簿が Waiting のまま）、
    //      (ii) 記録順が [手前の NOTIFY pump] → [OnChoiceTimeout] である。既定値が
    //      CHOICE_TIMEOUT_DEFAULT_MS より小さければ手前の Tick で発火し、この 2 点が両方とも破れる。
    assert_eq!(
        pump_calls[1].status,
        Some("talking,choosing".to_string()),
        "期限手前の pump はまだ選択待ち継続中＝複合 Status を帯びるはず（既定値が短ければ落ちる）: {recorded:?}"
    );
    let before_deadline_pump_pos = recorded
        .iter()
        .position(|c| *c == *pump_calls[1])
        .expect("期限手前の NOTIFY pump が記録されているはず");
    let timeout_get_pos = position_of(&recorded, CallMethod::Get, "OnChoiceTimeout");
    assert!(
        before_deadline_pump_pos < timeout_get_pos,
        "タイムアウトは期限手前の Tick より後（期限到達の Tick）で初めて発火するはず: {recorded:?}"
    );

    // (4) 204 で選択待ちが解除された（Cancel が届き、Resolve は混じらない・Req7.5・F3）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            "Cancel(1)".to_string(),
            "Start(2)".to_string(),
        ],
        "タイムアウト 204 は解決ではなく解除指示を出すはず（その後の起動は close talk のみ）: {commands:?}"
    );

    // (5) `TalkDone{Interrupted}` で `Steady{None}` へ復帰した——復帰後の pump が GET（Ref3=1・
    //     Status ヘッダなし）であることが観測面（DD-11 の正規到達点）。
    let expected_resumed = expected_call(events::on_second_change(
        MonotonicMs(CHOICE_DEADLINE_MS + 1_000),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(
        *pump_calls[2], expected_resumed,
        "解除後の pump は Steady{{None}} の GET（Ref3=1・Status ヘッダなし）のはず: {recorded:?}"
    );
    assert_eq!(
        pump_calls[2].references[3], "1",
        "Steady{{None}} 復帰の Ref3 は \"1\"（再生可能＝応答スクリプトを再生できる）"
    );
    assert_eq!(
        pump_calls[2].status, None,
        "復帰後は実行状態が空＝Status ヘッダ行そのものが省略される（Req6.3 の既存規律）"
    );

    // (6) 終了は driven close 由来（末尾 Unload・タイムアウトはゴーストを終了させない）。
    let close_pos = position_of(&recorded, CallMethod::Get, "OnClose");
    assert!(
        timeout_get_pos < close_pos,
        "close 握手はタイムアウト解除の後に走るはず（順序の退行検出）: {recorded:?}"
    );
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（driven close→別れ talk quit:true 由来）"
    );
}

/// タイムアウト応答が Value なら **既存の起動経路**で置換再生される（新 talk_id・Req9.2(d) 後段・
/// 7.4・C4 規則 6・F3）。
///
/// # 非空虚性・discriminative 性
/// [`choice_timeout_fires_then_204_cancels_and_rejects_later_choice`] と**同一の期限・同一の注入
/// Tick** で応答だけを Value に替えた対の檻である。
/// - `TalkCommand` 到着順が `Start(1)`→`Start(2)` であること＝置換起動は解決指示（`Resolve`）も
///   解除指示（`Cancel`）も伴わない（F3 の Value 分岐）。どちらかが混じれば落ちる。
/// - 起動された talk が **新 talk_id=2**・**タイムアウト応答のスクリプト**であること＝旧 id の
///   再利用や応答の取り違えを検出する。
/// - 選択由来 GET が `OnChoiceTimeout` 1 件のみであること＝Value 応答が更なる段を撃たないこと。
#[test]
fn choice_timeout_value_replaces_talk_via_existing_start_path() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            "OnChoiceTimeout",
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // quit_flags: index0（steady talk・保留）=false・index1（タイムアウト由来 talk）=true（終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[CANONICAL_CHOICE_ID]);
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(CHOICE_DEADLINE_MS),
        })
        .expect("send Tick at the deadline");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade timeout-value join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the timeout-derived talk (quit:true)");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura timeout-value join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 段列はタイムアウト 1 件のみ（Value が更なる段を撃たない）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceTimeout"],
        "タイムアウト GET は 1 件のみのはず: {recorded:?}"
    );

    // (2) 置換起動は解決も解除も伴わない（F3 の Value 分岐・Req7.4）。
    assert_eq!(
        command_tags(&commands),
        vec!["Start(1)".to_string(), "Start(2)".to_string()],
        "タイムアウト Value は置換起動のみを起こすはず（Resolve／Cancel を伴わない）: {commands:?}"
    );

    // (3) 起動は新 talk_id・タイムアウト応答のスクリプト（既存の起動棚に載った）。
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
        "2 本目はタイムアウト応答のスクリプト由来の talk"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "置換再生は新 talk_id=2（旧 id=1 を再利用しない・Req4.1）"
    );

    // (4) 終了系列を完走した（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（タイムアウト由来 talk quit:true→Unloading{{Quit}}→Unload）"
    );
}

// ============================================================================
// 群 6: Req9.2(e) — 一回性と棄却（統合経路でしか観測できない分）
// ============================================================================
//
// # 本群が担う範囲と、担わない範囲（設計 Testing Strategy 「(e) 補足（in-flight 分岐の檻方式）」）
// カスケードは kanade drive ループの同期往復で **1 メッセージ処理内に完結する**。したがって
// `ChoicePhase::Cascading`／`TimeoutInFlight` の最中に別の選択確定を**注入して**割り込ませることは
// 統合経路では構造上できない——段の進行中の二重確定棄却（`choice_rejected_busy`）と 1 世代 stale
// 防御（`talk_done_stale_choice`）は、状態を直接構成する `step()` 直呼びの純関数檻が担当する
// （`schedule::steady::tests::choice_during_cascade_or_timeout_is_rejected_as_busy`・
// `schedule::tests::stale_choice_talk_done_is_demoted_to_info_and_keeps_state`。ログ語彙は
// `schedule::log_firing_tests` が固定する）。
//
// 本群が担うのは、逆に**純関数檻では観測できない**面である: 実際の drive ループ・実際の
// `TalkCommand` チャンネル・実際の mock SHIORI 往復を通した end-to-end の件数（1 注入が発行した
// GET・`ResolveChoice`・`StartTalk` の実数）と、解決で帳簿が消えた**後**に到着した注入が
// 配送経路のどこからも副作用を出さないこと。
//
// # 決定性（Req9.1）——注入順序の確定と、記録読み出しの同期
// 決定性は**二つ**の面で要る。片方だけでは非決定が残る。
//
// **(i) 注入順序**: 群 1〜5 と違い、本群は `hold_indices=[0, 1]`（steady talk と選択由来 talk の
// **両方**の `TalkDone` を park）で駆動する。mock sakura が自発的に kanade inbox へ送るメッセージは
// `TalkDone` だけなので、両方を park すれば注入列の途中に非同期メッセージが一切割り込まない
// ——「遅延注入は必ずカスケード解決の後に処理される」ことが mpsc の FIFO だけで確定する
// （即応 sink だと選択由来 talk の `TalkDone` が遅延注入を追い越し得て、棄却が
// `choice_rejected_no_wait` ではなく終了系列の入力無視で起きたのか区別できなくなる）。
//
// **(ii) 記録の読み出し**: kanade の停止は `TalkCommand` を mock が**消費し終えた**ことを意味しない。
// 本群は `ForceQuit` で終了するため（下記）、`TalkDone` の往復という暗黙の同期が働かず、
// `MockSakura::commands()` の並行読みでは記録列が空のまま読まれ得る（実測: 群 6 の 2 檻を
// 100 回実行して 11 回、退行と無関係に `left: []` で FAILED した）。したがって本群は
// [`MockSakura::join_bounded_then_commands`]（recv ループの join 完了後に読む変種）を使う。
// 群 1〜5 は quit フラグ付き `TalkDone` の往復が mock の消費を強制するため `commands()` のままでよい。
//
// # 終了の駆動は `ForceQuit`——**檻の弁別力を退行時にも保つため**
// 群 1〜5 は「末尾 talk の quit:true」で終了を駆動するが、本群はそれを採らない。棄却が壊れる退行では
// **起動される talk の本数そのものが変わる**ため、`QuitPolicy::PerTalk` の index と quit フラグの
// 対応がずれて終了が駆動されず、檻が「期限付き join の失敗」（＝どの表明が破れたか分からない形）で
// しか落ちなくなる（実測で確認済み）。`ForceQuit` は quit ゲートを迂回して終了系列へ直行する
// （**close spec の DD-10**＝本 spec の DD-10（`ActiveTalk.script`→`OnChoiceTimeout` Ref0）とは
// 別物・`close_test.rs` の既存イディオム）ので、talk の本数に関わらず必ず join が成功し、
// **段列と `TalkCommand` の突合そのものが退行を名指しする**。ただしこの「名指しする」性質は
// 上記 (ii) の読み出し同期が前提である——同期を欠くと突合は退行と無関係な空列を報告し得る。
//
// 保留 talk は kanade 停止**後**に `release_all` で解放する。送り先の kanade は既に居らず送出は
// no-op で、観測対象の運行には影響しない——停止**前**に解放すると park した `TalkDone` が注入列へ
// 割り込み (i) が壊れる。`spawn_mock_sakura_gated` の「Sender drop より前に呼ぶ」契約は kanade 停止
// 後には timing 要件としての意味を失っており（実際の解放は `recv_closed` 安全弁が担う）、本檻は
// 決定性を優先して停止後に呼ぶ。

/// 1 回の選択確定が **高々 1 カスケード・高々 1 選択解決・高々 1 起動要求**しか起こさず、解決後に
/// 到着した遅延注入が何も起こさない（Req9.2(e)・1.1・1.3・4.6・5.4・C4 規則 1／7）。
///
/// # 駆動（決定的・sleep なし）
/// active talk 窓（id=1・保留）で `ChoiceWaiting{candidates:[NAMED]}` を確立し、同じ ID を **3 回**
/// 注入する。1 本目だけが受理されて任意名 1 段のカスケード（Value）→ `ResolveChoice`→`Start(2)` を
/// 起こし、2・3 本目は帳簿が消えた後に到着する遅延通知として棄却される。選択由来 talk（id=2）も
/// park するため、この 3 本は必ずこの順で連続処理される。終了は `ForceQuit` で駆動する。
///
/// # 非空虚性・discriminative 性
/// - 選択由来 GET が **ちょうど 1 件**であること。遅延注入を受理する退行（帳簿の消去漏れ・
///   `choice_rejected_no_wait` アームの欠落）では 2 件目・3 件目の任意名 GET が現れて落ちる。
///   実測: `on_cascade_reply` の Value 分岐で帳簿を消さず新 talk_id へ引き継ぐ変異を入れると、
///   段列が `[NAMED, NAMED, NAMED]` になって本表明が落ちる（もう一方の群 6 檻は緑のまま）。
/// - `TalkCommand` 到着順が `Start(1)`→`Resolve(1,NAMED)`→`Start(2)` であること。`ResolveChoice`
///   が 2 件以上になる（解決の重複発行・Req5.4 の破れ）／`CancelChoice` が混じる／選択由来
///   `StartTalk` が 2 本になる（Req4.6 の破れ）のいずれの退行でもタグ列が食い違って落ちる。
/// - 到達 `StartTalk` が steady と選択由来の 2 本ちょうどであること＝遅延注入が追加の talk を
///   起こしていないこと。
#[test]
fn one_choice_injection_yields_a_single_cascade_and_later_injections_are_rejected() {
    // 任意名 GET は Value（1 段で短絡）。終了は ForceQuit ゆえ close 応答の仕込みは要らない。
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            NAMED_CHOICE_ID,
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // hold_indices=[0, 1]: steady talk（id=1）と選択由来 talk（id=2）の TalkDone を両方 park。
    // quit フラグは終了駆動に使わない（ForceQuit が迂回する）ため一律 false。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::Fixed(false),
        vec![0, 1],
    );

    establish_choice_wait(&harness.sender, &[NAMED_CHOICE_ID]);
    // 1 本目——受理される唯一の注入。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)))
        .expect("send Choice (accepted)");
    // 2・3 本目——解決で帳簿が消えた後に届く遅延通知（いずれも棄却されるはず・Req1.3）。
    for nth in ["send Choice (late #1)", "send Choice (late #2)"] {
        harness
            .sender
            .send(KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)))
            .expect(nth);
    }
    harness
        .sender
        .send(KanadeMsg::ForceQuit {
            reason: CloseReason::User,
        })
        .expect("send ForceQuit");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade choice-once join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates directly on ForceQuit (close spec DD-10)");
    drop(sender);
    // 保留 talk の解放は kanade 停止**後**に行う（観測対象の運行には影響しない後始末——
    // 送り先が既に居ないため送出は no-op である）。解放を停止前に置くと park していた
    // `TalkDone` が注入列へ割り込み、群 6 の前提（非同期メッセージを走らせない）が壊れる。
    gate.release_all();
    // 記録は **join 完了後**に読む——`ForceQuit` 終了は mock sakura を経由しないため、
    // `commands()` の並行読みでは記録前のスナップショットを掴み得る（群 6 ヘッダ参照）。
    let commands =
        sakura.join_bounded_then_commands("mock-sakura choice-once join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) カスケードは高々 1 回（3 注入でも段列は任意名 1 件のみ・Req1.1）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec![NAMED_CHOICE_ID],
        "3 回の同一注入でカスケードは 1 回だけ走るはず（解決後の遅延注入は棄却）: {recorded:?}"
    );

    // (2) 解決も起動も高々 1 つ（順序込み・Req5.4／4.6）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{NAMED_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "選択解決は 1 件・選択由来の起動は 1 本のみのはず: {commands:?}"
    );

    // (3) 到達 StartTalk の内訳——選択由来は 2 本目ただ 1 本（遅延注入は talk を起こさない）。
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
        "起動は steady と選択由来の 2 本ちょうどのはず: {started:?}"
    );
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は steady talk"
    );
    assert_eq!(
        started[1].script, FIXED_CHOICE_SCRIPT,
        "2 本目は選択由来の応答スクリプト"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "選択由来 talk は新 talk_id=2（旧 id=1 を再利用しない）"
    );

    // (4) 終了系列を完走した（末尾 Unload）＝遅延注入の棄却後も状態整合を保っている。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（ForceQuit→終了系列直行・close spec の DD-10）"
    );
}

/// 候補集合に無い ID の注入は何も起こさず、**選択待ちを閉じない**（Req9.2(e) 後段・1.4・C4 規則 1）。
///
/// # 駆動（決定的・sleep なし）
/// [`one_choice_injection_yields_a_single_cascade_and_later_injections_are_rejected`] と同じ保留構成で、
/// 候補集合 `[NAMED]` に対し **候補外 ID → 候補内 ID** の順に注入する。
///
/// # 非空虚性・discriminative 性
/// - 候補外 ID の注入が **段を 1 つも発行しない**こと。受理する退行では正典形カスケードの
///   `OnChoiceSelectEx` が段列の先頭に現れて落ちる（そのために候補外 ID は正典形を選んである）。
///   加えて記録列全体に当該 ID の呼び出しが無いことも直接表明する。実測: `on_choice` の候補照合を
///   無効化する変異を入れると段列が `[OnChoiceSelectEx, OnChoiceSelect]` になって落ちる
///   （もう一方の群 6 檻は緑のまま）。
/// - 続く**候補内 ID が正常に受理される**こと＝棄却が選択待ちを閉じていない（Req1.4「選択待ち状態を
///   変更しない」の観測面）。棄却のついでに帳簿を落とす退行では段列が空になり、`Resolve` も消えて落ちる。
///   件数を数えるだけの檻ではこの「閉じてしまう」退行を弁別できない——後続注入の成功が要る。
#[test]
fn choice_outside_the_candidate_set_is_rejected_and_keeps_the_wait_open() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_choice_response(
            NAMED_CHOICE_ID,
            ChoiceResponse::Script(FIXED_CHOICE_SCRIPT.to_string()),
        );

    // 保留構成・終了駆動は群 6 共通（hold_indices=[0, 1]＋ForceQuit）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::Fixed(false),
        vec![0, 1],
    );

    establish_choice_wait(&harness.sender, &[NAMED_CHOICE_ID]);
    // (1) 候補外 ID——棄却されるはず（Req1.4）。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(OUT_OF_CANDIDATE_CHOICE_ID)))
        .expect("send Choice (out of candidates)");
    // (2) 候補内 ID——(1) が選択待ちを閉じていなければ受理されるはず。
    harness
        .sender
        .send(KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)))
        .expect("send Choice (in candidates)");
    harness
        .sender
        .send(KanadeMsg::ForceQuit {
            reason: CloseReason::User,
        })
        .expect("send ForceQuit");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade unknown-id join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates directly on ForceQuit (close spec DD-10)");
    drop(sender);
    // 後始末・記録の読み出し同期は群 6 共通（もう一方の檻と同一の手順）。
    gate.release_all();
    let commands =
        sakura.join_bounded_then_commands("mock-sakura unknown-id join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 候補外 ID は段を 1 つも発行していない（段列は候補内 ID の 1 件のみ）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec![NAMED_CHOICE_ID],
        "候補外 ID はカスケードを起こさず、候補内 ID のみが 1 段発行するはず: {recorded:?}"
    );
    assert!(
        !recorded
            .iter()
            .any(|c| c.id == OUT_OF_CANDIDATE_CHOICE_ID),
        "候補外 ID を任意名イベントとして発火する退行はここで落ちる: {recorded:?}"
    );

    // (2) 棄却は選択待ちを閉じていない——後続の候補内 ID が解決・起動まで到達している（Req1.4）。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{NAMED_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "候補外 ID の棄却後も選択待ちは生きており、候補内 ID が解決と起動を起こすはず: {commands:?}"
    );

    // (3) 終了系列を完走した（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（ForceQuit→終了系列直行・close spec の DD-10）"
    );
}
