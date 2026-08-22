use super::test_support::{mouse_gets, move_input};
use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, ExecutionSnapshot, FIXED_STEADY_SCRIPT, Fixture,
    Harness, KanadeConfig, KanadeMsg, MonotonicMs, MouseResponse, QuitPolicy, TalkId, events,
    expected_call, expected_unload, join_bounded, spawn_harness, spawn_harness_gated,
};

/// マウス GET へ注入する撫で talk スクリプト（cage 7／10・`Steady{None}` からの起動と
/// OnSecondChange 起動との混在を区別する fixture 語彙）。
const FIXED_MOUSE_SCRIPT: &str = r"\0\s[0]なでなで\e";

/// active talk 中のマウス由来 Value に注入する置換 talk スクリプト（cage 8・置換檻）。
/// steady／close／撫で（cage 7/10）と別文字列にし、到達 StartTalk の由来を script で識別する。
const FIXED_MOUSE_REPLACE_SCRIPT: &str = r"\0\s[0]なにかな？\e";

// ============================================================================
// 統合檻: マウス応答による talk 起動と置換（設計 Integration Tests #1〜#4・Req8(c)/(c')）
// ============================================================================
//
// 本群（cage 7〜10）は、単体檻（cage 1〜6・マウス GET 発行の layout／フェーズ規律）の上に
// **マウス GET 応答が既存 talk 棚へ載る**運行（Req4.1〜4.4・8.1(c)）を統合層で決定的に検証する:
//
// - cage 7 (c): `Steady{None}` のマウス GET へ Value 応答 → StartTalk 起動（mock sakura が受領）。
// - cage 8 (c'): active talk 中のマウス由来 Value → **置換**（新 talk_id で StartTalk・旧 Done は
//   下流で stale 破棄）。マウス GET は再生中でも **GET のまま**発行され `Status: talking` を帯びる
//   （DD-IE-1）。
// - cage 9 (DD-6 保存・cage 8 と**対**): active talk 中の非マウス出所（OnSecondChange pump）は
//   構造上 **NOTIFY（Ref3=0）**で発行され Value を運べない＝置換を起こさない。origin の一致判定は
//   wildcard でない（マウス出所は置換／非マウス出所は不発）ことを cage 8 と並置して固定する。
// - cage 10: マウス起動 talk と OnSecondChange 起動 talk を混在させても talk_id が単調・再利用されない。
//
// ## 置換檻（cage 8）と DD-6 保存檻（cage 9）の対（pairing）と非 wildcard の証明
// active talk 中の「マウス Value→置換／非マウス Value→防御破棄（DD-6）」は origin 別 reply 政策の
// 中核であり、両者は**同一テスト群に対で配置**する（設計 Integration #3・「origin の match は wildcard に
// しない」）。ただし**この統合ハーネスでは非マウス Value が active talk 中に届く経路は構造的に存在
// しない**——`Steady{Some}` の OnSecondChange pump は NOTIFY（Ref3=0）で発行され、mock shiori は NOTIFY へ
// 常に `Notified` を返す（Value を運ばない・common `respond` の NOTIFY アーム）。ゆえに DD-6 の**リテラルな
// Value 破棄分岐**（`Steady{Some}`+`Value`+非マウス origin→warn＋破棄）は純粋状態機械の単体檻
// `schedule::steady::tests::steady_some_non_mouse_value_is_discarded_dd6`／同 `..._logs`（mod.rs）が唯一の
// 検証点である。統合層（cage 9）は、その破棄分岐を**防御的 backstop に留める構造的前提**——「非マウス出所は
// active talk 中に GET でなく NOTIFY で発行され Value を運べない＝置換を起こさない」——を決定的に固定し、
// cage 8（マウス出所→置換）と並置することで origin 一致判定の非 wildcard 性を統合層で対に緑化する。

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
        Fixture::quitting()
            .without_boot_greeting()
            .with_mouse_response(
                "OnMouseMove",
                MouseResponse::Script(FIXED_MOUSE_SCRIPT.to_string()),
            ),
        // マウス起動 talk（受領 index 0）を quit:true にして終了駆動（close talk は生じない）。
        QuitPolicy::PerTalk(vec![true]),
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Mouse(move_input(x, y, scope, Some("Head"))))
        .expect("send Mouse");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // マウス talk の quit:true が終了系列を駆動する（CloseRequest 不要）。期限付き join。
    join_bounded("kanade mouse-start join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the mouse-started talk (quit:true)");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura mouse-start join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

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
#[test]
fn active_talk_mouse_value_replaces_with_new_talk_id() {
    let (x, y, scope) = (30_i64, 40_i64, 0_u32);
    // Tick1 の OnSecondChange GET（index 0）が Value を返し steady talk（id=1）を起こす。挨拶なし boot で
    // Tick1 を確実に `Steady{None}` の GET にする（DD-IT-12 の race を断つ）。マウス Value は置換スクリプト。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting()
        .with_mouse_response(
            "OnMouseMove",
            MouseResponse::Script(FIXED_MOUSE_REPLACE_SCRIPT.to_string()),
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
        .send(KanadeMsg::Mouse(move_input(x, y, scope, Some("Head"))))
        .expect("send Mouse during active talk");
    // 両保留を解放 → TalkDone(1,Ended)（stale 破棄）→ TalkDone(2,Quit)（終了駆動）。
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade mouse-replace join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the replacement talk (quit:true) after stale Done discard");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura mouse-replace join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

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
    // 置換 talk_id は旧 talk_id の直後（n+1・単調・再利用なし）。
    assert_eq!(
        started[1].talk_id.0,
        started[0].talk_id.0 + 1,
        "置換は旧 talk_id の直後（n+1）を採番する（単調・再利用なし）: {:?}",
        started
    );

    // (3) 状態整合: 旧 Done の stale 破棄後もハングせず、置換 talk の quit:true が終了系列を完走（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（置換 talk quit:true→Unloading{{Quit}}→Unload・旧 Done は stale 破棄済み）"
    );
}

/// cage 9（DD-6 保存・cage 8 と対）: active talk 中の非マウス pump は NOTIFY で置換を起こさない（Req4.3／DD-6）。
///
/// cage 8（マウス出所→置換）と**同一テスト群**に並置し、origin 一致判定の非 wildcard 性を対で固定する。
/// 保留ハーネスで steady talk（id=1）を active に保ち、その窓で Tick を挟む。`Steady{Some}` の OnSecondChange
/// pump は構造的に **NOTIFY（Ref3=0・Status: talking）**で発行され、mock shiori は NOTIFY へ常に `Notified` を
/// 返す（Value を運ばない）。ゆえに非マウス出所は active talk 中に talk を起こせない（置換も防御破棄分岐への
/// 到達もない）——これがマウス出所の置換（cage 8）と対を成す非 wildcard の統合証拠である。
///
/// # DD-6 リテラル破棄分岐の所在（統合層では構造的に到達不能）
/// `Steady{Some}`+`Value`+非マウス origin→warn＋破棄という**リテラルな破棄分岐**は、上記のとおり本統合
/// ハーネスでは（非マウス pump が NOTIFY ゆえ Value が届かず）構造的に到達しない。その分岐の直接検証は純粋
/// 状態機械の単体檻 `schedule::steady::tests::steady_some_non_mouse_value_is_discarded_dd6`（および mod.rs の
/// `warn_steady_value_during_talk_logs`）が担う。本 cage は、その破棄を防御的 backstop に留める**構造的前提**
/// （非マウス出所＝NOTIFY＝Value 不達）を統合層で決定的に固定する。
///
/// # 決定性
/// steady_test::active_talk_tick_emits_notify_ref3_zero と同一イディオム。hold_indices=[0] で steady talk を
/// 保留し、Tick2 を必ず `Steady{Some(1)}` で処理させる（保留 talk の TalkDone は inbox へ届かない＝interleaving
/// なし）。release_all→CloseRequest は両順序とも同一終了へ収束する（sleep なし・race-free）。
///
/// # 非空虚性
/// - active talk 中の Tick が OnSecondChange NOTIFY（Ref3=0・Status: talking）を発行する（events 導出と一致）。
///   GET を出していれば expected NOTIFY と一致する記録が現れず落ちる。
/// - NOTIFY tick は talk を起こさない＝到達 steady スクリプト talk はちょうど 1 本（保留分のみ）。置換や
///   自発 talk が起きれば 2 本以上になり落ちる。
#[test]
fn active_talk_non_mouse_pump_is_notify_no_replacement_dd6_preserved() {
    // Tick1 の OnSecondChange GET（index 0）が Value を返し steady talk（id=1）を起こす。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting();

    // hold_indices=[0]: steady talk（id=1）を保留し active 窓を作る。
    // quit_flags: index0（steady talk）=false（Ended→復帰）・index1（close talk）=true（Quit→終了）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    // Tick1（now=1s）: Steady{None}→GET→Value→steady talk（id=1・保留）→Steady{Some(1)}。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send Tick 1");
    // Tick2（now=2s）: Steady{Some(1)}→OnSecondChange NOTIFY（Ref3=0・Notified・talk を起こさない）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(2_000),
        })
        .expect("send Tick 2");
    // 保留 TalkDone（quit:false）を解放 → Steady{None} へ復帰。
    gate.release_all();
    // close 指示（両順序とも同一終了へ収束）。
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

    join_bounded("kanade dd6-preserve join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence (Unload→StopSelf)");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura dd6-preserve join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) active talk 中の Tick は OnSecondChange NOTIFY（Ref3=0・Status: talking）を発行する
    //     （非マウス pump は再生中に GET でなく NOTIFY＝Value を運べない＝置換不能・DD-6 保存の構造前提）。
    let expected_notify = expected_call(events::on_second_change(
        MonotonicMs(2_000),
        &ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
        },
    ));
    assert_eq!(
        expected_notify.method,
        CallMethod::Notify,
        "再生中の pump は NOTIFY"
    );
    assert_eq!(
        expected_notify.references[3], "0",
        "NOTIFY pump の Ref3 は \"0\"（active talk 中・応答無視）"
    );
    assert!(
        recorded.iter().any(|c| *c == expected_notify),
        "active talk 中の Tick は OnSecondChange NOTIFY（Ref3=0・Status: talking）を発行するはず（GET でない）: {:?}",
        recorded
    );

    // (2) NOTIFY tick は talk を起こさない: 到達 steady スクリプト talk はちょうど 1 本（保留分のみ）。
    //     非マウス出所は active talk 中に置換も自発 talk も起こさない（DD-6 保存）。
    let steady_starts = started
        .iter()
        .filter(|s| s.script == FIXED_STEADY_SCRIPT)
        .count();
    assert_eq!(
        steady_starts, 1,
        "steady talk は Tick1 の 1 本のみ・active talk 中の NOTIFY tick は talk を起こさない（DD-6 保存）: {:?}",
        started
    );

    // (3) マウス GET は一切現れない（本 cage は非マウス出所のみを注入・cage 8 の GET と対比）。
    assert!(
        mouse_gets(&recorded).is_empty(),
        "本 cage はマウス入力を注入しない＝マウス GET は記録に現れないはず: {:?}",
        recorded
    );

    // (4) close 握手は既存どおり完走: 末尾は Unload（close talk quit:true→Unloading{Quit}→Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（DD-6 保存後も close→quit で正常完走）"
    );
}

/// cage 10: OnSecondChange 起動 talk とマウス起動 talk の混在で talk_id が単調・再利用されない（Req2.1／4.1）。
///
/// マウス起動 talk と OnSecondChange 起動 talk を混在させ、talk_id が 1→2 と単調・一意（再利用しない）で
/// あることを固定する。到達 StartTalk は steady スクリプト（OnSecondChange 出所・id=1）と撫でスクリプト
/// （マウス出所・id=2）の 2 本で、両者は別の出所から採番された別 talk である。
///
/// # 決定性（cage 8／9 と同一の park-count バリア・retry ループなし・sleep なし・race-free）
/// 混在の順序は harness の構造から **OnSecondChange 起動 talk が先（id=1）・マウス起動 talk が後（id=2）**に
/// 固定される: active talk 中の OnSecondChange pump は NOTIFY（Value を運べない・cage 9）ゆえ、保留窓で第 2 の
/// talk を起こせるのはマウス由来 Value（置換）のみだからである。ゆえに逐次順（マウス完了→OnSecondChange 起動）を
/// 非同期 TalkDone→次 Tick で待つ retry ループ（scheduler 依存・flaky）は用いず、`expected_holds` park-count
/// バリアで両 talk を**観測点で決定的に**捉える:
///
/// 1. Boot → `Steady{None}`（挨拶なし・cross-thread TalkDone なし）。
/// 2. Tick1（now=1s）: `Steady{None}`→OnSecondChange GET(occ0)→Value→OnSecondChange 起動 talk（id=1・
///    **保留** hold_indices=[0]）→`Steady{Some(1)}`。保留ゆえ id=1 の TalkDone は inbox へ届かず、active 窓が
///    決定的に維持される（interleaving なし）。
/// 3. Move 注入: `Steady{Some(1)}`→OnMouseMove GET（Status: talking・DD-IE-1）→Value→**置換**→マウス起動
///    talk（id=2・非保留）→`Steady{Some(2)}`。
/// 4. マウス talk（id=2・非保留・quit:true）の TalkDone(2,Quit) は StartTalk(2) 配送に**因果順で後続**する
///    （kanade は Mouse 処理を終えてから次 inbox メッセージ＝TalkDone(2) を捌く・単一 inbox FIFO・in-flight ≤1）。
///    ゆえに競合相手なく Unloading{Quit}→Unload→StopSelf へ決定的に至る（retry ループ不要）。
///
/// 保留した id=1 の TalkDone は kanade 停止（StartTalk 送信端 drop）で releaser が recv_closed 経由で drain し
/// 無害に破棄する（宙吊りなし）。`release_all` を呼ばずとも sakura join は有界（recv_closed 安全弁）。
///
/// # 非空虚性・discriminative 性
/// - 到達 StartTalk はちょうど 2 本で、片方が steady スクリプト（OnSecondChange 出所・id=1）・片方が撫で
///   スクリプト（マウス出所・id=2）＝両出所の混在が現に起きた。置換が起きなければ 1 本のまま落ちる。
/// - talk_id 集合の要素数が talk 本数と一致（再利用なし）かつ 1<2（単調）。
#[test]
fn mixed_mouse_and_second_change_talk_ids_are_monotonic() {
    // Tick1 の OnSecondChange GET（index 0）へ steady Value・マウス GET へ撫でスクリプト。挨拶なし boot で
    // Tick1 を確実に `Steady{None}` の GET にする（DD-IT-12 の race を断つ）。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting()
        .with_mouse_response(
            "OnMouseMove",
            MouseResponse::Script(FIXED_MOUSE_SCRIPT.to_string()),
        );

    // hold_indices=[0]: OnSecondChange 起動 talk（id=1・受領 index 0）を保留し active 窓を作る。gate は
    // 保留窓の維持にのみ用い（release_all は呼ばない・id=1 の TalkDone は kanade 停止後に recv_closed 経由で
    // drain される）、明示解放しないため `_gate` で束縛する。
    // quit_flags: index0（OnSecondChange talk）=false（保留・Ended）・index1（マウス置換 talk）=true（Quit→終了駆動）。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    // Tick1（now=1s）: Steady{None}→GET(occ0)→Value→OnSecondChange 起動 talk（id=1・保留）→Steady{Some(1)}。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send Tick 1");
    // Move 注入: active 窓（Steady{Some(1)}）で GET talking→Value→置換→マウス起動 talk（id=2・非保留）。
    harness
        .sender
        .send(KanadeMsg::Mouse(move_input(5, 6, 0, Some("Head"))))
        .expect("send Mouse during active talk");
    // マウス talk（id=2・quit:true）の TalkDone(2,Quit) が終了系列を駆動する（因果順で後続・retry ループ不要）。
    // 保留した id=1 の TalkDone は kanade 停止後に releaser が recv_closed 経由で drain する（release_all 不要）。

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // マウス置換 talk（id=2・quit:true）の Quit が終了系列を完走させるまで期限付き join（バリア）。
    join_bounded("kanade mixed-monotonic join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the mouse replacement talk (quit:true)");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura mixed-monotonic join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) 混在: 到達 StartTalk は OnSecondChange talk とマウス talk のちょうど 2 本（別出所・別 talk）。
    assert_eq!(
        started.len(),
        2,
        "OnSecondChange talk とマウス talk のちょうど 2 本が到達するはず（混在起動）: {:?}",
        started
    );
    assert_eq!(
        started[0].script, FIXED_STEADY_SCRIPT,
        "1 本目は OnSecondChange 起動 talk（steady スクリプト）"
    );
    assert_eq!(
        started[0].talk_id,
        TalkId(1),
        "OnSecondChange talk は先頭採番 id=1"
    );
    assert_eq!(
        started[1].script, FIXED_MOUSE_SCRIPT,
        "2 本目はマウス起動 talk（撫でスクリプト）"
    );
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "マウス talk は id=2（OnSecondChange talk の id=1 を再利用しない）"
    );

    // (2) talk_id は単調・一意（混在起動でも再利用しない・Req2.1）。
    assert!(
        started[1].talk_id.0 > started[0].talk_id.0,
        "talk_id は単調増番（OnSecondChange {:?} < マウス {:?}）",
        started[0].talk_id,
        started[1].talk_id
    );
    let ids: std::collections::HashSet<u64> = started.iter().map(|s| s.talk_id.0).collect();
    assert_eq!(
        ids.len(),
        started.len(),
        "全 StartTalk の talk_id は一意（再利用しない）: {:?}",
        started
    );

    // (3) 終了系列完走: 末尾は Unload（マウス置換 talk quit:true→Unloading{Quit}→Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（混在 talk 後もマウス置換 talk quit:true で正常完走）"
    );
}
