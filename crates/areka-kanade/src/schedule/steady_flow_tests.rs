use super::test_support::{assert_no_second_change, config, steady_none, steady_some};
use super::*;
use crate::msg::ShioriCall;
use crate::schedule::step;
use crate::talk::TalkEndReason;

/// 単一 Action が期待 ShioriCall（GET/NOTIFY・id・references）と一致することを検証する。
fn assert_shiori(action: &Action, expected: &ShioriCall) {
    match (action, expected) {
        (
            Action::ShioriRequest(ShioriCall::Get {
                id,
                references,
                status,
            }),
            ShioriCall::Get {
                id: eid,
                references: erefs,
                status: estatus,
            },
        ) => {
            assert_eq!(id, eid, "GET id 不一致");
            assert_eq!(references, erefs, "GET references 不一致");
            assert_eq!(status, estatus, "GET status 不一致");
        }
        (
            Action::ShioriRequest(ShioriCall::Notify {
                id,
                references,
                status,
            }),
            ShioriCall::Notify {
                id: eid,
                references: erefs,
                status: estatus,
            },
        ) => {
            assert_eq!(id, eid, "NOTIFY id 不一致");
            assert_eq!(references, erefs, "NOTIFY references 不一致");
            assert_eq!(status, estatus, "NOTIFY status 不一致");
        }
        _ => panic!("ShioriRequest の GET/NOTIFY 種別が期待と不一致"),
    }
}

// === pump ゲート表駆動（観測可能な完了条件） ===
// {起動中, Steady(None), Steady(Some), close 握手中以降} × Tick の発行有無・種別。

// --- Steady{None} + Tick → OnSecondChange GET（Ref3=1）・Steady{None}・last_now 更新 ---

#[test]
fn steady_none_tick_emits_get_and_updates_last_now() {
    let now = MonotonicMs(7_200_000); // 2 hours。
    let (next, actions) = step(steady_none(5), Input::Tick { now }, &config());
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert_eq!(
        next.last_now,
        Some(now),
        "last_now は Tick ごとに更新される"
    );
    assert_eq!(actions.len(), 1);
    // GET（Ref3=1）——events:: の出力と厳密一致。
    assert_shiori(
        &actions[0],
        &events::on_second_change(
            now,
            &ExecutionSnapshot {
                talk_active: false,
                choice_active: false,
            },
        ),
    );
}

// --- Steady{Some} + Tick → OnSecondChange NOTIFY（Ref3=0）・Steady{Some} ---

#[test]
fn steady_some_tick_emits_notify() {
    let now = MonotonicMs(3_600_000); // 1 hour。
    let (next, actions) = step(steady_some(TalkId(3), 6), Input::Tick { now }, &config());
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
        _ => panic!("expected Steady{{Some}}"),
    }
    assert_eq!(next.last_now, Some(now));
    assert_eq!(actions.len(), 1);
    // NOTIFY（Ref3=0）——events:: の出力と厳密一致。
    assert_shiori(
        &actions[0],
        &events::on_second_change(
            now,
            &ExecutionSnapshot {
                talk_active: true,
                choice_active: false,
            },
        ),
    );
}

// --- Steady{None} + Tick with pending_close → OnClose GET・ClosePending・pending 消化 ---

#[test]
fn steady_none_tick_with_pending_close_begins_handshake() {
    let now = MonotonicMs(1_000);
    let mut s = steady_none(5);
    s.pending_close = Some(CloseReason::User);
    let (next, actions) = step(s, Input::Tick { now }, &config());
    assert!(
        matches!(
            next.phase,
            Phase::ClosePending {
                reason: CloseReason::User
            }
        ),
        "pending_close あり Tick は握手を開始し ClosePending へ"
    );
    assert!(next.pending_close.is_none(), "pending_close は消化される");
    assert_eq!(
        next.last_now,
        Some(now),
        "握手開始でも last_now は更新される"
    );
    assert_eq!(actions.len(), 1);
    assert_shiori(
        &actions[0],
        &events::on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE),
    );
    // OnSecondChange は発行しない。
    assert_no_second_change(&actions);
}

// --- boot 中（BootMain）+ Tick → OnSecondChange なし（ゲート閉・boot は pump しない） ---

#[test]
fn boot_phase_tick_emits_no_second_change() {
    let s = State {
        phase: Phase::BootMain,
        last_now: None,
        next_talk_id: 1,
        pending_close: None,
        choice: None,
        choice_prev_talk: None,
    };
    let (_next, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(1_000),
        },
        &config(),
    );
    // boot::step は pump を発行しない（ゲートは Steady に閉じている）。
    assert_no_second_change(&actions);
}

// --- close 握手中以降（ClosePending / CloseTalkWait）+ Tick → OnSecondChange なし ---

#[test]
fn close_pending_tick_emits_no_second_change() {
    let s = State {
        phase: Phase::ClosePending {
            reason: CloseReason::System,
        },
        last_now: None,
        next_talk_id: 1,
        pending_close: None,
        choice: None,
        choice_prev_talk: None,
    };
    let (_next, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(1_000),
        },
        &config(),
    );
    // close::step は現状 stub（pump 非発行）——OnSecondChange が出ないことを検証。
    assert_no_second_change(&actions);
}

#[test]
fn close_talk_wait_tick_emits_no_second_change() {
    let s = State {
        phase: Phase::CloseTalkWait {
            talk_id: TalkId(2),
            deadline: None,
        },
        last_now: None,
        next_talk_id: 3,
        pending_close: None,
        choice: None,
        choice_prev_talk: None,
    };
    let (_next, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(1_000),
        },
        &config(),
    );
    assert_no_second_change(&actions);
}

// === talk 調停（ShioriReply） ===

// --- Steady{None} + Value → StartTalk(id) + Steady{Some(id)}・id 単調増番 ---

#[test]
fn steady_none_value_starts_talk_and_ids_are_monotonic() {
    // 1 本目: next_id=5 → id=5。
    let (s1, actions1) = step(
        steady_none(5),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("hello".to_string()),
            origin: "OnSecondChange",
        },
        &config(),
    );
    match s1.phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id, origin, ..
            }),
        } => {
            assert_eq!(talk_id, TalkId(5));
            assert_eq!(
                origin, "OnSecondChange",
                "origin は応答の出所を転記（pump 起動）"
            );
        }
        _ => panic!("expected Steady{{Some}}"),
    }
    assert_eq!(s1.next_talk_id, 6, "採番カウンタが進む");
    assert_eq!(actions1.len(), 1);
    match &actions1[0] {
        Action::StartTalk(StartTalk {
            talk_id, script, ..
        }) => {
            assert_eq!(*talk_id, TalkId(5));
            assert_eq!(script, "hello");
        }
        _ => panic!("expected StartTalk"),
    }

    // 2 本目（引き継いだカウンタ 6 で別 Steady{None} を想定）→ id=6（再利用しない）。
    let (s2, actions2) = step(
        steady_none(s1.next_talk_id),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("world".to_string()),
            origin: "OnSecondChange",
        },
        &config(),
    );
    let id2 = match &actions2[0] {
        Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
        _ => panic!("expected StartTalk"),
    };
    assert_eq!(id2, TalkId(6), "id は単調増番・再利用しない");
    assert_eq!(s2.next_talk_id, 7);
}

// --- Steady{None} + NoContent(204) → no StartTalk・Steady{None} 維持 ---

#[test]
fn steady_none_no_content_starts_no_talk() {
    let (next, actions) = step(
        steady_none(5),
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "test",
        },
        &config(),
    );
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert_eq!(next.next_talk_id, 5, "204 は採番しない");
    assert!(actions.is_empty(), "204 は talk 起動しない（Req 2.3）");
}

// --- Steady{Some} + Notified → Steady{Some} 維持（NOTIFY pump の応答・無視） ---

#[test]
fn steady_some_notified_stays_and_emits_nothing() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(3)),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert!(actions.is_empty());
}

// --- Steady{Some} + 非マウス Value → warn!+破棄・Steady{Some} 維持・StartTalk なし（DD-6 防御） ---
// origin は非マウス（OnSecondChange）——マウス origin は置換アームへ抜けるため、DD-6 破棄は
// 非マウス origin 限定に狭まった（origin 別 reply 政策・DD-IE-2）。

#[test]
fn steady_some_value_is_discarded_without_start_talk() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("late".to_string()),
            origin: "OnSecondChange",
        },
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert_eq!(next.next_talk_id, 6, "破棄ゆえ採番しない");
    assert!(
        !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
        "Value-during-talk は StartTalk しない（DD-6）"
    );
    assert!(actions.is_empty(), "キュー・中断も発行しない");
}

// === origin 別 reply 政策: 置換 vs DD-6 防御破棄（Req 4.1／4.3／4.4・DD-IE-2／DD-IE-3） ===
// 置換檻（マウス origin→置換）と DD-6 保存檻（非マウス origin→warn＋破棄）は**対**であり
// 同一テスト群に配置する。実機では実 pasta の talking 自衛（204 相当）により置換が構造的に
// 発火しないため mock 檻が唯一の検証手段。origin の match は wildcard にしない（第 3 の origin
// 追加時にレビューで必ず政策を意識させるため）。

// --- (c) Steady{None} + Value（マウス origin）→ StartTalk・ActiveTalk.origin=マウス名（4.1・DD-IE-3） ---

#[test]
fn steady_none_mouse_value_starts_talk_with_mouse_origin() {
    let (next, actions) = step(
        steady_none(5),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("nade".to_string()),
            origin: "OnMouseMove",
        },
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id, origin, ..
            }),
        } => {
            assert_eq!(talk_id, TalkId(5));
            assert_eq!(
                origin, "OnMouseMove",
                "origin は応答の出所（マウス名）を帯びる（動的化）"
            );
        }
        _ => panic!("expected Steady{{Some}}"),
    }
    assert_eq!(next.next_talk_id, 6);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::StartTalk(StartTalk {
            talk_id, script, ..
        }) => {
            assert_eq!(*talk_id, TalkId(5));
            assert_eq!(script, "nade");
        }
        _ => panic!("expected StartTalk"),
    }
}

// --- (c') Steady{Some(id=3)} + Value + origin=OnMouseDoubleClick → 置換（新 talk_id・slot 上書き・4.3） ---

#[test]
fn steady_some_mouse_value_replaces_slot_with_new_talk_id() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("menu".to_string()),
            origin: "OnMouseDoubleClick",
        },
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id, origin, ..
            }),
        } => {
            assert_eq!(
                talk_id,
                TalkId(6),
                "slot は新 talk_id で上書きされる（置換）"
            );
            assert_eq!(
                origin, "OnMouseDoubleClick",
                "slot の origin も置換 origin へ更新"
            );
        }
        _ => panic!("expected Steady{{Some}} replaced"),
    }
    assert_eq!(next.next_talk_id, 7, "置換は新 talk_id を採番する");
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::StartTalk(StartTalk {
            talk_id, script, ..
        }) => {
            assert_eq!(
                *talk_id,
                TalkId(6),
                "StartTalk は新 talk_id（旧 talk は dispatcher が Close-then-spawn）"
            );
            assert_eq!(script, "menu");
        }
        _ => panic!("expected StartTalk（置換）"),
    }
}

// --- DD-6 保存: Steady{Some} + Value + 非マウス origin(OnSecondChange) → warn＋破棄・維持（4.3/4.4） ---
// 置換檻（上）と対。DD-6 防御の意味は「全 origin 防御」から「非マウス origin 限定の防御」へ
// 狭まる——マウス origin は上の置換アームへ抜けるため、本檻は非マウス origin でのみ発火する。

#[test]
fn steady_some_non_mouse_value_is_discarded_dd6() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("late".to_string()),
            origin: "OnSecondChange",
        },
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(
            talk_id,
            TalkId(3),
            "非マウス origin の Value は置換せず維持（DD-6）"
        ),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert_eq!(next.next_talk_id, 6, "破棄ゆえ採番しない");
    assert!(
        !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
        "非マウス Value-during-talk は StartTalk しない（DD-6）"
    );
    assert!(actions.is_empty(), "キュー・中断も発行しない");
}

// --- talk_id 単調性: マウス起動と OnSecondChange 起動を混在させても再利用しない ---

#[test]
fn talk_ids_never_reused_across_mixed_origins() {
    // OnSecondChange 起動（id=5）。
    let (s1, _) = step(
        steady_none(5),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("a".to_string()),
            origin: "OnSecondChange",
        },
        &config(),
    );
    assert_eq!(s1.next_talk_id, 6);
    // 当該 talk 完了 → 定常復帰。
    let (s2, _) = step(
        s1,
        Input::TalkDone(TalkDone {
            talk_id: TalkId(5),
            reason: TalkEndReason::Ended,
        }),
        &config(),
    );
    // マウス起動（id=6・再利用しない）。
    let (s3, actions3) = step(
        s2,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("b".to_string()),
            origin: "OnMouseMove",
        },
        &config(),
    );
    let id = match &actions3[0] {
        Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
        _ => panic!("expected StartTalk"),
    };
    assert_eq!(id, TalkId(6), "id は混在起動でも単調・再利用しない");
    assert_eq!(s3.next_talk_id, 7);
}

// --- 204: マウス origin の NoContent（Steady{None}）→ StartTalk なし（4.2） ---

#[test]
fn steady_none_mouse_no_content_starts_no_talk() {
    let (next, actions) = step(
        steady_none(5),
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnMouseDoubleClick",
        },
        &config(),
    );
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert_eq!(next.next_talk_id, 5, "204 は採番しない");
    assert!(
        actions.is_empty(),
        "マウス origin の 204 も talk 起動しない（Req 4.2）"
    );
}

// === TalkDone{reason: Ended | Interrupted}（非 quit の 2 値ルーティング網羅） ===

// --- Steady{Some(id)} + TalkDone{id, Ended}, pending None → Steady{None}・次 Tick で pump 再開 ---

#[test]
fn steady_talk_done_ended_resumes_steady_and_pump_restarts() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Ended,
        }),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: None }),
        "talk 完了で定常復帰"
    );
    assert!(actions.is_empty(), "TalkDone 自体は副作用なし");

    // 復帰後の次 Tick で pump（GET）が再開することを確認（Req 3.4）。
    let now = MonotonicMs(9_000);
    let (after, tick_actions) = step(next, Input::Tick { now }, &config());
    assert!(matches!(after.phase, Phase::Steady { talk: None }));
    assert_eq!(tick_actions.len(), 1);
    assert_shiori(
        &tick_actions[0],
        &events::on_second_change(
            now,
            &ExecutionSnapshot {
                talk_active: false,
                choice_active: false,
            },
        ),
    );
}

// --- Steady{Some(id)} + TalkDone{id, Interrupted}, pending None → 同じく Steady{None} 復帰 ---
// kanade の 3 値ルーティング（本タスクの担当）: Interrupted は Ended と同一経路（非 quit）
// として steady::on_talk_done に到達する（mod.rs が防御的に非 quit 扱いへ振る）。

#[test]
fn steady_talk_done_interrupted_resumes_steady_same_as_ended() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Interrupted,
        }),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: None }),
        "Interrupted も Ended と同じく定常復帰へ"
    );
    assert!(actions.is_empty(), "TalkDone 自体は副作用なし");
}

// --- Steady{Some(id)} + TalkDone{id, Ended}, pending Some → OnClose GET + ClosePending ---

#[test]
fn steady_talk_done_with_pending_close_begins_handshake() {
    let mut s = steady_some(TalkId(3), 6);
    s.pending_close = Some(CloseReason::System);
    let (next, actions) = step(
        s,
        Input::TalkDone(TalkDone {
            talk_id: TalkId(3),
            reason: TalkEndReason::Ended,
        }),
        &config(),
    );
    assert!(
        matches!(
            next.phase,
            Phase::ClosePending {
                reason: CloseReason::System
            }
        ),
        "talk 完了時に保留 close を消化して握手開始"
    );
    assert!(next.pending_close.is_none(), "pending_close は消化される");
    assert_eq!(actions.len(), 1);
    assert_shiori(
        &actions[0],
        &events::on_close(CloseReason::System, &ExecutionSnapshot::INACTIVE),
    );
}

// === CloseRequest ===

// --- Steady{None} + CloseRequest → OnClose GET + ClosePending（即握手） ---

#[test]
fn steady_none_close_request_begins_handshake_now() {
    let (next, actions) = step(
        steady_none(5),
        Input::CloseRequest {
            reason: CloseReason::User,
        },
        &config(),
    );
    assert!(matches!(
        next.phase,
        Phase::ClosePending {
            reason: CloseReason::User
        }
    ));
    assert_eq!(actions.len(), 1);
    assert_shiori(
        &actions[0],
        &events::on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE),
    );
}

// --- Steady{Some} + CloseRequest → pending_close 記録・Steady{Some} 維持（OnClose まだ） ---

#[test]
fn steady_some_close_request_records_pending_only() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::CloseRequest {
            reason: CloseReason::User,
        },
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(3), "active talk 中は Steady{{Some}} を維持"),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert!(
        matches!(next.pending_close, Some(CloseReason::User)),
        "pending_close に記録される（TalkDone を待つ）"
    );
    assert!(actions.is_empty(), "OnClose はまだ発行しない");
}

// === マウス GET 発行（Req 1.4／2.1／3.1・DD-IE-1／DD-IE-8） ===
// seam（on_mouse）の意図的充填。step() 経由（横断アーム込み）で駆動し、期待 GET は
// events:: の構築子と共有する（Reference 手書き重複を作らない）。

use crate::msg::{MouseButton, MouseEventKind, MouseInput};

fn mouse_move_input(region: Option<&str>) -> MouseInput {
    MouseInput {
        scope: 0,
        x: 10,
        y: 20,
        region: region.map(str::to_string),
        kind: MouseEventKind::Move,
    }
}

fn mouse_dbl_input(button: MouseButton) -> MouseInput {
    MouseInput {
        scope: 0,
        x: 10,
        y: 20,
        region: Some("Bust".to_string()),
        kind: MouseEventKind::DoubleClick { button },
    }
}

// --- Steady{None} + Move(region=Some) → OnMouseMove GET・Steady{None} 維持 ---

#[test]
fn steady_none_mouse_move_emits_get_and_keeps_phase() {
    let (next, actions) = step(
        steady_none(5),
        Input::Mouse(mouse_move_input(Some("Head"))),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: None }),
        "マウス GET は phase を変えない"
    );
    assert_eq!(next.next_talk_id, 5, "マウス GET は採番しない");
    assert_eq!(actions.len(), 1, "GET を 1 件だけ発行");
    // Reference 完全一致は構築子と共有（talk 非アクティブ→INACTIVE・Status 行なし）。
    assert_shiori(
        &actions[0],
        &events::on_mouse_move(
            10,
            20,
            0,
            Some("Head"),
            &ExecutionSnapshot {
                talk_active: false,
                choice_active: false,
            },
        ),
    );
}

// --- Steady{None} + DoubleClick{Left/Right} → OnMouseDoubleClick GET・Ref5 分岐 ---

#[test]
fn steady_none_mouse_double_click_left_emits_get_ref5_zero() {
    let (next, actions) = step(
        steady_none(5),
        Input::Mouse(mouse_dbl_input(MouseButton::Left)),
        &config(),
    );
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert_eq!(actions.len(), 1);
    assert_shiori(
        &actions[0],
        &events::on_mouse_double_click(
            10,
            20,
            0,
            Some("Bust"),
            MouseButton::Left,
            &ExecutionSnapshot {
                talk_active: false,
                choice_active: false,
            },
        ),
    );
}

#[test]
fn steady_none_mouse_double_click_right_emits_get_ref5_one() {
    let (_next, actions) = step(
        steady_none(5),
        Input::Mouse(mouse_dbl_input(MouseButton::Right)),
        &config(),
    );
    assert_eq!(actions.len(), 1);
    assert_shiori(
        &actions[0],
        &events::on_mouse_double_click(
            10,
            20,
            0,
            Some("Bust"),
            MouseButton::Right,
            &ExecutionSnapshot {
                talk_active: false,
                choice_active: false,
            },
        ),
    );
}

// --- Steady{Some(active)} + Move → GET は抑止せず発行・Status: talking を帯びる（DD-IE-1） ---
// active talk 中でもマウス GET は NOTIFY 化せず GET のまま。State::snapshot()（Steady{Some}）から
// talking が導出され Status ヘッダに載る。active talk は維持される。

#[test]
fn steady_some_mouse_move_emits_get_with_talking_status() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::Mouse(mouse_move_input(Some("Head"))),
        &config(),
    );
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert_eq!(
        actions.len(),
        1,
        "active talk 中でもマウス GET を発行（抑止しない・DD-IE-1）"
    );
    // 期待 GET は talk_active=true スナップショット由来＝Status: talking を帯びる。
    let expected = events::on_mouse_move(
        10,
        20,
        0,
        Some("Head"),
        &ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
        },
    );
    assert_shiori(&actions[0], &expected);
    // GET のまま（NOTIFY 化しない）ことも明示。
    assert!(
        matches!(&actions[0], Action::ShioriRequest(ShioriCall::Get { .. })),
        "マウス系は常に GET（NOTIFY 化しない・DD-IE-1）"
    );
}

// --- pending_close 中は Steady でもマウス GET を発行しない（close 優先・DD-IE-8） ---

#[test]
fn steady_mouse_with_pending_close_emits_no_get() {
    let mut s = steady_none(5);
    s.pending_close = Some(CloseReason::System);
    let (next, actions) = step(s, Input::Mouse(mouse_move_input(Some("Head"))), &config());
    assert!(
        matches!(next.phase, Phase::Steady { talk: None }),
        "phase 不変"
    );
    assert!(
        matches!(next.pending_close, Some(CloseReason::System)),
        "guard は pending_close を消費しない"
    );
    assert!(
        actions.is_empty(),
        "close 保留中はマウス GET を発行しない（close 優先）"
    );
}

// === ActiveTalk.script の保持（タスク 4.1・DD-10・Req4.4） ===
//
// `OnChoiceTimeout` の Reference0 は「タイムアウトした選択肢を含むトークのスクリプト」
// （Req3.4）である。その供給源は **kanade が `StartTalk` で自ら作った script** であり
// （DD-10: 通知同梱でなく kanade 内で完結）、起動時に `ActiveTalk` へ転記して保持する。
// 本檻は起動 2 経路（新規起動・マウス由来の置換）を実際に `step()` で通し、`ActiveTalk.script`
// が発行された `StartTalk.script` と一致することを固定する（Ref0 の割付自体はタスク 4.5）。

/// 現 Phase の `ActiveTalk.script` を取り出す（Steady{Some} 以外は panic）。
fn active_script(phase: &Phase) -> &str {
    match phase {
        Phase::Steady {
            talk: Some(active), ..
        } => &active.script,
        _ => panic!("expected Steady{{Some}}"),
    }
}

/// 単一 StartTalk Action の script を取り出す（StartTalk 以外は panic）。
fn started_script(action: &Action) -> &str {
    match action {
        Action::StartTalk(StartTalk { script, .. }) => script,
        _ => panic!("expected StartTalk"),
    }
}

#[test]
fn steady_none_value_records_started_script_in_active_talk() {
    let (next, actions) = step(
        steady_none(5),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(r"\0script-a\e".to_string()),
            origin: "OnSecondChange",
        },
        &config(),
    );
    assert_eq!(actions.len(), 1);
    assert_eq!(
        started_script(&actions[0]),
        r"\0script-a\e",
        "発行された StartTalk の script（既存挙動）"
    );
    assert_eq!(
        active_script(&next.phase),
        r"\0script-a\e",
        "起動した talk の script が ActiveTalk へ保持される（DD-10）"
    );
}

#[test]
fn steady_some_mouse_replacement_records_new_script_in_active_talk() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(r"\0script-b\e".to_string()),
            origin: "OnMouseDoubleClick",
        },
        &config(),
    );
    assert_eq!(actions.len(), 1);
    assert_eq!(started_script(&actions[0]), r"\0script-b\e");
    assert_eq!(
        active_script(&next.phase),
        r"\0script-b\e",
        "置換で差し替わった slot の script も新 talk のものへ更新される（DD-10）"
    );
}
