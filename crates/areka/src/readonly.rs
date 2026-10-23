//! 走りの状態（readonly-mode・design「ReadonlyState」）。
//!
//! 1 回の走りが読み取りだけかを起動の前に 1 度だけ決め、プロセスの旗・World の印
//! [`ReadOnlyRun`]・保存の口の控える範囲を同じ値から立てる唯一の口。
//! 旗を下ろす口と印を外す口は作らない（要件 1.6）。入口のモジュールは `use` しない
//! （入口の側がこのモジュールを見る）。

use std::sync::atomic::{AtomicBool, Ordering};

use bevy_ecs::prelude::*;

/// プロセスの旗（立てるのは [`engage`] だけ・下ろす口は無い）。
static ON: AtomicBool = AtomicBool::new(false);

/// World の中の読み取りだけの印（在れば読み取りだけ）。本番は [`install`] だけが足し、外さない。
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReadOnlyRun;

/// 起動の前に 1 度だけ呼ぶ（呼び手は `main` だけ）。`on` が真なら旗・保存の口の範囲・記録を立てる。
// 一時: 呼び手は 5.1 で `main` に足す。
#[allow(dead_code)]
pub(crate) fn engage(on: bool) {
    if !on {
        return;
    }
    ON.store(true, Ordering::SeqCst);
    areka_sylphya::persist::withhold::withhold_everything();
    tracing::info!(
        event = "readonly_engaged",
        "[readonly] --option readonly により、読み取りだけで起動する（記憶・利用者ごとの置き場・ゴーストのフォルダへ書かない）"
    );
}

/// プロセスの旗（起動の前に決まる守りが引数として読む）。
pub(crate) fn is_on() -> bool {
    ON.load(Ordering::SeqCst)
}

/// 旗が立っていれば World に [`ReadOnlyRun`] を足す（呼び手は `main` だけ）。
// 一時: 呼び手は 5.1 で `main` に足す。
#[allow(dead_code)]
pub(crate) fn install(world: &mut World) {
    if is_on() {
        world.insert_resource(ReadOnlyRun);
    }
}

/// World の中の守りが読む（試験は自分の World に [`ReadOnlyRun`] を足す）。
// 一時: 最初の読み手は 4.1（`install`）で足す。
#[allow(dead_code)]
pub(crate) fn of(world: &World) -> bool {
    world.contains_resource::<ReadOnlyRun>()
}

#[cfg(test)]
#[path = "readonly_tests.rs"]
mod tests;
