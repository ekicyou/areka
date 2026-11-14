mod clip;
mod command_list;
mod components;
mod composite;
mod core;
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "描画の係（タスク 3.x）が計画を使い始めたら外す")
)]
mod damage;
mod readback;
mod systems;
pub mod visual;
pub mod visual_manager;
mod wuc_resource;

pub use clip::*;
pub use command_list::*;
pub use components::*;
pub use core::*;
pub use readback::*;
pub use systems::*;
pub use visual::*;
pub use visual_manager::*;
pub use wuc_resource::WucGraphicsResource;

#[cfg(test)]
mod composite_damage_tests;
#[cfg(test)]
mod composite_scroll_tests;
#[cfg(test)]
mod composite_tests;
#[cfg(test)]
mod damage_tests;
#[cfg(test)]
mod readback_tests;
#[cfg(test)]
mod tests;
