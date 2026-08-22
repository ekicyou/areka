use super::test_support::onclose_get_index;
use super::{
    CallMethod, CloseReason, DEFAULT_TIMEOUT, ExecutionSnapshot, FIXED_BOOT_SCRIPT,
    FIXED_FAREWELL_SCRIPT, Fixture, Harness, KanadeConfig, KanadeMsg, MonotonicMs, QuitPolicy,
    RecordedCall, TalkId, drive_ticks_until_disconnect, events, expected_call, expected_unload,
    join_bounded, spawn_harness_gated,
};

/// `fetch` が返す記録列が `pred` を満たすまで壁時計 deadline（[`DEFAULT_TIMEOUT`]）まで yield して
/// 待つ（sleep なし・満たせば true）。
///
/// mock は即応ゆえ観測対象は短時間で現れる。本ループは cross-thread な記録の可視化を待つだけの
/// ハング検出付きバリアであり（sleep は用いず、打ち切りは反復回数ではなく壁時計 deadline のみ）、
/// 最終的な宙吊り検出は各テスト末尾の [`join_bounded`]（[`DEFAULT_TIMEOUT`]）が別途担保する。
///
/// `pred` 成立で `true`、[`DEFAULT_TIMEOUT`] を測る [`std::time::Instant`] deadline を超過すれば `false`。
/// 呼出側は既存どおり `assert!(wait_until(...))` で待機成否を表明し、deadline 超過（欠陥時）は
/// `false` → assert 失敗として顕在化する。
fn wait_until<F, P>(fetch: F, pred: P) -> bool
where
    F: Fn() -> Vec<RecordedCall>,
    P: Fn(&[RecordedCall]) -> bool,
{
    let deadline = std::time::Instant::now() + DEFAULT_TIMEOUT;
    loop {
        if pred(&fetch()) {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::yield_now();
    }
}

/// `OnSecondChange` NOTIFY（active talk 窓）**より後**に現れる最初の `OnSecondChange` GET を返す
/// （＝pump 再開＝`Steady{None}` 復帰の一意な指標）。NOTIFY が無ければ `None`。
///
/// `Steady{Some}` 中は NOTIFY のみで GET は出ず、TalkDone 着弾で `Steady{None}` へ復帰した後に初めて
/// GET が現れるため、「NOTIFY より後の最初の GET」が完了後の pump 再開を一意に指す（steady_test の
/// `resumed_get_after_active_window` と同旨）。
fn resumed_get_after_notify(recorded: &[RecordedCall]) -> Option<&RecordedCall> {
    let notify_idx = recorded
        .iter()
        .position(|c| c.method == CallMethod::Notify && c.id == "OnSecondChange")?;
    recorded
        .iter()
        .enumerate()
        .find(|(i, c)| *i > notify_idx && c.method == CallMethod::Get && c.id == "OnSecondChange")
        .map(|(_, c)| c)
}

// ============================================================================
// シナリオ 5: 挨拶 active 中の Tick → NOTIFY（Ref3=0・Status: talking）（Req 1.5・2.4・DD-IT-12）
// ============================================================================

/// boot 起動挨拶（default fixture）を保留ハーネスで active に保ち、その最中の Tick が
/// `OnSecondChange` NOTIFY（Ref3=0・Status: talking）を発行することを統合層で観測する。
///
/// DD-IT-12 で boot は起動挨拶 talk を**正規追跡**するようになり、挨拶 Value 経路は
/// `Steady{talk: Some(挨拶)}` へ完了する。その `Steady{Some}` 由来で発行される pump は「再生中」
/// 意味論＝NOTIFY（Ref3=0・応答無視）・`Status: talking` を運ぶ。本 cage は default（挨拶あり）
/// fixture の挨拶 talk（受領 index 0）を保留（park）して active 窓を作り、この playing-semantics
/// Tick が **BOOT 挨拶に由来する**ことを決定的に固定する（steady talk 由来を観測する
/// steady_test の `active_talk_tick_emits_notify_ref3_zero` の boot 版）。
///
/// # 決定性（sleep なし・race-free）
/// 挨拶 talk の TalkDone を保留する限り `Steady{Some(挨拶)}` は次 Tick まで確実に維持される
/// （TalkDone は二つの Tick の間に割り込み得る唯一のメッセージ）。ゆえに Boot の後に処理される
/// Tick は必ず `Steady{Some}` から NOTIFY を発行する。NOTIFY が記録に現れる（＝Tick が active 窓で
/// 処理された）ことを**解放前に**有界 yield で確認してから、挨拶 talk を quit:true で解放して
/// 終了系列（Unloading{Quit}→Unload→StopSelf）を駆動する。
///
/// # 非空虚性
/// - 挨拶が `Steady{None}` へ丸められていたら（DD-IT-12 未実装）Tick は GET（Ref3=1）になり
///   events 導出の NOTIFY と一致せず `wait_until` が有界回内に成立せず assert が落ちる。
#[test]
fn boot_greeting_active_tick_emits_notify_talking() {
    // 挨拶あり（default）fixture。挨拶 talk（受領 index 0）を保留し active 窓を作る。
    // quit_flags: 挨拶 talk（index 0）=quit:true → 解放で Unloading{Quit}→終了系列を駆動。
    let (harness, gate) = spawn_harness_gated(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::default(),
        QuitPolicy::PerTalk(vec![true]),
        vec![0],
    );

    // 起動指示 → boot 系列は Boot 処理内で同期完走し `Steady{Some(挨拶 id=1)}` へ（挨拶 talk 保留）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // 挨拶 active 中の Tick（now=1h）→ OnSecondChange NOTIFY（Ref3=0・Status: talking）。
    let notify_now = MonotonicMs(3_600_000); // 1h → Ref0="1"。
    harness
        .sender
        .send(KanadeMsg::Tick { now: notify_now })
        .expect("send held-greeting Tick");

    // events 表から NOTIFY 期待値を導出（talk_active=true・DD-IT-3／DD-IT-12・ハードコードしない）。
    let expected_notify = expected_call(events::on_second_change(
        notify_now,
        &ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
        },
    ));

    // NOTIFY が記録に現れる（＝Tick が `Steady{Some(挨拶)}` で処理された）まで有界 yield で待つ。
    // 解放前に確認する: 解放後だと挨拶 TalkDone が先着して `Steady{None}` になり Tick が GET になり得る。
    assert!(
        wait_until(
            || harness.shiori.recorded(),
            |rec| rec.iter().any(|c| *c == expected_notify),
        ),
        "挨拶 talk active 中の Tick は OnSecondChange NOTIFY（Ref3=0・talking）を発行するはず（DD-IT-12）"
    );

    // NOTIFY の構成を明示確認（BOOT 挨拶由来の playing-semantics）。
    assert_eq!(
        expected_notify.method,
        CallMethod::Notify,
        "active 窓の pump は NOTIFY"
    );
    assert_eq!(
        expected_notify.references.len(),
        4,
        "OnSecondChange の References は 4 要素"
    );
    assert_eq!(
        expected_notify.references[3], "0",
        "NOTIFY pump の Ref3 は \"0\"（active talk 中・応答無視）"
    );
    assert_eq!(
        expected_notify.status,
        Some("talking".to_string()),
        "挨拶 active の Tick は Status: talking を運ぶ（DD-IT-12・DD-IT-3）"
    );

    // 挨拶 talk（quit:true）を解放 → Unloading{Quit}→Unload→StopSelf で終了系列を駆動する。
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 終了系列完走（StopSelf 到達）まで期限付き join。成功時点で全記録・全配送が確定する。
    join_bounded("kanade boot-greeting notify join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after releasing the boot greeting as quit:true");

    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura boot-greeting notify join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (a) active な talk が現に boot 由来だった（先頭 StartTalk＝boot 挨拶スクリプト・先頭採番 id=1）。
    assert!(!started.is_empty(), "挨拶 talk が sink に配送されるはず");
    assert_eq!(
        started[0].script, FIXED_BOOT_SCRIPT,
        "先頭 StartTalk は boot 挨拶スクリプト（active 窓の talk は boot 由来）"
    );
    assert_eq!(started[0].talk_id, TalkId(1), "挨拶 talk は先頭採番 id=1");

    // (b) 確定記録に挨拶 active の OnSecondChange NOTIFY（Ref3=0・talking）が含まれる（Req 1.5/2.4）。
    assert!(
        recorded.iter().any(|c| *c == expected_notify),
        "確定記録に挨拶 active の OnSecondChange NOTIFY（Ref3=0・talking）が含まれるはず: {:?}",
        recorded
    );

    // (c) 終了系列完走: 末尾は Unload（挨拶 talk quit:true→Unloading{Quit}→Unload）。
    assert_eq!(
        recorded.last().expect("記録列は空でない"),
        &expected_unload(),
        "末尾は Unload（挨拶 talk quit:true の終了系列完走）で閉じるはず"
    );
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(unload_count, 1, "Unload は終了系列で 1 度だけ発行される");
}

// ============================================================================
// シナリオ 6: 挨拶 TalkDone → GET pump 再開（Req 4.4・DD-IT-12 相関成立の統合証左）
// ============================================================================

/// boot 挨拶を保留（active 窓）→ 解放で挨拶 TalkDone{Ended} を着弾させ、次 Tick で
/// `OnSecondChange` GET pump（Ref3=1・Status なし）が再開することを統合層で観測する。
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
/// 供給する。復帰の表明は join 後の最終記録列（`resumed_get_after_notify`＝active 窓後の GET）が担う。
///
/// # 非空虚性
/// - 挨拶が slot と照合されず `Steady{Some}` に留まれば GET は現れず終了も駆動されない。kanade が終了せず
///   inbox が切断されないため Tick send が成功し続け、`drive_ticks_until_disconnect` が DEFAULT_TIMEOUT の
///   壁時計 deadline に達して panic する（＝相関しなかったことを決定論的に検出する）。加えて join 後の
///   `resumed_get_after_notify` も `None` で最終表明が落ちる。
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
        },
    ));
    assert!(
        wait_until(
            || harness.shiori.recorded(),
            |rec| rec.iter().any(|c| *c == expected_notify),
        ),
        "挨拶 talk active 中の Tick は OnSecondChange NOTIFY（Ref3=0・talking）を発行するはず（DD-IT-12）"
    );
    assert_eq!(
        expected_notify.references[3], "0",
        "active 窓の pump は Ref3=\"0\"（NOTIFY）"
    );
    assert_eq!(
        expected_notify.status,
        Some("talking".to_string()),
        "active 窓は Status: talking"
    );

    // 保留した挨拶 talk の TalkDone{Ended}（quit:false）を解放 → slot と照合され `Steady{None}` へ復帰。
    gate.release_all();

    // 復帰後の pump を駆動する。復帰前（Steady{Some}）の Tick は NOTIFY のみ・復帰後の GET が Value を
    // 返し steady talk（quit:true）を起こして終了を駆動する。Tick 供給は反復回数上限でなく
    // `drive_ticks_until_disconnect`（inbox 切断バリア＋壁時計 deadline）へ一本化し、kanade が復帰後 pump
    // talk で終了して inbox を切断するまで 1 秒刻みの Tick を供給する（切断で戻る＝復帰→終了の完了バリア）。
    //
    // 既存の開始秒を保存（開始秒 2・時刻後退は既存構造の踏襲）: 直前の NOTIFY 用 Tick は now=3,600,000
    // （1h）だが、この drive Tick は now=2,000 から始まり時刻が後退する。これは意図的な既存挙動であり
    // （挙動不変が目的）、開始秒を変えずそのまま保存する。
    drive_ticks_until_disconnect(&harness.sender, 2, "boot_greeting_talkdone resume drive");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 終了系列完走（復帰後 pump→steady talk quit:true→Unloading{Quit}→Unload→StopSelf）まで期限付き join。
    // join 成功それ自体が「挨拶が slot と照合され復帰し、pump が再開して talk を起こした」ことの保証である。
    join_bounded("kanade boot-greeting resume join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the resumed pump talk after the boot greeting completes");

    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura boot-greeting resume join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (a) active だった talk は boot 挨拶（先頭 StartTalk・boot script・先頭採番 id=1）。
    assert!(!started.is_empty(), "少なくとも挨拶 talk が配送されるはず");
    assert_eq!(
        started[0].script, FIXED_BOOT_SCRIPT,
        "先頭 StartTalk は boot 挨拶スクリプト（保留した active talk は boot 由来）"
    );
    assert_eq!(started[0].talk_id, TalkId(1), "挨拶 talk は先頭採番 id=1");

    // (b) 完了後の GET 再開（DD-IT-12 相関成立の統合証左）: active 窓（NOTIFY）より後に OnSecondChange
    //     GET が現れ、Ref3=1・Status なし（`Steady{None}`＝挨拶 slot と照合されて復帰した証左）。復帰後の
    //     GET の now はループ依存ゆえ、now 非依存の Ref3／Status のみを events から導出して照合する。
    let resumed_get = resumed_get_after_notify(&recorded).expect(
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
/// `close_refused_...`／full_run の close 握手（`Steady{None}` からの即握手）に対し、本 cage は
/// **`Steady{Some(挨拶)}` からの繰延握手**（active 挨拶中の close）を統合層で埋める。
///
/// # 識別性（本 cage が狙う回帰を捕捉するための discriminative な 2 点・release 前に確定する）
/// terminal 側の握手アサーション（OnClose GET／別れ talk／末尾 Unload）だけでは「挨拶中 close の**繰延**」を
/// 「即握手」から識別できない（どちらも最終的に OnClose GET→別れ→Unload に至る）。ゆえに `release_all` の
/// **前**に、繰延を一意に決める 2 点を保留状態のまま確定・確認する（Cage 5/6 と同型の `wait_until` 檻）:
///   - **(A) 挨拶が active＝`Steady{Some(挨拶)}` である**: 挨拶保留中に Tick を 1 つ注入し、その Tick が
///     `OnSecondChange` **NOTIFY**（Ref3="0"・Status: talking）を発行する（`Steady{Some}` の pump 意味論）
///     ことを `wait_until` で確認する。DD-IT-12 が revert され boot が `Steady{None}` へ丸まっていれば Tick は
///     **GET**（Ref3="1"）になり、この NOTIFY は現れず (A) が有界回内に成立せず落ちる。
///   - **(B) この時点で OnClose 握手が始まっていない**: (A) 成立時点（release 前・挨拶なお active）で
///     `OnClose` GET が記録に**無い**ことを assert する。`Steady{Some}` の CloseRequest が `pending_close` を
///     迂回して即 `begin_close` する回帰（または `Steady{None}` 即握手）なら、この時点で既に OnClose GET が
///     出ていて (B) が落ちる。
/// (A)(B) を確定してから `release_all` → 挨拶 TalkDone{Ended} 着弾 → `pending_close` を消化して初めて
/// `begin_close`（OnClose GET→別れの Value→close talk）→ `CloseTalkWait` → close talk（受領 index 1・
/// quit:true）→ 終了系列完走。この順序で「OnClose は挨拶完了**後**に初めて出る」＝繰延が一意に固定される。
///
/// # メッセージ順序（決定性）
/// Boot・CloseRequest・Tick は同一 inbox（FIFO・test スレッド）ゆえ CloseRequest は挨拶保留中
/// （`Steady{Some}`）で処理され `pending_close` を記録し、続く Tick も `Steady{Some}` で NOTIFY を発行する
/// （`Steady{Some}` の Tick は `pending_close` を消化しない）。挨拶 TalkDone は保留され、`release_all`
/// （(A)(B) 確認後に呼ぶ）まで inbox へ入らないため、release 前は OnClose が構造的に出得ない＝(B) は
/// race-free。挨拶が active な限り (A) の NOTIFY も決定的に現れる（interleaving なし・sleep 不要）。
///
/// # 非空虚性（下の実 assert に対応する）
/// - (A) 挨拶が `Steady{None}` へ丸められていたら（DD-IT-12 revert）Tick は GET になり NOTIFY が現れず
///   `wait_until` が有界回内に成立せず落ちる。
/// - (B) `Steady{Some}` の close が即 `begin_close` する回帰（`pending_close` 迂回）なら release 前に
///   OnClose GET が既に出ており `is_none()` assert が落ちる。
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
            reason: CloseReason::User,
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
        },
    ));
    assert!(
        wait_until(
            || harness.shiori.recorded(),
            |rec| rec.iter().any(|c| *c == expected_notify),
        ),
        "挨拶保留中の Tick は OnSecondChange NOTIFY（Ref3=0・talking）を発行するはず（＝boot が Steady{{Some(挨拶)}} へ完了・DD-IT-12）"
    );
    assert_eq!(
        expected_notify.references[3], "0",
        "active 窓の pump は Ref3=\"0\"（NOTIFY）"
    );
    assert_eq!(
        expected_notify.status,
        Some("talking".to_string()),
        "active 窓は Status: talking"
    );

    // (B) この時点（release 前・挨拶なお active）で OnClose 握手が始まっていない: 挨拶保留中は TalkDone が
    //     inbox へ入らず握手の消化点が来ないため OnClose GET は構造的に出得ない（race-free）。即 begin_close
    //     回帰（`pending_close` 迂回）ならここで既に OnClose GET が出ており is_none() が落ちる。
    assert!(
        onclose_get_index(&harness.shiori.recorded(), CloseReason::User).is_none(),
        "挨拶再生中（release 前）は OnClose GET がまだ現れないはず（close は pending_close で繰延・即握手でない）"
    );

    // 挨拶 talk（Ended）を解放 → `pending_close` を消化して初めて握手開始（OnClose GET→別れの Value→close talk）。
    gate.release_all();

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // close talk（quit:true）で終了系列完走まで期限付き join。
    join_bounded("kanade boot-greeting close join", DEFAULT_TIMEOUT, kanade).expect(
        "kanade completes the deferred close handshake after the boot greeting and terminates",
    );

    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura boot-greeting close join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (a) 挨拶 talk が boot 由来で起動した（先頭 StartTalk・boot script・先頭採番 id=1）。
    assert!(!started.is_empty(), "挨拶 talk が sink に配送されるはず");
    assert_eq!(
        started[0].script, FIXED_BOOT_SCRIPT,
        "先頭 StartTalk は boot 挨拶スクリプト（close は挨拶中に受領された）"
    );
    assert_eq!(started[0].talk_id, TalkId(1), "挨拶 talk は先頭採番 id=1");

    // (b) 通常 close 握手を通った: OnClose GET（Ref0=user・INACTIVE＝Status なし）が現れる。
    //     挨拶中 close は即握手せず、挨拶 TalkDone 着弾で begin_close→OnClose GET が発行される
    //     （＝この GET の存在自体が握手経路＝CloseTalkWait を踏んだ証左・DD-IT-12）。
    let onclose_index = onclose_get_index(&recorded, CloseReason::User)
        .expect("挨拶 TalkDone 後に OnClose GET（Ref0=user）が現れるはず（挨拶中 close の握手）");

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
