//! インストールの手続きの判断の分かれ目（design「areka / install / judge」）。
//!
//! fs も World も読まない。`accept` の照合・宛先の種類・失敗理由の語・イベントの Reference の
//! 組み立て・台本の引数の検査を、渡された値だけから決める。記録も出さない（出すのは呼び手）。

use std::path::{Path, PathBuf};

use areka_nar::{ElementKind, InstallKind, InstallManifest, NarError, RefuseReason};

/// 複数の値の区切り（byte 値 1）。
pub(crate) const SEPARATOR: &str = "\u{1}";

/// 照合と宛先の判断に要る、今のゴーストの素性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GhostFacts {
    pub root: PathBuf,
    /// 根の下のフォルダ名（argv で根の外から起こしたゴーストは None）。
    pub folder: Option<String>,
    pub name: String,
    pub sakura_name: Option<String>,
    pub install_accept: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AcceptVerdict {
    /// 受け取る。`shell`／`supplement` は宛先のゴーストのフォルダ名つき。
    Accepted { target_ghost: Option<String> },
    /// 宛先違い（`accept` の値つき）。
    Refused { accept: String },
    /// `accept` の無い `shell`／`supplement`。
    AcceptMissing,
}

/// `install.txt` の `accept` を、起動中のゴーストの `sakura.name` と `install.accept` の各名前に
/// 完全一致（大文字小文字を区別）で照合する（要件 3.1・3.4・3.5・3.6）。
pub(crate) fn judge_accept(manifest: &InstallManifest, ghost: &GhostFacts) -> AcceptVerdict {
    let named = matches!(manifest.kind, InstallKind::Shell | InstallKind::Supplement);
    let Some(accept) = &manifest.accept else {
        return if named {
            AcceptVerdict::AcceptMissing
        } else {
            AcceptVerdict::Accepted { target_ghost: None }
        };
    };
    let matched = ghost.sakura_name.as_deref() == Some(accept.as_str())
        || ghost.install_accept.iter().any(|name| name == accept);
    if !matched {
        return AcceptVerdict::Refused {
            accept: accept.clone(),
        };
    }
    AcceptVerdict::Accepted {
        target_ghost: if named { ghost.folder.clone() } else { None },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Destination {
    /// 起動中のゴーストのフォルダそのもの（`ghost` の上書き・`supplement`）。
    RunningGhost,
    Elsewhere,
}

/// 宛先が起動中のゴーストのフォルダそのものか（要件 7.1・7.8）。
pub(crate) fn destination_of(manifest: &InstallManifest, ghost: &GhostFacts) -> Destination {
    let Some(folder) = &ghost.folder else {
        // argv で根の外から起こしたゴーストは根の下に居ない＝降ろして入れる一周へは入らない。
        return Destination::Elsewhere;
    };
    match manifest.kind {
        InstallKind::Supplement => Destination::RunningGhost,
        InstallKind::Ghost if manifest.directory.eq_ignore_ascii_case(folder) => {
            Destination::RunningGhost
        }
        InstallKind::Ghost | InstallKind::Shell | InstallKind::Balloon => Destination::Elsewhere,
    }
}

/// 正典の失敗理由の語（3 値だけ・要件 5.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureWord {
    Extraction,
    InvalidType,
    Unsupported,
}

impl FailureWord {
    /// `OnInstallFailure` の Reference0 に載せる語。
    pub(crate) fn as_ref_str(self) -> &'static str {
        match self {
            FailureWord::Extraction => "extraction",
            FailureWord::InvalidType => "invalid type",
            FailureWord::Unsupported => "unsupported",
        }
    }
}

/// `areka-nar` の失敗を正典の失敗理由の語へ写す（要件 5.2 の表）。
///
/// ワイルドカードの腕を置かない＝`areka-nar` の拒否の種類が増えたらここでビルドが止まる。
pub(crate) fn failure_word(error: &NarError) -> FailureWord {
    let reason = match error {
        NarError::Io { .. } => return FailureWord::Extraction,
        NarError::Refused { reason, .. } => reason,
    };
    match reason {
        RefuseReason::CorruptArchive { .. }
        | RefuseReason::IntegrityMismatch { .. }
        | RefuseReason::NameUndecodable { .. } => FailureWord::Extraction,
        RefuseReason::MissingInstallTxt { .. }
        | RefuseReason::UnsupportedType { found: None }
        | RefuseReason::MissingRequiredKey { .. }
        | RefuseReason::InvalidDirectoryName { .. }
        | RefuseReason::CompanionSourceMissing { .. }
        | RefuseReason::TargetGhostMissing { .. } => FailureWord::InvalidType,
        RefuseReason::UnsupportedEntry { .. }
        | RefuseReason::UnsupportedType { found: Some(_) }
        | RefuseReason::SymlinkEntry { .. }
        | RefuseReason::UnsafePath { .. }
        | RefuseReason::PathTooLong { .. }
        | RefuseReason::CaseCollision { .. } => FailureWord::Unsupported,
    }
}

/// 入れた物の識別子（正典の 4 語だけ・要件 2.5）。
pub(crate) fn kind_word(kind: &ElementKind) -> &'static str {
    match kind {
        ElementKind::Ghost => "ghost",
        ElementKind::Shell => "shell",
        ElementKind::Balloon => "balloon",
        ElementKind::Supplement => "supplement",
    }
}

/// 入れた物 1 つ（識別子・名前・場所）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledItem {
    pub kind: &'static str,
    pub name: String,
    pub place: String,
}

/// 値を byte 値 1 で繋ぐ（並びは渡された順）。
fn join<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values.collect::<Vec<_>>().join(SEPARATOR)
}

/// `OnInstallCompleteEx` の Reference0〜2（要件 2.4）。並びは `items` の順（本体が先・同梱バルーンが後）。
pub(crate) fn complete_ex_refs(items: &[InstalledItem]) -> Vec<String> {
    vec![
        join(items.iter().map(|item| item.kind)),
        join(items.iter().map(|item| item.name.as_str())),
        join(items.iter().map(|item| item.place.as_str())),
    ]
}

/// `OnInstallComplete` の Reference0〜2（要件 2.6・12.7）。
///
/// 正典の旧仕様どおり、同時に入った物のうち先頭の 2 件だけを載せる（byte 値 1 で繋がない）。
/// Reference0 は先頭の物（書庫の本体）の識別子で、同梱バルーンつきのゴーストでも `ghost`。
/// Reference2 は 2 件目の物（最初の同梱バルーン）の名前で、無ければ空。
pub(crate) fn complete_legacy_refs(manifest_name: &str, items: &[InstalledItem]) -> Vec<String> {
    let first = items.first().map_or("", |item| item.kind);
    let second = items.get(1).map_or("", |item| item.name.as_str());
    vec![
        first.to_owned(),
        manifest_name.to_owned(),
        second.to_owned(),
    ]
}

/// `OnInstallRefuse` の Reference0〜2（`accept` の値・識別子・`install.txt` の `name`＝要件 3.2）。
pub(crate) fn refuse_refs(accept: &str, manifest: &InstallManifest) -> Vec<String> {
    let kind = match manifest.kind {
        InstallKind::Ghost => ElementKind::Ghost,
        InstallKind::Shell => ElementKind::Shell,
        InstallKind::Supplement => ElementKind::Supplement,
        InstallKind::Balloon => ElementKind::Balloon,
    };
    vec![
        accept.to_owned(),
        kind_word(&kind).to_owned(),
        manifest.name.clone(),
    ]
}

/// 台本の引数が通ったときの 2 つの腕。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScriptRequest {
    /// `path` の腕: 手元の書庫の絶対パス。
    Path(PathBuf),
    /// `url` の腕: `http://`／`https://` で始まる URL（種別は `nar` か省略）。
    ///
    /// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1
    Url(String),
}

/// 台本の引数を断った理由（受け口が記録の語を選ぶ）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScriptRefusal {
    NotPath {
        found: String,
    },
    Empty,
    Relative {
        path: String,
    },
    /// URL が空か `http://`／`https://` で始まらない（要件 6.3）。
    BadUrl {
        found: String,
    },
    /// `url` の種別が `nar` でも省略でもない（`feed`・`homeurl`・`ical`・`ssf`・知らない語＝要件 6.2）。
    UnsupportedKind {
        found: String,
    },
}

/// `\![execute,install,…]` の 2 番目以降の引数を `path` か `url` の腕へ分ける（要件 1.5〜1.7・6.1〜6.3）。
///
/// `path` の腕はパスより後ろ、`url` の腕は種別より後ろの引数を読まない（残っていれば受け口が
/// 記録する）。`url` は種別を先に見る（種別と URL の両方が悪ければ種別の断り）。
pub(crate) fn script_request(arguments: &[&str]) -> Result<ScriptRequest, ScriptRefusal> {
    match arguments {
        ["path"] | ["path", "", ..] => Err(ScriptRefusal::Empty),
        ["path", path, ..] if !Path::new(path).is_absolute() => Err(ScriptRefusal::Relative {
            path: (*path).to_owned(),
        }),
        ["path", path, ..] => Ok(ScriptRequest::Path(PathBuf::from(path))),
        ["url", _, kind, ..] if *kind != "nar" => Err(ScriptRefusal::UnsupportedKind {
            found: (*kind).to_owned(),
        }),
        ["url", url, ..] if url.starts_with("http://") || url.starts_with("https://") => {
            Ok(ScriptRequest::Url((*url).to_owned()))
        }
        ["url", rest @ ..] => Err(ScriptRefusal::BadUrl {
            found: rest.first().copied().unwrap_or_default().to_owned(),
        }),
        _ => Err(ScriptRefusal::NotPath {
            found: arguments.first().copied().unwrap_or_default().to_owned(),
        }),
    }
}

#[cfg(test)]
#[path = "judge_tests.rs"]
mod tests;
