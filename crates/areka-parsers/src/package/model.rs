//! model — マウントモデル・失敗型の正本（型の正本）。
//!
//! 解決済みマウント所在（`MountModel` と付随値型 `GhostNames` / `ShioriMount` /
//! `ShellMount`）と、マウント解決の観測可能な失敗（`MountError`）を定義する。
//! この型群は下流（`ghost-setup` / `host-32` / `shell-parse`）と共有する
//! I/O 契約の片側であり、本 spec が生成者・正本を所有する。
//!
//! 設計規律（design.md「型定義（model）」）:
//! - 純粋な型定義に留める（I/O・`Result` は `resolve_shell` サブモジュールが持つ）。
//! - 派生は `Clone` / `Debug` / `PartialEq` / `Eq`（文字列/パスのみで
//!   `f32`/`Duration` を含まないため `Eq` 付与可・`sakura::Instruction` との差異）。
//!   `serde` は付さない（他兄弟型と整合・不要）。
//! - `#[non_exhaustive]` により後続のフィールド/variant 追加を後方互換に保つ。
//! - 名前情報・SHIORI ファイル名は `Option`（欠落を型で表現・推測しない・Req 2.3）。
//!   パス表現は `PathBuf`。
//!
//! 不変条件（design.md「Preconditions/Postconditions/Invariants」）:
//! - `MountModel` は `resolve_with_shell` 成功時のみ構築される。
//! - `shiori.dir` は起点 descript.txt の親（物理存在確定）、`shell.dir` は
//!   物理存在確認済み。
//! - `shiori.file` / `names.*` は `Option` で欠落を保持し、既定値は推測しない
//!   （`shell.dir` の `master` フォールバックのみ ukadoc 既定で例外）。

use std::collections::BTreeMap;
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
    /// descript `readme,<ファイル名>`。未指定なら None（推測禁止・Req 4.1）。
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#readme_2c_30d5_30a1_30a4_30eb_540d:1
    pub readme: Option<String>,
}

/// shell descript.txt の着せ替えの読み（bindgroup・bindoption の転記）をスコープ番号で引く表。
///
/// 鍵はスコープ番号（0＝`sakura.*`・1＝`kero.*`・2 以上＝`char{N}.*`）。書かれていない
/// スコープは表に無い。**転記のみ・展開しない**（範囲展開や surface 解決は行わない・
/// parsers 転写層原則）。引き方（同じ名前の後勝ち・3 値ポリシー）は下流 seriko が持つ。
pub type BindGroupDefaults = BTreeMap<u32, ScopeBinds>;

/// 1 つのスコープの着せ替えの読み（ukadoc カテゴリ `descript_shell`）。保持はどれも転記順。
// seriko のテストが組むので `#[non_exhaustive]` を付けない。
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ScopeBinds {
    /// `bindgroup*.default,1` の bindgroup 番号（昇順不問・保持は転記順）。
    pub default_on: Vec<u32>,
    /// `bindgroup*.name,カテゴリ名,パーツ名,…` の名前宣言転記（Req 1.1・1.2）。
    pub names: Vec<BindGroupName>,
    /// `bindoption*.group` のオプション欄に `mustselect` が現れたカテゴリ名（bindopt 1.2）。
    ///
    /// ukadoc 正典の 3 値（`mustselect`＝ちょうど 1 個・解除不可／非宣言＝既定で高々 1 個・
    /// 解除可／`multiple`＝複数可）のうち宣言の**所属**のみを写す。非宣言カテゴリは
    /// `mustselect`・`multiple` のどちらにも入らない＝下流が正典の既定として扱う。
    /// `+` 区切り併記（`mustselect+multiple`）は両方へ転記して情報を落とさない。
    pub mustselect: Vec<String>,
    /// `bindoption*.group` のオプション欄に `multiple` が現れたカテゴリ名（bindopt 1.1）。
    pub multiple: Vec<String>,
}

/// bindgroup 名前宣言 1 件の忠実転記（不透明文字列・ID 非生成・Req 1.5）。
///
/// `<prefix>NNNN.name,カテゴリ名,パーツ名,…` の 1 行を、bindgroup 番号 `NNNN`
/// （= 着せ替え/animation ID・恒等）とカテゴリ名・パーツ名・残余サムネイル名として
/// 忠実転記する。名前は不透明な文字列として保持し、宣言されていない着せ替え ID を
/// 新たに生成しない（Req 1.5・parsers 転写層原則）。
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
    /// この名前を決めた shell descript.txt の行の番号（1 始まり・同じキーは後の行）。
    pub line: usize,
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
    /// descript `shiori.encoding,<ラベル>` の生の値（解釈しない・charset-canon 2.5/8.5）。
    ///
    /// ラベルの解決（前後空白・大小文字・別名）は通信層の `Charset::for_label` の
    /// 責務であり、この層は `parse_kv` が返した文字列をそのまま持つ。
    pub encoding: Option<String>,
    /// descript `shiori.forceencoding,<ラベル>` の生の値（解釈しない・charset-canon 2.5/8.5）。
    pub force_encoding: Option<String>,
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
