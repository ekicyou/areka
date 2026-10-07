// =============================================================================
// 窓の知らせの決定論テスト（areka-P0-animated-image-playback task 5.3・要件 2.3・6.1）
//
// ⑴ 観測の列から送る知らせの列: 前と同じなら送らない／開いたままでも世代が進めば送る／
//    隠した後（面の番号が消える）は前に知らせた面の番号を保つ
// ⑵ 置き場のゴースト（seriko の送り手）が居ないフレームは台帳を変えずに見送る
// ⑶ 届けの相: 文字の層を借りられないフレームでも知らせる・表示層に未装着の scope は知らせない
// =============================================================================

use std::collections::BTreeMap;
use std::path::PathBuf;

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_seriko::{
    BindResolver, MockSurfaceOutput, SerikoLoopConfig, StageNote, SurfaceResolver, spawn_seriko,
};

use super::*;
use crate::ghost_session::GhostSession;

fn stage(scope: u32, open: bool, surface_id: Option<u32>, generation: u64) -> StageObservation {
    StageObservation {
        scope,
        open,
        surface_id,
        generation,
    }
}

fn note(scope: u32, open: bool, face: u32, generation: u64) -> StageNote {
    StageNote::Balloon {
        scope: ActorKey::from(scope.to_string()),
        open,
        face,
        generation,
    }
}

/// 観測の列をフレームごとに流し、送った知らせを全部集める（送り手は毎フレーム在る）。
fn notes_of(frames: &[Vec<StageObservation>]) -> (Vec<StageNote>, BalloonStatusLedger) {
    let mut ledger = BalloonStatusLedger::default();
    let mut notes = Vec::new();
    for observed in frames {
        report_stage_observed(&mut ledger, observed, Some(|n| notes.push(n)));
    }
    (notes, ledger)
}

/// seriko の送り手だけを持つ置き場のゴースト（出力は捕まえるだけ）。
fn world_with_seriko() -> World {
    let (sink, _actor) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::default(),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        MockSurfaceOutput::new(),
    );
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(
        GhostSession::for_test(None, PathBuf::from("ghost/test")).with_seriko_sink(sink),
    )));
    world
}

/// 表示層へ scope のバルーンを headless で装着する（見えていない・面の番号なし・世代 0 から）。
fn attach_balloon(presenter: &mut EmoPresenter, scope: u32) {
    let mut world = World::new();
    let window = world.spawn_empty().id();
    presenter
        .attach_target(
            &mut world,
            balloon_target(scope),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");
}

// ---------------------------------------------------------------------------
// ⑴ 観測の列から送る知らせの列
// ---------------------------------------------------------------------------

#[test]
fn unchanged_observation_is_not_sent_again() {
    let frame = vec![stage(0, true, Some(3), 1), stage(1, false, None, 0)];
    let (notes, _) = notes_of(&[frame.clone(), frame.clone(), frame]);
    assert_eq!(notes, vec![note(0, true, 3, 1), note(1, false, 0, 0)]);
}

/// 同じフレームの中で隠して出し直すと、開いたまま・同じ面でも世代だけが進む（要件 2.3）。
#[test]
fn advanced_generation_is_sent_even_while_staying_open() {
    let (notes, _) = notes_of(&[
        vec![stage(0, true, Some(3), 1)],
        vec![stage(0, true, Some(3), 2)],
    ]);
    assert_eq!(notes, vec![note(0, true, 3, 1), note(0, true, 3, 2)]);
}

/// 隠すと表示層は面の番号を消す（`Hide`）。閉じた知らせは前に知らせた面の番号を運ぶ。
#[test]
fn closing_keeps_the_face_sent_before() {
    let (notes, ledger) = notes_of(&[
        vec![stage(0, true, Some(3), 1)],
        vec![stage(0, false, None, 1)],
        vec![stage(0, false, None, 1)],
        vec![stage(0, true, Some(4), 2)],
    ]);
    assert_eq!(
        notes,
        vec![
            note(0, true, 3, 1),
            note(0, false, 3, 1),
            note(0, true, 4, 2)
        ]
    );
    assert_eq!(ledger.last_stage.get(&0), Some(&(true, 4, 2)));
}

// ---------------------------------------------------------------------------
// ⑵ 置き場のゴーストが居ないフレーム
// ---------------------------------------------------------------------------

#[test]
fn frame_without_ghost_keeps_the_ledger_and_sends_on_the_next_frame() {
    let mut ledger = BalloonStatusLedger::default();
    let frame = vec![stage(0, true, Some(3), 1)];
    report_stage_observed(&mut ledger, &frame, None::<fn(StageNote)>);
    assert!(
        ledger.last_stage.is_empty(),
        "見送ったフレームは台帳を変えない"
    );

    let mut notes = Vec::new();
    report_stage_observed(&mut ledger, &frame, Some(|n| notes.push(n)));
    assert_eq!(
        notes,
        vec![note(0, true, 3, 1)],
        "ゴーストが据わったフレームで送る"
    );
}

#[test]
fn empty_slot_in_the_phase_keeps_the_ledger() {
    let mut world = World::new();
    world.insert_non_send(GhostSlot(None));
    let mut presenter = EmoPresenter::new();
    attach_balloon(&mut presenter, 0);
    let mut ledger = BalloonStatusLedger::default();
    report_stages(&presenter, &world, &mut ledger, &[0]);
    assert!(ledger.last_stage.is_empty());
}

// ---------------------------------------------------------------------------
// ⑶ 届けの相
// ---------------------------------------------------------------------------

/// 知らせは文字の層を借りる手前にある＝借りられないフレームでも届く（台帳が送った組を覚える）。
#[test]
fn phase_sends_the_stage_even_while_the_runtime_is_busy() {
    use super::super::test_support::{headless_wiring_with, zero_clock};

    let world = world_with_seriko();
    let mut wiring = headless_wiring_with(std::sync::mpsc::channel().1, zero_clock());
    wiring
        .balloon_models
        .insert(0, areka_parsers::balloon::parse_str("", None));
    attach_balloon(&mut wiring.presenter, 0);
    let runtime = std::rc::Rc::clone(wiring.runtime());

    let held = runtime.borrow_mut();
    run_status_report_phase(&mut wiring, &world);
    drop(held);
    assert_eq!(
        wiring.balloon_status.last_stage,
        BTreeMap::from([(0, (false, 0, 0))])
    );
}

/// 表示層に未装着の scope は知らせない（装着されたフレームで知らせる）。
#[test]
fn unattached_scope_is_not_noted() {
    let world = world_with_seriko();
    let presenter = EmoPresenter::new();
    let mut ledger = BalloonStatusLedger::default();
    report_stages(&presenter, &world, &mut ledger, &[0, 1]);
    assert!(ledger.last_stage.is_empty());
}
