# ギャップ分析: dola-storyboard-random-loop-offset

## 1. 現状調査

### 1.1 関連モジュール・ファイル配置

| ファイル                                      | 責務                                   | 本機能への関連度                                                                                  |
| --------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `crates/dola/src/storyboard.rs`               | `Storyboard` 構造体定義（serde対応）   | **高**: `loop_offset` フィールド追加先                                                            |
| `crates/dola/src/runtime/loop_controller.rs`  | ループ周回判定・進行（フリー関数群）   | **高**: ランダム遅延ロジックの主実装先                                                            |
| `crates/dola/src/runtime/instance_manager.rs` | `StoryboardInstance` 構造体 + 状態遷移 | **高**: 遅延状態フィールド追加先                                                                  |
| `crates/dola/src/runtime/facade.rs`           | `DolaRuntime` Facade API               | **中**: `update()` 内ループ処理フロー変更                                                         |
| `crates/dola/src/validate.rs`                 | ドキュメントバリデーション             | **中**: loop_offset 検証ルール追加先                                                              |
| `crates/dola/src/error.rs`                    | `DolaError` バリアント定義             | **中**: 新バリデーションエラー追加                                                                |
| `crates/dola/src/compile.rs`                  | ストーリーボードコンパイラ             | **低**: `CompiledStoryboard` にメタ情報転送                                                       |
| `crates/dola/src/runtime/timeline_manager.rs` | タイムテーブル管理・評価               | **低**: `calculate_effective_time()` は変更不要（遅延分は `loop_start_time` / `end_time` で吸収） |
| `crates/dola/Cargo.toml`                      | クレート依存関係                       | **中**: `rand` クレート追加が必要                                                                 |

### 1.2 既存パターン・設計規約

- **ループ制御**: `loop_controller.rs` のフリー関数パターン（Decision: `dola-runtime-5-loop`）。状態は全て `StoryboardInstance` に保持、純粋関数で操作
- **時間管理**: `loop_start_time` + `loop_duration` 方式。`advance_loop()` で `loop_start_time += loop_duration`, `end_time += loop_duration`
- **Pause/Resume**: `pause_accumulated` フィールドで加算管理。`end_time += pause_duration` で延長
- **イージングシステム**: `easing.rs` に `Easing` enum と `EasingFunction` trait が確立済み。linear, ease_in/out/in_out, cubic, elastic 等の豊富なバリエーション。serde対応済み
- **serde 多形**: `#[serde(untagged)]` による短縮形サポート（`KeyframeRef`, `TransitionRef` と同パターン）
- **バリデーション**: `Validate` トレイトの `validate()` メソッド内で `V1`〜`V13` のルール適用。エラーは `Vec<DolaError>` で一括収集
- **警告 vs エラー**: 現在のバリデーションは全てエラー（`DolaError`）。**警告の仕組みは未実装**（Research Needed）

### 1.3 依存関係の現状

- **乱数ライブラリ**: dola クレートに `rand` 系の依存は**存在しない**。新規追加が必要
- **イージングシステム**: `easing.rs` が既に実装済み。Easing型のserde対応も完了しており、loop_offsetのeasingフィールドで再利用可能
- **serde**: 既に `derive` feature 有効。`#[serde(untagged)]` パターンは複数箇所で確立済み

## 2. 要件 → 既存アセット対応マップ

| 要件                                   | 対象アセット                                             | ギャップ                                                                                                                                                                        |
| -------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Req 1**: loop_offset フィールド定義  | `storyboard.rs::Storyboard`                              | **Missing**: フィールド・型が存在しない。**Partial**: easingフィールドは既存のEasing型を再利用可能                                                                              |
| **Req 2**: ランタイムランダム遅延      | `loop_controller.rs::process_loops()` / `advance_loop()` | **Missing**: 遅延待機状態・乱数生成が未実装。**Exists**: イージング適用は `easing.rs::EasingFunction::ease()` で実現可能                                                        |
| **Req 3**: バリデーション              | `validate.rs` / `error.rs`                               | **Missing**: loop_offset 検証なし（min/max範囲、easing妥当性）。**Unknown**: 警告の仕組みが未存在                                                                               |
| **Req 4**: time_scale 非適用           | `timeline_manager.rs::calculate_effective_time()`        | **設計上対応可能**: 遅延を `loop_start_time` / `end_time` に実時間ベースで加算すれば `time_scale` 乗算の外になる                                                                |
| **Req 5**: 短縮形serde                 | `storyboard.rs`                                          | **Missing**: 新型（`LoopOffset` enum）が必要。パターンは `KeyframeRef` に確立済み。easingフィールドのデフォルト「linear」はserde(default)で実現可能                             |
| **Req 6**: Pause/Cancel/割り込み整合性 | `instance_manager.rs` / `facade.rs`                      | **Constraint**: 既存の Pause/Resume は `pause_accumulated` + `end_time` 延長方式。遅延待機中の Pause は同じパターンで対応可能だが、**遅延残り時間の追跡**には新フィールドが必要 |

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張（推奨）

既存の `loop_controller.rs` フリー関数パターンと `StoryboardInstance` フィールド追加方式を踏襲し、最小限の変更で機能追加する。

**変更ファイル一覧**:

| 操作                   | ファイル                                  | 内容                                                                                                                                                |
| ---------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **新型追加**           | `storyboard.rs`                           | `LoopOffset` enum（短縮形: `Scalar(f64)` / `Range { min, max, easing }`）                                                                           |
| **フィールド追加**     | `storyboard.rs::Storyboard`               | `loop_offset: Option<LoopOffset>`                                                                                                                   |
| **フィールド追加**     | `instance_manager.rs::StoryboardInstance` | `loop_offset_min: f64`, `loop_offset_max: f64`, `loop_offset_easing: Easing`, `current_delay_remaining: Option<f64>`                                |
| **ロジック拡張**       | `loop_controller.rs`                      | `advance_loop()` に遅延計算（乱数生成 + イージング適用 + [min,max]マッピング）を追加。新関数 `is_in_delay()`, `process_delay()`, `generate_delay()` |
| **フロー修正**         | `facade.rs::start()`                      | `loop_offset` 情報を `StoryboardInstance` に伝達                                                                                                    |
| **フロー修正**         | `facade.rs::update()`                     | ループ処理部でdelay状態を考慮                                                                                                                       |
| **メタ転送**           | `compile.rs::CompiledStoryboard`          | `loop_offset` 情報の転送（またはfacadeが直接参照）                                                                                                  |
| **バリデーション追加** | `validate.rs`                             | 4件の検証ルール（V14〜V17相当）                                                                                                                     |
| **エラー追加**         | `error.rs`                                | `InvalidLoopOffset` 系バリアント                                                                                                                    |
| **依存追加**           | `Cargo.toml`                              | `rand` クレート                                                                                                                                     |

**トレードオフ**:
- ✅ 既存のフリー関数パターン（`dola-runtime-5-loop` Decision）を踏襲
- ✅ `StoryboardInstance` フィールド追加は確立されたパターン
- ✅ `advance_loop()` の拡張で遅延注入が自然にフィット
- ❌ `advance_loop()` のロジックが複雑化（遅延なし/あり分岐）

### Option B: 新コンポーネント作成

`runtime/delay_controller.rs` として遅延管理を独立モジュール化。

**トレードオフ**:
- ✅ loop_controller の既存ロジックに影響なし
- ✅ 遅延ロジックの独立テスト
- ❌ `loop_controller` との責務分離が不明瞭（密結合になりやすい）
- ❌ `StoryboardInstance` は結局変更が必要
- ❌ 過度な抽象化（機能規模に対して）

### Option C: ハイブリッド

`loop_controller.rs` 内に遅延関連フリー関数を追加しつつ、`LoopOffset` 型定義は `storyboard.rs` に配置（Option A ベース）。`process_loops()` のシグネチャは変更せず、遅延処理を `process_delay()` として分離。

**トレードオフ**:
- ✅ 既存テストの壊れにくさ（`process_loops` は変更最小限）
- ✅ 遅延処理が明示的に分離
- ❌ `facade.rs::update()` での呼び出し順序管理が必要

## 4. 複雑度・リスク評価

| 項目       | 評価            | 根拠                                                                                                            |
| ---------- | --------------- | --------------------------------------------------------------------------------------------------------------- |
| **工数**   | **S（1〜3日）** | 既存パターンの拡張。データモデル変更 + フリー関数追加 + バリデーション。アーキテクチャ変更なし                  |
| **リスク** | **Low**         | 既知のパターン、明確なスコープ、最小限の外部依存追加（`rand`）。`advance_loop()` の拡張は既存テストでカバー可能 |

### リスク詳細

| リスク                                  | 重大度     | 対策                                                                                    |
| --------------------------------------- | ---------- | --------------------------------------------------------------------------------------- |
| `rand` クレート追加による依存肥大       | **Low**    | `rand` は軽量で広く使われている。`getrandom` のみ必要なら最小依存も可                   |
| 遅延待機中の Pause/Resume 整合性        | **Medium** | `pause_accumulated` + `end_time` 延長パターンが確立済み。遅延も同パターンで吸収可能     |
| 警告 vs エラーの仕組み未存在（Req 3-4） | **Medium** | 設計フェーズで方針決定が必要（Research Needed）                                         |
| 複数周回スキップ時の遅延処理            | **Low**    | `process_loops()` の while ループ内で遅延を加算すれば、複数周回スキップも正確に処理可能 |

## 5. Research Needed（設計フェーズ持ち越し）

1. **警告メカニズム**: 現在の `DolaError` は全てエラー。Req 3-4 の「警告」をどう表現するか
   - 案A: `DolaError` に `Warning` 系バリアントを追加し、`validate()` の戻り値を `Result<Vec<DolaWarning>, Vec<DolaError>>` に変更
   - 案B: `DiagnosticLevel::Warning | Error` を `DolaError` に内包
   - 案C: 警告はログ出力のみとし、バリデーション結果には含めない
2. **乱数ソース**: `rand` クレートの `thread_rng()` vs テスト可能な乱数注入パターン
   - テスト時に確定的な結果を得るための `trait RngSource` 等のDI設計
3. **遅延状態の表現**: `StoryboardInstance` に遅延フィールドを追加する方式 vs `LoopAction::WaitingDelay` バリアント追加方式

## 6. 設計フェーズへの推奨事項

- **推奨アプローチ**: **Option A**（既存コンポーネント拡張）をベースに、`process_loops()` と遅延処理の呼び出し分離（Option C のエッセンス）を取り入れる
- **キー設計決定**:
  1. `LoopOffset` 型の serde 表現（`#[serde(untagged)]` パターン確認済み）
  2. 遅延状態の `StoryboardInstance` フィールド設計
  3. 乱数注入パターン（テスタビリティ）
  4. 警告メカニズムの方針
- **実装順序案**: データモデル → バリデーション → ランタイム拡張 → テスト → Pause/Cancel統合テスト
