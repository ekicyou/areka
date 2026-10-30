//! 位置の指令の預かり（window-work-area-clamp task 3.3・design「MoveHold」・要件 6.2／6.3／6.4／7.3）の檻。
//!
//! `\![move]` は、動かす窓と基準の窓（`--base`）の両方が最初の面を表示して拡大率が取れるまで預かる。
//! 預かった指令に関わる窓への後の指令は追い越さず、関わりの無い窓の指令は先に当たる。窓が表示した巡
//! では、同じ巡の再スナップが書いた表示の後の大きさを基準に当たる（research 12 節の 52px の隙間の元）。
//!
//! 表示の成立は GPU 無しでは作れないので、装着の前・装着の後の未表示は本物の表示層で、表示の後は
//! 読み口（閉包）で与える。

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{EmoWorld, ScaleRatio};
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use wintf::ecs::Point;

use crate::emo2_boot::move_cue::{MoveDirective, PositionCue, parse_move_directive};
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::emo2_boot::target_map::shell_target;
use crate::emo2_boot::zorder_cue::ZOrderDirective;
use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::sticky_group_ledger::StickyGroupLedger;

use super::test_support::{
    FrameHarness, PerTargetSizes, SPAWN_SIZE_1, all_shown, capture_logs, pos_of, size_of,
    synth_assets, zero_clock,
};
use super::{Emo2Wiring, run_move_drain_phase, run_move_drain_with};

/// 位置の線の送り口と、それを受ける headless な結線（表示層は与えたもの）。
fn wiring_with(presenter: EmoPresenter) -> (Sender<PositionCue>, Emo2Wiring) {
    let (tx, rx) = mpsc::channel::<PositionCue>();
    let wiring = Emo2Wiring::new(
        presenter,
        mpsc::channel::<PresentCommand>().1,
        rx,
        mpsc::channel::<TalkLifecycleSignal>().1,
        mpsc::channel::<ZOrderDirective>().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        zero_clock(),
        synth_assets(&[(0, 0)]),
    );
    (tx, wiring)
}

/// 起動の確定が済んだハーネス（連鎖の確定はこのフレームで何もしない）。
fn steady_harness() -> FrameHarness {
    let mut harness = FrameHarness::new();
    harness.world.insert_resource(ChainFinalized);
    harness.advance_frame();
    harness
}

fn directive(scope: u32, tokens: &[&str]) -> MoveDirective {
    let tokens: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
    parse_move_directive(scope, &tokens).directive
}

/// 測りの検体と同じ形: scope1 の右下を scope0 の左下へそろえる。
fn beside_scope0() -> PositionCue {
    PositionCue::Move(directive(
        1,
        &[
            "--X=0",
            "--Y=0",
            "--base=0",
            "--base-offset=left.bottom",
            "--move-offset=right.bottom",
        ],
    ))
}

/// 原点を基準に横だけ `x` へ置く（基準の窓を持たない移動・縦は据え置き）。
fn to_global(scope: u32, x: i32) -> PositionCue {
    PositionCue::Move(directive(scope, &[&format!("--X={x}"), "--base=global"]))
}

fn x_of(harness: &FrameHarness, scope: usize) -> Option<i32> {
    pos_of(&harness.world, harness.char_window(scope)).map(|p| p.x)
}

/// 表示したスコープだけ拡大率 1 を答える読み口。
fn shown_only(scopes: &'static [u32]) -> impl Fn(u32) -> Option<ScaleRatio> {
    move |s| scopes.contains(&s).then_some(ScaleRatio::ONE)
}

/// 装着の前（表示層に登録が無い）の指令は預かられ、取りこぼされずに表示の後で当たる。
#[test]
fn a_move_before_attach_is_held_and_applies_after_the_show() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    let target = harness.char_window(1);
    let before = pos_of(&harness.world, target);
    tx.send(to_global(1, 10)).expect("送出は成功する");

    run_move_drain_phase(&wiring, &mut harness.world);
    assert_eq!(pos_of(&harness.world, target), before, "装着の前に当たった");
    assert!(
        wiring.drain_move_directives().is_empty(),
        "線から取り出して預かる"
    );

    run_move_drain_with(&wiring, &all_shown, &mut harness.world);
    assert_eq!(x_of(&harness, 1), Some(10));
}

/// 装着の後でも最初の面をまだ表示していない窓（本物の表示層・拡大率が取れない）への指令は預かられる。
#[test]
fn a_move_after_attach_before_the_first_surface_is_held() {
    let mut harness = steady_harness();
    let mut presenter = EmoPresenter::new();
    for scope in [0u32, 1] {
        let window = harness.char_window(scope as usize);
        presenter
            .attach_target(
                &mut harness.world,
                shell_target(scope),
                window,
                EmoWorld::build(&areka_parsers::shell::parse("")),
                AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
                96,
            )
            .expect("headless 装着は成功する");
    }
    let (tx, wiring) = wiring_with(presenter);
    let target = harness.char_window(1);
    let before = pos_of(&harness.world, target);
    tx.send(beside_scope0()).expect("送出は成功する");

    let logs = capture_logs(|| run_move_drain_phase(&wiring, &mut harness.world));

    assert_eq!(pos_of(&harness.world, target), before, "表示の前に当たった");
    assert!(
        logs.iter()
            .any(|l| l.contains("move_ratio_unset") && l.contains("scope=1")),
        "預かった記録が無い: {logs:?}"
    );
}

/// 対象と基準のどちらか一方でも最初の面を表示していなければ預かる。
#[test]
fn a_move_waits_for_both_the_target_and_the_base() {
    for shown in [&[0u32][..], &[1u32][..]] {
        let mut harness = steady_harness();
        let (tx, wiring) = wiring_with(EmoPresenter::new());
        let target = harness.char_window(1);
        let before = pos_of(&harness.world, target);
        tx.send(beside_scope0()).expect("送出は成功する");

        run_move_drain_with(&wiring, &shown_only(shown), &mut harness.world);

        assert_eq!(
            pos_of(&harness.world, target),
            before,
            "表示したのが {shown:?} だけなのに当たった"
        );
    }
}

/// 基準の窓が表示した巡では、同じ巡の再スナップが書いた表示の後の大きさを基準に当たる
/// （表示の前の仮の矩形を基準にしない＝表示の後に隙間が残らない）。
#[test]
fn a_held_move_applies_in_the_showing_frame_after_the_resnap() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    let (base, target) = (harness.char_window(0), harness.char_window(1));
    tx.send(beside_scope0()).expect("送出は成功する");

    // 1 巡目: 相方だけ表示・本体はまだ（表示の前の大きさのまま）。
    run_move_drain_with(&wiring, &shown_only(&[1]), &mut harness.world);

    // 2 巡目: 本体が表示し、再スナップが表示の後の大きさを書いた後に当たる。
    harness.advance_frame();
    let sizes = PerTargetSizes::new([(0, Some((300, 500))), (1, Some(SPAWN_SIZE_1))]);
    harness.run_resnap(&sizes);
    run_move_drain_with(&wiring, &all_shown, &mut harness.world);

    let base_pos = pos_of(&harness.world, base).expect("本体の位置");
    let base_size = size_of(&harness.world, base).expect("本体の大きさ");
    assert_eq!(
        base_size.width, 300,
        "前提: 再スナップが本体の大きさを書いた"
    );
    assert_eq!(
        pos_of(&harness.world, target),
        Some(Point::new(
            base_pos.x - SPAWN_SIZE_1.0 as i32,
            base_pos.y + base_size.height - SPAWN_SIZE_1.1 as i32,
        )),
        "相方の右下が表示の後の本体の左下にそろっていない"
    );
}

/// 預かった指令に関わる窓への後の指令は、表示の後の当て待ちでなくても追い越さない（順のまま当たる）。
#[test]
fn a_later_move_to_a_held_window_does_not_overtake() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    let target = harness.char_window(1);
    let before = pos_of(&harness.world, target);
    tx.send(beside_scope0()).expect("送出は成功する");
    tx.send(to_global(1, 10)).expect("送出は成功する");

    // 相方は表示済み（後の指令だけなら当たる）・基準の本体はまだ。
    run_move_drain_with(&wiring, &shown_only(&[1]), &mut harness.world);
    assert_eq!(
        pos_of(&harness.world, target),
        before,
        "後の指令が追い越した"
    );

    run_move_drain_with(&wiring, &all_shown, &mut harness.world);
    assert_eq!(
        x_of(&harness, 1),
        Some(10),
        "順のまま当たっていない（最後の指令の位置で終わる）"
    );
}

/// 預かった指令に関わらない窓の指令は先に当たる（列全体を詰まらせない）。
#[test]
fn a_move_to_an_unrelated_window_goes_first() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    let scope1 = harness.char_window(1);
    let before1 = pos_of(&harness.world, scope1);
    tx.send(to_global(1, 10)).expect("送出は成功する");
    tx.send(to_global(0, 30)).expect("送出は成功する");

    run_move_drain_with(&wiring, &shown_only(&[0]), &mut harness.world);

    assert_eq!(
        pos_of(&harness.world, scope1),
        before1,
        "表示の前の相方に当たった"
    );
    assert_eq!(
        x_of(&harness, 0),
        Some(30),
        "関わりの無い本体の指令が待たされた"
    );
}

/// 組の作成の指令（窓の組の帳簿は既定の空から）。
fn set_sticky(scopes: &[&str]) -> PositionCue {
    PositionCue::SetSticky {
        tokens: scopes.iter().map(|s| s.to_string()).collect(),
    }
}

fn groups_of(harness: &FrameHarness) -> Vec<Vec<u32>> {
    harness
        .world
        .resource::<StickyGroupLedger>()
        .groups()
        .iter()
        .map(|g| g.members.iter().copied().collect())
        .collect()
}

/// 最初の面を表示しない窓への移動の後の並べ直しは、その移動を捨てて（記録を残して）当たり、続く
/// 表示済みの窓への指令も当たる（表示されない窓が 1 つで列全体を詰まらせない）。
#[test]
fn a_reset_after_a_move_to_a_never_shown_window_supersedes_it() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    let scope1 = harness.char_window(1);
    let before1 = pos_of(&harness.world, scope1);
    tx.send(to_global(1, 10)).expect("送出は成功する");
    tx.send(PositionCue::ResetWindowPos)
        .expect("送出は成功する");
    tx.send(to_global(0, 30)).expect("送出は成功する");

    let logs = capture_logs(|| {
        run_move_drain_with(&wiring, &shown_only(&[0]), &mut harness.world);
    });

    assert!(
        logs.iter()
            .any(|l| l.contains("move_hold_superseded") && l.contains("scope=1")),
        "捨てた記録が無い: {logs:?}"
    );
    assert!(
        logs.iter().any(|l| l.contains("reset_window_pos")),
        "並べ直しが預かられた: {logs:?}"
    );
    assert_eq!(x_of(&harness, 0), Some(30), "後の本体の指令が預かられた");

    // 捨てた移動は、相方が後で表示しても当たらない（並べ直しの後に古い移動が出てこない）。
    run_move_drain_with(&wiring, &all_shown, &mut harness.world);
    assert_eq!(
        pos_of(&harness.world, scope1),
        before1,
        "捨てた移動が当たった"
    );
}

/// 最初の面を表示しない窓に関わらない組の作成と、その後の表示済みの窓への移動は先に当たる。
#[test]
fn a_sticky_group_unrelated_to_a_never_shown_window_goes_first() {
    let mut harness = steady_harness();
    harness.world.insert_resource(StickyGroupLedger::default());
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    tx.send(to_global(1, 10)).expect("送出は成功する");
    tx.send(set_sticky(&["0", "2"])).expect("送出は成功する");
    tx.send(to_global(0, 30)).expect("送出は成功する");

    run_move_drain_with(&wiring, &shown_only(&[0]), &mut harness.world);

    assert_eq!(
        groups_of(&harness),
        vec![vec![0, 2]],
        "組の作成が預かられた"
    );
    assert_eq!(x_of(&harness, 0), Some(30), "後の本体の指令が預かられた");
}

/// 預かった移動の窓を組に入れる指令と、その組の窓への後の移動は追い越さず、表示の後に順のまま当たる
/// （先の移動は組の前なので相方だけ動き、後の移動は組の相方を連れ動かす）。
#[test]
fn a_sticky_group_with_a_held_window_waits_and_keeps_the_order() {
    let mut harness = steady_harness();
    harness.world.insert_resource(StickyGroupLedger::default());
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    let x0 = x_of(&harness, 0).expect("本体の位置");
    tx.send(to_global(1, 10)).expect("送出は成功する");
    tx.send(set_sticky(&["0", "1"])).expect("送出は成功する");
    tx.send(to_global(0, 30)).expect("送出は成功する");

    run_move_drain_with(&wiring, &shown_only(&[0]), &mut harness.world);
    assert!(groups_of(&harness).is_empty(), "組の作成が追い越した");
    assert_eq!(x_of(&harness, 0), Some(x0), "本体の指令が追い越した");

    run_move_drain_with(&wiring, &all_shown, &mut harness.world);
    assert_eq!(groups_of(&harness), vec![vec![0, 1]]);
    assert_eq!(x_of(&harness, 0), Some(30));
    assert_eq!(
        x_of(&harness, 1),
        Some(10 + 30 - x0),
        "順のまま当たっていない（組の後の移動が相方を連れ動かす）"
    );
}

/// 最初の面を表示しない窓への移動の後のバルーン位置初期化は、その移動を待たずに当たり（バルーンは
/// 窓ごとに自分のキャラ窓の今の位置から置き直し、後の移動が連れ動かすので入れ替えても同じ）、続く
/// 表示済みの窓への指令も当たる。預かった移動は、その窓が表示した巡で当たる。
#[test]
fn a_balloon_reset_after_a_move_to_a_never_shown_window_goes_first() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring_with(EmoPresenter::new());
    let scope1 = harness.char_window(1);
    let before1 = pos_of(&harness.world, scope1);
    tx.send(to_global(1, 10)).expect("送出は成功する");
    tx.send(PositionCue::ResetBalloonPos)
        .expect("送出は成功する");
    tx.send(to_global(0, 30)).expect("送出は成功する");

    let logs = capture_logs(|| {
        run_move_drain_with(&wiring, &shown_only(&[0]), &mut harness.world);
    });

    assert!(
        logs.iter().any(|l| l.contains("balloon_pos_reset")),
        "バルーン位置初期化が預かられた: {logs:?}"
    );
    assert_eq!(x_of(&harness, 0), Some(30), "後の本体の指令が預かられた");
    assert_eq!(
        pos_of(&harness.world, scope1),
        before1,
        "表示の前の相方に当たった"
    );

    run_move_drain_with(&wiring, &all_shown, &mut harness.world);
    assert_eq!(
        x_of(&harness, 1),
        Some(10),
        "預かった移動が表示の後に当たらない"
    );
}
