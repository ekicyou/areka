//! `catalog` の核となる採否と素性の決定論テスト（要件 2.1〜2.8・5.3）。
//!
//! 根は `temp-path-kit` の一時フォルダへその場で組むか（要件 8.1 の構成は
//! `catalog_test_support::temp_root`）、検体の根（`SampleRoot::acquire("emo2")`）を借りる。
//! 足すテストは列挙の採否と素性の判断分岐に限る（要件 8.4）。

use super::test_support::{
    TARGET, TempRoot, catalog_warns, hold_exclusive, put, put_ghost_in, sjis_descript, temp_root,
};
use super::*;
use crate::test_log_capture::{assert_logged_event, capture};
use std::fs;
use temp_path_kit::TempPath;
use tracing::Level;

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
    assert!(scan_ghosts(&root).is_empty());
    assert!(scan_balloons(&root).is_empty());
    assert!(list_shells(&root.ghost_dir("nobody")).is_empty());
}

#[test]
fn ghosts_are_folders_with_master_descript_in_byte_order() {
    let tmp = TempPath::new("catalog-ghosts");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_ghost_in(&root.ghost_store(), "b", "charset,UTF-8\nname,Bee\n");
    // Windows の fs は大小を区別しないので、大文字は別の綴り（`Z`）で置く。
    put_ghost_in(&root.ghost_store(), "Z", "charset,UTF-8\nName,Upper\n");
    put_ghost_in(&root.ghost_store(), "a", "charset,UTF-8\n");
    // descript の無いフォルダとフォルダ以外の項目は返さない（要件 2.1）。
    fs::create_dir_all(root.ghost_dir("no-descript").join("ghost").join("master")).unwrap();
    put(&root.ghost_store().join("stray.txt"), b"x");

    let listed = scan_ghosts(&root);
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
    let mut sjis = sjis_descript();
    sjis.extend_from_slice(b"readme,manual.txt\n");
    let dir = root.balloon_dir("sjis");
    put(&dir.join("descript.txt"), &sjis);
    put(&dir.join("manual.txt"), b"m");
    put(&dir.join("thumbnail.png"), b"p");

    let listed = scan_balloons(&root);
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].identity,
        Identity {
            folder: "sjis".into(),
            name: Some("吹き出し".into()),
            craftman: Some("stayse".into()),
            craftmanw: Some("作者".into()),
            id: Some("StayseeBalloon".into()),
            readme: Some(dir.join("manual.txt")),
            has_thumbnail: true,
        }
    );
}

#[test]
fn readme_falls_back_to_readme_txt_only_when_present() {
    let tmp = TempPath::new("catalog-readme");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let with = put_ghost_in(&root.ghost_store(), "with", "charset,UTF-8\n");
    put(&with.join("readme.txt"), b"r");
    // readme 鍵が実在しないファイルを指す → 無し（readme.txt へは倒さない）。
    let dangling = put_ghost_in(
        &root.ghost_store(),
        "dangling",
        "charset,UTF-8\nreadme,gone.txt\n",
    );
    put(&dangling.join("readme.txt"), b"r");

    let listed = scan_ghosts(&root);
    assert_eq!(listed[0].identity.folder, "dangling");
    assert_eq!(listed[0].identity.readme, None);
    assert_eq!(listed[1].identity.readme, Some(with.join("readme.txt")));
    assert!(!listed[1].identity.has_thumbnail);
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
    let events = capture(|| listed = scan_balloons(&root));
    let folders: Vec<&str> = listed.iter().map(|b| b.identity.folder.as_str()).collect();
    assert_eq!(folders, ["plain", "typed"]);
    assert_logged_event(&events, Level::WARN, TARGET, "catalog_type_not_balloon");
}

/// 無印と `balloon0` の両方 → 無印が最初の 1 個（`balloon0` は読まれない）・install.txt が無い → 無し。
/// 番号付きだけの場面は `catalog_standard_balloon_tests.rs`。
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
    let ghost = put_ghost_in(
        &root.ghost_store(),
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

    let ghosts = scan_ghosts(&root);
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
    let events = capture(|| balloons = scan_balloons(&root));
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
    let yes = put_ghost_in(&root.ghost_store(), "yes", "");
    assert!(is_ghost_dir(&yes));
    assert!(!is_ghost_dir(&root.ghost_store()));
    assert!(!is_ghost_dir(&tmp.path().join("nothing")));
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
        ghosts = scan_ghosts(&root);
        balloons = scan_balloons(&root);
        shells = list_shells(&alpha);
    });

    // ゴースト 2 体（descript の無い `empty`・フォルダ以外の `stray.txt` は返さない＝要件 2.1）。
    assert_eq!(
        ghosts,
        [
            GhostEntry {
                root: root.dir().to_path_buf(),
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
                root: root.dir().to_path_buf(),
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
                root: root.dir().to_path_buf(),
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
                root: root.dir().to_path_buf(),
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

/// 検体の根（emo2＝ゴースト 1・バルーン 1・同梱の install.txt）。同梱バルーンはゴーストの中の
/// `balloon\` に入るので根の `balloon\` は空（areka-P0-ghost-inner-balloon 要件 7.1）。期待値は
/// `vendors/sample_ghost/emo2.nar` の実物から取った（ゴーストの descript の `id.emo2` は
/// カンマが無いので `id` は無し）。
#[test]
fn sample_root_emo2_lists_one_ghost_one_balloon() {
    let emo2 = sample_ghost_kit::SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体");
    let root = BasewareRoot::new(emo2.root().to_path_buf());
    let ghost = emo2.folder().to_path_buf();
    let kaku = emo2.balloon("emo2-kakukaku").expect("同梱").to_path_buf();

    let mut ghosts = Vec::new();
    let mut root_balloons = Vec::new();
    let mut balloons = Vec::new();
    let mut shells = Vec::new();
    let events = capture(|| {
        ghosts = scan_ghosts(&root);
        root_balloons = scan_balloons(&root);
        balloons = scan_ghost_balloons(&ghost);
        shells = list_shells(&ghost);
    });

    assert_eq!(
        ghosts,
        [GhostEntry {
            root: emo2.root().to_path_buf(),
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
    assert_eq!(root_balloons, []);
    assert_eq!(
        balloons,
        [BalloonEntry {
            root: ghost.clone(),
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
    put_ghost_in(&root.ghost_store(), "ok", "charset,UTF-8\n");
    let locked = put_ghost_in(&root.ghost_store(), "locked", "charset,UTF-8\n");
    let _held = hold_exclusive(&locked.join("ghost").join("master").join("descript.txt"));

    let mut listed = Vec::new();
    let events = capture(|| listed = scan_ghosts(&root));
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
    put_ghost_in(&root.ghost_store(), "ok", "charset,UTF-8\n");
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
    let events = capture(|| listed = scan_ghosts(&root));
    let folders: Vec<&str> = listed.iter().map(|g| g.identity.folder.as_str()).collect();
    assert_eq!(folders, ["ok"]);
    assert_eq!(catalog_warns(&events), ["catalog_non_utf8_name"]);
}

/// `sakura.name` の単独の読み手（要件 2.1・8.7）: 在る → 値・鍵が無い／descript が無い → 黙って None。
#[test]
fn sakura_name_reads_one_key_from_master_descript() {
    let tmp = TempPath::new("catalog-sakura-name");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let with = put_ghost_in(
        &root.ghost_store(),
        "with",
        "charset,UTF-8\nSakura.Name,さくら\nname,With\n",
    );
    let without = put_ghost_in(
        &root.ghost_store(),
        "without",
        "charset,UTF-8\nname,Without\n",
    );

    let mut got = (None, None, None);
    let events = capture(|| {
        got = (
            sakura_name(&with),
            sakura_name(&without),
            sakura_name(&root.ghost_dir("none")),
        )
    });
    assert_eq!(got, (Some("さくら".to_owned()), None, None));
    assert_eq!(catalog_warns(&events), Vec::<&str>::new());
}

/// descript が I/O 失敗で読めない → warn!＋本体側の名前は無し。
#[test]
fn sakura_name_unreadable_warns_and_yields_none() {
    let tmp = TempPath::new("catalog-sakura-name-unreadable");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let ghost = put_ghost_in(
        &root.ghost_store(),
        "locked",
        "charset,UTF-8\nsakura.name,さくら\n",
    );
    let _held = hold_exclusive(&ghost.join("ghost").join("master").join("descript.txt"));

    let mut got = Some(String::new());
    let events = capture(|| got = sakura_name(&ghost));
    assert_eq!(got, None);
    assert_eq!(catalog_warns(&events), ["catalog_descript_unreadable"]);
}

/// `install.accept` の読み手（要件 3.1）: カンマで分けて前後の空白を落とし、空の名前は落とす。
/// 鍵は大文字小文字を問わない。鍵が無い／descript が無い → 黙って空の列。
#[test]
fn install_accept_splits_trims_and_drops_empty_names() {
    let tmp = TempPath::new("catalog-install-accept");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let with = put_ghost_in(
        &root.ghost_store(),
        "with",
        "charset,UTF-8\nInstall.Accept, さくら ,,Emily ,  ,\t毒子\nsakura.name,本体\n",
    );
    let one = put_ghost_in(
        &root.ghost_store(),
        "one",
        "charset,UTF-8\ninstall.accept,ひとり\n",
    );
    let without = put_ghost_in(
        &root.ghost_store(),
        "without",
        "charset,UTF-8\nsakura.name,本体\n",
    );

    let mut got = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let events = capture(|| {
        got = (
            install_accept(&with),
            install_accept(&one),
            install_accept(&without),
            install_accept(&root.ghost_dir("none")),
        )
    });
    assert_eq!(got.0, ["さくら", "Emily", "毒子"]);
    assert_eq!(got.1, ["ひとり"]);
    assert!(got.2.is_empty() && got.3.is_empty(), "{got:?}");
    assert_eq!(catalog_warns(&events), Vec::<&str>::new());
}

/// descript が I/O 失敗で読めない → warn! 1 件＋空の列。
#[test]
fn install_accept_unreadable_warns_and_yields_empty() {
    let tmp = TempPath::new("catalog-install-accept-unreadable");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let ghost = put_ghost_in(
        &root.ghost_store(),
        "locked",
        "charset,UTF-8\ninstall.accept,さくら\n",
    );
    let _held = hold_exclusive(&ghost.join("ghost").join("master").join("descript.txt"));

    let mut got = vec![String::new()];
    let events = capture(|| got = install_accept(&ghost));
    assert!(got.is_empty(), "{got:?}");
    assert_eq!(catalog_warns(&events), ["catalog_descript_unreadable"]);
}

/// `homeurl`・`name` の単独の読み手（要件 1.3・1.9・2.4）: 渡したフォルダの descript.txt から読む。
/// 鍵は大文字小文字を問わず、値は前後の空白を落として無変形。空の値・空白だけの値・鍵が無い・
/// descript が無い → 黙って None。
#[test]
fn homeurl_and_descript_name_read_one_key_each() {
    let tmp = TempPath::new("catalog-homeurl");
    let dir = |name: &str| tmp.path().join(name);
    let with = dir("with");
    put(
        &with.join("descript.txt"),
        "charset,UTF-8\nHomeURL,  https://Example.test/Ghost/  \nName, あるふぁ \n".as_bytes(),
    );
    let empty = dir("empty");
    put(
        &empty.join("descript.txt"),
        b"charset,UTF-8\nhomeurl,\nname,\n",
    );
    let blank = dir("blank");
    put(
        &blank.join("descript.txt"),
        b"charset,UTF-8\nhomeurl,   \nname,\t\n",
    );
    let without = dir("without");
    put(
        &without.join("descript.txt"),
        b"charset,UTF-8\ncraftman,x\n",
    );

    let mut got = Vec::new();
    let events = capture(|| {
        got = [&with, &empty, &blank, &without, &dir("none")]
            .into_iter()
            .map(|d| (homeurl(d), descript_name(d)))
            .collect()
    });
    assert_eq!(
        got,
        [
            (
                Some("https://Example.test/Ghost/".to_owned()),
                Some("あるふぁ".to_owned())
            ),
            (None, None),
            (None, None),
            (None, None),
            (None, None),
        ]
    );
    assert_eq!(catalog_warns(&events), Vec::<&str>::new());
}

/// ゴーストは `ghost/master`、バルーンはフォルダを渡す。`Shift_JIS` 宣言の descript も復号して読む。
#[test]
fn homeurl_and_descript_name_follow_the_given_folder_and_decode_shift_jis() {
    let TempRoot { _tmp, root } = temp_root("catalog-homeurl-layout");
    let master = root.ghost_dir("alpha").join("ghost").join("master");
    let staysee = root.balloon_dir("StayseeBalloon");

    let mut got = (None, None, None, None);
    let events = capture(|| {
        got = (
            descript_name(&master),
            descript_name(&staysee),
            homeurl(&staysee),
            descript_name(&root.ghost_dir("alpha")),
        )
    });
    assert_eq!(
        got,
        (
            Some("Alpha".to_owned()),
            Some("吹き出し".to_owned()),
            None,
            None
        )
    );
    assert_eq!(catalog_warns(&events), Vec::<&str>::new());
}

/// descript が I/O 失敗で読めない → 口ごとに warn! 1 件＋None。
#[test]
fn homeurl_and_descript_name_unreadable_warn_and_yield_none() {
    let tmp = TempPath::new("catalog-homeurl-unreadable");
    let path = tmp.path().join("locked").join("descript.txt");
    put(
        &path,
        b"charset,UTF-8\nhomeurl,https://example.test/\nname,Locked\n",
    );
    let _held = hold_exclusive(&path);
    let dir = path.parent().expect("親");

    let mut got = (Some(String::new()), Some(String::new()));
    let events = capture(|| got = (homeurl(dir), descript_name(dir)));
    assert_eq!(got, (None, None));
    assert_eq!(
        catalog_warns(&events),
        ["catalog_descript_unreadable", "catalog_descript_unreadable"]
    );
}

/// readme がフォルダの外・下のフォルダを指す → 無し（最上位のファイルだけを受ける・#24）。
#[test]
fn readme_outside_the_top_folder_is_absent() {
    let tmp = TempPath::new("catalog-readme-outside");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    put_ghost_in(&root.ghost_store(), "up", r"readme,..\outside.txt");
    put(&root.ghost_store().join("outside.txt"), b"o");
    let sub = put_ghost_in(&root.ghost_store(), "sub", r"readme,sub\x.txt");
    put(&sub.join("sub").join("x.txt"), b"x");

    let listed = scan_ghosts(&root);
    assert_eq!(listed.len(), 2);
    assert!(
        listed.iter().all(|g| g.identity.readme.is_none()),
        "{listed:?}"
    );
}

/// descript.txt がフォルダ → 一覧の読み手も単独の鍵の読み手も warn! 1 件ずつ（#23）。
#[test]
fn a_descript_that_is_a_folder_warns_in_both_readers() {
    let tmp = TempPath::new("catalog-descript-folder");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let ghost = root.ghost_store().join("g");
    fs::create_dir_all(ghost.join("ghost").join("master").join("descript.txt")).unwrap();

    let mut got = (Vec::new(), Some(String::new()));
    let events = capture(|| got = (scan_ghosts(&root), sakura_name(&ghost)));
    assert_eq!(got, (Vec::new(), None));
    assert_eq!(
        catalog_warns(&events),
        ["catalog_descript_unreadable", "catalog_descript_unreadable"]
    );
}
