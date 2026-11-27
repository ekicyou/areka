//! 時計が止まっている間（眠り）に台本の再生が進まず、動き出すと止めた所から続く事の結合テスト。

use std::ops::ControlFlow;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use areka_sakura::{
    ChoiceWaiting, CueCommand, CueSink, SakuraMsg, StartTalk, SystemVarSnapshot, TalkCue, TalkDone,
    TalkId, TimePoint, talk_handler,
};
use dola::time::testing::at_ms;

enum Notice {
    Done,
    ChoiceWaiting,
}

impl From<TalkDone> for Notice {
    fn from(_: TalkDone) -> Self {
        Notice::Done
    }
}

impl From<ChoiceWaiting> for Notice {
    fn from(_: ChoiceWaiting) -> Self {
        Notice::ChoiceWaiting
    }
}

struct Recorder(Arc<Mutex<Vec<TalkCue>>>);

impl CueSink for Recorder {
    fn emit(&mut self, cue: TalkCue) {
        self.0.lock().unwrap().push(cue);
    }
}

fn texts(records: &Mutex<Vec<TalkCue>>) -> Vec<(String, TimePoint)> {
    records
        .lock()
        .unwrap()
        .iter()
        .filter_map(|c| match &c.command {
            CueCommand::Text(t) => Some((t.clone(), c.fire_time)),
            _ => None,
        })
        .collect()
}

fn done_count(rx: &mpsc::Receiver<Notice>) -> usize {
    rx.try_iter().filter(|n| matches!(n, Notice::Done)).count()
}

/// `bye` は起点から 600 ms・占有の終わりは 750 ms。起点の 300 ms 後で時計が止まり、その値の刻みが
/// 何度来ても `bye` も完了も出ない。動き出した後は、起点＋600 ms ちょうどで `bye` が出て、
/// 起点＋750 ms で終わる（止まっていた間の分は足されない）。
#[test]
fn playback_holds_while_clock_is_stopped_and_resumes_from_there() {
    let records = Arc::new(Mutex::new(Vec::new()));
    let (done_tx, done_rx) = mpsc::channel::<Notice>();
    let mut handle = talk_handler(
        vec![Box::new(Recorder(Arc::clone(&records)))],
        SystemVarSnapshot::default(),
        done_tx,
        None,
    );
    let origin = at_ms(10_000);
    let stopped = at_ms(10_300);

    assert_eq!(
        handle(SakuraMsg::Start(StartTalk::new(
            TalkId(1),
            r"\s[0]hi\_w[500]bye\e"
        ))),
        ControlFlow::Continue(())
    );
    assert_eq!(handle(SakuraMsg::Tick(origin)), ControlFlow::Continue(()));
    assert_eq!(handle(SakuraMsg::Tick(stopped)), ControlFlow::Continue(()));
    let before_sleep = records.lock().unwrap().len();

    for _ in 0..5 {
        assert_eq!(handle(SakuraMsg::Tick(stopped)), ControlFlow::Continue(()));
    }
    assert_eq!(
        records.lock().unwrap().len(),
        before_sleep,
        "止まった時計の刻みでは合図が増えない"
    );
    assert_eq!(texts(&records), vec![("hi".to_string(), origin)]);
    assert_eq!(done_count(&done_rx), 0, "止まっている間に完了しない");

    assert_eq!(
        handle(SakuraMsg::Tick(at_ms(10_599))),
        ControlFlow::Continue(())
    );
    assert_eq!(texts(&records), vec![("hi".to_string(), origin)]);
    assert_eq!(
        handle(SakuraMsg::Tick(at_ms(10_600))),
        ControlFlow::Continue(())
    );
    assert_eq!(
        texts(&records),
        vec![
            ("hi".to_string(), origin),
            ("bye".to_string(), at_ms(10_600))
        ],
        "動き出した後、bye は起点＋600 ms ちょうどで出る"
    );
    assert_eq!(done_count(&done_rx), 0);

    assert_eq!(
        handle(SakuraMsg::Tick(at_ms(10_749))),
        ControlFlow::Continue(())
    );
    assert_eq!(
        handle(SakuraMsg::Tick(at_ms(10_750))),
        ControlFlow::Break(())
    );
    assert_eq!(done_count(&done_rx), 1, "占有の終わりで 1 度だけ完了する");
}
