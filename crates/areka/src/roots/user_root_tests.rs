//! 写す判断と写す口の決定論テスト（要件 3.4・4.3・4.4・4.7〜4.9・7.1・10.3・10.4）。
//!
//! 並びは印を自分で付けて組み、プロセスの形（`layout()`）に頼らない。実在の根は
//! `TempPath::under_target`（ワークツリーの `target\` の下）に掘る。

use super::*;
use areka_user_data::{Reason, UserRootOrigin};
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use temp_path_kit::TempPath;

/// 並び `[書ける根（印付き）, 実行ファイルの隣]` と、同梱のゴースト `exe\ghost\emo2`（中身 1 つ）。
struct Fixture {
    _tmp: TempPath,
    user: PathBuf,
    exe: PathBuf,
}

fn fixture(label: &str) -> Fixture {
    let tmp = TempPath::under_target(label);
    let user = tmp.path().join("user");
    let exe = tmp.path().join("exe");
    fs::create_dir_all(&user).unwrap();
    let bundled = exe.join("ghost").join("emo2");
    fs::create_dir_all(&bundled).unwrap();
    fs::write(bundled.join("a.txt"), "bundled").unwrap();
    Fixture {
        _tmp: tmp,
        user,
        exe,
    }
}

fn marked(user: &Path, exe: &Path) -> BasewareRoots {
    BasewareRoots::new(vec![
        BasewareRoot::user_root(user.to_path_buf(), false),
        BasewareRoot::new(exe.to_path_buf()),
    ])
}

// ---- plan_copy ----

#[test]
fn unmarked_list_is_as_is() {
    let f = fixture("ur-plan-unmarked");
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(f.user.clone()),
        BasewareRoot::new(f.exe.clone()),
    ]);
    let dir = f.exe.join("ghost").join("emo2");
    assert_eq!(
        plan_copy(&roots, Some(&f.exe), StoreKind::Ghost, &dir),
        CopyPlan::AsIs
    );
}

#[test]
fn ghost_and_balloon_under_exe_root_are_copied() {
    let f = fixture("ur-plan-copy");
    let roots = marked(&f.user, &f.exe);
    let ghost = f.exe.join("ghost").join("emo2");
    assert_eq!(
        plan_copy(&roots, Some(&f.exe), StoreKind::Ghost, &ghost),
        CopyPlan::Copy {
            from: ghost.clone(),
            to: f.user.join("ghost").join("emo2"),
        }
    );
    let balloon = f.exe.join("balloon").join("bln");
    assert_eq!(
        plan_copy(&roots, Some(&f.exe), StoreKind::Balloon, &balloon),
        CopyPlan::Copy {
            from: balloon.clone(),
            to: f.user.join("balloon").join("bln"),
        }
    );
}

#[test]
fn items_of_other_roots_and_shell_folders_are_as_is() {
    let f = fixture("ur-plan-other");
    let other = f._tmp.path().join("other");
    let roots = BasewareRoots::new(vec![
        BasewareRoot::user_root(f.user.clone(), false),
        BasewareRoot::new(f.exe.clone()),
        BasewareRoot::new(other.clone()),
    ]);
    let exe = Some(f.exe.as_path());
    // ほかの根の物。
    let in_other = other.join("ghost").join("x");
    assert_eq!(
        plan_copy(&roots, exe, StoreKind::Ghost, &in_other),
        CopyPlan::AsIs
    );
    // 書ける根そのものの物。
    let in_user = f.user.join("ghost").join("emo2");
    assert_eq!(
        plan_copy(&roots, exe, StoreKind::Ghost, &in_user),
        CopyPlan::AsIs
    );
    // シェルのフォルダ（ゴーストの置き場の直下でない）。
    let shell = f
        .exe
        .join("ghost")
        .join("emo2")
        .join("shell")
        .join("master");
    assert_eq!(
        plan_copy(&roots, exe, StoreKind::Ghost, &shell),
        CopyPlan::AsIs
    );
    // 種類が違う置き場（バルーンとしてゴーストの置き場の物を問う）。
    let ghost = f.exe.join("ghost").join("emo2");
    assert_eq!(
        plan_copy(&roots, exe, StoreKind::Balloon, &ghost),
        CopyPlan::AsIs
    );
}

/// ゴーストの中のバルーン（親が `<実行ファイルの隣>\balloon` でない）は、隣の根のゴーストの中でも
/// 書ける根の写しの中でも写さない（spec: areka-P0-ghost-inner-balloon 要件 8.3）。
#[test]
fn ghost_inner_balloon_is_as_is() {
    let f = fixture("ur-plan-ghost-inner-balloon");
    let roots = marked(&f.user, &f.exe);
    for ghost in [
        f.exe.join("ghost").join("emo2"),
        f.user.join("ghost").join("emo2"),
    ] {
        let inner = ghost.join("balloon").join("inner");
        assert_eq!(
            plan_copy(&roots, Some(&f.exe), StoreKind::Balloon, &inner),
            CopyPlan::AsIs
        );
    }
}

#[test]
fn user_root_below_exe_root_is_as_is() {
    let f = fixture("ur-plan-below");
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(f.exe.clone()),
        BasewareRoot::user_root(f.user.clone(), false),
    ]);
    let dir = f.exe.join("ghost").join("emo2");
    assert_eq!(
        plan_copy(&roots, Some(&f.exe), StoreKind::Ghost, &dir),
        CopyPlan::AsIs
    );
}

#[test]
fn exe_root_not_listed_or_unknown_is_as_is() {
    let f = fixture("ur-plan-noexe");
    let only_user = BasewareRoots::new(vec![BasewareRoot::user_root(f.user.clone(), false)]);
    let dir = f.exe.join("ghost").join("emo2");
    assert_eq!(
        plan_copy(&only_user, Some(&f.exe), StoreKind::Ghost, &dir),
        CopyPlan::AsIs
    );
    assert_eq!(
        plan_copy(&marked(&f.user, &f.exe), None, StoreKind::Ghost, &dir),
        CopyPlan::AsIs
    );
}

/// `--ghost <exe>\<フォルダ>` で実行ファイルのフォルダと同じ綴りの一時の置き場が先頭に足されても、
/// 実行ファイルの隣の根は一時の置き場でない項目で探す（要件 3.9: ほかの決まりは変えない）。
#[test]
fn temporary_root_at_exe_dir_does_not_hide_exe_root() {
    let f = fixture("ur-plan-temp-exe");
    let roots = BasewareRoots::new(vec![
        BasewareRoot::temporary(f.exe.clone()),
        BasewareRoot::user_root(f.user.clone(), false),
        BasewareRoot::new(f.exe.clone()),
    ]);
    let ghost = f.exe.join("ghost").join("emo2");
    assert_eq!(
        plan_copy(&roots, Some(&f.exe), StoreKind::Ghost, &ghost),
        CopyPlan::Copy {
            from: ghost.clone(),
            to: f.user.join("ghost").join("emo2"),
        }
    );
    // 一時の置き場のゴースト（`<exe>\<フォルダ>`）そのものは写さない。
    let temp_ghost = f.exe.join("loose");
    assert_eq!(
        plan_copy(&roots, Some(&f.exe), StoreKind::Ghost, &temp_ghost),
        CopyPlan::AsIs
    );
}

// ---- copy_for_write ----

#[test]
fn copy_for_write_copies_then_keeps_existing_copy() {
    let f = fixture("ur-copy-keep");
    let roots = marked(&f.user, &f.exe);
    let bundled = f.exe.join("ghost").join("emo2");
    let copy = f.user.join("ghost").join("emo2");

    let got = copy_for_write(&roots, Some(&f.exe), StoreKind::Ghost, &bundled);
    assert_eq!(got, copy);
    assert_eq!(fs::read_to_string(copy.join("a.txt")).unwrap(), "bundled");

    // 同梱が新しくなっても・写しが更新されていても上書きしない。
    fs::write(bundled.join("a.txt"), "bundled v2").unwrap();
    fs::write(bundled.join("new.txt"), "new").unwrap();
    fs::write(copy.join("a.txt"), "edited").unwrap();
    let again = copy_for_write(&roots, Some(&f.exe), StoreKind::Ghost, &bundled);
    assert_eq!(again, copy);
    assert_eq!(fs::read_to_string(copy.join("a.txt")).unwrap(), "edited");
    assert!(!copy.join("new.txt").exists());
}

#[test]
fn copy_for_write_failure_returns_source_and_leaves_nothing() {
    let f = fixture("ur-copy-fail");
    let roots = marked(&f.user, &f.exe);
    let bundled = f.exe.join("ghost").join("emo2");
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(bundled.join("a.txt"))
        .unwrap();

    let got = copy_for_write(&roots, Some(&f.exe), StoreKind::Ghost, &bundled);
    drop(held);

    assert_eq!(got, bundled);
    assert!(!f.user.join("ghost").join("emo2").exists());
    let shelf = f.user.join(areka_user_data::COPY_WORK);
    let left = fs::read_dir(&shelf).map(|d| d.count()).unwrap_or(0);
    assert_eq!(left, 0);
}

#[test]
fn copy_for_write_as_is_returns_dir() {
    let f = fixture("ur-copy-asis");
    let roots = BasewareRoots::new(vec![BasewareRoot::new(f.exe.clone())]);
    let bundled = f.exe.join("ghost").join("emo2");
    assert_eq!(
        copy_for_write(&roots, Some(&f.exe), StoreKind::Ghost, &bundled),
        bundled
    );
    assert!(!f.user.join("ghost").exists());
}

// ---- listed_from ----

#[test]
fn areka_root_present_lists_no_user_root() {
    let layout = Layout {
        user_root: Some(UserRoot {
            dir: PathBuf::from(r"C:\u\areka"),
            origin: UserRootOrigin::Default,
        }),
        reason: Reason::Unwritable,
    };
    assert_eq!(listed_from(None, &layout), layout.user_root.as_ref());
    assert_eq!(listed_from(Some(OsStr::new(r"C:\r")), &layout), None);
    assert_eq!(listed_from(Some(OsStr::new("")), &layout), None);
}
