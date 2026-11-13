//! 本体の口の束（runtime-module-untangle design「SessionHooks」）。
//!
//! `runtime` は本体の状態（置き場のゴースト・起動の文脈・切替と読み直しの予約）を名指さず、
//! 本体が据えた関数のポインタの束を通して同期で読み書きする（前例 `wintf::ecs::cursor::CursorQuery`）。
//! 中身と据え付けは `session::hooks` が持つ。

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use areka_kanade::KanadeMsg;
use areka_seriko::SerikoSink;
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use crate::runtime::shell_balloon_resolve::SkinCandidate;
use crate::runtime::skin_switch::SkinKind;

/// 本体の状態を同期で読み書きする口の束（`session::hooks::install` が据える）。
#[derive(Resource, Clone, Copy)]
pub(crate) struct SessionHooks {
    /// 置き場のゴーストの kanade への送り手（`GhostSession::kanade`）。
    pub kanade: fn(&World) -> Option<&Sender<KanadeMsg>>,
    /// 置き場のゴーストの seriko の送り手（`GhostSession::seriko_sink`）。
    pub seriko_sink: fn(&World) -> Option<&SerikoSink>,
    /// Reference のゴーストの名前（`descript.txt` の `name`、無ければフォルダ名、ゴーストが無ければ空）。
    pub ghost_ref_name: fn(&World) -> String,
    /// 起動の文脈の今のゴーストのフォルダ名（`BootContext.current.ghost.folder`）。
    pub ghost_folder: fn(&World) -> Option<String>,
    /// 実行系の今のシェルを書き換える（`GhostSession::set_shell_dir`・ゴーストが無ければ偽）。
    pub set_shell_dir: fn(&mut World, PathBuf) -> bool,
    /// 起動の文脈の今のバルーンを記憶の決め方で書き換える（文脈が無ければ偽）。
    pub set_current_balloon: fn(&mut World, &SkinCandidate) -> bool,
    /// 記憶の書き手で `LastShell`／`LastBalloon` を書く（書き手が無ければ偽）。
    pub record_last_skin: fn(&World, SkinKind, &str) -> bool,
    /// ゴースト切替の予約（`SwitchInFlight`）が在るか。
    pub ghost_switch_in_flight: fn(&World) -> bool,
    /// 読み直しの差し替えなら済んだ印を付けて真（`reload::skin_is_reload` → `reload::skin_applied`）。
    pub reload_skin_applied: fn(&mut World, SkinKind, &Path) -> bool,
    /// 差し替えの相が終わったことを読み直しの窓口へ告げる（`reload::skin_phase_ended`）。
    pub skin_phase_ended: fn(&mut World, SkinKind, &Path),
}

impl SessionHooks {
    /// 据えられた口。本番で無ければ `error!(session_hooks_missing)` を出して `NO_BODY` を返す
    /// （`&World` しか持たないので回数は絞らない・絞る `OnceLock` は共有の状態を増やす）。
    pub(crate) fn of(world: &World) -> Self {
        if let Some(hooks) = world.get_resource::<SessionHooks>() {
            return *hooks;
        }
        #[cfg(not(test))]
        tracing::error!(
            event = "session_hooks_missing",
            "本体の口が据わっていない（結線の誤り）——本体の状態が無いものとして続ける"
        );
        FALLBACK
    }
}

/// 本体の状態が無いときの答え: `None`・偽・空の文字・何もしない（今の「状態が無い」ときの答えと同じ）。
pub(crate) const NO_BODY: SessionHooks = SessionHooks {
    kanade: |_| None,
    seriko_sink: |_| None,
    ghost_ref_name: |_| String::new(),
    ghost_folder: |_| None,
    set_shell_dir: |_, _| false,
    set_current_balloon: |_, _| false,
    record_last_skin: |_, _, _| false,
    ghost_switch_in_flight: |_| false,
    reload_skin_applied: |_, _, _| false,
    skin_phase_ended: |_, _, _| {},
};

/// 口の無い World の倒れ先。本番は `NO_BODY`（記録つき）。
#[cfg(not(test))]
const FALLBACK: SessionHooks = NO_BODY;
/// テストの World は本体の口そのもの（今日と同じく本体の状態をそのまま読む・記録は出さない）。
#[cfg(test)]
const FALLBACK: SessionHooks = crate::session::hooks::HOOKS;
