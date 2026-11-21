use super::test_support::{mouse_gets, mouse_input};
use super::{
    Driven, ExecutionSnapshot, FIXED_STEADY_SCRIPT, Fixture, KanadeConfig, KanadeMsg,
    MouseEventKind, QuitPolicy, TalkId, events, expected_call, expected_unload, send,
    spawn_harness, spawn_harness_gated, tick,
};

/// マウス GET へ注入する撫で talk スクリプト（cage 7・`Steady{None}` からの起動）。
const FIXED_MOUSE_SCRIPT: &str = r"\0\s[0]なでなで\e";

/// active talk 中のマウス由来 Value に注入する置換 talk スクリプト（cage 8）。
/// 他のスクリプトと別文字列にし、到達 StartTalk の由来を script で識別する。
const FIXED_MOUSE_REPLACE_SCRIPT: &str = r"\0\s[0]なにかな？\e";

// ============================================================================
// 統合檻: マウス応答による talk 起動と置換
// ============================================================================
//
// cage 7・8 は、単体檻（cage 1〜6）の上に、マウス GET 応答が既存 talk 棚へ載る運行を検証する。
//
// - cage 7: `Steady{None}` のマウス GET へ Value 応答 → StartTalk 起動。
// - cage 8: active talk 中のマウス由来 Value → 置換（新 talk_id で StartTalk・旧 Done は stale 破棄）。
//   マウス GET は再生中でも GET のまま発行され `Status: talking` を帯びる。OnSecondChange 起動 talk
//   （id=1）とマウス起動 talk（id=2）の混在でも talk_id が単調で再利用されないことも見る。
//
// 置換の判断は述語 `events::value_replaces_active_talk` の 1 か所にある。対になる「非マウス出所の
// OnSecondChange は active talk 中に NOTIFY で発行され talk を起こさない」は `steady_test.rs` の
// `active_talk_tick_emits_notify_ref3_zero` が見る。この統合ハーネスでは mock shiori が NOTIFY へ常に
// `Notified` を返すので、非マウスの Value が active talk 中に届く経路は無い。その Value の破棄分岐
// そのものは `schedule::steady::flow_tests::steady_some_second_change_value_is_discarded_dd6` と
// `schedule::change::tests::raise_event_tests::second_change_reply_during_talk_is_still_discarded` が見る。

/// cage 7: `Steady{None}` のマウス GET へ Value 応答 → StartTalk 起動。
///
/// 挨拶なし boot で `Steady{None}` 直行 → Move 注入 → OnMouseMove GET → fixture が撫でスクリプト
/// Value を返し、mock sakura が `StartTalk{talk_id=1, script}` を受領する。マウス起動 talk を
/// quit:true にして終了系列を駆動する。
///
/// # 非空虚性
/// - マウス GET が現に記録され events 表導出と一致する。
/// - 到達 StartTalk はマウス talk ちょうど 1 本（close talk は起きない）。204 なら talk 0 本になり落ちる。
/// - 末尾 Unload（マウス起動 talk の quit:true が終了系列を完走）。
#[test]
fn mouse_value_from_steady_none_starts_talk() {
    let (x, y, scope) = (10_i64, 20_i64, 0_u32);
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting()
            .without_boot_greeting()
            .with_response("OnMouseMove", FIXED_MOUSE_SCRIPT.to_string()),
        QuitPolicy::PerTalk(vec![true]),
    );

    send(&harness, KanadeMsg::Boot);
    send(
        &harness,
        KanadeMsg::Mouse(mouse_input(MouseEventKind::Move, x, y, scope, Some("Head"))),
    );

    // マウス talk の quit:true が終了系列を駆動するので CloseRequest は送らない。
    let Driven {
        recorded, started, ..
    } = harness.finish("kanade mouse-start join");

    let gets = mouse_gets(&recorded);
    assert_eq!(
        gets.len(),
        1,
        "OnMouseMove GET はちょうど 1 件記録されるはず: {:?}",
        recorded
    );
    let expected = expected_call(events::on_mouse_move(
        x,
        y,
        scope,
        Some("Head"),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(
        *gets[0], expected,
        "記録 GET は events 表導出（INACTIVE）と一致するはず"
    );

    assert_eq!(
        started.len(),
        1,
        "マウス Value は StartTalk を起こす＝到達はマウス talk 1 本のみ（close talk は生じない）: {:?}",
        started
    );
    assert_eq!(
        started[0].script, FIXED_MOUSE_SCRIPT,
        "到達 StartTalk はマウス応答スクリプト（既存 talk 棚に載った）"
    );
    assert_eq!(
        started[0].talk_id,
        TalkId(1),
        "マウス起動 talk は先頭採番 id=1（挨拶なし boot ゆえ最初の StartTalk）"
    );

    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（マウス起動 talk quit:true→Unloading{{Quit}}→Unload）"
    );
}

/// cage 8: active talk 中のマウス由来 Value → 置換（新 talk_id）。
///
/// 保留ハーネス（`spawn_harness_gated`）で steady talk（id=1）を active に保ち、その窓で Move を注入する。
/// マウス GET は active talk 中でも GET のまま発行され（NOTIFY 化しない）`Status: talking` を帯びる。
/// その Value 応答が置換を起こし、新 talk_id（id=2）で StartTalk が下流へ配送される
/// （kanade は slot 上書き＋採番のみ・旧 talk の後始末は dispatcher の Close-then-spawn へ委譲）。
///
/// # 決定的 active-talk 窓と stale Done 破棄（sleep なし）
/// hold_indices=[0, 1] で steady talk（id=1）と置換 talk（id=2）の両方を park する。`expected_holds=2`
/// により releaser は「解放済みかつ 2 本 park 済み」まで待つので、Boot→Tick1→Mouse を kanade が処理し
/// 終えるまで release_all は空振りしない。`release_all` 後、採番順に TalkDone(1,Ended)・TalkDone(2,Quit)
/// を送る:
/// - TalkDone(1,Ended): slot は既に Some(2)。置き換えた旧 talk（1 世代保持の `prev_talk`）と一致するので
///   `talk_done_stale`（info＋現 Phase 維持）で破棄される。`unknown_talk_done`（error）は真に未知の id
///   にだけ出る。
/// - TalkDone(2,Quit): slot Some(2) と一致＝既知 quit → Unloading{Quit}→Unload→StopSelf。
///
/// # 非空虚性
/// - マウス GET が GET のまま `Status: talking` を帯びて記録される。NOTIFY 化していれば `mouse_gets`
///   （GET のみ抽出）に現れず (1) が落ちる。
/// - 到達 StartTalk は steady talk（id=1）と置換 talk（id=2）のちょうど 2 本。置換が起きなければ
///   1 本のまま落ちる。
/// - 末尾 Unload（旧 Done の stale 破棄後もハングしない）。
#[test]
fn active_talk_mouse_value_replaces_with_new_talk_id() {
    let (x, y, scope) = (30_i64, 40_i64, 0_u32);
    // Tick1 の OnSecondChange GET（index 0）が Value を返し steady talk（id=1）を起こす。挨拶なし boot で
    // Tick1 を確実に `Steady{None}` の GET にする。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting()
        .with_response("OnMouseMove", FIXED_MOUSE_REPLACE_SCRIPT.to_string());

    // index0（steady talk）=false（Ended は stale 破棄される）・index1（置換 talk）=true（Quit で終了）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0, 1],
    );

    send(&harness, KanadeMsg::Boot);
    tick(&harness, 1_000);
    send(
        &harness,
        KanadeMsg::Mouse(mouse_input(MouseEventKind::Move, x, y, scope, Some("Head"))),
    );
    gate.release_all();

    let Driven {
        recorded, started, ..
    } = harness.finish("kanade mouse-replace join");

    let gets = mouse_gets(&recorded);
    assert_eq!(
        gets.len(),
        1,
        "active talk 中でも OnMouseMove GET はちょうど 1 件（NOTIFY 化しない）: {:?}",
        recorded
    );
    let expected_talking = expected_call(events::on_mouse_move(
        x,
        y,
        scope,
        Some("Head"),
        &ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
            ..ExecutionSnapshot::INACTIVE
        },
    ));
    assert_eq!(
        *gets[0], expected_talking,
        "再生中のマウス GET は events 表導出（talk_active=true）と一致し Status: talking を帯びるはず"
    );

    assert_eq!(
        started.len(),
        2,
        "steady talk と置換 talk のちょうど 2 本が到達するはず（置換発火）: {:?}",
        started
    );
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は steady 起動 talk（OnSecondChange Value）"
    );
    assert_eq!(
        started[0].talk_id,
        TalkId(1),
        "steady talk は id=1（最初の StartTalk）"
    );
    assert_eq!(
        started[1].script, FIXED_MOUSE_REPLACE_SCRIPT,
        "2 本目はマウス由来の置換 talk（置換スクリプト）"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "置換 talk は新 talk_id=2（旧 id=1 を再利用しない）"
    );

    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（置換 talk quit:true→Unloading{{Quit}}→Unload・旧 Done は stale 破棄済み）"
    );
}
