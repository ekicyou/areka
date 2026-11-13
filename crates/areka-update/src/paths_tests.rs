//! 区切りの正規化・作業場所の判定・「実パスが対象フォルダの配下か」（要件 1.13・5.6・6.3）。

use super::*;
use crate::testkit::{junction, target_fixture};
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

// ---- 配下の判定 ----

struct Fixture {
    _work: WorkDir,
    target: PathBuf,
    target_real: PathBuf,
    outside: PathBuf,
}

fn fixture() -> Fixture {
    let (work, target, target_real) = target_fixture();
    let outside = work.path().join("outside");
    fs::create_dir_all(target.join("sub")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(target.join("sub/a.txt"), b"a").unwrap();
    fs::write(outside.join("o.txt"), b"o").unwrap();
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
fn a_self_looping_junction_is_an_error_not_under() {
    let f = fixture();
    let link = f.target.join("loop");
    junction(&link, &link);
    // 較正: 開くと NotFound でない失敗（1921）。崩れたらテストを曲げずに止める。
    let open = fs::metadata(&link).expect_err("較正: 自分を指すジャンクションは開けない");
    assert_ne!(open.kind(), std::io::ErrorKind::NotFound, "較正: {open}");

    // 下の名前は `symlink_metadata` で、リンクそのものは `canonicalize` で失敗する。
    let below = resolves_under(&f.target_real, &link.join("x")).expect_err("Ok に丸めない");
    assert_ne!(below.kind(), std::io::ErrorKind::NotFound, "{below}");
    let itself = resolves_under(&f.target_real, &link).expect_err("Ok に丸めない");
    assert_ne!(itself.kind(), std::io::ErrorKind::NotFound, "{itself}");
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
    // `..` そのものは数えない（呼び手が `DotDot` として拒む）。
    for ok in ["a", "a/b.txt", "..", "a/../b", ".hidden", "a.b/c d.txt"] {
        assert!(!unsafe_component(ok), "{ok:?} は通るはず");
    }
}
