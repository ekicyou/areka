// =============================================================================
// バルーンの表示の届けの決定論テスト（areka-P0-status-execution-states task 5.1）
//
// ⑴ 観測列から組を作る純関数の分岐（不可視・未装着・番号あり・番号なし）
// ⑵ 差分のときだけ送る・同じ組は送らない・置き場が空なら送らず台帳を保つ・受け手が落ちて
//    いれば error が 1 件（台帳は更新して毎フレーム鳴らさない）・警告は scope ごとに 1 回
// ⑶ 箱だけ・番号なしのスコープは最後に取れた番号（無ければ 0）で載せ、警告の対象にしない
//    （areka-P0-shell-balloon-frame-align 要件 3.1・3.2）
// 表示層の照会は headless の実 `EmoPresenter` を 1 件だけ踏ませ、残りは観測を直接与える。
// =============================================================================

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use tracing::Level;

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;

use super::test_support::{binding, obs, obs_box};
use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};
use log_capture_kit::{CapturedEvent, capture};
// 私有の欄は渡し口から読み書きする（設計 D3 ⑷）。
use areka_runtime::frame::status_report::test_support::{last_sent, runtime_busy_logged};

fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 置き場にゴースト（kanade の送出端だけを持つ）を据えた World と、その受信端。
fn world_with_ghost() -> (World, Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    crate::session::hooks::install(&mut world);
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
    crate::session::hooks::install(&mut world);
    world.insert_non_send(GhostSlot(None));
    let mut ledger = BalloonStatusLedger::default();

    let (_, events) = capture(|| {
        report_observed(&world, &mut ledger, &[obs(1, Some(true), Some(0))]);
    });
    let skipped = named(&events, "balloon_status_no_ghost");
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, Level::DEBUG);
    assert!(
        last_sent(&mut ledger).is_empty(),
        "見送った組は送った組にしない"
    );

    let (tx, rx) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        PathBuf::from("ghost/test"),
    ))));
    report_observed(&world, &mut ledger, &[obs(1, Some(true), Some(0))]);
    assert_eq!(sent(&rx), vec![vec![binding(1, 0)]]);
}

/// 受け手が落ちていれば error が 1 件。台帳は更新するので次のフレームで鳴り直さない（要件 7.1）。
#[test]
fn dropped_receiver_records_exactly_one_error() {
    let (world, rx) = world_with_ghost();
    drop(rx);
    let mut ledger = BalloonStatusLedger::default();

    let (_, events) = capture(|| {
        report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(2))]);
        report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(2))]);
    });
    let failed = named(&events, "balloon_status_send_failed");
    assert_eq!(failed.len(), 1, "渡せない旨が 1 行: {events:?}");
    assert_eq!(failed[0].level, Level::ERROR);
    assert_eq!(*last_sent(&mut ledger), vec![binding(0, 2)]);
}

/// 番号が取れない旨は scope ごとに 1 回。番号が取れたら再武装する。
#[test]
fn surface_unknown_warns_once_per_scope_and_rearms_when_the_number_returns() {
    let (world, _rx) = world_with_ghost();
    let mut ledger = BalloonStatusLedger::default();
    let unknown = [obs(0, Some(true), None), obs(1, Some(true), None)];
    let known = [obs(0, Some(true), Some(1)), obs(1, Some(true), None)];

    let (_, events) = capture(|| {
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

/// 番号が取れたフレームで覚え（窓が見えていなくても）、次に箱だけ・番号なしになったらその番号で載せる。
/// 警告の段の行は出さない（areka-P0-shell-balloon-frame-align 要件 3.1・3.2）。
#[test]
fn box_only_without_number_reports_the_number_remembered_from_an_earlier_frame() {
    let (world, rx) = world_with_ghost();
    let mut ledger = BalloonStatusLedger::default();

    let (_, events) = capture(|| {
        // 普通のバルーンは面 4 で不可視（装着しただけ・出して隠した後）→ 送らず、番号だけ覚える
        report_observed(&world, &mut ledger, &[obs(0, Some(false), Some(4))]);
        assert!(sent(&rx).is_empty());
        // 箱にだけ文字・今の番号なし → 覚えた番号で載る
        report_observed(&world, &mut ledger, &[obs_box(0, Some(false), None)]);
        assert_eq!(sent(&rx), vec![vec![binding(0, 4)]]);
        report_observed(&world, &mut ledger, &[obs_box(0, Some(false), None)]);
        assert!(sent(&rx).is_empty(), "同じ組は送り直さない");
    });
    assert!(
        named(&events, "balloon_status_surface_unknown").is_empty(),
        "箱だけは警告しない: {events:?}"
    );
}

/// 照会の経路: 未装着の scope は見えていない扱いで、前に送った組から落ちる（要件 4.5・4.7）。
#[test]
fn report_balloons_reads_the_presenter_and_drops_unattached_scopes() {
    let (world, rx) = world_with_ghost();
    let presenter = EmoPresenter::new();
    let mut ledger = BalloonStatusLedger::default();
    *last_sent(&mut ledger) = vec![binding(0, 2)];
    let runtime = TextLayerRuntime::new(TextLayerConfig::default());
    report_balloons(&presenter, &runtime, &world, &mut ledger, &[0, 1]);
    assert_eq!(sent(&rx), vec![Vec::new()]);
    assert!(last_sent(&mut ledger).is_empty());
}

// ---------------------------------------------------------------------------
// ⑷ 届けの相（areka-P0-shell-balloon-frame-align design「届けの相と台帳」）
// ---------------------------------------------------------------------------

/// 文字の層を借りられないフレームは誤りの段で 1 度だけ記録して届けを次のフレームへ回し、
/// 借りられたフレームで届けて記録の印を戻す（次に借りられなくなったらまた 1 度鳴る）。
#[test]
fn status_report_phase_defers_while_the_runtime_is_busy_and_logs_once_per_outage() {
    use super::super::test_support::{headless_wiring_with, zero_clock};

    let (world, rx) = world_with_ghost();
    let mut wiring = headless_wiring_with(mpsc::channel().1, zero_clock());
    // 装着済みバルーンの scope 0（表示層には未装着＝見えていない扱い）。前に送った組と違うので、
    // 借りられたフレームでは空の組が届く。
    wiring
        .balloon_models
        .insert(0, areka_parsers::balloon::parse_str("", None));
    *last_sent(&mut wiring.balloon_status) = vec![binding(0, 2)];
    let runtime = std::rc::Rc::clone(wiring.runtime());

    let (_, events) = capture(|| {
        let held = runtime.borrow_mut();
        run_status_report_phase(&mut wiring, &world);
        run_status_report_phase(&mut wiring, &world);
        drop(held);
    });
    let busy = named(&events, "balloon_status_runtime_busy");
    assert_eq!(busy.len(), 1, "借りられない旨は 1 行: {events:?}");
    assert_eq!(busy[0].level, Level::ERROR);
    assert!(sent(&rx).is_empty(), "借りられないフレームは届けない");
    assert_eq!(*last_sent(&mut wiring.balloon_status), vec![binding(0, 2)]);

    run_status_report_phase(&mut wiring, &world);
    assert_eq!(sent(&rx), vec![Vec::new()], "借りられたフレームで届く");
    assert!(!runtime_busy_logged(&wiring.balloon_status));

    let (_, events) = capture(|| {
        let held = runtime.borrow_mut();
        run_status_report_phase(&mut wiring, &world);
        drop(held);
    });
    assert_eq!(
        named(&events, "balloon_status_runtime_busy").len(),
        1,
        "印を戻した後の次の縮退でまた 1 行: {events:?}"
    );
}
