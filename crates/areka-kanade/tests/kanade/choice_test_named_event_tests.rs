use super::test_support::{
    FIXED_CHOICE_SCRIPT, NAMED_CHOICE_ID, cascading_snapshot, choice_get_ids, choice_gets,
    choice_input, choice_references, command_tags, establish_choice_wait,
};
use super::{
    Driven, FIXED_STEADY_SCRIPT, Fixture, KanadeConfig, KanadeMsg, QuitPolicy, events,
    expected_call, expected_unload, send, spawn_harness_gated,
};

// ============================================================================
// 群 1: `On` 始まり ID は任意名イベント 1 段のみ
// ============================================================================

/// `On` 始まり選択肢 ID の確定が **任意名イベントだけ**を発行し、その Value 応答から
/// `ResolveChoice`→`Start`（新 talk_id）が起きる。
///
/// # 非空虚性・discriminative 性
/// - 任意名 GET が **events 表導出と完全一致**（id＝選択肢 ID 逐語・Ref0 以降＝付随参照列のみ・
///   ラベルと ID を Reference に載せない・Status: `talking,choosing`）で 1 件だけ記録される。
/// - `OnChoiceSelectEx`／`OnChoiceSelect` が記録列に**一切現れない**ことを明示検証する——先行段を
///   発行する退行はここで落ちる。
/// - `TalkCommand` 到着順が `Start(1)`→`Resolve(1,NAMED)`→`Start(2)` である。解決が起動より後に
///   なる／解決が落ちる／起動が旧 id を再利用する、のいずれの退行でもタグ列が食い違って落ちる。
#[test]
fn on_id_choice_fires_named_event_only_then_resolves_and_starts() {
    let fixture = Fixture::default()
        .without_boot_greeting()
        .with_steady_value_indices([0])
        .with_response(NAMED_CHOICE_ID, FIXED_CHOICE_SCRIPT.to_string());

    // hold_indices=[0]: steady talk（id=1）の TalkDone を park して active talk 窓を維持する。
    // quit_flags: index0（steady talk）=false・index1（選択由来 talk）=true（終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    establish_choice_wait(&harness.sender, &[NAMED_CHOICE_ID], None);
    send(&harness, KanadeMsg::Choice(choice_input(NAMED_CHOICE_ID)));

    let Driven {
        recorded,
        commands,
        started,
        ..
    } = harness.finish("kanade named-choice join");

    // (1) 段列は任意名 1 段のみ（Ex／無印は一切発行されない）。
    assert_eq!(
        choice_get_ids(&recorded),
        vec![NAMED_CHOICE_ID],
        "`On` 始まり ID は任意名イベント 1 段のみを発行するはず（Ex／無印の先行なし・裁定 1）: {recorded:?}"
    );

    // (2) 任意名 GET の layout が events 表導出と完全一致する（Ref0 以降＝付随参照列のみ）。
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

    // (3) 応答から解決と起動がこの順で起きる。
    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            format!("Resolve(1,{NAMED_CHOICE_ID})"),
            "Start(2)".to_string(),
        ],
        "解決指示は起動指示の直前に、同一チャンネルの FIFO 順で届くはず: {commands:?}"
    );

    // (4) 起動は新 talk_id・選択由来スクリプト（旧 id を再利用しない）。
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は steady talk"
    );
    assert_eq!(
        started[1].script, FIXED_CHOICE_SCRIPT,
        "2 本目は選択由来の応答スクリプト（既存の起動棚に載った）"
    );

    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（選択由来 talk quit:true→Unloading{{Quit}}→Unload）"
    );
}
