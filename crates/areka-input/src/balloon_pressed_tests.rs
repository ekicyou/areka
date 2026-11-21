//! 普通のバルーンの窓の矢印の押下の檻。
//!
//! 手動の位置（自動の 1 行上）にいる文字の層を実 GPU で作り（`manual_balloon_runtime`）、矢印の絵を
//! 入れて提示し直して上下の矢印を出す。押下は印の面の entity を送り手にして `dispatch_pointer_events`
//! で配る（印の面の上の押下が窓の受け口へ届くこと）。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use areka_emo_present::balloon_marker::{MarkerImage, MarkerPair};
use areka_emo_text::actor::{TextLayerRuntime, present_frame};
use areka_emo_text::place::PlaceKey;
use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir};
use areka_emo_text::state::SpanKind;
use areka_kanade::KanadeMsg;
use areka_sakura::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::world::World;
use wintf::ecs::pointer::{DoubleClick, OnPointerPressed, PointerState, dispatch_pointer_events};
use wintf::ecs::{Point, Window};

use super::super::test_support::{
    capture_logs, headless_runtime_wiring, insert_wiring, left_press, runtime_with_active_choice,
};
use super::super::{BalloonWiring, ChoiceSelection, dispatch_selection};
use super::on_balloon_pointer_pressed;
use crate::user_break::{UserBreakWiring, drain_no_user_break_signals};
use areka_placement::spawn::BalloonWindowMarker;
use areka_runtime::frame::balloon_runtime_test_support::manual_balloon_runtime;
use areka_runtime::talk_lifecycle::TalkLifecycleSignal;
use areka_runtime::user_break_cue::NoUserBreakSignal;

/// 既定の置き場（120×40 の絵: 上 (100, 15)・下 (100, 25)）の矢印の不透明な点。
const UP: (i32, i32) = (105, 17);
const DOWN: (i32, i32) = (105, 27);
/// 上の矢印の α 100（< 0.5）の列。
const UP_TRANSPARENT: (i32, i32) = (101, 17);
/// 矢印の無い所。
const NO_ARROW: (i32, i32) = (10, 20);

/// 10×5 の矢印。左の 2 列だけ α 100、残りは α 255（乗算済み BGRA）。
fn arrow() -> Arc<MarkerImage> {
    let mut bgra = [255u8; 4].repeat(10 * 5);
    for y in 0..5 {
        bgra[y * 40..y * 40 + 8].copy_from_slice(&[100; 8]);
    }
    Arc::new(MarkerImage {
        width: 10,
        height: 5,
        bgra,
    })
}

/// 押下の配送の道具一式。
struct Rig {
    world: World,
    runtime: Rc<RefCell<TextLayerRuntime>>,
    /// 印の面（窓の子）。押下の送り手にする。
    marker: Entity,
    selections: Receiver<ChoiceSelection>,
    flag_tx: mpsc::Sender<NoUserBreakSignal>,
    lifecycle: Receiver<TalkLifecycleSignal>,
    kanade: Receiver<KanadeMsg>,
    /// GPU の装置の許可。World の装置より後に返すので欄の最後。
    _permit: areka_runtime::spine::GpuPermit,
}

/// 上下の矢印が出ている普通のバルーン（scope 0）の窓と、その受け口・結線。
fn rig() -> Rig {
    let (permit, mut world, runtime) = manual_balloon_runtime("0");
    let actor = ActorKey::from("0");
    {
        let mut rt = runtime.borrow_mut();
        rt.set_balloon_markers(
            actor.clone(),
            MarkerPair {
                up: Some(arrow()),
                down: Some(arrow()),
            },
        );
        present_frame(&mut rt, &mut world, 100.0).expect("矢印を出す提示");
    }
    let marker = runtime
        .borrow()
        .marker_surface(&actor)
        .and_then(|s| s.window_child())
        .expect("印の面");
    let window = world.get::<ChildOf>(marker).expect("窓の子").parent();
    world.entity_mut(window).insert((
        Window::default(),
        BalloonWindowMarker { scope: 0 },
        OnPointerPressed(on_balloon_pointer_pressed),
    ));
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    let selections = insert_wiring(&mut world);
    let (flag_tx, flag_rx) = mpsc::channel();
    let (lifecycle_tx, lifecycle) = mpsc::channel();
    let (kanade_tx, kanade) = mpsc::channel();
    world.insert_non_send(UserBreakWiring::new(flag_rx, lifecycle_tx, kanade_tx));
    Rig {
        world,
        runtime,
        marker,
        selections,
        flag_tx,
        lifecycle,
        kanade,
        _permit: permit,
    }
}

impl Rig {
    /// 台詞の始まり（話している最中にする）。
    fn start_talking(&mut self) {
        self.flag_tx.send(NoUserBreakSignal::TalkStarted).unwrap();
        drain_no_user_break_signals(&mut self.world);
    }

    /// 印の面を送り手にして左押下を 1 回配り、記録を返す。
    fn press(&mut self, (x, y): (i32, i32), double_click: DoubleClick) -> Vec<String> {
        self.world.entity_mut(self.marker).insert(PointerState {
            client_point: Point { x, y },
            left_down: true,
            double_click,
            ..Default::default()
        });
        let world = &mut self.world;
        capture_logs(|| dispatch_pointer_events(world))
    }

    /// 古い側へ 1 行送ったときの変化（手動の位置が行 0 なら `Unchanged`）。
    fn older_by_one(&self) -> ScrollChange {
        self.runtime.borrow_mut().scroll_manual_at(
            &PlaceKey::balloon(&ActorKey::from("0")),
            ScrollDir::Older,
            1,
        )
    }

    /// 新しい側へ 1 行送ったときの変化（自動の 1 行上なら自動へ戻る `Stopped`）。
    fn newer_by_one(&self) -> ScrollChange {
        self.runtime.borrow_mut().scroll_manual_at(
            &PlaceKey::balloon(&ActorKey::from("0")),
            ScrollDir::Newer,
            1,
        )
    }

    /// 選択肢・中断のどこへも流れていない。
    fn nothing_sent(&self) -> bool {
        self.selections.try_recv().is_err()
            && self.lifecycle.try_recv().is_err()
            && self.kanade.try_recv().is_err()
    }
}

/// 送りが効いていない（押下前の「自動の 1 行上」のまま）。
fn unmoved(rig: &Rig) -> bool {
    matches!(rig.newer_by_one(), ScrollChange::Stopped(_))
}

/// 話し終えた後、印の面の上の上の矢印の押下は窓の受け口へ届き、古い側へ 2 行送って消費する
/// 左ダブルクリックでも中断・選択肢の確定へ流さない。
#[test]
fn finished_up_arrow_press_scrolls_older_and_is_consumed() {
    let mut rig = rig();
    let logs = rig.press(UP, DoubleClick::Left);
    assert!(
        logs.iter().any(|l| l.contains("balloon_arrow_pressed")),
        "印の面の上の押下が受け口に届き、矢印として取られる: {logs:?}"
    );
    assert!(
        !logs
            .iter()
            .any(|l| l.contains("choice_click_rejected") || l.contains("balloon_break_detected")),
        "選択肢・中断の判断へ流れない: {logs:?}"
    );
    assert!(rig.nothing_sent());
    // 自動の 1 行上から 2 行古い側へ＝自動の 3 行上。新しい側へ 2 回戻しても手動のまま。
    assert_eq!(rig.newer_by_one(), ScrollChange::Moved);
    assert_eq!(rig.newer_by_one(), ScrollChange::Moved);
    assert!(
        matches!(rig.newer_by_one(), ScrollChange::Stopped(_)),
        "送ったのはちょうど 2 行"
    );
}

/// 話し終えた後の下の矢印の押下は新しい側へ送る（自動の 1 行上から自動へ戻る）。
#[test]
fn finished_down_arrow_press_scrolls_newer() {
    let mut rig = rig();
    let logs = rig.press(DOWN, DoubleClick::None);
    assert!(
        logs.iter().any(|l| l.contains("balloon_arrow_pressed")),
        "{logs:?}"
    );
    assert!(rig.nothing_sent());
    assert_eq!(
        rig.newer_by_one(),
        ScrollChange::Unchanged,
        "自動へ戻っている"
    );
}

/// 話している最中の矢印の押下は普通のクリック（中断の入口へ流れ、送らない）。
#[test]
fn talking_arrow_press_flows_to_the_usual_click() {
    let mut rig = rig();
    rig.start_talking();
    let logs = rig.press(UP, DoubleClick::Left);
    assert!(
        !logs.iter().any(|l| l.contains("balloon_arrow_pressed")),
        "{logs:?}"
    );
    assert!(
        logs.iter().any(|l| l.contains("balloon_break_detected")),
        "利用者の中断の入口まで流れる: {logs:?}"
    );
    assert!(unmoved(&rig), "送らない");
}

/// 矢印の絵の透けた画素と矢印の無い所は普通のクリック。
#[test]
fn transparent_pixel_and_no_arrow_flow_to_the_usual_click() {
    for point in [UP_TRANSPARENT, NO_ARROW] {
        let mut rig = rig();
        let logs = rig.press(point, DoubleClick::Left);
        assert!(
            !logs.iter().any(|l| l.contains("balloon_arrow_pressed")),
            "{point:?}: {logs:?}"
        );
        assert!(
            logs.iter().any(|l| l.contains("choice_click_rejected"))
                && logs.iter().any(|l| l.contains("balloon_break_detected")),
            "{point:?}: 選択肢・中断の判断へ流れる: {logs:?}"
        );
        assert!(unmoved(&rig), "{point:?}: 送らない");
    }
}

/// 話し終えたかの判断: 持ち物が無い（結線前）は話していない扱い・台詞の始まりで偽・終わりで真。
#[test]
fn finished_talking_follows_the_talk_signals() {
    let mut world = World::new();
    assert!(super::finished_talking(&world), "結線前は話していない扱い");
    let (flag_tx, flag_rx) = mpsc::channel();
    world.insert_non_send(UserBreakWiring::new(
        flag_rx,
        mpsc::channel().0,
        mpsc::channel().0,
    ));
    assert!(super::finished_talking(&world));
    flag_tx.send(NoUserBreakSignal::TalkStarted).unwrap();
    drain_no_user_break_signals(&mut world);
    assert!(!super::finished_talking(&world));
    flag_tx.send(NoUserBreakSignal::TalkEnded).unwrap();
    drain_no_user_break_signals(&mut world);
    assert!(super::finished_talking(&world));
}

/// 上の矢印を押し続けると行 0 で止まる（古い側の端）。
#[test]
fn repeated_up_arrow_stops_at_line_zero() {
    let mut rig = rig();
    for _ in 0..8 {
        rig.press(UP, DoubleClick::None);
    }
    assert_eq!(rig.older_by_one(), ScrollChange::Unchanged, "行 0 で止まる");
}

/// 受け口の無い送り出しは、選択肢でもアンカーでも ERROR を 1 行だけ出し、WARN を出さない。
#[test]
fn dispatch_selection_without_receiver_logs_one_error_and_no_warn() {
    for (kind, failed) in [
        (SpanKind::Choice, "choice_selection_send_failed"),
        (SpanKind::Anchor, "anchor_selection_send_failed"),
    ] {
        let (tx, rx) = mpsc::channel();
        drop(rx);
        let wiring = BalloonWiring::new(tx);
        let sel = ChoiceSelection {
            kind,
            id: "id".into(),
            label: "label".into(),
            scope: 0,
            references: Vec::new(),
        };
        let mut sent = true;
        let logs = capture_logs(|| {
            sent = dispatch_selection(&wiring, None, &PlaceKey::balloon(&ActorKey::from("0")), sel)
        });
        assert!(!sent, "{kind:?}");
        let errors = logs
            .iter()
            .filter(|l| l.contains("level=ERROR") && l.contains(failed))
            .count();
        assert_eq!(errors, 1, "{kind:?}: {logs:?}");
        assert!(
            !logs.iter().any(|l| l.contains("level=WARN")),
            "{kind:?}: {logs:?}"
        );
    }
}

/// 文字の層を借りられないと、矢印の押下と選択の写しの借用の失敗を ERROR で 1 行ずつ残し、選択を
/// 送らない（panic しない）。
#[test]
fn press_borrow_failure_is_recorded_for_the_arrow_and_the_snapshot() {
    let mut world = World::new();
    let e = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let runtime = runtime_with_active_choice("0");
    world.insert_non_send(headless_runtime_wiring(Rc::clone(&runtime)));
    let selections = insert_wiring(&mut world);

    let held = runtime.borrow_mut();
    let mut pressed = true;
    let logs = capture_logs(|| {
        pressed = on_balloon_pointer_pressed(&mut world, e, e, &left_press(10, 20))
    });
    drop(held);

    assert!(!pressed);
    let failures = |what: &str| {
        logs.iter()
            .filter(|l| {
                l.contains("level=ERROR")
                    && l.contains("balloon_runtime_borrow_failed")
                    && l.contains(what)
            })
            .count()
    };
    assert_eq!(failures("矢印の押下"), 1, "{logs:?}");
    assert_eq!(failures("click スナップショット"), 1, "{logs:?}");
    assert!(selections.try_recv().is_err());
}
