//! ワークスペース根と `doc/ukadoc-coverage/` 配下の場所（設計「入出力層」）。
//!
//! 判断は持たない。どの関数も引数（あればドメイン）だけから場所を組み立てて返す。
//! ファイルが実在するかどうかは見ない——生成の前は無いのが普通だからである。
//!
//! # 根の求め方
//!
//! 根は **`CARGO_MANIFEST_DIR` の 2 段上**。この crate の manifest は
//! `crates/ukadoc-survey/` にあるので、2 段上がワークスペース根になる
//! （前例: `crates/log-capture-kit/tests/workspace_scan/mod.rs:64`）。
//!
//! ここで使うのは**コンパイル時に埋め込まれる `env!`** であって、実行時の環境変数
//! ではない。実行時に読むと、テストがどのディレクトリから起動されたかで答えが変わり、
//! 常時走るテストが決定的でなくなる（要件 6.1）。実行時の環境変数はこの層では
//! 1 つも読まない（要件 9.7 が定める `AREKA_` 冠の変数を使うのはスナップショットの
//! 場所を決める `io::snapshot` だけである）。
//!
//! # 場所の綴り
//!
//! 台帳と報告のファイル名はドメインの綴り（[`Domain::as_key`]）をそのまま使う。
//! ここで綴り直さないのは、`sakura-script` の横棒のような 1 文字の違いが
//! 別の場所を指してしまうからである。

use std::path::{Path, PathBuf};

use crate::model::Domain;

/// この crate の manifest のあるディレクトリ（コンパイル時に決まる）。
const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// カタログ・台帳・報告を置くディレクトリの名前（ワークスペース根からの相対）。
const COVERAGE_DIR: &str = "doc/ukadoc-coverage";

/// ワークスペース根の絶対パス。
///
/// `crates/<クレート>/` の 2 段上。2 段上が取れないのは manifest の置き場が
/// `crates/` の下から動いたときだけで、それはこの crate の構造そのものの破壊なので
/// 起動時に止める（要件 9.6 の配置が前提）。
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(MANIFEST_DIR);
    match manifest.parent().and_then(Path::parent) {
        Some(root) => root.to_path_buf(),
        // コンパイル時に決まる値なので、ここに来るのは crate を `crates/` の外へ
        // 移した場合だけ。黙って別の場所を指すより、どこを見ていたかを告げて止める。
        None => panic!("crates/<クレート>/ の 2 段上が取れない: {MANIFEST_DIR}"),
    }
}

/// カタログ・台帳・報告を置くディレクトリ（`doc/ukadoc-coverage`）。
pub fn coverage_dir() -> PathBuf {
    workspace_root().join(COVERAGE_DIR)
}

/// 機械生成のカタログ（`doc/ukadoc-coverage/catalog.toml`）。
pub fn catalog_path() -> PathBuf {
    coverage_dir().join("catalog.toml")
}

/// ドメインの台帳（`doc/ukadoc-coverage/ledger/<ドメイン>.toml`）。
pub fn ledger_path(domain: Domain) -> PathBuf {
    coverage_dir()
        .join("ledger")
        .join(format!("{}.toml", domain.as_key()))
}

/// ドメイン別報告（`doc/ukadoc-coverage/report/<ドメイン>.md`）。要件 7.4 の突き合わせ対象。
pub fn domain_report_path(domain: Domain) -> PathBuf {
    coverage_dir()
        .join("report")
        .join(format!("{}.md", domain.as_key()))
}

/// 全体報告（`doc/ukadoc-coverage/report/summary.md`）。常時検査の対象外（要件 7.6）。
pub fn summary_report_path() -> PathBuf {
    coverage_dir().join("report").join("summary.md")
}

/// 伺からしさのテーマ定義（`doc/ukadoc-coverage/values.md`）。要件 4.4 の 8 テーマの正本。
pub fn values_path() -> PathBuf {
    coverage_dir().join("values.md")
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
