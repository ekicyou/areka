//! model — マウントモデル・失敗型の正本（型の正本）。
//!
//! 解決済みマウント所在（`MountModel` と付随値型 `GhostNames` / `ShioriMount` /
//! `ShellMount`）と、マウント解決の観測可能な失敗（`MountError`）を定義する。
//! この型群は下流（`ghost-setup` / `host-32` / `shell-parse`）と共有する
//! I/O 契約の片側であり、本 spec が生成者・正本を所有する。
//!
//! 設計規律（design.md「型定義（model）」）:
//! - 純粋な型定義に留める（I/O・`Result` は `resolve` サブモジュールが持つ）。
//! - 派生は `Clone` / `Debug` / `PartialEq` / `Eq`（文字列/パスのみで
//!   `f32`/`Duration` を含まないため `Eq` 付与可・`sakura::Instruction` との差異）。
//!   `serde` は付さない（他兄弟型と整合・不要）。
//! - `#[non_exhaustive]` により後続のフィールド/variant 追加を後方互換に保つ。
//! - 名前情報・SHIORI ファイル名は `Option`（欠落を型で表現・推測しない・Req 2.3）。
//!   パス表現は `PathBuf`。
//!
//! 不変条件（design.md「Preconditions/Postconditions/Invariants」）:
//! - `MountModel` は `resolve` 成功時のみ構築される。
//! - `shiori.dir` は起点 descript.txt の親（物理存在確定）、`shell.dir` は
//!   物理存在確認済み。
//! - `shiori.file` / `names.*` は `Option` で欠落を保持し、既定値は推測しない
//!   （`shell.dir` の `master` フォールバックのみ ukadoc 既定で例外）。

use std::path::PathBuf;

/// 解決済みゴーストマウントモデル（下流 I/O 契約の正本）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountModel {
    /// ゴースト名前情報（欠落は None・Req 1.4）。
    pub names: GhostNames,
    /// SHIORI マウント先（Req 2.1/2.2/2.3）。
    pub shiori: ShioriMount,
    /// shell マウント先（Req 3.1/3.2）。
    pub shell: ShellMount,
    /// shell descript の bindgroup default 転記（bindopt 1.1/1.2・既存 3 フィールドと非衝突）。
    pub bindgroups: BindGroupDefaults,
}

/// shell descript.txt の bindgroup default（`default,1`＝起動時オン）の転記保持。
///
/// `sakura.bindgroup*.default,数値`／`kero.bindgroup*.default,数値`（ukadoc カテゴリ
/// `descript_shell`）のうち値が `1` のものについて、bindgroup 番号（`*`）を本体
/// （sakura）・相方（kero）スコープ別に保持する。**転記のみ・展開しない**（範囲展開や
/// surface 解決は行わない・parsers 転写層原則）。保持は転記順（昇順不問）で、下流
/// （seriko の `build_static_bindset`）が集合として扱う。欠落スコープは空 `Vec`。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BindGroupDefaults {
    /// `default,1` の bindgroup 番号（sakura スコープ・昇順不問・保持は転記順）。
    pub sakura_default_on: Vec<u32>,
    /// `default,1` の bindgroup 番号（kero スコープ・昇順不問・保持は転記順）。
    pub kero_default_on: Vec<u32>,
    /// `sakura.bindgroup*.name,カテゴリ名,パーツ名,…` の名前宣言転記（本体側・Req 1.1）。
    /// 保持は転記順（欠落スコープは空 `Vec`・task 1.2 が resolve から充填する）。
    pub sakura_names: Vec<BindGroupName>,
    /// `kero.bindgroup*.name,…` の名前宣言転記（相方側・本体側と区別・Req 1.2）。
    pub kero_names: Vec<BindGroupName>,
    /// `sakura.bindoption*.group` のオプション欄に `mustselect` が現れたカテゴリ名（本体側・bindopt 1.2）。
    ///
    /// ukadoc 正典の 3 値（`mustselect`＝ちょうど 1 個・解除不可／非宣言＝既定で高々 1 個・
    /// 解除可／`multiple`＝複数可）のうち `mustselect` 宣言の**所属**のみを写す。非宣言
    /// カテゴリはどちらの集合にも入らない＝下流が正典の既定として扱う（3 値の区別が成立
    /// する）。`+` 区切り併記（`mustselect+multiple`）は両集合へ転記して情報を落とさない
    /// （転記のみ・解釈しない・保持は転記順・欠落は空 `Vec`）。
    pub sakura_mustselect: Vec<String>,
    /// `kero.bindoption*.group` のオプション欄に `mustselect` が現れたカテゴリ名
    /// （相方側・本体側と区別・bindopt 1.2）。
    pub kero_mustselect: Vec<String>,
    /// `sakura.bindoption*.group` のオプション欄に `multiple` が現れたカテゴリ名（本体側・bindopt 1.1）。
    ///
    /// `multiple`（複数可）宣言の**所属**を写し、非宣言（既定）と区別可能にする。旧実装は
    /// この宣言を破棄していたため下流に「明示 multiple」と「非宣言」を区別する情報が存在
    /// しなかった（転記のみ・解釈しない・保持は転記順・欠落は空 `Vec`）。
    pub sakura_multiple: Vec<String>,
    /// `kero.bindoption*.group` のオプション欄に `multiple` が現れたカテゴリ名
    /// （相方側・本体側と区別・bindopt 1.1）。
    pub kero_multiple: Vec<String>,
}

/// bindgroup 名前宣言 1 件の忠実転記（不透明文字列・ID 非生成・Req 1.5）。
///
/// `<prefix>NNNN.name,カテゴリ名,パーツ名,…` の 1 行を、bindgroup 番号 `NNNN`
/// （= 着せ替え/animation ID・恒等）とカテゴリ名・パーツ名・残余サムネイル名として
/// 忠実転記する。名前は不透明な文字列として保持し、宣言されていない着せ替え ID を
/// 新たに生成しない（Req 1.5・parsers 転写層原則）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindGroupName {
    /// bindgroup 番号（= 着せ替え/animation ID・恒等）。
    pub id: u32,
    /// 第 1 フィールド（カテゴリ名・trim 済み・不透明）。
    pub category: String,
    /// 第 2 フィールド（パーツ名・trim 済み・不透明）。
    pub part: String,
    /// 第 3 フィールド以降の残余（サムネイル名・M1 不使用・保持のみ）。
    pub thumbnail: Option<String>,
}

/// 名前空間選択（sakura=本体／kero=相方・Req 1.2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindScope {
    /// 本体側（`sakura.bindgroup*`）。
    Sakura,
    /// 相方側（`kero.bindgroup*`）。
    Kero,
}

impl BindGroupDefaults {
    /// スコープに対応する名前宣言スライスを返す（内部ヘルパ）。
    fn names(&self, scope: BindScope) -> &[BindGroupName] {
        match scope {
            BindScope::Sakura => &self.sakura_names,
            BindScope::Kero => &self.kero_names,
        }
    }

    /// (カテゴリ名, パーツ名) → 着せ替え ID。未宣言は None（捏造しない・Req 1.3/1.4）。
    ///
    /// 重複宣言（同一 (カテゴリ, パーツ)）はキー昇順走査の後勝ち（design D2）＝転記順
    /// を走査し最後の一致を採る。純関数（同一入力同一出力・副作用なし）。
    pub fn resolve_name(&self, scope: BindScope, category: &str, part: &str) -> Option<u32> {
        self.names(scope)
            .iter()
            .rev()
            .find(|n| n.category == category && n.part == part)
            .map(|n| n.id)
    }

    /// カテゴリに属する着せ替え ID の集合（昇順・重複排除・Req 1.3 後段）。
    ///
    /// 純関数。M1 の seriko は未消費＝将来差替シーム。
    pub fn category_ids(&self, scope: BindScope, category: &str) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .names(scope)
            .iter()
            .filter(|n| n.category == category)
            .map(|n| n.id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// スコープに対応する mustselect カテゴリ名スライスを返す（内部ヘルパ）。
    fn mustselect(&self, scope: BindScope) -> &[String] {
        match scope {
            BindScope::Sakura => &self.sakura_mustselect,
            BindScope::Kero => &self.kero_mustselect,
        }
    }

    /// 当該スコープで `category` が `mustselect` と**宣言されているか**（転写所属照会・bindopt D5）。
    ///
    /// `sakura/kero.bindoption*.group` のオプション欄に `mustselect` が現れたカテゴリのみ真
    /// （`+` 区切り併記でも真）。`multiple` 単独宣言・非宣言は偽。答えるのは宣言の**所属**
    /// だけで、「排他か」の解釈（非宣言＝既定も着衣は排他置換）は下流 seriko の責務であり
    /// 本メソッドは持たない（parsers 転写層原則）。純関数（同一入力同一出力・副作用なし）。
    pub fn is_mustselect(&self, scope: BindScope, category: &str) -> bool {
        self.mustselect(scope).iter().any(|c| c == category)
    }

    /// スコープに対応する multiple カテゴリ名スライスを返す（内部ヘルパ）。
    fn multiple(&self, scope: BindScope) -> &[String] {
        match scope {
            BindScope::Sakura => &self.sakura_multiple,
            BindScope::Kero => &self.kero_multiple,
        }
    }

    /// 当該スコープで `category` が `multiple` と**宣言されているか**（転写所属照会・bindopt 1.1）。
    ///
    /// `is_mustselect` と対称。オプション欄に `multiple` が現れたカテゴリのみ真（`+` 区切り
    /// 併記でも真）。非宣言カテゴリは偽——偽は「既定（高々 1 個・解除可）」を意味し、
    /// 「複数可」ではない。純関数（同一入力同一出力・副作用なし）。
    pub fn is_multiple(&self, scope: BindScope, category: &str) -> bool {
        self.multiple(scope).iter().any(|c| c == category)
    }
}

/// 名前情報（各値は未指定なら None・推測しない）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GhostNames {
    /// descript `name`。
    pub name: Option<String>,
    /// descript `sakura.name`。
    pub sakura_name: Option<String>,
    /// descript `sakura.name2`（本体側の別名・`%selfname2` 由来・Req 4.4）。
    pub sakura_name2: Option<String>,
    /// descript `kero.name`。
    pub kero_name: Option<String>,
}

/// SHIORI マウント先。dir は起点定義の所在（= ghost/master）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShioriMount {
    /// ghost_root/ghost/master（存在確定済み・Req 2.1）。
    pub dir: PathBuf,
    /// descript `shiori,<file>`。未指定なら None（推測禁止・Req 2.3）。
    pub file: Option<String>,
}

/// shell マウント先。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellMount {
    /// ghost_root/shell/<dir>（既定 master・Req 3.1/3.2、存在確認済み・Req 3.3）。
    pub dir: PathBuf,
}

/// マウント解決の観測可能な失敗（致命）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MountError {
    /// ghost/master/descript.txt が存在しない（Req 1.6/5.1）。
    StartPointMissing { expected: PathBuf },
    /// descript.txt は所在するが読み取れなかった（I/O エラー・Req 1.1/5.1）。
    StartPointUnreadable {
        path: PathBuf,
        kind: std::io::ErrorKind,
    },
    /// 解決した shell ディレクトリが存在しない（Req 3.3/5.1）。
    ShellDirMissing { expected: PathBuf },
}

#[cfg(test)]
mod bindgroup_name_tests {
    use super::*;

    /// 空の名前表への (カテゴリ, パーツ) 問い合わせは None（捏造しない・パニックしない・R1.4）。
    #[test]
    fn resolve_name_on_empty_returns_none() {
        let defaults = BindGroupDefaults::default();
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), None);
        assert_eq!(defaults.resolve_name(BindScope::Kero, "腕", "伸び"), None);
    }

    /// 空の名前表への category_ids は空 Vec（パニックしない・R1.3）。
    #[test]
    fn category_ids_on_empty_returns_empty() {
        let defaults = BindGroupDefaults::default();
        assert!(defaults.category_ids(BindScope::Sakura, "腕").is_empty());
        assert!(defaults.category_ids(BindScope::Kero, "腕").is_empty());
    }

    fn name(id: u32, category: &str, part: &str) -> BindGroupName {
        BindGroupName {
            id,
            category: category.to_string(),
            part: part.to_string(),
            thumbnail: None,
        }
    }

    /// 宣言済み (カテゴリ, パーツ) は対応 ID へ解決する（R1.3）。
    #[test]
    fn resolve_name_returns_declared_id() {
        let defaults = BindGroupDefaults {
            sakura_names: vec![name(1100, "腕", "伸び"), name(1200, "頬", "赤面")],
            ..Default::default()
        };
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "頬", "赤面"),
            Some(1200)
        );
        // 未宣言の組は None（R1.4）。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "曲げ"), None);
    }

    /// sakura（本体）と kero（相方）は区別して解決される（R1.2）。
    #[test]
    fn resolve_name_distinguishes_scope() {
        let defaults = BindGroupDefaults {
            sakura_names: vec![name(1100, "腕", "伸び")],
            kero_names: vec![name(2100, "腕", "伸び")],
            ..Default::default()
        };
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Kero, "腕", "伸び"),
            Some(2100)
        );
        // sakura に宣言があっても kero 側では別集合として扱う。
        let sakura_only = BindGroupDefaults {
            sakura_names: vec![name(1100, "腕", "伸び")],
            ..Default::default()
        };
        assert_eq!(
            sakura_only.resolve_name(BindScope::Kero, "腕", "伸び"),
            None
        );
    }

    /// 重複 (カテゴリ, パーツ) はキー昇順走査の後勝ち（D2）。
    #[test]
    fn resolve_name_duplicate_last_wins() {
        let defaults = BindGroupDefaults {
            sakura_names: vec![name(1100, "腕", "伸び"), name(1300, "腕", "伸び")],
            ..Default::default()
        };
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1300)
        );
    }

    /// category_ids はカテゴリ所属 ID を昇順・重複排除で返す（R1.3 後段）。
    #[test]
    fn category_ids_ascending_deduped() {
        let defaults = BindGroupDefaults {
            sakura_names: vec![
                name(1300, "腕", "伸び"),
                name(1100, "腕", "曲げ"),
                name(1300, "腕", "伸び"), // 重複 ID
                name(1200, "腕", "組む"),
                name(9000, "頬", "赤面"), // 別カテゴリ（除外）
            ],
            ..Default::default()
        };
        assert_eq!(
            defaults.category_ids(BindScope::Sakura, "腕"),
            vec![1100, 1200, 1300]
        );
        assert_eq!(defaults.category_ids(BindScope::Sakura, "頬"), vec![9000]);
        assert!(defaults.category_ids(BindScope::Sakura, "脚").is_empty());
    }

    /// 空の宣言表への問い合わせは全カテゴリ偽＝全カテゴリ既定（bindopt 1.6）。
    ///
    /// 両照会が偽であることが「既定（高々 1 個・解除可）」の表現であり、
    /// bindoption 宣言ゼロの shell はこの形で成立する。
    #[test]
    fn is_mustselect_on_empty_returns_false() {
        let defaults = BindGroupDefaults::default();
        assert!(!defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(!defaults.is_mustselect(BindScope::Kero, "腕"));
        assert!(!defaults.is_multiple(BindScope::Sakura, "腕"));
        assert!(!defaults.is_multiple(BindScope::Kero, "腕"));
    }

    /// 宣言済みカテゴリのみ mustselect＝真、非宣言（既定）は偽（bindopt 1.2）。
    #[test]
    fn is_mustselect_returns_declared_only() {
        let defaults = BindGroupDefaults {
            sakura_mustselect: vec!["腕".to_string(), "目".to_string()],
            ..Default::default()
        };
        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "目"));
        // 非宣言カテゴリ（紅）は偽＝既定（高々 1 個・解除可）。
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
        // mustselect 宣言は multiple 所属を意味しない（2 集合は独立）。
        assert!(!defaults.is_multiple(BindScope::Sakura, "腕"));
    }

    /// mustselect は本体（sakura）／相方（kero）を区別する（bindopt 1.2・スコープ隔離）。
    #[test]
    fn is_mustselect_distinguishes_scope() {
        let defaults = BindGroupDefaults {
            sakura_mustselect: vec!["腕".to_string()],
            kero_mustselect: vec!["口".to_string()],
            ..Default::default()
        };
        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(!defaults.is_mustselect(BindScope::Kero, "腕"));
        assert!(defaults.is_mustselect(BindScope::Kero, "口"));
        assert!(!defaults.is_mustselect(BindScope::Sakura, "口"));
    }

    /// 宣言済みカテゴリのみ multiple（複数可）＝真、非宣言（既定）は偽（bindopt 1.1）。
    ///
    /// 「偽」は既定（高々 1 個・解除可）であって複数可ではない——この区別の成立が
    /// 本 spec の是正の核である。
    #[test]
    fn is_multiple_returns_declared_only() {
        let defaults = BindGroupDefaults {
            sakura_multiple: vec!["紅".to_string(), "髪飾り".to_string()],
            ..Default::default()
        };
        assert!(defaults.is_multiple(BindScope::Sakura, "紅"));
        assert!(defaults.is_multiple(BindScope::Sakura, "髪飾り"));
        // 非宣言カテゴリ（まばたき）は偽＝既定。
        assert!(!defaults.is_multiple(BindScope::Sakura, "まばたき"));
        // multiple 宣言は mustselect 所属を意味しない（2 集合は独立）。
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
    }

    /// multiple は本体（sakura）／相方（kero）を区別する（bindopt 1.1・スコープ隔離）。
    #[test]
    fn is_multiple_distinguishes_scope() {
        let defaults = BindGroupDefaults {
            sakura_multiple: vec!["紅".to_string()],
            kero_multiple: vec!["尻尾飾り".to_string()],
            ..Default::default()
        };
        assert!(defaults.is_multiple(BindScope::Sakura, "紅"));
        assert!(!defaults.is_multiple(BindScope::Kero, "紅"));
        assert!(defaults.is_multiple(BindScope::Kero, "尻尾飾り"));
        assert!(!defaults.is_multiple(BindScope::Sakura, "尻尾飾り"));
    }

    /// `mustselect+multiple` 併記は両集合へ所属し、両照会がともに真になる（bindopt 1.3）。
    ///
    /// 併記の情報を落とさないのが転写層の責務で、どちらを優先するか（bindopt D4）は
    /// 下流 seriko の解釈。
    #[test]
    fn both_options_declared_belongs_to_both_sets() {
        let defaults = BindGroupDefaults {
            sakura_mustselect: vec!["腕".to_string()],
            sakura_multiple: vec!["腕".to_string()],
            ..Default::default()
        };
        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_multiple(BindScope::Sakura, "腕"));
    }

    /// `#[non_exhaustive]`＋`Default` ゆえ multiple フィールド追加は additive——
    /// 既存フィールドのみを指定する構築が無傷で通り、追加分は空になる（bindopt 1.1）。
    #[test]
    fn multiple_fields_are_additive_to_existing_construction() {
        let defaults = BindGroupDefaults {
            sakura_default_on: vec![1100],
            sakura_names: vec![name(1100, "腕", "伸び")],
            sakura_mustselect: vec!["腕".to_string()],
            ..Default::default()
        };
        // 既存フィールドの意味は不変。
        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        // 追加フィールドは既定で空＝全カテゴリ「multiple 非宣言」。
        assert!(defaults.sakura_multiple.is_empty());
        assert!(defaults.kero_multiple.is_empty());
    }
}
