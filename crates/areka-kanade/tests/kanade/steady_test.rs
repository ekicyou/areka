//! 定常運転の統合検証（pump ゲート・talk 調停・Value→StartTalk）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、
//! 定常運転（`Phase::Steady`）の観測可能な振る舞いを、テストごとに単一の合否として検証する（主な 3 つ）:
//!
//! - **応答なし（204）→ 再生起動要求なし**: `OnSecondChange` GET（Ref3=1）が 204 を返す間、
//!   close talk 以外の再生起動要求は宛先へ一切到達しない（boot は挨拶なし）。
//! - **散発的 Value → 一意 talk_id 付きの再生起動要求**: 特定の `OnSecondChange` GET 出現で fixture が
//!   Value を返すと、一意 talk_id（先頭採番 id=1）を持つ steady talk が宛先へ到達する
//!   （OnSecondChange Value が boot と同一 talk 経路を通る）。
//! - **再生中 Tick → NOTIFY（Ref3=0）・応答無視で StartTalk なし**: talk を 1 本 active に保ったまま
//!   Tick を挟むと、`OnSecondChange` は NOTIFY（Ref3=0）で発行され、その応答は構造的に破棄されて
//!   再生起動要求を一切生まない（active talk 中の重複調停を発生源から断つ）。
//!
//! # 決定性と同期イディオム
//! full_run_test.rs と同じ手順で決定的に同期する: quit シナリオ
//! （`Fixture::quitting()`＋末尾 talk quit:true）で Boot→Tick…→CloseRequest を送り、
//! close talk の完了が終了系列（Unload→StopSelf）を駆動する。kanade の期限付き join が
//! 成功すれば、それまでの全 shiori 呼出・全 StartTalk 配送は確定済みであり、実時間 sleep を
//! 一切用いずに記録列を確定できる。
//!
//! # 挨拶なし boot で steady Tick の決定性を保つ
//! boot は起動挨拶 talk を追跡する（挨拶 Value 経路は `Steady{talk: Some(挨拶)}` へ完了し、挨拶の
//! TalkDone が slot と照合される）。すると挨拶 talk の TalkDone（mock sakura が別スレッドから返す）と
//! Boot の後に注入する steady Tick とが inbox 上で競合し、最初期の Tick が `Steady{Some}`
//! （NOTIFY・Ref3=0）か `Steady{None}`（GET・Ref3=1）かが非決定になる。本ファイルの各テストは
//! `Fixture::without_boot_greeting()`（`OnBoot`→204）で boot を `Steady{talk: None}` へ直行させ、
//! この競合を発生源から断つ。挨拶 boot 自体の観測は full_run_test が担う。
//!
//! # active talk 中 Tick の NOTIFY（Ref3=0）
//! 「再生中の Tick が OnSecondChange を NOTIFY（Ref3=0・応答無視）で発行する」は純粋状態機械の
//! ユニットテスト（`src/schedule/steady_flow_tests.rs` の `steady_some_tick_emits_notify`／
//! `steady_some_second_change_value_is_discarded_dd6`）でも被覆されるが、本ファイルは統合層でも
//! 決定的に検証する（`active_talk_tick_emits_notify_ref3_zero`）。
//!
//! 決定的な active-talk 窓は、保留機能付きハーネス（`spawn_harness_gated`＋`SakuraGate`）で作る:
//! steady talk の TalkDone を保留（park）することで、当該 talk は kanade の inbox へ完了通知を返さない。
//! TalkDone は「二つの Tick の間に割り込み得る唯一のメッセージ」であり、これを送らない限り
//! 次 Tick は必ず `Steady{Some}` から処理され NOTIFY（Ref3=0）を発行する。観測後に保留を解放
//! （`release_all`）すれば `Steady{None}` へ復帰し、以降は他のテストと同じ quit 経路で記録列を確定できる。
//!
//! # 送出 ID egress スイープと完了後の GET 再開
//! - 送出 ID の egress スイープ: 自発トーク（`OnSecondChange` Value→StartTalk）が確実に発火する
//!   `sporadic_value_starts_unique_talk` の run 全域の記録を同じテストの末尾で走査し、
//!   `OnTalk`／`OnHour` が 1 件も現れず・Unload を除く全 SHIORI id が許可集合に属することを検証する。
//!   自発トークは pasta が `OnSecondChange` 内部で生成する＝kanade は OnTalk/OnHour を新規に足さず、
//!   StartTalk として下流へ配送する。
//! - `talk_completion_resumes_get_pump_ref3_one_status_none`: steady talk 完了（TalkDone 着弾）後の
//!   次 Tick で `OnSecondChange` GET pump（Ref3=1・Status なし）が再開することを統合層で検証する。
//!   `active_talk_tick_emits_notify_ref3_zero` は NOTIFY 窓までしか観測せず、close_test の別れの台詞は
//!   `CloseTalkWait` から終了へ進んで定常へ戻らないので、`Steady{Some}`→TalkDone→`Steady{None}` の
//!   復帰は本 cage が埋める（純粋状態機械では `steady_flow_tests.rs` の
//!   `steady_talk_done_ended_resumes_steady_and_pump_restarts` が被覆済み）。

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, StartTalk, TalkId,
    events, resources,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, Driven, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, Fixture,
    QuitPolicy, RecordedCall, drive_ticks_until_disconnect, expected_call, expected_unload,
    resumed_get_after_active_window, send, spawn_harness, spawn_harness_blocking,
    spawn_harness_gated, tick,
};

/// Boot→（steady Tick を tick_count 回）→CloseRequest を駆動し、終了完走まで待って記録を確定する。
///
/// `fixture` は close_quits=true（`Fixture::quitting()` 由来）かつ挨拶なし boot
/// （`without_boot_greeting()`）である前提。close talk を末尾 talk として quit:true にすることで
/// 終了系列（Unload→StopSelf）が完走し、kanade の期限付き join 成功時点で全 shiori 呼出・全 StartTalk
/// 配送が確定する。
///
/// sakura が受領する talk は「（発生し得る）steady talk・close talk」の順のみ。quit_flags はこの順で
/// quit を割り当てる。close talk が末尾に来るよう十分な false を前置し、末尾に唯一の true（close talk）
/// を置くこと。
fn drive_steady(fixture: Fixture, tick_count: u64, quit_flags: Vec<bool>) -> Driven {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(quit_flags),
    );

    send(&harness, KanadeMsg::Boot);

    // 時刻は注入 Tick のみで進行させる。
    for i in 1..=tick_count {
        tick(&harness, i * 1_000);
    }

    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );

    harness.finish("kanade steady join")
}

/// 記録列中の `OnSecondChange` GET（Ref3=1・pump 問い合わせ）の出現数を数える。
fn count_second_change_get(recorded: &[RecordedCall]) -> usize {
    recorded
        .iter()
        .filter(|c| c.method == CallMethod::Get && c.id == "OnSecondChange")
        .count()
}

/// 応答なし（204）→ 再生起動要求が発生しない。
///
/// 挨拶なし boot で boot を `Steady{None}` へ直行させ、steady_value_indices 空＝OnSecondChange GET は
/// 常に 204 の構成で Boot→複数 Tick→CloseRequest を駆動する。204 基調ゆえ一切 talk を起こさず、
/// 宛先へ到達する StartTalk は close talk ちょうど 1 本のみになる。
///
/// この不変量は cross-thread な TalkDone に依存しない: 挨拶なし boot は StartTalk を生まず、Value を
/// 返す OnSecondChange 応答も 1 度もない以上、steady talk は構造的に生じ得ない。
#[test]
fn steady_no_content_produces_no_start_talk() {
    // sakura が受領する唯一の talk は close talk（index 0）＝quit:true。PerTalk は範囲外 index を false
    // 扱いするので、万一 steady talk が生じても quit せず、下の assert が検出する。
    let driven = drive_steady(Fixture::quitting().without_boot_greeting(), 4, vec![true]);

    assert_eq!(
        driven.started.len(),
        1,
        "挨拶なし boot・204 基調では steady talk が起きず、close talk 1 本のみが到達するはず: {:?}",
        driven.started
    );

    assert_eq!(
        driven.started[0].script, FIXED_FAREWELL_SCRIPT,
        "唯一到達する StartTalk は close talk（別れの fixture スクリプト）"
    );

    // 204 の間も pump の問い合わせ（OnSecondChange GET・Ref3=1）は現に発行されている。
    let first_get = expected_call(events::on_second_change(
        MonotonicMs(1_000),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert!(
        driven.recorded.contains(&first_get),
        "events 表導出の OnSecondChange GET（Ref3=1・Status なし）が記録列に現れるはず: {:?}",
        driven.recorded
    );
}

/// 散発的な Value → 一意 talk_id 付きの再生起動要求。
///
/// 挨拶なし boot で boot を `Steady{None}` へ直行させ、`with_steady_value_indices([0])` で最初の
/// OnSecondChange GET 出現に Value を仕込む。Boot→十分な Tick→CloseRequest を駆動すると、その GET
/// 出現で steady talk が 1 本起動し、一意な talk_id を持つ StartTalk が宛先へ到達する。これは
/// OnSecondChange Value が close talk と同一の talk 経路（一意採番＋StartTalk）を通ることの直接証拠で
/// ある。最初の steady Tick は確実に `Steady{None}` の GET（index 0）で Value を返す。
#[test]
fn sporadic_value_starts_unique_talk() {
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting();

    // quit_flags: [steady=false, close=true]。到達順は steady→close。
    let driven = drive_steady(fixture, 8, vec![false, true]);

    assert_eq!(
        driven.started.len(),
        2,
        "steady talk・close talk の 2 本が到達するはず（挨拶なし boot）: {:?}",
        driven.started
    );

    let steady_talks: Vec<&StartTalk> = driven
        .started
        .iter()
        .filter(|s| s.script == FIXED_STEADY_SCRIPT)
        .collect();
    assert_eq!(
        steady_talks.len(),
        1,
        "steady fixture スクリプトの talk がちょうど 1 本到達するはず（散発 Value 1 回）: {:?}",
        driven.started
    );
    let steady_talk = steady_talks[0];
    assert_eq!(
        steady_talk.talk_id,
        TalkId(1),
        "steady talk は先頭採番 id=1（挨拶なし boot ゆえ最初の StartTalk）"
    );

    let close_talks: Vec<&StartTalk> = driven
        .started
        .iter()
        .filter(|s| s.script == FIXED_FAREWELL_SCRIPT)
        .collect();
    assert_eq!(
        close_talks.len(),
        1,
        "close talk がちょうど 1 本到達するはず: {:?}",
        driven.started
    );
    let close_talk = close_talks[0];
    assert!(
        close_talk.talk_id.0 > steady_talk.talk_id.0,
        "talk_id は単調増番（steady {:?} < close {:?}）",
        steady_talk.talk_id,
        close_talk.talk_id
    );

    // steady talk は OnSecondChange GET（Ref3=1）から起きた: events 表から導出した GET が記録列に現れる。
    let expected_get = expected_call(events::on_second_change(
        MonotonicMs(1_000),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert!(
        driven.recorded.contains(&expected_get),
        "events 表導出の GET（Ref3=1・Status なし）と一致する OnSecondChange GET が記録列に現れるはず: {:?}",
        driven.recorded
    );

    // Unload を除く全 GET/NOTIFY の id が許可集合の要素であること。submit ガードと同じく
    // 「イベント許可 ∨ リソース許可」で判定する（boot prefetch の `username` はリソース許可集合
    // `ALLOWED_RESOURCE_IDS` を通る）。`OnTalk`／`OnHour` はどちらにも無いので、自発トークが確実に
    // 発火したこの run でそれらが送られていないこともこの掃引が見る。
    for call in &driven.recorded {
        if call.method == CallMethod::Unload {
            // Unload は events 表対象外の正規終了経路ゆえ除外する。
            continue;
        }
        assert!(
            events::is_allowed_event_id(&call.id) || resources::is_allowed_resource_id(&call.id),
            "記録された SHIORI id={} がイベント許可 {:?} にもリソース許可 {:?} にも属さない: {:?}",
            call.id,
            events::ALLOWED_EVENT_IDS,
            resources::ALLOWED_RESOURCE_IDS,
            driven.recorded
        );
    }
}

/// 再生中 Tick → NOTIFY（Ref3=0）・応答無視で StartTalk なし。
///
/// 挨拶なし boot で boot を `Steady{None}` へ直行させ、保留機能付きハーネス（`spawn_harness_gated`）で
/// steady talk の TalkDone を保留（park）し、「talk を 1 本 active に保ったまま Tick を挟む」窓を
/// 決定的に作る。手順:
///
/// 1. Boot → 挨拶なし（`Steady{None}` 直行・StartTalk なし）。
/// 2. Tick 1（now=1s）: `Steady{None}` → OnSecondChange GET → fixture が Value（steady_value_indices
///    =[0]）→ steady talk（受領 index 0・保留）→ `Steady{Some}`。
/// 3. TalkDone は保留され inbox へ届かないため、次 Tick は必ず `Steady{Some}` から処理される。
/// 4. Tick 2（now=2s）: `Steady{Some}` → OnSecondChange NOTIFY（Ref3=0・Status: talking）→ fixture
///    Notified（応答は構造的に破棄）→ `Steady{Some}` 維持。これが本テストの観測対象。
/// 5. `release_all` で保留 TalkDone（quit:false）を解放 → `Steady{None}` へ復帰。
/// 6. CloseRequest → OnClose GET → 別れの Value → close talk（受領 index 1・quit:true）→ 終了系列完走。
///
/// close 指示は release と非同期だが、両順序（TalkDone 先着＝即 `Steady{None}`→握手／CloseRequest
/// 先着＝`pending_close` 記録→TalkDone 消化→握手）とも同一の終了へ収束する。
///
/// # 非空虚性
/// - active talk 中に GET を出していたら（NOTIFY でなければ）、events 導出の NOTIFY と一致する
///   記録が現れず (2) の assert が落ちる。
/// - NOTIFY tick が talk を起こしていたら steady スクリプトの StartTalk が 2 本になり (3) が落ちる。
#[test]
fn active_talk_tick_emits_notify_ref3_zero() {
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting();

    // 受領 index 0（steady talk）の TalkDone を保留し、active talk 窓を作る。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    send(&harness, KanadeMsg::Boot);
    tick(&harness, 1_000);
    tick(&harness, 2_000);
    gate.release_all();
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );

    let Driven {
        recorded, started, ..
    } = harness.finish("kanade steady notify join");

    let steady_starts = started
        .iter()
        .filter(|s| s.script == FIXED_STEADY_SCRIPT)
        .count();
    assert_eq!(
        steady_starts, 1,
        "steady talk は Value tick の 1 本のみ・NOTIFY tick は StartTalk を起こさない（DD-6）: {:?}",
        started
    );

    // Ref3=0 と Status: talking は同一スナップショットから出るので、events 導出の NOTIFY
    // （now=2s・talk_active=true）と一致する記録が現れる。
    let expected_notify = expected_call(events::on_second_change(
        MonotonicMs(2_000),
        &ExecutionSnapshot {
            talk_active: true,
            ..ExecutionSnapshot::INACTIVE
        },
    ));
    assert!(
        recorded.contains(&expected_notify),
        "active talk 中の Tick は OnSecondChange NOTIFY（Ref3=0）を発行するはず（GET でない）: {:?}",
        recorded
    );

    // GET OnSecondChange は Value を起こした Tick 1 の 1 回のみ（NOTIFY tick が GET を出していない裏取り）。
    let get_count = count_second_change_get(&recorded);
    assert_eq!(
        get_count, 1,
        "OnSecondChange GET は Value を起こした Tick 1 の 1 回のみ（NOTIFY tick は GET を出さない）: {:?}",
        recorded
    );

    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（NOTIFY 窓の後も close→quit で正常完走）"
    );
}

/// ブロッキング呼出中の Tick catch-up（in-flight ≤ 1）。
///
/// mock は既定で即応するため実経路の「呼出ブロック中」窓は生じない。本テストは
/// `spawn_harness_blocking` で最初の `OnSecondChange` GET を明示解放まで握り、その窓を決定的に
/// 作る。手順:
///
/// 1. Boot → `Steady{None}` 直行（挨拶なし boot）。
/// 2. Tick 1（now=1h）: OnSecondChange GET（Ref3=1）を発行 → mock がこの GET を握る → kanade は
///    drive ループの `ReplyReceiver::recv` でブロックする。`gate.wait_until_blocked(DEFAULT_TIMEOUT)`
///    で「kanade がブロック済み」を有界に確認する（sleep なし）。
/// 3. Tick 2（now=2h）・Tick 3（now=3h）を送る → kanade はブロック中で inbox を消費しないため、
///    この 2 件は inbox に溜まる（catch-up の対象）。
/// 4. `gate.release()` → Tick 1 の GET に 204 が返り、溜まっていた Tick 2・Tick 3 を順次処理する。
/// 5. CloseRequest{User} → OnClose GET → 別れの Value → close talk（quit:true）→ 終了系列完走。
///
/// # 非空虚性
/// - OnSecondChange GET がちょうど 3 件（注入 Tick 1 本につき 1 件・欠落なし・重複なし）。
///   1 件でも失われれば < 3、二重発火すれば > 3 になる。
/// - 3 件は処理順（now 昇順）で現れ、各々 Ref3="1" の GET。Ref0 は `now/3_600_000`（時）で
///   Tick ごとに 1→2→3 と異なるので順序を構造的に証明する（events 表導出）。
/// - OnSecondChange NOTIFY は 1 件も現れない（active talk なし）。
#[test]
fn blocking_call_ticks_catch_up_in_order_without_loss_or_duplication() {
    // OnSecondChange GET は常に 204（steady_value_indices 空）＝catch-up 観測を汚さない。
    // 挨拶 talk が active だと Tick 1 が NOTIFY になりブロック対象の GET が空振りして
    // wait_until_blocked が成立しないので、挨拶なし boot にする。
    let (harness, gate) = spawn_harness_blocking(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        // close talk が先頭 StartTalk＝index 0。steady talk は 204 基調ゆえ生じない。
        QuitPolicy::PerTalk(vec![true]),
        "OnSecondChange",
    );

    // Ref0（now/3_600_000）を 1→2→3 と異ならせ、順序を構造的に検証する。
    let now1 = MonotonicMs(3_600_000); // 1h → Ref0="1"
    let now2 = MonotonicMs(2 * 3_600_000); // 2h → Ref0="2"
    let now3 = MonotonicMs(3 * 3_600_000); // 3h → Ref0="3"

    send(&harness, KanadeMsg::Boot);
    send(&harness, KanadeMsg::Tick { now: now1 });

    assert!(
        gate.wait_until_blocked(DEFAULT_TIMEOUT),
        "kanade は Tick 1 の OnSecondChange GET 往復でブロックするはず（wait_until_blocked が成立）"
    );

    send(&harness, KanadeMsg::Tick { now: now2 });
    send(&harness, KanadeMsg::Tick { now: now3 });
    gate.release();

    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );

    let Driven { recorded, .. } = harness.finish("kanade blocking catch-up join");

    let second_change_gets: Vec<&RecordedCall> = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Get && c.id == "OnSecondChange")
        .collect();

    // Ref0 の値そのものは `events_tests.rs` の `on_second_change_*` が見る。
    let expected_sequence: Vec<RecordedCall> = [now1, now2, now3]
        .into_iter()
        .map(|n| expected_call(events::on_second_change(n, &ExecutionSnapshot::INACTIVE)))
        .collect();
    let actual_sequence: Vec<RecordedCall> =
        second_change_gets.iter().map(|c| (*c).clone()).collect();
    assert_eq!(
        actual_sequence, expected_sequence,
        "OnSecondChange GET 3 件は now 昇順（Ref0=1,2,3）で現れるはず（catch-up の順序保存）: {:?}",
        recorded
    );

    // NOTIFY が現れれば二重処理か Steady{Some} への誤遷移。
    let notify_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Notify && c.id == "OnSecondChange")
        .count();
    assert_eq!(
        notify_count, 0,
        "active talk が無い catch-up では OnSecondChange NOTIFY は 1 件も現れないはず: {:?}",
        recorded
    );
}

/// 完了後の復帰: steady talk 完了（TalkDone 着弾）→ 次 Tick で `OnSecondChange` GET pump が再開する
/// （Ref3=1・Status なし）。
///
/// `active_talk_tick_emits_notify_ref3_zero` は active talk 窓での NOTIFY までを観測するが、TalkDone
/// 着弾後の GET 再開（次 Tick）は観測しない。close_test の
/// `farewell_talk_without_quit_tag_still_terminates` は別れの台詞（`CloseTalkWait`）が定常へ戻らず
/// 終了することを観測するもので、steady talk 完了（`Steady{Some}`→TalkDone→`Steady{None}`）からの
/// GET 再開は本テストだけが統合層で見る（純粋状態機械では `steady_flow_tests.rs` の
/// `steady_talk_done_ended_resumes_steady_and_pump_restarts` が被覆済み）。
///
/// # 決定的駆動
/// 挨拶なし boot で `Steady{None}` 直行。保留ハーネスで最初の steady talk（受領 index 0）を保留し
/// active 窓を作る。`with_steady_value_indices([0, 1])`: GET 出現 index 0＝保留 talk を起こし、index 1＝
/// 復帰後の pump が起こす talk（quit:true）で終了を駆動する。
///
/// 1. Boot → `Steady{None}`。
/// 2. Tick 1（1s）: GET(occ0)→Value→steady talk 0（id=1・保留）→`Steady{Some}`。
/// 3. Tick 2（2s）: `OnSecondChange` NOTIFY（Ref3=0・active 窓）。
/// 4. `release_all` → 保留 talk 0 の TalkDone{Ended}（quit:false）着弾 → `Steady{None}` 復帰。
/// 5. 復帰後 Tick → GET(occ1・Ref3=1・Status なし）→ Value → steady talk 1（id=2・quit:true）→
///    終了系列完走。
///
/// 復帰後 Tick は `drive_ticks_until_disconnect` が 1 秒刻みで送り続け、kanade が Receiver を drop して
/// send が Err になった時点で戻る。復帰の表明は駆動ループ内では行わず、join 後の最終記録列に対してのみ
/// 評価する。
///
/// # 非空虚性
/// - 復帰後 pump が NOTIFY のままなら Value が破棄され talk 1 が起きず終了が駆動されない。send は成功し
///   続け、`drive_ticks_until_disconnect` が `DEFAULT_TIMEOUT` の壁時計 deadline に到達して panic する。
/// - active 窓（NOTIFY）が無ければ join 後の `resumed_get_after_active_window` が `None`＝(2) が落ちる。
#[test]
fn talk_completion_resumes_get_pump_ref3_one_status_none() {
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0, 1])
        .without_boot_greeting();

    // index0（保留 steady talk）=false（Ended で復帰）・index1（復帰後 steady talk）=true（Quit で終了）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    send(&harness, KanadeMsg::Boot);
    tick(&harness, 1_000);
    tick(&harness, 2_000);
    gate.release_all();

    // 復帰前（Steady{Some}）の Tick は NOTIFY のみで、復帰後の GET(occ1) が Value を返して終了を駆動する。
    drive_ticks_until_disconnect(&harness.sender, 3, "talk_completion resume drive");

    let Driven {
        recorded, started, ..
    } = harness.finish("kanade resume join");

    assert_eq!(
        started.len(),
        2,
        "保留 talk と復帰後 talk のちょうど 2 本が配送されるはず: {:?}",
        started
    );
    assert!(
        started.iter().all(|s| s.script == FIXED_STEADY_SCRIPT),
        "2 本とも steady スクリプト（close talk は起きない）: {:?}",
        started
    );
    assert_eq!(started[0].talk_id, TalkId(1), "保留 talk は先頭採番 id=1");
    assert_eq!(
        started[1].talk_id,
        TalkId(2),
        "復帰後 talk は id=2（単調増番）"
    );

    // active 窓（OnSecondChange NOTIFY）より後に OnSecondChange GET が現れ、その Ref3・Status が
    // events 表導出の shape（GET・Ref3=1・Status なし）と一致する。復帰後の GET の now はループ依存
    // なので、now 非依存の Ref3／Status のみを照合する。
    let resumed_get = resumed_get_after_active_window(&recorded)
        .expect("active 窓の後に pump 再開 GET が現れるはず（Req 4.4）");
    let expected_shape = expected_call(events::on_second_change(
        MonotonicMs(0),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(
        resumed_get.references.len(),
        4,
        "OnSecondChange の References は 4 要素"
    );
    assert_eq!(
        resumed_get.references[3], expected_shape.references[3],
        "再開 pump の Ref3 は events 導出値（\"1\"・GET・talk 再生可能）"
    );
    assert_eq!(
        resumed_get.status, expected_shape.status,
        "再開 pump（Steady{{None}}）は Status 行を出さない（events 導出 None）"
    );

    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（復帰後 pump talk quit:true の終了系列完走）で閉じるはず"
    );
}
