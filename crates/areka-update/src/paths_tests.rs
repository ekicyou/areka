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

#[test]
fn paths_resolving_into_the_work_area_are_not_under() {
    let f = fixture();
    let area = f.target.join(WORK_DIR);
    fs::create_dir_all(area.join("1-0")).unwrap();
    let link = f.target.join("to-area");
    junction(&link, &area);

    // 綴りでは見抜けない入口（ジャンクション）。在る物も、まだ無い物も。
    assert!(!resolves_under(&f.target_real, &link).unwrap());
    assert!(!resolves_under(&f.target_real, &link.join("1-0")).unwrap());
    assert!(!resolves_under(&f.target_real, &link.join("new/x.txt")).unwrap());
    // 綴りどおりの作業場所も、大小違いも。
    assert!(!resolves_under(&f.target_real, &area).unwrap());
    assert!(!resolves_under(&f.target_real, &f.target.join(".UPDATE-WORK/x")).unwrap());
    // 較正: 名前の頭が同じだけの隣は配下。
    assert!(resolves_under(&f.target_real, &f.target.join(".update-workx/a")).unwrap());
}

#[test]
fn the_short_name_of_the_work_area_is_not_under() {
    let f = fixture();
    let area = f.target.join(WORK_DIR);
    fs::create_dir_all(&area).unwrap();
    let Some(short) = crate::testkit::short_name(&area) else {
        // 決定論の檻は上のジャンクション版が持つ。ここは短縮名のあるボリュームでだけ判定する。
        eprintln!(
            "SKIP: {} に 8.3 短縮名が無い（ボリュームが作らない設定）",
            area.display()
        );
        return;
    };
    assert!(!resolves_under(&f.target_real, &f.target.join(&short)).unwrap());
    assert!(!resolves_under(&f.target_real, &f.target.join(&short).join("x.txt")).unwrap());
}

// ---- Windows が綴りと違う物を開く区切り要素 ----

#[test]
fn unsafe_component_catches_what_windows_rewrites() {
    for bad in [
        "",
        " ",
        ".",
        ". ",
        "...",
        "a//b",
        "a/./b",
        "sub/.. ",
        "a./b",
        "a /b",
        "x.",
        "x ",
        "a:b",
        ".update-work::$INDEX_ALLOCATION",
    ] {
        assert!(unsafe_component(bad), "{bad:?} は危ないはず");
    }
    // `..` そのものは呼び手が `DotDot` として先に拒む（ここでは数えない）。
    for ok in ["a", "a/b.txt", "..", "a/../b", ".hidden", "a.b/c d.txt"] {
        assert!(!unsafe_component(ok), "{ok:?} は通るはず");
    }
}
