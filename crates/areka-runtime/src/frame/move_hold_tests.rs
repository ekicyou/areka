//! 預かりの窓の集合（`Blocked`）の件数の形の檻。

use super::Blocked;

#[test]
fn blocked_keeps_one_entry_per_window_however_many_cues_wait() {
    let mut blocked = Blocked::default();
    for _ in 0..1_000 {
        blocked.add(Some(vec![0, 1]));
    }
    assert_eq!(blocked.scopes.len(), 2);
    assert!(blocked.overlaps(&Some(vec![1])));
    assert!(!blocked.overlaps(&Some(vec![2])));
}
