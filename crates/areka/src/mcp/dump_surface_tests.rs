//! `dump_surface` の決定論テスト（窓の無い World）。
//!
//! 結線状態の無い空の World で呼ぶと、その場で `NG:This ghost has no window`（isError: true・
//! content は本文 1 つ＝画像なし）を返す（要件 4.5・4.8・7.5）。描画を通る場合は GPU のテストが固定する。
//! 後半は窓の無い切り替えの道具（`SwitchRig`・GPU 不要）で、装着の前に預けた呼び出しを通す。

use std::path::PathBuf;

use areka_mcp::ToolContent;
use areka_mcp::tools::{ToolCall, ToolRequest};
use wintf::ecs::Input;
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::super::McpLater;
use super::*;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

#[test]
fn answers_no_window_at_once_with_an_empty_world() {
    let ghost = ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let args = Args {
        scope: Some(0),
        surface: Some(0),
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(args.clone()));

    handle(&mut World::new(), &ghost, args, req.reply);

    let answer = pending.try_answer().ok().flatten().expect("その場で答える");
    assert_eq!(
        answer.outcome.content,
        vec![ToolContent::Text("NG:This ghost has no window".to_string())]
    );
    assert!(answer.outcome.is_error);
}

/// 写しまで済んだ成功は、呼んだスレッドでなく符号化のスレッドで仕上がり、そのスレッドから答える
/// （要件 6.1・タスク 4.3）。仕事は自分の走ったスレッドの名前を答えに書く。
///
/// # 非空虚性
/// 呼んだスレッド（テストのスレッド）で仕上げると、答えの名前がテストの名前になって赤。
/// 答えは上限つきで待つ（届かなければ `None` で赤）。
#[test]
fn success_is_finished_on_the_encoding_thread_and_answered_from_there() {
    let args = Args {
        scope: Some(0),
        surface: None,
        ghost_name: None,
    };
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(args));
    let job: Job = Box::new(|_| outcome::ok(std::thread::current().name().unwrap_or("")));

    reply_elsewhere(TOOL, Step::Encode(0, job), Instant::now(), req.reply);

    assert_eq!(
        pending.wait_answer().map(|a| a.outcome),
        Some(outcome::ok(ENCODE_THREAD))
    );
}

/// `start` は文字の面の読み出し待ちを預けるので `reply_elsewhere` へ `Read` は渡らないが、渡されたら
/// 黙らせず `NG:the text read was handed to the wrong path` と `error!` 1 件で答える（読み出しの口は
/// 呼ばない）。
///
/// # 非空虚性
/// 腕を黙って捨てると答えが届かず `None` で赤。読み出しの口を呼ぶと呼んだ回数が 1 で赤。
#[test]
fn read_handed_to_reply_elsewhere_answers_wrong_path_with_one_error() {
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    let reads = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = reads.clone();
    let read: Reader = Box::new(move || {
        counted.set(counted.get() + 1);
        Ok(None)
    });

    let ((), levels) = log_capture_kit::count_levels(|| {
        reply_elsewhere(TOOL, Step::Read(0, read), Instant::now(), req.reply)
    });

    assert_eq!(
        (
            pending.try_answer().ok().flatten().map(|a| a.outcome),
            levels.error,
            reads.get()
        ),
        (
            Some(outcome::ng("the text read was handed to the wrong path")),
            1,
            0
        ),
        "（答え, ERROR, 読み出しの口を呼んだ回数）"
    );
}

/// 単体の合成の結果が「無い」（スコープのシェルが表示の層に無い）ときは、判断の断りの文言でなく
/// 専用の文言で想定外の失敗を答え、`error!` を 1 件だけ出す（要件 2.1・7.1）。
///
/// # 非空虚性
/// 判断の断りの文言（`NO_SUCH_SCOPE`）のままだと本文が違って赤。`refuse` へ回すと ERROR が 0 件で赤。
#[test]
fn alone_without_composed_result_answers_shell_not_ready_with_one_error() {
    let (step, levels) = log_capture_kit::count_levels(|| alone(0, 0, None));

    let Step::Now(answered) = step else {
        panic!("その場で答える");
    };
    assert_eq!(
        answered.content,
        vec![ToolContent::Text(
            "NG:the shell of this scope is not ready".to_string()
        )]
    );
    assert!(answered.is_error);
    assert_eq!(levels.error, 1);
}

/// 符号化のスレッドの体で出た `error!` の件数（答えと一緒に返す）。5.1 の判定の本体で、
/// 下の較正で「わざと失敗させれば 1・成功なら 0」を確かめる。
fn errors_in_finish(job: Job) -> (ToolOutcome, usize) {
    let (answered, levels) = log_capture_kit::count_levels(|| finish(TOOL, 0, job, Duration::ZERO));
    (answered, levels.error)
}

/// `finish` の較正（要件 3.3・5.2）: panic する仕事は `NG:the encoding thread panicked` と ERROR 1 件、
/// `fail` を呼ぶ仕事は ERROR 1 件、成功する仕事は ERROR 0 件。同じ判定の関数で数える。
///
/// # 非空虚性
/// `finish` が panic を受けないとテストが panic して赤。判定の関数が常に 0 を返すと前の 2 つで赤、
/// 常に 1 を返すと最後で赤。
#[test]
fn finish_calibration_counts_errors_of_failing_jobs_only() {
    let (panicked, errors) = errors_in_finish(Box::new(|_| panic!("わざと")));
    assert_eq!(
        panicked.content,
        vec![ToolContent::Text(
            "NG:the encoding thread panicked".to_string()
        )]
    );
    assert!(panicked.is_error);
    assert_eq!(errors, 1);

    let (failed, errors) = errors_in_finish(Box::new(|_| fail(TOOL, 0, "わざと")));
    assert!(failed.is_error);
    assert_eq!(errors, 1);

    let (ok, errors) = errors_in_finish(Box::new(|_| outcome::ok("撮れた")));
    assert!(!ok.is_error);
    assert_eq!(errors, 0);
}

/// 受け取り口の無い送りは黙って捨てる（要件 3.5・7.1 ⑸）: panic せず ERROR 0 件。
///
/// # 非空虚性
/// 送りの失敗を `expect` すると panic で赤、`fail` へ回すと ERROR 1 件で赤。
#[test]
fn send_back_without_receiver_drops_silently() {
    let (tx, rx) = mpsc::channel();
    drop(rx);

    let ((), levels) = log_capture_kit::count_levels(|| {
        send_back(
            tx,
            finish(TOOL, 0, Box::new(|_| outcome::ok("撮れた")), Duration::ZERO),
        )
    });

    assert_eq!(levels.error, 0);
}

/// 成功の記録の欄（要件 5.3・7.1 ⑺）: `picture` の 1×1 の仕事を UI 時間 1,234 µs で `finish` に通すと、
/// `[mcp] 絵を返す` が 1 件で、`ui_us` が 1234、`encode_us` が数として読める。
///
/// # 非空虚性
/// 記録が無い・2 件・欄の名前が変わる・`ui` を渡し損ねる（0 や別の値）といずれも赤。
#[test]
fn success_record_carries_ui_us_and_encode_us() {
    let Step::Encode(scope, job) = picture(0, 0, "撮れた".to_string(), &[0, 0, 0, 0], 1, 1)
    else {
        panic!("写しまで済んだ成功");
    };

    let (answered, events) =
        log_capture_kit::capture(|| finish(TOOL, scope, job, Duration::from_micros(1234)));

    assert!(!answered.is_error);
    let records: Vec<_> = events
        .iter()
        .filter(|e| e.message() == "[mcp] 絵を返す")
        .collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].field("ui_us"), Some("1234"));
    assert!(
        records[0]
            .field("encode_us")
            .and_then(|v| v.parse::<u64>().ok())
            .is_some()
    );
}

// ── 窓の無い切り替えの道具で、預けた呼び出しを通す（要件 3.1・3.2・3.3・3.5・4.1・4.2・4.4・7.1） ──

/// 画像つきの成功か（`isError` でなく、画像が 1 つ以上ある）。答えで判定するテストの判定の関数
/// （符号化のスレッドの `error!` は必ず `fail` を通って `NG:` の答えになるので、これが真なら
/// 符号化のスレッドの ERROR は 0 件・要件 5.1・5.2）。GPU を通るテストからも使う。
pub(in crate::mcp) fn is_picture(answered: &ToolOutcome) -> bool {
    !answered.is_error
        && answered
            .content
            .iter()
            .any(|c| matches!(c, ToolContent::Image { .. }))
}

/// 今の見た目を求める呼び出しの引数（ゴーストは省く）。
fn current_look() -> Args {
    Args {
        scope: Some(0),
        surface: None,
        ghost_name: None,
    }
}

/// `on_boot` を台本にした A と B を据え、A を起こして MCP の置き場を据えた土台（窓は作らないので
/// 装着は起きない）。A・B とも起動記録あり（`OnBoot` から始まる）で、切替で起きた側の
/// `OnGhostChanged` は 204。
fn rig_with(on_boot: &'static str) -> SwitchRig {
    let fake = |on_boot: &'static str| {
        FakeShiori::Scripted(Box::new(move || {
            standard_script(on_boot).get("OnGhostChanged", Ok(None))
        }))
    };
    let mut rig = SwitchRig::new(vec![("A", fake(on_boot)), ("B", fake(r"\0B\e"))]);
    // 切替先の窓の準備が閉包を投函する先（`Input` の段に作業プールの取り出しの系は無いので走らない）。
    rig.world.insert_resource(WintfTaskPool::new());
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");
    rig.boot("A");
    super::super::install(&mut rig.world, mpsc::channel().1);
    rig
}

/// 起きているゴースト（呼び出しを解決したゴースト）。
fn active_ghost(rig: &SwitchRig) -> ActiveGhost {
    resolve::active(&rig.world).expect("A が起きている")
}

/// 置き場（`McpLater`）に預かっている組の数。
fn held(rig: &SwitchRig) -> usize {
    rig.world.non_send::<McpLater>().0.len()
}

/// 本番の `start` に、1 回目（入口）は `None`・2 回目（最初の覗き）からは `step()` を返す `answer` を
/// 渡して預け、巡を答えが届くまで有界に回す。（預けた直後の答え, 届いた答え）。
fn deposit_and_answer(
    rig: &mut SwitchRig,
    step: impl Fn() -> Step + 'static,
) -> (Option<ToolOutcome>, Option<ToolOutcome>) {
    let ghost = active_ghost(rig);
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    let mut calls = 0;
    start(&mut rig.world, TOOL, &ghost, req.reply, move |_| {
        calls += 1;
        (calls >= 2).then(&step)
    });
    let at_deposit = pending.try_answer().ok().flatten().map(|a| a.outcome);
    let mut later = None;
    rig.pump_input_until(|_| {
        later = pending.try_answer().ok().flatten().map(|a| a.outcome);
        later.is_some()
    });
    (at_deposit, later)
}

/// 装着の前に預けた呼び出しの符号化は `mcp-encode` のスレッドで行われ、預けた時点では答えない
/// （要件 3.1・3.2・7.1 ⑶）。仕事は自分の走ったスレッドの名前を答えに書く。
///
/// # 非空虚性
/// 覗く関数が UI スレッド（テストのスレッド）で仕事を仕上げると名前が違って赤。入口で答えてしまうと
/// 預けた直後の答えが `Some` で赤。符号化待ちの段で受け取り口を覗かないと届かず `None` で赤。
#[test]
fn deposited_call_is_encoded_on_the_encoding_thread() {
    let mut rig = rig_with(r"\0A\e");

    let (at_deposit, later) = deposit_and_answer(&mut rig, || {
        Step::Encode(
            0,
            Box::new(|_| outcome::ok(std::thread::current().name().unwrap_or(""))),
        )
    });
    let down = rig.shutdown();

    assert_eq!(
        (at_deposit, later, down),
        (None, Some(outcome::ok("mcp-encode")), true),
        "（預けた直後の答え, 届いた答え, 降ろせた）"
    );
}

/// 預けた呼び出しの仕事が `mcp-encode` で panic すると `NG:the encoding thread panicked` で答え、
/// その答えを `is_picture` に通すと偽（要件 3.3・5.2・7.1 ⑷）。比べに 1×1 の絵の仕事を同じ形で
/// 預けると `is_picture` が真（判定の関数の較正）。
///
/// # 非空虚性
/// 判定の関数が常に真なら前で、常に偽なら後で赤。`finish` が panic を受けないと答えが届かず赤。
#[test]
fn deposited_panic_answers_encoding_thread_panicked_and_is_not_a_picture() {
    let mut rig = rig_with(r"\0A\e");

    let (_, panicked) =
        deposit_and_answer(&mut rig, || Step::Encode(0, Box::new(|_| panic!("わざと"))));
    let (_, pictured) = deposit_and_answer(&mut rig, || {
        picture(0, 0, "撮れた".to_string(), &[0, 0, 0, 0], 1, 1)
    });
    let down = rig.shutdown();

    assert_eq!(
        (
            panicked.clone(),
            panicked.as_ref().map(is_picture),
            pictured.as_ref().map(is_picture),
            down
        ),
        (
            Some(outcome::ng("the encoding thread panicked")),
            Some(false),
            Some(true),
            true
        ),
        "（panic の答え, それは絵か, 1×1 の絵の答えは絵か, 降ろせた）"
    );
}

/// 符号化待ちのまま待つ側が去ると、次の巡で置き場の組が 0 件になり（覗く関数ごと落ちる）、
/// その後に仕事を放して巡を回しても UI スレッドの ERROR は 0 件で panic しない（要件 3.5・7.1 ⑸）。
///
/// # 非空虚性
/// 待つ側の居ない組を残すと組の数が 1 で赤。受け取り口が切れたのを `fail` へ回すと ERROR 1 件で赤。
/// 符号化待ちまで進んでいたことは、放した仕事が走り終えること（`ran`）で確かめる。待つ側が去った後の組は
/// 覗かれずに落ちるので、仕事が走るのは去る前に符号化のスレッドを起こしていたときだけ。
#[test]
fn abandoned_while_encoding_drops_the_pair_without_ui_errors() {
    let mut rig = rig_with(r"\0A\e");
    let ghost = active_ghost(&rig);
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (ran_tx, ran_rx) = mpsc::channel::<()>();
    let mut parts = Some((release_rx, ran_tx));
    let mut calls = 0;
    start(&mut rig.world, TOOL, &ghost, req.reply, move |_| {
        calls += 1;
        if calls < 2 {
            return None;
        }
        let (release, ran) = parts.take()?;
        Some(Step::Encode(
            0,
            Box::new(move |_| {
                // 放されるまで（または止め札が落ちるまで）止まる。
                let _ = release.recv();
                let _ = ran.send(());
                outcome::ok("放した")
            }),
        ))
    });
    rig.world.run_schedule(Input);
    let held_while_encoding = held(&rig);

    drop(pending);
    let ((held_after_leave, ran), levels) = log_capture_kit::count_levels(|| {
        rig.world.run_schedule(Input);
        let held_after_leave = held(&rig);
        let _ = release_tx.send(());
        let ran = ran_rx.recv_timeout(Duration::from_secs(20)).is_ok();
        for _ in 0..3 {
            rig.world.run_schedule(Input);
        }
        (held_after_leave, ran)
    });
    let down = rig.shutdown();

    assert_eq!(
        (
            held_while_encoding,
            held_after_leave,
            ran,
            levels.error,
            down
        ),
        (1, 0, true, 0, true),
        "（符号化待ちの組の数, 去った後の組の数, 仕事が走り終えた, UI スレッドの ERROR, 降ろせた）"
    );
}

/// 預けている間に台本の `\![change,ghost,B]` で本物の切り替えが通ると、預けた呼び出しは
/// `NG:Specified ghost is not active`・`isError: true` で答え、`[mcp]` の記録に ERROR が無い
/// （要件 4.1・4.2・4.4・7.1 ⑹）。切り替えの手順そのものの記録は判定に入れない。
///
/// # 非空虚性
/// 覗く関数がゴーストを確かめないと、B は装着しないので答えが届かず `None` で赤。断りを `fail` で
/// 答えると ERROR 1 件で赤。B の定常まで届いたことで、切り替えが実際に起きたことを確かめる。
#[test]
fn switch_to_another_ghost_answers_not_active_without_mcp_errors() {
    let mut rig = rig_with(r"\0A\![change,ghost,B]\e");
    let ghost = active_ghost(&rig);
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    handle(&mut rig.world, &ghost, current_look(), req.reply);
    let at_once = pending.try_answer().ok().flatten().map(|a| a.outcome);
    let steady = rig.wait_steady();

    let (welcomed, events) = log_capture_kit::capture(|| {
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (!rig.calls("B").is_empty()
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let answered = pending.try_answer().ok().flatten().map(|a| a.outcome);
    let now = resolve::active(&rig.world).and_then(|g| g.name);
    let mcp_errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR && e.message().starts_with("[mcp]"))
        .count();
    let exit_requested = rig.exit_requested();
    let down = rig.shutdown();

    assert_eq!(
        (
            at_once,
            steady,
            welcomed,
            now,
            answered.as_ref().map(|a| a.is_error),
            answered,
            mcp_errors,
            exit_requested,
            down
        ),
        (
            None,
            true,
            true,
            Some("B".to_owned()),
            Some(true),
            Some(outcome::ng(resolve::NOT_ACTIVE)),
            0,
            false,
            true
        ),
        "（その場の答え, A の定常, B の定常まで届いた, 今のゴースト, isError, 届いた答え, \
         [mcp] の ERROR, 終了の指示, 降ろせた）"
    );
}

/// 写した後（読み出し待ち・符号化待ち）に台本の `\![change,ghost,B]` で本物の切り替えが通っても、
/// ゴーストを確かめ直さず、放すと写した時点の答えを返す（要件 4.5）。
///
/// # 非空虚性
/// 写した後の段でもゴーストを確かめると、どちらも `NG:Specified ghost is not active` で赤。
/// 切り替えの前に答えてしまうと切り替えの前の答えが `Some` で赤。B の定常まで届いたことで、
/// 切り替えが実際に起きたことを確かめる。
#[test]
fn switch_after_the_copy_still_answers_the_copied_picture() {
    let mut rig = rig_with(r"\0A\![change,ghost,B]\e");
    let ghost = active_ghost(&rig);

    // 読み出し待ち: 入口で写しを積んだ（`Read`）。放されるまで「まだ」。
    let freed = std::rc::Rc::new(std::cell::Cell::new(false));
    let seen = freed.clone();
    let mut read: Option<Reader> = Some(Box::new(move || {
        Ok(seen
            .get()
            .then(|| -> Job { Box::new(|_| outcome::ok("読み出し待ちで写した絵")) }))
    }));
    let (req, reading) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    start(&mut rig.world, TOOL, &ghost, req.reply, move |_| {
        read.take().map(|read| Step::Read(0, read))
    });

    // 符号化待ち: 最初の覗きで写しまで済み、仕事は放されるまで止まる。
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let mut release = Some(release_rx);
    let mut calls = 0;
    let (req, encoding) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    start(&mut rig.world, TOOL, &ghost, req.reply, move |_| {
        calls += 1;
        let release = (calls >= 2).then(|| release.take()).flatten()?;
        Some(Step::Encode(
            0,
            Box::new(move |_| {
                let _ = release.recv();
                outcome::ok("符号化待ちで写した絵")
            }),
        ))
    });
    rig.world.run_schedule(Input);

    let steady = rig.wait_steady();
    let welcomed = rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || (!rig.calls("B").is_empty() && rig.world.get_non_send::<SwitchInFlight>().is_none())
    });
    let now = resolve::active(&rig.world).and_then(|g| g.name);
    let before_release = (
        reading.try_answer().ok().flatten().map(|a| a.outcome),
        encoding.try_answer().ok().flatten().map(|a| a.outcome),
    );

    freed.set(true);
    let _ = release_tx.send(());
    let (mut read_answer, mut encode_answer) = (None, None);
    rig.pump_input_until(|_| {
        read_answer = read_answer
            .take()
            .or_else(|| reading.try_answer().ok().flatten().map(|a| a.outcome));
        encode_answer = encode_answer
            .take()
            .or_else(|| encoding.try_answer().ok().flatten().map(|a| a.outcome));
        read_answer.is_some() && encode_answer.is_some()
    });
    let down = rig.shutdown();

    assert_eq!(
        (
            steady,
            welcomed,
            now,
            before_release,
            read_answer,
            encode_answer,
            down
        ),
        (
            true,
            true,
            Some("B".to_owned()),
            (None, None),
            Some(outcome::ok("読み出し待ちで写した絵")),
            Some(outcome::ok("符号化待ちで写した絵")),
            true
        ),
        "（A の定常, B の定常まで届いた, 今のゴースト, 放す前の答え, 読み出し待ちの答え, \
         符号化待ちの答え, 降ろせた）"
    );
}

/// 台本の `\![change,ghost,A]` で同じゴーストを起こし直しても、同じゴーストと見なして断らず、
/// 起こし直した A が定常に達しても答えは届かない（まだ装着待ち・要件 4.2）。
///
/// # 非空虚性
/// ゴーストの見分けを起こし直しで変わるもの（実行系そのもの等）で行うと断りが届いて赤。
/// A の起動が 2 回であることで、起こし直しが実際に起きたことを確かめる。
#[test]
fn switch_to_the_same_ghost_keeps_waiting_for_attachment() {
    let mut rig = rig_with(r"\0A\![change,ghost,A]\e");
    let ghost = active_ghost(&rig);
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    handle(&mut rig.world, &ghost, current_look(), req.reply);
    let steady = rig.wait_steady();

    let welcomed = rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || (rig.calls("A").len() >= 2 && rig.world.get_non_send::<SwitchInFlight>().is_none())
    });
    // 起こし直した後の巡でも覗かれている（組が残り、答えはまだ）。
    rig.world.run_schedule(Input);
    let answered = pending.try_answer().ok().flatten().map(|a| a.outcome);
    let a_boots = rig.calls("A").len();
    let same = resolve::active(&rig.world) == Some(ghost);
    let held_after = held(&rig);
    let exit_requested = rig.exit_requested();
    let down = rig.shutdown();

    assert_eq!(
        (
            steady,
            welcomed,
            a_boots,
            same,
            answered,
            held_after,
            exit_requested,
            down
        ),
        (true, true, 2, true, None, 1, false, true),
        "（A の定常, 起こし直した A の定常まで届いた, A の起動の回数, 同じゴースト, 届いた答え, \
         預かった組の数, 終了の指示, 降ろせた）"
    );
}

/// 呼ばれた回数を数え、`ready` 回目に `then` を返す読み出しの口（それまでは「まだ」）。
fn counted_reader(
    ready: usize,
    then: impl FnOnce() -> Result<Option<Job>, ToolOutcome> + 'static,
) -> (Reader, std::rc::Rc<std::cell::Cell<usize>>) {
    let reads = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = reads.clone();
    let mut then = Some(then);
    let read: Reader = Box::new(move || {
        counted.set(counted.get() + 1);
        match counted.get() < ready {
            true => Ok(None),
            false => then.take().expect("読めた後は呼ばれない")(),
        }
    });
    (read, reads)
}

/// 入口の `answer` が文字の面の写しを積んだ（`Read`）なら、その場で答えず預け、後の巡で読み出しの口を
/// 覗くたびに 1 度呼び、「まだ」の間は待ち、読めたら仕事を `mcp-encode` で仕上げて答える（要件 1.1・1.2）。
/// 読み出し待ちの間は `answer` をやり直さない。
///
/// # 非空虚性
/// 入口で `reply_elsewhere` へ回すと「wrong path」の答えで赤。入口で口を呼ぶと入口の後の回数が 1 で赤。
/// 「まだ」で答えてしまう・読めた仕事を UI スレッドで仕上げると答えが違って赤。読み出し待ちで `answer` を
/// やり直すと `answer` の回数が 1 でなく赤。
#[test]
fn read_at_entry_is_deposited_and_read_on_later_frames() {
    let mut rig = rig_with(r"\0A\e");
    let ghost = active_ghost(&rig);
    let (req, pending) = ToolRequest::new(ToolCall::DumpSurface(current_look()));
    let (read, reads) = counted_reader(3, || {
        Ok(Some(Box::new(|_| {
            outcome::ok(std::thread::current().name().unwrap_or(""))
        })))
    });
    let mut read = Some(read);
    let answers = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = answers.clone();
    start(&mut rig.world, TOOL, &ghost, req.reply, move |_| {
        counted.set(counted.get() + 1);
        read.take().map(|read| Step::Read(0, read))
    });
    let at_deposit = pending.try_answer().ok().flatten().map(|a| a.outcome);
    let reads_at_deposit = reads.get();
    let mut later = None;
    rig.pump_input_until(|_| {
        later = pending.try_answer().ok().flatten().map(|a| a.outcome);
        later.is_some()
    });
    let down = rig.shutdown();

    assert_eq!(
        (
            at_deposit,
            reads_at_deposit,
            later,
            reads.get(),
            answers.get(),
            down
        ),
        (None, 0, Some(outcome::ok("mcp-encode")), 3, 1, true),
        "（預けた直後の答え, 入口の後の口の回数, 届いた答え, 口の回数, answer の回数, 降ろせた）"
    );
}

/// 装着待ちの覗きで `answer` が `Read` を返したら、同じ覗きの中で読み出しの口を 1 度呼び、口の失敗の
/// 答えをそのまま返す。記録は口が出した 1 件だけ（要件 1.5）。
///
/// # 非空虚性
/// 失敗の答えを捨てる・別の文言にすると答えが違って赤。覗く関数の側でも記録すると ERROR 2 件で赤。
#[test]
fn read_failure_after_attachment_answers_the_reader_outcome_with_one_error() {
    let mut rig = rig_with(r"\0A\e");

    let ((_, later), levels) = log_capture_kit::count_levels(|| {
        deposit_and_answer(&mut rig, || {
            Step::Read(0, counted_reader(1, || Err(fail(TOOL, 0, "わざと"))).0)
        })
    });
    let down = rig.shutdown();

    assert_eq!(
        (later, levels.error, down),
        (Some(outcome::ng("わざと")), 1, true),
        "（届いた答え, ERROR, 降ろせた）"
    );
}
