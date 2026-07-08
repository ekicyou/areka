// tests/ghost.rs — ghost 統合テストのエントリポイント（#[path] mod 束ねのみ）。
//
// 設計「File Structure Plan」に従う（テスト命名規約）。本ファイルにテストロジックは
// 置かず、実体は `tests/ghost/` 配下の各 `*_test.rs` に持つ。
//
// `spine_e2e_test.rs`（task 4.1 が `ScriptedShioriBackend`/`RecordingSink` を定義し、
// 後続タスク 4.2〜4.7 が同ファイルへ boot〜close の各シナリオ（S1〜S6）の `#[test]` を
// 追加していく）。`real_pasta_test.rs`（task 4.8・env ゲート実 pasta 追験）は
// `spine_e2e_test` の `RecordingSink` を再利用する（`crate::spine_e2e_test::RecordingSink`）。
#[path = "ghost/spine_e2e_test.rs"]
mod spine_e2e_test;
#[path = "ghost/real_pasta_test.rs"]
mod real_pasta_test;
