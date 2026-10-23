//! 控える範囲と重ね書きの控え（読み取りだけの走り・design「PersistWithhold」）。
//!
//! - 範囲は「全部」（[`withhold_everything`]・本番の読み取りだけの走り・下ろす口は無い）か、
//!   試験が [`WithholdGuard::under`] で登録した根の下だけ。
//! - 控えはパスから中身へのメモリの中の表で、プロセスが終われば消える（要件 2.4）。
//! - このモジュールはファイルシステムに触れない（FS に触れるのは `FsPersistIo` だけ）。
//!
//! パスの鍵は小文字にし、区切りを `\` にそろえた文字列。根の下かの判定はパスの部品の単位で
//! 行う（根の鍵に `\` を足してから頭を比べる）。文字の頭の一致だけだと `…-1` の根が `…-10` を
//! 含み、並んで走る別の試験の書き込みを黙って控えてしまう。

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

struct Withhold {
    everything: bool,
    roots: Vec<(u64, String)>,
    next_id: u64,
    staged: BTreeMap<String, String>,
}

static STATE: Mutex<Withhold> = Mutex::new(Withhold {
    everything: false,
    roots: Vec::new(),
    next_id: 0,
    staged: BTreeMap::new(),
});

fn lock() -> std::sync::MutexGuard<'static, Withhold> {
    // 控えの表は短い操作だけで、途中で落ちても壊れた中身は残らない＝毒は無視してよい。
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

fn key(path: &Path) -> String {
    let k = path.to_string_lossy().to_lowercase().replace('/', "\\");
    k.trim_end_matches('\\').to_owned()
}

/// `k` が根の鍵 `root` そのものか、その下か（部品の単位）。
fn is_under(k: &str, root: &str) -> bool {
    k == root
        || k.strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('\\'))
}

/// 本番の読み取りだけの走り: 以後すべてのパスを控える（戻す口は無い・呼び手は areka の
/// `readonly::engage` だけ）。
pub fn withhold_everything() {
    lock().everything = true;
}

/// このパスを書かずに控えるか。
pub fn withholds(path: &Path) -> bool {
    let state = lock();
    if state.everything {
        return true;
    }
    let k = key(path);
    state.roots.iter().any(|(_, root)| is_under(&k, root))
}

/// 控えた中身（無ければ `None`）。
pub(crate) fn staged(path: &Path) -> Option<String> {
    lock().staged.get(&key(path)).cloned()
}

/// 中身を控えへ入れる（同じパスは上書き）。
pub(crate) fn stage(path: &Path, content: &str) {
    lock().staged.insert(key(path), content.to_owned());
}

/// 試験の根だけを控える取っ手（落とすと登録と、その根の下の控えを消す）。
#[must_use]
pub struct WithholdGuard {
    id: u64,
}

impl WithholdGuard {
    /// `root` の下を控える範囲に登録する。
    pub fn under(root: &Path) -> Self {
        let mut state = lock();
        let id = state.next_id;
        state.next_id += 1;
        state.roots.push((id, key(root)));
        Self { id }
    }
}

impl Drop for WithholdGuard {
    fn drop(&mut self) {
        let mut state = lock();
        let Some(pos) = state.roots.iter().position(|(id, _)| *id == self.id) else {
            return;
        };
        let (_, root) = state.roots.remove(pos);
        state.staged.retain(|k, _| !is_under(k, &root));
    }
}

#[cfg(test)]
#[path = "withhold_tests.rs"]
mod tests;
