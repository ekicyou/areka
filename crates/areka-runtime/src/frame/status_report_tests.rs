// =============================================================================
// バルーンの表示の届けの決定論テスト（観測列から組を作る純関数の分岐）。
//
// 不可視・未装着・番号あり・番号なし・箱だけ・番号の覚え直しの分岐と、口が据わっていない
// World では何も送らないことを見る。送る・見送る・警告の側は本体の
// `runtime_tests/frame/status_report_tests.rs` が持つ。
// =============================================================================

use super::test_support::{binding, obs, obs_box};
use super::*;

// ---------------------------------------------------------------------------
// ⑴ 組を作る純関数
// ---------------------------------------------------------------------------

#[test]
fn collect_keeps_only_visible_scopes_in_input_order_with_their_number() {
    let (bindings, unknown) = collect_bindings(
        &[
            obs(0, Some(true), Some(2)),
            obs(1, Some(false), Some(5)), // 不可視
            obs(2, None, None),           // 未装着
            obs(3, Some(true), Some(0)),
        ],
        &BTreeMap::new(),
    );
    assert_eq!(bindings, vec![binding(0, 2), binding(3, 0)]);
    assert!(unknown.is_empty());
}

#[test]
fn collect_includes_visible_scope_without_number_as_zero_and_returns_it_as_warning_target() {
    let (bindings, unknown) = collect_bindings(
        &[
            obs(0, Some(true), None),
            obs(1, Some(true), Some(4)),
            obs(2, Some(false), None), // 見えていなければ警告の対象にもしない
        ],
        &BTreeMap::new(),
    );
    assert_eq!(bindings, vec![binding(0, 0), binding(1, 4)]);
    assert_eq!(unknown, vec![0]);
}

/// 箱だけに文字が出ているスコープも、普通のバルーンが見えているときと同じ形で載る
/// （番号は普通のバルーンの今の面）。どちらにも出ていなければ載らない。
/// 箱だけ・番号なし・覚えた番号なしは 0 で載り、警告の対象にしない。
#[test]
fn collect_lists_box_only_scopes_in_the_same_shape_as_visible_balloons() {
    let (bindings, unknown) = collect_bindings(
        &[
            obs(0, Some(true), Some(2)),      // 窓が見えている
            obs_box(1, Some(false), Some(5)), // 箱だけ
            obs(2, Some(false), Some(7)),     // どちらにも出ていない
            obs_box(3, Some(false), None),    // 箱だけ・番号なし
            obs_box(4, Some(true), Some(1)),  // 両方
        ],
        &BTreeMap::new(),
    );
    assert_eq!(
        bindings,
        vec![binding(0, 2), binding(1, 5), binding(3, 0), binding(4, 1)]
    );
    assert!(
        unknown.is_empty(),
        "箱だけは警告の対象にしない: {unknown:?}"
    );
}

/// 箱だけ・番号なしは覚えた番号で載り、警告の対象にしない。窓が見えていて番号なしは、覚えた番号が
/// あっても今までどおり 0 で警告の対象。
#[test]
fn collect_uses_the_remembered_number_for_box_only_scopes_without_a_number() {
    let remembered = BTreeMap::from([(0, 3), (1, 9), (2, 4)]);
    let (bindings, unknown) = collect_bindings(
        &[
            obs_box(0, Some(false), Some(6)), // 箱だけ・今の番号あり → 今の番号
            obs_box(1, Some(false), None),    // 箱だけ・今の番号なし → 覚えた番号
            obs(2, Some(true), None),         // 窓が見えて番号なし → 0 で警告
        ],
        &remembered,
    );
    assert_eq!(bindings, vec![binding(0, 6), binding(1, 9), binding(2, 0)]);
    assert_eq!(unknown, vec![2]);
}

#[test]
fn collect_of_nothing_visible_is_empty() {
    let (bindings, unknown) = collect_bindings(
        &[obs(0, Some(false), Some(1)), obs(1, None, None)],
        &BTreeMap::new(),
    );
    assert!(bindings.is_empty());
    assert!(unknown.is_empty());
}

/// 口（`SessionHooks`）が据わっていない World は `NO_BODY` が答えるので、何も送らず、送った組（`last_sent`）も動かさない。
#[test]
fn missing_slot_resource_sends_nothing() {
    let world = World::new();
    let mut ledger = BalloonStatusLedger::default();
    report_observed(&world, &mut ledger, &[obs(0, Some(true), Some(0))]);
    assert!(ledger.last_sent.is_empty());
}
