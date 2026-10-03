//! 台本の時間の指令（`\![set,choicetimeout,N]` が運ぶ `timeout_directive_secs`）を受けた kanade が、
//! 指令どおりの期限で `OnChoiceTimeout` を出すことを外側から確かめる檻（areka-P0-choice-timeout-directive
//! タスク 3.3・要件 1.1／1.2／1.4／2.1／2.2／9.4 後半／9.5）。
//!
//! 共有の補助関数 `establish_choice_wait` は指令を `None`（未指定）に固定して投函するため使わない。
//! 同じ注入列（Boot → `Tick(1_000)` で steady talk id=1 を起こす → `ChoiceWaiting`）を本ファイルの
//! [`run_choice_wait`] に自前で書き、指令だけを差し替える。時刻はすべて注入 Tick の論理値であり、
//! 実時刻も sleep も読まない（要件 9.5）。

use super::test_support::{
    CANONICAL_CHOICE_ID, cascading_snapshot, choice_get_ids, command_tags, position_of, pumps,
};
use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, FIXED_STEADY_SCRIPT, Fixture, Harness, KanadeConfig,
    KanadeMsg, MonotonicMs, QuitPolicy, RecordedCall, TalkCommand, TalkDone, TalkEndReason, TalkId,
    events, expected_call, join_bounded, spawn_harness_gated,
};

/// 注入 `ChoiceWaiting` の表示の終わり（期限の起点）。
const DISPLAY_END_MS: u64 = 1_000;

/// 境の値（ミリ秒）。台本 `\![set,choicetimeout,1234]` の `N`。対になる ghost のテストと共有する。
const BOUNDARY_MS: u64 = 1_234;

/// 既定の時間切れ（30 秒）を越えた先の時刻。無効化の指令がここでも閉じないことを見る。
const PAST_DEFAULT_MS: u64 = DISPLAY_END_MS + 60_000;

/// 選択待ちを指令付きで作り、`ticks` を順に注入し、最後に中断の完了と close で終わらせる。
///
/// 終わらせ方は時間切れが出た場合も出なかった場合も同じ注入列にする: `TalkDone{Interrupted}`
/// （mock sakura は再生層を持たないので檻が直接注入する）で `Steady{None}` へ戻し、`CloseRequest`
/// で即座に close 握手（別れ talk quit:true）を走らせる。kanade の期限付き join の成功を記録の確定点とする。
fn run_choice_wait(directive: Option<f64>, ticks: &[u64]) -> (Vec<RecordedCall>, Vec<TalkCommand>) {
    // OnChoiceTimeout は未注入＝204（時間切れなら CancelChoice が出る）。
    let fixture = Fixture::quitting()
        .without_boot_greeting()
        .with_steady_value_indices([0]);
    // index0（steady talk・保留）=false・index1（close talk）=true。
    let (harness, _gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    let tx = &harness.sender;
    tx.send(KanadeMsg::Boot).expect("send Boot");
    tx.send(KanadeMsg::Tick {
        now: MonotonicMs(DISPLAY_END_MS),
    })
    .expect("send Tick that starts the steady talk");
    tx.send(KanadeMsg::ChoiceWaiting {
        talk_id: TalkId(1),
        choice_ids: vec![CANONICAL_CHOICE_ID.to_string()],
        display_end: MonotonicMs(DISPLAY_END_MS),
        timeout_directive_secs: directive,
    })
    .expect("send ChoiceWaiting with the directive");
    for &now in ticks {
        tx.send(KanadeMsg::Tick {
            now: MonotonicMs(now),
        })
        .expect("send Tick");
    }
    tx.send(KanadeMsg::TalkDone(TalkDone {
        talk_id: TalkId(1),
        reason: TalkEndReason::Interrupted,
        quit_reserved: false,
    }))
    .expect("send TalkDone{Interrupted}");
    tx.send(KanadeMsg::CloseRequest {
        reason: CloseReason::User { scope: 0 },
    })
    .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;
    join_bounded("kanade timeout-directive join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close");
    drop(sender);
    let commands = sakura.commands();
    sakura.join_bounded("mock-sakura timeout-directive join", DEFAULT_TIMEOUT);
    (shiori.recorded(), commands)
}

/// **指令 `Some(1.234)` の期限の両側（要件 1.1・1.2・1.4・9.4 後半）**: 表示の終わり `1_000` から
/// ＋1233 ms の Tick では時間切れが出ず、＋1234 ms ちょうどの Tick で `OnChoiceTimeout` が出る。
///
/// 対になるテストは ghost の `script_choice_timeout_1234_reaches_kanade_with_display_end_and_directive`
/// （境の値 `1234`）。あちらは台本 `\![set,choicetimeout,1234]` が kanade の入口で `Some(1.234)` に
/// なることを、こちらはその `Some(1.234)` が「表示の終わり＋1234 ms」ちょうどの期限になることを示し、
/// 2 本の合成で「台本から期限まで通し」になる。片方だけ境の値を変えたら、もう片方も合わせること。
///
/// # 弁別
/// - 期限が遅れる（指令を無視して既定 30 秒になる・＋1235 以降になる）と `OnChoiceTimeout` が出ず落ちる。
/// - 期限が早まる（＋1233 以前）と＋1233 の Tick で発火してその周期の pump が消え、記録順が
///   「`OnChoiceTimeout` → NOTIFY pump」へ逆転し、pump から `choosing` も消えて落ちる。
/// - `OnChoiceTimeout` は既定の時間切れと同じ events 表導出（Ref0＝起動スクリプト）と完全一致し、
///   204 で `CancelChoice` が出る（同じ経路・要件 1.2）。
#[test]
fn choice_timeout_directive_1234_fires_exactly_at_display_end_plus_1234() {
    let just_before = DISPLAY_END_MS + BOUNDARY_MS - 1;
    let deadline = DISPLAY_END_MS + BOUNDARY_MS;
    let (recorded, commands) = run_choice_wait(Some(1.234), &[just_before, deadline]);

    assert_eq!(
        choice_get_ids(&recorded),
        vec!["OnChoiceTimeout"],
        "指令 1.234 秒では表示の終わり＋1234 ms で時間切れが 1 回だけ出るはず: {recorded:?}"
    );
    let timeout_pos = position_of(&recorded, CallMethod::Get, "OnChoiceTimeout");
    assert_eq!(
        recorded[timeout_pos],
        expected_call(events::on_choice_timeout(
            FIXED_STEADY_SCRIPT,
            &cascading_snapshot()
        )),
        "既定の時間切れと同じ形（Ref0＝起動スクリプト・Status）のはず: {recorded:?}"
    );

    // ＋1233 の Tick は発火せず pump（選択待ち継続中の NOTIFY）を出し、＋1234 の Tick は pump を出さない。
    let pump_calls = pumps(&recorded);
    let pump_methods: Vec<&CallMethod> = pump_calls.iter().map(|c| &c.method).collect();
    assert_eq!(
        pump_methods,
        vec![&CallMethod::Get, &CallMethod::Notify],
        "pump は Tick(1_000) の GET と＋1233 の NOTIFY だけ（発火した＋1234 は pump を出さない）: {recorded:?}"
    );
    assert_eq!(
        pump_calls[1].status,
        Some("talking,choosing".to_string()),
        "＋1233 ではまだ選択待ちが続いているはず: {recorded:?}"
    );
    let before_pos = recorded
        .iter()
        .position(|c| *c == *pump_calls[1])
        .expect("＋1233 の NOTIFY pump が記録されているはず");
    assert!(
        before_pos < timeout_pos,
        "時間切れは＋1233 の Tick の後（＋1234 の Tick）で初めて出るはず: {recorded:?}"
    );

    assert_eq!(
        command_tags(&commands),
        vec![
            "Start(1)".to_string(),
            "Cancel(1)".to_string(),
            "Start(2)".to_string()
        ],
        "時間切れ 204 は解除指示を出すはず（その後の起動は close talk のみ）: {commands:?}"
    );
}

/// **指令 `Some(0.0)`・`Some(-0.001)` は時間切れにしない（要件 2.1・2.2）**: 既定の 30 秒を越えた
/// 表示の終わり＋60,000 ms の Tick でも `OnChoiceTimeout` が出ず、その Tick は選択待ち継続中の pump を出す。
///
/// # 較正
/// 同じ注入列・同じ Tick で指令だけ `None`（既定 30 秒）にした対照では `OnChoiceTimeout` が出る。
/// ＋60,000 ms の Tick が期限の判定に届いていることを同じ檻の中で示し、0 件の主張を空振りでなくする。
#[test]
fn choice_timeout_directive_zero_and_negative_never_fire_even_past_default() {
    let (control, control_commands) = run_choice_wait(None, &[PAST_DEFAULT_MS]);
    assert_eq!(
        choice_get_ids(&control),
        vec!["OnChoiceTimeout"],
        "対照: 指令なし（既定 30 秒）なら＋60,000 ms で時間切れが出るはず: {control:?}"
    );
    assert!(
        command_tags(&control_commands).contains(&"Cancel(1)".to_string()),
        "対照: 時間切れ 204 で解除指示が出るはず: {control_commands:?}"
    );

    for directive in [Some(0.0), Some(-0.001)] {
        let (recorded, commands) = run_choice_wait(directive, &[PAST_DEFAULT_MS]);
        assert!(
            choice_get_ids(&recorded).is_empty(),
            "指令 {directive:?} では＋60,000 ms でも時間切れが出ないはず: {recorded:?}"
        );
        let pump_calls = pumps(&recorded);
        assert_eq!(
            pump_calls.len(),
            2,
            "指令 {directive:?}: pump は Tick(1_000) と＋60,000 ms の 2 回のはず: {recorded:?}"
        );
        assert_eq!(
            pump_calls[1].status,
            Some("talking,choosing".to_string()),
            "指令 {directive:?}: ＋60,000 ms でも選択待ちは続いているはず: {recorded:?}"
        );
        assert_eq!(
            command_tags(&commands),
            vec!["Start(1)".to_string(), "Start(2)".to_string()],
            "指令 {directive:?}: 解除指示は出ないはず: {commands:?}"
        );
    }
}
