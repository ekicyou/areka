//! 成功の結果と進捗の型（要件 7.1・7.2・7.6）。

use std::path::PathBuf;

/// 取得した定義ファイルの名前。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifestName {
    Updates2Dau,
    UpdatesTxt,
}

impl ManifestName {
    /// `"updates2.dau"` または `"updates.txt"`。
    pub fn file_name(self) -> &'static str {
        match self {
            ManifestName::Updates2Dau => "updates2.dau",
            ManifestName::UpdatesTxt => "updates.txt",
        }
    }
}

/// 起きた順に観測者へ渡す進捗。番号は 0 始まり（7.6）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    ManifestFetched {
        name: ManifestName,
    },
    DiffDecided {
        files: Vec<String>,
    },
    DownloadBegin {
        file: String,
        index: usize,
        total: usize,
    },
    Md5Compared {
        file: String,
        expected: String,
        actual: String,
        matched: bool,
    },
    Committed {
        placed: Vec<String>,
    },
    Deleted {
        removed: Vec<PathBuf>,
    },
}

/// 成功の 2 形（7.2）。
#[derive(Debug)]
pub enum UpdateOutcome {
    /// 差分 0。対象フォルダには何も書いていない。
    Unchanged { manifest: ManifestName },
    Updated {
        manifest: ManifestName,
        /// 置いたファイル（定義の順・定義ファイル自身は含めない＝`manifest` 欄が示す）。
        placed: Vec<String>,
        removed: Vec<PathBuf>,
        undeletable: Vec<Undeletable>,
        leftovers: Vec<PathBuf>,
    },
}

/// `delete.txt` の行が指すのに取り除けなかった物（6.6）。
#[derive(Debug)]
pub struct Undeletable {
    pub path: PathBuf,
    pub source: std::io::Error,
}
