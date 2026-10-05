//! 書かない時点で記憶が書き換わらないことを固定する（areka-P0-char-position-save-on-exit
//! タスク 4.2・要件 1.2・3.2・3.3・5.3・design Testing Strategy「並べ直しから記憶まで」15）。
//!
//! 窓の一式・記憶の送り口・最小のゴーストは共有の部品
//! `frame_chain_finalize_persist_test_support.rs` から取る。
