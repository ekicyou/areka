//! `plan` の兄弟テスト——4 種の宛先・同梱バルーンの接頭辞剥がしと複製の不在・
//! 拒否 3 種・フォルダの作り分け・根の外へ出ないことの全数勘定（要件 4.8・5.1〜5.8・6.3）。
//!
//! 期待値の置き方は 3 つ。⑴ 受理の側は組み上がった [`Placement`] の**全ての欄**を
//! 突き合わせる（「置かれた」ではなく「どこに何が置かれるか」を見る）。⑵ 拒否の側は
//! 理由の変種と中身（どのキーか・どの宛先か）まで見る。⑶ 根の外へ出ないことは
//! 1 本の道筋を見るのでは足りないので、全ての固定入力の全ての宛先・全てのファイル・
//! 全てのフォルダを歩いて**数える**。数を固定しておけば、計画が空になって恒真で
//! 緑になる道が塞がる。
//!
//! 固定入力は手で [`EntryName`] や [`InstallManifest`] を作らず、本物の zip を
//! 往復させて組む。`components` や `is_dir` の取り違えが緑のまま残らない。

use super::*;
use crate::container::{inflate_entry, read_central_directory};
use crate::manifest::{locate_install_txt, parse_manifest};
use crate::names::validate_entry_names;
use sample_ghost_kit::{NarBuilder, WorkDir, install_txt};
use std::path::Component;

// ---- 助手 ----

/// 書庫を読取 → 名前の検証 → マニフェストの解釈まで通してから計画を組む。
fn planned(
    builder: NarBuilder,
    request: &InstallRequest<'_>,
) -> Result<Vec<Placement>, RefuseReason> {
    let bytes = builder.bytes();
    let raw = read_central_directory(&bytes).expect("書庫そのものは無傷");
    let names = validate_entry_names(&raw).expect("名前は安全");
    let index = locate_install_txt(&names)
        .expect("最上位に install.txt がある")
        .index;
    let text = inflate_entry(&bytes, &raw[index]).expect("install.txt を伸長できる");
    let manifest = parse_manifest(&text).expect("マニフェストは受理される");
    build_plan(&manifest, &names, request)
}

/// 受理された計画だけを取り出す。拒否されたら主張ごと落とす。
fn plan_of(builder: NarBuilder, request: &InstallRequest<'_>) -> Vec<Placement> {
    match planned(builder, request) {
        Ok(plan) => plan,
        Err(reason) => panic!("受理されるはずの書庫が拒否された: {reason}"),
    }
}

/// 拒否だけを取り出す。受理されたら主張ごと落とす。
fn refusal_of(builder: NarBuilder, request: &InstallRequest<'_>) -> RefuseReason {
    match planned(builder, request) {
        Ok(plan) => panic!(
            "拒否されるはずの書庫が受理された: {:?}",
            plan.iter().map(|p| &p.destination).collect::<Vec<_>>()
        ),
        Err(reason) => reason,
    }
}

/// 宛先ゴースト `name` が実在する空の根を 1 つ借りる。
///
/// OS の一時フォルダは使わない（要件 7.10）。`WorkDir` が破棄で木ごと片付ける。
fn root_with_ghost(name: &str) -> WorkDir {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    std::fs::create_dir_all(work.path().join("ghost").join(name)).expect("宛先ゴーストを作れる");
    work
}

/// ファイルの相対パスだけを並べる（エントリ番号を見ないときの比較用）。
fn relative_files(placement: &Placement) -> Vec<&str> {
    placement
        .files
        .iter()
        .map(|(_, relative)| relative.as_str())
        .collect()
}

// ---- 固定入力 ----

/// 同梱バルーンつきのゴースト（検体 emo2 と同じ形）。
///
/// フォルダ名は登記表の検体とわざと違う綴りにする。実在の検体名でパスを組むと、
/// 検体の在処を自分で綴っていないかを見張る常設検査（段 ①）が当たる。綴りそのものは
/// `manifest` の兄弟テストが実物の `install.txt` で見ているので、こちらは形だけ借りる。
///
/// `ghost/`・`ghost/master/` はフォルダのエントリを持ち、`shell/master/` は持たない。
/// 同梱バルーンの側も `test-balloon/balloon/` のエントリを持たない。4.9 の 2 つの形が
/// 1 つの書庫に同居する。
fn ghost_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,えも？？",
                "directory,test-ghost",
                "balloon.directory,test-balloon",
                "balloon.source.directory,test-balloon",
            ]),
        )
        .done()
        .file("readme.txt", b"readme")
        .done()
        .dir("ghost")
        .dir("ghost/master")
        .file("ghost/master/descript.txt", b"ghost descript")
        .done()
        .file("shell/master/surface0.png", b"png")
        .done()
        .dir("test-balloon")
        .file("test-balloon/install.txt", b"type,balloon")
        .done()
        .file("test-balloon/descript.txt", b"balloon descript")
        .done()
        .file("test-balloon/balloon/s0.png", b"s0")
        .done()
}

/// 単独のバルーン。
fn balloon_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,balloon",
                "name,test balloon",
                "directory,test-balloon",
            ]),
        )
        .done()
        .file("descript.txt", b"balloon descript")
        .done()
        .file("balloon/s0.png", b"s0")
        .done()
}

/// 単独のシェル。
fn shell_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,shell",
                "name,かくかく",
                "directory,test-shell",
            ]),
        )
        .done()
        .file("descript.txt", b"shell descript")
        .done()
        .file("surface0.png", b"png")
        .done()
}

/// サプリメント。最上位と下の階層の両方に `install.txt` を置く。
fn supplement_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,supplement",
                "name,追加辞書",
                "directory,test-extra",
            ]),
        )
        .done()
        .file("ghost/master/dic.txt", b"dic")
        .done()
        .file("sub/install.txt", b"not the manifest")
        .done()
}

/// フォルダのエントリだけがあり中身が無い形（空フォルダ）。
fn empty_folder_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
            ]),
        )
        .done()
        .dir("work")
}

/// フォルダのエントリを 1 つも持たない形（実在の配布物 `hello-pasta.nar` と同じ）。
fn no_folder_entry_archive() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
            ]),
        )
        .done()
        .file("a/b/c.txt", b"c")
        .done()
}

// ---- 4 種の宛先（要件 5.2・5.5・5.6・5.8） ----

/// ゴーストは根のゴースト格納先へ置かれ、同梱バルーンの取り出し元だけが除かれる。
#[test]
fn a_ghost_goes_under_the_root_ghost_folder_without_the_companion_source() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let plan = plan_of(ghost_archive(), &request);

    assert_eq!(plan.len(), 2, "ゴースト本体と同梱バルーンの 2 配置になる");
    let ghost = &plan[0];
    assert_eq!(ghost.kind, ElementKind::Ghost);
    assert_eq!(ghost.name, "えも？？", "表示名は manifest.name");
    assert_eq!(
        ghost.destination,
        root.path().join("ghost").join("test-ghost")
    );
    assert_eq!(ghost.target_ghost, None);
    assert_eq!(ghost.existing, ExistingPolicy::Overlay);
    assert!(!ghost.skip_top_level_install_txt);
    assert_eq!(
        ghost.files,
        vec![
            (0, "install.txt".to_owned()),
            (1, "readme.txt".to_owned()),
            (4, "ghost/master/descript.txt".to_owned()),
            (5, "shell/master/surface0.png".to_owned()),
        ]
    );
    assert_eq!(
        ghost.dirs,
        vec!["ghost", "ghost/master", "shell", "shell/master"]
    );
}

/// 単独のバルーンは根のバルーン格納先へ置かれる。
#[test]
fn a_balloon_goes_under_the_root_balloon_folder() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let plan = plan_of(balloon_archive(), &request);

    assert_eq!(plan.len(), 1);
    let balloon = &plan[0];
    assert_eq!(balloon.kind, ElementKind::Balloon);
    assert_eq!(balloon.name, "test balloon");
    assert_eq!(
        balloon.destination,
        root.path().join("balloon").join("test-balloon")
    );
    assert_eq!(balloon.target_ghost, None);
    assert!(!balloon.skip_top_level_install_txt);
    assert_eq!(
        relative_files(balloon),
        vec!["install.txt", "descript.txt", "balloon/s0.png"]
    );
    assert_eq!(balloon.dirs, vec!["balloon"]);
}

/// シェルは宛先ゴーストの下のシェル格納先へ置かれる。
#[test]
fn a_shell_goes_under_the_target_ghosts_shell_folder() {
    let root = root_with_ghost("test-ghost");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: Some("test-ghost"),
    };
    let plan = plan_of(shell_archive(), &request);

    assert_eq!(plan.len(), 1);
    let shell = &plan[0];
    assert_eq!(shell.kind, ElementKind::Shell);
    assert_eq!(shell.name, "かくかく");
    assert_eq!(
        shell.destination,
        root.path()
            .join("ghost")
            .join("test-ghost")
            .join("shell")
            .join("test-shell")
    );
    assert_eq!(shell.target_ghost, Some("test-ghost".to_owned()));
    assert!(!shell.skip_top_level_install_txt);
    assert_eq!(
        relative_files(shell),
        vec!["install.txt", "descript.txt", "surface0.png"]
    );
    assert!(shell.dirs.is_empty());
}

/// サプリメントは宛先ゴーストへ重ね置きされ、最上位の `install.txt` だけが除かれる。
#[test]
fn a_supplement_overlays_the_target_ghost_without_the_top_level_install_txt() {
    let root = root_with_ghost("test-ghost");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: Some("test-ghost"),
    };
    let plan = plan_of(supplement_archive(), &request);

    assert_eq!(plan.len(), 1);
    let supplement = &plan[0];
    assert_eq!(supplement.kind, ElementKind::Supplement);
    assert_eq!(supplement.name, "追加辞書");
    assert_eq!(
        supplement.destination,
        root.path().join("ghost").join("test-ghost"),
        "宛先はゴースト本体そのもの（manifest.directory は使わない）"
    );
    assert_eq!(supplement.target_ghost, Some("test-ghost".to_owned()));
    assert!(supplement.skip_top_level_install_txt);
    assert_eq!(
        relative_files(supplement),
        vec!["ghost/master/dic.txt", "sub/install.txt"],
        "下の階層の install.txt は普通のファイルとして残る"
    );
    assert_eq!(supplement.dirs, vec!["ghost", "ghost/master", "sub"]);
}

// ---- 同梱バルーン（要件 5.3・5.4・6.3） ----

/// 同梱バルーンは接頭辞を剥がしてバルーン格納先へ置かれる。
#[test]
fn a_companion_balloon_goes_to_the_balloon_folder_with_its_prefix_stripped() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let plan = plan_of(ghost_archive(), &request);

    let companion = &plan[1];
    assert_eq!(companion.kind, ElementKind::Balloon);
    assert_eq!(
        companion.name, "test-balloon",
        "同梱バルーンの名前は directory"
    );
    assert_eq!(
        companion.destination,
        root.path().join("balloon").join("test-balloon")
    );
    assert_eq!(companion.target_ghost, None);
    assert_eq!(companion.existing, ExistingPolicy::Overlay);
    assert!(!companion.skip_top_level_install_txt);
    assert_eq!(
        companion.files,
        vec![
            (7, "install.txt".to_owned()),
            (8, "descript.txt".to_owned()),
            (9, "balloon/s0.png".to_owned()),
        ],
        "取り出し元の 1 階層が剥がれ、中の install.txt は普通のファイルとして残る"
    );
    assert_eq!(companion.dirs, vec!["balloon"]);
}

/// ゴースト側に同梱バルーンの複製が残らない（要件 5.3 の後半）。
///
/// バルーン側へ置かれることだけを見ると、両方へ置く計画も緑になる。
#[test]
fn the_companion_leaves_no_copy_on_the_ghost_side() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let plan = plan_of(ghost_archive(), &request);

    let ghost = &plan[0];
    for relative in relative_files(ghost) {
        assert!(
            !relative.starts_with("test-balloon"),
            "ゴースト側に同梱バルーンの複製が残っている: {relative}"
        );
    }
    for dir in &ghost.dirs {
        assert!(
            !dir.starts_with("test-balloon"),
            "ゴースト側に取り出し元の空フォルダが残っている: {dir}"
        );
    }
    // エントリ番号でも確かめる。相対パスの綴りだけで測ると、剥がした後の
    // 名前がたまたま一致しない形（別名の取り出し元）を見落とす。
    let companion_entries: Vec<usize> = plan[1].files.iter().map(|(index, _)| *index).collect();
    for (index, _) in &ghost.files {
        assert!(
            !companion_entries.contains(index),
            "エントリ {index} がゴースト側と同梱バルーン側の両方に居る"
        );
    }
}

/// 取り出し元の名前が宛先と違っても、剥がれるのは取り出し元のほう。
#[test]
fn the_companion_source_name_may_differ_from_its_destination() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
                "balloon.directory,installed-name",
                "balloon.source.directory,packed-name",
            ]),
        )
        .done()
        .file("packed-name/descript.txt", b"d")
        .done();
    let plan = plan_of(archive, &request);

    assert_eq!(relative_files(&plan[0]), vec!["install.txt"]);
    assert_eq!(
        plan[1].destination,
        root.path().join("balloon").join("installed-name")
    );
    assert_eq!(relative_files(&plan[1]), vec!["descript.txt"]);
}

/// 取り出し元の綴りが `install.txt` と書庫とで大小だけ違っても、同じフォルダを指す。
///
/// Windows のファイルシステムの意味論に合わせる。大小を区別してしまうと、本体側からは
/// 除かれないのに同梱バルーン側へは取り出せず、「複製が残る」と「取り出し元が無い」が
/// 同時に起きる。
#[test]
fn the_companion_source_matches_the_archive_spelling_ignoring_case() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
                "balloon.directory,test-balloon",
                "balloon.source.directory,Test-Balloon",
            ]),
        )
        .done()
        .file("test-balloon/descript.txt", b"d")
        .done();
    let plan = plan_of(archive, &request);

    assert_eq!(
        relative_files(&plan[0]),
        vec!["install.txt"],
        "大小違いの取り出し元が本体側に複製として残っている"
    );
    assert_eq!(
        relative_files(&plan[1]),
        vec!["descript.txt"],
        "大小違いの取り出し元から取り出せていない"
    );
}

/// 同梱バルーンは自分の `*.refresh` を持ち運ぶ（要件 6.3）。
#[test]
fn the_companion_carries_its_own_refresh_policy() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
                "refresh,1",
                "refreshundeletemask,profile.txt:emo.ini",
                "balloon.directory,test-balloon",
                "balloon.refresh,1",
                "balloon.refreshundeletemask,keep.txt",
            ]),
        )
        .done()
        .file("test-balloon/descript.txt", b"d")
        .done();
    let plan = plan_of(archive, &request);

    assert_eq!(
        plan[0].existing,
        ExistingPolicy::Replace {
            keep: vec!["profile.txt".to_owned(), "emo.ini".to_owned()],
        }
    );
    assert_eq!(
        plan[1].existing,
        ExistingPolicy::Replace {
            keep: vec!["keep.txt".to_owned()],
        }
    );
}

/// 取り出し元がフォルダのエントリだけ（中身が 1 件も無い）なら拒否する（要件 5.4）。
#[test]
fn refuses_a_companion_whose_source_folder_is_empty() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
                "balloon.directory,test-balloon",
            ]),
        )
        .done()
        .dir("test-balloon");

    assert_eq!(
        refusal_of(archive, &request),
        RefuseReason::CompanionSourceMissing {
            key: "balloon.source.directory".to_owned(),
            source_directory: "test-balloon".to_owned(),
        }
    );
}

/// 取り出し元の名前が書庫のどこにも無ければ拒否する（要件 5.4）。
#[test]
fn refuses_a_companion_whose_source_folder_is_absent() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let archive = NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,emo",
                "directory,test-ghost",
                "balloon.directory,test-balloon",
                "balloon.source.directory,nowhere",
            ]),
        )
        .done()
        .file("test-balloon/descript.txt", b"d")
        .done();

    assert_eq!(
        refusal_of(archive, &request),
        RefuseReason::CompanionSourceMissing {
            key: "balloon.source.directory".to_owned(),
            source_directory: "nowhere".to_owned(),
        }
    );
}

// ---- 宛先ゴースト（要件 5.7） ----

/// 宛先ゴーストが渡されなければ、シェルもサプリメントも拒否する。
#[test]
fn refuses_a_shell_or_supplement_without_a_target_ghost() {
    let root = root_with_ghost("test-ghost");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    for archive in [shell_archive(), supplement_archive()] {
        assert_eq!(
            refusal_of(archive, &request),
            RefuseReason::TargetGhostMissing { target: None }
        );
    }
}

/// 渡された宛先ゴーストが根に実在しなければ拒否する。
#[test]
fn refuses_a_target_ghost_that_is_not_on_disk() {
    let root = root_with_ghost("test-ghost");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: Some("not-installed"),
    };
    for archive in [shell_archive(), supplement_archive()] {
        assert_eq!(
            refusal_of(archive, &request),
            RefuseReason::TargetGhostMissing {
                target: Some("not-installed".to_owned()),
            }
        );
    }
}

/// 宛先ゴーストが 1 階層のフォルダ名でなければ拒否する。
///
/// `..` は `<根>/ghost/..` として**実在してしまう**ので、実在の検査だけでは
/// 根の外が宛先になる。名前の形を先に見るのはそのため。
#[test]
fn refuses_a_target_ghost_name_that_is_not_a_one_level_name() {
    let root = root_with_ghost("test-ghost");
    for target in ["..", "../..", "a/b", "a\\b", "C:", "", "."] {
        let request = InstallRequest {
            root: root.path(),
            target_ghost: Some(target),
        };
        assert_eq!(
            refusal_of(shell_archive(), &request),
            RefuseReason::TargetGhostMissing {
                target: Some(target.to_owned()),
            },
            "宛先ゴースト {target:?} が拒否されない"
        );
    }
}

// ---- フォルダの作り分け（要件 4.9） ----

/// フォルダのエントリは空フォルダになり、エントリが無くてもファイルの親は作られる。
#[test]
fn folder_entries_become_folders_and_missing_parents_are_derived() {
    let root = root_with_ghost("other");
    let request = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };

    let empty = plan_of(empty_folder_archive(), &request);
    assert_eq!(relative_files(&empty[0]), vec!["install.txt"]);
    assert_eq!(
        empty[0].dirs,
        vec!["work"],
        "中身の無いフォルダのエントリも空フォルダとして残す"
    );

    let derived = plan_of(no_folder_entry_archive(), &request);
    assert_eq!(
        relative_files(&derived[0]),
        vec!["install.txt", "a/b/c.txt"]
    );
    assert_eq!(
        derived[0].dirs,
        vec!["a", "a/b"],
        "フォルダのエントリが無くても親を全段そろえる"
    );
}

// ---- 根の外へ出ない（要件 4.8） ----

/// `candidate` が `root` の配下か。`root` 自身は配下とみなす。
///
/// 宛先はまだ実在しないので [`std::fs::canonicalize`] は使えない。代わりに
/// [`Path::components`] で**要素ごとに**比べる。`components` は `.` と重なった
/// 区切りだけを畳み、`..` は [`Component::ParentDir`] として残すので、根の後ろに
/// 普通の名前（[`Component::Normal`]）以外が 1 つでも出れば外へ出得ると判る。
/// 綴りの前方一致で測らないのは、`work/1234-0` に対する `work/1234-01` のような
/// 兄弟を配下と誤認するため。
fn is_inside(root: &Path, candidate: &Path) -> bool {
    let mut rest = candidate.components();
    for expected in root.components() {
        if rest.next() != Some(expected) {
            return false;
        }
    }
    rest.all(|component| matches!(component, Component::Normal(_)))
}

/// 計画の中で根の外へ出るパスを全て拾い、同時に検査したパスの総数を返す。
///
/// 数えるのは「宛先そのもの」「宛先＋ファイルの相対パス」「宛先＋フォルダの相対パス」の
/// 3 種。1 つの計画に 1 本の道筋を見るのでは、残りが外へ出ていても気付けない。
fn escapes(root: &Path, plan: &[Placement]) -> (Vec<PathBuf>, usize) {
    let mut outside = Vec::new();
    let mut checked = 0;
    let mut check = |path: PathBuf| {
        checked += 1;
        if !is_inside(root, &path) {
            outside.push(path);
        }
    };
    for placement in plan {
        check(placement.destination.clone());
        for (_, relative) in &placement.files {
            check(placement.destination.join(relative));
        }
        for relative in &placement.dirs {
            check(placement.destination.join(relative));
        }
    }
    (outside, checked)
}

/// 受理される固定入力を全て並べる。無印の名前は差分を読むときの手掛かり。
fn every_accepted_plan(root: &WorkDir) -> Vec<(&'static str, Vec<Placement>)> {
    let free = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let targeted = InstallRequest {
        root: root.path(),
        target_ghost: Some("test-ghost"),
    };
    vec![
        ("ghost", plan_of(ghost_archive(), &free)),
        ("balloon", plan_of(balloon_archive(), &free)),
        ("shell", plan_of(shell_archive(), &targeted)),
        ("supplement", plan_of(supplement_archive(), &targeted)),
        ("empty-folder", plan_of(empty_folder_archive(), &free)),
        ("no-folder-entry", plan_of(no_folder_entry_archive(), &free)),
    ]
}

/// 全ての固定入力の全ての宛先・ファイル・フォルダが根の配下に収まる（要件 4.8）。
#[test]
fn every_path_of_every_plan_stays_under_the_root() {
    let root = root_with_ghost("test-ghost");
    let mut total = 0;
    for (label, plan) in every_accepted_plan(&root) {
        assert!(!plan.is_empty(), "{label} の計画が空");
        for placement in &plan {
            assert!(
                !placement.files.is_empty(),
                "{label} の配置 {:?} が 1 件もファイルを置かない",
                placement.destination
            );
        }
        let (outside, checked) = escapes(root.path(), &plan);
        assert!(outside.is_empty(), "{label} が根の外へ出る: {outside:?}");
        total += checked;
    }
    assert_eq!(
        total, 37,
        "数えた道筋の数が変わった（数を固定しないと、計画が痩せても恒真で緑になる）"
    );
}

/// 上の勘定の較正。`..` を含む計画を通せば必ず赤になる。
///
/// [`is_inside`] を単体で試すだけでは、`escapes` が実際にその判定を全ての道筋に
/// 当てている保証がない。手で組んだ配置を同じ入口へ通して測る。
#[test]
fn the_containment_walk_catches_a_plan_that_leaves_the_root() {
    let root = Path::new(r"C:\root\baseware").to_path_buf();
    let planted = vec![Placement {
        kind: ElementKind::Ghost,
        name: "evil".to_owned(),
        destination: root.join("ghost").join(".."),
        target_ghost: None,
        existing: ExistingPolicy::Overlay,
        skip_top_level_install_txt: false,
        files: vec![(0, "../../evil.txt".to_owned())],
        dirs: vec!["ok".to_owned()],
    }];
    let (outside, checked) = escapes(&root, &planted);
    assert_eq!(checked, 3, "3 本とも検査していない");
    assert_eq!(
        outside,
        vec![
            root.join("ghost").join(".."),
            root.join("ghost").join("..").join("../../evil.txt"),
            root.join("ghost").join("..").join("ok"),
        ],
        "`..` を含む道筋が拾えていない"
    );

    // 対照。正しい形は 1 本も拾わない（走査が何でも赤にするのでは意味が無い）。
    let honest = vec![Placement {
        kind: ElementKind::Ghost,
        name: "emo".to_owned(),
        destination: root.join("ghost").join("test-ghost"),
        target_ghost: None,
        existing: ExistingPolicy::Overlay,
        skip_top_level_install_txt: false,
        files: vec![(0, "ghost/master/descript.txt".to_owned())],
        dirs: vec!["ghost".to_owned()],
    }];
    assert_eq!(escapes(&root, &honest), (Vec::new(), 3));
}

/// 兄弟フォルダを配下と誤認しない（綴りの前方一致で測っていたら赤になる）。
#[test]
fn a_sibling_with_a_longer_name_is_not_inside() {
    let root = Path::new(r"C:\work\1234-0").to_path_buf();
    assert!(is_inside(&root, &root));
    assert!(is_inside(&root, &root.join("ghost").join("test-ghost")));
    assert!(!is_inside(
        &root,
        &Path::new(r"C:\work\1234-01").join("ghost")
    ));
    assert!(!is_inside(&root, Path::new(r"C:\other")));
    assert!(!is_inside(&root, &root.join("..")));
}

// ---- 書き込みをしない（要件 4.1・設計「読む → 入れる」） ----

/// 根の下の木を「相対パスと長さ」の名前順の一覧に写す。フォルダは末尾 `/` で表す。
fn snapshot(root: &Path) -> Vec<(String, u64)> {
    let mut found = Vec::new();
    collect_snapshot(root, "", &mut found);
    found.sort();
    found
}

fn collect_snapshot(dir: &Path, prefix: &str, found: &mut Vec<(String, u64)>) {
    for child in std::fs::read_dir(dir).expect("根を読める") {
        let child = child.expect("要素を読める");
        let name = child.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}{name}");
        let meta = child.metadata().expect("属性を読める");
        if meta.is_dir() {
            found.push((format!("{path}/"), 0));
            collect_snapshot(&child.path(), &format!("{path}/"), found);
        } else {
            found.push((path, meta.len()));
        }
    }
}

/// `build_plan` は根を 1 バイトも書き換えない（読むのは宛先ゴーストの実在だけ）。
#[test]
fn build_plan_writes_nothing_under_the_root() {
    let root = root_with_ghost("test-ghost");
    std::fs::write(
        root.path()
            .join("ghost")
            .join("test-ghost")
            .join("install.txt"),
        b"x",
    )
    .expect("宛先ゴーストに 1 件置ける");
    let before = snapshot(root.path());
    assert!(!before.is_empty(), "根が空なら突合は恒真で緑になる");

    let free = InstallRequest {
        root: root.path(),
        target_ghost: None,
    };
    let targeted = InstallRequest {
        root: root.path(),
        target_ghost: Some("test-ghost"),
    };
    let missing = InstallRequest {
        root: root.path(),
        target_ghost: Some("not-installed"),
    };
    let mut walked = 0;
    for (archive, request) in [
        (ghost_archive(), &free),
        (balloon_archive(), &free),
        (shell_archive(), &targeted),
        (supplement_archive(), &targeted),
        (empty_folder_archive(), &free),
        (no_folder_entry_archive(), &free),
        (shell_archive(), &missing),
        (supplement_archive(), &free),
    ] {
        let _ = planned(archive, request);
        walked += 1;
    }
    assert_eq!(walked, 8, "固定入力を 1 本も通していなければ突合は恒真");
    assert_eq!(
        before,
        snapshot(root.path()),
        "build_plan が根の中身を変えた"
    );
}

/// 上の突合の較正。書き込みを 3 通り持ち込めば必ず差が出る。
#[test]
fn the_snapshot_notices_every_shape_of_write() {
    let root = root_with_ghost("test-ghost");
    let ghost = root.path().join("ghost").join("test-ghost");

    let before = snapshot(root.path());
    std::fs::write(ghost.join("x.txt"), b"1").expect("ファイルを作れる");
    let after_create = snapshot(root.path());
    assert_ne!(before, after_create, "ファイルの追加を見落とす");

    std::fs::write(ghost.join("x.txt"), b"22").expect("ファイルを書き替えられる");
    let after_write = snapshot(root.path());
    assert_ne!(after_create, after_write, "中身の変化を見落とす");

    std::fs::create_dir(root.path().join("balloon")).expect("フォルダを作れる");
    assert_ne!(
        after_write,
        snapshot(root.path()),
        "空フォルダの追加を見落とす"
    );
}
