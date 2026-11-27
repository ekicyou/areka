use super::*;
use crate::key::parse_dotted;
use crate::store::{MemBackend, StoreBackend};
use crate::test_log_capture::capture;
use std::sync::Arc;
use temp_path_kit::TempPath;
use tracing::Level;

fn all_families() -> Vec<PersistKey> {
    vec![
        PersistKey::WindowPos {
            scope: 0,
            axis: Axis::X,
        },
        PersistKey::WindowPos {
            scope: 2,
            axis: Axis::Y,
        },
        PersistKey::BalloonOffset {
            scope: 0,
            axis: Axis::X,
        },
        PersistKey::BalloonOffset {
            scope: 1,
            axis: Axis::Y,
        },
        PersistKey::BootCount,
        PersistKey::VanishCount,
        PersistKey::LastGhost,
        PersistKey::LastBalloon,
        PersistKey::LastShell,
        PersistKey::LastRunning,
        PersistKey::BasewareRoots,
        PersistKey::WindowAdjacent { scope: 1 },
    ]
}

fn mem() -> (Arc<MemBackend>, Arc<dyn StoreBackend>) {
    let mem = MemBackend::new();
    let backend: Arc<dyn StoreBackend> = mem.clone();
    (mem, backend)
}

fn ghost_roots(label: &str) -> ScopeRoots {
    ScopeRoots {
        ghost: Some(PathBuf::from(format!(r"C:\偽の置き場\persist-{label}"))),
        ..ScopeRoots::default()
    }
}

fn sorted(mut entries: Vec<(PersistKey, String)>) -> Vec<(PersistKey, String)> {
    entries.sort_by_key(|(key, _)| key.to_canonical_key());
    entries
}

#[test]
fn canonical_key_round_trips_with_to_canonical_string() {
    for key in all_families() {
        let s = key.to_canonical_key();
        let back = parse_dotted(&s).unwrap().to_canonical_string();
        assert_eq!(back, s, "canonical mismatch for {key:?}");
    }
}

#[test]
fn canonical_key_exact_strings() {
    let expected = [
        "areka.window.scope(0).x",
        "areka.window.scope(2).y",
        "areka.balloon.offset.scope(0).x",
        "areka.balloon.offset.scope(1).y",
        "areka.boot.count",
        "areka.vanish.count",
        "areka.last.ghost",
        "areka.last.balloon",
        "areka.last.shell",
        "areka.last.running",
        "areka.baseware.roots",
        "areka.window.scope(1).adjacent",
    ];
    let names: Vec<String> = all_families()
        .into_iter()
        .map(PersistKey::to_canonical_key)
        .collect();
    assert_eq!(names, expected);
}

#[test]
fn canonical_name_turns_back_into_the_typed_key() {
    for key in all_families() {
        assert_eq!(
            PersistKey::from_canonical_key(&key.to_canonical_key()),
            Some(key)
        );
    }
    for name in [
        "areka.window.scope(00).x",
        "areka.window.scope(+1).x",
        "areka.window.scope(1).z",
        "areka.window.scope(x).x",
        "areka.balloon.offset.scope(1).adjacent",
        "areka.boot.count.x",
        "areka.window.scope(4294967296).x",
        "free.key",
        "",
    ] {
        assert_eq!(PersistKey::from_canonical_key(name), None, "{name}");
    }
}

#[test]
fn every_family_saves_and_loads_through_the_store() {
    let (_mem, backend) = mem();
    let roots = ghost_roots("round-trip");
    let entries: Vec<(PersistKey, String)> = all_families()
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, format!("-{i}")))
        .collect();
    assert_eq!(
        save_scope(PersistScope::Ghost, &roots, &backend, entries.clone()),
        PersistOutcome::Saved
    );
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        sorted(entries)
    );
}

#[test]
fn overwrite_keeps_other_keys() {
    let (_mem, backend) = mem();
    let roots = ghost_roots("overwrite");
    let save = |entries| save_scope(PersistScope::Ghost, &roots, &backend, entries);
    save(vec![
        (PersistKey::BootCount, "1".into()),
        (PersistKey::LastGhost, "emo2".into()),
    ]);
    save(vec![(PersistKey::BootCount, "2".into())]);
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        sorted(vec![
            (PersistKey::BootCount, "2".into()),
            (PersistKey::LastGhost, "emo2".into()),
        ])
    );
}

#[test]
fn scopes_and_roots_do_not_cross() {
    let (_mem, backend) = mem();
    let roots = ScopeRoots {
        app: Some(PathBuf::from(r"C:\偽の置き場\persist-cross-app")),
        ghost: Some(PathBuf::from(r"C:\偽の置き場\persist-cross-ghost")),
        ..ScopeRoots::default()
    };
    let other_ghost = ghost_roots("cross-other");
    save_scope(
        PersistScope::App,
        &roots,
        &backend,
        vec![(PersistKey::LastGhost, "app".into())],
    );
    save_scope(
        PersistScope::Ghost,
        &other_ghost,
        &backend,
        vec![(PersistKey::LastGhost, "other".into())],
    );
    assert!(load_scope(PersistScope::Ghost, &roots, &backend).is_empty());
    assert_eq!(
        load_scope(PersistScope::App, &roots, &backend),
        vec![(PersistKey::LastGhost, "app".into())]
    );
}

#[test]
fn absent_root_loads_empty_and_saves_no_root() {
    let (_mem, backend) = mem();
    let roots = ScopeRoots::default();
    assert!(load_scope(PersistScope::Shell, &roots, &backend).is_empty());
    let entries = vec![
        (PersistKey::LastGhost, "emo2".into()),
        (PersistKey::BootCount, "3".into()),
    ];
    assert_eq!(
        save_scope(PersistScope::Shell, &roots, &backend, entries),
        PersistOutcome::Degraded(PersistFailure {
            scope: PersistScope::Shell,
            dir: None,
            keys: vec![PersistKey::LastGhost, PersistKey::BootCount],
            cause: PersistFailureCause::NoRoot,
        })
    );
}

#[test]
fn an_os_refusal_degrades_with_dir_keys_and_reason() {
    let (mem, backend) = mem();
    let roots = ghost_roots("refusal");
    save_scope(
        PersistScope::Ghost,
        &roots,
        &backend,
        vec![(PersistKey::BootCount, "1".into())],
    );
    mem.fail_next_write();
    let mut outcome = None;
    let events = capture(|| {
        outcome = Some(save_scope(
            PersistScope::Ghost,
            &roots,
            &backend,
            vec![(PersistKey::BootCount, "2".into())],
        ));
    });
    let Some(PersistOutcome::Degraded(failure)) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(failure.dir, roots.ghost);
    assert_eq!(failure.keys, vec![PersistKey::BootCount]);
    assert!(matches!(failure.cause, PersistFailureCause::Io(_)));
    assert!(
        events.iter().any(|e| e.level == Level::ERROR
            && e.field("keys")
                .is_some_and(|k| k.contains("areka.boot.count"))),
        "書けなかった鍵の正規名が記録に載る: {events:?}"
    );
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        vec![(PersistKey::BootCount, "1".into())],
        "書けなかった値は入らない"
    );
}

#[test]
fn a_writer_lock_held_past_the_wait_degrades_and_keeps_the_old_value() {
    let (mem, backend) = mem();
    let roots = ghost_roots("lock-limit");
    let save = |value: &str| {
        save_scope(
            PersistScope::Ghost,
            &roots,
            &backend,
            vec![(PersistKey::BootCount, value.into())],
        )
    };
    assert_eq!(save("1"), PersistOutcome::Saved);
    let hold = mem.hold_writer(roots.ghost.as_ref().unwrap());
    let start = mem.now();
    let PersistOutcome::Degraded(failure) = save("2") else {
        panic!("錠を取れない書きは縮退する");
    };
    assert!(mem.now() - start > crate::store::OP_WAIT);
    assert_eq!(
        (failure.dir, failure.keys),
        (roots.ghost.clone(), vec![PersistKey::BootCount])
    );
    drop(hold);
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        vec![(PersistKey::BootCount, "1".into())]
    );
}

#[test]
fn a_refused_removal_degrades_and_keeps_the_key() {
    let (mem, backend) = mem();
    let roots = ghost_roots("remove-refusal");
    save_scope(
        PersistScope::Ghost,
        &roots,
        &backend,
        vec![(PersistKey::LastRunning, "印".into())],
    );
    mem.fail_next_write();
    let PersistOutcome::Degraded(failure) = remove_scope_keys(
        PersistScope::Ghost,
        &roots,
        &backend,
        vec![PersistKey::LastRunning],
    ) else {
        panic!("消せない消しは縮退する");
    };
    assert_eq!(failure.keys, vec![PersistKey::LastRunning]);
    assert!(matches!(failure.cause, PersistFailureCause::Io(_)));
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        vec![(PersistKey::LastRunning, "印".into())]
    );
}

#[test]
fn an_unusable_memory_file_degrades_as_unusable() {
    let (mem, backend) = mem();
    let roots = ghost_roots("unusable");
    mem.mark_unusable(roots.ghost.as_ref().unwrap());
    let outcome = save_scope(
        PersistScope::Ghost,
        &roots,
        &backend,
        vec![(PersistKey::BootCount, "1".into())],
    );
    assert!(
        matches!(
            &outcome,
            PersistOutcome::Degraded(PersistFailure {
                cause: PersistFailureCause::Unusable(_),
                ..
            })
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_legacy_file_is_read_through_the_import() {
    let (mem, backend) = mem();
    let roots = ghost_roots("legacy");
    let root = roots.ghost.clone().unwrap();
    mem.plant_legacy(
        &root,
        "format-version = 1\n[last]\nghost = \"emo2\"\nrunning = \"\"\n[window.\"0\"]\nx = \"5\"\n",
    );
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        sorted(vec![
            (PersistKey::LastGhost, "emo2".into()),
            (
                PersistKey::WindowPos {
                    scope: 0,
                    axis: Axis::X
                },
                "5".into()
            ),
        ])
    );
    assert_eq!(mem.legacy_names(&root), ["sylphya.toml.imported"]);
}

#[test]
fn last_running_empty_string_reads_as_absent() {
    let (_mem, backend) = mem();
    let roots = ghost_roots("running");
    save_scope(
        PersistScope::Ghost,
        &roots,
        &backend,
        vec![
            (PersistKey::LastRunning, String::new()),
            (PersistKey::BasewareRoots, String::new()),
        ],
    );
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        vec![(PersistKey::BasewareRoots, String::new())],
        "根の並びの空文字は空文字のまま"
    );
}

#[test]
fn removing_keys_keeps_the_others() {
    let (_mem, backend) = mem();
    let roots = ghost_roots("remove");
    let entries: Vec<(PersistKey, String)> = all_families()
        .into_iter()
        .map(|key| (key, "v".to_owned()))
        .collect();
    save_scope(PersistScope::Ghost, &roots, &backend, entries);
    let gone: Vec<PersistKey> = all_families().into_iter().step_by(2).collect();
    let kept: Vec<PersistKey> = all_families().into_iter().skip(1).step_by(2).collect();
    assert_eq!(
        remove_scope_keys(PersistScope::Ghost, &roots, &backend, gone),
        PersistOutcome::Saved
    );
    assert_eq!(
        load_scope(PersistScope::Ghost, &roots, &backend),
        sorted(kept.iter().map(|key| (*key, "v".to_owned())).collect())
    );
    assert_eq!(
        remove_scope_keys(PersistScope::Ghost, &roots, &backend, kept),
        PersistOutcome::Saved
    );
    assert!(load_scope(PersistScope::Ghost, &roots, &backend).is_empty());
}

#[test]
fn removing_from_a_layer_without_a_file_creates_nothing() {
    let tmp = TempPath::under_target("sylphya-persist-remove-none");
    let roots = ScopeRoots {
        ghost: Some(tmp.child("areka")),
        ..ScopeRoots::default()
    };
    let backend: Arc<dyn StoreBackend> = crate::store::FsBackend::with_manual_clock();
    assert_eq!(
        remove_scope_keys(
            PersistScope::Ghost,
            &roots,
            &backend,
            vec![PersistKey::LastRunning]
        ),
        PersistOutcome::Saved
    );
    assert!(!tmp.child("areka").exists());
}
