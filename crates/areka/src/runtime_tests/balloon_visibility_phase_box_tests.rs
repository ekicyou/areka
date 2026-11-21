// =============================================================================
// 表示の相の観測と発行に箱をつなぐ配線の決定論テスト
//
// 観測: 「箱に文字が出ている」は四角の写し（`shown_boxes`）から、滞在は「バルーンの窓の上、
// または文字の出ている箱の上」。文字の数（普通のバルーンの窓に今出ている数）はシェルの窓の
// 絵の番号で決まり、ここの表示層（`attach_headless`）はシェルの窓も絵の番号も持たないので、
// 数の判断は文字の層の檻が、番号を渡すことは枠の檻（`frame_shell_box_integration_tests.rs`）が持つ。
// 箱の上の滞在は観測の直前に毎フレーム整える。発行: 隠す契機が箱に届くなら箱を隠す印も立てる。
//
// 写しは提示（GPU）でしか埋まらないので、写しを受け取る箱の観測は 1 スコープ分の関数へ
// 作った写しを渡して確かめ、相の観測の収集は写しが空の形で通す。
// =============================================================================

use areka_runtime::balloon_hover::BalloonHover;
use std::collections::BTreeMap;
use std::sync::mpsc;

use areka_emo_compose::{BoxLayout, BoxName, EmoWorld, fold_boxes};
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::{HitRectPx, ShownBox};
use areka_parsers::shell::{parse, parse_boxes};
use tracing::Level;

use super::super::BalloonVisibilityState;
use super::test_support::attach_headless;
use super::test_support::{runtime, world_at};
use super::*;
use crate::input_events::balloon::BalloonWiring;
use crate::input_events::shell_box::ShellBoxHover;
use areka_placement::spawn::CharWindowMarker;
use log_capture_kit::capture;
use wintf::ecs::WindowDragging;

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

fn box_hover_errors(events: &[log_capture_kit::CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == Level::ERROR && e.message().contains("箱の上の滞在"))
        .count()
}

// ---------------------------------------------------------------------------
// 観測
// ---------------------------------------------------------------------------

/// 箱の観測は写しから「文字が出ているか」を、整えた後の記録から滞在を読む。
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

    let (results, events) = capture(|| {
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
    world.insert_non_send(BalloonHover::default());
    let mut presenter = EmoPresenter::new();
    attach_headless(&mut presenter, &mut world, 0);
    let runtime = runtime();
    let mut state = BalloonVisibilityState::default();

    let (observed, events) = capture(|| {
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

/// 箱に文字が出ていない scope では、シェルの窓のドラッグはドラッグ中に数えない。
#[test]
fn shell_window_drag_without_box_text_is_not_dragging() {
    let mut world = world_at(0.0);
    world.insert_non_send(BalloonWiring::new(mpsc::channel().0));
    world.insert_non_send(BalloonHover::default());
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
