//! `list_all_shells` の兄弟テスト（shell-balloon-switch 要件 1.6・8.4）。
//!
//! 隠しシェル（`menu,hidden`）は名指しの切替の候補に入る（要件 1.6）が、今の列挙
//! `list_shells` は除いたまま（`updateother` が除外に依る）。素性の型に欄は足さない（要件 8.4）。

use super::test_support::put;
use super::*;
use std::fs;
use temp_path_kit::TempPath;

#[test]
fn all_shells_include_menu_hidden_while_list_shells_excludes_it() {
    let tmp = TempPath::new("catalog-all-shells");
    let ghost = tmp.path().join("g");
    let shell = ghost.join("shell");
    put(
        &shell.join("master").join("descript.txt"),
        b"charset,UTF-8\nname,Master\n",
    );
    put(
        &shell.join("secret").join("descript.txt"),
        b"charset,UTF-8\nname,Secret\nMENU, Hidden \n",
    );
    fs::create_dir_all(shell.join("empty")).unwrap();

    let all = list_all_shells(&ghost);
    assert_eq!(
        all,
        [
            ShellEntry {
                dir: shell.join("master"),
                identity: Identity {
                    folder: "master".into(),
                    name: Some("Master".into()),
                    ..Identity::default()
                },
            },
            ShellEntry {
                dir: shell.join("secret"),
                identity: Identity {
                    folder: "secret".into(),
                    name: Some("Secret".into()),
                    ..Identity::default()
                },
            },
        ]
    );

    let folders: Vec<String> = list_shells(&ghost)
        .into_iter()
        .map(|s| s.identity.folder)
        .collect();
    assert_eq!(folders, ["master"]);
}
