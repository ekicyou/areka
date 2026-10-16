//! 並び・見つからない根・隠れの記録（design.md「RootsReport」・要件 1.15・2.5・8.3・9.1）。
//!
//! 記録は起動と並びの変更のときだけ残す（要件 9.3）。判断の関数は失敗を値で返し、
//! 記録はここで残す（起動が失敗した値からも控えと「既定へ倒した」を warn に出す）。

use areka_ghost::{Hidden, StoreKind};

use super::resolve::{ResolvedRoots, RootError, RootSkip, RootsOrigin};

/// 記録の場面（起動か並びの変更か）。どの行にも `occasion` 欄で添える。
#[derive(Clone, Copy, Debug)]
pub(crate) enum ReportOccasion {
    Boot,
    #[allow(dead_code)] // 使い手は並びの変更の適用（tasks.md 5.3）
    Change,
}

impl ReportOccasion {
    fn as_str(self) -> &'static str {
        match self {
            Self::Boot => "boot",
            Self::Change => "change",
        }
    }
}

fn origin_str(origin: RootsOrigin) -> &'static str {
    match origin {
        RootsOrigin::EnvVar => "env_var",
        RootsOrigin::Memory => "memory",
        RootsOrigin::Default => "default",
    }
}

/// 並びの 1 根ごとに info（絶対パス・出どころ・見つかったか）、見つからない根は warn（9.1）、
/// 控え（`RootSkip`）と `memory_fell_back` は warn。
pub(crate) fn log_roots(resolved: &ResolvedRoots, occasion: ReportOccasion) {
    let occ = occasion.as_str();
    let origin = origin_str(resolved.origin);
    for s in &resolved.status {
        tracing::info!(
            event = "root_listed",
            path = %s.dir.display(),
            origin,
            found = s.found,
            occasion = occ,
            "[RootsReport] ベースウェアの根"
        );
        if !s.found {
            tracing::warn!(
                event = "root_missing",
                path = %s.dir.display(),
                origin,
                occasion = occ,
                "[RootsReport] 根が見つからないので飛ばす"
            );
        }
    }
    log_notes(&resolved.skipped, resolved.memory_fell_back, occasion);
}

/// 並びが決まらなかった値から、控えと「既定へ倒した」を warn に出す（要件 1.7・1.9・1.12・1.13）。
/// 失敗そのものの告知は呼び手が残す。
pub(crate) fn log_root_error(err: &RootError, occasion: ReportOccasion) {
    let (RootError::ExeLocationUnavailable {
        skipped,
        memory_fell_back,
        ..
    }
    | RootError::EnvAllEmpty {
        skipped,
        memory_fell_back,
    }
    | RootError::NoneFound {
        skipped,
        memory_fell_back,
        ..
    }) = err;
    log_notes(skipped, *memory_fell_back, occasion);
}

fn log_notes(skipped: &[RootSkip], memory_fell_back: bool, occasion: ReportOccasion) {
    let occ = occasion.as_str();
    for skip in skipped {
        match skip {
            RootSkip::Empty { index } => tracing::warn!(
                event = "root_skipped",
                reason = "empty",
                index,
                occasion = occ,
                "[RootsReport] AREKA_ROOT の空の項目を飛ばした"
            ),
            RootSkip::Unreadable { item, reason } => tracing::warn!(
                event = "root_skipped",
                reason = *reason,
                item = item.as_str(),
                occasion = occ,
                "[RootsReport] 記憶の読めない項目を飛ばした"
            ),
            RootSkip::Duplicate { dir, kept } => tracing::warn!(
                event = "root_skipped",
                reason = "duplicate",
                path = %dir.display(),
                kept = %kept.display(),
                occasion = occ,
                "[RootsReport] 上に同じ根があるので飛ばした"
            ),
        }
    }
    if memory_fell_back {
        tracing::warn!(
            event = "roots_memory_fell_back",
            occasion = occ,
            "[RootsReport] 記憶の並びに読める項目が無いので既定へ倒した"
        );
    }
}

/// 隠れを 1 件 1 行で info（種類・フォルダ名・隠れた場所・隠した場所）＋件数を info（0 件も・要件 2.5）。
pub(crate) fn log_hidden(hidden: &[Hidden], occasion: ReportOccasion) {
    let occ = occasion.as_str();
    for h in hidden {
        let kind = match h.kind {
            StoreKind::Ghost => "ghost",
            StoreKind::Balloon => "balloon",
        };
        tracing::info!(
            event = "root_hidden_item",
            kind,
            folder = h.folder.as_str(),
            hidden = %h.hidden.display(),
            by = %h.by.display(),
            occasion = occ,
            "[RootsReport] 上の根の同じ名前に隠れた"
        );
    }
    tracing::info!(
        event = "roots_hidden_count",
        count = hidden.len(),
        occasion = occ,
        "[RootsReport] 隠れの件数"
    );
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
