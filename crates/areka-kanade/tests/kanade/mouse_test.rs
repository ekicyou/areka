//! マウスイベント発行の単一 pass/fail 檻（Req8.1・8.2 — 設計 Testing Strategy「Unit Tests
//! （kanade・mouse_test.rs — Req8 (a)(b)(d)＋フェーズ規律）」の 6 檻）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、`KanadeMsg::Mouse`
//! 注入に対する運行系の観測可能な振る舞いを、実時間 sleep なし・注入時刻/入力のみ・全 join 期限付きで
//! 単一の合否として検証する。期待 Reference 構成は必ず `events::on_mouse_*` 構築子から `expected_call`
//! で導出し、ハーネス側にハードコードしない（fixture・assert・実装の三点一正本・Req7.1）。
//!
//! # 檻一覧（設計 Testing Strategy #1〜#6）
//! 1. **(a) OnMouseMove layout**: `Steady{None}` で Move（region=Some("Head")）注入 → 記録 GET が
//!    `expected_call(on_mouse_move(x,y,0,Some("Head"),&INACTIVE))` と一致（Ref0..6・Ref2="0"・Ref5="0"・
//!    Ref6="mouse"・Status 行なし）。
//! 2. **(a') Ref4 None**: region=None → Ref4 が空文字 `""`（references 長は 7 のまま）。
//! 3. **(b) Ref5 左右**: DoubleClick Left→Ref5="0"／Right→Ref5="1"（`on_mouse_double_click` 導出共有）。
//! 4. **(d) 204→無動作**: マウス GET へ NoContent → StartTalk 不発（close talk のみ）・`Steady{None}` 維持。
//! 5. **フェーズ無視**: 非 Steady（Boot 完了前=Idle／close 系列中=CloseTalkWait）で Mouse 注入 →
//!    マウス GET は記録に現れない・状態不変（終了系列は正常完走）。
//! 6. **pending_close ガード**: active talk 中に CloseRequest（→pending_close）→ Mouse 注入 →
//!    マウス GET 不発・close 握手は既存どおり完走。
//!
//! # 決定性（Req8.2）と同期イディオム
//! steady_test.rs／close_test.rs と同じ枠組み: 挨拶なし boot（`without_boot_greeting`）で
//! `Steady{None}` へ直行させ（DD-IT-12 の挨拶 talk race を断つ）、末尾 talk（close talk か保留解放 talk）を
//! quit:true にして終了系列（Unload→StopSelf）を駆動する。kanade の期限付き join が成功した時点で、
//! それまでの全 shiori 呼出・全 StartTalk 配送は確定済みであり、実時間 sleep を一切用いずに記録列を
//! 確定できる。マウス GET は kanade drive ループの同期往復ゆえ、Boot・CloseRequest の同期完走後に
//! FIFO 順で処理される（in-flight ≤ 1・割り込みなし）。

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MouseButton, MouseEventKind,
    MouseInput, StartTalk, TalkId, events,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, Fixture, Harness, MouseResponse,
    QuitPolicy, RecordedCall, expected_call, expected_unload, join_bounded, spawn_harness,
    spawn_harness_gated,
};

/// 駆動結果: 確定した shiori 記録列と、宛先へ到達した StartTalk 列。
struct Driven {
    recorded: Vec<RecordedCall>,
    started: Vec<StartTalk>,
}

/// Move 入力を組む（`region` は不透明転写・`None`＝判定外）。
fn move_input(x: i64, y: i64, scope: u32, region: Option<&str>) -> MouseInput {
    MouseInput {
        scope,
        x,
        y,
        region: region.map(str::to_string),
        kind: MouseEventKind::Move,
    }
}

/// DoubleClick 入力を組む（Ref5 はボタン識別）。
fn dbl_input(x: i64, y: i64, scope: u32, region: Option<&str>, button: MouseButton) -> MouseInput {
    MouseInput {
        scope,
        x,
        y,
        region: region.map(str::to_string),
        kind: MouseEventKind::DoubleClick { button },
    }
}

/// 記録がマウス GET（`OnMouseMove`／`OnMouseDoubleClick`）か。
fn is_mouse_get(c: &RecordedCall) -> bool {
    c.method == CallMethod::Get && (c.id == "OnMouseMove" || c.id == "OnMouseDoubleClick")
}

/// 記録列からマウス GET のみを処理順に抽出する。
fn mouse_gets(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded.iter().filter(|c| is_mouse_get(c)).collect()
}

/// Boot（挨拶なし＝`Steady{None}` 直行）→ 各 Mouse を FIFO 注入 → CloseRequest → 終了完走まで駆動し
/// 記録を確定する（cage 1〜4 の共通ドライバ）。
///
/// `fixture` は `Fixture::quitting().without_boot_greeting()` 由来（挨拶なし・quit close）である前提。
/// 挨拶なし boot は StartTalk を生まないため、Boot は同期完走後 `Steady{None}` にあり、続く各 Mouse は
/// `Steady{None}` で処理される（同期往復ゆえ mouse GET は reply まで畳んでから次メッセージへ進む）。
/// close talk を末尾 talk（quit:true）にして終了系列を駆動する。
fn drive_mouse_steady_none(
    fixture: Fixture,
    mouse: Vec<MouseInput>,
    quit_flags: Vec<bool>,
) -> Driven {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(quit_flags),
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    for m in mouse {
        harness
            .sender
            .send(KanadeMsg::Mouse(m))
            .expect("send Mouse");
    }
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

    join_bounded("kanade mouse steady join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence (Unload→StopSelf)");

    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura mouse join", DEFAULT_TIMEOUT);

    Driven {
        recorded: shiori.recorded(),
        started,
    }
}

// ============================================================================
// Cage 1: (a) OnMouseMove layout（Ref0..6 正典・Status 行なし）
// ============================================================================

/// `Steady{None}` の Move 注入が正典 Reference layout の `OnMouseMove` GET を発行し、記録が
/// events 表導出の期待値と Ref0..6・Status まで完全一致する（Req8.1(a)・2.1〜2.5）。
///
/// # 非空虚性
/// マウス GET は 204（未注入既定）ゆえ StartTalk を生まない。到達 StartTalk は close talk のみ（1 本）で
/// あり、記録には events 導出の OnMouseMove GET がちょうど 1 件現れる。Ref2/Ref5/Ref6/Status を明示確認
/// することで layout の退行（例: Ref6 欠落・Status 誤付与）を捕捉する。
#[test]
fn onmousemove_layout_matches_events_table() {
    let (x, y, scope) = (10_i64, 20_i64, 0_u32);
    let driven = drive_mouse_steady_none(
        Fixture::quitting().without_boot_greeting(),
        vec![move_input(x, y, scope, Some("Head"))],
        // close talk が唯一の StartTalk（挨拶なし・204 マウス）＝index 0 を quit:true で終了駆動。
        vec![true],
    );

    // events 表から期待 GET を導出（Steady{None}＝INACTIVE・Status 行なし・ハードコードしない）。
    let expected = expected_call(events::on_mouse_move(
        x,
        y,
        scope,
        Some("Head"),
        &ExecutionSnapshot::INACTIVE,
    ));

    // 記録された OnMouseMove GET はちょうど 1 件で、期待値と完全一致する。
    let gets = mouse_gets(&driven.recorded);
    assert_eq!(
        gets.len(),
        1,
        "OnMouseMove GET はちょうど 1 件記録されるはず: {:?}",
        driven.recorded
    );
    let recorded_mouse = gets[0];
    assert_eq!(
        *recorded_mouse, expected,
        "記録 GET は events 表導出の OnMouseMove（Ref0..6・Status）と完全一致するはず: {:?}",
        driven.recorded
    );

    // layout の要所を明示確認（設計 #1・退行の直接検出）。
    assert_eq!(recorded_mouse.method, CallMethod::Get, "マウスは常に GET（NOTIFY 化しない）");
    assert_eq!(recorded_mouse.id, "OnMouseMove");
    assert_eq!(recorded_mouse.references.len(), 7, "References は常に 7 要素（Ref0..6）");
    assert_eq!(recorded_mouse.references[0], x.to_string(), "Ref0=x");
    assert_eq!(recorded_mouse.references[1], y.to_string(), "Ref1=y");
    assert_eq!(recorded_mouse.references[2], "0", "Ref2=ホイール量（M1 固定 \"0\"）");
    assert_eq!(recorded_mouse.references[3], scope.to_string(), "Ref3=scope");
    assert_eq!(recorded_mouse.references[4], "Head", "Ref4=region（不透明転写）");
    assert_eq!(recorded_mouse.references[5], "0", "Ref5=移動は常に \"0\"（ボタン非押下）");
    assert_eq!(recorded_mouse.references[6], "mouse", "Ref6=デバイス種（M1 固定 \"mouse\"）");
    assert_eq!(
        recorded_mouse.status, None,
        "Steady{{None}}（INACTIVE）のマウス GET は Status 行を出さない"
    );

    // 204 ゆえマウス由来 talk は起きない: 到達 StartTalk は close talk のみ（Req8.1(a) の副次確認）。
    assert_eq!(
        driven.started.len(),
        1,
        "204 マウスは StartTalk を生まない＝close talk 1 本のみ到達: {:?}",
        driven.started
    );
    assert_eq!(
        driven.started[0].script, FIXED_FAREWELL_SCRIPT,
        "唯一到達する StartTalk は close talk（別れのスクリプト）"
    );
}

// ============================================================================
// Cage 2: (a') Ref4 None → 空文字転写（references 長は 7）
// ============================================================================

/// `region=None` のとき Ref4 が空文字 `""` へ転写され、references 長は 7 のまま（Req8.1(a')・2.3・DD-IE-6）。
///
/// 記録が events 表導出（region=None）と完全一致することで、「ヘッダ枠は存在・値のみ空」という
/// 実 SSP wire（Reference4 空値）を固定する。
#[test]
fn onmousemove_region_none_ref4_empty() {
    let (x, y, scope) = (1_i64, 2_i64, 1_u32);
    let driven = drive_mouse_steady_none(
        Fixture::quitting().without_boot_greeting(),
        vec![move_input(x, y, scope, None)],
        vec![true],
    );

    let expected = expected_call(events::on_mouse_move(
        x,
        y,
        scope,
        None,
        &ExecutionSnapshot::INACTIVE,
    ));

    let gets = mouse_gets(&driven.recorded);
    assert_eq!(gets.len(), 1, "OnMouseMove GET はちょうど 1 件: {:?}", driven.recorded);
    let recorded_mouse = gets[0];
    assert_eq!(
        *recorded_mouse, expected,
        "region=None の記録 GET は events 表導出と完全一致するはず: {:?}",
        driven.recorded
    );
    assert_eq!(
        recorded_mouse.references.len(),
        7,
        "region=None でも References は 7 要素（ヘッダ枠は存在）"
    );
    assert_eq!(
        recorded_mouse.references[4], "",
        "Ref4 は region=None のとき空文字 \"\"（不透明転写・DD-IE-6）"
    );
}

// ============================================================================
// Cage 3: (b) Ref5 左右ボタン識別
// ============================================================================

/// DoubleClick の左右ボタンが Ref5 で識別される（左 "0"／右 "1"・Req8.1(b)・3.3）。
///
/// 同一 run に Left・Right の 2 つの DoubleClick を注入し、両 `OnMouseDoubleClick` GET が
/// events 表導出（`on_mouse_double_click`）と完全一致することを検証する。Ref2 は正典で常に "0"、
/// Ref5 のみが左右で分岐する。
#[test]
fn double_click_ref5_left_right() {
    let (x, y, scope) = (33_i64, 44_i64, 0_u32);
    let driven = drive_mouse_steady_none(
        Fixture::quitting().without_boot_greeting(),
        vec![
            dbl_input(x, y, scope, Some("Face"), MouseButton::Left),
            dbl_input(x, y, scope, Some("Face"), MouseButton::Right),
        ],
        vec![true],
    );

    let expected_left = expected_call(events::on_mouse_double_click(
        x,
        y,
        scope,
        Some("Face"),
        MouseButton::Left,
        &ExecutionSnapshot::INACTIVE,
    ));
    let expected_right = expected_call(events::on_mouse_double_click(
        x,
        y,
        scope,
        Some("Face"),
        MouseButton::Right,
        &ExecutionSnapshot::INACTIVE,
    ));

    // 左右 Ref5 の正典値（events 導出）を明示確認（ハードコードでなく構築子共有の裏取り）。
    assert_eq!(expected_left.references[5], "0", "左ダブルクリックの Ref5 は \"0\"");
    assert_eq!(expected_right.references[5], "1", "右ダブルクリックの Ref5 は \"1\"");

    // 記録された 2 件の OnMouseDoubleClick GET が左→右の順で期待値と一致する。
    let gets = mouse_gets(&driven.recorded);
    assert_eq!(
        gets.len(),
        2,
        "OnMouseDoubleClick GET は左右 2 件記録されるはず: {:?}",
        driven.recorded
    );
    assert_eq!(*gets[0], expected_left, "1 件目は左（Ref5=\"0\"）と一致");
    assert_eq!(*gets[1], expected_right, "2 件目は右（Ref5=\"1\"）と一致");

    // 204 ゆえマウス由来 talk なし: 到達は close talk のみ。
    assert_eq!(
        driven.started.len(),
        1,
        "204 マウスは StartTalk を生まない＝close talk 1 本のみ: {:?}",
        driven.started
    );
}

// ============================================================================
// Cage 4: (d) 204 → StartTalk 不発・Steady{None} 維持
// ============================================================================

/// マウス GET へ mock fixture が NoContent（204）を返すと、GET は記録されるが StartTalk は生じない
/// （Req8.1(d)・2.3）。`Steady{None}` は維持され、以降の close 握手が正常完走する。
///
/// # 非空虚性
/// マウス GET が記録に現れる（＝GET は現に発行された）ことと、到達 StartTalk が close talk のみ（1 本）で
/// あること（＝マウス由来 talk ゼロ）を対で確認する。もし 204 で talk が起きれば started が 2 本になり
/// 落ちる。`Steady{None}` 維持は「マウス処理後も close talk が唯一の追加 talk」であることで観測する。
#[test]
fn no_content_produces_no_start_talk() {
    let (x, y, scope) = (7_i64, 8_i64, 0_u32);
    // OnMouseMove へ明示的に 204 を注入（未注入既定と同値だが、設計 #4 の意図を明示する）。
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_mouse_response("OnMouseMove", MouseResponse::NoContent);

    let driven = drive_mouse_steady_none(fixture, vec![move_input(x, y, scope, Some("Head"))], vec![true]);

    // (1) マウス GET は現に記録された（発行はされている）。
    let gets = mouse_gets(&driven.recorded);
    assert_eq!(
        gets.len(),
        1,
        "204 でも OnMouseMove GET は 1 件発行・記録されるはず: {:?}",
        driven.recorded
    );
    let expected = expected_call(events::on_mouse_move(
        x,
        y,
        scope,
        Some("Head"),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(*gets[0], expected, "記録 GET は events 表導出と一致（204 でも layout 同一）");

    // (2) 204 は StartTalk を生まない: 到達は close talk のみ（マウス由来 talk ゼロ＝Steady{None} 維持）。
    assert_eq!(
        driven.started.len(),
        1,
        "204 マウスは StartTalk を起こさない＝到達は close talk 1 本のみ: {:?}",
        driven.started
    );
    assert_eq!(
        driven.started[0].script, FIXED_FAREWELL_SCRIPT,
        "唯一到達する StartTalk は close talk（マウス由来 talk は生じない）"
    );

    // (3) 終了系列は正常完走（末尾 Unload）＝マウス 204 が運行を乱していない。
    assert_eq!(
        driven.recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（204 マウス後も close→quit で正常完走）"
    );
}

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
    assert_eq!(started[0].talk_id, TalkId(1), "先頭 StartTalk は挨拶 talk（id=1）");
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
