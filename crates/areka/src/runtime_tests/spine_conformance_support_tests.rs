//! 段の駆動器の受け入れ確認。
//!
//! `spine_conformance_support.rs` は 1 ファイル 1,000 行の分量規律に収まらないため、
//! **主題単位に分けて接続する**——駆動器は
//! 支援層に残し、駆動器を縛る檻と進行状態の記録の檻を本ファイルへ分けた（進行状態の記録の取り出し口の本体は `areka-runtime` の `spine`）。
//! 檻は支援層の子として置く。
//!
//! 本ファイルは**台本の期待値を持たない**。段の区間の逐語は `spine_conformance_script.rs` の領分であり、
//! 檻は自分が使う段を自前で宣言する（借りると、台本の宣言を変えたときに檻が黙って追随する）。

use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::super::settle_bounded;
use super::super::test_support::boot_finished;
use super::super::{ExitKind, LoopDriver, ScriptedShioriHandle, shell_target, wait_until};
use super::{
    ClosedInboxes, Inbox, Injection, LapDriver, LapStage, PresentCommand, RecordedCall,
    StageFailure, StagePlan, StageSink, WaitFailure, WaitInjection,
};
use super::{Progress, RecordedStatus, ScriptedShioriBackend, ShioriBackend, SpineHarness};
use areka_kanade::{
    ChoiceInput, CloseReason, ExecutionSnapshot, MouseEventKind, MouseInput, ShioriCall,
};

/// 檻の偽物が注入を記録するときの種別名。
fn injection_kind(injection: &Injection) -> &'static str {
    match injection {
        Injection::KanadeTick => "kanade-tick",
        Injection::DispatcherTick => "dispatcher-tick",
        Injection::Mouse(_) => "mouse",
        Injection::Choice(_) => "choice",
        Injection::CloseRequest(_) => "close-request",
        Injection::KanadeProbe => "kanade-probe",
    }
}

// 段の駆動器の受け入れ確認
// ===========================================================================

/// 檻用の段宣言を組む（区間の逐語は `spine_conformance_script.rs` の領分ゆえ、檻は自前で宣言する）。
pub(super) fn stage(name: &'static str, begin_ms: u64, limit_ms: u64) -> LapStage {
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
    /// 受けた注入の数（進みの目印）。`None` は目印を持たない（`［進みは不明］` の形を確かめる檻だけ）。
    injections: Option<Rc<Cell<u64>>>,
}

impl FakeStageSink {
    /// 表示指令を 1 件も払い出さない投函先。
    fn silent() -> Self {
        FakeStageSink {
            injected: Vec::new(),
            handout: VecDeque::new(),
            closed_inbox: None,
            injections: Some(Rc::default()),
        }
    }

    /// 目印を持たない投函先。`［進みは不明］` の形そのものを確かめる檻だけが使う（目印を持つと
    /// 打ち切りが `［止まった］` になり、確かめたい形を踏めない・`load-repro.md` の ⑵）。
    fn silent_unmarked() -> Self {
        FakeStageSink {
            injections: None,
            ..FakeStageSink::silent()
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
        if let Some(n) = &self.injections {
            n.set(n.get() + 1);
        }
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

    fn progress_probe(&self) -> Option<Box<dyn Fn() -> u64>> {
        let n = Rc::clone(self.injections.as_ref()?);
        Some(Box::new(move || n.get()))
    }
}

/// 完了条件が成立しないまま有界時間が尽きたら、**段名を名指しした失敗を呼び手へ返す**。
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
    let mut sink = FakeStageSink::silent_unmarked();
    let mut driver = LapDriver::new();

    // 1 回読むごとに 10 秒進む時計。待ちの芯は 1 反復目の後に待ち始めの時刻を読み、以後は反復ごとに
    // 1 回読むので、4 反復目の後に 30 秒（目印の無い待ちの上限）へ届く。
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
            wait: Box::new(WaitFailure::TimedOut {
                what: "自発会話".to_string(),
                waited: Duration::from_secs(30),
            }),
            stage: "自発会話",
            injected_at_ms: vec![1_000, 2_000, 3_000, 4_000],
            collected: 0,
            now_ms: 5_000,
        },
        "有界時間が尽きたことが段名つきで呼び手へ返っていない"
    );
    assert!(
        failure.to_string().contains("自発会話"),
        "失敗の文面が段名を名指ししていない: {failure}"
    );
    assert!(
        failure
            .to_string()
            .starts_with("待ちの打ち切り［進みは不明］"),
        "打ち切りの文言が待ちの芯の見出しで始まっていない: {failure}"
    );
}

/// 注入時刻が段の上限に達したら**以後は注入せず観測だけを待つ**。
///
/// 檻に入れる判断分岐:
/// - **頭打ちで注入が止まること**: 投函の時刻列が上限の手前で終わる。止まらないと注入時刻が観測を
///   追い越し、待っている条件そのものが壊れて永久に不成立になる（`SPIN_WAIT` の doc の実測）。
/// - **止まった後も観測は続くこと**: 頭打ちの後の反復で払い出した 1 件が、頭打ち後の注入
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

/// 注入時刻は走行を通じて単調増加し、各段の上限を超えない。
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
                Injection::CloseRequest(CloseReason::User { scope: 0 }),
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

/// 採取時の注入時刻が段の宣言区間の外なら、**製品の退行と区別できる形**で失敗する。
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

/// 投函先が既に閉じていたことを握り潰さず、完了条件と成果の両方へ渡す。
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
                once: vec![Injection::CloseRequest(CloseReason::User { scope: 0 })],
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

/// 毎秒の変化通知は **kanade の送信端へ直接**投函され、待ちの繰り返しは 1 件も増やさない。
///
/// 檻に入れる判断分岐:
/// - **投函先が kanade であり本数が 1 対 1 であること**: kanade Tick を 1 本投函すると
///   `OnSecondChange` がちょうど 1 件現れる（`schedule/steady.rs`）。dispatcher 側へ投げて
///   いたら 0 件のままである（`spine.rs` の標準台本が `OnSecondChange` を持たないのはこのため）。
/// - **待ちが増やさないこと**: 完了条件が成立しない段を有界待ち切れまで回しても件数が 1 のまま
///   ＝繰り返す注入（再生側）が SHIORI 呼出を 1 件も起こしていない。交信の列は等値照合ゆえ、
///   待ちの最中に 1 本でも増えると期待列が満たせなくなる。
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

    // 起動の系列が最後の basewareversion まで終わるのを待つ（目印は SHIORI の呼び出しの数）。
    let calls = || handle.call_count();
    assert!(
        wait_until(
            "起動の系列（最後は basewareversion）",
            Progress::Count(&calls),
            || { boot_finished(&handle.non_status_calls()) }
        )
        .inspect_err(|failure| eprintln!("{failure}"))
        .is_ok(),
        "起動の系列（最後は basewareversion）が有界内に終わらない"
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
    // 負の検証は回収（settle）で届くのを待ってから読む（待ちが打ち切られた直後の 1 回読みでは、非同期の記録が間に合わない）。
    let mut seen = handle.non_status_calls().len();
    settle_bounded(|| {
        let now = handle.non_status_calls().len();
        let fresh = now.saturating_sub(seen);
        seen = now;
        fresh
    });
    assert_eq!(
        second_change_calls(&handle),
        1,
        "待ちの繰り返しが毎秒の変化通知を増やしている（等値照合が成立しなくなる）: {:?}",
        handle.non_status_calls()
    );

    harness.shutdown_bounded();
}

/// 記録の中の `OnSecondChange`（照会・片道の別を問わない）の件数。
pub(super) fn second_change_calls(handle: &ScriptedShioriHandle) -> usize {
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

// ===========================================================================
// 据え置きの門の受け入れ確認
// ===========================================================================

/// 据え置きの門だけを検査する投函先（起動を伴わずに駆動器の時刻の進め方を見る）。
struct HoldingSink {
    /// 投函の記録（注入時刻・投函順）。
    injected: Vec<u64>,
    /// 残りの据え置き回数（0 になったら注入時刻を進めてよい）。
    hold_rounds: Cell<usize>,
    /// 受けた注入の数（進みの目印）。
    injections: Rc<Cell<u64>>,
}

impl StageSink for HoldingSink {
    fn inject(&mut self, _injection: &Injection, now_ms: u64) -> Result<(), Inbox> {
        self.injected.push(now_ms);
        self.injections.set(self.injections.get() + 1);
        Ok(())
    }

    fn progress_probe(&self) -> Option<Box<dyn Fn() -> u64>> {
        let n = Rc::clone(&self.injections);
        Some(Box::new(move || n.get()))
    }

    fn collect(&mut self) -> Vec<PresentCommand> {
        Vec::new()
    }

    fn may_advance_clock(&self) -> bool {
        let left = self.hold_rounds.get();
        if left > 0 {
            self.hold_rounds.set(left - 1);
            false
        } else {
            true
        }
    }
}

/// 駆動器の 2 つの門——**着地待ちの据え置き**と**余韻での注入停止**——を 1 本の投函列で固定する。
///
/// 檻に入れる判断分岐:
/// - **据え置き中も投函は続くこと**: 再生側 Tick が止まると、待っている会話は永久に起動しない
///   （駆動器が注入をやめると再生が凍る）。
/// - **据え置き中は時刻が 1 ミリ秒も動かないこと**: 動くと予算が減り、着地が遅いと上限へ達して
///   同じ凍り方をする。実測ではこれが高負荷で装着段・選択確定段を赤くしていた。
/// - **門が開いたら刻みどおり進むこと**: 据え置きが解けない実装は再生を進める相へ進めない。
/// - **完了条件が成立した後は 1 本も投函しないこと**: 余韻のあいだ注入が続くと、選択待ちのまま
///   止まっているはずの台本へ再生が流し込まれ、次段の選択確定が棄却される。
///
/// 期待する投函列は「据え置き 5 本（すべて下限）＋前進 4 本（刻みどおり）＋余韻 0 本」である。
/// 区間は期待の列より長く取ってあるので、余韻の間に注入が続けば投函列が期待の列より長くなる。
#[test]
fn stage_driver_holds_the_injection_clock_until_the_landing_is_observed() {
    let stage = LapStage {
        name: "門の検査",
        begin_ms: 1_000,
        limit_ms: 20_000,
    };
    let mut sink = HoldingSink {
        injected: Vec::new(),
        hold_rounds: Cell::new(5),
        injections: Rc::default(),
    };
    let mut driver = LapDriver::new();
    let mut rounds = 0usize;
    let observed = driver
        .run_stage(
            &mut sink,
            &StagePlan {
                stage: &stage,
                once: Vec::new(),
                waiting: WaitInjection::DispatcherTickThenObserve { settle_rounds: 4 },
            },
            |_| {
                rounds += 1;
                rounds > 9
            },
        )
        .expect("門は待ち切れを起こさない");

    assert_eq!(
        observed.injected_at_ms,
        vec![
            1_000, 1_000, 1_000, 1_000, 1_000, 1_000, 2_000, 3_000, 4_000
        ],
        "据え置き 5 本（下限に釘付け）→ 前進 4 本（刻みどおり）→ 余韻 0 本、になっていない"
    );
    assert!(
        observed
            .injected_at_ms
            .iter()
            .all(|at| (stage.begin_ms..=stage.limit_ms).contains(at)),
        "投函が段の区間の外へ出ている: {:?}",
        observed.injected_at_ms
    );
}

// ===========================================================================
// 進行状態の記録（第 2 系統）の受け入れ確認
// ===========================================================================

/// 進行状態の記録が**選択待ちを会話中と区別して**読めることを固定する。
///
/// 檻に入れる判断分岐:
/// - **付随参照だけでは足りないこと**: 会話中のみと会話中かつ選択待ちの 2 つのスナップショットから
///   本番の構築関数が組む毎秒の変化通知は、**参照列が 1 バイトも違わない**（Ref3 の源は
///   `talk_active` だけ）。既存の呼出記録は参照列までしか持たないので、ここで区別が消える。
/// - **新しい取り出し口が区別を回復すること**: 同じ 3 呼出を台本受け口へ通すと、進行状態の記録には
///   組み立て済みのヘッダ値がそのまま残り、選択待ちの回だけが `talking,choosing` として読める。
/// - **ヘッダ行を出さない場合の表し方**: 全状態が非アクティブなら記録は `None`（＝ヘッダ行なし）。
/// - **既存の記録が変わらないこと**: 同じ走行で `non_status_calls()` が従来どおり 3 件を返す。
#[test]
fn status_ledger_reads_choosing_where_references_cannot() {
    // ── 実状態（非アクティブ・会話中・会話中かつ選択待ち。いずれも本番で起こりうる組み合わせのみ） ──
    // 選択待ちの間も会話の枠は占有されたままゆえ、選択待ちは常に talk_active と同時に真になる。
    let idle = ExecutionSnapshot::INACTIVE;
    let talking = ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
        ..ExecutionSnapshot::INACTIVE
    };
    let choosing = ExecutionSnapshot {
        talk_active: true,
        choice_active: true,
        ..ExecutionSnapshot::INACTIVE
    };

    // ── (1) 付随参照は会話中と選択待ちを区別しない（＝記録の第 2 系統が要る理由） ──
    let talking_call = areka_kanade::events::on_second_change(dola::TimePoint::ZERO, &talking);
    let choosing_call = areka_kanade::events::on_second_change(dola::TimePoint::ZERO, &choosing);
    assert_eq!(
        call_references(&talking_call),
        call_references(&choosing_call),
        "会話中と選択待ちで参照列が違うなら Ref3 で区別できてしまい、進行状態の記録は要らない"
    );

    // ── (2) 同じ 3 呼出を台本受け口へ通す ──
    let (mut backend, handle) = ScriptedShioriBackend::builder()
        .get("OnSecondChange", Ok(None))
        .notify("OnSecondChange", Ok(()))
        .notify("OnSecondChange", Ok(()))
        .build();
    let idle_call = areka_kanade::events::on_second_change(dola::TimePoint::ZERO, &idle);
    drive_call(&mut backend, &idle_call);
    drive_call(&mut backend, &talking_call);
    drive_call(&mut backend, &choosing_call);

    // ── (3) 新しい取り出し口から進行状態が読める（選択待ちが会話中と別物として現れる） ──
    assert_eq!(
        handle.status_calls(),
        vec![
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: None,
            },
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: Some("talking".to_string()),
            },
            RecordedStatus {
                id: "OnSecondChange".to_string(),
                status: Some("talking,choosing".to_string()),
            },
        ],
        "進行状態の記録が組み立て済みのヘッダ値（無いときは None）を呼出順に保持していない"
    );

    // ── (4) 既存の呼出記録は素通し（追補は書き込みのみで既存の読み手を増やさない） ──
    let existing = handle.non_status_calls();
    assert_eq!(
        existing.len(),
        3,
        "既存の呼出記録が追補で変質している: {existing:?}"
    );
    assert!(
        matches!(existing[0], RecordedCall::Get { .. })
            && matches!(existing[1], RecordedCall::Notify { .. })
            && matches!(existing[2], RecordedCall::Notify { .. }),
        "既存の呼出記録の別（照会・片道）が従来どおりに残っていない: {existing:?}"
    );
}

/// [`ShioriCall`] の参照列を借りる（照会・片道のどちらでも同じ位置にある）。
fn call_references(call: &ShioriCall) -> &[String] {
    match call {
        ShioriCall::Get { references, .. } | ShioriCall::Notify { references, .. } => references,
    }
}

/// 本番の構築関数が組んだ [`ShioriCall`] を、別と id と参照列と**組み立て済み進行状態**の
/// まま台本受け口へ通す（`crates/areka-kanade/src/shiori/real.rs` と同じ渡し方）。
fn drive_call(backend: &mut ScriptedShioriBackend, call: &ShioriCall) {
    match call {
        ShioriCall::Get {
            id,
            references,
            status,
        } => {
            let wire = status.render();
            backend
                .get(id.as_str(), references, wire.as_deref())
                .expect("台本の照会応答は Ok");
        }
        ShioriCall::Notify {
            id,
            references,
            status,
        } => {
            let wire = status.render();
            backend
                .notify(id.as_str(), references, wire.as_deref())
                .expect("台本の片道応答は Ok");
        }
    }
}
