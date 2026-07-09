//! 失敗経路の統合検証（Req 2.5・4.8・4.9・5.4・6.1・6.2）。
//!
//! mock 結線（`super::common` のハーネス）で、kanade の失敗・停止・回復各経路を統合層で
//! 決定的に観測する（実時間 sleep なし・全 join は期限付き・宙吊りしない）。検証する 4 系:
//!
//! 1. **区別語彙ごとの呼出失敗 → 観測可能な終了**（Req 6.1）: [`ShioriFailure`] の 4 語彙
//!    （Handshake／Timeout／Ipc／Shiori）それぞれについて、boot 最初の呼出（`OnInitialize`）を
//!    その語彙で失敗させ、kanade が終了系列（Unloading{Fault}→best-effort Unload→Stopped）へ
//!    倒れて停止すること（kanade の期限付き join 成功＋Unload 記録）を観測する。
//! 2. **死活報告 → 観測可能な停止**（Req 5.4）: boot 定常化後に `KanadeMsg::ShioriDown` を注入し、
//!    kanade が Unloading{Fault}→Unload→Stopped で停止すること（join 成功＋Unload 記録）を観測する。
//! 3. **未知 talk_id の再生完了通知 → 運行継続**（Req 2.5・6.2）: 採番されていない talk_id の
//!    `TalkDone` を注入しても kanade は終了せず、その後 driven な close で初めて正常終了することを
//!    観測する（未知 TalkDone は現 Phase 維持で無害）。
//! 4. **全ての指示送信元切断 → 宙吊りにならず正常終了・停止観測**（Req 4.8・4.9・6.2）: kanade
//!    inbox の全 Sender を drop すると `run_inbox` が切断で正常終了し、kanade の期限付き join が
//!    成功する（受信待ちのままハングしない・4.9 の構造保証）。

use areka_kanade::{
    CloseReason, KanadeConfig, KanadeMsg, MonotonicMs, ShioriFailure, TalkDone, TalkEndReason,
    TalkId,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FailKind, FailOn, Fixture, Harness, QuitPolicy, SinklessHarness,
    join_bounded, spawn_harness, spawn_harness_failing, spawn_harness_no_sink,
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

// ============================================================================
// ケース 1: 区別語彙ごとの呼出失敗 → 観測可能な終了（Req 6.1）
// ============================================================================

/// [`ShioriFailure`] の 4 語彙それぞれについて、boot 最初の呼出（`OnInitialize`）を当該語彙で
/// 失敗させると kanade が終了系列（Unloading{Fault}→Unload→Stopped）へ倒れて停止する。
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
#[test]
fn each_failure_vocabulary_drives_observable_termination() {
    // 4 語彙を静的に網羅する。ShioriFailure は非 Clone ゆえ Copy な FailKind で回し、mock 内で
    // その都度 fresh に構築する（Fixture/FailOn は Copy／Clone で複製可能）。
    for kind in [
        FailKind::Handshake,
        FailKind::Timeout,
        FailKind::Ipc,
        FailKind::Shiori,
    ] {
        // 網羅の非空虚性を型で担保: 各 kind は実 ShioriFailure の 4 バリアントに 1:1 対応する。
        let _witness: ShioriFailure = match kind {
            FailKind::Handshake => ShioriFailure::Handshake(String::new()),
            FailKind::Timeout => ShioriFailure::Timeout(String::new()),
            FailKind::Ipc => ShioriFailure::Ipc(String::new()),
            FailKind::Shiori => ShioriFailure::Shiori(String::new()),
        };

        // boot 最初の呼出（OnInitialize NOTIFY）を当該語彙で失敗させる。
        let harness = spawn_harness_failing(
            KanadeConfig::new("master", "1.0.0"),
            Fixture::default(),
            QuitPolicy::PerTalk(vec![false]),
            FailOn::on_initialize(kind),
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
        join_bounded(
            "kanade failing-boot join",
            DEFAULT_TIMEOUT,
            kanade,
        )
        .unwrap_or_else(|_| {
            panic!("kanade should terminate on injected OnInitialize failure ({kind:?})")
        });

        // Fault 経路の best-effort Unload が記録列に 1 度現れる（正規終了経路の直接証拠）。
        let recorded = shiori.recorded();
        assert_unload_recorded_once(&recorded);

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
#[test]
fn shiori_down_drives_observable_stop() {
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

    // 死活報告を注入 → 終了系列（Fault）へ。
    harness
        .sender
        .send(KanadeMsg::ShioriDown {
            reason: "helper crashed".to_string(),
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
        }))
        .expect("send unknown TalkDone");

    // 運行が継続していることを、driven な close で正常終了できることによって観測する。
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
    let harness = spawn_harness_no_sink(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::default(),
    );

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
