//! 配置と本体のほかの部分をまたぐテストの置き場。
//! 遷移の観測ログの判定器は `areka-runtime` の `src/placement_tests/` にある。
//!
//! 配置の crate（`areka-placement`）は本体（`areka`）に依存できないので、本体の部品
//! （`sample_test_support`・`crate_source_test_support`）や `areka-runtime` も使うテストは上の crate であるここに置く。
//! `main.rs` が `#[cfg(test)]` の下で宣言するので、本番のビルドには入らない。

mod measure_bake_count_tests;
mod measure_template_tests;
mod zorder_branch_coverage_test_support;
mod zorder_group_branch_coverage_tests;
mod zorder_inversion_branch_coverage_tests;
