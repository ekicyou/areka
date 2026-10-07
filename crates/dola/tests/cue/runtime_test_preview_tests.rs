//! 先渡し（`CueSink::preview`）の檻: 中身・時点・途中登録・既定の実装の受け手の `emit` 列
//! （areka-P0-budoux-reveal-reflow 要件 2.1・2.8）。

use super::test_support::{barrier, choice, logged_commands, recording_sink, text};
use super::{BarrierKind, Cue, CueCommand, CuePlayer, CueSheet, CueSink, Rc, RefCell, TalkCue};
use dola::cue::{ActorKey, CueTarget, RoutingCommand};

/// 受け手が受け取ったものを届いた順に残す記録。
#[derive(Clone, Debug, PartialEq)]
enum Event {
    Preview(Vec<TalkCue>),
    Emit(TalkCue),
}

/// `preview` を上書きして、先渡しと `emit` を 1 本の列へ記録する受け手。
struct PreviewSink {
    log: Rc<RefCell<Vec<Event>>>,
}

impl CueSink for PreviewSink {
    fn emit(&mut self, cue: TalkCue) {
        self.log.borrow_mut().push(Event::Emit(cue));
    }

    fn preview(&mut self, upcoming: &[TalkCue]) {
        self.log
            .borrow_mut()
            .push(Event::Preview(upcoming.to_vec()));
    }
}

fn preview_sink() -> (Rc<RefCell<Vec<Event>>>, Box<dyn CueSink>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    (log.clone(), Box::new(PreviewSink { log }))
}

fn previews(log: &Rc<RefCell<Vec<Event>>>) -> Vec<Vec<TalkCue>> {
    log.borrow()
        .iter()
        .filter_map(|e| match e {
            Event::Preview(v) => Some(v.clone()),
            Event::Emit(_) => None,
        })
        .collect()
}

fn emits(log: &Rc<RefCell<Vec<Event>>>) -> Vec<TalkCue> {
    log.borrow()
        .iter()
        .filter_map(|e| match e {
            Event::Emit(c) => Some(c.clone()),
            Event::Preview(_) => None,
        })
        .collect()
}

/// 区切り（入力待ち・選択待ち）と配送の制御を挟んだ台本。同じ時刻の合図を複数含む。
fn sheet() -> CueSheet {
    CueSheet::new(vec![
        text(0.0, "a", 0.1),
        text(0.0, "b", 0.1),
        Cue {
            actor: ActorKey::from("0"),
            start_time: 0.1,
            payload: RoutingCommand::RouteRemove {
                target: CueTarget::Balloon,
            }
            .into(),
            duration: 0.0,
        },
        barrier(0.2, BarrierKind::WaitForInput { timeout: None }),
        text(0.3, "c", 0.1),
        choice(0.4, "x", "X"),
        barrier(0.5, BarrierKind::WaitForChoice { timeout: None }),
        text(0.6, "d", 0.1),
    ])
}

/// 台本の全体を最後まで歩かせる（区切りは外から解く）。
fn drive_to_end(player: &mut CuePlayer) {
    player.tick(0.0);
    player.tick(0.1);
    player.tick(0.2);
    player.resolve_click();
    player.tick(0.3);
    player.tick(0.4);
    player.tick(0.5);
    player.resolve_choice("x");
    player.tick(0.6);
    player.tick(0.7);
}

/// 先渡しの中身は、これから `emit` で届く合図の全部（配る順）。選択待ちの後ろを含み、
/// 区切りと配送の制御を含まない。
#[test]
fn preview_carries_every_upcoming_cue_in_delivery_order() {
    let mut player = CuePlayer::from_sheet(&sheet());
    let (log, sink) = preview_sink();
    player.register_sink(sink);

    let got = previews(&log);
    assert_eq!(got.len(), 1, "先渡しは登録の時点でちょうど 1 度");
    let commands: Vec<_> = got[0].iter().map(|c| c.command.clone()).collect();
    assert_eq!(
        commands,
        vec![
            CueCommand::Text("a".into()),
            CueCommand::Text("b".into()),
            CueCommand::Text("c".into()),
            CueCommand::Choice {
                id: "x".into(),
                text: "X".into(),
                references: vec![],
            },
            CueCommand::Text("d".into()),
        ],
        "選択待ちの後ろの d を含み、区切りと配送の制御を含まない"
    );

    drive_to_end(&mut player);
    assert_eq!(
        got[0],
        emits(&log),
        "先渡しの列は、後から `emit` で届く列と同じ順・同じ値"
    );
    assert_eq!(previews(&log).len(), 1, "tick では先渡しを繰り返さない");
}

/// 先渡しは最初の `emit` より前に届く。
#[test]
fn preview_arrives_before_first_emit() {
    let mut player = CuePlayer::from_sheet(&sheet());
    let (log, sink) = preview_sink();
    player.register_sink(sink);
    drive_to_end(&mut player);

    let log = log.borrow();
    assert!(
        matches!(log.first(), Some(Event::Preview(_))),
        "最初に届くのは先渡し: {:?}",
        log.first()
    );
    assert!(log.iter().skip(1).all(|e| matches!(e, Event::Emit(_))));
    assert!(log.len() > 1, "emit も届いている");
}

/// 途中で登録した受け手には、まだ配っていない残りだけが渡る。
#[test]
fn late_registered_sink_previews_only_the_rest() {
    let mut player = CuePlayer::from_sheet(&sheet());
    let (early_log, early) = preview_sink();
    player.register_sink(early);
    player.tick(0.0);
    player.tick(0.2);
    player.resolve_click();
    player.tick(0.3); // c まで配り済み

    let (late_log, late) = preview_sink();
    player.register_sink(late);
    player.tick(0.4);
    player.tick(0.5);
    player.resolve_choice("x");
    player.tick(0.6);

    let late_preview = previews(&late_log);
    assert_eq!(late_preview.len(), 1);
    let early_all = previews(&early_log).remove(0);
    assert_eq!(
        late_preview[0],
        early_all[3..].to_vec(),
        "残り（選択肢 x と d）だけが渡る"
    );
    assert_eq!(
        late_preview[0],
        emits(&late_log),
        "残りは後から届く emit と一致"
    );
}

/// 先渡しを上書きしない受け手（既定の実装）に届く `emit` の列は変わらない。
#[test]
fn default_preview_sink_emits_unchanged() {
    let mut player = CuePlayer::from_sheet(&sheet());
    let (plain_log, plain) = recording_sink();
    let (preview_log, previewing) = preview_sink();
    player.register_sink(plain);
    player.register_sink(previewing);
    drive_to_end(&mut player);

    let plain_emits = plain_log.borrow().clone();
    assert_eq!(plain_emits, emits(&preview_log));
    assert_eq!(
        plain_emits,
        previews(&preview_log).remove(0),
        "既定の受け手にも台本の合図が全部・同じ順で届く"
    );
    assert_eq!(logged_commands(&plain_log).len(), 5);
}
