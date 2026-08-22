use super::test_support::{mouse_gets, move_input};
use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, ExecutionSnapshot, FIXED_FAREWELL_SCRIPT, Fixture,
    Harness, KanadeConfig, KanadeMsg, MouseButton, MouseEventKind, MouseInput, MouseResponse,
    QuitPolicy, RecordedCall, StartTalk, events, expected_call, expected_unload, join_bounded,
    spawn_harness,
};

/// 駆動結果: 確定した shiori 記録列と、宛先へ到達した StartTalk 列。
struct Driven {
    recorded: Vec<RecordedCall>,
    started: Vec<StartTalk>,
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
    assert_eq!(
        recorded_mouse.method,
        CallMethod::Get,
        "マウスは常に GET（NOTIFY 化しない）"
    );
    assert_eq!(recorded_mouse.id, "OnMouseMove");
    assert_eq!(
        recorded_mouse.references.len(),
        7,
        "References は常に 7 要素（Ref0..6）"
    );
    assert_eq!(recorded_mouse.references[0], x.to_string(), "Ref0=x");
    assert_eq!(recorded_mouse.references[1], y.to_string(), "Ref1=y");
    assert_eq!(
        recorded_mouse.references[2], "0",
        "Ref2=ホイール量（M1 固定 \"0\"）"
    );
    assert_eq!(
        recorded_mouse.references[3],
        scope.to_string(),
        "Ref3=scope"
    );
    assert_eq!(
        recorded_mouse.references[4], "Head",
        "Ref4=region（不透明転写）"
    );
    assert_eq!(
        recorded_mouse.references[5], "0",
        "Ref5=移動は常に \"0\"（ボタン非押下）"
    );
    assert_eq!(
        recorded_mouse.references[6], "mouse",
        "Ref6=デバイス種（M1 固定 \"mouse\"）"
    );
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
    assert_eq!(
        gets.len(),
        1,
        "OnMouseMove GET はちょうど 1 件: {:?}",
        driven.recorded
    );
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
    assert_eq!(
        expected_left.references[5], "0",
        "左ダブルクリックの Ref5 は \"0\""
    );
    assert_eq!(
        expected_right.references[5], "1",
        "右ダブルクリックの Ref5 は \"1\""
    );

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

    let driven = drive_mouse_steady_none(
        fixture,
        vec![move_input(x, y, scope, Some("Head"))],
        vec![true],
    );

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
    assert_eq!(
        *gets[0], expected,
        "記録 GET は events 表導出と一致（204 でも layout 同一）"
    );

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
