//! 置き場の口（`StoreBackend`）: 層の置き場を、DB の在りか・古いファイル・時計へ結ぶ継ぎ目。
//!
//! `areka-sylphya` でファイルシステムに触れるのは [`FsBackend`] だけ。試験の偽物
//! [`MemBackend`] は、層のファイルの代わりに名前つきの memdb を指し、本物の sqlite が動く。

use super::locate::{self, DB_FILE, LEGACY_FILE};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

const LOG_TARGET: &str = "areka_sylphya::store";

/// 層の記憶の DB の開き方。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ReadOnly,
    ReadWrite,
    /// 無ければ作る（ファイルを作ってよいのはこの開き方だけ）。
    Create,
}

/// 層の記憶の DB の在りか。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Located {
    /// ATTACH に渡す URI。
    Uri(String),
    /// 記憶のファイルがまだ無い。
    Missing,
}

/// 層の置き場を、記憶の DB の在りか・古いファイル・時計・控える範囲へ結ぶ口。
pub trait StoreBackend: Send + Sync {
    /// 層の置き場の記憶の DB を ATTACH する先。`Create` のほかは、無ければ `Missing`。
    fn locate(&self, root: &Path, access: Access) -> std::io::Result<Located>;
    fn ensure_dir(&self, root: &Path) -> std::io::Result<()>;
    /// 古い `sylphya.toml`（無ければ `None`）。
    fn read_legacy(&self, root: &Path) -> std::io::Result<Option<String>>;
    /// 古いファイルを、上書きせずに別の名前へ替える（もう無ければ `Ok`）。
    fn retire_legacy(&self, root: &Path) -> std::io::Result<()>;
    fn withholds(&self, root: &Path) -> bool;
    /// アプリの層の揮発の DB の在りか（生かしておく接続はこの口が持つ）。
    fn volatile_uri(&self, app_root: Option<&Path>) -> String;
    /// 控えの DB の在りか（生かしておく接続はこの口が持つ）。
    fn held_uri(&self) -> String;
    /// 待ちの時計（単調）。
    fn now(&self) -> Duration;
    /// 取り直しの間を空ける。
    fn pause(&self, span: Duration);
    /// 書きの直前に呼ばれる（本番は常に `Ok`・偽物は注ぎ込まれた失敗を 1 回返す）。
    fn before_write(&self, root: &Path) -> std::io::Result<()>;
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // どの表も短い挿入だけで、途中で落ちても壊れた中身は残らない＝毒は無視してよい。
    m.lock().unwrap_or_else(|e| e.into_inner())
}

enum Clock {
    Real(Instant),
    Manual(Mutex<Duration>),
}

impl Clock {
    fn manual() -> Self {
        Clock::Manual(Mutex::new(Duration::ZERO))
    }

    fn now(&self) -> Duration {
        match self {
            Clock::Real(start) => start.elapsed(),
            Clock::Manual(at) => *lock(at),
        }
    }

    fn pause(&self, span: Duration) {
        match self {
            Clock::Real(_) => std::thread::sleep(span),
            Clock::Manual(at) => *lock(at) += span,
        }
    }
}

/// 同じプロセスの中で、口の実体ごとに memdb の名前を分ける番号。
static NEXT_BACKEND: AtomicU64 = AtomicU64::new(0);

/// 名前つきの memdb と、それを生かしておく接続。
struct MemDbs {
    prefix: String,
    names: Mutex<BTreeMap<Option<PathBuf>, String>>,
    keep: Mutex<Vec<rusqlite::Connection>>,
}

impl MemDbs {
    fn new(kind: &str) -> Self {
        let n = NEXT_BACKEND.fetch_add(1, Ordering::Relaxed);
        MemDbs {
            prefix: format!("areka-{kind}-{n}"),
            names: Mutex::new(BTreeMap::new()),
            keep: Mutex::new(Vec::new()),
        }
    }

    /// `slot` の memdb の名前。初めての `slot` なら、生かしておく接続を開いてから返す。
    fn name(&self, slot: Option<&Path>) -> String {
        let mut names = lock(&self.names);
        if let Some(name) = names.get(&slot.map(Path::to_path_buf)) {
            return name.clone();
        }
        let name = format!("{}-{}", self.prefix, names.len());
        match rusqlite::Connection::open(locate::memdb_uri(&name, Access::Create)) {
            Ok(conn) => lock(&self.keep).push(conn),
            Err(e) => tracing::error!(
                target: LOG_TARGET,
                event = "store_memdb_open_failed",
                name = %name,
                error = %e,
                "memdb keep-alive connection failed; its values last only while attached"
            ),
        }
        names.insert(slot.map(Path::to_path_buf), name.clone());
        name
    }

    fn get(&self, slot: &Path) -> Option<String> {
        lock(&self.names).get(&Some(slot.to_path_buf())).cloned()
    }
}

/// 揮発の DB と控えの DB（どちらも memdb）を、本番と偽物で同じ形に持つ。
struct Volatile {
    app: MemDbs,
    held: MemDbs,
}

impl Volatile {
    fn new() -> Self {
        Volatile {
            app: MemDbs::new("appvol"),
            held: MemDbs::new("held"),
        }
    }

    fn app_uri(&self, app_root: Option<&Path>) -> String {
        locate::memdb_uri(&self.app.name(app_root), Access::ReadWrite)
    }

    fn held_uri(&self) -> String {
        locate::memdb_uri(&self.held.name(None), Access::ReadWrite)
    }
}

/// 本番の置き場の口（ファイル）。プロセスに 1 つ（[`fs_backend`]）。
pub struct FsBackend {
    clock: Clock,
    volatile: Volatile,
}

impl FsBackend {
    /// 本物のファイルの上で、進めるだけで眠らない時計を持つ口（試験用）。
    pub fn with_manual_clock() -> Arc<FsBackend> {
        Arc::new(FsBackend {
            clock: Clock::manual(),
            volatile: Volatile::new(),
        })
    }
}

/// プロセスに 1 つの本番の置き場の口（実時間の時計）。
pub fn fs_backend() -> Arc<dyn StoreBackend> {
    static ONE: OnceLock<Arc<FsBackend>> = OnceLock::new();
    ONE.get_or_init(|| {
        Arc::new(FsBackend {
            clock: Clock::Real(Instant::now()),
            volatile: Volatile::new(),
        })
    })
    .clone()
}

impl StoreBackend for FsBackend {
    fn locate(&self, root: &Path, access: Access) -> std::io::Result<Located> {
        let file = root.join(DB_FILE);
        if access != Access::Create {
            match std::fs::metadata(&file) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Located::Missing),
                Err(e) => return Err(e),
            }
        }
        locate::file_uri(&file, access).map(Located::Uri)
    }

    fn ensure_dir(&self, root: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(root)
    }

    fn read_legacy(&self, root: &Path) -> std::io::Result<Option<String>> {
        match std::fs::read_to_string(root.join(LEGACY_FILE)) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn retire_legacy(&self, root: &Path) -> std::io::Result<()> {
        let legacy = root.join(LEGACY_FILE);
        match std::fs::metadata(&legacy) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        }
        // Windows の rename は在る行き先を置き換えるので、行き先の名前を空のファイルとして先に取る。
        let mut n = 0;
        let claimed = loop {
            let candidate = root.join(locate::retired_name(n));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(_) => break candidate,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
                Err(e) => return Err(e),
            }
        };
        match std::fs::rename(&legacy, &claimed) {
            Ok(()) => Ok(()),
            Err(e) => {
                if let Err(cleanup) = std::fs::remove_file(&claimed) {
                    tracing::warn!(
                        target: LOG_TARGET,
                        event = "store_retire_claim_left",
                        path = %claimed.display(),
                        error = %cleanup,
                        "empty claimed name left after a failed rename"
                    );
                }
                // 相手が先に替えていた＝もう無いので、替えるものは残っていない。
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    fn withholds(&self, root: &Path) -> bool {
        crate::persist::withhold::withholds(root)
    }

    fn volatile_uri(&self, app_root: Option<&Path>) -> String {
        self.volatile.app_uri(app_root)
    }

    fn held_uri(&self) -> String {
        self.volatile.held_uri()
    }

    fn now(&self) -> Duration {
        self.clock.now()
    }

    fn pause(&self, span: Duration) {
        self.clock.pause(span);
    }

    fn before_write(&self, _root: &Path) -> std::io::Result<()> {
        Ok(())
    }
}

/// [`MemBackend::mark_unusable`] が層の DB に付ける、areka のものでない `application_id`。
pub const UNUSABLE_APPLICATION_ID: i32 = 1;

/// 試験の偽物の置き場の口（memdb の上の本物の sqlite・失敗を注ぎ込める）。
pub struct MemBackend {
    clock: Clock,
    volatile: Volatile,
    layers: MemDbs,
    legacy: Mutex<BTreeMap<PathBuf, BTreeMap<String, String>>>,
    fail_next_write: AtomicBool,
    fail_next_retire: AtomicBool,
}

/// 別の書き手が層の DB の書きの錠を持っている間の取っ手（落とすと放す）。
#[must_use]
pub struct WriterHold {
    _conn: rusqlite::Connection,
}

impl MemBackend {
    pub fn new() -> Arc<MemBackend> {
        Arc::new(MemBackend {
            clock: Clock::manual(),
            volatile: Volatile::new(),
            layers: MemDbs::new("layer"),
            legacy: Mutex::new(BTreeMap::new()),
            fail_next_write: AtomicBool::new(false),
            fail_next_retire: AtomicBool::new(false),
        })
    }

    fn layer_uri(&self, root: &Path, access: Access) -> String {
        locate::memdb_uri(&self.layers.name(Some(root)), access)
    }

    fn open_layer(&self, root: &Path) -> rusqlite::Connection {
        rusqlite::Connection::open(self.layer_uri(root, Access::Create))
            .expect("偽の層の memdb は開けるはず")
    }

    /// 次の 1 回の書きを、OS が拒んだ形で失敗させる。
    pub fn fail_next_write(&self) {
        self.fail_next_write.store(true, Ordering::SeqCst);
    }

    /// 次の 1 回の古いファイルの名前替えを、OS が拒んだ形で失敗させる。
    pub fn fail_next_retire(&self) {
        self.fail_next_retire.store(true, Ordering::SeqCst);
    }

    /// その置き場の記憶の DB を、areka のものでない（使えない）形にする。
    pub fn mark_unusable(&self, root: &Path) {
        self.open_layer(root)
            .execute_batch(&format!(
                "PRAGMA application_id = {UNUSABLE_APPLICATION_ID}; CREATE TABLE IF NOT EXISTS foreign_data(x);"
            ))
            .expect("偽の層の memdb へ書けるはず");
    }

    /// 別の書き手としてその置き場の DB の書きの錠を取る。
    pub fn hold_writer(&self, root: &Path) -> WriterHold {
        let conn = self.open_layer(root);
        conn.execute_batch("BEGIN IMMEDIATE")
            .expect("偽の層の memdb の書きの錠は取れるはず");
        WriterHold { _conn: conn }
    }

    /// 時計を進める（眠らない）。
    pub fn advance(&self, span: Duration) {
        self.clock.pause(span);
    }

    /// その置き場に古い `sylphya.toml` を植える。
    pub fn plant_legacy(&self, root: &Path, toml: &str) {
        lock(&self.legacy)
            .entry(root.to_path_buf())
            .or_default()
            .insert(LEGACY_FILE.to_owned(), toml.to_owned());
    }

    /// その置き場に在る古いファイルの名前（名前の順）。
    pub fn legacy_names(&self, root: &Path) -> Vec<String> {
        lock(&self.legacy)
            .get(root)
            .map(|files| files.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl StoreBackend for MemBackend {
    fn locate(&self, root: &Path, access: Access) -> std::io::Result<Located> {
        if access == Access::Create || self.layers.get(root).is_some() {
            Ok(Located::Uri(self.layer_uri(root, access)))
        } else {
            Ok(Located::Missing)
        }
    }

    fn ensure_dir(&self, _root: &Path) -> std::io::Result<()> {
        Ok(())
    }

    fn read_legacy(&self, root: &Path) -> std::io::Result<Option<String>> {
        Ok(lock(&self.legacy)
            .get(root)
            .and_then(|files| files.get(LEGACY_FILE).cloned()))
    }

    fn retire_legacy(&self, root: &Path) -> std::io::Result<()> {
        if self.fail_next_retire.swap(false, Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "試験の偽物が注ぎ込んだ失敗（OS が名前替えを拒んだ）",
            ));
        }
        let mut legacy = lock(&self.legacy);
        let Some(files) = legacy.get_mut(root) else {
            return Ok(());
        };
        let Some(content) = files.remove(LEGACY_FILE) else {
            return Ok(());
        };
        let name = (0..)
            .map(locate::retired_name)
            .find(|name| !files.contains_key(name))
            .expect("空いた名前は必ず在る");
        files.insert(name, content);
        Ok(())
    }

    fn withholds(&self, root: &Path) -> bool {
        crate::persist::withhold::withholds(root)
    }

    fn volatile_uri(&self, app_root: Option<&Path>) -> String {
        self.volatile.app_uri(app_root)
    }

    fn held_uri(&self) -> String {
        self.volatile.held_uri()
    }

    fn now(&self) -> Duration {
        self.clock.now()
    }

    fn pause(&self, span: Duration) {
        self.clock.pause(span);
    }

    fn before_write(&self, _root: &Path) -> std::io::Result<()> {
        if self.fail_next_write.swap(false, Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "試験の偽物が注ぎ込んだ失敗（OS が書きを拒んだ）",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
