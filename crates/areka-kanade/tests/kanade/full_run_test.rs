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
//! 単一 `#[test]` の中で 5 回連続して通ることを確かめる（各反復は新規ハーネスを spawn する）。

use areka_kanade::{
    CloseReason, ExecutionSnapshot, KanadeConfig, KanadeMsg, StartTalk, TalkId, events, resources,
};

use super::common::{
    CallMethod, Driven, FIXED_BOOT_SCRIPT, FIXED_FAREWELL_SCRIPT, Fixture, QuitPolicy,
    expected_call, expected_translate, expected_unload, send, spawn_harness, tick,
};

/// 主観測の駆動と検証を 1 回分実行する（反復のため単一関数に閉じる）。
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
    send(&harness, KanadeMsg::Boot);

    // 毎秒 Tick を数回（定常運転の pump を駆動）。OnSecondChange は 204 基調ゆえ talk なし。
    for i in 1..=3u64 {
        tick(&harness, i * 1_000);
    }

    // close 指示（active talk なしなら即握手・boot 挨拶完了後に握手）。握手は OnClose GET→別れの Value→close talk。
    send(
        &harness,
        KanadeMsg::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
    );

    // close talk（id 2）の TalkDone は mock sakura が quit:true で自動返送する
    // （PerTalk index 1）。これが終了系列（Unloading{Quit}→Unload→StopSelf）を駆動する。
    //
    // (c) 終了完走の観測: StopSelf まで到達しないと kanade スレッドは join できない。
    // 期限付き join が成功すれば、それまでの全 shiori 呼出は記録済みである。
    let Driven {
        recorded, started, ..
    } = harness.finish("kanade full-run join");

    // ========================================================================
    // (a) 起動系列が正典順序で発火（NOTIFY／GET の別・Reference 構成込み）
    // ========================================================================
    // 期待値は events 表から導出する（ハーネスに Reference をハードコードしない・Req 7.1）。
    // boot 系列前段は INACTIVE（Status 行なし）・basewareversion は挨拶追跡後の talk_active=true
    // （Status: talking・DD-IT-12）。挨拶の台詞は再生の前に OnTranslate へかける（偽の shiori は 204）。
    let talking = ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
        ..ExecutionSnapshot::INACTIVE
    };
    let on_boot = events::on_boot(&config, &ExecutionSnapshot::INACTIVE);
    // 挨拶の翻訳の期待は OnBoot の呼出を借りて先に組む（`ShioriCall` は Clone を持たない）。
    let boot_translate = expected_translate(FIXED_BOOT_SCRIPT, &on_boot, &talking);
    let expected_boot_prefix = vec![
        expected_call(events::on_initialize(&ExecutionSnapshot::INACTIVE)), // NOTIFY
        expected_call(resources::resource_get(
            "username",
            &ExecutionSnapshot::INACTIVE,
        )), // GET（prefetch・R4.1）→204
        expected_call(events::on_first_boot(&ExecutionSnapshot::INACTIVE, 0)), // GET →204
        expected_call(on_boot),                                             // GET →Value
        boot_translate,                                                     // GET（挨拶の翻訳）→204
        expected_call(events::baseware_version(&config, &talking)), // NOTIFY（Status: talking）
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
    assert_eq!(
        boot_talk.talk_id,
        TalkId(1),
        "boot talk の talk_id は 1（先頭採番）"
    );

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

    // ========================================================================
    // (c) close 指示→再生完了通知を待って終了系列が完走
    // ========================================================================
    // OnClose GET（Ref0=user）が pump 群の後に現れ、記録列の末尾が Unload で閉じる。
    // 通常握手の OnClose は talk 非アクティブ（INACTIVE）で発行される（Status 行なし・begin_close）。
    let onclose = expected_call(events::on_close(
        CloseReason::User { scope: 0 },
        &ExecutionSnapshot::INACTIVE,
    ));
    // 起動系列は先頭で一致済み（OnClose を含まない）なので、現れれば起動系列の後にある。
    assert!(
        recorded.contains(&onclose),
        "OnClose GET（Ref0=user）が記録列に現れるはず"
    );

    // 終了系列: 記録列の末尾は Unload（正規終了経路）で閉じる。
    let last = recorded.last().expect("記録列は空でない");
    assert_eq!(
        *last,
        expected_unload(),
        "記録列の末尾は Unload（終了系列完走）で閉じるはず"
    );

    // 終了系列以外（pump フェーズ）で Unload は 1 度だけ（末尾のみ）。
    let unload_count = recorded
        .iter()
        .filter(|c| c.method == CallMethod::Unload)
        .count();
    assert_eq!(unload_count, 1, "Unload は終了系列で 1 度だけ発行される");

    // pump: 送った刻み 3 本がどれも OnSecondChange として届く（GET か NOTIFY かは問わない）。
    let ticks = recorded.iter().filter(|c| c.id == "OnSecondChange").count();
    assert_eq!(
        ticks, 3,
        "刻み 3 本ぶんの OnSecondChange が記録に残るはず: {recorded:?}"
    );
}

/// 主観測: boot→pump→close→終了の運行全体を単一の合否として検証し、新規ハーネスで 5 回連続して
/// 通ることを確かめる（Req 7.1・7.2・7.3）。
///
/// 各反復は新規ハーネスを spawn し、実時間 sleep を用いず注入 Tick のみで運行を進める。
#[test]
fn full_run_boot_pump_close_terminates_deterministically() {
    for _ in 0..5 {
        drive_full_run();
    }
}
