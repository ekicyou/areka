# Implementation Plan — dola-runtime-5-loop

## タスク概要

本実装計画は、dola ランタイムエンジンにループ再生機能を追加するための作業項目を定義する。タイムテーブルを1周分のみ保持しつつ、loop_count に基づく効率的な繰り返し再生を実現する。

---

## Tasks

- [x] 1. データ構造とエラー定義の拡張
- [x] 1.1 (P) RuntimeError にループ関連エラーバリアントを追加
  - `TooShortDurationWithInfiniteLoop { storyboard: String, duration: f64 }` バリアント追加
  - MIN_LOOP_DURATION 定数定義（0.1秒）
  - エラーメッセージ実装（Display trait）
  - _Requirements: 該当なし（設計判断による追加）_

- [x] 1.2 (P) StoryboardInstance にループ状態管理フィールドを追加
  - `loops_completed` の型を u32 → u64 に変更
  - `loop_start_time: f64` フィールド追加（周回開始時刻）
  - `loop_duration: f64` フィールド追加（1周分の再生時間）
  - `create_instance()` の引数に loop_start_time, loop_duration を追加し初期化
  - `instances_mut()` メソッド追加（&mut HashMap<u64, StoryboardInstance> を返す）
  - _Requirements: 3.1, 3.3, 2.3, 2.4, 4.3_

- [x] 2. LoopController モジュール実装
- [x] 2.1 LoopAction enum と周回判定・進行関数を実装
  - `LoopAction` enum 定義（Continue, Conclude バリアント）
  - `should_continue_loop()` 実装（loop_count=-1 は常に true、それ以外は loops_completed < loop_count）
  - `advance_loop()` 実装（loops_completed += 1, loop_start_time += loop_duration, end_time += loop_duration）
  - _Requirements: 1.5, 2.4, 3.2, 3.4_

- [x] 2.2 process_loops() 統合関数を実装
  - while ループで current_time >= end_time の間、advance_loop() を繰り返す
  - 各周回で should_continue_loop() を呼び、false なら LoopAction::Conclude を返す
  - loop_count=1 の場合は while ループをスキップして即座に Conclude
  - 無限ループ時は全終了済み周回を一括処理
  - _Requirements: 1.2, 1.3, 1.4, 2.2, 2.5_

- [x] 3. facade.rs にループ再生統合
- [x] 3.1 start() でループバリデーションと end_time 算出を実装
  - loop_duration 計算（total_base_duration / time_scale）
  - loop_duration < MIN_LOOP_DURATION && loop_count == -1 のエラーチェック
  - 無限ループ時も end_time = start_time + loop_duration（INFINITY 不使用）
  - create_instance() 呼び出しに loop_start_time, loop_duration を渡す
  - _Requirements: 1.1, 1.2, 1.3, 2.1_

- [x] 3.2 update() Step 2 でループ処理を統合
  - instances_mut() で可変参照取得
  - Playing かつ current_time >= end_time のインスタンスをフィルタ
  - 各インスタンスに process_loops() を適用し結果を Vec に collect
  - LoopAction::Conclude のインスタンスに conclude_internal() を実行
  - _Requirements: 1.4, 1.5, 2.2, 2.3, 2.5, 5.3_

- [x] 4. (P) timeline_manager.rs のループ対応
- [x] 4.1 (P) calculate_effective_time() を loop_start_time ベースに変更
  - start_time → loop_start_time への置換（3箇所）
  - Pause/Resume との整合性確認（pause_accumulated は独立して動作）
  - loop_start_time の初期値は start_time なので既存動作と互換性維持
  - _Requirements: 2.3, 4.1, 4.2, 4.3_

- [x] 5. テスト実装
- [x] 5.1 LoopController 単体テストを実装
  - should_continue_loop() の基本・無限・単回ケーステスト
  - advance_loop() のフィールド更新テスト
  - process_loops() の周回内・1周完了・ループ完了・複数周回一括・全周回一括完了・無限ループ複数周回テスト
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 2.2, 2.4, 3.2, 3.4_

- [x]* 5.2 facade 統合テストを実装
  - ループなし（loop_count=1）の既存動作互換性テスト
  - 有限ループ（loop_count=3）の3周再生テスト
  - 無限ループ（loop_count=-1）の継続再生テスト
  - 複数周回一括処理テスト（大きな dt）
  - Pause + ループの正確な再開テスト
  - Cancel + ループの即座停止テスト
  - end_time 統一テスト（loop_count=1 vs -1）
  - 短周期無限ループエラーテスト
  - 短周期有限ループ許可テスト
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.5, 4.1, 4.2, 5.1, 5.2_

---

## Notes

- タスク 1.1 と 1.2 は並列実行可能（独立したファイル変更）
- タスク 4.1 は 1.2 完了後に並列実行可能（StoryboardInstance のフィールド参照のみ）
- タスク 2 は 1.2 完了が前提（StoryboardInstance のフィールドに依存）
- タスク 3 は 1, 2 完了が前提（エラーバリアント、LoopController、StoryboardInstance を使用）
- タスク 5.2 は統合テストのため、全実装完了後に実行（オプショナルマーク付き）
