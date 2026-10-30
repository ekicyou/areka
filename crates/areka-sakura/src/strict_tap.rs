//! strict の台本の合図の配り口（spec: mcp-strict-errors）。`compile_strict` が残す控え [`StrictTrace`] と、
//! strict の talk だけ実の sink を覆う [`StrictTap`] を置く。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use dola::cue::{CueProbe, CueSink, TalkCue};

use crate::strict::{CharCursor, DetectionPoint, Place, record};

/// `compile_strict` の控え: 合図ごとの台本の位置と、合図の順番に結びつけた誤り。
///
/// 順番は sink へ配られる合図（`CuePayload::Command`）だけを数える。先頭の全消しは順番 0、
/// 末尾の選択待ちの区切りと、後で足すエピローグの合図は数えない。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StrictTrace {
    /// 合図の順番 → 台本の読みの範囲（バイト）。None は台本に元の無い合図。
    pub(crate) cue_spans: Vec<Option<Range<usize>>>,
    /// （結びつけた合図の順番, 検出点, 読みの範囲）を位置の順に。順番が合図の数と等しいものは
    /// 最後の合図より後ろの命令の誤り。
    pub(crate) pending: Vec<Pending>,
}

/// 合図にまだ届いていない控え（位置は文字の数へ直し済み）。覆いと駆動が分け持つ（talk スレッドの中だけ）。
pub(crate) struct StrictLedger {
    ghost: Arc<str>,
    pending: VecDeque<(usize, DetectionPoint, Place)>,
}

impl StrictLedger {
    /// 控えを位置の順に 1 度だけ数えて文字の数へ直す。
    pub(crate) fn new(ghost: Arc<str>, script: &str, trace_pending: Vec<Pending>) -> Self {
        let mut cursor = CharCursor::new(script);
        let pending = trace_pending
            .into_iter()
            .map(|(at, point, span)| (at, point, cursor.place(span)))
            .collect();
        Self { ghost, pending }
    }

    /// 順番 `k` 以下の合図に結びついた控えを順に記録する。
    fn record_up_to(&mut self, k: usize) {
        while let Some((at, point, place)) = self.pending.front() {
            if *at > k {
                break;
            }
            record(&self.ghost, point.finding(), place);
            self.pending.pop_front();
        }
    }

    /// 残りの控えを全部記録する（自然な終わり・選択待ちの始まり・合図の無い台本）。
    pub(crate) fn finish(&mut self) {
        self.record_up_to(usize::MAX);
    }
}

/// 控え 1 件（結びつけた合図の順番, 検出点, 読みの範囲）。
type Pending = (usize, DetectionPoint, Range<usize>);

/// strict の talk だけ実の sink を全部覆い、`CuePlayer` へ 1 つの sink として登録される配り口。
///
/// 合図を配るたびに、その順番までの控えを記録してから各 sink へ登録順に配る。位置の在る合図は
/// 知らせの口を添える（`emit_with`）。打ち切りでは覆いごと捨てる（残りの控えは記録しない）。
pub(crate) struct StrictTap {
    sinks: Vec<Box<dyn CueSink + Send>>,
    /// 合図の順番 → 台本の位置（文字の数）。配った合図の分は取り出して知らせの口へ渡す。
    cue_places: Vec<Option<Place>>,
    ghost: Arc<str>,
    ledger: Rc<RefCell<StrictLedger>>,
    /// 次に配る合図の順番。
    next: usize,
}

impl StrictTap {
    /// 合図の位置は talk スレッドの上で 1 度だけ前から数えて文字の数へ直す（範囲は台本の順）。
    pub(crate) fn new(
        sinks: Vec<Box<dyn CueSink + Send>>,
        cue_spans: Vec<Option<Range<usize>>>,
        script: &str,
        ghost: Arc<str>,
        ledger: Rc<RefCell<StrictLedger>>,
    ) -> Self {
        let mut cursor = CharCursor::new(script);
        let cue_places = cue_spans
            .into_iter()
            .map(|span| span.map(|span| cursor.place(span)))
            .collect();
        Self {
            sinks,
            cue_places,
            ghost,
            ledger,
            next: 0,
        }
    }
}

impl CueSink for StrictTap {
    fn emit(&mut self, cue: TalkCue) {
        self.ledger.borrow_mut().record_up_to(self.next);
        match self.cue_places.get_mut(self.next).and_then(Option::take) {
            Some(place) => {
                // 口は出来上がった位置と名だけを持つ。どのスレッドから知らせても記録 1 件を書くだけ。
                let ghost = Arc::clone(&self.ghost);
                let probe = CueProbe::new(move |finding| record(&ghost, finding, &place));
                for sink in self.sinks.iter_mut() {
                    sink.emit_with(cue.clone(), &probe);
                }
            }
            None => {
                for sink in self.sinks.iter_mut() {
                    sink.emit(cue.clone());
                }
            }
        }
        self.next += 1;
    }

    fn preview(&mut self, upcoming: &[TalkCue]) {
        for sink in self.sinks.iter_mut() {
            sink.preview(upcoming);
        }
    }

    /// 選択待ちの区切りは列の末尾だけなので、再生は台本の全部の命令に届いている。
    fn choice_wait_began(&mut self) {
        self.ledger.borrow_mut().finish();
        for sink in self.sinks.iter_mut() {
            sink.choice_wait_began();
        }
    }
}
