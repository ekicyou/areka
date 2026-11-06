use super::choice_tests::close_and_join;
use super::*;
use crate::test_support::{RecordingSink, test_system_vars};
use std::sync::mpsc;
use std::time::Duration;

// ── task 3.2: `\![set,choicetimeout,…]` の台本から kanade の入口までの値
//    （spec areka-P0-choice-timeout-directive・要件 9.4／9.5）。読めない値の警告は
//    areka-sakura の `unreadable_choice_timeout_warns_once_readable_none` が縛る ──

/// 選択待ちの台本に時間の指定を足したもの（時間の欄を `value` で差し替える）。
///
/// compile 後（アンカー 0）: `hello`@0（D=0.25）／Wait@0.25（`\_w[100]`=0.1）／Choice@0.35
/// （id=targetA）／Barrier@0.35。区切りの時刻は 0.35 で、指定の有無で変わらない。
fn script_with_choice_timeout(value: &str) -> String {
    format!(r"\s[10]hello\_w[100]\q[選択A,targetA]\![set,choicetimeout,{value}]\e")
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
        None,
    );

    let talk_id = TalkId(951);
    tx.send(DispatcherMsg::Start(StartTalk::new(
        talk_id,
        script_with_choice_timeout("1234"),
    )))
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

    close_and_join(&tx, handle);
}
