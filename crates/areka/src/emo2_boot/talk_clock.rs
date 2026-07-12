//! talk 時刻の推定（epoch 推定・クロック注入）とテキスト sink への時刻付与。
//!
//! `TalkClock`（cue 観測による単調 max epoch 推定・クロック注入可・負値 0.0 clamp・
//! epoch 未確立は `None`）と `ClockedTextSink<T: TextSink + Clone>`（`emit` 時に `observe_cue`
//! した後に内側 sink へ透過転送）を所有する（sakura 契約型＋dola clock を消費）。
//!
//! 骨格のみ。`TalkClock::new`／`observe_cue`／`talk_time` と `ClockedTextSink` の実装は
//! tasks.md task 2.2 が担う。
