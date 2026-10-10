//! 置き場所のファイルの読み書き（排他・読み・退避・置き換え書き・`status.md`）。
//!
//! 状態を変える呼び出しは全部 [`Store::with_state`] を通る。状態ファイルとは別のロックファイル
//! `state.lock` の排他の中で「読む → 判断 → 変わっていれば置き換え書き → 読み物」を行い、排他は
//! この読み書きの間だけ続く（机の保持は排他で表さない。要件 8.2）。
//!
//! - 読み: 版の数字だけを先に見る。無い → 空から始める。知らない版 → 読まず動かさず失敗。
//!   JSON として読めない・形が合わない → 時刻入りの別名へ退避して空から始める。
//! - 書き: プロセス番号入りの一時ファイルに全部書いて確定してから、名前を替えて置き換える
//!   （読む側はロック無しでも書きかけを見ない。要件 8.3）。
//!
//! 時計・生死・試しの間の待ちは欄に持ち、テストは差し替える。

// 使い手（wait・cli）が載るまで、本番のビルドではここが未使用になる。
// 「満たされない expect」の警告が出たら外す。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "使い手のモジュールは後のタスクで載る")
)]

use std::fs::{self, File, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::error::WatchError;
use crate::home::Home;
use crate::plan::{Applied, Presence};
use crate::presence::LockFilePresence;
use crate::state::{Recent, RecentKind, State, VERSION};
use crate::status;

/// `state.lock` を試す間隔。
const LOCK_PAUSE: Duration = Duration::from_millis(10);
/// `state.lock` を待つ上限。待とうとした時間の合計で数える（時計は見ない）。
const LOCK_LIMIT: Duration = Duration::from_secs(10);
/// 置き換え（名前の付け替え）を試す回数。読む側がロック無しで開いている一瞬のため。
const REPLACE_TRIES: u32 = 5;
/// 置き換えの試しの間の待ち。
const REPLACE_PAUSE: Duration = Duration::from_millis(20);

/// 置き場所のファイルへの口。
pub struct Store {
    home: Home,
    /// いまの時刻（UNIX 秒）。
    clock: Box<dyn Fn() -> u64>,
    presence: Box<dyn Presence>,
    /// 試しの間の待ち（本物は眠る）。
    pause: Box<dyn Fn(Duration)>,
}

/// 本物の時計・ロックファイルの生死・本物の眠りで口を開く。ファイルにはまだ触らない。
pub fn open(home: Home) -> Result<Store, WatchError> {
    let presence = LockFilePresence::new(&home);
    Ok(Store {
        home,
        clock: Box::new(unix_now),
        presence: Box::new(presence),
        pause: Box::new(std::thread::sleep),
    })
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// 読みで状態を空から作り直した訳（要件 8.4・8.5）。
enum Recovery {
    /// 状態ファイルが無かった。
    Created,
    /// 状態ファイルが読めなかった・形が合わなかった。中身は退避先の道筋。
    BackedUp(PathBuf),
}

/// 握っている `state.lock`。落とすと解ける（ファイルは残る）。
struct StateLock(File);

impl Drop for StateLock {
    fn drop(&mut self) {
        // ハンドルを閉じるだけでも解けるが、解ける時機を OS 任せにしない。
        let _ = self.0.unlock();
    }
}

impl Store {
    /// 状態を変える 1 回。排他の中で 読む → `f`（判断）→ 変わっていれば置き換え書き → 読み物。
    ///
    /// `f` へ渡すのは、読んだ状態・いまの時刻・生死の口。`f` の返す 2 つ目の値がそのまま返る。
    /// どの失敗もログに残してから返す。
    pub fn with_state<T>(
        &self,
        f: impl FnOnce(&mut State, u64, &dyn Presence) -> (Applied, T),
    ) -> Result<T, WatchError> {
        self.change(f)
            .inspect_err(|err| tracing::error!(error = %err, "[store] state change failed"))
    }

    fn change<T>(
        &self,
        f: impl FnOnce(&mut State, u64, &dyn Presence) -> (Applied, T),
    ) -> Result<T, WatchError> {
        let _lock = self.lock()?;
        let now = (self.clock)();
        let (mut state, recovery) = self.load(now)?;
        let (applied, out) = f(&mut state, now, self.presence.as_ref());
        // 書くかどうかは `changed` だけで決める（当てはまらなかった呼び出しでも回収の分は
        // 変わっている）。作り直した状態は、判断が何も変えなくても書く（作り直しの記録を
        // 1 度だけ残す。書かなければ次の呼び出しがまた「無かった」と記録する）。
        if applied.changed || recovery.is_some() {
            self.save(&state, now)?;
        }
        Ok(out)
    }

    /// 待ちの読み直し用: 状態ファイルの（更新時刻, 大きさ）。無ければ `None`。中身は読まない。
    pub fn fingerprint(&self) -> io::Result<Option<(SystemTime, u64)>> {
        match fs::metadata(self.home.state_path()) {
            Ok(meta) => Ok(Some((meta.modified()?, meta.len()))),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// `state.lock` を排他で握る。短い間隔で試し、上限まで取れなければ `LockBusy`。
    fn lock(&self) -> Result<StateLock, WatchError> {
        // 中身は使わない。在るファイルはそのまま開く。
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.home.lock_path())
            .map_err(|err| WatchError::io("open state.lock", &err))?;
        let mut waited = Duration::ZERO;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(StateLock(file)),
                Err(TryLockError::WouldBlock) if waited < LOCK_LIMIT => {
                    (self.pause)(LOCK_PAUSE);
                    waited += LOCK_PAUSE;
                }
                Err(TryLockError::WouldBlock) => return Err(WatchError::LockBusy),
                Err(TryLockError::Error(err)) => {
                    return Err(WatchError::io("lock state.lock", &err));
                }
            }
        }
    }

    /// 状態ファイルを読む。作り直したときは、その訳を「最近の出来事」に足して一緒に返す。
    fn load(&self, now: u64) -> Result<(State, Option<Recovery>), WatchError> {
        let bytes = match fs::read(self.home.state_path()) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                tracing::info!("[store] no state file; started a new one");
                return Ok(recovered(now, Recovery::Created));
            }
            // 開けない・読めないという OS の失敗は「壊れている」と見なさない（退避で動かさない）。
            Err(err) => return Err(WatchError::io("read state.json", &err)),
        };
        // 版の数字だけを先に見る。知らない版のファイルは、形を読みも動かしもしない。
        // 版の数字が無いもの（どの版の実行ファイルも書かない）は、形の合わないファイル。
        let value = serde_json::from_slice::<Value>(&bytes).ok();
        let version = value
            .as_ref()
            .and_then(|value| value.get("version"))
            .and_then(Value::as_u64);
        if let Some(found) = version.filter(|found| *found != u64::from(VERSION)) {
            return Err(WatchError::VersionMismatch {
                found,
                known: VERSION,
            });
        }
        let state = version
            .and(value)
            .and_then(|value| serde_json::from_value::<State>(value).ok());
        if let Some(state) = state {
            return Ok((state, None));
        }
        let backup = self.set_aside_broken(now)?;
        tracing::warn!(
            backup = %backup.display(),
            "[store] state file was broken; set aside and started a new one"
        );
        Ok(recovered(now, Recovery::BackedUp(backup)))
    }

    /// 壊れた状態ファイルを `state.json.broken-<UTC>` へ動かし、その道筋を返す。
    /// 同じ秒の退避がすでに在れば番号を足す（前の退避を上書きしない）。
    fn set_aside_broken(&self, now: u64) -> Result<PathBuf, WatchError> {
        let name = format!("state.json.broken-{}", status::utc_compact(now));
        let mut backup = self.home.dir.join(&name);
        let mut serial = 0;
        while backup.exists() {
            serial += 1;
            backup = self.home.dir.join(format!("{name}-{serial}"));
        }
        fs::rename(self.home.state_path(), &backup)
            .map_err(|err| WatchError::io("set aside state.json", &err))?;
        Ok(backup)
    }

    /// 状態ファイルを書き、続けて読み物を書く。
    fn save(&self, state: &State, now: u64) -> Result<(), WatchError> {
        let json = serde_json::to_vec_pretty(state)?;
        self.replace(&self.home.state_path(), &json, "write state.json")?;
        let text = status::render_markdown(state, self.presence.as_ref(), now);
        self.replace(&self.home.status_path(), text.as_bytes(), "write status.md")
    }

    /// 置き換え書き。一時ファイル `<名前>.<プロセス番号>.tmp` に全部書いて確定してから、
    /// 名前を替えて置き換える。失敗したら一時ファイルを消す（元のファイルは無傷）。
    fn replace(&self, path: &Path, bytes: &[u8], op: &'static str) -> Result<(), WatchError> {
        let mut name = path.file_name().unwrap_or_default().to_owned();
        name.push(format!(".{}.tmp", std::process::id()));
        let temp = path.with_file_name(name);
        write_synced(&temp, bytes)
            .and_then(|()| self.rename_retrying(&temp, path))
            .map_err(|err| {
                let _ = fs::remove_file(&temp);
                WatchError::io(op, &err)
            })
    }

    /// 名前の付け替えを [`REPLACE_TRIES`] 回まで試す。
    fn rename_retrying(&self, from: &Path, to: &Path) -> io::Result<()> {
        for _ in 1..REPLACE_TRIES {
            if fs::rename(from, to).is_ok() {
                return Ok(());
            }
            (self.pause)(REPLACE_PAUSE);
        }
        fs::rename(from, to)
    }
}

/// 空から作り直した状態（訳の記録 1 件つき）。
fn recovered(now: u64, recovery: Recovery) -> (State, Option<Recovery>) {
    let detail = match &recovery {
        Recovery::Created => "created".to_owned(),
        Recovery::BackedUp(backup) => format!("backed up: {}", backup.display()),
    };
    let mut state = State::empty();
    state.push_recent(Recent {
        at: now,
        kind: RecentKind::Recovered,
        id: None,
        detail,
    });
    (state, Some(recovery))
}

/// 新しいファイルに全部書き、ディスクへ確定する。
fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
