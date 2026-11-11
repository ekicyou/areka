//! cue のテストが共有する道具（合図を作る補助と、配られた合図を記録する受け手）。

use dola::cue::{ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSink, TalkCue};
use std::cell::RefCell;
use std::rc::Rc;

/// バリア cue を作る補助（Barrier ペイロードは presentation でなく duration 0）。
pub(crate) fn barrier(start_time: f64, kind: BarrierKind) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CuePayload::Barrier(kind),
        duration: 0.0,
    }
}

/// テキスト cue を作る補助。
pub(crate) fn text(start_time: f64, s: &str, duration: f64) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::Text(s.into()).into(),
        duration,
    }
}

/// 選択肢 cue を作る補助（WaitForChoice の前に先積みされる）。
pub(crate) fn choice(start_time: f64, id: &str, s: &str) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::Choice {
            id: id.into(),
            text: s.into(),
            references: vec![],
        }
        .into(),
        duration: 0.0,
    }
}

/// broadcast の記録用テストダブル sink。受け取った全 `TalkCue` を共有ログへ push する。
///
/// 演者側 relevance（担当 action の選別）は 4.3 の範囲外（演者タスク 6.x）ゆえ、ここでは
/// 「登録された全 sink が全 cue を無変形で受け取る」ことのみを観測する（broadcast＝全員が
/// 全 cue を受ける・R2.1）。
struct RecordingSink {
    log: Rc<RefCell<Vec<TalkCue>>>,
}

impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.log.borrow_mut().push(cue);
    }
}

/// 待ちの cue を作る補助。
pub(crate) fn wait(start_time: f64, duration: f64) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::Wait.into(),
        duration,
    }
}

/// 記録用 sink を作り、（外から中身を覗くための共有ログ, `CuePlayer` へ渡す trait object）を返す。
pub(crate) fn recording_sink() -> (Rc<RefCell<Vec<TalkCue>>>, Box<dyn CueSink>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let sink = Box::new(RecordingSink { log: log.clone() });
    (log, sink)
}

/// ログに記録された cue の command 列だけを取り出す補助。
pub(crate) fn logged_commands(log: &Rc<RefCell<Vec<TalkCue>>>) -> Vec<CueCommand> {
    log.borrow().iter().map(|c| c.command.clone()).collect()
}
