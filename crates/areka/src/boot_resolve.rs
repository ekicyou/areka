//! 起動解決の純粋な判断（areka-P0-baseware-root-layout 要件 4・5）。
//!
//! 起動するゴースト（6 分岐）とバルーン（7 分岐）を「argv・記憶の値・同梱の名・列挙の名前」
//! だけから決める。判断は I/O・時計・乱数源を持たず、無作為は「候補数 → 添字」の関数を
//! 注入する（本番は [`pick_index`]）。列挙の並びは判断に使わない（裁定 3）。
//!
//! 既定の定数 [`DEFAULT_GHOST_FOLDER`]／[`DEFAULT_BALLOON_FOLDER`] はこのファイルだけが持ち、
//! 参照はそれぞれゴーストの段 4・バルーンの段 5 の 1 か所だけ（要件 4.11・5.9）。

use std::hash::{BuildHasher, RandomState};
use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoot;

// 消費者（`main` の起動解決）は task 5.1・5.3 で結線する。それまでは檻だけが呼ぶので、
// 本番ビルドの dead_code を各項目に限って許す（5.1 で外す）。

/// 既定ゴースト（要件 4.4・4.11。配布物に必ず同梱＝裁定 3）。
#[allow(dead_code)]
pub(crate) const DEFAULT_GHOST_FOLDER: &str = "emo2";
/// 既定バルーン（要件 5.5・5.9。フォルダ名と id はバイト一致＝完了 spec で実測済み）。
#[allow(dead_code)]
pub(crate) const DEFAULT_BALLOON_FOLDER: &str = "StayseeBalloon";

/// ゴーストが決まった経路（要件 4.10 の記録に載せる）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GhostRoute {
    Argv,
    Memory,
    Only,
    Default,
    Random,
}

/// バルーンが決まった経路（要件 5.11 の記録に載せる）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BalloonRoute {
    Argv,
    Memory,
    Companion,
    Only,
    Default,
    Random,
}

/// 決まったゴースト（argv なら渡されたパスそのもの・それ以外は `<根>/ghost/<folder>`）。
/// argv の経路だけ `folder` が `None`（根の外かもしれないので名前を持たない）。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GhostDecision {
    pub route: GhostRoute,
    pub dir: PathBuf,
    pub folder: Option<String>,
}

/// 決まったバルーン（argv なら渡されたパスそのもの・それ以外は `<根>/balloon/<folder>`）。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BalloonDecision {
    pub route: BalloonRoute,
    pub dir: PathBuf,
    pub folder: Option<String>,
}

/// 0 体（要件 4.7）。告知の文面に置くべき場所を載せるため格納フォルダを持つ。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoGhost {
    pub ghost_store: PathBuf,
}

/// 0（要件 5.8）。告知の文面に置くべき場所を載せるため格納フォルダを持つ。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoBalloon {
    pub balloon_store: PathBuf,
}

/// ゴーストの判断の入力（要件 4.9「argv・記憶の値・列挙の結果だけ」）。
/// `argv` があるとき `memory`／`listed` は読まない（空でよい）。
#[allow(dead_code)]
pub(crate) struct GhostInputs<'a> {
    pub root: &'a BasewareRoot,
    /// argv[1]
    pub argv: Option<&'a Path>,
    /// App スコープ `areka.last.ghost`
    pub memory: Option<&'a str>,
    /// `list_ghosts` の folder（昇順）
    pub listed: &'a [String],
}

/// バルーンの判断の入力。`argv` があるとき他は読まない（空でよい）。
#[allow(dead_code)]
pub(crate) struct BalloonInputs<'a> {
    pub root: &'a BasewareRoot,
    /// argv[2]
    pub argv: Option<&'a Path>,
    /// 起動するゴーストの Ghost スコープ `areka.last.balloon`
    pub memory: Option<&'a str>,
    /// `<ゴースト>/install.txt` の `balloon.directory`
    pub companion: Option<&'a str>,
    /// `list_balloons` の folder（昇順）
    pub listed: &'a [String],
}

/// `name` が列挙に在ればその名を返す（在る・無いの判定だけ。並びは見ない）。
fn find<'a>(listed: &'a [String], name: &str) -> Option<&'a str> {
    listed.iter().find(|f| *f == name).map(String::as_str)
}

/// 段 1〜5＋0 体（要件 4.1〜4.7）。`pick` は「候補数 n（≥ 2）→ 0..n の添字」。純粋。
#[allow(dead_code)]
pub(crate) fn resolve_ghost(
    inputs: &GhostInputs<'_>,
    pick: impl FnOnce(usize) -> usize,
) -> Result<GhostDecision, NoGhost> {
    // 段 1: argv（開発者の上書き・記憶と列挙は見ない）。
    if let Some(argv) = inputs.argv {
        return Ok(GhostDecision {
            route: GhostRoute::Argv,
            dir: argv.to_path_buf(),
            folder: None,
        });
    }
    let listed = inputs.listed;
    let at = |route, folder: &str| GhostDecision {
        route,
        dir: inputs.root.ghost_dir(folder),
        folder: Some(folder.to_owned()),
    };
    // 段 2: 記憶（列挙に在れば）。在らねば黙って読み替えず warn して次へ（要件 4.6）。
    if let Some(memory) = inputs.memory {
        match find(listed, memory) {
            Some(folder) => return Ok(at(GhostRoute::Memory, folder)),
            None => tracing::warn!(
                event = "last_ghost_not_found",
                memory,
                ghost_store = %inputs.root.ghost_store().display(),
                "[boot_resolve] 前回のゴーストが根に見つからないので次の候補へ進みます"
            ),
        }
    }
    match listed {
        // 0 体（要件 4.7）。
        [] => Err(NoGhost {
            ghost_store: inputs.root.ghost_store(),
        }),
        // 段 3: 唯一。
        [only] => Ok(at(GhostRoute::Only, only)),
        _ => {
            // 段 4: 既定（DEFAULT_GHOST_FOLDER を参照するのはこの段だけ）。
            if let Some(folder) = find(listed, DEFAULT_GHOST_FOLDER) {
                return Ok(at(GhostRoute::Default, folder));
            }
            // 段 5: 無作為。
            let folder = listed[pick(listed.len())].as_str();
            tracing::info!(
                event = "ghost_picked_randomly",
                folder,
                candidates = listed.len(),
                "[boot_resolve] 既定のゴーストが無いので無作為に選びました"
            );
            Ok(at(GhostRoute::Random, folder))
        }
    }
}

/// 段 1〜6＋0（要件 5.1〜5.8）。`pick` は「候補数 n（≥ 2）→ 0..n の添字」。純粋。
#[allow(dead_code)]
pub(crate) fn resolve_balloon(
    inputs: &BalloonInputs<'_>,
    pick: impl FnOnce(usize) -> usize,
) -> Result<BalloonDecision, NoBalloon> {
    // 段 1: argv（開発者の上書き・以下を見ない）。
    if let Some(argv) = inputs.argv {
        return Ok(BalloonDecision {
            route: BalloonRoute::Argv,
            dir: argv.to_path_buf(),
            folder: None,
        });
    }
    let listed = inputs.listed;
    let at = |route, folder: &str| BalloonDecision {
        route,
        dir: inputs.root.balloon_dir(folder),
        folder: Some(folder.to_owned()),
    };
    // 段 2: ゴーストごとの記憶（同梱より先＝裁定 4）。
    if let Some(memory) = inputs.memory {
        match find(listed, memory) {
            Some(folder) => return Ok(at(BalloonRoute::Memory, folder)),
            None => tracing::warn!(
                event = "last_balloon_not_found",
                memory,
                balloon_store = %inputs.root.balloon_store().display(),
                "[boot_resolve] 前回のバルーンが根に見つからないので次の候補へ進みます"
            ),
        }
    }
    // 段 3: ゴーストの同梱（install.txt の balloon.directory）。
    if let Some(companion) = inputs.companion {
        match find(listed, companion) {
            Some(folder) => return Ok(at(BalloonRoute::Companion, folder)),
            None => tracing::warn!(
                event = "companion_balloon_not_found",
                companion,
                balloon_store = %inputs.root.balloon_store().display(),
                "[boot_resolve] ゴーストの同梱バルーンが根に見つからないので次の候補へ進みます"
            ),
        }
    }
    match listed {
        // 0（要件 5.8）。
        [] => Err(NoBalloon {
            balloon_store: inputs.root.balloon_store(),
        }),
        // 段 4: 唯一。
        [only] => Ok(at(BalloonRoute::Only, only)),
        _ => {
            // 段 5: 既定（DEFAULT_BALLOON_FOLDER を参照するのはこの段だけ）。
            if let Some(folder) = find(listed, DEFAULT_BALLOON_FOLDER) {
                return Ok(at(BalloonRoute::Default, folder));
            }
            // 段 6: 無作為。
            let folder = listed[pick(listed.len())].as_str();
            tracing::info!(
                event = "balloon_picked_randomly",
                folder,
                candidates = listed.len(),
                "[boot_resolve] 既定のバルーンが無いので無作為に選びました"
            );
            Ok(at(BalloonRoute::Random, folder))
        }
    }
}

/// 本番の添字（std の `RandomState` のプロセスごとの鍵から。新規依存 0）。
/// 質は問わない（初回起動の 1 回だけ・その後は記憶で固定＝design の Risks）。`n == 0` は呼ばれない。
#[allow(dead_code)]
pub(crate) fn pick_index(n: usize) -> usize {
    (RandomState::new().hash_one(n) % n as u64) as usize
}

#[cfg(test)]
#[path = "boot_resolve_tests.rs"]
mod boot_resolve_tests;
