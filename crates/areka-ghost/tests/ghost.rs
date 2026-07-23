// tests/ghost.rs — ghost 統合テストのエントリポイント（#[path] mod 束ねのみ）。
//
// 設計「File Structure Plan」に従う（テスト命名規約）。本ファイルにテストロジックは
// 置かず、実体は `tests/ghost/` 配下の各 `*_test.rs` に持つ。
//
// `spine_e2e_test.rs`（task 4.1 が `ScriptedShioriBackend`/`RecordingSink` を定義し、
// 後続タスク 4.2〜4.7 が同ファイルへ boot〜close の各シナリオ（S1〜S6）の `#[test]` を
// 追加していく）。`real_pasta_test.rs`（task 4.8・env ゲート実 pasta 追験）は
// `spine_e2e_test` の `RecordingSink` を再利用する（`crate::spine_e2e_test::RecordingSink`）。
// `common`（task 8.3・退役 `default_system_vars` の代役スタンドイン）はテスト支援モジュール。
// 各 e2e／追験が `crate::common::test_system_vars` を消費し、`SystemVarWiring::Custom` 注入で
// 既存テストの既定 username 前提を無改変のまま保つ。
#[path = "ghost/common.rs"]
mod common;
#[path = "ghost/real_pasta_test.rs"]
mod real_pasta_test;
#[path = "ghost/spine_e2e_test.rs"]
mod spine_e2e_test;
// `recorder`（task 3・交信記録デコレータ）はテスト支援モジュール。後続タスク（5.1/5.2/6.1）が
// `crate::recorder::*`（`Recorder`／`ExchangeRecord` 等）を e2e／採取ハーネスから消費する。
#[path = "ghost/recorder.rs"]
mod recorder;
// `inproc_fixture`（task 4.1 が成果物 DLL の locate、task 4.2 が InProc 実 DLL 駆動のテストゴースト
// 組立を定義する）はテスト支援モジュール。後続タスク（5.x）が e2e ハーネスから消費する。
#[path = "ghost/inproc_fixture.rs"]
mod inproc_fixture;
// `inproc_e2e_test`（task 5.1・I1 canonical 一周 e2e）は `inproc_fixture::shared_test_ghost` の共有
// fixture と `spine_e2e_test::RecordingSink` を再利用し、実 InProc DLL 境界を横断した起動挨拶の演出列を
// 凍結スナップショット由来の期待列と全順序照合する（ドリフト検出込み）。
#[path = "ghost/inproc_e2e_test.rs"]
mod inproc_e2e_test;
// `snapshot_capture_test`（task 6.1・env-gate 採取ハーネス）は `recorder` の `ExchangeRecord`／
// `ExchangeOutcome` を消費し、出力形式生成ロジック（`reconstruct_envelope`／
// `collect_first_get_snapshots`）を純関数として単体テストする。実採取は
// `HOST32_PASTA_DLL`＋`AREKA_SNAPSHOT_OUT` 両設定時のみ動作（欠落は silent skip）。
#[path = "ghost/snapshot_capture_test.rs"]
mod snapshot_capture_test;
// `sylphya_integration_test`（task 8.4・sylphya × ghost 結線 end-to-end）は
// `spine_e2e_test::RecordingSink` を再利用し、実 boot（mock shiori backend）を駆動して
// (1) boot 後の reader 実値解決（selfname 系／baseware）・(2) shutdown 全段成功（sylphya 段含む）・
// (3) username=Value → 初回 talk スナップショット包含（barrier フェンス・レース非依存）を檻化する。
#[path = "ghost/sylphya_integration_test.rs"]
mod sylphya_integration_test;
