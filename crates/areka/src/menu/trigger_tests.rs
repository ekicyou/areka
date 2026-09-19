//! 引き金の純粋な判定（[`poll_step`]・[`decide`]・[`InFlightGuard`]）の決定論テスト（要件 9.3）。
//!
//! 確かめること: 表示可否と預かりの 4 組・返事の覗き見の 4 通り（届いた／期限内で未着／期限超過／
//! 切断）・照会を送れなかったとき・届いた返事が期限超過より優先されること・表示 1 枚の旗が
//! 持ち主の生存とともに立ち、落ちると必ず降りること。
//!
//! 時刻は引数で渡す。期限は `Instant::now()` を基点に組み立て、その前後の時刻を直接渡すので
//! どのテストも待たない（上限の 1 秒を実際に過ごすと毎回のテストが遅くなるうえ、機械の都合で
//! 揺れる）。期待値はすべて書き下した値で、判定の実装から導かない。

use std::time::Duration;

use areka_actor::reply_channel;
use areka_kanade::resources::ResourceOutcome;

use super::*;
use crate::menu::captions::QUERY_TIMEOUT;

/// 預かりの見本（値そのものは判定に使われない＝有無だけが効く）。
fn pending_double_click() -> PendingDoubleClick {
    PendingDoubleClick {
        scope: 1,
        surface_pos: (12, 34),
        region: Some("Head".to_string()),
    }
}

/// 判定に使う要求の見本。窓ハンドルも entity も純粋関数は触らない。
fn request() -> MenuRequest {
    MenuRequest {
        scope: 0,
        entity: Entity::from_raw_u32(1).expect("テスト用 entity 索引は有効"),
        hwnd: HWND(0x1234 as *mut _),
        screen_pos: (100, 200),
    }
}

/// 返事待ちを 1 件組み立てる。`deadline` は基点から [`QUERY_TIMEOUT`] 後。
fn pending_query(rx: Option<ReplyReceiver<QueryReply>>, base: Instant) -> PendingQuery {
    PendingQuery::new(
        request(),
        Vec::new(),
        rx,
        base + QUERY_TIMEOUT,
        InFlightGuard::engage(&Rc::new(Cell::new(false))),
    )
}

/// 返事の見本（この値がそのまま戻ってくることを確かめる）。
fn reply() -> QueryReply {
    vec![
        (
            "sakura.popupmenu.visible",
            ResourceOutcome::Value("1".to_string()),
        ),
        (
            "closebutton.caption",
            ResourceOutcome::Value("おわる".to_string()),
        ),
    ]
}

#[test]
fn decide_shows_and_drops_the_pending_double_click() {
    assert_eq!(decide(Visibility::Show, None), Decision::Show);
    let pending = pending_double_click();
    assert_eq!(decide(Visibility::Show, Some(&pending)), Decision::Show);
}

#[test]
fn decide_sends_the_double_click_only_when_suppressed_and_deferred() {
    let pending = pending_double_click();
    assert_eq!(
        decide(Visibility::Suppress, Some(&pending)),
        Decision::Suppress {
            send_double_click: true
        }
    );
    assert_eq!(
        decide(Visibility::Suppress, None),
        Decision::Suppress {
            send_double_click: false
        }
    );
}

#[test]
fn poll_step_returns_the_reply_that_arrived() {
    let base = Instant::now();
    let (tx, rx) = reply_channel::<QueryReply>();
    tx.send(reply()).expect("受け口は生きている");
    let pending = pending_query(Some(rx), base);

    match poll_step(&pending, base) {
        PollOutcome::Decided(Ok(got)) => assert_eq!(got, reply()),
        other => panic!("届いた返事はそのまま返るべき: {other:?}"),
    }
}

#[test]
fn poll_step_waits_while_the_deadline_has_not_passed() {
    let base = Instant::now();
    let (_tx, rx) = reply_channel::<QueryReply>();
    let pending = pending_query(Some(rx), base);

    match poll_step(&pending, base + QUERY_TIMEOUT - Duration::from_millis(1)) {
        PollOutcome::Wait => {}
        other => panic!("期限内で未着なら待つべき: {other:?}"),
    }
}

#[test]
fn poll_step_times_out_after_the_deadline() {
    let base = Instant::now();
    let (_tx, rx) = reply_channel::<QueryReply>();
    let pending = pending_query(Some(rx), base);

    match poll_step(&pending, base + QUERY_TIMEOUT + Duration::from_millis(1)) {
        PollOutcome::Decided(Err(QueryFailure::Timeout)) => {}
        other => panic!("期限を過ぎて未着なら上限超過であるべき: {other:?}"),
    }
}

/// 期限ちょうどの時刻は「期限内」に入らない（待つのは `now < deadline` の間だけ）。
#[test]
fn poll_step_times_out_exactly_at_the_deadline() {
    let base = Instant::now();
    let (_tx, rx) = reply_channel::<QueryReply>();
    let pending = pending_query(Some(rx), base);

    match poll_step(&pending, base + QUERY_TIMEOUT) {
        PollOutcome::Decided(Err(QueryFailure::Timeout)) => {}
        other => panic!("期限ちょうどで未着なら上限超過であるべき: {other:?}"),
    }
}

#[test]
fn poll_step_reports_the_dropped_reply_channel() {
    let base = Instant::now();
    let (tx, rx) = reply_channel::<QueryReply>();
    drop(tx);
    let pending = pending_query(Some(rx), base);

    match poll_step(&pending, base) {
        PollOutcome::Decided(Err(QueryFailure::Dropped)) => {}
        other => panic!("返信端が捨てられたら切断であるべき: {other:?}"),
    }
}

#[test]
fn poll_step_reports_the_query_that_could_not_be_sent() {
    let base = Instant::now();
    let pending = pending_query(None, base);

    match poll_step(&pending, base) {
        PollOutcome::Decided(Err(QueryFailure::SendFailed)) => {}
        other => panic!("送れなかった照会は送出失敗であるべき: {other:?}"),
    }
}

#[test]
fn poll_step_prefers_a_reply_that_arrived_over_the_passed_deadline() {
    let base = Instant::now();
    let (tx, rx) = reply_channel::<QueryReply>();
    tx.send(reply()).expect("受け口は生きている");
    let pending = pending_query(Some(rx), base);

    match poll_step(&pending, base + QUERY_TIMEOUT + Duration::from_millis(1)) {
        PollOutcome::Decided(Ok(got)) => assert_eq!(got, reply()),
        other => panic!("手元に返事があるなら期限超過より優先されるべき: {other:?}"),
    }
}

#[test]
fn in_flight_guard_raises_the_flag_and_lowers_it_on_drop() {
    let flag = Rc::new(Cell::new(false));
    let guard = InFlightGuard::engage(&flag);
    assert!(flag.get(), "持ち主が生きている間は旗が立っている");
    drop(guard);
    assert!(!flag.get(), "持ち主が落ちたら旗は降りる");
}

#[test]
fn in_flight_guard_lowers_the_flag_when_the_pending_query_is_dropped() {
    let flag = Rc::new(Cell::new(false));
    let (_tx, rx) = reply_channel::<QueryReply>();
    let pending = PendingQuery::new(
        request(),
        Vec::new(),
        Some(rx),
        Instant::now() + QUERY_TIMEOUT,
        InFlightGuard::engage(&flag),
    );
    assert!(flag.get(), "待ちが抱えている間は旗が立っている");
    drop(pending);
    assert!(!flag.get(), "待ちを捨てたら旗は降りる");
}
