// tests/kanade.rs — kanade 統合テストのエントリポイント（#[path] mod 束ねのみ）。
//
// 設計「File Structure Plan」に従い、全テストモジュールを先出しで宣言する。
// 各 `*_test.rs` は後続タスク（4.2〜4.6・5）が `#[test]` を自ファイルへ追加するだけで
// 済むよう、この時点では空のプレースホルダとして用意する（本ファイルは以後編集不要）。
#[path = "kanade/boot_test.rs"]
mod boot_test;
#[path = "kanade/choice_test.rs"]
mod choice_test;
#[path = "kanade/close_test.rs"]
mod close_test;
#[path = "kanade/common/mod.rs"]
mod common;
#[path = "kanade/failure_test.rs"]
mod failure_test;
#[path = "kanade/full_run_test.rs"]
mod full_run_test;
#[path = "kanade/mouse_test.rs"]
mod mouse_test;
#[path = "kanade/prefetch_test.rs"]
mod prefetch_test;
#[path = "kanade/real_helper_test.rs"]
mod real_helper_test;
#[path = "kanade/steady_test.rs"]
mod steady_test;
