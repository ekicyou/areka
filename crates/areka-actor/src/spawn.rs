//! 純粋層: 名前付きアクタースレッドの spawn／`ActorHandle`／受信ループヘルパ。
//!
//! 具体的な `spawn_actor`・`ActorHandle`・`ActorError`・`run_inbox` は後続タスクで
//! std（`std::sync::mpsc`＋`std::thread`）のみを用いて実装される。
