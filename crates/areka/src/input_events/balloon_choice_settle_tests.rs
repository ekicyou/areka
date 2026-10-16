// =============================================================================
// 押下の後の済み（areka-P0-balloon-text-bugs task 4.6・design「押下の後の済み」）
//
// 選択を送れた押下の後に呼ぶ `settle_after_selection` の判断分岐だけを固定する: 種類が選択肢なら
// 場所の選択肢が済む・アンカーなら済まない・文字の層を借りられなければ `error!` 1 行で何もしない。
// 押下の口（`TextLayerRuntime::settle_choices_at`）は選択待ちに入った後だけ効くので、実の文字の線
// （`spawn_emo_text` の UI ドレイン）へ選択待ちの始まりを積んで、このスレッドの待ち行列を汲んでから押す。
// =============================================================================

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::place::PlaceKey;
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::{SpanKind, TextLayerConfig};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::executor::{FilterResult, JoinHandle, MessageLoop};

use super::settle_after_selection;
use super::test_support::{anchor_cues, capture_logs};

const ACTOR: &str = "0";

/// このスレッドの待ち行列に積まれた伝言を全部汲んでから抜ける（`spine.rs` の `pump_until_idle` と同じ定石）。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// 普通のバルーンに選択肢 1 つとアンカー 1 つを載せ、選択待ちに入った文字の層。
///
/// 文字の線の受け口は持って返す（落とすとトークの終わりが積まれる）。
fn runtime_in_choice_wait() -> (Rc<RefCell<TextLayerRuntime>>, EmoTextSink, JoinHandle<()>) {
    let rt = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    rt.borrow_mut().apply_cue(&TalkCue {
        at: 0.0,
        actor: ActorKey::from(ACTOR),
        command: CueCommand::Choice {
            id: "OnYes".into(),
            text: "はい".into(),
            references: Vec::new(),
        },
        duration: 0.0,
    });
    for cue in anchor_cues(ACTOR) {
        rt.borrow_mut().apply_cue(&cue);
    }
    let (mut sink, join) =
        spawn_emo_text(Rc::clone(&rt)).expect("このスレッドで文字の線を起こせる");
    sink.choice_wait_began();
    pump_until_idle();
    let actor = ActorKey::from(ACTOR);
    assert!(
        rt.borrow().choice_active(&actor) && rt.borrow().hit_active(&actor),
        "前提: 済んでいない選択肢とアンカーが出ている"
    );
    (rt, sink, join)
}

fn balloon() -> PlaceKey {
    PlaceKey::balloon(&ActorKey::from(ACTOR))
}

/// 種類が選択肢なら、その場所の選択肢が済む（選択肢の問いが偽）。アンカーは押せる範囲に残る。
#[test]
fn settle_after_a_choice_settles_the_place() {
    let (rt, _sink, _join) = runtime_in_choice_wait();
    let actor = ActorKey::from(ACTOR);

    settle_after_selection(&rt, &balloon(), SpanKind::Choice);

    assert!(!rt.borrow().choice_active(&actor), "選択肢は済む");
    assert!(rt.borrow().hit_active(&actor), "アンカーは押せるまま残る");
}

/// 種類がアンカーなら済みにしない（同じ場所の選択肢は済んでいないまま）。
#[test]
fn settle_after_an_anchor_leaves_the_choices_live() {
    let (rt, _sink, _join) = runtime_in_choice_wait();

    settle_after_selection(&rt, &balloon(), SpanKind::Anchor);

    assert!(
        rt.borrow().choice_active(&ActorKey::from(ACTOR)),
        "アンカーの押下では選択肢を済みにしない"
    );
}

/// 文字の層を借りられなければ `error!` を 1 行残して何もしない（panic しない）。
#[test]
fn settle_after_selection_borrow_failure_logs_one_error_and_does_nothing() {
    let (rt, _sink, _join) = runtime_in_choice_wait();

    let logs = {
        let _held = rt.borrow_mut();
        capture_logs(|| settle_after_selection(&rt, &balloon(), SpanKind::Choice))
    };

    let errors: Vec<&String> = logs.iter().filter(|l| l.contains("level=ERROR")).collect();
    assert_eq!(errors.len(), 1, "error が 1 行: {logs:?}");
    assert!(
        errors[0].contains("balloon_runtime_borrow_failed"),
        "借用の失敗の名前: {logs:?}"
    );
    assert!(
        rt.borrow().choice_active(&ActorKey::from(ACTOR)),
        "借りられなかったので済みにしていない"
    );
}

// =============================================================================
// 押下のハンドラから済みまでの通し（task 4.7・実 GPU・要件 1.10・1.11）
//
// 実 GPU で提示して当たり行を作り、本番の押下のハンドラ（`on_balloon_pointer_pressed`）で選択肢の行を
// 押す。1 回目は選択を送ってその場所の選択肢が済み、同じ座標の 2 回目は選択を作らず WARN を出さない。
// 箱の押下のハンドラ（`shell_box_handler.rs`）は同じ `settle_after_selection` を呼び、箱の場所の
// 済んだ行は文字の層の実 GPU の檻（`areka-emo-text` の `actor_choice_settle_tests.rs`）が固定する。
// =============================================================================

use std::sync::mpsc;

use areka_emo_text::actor::{ResolvedBalloonText, TextSlotBinding, present_frame};
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::world::World;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::pointer::{Phase, PointerState};
use wintf::ecs::{GraphicsCore, Point, Visual, WucGraphicsResource};

use super::test_support::headless_emo2_wiring;
use super::{BalloonWiring, ChoiceSelection, on_balloon_pointer_pressed};
use crate::placement::spawn::BalloonWindowMarker;

/// 左シングルクリック（窓の client 物理 px）。
fn left_press(x: i32, y: i32) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        left_down: true,
        ..Default::default()
    })
}

/// 幾何だけのバルーン定義（既定の書体・描画範囲は画像の全域）。
fn plain_model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

#[test]
fn a_pressed_choice_row_settles_and_pressing_it_again_selects_nothing_without_warn() {
    // SAFETY: 描画資源に要る COM の MTA 初期化（既初期化の S_FALSE は無視・テストスレッドごと）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let _gpu = crate::emo2_boot::spine::GpuPermit::take();
    let (rt, _sink, _join) = runtime_in_choice_wait();
    let actor = ActorKey::from(ACTOR);

    // 普通のバルーンの窓（押下の宛先）と文字の層の差し込み口。
    let mut world = World::new();
    let window = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let slot = world.spawn((Visual::default(), ChildOf(window))).id();
    world.flush();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device"))
        .expect("WucGraphicsResource::new 失敗");
    world.insert_resource(core);
    world.insert_resource(wuc);
    let image = (120u32, 60u32);
    rt.borrow_mut().register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&plain_model(), image),
    );
    present_frame(&mut rt.borrow_mut(), &mut world, 10.0).expect("提示");
    let row = rt
        .borrow()
        .choice_hit_rows(&actor)
        .iter()
        .find(|r| r.kind == SpanKind::Choice)
        .cloned()
        .expect("前提: 選択肢の当たり行がある");
    let x = ((row.rect.left + row.rect.right) / 2.0) as i32;
    let y = ((row.rect.top + row.rect.bottom) / 2.0) as i32;

    world.insert_non_send(headless_emo2_wiring(Rc::clone(&rt)));
    let (tx, rx) = mpsc::channel::<ChoiceSelection>();
    world.insert_non_send(BalloonWiring::new(tx));

    // 1 回目: 選択を送り、その場所の選択肢が済む（アンカーは押せるまま）。
    assert!(on_balloon_pointer_pressed(
        &mut world,
        window,
        window,
        &left_press(x, y)
    ));
    let sent = rx.try_recv().expect("1 回目は選択を送る");
    assert_eq!((sent.kind, sent.id.as_str()), (SpanKind::Choice, "OnYes"));
    assert!(!rt.borrow().choice_active(&actor), "押した選択肢は済む");
    assert!(rt.borrow().hit_active(&actor), "アンカーは押せるまま");

    // 次の提示で写しを作り直しても済んだ行は戻らない。同じ座標の 2 回目は選択を作らず、WARN も出さない。
    present_frame(&mut rt.borrow_mut(), &mut world, 10.0).expect("済んだ後の提示");
    let logs = capture_logs(|| {
        assert!(
            !on_balloon_pointer_pressed(&mut world, window, window, &left_press(x, y)),
            "済んだ行の押下は選択にならない"
        );
        tracing::warn!("対照の警告");
    });
    assert!(rx.try_recv().is_err(), "2 回目は送らない");
    let warns: Vec<&String> = logs.iter().filter(|l| l.contains("level=WARN")).collect();
    assert_eq!(
        warns.len(),
        1,
        "数えられるのは対照の 1 行だけ（押下で WARN 0 行）: {logs:?}"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("choice_click_rejected") && l.contains("no_hit")),
        "当たらなかった押下として棄却される: {logs:?}"
    );
    assert!(
        !logs.iter().any(|l| l.contains("choice_selected")),
        "選択の知らせを作らない: {logs:?}"
    );
}
