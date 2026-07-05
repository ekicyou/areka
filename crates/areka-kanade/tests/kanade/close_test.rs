//! close 握手・quit 分岐・期限・強制終了の統合検証（Req 3.4・4.2・4.4・4.5・4.6・4.7）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、close
//! 握手の 4 シナリオを個別の `#[test]` として決定的に観測する（実時間 sleep なし・時刻は注入
//! [`MonotonicMs`] Tick のみ・全 join は期限付き）:
//!
//! 1. **終了拒否 → 定常復帰 → pump 再開**（Req 4.5・3.4）: OnClose が別れの Value を返すが
//!    close talk の TalkDone が quit:false → kanade は終了せず `Steady{None}` へ復帰し、以降の
//!    Tick で OnSecondChange GET（pump）が再開する。再開後の pump が起こす steady talk を quit:true に
//!    することで終了系列を駆動し、「拒否点で停止していない・pump 再開」を終了到達それ自体で保証する。
//! 2. **無言終了**（Req 4.6）: OnClose が 204 → 追加イベントなしで終了系列へ直行し、記録列の末尾は
//!    `[.., OnClose GET, Unload]`（OnCloseAll 非発行・close talk 非起動）。
//! 3. **再生完了待ちの時間超過**（Req 4.7）: 保留ハーネスで close talk の TalkDone を差し止め、
//!    小さな `close_talk_deadline_ms` を超える Tick を注入 → DeadlineExceeded で終了系列を継続
//!    （TalkDone 不着でも join 成功・宙吊りなし）。
//! 4. **強制終了直行**（Req 4.4・DD-10）: ForceQuit → best-effort OnClose NOTIFY → Unload →
//!    StopSelf へ直行し join 成功。

use areka_kanade::{CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriCall, events};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT, Fixture, Harness,
    QuitPolicy, RecordedCall, expected_call, expected_unload, join_bounded, spawn_harness,
    spawn_harness_gated,
};

/// 記録列中の OnClose GET（events 表導出・reason 指定）の初出インデックスを返す。
fn onclose_get_index(recorded: &[RecordedCall], reason: CloseReason) -> Option<usize> {
    let onclose = expected_call(events::on_close(reason));
    recorded.iter().position(|c| *c == onclose)
}

// ============================================================================
// シナリオ 1: 終了拒否 → 定常復帰 → pump 再開（Req 4.5・3.4）
// ============================================================================

/// OnClose が別れの Value を返すが close talk の TalkDone が quit:false のとき、kanade は
/// 終了せず定常運転へ復帰し、以降の Tick で pump（OnSecondChange GET）が再開する。
///
/// # 決定的な駆動（full_run/steady と同一イディオム・talk 駆動終了）
/// 終了拒否後の pump 再開を、**再開後の pump が起こす talk を quit:true にして終了系列を駆動する**
/// ことで観測する。これにより、cross-thread な TalkDone 到着順に依らず「join 成功＝全記録確定」
/// の後に記録を検証できる（poll も sleep も不要・full_run_test.rs / steady_test.rs と同じ枠組み）:
///
/// 1. Boot → pre-close Tick（`Steady{None}`・OnSecondChange 204＝talk なし）。
/// 2. CloseRequest{User}（即握手）→ OnClose GET → 別れの Value → close talk（受領 index 1・
///    quit:false）→ **終了拒否**・`Steady{None}` 復帰（kanade は close talk の TalkDone を待って
///    CloseTalkWait に留まり、到着で復帰する）。
/// 3. 復帰後の Tick で OnSecondChange GET（pump 再開・Req 3.4）→ fixture が Value（steady_value_indices
///    に「復帰後にだけ現れる GET 出現」を仕込む）→ steady talk（受領 index 2・quit:true）→ 終了系列完走。
///
/// close talk（index 1）で終了せず、その後の steady talk（index 2）で初めて終了する構成ゆえ、
/// 「終了拒否点で停止していない・pump が再開した」ことが終了到達それ自体で保証される。
///
/// # 非空虚性
/// - close 握手を通らなければ OnClose GET が現れず (a) が落ちる。
/// - pump が再開しなければ復帰後 GET → steady talk が起きず、終了系列が駆動されないため join が
///   期限超過して panic する（＝終了拒否点で停止＝復帰しなかったことを検出する）。
/// - 復帰後 pump が GET でなく NOTIFY だと Value が破棄され steady talk が起きず、同様に終了しない。
#[test]
fn close_refused_resumes_pump_then_terminates_via_resumed_talk() {
    // steady_value_indices=[1]: OnSecondChange GET の 2 度目の出現（0 始まり index 1）に Value。
    // 1 度目の GET 出現（index 0）は close 前の pre-close Tick で消費し 204、2 度目の GET 出現は
    // close 拒否→復帰後の pump で現れ Value を返す（＝pump 再開の直接証左）。
    let fixture = Fixture::quitting().with_steady_value_indices([1]);

    // close 握手の deadline を実質無限にする。本シナリオは close talk を「拒否」で復帰させる意図であり、
    // CloseTalkWait で TalkDone を待つ間に注入する poll Tick が既定 deadline（last_now+30_000ms）を
    // 超えると DeadlineExceeded で終了してしまい、拒否→復帰を観測できない（Tick の now は増え続けるため
    // 負荷下で TalkDone が遅れると容易に超過する）。deadline を巨大値にして期限判定を無効化し、復帰は
    // 純粋に close talk の TalkDone{quit:false} で起こす（deadline 超過はシナリオ 3 の担当）。
    let mut config = KanadeConfig::new("master", "1.0.0");
    config.close_talk_deadline_ms = u64::MAX;

    // quit_flags: boot(index0)=false・close(index1)=false（終了拒否）・steady(index2)=true（終了駆動）。
    let harness = spawn_harness(config, fixture, QuitPolicy::PerTalk(vec![false, false, true]));

    // 起動 → pre-close Tick（GET 出現 index 0＝204・talk なし）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send pre-close Tick");

    // close 指示（active talk なし＝即握手・OnClose GET→別れの Value→close talk→quit:false→拒否）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    // 終了拒否→定常復帰後の pump を駆動する。close talk の TalkDone{quit:false} は非保留 sakura が
    // 別スレッドから即返すため、その到着は test スレッドの Tick と inbox 上で競合する（別 Sender）。
    // 復帰前（CloseTalkWait）に届いた Tick は pump しない（last_now 更新のみ）ため、単に多数の Tick を
    // 一括送出すると「全 Tick が復帰前に消費され、復帰後に Tick が 1 本も残らない」経路があり得る。
    // これを避け、かつ sleep を用いないため、復帰の証左が得られるまで「1 Tick 送出→ kanade へ処理を
    // 譲る（yield）→記録確認」を有界回数繰り返す。復帰後の GET 出現（value_indices=[1]）は Value を
    // 返し steady talk（quit:true）を起こすため、pump 再開が起きた時点で kanade は終了系列へ自走する。
    //
    // 復帰の証左は 2 通りで捉える（いずれも「拒否点で停止していない・pump が再開した」ことを意味する）:
    //   (i)  記録に OnClose の後の OnSecondChange GET が現れる、または
    //   (ii) Tick 送出が失敗する＝kanade が既に終了（＝復帰後 pump talk quit:true で自走終了した）。
    // (ii) は復帰後 pump→Value→steady talk→終了が観測前に完走した場合であり、これも復帰の成立を示す。
    let mut pump_resumed = false;
    'drive: for i in 2..=500u64 {
        if harness
            .sender
            .send(KanadeMsg::Tick {
                now: MonotonicMs(i * 1_000),
            })
            .is_err()
        {
            // inbox 切断＝kanade は復帰後 pump talk（quit:true）で終了済み（証左 (ii)）。
            pump_resumed = true;
            break 'drive;
        }
        // kanade が本 Tick（および先行 TalkDone）を処理し終えるのを譲って待つ（sleep なし）。
        for _ in 0..64 {
            std::thread::yield_now();
            let snap = harness.shiori.recorded();
            if let Some(idx) = onclose_get_index(&snap, CloseReason::User) {
                let pumped_after = snap.iter().enumerate().any(|(j, c)| {
                    j > idx && c.method == CallMethod::Get && c.id == "OnSecondChange"
                });
                if pumped_after {
                    // 証左 (i)。
                    pump_resumed = true;
                    break 'drive;
                }
            }
        }
    }
    assert!(
        pump_resumed,
        "終了拒否→定常復帰後に pump が有界回数内に再開するはず（Req 3.4・GET 出現または終了自走で観測）"
    );

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 終了系列完走（復帰後 pump→steady talk quit:true→Unload→StopSelf）まで期限付き join。
    // ここで join が成功すること自体が「終了拒否点で停止せず pump が再開した」ことの保証である。
    join_bounded("kanade close-refuse join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade resumes after close-reject and terminates via the resumed pump talk");

    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura close-refuse join", DEFAULT_TIMEOUT);

    let recorded = shiori.recorded();

    // (a) close 握手を通った証拠: OnClose GET（Ref0=user）が現れる。
    let onclose_index = onclose_get_index(&recorded, CloseReason::User)
        .expect("OnClose GET（Ref0=user）が記録列に現れるはず（握手を通った）");

    // (b) close talk が現に起動した（別れの Value を受け取り再生起動要求を配送した）。
    let farewell_started = started
        .iter()
        .filter(|s| s.script == FIXED_FAREWELL_SCRIPT)
        .count();
    assert_eq!(
        farewell_started, 1,
        "OnClose の別れ Value で close talk が 1 本起動するはず: {:?}",
        started
    );

    // (c) 終了拒否後に pump が再開した: OnClose GET より後に OnSecondChange GET が ≥1 本現れる。
    let post_close_pumps = recorded
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            *i > onclose_index && c.method == CallMethod::Get && c.id == "OnSecondChange"
        })
        .count();
    assert!(
        post_close_pumps >= 1,
        "終了拒否→定常復帰後に OnSecondChange GET（pump）が再開するはず（Req 3.4）: {:?}",
        recorded
    );

    // GET pump の Reference 構成が events 表の正典（Ref3="1"）と一致する 1 件を確認する。
    let resumed_get = recorded
        .iter()
        .enumerate()
        .find(|(i, c)| {
            *i > onclose_index && c.method == CallMethod::Get && c.id == "OnSecondChange"
        })
        .map(|(_, c)| c)
        .expect("再開後の OnSecondChange GET が存在するはず");
    assert_eq!(resumed_get.references.len(), 4, "OnSecondChange の References は 4 要素");
    assert_eq!(
        resumed_get.references[3], "1",
        "再開 pump の Ref3 は \"1\"（GET・talk 再生可能）"
    );

    // (d) 復帰後 pump が Value を起こし steady talk が終了を駆動した証拠: close talk とは別の
    //     steady script talk が 1 本現れる（＝拒否点で停止していなければ到達し得ない）。
    let steady_started = started
        .iter()
        .filter(|s| s.script == FIXED_STEADY_SCRIPT)
        .count();
    assert_eq!(
        steady_started, 1,
        "復帰後 pump の Value で steady talk が 1 本起動し終了を駆動するはず: {:?}",
        started
    );

    // 終了系列: 末尾は Unload（正規終了経路）で閉じる。OnClose→…→Unload の順。
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(
        *last,
        expected_unload(),
        "末尾は Unload（復帰後 pump talk quit:true の終了系列完走）で閉じるはず"
    );
    assert!(
        onclose_index < recorded.len() - 1,
        "OnClose（{onclose_index}）は Unload（{}）より前に現れるはず",
        recorded.len() - 1
    );
    // Unload は終了系列で 1 度だけ（末尾のみ）。
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(unload_count, 1, "Unload は終了系列で 1 度だけ発行される");
}

// ============================================================================
// シナリオ 2: 無言終了（Req 4.6）
// ============================================================================

/// OnClose が 204（応答なし）のとき、kanade は追加イベントを発行せず終了系列へ直行する
/// （OnCloseAll 非発行・close talk 非起動＝DD-11）。
///
/// # 駆動
/// Boot → CloseRequest{User}（active talk なし＝即握手）→ OnClose GET(204) →
/// `Unloading{CloseSilent}` → Unload → StopSelf。
///
/// # 非空虚性
/// - OnClose の後に GET/NOTIFY が挟まれば（例: OnCloseAll）記録の末尾が `[.., OnClose GET, Unload]`
///   にならず (b) が落ちる。
/// - 204 なのに close talk が起動すれば sink に boot talk 以外の StartTalk が届き (c) が落ちる。
#[test]
fn silent_close_on_204_terminates_without_extra_events() {
    // Fixture::default(): OnClose→204（無言終了）。boot talk は起動するが close talk は起きない。
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::default(),
        // boot talk(index0)=false。無言終了ゆえ close talk は起きない（PerTalk 範囲外は false）。
        QuitPolicy::PerTalk(vec![false]),
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
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

    // 204 無言終了で終了系列完走（Unload→StopSelf）まで到達する。
    join_bounded("kanade silent-close join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates directly on OnClose 204 (silent close・Req 4.6)");

    drop(sender);
    // boot talk（唯一の StartTalk）が sakura sink に届くのを待つ。kanade は 204 で即終了するため、
    // sink スレッドが boot StartTalk を recv・記録するのは非同期であり、drop 直後に started() を読むと
    // 未記録（空）を掴む race がある。sink スレッド終了（全 StartTalk Sender drop で recv 終了）を
    // join_bounded で見届けてから記録を確定させる。ただし join_bounded は sakura を消費するため、
    // 記録スナップショットは boot talk 到達を有界回数の yield で待って確定してから join する（sleep なし）。
    let mut started = sakura.started();
    for _ in 0..10_000 {
        if !started.is_empty() {
            break;
        }
        std::thread::yield_now();
        started = sakura.started();
    }
    sakura.join_bounded("mock-sakura silent-close join", DEFAULT_TIMEOUT);
    // join 後に最終スナップショットを取り直せないため（消費済み）、上の待機で確定した started を使う。
    // sink スレッドは boot StartTalk を 1 本受けて記録した後、他に送られる talk はない（204・Tick なし）。

    let recorded = shiori.recorded();

    // (a) OnClose GET（Ref0=user）が現れる。
    let onclose_index = onclose_get_index(&recorded, CloseReason::User)
        .expect("OnClose GET（Ref0=user）が記録列に現れるはず");

    // (b) OnClose GET の直後は Unload（末尾）で、間に追加の GET/NOTIFY は一切ない
    //     （OnCloseAll 非発行・追加 OnClose なし＝DD-11・Req 4.6）。
    assert_eq!(
        onclose_index,
        recorded.len() - 2,
        "OnClose GET は末尾から 2 番目（直後が Unload）であるべき: {:?}",
        recorded
    );
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(*last, expected_unload(), "記録列の末尾は Unload（無言終了の完走）");
    assert_eq!(last.method, CallMethod::Unload);

    // OnClose 以降に追加イベント（GET/NOTIFY）が挟まっていないことを明示的に確認する。
    let events_after_onclose: Vec<&RecordedCall> = recorded
        .iter()
        .enumerate()
        .filter(|(i, c)| {
            *i > onclose_index && matches!(c.method, CallMethod::Get | CallMethod::Notify)
        })
        .map(|(_, c)| c)
        .collect();
    assert!(
        events_after_onclose.is_empty(),
        "無言終了では OnClose の後に追加イベント（OnCloseAll 等）を発行しないはず: {:?}",
        events_after_onclose
    );

    // OnClose GET は記録列にちょうど 1 度（追加 OnClose なし）。
    let onclose_count = recorded
        .iter()
        .filter(|c| **c == expected_call(events::on_close(CloseReason::User)))
        .count();
    assert_eq!(onclose_count, 1, "OnClose GET はちょうど 1 度（再発行なし）");

    // (c) close は StartTalk を一切生まない: sink には boot talk のみが届く（close talk 非起動）。
    assert!(
        started
            .iter()
            .all(|s| s.script != FIXED_FAREWELL_SCRIPT),
        "204 無言終了では別れの close talk は起動しないはず: {:?}",
        started
    );
    // steady script も出ない（Tick を送っていない・boot talk のみ）。
    assert!(
        started.iter().all(|s| s.script != FIXED_STEADY_SCRIPT),
        "本シナリオで steady talk は起動しないはず: {:?}",
        started
    );
    assert_eq!(
        started.len(),
        1,
        "sink に届くのは boot talk 1 本のみ（close は talk を起こさない）: {:?}",
        started
    );
}

// ============================================================================
// シナリオ 3: 再生完了待ちの時間超過（Req 4.7）
// ============================================================================

/// close talk の TalkDone が来ないまま `close_talk_deadline_ms` を超える Tick が届くと、kanade は
/// DeadlineExceeded を検出して終了系列を継続する（TalkDone 不着でも join 成功＝宙吊りなし）。
///
/// # 駆動（保留ハーネスで close talk の TalkDone を差し止める）
/// Boot → Tick(now=1s)（`last_now=Some(1s)` を確定）→ CloseRequest{User}（即握手・OnClose GET→
/// 別れの Value→close talk・**保留**）→ CloseTalkWait 進入時に deadline=`last_now + D`=1s+5s=6s。
/// → Tick(now=100s)（>> 6s）→ DeadlineExceeded → Unload → StopSelf。
///
/// deadline 基準は握手入口の `last_now`（Tick(1s) で Some(1s)）ゆえ deadline=6s に確定する。注入
/// Tick(100s) は余裕を持って超過し、単一 Tick で確実に判定される（入口 last_now が Some の経路）。
///
/// # 非空虚性
/// - deadline 判定が働かなければ、TalkDone が永久に来ないため kanade は CloseTalkWait で宙吊りになり
///   join が期限超過して panic する（＝このテスト自体が落ちる）。
#[test]
fn close_talk_deadline_exceeded_terminates_without_talkdone() {
    // close_talk_deadline_ms を小さく差し替える（既定 30s では大きすぎる）。
    let mut config = KanadeConfig::new("master", "1.0.0");
    config.close_talk_deadline_ms = 5_000;

    // close talk（受領 index 1）の TalkDone を保留し、CloseTalkWait を維持したまま Tick を注入する。
    // quit_policy は使われない（保留 talk は解放されない・deadline で終了する）が、契約上与える。
    let (harness, gate) = spawn_harness_gated(
        config,
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, false]),
        vec![1],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // Tick(now=1s): last_now=Some(1s) を確定（握手入口 deadline 基準を Some にする）。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send arming Tick");

    // close 指示（active talk なし＝即握手・OnClose GET→別れの Value→close talk・保留）。
    // CloseTalkWait 進入時に deadline=1s+5s=6s が確定する。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    // Tick(now=100s): deadline(6s)を大きく超過 → DeadlineExceeded → Unload → StopSelf。
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(100_000),
        })
        .expect("send deadline-exceeding Tick");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // TalkDone が一度も来ないにもかかわらず、deadline 判定で終了系列が完走し join が成功する。
    join_bounded("kanade deadline join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates on close-talk deadline even without TalkDone (Req 4.7)");

    // kanade 停止（StartTalk 全 Sender drop）で保留ハーネスの recv ループが閉じ、releaser の
    // recv_closed 安全弁で releaser→本体スレッドが自然終了する。念のため release_all も呼ぶ
    // （解放は無害・保留は kanade 停止済みで送っても捨てられる）。
    gate.release_all();
    drop(sender);
    sakura.join_bounded("mock-sakura deadline join", DEFAULT_TIMEOUT);

    let recorded = shiori.recorded();

    // (a) close 握手を通った証拠: OnClose GET（Ref0=user）が現れる。
    let onclose_index = onclose_get_index(&recorded, CloseReason::User)
        .expect("OnClose GET（Ref0=user）が記録列に現れるはず（握手を通った）");

    // (b) TalkDone 不着でも終了系列は完走: 末尾は Unload（DeadlineExceeded 継続）で閉じる。
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(
        *last,
        expected_unload(),
        "TalkDone 不着でも deadline で終了系列が完走し末尾は Unload になるはず"
    );
    assert!(
        onclose_index < recorded.len() - 1,
        "OnClose（{onclose_index}）は Unload（{}）より前に現れるはず",
        recorded.len() - 1
    );
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(unload_count, 1, "Unload は終了系列で 1 度だけ発行される");
}

// ============================================================================
// シナリオ 4: 強制終了直行（Req 4.4・DD-10）
// ============================================================================

/// ForceQuit は close 握手を経ず終了系列へ直行する（DD-10: best-effort OnClose NOTIFY →
/// Unload → StopSelf）。
///
/// # 駆動
/// Boot →（定常運転へ落ち着く）→ ForceQuit{User} → OnClose NOTIFY → Unload → StopSelf。
///
/// # 非空虚性
/// - ForceQuit が終了へ直行しなければ join が期限超過して panic する。
/// - DD-10 の best-effort NOTIFY は force_quit がインラインで組む OnClose（Ref0=reason）であり、
///   events 表の OnClose **GET** とは Method が異なる。GET と NOTIFY を取り違えると (b) が落ちる。
#[test]
fn force_quit_terminates_directly_with_best_effort_onclose_notify() {
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::default(),
        QuitPolicy::PerTalk(vec![false]),
    );

    // 起動して定常運転へ落ち着かせる（boot 系列は Boot 処理内で同期完走する）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send settle Tick");

    // 強制終了指示 → 終了系列へ直行（DD-10）。
    harness
        .sender
        .send(KanadeMsg::ForceQuit {
            reason: CloseReason::User,
        })
        .expect("send ForceQuit");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 直行終了で join 成功（＝終了へ直行した証拠）。
    join_bounded("kanade force-quit join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates directly on ForceQuit (Req 4.4)");

    drop(sender);
    sakura.join_bounded("mock-sakura force-quit join", DEFAULT_TIMEOUT);

    let recorded = shiori.recorded();

    // (a) 末尾は Unload（終了系列完走）で閉じる。
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(*last, expected_unload(), "ForceQuit 終了系列の末尾は Unload で閉じる");
    assert_eq!(last.method, CallMethod::Unload);

    // (b) DD-10 の best-effort OnClose NOTIFY（Ref0=user）が Unload の直前に現れる。
    //     force_quit がインラインで組む退化 NOTIFY（events 表由来ではない）に一致させる。
    let force_notify = force_quit_onclose_notify(CloseReason::User);
    let notify_index = recorded
        .iter()
        .position(|c| *c == force_notify)
        .expect("ForceQuit の best-effort OnClose NOTIFY（Ref0=user）が現れるはず（DD-10）");
    let unload_index = recorded.len() - 1;
    assert!(
        notify_index < unload_index,
        "OnClose NOTIFY（{notify_index}）は Unload（{unload_index}）より前に現れるはず（DD-10 順序）"
    );

    // (c) close 握手（OnClose GET）は通っていない: ForceQuit は握手を経ず直行する。
    assert!(
        onclose_get_index(&recorded, CloseReason::User).is_none(),
        "ForceQuit は close 握手（OnClose GET）を経ず終了へ直行するはず: {:?}",
        recorded
    );

    // Unload は 1 度だけ。
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(unload_count, 1, "Unload は終了系列で 1 度だけ発行される");
}

// ============================================================================
// 期待値導出ヘルパ（ForceQuit の best-effort OnClose NOTIFY）
// ============================================================================

/// ForceQuit（DD-10）が Action 先頭に積む best-effort OnClose **NOTIFY** の期待記録。
///
/// この NOTIFY は `src/schedule/mod.rs` の `force_quit` がインラインで組む退化 NOTIFY であり
/// （events 表の OnClose は GET・タスク 2.1 の Implementation Note）、id=`"OnClose"`・
/// Ref0=`reason.as_ref_str()` で構成される。`ShioriCall::Notify` を [`RecordedCall`] へ写して
/// 期待値とすることで、ハーネスに References 文字列をハードコードしない。
fn force_quit_onclose_notify(reason: CloseReason) -> RecordedCall {
    expected_call(ShioriCall::Notify {
        id: "OnClose",
        references: vec![reason.as_ref_str().to_string()],
    })
}
