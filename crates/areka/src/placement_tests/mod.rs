//! 配置と本体のほかの部分をまたぐテストと、遷移の観測ログの判定器の置き場
//! （spec: placement-crate-split 要件 3.7・3.9・3.10）。
//!
//! 配置の crate（`areka-placement`）は本体（`areka`）に依存できないので、本体の部品
//! （`emo2_boot`・`input_events` ほか）も使うテストは上の crate であるここに置く。
//! `main.rs` が `#[cfg(test)]` の下で宣言するので、本番のビルドには入らない。

pub(crate) mod transition_judge;
pub(crate) mod transition_judge_offset;

mod measure_bake_count_tests;
mod measure_bake_log_tests;
mod reset_balloon_pos_tests;
mod spawn_assembly_tests;
mod zorder_group_branch_coverage_tests;
mod zorder_inversion_branch_coverage_tests;
