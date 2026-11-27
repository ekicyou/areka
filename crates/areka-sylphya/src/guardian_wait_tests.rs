use super::*;
use crate::actor::Writer;
use crate::persist::PersistKey;
use crate::store::{Access, FsBackend, Located};
use areka_actor::{ReplyReceiver, reply_channel};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use temp_path_kit::TempPath;

/// 退行したときに止まらず赤になるための上限。待ちの判定には使わない。
const BOUND: std::time::Duration = std::time::Duration::from_secs(5);

const BOOT: &str = "areka.boot.count";
const LAST_SHELL: &str = "areka.last.shell";

fn asker() -> AskerId {
    AskerId::new("ghost-a")
}

fn layer_roots(layer: PersistScope, root: &Path) -> ScopeRoots {
    let mut roots = ScopeRoots::default();
    *match layer {
        PersistScope::App => &mut roots.app,
        PersistScope::Ghost => &mut roots.ghost,
        PersistScope::Shell => &mut roots.shell,
        PersistScope::Balloon => &mut roots.balloon,
    } = Some(root.to_path_buf());
    roots
}

fn file_write(layer: PersistScope, root: &Path, key: &str, value: &str) -> FileWrite {
    FileWrite {
        layer,
        root: Some(root.to_path_buf()),
        puts: vec![(key.to_owned(), value.to_owned())],
        removes: Vec::new(),
        typed: Vec::new(),
    }
}

/// 番人とは別の核で、層のファイルへ書いておく。
fn seed(backend: &Arc<FsBackend>, layer: PersistScope, root: &Path, key: &str, value: &str) {
    let mut store = Store::open(layer_roots(layer, root), backend.clone()).unwrap();
    let attempt = store.try_write(&file_write(layer, root, key, value));
    assert!(
        matches!(attempt, WriteAttempt::Done(PersistOutcome::Saved)),
        "{attempt:?}"
    );
}

/// 番人とは別の核で、層のファイルを読む。
fn read_file(
    backend: &Arc<FsBackend>,
    layer: PersistScope,
    root: &Path,
    key: &str,
) -> Option<String> {
    Store::open(layer_roots(layer, root), backend.clone())
        .unwrap()
        .read(&asker(), key, Some(layer), Home::File)
}

/// 別の書き手として、層のファイルの書きの錠を取る。
fn hold_writer(root: &Path) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(root.join("sylphya.sqlite")).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    conn
}

/// 番人へ名前での読みを送り、返事を待つ。
pub(crate) fn ask(
    p: &SylphyaPublisher,
    asker: AskerId,
    layer: Option<PersistScope>,
    key: &str,
    home: Home,
) -> Option<String> {
    let (reply, rx) = reply_channel();
    p.send(SylphyaMsg::Get {
        asker,
        key: key.to_owned(),
        layer,
        home,
        reply,
    });
    rx.recv().expect("読みの返事")
}

fn get(p: &SylphyaPublisher, layer: PersistScope, key: &str, home: Home) -> Option<String> {
    ask(p, asker(), Some(layer), key, home)
}

fn write(
    p: &SylphyaPublisher,
    writer: Writer,
    layer: PersistScope,
    key: &str,
    value: &str,
) -> ReplyReceiver<WriteOutcome> {
    let (reply, rx) = reply_channel();
    p.send(SylphyaMsg::Write {
        writer,
        asker: asker(),
        layer,
        puts: vec![(key.to_owned(), value.to_owned())],
        removes: Vec::new(),
        reply,
    });
    rx
}

fn still_waiting(rx: &ReplyReceiver<WriteOutcome>) -> bool {
    matches!(rx.try_recv(), Ok(None))
}

fn io_failure(failure: &PersistFailure) -> bool {
    matches!(failure.cause, PersistFailureCause::Io(_))
}

#[test]
fn a_read_behind_a_waiting_write_answers_first_with_the_value_before_the_write() {
    let dir = TempPath::under_target("sylphya-guardian-read-first");
    let root = dir.child("ghost");
    let backend = FsBackend::with_manual_clock();
    seed(&backend, PersistScope::Ghost, &root, BOOT, "1");
    let hold = hold_writer(&root);
    let (tx, failures) = mpsc::channel();
    let parts = spawn_sylphya_with_failures(
        SylphyaInit::new(layer_roots(PersistScope::Ghost, &root), backend.clone()),
        Some(tx),
    );
    let p = &parts.publisher;

    let pending = write(p, Writer::Fixed, PersistScope::Ghost, BOOT, "2");
    assert_eq!(
        get(p, PersistScope::Ghost, BOOT, Home::File).as_deref(),
        Some("1")
    );
    assert!(still_waiting(&pending), "錠が在る間、書きの返事はまだ無い");

    drop(hold);
    assert_eq!(pending.recv_timeout(BOUND).unwrap(), WriteOutcome::Written);
    assert_eq!(
        get(p, PersistScope::Ghost, BOOT, Home::File).as_deref(),
        Some("2")
    );
    assert_eq!(failures.try_iter().count(), 0);

    p.close();
    parts.handle.join().expect("join");
}

#[test]
fn a_write_still_locked_past_one_second_fails_once_into_the_sink() {
    let dir = TempPath::under_target("sylphya-guardian-wait-limit");
    let root = dir.child("ghost");
    let backend = FsBackend::with_manual_clock();
    seed(&backend, PersistScope::Ghost, &root, BOOT, "1");
    let hold = hold_writer(&root);
    let (tx, failures) = mpsc::channel();
    let parts = spawn_sylphya_with_failures(
        SylphyaInit::new(layer_roots(PersistScope::Ghost, &root), backend.clone()),
        Some(tx),
    );
    let p = &parts.publisher;

    p.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "2".into())],
    );
    // 読みの返事が来た時点で、列の頭は 1 回目を試み終えている（期限の起点が決まっている）。
    assert_eq!(
        get(p, PersistScope::Ghost, BOOT, Home::File).as_deref(),
        Some("1")
    );
    backend.pause(OP_WAIT + RETRY_STEP);
    p.barrier()
        .expect("期限を越えた書きが片づくと反映待ちが返る");

    let got: Vec<PersistFailure> = failures.try_iter().collect();
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].scope, PersistScope::Ghost);
    assert_eq!(got[0].dir.as_deref(), Some(root.as_path()));
    assert_eq!(got[0].keys, vec![PersistKey::BootCount]);
    assert!(io_failure(&got[0]), "{got:?}");

    drop(hold);
    assert_eq!(
        get(p, PersistScope::Ghost, BOOT, Home::File).as_deref(),
        Some("1")
    );
    p.close();
    parts.handle.join().expect("join");
}

#[test]
fn reads_while_the_import_cannot_lock_return_the_legacy_values() {
    let dir = TempPath::under_target("sylphya-guardian-import-busy");
    let root = dir.child("ghost");
    let backend = FsBackend::with_manual_clock();
    seed(
        &backend,
        PersistScope::Ghost,
        &root,
        "areka.last.ghost",
        "g",
    );
    std::fs::write(
        root.join("sylphya.toml"),
        "format-version = 1\n[boot]\ncount = \"5\"\n",
    )
    .unwrap();
    let hold = hold_writer(&root);
    let parts = spawn_sylphya(SylphyaInit::new(
        layer_roots(PersistScope::Ghost, &root),
        backend.clone(),
    ));
    let p = &parts.publisher;

    let pending = write(
        p,
        Writer::Fixed,
        PersistScope::Ghost,
        "areka.vanish.count",
        "3",
    );
    assert_eq!(
        get(p, PersistScope::Ghost, BOOT, Home::File).as_deref(),
        Some("5")
    );
    assert!(still_waiting(&pending));
    assert!(root.join("sylphya.toml").exists());

    drop(hold);
    assert_eq!(pending.recv_timeout(BOUND).unwrap(), WriteOutcome::Written);
    assert!(!root.join("sylphya.toml").exists());
    assert!(root.join("sylphya.toml.imported").exists());
    p.close();
    parts.handle.join().expect("join");
    assert_eq!(
        read_file(&backend, PersistScope::Ghost, &root, BOOT).as_deref(),
        Some("5")
    );
}

#[test]
fn a_re_root_while_waiting_moves_volatile_values_but_not_the_waiting_write() {
    let dir = TempPath::under_target("sylphya-guardian-re-root");
    let old = dir.child("shell-old");
    let new = dir.child("shell-new");
    let backend = FsBackend::with_manual_clock();
    seed(&backend, PersistScope::Shell, &old, LAST_SHELL, "a");
    let hold = hold_writer(&old);
    let parts = spawn_sylphya(SylphyaInit::new(
        layer_roots(PersistScope::Shell, &old),
        backend.clone(),
    ));
    let p = &parts.publisher;

    assert_eq!(
        write(p, Writer::ByName, PersistScope::Shell, "my.before", "x")
            .recv()
            .unwrap(),
        WriteOutcome::Written
    );
    let pending = write(p, Writer::Fixed, PersistScope::Shell, LAST_SHELL, "b");
    p.send(SylphyaMsg::SetLayerRoot {
        layer: PersistScope::Shell,
        root: Some(new.clone()),
    });
    assert_eq!(
        write(p, Writer::ByName, PersistScope::Shell, "my.after", "y")
            .recv()
            .unwrap(),
        WriteOutcome::Written,
        "揮発の書きは列を待たない"
    );
    assert_eq!(get(p, PersistScope::Shell, "my.before", Home::Memory), None);
    assert_eq!(
        get(p, PersistScope::Shell, "my.after", Home::Memory).as_deref(),
        Some("y")
    );
    assert_eq!(get(p, PersistScope::Shell, LAST_SHELL, Home::File), None);
    assert!(still_waiting(&pending));

    drop(hold);
    assert_eq!(pending.recv_timeout(BOUND).unwrap(), WriteOutcome::Written);
    p.close();
    parts.handle.join().expect("join");
    assert_eq!(
        read_file(&backend, PersistScope::Shell, &old, LAST_SHELL).as_deref(),
        Some("b"),
        "待っていた書きは受けた時点の置き場へ"
    );
    assert!(matches!(
        backend.locate(&new, Access::ReadWrite).unwrap(),
        Located::Missing
    ));
}

#[test]
fn close_tries_each_queued_write_once_and_reports_the_locked_one() {
    let dir = TempPath::under_target("sylphya-guardian-close");
    let ghost = dir.child("ghost");
    let app = dir.child("app");
    let backend = FsBackend::with_manual_clock();
    seed(&backend, PersistScope::Ghost, &ghost, BOOT, "1");
    let hold = hold_writer(&ghost);
    let roots = ScopeRoots {
        app: Some(app.clone()),
        ghost: Some(ghost.clone()),
        ..ScopeRoots::default()
    };
    let (tx, failures) = mpsc::channel();
    let parts = spawn_sylphya_with_failures(SylphyaInit::new(roots, backend.clone()), Some(tx));
    let p = &parts.publisher;

    p.persist_put(
        PersistScope::Ghost,
        vec![(PersistKey::BootCount, "2".into())],
    );
    p.persist_put(
        PersistScope::App,
        vec![(PersistKey::LastGhost, "emo".into())],
    );
    let (fence, fenced) = reply_channel();
    p.send(SylphyaMsg::Barrier { reply: fence });
    p.close();
    parts.handle.join().expect("join");
    drop(hold);

    let got: Vec<PersistFailure> = failures.try_iter().collect();
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].scope, PersistScope::Ghost);
    assert_eq!(got[0].keys, vec![PersistKey::BootCount]);
    assert!(io_failure(&got[0]), "{got:?}");
    fenced.recv().expect("列に並んだ反映待ちも返る");
    assert_eq!(
        read_file(&backend, PersistScope::Ghost, &ghost, BOOT).as_deref(),
        Some("1")
    );
    assert_eq!(
        read_file(&backend, PersistScope::App, &app, "areka.last.ghost").as_deref(),
        Some("emo"),
        "錠の無い層の書きは終了のときに書ける"
    );
}

/// 置き場の口の 1 か所だけを差し替える試験の口（揮発の DB の在りか・書きの直前の panic）。
pub(crate) struct Hooked {
    pub(crate) inner: Arc<dyn StoreBackend>,
    pub(crate) volatile_uri: Option<String>,
    pub(crate) panic_on_write: bool,
}

impl StoreBackend for Hooked {
    fn locate(&self, root: &Path, access: Access) -> std::io::Result<Located> {
        self.inner.locate(root, access)
    }
    fn ensure_dir(&self, root: &Path) -> std::io::Result<()> {
        self.inner.ensure_dir(root)
    }
    fn read_legacy(&self, root: &Path) -> std::io::Result<Option<String>> {
        self.inner.read_legacy(root)
    }
    fn retire_legacy(&self, root: &Path) -> std::io::Result<()> {
        self.inner.retire_legacy(root)
    }
    fn withholds(&self, root: &Path) -> bool {
        self.inner.withholds(root)
    }
    fn volatile_uri(&self, app_root: Option<&Path>) -> String {
        self.volatile_uri
            .clone()
            .unwrap_or_else(|| self.inner.volatile_uri(app_root))
    }
    fn held_uri(&self) -> String {
        self.inner.held_uri()
    }
    fn now(&self) -> Duration {
        self.inner.now()
    }
    fn pause(&self, span: Duration) {
        self.inner.pause(span);
    }
    fn before_write(&self, root: &Path) -> std::io::Result<()> {
        assert!(!self.panic_on_write, "injected panic before a write");
        self.inner.before_write(root)
    }
}

#[test]
fn a_guardian_whose_store_cannot_open_still_runs_and_degrades() {
    let dir = TempPath::under_target("sylphya-guardian-no-store");
    let root: PathBuf = dir.child("ghost");
    let backend = Arc::new(Hooked {
        inner: FsBackend::with_manual_clock(),
        volatile_uri: Some("file:areka-missing?vfs=areka-no-such-vfs".to_owned()),
        panic_on_write: false,
    });
    let (tx, failures) = mpsc::channel();
    let parts = spawn_sylphya_with_failures(
        SylphyaInit::new(layer_roots(PersistScope::Ghost, &root), backend),
        Some(tx),
    );
    let p = &parts.publisher;

    assert_eq!(get(p, PersistScope::Ghost, BOOT, Home::File), None);
    let persisted = write(p, Writer::Fixed, PersistScope::Ghost, BOOT, "1").recv();
    assert!(
        matches!(&persisted, Ok(WriteOutcome::Failed(WriteFailed::Persist(f))) if io_failure(f)),
        "{persisted:?}"
    );
    let volatile = write(p, Writer::ByName, PersistScope::Ghost, "my.mood", "x").recv();
    assert!(
        matches!(
            volatile,
            Ok(WriteOutcome::Failed(WriteFailed::Volatile { .. }))
        ),
        "{volatile:?}"
    );
    p.close();
    parts.handle.join().expect("join");

    assert_eq!(
        failures.try_iter().count(),
        1,
        "覚える値の失敗だけが受け口へ"
    );
    assert!(!root.exists(), "何も作らない");
}
