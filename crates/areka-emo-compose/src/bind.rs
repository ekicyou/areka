//! `BindSet`: 呼び手が渡す有効 bind 集合（`Send` 所有・整列済み）。
//!
//! bindgroup 切替・blink 発火・着せ替えなどの動的状態管理は行わず、渡された静的集合のみを
//! 合成対象とする。集合内の順序は `animation-sort` → animation ID の2段規則に沿って整列される。

/// 有効 bind 集合（animation ID の整列済み集合・`Send` 所有・重複なし）。
///
/// 内部表現は昇順・重複除去済みの `Vec<u32>`（opaque）。呼び手が渡した順序には依存せず、
/// [`from_ids`] が構築時に一度だけ整列＋dedup する。以降 [`contains`] は二分探索、
/// [`ids`] は昇順スライスの参照を返す。
///
/// [`from_ids`]: BindSet::from_ids
/// [`contains`]: BindSet::contains
/// [`ids`]: BindSet::ids
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BindSet(Vec<u32>);

impl BindSet {
    /// animation ID 群から `BindSet` を構築する（昇順整列＋重複除去）。
    ///
    /// 入力順序は問わない。同一 ID の重複は 1 件へ畳み込む。
    pub fn from_ids(ids: impl IntoIterator<Item = u32>) -> Self {
        let mut v: Vec<u32> = ids.into_iter().collect();
        v.sort_unstable();
        v.dedup();
        BindSet(v)
    }

    /// 指定 animation ID が有効集合に含まれるか（昇順前提の二分探索）。
    pub fn contains(&self, animation_id: u32) -> bool {
        self.0.binary_search(&animation_id).is_ok()
    }

    /// 有効 animation ID の昇順スライス。
    pub fn ids(&self) -> &[u32] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 構築時に昇順整列＋重複除去される（入力順・重複不問）。
    #[test]
    fn from_ids_sorts_and_dedups() {
        let set = BindSet::from_ids([3, 1, 2, 1]);
        assert_eq!(set.ids(), &[1, 2, 3]);
    }

    /// `contains` は二分探索で在／不在を判定する。
    #[test]
    fn contains_reports_membership() {
        let set = BindSet::from_ids([3, 1, 2, 1]);
        assert!(set.contains(2));
        assert!(!set.contains(9));
    }

    /// 空集合は既定値と一致し、何も含まない。
    #[test]
    fn default_is_empty() {
        let set = BindSet::default();
        assert!(set.ids().is_empty());
        assert!(!set.contains(0));
        assert_eq!(set, BindSet::from_ids([]));
    }
}
