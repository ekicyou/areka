//! 終了の後始末での窓口の片付けのテスト（design「Testing Strategy / 窓口と入口」の終了・要件 8.2・
//! 8.5・8.6・11.8）。`desk_tests.rs` の子（道具立ての `Rig`・根・書庫を借りる）。
//!
//! 確かめること: 終了が始まると待っている依頼を始めずに捨て、捨てた書庫の件数とパスを `warn!` に
//! 1 件残す・終了の後に届いたイベントの頼みは kanade へ 0 件で、返信端は落ちる・預かった書庫
//! （起動中のゴーストへ入れる一周）は捨てて背景のスレッドを閉じた扱いで放す・展開し終えた書庫は
//! 結果を返し、書く前にやめた書庫として記録しない・OS のセッションの終了では、降ろす待ちと同じ
//! 出発点から数えて合計が上限を超えない。

use std::time::{Duration, Instant};

use areka_kanade::WaitBudget;
use wintf::AppExit;

use super::*;
use crate::emo2_boot::frame::KanadeNoticeRx;
use crate::exit_wait::EXIT_WAIT_LIMIT;
use crate::session_end::end_session_from;

/// 待っている依頼（2 本の依頼と 1 本の依頼）は始めずに捨て、`warn!(install_pending_discarded)` を
/// 1 件残す。件数は捨てた書庫の数（要件 8.5「待っている依頼（まだ始めていない書庫）」）で、パスと
/// 数が揃う。背景のスレッドへは 1 件も渡らない。
#[test]
fn exit_discards_waiting_orders_with_archive_count_and_paths() {
    let mut rig = Rig::new();
    rig.world.non_send_mut::<InstallDesk>().busy = true;
    let two = InstallOrder {
        archives: vec![PathBuf::from("C:/a.nar"), PathBuf::from("C:/b.nar")],
        origin: InstallOrigin::Script,
    };
    assert_eq!(submit(&mut rig.world, two), SubmitVerdict::Queued);
    assert_eq!(
        submit(&mut rig.world, order("C:/c.nar")),
        SubmitVerdict::Queued
    );

    let (_, events) = capture(|| begin_close(&mut rig.world));
    let discarded = events_named(&events, "install_pending_discarded");
    assert_eq!(discarded.len(), 1, "{events:?}");
    assert_eq!(discarded[0].level, Level::WARN);
    assert_eq!(discarded[0].field("count"), Some("3"), "件数は書庫の数");
    let paths = discarded[0].field("paths").unwrap_or_default();
    for path in ["C:/a.nar", "C:/b.nar", "C:/c.nar"] {
        assert!(paths.contains(path), "{path} が載る: {paths}");
    }
    assert!(rig.world.non_send::<InstallDesk>().queue.is_empty());

    rig.world.non_send_mut::<InstallDesk>().busy = false;
    rig.drain();
    assert!(rig.orders.try_recv().is_err(), "捨てた依頼は渡らない");
}

/// 終了の前に手元に置いた頼みも、終了の後に届いた頼み（イベント・素性）も、kanade へは 0 件で、
/// 返信端は落ちる（背景のスレッドは閉じた扱いで止まる・要件 8.6）。予約が下りた後も送らない。
#[test]
fn asks_after_exit_reach_kanade_zero_times() {
    let tmp = TempPath::new("desk-exit-asks");
    let mut rig = Rig::new();
    let kanade = rig.put_ghost(tmp.path().to_path_buf());
    rig.world.insert_non_send(reservation());
    let before = rig.ask_raise("OnInstallBegin", false);
    rig.drain();
    assert!(
        matches!(before.try_recv(), Ok(None)),
        "予約の間は手元に置く"
    );

    let (_, events) = capture(|| begin_close(&mut rig.world));
    let discarded = events_named(&events, "install_pending_discarded");
    assert_eq!(discarded.len(), 1, "{events:?}");
    assert_eq!(discarded[0].field("count"), Some("0"));
    assert!(
        discarded[0]
            .field("held")
            .is_some_and(|h| h.contains("OnInstallBegin")),
        "{events:?}"
    );

    let after = rig.ask_raise("OnInstallComplete", false);
    let (facts_reply, facts) = reply_channel();
    rig.asks
        .send(DeskAsk::Facts { reply: facts_reply })
        .unwrap();
    rig.world.remove_non_send::<SwitchInFlight>();
    rig.drain();
    rig.drain();

    assert_eq!(kanade.try_iter().count(), 0, "kanade へは 1 件も送らない");
    assert!(before.try_recv().is_err(), "手元の頼みの返信端は落ちる");
    assert!(after.try_recv().is_err(), "終了の後の頼みの返信端は落ちる");
    assert!(facts.try_recv().is_err(), "素性の頼みの返信端も落ちる");
}

/// 「切替を頼んだ」まま終了が始まった預かった書庫は捨てる: 背景のスレッドの返信端が落ち（閉じた
/// 扱い）、捨てた宛先が `warn!(install_pending_discarded)` に載る。書く前なので、門の名前（書庫の
/// パス）は `exit_wait_abandoned` に残る（要件 8.4）。切替の道筋から後で呼ばれても展開しない。
#[test]
fn exit_discards_a_parked_overwrite_and_releases_the_worker() {
    let tmp = TempPath::new("desk-exit-parked");
    let root = root_with_ghost(&tmp, "A");
    let mut rig = Rig::new();
    rig.world.insert_resource(boot_context(&root, "A"));
    let _kanade = rig.put_ghost(root.ghost_dir("A"));
    let answer = rig.ask_overwrite(ghost_archive(&tmp, "A"));
    rig.drain();
    assert!(matches!(answer.try_recv(), Ok(None)), "切替を頼んで預かる");
    let gate = rig.world.non_send::<InstallDesk>().gate.clone();
    assert!(gate.begin("C:/A.nar".to_owned()));

    let (_, events) = capture(|| begin_close(&mut rig.world));
    assert!(
        answer.try_recv().is_err(),
        "背景のスレッドは閉じた扱いで放す"
    );
    let discarded = events_named(&events, "install_pending_discarded");
    assert_eq!(discarded.len(), 1, "{events:?}");
    assert!(
        discarded[0]
            .field("overwrite")
            .is_some_and(|o| o.contains("\"A\"") && o.contains("requested")),
        "{events:?}"
    );
    let abandoned = events_named(&events, "exit_wait_abandoned");
    assert_eq!(abandoned.len(), 1, "{events:?}");
    assert_eq!(abandoned[0].field_str("label"), Some("C:/A.nar"));

    run_overwrite_between(&mut rig.world);
    assert!(
        !root
            .ghost_dir("A")
            .join("ghost/master/overwritten.txt")
            .exists(),
        "捨てた書庫は展開しない"
    );
}

/// 展開し終えた（定常到達を待っている）書庫は、終了で結果を背景のスレッドへ返し、書く前にやめた
/// 書庫として記録しない（展開は UI スレッドで同期に済んでいる）。
#[test]
fn exit_after_the_overwrite_ran_returns_the_result_without_abandoning() {
    let tmp = TempPath::new("desk-exit-ran");
    let root = root_with_ghost(&tmp, "A");
    let mut rig = Rig::new();
    rig.world.insert_resource(boot_context(&root, "A"));
    let _kanade = rig.put_ghost(root.ghost_dir("A"));
    let answer = rig.ask_overwrite(ghost_archive(&tmp, "A"));
    let gate = rig.world.non_send::<InstallDesk>().gate.clone();
    assert!(gate.begin("C:/A.nar".to_owned()));
    rig.drain();
    run_overwrite_between(&mut rig.world);
    assert!(
        root.ghost_dir("A")
            .join("ghost/master/overwritten.txt")
            .exists(),
        "展開した"
    );

    let (_, events) = capture(|| begin_close(&mut rig.world));
    assert!(
        events_named(&events, "exit_wait_abandoned").is_empty(),
        "書き終えた書庫を書く前にやめたと記録しない: {events:?}"
    );
    assert!(
        matches!(answer.try_recv(), Ok(Some(Overwritten::Ran(Ok(_))))),
        "展開の結果を返す"
    );
    let discarded = events_named(&events, "install_pending_discarded");
    assert_eq!(discarded.len(), 1, "{events:?}");
    assert!(
        discarded[0]
            .field("overwrite")
            .is_some_and(|o| o.contains("ran")),
        "{events:?}"
    );
}

/// OS のセッションの終了: 展開の最中（門が書いている最中のまま）でも、後始末に入った時点から
/// 数えて上限で待つのをやめ、`warn!(exit_wait_timeout)` を 1 件残して後始末を終える。出発点を上限
/// だけ過去に置いた予算（前の段が上限を使い切った形）では待たずに戻る＝降ろす待ちの後に上限を
/// 数え直さず、合計は上限を超えない（要件 8.2・11.8）。数え直すと 3 秒待つ。
#[test]
fn os_session_end_waits_within_the_shared_budget() {
    let mut rig = Rig::new();
    rig.world.insert_non_send(AppExit::new());
    rig.world.insert_non_send(KanadeNoticeRx(mpsc::channel().1));
    let gate = rig.world.non_send::<InstallDesk>().gate.clone();
    assert!(gate.begin("C:/A.nar → 根".to_owned()));
    assert!(gate.enter_write());

    let budget = WaitBudget {
        started: Instant::now() - EXIT_WAIT_LIMIT,
        limit: EXIT_WAIT_LIMIT,
    };
    let t0 = Instant::now();
    let ((), events) = capture(|| end_session_from(&mut rig.world, budget));
    let spent = t0.elapsed();

    let timeout = events_named(&events, "exit_wait_timeout");
    assert_eq!(timeout.len(), 1, "{events:?}");
    assert_eq!(timeout[0].field_str("label"), Some("C:/A.nar → 根"));
    let position = |name: &str| {
        events
            .iter()
            .position(|e| e.field_str("event") == Some(name))
    };
    assert!(
        position("exit_wait_timeout") < position("os_session_end_done"),
        "待ちは後始末の記録の前: {events:?}"
    );
    assert!(
        spent < Duration::from_secs(1),
        "使い切った予算では待たない（数え直すと上限まで待つ）: {spent:?}"
    );
    assert!(gate.is_closing());
}
