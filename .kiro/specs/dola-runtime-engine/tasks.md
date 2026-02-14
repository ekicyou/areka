# 実装計画 — dola-runtime-engine

## タスク概要

dola クレート内に `runtime` サブモジュールを構築し、12要件すべてをカバーする段階的実装を行う。4子仕様分割計画（core-types, runtime-facade, conflict-loop, clock）に沿って、基盤型定義 → 主要ランタイム機能 → 高度機能 → ユーティリティの順で実装する。

---

## 実装タスク

- [ ] 1. ランタイムコア型定義 (Child Spec 1: core-types)
- [ ] 1.1 (P) 実行インスタンス状態管理型の実装
  - `InstanceState` enum を7バリアント（Created, Playing, Paused, Concluded, Cancelled, Trimmed, Compressed）で定義
  - 状態遷移ロジックの実装（Playing ⇄ Paused、Playing/Paused → 各終了状態）
  - 終了状態への遷移後は変更不可の不変条件を保証
  - 単体テスト: 全遷移パターン（許可/拒否）の網羅
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [ ] 1.2 (P) 評価済み変数値型とエラー型の実装
  - `EvaluatedValue` enum（Float, Integer, Object）を定義
  - `RuntimeError` enum を6バリアント（StoryboardNotFound, InvalidGroupId, TerminatedInstance, DocumentParseError, ZeroDurationWithLoop, CompileError）で定義
  - `TerminatedInstance` バリアントに group_id と state を含める
  - 単体テスト: エラーメッセージのフォーマット検証
  - _Requirements: 1.5, 2.8, 2.9, 3.7_

- [ ] 1.3 (P) イージング補間計算機能の実装
  - `Interpolator` 構造体と `InterpolatorApi` trait を定義
  - `interpolation` クレート (0.3.0) の `Ease` trait および `EaseFunction` を使用
  - `EasingName` 30バリアント + `Linear` + `ParametricEasing`（QuadraticBezier, CubicBezier）の1対1マッピング
  - `VariableTypeHint` による型別処理（Float直接, Integer丸め, Object即時切替）
  - 単体テスト: 全イージング関数の出力値検証、境界値（t=0.0, 1.0）検証
  - _Requirements: 10.1, 10.2, 10.3, 10.4_

- [ ] 2. ドキュメント管理層の実装
- [ ] 2.1 (P) DocumentStore コンポーネント実装
  - 指示書（TOML文字列）のパースと `DolaDocument` 保持機能
  - 新指示書による定義上書きロジック（同名変数の引き継ぎ、旧変数の削除）
  - パース失敗時の既存定義保持（ロールバック）
  - ストーリーボード定義の永続的保持（上書きまで維持）
  - 単体テスト: パース成功/失敗、定義上書き、変数引き継ぎ、ロールバック
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [ ] 3. インスタンス管理層の実装
- [ ] 3.1 InstanceManager コンポーネント基盤実装
  - `StoryboardInstance` 構造体の定義（group_id, state, interruption_policy, start_time, time_scale, pause_accumulated等）
  - group_id 単調増加連番生成機構（`AtomicU64` または同等）
  - `HashMap<u64, StoryboardInstance>` による O(1) ルックアップ
  - Start コマンド処理: コンパイル実行、group_id 採番、終了予定時刻計算（`StartResult` 返却）
  - CalculateEndTime コマンド処理: コンパイルのみ実行、タイムテーブル追加なし
  - 単体テスト: group_id 単調増加性、終了予定時刻計算精度、loop_count=0 で INFINITY 返却
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9_

- [ ] 3.2 制御コマンド群の実装
  - Pause コマンド: 経過時刻加算停止、`pause_start` 記録
  - Resume コマンド: `pause_accumulated` 更新、経過時刻再開、終了予定時刻再計算
  - Conclude コマンド: 現在トランジションの最終値ジャンプ、未開始スキップ、状態を Concluded へ遷移
  - Cancel コマンド: 現在値で凍結、状態を Cancelled へ遷移
  - Finish(offset) コマンド: offset 秒後に Conclude 実行の遅延予約
  - 終了済みインスタンスへの操作時のエラー返却（`RuntimeError::TerminatedInstance`）
  - 単体テスト: 各コマンドの状態遷移検証、終了済みへの操作拒否、Finish 遅延実行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

- [ ] 4. タイムテーブル管理層の実装
- [ ] 4.1 TimelineManager コンポーネント基盤実装
  - `VariableTimeline` および `TimelineEntry` 構造体の定義
  - `HashMap<String, VariableTimeline>` による変数名ベースのタイムテーブル管理
  - コンパイル結果の挿入: `insert_entries()` による group_id 付きエントリ追加
  - 購読変数数に比例した計算コスト（非購読変数の評価スキップ）
  - 単体テスト: エントリ追加、複数 group_id の並行保持、非購読変数の無視
  - _Requirements: 6.1, 6.2, 9.1, 9.2, 9.3_

- [ ] 4.2 時刻ベース変数評価機能の実装
  - `evaluate()` メソッド: 現在時刻で変数値を評価
  - effective_time 計算（`(current_time - start_time - pause_accumulated) * time_scale`）
  - active segment 特定とセグメント内 progress_t 計算
  - 複数 group_id 存在時の最新（最大）group_id 優先ルール
  - 終了済みトランジションの自動破棄（Update 時）
  - Interpolator への補間計算委譲
  - 単体テスト: time_scale 2.0/0.5 の速度変化検証、pause_accumulated 考慮、終了済み破棄
  - _Requirements: 5.1, 5.2, 6.5_

- [ ] 4.3 (P) 競合検出インターフェースの実装
  - `detect_conflicts()` メソッド: 新エントリと既存エントリの時間的重複チェック
  - 重複判定: 新セグメント時間範囲と既存セグメント時間範囲の交差検出
  - 競合する group_id のリスト返却
  - 単体テスト: 重複あり/なしパターン、複数変数での競合検出
  - _Requirements: 7.1_

- [ ] 5. 購読管理層の実装
- [ ] 5.1 (P) SubscriptionManager コンポーネント実装
  - `SubscriberState` 構造体（variables セット、last_values キャッシュ）の定義
  - subscribe/unsubscribe/unsubscribe_all メソッドの実装
  - 指示書受信前の購読登録許可
  - Drop trait による自動全購読解除
  - 指示書に存在しない変数の無視
  - 単体テスト: subscribe/unsubscribe、Drop 自動解除、存在しない変数の無視
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.6_

- [ ] 5.2 差分検出と Update レスポンス生成の実装
  - 購読変数リストの取得と TimelineManager への評価要求
  - 前回値との差分検出（`last_values` との比較）
  - `Vec<(String, EvaluatedValue)>` 形式での変更変数のみ返却
  - 凍結状態変数に対する空 Vec 返却
  - 単体テスト: 値変化あり/なしパターン、凍結状態処理、複数購読者の独立性
  - _Requirements: 5.1, 5.3, 5.4, 4.5_

- [ ] 6. DolaRuntime Facade API の統合
- [ ] 6.1 DolaRuntime 構造体とトップレベル API の実装
  - 全内部コンポーネント（DocumentStore, InstanceManager, TimelineManager, SubscriptionManager）の保持
  - group_id 生成機構の一元管理
  - load_document / start / calculate_end_time メソッドの実装（内部コンポーネントへの委譲）
  - pause / resume / conclude / cancel / finish メソッドの実装
  - subscribe / unsubscribe / unsubscribe_all / update メソッドの実装
  - エラーハンドリングの統一（`RuntimeError` 返却）
  - 単体テスト: 各 API の委譲確認、エラー伝播検証
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 4.1, 4.2, 4.3, 4.4, 5.1, 5.3, 5.4_

- [ ] 7. 競合解決とループ制御の実装 (Child Spec 3: conflict-loop)
- [ ] 7.1 ConflictResolver コンポーネント実装
  - `resolve_conflicts()` メソッド: 競合検出と終了戦略適用
  - group_id 単位一括適用ロジック（1変数競合で同一 group_id の全変数に戦略適用）
  - Cancel 戦略: 現在補間値で凍結、状態を Cancelled へ遷移
  - Conclude 戦略: 現在トランジション最終値ジャンプ + 未開始スキップ、状態を Concluded へ遷移
  - Trim 戦略: 割り込み開始時点で切断、状態を Trimmed へ遷移
  - Compress 戦略: 全トランジション最終値ジャンプ、状態を Compressed へ遷移
  - デフォルト戦略: Conclude（`InterruptionPolicy` 未指定時）
  - 単体テスト: 各戦略の個別適用結果検証、group_id 一括適用、デフォルト戦略
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.9_

- [ ] 7.2 Never ポリシー延期キューの実装
  - `DeferredEntry` 構造体（group_id, variable_name, segments, blocked_by）の定義
  - ConflictResolver または TimelineManager 内での延期キュー保持
  - Never 戦略適用時の新エントリ延期処理
  - 先行 group_id 終了時の再評価トリガー: InstanceManager からの終了通知受信
  - 延期エントリのタイムテーブル自動追加
  - 単体テスト: 延期エントリ生成、再評価トリガー動作、無限ループ先行 group_id の永続延期
  - _Requirements: 7.8_

- [ ] 7.3 LoopController コンポーネント実装
  - `should_continue_loop()` メソッド: loop_count による周回継続判定
  - loop_count None → 1回再生、Some(0) → 無限ループ、Some(n) → n回ループ
  - `advance_loop()` メソッド: `pause_accumulated` 調整によるタイムテーブル再利用
  - ループ完了時の終了状態遷移
  - ループ中の競合検出・中断戦略適用サポート
  - 単体テスト: None/Some(0)/Some(n) の周回判定、タイムテーブル再利用、ループ中中断
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 12.7, 12.8_

- [ ] 8. 時刻ユーティリティの実装 (Child Spec 4: clock)
- [ ] 8.1 (P) Clock モジュール実装
  - feature gate `windows-clock` による条件付きコンパイル
  - `now()` 関数: Win32 `GetTickCount64` を使用して OS 起動時からの f64 秒数を返却
  - 実装: `GetTickCount64() as f64 / 1000.0`
  - 単体テスト: 時刻の単調増加性検証、ms 精度確認
  - _Requirements: 11.1, 11.2, 11.3_

- [ ] 9. 統合テスト
- [ ] 9.1 基本再生サイクル統合テスト
  - load_document → start → update(複数回) → 自然終了の完全フロー
  - 値の時系列変化検証（開始値 → 補間中間値 → 最終値）
  - 終了後の update で空 Vec 返却確認
  - _Requirements: 1.1, 2.1, 5.1_

- [ ] 9.2 制御コマンド統合テスト
  - Pause/Resume サイクル: 一時停止中の値固定、再開後の継続確認
  - Conclude: 最終値ジャンプ動作検証
  - Cancel: 中間値凍結動作検証
  - Finish(offset): 遅延 Conclude のタイミング検証
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 9.3 競合解決統合テスト
  - 2ストーリーボードが同一変数を操作する各戦略（Cancel/Conclude/Trim/Compress/Never）の結果検証
  - group_id 単位一括適用の動作確認（1変数競合で全変数に戦略適用）
  - Never ポリシーの延期キュー自動追加検証
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8_

- [ ] 9.4 ループ再生統合テスト
  - loop_count=Some(3) の3周回完了と値の周期的変化検証
  - loop_count=Some(0) の無限ループ継続検証（明示的中断まで）
  - タイムテーブル再利用による効率的再生確認
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_

- [ ] 9.5 指示書差し替え統合テスト
  - 再生中の load_document による定義上書き
  - 同名変数の値引き継ぎ検証
  - 旧定義変数の凍結状態検証
  - 新定義変数の即時反映確認
  - _Requirements: 1.2, 1.3, 1.4_

- [ ] 9.6* 購読管理統合テスト
  - 複数購読者の独立した subscribe/update 動作検証
  - Drop による自動全購読解除の動作確認
  - 指示書受信前の購読登録と事後反映検証
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1_

- [ ] 10. 性能テスト
- [ ] 10.1* 大規模購読性能テスト
  - 100変数同時購読時の update レイテンシ計測
  - ベースライン: 16ms（60fps）未満
  - _Requirements: 9.3_

- [ ] 10.2* 同時再生性能テスト
  - 50ストーリーボード同時再生時のメモリ使用量計測
  - ベースライン: 10MB 未満
  - _Requirements: 9.1, 9.2_

- [ ] 10.3* 無限ループ精度テスト
  - loop_count=Some(0) の1時間連続再生での値変化精度検証
  - time_scale 精度劣化の観測
  - _Requirements: 12.2_

---

## 要件カバレッジサマリー

| 要件 | タスク |
|------|--------|
| Req 1 | 2.1, 6.1, 9.1, 9.5 |
| Req 2 | 3.1, 6.1, 9.1 |
| Req 3 | 3.2, 6.1, 9.2 |
| Req 4 | 5.1, 6.1, 9.6 |
| Req 5 | 4.2, 5.2, 6.1, 9.1, 9.6 |
| Req 6 | 4.1, 4.2 |
| Req 7 | 4.3, 7.1, 7.2, 9.3 |
| Req 8 | 1.1, 3.2 |
| Req 9 | 4.1, 10.1, 10.2 |
| Req 10 | 1.3 |
| Req 11 | 8.1 |
| Req 12 | 7.3, 9.4, 10.3 |

全12要件がタスクにマッピング済み。
