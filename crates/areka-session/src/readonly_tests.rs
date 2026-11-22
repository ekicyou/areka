//! 走りの状態の口の決定論テスト。
//!
//! 試験のプロセスでは `engage` を呼ばない（旗はプロセスに 1 つで、戻す口が無い）。
//! そのことは本番の源の走査（構造の試験）で固める。

use std::path::Path;

use super::*;
use crate::boot_resolve::boot_resolve_tests::count_event;

#[test]
fn flag_is_off_and_install_adds_nothing_in_the_test_process() {
    assert!(!is_on(), "試験のプロセスでは旗が立たない");
    let mut world = World::new();
    install(&mut world);
    assert!(
        !world.contains_resource::<ReadOnlyRun>(),
        "旗が偽なら印を足さない"
    );
    assert!(!of(&world));
}

#[test]
fn of_follows_the_resource() {
    let mut world = World::new();
    assert!(!of(&world), "印が無ければ偽");
    world.insert_resource(ReadOnlyRun);
    assert!(of(&world), "印が在れば真");
}

/// 置き場の木（下の階層まで、相対パスと中身）。
pub(crate) fn tree(dir: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        for e in std::fs::read_dir(dir).expect("木が読めない") {
            let p = e.expect("木の項目").path();
            let content = if p.is_dir() {
                walk(root, &p, out);
                Vec::new()
            } else {
                std::fs::read(&p).expect("ファイルが読めない")
            };
            out.push((p.strip_prefix(root).unwrap().to_path_buf(), content));
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// アプリもゴーストも書けない置き場（フォルダの在るべき所に普通のファイル）。根は `target\` の下。
pub(crate) fn unwritable_places(
    tmp: &temp_path_kit::TempPath,
) -> (crate::boot_config::ConfigInputs, std::path::PathBuf) {
    let app = tmp.child("app-profile");
    std::fs::write(&app, "x").unwrap();
    let root = tmp.child("ghost-a");
    std::fs::create_dir_all(root.join("ghost").join("master")).unwrap();
    std::fs::write(root.join("ghost").join("master").join("profile"), "x").unwrap();
    let cfg = crate::boot_config::ConfigInputs {
        ghost_root: root,
        balloon_root: tmp.child("balloon"),
    };
    (cfg, app)
}

/// 書けない置き場でも、読み取りだけなら通り、記録が 1 件出て、木が変わらない。
#[test]
fn boot_check_passes_readonly_without_touching_the_tree() {
    let tmp = temp_path_kit::TempPath::under_target("readonly-boot-check-skip");
    let (cfg, app) = unwritable_places(&tmp);
    let before = tree(tmp.path());

    let (result, events) =
        log_capture_kit::capture(|| crate::write_failure::check_boot(true, &cfg, &app));

    assert!(result.is_ok(), "{result:?}");
    let skipped = count_event(&events, "readonly_boot_check_skipped");
    assert_eq!(skipped, 1, "{events:?}");
    assert_eq!(tree(tmp.path()), before, "確かめが木を変えた");
}

/// 読み取りだけでなければ、通常の確かめと同じ失敗を返し、飛ばした記録は出ない。
#[test]
fn boot_check_without_readonly_fails_as_the_plain_check_does() {
    let tmp = temp_path_kit::TempPath::under_target("readonly-boot-check-normal");
    let (cfg, app) = unwritable_places(&tmp);

    let expected = crate::write_failure::check_boot_dirs(&cfg, &app).expect_err("書けない");
    let (result, events) =
        log_capture_kit::capture(|| crate::write_failure::check_boot(false, &cfg, &app));

    assert_eq!(result.expect_err("通常の確かめと同じく止まる"), expected);
    assert!(
        events
            .iter()
            .all(|e| e.field_str("event") != Some("readonly_boot_check_skipped")),
        "{events:?}"
    );
}

/// 写すはずの並びでも、読み取りだけなら元を返し、記録が 1 件出て、写し先が作られない。
#[test]
fn copy_is_withheld_and_the_original_is_used() {
    use crate::roots::user_root::{CopyPlan, copy_unless_readonly, plan_copy};
    use areka_ghost::{BasewareRoot, BasewareRoots, StoreKind};

    let tmp = temp_path_kit::TempPath::under_target("readonly-copy-withheld");
    let user = tmp.child("user");
    let exe = tmp.child("exe");
    std::fs::create_dir_all(&user).unwrap();
    let bundled = exe.join("ghost").join("emo2");
    std::fs::create_dir_all(&bundled).unwrap();
    std::fs::write(bundled.join("a.txt"), "bundled").unwrap();
    let roots = BasewareRoots::new(vec![
        BasewareRoot::user_root(user.clone(), false),
        BasewareRoot::new(exe.clone()),
    ]);
    // 較正: この並びは写すはずの並び。
    assert!(matches!(
        plan_copy(&roots, Some(&exe), StoreKind::Ghost, &bundled),
        CopyPlan::Copy { .. }
    ));
    let before = tree(tmp.path());

    let (got, events) = log_capture_kit::capture(|| {
        copy_unless_readonly(true, &roots, Some(&exe), StoreKind::Ghost, &bundled)
    });

    assert_eq!(got, bundled, "元のフォルダを返す");
    let withheld = count_event(&events, "user_data_copy_withheld");
    assert_eq!(withheld, 1, "{events:?}");
    assert!(!user.join("ghost").exists(), "写し先が作られた");
    assert_eq!(tree(tmp.path()), before, "木が変わった");
}
