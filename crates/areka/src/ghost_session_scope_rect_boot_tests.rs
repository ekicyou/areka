//! 本物の起動（切替の試験台・`boot_ghost` → `rewire_runtime_consumers` → `wire`）を通す結合テスト
//! （spec mcp-scope-rect-property 要件 6.1・6.2・6.5・8.1）。
//!
//! 起動の前に偽の OS の読み取り（`ScopeRectReadSeam`）と、窓なしで組んだ窓の一式と偽の窓の印を
//! World に入れておき、MCP の `get_property` を受け口から汲んで答えを見る。実の窓・実の画面は使わない。

use std::sync::mpsc::{self, Sender};

use areka_mcp::ToolOutcome;
use areka_mcp::tools::get_property::Args;
use areka_mcp::tools::{ToolCall, ToolRequest, outcome};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::super::*;
use super::LEFT_SCREEN;
use super::wiring::{fake_handle, placement};
use crate::emo2_boot::ghost_switch::SwitchInFlight;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::spawn_ghost_windows;

/// A の `OnBoot`: 台詞の途中で B への切替を命じる。
const A_TO_B: &str = "\\0A\\![change,ghost,B,--option=raise-event]\\e";
const CHAR: &str = "currentghost.scope(0).rect";
const BALLOON: &str = "currentghost.balloon.scope(0).rect";

fn ghost_a(on_boot: &'static str) -> FakeShiori {
    FakeShiori::Scripted(Box::new(move || {
        standard_script(on_boot)
            .get(
                "OnGhostChanging",
                Ok(Some("\\0Bへ交代します\\e".to_owned())),
            )
            .get("OnGhostChanged", Ok(None))
    }))
}

fn ghost_b() -> FakeShiori {
    FakeShiori::Scripted(Box::new(|| {
        standard_script("\\0B\\e").get("OnGhostChanged", Ok(None))
    }))
}

/// A・B を据えた試験台と MCP の受け口（A は起動記録あり）。
fn rig(a: FakeShiori) -> (SwitchRig, Sender<ToolRequest>) {
    let mut rig = SwitchRig::new(vec![("A", a), ("B", ghost_b())]);
    // 切替先の窓の閉包の投函先（取り出しの系は無いので走らない）。
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
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

/// MCP の `get_property` を受け口へ送って汲み、その場の答え（ゴースト名と結果）を取る。
fn ask(rig: &mut SwitchRig, tx: &Sender<ToolRequest>, name: &str) -> Option<(String, ToolOutcome)> {
    let (req, pending) = ToolRequest::new(ToolCall::GetProperty(Args {
        property_name: name.to_owned(),
        ghost_name: None,
    }));
    tx.send(req).ok()?;
    crate::mcp::drain(&mut rig.world);
    pending
        .try_answer()
        .ok()
        .flatten()
        .map(|a| (a.ghost, a.outcome))
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

/// 起こした時点で偽の矩形を答え（3.1 の完了の状態）、B へ替えた後は前のゴーストの窓の印の
/// 矩形を答えない（要件 6.1・6.2）。
///
/// # 非空虚性
/// `rewire_runtime_consumers` の `wire` を外すと起動の直後の答えが「無い名前」で赤。
/// `wire` が前の表を使い回すと、替えた後も A の窓の矩形を答えて赤。
#[test]
fn switching_to_another_ghost_drops_the_previous_window_rects() {
    let (mut rig, tx) = rig(ghost_a(A_TO_B));
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

/// 新しい World と新しい実行系で起こし直すと、前の表を持ち越さない（要件 6.5）。
/// 2 度目の World にも同じ偽の OS を入れる（前の印が残っていれば矩形を答えてしまう形）。
///
/// # 非空虚性
/// `wire` が表をプロセスで 1 つ持つと、2 度目も前の窓の矩形を答えて赤。
#[test]
fn restarting_with_a_new_world_does_not_carry_the_table() {
    let (mut first, tx) = rig(ghost_a("\\0A\\e"));
    install_fake_os(&mut first);
    install_windows(&mut first);
    first.boot("A");
    let before = both(&mut first, &tx);
    let first_down = first.shutdown();
    drop(first);

    let (mut second, tx) = rig(ghost_a("\\0A\\e"));
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
