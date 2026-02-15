# 実装計画 — dola-runtime-3-facade

## タスク概要

DolaRuntime facade と4つの内部コンポーネント（DocumentStore, InstanceManager, TimelineManager, SubscriptionManager）を実装する。core-types (Tier 1) の型・補間計算を消費し、ランタイムの再生パイプライン全体を構築する。

---

## 実装タスク

- [ ] 1. DocumentStore の実装
  - `crates/dola/src/runtime/document_store.rs` を作成
  - `doc.validate()` によるバリデーション実行
  - バリデーション成功時のみ `DolaDocument` を保持、失敗時は既存保持
  - ストーリーボード定義の取得メソッド
  - 単体テスト: 保持、上書き、バリデーション失敗時の既存保持
  - _Requirements: 1.1, 1.5, 2.1, 2.4_

- [ ] 2. InstanceManager の実装 (P)
  - `crates/dola/src/runtime/instance_manager.rs` を作成
  - `StoryboardInstance` 構造体定義
  - `HashMap<u64, StoryboardInstance>` によるインスタンス管理
  - `create_instance()`, `get()`, `get_mut()`, `transition()` メソッド
  - `pause()`: `pause_start` 記録 + Paused 遷移
  - `resume()`: `pause_accumulated` 加算 + Playing 遷移 + 終了予定時刻再計算
  - `conclude()`, `cancel()`: 状態遷移 + タイムテーブル操作通知
  - `finish()`: `finish_deadline` 設定
  - 単体テスト: 状態遷移、Pause/Resume 時間計算、終了済みインスタンス拒否
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 9.1, 9.2, 9.3_

- [ ] 3. TimelineManager の実装 (P)
  - `crates/dola/src/runtime/timeline_manager.rs` を作成
  - `VariableTimeline` / `TimelineEntry` 構造体定義
  - `insert_entries()`: コンパイル結果を変数ごとのタイムテーブルに追加
  - `evaluate()`: effective_time 計算 → アクティブセグメント特定 → Interpolator 呼び出し → 最新 group_id 優先
  - 終了済みエントリの自動破棄
  - `remove_entries()`: group_id の全エントリ削除
  - 単体テスト: エントリ挿入、evaluate 計算、最新 group_id 優先、エントリ破棄
  - _Requirements: 7.1, 7.2, 7.5, 8.1, 8.2, 8.3, 8.4, 8.5, 10.1, 10.2, 10.3, 11.1_

- [ ] 4. SubscriptionManager の実装
  - `crates/dola/src/runtime/subscription_manager.rs` を作成
  - `SubscriberState` 構造体（購読変数セット + last_values キャッシュ）
  - `subscribe()`, `unsubscribe()`, `unsubscribe_all()` メソッド
  - `get_subscribed_variables()`: 購読変数名リスト取得
  - `diff_and_update()`: 前回値との比較、変化した変数のみ返却、last_values 更新
  - 単体テスト: subscribe/unsubscribe、差分検出、unsubscribe_all
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 7.1, 7.4_

- [ ] 5. DolaRuntime facade の実装
  - `crates/dola/src/runtime/facade.rs` を作成
  - 4コンポーネントの統合 + `next_group_id` カウンタ
  - `load_document()`: DocumentStore 委譲 + 変数引き継ぎロジック
  - `start()`: コンパイル → インスタンス生成 → [Tier 3 Hook] → タイムテーブル挿入 → 状態遷移
  - `calculate_end_time()`: コンパイルのみ（インスタンス非生成）
  - `pause()` / `resume()` / `conclude()` / `cancel()` / `finish()`: InstanceManager 委譲
  - `subscribe()` / `unsubscribe()` / `unsubscribe_all()`: SubscriptionManager 委譲
  - `update()`: 購読変数取得 → 評価 → 差分検出 → Finish deadline チェック
  - `runtime/mod.rs` を更新して `pub use facade::DolaRuntime` をエクスポート
  - _Requirements: 全要件 (1-11)_

- [ ] 6. 統合テスト
  - `crates/dola/tests/runtime_facade_test.rs` を作成
  - フル再生サイクル: load → subscribe → start → update(複数回) → 自然終了
  - Pause/Resume サイクル: 値固定と再開後の継続
  - 指示書差し替え: 再生中の load、同名引き継ぎ + 消失凍結
  - 同時再生: 異なる変数の並行ストーリーボード
  - 制御コマンド: Conclude / Cancel / Finish の動作検証
  - CalculateEndTime: インスタンス非生成の確認
  - Tier 2 暫定動作: 同一変数への複数 Start で最新 group_id 優先
  - `cargo test --features runtime` で全テスト通過を確認
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 7.4, 7.5, 8.1, 8.2, 9.1, 10.1, 11.1, 11.2, 11.3_
