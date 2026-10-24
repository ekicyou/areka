//! 起動解決の純粋な判断（areka-P0-baseware-root-layout 要件 4・5）。
//!
//! 起動するゴースト（6 分岐）とバルーン（8 分岐＝argv・記憶・descript・同梱・唯一・既定・
//! 無作為・0）を「argv・記憶の値・ゴーストの descript の 2 鍵・同梱の名・列挙の素性」だけから
//! 決める。判断は I/O・時計・乱数源を持たず、無作為は「候補数 → 添字」の関数を注入する
//! （本番は [`pick_index`]）。列挙の並びは判断に使わない（裁定 3）。ただ 1 点、descript の
//! `balloon` に `name` が一致するバルーンが複数在るときだけ、列挙の並び（ゴーストの中が先、それぞれフォルダ名の昇順）で
//! 最初のものにする（areka-P0-ghost-standard-balloon 要件 2.6）。
//!
//! 既定の定数 [`DEFAULT_GHOST_FOLDER`]／[`DEFAULT_BALLOON_FOLDER`] はこのファイルだけが持ち、
//! 解決の判断での参照はそれぞれゴーストの段 4・バルーンの段 6 の 1 か所だけ（要件 4.11・5.9）。
//! 判断の外では、切替の降ろした直後の記憶 [`write_switch_drop`] が既定ゴーストを最後のゴーストの
//! 書き先に使う（要件 12.6）。

use std::hash::{BuildHasher, RandomState};
use std::path::{Path, PathBuf};

use crate::app_exit::ExitOrigin;
use areka_ghost::catalog::{BalloonEntry, GhostEntry};
use areka_ghost::same_folder_name;
use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{
    PersistFailure, PersistKey, PersistOutcome, PersistScope, ScopeRoots, SylphyaPublisher,
    load_scope, save_scope,
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
    /// `--ghost` の指し先で決まった（記憶・印の書き込みは他の経路と同じ・areka-P0-startup-options-core 要件 3.9）。
    Pointed,
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
    /// ゴーストの descript.txt の `default.balloon.path`・`balloon` で決まった。
    Descript,
    Companion,
    Only,
    Default,
    Random,
}

/// 決まったゴースト（argv なら渡されたパスそのもの・それ以外は当たった和の項目の場所）。
/// argv の経路だけ `folder` が `None`（根の外かもしれないので名前を持たない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GhostDecision {
    pub route: GhostRoute,
    pub dir: PathBuf,
    pub folder: Option<String>,
}

/// 決まったバルーン（argv なら渡されたパスそのもの・それ以外は当たった和の項目の場所）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BalloonDecision {
    pub route: BalloonRoute,
    pub dir: PathBuf,
    pub folder: Option<String>,
}

/// 0 体（要件 4.7・9.6）。告知の文面に置くべき場所を載せるため各根の格納フォルダ（上から）を持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoGhost {
    pub ghost_stores: Vec<PathBuf>,
}

/// 0（要件 5.8・9.7）。告知の文面に置くべき場所を載せるため各根の格納フォルダ（上から）を持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoBalloon {
    pub balloon_stores: Vec<PathBuf>,
}

/// ゴーストの判断の入力（要件 4.9「argv・記憶の値・列挙の結果だけ」）。
/// `argv` があるとき `memory`／`listed`／`stores` は読まない（空でよい）。
pub(crate) struct GhostInputs<'a> {
    /// argv[1]
    pub argv: Option<&'a Path>,
    /// `--ghost` の指し先（場所・フォルダ名）。在れば記憶・列挙は読まない。
    pub pointed: Option<(&'a Path, &'a str)>,
    /// `--random`（記憶・唯一・既定の段を飛ばして列挙から無作為に選ぶ）。
    pub random: bool,
    /// App スコープ `areka.last.ghost`
    pub memory: Option<&'a str>,
    /// 和の `list_ghosts`（フォルダ名の昇順）
    pub listed: &'a [GhostEntry],
    /// 見つかる根の `ghost\`（上から・記録と 0 体の失敗に載せる）
    pub stores: &'a [PathBuf],
}

/// バルーンの判断の入力。`argv` があるとき他は読まない（空でよい）。
pub(crate) struct BalloonInputs<'a> {
    /// argv[2]
    pub argv: Option<&'a Path>,
    /// 起動するゴーストの Ghost スコープ `areka.last.balloon`
    pub memory: Option<&'a str>,
    /// ゴーストの descript.txt の `default.balloon.path`
    pub default_balloon_path: Option<&'a str>,
    /// ゴーストの descript.txt の `balloon`
    pub balloon_name: Option<&'a str>,
    /// `<ゴースト>/install.txt` の同梱の最初の 1 個（無印 → `balloon0`）の `*.directory`
    pub companion: Option<&'a str>,
    /// `balloons_for` の一覧（ゴーストの中が先、それぞれフォルダ名の昇順）
    pub listed: &'a [BalloonEntry],
    /// 見つかる根の `balloon\`（上から・記録と 0 の失敗に載せる）
    pub stores: &'a [PathBuf],
}

/// 格納フォルダの列を記録の値へ（`; ` でつなぐ・根 1 つなら今と同じ綴り）。
fn joined(stores: &[PathBuf]) -> String {
    stores
        .iter()
        .map(|s| s.display().to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// フォルダ名が `name` と同じ（要件 2.8 の 1 つの規則）和の項目（在る・無いの判定だけ。並びは見ない）。
fn find<'a>(listed: &'a [GhostEntry], name: &str) -> Option<&'a GhostEntry> {
    listed
        .iter()
        .find(|e| same_folder_name(&e.identity.folder, name))
}

/// [`find`] のバルーンの一覧版（フォルダ名で突き合わせる）。
fn find_balloon<'a>(listed: &'a [BalloonEntry], name: &str) -> Option<&'a BalloonEntry> {
    listed
        .iter()
        .find(|e| same_folder_name(&e.identity.folder, name))
}

/// descript の `balloon` の突き合わせ: `name` が一致する最初（列挙の並び＝ゴーストの中が先、それぞれフォルダ名の昇順）、
/// 1 つも無いときに限りフォルダ名（実行中の `\![change,balloon,名前]` のふつうの名前と同じ・要件 2.6）。
fn find_balloon_by_name<'a>(listed: &'a [BalloonEntry], name: &str) -> Option<&'a BalloonEntry> {
    listed
        .iter()
        .find(|e| e.identity.name.as_deref() == Some(name))
        .or_else(|| find_balloon(listed, name))
}

/// descript の段で鍵ごとに使う突き合わせ（[`find_balloon`] か [`find_balloon_by_name`]）。
type FindBalloon = for<'l> fn(&'l [BalloonEntry], &str) -> Option<&'l BalloonEntry>;

/// 段 1〜5＋0 体（要件 4.1〜4.7）に、areka-P0-startup-options-core の段 1b（指し先）と
/// `--random` の段を足した並び。`pick` は「候補数 n（≥ 2）→ 0..n の添字」。純粋。
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
    // 段 1b: `--ghost` の指し先（記憶・印・列挙に関わらず・要件 3.1・4.3）。
    if let Some((dir, folder)) = inputs.pointed {
        return Ok(GhostDecision {
            route: GhostRoute::Pointed,
            dir: dir.to_path_buf(),
            folder: Some(folder.to_owned()),
        });
    }
    let listed = inputs.listed;
    // 当たった和の項目の場所とフォルダ名をそのまま使う（根から組み直さない）。
    let at = |route, entry: &GhostEntry| GhostDecision {
        route,
        dir: entry.dir.clone(),
        folder: Some(entry.identity.folder.clone()),
    };
    // `--random`: 記憶・唯一・既定を飛ばして列挙から選ぶ（要件 5.1・5.2・5.4）。
    if inputs.random {
        let entry = match listed {
            [] => {
                return Err(NoGhost {
                    ghost_stores: inputs.stores.to_vec(),
                });
            }
            [only] => only,
            _ => &listed[pick(listed.len())],
        };
        tracing::info!(
            event = "ghost_random_requested",
            folder = entry.identity.folder.as_str(),
            candidates = listed.len(),
            "[boot_resolve] --random なので置き場のゴーストから無作為に選びました"
        );
        return Ok(at(GhostRoute::Random, entry));
    }
    // 段 2: 記憶（列挙に在れば）。在らねば黙って読み替えず warn して次へ（要件 4.6）。
    if let Some(memory) = inputs.memory {
        match find(listed, memory) {
            Some(entry) => return Ok(at(GhostRoute::Memory, entry)),
            None => tracing::warn!(
                event = "last_ghost_not_found",
                memory,
                ghost_store = %joined(inputs.stores),
                "[boot_resolve] 前回のゴーストが根に見つからないので次の候補へ進みます"
            ),
        }
    }
    match listed {
        // 0 体（要件 4.7）。
        [] => Err(NoGhost {
            ghost_stores: inputs.stores.to_vec(),
        }),
        // 段 3: 唯一。
        [only] => Ok(at(GhostRoute::Only, only)),
        _ => {
            // 段 4: 既定（DEFAULT_GHOST_FOLDER を参照するのはこの段だけ）。
            if let Some(entry) = find(listed, DEFAULT_GHOST_FOLDER) {
                return Ok(at(GhostRoute::Default, entry));
            }
            // 段 5: 無作為。
            let entry = &listed[pick(listed.len())];
            tracing::info!(
                event = "ghost_picked_randomly",
                folder = entry.identity.folder.as_str(),
                candidates = listed.len(),
                "[boot_resolve] 既定のゴーストが無いので無作為に選びました"
            );
            Ok(at(GhostRoute::Random, entry))
        }
    }
}

/// 段 1〜7＋0（完了 spec の要件 5.1〜5.8 に areka-P0-ghost-standard-balloon の要件 3.1 の descript の段を
/// 足した並び）。`pick` は「候補数 n（≥ 2）→ 0..n の添字」。純粋。
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
    // 当たった和の項目の場所とフォルダ名をそのまま使う（根から組み直さない）。
    let at = |route, entry: &BalloonEntry| BalloonDecision {
        route,
        dir: entry.dir.clone(),
        folder: Some(entry.identity.folder.clone()),
    };
    let store = joined(inputs.stores);
    // 段 2: ゴーストごとの記憶（同梱より先＝裁定 4）。
    if let Some(memory) = inputs.memory {
        match find_balloon(listed, memory) {
            Some(entry) => return Ok(at(BalloonRoute::Memory, entry)),
            None => tracing::warn!(
                event = "last_balloon_not_found",
                memory,
                balloon_store = %store,
                "[boot_resolve] 前回のバルーンが根にもゴーストの中にも見つからないので次の候補へ進みます"
            ),
        }
    }
    // 段 3: ゴーストの descript.txt（`default.balloon.path` → `balloon`・当たった鍵より後ろは見ない）。
    // 値は読み替えない（`..`・絶対パス・区切りはフォルダ名に完全一致しないので当たらない）。
    // `random`・`lastinstalled` も他の値と同じく突き合わせるだけ（要件 2.8）。
    let descript: [(&str, Option<&str>, FindBalloon); 2] = [
        (
            "default.balloon.path",
            inputs.default_balloon_path,
            find_balloon,
        ),
        ("balloon", inputs.balloon_name, find_balloon_by_name),
    ];
    for (key, value, find) in descript {
        let Some(value) = value else { continue };
        match find(listed, value) {
            Some(entry) => return Ok(at(BalloonRoute::Descript, entry)),
            None => tracing::warn!(
                event = "descript_balloon_not_found",
                key,
                value,
                balloon_store = %store,
                "[boot_resolve] ゴーストの descript.txt が指すバルーンが根にもゴーストの中にも見つからないので次の候補へ進みます"
            ),
        }
    }
    // 段 4: ゴーストの同梱（install.txt の同梱の最初の 1 個＝無印 → `balloon0`）。
    if let Some(companion) = inputs.companion {
        match find_balloon(listed, companion) {
            Some(entry) => return Ok(at(BalloonRoute::Companion, entry)),
            None => tracing::warn!(
                event = "companion_balloon_not_found",
                companion,
                balloon_store = %store,
                "[boot_resolve] ゴーストの同梱バルーンが根にもゴーストの中にも見つからないので次の候補へ進みます"
            ),
        }
    }
    match listed {
        // 0（要件 5.8）。
        [] => Err(NoBalloon {
            balloon_stores: inputs.stores.to_vec(),
        }),
        // 段 5: 唯一。
        [only] => Ok(at(BalloonRoute::Only, only)),
        _ => {
            // 段 6: 既定（DEFAULT_BALLOON_FOLDER を参照するのはこの段だけ）。
            if let Some(entry) = find_balloon(listed, DEFAULT_BALLOON_FOLDER) {
                return Ok(at(BalloonRoute::Default, entry));
            }
            // 段 7: 無作為。
            let entry = &listed[pick(listed.len())];
            tracing::info!(
                event = "balloon_picked_randomly",
                folder = entry.identity.folder.as_str(),
                candidates = listed.len(),
                "[boot_resolve] 既定のバルーンが無いので無作為に選びました"
            );
            Ok(at(BalloonRoute::Random, entry))
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
    read_last(
        PersistScope::Ghost,
        &ghost_roots(ghost_dir),
        PersistKey::LastBalloon,
    )
}

/// 起動前の記憶の直読み（起動するゴーストの Ghost スコープ `areka.last.shell`・要件 6.2）。
/// 置き場は [`read_last_balloon`] と同じ。無ければ `None`。
pub(crate) fn read_last_shell(ghost_dir: &Path) -> Option<String> {
    read_last(
        PersistScope::Ghost,
        &ghost_roots(ghost_dir),
        PersistKey::LastShell,
    )
}

/// 起動するゴーストの Ghost スコープの根（boot が据える `profile_areka_root(<ゴースト>/ghost/master)`）。
fn ghost_roots(ghost_dir: &Path) -> ScopeRoots {
    ScopeRoots {
        ghost: Some(profile_areka_root(&ghost_dir.join("ghost").join("master"))),
        ..ScopeRoots::default()
    }
}

/// 起動のシェル（要件 6.2・6.3）。記憶の名の `shell/<名>/descript.txt` が在ればその名
/// （`menu,hidden` でも可）・記憶が無ければ `None`（既定のシェル）。記憶の先が無いか、名前が
/// `shell/` の下の 1 段のフォルダ名でない（`..`・区切り・絶対パス＝壊れた記憶）ときは
/// `warn!` 1 件で `None`。記憶は起動の成功（`LastUsed::record`）が既定のシェルへ書き直す。
/// シェル名つきの解決は名前を検査しないので、ここが `shell/` の外を指す名前を止める。
pub(crate) fn decide_boot_shell(ghost_dir: &Path) -> Option<String> {
    let name = read_last_shell(ghost_dir)?;
    let mut parts = Path::new(&name).components();
    let single = matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(_)), None)
    );
    let descript = ghost_dir.join("shell").join(&name).join("descript.txt");
    if single && descript.is_file() {
        return Some(name);
    }
    tracing::warn!(
        event = "boot_shell_missing",
        name = %name,
        expected = %descript.display(),
        "[boot_resolve] 前回のシェルが見つからないので既定のシェルで起動します（起動に成功すれば記憶は既定のシェルへ書き直ります）"
    );
    None
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
/// 書けなければ失敗の中身を返す（呼び手が起動を止める・要件 3.1）。
pub(crate) fn write_session_mark(
    app_profile_dir: &Path,
    running: &str,
) -> Result<(), PersistFailure> {
    match save_app(
        app_profile_dir,
        vec![(PersistKey::LastRunning, running.to_owned())],
    ) {
        PersistOutcome::Saved => {
            tracing::info!(
                event = "session_mark_written",
                ghost = running,
                "[boot_resolve] 起動中の印を書きました"
            );
            Ok(())
        }
        PersistOutcome::Withheld => {
            tracing::info!(
                event = "session_mark_withheld",
                ghost = running,
                "[boot_resolve] 読み取りだけなので、起動中の印を書きませんでした"
            );
            Ok(())
        }
        PersistOutcome::Degraded(failure) => {
            tracing::warn!(
                event = "session_mark_write_degraded",
                ghost = running,
                dir = %app_profile_dir.display(),
                reason = ?failure.cause,
                "[boot_resolve] 起動中の印を記憶へ書けませんでした（起動を止めます）"
            );
            Err(failure)
        }
    }
}

/// 切替で前のゴーストを降ろした直後: 最後に使ったゴースト＝既定と（`mark` が在れば）印＝切替先を
/// 1 回の書き込みで書く。argv で始まったプロセスは `mark` に `None` を渡す（印に触れない・要件 12.5）。
/// 呼び手は `ghost_switch::switch_to`。書けなければ失敗の中身を返す（呼び手が終える・要件 3.3）。
pub(crate) fn write_switch_drop(
    app_profile_dir: &Path,
    mark: Option<&str>,
) -> Result<(), PersistFailure> {
    let mut entries = vec![(PersistKey::LastGhost, DEFAULT_GHOST_FOLDER.to_owned())];
    entries.extend(mark.map(|m| (PersistKey::LastRunning, m.to_owned())));
    match save_app(app_profile_dir, entries) {
        PersistOutcome::Saved => {
            tracing::info!(
                event = "switch_drop_recorded",
                last_ghost = DEFAULT_GHOST_FOLDER,
                mark = mark.unwrap_or("-"),
                "[boot_resolve] 降ろした直後の記憶を書きました（最後のゴーストは既定・印は切替先・- は argv なので印に触れていない）"
            );
            Ok(())
        }
        PersistOutcome::Withheld => {
            tracing::info!(
                event = "switch_drop_withheld",
                last_ghost = DEFAULT_GHOST_FOLDER,
                mark = mark.unwrap_or("-"),
                "[boot_resolve] 読み取りだけなので、降ろした直後の記憶を書きませんでした"
            );
            Ok(())
        }
        PersistOutcome::Degraded(failure) => {
            tracing::warn!(
                event = "switch_drop_record_degraded",
                last_ghost = DEFAULT_GHOST_FOLDER,
                mark = mark.unwrap_or("-"),
                dir = %app_profile_dir.display(),
                reason = ?failure.cause,
                "[boot_resolve] 降ろした直後の記憶を書けませんでした（終了します）"
            );
            Err(failure)
        }
    }
}

/// きれいな終わり: 印を消す（空文字を書く）。呼び手は後始末で降ろした後。
/// 消せなければ記憶の印を読み直し、残る印が在るときだけ次の起動を予告する（要件 4.2）。
pub(crate) fn clear_session_mark(app_profile_dir: &Path) -> Result<(), PersistFailure> {
    match save_app(
        app_profile_dir,
        vec![(PersistKey::LastRunning, String::new())],
    ) {
        PersistOutcome::Saved => {
            tracing::info!(
                event = "session_mark_cleared",
                "[boot_resolve] きれいに終わったので起動中の印を消しました"
            );
            Ok(())
        }
        PersistOutcome::Withheld => {
            tracing::info!(
                event = "session_mark_clear_withheld",
                "[boot_resolve] 読み取りだけなので、起動中の印に触れていません"
            );
            Ok(())
        }
        PersistOutcome::Degraded(failure) => {
            match read_session_mark(app_profile_dir) {
                Some(remaining) => tracing::warn!(
                    event = "session_mark_clear_degraded",
                    dir = %app_profile_dir.display(),
                    remaining = remaining.as_str(),
                    "[boot_resolve] 起動中の印を消せませんでした（きれいに終わったのに、次の起動は前回落ちたとして既定のゴーストで起き、Ref7 は{remaining}になります）"
                ),
                None => tracing::warn!(
                    event = "session_mark_clear_degraded",
                    dir = %app_profile_dir.display(),
                    remaining = "-",
                    "[boot_resolve] 起動中の印を消せませんでした（記憶に起動中の印は残っていません）"
                ),
            }
            Err(failure)
        }
    }
}

/// きれいに終わらなかったので印を残すときの記録（`session_mark_kept`）。記憶の印を読み直し、
/// 印が在るときだけ次の起動の Ref6/7 を予告する（要件 4.3）。呼び手は `main::settle_session_mark`。
/// 置き場が控える範囲の中なら印を読み直さない（控えの印で次の起動を予告しない・readonly-mode 要件 6.3）。
pub(crate) fn keep_session_mark(
    app_profile_dir: &Path,
    reason: &'static str,
    first: Option<&ExitOrigin>,
) {
    if areka_sylphya::persist::withhold::withholds(app_profile_dir) {
        tracing::info!(
            event = "session_mark_kept",
            readonly = true,
            reason,
            first = ?first,
            "[boot_resolve] 読み取りだけなので起動中の印に触れていません（次の起動は、この走りの前の記憶のとおりに起きます）"
        );
        return;
    }
    match read_session_mark(app_profile_dir) {
        Some(remaining) => tracing::info!(
            event = "session_mark_kept",
            reason,
            first = ?first,
            remaining = remaining.as_str(),
            "[boot_resolve] きれいに終わらなかったので起動中の印を残します（次の起動は既定のゴーストで Ref6/7 付き（Ref7＝{remaining}））"
        ),
        None => tracing::info!(
            event = "session_mark_kept",
            reason,
            first = ?first,
            remaining = "-",
            "[boot_resolve] きれいに終わらなかったので起動中の印を残します（記憶に起動中の印が無いので、次の起動は前回落ちたとは扱われません）"
        ),
    }
}

/// 印に書く名前: 決まったゴーストの場所の descript の `name`、無ければフォルダ名（argv 以外の決定だけが来る）。
/// 和は引かない（同じフォルダ名が別の根から見えても、動いているゴーストの名前を書く）。
pub(crate) fn running_name(ghost: &GhostDecision) -> String {
    let folder = ghost.folder.clone().unwrap_or_else(|| {
        // argv の決定は印を書かない（呼び手が旗で止める）。来たらフォルダの末尾で代える。
        ghost
            .dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    areka_ghost::catalog::descript_name(&ghost.dir.join("ghost").join("master")).unwrap_or(folder)
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
    /// `areka.last.shell` は次の起動のシェルの決定（[`decide_boot_shell`]・要件 6.2）が読む。
    /// 記憶の先が無かった起動では、ここで今マウントした既定のシェルへ書き直る（要件 6.3）。
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
            "[boot_resolve] 最後に使ったものを記憶へ書くように頼みました（- は argv なので書いていない）"
        );
    }
}

/// シェルの差し替えの完了: Ghost スコープへ `LastShell` だけを投函する（要件 6.1）。
/// `LastBalloon`・App スコープには触れない。投函だけで待たない（[`LastUsed::record`] と同じ）。
pub(crate) fn record_last_shell(publisher: &SylphyaPublisher, folder: &str) {
    publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::LastShell, folder.to_owned())],
    );
    tracing::info!(
        event = "last_shell_recorded",
        shell = folder,
        "[boot_resolve] 最後のシェルだけを記憶へ書くように頼みました"
    );
}

/// バルーンの差し替えの完了: Ghost スコープへ `LastBalloon` だけを投函する（要件 6.1・6.7）。
/// 3 つを書く [`LastUsed::record`] を通さないのは、入れた直後の `remember_balloon` の記憶を
/// 上書きしないため。`LastShell`・App スコープには触れない。
pub(crate) fn record_last_balloon(publisher: &SylphyaPublisher, folder: &str) {
    publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::LastBalloon, folder.to_owned())],
    );
    tracing::info!(
        event = "last_balloon_recorded",
        balloon = folder,
        "[boot_resolve] 最後のバルーンだけを記憶へ書くように頼みました"
    );
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

#[cfg(test)]
#[path = "boot_resolve_option_tests.rs"]
mod boot_resolve_option_tests;

#[cfg(test)]
#[path = "boot_resolve_balloon_tests.rs"]
mod boot_resolve_balloon_tests;

#[cfg(test)]
#[path = "boot_resolve_mark_text_tests.rs"]
mod boot_resolve_mark_text_tests;

#[cfg(test)]
#[path = "boot_resolve_readonly_tests.rs"]
mod boot_resolve_readonly_tests;
