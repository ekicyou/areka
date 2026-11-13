//! クレートのテスト全体の支え: 時刻の定数・一時の置き場所・フォルダの中身と状態ファイルの読み取り・
//! 机の持ち主の組み立て。
//! 各テストは `crate::test_support` から直に読む（モジュールごとの支えで出し直さない）。

use std::fs;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use crate::home::Home;
use crate::state::{MergeHolder, SoleHolder, State, TestHolder};

/// テストの「いま」の起点（UNIX 秒）。UTC では 2026-10-03T04:00:00Z。後の時刻は `T0 + n` と書く。
pub(crate) const T0: u64 = 1_791_000_000;

/// フォルダの直下に在るものの一覧（並べ替え済み）。
pub(crate) fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| entry.expect("項目を読める").path())
        .collect();
    found.sort();
    found
}

/// フォルダの直下に在るものの名前（並べ替え済み）。
pub(crate) fn names(dir: &Path) -> Vec<String> {
    let name = |path: PathBuf| {
        path.file_name()
            .expect("名前が在る")
            .to_string_lossy()
            .into_owned()
    };
    entries(dir).into_iter().map(name).collect()
}

/// 一時フォルダを置き場所にした `Home`（フォルダは在る・中身はまだ無い）。
pub(crate) fn home_in(root: &TempPath) -> Home {
    Home {
        dir: root.path().to_path_buf(),
    }
}

/// `dir` の `state.json` を状態として読む（無い・読めないなら panic）。
pub(crate) fn read_state(dir: &Path) -> State {
    let bytes = fs::read(dir.join("state.json")).expect("状態ファイルが在る");
    serde_json::from_slice(&bytes).expect("状態ファイルが状態として読める")
}

/// `requested` に申し込み、`granted` に番の来たマージの持ち主（spec は `spec-<識別>`・バグでない）。
pub(crate) fn merge_holder(id: &str, requested: u64, granted: u64) -> Option<MergeHolder> {
    Some(MergeHolder {
        id: id.to_owned(),
        spec: format!("spec-{id}"),
        bug: false,
        requested,
        granted,
    })
}

/// 内容が `purpose` の、`requested` に申し込み `granted` に番の来た排他テストの机の持ち主。
pub(crate) fn test_holder(
    id: &str,
    purpose: &str,
    requested: u64,
    granted: u64,
) -> Option<TestHolder> {
    Some(TestHolder {
        id: id.to_owned(),
        purpose: purpose.to_owned(),
        requested,
        granted,
    })
}

/// 内容が `purpose` の、`requested` に申し込み `granted` に番の来たひとり占めの持ち主。
/// `running` は走っている印、`stopped` は番が来たときに止まっていた識別。
pub(crate) fn sole_holder(
    id: &str,
    purpose: &str,
    requested: u64,
    granted: u64,
    running: bool,
    stopped: &[&str],
) -> Option<SoleHolder> {
    Some(SoleHolder {
        id: id.to_owned(),
        purpose: purpose.to_owned(),
        requested,
        granted,
        running,
        stopped: stopped.iter().map(|id| (*id).to_owned()).collect(),
    })
}
