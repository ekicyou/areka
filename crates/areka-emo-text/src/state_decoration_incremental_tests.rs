//! # state_decoration_incremental_tests — 装飾の積み増しの檻（純粋層・兄弟テスト）
//!
//! spec `emo-text-reveal-incremental` 件 2（要件 2.7／2.8）。台本の写しと当て直しが届いた分に
//! 比例すること（数え）、混ぜた台本で全部当て直す形（`REPLAY_ALL`）と結果が一致すること、
//! 寿命と記録が今と変わらないことを確かめる。
//!
//! [`super`]（`state_decoration.rs`）の子モジュールとして接続してある。
