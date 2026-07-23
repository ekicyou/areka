//! `areka-ghost` ⓪ghost の sylphya 結線層（task 8.1〜8.2）。
//!
//! ここでは descript 由来の名前情報（`GhostNames`）から sylphya のフラット静的値
//! （`selfname`／`selfname2`／`keroname`）を導出する純関数 [`derive_flat_statics`] を提供する。
//! 実際の publish（`PublishStatic`）・provider 差替・prefetch sink 構成は task 8.2 が担う。
//!
//! # 決定論檻（要件 9.4）
//! [`derive_flat_statics`] は純関数（I/O・時計・乱数を持たない）であり、同一の `GhostNames`
//! からは常に同一順序・同一内容の `Vec` を返す。descript 実値解決の全判断分岐
//! （keroname の 3 分岐・selfname2 の有無・selfname の有無）を x64 純粋単体テストで檻に入れる。

use areka_parsers::package::GhostNames;

/// descript の名前情報 → sylphya フラット静的値（純関数・決定論檻対象・R9.4）。
///
/// 生成規則（要件 4.3／4.4／4.5・design「ghost（結線・provider 差替）」・
/// `doc/COMPAT_ARCHITECTURE.md` §8 対応表 ②③）:
///
/// - `sakura.name` が `Some(v)` → `("selfname", v)` を積む（R4.3）。未定義なら積まない（素通し縮退）。
/// - `sakura.name2` が `Some(v)` → `("selfname2", v)` を積む（R4.4）。未定義なら**何も積まない**
///   （素通し縮退・フォールバック創作なし・既定値なし）。
/// - keroname（R4.5・SSP 互換）:
///   - `kero.name` が `Some(v)` → `("keroname", v)` を積む。
///   - `kero.name` が `None` かつ `sakura.name` が `Some(v)` → `("keroname", v)` を積む
///     （SSP 互換フォールバック＝本体側の名前）。
///   - 両方 `None` → 何も積まない（素通し縮退）。
///
/// フラットトークン名は sylphya のフラット語彙に合わせ `%` を含まない
/// （`"selfname"`／`"selfname2"`／`"keroname"`）。返す `Vec` は決定論的な安定順
/// （selfname → selfname2 → keroname）で、これらは task 8.2 で ghost が `PublishStatic` する。
pub fn derive_flat_statics(names: &GhostNames) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();

    // %selfname＝sakura.name（R4.3）。未定義→積まない（素通し縮退）。
    if let Some(v) = &names.sakura_name {
        out.push(("selfname".to_string(), v.clone()));
    }

    // %selfname2＝sakura.name2（R4.4）。未定義→積まない（素通し縮退・対応表 ②）。
    if let Some(v) = &names.sakura_name2 {
        out.push(("selfname2".to_string(), v.clone()));
    }

    // %keroname＝kero.name。未定義なら sakura.name へフォールバック（SSP 互換・R4.5・対応表 ③）。
    // 両者未定義なら積まない（素通し縮退）。
    if let Some(v) = &names.kero_name {
        out.push(("keroname".to_string(), v.clone()));
    } else if let Some(v) = &names.sakura_name {
        out.push(("keroname".to_string(), v.clone()));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `GhostNames` は `#[non_exhaustive]` ゆえ他クレートから構造体リテラル構築不可。
    /// `Default` を起点にフィールドをミューテートして組み立てる（決定論檻の入力構築）。
    fn names(
        sakura_name: Option<&str>,
        sakura_name2: Option<&str>,
        kero_name: Option<&str>,
    ) -> GhostNames {
        let mut n = GhostNames::default();
        n.sakura_name = sakura_name.map(|s| s.to_string());
        n.sakura_name2 = sakura_name2.map(|s| s.to_string());
        n.kero_name = kero_name.map(|s| s.to_string());
        n
    }

    // --- selfname（R4.3） ---

    /// sakura.name あり → `selfname` エントリが実値で積まれる（R4.3）。
    #[test]
    fn selfname_present_when_sakura_name_defined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, None));
        assert!(
            got.contains(&("selfname".to_string(), "むらさき".to_string())),
            "sakura.name 定義時は selfname が実値で積まれるべき: {got:?}"
        );
    }

    /// sakura.name 未定義 → `selfname` エントリを積まない（素通し縮退・R4.3）。
    #[test]
    fn selfname_absent_when_sakura_name_undefined() {
        let got = derive_flat_statics(&names(None, None, None));
        assert!(
            !got.iter().any(|(k, _)| k == "selfname"),
            "sakura.name 未定義時は selfname を積まない（素通し）: {got:?}"
        );
    }

    // --- selfname2（R4.4・対応表 ②） ---

    /// sakura.name2 あり → `selfname2` エントリが実値で積まれる（R4.4）。
    #[test]
    fn selfname2_present_when_name2_defined() {
        let got = derive_flat_statics(&names(Some("むらさき"), Some("紫"), None));
        assert!(
            got.contains(&("selfname2".to_string(), "紫".to_string())),
            "sakura.name2 定義時は selfname2 が実値で積まれるべき: {got:?}"
        );
    }

    /// sakura.name2 未定義 → `selfname2` エントリを積まない（素通し縮退・フォールバック創作なし・R4.4）。
    #[test]
    fn selfname2_absent_when_name2_undefined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, None));
        assert!(
            !got.iter().any(|(k, _)| k == "selfname2"),
            "sakura.name2 未定義時は selfname2 を積まない（素通し・既定値やフォールバックを創作しない）: {got:?}"
        );
    }

    // --- keroname 3 分岐（R4.5・対応表 ③） ---

    /// (a) kero.name あり → `keroname` = kero.name（R4.5）。
    #[test]
    fn keroname_from_kero_name_when_defined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, Some("エモ")));
        assert!(
            got.contains(&("keroname".to_string(), "エモ".to_string())),
            "kero.name 定義時は keroname = kero.name: {got:?}"
        );
    }

    /// (b) kero.name 未定義＋sakura.name あり → `keroname` = sakura.name（SSP 互換フォールバック・R4.5）。
    #[test]
    fn keroname_falls_back_to_sakura_name_when_kero_undefined() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, None));
        assert!(
            got.contains(&("keroname".to_string(), "むらさき".to_string())),
            "kero.name 未定義＋sakura.name 定義時は keroname が sakura.name へフォールバック: {got:?}"
        );
    }

    /// (c) kero.name・sakura.name 両方未定義 → `keroname` エントリを積まない（素通し縮退・R4.5）。
    #[test]
    fn keroname_absent_when_both_undefined() {
        let got = derive_flat_statics(&names(None, None, None));
        assert!(
            !got.iter().any(|(k, _)| k == "keroname"),
            "kero.name・sakura.name 両方未定義時は keroname を積まない（素通し）: {got:?}"
        );
    }

    /// kero.name が定義されていれば sakura.name の有無に依らず kero.name が勝つ（フォールバック非適用）。
    #[test]
    fn keroname_prefers_kero_name_over_sakura_fallback() {
        let got = derive_flat_statics(&names(Some("むらさき"), None, Some("エモ")));
        assert!(
            got.contains(&("keroname".to_string(), "エモ".to_string()))
                && !got.contains(&("keroname".to_string(), "むらさき".to_string())),
            "kero.name 定義時は sakura.name へフォールバックしない: {got:?}"
        );
    }

    // --- 全語彙同時・順序・決定論 ---

    /// 全て定義 → selfname／selfname2／keroname がこの安定順で並ぶ（決定論の順序契約）。
    #[test]
    fn full_names_produce_stable_order() {
        let got = derive_flat_statics(&names(Some("むらさき"), Some("紫"), Some("エモ")));
        assert_eq!(
            got,
            vec![
                ("selfname".to_string(), "むらさき".to_string()),
                ("selfname2".to_string(), "紫".to_string()),
                ("keroname".to_string(), "エモ".to_string()),
            ],
            "安定順（selfname → selfname2 → keroname）で並ぶべき"
        );
    }

    /// 同一 `GhostNames` からは常に同一の `Vec`（順序・内容）を返す（決定論・R9.4/R2.5）。
    #[test]
    fn deterministic_same_input_same_output() {
        let n = names(Some("むらさき"), None, None);
        let a = derive_flat_statics(&n);
        let b = derive_flat_statics(&n);
        assert_eq!(a, b, "同一入力は同一出力（決定論）");
        // フォールバック経路でも selfname と keroname の両方が sakura.name 由来で並ぶ。
        assert_eq!(
            a,
            vec![
                ("selfname".to_string(), "むらさき".to_string()),
                ("keroname".to_string(), "むらさき".to_string()),
            ]
        );
    }
}
