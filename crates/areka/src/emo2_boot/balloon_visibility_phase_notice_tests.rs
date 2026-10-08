// =============================================================================
// 可視性の相が時間切れの知らせを kanade へ送る配線の檻（areka-P0-balloon-lifecycle-events task 5.1）
//
// 判断（`decide`）は親の観測表の道具（`Frame`）で 1 フレームずつ進め、相と同じく判断の後に
// [`notify_timeout`] へ知らせを渡す。送り先は置き場（`GhostSlot`）のゴーストの kanade で、
// 檻はその受け端を直に読む（要件 2.1・7.2・design「VisibilityPhase の送出」）。
//
// headless の表示層は表示が一度も確立しないので、相そのものでは時間切れに至らない
// （`balloon_visibility_phase_reappear_tests.rs` と同じ事情）。可視は観測表に書き込む。
// =============================================================================

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_kanade::KanadeMsg;
use areka_sakura::TalkId;
use tracing::Level;

use super::super::test_support::{Frame, TALK_ID, seen};
use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::placement::test_support::{ExpectField, LogEvent, capture_logs};

/// 待ち時間（秒）と前の台詞の占有終端（talk 相対秒）。
const WAIT: f64 = 2.0;
const END: f64 = 1.0;

/// 置き場にゴースト（kanade への送出端を持つか持たないか）を据えた World。
fn world_with_ghost(kanade: Option<Sender<KanadeMsg>>) -> World {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        kanade,
        PathBuf::from("balloon_visibility_phase_notice_tests"),
    ))));
    world
}

/// 受け端に届いた時間切れの知らせの番号（他のメッセージが届いたら檻の誤り）。
fn received(rx: &Receiver<KanadeMsg>) -> Vec<TalkId> {
    rx.try_iter()
        .map(|msg| match msg {
            KanadeMsg::BalloonTimeout { talk_id } => talk_id,
            _ => panic!("時間切れの知らせ以外のメッセージが届いた"),
        })
        .collect()
}

/// 相の 1 フレーム（判断 → 知らせの送出）。
fn frame(state: &mut BalloonVisibilityState, world: &World, frame: Frame) {
    let decision = frame.timeout(WAIT).run(state);
    notify_timeout(world, decision.timeout_notice);
}

/// トーク 1 を出して終わらせ、満了の直前まで進める（ここまで知らせは立たない）。
fn talk_until_just_before_timeout(state: &mut BalloonVisibilityState, world: &World) {
    frame(
        state,
        world,
        Frame::new(&[(0, seen(3, false))])
            .at(0.0)
            .talk_started()
            .display_end(END),
    );
    frame(
        state,
        world,
        Frame::new(&[(0, seen(3, true))]).at(END).ended_at(END),
    );
    frame(
        state,
        world,
        Frame::new(&[(0, seen(3, true))]).at(END + WAIT - 0.001),
    );
}

/// 満了のフレーム。
fn expiry_frame() -> Frame {
    Frame::new(&[(0, seen(3, true))]).at(END + WAIT)
}

/// 可視性の相が書いた行だけを拾う。
fn notice_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains("[balloon-visibility]"))
        .collect()
}

/// 時間切れで隠したフレームにだけ、置き場のゴーストの kanade へ知らせが 1 通届き、番号つきで
/// 1 行記録される。満了の前後のフレームでは送らない（要件 2.1・design の Error Strategy）。
#[test]
fn timeout_frame_sends_one_notice_to_the_ghost_in_the_slot() {
    let (tx, rx) = mpsc::channel();
    let world = world_with_ghost(Some(tx));
    let mut state = BalloonVisibilityState::default();

    talk_until_just_before_timeout(&mut state, &world);
    assert_eq!(received(&rx), vec![], "満了の前は送らない");

    let ((), events) = capture_logs(|| frame(&mut state, &world, expiry_frame()));
    assert_eq!(received(&rx), vec![TALK_ID], "満了のフレームに 1 通");
    let lines = notice_lines(&events);
    assert_eq!(lines.len(), 1, "送ったら 1 行: {events:?}");
    assert_eq!(lines[0].level, Level::INFO);
    assert_eq!(
        lines[0].expect_field("event"),
        "\"balloon_timeout_notified\""
    );
    assert_eq!(lines[0].expect_field("talk_id"), "1");

    frame(
        &mut state,
        &world,
        Frame::new(&[(0, seen(3, false))]).at(END + WAIT + 1.0),
    );
    assert_eq!(received(&rx), vec![], "隠した後のフレームは送らない");
}

/// 置き場にゴーストが無い（置き場そのものが無い・空・kanade への送出端が無い）ときは送らず、
/// 送れなかった理由を番号つきで警告 1 行に残す（要件 7.2）。
#[test]
fn notice_without_a_ghost_in_the_slot_is_not_sent_and_logged() {
    let no_slot = World::new();
    let mut empty_slot = World::new();
    empty_slot.insert_non_send(GhostSlot(None));
    let no_kanade = world_with_ghost(None);

    for (case, world) in [
        ("置き場が無い", &no_slot),
        ("置き場が空（切替の最中）", &empty_slot),
        ("kanade への送出端が無い", &no_kanade),
    ] {
        let ((), events) = capture_logs(|| notify_timeout(world, Some(TALK_ID)));
        let lines = notice_lines(&events);
        assert_eq!(lines.len(), 1, "{case}: 理由を 1 行: {events:?}");
        assert_eq!(lines[0].level, Level::WARN, "{case}");
        assert_eq!(
            lines[0].expect_field("event"),
            "\"balloon_timeout_notice_failed\"",
            "{case}"
        );
        assert_eq!(lines[0].expect_field("reason"), "\"no_ghost\"", "{case}");
        assert_eq!(lines[0].expect_field("talk_id"), "1", "{case}");
    }
}

/// 受け手（kanade）が消えていれば送れず、理由を誤り 1 行に残す（要件 7.2）。
#[test]
fn notice_to_a_gone_receiver_is_logged_as_an_error() {
    let (tx, rx) = mpsc::channel();
    let world = world_with_ghost(Some(tx));
    drop(rx);

    let ((), events) = capture_logs(|| notify_timeout(&world, Some(TALK_ID)));

    let lines = notice_lines(&events);
    assert_eq!(lines.len(), 1, "理由を 1 行: {events:?}");
    assert_eq!(lines[0].level, Level::ERROR);
    assert_eq!(
        lines[0].expect_field("event"),
        "\"balloon_timeout_notice_failed\""
    );
    assert_eq!(lines[0].expect_field("reason"), "\"receiver_gone\"");
    assert_eq!(lines[0].expect_field("talk_id"), "1");
}

/// 知らせの無いフレームは何も送らず、何も書かない。
#[test]
fn frame_without_a_notice_is_silent() {
    let (tx, rx) = mpsc::channel();
    let world = world_with_ghost(Some(tx));

    let ((), events) = capture_logs(|| {
        notify_timeout(&world, None);
        tracing::info!("[balloon-visibility] 較正（捕捉窓の生存確認）");
    });

    assert_eq!(received(&rx), vec![]);
    assert_eq!(
        notice_lines(&events).len(),
        1,
        "較正の 1 行だけ（知らせの行は無い）: {events:?}"
    );
}

/// ゴーストの切替の直後のフレームに、前のゴーストのトークの番号の知らせが新しいゴーストの
/// kanade へ届かない（トークの番号はゴーストごとに 1 から数え直すので、届くと偶然に合いうる）。
///
/// 切替は置き場の中身と、ゴーストごとの結線（`Emo2Wiring::new` が可視性の状態を既定で組む）を
/// 一緒に入れ替える。新しいゴーストの状態は自分のトークの合図からしか番号を持たないので、前の
/// ゴーストの満了の時刻を過ぎても知らせは立たず、新しいゴーストが同じ番号 1 のトークを終えて
/// 時間切れになったときだけ、その kanade へ 1 通届く。
#[test]
fn notice_of_the_previous_ghost_does_not_reach_the_new_ghost_after_a_switch() {
    let (old_tx, old_rx) = mpsc::channel();
    let mut world = world_with_ghost(Some(old_tx));
    let mut old_state = BalloonVisibilityState::default();
    talk_until_just_before_timeout(&mut old_state, &world);

    // 切替: 置き場は新しいゴーストへ、可視性の状態は新品へ。
    let (new_tx, new_rx) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(new_tx),
        PathBuf::from("balloon_visibility_phase_notice_tests/new"),
    ))));
    let mut new_state = BalloonVisibilityState::default();

    // 較正: 前のゴーストの状態のままなら、このフレームで番号 1 の知らせが立つ（判断だけ・送らない）。
    assert_eq!(
        expiry_frame()
            .timeout(WAIT)
            .run(&mut old_state)
            .timeout_notice,
        Some(TALK_ID),
        "較正: 切替が無ければ満了のフレーム"
    );

    frame(&mut new_state, &world, expiry_frame());
    frame(
        &mut new_state,
        &world,
        Frame::new(&[(0, seen(3, true))]).at(END + WAIT + 5.0),
    );
    assert_eq!(
        received(&new_rx),
        vec![],
        "前のゴーストの番号は新しいゴーストへ届かない"
    );
    assert_eq!(
        received(&old_rx),
        vec![],
        "降ろした前のゴーストへも送らない"
    );

    // 新しいゴーストが自分のトーク（番号 1）を終えて時間切れになれば、その kanade へ 1 通。
    talk_until_just_before_timeout(&mut new_state, &world);
    frame(&mut new_state, &world, expiry_frame());
    assert_eq!(received(&new_rx), vec![TALK_ID]);
}
