//! バルーン選択肢対話配線（areka-P0-choice-interact）。
//!
//! バルーン窓のポインタイベントを捉え、選択肢ヒットの判定・ハイライト追従・クリック確定を
//! kanade／文字層 runtime へ橋渡しするサブモジュール。単一責務＝バルーン選択肢対話配線。
//!
//! 本 task（1）はモジュール雛形のみを確立する。以降の task（2.x〜6.x）で本 mod へ増設される:
//! - 契約型（`ChoiceSelection` ほか）
//! - NonSend 資源（`BalloonWiring`・`ChoiceSelectionInbox`）
//! - 純関数判定核（選択肢ヒット・ハイライト遷移の決定的判定）
//! - 配線層（`on_balloon_pointer_moved`／`on_balloon_pointer_pressed`・
//!   `attach_balloon_pointer_handlers`・`wire_balloon_choice`）
//!
//! 上流契約（collision-geometry の resolver・`Emo2Wiring::runtime()` の読み口）は消費のみ行い、
//! 逆方向依存（上流が balloon.rs を知る）は禁止（design「依存方向」）。

/// 選択確定のワイヤ形（本 spec 契約正本・2.2）。
///
/// 下流 W6 が表示層へ再照会せず選択解決とカスケード発火を組み立てられる自己完結データ。
/// 解決キーは `id`（`SakuraMsg::ResolveChoice { id }` と整合）であり、表示層内部の主キーである
/// `ordinal` はワイヤ形に含めない（漏洩防止・design 2.6）。本 task（2.1）は型定義のみで、
/// 発行・`resolve_choice` 呼出は行わない（後続 task の範囲）。フィールド消費は下流 task で結線される。
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChoiceSelection {
    /// `\q` ID（選択解決の主キー・不透明転写）。
    pub id: String,
    /// 表示ラベル（不透明転写）。
    pub label: String,
    /// 発生元 scope（`BalloonWindowMarker.scope` 由来）。
    pub scope: usize,
    /// `\q` 第 3 引数以降（参照列・不透明転写）。
    pub references: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ChoiceSelection {
        ChoiceSelection {
            id: "q0".to_string(),
            label: "はい".to_string(),
            scope: 0,
            references: vec!["ref0".to_string(), "ref1".to_string()],
        }
    }

    #[test]
    fn identical_field_contents_compare_equal() {
        let a = sample();
        let b = sample();
        assert_eq!(a, b, "同一フィールド内容の ChoiceSelection は等価であるべき");
    }

    #[test]
    fn differing_field_contents_compare_unequal() {
        let base = sample();

        let mut different_id = sample();
        different_id.id = "q1".to_string();
        assert_ne!(base, different_id, "id が異なれば非等価であるべき");

        let mut different_refs = sample();
        different_refs.references = vec!["ref0".to_string()];
        assert_ne!(base, different_refs, "references が異なれば非等価であるべき");
    }

    #[test]
    fn clone_equals_original_and_debug_is_usable() {
        let original = sample();
        let cloned = original.clone();
        assert_eq!(original, cloned, "clone は元と等価であるべき（Clone 導出の証跡）");

        let rendered = format!("{original:?}");
        assert!(!rendered.is_empty(), "Debug 出力は非空であるべき（Debug 導出の証跡）");
    }
}
