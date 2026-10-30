//! 名前の形の `\b[名前]` を、文字の層が箱を探した場で知らせる（mcp-strict-errors task 5.2・要件 4.2・7.3）。
//!
//! 受け口 [`EmoTextSink::emit_with`] は名前の形の `\b` だけ [`TextMsg::ProbedCue`] を積み、UI の取り出し
//! （[`spawn_emo_text`]）は箱が見つからず空回しの写しでもないときだけ 1 回知らせる。実 channel・実 pump。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use areka_emo_compose::{BoxName, EmoWorld, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use dola::cue::{CueFinding, CueProbe, CueSink};
use log_capture_kit::count_levels;

use super::test_support::{cue, pump_until_idle};
use super::{TextLayerRuntime, spawn_emo_text};
use crate::sink::{EmoTextSink, TextMsg};
use crate::state::{SurfaceKeyOutcome, TextLayerConfig};

/// サーフェス 0 は箱 a だけ。
const SHELL: &str = "\
balloon.a
{
size,100,50
}
surface0
{
element1,balloon,a,0,0
}
";

fn index() -> BTreeMap<u32, Vec<BoxName>> {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    let names = layout
        .placements(0)
        .iter()
        .map(|p| p.name.clone())
        .collect();
    BTreeMap::from([(0, names)])
}

fn select(key: &str) -> TalkCue {
    cue("0", 0.0, CueCommand::BalloonSurface { key: key.into() })
}

/// 知らせを積む口と、積まれた知らせ。
fn recording_probe() -> (CueProbe, Arc<Mutex<Vec<CueFinding>>>) {
    let found = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&found);
    let probe = CueProbe::new(move |f| sink.lock().expect("findings lock").push(f));
    (probe, found)
}

/// 受け口は名前の形の `\b` だけ口付きの知らせを積み、数の `\b` とほかの合図は今と同じ `Cue` で積む。
#[test]
fn emit_with_wraps_only_name_form_balloon_cues() {
    let records = Arc::new(Mutex::new(Vec::new()));
    let recs = Arc::clone(&records);
    let (tx, _drain) = areka_actor::spawn_ui(
        "emo-text-probe-sink-test",
        move |msg: TextMsg| -> Result<ControlFlow<()>, String> {
            recs.lock().expect("records lock").push(msg);
            Ok(ControlFlow::Continue(()))
        },
    )
    .expect("spawn_ui on the test (pump) thread must succeed");
    let mut sink = EmoTextSink::new(tx);
    let (probe, _found) = recording_probe();

    let named = select("a");
    let numbered = select("0");
    let text = cue("0", 0.0, CueCommand::Text("あ".into()));
    sink.emit_with(named.clone(), &probe);
    sink.emit_with(numbered.clone(), &probe);
    sink.emit_with(text.clone(), &probe);
    pump_until_idle();

    assert_eq!(
        *records.lock().expect("records lock"),
        vec![
            TextMsg::ProbedCue(named, probe.clone()),
            TextMsg::Cue(numbered),
            TextMsg::Cue(text),
        ],
        "口が付くのは名前の形の \\b だけ・順は積んだ順"
    );
}

/// 知らせの等しさは口の等しさ（同じ口なら等しく、別の口なら合図が同じでも等しくない）。
#[test]
fn probed_cue_equality_follows_the_probe() {
    let (a, _) = recording_probe();
    let (b, _) = recording_probe();
    let msg = TextMsg::ProbedCue(select("a"), a.clone());
    assert_eq!(msg.clone(), msg, "複製は同じ口");
    assert_ne!(msg, TextMsg::ProbedCue(select("a"), b), "別の口");
    assert!(format!("{msg:?}").contains("CueProbe"), "表示は口を載せる");
}

/// UI の取り出し: 箱が無い名前の `\b` は 1 回だけ知らせ（`warn!` も残る）、在る箱では知らせない。
#[test]
fn drain_reports_missing_box_once_and_not_for_present_box() {
    let ((), counts) = count_levels(|| {
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        {
            let mut rt = runtime.borrow_mut();
            rt.state.set_box_index(index());
            rt.state
                .route_surface(&ActorKey::from("0"), SurfaceKeyOutcome::Show(0));
        }
        let (mut sink, _drain) =
            spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the pump thread");
        let (present, present_found) = recording_probe();
        let (missing, missing_found) = recording_probe();

        sink.emit_with(select("a"), &present);
        sink.emit_with(select("無い箱"), &missing);
        pump_until_idle();

        assert_eq!(
            *present_found.lock().expect("findings lock"),
            vec![],
            "在る箱では知らせない"
        );
        let found = missing_found.lock().expect("findings lock").clone();
        assert_eq!(found.len(), 1, "箱が無いと 1 回だけ知らせる");
        assert_eq!(found[0].point, "BalloonBoxMissing");
        assert_eq!(found[0].kind, "missing_balloon");
    });
    assert_eq!(counts.warn, 1, "今の warn! は残る（箱が無い 1 回）");
}

/// 空回しの写しは箱が無くても「知らせる」と答えない（本番の適用だけが知らせる）。
#[test]
fn rehearsal_copy_never_answers_box_missing() {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.state.set_box_index(index());
    rt.state
        .route_surface(&ActorKey::from("0"), SurfaceKeyOutcome::Show(0));
    let mut copy = rt.state.rehearsal_copy();
    assert!(!copy.apply_cue(&select("無い箱")), "空回しの写しは答えない");
    assert!(rt.state.apply_cue(&select("無い箱")), "本番の状態は答える");
    assert!(!rt.state.apply_cue(&select("a")), "在る箱は答えない");
}
