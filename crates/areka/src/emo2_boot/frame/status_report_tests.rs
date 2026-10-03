// =============================================================================
// バルーンの表示の届けの決定論テスト（areka-P0-status-execution-states task 5.1）
//
// ⑴ 観測列から組を作る純関数の分岐（不可視・未装着・番号あり・番号なし）
// ⑵ 差分のときだけ送る・同じ組は送らない・置き場が空なら送らず台帳を保つ・受け手が落ちて
//    いれば error が 1 件（台帳は更新して毎フレーム鳴らさない）・警告は scope ごとに 1 回
// 表示層の照会は headless の実 `EmoPresenter` を 1 件だけ踏ませ、残りは観測を直接与える。
// =============================================================================

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use tracing::Level;

use super::*;
use crate::ghost_session::GhostSession;
use crate::placement::test_support::{LogEvent, capture_logs};

fn obs(scope: u32, visible: Option<bool>, surface_id: Option<u32>) -> BalloonObservation {
    BalloonObservation {
        scope,
        visible,
        surface_id,
    }
}

fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
    BalloonBinding {
        character_id,
        balloon_id,
    }
}

fn named<'a>(events: &'a [LogEvent], name: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 置き場にゴースト（kanade の送出端だけを持つ）を据えた World と、その受信端。
fn world_with_ghost() -> (World, Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        PathBuf::from("ghost/test"),
    ))));
    (world, rx)
}

/// 受信端に溜まったバルーンの組の知らせ（それ以外の知らせは失敗）。
fn sent(rx: &Receiver<KanadeMsg>) -> Vec<Vec<BalloonBinding>> {
    rx.try_iter()
        .map(|msg| match msg {
            KanadeMsg::ExecutionState(ExecutionStateUpdate::Balloons(b)) => b,
            _ => panic!("バルーンの組の知らせ以外が届いた"),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ⑴ 組を作る純関数
// ---------------------------------------------------------------------------

#[test]
fn collect_keeps_only_visible_scopes_in_input_order_with_their_number() {
    let (bindings, unknown) = collect_bindings(&[
        obs(0, Some(true), Some(2)),
        obs(1, Some(false), Some(5)), // 不可視
        obs(2, None, None),           // 未装着
        obs(3, Some(true), Some(0)),
    ]);
    assert_eq!(bindings, vec![binding(0, 2), binding(3, 0)]);
    assert!(unknown.is_empty());
}

#[test]
fn collect_includes_visible_scope_without_number_as_zero_and_returns_it_as_warning_target() {
    let (bindings, unknown) = collect_bindings(&[
        obs(0, Some(true), None),
        obs(1, Some(true), Some(4)),
        obs(2, Some(false), None), // 見えていなければ警告の対象にもしない
    ]);
    assert_eq!(bindings, vec![binding(0, 0), binding(1, 4)]);
    assert_eq!(unknown, vec![0]);
}

#[test]
fn collect_of_nothing_visible_is_empty() {
    let (bindings, unknown) = collect_bindings(&[obs(0, Some(false), Some(1)), obs(1, None, None)]);
    assert!(bindings.is_empty());
    assert!(unknown.is_empty());
}

// ---------------------------------------------------------------------------
// ⑵ 差分・送出・記録
// ---------------------------------------------------------------------------

/// 差分のときだけ送り、同じ組は送らない。消えたら空の組を送る（要件 4.1・4.5・4.6）。
#[test]
fn sends_only_on_change_and_never_resends_the_same_set() {
    let (world, rx) = world_with_ghost();
    let mut ledger = BalloonStatusLedger::default();

    report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(2))]);
    assert_eq!(sent(&rx), vec![vec![binding(0, 2)]]);

    report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(2))]);
    assert!(sent(&rx).is_empty(), "同じ組は送らない");

    report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(3))]);
    assert_eq!(sent(&rx), vec![vec![binding(0, 3)]], "番号が変われば送る");

    report_observed(&world, &mut ledger, &[obs(0, Some(false), Some(3))]);
    assert_eq!(sent(&rx), vec![Vec::new()], "消えたら空の組を送る");

    report_observed(&world, &mut ledger, &[obs(0, Some(false), Some(3))]);
    assert!(sent(&rx).is_empty());
}

/// 何も見えていない最初のフレームは送らない（台帳の既定＝空の組と等しい）。
#[test]
fn nothing_visible_from_the_start_sends_nothing() {
    let (world, rx) = world_with_ghost();
    let mut ledger = BalloonStatusLedger::default();
    report_observed(&world, &mut ledger, &[obs(0, Some(false), Some(0))]);
    assert!(sent(&rx).is_empty());
}

/// 置き場が空なら送らず台帳を保ち、ゴーストが据わった最初のフレームで送る。
#[test]
fn empty_slot_skips_with_debug_and_keeps_the_ledger() {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(None));
    let mut ledger = BalloonStatusLedger::default();

    let (_, events) = capture_logs(|| {
        report_observed(&world, &mut ledger, &[obs(1, Some(true), Some(0))]);
    });
    let skipped = named(&events, "balloon_status_no_ghost");
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, Level::DEBUG);
    assert!(ledger.last_sent.is_empty(), "見送った組は送った組にしない");

    let (tx, rx) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        PathBuf::from("ghost/test"),
    ))));
    report_observed(&world, &mut ledger, &[obs(1, Some(true), Some(0))]);
    assert_eq!(sent(&rx), vec![vec![binding(1, 0)]]);
}

/// 置き場そのものが World に無くても送らない（結線なしの起動）。
#[test]
fn missing_slot_resource_sends_nothing() {
    let world = World::new();
    let mut ledger = BalloonStatusLedger::default();
    report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(0))]);
    assert!(ledger.last_sent.is_empty());
}

/// 受け手が落ちていれば error が 1 件。台帳は更新するので次のフレームで鳴り直さない（要件 7.1）。
#[test]
fn dropped_receiver_records_exactly_one_error() {
    let (world, rx) = world_with_ghost();
    drop(rx);
    let mut ledger = BalloonStatusLedger::default();

    let (_, events) = capture_logs(|| {
        report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(2))]);
        report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(2))]);
    });
    let failed = named(&events, "balloon_status_send_failed");
    assert_eq!(failed.len(), 1, "渡せない旨が 1 行: {events:?}");
    assert_eq!(failed[0].level, Level::ERROR);
    assert_eq!(ledger.last_sent, vec![binding(0, 2)]);
}

/// 番号が取れない旨は scope ごとに 1 回。番号が取れたら再武装する。
#[test]
fn surface_unknown_warns_once_per_scope_and_rearms_when_the_number_returns() {
    let (world, _rx) = world_with_ghost();
    let mut ledger = BalloonStatusLedger::default();
    let unknown = [obs(0, Some(true), None), obs(1, Some(true), None)];
    let known = [obs(0, Some(true), Some(1)), obs(1, Some(true), None)];

    let (_, events) = capture_logs(|| {
        report_observed(&world, &mut ledger, &unknown);
        report_observed(&world, &mut ledger, &unknown);
        report_observed(&world, &mut ledger, &known); // scope 0 だけ再武装
        report_observed(&world, &mut ledger, &unknown);
    });
    let warned: Vec<Option<&str>> = named(&events, "balloon_status_surface_unknown")
        .iter()
        .inspect(|e| assert_eq!(e.level, Level::WARN))
        .map(|e| e.field("scope"))
        .collect();
    assert_eq!(warned, vec![Some("0"), Some("1"), Some("0")]);
}

/// 照会の経路: 未装着の scope は見えていない扱いで、前に送った組から落ちる（要件 4.5・4.7）。
#[test]
fn report_balloons_reads_the_presenter_and_drops_unattached_scopes() {
    let (world, rx) = world_with_ghost();
    let presenter = EmoPresenter::new();
    let mut ledger = BalloonStatusLedger {
        last_sent: vec![binding(0, 2)],
        ..Default::default()
    };
    report_balloons(&presenter, &world, &mut ledger, &[0, 1]);
    assert_eq!(sent(&rx), vec![Vec::new()]);
    assert!(ledger.last_sent.is_empty());
}
