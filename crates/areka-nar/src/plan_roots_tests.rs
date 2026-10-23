//! `plan` の兄弟テストの続き——宛先の根を配置ごとに 1 回だけ尋ね、答えをその配置に
//! 固定すること（要件 5.4・5.5）。
//!
//! 助手は [`super`]（`plan_tests.rs`）から借りる。根は配置ごとに別の `WorkDir` を答える
//! 口を使い、尋ねられた順と回数を記録して突き合わせる。

use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// フォルダ名ごとに決めた根を答え、尋ねられた問いを順に記録する口。
struct Recorded<'a> {
    ghost: HashMap<&'a str, &'a Path>,
    balloon: HashMap<&'a str, &'a Path>,
    asked: RefCell<Vec<String>>,
}

impl InstallRoots for Recorded<'_> {
    fn ghost_root(&self, folder: &str) -> PathBuf {
        self.asked.borrow_mut().push(format!("ghost:{folder}"));
        self.ghost[folder].to_path_buf()
    }

    fn balloon_root(&self, folder: &str) -> PathBuf {
        self.asked.borrow_mut().push(format!("balloon:{folder}"));
        self.balloon[folder].to_path_buf()
    }
}

/// ゴースト本体 `test-ghost` に同梱のバルーンを 2 つ持つ書庫。
fn two_companions_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
                "balloon.directory,first-balloon",
                "balloon.source.directory,bal1",
                "balloon0.directory,second-balloon",
                "balloon0.source.directory,bal2",
            ]),
        )
        .done()
        .file("ghost/master/descript.txt", b"ghost descript")
        .done()
        .file("bal1/descript.txt", b"balloon 1")
        .done()
        .file("bal2/descript.txt", b"balloon 2")
        .done()
}

/// 同梱のバルーンが複数あっても、同梱はゴーストの中へ入り、根は本体の根のまま。
/// バルーンの根は 1 度も尋ねない（要件 6.4・7.1・7.3・8.2）。尋ねるのは本体の 1 回だけ。
#[test]
fn companions_go_inside_the_ghost_under_the_body_root_without_asking_balloon_roots() {
    let ghost_root = WorkDir::new().expect("根を取れる");
    let roots = Recorded {
        ghost: HashMap::from([("test-ghost", ghost_root.path())]),
        // 答える根を持たない。尋ねられたら引けずに落ちる。
        balloon: HashMap::new(),
        asked: RefCell::default(),
    };
    let request = InstallRequest {
        roots: &roots,
        target_ghost: None,
    };

    let plan = plan_of(two_companions_archive(), &request);

    let ghost = ghost_root.path().join("ghost").join("test-ghost");
    let placed: Vec<(&str, &Path, PathBuf, bool)> = plan
        .iter()
        .map(|p| {
            (
                p.name.as_str(),
                p.root.as_path(),
                p.destination.clone(),
                p.within_body,
            )
        })
        .collect();
    assert_eq!(
        placed,
        vec![
            ("emo", ghost_root.path(), ghost.clone(), false),
            (
                "first-balloon",
                ghost_root.path(),
                ghost.join("balloon").join("first-balloon"),
                true,
            ),
            (
                "second-balloon",
                ghost_root.path(),
                ghost.join("balloon").join("second-balloon"),
                true,
            ),
        ]
    );
    assert_eq!(
        plan[0].carve_out,
        vec![r"balloon\first-balloon", r"balloon\second-balloon"],
        "置く同梱ごとに本体の下敷きから外す"
    );
    assert_eq!(
        *roots.asked.borrow(),
        vec!["ghost:test-ghost"],
        "同梱のために balloon_root を尋ねない"
    );
}

/// シェルの書庫の同梱は、宛先のゴーストの中の `balloon\<dir>` へ入り、根はそのゴーストの根
/// （要件 7.8）。シェルは入れ子にならないので外す場所は無く、同梱は今の確定のまま。
#[test]
fn a_shell_companion_goes_inside_the_target_ghost() {
    let holder = root_with_ghost("test-ghost");
    let roots = Recorded {
        ghost: HashMap::from([("test-ghost", holder.path())]),
        balloon: HashMap::new(),
        asked: RefCell::default(),
    };
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,shell",
                "name,かくかく",
                "directory,test-shell",
                "balloon.directory,kaku",
                "balloon.source.directory,bal1",
            ]),
        )
        .done()
        .file("surface0.png", b"png")
        .done()
        .file("bal1/descript.txt", b"balloon")
        .done();

    let plan = plan_of(
        archive,
        &InstallRequest {
            roots: &roots,
            target_ghost: Some("test-ghost"),
        },
    );

    let ghost = holder.path().join("ghost").join("test-ghost");
    assert_eq!(plan.len(), 2);
    assert_eq!(plan[0].destination, ghost.join("shell").join("test-shell"));
    assert!(plan[0].carve_out.is_empty(), "シェルは外す場所を持たない");
    assert_eq!(plan[1].destination, ghost.join("balloon").join("kaku"));
    assert_eq!(plan[1].root, holder.path(), "根は宛先のゴーストの根");
    assert!(!plan[1].within_body, "シェルの宛先の下ではない");
    assert_eq!(*roots.asked.borrow(), vec!["ghost:test-ghost"]);
}

/// バルーンだけの書庫は今どおりバルーンの根を尋ねる（要件 7.4）。
#[test]
fn a_balloon_only_archive_still_asks_the_balloon_root() {
    let free = WorkDir::new().expect("根を取れる");
    let roots = Recorded {
        ghost: HashMap::new(),
        balloon: HashMap::from([("test-balloon", free.path())]),
        asked: RefCell::default(),
    };
    let plan = plan_of(
        balloon_archive(),
        &InstallRequest {
            roots: &roots,
            target_ghost: None,
        },
    );

    assert_eq!(
        plan[0].destination,
        free.path().join("balloon").join("test-balloon")
    );
    assert_eq!(*roots.asked.borrow(), vec!["balloon:test-balloon"]);
}

// ---- 名指しされた同梱と書庫の `balloon\<名>\`（要件 7.5） ----

/// ゴースト本体 `test-ghost` に `companion_lines` を足し、`extra` のファイルを並べた書庫。
fn ghost_with(companion_lines: &[&str], extra: &[&str]) -> NarBuilder {
    let mut lines = vec![
        "charset,UTF-8",
        "type,ghost",
        "name,emo",
        "directory,test-ghost",
    ];
    lines.extend_from_slice(companion_lines);
    let mut builder = NarBuilder::new()
        .file("install.txt", &install_txt(&lines))
        .done()
        .file("ghost/master/descript.txt", b"ghost")
        .done();
    for &path in extra {
        builder = builder.file(path, b"x").done();
    }
    builder
}

/// 書庫の `balloon/kaku/` は名指しされた同梱 `kaku` に譲って本体から除き、警告 1 件。
/// 名指しされない `balloon/other/` は本体に残る。
#[test]
fn an_archive_folder_at_a_companion_home_is_dropped_from_the_body_with_one_warning() {
    let root = root_with_ghost("other");
    let (plan, _skipped, overridden) = accepted(
        ghost_with(
            &["balloon.directory,kaku", "balloon.source.directory,bal1"],
            &[
                "balloon/Kaku/old.txt",
                "balloon/Kaku/sub/old.png",
                "balloon/other/descript.txt",
                "bal1/descript.txt",
            ],
        ),
        &InstallRequest {
            roots: &SingleRoot(root.path()),
            target_ghost: None,
        },
    );

    assert_eq!(
        relative_files(&plan[0]),
        vec![
            "install.txt",
            "ghost/master/descript.txt",
            "balloon/other/descript.txt",
        ],
        "同梱の宛先と同じ名前の下だけを除く（大小は問わない）"
    );
    assert_eq!(
        plan[0].dirs,
        vec!["balloon", "balloon/other", "ghost", "ghost/master"]
    );
    assert_eq!(plan[0].carve_out, vec![r"balloon\kaku"]);
    assert_eq!(overridden, vec!["kaku"], "警告は同梱 1 つにつき 1 件");
    assert_eq!(relative_files(&plan[1]), vec!["descript.txt"]);
}

/// 取り出し元がその `balloon/<dir>/` 自身なら、項目は今の決まりで同梱へ渡るので警告しない。
#[test]
fn a_companion_taken_from_its_own_home_folder_is_not_warned() {
    let root = root_with_ghost("other");
    let (plan, _skipped, overridden) = accepted(
        ghost_with(
            &[
                "balloon.directory,kaku",
                "balloon.source.directory,balloon\\kaku",
            ],
            &["balloon/kaku/descript.txt", "balloon/other/descript.txt"],
        ),
        &InstallRequest {
            roots: &SingleRoot(root.path()),
            target_ghost: None,
        },
    );

    assert!(overridden.is_empty(), "警告は 0 件: {overridden:?}");
    assert_eq!(
        relative_files(&plan[0]),
        vec![
            "install.txt",
            "ghost/master/descript.txt",
            "balloon/other/descript.txt",
        ]
    );
    assert_eq!(relative_files(&plan[1]), vec!["descript.txt"]);
    assert_eq!(
        plan[1].destination,
        root.path()
            .join("ghost")
            .join("test-ghost")
            .join("balloon")
            .join("kaku")
    );
}

/// 飛ばした同梱の名前の `balloon/<dir>/` は本体に残り、外す場所にも警告にも入らない。
#[test]
fn a_skipped_companion_leaves_the_archive_folder_in_the_body() {
    let root = root_with_ghost("other");
    let (plan, skipped, overridden) = accepted(
        ghost_with(
            &["balloon.directory,kaku", "balloon.source.directory,nowhere"],
            &["balloon/kaku/descript.txt"],
        ),
        &InstallRequest {
            roots: &SingleRoot(root.path()),
            target_ghost: None,
        },
    );

    assert_eq!(plan.len(), 1);
    assert_eq!(skipped.len(), 1);
    assert!(overridden.is_empty());
    assert!(plan[0].carve_out.is_empty());
    assert_eq!(
        relative_files(&plan[0]),
        vec![
            "install.txt",
            "ghost/master/descript.txt",
            "balloon/kaku/descript.txt",
        ]
    );
}

/// シェル・サプリメントは宛先ゴーストの根を尋ね、在否もその答えの根の下で見る（要件 5.4）。
#[test]
fn the_target_ghost_is_looked_up_under_the_answered_root() {
    let holder = root_with_ghost("test-ghost");
    let empty = WorkDir::new().expect("根を取れる");
    for archive in [shell_archive, supplement_archive] {
        // 宛先ゴーストの在る根を答えれば受理され、宛先はその根の下。
        let found = Recorded {
            ghost: HashMap::from([("test-ghost", holder.path())]),
            balloon: HashMap::new(),
            asked: RefCell::default(),
        };
        let plan = plan_of(
            archive(),
            &InstallRequest {
                roots: &found,
                target_ghost: Some("test-ghost"),
            },
        );
        assert_eq!(plan[0].root, holder.path());
        assert!(
            plan[0]
                .destination
                .starts_with(holder.path().join("ghost").join("test-ghost"))
        );
        assert_eq!(*found.asked.borrow(), vec!["ghost:test-ghost"]);

        // 宛先ゴーストの無い根を答えれば、別の根に在っても拒否する。
        let absent = Recorded {
            ghost: HashMap::from([("test-ghost", empty.path())]),
            balloon: HashMap::new(),
            asked: RefCell::default(),
        };
        assert_eq!(
            refusal_of(
                archive(),
                &InstallRequest {
                    roots: &absent,
                    target_ghost: Some("test-ghost"),
                },
            ),
            RefuseReason::TargetGhostMissing {
                target: Some("test-ghost".to_owned()),
            }
        );
    }
}
