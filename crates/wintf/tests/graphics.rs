// tests/graphics.rs — wintf graphics domain test entry point
#[path = "graphics/common/mod.rs"]
mod common;
#[path = "graphics/brushes_system_test.rs"]
mod brushes_system_test;
#[path = "graphics/clip_sync_system_test.rs"]
mod clip_sync_system_test;
#[path = "graphics/command_list_test.rs"]
mod command_list_test;
#[path = "graphics/core_accessor_test.rs"]
mod core_accessor_test;
#[path = "graphics/components_test.rs"]
mod components_test;
#[path = "graphics/core_ecs_test.rs"]
mod core_ecs_test;
#[path = "graphics/core_test.rs"]
mod core_test;
#[path = "graphics/dcomp_integration_test.rs"]
mod dcomp_integration_test;
#[path = "graphics/init_window_graphics_test.rs"]
mod init_window_graphics_test;
#[path = "graphics/reinit_unit_test.rs"]
mod reinit_unit_test;
#[path = "graphics/surface_optimization_test.rs"]
mod surface_optimization_test;
#[path = "graphics/surface_pixel_equivalence_test.rs"]
mod surface_pixel_equivalence_test;
#[path = "graphics/surface_systems_test.rs"]
mod surface_systems_test;
#[path = "graphics/frame_time_test.rs"]
mod frame_time_test;
#[path = "graphics/window_pos_systems_test.rs"]
mod window_pos_systems_test;
#[path = "graphics/wuc_restart_regression_test.rs"]
mod wuc_restart_regression_test;
