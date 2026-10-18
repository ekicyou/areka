//! `evidence` の兄弟のテスト。落ちたかの真偽を `finish` へ直に渡して、残す・消すを確かめる。
//! 残した置き場は各テストの最後に自分で消す。

use super::*;

#[test]
fn failed_finish_keeps_run_dir_and_sample_copy() {
    let mut dirs = RunDirs::create(Some("emo2"));
    let run = dirs.run().to_path_buf();
    assert!(dirs.root().is_dir() && dirs.profile().is_dir());
    assert_eq!(dirs.stdout_log(), run.join("stdout.log"));
    assert_eq!(dirs.stderr_log(), run.join("stderr.log"));
    let copy = dirs
        .sample()
        .expect("検体の写しがあるはず")
        .root()
        .to_path_buf();
    assert!(copy.join("ghost").join("emo2").is_dir());

    dirs.finish(true);
    drop(dirs);

    assert!(
        run.is_dir(),
        "落ちた回の置き場が残るはず: {}",
        run.display()
    );
    assert!(
        run.join("sample").join("ghost").join("emo2").is_dir(),
        "検体の写しが <回>\\sample へ複写されるはず"
    );
    assert!(run.starts_with(target_test_roots_of(&run)));
    std::fs::remove_dir_all(&run).unwrap();
}

#[test]
fn successful_finish_removes_run_dir_and_sample_copy() {
    let mut dirs = RunDirs::create(Some("emo2"));
    let run = dirs.run().to_path_buf();
    let copy = dirs.sample().unwrap().root().to_path_buf();
    std::fs::write(dirs.stdout_log(), "out\n").unwrap();

    dirs.finish(false);
    drop(dirs);

    assert!(
        !run.exists(),
        "成功した回の置き場は消えるはず: {}",
        run.display()
    );
    assert!(
        !copy.exists(),
        "成功した回の写しは消えるはず: {}",
        copy.display()
    );
}

#[test]
fn failed_finish_without_sample_keeps_run_dir_without_sample() {
    let mut dirs = RunDirs::create(None);
    let run = dirs.run().to_path_buf();
    assert!(dirs.sample().is_none());

    dirs.finish(true);
    drop(dirs);

    assert!(run.is_dir());
    assert!(!run.join("sample").exists());
    std::fs::remove_dir_all(&run).unwrap();
}

#[test]
fn non_empty_handed_folder_is_neither_used_nor_deleted() {
    let stale = TempPath::under_target("rr");
    let stale_path = stale.path().to_path_buf();
    std::fs::write(stale.child("stdout.log"), "前の回の証拠").unwrap();
    let mut handed = vec![TempPath::under_target("rr"), stale];

    let dirs = RunDirs::create_with(None, || handed.pop().unwrap());
    let run = dirs.run().to_path_buf();
    drop(dirs);

    assert_ne!(run, stale_path, "中身のあるフォルダは使わないはず");
    assert_eq!(
        std::fs::read_to_string(stale_path.join("stdout.log")).unwrap(),
        "前の回の証拠",
        "中身のあるフォルダは消さないはず"
    );
    std::fs::remove_dir_all(&stale_path).unwrap();
}

#[test]
#[should_panic(expected = "16")]
fn non_empty_folders_up_to_the_limit_fail() {
    let mut kept = Vec::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        RunDirs::create_with(None, || {
            let t = TempPath::under_target("rr");
            std::fs::write(t.child("x"), "x").unwrap();
            kept.push(t.path().to_path_buf());
            t
        })
    }));
    for path in &kept {
        let _ = std::fs::remove_dir_all(path);
    }
    assert_eq!(kept.len(), CREATE_TRIES);
    std::panic::resume_unwind(result.err().expect("上限で失敗するはず"));
}

/// 置き場が `target\test-roots\` の下にあることの確かめ用（`<回>` の親）。
fn target_test_roots_of(run: &std::path::Path) -> std::path::PathBuf {
    let parent = run.parent().unwrap();
    assert!(parent.ends_with(std::path::Path::new("target").join("test-roots")));
    parent.to_path_buf()
}
