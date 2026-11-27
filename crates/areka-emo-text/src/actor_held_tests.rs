//! 文字の層の、時刻の来た合図を当てる口（受け取りは列へ足し、`advance` が届いた順に当てる）。
//!
//! 受け取りは UI のスレッドの取り出しと同じ口（[`receive_msg`]）を直に呼ぶ。期待の状態は、同じ合図を
//! 同じ順に [`TextLayerRuntime::apply_cue`] へ直に当てた状態と比べる。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use areka_emo_compose::BoxName;
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue, TimePoint, TimeSpan};
use dola::cue::{CueFinding, CueProbe, OriginNotice};
use dola::time::testing::{at_ms, delta_ms, ms};
use wintf::ecs::world::tick_wake::{self, WakeBits};

use super::receive_msg;
use crate::actor::TextLayerRuntime;
use crate::actor::test_support::{
    balloon, box_index, box_layout, pump_until_idle, reveal_duration, spawn_message_drain,
};
use crate::sink::TextMsg;
use crate::state::{SurfaceKeyOutcome, TextLayerConfig};

/// 台本の起点。0 でない値にして、台本の時刻と発火時刻の取り違えを拾う。
const ORIGIN: TimePoint = at_ms(10_000);

fn actor() -> ActorKey {
    ActorKey::from("0")
}

fn runtime() -> RefCell<TextLayerRuntime> {
    RefCell::new(TextLayerRuntime::new(TextLayerConfig::default()))
}

/// 起点 [`ORIGIN`] の台本の、台本の時刻 `at` の合図（文字は 1 字 50 ms・ほかは長さ 0）。
fn cue(at: u32, command: CueCommand) -> TalkCue {
    let duration = match &command {
        CueCommand::Text(t) => reveal_duration(t),
        _ => TimeSpan::ZERO,
    };
    TalkCue::scheduled(ORIGIN, ms(at), actor(), command, duration)
}

fn text(at: u32, s: &str) -> TalkCue {
    cue(at, CueCommand::Text(s.into()))
}

fn choice(at: u32, id: &str) -> TalkCue {
    cue(
        at,
        CueCommand::Choice {
            id: id.into(),
            text: id.into(),
            references: Vec::new(),
        },
    )
}

/// 台本の時刻 `at` に当たるフレームの時刻。
fn frame(at: u32) -> TimePoint {
    ORIGIN + ms(at)
}

fn send(rt: &RefCell<TextLayerRuntime>, msg: TextMsg) {
    assert_eq!(receive_msg(rt, msg), Ok(ControlFlow::Continue(())));
}

fn send_cues(rt: &RefCell<TextLayerRuntime>, cues: &[TalkCue]) {
    for c in cues {
        send(rt, TextMsg::Cue(c.clone()));
    }
}

/// 同じ合図を、届いた順のまま直に当てた文字の層。
fn applied_directly(cues: &[TalkCue]) -> TextLayerRuntime {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    for c in cues {
        rt.apply_cue(c);
    }
    rt
}

/// 発火時刻が 3 秒後の消去は、そのフレームの時刻が届くまで画面を消さない。
#[test]
fn erase_three_seconds_later_takes_effect_at_its_fire_time() {
    let rt = runtime();
    send_cues(&rt, &[text(0, "あいう"), cue(3000, CueCommand::Clear)]);
    let mut rt = rt.borrow_mut();

    assert_eq!(
        rt.advance(frame(0)),
        Some(frame(3000)),
        "消去が発火時刻を待つ"
    );
    assert_eq!(rt.advance(frame(2999)), Some(frame(3000)));
    assert_eq!(
        rt.state().clear_count(&actor()),
        0,
        "発火時刻の前は消さない"
    );
    assert_eq!(rt.state().visible_glyphs(&actor(), frame(2999)), 3);

    assert_eq!(rt.advance(frame(3000)), None);
    assert_eq!(
        rt.state().clear_count(&actor()),
        1,
        "発火時刻のフレームで消す"
    );
    assert_eq!(rt.state().visible_glyphs(&actor(), frame(3000)), 0);
}

/// 字のほかの指令（改行・カーソル・書体・アンカー・印）も、発火時刻のフレームで初めて効く。
#[test]
fn non_glyph_commands_take_effect_at_their_fire_time() {
    let later = [
        CueCommand::NewLine { ratio: 1.0 },
        CueCommand::Cursor {
            x: "10".into(),
            y: "0".into(),
        },
        crate::state::test_support::font("0", &["height", "40"]).command,
        CueCommand::AnchorBegin {
            id: "a".into(),
            references: Vec::new(),
        },
        CueCommand::command_carrier("*", vec![]),
    ];
    for command in later {
        let head = text(0, "あ");
        let late = cue(3000, command.clone());
        let before = applied_directly(std::slice::from_ref(&head));
        let after = applied_directly(&[head.clone(), late.clone()]);
        assert_ne!(before.state(), after.state(), "{command:?} は状態を変える");

        let rt = runtime();
        send_cues(&rt, &[head, late]);
        let mut rt = rt.borrow_mut();
        assert_eq!(rt.advance(frame(2999)), Some(frame(3000)), "{command:?}");
        assert_eq!(
            rt.state(),
            before.state(),
            "{command:?} は発火時刻の前に効かない"
        );
        assert_eq!(rt.advance(frame(3000)), None, "{command:?}");
        assert_eq!(
            rt.state(),
            after.state(),
            "{command:?} は発火時刻のフレームで効く"
        );
    }
}

/// 届いたフレームの時刻より後の発火時刻の合図は、そのフレームでは当たらず、`advance` がその時刻を返す。
#[test]
fn cue_later_than_the_frame_waits_and_its_time_is_returned() {
    let rt = runtime();
    send_cues(&rt, &[text(500, "あ")]);
    let mut rt = rt.borrow_mut();

    assert_eq!(rt.advance(frame(0)), Some(frame(500)));
    assert!(
        rt.state().actor_state(&actor()).is_none(),
        "まだ当たっていない"
    );

    assert_eq!(rt.advance(frame(500)), None, "待つ物が無い");
    assert_eq!(rt.state().visible_glyphs(&actor(), frame(500)), 1);
}

/// 同じ時刻の合図は、届いた順（台本の順）に当たる。
#[test]
fn same_time_cues_apply_in_script_order() {
    let cues = [
        text(500, "あ"),
        cue(500, CueCommand::Clear),
        text(500, "い"),
    ];
    let rt = runtime();
    send_cues(&rt, &cues);
    let mut rt = rt.borrow_mut();

    assert_eq!(rt.advance(frame(500)), None);
    assert_eq!(rt.state(), applied_directly(&cues).state());
    assert_eq!(rt.state().clear_count(&actor()), 1);
    assert_eq!(
        rt.state().visible_glyphs(&actor(), TimePoint::MAX),
        1,
        "消去の後の「い」だけが残る"
    );
}

/// 遅れて届いた合図は、届いた後の最初のフレームで、過ぎた分が順に全部当たる。
#[test]
fn late_cues_all_apply_in_order_on_the_next_frame() {
    let cues = [
        text(0, "あ"),
        cue(100, CueCommand::NewLine { ratio: 1.0 }),
        text(200, "い"),
        cue(300, CueCommand::Clear),
        text(400, "う"),
    ];
    let rt = runtime();
    send_cues(&rt, &cues);
    let mut rt = rt.borrow_mut();

    assert_eq!(rt.advance(frame(1000)), None);
    assert_eq!(rt.state(), applied_directly(&cues).state());
}

/// 1 字 50 ms の塊が 250 ms 遅れて当たると、過ぎた字は一度に現れ、残りは元の間隔で現れる。
#[test]
fn typewriter_cue_250ms_late_shows_passed_glyphs_at_once() {
    let rt = runtime();
    send_cues(&rt, &[text(0, "あいうえおかきくけこ")]);
    let mut rt = rt.borrow_mut();

    assert_eq!(rt.advance(frame(249)), None);
    let times = rt
        .state()
        .actor_state(&actor())
        .expect("当たった")
        .reveal()
        .times();
    assert_eq!(times.first(), Some(&frame(0)), "字の時刻は発火時刻から積む");
    assert_eq!(times.last(), Some(&frame(450)));
    let visible = |at: u32| rt.state().visible_glyphs(&actor(), frame(at));
    assert_eq!(
        visible(249),
        5,
        "0・50・100・150・200 ms の 5 字が一度に現れる"
    );
    assert_eq!(visible(250), 6, "ちょうど 250 ms なら 6 字目も現れる");
    assert_eq!(visible(299), 6);
    assert_eq!(visible(300), 7, "残りは元の間隔のまま");
    assert_eq!(visible(450), 10);
}

/// 先渡しは、前の台詞の合図が全部当たった後・その台詞の最初の合図の前に当たる（区間の番号がずれない）。
#[test]
fn preview_applies_after_previous_talk_and_before_its_first_cue() {
    let rt = runtime();
    send_cues(
        &rt,
        &[
            cue(0, CueCommand::ClearAll),
            text(0, "前"),
            cue(500, CueCommand::Clear),
            text(500, "後"),
        ],
    );
    assert_eq!(rt.borrow_mut().advance(frame(0)), Some(frame(500)));

    let next = [cue(1000, CueCommand::ClearAll), text(1000, "次")];
    send(&rt, TextMsg::Upcoming(next.to_vec()));
    assert_eq!(
        rt.borrow().lookahead_rev,
        0,
        "前の台詞の合図が残る間は当てない"
    );
    send_cues(&rt, &next);

    let mut rt = rt.borrow_mut();
    assert_eq!(rt.advance(frame(1000)), None);
    assert_eq!(rt.lookahead_rev, 1);
    assert_eq!(
        rt.lookahead.number(&balloon("0")),
        1,
        "先渡しの後に数えた消去は次の台詞の頭の 1 つだけ"
    );
}

/// トークの終わりの印は、その前に届いた選択肢の合図が当たった後に当たる。
#[test]
fn talk_end_mark_applies_after_the_choice_cue() {
    let rt = runtime();
    send_cues(&rt, &[choice(100, "c")]);
    send(&rt, TextMsg::TalkEnded);
    let mut rt = rt.borrow_mut();

    assert_eq!(rt.advance(frame(0)), Some(frame(100)));
    assert!(
        rt.state().actor_state(&actor()).is_none(),
        "選択肢はまだ無い"
    );

    assert_eq!(rt.advance(frame(100)), None);
    let spans = rt
        .state()
        .actor_state(&actor())
        .expect("当たった")
        .choices()
        .len();
    assert_eq!(spans, 1, "選択肢は当たった");
    assert!(
        !rt.choice_active(&actor()),
        "その後に終わりの印が済みにした"
    );
}

/// 列が空の時に届いた先渡しとトークの印は、受け取った場で当たる。
#[test]
fn preview_and_marks_apply_on_receipt_when_the_queue_is_empty() {
    let rt = runtime();
    send(&rt, TextMsg::Upcoming(vec![text(0, "あ")]));
    assert_eq!(rt.borrow().lookahead_rev, 1, "先渡しがその場で当たった");

    send(&rt, TextMsg::ChoiceWaitBegan);
    assert!(
        rt.borrow().choice_wait_open,
        "選択待ちの始まりがその場で当たった"
    );

    send_cues(&rt, &[choice(0, "c")]);
    assert_eq!(rt.borrow_mut().advance(frame(0)), None);
    assert!(rt.borrow().choice_active(&actor()));
    send(&rt, TextMsg::TalkEnded);
    let rt = rt.borrow();
    assert!(!rt.choice_active(&actor()), "終わりの印がその場で当たった");
    assert!(!rt.choice_wait_open);
    assert!(rt.held.is_empty());
}

/// 画面に出た選択肢への押下は、選択待ちの始まりの印が当たった後なら選択肢を済みにする。
#[test]
fn press_settles_the_choice_after_the_choice_wait_mark_applies() {
    let rt = runtime();
    send_cues(&rt, &[choice(0, "c")]);
    assert_eq!(rt.borrow_mut().advance(frame(0)), None);

    // 後ろの合図が列で待つ間に届いた印は、列で待つ。
    send_cues(&rt, &[text(100, "あ")]);
    send(&rt, TextMsg::ChoiceWaitBegan);
    rt.borrow_mut().settle_choices_at(&balloon("0"));
    assert!(
        rt.borrow().choice_active(&actor()),
        "印の前の押下は済みにしない"
    );

    let mut rt = rt.borrow_mut();
    assert_eq!(rt.advance(frame(100)), None);
    rt.settle_choices_at(&balloon("0"));
    assert!(!rt.choice_active(&actor()), "印の後の押下は済みにする");
}

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

/// 名前の形の `\b` の箱が無い事は、受け取った時でなく、当てた時に知らせる。
#[test]
fn missing_named_box_is_reported_when_applied() {
    let rt = runtime();
    {
        let mut rt = rt.borrow_mut();
        let index: BTreeMap<u32, Vec<BoxName>> = box_index(&box_layout(SHELL), 0..=0);
        rt.state.set_box_index(index);
        rt.state.route_surface(&actor(), SurfaceKeyOutcome::Show(0));
    }
    let found: Arc<Mutex<Vec<CueFinding>>> = Arc::default();
    let sink = Arc::clone(&found);
    let probe = CueProbe::new(move |f| sink.lock().expect("findings").push(f));
    let missing = cue(
        500,
        CueCommand::BalloonSurface {
            key: "無い箱".into(),
        },
    );
    send(&rt, TextMsg::ProbedCue(missing, probe));
    assert!(
        found.lock().expect("findings").is_empty(),
        "受け取りでは知らせない"
    );

    assert_eq!(rt.borrow_mut().advance(frame(499)), Some(frame(500)));
    assert!(found.lock().expect("findings").is_empty());
    assert_eq!(rt.borrow_mut().advance(frame(500)), None);
    let found = found.lock().expect("findings").clone();
    assert_eq!(found.len(), 1, "当てた時に 1 回だけ知らせる");
    assert_eq!(found[0].point, "BalloonBoxMissing");
}

/// 受け取りは列へ足すだけで当てず、足した後に次の画面の更新でフレームを回す旗を立てる。
#[test]
fn receipt_holds_without_applying_and_marks_present() {
    let rt = runtime();
    let _ = tick_wake::take(Instant::now());
    send_cues(&rt, &[text(0, "あ")]);
    let woke = tick_wake::take(Instant::now());
    assert!(WakeBits(woke.bits).contains(tick_wake::PRESENT), "旗が立つ");
    let rt = rt.borrow();
    assert!(
        rt.state().actor_state(&actor()).is_none(),
        "受け取りでは当てない"
    );
    assert!(!rt.held.is_empty(), "列に持つ");
}

/// 文字の層が借用中なら、受け取りは `Err` を返し、列に何も足さない。
#[test]
fn receipt_while_borrowed_is_an_error_and_holds_nothing() {
    let rt = runtime();
    {
        let _busy = rt.borrow();
        assert!(receive_msg(&rt, TextMsg::Cue(text(0, "あ"))).is_err());
        assert!(receive_msg(&rt, TextMsg::TalkEnded).is_err());
    }
    assert!(rt.borrow().held.is_empty());
}

// ── 置き直しの知らせ ──

/// 効く時刻 `effective`（台本の時刻）に、起点を [`ORIGIN`] から `delta` ms 動かす置き直しの知らせ。
fn moved(effective: u32, delta: i64) -> OriginNotice {
    OriginNotice {
        effective_at: frame(effective),
        origin: ORIGIN + delta_ms(delta),
        previous: Some(ORIGIN),
    }
}

fn reveal_times(rt: &TextLayerRuntime, actor: &ActorKey) -> Vec<TimePoint> {
    rt.state()
        .actor_state(actor)
        .expect("当たった")
        .reveal()
        .times()
        .to_vec()
}

/// 効く時刻以前の字は動かず、後の字だけが動いた長さだけ後ろへ動く。
#[test]
fn reorigin_moves_only_glyphs_after_the_effective_time() {
    let rt = runtime();
    send_cues(&rt, &[text(0, "あいうえお")]);
    assert_eq!(rt.borrow_mut().advance(frame(0)), None);

    send(&rt, TextMsg::Origin(moved(100, 500)));
    let mut rt = rt.borrow_mut();
    assert_eq!(
        rt.advance(frame(99)),
        Some(frame(100)),
        "知らせは効く時刻を待つ"
    );
    assert_eq!(
        reveal_times(&rt, &actor()),
        [0, 50, 100, 150, 200].map(frame),
        "効く時刻の前は動かない"
    );
    assert_eq!(rt.advance(frame(100)), None);
    assert_eq!(
        reveal_times(&rt, &actor()),
        [0, 50, 100, 650, 700].map(frame),
        "ちょうど効く時刻の字は動かず、後の字は 500 ms 後ろへ"
    );
}

/// 前へ動いて効く時刻より前になる字は、効く時刻に現れる。
#[test]
fn reorigin_earlier_clamps_to_the_effective_time() {
    let rt = runtime();
    send_cues(&rt, &[text(0, "あいうえおか")]);
    send(&rt, TextMsg::Origin(moved(100, -120)));
    let mut rt = rt.borrow_mut();
    assert_eq!(rt.advance(frame(100)), None);
    assert_eq!(
        reveal_times(&rt, &actor()),
        [0, 50, 100, 100, 100, 130].map(frame)
    );
    assert_eq!(
        rt.state().visible_glyphs(&actor(), frame(100)),
        5,
        "時刻の来ている字は効く時刻のフレームで一度に現れる"
    );
}

/// 知らせより前に届いてまだ当たっていない合図の字も、当てた後に動く（合図を先に当てた時と同じ時刻）。
#[test]
fn reorigin_moves_glyphs_of_cues_held_before_it() {
    let applied_first = runtime();
    send_cues(&applied_first, &[text(0, "あいうえお")]);
    assert_eq!(applied_first.borrow_mut().advance(frame(0)), None);
    send(&applied_first, TextMsg::Origin(moved(100, 500)));
    assert_eq!(applied_first.borrow_mut().advance(frame(100)), None);

    let held = runtime();
    send_cues(&held, &[text(0, "あいうえお")]);
    send(&held, TextMsg::Origin(moved(100, 500)));
    assert_eq!(held.borrow_mut().advance(frame(100)), None);

    assert_eq!(
        reveal_times(&held.borrow(), &actor()),
        [0, 50, 100, 650, 700].map(frame)
    );
    assert_eq!(held.borrow().state(), applied_first.borrow().state());
}

/// 知らせは全部の場所の字を動かす。
#[test]
fn reorigin_moves_glyphs_of_every_place() {
    let other = ActorKey::from("1");
    let rt = runtime();
    send_cues(
        &rt,
        &[
            text(0, "あいう"),
            TalkCue::scheduled(
                ORIGIN,
                ms(0),
                other.clone(),
                CueCommand::Text("かきく".into()),
                reveal_duration("かきく"),
            ),
        ],
    );
    send(&rt, TextMsg::Origin(moved(50, 1000)));
    let mut rt = rt.borrow_mut();
    assert_eq!(rt.advance(frame(50)), None);
    assert_eq!(reveal_times(&rt, &actor()), [0, 50, 1100].map(frame));
    assert_eq!(reveal_times(&rt, &other), [0, 50, 1100].map(frame));
}

/// 前へ動いた後に届いた合図は、新しい起点での発火時刻が効く時刻より前でも、効く時刻より前のフレームで当たらない。
#[test]
fn cue_after_reorigin_waits_for_the_effective_time() {
    let rt = runtime();
    send(&rt, TextMsg::Origin(moved(1000, -500)));
    let after = TalkCue::scheduled(
        ORIGIN - ms(500),
        ms(1200),
        actor(),
        CueCommand::Text("あ".into()),
        reveal_duration("あ"),
    );
    assert_eq!(after.fire_time, frame(700));
    send_cues(&rt, &[after]);
    let mut rt = rt.borrow_mut();

    assert_eq!(rt.advance(frame(800)), Some(frame(1000)));
    assert!(
        rt.state().actor_state(&actor()).is_none(),
        "効く時刻より前は当たらない"
    );
    assert_eq!(rt.advance(frame(1000)), None);
    assert_eq!(rt.state().visible_glyphs(&actor(), frame(1000)), 1);
}

/// 受け口は、最初の起点の知らせを送らず、置き直しの知らせだけを送る。
#[test]
fn sink_forwards_only_reorigin_notices() {
    let (mut sink, records) = spawn_message_drain("emo-text-origin-test");
    sink.origin_set(OriginNotice::first(ORIGIN));
    pump_until_idle();
    assert!(
        records.lock().expect("records").is_empty(),
        "最初の起点の知らせは送らない"
    );

    sink.origin_set(moved(100, 500));
    pump_until_idle();
    assert_eq!(
        *records.lock().expect("records"),
        vec![TextMsg::Origin(moved(100, 500))]
    );
}
