//! 失敗経路の統合検証（Req 2.5・4.8・4.9・5.4・6.1・6.2）。
//!
//! mock 結線（`super::common` のハーネス）で、kanade の失敗・停止・回復各経路を統合層で
//! 決定的に観測する（実時間 sleep なし・全 join は期限付き・宙吊りしない）。検証する 4 系:
//!
//! 1. **区別語彙ごとの呼出失敗 → 観測可能な終了**（Req 6.1）: [`ShioriFailure`] のうち終了へ
//!    倒れる 4 語彙（Handshake／Timeout／Ipc／Internal）それぞれについて、boot 最初の呼出
//!    （`OnInitialize`）をその語彙で失敗させ、kanade が終了系列（Unloading{Fault}→best-effort
//!    Unload→Stopped）へ倒れて停止すること（kanade の期限付き join 成功＋Unload 記録）を観測する。
//!    エラー応答（`Shiori`）は終了へ倒れないので 5 で別に見る。停止通知は Fault の種類と理由
//!    （記録 `shiori_failed` の `error` と同じ文言）で届く（本仕様 要件 2.1・2.2・7.3）。
//! 2. **死活報告 → 観測可能な停止**（Req 5.4）: boot 定常化後に `KanadeMsg::ShioriDown` を 2 種類
//!    （接続できなかった・helper の終了）それぞれで注入し、kanade が Unloading{Fault}→Unload→Stopped
//!    で停止すること（join 成功＋Unload 記録）と、停止通知が Fault の種類と理由（記録 `shiori_down`
//!    の `reason` と同じ文言）で届くことを観測する（本仕様 要件 2.1・2.2）。
//! 3. **未知 talk_id の再生完了通知 → 運行継続**（Req 2.5・6.2）: 採番されていない talk_id の
//!    `TalkDone` を注入しても kanade は終了せず、その後 driven な close で初めて正常終了することを
//!    観測する（未知 TalkDone は現 Phase 維持で無害）。
//! 4. **全ての指示送信元切断 → 宙吊りにならず正常終了・停止観測**（Req 4.8・4.9・6.2）: kanade
//!    inbox の全 Sender を drop すると `run_inbox` が切断で正常終了し、kanade の期限付き join が
//!    成功する（受信待ちのままハングしない・4.9 の構造保証）。
//! 5. **エラー応答 → 会話を続ける**（本仕様 要件 6.1・7.3）: SHIORI のエラー応答（400・500 など）は
//!    起動時（`OnInitialize`＝NOTIFY）でも会話中（`OnSecondChange`＝GET）でも止めず、返事なしと
//!    同じ扱いで次の呼出へ進む。記録 `shiori_error_response` は 1 往復に 1 件。
//! 6. **送出失敗・応答の切断 → 通信が切れた**（本仕様 要件 2.2）: SHIORI 側の受け口が先に閉じて
//!    いると呼出の送出が失敗し、SHIORI 側が返事をせずに返信口を捨てると応答が切れる。どちらも終了
//!    系列へ入り、停止通知は「通信が切れた」の種類で届く。応答の期限切れの腕は、無期限の受信
//!    （`recv`）が期限切れを返さないので届かず、ここでは固定しない。

use std::sync::mpsc;

use areka_kanade::{
    CloseReason, KanadeConfig, KanadeMsg, KanadeStopCause, KanadeStopped, MonotonicMs,
    ShioriDownKind, ShioriFailure, ShioriFault, ShioriFaultKind, TalkDone, TalkEndReason, TalkId,
};

use log_capture_kit::install_global_capture_all;

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_FAREWELL_SCRIPT, FailKind, FailOn, Fixture, Harness,
    QuitPolicy, RecordedCall, SinklessHarness, join_bounded, spawn_harness, spawn_harness_failing,
    spawn_harness_failing_with_stop_sink, spawn_harness_no_sink, spawn_harness_with_stop_sink,
};

/// 記録列に Unload（正規終了経路の best-effort unload）がちょうど 1 度現れることを確認する。
///
/// 設計「Error Categories」: ShioriFailure／ShioriDown いずれも Unloading{Fault} を経て
/// best-effort Unload を発行してから Stopped へ進む。ゆえに Fault 経路の終了では Unload が
/// 記録列に必ず 1 度現れる（宙吊りせず正規終了経路を通った直接証拠）。
fn assert_unload_recorded_once(recorded: &[super::common::RecordedCall]) {
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(
        unload_count, 1,
        "Fault 経路の終了では best-effort Unload が 1 度だけ記録されるはず: {:?}",
        recorded
    );
}

/// 停止通知が 1 件だけ届き、原因が Fault であることを確かめ、その中身を返す。
///
/// 呼び手は kanade を join してから読む——join の成功は終了系列の完了（停止通知の投函）の後なので、
/// 通知は既に届いている。
fn single_fault_notification(rx: &mpsc::Receiver<KanadeStopped>) -> ShioriFault {
    let first = rx
        .try_recv()
        .expect("終了系列の完了で停止通知が 1 件届くはず");
    assert!(rx.try_recv().is_err(), "停止通知は 1 件だけのはず");
    match first.cause {
        KanadeStopCause::Fault(fault) => fault,
        other => panic!("SHIORI の失敗で止まったなら停止原因は Fault のはず: {other:?}"),
    }
}

/// 記録 `event`（`kanade` の error）のうち、欄 `field` が `text` と同じ文言のものが残っているか。
///
/// 停止通知の理由が、対応する `error!` と同じ文言であることを確かめるのに使う。
fn error_logged_with(
    buffer: &std::sync::Mutex<Vec<log_capture_kit::CapturedEvent>>,
    event: &str,
    field: &str,
    text: &str,
) -> bool {
    buffer
        .lock()
        .expect("capture buffer mutex")
        .iter()
        .any(|ev| {
            ev.level == tracing::Level::ERROR
                && ev.target == "kanade"
                && ev.field_str("event") == Some(event)
                && ev.field(field) == Some(text)
        })
}

// ============================================================================
// ケース 1: 区別語彙ごとの呼出失敗 → 観測可能な終了（Req 6.1）
// ============================================================================

/// [`ShioriFailure`] のうち終了へ倒れる 4 語彙それぞれについて、boot 最初の呼出（`OnInitialize`）を
/// 当該語彙で失敗させると kanade が終了系列（Unloading{Fault}→Unload→Stopped）へ倒れて停止する。
///
/// # 駆動と観測（決定的・sleep なし）
/// Boot → `OnInitialize` NOTIFY で `Failed(kind)` を返す → 応答待ち（BootInit）で Failed を受領
/// → Unloading{Fault} → best-effort Unload（mock は `Unloaded` を返す）→ Stopped → StopSelf。
/// kanade は StopSelf で受信ループを Break するため、期限付き join が成功する。加えて記録列に
/// Unload が 1 度現れることで「Fault 経路の正規終了（best-effort unload）を通った」ことを確認する。
///
/// # 非空虚性
/// - Failed が終了を駆動しなければ kanade は BootInit で応答待ちのまま止まらず、join が期限超過して
///   panic する（＝失敗が観測可能な終了へ写像されていないことを検出する）。
/// - Fault 経路が Unload を発行しなければ `assert_unload_recorded_once` が落ちる。
/// - 4 語彙をループで網羅し、いずれか 1 つでも終了しなければそのイテレーションで panic する。
///   エラー応答（`Shiori`）は終了へ倒れない（本仕様 要件 6.1）ので、ケース 5 で別に見る。
///
/// # 停止通知の中身（本仕様 要件 2.1・2.2・7.3）
/// 停止通知は Fault で、種類は語彙ごとに期待した値（接続→接続できなかった・期限切れ→期限切れ・
/// 通信→通信が切れた・内部→内部の失敗）、理由は注入した失敗の文言であり、記録 `shiori_failed` の
/// `error` と同じ文言である。種類の写しを取り違えれば種類が、理由の運び方を変えれば理由が合わない。
#[test]
fn each_failure_vocabulary_drives_observable_termination() {
    // 記録 `shiori_failed` は kanade のアクタースレッドで出るので、起動前に全スレッド捕捉を据える。
    let buffer = install_global_capture_all();

    // 終了へ倒れる 4 語彙を網羅する。ShioriFailure は非 Clone ゆえ Copy な FailKind で回し、mock 内で
    // その都度 fresh に構築する（Fixture/FailOn は Copy／Clone で複製可能）。
    for (kind, expected_kind) in [
        (FailKind::Handshake, ShioriFaultKind::ConnectFailed),
        (FailKind::Timeout, ShioriFaultKind::Timeout),
        (FailKind::Ipc, ShioriFaultKind::Disconnected),
        (FailKind::Internal, ShioriFaultKind::Internal),
    ] {
        // 網羅の非空虚性を型で担保: 各 kind は実 ShioriFailure のバリアントに 1:1 対応する
        // （`Shiori` の腕はケース 5 が受け持つ）。mock が注入するのと同じ文言で組み、停止通知の
        // 理由の期待値にする（注入の文言が変われば理由が合わず赤になる）。
        // `FailKind::Internal`（DD-IT-11・kanade 内部規律違反＝ホワイトリスト違反で choke が返す語彙）
        // も本ループで掃引し、`Failed(Internal)` が外部4語彙と同じく既存 fault 終端（Unloading{Fault}
        // →Unload→Stopped）へ合流することを統合層で確認する（設計 Testing Strategy #7・DD-IT-11:
        // choke の ID 検証自体は actor.rs in-source 檻が担い、本檻は Internal→fault の routing を担う）。
        let witness: ShioriFailure = match kind {
            FailKind::Handshake => ShioriFailure::Handshake("injected handshake failure".into()),
            FailKind::Timeout => ShioriFailure::Timeout("injected timeout".into()),
            FailKind::Ipc => ShioriFailure::Ipc("injected ipc failure".into()),
            FailKind::Shiori => unreachable!("エラー応答はケース 5 で見る"),
            FailKind::Internal => ShioriFailure::Internal("injected internal violation".into()),
        };

        // boot 最初の呼出（OnInitialize NOTIFY）を当該語彙で失敗させる（停止通知の投函端つき）。
        let (stop_tx, stop_rx) = mpsc::channel::<KanadeStopped>();
        let harness = spawn_harness_failing_with_stop_sink(
            KanadeConfig::new("master", "1.0.0"),
            Fixture::default(),
            QuitPolicy::PerTalk(vec![false]),
            FailOn::on_initialize(kind),
            Some(stop_tx),
        );

        // Boot を駆動 → OnInitialize 失敗 → Unloading{Fault} → Unload → Stopped → StopSelf。
        harness.sender.send(KanadeMsg::Boot).expect("send Boot");

        let Harness {
            sender,
            kanade,
            shiori,
            sakura,
        } = harness;

        // 呼出失敗が観測可能な終了（停止）へ写像された証拠: kanade が期限内に join できる。
        join_bounded("kanade failing-boot join", DEFAULT_TIMEOUT, kanade).unwrap_or_else(|_| {
            panic!("kanade should terminate on injected OnInitialize failure ({kind:?})")
        });

        // Fault 経路の best-effort Unload が記録列に 1 度現れる（正規終了経路の直接証拠）。
        let recorded = shiori.recorded();
        assert_unload_recorded_once(&recorded);

        // 停止通知は Fault の種類と理由つきで 1 件届く。
        let fault = single_fault_notification(&stop_rx);
        assert_eq!(
            fault,
            ShioriFault {
                kind: expected_kind,
                reason: witness.to_string(),
            },
            "{kind:?} の失敗の停止通知は種類と理由を運ぶはず"
        );
        assert!(
            error_logged_with(&buffer, "shiori_failed", "error", &fault.reason),
            "停止通知の理由は記録 shiori_failed の error と同じ文言のはず: {:?}",
            fault.reason
        );

        // 後片付け: kanade 停止後に全 Sender を drop → sakura sink スレッドも自然終了。
        drop(sender);
        sakura.join_bounded("mock-sakura failing-boot join", DEFAULT_TIMEOUT);
    }
}

// ============================================================================
// ケース 2: 死活報告 → 観測可能な停止（Req 5.4）
// ============================================================================

/// `KanadeMsg::ShioriDown` を注入すると kanade は終了系列（Unloading{Fault}→Unload→Stopped）へ
/// 倒れて停止する（DD-4 の暫定 seam・Req 5.4）。
///
/// # 駆動と観測
/// Boot →（定常運転へ落ち着く）→ ShioriDown{reason} → Unloading{Fault} → best-effort Unload →
/// Stopped → StopSelf。ShioriDown は公開 inbox variant ゆえ `harness.sender` へ直接注入できる。
/// kanade の期限付き join が成功し、記録列に Unload が 1 度現れることで停止を観測する。
///
/// # 非空虚性
/// - ShioriDown が終了を駆動しなければ kanade は Steady のまま止まらず join が期限超過して panic する。
///
/// # 停止通知の中身（本仕様 要件 2.1・2.2）
/// 死活報告の 2 種類それぞれで、停止通知は Fault で、種類は「接続できなかった」→接続できなかった・
/// 「helper の終了」→通信が切れた、理由は報告の理由そのもの（記録 `shiori_down` の `reason` と同じ
/// 文言）。両方の理由を同じ綴りにしてあるので、種類を理由の綴りから判別していれば片方が合わない。
#[test]
fn shiori_down_drives_observable_stop() {
    // 記録 `shiori_down` は kanade のアクタースレッドで出るので、起動前に全スレッド捕捉を据える。
    let buffer = install_global_capture_all();

    for (down_kind, expected_kind) in [
        (
            ShioriDownKind::ConnectFailed,
            ShioriFaultKind::ConnectFailed,
        ),
        (ShioriDownKind::HelperExited, ShioriFaultKind::Disconnected),
    ] {
        shiori_down_case(&buffer, down_kind, expected_kind);
    }
}

/// 死活報告 1 種類ぶんの駆動と観測（[`shiori_down_drives_observable_stop`] の本体）。
fn shiori_down_case(
    buffer: &std::sync::Mutex<Vec<log_capture_kit::CapturedEvent>>,
    down_kind: ShioriDownKind,
    expected_kind: ShioriFaultKind,
) {
    let reason = "shiori went down (failure_test)";
    let (stop_tx, stop_rx) = mpsc::channel::<KanadeStopped>();
    let harness = spawn_harness_with_stop_sink(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::default(),
        QuitPolicy::PerTalk(vec![false]),
        Some(stop_tx),
    );

    // 起動して定常運転へ落ち着かせる（boot 系列は Boot 処理内で同期完走する）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send settle Tick");

    // 死活報告を注入 → 終了系列（Fault）へ。
    harness
        .sender
        .send(KanadeMsg::ShioriDown {
            kind: down_kind,
            reason: reason.to_string(),
        })
        .expect("send ShioriDown");

    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // 死活報告が観測可能な停止へ写像された証拠: kanade が期限内に join できる。
    join_bounded("kanade shiori-down join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates on ShioriDown (Req 5.4)");

    // Fault 経路の best-effort Unload が記録列に 1 度現れる。
    let recorded = shiori.recorded();
    assert_unload_recorded_once(&recorded);

    // 停止通知は Fault の種類と理由つきで 1 件届く。
    let fault = single_fault_notification(&stop_rx);
    assert_eq!(
        fault,
        ShioriFault {
            kind: expected_kind,
            reason: reason.to_string(),
        },
        "{down_kind:?} の死活報告の停止通知は種類と理由を運ぶはず"
    );
    assert!(
        error_logged_with(buffer, "shiori_down", "reason", &fault.reason),
        "停止通知の理由は記録 shiori_down の reason と同じ文言のはず: {:?}",
        fault.reason
    );

    drop(sender);
    sakura.join_bounded("mock-sakura shiori-down join", DEFAULT_TIMEOUT);
}

// ============================================================================
// ケース 3: 未知 talk_id の再生完了通知 → 運行継続（Req 2.5・6.2）
// ============================================================================

/// 採番されていない talk_id の `TalkDone` を注入しても kanade は終了せず運行を継続し、その後の
/// driven な close 指示で初めて正常終了する（未知 TalkDone は現 Phase 維持で無害・Req 2.5・6.2）。
///
/// # 駆動と観測（決定的・sleep なし）
/// Boot →（定常化）→ 未知 talk_id（9999）の TalkDone{quit:false} を注入 → kanade は現 Phase 維持で
/// 継続 → CloseRequest{User}（quitting fixture・OnClose Value→close talk quit:true）→ 終了系列完走。
/// kanade の期限付き join が成功する。もし未知 TalkDone が kanade を終了・破綻させていたら、後続の
/// close 駆動が届く前に停止しているか、あるいは異常終了しており、いずれにせよ「close で正常終了する」
/// 単一の合否を満たさない。
///
/// # 非空虚性
/// - 未知 TalkDone が kanade を終了させていたら、close talk は起動できず（Boot 済みだが Steady を
///   離れている）、close 経路の終了系列が駆動されない——それでも join は成功し得るため、close talk が
///   現に起動した（別れの StartTalk が sink に到達した）ことも併せて確認して非空虚にする。
/// - 未知 TalkDone が panic を誘発すれば kanade スレッドが異常終了し join がエラーを返して落ちる。
#[test]
fn unknown_talk_done_keeps_running_until_driven_close() {
    // quitting fixture: OnClose が別れの Value を返す（close talk 発生）。close talk（受領 index 1）を
    // quit:true にして終了系列を駆動する（boot talk index0=false・close talk index1=true）。
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, true]),
    );

    // 起動して定常運転へ落ち着かせる。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send settle Tick");

    // 未知 talk_id の再生完了通知を注入（採番されていない 9999・reason=Ended）。現 Phase 維持で無害。
    harness
        .sender
        .send(KanadeMsg::TalkDone(TalkDone {
            talk_id: TalkId(9_999),
            reason: TalkEndReason::Ended,
            quit_reserved: false,
        }))
        .expect("send unknown TalkDone");

    // 運行が継続していることを、driven な close で正常終了できることによって観測する。
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

    // 未知 TalkDone で終了・破綻していなければ、close→quit talk で終了系列が完走し join が成功する。
    join_bounded("kanade unknown-talkdone join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade keeps running past unknown TalkDone, then terminates on driven close");

    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura unknown-talkdone join", DEFAULT_TIMEOUT);

    // 非空虚性の補強: 未知 TalkDone が Steady を離脱させていなければ、その後の close 握手が
    // OnClose の別れ Value を受けて close talk を 1 本起動する（＝運行が確かに継続していた証拠）。
    let farewell_started = started
        .iter()
        .filter(|s| s.script == super::common::FIXED_FAREWELL_SCRIPT)
        .count();
    assert_eq!(
        farewell_started, 1,
        "未知 TalkDone 後も運行は継続し、driven な close が close talk を 1 本起動するはず: {:?}",
        started
    );

    // 終了系列を確かに通った証拠: 記録列末尾は Unload（close→quit talk→Unload→StopSelf）。
    let recorded = shiori.recorded();
    assert_unload_recorded_once(&recorded);
}

// ============================================================================
// ケース 4: 全ての指示送信元切断 → 宙吊りにならず正常終了・停止観測（Req 4.8・4.9・6.2）
// ============================================================================

/// kanade inbox の全 Sender を drop すると `run_inbox` が切断で正常終了し、kanade の期限付き join が
/// 成功する（受信待ちのままハングしない・Req 4.9 の構造保証）。
///
/// # sink を持たないハーネスを使う理由（sender 位相の罠）
/// 通常ハーネス（[`spawn_harness`]）の mock sakura sink は TalkDone 返送のため kanade inbox 送信端の
/// クローンを**恒久的に保持する**。そのため `Harness.sender` を drop しても sink のクローン越しに
/// inbox が生き続け、kanade は止まらない（sink は kanade の StartTalk 送信端 drop＝kanade 停止でしか
/// 閉じないため相互待ちになる）。全 Sender drop 経路（Req 4.9）を観測するには「kanade inbox の
/// クローンを誰も保持しない」結線が必要で、それを [`spawn_harness_no_sink`] が提供する——sink を
/// 起動せず、返す `sender` が inbox の唯一の送信端になる。
///
/// # 駆動と観測（決定的・sleep なし）
/// Boot で定常化させた上で、唯一の inbox 送信端 `sender` を drop する。inbox が完全に切断され
/// `run_inbox` の `rx.recv()` が Err を返してループが正常終了する（Close 未送信・StopSelf 未経由でも
/// 停止する＝4.9）。kanade の期限付き join が成功することで「宙吊りにならず正常終了した」ことを観測する。
///
/// # 非空虚性
/// - inbox 切断で kanade が正常終了しなければ、Close も StopSelf も送っていないため kanade は
///   `rx.recv()` で永久にブロックし、join が期限超過して panic する（＝4.9 の構造保証が壊れている
///   ことを検出する）。
/// - Boot を先に送ることで「運行中（Steady）の kanade でも全 Sender drop で正常終了する」ことを
///   確認する（Idle のまま切断する自明経路ではなく、運行途中からの切断を踏む）。
#[test]
fn all_command_senders_dropped_terminates_normally_without_hang() {
    let harness = spawn_harness_no_sink(KanadeConfig::new("master", "1.0.0"), Fixture::default());

    // 起動して定常運転へ落ち着かせる（運行中からの切断を踏む）。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    harness
        .sender
        .send(KanadeMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send settle Tick");

    let SinklessHarness {
        sender,
        kanade,
        shiori,
        talk_rx,
    } = harness;

    // 全ての指示送信元（inbox の唯一の Sender）を drop → inbox 完全切断。
    // talk_rx は保持したまま（drop すると StartTalk 送出が error! になるだけで停止観測には無関係だが、
    // 生かしておけば「送出成功しつつ受信待ち」の純粋な切断経路を踏む）。
    drop(sender);

    // Close も StopSelf も経ずに、inbox 切断だけで kanade が正常終了する（Req 4.9・宙吊りなし）。
    join_bounded("kanade all-senders-dropped join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates normally on inbox disconnect without hanging (Req 4.9)");

    // talk_rx はここまで保持していた（kanade の StartTalk 送出を成功させるため）。明示的に drop する。
    drop(talk_rx);

    // mock shiori は kanade が StopSelf を経ないため Close を受けない。送信端 drop で自然終了させる
    // （記録は不要・停止観測はここまでで完了）。
    drop(shiori);
}

// ============================================================================
// ケース 5: エラー応答 → 返事なしと同じ扱いで会話を続ける（本仕様 要件 6.1・7.3）
// ============================================================================

/// 記録 `shiori_error_response`（`kanade` の warn）のうち、`id` の往復で注入したエラー応答のものを数える。
///
/// 記録は kanade のアクタースレッドで出るので、全スレッド捕捉の蓄積先から拾う。同じテスト
/// バイナリの他のテストの記録も混ざるので、注入の文言（`FailKind::Shiori` が作る
/// `"injected shiori error"`）と `id` で絞る（このケース以外にエラー応答を注入するテストは無い）。
fn count_error_response_records(
    buffer: &std::sync::Mutex<Vec<log_capture_kit::CapturedEvent>>,
    id: &str,
) -> usize {
    buffer
        .lock()
        .expect("capture buffer mutex")
        .iter()
        .filter(|ev| {
            ev.level == tracing::Level::WARN
                && ev.target == "kanade"
                && ev.field_str("event") == Some("shiori_error_response")
                && ev.field("id") == Some(id)
                && ev
                    .field("error")
                    .is_some_and(|e| e.contains("injected shiori error"))
        })
        .count()
}

/// 記録列で `method` の `id` の呼出が最初に現れる位置。
fn position_of(recorded: &[RecordedCall], method: CallMethod, id: &str) -> Option<usize> {
    recorded
        .iter()
        .position(|c| c.method == method && c.id == id)
}

/// close 指示で終わったこと（Fault で止まっていないこと）を確かめる。
///
/// OnClose GET が記録され、別れの talk が 1 本起き、Unload は末尾に 1 件だけ。エラー応答が
/// Fault へ倒れていれば OnClose も別れの talk も現れない。
fn assert_ended_by_driven_close(recorded: &[RecordedCall], started: &[areka_kanade::StartTalk]) {
    assert!(
        position_of(recorded, CallMethod::Get, "OnClose").is_some(),
        "close 指示の OnClose GET が記録されるはず（Fault で止まれば現れない）: {recorded:?}"
    );
    assert_eq!(
        started
            .iter()
            .filter(|s| s.script == FIXED_FAREWELL_SCRIPT)
            .count(),
        1,
        "close 指示で別れの talk が 1 本起きるはず: {started:?}"
    );
    assert_unload_recorded_once(recorded);
    assert_eq!(
        recorded.last().map(|c| &c.method),
        Some(&CallMethod::Unload),
        "末尾は Unload（close 指示→別れの talk quit:true 由来）: {recorded:?}"
    );
}

/// 起動時の `OnInitialize`（NOTIFY）がエラー応答でも kanade は止まらず起動を続け、その後の
/// close 指示で初めて終わる。記録 `shiori_error_response` は 1 件。
///
/// # 駆動と観測（決定的・sleep なし）
/// 挨拶なし boot（`OnBoot`→204）で `Steady{None}` へ直行させ、close 指示（`OnClose` Value→別れの
/// talk quit:true→Unload）で終える。kanade の期限付き join 成功時点で記録列は確定している。
///
/// # 非空虚性
/// エラー応答が Fault へ倒れていれば、`OnInitialize` の直後に Unload が来て `OnBoot` も `OnClose` も
/// 記録されず、別れの talk も起きない。
#[test]
fn error_response_at_boot_notify_continues_as_notified() {
    // アクター起動前に据え付ける（別スレッドで出る warn を拾う）。
    let buffer = install_global_capture_all();

    let harness = spawn_harness_failing(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
        FailOn::on_initialize(FailKind::Shiori),
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

    join_bounded("kanade boot-error-response join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close, not via the error response");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura boot-error-response join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) エラー応答のあとも起動が進んだ: OnInitialize の後に OnBoot GET が記録される。
    let init = position_of(&recorded, CallMethod::Notify, "OnInitialize")
        .expect("OnInitialize NOTIFY が記録されるはず");
    let boot = position_of(&recorded, CallMethod::Get, "OnBoot").unwrap_or_else(|| {
        panic!("エラー応答で止まらなければ OnBoot GET まで進むはず: {recorded:?}")
    });
    assert!(
        init < boot,
        "起動は OnInitialize の後に進むはず: {recorded:?}"
    );

    // (2) 終わりは close 指示から（Fault で止まっていない）。
    assert_ended_by_driven_close(&recorded, &started);

    // (3) 記録は 1 往復に 1 件。
    assert_eq!(
        count_error_response_records(&buffer, "OnInitialize"),
        1,
        "OnInitialize のエラー応答の記録 shiori_error_response は 1 件のはず"
    );
}

/// 会話中の `OnSecondChange`（GET）がエラー応答でも kanade は止まらず、次の Tick の
/// `OnSecondChange` GET も発行され、その後の close 指示で初めて終わる。記録は 1 件。
///
/// # 非空虚性
/// エラー応答が Fault へ倒れていれば、1 本目の `OnSecondChange` の後に Unload が来て、2 本目の
/// `OnSecondChange` も `OnClose` も記録されない。
#[test]
fn error_response_at_conversation_get_continues_as_no_content() {
    // アクター起動前に据え付ける（別スレッドで出る warn を拾う）。
    let buffer = install_global_capture_all();

    let harness = spawn_harness_failing(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
        FailOn {
            id: "OnSecondChange",
            kind: FailKind::Shiori,
        },
    );

    harness.sender.send(KanadeMsg::Boot).expect("send Boot");
    for i in 1..=2 {
        harness
            .sender
            .send(KanadeMsg::Tick {
                now: MonotonicMs(i * 1_000),
            })
            .expect("send Tick");
    }
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

    join_bounded("kanade steady-error-response join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates via the driven close, not via the error response");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura steady-error-response join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) エラー応答のあとも会話が続いた: 次の Tick の OnSecondChange GET も記録される（計 2 件）。
    assert_eq!(
        recorded
            .iter()
            .filter(|c| c.method == CallMethod::Get && c.id == "OnSecondChange")
            .count(),
        2,
        "エラー応答で止まらなければ 2 本目の OnSecondChange GET も記録されるはず: {recorded:?}"
    );

    // (2) 終わりは close 指示から（Fault で止まっていない）。
    assert_ended_by_driven_close(&recorded, &started);

    // (3) 記録は 1 往復に 1 件（エラー応答を返したのは 1 本目だけ）。
    assert_eq!(
        count_error_response_records(&buffer, "OnSecondChange"),
        1,
        "OnSecondChange のエラー応答の記録 shiori_error_response は 1 件のはず"
    );
}

/// close 指示の `OnClose`（GET）がエラー応答なら、返事なしと同じ扱いで別れの台詞なしに閉じる
/// （無言の終了→Unload）。kanade は期限内に終わり、記録は 1 件。
///
/// # 非空虚性
/// GET のエラー応答を取り違えて通知済み（Notified）に写すと、閉じる相は Notified を想定外として
/// 相を保つので終わらず、期限付き join が赤になる。
#[test]
fn error_response_at_close_get_closes_silently() {
    // アクター起動前に据え付ける（別スレッドで出る warn を拾う）。
    let buffer = install_global_capture_all();

    let harness = spawn_harness_failing(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting().without_boot_greeting(),
        QuitPolicy::PerTalk(vec![true]),
        FailOn {
            id: "OnClose",
            kind: FailKind::Shiori,
        },
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

    join_bounded("kanade close-error-response join", DEFAULT_TIMEOUT, kanade)
        .expect("OnClose のエラー応答は返事なしとして無言で閉じ、kanade は期限内に終わるはず");
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura close-error-response join", DEFAULT_TIMEOUT);
    let recorded = shiori.recorded();

    // (1) OnClose GET は発行され、別れの talk は起きない（返事なし＝無言の終了）。
    assert!(
        position_of(&recorded, CallMethod::Get, "OnClose").is_some(),
        "OnClose GET が記録されるはず: {recorded:?}"
    );
    assert!(
        started.iter().all(|s| s.script != FIXED_FAREWELL_SCRIPT),
        "返事なしの close では別れの talk は起きないはず: {started:?}"
    );

    // (2) 終了系列を通った: Unload は末尾に 1 件。
    assert_unload_recorded_once(&recorded);
    assert_eq!(
        recorded.last().map(|c| &c.method),
        Some(&CallMethod::Unload),
        "末尾は Unload（無言の終了）: {recorded:?}"
    );

    // (3) 記録は 1 往復に 1 件。
    assert_eq!(
        count_error_response_records(&buffer, "OnClose"),
        1,
        "OnClose のエラー応答の記録 shiori_error_response は 1 件のはず"
    );
}

// ============================================================================
// ケース 6: 送出失敗 → 通信が切れた（本仕様 要件 2.2）
// ============================================================================

/// SHIORI 側の受け口が先に閉じていると、起動の最初の呼出の送出が失敗して終了系列へ入り、停止通知は
/// 「通信が切れた」の種類と、記録 `shiori_failed` の `error` と同じ理由で届く。
///
/// # 駆動と観測（決定的・sleep なし）
/// mock shiori へ `Close` を送って受信ループを抜けさせ、スレッドの終わりを待つ（受け口が落ちる）。
/// その後 Boot を送ると、kanade の送出点が送出失敗を「通信の失敗」として再投入し、Unloading{Fault}
/// → Unload（これも送出失敗）→ Stopped へ進む。
///
/// # 非空虚性
/// 送出失敗が終了を駆動しなければ join が期限超過する。送出失敗の種類を取り違えれば種類が合わない。
#[test]
fn send_failure_delivers_a_disconnected_fault() {
    // 記録 `shiori_failed` は kanade のアクタースレッドで出るので、起動前に全スレッド捕捉を据える。
    let buffer = install_global_capture_all();

    let (stop_tx, stop_rx) = mpsc::channel::<KanadeStopped>();
    let harness = spawn_harness_with_stop_sink(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::default(),
        QuitPolicy::PerTalk(vec![false]),
        Some(stop_tx),
    );
    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    // SHIORI 側の受け口を先に閉じる（kanade の持つ送信端はそのまま＝送出だけが失敗する）。
    shiori
        .sender
        .send(areka_kanade::ShioriMsg::Close)
        .expect("send Close to mock shiori");
    join_bounded("mock-shiori close join", DEFAULT_TIMEOUT, shiori.handle)
        .expect("mock shiori stops on Close");

    sender.send(KanadeMsg::Boot).expect("send Boot");

    join_bounded("kanade send-failure join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates when the SHIORI request cannot be sent");

    let fault = single_fault_notification(&stop_rx);
    assert_eq!(
        fault,
        ShioriFault {
            kind: ShioriFaultKind::Disconnected,
            reason: ShioriFailure::Ipc("shiori channel disconnected".into()).to_string(),
        },
        "送出失敗の停止通知は「通信が切れた」の種類と理由を運ぶはず"
    );
    assert!(
        error_logged_with(&buffer, "shiori_failed", "error", &fault.reason),
        "停止通知の理由は記録 shiori_failed の error と同じ文言のはず: {:?}",
        fault.reason
    );

    drop(sender);
    sakura.join_bounded("mock-sakura send-failure join", DEFAULT_TIMEOUT);
}

/// SHIORI 側が返事をせずに返信口を捨てると（応答の切断）、停止通知は「通信が切れた」の種類と、
/// 記録 `shiori_failed` の `error` と同じ理由で届く。
///
/// # 駆動と観測（決定的・sleep なし）
/// 呼出（`Request`）の返信口は答えずに捨て、`Unload` には `Unloaded` を返す SHIORI 側をテストの
/// スレッドで立てる。Boot を送ると最初の呼出の応答が切れ、Unloading{Fault}→Unload→Stopped へ進む。
///
/// # 非空虚性
/// 応答の切断が終了を駆動しなければ join が期限超過する。切断の種類を取り違えれば種類が合わない。
#[test]
fn reply_dropped_delivers_a_disconnected_fault() {
    // 記録 `shiori_failed` は kanade のアクタースレッドで出るので、起動前に全スレッド捕捉を据える。
    let buffer = install_global_capture_all();

    let (shiori_tx, shiori_rx) = mpsc::channel::<areka_kanade::ShioriMsg>();
    let shiori = std::thread::spawn(move || {
        // kanade が止まって送信端を捨てると受信が切れて抜ける。
        while let Ok(msg) = shiori_rx.recv() {
            match msg {
                // 答えずに返信口を捨てる（応答の切断）。
                areka_kanade::ShioriMsg::Request { reply, .. } => drop(reply),
                areka_kanade::ShioriMsg::Unload { reply } => {
                    let _ = reply.send(areka_kanade::ShioriOutcome::Unloaded);
                }
                areka_kanade::ShioriMsg::Close => break,
            }
        }
    });

    // 起動の最初の呼出で止まるので talk は起きない。受信端は保持だけする。
    let (talk_tx, _talk_rx) = mpsc::channel::<areka_kanade::TalkCommand>();
    let (stop_tx, stop_rx) = mpsc::channel::<KanadeStopped>();
    let (sender, kanade) = areka_kanade::spawn_kanade_with_stop_sink(
        KanadeConfig::new("master", "1.0.0"),
        shiori_tx,
        talk_tx,
        Box::new(|_, _| {}),
        Some(stop_tx),
    );

    sender.send(KanadeMsg::Boot).expect("send Boot");

    join_bounded("kanade reply-dropped join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates when the SHIORI reply is dropped");

    let fault = single_fault_notification(&stop_rx);
    assert_eq!(
        fault,
        ShioriFault {
            kind: ShioriFaultKind::Disconnected,
            reason: ShioriFailure::Ipc("shiori reply dropped".into()).to_string(),
        },
        "応答の切断の停止通知は「通信が切れた」の種類と理由を運ぶはず"
    );
    assert!(
        error_logged_with(&buffer, "shiori_failed", "error", &fault.reason),
        "停止通知の理由は記録 shiori_failed の error と同じ文言のはず: {:?}",
        fault.reason
    );

    // kanade が止まって SHIORI 側の送信端が消えたので、テストのスレッドも抜ける。
    drop(sender);
    shiori.join().expect("reply-dropping shiori thread");
}
