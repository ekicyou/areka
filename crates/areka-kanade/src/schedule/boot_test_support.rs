use super::*;

pub(super) fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// boot 進行は `State::initial()` を起点に `step()` 経由で駆動する（統合貫通テスト）。
pub(super) fn initial() -> State {
    State::initial()
}

/// Action が期待の GET（id・references が events:: と一致）であることを検証する。
pub(super) fn assert_get(action: &Action, expected: &crate::msg::ShioriCall) {
    match (action, expected) {
        (
            Action::ShioriRequest(crate::msg::ShioriCall::Get { id, references, .. }),
            crate::msg::ShioriCall::Get {
                id: eid,
                references: erefs,
                ..
            },
        ) => {
            assert_eq!(id, eid, "GET id 不一致");
            assert_eq!(references, erefs, "GET references 不一致");
        }
        _ => panic!("expected ShioriRequest(Get) matching events output"),
    }
}

/// Action が期待の NOTIFY（id・references が events:: と一致）であることを検証する。
pub(super) fn assert_notify(action: &Action, expected: &crate::msg::ShioriCall) {
    match (action, expected) {
        (
            Action::ShioriRequest(crate::msg::ShioriCall::Notify { id, references, .. }),
            crate::msg::ShioriCall::Notify {
                id: eid,
                references: erefs,
                ..
            },
        ) => {
            assert_eq!(id, eid, "NOTIFY id 不一致");
            assert_eq!(references, erefs, "NOTIFY references 不一致");
        }
        _ => panic!("expected ShioriRequest(Notify) matching events output"),
    }
}

/// 起動挨拶（talk_id=1）を追跡したまま基盤バージョン通知の応答待ち（`BootVersion{talk: Some(id=1)}`）
/// まで boot を進める（`pending` があれば `Boot` 直後に close 指示を挟んで保留させる）。
///
/// 戻る前に、相が「応答待ち・追跡あり（id=1）」であることと、保留された close 指示が `pending` と
/// 一致することを表明する（`CloseReason` は `PartialEq` を持たないので綴りで比べる）。
pub(super) fn boot_until_version_with_greeting(
    cfg: &KanadeConfig,
    pending: Option<CloseReason>,
) -> State {
    use crate::schedule::step;
    let reply = |s, outcome| {
        step(
            s,
            Input::ShioriReply {
                outcome,
                origin: "test",
            },
            cfg,
        )
        .0
    };

    let (mut s, _) = step(initial(), Input::Boot, cfg); // Idle→BootInit（OnInitialize NOTIFY）
    if let Some(reason) = pending {
        s = step(s, Input::CloseRequest { reason }, cfg).0; // BootInit のまま close を保留記録
    }
    let s = reply(s, ShioriOutcome::Notified); // BootInit→BootPrefetch（username GET）
    let s = reply(s, ShioriOutcome::NoContent); // BootPrefetch→BootType（初回起動＝OnFirstBoot GET）
    let s = reply(s, ShioriOutcome::Value("greeting".to_string())); // BootType(Value)→BootVersion{talk: Some(id=1)}（OnBoot を飛ばす）

    assert!(
        matches!(
            s.phase,
            Phase::BootVersion {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(1),
                    ..
                })
            }
        ),
        "挨拶 talk（id=1）を追跡した BootVersion のはず"
    );
    assert_eq!(
        s.pending_close.map(CloseReason::as_ref_str),
        pending.map(CloseReason::as_ref_str),
        "保留された close 指示が引数と一致するはず"
    );
    s
}
