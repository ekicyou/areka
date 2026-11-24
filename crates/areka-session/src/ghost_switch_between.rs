//! 切替の「降ろした」と「起こす」の間の待ち。

use areka_ghost::same_folder_name;
use areka_kanade::BootOrigin;
use areka_runtime::shell_carry::ShellCarry;
use bevy_ecs::world::World;

use super::{WelcomeAttempt, boot_into, fatal, set_stage, switch_to_default};
use crate::app_exit::WindowsClosed;
use crate::boot_resolve::{DEFAULT_GHOST_FOLDER, GhostDecision};

/// 降ろして全窓を閉じた後に起こすのを待たせる印。材料は上書きの口から戻った `switch_to` が預ける。
pub(super) struct BetweenHold(pub(super) Option<BootPlan>);

/// 起こす後半の材料。
pub(super) struct BootPlan {
    pub(super) closed: WindowsClosed,
    pub(super) ghost: GhostDecision,
    pub(super) origin: BootOrigin,
    pub(super) carry: Option<ShellCarry>,
    pub(super) name: String,
    pub(super) folder: String,
}

/// 本体の上書きの口の中から呼ぶ: 起こすのを答えが来るまで待たせる。
pub fn hold_between(world: &mut World) {
    world.insert_non_send(BetweenHold(None));
}

/// 待たせた切替を続けて起こす。待ちが無ければ warn を 1 件残して何もしない。
pub fn resume_between(world: &mut World) {
    let Some(plan) = world
        .remove_non_send::<BetweenHold>()
        .and_then(|hold| hold.0)
    else {
        tracing::warn!(
            event = "ghost_switch_no_hold",
            "待たせた切替が無いのに続きを起こす指示が来た——何もしない"
        );
        return;
    };
    boot_target(world, plan);
}

pub(super) fn clear_hold(world: &mut World) {
    world.remove_non_send::<BetweenHold>();
}

/// 印が立っていれば材料を預けて戻り、無ければその場で起こす。
pub(super) fn boot_or_hold(world: &mut World, plan: BootPlan) {
    match world.get_non_send_mut::<BetweenHold>() {
        Some(mut hold) => hold.0 = Some(plan),
        None => boot_target(world, plan),
    }
}

/// 起こす後半: 起こせたらシェルの状態を渡して段を「迎え入れ（切替先）」に。起こせなければ、
/// 切替先が既定なら致命、そうでなければ既定へ戻す。
fn boot_target(world: &mut World, plan: BootPlan) {
    if boot_into(world, plan.closed, plan.ghost, plan.origin) {
        areka_runtime::shell_carry::hand_over(world, plan.carry);
        set_stage(world, WelcomeAttempt::Target);
    } else if same_folder_name(&plan.folder, DEFAULT_GHOST_FOLDER) {
        fatal(
            world,
            format!("切替先の既定ゴースト {} を起こせなかった", plan.name),
        );
    } else {
        switch_to_default(world, &plan.name);
    }
}
