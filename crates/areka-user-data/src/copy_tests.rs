//! 写す手続きの決定論テスト（要件 4.6・4.7・10.4・10.6）。根はすべて `target\` の下。

use super::*;
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use temp_path_kit::TempPath;

/// 元の木（`ghost\master\descript.txt` と、読み取り専用の `a.txt`・`z.txt`）と書ける根を作る。
fn fixture(label: &str) -> (TempPath, PathBuf, PathBuf) {
    let tmp = TempPath::under_target(label);
    let from = tmp.child("exe").join("ghost").join("emo2");
    fs::create_dir_all(from.join("ghost").join("master")).unwrap();
    fs::write(
        from.join("ghost").join("master").join("descript.txt"),
        "name,emo2",
    )
    .unwrap();
    for name in ["a.txt", "z.txt"] {
        let file = from.join(name);
        fs::write(&file, name).unwrap();
        let mut mode = fs::metadata(&file).unwrap().permissions();
        mode.set_readonly(true);
        fs::set_permissions(&file, mode).unwrap();
    }
    let root = tmp.child("user");
    fs::create_dir_all(&root).unwrap();
    (tmp, from, root)
}

/// `<root>\.copy-work\` の中身の数（棚が無ければ 0）。
fn work_left(root: &Path) -> usize {
    fs::read_dir(root.join(crate::COPY_WORK)).map_or(0, |dir| dir.count())
}

/// 元の木の読み取り専用を落として片付けられるようにする。
fn writable(path: &Path) {
    let mut mode = fs::metadata(path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    mode.set_readonly(false);
    fs::set_permissions(path, mode).unwrap();
}

fn cleanup(from: &Path) {
    writable(&from.join("a.txt"));
    writable(&from.join("z.txt"));
}

fn junction(link: &Path, target: &Path) {
    let out = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .expect("cmd を起こせる");
    assert!(
        out.status.success() && link.exists(),
        "ジャンクションを作れない"
    );
}

#[test]
fn copies_content_and_drops_read_only() {
    let (_tmp, from, root) = fixture("ud-copy");
    let to = root.join("ghost").join("emo2");

    let done = copy_tree_atomic(&from, &to, &root).expect("写せる");

    let CopyDone::Copied(stats) = done else {
        panic!("写したはず: {done:?}");
    };
    assert_eq!(stats.files, 3);
    assert_eq!(stats.bytes, ("name,emo2".len() + 5 + 5) as u64);
    assert_eq!(
        fs::read_to_string(to.join("ghost").join("master").join("descript.txt")).unwrap(),
        "name,emo2"
    );
    assert_eq!(fs::read_to_string(to.join("a.txt")).unwrap(), "a.txt");
    assert!(
        !fs::metadata(to.join("a.txt"))
            .unwrap()
            .permissions()
            .readonly()
    );
    assert!(
        !fs::metadata(to.join("z.txt"))
            .unwrap()
            .permissions()
            .readonly()
    );
    // 元は読むだけ（属性も中身もそのまま）。
    assert!(
        fs::metadata(from.join("a.txt"))
            .unwrap()
            .permissions()
            .readonly()
    );
    assert_eq!(work_left(&root), 0);
    cleanup(&from);
}

#[test]
fn existing_destination_is_not_written() {
    let (_tmp, from, root) = fixture("ud-copy-raced");
    let to = root.join("ghost").join("emo2");
    fs::create_dir_all(&to).unwrap();
    fs::write(to.join("a.txt"), "mine").unwrap();

    let done = copy_tree_atomic(&from, &to, &root).expect("失敗ではない");

    assert_eq!(done, CopyDone::Raced);
    assert_eq!(fs::read_to_string(to.join("a.txt")).unwrap(), "mine");
    assert!(!to.join("z.txt").exists());
    assert_eq!(work_left(&root), 0);
    cleanup(&from);
}

#[test]
fn midway_failure_leaves_no_destination_and_no_work_folder() {
    let (_tmp, from, root) = fixture("ud-copy-fail");
    let to = root.join("ghost").join("emo2");
    // `a.txt` を写した後に `z.txt` を読めない（共有なしで掴む）。
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(from.join("z.txt"))
        .unwrap();

    let error = copy_tree_atomic(&from, &to, &root).expect_err("写せないはず");
    drop(held);

    assert!(
        matches!(&error, CopyError::Io { path, .. } if path.ends_with("z.txt")),
        "{error}"
    );
    assert!(!to.exists());
    assert_eq!(work_left(&root), 0);
    cleanup(&from);
}

#[test]
fn link_in_tree_fails_without_following() {
    let (tmp, from, root) = fixture("ud-copy-link");
    let outside = tmp.child("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "x").unwrap();
    junction(&from.join("link"), &outside);
    let to = root.join("ghost").join("emo2");

    let error = copy_tree_atomic(&from, &to, &root).expect_err("リンクで失敗するはず");

    assert!(
        matches!(&error, CopyError::Link { path } if path.ends_with("link")),
        "{error}"
    );
    assert!(!to.exists());
    assert_eq!(work_left(&root), 0);
    // 元のリンクの先は触らない。
    assert!(outside.join("secret.txt").exists());
    fs::remove_dir(from.join("link")).unwrap();
    cleanup(&from);
}
