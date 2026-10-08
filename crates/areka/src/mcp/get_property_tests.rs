//! `get_property` の決定論テスト（2 本）。
//!
//! - 実行系が無い（空の World）: `NG:Property system is not available`（isError: true）で答え、
//!   `event = "mcp_get_property_unavailable"` の `warn!` をちょうど 1 件残す（要件 3.1・3.3）。
//! - 本物の実行系を 1 回だけ起こし、ゴースト自身の問い手・全体・別の問い手・空の値を載せてから
//!   7 つの名前を聞く（値・空の値・無い名前・問い手の選び方・名前を手直ししない・要件 1.1〜1.4・2.1〜2.3）。
//!
//! どちらも `handle` を直に呼び、答えは返った直後に `try_answer` で取り出す（その場で答える・
//! 要件 4.2）。`ghost_name` はすべて省略（要件 4.8）、英字の大小を混ぜた名前は使わない（要件 4.7）。

use std::path::PathBuf;

use areka_ghost::sylphya_wiring::ghost_asker_id;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::{ToolCall, ToolRequest};
use areka_sylphya::AskerId;
use log_capture_kit::capture;

use super::*;
use crate::ghost_session::GhostSlot;

/// `handle` を 1 回呼び、返った直後の答えを取り出す（まだ答えていなければ None）。
fn ask(world: &mut World, ghost: &ActiveGhost, property_name: &str) -> Option<ToolOutcome> {
    let args = Args {
        property_name: property_name.to_string(),
        ghost_name: None,
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetProperty(args.clone()));
    handle(world, ghost, args, req.reply);
    pending.try_answer().ok().flatten().map(|a| a.outcome)
}

#[test]
fn answers_unavailable_and_warns_once_without_a_runtime() {
    let ghost = ActiveGhost {
        name: Some("emily4".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    };
    let mut world = World::new();

    let (answer, events) = capture(|| ask(&mut world, &ghost, "baseware.name"));

    let warned: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("mcp_get_property_unavailable"))
        .map(|e| {
            (
                e.level,
                e.field_str("ghost").map(str::to_owned),
                e.field_str("property_name").map(str::to_owned),
            )
        })
        .collect();
    assert_eq!(
        (answer, warned),
        (
            Some(outcome::ng("Property system is not available")),
            vec![(
                tracing::Level::WARN,
                Some("emily4".to_owned()),
                Some("baseware.name".to_owned())
            )]
        ),
        "（その場の答え・記録の段と欄 ghost・property_name）"
    );
}

#[test]
fn reads_values_through_the_ghost_own_asker_on_a_real_runtime() {
    use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};

    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script(r"\0A\e"))),
    )]);
    rig.boot("A");
    let (publisher, own) = {
        let runtime = rig
            .world
            .get_non_send::<GhostSlot>()
            .and_then(|slot| slot.0.as_ref())
            .and_then(|session| session.runtime())
            .expect("置き場に実行系がある");
        (
            runtime.sylphya_publisher().clone(),
            ghost_asker_id(&runtime.mount().shiori.dir),
        )
    };
    let someone_else = AskerId::new("someone-else");
    publisher.set(own.clone(), "test.key".into(), "ghost".into());
    publisher.publish_static(
        own.clone(),
        vec![],
        vec![("test.key".into(), "global".into())],
    );
    publisher.set(someone_else.clone(), "test.key".into(), "other".into());
    publisher.set(someone_else, "test.other_only".into(), "other".into());
    publisher.set(own, "test.empty".into(), String::new());
    publisher.barrier().expect("載せた値が反映される");

    let ghost = crate::mcp::resolve::active(&rig.world).expect("起動中のゴーストがある");
    let names = [
        "baseware.name",
        "test.key",
        "test.other_only",
        "test.empty",
        "currentghost.name",
        "",
        " baseware.name",
    ];
    let answers: Vec<_> = names
        .iter()
        .map(|name| ask(&mut rig.world, &ghost, name))
        .collect();
    let down = rig.shutdown();

    let not_found = || Some(outcome::ng("Cannot find such property name."));
    assert_eq!(
        (answers, down),
        (
            vec![
                Some(outcome::value("areka")),
                Some(outcome::value("ghost")),
                not_found(),
                Some(outcome::value("")),
                not_found(),
                not_found(),
                not_found(),
            ],
            true
        ),
        "（{names:?} への答え・降ろせた）"
    );
}
