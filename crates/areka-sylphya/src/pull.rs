//! 読まれた時に外の一次情報源から値を取りに行く口（プル・spec mcp-scope-rect-property 要件 7.1〜7.4）。
//!
//! sylphya は読み手の中身（OS・窓の印）を知らない。表は「名前の形 → 読み手」だけで、
//! 読み手は外（areka の側）から登録される。読み手の答えは写しにも保存にも書かない。

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::key::{KeyParseError, PropPath, Selector, parse_dotted};

/// 読まれた時に、外の一次情報源から値を取りに行く読み手（areka の側が実装して登録する）。
///
/// 約束（要件 5.1・7.3）: 窓を持つスレッドやほかのスレッドの返事を待たず、遅れもしない読み取りだけを
/// 行う。メッセージを送る OS の関数・チャネルの受け取り・他のアクターへの問い合わせを使わない。
pub trait PullSource: Send + Sync {
    /// 聞かれた名前（形が登録と合ったもの）に答える。`None` は値なし。
    fn read(&self, path: &PropPath) -> Option<String>;
}

/// 読み手を登録する名前の形（選択子を `(*)` に寄せた正準の文字）。
///
/// `currentghost.scope(0).rect` も `currentghost.scope(*).rect` も `currentghost.scope(*).rect`。
/// 選択子の無い区切りはそのまま（`currentghost.scope.rect` は別の形）。英字は
/// [`parse_dotted`] が小文字に畳んだもの。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PullShape(String);

impl PullShape {
    /// 名前または形の文字から形を作る（`parse_dotted` の誤りはそのまま返す）。
    pub fn parse(pattern: &str) -> Result<PullShape, KeyParseError> {
        parse_dotted(pattern).map(|path| PullShape::of(&path))
    }

    /// 読んだ名前の形（引く時に使う）。
    pub fn of(path: &PropPath) -> PullShape {
        let mut wild = path.clone();
        for seg in &mut wild.segs {
            if seg.selector.is_some() {
                seg.selector = Some(Selector::ByName("*".to_string()));
            }
        }
        PullShape(wild.to_canonical_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 形 → 読み手の表（読み口の複製どうしで共有する）。表を書くのは登録の時だけ。
// task 1.2 で SylphyaReader が持つまでは crate の中のテストだけが使う。
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Default)]
pub(crate) struct PullRegistry {
    inner: Arc<RwLock<BTreeMap<PullShape, Arc<dyn PullSource>>>>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl PullRegistry {
    /// 同じ形への 2 度目の登録は前の読み手を置き換える（実行系の作り直しで登録し直すため）。
    pub(crate) fn register(&self, shape: PullShape, source: Arc<dyn PullSource>) {
        // poison でも表は常に整合な値なので中身で続ける（`SharedMirror::load` と同じ）。
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        guard.insert(shape, source);
    }

    /// 読み手の `Arc` を写して鍵をすぐ外す（呼ぶのは鍵の外・読み手から読み口を呼び戻しても止まらない）。
    pub(crate) fn find(&self, path: &PropPath) -> Option<Arc<dyn PullSource>> {
        let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
        guard.get(&PullShape::of(path)).cloned()
    }
}

#[cfg(test)]
#[path = "pull_tests.rs"]
mod tests;
