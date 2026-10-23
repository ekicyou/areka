//! 走りの状態の口の決定論テスト（readonly-mode 要件 1.1・1.2・1.6・8.1・8.3）。
//!
//! 試験のプロセスでは `engage` を呼ばない（旗はプロセスに 1 つで、戻す口が無い）。
//! そのことは本番の源の走査（構造の試験）で固める。

use std::path::Path;

use super::*;

#[test]
fn flag_is_off_and_install_adds_nothing_in_the_test_process() {
    assert!(!is_on(), "試験のプロセスでは旗が立たない");
    let mut world = World::new();
    install(&mut world);
    assert!(
        !world.contains_resource::<ReadOnlyRun>(),
        "旗が偽なら印を足さない"
    );
    assert!(!of(&world));
}

#[test]
fn of_follows_the_resource() {
    let mut world = World::new();
    assert!(!of(&world), "印が無ければ偽");
    world.insert_resource(ReadOnlyRun);
    assert!(of(&world), "印が在れば真");
}

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
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = all_sources(&src);
    let def = ["fn engage", "("].concat();
    // 較正: 走査が main.rs の本文を見ている。
    let calibration = ["mod ", "readonly;"].concat();
    assert!(files_calling(&files, &calibration, &def).contains(&"main.rs".to_owned()));

    let mut outside: Vec<String> = files_calling(&files, &["readonly::", "engage"].concat(), &def)
        .into_iter()
        .filter(|rel| rel != "main.rs")
        .collect();
    // `use super::*` で名前だけで呼べる所（このモジュールとその試験）は、裸の呼び出しも禁じる。
    let bare: Vec<(String, String)> = files
        .iter()
        .filter(|(rel, _)| rel == "readonly.rs" || rel.starts_with("readonly_"))
        .cloned()
        .collect();
    outside.extend(files_calling(&bare, &["engage", "("].concat(), &def));
    assert!(
        outside.is_empty(),
        "main.rs の外から engage を呼ぶ所: {outside:?}"
    );
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
            if krate == "areka" && rel == "readonly.rs" {
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

/// 置き場の木（下の階層まで、相対パスと中身）。
fn tree(dir: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        for e in std::fs::read_dir(dir).expect("木が読めない") {
            let p = e.expect("木の項目").path();
            let content = if p.is_dir() {
                walk(root, &p, out);
                Vec::new()
            } else {
                std::fs::read(&p).expect("ファイルが読めない")
            };
            out.push((p.strip_prefix(root).unwrap().to_path_buf(), content));
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// アプリもゴーストも書けない置き場（フォルダの在るべき所に普通のファイル）。根は `target\` の下。
fn unwritable_places(
    tmp: &temp_path_kit::TempPath,
) -> (crate::boot_config::ConfigInputs, std::path::PathBuf) {
    let app = tmp.child("app-profile");
    std::fs::write(&app, "x").unwrap();
    let root = tmp.child("ghost-a");
    std::fs::create_dir_all(root.join("ghost").join("master")).unwrap();
    std::fs::write(root.join("ghost").join("master").join("profile"), "x").unwrap();
    let cfg = crate::boot_config::ConfigInputs {
        ghost_root: root,
        balloon_root: tmp.child("balloon"),
    };
    (cfg, app)
}

/// 書けない置き場でも、読み取りだけなら通り、記録が 1 件出て、木が変わらない（要件 5.1・5.2）。
#[test]
fn boot_check_passes_readonly_without_touching_the_tree() {
    let tmp = temp_path_kit::TempPath::under_target("readonly-boot-check-skip");
    let (cfg, app) = unwritable_places(&tmp);
    let before = tree(tmp.path());

    let (result, events) =
        log_capture_kit::capture(|| crate::write_failure::check_boot(true, &cfg, &app));

    assert!(result.is_ok(), "{result:?}");
    let skipped = events
        .iter()
        .filter(|e| e.field_str("event") == Some("readonly_boot_check_skipped"))
        .count();
    assert_eq!(skipped, 1, "{events:?}");
    assert_eq!(tree(tmp.path()), before, "確かめが木を変えた");
}

/// 読み取りだけでなければ、今の確かめと同じ失敗を返し、飛ばした記録は出ない（要件 6.4）。
#[test]
fn boot_check_without_readonly_is_the_current_check() {
    let tmp = temp_path_kit::TempPath::under_target("readonly-boot-check-normal");
    let (cfg, app) = unwritable_places(&tmp);

    let expected = crate::write_failure::check_boot_dirs(&cfg, &app).expect_err("書けない");
    let (result, events) =
        log_capture_kit::capture(|| crate::write_failure::check_boot(false, &cfg, &app));

    assert_eq!(result.expect_err("今の確かめと同じく止まる"), expected);
    assert!(
        events
            .iter()
            .all(|e| e.field_str("event") != Some("readonly_boot_check_skipped")),
        "{events:?}"
    );
}
