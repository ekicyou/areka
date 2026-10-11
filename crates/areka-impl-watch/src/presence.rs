//! ロックファイルの「握る」と「探る」（「居る」の印の唯一の実装）。
//!
//! 印は `alive/<id>.<kind>.lock` の排他ロックそのもの。ロックはプロセスが死ねば OS が解くので、
//! プロセス番号は判定に使わない（番号の使い回しに左右されない。要件 7.4）。
//! 握るのは排他、探るのは共有のロックの試し（探り同士は互いを妨げない）。
//!
//! 不変:
//! - ロックファイルは消さない（Windows では握られているファイルの消去と作り直しが衝突する）。
//!   ファイルが在るかではなく、ロックが掛かっているかだけを見る。
//! - 探り（[`LockFilePresence`]）はファイルもフォルダも作らない（`status` が読むだけで済む）。
//!   `alive/` を作るのは [`hold`] だけ。

use std::fs::{File, TryLockError};
use std::io;
use std::path::Path;
use std::time::Duration;

use crate::home::Home;
use crate::plan::Presence;
use crate::state::WaitKind;

/// 握りを試す回数。探りが一瞬（共有の）ロックを取るので、同時に始まった握りの 1 回目は外れることがある。
const HOLD_TRIES: u32 = 5;
/// 試しの間の待ち。
const HOLD_PAUSE: Duration = Duration::from_millis(20);

/// 握っているロックファイル。落とすとロックが解ける（ファイルは残る）。
#[derive(Debug)]
pub struct Held {
    file: File,
}

impl Drop for Held {
    fn drop(&mut self) {
        // ハンドルを閉じるだけでも解けるが、解ける時機を OS 任せにしない。
        let _ = self.file.unlock();
    }
}

/// ロックファイルを作って排他で握る。`None` = 他のプロセス（ハンドル）が握っている。
///
/// 親のフォルダ（`alive/`）が無ければ作る。開けない・ロックの失敗は `Err`（`None` に化かさない）。
pub fn hold(path: &Path) -> io::Result<Option<Held>> {
    hold_retrying(path, || std::thread::sleep(HOLD_PAUSE))
}

/// [`hold`] の芯。試しの間の待ちを引数で受ける（テストは眠らずに回数だけを数える）。
fn hold_retrying(path: &Path, mut pause: impl FnMut()) -> io::Result<Option<Held>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // 中身は使わない。在るファイルはそのまま開く。
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    for tried in 1..=HOLD_TRIES {
        match file.try_lock() {
            Ok(()) => return Ok(Some(Held { file })),
            Err(TryLockError::WouldBlock) if tried < HOLD_TRIES => pause(),
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(err)) => return Err(err),
        }
    }
    Ok(None)
}

/// ロックファイルを探って「居る」かを答える、生死の口の本物の実装。
pub struct LockFilePresence {
    // `<id>.<kind>.lock` の綴りを `Home::alive_path` の 1 か所に保つため、置き場所ごと持つ。
    home: Home,
}

impl LockFilePresence {
    pub fn new(home: &Home) -> Self {
        Self {
            home: Home {
                dir: home.dir.clone(),
            },
        }
    }
}

impl Presence for LockFilePresence {
    /// 作らずに開き、無ければ「居ない」。共有のロックが取れたら直ちに解いて「居ない」、
    /// 取れなければ（排他で握られている）「居る」。共有で探るのは、同時に走った 2 つの探りが
    /// 互いのロックを見て「居る」と見誤らないため。
    ///
    /// 「無い」でも「握られている」でもない失敗（アクセス拒否など）は「居る」に倒す。
    /// 「居ない」に倒すと生きている参加者が回収されて机を失い（要件 7.2）、「居る」に倒した
    /// ときの害は回収が遅れるだけで、開発者が手で外せる（要件 7.6）。
    fn is_present(&self, id: &str, kind: WaitKind) -> bool {
        let file = match File::open(self.home.alive_path(id, kind)) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return false,
            Err(err) => return unsure(id, kind, &err),
        };
        match file.try_lock_shared() {
            Ok(()) => {
                let _ = file.unlock();
                false
            }
            Err(TryLockError::WouldBlock) => true,
            Err(TryLockError::Error(err)) => unsure(id, kind, &err),
        }
    }
}

/// 探りが確かめられなかったときの答え（「居る」）。黙って倒さず、ログに残す。
fn unsure(id: &str, kind: WaitKind, err: &io::Error) -> bool {
    tracing::warn!(id, kind = ?kind, error = %err, "[presence] probe failed; treated as present");
    true
}

#[cfg(test)]
#[path = "presence_tests.rs"]
mod tests;
