//! アンカーの受理と 2 段の送出を、最上位の `step` から通すテスト
//! （areka-P0-anchor-tag-canon 要件 4.1〜4.6・4.8〜4.11・8.3）。
//!
//! 知らせ（`Input::Anchor`）を 1 件入れ、返った GET に模擬の応答（`Input::ShioriReply`）を順に
//! 入れ直して、送った GET の列と始めた台詞を数える。実 SHIORI は使わない。期待はすべて手書き。

use super::*;
use crate::msg::CloseReason;
use crate::schedule::log_capture::assert_not_logged;
use crate::schedule::translate_test_support::pass_translate;
use crate::schedule::{ChoicePhase, ChoiceState, TermCause, phase_label};

/// 1 回の知らせと、それに続く応答の列を通した結果。
struct Chain {
    state: State,
    /// 送った GET の（イベント名, Reference の並び）を順に。
    gets: Vec<(String, Vec<String>)>,
    /// 送った GET の Status の行を順に（行が無ければ `None`）。
    statuses: Vec<Option<String>>,
    /// 始めた台詞の番号を順に。
    talks: Vec<TalkId>,
}

/// 知らせを 1 件入れ、応答を順に入れ直す。応答は、直前の一括が GET で終わったときにだけ入れる
/// （殻は GET の結果しか入れ直さない）。GET と台詞の起動のほかの行動が出たら失敗させる。
fn run(start: State, input: AnchorInput, replies: Vec<(ShioriOutcome, &'static str)>) -> Chain {
    let (mut state, mut batch) = step(start, Input::Anchor(input), &config());
    let (mut gets, mut statuses, mut talks) = (Vec::new(), Vec::new(), Vec::new());
    let mut replies = replies.into_iter();
    loop {
        let waiting = matches!(
            batch.last(),
            Some(Action::ShioriRequest(ShioriCall::Get { .. }))
        );
        for action in batch {
            match action {
                Action::ShioriRequest(ShioriCall::Get {
                    id,
                    references,
                    status,
                }) => {
                    gets.push((id.as_str().to_string(), references));
                    statuses.push(status.render());
                }
                Action::StartTalk(start) => talks.push(start.talk_id),
                _ => panic!("アンカーの連鎖に無いはずの行動（終了の往復など）"),
            }
        }
        let Some((outcome, origin)) = replies.next() else {
            break;
        };
        assert!(waiting, "応答を待つ GET が無いのに、応答が残っている");
        // 台本の応答は、翻訳を台詞そのままで通して台詞の起動まで進める。
        (state, batch) = pass_translate(
            step(state, Input::ShioriReply { outcome, origin }, &config()),
            &config(),
        );
    }
    Chain {
        state,
        gets,
        statuses,
        talks,
    }
}

/// 期待する GET 1 本（イベント名, Reference の並び）。
fn get(name: &str, references: &[&str]) -> (String, Vec<String>) {
    (
        name.to_string(),
        references.iter().map(|s| s.to_string()).collect(),
    )
}

fn no_content(origin: &'static str) -> (ShioriOutcome, &'static str) {
    (ShioriOutcome::NoContent, origin)
}

fn script(origin: &'static str) -> (ShioriOutcome, &'static str) {
    (ShioriOutcome::Value("\\0やあ\\e".to_string()), origin)
}

/// 話している最中（トーク 3）で、選択待ちの帳簿もある。
fn steady_talking_with_choice() -> State {
    State {
        choice: Some(ChoiceState {
            talk_id: TalkId(3),
            candidates: vec!["はい".to_string(), "いいえ".to_string()],
            deadline: None,
            phase: ChoicePhase::Waiting,
        }),
        ..steady_talking()
    }
}

/// 再生中の台詞がトーク 3 のまま。
fn still_talk_3(state: &State) -> bool {
    matches!(&state.phase, Phase::Steady { talk: Some(talk) } if talk.talk_id == TalkId(3))
}

/// 選択待ちの帳簿が元のまま残っている。
fn choice_ledger_kept(state: &State) -> bool {
    state.choice.as_ref().is_some_and(|ledger| {
        ledger.talk_id == TalkId(3)
            && ledger.candidates == ["はい", "いいえ"]
            && matches!(ledger.phase, ChoicePhase::Waiting)
    })
}

// --- イベントの列と Reference ---

/// 要件 4.1・4.2・4.6・4.9: `On` で始まらない ID は `OnAnchorSelectEx`（文字・ID・引数の順）→ 204 →
/// `OnAnchorSelect`（ID だけ）→ 204 で何も起きない。引数が無ければ Reference1 より後ろの位置は無い。
#[test]
fn canonical_id_sends_select_ex_then_select_and_then_nothing() {
    let both_empty = vec![no_content("OnAnchorSelectEx"), no_content("OnAnchorSelect")];
    let chain = run(steady_none(), anchor("詳細", "くわしく", &[]), both_empty);
    assert_eq!(
        chain.gets,
        [
            get("OnAnchorSelectEx", &["くわしく", "詳細"]),
            get("OnAnchorSelect", &["詳細"]),
        ]
    );
    assert!(chain.talks.is_empty());
    assert!(matches!(chain.state.phase, Phase::Steady { talk: None }));
    assert!(chain.state.anchor.is_none());
    assert_eq!(chain.state.next_talk_id, 7, "台詞の番号を使っていない");

    // 引数は ID の後ろに書いた順で載る（空の引数も位置を保つ）。続きの `OnAnchorSelect` には載らない。
    let both_empty = vec![no_content("OnAnchorSelectEx"), no_content("OnAnchorSelect")];
    let chain = run(
        steady_none(),
        anchor("詳細", "くわしく", &["a,b", ""]),
        both_empty,
    );
    assert_eq!(
        chain.gets,
        [
            get("OnAnchorSelectEx", &["くわしく", "詳細", "a,b", ""]),
            get("OnAnchorSelect", &["詳細"]),
        ]
    );
}

/// 要件 4.4〜4.6: `On` で始まる ID はその名前のイベント 1 本だけ。Reference は引数だけで、範囲の
/// 文字も ID も載らない。`OnAnchorSelectEx`／`OnAnchorSelect` は列のどこにも無い。
#[test]
fn on_prefixed_id_sends_only_the_event_of_that_name() {
    for (references, expected) in [
        (&["r0", "r1"][..], get("Onメニューを開く", &["r0", "r1"])),
        (&[][..], get("Onメニューを開く", &[])),
    ] {
        let chain = run(
            steady_none(),
            anchor("Onメニューを開く", "メニュー", references),
            vec![no_content("OnChoiceEvent")],
        );
        assert_eq!(chain.gets, [expected]);
        assert!(chain.talks.is_empty());
        assert!(matches!(chain.state.phase, Phase::Steady { talk: None }));
        assert!(chain.state.anchor.is_none());
    }
}

/// 要件 4.3・4.11: 1 回の知らせで送るイベントの列は 1 本、台詞の起動は高々 1 回。台本が返った段で
/// 列は終わる（`OnAnchorSelectEx` に台本なら `OnAnchorSelect` を送らない）。
#[test]
fn one_notification_makes_one_event_chain_and_at_most_one_talk() {
    for start in [steady_none, steady_talking] {
        // (ID, 応答の列, 送るイベントの名前の列, 始める台詞の番号の列)
        let cases = [
            (
                "詳細",
                vec![no_content("OnAnchorSelectEx"), no_content("OnAnchorSelect")],
                &["OnAnchorSelectEx", "OnAnchorSelect"][..],
                &[][..],
            ),
            (
                "詳細",
                vec![script("OnAnchorSelectEx")],
                &["OnAnchorSelectEx"][..],
                &[TalkId(7)][..],
            ),
            (
                "詳細",
                vec![no_content("OnAnchorSelectEx"), script("OnAnchorSelect")],
                &["OnAnchorSelectEx", "OnAnchorSelect"][..],
                &[TalkId(7)][..],
            ),
            (
                "Onメニューを開く",
                vec![no_content("OnChoiceEvent")],
                &["Onメニューを開く"][..],
                &[][..],
            ),
            (
                "Onメニューを開く",
                vec![script("OnChoiceEvent")],
                &["Onメニューを開く"][..],
                &[TalkId(7)][..],
            ),
        ];
        for (index, (id, replies, names, talks)) in cases.into_iter().enumerate() {
            let chain = run(start(), anchor(id, "文字", &[]), replies);
            let sent: Vec<&str> = chain.gets.iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(sent, names, "{index}: {id}");
            assert_eq!(chain.talks, talks, "{index}: {id}");
            assert!(chain.state.anchor.is_none(), "{index}: 段の記憶は残さない");
        }
    }
}

// --- 話している最中 ---

/// 要件 4.8: 話している最中に台本が返ると、どの段の応答でも新しい番号の台詞に置き換わり、
/// 選択待ちの帳簿はその置き換えで消える。
#[test]
fn script_reply_while_talking_replaces_the_talk_and_clears_the_choice_ledger() {
    let cases = [
        ("詳細", vec![script("OnAnchorSelectEx")]),
        (
            "詳細",
            vec![no_content("OnAnchorSelectEx"), script("OnAnchorSelect")],
        ),
        ("Onメニューを開く", vec![script("OnChoiceEvent")]),
    ];
    for (index, (id, replies)) in cases.into_iter().enumerate() {
        let chain = run(
            steady_talking_with_choice(),
            anchor(id, "文字", &[]),
            replies,
        );
        assert_eq!(chain.talks, [TalkId(7)], "{index}: 起動は 1 回・新しい番号");
        assert!(
            matches!(
                &chain.state.phase,
                Phase::Steady { talk: Some(talk) }
                    if talk.talk_id == TalkId(7) && talk.script == "\\0やあ\\e"
            ),
            "{index}: 再生中の台詞が置き換わる"
        );
        assert!(
            chain.state.choice.is_none(),
            "{index}: 帳簿は置き換えで消える"
        );
        assert!(chain.state.anchor.is_none(), "{index}");
    }
}

/// 要件 4.9: 話している最中に何も返らなければ、再生中の台詞も選択待ちの帳簿もそのまま。
#[test]
fn no_content_while_talking_leaves_the_talk_and_the_choice_ledger() {
    let cases = [
        (
            "詳細",
            vec![no_content("OnAnchorSelectEx"), no_content("OnAnchorSelect")],
        ),
        ("Onメニューを開く", vec![no_content("OnChoiceEvent")]),
    ];
    for (index, (id, replies)) in cases.into_iter().enumerate() {
        let chain = run(
            steady_talking_with_choice(),
            anchor(id, "文字", &[]),
            replies,
        );
        assert!(chain.talks.is_empty(), "{index}");
        assert!(still_talk_3(&chain.state), "{index}: 台詞はそのまま");
        assert!(choice_ledger_kept(&chain.state), "{index}: 帳簿は残る");
        assert_eq!(chain.state.next_talk_id, 7, "{index}");
        assert!(chain.state.anchor.is_none(), "{index}");
    }
}

// --- 送信の失敗 ---

/// 要件 4.10: 送信の失敗は、どの段でも 204 と同じに進む。記録は `anchor_shiori_failed_as_204` が
/// ちょうど 1 件で、横断の `shiori_failed` は出ず、終了へ倒れない（台詞も帳簿もそのまま）。
#[test]
fn send_failure_advances_like_no_content_with_one_error_and_no_fault() {
    // (ID, 応答の列, 失敗した往復の出どころ, 記録の段, 送るイベントの名前の列)
    let cases = [
        (
            "詳細",
            vec![(failed(), "OnAnchorSelectEx"), no_content("OnAnchorSelect")],
            "OnAnchorSelectEx",
            "select_ex",
            &["OnAnchorSelectEx", "OnAnchorSelect"][..],
        ),
        (
            "詳細",
            vec![no_content("OnAnchorSelectEx"), (failed(), "OnAnchorSelect")],
            "OnAnchorSelect",
            "final",
            &["OnAnchorSelectEx", "OnAnchorSelect"][..],
        ),
        (
            "Onメニューを開く",
            vec![(failed(), "OnChoiceEvent")],
            "OnChoiceEvent",
            "final",
            &["Onメニューを開く"][..],
        ),
    ];
    for (index, (id, replies, origin, stage, names)) in cases.into_iter().enumerate() {
        let mut out = None;
        let events = capture(|| {
            out = Some(run(
                steady_talking_with_choice(),
                anchor(id, "文字", &[]),
                replies,
            ));
        });
        let chain = out.unwrap();
        let sent: Vec<&str> = chain.gets.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(sent, names, "{index}: 204 と同じ列");
        assert!(chain.talks.is_empty(), "{index}");
        assert!(still_talk_3(&chain.state), "{index}: 終了へ倒れない");
        assert!(choice_ledger_kept(&chain.state), "{index}: 帳簿は残る");
        assert!(chain.state.anchor.is_none(), "{index}");

        let record = logged_once(&events, Level::ERROR, "anchor_shiori_failed_as_204");
        let field = |name: &str| record.fields.get(name).map(String::as_str);
        assert_eq!(field("id"), Some(id), "{index}");
        assert_eq!(field("stage"), Some(stage), "{index}");
        assert_eq!(field("origin"), Some(origin), "{index}");
        assert_eq!(
            field("error"),
            Some("shiori ipc failure: pipe closed"),
            "{index}"
        );
        assert_eq!(
            events.iter().filter(|e| e.level == Level::ERROR).count(),
            1,
            "{index}: error の記録は失敗 1 回につき 1 件だけ"
        );
        assert_not_logged(&events, "shiori_failed");
    }
}

// --- 定常以外 ---

/// 定常以外では、最上位の入口からでも棄却の警告 1 件だけで、何も送らず何も覚えない。
#[test]
fn anchor_outside_steady_is_rejected_and_nothing_is_queued() {
    let phases = [
        Phase::Idle,
        Phase::BootMain,
        Phase::ClosePending {
            reason: CloseReason::System,
        },
        Phase::ChangePending,
        Phase::Unloading {
            cause: TermCause::Quit,
        },
        Phase::Stopped,
        // 別れ・切替の台詞がバルーンに出ている相。利用者が実際にアンカーを押せるのはここ。
        Phase::CloseTalkWait {
            talk_id: TalkId(3),
            deadline: None,
        },
        Phase::ChangeTalkWait {
            talk_id: TalkId(3),
            deadline: None,
        },
        Phase::ChangeCloseTalkWait {
            talk_id: TalkId(3),
            deadline: None,
        },
    ];
    let labels = [
        "Idle",
        "BootMain",
        "ClosePending",
        "ChangePending",
        "Unloading",
        "Stopped",
        "CloseTalkWait",
        "ChangeTalkWait",
        "ChangeCloseTalkWait",
    ];
    for (phase, label) in phases.into_iter().zip(labels) {
        let start = State {
            phase,
            ..steady_none()
        };
        let mut out = None;
        let events = capture(|| {
            out = Some(step(
                start,
                Input::Anchor(anchor("詳細", "くわしく", &["a"])),
                &config(),
            ));
        });
        let (state, actions) = out.unwrap();
        assert!(actions.is_empty(), "{label}: 何も送らない");
        assert_eq!(phase_label(&state.phase), label, "{label}: 相は変えない");
        assert!(state.anchor.is_none(), "{label}: 段を覚えない");
        assert!(
            state.reply_source.is_none(),
            "{label}: 応答待ちの控えも無い"
        );
        assert_eq!(state.next_talk_id, 7, "{label}");

        let record = logged_once(&events, Level::WARN, "anchor_rejected_phase");
        assert_eq!(
            record.fields.get("phase").map(String::as_str),
            Some(label),
            "{label}"
        );
        assert_eq!(
            events.iter().filter(|e| e.level == Level::ERROR).count(),
            0,
            "{label}: error の記録は無い"
        );
        assert_eq!(
            events.iter().filter(|e| e.level == Level::WARN).count(),
            1,
            "{label}: 警告はちょうど 1 件"
        );
        assert_not_logged(&events, "anchor_accepted");
    }
}

// --- 選択待ちの印 ---

/// アンカーは選択待ちの印（`choosing`）を立てない。アンカー由来の GET の Status は、送る時点の
/// 台詞と選択待ちの帳簿だけで決まる（帳簿があるときに載る印は、帳簿のもの）。
#[test]
fn anchor_never_raises_the_choosing_mark() {
    // (始まりの状態, アンカー由来の GET すべてに付く Status の行)
    type Start = fn() -> State;
    let cases: [(Start, Option<&str>); 3] = [
        (steady_none, None),
        (steady_talking, Some("talking")),
        (steady_talking_with_choice, Some("talking,choosing")),
    ];
    for (index, (start, status)) in cases.into_iter().enumerate() {
        let status = status.map(str::to_string);
        let chain = run(
            start(),
            anchor("詳細", "くわしく", &[]),
            vec![no_content("OnAnchorSelectEx"), no_content("OnAnchorSelect")],
        );
        assert_eq!(
            chain.statuses,
            [status.clone(), status.clone()],
            "{index}: 2 段とも"
        );

        let chain = run(
            start(),
            anchor("Onメニューを開く", "メニュー", &[]),
            vec![no_content("OnChoiceEvent")],
        );
        assert_eq!(chain.statuses, [status], "{index}: On 始まり");
    }

    // 応答を待っている間（段の記憶がある間）も、選択待ちの扱いにならない。
    let (state, _) = step(
        steady_talking(),
        Input::Anchor(anchor("詳細", "くわしく", &[])),
        &config(),
    );
    assert!(state.anchor.is_some());
    assert!(state.choice.is_none(), "選択待ちの帳簿を作らない");
    assert!(!state.snapshot().choice_active);
}
