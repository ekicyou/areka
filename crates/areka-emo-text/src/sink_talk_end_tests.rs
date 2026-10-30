//! 文字の線のトークの終わりと選択待ちの始まりの檻（areka-P0-balloon-text-bugs 件 1・要件 1.10・1.13・4.1）。
//!
//! 実 `spawn_ui` の UI ドレイン（本テストスレッド＝pump スレッド）で固定する:
//! cue を送った複製を落とすと `TalkEnded` が cue の後に 1 回届く／登録の原本と cue を送らなかった複製は
//! 送らない／`choice_wait_began` が `ChoiceWaitBegan` として cue と同じ順で届く／`spawn_emo_text` の
//! ドレインが始まりで押下の口を開き、終わりで全部の場所を済みにして口を閉じ、次のトークの先渡しでも
//! 口を閉じる／2 つの受け口の形は両方を読み飛ばす／送れないときの記録は警告・エラー 1 行ずつ。
//!
//! 選択待ちに入ったかの欄は外から読めないので、公開の押下の口 `settle_choices_at` が効くか
//! （`choice_active` が偽になるか）で観測する。

use std::cell::RefCell;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use log_capture_kit::count_levels;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use wintf_winmsg_executor::{FilterResult, MessageLoop};

use super::{EmoTextSink, TextMsg, handle_text_msg, handle_text_msg_with};
use crate::actor::{TextLayerRuntime, spawn_emo_text};
use crate::place::PlaceKey;
use crate::state::TextLayerConfig;

fn cue(actor: &str, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

fn text(actor: &str, s: &str) -> TalkCue {
    cue(actor, CueCommand::Text(s.to_owned()))
}

fn choice(actor: &str, id: &str) -> TalkCue {
    cue(
        actor,
        CueCommand::Choice {
            id: id.to_owned(),
            text: id.to_owned(),
            references: Vec::new(),
        },
    )
}

fn balloon(actor: &str) -> PlaceKey {
    PlaceKey::balloon(&ActorKey::from(actor))
}

/// 事前に積んだ便りを全部処理してから抜ける bounded pump（`sink.rs` の檻と同じ手口）。
fn pump_until_idle() {
    // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけ（テストスレッド＝pump）。
    unsafe { PostQuitMessage(0) };
    MessageLoop::run(|_, _| FilterResult::Forward);
}

/// 4 つの受け口の形で届いた便りを全部記録するドレイン。
fn spawn_recording_drain() -> (EmoTextSink, Arc<Mutex<Vec<TextMsg>>>) {
    let records = Arc::new(Mutex::new(Vec::new()));
    let (on_cue, on_upcoming, on_mark) = (
        Arc::clone(&records),
        Arc::clone(&records),
        Arc::clone(&records),
    );
    let (tx, _detached_drain) = areka_actor::spawn_ui(
        "emo-text-talk-end-test",
        move |msg: TextMsg| -> Result<ControlFlow<()>, String> {
            handle_text_msg_with(
                msg,
                |c, _| {
                    on_cue.lock().expect("records").push(TextMsg::Cue(c));
                    Ok(())
                },
                |u| {
                    on_upcoming
                        .lock()
                        .expect("records")
                        .push(TextMsg::Upcoming(u));
                    Ok(())
                },
                |m| {
                    on_mark.lock().expect("records").push(m);
                    Ok(())
                },
            )
        },
    )
    .expect("spawn_ui on the test (pump) thread must succeed");
    (EmoTextSink::new(tx), records)
}

/// cue を送った複製を落とすと、トークの終わりがその cue の後に 1 回だけ届く。
#[test]
fn talk_end_arrives_once_after_the_cues_of_the_dropped_clone() {
    let ((), counts) = count_levels(|| {
        let (original, records) = spawn_recording_drain();
        let mut talk = original.clone();
        let (c0, c1) = (text("0", "アヒル"), text("1", "や"));
        talk.emit(c0.clone());
        talk.emit(c1.clone());
        drop(talk);
        pump_until_idle();
        assert_eq!(
            *records.lock().expect("records"),
            vec![TextMsg::Cue(c0), TextMsg::Cue(c1), TextMsg::TalkEnded],
            "終わりは cue の後に 1 回"
        );
        drop(original);
    });
    assert_eq!(
        (counts.warn, counts.error),
        (0, 0),
        "正常な終わりは記録を伴わない"
    );
}

/// 登録の原本と cue を送らなかった複製は、落ちても終わりを送らない。複製は印を持ち越さない。
#[test]
fn original_and_silent_clones_send_no_talk_end() {
    let (original, records) = spawn_recording_drain();
    let mut talk = original.clone();
    talk.emit(text("0", "前"));
    // cue を送った複製からの複製も、自分で送るまでは印が立たない。
    let silent = talk.clone();
    let silent2 = original.clone();
    drop(silent);
    drop(silent2);
    drop(original);
    pump_until_idle();
    assert_eq!(
        *records.lock().expect("records"),
        vec![TextMsg::Cue(text("0", "前"))],
        "原本・送らなかった複製は終わりを送らない"
    );
    drop(talk);
}

/// 選択待ちの始まりは cue と同じ口へ積まれ、積んだ順に（終わりより前に）届く。
#[test]
fn choice_wait_began_arrives_in_the_same_order_as_cues() {
    let (original, records) = spawn_recording_drain();
    let mut talk = original.clone();
    let (c0, c1) = (choice("0", "a"), choice("0", "b"));
    talk.emit(c0.clone());
    talk.emit(c1.clone());
    talk.choice_wait_began();
    drop(talk);
    pump_until_idle();
    assert_eq!(
        *records.lock().expect("records"),
        vec![
            TextMsg::Cue(c0),
            TextMsg::Cue(c1),
            TextMsg::ChoiceWaitBegan,
            TextMsg::TalkEnded,
        ],
        "始まりは cue と同じ線で、積んだ順に届く"
    );
    drop(original);
}

/// 2 つの受け口の形（`handle_text_msg`）は終わりと始まりを読み飛ばす（`on_cue` は呼ばれない）。
#[test]
fn two_receiver_shape_skips_talk_end_and_choice_wait_began() {
    for msg in [TextMsg::TalkEnded, TextMsg::ChoiceWaitBegan] {
        let result =
            handle_text_msg::<String>(msg, |_| panic!("on_cue must not be called for marks"));
        assert_eq!(result, Ok(ControlFlow::Continue(())));
    }
}

/// 4 つ目の受け口の失敗は `Err` で基盤へ返る（基盤が `error!`＋続行）。
#[test]
fn fourth_receiver_failure_propagates_as_err() {
    let result = handle_text_msg_with(
        TextMsg::TalkEnded,
        |_, _| -> Result<(), String> { panic!("on_cue must not be called") },
        |_| panic!("on_upcoming must not be called"),
        |_| Err("busy".to_string()),
    );
    assert_eq!(result, Err("busy".to_string()));
}

/// 送れないとき: 終わりは警告 1 行、始まりはエラー 1 行（どちらも panic しない）。
#[test]
fn undeliverable_marks_log_one_line_each() {
    let (original, _records) = spawn_recording_drain();
    original.close();
    pump_until_idle();

    let mut talk = original.clone();
    let ((), emitted) = count_levels(|| talk.emit(text("0", "届かない")));
    assert_eq!(emitted.error, 1, "前提: 停止後の emit はエラー 1 行");

    let ((), began) = count_levels(|| talk.choice_wait_began());
    assert_eq!(
        (began.warn, began.error),
        (0, 1),
        "始まりを積めなければエラー 1 行"
    );

    let ((), ended) = count_levels(|| drop(talk));
    assert_eq!(
        (ended.warn, ended.error),
        (1, 0),
        "終わりを送れなければ警告 1 行"
    );
}

// ── spawn_emo_text のドレインの結線（4.4 の口へ） ──

/// 実ドレインを起こし、スコープ 0 と 1 に選択肢を 1 つずつ届けたトークの受け口を返す。
fn drain_with_choices() -> (Rc<RefCell<TextLayerRuntime>>, EmoTextSink, EmoTextSink) {
    let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )));
    let (original, _join) =
        spawn_emo_text(Rc::clone(&runtime)).expect("spawn_emo_text on the test (pump) thread");
    let mut talk = original.clone();
    talk.emit(choice("0", "a"));
    talk.emit(choice("1", "b"));
    pump_until_idle();
    (runtime, original, talk)
}

fn active(runtime: &Rc<RefCell<TextLayerRuntime>>, actor: &str) -> bool {
    runtime.borrow().choice_active(&ActorKey::from(actor))
}

/// 始まりを受けたドレインは押下の口を開く（押下でその場所が済む・ほかの場所は残る）。
#[test]
fn drain_opens_the_press_on_choice_wait_began() {
    let (runtime, original, mut talk) = drain_with_choices();
    runtime.borrow_mut().settle_choices_at(&balloon("0"));
    assert!(active(&runtime, "0"), "前提: 始まりの前の押下は何もしない");

    talk.choice_wait_began();
    pump_until_idle();
    runtime.borrow_mut().settle_choices_at(&balloon("0"));
    assert!(!active(&runtime, "0"), "始まりの後の押下でその場所が済む");
    assert!(active(&runtime, "1"), "ほかの場所は済まない");
    drop((talk, original));
}

/// 終わりを受けたドレインは全部の場所を済みにし、押下の口を閉じる。
#[test]
fn drain_settles_every_place_and_closes_the_press_on_talk_end() {
    let ((), counts) = count_levels(|| {
        let (runtime, original, mut talk) = drain_with_choices();
        talk.choice_wait_began();
        drop(talk);
        pump_until_idle();
        assert!(!active(&runtime, "0"), "終わりでスコープ 0 が済む");
        assert!(!active(&runtime, "1"), "終わりでスコープ 1 も済む");

        // 口が閉じた: 後から足された選択肢は押下では済まない。
        runtime.borrow_mut().apply_cue(&choice("1", "c"));
        runtime.borrow_mut().settle_choices_at(&balloon("1"));
        assert!(active(&runtime, "1"), "終わりの後は押下の口が閉じている");
        drop(original);
    });
    assert_eq!(counts.error, 0, "正常な終わりの適用はエラーを伴わない");
}

/// 次のトークの先渡しを受けたドレインも押下の口を閉じる。
#[test]
fn drain_closes_the_press_on_the_next_talk_preview() {
    let (runtime, original, mut talk) = drain_with_choices();
    talk.choice_wait_began();
    talk.preview(&[]);
    pump_until_idle();
    runtime.borrow_mut().settle_choices_at(&balloon("0"));
    assert!(active(&runtime, "0"), "先渡しの後は押下の口が閉じている");
    drop((talk, original));
}
