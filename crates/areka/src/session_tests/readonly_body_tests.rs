//! 走りの状態の口の呼び手を、本体の `src` とワークスペースの木を歩いて固める構造の試験。

use std::path::Path;

/// `src_root` の下の `.rs` を全部（試験と試験の土台も含む）`(src/ からの相対パス, 本文)` で集める。
fn all_sources(src_root: &Path) -> Vec<(String, String)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = std::fs::read_dir(dir).unwrap_or_else(|e| {
            panic!("木を歩けない（走査が空振りする）: {} — {e}", dir.display())
        });
        for entry in entries {
            let path = entry.expect("ディレクトリ項目が読めない").path();
            if path.is_dir() {
                walk(root, &path, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .expect("走査の根の下に無い")
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&path).expect("源のファイルが読めない");
            out.push((rel, src));
        }
    }
    let mut out = Vec::new();
    walk(src_root, src_root, &mut out);
    assert!(!out.is_empty(), "走査が空振りした: {}", src_root.display());
    out
}

/// 字面 `needle` を含む行（註釈の行と、定義の行 `def` を除く）を持つファイルの相対パス。
fn files_calling(files: &[(String, String)], needle: &str, def: &str) -> Vec<String> {
    files
        .iter()
        .filter(|(_, src)| {
            src.lines()
                .map(str::trim)
                .filter(|line| !line.starts_with("//") && !line.contains(def))
                .any(|line| line.contains(needle))
        })
        .map(|(rel, _)| rel.clone())
        .collect()
}

// 針は実行時に組む（この試験の源そのものが当たらないように）。

#[test]
fn engage_is_called_only_from_main() {
    let mut files = all_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    files.extend(
        all_sources(&areka_session::crate_source_test_support::crate_path(
            "areka-session",
            "src",
        ))
        .into_iter()
        .map(|(rel, src)| (format!("areka-session/{rel}"), src)),
    );
    let def = ["fn engage", "("].concat();
    // 較正: 走査が main.rs の本文を見ている。
    let calibration = ["use areka_", "session::"].concat();
    assert!(files_calling(&files, &calibration, &def).contains(&"main.rs".to_owned()));

    let mut outside: Vec<String> = files_calling(&files, &["readonly::", "engage"].concat(), &def)
        .into_iter()
        .filter(|rel| rel != "main.rs")
        .collect();
    // `use super::*` で名前だけで呼べる所（このモジュールとその試験）は、裸の呼び出しも禁じる。
    let bare: Vec<(String, String)> = files
        .iter()
        .filter(|(rel, _)| {
            rel == "areka-session/readonly.rs" || rel.starts_with("areka-session/readonly_")
        })
        .cloned()
        .collect();
    assert!(
        !bare.is_empty(),
        "readonly のモジュールとその試験が走査に見えない"
    );
    outside.extend(files_calling(&bare, &["engage", "("].concat(), &def));
    assert!(
        outside.is_empty(),
        "main.rs の外から engage を呼ぶ所: {outside:?}"
    );
}

/// `main.rs` が入れの口を 1 度だけ、起動の値を解く段（`resolve_boot`）より前で呼ぶ。
#[test]
fn main_calls_engage_exactly_once_before_resolve_boot() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
        .expect("main.rs が読めない");
    let code: Vec<&str> = src
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect();
    let needle = ["readonly::eng", "age("].concat();
    let calls: Vec<usize> = code
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(&needle))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(calls.len(), 1, "main.rs の engage の呼び出し: {calls:?}");
    let resolve = code
        .iter()
        .position(|line| line.contains(&["resolve_", "boot(&"].concat()))
        .expect("main.rs に resolve_boot の呼び出しが無い（走査が空振りした）");
    assert!(calls[0] < resolve, "engage が resolve_boot より後");
}

#[test]
fn withhold_everything_is_called_only_from_readonly() {
    // 定義の在る sylphya も含め、ワークスペースのクレートの源（試験も含む）を全部見る。
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ が無い");
    let needle = ["withhold_", "everything("].concat();
    let def = ["fn ", &needle].concat();
    let mut outside = Vec::new();
    let mut seen_in_readonly = false;
    for entry in std::fs::read_dir(crates).expect("crates/ が読めない") {
        let src = entry
            .expect("ディレクトリ項目が読めない")
            .path()
            .join("src");
        if !src.is_dir() {
            continue;
        }
        let krate = src
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        for rel in files_calling(&all_sources(&src), &needle, &def) {
            if krate == "areka-session" && rel == "readonly.rs" {
                seen_in_readonly = true;
            } else {
                outside.push(format!("{krate}/src/{rel}"));
            }
        }
    }
    // 較正: 走査が readonly.rs の呼び出しを見つける（空振りしていない）。
    assert!(seen_in_readonly, "readonly.rs の呼び出しが走査に見えない");
    assert!(
        outside.is_empty(),
        "readonly.rs の外から withhold_everything を呼ぶ所: {outside:?}"
    );
}
