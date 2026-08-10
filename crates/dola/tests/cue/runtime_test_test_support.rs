//! `runtime_test` のテーマ別テストファイルが共有するヘルパ（要件 1.7 のテーマ分割・design.md:151）。
//!
//! 収録は 2 テーマ以上から参照される項目と、その内部被呼出だけである（§40.5／§42.5）。単一テーマ
//! からしか参照されないヘルパ（`newline`・`cursor`・`ready_has_text`）は各テーマファイルに残した。
//!
//! 注: `RecordingSink` の直前にある区画バナー（原本 `:656-658`＝Task 4.3 の broadcast 区画）は、
//! 本文一致検証が先行コメント塊を次の項目へ束ねるため、共有ヘルパ `RecordingSink` に同伴して
//! ここへ移っている（§46.3 と同型・切り離すと `ITEM-MISSING` になる）。

use super::{ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CueSink, Rc, RefCell, TalkCue};

/// バリア cue を作る補助（Barrier ペイロードは presentation でなく duration 0）。
pub(super) fn barrier(start_time: f64, kind: BarrierKind) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CuePayload::Barrier(kind),
        duration: 0.0,
    }
}

/// テキスト cue を作る補助。
pub(super) fn text(start_time: f64, s: &str, duration: f64) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::Text(s.into()).into(),
        duration,
    }
}

/// 選択肢 cue を作る補助（WaitForChoice の前に先積みされる）。
pub(super) fn choice(start_time: f64, id: &str, s: &str) -> Cue {
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

// ============================================================================
// Task 4.3: broadcast 配送・占有終了検知（is_completed）・中断（stop）・sink 登録
// ============================================================================

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

/// 記録用 sink を作り、（外から中身を覗くための共有ログ, `CuePlayer` へ渡す trait object）を返す。
pub(super) fn recording_sink() -> (Rc<RefCell<Vec<TalkCue>>>, Box<dyn CueSink>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let sink = Box::new(RecordingSink { log: log.clone() });
    (log, sink)
}

/// ログに記録された cue の command 列だけを取り出す補助。
pub(super) fn logged_commands(log: &Rc<RefCell<Vec<TalkCue>>>) -> Vec<CueCommand> {
    log.borrow().iter().map(|c| c.command.clone()).collect()
}
