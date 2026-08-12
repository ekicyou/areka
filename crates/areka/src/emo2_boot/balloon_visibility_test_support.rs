// =============================================================================
// 判断中核テストの共有ヘルパ（観測スナップショットの組立て）
//
// design.md「File Structure Plan」の `balloon_visibility_test_support.rs`。
// 観測は「scope → (可視グリフ数, 現に可視か)」の表でしかないため、ここに置くのは
// その表を組む最小の道具だけにする。実時間の待機・スリープは一切用いない。
// =============================================================================

use std::collections::BTreeMap;

use super::{
    BalloonVisibilityState, ScopeObservation, VisibilityDecision, VisibilityObservations, decide,
};

/// 観測できた scope（可視グリフ数と実可視）。
pub(crate) fn seen(visible_glyphs: usize, visible: bool) -> ScopeObservation {
    ScopeObservation {
        visible_glyphs: Some(visible_glyphs),
        visible,
    }
}

/// グリフ数の観測が取れなかった scope（本番では文字層ランタイムの借用失敗）。
pub(crate) fn unobserved(visible: bool) -> ScopeObservation {
    ScopeObservation {
        visible_glyphs: None,
        visible,
    }
}

/// 観測スナップショットを組む。引数の並び順は結果に影響しない（走査は scope 昇順）。
pub(crate) fn observations(entries: &[(u32, ScopeObservation)]) -> VisibilityObservations {
    VisibilityObservations {
        scopes: entries.iter().copied().collect::<BTreeMap<_, _>>(),
    }
}

/// 1 フレーム分の判断を進める（時刻とタイムアウトは本タスクの判断が読まない値を固定で渡す）。
///
/// 固定値は「読まないこと」の主張も兼ねる——タイムアウト側の判断が本タスクへ紛れ込めば、
/// この呼び方をしているテストのどれかが必ず落ちる。
pub(crate) fn step(
    state: &mut BalloonVisibilityState,
    entries: &[(u32, ScopeObservation)],
) -> VisibilityDecision {
    decide(state, &observations(entries), None, 30.0)
}
