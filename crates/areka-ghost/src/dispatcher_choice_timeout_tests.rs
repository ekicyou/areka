use super::choice_tests::close_and_join;
use super::*;
use crate::test_support::{RecordingSink, test_system_vars};
use dola::time::testing::{at_ms, delta_ms};
use std::sync::mpsc;
use std::time::Duration;

// ── `\![set,choicetimeout,…]` の台本から kanade の入口までの値 ──

/// 選択待ちの台本に時間の指定を足したもの（時間の欄を `value` で差し替える）。
///
/// 区切り（Choice・Barrier）の時刻は 0.35 秒で、指定の有無で変わらない。
fn script_with_choice_timeout(value: &str) -> String {
    format!(r"\s[10]hello\_w[100]\q[選択A,targetA]\![set,choicetimeout,{value}]\e")
}

/// 指定 `1234` の台本を dispatcher に流すと、kanade への選択待ちの知らせは表示の終わり
/// `1_350`（起点の最初の刻み 1_000＋区切り 0.35 s）・指令 `Some` で届く。
///
/// 対になるテストは kanade の `choice_timeout_directive_1234_fires_exactly_at_display_end_plus_1234`
/// （境の値 `1234`）。片方だけ境の値を変えたら、もう片方も合わせること。
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
    // 最初の刻み 1_000 が台本の起点になり、1_500 で区切り 0.35 に届いて選択待ちになる。
    tx.send(DispatcherMsg::Tick { now: at_ms(1_000) })
        .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick { now: at_ms(1_500) })
        .expect("send Tick(base+500ms)");

    match kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("区切りで ChoiceWaiting が kanade へ届くべき")
    {
        KanadeMsg::ChoiceWaiting {
            talk_id: got,
            choice_ids,
            display_end,
            timeout_directive,
        } => {
            assert_eq!(got, talk_id);
            assert_eq!(choice_ids, vec!["targetA".to_string()]);
            assert_eq!(
                display_end,
                at_ms(1_350),
                "表示の終わりは起点 1_000＋区切り 0.35s（tick 時刻 1_500 ではない）"
            );
            assert_eq!(
                timeout_directive,
                Some(delta_ms(1_234)),
                "台本の `1234`（ミリ秒）が長さの指令として kanade の入口まで届く"
            );
        }
        _ => unreachable!("区切りで最初に届くのは ChoiceWaiting"),
    }

    close_and_join(&tx, handle);
}
