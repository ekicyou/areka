use super::test_support::{RecordingSink, run_bounded, test_system_vars};
use super::*;
use crate::test_log_capture::{CapturedEvent, assert_logged_event, capture};
use std::sync::mpsc;
use std::time::Duration;
use tracing::Level;

// ── task 3.2: `\![set,choicetimeout,…]` の警告の記録と、台本から kanade の入口までの値
//    （spec areka-P0-choice-timeout-directive・要件 6.1／9.2／9.4／9.5） ──

/// 選択待ちの台本に時間の指定を足したもの（時間の欄を `value` で差し替える）。
///
/// compile 後（アンカー 0）: `hello`@0（D=0.25）／Wait@0.25（`\_w[100]`=0.1）／Choice@0.35
/// （id=targetA）／Barrier@0.35。区切りの時刻は 0.35 で、指定の有無で変わらない。
fn script_with_choice_timeout(value: &str) -> String {
    format!(r"\s[10]hello\_w[100]\q[選択A,targetA]\![set,choicetimeout,{value}]\e")
}

/// 台本を捕捉の窓の中で compile し、捕捉した記録を返す。
///
/// 窓は番兵で空振りを検出する（捕捉が働いていなければ `capture` が panic する）。
fn compile_captured(value: &str) -> Vec<CapturedEvent> {
    let instructions = areka_parsers::sakura::parse(&script_with_choice_timeout(value));
    capture(|| {
        let _ = areka_sakura::compile(&instructions, &areka_sakura::SystemVarSnapshot::default());
    })
}

/// 語彙 `choice_timeout_unreadable` の WARN（target は compile）の件数。
fn unreadable_warn_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            e.target == "areka_sakura::compile"
                && e.level == Level::WARN
                && e.event.as_deref() == Some("choice_timeout_unreadable")
        })
        .count()
}

/// **警告の記録（要件 6.1・9.2）**: 整数として読めない時間の欄（`abc`）は、compile の中で
/// WARN `choice_timeout_unreadable` をちょうど 1 行残す。読める値（`500`）では同じ語彙は 0 件。
///
/// 0 件の主張は、同じ捕捉の窓で `abc` の 1 件が見えること（正の対照）と、窓の番兵の 2 つで
/// 較正している。
#[test]
fn unreadable_choice_timeout_logs_warn_once_and_readable_value_logs_none() {
    let unreadable = compile_captured("abc");
    assert_logged_event(
        &unreadable,
        Level::WARN,
        "areka_sakura::compile",
        "choice_timeout_unreadable",
    );
    assert_eq!(
        unreadable_warn_count(&unreadable),
        1,
        "読めない指定 1 回につき WARN は 1 行"
    );

    assert_eq!(
        unreadable_warn_count(&compile_captured("500")),
        0,
        "読める値では choice_timeout_unreadable を出さない"
    );
}

/// **台本 → kanade の入口（要件 9.4 前半・9.5）**: 指定 `1234` の台本を dispatcher に流すと、
/// kanade への選択待ちの知らせは表示の終わり `1_350`（base_now 1_000＋区切り 0.35 s）・
/// 指令 `Some(1.234)` で届く。
///
/// 対になるテストは kanade の `choice_timeout_directive_1234_fires_exactly_at_display_end_plus_1234`
/// （境の値 `1234`）。kanade の期限の判定は `pub(crate)` で 1 本では貫けないため、2 本の合成で
/// 「台本から期限まで通し」を示す。片方だけ境の値を変えたら、もう片方も合わせること。
///
/// この檻は時計を区切り＋1.234 秒の先まで進めない（`Tick` は 1_000 と 1_500 だけ）ので、
/// 区切りを値で飛ばさない再生層の修正には依らない（その証拠は dola と drive の檻が担う）。
#[test]
fn script_choice_timeout_1234_reaches_kanade_with_display_end_and_directive() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![
            Box::new(RecordingSink::new()),
            Box::new(RecordingSink::new()),
        ],
        test_system_vars(),
    );

    let talk_id = TalkId(951);
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id,
        script: script_with_choice_timeout("1234"),
    }))
    .expect("send Start(choicetimeout 1234)");
    // base_now=1_000 刻印 → elapsed 0.5 で Choice@0.35・Barrier@0.35 到達（WaitingForChoice）。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_000),
    })
    .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_500),
    })
    .expect("send Tick(base+500ms)");

    match kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("区切りで ChoiceWaiting が kanade へ届くべき")
    {
        KanadeMsg::ChoiceWaiting {
            talk_id: got,
            choice_ids,
            display_end,
            timeout_directive_secs,
        } => {
            assert_eq!(got, talk_id);
            assert_eq!(choice_ids, vec!["targetA".to_string()]);
            assert_eq!(
                display_end,
                MonotonicMs(1_350),
                "表示の終わりは base_now(1_000)＋区切り 0.35s（tick 時刻 1_500 ではない）"
            );
            assert_eq!(
                timeout_directive_secs,
                Some(1.234),
                "台本の `1234`（ミリ秒）が秒の指令として kanade の入口まで届く"
            );
        }
        _ => unreachable!("区切りで最初に届くのは ChoiceWaiting"),
    }

    tx.send(DispatcherMsg::Close).expect("send Close");
    run_bounded(
        "dispatcher join after Close",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("dispatcher terminates normally after Close");
        },
    );
}
