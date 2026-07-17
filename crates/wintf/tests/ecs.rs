// tests/ecs.rs — wintf ecs domain test entry point
#[path = "ecs/component_state_pattern_test.rs"]
mod component_state_pattern_test;
#[path = "ecs/lazy_reinit_pattern_test.rs"]
mod lazy_reinit_pattern_test;
#[path = "ecs/resource_removal_detection_test.rs"]
mod resource_removal_detection_test;
#[path = "ecs/tree_propagation_test.rs"]
mod tree_propagation_test;
#[path = "ecs/world_lifecycle_test.rs"]
mod world_lifecycle_test;

#[path = "ecs/dola_animator_test.rs"]
mod dola_animator_test;
