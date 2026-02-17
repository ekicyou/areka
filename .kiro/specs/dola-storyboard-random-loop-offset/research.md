# Research & Design Decisions

## Summary
- **Feature**: `dola-storyboard-random-loop-offset`
- **Discovery Scope**: Extension
- **Key Findings**:
  - `loop_controller.rs` のフリー関数パターンを踏襲し、遅延処理を `advance_loop()` 拡張 + 新関数で実装可能
  - `easing.rs` の `EasingFunction` 型（`#[serde(untagged)]` + 30+ variants）がそのまま `loop_offset.easing` に再利用可能
  - `calculate_effective_time()` は `(current_time - loop_start_time - pause_accumulated) * time_scale` であり、遅延を `end_time` に wall clock ベースで加算すれば `time_scale` 非適用が自然に実現される

## Research Log

### 乱数クレート選定
- **Context**: dola クレートに乱数依存が存在しないため新規追加が必要（Cargo.toml 確認済み）
- **Sources Consulted**: Rust エコシステムの乱数クレート（`rand`, `fastrand`, `getrandom`）
- **Findings**:
  - `rand` は Rust 標準的な乱数クレート。`thread_rng()` で OS エントロピーベースの高品質乱数、`SmallRng` でテスト用の決定的シード
  - `fastrand` は軽量（依存なし）だが、trait ベースの DI パターンが弱い
  - `getrandom` は低レベルすぎて分布生成には不向き
- **Implications**: `rand` を採用。テスタビリティのため関数パラメータで `&mut impl Rng` を受け取る DI パターンを使用

### 遅延状態の表現方法
- **Context**: 周回完了後の遅延待機状態をどう表現するか（gap-analysis Research Needed #3）
- **Sources Consulted**: 既存の `InstanceState` enum（7 variants）、`StoryboardInstance` フィールド群
- **Findings**:
  - 案A: `InstanceState::WaitingDelay` 新バリアント → `try_transition()` の状態遷移グラフ変更が大規模
  - 案B: `StoryboardInstance` にフィールド追加（`delay_remaining: Option<f64>`）→ 既存パターン踏襲、最小変更
  - 案C: `LoopAction::WaitingDelay(f64)` バリアント → `process_loops` の呼び出し元が遅延管理する責務を持つ
- **Implications**: 案B + 案C のハイブリッドを採用。`StoryboardInstance` に遅延フィールドを追加しつつ、`LoopAction` にも `Delay` バリアントを追加して呼び出し元に遅延状態を通知

### イージングによる確率分布制御
- **Context**: `easing` フィールドで `[0,1]` 一様乱数を非線形変換し、ランダム遅延の分布を制御する
- **Sources Consulted**: `easing.rs` の `EasingFunction` 型、`interpolation` クレート
- **Findings**:
  - `EasingFunction::Named(EasingName::Linear)` → 一様分布（デフォルト）
  - `EasingFunction::Named(EasingName::QuadraticIn)` → 短い遅延に偏る（頻繁な瞬き）
  - `EasingFunction::Named(EasingName::QuadraticOut)` → 長い遅延に偏る（稀な瞬き）
  - `EasingFunction` は `ease()` メソッドを持っていない（`interpolation` クレートの `EaseFunction` にマッピングが必要）
- **Implications**: easing 適用は `interpolation::Ease::calc()` を使用。`EasingName` → `interpolation::EaseFunction` の変換ヘルパーが必要（またはマッチ式で直接計算）

### 既存ループ処理フローとの統合
- **Context**: `facade.rs::update()` → `loop_controller::process_loops()` → `advance_loop()` の呼び出しチェーン
- **Sources Consulted**: `facade.rs` (353 lines), `loop_controller.rs` (251 lines)
- **Findings**:
  - `update()` Step 2: `current_time >= end_time` の条件でフィルタ → `process_loops()` 呼び出し
  - `process_loops()`: while ループで `current_time >= end_time` を繰り返し、`advance_loop()` で `end_time += loop_duration`
  - 遅延注入ポイント: `advance_loop()` 内で遅延を生成し `end_time += delay` とすれば、while ループの次イテレーションで `current_time < end_time` となり自然に待機状態に入る
  - 複数周回スキップ: 遅延ありの場合、1回の `advance_loop()` で遅延が加算されるため、while ループは最大1回の追加周回で停止する
- **Implications**: `advance_loop()` のシグネチャを拡張し、遅延パラメータと乱数ソースを受け取る設計

### Pause/Resume と遅延の整合性
- **Context**: Req 5.1 — 遅延待機中の Pause で残り時間を保持し Resume で再開
- **Sources Consulted**: `instance_manager.rs` の `pause()`, `resume()`, `set_pause_start()`
- **Findings**:
  - 既存パターン: `pause_start` 記録 → Resume 時に `pause_accumulated += current_time - pause_start` → `end_time += pause_duration`
  - 遅延待機中も同じメカニズムで対応可能: `end_time` は遅延分を含んでいるため、Pause/Resume の `end_time` 延長が遅延残り時間を自然に保存する
  - 追加フィールドは不要 — `end_time` が全ての「待つべき時刻」を包含
- **Implications**: 遅延中の Pause/Resume は既存メカニズムで完全に対応。特別な処理は不要

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 既存コンポーネント拡張 | `loop_controller.rs` のフリー関数 + `StoryboardInstance` フィールド追加 | 確立パターン踏襲、最小変更 | `advance_loop()` の引数増加 | **推奨** |
| B: 新モジュール `delay_controller.rs` | 遅延管理を独立モジュール化 | 関心分離 | `loop_controller` との密結合、過度な抽象化 | 機能規模に不釣り合い |
| C: ハイブリッド | A ベース + `process_delay()` 分離関数 | 呼び出し順序が明示的 | `facade.rs` での呼び出し管理が複雑化 | |

## Design Decisions

### Decision: 遅延状態のフィールド表現
- **Context**: 遅延待機中であることをランタイムがどう認識するか
- **Alternatives Considered**:
  1. `InstanceState::WaitingDelay` 新バリアント — 状態遷移グラフの大規模変更
  2. `StoryboardInstance` フィールド追加 — 既存パターン踏襲
  3. `LoopAction` バリアント追加 — 呼び出し元への通知のみ
- **Selected Approach**: `StoryboardInstance` に `delay_end_time: Option<f64>` を追加。遅延中は `delay_end_time.is_some()` で判定。`LoopAction` には `WaitingDelay` は追加せず、`Continue` で統一（遅延中は `current_time < end_time` なので自然に `Continue` になる）
- **Rationale**: `end_time` に遅延を加算する方式なら、`process_loops()` の while ループ条件 `current_time >= end_time` が遅延終了の判定を兼ねる。`delay_end_time` はデバッグ・ログ用途
- **Trade-offs**: ✅ 最小変更、既存テスト互換 ❌ 遅延状態の明示性が低い（`end_time` の暗黙的意味変化）
- **Follow-up**: ログ出力で遅延状態を明示的に表示

### Decision: 乱数 DI パターン
- **Context**: テスト時に決定的な乱数を注入する必要がある
- **Alternatives Considered**:
  1. `trait RngSource` DI — 型パラメータ汚染
  2. `&mut impl Rng` 関数パラメータ — シンプルだがシグネチャ変更
  3. クロージャ `Fn() -> f64` — 最も柔軟
- **Selected Approach**: `generate_delay()` 関数が `&mut impl Rng` を受け取る。`process_loops()` にも `rng` パラメータを追加。`facade.rs` では `thread_rng()` を使用
- **Rationale**: Rust の `rand` エコシステム標準パターン。`SmallRng::seed_from_u64()` でテスト時の再現性を確保
- **Trade-offs**: ✅ テスト容易性、標準的 ❌ `process_loops()` のシグネチャ変更（既存テスト修正が必要）
- **Follow-up**: 既存の `process_loops` テストに `&mut thread_rng()` を追加（マイナー修正）

### Decision: LoopOffset serde 表現
- **Context**: Req 4 — 短縮形（数値）とオブジェクト形式の両サポート
- **Alternatives Considered**:
  1. `#[serde(untagged)]` enum — `KeyframeRef` と同パターン
  2. カスタム Deserialize 実装 — 柔軟だが複雑
- **Selected Approach**: `#[serde(untagged)]` enum。`Scalar(f64)` → `max` として解釈（`min=0.0`, `easing=Linear`）、`Range { min, max, easing }` → フルスペック
- **Rationale**: `KeyframeRef`, `TransitionRef` で確立済みのパターン
- **Trade-offs**: ✅ codebase 一貫性 ❌ `untagged` のデシリアライズ順序依存性（Scalar を先に定義で解決）
- **Follow-up**: serde テストで両形式の round-trip を検証

### Decision: time_scale 非適用の実現方法
- **Context**: Req 2.6 — `time_scale` はアニメーション再生速度のみ、遅延には適用しない
- **Selected Approach**: 遅延を wall clock ベースで `end_time` に直接加算。`calculate_effective_time() = (current_time - loop_start_time - pause_accumulated) * time_scale` の `time_scale` 乗算の外で遅延が処理される
- **Rationale**: 既存アーキテクチャの自然な帰結。`end_time` は wall clock ベースの絶対時刻であり、遅延を wall clock で加算すれば `time_scale` の影響を受けない
- **Trade-offs**: ✅ 変更なしで実現 ❌ なし

## Risks & Mitigations
- `rand` クレート追加による依存増加 — `rand` は広く使用されており事実上のリスクなし
- `process_loops()` シグネチャ変更 — 既存テストの `thread_rng()` 追加で対応（軽微）
- 複数周回スキップ時の遅延蓄積 — 遅延ありの場合、while ループは最大1追加周回で停止するため逸脱なし

## References
- `rand` crate: https://docs.rs/rand/ — Rust 標準乱数ライブラリ
- `interpolation` crate: https://docs.rs/interpolation/ — イージング関数の計算基盤（既存依存）
- SERIKO animation.interval: 伺かの定義書式。`sometimes`, `rarely`, `random` 等のランダム再生パターン
