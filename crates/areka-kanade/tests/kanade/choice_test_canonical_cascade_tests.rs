use super::test_support::{
    CANONICAL_CHOICE_ID, CHOICE_LABEL, FIXED_CHOICE_SCRIPT, cascading_snapshot, choice_get_ids,
    choice_gets, choice_input, choice_references, command_tags, establish_choice_wait,
};
use super::{
    ChoiceResponse, CloseReason, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT,
    Fixture, Harness, KanadeConfig, KanadeMsg, QuitPolicy, TalkCommand, TalkId, events,
    expected_call, expected_unload, join_bounded, spawn_harness_gated,
};

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
