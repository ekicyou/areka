//! 手続きのテストの支え（design「Testing Strategy / 手続き」）。
//!
//! 偽の口 [`FakePorts`] は、口への呼び出しを順に記録し、台本どおりに応える。`run_engine` は
//! 固定の `Progress` の列を観測の閉包へ流してから固定の結果を返す（エンジンは呼ばない）。

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use areka_update::{
    FailReason, FetchError, ManifestName, Progress, Stage, Undeletable, UpdateError, UpdateOutcome,
};

use super::{EngineRun, GhostResources, Raised, UpdatePorts};
use crate::update::{SummaryKind, TargetKind, TargetSpec, UpdateOrder, UpdateReason};

/// 口への呼び出し 1 回。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Call {
    Raise(&'static str, Vec<String>),
    Resources,
    RunEngine { homeurl: String, target: PathBuf },
    Reload(PathBuf),
    Started,
}

/// エンジンの一周の台本（観測へ流す進捗と、返す結果）。
pub(super) struct EngineScript {
    pub progress: Vec<Progress>,
    pub result: EngineRun,
}

/// 台本どおりに応える偽の口。
pub(super) struct FakePorts {
    /// イベントごとの応え（無ければ返事なし）。
    pub replies: HashMap<&'static str, Raised>,
    /// 照会の答え（None＝kanade が居ない）。
    pub resources: Option<GhostResources>,
    /// `run_engine` の呼ばれるたびに先頭から 1 つずつ使う。
    pub engine: RefCell<VecDeque<EngineScript>>,
    pub calls: RefCell<Vec<Call>>,
}

impl FakePorts {
    /// 返事なし・照会の答えは空・エンジンの台本は `scripts` の順。
    pub(super) fn new(scripts: impl IntoIterator<Item = EngineScript>) -> Self {
        Self {
            replies: HashMap::new(),
            resources: Some(GhostResources::default()),
            engine: RefCell::new(scripts.into_iter().collect()),
            calls: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }

    /// 送ったイベントの (名前, Reference) の列。
    pub(super) fn raised(&self) -> Vec<(&'static str, Vec<String>)> {
        self.calls
            .borrow()
            .iter()
            .filter_map(|call| match call {
                Call::Raise(id, refs) => Some((*id, refs.clone())),
                _ => None,
            })
            .collect()
    }

    /// 送ったイベントの名前の列。
    pub(super) fn raised_ids(&self) -> Vec<&'static str> {
        self.raised().into_iter().map(|(id, _)| id).collect()
    }
}

impl UpdatePorts for FakePorts {
    fn raise(&self, id: &'static str, references: Vec<String>) -> Raised {
        self.calls.borrow_mut().push(Call::Raise(id, references));
        self.replies.get(id).copied().unwrap_or(Raised::NoReply)
    }

    fn resources(&self) -> Option<GhostResources> {
        self.calls.borrow_mut().push(Call::Resources);
        self.resources.clone()
    }

    fn run_engine(
        &self,
        homeurl: &str,
        target: &Path,
        observe: &mut dyn FnMut(&Progress),
    ) -> EngineRun {
        self.calls.borrow_mut().push(Call::RunEngine {
            homeurl: homeurl.to_owned(),
            target: target.to_path_buf(),
        });
        let script = self
            .engine
            .borrow_mut()
            .pop_front()
            .expect("エンジンの台本が呼ばれた回数だけ在る");
        for p in &script.progress {
            observe(p);
        }
        script.result
    }

    fn request_reload(&self, ghost_dir: &Path) {
        self.calls
            .borrow_mut()
            .push(Call::Reload(ghost_dir.to_path_buf()));
    }

    fn standard_started(&self) {
        self.calls.borrow_mut().push(Call::Started);
    }
}

pub(super) const GHOST_URL: &str = "https://example.invalid/ghost/";
pub(super) const SHELL_URL: &str = "https://example.invalid/shell/";

/// 対象 1 つ（フォルダは相対でよい＝fs に触れない）。
pub(super) fn spec(kind: TargetKind, name: &str, homeurl: Option<&str>) -> TargetSpec {
    TargetSpec {
        kind,
        dir: PathBuf::from(format!("root/{name}")),
        name: name.to_owned(),
        descript_homeurl: homeurl.map(str::to_owned),
    }
}

pub(super) fn order(
    targets: Vec<TargetSpec>,
    reason: UpdateReason,
    summary: SummaryKind,
) -> UpdateOrder {
    UpdateOrder {
        targets,
        reason,
        summary,
        ghost_dir: PathBuf::from("root/ghost/emo2"),
        ghost_folder: Some("emo2".to_owned()),
    }
}

// 4 経路の台本（design「Testing Strategy / 手続き」）。

/// 差分 0＝`none`。
pub(super) fn script_none() -> EngineScript {
    EngineScript {
        progress: vec![
            Progress::ManifestFetched {
                name: ManifestName::UpdatesTxt,
            },
            Progress::DiffDecided { files: Vec::new() },
        ],
        result: EngineRun::Done(Ok(UpdateOutcome::Unchanged {
            manifest: ManifestName::UpdatesTxt,
        })),
    }
}

/// 差分 2 件の成功＝`changed`。
pub(super) fn script_changed() -> EngineScript {
    EngineScript {
        progress: vec![
            Progress::ManifestFetched {
                name: ManifestName::UpdatesTxt,
            },
            Progress::DiffDecided {
                files: vec!["a.txt".to_owned(), "b.txt".to_owned()],
            },
            download("a.txt", 0, 2),
            md5("a.txt", true),
            download("b.txt", 1, 2),
            md5("b.txt", true),
            Progress::Committed {
                placed: vec!["a.txt".to_owned(), "b.txt".to_owned()],
            },
            Progress::Deleted {
                removed: Vec::new(),
            },
        ],
        result: EngineRun::Done(Ok(UpdateOutcome::Updated {
            manifest: ManifestName::UpdatesTxt,
            placed: vec!["a.txt".to_owned(), "b.txt".to_owned()],
            removed: Vec::new(),
            undeletable: Vec::new(),
            leftovers: Vec::new(),
        })),
    }
}

/// 取得の失敗（`a.txt` が時間切れ）。
pub(super) fn script_fetch_failed(target: &Path) -> EngineScript {
    EngineScript {
        progress: vec![
            Progress::ManifestFetched {
                name: ManifestName::UpdatesTxt,
            },
            Progress::DiffDecided {
                files: vec!["a.txt".to_owned()],
            },
            download("a.txt", 0, 1),
        ],
        result: EngineRun::Done(Err(error(
            target,
            Stage::Download { index: 0, total: 1 },
            FailReason::FileFetch {
                file: "a.txt".to_owned(),
                source: FetchError::Timeout,
            },
        ))),
    }
}

/// MD5 の不一致（`a.txt`）。
pub(super) fn script_md5_mismatch(target: &Path) -> EngineScript {
    EngineScript {
        progress: vec![
            Progress::ManifestFetched {
                name: ManifestName::UpdatesTxt,
            },
            Progress::DiffDecided {
                files: vec!["a.txt".to_owned()],
            },
            download("a.txt", 0, 1),
            md5("a.txt", false),
        ],
        result: EngineRun::Done(Err(error(
            target,
            Stage::Verify { index: 0, total: 1 },
            FailReason::Md5Mismatch {
                file: "a.txt".to_owned(),
                expected: EXPECTED.to_owned(),
                actual: WRONG.to_owned(),
            },
        ))),
    }
}

/// 成功したが、消せなかった物 1 件と片付けられなかった作業場所 1 件が残った（要件 4.8）。
pub(super) fn script_updated_with_leftovers(undeletable: &Path, leftover: &Path) -> EngineScript {
    EngineScript {
        progress: vec![
            Progress::DiffDecided {
                files: vec!["a.txt".to_owned()],
            },
            download("a.txt", 0, 1),
            md5("a.txt", true),
        ],
        result: EngineRun::Done(Ok(UpdateOutcome::Updated {
            manifest: ManifestName::UpdatesTxt,
            placed: vec!["a.txt".to_owned()],
            removed: Vec::new(),
            undeletable: vec![Undeletable {
                path: undeletable.to_path_buf(),
                source: std::io::Error::other("使用中"),
            }],
            leftovers: vec![leftover.to_path_buf()],
        })),
    }
}

/// 確定で書けず、戻せなかった（作業場所 `work` に元の内容が残る・要件 4.5・4.6）。
pub(super) fn script_rollback_failed(target: &Path, work: &Path) -> EngineScript {
    let mut err = error(
        target,
        Stage::Commit,
        FailReason::RollbackFailed {
            path: target.join("a.txt"),
            source: std::io::Error::other("書けない"),
            restored: Vec::new(),
            stuck: Vec::new(),
        },
    );
    err.work = Some(work.to_path_buf());
    EngineScript {
        progress: vec![
            Progress::DiffDecided {
                files: vec!["a.txt".to_owned()],
            },
            download("a.txt", 0, 1),
            md5("a.txt", true),
        ],
        result: EngineRun::Done(Err(err)),
    }
}

pub(super) const EXPECTED: &str = "0123456789abcdef0123456789abcdef";
pub(super) const WRONG: &str = "ffffffffffffffffffffffffffffffff";

fn download(file: &str, index: usize, total: usize) -> Progress {
    Progress::DownloadBegin {
        file: file.to_owned(),
        index,
        total,
    }
}

fn md5(file: &str, matched: bool) -> Progress {
    Progress::Md5Compared {
        file: file.to_owned(),
        expected: EXPECTED.to_owned(),
        actual: if matched { EXPECTED } else { WRONG }.to_owned(),
        matched,
    }
}

fn error(target: &Path, stage: Stage, reason: FailReason) -> UpdateError {
    UpdateError {
        homeurl: GHOST_URL.to_owned(),
        target: target.to_path_buf(),
        stage,
        reason,
        leftovers: Vec::new(),
        work: None,
    }
}
