// tests/ghost.rs — ghost 統合テストのエントリポイント（#[path] mod 束ねのみ）。
//
// 設計「File Structure Plan」に従う（テスト命名規約）。本ファイルにテストロジックは
// 置かず、実体は `tests/ghost/` 配下の各 `*_test.rs` に持つ。
//
// `spine_e2e_test.rs`（task 4.1 が `ScriptedShioriBackend`/`RecordingSink` を定義し、
// 後続タスク 4.2〜4.7 が同ファイルへ boot〜close の各シナリオ（S1〜S6）の `#[test]` を
// 追加していく）。`real_pasta_test.rs`（task 4.8・env ゲート実 pasta 追験）はまだ存在
// しないため、ここでは宣言しない（存在しないファイルへの `#[path]` はコンパイルエラー
// になるため）。task 4.8 はこのファイルへ 1 行追記するだけで済む。
#[path = "ghost/spine_e2e_test.rs"]
mod spine_e2e_test;
