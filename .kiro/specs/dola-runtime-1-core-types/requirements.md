# Requirements Document — dola-runtime-1-core-types

## Introduction

本ドキュメントは dola ランタイムエンジンの基盤型を定義する子仕様 `dola-runtime-core-types` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 8（ストーリーボード状態遷移）と Req 10（イージング関数）を子仕様の粒度に詳細化する。

本子仕様は Tier 1（基盤）に位置し、他の子仕様への依存を持たない。ここで定義する型（`InstanceState`, `EvaluatedValue`, `RuntimeError`, `StartResult`）と補間計算（`Interpolator`）は、Tier 2 以降の子仕様が共通基盤として消費する。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` を参照

---

## Requirements

### Requirement 1: InstanceState — 実行インスタンスの状態管理

_Parent: Req 8.1, 8.4_

**Objective:** ランタイム実装者として、ストーリーボード実行インスタンスの7つの状態を型安全に管理したい。これにより、不正な状態遷移をコンパイル時・実行時に検出できる。

#### Acceptance Criteria

1. The InstanceState enum shall 7バリアント（Created, Playing, Paused, Concluded, Cancelled, Trimmed, Compressed）を定義する。
2. The InstanceState enum shall `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` を derive する。シリアライズは不要（ランタイム内部専用）。
3. The InstanceState enum shall 終了状態判定メソッド `is_terminal() -> bool` を提供し、Concluded / Cancelled / Trimmed / Compressed で `true` を返す。
4. The InstanceState enum shall `InterruptionPolicy` から対応する終了状態への変換メソッド `from_policy(policy: InterruptionPolicy) -> Option<Self>` を提供する（Cancel→Some(Cancelled), Conclude→Some(Concluded), Trim→Some(Trimmed), Compress→Some(Compressed)）。Never に対しては `None` を返す（Never は終了状態ではなく延期戦略であり、競合解決時に正常に呼ばれ得る）。

---

### Requirement 2: InstanceState — 状態遷移ルール

_Parent: Req 8.1, 8.2, 8.3, 8.4_

**Objective:** ランタイム実装者として、状態遷移の正当性を検証する仕組みが欲しい。これにより、不正な遷移（例: 終了状態からの復帰）を確実に拒否できる。

#### Acceptance Criteria

1. The InstanceState shall 遷移検証メソッド `try_transition(target: InstanceState) -> Result<InstanceState, InstanceState>` を提供する。遷移成功時は `Ok(target)`、失敗時は `Err(self)` を返す。ドメイン層の責務として RuntimeError に依存せず、InstanceManager が必要に応じて InvalidGroupId へ変換する。
2. When Created → Playing の遷移が要求された場合, the InstanceState shall `Ok(Playing)` を返す。
3. When Playing → Paused の遷移が要求された場合, the InstanceState shall `Ok(Paused)` を返す。
4. When Paused → Playing の遷移が要求された場合, the InstanceState shall `Ok(Playing)` を返す。
5. When Playing または Paused から終了状態（Concluded / Cancelled / Trimmed / Compressed）への遷移が要求された場合, the InstanceState shall `Ok(target)` を返す。
6. When 上記以外の遷移が要求された場合（終了状態からの遷移、Created → Paused など）, the InstanceState shall `Err(self)` を返す。InstanceManager の実装が正しければ発生しない（内部バグのみ）。

---

### Requirement 3: EvaluatedValue — 評価済み変数値型

_Parent: Req 10（間接）、統合指針 Section 3.1_

**Objective:** ランタイム実装者として、補間計算の結果を表現する型安全な値型が欲しい。これにより、`VariableTypeHint` との1対1対応を保証でき、上位レイヤーが型分岐なしで値を取得できる。

#### Acceptance Criteria

1. The EvaluatedValue enum shall 3バリアント（`Float(f64)`, `Integer(i64)`, `Object(DynamicValue)`）を定義する。
2. The EvaluatedValue enum shall `Debug`, `Clone`, `PartialEq` を derive する。
3. The EvaluatedValue enum shall `Display` trait を実装し、Float は `{:.6}` 形式、Integer は整数表記、Object は `Debug` 形式で出力する。
4. The EvaluatedValue enum shall `VariableTypeHint` の各バリアントと1対1対応を持つ: Float↔Float, Integer↔Integer, Object↔Object。

---

### Requirement 4: RuntimeError — エラー型

_Parent: Req 1.5, 2.8, 2.9, 3.7、統合指針 Section 3.1_

**Objective:** ランタイム実装者として、全子仕様が共通利用するエラー型が欲しい。これにより、facade API のエラーハンドリングを統一し、呼び出し側が網羅的パターンマッチで対応できる。

#### Acceptance Criteria

1. The RuntimeError enum shall 以下のバリアントを定義する:
   - `StoryboardNotFound(String)` — 存在しないストーリーボード名（Parent Req 2.8）
   - `InvalidGroupId(u64)` — 存在しない group_id（終了済みインスタンスへの操作を含む。終了インスタンスは即座に削除される設計）
   - `DocumentParseError(String)` — TOML パース失敗（Parent Req 1.5）
   - `InvalidLoopCount(i32)` — 不正な loop_count 値(0 以下、-1 を除く)
   - `ZeroDurationWithLoop { storyboard: String }` — duration=0 + loop_count=-1(Parent Req 2.9)
   - `CompileError(Vec<DolaError>)` — 既存コンパイルエラーのラップ（`compile_storyboard()` が `Vec<DolaError>` を返すため）
2. The RuntimeError enum shall `Debug`, `Clone` を derive する。
3. The RuntimeError enum shall `std::fmt::Display` と `std::error::Error` を実装する。
4. The RuntimeError enum shall `From<Vec<DolaError>>` を実装し、コンパイルエラーの自動変換を提供する。

---

### Requirement 5: StartResult — Start 返却値

_Parent: Req 2.5、統合指針 Section 3.1_

**Objective:** ランタイム実装者として、Start コマンドの結果を構造体として返却したい。これにより、group_id と終了予定時刻をオーケストレーターが型安全に受け取れる。

#### Acceptance Criteria

1. The StartResult struct shall `group_id: u64` フィールドを持つ。
2. The StartResult struct shall `end_time: f64` フィールドを持つ。
3. The StartResult struct shall `Debug`, `Clone`, `PartialEq` を derive する。

---

### Requirement 6: Interpolator — イージング適用と補間計算

_Parent: Req 10.1, 10.2, 10.3, 10.4_

**Objective:** ランタイム実装者として、`CompiledSegment` の進捗率 `t` に基づいてイージングを適用し、補間値を計算する機能が欲しい。これにより、TimelineManager が補間ロジックを知らずに値を評価できる。

#### Acceptance Criteria

1. The Interpolator shall `interpolate(segment: &CompiledSegment, variable_type: &VariableTypeHint, progress_t: f64) -> EvaluatedValue` メソッドを提供する。
2. When `EasingFunction::Named(name)` が指定されている場合, the Interpolator shall `EasingName` を `interpolation::EaseFunction` にマッピングして `Ease::calc(eased_t)` で補間率を計算する。`EasingName::Linear` はマッピング不要で `t` をそのまま使用する。
3. When `EasingFunction::Parametric(ParametricEasing::QuadraticBezier { x0, x1, x2 })` が指定されている場合, the Interpolator shall `interpolation::quad_bez(&x0, &x1, &x2, &progress_t)` で補間率を計算する。
4. When `EasingFunction::Parametric(ParametricEasing::CubicBezier { x0, x1, x2, x3 })` が指定されている場合, the Interpolator shall `interpolation::cub_bez(&x0, &x1, &x2, &x3, &progress_t)` で補間率を計算する。
5. When `easing` が `None` の場合, the Interpolator shall 線形補間（`t` をそのまま使用）を適用する。
6. The Interpolator shall `progress_t` を `0.0..=1.0` にクランプしてから処理する。
7. When `VariableTypeHint::Float` の場合, the Interpolator shall `from` と `to` の `Scalar(f64)` 値を線形補間し、`EvaluatedValue::Float` を返す。
8. When `VariableTypeHint::Integer` の場合, the Interpolator shall `from` と `to` の `Scalar(f64)` 値を線形補間し、`round()` で i64 に丸めて `EvaluatedValue::Integer` を返す。
9. When `VariableTypeHint::Object` の場合, the Interpolator shall 補間を行わず、`progress_t >= 1.0` なら `to_value`、それ以外は `from_value` を `EvaluatedValue::Object` として返す。

---

### Requirement 7: EasingName マッピング — 名前付きイージングの完全対応

_Parent: Req 10.1, 10.2_

**Objective:** ランタイム実装者として、dola 既存の `EasingName` 30バリアントと `interpolation::EaseFunction` 30バリアントの1対1マッピングが正確に行われることを保証したい。

#### Acceptance Criteria

1. The Interpolator shall `EasingName` の30バリアント全てに対して `interpolation::EaseFunction` への1対1マッピングを提供する。
2. The mapping shall 以下の対応を持つ: QuadraticIn↔QuadraticIn, QuadraticOut↔QuadraticOut, QuadraticInOut↔QuadraticInOut, CubicIn↔CubicIn, CubicOut↔CubicOut, CubicInOut↔CubicInOut, QuarticIn↔QuarticIn, QuarticOut↔QuarticOut, QuarticInOut↔QuarticInOut, QuinticIn↔QuinticIn, QuinticOut↔QuinticOut, QuinticInOut↔QuinticInOut, SineIn↔SineIn, SineOut↔SineOut, SineInOut↔SineInOut, CircularIn↔CircularIn, CircularOut↔CircularOut, CircularInOut↔CircularInOut, ExponentialIn↔ExponentialIn, ExponentialOut↔ExponentialOut, ExponentialInOut↔ExponentialInOut, ElasticIn↔ElasticIn, ElasticOut↔ElasticOut, ElasticInOut↔ElasticInOut, BackIn↔BackIn, BackOut↔BackOut, BackInOut↔BackInOut, BounceIn↔BounceIn, BounceOut↔BounceOut, BounceInOut↔BounceInOut。
3. The mapping shall テストで全30バリアントのマッピング正確性を検証する。

