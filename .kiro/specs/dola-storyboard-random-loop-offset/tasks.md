# Implementation Plan

## Task Overview

実装を論理的な段階に分割し、定義層→コンパイル層→ランタイム層→バリデーション層→テストの順で進行する。並列実行可能なタスクには `(P)` マークを付与。

---

## Tasks

- [ ] 1. ループオフセット型の定義
- [ ] 1.1 `LoopOffset` enum と `LoopOffsetRange` struct の実装
  - `storyboard.rs` に `LoopOffset` 型を定義（`#[serde(untagged)]` で `Scalar(f64)` と `Range(LoopOffsetRange)` バリアント）
  - `LoopOffsetRange` struct に `min: f64`, `max: f64`, `easing: EasingFunction` フィールドを定義
  - `default_easing_linear()` ヘルパー関数を実装（`EasingFunction::Named(EasingName::Linear)` を返す）
  - `#[serde(default)]` と `#[serde(default = "default_easing_linear")]` 属性を適用
  - 短縮形（スカラー）がデシリアライズ優先されるよう `Scalar` を先に定義
  - _Requirements: 1.2, 1.4, 1.5, 4.1, 4.2_

- [ ] 1.2 (P) serde ラウンドトリップテストの追加
  - スカラー短縮形（`3.0`）のデシリアライズ/シリアライズ検証
  - オブジェクト形式（`{ min, max, easing }`）のラウンドトリップ検証
  - `easing` 省略時のデフォルト値（`Linear`）検証
  - パラメトリックイージング（`cubic_bezier` 等）のラウンドトリップ検証
  - _Requirements: 1.4, 4.1, 4.2, 4.3_

- [ ] 2. ストーリーボード定義への統合
- [ ] 2.1 `Storyboard` と `CompiledStoryboard` への `loop_offset` フィールド追加
  - `Storyboard` struct に `loop_offset: Option<LoopOffset>` フィールドを追加
  - `#[serde(default, skip_serializing_if = "Option::is_none")]` 属性で後方互換性を確保
  - `CompiledStoryboard` struct にも同じフィールドを追加
  - コンパイラの `compile_storyboard()` で `loop_offset` をそのまま転送
  - _Requirements: 1.1, 1.3_

- [ ] 2.2 (P) コンパイルパイプラインの統合テスト
  - `loop_offset` ありのストーリーボード定義をコンパイルし、`CompiledStoryboard` に正しく反映されるかを検証
  - `loop_offset` なしの定義でも従来通りコンパイル成功することを検証
  - JSON/TOML/YAML 各フォーマットでの統合を確認
  - _Requirements: 1.1, 1.3, 1.4_

- [ ] 3. ランダム遅延のランタイム実装
- [ ] 3.1 `rand` クレート依存の追加と乱数DI設計
  - `crates/dola/Cargo.toml` に `rand` クレートを追加
  - `loop_controller.rs` に `use rand::Rng;` を追加
  - `process_loops()` のシグネチャを拡張（`rng: &mut impl Rng` パラメータ追加）
  - `facade.rs::update()` から `process_loops()` 呼び出し時に `&mut rand::thread_rng()` を渡す
  - _Requirements: 2.1, 2.5_

- [ ] 3.2 `generate_delay()` 関数の実装
  - `loop_controller.rs` に `generate_delay(min, max, easing, rng)` 関数を追加
  - `min == max` の場合は early return で固定遅延を返す（research.md Decision参照）
  - 一様乱数生成（`rng.gen_range(0.0..1.0)`）
  - `apply_easing()` ヘルパー関数でイージング適用（`EasingName` → `interpolation::EaseFunction` マッピング）
  - `[min, max]` へのマッピング（`min + eased * (max - min)`）
  - _Requirements: 2.1, 2.5, 2.6_

- [ ] 3.3 `StoryboardInstance` への遅延パラメータフィールド追加
  - `playback.rs` の `StoryboardInstance` に3フィールドを追加（`loop_offset_min: Option<f64>`, `loop_offset_max: f64`, `loop_offset_easing: EasingFunction`）
  - `facade.rs::start()` で `CompiledStoryboard::loop_offset` から値を展開してフィールドに設定
  - `loop_offset_min: None` の場合はオフセットなしとして扱う
  - _Requirements: 2.1, 2.4, 2.6_

- [ ] 3.4 `advance_loop()` への遅延生成ロジック統合
  - `advance_loop()` のシグネチャを拡張（`rng: &mut impl Rng` パラメータ追加）
  - `loop_offset_min.is_some()` の場合に `generate_delay()` を呼び出し
  - 生成した遅延を `instance.end_time += delay` で加算
  - `loop_offset_min.is_none()` の場合は遅延 0（既存動作維持）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6_

- [ ] 3.5 (P) 遅延生成の単体テスト
  - `generate_delay()` を固定シード `SmallRng` で決定的にテスト
  - 各イージング関数（`Linear`, `QuadraticIn`, `QuadraticOut` 等）で分布が異なることを検証
  - `min == max` エッジケースで固定遅延（常に `min`）が返されることを検証
  - 生成値が常に `[min, max]` 範囲内であることを検証
  - _Requirements: 2.1, 2.5_

- [ ] 3.6 (P) ループ周回動作の統合テスト
  - `advance_loop()` with delay: 遅延が `end_time` に正しく加算されるか
  - `process_loops()` with delay: while ループが遅延で正しく停止するか（複数周回スキップなし）
  - `loop_count = 1` + `loop_offset` 定義済み → 遅延が無視されることを確認
  - 無限ループ（`loop_count = -1`）+ `loop_offset` で各周回に遅延が適用されることを確認
  - _Requirements: 2.2, 2.3, 2.4_

- [ ] 4. Pause/Resume/Cancel との整合性確保
- [ ] 4.1 既存の Pause/Resume メカニズムとの動作確認
  - 遅延待機中に Pause → 残り時間保持（既存 `end_time` 延長メカニズムで自動対応）の挙動を検証
  - Resume 時に遅延残り時間から再開することを検証
  - 遅延待機中に Cancel → 即座に中断されることを検証
  - 遅延待機中の割り込み（InterruptionPolicy）が正しく動作することを検証
  - _Requirements: 5.1, 5.2, 5.3_

- [ ] 5. (P) バリデーションルールの実装
- [ ] 5.1 (P) `DolaError` への新バリアント追加
  - `error.rs` に `LoopOffsetNegativeMin { storyboard: String, value: f64 }` を追加
  - `LoopOffsetNegativeMax { storyboard: String, value: f64 }` を追加
  - `LoopOffsetRangeInverted { storyboard: String, min: f64, max: f64 }` を追加
  - 各バリアントの `Display` 実装でエラーメッセージを提供
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 5.2 (P) `validate.rs` への V14-V17 ルール追加
  - `validate_loop_offset()` 関数を実装
  - V14: `loop_offset.min < 0` → `LoopOffsetNegativeMin` エラー
  - V15: `loop_offset.max < 0` → `LoopOffsetNegativeMax` エラー（スカラー短縮形の場合も対応）
  - V16: `loop_offset.min > loop_offset.max` → `LoopOffsetRangeInverted` エラー
  - V17: `easing` の妥当性は serde の型システムが保証（追加バリデーション不要）
  - `impl Validate for DolaDocument` 内の storyboard ループで `validate_loop_offset()` を呼び出す
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 5.3 (P) バリデーションの単体テスト
  - V14-V17 各ルールの正常ケース・異常ケースを網羅的にテスト
  - スカラー短縮形の負値検証
  - オブジェクト形式の各フィールド検証
  - デシリアライズ時の不正 `easing` 値の検証（serde エラー）
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 6. E2E 統合テストとパフォーマンス検証
- [ ] 6.1 E2E フロー統合テスト
  - ストーリーボード定義（遅延あり/なし）→ コンパイル → ランタイム実行の完全フローを検証
  - 実際に遅延が発生し、周回間に待機時間が挿入されることを時間計測で確認
  - JSON/TOML/YAML 各フォーマットでの E2E 動作確認
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [ ] 6.2* パフォーマンスとエッジケース検証
  - 無限ループ + 遅延の長時間実行で `f64` 精度劣化がないことを確認
  - `generate_delay()` のオーバーヘッド測定（ループ周回ごとの乱数生成コスト）
  - 極端に小さい/大きい遅延値（`0.001` 秒、`3600.0` 秒）での動作確認
  - _Requirements: 2.1, 2.5_

---

## Progress Tracking

- **Total**: 6 major tasks, 15 sub-tasks
- **Parallel Candidates**: 1.2, 2.2, 3.5, 3.6, 4.1, 5.1, 5.2, 5.3 (8 sub-tasks)
- **Coverage**: All 21 acceptance criteria (Requirements 1.1-5.3) mapped to tasks

## Next Steps

1. タスク一覧をレビューし、必要に応じて調整
2. `/kiro-spec-impl dola-storyboard-random-loop-offset <task-ids>` で実装開始
3. 各タスク完了後にテストを実行し、品質を確認
