// =============================================================================
// 表示の相の観測と発行に箱をつなぐ配線の決定論テスト（areka-P0-shell-balloon task 10.3）
//
// 観測: 「箱に文字が出ている」は四角の写し（`shown_boxes`）から、滞在は「バルーンの窓の上、
// または文字の出ている箱の上」。文字の数（普通のバルーンの窓に今出ている数）はシェルの窓の
// 絵の番号で決まり、ここの表示層（`attach_headless`）はシェルの窓も絵の番号も持たないので、
// 数の判断は文字の層の檻が、番号を渡すことは枠の檻（`frame_shell_box_integration_tests.rs`）が持つ
// （areka-P0-shell-balloon-frame-align）。
// 箱の上の滞在は観測の直前に毎フレーム整える。発行: 隠す契機が箱に届くなら箱を隠す印も立てる。
//
// 写しは提示（GPU）でしか埋まらないので、写しを受け取る箱の観測は 1 スコープ分の関数へ
// 作った写しを渡して確かめ、相の観測の収集は写しが空の形で通す（通しの確かめは 12.3）。
// =============================================================================

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc;

use areka_emo_compose::{BoxLayout, BoxName, EmoWorld, fold_boxes};
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::{HitRectPx, ShownBox, TextLayerRuntime};
use areka_emo_text::state::{SurfaceKeyOutcome, TextLayerConfig};
use areka_parsers::shell::{parse, parse_boxes};
use dola::cue::{CueCommand, TalkCue};
use tracing::Level;

use super::super::BalloonVisibilityState;
use super::test_support::attach_headless;
use super::*;
use crate::input_events::balloon::BalloonWiring;
use crate::input_events::shell_box::ShellBoxHover;
use crate::placement::spawn::CharWindowMarker;
use crate::placement::test_support::capture_logs;
use wintf::ecs::{FrameTime, WindowDragging};

/// サーフェス 0 に箱 a、サーフェス 1 は箱を持たない。
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

/// 箱を隠す印を立てたときに文字の層が残す 1 行（`actor_box.rs` の `hide_boxes`）。
const HIDE_BOXES_MESSAGE: &str = "箱を隠す印を立てた（次の台詞の頭まで）";

fn layout() -> BoxLayout {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
}

fn box_a() -> BoxName {
    layout().placements(0)[0].name.clone()
}

fn shown_a() -> Vec<ShownBox> {
    vec![ShownBox {
        name: box_a(),
        element: 1,
        rect: HitRectPx {
            left: 0.0,
            top: 0.0,
            right: 100.0,
            bottom: 50.0,
        },
    }]
}

fn cue(scope: u32, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: areka_sakura::ActorKey::from(scope.to_string()),
        command,
        duration: 0.5,
    }
}

fn world_at(frame_now: f64) -> World {
    let mut world = World::new();
    world.insert_resource(FrameTime(frame_now));
    world
}

fn runtime() -> Rc<RefCell<TextLayerRuntime>> {
    Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )))
}

/// 箱の束（[`SHELL`]）を受け取ったランタイム。`\s` の鍵は数字ならその番号。
fn runtime_with_boxes(world: &mut World) -> Rc<RefCell<TextLayerRuntime>> {
    let runtime = runtime();
    runtime.borrow_mut().set_box_layout(
        world,
        layout(),
        Box::new(|key: &str| {
            key.parse()
                .map_or(SurfaceKeyOutcome::Unresolved, SurfaceKeyOutcome::Show)
        }),
        Vec::new(),
    );
    runtime
}

fn box_hover_errors(events: &[crate::placement::test_support::LogEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == Level::ERROR && e.message().contains("箱の上の滞在"))
        .count()
}

// ---------------------------------------------------------------------------
// 観測
// ---------------------------------------------------------------------------

/// 箱の観測は写しから「文字が出ているか」を、整えた後の記録から滞在を読む（要件 6.10・6.11）。
#[test]
fn box_observation_reads_the_shown_boxes_and_the_settled_hover() {
    let a = box_a();
    let mut hover = ShellBoxHover::default();
    let mut logged = false;

    hover.set(0, Some(a.clone()));
    assert_eq!(
        observe_scope_boxes(&shown_a(), Some(&mut hover), 0, &mut logged),
        (true, true),
        "文字の出ている箱の上に居る"
    );
    assert_eq!(hover.get(0), Some(&a), "居る箱が出ている間は記録を保つ");

    assert_eq!(
        observe_scope_boxes(&[], Some(&mut hover), 0, &mut logged),
        (false, false),
        "箱の文字が消えたら、ポインタが動かなくても滞在なし"
    );
    assert_eq!(hover.get(0), None, "整えた値を書き戻している");

    assert_eq!(
        observe_scope_boxes(&shown_a(), Some(&mut hover), 0, &mut logged),
        (true, false),
        "箱に文字は出ているが上に居ない"
    );
    assert!(!logged, "記録が在る間は何も記録しない");
}

/// 箱の上の滞在の記録が無いときは滞在なしとして扱い、記録は 1 度だけ（箱に文字が出ている
/// ときだけ滞在の観測を失う）。記録が戻れば武装し直す。
#[test]
fn missing_box_hover_record_means_no_hover_and_is_logged_once() {
    let mut logged = false;

    let (results, events) = capture_logs(|| {
        let empty = observe_scope_boxes(&[], None, 0, &mut logged);
        let first = observe_scope_boxes(&shown_a(), None, 0, &mut logged);
        let second = observe_scope_boxes(&shown_a(), None, 1, &mut logged);
        (empty, first, second)
    });

    assert_eq!(results.0, (false, false));
    assert_eq!(results.1, (true, false), "滞在なしとして扱う");
    assert_eq!(results.2, (true, false));
    assert_eq!(box_hover_errors(&events), 1, "記録は 1 度だけ: {events:?}");

    let mut hover = ShellBoxHover::default();
    observe_scope_boxes(&shown_a(), Some(&mut hover), 0, &mut logged);
    assert!(!logged, "記録が戻れば次の欠落で再び 1 度鳴らせる");
}

/// 相の観測の収集は、観測の直前に箱の上の滞在を整える（写しに無い箱の滞在は消える）。
/// 記録が無ければ、箱に文字が出ていない限り何も記録しない。
#[test]
fn collection_settles_the_box_hover_before_observing() {
    let mut world = world_at(0.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    let runtime = runtime();
    let mut state = BalloonVisibilityState::default();

    let (observed, events) = capture_logs(|| {
        collect_observations(
            &presenter,
            &runtime,
            &mut world,
            &[0],
            Some(0.0),
            Vec::new(),
            &mut state,
        )
    });
    assert_eq!(observed.scopes[&0].hover, Some(false));
    assert_eq!(
        box_hover_errors(&events),
        0,
        "記録が無くても、文字の出ている箱が無ければ失う観測は無い: {events:?}"
    );

    let mut hover = ShellBoxHover::default();
    hover.set(0, Some(box_a()));
    world.insert_non_send(hover);
    let observed = collect_observations(
        &presenter,
        &runtime,
        &mut world,
        &[0],
        Some(0.0),
        Vec::new(),
        &mut state,
    );

    assert_eq!(
        observed.scopes[&0].hover,
        Some(false),
        "写しに無い箱の滞在は待ちを止めない"
    );
    assert_eq!(
        world
            .get_non_send::<ShellBoxHover>()
            .expect("挿入済み")
            .get(0),
        None,
        "整えた値を記録へ書き戻している"
    );
}

/// 滞在はバルーンの窓の上、または文字の出ている箱の上（要件 6.11）。窓の観測が無くても
/// 箱の上に居れば滞在あり、どちらも観測なしなら観測なしのまま（抑止しない）。
#[test]
fn hover_is_over_the_window_or_over_a_shown_box() {
    assert_eq!(combine_hover(Some(false), false), Some(false));
    assert_eq!(combine_hover(Some(true), false), Some(true));
    assert_eq!(combine_hover(Some(false), true), Some(true));
    assert_eq!(combine_hover(None, true), Some(true));
    assert_eq!(combine_hover(None, false), None);
}

/// 箱に文字が出ている scope では、その scope のシェルの窓のドラッグも待ちを止める（要件 6.11）。
/// 別の scope のシェルの窓・ドラッグしていないシェルの窓は数えない。
#[test]
fn shell_window_drag_counts_for_scopes_with_box_text() {
    let mut world = World::new();
    let shell0 = world.spawn(CharWindowMarker { scope: 0 }).id();
    world.spawn((CharWindowMarker { scope: 1 }, WindowDragging));

    assert!(
        !observe_shell_dragging(&mut world, &[0]),
        "scope 0 のシェルの窓はドラッグしていない"
    );
    assert!(
        observe_shell_dragging(&mut world, &[1]),
        "箱に文字の出ている scope 1 のシェルの窓をドラッグ中"
    );
    assert!(
        !observe_shell_dragging(&mut world, &[]),
        "箱に文字の出ている scope が無ければ数えない"
    );

    world.entity_mut(shell0).insert(WindowDragging);
    assert!(observe_shell_dragging(&mut world, &[0]));
}

/// 箱に文字が出ていない scope では、シェルの窓のドラッグはドラッグ中に数えない
/// （本 spec の前と同じ観測）。
#[test]
fn shell_window_drag_without_box_text_is_not_dragging() {
    let mut world = world_at(0.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    world.spawn((CharWindowMarker { scope: 0 }, WindowDragging));
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    let runtime = runtime();
    let mut state = BalloonVisibilityState::default();

    let observed = collect_observations(
        &presenter,
        &runtime,
        &mut world,
        &[0],
        Some(0.0),
        Vec::new(),
        &mut state,
    );

    assert!(
        !observed.scopes[&0].box_showing,
        "較正: 箱に文字は出ていない"
    );
    assert!(!observed.dragging);
}

/// 箱に選択肢が出ていれば、スコープの「選択肢が表示中」が真になって待ちを止める（要件 6.11・
/// 数えるのは文字の層の `choice_active`＝スコープのどの場所かに選択肢があるか）。
#[test]
fn choice_in_a_box_is_observed_as_choice_active() {
    let mut world = world_at(1.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    let runtime = runtime_with_boxes(&mut world);
    {
        let mut rt = runtime.borrow_mut();
        rt.apply_cue(&cue(0, CueCommand::Emote { key: "0".into() }));
        rt.apply_cue(&cue(
            0,
            CueCommand::Choice {
                id: "OnA".into(),
                text: "はい".into(),
                references: Vec::new(),
            },
        ));
    }
    let mut state = BalloonVisibilityState::default();

    let observed = collect_observations(
        &presenter,
        &runtime,
        &mut world,
        &[0],
        Some(1.0),
        Vec::new(),
        &mut state,
    );

    assert_eq!(
        observed.scopes[&0].visible_glyphs.map(|g| g.count),
        Some(0),
        "較正: 選択肢は箱の場所にあり、普通のバルーンの窓には出ていない"
    );
    assert_eq!(observed.scopes[&0].choice_active, Some(true));
}

// ---------------------------------------------------------------------------
// 発行
// ---------------------------------------------------------------------------

/// 隠す発行 1 件を流し、窓を隠した scope と、箱を隠す印を立てた actor を返す。
fn issue_hide(trigger: VisibilityTrigger, latch: bool) -> (Vec<u32>, Vec<String>) {
    let mut world = world_at(0.0);
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    let runtime = runtime();
    let mut state = BalloonVisibilityState {
        break_latch: latch,
        ..BalloonVisibilityState::default()
    };
    let (outcome, events) = capture_logs(|| {
        issue_actions(
            &mut presenter,
            &mut world,
            &runtime,
            &mut state,
            &VisibilityObservations::default(),
            &[VisibilityAction::HideScopes {
                scopes: vec![0],
                trigger,
            }],
        )
    });
    // 較正: 窓の非表示がこの捕捉窓で見えている（印が無いことの主張を恒真にしない）。
    assert!(
        events
            .iter()
            .any(|e| e.message().contains("apply(Hide): 非表示へ")),
        "窓の非表示は今までどおり発行される: {events:?}"
    );
    let marked = events
        .iter()
        .filter(|e| e.message().contains(HIDE_BOXES_MESSAGE))
        .map(|e| e.field("actor").unwrap_or_default().to_owned())
        .collect();
    (outcome.hidden, marked)
}

/// 時間切れは窓と箱の両方を隠す（要件 6.10）。
#[test]
fn timeout_hides_the_window_and_raises_the_hide_boxes_flag() {
    for latch in [false, true] {
        let (hidden, marked) = issue_hide(VisibilityTrigger::Timeout, latch);
        assert_eq!(hidden, vec![0]);
        assert_eq!(marked, vec!["0".to_owned()], "latch={latch}");
    }
}

/// 利用者の中断は、掛け金が掛かったままなら箱も隠す。同じ巡に次の台詞が始まって掛け金が
/// 解けていれば、箱の文字は新しい台詞のものなので隠さない。
#[test]
fn user_break_raises_the_flag_only_while_latched() {
    let (hidden, marked) = issue_hide(VisibilityTrigger::UserBreak, true);
    assert_eq!(hidden, vec![0]);
    assert_eq!(marked, vec!["0".to_owned()]);

    let (hidden, marked) = issue_hide(VisibilityTrigger::UserBreak, false);
    assert_eq!(hidden, vec![0]);
    assert!(marked.is_empty(), "掛け金が解けていれば箱は隠さない");
}

/// 文字が 0 に落ちたことによる非表示は窓だけ（箱のあるサーフェスへ移っても箱の文字は出し
/// 続ける・要件 5.1）。
#[test]
fn clear_hides_only_the_window() {
    for latch in [false, true] {
        let (hidden, marked) = issue_hide(VisibilityTrigger::Clear, latch);
        assert_eq!(hidden, vec![0]);
        assert!(marked.is_empty(), "latch={latch}");
    }
}
