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

use super::*;
use crate::ghost_session::GhostSession;
use crate::placement::test_support::{LogEvent, capture_logs};

fn obs(scope: u32, visible: Option<bool>, surface_id: Option<u32>) -> BalloonObservation {
    BalloonObservation {
        scope,
        visible,
        surface_id,
        box_showing: false,
    }
}

/// 箱に文字が出ている観測（areka-P0-shell-balloon 要件 5.5・5.6）。
fn obs_box(scope: u32, visible: Option<bool>, surface_id: Option<u32>) -> BalloonObservation {
    BalloonObservation {
        box_showing: true,
        ..obs(scope, visible, surface_id)
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
    let (bindings, unknown) = collect_bindings(
        &[
            obs(0, Some(true), Some(2)),
            obs(1, Some(false), Some(5)), // 不可視
            obs(2, None, None),           // 未装着
            obs(3, Some(true), Some(0)),
        ],
        &BTreeMap::new(),
    );
    assert_eq!(bindings, vec![binding(0, 2), binding(3, 0)]);
    assert!(unknown.is_empty());
}

#[test]
fn collect_includes_visible_scope_without_number_as_zero_and_returns_it_as_warning_target() {
    let (bindings, unknown) = collect_bindings(
        &[
            obs(0, Some(true), None),
            obs(1, Some(true), Some(4)),
            obs(2, Some(false), None), // 見えていなければ警告の対象にもしない
        ],
        &BTreeMap::new(),
    );
    assert_eq!(bindings, vec![binding(0, 0), binding(1, 4)]);
    assert_eq!(unknown, vec![0]);
}

/// 箱だけに文字が出ているスコープも、普通のバルーンが見えているときと同じ形で載る
/// （番号は普通のバルーンの今の面）。どちらにも出ていなければ載らない（areka-P0-shell-balloon 要件 5.5・5.6）。
/// 箱だけ・番号なし・覚えた番号なしは 0 で載り、警告の対象にしない（areka-P0-shell-balloon-frame-align 要件 3.1・3.2）。
#[test]
fn collect_lists_box_only_scopes_in_the_same_shape_as_visible_balloons() {
    let (bindings, unknown) = collect_bindings(
        &[
            obs(0, Some(true), Some(2)),      // 窓が見えている
            obs_box(1, Some(false), Some(5)), // 箱だけ
            obs(2, Some(false), Some(7)),     // どちらにも出ていない
            obs_box(3, Some(false), None),    // 箱だけ・番号なし
            obs_box(4, Some(true), Some(1)),  // 両方
        ],
        &BTreeMap::new(),
    );
    assert_eq!(
        bindings,
        vec![binding(0, 2), binding(1, 5), binding(3, 0), binding(4, 1)]
    );
    assert!(
        unknown.is_empty(),
        "箱だけは警告の対象にしない: {unknown:?}"
    );
}

/// 箱だけ・番号なしは覚えた番号で載り、警告の対象にしない。窓が見えていて番号なしは、覚えた番号が
/// あっても今までどおり 0 で警告の対象（areka-P0-shell-balloon-frame-align 要件 3.1・3.2）。
#[test]
fn collect_uses_the_remembered_number_for_box_only_scopes_without_a_number() {
    let remembered = BTreeMap::from([(0, 3), (1, 9), (2, 4)]);
    let (bindings, unknown) = collect_bindings(
        &[
            obs_box(0, Some(false), Some(6)), // 箱だけ・今の番号あり → 今の番号
            obs_box(1, Some(false), None),    // 箱だけ・今の番号なし → 覚えた番号
            obs(2, Some(true), None),         // 窓が見えて番号なし → 0 で警告
        ],
        &remembered,
    );
    assert_eq!(bindings, vec![binding(0, 6), binding(1, 9), binding(2, 0)]);
    assert_eq!(unknown, vec![2]);
}

#[test]
fn collect_of_nothing_visible_is_empty() {
    let (bindings, unknown) = collect_bindings(
        &[obs(0, Some(false), Some(1)), obs(1, None, None)],
        &BTreeMap::new(),
    );
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

/// 番号が取れたフレームで覚え（窓が見えていなくても）、次に箱だけ・番号なしになったらその番号で載せる。
/// 警告の段の行は出さない（areka-P0-shell-balloon-frame-align 要件 3.1・3.2）。
#[test]
fn box_only_without_number_reports_the_number_remembered_from_an_earlier_frame() {
    let (world, rx) = world_with_ghost();
    let mut ledger = BalloonStatusLedger::default();

    let (_, events) = capture_logs(|| {
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
    let mut ledger = BalloonStatusLedger {
        last_sent: vec![binding(0, 2)],
        ..Default::default()
    };
    let runtime = TextLayerRuntime::new(TextLayerConfig::default());
    report_balloons(&presenter, &runtime, &world, &mut ledger, &[0, 1]);
    assert_eq!(sent(&rx), vec![Vec::new()]);
    assert!(ledger.last_sent.is_empty());
}
