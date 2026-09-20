use super::test_support::onclose_get_index;
use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, ExecutionSnapshot, FIXED_FAREWELL_SCRIPT,
    FIXED_STEADY_SCRIPT, Fixture, Harness, KanadeConfig, KanadeMsg, MonotonicMs, QuitPolicy,
    RecordedCall, events, expected_call, expected_unload, join_bounded, spawn_harness,
    spawn_harness_gated,
};

// ============================================================================
// シナリオ 1: 別れの台詞は `\-` 無しで終わっても終了する（Req 3.6・3.7）
// ============================================================================

/// OnClose の別れの台詞が `\-` に辿り着かずに終わっても（TalkDone が quit:false）、kanade は
/// 定常運転へ戻らずそのまま終了系列を完走する。終了の握手から定常へ戻る経路は 1 本も無い
/// （完了 spec `areka-P0-kanade` の要件 4.5 を上書きする 2026-09-20 開発者裁定）。
///
/// # 決定的な駆動（Tick 0 本・deadline 無効）
/// 1. Boot → 挨拶なし boot（`Steady{None}` 直行・挨拶 talk の TalkDone 競合を断つ）。
/// 2. CloseRequest{User}（active talk なし＝即握手）→ OnClose GET → 別れの Value →
///    close talk（受領 index 0・quit:false＝`Ended`）。
/// 3. close talk の TalkDone{Ended} 着弾 → **終了へ進む** → Unload → StopSelf。
///
/// Tick を 1 本も送らないので `last_now` は None のまま＝握手入口の deadline も None のままで、
/// 再生完了待ちの上限は永久に設定されない（`close_talk_deadline_ms` も `u64::MAX` にしてある）。
/// ゆえに終了へ至る道は TalkDone の腕しか無く、期限超過（シナリオ 3）との取り違えが起き得ない。
///
/// # 非空虚性
/// - close 握手を通らなければ OnClose GET が現れず (a) が落ちる。
/// - 旧規則（quit:false は終了を拒んで `Steady{None}` へ復帰）なら kanade は終了せず、Tick も
///   来ないため CloseTalkWait のまま留まり、join が DEFAULT_TIMEOUT で panic する。
/// - 復帰してしまえば (c) の「OnClose 以降は Unload だけ」も同時に落ちる。
#[test]
fn farewell_talk_without_quit_tag_still_terminates() {
    // 別れの Value を返す fixture・挨拶なし boot（`Steady{None}` 直行）。
    let fixture = Fixture::quitting().without_boot_greeting();

    // 再生完了待ちの上限を実質無限にする。終了が期限超過ではなく TalkDone の腕で起きることを
    // 一意にするための封じ（期限超過はシナリオ 3 の担当）。
    let mut config = KanadeConfig::new("master", "1.0.0");
    config.close_talk_deadline_ms = u64::MAX;

    // quit_flags: close talk（index 0・挨拶なし boot ゆえ先頭 StartTalk）=false＝`Ended` で終わる
    // ＝台本が `\-` に辿り着かなかった場合そのものである。
    let harness = spawn_harness(config, fixture, QuitPolicy::PerTalk(vec![false]));

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // close 指示（active talk なし＝即握手・OnClose GET→別れの Value→close talk→quit:false）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("send CloseRequest");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // close talk の TalkDone{Ended} だけで終了系列が完走する（Tick は 1 本も送っていない）。
    join_bounded("kanade farewell-quit join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates when the farewell talk ends without a quit tag (Req 3.6)");

    drop(sender);
    // mock sakura は起動要求を記録してから TalkDone を返すので、join 成功時点で記録は確定している。
    let started = sakura.started();
    sakura.join_bounded("mock-sakura farewell-quit join", DEFAULT_TIMEOUT);

    let recorded = shiori.recorded();

    // (a) close 握手を通った証拠: OnClose GET（Ref0=user）が現れる。
    let onclose_index = onclose_get_index(&recorded, CloseReason::User { scope: 0 })
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

    // (c) 定常へ戻る経路は 0 本: OnClose GET より後に現れる記録は Unload 1 件だけで、pump
    //     （OnSecondChange）も追加イベントも一切無い。
    let after_close: Vec<&RecordedCall> = recorded
        .iter()
        .enumerate()
        .filter(|(i, _)| *i > onclose_index)
        .map(|(_, c)| c)
        .collect();
    assert_eq!(
        after_close.len(),
        1,
        "OnClose の後に現れるのは Unload だけ（終了の握手から定常へ戻る経路は 0 本）: {:?}",
        recorded
    );
    assert_eq!(
        *after_close[0],
        expected_unload(),
        "OnClose の後の 1 件は Unload"
    );

    // (d) 定常の talk も起動しない: sink に届くのは別れの close talk 1 本のみ。
    assert!(
        started.iter().all(|s| s.script != FIXED_STEADY_SCRIPT),
        "定常へ戻らないので steady talk は起動しないはず: {:?}",
        started
    );
    assert_eq!(
        started.len(),
        1,
        "sink に届くのは別れの close talk 1 本のみ: {:?}",
        started
    );

    // 終了系列: 末尾は Unload（正規終了経路）で 1 度だけ。
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(
        *last,
        expected_unload(),
        "末尾は Unload（別れの台詞の完了で駆動された終了系列の完走）で閉じるはず"
    );
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
            reason: CloseReason::User { scope: 0 },
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
    let onclose_index = onclose_get_index(&recorded, CloseReason::User { scope: 0 })
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
    assert_eq!(
        *last,
        expected_unload(),
        "記録列の末尾は Unload（無言終了の完走）"
    );
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
        .filter(|c| {
            **c == expected_call(events::on_close(
                CloseReason::User { scope: 0 },
                &ExecutionSnapshot::INACTIVE,
            ))
        })
        .count();
    assert_eq!(
        onclose_count, 1,
        "OnClose GET はちょうど 1 度（再発行なし）"
    );

    // (c) close は StartTalk を一切生まない: sink には boot talk のみが届く（close talk 非起動）。
    assert!(
        started.iter().all(|s| s.script != FIXED_FAREWELL_SCRIPT),
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
/// Boot（挨拶なし・`Steady{None}` 直行）→ Tick(now=1s)（`last_now=Some(1s)` を確定）→
/// CloseRequest{User}（`Steady{None}` の即握手・OnClose GET→別れの Value→close talk・**保留**）→
/// CloseTalkWait 進入時に deadline=`last_now + D`=1s+5s=6s。→ Tick(now=100s)（>> 6s）→
/// DeadlineExceeded → Unload → StopSelf。
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

    // 挨拶なし boot（without_boot_greeting）で boot→Steady{None} へ直行させ、CloseRequest を確実に
    // `Steady{None}` の即握手にする（DD-IT-12 の挨拶 talk race を断つ）。close talk（受領 index 0・挨拶
    // なし boot ゆえ先頭 StartTalk）の TalkDone を保留し、CloseTalkWait を維持したまま Tick を注入する。
    // quit_policy は使われない（保留 talk は解放されない・deadline で終了する）が、契約上与える。
    let (harness, gate) = spawn_harness_gated(
        config,
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![false]),
        vec![0],
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
            reason: CloseReason::User { scope: 0 },
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
    let onclose_index = onclose_get_index(&recorded, CloseReason::User { scope: 0 })
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
            reason: CloseReason::User { scope: 0 },
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
    assert_eq!(
        *last,
        expected_unload(),
        "ForceQuit 終了系列の末尾は Unload で閉じる"
    );
    assert_eq!(last.method, CallMethod::Unload);

    // (b) DD-10 の best-effort OnClose NOTIFY（Ref0=user）が Unload の直前に現れる。
    //     force_quit がインラインで組む退化 NOTIFY（events 表由来ではない）に一致させる。
    let force_notify = force_quit_onclose_notify(CloseReason::User { scope: 0 });
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
        onclose_get_index(&recorded, CloseReason::User { scope: 0 }).is_none(),
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
/// DD-IT-8: この NOTIFY は `events::on_close_notify` が単一列挙点として構成する（`force_quit` は
/// もはや inline 構築しない）。通常握手の [`events::on_close`] は **GET** を返すため force_quit には
/// 流用できず、NOTIFY 版を別に用いる。snapshot は Unloading{Forced} 遷移後の
/// [`ExecutionSnapshot::INACTIVE`]（Status 行なし・DD-IT-4）。events 表から導出することで、ハーネスに
/// References/Status 文字列をハードコードしない。
fn force_quit_onclose_notify(reason: CloseReason) -> RecordedCall {
    expected_call(events::on_close_notify(
        reason,
        &ExecutionSnapshot::INACTIVE,
    ))
}
