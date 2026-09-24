//! 確定と逆順の戻し（要件 2.3・5.2・5.4〜5.7・9.4）。
//! 注入（保持・削除を拒む仕掛け・ジャンクション）は `commit` を呼ぶ前に仕込む。

use super::*;
use crate::paths::WORK_DIR;
use crate::testkit::{hold, junction, pe_image, pin, tree};
use sample_ghost_kit::WorkDir;
use std::collections::BTreeMap;

/// `ERROR_SHARING_VIOLATION`（`hold` で掴まれた宛先を退避しようとした）。
const SHARING_VIOLATION: i32 = 32;

struct Fixture {
    _work: WorkDir,
    root: PathBuf,
    target: PathBuf,
    target_real: PathBuf,
    area: WorkArea,
}

fn fixture() -> Fixture {
    let work = WorkDir::new().expect("作業フォルダ");
    let root = work.path().to_path_buf();
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();
    let target_real = fs::canonicalize(&target).unwrap();
    let area = WorkArea::create(&target).unwrap();
    Fixture {
        _work: work,
        root,
        target,
        target_real,
        area,
    }
}

fn names(files: &[&str]) -> Vec<String> {
    files.iter().map(|f| f.to_string()).collect()
}

/// 対象フォルダの木から作業場所を除いた物（利用者に見える内容）。
fn visible(target: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut t = tree(target);
    t.retain(|k, _| !k.starts_with(WORK_DIR));
    t
}

#[test]
fn calibrate_pin_lets_rename_through_and_refuses_remove() {
    // 較正の記録: 読み取り専用属性は Rust 1.98 の `remove_file` が消してしまう（使えない）。
    let fx = fixture();
    let ro = fx.root.join("ro.txt");
    fs::write(&ro, b"x").unwrap();
    let mut perm = fs::metadata(&ro).unwrap().permissions();
    perm.set_readonly(true);
    fs::set_permissions(&ro, perm).unwrap();
    assert!(
        fs::remove_file(&ro).is_ok(),
        "読み取り専用が削除を拒むなら較正をやり直す"
    );

    let src = fx.root.join("a.bin");
    let dst = fx.target.join("a.bin");
    fs::write(&src, pe_image()).unwrap();
    let pinned = pin(&src);
    fs::rename(&src, &dst).expect("写したままでも移せる");
    let refused = fs::remove_file(&dst).expect_err("写したままは消せない");
    assert_eq!(refused.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(dst.exists());
    drop(pinned);
    fs::remove_file(&dst).expect("写しを解けば消せる");
}

#[test]
fn places_files_in_order_and_retires_existing_leaving_others_alone() {
    let fx = fixture();
    fs::write(fx.target.join("a.txt"), b"old").unwrap();
    fs::write(fx.target.join("keep.txt"), b"k").unwrap();
    fx.area.put("a.txt", b"new").unwrap();
    fx.area.put("sub/deep/b.txt", b"b").unwrap();
    let files = names(&["a.txt", "sub/deep/b.txt"]);
    let placed = commit(&fx.target, &fx.target_real, &fx.area, &files)
        .unwrap_or_else(|e| panic!("置ける: {e:?}"));
    assert_eq!(placed, files);
    assert_eq!(fs::read(fx.target.join("a.txt")).unwrap(), b"new");
    assert_eq!(fs::read(fx.target.join("sub/deep/b.txt")).unwrap(), b"b");
    assert_eq!(fs::read(fx.target.join("keep.txt")).unwrap(), b"k");
    assert_eq!(fs::read(fx.area.retired("a.txt")).unwrap(), b"old");
}

/// ⑴ 宛先の保持で退避が失敗 → 木がバイト単位で同一（9.4・5.7）。
#[test]
fn held_destination_fails_retire_and_tree_is_identical() {
    let fx = fixture();
    fs::write(fx.target.join("a.txt"), b"old").unwrap();
    fs::write(fx.target.join("keep.txt"), b"k").unwrap();
    fx.area.put("a.txt", b"new").unwrap();
    let before = visible(&fx.target);
    let held = hold(&fx.target.join("a.txt"));
    let err = commit(&fx.target, &fx.target_real, &fx.area, &names(&["a.txt"]))
        .expect_err("掴まれた宛先は退避できない");
    drop(held);
    match err {
        CommitFailure::Write { path, source } => {
            assert_eq!(path, fx.target.join("a.txt"));
            assert_eq!(source.raw_os_error(), Some(SHARING_VIOLATION));
        }
        other => panic!("Write のはず: {other:?}"),
    }
    assert_eq!(visible(&fx.target), before);
}

/// ⑵ 無い親の下に置いた 1 件目が 2 件目の失敗で親ごと消える（5.4）。
#[test]
fn second_failure_removes_first_and_the_parents_it_created() {
    let fx = fixture();
    fs::write(fx.target.join("b.txt"), b"old-b").unwrap();
    fx.area.put("sub/deep/a.txt", b"a").unwrap();
    fx.area.put("b.txt", b"new-b").unwrap();
    let before = visible(&fx.target);
    let held = hold(&fx.target.join("b.txt"));
    let err = commit(
        &fx.target,
        &fx.target_real,
        &fx.area,
        &names(&["sub/deep/a.txt", "b.txt"]),
    )
    .expect_err("2 件目で失敗");
    drop(held);
    match err {
        CommitFailure::Write { path, source } => {
            assert_eq!(path, fx.target.join("b.txt"));
            assert_eq!(source.raw_os_error(), Some(SHARING_VIOLATION));
        }
        other => panic!("Write のはず: {other:?}"),
    }
    assert!(
        !fx.area.fresh("sub/deep/a.txt").exists(),
        "1 件目は一度置かれた（new/ から出た）"
    );
    assert!(!fx.target.join("sub").exists(), "作った親ごと消える");
    assert_eq!(visible(&fx.target), before);
}

/// ⑵ の対: 元から在った親フォルダは、空でも中身があっても戻しで消さない（作った段だけ消す）。
#[test]
fn second_failure_keeps_parents_that_already_existed() {
    for unrelated in [false, true] {
        let fx = fixture();
        let sub = fx.target.join("sub");
        fs::create_dir_all(&sub).unwrap();
        if unrelated {
            fs::write(sub.join("other.txt"), b"o").unwrap();
        }
        fs::write(fx.target.join("b.txt"), b"old-b").unwrap();
        fx.area.put("sub/deep/a.txt", b"a").unwrap();
        fx.area.put("b.txt", b"new-b").unwrap();
        let before = visible(&fx.target);
        let held = hold(&fx.target.join("b.txt"));
        let err = commit(
            &fx.target,
            &fx.target_real,
            &fx.area,
            &names(&["sub/deep/a.txt", "b.txt"]),
        )
        .expect_err("2 件目で失敗");
        drop(held);
        assert!(
            matches!(err, CommitFailure::Write { .. }),
            "Write のはず: {err:?}"
        );
        assert!(
            !fx.area.fresh("sub/deep/a.txt").exists(),
            "1 件目は一度置かれた"
        );
        assert!(
            sub.is_dir(),
            "元から在った sub/ は残る（中身あり={unrelated}）"
        );
        assert!(!sub.join("deep").exists(), "作った sub/deep は消える");
        assert_eq!(visible(&fx.target), before, "中身あり={unrelated}");
    }
}

/// 削除も復元も失敗したファイルは、戻せなかった一覧に 1 件だけ載る（5.5）。
#[test]
fn file_whose_remove_and_restore_both_fail_is_stuck_once() {
    let fx = fixture();
    fs::write(fx.target.join("a.bin"), b"old-a").unwrap();
    fs::write(fx.target.join("b.txt"), b"old-b").unwrap();
    fx.area.put("a.bin", &pe_image()).unwrap();
    fx.area.put("b.txt", b"new-b").unwrap();
    let pinned = pin(&fx.area.fresh("a.bin"));
    let held = hold(&fx.target.join("b.txt"));
    let err = commit(
        &fx.target,
        &fx.target_real,
        &fx.area,
        &names(&["a.bin", "b.txt"]),
    )
    .expect_err("2 件目で失敗し、1 件目は消せず上書きもできない");
    drop(held);
    match err {
        CommitFailure::RollbackFailed {
            restored, stuck, ..
        } => {
            assert!(restored.is_empty(), "{restored:?}");
            let stuck: Vec<&str> = stuck.iter().map(|s| s.file.as_str()).collect();
            assert_eq!(stuck, ["a.bin"]);
        }
        other => panic!("RollbackFailed のはず: {other:?}"),
    }
    assert_eq!(
        fs::read(fx.area.retired("a.bin")).unwrap(),
        b"old-a",
        "元の内容は old/ に残る"
    );
    drop(pinned);
}

/// ⑶ 削除を拒む注入で戻せなかった一覧が返る。戻しは途中の失敗で止まらない（5.5）。
#[test]
fn refused_removal_reports_stuck_and_keeps_unwinding() {
    let fx = fixture();
    fs::write(fx.target.join("b.txt"), b"old-b").unwrap();
    fx.area.put("x/c.txt", b"c").unwrap();
    fx.area.put("a.bin", &pe_image()).unwrap();
    fx.area.put("b.txt", b"new-b").unwrap();
    let pinned = pin(&fx.area.fresh("a.bin"));
    let held = hold(&fx.target.join("b.txt"));
    let err = commit(
        &fx.target,
        &fx.target_real,
        &fx.area,
        &names(&["x/c.txt", "a.bin", "b.txt"]),
    )
    .expect_err("2 件目で失敗し、1 件目を消せない");
    drop(held);
    match err {
        CommitFailure::RollbackFailed {
            path,
            source,
            restored,
            stuck,
        } => {
            assert_eq!(path, fx.target.join("b.txt"));
            assert_eq!(source.raw_os_error(), Some(SHARING_VIOLATION));
            assert_eq!(restored, names(&["x/c.txt"]), "a.bin の後も戻しを続けた");
            let stuck: Vec<&str> = stuck.iter().map(|s| s.file.as_str()).collect();
            assert_eq!(stuck, ["a.bin"]);
        }
        other => panic!("RollbackFailed のはず: {other:?}"),
    }
    assert!(fx.target.join("a.bin").exists(), "戻せなかった物は残る");
    assert!(!fx.target.join("x").exists(), "戻せた物は消えた");
    assert_eq!(fs::read(fx.target.join("b.txt")).unwrap(), b"old-b");
    drop(pinned);
}

/// ⑷ ジャンクションに置き換えた親で外側に何も作られず 1 件目が戻る（5.6）。
#[test]
fn junctioned_parent_escapes_creates_nothing_outside_and_restores_first() {
    let fx = fixture();
    let outside = fx.root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(fx.target.join("a.txt"), b"old-a").unwrap();
    fs::create_dir_all(fx.target.join("sub")).unwrap();
    fx.area.put("a.txt", b"new-a").unwrap();
    fx.area.put("sub/deeper/b.txt", b"b").unwrap();
    let before = visible(&fx.target);
    let sub = fx.target.join("sub");
    fs::remove_dir(&sub).unwrap();
    junction(&sub, &outside);

    let err = commit(
        &fx.target,
        &fx.target_real,
        &fx.area,
        &names(&["a.txt", "sub/deeper/b.txt"]),
    )
    .expect_err("外へ解決される");
    match err {
        CommitFailure::Escapes { path } => assert_eq!(path, fx.target.join("sub/deeper/b.txt")),
        other => panic!("Escapes のはず: {other:?}"),
    }
    assert!(tree(&outside).is_empty(), "外側には何も作らない");
    fs::remove_dir(&sub).unwrap();
    fs::create_dir(&sub).unwrap();
    assert_eq!(visible(&fx.target), before, "1 件目は戻された");
}
