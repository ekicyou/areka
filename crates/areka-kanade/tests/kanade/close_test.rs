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
//!
//! # 追加: boot 挨拶の統合檻（DD-IT-12・Req 1.5/2.4/3.1）
//! さらに boot 起動挨拶（default fixture・挨拶 talk＝StartTalk index 0）を保留ハーネスで active に
//! 保ち、DD-IT-12 の「挨拶を正規追跡する」意味論を統合層で 3 本、決定的に観測する（いずれも
//! additive・既存 4 シナリオは無改変）:
//! 5. **挨拶 active 中の Tick → NOTIFY（Ref3=0・Status: talking）**（Req 1.5/2.4）: 挨拶 talk を保留し、
//!    その最中の Tick が BOOT 挨拶由来の playing-semantics（NOTIFY・talking）を出すことを観測する
//!    （＝boot が `Steady{Some(挨拶)}` へ完了し、その slot 由来で pump を発行している証左）。
//! 6. **挨拶 TalkDone → GET pump 再開**（Req 4.4・相関成立の統合証左）: 保留解放で挨拶 TalkDone{Ended}
//!    を着弾させ、次 Tick で GET（Ref3=1・Status なし）が再開する＝挨拶が slot と照合され `Steady{None}`
//!    へ復帰した証左（照合しなければ `Steady{Some}` のまま NOTIFY を出し続け GET は現れない）。
//! 7. **挨拶中 CloseRequest → CloseTalkWait 経由 OnClose**（Req 3.1）: 挨拶再生中の close は即握手せず
//!    `pending_close` に記録され、挨拶 TalkDone 着弾で通常 talk と同じ握手（OnClose GET→別れ→close talk）が
//!    始まる（DD-IT-12「挨拶中 close は通常 talk と同じ CloseTalkWait」）。

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, TalkId, events,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_BOOT_SCRIPT, FIXED_FAREWELL_SCRIPT, FIXED_STEADY_SCRIPT,
    Fixture, Harness, QuitPolicy, RecordedCall, drive_ticks_until_disconnect, expected_call,
    expected_unload, join_bounded, spawn_harness, spawn_harness_gated,
};

/// 記録列中の OnClose GET（events 表導出・reason 指定）の初出インデックスを返す。
///
/// 通常握手の OnClose は talk 非アクティブ（`begin_close` が INACTIVE スナップショットで発行）ゆえ
/// Status 行なし。events 表から導出して照合する（References/Status をハードコードしない）。
fn onclose_get_index(recorded: &[RecordedCall], reason: CloseReason) -> Option<usize> {
    let onclose = expected_call(events::on_close(reason, &ExecutionSnapshot::INACTIVE));
    recorded.iter().position(|c| *c == onclose)
}

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
// シナリオ 1: 終了拒否 → 定常復帰 → pump 再開（Req 4.5・3.4）
// ============================================================================

/// OnClose が別れの Value を返すが close talk の TalkDone が quit:false のとき、kanade は
/// 終了せず定常運転へ復帰し、以降の Tick で pump（OnSecondChange GET）が再開する。
///
/// # 決定的な駆動（バリア駆動・join 後表明・talk 駆動終了）
/// 終了拒否後の pump 再開を、**再開後の pump が起こす talk を quit:true にして終了系列を駆動する**
/// ことで観測する。Tick 供給は反復回数上限でなく `drive_ticks_until_disconnect`（inbox 切断バリア＋
/// 壁時計 deadline）へ一本化し、kanade が復帰後 pump talk で終了して inbox を切断するまで 1 秒刻みの
/// Tick を供給する。これにより、cross-thread な TalkDone 到着順に依らず「切断＝終了＝全記録確定」の
/// 後に join 後の最終表明で記録を検証できる（poll も sleep も不要・full_run_test.rs / steady_test.rs と
/// 同じ枠組み）:
///
/// 1. Boot → 挨拶なし boot（`Steady{None}` 直行）→ pre-close Tick（`Steady{None}`・OnSecondChange
///    204＝talk なし）。
/// 2. CloseRequest{User}（即握手）→ OnClose GET → 別れの Value → close talk（受領 index 0・
///    quit:false）→ **終了拒否**・`Steady{None}` 復帰（kanade は close talk の TalkDone を待って
///    CloseTalkWait に留まり、到着で復帰する）。
/// 3. 復帰後の Tick で OnSecondChange GET（pump 再開・Req 3.4）→ fixture が Value（steady_value_indices
///    に「復帰後にだけ現れる GET 出現」を仕込む）→ steady talk（受領 index 1・quit:true）→ 終了系列完走。
///
/// close talk（index 0）で終了せず、その後の steady talk（index 1）で初めて終了する構成ゆえ、
/// 「終了拒否点で停止していない・pump が再開した」ことが終了到達それ自体で保証される。挨拶なし boot
/// （`without_boot_greeting`）で boot→`Steady{None}` へ直行させ、pre-close Tick を確実に GET index 0
/// にする（DD-IT-12 の挨拶 talk race を断つ）。
///
/// # 非空虚性
/// - close 握手を通らなければ OnClose GET が現れず (a) が落ちる。
/// - pump が再開しなければ復帰後 GET → steady talk が起きず、終了系列が駆動されない。kanade が終了せず
///   inbox が切断されないため Tick send が成功し続け、`drive_ticks_until_disconnect` が DEFAULT_TIMEOUT の
///   壁時計 deadline に達して panic する（＝終了拒否点で停止＝復帰しなかったことを決定論的に検出する）。
/// - 復帰後 pump が GET でなく NOTIFY だと Value が破棄され steady talk が起きず、同様に終了しない。
#[test]
fn close_refused_resumes_pump_then_terminates_via_resumed_talk() {
    // steady_value_indices=[1]: OnSecondChange GET の 2 度目の出現（0 始まり index 1）に Value。
    // 1 度目の GET 出現（index 0）は close 前の pre-close Tick で消費し 204、2 度目の GET 出現は
    // close 拒否→復帰後の pump で現れ Value を返す（＝pump 再開の直接証左）。挨拶なし boot ゆえ
    // pre-close Tick は確実に GET index 0 になる（DD-IT-12 の race を断つ）。
    let fixture = Fixture::quitting()
        .with_steady_value_indices([1])
        .without_boot_greeting();

    // close 握手の deadline を実質無限にする。本シナリオは close talk を「拒否」で復帰させる意図であり、
    // CloseTalkWait で TalkDone を待つ間に注入する poll Tick が既定 deadline（last_now+30_000ms）を
    // 超えると DeadlineExceeded で終了してしまい、拒否→復帰を観測できない（Tick の now は増え続けるため
    // 負荷下で TalkDone が遅れると容易に超過する）。deadline を巨大値にして期限判定を無効化し、復帰は
    // 純粋に close talk の TalkDone{quit:false} で起こす（deadline 超過はシナリオ 3 の担当）。
    let mut config = KanadeConfig::new("master", "1.0.0");
    config.close_talk_deadline_ms = u64::MAX;

    // quit_flags: close(index0)=false（終了拒否）・steady(index1)=true（終了駆動）。挨拶なし boot ゆえ
    // boot talk は無く、close talk が先頭 StartTalk＝index 0・resumed steady talk が index 1。
    let harness = spawn_harness(config, fixture, QuitPolicy::PerTalk(vec![false, true]));

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
    // 復帰前（CloseTalkWait）に届いた Tick は pump しない（last_now 更新のみ）ため、復帰後の pump（GET
    // 出現 value_indices=[1]）が Value を返し steady talk（quit:true）を起こすまで Tick を供給し続ける
    // 必要がある。これを反復回数上限でなく inbox 切断バリア＋壁時計 deadline で駆動する共有ヘルパーへ
    // 一本化する: kanade が復帰後 pump talk（quit:true）で終了して inbox を切断するまで 1 秒刻みの Tick を
    // 供給し、切断で戻る（＝復帰→終了の完了バリア）。既存の開始秒を保存（開始秒 2）。復帰しなければ
    // kanade は終了せず Tick send は成功し続けるため、DEFAULT_TIMEOUT の壁時計 deadline に達した時点で
    // ヘルパーが panic し、「拒否点で停止＝復帰しなかった」欠陥を決定論的な失敗へ変換する。
    drive_ticks_until_disconnect(&harness.sender, 2, "close_refused resume drive");

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
        .filter(|c| {
            **c == expected_call(events::on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE))
        })
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
        &ExecutionSnapshot { talk_active: true, choice_active: false },
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
    assert_eq!(expected_notify.method, CallMethod::Notify, "active 窓の pump は NOTIFY");
    assert_eq!(expected_notify.references.len(), 4, "OnSecondChange の References は 4 要素");
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
        &ExecutionSnapshot { talk_active: true, choice_active: false },
    ));
    assert!(
        wait_until(
            || harness.shiori.recorded(),
            |rec| rec.iter().any(|c| *c == expected_notify),
        ),
        "挨拶 talk active 中の Tick は OnSecondChange NOTIFY（Ref3=0・talking）を発行するはず（DD-IT-12）"
    );
    assert_eq!(expected_notify.references[3], "0", "active 窓の pump は Ref3=\"0\"（NOTIFY）");
    assert_eq!(expected_notify.status, Some("talking".to_string()), "active 窓は Status: talking");

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
    let resumed_get = resumed_get_after_notify(&recorded)
        .expect("active 窓の後に pump 再開 GET が現れるはず（挨拶 TalkDone が slot と照合された証左）");
    let expected_shape = expected_call(events::on_second_change(
        MonotonicMs(0),
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(resumed_get.references.len(), 4, "OnSecondChange の References は 4 要素");
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
        &ExecutionSnapshot { talk_active: true, choice_active: false },
    ));
    assert!(
        wait_until(
            || harness.shiori.recorded(),
            |rec| rec.iter().any(|c| *c == expected_notify),
        ),
        "挨拶保留中の Tick は OnSecondChange NOTIFY（Ref3=0・talking）を発行するはず（＝boot が Steady{{Some(挨拶)}} へ完了・DD-IT-12）"
    );
    assert_eq!(expected_notify.references[3], "0", "active 窓の pump は Ref3=\"0\"（NOTIFY）");
    assert_eq!(expected_notify.status, Some("talking".to_string()), "active 窓は Status: talking");

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
    join_bounded("kanade boot-greeting close join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade completes the deferred close handshake after the boot greeting and terminates");

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
    assert_eq!(*last, expected_unload(), "末尾は Unload（close talk quit:true の終了系列完走）");
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
    expected_call(events::on_close_notify(reason, &ExecutionSnapshot::INACTIVE))
}
