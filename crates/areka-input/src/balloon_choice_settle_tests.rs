// =============================================================================
// 押下の後の済み
//
// 選択を送れた押下の後に呼ぶ `settle_after_selection` の判断分岐だけを固定する: 種類が選択肢なら
// 場所の選択肢が済む・アンカーなら済まない・文字の層を借りられなければ `error!` 1 行で何もしない。
// 押下の口（`TextLayerRuntime::settle_choices_at`）は選択待ちに入った後だけ効くので、実の文字の線
// （`spawn_emo_text` の UI ドレイン）へ選択待ちの始まりを積んで、このスレッドの待ち行列を汲んでから押す。
// =============================================================================

use std::cell::RefCell;
use std::rc::Rc;

use areka_emo_text::actor::{TextLayerRuntime, present_frame, spawn_emo_text};
use areka_emo_text::place::PlaceKey;
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::SpanKind;
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf::executor::{FilterResult, JoinHandle, MessageLoop};

use super::settle_after_selection;
use super::test_support::{anchor_cues, capture_logs, empty_runtime};

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
    let rt = empty_runtime();
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
    let (mut sink, join) = spawn_emo_text(Rc::clone(&rt));
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
// 押下のハンドラから済みまでの通し（実 GPU）
//
// 実 GPU で提示して当たり行を作り、本番の押下のハンドラ（`on_balloon_pointer_pressed`）で選択肢の行を
// 押す。1 回目は選択を送って INFO `choice_selected` を 1 行残し、その場所の選択肢が済む。同じ座標の
// 2 回目は選択を作らず WARN を出さない。
// 箱の押下のハンドラ（`shell_box_handler.rs`）は同じ `settle_after_selection` を呼び、箱の場所の
// 済んだ行は文字の層の実 GPU の檻（`areka-emo-text` の `actor_choice_settle_tests.rs`）が固定する。
// =============================================================================

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use super::on_balloon_pointer_pressed;
use super::test_support::{gpu_balloon_world, headless_runtime_wiring, insert_wiring, left_press};

#[test]
fn a_pressed_choice_row_settles_and_pressing_it_again_selects_nothing_without_warn() {
    // SAFETY: 描画資源に要る COM の MTA 初期化（既初期化の S_FALSE は無視・テストスレッドごと）。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let _gpu = areka_runtime::spine::GpuPermit::take();
    let (rt, _sink, _join) = runtime_in_choice_wait();
    let actor = ActorKey::from(ACTOR);

    let (mut world, window) = gpu_balloon_world(&rt, 1.0, (120, 60), (0, 0));
    let row = rt
        .borrow()
        .choice_hit_rows(&actor)
        .iter()
        .find(|r| r.kind == SpanKind::Choice)
        .cloned()
        .expect("前提: 選択肢の当たり行がある");
    let x = ((row.rect.left + row.rect.right) / 2.0) as i32;
    let y = ((row.rect.top + row.rect.bottom) / 2.0) as i32;

    world.insert_non_send(headless_runtime_wiring(Rc::clone(&rt)));
    let rx = insert_wiring(&mut world);

    // 1 回目: 選択を送り、その場所の選択肢が済む（アンカーは押せるまま）。
    let logs = capture_logs(|| {
        assert!(on_balloon_pointer_pressed(
            &mut world,
            window,
            window,
            &left_press(x, y)
        ));
    });
    let selected: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("level=INFO") && l.contains("choice_selected"))
        .collect();
    assert_eq!(selected.len(), 1, "INFO choice_selected が 1 行: {logs:?}");
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
