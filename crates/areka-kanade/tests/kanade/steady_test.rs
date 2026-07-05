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
//! # boot talk は fire-and-forget（steady Tick との race がない根拠）
//! boot 系列は単一 `KanadeMsg::Boot` の処理内で**同期的に** `Steady{talk: None}` まで完走する
//! （DD-2 同期往復ループ・boot talk は fire-and-forget＝kanade は active talk として追跡しない・
//! `src/schedule/boot.rs` の `to_baseware_version`／`Phase::Steady { talk: None }` 完了）。ゆえに
//! Boot の次に処理される steady Tick は必ず `Steady{None}` から始まり、GET（Ref3=1）を発行する。
//! boot talk の TalkDone は後から未知 talk_id として届いても現 Phase を変えない（mod.rs
//! `current_talk_id`→None＝未知）。よって boot TalkDone と steady Tick の inbox 順序に関わらず
//! steady Tick の pump 経路は決定的である（本ファイルはこの決定性の上に立つ）。
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
    CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, StartTalk, TalkId, events,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_BOOT_SCRIPT, FIXED_STEADY_SCRIPT, Fixture, Harness,
    QuitPolicy, RecordedCall, join_bounded, spawn_harness, spawn_harness_gated,
};

/// 駆動結果: 確定した shiori 記録列と、宛先へ到達した StartTalk 列。
struct Driven {
    recorded: Vec<RecordedCall>,
    started: Vec<StartTalk>,
}

/// Boot→（steady Tick を tick_count 回）→CloseRequest を駆動し、終了完走まで待って記録を確定する。
///
/// `fixture` は close_quits=true（`Fixture::quitting()` 由来）である前提。close talk を末尾 talk と
/// して quit:true にすることで終了系列（Unload→StopSelf）が完走し、kanade の期限付き join 成功時点で
/// 全 shiori 呼出・全 StartTalk 配送が確定する（実時間 sleep なし）。
///
/// quit_flags は「boot talk・（発生し得る）steady talk・close talk」の順で quit を割り当てる。close
/// talk が末尾に来るよう十分な false を前置し、末尾に唯一の true（close talk）を置くこと。
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
/// `Fixture::quitting()`（steady_value_indices 空＝OnSecondChange GET は常に 204）で Boot→複数 Tick→
/// CloseRequest を駆動する。boot talk（OnBoot Value）は必ず 1 本起動するが、204 基調の steady Tick は
/// 一切 talk を起こさない。ゆえに宛先へ到達する StartTalk は boot talk と close talk のちょうど 2 本で
/// あり、その間に steady talk は 1 本も挟まらない（＝steady 204 は再生起動要求ゼロ・Req 2.3）。
///
/// この不変量は boot TalkDone と steady Tick の inbox 順序（interleaving）に依存しない: どの順序で
/// 処理されても、Value を返す OnSecondChange 応答が 1 度もない以上、steady talk は構造的に生じ得ない。
#[test]
fn steady_no_content_produces_no_start_talk() {
    // quit_flags: boot talk=false・close talk=true。steady talk は生じない前提ゆえ 2 要素で足りる
    // （PerTalk は範囲外 index を false 扱い＝万一 steady talk が誤って生じても quit しない＝この
    // テストの assert がその誤りを検出する）。
    let driven = drive_steady(Fixture::quitting(), 4, vec![false, true]);

    // (1) 宛先へ到達した StartTalk は boot talk と close talk のちょうど 2 本（steady talk なし）。
    assert_eq!(
        driven.started.len(),
        2,
        "204 基調では steady talk が起きず、boot talk と close talk の 2 本のみが到達するはず: {:?}",
        driven.started
    );

    // 先頭は boot talk（OnBoot Value・先頭採番 id=1）。
    assert_eq!(
        driven.started[0].script, FIXED_BOOT_SCRIPT,
        "先頭 StartTalk は boot talk（boot fixture スクリプト）"
    );
    assert_eq!(
        driven.started[0].talk_id,
        TalkId(1),
        "boot talk の talk_id は 1（先頭採番）"
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
}

/// 散発的な Value → 一意 talk_id 付きの再生起動要求（Req 2.1・3.3）。
///
/// `Fixture::quitting().with_steady_value_indices([idx])` で、`idx` 番目（0 始まり）の OnSecondChange
/// GET 出現に Value を仕込む。Boot→十分な Tick→CloseRequest を駆動すると、その GET 出現で steady talk が
/// 1 本起動し、boot talk とは別の一意 talk_id を持つ StartTalk が宛先へ到達する。これは OnSecondChange
/// Value が boot と同一の talk 経路（一意採番＋StartTalk）を通ることの直接証拠である（Req 3.3）。
///
/// idx=0（最初の GET 出現に Value）を選ぶことで、interleaving に依らず「GET が 1 度でも発行されれば
/// その初回で Value を返す」構成にし、steady talk がちょうど 1 本発生することを決定的にする。十分な
/// Tick 数（8）を送り、boot talk 完了後に GET pump が最低 1 度は回ることを保証する。
#[test]
fn sporadic_value_starts_unique_talk() {
    // idx=0: 最初の OnSecondChange GET 出現に Value。GET は Steady{None}（boot talk 完了後）でのみ
    // 発行されるため、この Value は boot talk とは別タイミングで steady talk を 1 本だけ起こす。
    let fixture = Fixture::quitting().with_steady_value_indices([0]);

    // quit_flags: [boot=false, steady=false, close=true]。到達順は boot→steady→close の想定。
    // close talk を末尾 talk（quit:true）にして終了系列を駆動する。
    let driven = drive_steady(fixture, 8, vec![false, false, true]);

    // (1) 宛先へ到達した StartTalk は boot talk・steady talk・close talk のちょうど 3 本。
    assert_eq!(
        driven.started.len(),
        3,
        "boot talk・steady talk・close talk の 3 本が到達するはず: {:?}",
        driven.started
    );

    // boot talk（先頭・id=1）。
    let boot_talk = &driven.started[0];
    assert_eq!(
        boot_talk.script, FIXED_BOOT_SCRIPT,
        "先頭 StartTalk は boot talk"
    );
    assert_eq!(boot_talk.talk_id, TalkId(1), "boot talk の talk_id は 1");

    // (2) steady talk が fixture の steady スクリプトで、ちょうど 1 本到達する（Req 2.1）。
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

    // (3) steady talk の talk_id は boot talk と別＝一意（再利用しない・単調増番・Req 2.1）。
    assert_ne!(
        steady_talk.talk_id, boot_talk.talk_id,
        "steady talk の talk_id は boot talk と別（一意）"
    );
    assert!(
        steady_talk.talk_id.0 > boot_talk.talk_id.0,
        "talk_id は単調増番（boot {:?} < steady {:?}）",
        boot_talk.talk_id,
        steady_talk.talk_id
    );

    // (4) 到達した全 talk_id が互いに一意（boot・steady・close が id を共有しない・Req 2.1）。
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

    // events 表から導出した GET（Ref3=1・talk_playable=true）の Reference 構成が記録列に現れる
    //   （時刻依存の Ref0 を持つ具体呼出を events から作って照合する）。
    let expected_get = RecordedCall {
        method: CallMethod::Get,
        id: "OnSecondChange".to_string(),
        references: match events::on_second_change(MonotonicMs(1_000), true) {
            areka_kanade::ShioriCall::Get { references, .. } => references,
            _ => panic!("on_second_change(_, true) は GET を返すはず"),
        },
    };
    assert!(
        driven.recorded.iter().any(|c| *c == expected_get),
        "events 表導出の GET（Ref3=1・now=1s）と一致する OnSecondChange GET が記録列に現れるはず: {:?}",
        driven.recorded
    );
}

/// 再生中 Tick → NOTIFY（Ref3=0）・応答無視で StartTalk なし（Req 3.3・DD-6）。
///
/// 保留機能付きハーネス（`spawn_harness_gated`）で steady talk の TalkDone を保留（park）し、
/// 「talk を 1 本 active に保ったまま Tick を挟む」窓を決定的に作る。手順:
///
/// 1. Boot → boot talk（受領 index 0・非保留）→ 即 TalkDone（kanade には未知 talk＝無害）。
/// 2. Tick 1（now=1s）: `Steady{None}` → OnSecondChange GET → fixture が Value（steady_value_indices
///    =[0]・最初の GET 出現）→ steady talk（受領 index 1・**保留**）→ `Steady{Some}`。
/// 3. steady talk の TalkDone は保留され inbox へ届かないため、次 Tick は必ず `Steady{Some}` から
///    処理される（interleaving なし）。
/// 4. Tick 2（now=2s）: `Steady{Some}` → OnSecondChange **NOTIFY**（Ref3=0）→ fixture Notified
///    （応答は構造的に破棄）→ `Steady{Some}` 維持。これが本テストの観測対象。
/// 5. `release_all` で保留 TalkDone（quit:false）を解放 → `Steady{None}` へ復帰。
/// 6. CloseRequest → OnClose GET → 別れの Value → close talk（quit:true）→ 終了系列完走。
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
    // idx=0: 最初の OnSecondChange GET 出現に Value（Tick 1 で steady talk を 1 本起こす）。
    let fixture = Fixture::quitting().with_steady_value_indices([0]);

    // 受領 index 1（steady talk）の TalkDone を保留し、active talk 窓を作る。
    // quit_flags: [boot=false, steady=false, close=true]。到達順は boot→steady→close。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, false, true]),
        vec![1],
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

    // (2) active talk 中の Tick が OnSecondChange NOTIFY（Ref3=0）を発行した——events 導出の
    //     NOTIFY（now=2s・talk_playable=false）と一致する記録が現れる（ハードコードしない）。
    let expected_notify = RecordedCall {
        method: CallMethod::Notify,
        id: "OnSecondChange".to_string(),
        references: match events::on_second_change(MonotonicMs(2_000), false) {
            areka_kanade::ShioriCall::Notify { references, .. } => references,
            _ => panic!("on_second_change(_, false) は NOTIFY を返すはず"),
        },
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
