//! 作業場所の作成・書き込み・片付け・他の走行の残骸（要件 4.1・4.7・4.8・5.5）。

use super::*;
use crate::testkit::{hold, tree};
use sample_ghost_kit::WorkDir;

fn fixture() -> (WorkDir, PathBuf) {
    let work = WorkDir::new().expect("作業フォルダ");
    let target = work.path().join("target");
    fs::create_dir_all(&target).unwrap();
    (work, target)
}

/// 他の走行が残したフォルダ `<棚>/<name>/<rel>` に中身を置く。
fn other_run(target: &Path, name: &str, rel: &str) -> PathBuf {
    let dir = target.join(WORK_DIR).join(name);
    let file = dir.join(rel);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"x").unwrap();
    dir
}

#[test]
fn create_makes_own_folder_under_shelf_named_by_pid_and_serial() {
    let (_w, target) = fixture();
    // 同じ対象への同時の走行は前提の外（run の前提条件）なので、1 周ずつ作って片付ける。
    let mut dirs = Vec::new();
    for _ in 0..2 {
        let area = WorkArea::create(&target).expect("作れる");
        assert!(area.dir().is_dir());
        assert_eq!(area.dir().parent().unwrap(), target.join(WORK_DIR));
        let name = area.dir().file_name().unwrap().to_str().unwrap();
        let (pid, serial) = name.split_once('-').expect("<pid>-<連番>");
        assert_eq!(pid, std::process::id().to_string());
        serial.parse::<u32>().expect("連番は数");
        assert_eq!(
            area.fresh("x/y.txt"),
            area.dir().join("new").join("x/y.txt")
        );
        assert_eq!(
            area.retired("x/y.txt"),
            area.dir().join("old").join("x/y.txt")
        );
        dirs.push(area.dir().to_path_buf());
        assert!(area.cleanup().is_empty());
    }
    assert_ne!(dirs[0], dirs[1], "同じプロセスの 2 回目は別の連番");
}

#[test]
fn put_writes_bytes_unchanged_under_new_creating_parents() {
    let (_w, target) = fixture();
    let area = WorkArea::create(&target).unwrap();
    let bytes = b"a\r\nb\n\x82\xa0\x00\xff";
    area.put("sub/dir/f.bin", bytes).expect("書ける");
    assert_eq!(fs::read(area.fresh("sub/dir/f.bin")).unwrap(), bytes);
    area.cleanup();
}

#[test]
fn put_failure_names_the_path() {
    let (_w, target) = fixture();
    let area = WorkArea::create(&target).unwrap();
    area.put("a", b"file").unwrap();
    let (path, _) = area
        .put("a/b", b"x")
        .expect_err("親がファイルなので書けない");
    assert!(path.starts_with(area.fresh("a")), "{}", path.display());
    area.cleanup();
}

#[test]
fn cleanup_removes_own_folder_and_empty_shelf() {
    let (_w, target) = fixture();
    fs::write(target.join("keep.txt"), b"k").unwrap();
    let before = tree(&target);
    let area = WorkArea::create(&target).unwrap();
    area.put("deep/a.txt", b"a").unwrap();
    assert!(area.cleanup().is_empty());
    assert_eq!(tree(&target), before, "棚ごと消えて対象は元どおり");
}

#[test]
fn create_removes_other_runs_residue() {
    let (_w, target) = fixture();
    let other = other_run(&target, "999-0", "new/a.txt");
    fs::write(target.join(WORK_DIR).join("stray.txt"), b"s").unwrap();
    let area = WorkArea::create(&target).unwrap();
    assert!(!other.exists(), "他の走行の残骸は消える");
    assert!(!target.join(WORK_DIR).join("stray.txt").exists());
    assert!(area.cleanup().is_empty());
    assert!(!target.join(WORK_DIR).exists());
}

#[test]
fn undeletable_residue_is_listed_not_fatal() {
    let (_w, target) = fixture();
    let other = other_run(&target, "999-0", "new/a.txt");
    let held = hold(&other.join("new/a.txt"));
    let area = WorkArea::create(&target).expect("残骸が消せなくても止めない");
    // 較正: 消せなかったことを実物で確かめる（黙って消えていたら赤）。
    assert!(other.join("new/a.txt").exists());
    let leftovers = area.cleanup();
    assert_eq!(leftovers, vec![other.clone()]);
    assert!(target.join(WORK_DIR).is_dir(), "残骸があるので棚は残る");
    drop(held);
}

#[test]
fn cleanup_lists_own_folder_it_cannot_remove() {
    let (_w, target) = fixture();
    let area = WorkArea::create(&target).unwrap();
    area.put("a.txt", b"a").unwrap();
    let dir = area.dir().to_path_buf();
    let held = hold(&area.fresh("a.txt"));
    let leftovers = area.cleanup();
    assert!(dir.join("new/a.txt").exists(), "較正: 実際に消せていない");
    assert_eq!(leftovers, vec![dir]);
    drop(held);
}

#[test]
fn folder_with_content_in_old_survives_repeated_creates_and_is_listed() {
    let (_w, target) = fixture();
    let stuck = other_run(&target, "999-1", "old/sub/orig.txt");
    fs::create_dir_all(stuck.join("new")).unwrap();
    // `old/` が空のフォルダは戻せなかった走行ではないので消える。
    let empty_old = target.join(WORK_DIR).join("999-2");
    fs::create_dir_all(empty_old.join("old")).unwrap();
    for _ in 0..2 {
        let area = WorkArea::create(&target).unwrap();
        assert!(
            stuck.join("old/sub/orig.txt").exists(),
            "元の内容は消さない"
        );
        assert!(!empty_old.exists());
        assert_eq!(area.cleanup(), vec![stuck.clone()]);
    }
}

#[test]
fn keep_leaves_folder_and_returns_path_with_residue() {
    let (_w, target) = fixture();
    let stuck = other_run(&target, "999-1", "old/orig.txt");
    let area = WorkArea::create(&target).unwrap();
    area.put("a.txt", b"a").unwrap();
    let dir = area.dir().to_path_buf();
    let (kept, residue) = area.keep();
    assert_eq!(kept, dir);
    assert_eq!(residue, vec![stuck]);
    assert!(dir.join("new/a.txt").exists());
}

#[test]
fn create_fails_with_path_when_shelf_is_a_file() {
    let (_w, target) = fixture();
    fs::write(target.join(WORK_DIR), b"blocked").unwrap();
    let (path, _) = WorkArea::create(&target)
        .map(|_| ())
        .expect_err("棚がファイルなら作れない");
    assert_eq!(path, target.join(WORK_DIR));
}
