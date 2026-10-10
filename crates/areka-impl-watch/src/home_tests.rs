//! 置き場所の解決の決定論テスト。値は引数で渡し、プロセスの環境変数は触らない
//! （テストは並んで走る）。向け先はワークツリーの `target\` の下の一時フォルダだけ。

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{Home, resolve};
use crate::error::WatchError;
use crate::state::WaitKind;

/// フォルダの直下に在るものの一覧（並べ替え済み）。
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| entry.expect("項目を読める").path())
        .collect();
    found.sort();
    found
}

#[test]
fn missing_or_empty_value_is_unset_and_creates_nothing() {
    // 値に行き先が無いので、作るとすれば相対の道筋が落ちる先（カレント＝このクレートのフォルダ）。
    let cwd = std::env::current_dir().expect("カレントが分かる");
    let before = entries(&cwd);
    for value in [None, Some(""), Some(" "), Some(" \t\r\n")] {
        let got = resolve(value.map(OsString::from));
        assert!(
            matches!(got, Err(WatchError::HomeUnset)),
            "{value:?}: {got:?}"
        );
    }
    assert_eq!(entries(&cwd), before, "既定の場所へ倒れて何かを作った");
}

#[test]
fn present_value_creates_the_folder_and_nothing_inside() {
    let root = TempPath::under_target("impl-watch-home");
    // 途中のフォルダもまだ無い。
    let dir = root.child("a").join("b").join("home");
    assert!(!dir.exists());

    let home = resolve(Some(dir.clone().into_os_string())).expect("作れる");

    assert_eq!(home.dir, dir);
    assert!(dir.is_dir());
    assert_eq!(
        entries(&dir),
        Vec::<PathBuf>::new(),
        "解決はフォルダだけを作る"
    );
}

#[test]
fn existing_folder_is_used_as_it_is() {
    let root = TempPath::under_target("impl-watch-home");
    let kept = root.child("state.json");
    std::fs::write(&kept, b"keep").expect("書ける");

    let home = resolve(Some(root.path().as_os_str().to_owned())).expect("在るフォルダは通る");

    assert_eq!(home.dir, root.path());
    assert_eq!(std::fs::read(&kept).expect("読める"), b"keep");
    assert_eq!(entries(root.path()), vec![kept]);
}

#[test]
fn a_place_that_cannot_be_a_folder_is_not_creatable() {
    let root = TempPath::under_target("impl-watch-home");
    let file = root.child("file.txt");
    std::fs::write(&file, b"keep").expect("書ける");

    // 指す先そのものがファイル／親がファイル（道筋には ASCII の外の字も入れる）。
    for dir in [file.clone(), file.join("置き場")] {
        let err = resolve(Some(dir.clone().into_os_string())).expect_err("フォルダにできない");
        let WatchError::HomeNotCreatable { dir: spelled, .. } = &err else {
            panic!("別の失敗: {err:?}");
        };
        assert_eq!(spelled.as_str(), dir.to_string_lossy());
        let text = err.to_string();
        assert!(
            text.starts_with("AREKA_IMPL_WATCH_HOME cannot be created: "),
            "{text}"
        );
        assert!(text.is_ascii(), "ASCII の外の字: {text}");
        assert!(!text.contains(['\r', '\n']), "1 行でない: {text:?}");
    }

    assert_eq!(std::fs::read(&file).expect("読める"), b"keep");
    assert_eq!(entries(root.path()), vec![file]);
}

#[test]
fn paths_under_the_home_use_the_design_names() {
    let dir = Path::new("H");
    let home = Home {
        dir: dir.to_path_buf(),
    };
    assert_eq!(home.state_path(), dir.join("state.json"));
    assert_eq!(home.lock_path(), dir.join("state.lock"));
    assert_eq!(home.status_path(), dir.join("status.md"));
    assert_eq!(home.log_path(), dir.join("impl-watch.log"));
    assert_eq!(home.alive_dir(), dir.join("alive"));
}

#[test]
fn alive_path_is_id_dot_kind_dot_lock_for_every_kind() {
    let home = Home {
        dir: PathBuf::from("H"),
    };
    // 識別は点を含みうる（拡張子の付け替えで組むと欠ける）。
    let id = "areka-P0.impl_watch";
    for (kind, name) in [
        (WaitKind::Watch, "watch"),
        (WaitKind::Merge, "merge"),
        (WaitKind::Load, "load"),
        (WaitKind::Resume, "resume"),
    ] {
        assert_eq!(
            home.alive_path(id, kind),
            Path::new("H")
                .join("alive")
                .join(format!("areka-P0.impl_watch.{name}.lock"))
        );
        // ファイル名の綴りは状態ファイルの綴りと同じ。
        assert_eq!(
            serde_json::to_string(&kind).expect("綴れる"),
            format!("\"{name}\"")
        );
    }
}
