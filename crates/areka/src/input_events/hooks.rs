use bevy_ecs::prelude::*;

/// 本体が据える強制退避の口（本体の `app_exit::install_input_hooks` が `register_systems` で据える）。
#[derive(Resource, Clone, Copy)]
pub(crate) struct InputHooks {
    /// 全窓を閉じて終了を指示する（本体の終了の統合操作・出所 `Escape`）。
    pub(crate) escape: fn(&mut World),
}

/// 据えられた口で強制退避する。口が無ければ `error!(input_hooks_missing)` を残し、
/// `wintf::AppExit` が在れば終わりを指示する。
pub(crate) fn escape(world: &mut World) {
    if let Some(hooks) = world.get_resource::<InputHooks>().copied() {
        (hooks.escape)(world);
        return;
    }
    tracing::error!(
        event = "input_hooks_missing",
        "強制退避の口が据わっていない——終了の受け口があれば終了だけを指示する"
    );
    if let Some(exit) = world.get_non_send::<wintf::AppExit>() {
        exit.request_exit();
    }
}

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod tests;
