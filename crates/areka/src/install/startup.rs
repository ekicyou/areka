//! 起動の依頼の持ち物（`--install` と exe の隣の `initial_install.nar`・spec: startup-install-option）。
//!
//! このファイルの純粋な判断（消す書庫・`callghost` を頼むか・頼み方）は World にもファイルにも触らない。

use std::path::{Path, PathBuf};

use areka_ghost::same_folder_name;
use areka_kanade::ChangeOrigin;

use super::procedure::ArchiveEnd;
use crate::emo2_boot::ghost_switch::{GhostSpec, SwitchRequest};

/// 純粋: 消す書庫（`Installed` のパスだけ・重ねは最初の 1 つ・書いた順＝要件 3.1・3.2・4.3・4.4）。
///
/// `ends` が短い（途中でやめた）とき、終わり方の無い書庫は「まだ扱っていない」＝消さない。
#[allow(dead_code)] // 読み手は同じ spec の on_order_done（タスク 3.3）
pub(crate) fn sources_to_delete(archives: &[PathBuf], ends: &[ArchiveEnd]) -> Vec<PathBuf> {
    let key = |path: &Path| path.to_string_lossy().to_lowercase();
    let mut out: Vec<PathBuf> = Vec::new();
    for (index, archive) in archives.iter().enumerate() {
        let lower = key(archive);
        if out.iter().any(|done| key(done) == lower) {
            continue;
        }
        // 同じパスのどれか 1 つでも入ったなら、最初の綴りで 1 回だけ。
        let installed = archives
            .iter()
            .zip(ends)
            .skip(index)
            .any(|(other, end)| key(other) == lower && matches!(end, ArchiveEnd::Installed(_)));
        if installed {
            out.push(archive.clone());
        }
    }
    out
}

/// `callghost` の判断に要る事実（集めるのは on_order_done）。
#[allow(dead_code)] // 作り手は同じ spec の on_order_done（タスク 3.3）
pub(crate) struct CallGhostFacts<'a> {
    pub archives: usize,
    pub ends: &'a [ArchiveEnd],
    /// 窓口の門が閉じている（終了が始まっている）。
    pub closing: bool,
    /// 切替の予約 `SwitchInFlight` が在る。
    pub switching: bool,
    /// 依頼を積んだとき・終わりを受けたときの切り替えの受付の数（`SwitchAcceptCount`）。
    pub switches_at_submit: u64,
    pub switches_now: u64,
    /// 今のゴーストのフォルダ名（`AlreadyRunning` の比べ）。
    pub ghost_now: Option<&'a str>,
}

/// `callghost` の判断の答え。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallGhost {
    /// 切り替え先のフォルダ名（最後に入ったゴーストの書庫の本体の `place` の末尾）。
    Request {
        folder: String,
    },
    Skip(CallGhostSkip),
}

/// 頼まない理由（記録 `startup_callghost_skipped` の `reason`）。
#[allow(dead_code)] // 理由の語へ下ろす読み手は同じ spec の on_order_done（タスク 3.3）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallGhostSkip {
    Abandoned,
    Closing,
    NoGhostInstalled,
    Switching,
    SwitchedMeanwhile,
    AlreadyRunning,
}

/// 純粋: `callghost` を頼むか（design「`callghost` の判断」の順＝要件 5.1〜5.4）。
#[allow(dead_code)] // 読み手は同じ spec の on_order_done（タスク 3.3）
pub(crate) fn decide_call_ghost(facts: &CallGhostFacts<'_>) -> CallGhost {
    let ends = facts.ends;
    if ends.len() < facts.archives || ends.contains(&ArchiveEnd::Abandoned) {
        return CallGhost::Skip(CallGhostSkip::Abandoned);
    }
    if facts.closing {
        return CallGhost::Skip(CallGhostSkip::Closing);
    }
    let folder = ends.iter().rev().find_map(|end| match end {
        ArchiveEnd::Installed(items) if items.first().is_some_and(|i| i.kind == "ghost") => {
            Path::new(&items[0].place)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        }
        _ => None,
    });
    let Some(folder) = folder else {
        return CallGhost::Skip(CallGhostSkip::NoGhostInstalled);
    };
    if facts.switching {
        return CallGhost::Skip(CallGhostSkip::Switching);
    }
    if facts.switches_now != facts.switches_at_submit {
        return CallGhost::Skip(CallGhostSkip::SwitchedMeanwhile);
    }
    if facts
        .ghost_now
        .is_some_and(|now| same_folder_name(now, &folder))
    {
        return CallGhost::Skip(CallGhostSkip::AlreadyRunning);
    }
    CallGhost::Request { folder }
}

/// 純粋: メニューの「ゴースト」と同じ形の頼み（Folder・raise_event 真・Manual・boot_event なし＝要件 5.1・10.1）。
#[allow(dead_code)] // 読み手は同じ spec の on_order_done（タスク 3.3）
pub(crate) fn callghost_request(folder: String) -> SwitchRequest {
    SwitchRequest {
        ghost: GhostSpec::Folder(folder),
        raise_event: true,
        origin: ChangeOrigin::Manual,
        boot_event: None,
    }
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
