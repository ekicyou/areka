# Design Document — dola-runtime-1-core-types

## Overview

**Purpose**: dola ランタイムエンジンの基盤型（`InstanceState`, `EvaluatedValue`, `RuntimeError`, `StartResult`）およびイージング補間計算（`Interpolator`）を提供する。Tier 2 以降の子仕様が共通基盤として消費する最下層モジュール。

**Users**: Tier 2 `dola-runtime-facade`（InstanceManager, TimelineManager 等）および Tier 3 `dola-runtime-conflict-loop`（ConflictResolver, LoopController）が直接消費する。

**Impact**: 既存 dola クレートに `runtime` サブモジュールのうち `instance_state.rs`, `types.rs`, `interpolator.rs` を追加。`Cargo.toml` に `interpolation` オプショナル依存を追加。既存コードの変更なし。

### Goals

- 7バリアント `InstanceState` と状態遷移ロジックの型安全な実装
- `interpolation` クレートとの1対1マッピングによるイージング計算
- 全子仕様が共通利用するエラー型・値型の統一定義
- feature gate `runtime` による有効化制御

### Non-Goals

- facade API の実装（Tier 2 `dola-runtime-facade` の責務）
- 競合解決・ループ制御のロジック（Tier 3 `dola-runtime-conflict-loop` の責務）
- 時刻取得ユーティリティ（`dola-runtime-clock` の責務）
- 既存 dola データモデル型の変更

---

## Architecture

### Existing Architecture Analysis

既存 dola クレートの型を消費する:

| 既存型 | 定義元モジュール | 本子仕様での用途 |
|--------|-----------------|-----------------|
| `InterruptionPolicy` | `storyboard.rs` | `InstanceState::from_policy()` 変換元 |
| `CompiledSegment` | `compile.rs` | `Interpolator::interpolate()` 入力 |
| `VariableTypeHint` | `compile.rs` | 型別補間ディスパッチ |
| `EasingFunction` / `EasingName` | `easing.rs` | イージングマッピング |
| `ParametricEasing` | `easing.rs` | quad_bez / cub_bez 計算 |
| `TransitionValue` | `transition.rs` | from_value / to_value の値取得 |
| `DynamicValue` | `value.rs` | Object型の値ラップ |
| `DolaError` | `error.rs` | `RuntimeError::CompileError` ラップ元 |

### Architecture Pattern & Boundary Map

```mermaid
graph TB
    subgraph CoreTypes["runtime/ feature = runtime"]
        IS[instance_state.rs<br/>InstanceState]
        TY[types.rs<br/>EvaluatedValue, RuntimeError, StartResult]
        IP[interpolator.rs<br/>Interpolator]
    end

    subgraph ExistingDola["既存 dola 層"]
        ease[easing.rs<br/>EasingName, ParametricEasing]
        comp[compile.rs<br/>CompiledSegment, VariableTypeHint]
        trans[transition.rs<br/>TransitionValue]
        val[value.rs<br/>DynamicValue]
        sb[storyboard.rs<br/>InterruptionPolicy]
        err[error.rs<br/>DolaError]
    end

    subgraph ExternalCrate["interpolation 0.3.0"]
        ef[EaseFunction enum]
        et[Ease trait]
        qb[quad_bez fn]
        cb[cub_bez fn]
        lp[lerp fn]
    end

    IS --> sb
    TY --> val
    TY --> err
    IP --> ease
    IP --> comp
    IP --> trans
    IP --> val
    IP --> TY
    IP --> ef
    IP --> et
    IP --> qb
    IP --> cb
    IP --> lp
```

**Architecture Integration**:
- **選定パターン**: 型定義 + ステートレス関数群。状態を持たない純粋なロジック層
- **境界**: `pub` re-export（`mod.rs` 経由）。facade 子仕様がさらなる可視性境界を決定する
- **レイヤー分離**: InstanceState（ドメイン層）は RuntimeError（API層）に依存しない。try_transition は `Result<InstanceState, InstanceState>` を返し、InstanceManager が必要に応じて変換する
- **既存パターン保持**: 既存 dola の `Serialize/Deserialize` パターンには従わない（ランタイム内部型はシリアライズ不要）
- **Steering 準拠**: Rust 2024 Edition、`unsafe` なし、パニック禁止

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| Runtime | Rust 2024 Edition | 型定義 + 補間ロジック | 既存 dola クレート内 |
| Interpolation | `interpolation` 0.3.0 | イージング評価 + lerp | feature `runtime` で有効化 |

---

## System Flows

### InstanceState 状態遷移図

```mermaid
stateDiagram-v2
    [*] --> Created: Start
    Created --> Playing: コンパイル完了
    Playing --> Paused: Pause
    Paused --> Playing: Resume
    Playing --> Concluded: 自然終了 / Conclude
    Playing --> Cancelled: Cancel
    Playing --> Trimmed: Trim 競合
    Playing --> Compressed: Compress 競合
    Paused --> Concluded: Conclude
    Paused --> Cancelled: Cancel
    Paused --> Trimmed: Trim 競合
    Paused --> Compressed: Compress 競合
    Concluded --> [*]
    Cancelled --> [*]
    Trimmed --> [*]
    Compressed --> [*]
```

**遷移ルール（`try_transition` の判定表）**:

| From \ To | Created | Playing | Paused | Concluded | Cancelled | Trimmed | Compressed |
|-----------|---------|---------|--------|-----------|-----------|---------|------------|
| Created | - | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Playing | ✗ | - | ✓ | ✓ | ✓ | ✓ | ✓ |
| Paused | ✗ | ✓ | - | ✓ | ✓ | ✓ | ✓ |
| Concluded | ✗ | ✗ | ✗ | - | ✗ | ✗ | ✗ |
| Cancelled | ✗ | ✗ | ✗ | ✗ | - | ✗ | ✗ |
| Trimmed | ✗ | ✗ | ✗ | ✗ | ✗ | - | ✗ |
| Compressed | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | - |

**Key Decisions**: 不正遷移は `Err(self)` を返す（RuntimeError ではない）。終了状態からの全遷移は拒否。

### Interpolator 補間フロー

```mermaid
flowchart TD
    Start[interpolate 呼び出し] --> Clamp[progress_t を 0.0..=1.0 にクランプ]
    Clamp --> TypeCheck{VariableTypeHint?}

    TypeCheck -->|Object| ObjCheck{progress_t >= 1.0?}
    ObjCheck -->|Yes| ObjTo[to_value を Object として返却]
    ObjCheck -->|No| ObjFrom[from_value を Object として返却]

    TypeCheck -->|Float / Integer| EasingCheck{easing 指定?}
    EasingCheck -->|None| LinearT[eased_t = progress_t]
    EasingCheck -->|Named name| NamedEase[EasingName から EaseFunction マッピング]
    EasingCheck -->|Parametric QB| QuadBez[quad_bez で eased_t 計算]
    EasingCheck -->|Parametric CB| CubBez[cub_bez で eased_t 計算]

    NamedEase --> LinCheck{Linear?}
    LinCheck -->|Yes| LinearT
    LinCheck -->|No| CalcEase[eased_t = f64 calc ease_fn progress_t]

    LinearT --> Lerp[value = lerp from to eased_t]
    CalcEase --> Lerp
    QuadBez --> Lerp
    CubBez --> Lerp

    Lerp --> RetType{VariableTypeHint?}
    RetType -->|Float| RetFloat[EvaluatedValue Float value]
    RetType -->|Integer| RetInt[EvaluatedValue Integer round as i64]
```

---

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1-1.4 | InstanceState 7バリアント + ユーティリティ | InstanceState | `is_terminal()`, `from_policy()` | 状態遷移図 |
| 2.1-2.6 | 状態遷移ルール | InstanceState | `try_transition()` | 状態遷移図 |
| 3.1-3.4 | EvaluatedValue 値型 | EvaluatedValue | `Display` | — |
| 4.1-4.4 | RuntimeError エラー型 | RuntimeError | `Display`, `Error`, `From<Vec<DolaError>>` | — |
| 5.1-5.3 | StartResult 返却型 | StartResult | — | — |
| 6.1-6.9 | Interpolator 補間計算 | Interpolator | `interpolate()` | 補間フロー |
| 7.1-7.3 | EasingName マッピング | Interpolator | 内部マッピング | — |

---

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies | Contracts |
|-----------|-------------|--------|-------------|-----------------|-----------|
| InstanceState | Core Types | 実行インスタンスの状態enum + 遷移ロジック | 1, 2 | InterruptionPolicy (P0) | State |
| EvaluatedValue | Core Types | 補間結果の型安全な値型 | 3 | DynamicValue (P0) | — |
| RuntimeError | Core Types | 全子仕様共通のエラー型 | 4 | DolaError (P0) | — |
| StartResult | Core Types | Start 返却値構造体 | 5 | — | — |
| Interpolator | Core | イージング適用 + 補間計算 | 6, 7 | interpolation crate (P0), CompiledSegment (P0) | Service |

### Core Types

#### InstanceState

| Field | Detail |
|-------|--------|
| Intent | ストーリーボード実行インスタンスの状態管理と遷移検証 |
| Requirements | 1, 2 |

**Responsibilities & Constraints**
- 7バリアント enum の定義と derive マクロ（Debug, Clone, Copy, PartialEq, Eq）
- 状態遷移の正当性検証（`try_transition`）— `Result<InstanceState, InstanceState>` を返しドメイン層で完結
- `InterruptionPolicy` から対応終了状態への変換（`from_policy`）— `Option<Self>` で Never を安全に処理
- 終了状態判定（`is_terminal`）
- シリアライズなし（ランタイム内部専用）
- **パニック禁止**: 全パスが `Result` / `Option` 返却

**Dependencies**
- Inbound: InstanceManager (Tier 2) — 状態管理 (P0)
- Inbound: ConflictResolver (Tier 3) — 終了状態変換 (P0)
- Outbound: InterruptionPolicy — `from_policy()` 変換元 (P0)

**Contracts**: State [x]

##### State Management

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    Created,
    Playing,
    Paused,
    Concluded,
    Cancelled,
    Trimmed,
    Compressed,
}

impl InstanceState {
    /// 終了状態かどうかを判定
    pub fn is_terminal(&self) -> bool;

    /// 状態遷移を試行する。
    /// 遷移成功時は Ok(target)、不正遷移時は Err(self) を返す。
    /// ドメイン層の責務として RuntimeError に依存しない。
    pub fn try_transition(&self, target: InstanceState)
        -> Result<InstanceState, InstanceState>;

    /// InterruptionPolicy に対応する終了状態を返す。
    /// Never は終了状態に直接対応しないため None を返す。
    pub fn from_policy(policy: InterruptionPolicy) -> Option<Self>;
}
```

- **Preconditions**: なし
- **Postconditions**: `try_transition` — Ok は target と同値、Err は self と同値
- **Invariants**: 終了状態（`is_terminal() == true`）からの遷移は常に `Err`

#### EvaluatedValue

| Field | Detail |
|-------|--------|
| Intent | 補間計算の出力値を型安全に表現する共通値型 |
| Requirements | 3 |

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum EvaluatedValue {
    Float(f64),
    Integer(i64),
    Object(DynamicValue),
}

impl std::fmt::Display for EvaluatedValue {
    // Float → "{v:.6}", Integer → "{v}", Object → "{v:?}"
}
```

**Implementation Notes**
- `VariableTypeHint` の各バリアントと1対1対応: Float↔Float, Integer↔Integer, Object↔Object
- `Display` はデバッグ・ログ用途。Float は小数6桁固定

#### RuntimeError

| Field | Detail |
|-------|--------|
| Intent | 全子仕様が共通利用するエラー型 |
| Requirements | 4 |

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// 存在しないストーリーボード名
    StoryboardNotFound(String),
    /// 存在しない group_id（終了済みインスタンスへの操作を含む）
    InvalidGroupId(u64),
    /// duration=0 + loop_count
    ZeroDurationWithLoop { storyboard: String },
    /// コンパイルエラー（複数エラー対応）
    CompileError(Vec<DolaError>),
}

impl std::fmt::Display for RuntimeError { /* バリアント別メッセージ */ }
impl std::error::Error for RuntimeError {}
impl From<Vec<DolaError>> for RuntimeError {
    fn from(errors: Vec<DolaError>) -> Self {
        Self::CompileError(errors)
    }
}
```

**Key Decisions**:
- **5バリアント構成**: `TerminatedInstance`（終了インスタンスは即削除→InvalidGroupId で統一）と `InvalidStateTransition`（try_transition がドメイン層で完結）を排除
- **`CompileError(Vec<DolaError>)`**: `compile_storyboard()` が `Vec<DolaError>` を返す既存APIに合わせて複数エラー保持
- **`From<Vec<DolaError>>`**: `?` 演算子による自動変換を提供

#### StartResult

| Field | Detail |
|-------|--------|
| Intent | Start コマンドの返却値 |
| Requirements | 5 |

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct StartResult {
    pub group_id: u64,
    pub end_time: f64,
}
```

### Core Layer

#### Interpolator

| Field | Detail |
|-------|--------|
| Intent | イージング関数の適用と値の補間計算 |
| Requirements | 6, 7 |

**Responsibilities & Constraints**
- `EasingName` の30バリアント → `interpolation::EaseFunction` への1対1マッピング
- `EasingName::Linear` は `EaseFunction` を使わず `t` をそのまま返す
- `ParametricEasing::QuadraticBezier` → `interpolation::quad_bez()`
- `ParametricEasing::CubicBezier` → `interpolation::cub_bez()`
- `VariableTypeHint` による型別ディスパッチ: Float→f64直接, Integer→f64補間→round→i64, Object→即時切替
- `progress_t` は 0.0..=1.0 にクランプ
- 補間計算には `interpolation::lerp()` を使用

**Dependencies**
- Inbound: TimelineManager (Tier 2) — 補間要求 (P0)
- External: `interpolation` 0.3.0 — `Ease`, `EaseFunction`, `quad_bez`, `cub_bez`, `lerp` (P0)

**Contracts**: Service [x]

##### Service Interface

```rust
/// イージング適用 + 補間計算（ステートレス ZST）
pub struct Interpolator;

impl Interpolator {
    /// セグメントの進捗率 t で補間値を計算
    pub fn interpolate(
        segment: &CompiledSegment,
        variable_type: &VariableTypeHint,
        progress_t: f64,
    ) -> EvaluatedValue;
}

// 内部は自由関数で実装（Interpolator のメソッドではない）
fn apply_easing(t: f64, easing: &Option<EasingFunction>) -> f64;
fn apply_named_easing(t: f64, name: EasingName) -> f64;
fn apply_parametric_easing(t: f64, param: &ParametricEasing) -> f64;
fn scalar_value(value: &TransitionValue) -> f64;
fn transition_value_to_dynamic(value: &TransitionValue) -> DynamicValue;
```

- **Preconditions**: `progress_t` は任意の f64（内部でクランプ）。`segment` は有効な `CompiledSegment`
- **Postconditions**: `VariableTypeHint::Integer` → `EvaluatedValue::Integer`, `Float` → `Float`, `Object` → `Object`
- **Invariants**: `EasingName` 30バリアント（+Linear）と `EaseFunction` 30バリアントの1対1対応。マッピング漏れはコンパイルエラーで検出（`match` 網羅性）

---

## Data Models

### EasingName → EaseFunction マッピング表

| EasingName (dola) | EaseFunction (interpolation) |
|-------------------|------------------------------|
| Linear | — (t をそのまま返す) |
| QuadraticIn | QuadraticIn |
| QuadraticOut | QuadraticOut |
| QuadraticInOut | QuadraticInOut |
| CubicIn | CubicIn |
| CubicOut | CubicOut |
| CubicInOut | CubicInOut |
| QuarticIn | QuarticIn |
| QuarticOut | QuarticOut |
| QuarticInOut | QuarticInOut |
| QuinticIn | QuinticIn |
| QuinticOut | QuinticOut |
| QuinticInOut | QuinticInOut |
| SineIn | SineIn |
| SineOut | SineOut |
| SineInOut | SineInOut |
| CircularIn | CircularIn |
| CircularOut | CircularOut |
| CircularInOut | CircularInOut |
| ExponentialIn | ExponentialIn |
| ExponentialOut | ExponentialOut |
| ExponentialInOut | ExponentialInOut |
| ElasticIn | ElasticIn |
| ElasticOut | ElasticOut |
| ElasticInOut | ElasticInOut |
| BackIn | BackIn |
| BackOut | BackOut |
| BackInOut | BackInOut |
| BounceIn | BounceIn |
| BounceOut | BounceOut |
| BounceInOut | BounceInOut |

### VariableTypeHint → EvaluatedValue 対応表

| VariableTypeHint | 補間方式 | 出力 EvaluatedValue |
|-----------------|---------|-------------------|
| Float | `lerp(from, to, eased_t)` | `Float(f64)` |
| Integer { typewriter } | `lerp(from, to, eased_t).round() as i64` | `Integer(i64)` |
| Object | `progress_t >= 1.0 ? to : from` | `Object(DynamicValue)` |

---

## Error Handling

### Error Strategy

- **型安全**: `RuntimeError` enum 5バリアントでエラーを網羅的に定義。呼び出し側は `match` でパターンマッチ
- **パニック禁止**: 全パスが `Result` / `Option` 返却。`from_policy(Never)` は `None` を返す
- **レイヤー分離**: `try_transition` は `Result<InstanceState, InstanceState>` を返し、InstanceManager が RuntimeError に変換する責務を持つ
- **`From<Vec<DolaError>>`**: `?` 演算子による自動変換で ergonomic なエラーハンドリング

### Error Categories

| エラー | 原因 | 対応 |
|--------|------|------|
| `StoryboardNotFound` | 未定義名での Start | 呼び出し側で名前を確認 |
| `InvalidGroupId` | 存在しない/終了済み group_id への操作 | 呼び出し側で group_id の有効性を確認 |
| `ZeroDurationWithLoop` | duration=0 + loop | 定義を修正 |
| `CompileError` | バリデーション/コンパイル失敗 | Vec<DolaError> の詳細を確認 |

---

## Testing Strategy

### Unit Tests

**InstanceState 遷移テスト** (1, 2):
- 全7×7=49の遷移パターン（対角を除く42パターン）を網羅的に検証
- `is_terminal()` の7状態（4 terminal + 3 non-terminal）
- `from_policy()` の5パターン（4 Some + 1 None for Never）

**EvaluatedValue テスト** (3):
- 3バリアントの構築と `Display` 出力フォーマット（Float小数6桁、Integer整数、Object Debug）
- `PartialEq` の等価比較

**RuntimeError テスト** (4):
- 全5バリアントの `Display` 出力
- `From<Vec<DolaError>>` 変換
- `std::error::Error` trait 準拠

**StartResult テスト** (5):
- 構築と `PartialEq` 比較

**Interpolator テスト** (6, 7):
- **マッピングテスト**: 30バリアント全ての `EasingName` → `EaseFunction` マッピング正確性
- **境界値テスト**: `progress_t = 0.0`（from_value）、`progress_t = 1.0`（to_value）、クランプ（-0.5 → 0.0, 1.5 → 1.0）
- **Float 補間**: `from=0.0, to=100.0, t=0.5, Linear` → `50.0`
- **Integer 丸め**: `from=0.0, to=10.0, t=0.25` → `round(2.5) = 3`
- **Object 即時切替**: `t=0.5` → from_value、`t=1.0` → to_value
- **ParametricEasing**: QuadraticBezier / CubicBezier の値範囲検証

---

## Supporting References

### Cargo.toml 変更計画

```toml
# 既に追加済み
[features]
runtime = ["dep:interpolation"]

[dependencies]
interpolation = { version = "0.3.0", optional = true }
```

### モジュール構成

```
crates/dola/src/
├── lib.rs              # #[cfg(feature = "runtime")] mod runtime;
├── runtime/
│   ├── mod.rs          # pub use re-export
│   ├── instance_state.rs  # InstanceState enum + impl
│   ├── types.rs           # EvaluatedValue, RuntimeError, StartResult
│   └── interpolator.rs    # Interpolator struct + 自由関数群
```

### `interpolation` クレート API メモ

```rust
// Ease trait (impl for f64)
trait Ease {
    fn calc(self, function: EaseFunction) -> Self;
}

// EaseFunction enum — 30バリアント
enum EaseFunction {
    QuadraticIn, QuadraticOut, QuadraticInOut,
    CubicIn, CubicOut, CubicInOut,
    // ... (全30バリアント)
}

// パラメトリック補間
fn quad_bez<T>(x0: &T, x1: &T, x2: &T, t: &T) -> T;
fn cub_bez<T>(x0: &T, x1: &T, x2: &T, x3: &T, t: &T) -> T;

// 線形補間
fn lerp<T>(x0: &T, x1: &T, t: &T) -> T;
```
