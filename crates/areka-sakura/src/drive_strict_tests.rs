//! strict の talk の配り口の覆いの檻（spec: mcp-strict-errors 要件 2.5・2.9・3.5・3.6・3.7・3.11・4.3）。
//!
//! 記録は talk スレッドの上で書かれる。捕捉の窓はスレッドごとなので、多くの檻は駆動の状態機械
//! （[`TalkDriver`]）をテストのスレッドの上で直に回す（`spawn_talk` の本体と同じ `handle`）。
//! 別のスレッドから後で届く知らせだけは実の talk スレッドで回し、talk の終わった後にテストのスレッドから送る。

use super::test_support::{NoopSink, TalkNotice};
use super::*;
use crate::contract::{CueCommand, TalkCue};
use crate::strict::{DetectionPoint, RECORD_TARGET, StrictJudges};
use dola::cue::CueProbe;
use log_capture_kit::CapturedEvent;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// `nosuch` だけ誰も拾わない。`\f` は診ない。
struct Judges;

impl StrictJudges for Judges {
    fn command_handled(&self, name: &str, _first: Option<&str>) -> bool {
        name != "nosuch"
    }
    fn font(&self, _args: &[String]) -> Option<DetectionPoint> {
        None
    }
}

fn hooks() -> StrictHooks {
    StrictHooks {
        ghost: "さくら".into(),
        judges: Arc::new(Judges),
    }
}

fn start(script: &str, strict: bool) -> StartTalk {
    StartTalk {
        strict,
        ..StartTalk::new(TalkId(1), script)
    }
}

/// 配られた合図を捕捉の列へ 1 件の出来事として出す受け手（記録と合図の前後を見るため）。
/// `emit_with` は上書きしない（窓の無いゴーストの `LogSink`・`DiscardSink` と同じ形）。
struct TraceSink;

impl CueSink for TraceSink {
    fn emit(&mut self, cue: TalkCue) {
        tracing::info!(target: "strict_test::cue", "{:?}", cue.command);
    }
}

/// 面の合図に知らせの口が添えられたら、その場で「シェルに無い」を知らせる受け手。
/// 知らせの口が添えられた回数も数える。
#[derive(Clone, Default)]
struct SurfaceProbeSink {
    with_probe: Arc<Mutex<usize>>,
    /// 知らせずに口だけ取っておく（別のスレッドから後で知らせる檻）。
    keep: Option<Arc<Mutex<Option<CueProbe>>>>,
}

impl CueSink for SurfaceProbeSink {
    fn emit(&mut self, _cue: TalkCue) {}

    fn emit_with(&mut self, cue: TalkCue, probe: &CueProbe) {
        *self.with_probe.lock().unwrap() += 1;
        if matches!(cue.command, CueCommand::Emote { .. }) {
            match &self.keep {
                Some(keep) => *keep.lock().unwrap() = Some(probe.clone()),
                None => probe.report(DetectionPoint::SurfaceAbsent.finding()),
            }
        }
        self.emit(cue);
    }
}

fn driver(
    sinks: Vec<Box<dyn CueSink + Send>>,
    hooks: Option<StrictHooks>,
) -> (TalkDriver<TalkNotice>, Receiver<TalkNotice>) {
    let (tx, rx) = mpsc::channel();
    (
        TalkDriver::new(sinks, SystemVarSnapshot::default(), tx, hooks),
        rx,
    )
}

/// 窓の無いゴーストの受け手の組（知らせない受け手 2 つ）。
fn windowless() -> Vec<Box<dyn CueSink + Send>> {
    vec![Box::new(TraceSink), Box::new(NoopSink)]
}

/// strict の記録だけを（点, 本文）にする。
fn records(events: &[CapturedEvent]) -> Vec<(String, String)> {
    events
        .iter()
        .filter(|ev| ev.target == RECORD_TARGET)
        .map(|ev| {
            assert_eq!(ev.field_str("ghost"), Some("さくら"));
            (
                ev.field_str("point").unwrap_or_default().to_string(),
                ev.message().to_string(),
            )
        })
        .collect()
}

/// 記録と合図の並び（記録は点の名・合図は `cue`）。
fn sequence(events: &[CapturedEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|ev| match ev.target.as_str() {
            RECORD_TARGET => Some(ev.field_str("point").unwrap_or_default().to_string()),
            "strict_test::cue" => Some("cue".to_string()),
            _ => None,
        })
        .collect()
}

fn dones(rx: &Receiver<TalkNotice>) -> Vec<TalkDone> {
    rx.try_iter()
        .filter_map(|notice| match notice {
            TalkNotice::Done(done) => Some(done),
            TalkNotice::ChoiceWaiting(_) => None,
        })
        .collect()
}

const MIDDLE_AND_TAIL: &str = "\\![nosuch]あ\\w9\\x";

/// 合図の順で記録する: 命令の誤りはその命令の後の最初の合図の直前に、最後の合図より後ろの
/// 誤りは自然な終わりで、終わりの知らせの前に（2.5・3.7）。窓の無いゴーストの組では台本の
/// 解釈の誤りだけが残る（4.3）。
#[test]
fn records_in_cue_order_and_tail_at_natural_end() {
    let (mut driver, rx) = driver(windowless(), Some(hooks()));
    let ((), events) = log_capture_kit::capture(|| {
        assert!(
            driver
                .handle(SakuraMsg::Start(start(MIDDLE_AND_TAIL, true)))
                .is_continue()
        );
        assert!(driver.handle(SakuraMsg::Tick(0.0)).is_continue());
        assert!(dones(&rx).is_empty());
        assert!(driver.handle(SakuraMsg::Tick(100.0)).is_break());
    });
    // 全消し → `\![nosuch]` の記録 → `\![nosuch]` の合図 → あ → 待ち → 終わりで `\x`。
    assert_eq!(
        sequence(&events),
        [
            "cue",
            "CommandUnhandled",
            "cue",
            "cue",
            "cue",
            "ReadUnknownTag"
        ]
    );
    assert_eq!(
        records(&events),
        [
            (
                "CommandUnhandled".to_string(),
                format!(
                    "unknown_command 0-10 {}: \\![nosuch]",
                    crate::strict::MSG_UNKNOWN_COMMAND
                )
            ),
            (
                "ReadUnknownTag".to_string(),
                format!("unknown_tag 14-16 {}: \\x", crate::strict::MSG_UNKNOWN_TAG)
            ),
        ]
    );
    assert_eq!(dones(&rx).len(), 1);
}

/// 打ち切りでは配った合図より後ろの控えを記録しない。記録済みの分は残る（3.6・3.7）。
#[test]
fn interrupt_drops_the_rest() {
    let (mut driver, rx) = driver(windowless(), Some(hooks()));
    let ((), events) = log_capture_kit::capture(|| {
        let _ = driver.handle(SakuraMsg::Start(start(MIDDLE_AND_TAIL, true)));
        let _ = driver.handle(SakuraMsg::Tick(0.0));
        assert!(driver.handle(SakuraMsg::Close).is_break());
    });
    // 時刻 0 の合図（全消し・`\![nosuch]`・あ）まで。待ちの後ろの `\x` は記録しない。
    assert_eq!(sequence(&events), ["cue", "CommandUnhandled", "cue", "cue"]);
    let done = dones(&rx);
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].reason, TalkEndReason::Interrupted);
}

/// 初回 `Tick` の前の打ち切りは何も記録しない。
#[test]
fn interrupt_before_first_tick_records_nothing() {
    let (mut driver, _rx) = driver(windowless(), Some(hooks()));
    let ((), events) = log_capture_kit::capture(|| {
        let _ = driver.handle(SakuraMsg::Start(start(MIDDLE_AND_TAIL, true)));
        let _ = driver.handle(SakuraMsg::Close);
    });
    assert!(records(&events).is_empty());
}

/// 選択肢で終わる台本は選択待ちの始まりで残りを記録し、その後に閉じても落ちない（3.7）。
#[test]
fn choice_wait_records_the_rest_and_close_keeps_it() {
    let (mut driver, rx) = driver(windowless(), Some(hooks()));
    let ((), events) = log_capture_kit::capture(|| {
        let _ = driver.handle(SakuraMsg::Start(start("\\q[題,ID]\\x", true)));
        let _ = driver.handle(SakuraMsg::Tick(0.0));
        let _ = driver.handle(SakuraMsg::Tick(100.0));
    });
    assert!(
        rx.try_iter()
            .any(|n| matches!(n, TalkNotice::ChoiceWaiting(_)))
    );
    let recs = records(&events);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].0, "ReadUnknownTag");
    // 閉じても記録は増えも消えもしない。
    let ((), events) = log_capture_kit::capture(|| {
        assert!(driver.handle(SakuraMsg::Close).is_break());
    });
    assert_eq!(records(&events), []);
    assert_eq!(dones(&rx)[0].reason, TalkEndReason::Interrupted);
}

/// 合図の無い台本は、終わりを送る前に控えを全部記録する。
#[test]
fn empty_sheet_records_before_done() {
    let (mut driver, rx) = driver(windowless(), Some(hooks()));
    let (flow, events) =
        log_capture_kit::capture(|| driver.handle(SakuraMsg::Start(start("\\x", true))));
    assert!(flow.is_break());
    assert_eq!(sequence(&events), ["ReadUnknownTag"]);
    assert_eq!(dones(&rx).len(), 1);
}

/// strict 偽（束ねが在っても）と、束ねの無い strict 真は今の道: 記録 0・知らせの口も添えない（3.5）。
#[test]
fn not_strict_keeps_the_current_path() {
    for (strict, hooks) in [(false, Some(hooks())), (true, None)] {
        let sink = SurfaceProbeSink::default();
        let with_probe = Arc::clone(&sink.with_probe);
        let (mut driver, rx) = driver(vec![Box::new(sink), Box::new(TraceSink)], hooks);
        let ((), events) = log_capture_kit::capture(|| {
            let _ = driver.handle(SakuraMsg::Start(start("\\s[99]\\![nosuch]あ\\x", strict)));
            let _ = driver.handle(SakuraMsg::Tick(0.0));
            let _ = driver.handle(SakuraMsg::Tick(100.0));
        });
        assert_eq!(records(&events), [], "strict={strict}");
        assert_eq!(*with_probe.lock().unwrap(), 0, "strict={strict}");
        // 合図は全部、今のまま配られる（全消し・面・`\![nosuch]`・あ）。
        assert_eq!(sequence(&events).len(), 4, "strict={strict}");
        assert_eq!(dones(&rx).len(), 1);
    }
}

/// 位置の在る合図だけ知らせの口が添えられ、talk スレッドの上の知らせはその合図の位置で記録される。
/// 位置は再生した台本（翻訳で書き換えた後の `StartTalk::script`）の文字の数（2.5・3.3）。
#[test]
fn probe_records_at_the_cue_place_of_the_played_script() {
    let sink = SurfaceProbeSink::default();
    let with_probe = Arc::clone(&sink.with_probe);
    let (mut driver, _rx) = driver(vec![Box::new(sink)], Some(hooks()));
    // 翻訳の後の台本（元は `\s[99]う`）。
    let ((), events) = log_capture_kit::capture(|| {
        let _ = driver.handle(SakuraMsg::Start(start("やくした\\s[99]う", true)));
        let _ = driver.handle(SakuraMsg::Tick(0.0));
        let _ = driver.handle(SakuraMsg::Tick(100.0));
    });
    assert_eq!(
        records(&events),
        [(
            "SurfaceAbsent".to_string(),
            format!(
                "missing_surface 4-10 {}: \\s[99]",
                crate::strict::MSG_MISSING_SURFACE
            )
        )]
    );
    // 全消し（位置なし）には添えず、やくした・面・う の 3 つに添える。
    assert_eq!(*with_probe.lock().unwrap(), 3);
}

/// 別のスレッドから後で届く知らせ（talk も覆いも終わった後）は、talk スレッドの上で数えた位置で
/// 記録 1 件を書くだけ（知らせる側で台本を数えない・要件 3.11）。
#[test]
fn late_report_records_the_place_computed_on_the_talk_thread() {
    let keep = Arc::new(Mutex::new(None));
    let sink = SurfaceProbeSink {
        keep: Some(Arc::clone(&keep)),
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel::<TalkNotice>();
    let handle = spawn_talk(
        start("あい\\s[99]う", true),
        tx,
        vec![Box::new(sink)],
        SystemVarSnapshot::default(),
        Some(hooks()),
    );
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(100.0)).unwrap();
    super::test_support::recv_done(&rx, Duration::from_secs(5)).expect("自然に終わる");
    handle.actor.join().expect("body は正常終了する");

    let probe = keep
        .lock()
        .unwrap()
        .take()
        .expect("面の合図に口が添えられた");
    let ((), events) =
        log_capture_kit::capture(|| probe.report(DetectionPoint::SurfaceAbsent.finding()));
    assert_eq!(
        records(&events),
        [(
            "SurfaceAbsent".to_string(),
            format!(
                "missing_surface 2-8 {}: \\s[99]",
                crate::strict::MSG_MISSING_SURFACE
            )
        )]
    );
}
