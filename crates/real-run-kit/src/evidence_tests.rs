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
    assert!(
        run.parent()
            .unwrap()
            .ends_with(Path::new("target").join("test-roots"))
    );
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
