//! `update/` のテストが分け合う小道具（`#[test]` を持たない）。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use areka_ghost::BasewareRoot;
use areka_kanade::{KanadeMsg, ShioriMethod};
use areka_runtime::spine::RecordedCall;
use areka_session::exit_wait::{ClosingWaits, WorkGate, begin_close, register_gate};
use areka_session::ghost_session::{GhostSession, GhostSlot};
use areka_session::reload::test_support::boot_context_named;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;

/// 窓口を据え、根 `root` の起動の文脈（バルーン `balloon`）と置き場のゴースト `ghost`（kanade の送出端
/// だけ・実行系なし＝シェルは解けない）を置いた World と、kanade の代わりの受信端。
pub(crate) fn slot_world(root: &Path, ghost: &str, balloon: &str) -> (World, Receiver<KanadeMsg>) {
    let base = BasewareRoot::new(root.to_path_buf());
    let mut world = World::new();
    world.init_resource::<Schedules>();
    crate::update::register(&mut world);
    world.insert_resource(boot_context_named(root, &base, ghost, balloon));
    let (tx, kanade) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        base.ghost_dir(ghost),
    ))));
    (world, kanade)
}

/// kanade に届いた送出（id・Reference・返信端の有無）。どれも汎用の通知の入口の GET。
pub(crate) fn raised(kanade: &Receiver<KanadeMsg>) -> Vec<(String, Vec<String>, bool)> {
    kanade
        .try_iter()
        .map(|msg| match msg {
            KanadeMsg::RaiseEvent {
                id,
                references,
                method,
                reply,
            } => {
                assert_eq!(method, ShioriMethod::Get);
                (id, references, reply.is_some())
            }
            _ => panic!("汎用の通知の入口の送出だけが届く"),
        })
        .collect()
}

/// ゴーストの `.update-work/` の下に、成功した走行の残り（印 `committed` と古い DLL の写し）を置く。
pub(crate) fn plant_committed(ghost_dir: &Path, run: &str) -> PathBuf {
    let dir = ghost_dir.join(".update-work").join(run);
    std::fs::create_dir_all(dir.join("old")).expect("走行フォルダを組む");
    std::fs::write(dir.join("old").join("yaya.dll"), b"old").expect("古い写しを置く");
    std::fs::write(dir.join("committed"), b"").expect("印を置く");
    dir
}

/// 門 `update` を 1 つ登記した World で終了を始める（門が閉じる）。書いている最中だった仕事を返す。
pub(crate) fn close(gate: &Arc<WorkGate>) -> ClosingWaits {
    let mut world = World::new();
    register_gate(&mut world, "update", gate.clone(), |_| {});
    begin_close(&mut world)
}

/// `id` の GET の Reference（最初の 1 件）。
pub(crate) fn refs_of(calls: &[RecordedCall], id: &str) -> Option<Vec<String>> {
    areka_session::ghost_session::shell_balloon_switch_session_lap_test_support::refs_of(calls, id)
        .into_iter()
        .next()
}
