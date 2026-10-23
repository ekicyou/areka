//! 「バルーン位置初期化」の葉の檻（areka-P0-window-link-directives 要件 7.5〜7.7）。
//!
//! 初期化を呼んだかは [`reset_balloon_positions`] の記録で見る: 材料の無い World では
//! `balloon_pos_reset_skipped` がきっかけつきで必ず 1 行出るので、その行の有無が「呼んだか」になる。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

use bevy_ecs::world::World;

use super::{reset_balloon_enabled, reset_balloon_item};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::menu::{Frame, ItemBody, MenuContext, MenuItem};
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::reset_pos::DefaultPlacementBasis;
use crate::placement::source::{DescriptSource, GhostTitles};
use crate::placement::spawn::spawn_ghost_windows;
use crate::placement::test_support::capture_logs;

fn basis() -> DefaultPlacementBasis {
    DefaultPlacementBasis {
        source: DescriptSource {
            ghost_kv: BTreeMap::new(),
            shell_kv: BTreeMap::new(),
            shell_dir: PathBuf::from("shell/master"),
            titles: GhostTitles::from_scope_titles([(0, "s".to_string())]),
        },
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: BTreeMap::new(),
        },
    }
}

/// 窓の組（窓の無い空の組）を置く。
fn put_windows(world: &mut World) {
    spawn_ghost_windows(world, &[], &GhostTitles::from_scope_titles([]));
}

fn item(world: &World) -> MenuItem {
    reset_balloon_item(world, &MenuContext { scope: 0 })
}

#[test]
fn item_has_default_label_and_shiori_caption() {
    let world = World::new();
    let item = item(&world);
    assert_eq!(item.label, "バルーン位置初期化");
    assert_eq!(
        item.caption_resource,
        Some("resetballoonpositionbutton.caption")
    );
    assert_eq!(item.checked, None);
    assert!(matches!(item.body, ItemBody::Action(_)), "葉である");
}

#[test]
fn enabled_only_with_windows_and_basis() {
    let mut world = World::new();
    assert!(!reset_balloon_enabled(&world), "どちらも無い");
    world.insert_resource(basis());
    assert!(!reset_balloon_enabled(&world), "材料だけ");
    put_windows(&mut world);
    assert!(reset_balloon_enabled(&world), "窓の組と材料");
    assert!(item(&world).enabled);
    world.remove_resource::<DefaultPlacementBasis>();
    assert!(!reset_balloon_enabled(&world), "窓の組だけ");
    assert!(!item(&world).enabled);
}

#[test]
fn select_resets_with_menu_trigger_and_sends_nothing_to_ghost() {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        PathBuf::from("ghost/A"),
    ))));
    let ItemBody::Action(action) = item(&world).body else {
        panic!("葉である");
    };
    let (_, events) = capture_logs(|| action(&mut world, &MenuContext { scope: 0 }));
    let triggers: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("balloon_pos_reset_skipped"))
        .map(|e| e.field_str("trigger").map(str::to_owned))
        .collect();
    assert_eq!(triggers, vec![Some("menu".to_owned())], "{events:?}");
    assert!(rx.try_recv().is_err(), "ゴーストへ何も送らない");
}

#[test]
fn frame_is_reset_balloon_position() {
    assert_eq!(Frame::ORDER[4], Frame::ResetBalloonPosition);
}
