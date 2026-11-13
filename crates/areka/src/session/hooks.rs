//! 本体の口の束の中身と据え付け（runtime-module-untangle design「SessionHooksImpl」）。
//!
//! 各関数は置き換える前の `runtime` の 1 か所のコードを移したもの。記録は持たない（記録は
//! `runtime` の呼び手に残る）。

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use areka_kanade::KanadeMsg;
use areka_seriko::SerikoSink;
use bevy_ecs::world::World;

use crate::boot_config::BootContext;
use crate::boot_resolve::{BalloonDecision, BalloonRoute, record_last_balloon, record_last_shell};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::runtime::ghost_switch::SwitchInFlight;
use crate::runtime::reload;
use crate::runtime::session_hooks::SessionHooks;
use crate::runtime::shell_balloon_resolve::SkinCandidate;
use crate::runtime::skin_switch::SkinKind;

/// 口の中身。
pub(crate) const HOOKS: SessionHooks = SessionHooks {
    kanade: kanade_of,
    seriko_sink: seriko_sink_of,
    ghost_ref_name,
    set_shell_dir,
    set_current_balloon,
    record_last_skin,
    ghost_switch_in_flight,
    reload_skin_applied,
    skin_phase_ended: reload::skin_phase_ended,
};

/// 口を World へ据える（`ghost_session::register_systems` の先頭から・何度呼んでも同じ）。
pub(crate) fn install(world: &mut World) {
    world.insert_resource(HOOKS);
}

/// 置き場のゴースト（無ければ `None`）。
fn session(world: &World) -> Option<&GhostSession> {
    world.get_non_send::<GhostSlot>()?.0.as_ref()
}

fn kanade_of(world: &World) -> Option<&Sender<KanadeMsg>> {
    session(world)?.kanade()
}

fn seriko_sink_of(world: &World) -> Option<&SerikoSink> {
    session(world)?.seriko_sink()
}

/// Reference のゴーストの名前（`descript.txt` の `name`、無ければフォルダ名・ゴーストが無ければ空）。
fn ghost_ref_name(world: &World) -> String {
    session(world).map_or_else(String::new, |s| {
        s.ghost_name().unwrap_or_else(|| {
            s.ghost_dir()
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
    })
}

fn set_shell_dir(world: &mut World, dir: PathBuf) -> bool {
    world
        .get_non_send_mut::<GhostSlot>()
        .is_some_and(|mut slot| slot.0.as_mut().is_some_and(|s| s.set_shell_dir(dir)))
}

/// 記憶に書くので、これが次の起動の選び方になる（`BalloonRoute` に腕を足さない）。
fn set_current_balloon(world: &mut World, target: &SkinCandidate) -> bool {
    let Some(mut ctx) = world.get_resource_mut::<BootContext>() else {
        return false;
    };
    ctx.current.balloon = BalloonDecision {
        route: BalloonRoute::Memory,
        dir: target.dir.clone(),
        folder: Some(target.folder.clone()),
    };
    true
}

fn record_last_skin(world: &World, kind: SkinKind, folder: &str) -> bool {
    let Some(publisher) = session(world).and_then(GhostSession::memory_publisher) else {
        return false;
    };
    match kind {
        SkinKind::Shell => record_last_shell(publisher, folder),
        SkinKind::Balloon => record_last_balloon(publisher, folder),
    }
    true
}

fn ghost_switch_in_flight(world: &World) -> bool {
    world.get_non_send::<SwitchInFlight>().is_some()
}

fn reload_skin_applied(world: &mut World, kind: SkinKind, dir: &Path) -> bool {
    if !reload::skin_is_reload(world, kind, dir) {
        return false;
    }
    reload::skin_applied(world);
    true
}
