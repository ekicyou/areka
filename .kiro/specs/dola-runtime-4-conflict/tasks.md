# 実装計画 — dola-runtime-4-conflict

## タスク一覧

- [ ] 1. エラー型とモジュール骨組み準備
- [ ] 1.1 RuntimeError::Conflict バリアント追加
  - `RuntimeError` enum に `Conflict` バリアントを追加する
  - Never 戦略でストーリーボード起動を拒否する際に返すエラー型を定義
  - エラーメッセージには競合した group_id と変数名を含める
  - _Requirements: 7.2_

- [ ] 1.2 conflict_resolver モジュールのスケルトン作成
  - `runtime/conflict_resolver.rs` を新規作成し、モジュール構造を定義する
  - `resolve_conflicts()` の関数シグネチャを実装（空の本体）
  - 内部関数（`detect_overlaps`, `apply_cancel`, `apply_conclude`, `apply_trim`, `apply_compress`）のプレースホルダを配置
  - `use` 文と依存関係（TimelineManager, InstanceManager, SubscriptionManager）をインポート
  - _Requirements: 1.1, 1.2, 1.3_

- [ ] 2. 競合検出の中核ロジック実装
- [ ] 2.1 (P) detect_overlaps() による時間重複検出
  - 新規ストーリーボードの各変数セグメントと既存タイムテーブルの時間範囲を比較する
  - Playing / Paused 状態のインスタンスのみを競合対象とする（Created/終了状態は除外）
  - 複数変数で独立に競合を検出し、結果を group_id のセットとして集約する
  - セグメントの `start_time..end_time` が重複する場合に競合と判定
  - _Requirements: 1.1, 1.4, 1.5_

- [ ] 2.2 (P) resolve_conflicts() のディスパッチ骨組み実装
  - `detect_overlaps()` を呼び出して競合 group_id を取得する
  - 競合なしの場合は空のベクタを返す（早期リターン）
  - 各競合 group_id の `InterruptionPolicy` を取得し、対応する戦略関数にディスパッチする
  - Never 戦略は後続タスクで実装（現時点では `unimplemented!()` でマーク）
  - _Requirements: 1.2, 1.3, 2.1, 2.3_

- [ ] 3. 4種の基本終了戦略実装
- [ ] 3.1 (P) Cancel 戦略: 中断時点の値で凍結
  - `apply_cancel()` を実装し、start_time 時点の補間値を取得する
  - `TimelineManager::evaluate()` で各変数の値を評価
  - `SubscriptionManager::force_update_last_values()` で購読者に値を伝播
  - `InstanceManager::transition()` で Cancelled 状態に遷移
  - `TimelineManager::remove_entries()` でタイムテーブルエントリを削除
  - group_id 単位で同一グループの全変数に一括適用する
  - _Requirements: 2.2, 3.1, 3.2, 3.3_

- [ ] 3.2 (P) Conclude 戦略: 現在セグメントの最終値にジャンプ
  - `apply_conclude()` を実装し、現在再生中セグメントの終了値を取得する
  - `TimelineManager::collect_current_segment_final_values()` を新規実装（start_time でアクティブなセグメントの `to_value` を progress_t=1.0 で評価）
  - `calculate_effective_time()` を `pub(crate)` に変更して再利用
  - 未開始セグメントをスキップし、アクティブセグメントのみ最終値を収集
  - Concluded 状態に遷移してエントリを削除
  - group_id 単位で同一グループの全変数に一括適用する
  - _Requirements: 2.2, 4.1, 4.2, 4.3_

- [ ] 3.3 (P) Trim 戦略: 中断時点で切断して値を伝播
  - `apply_trim()` を実装し、start_time 時点の補間値で確定する
  - `TimelineManager::evaluate()` で各変数の中断時点の値を取得
  - `SubscriptionManager::force_update_last_values()` で購読者に値を伝播
  - start_time 以降のセグメントをすべて除去（方式 B: エントリ全削除 + 値伝播）
  - Trimmed 状態に遷移
  - group_id 単位で同一グループの全変数に一括適用する
  - _Requirements: 2.2, 5.1, 5.2, 5.3, 5.4_

- [ ] 3.4 (P) Compress 戦略: ストーリーボード全体の最終値にジャンプ
  - `apply_compress()` を実装し、全セグメントの最終値を収集する
  - 既存の `TimelineManager::collect_final_values()` を再利用
  - 全トランジションを完走した扱いで購読者に最終値を伝播
  - Compressed 状態に遷移してエントリを削除
  - group_id 単位で同一グループの全変数に一括適用する
  - _Requirements: 2.2, 6.1, 6.2, 6.3, 6.4_

- [ ] 4. Never 戦略とデフォルト設定
- [ ] 4.1 Never 戦略: 競合時に start() をエラー終了
  - Never 戦略検出時に新規インスタンス（new_group_id）を削除する
  - `InstanceManager::remove(new_group_id)` でインスタンスを破棄
  - `Err(RuntimeError::Conflict)` を返して起動を拒否
  - 部分競合（一部変数のみ競合）でも全体を拒否する
  - インスタンス作成後・タイムテーブル挿入前のタイミングでエラーを発生
  - _Requirements: 7.1, 7.3, 7.4_

- [ ] 4.2 デフォルト戦略の設定
  - `InterruptionPolicy` のデフォルト値が `Conclude` であることを確認
  - facade での policy 指定なしのケースで Conclude が適用されることを検証
  - _Requirements: 8.1, 8.2_

- [ ] 5. 既存モジュールの修正
- [ ] 5.1 (P) InstanceManager::transition() の自動削除統一
  - `transition()` 内の削除条件を `if to == InstanceState::Concluded` から `if new_state.is_terminal()` に変更
  - Cancelled / Trimmed / Compressed 遷移時もインスタンスが自動削除されるようにする
  - facade の `cancel()` 内の冗長な `remove()` 呼び出しを削除（オプション）
  - _Requirements: 3.2, 4.2, 5.4, 6.3_

- [ ] 5.2 facade::start() への Tier 3 Hook 統合
  - `start()` メソッド内の Tier 3 Hook 位置（L116-117）に `conflict_resolver::resolve_conflicts()` を呼び出す
  - インスタンス作成後、タイムテーブル挿入前に競合解決を実行
  - Never 競合時の `RuntimeError::Conflict` をそのまま伝播
  - 影響を受けた group_id リストを `StartResult::affected_group_ids` に渡す
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 7.1, 7.2, 7.3, 7.4_

- [ ] 6. テスト実装
- [ ] 6.1 (P) 競合検出ロジックのユニットテスト
  - `detect_overlaps()` の時間重複検出を検証（重複あり・なし・部分重複）
  - Playing / Paused 状態フィルタリングの正確性を確認
  - 複数変数の独立検出と集約を検証
  - _Requirements: 1.1, 1.4, 1.5_

- [ ] 6.2 (P) 各終了戦略の統合テスト
  - Cancel: 中断時点の補間値で凍結されることを検証
  - Conclude: 現在セグメント最終値にジャンプすることを検証
  - Trim: 中断時点で切断され、以降のセグメントが除去されることを検証
  - Compress: ストーリーボード全体の最終値にジャンプすることを検証
  - 各戦略で group_id 単位の一括適用が動作することを確認
  - 状態遷移とエントリ削除の正確性を検証
  - _Requirements: 3.1, 3.2, 3.3, 4.1, 4.2, 4.3, 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 6.4_

- [ ] 6.3 (P) エラーパスと境界条件のテスト
  - Never 戦略で `RuntimeError::Conflict` が返されることを検証
  - Never 競合時に新規インスタンスが削除されることを確認
  - 部分競合でも全体が拒否されることを検証
  - デフォルト戦略（Conclude）が適用されることを確認
  - 競合なしの場合に副作用がないことを検証
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 8.1, 8.2_
