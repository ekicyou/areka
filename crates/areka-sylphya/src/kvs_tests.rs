use super::*;
use crate::store::{MemBackend, StoreBackend};
use std::path::{Path, PathBuf};
use std::time::Duration;

const BOOT: &str = "areka.boot.count";
const FREE: &str = "myplugin.state";

fn ghost() -> PathBuf {
    PathBuf::from("/g")
}

fn open(mem: &Arc<MemBackend>) -> KvsDirect {
    KvsDirect::open(
        ScopeRoots {
            ghost: Some(ghost()),
            ..ScopeRoots::default()
        },
        mem.clone(),
    )
}

#[test]
fn the_direct_port_reads_back_what_it_wrote_and_refuses_fixed_names() {
    let mem = MemBackend::new();
    let kvs = open(&mem);

    assert_eq!(
        kvs.put(PersistScope::Ghost, &[(BOOT, "4")]),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(BOOT).as_deref(), Some("4"));
    assert_eq!(kvs.get_in(PersistScope::App, BOOT), None);
    assert_eq!(
        open(&mem).get(BOOT).as_deref(),
        Some("4"),
        "別に開いた口からも読める"
    );

    assert_eq!(
        kvs.set(PersistScope::Ghost, BOOT, "5"),
        WriteOutcome::Refused
    );
    assert_eq!(kvs.get(BOOT).as_deref(), Some("4"));

    assert_eq!(
        kvs.set(PersistScope::Ghost, FREE, "on"),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(FREE).as_deref(), Some("on"));
    assert_eq!(kvs.unset(PersistScope::Ghost, FREE), WriteOutcome::Written);
    assert_eq!(kvs.get(FREE), None);

    assert_eq!(
        kvs.remove(PersistScope::Ghost, &[BOOT]),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(BOOT), None);
}

#[test]
fn a_direct_write_behind_a_lock_retries_with_pause_and_gives_up_after_one_second() {
    let mem = MemBackend::new();
    let kvs = open(&mem);
    assert_eq!(
        kvs.put(PersistScope::Ghost, &[(BOOT, "1")]),
        WriteOutcome::Written
    );
    let hold = mem.hold_writer(Path::new("/g"));
    let before = mem.now();

    let outcome = kvs.put(PersistScope::Ghost, &[(BOOT, "2")]);
    assert!(
        matches!(&outcome, WriteOutcome::Failed(WriteFailed::Persist(f))
            if matches!(f.cause, crate::persist::PersistFailureCause::Io(_))),
        "{outcome:?}"
    );
    assert!(mem.now() - before > Duration::from_secs(1));

    drop(hold);
    assert_eq!(
        kvs.get(BOOT).as_deref(),
        Some("1"),
        "巻き戻して何も入れない"
    );
    assert_eq!(
        kvs.put(PersistScope::Ghost, &[(BOOT, "2")]),
        WriteOutcome::Written
    );
    assert_eq!(kvs.get(BOOT).as_deref(), Some("2"));
}
