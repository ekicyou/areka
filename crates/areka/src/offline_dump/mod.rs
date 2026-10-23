//! オフラインの書き出し（`--offline-dump`）の走り（spec: areka-P0-offline-surface-dump）。
//!
//! ゴーストを起こさず、窓も出さずに、指定のシェルの面を PNG のファイルへ書き出し、その間の
//! 記録を CSV に残して終了コードを返す。使ってよい部品は design.md の許す依存の一覧だけで、
//! areka の記憶・置き場・MCP・窓の部品は呼ばない（要件 9 の構造の上の担保）。
//!
//! 順番（design「OfflineDumpRun」）: 始めの info → ゴーストの名前 → 写しの窓を開ける → スコープ →
//! `--dump-shell` → シェルの解決 → COM → シェルの読み込み → 面の段 → 写しの窓を閉じる →
//! 終了コード → CSV → 終わりの info。途中の失敗はどれも窓を閉じる段へ流れる。

use std::path::{Path, PathBuf};

use areka_emo_atlas::WicDecoderArm;
use areka_emo_compose::EmoWorld;
use areka_emo_present::{ShellTarget, load_shell_target};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::{BindGroupDefaults, read_ghost_names, resolve_with_shell};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

use crate::cli::OfflineDumpRequest;
use crate::emo2_boot::shell_balloon_resolve::{find_by_name_or_folder, shell_candidates};
use crate::log_history;

/// `--dump-bind` の読みと、スコープの着せ替えへの当て方。
mod bind_spec;
/// 面 1 つを時点 T まで進めて組み、シェル全体の下地に置いて PNG のファイルにする。
mod render;
/// 写した記録から CSV の中身・終了コード・CSV のファイルを作る。
mod report;
/// `--dump-surface-list` の読みと、シェルの面の番号への展開。
mod surface_list;

/// 面の段の結果（終わりの info に使う）。
#[derive(Debug, Default, Clone, Copy)]
struct DumpTally {
    written: usize,
    failed: usize,
}

/// 読み込んだシェル（面の段へ 1 度だけ渡す）。
#[allow(dead_code)] // 読み手は面の段（タスク 7.8）
struct LoadedShell {
    /// 選んだシェルの descript.txt の着せ替えの読み（既定の組と名前の表の元）。
    bindgroups: BindGroupDefaults,
    target: ShellTarget,
    /// 読み込みで組んだ面の表（`take_world` で 1 度だけ受け取る）。
    world: EmoWorld,
}

/// オフラインの書き出しを 1 回走らせ、終了コード（0・1・2）を返す。
#[allow(dead_code)] // 呼び手は main の入口（タスク 8）
pub(crate) fn run(req: &OfflineDumpRequest) -> i32 {
    tracing::info!(
        event = "offline_dump_started",
        ghost = ?req.ghost_dir,
        shell = ?req.shell,
        scope = ?req.scope,
        surfaces = ?req.surface_list,
        output_dir = ?req.output_dir,
        error_log = ?req.error_log,
        "[offline_dump] 書き出しを始めます"
    );
    // ゴーストの読み込みの失敗は窓を開けずに 0（CSV も作らない＝2.6・8.4）。
    let (code, tally, worst) = match read_ghost(req.ghost_dir.as_deref()) {
        Some((root, ghost_name)) => dump(req, &root, &ghost_name),
        None => (0, DumpTally::default(), None),
    };
    tracing::info!(
        event = "offline_dump_finished",
        written = tally.written,
        failed = tally.failed,
        worst = worst.unwrap_or("none"),
        exit_code = code,
        "[offline_dump] 書き出しを終えました"
    );
    code
}

/// 写しの窓を開けてからの段（窓は必ずここで閉じる）。終了コード・面の数・最も重いレベルを返す。
fn dump(
    req: &OfflineDumpRequest,
    root: &Path,
    ghost_name: &str,
) -> (i32, DumpTally, Option<&'static str>) {
    log_history::capture_begin();
    let scope = read_scope(req.scope.as_deref());
    let tally = match load_shell(root, req.shell.as_deref()) {
        Some(shell) => surface_stage(req, scope, shell),
        None => DumpTally::default(),
    };
    let records = log_history::capture_finish();
    let code = report::exit_code(&records);
    if let Some(raw) = req.error_log.as_deref() {
        match std::path::absolute(raw) {
            // 書けなければ write_csv が error! を出す（終了コードは変えない＝7.7）。
            Ok(path) => {
                let _ = report::write_csv(&path, ghost_name, &records);
            }
            Err(e) => tracing::error!(
                event = "offline_dump_error_log_write_failed",
                path = raw,
                error = %e,
                "[offline_dump] --dump-error-log のパスを読めませんでした"
            ),
        }
    }
    (code, tally, report::worst(&records))
}

/// ゴーストのフォルダを今のフォルダから絶対にし、descript.txt の `name` を読む（無ければ空）。
/// 値なし・読めないときは error! を出して None（2.1・2.2・2.6・1.8）。
fn read_ghost(raw: Option<&str>) -> Option<(PathBuf, String)> {
    let Some(raw) = raw else {
        tracing::error!(
            event = "offline_dump_ghost_missing_value",
            "[offline_dump] --offline-dump にゴーストのフォルダが渡されていません"
        );
        return None;
    };
    let fail = |error: &dyn std::fmt::Debug| {
        tracing::error!(
            event = "offline_dump_ghost_unreadable",
            ghost = raw,
            error = ?error,
            "[offline_dump] ゴーストを読めませんでした"
        );
    };
    let root = std::path::absolute(raw).inspect_err(|e| fail(e)).ok()?;
    let names = read_ghost_names(&root, DefaultEncoding::Ansi)
        .inspect_err(|e| fail(e))
        .ok()?;
    Some((root, names.name.unwrap_or_default()))
}

/// `--dump-scope` の値。無ければ 0、0 以上の整数として読めなければ 0 と warn!（3.8・3.10）。
fn read_scope(raw: Option<&str>) -> u32 {
    let Some(raw) = raw else { return 0 };
    raw.parse().unwrap_or_else(|_| {
        tracing::warn!(
            event = "offline_dump_scope_unreadable",
            value = raw,
            "[offline_dump] --dump-scope を 0 以上の整数として読めないのでスコープ 0 で書き出します"
        );
        0
    })
}

/// `--dump-shell` を探してシェルを解決し、COM（MTA）を初期化して読み込む（2.3〜2.5・2.7）。
/// 見つからない・フォルダが無い・COM の失敗は error! を出して None、読み込みの失敗は下の層の
/// error! だけで None（1 つの誤りは 1 行）。
fn load_shell(root: &Path, want: Option<&str>) -> Option<LoadedShell> {
    // 値なしは既定のシェル（利用者の記憶は読まない＝2.3）。random・lastinstalled はただの名前（2.4）。
    let folder = match want {
        None => None,
        Some(want) => {
            let candidates = shell_candidates(root);
            let Some(hit) = find_by_name_or_folder(&candidates, want) else {
                tracing::error!(
                    event = "offline_dump_shell_missing",
                    shell = want,
                    "[offline_dump] --dump-shell のシェルが見つからないので面を書き出しません"
                );
                return None;
            };
            Some(hit.folder.clone())
        }
    };
    let model = resolve_with_shell(root, DefaultEncoding::Ansi, folder.as_deref())
        .inspect_err(|e| {
            tracing::error!(
                event = "offline_dump_shell_unreadable",
                error = ?e,
                "[offline_dump] シェルを読めませんでした"
            );
        })
        .ok()?;
    // SAFETY: このスレッドの COM 初期化（MTA・WIC のデコーダの前提）。既に MTA なら S_FALSE で続ける。
    // プロセスはすぐ終わるので対の解除は呼ばない（design「OfflineDumpRun」）。
    let decoder = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .and_then(|()| WicDecoderArm::new())
        .inspect_err(|e| {
            tracing::error!(
                event = "offline_dump_com_failed",
                error = %e,
                "[offline_dump] COM（MTA）と画像のデコーダを用意できませんでした"
            );
        })
        .ok()?;
    // 失敗は load_shell_target 自身が error! を出す。
    let mut target = load_shell_target(&model.shell.dir, &decoder).ok()?;
    let world = target.take_world();
    tracing::info!(
        event = "offline_dump_shell_loaded",
        shell_dir = %model.shell.dir.display(),
        surfaces = world.surface_ids().count(),
        "[offline_dump] シェルを読みました"
    );
    Some(LoadedShell {
        bindgroups: model.bindgroups,
        target,
        world,
    })
}

/// 面の段。シェルを読めた走りで、写しの窓を閉じる前に 1 度だけ呼ぶ。スコープの着せ替え・
/// `--dump-bind`・面を描いて書く段はタスク 7.8 がこの関数の中に置く。
fn surface_stage(_req: &OfflineDumpRequest, _scope: u32, _shell: LoadedShell) -> DumpTally {
    DumpTally::default()
}

/// 検体のゴーストを `target\` の下に組む部品（試験専用）。
#[cfg(test)]
mod offline_dump_test_support;

#[cfg(test)]
#[path = "offline_dump_tests.rs"]
mod offline_dump_tests;
