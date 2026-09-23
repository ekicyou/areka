//! `catalog` の核となる採否と素性の決定論テスト（要件 2.1〜2.8・5.3）。
//!
//! 根は `temp-path-kit` の一時フォルダへその場で組む。網羅の拡張（検体の根・偽の
//! `StayseeBalloon` 等）は task 2.2 が本ファイルと支援ヘルパへ足す。

use super::*;
use crate::test_log_capture::{assert_logged_event, capture};
use std::fs;
use temp_path_kit::TempPath;
use tracing::Level;

const TARGET: &str = "areka_ghost::catalog";

/// `path` に `body` を書く（親フォルダも作る）。
fn put(path: &Path, body: &[u8]) {
    fs::create_dir_all(path.parent().expect("親")).expect("親フォルダ作成");
    fs::write(path, body).expect("書き込み");
}

/// `<根>/ghost/<folder>/ghost/master/descript.txt` を置く。
fn put_ghost(root: &BasewareRoot, folder: &str, descript: &str) -> PathBuf {
    let dir = root.ghost_dir(folder);
    put(
        &dir.join("ghost").join("master").join("descript.txt"),
        descript.as_bytes(),
    );
    dir
}

#[test]
fn basewareroot_composes_store_paths_without_touching_fs() {
    let root = BasewareRoot::new(PathBuf::from("Z:/no/such/root"));
    assert_eq!(root.dir(), Path::new("Z:/no/such/root"));
    assert_eq!(
        root.ghost_store(),
        Path::new("Z:/no/such/root").join("ghost")
    );
    assert_eq!(
        root.balloon_store(),
        Path::new("Z:/no/such/root").join("balloon")
    );
    assert_eq!(
        root.ghost_dir("a"),
        Path::new("Z:/no/such/root").join("ghost").join("a")
    );
    assert_eq!(
        root.balloon_dir("b"),
        Path::new("Z:/no/such/root").join("balloon").join("b")
    );
}

#[test]
fn missing_stores_list_zero() {
    let tmp = TempPath::new("catalog-missing");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    assert!(list_ghosts(&root).is_empty());
    assert!(list_balloons(&root).is_empty());
    assert!(list_shells(&root.ghost_dir("nobody")).is_empty());
}

#[test]
fn ghosts_are_folders_with_master_descript_in_byte_order() {
    let tmp = TempPath::new("catalog-ghosts");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_ghost(&root, "b", "charset,UTF-8\nname,Bee\n");
    // Windows の fs は大小を区別しないので、大文字は別の綴り（`Z`）で置く。
    put_ghost(&root, "Z", "charset,UTF-8\nName,Upper\n");
    put_ghost(&root, "a", "charset,UTF-8\n");
    // descript の無いフォルダとフォルダ以外の項目は返さない（要件 2.1）。
    fs::create_dir_all(root.ghost_dir("no-descript").join("ghost").join("master")).unwrap();
    put(&root.ghost_store().join("stray.txt"), b"x");

    let listed = list_ghosts(&root);
    let folders: Vec<&str> = listed.iter().map(|g| g.identity.folder.as_str()).collect();
    assert_eq!(folders, ["Z", "a", "b"], "バイト順（要件 2.5）");
    assert_eq!(listed[0].dir, root.ghost_dir("Z"));
    // 鍵は ASCII 小文字化して引く（R5）。
    assert_eq!(listed[0].identity.name.as_deref(), Some("Upper"));
    assert_eq!(listed[1].identity.name, None);
}

#[test]
fn identity_reads_seven_items_and_decodes_shift_jis() {
    let tmp = TempPath::new("catalog-identity");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    // Shift_JIS の生バイト: 「吹き出し」＝90 81 82 AB 8F 6F 82 B5・「作者」＝8D EC 8E D2（要件 2.8）。
    let mut sjis = b"charset,Shift_JIS\nname,".to_vec();
    sjis.extend_from_slice(&[0x90, 0x81, 0x82, 0xAB, 0x8F, 0x6F, 0x82, 0xB5]);
    sjis.extend_from_slice(b"\ncraftman,maker\ncraftmanw,");
    sjis.extend_from_slice(&[0x8D, 0xEC, 0x8E, 0xD2]);
    sjis.extend_from_slice(b"\nid,IdX\nreadme,manual.txt\n");
    let dir = root.balloon_dir("sjis");
    put(&dir.join("descript.txt"), &sjis);
    put(&dir.join("manual.txt"), b"m");
    put(&dir.join("thumbnail.png"), b"p");

    let listed = list_balloons(&root);
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].identity,
        Identity {
            folder: "sjis".into(),
            name: Some("吹き出し".into()),
            craftman: Some("maker".into()),
            craftmanw: Some("作者".into()),
            id: Some("IdX".into()),
            readme: Some(dir.join("manual.txt")),
            has_thumbnail: true,
        }
    );
}

#[test]
fn readme_falls_back_to_readme_txt_only_when_present() {
    let tmp = TempPath::new("catalog-readme");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let with = put_ghost(&root, "with", "charset,UTF-8\n");
    put(&with.join("readme.txt"), b"r");
    // readme 鍵が実在しないファイルを指す → 無し（readme.txt へは倒さない）。
    let dangling = put_ghost(&root, "dangling", "charset,UTF-8\nreadme,gone.txt\n");
    put(&dangling.join("readme.txt"), b"r");

    let listed = list_ghosts(&root);
    assert_eq!(listed[0].identity.folder, "dangling");
    assert_eq!(listed[0].identity.readme, None);
    assert_eq!(listed[1].identity.readme, Some(with.join("readme.txt")));
    assert!(!listed[1].identity.has_thumbnail);
}

#[test]
fn shells_exclude_menu_hidden_and_folders_without_descript() {
    let tmp = TempPath::new("catalog-shells");
    let ghost = tmp.path().join("g");
    let shell = ghost.join("shell");
    put(
        &shell.join("master").join("descript.txt"),
        b"charset,UTF-8\nname,Master\n",
    );
    put(
        &shell.join("secret").join("descript.txt"),
        b"charset,UTF-8\nMENU, Hidden \n",
    );
    fs::create_dir_all(shell.join("empty")).unwrap();

    let listed = list_shells(&ghost);
    let folders: Vec<&str> = listed.iter().map(|s| s.identity.folder.as_str()).collect();
    assert_eq!(folders, ["master"]);
    assert_eq!(listed[0].dir, shell.join("master"));
}

#[test]
fn balloons_keep_absent_or_balloon_type_and_warn_on_others() {
    let tmp = TempPath::new("catalog-balloons");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put(
        &root.balloon_dir("plain").join("descript.txt"),
        b"charset,UTF-8\n",
    );
    put(
        &root.balloon_dir("typed").join("descript.txt"),
        b"charset,UTF-8\nTYPE, Balloon\n",
    );
    put(
        &root.balloon_dir("plugin").join("descript.txt"),
        b"charset,UTF-8\ntype,plugin\n",
    );

    let mut listed = Vec::new();
    let events = capture(|| listed = list_balloons(&root));
    let folders: Vec<&str> = listed.iter().map(|b| b.identity.folder.as_str()).collect();
    assert_eq!(folders, ["plain", "typed"]);
    assert_logged_event(&events, Level::WARN, TARGET, "catalog_type_not_balloon");
}

#[test]
fn companion_balloon_reads_one_key() {
    let tmp = TempPath::new("catalog-install");
    let with = tmp.path().join("with");
    put(
        &with.join("install.txt"),
        b"charset,UTF-8\nBalloon.Directory, kaku \nballoon0.directory,other\n",
    );
    assert_eq!(companion_balloon(&with).as_deref(), Some("kaku"));
    assert_eq!(companion_balloon(&tmp.path().join("none")), None);
}

#[test]
fn is_ghost_dir_checks_master_descript_only() {
    let tmp = TempPath::new("catalog-isghost");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let yes = put_ghost(&root, "yes", "");
    assert!(is_ghost_dir(&yes));
    assert!(!is_ghost_dir(&root.ghost_store()));
    assert!(!is_ghost_dir(&tmp.path().join("nothing")));
}
