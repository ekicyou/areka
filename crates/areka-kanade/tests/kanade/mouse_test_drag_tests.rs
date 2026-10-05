//! ドラッグの開始・終了（`OnMouseDragStart`／`OnMouseDragEnd`）の決定論の檻
//! （areka-P0-mouse-drag-events 設計 Testing Strategy「kanade」K3〜K7・要件 9.1 ⑴⑸⑹⑼・9.2）。
//!
//! mock の SHIORI を結線したハーネスへ `KanadeMsg::Mouse`（種類 `DragStart`／`DragEnd`）を入れ、
//! 送ってよい表との照合まで通った GET が記録に現れるか・現れないかを見る。期待する要求は必ず
//! `events::on_mouse_drag_start`／`on_mouse_drag_end` の組み立てから `expected_call` で作り、
//! Reference の並びを書き写さない。同期のしかたは兄弟の檻（`mouse_test_phase_guard_tests.rs` など）と
//! 同じ: 末尾の talk を quit:true にして終了の系列を走らせ、kanade の期限付き join が成功した時点で
//! 記録を確定する（実時間の待ちなし）。

use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, ExecutionSnapshot, FIXED_FAREWELL_SCRIPT, Fixture,
    Harness, KanadeConfig, KanadeMsg, MouseEventKind, MouseInput, QuitPolicy, RecordedCall, events,
    expected_call, expected_unload, join_bounded, spawn_harness, spawn_harness_gated,
};

/// ドラッグの知らせを組む（`region` は不透明転写・`None`＝判定外）。
fn drag_input(
    kind: MouseEventKind,
    x: i64,
    y: i64,
    scope: u32,
    region: Option<&str>,
) -> MouseInput {
    MouseInput {
        scope,
        x,
        y,
        region: region.map(str::to_string),
        kind,
    }
}

/// 記録列からドラッグの 2 つの GET だけを処理順に抜き出す。
fn drag_gets(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded
        .iter()
        .filter(|c| {
            c.method == CallMethod::Get && (c.id == "OnMouseDragStart" || c.id == "OnMouseDragEnd")
        })
        .collect()
}

/// 終了の CloseRequest（利用者・本体）。
fn close_request() -> KanadeMsg {
    KanadeMsg::CloseRequest {
        reason: CloseReason::User { scope: 0 },
    }
}

/// 期限付き join で kanade と mock sakura を畳み、確定した記録と到達した StartTalk の台本を返す。
fn finish(harness: Harness, label: &str) -> (Vec<RecordedCall>, Vec<String>) {
    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;
    join_bounded(label, DEFAULT_TIMEOUT, kanade).expect("kanade は終了の系列を完走して止まる");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded(label, DEFAULT_TIMEOUT);
    (
        shiori.recorded(),
        started.into_iter().map(|s| s.script).collect(),
    )
}

// ============================================================================
// K3: 定常で開始 → 終了（要件 1.1・2.1・2.4・4.8・5.1・5.5・9.1 ⑴⑸⑼）
// ============================================================================

/// 挨拶なしの起動で `Steady{None}` へ直行し、`DragStart` → `DragEnd` を入れると、記録に組み立てから
/// 作った期待と一致する `OnMouseDragStart` → `OnMouseDragEnd` の GET がこの順に各 1 件現れる。
///
/// # 外すと赤
/// `steady::on_mouse` の腕が無ければ GET が組めず、送ってよい表の行が無ければ送る直前の照合で
/// 内部の失敗（`event_id_not_allowed`）になり、どちらも記録の 2 件が揃わない。開始と終了は座標・
/// スコープ・当たり判定を変えてあるので、取り違え・順の入れ替わりも一致比較で落ちる。
#[test]
fn drag_start_then_end_in_steady_records_both_gets_in_order() {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        // 挨拶なし・マウスは 204 ゆえ唯一の StartTalk は close talk（index 0）＝quit:true で終了を駆動。
        QuitPolicy::PerTalk(vec![true]),
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    for m in [
        drag_input(MouseEventKind::DragStart, 10, 20, 1, Some("Head")),
        drag_input(MouseEventKind::DragEnd, 70, 90, 1, None),
    ] {
        harness
            .sender
            .send(KanadeMsg::Mouse(m))
            .expect("send drag Mouse");
    }
    harness
        .sender
        .send(close_request())
        .expect("send CloseRequest");

    let (recorded, started) = finish(harness, "kanade drag steady join");

    let expected = [
        expected_call(events::on_mouse_drag_start(
            10,
            20,
            1,
            Some("Head"),
            &ExecutionSnapshot::INACTIVE,
        )),
        expected_call(events::on_mouse_drag_end(
            70,
            90,
            1,
            None,
            &ExecutionSnapshot::INACTIVE,
        )),
    ];
    let gets = drag_gets(&recorded);
    assert_eq!(
        gets,
        expected.iter().collect::<Vec<_>>(),
        "定常のドラッグは組み立てどおりの 開始 → 終了 の GET が各 1 件この順に記録されるはず: {:?}",
        recorded
    );
    // 204 のドラッグは talk を起こさない: 到達するのは close talk だけで、終了の系列は完走する。
    assert_eq!(started, vec![FIXED_FAREWELL_SCRIPT.to_string()]);
    assert_eq!(recorded.last(), Some(&expected_unload()));
}

// ============================================================================
// K4: 会話の再生中でも送り、実行状態に talking が付く（要件 4.8・5.1）
// ============================================================================

/// 挨拶の talk（受領 index 0）を保留して `Steady{Some(挨拶)}` を保ったまま開始 → 終了を入れると、
/// 2 つの GET は抑えられずに出て、`talk_active` の snapshot から組んだ期待（`Status: talking`）と一致する。
///
/// # 決定性
/// Boot → DragStart → DragEnd → CloseRequest は 1 つの受信箱の先入れ先出しで処理される。
/// `release_all` は全部を送った後なので、2 つの知らせは必ず挨拶の再生中に処理される。CloseRequest は
/// `Steady{Some}` ゆえ `pending_close` に残り、挨拶の TalkDone（quit:false）で握手へ進み、close talk
/// （quit:true）で終了する。
#[test]
fn drag_during_active_talk_is_sent_with_talking_status() {
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    for m in [
        drag_input(MouseEventKind::DragStart, 3, 4, 0, Some("Bust")),
        drag_input(MouseEventKind::DragEnd, 5, 6, 0, Some("Bust")),
    ] {
        harness
            .sender
            .send(KanadeMsg::Mouse(m))
            .expect("send drag Mouse during greeting");
    }
    harness
        .sender
        .send(close_request())
        .expect("send CloseRequest");
    gate.release_all();

    let (recorded, started) = finish(harness, "kanade drag talking join");

    let talking = ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
        ..ExecutionSnapshot::INACTIVE
    };
    let expected = [
        expected_call(events::on_mouse_drag_start(3, 4, 0, Some("Bust"), &talking)),
        expected_call(events::on_mouse_drag_end(5, 6, 0, Some("Bust"), &talking)),
    ];
    let gets = drag_gets(&recorded);
    assert_eq!(
        gets,
        expected.iter().collect::<Vec<_>>(),
        "再生中もドラッグの GET は出て、talk_active の snapshot から組んだ期待と一致するはず: {:?}",
        recorded
    );
    assert!(
        gets.iter().all(|c| c.status.as_deref() == Some("talking")),
        "再生中のドラッグの GET は Status: talking を併送する: {:?}",
        gets
    );
    // 握手は完走: 挨拶の後に close talk が 1 本起動し、末尾は Unload。
    assert_eq!(
        started
            .iter()
            .filter(|s| s.as_str() == FIXED_FAREWELL_SCRIPT)
            .count(),
        1,
        "{:?}",
        started
    );
    assert_eq!(recorded.last(), Some(&expected_unload()));
}

// ============================================================================
// K5: 起動の前・終了の系列の途中では送らない（要件 5.3・9.1 ⑹）
// ============================================================================

/// Boot より前（`Phase::Idle`＝起動が済む前）に入れた開始 → 終了は、記録に現れない。
/// Boot は 1 通の知らせで起動の系列を同期で走り切るため、受信箱から起動の段の途中へ知らせを
/// 差し込める窓は Boot の前だけである（兄弟の cage 5a と同じ窓）。
///
/// # 外すと赤
/// 「定常だけ」の横断の腕を外して `on_mouse` へ流すと、`Idle` でも GET が出て `drag_gets` が空でなくなる。
#[test]
fn drag_before_boot_is_not_sent_and_boot_completes() {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
    );

    for kind in [MouseEventKind::DragStart, MouseEventKind::DragEnd] {
        harness
            .sender
            .send(KanadeMsg::Mouse(drag_input(kind, 1, 2, 0, Some("Head"))))
            .expect("send drag Mouse before Boot");
    }
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(close_request())
        .expect("send CloseRequest");

    let (recorded, started) = finish(harness, "kanade drag before-boot join");

    assert!(
        drag_gets(&recorded).is_empty(),
        "起動が済む前のドラッグは GET を出さないはず: {:?}",
        recorded
    );
    // 起動の系列と終了の系列は今どおり完走（close talk 1 本・末尾 Unload）。
    assert_eq!(started, vec![FIXED_FAREWELL_SCRIPT.to_string()]);
    assert_eq!(recorded.last(), Some(&expected_unload()));
}

/// close talk を保留して `CloseTalkWait`（終了の系列の途中）に入れた開始 → 終了は、記録に現れない。
///
/// # 決定性
/// 挨拶なしで `Steady{None}` へ直行 → CloseRequest は即握手（OnClose → close talk・保留）。
/// `release_all` は 2 つの知らせを送った後なので、知らせは必ず `CloseTalkWait` で処理される。
#[test]
fn drag_during_close_series_is_not_sent_and_close_completes() {
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(close_request())
        .expect("send CloseRequest");
    for kind in [MouseEventKind::DragStart, MouseEventKind::DragEnd] {
        harness
            .sender
            .send(KanadeMsg::Mouse(drag_input(kind, 7, 8, 0, None)))
            .expect("send drag Mouse during close series");
    }
    gate.release_all();

    let (recorded, started) = finish(harness, "kanade drag close-series join");

    assert!(
        drag_gets(&recorded).is_empty(),
        "終了の系列の途中（CloseTalkWait）のドラッグは GET を出さないはず: {:?}",
        recorded
    );
    assert_eq!(started, vec![FIXED_FAREWELL_SCRIPT.to_string()]);
    assert_eq!(recorded.last(), Some(&expected_unload()));
}

// ============================================================================
// K6: 終了の握手の待ちでは送らない（要件 5.2・9.1 ⑹）
// ============================================================================

/// 挨拶の再生中に CloseRequest（→ `pending_close`）を受けた後の開始 → 終了は、`Steady` にありながら
/// 記録に現れない（`on_mouse` の先頭の防御）。握手は挨拶の TalkDone で進み、今どおり完走する。
///
/// # 外すと赤
/// `Steady{Some}` のドラッグは本来 GET を出す（K4）ので、GET が現れないのは先頭の防御だけが理由になる。
#[test]
fn drag_while_close_pending_is_not_sent_and_handshake_completes() {
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(close_request())
        .expect("send CloseRequest during greeting");
    for kind in [MouseEventKind::DragStart, MouseEventKind::DragEnd] {
        harness
            .sender
            .send(KanadeMsg::Mouse(drag_input(kind, 11, 22, 0, Some("Head"))))
            .expect("send drag Mouse while close pending");
    }
    gate.release_all();

    let (recorded, started) = finish(harness, "kanade drag close-pending join");

    assert!(
        drag_gets(&recorded).is_empty(),
        "終了の握手の待ちのドラッグは GET を出さないはず: {:?}",
        recorded
    );
    assert_eq!(
        started
            .iter()
            .filter(|s| s.as_str() == FIXED_FAREWELL_SCRIPT)
            .count(),
        1,
        "pending_close を消化した後に close talk が 1 本起動するはず: {:?}",
        started
    );
    assert_eq!(recorded.last(), Some(&expected_unload()));
}

// ============================================================================
// K7: 開始を送った後に終了を要求すると、後の終了は送らない（要件 5.4・9.1 ⑹）
// ============================================================================

/// 定常で `DragStart`（送られる）→ CloseRequest → `DragEnd` の順に入れると、開始の GET だけが記録に
/// 現れ、終了の GET は現れない（対にするための例外を作らない）。握手は今どおり完走する。
///
/// # 決定性
/// close talk（受領 index 0）を保留し、`DragEnd` は必ず `CloseTalkWait` で処理される
/// （`release_all` は `DragEnd` を送った後）。
///
/// # 外すと赤
/// 「開始を送ったなら終了も送る」例外を足すと、`OnMouseDragEnd` が記録に現れて落ちる。
#[test]
fn drag_end_after_close_request_is_not_sent() {
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Mouse(drag_input(
            MouseEventKind::DragStart,
            30,
            40,
            0,
            Some("Head"),
        )))
        .expect("send DragStart in steady");
    harness
        .sender
        .send(close_request())
        .expect("send CloseRequest");
    harness
        .sender
        .send(KanadeMsg::Mouse(drag_input(
            MouseEventKind::DragEnd,
            50,
            60,
            0,
            Some("Head"),
        )))
        .expect("send DragEnd after CloseRequest");
    gate.release_all();

    let (recorded, started) = finish(harness, "kanade drag end-after-close join");

    let expected_start = expected_call(events::on_mouse_drag_start(
        30,
        40,
        0,
        Some("Head"),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(
        drag_gets(&recorded),
        vec![&expected_start],
        "開始だけが送られ、終了の要求の後の終了は送られないはず: {:?}",
        recorded
    );
    assert_eq!(started, vec![FIXED_FAREWELL_SCRIPT.to_string()]);
    assert_eq!(recorded.last(), Some(&expected_unload()));
}
