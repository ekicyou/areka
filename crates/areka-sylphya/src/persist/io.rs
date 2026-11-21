//! 永続 IO シーム（`PersistIo`）: 実 FS の原子的確定と、決定論テスト向け故障注入 fake。
//!
//! この層は「1 スコープの `sylphya.toml` を 1 本の文字列として往復する」だけの薄い境界で
//! あり、TOML スキーマ（[`super::format`]）にも鏡像にも依存しない（content は不透明 `String`）。
//! **areka-sylphya でファイルシステムに触れるのはこのモジュールだけ**。暦時計・OS ユーザー名等の
//! 非決定な外部環境は一切読まない（fake は外部環境を必要とせず・real は temp 名に PID/時刻等を混ぜない）。
//!
//! ## 原子的確定
//!
//! [`FsPersistIo::commit`] は **同一ディレクトリ内の一時ファイルへ全書込→`fsync`→`rename`**
//! で確定する。宛先と同一ボリューム上の temp を使うため `rename` は原子的で、Windows でも
//! 既存宛先を置換する（`MoveFileEx` の REPLACE 相当）。temp 書込・rename の
//! いずれが失敗しても宛先の既存ファイルは無傷であり（本体を直接 truncate しない）、失敗は
//! 必ず `Err` で表面化する。temp は失敗時も可能な限り掃除する。
//!
//! ## 決定論テスト
//!
//! [`FakePersistIo`] はメモリ内ストア（`Mutex<BTreeMap<PathBuf, String>>`）で、
//! [`FakePersistIo::fail_next_commit`] / [`FakePersistIo::fail_next_read`] により次回操作を
//! 決定論的に失敗させられる。原子性の観測条件——「commit 中断後も既存内容が無傷」——は
//! この fake で純 x64 検証する（実 FS 障害を注入せずに済む）。

use std::io::Write;
use std::path::Path;

/// 永続 IO の注入シーム。
///
/// - [`read`](PersistIo::read): 宛先が無ければ `Ok(None)`（不在はエラーではない）・在れば
///   `Ok(Some(content))`・真の IO 障害（権限等）のみ `Err`。
/// - [`commit`](PersistIo::commit): `content` を `path` へ**原子的**に確定する。書込・確定の
///   いずれが失敗しても既存ファイルは破壊せず `Err` を返す。
///
/// `Send` 境界: sylphya アクタースレッド（[`super`] の spawn）が `Box<dyn PersistIo>` を保持
/// して移送するため。
pub trait PersistIo: Send {
    /// `path` の内容を読む。不在は `Ok(None)`（エラーにしない）。
    fn read(&self, path: &Path) -> std::io::Result<Option<String>>;
    /// `content` を `path` へ原子的に確定する（temp 書込→rename）。失敗時、既存ファイルは無傷。
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()>;
    /// 書かずに控える道か（既定は控えない＝試験の中の偽の IO はそのまま）。
    fn withholds(&self, _path: &Path) -> bool {
        false
    }
    /// `path` を読んで重ねて書く間、ほかのプロセスの書き手を締め出す錠を取る。
    /// 返した File を捨てると錠が放れる。既定は錠を持たない（`Ok(None)`・試験の偽の IO）。
    fn lock_for_update(&self, _path: &Path) -> std::io::Result<Option<std::fs::File>> {
        Ok(None)
    }
}

/// 一時の掴み（共有違反・拒み）を取り直す待ちの上限（記憶のファイルの読み・確定の `rename`・更新の間の錠）。
pub(crate) const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(1);
/// 一時の掴みを取り直す間隔。
pub(crate) const LOCK_RETRY: std::time::Duration = std::time::Duration::from_millis(10);

/// 待てば通るかもしれない失敗か（共有違反＝os error 32・拒み＝5。ウイルス対策などの一時の掴み）。
pub(crate) fn is_transient(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(32) | Some(5))
}

/// `op` を一時の掴み（[`is_transient`]）の間だけ [`LOCK_RETRY`] ごとに最大 `wait` 取り直す
/// （本番の呼び手は [`LOCK_WAIT`]）。
pub(crate) fn retry_transient<T>(
    wait: std::time::Duration,
    mut op: impl FnMut() -> std::io::Result<T>,
) -> std::io::Result<T> {
    let deadline = std::time::Instant::now() + wait;
    loop {
        match op() {
            Err(e) if is_transient(&e) && std::time::Instant::now() < deadline => {
                std::thread::sleep(LOCK_RETRY);
            }
            result => return result,
        }
    }
}

/// `FILE_FLAG_DELETE_ON_CLOSE`。`areka-sylphya` は `windows` に依存しないので値で書く。
const FILE_FLAG_DELETE_ON_CLOSE: u32 = 0x0400_0000;

/// 同じ IO を複数の持ち手で分ける包み（4 つの口をすべて中身へ渡す）。
impl<T: PersistIo + Sync + ?Sized> PersistIo for std::sync::Arc<T> {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        (**self).read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        (**self).commit(path, content)
    }
    fn withholds(&self, path: &Path) -> bool {
        (**self).withholds(path)
    }
    fn lock_for_update(&self, path: &Path) -> std::io::Result<Option<std::fs::File>> {
        (**self).lock_for_update(path)
    }
}

/// 本番 IO 実装（`std::fs` による temp 書込→`rename` の原子的確定）。
///
/// areka-sylphya でファイルシステムに触れる唯一の型。暦時計・OS 環境は読まない。
#[derive(Clone, Copy, Debug, Default)]
pub struct FsPersistIo;

impl FsPersistIo {
    /// `path` と同一ディレクトリに置く一時ファイル名を組む（同一ボリューム＝rename 原子性の前提）。
    ///
    /// 非決定源（時刻・乱数・PID）を混ぜない。ほかのプロセスの書き手とは更新の間の錠
    /// （[`PersistIo::lock_for_update`]）で並ぶので、宛先ファイル名から導いた固定サフィックスで足りる。
    fn temp_path_for(path: &Path) -> std::path::PathBuf {
        let file_name = path
            .file_name()
            .map(|n| n.to_owned())
            .unwrap_or_else(|| std::ffi::OsString::from("sylphya"));
        let mut temp_name = file_name;
        temp_name.push(".tmp");
        path.with_file_name(temp_name)
    }

    /// 錠のファイルの名前（宛先の名前に `.lock` を足す・同じフォルダ）。非決定源は混ぜない。
    fn lock_path_for(path: &Path) -> std::path::PathBuf {
        let mut lock_name = path
            .file_name()
            .map(|n| n.to_owned())
            .unwrap_or_else(|| std::ffi::OsString::from("sylphya"));
        lock_name.push(".lock");
        path.with_file_name(lock_name)
    }

    /// 錠のファイルを共有なし・閉じたら消える指定で開く。共有違反と拒み（消える途中）の間だけ
    /// `LOCK_RETRY` ごとに `wait` まで取り直し、ほかの失敗は待たずに返す。
    fn lock_within(path: &Path, wait: std::time::Duration) -> std::io::Result<std::fs::File> {
        use std::os::windows::fs::OpenOptionsExt;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let lock_path = Self::lock_path_for(path);
        retry_transient(wait, || {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .share_mode(0)
                .custom_flags(FILE_FLAG_DELETE_ON_CLOSE)
                .open(&lock_path)
        })
        .map_err(|e| {
            // 拒みのままの期限切れは本当に書けないフォルダかもしれない＝「ほかの areka」と言い違えない。
            if e.raw_os_error() != Some(32) {
                return e;
            }
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "記憶のファイル sylphya.toml の錠を 1 秒待っても取れないので、書くのをやめました（ほかの areka が同じ記憶を書いている）",
            )
        })
    }

    /// ゴーストの記憶のフォルダを先に作る。控える範囲の中なら作らずに `Ok`、外なら
    /// `create_dir_all`。
    pub fn ensure_root_dir(dir: &Path) -> std::io::Result<()> {
        if super::withhold::withholds(dir) {
            tracing::debug!(
                target: super::LOG_TARGET,
                event = "persist_root_dir_withheld",
                dir = %dir.display(),
                "persist root dir not created (read-only run)"
            );
            return Ok(());
        }
        std::fs::create_dir_all(dir)
    }
}

impl PersistIo for FsPersistIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        // 控えを先に見る＝同じ走りの中では書けたときと同じ値を読む。
        if let Some(content) = super::withhold::staged(path) {
            return Ok(Some(content));
        }
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        if self.withholds(path) {
            super::withhold::stage(path, content);
            return Ok(());
        }
        // 新規ゴーストで `<profile>/areka/` が未作成でも書けるよう親を作る。無いと `File::create` が
        // NotFound(os error 3) を返し、全永続書込が Degraded に倒れる。
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let temp = Self::temp_path_for(path);

        let write_result = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
            file.sync_all()?;
            Ok(())
        })();
        if let Err(e) = write_result {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }

        // Windows の rename は既存宛先を置換する。一時の掴みは取り直す。
        if let Err(e) = retry_transient(LOCK_WAIT, || std::fs::rename(&temp, path)) {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }
        Ok(())
    }

    fn withholds(&self, path: &Path) -> bool {
        super::withhold::withholds(path)
    }

    fn lock_for_update(&self, path: &Path) -> std::io::Result<Option<std::fs::File>> {
        if self.withholds(path) {
            return Ok(None);
        }
        Self::lock_within(path, LOCK_WAIT).map(Some)
    }
}

// --- テスト支援: 他モジュールのテストも再利用するため `#[cfg(test)]` に閉じず `pub` 型として提供する ---

/// 決定論テスト向けのインメモリ `PersistIo`（故障注入可能）。**テスト支援専用**。
///
/// 本番経路では使わない（本番は [`FsPersistIo`]）。commit を途中で中断させても既存内容が
/// 無傷であること（原子性）を、実 FS 障害なしに純 x64 で検証するために存在する。他モジュールの
/// テストからも再利用できるよう、`#[cfg(test)]` ではなく通常の `pub` 型として公開する。
///
/// スレッド安全（`Mutex`）。決定論: 同一の注入故障列に対し同一結果を返す。
#[derive(Debug, Default)]
pub struct FakePersistIo {
    inner: std::sync::Mutex<FakeState>,
}

#[derive(Debug, Default)]
struct FakeState {
    /// 確定済みストア（path → content）。commit 成功時のみ更新される。
    store: std::collections::BTreeMap<std::path::PathBuf, String>,
    /// 次回 `commit` を失敗させる（原子性檻: 失敗時 `store` は変更しない）。
    fail_next_commit: bool,
    /// 次回 `read` を失敗させる。
    fail_next_read: bool,
}

impl FakePersistIo {
    /// 空ストアの fake を作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 次回の [`PersistIo::commit`] を 1 回だけ失敗させる（`store` は書き換えない）。
    ///
    /// 「途中で中断された書込」をシミュレートする。中断後 [`PersistIo::read`] は直前に確定した
    /// 内容を返す（既存内容の無傷を観測できる）。
    pub fn fail_next_commit(&self) {
        self.inner.lock().unwrap().fail_next_commit = true;
    }

    /// 次回の [`PersistIo::read`] を 1 回だけ失敗させる。
    pub fn fail_next_read(&self) {
        self.inner.lock().unwrap().fail_next_read = true;
    }
}

impl PersistIo for FakePersistIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        let mut state = self.inner.lock().unwrap();
        if state.fail_next_read {
            state.fail_next_read = false;
            return Err(std::io::Error::other(
                "FakePersistIo: injected read failure",
            ));
        }
        Ok(state.store.get(path).cloned())
    }

    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        let mut state = self.inner.lock().unwrap();
        if state.fail_next_commit {
            state.fail_next_commit = false;
            return Err(std::io::Error::other(
                "FakePersistIo: injected commit interruption",
            ));
        }
        state.store.insert(path.to_path_buf(), content.to_string());
        Ok(())
    }
}

#[cfg(test)]
#[path = "io_lock_tests.rs"]
mod lock_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use temp_path_kit::TempPath;

    // --- 実 I/O 往復（一時パスは共有の窓口 `TempPath` から取る＝プロセス間で一意・
    //     破棄で再帰削除。固定名だと同じテストの 2 プロセスが一時ファイルを奪い合う）---

    #[test]
    fn real_round_trip_and_overwrite() {
        let io = FsPersistIo;
        let dir = TempPath::new("sylphya-persist-io-round-trip");
        let path = dir.child("areka_sylphya_persist_io_round_trip.toml");

        io.commit(&path, "first").unwrap();
        assert_eq!(io.read(&path).unwrap().as_deref(), Some("first"));

        io.commit(&path, "second").unwrap();
        assert_eq!(io.read(&path).unwrap().as_deref(), Some("second"));
    }

    #[test]
    fn real_read_absent_is_ok_none() {
        let io = FsPersistIo;
        let dir = TempPath::new("sylphya-persist-io-absent");
        let path = dir.child("areka_sylphya_persist_io_absent_never_created.toml");
        assert_eq!(io.read(&path).unwrap(), None);
    }

    /// 宛先の親ディレクトリ（`<profile>/areka/`）が未作成でも commit は親を作ってから書き込み成功する。
    /// 事前に親 dir を作らない——`File::create` は親不在で NotFound(os error 3)。
    #[test]
    fn real_commit_creates_missing_parent_dirs() {
        let io = FsPersistIo;
        let root = TempPath::new("sylphya-missing-parent");
        let path = root.child("profile").join("areka").join("sylphya.toml");
        assert!(
            !path.parent().unwrap().exists(),
            "前提: 宛先の親 dir は存在しない"
        );

        io.commit(&path, "format-version = 1\n")
            .expect("親 dir 不在でも commit は成功する（create_dir_all 済み）");
        assert_eq!(
            io.read(&path).unwrap().as_deref(),
            Some("format-version = 1\n")
        );
    }

    // --- 原子性（fake IO・純 x64）---

    #[test]
    fn fake_commit_interrupt_leaves_prior_content_intact() {
        let io = FakePersistIo::new();
        let path = PathBuf::from("/scope/sylphya.toml");
        io.commit(&path, "A").unwrap();

        io.fail_next_commit();
        let err = io.commit(&path, "B");
        assert!(
            err.is_err(),
            "中断された commit は Err を返す（無音成功なし）"
        );
        assert_eq!(
            io.read(&path).unwrap().as_deref(),
            Some("A"),
            "既存内容は無傷"
        );
    }

    #[test]
    fn fake_read_failure_injection() {
        let io = FakePersistIo::new();
        let path = PathBuf::from("/scope/sylphya.toml");
        io.commit(&path, "A").unwrap();
        io.fail_next_read();
        assert!(io.read(&path).is_err(), "注入した read 失敗は Err で表面化");
        assert_eq!(
            io.read(&path).unwrap().as_deref(),
            Some("A"),
            "1 回限りの故障"
        );
    }

    #[test]
    fn fake_read_absent_is_ok_none() {
        let io = FakePersistIo::new();
        assert_eq!(io.read(&PathBuf::from("/nope.toml")).unwrap(), None);
    }

    /// `Arc` で包んでも 4 つの口（read・commit・withholds・lock_for_update）はすべて中身へ渡る。
    #[test]
    fn an_arc_forwards_all_four_methods() {
        struct Withholding(FakePersistIo);
        impl PersistIo for Withholding {
            fn lock_for_update(&self, _path: &Path) -> std::io::Result<Option<std::fs::File>> {
                // 開いた手なら何でもよい（既定の None と見分けられれば足りる）。
                std::fs::File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).map(Some)
            }
            fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
                self.0.read(path)
            }
            fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
                self.0.commit(path, content)
            }
            fn withholds(&self, _path: &Path) -> bool {
                true
            }
        }
        let inner = std::sync::Arc::new(Withholding(FakePersistIo::new()));
        let io: Box<dyn PersistIo> = Box::new(inner.clone());
        let path = PathBuf::from("/scope/sylphya.toml");

        io.commit(&path, "A").unwrap();
        assert_eq!(inner.read(&path).unwrap().as_deref(), Some("A"));
        assert_eq!(io.read(&path).unwrap().as_deref(), Some("A"));
        assert!(
            io.withholds(&path),
            "withholds も中身へ渡る（既定の false にならない）"
        );
        assert!(
            io.lock_for_update(&path).unwrap().is_some(),
            "lock_for_update も中身へ渡る（既定の None にならない）"
        );
    }
}
