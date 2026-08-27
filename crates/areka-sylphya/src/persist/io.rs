//! 永続 IO シーム（`PersistIo`）: 実 FS の原子的確定と、決定論テスト向け故障注入 fake。
//!
//! この層は「1 スコープの `sylphya.toml` を 1 本の文字列として往復する」だけの薄い境界で
//! あり、TOML スキーマ（[`super::format`]）にも鏡像にも依存しない（content は不透明 `String`）。
//! **areka-sylphya でファイルシステムに触れるのはこのモジュールだけ**（design「IO はすべて
//! このシーム経由」・R8.3）。暦時計・OS ユーザー名等の非決定な外部環境は一切読まない
//! （R8.3。fake は外部環境を必要とせず・real は temp 名に PID/時刻等を混ぜない）。
//!
//! ## 原子的確定（R6.2・design「永続層 → Responsibilities」）
//!
//! [`FsPersistIo::commit`] は **同一ディレクトリ内の一時ファイルへ全書込→`fsync`→`rename`**
//! で確定する。宛先と同一ボリューム上の temp を使うため `rename` は原子的で、Windows でも
//! 既存宛先を置換する（`MoveFileEx` の REPLACE 相当・研究 §11-2）。temp 書込・rename の
//! いずれが失敗しても宛先の既存ファイルは無傷であり（本体を直接 truncate しない）、失敗は
//! 必ず `Err` で表面化する（無音成功なし・R8.1）。temp は失敗時も可能な限り掃除する。
//!
//! ## 決定論檻（R9.2）
//!
//! [`FakePersistIo`] はメモリ内ストア（`Mutex<BTreeMap<PathBuf, String>>`）で、
//! [`FakePersistIo::fail_next_commit`] / [`FakePersistIo::fail_next_read`] により次回操作を
//! 決定論的に失敗させられる。原子性の観測条件——「commit 中断後も既存内容が無傷」——は
//! この fake で純 x64 検証する（実 FS 障害を注入せずに済む）。

use std::io::Write;
use std::path::Path;

/// 永続 IO の注入シーム（design「永続層 → Service Interface」）。
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
}

/// 本番 IO 実装（`std::fs` による temp 書込→`rename` の原子的確定）。
///
/// areka-sylphya でファイルシステムに触れる唯一の型（R8.3）。暦時計・OS 環境は読まない。
#[derive(Clone, Copy, Debug, Default)]
pub struct FsPersistIo;

impl FsPersistIo {
    /// `path` と同一ディレクトリに置く一時ファイル名を組む（同一ボリューム＝rename 原子性の前提）。
    ///
    /// 非決定源（時刻・乱数・PID）を混ぜない（R8.3）。M1 は単一プロセス前提（design「Risks:
    /// 複数プロセス同時 open の write 競合は M1 想定外」）ゆえ、宛先ファイル名から決定論的に
    /// 導いた固定サフィックスで十分。競合が必要になれば PersistIo 拡張でロックを導入する。
    fn temp_path_for(path: &Path) -> std::path::PathBuf {
        let file_name = path
            .file_name()
            .map(|n| n.to_owned())
            .unwrap_or_else(|| std::ffi::OsString::from("sylphya"));
        let mut temp_name = file_name;
        temp_name.push(".tmp");
        path.with_file_name(temp_name)
    }
}

impl PersistIo for FsPersistIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            // 不在はエラーではなく `None`（design「read: 不在 → Ok(None)」）。
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            // 真の IO 障害（権限等）のみ Err で表面化（無音失敗なし・R8.1）。
            Err(e) => Err(e),
        }
    }

    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        // 宛先の親ディレクトリを保証する（新規ゴーストで `<profile>/areka/` が未作成でも書ける）。
        // 実機初回起動の恒久修正: これが無いと `File::create` が親不在で NotFound(os error 3) を返し、
        // 全永続書込が Degraded に倒れて位置・起動記録が保存されない。create_dir_all は既存 dir に対し
        // 冪等（No-op）ゆえ通常経路に影響しない。
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let temp = Self::temp_path_for(path);

        // temp へ全書込→flush→fsync。失敗したら temp を掃除して Err（本体は未変更＝無傷）。
        let write_result = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
            file.sync_all()?;
            Ok(())
        })();
        if let Err(e) = write_result {
            let _ = std::fs::remove_file(&temp); // best-effort cleanup（失敗は無視・本体は無傷）。
            return Err(e);
        }

        // 原子的置換。Windows の rename は既存宛先を置換する（研究 §11-2）。失敗時も
        // 本体（既存ファイル）は無傷——temp を掃除して Err を返す。
        if let Err(e) = std::fs::rename(&temp, path) {
            let _ = std::fs::remove_file(&temp);
            return Err(e);
        }
        Ok(())
    }
}

// --- テスト支援（決定論檻・R9.2）: 後続 Task 4.3/4.4 の他モジュールテストも再利用するため
//     `#[cfg(test)]` に閉じず `pub` 型として提供する（テスト支援であることは rustdoc で明示）。---

/// 決定論テスト向けのインメモリ `PersistIo`（故障注入可能）。**テスト支援専用**。
///
/// 本番経路では使わない（本番は [`FsPersistIo`]）。原子性の観測条件——commit を途中で
/// 中断させても既存内容が無傷であること（R6.2）——を実 FS 障害なしに純 x64 で檻に入れる
/// ために存在する。後続タスク（4.3 永続 orchestration・4.4）の他モジュールテストからも
/// 再利用できるよう、`#[cfg(test)]` ではなく通常の `pub` 型として公開する（design「テスト
/// 用に故障注入可能な fake」）。
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
    /// 内容を返す（既存内容の無傷を観測できる＝原子性・R6.2）。
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
            // 原子性の核心: 失敗時は store を一切変更しない（既存内容が無傷）。
            return Err(std::io::Error::other(
                "FakePersistIo: injected commit interruption",
            ));
        }
        state.store.insert(path.to_path_buf(), content.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use temp_path_kit::TempPath;

    // --- 実 I/O 往復（一時パスは共有の窓口 `TempPath` から取る＝プロセス間で一意・
    //     破棄で再帰削除。固定名だと同じテストの 2 プロセスが一時ファイルを奪い合う）---
    // real 実装は暦時計・OS 環境を暗黙に直読しない（R8.3）——temp 名は宛先から決定論的に導く。

    #[test]
    fn real_round_trip_and_overwrite() {
        let io = FsPersistIo;
        // 窓口が配るディレクトリは呼び出しごとに別物なので、その下に取る宛先は他プロセスと
        // 衝突しない。事前の削除は要らない（`child` はパスだけ返して実体を作らない）。
        let dir = TempPath::new("sylphya-persist-io-round-trip");
        let path = dir.child("areka_sylphya_persist_io_round_trip.toml");

        io.commit(&path, "first").unwrap();
        assert_eq!(io.read(&path).unwrap().as_deref(), Some("first"));

        // 既存宛先を rename-replace で上書き（Windows の置換 rename を実プラットフォームで実証）。
        io.commit(&path, "second").unwrap();
        assert_eq!(io.read(&path).unwrap().as_deref(), Some("second"));
        // 後始末は `dir` の破棄に委ねる（呼び忘れの余地を残さない）。
    }

    #[test]
    fn real_read_absent_is_ok_none() {
        let io = FsPersistIo;
        // このテストの前提は宛先が**存在しない**こと。`child` はパスだけ返して実体を作らない
        // ので、前提は窓口の性質からそのまま成り立つ（事前の削除は要らない）。
        let dir = TempPath::new("sylphya-persist-io-absent");
        let path = dir.child("areka_sylphya_persist_io_absent_never_created.toml");
        assert_eq!(io.read(&path).unwrap(), None);
    }

    /// 新規ゴースト回帰: 宛先の親ディレクトリ（`<profile>/areka/`）が未作成でも commit は
    /// 親を `create_dir_all` してから書き込み成功する（実機初回起動 os error 3 の恒久修正）。
    /// 檻の要点: 事前に親 dir を作らない——現行 `File::create` は親不在で NotFound(os error 3)。
    #[test]
    fn real_commit_creates_missing_parent_dirs() {
        let io = FsPersistIo;
        // 窓口が配るディレクトリの下に未作成の 2 段ネスト（宛先の親が存在しない状況を実 FS で
        // 再現）。`child`／`join` は実体を作らないので親不在は構成から保証され、前回残骸の掃除も
        // 要らない（窓口の名前は呼び出しごとに別物）。
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
        // 後始末は `root` の破棄に委ねる。
    }

    // --- 原子性の檻（fake IO・純 x64・R6.2）---

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

    #[test]
    fn fake_is_deterministic_given_same_faults() {
        let path = PathBuf::from("/scope/sylphya.toml");
        let run = || {
            let io = FakePersistIo::new();
            io.commit(&path, "A").unwrap();
            io.fail_next_commit();
            let interrupted = io.commit(&path, "B").is_err();
            (interrupted, io.read(&path).unwrap())
        };
        assert_eq!(run(), run());
    }

    // fake は外部環境（暦時計・OS 名・実 FS）を一切必要としない（R8.3・R9.2 の純 x64 檻）。
}
