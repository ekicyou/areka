//! 手続きのテストの支え（design「Testing Strategy / 手続き」）。
//!
//! 偽の口 [`FakePorts`] は、口への呼び出しを順に記録し、台本どおりに応える。展開は本物の
//! `areka-nar` を一時の根へ走らせる（`install_elsewhere`／`overwrite_running` の中で
//! `archive.install` を呼ぶ）。書庫は `sample_ghost_kit` の `nar_writer` で組む。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use areka_nar::{InstallOutcome, InstallRequest, IoPhase, NarArchive, NarError, SurvivingTree};
use sample_ghost_kit::{NarBuilder, install_txt};
use temp_path_kit::TempPath;

use super::{InstallPorts, InstalledRecord, Overwritten, Raised};
use crate::alert::YesNo;
use crate::install::judge::GhostFacts;
use crate::install::terms::TermsNotice;
use crate::install::{InstallOrder, InstallOrigin};

/// 起動中のゴーストのフォルダ名。
pub(super) const RUNNING: &str = "running";
/// 起動中のゴーストの `sakura.name`。
pub(super) const RUNNING_SAKURA: &str = "さくら";
/// 起動中のゴーストの `install.accept` の 2 番目の名前。
pub(super) const RUNNING_ALIAS: &str = "kinoko";

/// 口への呼び出し 1 回。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Call {
    Raise(&'static str, Vec<String>),
    Facts,
    Ask(TermsNotice),
    Elsewhere(Option<String>),
    Overwrite(Option<String>),
    Record(InstalledRecord),
}

/// `overwrite_running` の応え方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OverwriteScript {
    /// 本物の `areka-nar` で入れる。
    Run,
    /// もう起動中のゴーストではない（書庫を返す）。
    NotRunning,
    Closed,
}

/// 台本どおりに応える偽の口。根は一時フォルダで、起動中のゴースト `running` が居る。
pub(super) struct FakePorts {
    pub root: TempPath,
    pub facts: Option<GhostFacts>,
    /// イベントごとの応え（無ければ返事なし）。
    pub replies: HashMap<&'static str, Raised>,
    pub terms_answer: YesNo,
    /// 真なら `install_elsewhere` が入らずに None を返す。
    pub elsewhere_closed: bool,
    /// 在れば `install_elsewhere` が入れずに確定の段の I/O の失敗を返す（生き残りはこの列）。
    pub elsewhere_error: Option<Vec<SurvivingTree>>,
    pub overwrite: OverwriteScript,
    pub calls: Vec<Call>,
}

impl FakePorts {
    pub(super) fn new() -> Self {
        let root = TempPath::new("install-procedure");
        let master = root
            .path()
            .join("ghost")
            .join(RUNNING)
            .join("ghost")
            .join("master");
        std::fs::create_dir_all(&master).expect("起動中のゴーストの置き場を作れる");
        std::fs::write(
            master.join("descript.txt"),
            format!("charset,UTF-8\r\nsakura.name,{RUNNING_SAKURA}\r\ninstall.accept,x,{RUNNING_ALIAS}\r\n"),
        )
        .expect("起動中のゴーストの descript.txt を置ける");
        let facts = GhostFacts {
            root: root.path().to_path_buf(),
            folder: Some(RUNNING.to_owned()),
            name: "Running".to_owned(),
            sakura_name: Some(RUNNING_SAKURA.to_owned()),
            install_accept: vec!["x".to_owned(), RUNNING_ALIAS.to_owned()],
        };
        FakePorts {
            root,
            facts: Some(facts),
            replies: HashMap::new(),
            terms_answer: YesNo::Yes,
            elsewhere_closed: false,
            elsewhere_error: None,
            overwrite: OverwriteScript::Run,
            calls: Vec::new(),
        }
    }

    /// 根の下のパス。
    pub(super) fn under_root(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.root.path().to_path_buf(), |path, part| path.join(part))
    }

    /// 送られたイベント（名前と Reference）を順に。
    pub(super) fn raised(&self) -> Vec<(&'static str, Vec<String>)> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                Call::Raise(id, refs) => Some((*id, refs.clone())),
                _ => None,
            })
            .collect()
    }

    /// 送られたイベントの名前を順に。
    pub(super) fn raised_ids(&self) -> Vec<&'static str> {
        self.raised().into_iter().map(|(id, _)| id).collect()
    }

    fn install(
        &self,
        archive: &NarArchive,
        target_ghost: Option<&str>,
    ) -> Result<InstallOutcome, NarError> {
        archive.install(&InstallRequest {
            root: self.root.path(),
            target_ghost,
        })
    }
}

impl InstallPorts for FakePorts {
    fn raise(&mut self, id: &'static str, references: Vec<String>) -> Raised {
        self.calls.push(Call::Raise(id, references));
        self.replies.get(id).copied().unwrap_or(Raised::NoReply)
    }

    fn ghost_facts(&mut self) -> Option<GhostFacts> {
        self.calls.push(Call::Facts);
        self.facts.clone()
    }

    fn ask_terms(&mut self, _title: &str, notice: &TermsNotice) -> YesNo {
        self.calls.push(Call::Ask(notice.clone()));
        self.terms_answer
    }

    fn install_elsewhere(
        &mut self,
        archive: &NarArchive,
        target_ghost: Option<&str>,
    ) -> Option<Result<InstallOutcome, NarError>> {
        self.calls
            .push(Call::Elsewhere(target_ghost.map(str::to_owned)));
        if self.elsewhere_closed {
            return None;
        }
        if let Some(survivors) = &self.elsewhere_error {
            return Some(Err(NarError::Io {
                archive: PathBuf::from("fake.nar"),
                phase: IoPhase::Commit,
                path: self.root.path().to_path_buf(),
                source: std::io::Error::other("偽の確定の失敗"),
                committed: Vec::new(),
                rolled_back: survivors.is_empty(),
                survivors: survivors.clone().into_boxed_slice(),
            }));
        }
        Some(self.install(archive, target_ghost))
    }

    fn overwrite_running(
        &mut self,
        archive: NarArchive,
        target_ghost: Option<String>,
    ) -> Overwritten {
        self.calls.push(Call::Overwrite(target_ghost.clone()));
        match self.overwrite {
            OverwriteScript::Run => {
                Overwritten::Ran(self.install(&archive, target_ghost.as_deref()))
            }
            OverwriteScript::NotRunning => Overwritten::NotRunning(archive),
            OverwriteScript::Closed => Overwritten::Closed,
        }
    }

    fn record(&mut self, record: InstalledRecord) {
        self.calls.push(Call::Record(record));
    }
}

/// 書庫を一時フォルダへ書き、そのパスを返す（フォルダは `dir` が生きている間だけ在る）。
pub(super) fn write_nar(dir: &TempPath, file: &str, builder: NarBuilder) -> PathBuf {
    let path = dir.child(file);
    builder.write_to(&path).expect("書庫を置ける");
    path
}

/// 書庫 1 本の依頼。
pub(super) fn order_of(archives: &[&Path]) -> InstallOrder {
    InstallOrder {
        archives: archives.iter().map(|path| path.to_path_buf()).collect(),
        origin: InstallOrigin::Menu,
    }
}

/// ゴースト（`directory` を選べる・`extra` で `install.txt` に行を足す）。
pub(super) fn ghost_nar(directory: &str, extra: &[&str]) -> NarBuilder {
    let directory_line = format!("directory,{directory}");
    let mut lines = vec![
        "charset,UTF-8",
        "type,ghost",
        "name,あたらしい",
        directory_line.as_str(),
    ];
    lines.extend_from_slice(extra);
    NarBuilder::new()
        .file("install.txt", &install_txt(&lines))
        .done()
        .file(
            "ghost/master/descript.txt",
            b"charset,UTF-8\r\nname,Newbie\r\n",
        )
        .done()
}

/// バルーン `kaku`（目録の名前は `Kaku`）を同梱したゴースト。
pub(super) fn ghost_with_balloon_nar() -> NarBuilder {
    ghost_with_balloon_descript(b"charset,UTF-8\r\nname,Kaku\r\n")
}

/// バルーン `kaku` を同梱したゴースト（同梱バルーンの `descript.txt` を選べる）。
pub(super) fn ghost_with_balloon_descript(descript: &[u8]) -> NarBuilder {
    ghost_nar(
        "newbie",
        &["balloon.directory,kaku", "balloon.source.directory,kaku"],
    )
    .file("kaku/descript.txt", descript)
    .done()
}

/// バルーン。
pub(super) fn balloon_nar() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,balloon",
                "name,まるい",
                "directory,round",
            ]),
        )
        .done()
        .file("descript.txt", b"charset,UTF-8\r\nname,Round\r\n")
        .done()
}

/// `accept` 付きのシェル。
pub(super) fn shell_nar(accept: &str) -> NarBuilder {
    let accept_line = format!("accept,{accept}");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,shell",
                "name,きがえ",
                "directory,dress",
                accept_line.as_str(),
            ]),
        )
        .done()
        .file("descript.txt", b"charset,UTF-8\r\nname,Dress\r\n")
        .done()
}

/// `accept` 付きの追加ファイル。
pub(super) fn supplement_nar(accept: &str) -> NarBuilder {
    let accept_line = format!("accept,{accept}");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,supplement",
                "name,ついか",
                "directory,extra",
                accept_line.as_str(),
            ]),
        )
        .done()
        .file("ghost/master/extra.dic", b"extra")
        .done()
}
