//! # areka-sylphya — 統一プロパティシステム
//!
//! areka の「名前で引ける値」の解決機構をただ 1 つ確立する最下層クレート。
//! %フラット環境変数・点付きプロパティ木・SHIORI Resource 系の全語彙を、
//! 単一名前空間の第一級エントリとして保持する。
//!
//! ## 掲示板（マテリアライズド・ビュー）モデル
//!
//! アーキテクチャは **同期読み・非同期供給の掲示板（マテリアライズド・ビュー）**。
//! 読みは共有読みハンドル（[`reader`]）による同期・無待機（epoch 交換の不変スナップショット
//! の read lock 内 `Arc` clone のみ）。供給（publish・SET 中継・永続書込・prefetch 編成）は
//! `areka-actor` 規約に載る **古典スレッド 1 本の同期アクター**（[`actor`]）が単独所有する。
//! 同期読み経路でのクロスアクター pull 照会は禁止（push 鏡像・pull 棄却）。
//!
//! ## 5 つの backing 実体層
//!
//! 「名前で引ける値」の源は次の 5 実体層に分類される（[`vocab::BackingLayer`]）:
//!
//! 1. **StaticConfig** — descript 等の静的構成（selfname/keroname/baseware 系）
//! 2. **RuntimeState** — 実行時状態（surface/scope 系）
//! 3. **SystemEnv** — システム環境（時刻・画面系）
//! 4. **ShioriQuery** — SHIORI 照会（username 等）
//! 5. **Persistent** — 層別 TOML 永続（窓位置・オフセット・起動記録・vanish 回数）
//!
//! M1 実導出は StaticConfig の一部・ShioriQuery(username)・Persistent の器のみ。
//! 残りは差替シーム（backing 登録だけで実導出化する）付きで決定論的に縮退する。
//!
//! ## 読み口 2 形
//!
//! 単一名前空間を 2 つの窓から読む:
//!
//! 1. **per-talk 凍結スナップショット** — talk 開始時に鏡像の実在値を写し取り、
//!    その talk の生存期間中は凍結した像を参照する（同一 talk 内で値がブレない）。
//! 2. **逐次同期解決** — GetProperty 等が呼ばれた時点の鏡像を都度・同期・無待機で解決する。
//!
//! いずれも問い合わせ元コンテキスト（[`asker`]）を第一級に取り、由来（どの backing から
//! 来たか）は API 型に載せない（ログには記録する）。

pub mod actor;
pub mod asker;
pub mod key;
pub mod mirror;
pub mod persist;
pub mod reader;
pub mod value;
pub mod vocab;

// Task 2.5: 語彙台帳・key パーサ 決定論単体テスト群（criterion → assertion 集約・test-only）。
#[cfg(test)]
mod ledger_key_determinism_tests;

// Task 4.1 flaky 根治: テスト専用ログ捕捉ヘルパ（interest-keeper で並列負荷下の Interest::never
// 焼き付きを根絶・決定論檻の共有基盤・R9.1／R8.1）。persist/format.rs・persist/mod.rs・後続 4.4 が
// `crate::test_log_capture::capture` から利用する。
#[cfg(test)]
mod test_log_capture;

// --- 公開面 re-export（後続タスクが素直に使えるよう主要型を crate 直下へ）---
pub use actor::{
    classify_set, spawn_sylphya, Effect, RuntimeCommandSink, SetClass, SylphyaCore, SylphyaInit,
    SylphyaMsg, SylphyaParts, SylphyaPublisher,
};
pub use asker::{AskerContext, AskerId};
pub use key::{parse_dotted, KeyParseError, PathSeg, PropPath, Selector};
pub use mirror::{MirrorImage, SharedMirror};
pub use persist::{
    load_scope, save_scope, Axis, PersistKey, PersistOutcome, PersistScope, ScopeRoots,
};
pub use reader::SylphyaReader;
pub use value::{DottedResolution, FlatResolution};
pub use vocab::{BackingLayer, DegradePolicy, M1Status, SetSemantics};
