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

// cue-system tests
#[path = "ecs/cue_barrier_test.rs"]
mod cue_barrier_test;
#[path = "ecs/cue_data_model_test.rs"]
mod cue_data_model_test;
#[path = "ecs/cue_dispatch_e2e_test.rs"]
mod cue_dispatch_e2e_test;
#[path = "ecs/cue_performance_test.rs"]
mod cue_performance_test;
#[path = "ecs/cue_queue_test.rs"]
mod cue_queue_test;
#[path = "ecs/cue_registry_test.rs"]
mod cue_registry_test;
#[path = "ecs/cue_tracker_lifecycle_test.rs"]
mod cue_tracker_lifecycle_test;
#[path = "ecs/cue_integration_test.rs"]
mod cue_integration_test;
#[path = "ecs/dola_animator_test.rs"]
mod dola_animator_test;
