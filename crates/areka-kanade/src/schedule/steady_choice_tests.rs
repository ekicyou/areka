use super::*;
use crate::msg::ShioriCall;
use crate::schedule::step;
use super::test_support::{
    choice_input_of, config, expect_get_call, expect_ledger, status_wire, steady_none,
    steady_some, steady_with_ledger,
};

// --- A. 棄却分岐（規則 1）: すべて状態不変・Action なし ---

/// Req1.3: 選択待ち帳簿が無い（解決済み・未成立）状態の選択確定は棄却する。
#[test]
fn choice_without_ledger_is_rejected_and_leaves_state_unchanged() {
    let (next, actions) = step(
        steady_some(TalkId(3), 6),
        Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
        &config(),
    );
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "棄却は Phase を触らない"
    );
    assert!(next.choice.is_none(), "棄却は帳簿を作らない");
    assert_eq!(next.next_talk_id, 6, "棄却は採番しない");
    assert!(actions.is_empty(), "棄却は Action を発行しない");
}

/// Req1.3: 帳簿の対象 talk が現行 talk と食い違う場合は棄却する（帳簿も無傷）。
#[test]
fn choice_with_ledger_of_other_talk_is_rejected() {
    let mut s = steady_some(TalkId(3), 6);
    s.choice = Some(ChoiceState {
        talk_id: TalkId(999),
        candidates: vec!["OnMenu".to_string()],
        deadline: None,
        phase: ChoicePhase::Waiting,
    });
    let (next, actions) = step(
        s,
        Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
        &config(),
    );
    let ledger = expect_ledger(&next);
    assert_eq!(ledger.talk_id, TalkId(999), "既存帳簿は棄却で変わらない");
    assert!(matches!(ledger.phase, ChoicePhase::Waiting));
    assert!(actions.is_empty());
}

/// Req1.3: 再生中でない（`Steady{None}`＝トーク切替で選択肢が消滅済み）なら棄却する。
#[test]
fn choice_without_active_talk_is_rejected() {
    let mut s = steady_none(5);
    s.choice = Some(ChoiceState {
        talk_id: TalkId(3),
        candidates: vec!["OnMenu".to_string()],
        deadline: None,
        phase: ChoicePhase::Waiting,
    });
    let (next, actions) = step(
        s,
        Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
        &config(),
    );
    assert!(matches!(next.phase, Phase::Steady { talk: None }));
    assert!(matches!(expect_ledger(&next).phase, ChoicePhase::Waiting));
    assert!(actions.is_empty());
}

/// Req1.1: 段の進行中（`Cascading`／`TimeoutInFlight`）の二重確定は棄却する。
#[test]
fn choice_during_cascade_or_timeout_is_rejected_as_busy() {
    for phase in [
        ChoicePhase::Cascading {
            choice_id: "OnMenu".to_string(),
            next: None,
        },
        ChoicePhase::TimeoutInFlight,
    ] {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], phase);
        let (next, actions) = step(
            s,
            Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
            &config(),
        );
        assert!(
            actions.is_empty(),
            "in-flight 中の二重確定は Action を発行しない（Req1.1）"
        );
        assert_eq!(next.next_talk_id, 6, "二重確定は採番しない");
        let ledger = expect_ledger(&next);
        assert!(
            !matches!(ledger.phase, ChoicePhase::Waiting),
            "棄却は段フェーズを巻き戻さない"
        );
    }
}

/// Req1.4: 候補集合に無い ID は棄却し、選択待ち状態を変更しない。
#[test]
fn choice_with_id_outside_candidates_is_rejected() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu", "choice1"], ChoicePhase::Waiting);
    let (next, actions) = step(
        s,
        Input::Choice(choice_input_of("choice9", "他", &[])),
        &config(),
    );
    assert!(actions.is_empty(), "候補外 ID は Action を発行しない");
    let ledger = expect_ledger(&next);
    assert!(
        matches!(ledger.phase, ChoicePhase::Waiting),
        "候補外 ID の棄却は選択待ちを継続させる"
    );
    assert_eq!(
        ledger.candidates,
        vec!["OnMenu".to_string(), "choice1".to_string()],
        "候補列は棄却で変わらない"
    );
}

// --- B. 受理とカスケード第 1 段（規則 2・裁定 1／7） ---

/// Req2.1・裁定 1: `On` 始まり ID は任意名 1 段のみ（Ex／無印を先行発火しない）。
#[test]
fn named_choice_emits_only_the_named_event() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["Onおしゃべり頻度メニュー"],
        ChoicePhase::Waiting,
    );
    let (next, actions) = step(
        s,
        Input::Choice(choice_input_of(
            "Onおしゃべり頻度メニュー",
            "おしゃべり頻度",
            &["a0", "a1"],
        )),
        &config(),
    );
    assert_eq!(actions.len(), 1, "第 1 段の GET を 1 件だけ発行する");
    let (id, refs) = expect_get_call(&actions[0]);
    assert_eq!(id, "Onおしゃべり頻度メニュー", "任意名イベントを逐語発火");
    assert_ne!(id, "OnChoiceSelectEx", "Ex を先行発火しない（裁定 1）");
    assert_ne!(id, "OnChoiceSelect", "無印を先行発火しない（裁定 1）");
    assert_eq!(
        refs,
        vec!["a0".to_string(), "a1".to_string()],
        "Ref0 以降＝付随参照列のみ（Req3.3）"
    );
    match &expect_ledger(&next).phase {
        ChoicePhase::Cascading { choice_id, next } => {
            assert_eq!(choice_id, "Onおしゃべり頻度メニュー");
            assert!(next.is_none(), "任意名形に残段は無い（1 段のみ）");
        }
        _ => panic!("受理で Cascading へ進む"),
    }
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "受理は Phase を触らない（DD-3）"
    );
}

/// Req2.2／3.1: 正典形は `OnChoiceSelectEx` が先行し Reference が正典 layout で並ぶ。
#[test]
fn canonical_choice_emits_choice_select_ex_with_canonical_layout() {
    let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
    let (next, actions) = step(
        s,
        Input::Choice(choice_input_of("choice1", "ラベル", &["r0", "r1"])),
        &config(),
    );
    assert_eq!(actions.len(), 1);
    let (id, refs) = expect_get_call(&actions[0]);
    assert_eq!(id, "OnChoiceSelectEx", "正典形は Ex が先行段（Req2.2）");
    assert_eq!(
        refs,
        vec![
            "ラベル".to_string(),
            "choice1".to_string(),
            "r0".to_string(),
            "r1".to_string()
        ],
        "Ref0=ラベル／Ref1=ID／Ref2 以降=付随参照列（Req3.1）"
    );
    match &expect_ledger(&next).phase {
        ChoicePhase::Cascading { choice_id, next } => {
            assert_eq!(choice_id, "choice1");
            assert!(
                matches!(next, Some(CascadeNext::Select)),
                "正典形は無印 1 段を残段に持つ（Req2.2）"
            );
        }
        _ => panic!("受理で Cascading へ進む"),
    }
}

/// Req2.7・裁定 7: `script:` 前置はイベントを発行せず選択解決のみを行う。
#[test]
fn unsupported_choice_resolves_without_emitting_any_event() {
    let s = steady_with_ledger(TalkId(3), 6, &["script:\\e"], ChoicePhase::Waiting);
    let (next, actions) = step(
        s,
        Input::Choice(choice_input_of("script:\\e", "実行", &[])),
        &config(),
    );
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::ShioriRequest(_))),
        "未対応カテゴリは SHIORI イベントを発行しない（Req2.7）"
    );
    match actions.as_slice() {
        [Action::ResolveChoice { talk_id, id }] => {
            assert_eq!(*talk_id, TalkId(3));
            assert_eq!(id, "script:\\e");
        }
        _ => panic!("未対応カテゴリは ResolveChoice のみを発行する"),
    }
    assert!(next.choice.is_none(), "解決で帳簿は消える");
    assert_eq!(next.next_talk_id, 6, "未対応カテゴリは talk を起動しない");
}

// --- C. カスケード応答（規則 3・DD-4） ---

/// Req4.3／4.6／5.1・DD-4: 応答スクリプトは `[ResolveChoice, StartTalk]` をこの順で同一バッチに載せる。
#[test]
fn cascade_value_emits_resolve_then_start_in_this_order() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["OnMenu"],
        ChoicePhase::Cascading {
            choice_id: "OnMenu".to_string(),
            next: None,
        },
    );
    let (next, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(r"\0次のシーン\e".to_string()),
            origin: "OnChoiceEvent",
        },
        &config(),
    );
    match actions.as_slice() {
        [
            Action::ResolveChoice { talk_id, id },
            Action::StartTalk(StartTalk {
                talk_id: new_id,
                script,
                ..
            }),
        ] => {
            assert_eq!(*talk_id, TalkId(3), "解決対象は旧 talk");
            assert_eq!(id, "OnMenu");
            assert_eq!(*new_id, TalkId(6), "新 talk_id を採番する（Req4.1）");
            assert_eq!(script, r"\0次のシーン\e");
        }
        _ => panic!("[ResolveChoice, StartTalk] のこの順で発行されること（DD-4）"),
    }
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk {
                talk_id,
                origin,
                ref script,
            }),
        } => {
            assert_eq!(talk_id, TalkId(6), "slot は新 talk へ差し替わる（Req4.3）");
            assert_eq!(origin, "OnChoiceEvent", "応答の出所を転記する");
            assert_eq!(script, r"\0次のシーン\e", "起動 script を保持（DD-10）");
        }
        _ => panic!("expected Steady{{Some}} replaced"),
    }
    assert_eq!(next.next_talk_id, 7);
    assert!(next.choice.is_none(), "解決で帳簿は消える");
    assert_eq!(
        next.choice_prev_talk,
        Some(TalkId(3)),
        "choice 起因の slot 差替で旧 talk_id を 1 世代保持する（遷移規則 9）"
    );
}

/// Req2.3: 204 かつ残段ありなら次段（無印・Ref0=ID）を発行する。
#[test]
fn cascade_no_content_advances_to_choice_select_stage() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["choice1"],
        ChoicePhase::Cascading {
            choice_id: "choice1".to_string(),
            next: Some(CascadeNext::Select),
        },
    );
    let (next, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceSelectEx",
        },
        &config(),
    );
    assert_eq!(actions.len(), 1, "次段の GET を 1 件だけ発行する");
    let (id, refs) = expect_get_call(&actions[0]);
    assert_eq!(id, "OnChoiceSelect", "残段は無印イベント（Req2.2）");
    assert_eq!(refs, vec!["choice1".to_string()], "Ref0=ID のみ（Req3.2）");
    match &expect_ledger(&next).phase {
        ChoicePhase::Cascading { choice_id, next } => {
            assert_eq!(choice_id, "choice1");
            assert!(next.is_none(), "無印段の後に残段は無い");
        }
        _ => panic!("次段発行後も Cascading を維持する"),
    }
    assert_eq!(next.next_talk_id, 6, "204 は採番しない");
}

/// Req2.3／5.3: 204 かつ残段なしなら選択解決のみ（起動なし）。
#[test]
fn cascade_no_content_at_last_stage_resolves_without_start() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["choice1"],
        ChoicePhase::Cascading {
            choice_id: "choice1".to_string(),
            next: None,
        },
    );
    let (next, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceSelect",
        },
        &config(),
    );
    match actions.as_slice() {
        [Action::ResolveChoice { talk_id, id }] => {
            assert_eq!(*talk_id, TalkId(3));
            assert_eq!(id, "choice1");
        }
        _ => panic!("最終段 204 は ResolveChoice のみ（DD-4・Req5.3）"),
    }
    assert!(
        !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
        "最終段 204 で talk を起動しない（Req4.2）"
    );
    assert!(next.choice.is_none(), "解決で帳簿は消える");
    match next.phase {
        Phase::Steady {
            talk: Some(ActiveTalk { talk_id, .. }),
        } => assert_eq!(talk_id, TalkId(3), "現行 talk は維持される"),
        _ => panic!("expected Steady{{Some}} preserved"),
    }
    assert_eq!(next.next_talk_id, 6);
}

/// Req4.5・規則 3: 段の失敗は error 記録の上で 204 と同一遷移（残段ありなら次段）。
///
/// 本檻は steady 側の 204 相当処理そのものを `steady::step` の直接駆動で固定する（層を分けた
/// 単体檻）。`step()` 経由の end-to-end——横断 `Failed`→`Unloading{Fault}` アームの免除
/// （DD-12）が効いて終了系列へ倒れないこと——は
/// [`cascade_failed_via_step_does_not_fall_into_unloading_fault`] が固定する。
#[test]
fn cascade_failed_is_treated_as_no_content_stage_advance() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["choice1"],
        ChoicePhase::Cascading {
            choice_id: "choice1".to_string(),
            next: Some(CascadeNext::Select),
        },
    );
    let (next, actions) = super::step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(crate::msg::ShioriFailure::Timeout(
                "30s".to_string(),
            )),
            origin: "OnChoiceSelectEx",
        },
        &config(),
    );
    assert!(
        !matches!(next.phase, Phase::Unloading { .. }),
        "選択由来の失敗で終了系列へ倒れない（Req4.5）"
    );
    assert_eq!(actions.len(), 1);
    let (id, _) = expect_get_call(&actions[0]);
    assert_eq!(id, "OnChoiceSelect", "失敗は 204 と同一遷移＝次段へ前進");
}

/// Req4.5／5.3: 最終段の失敗も 204 と同一＝選択解決のみで会話を止めない。
#[test]
fn cascade_failed_at_last_stage_resolves_without_start() {
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["choice1"],
        ChoicePhase::Cascading {
            choice_id: "choice1".to_string(),
            next: None,
        },
    );
    let (next, actions) = super::step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(crate::msg::ShioriFailure::Ipc(
                "pipe closed".to_string(),
            )),
            origin: "OnChoiceSelect",
        },
        &config(),
    );
    assert!(!matches!(next.phase, Phase::Unloading { .. }));
    match actions.as_slice() {
        [Action::ResolveChoice { id, .. }] => assert_eq!(id, "choice1"),
        _ => panic!("最終段の失敗は ResolveChoice のみ（Req5.3）"),
    }
    assert!(next.choice.is_none());
}

// --- D. 完了状態（一回性・Req1.1／4.6／5.4） ---

/// 1 回の選択確定は高々 1 カスケード・高々 1 選択解決・高々 1 起動要求しか生じない（任意名形）。
#[test]
fn one_choice_yields_at_most_one_cascade_resolve_and_start() {
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
    let (s1, a1) = step(
        s,
        Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
        &config(),
    );
    let (s2, a2) = step(
        s1,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("script".to_string()),
            origin: "OnChoiceEvent",
        },
        &config(),
    );
    // 解決後に遅れて届く応答・遅延した選択確定はいずれも追加のカスケードを起こさない。
    let (s3, a3) = step(
        s2,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceSelect",
        },
        &config(),
    );
    let (s4, a4) = step(
        s3,
        Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
        &config(),
    );
    let all: Vec<&Action> = a1.iter().chain(&a2).chain(&a3).chain(&a4).collect();
    assert_eq!(
        all.iter()
            .filter(|a| matches!(a, Action::ShioriRequest(_)))
            .count(),
        1,
        "カスケードは高々 1 回（Req1.1）"
    );
    assert_eq!(
        all.iter()
            .filter(|a| matches!(a, Action::ResolveChoice { .. }))
            .count(),
        1,
        "選択解決は高々 1 回（Req5.4）"
    );
    assert_eq!(
        all.iter()
            .filter(|a| matches!(a, Action::StartTalk(_)))
            .count(),
        1,
        "起動要求は高々 1 つ（Req4.6）"
    );
    assert!(s4.choice.is_none(), "解決後に帳簿は復活しない");
}

/// 正典形の 2 段が両方 204 でも、選択解決はちょうど 1 回・起動要求は 0（Req2.3／4.2／5.3）。
#[test]
fn canonical_two_stage_204_yields_single_resolve_and_no_start() {
    let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
    let (s1, a1) = step(
        s,
        Input::Choice(choice_input_of("choice1", "ラベル", &[])),
        &config(),
    );
    let (s2, a2) = step(
        s1,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceSelectEx",
        },
        &config(),
    );
    let (s3, a3) = step(
        s2,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceSelect",
        },
        &config(),
    );
    let all: Vec<&Action> = a1.iter().chain(&a2).chain(&a3).collect();
    let gets: Vec<String> = all
        .iter()
        .filter(|a| matches!(a, Action::ShioriRequest(_)))
        .map(|a| expect_get_call(a).0)
        .collect();
    assert_eq!(
        gets,
        vec!["OnChoiceSelectEx".to_string(), "OnChoiceSelect".to_string()],
        "Ex 先行→無印後続の 2 段（Req2.2／2.3）"
    );
    assert_eq!(
        all.iter()
            .filter(|a| matches!(a, Action::ResolveChoice { .. }))
            .count(),
        1,
        "選択解決はちょうど 1 回（Req5.3／5.4）"
    );
    assert!(
        !all.iter().any(|a| matches!(a, Action::StartTalk(_))),
        "全段 204 では起動要求を生じない（Req4.2）"
    );
    assert!(s3.choice.is_none());
}

/// Req2.4: 先行段が応答スクリプトを返したら以降の段を発行しない（正典形の短絡）。
#[test]
fn canonical_value_at_first_stage_skips_the_remaining_stage() {
    let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
    let (s1, _) = step(
        s,
        Input::Choice(choice_input_of("choice1", "ラベル", &[])),
        &config(),
    );
    let (s2, a2) = step(
        s1,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("script".to_string()),
            origin: "OnChoiceSelectEx",
        },
        &config(),
    );
    assert!(
        !a2.iter().any(|a| matches!(a, Action::ShioriRequest(_))),
        "Value を返した段の後に無印段を発行しない（Req2.4）"
    );
    assert!(matches!(
        a2.as_slice(),
        [Action::ResolveChoice { .. }, Action::StartTalk(_)]
    ));
    assert!(s2.choice.is_none());
}

// --- E. 既存挙動の保存（DD-6 防御アームへ choice 応答が到達しないこと） ---

/// C4 Implementation Notes: choice 応答は先行アームで捌かれ `steady_value_during_talk`
/// （DD-6 防御）へ**到達しない**。到達すると選択応答が warn 破棄で沈黙する（既知の罠）。
#[test]
fn cascade_reply_does_not_reach_the_dd6_defense_arm() {
    let cfg = config();
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["OnMenu"],
        ChoicePhase::Cascading {
            choice_id: "OnMenu".to_string(),
            next: None,
        },
    );
    let mut actions = Vec::new();
    let ev = crate::schedule::log_capture::capture(|| {
        let (_next, a) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("script".to_string()),
                origin: "OnChoiceEvent",
            },
            &cfg,
        );
        actions = a;
    });
    assert!(
        !ev.iter()
            .any(|e| e.event.as_deref() == Some("steady_value_during_talk")),
        "choice 応答が DD-6 防御アームへ落ちてはならない。\n捕捉={ev:#?}"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
        "choice 応答は先行アームで置換起動される"
    );
}

/// Req6.1／6.3／6.4・裁定 6: 選択待ち中の周期リクエストは **NOTIFY**（Ref3=`"0"`）で送出され、
/// `Status` に複合値 `talking,choosing` が**正典順**で載る。
///
/// NOTIFY は応答スクリプトを運べない型であり（[`ShioriOutcome::Notified`] のみ）、選択待ち中の
/// 自発トーク抑止は既存 pump 分岐の構造だけで成立する（Req6.5・新しい抑止機構を作らない）。
#[test]
fn tick_during_choice_waiting_notifies_with_talking_and_choosing() {
    let now = MonotonicMs(1_000);
    let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
    let (next, actions) = step(s, Input::Tick { now }, &config());
    assert_eq!(actions.len(), 1, "選択待ち中も周期イベントは 1 件発行する");
    match &actions[0] {
        Action::ShioriRequest(ShioriCall::Notify {
            id,
            references,
            status,
        }) => {
            assert_eq!(id.as_str(), "OnSecondChange");
            assert_eq!(
                references[3], "0",
                "選択待ち中も再生中扱い＝再生可否 Reference は \"0\"（Req6.4）"
            );
            assert_eq!(
                status.render(),
                Some("talking,choosing".to_string()),
                "複合値は正典順で連結される（Req6.1／6.3・裁定 6）"
            );
        }
        _ => panic!("選択待ち中の周期イベントは NOTIFY で送出される（Req6.4／6.5）"),
    }
    assert!(
        matches!(next.phase, Phase::Steady { talk: Some(_) }),
        "選択待ち中も slot 占有（Steady{{Some}}）が維持される（Req6.4）"
    );
    assert!(
        matches!(expect_ledger(&next).phase, ChoicePhase::Waiting),
        "pump は選択帳簿を触らない"
    );
}

/// Req6.2: 選択が解決して帳簿が消えた後の周期リクエストからは `choosing` が消える。
///
/// 実解決経路（カスケード最終段 204 → `ResolveChoice` 発行・帳簿消去）を通してから pump を
/// 採る——帳簿を手で消すのではなく、解決の実装が `choosing` を落とすことを固定する。
#[test]
fn tick_after_choice_resolution_drops_choosing() {
    let cfg = config();
    let s = steady_with_ledger(
        TalkId(3),
        6,
        &["OnMenu"],
        ChoicePhase::Cascading {
            choice_id: "OnMenu".to_string(),
            next: None,
        },
    );
    let (resolved, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceEvent",
        },
        &cfg,
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ResolveChoice { .. })),
        "最終段 204 は選択解決を発行する"
    );
    assert!(resolved.choice.is_none(), "解決で帳簿が消える（Req6.2 の源）");
    let (_next, tick_actions) = step(
        resolved,
        Input::Tick {
            now: MonotonicMs(1_000),
        },
        &cfg,
    );
    assert_eq!(
        status_wire(&tick_actions[0]),
        Some("talking".to_string()),
        "解決後は choosing が消え talking のみが残る（Req6.2）"
    );
}

/// Req6.1・C5: **カスケード各段の GET も** 選択待ち継続中として `choosing` を帯びる。
///
/// `on_choice`／`on_cascade_reply` は検証・分解のため帳簿を `State` から取り出した状態で
/// スナップショットを採る。`State::snapshot` をそのまま呼ぶと当該 2 点だけ `choosing` が
/// 落ちるため、両段の wire 値を実値で突合して固定する。
#[test]
fn cascade_stage_gets_carry_choosing() {
    let cfg = config();
    // 第 1 段（on_choice: 帳簿を take 済みの状態で採るスナップショット）。
    let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
    let (next, stage1) = step(
        s,
        Input::Choice(choice_input_of("choice1", "ラベル", &[])),
        &cfg,
    );
    let (stage1_id, _) = expect_get_call(&stage1[0]);
    assert_eq!(stage1_id, "OnChoiceSelectEx", "正典形の先行段");
    assert_eq!(
        status_wire(&stage1[0]),
        Some("talking,choosing".to_string()),
        "第 1 段の GET に choosing が載る（C5）"
    );
    // 第 2 段（on_cascade_reply: 帳簿を分解済みの状態で採るスナップショット）。
    let (_next2, stage2) = step(
        next,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnChoiceSelectEx",
        },
        &cfg,
    );
    let (stage2_id, _) = expect_get_call(&stage2[0]);
    assert_eq!(stage2_id, "OnChoiceSelect", "残段（無印）へ前進している");
    assert_eq!(
        status_wire(&stage2[0]),
        Some("talking,choosing".to_string()),
        "次段の GET にも choosing が載る（C5）"
    );
}
