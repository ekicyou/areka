//! ツールチップ（マウスを置いた所に出る短い説明）。

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "OS の境界（os.rs）が使うが、os.rs は公開の口（system.rs）から使い始める"
    )
)]
mod geometry;
#[expect(
    dead_code,
    reason = "公開の口（system.rs）から使い始める。テストも純粋な部分しか使わない"
)]
mod os;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "公開の口（system.rs）から使い始める")
)]
mod ranges;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "画面更新ごとの判定（system.rs）から使い始める")
)]
mod turn;

pub use os::TooltipOsError;
pub use ranges::{TooltipArea, TooltipRange, TooltipRangeId};
pub use turn::{TooltipEndReason, TooltipTurnToken};

use crate::ecs::{PointF, Window};
use bevy_ecs::prelude::*;
use std::time::Instant;

/// 出す番が来た知らせの中身。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipTurn {
    pub window: Entity,
    pub range: TooltipRangeId,
    /// 知らせを出した時のマウスの位置（窓の中・論理の単位）。
    pub position: PointF,
    pub token: TooltipTurnToken,
    /// その範囲に文字が預けてあるか（真なら既に出ている）。
    pub has_text: bool,
}

/// 知らせの種類（来た・終わった）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipNotice {
    TurnStarted(TooltipTurn),
    TurnEnded {
        window: Entity,
        range: TooltipRangeId,
        token: TooltipTurnToken,
        reason: TooltipEndReason,
    },
}

/// 窓に付ける、知らせを受ける関数。World を借りたまま同期で呼ばれる。
#[derive(Component, Clone, Copy)]
pub struct OnTooltip(pub fn(world: &mut World, notice: &TooltipNotice));

/// 文字を渡した・消した結果。
#[non_exhaustive]
#[derive(Debug)]
pub enum TooltipSupply {
    /// 出した（または出ていた表示を置き換えた）。
    Shown,
    /// 文字が空だったので出さなかった（出ていれば消した）。
    Cleared,
    /// 印の出す番は既に終わっていた。何も出していない。
    StaleTurn,
    /// OS の側の失敗で出せなかった。出す番は続いている。
    Failed(TooltipOsError),
}

/// 範囲の登録の誤り。
#[derive(Debug, thiserror::Error)]
pub enum TooltipRegisterError {
    #[error("窓のエンティティが無い")]
    NoSuchWindow,
}

/// 範囲を登録する（後から登録したものが、重なった所で勝つ）。
///
/// `window` は [`Window`] を持つエンティティ（まだ窓のハンドルが無くてもよい）。
/// 無ければ [`TooltipRegisterError::NoSuchWindow`]。
pub fn register(
    world: &mut World,
    window: Entity,
    range: TooltipRange,
) -> Result<TooltipRangeId, TooltipRegisterError> {
    let mut e = world
        .get_entity_mut(window)
        .ok()
        .filter(|e| e.contains::<Window>())
        .ok_or(TooltipRegisterError::NoSuchWindow)?;
    let id = match e.get_mut::<ranges::TooltipRanges>() {
        Some(mut table) => table.add(window, range),
        None => {
            let mut table = ranges::TooltipRanges::default();
            let id = table.add(window, range);
            e.insert(table);
            id
        }
    };
    wake_next_frame();
    Ok(id)
}

/// 登録の中身を差し替える（重なりの順は変えない・次の出す番から効く）。無ければ偽。
pub fn update(world: &mut World, id: TooltipRangeId, range: TooltipRange) -> bool {
    let replaced = world
        .get_mut::<ranges::TooltipRanges>(id.window())
        .is_some_and(|mut table| table.replace(id, range));
    if replaced {
        wake_next_frame();
    }
    replaced
}

/// マウスが動かないままでも、次の画面更新で判定が回るよう期限を預ける。
fn wake_next_frame() {
    crate::ecs::world::tick_wake::arm_deadline(Instant::now());
}

#[cfg(test)]
#[path = "system_tests.rs"]
mod system_tests;
