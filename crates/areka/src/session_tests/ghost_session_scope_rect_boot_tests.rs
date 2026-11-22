//! 本物の起動（切替の試験台・`boot_ghost` → `rewire_runtime_consumers` → `wire`）を通す結合テスト。
//!
//! 起動の前に偽の OS の読み取り（`ScopeRectReadSeam`）と、窓なしで組んだ窓の一式と偽の窓の印を
//! World に入れておき、MCP の `get_property` を受け口から汲んで答えを見る。実の窓・実の画面は使わない。

use std::sync::mpsc::{self, Sender};

use areka_mcp::ToolOutcome;
use areka_mcp::tools::{ToolRequest, outcome};

use super::ghost_session_switch_tests::{A_TO_B, ask, ghost_a, ghost_b, lap_rig};
use areka_placement::source::GhostTitles;
use areka_placement::spawn::spawn_ghost_windows;
use areka_placement::test_support::fake_handle;
use areka_session::ghost_session::ghost_session_scope_rect::test_support::LEFT_SCREEN;
use areka_session::ghost_session::ghost_session_scope_rect::test_support::placement;
use areka_session::ghost_session::ghost_session_scope_rect::*;
use areka_session::ghost_switch::SwitchInFlight;
use areka_session::ghost_switch_test_support::{FakeShiori, SwitchRig};
use std::sync::Arc;

const CHAR: &str = "currentghost.scope(0).rect";
const BALLOON: &str = "currentghost.balloon.scope(0).rect";

/// A・B を据えた試験台と MCP の受け口（A は起動記録あり）。
fn rig(a: FakeShiori) -> (SwitchRig, Sender<ToolRequest>) {
    let mut rig = lap_rig(a, ghost_b());
    let (tx, rx) = mpsc::channel();
    crate::mcp::install(&mut rig.world, rx);
    (rig, tx)
}

/// 偽の OS（印 0x10・0x20 だけが `LEFT_SCREEN` で読める）を差し替え口に入れる。
fn install_fake_os(rig: &mut SwitchRig) {
    rig.world
        .insert_resource(ScopeRectReadSeam(Arc::new(|k: WindowKey| {
            matches!(k.0, 0x10 | 0x20).then_some(LEFT_SCREEN)
        })));
}

/// スコープ 0 の窓の一式を窓なしで組み、キャラクター 0x10・バルーン 0x20 の印を付ける。
fn install_windows(rig: &mut SwitchRig) {
    let gw = spawn_ghost_windows(
        &mut rig.world,
        &[placement()],
        &GhostTitles::from_scope_titles([(0, "s".to_string())]),
    );
    let world = &mut rig.world;
    world
        .entity_mut(gw.char_window(0).unwrap())
        .insert(fake_handle(0x10));
    world
        .entity_mut(gw.balloon_window(0).unwrap())
        .insert(fake_handle(0x20));
}

fn both(rig: &mut SwitchRig, tx: &Sender<ToolRequest>) -> [Option<(String, ToolOutcome)>; 2] {
    [ask(rig, tx, CHAR), ask(rig, tx, BALLOON)]
}

fn answer(ghost: &str, o: ToolOutcome) -> Option<(String, ToolOutcome)> {
    Some((ghost.to_owned(), o))
}

fn not_found() -> ToolOutcome {
    outcome::ng("Cannot find such property name.")
}

/// 起こした時点で偽の矩形を答え、B へ替えた後は前のゴーストの窓の印の
/// 矩形を答えない。
///
/// # 非空虚性
/// `rewire_runtime_consumers` の `wire` を外すと起動の直後の答えが「無い名前」で赤。
/// `wire` が前の表を使い回すと、替えた後も A の窓の矩形を答えて赤。
#[test]
fn switching_to_another_ghost_drops_the_previous_window_rects() {
    let (mut rig, tx) = rig(ghost_a(A_TO_B, None));
    install_fake_os(&mut rig);
    install_windows(&mut rig);
    rig.boot("A");
    let booted = both(&mut rig, &tx);
    let steady = rig.wait_steady();
    let welcomed = rig.pump_talking_until(|rig| {
        rig.exit_requested()
            || (!rig.calls("B").is_empty() && rig.world.get_non_send::<SwitchInFlight>().is_none())
    });
    let switched = both(&mut rig, &tx);
    let down = rig.shutdown();

    let rect = outcome::value("-782,201,49,1227");
    assert_eq!(
        (booted, steady, welcomed, switched, down),
        (
            [answer("A", rect.clone()), answer("A", rect)],
            true,
            true,
            [
                answer("B", outcome::value("0,0,0,0")),
                answer("B", not_found())
            ],
            true,
        ),
        "（A を起こした直後の答え・A の定常・B の定常・B へ替えた後の答え・降ろせた）"
    );
}

/// 新しい World と新しい実行系で起こし直すと、前の表を持ち越さない。
/// 2 度目の World にも同じ偽の OS を入れる（前の印が残っていれば矩形を答えてしまう形）。
///
/// # 非空虚性
/// `wire` が表をプロセスで 1 つ持つと、2 度目も前の窓の矩形を答えて赤。
#[test]
fn restarting_with_a_new_world_does_not_carry_the_table() {
    let (mut first, tx) = rig(ghost_a("\\0A\\e", None));
    install_fake_os(&mut first);
    install_windows(&mut first);
    first.boot("A");
    let before = both(&mut first, &tx);
    let first_down = first.shutdown();
    drop(first);

    let (mut second, tx) = rig(ghost_a("\\0A\\e", None));
    install_fake_os(&mut second);
    second.boot("A");
    let after = both(&mut second, &tx);
    let second_down = second.shutdown();

    let rect = outcome::value("-782,201,49,1227");
    assert_eq!(
        (before, first_down, after, second_down),
        (
            [answer("A", rect.clone()), answer("A", rect)],
            true,
            [
                answer("A", outcome::value("0,0,0,0")),
                answer("A", not_found())
            ],
            true,
        ),
        "（1 度目の答え・降ろせた・起こし直した後の答え・降ろせた）"
    );
}
