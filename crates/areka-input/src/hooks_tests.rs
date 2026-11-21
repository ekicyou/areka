use bevy_ecs::prelude::*;
use log_capture_kit::capture;

use super::escape;

#[test]
fn escape_without_hooks_logs_and_still_requests_exit() {
    let mut world = World::new();
    world.insert_non_send(wintf::AppExit::new());

    let ((), events) = capture(|| escape(&mut world));

    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    assert_eq!(errors[0].field_str("event"), Some("input_hooks_missing"));
    assert!(
        world
            .get_non_send::<wintf::AppExit>()
            .expect("直上で挿している")
            .is_requested()
    );
}
