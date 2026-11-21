// =============================================================================
// 中断の知らせが表示の側のトークの番号を運ぶテスト（audit-kanade #6）
// =============================================================================
//
// 送り出し `send_break` が、渡されたトークの番号をそのまま `KanadeMsg::UserBreak` に載せ、
// 表示の側へは今どおり「中断された」を 1 件送ることを固定する。番号が今のトークと合うかの
// 判断は kanade の側（`schedule/user_break.rs`）が持つので、ここは運ぶことだけを見る。
// 記録は見ない。

use std::sync::mpsc;

use super::*;

#[test]
fn send_break_carries_the_talk_id_it_was_given() {
    let (_flag_tx, flag_rx) = mpsc::channel();
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel();
    let (kanade_tx, kanade_rx) = mpsc::channel();
    let wiring = UserBreakWiring::new(flag_rx, lifecycle_tx, kanade_tx);

    send_break(&wiring, 2, Some(TalkId(7)));

    let to_kanade: Vec<KanadeMsg> = kanade_rx.try_iter().collect();
    assert!(
        matches!(
            to_kanade.as_slice(),
            [KanadeMsg::UserBreak {
                scope: 2,
                talk_id: Some(TalkId(7))
            }]
        ),
        "kanade へ届いた知らせの数 {}",
        to_kanade.len()
    );
    let to_display: Vec<TalkLifecycleSignal> = lifecycle_rx.try_iter().collect();
    assert_eq!(to_display, vec![TalkLifecycleSignal::UserBreak]);
}
