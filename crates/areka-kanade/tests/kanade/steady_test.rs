//! 定常運転の統合検証（pump ゲート・talk 調停・Value→StartTalk）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、
//! 定常運転（`Phase::Steady`）の観測可能な振る舞いを 2 パターン、単一の合否として検証する:
//!
//! - **応答なし（204）→ 再生起動要求なし**（Req 2.3）: `OnSecondChange` GET（Ref3=1）が
//!   204 を返す間、boot talk 以外の再生起動要求は宛先へ一切到達しない。
//! - **散発的 Value → 一意 talk_id 付きの再生起動要求**（Req 2.1・3.3）: 特定の
//!   `OnSecondChange` GET 出現で fixture が Value を返すと、boot talk とは別の一意 talk_id を
//!   持つ steady talk が宛先へ到達する（＝OnSecondChange Value が boot と同一 talk 経路を通る・
//!   Req 3.3）。
//! - **再生中 Tick → NOTIFY（Ref3=0）・応答無視で StartTalk なし**（Req 3.3・DD-6）: talk を
//!   1 本 active に保ったまま Tick を挟むと、`OnSecondChange` は NOTIFY（Ref3=0）で発行され、
//!   その応答は構造的に破棄されて再生起動要求を一切生まない（active talk 中の重複調停を
//!   発生源から断つ・DD-6）。
//!
//! # 決定性（Req 7.3）と同期イディオム
//! full_run_test.rs／boot_test.rs と同じ手順で決定的に同期する: quit シナリオ
//! （`Fixture::quitting()`＋末尾 talk quit:true）で Boot→Tick…→CloseRequest を送り、
//! close talk の完了が終了系列（Unload→StopSelf）を駆動する。kanade の期限付き join が
//! 成功すれば、それまでの全 shiori 呼出・全 StartTalk 配送は確定済みであり、実時間 sleep を
//! 一切用いずに記録列を確定できる。
//!
//! # 挨拶なし boot で steady Tick の決定性を保つ（DD-IT-12）
//! DD-IT-12 で boot は起動挨拶 talk を**正規追跡**するようになった（挨拶 Value 経路は
//! `Steady{talk: Some(挨拶)}` へ完了し、挨拶の TalkDone が slot と照合される）。その結果、挨拶 talk の
//! TalkDone（mock sakura が別スレッドから返す）と、Boot の後に注入する steady Tick とが inbox 上で
//! 競合し、最初期の Tick が `Steady{Some}`（NOTIFY・Ref3=0）か `Steady{None}`（GET・Ref3=1）かが
//! 非決定になる。定常 pump（`Steady{None}` の GET）の決定的観測を要する本ファイルの各テストは、
//! `Fixture::without_boot_greeting()`（`OnBoot`→204）で boot を `Steady{talk: None}` へ直行させ、この
//! 競合を発生源から断つ（設計 Testing Strategy「boot が 204 を返す fixture」）。挨拶を出さない boot は
//! StartTalk を一切生まないため、Boot の後に処理される steady Tick は必ず `Steady{None}` から始まり
//! GET（Ref3=1）を発行する（cross-thread な TalkDone が存在しない＝決定的）。挨拶 boot 自体の観測は
//! boot_test／full_run_test が担う。
//!
//! # active talk 中 Tick の NOTIFY（Ref3=0）——統合層で被覆する
//! 「再生中の Tick が OnSecondChange を NOTIFY（Ref3=0・応答無視）で発行する」（DD-6・Req 3.3）は
//! 純粋状態機械のユニットテスト（`src/schedule/steady.rs::tests::steady_some_tick_emits_notify`／
//! `steady_some_value_is_discarded_without_start_talk`）でも被覆されるが、本ファイルは**統合層でも**
//! これを決定的に検証する（`active_talk_tick_emits_notify_ref3_zero`）。
//!
//! 決定的な active-talk 窓は、保留機能付きハーネス（`spawn_harness_gated`＋`SakuraGate`）で作る:
//! steady talk の TalkDone を保留（park）することで、当該 talk は kanade の inbox へ完了通知を返さ
//! ない。TalkDone は「二つの Tick の間に割り込み得る唯一のメッセージ」であり、これを送らない限り
//! 次 Tick は必ず `Steady{Some}` から処理され NOTIFY（Ref3=0）を発行する（interleaving が起きない・
//! sleep も wall-clock も用いない）。観測後に保留を解放（`release_all`）すれば `Steady{None}` へ復帰
//! し、以降は 2 パターンと同じ quit 経路（OnClose→別れの talk→終了）で記録列を確定できる。

use std::collections::HashSet;

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, StartTalk, TalkId, events,
};

use super::common::{
    BlockOn, CallMethod, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, Fixture,
    Harness, QuitPolicy, RecordedCall, join_bounded, spawn_harness, spawn_harness_blocking,
    spawn_harness_gated,
};

/// 駆動結果: 確定した shiori 記録列と、宛先へ到達した StartTalk 列。
struct Driven {
    recorded: Vec<RecordedCall>,
    started: Vec<StartTalk>,
}

/// Boot→（steady Tick を tick_count 回）→CloseRequest を駆動し、終了完走まで待って記録を確定する。
///
/// `fixture` は close_quits=true（`Fixture::quitting()` 由来）かつ**挨拶なし boot**
/// （`without_boot_greeting()`・DD-IT-12 の race を断つ）である前提。close talk を末尾 talk として
/// quit:true にすることで終了系列（Unload→StopSelf）が完走し、kanade の期限付き join 成功時点で
/// 全 shiori 呼出・全 StartTalk 配送が確定する（実時間 sleep なし）。
///
/// 挨拶なし boot は StartTalk を生まないため、sakura が受領する talk は「（発生し得る）steady talk・
/// close talk」の順のみ。quit_flags はこの順で quit を割り当てる（boot talk は存在しない）。close talk
/// が末尾に来るよう十分な false を前置し、末尾に唯一の true（close talk）を置くこと。
fn drive_steady(fixture: Fixture, tick_count: u64, quit_flags: Vec<bool>) -> Driven {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(quit_flags),
    );

    // 起動指示（boot 系列→OnBoot Value→boot talk 起動）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // 毎秒 Tick を数回（定常運転の pump を駆動）。時刻は注入 Tick のみで進行させる。
    for i in 1..=tick_count {
        harness
            .sender
            .send(KanadeMsg::Tick {
                now: MonotonicMs(i * 1_000),
            })
            .expect("send Tick");
    }

    // close 指示。active talk なしなら即握手（OnClose GET→別れの Value→close talk→quit:true）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 終了系列完走（StopSelf 到達）まで期限付き join。成功時点で全記録・全配送が確定する。
    join_bounded("kanade steady join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence (Unload→StopSelf)");

    // kanade 停止後に送信端 drop → sakura sink スレッドも自然終了（期限付き join）。
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura steady join", DEFAULT_TIMEOUT);

    Driven {
        recorded: shiori.recorded(),
        started,
    }
}

/// 記録列中の `OnSecondChange` GET（Ref3=1・pump 問い合わせ）の出現数を数える。
fn count_second_change_get(recorded: &[RecordedCall]) -> usize {
    recorded
        .iter()
        .filter(|c| c.method == CallMethod::Get && c.id == "OnSecondChange")
        .count()
}

/// 応答なし（204）→ 再生起動要求が発生しない（Req 2.3）。
///
/// 挨拶なし boot（`without_boot_greeting`・`OnBoot`→204）で boot を `Steady{None}` へ直行させ、
/// steady_value_indices 空＝OnSecondChange GET は常に 204 の構成で Boot→複数 Tick→CloseRequest を
/// 駆動する。挨拶 talk が無いため steady Tick は必ず `Steady{None}` から GET を発行し（DD-IT-12 の
/// 挨拶 talk race を発生源から断つ）、204 基調ゆえ一切 talk を起こさない。ゆえに宛先へ到達する
/// StartTalk は close talk ちょうど 1 本のみであり、steady talk は 1 本も挟まらない（＝steady 204 は
/// 再生起動要求ゼロ・Req 2.3）。
///
/// この不変量は cross-thread な TalkDone に依存しない: 挨拶なし boot は StartTalk を生まず、Value を
/// 返す OnSecondChange 応答も 1 度もない以上、steady talk は構造的に生じ得ない（決定的）。
#[test]
fn steady_no_content_produces_no_start_talk() {
    // 挨拶なし boot＋204 基調 steady。sakura が受領する唯一の talk は close talk（index 0）＝quit:true
    // で終了駆動（PerTalk は範囲外 index を false 扱い＝万一 steady talk が誤って生じても quit しない＝
    // この テストの assert がその誤りを検出する）。
    let driven = drive_steady(Fixture::quitting().without_boot_greeting(), 4, vec![true]);

    // (1) 宛先へ到達した StartTalk は close talk ちょうど 1 本（挨拶なし boot・204 基調で talk ゼロ）。
    assert_eq!(
        driven.started.len(),
        1,
        "挨拶なし boot・204 基調では steady talk が起きず、close talk 1 本のみが到達するはず: {:?}",
        driven.started
    );

    // 唯一の talk は別れの close talk（Req 2.3 の直接証拠: steady 由来の talk は 1 本も生じない）。
    assert_eq!(
        driven.started[0].script, FIXED_FAREWELL_SCRIPT,
        "唯一到達する StartTalk は close talk（別れの fixture スクリプト）"
    );

    // steady talk スクリプトを持つ StartTalk は 1 本も到達しない（Req 2.3 の直接検証）。
    assert!(
        driven
            .started
            .iter()
            .all(|s| s.script != FIXED_STEADY_SCRIPT),
        "204 基調では steady fixture スクリプトの StartTalk は到達しないはず: {:?}",
        driven.started
    );

    // (2) 204 の間も pump の問い合わせ（OnSecondChange GET・Ref3=1）は現に発行されている
    //     （＝ゲートが Steady{None} で開き、GET を出し、204 で talk を起こさなかった経路の証拠）。
    let get_count = count_second_change_get(&driven.recorded);
    assert!(
        get_count >= 1,
        "204 経路でも OnSecondChange GET（Ref3=1）が少なくとも 1 度は発行されるはず: {:?}",
        driven.recorded
    );

    // GET の Reference 構成が events 表の正典（Ref3="1"）と一致することを 1 件抽出して確認する
    //   （時刻依存の Ref0 はここでは問わず、Ref3=1＝pump 問い合わせであることを検証する）。
    let a_get = driven
        .recorded
        .iter()
        .find(|c| c.method == CallMethod::Get && c.id == "OnSecondChange")
        .expect("OnSecondChange GET が記録列に存在するはず");
    assert_eq!(
        a_get.references.len(),
        4,
        "OnSecondChange の References は 4 要素（Ref0..Ref3）"
    );
    assert_eq!(
        a_get.references[3], "1",
        "GET pump の Ref3 は \"1\"（talk 再生可能・問い合わせ）"
    );
    // pump GET は talk 非アクティブゆえ Status ヘッダを持たない（DD-IT-3/DD-IT-5・5.1）。
    assert_eq!(
        a_get.status, None,
        "Steady{{None}} の pump GET は Status 行を出さない（talk 非アクティブ）"
    );
}

/// 散発的な Value → 一意 talk_id 付きの再生起動要求（Req 2.1・3.3）。
///
/// 挨拶なし boot（`without_boot_greeting`）で boot を `Steady{None}` へ直行させ、
/// `with_steady_value_indices([0])` で最初の OnSecondChange GET 出現に Value を仕込む。Boot→十分な
/// Tick→CloseRequest を駆動すると、その GET 出現で steady talk が 1 本起動し、一意な talk_id を持つ
/// StartTalk が宛先へ到達する。これは OnSecondChange Value が close talk と同一の talk 経路（一意採番＋
/// StartTalk）を通ることの直接証拠である（Req 3.3）。挨拶なし boot は StartTalk を生まないため、最初の
/// steady Tick は確実に `Steady{None}` の GET（index 0）で Value を返す（DD-IT-12 の race を断つ・決定的）。
///
/// close talk（quit:true）で終了系列を駆動し、期限付き join 成功時点で記録を確定する。
#[test]
fn sporadic_value_starts_unique_talk() {
    // idx=0: 最初の OnSecondChange GET 出現に Value。挨拶なし boot ゆえ最初の steady Tick は Steady{None}
    // の GET（index 0）で確実に Value を返し、steady talk を 1 本だけ起こす。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting();

    // quit_flags: [steady=false, close=true]。到達順は steady→close の想定（挨拶 talk は無い）。
    // close talk を末尾 talk（quit:true）にして終了系列を駆動する。
    let driven = drive_steady(fixture, 8, vec![false, true]);

    // (1) 宛先へ到達した StartTalk は steady talk・close talk のちょうど 2 本（挨拶 talk なし）。
    assert_eq!(
        driven.started.len(),
        2,
        "steady talk・close talk の 2 本が到達するはず（挨拶なし boot）: {:?}",
        driven.started
    );

    // (2) steady talk が fixture の steady スクリプトで、ちょうど 1 本到達する（Req 2.1）。先頭採番 id=1。
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

    // (3) close talk（別れのスクリプト）も 1 本到達し、その talk_id は steady talk と別＝一意（Req 2.1）。
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
    assert_ne!(
        steady_talk.talk_id, close_talk.talk_id,
        "steady talk の talk_id は close talk と別（一意）"
    );
    assert!(
        close_talk.talk_id.0 > steady_talk.talk_id.0,
        "talk_id は単調増番（steady {:?} < close {:?}）",
        steady_talk.talk_id,
        close_talk.talk_id
    );

    // (4) 到達した全 talk_id が互いに一意（steady・close が id を共有しない・Req 2.1）。
    let ids: HashSet<u64> = driven.started.iter().map(|s| s.talk_id.0).collect();
    assert_eq!(
        ids.len(),
        driven.started.len(),
        "全 StartTalk の talk_id は一意（再利用しない）: {:?}",
        driven.started
    );

    // (5) steady talk は OnSecondChange GET（Ref3=1）から起きた＝GET pump が現に発行されている。
    //     Value を返す構成でも、まず GET（Ref3=1）で問い合わせる経路を通る（Req 3.3・同一 talk 経路）。
    let get_count = count_second_change_get(&driven.recorded);
    assert!(
        get_count >= 1,
        "Value を起こすには OnSecondChange GET（Ref3=1）が少なくとも 1 度発行されるはず: {:?}",
        driven.recorded
    );

    // events 表から導出した GET（Ref3=1・talk 非アクティブ＝Status 行なし）の Reference 構成が記録列に
    //   現れる（now=1s は sub-hour ゆえ Ref0="0"・ドレイン中の任意 sub-hour GET と一致する）。
    let expected_get = RecordedCall {
        method: CallMethod::Get,
        id: "OnSecondChange".to_string(),
        references: match events::on_second_change(
            MonotonicMs(1_000),
            &ExecutionSnapshot { talk_active: false },
        ) {
            areka_kanade::ShioriCall::Get { references, .. } => references,
            _ => panic!("on_second_change（talk 非アクティブ）は GET を返すはず"),
        },
        status: None,
    };
    assert!(
        driven.recorded.iter().any(|c| *c == expected_get),
        "events 表導出の GET（Ref3=1・Status なし）と一致する OnSecondChange GET が記録列に現れるはず: {:?}",
        driven.recorded
    );
}

/// 再生中 Tick → NOTIFY（Ref3=0）・応答無視で StartTalk なし（Req 3.3・DD-6）。
///
/// 挨拶なし boot（`without_boot_greeting`）で boot を `Steady{None}` へ直行させ（DD-IT-12 の挨拶 talk
/// race を発生源から断つ）、保留機能付きハーネス（`spawn_harness_gated`）で steady talk の TalkDone を
/// 保留（park）し、「talk を 1 本 active に保ったまま Tick を挟む」窓を決定的に作る。手順:
///
/// 1. Boot → 挨拶なし（`Steady{None}` 直行・StartTalk なし・cross-thread TalkDone なし）。
/// 2. Tick 1（now=1s）: `Steady{None}` → OnSecondChange GET → fixture が Value（steady_value_indices
///    =[0]・最初の GET 出現）→ steady talk（受領 index 0・**保留**）→ `Steady{Some}`。
/// 3. steady talk の TalkDone は保留され inbox へ届かないため、次 Tick は必ず `Steady{Some}` から
///    処理される（interleaving なし）。
/// 4. Tick 2（now=2s）: `Steady{Some}` → OnSecondChange **NOTIFY**（Ref3=0・Status: talking）→ fixture
///    Notified（応答は構造的に破棄）→ `Steady{Some}` 維持。これが本テストの観測対象。
/// 5. `release_all` で保留 TalkDone（quit:false）を解放 → `Steady{None}` へ復帰。
/// 6. CloseRequest → OnClose GET → 別れの Value → close talk（受領 index 1・quit:true）→ 終了系列完走。
///
/// close 指示は release と非同期だが、両順序（TalkDone 先着＝即 `Steady{None}`→握手／CloseRequest
/// 先着＝`pending_close` 記録→TalkDone 消化→握手）とも同一の終了へ収束する（sleep 不要・race-free）。
///
/// # 非空虚性
/// - active talk 中に GET を出していたら（NOTIFY でなければ）、events 導出の NOTIFY と一致する
///   記録が現れず (2) の assert が落ちる。
/// - NOTIFY tick が talk を起こしていたら steady スクリプトの StartTalk が 2 本になり (3) が落ちる。
#[test]
fn active_talk_tick_emits_notify_ref3_zero() {
    // idx=0: 最初の OnSecondChange GET 出現に Value（Tick 1 で steady talk を 1 本起こす）。挨拶なし boot
    // で Tick 1 を確実に `Steady{None}` の GET にする（DD-IT-12 の race を断つ）。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([0])
        .without_boot_greeting();

    // 受領 index 0（steady talk）の TalkDone を保留し、active talk 窓を作る（挨拶 talk が無いため steady
    // talk が先頭 StartTalk＝index 0）。quit_flags: [steady=false, close=true]。到達順は steady→close。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    // 起動指示（boot 系列→OnBoot Value→boot talk・即 TalkDone＝未知 talk・無害）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // Tick 1（now=1s）: Steady{None}→GET→Value→steady talk（保留）→Steady{Some}。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send Tick 1");

    // Tick 2（now=2s）: Steady{Some}→NOTIFY（Ref3=0・応答無視）→Steady{Some} 維持。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(2_000),
        })
        .expect("send Tick 2");

    // 保留 TalkDone（quit:false）を解放 → Steady{None} へ復帰。
    gate.release_all();

    // close 指示（active talk なしなら即握手／pending 経由でも同一終了へ収束）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 終了系列完走（StopSelf 到達）まで期限付き join。成功時点で全記録・全配送が確定する。
    join_bounded("kanade steady notify join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence (Unload→StopSelf)");

    // kanade 停止後に送信端 drop → sakura sink スレッドも自然終了（保留は解放済み）。
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura gated steady join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) steady スクリプトの StartTalk はちょうど 1 本（Value tick 由来のみ・NOTIFY tick は起こさない）。
    let steady_starts = started
        .iter()
        .filter(|s| s.script == FIXED_STEADY_SCRIPT)
        .count();
    assert_eq!(
        steady_starts, 1,
        "steady talk は Value tick の 1 本のみ・NOTIFY tick は StartTalk を起こさない（DD-6）: {:?}",
        started
    );

    // (2) active talk 中の Tick が OnSecondChange NOTIFY（Ref3=0・Status: talking）を発行した——
    //     events 導出の NOTIFY（now=2s・talk_active=true）と一致する記録が現れる（ハードコードしない）。
    let expected_notify = RecordedCall {
        method: CallMethod::Notify,
        id: "OnSecondChange".to_string(),
        references: match events::on_second_change(
            MonotonicMs(2_000),
            &ExecutionSnapshot { talk_active: true },
        ) {
            areka_kanade::ShioriCall::Notify { references, .. } => references,
            _ => panic!("on_second_change（talk アクティブ）は NOTIFY を返すはず"),
        },
        // DD-IT-3: Ref3=0 と Status: talking は同一スナップショットから出る（不整合は構造的に不可能）。
        status: Some("talking".to_string()),
    };
    // Ref3="0"（talk 再生不能＝応答無視）であることを明示的に確認する。
    assert_eq!(
        expected_notify.references[3], "0",
        "NOTIFY pump の Ref3 は \"0\"（active talk 中・応答無視）"
    );
    assert!(
        recorded.iter().any(|c| *c == expected_notify),
        "active talk 中の Tick は OnSecondChange NOTIFY（Ref3=0）を発行するはず（GET でない）: {:?}",
        recorded
    );

    // (3) NOTIFY で発行された OnSecondChange は GET でない＝その出現は問い合わせでない。
    //     GET OnSecondChange の出現数は「Value を起こした Tick 1」の 1 回のみであることを確認
    //     （NOTIFY tick が GET を出していないことの裏取り・非空虚性の補強）。
    let get_count = count_second_change_get(&recorded);
    assert_eq!(
        get_count, 1,
        "OnSecondChange GET は Value を起こした Tick 1 の 1 回のみ（NOTIFY tick は GET を出さない）: {:?}",
        recorded
    );
}

/// ブロッキング呼出中の Tick catch-up（DD-2・in-flight ≤ 1・Req 3.1/3.2）。
///
/// mock は既定で即応するため実経路の「呼出ブロック中」窓は生じない。本テストは
/// `spawn_harness_blocking` で最初の `OnSecondChange` GET を明示解放まで握り、その窓を決定的に
/// 作る。手順:
///
/// 1. Boot → boot 系列（各呼出は即応・挨拶なし boot ゆえ StartTalk なし）→ `Steady{None}` 直行。
/// 2. Tick 1（now=1h）: `Steady{None}` → OnSecondChange GET（Ref3=1）を発行 → mock がこの GET を
///    握る → kanade は drive ループの `ReplyReceiver::recv` で**ブロック**する。
///    `gate.wait_until_blocked(DEFAULT_TIMEOUT)` で「kanade がブロック済み」を有界に確認する
///    （決定的バリア・sleep なし）。
/// 3. Tick 2（now=2h）・Tick 3（now=3h）を送る → kanade はブロック中で inbox を消費しないため、
///    この 2 件は inbox に**溜まる**（catch-up の対象）。
/// 4. `gate.release()` → 握られた Tick 1 の GET に 204 が返り、kanade は Tick 1 を畳んで
///    `Steady{None}` へ戻る → 溜まっていた Tick 2 → OnSecondChange GET（即 204）→ Tick 3 →
///    OnSecondChange GET（即 204）を順次処理する（catch-up・in-flight ≤ 1）。
/// 5. CloseRequest{User} → OnClose GET → 別れの Value → close talk（quit:true）→ 終了系列完走。
///
/// # 非空虚性（core assertion）
/// - OnSecondChange GET がちょうど **3 件**（注入 Tick 1 本につき 1 件・欠落なし・重複なし）。
///   1 件でも失われれば < 3、二重発火すれば > 3 になる（in-flight ≤ 1 ＋ catch-up の直接検証）。
/// - 3 件は inbox/処理順（now 昇順）で現れ、各々 Ref3="1" の GET（NOTIFY でない）。Ref0 は
///   `now/3_600_000`（時）で Tick ごとに 1→2→3 と異なる＝順序を構造的に証明する（events 表導出・
///   ハードコードしない）。
/// - OnSecondChange NOTIFY は 1 件も現れない（active talk なし＝全 3 件 GET・二重処理なし）。
#[test]
fn blocking_call_ticks_catch_up_in_order_without_loss_or_duplication() {
    // OnSecondChange GET は常に 204（steady_value_indices 空）＝catch-up 観測を汚さない。
    // 挨拶なし boot（without_boot_greeting）で boot→Steady{None} へ直行させ、Tick 1 を確実に
    // `Steady{None}` の GET にする（DD-IT-12 の挨拶 talk race を断つ・挨拶 talk が active だと Tick 1 が
    // NOTIFY になり block_on が空振りして wait_until_blocked が成立しない）。quit シナリオ
    // （OnClose→別れの Value→close talk）で終了系列を駆動する。
    let (harness, gate) = spawn_harness_blocking(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        // quit_flags: [close=true]。挨拶なし boot ゆえ close talk が先頭 StartTalk＝index 0。steady talk は
        // 204 基調ゆえ生じない（PerTalk 範囲外 index は false＝万一 steady talk が生じても quit しない）。
        QuitPolicy::PerTalk(vec![true]),
        BlockOn::Get("OnSecondChange"),
    );

    // 各 Tick の now は時（hour）境界に置き、Ref0（now/3_600_000）を 1→2→3 と異ならせる
    // （順序を構造的に検証するため）。
    let now1 = MonotonicMs(3_600_000); // 1h → Ref0="1"
    let now2 = MonotonicMs(2 * 3_600_000); // 2h → Ref0="2"
    let now3 = MonotonicMs(3 * 3_600_000); // 3h → Ref0="3"

    // 起動指示（boot 系列は即応で `Steady{None}` まで完走・挨拶なし boot ゆえ StartTalk なし）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // Tick 1: Steady{None} → OnSecondChange GET → mock が握る → kanade は round-trip でブロック。
    harness
        .sender
        .send(KanadeMsg::Tick { now: now1 })
        .expect("send Tick 1");

    // kanade が Tick 1 の GET round-trip でブロック済みであることを有界に確認（決定的バリア）。
    assert!(
        gate.wait_until_blocked(DEFAULT_TIMEOUT),
        "kanade は Tick 1 の OnSecondChange GET 往復でブロックするはず（wait_until_blocked が成立）"
    );

    // Tick 2・Tick 3 を送る（kanade はブロック中＝inbox に溜まる＝catch-up の対象）。
    harness
        .sender
        .send(KanadeMsg::Tick { now: now2 })
        .expect("send Tick 2");
    harness
        .sender
        .send(KanadeMsg::Tick { now: now3 })
        .expect("send Tick 3");

    // 解放 → Tick 1 の GET に 204 が返り、溜まっていた Tick 2・Tick 3 を順次処理する（catch-up）。
    gate.release();

    // close 指示（active talk なし＝即握手 OnClose GET→別れの Value→close talk→quit:true→終了）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 終了系列完走（StopSelf 到達）まで期限付き join。成功時点で全記録・全配送が確定する。
    join_bounded("kanade blocking catch-up join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence (Unload→StopSelf)");

    // kanade 停止後に送信端 drop → sakura sink スレッドも自然終了（期限付き join）。
    drop(sender);
    sakura.join_bounded("mock-sakura blocking join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // 記録列から OnSecondChange GET のみを処理順に抽出する。
    let second_change_gets: Vec<&RecordedCall> = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Get && c.id == "OnSecondChange")
        .collect();

    // (core) 注入 Tick 3 本につき OnSecondChange GET はちょうど 3 件——欠落なし・重複なし。
    //   < 3 なら tick 欠落（catch-up 破綻）、> 3 なら二重発火（in-flight ≤ 1 破綻）。
    assert_eq!(
        second_change_gets.len(),
        3,
        "注入 Tick 3 本に対し OnSecondChange GET はちょうど 3 件（catch-up・in-flight ≤ 1）: {:?}",
        recorded
    );

    // (order) 3 件は now 昇順（inbox/処理順）で現れ、各々 events 表導出の GET（Ref3=1）と一致する。
    //   Ref0 が 1→2→3 と異なる＝処理順が now 昇順であることを構造的に証明する（ハードコードしない）。
    for (tick_now, expected_ref0) in [(now1, "1"), (now2, "2"), (now3, "3")] {
        let expected_get = RecordedCall {
            method: CallMethod::Get,
            id: "OnSecondChange".to_string(),
            references: match events::on_second_change(
                tick_now,
                &ExecutionSnapshot { talk_active: false },
            ) {
                areka_kanade::ShioriCall::Get { references, .. } => references,
                _ => panic!("on_second_change（talk 非アクティブ）は GET を返すはず"),
            },
            // catch-up pump は talk 非アクティブ（Steady{None}）＝Status 行なし。
            status: None,
        };
        // events 導出の Ref0 が期待どおり時境界の hour（1/2/3）であることを明示確認。
        assert_eq!(
            expected_get.references[0], expected_ref0,
            "OnSecondChange GET の Ref0（時）は now/3_600_000＝{}",
            expected_ref0
        );
        assert_eq!(
            expected_get.references[3], "1",
            "GET pump の Ref3 は \"1\"（talk 再生可能・問い合わせ）"
        );
    }

    // 3 件が処理順（now 昇順・Ref0=1,2,3）で並ぶことを構造照合する。
    let expected_sequence: Vec<RecordedCall> = [now1, now2, now3]
        .into_iter()
        .map(|n| RecordedCall {
            method: CallMethod::Get,
            id: "OnSecondChange".to_string(),
            references: match events::on_second_change(
                n,
                &ExecutionSnapshot { talk_active: false },
            ) {
                areka_kanade::ShioriCall::Get { references, .. } => references,
                _ => panic!("on_second_change（talk 非アクティブ）は GET を返すはず"),
            },
            status: None,
        })
        .collect();
    let actual_sequence: Vec<RecordedCall> =
        second_change_gets.iter().map(|c| (*c).clone()).collect();
    assert_eq!(
        actual_sequence, expected_sequence,
        "OnSecondChange GET 3 件は now 昇順（Ref0=1,2,3）で現れるはず（catch-up の順序保存）: {:?}",
        recorded
    );

    // (no spurious NOTIFY) active talk が無い以上、OnSecondChange は全 3 件 GET＝NOTIFY は 0 件。
    //   NOTIFY が現れれば二重処理か Steady{Some} 誤遷移＝catch-up の不変量違反。
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
