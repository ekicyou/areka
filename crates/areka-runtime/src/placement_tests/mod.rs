//! 本体を引かず、配置と runtime だけを使うテストと、遷移の観測ログの判定器の置き場
//! （spec: runtime-crate-split 要件 4.2・4.4・4.5）。
//!
//! 本体（`areka`）の項目も使うものは本体の `placement_tests/` に残る。`lib.rs` が `#[cfg(test)]` の下で
//! 宣言するので、本番のビルドには入らない。

pub(crate) mod transition_judge;
pub(crate) mod transition_judge_offset;

mod measure_bake_log_tests;
mod reset_balloon_pos_tests;
mod spawn_zorder_pair_deferred_tests;
