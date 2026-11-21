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

use areka_emo_present::EmoPresenter;

use super::super::BalloonVisibilityState;
use super::test_support::attach_headless;
use super::test_support::{runtime, world_at};
use super::*;
use areka_placement::spawn::CharWindowMarker;
use log_capture_kit::capture;
use wintf::ecs::WindowDragging;

/// 箱を隠す印を立てたときに文字の層が残す 1 行（`actor_box.rs` の `hide_boxes`）。
const HIDE_BOXES_MESSAGE: &str = "箱を隠す印を立てた（次の台詞の頭まで）";

/// 滞在はバルーンの窓の上、または文字の出ている箱の上。窓の観測が無くても
/// 箱の上に居れば滞在あり、どちらも観測なしなら観測なしのまま（抑止しない）。
#[test]
fn hover_is_over_the_window_or_over_a_shown_box() {
    assert_eq!(combine_hover(Some(false), false), Some(false));
    assert_eq!(combine_hover(Some(true), false), Some(true));
    assert_eq!(combine_hover(Some(false), true), Some(true));
    assert_eq!(combine_hover(None, true), Some(true));
    assert_eq!(combine_hover(None, false), None);
}

/// 箱に文字が出ている scope では、その scope のシェルの窓のドラッグも待ちを止める。
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
    let (outcome, events) = capture(|| {
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

/// 時間切れは窓と箱の両方を隠す。
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
/// 続ける）。
#[test]
fn clear_hides_only_the_window() {
    for latch in [false, true] {
        let (hidden, marked) = issue_hide(VisibilityTrigger::Clear, latch);
        assert_eq!(hidden, vec![0]);
        assert!(marked.is_empty(), "latch={latch}");
    }
}

/// 隠すと普通のバルーンの手動の位置を捨てる。箱に届く契機
/// （時間切れ）でも届かない契機（文字が 0 に落ちた）でも同じ。
#[test]
fn hiding_forgets_the_manual_position() {
    use crate::frame::balloon_runtime_test_support::{balloon_is_manual, manual_balloon_runtime};
    for trigger in [VisibilityTrigger::Timeout, VisibilityTrigger::Clear] {
        let (_permit, _gpu_world, runtime) = manual_balloon_runtime("0");
        let mut world = world_at(0.0);
        let mut presenter = EmoPresenter::new();
        attach_headless(&mut presenter, &mut world, 0);
        issue_actions(
            &mut presenter,
            &mut world,
            &runtime,
            &mut BalloonVisibilityState::default(),
            &VisibilityObservations::default(),
            &[VisibilityAction::HideScopes {
                scopes: vec![0],
                trigger,
            }],
        );
        assert!(!balloon_is_manual(&runtime, "0"), "{trigger:?}");
    }
}
