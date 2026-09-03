//! `crates/**/*.rs` の列挙と読み込み（`crates/ukadoc-survey/` は除く・設計 D-3）。
//!
//! 証拠の取り出し（要件 5）が読む相手を揃えるのがこの層の仕事で、判断は持たない。
//! 返すのは「ワークスペース根からの相対パスと本文」の対の並びだけである。
//!
//! # 走査部品を自前で持つ理由（設計 D-3）
//!
//! ほぼ同じ列挙が `crates/log-capture-kit/tests/workspace_scan/mod.rs` にあるが、
//! あちらは `tests/` 配下なので他クレートから引けない。共有クレートへ出すには既存
//! クレートのテストを書き換えることになり、要件 9.1（既存クレート非接触）を越える。
//! 40 行程度の重複を受け入れる。3 か所目の走査が現れたとき、または `log-capture-kit`
//! が走査部品を `src/` へ出したときに見直す。
//!
//! # 調査クレート自身を除く理由（設計 D-3）
//!
//! この道具は areka の実装ではないので正典 URL の証拠を持つことがない。それどころか
//! 見本データには ukadoc の URL を書いた文字列が並ぶので、走査に入れると**見本が
//! 本物の証拠として読まれる**。除外は綴りの問題ではなく、判定の正しさの問題である。
//!
//! # 返すパスの形
//!
//! ワークスペース根からの相対で、区切りは `/` に揃える。環境によって報告の本文が
//! 変わらないようにするためで（設計「入出力層」）、相対パスは走査の途中で組み立てる
//! ——後から区切りを置き換えるのではなく、はじめから `/` で綴る。

use std::path::Path;

use crate::error::SurveyError;
use crate::io::files;

/// 列挙から外すディレクトリ名。生成物・外部取り込み・版管理
/// （`crates/log-capture-kit/tests/workspace_scan/mod.rs:41` と同じ）。
const EXCLUDED_DIRS: &[&str] = &["target", "vendors", ".git"];

/// 走査の起点（ワークスペース根からの相対）。
const SOURCE_ROOT: &str = "crates";

/// 走査から外す調査クレート自身（ワークスペース根からの相対・設計 D-3）。
const SELF_CRATE_DIR: &str = "crates/ukadoc-survey";

/// `crates/**/*.rs` を名前順・重複なしで返す。`crates/ukadoc-survey/` は除く。
///
/// 返すのは `(ワークスペース根からの相対パス, 本文)` の対。本文は
/// [`files::read_normalized`] を通しているので復帰文字を含まない。
///
/// 本番（`src/`）・テスト（`tests/`）・実行例（`examples/`）・本番の隣に置いた兄弟
/// テストファイルをすべて含む。列挙できないディレクトリや読めないファイルに出会ったら、
/// そのパスを載せて失敗する（黙って飛ばさない・要件 6.12）。
pub fn walk(root: &Path) -> Result<Vec<(String, String)>, SurveyError> {
    let mut found = Vec::new();
    collect(&root.join(SOURCE_ROOT), SOURCE_ROOT, &mut found)?;
    found.sort();
    found.dedup();

    let mut out = Vec::with_capacity(found.len());
    for rel in found {
        let body = files::read_normalized(&root.join(&rel))?;
        out.push((rel, body));
    }
    Ok(out)
}

/// `dir`（根からの相対が `rel`）の下の `.rs` を集める。
fn collect(dir: &Path, rel: &str, out: &mut Vec<String>) -> Result<(), SurveyError> {
    let entries = std::fs::read_dir(dir).map_err(|err| files::io_error(dir, &err.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|err| files::io_error(dir, &err.to_string()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| files::io_error(&path, &err.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // 相対パスははじめから `/` で綴る（後から区切りを置き換えない）。
        let child = format!("{rel}/{name}");
        if file_type.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_str()) || child == SELF_CRATE_DIR {
                continue;
            }
            collect(&path, &child, out)?;
        } else if file_type.is_file() && name.ends_with(".rs") {
            out.push(child);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;
