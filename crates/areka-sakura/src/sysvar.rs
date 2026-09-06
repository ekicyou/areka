//! システム変数（`%username` 等）展開の消費側契約（R7 の正本）。
//!
//! sakura は**値源を所有しない**。⓪ghost が talk 開始時に手渡す名前→値の凍結
//! スナップショット [`SystemVarSnapshot`]（プロパティシステム＝`areka-P0-sylphya`
//! 読み口の凍結像）を**参照するだけ**で、永続化層・SHIORI・OS 環境のいずれも直接
//! 読まない（R7.3/R7.6）。展開は純関数 [`resolve_system_var`]（no I/O・同一入力→
//! 同一出力＝決定論）で行い、tracing 記録は副作用のないログに留める。
//!
//! 縮退規則（R7.1/R7.4/R7.5）:
//! - スナップショットに当該名の値あり → [`ResolvedVar::Text`]（値・通常テキスト同格）。
//! - スナップショットに無く、かつ M1 対応語彙（`username` のみ）→ [`ResolvedVar::Text`]
//!   （既定値 [`DEFAULT_USERNAME`]・決定論定数）。
//! - それ以外（M1 未対応名）→ [`ResolvedVar::PassThrough`]（元の `%名前` をそのまま・
//!   情報を失わない縮退・記録付き）。
//!
//! 既定値 [`DEFAULT_USERNAME`] は**唯一の定義点**であり、⓪ghost の暫定 provider は
//! これを re-use する（`%username` だけの偽ストアを二重定義しない・R7.4）。

use std::collections::BTreeMap;

/// 名前→値の凍結スナップショット（プロパティシステム読み口の凍結像・R7.3）。
///
/// `BTreeMap` NewType＝キー順が決定論（走査・デバッグ表示が安定）。供給は ⓪ghost
/// （W1 暫定 provider→`areka-P0-sylphya` 読み口へ差し替え・sakura 側契約は無改変）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemVarSnapshot(BTreeMap<String, String>);

impl SystemVarSnapshot {
    /// 名前に対応する値を参照する（無ければ `None`）。
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// 名前→値を書き込む（provider がスナップショットを充填する口）。
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.insert(name.into(), value.into());
    }
}

/// `%username` 未解決時の既定値（R7.4・**唯一の定義点**）。
///
/// 正典は既定値を規定せず（沈黙）、伺かの伝統的な未指定時デフォルトを採る areka 裁量
/// （開発者裁定 2026-07-18・設計ディスカッション#2・`doc/COMPAT_ARCHITECTURE.md`
/// 対応表記録）。決定論定数（同一入力→同一出力を担保）。
pub const DEFAULT_USERNAME: &str = "ユーザーさん";

/// システム変数展開の結果（compile が Text cue へ写像する）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedVar {
    /// 展開成立（スナップショット値 or 既定値）＝通常テキスト同格（R7.1/R7.2/R7.4）。
    Text(String),
    /// M1 未対応名の素通し（`%名前` をそのまま・記録付き縮退・R7.5）。
    PassThrough(String),
}

/// システム変数名を純粋展開する（no I/O・同一入力→同一出力＝決定論・R7.6）。
///
/// - スナップショットに `name` の値あり → [`ResolvedVar::Text`]（値）。
/// - スナップショットに無く、かつ M1 対応語彙（`username` のみ）→ [`ResolvedVar::Text`]
///   （[`DEFAULT_USERNAME`]）。
/// - それ以外（M1 未対応名）→ [`ResolvedVar::PassThrough`]（`%{name}` のまま・記録付き）。
///
/// OS のユーザー名などの外部環境を暗黙に読まない（R7.6）。tracing 記録は副作用のない
/// ログでありテストの決定論を壊さない。
pub fn resolve_system_var(name: &str, vars: &SystemVarSnapshot) -> ResolvedVar {
    if let Some(value) = vars.get(name) {
        return ResolvedVar::Text(value.to_owned());
    }
    // M1 対応語彙は `username` のみ（他は源が着地した時点で just-in-time 実導出）。
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_25username:1
    if name == "username" {
        tracing::debug!(
            var = %name,
            "system var absent from snapshot; expanding to DEFAULT_USERNAME (R7.4)"
        );
        return ResolvedVar::Text(DEFAULT_USERNAME.to_owned());
    }
    // M1 未対応名: 元の `%名前` をそのまま素通し（情報を失わない縮退・R7.5）。
    tracing::debug!(
        var = %name,
        "unsupported system var; passing through raw `%name` (R7.5)"
    );
    ResolvedVar::PassThrough(format!("%{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SystemVarSnapshot::get`/`insert` の往復（provider の充填と読みが一致する）。
    #[test]
    fn snapshot_insert_get_round_trip() {
        let mut vars = SystemVarSnapshot::default();
        assert_eq!(vars.get("username"), None, "未挿入なら None");
        vars.insert("username", "アヒル");
        assert_eq!(vars.get("username"), Some("アヒル"), "挿入値を読み戻せる");
        // 上書きも往復する。
        vars.insert("username", "カモ");
        assert_eq!(
            vars.get("username"),
            Some("カモ"),
            "上書き後の値を読み戻せる"
        );
    }

    /// 値ありスナップショット → `Text(その値)`（R7.1）。
    #[test]
    fn value_present_expands_to_snapshot_value() {
        let mut vars = SystemVarSnapshot::default();
        vars.insert("username", "アヒル");
        assert_eq!(
            resolve_system_var("username", &vars),
            ResolvedVar::Text("アヒル".to_owned())
        );
    }

    /// `username` 欠落 → 既定値 `DEFAULT_USERNAME`（R7.4・生の `%username` を露出しない）。
    #[test]
    fn username_missing_expands_to_default() {
        let vars = SystemVarSnapshot::default();
        assert_eq!(
            resolve_system_var("username", &vars),
            ResolvedVar::Text(DEFAULT_USERNAME.to_owned())
        );
        // 既定値の実体は「ユーザーさん」（対応表記録・唯一の定義点）。
        assert_eq!(DEFAULT_USERNAME, "ユーザーさん");
    }

    /// M1 未対応名 → 素通し `%名前`（R7.5・情報を失わない縮退）。
    #[test]
    fn unsupported_name_passes_through_raw() {
        let vars = SystemVarSnapshot::default();
        assert_eq!(
            resolve_system_var("selfname", &vars),
            ResolvedVar::PassThrough("%selfname".to_owned())
        );
        // スナップショットに値があれば未対応名でも展開される（値源優先）。
        let mut vars2 = SystemVarSnapshot::default();
        vars2.insert("selfname", "さくら");
        assert_eq!(
            resolve_system_var("selfname", &vars2),
            ResolvedVar::Text("さくら".to_owned())
        );
    }

    /// 決定論: 同一 (name, snapshot) は常に同一出力（R7.6・no I/O・外部環境非読取）。
    #[test]
    fn resolution_is_deterministic() {
        let mut vars = SystemVarSnapshot::default();
        vars.insert("username", "アヒル");
        for name in ["username", "selfname", "keroname"] {
            let first = resolve_system_var(name, &vars);
            let second = resolve_system_var(name, &vars);
            assert_eq!(first, second, "{name} の展開が再計算で異なる");
        }
    }
}
