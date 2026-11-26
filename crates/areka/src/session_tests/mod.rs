//! 集めた層のテストのうち、本体の部品も使うものの置き場。
//!
//! 集めた層は本体を使えないので、本体の根の関数・`cli`・menu・install・update・mcp の本物を
//! 見るテストはここに置く。各ファイルは元の親のモジュールの `use` を自分で持つ。本番のコードは置かない。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_session::alert::AlertScene;
use areka_session::boot_config::{BootContext, BootResolved, resolve_boot_with};
use areka_session::boot_resolve::{BalloonRoute, GhostRoute};
use areka_session::ghost_session::GhostSlot;
use areka_session::roots::{ResolvedRoots, RootError};
pub(crate) use areka_session::session_end::test_support::{put_memory, reservation};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, SylphyaPublisher, load_scope};
use bevy_ecs::prelude::World;

/// テストの口: 引数の列を読み方（`cli::parse`）に通し、起動の求めで [`resolve_boot_with`] を呼ぶ。
pub(crate) fn resolve_boot_from(
    resolved: Result<ResolvedRoots, RootError>,
    args: &[String],
    app_profile_dir: &std::path::Path,
    pick: fn(usize) -> usize,
    prepare_ghost: fn(&areka_ghost::BasewareRoots, &std::path::Path) -> std::path::PathBuf,
) -> Result<BootResolved, AlertScene> {
    resolve_boot_with(
        resolved,
        &crate::cli::parse::parse(args).options.boot,
        None,
        app_profile_dir,
        pick,
        prepare_ghost,
    )
}

/// 無作為の段へ届いてはならない起動の解決へ渡す選び手。
pub(crate) fn no_pick(n: usize) -> usize {
    panic!("無作為の段へ届いてはならない（候補 {n}）")
}

/// `dir` に descript だけのゴーストを置く。
pub(crate) fn put_ghost(dir: &Path) {
    let master = dir.join("ghost").join("master");
    std::fs::create_dir_all(&master).expect("フォルダを組む");
    std::fs::write(master.join("descript.txt"), "charset,UTF-8\n").expect("descript");
}

/// アプリの記憶に最後のゴーストを書く。
pub(crate) fn save_scope(app_dir: &Path, ghost_folder: &str) {
    assert_eq!(
        areka_sylphya::save_scope(
            PersistScope::App,
            &ScopeRoots {
                app: Some(app_dir.to_path_buf()),
                ..ScopeRoots::default()
            },
            &FsPersistIo,
            vec![(PersistKey::LastGhost, ghost_folder.to_owned())],
        ),
        areka_sylphya::PersistOutcome::Saved,
        "記憶を書けない: {}",
        app_dir.display()
    );
}

/// 根の下の木（相対パス → ファイルの中身。フォルダは `None`）。
pub(crate) fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).expect("木を歩く") {
            let path = entry.expect("ディレクトリ項目").path();
            let rel = path.strip_prefix(root).expect("根の下").to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(root, &path, out);
            } else {
                out.insert(rel, Some(std::fs::read(&path).expect("ファイルを読む")));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    assert!(!out.is_empty(), "走査が空振りした: {}", root.display());
    out
}

/// 保存の口から 1 つの key を読み返す。
pub(crate) fn read_back(
    app: &Path,
    ghost: &Path,
    scope: PersistScope,
    key: PersistKey,
) -> Option<String> {
    let roots = ScopeRoots {
        app: Some(app.to_path_buf()),
        ghost: Some(ghost.to_path_buf()),
        ..ScopeRoots::default()
    };
    load_scope(scope, &roots, &FsPersistIo)
        .into_iter()
        .find_map(|(k, v)| (k == key).then_some(v))
}

/// 動いているゴーストの記憶の書き手。
pub(crate) fn publisher_of(world: &World) -> SylphyaPublisher {
    world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.runtime())
        .expect("ゴーストの実行系が起きている")
        .sylphya_publisher()
        .clone()
}

/// 置き場を普通のファイルに替える（書けない）。
pub(crate) fn unwritable(dir: &Path) {
    if dir.is_dir() {
        std::fs::remove_dir_all(dir).expect("置き場のフォルダを外す");
    }
    std::fs::write(dir, "x").expect("置き場の位置に普通のファイルを置く");
}

/// 置き場をフォルダへ戻す（書ける）。
pub(crate) fn writable(dir: &Path) {
    if dir.is_file() {
        std::fs::remove_file(dir).expect("置き場の位置のファイルを外す");
    }
    std::fs::create_dir_all(dir).expect("置き場のフォルダを作る");
}

/// `event` の記録の水準（届いた順）。
pub(crate) fn levels_of(
    events: &[log_capture_kit::CapturedEvent],
    event: &str,
) -> Vec<tracing::Level> {
    events_named(events, event)
        .into_iter()
        .map(|e| e.level)
        .collect()
}

/// `warn!` の記録の `event` 欄（届いた順）。
pub(crate) fn warns(events: &[log_capture_kit::CapturedEvent]) -> Vec<Option<&str>> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .map(|e| e.field_str("event"))
        .collect()
}

/// `event` の記録が最初に現れた位置。
pub(crate) fn position_of(events: &[log_capture_kit::CapturedEvent], event: &str) -> Option<usize> {
    events
        .iter()
        .position(|e| e.field_str("event") == Some(event))
}

/// 台本の文字列を本番と同じ解析と組み立てで cue にし、受け口へ最後まで配る。
pub(crate) fn play_into(script: &str, sink: impl dola::cue::CueSink + 'static) {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(
        &instructions,
        &areka_sakura::sysvar::SystemVarSnapshot::default(),
    );
    let sheet = compiled
        .sheet
        .with_absolute_start_time(dola::TimePoint::ZERO);
    let mut player = dola::cue::CuePlayer::from_sheet(&sheet).expect("原点は刻印済み");
    player.register_sink(Box::new(sink));
    player.tick(sheet.absolute_end_time().expect("原点は刻印済み"));
}

/// `dir` に `descript.txt`（`charset,UTF-8` の頭＋`body`＋末尾の改行・CRLF）を置く。
pub(crate) fn write_descript(dir: &Path, body: &str) {
    std::fs::create_dir_all(dir).expect("検体のフォルダを作る");
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{body}\r\n"),
    )
    .expect("descript.txt を書く");
}

/// 読むことだけを共有して開いたまま持つ（起動中の `shiori.dll` と同じ状態＝`areka-nar` のテストの
/// 助手と同じ形）。共有を 0 にすると組み上げの段の複写が先に落ちて確定まで届かない。
pub(crate) fn hold(path: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1 /* FILE_SHARE_READ */)
        .open(path)
        .unwrap_or_else(|err| panic!("{} を掴めるはず: {err}", path.display()))
}

/// `event` の記録（`event` 欄で引く）。
pub(crate) fn events_named<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    event: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// warn 以上の記録。
pub(crate) fn warn_or_worse(
    events: &[log_capture_kit::CapturedEvent],
) -> Vec<&log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .collect()
}

/// `event` の記録のうち水準が `level` の件数。
pub(crate) fn count(
    events: &[log_capture_kit::CapturedEvent],
    event: &str,
    level: tracing::Level,
) -> usize {
    events_named(events, event)
        .iter()
        .filter(|e| e.level == level)
        .count()
}

/// 今のゴースト・バルーンの決め方を書き換える（argv で決まった側は最後に使ったものを書かない）。
pub(crate) fn routes(world: &mut World, ghost: GhostRoute, balloon: BalloonRoute) {
    let mut ctx = world.resource_mut::<BootContext>();
    ctx.current.ghost.route = ghost;
    ctx.current.balloon.route = balloon;
}

mod body_systems_tests;
mod boot_config_balloon_tests;
mod boot_config_callghost_tests;
mod boot_config_copy_tests;
mod boot_config_input_tests;
mod boot_config_module_policy_tests;
mod boot_config_option_tests;
mod boot_resolve_mark_text_tests;
mod ghost_session_reload_tests;
mod ghost_session_restart_tests;
mod ghost_session_scope_rect_boot_tests;
mod ghost_session_scope_scale_boot_tests;
mod ghost_session_strict_tests;
mod ghost_session_switch_tests;
mod log_history_convention_body_tests;
mod readme_body_tests;
mod readonly_body_tests;
mod readonly_run_tests;
mod readonly_sites_tests;
mod reload_tests;
mod session_end_deadline_body_tests;
mod session_end_tests;
mod shell_balloon_switch_session_tests;
mod shell_balloon_switch_session_update_tests;
mod shell_balloon_switch_tests;
mod write_failure_sites_tests;
