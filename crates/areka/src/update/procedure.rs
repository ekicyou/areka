//! 更新の手続き（要求 1 件の一周・対象 1 つの一周。design「手続き（`update/procedure.rs`）」）。
//!
//! 外とのやり取りは口 [`UpdatePorts`] の 5 つだけ（イベントを送って応えを待つ・リソースを 1 回
//! 照会する・エンジンを 1 周回す・読み直しを頼む・標準の手続きが始まったと知らせる）。本番の口は
//! 背景スレッドの口（6.1）、テストは偽の口。手続きは告知（`alert`）を呼ばない（要件 4.1）。

use std::path::Path;

use areka_update::{FetchError, Progress, UpdateError, UpdateOutcome};

use super::refs::{
    self, EventNames, Numbering, ON_UPDATE_PROCESS_EXEC, ON_UPDATE_RESULT, ON_UPDATE_RESULT_EX,
    Tail, TargetEnd,
};
use super::{SummaryKind, TargetKind, TargetSpec, UpdateOrder, UpdateReason};

/// 手続きが外とやり取りする口（本番は背景スレッドの口・テストは偽物）。観測の閉包の中から
/// 同じ口で送るため、どれも `&self`。
pub(crate) trait UpdatePorts {
    /// イベントを今のゴーストへ GET で送り、応えを待つ。
    fn raise(&self, id: &'static str, references: Vec<String>) -> Raised;
    /// `homeurl`・`useorigin1` を 1 回照会する。kanade が居なければ None。
    fn resources(&self) -> Option<GhostResources>;
    /// エンジンを 1 周回す（門の出入り込み）。
    fn run_engine(
        &self,
        homeurl: &str,
        target: &Path,
        observe: &mut dyn FnMut(&Progress),
    ) -> EngineRun;
    /// 同じゴーストの読み直しを窓口へ頼む。
    fn request_reload(&self, ghost_dir: &Path);
    /// 標準の手続きが始まったと窓口へ知らせる（`OnUpdateProcessExec` の段を抜けた直後・要求 1 件につき高々 1 回）。
    fn standard_started(&self);
}

/// イベントへの応え。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Raised {
    /// ゴーストが台本を返した。
    Script,
    /// 返事なし（空の台本を含む）。
    NoReply,
    /// 終了が始まった・ゴーストが居なくなった・定常でない。
    Closed,
}

/// 照会の答え（空の値は口が None に写す）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GhostResources {
    pub homeurl: Option<String>,
    pub useorigin1: Option<String>,
}

/// エンジンの一周の結果。
#[derive(Debug)]
pub(crate) enum EngineRun {
    Done(Result<UpdateOutcome, UpdateError>),
    /// 取得口を作れない（理由 connect）。
    Unavailable(FetchError),
    /// 終了が始まっていて走らせなかった。
    Closed,
}

/// 要求 1 件の終わり方（対象ごとの終わり方の列と、総括が受け取られたか・読み直しを頼んだか）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct OrderEnd {
    pub ends: Vec<TargetEnd>,
    pub summarised: bool,
    pub reload_requested: bool,
}

/// 要求 1 件を並んだ順に扱う（design「要求 1 件の一周」）。
pub(crate) fn run_order(order: &UpdateOrder, ports: &dyn UpdatePorts) -> OrderEnd {
    tracing::info!(
        event = "update_order_begin",
        reason = order.reason.as_ref_str(),
        targets = order.targets.len(),
        ghost_dir = %order.ghost_dir.display(),
        "[update] 更新の要求を始めます"
    );
    let mut end = OrderEnd::default();

    // メニューからだけ `OnUpdateProcessExec`（要件 1.12・1.13）。
    if order.reason == UpdateReason::Manual {
        match send(
            ports,
            ON_UPDATE_PROCESS_EXEC,
            refs::process_exec_refs(order.reason),
        ) {
            Raised::Script => {
                tracing::info!(
                    event = "update_process_exec",
                    answered = true,
                    "[update] ゴーストが更新を引き受けたので、標準の手続きを行いません"
                );
                return end;
            }
            Raised::Closed => {
                tracing::warn!(
                    event = "update_abandoned",
                    at = "process_exec",
                    "[update] ゴーストが居ないので、更新をやめます"
                );
                return end;
            }
            Raised::NoReply => {
                tracing::info!(
                    event = "update_process_exec",
                    answered = false,
                    "[update] 返事が無いので、標準の手続きへ進みます"
                );
            }
        }
    }
    ports.standard_started();

    let Some(resources) = ports.resources() else {
        tracing::warn!(
            event = "update_abandoned",
            at = "resources",
            "[update] リソースを照会できないので、更新をやめます"
        );
        return end;
    };
    tracing::info!(
        event = "update_resources",
        homeurl = resources.homeurl.as_deref().unwrap_or_default(),
        useorigin1 = resources.useorigin1.as_deref().unwrap_or_default(),
        "[update] リソースを照会しました"
    );
    let numbering = Numbering::from_useorigin1(resources.useorigin1.as_deref());

    for spec in &order.targets {
        let target_end = run_target(spec, order.reason, &resources, numbering, ports);
        let abandoned = target_end == TargetEnd::Abandoned;
        end.ends.push(target_end);
        if abandoned {
            tracing::warn!(
                event = "update_abandoned",
                at = "target",
                target = %spec.dir.display(),
                "[update] 途中でゴーストが居なくなったので、残りの対象・総括・読み直しを送りません"
            );
            return end;
        }
    }

    // 総括（飛ばした対象は載せない・全部飛ばせば送らない。要件 3・1.10）。
    let pairs: Vec<(&TargetSpec, &TargetEnd)> = order.targets.iter().zip(&end.ends).collect();
    let summary = refs::summary_refs(order.summary, &pairs);
    if summary.is_empty() {
        tracing::info!(
            event = "update_summary",
            sent = false,
            "[update] 更新した対象が無いので、総括を送りません"
        );
        return end;
    }
    let id = match order.summary {
        SummaryKind::Result => ON_UPDATE_RESULT,
        SummaryKind::ResultEx => ON_UPDATE_RESULT_EX,
    };
    tracing::info!(event = "update_summary", sent = true, id, references = ?summary, "[update] 総括を送ります");
    if send(ports, id, summary) == Raised::Closed {
        tracing::warn!(
            event = "update_abandoned",
            at = "summary",
            "[update] 総括の後にゴーストが居ないので、読み直しを頼みません"
        );
        return end;
    }
    end.summarised = true;

    // 読み直しは changed が 1 つでもあるときだけ（要件 5.2・5.4）。
    if end.ends.iter().any(|e| matches!(e, TargetEnd::Changed(_))) {
        ports.request_reload(&order.ghost_dir);
        end.reload_requested = true;
        tracing::info!(
            event = "update_reload_requested",
            ghost_dir = %order.ghost_dir.display(),
            "[update] 同じゴーストの読み直しを頼みました"
        );
    }
    end
}

/// 対象 1 つの一周（更新先を解く → 始まり → エンジン → 締め 1 件）。
fn run_target(
    spec: &TargetSpec,
    reason: UpdateReason,
    resources: &GhostResources,
    numbering: Numbering,
    ports: &dyn UpdatePorts,
) -> TargetEnd {
    let tail = Tail::new(spec.kind, reason);
    let names = EventNames::for_kind(spec.kind);

    // ゴーストは照会の値 → descript.txt、他は descript.txt（要件 1.9）。
    let resource = match spec.kind {
        TargetKind::Ghost => resources
            .homeurl
            .as_deref()
            .filter(|url| !url.trim().is_empty()),
        TargetKind::Shell | TargetKind::Balloon => None,
    };
    let Some(homeurl) = resource.or(spec.descript_homeurl.as_deref()) else {
        tracing::warn!(
            event = "update_target_skipped",
            reason = "no_homeurl",
            kind = tail.kind,
            name = %spec.name,
            dir = %spec.dir.display(),
            "[update] 更新先が無いので、この対象を飛ばします"
        );
        return TargetEnd::Skipped;
    };
    tracing::info!(
        event = "update_target_begin",
        kind = tail.kind,
        name = %spec.name,
        homeurl,
        dir = %spec.dir.display(),
        "[update] 対象の更新を始めます"
    );

    // Reference1 は絶対パス（要件 2.4）。
    let dir = std::path::absolute(&spec.dir).unwrap_or_else(|error| {
        tracing::warn!(
            event = "update_absolute_failed",
            dir = %spec.dir.display(),
            %error,
            "[update] フォルダを絶対パスにできないので、そのまま載せます"
        );
        spec.dir.clone()
    });
    if send(ports, names.begin, refs::begin_refs(&spec.name, &dir, tail)) == Raised::Closed {
        return done(spec, TargetEnd::Abandoned);
    }

    // 観測の中で進捗をイベントへ写して送る。閉じた後は送らない（要件 5.7）。
    let mut closed = false;
    let mut observe = |p: &Progress| {
        tracing::debug!(event = "update_progress", progress = ?p, "[update] 進捗");
        for (id, references) in refs::progress_events(p, names, numbering, tail) {
            if closed {
                tracing::warn!(
                    event = "update_event_dropped",
                    id,
                    "[update] ゴーストが居ないので、進捗のイベントを送りません"
                );
            } else if send(ports, id, references) == Raised::Closed {
                closed = true;
            }
        }
    };
    let run = ports.run_engine(homeurl, &spec.dir, &mut observe);

    let (end, closing) = match run {
        EngineRun::Closed => return done(spec, TargetEnd::Abandoned),
        EngineRun::Unavailable(error) => {
            tracing::error!(
                event = "update_fetch_unavailable",
                homeurl,
                target = %spec.dir.display(),
                %error,
                "[update] 取得の部品を用意できないので、更新できません"
            );
            let word = "connect";
            (
                failed(word, None),
                (names.failure, refs::failure_refs(word, None, tail)),
            )
        }
        EngineRun::Done(Ok(outcome)) => {
            let end = match &outcome {
                UpdateOutcome::Unchanged { .. } => TargetEnd::Unchanged,
                UpdateOutcome::Updated {
                    placed,
                    undeletable,
                    leftovers,
                    ..
                } => {
                    for item in undeletable {
                        tracing::warn!(event = "update_leftover", homeurl, path = %item.path.display(), error = %item.source, "[update] 更新はできましたが、消せなかった物が残っています");
                    }
                    for path in leftovers {
                        tracing::warn!(event = "update_leftover", homeurl, path = %path.display(), "[update] 更新はできましたが、片付けられなかった作業場所が残っています");
                    }
                    TargetEnd::Changed(placed.len())
                }
            };
            (end, (names.complete, refs::complete_refs(&outcome, tail)))
        }
        EngineRun::Done(Err(error)) => {
            let word = refs::failure_word(&error.reason);
            let file = error.file();
            tracing::error!(
                event = "update_failed",
                homeurl = error.homeurl.as_str(),
                target = %error.target.display(),
                stage = ?error.stage,
                kind = error.reason.kind(),
                file = file.unwrap_or_default(),
                rolled_back = error.rolled_back(),
                work = error.work.as_ref().map(|w| w.display().to_string()).unwrap_or_default(),
                word = %word,
                "[update] 対象を更新できませんでした"
            );
            let references = refs::failure_refs(&word, file, tail);
            (failed(&word, file), (names.failure, references))
        }
    };
    if closed {
        return done(spec, TargetEnd::Abandoned);
    }
    if send(ports, closing.0, closing.1) == Raised::Closed {
        return done(spec, TargetEnd::Abandoned);
    }
    done(spec, end)
}

fn failed(word: &str, file: Option<&str>) -> TargetEnd {
    TargetEnd::Failed {
        word: word.to_owned(),
        file: file.map(str::to_owned),
    }
}

/// 対象の終わり方を記録して返す。
fn done(spec: &TargetSpec, end: TargetEnd) -> TargetEnd {
    tracing::info!(event = "update_target_done", name = %spec.name, end = ?end, "[update] 対象の更新を終えました");
    end
}

/// イベントを送って応えを記録する。
fn send(ports: &dyn UpdatePorts, id: &'static str, references: Vec<String>) -> Raised {
    let raised = ports.raise(id, references);
    tracing::info!(event = "update_event", id, raised = ?raised, "[update] イベントを送りました");
    raised
}

#[cfg(test)]
#[path = "procedure_test_support.rs"]
mod procedure_test_support;

#[cfg(test)]
#[path = "procedure_tests.rs"]
mod procedure_tests;
