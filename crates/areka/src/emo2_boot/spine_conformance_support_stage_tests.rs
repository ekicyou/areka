//! 段の駆動器の受け入れ確認のうち、kanade を実際に起こして終了段の始まりを縛る檻
//! （areka-P0-emo2-conformance-e2e・task 5.5・R2.10）。
//!
//! `spine_conformance_support_tests.rs` が 1 ファイル 1,000 行の上限に近いため、中身を変えずに
//! 後ろ半分の 2 本を分けた（areka-test-binary-stability・task 1.2・R10.4）。檻の趣旨は前半の
//! ファイルの冒頭の説明と同じで、期待値は持たない。

use super::driver_tests::{second_change_calls, stage};
use super::spin_wait_until;
use super::{ExitKind, LoopDriver, ScriptedShioriBackend, SpineHarness};
use super::{Injection, LapDriver, RecordedCall, StagePlan, StageSink, WaitInjection};
use areka_kanade::CloseReason;

/// 受信端の開閉の探りは交信の列を **1 件も**増やさず、終了段の自己終了はその探りだけで観測できる
/// （design D1 の終了段・`Error Handling`「解放の後も kanade の送信端が閉じない」）。
///
/// 檻に入れる判断分岐:
/// - **探りが無害であること**: kanade が生きている間に探りを 12 本投函しても、記録された照会・通知は
///   **1 件も増えない**。毎秒の変化通知で代用すると 1 本ごとに 1 件増えるので、ここが両者を分ける。
/// - **探りが終了を捉えること**: 終了指示 → close 握手 → 解放 → kanade の自己終了、まで進むと探りの
///   投函が `Err` になり、完了条件が段の中で成立する。探りが無ければこの段は必ず有界待ち切れになる。
/// - **握手が正典どおり 1 度ずつであること**: `OnClose` の照会 1 件・解放 1 件（R3.9）。
/// - **boot 中に届いた終了指示も必ず果たされること**: 終了指示は kanade が boot に居るあいだに
///   届いても失われず、起動記録トークの再生完了で握手が始まる。しかも**毎秒の変化通知は 1 件も
///   増えない**（`OnSecondChange` 0 件）——増えていたら保留の消化経路が変わっている。
///
/// ERROR の件数は檻に入れない。kanade は別スレッドで走り、本ハーネスのログ捕捉はスレッドを跨がない
/// ——「0 件」を主張しても恒真になりうる（steering: ログ檻の盲点）。探りが副作用を持たないことの
/// 根拠は `crates/areka-kanade/src/schedule/mod.rs:425-431` の防御アームが `warn!` を出して
/// `(state, Vec::new())`（副作用指示 0 件）を返すことであり、上の (i) がその帰結を観測している。
///
/// # 終了段の待ちが「据え置き時刻の再生側 Tick」でなければならない理由（task 5.5・R2.10）
///
/// **前提（構造で決まっている）**——本ハーネスの起動は**必ず**起動記録トークを起こす。fixture の
/// 永続状態は起動のたびに消され（`crates/areka/src/emo2_boot/spine.rs:490-499`）、初回起動と判定
/// された boot は起動記録の書込 cue を `first_boot_epilogue` へ据える
/// （`crates/areka-ghost/src/runtime.rs:459-464`）。ゆえに `OnBoot` が 204 でも kanade は「空
/// script＋末尾 SET 1 件」の記録トークを起こし（`crates/areka-kanade/src/schedule/boot.rs:253-273`）、
/// `BootVersion{talk: Some}` を経て `Steady{talk: Some}` へ入る（同 `:276-280`・`:105-113`）。
/// この前提は下の (ii-a) で**進行状態の記録から逐語に固定する**（`basewareversion` の
/// 組み立て済み進行状態が `talking`＝送出時点の phase が `BootVersion{talk: Some}`
/// ＝`crates/areka-kanade/src/schedule/mod.rs:449-460`）。
///
/// **どの交錯順でも握手が 1 度だけ始まる**——終了指示の到着時点で kanade が居られる場所は 3 つ
/// しかない。
///
/// - **⑴ `Steady{talk: None}`（記録トークが終わっている）**: その場で `OnClose` の照会を発行する
///   （`crates/areka-kanade/src/schedule/steady.rs:866-869`）。再生側 Tick は 1 本も要らない。
/// - **⑵ `Steady{talk: Some}`（記録トーク再生中）**: 保留に入るだけ（同 `:870-874`）。保留を消化
///   するのは**そのトークの再生完了通知**であり（同 `:840-855`・消化は `:847-849`）、再生を進める
///   のは再生側 Tick だけである。ゆえに何も注入しない待ちでは握手が永久に始まらない。
/// - **⑶ boot 系列のいずれか**: 保留記録のみで boot は継続する（`schedule/boot.rs:31`・実体は
///   `:285-288`）。boot が完了すると `Steady{talk}` へ入る（同 `:105-113`）ので、以後は ⑵ に合流
///   する（本ハーネスでは `talk` は必ず `Some` である＝上の前提）。
///
/// いずれの場合も `OnSecondChange` は 1 件も出ない。再生側 Tick は dispatcher の受信端へ入り
/// SHIORI 呼出を 1 件も起こさず（[`Injection::DispatcherTick`]）、kanade Tick は 1 本も投函しない
/// ——`Steady{talk: Some}` へ kanade Tick を投げれば毎秒の変化通知が 1 件ずつ増える
/// （`schedule/steady.rs:705-714`）。台本に `OnSecondChange` の応答を 1 件も積んでいないので、
/// 万一発行されればその場で受け口が落ちる（`spine.rs:245`／`:267`）＝0 件の主張は恒真ではない。
/// 握手が始まった後に遅れて届く再生側 Tick も無害である（dispatcher 側であり kanade へ渡らない）。
///
/// **なぜ注入時刻を据え置くのか**——駆動器を通すと 1 反復ごとに注入時刻が進み、この段の区間
/// （1,000ms÷刻み 1,000ms）では注入がミリ秒未満で頭打ちに達する。頭打ちの後は注入されないので
/// 再生が凍り、待ちは必ず期限切れになる（[`StageSink::may_advance_clock`] の doc が同じ実測を
/// 持つ）。据え置いた注入時刻は待っている観測を追い越しようがなく、予算も減らない——実測では
/// 握手の開始までに 1,621 反復を要した（据え置き・静かな機械・task 5.5）。
///
/// **塞がれた危険（`areka-P0-kanade-boot-talkdone-drop` が受理経路を入れた）**——記録トークの
/// 再生完了通知が kanade の `BootVersion{talk: Some}` 滞在中に届くと、以前は `schedule/boot.rs` の
/// `step` のワイルドカード腕が**それを捨て**、以後 `Steady{talk: Some}` のまま保留が消化されな
/// かった（`schedule/mod.rs` の `current_talk_id` が `BootVersion{Some}` を突合対象に含めておき
/// ながら委譲先が捨てる形）。本仕様で同 `step` にガード腕を足し、通知を `on_talk_done` が受理して
/// トーク枠を空にし `BootVersion{talk: None}` に留まるようにした。
/// ただし本檻はアクターシェル経由であり、この相で通知を受け取る経路に構造上到達しない（`actor.rs`
/// の `drive` が `basewareversion` の往復を同じ呼出の中で済ませ `Steady` まで進み切る）。ゆえに
/// 本檻はこの修正の非回帰の検出器であって欠陥の検出器ではない——欠陥そのものを赤にするのは
/// `areka-kanade` の `boot_version_talkdone_*` テスト（`step` を直接駆動する）である。
#[test]
fn kanade_probe_raises_no_shiori_call_and_observes_the_close() {
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        // 起動挨拶を持たせない（204）。持たせると close 指示が active talk 中に届いた場合に保留され、
        // 消化に毎秒の変化通知が要る＝本檻が測りたい経路と別の非決定が混ざる。
        .get("OnBoot", Ok(None))
        .notify("basewareversion", Ok(()))
        // 終了段の応答は実物に合わせて `\-`（ゴースト終了）で終える。運行側は別れの台詞を
        // 末尾に `\-` が在るのと同じ結果として扱うので、`\e` で終えても終了へ進む
        // （`crates/areka-kanade/src/schedule/close.rs` の `fn on_close_talk_wait`）。
        .get("OnClose", Ok(Some(r"\0\s[0]またね。\-".to_string())))
        .unload(Ok(ExitKind::Clean))
        .build();
    let mut harness = SpineHarness::boot_with(backend, handle.clone(), LoopDriver::Inert);
    assert!(
        spin_wait_until(|| handle.non_status_calls().len() >= 5),
        "起動系列 5 呼出が有界内に揃わない"
    );

    let mut driver = LapDriver::new();

    // ── (i) kanade が生きている間、探りは記録を 1 件も増やさない ──
    let before = handle.non_status_calls();
    let probing = stage("探りの無害性", 1_000, 6_000);
    let observed = driver
        .run_stage(
            &mut harness,
            &StagePlan {
                stage: &probing,
                once: vec![],
                waiting: WaitInjection::DispatcherTickAndKanadeProbe,
            },
            |progress| progress.kanade_probes >= 12,
        )
        .expect("探りは 12 本投げれば足り、待ち切れにはならない");
    assert!(observed.kanade_probes >= 12, "探りが投函されていない");
    assert!(
        !observed.closed.kanade,
        "kanade は生きているのに閉じたと観測している"
    );
    assert_eq!(
        handle.non_status_calls(),
        before,
        "探り {} 本で交信の記録が増えた（探りが SHIORI 呼出を起こしている）",
        observed.kanade_probes
    );

    // ── (ii-a) 終了指示 → 握手の開始。**駆動器を通さず**、注入時刻を据え置いた再生側 Tick を
    //     `OnClose` の照会が記録されるまで有界に繰り返す（理由の全体は本檻の doc コメント）。
    //
    //     駆動器の [`WaitInjection`] には足さない——`spine_conformance_support.rs:231-232` が
    //     「封じているのは駆動器が持つ経路であって、[`StageSink`] を直に呼ぶ経路までは縛らない」
    //     と明記している。据え置きゆえ注入の予算を 1 本も食わず、注入時刻が観測を追い越さない。
    const CLOSE_HOLD_MS: u64 = 11_000;

    // 待ちの形が寄りかかっている前提を、進行状態の記録から逐語に固定する。5 呼出目
    // （`basewareversion`）の組み立て済み進行状態が `talking` であることは、送出時点の phase が
    // `BootVersion{talk: Some}`＝**起動記録トークが走っている**ことを意味する
    // （`crates/areka-kanade/src/schedule/boot.rs:276-280`＋`schedule/mod.rs:449-460`）。
    let boot_status = handle.status_calls();
    assert_eq!(
        boot_status
            .get(4)
            .map(|record| (record.id.as_str(), record.status.as_deref())),
        Some(("basewareversion", Some("talking"))),
        "起動が記録トークを伴っていない——保留 close を消化する経路（再生完了通知）が変わっている: {boot_status:?}"
    );

    harness
        .inject(
            &Injection::CloseRequest(CloseReason::User { scope: 0 }),
            CLOSE_HOLD_MS,
        )
        .expect("kanade の受信端は生きている（直前の段で探りが 12 本通っている）");
    let probe = handle.clone();
    let mut dispatcher_closed = false;
    let began = spin_wait_until(|| {
        // 据え置いた注入時刻の再生側 Tick を毎反復 1 本。SHIORI 呼出は 1 件も起こさない。
        if harness
            .inject(&Injection::DispatcherTick, CLOSE_HOLD_MS)
            .is_err()
        {
            dispatcher_closed = true;
        }
        probe
            .non_status_calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"))
    });
    assert!(
        began,
        "終了指示のあと OnClose の照会が有界内に現れない（dispatcher の受信端が閉じたか: {dispatcher_closed}）: {:?}",
        handle.non_status_calls()
    );

    // ── (ii-b) 終了挨拶の再生 → 解放 → kanade の自己終了を探りが捉える ──
    //
    //     握手が始まってから再生を進める。区間を広く取るのは、注入の予算が反復数の予算でもあり
    //     （1 反復 200µs）、実 async の着地に十分な反復を与えるためである。
    let closing = stage("終了", 21_000, 71_000);
    let closed = driver
        .run_stage(
            &mut harness,
            &StagePlan {
                stage: &closing,
                once: vec![],
                waiting: WaitInjection::DispatcherTickAndKanadeProbe,
            },
            |progress| progress.closed.kanade,
        )
        .expect("終了挨拶の再生 → 解放 → 自己終了が探りで観測できる");
    assert!(
        closed.closed.kanade,
        "kanade の自己終了が完了条件へ渡っていない"
    );

    let calls = handle.non_status_calls();
    assert_eq!(
        calls
            .iter()
            .filter(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"))
            .count(),
        1,
        "OnClose の照会がちょうど 1 件でない: {calls:?}"
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| matches!(c, RecordedCall::Unload))
            .count(),
        1,
        "解放がちょうど 1 度でない（R3.9）: {calls:?}"
    );
    assert_eq!(
        second_change_calls(&handle),
        0,
        "終了段の待ちが毎秒の変化通知を起こしている（kanade Tick が混ざったか、保留の消化経路が変わった）: {calls:?}"
    );

    harness.shutdown_bounded();
}

/// boot 系列に居るあいだに届いた終了指示が、起動記録トークの再生完了で**必ず**果たされる
/// （task 5.5・R2.10・R9.1）。上の檻が高負荷で稀に踏んでいた交錯順を、眠りも壁時計も使わず
/// **構造で**再現する決定論の檻である。
///
/// # 交錯順を決定論にする仕掛け
///
/// 台本受け口は照会・片道のどちらでも**最初に進行状態の記録へ書く**（`spine.rs:233`／`:255` が
/// 呼ぶ `record_status`・本体は `spine_conformance_support.rs:50-59`）。ゆえにその台帳の錠を
/// テスト側が握っていれば受け口は次の呼出の入口で止まり、**応答が kanade へ帰らない**。起動の前に
/// 錠を握り、握ったまま終了指示を投函すれば、kanade は必ず boot 系列に居るあいだにそれを受け取る
/// （保留記録＝`crates/areka-kanade/src/schedule/boot.rs:31`・実体は `:285-288`）。錠を放すと起動が
/// 進み、`Steady{talk: Some}`（起動記録トーク）へ保留を抱えたまま入る——上の檻が稀に踏んでいた形と
/// 同じ状態である。台本の応答は 1 件も差し替えていない（遅らせているのは**返る時点**だけ）。
///
/// **`calls` の側の錠を握ってはならない。** 死活問い合わせの記録も同じ錠を取るため
/// （`spine.rs:281-285`）、受け口は起動系列へ入る前に止まり、起動そのものが進まなくなる（実測: 錠を
/// 握ったまま 30 秒経っても `basewareversion` が記録されない）。
///
/// 檻に入れる判断分岐:
/// - **boot 中の終了指示が失われないこと**: `OnClose` の照会がちょうど 1 件・解放がちょうど 1 度。
/// - **消化に kanade Tick が要らないこと**: `OnSecondChange` は 0 件。台本に応答を 1 件も積んで
///   いないため、発行されればその場で受け口が落ちる（`spine.rs:245`／`:267`）＝0 件の主張は恒真
///   ではない。
#[test]
fn close_request_that_lands_during_boot_is_honored_without_any_second_change() {
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(None))
        .notify("basewareversion", Ok(()))
        .get("OnClose", Ok(Some(r"\0\s[0]またね。\-".to_string())))
        .unload(Ok(ExitKind::Clean))
        .build();

    // 起動系列の応答を凍結してから起動する。
    let frozen = handle
        .status_calls
        .lock()
        .expect("status ledger mutex poisoned");
    let mut harness = SpineHarness::boot_with(backend, handle.clone(), LoopDriver::Inert);
    harness
        .inject(
            &Injection::CloseRequest(CloseReason::User { scope: 0 }),
            11_000,
        )
        .expect("起動直後の kanade の受信端は開いている");
    assert!(
        frozen.is_empty(),
        "凍結より先に起動系列が進んでいる（決定論の前提が崩れている）: {frozen:?}"
    );
    drop(frozen);

    // 上の檻の (ii-a) と同じ待ち（据え置き時刻の再生側 Tick）。
    let probe = handle.clone();
    let began = spin_wait_until(|| {
        let _ = harness.inject(&Injection::DispatcherTick, 11_000);
        probe
            .non_status_calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"))
    });
    assert!(
        began,
        "boot 中に届いた終了指示が握手を始めていない: {:?}",
        handle.non_status_calls()
    );

    // 終了挨拶の再生 → 解放 → 自己終了（区間は上の檻の終了段と同じ）。
    let mut driver = LapDriver::new();
    let closing = stage("終了", 21_000, 71_000);
    driver
        .run_stage(
            &mut harness,
            &StagePlan {
                stage: &closing,
                once: vec![],
                waiting: WaitInjection::DispatcherTickAndKanadeProbe,
            },
            |progress| progress.closed.kanade,
        )
        .expect("終了挨拶の再生 → 解放 → 自己終了が探りで観測できる");

    let calls = handle.non_status_calls();
    assert_eq!(
        calls
            .iter()
            .filter(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"))
            .count(),
        1,
        "OnClose の照会がちょうど 1 件でない: {calls:?}"
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| matches!(c, RecordedCall::Unload))
            .count(),
        1,
        "解放がちょうど 1 度でない（R3.9）: {calls:?}"
    );
    assert_eq!(
        second_change_calls(&handle),
        0,
        "boot 中の保留 close の消化に毎秒の変化通知が混ざっている: {calls:?}"
    );

    harness.shutdown_bounded();
}
