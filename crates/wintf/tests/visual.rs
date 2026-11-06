// tests/visual.rs — wintf visual domain test entry point
#[path = "visual/child_order_test.rs"]
mod child_order_test;
#[path = "visual/common/mod.rs"]
mod common;
#[path = "visual/component_test.rs"]
mod component_test;
// GPU の持ち主のスレッドは graphics の実行ファイルと同じファイルを使う（写さない）。
#[path = "graphics/common/mod.rs"]
mod gpu;
#[path = "visual/graphics_auto_creation_test.rs"]
mod graphics_auto_creation_test;
#[path = "visual/hierarchy_reparent_gap_test.rs"]
mod hierarchy_reparent_gap_test;
#[path = "visual/hierarchy_sync_test.rs"]
mod hierarchy_sync_test;
#[path = "visual/property_sync_test.rs"]
mod property_sync_test;
#[path = "visual/resource_management_gap_test.rs"]
mod resource_management_gap_test;
#[path = "visual/widget_visual_auto_insert_test.rs"]
mod widget_visual_auto_insert_test;
#[path = "visual/window_join_test.rs"]
mod window_join_test;
