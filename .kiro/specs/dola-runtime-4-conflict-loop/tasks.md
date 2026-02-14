# Tasks — dola-runtime-4-conflict-loop

## Task 1: ConflictResolver 基盤実装

- [ ] 1.1 `conflict_resolver.rs` を作成し、`ConflictResolver` 構造体 + `DeferredEntry` 構造体を定義
- [ ] 1.2 `resolve_conflicts()` メソッドを実装（競合検出 + group_id 収集）
- [ ] 1.3 5種の終了戦略適用ロジックを実装（Cancel / Conclude / Trim / Compress / Never）
- [ ] 1.4 デフォルト戦略（Conclude）の適用を実装
- [ ] 1.5 `runtime/mod.rs` に `pub(crate) mod conflict_resolver;` を追加

## Task 2: Never 延期キュー実装

- [ ] 2.1 Never 戦略時の `DeferredEntry` 生成・格納ロジックを実装
- [ ] 2.2 `release_deferred()` メソッドを実装（先行 group_id 終了時の延期解放）
- [ ] 2.3 無限ループ（`Some(0)`）の先行インスタンスに対する延期保持ロジックを実装

## Task 3: LoopController 実装 (P)

- [ ] 3.1 `loop_controller.rs` を作成し、`LoopController` 構造体を定義
- [ ] 3.2 `should_continue_loop()` を実装（None / Some(0) / Some(n) の判定）
- [ ] 3.3 `advance_loop()` を実装（`loops_completed` インクリメント + `pause_accumulated` 調整）
- [ ] 3.4 `runtime/mod.rs` に `pub(crate) mod loop_controller;` を追加

## Task 4: facade 統合

- [ ] 4.1 `DolaRuntime` 構造体に `ConflictResolver` フィールドを追加
- [ ] 4.2 Start フローに `resolve_conflicts()` 呼び出しを挿入（`insert_entries` 前）
- [ ] 4.3 Update フローにループ判定（`should_continue_loop` + `advance_loop`）を挿入
- [ ] 4.4 インスタンス終了処理に `release_deferred()` 呼び出しを挿入

## Task 5: ユニットテスト

- [ ] 5.1 ConflictResolver テスト: 競合検出（重複あり/なし）、group_id 一括適用
- [ ] 5.2 5種戦略テスト: Cancel (凍結)、Conclude (最終値ジャンプ)、Trim (切断)、Compress (全体ジャンプ)、Never (延期)
- [ ] 5.3 延期キューテスト: Never 延期→解放、無限ループ先行インスタンス
- [ ] 5.4 LoopController テスト: None/Some(0)/Some(n) 判定、advance_loop オフセット検証

## Task 6: 統合テスト

- [ ] 6.1 Start 競合→Conclude→新再生 E2E
- [ ] 6.2 Start 競合→Never→延期→先行終了→解放 E2E
- [ ] 6.3 ループ再生 n 回→終了 E2E
- [ ] 6.4 ループ中の競合発生 E2E
- [ ] 6.5 全5戦略の再生パイプライン統合テスト
