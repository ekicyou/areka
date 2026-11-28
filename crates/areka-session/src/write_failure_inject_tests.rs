//! 書けない場面が致命の机へ届くことを、偽物の置き場の口の注ぎ込みで確かめる。
//!
//! 番人（sylphya のアクター）の受け口に致命の机の送り手を渡し、場面を 1 つ起こして
//! 汲み出すと、出所が `MemoryWriteFailed` になり `memory_write_failed` が 1 件残る。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use areka_sylphya::store::MemBackend;
use areka_sylphya::{
    AskerId, PersistKey, ScopeRoots, SylphyaInit, SylphyaParts, spawn_sylphya_with_failures,
};
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use super::tests::installed_world;
use super::*;
use crate::app_exit::FirstExit;
use crate::boot_resolve::boot_resolve_tests::events_named;

/// 偽の置き場（`MemBackend` はディスクに触らない）。
fn ghost_dir() -> PathBuf {
    PathBuf::from(r"C:\fake\ghost\master\profile\areka")
}

fn ghost_roots() -> ScopeRoots {
    ScopeRoots {
        ghost: Some(ghost_dir()),
        ..ScopeRoots::default()
    }
}

fn spawn(world: &World, roots: ScopeRoots, backend: &Arc<MemBackend>) -> SylphyaParts {
    spawn_sylphya_with_failures(SylphyaInit::new(roots, backend.clone()), sender(world))
}

fn put_boot_count(parts: &SylphyaParts) {
    parts.publisher.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "2".into())],
    );
}

/// 書きを片づけてから番人を止め、致命の机を汲み出す。返すのは最初の出所の失敗と記録の数。
fn settle_and_drain(world: &mut World, parts: SylphyaParts) -> (Option<PersistFailure>, usize) {
    parts
        .publisher
        .barrier_within(std::time::Duration::from_secs(5))
        .expect("書きが片づく");
    parts.publisher.close();
    parts.handle.join().expect("join");
    let (_, events) = capture(|| drain_system(world));
    let first = world
        .get_resource::<FirstExit>()
        .map(|first| match &first.0 {
            ExitOrigin::MemoryWriteFailed(failure) => failure.clone(),
            other => panic!("出所が記憶の失敗でない: {other:?}"),
        });
    (first, events_named(&events, "memory_write_failed").len())
}

fn assert_one_ghost_failure(
    got: (Option<PersistFailure>, usize),
    dir: Option<&Path>,
) -> PersistFailureCause {
    let (failure, logs) = got;
    let failure = failure.expect("致命の机へ届く");
    assert_eq!(logs, 1, "memory_write_failed はちょうど 1 件");
    assert_eq!(failure.scope, PersistScope::Ghost);
    assert_eq!(failure.dir.as_deref(), dir);
    assert_eq!(failure.keys, vec![PersistKey::BootCount]);
    failure.cause
}

#[test]
fn no_root_reaches_the_fatal_desk_once() {
    let mut world = installed_world();
    let backend = MemBackend::new();
    let parts = spawn(&world, ScopeRoots::default(), &backend);

    put_boot_count(&parts);

    let cause = assert_one_ghost_failure(settle_and_drain(&mut world, parts), None);
    assert_eq!(cause, PersistFailureCause::NoRoot);
}

#[test]
fn os_refusal_reaches_the_fatal_desk_once() {
    let mut world = installed_world();
    let backend = MemBackend::new();
    let parts = spawn(&world, ghost_roots(), &backend);

    backend.fail_next_write();
    put_boot_count(&parts);

    let cause = assert_one_ghost_failure(settle_and_drain(&mut world, parts), Some(&ghost_dir()));
    assert!(matches!(cause, PersistFailureCause::Io(_)), "{cause:?}");
}

#[test]
fn wait_limit_reaches_the_fatal_desk_once() {
    let mut world = installed_world();
    let backend = MemBackend::new();
    let hold = backend.hold_writer(&ghost_dir());
    let parts = spawn(&world, ghost_roots(), &backend);

    put_boot_count(&parts);
    // 読みの返事が来た時点で、列の頭は 1 回目を試み終えている（期限の起点が決まっている）。
    let _ = parts
        .reader
        .kvs(AskerId::new("t"))
        .get_in(PersistScope::App, "x");
    backend.advance(Duration::from_secs(2));

    let got = settle_and_drain(&mut world, parts);
    drop(hold);
    let cause = assert_one_ghost_failure(got, Some(&ghost_dir()));
    assert!(
        matches!(&cause, PersistFailureCause::Io(reason) if reason.contains("1 秒待っても")),
        "{cause:?}"
    );
}

#[test]
fn unusable_file_reaches_the_fatal_desk_once() {
    let mut world = installed_world();
    let backend = MemBackend::new();
    backend.mark_unusable(&ghost_dir());
    let parts = spawn(&world, ghost_roots(), &backend);

    put_boot_count(&parts);

    let cause = assert_one_ghost_failure(settle_and_drain(&mut world, parts), Some(&ghost_dir()));
    assert!(
        matches!(cause, PersistFailureCause::Unusable(_)),
        "{cause:?}"
    );
}

/// 書けるときは致命の机へ何も届かず、起動の前の確かめも通る。
#[test]
fn writable_memory_tells_nothing_and_boot_check_passes() {
    let mut world = installed_world();
    let backend = MemBackend::new();
    let parts = spawn(&world, ghost_roots(), &backend);

    put_boot_count(&parts);

    assert_eq!(settle_and_drain(&mut world, parts), (None, 0));

    let tmp = TempPath::under_target("write-failure-inject-boot-check");
    let app = tmp.child("app-profile");
    let ghost_root = tmp.child("ghost-a");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir_all(ghost_root.join("ghost").join("master")).unwrap();
    let cfg = ConfigInputs {
        ghost_root,
        balloon_root: tmp.child("balloon"),
    };
    check_boot(false, &cfg, &app).expect("書ける置き場は通る");
}
