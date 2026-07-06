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
    /// shell descript の bindgroup default 転記（Req 4.5・既存 3 フィールドと非衝突）。
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
}

/// 名前情報（各値は未指定なら None・推測しない）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GhostNames {
    /// descript `name`。
    pub name: Option<String>,
    /// descript `sakura.name`。
    pub sakura_name: Option<String>,
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
