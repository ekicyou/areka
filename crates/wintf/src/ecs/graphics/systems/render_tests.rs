//! render_surface: 続く失敗の記録の間引き（設計討議 議題 1・要件 6.6）

use super::render::{FailureLog, note_failure};
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};

fn entities() -> (Entity, Entity) {
    let mut world = World::new();
    (world.spawn_empty().id(), world.spawn_empty().id())
}

#[test]
fn same_stage_and_hresult_is_reported_once() {
    let (a, _) = entities();
    let mut log = FailureLog::default();
    assert!(
        note_failure(&mut log, a, "composite", E_FAIL),
        "1 回目は error! の番"
    );
    assert!(
        !note_failure(&mut log, a, "composite", E_FAIL),
        "2 回目は debug! の番"
    );
    assert!(
        !note_failure(&mut log, a, "composite", E_FAIL),
        "3 回目も debug! の番"
    );
}

#[test]
fn different_stage_or_hresult_is_reported_again() {
    let (a, _) = entities();
    let mut log = FailureLog::default();
    assert!(note_failure(&mut log, a, "composite", E_FAIL));
    assert!(
        note_failure(&mut log, a, "EndDraw", E_FAIL),
        "段が違えば error!"
    );
    assert!(
        note_failure(&mut log, a, "EndDraw", E_INVALIDARG),
        "HRESULT が違えば error!"
    );
    assert!(
        note_failure(&mut log, a, "EndDraw", E_FAIL),
        "別の失敗で控えが消えた後はまた error!"
    );
}

#[test]
fn success_clears_the_memo() {
    let (a, _) = entities();
    let mut log = FailureLog::default();
    assert!(note_failure(&mut log, a, "composite create", E_FAIL));
    log.remove(&a);
    assert!(
        note_failure(&mut log, a, "composite create", E_FAIL),
        "成功の後はまた error!"
    );
}

#[test]
fn memo_is_per_surface() {
    let (a, b) = entities();
    let mut log = FailureLog::default();
    assert!(note_failure(&mut log, a, "composite", E_FAIL));
    assert!(
        note_failure(&mut log, b, "composite", E_FAIL),
        "別の描画面は別の控え"
    );
    log.remove(&b);
    assert!(
        !note_failure(&mut log, a, "composite", E_FAIL),
        "別の描画面の成功は控えを消さない"
    );
}
