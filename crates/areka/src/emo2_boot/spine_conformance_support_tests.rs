//! 段の駆動器（design D6）の受け入れ確認（areka-P0-emo2-conformance-e2e・task 2.3・R2.10・R3.2）。
//!
//! `spine_conformance_support.rs` は 1 ファイル 1,000 行の分量規律（R2.11・
//! `.kiro/steering/structure.md:176`）に収まらないため、**主題単位に分けて接続する**——駆動器と
//! 進行状態の記録の本体は支援層に残し、駆動器を縛る檻だけを本ファイルへ分けた。接続宣言は支援層の
//! 末尾に置く（`spine.rs` は design が接続宣言 3 本に固定しており余白も 44 行しかないため、
//! 経路をそちらへ増やさない）。
//!
//! 本ファイルは**期待値を持たない**。段の区間の逐語は `spine_conformance_script.rs` の領分であり、
//! 檻は自分が使う段を自前で宣言する（借りると、台本の宣言を変えたときに檻が黙って追随する）。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::{
    ClosedInboxes, Inbox, Injection, LapDriver, LapStage, PresentCommand, RecordedCall,
    StageFailure, StagePlan, StageSink, WaitInjection, injection_kind,
};
use super::{ExitKind, LoopDriver, ScriptedShioriBackend, ScriptedShioriHandle, SpineHarness};
use super::{shell_target, spin_wait_until};
use areka_kanade::{ChoiceInput, CloseReason, MouseEventKind, MouseInput};

// 段の駆動器の受け入れ確認（task 2.3・R2.10・R3.2）
// ===========================================================================

/// 檻用の段宣言を組む（区間の逐語は `spine_conformance_script.rs` の領分ゆえ、檻は自前で宣言する）。
fn stage(name: &'static str, begin_ms: u64, limit_ms: u64) -> LapStage {
    LapStage {
        name,
        begin_ms,
        limit_ms,
    }
}

/// 1 回読むごとに `step` だけ進む檻用の時計（期限切れの経路を数反復で踏ませる）。
fn stepping_clock(step: Duration) -> impl FnMut() -> Instant {
    let start = Instant::now();
    let mut reads = 0u32;
    move || {
        let at = start + step * reads;
        reads += 1;
        at
    }
}

/// 注入を記録するだけの投函先（起動を伴わずに段の駆動そのものを検査する）。
struct FakeStageSink {
    /// 投函の記録（種別名と注入時刻の対・投函順）。
    injected: Vec<(&'static str, u64)>,
    /// 1 反復ぶんの払い出し件数（先頭から 1 反復 1 件消費・尽きたら 0 件）。
    handout: VecDeque<usize>,
    /// 閉じている（投函が `Err` を返す）と見なす受信端。
    closed_inbox: Option<Inbox>,
}

impl FakeStageSink {
    /// 表示指令を 1 件も払い出さない投函先。
    fn silent() -> Self {
        FakeStageSink {
            injected: Vec::new(),
            handout: VecDeque::new(),
            closed_inbox: None,
        }
    }

    /// 反復ごとの払い出し件数を先に決めた投函先。
    fn handing_out(counts: &[usize]) -> Self {
        FakeStageSink {
            handout: counts.iter().copied().collect(),
            ..FakeStageSink::silent()
        }
    }

    /// 指定の受信端が既に閉じている投函先。
    fn with_closed(inbox: Inbox) -> Self {
        FakeStageSink {
            closed_inbox: Some(inbox),
            ..FakeStageSink::silent()
        }
    }
}

impl StageSink for FakeStageSink {
    fn inject(&mut self, injection: &Injection, now_ms: u64) -> Result<(), Inbox> {
        self.injected.push((injection_kind(injection), now_ms));
        match self.closed_inbox {
            Some(inbox) => Err(inbox),
            None => Ok(()),
        }
    }

    fn collect(&mut self) -> Vec<PresentCommand> {
        let count = self.handout.pop_front().unwrap_or(0);
        (0..count)
            .map(|_| PresentCommand::Hide {
                target: shell_target(0),
                reply: None,
            })
            .collect()
    }
}

/// 完了条件が成立しないまま有界時間が尽きたら、**段名を名指しした失敗を呼び手へ返す**（R1.6）。
///
/// 檻に入れる判断分岐:
/// - **素通りさせないこと**: 尽きた結果を「観測なし」の成功として返さない。`Ok` で返ると、後段の
///   照合が「まだ届いていないだけ」を見て別の失敗を名乗り、どの段が不成立だったのかが消える。
/// - **尽きるまでに何をしたかを運ぶこと**: 段名に加えて注入の時刻列と採取件数を持つので、「注入が
///   届いていない」のか「注入は届いたが観測が成立しない」のかを失敗の中身だけで読み分けられる。
#[test]
fn stage_driver_returns_named_timeout_instead_of_silent_success() {
    let stage = stage("自発会話", 1_000, 10_000);
    let plan = StagePlan {
        stage: &stage,
        once: vec![Injection::KanadeTick],
        waiting: WaitInjection::DispatcherTick,
    };
    let mut sink = FakeStageSink::silent();
    let mut driver = LapDriver::new();

    // 1 反復 10 秒進む時計＝3 反復目に SPIN_WAIT（30 秒）へ届く。
    let outcome = driver.run_stage_with(
        stepping_clock(Duration::from_secs(10)),
        &mut sink,
        &plan,
        |_| false,
    );

    let failure = match outcome {
        Ok(observed) => panic!(
            "完了条件が成立しないのに成功が返った（沈黙の失敗）: 段「{}」・採取 {} 件",
            observed.stage,
            observed.collected.len()
        ),
        Err(failure) => failure,
    };
    assert_eq!(
        failure,
        StageFailure::Timeout {
            stage: "自発会話",
            injected_at_ms: vec![1_000, 2_000],
            collected: 0,
            now_ms: 3_000,
        },
        "有界時間が尽きたことが段名つきで呼び手へ返っていない"
    );
    assert!(
        failure.to_string().contains("自発会話"),
        "失敗の文面が段名を名指ししていない: {failure}"
    );
}

/// 注入時刻が段の上限に達したら**以後は注入せず観測だけを待つ**（design D6 の不変条件）。
///
/// 檻に入れる判断分岐:
/// - **頭打ちで注入が止まること**: 投函の時刻列が上限の手前で終わる。止まらないと注入時刻が観測を
///   追い越し、待っている条件そのものが壊れて永久に不成立になる（`SPIN_WAIT` の doc の実測）。
/// - **止まった後も観測は続くこと**: 最後の注入より 3 反復あとに払い出した 1 件が、頭打ち後の注入
///   時刻つきで採取できている。「注入を止める」を「待つのをやめる」と取り違えると採れない。
#[test]
fn stage_driver_stops_injecting_at_the_limit_and_keeps_observing() {
    let stage = stage("撫で", 1_000, 4_000);
    // 7 反復目にだけ 1 件払い出す（注入が止まる 4 反復目より後＝観測が続いている証跡）。
    let mut sink = FakeStageSink::handing_out(&[0, 0, 0, 0, 0, 0, 1]);
    let mut driver = LapDriver::new();

    let observed = driver
        .run_stage(
            &mut sink,
            &StagePlan {
                stage: &stage,
                once: vec![Injection::KanadeTick],
                waiting: WaitInjection::DispatcherTick,
            },
            |progress| !progress.collected.is_empty(),
        )
        .expect("頭打ちの後も観測を続ければ 7 反復目の 1 件が採れる");

    assert_eq!(
        sink.injected,
        vec![
            ("kanade-tick", 1_000),
            ("dispatcher-tick", 2_000),
            ("dispatcher-tick", 3_000),
        ],
        "上限に達した後も注入が続いている（注入時刻が観測を追い越す形）"
    );
    assert_eq!(observed.injected_at_ms, vec![1_000, 2_000, 3_000]);
    assert_eq!(observed.collected.len(), 1, "頭打ちの後の採取が落ちている");
    assert_eq!(
        observed.collected[0].collected_at_ms, 4_000,
        "採取時刻が頭打ち後の注入時刻と一致しない"
    );
    assert!(
        matches!(observed.collected[0].command, PresentCommand::Hide { .. }),
        "採取した表示指令そのものが運ばれていない"
    );
}

/// 注入時刻は走行を通じて単調増加し、各段の上限を超えない（design D6 の不変条件）。
///
/// 檻に入れる判断分岐: ⑴ 2 段ぶんの注入時刻を 1 本に並べて非減少である ⑵ 各段の幅は刻み
/// （1,000ms）の整数倍ではない（1,500ms）ので、上限で止めていなければ注入時刻が段の外へ出て次段の
/// 下限を侵す ⑶ 完了時に未投函の `once` が 0 本（沈黙の失敗を作らない）。
#[test]
fn stage_driver_keeps_injection_time_monotonic_within_each_stage() {
    let idle = stage("自発会話", 1_000, 2_500);
    let hold = stage("会話中の抑止", 11_000, 12_500);
    let mut sink = FakeStageSink::silent();
    let mut driver = LapDriver::new();

    let first = driver
        .run_stage(
            &mut sink,
            &StagePlan {
                stage: &idle,
                once: vec![Injection::KanadeTick],
                waiting: WaitInjection::DispatcherTick,
            },
            |progress| progress.now_ms >= 2_500,
        )
        .expect("上限に達した時点で完了条件が成立する");
    assert_eq!(
        driver.now_ms(),
        2_500,
        "段 1 の注入時刻が上限で止まっていない（次段の下限を侵す）"
    );

    let second = driver
        .run_stage(
            &mut sink,
            &StagePlan {
                stage: &hold,
                once: vec![Injection::KanadeTick],
                waiting: WaitInjection::DispatcherTick,
            },
            |progress| progress.now_ms >= 12_500,
        )
        .expect("上限に達した時点で完了条件が成立する");

    assert_eq!(first.injected_at_ms, vec![1_000, 2_000], "段 1 の注入時刻");
    assert_eq!(
        second.injected_at_ms,
        vec![11_000, 12_000],
        "段 2 の注入時刻"
    );
    assert_eq!(
        (first.once_pending, second.once_pending),
        (0, 0),
        "計画した注入が投函されないまま完了している（沈黙の失敗）"
    );
    assert_eq!(
        sink.injected,
        vec![
            ("kanade-tick", 1_000),
            ("dispatcher-tick", 2_000),
            ("kanade-tick", 11_000),
            ("dispatcher-tick", 12_000),
        ],
        "走行を通じた注入の並びが単調増加でない"
    );
    assert_eq!(
        driver.now_ms(),
        12_500,
        "段 2 の注入時刻が上限で止まっていない"
    );
}

/// 事前条件——段の下限が走行中の注入時刻より手前なら、駆動せずに呼び手へ返す。檻に入れる判断分岐:
/// 段の順序を取り違えたときに**時刻を巻き戻して駆動しない**（巻き戻すと表示の台帳の採取時刻が
/// 前段と重なり、段の切れ目が意味を失う）。
#[test]
fn stage_driver_refuses_a_stage_that_starts_before_the_running_clock() {
    let first = stage("自発会話", 1_000, 3_000);
    let backwards = stage("撫で", 2_000, 5_000);
    let mut sink = FakeStageSink::silent();
    let mut driver = LapDriver::new();

    driver
        .run_stage(
            &mut sink,
            &StagePlan {
                stage: &first,
                once: vec![],
                waiting: WaitInjection::DispatcherTick,
            },
            |progress| progress.now_ms >= 3_000,
        )
        .expect("段 1 は上限に達して完了する");

    let outcome = driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: &backwards,
            once: vec![],
            waiting: WaitInjection::DispatcherTick,
        },
        |_| true,
    );

    assert_eq!(
        outcome.err(),
        Some(StageFailure::StartsBeforeStage {
            stage: "撫で",
            arrived_at_ms: 3_000,
            begin_ms: 2_000,
        }),
        "前段の上限より手前から始まる段を、時刻を巻き戻して駆動している"
    );
}

/// 計画した注入が段の区間に収まらないなら、駆動せずに呼び手へ返す。檻に入れる判断分岐: 頭打ちは
/// **注入を黙って落とす**形にしうる——区間が 2 本しか収容できないのに 3 本計画したら、3 本目は
/// 永久に投函されないまま完了条件だけを待つ。着手前に断る。
#[test]
fn stage_driver_refuses_a_plan_that_cannot_fit_in_the_stage_interval() {
    let narrow = stage("メニュー", 1_000, 3_000);
    let mut sink = FakeStageSink::silent();
    let mut driver = LapDriver::new();

    let outcome = driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: &narrow,
            // 一周の実際の段が投函する 3 種（撫で・選択確定・終了指示）を 3 本並べる。
            once: vec![
                Injection::Mouse(MouseInput {
                    scope: 0,
                    x: 1,
                    y: 2,
                    region: Some("Head".to_string()),
                    kind: MouseEventKind::Move,
                }),
                Injection::Choice(ChoiceInput {
                    id: "Onメインメニュー".to_string(),
                    label: "もどる".to_string(),
                    scope: 0,
                    references: vec![],
                }),
                Injection::CloseRequest(CloseReason::User),
            ],
            waiting: WaitInjection::Idle,
        },
        |_| false,
    );

    assert_eq!(
        outcome.err(),
        Some(StageFailure::PlanExceedsInterval {
            stage: "メニュー",
            planned: 3,
            capacity: 2,
        }),
        "区間に収まらない計画を受け付けている（投函されない注入が黙って落ちる）"
    );
    assert!(sink.injected.is_empty(), "断る前に注入してしまっている");
}

/// 採取時の注入時刻が段の宣言区間の外なら、**製品の退行と区別できる形**で失敗する（design D3）。
///
/// 檻に入れる判断分岐: ⑴ 区間が逆転した段宣言（下限 > 上限）で駆動すると最初の採取で検査が落ちる
/// ——落ちなければ採取が黙って続き、後段の段名の食い違いとして**製品の退行に化ける** ⑵ 失敗が
/// 有界待ち切れとは別の値であり、文面が「駆動が壊れている」と言う。
#[test]
fn stage_driver_self_check_names_a_collection_outside_the_declared_interval() {
    // 区間の逆転（下限 5,000 > 上限 1,000）＝段の宣言そのものが壊れている状態。
    let inverted = stage("位置調整", 5_000, 1_000);
    let mut sink = FakeStageSink::handing_out(&[1]);
    let mut driver = LapDriver::new();

    let outcome = driver.run_stage(
        &mut sink,
        &StagePlan {
            stage: &inverted,
            once: vec![],
            waiting: WaitInjection::Idle,
        },
        |_| false,
    );

    let failure = outcome
        .err()
        .expect("区間外の採取は自己検査で失敗するはず（成功が返るなら検査が死んでいる）");
    assert_eq!(
        failure,
        StageFailure::CollectedOutsideInterval {
            stage: "位置調整",
            collected_at_ms: 5_000,
            begin_ms: 5_000,
            limit_ms: 1_000,
        },
        "駆動器の自己検査が採取時刻の区間外を捉えていない"
    );
    assert!(
        failure.to_string().contains("駆動が壊れている"),
        "自己検査の失敗が製品の退行と読み分けられない文面になっている: {failure}"
    );
}

/// 投函先が既に閉じていたことを握り潰さず、完了条件と成果の両方へ渡す（design D1 の終了段）。
/// 檻に入れる判断分岐: 終了段の完了条件は kanade の自己終了の観測を含む——投函の `Err` を捨てると
/// その観測点が消え、代わりに panic させると段名が消える。どちらでもない形で運ぶ。
#[test]
fn stage_driver_reports_a_closed_inbox_without_swallowing_or_panicking() {
    let closing = stage("終了", 1_000, 3_000);
    let mut sink = FakeStageSink::with_closed(Inbox::Kanade);
    let mut driver = LapDriver::new();

    let observed = driver
        .run_stage(
            &mut sink,
            &StagePlan {
                stage: &closing,
                once: vec![Injection::CloseRequest(CloseReason::User)],
                waiting: WaitInjection::Idle,
            },
            |progress| progress.closed.kanade,
        )
        .expect("受信端が閉じていることは完了条件から読めるはず");

    assert_eq!(
        observed.closed,
        ClosedInboxes {
            kanade: true,
            dispatcher: false,
        },
        "閉じていた受信端が成果へ運ばれていない"
    );
    assert_eq!(sink.injected, vec![("close-request", 1_000)]);
}

/// 毎秒の変化通知は **kanade の送信端へ直接**投函され、待ちの繰り返しは 1 件も増やさない（R3.2）。
///
/// 檻に入れる判断分岐:
/// - **投函先が kanade であり本数が 1 対 1 であること**: kanade Tick を 1 本投函すると
///   `OnSecondChange` がちょうど 1 件現れる（`schedule/steady.rs:669-718`）。dispatcher 側へ投げて
///   いたら 0 件のままである（`spine.rs` の標準台本が `OnSecondChange` を持たないのはこのため）。
/// - **待ちが増やさないこと**: 完了条件が成立しない段を有界待ち切れまで回しても件数が 1 のまま
///   ＝繰り返す注入（再生側）が SHIORI 呼出を 1 件も起こしていない。交信の列は等値照合ゆえ
///   （R2.3）、待ちの最中に 1 本でも増えると期待列が満たせなくなる。
#[test]
fn kanade_tick_raises_one_second_change_where_the_waiting_injection_raises_none() {
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        .get("OnBoot", Ok(Some(r"\s[0]\e".to_string())))
        .notify("basewareversion", Ok(()))
        .get("OnSecondChange", Ok(None))
        .get("OnSecondChange", Ok(None))
        .notify("OnSecondChange", Ok(()))
        .notify("OnSecondChange", Ok(()))
        .notify("OnClose", Ok(()))
        .unload(Ok(ExitKind::Clean))
        .build();
    let mut harness = SpineHarness::boot_with(backend, handle.clone(), LoopDriver::Inert);

    // 起動系列 5 呼出が揃うまで待つ（既存の手本と同じ「条件が満たされるまで待つ」形）。
    assert!(
        spin_wait_until(|| handle.non_status_calls().len() >= 5),
        "起動系列 5 呼出が有界内に揃わない"
    );
    assert_eq!(
        second_change_calls(&handle),
        0,
        "起動だけで毎秒の変化通知が発行されている（標準台本の前提が崩れている）"
    );

    let mut driver = LapDriver::new();

    // ── (1) kanade Tick 1 本 → OnSecondChange がちょうど 1 件 ──
    let idle = stage("自発会話", 1_000, 6_000);
    let probe = handle.clone();
    let observed = driver.run_stage(
        &mut harness,
        &StagePlan {
            stage: &idle,
            once: vec![Injection::KanadeTick],
            waiting: WaitInjection::DispatcherTick,
        },
        |_| second_change_calls(&probe) >= 1,
    );
    assert!(
        observed.is_ok(),
        "kanade Tick 1 本で毎秒の変化通知が有界内に届かない: {:?}",
        observed.err().map(|f| f.to_string())
    );
    assert_eq!(
        second_change_calls(&handle),
        1,
        "kanade Tick 1 本に対し毎秒の変化通知が 1 件でない: {:?}",
        handle.non_status_calls()
    );

    // ── (2) 待ちの繰り返し（再生側 Tick）は毎秒の変化通知を 1 件も起こさない ──
    let hold = stage("会話中の抑止", 11_000, 16_000);
    let timed_out = driver
        .run_stage_with(
            stepping_clock(Duration::from_secs(4)),
            &mut harness,
            &StagePlan {
                stage: &hold,
                once: vec![],
                waiting: WaitInjection::DispatcherTick,
            },
            |_| false,
        )
        .err();
    assert!(
        matches!(
            timed_out,
            Some(StageFailure::Timeout {
                stage: "会話中の抑止",
                ..
            })
        ),
        "待ちの繰り返しだけの段が段名つきの待ち切れで返っていない: {timed_out:?}"
    );
    assert_eq!(
        second_change_calls(&handle),
        1,
        "待ちの繰り返しが毎秒の変化通知を増やしている（等値照合が成立しなくなる）: {:?}",
        handle.non_status_calls()
    );

    harness.shutdown_bounded();
}

/// 記録の中の `OnSecondChange`（照会・片道の別を問わない）の件数。
fn second_change_calls(handle: &ScriptedShioriHandle) -> usize {
    handle
        .non_status_calls()
        .iter()
        .filter(|call| match call {
            RecordedCall::Get { id, .. } | RecordedCall::Notify { id, .. } => {
                id == "OnSecondChange"
            }
            _ => false,
        })
        .count()
}

/// 受信端の開閉の探りは交信の列を **1 件も**増やさず、終了段の自己終了はその探りだけで観測できる
/// （design D1 の終了段・`Error Handling`「解放の後も kanade の送信端が閉じない」）。
///
/// 檻に入れる判断分岐:
/// - **探りが無害であること**: kanade が生きている間に探りを 12 本投函しても、記録された照会・通知は
///   **1 件も増えない**。毎秒の変化通知で代用すると 1 本ごとに 1 件増えるので、ここが両者を分ける。
/// - **探りが終了を捉えること**: 終了指示 → close 握手 → 解放 → kanade の自己終了、まで進むと探りの
///   投函が `Err` になり、完了条件が段の中で成立する。探りが無ければこの段は必ず有界待ち切れになる。
/// - **握手が正典どおり 1 度ずつであること**: `OnClose` の照会 1 件・解放 1 件（R3.9）。
///
/// ERROR の件数は檻に入れない。kanade は別スレッドで走り、本ハーネスのログ捕捉はスレッドを跨がない
/// ——「0 件」を主張しても恒真になりうる（steering: ログ檻の盲点）。探りが副作用を持たないことの
/// 根拠は `crates/areka-kanade/src/schedule/mod.rs:425-431` の防御アームが `warn!` を出して
/// `(state, Vec::new())`（副作用指示 0 件）を返すことであり、上の (i) がその帰結を観測している。
#[test]
fn kanade_probe_raises_no_shiori_call_and_observes_the_close() {
    let (backend, handle) = ScriptedShioriBackend::builder()
        .notify("OnInitialize", Ok(()))
        .get("OnFirstBoot", Ok(None))
        // 起動挨拶を持たせない（204）。持たせると close 指示が active talk 中に届いた場合に保留され、
        // 消化に毎秒の変化通知が要る＝本檻が測りたい経路と別の非決定が混ざる。
        .get("OnBoot", Ok(None))
        .notify("basewareversion", Ok(()))
        // 終了段の応答は `\-`（ゴースト終了）で終える。`\e` で終えると「終了拒否」として定常運転へ
        // 戻り解放が起きない（tasks.md Implementation Notes・`schedule/close.rs:15-17`）。
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

    // ── (ii-a) 終了指示 → 握手の開始。**再生側 Tick を 1 本も使わない**（`waiting: Idle`）。
    //
    //     active talk が無い Steady への close 指示は、その場で `OnClose` 照会を発行する
    //     （`crates/areka-kanade/src/schedule/steady.rs:864-899`）。ゆえにこの段の完了条件は
    //     注入時刻に一切依存せず、純粋な別スレッド到着の待ちになる——注入の予算（＝段の区間が
    //     収容する本数）が実時間で先に尽きる、という競争がここには構造的に存在しない。
    let close_begin = stage("終了指示", 11_000, 12_000);
    let probe = handle.clone();
    driver
        .run_stage(
            &mut harness,
            &StagePlan {
                stage: &close_begin,
                once: vec![Injection::CloseRequest(CloseReason::User)],
                waiting: WaitInjection::Idle,
            },
            |_| {
                probe
                    .non_status_calls()
                    .iter()
                    .any(|c| matches!(c, RecordedCall::Get { id, .. } if id == "OnClose"))
            },
        )
        .expect("close 指示はその場で OnClose の照会を発行する（再生側 Tick を要さない）");

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

    harness.shutdown_bounded();
}
