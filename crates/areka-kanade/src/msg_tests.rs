//! 知らせの型（[`KanadeMsg`]・[`ShioriCall`] ほか）のテスト。
//!
//! もとは `msg.rs` のファイルの中にあった塊を中身を変えずに移したもの。

use super::*;

/// `Send + 'static` を型検査で強制する静的アサーション。
fn assert_send_static<T: Send + 'static>() {}

#[test]
fn kanade_and_shiori_msgs_are_send_static() {
    // 観測可能な完了条件: メッセージ型が Send + 'static な所有データのみで構成される。
    assert_send_static::<KanadeMsg>();
    assert_send_static::<ShioriMsg>();
    assert_send_static::<ShioriCall>();
    assert_send_static::<ShioriOutcome>();
    assert_send_static::<ShioriFailure>();
    assert_send_static::<KanadeConfig>();
    assert_send_static::<CloseReason>();
    assert_send_static::<MonotonicMs>();
    // マウス入力契約型（Task 1・DD-IE 系）も Send + 'static な所有データのみ。
    assert_send_static::<MouseInput>();
    assert_send_static::<MouseEventKind>();
    assert_send_static::<MouseButton>();
    // 送出イベント ID の出所カテゴリ型（DD-1）も同様。
    assert_send_static::<EventId>();
    // 選択確定入力（Task 1.3・C2 契約増分）も Send + 'static な所有データのみ。
    assert_send_static::<ChoiceInput>();
}

/// [`ChoiceInput`] が 4 情報（id／label／scope／references）を**無改変**で保持すること。
///
/// kanade は選択肢 ID・表示ラベル・付随参照列のいずれも意味解釈せず不透明転写する
/// （Req 1.5 の前提・[[areka-surface-args-opaque-string-downstream-resolve]] と同精神）。
/// 参照列は記述順を保存する（Req 3.1／3.3 の Reference 割付が順序に依存する）。
#[test]
fn choice_input_transcribes_four_fields_verbatim() {
    let input = ChoiceInput {
        id: "Onおしゃべり頻度メニュー".to_string(),
        label: r"\![*]おしゃべり頻度".to_string(),
        scope: 1,
        references: vec![
            "arg0".to_string(),
            String::new(),
            "arg2 with spaces".to_string(),
        ],
    };
    // 4 情報が構築時の値のまま観測できる（正規化・trim・空要素の除去をしない）。
    assert_eq!(input.id, "Onおしゃべり頻度メニュー");
    assert_eq!(input.label, r"\![*]おしゃべり頻度");
    assert_eq!(input.scope, 1);
    assert_eq!(
        input.references,
        vec![
            "arg0".to_string(),
            String::new(),
            "arg2 with spaces".to_string()
        ]
    );
    // clone は値等価（値オブジェクト＝同一性なし・MouseInput と同型）。
    assert_eq!(input.clone(), input);
    // KanadeMsg::Choice variant が ChoiceInput を運ぶ（additive）。
    let _ = KanadeMsg::Choice(input);
}

/// [`KanadeMsg::ChoiceWaiting`] は dispatcher が `base_now` で ms 換算済みの値を運ぶ（DD-9）。
///
/// `display_end` は [`MonotonicMs`]（換算済み）であり、talk 層の経過秒ではない。
/// `timeout_directive_secs` は指令のまま搬送し、deadline 写像（DD-8）は kanade の領分
/// （本タスクでは写像を行わない＝タスク 2.1 の担当）。
#[test]
fn choice_waiting_msg_carries_converted_display_end_and_raw_directive() {
    let msg = KanadeMsg::ChoiceWaiting {
        talk_id: crate::talk::TalkId(7),
        choice_ids: vec!["a".to_string(), "b".to_string()],
        display_end: MonotonicMs(12_345),
        timeout_directive_secs: None,
    };
    match msg {
        KanadeMsg::ChoiceWaiting {
            talk_id,
            choice_ids,
            display_end,
            timeout_directive_secs,
        } => {
            assert_eq!(talk_id, crate::talk::TalkId(7));
            assert_eq!(choice_ids, vec!["a".to_string(), "b".to_string()]);
            assert_eq!(display_end, MonotonicMs(12_345));
            // 未指定は None のまま届く（既定値の適用はこの境界では行われない・DD-8）。
            assert!(timeout_directive_secs.is_none());
        }
        _ => unreachable!("constructed ChoiceWaiting"),
    }
    // 無効化指令・明示秒指定も逐語で運べる（完全語彙 3 値・DD-8）。
    let disabled = KanadeMsg::ChoiceWaiting {
        talk_id: crate::talk::TalkId(8),
        choice_ids: Vec::new(),
        display_end: MonotonicMs(0),
        timeout_directive_secs: Some(-1.0),
    };
    let explicit = KanadeMsg::ChoiceWaiting {
        talk_id: crate::talk::TalkId(9),
        choice_ids: Vec::new(),
        display_end: MonotonicMs(0),
        timeout_directive_secs: Some(45.0),
    };
    let directive = |m: &KanadeMsg| match m {
        KanadeMsg::ChoiceWaiting {
            timeout_directive_secs,
            ..
        } => *timeout_directive_secs,
        _ => unreachable!("constructed ChoiceWaiting"),
    };
    assert_eq!(directive(&disabled), Some(-1.0));
    assert_eq!(directive(&explicit), Some(45.0));
}

/// 既存 8 variant が無改変のままであること（Req 4.4 の additive 規律）。
///
/// 網羅 match は漏れをコンパイラが強制するため、既存 variant の綴り・保持データを
/// 変更するとこのテストのコンパイルが壊れる。新 2 variant は末尾に additive 追加され、
/// 既存 8 種の判別結果を一切変えない。
#[test]
fn existing_eight_kanade_msg_variants_are_unchanged_by_additive_growth() {
    fn label(msg: &KanadeMsg) -> &'static str {
        match msg {
            KanadeMsg::Boot => "Boot",
            KanadeMsg::Tick { now: _ } => "Tick",
            KanadeMsg::TalkDone(_) => "TalkDone",
            KanadeMsg::CloseRequest { reason: _ } => "CloseRequest",
            KanadeMsg::ForceQuit { reason: _ } => "ForceQuit",
            KanadeMsg::ShioriDown { kind: _, reason: _ } => "ShioriDown",
            KanadeMsg::Mouse(_) => "Mouse",
            KanadeMsg::Close => "Close",
            // 新 2 variant（Task 1.3）。
            KanadeMsg::Choice(_) => "Choice",
            KanadeMsg::ChoiceWaiting { .. } => "ChoiceWaiting",
            // 殻で答える複数件のリソース照会（additive・既存の判別結果を変えない）。
            KanadeMsg::ResourceQuery { ids: _, reply: _ } => "ResourceQuery",
            // 利用者の中断（additive・既存の判別結果を変えない）。
            KanadeMsg::UserBreak { scope: _ } => "UserBreak",
            // ゴーストの切替の要求（additive・既存の判別結果を変えない）。
            KanadeMsg::ChangeGhost(_) => "ChangeGhost",
            // 汎用の通知の入口（additive・既存の判別結果を変えない）。
            KanadeMsg::RaiseEvent { .. } => "RaiseEvent",
            // 台詞の切れ目の口（additive・既存の判別結果を変えない）。
            KanadeMsg::AwaitTalkGap { .. } => "AwaitTalkGap",
            // 外から届いた実行状態の知らせ（additive・既存の判別結果を変えない）。
            KanadeMsg::ExecutionState(_) => "ExecutionState",
            // 今の実行の状態の問い合わせ（additive・既存の判別結果を変えない）。
            KanadeMsg::StatusQuery { reply: _ } => "StatusQuery",
            // 時間切れの知らせ（additive・既存の判別結果を変えない）。
            KanadeMsg::BalloonTimeout { talk_id: _ } => "BalloonTimeout",
            // アンカーの選択（additive・既存の判別結果を変えない）。
            KanadeMsg::Anchor(_) => "Anchor",
            // SSTP の 2 つの入口（additive・既存の判別結果を変えない）。
            KanadeMsg::SstpSend { .. } => "SstpSend",
            KanadeMsg::SstpNotify { .. } => "SstpNotify",
        }
    }
    let existing = [
        KanadeMsg::Boot,
        KanadeMsg::Tick {
            now: MonotonicMs(1),
        },
        KanadeMsg::TalkDone(crate::talk::TalkDone {
            talk_id: crate::talk::TalkId(1),
            reason: crate::talk::TalkEndReason::Ended,
            quit_reserved: false,
        }),
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
        KanadeMsg::ForceQuit {
            reason: CloseReason::System,
        },
        KanadeMsg::ShioriDown {
            kind: ShioriDownKind::HelperExited,
            reason: "pipe closed".to_string(),
        },
        KanadeMsg::Mouse(MouseInput {
            scope: 0,
            x: 0,
            y: 0,
            region: None,
            kind: MouseEventKind::Move,
        }),
        KanadeMsg::Close,
    ];
    let labels: Vec<&'static str> = existing.iter().map(label).collect();
    assert_eq!(
        labels,
        vec![
            "Boot",
            "Tick",
            "TalkDone",
            "CloseRequest",
            "ForceQuit",
            "ShioriDown",
            "Mouse",
            "Close",
        ],
        "既存 8 variant は additive 追加後も同一の判別結果を保つ"
    );
}

/// wire 形（SHIORI へ渡る文字列）は出所カテゴリに依らず逐語であること（DD-1・Req2.6/3.6）。
///
/// `EventId` はチョークポイントが出所別の受理規則を適用するためだけの区別であり、
/// SHIORI へ出る文字列は `as_str()`（および `Display`）が与える逐語表現のみである。
#[test]
fn event_id_as_str_is_verbatim_wire_form_for_both_origins() {
    assert_eq!(EventId::Static("OnBoot").as_str(), "OnBoot");
    assert_eq!(
        EventId::Static("basewareversion").as_str(),
        "basewareversion"
    );
    // 選択起源は任意名を逐語で運ぶ（事前登録不要・Req2.6）。
    assert_eq!(
        EventId::Choice("OnMenuBack".to_string()).as_str(),
        "OnMenuBack"
    );
    // 同一綴りなら出所が違っても wire 形は同一（表現の一字一句同一性）。
    assert_eq!(
        EventId::Choice("OnBoot".to_string()).as_str(),
        EventId::Static("OnBoot").as_str()
    );
    // Display は as_str と一致する（`id.to_string()` の既存呼出面が wire 形を保つ）。
    assert_eq!(EventId::Static("OnClose").to_string(), "OnClose");
    assert_eq!(
        EventId::Choice("OnMenuBack".to_string()).to_string(),
        "OnMenuBack"
    );
    // 外から頼まれた任意の名前も逐語（On 接頭を要さない）。
    assert_eq!(
        EventId::Sstp("UserDefined".to_string()).as_str(),
        "UserDefined"
    );
}

/// `ShioriCall` の `id` が [`EventId`] を運び、GET/NOTIFY 双方で wire 形を取り出せる。
#[test]
fn shiori_call_carries_event_id_for_both_methods() {
    let get = ShioriCall::Get {
        id: EventId::Static("OnBoot"),
        references: Vec::new(),
        status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
    };
    let notify = ShioriCall::Notify {
        id: EventId::Choice("OnMenuBack".to_string()),
        references: Vec::new(),
        status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
    };
    let wire = |call: &ShioriCall| match call {
        ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => id.as_str().to_string(),
    };
    assert_eq!(wire(&get), "OnBoot");
    assert_eq!(wire(&notify), "OnMenuBack");
}

#[test]
fn mouse_input_types_construct_and_compare() {
    // 境界メッセージの形（scope・座標・当たり判定名・種別・ボタン）が構築でき、
    // 値の同一性（PartialEq/Eq）で観測できる。
    let mv = MouseInput {
        scope: 0,
        x: 12,
        y: 34,
        region: Some("head".to_string()),
        kind: MouseEventKind::Move,
    };
    assert_eq!(mv.kind, MouseEventKind::Move);
    assert_eq!(mv.region.as_deref(), Some("head"));

    let dbl_left = MouseInput {
        scope: 1,
        x: -5,
        y: 0,
        region: None,
        kind: MouseEventKind::DoubleClick {
            button: MouseButton::Left,
        },
    };
    assert_eq!(
        dbl_left.kind,
        MouseEventKind::DoubleClick {
            button: MouseButton::Left
        }
    );
    assert_ne!(
        MouseEventKind::DoubleClick {
            button: MouseButton::Left
        },
        MouseEventKind::DoubleClick {
            button: MouseButton::Right
        }
    );
    // KanadeMsg::Mouse variant が MouseInput を運ぶ（additive・Req 4.4）。
    let _ = KanadeMsg::Mouse(mv);
}

#[test]
fn shiori_failure_display_matches_vocabulary() {
    // 区別語彙（接続確立失敗／タイムアウト／helper 死活／SHIORI エラー）の Display。
    assert_eq!(
        ShioriFailure::Handshake("boom".to_string()).to_string(),
        "shiori handshake failure: boom"
    );
    assert_eq!(
        ShioriFailure::Timeout("30s".to_string()).to_string(),
        "shiori request timeout: 30s"
    );
    assert_eq!(
        ShioriFailure::Ipc("pipe closed".to_string()).to_string(),
        "shiori ipc failure: pipe closed"
    );
    assert_eq!(
        ShioriFailure::Shiori("400".to_string()).to_string(),
        "shiori error response: 400"
    );
    // Internal は境界写像でなく kanade 内部で構成される内部規律違反語彙（DD-IT-11）。
    assert_eq!(
        ShioriFailure::Internal("event_id_not_allowed: OnTalk".to_string()).to_string(),
        "kanade internal violation: event_id_not_allowed: OnTalk"
    );
}

#[test]
fn close_reason_maps_to_onclose_ref0() {
    assert_eq!(CloseReason::User { scope: 0 }.as_ref_str(), "user");
    assert_eq!(CloseReason::System.as_ref_str(), "system");
}

#[test]
fn kanade_config_new_supplies_documented_defaults() {
    let config = KanadeConfig::new("master", "1.0.0");
    assert_eq!(config.shell_name, "master");
    assert_eq!(config.baseware_version, "1.0.0");
    assert_eq!(config.baseware_name, "areka");
    assert_eq!(config.close_talk_deadline_ms, 30_000);
}

/// 選択肢タイムアウト既定値は 30_000 ms（Req 7.8・DD-8・裁定 5）。
///
/// 正典（ukadoc）は数値を規定していないため areka 裁量で単一の値を定め、対応表へ記録する。
/// 既定は `KanadeConfig::new`（既定構築経路）から観測できる。
#[test]
fn kanade_config_new_supplies_choice_timeout_default_of_30s() {
    let config = KanadeConfig::new("master", "1.0.0");
    assert_eq!(
        config.choice_timeout_default_ms, 30_000,
        "choice_timeout_default_ms default must be 30_000 (Req 7.8・DD-8)"
    );
}

#[test]
fn kanade_config_new_defaults_additive_fields() {
    // Task 5.1 / design C8: 追加3フィールドの既定は true / 0 / 空。
    // 既定により既存 boot happy-path 檻は意味論無改変で緑（3.1）。
    let config = KanadeConfig::new("master", "1.0.0");
    assert!(
        config.first_boot,
        "first_boot default must be true (現行挙動不変)"
    );
    assert_eq!(
        config.vanish_count, 0,
        "vanish_count default must be 0 (OnFirstBoot Ref0)"
    );
    assert!(
        config.first_boot_epilogue.is_empty(),
        "first_boot_epilogue default must be empty (何も添付しない)"
    );
    // script 種別の記録の名は結線側が入れる（既定は空文字）。
    assert_eq!(config.ghost_name, "");
}

#[test]
fn monotonic_ms_orders_by_inner_value() {
    assert!(MonotonicMs(1) < MonotonicMs(2));
    assert_eq!(MonotonicMs(5), MonotonicMs(5));
}

#[test]
fn shiori_request_envelope_wires_and_outcome_variants_exist() {
    // envelope 結線（reply_channel）が型検査を通り、応答端で全 outcome を送れる。
    let (reply, receiver) = areka_actor::reply_channel::<ShioriOutcome>();
    let msg = ShioriMsg::Request {
        call: ShioriCall::Get {
            id: EventId::Static("OnBoot"),
            references: vec!["master".to_string()],
            status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
        },
        reply,
    };
    // Request の reply 端を取り出して 1 度だけ応答を送る。
    match msg {
        ShioriMsg::Request { call, reply } => {
            match call {
                ShioriCall::Get { id, references, .. } => {
                    assert_eq!(id.as_str(), "OnBoot");
                    assert_eq!(references, vec!["master".to_string()]);
                }
                ShioriCall::Notify { .. } => unreachable!("constructed Get"),
            }
            let _ = reply.send(ShioriOutcome::Value(r"\0hi\e".to_string()));
        }
        _ => unreachable!("constructed Request"),
    }
    // 受信側で Value を観測できる。
    match receiver.recv() {
        Ok(ShioriOutcome::Value(script)) => assert_eq!(script, r"\0hi\e"),
        Ok(_) => unreachable!("expected Value outcome"),
        Err(_) => unreachable!("reply should have been sent"),
    }

    // 残りの outcome variant が構築可能であることを型検査で確認。
    let _ = ShioriOutcome::NoContent;
    let _ = ShioriOutcome::Notified;
    let _ = ShioriOutcome::Unloaded;
    let _ = ShioriOutcome::Failed(ShioriFailure::Timeout("x".to_string()));

    // Notify 呼出も構築できる。
    let _notify = ShioriCall::Notify {
        id: EventId::Static("OnInitialize"),
        references: Vec::new(),
        status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
    };
}
