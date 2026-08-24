//! [`super`]（アクタースレッドの役割宣言）の決定論テスト。
//!
//! 名簿はプロセス共有で、テストは並列に走る——**全体の件数は当てにせず**、自分が起こした
//! スレッドの役割名だけを検査する。実時間の閾値は判定に使わない（待ちは「登録が起きなかった
//! ときに無限に待たない」ための上限であって合否の対象ではない）。

use super::*;

use std::sync::mpsc;
use std::time::{Duration, Instant};

use areka_ghost::ticker::{
    LoopTickerConfig, Tick, TickerConfig, TickerMsg, spawn_loop_ticker, spawn_ticker,
};
use areka_kanade::KanadeMsg;
use wintf::ecs::world::thread_registry::{
    self, ROLE_TICKER_DISPATCHER_KANADE, ROLE_TICKER_LOOP, is_known_role, role_actor,
};

/// テスト中にティッカーが 1 度も発火しないよう十分長く取った周期。
/// 停止は `TickerMsg::Close` で行うので、待たされることはない。
const NEVER_FIRES: Duration = Duration::from_secs(3600);

/// 名簿に当該役割名が現れるまで待つ。現れれば `true`。
fn wait_for_role(role: &str, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        if thread_registry::snapshot()
            .into_iter()
            .any(|entry| entry.role == role)
        {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

// ── 役割名の写像（純関数）──────────────────────────────────────

#[test]
fn ticker_names_map_to_the_two_ticker_roles() {
    assert_eq!(role_for_actor_name("ticker"), ROLE_TICKER_DISPATCHER_KANADE);
    assert_eq!(role_for_actor_name("loop-ticker"), ROLE_TICKER_LOOP);
}

#[test]
fn other_actor_names_map_to_the_actor_role() {
    assert_eq!(role_for_actor_name("emo-text"), "actor:emo-text");
    assert_eq!(role_for_actor_name("kanade"), "actor:kanade");
    assert_eq!(role_for_actor_name("seriko"), role_actor("seriko"));
}

#[test]
fn every_mapped_role_is_in_the_fixed_vocabulary() {
    for name in ["ticker", "loop-ticker", "emo-text", "kanade", "seriko"] {
        let role = role_for_actor_name(name);
        assert!(
            is_known_role(&role),
            "写像した役割名 {role} は固定語彙に含まれるはず（{name}）"
        );
    }
}

// ── 実際のスレッド生成点との結線 ────────────────────────────────

/// 導入したフックが、**本物の**ティッカー 2 系統と素のアクターを、宣言どおりの役割名で
/// 名簿へ載せる（要件 2.3）。
///
/// ティッカーの名前（`ticker`／`loop-ticker`）を実物から取るので、`areka-ghost` 側で名前が
/// 変わればこのテストが落ちる（`actor:` へ黙って落ちない）。3 本とも**生かしたまま**観測して
/// から閉じる——終了した TID が別スレッドへ再利用されて項目が置き換わる余地を残さないため。
#[test]
fn installed_hook_registers_the_real_ticker_threads_with_their_declared_roles() {
    // 既に他のテストが導入していても同じ関数ゆえ等価（最初が勝つ・戻り値は問わない）。
    let _ = install();

    let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (dispatcher_tx, _dispatcher_rx) = mpsc::channel::<Tick>();
    let (ticker_tx, ticker_handle) = spawn_ticker::<Tick>(
        TickerConfig {
            base_interval: NEVER_FIRES,
            kanade_interval: NEVER_FIRES,
            ..Default::default()
        },
        kanade_tx,
        dispatcher_tx,
    );
    let (loop_tx, loop_handle) = spawn_loop_ticker(
        LoopTickerConfig {
            interval: NEVER_FIRES,
            ..Default::default()
        },
        Box::new(|_tick| {}),
    );
    let probe_role = role_actor("thread-roles-probe");
    let (probe_tx, probe_handle) = areka_actor::spawn_actor::<(), _>("thread-roles-probe", |rx| {
        // 全 Sender drop（切断）まで居座る＝観測中はスレッドが生きている。
        while rx.recv().is_ok() {}
    });

    let limit = Duration::from_secs(5);
    let ticker_seen = wait_for_role(ROLE_TICKER_DISPATCHER_KANADE, limit);
    let loop_seen = wait_for_role(ROLE_TICKER_LOOP, limit);
    let probe_seen = wait_for_role(&probe_role, limit);

    // 観測を終えてから停止（Close 送信 → join）。
    ticker_tx
        .send(TickerMsg::Close)
        .expect("ticker へ Close を送れるはず");
    loop_tx
        .send(TickerMsg::Close)
        .expect("loop-ticker へ Close を送れるはず");
    drop(probe_tx);
    ticker_handle.join().expect("ticker は正常終了するはず");
    loop_handle.join().expect("loop-ticker は正常終了するはず");
    probe_handle.join().expect("probe は正常終了するはず");

    assert!(
        ticker_seen,
        "ticker スレッドは {ROLE_TICKER_DISPATCHER_KANADE} として名簿へ載るはず"
    );
    assert!(
        loop_seen,
        "loop-ticker スレッドは {ROLE_TICKER_LOOP} として名簿へ載るはず"
    );
    assert!(
        probe_seen,
        "素のアクターは {probe_role} として名簿へ載るはず"
    );
}
