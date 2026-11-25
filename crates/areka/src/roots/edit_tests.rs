//! 置き場の追加・外す・先頭へ動かすの判断の決定論テスト。
//!
//! 実在する根は `TempPath::under_target`（ワークツリーの `target\` の下）に掘る。

use super::*;
use crate::roots::roots_test_support::{root_with_ghost, roots_of};
use areka_ghost::{BasewareRoot, BasewareRoots};
use areka_session::roots::{EXE_TOKEN, encode_memory};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// `target\` の下に実在する根を掘る（`ghost\` を持たせるかどうか）。
fn root(tmp: &TempPath, name: &str, with_ghost: bool) -> PathBuf {
    if with_ghost {
        return root_with_ghost(tmp.path(), name);
    }
    let dir = tmp.path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
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
    // フォルダは消さず動かさない。
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
        decide_front(&roots_of(&[&a, &b, &c]), RootsOrigin::Memory, &picked, None),
        FrontVerdict::Moved {
            roots: roots_of(&[&c, &a, &b])
        }
    );
    // 真ん中の根でもほかの順は保つ。
    assert_eq!(
        decide_front(&roots_of(&[&a, &b, &c]), RootsOrigin::Default, &b, None),
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
        decide_front(&roots_of(&[&a, &b]), RootsOrigin::Memory, &a, None),
        FrontVerdict::AlreadyFront
    );
}

#[test]
fn front_env_var_is_locked() {
    let tmp = TempPath::under_target("roots-edit-front-env");
    let a = root(&tmp, "a", true);
    let b = root(&tmp, "b", true);
    assert_eq!(
        decide_front(&roots_of(&[&a, &b]), RootsOrigin::EnvVar, &b, None),
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
        decide_front(&roots_of(&[&a, &b]), RootsOrigin::Memory, &gone, None),
        FrontVerdict::Stale
    );
}

#[test]
fn remove_user_root_is_refused() {
    let tmp = TempPath::under_target("roots-edit-remove-user-root");
    let user = root(&tmp, "user", true);
    let other = root(&tmp, "other", true);
    let roots = BasewareRoots::new(vec![
        BasewareRoot::user_root(user.clone(), false),
        BasewareRoot::new(other.clone()),
    ]);
    assert_eq!(
        decide_remove(&roots, RootsOrigin::Memory, &user),
        RemoveVerdict::UserRoot
    );
}

#[test]
fn front_exe_side_is_refused_only_while_marked() {
    let tmp = TempPath::under_target("roots-edit-front-exe-side");
    let user = root(&tmp, "user", true);
    let exe = root(&tmp, "exe", true);
    let marked = BasewareRoots::new(vec![
        BasewareRoot::user_root(user.clone(), false),
        BasewareRoot::new(exe.clone()),
    ]);
    assert_eq!(
        decide_front(&marked, RootsOrigin::Memory, &exe, Some(&exe)),
        FrontVerdict::ExeSide
    );
    assert_eq!(
        decide_front(
            &roots_of(&[&user, &exe]),
            RootsOrigin::Memory,
            &exe,
            Some(&exe)
        ),
        FrontVerdict::Moved {
            roots: roots_of(&[&exe, &user])
        }
    );
}

/// 書ける根の印は足す・外す・先頭へ動かすの並びの複写で残る。
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
        decide_front(&removed, RootsOrigin::Memory, &extra, None)
    else {
        panic!("先頭へ動かせるはず");
    };
    assert_eq!(fronted.roots()[0].dir(), extra);
    assert!(is_marked(&fronted), "先頭へ動かした後も印が残る");
}

// ---------------------------------------------------------------- 適用（実行系が居ない経路）

mod apply {
    use super::*;
    use crate::roots::roots_test_support::world;
    use crate::session_tests::{events_named, put_ghost};
    use areka_runtime::spine::{Progress, wait_until};
    use areka_session::boot_config::BootContext;
    use areka_session::roots::{decode_memory, read_memory};
    use log_capture_kit::capture;
    use std::sync::{Arc, Mutex};

    /// 根の下に読めるゴースト 1 体を置く。
    fn ghost(root: &Path, folder: &str) {
        put_ghost(&root.join("ghost").join(folder));
    }

    /// 1 件の記録と、それを出したスレッドの名。
    #[derive(Debug)]
    struct Record {
        thread: Option<String>,
        level: tracing::Level,
        fields: Vec<(String, String)>,
    }

    impl Record {
        fn field(&self, name: &str) -> Option<&str> {
            self.fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
        }
    }

    struct Fields<'a>(&'a mut Vec<(String, String)>);

    impl tracing::field::Visit for Fields<'_> {
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.0.push((field.name().to_owned(), value.to_owned()));
        }
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0.push((field.name().to_owned(), format!("{value:?}")));
        }
    }

    /// 記録ごとに出したスレッドの名を残す受け手（`capture` の記録はスレッドの名を持たない）。
    struct ByThread(Arc<Mutex<Vec<Record>>>);

    impl tracing::Subscriber for ByThread {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            let mut fields = Vec::new();
            event.record(&mut Fields(&mut fields));
            self.0.lock().unwrap().push(Record {
                thread: std::thread::current().name().map(str::to_owned),
                level: *event.metadata().level(),
                fields,
            });
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    const CHANGE_EVENTS: [&str; 3] = ["root_listed", "root_hidden_item", "roots_hidden_count"];

    /// `f` を [`ByThread`] の下で呼び、呼んだスレッドの記録と `roots-log` の記録を返す。
    /// 呼んだスレッドに並びと隠れの記録が無いことを先に見て、`roots-log` の `roots_hidden_count` を待つ。
    fn by_thread(f: impl FnOnce()) -> (Vec<Record>, Vec<Record>) {
        let records = Arc::new(Mutex::new(Vec::new()));
        log_capture_kit::ensure_interest_probes();
        tracing::subscriber::with_default(ByThread(Arc::clone(&records)), || {
            tracing::callsite::rebuild_interest_cache();
            f();
        });
        let me = std::thread::current().name().map(str::to_owned);
        let caller: Vec<Record> = {
            let mut all = records.lock().unwrap();
            let (mine, rest) = all.drain(..).partition(|r: &Record| r.thread == me);
            *all = rest;
            mine
        };
        let on_caller: Vec<_> = caller
            .iter()
            .filter(|r| r.field("event").is_some_and(|e| CHANGE_EVENTS.contains(&e)))
            .collect();
        assert!(
            on_caller.is_empty(),
            "呼んだスレッドで根を見に行った: {on_caller:?}"
        );
        wait_until("roots-log の隠れの件数", Progress::Unknown, || {
            records.lock().unwrap().iter().any(|r| {
                r.thread.as_deref() == Some("roots-log")
                    && r.field("event") == Some("roots_hidden_count")
            })
        })
        .expect("roots-log が並びと隠れを記録する");
        let logged = std::mem::take(&mut *records.lock().unwrap())
            .into_iter()
            .filter(|r| r.thread.as_deref() == Some("roots-log"))
            .collect();
        (caller, logged)
    }

    fn with_event<'a>(records: &'a [Record], name: &str) -> Vec<&'a Record> {
        records
            .iter()
            .filter(|r| r.field("event") == Some(name))
            .collect()
    }

    #[test]
    fn add_writes_memory_and_takes_effect_without_switching() {
        let tmp = TempPath::under_target("roots-edit-apply-add");
        let a = root(&tmp, "a", true);
        ghost(&a, "A");
        let b = root(&tmp, "b", true);
        ghost(&b, "gb");
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());
        let before = world.resource::<BootContext>().current.ghost.folder.clone();

        let (caller, logged) = by_thread(|| add_root(&mut world, b.clone()));

        let ctx = world.resource::<BootContext>();
        assert_eq!(ctx.roots, roots_of(&[&a, &b]), "その回に効く");
        assert_eq!(ctx.roots_origin, RootsOrigin::Memory);
        assert_eq!(ctx.current.ghost.folder, before, "切り替えない");
        // 記憶を直に読む口で読み戻すと同じ並び。
        let value = read_memory(&profile).expect("記憶に並びが書かれる");
        let (decoded, skipped) = decode_memory(
            &value,
            areka_session::roots::user_root::exe_dir().as_deref(),
        );
        assert_eq!(decoded, vec![a.clone(), b.clone()]);
        assert!(skipped.is_empty());
        // 次に引く和に足した根のゴーストが入る。
        let survey = areka_ghost::catalog_roots::survey(&ctx.roots);
        assert!(survey.ghosts.iter().any(|g| g.identity.folder == "gb"));
        // 変更のときの並びと隠れの記録。
        let listed = with_event(&logged, "root_listed");
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|r| r.field("occasion") == Some("change")));
        assert_eq!(with_event(&logged, "roots_hidden_count").len(), 1);
        assert!(with_event(&caller, "roots_memory_write_failed").is_empty());
    }

    #[test]
    fn remove_writes_memory_and_keeps_folder() {
        let tmp = TempPath::under_target("roots-edit-apply-remove");
        let a = root(&tmp, "a", true);
        let b = root(&tmp, "b", true);
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());
        world.resource_mut::<BootContext>().roots = roots_of(&[&a, &b]);

        let ((), events) = capture(|| remove_root(&mut world, b.clone()));

        let removed = events_named(&events, "roots_removed");
        assert_eq!(removed.len(), 1, "{events:?}");
        assert_eq!(removed[0].level, tracing::Level::INFO);
        assert_eq!(
            removed[0].field("path"),
            Some(b.display().to_string().as_str())
        );
        assert_eq!(world.resource::<BootContext>().roots, roots_of(&[&a]));
        let value = read_memory(&profile).expect("記憶に並びが書かれる");
        let (decoded, _) = decode_memory(
            &value,
            areka_session::roots::user_root::exe_dir().as_deref(),
        );
        assert_eq!(decoded, vec![a.clone()]);
        assert!(b.is_dir(), "フォルダは消さない");
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
        assert_eq!(events_named(&events, "roots_add_already_listed").len(), 1);
        let refused = events_named(&events, "roots_remove_refused");
        assert_eq!(refused.len(), 1);
        assert_eq!(refused[0].field_str("reason"), Some("last_one"));
        assert!(
            read_memory(&profile).is_none(),
            "既定の並びのままなら書かない"
        );

        let ((), events) = capture(|| add_root(&mut world, empty.clone()));
        let warned = events_named(&events, "roots_add_empty_root");
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
        ghost(&a, "A");
        let b = root(&tmp, "b", true);
        let profile = tmp.path().join("profile");
        let mut world = world(&a, profile.clone());
        world.resource_mut::<BootContext>().roots = roots_of(&[&a, &b]);
        let before = world.resource::<BootContext>().current.ghost.clone();

        let (caller, logged) = by_thread(|| move_to_front(&mut world, b.clone()));

        let ctx = world.resource::<BootContext>();
        assert_eq!(ctx.roots, roots_of(&[&b, &a]), "その回に効く");
        assert_eq!(ctx.roots_origin, RootsOrigin::Memory);
        assert_eq!(ctx.current.ghost.folder, before.folder, "切り替えない");
        assert_eq!(ctx.current.ghost.dir, before.dir);
        let value = read_memory(&profile).expect("記憶に並びが書かれる");
        let (decoded, _) = decode_memory(
            &value,
            areka_session::roots::user_root::exe_dir().as_deref(),
        );
        assert_eq!(decoded, vec![b.clone(), a.clone()]);
        let moved = with_event(&caller, "roots_moved_to_front");
        assert_eq!(moved.len(), 1, "{caller:?}");
        assert_eq!(moved[0].level, tracing::Level::INFO);
        assert_eq!(with_event(&logged, "root_listed").len(), 2);
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

        let reasons: Vec<_> = events_named(&events, "roots_front_refused")
            .iter()
            .chain(events_named(&env_events, "roots_front_refused").iter())
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
