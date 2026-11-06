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

#[test]
fn a_work_folder_that_cannot_be_removed_is_logged() {
    let tmp = TempPath::under_target("ud-copy-work-left");
    let work = tmp.child("work");
    fs::create_dir_all(&work).unwrap();
    fs::write(work.join("held.txt"), "x").unwrap();
    // 中のファイルを共有なしで掴み、消せなくする。
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(work.join("held.txt"))
        .unwrap();

    let ((), events) = log_capture_kit::capture(|| discard(&work));
    drop(held);

    let warns = crate::test_support::logged(&events, "user_data_copy_work_left");
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0].level, tracing::Level::WARN);
    assert!(work.exists());
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

#[test]
fn refuses_to_copy_into_itself() {
    let (_tmp, from, root) = fixture("ud-copy-self");
    let inside = from.join("x");
    // 大小だけ違う綴りも同じフォルダ（Windows の名前の決まり）。
    let upper = from.with_file_name("EMO2").join("x");
    for (to, work_root) in [
        (inside.join("to"), root.clone()),
        (upper.join("to"), root.clone()),
        (root.join("to"), inside.clone()),
    ] {
        let error = copy_tree_atomic(&from, &to, &work_root).expect_err("断るはず");

        assert!(
            matches!(&error, CopyError::Io { error, .. } if error.kind() == std::io::ErrorKind::InvalidInput),
            "{error}"
        );
        assert!(!to.exists());
    }
    // 語末の ς も σ も Σ と同じ名前（NTFS は 1 字ずつ大文字で比べる・to_lowercase では分かれる）。
    let greek = from.with_file_name("ΟΔΟΣ");
    fs::create_dir_all(&greek).unwrap();
    let to = from.with_file_name("οδοσ").join("to");
    let error = copy_tree_atomic(&greek, &to, &root).expect_err("断るはず");
    assert!(
        matches!(&error, CopyError::Io { error, .. } if error.kind() == std::io::ErrorKind::InvalidInput),
        "{error}"
    );
    assert!(!to.exists());
    cleanup(&from);
}

#[test]
fn a_taken_work_address_is_skipped() {
    let tmp = TempPath::under_target("ud-copy-taken");
    let shelf = tmp.child("shelf");
    let pid = std::process::id();
    fs::create_dir_all(shelf.join(format!("{pid}-0"))).unwrap();
    let mut next = 0;

    let work = make_work_dir(&shelf, || {
        next += 1;
        next - 1
    })
    .expect("取り直せる");

    assert_eq!(work, shelf.join(format!("{pid}-1")));
    assert!(work.is_dir());
}

#[test]
fn stale_work_folders_are_swept_and_fresh_ones_kept() {
    let tmp = TempPath::under_target("ud-copy-sweep");
    let shelf = tmp.child("shelf");
    let old = shelf.join("1-0");
    fs::create_dir_all(&old).unwrap();
    fs::write(old.join("a.txt"), "a").unwrap();
    let modified = fs::metadata(&old).unwrap().modified().unwrap();

    sweep_stale(
        &shelf,
        modified + STALE_WORK_AFTER - std::time::Duration::from_secs(1),
    );
    assert!(old.join("a.txt").exists(), "24 時間に満たないものは残す");

    sweep_stale(&shelf, modified + STALE_WORK_AFTER);
    assert!(!old.exists(), "24 時間たったものは消す");
    assert!(shelf.is_dir(), "棚そのものは残す");
}

#[test]
fn a_raced_destination_returns_raced_and_drops_the_work() {
    let tmp = TempPath::under_target("ud-copy-finish");
    let work = tmp.child("work");
    let to = tmp.child("to");
    fs::create_dir_all(&work).unwrap();
    fs::write(work.join("a.txt"), "work").unwrap();
    fs::create_dir_all(&to).unwrap();
    fs::write(to.join("a.txt"), "mine").unwrap();
    let stats = CopyStats {
        files: 1,
        bytes: 4,
        millis: 0,
    };

    let done = finish_rename(&work, &to, fs::rename(&work, &to), stats).expect("失敗ではない");

    assert_eq!(done, CopyDone::Raced);
    assert!(!work.exists());
    assert_eq!(fs::read_to_string(to.join("a.txt")).unwrap(), "mine");
}

/// 再帰で歩けば 256 KiB のスタックを溢れる深さ（1 段 1 字）。
const DEEP: usize = 300;

#[test]
fn deep_trees_copy_on_a_small_stack() {
    let tmp = TempPath::under_target("ud-copy-deep");
    let from = tmp.child("from");
    // 1 段ずつ作る（`create_dir_all` は深い木で親を遡って探すので遅い）。
    let deep = (0..DEEP).fold(from.clone(), |path, _| {
        let next = path.join("d");
        fs::create_dir_all(&next).unwrap();
        next
    });
    fs::write(deep.join("a.txt"), "a").unwrap();
    let root = tmp.child("user");
    fs::create_dir_all(&root).unwrap();
    let to = root.join("to");

    let (from_t, to_t, root_t) = (from.clone(), to.clone(), root.clone());
    let done = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || copy_tree_atomic(&from_t, &to_t, &root_t))
        .unwrap()
        .join()
        .expect("小さなスタックで溢れない")
        .expect("写せる");

    assert!(
        matches!(done, CopyDone::Copied(CopyStats { files: 1, .. })),
        "{done:?}"
    );
    let copied = (0..DEEP).fold(to, |path, _| path.join("d"));
    assert!(copied.join("a.txt").exists());
}
