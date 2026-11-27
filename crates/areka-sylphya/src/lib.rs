//! # areka-sylphya — 名前で引ける値の KVS（番人のアクター・鍵の表・3 つの住み場所・4 つの層）
//!
//! 鍵の表（[`keytable`]）が鍵ごとの住み場所（速い・メモリー・ファイル）と名前で書けるかを決める。
//! 番人（[`spawn_sylphya`]）はゴーストごとに 1 本で住み場所の核を持ち、読み口（[`reader`]）と
//! KVS の口（[`kvs`]）は番人へ聞く。速い住み場所（[`fast`]）だけは番人を通らずに読む。
//! 番人が居ない時点は同期の口（[`KvsDirect`]）を使う。

pub mod actor;
pub mod asker;
pub mod fast;
mod guardian;
pub mod key;
pub mod keytable;
pub mod kvs;
pub mod persist;
pub mod pull;
pub mod reader;
pub mod store;
pub mod value;
pub mod vocab;

// テスト専用ログ捕捉ヘルパ: 共有 crate `log-capture-kit` の捕捉窓と型の上の薄い 2 つの関数
// （`capture`・`assert_logged`）。使い手は persist と actor のテスト。
#[cfg(test)]
mod test_log_capture;

// --- 公開面 re-export ---
pub use actor::{
    Effect, SetClass, SylphyaInit, SylphyaMsg, SylphyaParts, SylphyaPublisher, Writer, apply,
    classify_set, spawn_sylphya, spawn_sylphya_with_failures,
};
pub use asker::{AskerContext, AskerId};
pub use key::{KeyParseError, PathSeg, PropPath, Selector, parse_dotted};
pub use kvs::{Kvs, KvsDirect, WriteFailed, WriteOutcome};
pub use persist::{
    Axis, PersistFailure, PersistFailureCause, PersistKey, PersistOutcome, PersistScope,
    ScopeRoots, load_scope, remove_scope_keys, save_scope,
};
pub use pull::{PullShape, PullSource};
pub use reader::SylphyaReader;
pub use value::{DottedResolution, FlatResolution};
pub use vocab::{BackingLayer, DegradePolicy, M1Status, SetSemantics};
