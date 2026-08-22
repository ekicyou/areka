// tests/visual.rs — wintf visual domain test entry point
#[path = "visual/child_order_test.rs"]
mod child_order_test;
#[path = "visual/common/mod.rs"]
mod common;
#[path = "visual/component_test.rs"]
mod component_test;
#[path = "visual/graphics_auto_creation_test.rs"]
mod graphics_auto_creation_test;
#[path = "visual/hierarchy_reparent_gap_test.rs"]
mod hierarchy_reparent_gap_test;
#[path = "visual/hierarchy_sync_test.rs"]
mod hierarchy_sync_test;
#[path = "visual/insert_visual_test.rs"]
mod insert_visual_test;
#[path = "visual/parent_visual_test.rs"]
mod parent_visual_test;
#[path = "visual/property_sync_test.rs"]
mod property_sync_test;
#[path = "visual/remove_visual_api_test.rs"]
mod remove_visual_api_test;
#[path = "visual/resource_management_gap_test.rs"]
mod resource_management_gap_test;
#[path = "visual/widget_visual_auto_insert_test.rs"]
mod widget_visual_auto_insert_test;
