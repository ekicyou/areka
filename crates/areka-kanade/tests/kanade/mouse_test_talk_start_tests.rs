use super::test_support::{mouse_gets, mouse_input};
use super::{
    CallMethod, Driven, ExecutionSnapshot, FIXED_STEADY_SCRIPT, Fixture, KanadeConfig, KanadeMsg,
    MonotonicMs, MouseEventKind, QuitPolicy, Response, TalkId, events, expected_call,
    expected_unload, spawn_harness, spawn_harness_gated,
};

/// マウス GET へ注入する撫で talk スクリプト（cage 7・`Steady{None}` からの起動）。
const FIXED_MOUSE_SCRIPT: &str = r"\0\s[0]なでなで\e";

/// active talk 中のマウス由来 Value に注入する置換 talk スクリプト（cage 8・置換檻）。
/// steady／close／撫で（cage 7）と別文字列にし、到達 StartTalk の由来を script で識別する。
const FIXED_MOUSE_REPLACE_SCRIPT: &str = r"\0\s[0]なにかな？\e";

// ============================================================================
// 統合檻: マウス応答による talk 起動と置換（設計 Integration Tests #1〜#4・Req8(c)/(c')）
// ============================================================================
//
// 本群（cage 7・8）は、単体檻（cage 1〜6・マウス GET 発行の layout／フェーズ規律）の上に
// **マウス GET 応答が既存 talk 棚へ載る**運行（Req4.1〜4.4・8.1(c)）を統合層で決定的に検証する:
//
// - cage 7 (c): `Steady{None}` のマウス GET へ Value 応答 → StartTalk 起動（mock sakura が受領）。
// - cage 8 (c'): active talk 中のマウス由来 Value → **置換**（新 talk_id で StartTalk・旧 Done は
//   下流で stale 破棄）。マウス GET は再生中でも **GET のまま**発行され `Status: talking` を帯びる
//   （DD-IE-1）。
//   cage 8 は OnSecondChange 起動 talk（id=1）とマウス起動 talk（id=2）の混在でも talk_id が単調・再利用
//   されないことも見る（設計 Integration #4）。
//
// ## 置換檻（cage 8）と DD-6 保存の対（pairing）と非 wildcard の証明
// active talk 中の「マウス Value→置換／非マウス Value→防御破棄（DD-6）」は origin 別 reply 政策の
// 中核であり、両者は対で固定する（設計 Integration #3・「origin の match は wildcard にしない」）。対の
// 片方（非マウス出所＝active talk 中の OnSecondChange pump は NOTIFY（Ref3=0・Status: talking）で発行され
// talk を起こさない）は、同じ駆動の `steady_test.rs` の `active_talk_tick_emits_notify_ref3_zero` が見る。
// **この統合ハーネスでは非マウス Value が active talk 中に届く経路は構造的に存在しない**——`Steady{Some}`
// の OnSecondChange pump は NOTIFY（Ref3=0）で発行され、mock shiori は NOTIFY へ常に `Notified` を返す
// （Value を運ばない・common `respond` の NOTIFY アーム）。ゆえに DD-6 の**リテラルな Value 破棄分岐**（`Steady{Some}`+`Value`+非マウス origin→warn＋破棄）は純粋状態機械の単体檻
// `schedule::steady::tests::steady_some_non_mouse_value_is_discarded_dd6`／同 `..._logs`（mod.rs）が唯一の
// 検証点である。統合層（`active_talk_tick_emits_notify_ref3_zero`）は、その破棄分岐を**防御的 backstop に
// 留める構造的前提**——「非マウス出所は active talk 中に GET でなく NOTIFY で発行され Value を運べない＝置換を
// 起こさない」——を決定的に固定し、cage 8（マウス出所→置換）と対で origin 一致判定の非 wildcard 性を固定する。

/// cage 7 (c): `Steady{None}` のマウス GET へ Value 応答 → StartTalk 起動（Req4.1・8.1(c)）。
///
/// 挨拶なし boot で `Steady{None}` 直行 → Move 注入 → OnMouseMove GET（INACTIVE・Status 行なし）→
/// fixture が撫でスクリプト Value を返す → mock sakura が `StartTalk{talk_id=1, script}` を受領する。
/// マウス起動 talk（受領 index 0）を quit:true にして終了系列（Unloading{Quit}→Unload→StopSelf）を
/// 駆動する（`ActiveTalk.origin`＝実マウスイベント名はログ専用ゆえアクセサを発明せず、StartTalk 起動＋
/// その talk が quit 終了を駆動する後続挙動で「マウス応答が既存 talk 棚に載った」ことを観測する）。
///
/// # 非空虚性
/// - マウス GET が現に記録され events 表導出と一致する（発行された）。
/// - 到達 StartTalk はマウス talk ちょうど 1 本（撫でスクリプト・close talk は起きない＝終了は
///   マウス talk の quit で駆動）。204 なら talk 0 本になり落ちる。
/// - 末尾 Unload（マウス起動 talk の quit:true が終了系列を完走）。
#[test]
fn mouse_value_from_steady_none_starts_talk() {
    let (x, y, scope) = (10_i64, 20_i64, 0_u32);
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting().with_response(
            "OnMouseMove",
            Response::Script(FIXED_MOUSE_SCRIPT.to_string()),
        ),
        // マウス起動 talk（受領 index 0）を quit:true にして終了駆動（close talk は生じない）。
        QuitPolicy::PerTalk(vec![true]),
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Mouse(mouse_input(
            MouseEventKind::Move,
            x,
            y,
            scope,
            Some("Head"),
        )))
        .expect("send Mouse");

    // マウス talk の quit:true が終了系列を駆動する（CloseRequest 不要）。期限付き join。
    let Driven {
        recorded, started, ..
    } = harness.finish("kanade mouse-start join");

    // (1) マウス GET は現に発行・記録され、events 表導出（Steady{None}＝INACTIVE・Status なし）と一致する。
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
        gets[0].status, None,
        "Steady{{None}} のマウス GET は Status 行を出さない"
    );

    // (2) マウス GET Value → StartTalk 起動: 到達はマウス talk ちょうど 1 本（撫でスクリプト・id=1）。
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

    // (3) 後続挙動: マウス起動 talk の quit:true が終了系列を完走させる（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（マウス起動 talk quit:true→Unloading{{Quit}}→Unload）"
    );
}

/// cage 8 (c'): active talk 中のマウス由来 Value → 置換（新 talk_id・DD-IE-1／DD-IE-2・Req4.3）。
///
/// 保留ハーネス（`spawn_harness_gated`）で steady talk（id=1）を active に保ち、その窓で Move を注入する。
/// active talk 中でもマウス GET は **GET のまま**発行され（NOTIFY 化しない）`Status: talking` を帯びる
/// （DD-IE-1）。その Value 応答が **置換**を起こし、新 talk_id（id=2）で StartTalk が下流へ配送される
/// （kanade は slot 上書き＋採番のみ・旧 talk の後始末は dispatcher の Close-then-spawn へ委譲）。
///
/// # 決定的 active-talk 窓＋stale Done 破棄（sleep なし・race-free）
/// hold_indices=[0, 1]: steady talk（id=1）と置換 talk（id=2）の **両**を park し、その TalkDone を inbox へ
/// 送らせない。`expected_holds=2` により releaser は「解放済み **かつ** 2 本 park 済み」まで待つ——ゆえに
/// Boot→Tick1（steady talk 起動・保留）→Mouse（置換・保留）を kanade が処理し終えるまで release_all は空振り
/// しない（park 数バリアが race を閉じる）。`release_all` 後、park された 2 本を採番順（TalkDone(1,Ended)→
/// TalkDone(2,Quit)）で drain 送出する:
/// - TalkDone(1,Ended): slot は既に Some(2)＝**未知 talk_id** ゆえ `unknown_talk_done`（error＋現 Phase 維持）で
///   破棄される（＝旧 talk の Done は stale として棄却・状態整合・DD-IE-2）。
/// - TalkDone(2,Quit): slot Some(2) と一致＝既知 quit → Unloading{Quit}→Unload→StopSelf（終了駆動）。
///
/// # 非空虚性・discriminative 性
/// - マウス GET が **GET のまま**（NOTIFY 化せず）`Status: talking` を帯びて記録される（events 表導出と一致）。
///   NOTIFY 化していれば `mouse_gets`（GET のみ抽出）に現れず (1) が落ちる。
/// - 到達 StartTalk は steady talk（id=1）と置換 talk（id=2）のちょうど 2 本で、置換 talk の script は
///   マウス置換スクリプト・talk_id は n+1（単調・再利用なし）。置換が起きなければ 1 本のまま落ちる。
/// - 末尾 Unload（置換 talk の quit:true で終了完走）＝旧 Done の stale 破棄後もハングせず状態整合。
///
/// # 対と混在（設計 Integration #3・#4）
/// - 対の片方（非マウス出所の pump は active talk 中に NOTIFY で talk を起こさない・DD-6 保存）は
///   `steady_test.rs` の `active_talk_tick_emits_notify_ref3_zero` が同じ保留の駆動で見る。
/// - OnSecondChange 起動 talk（id=1）とマウス起動 talk（id=2）の混在で talk_id が単調・再利用されない
///   ことは、本 cage の (2) が見る（steady スクリプトの id=1 と置換スクリプトの id=2）。
#[test]
fn active_talk_mouse_value_replaces_with_new_talk_id() {
    let (x, y, scope) = (30_i64, 40_i64, 0_u32);
    // Tick1 の OnSecondChange GET（index 0）が Value を返し steady talk（id=1）を起こす。挨拶なし boot で
    // Tick1 を確実に `Steady{None}` の GET にする（DD-IT-12 の race を断つ）。マウス Value は置換スクリプト。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting()
        .with_response(
            "OnMouseMove",
            Response::Script(FIXED_MOUSE_REPLACE_SCRIPT.to_string()),
        );

    // hold_indices=[0, 1]: steady talk（id=1）と置換 talk（id=2）の両 TalkDone を保留。
    // quit_flags: index0（steady talk）=false（Ended→stale 破棄・slot は既に置換済み）・
    //             index1（置換 talk）=true（Quit→終了駆動）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0, 1],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    // Tick1（now=1s）: Steady{None}→GET→Value→steady talk（id=1・保留）→Steady{Some(1)}。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send Tick 1");
    // active talk 窓（Steady{Some(1)}）で Move 注入: GET のまま Status: talking→Value→置換（id=2・保留）。
    harness
        .sender
        .send(KanadeMsg::Mouse(mouse_input(
            MouseEventKind::Move,
            x,
            y,
            scope,
            Some("Head"),
        )))
        .expect("send Mouse during active talk");
    // 両保留を解放 → TalkDone(1,Ended)（stale 破棄）→ TalkDone(2,Quit)（終了駆動）。
    gate.release_all();

    let Driven {
        recorded, started, ..
    } = harness.finish("kanade mouse-replace join");

    // (1) 再生中でもマウス GET は GET のまま発行され Status: talking を帯びる（DD-IE-1）。
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
        // active talk 中＝talk_active:true スナップショット由来＝Status: talking。
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
        gets[0].method,
        CallMethod::Get,
        "マウス系は常に GET（NOTIFY 化しない・DD-IE-1）"
    );
    assert_eq!(
        gets[0].status,
        Some("talking".to_string()),
        "active talk 中のマウス GET は Status: talking を併送する（DD-IE-1）"
    );

    // (2) 置換: 到達 StartTalk は steady talk（id=1）と置換 talk（id=2）のちょうど 2 本。
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

    // (3) 状態整合: 旧 Done の stale 破棄後もハングせず、置換 talk の quit:true が終了系列を完走（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（置換 talk quit:true→Unloading{{Quit}}→Unload・旧 Done は stale 破棄済み）"
    );
}
