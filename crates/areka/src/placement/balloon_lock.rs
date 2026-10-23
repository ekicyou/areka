//! スコープごとの「バルーンの動かせない指定」の帳簿と、バルーンの窓の
//! `DragConfig.move_window` の書き手（descript-window-keys 要件 4・5・design「BalloonLock」）。
//!
//! 帳簿は descript の段（ゴースト・シェルの `balloon.dontmove`）と台本の lock
//! （`\![lock,balloonmove]`）を分けて持つ。unlock は台本の分だけを解くので、descript の段が
//! 禁止している間は unlock で動かせるようにならない（要件 5.3）。
//!
//! 窓を作った後にバルーンの窓の `move_window` を書くのは [`sync_balloon_drag_writer`] だけ。
//! `enabled` と閾値は触らない（クリック・選択肢・リンク・スクロールは今どおり・要件 6.2）。

use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use tracing::{info, warn};
use wintf::ecs::WindowHandle;
use wintf::ecs::drag::{DragConfig, WindowDragContextResource};

use super::spawn::BalloonWindowMarker;

/// 止めた理由（記録に出す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HoldReason {
    pub descript: bool,
    pub script: bool,
}

/// スコープごとの動かせない指定の帳簿。
#[derive(Resource, Debug, Default)]
pub(crate) struct BalloonLockLedger {
    descript: BTreeSet<u32>,
    script: BTreeSet<u32>,
}

impl BalloonLockLedger {
    /// 動かせないなら理由、動かせるなら `None`。
    pub(crate) fn reason(&self, scope: u32) -> Option<HoldReason> {
        let reason = HoldReason {
            descript: self.descript.contains(&scope),
            script: self.script.contains(&scope),
        };
        (reason.descript || reason.script).then_some(reason)
    }
}

/// 起動（窓を作る閉包・`seed_sticky_descript` の直後）: 帳簿を新しく据え、書き手を呼ぶ。
/// 窓に受け手は付けない（バルーンの窓の受け手は今の `OnDrag`・`OnDragEnd` のまま）。
pub(crate) fn seed_balloon_lock(world: &mut World, descript: &BTreeSet<u32>) {
    world.insert_resource(BalloonLockLedger {
        descript: descript.clone(),
        script: BTreeSet::new(),
    });
    sync_balloon_drag_writer(world);
}

/// シェルの切り替えの後始末: descript の段だけを取り替え、台本の lock は保ち、書き手を呼ぶ（要件 4.4・5.4）。
pub(crate) fn replace_balloon_lock_descript(world: &mut World, descript: &BTreeSet<u32>) {
    world
        .get_resource_or_insert_with(BalloonLockLedger::default)
        .descript = descript.clone();
    sync_balloon_drag_writer(world);
}

/// 台本の lock（`locked: true`）・unlock（`false`）を `scope` にだけ当て、記録し、書き手を呼ぶ
/// （要件 5.1〜5.3・5.8）。帳簿が無ければ空の帳簿を据えてから当てる。
pub(crate) fn apply_script_balloon_lock(world: &mut World, scope: u32, locked: bool) {
    if !world.contains_resource::<BalloonLockLedger>() {
        warn!(event = "balloon_lock_ledger_missing", scope);
        world.init_resource::<BalloonLockLedger>();
    }
    let mut ledger = world.resource_mut::<BalloonLockLedger>();
    if locked {
        ledger.script.insert(scope);
        info!(event = "balloon_move_locked", scope);
    } else {
        let was_locked = ledger.script.remove(&scope);
        info!(event = "balloon_move_unlocked", scope, was_locked);
        if ledger.descript.contains(&scope) {
            info!(event = "balloon_move_unlock_held", scope);
        }
    }
    sync_balloon_drag_writer(world);
}

/// すべてのバルーンの窓の `DragConfig.move_window` を帳簿から付け直す（書き手は 1 か所）。
/// 帳簿が無ければ全部動かせる（要件 4.3）。
pub(crate) fn sync_balloon_drag_writer(world: &mut World) {
    let held: BTreeSet<u32> = world
        .get_resource::<BalloonLockLedger>()
        .map(|l| l.descript.union(&l.script).copied().collect())
        .unwrap_or_default();
    let mut query = world.query::<(&BalloonWindowMarker, &mut DragConfig)>();
    for (marker, mut drag) in query.iter_mut(world) {
        let locked = u32::try_from(marker.scope).is_ok_and(|s| held.contains(&s));
        drag.move_window = !locked;
    }
}

/// `on_balloon_drag` の入口の見張り: このドラッグを止めているか（要件 5.6・6.1）。
/// wintf がドラッグの始まりに控えた `WindowDragContextResource` の `hwnd` がこの窓のものなら
/// 控えの `move_window` で決める。控えが消えた後（`hwnd: None`・`move_window: false`）や
/// 別の窓のものなら、控えを読まずにこの窓の今の `DragConfig.move_window` で決める
/// （先例 `sticky_follow::drag_wintf_moves` と同じ縮退）。
pub(crate) fn balloon_drag_held(world: &World, entity: Entity) -> bool {
    let start = world
        .get_resource::<WindowDragContextResource>()
        .and_then(|r| r.get());
    let hwnd = world.get::<WindowHandle>(entity).map(|h| h.hwnd);
    match start {
        Some(ctx) if ctx.hwnd.is_some() && ctx.hwnd == hwnd => !ctx.move_window,
        _ => world
            .get::<DragConfig>(entity)
            .is_some_and(|d| !d.move_window),
    }
}

/// `on_balloon_drag_end` の入口の見張り: [`balloon_drag_held`] と同じ見方で止めたドラッグなら、
/// 終わりのときの帳簿の理由（記録のためだけ）で記録を 1 件残して `true`（要件 6.6）。
pub(crate) fn end_balloon_drag(world: &World, entity: Entity) -> bool {
    if !balloon_drag_held(world, entity) {
        return false;
    }
    let scope = world.get::<BalloonWindowMarker>(entity).map(|m| m.scope);
    let reason = scope
        .and_then(|s| u32::try_from(s).ok())
        .and_then(|s| world.get_resource::<BalloonLockLedger>()?.reason(s));
    info!(
        event = "balloon_drag_held",
        scope = scope.map(|s| s as u64),
        descript = reason.is_some_and(|r| r.descript),
        script = reason.is_some_and(|r| r.script),
    );
    true
}

#[cfg(test)]
#[path = "balloon_lock_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "balloon_lock_drag_tests.rs"]
mod drag_tests;
