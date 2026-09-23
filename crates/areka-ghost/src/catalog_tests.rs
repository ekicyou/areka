//! `catalog` の核となる採否と素性の決定論テスト（要件 2.1〜2.8・5.3）。
//!
//! 根は `temp-path-kit` の一時フォルダへその場で組むか（要件 8.1 の構成は
//! `catalog_test_support::temp_root`）、検体の根（`SampleRoot::acquire("emo2")`）を借りる。
//! 足すテストは列挙の採否と素性の判断分岐に限る（要件 8.4）。

use super::test_support::{TempRoot, put, put_ghost, temp_root};
use super::*;
use crate::test_log_capture::{CapturedEvent, assert_logged_event, capture};
use std::fs;
use temp_path_kit::TempPath;
use tracing::Level;

const TARGET: &str = "areka_ghost::catalog";

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

/// 空の値（trim 後に空）はどの鍵でも「無し」（仕様は空の値を定めない・実装時の裁定）。
#[test]
fn empty_values_count_as_absent() {
    let tmp = TempPath::new("catalog-empty");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let ghost = put_ghost(
        &root,
        "g",
        "charset,UTF-8\nname,\ncraftman, \ncraftmanw,\nid,\nreadme,\n",
    );
    put(
        &ghost.join("shell").join("s").join("descript.txt"),
        b"charset,UTF-8\nmenu,\n",
    );
    put(
        &ghost.join("install.txt"),
        b"charset,UTF-8\nballoon.directory, \n",
    );
    put(
        &root.balloon_dir("b").join("descript.txt"),
        b"charset,UTF-8\ntype,\n",
    );

    let ghosts = list_ghosts(&root);
    assert_eq!(
        ghosts[0].identity,
        Identity {
            folder: "g".into(),
            ..Identity::default()
        }
    );
    assert_eq!(list_shells(&ghost).len(), 1, "menu, は隠さない");
    assert_eq!(companion_balloon(&ghost), None);
    let mut balloons = Vec::new();
    let events = capture(|| balloons = list_balloons(&root));
    assert_eq!(balloons.len(), 1, "type, は type 無しと同じ");
    assert!(
        events.iter().all(|e| e.target != TARGET),
        "type, は warn! しない: {events:?}"
    );
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

/// 捕捉列のうち本モジュールの `warn!` の `event` を並びのまま返す。
fn catalog_warns(events: &[CapturedEvent]) -> Vec<&str> {
    events
        .iter()
        .filter(|e| e.target == TARGET && e.level == Level::WARN)
        .map(|e| e.event.as_deref().unwrap_or("<event 無し>"))
        .collect()
}

/// 他者が読めないよう共有なしで開いたまま持つ（`fs::read` を I/O 失敗させる・有無の検査は通る）。
fn hold_exclusive(path: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .expect("共有なしで開く")
}

/// 要件 8.1 の一時の根: 件数・採否・7 項目・バイト順・Shift_JIS の復号を全部突き合わせる。
#[test]
fn temp_root_lists_requirement_8_1_layout() {
    let TempRoot { _tmp, root } = temp_root("catalog-layout");
    let alpha = root.ghost_dir("alpha");

    let mut ghosts = Vec::new();
    let mut balloons = Vec::new();
    let mut shells = Vec::new();
    let events = capture(|| {
        ghosts = list_ghosts(&root);
        balloons = list_balloons(&root);
        shells = list_shells(&alpha);
    });

    // ゴースト 2 体（descript の無い `empty`・フォルダ以外の `stray.txt` は返さない＝要件 2.1）。
    assert_eq!(
        ghosts,
        [
            GhostEntry {
                dir: alpha.clone(),
                identity: Identity {
                    folder: "alpha".into(),
                    name: Some("Alpha".into()),
                    craftman: Some("maker".into()),
                    craftmanw: Some("作り手".into()),
                    id: Some("AlphaId".into()),
                    readme: Some(alpha.join("manual.txt")),
                    has_thumbnail: true,
                },
            },
            GhostEntry {
                dir: root.ghost_dir("beta"),
                identity: Identity {
                    folder: "beta".into(),
                    name: Some("Beta".into()),
                    ..Identity::default()
                },
            },
        ]
    );
    // シェル 2 つのうち `menu,hidden` を除く 1 つ（要件 2.2）。
    let shell = alpha.join("shell").join("master");
    assert_eq!(
        shells,
        [ShellEntry {
            dir: shell.clone(),
            identity: Identity {
                folder: "master".into(),
                name: Some("Shell".into()),
                readme: Some(shell.join("readme.txt")),
                ..Identity::default()
            },
        }]
    );
    // バルーン: `type` 無しの偽の既定と `type,balloon`。`plugin` は warn!＋除外・`empty` は黙って
    // 除外（要件 2.3）。大文字始まりの `StayseeBalloon` が先＝バイト順（要件 2.5）。
    let staysee = root.balloon_dir("StayseeBalloon");
    assert_eq!(
        balloons,
        [
            BalloonEntry {
                dir: staysee.clone(),
                identity: Identity {
                    folder: "StayseeBalloon".into(),
                    name: Some("吹き出し".into()),
                    craftman: Some("stayse".into()),
                    craftmanw: Some("作者".into()),
                    id: Some("StayseeBalloon".into()),
                    readme: Some(staysee.join("readme.txt")),
                    has_thumbnail: false,
                },
            },
            BalloonEntry {
                dir: root.balloon_dir("kaku"),
                identity: Identity {
                    folder: "kaku".into(),
                    name: Some("Kaku".into()),
                    ..Identity::default()
                },
            },
        ]
    );
    assert_eq!(catalog_warns(&events), ["catalog_type_not_balloon"]);
    // 同梱バルーン名とゴーストの判定（要件 5.3・4.8）。
    assert_eq!(companion_balloon(&alpha).as_deref(), Some("kaku"));
    assert_eq!(companion_balloon(&root.ghost_dir("beta")), None);
    assert!(is_ghost_dir(&alpha));
    assert!(!is_ghost_dir(&root.ghost_dir("empty")));
    assert!(!is_ghost_dir(&root.balloon_dir("kaku")));
}

/// 検体の根（emo2＝ゴースト 1・バルーン 1・同梱の install.txt）。期待値は
/// `vendors/sample_ghost/emo2.nar` の実物から取った（ゴーストの descript の `id.emo2` は
/// カンマが無いので `id` は無し）。
#[test]
fn sample_root_emo2_lists_one_ghost_one_balloon() {
    let emo2 = sample_ghost_kit::SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let root = BasewareRoot::new(emo2.root().to_path_buf());
    let ghost = emo2.folder().to_path_buf();
    let kaku = emo2.balloon("emo2-kakukaku").expect("同梱").to_path_buf();

    let mut ghosts = Vec::new();
    let mut balloons = Vec::new();
    let mut shells = Vec::new();
    let events = capture(|| {
        ghosts = list_ghosts(&root);
        balloons = list_balloons(&root);
        shells = list_shells(&ghost);
    });

    assert_eq!(
        ghosts,
        [GhostEntry {
            dir: ghost.clone(),
            identity: Identity {
                folder: "emo2".into(),
                name: Some("えも？？".into()),
                craftman: Some("ekicyou".into()),
                craftmanw: Some("えちょ".into()),
                id: None,
                readme: Some(ghost.join("readme.txt")),
                has_thumbnail: false,
            },
        }]
    );
    assert_eq!(
        balloons,
        [BalloonEntry {
            dir: kaku.clone(),
            identity: Identity {
                folder: "emo2-kakukaku".into(),
                name: Some("kakukaku for emo-gs".into()),
                craftman: Some("ekicyou".into()),
                craftmanw: Some("えちょ".into()),
                ..Identity::default()
            },
        }]
    );
    let shell = ghost.join("shell").join("master");
    assert_eq!(
        shells,
        [ShellEntry {
            dir: shell.clone(),
            identity: Identity {
                folder: "master".into(),
                name: Some("「コンフィズリー」＆「City-Pop'n」".into()),
                craftman: Some("ekicyou".into()),
                craftmanw: Some("えちょ".into()),
                id: None,
                readme: Some(shell.join("readme.txt")),
                has_thumbnail: false,
            },
        }]
    );
    assert_eq!(catalog_warns(&events), Vec::<&str>::new());
    assert_eq!(companion_balloon(&ghost).as_deref(), Some("emo2-kakukaku"));
    assert!(is_ghost_dir(&ghost));
    assert!(!is_ghost_dir(&kaku));
}

/// descript が I/O 失敗で読めない項目は warn! 1 件＋除外し、残りの列挙を続ける（要件 2.7）。
#[test]
fn unreadable_descript_warns_once_and_is_excluded() {
    let tmp = TempPath::new("catalog-unreadable");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_ghost(&root, "ok", "charset,UTF-8\n");
    let locked = put_ghost(&root, "locked", "charset,UTF-8\n");
    let _held = hold_exclusive(&locked.join("ghost").join("master").join("descript.txt"));

    let mut listed = Vec::new();
    let events = capture(|| listed = list_ghosts(&root));
    let folders: Vec<&str> = listed.iter().map(|g| g.identity.folder.as_str()).collect();
    assert_eq!(folders, ["ok"]);
    assert_eq!(catalog_warns(&events), ["catalog_descript_unreadable"]);
}

/// install.txt が I/O 失敗で読めない → warn!＋同梱は無し。
#[test]
fn unreadable_install_warns_and_yields_none() {
    let tmp = TempPath::new("catalog-install-unreadable");
    let ghost = tmp.path().join("g");
    put(&ghost.join("install.txt"), b"balloon.directory,kaku\n");
    let _held = hold_exclusive(&ghost.join("install.txt"));

    let mut companion = Some(String::new());
    let events = capture(|| companion = companion_balloon(&ghost));
    assert_eq!(companion, None);
    assert_eq!(catalog_warns(&events), ["catalog_install_unreadable"]);
}

/// 非 UTF-8 のフォルダ名（対の無いサロゲートを含む UTF-16 名）は warn!＋除外（R6）。
#[test]
fn non_utf8_folder_name_warns_and_is_excluded() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let tmp = TempPath::new("catalog-non-utf8");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_ghost(&root, "ok", "charset,UTF-8\n");
    let bad = root
        .ghost_store()
        .join(OsString::from_wide(&[0xD800, u16::from(b'x')]));
    put(
        &bad.join("ghost").join("master").join("descript.txt"),
        b"charset,UTF-8\n",
    );
    assert!(
        is_ghost_dir(&bad),
        "前提: 名前以外はゴーストとして揃っている"
    );

    let mut listed = Vec::new();
    let events = capture(|| listed = list_ghosts(&root));
    let folders: Vec<&str> = listed.iter().map(|g| g.identity.folder.as_str()).collect();
    assert_eq!(folders, ["ok"]);
    assert_eq!(catalog_warns(&events), ["catalog_non_utf8_name"]);
}
