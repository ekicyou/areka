//! 置き場の追加・外す・先頭へ動かすの判断の決定論テスト（要件 1.5・6.4・6.6〜6.8・7.2・7.3・8.6・12.5、
//! areka-P0-user-data-root 要件 3.7・10.3）。
//!
//! 実在する根は `TempPath::under_target`（ワークツリーの `target\` の下）に掘る。

use super::*;
use crate::roots::{EXE_TOKEN, encode_memory};
use areka_ghost::{BasewareRoot, BasewareRoots};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// `target\` の下に実在する根を掘る（`ghost\` を持たせるかどうか）。
fn root(tmp: &TempPath, name: &str, with_ghost: bool) -> PathBuf {
    let dir = tmp.path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    if with_ghost {
        std::fs::create_dir_all(dir.join("ghost")).unwrap();
    }
    dir
}

fn roots_of(dirs: &[&Path]) -> BasewareRoots {
    BasewareRoots::new(
        dirs.iter()
            .map(|d| BasewareRoot::new(d.to_path_buf()))
            .collect(),
    )
}

#[test]
fn add_appends_at_end() {
    let tmp = TempPath::under_target("roots-edit-add-append");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let v = decide_add(&roots_of(&[&a]), RootsOrigin::Memory, &b);
    assert_eq!(
        v,
        AddVerdict::Added {
            roots: roots_of(&[&a, &b]),
            empty: false
        }
    );
}

#[test]
fn add_already_listed_is_unchanged() {
    let tmp = TempPath::under_target("roots-edit-add-listed");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    // 大文字小文字と末尾の区切りだけ違う綴りも同じ根（same_root）。
    let picked = PathBuf::from(format!("{}\\", b.to_string_lossy().to_uppercase()));
    let v = decide_add(&roots_of(&[&a, &b]), RootsOrigin::Memory, &picked);
    assert_eq!(v, AddVerdict::AlreadyListed { existing: b });
}

#[test]
fn add_empty_root_is_added_and_marked() {
    let tmp = TempPath::under_target("roots-edit-add-empty");
    let a = root(&tmp, "a", true);
    let e = root(&tmp, "e", false);
    let v = decide_add(&roots_of(&[&a]), RootsOrigin::Memory, &e);
    assert_eq!(
        v,
        AddVerdict::Added {
            roots: roots_of(&[&a, &e]),
            empty: true
        }
    );
    // balloon\ だけでも空ではない。
    let bl = root(&tmp, "bl", false);
    std::fs::create_dir_all(bl.join("balloon")).unwrap();
    assert!(matches!(
        decide_add(&roots_of(&[&a]), RootsOrigin::Memory, &bl),
        AddVerdict::Added { empty: false, .. }
    ));
}

#[test]
fn add_exe_dir_enters_as_absolute_and_memory_writes_exe_token() {
    let tmp = TempPath::under_target("roots-edit-add-exe");
    let exe = root(&tmp, "exe", true);
    let other = root(&tmp, "other", true);
    // 既定でない並び（記憶の並びから実行ファイルの隣を外した後）へ実行ファイルのフォルダを足す。
    let v = decide_add(&roots_of(&[&other]), RootsOrigin::Memory, &exe);
    let AddVerdict::Added { roots, empty } = v else {
        panic!("足せるはず: {v:?}");
    };
    assert!(!empty);
    assert_eq!(roots, roots_of(&[&other, &exe]));
    assert!(roots.roots()[1].dir().is_absolute());
    assert_eq!(
        encode_memory(&roots, Some(&exe)),
        format!("{}|{EXE_TOKEN}", other.to_string_lossy())
    );
}

#[test]
fn add_env_var_is_locked() {
    let tmp = TempPath::under_target("roots-edit-add-env");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    assert_eq!(
        decide_add(&roots_of(&[&a]), RootsOrigin::EnvVar, &b),
        AddVerdict::EnvLocked
    );
}

#[test]
fn remove_takes_out_and_keeps_folder() {
    let tmp = TempPath::under_target("roots-edit-remove");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let c = root(&tmp, "c", true);
    let picked = PathBuf::from(b.to_string_lossy().to_uppercase());
    let v = decide_remove(&roots_of(&[&a, &b, &c]), RootsOrigin::Memory, &picked);
    assert_eq!(
        v,
        RemoveVerdict::Removed {
            roots: roots_of(&[&a, &c]),
            removed: b.clone()
        }
    );
    // フォルダは消さず動かさない（要件 7.2）。
    assert!(b.join("ghost").is_dir());
}

#[test]
fn remove_last_one_is_refused() {
    let tmp = TempPath::under_target("roots-edit-remove-last");
    let a = root(&tmp, "a", true);
    assert_eq!(
        decide_remove(&roots_of(&[&a]), RootsOrigin::Default, &a),
        RemoveVerdict::LastOne
    );
}

#[test]
fn remove_env_var_is_locked() {
    let tmp = TempPath::under_target("roots-edit-remove-env");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    assert_eq!(
        decide_remove(&roots_of(&[&a, &b]), RootsOrigin::EnvVar, &b),
        RemoveVerdict::EnvLocked
    );
}

#[test]
fn remove_not_listed_is_stale() {
    let tmp = TempPath::under_target("roots-edit-remove-stale");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let gone = root(&tmp, "gone", true);
    assert_eq!(
        decide_remove(&roots_of(&[&a, &b]), RootsOrigin::Memory, &gone),
        RemoveVerdict::Stale
    );
}

#[test]
fn front_moves_to_front_keeping_others_order() {
    let tmp = TempPath::under_target("roots-edit-front-move");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let c = root(&tmp, "c", true);
    let picked = PathBuf::from(c.to_string_lossy().to_uppercase());
    assert_eq!(
        decide_front(&roots_of(&[&a, &b, &c]), RootsOrigin::Memory, &picked),
        FrontVerdict::Moved {
            roots: roots_of(&[&c, &a, &b])
        }
    );
    // 真ん中の根でもほかの順は保つ。
    assert_eq!(
        decide_front(&roots_of(&[&a, &b, &c]), RootsOrigin::Default, &b),
        FrontVerdict::Moved {
            roots: roots_of(&[&b, &a, &c])
        }
    );
}

#[test]
fn front_already_first_is_unchanged() {
    let tmp = TempPath::under_target("roots-edit-front-first");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    assert_eq!(
        decide_front(&roots_of(&[&a, &b]), RootsOrigin::Memory, &a),
        FrontVerdict::AlreadyFront
    );
}

#[test]
fn front_env_var_is_locked() {
    let tmp = TempPath::under_target("roots-edit-front-env");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    assert_eq!(
        decide_front(&roots_of(&[&a, &b]), RootsOrigin::EnvVar, &b),
        FrontVerdict::EnvLocked
    );
}

#[test]
fn front_not_listed_is_stale() {
    let tmp = TempPath::under_target("roots-edit-front-stale");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    let gone = root(&tmp, "gone", true);
    assert_eq!(
        decide_front(&roots_of(&[&a, &b]), RootsOrigin::Memory, &gone),
        FrontVerdict::Stale
    );
}

/// 書ける根の印は足す・外す・先頭へ動かすの並びの複写で残る（areka-P0-user-data-root 10.3）。
#[test]
fn user_root_mark_survives_add_remove_front() {
    use areka_ghost::RootMark;
    let tmp = TempPath::under_target("roots-edit-mark-survives");
    let user = root(&tmp, "user", true);
    let exe = root(&tmp, "exe", true);
    let extra = root(&tmp, "extra", true);
    let marked = BasewareRoots::new(vec![
        BasewareRoot::user_root(user.clone(), false),
        BasewareRoot::new(exe.clone()),
    ]);
    let is_marked = |roots: &BasewareRoots| {
        roots
            .roots()
            .iter()
            .find(|r| r.dir() == user)
            .map(|r| r.mark())
            == Some(RootMark::UserRoot { from_env: false })
    };

    let AddVerdict::Added { roots: added, .. } = decide_add(&marked, RootsOrigin::Memory, &extra)
    else {
        panic!("足せるはず");
    };
    assert!(is_marked(&added), "足した後も印が残る");

    let RemoveVerdict::Removed { roots: removed, .. } =
        decide_remove(&added, RootsOrigin::Memory, &exe)
    else {
        panic!("外せるはず");
    };
    assert!(is_marked(&removed), "外した後も印が残る");

    let FrontVerdict::Moved { roots: fronted } =
        decide_front(&removed, RootsOrigin::Memory, &extra)
    else {
        panic!("先頭へ動かせるはず");
    };
    assert_eq!(fronted.roots()[0].dir(), extra);
    assert!(is_marked(&fronted), "先頭へ動かした後も印が残る");
}

// ---------------------------------------------------------------- 適用（実行系が居ない経路・要件 8.1〜8.5）

mod apply {
    use super::*;
    use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
    use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
    use crate::roots::{decode_memory, read_memory};
    use bevy_ecs::world::World;
    use log_capture_kit::{CapturedEvent, capture};

    /// 根の下に読めるゴースト 1 体を置く。
    fn ghost(root: &Path, folder: &str) {
        let master = root.join("ghost").join(folder).join("ghost").join("master");
        std::fs::create_dir_all(&master).unwrap();
        std::fs::write(
            master.join("descript.txt"),
            format!("charset,UTF-8\r\nname,{folder}\r\n"),
        )
        .unwrap();
    }

    fn world(root: &Path, app_profile_dir: PathBuf) -> World {
        let mut world = World::new();
        world.insert_resource(BootContext {
            roots: roots_of(&[root]),
            roots_origin: RootsOrigin::Default,
            app_profile_dir,
            helper_exe: root.join("helper.exe"),
            argv_session: false,
            current: CurrentGhost {
                cfg: ConfigInputs {
                    ghost_root: root.join(r"ghost\ga"),
                    balloon_root: root.join(r"balloon\B"),
                },
                ghost: GhostDecision {
                    route: GhostRoute::Default,
                    dir: root.join(r"ghost\ga"),
                    folder: Some("ga".to_owned()),
                },
                balloon: BalloonDecision {
                    route: BalloonRoute::Default,
                    dir: root.join(r"balloon\B"),
                    folder: Some("B".to_owned()),
                },
            },
        });
        world
    }

    fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
        events
            .iter()
            .filter(|e| e.field_str("event") == Some(name))
            .collect()
    }

    #[test]
    fn add_writes_memory_and_takes_effect_without_switching() {
        let tmp = TempPath::under_target("roots-edit-apply-add");
        let a = root(&tmp, "a", true);
        ghost(&a, "ga");
        let b = root(&tmp, "b", true);
        ghost(&b, "gb");
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());

        let ((), events) = capture(|| add_root(&mut world, b.clone()));

        let ctx = world.resource::<BootContext>();
        assert_eq!(ctx.roots, roots_of(&[&a, &b]), "その回に効く（要件 8.2）");
        assert_eq!(ctx.roots_origin, RootsOrigin::Memory);
        assert_eq!(
            ctx.current.ghost.folder.as_deref(),
            Some("ga"),
            "切り替えない（要件 8.4）"
        );
        // 記憶を直に読む口で読み戻すと同じ並び（要件 8.1）。
        let value = read_memory(&profile).expect("記憶に並びが書かれる");
        let (decoded, skipped) = decode_memory(&value, super::super::exe_dir().as_deref());
        assert_eq!(decoded, vec![a.clone(), b.clone()]);
        assert!(skipped.is_empty());
        // 次に引く和に足した根のゴーストが入る。
        let survey = areka_ghost::catalog_roots::survey(&ctx.roots);
        assert!(survey.ghosts.iter().any(|g| g.identity.folder == "gb"));
        // 変更のときの並びと隠れの記録（要件 8.3）。
        let listed = named(&events, "root_listed");
        assert_eq!(listed.len(), 2);
        assert!(
            listed
                .iter()
                .all(|e| e.field_str("occasion") == Some("change"))
        );
        assert_eq!(named(&events, "roots_hidden_count").len(), 1);
        assert!(named(&events, "roots_memory_write_failed").is_empty());
    }

    #[test]
    fn add_takes_effect_and_logs_error_when_memory_unwritable() {
        let tmp = TempPath::under_target("roots-edit-apply-unwritable");
        let a = root(&tmp, "a", true);
        let b = root(&tmp, "b", true);
        // 記憶の置き場がファイル＝書けない。
        let profile = tmp.path().join("profile-is-a-file");
        std::fs::write(&profile, "x").unwrap();
        let mut world = world(&a, profile);

        let ((), events) = capture(|| add_root(&mut world, b.clone()));

        assert_eq!(world.resource::<BootContext>().roots, roots_of(&[&a, &b]));
        let failed = named(&events, "roots_memory_write_failed");
        assert_eq!(failed.len(), 1, "{events:?}");
        assert_eq!(failed[0].level, tracing::Level::ERROR);
    }

    #[test]
    fn remove_writes_memory_and_keeps_folder() {
        let tmp = TempPath::under_target("roots-edit-apply-remove");
        let a = root(&tmp, "a", true);
        let b = root(&tmp, "b", true);
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());
        world.resource_mut::<BootContext>().roots = roots_of(&[&a, &b]);

        remove_root(&mut world, b.clone());

        assert_eq!(world.resource::<BootContext>().roots, roots_of(&[&a]));
        let value = read_memory(&profile).expect("記憶に並びが書かれる");
        let (decoded, _) = decode_memory(&value, super::super::exe_dir().as_deref());
        assert_eq!(decoded, vec![a.clone()]);
        assert!(b.is_dir(), "フォルダは消さない（要件 7.2）");
    }

    #[test]
    fn refusals_warn_and_never_write_memory() {
        let tmp = TempPath::under_target("roots-edit-apply-refuse");
        let a = root(&tmp, "a", true);
        let empty = root(&tmp, "empty", false);
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());

        let ((), events) = capture(|| {
            add_root(&mut world, a.clone());
            remove_root(&mut world, a.clone());
        });
        assert_eq!(named(&events, "roots_add_already_listed").len(), 1);
        let refused = named(&events, "roots_remove_refused");
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].field_str("reason"), Some("last_one"));
        assert!(
            read_memory(&profile).is_none(),
            "既定の並びのままなら書かない（要件 1.11）"
        );

        let ((), events) = capture(|| add_root(&mut world, empty.clone()));
        let warned = named(&events, "roots_add_empty_root");
        assert_eq!(warned.len(), 1);
        assert_eq!(warned[0].level, tracing::Level::WARN);
        assert_eq!(
            world.resource::<BootContext>().roots,
            roots_of(&[&a, &empty])
        );
    }

    #[test]
    fn front_writes_memory_and_takes_effect_without_switching() {
        let tmp = TempPath::under_target("roots-edit-apply-front");
        let a = root(&tmp, "a", true);
        ghost(&a, "ga");
        let b = root(&tmp, "b", true);
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());
        world.resource_mut::<BootContext>().roots = roots_of(&[&a, &b]);

        let ((), events) = capture(|| move_to_front(&mut world, b.clone()));

        let ctx = world.resource::<BootContext>();
        assert_eq!(ctx.roots, roots_of(&[&b, &a]), "その回に効く（要件 3.7）");
        assert_eq!(ctx.roots_origin, RootsOrigin::Memory);
        assert_eq!(
            ctx.current.ghost.folder.as_deref(),
            Some("ga"),
            "切り替えない"
        );
        assert_eq!(ctx.current.ghost.dir, a.join(r"ghost\ga"));
        let value = read_memory(&profile).expect("記憶に並びが書かれる");
        let (decoded, _) = decode_memory(&value, super::super::exe_dir().as_deref());
        assert_eq!(decoded, vec![b.clone(), a.clone()]);
        let moved = named(&events, "roots_moved_to_front");
        assert_eq!(moved.len(), 1, "{events:?}");
        assert_eq!(moved[0].level, tracing::Level::INFO);
        assert_eq!(named(&events, "root_listed").len(), 2);
    }

    #[test]
    fn front_refusals_warn_and_never_write_memory() {
        let tmp = TempPath::under_target("roots-edit-apply-front-refuse");
        let a = root(&tmp, "a", true);
        let b = root(&tmp, "b", true);
        let gone = root(&tmp, "gone", true);
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());
        world.resource_mut::<BootContext>().roots = roots_of(&[&a, &b]);

        let ((), events) = capture(|| {
            move_to_front(&mut world, a.clone());
            move_to_front(&mut world, gone.clone());
        });
        world.resource_mut::<BootContext>().roots_origin = RootsOrigin::EnvVar;
        let ((), env_events) = capture(|| move_to_front(&mut world, b.clone()));

        let reasons: Vec<_> = named(&events, "roots_front_refused")
            .iter()
            .chain(named(&env_events, "roots_front_refused").iter())
            .map(|e| {
                assert_eq!(e.level, tracing::Level::WARN);
                e.field_str("reason").map(str::to_owned)
            })
            .collect();
        assert_eq!(
            reasons,
            vec![
                Some("already_front".to_owned()),
                Some("stale".to_owned()),
                Some("env_var".to_owned())
            ]
        );
        assert_eq!(world.resource::<BootContext>().roots, roots_of(&[&a, &b]));
        assert!(read_memory(&profile).is_none(), "変えないなら書かない");
    }
}
