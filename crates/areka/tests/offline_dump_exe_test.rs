//! 建った `areka.exe` の `--offline-dump` を `std::process::Command` で回し、終了コードと
//! ファイルを読む決定論の試験の入口（spec: areka-P0-offline-surface-dump・design「E2E Tests」）。
//!
//! 窓を作らない走りなので入力の机・静かな机は要らない。根・検体・出力は `temp-path-kit` で
//! ワークツリーの `target\` の下に置く（要件 12.6）。本ファイルは `#[path]` の宣言だけを持つ。
#[path = "offline_dump_exe_test/common/mod.rs"]
mod common;
#[path = "offline_dump_exe_test/dump_success_test.rs"]
mod dump_success_test;
#[path = "offline_dump_exe_test/error_log_test.rs"]
mod error_log_test;
#[path = "offline_dump_exe_test/failure_branches_test.rs"]
mod failure_branches_test;
#[path = "offline_dump_exe_test/no_side_write_test.rs"]
mod no_side_write_test;
