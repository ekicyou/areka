// =============================================================================
// 窓の知らせの決定論テスト（観測の列から送る知らせの列）。
//
// 前と同じなら送らない・開いたままでも世代が進めば送る・隠した後（面の番号が消える）は
// 前に知らせた面の番号を保つ・置き場のゴーストが居ないフレームは台帳を変えずに見送る。
// 届けの相（文字の層を借りられないフレーム・未装着の scope）は本体の
// `runtime_tests/frame/status_report_stage_tests.rs` が持つ。
// =============================================================================

use areka_seriko::StageNote;

use super::*;

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

// ---------------------------------------------------------------------------
// ⑴ 観測の列から送る知らせの列
// ---------------------------------------------------------------------------

#[test]
fn unchanged_observation_is_not_sent_again() {
    let frame = vec![stage(0, true, Some(3), 1), stage(1, false, None, 0)];
    let (notes, _) = notes_of(&[frame.clone(), frame.clone(), frame]);
    assert_eq!(notes, vec![note(0, true, 3, 1), note(1, false, 0, 0)]);
}

/// 同じフレームの中で隠して出し直すと、開いたまま・同じ面でも世代だけが進む。
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

// ---------------------------------------------------------------------------
// ⑶ フレームの時刻が無い世界
// ---------------------------------------------------------------------------

/// 置き場のゴーストの seriko が据わった World と、scope 0 のバルーンを headless 装着した表示層
/// （隠れたまま・世代 0）。フレームの時刻だけを足すかどうかで分ける。
fn stage_fixture(
    frame_time: Option<dola::TimePoint>,
) -> (
    crate::session_hooks::test_support::SerikoWorld,
    EmoPresenter,
) {
    let mut seriko = crate::session_hooks::test_support::SerikoWorld::new();
    if let Some(t) = frame_time {
        seriko.world.insert_resource(wintf::ecs::FrameTime(t));
    }
    let window = seriko.world.spawn_empty().id();
    let mut presenter = EmoPresenter::new();
    presenter.attach_target(
        balloon_target(0),
        window,
        areka_emo_compose::EmoWorld::build(&areka_parsers::shell::parse("")),
        areka_emo_atlas::AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
        96,
    );
    (seriko, presenter)
}

/// 観測も送り手も在るのにフレームの時刻が無ければ、`balloon_stage_no_frame_time` の `error!` を
/// 1 件残し、台帳を変えない。
#[test]
fn report_stages_without_frame_time_logs_an_error_and_sends_nothing() {
    let (seriko, presenter) = stage_fixture(None);
    let mut ledger = BalloonStatusLedger::default();
    let ((), events) = log_capture_kit::capture(|| {
        report_stages(&presenter, &seriko.world, &mut ledger, &[0]);
    });
    let errors = events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::ERROR
                && e.field_str("event") == Some("balloon_stage_no_frame_time")
        })
        .count();
    assert_eq!(errors, 1, "{events:?}");
    assert!(ledger.last_stage.is_empty());
    seriko.finish();
}

/// 同じ組み立てにフレームの時刻を足すと送り、台帳に隠れた scope 0 の組が載る。
#[test]
fn report_stages_with_frame_time_sends_the_hidden_balloon() {
    let (seriko, presenter) = stage_fixture(Some(dola::TimePoint::from_ticks(1)));
    let mut ledger = BalloonStatusLedger::default();
    report_stages(&presenter, &seriko.world, &mut ledger, &[0]);
    assert_eq!(
        ledger.last_stage.iter().collect::<Vec<_>>(),
        vec![(&0, &(false, 0, 0))]
    );
    seriko.finish();
}
