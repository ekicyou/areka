use super::test_support::{
    CANONICAL_CHOICE_ID, choice_get_ids, choice_input, establish_choice_wait,
};
use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, FailKind, FailOn, Fixture, Harness, KanadeConfig,
    KanadeMsg, MouseButton, MouseEventKind, MouseInput, QuitPolicy, TalkCommand, expected_unload,
    join_bounded, spawn_harness_gated_failing,
};

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
