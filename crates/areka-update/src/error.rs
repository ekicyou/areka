//! 失敗の閉じた語彙・段・取得の失敗・警告（要件 7.3・7.4・8.2）。
//!
//! ここではログを出さない。記録は `lib.rs` が `Err` を返す直前に 1 回だけ出す。
//! そのため各変種の表示は、その 1 行だけで診断できる情報（どのファイルか・
//! 期待と実際）を落とさずに持つ。

use crate::outcome::ManifestName;
use crate::paths::WORK_DIR;
use std::path::{Path, PathBuf};

/// 失敗の語彙を 1 か所の宣言から組み立てる（`areka-nar` の `refuse_reasons!` と同じ形）。
///
/// `kind()` の分岐と `ALL_KINDS` の並びを同じ宣言から展開するので食い違いが起こり得ない。
macro_rules! fail_reasons {
    ($(
        $(#[$variant_attr:meta])*
        $variant:ident { $( $(#[$field_attr:meta])* $field:ident : $ty:ty ),* $(,)? }
    ),* $(,)?) => {
        /// 一周を失敗にした理由。11 変種で閉じる（7.4 の 10 に `EscapesTarget` を足した）。
        ///
        /// `std::io::Error` を持つので `Clone`／`PartialEq` は導出しない。比べるときは `kind()`。
        #[derive(Debug, thiserror::Error)]
        pub enum FailReason {
            $(
                $(#[$variant_attr])*
                $variant { $( $(#[$field_attr])* $field : $ty ),* }
            ),*
        }

        impl FailReason {
            /// 変種名（`OnUpdateFailure` の理由に写す短い語）。ASCII・安定。
            pub fn kind(&self) -> &'static str {
                match self {
                    $( FailReason::$variant { .. } => stringify!($variant) ),*
                }
            }

            /// 失敗の語彙の全数。宣言順に並ぶ。
            pub const ALL_KINDS: &'static [&'static str] = &[ $( stringify!($variant) ),* ];
        }
    };
}

/// どの段まで進んだか（7.3）。削除の段は一周を失敗にしないので（6.6）ここに無い。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Entry,
    Manifest,
    Diff,
    Download { index: usize, total: usize },
    Verify { index: usize, total: usize },
    Commit,
}

/// 取得の境界が返す失敗（閉じた語彙・`Clone`＋`PartialEq`＝偽の取得口の固定表に置ける）。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    /// 404。
    #[error("見つからない（404）")]
    NotFound,
    /// 2xx・3xx・404 以外の状態コード。
    #[error("状態コード {code}")]
    Status { code: u16 },
    /// `ERROR_WINHTTP_NAME_NOT_RESOLVED`。
    #[error("名前解決に失敗")]
    NameResolution,
    /// `ERROR_WINHTTP_CANNOT_CONNECT`／`CONNECTION_ERROR`。
    #[error("接続に失敗")]
    Connect,
    /// `ERROR_WINHTTP_TIMEOUT`。
    #[error("時間切れ")]
    Timeout,
    /// `ERROR_WINHTTP_SECURE_FAILURE`（`_PROXY` を含む）。
    #[error("TLS に失敗")]
    Tls,
    /// 本文が上限を超えた。
    #[error("本文が上限 {limit} バイトを超えた")]
    TooLarge { limit: usize },
    /// 上記以外の Win32 エラー番号。
    #[error("その他の失敗（エラー番号 {code}）")]
    Other { code: u32 },
}

fail_reasons! {
    /// 対象フォルダが無い・フォルダでない。
    #[error("対象フォルダが無い: {path}")]
    TargetMissing { path: PathBuf },

    /// 更新先 URL が http／https で始まらない。
    #[error("更新先 URL が不正: {homeurl}")]
    InvalidHomeurl { homeurl: String },

    /// updates2.dau も updates.txt も 404。
    #[error("定義ファイルが無い（updates2.dau・updates.txt とも見つからない）")]
    ManifestMissing {},

    /// 404 以外の理由で定義ファイルが取れない。
    #[error("定義ファイル {} の取得に失敗: {source}", name.file_name())]
    ManifestFetch { name: ManifestName, source: FetchError },

    /// 在るのに読めない（4 GiB 超を含む）。
    #[error("ローカルのファイルが読めない: {path}: {source}")]
    LocalUnreadable { path: PathBuf, source: std::io::Error },

    /// 作業場所を作れない・書けない。
    #[error("作業場所を作れない・書けない: {path}: {source}")]
    WorkArea { path: PathBuf, source: std::io::Error },

    /// 1 件の取得失敗。
    #[error("ファイル {file} の取得に失敗: {source}")]
    FileFetch { file: String, source: FetchError },

    /// 落としたバイト列の MD5 が定義と合わない。
    #[error("ファイル {file} の MD5 が一致しない（期待 {expected}・実際 {actual}）")]
    Md5Mismatch { file: String, expected: String, actual: String },

    /// 実パスが対象フォルダの外へ解決される。
    #[error("実パスが対象フォルダの外へ出る: {path}")]
    EscapesTarget { path: PathBuf },

    /// 確定で置けない（戻せた）。
    #[error("確定で書けない（元へ戻した）: {path}: {source}")]
    CommitWrite { path: PathBuf, source: std::io::Error },

    /// 戻せなかった。`path`／`source` は確定を止めた失敗。表示は両方の一覧を名前で持つ
    /// （error の記録の `detail` がこの表示＝5.5）。
    #[error("確定で書けず、戻せなかった: {path}: {source}（戻せた {restored:?}・戻せなかった {stuck:?}）")]
    RollbackFailed {
        path: PathBuf,
        source: std::io::Error,
        restored: Vec<String>,
        stuck: Vec<Stuck>,
    },
}

/// 戻せなかったファイル 1 件。
#[derive(Debug)]
pub struct Stuck {
    pub file: String,
    pub source: std::io::Error,
}

/// 一周の失敗（7.3）。
#[derive(Debug, thiserror::Error)]
#[error("{homeurl} → {target}: {stage:?} で失敗: {reason}")]
pub struct UpdateError {
    pub homeurl: String,
    pub target: PathBuf,
    pub stage: Stage,
    pub reason: FailReason,
    pub leftovers: Vec<PathBuf>,
    /// 戻せなかったときだけ。元の内容が残る作業場所。
    pub work: Option<PathBuf>,
    // 要取得の一覧は持たない: 差分の段を越えた失敗なら `Progress::DiffDecided` で既に渡している（2.6）。
}

impl UpdateError {
    /// 原因のファイル名（分かるとき）。名前空間は 1 つ＝対象フォルダからの相対名
    /// （定義ファイルが書く形。ゴーストへ渡す `OnUpdateFailure` の Ref1 に写すので、
    /// 利用者の絶対パスは出さない）。
    ///
    /// - 名前を持つ理由（`ManifestFetch`・`FileFetch`・`Md5Mismatch`）はその名前。
    /// - パスを持つ理由（`LocalUnreadable`・`EscapesTarget`・`CommitWrite`・`RollbackFailed`）は
    ///   `path` から `target` を剥がした残り。残りが空（対象フォルダそのもの）・`target` の
    ///   配下でない・UTF-8 でない、のときは `None`。相対名を取り戻せるよう、これらの
    ///   `path` は `target_real` からでなく `target.join(rel)` で組むこと。
    /// - `WorkArea` は落とした物の書き先（`<target>/.update-work/<走行>/new/<rel>`）なら `<rel>`
    ///   （作業場所の `new/` は対象フォルダと同じ相対名で並ぶ）。作業場所そのものなら `None`。
    /// - フォルダ単位の理由（`TargetMissing`・`InvalidHomeurl`・`ManifestMissing`）は `None`。
    pub fn file(&self) -> Option<&str> {
        fn rel<'a>(path: &'a Path, base: &Path) -> Option<&'a str> {
            path.strip_prefix(base)
                .ok()
                .and_then(|rel| rel.to_str())
                .filter(|rel| !rel.is_empty())
        }
        match &self.reason {
            FailReason::ManifestFetch { name, .. } => Some(name.file_name()),
            FailReason::FileFetch { file, .. } | FailReason::Md5Mismatch { file, .. } => Some(file),
            FailReason::LocalUnreadable { path, .. }
            | FailReason::EscapesTarget { path }
            | FailReason::CommitWrite { path, .. }
            | FailReason::RollbackFailed { path, .. } => rel(path, &self.target),
            FailReason::WorkArea { path, .. } => {
                let mut run = path
                    .strip_prefix(self.target.join(WORK_DIR))
                    .ok()?
                    .components();
                run.next()?;
                rel(run.as_path(), Path::new("new"))
            }
            FailReason::TargetMissing { .. }
            | FailReason::InvalidHomeurl { .. }
            | FailReason::ManifestMissing {} => None,
        }
    }

    /// `RollbackFailed` 以外は真。
    pub fn rolled_back(&self) -> bool {
        !matches!(self.reason, FailReason::RollbackFailed { .. })
    }
}

/// 拒否ではないが黙って通さないもの（8.2）。`run` が 1 件 1 行の `warn!` に写す。
/// 取り除けなかった物と残骸は結果の一覧（`undeletable`・`leftovers`）が正本で、
/// `run` がそこから直接 `warn!` に写す（`io::Error` は `Clone` できないので 2 か所に持たない）。
#[derive(Debug)]
pub enum UpdateWarning {
    HomeurlSlashAppended,
    UnknownCharset {
        name: String,
    },
    InvalidEntry {
        line: usize,
        why: InvalidWhy,
    },
    DuplicateEntry {
        line: usize,
        path: String,
    },
    DeleteLineIgnored {
        file: String,
        line: usize,
        why: DeleteWhy,
    },
    /// 行の種別（ファイル／フォルダ）と実体の種別が違う → 取り除かない（6.4・両方向）。
    DeleteKindMismatch {
        file: String,
        line: usize,
        path: PathBuf,
    },
    /// `delete.txt`／`delete<N>.txt` が読めない → そのファイルを飛ばす（6.6）。
    DeleteFileUnreadable {
        file: String,
        source: std::io::Error,
    },
}

/// 定義ファイルのエントリを無効として捨てた理由（1.11・1.12）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidWhy {
    NoMd5,
    BadMd5,
    FolderEntry,
    DotDot,
    Absolute,
    EmptyComponent,
    Nul,
    SelfReference,
    InsideWorkArea,
}

/// `delete.txt` の行を無視した理由（6.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteWhy {
    Absolute,
    DotDot,
    EscapesTarget,
    InsideWorkArea,
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
