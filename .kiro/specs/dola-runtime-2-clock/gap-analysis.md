# ギャップ分析 — dola-runtime-2-clock

## 概要

本文書は `dola-runtime-2-clock` の要件と既存コードベースのギャップを分析し、実装戦略の判断材料を提供する。

**結論**: 本仕様は完全新規コンポーネントであり、既存コードとの衝突リスクはほぼ皆無。既存パターンに沿った低リスクな実装。

---

## 1. 現状調査

### 1.1 既存アセットスキャン

| アセット | パス | 関連性 |
|---------|------|--------|
| runtime モジュール | `crates/dola/src/runtime/mod.rs` | **直接**: clock モジュールの親。`#[cfg(feature = "runtime")]` で条件コンパイル済み |
| runtime/instance_state.rs | `crates/dola/src/runtime/instance_state.rs` | **参考**: 同一 runtime サブモジュール内の実装パターン |
| runtime/interpolator.rs | `crates/dola/src/runtime/interpolator.rs` | **参考**: 同一 runtime サブモジュール内の実装パターン |
| runtime/types.rs | `crates/dola/src/runtime/types.rs` | **参考**: 型定義パターン |
| FrameTime::get_precise_time() | `crates/wintf/src/ecs/graphics/core.rs:176-181` | **参考**: Win32 SystemInformation API の unsafe 呼び出しパターン |
| IUIAnimationTimer::get_time() | `crates/wintf/src/com/animation.rs:16-24` | **参考**: OS 起動時起点の f64 秒取得（COM 経由、dola では不採用） |
| dola Cargo.toml | `crates/dola/Cargo.toml` | **直接**: feature gate と依存定義の追加対象 |
| ワークスペース Cargo.toml | `Cargo.toml:53-78` | **参考**: `windows` クレートの workspace 定義（`Win32_System_SystemInformation` 既に含む） |

### 1.2 runtime/mod.rs の現在構造

```rust
mod instance_state;
mod interpolator;
mod types;

pub use instance_state::InstanceState;
pub use interpolator::Interpolator;
pub use types::{EvaluatedValue, RuntimeError, StartResult};
```

**観察**: `clock` モジュールの追加は `#[cfg(feature = "windows-clock")] pub mod clock;` を1行追加するのみ。既存の re-export パターンに直接影響しない。

### 1.3 既存の Windows API unsafe パターン

`wintf/src/ecs/graphics/core.rs` に `GetSystemTimePreciseAsFileTime` の使用例が存在:

```rust
fn get_precise_time() -> u64 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::SystemInformation::GetSystemTimePreciseAsFileTime;
    let ft: FILETIME = unsafe { GetSystemTimePreciseAsFileTime() };
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}
```

**観察**: `use` 文を関数内に局所化し、`unsafe` ブロックは最小限。`GetTickCount64` も同じパターンで実装可能。

### 1.4 Feature Gate パターン

`crates/dola/src/lib.rs` で `runtime` feature の条件コンパイルパターンが確立済み:

```rust
#[cfg(feature = "runtime")]
pub mod runtime;
```

**観察**: `windows-clock` も同じ `#[cfg()]` パターンで追加可能。ただし `clock` は `runtime` サブモジュール内なので、`runtime/mod.rs` 側に条件を記述する。

### 1.5 テストパターン

`tests/runtime_core_types_test.rs` が既存のテストパターンを示す:

```rust
#![cfg(feature = "runtime")]
use dola::runtime::{EvaluatedValue, InstanceState, RuntimeError, StartResult};
```

**観察**: clock テストも `#![cfg(feature = "windows-clock")]` で同様に記述可能。

---

## 2. 要件フィージビリティ分析

### 2.1 要件-アセットマップ

| 要件 | 既存アセット | ギャップ |
|------|------------|---------|
| Req 1: `now() -> f64` | なし | **Missing**: 新規関数の作成が必要 |
| Req 2: `GetTickCount64` 使用 | `Win32_System_SystemInformation` がワークスペース Cargo.toml に既存 | **Missing (軽微)**: dola の Cargo.toml への `windows` 依存追加のみ |
| Req 3: `windows-clock` feature gate | `runtime` feature のパターンが確立済み | **Missing (軽微)**: 新規 feature 定義を追加 |
| Req 4: Cargo.toml 依存定義 | `windows = "0.62.2"` がワークスペースレベルで定義済み | **Missing (軽微)**: dola 固有の feature 指定が必要 |
| Req 5: モジュール公開 | `runtime/mod.rs` のモジュール構造が確立済み | **Missing (軽微)**: 1行の条件付き `pub mod` 追加 |
| Req 6: テスト可能性 | `runtime_core_types_test.rs` のパターンが確立済み | **Missing**: テストファイルの新規作成 |

### 2.2 制約と懸念

| 制約 | 影響度 | 備考 |
|------|--------|------|
| `GetTickCount64` は `unsafe` | 低 | windows crate docs で確認済み: `pub unsafe fn GetTickCount64() -> u64`。1行の unsafe ブロックで完結 |
| dola の `windows` 依存追加 | 低 | optional dependency + feature gate で隔離。`runtime` feature のみの場合は `windows` クレートに依存しない |
| ワークスペース vs ローカル feature 指定 | 低 | dola は `windows` を workspace 依存として参照しつつ、独自の features subset を指定する必要がある |
| `GetTickCount64` の精度（ms） | なし | 60fps = 16.67ms に対して十分。要件 1.4 で明示的に合意済み |
| `GetTickCount64` のオーバーフロー | なし | u64 で最大 585 百万年。f64 への変換でも精度は秒単位で十分 |

### 2.3 複雑度シグナル

**分類**: 単純なユーティリティ関数（CRUD 未満の最小実装）

- ビジネスロジック: なし
- 外部統合: Win32 API 1関数のみ
- 状態管理: なし（ステートレス関数）
- エラーハンドリング: なし（`GetTickCount64` は常に成功）
- データモデル: なし（f64 のみ）

---

## 3. 実装アプローチ評価

### Option A: 新規コンポーネント作成（**推奨**）

統合指針 Section 5.3 に従い、`crates/dola/src/runtime/clock.rs` を新規作成する。

**変更対象ファイル**:

| ファイル | 変更内容 |
|---------|---------|
| `crates/dola/Cargo.toml` | `windows-clock` feature + `windows` optional dependency 追加 |
| `crates/dola/src/runtime/mod.rs` | `#[cfg(feature = "windows-clock")] pub mod clock;` を1行追加 |
| `crates/dola/src/runtime/clock.rs` | **新規作成**: `now()` 関数 + テスト |

**トレードオフ**:
- ✅ 統合指針のモジュール構成に完全準拠
- ✅ 既存コードへの変更は最小限（Cargo.toml + mod.rs の2行）
- ✅ feature gate で完全に隔離、他の機能に影響なし
- ✅ 実装は10-20行程度の最小コンポーネント

**他のオプションは不要**: 本仕様は既存コンポーネントの拡張対象がなく、ハイブリッドアプローチも不要な単純な新規追加。

---

## 4. ワークスペース `windows` 依存との整合性

### 4.1 現在のワークスペース定義

```toml
[workspace.dependencies.windows]
version = "0.62.2"
features = [
    "Win32_System_SystemInformation",  # ← GetTickCount64 を含む（既存）
    # ... 他の features ...
]
```

### 4.2 dola Cargo.toml への追加方針

**方式1**: ワークスペース参照 + 独自 features

```toml
[dependencies.windows]
workspace = true
optional = true
```

この方式では、ワークスペースの全 features を継承する。dola は `Win32_System_SystemInformation` のみ必要だが、optional dependency なので feature gate 無効時はコンパイルされない。

**方式2**: dola 独自のバージョン + 最小 features

```toml
[dependencies.windows]
version = "0.62"
optional = true
features = ["Win32_System_SystemInformation"]
```

この方式では、dola が独立して必要最小限の features のみを指定する。

**評価**:
- 方式1: ワークスペース統一性を保つ（`Cargo.lock` のバージョン一致保証）。ただし dola に不要な features が含まれる（コンパイル時のみの影響、バイナリには含まれない）
- 方式2: dola の独立性を保つ（publish 時に最小依存）。ワークスペースとの version 不一致リスクあり

**Research Needed**: `workspace = true` と `optional = true` の組み合わせが Cargo で正しく動作するかの確認（Cargo リファレンス上は対応しているが、features の上書き挙動に注意）

---

## 5. 実装複雑度とリスク

| 項目 | 評価 | 根拠 |
|------|------|------|
| **工数** | **S（1日以内）** | 新規ファイル1つ + 既存ファイル2行変更 + テスト3件。実装コードは10行未満 |
| **リスク** | **Low** | 確立済みパターンの踏襲、アーキテクチャ変更なし、最小統合面、既存機能への影響なし |

---

## 6. 設計フェーズへの推奨事項

### 6.1 推奨アプローチ

**Option A（新規コンポーネント作成）を推奨**。唯一の合理的な選択肢。

### 6.2 設計フェーズで確定すべき事項

1. **Cargo.toml の依存指定方式**: ワークスペース参照（方式1）vs 独自定義（方式2）の決定
2. **テスト配置**: `clock.rs` 内の `#[cfg(test)]` vs `tests/` ディレクトリの統合テストファイル
3. **doc comment スタイル**: `/// OS 起動時からの現在時刻（f64秒）を取得` のフォーマット確認

### 6.3 Research Items

| 項目 | 優先度 | 状態 | 備考 |
|------|--------|------|------|
| `workspace = true` + `optional = true` の Cargo 挙動 | P0 | **解決済み** | Cargo 公式ドキュメントで明示的にサポート。メンバークレート側で `optional = true` を指定可能。features はワークスペース定義を継承し、追加は可能だが削除・上書きは不可。publish 時は通常の optional dependency として扱われる。方式1 vs 方式2 の選択は設計判断として残す |

**注**: 全 Research Items が解決済み。`GetTickCount64` の API 仕様と動作は十分に枯れており、追加調査は不要。
