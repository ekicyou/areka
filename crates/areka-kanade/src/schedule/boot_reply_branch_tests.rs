use super::test_support::{
    assert_get, assert_notify, config, drive_to_main, drive_to_prefetch, reply,
};
use super::*;
use crate::schedule::{ActiveTalk, step};
use crate::status::ExecutionSnapshot;
use crate::talk::TalkId;

// ========================================================================
// username リソース照会 prefetch（OnInitialize 後・OnFirstBoot 前・R4.1/R9.3）
// ========================================================================

use crate::msg::ShioriFailure;
use resources::ResourceOutcome;

/// Action 列から `Action::ResourceOutcome` の outcome を取り出す（無ければ panic）。
fn resource_outcome_of(actions: &[Action]) -> &ResourceOutcome {
    actions
        .iter()
        .find_map(|a| match a {
            Action::ResourceOutcome { id, outcome } => {
                assert_eq!(*id, "username", "リソース照会 id は username");
                Some(outcome)
            }
            _ => None,
        })
        .expect("Action::ResourceOutcome が発行されるはず")
}

/// prefetch は OnInitialize（NOTIFY）の後・OnFirstBoot（GET）の前に username GET を 1 回だけ発行し、
/// 応答受領後は OnFirstBoot GET を発行して BootType へ進む（順序の檻・R4.1）。
#[test]
fn prefetch_username_get_is_issued_once_between_initialize_and_firstboot() {
    let cfg = config();
    // Idle+Boot → OnInitialize NOTIFY（username GET はまだ出ない）。
    let (s, actions) = step(State::initial(), Input::Boot, &cfg);
    assert_notify(
        &actions[0],
        &events::on_initialize(&ExecutionSnapshot::INACTIVE),
    );
    assert!(
        !actions.iter().any(is_username_get),
        "OnInitialize 段では username GET を発行しない"
    );

    // BootInit+Notified → username GET（OnFirstBoot はまだ出ない）。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
        &cfg,
    );
    assert!(matches!(s.phase, Phase::BootPrefetch));
    assert_eq!(
        actions.iter().filter(|a| is_username_get(a)).count(),
        1,
        "prefetch の username GET はちょうど 1 回"
    );
    assert!(
        !actions.iter().any(is_onfirstboot_get),
        "prefetch 応答前に OnFirstBoot を発行してはならない（照会後に続行）"
    );

    // BootPrefetch+NoContent → [ResourceOutcome, OnFirstBoot GET]・BootType。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "test",
        },
        &cfg,
    );
    assert!(
        matches!(s.phase, Phase::BootType),
        "prefetch 完了後は BootType（OnFirstBoot 待ち）"
    );
    // sink 呼出指示（ResourceOutcome）が OnFirstBoot GET より前に積まれる（design boot 図: sink 先行）。
    assert!(
        matches!(actions[0], Action::ResourceOutcome { .. }),
        "ResourceOutcome は OnFirstBoot より前（sink 先行）"
    );
    assert_eq!(
        actions.iter().filter(|a| is_onfirstboot_get(a)).count(),
        1,
        "prefetch 応答後に OnFirstBoot GET をちょうど 1 回発行する"
    );
    // 二度目の username GET は出ない（prefetch は 1 回・後段で再照会しない）。
    assert!(
        !actions.iter().any(is_username_get),
        "prefetch は 1 回のみ——後段で username を再照会しない"
    );
}

/// 応答の [`ResourceOutcome`] 写像: 200 Value→Value(body)／204→NoContent／失敗→Failed（R4.1）。
#[test]
fn prefetch_maps_outcomes_to_resource_outcome() {
    let cfg = config();

    // 200 Value → ResourceOutcome::Value(body)。
    let (_, actions) = step(
        drive_to_prefetch(&cfg),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value("bob".to_string(), areka_talk::SecurityLevel::Local),
            origin: "test",
        },
        &cfg,
    );
    assert_eq!(
        resource_outcome_of(&actions),
        &ResourceOutcome::Value("bob".to_string())
    );

    // 204 → ResourceOutcome::NoContent。
    let (_, actions) = step(
        drive_to_prefetch(&cfg),
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "test",
        },
        &cfg,
    );
    assert_eq!(resource_outcome_of(&actions), &ResourceOutcome::NoContent);

    // 失敗（タイムアウト）→ ResourceOutcome::Failed（理由文字列を保持）。
    let (_, actions) = step(
        drive_to_prefetch(&cfg),
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            origin: "test",
        },
        &cfg,
    );
    match resource_outcome_of(&actions) {
        ResourceOutcome::Failed(reason) => {
            assert!(
                reason.contains("timeout"),
                "Failed の理由に失敗語彙が載る: {reason}"
            );
        }
        other => panic!("失敗応答は ResourceOutcome::Failed へ写像されるべき: {other:?}"),
    }
}

/// 照会失敗（タイムアウト/IPC 断）でも boot は殺さず OnFirstBoot へ続行する（起動を殺さない・R4.1）。
/// `step()`（mod.rs 横断アーム込み）を通し、BootPrefetch の Failed が Unloading{Fault} へ**倒れない**
/// ことを固定する（横断 Failed→Fault 経路が prefetch では迂回される檻）。
#[test]
fn prefetch_failure_continues_boot_not_fault() {
    let cfg = config();
    let (s, actions) = step(
        drive_to_prefetch(&cfg),
        Input::ShioriReply {
            outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
            origin: "test",
        },
        &cfg,
    );
    // 終了系列（Unloading/Stopped）へ倒れず、OnFirstBoot 待ち（BootType）へ続行する。
    assert!(
        matches!(s.phase, Phase::BootType),
        "prefetch 失敗は Unloading{{Fault}} へ倒さず OnFirstBoot へ続行する（起動を殺さない・R4.1）"
    );
    assert_eq!(
        resource_outcome_of(&actions),
        &ResourceOutcome::Failed("shiori ipc failure: pipe closed".to_string())
    );
    // OnFirstBoot GET が発行され boot が前進する（Unload ではない）。
    assert!(
        actions.iter().any(is_onfirstboot_get),
        "prefetch 失敗後も OnFirstBoot を発行して boot を継続する"
    );
    assert!(
        !actions.iter().any(|a| matches!(a, Action::ShioriUnload)),
        "prefetch 失敗で Unload（終了系列）を起こしてはならない"
    );
}

/// リソース照会は talk を生成しない（Invariant）: Value 応答でも StartTalk を発行しない。
#[test]
fn prefetch_value_does_not_produce_a_talk() {
    let cfg = config();
    let (_, actions) = step(
        drive_to_prefetch(&cfg),
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(
                r"\0greeting\e".to_string(),
                areka_talk::SecurityLevel::Local,
            ),
            origin: "test",
        },
        &cfg,
    );
    assert!(
        !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
        "リソース照会 Value は StartTalk を生成しない（sink へ渡すのみ・Invariant）"
    );
}

/// 完了固定ログ（R9.3 grep 証跡）: 経路（value/no_content/failed）ごとに `info!` が
/// `target="areka_kanade::resource"`・`outcome=<label>`・message 固定で**ちょうど 1 回**出る。
#[test]
fn prefetch_emits_fixed_completion_log_exactly_once() {
    use crate::schedule::log_capture::{assert_resource_prefetch_logged_once, capture};

    let cases: [(ShioriOutcome, &str); 3] = [
        (
            ShioriOutcome::Value("bob".to_string(), areka_talk::SecurityLevel::Local),
            "value",
        ),
        (ShioriOutcome::NoContent, "no_content"),
        (
            ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
            "failed",
        ),
    ];
    for (outcome, label) in cases {
        let cfg = config();
        let s = drive_to_prefetch(&cfg);
        let events = capture(|| {
            let _ = step(
                s,
                Input::ShioriReply {
                    outcome,
                    origin: "test",
                },
                &cfg,
            );
        });
        assert_resource_prefetch_logged_once(&events, label);
    }
}

/// Action が username リソース GET（id="username"）か判定する。
fn is_username_get(action: &Action) -> bool {
    matches!(
        action,
        Action::ShioriRequest(crate::msg::ShioriCall::Get { id, .. }) if id.as_str() == "username"
    )
}

/// Action が OnFirstBoot GET か判定する。
fn is_onfirstboot_get(action: &Action) -> bool {
    matches!(
        action,
        Action::ShioriRequest(crate::msg::ShioriCall::Get { id, .. }) if id.as_str() == "OnFirstBoot"
    )
}

/// Action が OnBoot GET か判定する。
fn is_onboot_get(action: &Action) -> bool {
    matches!(
        action,
        Action::ShioriRequest(crate::msg::ShioriCall::Get { id, .. }) if id.as_str() == "OnBoot"
    )
}

// ========================================================================
// 初回ゲート分岐（3.1-3.4）＋ epilogue 添付（design C9）
// ========================================================================

use crate::talk::EpilogueCommand;

/// 2 回目以降起動（起動記録あり）を表す config（first_boot=false）。
fn config_not_first_boot() -> KanadeConfig {
    let mut cfg = config();
    cfg.first_boot = false;
    cfg
}

/// 初回起動（記録なし）＋起動記録書込 epilogue を持つ config（SET cue 1 件）。
fn config_with_epilogue() -> KanadeConfig {
    let mut cfg = config();
    cfg.first_boot_epilogue = vec![EpilogueCommand {
        name: "areka.prop.set".to_string(),
        tokens: vec!["areka.boot.count".to_string(), "1".to_string()],
    }];
    cfg
}

/// Action 列から `Action::StartTalk` の中身を取り出す（無ければ panic）。
fn start_talk_of(actions: &[Action]) -> &StartTalk {
    actions
        .iter()
        .find_map(|a| match a {
            Action::StartTalk(st) => Some(st),
            _ => None,
        })
        .expect("Action::StartTalk が発行されるはず")
}

/// 3.3: 起動記録あり（first_boot=false）→ OnFirstBoot をスキップし OnBoot（BootMain）から起動運行を始める。
/// prefetch 段は不変（3.5）——照会応答後の分岐のみが分岐する。`boot_gate skip_first_boot` を info で残す。
#[test]
fn prefetch_first_boot_false_skips_onfirstboot_and_starts_from_onboot() {
    use crate::schedule::log_capture::{assert_logged, capture};
    use tracing::Level;

    let cfg = config_not_first_boot();
    let s = drive_to_prefetch(&cfg);

    // BootPrefetch + 応答（204）→ gate 分岐（first_boot=false）。
    let mut phase_is_main = false;
    let mut actions_out: Vec<Action> = Vec::new();
    let events = capture(|| {
        let (next, actions) = reply(s, ShioriOutcome::NoContent, &cfg);
        phase_is_main = matches!(next.phase, Phase::BootMain);
        actions_out = actions;
    });

    // OnFirstBoot・BootType を飛ばして BootMain 直行（3.3）。
    assert!(
        phase_is_main,
        "first_boot=false は BootType を飛ばし BootMain へ直行する（OnFirstBoot スキップ・3.3）"
    );
    // sink 呼出（ResourceOutcome）は依然として OnBoot GET より先（sink 先行を保存）。
    assert!(
        matches!(actions_out[0], Action::ResourceOutcome { .. }),
        "ResourceOutcome は request より前（sink 先行）"
    );
    // OnFirstBoot GET は一切発行されない。
    assert!(
        !actions_out.iter().any(is_onfirstboot_get),
        "first_boot=false では OnFirstBoot を発行しない（3.3）"
    );
    // 代わりに OnBoot GET をちょうど 1 回発行して起動運行を始める。
    assert_eq!(
        actions_out.iter().filter(|a| is_onboot_get(a)).count(),
        1,
        "first_boot=false は OnBoot GET を発行して BootMain から始める（3.3）"
    );
    // grep 証跡: boot_gate skip_first_boot。
    assert_logged(&events, Level::INFO, "boot_gate");
}

/// 3.2: first_boot=true の 204 フォールスルーは不変（BootType 204 → OnBoot GET → BootMain）。
#[test]
fn first_boot_true_204_falls_through_to_onboot() {
    let cfg = config(); // 既定 first_boot=true
    let s = drive_to_prefetch(&cfg);
    // BootPrefetch 204 → OnFirstBoot GET / BootType（従来どおり）。
    let (s, actions) = reply(s, ShioriOutcome::NoContent, &cfg);
    assert!(
        matches!(s.phase, Phase::BootType),
        "first_boot=true は BootType（OnFirstBoot 待ち）へ"
    );
    assert!(
        actions.iter().any(is_onfirstboot_get),
        "first_boot=true は OnFirstBoot GET を発行する（3.1）"
    );
    // BootType 204 → OnBoot GET / BootMain（204 フォールスルー・3.2）。
    let (s, actions) = reply(s, ShioriOutcome::NoContent, &cfg);
    assert!(
        matches!(s.phase, Phase::BootMain),
        "OnFirstBoot 204 は OnBoot へフォールスルー（3.2）"
    );
    assert_eq!(
        actions.iter().filter(|a| is_onboot_get(a)).count(),
        1,
        "204 フォールスルーで OnBoot GET を 1 回発行する（3.2）"
    );
}

/// 4.1: OnFirstBoot の Ref0 は `config.vanish_count` 由来（従来 literal 0 の置換）。
#[test]
fn onfirstboot_ref0_reflects_config_vanish_count() {
    let mut cfg = config();
    cfg.vanish_count = 7;
    let s = drive_to_prefetch(&cfg);
    let (_, actions) = reply(s, ShioriOutcome::NoContent, &cfg);
    // 発行された OnFirstBoot GET は events::on_first_boot(_, 7) と一致する（Ref0="7"）。
    let onfirstboot = actions
        .iter()
        .find(|a| is_onfirstboot_get(a))
        .expect("first_boot=true は OnFirstBoot GET を発行する");
    assert_get(
        onfirstboot,
        &events::on_first_boot(&ExecutionSnapshot::INACTIVE, 7),
    );
}

/// design C9 Some アーム: 挨拶 talk は `config.first_boot_epilogue` を添付して起動する。
/// 追跡の枠（`ActiveTalk.script`）も自ら作った script を保持する（DD-10・`OnChoiceTimeout` Ref0 の供給源）。
#[test]
fn boot_greeting_talk_carries_epilogue() {
    let cfg = config_with_epilogue();
    let (s, actions) = reply(
        drive_to_main(&cfg),
        ShioriOutcome::Value("greeting".to_string(), areka_talk::SecurityLevel::Local),
        &cfg,
    );
    let st = start_talk_of(&actions);
    assert_eq!(st.script, "greeting");
    assert_eq!(
        st.epilogue, cfg.first_boot_epilogue,
        "Value 経路の StartTalk は first_boot_epilogue を添付する（design C9 Some アーム）"
    );
    assert!(
        matches!(
            &s.phase,
            Phase::BootVersion { talk: Some(active) } if active.script == "greeting"
        ),
        "挨拶 talk の script が ActiveTalk へ保持される（DD-10）"
    );
}

/// design C9 None アーム（epilogue 非空・204-204）: 挨拶トーク皆無でも epilogue-only StartTalk
/// （空 script・talk_id 採番・`BootVersion{talk: Some}` で正規追跡）を発行して起動記録を書く。
#[test]
fn first_boot_204_204_with_epilogue_emits_epilogue_only_tracked_talk() {
    let cfg = config_with_epilogue(); // first_boot=true・epilogue 非空
    // Idle→…→BootMain（prefetch 204 → OnFirstBoot 204）。
    let s = drive_to_main(&cfg);

    // BootMain 204（トーク皆無）＋epilogue 非空 → epilogue-only StartTalk。
    let (s, actions) = reply(s, ShioriOutcome::NoContent, &cfg);
    let st = start_talk_of(&actions);
    assert_eq!(st.script, "", "epilogue-only talk の script は空");
    assert_eq!(
        st.talk_id,
        TalkId(1),
        "epilogue-only talk も talk_id を採番する"
    );
    assert_eq!(
        st.epilogue, cfg.first_boot_epilogue,
        "epilogue-only talk は first_boot_epilogue を運ぶ"
    );
    // 正規追跡（Some）——即時完走で記録が書かれる経路（DD-IT-12 と同じ slot）。
    assert!(
        matches!(
            s.phase,
            Phase::BootVersion {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(1),
                    origin: "boot",
                    ref script,
                })
            } if script.is_empty()
        ),
        "epilogue-only talk は BootVersion{{talk: Some(origin=boot)}} で正規追跡され、空 script も同じ規律で保持される"
    );
    assert_eq!(
        s.next_talk_id, 2,
        "epilogue-only talk も採番カウンタを進める"
    );
}

/// design C9 None アーム（epilogue 空・通常 204）: 従来どおり StartTalk なし・`BootVersion{talk: None}`。
#[test]
fn normal_204_with_empty_epilogue_emits_no_talk() {
    let cfg = config(); // 既定 epilogue 空
    let (s, actions) = reply(drive_to_main(&cfg), ShioriOutcome::NoContent, &cfg);
    assert!(
        matches!(s.phase, Phase::BootVersion { talk: None }),
        "epilogue 空の 204 は従来どおり talk なし（BootVersion{{talk: None}}）"
    );
    assert!(
        !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
        "epilogue 空の 204 は StartTalk を発行しない（既存挙動不変）"
    );
    assert_eq!(
        s.next_talk_id, 1,
        "epilogue 空の 204 は採番カウンタを進めない"
    );
}
