//! `-1` で止まった重ね絵が指令に残らないことの決定論テスト。

use super::tests::*;
use super::*;
use areka_parsers::shell::{Animation, Interval};
use dola::time::testing::at_ms;

// ── `-1` で止まった重ね絵は指令に残らない ──

/// ぱすたの animation1400 と同じ並び（bind+random・1410 を 0ms・1411 を 120ms・`-1` を 50ms）を
/// 16ms の刻みで進めると、閉じる → 半目 → 外す の順に指令が出て、止まった後の指令に 1400 の欄が無い
/// （seriko の側の境目の判定・外から与えた時刻と乱数だけで決まる）。
#[test]
fn blink_ending_in_minus_one_leaves_no_frame_in_the_command_after_stop() {
    let table = table_single(
        1000,
        1400,
        Interval::BindRandom { k: 4 },
        &[(1410, 0), (1411, 120), (-1, 50)],
    );
    let (rng, _probe) = counting_rng(&[0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, scope) = shown_shell_with_binds(1000, BindSet::from_ids([1400]));
    assert!(rt.on_tick(at_ms(0), &mut states).is_empty());
    let mut seen: Vec<Option<u32>> = Vec::new();
    let mut now = 1000;
    while now <= 1400 {
        for cmd in rt.on_tick(at_ms(now), &mut states) {
            let DisplayCommand::Show { pattern, .. } = cmd else {
                panic!("Show を期待");
            };
            seen.push(pattern.get(1400).map(|f| f.surface_id));
        }
        now += 16;
    }
    assert_eq!(seen, [Some(1410), Some(1411), None], "閉じる → 半目 → 外す");
    assert!(
        states
            .current_pattern(&scope, Slot::Shell)
            .get(1400)
            .is_none()
    );
}

/// 同じ面に動く絵（純 `random` の 1500・1500 → 1501 で `-1` を持たず最後のコマを保つ）が並ぶとき、
/// まばたきが止まった後の指令は 1400 の欄だけを欠き、1500 の欄は残る（表示の側はこの指令で矩形の
/// 重ね直しの道を通る・`presenter_blink_stop_tests.rs`）。
#[test]
fn blink_stop_beside_a_moving_picture_drops_only_the_blink_slot() {
    let blink = Animation {
        id: 1400,
        interval: Interval::BindRandom { k: 4 },
        patterns: vec![pat(0, 1410, 0), pat(1, 1411, 120), pat(2, -1, 50)],
    };
    let mover = Animation {
        id: 1500,
        interval: Interval::Random { k: 2 },
        patterns: vec![pat(0, 1500, 0), pat(1, 1501, 100)],
    };
    let table = shell_table(vec![surface_with(1000, vec![blink, mover])]);
    let (rng, _probe) = counting_rng(&[0, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, scope) = shown_shell_with_binds(1000, BindSet::from_ids([1400]));
    assert!(rt.on_tick(at_ms(0), &mut states).is_empty());
    let mut seen: Vec<(Option<u32>, Option<u32>)> = Vec::new();
    let mut now = 1000;
    while now <= 1400 {
        for cmd in rt.on_tick(at_ms(now), &mut states) {
            let DisplayCommand::Show { pattern, .. } = cmd else {
                panic!("Show を期待");
            };
            let slot = |id| pattern.get(id).map(|f| f.surface_id);
            seen.push((slot(1400), slot(1500)));
        }
        now += 16;
    }
    assert!(
        seen.iter().any(|&(_, m)| m == Some(1500)) && seen.iter().any(|&(_, m)| m == Some(1501)),
        "動く絵がコマを替える: {seen:?}"
    );
    assert_eq!(
        seen.last(),
        Some(&(None, Some(1501))),
        "止まった後は 1400 だけが欠ける: {seen:?}"
    );
    let after = states.current_pattern(&scope, Slot::Shell);
    assert!(after.get(1400).is_none());
    assert_eq!(after.get(1500).map(|f| f.surface_id), Some(1501));
}
