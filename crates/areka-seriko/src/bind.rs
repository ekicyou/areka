//! bind（着せ替え）構築モジュール。
//!
//! bindgroup の既定オン集合（番号）から、合成入力が要求する静的 bind 集合
//! （[`areka_emo_compose::BindSet`]）をアクター構築時に一度だけ組み立てる。
//!
//! bindgroup 番号は animation ID の**恒等写像**（決定3 / R-2）。中間変換・添字付け・
//! 特別扱いは一切行わず、番号をそのまま [`BindSet::from_ids`] へ流す。整列・重複除去は
//! `from_ids` が担う（`BindSet` は昇順・dedup 済み）。

/// bindgroup default 番号集合 → 静的 [`BindSet`]（恒等写像・`from_ids` が整列/dedup）。
///
/// bindgroup 番号 = animation ID（決定3 / R-2）。呼び手（アクター構築時）が渡した
/// 既定オン集合をそのまま `BindSet::from_ids` へ渡すのみ。状態を持たず、構築時に一度だけ
/// 呼ばれる純関数（要件 4.1）。
///
/// [`BindSet`]: areka_emo_compose::BindSet
/// [`BindSet::from_ids`]: areka_emo_compose::BindSet::from_ids
pub fn build_static_bindset(default_on: &[u32]) -> areka_emo_compose::BindSet {
    areka_emo_compose::BindSet::from_ids(default_on.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 恒等写像（KEY・emo2 追験値）: 既定オン集合の番号がそのまま bind 集合の
    /// animation ID になる。emo2 実測の `default,1` 集合 `{1100,1207,1302,1500,1800}`
    /// （既に昇順）を渡し、`ids()` が同一スライスを返すことを全値比較で確認する。
    #[test]
    fn identity_mapping_emo2_default_on() {
        let set = build_static_bindset(&[1100, 1207, 1302, 1500, 1800]);
        assert_eq!(
            set.ids(),
            &[1100, 1207, 1302, 1500, 1800],
            "bindgroup 番号は animation ID の恒等写像（決定3 / R-2）: 変換なし"
        );
    }

    /// 整列＋dedup パススルー: 未整列・重複入力を渡すと、`from_ids` により
    /// 昇順・重複除去された集合へ正規化される（発明された変換ではなく忠実な passthrough）。
    #[test]
    fn sort_and_dedup_passthrough() {
        let set = build_static_bindset(&[1800, 1100, 1100, 1207]);
        assert_eq!(
            set.ids(),
            &[1100, 1207, 1800],
            "未整列・重複入力は from_ids がそのまま整列/dedup する（集合の恒等・独自変換なし）"
        );
    }

    /// 空入力: kero 側は emo2 で空（design Risks）。空スライスから空 BindSet を得る。
    #[test]
    fn empty_input_yields_empty_bindset() {
        let set = build_static_bindset(&[]);
        assert_eq!(set.ids(), &[] as &[u32], "空の既定オン集合は空の bind 集合");
        assert_eq!(
            set,
            areka_emo_compose::BindSet::default(),
            "空入力は既定（空）BindSet と等価"
        );
    }
}
