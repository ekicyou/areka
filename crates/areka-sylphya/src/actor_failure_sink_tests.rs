//! 書き手のアクターの失敗の受け口（spec areka-P0-write-failure-notice タスク 1.2・要件 1.3・6.1）。
//!
//! 投函の書き込みが失敗すると受け口へ 1 件ずつ届き、中身は投函の key とスコープに合う。
//! 成功では届かない。受け口なし（`spawn_sylphya`）でも今どおり動き、受け手を落としても
//! アクターは止まらない。同期は barrier だけで取る（`thread::sleep` なし）。

use super::*;
use crate::persist::{FakePersistIo, PersistFailureCause, PersistKey, PersistScope, ScopeRoots};
use std::path::PathBuf;
use std::sync::mpsc;

/// Ghost スコープの root だけ持つ init（App スコープは root 不在＝投函すると書けない）。
fn init_ghost_root_only() -> SylphyaInit {
    SylphyaInit {
        roots: ScopeRoots {
            ghost: Some(PathBuf::from("/g")),
            ..ScopeRoots::default()
        },
        io: Box::new(FakePersistIo::new()),
        runtime_sink: None,
    }
}

fn failing_entries() -> Vec<(PersistKey, String)> {
    vec![
        (PersistKey::BootCount, "3".into()),
        (PersistKey::BasewareRoots, "C:\\a".into()),
    ]
}

#[test]
fn failing_post_sends_exactly_one_failure_matching_keys_and_scope() {
    let (tx, rx) = mpsc::channel();
    let parts = spawn_sylphya_with_failures(init_ghost_root_only(), Some(tx));

    parts
        .publisher
        .persist_put(PersistScope::App, failing_entries());
    parts.publisher.barrier().expect("barrier");

    let got: Vec<PersistFailure> = rx.try_iter().collect();
    assert_eq!(got.len(), 1, "失敗の投函 1 回につき 1 件: {got:?}");
    assert_eq!(got[0].scope, PersistScope::App);
    assert_eq!(
        got[0].keys,
        vec![PersistKey::BootCount, PersistKey::BasewareRoots]
    );
    assert_eq!(got[0].cause, PersistFailureCause::NoRoot);

    parts.publisher.close();
    parts.handle.join().expect("join");
}

#[test]
fn successful_post_sends_nothing() {
    let (tx, rx) = mpsc::channel();
    let parts = spawn_sylphya_with_failures(init_ghost_root_only(), Some(tx));

    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "1".into())],
    );
    parts.publisher.barrier().expect("barrier");

    assert_eq!(rx.try_iter().count(), 0, "成功では送らない");

    parts.publisher.close();
    parts.handle.join().expect("join");
}

#[test]
fn spawn_without_receiver_behaves_as_today() {
    let parts = spawn_sylphya(init_ghost_root_only());

    // 失敗も成功も今どおり処理され、返信つきの投函は結果を返す。
    parts
        .publisher
        .persist_put(PersistScope::App, failing_entries());
    let (reply, reply_rx) = areka_actor::reply_channel();
    parts
        .publisher
        .tx
        .send(SylphyaMsg::PersistPut {
            scope: PersistScope::Ghost,
            entries: vec![(PersistKey::BootCount, "2".into())],
            reply: Some(reply),
        })
        .expect("post");
    assert_eq!(reply_rx.recv().expect("reply"), PersistOutcome::Saved);
    parts.publisher.barrier().expect("barrier");

    parts.publisher.close();
    parts.handle.join().expect("join");
}

#[test]
fn dropped_receiver_does_not_stop_the_actor() {
    let (tx, rx) = mpsc::channel();
    let parts = spawn_sylphya_with_failures(init_ghost_root_only(), Some(tx));
    drop(rx);

    // 受け手が落ちた後の失敗の投函でも panic せず続ける（warn! を残す）。
    parts
        .publisher
        .persist_put(PersistScope::App, failing_entries());
    parts
        .publisher
        .barrier()
        .expect("受け手を落としてもアクターは生きている");
    parts
        .publisher
        .persist_put(PersistScope::App, failing_entries());
    parts.publisher.barrier().expect("2 回目も生きている");

    parts.publisher.close();
    parts.handle.join().expect("panic せず終わる");
}
