//! シェルとバルーンの切り替えで常に手前の状態と印が残ること（stay-on-top-directives task 5.3・
//! 要件 4.4・4.5・5.1・7.2・7.4・design「常に手前の状態の寿命」）。
//!
//! 土台は祖父（`switch_finish_tests.rs`・宣言は `switch_reload_tests.rs`）の頼んだ段の組み立てを、窓を組んだ World の上で使う。
//! 常に手前は本物の取り出しの相へ指令を流してオンにし（窓に印 `StayOnTop` が付く）、台本の組を
//! 1 本載せてから、差し替えの相の完了の後始末まで回す。ゴーストの側の寿命は
//! `frame_stay_on_top_lifecycle_tests.rs` が固定する。

use bevy_ecs::entity::Entity;
use wintf::ecs::window::StayOnTop;

use super::super::*;
use crate::emo2_boot::frame::window_state::run_window_state_phase;
use crate::emo2_boot::window_state_cue::WindowStateDirective;
use crate::placement::spawn::GhostWindows;
use crate::placement::zorder_group_ledger::parse_zorder_tokens;

/// ゴーストの全部の窓（キャラクターとバルーン）。
fn all_windows(windows: &GhostWindows) -> Vec<Entity> {
    windows
        .scopes()
        .flat_map(|scope| [windows.char_window(scope), windows.balloon_window(scope)])
        .flatten()
        .collect()
}

/// (常に手前か, 印の付いた窓の数, 台本の組の数)。
fn observe(rig: &Rig, windows: &[Entity]) -> (bool, usize, usize) {
    let marked = windows
        .iter()
        .filter(|window| rig.world.get::<StayOnTop>(**window).is_some())
        .count();
    let tags = rig
        .wiring
        .zorder_ledger
        .groups()
        .iter()
        .filter(|group| group.source == GroupSource::Tag)
        .count();
    (rig.wiring.window_state.on, marked, tags)
}

/// 窓を組んだ World で `kind` の差し替えを頼んだ段まで組み、常に手前をオンにして台本の組を載せる。
fn rig_on_top(kind: SkinKind, finish: SwapFinish) -> (Rig, Vec<Entity>) {
    let (world, windows) = reseed_tests::booted_world(96);
    let windows = all_windows(&windows);
    let mut rig = rig_on(world, kind, false, finish);
    let (tx, rx) = mpsc::channel();
    rig.wiring.attach_window_state_rx(rx);
    tx.send(WindowStateDirective::StayOnTop { on: true })
        .expect("受信端は結線状態が持つ");
    run_window_state_phase(&mut rig.wiring.window_state, &mut rig.world);
    let (members, _) = parse_zorder_tokens(&["0", "1"]).expect("0,1 は読める");
    rig.wiring
        .zorder_ledger
        .try_add_tag_group(members)
        .expect("台本の組を載せる");
    assert_eq!(
        observe(&rig, &windows),
        (true, 4, 1),
        "前提: オンで 4 窓に印・台本の組が 1 本"
    );
    (rig, windows)
}

/// 差し替えの返信を全部返して後始末まで回し、(観測, 印の段) を返す。
fn finish(mut rig: Rig, windows: &[Entity]) -> ((bool, usize, usize), Option<&'static str>) {
    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    frame(&mut rig);
    (observe(&rig, windows), stage(&rig.world))
}

/// シェルの切り替え（要件 4.4・5.1）: 後始末を終えても常に手前のオン・全窓の印・台本の組が残る。
///
/// # 非空虚性
/// シェルの後始末が状態を捨てる・印を外す・台本の組を落とす（申し送りの件 1 の旧い形
/// `set_descript_base`）と、いずれかの欄が変わって赤。
#[test]
fn shell_switch_keeps_stay_on_top_the_marks_and_the_script_group() {
    let (rig, windows) = rig_on_top(SkinKind::Shell, shell_finish());
    assert_eq!(
        finish(rig, &windows),
        ((true, 4, 1), None),
        "((常に手前, 印の付いた窓, 台本の組), 印の段)"
    );
}

/// バルーンの切り替え（要件 4.5）: 後始末を終えても常に手前のオン・全窓の印・台本の組が残る。
///
/// # 非空虚性
/// バルーンの後始末が状態を捨てる・印を外すと、いずれかの欄が変わって赤。
#[test]
fn balloon_switch_keeps_stay_on_top_the_marks_and_the_script_group() {
    let (rig, windows) = rig_on_top(SkinKind::Balloon, balloon_finish());
    assert_eq!(
        finish(rig, &windows),
        ((true, 4, 1), None),
        "((常に手前, 印の付いた窓, 台本の組), 印の段)"
    );
}
