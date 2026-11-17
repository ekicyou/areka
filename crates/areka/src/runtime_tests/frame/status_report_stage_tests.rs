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
    BindResolver, MockSurfaceOutput, SerikoLoopConfig, SurfaceResolver, spawn_seriko,
};

use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};
// 私有の欄は渡し口から読み書きする（設計 D3 ⑷）。
use areka_runtime::frame::status_report::test_support::last_stage;

/// seriko の送り手だけを持つ置き場のゴースト（出力は捕まえるだけ）。
fn world_with_seriko() -> World {
    let (sink, _actor) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        areka_seriko::DefaultBinds::uniform(BindSet::default()),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        MockSurfaceOutput::new(),
    );
    let mut world = World::new();
    crate::session::hooks::install(&mut world);
    world.insert_non_send(GhostSlot(Some(
        GhostSession::for_test(None, PathBuf::from("ghost/test")).with_seriko_sink(sink),
    )));
    world
}

/// 表示層へ scope のバルーンを headless で装着する（見えていない・面の番号なし・世代 0 から）。
fn attach_balloon(presenter: &mut EmoPresenter, scope: u32) {
    let mut world = World::new();
    let window = world.spawn_empty().id();
    presenter.attach_target(
        balloon_target(scope),
        window,
        EmoWorld::build(&areka_parsers::shell::parse("")),
        AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
        96,
    );
}

#[test]
fn empty_slot_in_the_phase_keeps_the_ledger() {
    let mut world = World::new();
    crate::session::hooks::install(&mut world);
    world.insert_non_send(GhostSlot(None));
    let mut presenter = EmoPresenter::new();
    attach_balloon(&mut presenter, 0);
    let mut ledger = BalloonStatusLedger::default();
    report_stages(&presenter, &world, &mut ledger, &[0]);
    assert!(last_stage(&ledger).is_empty());
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
        *last_stage(&wiring.balloon_status),
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
    assert!(last_stage(&ledger).is_empty());
}
