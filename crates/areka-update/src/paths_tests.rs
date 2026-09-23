//! 区切りの正規化・作業場所の判定・「実パスが対象フォルダの配下か」（要件 1.13・5.6・6.3）。

use super::*;
use crate::testkit::junction;
use sample_ghost_kit::WorkDir;
use std::fs;

// ---- 区切りの正規化 ----

#[test]
fn backslash_becomes_slash_and_slash_stays() {
    assert_eq!(normalize_separators(r"a\b/c\\d"), "a/b/c//d");
    assert_eq!(normalize_separators(r"..\x"), "../x");
    assert_eq!(normalize_separators("plain"), "plain");
}

// ---- 作業場所の判定 ----

#[test]
fn first_component_equal_to_work_dir_ignoring_case_is_the_work_area() {
    assert!(is_in_work_area(".update-work"));
    assert!(is_in_work_area(".update-work/1-0/new/a.txt"));
    assert!(is_in_work_area(".UPDATE-Work/x"));
    assert!(is_in_work_area(r".update-work\x"));
}

#[test]
fn other_first_components_are_not_the_work_area() {
    assert!(!is_in_work_area(".update-workx/a"));
    assert!(!is_in_work_area("update-work/a"));
    assert!(!is_in_work_area("a/.update-work/b"));
    assert!(!is_in_work_area(""));
}

// ---- ローカルパス ----

#[test]
fn local_path_is_the_target_joined_with_the_relative_name_as_is() {
    let target = Path::new(r"C:\ghost\g");
    assert_eq!(
        local_path(target, "shell/master/a.png"),
        target.join("shell/master/a.png")
    );
    // 比較は部品単位なので、区切りの綴りが違っても同じ場所。
    assert_eq!(
        local_path(target, "shell/master/a.png"),
        PathBuf::from(r"C:\ghost\g\shell\master\a.png")
    );
}

// ---- 配下の判定 ----

struct Fixture {
    _work: WorkDir,
    target: PathBuf,
    target_real: PathBuf,
    outside: PathBuf,
}

fn fixture() -> Fixture {
    let work = WorkDir::new().expect("作業フォルダ");
    let target = work.path().join("target");
    let outside = work.path().join("outside");
    fs::create_dir_all(target.join("sub")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(target.join("sub/a.txt"), b"a").unwrap();
    fs::write(outside.join("o.txt"), b"o").unwrap();
    let target_real = fs::canonicalize(&target).unwrap();
    Fixture {
        _work: work,
        target,
        target_real,
        outside,
    }
}

#[test]
fn existing_and_not_yet_existing_paths_inside_the_target_are_under() {
    let f = fixture();
    assert!(resolves_under(&f.target_real, &f.target).unwrap());
    assert!(resolves_under(&f.target_real, &f.target.join("sub/a.txt")).unwrap());
    assert!(resolves_under(&f.target_real, &f.target.join("new/deep/b.txt")).unwrap());
    // 途中にファイルが挟まる（作れないパス）でも、外へは出ていない。
    assert!(resolves_under(&f.target_real, &f.target.join("sub/a.txt/x")).unwrap());
}

#[test]
fn paths_outside_the_target_are_not_under() {
    let f = fixture();
    assert!(!resolves_under(&f.target_real, &f.outside.join("o.txt")).unwrap());
    // 名前の頭が同じだけの隣（文字列の前方一致ではなく部品で比べる）。
    let sibling = f.target.with_file_name("target2");
    fs::create_dir_all(&sibling).unwrap();
    assert!(!resolves_under(&f.target_real, &sibling.join("x")).unwrap());
}

#[test]
fn a_junction_pointing_outside_is_not_under_even_for_paths_not_yet_created() {
    let f = fixture();
    let link = f.target.join("link");
    junction(&link, &f.outside);

    assert!(!resolves_under(&f.target_real, &link).unwrap());
    assert!(!resolves_under(&f.target_real, &link.join("o.txt")).unwrap());
    assert!(!resolves_under(&f.target_real, &link.join("new/deep/b.txt")).unwrap());
    // 較正: ジャンクションの横の普通のパスは配下のまま。
    assert!(resolves_under(&f.target_real, &f.target.join("sub/a.txt")).unwrap());
}

#[test]
fn a_dangling_junction_is_not_under() {
    // 行き先の消えたジャンクションは実パスを確かめられない → 安全側で外扱い。
    let f = fixture();
    let gone = f.outside.join("gone");
    fs::create_dir_all(&gone).unwrap();
    let link = f.target.join("dangling");
    junction(&link, &gone);
    fs::remove_dir(&gone).unwrap();

    assert!(!resolves_under(&f.target_real, &link.join("b.txt")).unwrap());
}
