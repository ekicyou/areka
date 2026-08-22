use super::test_support::{mouse_gets, move_input};
use super::{
    CloseReason, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, Fixture, Harness, KanadeConfig, KanadeMsg,
    QuitPolicy, TalkId, expected_unload, join_bounded, spawn_harness, spawn_harness_gated,
};

// ============================================================================
// Cage 5a: フェーズ無視（Boot 完了前＝Idle）
// ============================================================================

/// Boot 完了前（`Phase::Idle`）に届いた Mouse は無視され、マウス GET を一切発行しない（Req8.1・DD-IE-8）。
///
/// # 決定性
/// Boot より**前**に Mouse を送ると、単一 inbox の FIFO により Mouse は `Idle` で処理され（横断アームの
/// `_ =>` trace 無視・状態不変）、その後 Boot が boot 系列を同期完走して `Steady{None}` へ至る。ゆえに
/// マウス GET は構造的に発行されない。close talk（quit:true）で終了系列を駆動し記録を確定する。
///
/// # 非空虚性
/// もし非 Steady のマウスが GET を発行していれば `mouse_gets` が空でなくなり落ちる。終了が正常完走
/// （末尾 Unload）することで「無視しただけで運行は乱れていない（状態不変）」を確認する。
#[test]
fn phase_ignore_before_boot_no_mouse_get() {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
    );

    // Boot より前に Mouse を注入（Idle で処理される＝非 Steady・無視）。
    harness
        .sender
        .send(KanadeMsg::Mouse(move_input(5, 6, 0, Some("Head"))))
        .expect("send Mouse before Boot");
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
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

    join_bounded("kanade mouse idle-ignore join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence");
    drop(sender);
    sakura.join_bounded("mock-sakura idle-ignore join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // Idle のマウスは GET を発行しない（記録に現れない）。
    assert!(
        mouse_gets(&recorded).is_empty(),
        "Boot 完了前（Idle）の Mouse はマウス GET を発行しないはず: {:?}",
        recorded
    );
    // 状態不変の観測: 終了系列は正常完走（末尾 Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "非 Steady マウス無視後も close→quit で正常完走（状態不変）"
    );
}

// ============================================================================
// Cage 5b: フェーズ無視（close 系列中＝CloseTalkWait）
// ============================================================================

/// close 握手中（`Phase::CloseTalkWait`＝非 Steady）に届いた Mouse は無視され、マウス GET を発行しない
/// （Req8.1・DD-IE-8）。これは `Steady` の `pending_close` ガード（cage 6）とは別経路——横断アームの
/// `_ =>` 無視である。
///
/// # 決定性
/// 挨拶なし boot で `Steady{None}` 直行 → CloseRequest（即握手・OnClose GET→別れの Value→close talk）。
/// close talk を保留ハーネス（`spawn_harness_gated`・hold_indices=[0]）で park すると、CloseRequest 処理の
/// 同期完走後に phase は `CloseTalkWait`（close talk の TalkDone 待ち）で確定する。続く Mouse は
/// `CloseTalkWait` で処理され横断アームで無視される（GET 不発）。`release_all` で close talk（quit:true）を
/// 解放して終了系列を駆動する。
///
/// # 非空虚性
/// close talk が保留されている限り phase は非 Steady（CloseTalkWait）で確定し、Mouse は必ずその窓で
/// 処理される（FIFO・release は Mouse 送出後）。GET が現れれば横断アームの無視が壊れており落ちる。
#[test]
fn phase_ignore_during_close_series_no_mouse_get() {
    // close talk（挨拶なし・204 基調ゆえ先頭 StartTalk＝index 0）を保留し CloseTalkWait を維持する。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    // CloseRequest は Steady{None} の即握手＝OnClose GET→別れの Value→close talk（保留）→CloseTalkWait。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");
    // close 系列中（CloseTalkWait）の Mouse 注入（横断アームで無視・GET 不発）。
    harness
        .sender
        .send(KanadeMsg::Mouse(move_input(9, 9, 0, Some("Head"))))
        .expect("send Mouse during close series");
    // close talk（quit:true）を解放 → Unloading{Quit}→Unload→StopSelf。
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade mouse close-ignore join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after the close talk is released as quit:true");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura close-ignore join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // CloseTalkWait のマウスは GET を発行しない（記録に現れない）。
    assert!(
        mouse_gets(&recorded).is_empty(),
        "close 系列中（CloseTalkWait）の Mouse はマウス GET を発行しないはず: {:?}",
        recorded
    );
    // close 握手は既存どおり完走: close talk が起動し末尾は Unload。
    assert_eq!(
        started.len(),
        1,
        "到達 StartTalk は close talk 1 本（マウスは talk を起こさない）: {:?}",
        started
    );
    assert_eq!(
        started[0].script, FIXED_FAREWELL_SCRIPT,
        "唯一到達する StartTalk は close talk"
    );
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（close 系列は正常完走・状態不変）"
    );
}

// ============================================================================
// Cage 6: pending_close ガード（active talk 中の close→Mouse 抑止）
// ============================================================================

/// active talk 中に CloseRequest（→`pending_close`）を受けた後の Mouse は、`Steady` にありながら
/// マウス GET を発行しない（`steady::on_mouse` の pending_close ガード・DD-IE-8・close 優先）。
/// close 握手は挨拶 TalkDone 着弾で消化され、既存どおり完走する。
///
/// # 決定性・discriminative 性
/// 挨拶あり（default）fixture の挨拶 talk（受領 index 0）を保留ハーネスで park し `Steady{Some(挨拶)}` を
/// 維持する。Boot→CloseRequest→Mouse は単一 inbox の FIFO で順に処理される: CloseRequest は
/// `Steady{Some}` ゆえ即握手せず `pending_close` を記録し、続く Mouse は `pending_close.is_some()` の
/// ガードで GET を発行しない（`release_all` は Mouse 送出後ゆえ、Mouse は必ず pending_close 成立窓で処理
/// される）。`Steady{Some}` のマウスは本来 GET を発行する（DD-IE-1「再生中でも GET は常に発行」）ため、
/// GET が現れないのは pending_close ガードのみが原因＝本檻は非空虚かつ discriminative。
/// `release_all` で挨拶 TalkDone{Ended}（quit:false）を着弾させ `pending_close` を消化 → begin_close
/// （OnClose GET→別れの Value→close talk・quit:true）→ 終了系列完走。
#[test]
fn pending_close_guard_suppresses_mouse_get() {
    // 挨拶あり＋quit close。挨拶 talk（受領 index 0）を保留し active に保つ。
    // quit_flags: index0（挨拶 talk）=false（Ended→pending_close 消化）・index1（close talk）=true（Quit→終了）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    // Boot → `Steady{Some(挨拶 id=1)}`（挨拶 talk 保留）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    // 挨拶再生中の close 指示 → `Steady{Some}` ゆえ即握手せず pending_close に記録（OnClose まだ）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest during greeting");
    // pending_close 成立窓での Mouse 注入 → ガードで GET 不発（release 前ゆえ Mouse は必ずこの窓で処理）。
    harness
        .sender
        .send(KanadeMsg::Mouse(move_input(11, 22, 0, Some("Head"))))
        .expect("send Mouse while close pending");
    // 挨拶 talk（Ended）を解放 → pending_close を消化して begin_close（OnClose→別れ→close talk quit:true）。
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade mouse pending-close join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade completes the deferred close handshake and terminates");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura pending-close join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) pending_close 中のマウスは GET を発行しない（記録に現れない）。
    assert!(
        mouse_gets(&recorded).is_empty(),
        "pending_close 中の Mouse はマウス GET を発行しないはず（close 優先ガード）: {:?}",
        recorded
    );

    // (2) close 握手は既存どおり完走: 挨拶 talk（id=1）と close talk が到達し、close talk は別れのスクリプト。
    assert!(!started.is_empty(), "少なくとも挨拶 talk が配送されるはず");
    assert_eq!(
        started[0].talk_id,
        TalkId(1),
        "先頭 StartTalk は挨拶 talk（id=1）"
    );
    let farewell_started = started
        .iter()
        .filter(|s| s.script == FIXED_FAREWELL_SCRIPT)
        .count();
    assert_eq!(
        farewell_started, 1,
        "pending_close 消化後に close talk が 1 本起動するはず（握手完走）: {:?}",
        started
    );

    // (3) 終了系列完走: 末尾は Unload（close talk quit:true→Unloading{Quit}→Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（close 握手は既存どおり完走）"
    );
}
