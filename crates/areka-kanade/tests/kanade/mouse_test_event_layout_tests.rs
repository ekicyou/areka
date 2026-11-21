use super::test_support::{mouse_gets, mouse_input};
use super::{
    CloseReason, Driven, ExecutionSnapshot, FIXED_FAREWELL_SCRIPT, Fixture, KanadeConfig,
    KanadeMsg, MouseButton, MouseEventKind, MouseInput, QuitPolicy, events, expected_call,
    expected_unload, send, spawn_harness,
};

/// Boot（挨拶なし＝`Steady{None}` 直行）→ 各 Mouse を FIFO 注入 → CloseRequest → 終了完走まで駆動し
/// 記録を確定する（cage 1〜3 の共通ドライバ）。
///
/// close talk が唯一の StartTalk（挨拶なし・204 マウス）なので、index 0 を quit:true にして終了系列を
/// 駆動する。
fn drive_mouse_steady_none(mouse: Vec<MouseInput>) -> Driven {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
    );

    send(&harness, KanadeMsg::Boot);
    for m in mouse {
        send(&harness, KanadeMsg::Mouse(m));
    }
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );

    harness.finish("kanade mouse steady join")
}

// ============================================================================
// Cage 1: OnMouseMove layout（Ref0..6・Status 行なし）＋ 204 → StartTalk 不発
// ============================================================================

/// `Steady{None}` の Move 注入が正典 Reference layout の `OnMouseMove` GET を発行し、記録が
/// events 表導出の期待値と Ref0..6・Status まで完全一致する。
///
/// 204 の扱いもここで見る: マウス GET は 204（未注入の既定＝明示の NoContent と同値）
/// ゆえ StartTalk を生まず、`Steady{None}` は維持され、以降の close 握手が正常完走する（末尾 Unload）。
///
/// # 非空虚性
/// 到達 StartTalk は close talk のみ（1 本）であり、記録には events 導出の OnMouseMove GET がちょうど 1 件
/// 現れる。もし 204 で talk が起きれば started が 2 本になり落ちる。記録を期待値と完全一致で比べるので、
/// layout の退行（例: Ref6 欠落・Status 誤付与）もそこで捕捉する。
#[test]
fn onmousemove_layout_matches_events_table() {
    let (x, y, scope) = (10_i64, 20_i64, 0_u32);
    let driven = drive_mouse_steady_none(vec![mouse_input(
        MouseEventKind::Move,
        x,
        y,
        scope,
        Some("Head"),
    )]);

    let expected = expected_call(events::on_mouse_move(
        x,
        y,
        scope,
        Some("Head"),
        &ExecutionSnapshot::INACTIVE,
    ));

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

    assert_eq!(
        driven.recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（204 マウス後も close→quit で正常完走）"
    );
}

// ============================================================================
// Cage 2: Ref4 None → 空文字転写（references 長は 7）
// ============================================================================

/// `region=None` のとき Ref4 が空文字 `""` へ転写され、references 長は 7 のまま。
///
/// 記録が events 表導出（region=None）と完全一致することで、「ヘッダ枠は存在・値のみ空」という
/// 実 SSP wire（Reference4 空値）を固定する。
#[test]
fn onmousemove_region_none_ref4_empty() {
    let (x, y, scope) = (1_i64, 2_i64, 1_u32);
    let driven =
        drive_mouse_steady_none(vec![mouse_input(MouseEventKind::Move, x, y, scope, None)]);

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
}

// ============================================================================
// Cage 3: Ref5 左右ボタン識別
// ============================================================================

/// DoubleClick の左右ボタンが Ref5 で識別される（左 "0"／右 "1"）。
///
/// 同一 run に Left・Right の 2 つの DoubleClick を注入し、両 `OnMouseDoubleClick` GET が
/// events 表導出（`on_mouse_double_click`）と完全一致することを検証する。Ref2 は正典で常に "0"、
/// Ref5 のみが左右で分岐する。
#[test]
fn double_click_ref5_left_right() {
    let (x, y, scope) = (33_i64, 44_i64, 0_u32);
    let driven = drive_mouse_steady_none(vec![
        mouse_input(
            MouseEventKind::DoubleClick {
                button: MouseButton::Left,
            },
            x,
            y,
            scope,
            Some("Face"),
        ),
        mouse_input(
            MouseEventKind::DoubleClick {
                button: MouseButton::Right,
            },
            x,
            y,
            scope,
            Some("Face"),
        ),
    ]);

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

    let gets = mouse_gets(&driven.recorded);
    assert_eq!(
        gets.len(),
        2,
        "OnMouseDoubleClick GET は左右 2 件記録されるはず: {:?}",
        driven.recorded
    );
    assert_eq!(*gets[0], expected_left, "1 件目は左（Ref5=\"0\"）と一致");
    assert_eq!(*gets[1], expected_right, "2 件目は右（Ref5=\"1\"）と一致");

    assert_eq!(
        driven.started.len(),
        1,
        "204 マウスは StartTalk を生まない＝close talk 1 本のみ: {:?}",
        driven.started
    );
}
