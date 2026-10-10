//! 置き場所のファイルの読み書き（排他・読み・退避・置き換え書き・`status.md`・ログ）。
//!
//! 状態を変える呼び出しは [`Store::with_state`] か [`Store::clear`] を通る。状態ファイルとは別の
//! ロックファイル `state.lock` の排他の中で「読む → 判断 → 変わっていれば置き換え書き → 読み物」を
//! 行い、排他はこの読み書きの間だけ続く（机の保持は排他で表さない。要件 8.2）。
//! [`Store::read_only`] は排他を取らず、何も書かない。
//!
//! - 読み: 版の数字だけを先に見る。無い → 空から始める。知らない版 → 読まず動かさず失敗。
//!   JSON として読めない・形が合わない → 時刻入りの別名へ退避して空から始める。
//! - 書き: プロセス番号入りの一時ファイルに全部書いて確定してから、名前を替えて置き換える
//!   （読む側はロック無しでも書きかけを見ない。要件 8.3）。
//! - ログ: 置き場所の下の `impl-watch.log` へ追記する。状態の変化は出来事 1 件ずつログの口へ
//!   渡し（本物は 1 行ずつ書く）、失敗は各入口の出口で `error!` を 1 行出してから返す。
//!   渡すのは、状態ファイルの置き換え（か退避）が済んだ変化だけ。読み物を書くのはその後。
//!
//! 時計・生死・試しの間の待ち・ログの口は欄に持ち、テストは差し替える。

// 使い手（wait・cli）が載るまで、本番のビルドではここが未使用になる。
// 「満たされない expect」の警告が出たら外す。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "使い手のモジュールは後のタスクで載る")
)]

use std::fs::{self, File, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::error::WatchError;
use crate::home::Home;
use crate::plan::{Applied, Event, Presence};
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
/// 状態ファイルが無いときの `clear` が、「消した記録」の退避先の欄に書く綴り（端末へ出す
/// `backup: none` も同じ綴り）。
pub(crate) const NO_BACKUP: &str = "none";

/// 置き場所のファイルへの口。
pub struct Store {
    home: Home,
    /// いまの時刻（UNIX 秒）。
    clock: Box<dyn Fn() -> u64>,
    presence: Box<dyn Presence>,
    /// 試しの間の待ち（本物は眠る）。
    pause: Box<dyn Fn(Duration)>,
    /// ログの口。状態の変化を出来事 1 件ずつ受ける（本物は 1 行ずつ書く）。
    log: Box<dyn Fn(&Event)>,
}

/// 本物の時計・ロックファイルの生死・本物の眠り・本物のログで口を開く。触るのはログのファイル
/// だけで、状態ファイルにはまだ触らない。
///
/// `command` はこのプロセスが受けたコマンドの名前（ASCII）で、ログの行に載る。ログの受け手が
/// 据わるのはプロセスで最初の 1 度だけ（2 度目からの `open` は据え直さず、行は最初に開いた
/// 置き場所の `impl-watch.log` へ行く）。
pub fn open(home: Home, command: &'static str) -> Result<Store, WatchError> {
    let file = File::options()
        .create(true)
        .append(true)
        .open(home.log_path())
        .map_err(|err| WatchError::io("open impl-watch.log", &err))?;
    // 返る失敗は「もう据わっている」だけなので捨てる。
    let _ = tracing_subscriber::fmt()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .try_init();
    let presence = LockFilePresence::new(&home);
    Ok(Store {
        home,
        clock: Box::new(unix_now),
        presence: Box::new(presence),
        pause: Box::new(std::thread::sleep),
        log: Box::new(move |event| write_line(command, event)),
    })
}

/// 本物のログの口: 出来事 1 件を 1 行にする（時刻は受け手が付ける）。識別と変化の中身は
/// 出来事の綴りに在る。
fn write_line(command: &'static str, event: &Event) {
    if matches!(event, Event::Recovered { backup: Some(_) }) {
        tracing::warn!(command, event = ?event, "[store] state file was broken; set aside");
    } else {
        tracing::info!(command, event = ?event, "[store] state changed");
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
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
    /// 状態ファイルを置き換えた時点で、判断の返した出来事を 1 件ずつログの口へ渡す（状態
    /// ファイルが書けなかった変化は起きていないので渡さない）。読み物を書くのはその後で、
    /// 読み物が書けなくても、起きた変化はログに残る。どの失敗もログに残してから返す。
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
        let (mut state, rebuilt) = self.load(now)?;
        // 退避はもう起きたことなので、この後の書きを待たずにログへ渡す。
        // 「無かったので作った」は、状態ファイルが書けてから渡す。
        let set_aside = matches!(rebuilt, Some(Event::Recovered { backup: Some(_) }));
        if set_aside {
            rebuilt.iter().for_each(&self.log);
        }
        let (applied, out) = f(&mut state, now, self.presence.as_ref());
        // 書くかどうかは `changed` だけで決める（当てはまらなかった呼び出しでも回収の分は
        // 変わっている）。作り直した状態は、判断が何も変えなくても書く（作り直しの記録を
        // 1 度だけ残す。書かなければ次の呼び出しがまた「無かった」と記録する）。
        let write = applied.changed || rebuilt.is_some();
        if write {
            self.write_state(&state)?;
        }
        // 状態ファイルを置き換えた時点で変化は起きている。読み物が書けなくても記録が残るよう、
        // 読み物より先に渡す。
        let created = rebuilt.iter().filter(|_| !set_aside);
        created.chain(&applied.events).for_each(&self.log);
        if write {
            self.write_status(&state, now)?;
        }
        Ok(out)
    }

    /// 読むだけ。排他を取らず、作らず、退避もせず、読み物も書かない。
    /// 無ければ `None`、読めない・形が合わなければ `Broken`、知らない版なら `VersionMismatch`。
    pub fn read_only(&self) -> Result<Option<State>, WatchError> {
        let read = || match self.read()? {
            Some(bytes) => parse(&bytes)?.map(Some).ok_or(WatchError::Broken),
            None => Ok(None),
        };
        read().inspect_err(|err| tracing::error!(error = %err, "[store] read failed"))
    }

    /// 状態の確認。排他を取らずに読み（[`Store::read_only`]）、読み物を置き換え、端末向けの
    /// 要約（ASCII）を返す。状態ファイルには触らない。状態ファイルが無ければ `None`（読み物も
    /// 書かない）。
    pub fn summary(&self) -> Result<Option<String>, WatchError> {
        let Some(state) = self.read_only()? else {
            return Ok(None);
        };
        self.write_status(&state, (self.clock)())
            .inspect_err(|err| tracing::error!(error = %err, "[store] status.md not written"))?;
        let summary = status::render_terminal(&state, self.presence.as_ref());
        Ok(Some(summary))
    }

    /// 全部消す。排他の中で今の状態ファイルを `state.json.cleared-<UTC>` へ退避し、「消した記録」
    /// だけの空の状態を書く。退避先を返す（状態ファイルが無ければ `None`。空の状態は書く）。
    pub fn clear(&self) -> Result<Option<PathBuf>, WatchError> {
        self.wipe()
            .inspect_err(|err| tracing::error!(error = %err, "[store] clear failed"))
    }

    fn wipe(&self) -> Result<Option<PathBuf>, WatchError> {
        let _lock = self.lock()?;
        let now = (self.clock)();
        // 今のファイルは読まない（壊れていても、知らない版でも、退避して空からやり直せる）。
        let backup = self.set_aside("cleared", now)?;
        (self.log)(&Event::Cleared {
            backup: backup.as_deref().map(lossy),
        });
        let recorded = backup.as_deref().unwrap_or(Path::new(NO_BACKUP));
        let state = State::cleared(now, recorded);
        self.write_state(&state)?;
        self.write_status(&state, now)?;
        Ok(backup)
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

    /// 状態ファイルの中身。無ければ `None`。
    fn read(&self) -> Result<Option<Vec<u8>>, WatchError> {
        match fs::read(self.home.state_path()) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            // 開けない・読めないという OS の失敗は「壊れている」と見なさない（退避で動かさない）。
            Err(err) => Err(WatchError::io("read state.json", &err)),
        }
    }

    /// 状態ファイルを読む。2 つ目の値は、空から作り直した（無かった・読めなかった）ときの
    /// 「作り直した」の出来事で、ログの口へ渡すのは呼び手。作り直したときは、その訳を
    /// 「最近の出来事」に足す（要件 8.4・8.5）。
    fn load(&self, now: u64) -> Result<(State, Option<Event>), WatchError> {
        let backup = match self.read()? {
            Some(bytes) => match parse(&bytes)? {
                Some(state) => return Ok((state, None)),
                // 読めない・形が合わないファイルは退避する。
                None => self.set_aside("broken", now)?,
            },
            None => None,
        };
        let event = Event::Recovered {
            backup: backup.as_deref().map(lossy),
        };
        let mut state = State::empty();
        state.push_recent(Recent {
            at: now,
            kind: RecentKind::Recovered,
            id: None,
            detail: match backup {
                Some(backup) => format!("backed up: {}", backup.display()),
                None => "created".to_owned(),
            },
        });
        Ok((state, Some(event)))
    }

    /// 状態ファイルを `state.json.<札>-<UTC>` へ動かし、その道筋を返す。無ければ `None`。
    /// 同じ秒の退避がすでに在れば番号を足す（前の退避を上書きしない）。
    fn set_aside(&self, tag: &str, now: u64) -> Result<Option<PathBuf>, WatchError> {
        let name = format!("state.json.{tag}-{}", status::utc_compact(now));
        let mut backup = self.home.dir.join(&name);
        let mut serial = 0;
        while backup.exists() {
            serial += 1;
            backup = self.home.dir.join(format!("{name}-{serial}"));
        }
        match fs::rename(self.home.state_path(), &backup) {
            Ok(()) => Ok(Some(backup)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(WatchError::io("set aside state.json", &err)),
        }
    }

    /// 状態ファイルを置き換える。
    fn write_state(&self, state: &State) -> Result<(), WatchError> {
        let json = serde_json::to_vec_pretty(state)?;
        self.replace(&self.home.state_path(), &json, "write state.json")
    }

    /// 読み物を置き換える。
    fn write_status(&self, state: &State, now: u64) -> Result<(), WatchError> {
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

/// 状態ファイルの中身を状態として読む。JSON として読めない・形が合わないなら `None`。
///
/// 版の数字だけを先に見る。知らない版のファイルは、形を読まずに失敗にする。
/// 版の数字が無いもの（どの版の実行ファイルも書かない）は、形の合わないファイル。
fn parse(bytes: &[u8]) -> Result<Option<State>, WatchError> {
    let value = serde_json::from_slice::<Value>(bytes).ok();
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
    Ok(version
        .and(value)
        .and_then(|value| serde_json::from_value(value).ok()))
}

fn lossy(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// 新しいファイルに全部書き、ディスクへ確定する。
fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(test)]
#[path = "store_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "store_ports_tests.rs"]
mod ports_tests;
