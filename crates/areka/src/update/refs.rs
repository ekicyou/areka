//! 更新のイベント名・Reference・番号・失敗理由・総括の写し（design「写し（`update/refs.rs`）」）。
//!
//! fs・記録・スレッドに触れない。渡された値だけから組む（記録を出すのは呼び手の手続き）。

use std::path::Path;

use areka_update::{FailReason, FetchError, Progress, UpdateOutcome};

use super::{SummaryKind, TargetKind, TargetSpec, UpdateReason};
use crate::install::judge::SEPARATOR;

// 送る 19 語（正典に在る語だけ・要件 2.15）。

// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateProcessExec:1
pub(crate) const ON_UPDATE_PROCESS_EXEC: &str = "OnUpdateProcessExec";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateBegin:1
pub(crate) const ON_UPDATE_BEGIN: &str = "OnUpdateBegin";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateReady:1
pub(crate) const ON_UPDATE_READY: &str = "OnUpdateReady";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnDownloadBegin:1
pub(crate) const ON_UPDATE_DOWNLOAD_BEGIN: &str = "OnUpdate.OnDownloadBegin";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnMD5CompareBegin:1
pub(crate) const ON_UPDATE_MD5_BEGIN: &str = "OnUpdate.OnMD5CompareBegin";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnMD5CompareComplete:1
pub(crate) const ON_UPDATE_MD5_COMPLETE: &str = "OnUpdate.OnMD5CompareComplete";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnMD5CompareFailure:1
pub(crate) const ON_UPDATE_MD5_FAILURE: &str = "OnUpdate.OnMD5CompareFailure";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateComplete:1
pub(crate) const ON_UPDATE_COMPLETE: &str = "OnUpdateComplete";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateFailure:1
pub(crate) const ON_UPDATE_FAILURE: &str = "OnUpdateFailure";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOtherBegin:1
pub(crate) const ON_UPDATE_OTHER_BEGIN: &str = "OnUpdateOtherBegin";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOtherReady:1
pub(crate) const ON_UPDATE_OTHER_READY: &str = "OnUpdateOtherReady";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOther.OnDownloadBegin:1
pub(crate) const ON_UPDATE_OTHER_DOWNLOAD_BEGIN: &str = "OnUpdateOther.OnDownloadBegin";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOther.OnMD5CompareBegin:1
pub(crate) const ON_UPDATE_OTHER_MD5_BEGIN: &str = "OnUpdateOther.OnMD5CompareBegin";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOther.OnMD5CompareComplete:1
pub(crate) const ON_UPDATE_OTHER_MD5_COMPLETE: &str = "OnUpdateOther.OnMD5CompareComplete";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOther.OnMD5CompareFailure:1
pub(crate) const ON_UPDATE_OTHER_MD5_FAILURE: &str = "OnUpdateOther.OnMD5CompareFailure";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOtherComplete:1
pub(crate) const ON_UPDATE_OTHER_COMPLETE: &str = "OnUpdateOtherComplete";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOtherFailure:1
pub(crate) const ON_UPDATE_OTHER_FAILURE: &str = "OnUpdateOtherFailure";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateResult:1
pub(crate) const ON_UPDATE_RESULT: &str = "OnUpdateResult";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateResultEx:1
pub(crate) const ON_UPDATE_RESULT_EX: &str = "OnUpdateResultEx";

/// 対象の種別ごとのイベント名の組（ゴースト＝`OnUpdate*`・シェル／バルーン＝`OnUpdateOther*`）。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EventNames {
    pub begin: &'static str,
    pub ready: &'static str,
    pub download_begin: &'static str,
    pub md5_begin: &'static str,
    pub md5_complete: &'static str,
    pub md5_failure: &'static str,
    pub complete: &'static str,
    pub failure: &'static str,
}

impl EventNames {
    /// 種別から組を選ぶ。
    pub(crate) fn for_kind(kind: TargetKind) -> &'static EventNames {
        match kind {
            TargetKind::Ghost => &GHOST_NAMES,
            TargetKind::Shell | TargetKind::Balloon => &OTHER_NAMES,
        }
    }
}

static GHOST_NAMES: EventNames = EventNames {
    begin: ON_UPDATE_BEGIN,
    ready: ON_UPDATE_READY,
    download_begin: ON_UPDATE_DOWNLOAD_BEGIN,
    md5_begin: ON_UPDATE_MD5_BEGIN,
    md5_complete: ON_UPDATE_MD5_COMPLETE,
    md5_failure: ON_UPDATE_MD5_FAILURE,
    complete: ON_UPDATE_COMPLETE,
    failure: ON_UPDATE_FAILURE,
};

static OTHER_NAMES: EventNames = EventNames {
    begin: ON_UPDATE_OTHER_BEGIN,
    ready: ON_UPDATE_OTHER_READY,
    download_begin: ON_UPDATE_OTHER_DOWNLOAD_BEGIN,
    md5_begin: ON_UPDATE_OTHER_MD5_BEGIN,
    md5_complete: ON_UPDATE_OTHER_MD5_COMPLETE,
    md5_failure: ON_UPDATE_OTHER_MD5_FAILURE,
    complete: ON_UPDATE_OTHER_COMPLETE,
    failure: ON_UPDATE_OTHER_FAILURE,
};

/// 番号の原点（`useorigin1` が `1` なら 1・それ以外と返事なしは 0）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Numbering {
    origin: usize,
}

impl Numbering {
    /// SHIORI リソース `useorigin1` の答えから読む。
    pub(crate) fn from_useorigin1(value: Option<&str>) -> Self {
        let origin = usize::from(value.map(str::trim) == Some("1"));
        Self { origin }
    }

    /// ファイルの番号（エンジンは 0 始まり）。
    fn index(self, index: usize) -> String {
        (index + self.origin).to_string()
    }

    /// 「件数から 1 引いた数」（1 始まりなら件数そのもの）。
    fn last(self, total: usize) -> String {
        (total + self.origin).saturating_sub(1).to_string()
    }
}

/// 対象の全イベントに同じ値で載せる Reference3（種別）・Reference4（理由）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tail {
    pub kind: &'static str,
    pub reason: &'static str,
}

impl Tail {
    pub(crate) fn new(kind: TargetKind, reason: UpdateReason) -> Self {
        Self {
            kind: kind.as_ref_str(),
            reason: reason.as_ref_str(),
        }
    }
}

/// 対象 1 つの終わり方（手続きが決め、総括の材料になる。総括の写しの入力なのでここに置く）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetEnd {
    /// 入れ替えた件数。
    Changed(usize),
    Unchanged,
    Failed {
        word: String,
        file: Option<String>,
    },
    /// 更新先が無い（イベント 0・総括に載せない）。
    Skipped,
    /// 途中でやめた（締めの知らせ 0）。
    Abandoned,
}

/// Reference0〜2 の後ろに種別と理由を足した 5 欄。
fn five(r0: String, r1: String, r2: String, tail: Tail) -> Vec<String> {
    vec![r0, r1, r2, tail.kind.to_owned(), tail.reason.to_owned()]
}

/// `OnUpdateProcessExec` の Reference（Reference0＝理由・メニューからだけ送る＝`manual`）。
pub(crate) fn process_exec_refs(reason: UpdateReason) -> Vec<String> {
    vec![reason.as_ref_str().to_owned()]
}

/// `OnUpdateBegin` の Reference（名前・フォルダ・空・種別・理由）。
pub(crate) fn begin_refs(name: &str, dir: &Path, tail: Tail) -> Vec<String> {
    five(
        name.to_owned(),
        dir.display().to_string(),
        String::new(),
        tail,
    )
}

/// 進捗 1 件を送るイベントの列へ写す（design「`Progress` からイベントへの写し」）。
pub(crate) fn progress_events(
    p: &Progress,
    names: &EventNames,
    n: Numbering,
    tail: Tail,
) -> Vec<(&'static str, Vec<String>)> {
    match p {
        Progress::DiffDecided { files } if !files.is_empty() => {
            let refs = five(n.last(files.len()), files.join(","), String::new(), tail);
            vec![(names.ready, refs)]
        }
        Progress::DownloadBegin { file, index, total } => {
            let refs = five(file.clone(), n.index(*index), n.last(*total), tail);
            vec![(names.download_begin, refs)]
        }
        Progress::Md5Compared {
            file,
            expected,
            actual,
            matched,
        } => {
            // 照合は結果だけが届く＝始まりと結果を続けて 2 件（要件 2.9）。
            let refs = five(file.clone(), expected.clone(), actual.clone(), tail);
            let end = if *matched {
                names.md5_complete
            } else {
                names.md5_failure
            };
            vec![(names.md5_begin, refs.clone()), (end, refs)]
        }
        // 差分 0 の締めは `OnUpdateComplete` の `none`。他は送らない（呼び手が `debug!`）。
        Progress::ManifestFetched { .. }
        | Progress::DiffDecided { .. }
        | Progress::Committed { .. }
        | Progress::Deleted { .. } => Vec::new(),
    }
}

/// `OnUpdateComplete` の Reference（`none`／`changed` と入れ替えた一覧）。
pub(crate) fn complete_refs(outcome: &UpdateOutcome, tail: Tail) -> Vec<String> {
    let (word, list) = match outcome {
        UpdateOutcome::Unchanged { .. } => ("none", String::new()),
        UpdateOutcome::Updated { placed, .. } => ("changed", placed.join(",")),
    };
    five(word.to_owned(), list, String::new(), tail)
}

/// `OnUpdateFailure` の Reference（理由の語・原因のファイル〔無ければ空〕）。
pub(crate) fn failure_refs(word: &str, file: Option<&str>, tail: Tail) -> Vec<String> {
    five(
        word.to_owned(),
        file.unwrap_or_default().to_owned(),
        String::new(),
        tail,
    )
}

/// 二重起動を断る `OnUpdateFailure` の Reference（理由 `executing`）。
pub(crate) fn executing_refs(kind: TargetKind, reason: UpdateReason) -> Vec<String> {
    failure_refs("executing", None, Tail::new(kind, reason))
}

/// 失敗理由の表（要件 4.2）。網羅の分岐＝種類が増えればビルドが止まる。
pub(crate) fn failure_word(reason: &FailReason) -> String {
    match reason {
        FailReason::InvalidHomeurl { .. } => "paramerror".to_owned(),
        FailReason::ManifestMissing {} => "404".to_owned(),
        FailReason::ManifestFetch { source, .. } | FailReason::FileFetch { source, .. } => {
            fetch_word(source)
        }
        FailReason::Md5Mismatch { .. } => "md5 miss".to_owned(),
        FailReason::TargetMissing { .. }
        | FailReason::LocalUnreadable { .. }
        | FailReason::WorkArea { .. }
        | FailReason::EscapesTarget { .. }
        | FailReason::CommitWrite { .. }
        | FailReason::RollbackFailed { .. } => "fileio".to_owned(),
    }
}

/// 取得の失敗の語。正典に語の無い輸送の失敗は areka の語（裁定 10）。
pub(crate) fn fetch_word(error: &FetchError) -> String {
    match error {
        FetchError::NotFound => "404".to_owned(),
        FetchError::Status { code } => code.to_string(),
        FetchError::Timeout => "timeout".to_owned(),
        FetchError::NameResolution => "dns".to_owned(),
        FetchError::Connect => "connect".to_owned(),
        FetchError::Tls => "tls".to_owned(),
        FetchError::TooLarge { .. } => "toolarge".to_owned(),
        FetchError::Other { .. } => "http".to_owned(),
    }
}

/// 総括の Reference（実行した順に対象 1 つにつき 1 つ）。飛ばした対象・途中でやめた対象は載せない。
pub(crate) fn summary_refs(kind: SummaryKind, ends: &[(&TargetSpec, &TargetEnd)]) -> Vec<String> {
    ends.iter()
        .filter_map(|(spec, end)| {
            let mut fields = match kind {
                SummaryKind::Result => Vec::new(),
                SummaryKind::ResultEx => vec![spec.name.clone()],
            };
            fields.push(spec.kind.as_ref_str().to_owned());
            match end {
                TargetEnd::Changed(count) => fields.extend(["OK".to_owned(), count.to_string()]),
                TargetEnd::Unchanged => fields.extend(["OK".to_owned(), "0".to_owned()]),
                TargetEnd::Failed { word, file } => {
                    fields.extend(["NG".to_owned(), word.clone()]);
                    fields.extend(file.clone());
                }
                TargetEnd::Skipped | TargetEnd::Abandoned => return None,
            }
            Some(fields.join(SEPARATOR))
        })
        .collect()
}

#[cfg(test)]
#[path = "refs_tests.rs"]
mod tests;
