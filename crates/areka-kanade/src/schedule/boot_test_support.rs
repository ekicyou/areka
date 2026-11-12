use super::*;
use crate::schedule::step;
use crate::schedule::translate_test_support::pass_translate;
use crate::status::ExecutionSnapshot;
use crate::talk::TalkId;

pub(super) use crate::schedule::translate_test_support::config;

/// 起動記録の有無（`first_boot`）と起動の由来（`boot_origin`）を差し替えた config。
pub(super) fn cfg(first_boot: bool, boot_origin: crate::change::BootOrigin) -> KanadeConfig {
    let mut c = config();
    c.first_boot = first_boot;
    c.boot_origin = boot_origin;
    c
}

/// Action 列に含まれる GET の ID を順に並べる。
pub(super) fn get_ids<'a>(actions: impl IntoIterator<Item = &'a Action>) -> Vec<String> {
    actions
        .into_iter()
        .filter_map(|a| match a {
            Action::ShioriRequest(crate::msg::ShioriCall::Get { id, .. }) => {
                Some(id.as_str().to_string())
            }
            _ => None,
        })
        .collect()
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

/// SHIORI の応答を入れる（翻訳の行動が出たら、`OnTranslate` が台詞をそのまま返したときの続きを返す）。
///
/// 実の `step` で進めた状態なら、台詞の応答は元のイベントの控えがあるので必ず翻訳を経る。翻訳を
/// 経ずに台詞の再生が出たら、控えの無い状態を手で組んだ誤りとして落とす。
pub(super) fn reply(s: State, outcome: ShioriOutcome, c: &KanadeConfig) -> (State, Vec<Action>) {
    let input = Input::ShioriReply {
        outcome,
        origin: "test",
    };
    let out = step(s, input, c);
    assert!(
        !out.1
            .iter()
            .any(|a| matches!(a, Action::StartTalk(st) if !st.script.is_empty())),
        "台詞の再生は翻訳（OnTranslate）を経るはず"
    );
    pass_translate(out, c)
}

/// Idle→Boot→（OnInitialize 完了）まで進め、`BootPrefetch`（username GET 発行済み）へ着かせる。
pub(super) fn drive_to_prefetch(c: &KanadeConfig) -> State {
    let (s, _) = step(State::initial(), Input::Boot, c); // Idle→BootInit（OnInitialize NOTIFY）
    let (s, actions) = reply(s, ShioriOutcome::Notified, c); // BootInit→BootPrefetch（username GET）
    assert!(
        matches!(s.phase, Phase::BootPrefetch),
        "OnInitialize 完了後は BootPrefetch（username 照会待ち）であるべき"
    );
    // BootInit の応答は username GET を 1 件だけ発行する（OnFirstBoot ではない）。
    assert_eq!(actions.len(), 1, "prefetch は GET を 1 件だけ発行する");
    assert_get(
        &actions[0],
        &resources::resource_get("username", &ExecutionSnapshot::INACTIVE),
    );
    s
}

/// `BootMain`（OnBoot GET の応答待ち）まで実の `step` で進める（初回起動なら OnFirstBoot に 204 を返して通る）。
pub(super) fn drive_to_main(c: &KanadeConfig) -> State {
    let mut s = reply(drive_to_prefetch(c), ShioriOutcome::NoContent, c).0;
    if matches!(s.phase, Phase::BootType) {
        s = reply(s, ShioriOutcome::NoContent, c).0;
    }
    assert!(
        matches!(s.phase, Phase::BootMain),
        "OnBoot の応答待ちのはず"
    );
    s
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
    let (mut s, _) = step(State::initial(), Input::Boot, cfg); // Idle→BootInit（OnInitialize NOTIFY）
    if let Some(reason) = pending {
        s = step(s, Input::CloseRequest { reason }, cfg).0; // BootInit のまま close を保留記録
    }
    let s = reply(s, ShioriOutcome::Notified, cfg).0; // BootInit→BootPrefetch（username GET）
    let s = reply(s, ShioriOutcome::NoContent, cfg).0; // BootPrefetch→BootType（初回起動＝OnFirstBoot GET）
    let s = reply(
        s,
        ShioriOutcome::Value("greeting".to_string(), areka_talk::SecurityLevel::Local),
        cfg,
    )
    .0; // BootType(Value)→翻訳→BootVersion{talk: Some(id=1)}（OnBoot を飛ばす）

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
