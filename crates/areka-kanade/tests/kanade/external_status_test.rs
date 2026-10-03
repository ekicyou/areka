//! 外から届く実行状態（balloon・nouserbreak・online）が送り口の `Status` に載ることの統合檻
//! （areka-P0-status-execution-states タスク 2.5・要件 2.3／2.6／3.1／3.2／4.1／5.1／7.2／8.2）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、
//! `KanadeMsg::ExecutionState` の知らせと、構成で差し替えた通信中の数だけで観測する。
//! 期待値は `events::*` から `expected_call` で導出し、wire の実値（`status`）も逐語で突き合わせる。
//!
//! # 決定性
//! - トーク中の窓は保留ハーネス（`spawn_harness_gated`・hold_indices=[0]）で作る（steady_test と同じ）。
//!   知らせと Tick は同じ送信端から送るので kanade の受信箱で FIFO に並び、割り込むメッセージは無い。
//! - online は関数内の `static` を `KanadeConfig.online` で渡す（プロセスの数 `online::PROCESS` には
//!   触れない＝同じ実行ファイルで並走する他のテストの `Status` を揺らさない）。守り手を落とす時点は
//!   空の `ResourceQuery` の返事（殻がその場で答える）で「Boot の処理が済んだ」ことを確かめてから選ぶ。

use areka_actor::reply_channel;
use areka_kanade::{
    BalloonBinding, CloseReason, ExecutionSnapshot, ExecutionStateUpdate, KanadeConfig, KanadeMsg,
    MonotonicMs, OnlineCounter, events, resources::ResourceOutcome,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, Fixture, Harness, QuitPolicy, RecordedCall, expected_call,
    expected_unload, join_bounded, spawn_harness, spawn_harness_gated,
};

fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}

fn pumps(recorded: &[RecordedCall]) -> Vec<&RecordedCall> {
    recorded
        .iter()
        .filter(|c| c.id == "OnSecondChange")
        .collect()
}

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

/// 期限付き join の後に記録列を確定する（終了は駆動した close の別れ talk quit:true）。
fn finish(harness: Harness, what: &str) -> Vec<RecordedCall> {
    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;
    join_bounded(what, DEFAULT_TIMEOUT, kanade).expect("kanade terminates via the driven close");
    drop(sender);
    sakura.join_bounded(what, DEFAULT_TIMEOUT);
    shiori.recorded()
}

/// バルーンの組と中断の旗の知らせが、送り口を通って次の周期リクエストの `Status` に載る
/// （要件 3.1／3.2／4.1／5.1／8.2）。
///
/// 駆動: Boot（挨拶なし）→ `Balloons([0=2,1=0])` → Tick1（トーク無し・GET Value で talk id=1 を起こし
/// TalkDone を保留）→ `Balloons([0=0])` → Tick2 → `Balloons([])` → `NoUserBreak(true)` → Tick3 →
/// `NoUserBreak(false)` → Tick4 → CloseRequest → 保留解放。
///
/// 期待: Tick1 GET `balloon(0=2/1=0)`・Tick2 NOTIFY `talking,balloon(0=0)`・
/// Tick3 NOTIFY `talking,nouserbreak`・Tick4 NOTIFY `talking`。知らせを受け取る腕・写し・作り手の
/// どれが欠けても該当の行が `talking` 単独か `None` に落ちる。
#[test]
fn balloon_and_no_user_break_updates_ride_the_next_pump_status() {
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    send(&harness, KanadeMsg::Boot);
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::Balloons(vec![
            binding(0, 2),
            binding(1, 0),
        ])),
    );
    tick(&harness, 1_000);
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::Balloons(vec![binding(0, 0)])),
    );
    tick(&harness, 2_000);
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::Balloons(Vec::new())),
    );
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::NoUserBreak(true)),
    );
    tick(&harness, 3_000);
    send(
        &harness,
        KanadeMsg::ExecutionState(ExecutionStateUpdate::NoUserBreak(false)),
    );
    tick(&harness, 4_000);
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );
    gate.release_all();

    let recorded = finish(harness, "kanade external-status (balloon/nouserbreak) join");
    let pumps = pumps(&recorded);
    let statuses: Vec<Option<&str>> = pumps.iter().map(|c| c.status.as_deref()).collect();
    assert_eq!(
        statuses,
        vec![
            Some("balloon(0=2/1=0)"),
            Some("talking,balloon(0=0)"),
            Some("talking,nouserbreak"),
            Some("talking"),
        ],
        "周期リクエストの Status は知らせの写しを順に帯びるはず: {recorded:?}"
    );

    let talking = ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    };
    let expected = [
        events::on_second_change(
            MonotonicMs(1_000),
            &ExecutionSnapshot {
                balloons: vec![binding(0, 2), binding(1, 0)],
                ..ExecutionSnapshot::INACTIVE
            },
        ),
        events::on_second_change(
            MonotonicMs(2_000),
            &ExecutionSnapshot {
                balloons: vec![binding(0, 0)],
                ..talking.clone()
            },
        ),
        events::on_second_change(
            MonotonicMs(3_000),
            &ExecutionSnapshot {
                no_user_break: true,
                ..talking.clone()
            },
        ),
        events::on_second_change(MonotonicMs(4_000), &talking),
    ];
    for (i, (actual, call)) in pumps.iter().zip(expected).enumerate() {
        assert_eq!(
            **actual,
            expected_call(call),
            "pump {i} は events 表導出（GET/NOTIFY・Ref3・Status）と一致するはず: {recorded:?}"
        );
    }
    assert_eq!(pumps[0].method, CallMethod::Get, "Tick1 はトーク無し＝GET");
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（driven close→別れ talk quit:true 由来）"
    );
}

/// 通信中の数を立てたまま kanade を起こすと最初のリクエスト（OnInitialize）から `online` が載り
/// （要件 2.6／5.1）、守り手を落とした後の次の Tick からは載らない（要件 2.3）。
///
/// 数は関数内の `static` を `KanadeConfig.online` で渡す（`online::PROCESS` には触れない）。
#[test]
fn online_rides_from_on_initialize_while_held_and_clears_from_the_next_tick_after_drop() {
    static COUNTER: OnlineCounter = OnlineCounter::new();
    let guard = COUNTER.begin("external-status-test");
    let config = KanadeConfig {
        online: &COUNTER,
        ..KanadeConfig::new("master", "1.0.0")
    };
    let harness = spawn_harness(
        config,
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::Fixed(true),
    );

    send(&harness, KanadeMsg::Boot);
    // 殻がその場で答える空の照会を Boot の後ろに並べ、その返事で Boot の処理が済んだことを確かめる
    // （ids が空なので SHIORI へは何も送らない）。
    let (reply, receiver) = reply_channel::<Vec<(&'static str, ResourceOutcome)>>();
    send(
        &harness,
        KanadeMsg::ResourceQuery {
            ids: Vec::new(),
            reply,
        },
    );
    receiver
        .recv_timeout(DEFAULT_TIMEOUT)
        .expect("barrier ResourceQuery answered after Boot");
    drop(guard);
    tick(&harness, 1_000);
    tick(&harness, 2_000);
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );

    let recorded = finish(harness, "kanade external-status (online) join");

    // (1) 最初のリクエストは OnInitialize で、online を帯びる。
    let online = ExecutionSnapshot {
        online: true,
        ..ExecutionSnapshot::INACTIVE
    };
    assert_eq!(
        recorded.first().expect("記録列は空でない"),
        &expected_call(events::on_initialize(&online)),
        "数を立てたまま起こすと OnInitialize から online が載るはず: {recorded:?}"
    );
    assert_eq!(recorded[0].status.as_deref(), Some("online"));

    // (2) 守り手を持っていた間（最初の Tick より前）の起動のリクエストはすべて online を帯びる。
    let first_pump = recorded
        .iter()
        .position(|c| c.id == "OnSecondChange")
        .expect("Tick の周期リクエストが記録されているはず");
    assert!(
        recorded[..first_pump]
            .iter()
            .all(|c| c.status.as_deref() == Some("online")),
        "起動の各段のリクエストは online を帯びるはず: {recorded:?}"
    );

    // (3) 落とした後の Tick 以降（周期リクエスト・OnClose）は online を帯びない。
    let pumps = pumps(&recorded);
    assert_eq!(
        pumps.len(),
        2,
        "Tick 2 本ぶんの周期リクエスト: {recorded:?}"
    );
    assert_eq!(
        *pumps[0],
        expected_call(events::on_second_change(
            MonotonicMs(1_000),
            &ExecutionSnapshot::INACTIVE
        )),
        "守り手を落とした後の次の Tick から online は載らないはず: {recorded:?}"
    );
    assert!(
        recorded[first_pump..].iter().all(|c| c.status.is_none()),
        "落とした後のリクエストはどれも Status 行なしのはず: {recorded:?}"
    );
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload"
    );
}
