//! 値だけを受ける内側の口（`Config::from_inputs`）を、ワークツリーの `target\` の下に組んだ根で呼ぶテスト
//! （spec: bridge-store-user-root 要件 2.2〜2.11・3.2・3.3・3.8・6.3〜6.7）。
//!
//! 偽の `LOCALAPPDATA` と exe のフォルダ（空の `areka.exe`・隣の記憶 `profile\areka\sylphya.toml`）を組み、
//! 記録を捕まえながら呼ぶ。プロセスの環境変数は書き換えない（値と `Signals` を注入する）。
//! 控えを持たない場面では、続けて控えを保存しても木が同じこと（作らない・写さない）を確かめる。

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use areka_user_data::{Signals, Writability};
use log_capture_kit::{CapturedEvent, capture};
use rmcp::model::{ListToolsResult, ServerPeerInfo};
use serde_json::json;
use temp_path_kit::TempPath;

use super::*;
use crate::cache::Cache;

/// W1 の文（`log_without_cache`）。
const W1: &str = "[bridge] running without the cache until areka prepares its user root; it is used from the next start";
/// W2 の文。
const W2: &str = "[bridge] the user root of areka could not be located; running without the cache";

/// 組んだ根: 一時フォルダ・exe のフォルダ・偽の `LOCALAPPDATA`・まだ無い書ける根（`LOCALAPPDATA\areka`）。
struct Fixture {
    tmp: TempPath,
    exe: PathBuf,
    lad: PathBuf,
    root: PathBuf,
}

fn fixture(label: &str) -> Fixture {
    let tmp = TempPath::under_target(label);
    let exe = tmp.child("exe");
    fs::create_dir_all(profile(&exe)).unwrap();
    fs::write(exe.join("areka.exe"), b"").unwrap();
    fs::write(profile(&exe).join("sylphya.toml"), "old = 1").unwrap();
    let lad = tmp.child("lad");
    fs::create_dir_all(&lad).unwrap();
    let root = lad.join("areka");
    Fixture {
        tmp,
        exe,
        lad,
        root,
    }
}

/// winget の控え在りの入力（書ける根は `LOCALAPPDATA\areka`）。
fn winget_signals(lad: Option<&Path>) -> Signals {
    Signals {
        user_root_env: None,
        local_app_data: lad.map(OsString::from),
        winget_marker: Some("Areka.Areka.Portable_winget.db".to_owned()),
        writability: Writability::Writable,
    }
}

fn profile(dir: &Path) -> PathBuf {
    dir.join("profile").join("areka")
}

/// 木の写し（相対パスと、ファイルなら中身）。並びは名前順。
fn tree(dir: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).unwrap().flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(dir).unwrap().to_path_buf();
            if path.is_dir() {
                stack.push(path);
                out.push((rel, None));
            } else {
                out.push((rel, Some(fs::read(&path).unwrap())));
            }
        }
    }
    out.sort();
    out
}

/// 記録を捕まえながら 1 回呼ぶ（中継の `LOCALAPPDATA` は偽の `lad`）。
fn run(
    fx: &Fixture,
    profile_env: Option<&Path>,
    areka: Option<&Signals>,
) -> (Config, Vec<CapturedEvent>) {
    let (config, events) = capture(|| {
        Config::from_inputs(
            None,
            profile_env.map(Path::as_os_str),
            areka.map(|signals| (fx.exe.as_path(), signals)),
            Some(fx.lad.clone()),
        )
    });
    assert_bridge_lines_ascii(&events);
    (config, events)
}

/// 控えの 2 つのファイルを保存する（起動の後に中継が答えを受け取った形）。
fn save_both(config: &Config) {
    let info: ServerPeerInfo = serde_json::from_value(json!({
        "protocolVersion": "2025-11-25",
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "areka-mcp-server", "version": "0.0.3" },
    }))
    .unwrap();
    let tools: ListToolsResult = serde_json::from_value(
        json!({ "tools": [{ "name": "t", "inputSchema": { "type": "object" } }] }),
    )
    .unwrap();
    let cache = Cache::new(config.cache_dir.clone());
    cache.save_tools(&tools);
    cache.save_server_info(&info);
}

/// 控え無しで、続けて保存しても木が同じ（根も `profile\areka` も作られず、exe の隣の中身が同じ）。
fn assert_no_cache_and_nothing_written(
    fx: &Fixture,
    config: &Config,
    before: &[(PathBuf, Option<Vec<u8>>)],
) {
    assert_eq!(config.cache_dir, None, "控え無し");
    save_both(config);
    assert_eq!(
        tree(fx.tmp.path()),
        before,
        "作らない・写さない・書き換えない"
    );
}

/// 保存の後に増えた物が `cache_dir` の下（とそこへ至るフォルダ）だけ。
fn assert_growth_only_under(fx: &Fixture, cache_dir: &Path, before: &[(PathBuf, Option<Vec<u8>>)]) {
    save_both(&Config::from_parts(
        Vec::new(),
        Some(cache_dir.to_path_buf()),
        Timeouts::default(),
    ));
    let rel = cache_dir.strip_prefix(fx.tmp.path()).unwrap();
    let after = tree(fx.tmp.path());
    let grown: Vec<_> = after.iter().filter(|e| !before.contains(e)).collect();
    assert!(
        before.iter().all(|e| after.contains(e)),
        "前から在った物は変わらない"
    );
    assert!(!grown.is_empty(), "控えが書かれた");
    for (path, contents) in grown {
        // 控えのフォルダを作る `create_dir_all` が、まだ無い親のフォルダも作る。
        let ancestor = contents.is_none() && rel.starts_with(path);
        assert!(
            path.starts_with(rel) || ancestor,
            "控えの外に増えた: {path:?}"
        );
    }
}

/// 頭が `[bridge]` の行の文と、決まった語の欄（`reason`）が ASCII（要件 3.8）。パスの欄は見ない。
fn assert_bridge_lines_ascii(events: &[CapturedEvent]) {
    for event in events
        .iter()
        .filter(|e| e.message().starts_with("[bridge]"))
    {
        assert!(event.message().is_ascii(), "{}", event.message());
        if let Some(reason) = event.field_str("reason") {
            assert!(reason.is_ascii(), "{reason}");
        }
    }
}

fn with_message<'a>(events: &'a [CapturedEvent], message: &str) -> Vec<&'a CapturedEvent> {
    events.iter().filter(|e| e.message() == message).collect()
}

fn bridge_lines(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.message().starts_with("[bridge]"))
        .collect()
}

/// W1 がちょうど 1 件（理由の語と根つき）で、ほかの `[bridge]` の行が無い。
fn assert_only_w1(events: &[CapturedEvent], reason: &str, root: &Path) {
    let lines = bridge_lines(events);
    assert_eq!(lines.len(), 1, "中継の行は W1 の 1 件だけ");
    assert_eq!(lines[0].message(), W1);
    assert_eq!(lines[0].level, tracing::Level::INFO);
    assert_eq!(lines[0].field_str("reason"), Some(reason));
    assert_eq!(
        lines[0].field("user_root"),
        Some(root.display().to_string().as_str())
    );
}

/// 要件 2.4・3.2: 根がまだ無い → 控え無し・W1（`user_root_missing`）。保存しても根は作られない（2.2・6.5）。
#[test]
fn absent_root_runs_without_cache_and_creates_nothing() {
    let fx = fixture("bridge-prep-absent");
    let before = tree(fx.tmp.path());
    let (config, events) = run(&fx, None, Some(&winget_signals(Some(&fx.lad))));
    assert_only_w1(&events, "user_root_missing", &fx.root);
    assert_no_cache_and_nothing_written(&fx, &config, &before);
    assert!(!fx.root.exists(), "書ける根は作らない");
}

/// 要件 2.5・3.2: 根は在るが記憶がまだ写されていない → 控え無し・W1（`app_memory_not_migrated`）。
/// 保存しても根の `profile\areka` は作られず、exe の隣の中身が同じ（2.3・6.5）。
#[test]
fn pending_migration_runs_without_cache_and_copies_nothing() {
    let fx = fixture("bridge-prep-pending");
    fs::create_dir_all(&fx.root).unwrap();
    let before = tree(fx.tmp.path());
    let (config, events) = run(&fx, None, Some(&winget_signals(Some(&fx.lad))));
    assert_only_w1(&events, "app_memory_not_migrated", &fx.root);
    assert_no_cache_and_nothing_written(&fx, &config, &before);
    assert!(
        !profile(&fx.root).exists(),
        "根の profile\\areka は作らない"
    );
}

/// 要件 2.6: 根の記憶が在る → 根の `profile\areka\mcp-bridge`。中継の行は 0 件。
#[test]
fn root_memory_present_puts_cache_under_root() {
    let fx = fixture("bridge-prep-ready");
    fs::create_dir_all(profile(&fx.root)).unwrap();
    fs::write(profile(&fx.root).join("sylphya.toml"), "new = 2").unwrap();
    let before = tree(fx.tmp.path());
    let (config, events) = run(&fx, None, Some(&winget_signals(Some(&fx.lad))));
    let expected = profile(&fx.root).join("mcp-bridge");
    assert_eq!(config.cache_dir.as_deref(), Some(expected.as_path()));
    assert!(bridge_lines(&events).is_empty(), "W1〜W3 は 0 件");
    assert_eq!(tree(fx.tmp.path()), before, "決めるだけでは作らない");
    assert_growth_only_under(&fx, &expected, &before);
}

/// 要件 2.6: 根は在るが記憶がどこにも無い → 同じ置き場で、保存で増えるのはその下だけ。
#[test]
fn root_without_memory_anywhere_puts_cache_under_root() {
    let fx = fixture("bridge-prep-ready-empty");
    fs::remove_dir_all(fx.exe.join("profile")).unwrap();
    fs::create_dir_all(&fx.root).unwrap();
    let before = tree(fx.tmp.path());
    let (config, events) = run(&fx, None, Some(&winget_signals(Some(&fx.lad))));
    let expected = profile(&fx.root).join("mcp-bridge");
    assert_eq!(config.cache_dir.as_deref(), Some(expected.as_path()));
    assert!(bridge_lines(&events).is_empty(), "W1〜W3 は 0 件");
    assert_eq!(tree(fx.tmp.path()), before, "決めるだけでは作らない");
    assert_growth_only_under(&fx, &expected, &before);
}

/// 要件 2.7・3.3: 決められない（`LOCALAPPDATA` が無い・`AREKA_USER_ROOT` が相対）→ exe の隣に置かず控え無し。
/// `areka-user-data` の error（`user_root_unavailable`）の後に W2 が warn で 1 件（順を固める）。
#[test]
fn unlocatable_root_runs_without_cache_after_the_user_data_error() {
    let cases = [
        ("bridge-prep-unloc-lad", None),
        ("bridge-prep-unloc-rel", Some(OsString::from("rel\\root"))),
    ];
    for (label, user_root_env) in cases {
        let fx = fixture(label);
        let signals = Signals {
            user_root_env,
            // 相対の根の場面は `LOCALAPPDATA` が在っても決められない（そこへ落ちない）。
            ..winget_signals(if label.ends_with("rel") {
                Some(&fx.lad)
            } else {
                None
            })
        };
        let before = tree(fx.tmp.path());
        let (config, events) = run(&fx, None, Some(&signals));
        assert_no_cache_and_nothing_written(&fx, &config, &before);
        assert!(
            !profile(&fx.exe).join("mcp-bridge").exists(),
            "{label}: exe の隣に置かない"
        );

        let error_at = events
            .iter()
            .position(|e| e.field_str("event") == Some("user_root_unavailable"))
            .unwrap_or_else(|| panic!("{label}: areka-user-data の error"));
        assert_eq!(events[error_at].level, tracing::Level::ERROR, "{label}");
        let lines = bridge_lines(&events);
        assert_eq!(lines.len(), 1, "{label}: 中継の行は W2 の 1 件だけ");
        assert_eq!(lines[0].message(), W2, "{label}");
        assert_eq!(lines[0].level, tracing::Level::WARN, "{label}");
        let w2_at = events.iter().position(|e| e.message() == W2).unwrap();
        assert!(error_at < w2_at, "{label}: error → W2 の順");
    }
}

/// 要件 2.8: zip の形（winget の控え無し・書ける）→ exe の隣の `profile\areka\mcp-bridge`。
#[test]
fn zip_layout_puts_cache_beside_exe() {
    let fx = fixture("bridge-prep-zip");
    let signals = Signals {
        winget_marker: None,
        ..winget_signals(Some(&fx.lad))
    };
    let (config, events) = run(&fx, None, Some(&signals));
    assert_eq!(config.cache_dir, Some(profile(&fx.exe).join("mcp-bridge")));
    assert!(bridge_lines(&events).is_empty());
    assert!(!fx.root.exists());
}

/// 要件 2.9: `AREKA_PROFILE_DIR` 在り → その下の `mcp-bridge`・根は作られない・W1 は 0 件。
#[test]
fn profile_env_puts_cache_under_it_without_touching_root() {
    let fx = fixture("bridge-prep-profile-env");
    let dir = fx.tmp.child("profile-env");
    let before = tree(fx.tmp.path());
    let (config, events) = run(&fx, Some(&dir), Some(&winget_signals(Some(&fx.lad))));
    assert_eq!(config.cache_dir, Some(dir.join("mcp-bridge")));
    assert!(with_message(&events, W1).is_empty(), "W1 は 0 件");
    assert!(bridge_lines(&events).is_empty());
    assert_eq!(tree(fx.tmp.path()), before, "決めるだけでは作らない");
    assert!(!fx.root.exists(), "書ける根は作らない");
}

/// 要件 2.10: 隣に areka が無い → `LOCALAPPDATA\areka\mcp-bridge`。
#[test]
fn no_areka_beside_puts_cache_under_local_app_data() {
    let fx = fixture("bridge-prep-no-areka");
    let (config, events) = run(&fx, None, None);
    assert_eq!(config.cache_dir, Some(fx.root.join("mcp-bridge")));
    assert!(bridge_lines(&events).is_empty());
}

/// 要件 2.11: 根がまだ無い起動は控え無し。areka が置き場を作った後の 2 回目の起動で控えが置かれる。
#[test]
fn second_start_after_areka_prepares_root_uses_cache() {
    let fx = fixture("bridge-prep-second-start");
    let signals = winget_signals(Some(&fx.lad));
    let (first, _) = run(&fx, None, Some(&signals));
    assert_eq!(first.cache_dir, None, "1 回目は控え無し");

    // areka が根を作り、記憶を写した形（exe の隣の記憶は写した後も残る）。
    fs::create_dir_all(profile(&fx.root)).unwrap();
    fs::write(profile(&fx.root).join("sylphya.toml"), "old = 1").unwrap();

    let (second, events) = run(&fx, None, Some(&signals));
    assert_eq!(second.cache_dir, Some(profile(&fx.root).join("mcp-bridge")));
    assert!(bridge_lines(&events).is_empty());
}
