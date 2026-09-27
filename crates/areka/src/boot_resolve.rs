//! 起動解決の純粋な判断（areka-P0-baseware-root-layout 要件 4・5）。
//!
//! 起動するゴースト（6 分岐）とバルーン（7 分岐）を「argv・記憶の値・同梱の名・列挙の名前」
//! だけから決める。判断は I/O・時計・乱数源を持たず、無作為は「候補数 → 添字」の関数を
//! 注入する（本番は [`pick_index`]）。列挙の並びは判断に使わない（裁定 3）。
//!
//! 既定の定数 [`DEFAULT_GHOST_FOLDER`]／[`DEFAULT_BALLOON_FOLDER`] はこのファイルだけが持ち、
//! 解決の判断での参照はそれぞれゴーストの段 4・バルーンの段 5 の 1 か所だけ（要件 4.11・5.9）。
//! 判断の外では、切替の降ろした直後の記憶 [`write_switch_drop`] が既定ゴーストを最後のゴーストの
//! 書き先に使う（要件 12.6）。

use std::hash::{BuildHasher, RandomState};
use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoot;
use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{
    PersistKey, PersistOutcome, PersistScope, ScopeRoots, SylphyaPublisher, load_scope, save_scope,
};

// 判断と記憶の直読みの消費者は起動前の解決（`boot_config::resolve_boot`）。記憶の書き込み
// （[`LastUsed`]）の消費者は boot 成功直後の `main::on_boot_ok`。

/// 既定ゴースト（要件 4.4・4.11。配布物に必ず同梱＝裁定 3）。
pub(crate) const DEFAULT_GHOST_FOLDER: &str = "emo2";
/// 既定バルーン（要件 5.5・5.9。フォルダ名と id はバイト一致＝完了 spec で実測済み）。
pub(crate) const DEFAULT_BALLOON_FOLDER: &str = "StayseeBalloon";

/// ゴーストが決まった経路（要件 4.10 の記録に載せる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GhostRoute {
    Argv,
    Memory,
    Only,
    Default,
    Random,
    /// 実行中の切替で決まった（記憶を書く経路＝要件 4.6。書かないのは `Argv` だけ）。
    Switched,
}

/// バルーンが決まった経路（要件 5.11 の記録に載せる）。
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GhostDecision {
    pub route: GhostRoute,
    pub dir: PathBuf,
    pub folder: Option<String>,
}

/// 決まったバルーン（argv なら渡されたパスそのもの・それ以外は `<根>/balloon/<folder>`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BalloonDecision {
    pub route: BalloonRoute,
    pub dir: PathBuf,
    pub folder: Option<String>,
}

/// 0 体（要件 4.7）。告知の文面に置くべき場所を載せるため格納フォルダを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoGhost {
    pub ghost_store: PathBuf,
}

/// 0（要件 5.8）。告知の文面に置くべき場所を載せるため格納フォルダを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoBalloon {
    pub balloon_store: PathBuf,
}

/// ゴーストの判断の入力（要件 4.9「argv・記憶の値・列挙の結果だけ」）。
/// `argv` があるとき `memory`／`listed` は読まない（空でよい）。
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
pub(crate) fn pick_index(n: usize) -> usize {
    (RandomState::new().hash_one(n) % n as u64) as usize
}

// ---------------------------------------------------------------- 記憶の直読みと書き込み（要件 3）

/// `scope` の記憶ファイルを実 fs から直接読み、`key` の値を拾う（アクター不在の起動前用）。
/// 読めなければ無し（`load_scope` が不在・読取失敗を空へ縮退し、失敗は warn を残す）。
fn read_last(scope: PersistScope, roots: &ScopeRoots, key: PersistKey) -> Option<String> {
    load_scope(scope, roots, &FsPersistIo)
        .into_iter()
        .find_map(|(k, v)| (k == key).then_some(v))
}

/// 起動前の記憶の直読み（App スコープ `areka.last.ghost`・要件 4.2）。無ければ `None`。
pub(crate) fn read_last_ghost(app_profile_dir: &Path) -> Option<String> {
    read_last(
        PersistScope::App,
        &app_roots(app_profile_dir),
        PersistKey::LastGhost,
    )
}

/// 起動前の記憶の直読み（起動するゴーストの Ghost スコープ `areka.last.balloon`・要件 5.2・裁定 4）。
/// 根は boot が据える場所と同じ `profile_areka_root(<ゴースト>/ghost/master)`。無ければ `None`。
pub(crate) fn read_last_balloon(ghost_dir: &Path) -> Option<String> {
    let roots = ScopeRoots {
        ghost: Some(profile_areka_root(&ghost_dir.join("ghost").join("master"))),
        ..ScopeRoots::default()
    };
    read_last(PersistScope::Ghost, &roots, PersistKey::LastBalloon)
}

fn app_roots(app_profile_dir: &Path) -> ScopeRoots {
    ScopeRoots {
        app: Some(app_profile_dir.to_path_buf()),
        ..ScopeRoots::default()
    }
}

/// App スコープへ実 fs で直接書く。失敗は `save_scope` がログ済み・結末の `warn!` は呼び手が残す。
fn save_app(app_profile_dir: &Path, entries: Vec<(PersistKey, String)>) -> PersistOutcome {
    save_scope(
        PersistScope::App,
        &app_roots(app_profile_dir),
        &FsPersistIo,
        entries,
    )
}

// ---------------------------------------------------------------- 起動中の印（要件 12.1〜12.8）
//
// 印（App スコープ `areka.last.running`）は「今動いているゴーストの名前」。起こす前に書き、
// きれいな終わりでだけ消す（強制終了・クラッシュ・電源断では消えずに残る）。次の起動で残って
// いれば前回はきれいに終わらなかった＝最後のゴーストの記憶を読まずに解き、`OnBoot` の Ref7 に載せる。
//
// 下の書き手（`write_session_mark`・`write_switch_drop`・`clear_session_mark`）を UI スレッドから
// 呼ぶのは、ゴーストの実行系が 1 つも動いていない間だけ（初回の起動の前・切替の降ろした直後・
// 後始末で降ろした後）。記憶の保存は「読んで重ねて書く」ので、動いている実行系の記憶の書き手と
// 同時に同じファイルを書くと片方が消えうるため。

/// 起動中の印（App スコープ `areka.last.running`）。空文字・無しは `None`。読むだけで消さない。
pub(crate) fn read_session_mark(app_profile_dir: &Path) -> Option<String> {
    read_last(
        PersistScope::App,
        &app_roots(app_profile_dir),
        PersistKey::LastRunning,
    )
}

/// 印を書く（初回の起動の前・呼び手は argv で始まったプロセスでないときだけ呼ぶ）。
pub(crate) fn write_session_mark(app_profile_dir: &Path, running: &str) {
    match save_app(
        app_profile_dir,
        vec![(PersistKey::LastRunning, running.to_owned())],
    ) {
        PersistOutcome::Saved => tracing::info!(
            event = "session_mark_written",
            ghost = running,
            "[boot_resolve] 起動中の印を書きました"
        ),
        PersistOutcome::Degraded => tracing::warn!(
            event = "session_mark_write_degraded",
            ghost = running,
            dir = %app_profile_dir.display(),
            "[boot_resolve] 起動中の印を記憶へ書けませんでした（このゴーストが落ちても、次の起動の Ref6/7 にこのゴーストの名前は載りません）"
        ),
    }
}

/// 切替で前のゴーストを降ろした直後: 最後に使ったゴースト＝既定と（`mark` が在れば）印＝切替先を
/// 1 回の書き込みで書く。argv で始まったプロセスは `mark` に `None` を渡す（印に触れない・要件 12.5）。
// 本番の呼び手は切替の記憶の時点を移すタスク（`ghost_switch::switch_to`）で付く。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn write_switch_drop(app_profile_dir: &Path, mark: Option<&str>) {
    let mut entries = vec![(PersistKey::LastGhost, DEFAULT_GHOST_FOLDER.to_owned())];
    entries.extend(mark.map(|m| (PersistKey::LastRunning, m.to_owned())));
    match save_app(app_profile_dir, entries) {
        PersistOutcome::Saved => tracing::info!(
            event = "switch_drop_recorded",
            last_ghost = DEFAULT_GHOST_FOLDER,
            mark = mark.unwrap_or("-"),
            "[boot_resolve] 降ろした直後の記憶を書きました（最後のゴーストは既定・印は切替先・- は argv なので印に触れていない）"
        ),
        PersistOutcome::Degraded => tracing::warn!(
            event = "switch_drop_record_degraded",
            last_ghost = DEFAULT_GHOST_FOLDER,
            mark = mark.unwrap_or("-"),
            dir = %app_profile_dir.display(),
            "[boot_resolve] 降ろした直後の記憶を書けませんでした（切替先が落ちても、次の起動の印は前のゴーストの名前のまま・最後のゴーストは前のゴーストのままになります）"
        ),
    }
}

/// きれいな終わり: 印を消す（空文字を書く）。呼び手は後始末で降ろした後。
pub(crate) fn clear_session_mark(app_profile_dir: &Path) {
    match save_app(
        app_profile_dir,
        vec![(PersistKey::LastRunning, String::new())],
    ) {
        PersistOutcome::Saved => tracing::info!(
            event = "session_mark_cleared",
            "[boot_resolve] きれいに終わったので起動中の印を消しました"
        ),
        PersistOutcome::Degraded => tracing::warn!(
            event = "session_mark_clear_degraded",
            dir = %app_profile_dir.display(),
            "[boot_resolve] 起動中の印を消せませんでした（きれいに終わったのに、次の起動は前回落ちたとして既定のゴーストで Ref6/7 付きになります）"
        ),
    }
}

/// 印に書く名前: 目録の同じフォルダの descript の `name`、無ければフォルダ名（argv 以外の決定だけが来る）。
pub(crate) fn running_name(root: &BasewareRoot, ghost: &GhostDecision) -> String {
    let folder = ghost.folder.clone().unwrap_or_else(|| {
        // argv の決定は印を書かない（呼び手が旗で止める）。来たらフォルダの末尾で代える。
        ghost
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    areka_ghost::catalog::list_ghosts(root)
        .into_iter()
        .find(|e| e.identity.folder == folder)
        .and_then(|e| e.identity.name)
        .unwrap_or(folder)
}

/// 起動成功時に書く内容（要件 3.2〜3.5・裁定 5）。
pub(crate) struct LastUsed<'a> {
    pub ghost: &'a GhostDecision,
    pub balloon: &'a BalloonDecision,
    /// `mount().shell.dir` の末尾（`seriko.defaultsurfacedirectoryname` か `master`）
    pub shell_folder: &'a str,
}

impl LastUsed<'_> {
    /// App へ `LastGhost`（argv 以外のとき）、Ghost へ `LastBalloon`（argv 以外のとき）＋`LastShell`
    /// （常に）を投函する。argv で決まった側は書かず info を残す。投函だけで待たない
    /// （反映は `GhostRuntime::shutdown` の barrier に任せる＝design R1）。
    /// `areka.last.shell` は書くだけで起動の解決には使わない（要件 3.8）。
    pub(crate) fn record(&self, publisher: &SylphyaPublisher) {
        let ghost = remembered(
            self.ghost.route == GhostRoute::Argv,
            "ghost",
            &self.ghost.dir,
            &self.ghost.folder,
        );
        let balloon = remembered(
            self.balloon.route == BalloonRoute::Argv,
            "balloon",
            &self.balloon.dir,
            &self.balloon.folder,
        );
        if let Some(folder) = ghost {
            publisher.persist_put(
                PersistScope::App,
                vec![(PersistKey::LastGhost, folder.to_owned())],
            );
        }
        let mut entries: Vec<_> = balloon
            .map(|folder| (PersistKey::LastBalloon, folder.to_owned()))
            .into_iter()
            .collect();
        entries.push((PersistKey::LastShell, self.shell_folder.to_owned()));
        publisher.persist_put(PersistScope::Ghost, entries);
        tracing::info!(
            event = "last_used_recorded",
            ghost = ghost.unwrap_or("-"),
            balloon = balloon.unwrap_or("-"),
            shell = self.shell_folder,
            "[boot_resolve] 最後に使ったものを記憶へ書きました（- は argv なので書いていない）"
        );
    }
}

/// argv で決まった側は `None`（書かない旨を info に残す＝要件 3.5）。それ以外はフォルダ名。
fn remembered<'a>(
    argv: bool,
    side: &str,
    dir: &Path,
    folder: &'a Option<String>,
) -> Option<&'a str> {
    if argv {
        tracing::info!(
            event = "last_used_skipped_argv",
            side,
            dir = %dir.display(),
            "[boot_resolve] argv で決まったので記憶を書き換えません"
        );
        return None;
    }
    if folder.is_none() {
        // resolve_* は argv 以外で必ず folder を持つ。来たら型の約束が崩れている。
        tracing::warn!(
            event = "last_used_folder_missing",
            side,
            dir = %dir.display(),
            "[boot_resolve] argv でないのにフォルダ名が無いので記憶を書きません"
        );
    }
    folder.as_deref()
}

#[cfg(test)]
#[path = "boot_resolve_tests.rs"]
mod boot_resolve_tests;
