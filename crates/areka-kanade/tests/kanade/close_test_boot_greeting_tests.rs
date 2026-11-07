use super::test_support::onclose_get_index;
use super::{
    CallMethod, CloseReason, Driven, ExecutionSnapshot, FIXED_BOOT_SCRIPT, FIXED_FAREWELL_SCRIPT,
    Fixture, KanadeConfig, KanadeMsg, MonotonicMs, QuitPolicy, TalkId,
    drive_ticks_until_disconnect, events, expected_call, expected_unload,
    resumed_get_after_active_window, spawn_harness_gated,
};

// ============================================================================
// シナリオ 5・6: 挨拶 active 中の Tick → NOTIFY（Req 1.5・2.4）と、挨拶 TalkDone → GET pump 再開
// （Req 4.4・DD-IT-12 相関成立の統合証左）
// ============================================================================

/// boot 挨拶を保留（active 窓）→ 解放で挨拶 TalkDone{Ended} を着弾させ、次 Tick で
/// `OnSecondChange` GET pump（Ref3=1・Status なし）が再開することを統合層で観測する。
///
/// 保留中の Tick が BOOT 挨拶由来の `OnSecondChange` NOTIFY（Ref3=0・Status: talking）を出すこと
/// （＝boot が `Steady{Some(挨拶)}` へ完了し、その slot 由来で pump を発行している証左・シナリオ 5）も
/// ここで見る。挨拶が `Steady{None}` へ丸められていたら Tick は GET になり NOTIFY の確かめが落ちる。
///
/// これは DD-IT-12 の「挨拶 TalkDone が slot と照合される（`unknown_talk_done` ERROR が出ない）」
/// の**統合層の証左**である: 挨拶 talk（id=1）の TalkDone が `Steady{Some(挨拶)}` の slot と照合されて
/// `Steady{None}` へ復帰したからこそ、以降の Tick が GET（問い合わせ）へ戻る。もし照合が成立しなければ
/// kanade は `Steady{Some}` に留まり NOTIFY を出し続け、GET は決して現れない——ゆえに「active 窓（NOTIFY）
/// の後の GET 再開」の存在それ自体が相関成立を意味する（log_capture は別スレッドのアクターログを
/// 捕えられないため、この相関は in-source 檻 `boot_greeting_talkdone_correlates_without_unknown_error`
/// が直接、本 cage が挙動として、二重に固める）。
///
/// # 決定的駆動（バリア駆動・join 後表明・race-free）
/// `with_steady_value_indices([0])`: 復帰後最初の GET 出現（occurrence 0）が Value を返し steady talk
/// （quit:true）を起こして終了を駆動する。挨拶 talk（受領 index 0）を保留し、復帰後の steady talk は
/// 受領 index 1。Tick 供給は反復回数上限でなく `drive_ticks_until_disconnect`（inbox 切断バリア＋壁時計
/// deadline）へ一本化し、kanade が復帰後 pump talk で終了して inbox を切断するまで 1 秒刻みの Tick を
/// 供給する。復帰の表明は join 後の最終記録列（`resumed_get_after_active_window`＝active 窓後の GET）が担う。
///
/// # 非空虚性
/// - 挨拶が slot と照合されず `Steady{Some}` に留まれば GET は現れず終了も駆動されない。kanade が終了せず
///   inbox が切断されないため Tick send が成功し続け、`drive_ticks_until_disconnect` が DEFAULT_TIMEOUT の
///   壁時計 deadline に達して panic する（＝相関しなかったことを決定論的に検出する）。加えて join 後の
///   `resumed_get_after_active_window` も `None` で最終表明が落ちる。
#[test]
fn boot_greeting_talkdone_resumes_get_pump() {
    // 挨拶あり（default）＋復帰後 GET occurrence 0 に Value（steady talk を起こし終了駆動）。
    let fixture = Fixture::default().with_steady_value_indices([0]);

    // hold_indices=[0]: 挨拶 talk（受領 index 0・id=1）を保留し active 窓を作る。
    // quit_flags: index0（挨拶 talk）=false（Ended→復帰）・index1（復帰後 steady talk）=true（Quit→終了）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        fixture,
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // 挨拶 active 中の Tick（now=1h）→ OnSecondChange NOTIFY（Ref3=0・talking）。
    let notify_now = MonotonicMs(3_600_000); // 1h → Ref0="1"。
    harness
        .sender
        .send(KanadeMsg::Tick { now: notify_now })
        .expect("send held-greeting Tick");

    let expected_notify = expected_call(events::on_second_change(
        notify_now,
        &ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
            ..ExecutionSnapshot::INACTIVE
        },
    ));

    // 保留した挨拶 talk の TalkDone{Ended}（quit:false）を解放 → slot と照合され `Steady{None}` へ復帰。
    gate.release_all();

    // 復帰後の pump を駆動する。復帰前（Steady{Some}）の Tick は NOTIFY のみ・復帰後の GET が Value を
    // 返し steady talk（quit:true）を起こして終了を駆動する。Tick 供給は反復回数上限でなく
    // `drive_ticks_until_disconnect`（inbox 切断バリア＋壁時計 deadline）へ一本化し、kanade が復帰後 pump
    // talk で終了して inbox を切断するまで 1 秒刻みの Tick を供給する（切断で戻る＝復帰→終了の完了バリア）。
    //
    // 直前の NOTIFY 用 Tick（now=1h）の次の秒から刻む。
    drive_ticks_until_disconnect(
        &harness.sender,
        3_601,
        "boot_greeting_talkdone resume drive",
    );

    // 終了系列完走（復帰後 pump→steady talk quit:true→Unloading{Quit}→Unload→StopSelf）まで期限付き join。
    // join 成功それ自体が「挨拶が slot と照合され復帰し、pump が再開して talk を起こした」ことの保証である。
    let Driven {
        recorded, started, ..
    } = harness.finish("kanade boot-greeting resume join");

    // (a) active だった talk は boot 挨拶（先頭 StartTalk・boot script・先頭採番 id=1）。
    assert_eq!(
        started[0].script, FIXED_BOOT_SCRIPT,
        "先頭 StartTalk は boot 挨拶スクリプト（保留した active talk は boot 由来）"
    );
    assert_eq!(started[0].talk_id, TalkId(1), "挨拶 talk は先頭採番 id=1");
    assert!(
        recorded.contains(&expected_notify),
        "挨拶 talk active 中の Tick は OnSecondChange NOTIFY（Ref3=0・talking）を発行するはず（DD-IT-12）"
    );

    // (b) 完了後の GET 再開（DD-IT-12 相関成立の統合証左）: active 窓（NOTIFY）より後に OnSecondChange
    //     GET が現れ、Ref3=1・Status なし（`Steady{None}`＝挨拶 slot と照合されて復帰した証左）。復帰後の
    //     GET の now はループ依存ゆえ、now 非依存の Ref3／Status のみを events から導出して照合する。
    let resumed_get = resumed_get_after_active_window(&recorded).expect(
        "active 窓の後に pump 再開 GET が現れるはず（挨拶 TalkDone が slot と照合された証左）",
    );
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

    // (c) 終了系列完走: 末尾は Unload（復帰後 steady talk quit:true→Unloading{Quit}→Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（復帰後 pump talk quit:true の終了系列完走）で閉じるはず"
    );
}

// ============================================================================
// シナリオ 7: 挨拶中 CloseRequest → CloseTalkWait 経由 OnClose 握手（Req 3.1・DD-IT-12）
// ============================================================================

/// boot 挨拶再生中に受領した close 指示は即握手せず `pending_close` に記録され、挨拶 TalkDone
/// 着弾で通常 talk と同じ close 握手（OnClose GET→別れの Value→close talk）が始まることを観測する
/// （DD-IT-12「挨拶中 close は通常 talk と同じ CloseTalkWait」）。既存の
/// `farewell_talk_without_quit_tag_still_terminates`／full_run の close 握手
/// （`Steady{None}` からの即握手）に対し、本 cage は
/// **`Steady{Some(挨拶)}` からの繰延握手**（active 挨拶中の close）を統合層で埋める。
///
/// # 識別性（本 cage が狙う回帰を捕捉するための discriminative な 2 点・join の後の記録で確かめる）
/// terminal 側の握手アサーション（OnClose GET／別れ talk／末尾 Unload）だけでは「挨拶中 close の**繰延**」を
/// 「即握手」から識別できない（どちらも最終的に OnClose GET→別れ→Unload に至る）。ゆえに挨拶保留中に
/// 送った Tick を目印にして、繰延を一意に決める 2 点を join の後の記録で確かめる:
///   - **(A) 挨拶が active＝`Steady{Some(挨拶)}` である**: 挨拶保留中に Tick を 1 つ注入し、その Tick が
///     `OnSecondChange` **NOTIFY**（Ref3="0"・Status: talking）を発行する（`Steady{Some}` の pump 意味論）。
///     DD-IT-12 が revert され boot が `Steady{None}` へ丸まっていれば Tick は **GET**（Ref3="1"）になり、
///     この NOTIFY は現れず (A) が落ちる。
///   - **(B) OnClose 握手はその NOTIFY より後に始まる**: `Steady{Some}` の CloseRequest が `pending_close` を
///     迂回して即 `begin_close` する回帰（または `Steady{None}` 即握手）なら、OnClose GET が NOTIFY より前に
///     出て (B) が落ちる。
///
/// `release_all` → 挨拶 TalkDone{Ended} 着弾 → `pending_close` を消化して初めて
/// `begin_close`（OnClose GET→別れの Value→close talk）→ `CloseTalkWait` → close talk（受領 index 1・
/// quit:true）→ 終了系列完走。この順序で「OnClose は挨拶完了**後**に初めて出る」＝繰延が一意に固定される。
///
/// # メッセージ順序（決定性）
/// Boot・CloseRequest・Tick は同一 inbox（FIFO・test スレッド）ゆえ CloseRequest は挨拶保留中
/// （`Steady{Some}`）で処理され `pending_close` を記録し、続く Tick も `Steady{Some}` で NOTIFY を発行する
/// （`Steady{Some}` の Tick は `pending_close` を消化しない）。挨拶 TalkDone は保留され、`release_all`
/// まで inbox へ入らないため、Tick は必ず TalkDone より先に処理される（interleaving なし・待ち不要）。
///
/// # 非空虚性（下の実 assert に対応する）
/// - (A) 挨拶が `Steady{None}` へ丸められていたら（DD-IT-12 revert）Tick は GET になり NOTIFY が現れず
///   (A) の確かめが落ちる。
/// - (B) `Steady{Some}` の close が即 `begin_close` する回帰（`pending_close` 迂回）なら OnClose GET が
///   NOTIFY より前に出て (B) の確かめが落ちる。
/// - 挨拶が slot と照合されず `Steady{Some}` に留まれば TalkDone 消化が起きず握手が始まらないため、
///   終了が駆動されず join が期限超過して panic する。
#[test]
fn boot_greeting_close_during_greeting_uses_close_handshake() {
    // 挨拶あり＋quit close（Fixture::quitting()）: 挨拶 talk（受領 index 0）を保留し active に保つ。
    // quit_flags: index0（挨拶 talk）=false（Ended→握手消化）・index1（close talk）=true（Quit→終了）。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, true]),
        vec![0],
    );

    // Boot → `Steady{Some(挨拶 id=1)}`（挨拶 talk 保留）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // 挨拶再生中の close 指示。`Steady{Some}` ゆえ即握手せず `pending_close` に記録される（OnClose まだ）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        })
        .expect("send CloseRequest during greeting");

    // (A) 挨拶が active（`Steady{Some(挨拶)}`）であることを、Tick が NOTIFY（Ref3=0・talking）を出すことで
    //     確認する。`Steady{Some}` の Tick は `pending_close` を消化しない（NOTIFY 発行のみ・維持）ため、
    //     この Tick は握手を進めず「挨拶が今なお active・close は繰延中」という状態を可視化するだけである。
    let notify_now = MonotonicMs(3_600_000); // 1h → Ref0="1"。
    harness
        .sender
        .send(KanadeMsg::Tick { now: notify_now })
        .expect("send held-greeting Tick");

    let expected_notify = expected_call(events::on_second_change(
        notify_now,
        &ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
            ..ExecutionSnapshot::INACTIVE
        },
    ));

    // 挨拶 talk（Ended）を解放 → `pending_close` を消化して初めて握手開始（OnClose GET→別れの Value→close talk）。
    gate.release_all();

    // close talk（quit:true）で終了系列完走まで期限付き join。
    let Driven {
        recorded, started, ..
    } = harness.finish("kanade boot-greeting close join");

    // (a) 挨拶 talk が boot 由来で起動した（先頭 StartTalk・boot script・先頭採番 id=1）。
    assert_eq!(
        started[0].script, FIXED_BOOT_SCRIPT,
        "先頭 StartTalk は boot 挨拶スクリプト（close は挨拶中に受領された）"
    );
    assert_eq!(started[0].talk_id, TalkId(1), "挨拶 talk は先頭採番 id=1");

    // (b) 通常 close 握手を通った: OnClose GET（Ref0=user・INACTIVE＝Status なし）が現れる。
    //     挨拶中 close は即握手せず、挨拶 TalkDone 着弾で begin_close→OnClose GET が発行される
    //     （＝この GET の存在自体が握手経路＝CloseTalkWait を踏んだ証左・DD-IT-12）。
    let onclose_index = onclose_get_index(&recorded, CloseReason::User { scope: 0 })
        .expect("挨拶 TalkDone 後に OnClose GET（Ref0=user）が現れるはず（挨拶中 close の握手）");

    // (A) 挨拶保留中の Tick は NOTIFY（＝boot が Steady{Some(挨拶)} へ完了・DD-IT-12）。
    let notify_index = recorded
        .iter()
        .position(|c| *c == expected_notify)
        .expect("挨拶保留中の Tick は OnSecondChange NOTIFY（Ref3=0・talking）を発行するはず");
    // (B) 握手はその後（close は pending_close で繰延・即握手でない）。
    assert!(
        notify_index < onclose_index,
        "OnClose GET（{onclose_index}）は挨拶保留中の NOTIFY（{notify_index}）より後のはず"
    );

    // (c) 別れの close talk が現に起動した（OnClose の Value を受けて再生起動要求を配送した）。
    let farewell_started = started
        .iter()
        .filter(|s| s.script == FIXED_FAREWELL_SCRIPT)
        .count();
    assert_eq!(
        farewell_started, 1,
        "OnClose の別れ Value で close talk が 1 本起動するはず: {:?}",
        started
    );

    // (d) 終了系列完走: 末尾は Unload（close talk quit:true→Unloading{Quit}→Unload）。OnClose→…→Unload の順。
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(
        *last,
        expected_unload(),
        "末尾は Unload（close talk quit:true の終了系列完走）"
    );
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(unload_count, 1, "Unload は終了系列で 1 度だけ発行される");
}
