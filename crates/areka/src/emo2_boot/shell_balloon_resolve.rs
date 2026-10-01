//! シェル・バルーンの切替先の名前の解決と、インストールの控え（spec: areka-P0-shell-balloon-switch
//! 要件 1.6〜1.11・1.13・12.3〜12.5・design「ShellBalloonSwitch」）。
//!
//! 入口（`shell_balloon_switch.rs`）が長くなるので、解決の純関数と控えをこちらへ分けた
//! （design「File Structure Plan」の `shell_balloon_resolve.rs`）。
//! - 候補: シェルは今のゴーストの `shell/` の下の全部（`menu,hidden` を含む）、バルーンは根の目録。
//!   今のものも候補に入る（自分自身への切替は作り直す＝要件 1.8・裁定 5）。
//! - 照合の順: `random`・`lastinstalled` を先に解き、次に `name` → フォルダ名（大文字小文字を区別）。
//!   メニューのフォルダの名指しはフォルダ名とだけ突き合わせる（特別な名前は解かない）。
//! - 控え: このプロセスで最後に入れたシェル・バルーン（プロセスの中だけ・ファイルへは書かない）。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoot;
use areka_ghost::catalog::{list_all_shells, list_balloons, list_shells};
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;

use super::shell_balloon_switch::{SkinKind, SkinSpec};

/// 切替先の候補 1 つ（目録の項目の写し）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkinCandidate {
    /// シェル／バルーンのフォルダの絶対パス。
    pub dir: PathBuf,
    /// フォルダ名（目録の綴り）。
    pub folder: String,
    /// `descript.txt` の `name`（無ければ無し）。
    pub name: Option<String>,
    /// `menu,hidden` のシェルか（`random` の候補から外す・バルーンは常に偽）。
    pub hidden: bool,
}

/// 切替先が決まらない理由（入口の `warn!(skin_switch_unknown)` の `reason` 欄）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotFoundReason {
    /// 名前もフォルダ名も候補のどれにも一致しない。
    NoMatch,
    /// `random` だが候補が無く、今のものも目録に無い。
    RandomEmpty,
    /// `lastinstalled` だが、このプロセスで入れていない。
    LastInstalledNone,
    /// シェルの `lastinstalled` だが、最後に入れたシェルは今のゴーストのものではない。
    LastInstalledOtherGhost,
    /// `lastinstalled` だが、最後に入れたものが候補に無い。
    LastInstalledMissing,
}

/// このプロセスで最後に入れたシェル（入れた先のゴーストのフォルダ名とシェルのフォルダ名）。
///
/// プロセスの中だけ（`LastInstalledGhost` と同じ形）。入口は読むだけで、切替に使っても消さない。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub(crate) struct LastInstalledShell {
    pub ghost_folder: Option<String>,
    pub folder: String,
}

/// このプロセスで最後に入れたバルーンのフォルダ名（目録の綴り・プロセスの中だけ）。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub(crate) struct LastInstalledBalloon(pub String);

/// シェルのインストールの完了で控えを置き換える（最後の 1 件だけ残る・要件 1.10・裁定 3）。
pub(crate) fn record_installed_shell(
    world: &mut World,
    ghost_folder: Option<String>,
    folder: String,
) {
    tracing::info!(
        event = "last_installed_shell_recorded",
        ghost = ?ghost_folder,
        folder = %folder,
        "最後に入れたシェルを控えた"
    );
    world.insert_resource(LastInstalledShell {
        ghost_folder,
        folder,
    });
}

/// バルーンのインストールの完了で控えを置き換える（要件 1.11）。
pub(crate) fn record_installed_balloon(world: &mut World, folder: String) {
    tracing::info!(
        event = "last_installed_balloon_recorded",
        folder = %folder,
        "最後に入れたバルーンを控えた"
    );
    world.insert_resource(LastInstalledBalloon(folder));
}

/// 今のゴースト（`ghost_dir`）の `shell/` の下の全シェル（隠しを含む・隠しには印）。
///
/// 隠しの印は「メニュー向けの列挙 `list_shells` に居ないこと」で付ける（目録の素性の型に欄を足さない）。
pub(crate) fn shell_candidates(ghost_dir: &Path) -> Vec<SkinCandidate> {
    let shown: Vec<String> = list_shells(ghost_dir)
        .into_iter()
        .map(|e| e.identity.folder)
        .collect();
    list_all_shells(ghost_dir)
        .into_iter()
        .map(|e| SkinCandidate {
            hidden: !shown.contains(&e.identity.folder),
            dir: e.dir,
            folder: e.identity.folder,
            name: e.identity.name,
        })
        .collect()
}

/// 根の `balloon/` の目録のバルーン。
pub(crate) fn balloon_candidates(root: &BasewareRoot) -> Vec<SkinCandidate> {
    list_balloons(root)
        .into_iter()
        .map(|e| SkinCandidate {
            dir: e.dir,
            folder: e.identity.folder,
            name: e.identity.name,
            hidden: false,
        })
        .collect()
}

/// `lastinstalled` の控えを読む（読むだけ・消さない）。
///
/// シェルは控えのゴーストが今のゴースト（`ghost_dir` のフォルダ名・大文字小文字を区別しない＝
/// インストールの記録と同じ比較）のときだけフォルダ名を返し、別のゴーストなら該当なし（ゴースト切替は
/// 起こさない＝要件 1.10・裁定 3）。
pub(crate) fn installed_for(
    world: &World,
    kind: SkinKind,
    ghost_dir: &Path,
) -> Result<String, NotFoundReason> {
    match kind {
        SkinKind::Shell => {
            let memo = world
                .get_resource::<LastInstalledShell>()
                .ok_or(NotFoundReason::LastInstalledNone)?;
            let here = ghost_dir.file_name().and_then(|n| n.to_str());
            match (memo.ghost_folder.as_deref(), here) {
                (Some(memo_ghost), Some(here)) if memo_ghost.eq_ignore_ascii_case(here) => {
                    Ok(memo.folder.clone())
                }
                _ => Err(NotFoundReason::LastInstalledOtherGhost),
            }
        }
        SkinKind::Balloon => world
            .get_resource::<LastInstalledBalloon>()
            .map(|memo| memo.0.clone())
            .ok_or(NotFoundReason::LastInstalledNone),
    }
}

/// 純粋: 候補と指し方から切替先を決める（要件 1.6〜1.11）。
///
/// `current`＝今のシェル／バルーンのフォルダ名、`installed`＝[`installed_for`] の答え（`lastinstalled`
/// のときだけ読む）、`pick`＝乱数（本番は `boot_resolve::pick_index`・候補が 1 つ以上のときだけ呼ぶ）。
/// `random`・`lastinstalled` は同名の候補より先に解く。`lastinstalled` の控えは候補のフォルダ名と
/// 大文字小文字を区別せずに突き合わせる（控えはインストールの綴り）。fs・World は読まない。
pub(crate) fn resolve_skin_target(
    candidates: &[SkinCandidate],
    spec: &SkinSpec,
    current: Option<&str>,
    installed: Result<&str, NotFoundReason>,
    pick: impl FnOnce(usize) -> usize,
) -> Result<SkinCandidate, NotFoundReason> {
    let by_folder = |want: &str| candidates.iter().find(|c| c.folder == want);
    let hit = match spec {
        SkinSpec::Folder(want) => by_folder(want),
        SkinSpec::Name(name) if name == "random" => {
            return pick_random(candidates, current, pick);
        }
        SkinSpec::Name(name) if name == "lastinstalled" => {
            let want = installed?;
            return candidates
                .iter()
                .find(|c| c.folder.eq_ignore_ascii_case(want))
                .cloned()
                .ok_or(NotFoundReason::LastInstalledMissing);
        }
        SkinSpec::Name(want) => candidates
            .iter()
            .find(|c| c.name.as_deref() == Some(want.as_str()))
            .or_else(|| by_folder(want)),
    };
    hit.cloned().ok_or(NotFoundReason::NoMatch)
}

/// `random`: 隠しと今のものを除いて 1 つ選ぶ。候補 0 なら今のもの（要件 1.9・裁定 4）。
fn pick_random(
    candidates: &[SkinCandidate],
    current: Option<&str>,
    pick: impl FnOnce(usize) -> usize,
) -> Result<SkinCandidate, NotFoundReason> {
    let is_current = |c: &SkinCandidate| Some(c.folder.as_str()) == current;
    let others: Vec<&SkinCandidate> = candidates
        .iter()
        .filter(|c| !c.hidden && !is_current(c))
        .collect();
    let chosen = if others.is_empty() {
        candidates
            .iter()
            .find(|c| is_current(c))
            .ok_or(NotFoundReason::RandomEmpty)?
    } else {
        others[pick(others.len())]
    };
    tracing::info!(
        event = "skin_switch_random_pick",
        to = %chosen.folder,
        candidates = others.len(),
        "random の切替先を選んだ"
    );
    Ok(chosen.clone())
}

#[cfg(test)]
#[path = "shell_balloon_resolve_tests.rs"]
mod tests;
