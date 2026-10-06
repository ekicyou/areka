//! 殻が実行の状態の問い合わせ（`KanadeMsg::StatusQuery`）にその場で答えることの統合檻
//! （areka-P0-mcp-get-status タスク 1.2・要件 1.2／1.4／1.5／2.1／2.2／2.3／2.5／3.2／4.1／4.2／5.1／5.3）。
//!
//! 共通の土台（`super::common`）の保留できる偽の sakura（`spawn_harness_gated`）と、呼出を
//! `Status` つきで記録する偽の shiori を使う。問い合わせは同じ送出端から送るので受信箱で順に並び、
//! 「直前の知らせの処理が済んだ後」に必ず処理される。答えは上限つきの受け取りでその場で取る
//! （再生の完了は保留したまま＝再生の終わりを待たずに返ることも同時に見る）。
//!
//! K3（問い合わせは運行を変えない）は、同じ筋書きを「問う」「問わない」の 2 通りで走らせ、
//! 偽の shiori が受けた呼出の列と偽の sakura が受けた指示の列が一致することで見る。

use areka_actor::{ReplyError, reply_channel};
use areka_kanade::{
    BalloonBinding, ChoiceInput, CloseReason, ExecutionSnapshot, ExecutionStateUpdate,
    ExecutionStatus, KanadeConfig, KanadeMsg, MonotonicMs, TalkCommand, TalkId, events,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, Fixture, Harness, QuitPolicy, RecordedCall, expected_call,
    expected_unload, join_bounded, spawn_harness, spawn_harness_gated,
};

/// 選択待ちの候補（正典形＝`On` で始まらない）。
const CHOICE_ID: &str = "頻度変更";

fn send(harness: &Harness, msg: KanadeMsg) {
    harness.sender.send(msg).expect("send to kanade inbox");
}

fn tick(harness: &Harness, now: u64) {
    send(
        harness,
        KanadeMsg::Tick {
            now: MonotonicMs(now),
        },
    );
}

/// 問い合わせを 1 通送り、その場で返る答えを上限つきで受け取る（wire の形 `render()` で返す）。
fn query(harness: &Harness) -> Option<String> {
    let (reply, receiver) = reply_channel::<ExecutionStatus>();
    send(harness, KanadeMsg::StatusQuery { reply });
    receiver
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("問い合わせは期限内にその場で答えられるはず（再生の終わりを待たない）")
        .render()
}

/// 筋書きの 1 か所で問う（`ask` が偽なら何も送らない＝問わない筋書き）。
/// 続けて 2 回問い、同じ値が返ることを確かめる（K3⑴）。
fn ask_here(harness: &Harness, ask: bool, answers: &mut Vec<Option<String>>) {
    if !ask {
        return;
    }
    let first = query(harness);
    let second = query(harness);
    assert_eq!(
        first, second,
        "同じ所で続けて 2 回問うと同じ値が返るはず（問い合わせは状態を変えない・要件 4.1）"
    );
    answers.push(first);
}

fn pumps(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded
        .iter()
        .filter(|c| c.id == "OnSecondChange")
        .collect()
}

/// 偽の sakura が受けた指示の列を比べられる形へ写す（`TalkCommand` は `PartialEq` を持たない）。
fn command_tags(commands: &[TalkCommand]) -> Vec<String> {
    commands
        .iter()
        .map(|c| match c {
            TalkCommand::Start(s) => format!("Start({},{})", s.talk_id.0, s.script),
            TalkCommand::ResolveChoice { talk_id, id } => format!("Resolve({},{})", talk_id.0, id),
            TalkCommand::CancelChoice { talk_id } => format!("Cancel({})", talk_id.0),
        })
        .collect()
}

/// 1 回の走行の結果（問い合わせの答え・shiori の記録列・sakura の指示の列）。
struct Run {
    answers: Vec<Option<String>>,
    recorded: Vec<RecordedCall>,
    commands: Vec<String>,
}

/// close（別れ talk quit:true）で終わらせ、記録を確定する。
fn finish(harness: Harness, answers: Vec<Option<String>>, what: &str) -> Run {
    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;
    join_bounded(what, DEFAULT_TIMEOUT, kanade).expect("kanade terminates via the driven close");
    drop(sender);
    let commands = command_tags(&sakura.join_bounded_then_commands(what, DEFAULT_TIMEOUT));
    Run {
        answers,
        recorded: shiori.recorded(),
        commands,
    }
}

fn gated_harness() -> (Harness, super::common::SakuraGate) {
    spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting()
            .without_boot_greeting()
            .with_steady_value_indices([0]),
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    )
}

fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}

/// K1 の筋書き: 起動の前 → 起動直後 → 中断の無効化（真）→ バルーン → Tick1（再生中・完了は保留）→
/// 中断の無効化（偽）→ Tick2 → 終了。
fn run_k1(ask: bool) -> Run {
    let (harness, gate) = gated_harness();
    let mut answers = Vec::new();

    ask_here(&harness, ask, &mut answers); // ① 起動の前
    send(&harness, KanadeMsg::Boot);
    ask_here(&harness, ask, &mut answers); // ② 起動直後（何も無い）
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::NoUserBreak(true)),
    );
    ask_here(&harness, ask, &mut answers); // ③ 中断の無効化（真）・再生の前
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::Balloons(vec![
            binding(0, 0),
            binding(1, 0),
        ])),
    );
    ask_here(&harness, ask, &mut answers); // ④ バルーン
    tick(&harness, 1_000);
    ask_here(&harness, ask, &mut answers); // ⑤ 再生中（完了は保留）
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::NoUserBreak(false)),
    );
    ask_here(&harness, ask, &mut answers); // ⑥ 中断の無効化（偽）・再生中
    tick(&harness, 2_000);
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );
    gate.release_all();
    finish(harness, answers, "kanade status-query K1 join")
}

/// K2 の筋書き: 起動 → Tick1（再生中・完了は保留）→ 選択待ち → Tick2 → 選択 → 終了。
fn run_k2(ask: bool) -> Run {
    let (harness, gate) = gated_harness();
    let mut answers = Vec::new();

    send(&harness, KanadeMsg::Boot);
    tick(&harness, 1_000);
    ask_here(&harness, ask, &mut answers); // ① 再生中（バルーンの知らせなし）
    send(
        &harness,
        KanadeMsg::ChoiceWaiting {
            talk_id: TalkId(1),
            choice_ids: vec![CHOICE_ID.to_string()],
            display_end: MonotonicMs(1_000),
            timeout_directive_secs: None,
        },
    );
    ask_here(&harness, ask, &mut answers); // ② 選択待ち
    tick(&harness, 2_000);
    send(
        &harness,
        KanadeMsg::Choice(ChoiceInput {
            id: CHOICE_ID.to_string(),
            label: "おしゃべり頻度".to_string(),
            scope: 0,
            references: Vec::new(),
        }),
    );
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );
    gate.release_all();
    finish(harness, answers, "kanade status-query K2 join")
}

/// K3⑵⑶: 問い合わせを挟んだ走行と挟まない走行で、shiori・sakura が受けた列が一字違わず同じ。
fn assert_same_run(asked: &Run, plain: &Run) {
    assert_eq!(
        asked.recorded, plain.recorded,
        "問い合わせは SHIORI へ 1 通も送らず、運行（GET/NOTIFY・Reference・Status）を変えないはず（要件 4.1）"
    );
    assert_eq!(
        asked.commands, plain.commands,
        "問い合わせは sakura への指示の列を変えないはず（要件 4.1）"
    );
    assert!(
        plain.answers.is_empty(),
        "問わない筋書きは答えを持たない（比べる相手として正しく組めている）"
    );
}

// ============================================================================
// K1: 起動の前・何も無い・中断の無効化・バルーン・再生中（要件 5.1⑴⑵⑷⑸・5.3・4.2）
// ============================================================================

#[test]
fn k1_answers_follow_boot_balloon_no_user_break_and_talking() {
    let run = run_k1(true);

    assert_eq!(
        run.answers,
        vec![
            None,
            None,
            None,
            Some("balloon(0=0/1=0)".to_string()),
            Some("talking,nouserbreak,balloon(0=0/1=0)".to_string()),
            Some("talking,balloon(0=0/1=0)".to_string()),
        ],
        "起動の前・起動直後・再生の前の中断の無効化は値なし、バルーンと再生中はその集合を返すはず: {:?}",
        run.recorded
    );

    // 5.3: 周期の要求に付く Status が、直前の問い合わせの答えと一字違わず同じ。
    let pumps = pumps(&run.recorded);
    assert_eq!(
        pumps.len(),
        2,
        "Tick 2 本ぶんの周期の要求: {:?}",
        run.recorded
    );
    assert_eq!(
        pumps[0].status, run.answers[3],
        "Tick1 の要求（GET）の Status は直前の問い合わせの答えと同じはず"
    );
    assert_eq!(
        pumps[1].status, run.answers[5],
        "Tick2 の要求（再生中）の Status は直前の問い合わせの答えと同じはず"
    );
    assert_eq!(pumps[0].method, CallMethod::Get, "Tick1 はトーク無し＝GET");
    assert_eq!(
        *pumps[1],
        expected_call(events::on_second_change(
            MonotonicMs(2_000),
            &ExecutionSnapshot {
                talk_active: true,
                balloons: vec![binding(0, 0), binding(1, 0)],
                ..ExecutionSnapshot::INACTIVE
            },
        )),
        "Tick2 の要求は events 表から導いた NOTIFY と一致するはず: {:?}",
        run.recorded
    );
}

// ============================================================================
// K2: 選択待ち（要件 5.1⑵⑶・5.3）
// ============================================================================

#[test]
fn k2_answers_talking_then_choosing_and_match_the_next_pump() {
    let run = run_k2(true);

    assert_eq!(
        run.answers,
        vec![
            Some("talking".to_string()),
            Some("talking,choosing".to_string()),
        ],
        "再生中は talking だけ、選択待ちは talking,choosing を返すはず: {:?}",
        run.recorded
    );
    let pumps = pumps(&run.recorded);
    let notify = pumps
        .iter()
        .find(|c| c.method == CallMethod::Notify)
        .expect("選択待ち中の Tick2 は NOTIFY の周期の要求を出すはず");
    assert_eq!(
        notify.status, run.answers[1],
        "選択待ち中の周期の要求の Status は問い合わせの答えと一字違わず同じはず（要件 5.3）"
    );
    assert!(
        run.recorded
            .iter()
            .any(|c| c.method == CallMethod::Get && c.id == "OnChoiceSelectEx"),
        "問い合わせの後でも選択のイベントは SHIORI へ届くはず（選択待ちの帳簿を壊していない）: {:?}",
        run.recorded
    );
}

// ============================================================================
// K3: 問い合わせは運行を変えない（要件 5.1⑹・4.1）
// ============================================================================

#[test]
fn k3_queries_do_not_change_the_run() {
    let asked = run_k1(true);
    let plain = run_k1(false);
    assert_same_run(&asked, &plain);
    assert_eq!(
        plain.recorded.last(),
        Some(&expected_unload()),
        "筋書きの終わり（Unload）まで走っている"
    );

    let asked = run_k2(true);
    let plain = run_k2(false);
    assert_same_run(&asked, &plain);
    assert_eq!(
        plain.recorded.last(),
        Some(&expected_unload()),
        "筋書きの終わり（Unload）まで走っている"
    );
}

// ============================================================================
// K4: 止まった kanade に残った問い合わせ（要件 3.2 の kanade の側）
// ============================================================================

#[test]
fn k4_query_after_close_ends_with_reply_dropped() {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::Fixed(true),
    );
    send(&harness, KanadeMsg::Boot);
    send(&harness, KanadeMsg::Close);
    let (reply, receiver) = reply_channel::<ExecutionStatus>();
    send(&harness, KanadeMsg::StatusQuery { reply });

    assert!(
        matches!(
            receiver.recv_timeout(DEFAULT_TIMEOUT),
            Err(ReplyError::Dropped)
        ),
        "終了の知らせの後の問い合わせは受信箱ごと落ちて「返信端が落ちた」で終わるはず"
    );

    let Harness {
        sender,
        kanade,
        sakura,
        ..
    } = harness;
    join_bounded("kanade status-query K4 join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade stops on Close");
    drop(sender);
    sakura.join_bounded("mock-sakura status-query K4 join", DEFAULT_TIMEOUT);
}
