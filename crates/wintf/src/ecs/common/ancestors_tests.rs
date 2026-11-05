//! `ancestors` のテスト（根で止まる・輪でも上限で止まる・上限ちょうどの鎖は全部返す）。

use super::*;

/// 親の関係を閉包で組むための entity を `n` 個作る。
fn entities(n: usize) -> Vec<Entity> {
    let mut world = World::new();
    (0..n).map(|_| world.spawn_empty().id()).collect()
}

fn pos(ids: &[Entity], x: Entity) -> usize {
    ids.iter().position(|&i| i == x).unwrap()
}

#[test]
fn stops_at_root_in_order() {
    // ids[2] → ids[1] → ids[0]（根）
    let ids = entities(3);
    let got: Vec<Entity> =
        ancestors(ids[2], |x| pos(&ids, x).checked_sub(1).map(|i| ids[i])).collect();
    assert_eq!(got, vec![ids[1], ids[0]]);
}

#[test]
fn root_has_no_ancestors() {
    let ids = entities(1);
    assert_eq!(ancestors(ids[0], |_| None).count(), 0);
}

#[test]
fn cycle_stops_at_limit() {
    // ids[0] → ids[1] → ids[0] → …（輪）
    let ids = entities(2);
    let n = ancestors(ids[0], |x| Some(ids[1 - pos(&ids, x)])).count();
    assert_eq!(n, MAX_ANCESTOR_DEPTH);
}

#[test]
fn self_loop_stops_at_limit() {
    let ids = entities(1);
    assert_eq!(ancestors(ids[0], Some).count(), MAX_ANCESTOR_DEPTH);
}

#[test]
fn chain_of_exactly_limit_is_fully_walked() {
    // 上限ちょうどの段数の鎖は欠けずに全部返る（根で止まり、上限の error! は出ない）
    let ids = entities(MAX_ANCESTOR_DEPTH + 1);
    let top = *ids.last().unwrap();
    let n = ancestors(top, |x| pos(&ids, x).checked_sub(1).map(|i| ids[i])).count();
    assert_eq!(n, MAX_ANCESTOR_DEPTH);
}
