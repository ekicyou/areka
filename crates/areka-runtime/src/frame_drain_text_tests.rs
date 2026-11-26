use std::sync::mpsc;
use wintf::ecs::Point;

use crate::move_cue::MoveDirective;
use crate::talk_clock::{controllable_clock, set_now};

use dola::TimeSpan;
use dola::time::testing::{at_ms, ms};

use super::shared_test_support::wiring;
use super::test_support::{
    all_shown, capture_logs, count_level, headless_wiring_with, pos_of, resnap_world, zero_clock,
};
use super::*;

/// drain: attach 前は drain せず（保留＝取りこぼしなし）、attach 後に FIFO 到着順で
/// **全件** `presenter.apply` へ適用し切る。
///
/// 未装着 target への `Hide`（reply: None）は `EmoPresenter::apply_hide` が
/// `error!(?target_id, "apply(Hide): 未装着ターゲット")` を発火する（reply-less でも log-first・
/// panic しない）。この ERROR を capture subscriber で観測し、(a) attach 前は 0 件（gate 閉＝
/// チャネル未 drain）、(b) attach 後は送信順 `TargetId(0)→(1)→(2)` でちょうど 3 件（drain-all＋
/// FIFO 到着順）、(c) 2 度目の drain は 0 件（空チャネル・二重適用なし）を決定論的に反証する。
#[test]
fn run_drain_phase_gates_on_attach_then_drains_all_in_fifo_order() {
    let (tx, rx) = mpsc::channel::<PresentCommand>();
    let mut wiring = headless_wiring_with(rx, zero_clock());
    // GhostWindows/GPU を持たない素の World（drain は presenter.apply のみ・GPU 不要）。
    let mut world = World::new();

    // FIFO で 3 件送る（未装着 target ゆえ apply は error!＋return の no-op-with-log・panic しない）。
    for t in [0u32, 1, 2] {
        tx.send(PresentCommand::Hide {
            target: TargetId(t),
            reply: None,
        })
        .expect("送信は成功する（受信端 rx は wiring が保持）");
    }

    assert!(!wiring.attached, "前提: 未装着（run_attach_phase 未実行）");
    let logs_gated = capture_logs(|| run_drain_phase(&mut wiring, &mut world));
    assert_eq!(
        count_level(&logs_gated, "ERROR"),
        0,
        "attach 前は drain せず apply も呼ばない（チャネルが保留バッファ・取りこぼしなし・DD-1）: {logs_gated:?}"
    );

    wiring.attached = true;

    let logs_drained = capture_logs(|| run_drain_phase(&mut wiring, &mut world));
    let errs: Vec<&String> = logs_drained
        .iter()
        .filter(|l| l.contains("level=ERROR"))
        .collect();
    assert_eq!(
        errs.len(),
        3,
        "gate 開後は 3 件全て apply（drain-all）: {logs_drained:?}"
    );
    for (i, expected) in [0u32, 1, 2].iter().enumerate() {
        assert!(
            errs[i].contains(&format!("TargetId({expected})")),
            "apply は FIFO 到着順（{i} 番目は TargetId({expected})）: {}",
            errs[i]
        );
    }

    let logs_empty = capture_logs(|| run_drain_phase(&mut wiring, &mut world));
    assert_eq!(
        count_level(&logs_empty, "ERROR"),
        0,
        "drain 済みチャネルは空・再適用しない: {logs_empty:?}"
    );
}

/// text 判断: `resolve_talk_time` は override 優先→`clock.talk_time`→`None` を返す。
///
/// GPU/時刻 I/O 抜きの純関数として 4 経路を決定論檻へ入れる: override 勝ち（frame_now 無視）・
/// override 無し×epoch 確立×frame_now 有り＝差分・frame_now 不在＝None・epoch 未確立＝None。
#[test]
fn resolve_talk_time_override_wins_else_clock_else_none() {
    let clock_unset = zero_clock();

    assert_eq!(
        resolve_talk_time(Some(ms(5_000)), Some(at_ms(999_000)), &clock_unset),
        Some(ms(5_000)),
        "override は最優先で採用（frame_now は無視）"
    );
    assert_eq!(
        resolve_talk_time(Some(ms(5_000)), None, &clock_unset),
        Some(ms(5_000)),
        "override は frame_now 不在でも採用"
    );

    let (now, clock) = controllable_clock();
    set_now(&now, at_ms(100_000));
    clock.observe_cue(TimeSpan::ZERO); // epoch = 100 s - 0 s = 100 s
    assert_eq!(
        resolve_talk_time(None, Some(at_ms(105_000)), &clock),
        Some(ms(5_000)),
        "override 無しは clock.talk_time(frame_now)（105-100=5）"
    );

    assert_eq!(
        resolve_talk_time(None, None, &clock),
        None,
        "frame_now 不在は None（present_frame を呼ばない）"
    );

    assert_eq!(
        resolve_talk_time(None, Some(at_ms(105_000)), &clock_unset),
        None,
        "epoch 未確立は None（talk 未到達）"
    );
}

/// text smoke（no panic）: `run_text_phase` は override で `present_frame` へ到達し、
/// override 無し×`FrameTime` 不在では skip する（いずれも panic しない）。
///
/// 登録 actor の無い空 `TextLayerRuntime` に対し `present_frame` は `Ok(())` で即復帰する
/// （GPU 不要・upstream 契約）。override=Some(2.0) で present_frame を踏み、override=None かつ
/// `FrameTime` 資源なしで skip することを、panic なし＋ runtime 再借用可で担保する。
#[test]
fn run_text_phase_override_reaches_present_frame_without_panic() {
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    let mut wiring = headless_wiring_with(rx, zero_clock());
    // FrameTime 資源を持たない素の World（override 経路と skip 経路の双方を踏む）。
    let mut world = World::new();

    run_text_phase(&mut wiring, &mut world, Some(ms(2_000)));

    run_text_phase(&mut wiring, &mut world, None);

    // present_frame は borrow を残さない（RefCell を再借用できる＝lingering borrow / poison なし）。
    assert!(
        wiring.runtime.try_borrow_mut().is_ok(),
        "present_frame 後に runtime を再借用できる（借用を残さない）"
    );
}

/// 排他 system の疎通: `frame_system` は NonSend `RuntimeWiring` を remove→各相→insert で
/// 駆動して**必ず戻す**、かつ未挿入 World では安全に no-op（panic しない）。
///
/// GPU/GhostWindows を持たない World ではゲート不成立で attach は起きず、drain は attach 前ゆえ
/// 走らず、text は FrameTime 不在で skip する（＝実質 no-op）。それでも system が wiring を取り出して
/// 戻す配線（remove→insert）が働くことを、実行後に NonSend resource が再取得できることで反証する。
#[test]
fn frame_system_removes_runs_and_reinserts_wiring() {
    let (_tx, rx) = mpsc::channel::<PresentCommand>();
    let wiring = headless_wiring_with(rx, zero_clock());
    let mut world = World::new();
    world.insert_non_send(wiring);

    frame_system(&mut world);
    assert!(
        world.get_non_send::<RuntimeWiring>().is_some(),
        "frame_system は wiring を取り出して駆動後に必ず戻す（配線の疎通）"
    );

    frame_system(&mut world);
    assert!(
        world.get_non_send::<RuntimeWiring>().is_some(),
        "再実行でも wiring を保つ（remove→insert の冪等）"
    );

    // 資源が無い World でも安全に no-op（wire_runtime 前・LogSink フォールバック boot 経路）。
    let mut empty_world = World::new();
    frame_system(&mut empty_world); // panic しない
    assert!(
        empty_world.get_non_send::<RuntimeWiring>().is_none(),
        "未挿入なら no-op（何も挿入しない）"
    );
}

// ── run_move_drain_phase（frame 相 move drain→apply）の存在＋ゲート檻 ──────
//
// 送出端の到達（`move_cue_move_sink_tests.rs`）と apply 単体
// （`move_cue_apply_move_tests.rs`・`move_cue_place_tests.rs`）を frame 相 drain で接ぐ結線の存在チェック。full spine
// （cue→CueSheet→dispatch→sink→channel→frame）は本体の `runtime_tests/spine_move_cue_tests.rs` が持つ。

/// fixture `\1\![move,-353,,,0,base,base]` の `MoveDirective`（scope1・base scope0）。
fn fixture_move_directive() -> MoveDirective {
    crate::move_cue::parse_move_directive(
        1,
        &["-353", "", "", "0", "base", "base"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    )
    .directive
}

/// ゲート檻: `GhostWindows` 未挿入の間は `move_rx` を drain せず保留する（取りこぼしなし）。
///
/// 素の `World`（`GhostWindows` なし）で `run_move_drain_phase` を呼んでも、送出済みの
/// `MoveDirective` はチャネルに残る（後から test-support `drain_move_directives` で取り出せる＝
/// gate 閉で未消費の実証）。move はキャラ窓生成後に一括適用され OnFirstBoot 移動を取りこぼさない。
#[test]
fn run_move_drain_phase_buffers_until_ghost_windows_present() {
    let (tx, wiring) = wiring();
    tx.send(PositionCue::Move(fixture_move_directive()))
        .expect("送出は成功する（受信端は wiring 保持）");

    let mut world = World::new();
    run_move_drain_phase(&wiring, &mut world);

    let remaining = wiring.drain_move_directives();
    assert_eq!(
        remaining.len(),
        1,
        "GhostWindows 未挿入では drain せず保留する（取りこぼしなし）"
    );
    assert_eq!(
        remaining[0],
        PositionCue::Move(fixture_move_directive()),
        "保留された指令は fixture（scope1）"
    );
}

/// apply 檻: `GhostWindows` 存在下で `move_rx` を drain すると `apply_move_directive` が
/// 対象窓を fixture 検算位置へ即時移動する（channel→frame 相 drain→apply→窓移動の結線存在）。
///
/// base scope0 (1483,757,434,687)・target scope1 (1049,1087,278,357)・x=Px(-353)・y=Fix:
/// x' = 1483 + 434/2 − 353 − 278/2 = 1208・y は現状維持 1087（`move_cue_geometry.rs` の `compute_move_target` 検算）。
#[test]
fn run_move_drain_phase_applies_directive_when_ghost_windows_present() {
    let (mut world, gw) = resnap_world();
    let target = gw.char_window(1).unwrap();
    assert_eq!(
        pos_of(&world, target),
        Some(Point { x: 1049, y: 1087 }),
        "前提: 移動前の scope1 初期位置"
    );

    let (tx, wiring) = wiring();
    tx.send(PositionCue::Move(fixture_move_directive()))
        .expect("送出は成功する");

    // 前提: 対象と基準の窓は最初の面を表示済み（位置の指令を預からない）。
    run_move_drain_with(&wiring, &all_shown, &mut world);

    assert_eq!(
        pos_of(&world, target),
        Some(Point { x: 1208, y: 1087 }),
        "x'=1483+217−353−139=1208・y=Fix は現状維持（channel→frame drain→apply）"
    );
    assert_eq!(
        wiring.drain_move_directives().len(),
        0,
        "drain 後チャネルは空（全件消費・二重適用なし）"
    );
}

/// 並べ直しの指令は並べ直しの手続きへ渡り（`OnResetWindowPos` は送らない）、その失敗は後ろの
/// 移動を止めない。
///
/// `resnap_world` は並べ直しの材料（`DefaultPlacementBasis`）を置かないので、並べ直しは
/// `reset_window_pos_skipped` の WARN で何もしない。その後ろの移動は fixture の検算位置へ届く。
#[test]
fn run_move_drain_phase_routes_reset_and_continues_after_its_failure() {
    let (mut world, gw) = resnap_world();
    let target = gw.char_window(1).unwrap();
    let (tx, wiring) = wiring();
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");
    tx.send(PositionCue::Move(fixture_move_directive()))
        .expect("送出は成功する");

    // 前提: 対象と基準の窓は最初の面を表示済み（位置の指令を預からない）。
    let logs = capture_logs(|| run_move_drain_with(&wiring, &all_shown, &mut world));

    assert_eq!(
        logs.iter()
            .filter(|l| l.contains("level=WARN") && l.contains("reset_window_pos_skipped"))
            .count(),
        1,
        "並べ直しは 2.1 の手続きへ渡った（材料が無いので記録だけ）: {logs:?}"
    );
    assert_eq!(
        pos_of(&world, target),
        Some(Point { x: 1208, y: 1087 }),
        "並べ直しの失敗の後ろの移動も適用される"
    );
    assert!(wiring.drain_move_directives().is_empty(), "全件消費");
}

/// 表示の前
/// （`applied_ratio` が無い）の指令は当てずに預かり、`move_ratio_unset` をスコープつきで 1 行だけ残す。
/// 窓が表示した後の巡で当たり、当てた記録には拡大率と動かす窓の大きさが載る。
///
/// target scope1 は (1049,1087,278,357)。表示の後の k は 1 とする。
#[test]
fn run_move_drain_phase_logs_ratio_unset_and_applied_scale_and_size() {
    let (mut world, gw) = resnap_world();
    let target = gw.char_window(1).unwrap();
    let (tx, wiring) = wiring();
    tx.send(PositionCue::Move(fixture_move_directive()))
        .expect("送出は成功する");

    // 表示していない presenter（拡大率が取れない）→ 預かって記録だけ残す。
    let before = capture_logs(|| run_move_drain_phase(&wiring, &mut world));
    let again = capture_logs(|| run_move_drain_phase(&wiring, &mut world));

    let unset: Vec<&String> = before
        .iter()
        .filter(|l| l.contains("move_ratio_unset"))
        .collect();
    assert_eq!(unset.len(), 1, "表示の前の指令は 1 行残す: {before:?}");
    assert!(
        unset[0].contains("level=INFO") && unset[0].contains("scope=1"),
        "info でスコープつき: {unset:?}"
    );
    assert!(
        !again.iter().any(|l| l.contains("move_ratio_unset")),
        "預かり続ける巡ごとには記録しない: {again:?}"
    );
    assert!(
        !before
            .iter()
            .chain(&again)
            .any(|l| l.contains("move_applied")),
        "表示の前に当たっている"
    );
    assert_eq!(pos_of(&world, target), Some(Point { x: 1049, y: 1087 }));

    let logs = capture_logs(|| run_move_drain_with(&wiring, &all_shown, &mut world));
    let applied: Vec<&String> = logs.iter().filter(|l| l.contains("move_applied")).collect();
    assert_eq!(applied.len(), 1, "{logs:?}");
    for field in [
        "k=ScaleRatio { num: 1, den: 1 }",
        "target_w=278",
        "target_h=357",
    ] {
        assert!(applied[0].contains(field), "{field} が無い: {}", applied[0]);
    }
    assert!(
        !logs.iter().any(|l| l.contains("move_ratio_unset")),
        "表示の後の当てでは記録しない: {logs:?}"
    );
}
