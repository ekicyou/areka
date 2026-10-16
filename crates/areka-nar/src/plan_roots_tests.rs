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

/// 同梱のバルーンが複数あるとき、配置ごとに違う根が答えられ、その根の下に宛先が組まれる。
/// 尋ねるのは配置ごとに 1 回だけ。
#[test]
fn each_placement_gets_its_own_answered_root() {
    let ghost_root = WorkDir::new().expect("根を取れる");
    let first_root = WorkDir::new().expect("根を取れる");
    let second_root = WorkDir::new().expect("根を取れる");
    let roots = Recorded {
        ghost: HashMap::from([("test-ghost", ghost_root.path())]),
        balloon: HashMap::from([
            ("first-balloon", first_root.path()),
            ("second-balloon", second_root.path()),
        ]),
        asked: RefCell::default(),
    };
    let request = InstallRequest {
        roots: &roots,
        target_ghost: None,
    };

    let plan = plan_of(two_companions_archive(), &request);

    let placed: Vec<(&str, &Path, PathBuf)> = plan
        .iter()
        .map(|p| (p.name.as_str(), p.root.as_path(), p.destination.clone()))
        .collect();
    assert_eq!(
        placed,
        vec![
            (
                "emo",
                ghost_root.path(),
                ghost_root.path().join("ghost").join("test-ghost"),
            ),
            (
                "first-balloon",
                first_root.path(),
                first_root.path().join("balloon").join("first-balloon"),
            ),
            (
                "second-balloon",
                second_root.path(),
                second_root.path().join("balloon").join("second-balloon"),
            ),
        ]
    );
    assert_eq!(
        *roots.asked.borrow(),
        vec![
            "ghost:test-ghost",
            "balloon:first-balloon",
            "balloon:second-balloon",
        ],
        "配置ごとに 1 回だけ尋ねる"
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
