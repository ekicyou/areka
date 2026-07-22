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
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, MouseButton,
    MouseEventKind, MouseInput, StartTalk, TalkId, events,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, Fixture, Harness,
    MouseResponse, QuitPolicy, RecordedCall, expected_call, expected_unload, join_bounded,
    spawn_harness, spawn_harness_gated,
};

/// マウス GET へ注入する撫で talk スクリプト（cage 7／10・`Steady{None}` からの起動と
/// OnSecondChange 起動との混在を区別する fixture 語彙）。
const FIXED_MOUSE_SCRIPT: &str = r"\0\s[0]なでなで\e";

/// active talk 中のマウス由来 Value に注入する置換 talk スクリプト（cage 8・置換檻）。
/// steady／close／撫で（cage 7/10）と別文字列にし、到達 StartTalk の由来を script で識別する。
const FIXED_MOUSE_REPLACE_SCRIPT: &str = r"\0\s[0]なにかな？\e";

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
        Fixture::quitting().without_boot_greeting().with_mouse_response(
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
    assert_eq!(*gets[0], expected, "記録 GET は events 表導出（INACTIVE）と一致するはず");
    assert_eq!(gets[0].status, None, "Steady{{None}} のマウス GET は Status 行を出さない");

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
        &ExecutionSnapshot { talk_active: true },
    ));
    assert_eq!(
        *gets[0], expected_talking,
        "再生中のマウス GET は events 表導出（talk_active=true）と一致し Status: talking を帯びるはず"
    );
    assert_eq!(gets[0].method, CallMethod::Get, "マウス系は常に GET（NOTIFY 化しない・DD-IE-1）");
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
    assert_eq!(started[0].talk_id, TalkId(1), "steady talk は id=1（最初の StartTalk）");
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
        &ExecutionSnapshot { talk_active: true },
    ));
    assert_eq!(expected_notify.method, CallMethod::Notify, "再生中の pump は NOTIFY");
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
    assert_eq!(started[0].talk_id, TalkId(1), "OnSecondChange talk は先頭採番 id=1");
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
