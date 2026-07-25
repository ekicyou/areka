//! 【主観測】boot→pump→close 完走の単一 pass/fail（Req 7.1–7.3）。
//!
//! mock shiori＋mock sakura sink を kanade に結線し（`super::common` のハーネス）、
//! 起動指示から毎秒 Tick 数回・close 指示・終了完了までの運行全体を単一の `#[test]` で
//! 駆動する。時刻は注入した [`MonotonicMs`] Tick のみで進行させ（実時間 sleep なし・
//! mock 即応・全 join は期限付き）、以下 3 点を単一の合否として検証する:
//!
//! - (a) 起動系列が正典順序で発火したこと（NOTIFY／GET の別・Reference 構成込み）。
//! - (b) スクリプト受領から再生起動要求が宛先（mock sakura sink）に到達したこと。
//! - (c) close 指示から再生完了通知を待って終了系列（Unload→停止 join）が完走したこと。
//!
//! 決定性（Req 7.3）を担保するため、シナリオ全体を [`drive_full_run`] に閉じ込め、
//! 単一 `#[test]` の中で複数回連続実行して同一結果になることを確認する（各反復は
//! 新規ハーネスを spawn する）。

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, MonotonicMs, StartTalk, TalkId, events,
    resources,
};

use super::common::{
    CallMethod, DEFAULT_TIMEOUT, FIXED_BOOT_SCRIPT, FIXED_FAREWELL_SCRIPT, Fixture, Harness,
    QuitPolicy, expected_call, expected_unload, join_bounded, spawn_harness,
};

/// 主観測の駆動と検証を 1 回分実行する（反復同一性のため単一関数に閉じる）。
///
/// 各呼出は新規ハーネスを spawn し、boot→pump→close→終了の運行全体を注入 Tick のみで
/// 駆動する。終了系列完了（kanade の期限付き join 成功）まで待った上で、shiori 記録列と
/// sakura 受領列を単一の合否として検証する。
fn drive_full_run() {
    // 期待値導出用の config（events 表への入力）。`KanadeConfig` は Clone を実装しないため、
    // ハーネスへ渡す実体は同一パラメタで別途構築する（`new` は決定的ゆえ同値になる）。
    let config = KanadeConfig::new("master", "1.0.0");

    // quit シナリオ: OnClose が別れの Value を返す（close talk 発生）。
    // boot talk（id 1）は quit:false・close talk（id 2）は quit:true にして、
    // close talk 完了で終了系列（Unload→StopSelf）が完走するようにする。
    let harness = spawn_harness(
        KanadeConfig::new("master", "1.0.0"),
        Fixture::quitting(),
        QuitPolicy::PerTalk(vec![false, true]),
    );

    // --- 運行全体を注入で駆動する（実時間 sleep なし・時刻は Tick のみ） ---

    // 起動指示。
    harness.sender.send(KanadeMsg::Boot).expect("send Boot");

    // 毎秒 Tick を数回（定常運転の pump を駆動）。OnSecondChange は 204 基調ゆえ talk なし。
    for i in 1..=3u64 {
        harness
            .sender
            .send(KanadeMsg::Tick {
                now: MonotonicMs(i * 1_000),
            })
            .expect("send Tick");
    }

    // close 指示。active talk なしゆえ即握手開始（OnClose GET→別れの Value→close talk）。
    harness
        .sender
        .send(KanadeMsg::CloseRequest {
            reason: CloseReason::User,
        })
        .expect("send CloseRequest");

    // close talk（id 2）の TalkDone は mock sakura が quit:true で自動返送する
    // （PerTalk index 1）。これが終了系列（Unloading{Quit}→Unload→StopSelf）を駆動する。
    //
    // (c) 終了完走の観測: StopSelf まで到達しないと kanade スレッドは join できない。
    // 期限付き join が成功すれば、それまでの全 shiori 呼出は記録済みである。
    let Harness {
        sender,
        kanade,
        shiori,
        sakura,
    } = harness;

    join_bounded("kanade full-run join", DEFAULT_TIMEOUT, kanade)
        .expect("kanade terminates after close→quit sequence (Unload→StopSelf)");

    // kanade 停止後に送信端を drop → sakura sink スレッドも自然終了（期限付き join）。
    drop(sender);
    let started = sakura.started();
    sakura.join_bounded("mock-sakura full-run join", DEFAULT_TIMEOUT);

    let recorded = shiori.recorded();

    // ========================================================================
    // (a) 起動系列が正典順序で発火（NOTIFY／GET の別・Reference 構成込み）
    // ========================================================================
    // 期待値は events 表から導出する（ハーネスに Reference をハードコードしない・Req 7.1）。
    // boot 系列前段は INACTIVE（Status 行なし）・basewareversion は挨拶追跡後の talk_active=true
    // （Status: talking・DD-IT-12）。
    let expected_boot_prefix = vec![
        expected_call(events::on_initialize(&ExecutionSnapshot::INACTIVE)), // NOTIFY
        expected_call(resources::resource_username(&ExecutionSnapshot::INACTIVE)), // GET（prefetch・R4.1）→204
        expected_call(events::on_first_boot(&ExecutionSnapshot::INACTIVE, 0)), // GET →204
        expected_call(events::on_boot(&config, &ExecutionSnapshot::INACTIVE)), // GET →Value
        expected_call(events::baseware_version(
            &config,
            &ExecutionSnapshot { talk_active: true },
        )), // NOTIFY（Status: talking）
    ];
    assert!(
        recorded.len() >= expected_boot_prefix.len(),
        "記録列が起動系列より短い: {recorded:?}"
    );
    assert_eq!(
        &recorded[..expected_boot_prefix.len()],
        expected_boot_prefix.as_slice(),
        "起動系列が正典順序（Method・Reference 構成込み）で発火していない"
    );

    // ========================================================================
    // (b) スクリプト受領→再生起動要求が宛先（sakura sink）に到達
    // ========================================================================
    // boot talk（OnBoot Value）と close talk（OnClose Value）の 2 本が sink に届く。
    assert_eq!(
        started.len(),
        2,
        "boot talk と close talk の 2 本が sink に到達するはず: {started:?}"
    );

    // boot talk: script は boot fixture スクリプト・talk_id は先頭採番（1）。
    let boot_talk: &StartTalk = &started[0];
    assert_eq!(
        boot_talk.script, FIXED_BOOT_SCRIPT,
        "boot talk の script は boot fixture スクリプトと一致するはず"
    );
    assert_eq!(boot_talk.talk_id, TalkId(1), "boot talk の talk_id は 1（先頭採番）");

    // close talk: script は別れの fixture スクリプト。
    let close_talk: &StartTalk = &started[1];
    assert_eq!(
        close_talk.script, FIXED_FAREWELL_SCRIPT,
        "close talk の script は別れの fixture スクリプトと一致するはず"
    );

    // talk_id は一意・単調増番（再利用しない）。TalkId は PartialOrd 非実装ゆえ内部値で比較する。
    assert!(
        close_talk.talk_id.0 > boot_talk.talk_id.0,
        "talk_id は単調増番（{:?} < {:?}）",
        boot_talk.talk_id,
        close_talk.talk_id
    );
    assert_ne!(boot_talk.talk_id, close_talk.talk_id, "talk_id は一意");

    // ========================================================================
    // (c) close 指示→再生完了通知を待って終了系列が完走
    // ========================================================================
    // OnClose GET（Ref0=user）が pump 群の後に現れ、記録列の末尾が Unload で閉じる。
    // 通常握手の OnClose は talk 非アクティブ（INACTIVE）で発行される（Status 行なし・begin_close）。
    let onclose = expected_call(events::on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE));
    let onclose_index = recorded
        .iter()
        .position(|c| *c == onclose)
        .expect("OnClose GET（Ref0=user）が記録列に現れるはず");

    // OnClose は起動系列の後（＝pump フェーズより後）に現れる。
    assert!(
        onclose_index >= expected_boot_prefix.len(),
        "OnClose は起動系列の後に現れるはず（index={onclose_index}）"
    );

    // 終了系列: 記録列の末尾は Unload（正規終了経路）で閉じる。
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(
        *last,
        expected_unload(),
        "記録列の末尾は Unload（終了系列完走）で閉じるはず"
    );
    assert_eq!(last.method, CallMethod::Unload);

    // OnClose→…→Unload の順序（OnClose が Unload より前）。
    let unload_index = recorded.len() - 1;
    assert!(
        onclose_index < unload_index,
        "OnClose（{onclose_index}）は Unload（{unload_index}）より前に現れるはず"
    );

    // 終了系列以外（pump フェーズ）で Unload は 1 度だけ（末尾のみ）。
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(unload_count, 1, "Unload は終了系列で 1 度だけ発行される");
}

/// 主観測: boot→pump→close→終了の運行全体を単一の合否として検証し、反復実行で同一結果に
/// なることを確認する（Req 7.1・7.2・7.3）。
///
/// 各反復は新規ハーネスを spawn し、実時間 sleep を用いず注入 Tick のみで運行を進める。
/// 5 反復すべてで (a)(b)(c) が green になれば、決定的観測として合格とする。
#[test]
fn full_run_boot_pump_close_terminates_deterministically() {
    // 反復同一性（Req 7.3）: 複数回連続実行しても同一結果で green になることを確認する。
    for iteration in 0..5 {
        // 各反復は独立関数呼出＝新規ハーネス。失敗時にどの反復かを添えて報告する。
        let result = std::panic::catch_unwind(drive_full_run);
        assert!(
            result.is_ok(),
            "full_run は反復 {iteration} 回目で失敗した（決定性違反）"
        );
    }
}
