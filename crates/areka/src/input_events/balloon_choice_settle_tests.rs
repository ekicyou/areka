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
