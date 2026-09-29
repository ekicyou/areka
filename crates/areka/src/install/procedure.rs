//! インストールの手続き（design「areka / install / procedure」）。
//!
//! 書庫 1 本の一周（`OnInstallBegin` → 読み取りと検査 → `accept` の照合 → 利用条件 → 展開 →
//! 入れた後の記録 → 締めの知らせ）と、依頼 1 件の一周を正典の順で進める。World も
//! スレッドも門も知らず、外とのやり取りは口 [`InstallPorts`] の 7 つだけ（本番は背景の
//! スレッドの口・テストは偽物）。口に切替を頼む関数は無い＝手続きが切替を要求することは
//! 型の上で起きない（要件 6.2）。

use std::path::{Path, PathBuf};

use areka_ghost::catalog::{BasewareRoot, list_balloons};
use areka_nar::{
    ElementKind, InstallKind, InstallOutcome, IoPhase, NarArchive, NarError, SurvivingTree,
};

use super::judge::{
    AcceptVerdict, Destination, FailureWord, GhostFacts, InstalledItem, complete_ex_refs,
    complete_legacy_refs, destination_of, failure_word, judge_accept, kind_word, refuse_refs,
};
use super::terms::{TermsNotice, find_terms, nested_terms};
use super::{InstallOrder, InstallOrigin};
use crate::alert::YesNo;

// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallBegin:1
const ON_INSTALL_BEGIN: &str = "OnInstallBegin";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallCompleteEx:1
const ON_INSTALL_COMPLETE_EX: &str = "OnInstallCompleteEx";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallComplete:1
const ON_INSTALL_COMPLETE: &str = "OnInstallComplete";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallCompleteAll:1
const ON_INSTALL_COMPLETE_ALL: &str = "OnInstallCompleteAll";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallFailure:1
const ON_INSTALL_FAILURE: &str = "OnInstallFailure";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallRefuse:1
const ON_INSTALL_REFUSE: &str = "OnInstallRefuse";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnGhostTermsAccept:1
const ON_GHOST_TERMS_ACCEPT: &str = "OnGhostTermsAccept";
// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnGhostTermsDecline:1
const ON_GHOST_TERMS_DECLINE: &str = "OnGhostTermsDecline";

/// 手続きが外とやり取りする口（本番は背景のスレッド・テストは偽物）。
pub(crate) trait InstallPorts {
    /// 書庫 1 本の手続きを始める（`path` は書庫のパス・終了の待ちの記録に載せる）。
    fn begin_archive(&mut self, path: &Path);
    /// イベントを今のゴーストへ GET で送り、応えを待つ。定常でなければ定常まで待つ。
    fn raise(&mut self, id: &'static str, references: Vec<String>) -> Raised;
    /// 今のゴーストの素性。ゴーストが居なければ None。
    fn ghost_facts(&mut self) -> Option<GhostFacts>;
    /// 利用条件を出して答えを受ける。
    fn ask_terms(&mut self, title: &str, notice: &TermsNotice) -> YesNo;
    /// 起動中のゴースト以外の宛先へ入れる。終了が始まっていて入らなかったら None。
    fn install_elsewhere(
        &mut self,
        archive: &NarArchive,
        target_ghost: Option<&str>,
    ) -> Option<Result<InstallOutcome, NarError>>;
    /// 起動中のゴーストを降ろしてから入れ、起こし直す。
    fn overwrite_running(
        &mut self,
        archive: NarArchive,
        target_ghost: Option<String>,
    ) -> Overwritten;
    /// 入れた後の記録（受け皿・バルーンの記憶・置換語）。反映が済んでから戻る。
    fn record(&mut self, record: InstalledRecord);
}

/// イベントを送った結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Raised {
    /// ゴーストが台本を返した。
    Script,
    /// 返事なし・空の台本。
    NoReply,
    /// 終了が始まった・ゴーストが居ない。
    Closed,
}

/// 起動中のゴーストへ入れた結果。
pub(crate) enum Overwritten {
    Ran(Result<InstallOutcome, NarError>),
    /// 宛先はもう起動中のゴーストではない（書庫を返す）。
    NotRunning(NarArchive),
    Closed,
}

/// 入れた後に覚えること（書庫 1 本ぶん）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledRecord {
    pub kind: InstallKind,
    /// `install.txt` の `name`。
    pub object_name: String,
    /// 入れた先のフォルダ名（`ghost` はゴースト・`balloon` はバルーン）。
    pub folder: String,
    /// 関わったゴーストのフォルダ名（`balloon` は None）。
    pub ghost_folder: Option<String>,
}

/// 書庫 1 本の終わり方。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArchiveEnd {
    Installed(Vec<InstalledItem>),
    Refused,
    Declined,
    Failed(FailureWord),
    /// 終了が始まったので途中でやめた（締めの知らせ 0 件）。
    Abandoned,
}

/// 口が「閉じた」を返した（終了が始まった・ゴーストが居ない）。以後は送らない。
struct Closed;

/// 依頼 1 件を並んだ順に扱い、書庫ごとの終わり方を返す（要件 2.8・2.9・9.2）。
///
/// 途中でやめた書庫があれば、そこで抜ける（以後の書庫は扱わず、イベントも送らない）。
/// 書庫が 2 本以上で全部が入ったときだけ、最後に `OnInstallCompleteAll` を送る。
pub(crate) fn run_order(order: &InstallOrder, ports: &mut dyn InstallPorts) -> Vec<ArchiveEnd> {
    let mut ends = Vec::with_capacity(order.archives.len());
    for (index, archive) in order.archives.iter().enumerate() {
        let end = run_archive(archive, &order.archives[index + 1..], order.origin, ports);
        let abandoned = end == ArchiveEnd::Abandoned;
        ends.push(end);
        if abandoned {
            return ends;
        }
    }
    if ends.len() >= 2 {
        let all: Option<Vec<InstalledItem>> = ends
            .iter()
            .map(|end| match end {
                ArchiveEnd::Installed(items) => Some(items.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
            .map(|lists| lists.concat());
        if let Some(items) = all {
            // 最後の知らせ。閉じていても、この後に送るものは無い。
            let _ = send(ports, ON_INSTALL_COMPLETE_ALL, complete_ex_refs(&items));
        }
    }
    ends
}

/// 書庫 1 本の一周。口が閉じたら、同じ依頼のまだ始めていない書庫（`rest`）の件数とパスも
/// 載せた記録を 1 件残して `Abandoned`。
fn run_archive(
    path: &Path,
    rest: &[PathBuf],
    origin: InstallOrigin,
    ports: &mut dyn InstallPorts,
) -> ArchiveEnd {
    match archive_steps(path, origin, ports) {
        Ok(end) => end,
        Err(Closed) => {
            tracing::warn!(
                event = "install_abandoned",
                archive = %path.display(),
                skipped = rest.len(),
                skipped_archives = ?rest,
                "[install] 終了が始まった・ゴーストが居ないので、書庫の手続きを途中でやめました（まだ始めていない書庫も扱いません）"
            );
            ArchiveEnd::Abandoned
        }
    }
}

fn archive_steps(
    path: &Path,
    origin: InstallOrigin,
    ports: &mut dyn InstallPorts,
) -> Result<ArchiveEnd, Closed> {
    tracing::info!(
        event = "install_begin",
        archive = %path.display(),
        origin = ?origin,
        "[install] 書庫の手続きを始めます"
    );
    ports.begin_archive(path);
    send(ports, ON_INSTALL_BEGIN, Vec::new())?;

    // 読み取りと検査（1 バイトも書かない）。
    let archive = match NarArchive::open(path) {
        Ok(archive) => archive,
        Err(error) => return fail_nar(ports, path, "open", &error),
    };
    let manifest = archive.manifest();
    let kind = manifest.kind;

    // `accept` の照合（`install.txt` の内容だけで・展開の前に＝要件 3.3）。
    let facts = ports.ghost_facts().ok_or(Closed)?;
    let target_ghost = match judge_accept(manifest, &facts) {
        AcceptVerdict::Accepted { target_ghost } => {
            tracing::info!(
                event = "install_accept",
                archive = %path.display(),
                verdict = "accepted",
                accept = ?manifest.accept,
                target_ghost = ?target_ghost,
                "[install] 受け取ります"
            );
            target_ghost
        }
        AcceptVerdict::Refused { accept } => {
            // 宛先違いは失敗ではない（要件 3.7）。
            tracing::warn!(
                event = "install_refused",
                archive = %path.display(),
                accept = %accept,
                running = %facts.name,
                "[install] 別のゴーストを指名した書庫なので断ります"
            );
            send(ports, ON_INSTALL_REFUSE, refuse_refs(&accept, manifest))?;
            return Ok(ArchiveEnd::Refused);
        }
        AcceptVerdict::AcceptMissing => {
            return fail(
                ports,
                path,
                Failure {
                    kind: "AcceptMissing",
                    phase: "accept",
                    rolled_back: true,
                    word: FailureWord::InvalidType,
                    survivors: &[],
                },
            );
        }
    };

    // 利用条件（`OnInstallBegin` と照合の後・展開の前＝要件 12.8）。
    if let Some(end) = terms_step(ports, path, &archive)? {
        return Ok(end);
    }

    // 展開。
    let result = match destination_of(manifest, &facts) {
        Destination::RunningGhost => match ports.overwrite_running(archive, target_ghost.clone()) {
            Overwritten::Ran(result) => result,
            // その間に切り替わった＝もう起動中のゴーストではない。よそへ入れる。
            Overwritten::NotRunning(archive) => ports
                .install_elsewhere(&archive, target_ghost.as_deref())
                .ok_or(Closed)?,
            Overwritten::Closed => return Err(Closed),
        },
        Destination::Elsewhere => ports
            .install_elsewhere(&archive, target_ghost.as_deref())
            .ok_or(Closed)?,
    };
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(error) => return fail_nar(ports, path, "install", &error),
    };

    // 入れた物（識別子・名前・場所・並びは本体が先・同梱バルーンが後＝要件 2.4）。
    let items = installed_items(&outcome, &facts);
    tracing::info!(
        event = "install_done",
        archive = %path.display(),
        kind = ?kind,
        places = ?items.iter().map(|item| item.place.as_str()).collect::<Vec<_>>(),
        "[install] 入れました"
    );
    ports.record(installed_record(kind, &outcome));

    // 締めの知らせ（応えが無いときだけ旧仕様へ続ける＝要件 2.6・2.7）。
    if send(ports, ON_INSTALL_COMPLETE_EX, complete_ex_refs(&items))? == Raised::NoReply {
        send(
            ports,
            ON_INSTALL_COMPLETE,
            complete_legacy_refs(&outcome.name, &items),
        )?;
    }
    Ok(ArchiveEnd::Installed(items))
}

/// 利用条件を出し、拒否なら `OnGhostTermsDecline` だけを送って `Declined` を返す（要件 4・12.8）。
/// 利用条件が無いか受諾なら None（展開へ進む）。
fn terms_step(
    ports: &mut dyn InstallPorts,
    path: &Path,
    archive: &NarArchive,
) -> Result<Option<ArchiveEnd>, Closed> {
    let nested = nested_terms(archive);
    if !nested.is_empty() {
        tracing::info!(
            event = "install_terms_nested_skipped",
            archive = %path.display(),
            files = ?nested,
            "[install] 同梱バルーンの中の利用条件は出しません"
        );
    }
    let Some(notice) = find_terms(archive) else {
        return Ok(None);
    };
    let title = format!("利用条件 - {}", archive.manifest().name);
    let answer = ports.ask_terms(&title, &notice);
    let archive = path.display();
    match answer {
        YesNo::Yes => {
            tracing::info!(event = "install_terms", %archive, file = notice.file, answer = "accept", "[install] 利用条件が受諾されました");
            send(ports, ON_GHOST_TERMS_ACCEPT, Vec::new())?;
            return Ok(None);
        }
        YesNo::No => {
            tracing::info!(event = "install_terms", %archive, file = notice.file, answer = "decline", "[install] 利用条件が拒否されました");
        }
        YesNo::Suppressed => {
            tracing::warn!(event = "install_terms", %archive, file = notice.file, answer = "suppressed", "[install] 告知が抑止されているので、利用条件を拒否として扱います");
        }
        YesNo::Unavailable => {
            tracing::error!(event = "install_terms", %archive, file = notice.file, answer = "unavailable", "[install] 利用条件の画面を出せなかったので、拒否として扱います");
        }
    }
    send(ports, ON_GHOST_TERMS_DECLINE, Vec::new())?;
    Ok(Some(ArchiveEnd::Declined))
}

/// イベントを送って応えを記録する。`Closed` なら以後は送らない。
fn send(
    ports: &mut dyn InstallPorts,
    id: &'static str,
    references: Vec<String>,
) -> Result<Raised, Closed> {
    let raised = ports.raise(id, references);
    tracing::info!(event = "install_event", id, raised = ?raised, "[install] イベントを送りました");
    match raised {
        Raised::Closed => Err(Closed),
        other => Ok(other),
    }
}

/// 失敗 1 件の要約（`install_failed` の欄）。
struct Failure<'a> {
    /// `areka-nar` の失敗の種類の語（拒否は `RefuseReason::kind`・I/O は `Io`）。
    kind: &'static str,
    /// どの段で失敗したか。
    phase: &'static str,
    /// 宛先が元のままか（書く前の失敗は真）。
    rolled_back: bool,
    word: FailureWord,
    survivors: &'a [SurvivingTree],
}

fn fail_nar(
    ports: &mut dyn InstallPorts,
    path: &Path,
    stage: &'static str,
    error: &NarError,
) -> Result<ArchiveEnd, Closed> {
    let failure = match error {
        NarError::Refused { reason, .. } => Failure {
            kind: reason.kind(),
            phase: stage,
            rolled_back: true,
            word: failure_word(error),
            survivors: &[],
        },
        NarError::Io {
            phase,
            rolled_back,
            survivors,
            ..
        } => Failure {
            kind: "Io",
            phase: match phase {
                IoPhase::Read => "read",
                IoPhase::Stage => "stage",
                IoPhase::Commit => "commit",
                IoPhase::Rollback => "rollback",
            },
            rolled_back: *rolled_back,
            word: failure_word(error),
            survivors,
        },
    };
    fail(ports, path, failure)
}

/// 失敗を記録し、`OnInstallFailure`（Reference0 は正典の語 1 つだけ）を送る（要件 5.1・5.5・5.6）。
fn fail(
    ports: &mut dyn InstallPorts,
    path: &Path,
    failure: Failure<'_>,
) -> Result<ArchiveEnd, Closed> {
    let word = failure.word.as_ref_str();
    tracing::error!(
        event = "install_failed",
        archive = %path.display(),
        kind = failure.kind,
        phase = failure.phase,
        rolled_back = failure.rolled_back,
        word,
        "[install] 書庫を入れられませんでした"
    );
    for survivor in failure.survivors {
        tracing::error!(
            event = "install_survivor",
            archive = %path.display(),
            destination = %survivor.destination.display(),
            kept_at = %survivor.path.display(),
            "[install] 宛先を元へ戻せませんでした。元の中身は kept_at に残っていて、7 日で消えます"
        );
    }
    send(ports, ON_INSTALL_FAILURE, vec![word.to_owned()])?;
    Ok(ArchiveEnd::Failed(failure.word))
}

/// 入れた物（識別子・名前・場所）。同梱バルーン（先頭より後のバルーン）の名前は、入れた後の
/// 目録の `name`（設計で決めたこと 14）。目録に無い・名前が無ければ `areka-nar` の名前（フォルダ名）。
fn installed_items(outcome: &InstallOutcome, facts: &GhostFacts) -> Vec<InstalledItem> {
    let has_companion = outcome
        .installed
        .iter()
        .skip(1)
        .any(|element| element.kind == ElementKind::Balloon);
    let balloons = if has_companion {
        list_balloons(&BasewareRoot::new(facts.root.clone()))
    } else {
        Vec::new()
    };
    outcome
        .installed
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let folder = element.path.file_name().map(|name| name.to_string_lossy());
            let catalog_name = (index > 0 && element.kind == ElementKind::Balloon)
                .then(|| {
                    balloons.iter().find(|entry| {
                        folder.as_deref().is_some_and(|folder| {
                            entry.identity.folder.eq_ignore_ascii_case(folder)
                        })
                    })
                })
                .flatten()
                .and_then(|entry| entry.identity.name.clone());
            InstalledItem {
                kind: kind_word(&element.kind),
                name: catalog_name.unwrap_or_else(|| element.name.clone()),
                place: element.path.display().to_string(),
            }
        })
        .collect()
}

/// 入れた後に覚えること。フォルダ名は本体（`installed` の先頭）の置き場の名前。
fn installed_record(kind: InstallKind, outcome: &InstallOutcome) -> InstalledRecord {
    let body = outcome.installed.first();
    let folder = body
        .and_then(|element| element.path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ghost_folder = match kind {
        InstallKind::Ghost => Some(folder.clone()),
        InstallKind::Shell | InstallKind::Supplement => {
            body.and_then(|element| element.target_ghost.clone())
        }
        InstallKind::Balloon => None,
    };
    InstalledRecord {
        kind,
        object_name: outcome.name.clone(),
        folder,
        ghost_folder,
    }
}

#[cfg(test)]
#[path = "procedure_test_support.rs"]
mod procedure_test_support;

#[cfg(test)]
#[path = "procedure_tests.rs"]
mod procedure_tests;

#[cfg(test)]
#[path = "procedure_branch_tests.rs"]
mod procedure_branch_tests;
