# Implementation Plan — dola-nested-storyboard

## Task Breakdown

### データモデル拡張

- [ ] 1.1 (P) StoryboardEntry にトリガーフィールドを追加
  - `trigger_storyboard: Option<String>` でトリガー対象ストーリーボード名を保持
  - `trigger_start_offset: Option<f64>` で開始オフセットを保持
  - serde アトリビュート（`#[serde(default, skip_serializing_if)]`）で後方互換性を維持
  - エントリ分類ロジック（トリガー/トランジション/純粋キーフレーム）の判定条件を更新
  - _Requirements: 1.1, 1.2, 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 1.2 (P) CompiledTrigger 構造体を定義
  - `fire_time: f64` でトリガー発火時刻を保持
  - `target_storyboard: String` で対象ストーリーボード名を保持
  - `start_offset: Option<f64>` で開始オフセットを保持
  - `source_entry_index: usize` でデバッグ用エントリインデックスを保持
  - `CompiledStoryboard` に `triggers: Vec<CompiledTrigger>` フィールドを追加
  - _Requirements: 1.4, 3.3, 4.3_

- [ ] 1.3 (P) UpdateResult 構造体を定義
  - `changes: Vec<(String, EvaluatedValue)>` で変数差分を保持
  - `triggered: Vec<TriggerResult>` でトリガー実行結果を保持
  - `TriggerResult::Started` バリアント（起動元・起動先・StartResult を含む）を定義
  - `TriggerResult::Error` バリアント（起動元・起動先・RuntimeError を含む）を定義
  - _Requirements: 2.2, 2.5_

- [ ] 1.4 (P) TriggerState 構造体を定義
  - `trigger_index: usize` で CompiledTrigger のインデックスを保持
  - `fired: bool` で当該周回の発火済みフラグを保持
  - `StoryboardInstance` に `trigger_states: Vec<TriggerState>` フィールドを追加
  - _Requirements: 5.1, 5.2_

- [ ] 1.5 (P) DolaError にトリガー関連エラーを追加
  - `TriggerSelfReference` バリアント（ストーリーボード名・エントリインデックス含む）を定義
  - `TriggerCycle` バリアント（循環パス Vec<String> 含む）を定義
  - `TriggerExclusiveViolation` バリアント（ストーリーボード名・エントリインデックス・理由含む）を定義
  - 各エラーバリアントの Display 実装でトリガー元・先を表示
  - _Requirements: 3.1, 3.2, 3.5_

### バリデーション実装

- [ ] 2.1 (P) V9 バリデーションルールを更新
  - 「エントリに variable/transition がない場合、keyframe または trigger_storyboard のいずれかが必須」に変更
  - トリガーエントリが V9 エラーで拒否されないことを確認
  - _Requirements: 1.4_

- [ ] 2.2 (P) V14 自己参照検出を実装
  - エントリの `trigger_storyboard` が自身のストーリーボード名と一致する場合にエラー
  - O(1) の文字列比較で検出
  - `TriggerSelfReference` エラーを返す
  - _Requirements: 3.1_

- [ ] 2.3 (P) V15 循環参照検出を実装
  - ドキュメント内全ストーリーボードのトリガーグラフを構築（HashMap<String, Vec<String>>）
  - DFS でグラフを走査し循環を検出（O(V+E)）
  - 循環パスを `TriggerCycle` エラーで報告
  - _Requirements: 3.2_

- [ ] 2.4 (P) V16 トリガーエントリ排他チェックを実装
  - `trigger_storyboard` と `variable`/`transition` の同時指定を検出
  - `TriggerExclusiveViolation` エラーを返す
  - _Requirements: 1.3_

- [ ] 2.5 (P) V17 トランジションフィールド拒否を実装
  - トリガーエントリに `from`/`to`/`easing`/`duration` が存在する場合にエラー
  - 既存 V7-V9 と同パターンのフィールド存在チェック
  - `TriggerExclusiveViolation` エラーを返す
  - _Requirements: 3.4_

- [ ] 2.6 (P) V18 トリガー対象存在確認を実装
  - `trigger_storyboard` の値が `doc.storyboards` に存在するか検証
  - 既存 V4, V5 と同パターンの名前解決チェック
  - 存在しない場合はバリデーションエラーを返す
  - _Requirements: 1.5_

### コンパイル処理拡張

- [ ] 3.1 compile_storyboard() でトリガーエントリを処理
  - トリガーエントリ判定（`trigger_storyboard.is_some()`）を追加
  - 既存の `resolve_pure_keyframe_time()` と同パターンで `fire_time` を解決
  - `keyframe_times` にトリガー発火時刻を登録（0秒完了：keyframe_time = fire_time）
  - `CompiledTrigger` インスタンスを生成し `triggers` ベクタに追加
  - `triggers` を `fire_time` 昇順でソート
  - `total_base_duration` 計算にトリガーを含めない
  - _Requirements: 1.2, 1.4, 3.3, 4.4_

### ランタイムトリガー実行

- [ ] 4.1 update() にトリガー収集フェーズを追加
  - 既存 Step2（ループ処理）と Step3（評価）の間に新フェーズを挿入
  - `collect_pending_triggers()` で全インスタンスのトリガー状態を走査
  - 発火条件（`current_time >= loop_start_time + trigger.fire_time` かつ `!fired`）を満たすトリガーを収集
  - `Vec<PendingTrigger>` に（インスタンス情報、トリガー、発火時刻）を格納
  - _Requirements: 2.1, 5.1, 5.2_

- [ ] 4.2 update() でトリガー実行を順次行う
  - 中間バッファ `Vec<PendingTrigger>` から順次取り出し
  - 各トリガーに対し `start()` を内部呼び出し（`fire_time + offset` を start_time として渡す）
  - `start()` の成功時は `TriggerResult::Started` を記録
  - `start()` のエラー時は `TriggerResult::Error` を記録（`update()` は中断しない）
  - トリガー実行順は `fire_time` 昇順、同一時刻ではエントリインデックス順
  - _Requirements: 2.1, 2.2, 2.3, 5.4_

- [ ] 4.3 update() の返却値を UpdateResult に変更
  - 既存の `Vec<(String, EvaluatedValue)>` を `UpdateResult.changes` に移動
  - トリガー実行結果を `UpdateResult.triggered` に格納
  - 呼び出し元が `triggered` から子インスタンスの `group_id`/`end_time` を取得可能に
  - _Requirements: 2.5_

- [ ] 4.4 ループ周回時にトリガー状態をリセット
  - `advance_loop()` 内で全 `trigger_states` の `fired = false` にリセット
  - `loop_start_time` を現在周回の開始時刻に更新
  - 周回ごとにトリガーが再発火することを確認
  - _Requirements: 5.1, 5.2_

- [ ] 4.5 親ストーリーボードのライフサイクル独立性を確認
  - 親ストーリーボードのキャンセル・中断時に子ストーリーボードが影響を受けないことを確認
  - 子ストーリーボードのループ設定が親から独立していることを確認
  - _Requirements: 2.4, 5.3_

### テスト

- [ ] 5.1 (P) トリガーエントリの serde 往復テストを追加
  - JSON/TOML/YAML で `trigger_storyboard` / `trigger_start_offset` のシリアライズ・デシリアライズを検証
  - 最小構成（`trigger_storyboard` のみ）と完全構成（オフセット含む）の両方をテスト
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 5.2 (P) バリデーションルールのユニットテストを追加
  - V9 更新（keyframe または trigger_storyboard が必須）を検証
  - V14 自己参照検出（A→A）を検証
  - V15 循環参照検出（A→B→A、A→B→C→A、深いチェーン）を検証
  - V16 排他チェック（trigger + variable）を検証
  - V17 トランジションフィールド拒否を検証
  - V18 トリガー対象存在確認を検証
  - _Requirements: 1.3, 1.5, 3.1, 3.2, 3.4_

- [ ] 5.3 (P) CompiledTrigger の fire_time 計算テストを追加
  - 4配置パターン（前エントリ連結、KF起点、KF間、純粋KF）× トリガーエントリの組み合わせで検証
  - `keyframe_times` にトリガー発火時刻が正しく登録されることを確認
  - `total_base_duration` にトリガーが寄与しないことを確認
  - _Requirements: 1.2, 1.4, 3.3_

- [ ] 5.4 update() トリガー実行の統合テストを追加
  - トリガー発火時に子ストーリーボードが自動開始されることを検証
  - `UpdateResult.triggered` に正しい結果が含まれることを検証
  - トリガー先の Never ポリシー競合時に `TriggerResult::Error` が記録されることを検証
  - _Requirements: 2.1, 2.2, 2.5_

- [ ] 5.5 ループ内トリガーの統合テストを追加
  - `loop_count >= 2` のストーリーボード内トリガーが周回ごとに再実行されることを検証
  - `loop_count = -1` の無限ループでトリガーが各周回で実行されることを検証
  - 複数トリガーの同時発火（同一 fire_time）を検証
  - _Requirements: 5.1, 5.2_

- [ ]* 5.6 E2E テストを追加
  - トリガーチェーン A→B→C の3段連鎖起動を検証
  - ループ + トリガー + 競合解決の組み合わせを検証
  - `load_document()` → `start()` → `update()` × N → 全インスタンス終了の完全シナリオを検証
  - _Requirements: 2.3, 2.4, 5.3, 5.4_

## Task Summary

- **Major Tasks**: 5
- **Sub-tasks**: 23
- **Estimated Hours**: 35-70 hours (1-3 hours per sub-task)
- **Parallel Tasks**: 14 tasks marked with (P)
- **Optional Tasks**: 1 task marked with *

## Progress Tracking

| Task | Status        | Requirements                      |
| ---- | ------------- | --------------------------------- |
| 1.1  | ⬜ Not Started | 1.1, 1.2, 4.1, 4.2, 4.3, 4.4, 4.5 |
| 1.2  | ⬜ Not Started | 1.4, 3.3, 4.3                     |
| 1.3  | ⬜ Not Started | 2.2, 2.5                          |
| 1.4  | ⬜ Not Started | 5.1, 5.2                          |
| 1.5  | ⬜ Not Started | 3.1, 3.2, 3.5                     |
| 2.1  | ⬜ Not Started | 1.4                               |
| 2.2  | ⬜ Not Started | 3.1                               |
| 2.3  | ⬜ Not Started | 3.2                               |
| 2.4  | ⬜ Not Started | 1.3                               |
| 2.5  | ⬜ Not Started | 3.4                               |
| 2.6  | ⬜ Not Started | 1.5                               |
| 3.1  | ⬜ Not Started | 1.2, 1.4, 3.3, 4.4                |
| 4.1  | ⬜ Not Started | 2.1, 5.1, 5.2                     |
| 4.2  | ⬜ Not Started | 2.1, 2.2, 2.3, 5.4                |
| 4.3  | ⬜ Not Started | 2.5                               |
| 4.4  | ⬜ Not Started | 5.1, 5.2                          |
| 4.5  | ⬜ Not Started | 2.4, 5.3                          |
| 5.1  | ⬜ Not Started | 4.1, 4.2, 4.3                     |
| 5.2  | ⬜ Not Started | 1.3, 1.5, 3.1, 3.2, 3.4           |
| 5.3  | ⬜ Not Started | 1.2, 1.4, 3.3                     |
| 5.4  | ⬜ Not Started | 2.1, 2.2, 2.5                     |
| 5.5  | ⬜ Not Started | 5.1, 5.2                          |
| 5.6  | ⬜ Not Started | 2.3, 2.4, 5.3, 5.4                |

## Requirements Coverage Check

✅ **All requirements mapped to tasks**:
- Requirement 1.1-1.5: Tasks 1.1, 1.2, 2.1, 2.4, 2.6, 3.1, 5.1, 5.2, 5.3
- Requirement 2.1-2.5: Tasks 4.1, 4.2, 4.3, 4.5, 5.4, 5.6
- Requirement 3.1-3.5: Tasks 1.2, 1.5, 2.2, 2.3, 2.5, 3.1, 5.2, 5.3
- Requirement 4.1-4.5: Tasks 1.1, 5.1
- Requirement 5.1-5.4: Tasks 1.4, 4.1, 4.4, 4.5, 5.5, 5.6
