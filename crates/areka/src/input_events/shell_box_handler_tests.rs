// =============================================================================
// シェルの窓のハンドラの箱の前段（areka-P0-shell-balloon task 11.2）
//
// 箱の四角（`shown_boxes`）と箱の選択肢の当たり行（`choice_hit_rows_at`）は提示（GPU）でしか
// 埋まらないので、箱の中の操作は前段の本体（`move_with_point`・`press_with_point`）へ作った写しを
// 渡して確かめる。シェルへ届く通知は、`mod.rs` のハンドラと同じ並び（前段が「処理した」なら
// 落とさない）で既存の道を続けて呼んで数える（`Fixture::moved`・`Fixture::pressed`）。このとき
// world に `Emo2Wiring` を置かないので、ハンドラの中の本物の前段は何もしない（写しを 2 度当てない）。
// 箱の外は `Emo2Wiring` を置いた本物のハンドラ（写しが空の文字の層）で、本 spec の前と同じ通知に
// なることを確かめる。
// =============================================================================

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_emo_compose::{BoxName, EmoWorld, fold_boxes};
use areka_emo_text::actor::{HitRectPx, ShownBox, TextLayerRuntime};
use areka_emo_text::state::{SpanKind, TextLayerConfig};
use areka_kanade::{KanadeMsg, MouseButton, MouseEventKind};
use areka_parsers::shell::{parse, parse_boxes};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use wintf::ecs::pointer::{DoubleClick, Phase, PointerLeave, PointerState};
use wintf::ecs::{Input, Point, Window};

use super::*;
use crate::emo2_boot::hit_region::HitRegion;
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::emo2_boot::user_break_cue::NoUserBreakSignal;
use crate::input_events::balloon::test_support::{
    anchor_cues, anchor_row, headless_emo2_wiring, row,
};
use crate::input_events::balloon::{ChoiceSelectionInbox, click_selection, wire_balloon_choice};
use crate::input_events::shell_box::ShellBoxHover;
use crate::input_events::user_break::{UserBreakWiring, drain_no_user_break_signals};
use crate::input_events::{
    MouseWiring, RegionSource, on_char_pointer_moved, on_char_pointer_pressed,
};
use crate::placement::spawn::CharWindowMarker;
use crate::placement::test_support::capture_logs;

const SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.b
{
size,100,50
}
surface0
{
element1,balloon,a,0,0
element2,balloon,b,200,0
}
";

/// 検体の箱の名前（`BoxName` は畳み込みからしか作れない）。
fn name(s: &str) -> BoxName {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, _) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    layout
        .placements(0)
        .iter()
        .find(|p| p.name.as_str() == s)
        .expect("検体に在る箱")
        .name
        .clone()
}

/// 箱 a（0,0〜100,50）に当たった写し。選択肢の行は (10,10)〜(90,20) の 1 本。
fn on_a(active: bool) -> BoxPoint {
    BoxPoint {
        hit: Some(ShownBox {
            name: name("a"),
            element: 1,
            rect: HitRectPx {
                left: 0.0,
                top: 0.0,
                right: 100.0,
                bottom: 50.0,
            },
        }),
        rows: vec![row(0, 10.0, 10.0, 90.0, 20.0)],
        active,
    }
}

/// 箱 b（200,0〜300,50）に当たった写し（選択肢なし）。
fn on_b() -> BoxPoint {
    BoxPoint {
        hit: Some(ShownBox {
            name: name("b"),
            element: 2,
            rect: HitRectPx {
                left: 200.0,
                top: 0.0,
                right: 300.0,
                bottom: 50.0,
            },
        }),
        rows: Vec::new(),
        active: true,
    }
}

const ROW: (i32, i32) = (50, 15);
const BODY: (i32, i32) = (50, 40);

/// 結線済みの World と、シェルへの通知・中断の 2 本の線の受信端。
struct Fixture {
    world: World,
    entity: Entity,
    runtime: Rc<RefCell<TextLayerRuntime>>,
    /// シェルへの通知（`MouseWiring` の線）。
    shell_rx: Receiver<KanadeMsg>,
    /// 中断の「止めろ」（`UserBreakWiring` の運行の線）。
    break_rx: Receiver<KanadeMsg>,
    /// 中断の「隠せ」。
    lifecycle_rx: Receiver<TalkLifecycleSignal>,
    flag_tx: Sender<NoUserBreakSignal>,
}

impl Fixture {
    fn new() -> Self {
        let (shell_tx, shell_rx) = mpsc::channel();
        let (break_tx, break_rx) = mpsc::channel();
        let (lifecycle_tx, lifecycle_rx) = mpsc::channel();
        let (flag_tx, flag_rx) = mpsc::channel();
        let mut world = World::new();
        world.insert_non_send(MouseWiring::new(
            shell_tx,
            RegionSource::Mock(|scope, x, y| HitRegion {
                scope,
                region: Some("Head".to_owned()),
                surface_point: (x, y),
            }),
        ));
        world.insert_non_send(UserBreakWiring::new(flag_rx, lifecycle_tx, break_tx));
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        wire_balloon_choice(&mut world);
        let entity = world.spawn(CharWindowMarker { scope: 0 }).id();
        Fixture {
            world,
            entity,
            runtime,
            shell_rx,
            break_rx,
            lifecycle_rx,
            flag_tx,
        }
    }

    /// 文字の層を `Emo2Wiring` に載せる（本物の前段が写しを読む・写しは空＝箱の外）。
    fn with_emo2(mut self) -> Self {
        self.world
            .insert_non_send(headless_emo2_wiring(Rc::clone(&self.runtime)));
        self
    }

    fn talking(mut self) -> Self {
        self.flag_tx
            .send(NoUserBreakSignal::TalkStarted)
            .expect("受信端は持ち物が持つ");
        drain_no_user_break_signals(&mut self.world);
        self.break_rx.try_iter().for_each(drop); // 旗の知らせがあれば捨てる
        self
    }

    /// `mod.rs` の押下ハンドラと同じ並び: 前段が処理しなければ既存の道へ。
    fn pressed(&mut self, at: (i32, i32), double_click: DoubleClick, point: BoxPoint) -> bool {
        let state = pointer(at, double_click);
        press_with_point(&mut self.world, 0, &state, point)
            || on_char_pointer_pressed(
                &mut self.world,
                self.entity,
                self.entity,
                &Phase::Bubble(state),
            )
    }

    /// `mod.rs` の移動ハンドラと同じ並び。
    fn moved(&mut self, at: (i32, i32), point: BoxPoint) -> bool {
        let state = pointer(at, DoubleClick::None);
        move_with_point(
            &mut self.world,
            &self.runtime,
            0,
            point,
            at.0 as f32,
            at.1 as f32,
        ) || on_char_pointer_moved(
            &mut self.world,
            self.entity,
            self.entity,
            &Phase::Bubble(state),
        )
    }

    fn shell(&self) -> Vec<KanadeMsg> {
        self.shell_rx.try_iter().collect()
    }

    fn selections(&self) -> Vec<crate::input_events::balloon::ChoiceSelection> {
        self.world
            .get_non_send::<ChoiceSelectionInbox>()
            .expect("結線済み")
            .0
            .try_iter()
            .collect()
    }

    fn hover(&self) -> (Option<BoxName>, Option<usize>) {
        let hover = self
            .world
            .get_non_send::<ShellBoxHover>()
            .expect("wire_balloon_choice が入れる");
        (hover.get(0).cloned(), hover.injected(0))
    }
}

fn pointer(at: (i32, i32), double_click: DoubleClick) -> PointerState {
    PointerState {
        client_point: Point { x: at.0, y: at.1 },
        double_click,
        left_down: true,
        ..Default::default()
    }
}

fn hover_injections(events: &[crate::placement::test_support::LogEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("box_choice_hover_inject"))
        .count()
}

// ---------------------------------------------------------------- 結線

/// 箱の上の滞在の資源は `wire_balloon_choice` が入れる（ゴーストの切替でも同じ関数が呼ばれる）。
#[test]
fn wire_balloon_choice_inserts_the_box_hover() {
    let mut world = World::new();
    wire_balloon_choice(&mut world);
    assert!(world.get_non_send::<ShellBoxHover>().is_some());
}

/// 離脱の系の登録は入力の段へ 1 本だけ足す。
#[test]
fn register_box_leave_system_adds_one_input_system() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register_box_leave_system(&mut world);
    let len = world
        .resource::<Schedules>()
        .get(Input)
        .map_or(0, |s| s.systems_len());
    assert_eq!(len, 1);
}

// ---------------------------------------------------------------- 押下

/// 箱の選択肢のクリックは、普通のバルーンと同じ中身の選択のイベントが 1 件届き、シェルへは
/// 何も届かない（要件 8.3・8.5）。
#[test]
fn box_choice_click_sends_one_selection_and_nothing_to_the_shell() {
    let mut f = Fixture::new();
    assert!(f.pressed(ROW, DoubleClick::None, on_a(true)), "処理した");
    let expected = click_selection(true, &on_a(true).rows, 50.0, 15.0, 0);
    assert_eq!(f.selections(), vec![expected.expect("行の上")]);
    assert!(f.shell().is_empty(), "シェルへのクリックは 0 件");
}

/// 選択の確定に続くダブルクリックの 2 打目もシェルのダブルクリックにしない（要件 8.5）。
#[test]
fn double_click_on_box_choice_is_not_a_shell_double_click() {
    let mut f = Fixture::new().talking();
    assert!(f.pressed(ROW, DoubleClick::None, on_a(true)));
    // 2 打目は行の外（選択肢が消えた後を模す）・左ダブルクリック。
    assert!(f.pressed(BODY, DoubleClick::Left, on_a(false)));
    assert!(f.shell().is_empty(), "シェルへのダブルクリックは 0 件");
    assert!(f.break_rx.try_iter().next().is_none(), "中断もしない");
}

/// 話している最中の箱の左ダブルクリックは中断になり、シェルへのダブルクリックは 0 件（要件 9.6）。
#[test]
fn talking_left_double_click_on_box_breaks_without_a_shell_double_click() {
    let mut f = Fixture::new().talking();
    assert!(f.pressed(BODY, DoubleClick::Left, on_a(false)), "処理した");
    assert_eq!(
        f.lifecycle_rx.try_iter().collect::<Vec<_>>(),
        vec![TalkLifecycleSignal::UserBreak]
    );
    assert!(matches!(
        f.break_rx.try_iter().collect::<Vec<_>>()[..],
        [KanadeMsg::UserBreak { scope: 0 }]
    ));
    assert!(f.shell().is_empty(), "シェルへのダブルクリックは 0 件");
}

/// 箱の中の行の上でない単押しはシェルへの操作のまま（今までどおり何も送らない）。
/// 話していないあいだの左ダブルクリックはシェルのダブルクリックになる（要件 9.1）。
#[test]
fn not_talking_left_double_click_on_box_reaches_the_shell() {
    let mut f = Fixture::new();
    assert!(
        !f.pressed(BODY, DoubleClick::None, on_a(false)),
        "単押しは何もしない"
    );
    assert!(f.pressed(BODY, DoubleClick::Left, on_a(false)));
    assert!(matches!(
        f.shell()[..],
        [KanadeMsg::Mouse(ref m)] if m.kind == MouseEventKind::DoubleClick { button: MouseButton::Left }
    ));
    assert!(f.break_rx.try_iter().next().is_none());
}

/// 箱の外の押下は、話している最中でも本 spec の前と同じ（本物のハンドラ・写しが空の文字の層・
/// 要件 9.2・9.8）。
#[test]
fn press_outside_boxes_is_unchanged() {
    let mut f = Fixture::new().with_emo2().talking();
    let ev = Phase::Bubble(pointer(BODY, DoubleClick::Left));
    assert!(on_char_pointer_pressed(
        &mut f.world,
        f.entity,
        f.entity,
        &ev
    ));
    assert!(matches!(
        f.shell()[..],
        [KanadeMsg::Mouse(ref m)] if m.kind == MouseEventKind::DoubleClick { button: MouseButton::Left }
    ));
    assert!(f.break_rx.try_iter().next().is_none(), "中断しない");
    assert!(f.selections().is_empty());
    // 写しで箱の外を渡しても素通り。
    assert!(!press_with_point(
        &mut f.world,
        0,
        &pointer(BODY, DoubleClick::Left),
        BoxPoint::default()
    ));
}

// ---------------------------------------------------------------- 移動

/// 選択肢の行の上の移動は強調だけで、シェルへは送らない。同じ行の上の 2 度目は注入しない（要件 8.2）。
#[test]
fn move_over_box_choice_highlights_once_without_sending_to_the_shell() {
    let mut f = Fixture::new();
    let (handled, events) = capture_logs(|| {
        let first = f.moved(ROW, on_a(true));
        let second = f.moved((51, 15), on_a(true));
        first && second
    });
    assert!(handled, "処理した");
    assert_eq!(f.hover(), (Some(name("a")), Some(0)));
    assert_eq!(hover_injections(&events), 1, "同値は注入しない: {events:?}");
    assert!(f.shell().is_empty(), "シェルへの移動は 0 件");
}

/// 箱の中で行の上でない移動は滞在を記録してからシェルへ送る（要件 9.1・9.7）。行から外れたら
/// 強調を 1 度だけ外し、行の上でない移動を続けても注入しない（Keep）。
#[test]
fn move_over_box_body_records_hover_and_reaches_the_shell() {
    let mut f = Fixture::new();
    assert!(f.moved(ROW, on_a(true)));
    let (_, events) = capture_logs(|| {
        f.moved(BODY, on_a(true));
        f.moved((52, 41), on_a(true));
    });
    assert_eq!(f.hover(), (Some(name("a")), None));
    assert_eq!(hover_injections(&events), 1, "外すのは 1 度: {events:?}");
    let moves = f
        .shell()
        .into_iter()
        .filter(|m| matches!(m, KanadeMsg::Mouse(m) if m.kind == MouseEventKind::Move))
        .count();
    assert!(moves >= 1, "行の上でない移動はシェルへ届く");
}

/// 別の箱へ移ると前の箱の強調を外し、箱の外へ出ると滞在を無しにする（要件 6.11・8.2）。
#[test]
fn move_to_another_box_and_outside() {
    let mut f = Fixture::new();
    f.moved(ROW, on_a(true));
    let (_, events) = capture_logs(|| f.moved((250, 10), on_b()));
    assert_eq!(f.hover(), (Some(name("b")), None));
    assert_eq!(hover_injections(&events), 1, "前の箱の強調を外す");
    assert!(!f.moved((500, 400), BoxPoint::default()));
    assert_eq!(f.hover(), (None, None));
}

/// 箱の外の移動は本 spec の前と同じ（本物のハンドラ・写しが空の文字の層・要件 9.2）。
#[test]
fn move_outside_boxes_is_unchanged() {
    let mut f = Fixture::new().with_emo2();
    let ev = Phase::Bubble(pointer(BODY, DoubleClick::None));
    assert!(on_char_pointer_moved(&mut f.world, f.entity, f.entity, &ev));
    assert!(matches!(
        f.shell()[..],
        [KanadeMsg::Mouse(ref m)] if m.kind == MouseEventKind::Move && (m.x, m.y) == (50, 40)
    ));
    assert_eq!(f.hover(), (None, None));
}

// ---------------------------------------------------------------- 離脱

/// シェルの窓から出ると滞在を無しにし、強調も外す（`balloon_exit.rs` と同じ後始末・要件 6.11）。
#[test]
fn leaving_the_shell_window_clears_hover_and_highlight() {
    let mut f = Fixture::new();
    f.moved(ROW, on_a(true));
    let mut f = f.with_emo2();
    // 強調を外す注入は「選択肢が出ている」ときだけ（普通のバルーンと同じ hover_action）。
    f.runtime
        .borrow_mut()
        .apply_cue(&areka_sakura::contract::TalkCue {
            at: 0.0,
            actor: areka_sakura::contract::ActorKey::from("0"),
            command: areka_sakura::contract::CueCommand::Choice {
                id: "OnYes".into(),
                text: "はい".into(),
                references: Vec::new(),
            },
            duration: 0.0,
        });
    let win = f
        .world
        .spawn((CharWindowMarker { scope: 0 }, Window::default()))
        .id();
    f.world.spawn((PointerLeave, ChildOf(win)));

    let ((), events) = capture_logs(|| clear_box_hover_on_leave(&mut f.world));
    assert_eq!(f.hover(), (None, None));
    assert_eq!(hover_injections(&events), 1, "強調を外す: {events:?}");
}

// ---------------------------------------------------------------- アンカー（areka-P0-anchor-tag-canon）

/// 箱 a の行を、選択肢でなくアンカーの範囲にした写し。
fn anchor_on_a() -> BoxPoint {
    BoxPoint {
        rows: vec![anchor_row(0, 10.0, 10.0, 90.0, 20.0)],
        ..on_a(true)
    }
}

/// 文字の層にアンカーの範囲だけを載せる（選択肢は無い）。
fn with_anchor_only(f: Fixture) -> Fixture {
    for cue in anchor_cues("0") {
        f.runtime.borrow_mut().apply_cue(&cue);
    }
    let actor = areka_sakura::contract::ActorKey::from("0");
    assert!(
        !f.runtime.borrow().choice_active(&actor),
        "前提: 選択肢は無い"
    );
    f
}

/// 話している最中の箱のアンカーの単押しは、アンカーの知らせが 1 件届き、シェルへも中断へも
/// 届かない。続く 2 打目（左ダブルクリック・範囲の外）も中断にしない（要件 3.3・3.4・3.7）。
#[test]
fn box_anchor_click_while_talking_selects_without_breaking() {
    let mut f = Fixture::new().talking();
    assert!(f.pressed(ROW, DoubleClick::None, anchor_on_a()), "処理した");
    let got = f.selections();
    assert_eq!(got.len(), 1, "知らせは 1 件: {got:?}");
    assert_eq!(got[0].kind, SpanKind::Anchor);
    assert!(f.pressed(BODY, DoubleClick::Left, anchor_on_a()));
    assert!(f.selections().is_empty(), "範囲の外の 2 打目は知らせない");
    assert!(f.shell().is_empty(), "シェルへは 0 件");
    assert!(f.break_rx.try_iter().next().is_none(), "中断もしない");
    assert!(f.lifecycle_rx.try_iter().next().is_none(), "隠しもしない");
}

/// 右ボタンの押下は、箱のアンカーの上でも選択にしない（要件 3.8）。
#[test]
fn right_press_on_box_anchor_is_not_a_selection() {
    let mut f = Fixture::new();
    let state = PointerState {
        left_down: false,
        right_down: true,
        ..pointer(ROW, DoubleClick::None)
    };
    assert!(!press_with_point(&mut f.world, 0, &state, anchor_on_a()));
    assert!(f.selections().is_empty());
}

/// 座標の写しの「押せる範囲があるか」は、アンカーだけでも真（要件 3.7）。
#[test]
fn read_point_is_active_with_only_an_anchor() {
    let f = with_anchor_only(Fixture::new());
    let point = read_point(&f.runtime, 0, 5.0, 5.0).expect("借りられる");
    assert!(point.active);
}

/// シェルの窓から出ると、箱のアンカーの強調も外す（要件 3.2・3.7）。
#[test]
fn leaving_the_shell_window_clears_an_anchor_highlight() {
    let mut f = Fixture::new();
    assert!(f.moved(ROW, anchor_on_a()), "アンカーの上の移動は強調する");
    assert_eq!(f.hover(), (Some(name("a")), Some(0)));
    let mut f = with_anchor_only(f.with_emo2());
    let win = f
        .world
        .spawn((CharWindowMarker { scope: 0 }, Window::default()))
        .id();
    f.world.spawn((PointerLeave, ChildOf(win)));

    let ((), events) = capture_logs(|| clear_box_hover_on_leave(&mut f.world));
    assert_eq!(f.hover(), (None, None));
    assert_eq!(hover_injections(&events), 1, "強調を外す: {events:?}");
}

// ---------------------------------------------------------------- 前段の呼び出しの位置

/// 写しの借用の失敗の記録（前段が走ったことの印）を数える。
fn snapshot_borrow_failures(events: &[crate::placement::test_support::LogEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            e.field_str("event") == Some("balloon_runtime_borrow_failed")
                && e.message().contains("箱の写し")
        })
        .count()
}

/// 本物のハンドラが前段を呼ぶ: 文字の層を借りたままにすると、前段は写しを取れずに記録を 1 行
/// 残して素通りする。移動はそのままシェルへ届き、押下は単押しでも前段を通る（単押しの早期の
/// 戻りより前に前段がある＝箱の選択肢の単押しでの確定が届く・要件 8.3・8.5）。
#[test]
fn real_handlers_call_the_prelude_before_the_existing_path() {
    let mut f = Fixture::new().with_emo2();
    let runtime = Rc::clone(&f.runtime);
    let _held = runtime.borrow_mut();

    let ((), events) = capture_logs(|| {
        let ev = Phase::Bubble(pointer(BODY, DoubleClick::None));
        on_char_pointer_moved(&mut f.world, f.entity, f.entity, &ev);
    });
    assert_eq!(snapshot_borrow_failures(&events), 1, "移動: {events:?}");
    assert!(
        matches!(f.shell()[..], [KanadeMsg::Mouse(ref m)] if m.kind == MouseEventKind::Move),
        "移動は素通りしてシェルへ届く"
    );

    let (pressed, events) = capture_logs(|| {
        let ev = Phase::Bubble(pointer(ROW, DoubleClick::None));
        on_char_pointer_pressed(&mut f.world, f.entity, f.entity, &ev)
    });
    assert!(!pressed, "単押しは今までどおり何も送らない");
    assert_eq!(snapshot_borrow_failures(&events), 1, "押下: {events:?}");
    assert!(f.shell().is_empty());
}

// ---------------------------------------------------------------- 実機の判定の記録

fn lines_of<'a>(
    events: &'a [crate::placement::test_support::LogEvent],
    event: &str,
) -> Vec<&'a crate::placement::test_support::LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// 箱の中の押下は結論を 1 行だけ残す（箱の中の単押しを箱の外の押下とログで見分ける）。
/// 箱の外の押下は残さない。
#[test]
fn box_press_logs_the_verdict_once() {
    use crate::placement::test_support::ExpectField;
    let mut f = Fixture::new();
    let ((), events) = capture_logs(|| {
        f.pressed(BODY, DoubleClick::None, on_a(false));
        f.pressed(BODY, DoubleClick::None, BoxPoint::default());
    });
    let lines = lines_of(&events, "box_press");
    assert_eq!(lines.len(), 1, "箱の中の押下 1 回で 1 行: {events:?}");
    assert_eq!(lines[0].expect_field("verdict"), "ShellOp");
    assert_eq!(lines[0].field_str("box"), Some("a"));
    assert_eq!(lines[0].expect_field("selected_now"), "false");
}

/// 滞在の箱が替わったときだけ 1 行残す（同じ箱の中の移動では残さない）。
#[test]
fn box_hover_change_logs_only_on_change() {
    let mut f = Fixture::new();
    let ((), events) = capture_logs(|| {
        f.moved(BODY, on_a(true));
        f.moved((52, 41), on_a(true));
        f.moved((250, 10), on_b());
        f.moved((500, 400), BoxPoint::default());
    });
    assert_eq!(
        lines_of(&events, "box_hover_changed").len(),
        3,
        "a へ・b へ・外へ: {events:?}"
    );
}

// ---------------------------------------------------------------- 選択の記録と kanade への伝言

/// 箱 a の行を左の単押しで押し、その間の記録と、知らせの取り出しが kanade へ送った伝言を返す。
fn press_row_and_drain(
    point: BoxPoint,
) -> (
    Vec<crate::placement::test_support::LogEvent>,
    Vec<KanadeMsg>,
) {
    use crate::input_events::choice_drain::{drain_choice_selections, wire_choice_drain};
    let mut f = Fixture::new();
    let (kanade_tx, kanade_rx) = mpsc::channel();
    wire_choice_drain(&mut f.world, kanade_tx);
    let (handled, events) = capture_logs(|| f.pressed(ROW, DoubleClick::None, point));
    assert!(handled, "処理した");
    drain_choice_selections(&mut f.world);
    (events, kanade_rx.try_iter().collect())
}

/// 発行の記録 1 行の欄（名前と値）。選択肢とアンカーで違うのは `event` だけ。
fn selected_fields(event: &str) -> BTreeMap<&'static str, String> {
    BTreeMap::from([
        ("event", format!("{event:?}")),
        ("scope", "0".to_owned()),
        ("id", "q0".to_owned()),
        ("label", "label0".to_owned()),
        ("references_len", "0".to_owned()),
        ("box", "\"a\"".to_owned()),
        (
            "message",
            "選択確定: ChoiceSelection を発行（箱）".to_owned(),
        ),
    ])
}

fn fields_of(event: &crate::placement::test_support::LogEvent) -> BTreeMap<&str, String> {
    event
        .fields_map()
        .into_iter()
        .map(|(name, value)| (name, value.to_owned()))
        .collect()
}

/// 箱のアンカーの押下は `anchor_selected` を 1 行だけ残し（箱の印つき）、`choice_selected` は
/// 残さない。知らせはアンカーの伝言として kanade へ届き、中身は当たった行のまま
/// （areka-P0-anchor-tag-canon 要件 4.11・4.12）。
#[test]
fn box_anchor_press_records_anchor_selected_and_reaches_kanade_as_an_anchor() {
    let (events, msgs) = press_row_and_drain(anchor_on_a());
    let lines = lines_of(&events, "anchor_selected");
    assert_eq!(lines.len(), 1, "発行 1 回で 1 行: {events:?}");
    assert_eq!(lines[0].level, tracing::Level::INFO);
    assert_eq!(fields_of(lines[0]), selected_fields("anchor_selected"));
    assert!(
        lines_of(&events, "choice_selected").is_empty(),
        "選択肢の記録は残さない: {events:?}"
    );
    let expected = areka_kanade::AnchorInput {
        id: "q0".to_owned(),
        text: "label0".to_owned(),
        scope: 0,
        references: Vec::new(),
    };
    assert!(
        matches!(&msgs[..], [KanadeMsg::Anchor(a)] if *a == expected),
        "アンカーの伝言が 1 件"
    );
}

/// 箱の選択肢の押下の記録と伝言は今までどおり（`choice_selected` が 1 行・欄も同じ・選択肢の伝言）。
#[test]
fn box_choice_press_still_records_choice_selected_and_reaches_kanade_as_a_choice() {
    let (events, msgs) = press_row_and_drain(on_a(true));
    let lines = lines_of(&events, "choice_selected");
    assert_eq!(lines.len(), 1, "発行 1 回で 1 行: {events:?}");
    assert_eq!(lines[0].level, tracing::Level::INFO);
    assert_eq!(fields_of(lines[0]), selected_fields("choice_selected"));
    assert!(lines_of(&events, "anchor_selected").is_empty());
    assert!(
        matches!(&msgs[..], [KanadeMsg::Choice(c)] if c.id == "q0" && c.label == "label0"),
        "選択肢の伝言が 1 件"
    );
}

/// 知らせの受け口が無くなっていて送れなかったら、種類の名前で error を 1 行残し、選択の記録は
/// 残さない（選択肢は今までの名前のまま・アンカーは選択肢の名前を使わない）。
#[test]
fn box_selection_send_failure_is_recorded_under_the_kind() {
    for (point, failed, other) in [
        (
            on_a(true),
            "choice_selection_send_failed",
            "anchor_selection_send_failed",
        ),
        (
            anchor_on_a(),
            "anchor_selection_send_failed",
            "choice_selection_send_failed",
        ),
    ] {
        let mut f = Fixture::new();
        f.world.remove_non_send::<ChoiceSelectionInbox>();
        let (_, events) = capture_logs(|| f.pressed(ROW, DoubleClick::None, point));
        let errors = lines_of(&events, failed)
            .into_iter()
            .filter(|e| e.level == tracing::Level::ERROR)
            .count();
        assert_eq!(errors, 1, "{failed} の error が 1 行: {events:?}");
        assert!(lines_of(&events, other).is_empty(), "{other}: {events:?}");
        assert!(
            lines_of(&events, "choice_selected").is_empty()
                && lines_of(&events, "anchor_selected").is_empty(),
            "送れなかった押下は選択の記録を残さない: {events:?}"
        );
    }
}
