# Research & Design Decisions — dola-runtime-engine

## Summary
- **Feature**: `dola-runtime-engine`
- **Discovery Scope**: Extension（既存 dola クレート基盤上にランタイム層を構築）
- **Key Findings**:
  - `interpolation` (0.3.0) の API は dola の `EasingName` 31バリアントと完全マッピング可能。`Linear` のみ自明実装（`t` をそのまま返す）
  - 時刻ユーティリティは `GetTickCount64`（ms精度）が最もシンプルだが、アニメーション用途では QPC ベースの高精度実装も選択肢。dola クレート内に feature gate `windows-clock` で配置する方針
  - 既存 `PlaybackState` enum は要件の状態遷移モデルと乖離。新 `InstanceState` enum をランタイム専用に定義し、既存 `PlaybackState` はデータモデル層として温存

---

## Research Log

### interpolation クレート API マッピング

- **Context**: Req 10 イージング評価の実装基盤として `interpolation` (0.3.0) を採用確定済み。設計フェーズで具体的なマッピング仕様を確定する必要がある
- **Sources Consulted**: https://docs.rs/interpolation/0.3.0/interpolation/
- **Findings**:
  - `EaseFunction` enum: 30バリアント（QuadraticIn〜BounceInOut）
  - `Ease` trait: `fn calc(t: f64, function: EaseFunction) -> f64` — `impl Ease for f64`
  - `lerp(a, b, t)`: 線形補間関数
  - `quad_bez(x0, x1, x2, t)`: 2次ベジェ補間
  - `cub_bez(x0, x1, x2, x3, t)`: 3次ベジェ補間
  - dola `EasingName::Linear` → `interpolation` に対応なし → `t` をそのまま返す自明実装
  - dola `EasingName::QuadraticIn` → `EaseFunction::QuadraticIn` — 以下同様に1対1マッピング
  - `ParametricEasing::CubicBezier { x0, x1, x2, x3 }` → `cub_bez(x0, x1, x2, x3, t)`
  - `ParametricEasing::QuadraticBezier { x0, x1, x2 }` → `quad_bez(x0, x1, x2, t)`
- **Implications**: マッピング関数は `match` による直接変換。追加のラッパーは不要

### CSS cubic-bezier パラメータマッピング

- **Context**: CSS `cubic-bezier(x1, y1, x2, y2)` と `cub_bez(x0, x1, x2, x3, t)` のパラメータ形式が異なる
- **Sources Consulted**: CSS Transitions Level 1 仕様、interpolation クレートソースコード
- **Findings**:
  - CSS `cubic-bezier(x1, y1, x2, y2)` は制御点2つ（始点(0,0)と終点(1,1)は暗黙）
  - `cub_bez(x0, x1, x2, x3, t)` は1次元4点ベジェ。CSS のY軸カーブ再現には `cub_bez(0.0, y1, y2, 1.0, t)` と対応
  - X軸の非線形マッピング（CSS cubic-bezier の特徴）は別途 t 解決が必要 → これは dola の `ParametricEasing::CubicBezier` がすでに4点形式で定義しているため、CSS 互換は dola ドキュメント定義側の責務
- **Implications**: ランタイムは `ParametricEasing` の値をそのまま `cub_bez` / `quad_bez` に渡すだけ。CSS パラメータ変換は対象外

### 時刻ユーティリティ選定

- **Context**: Req 11 で「OS起動時からのf64秒数」を要求。dola クレートのプラットフォーム非依存性との兼ね合い
- **Sources Consulted**: Win32 API ドキュメント、quanta クレート (0.12.6)、std::time::Instant
- **Findings**:

  | 方式 | 精度 | OS起動時起点 | プラットフォーム | 依存 |
  |------|------|-------------|----------------|------|
  | `GetTickCount64` | ms | ✅ | Windows | windows クレート |
  | QPC/QPF | μs〜ns | ❌（差分のみ） | Windows | windows クレート |
  | `quanta::Clock` | ns | ❌（Instant相当） | クロスプラット | quanta クレート |
  | `std::time::Instant` | ns | ❌（差分のみ） | クロスプラット | なし |
  | `IUIAnimationTimer::GetTime()` | μs | ✅ | Windows (COM) | windows クレート + COM |

  - `GetTickCount64` が最もシンプル: `GetTickCount64() as f64 / 1000.0` で要件を満たす。ms精度はアニメーション（60fps ≈ 16.67ms）に十分
  - QPC ベースで起動時起点を実現するには `GetTickCount64` をベースラインとしてQPC差分を加算する方式も可能だが、複雑さに対する利点が薄い
  - `quanta` は高性能だが OS 起動時起点を直接提供しない。Mock 機能はテスト用途に有用だが、追加依存コスト（TSC キャリブレーション等）が大きい
  - `IUIAnimationTimer::GetTime()` は完璧だが COM 依存を dola に持ち込むのは不適切
- **Implications**: `GetTickCount64` を主方式として採用。feature gate `windows-clock` で隔離。高精度が必要になった場合は QPC ベースに差し替え可能な抽象化を設計

### PlaybackState の整理方針

- **Context**: 既存 `PlaybackState`（Idle/Playing/Paused/Completed/Cancelled）と要件の状態遷移モデル（Created/Playing/Paused/Concluded/Cancelled/Trimmed/Compressed）の乖離
- **Sources Consulted**: 既存 `crates/dola/src/playback.rs`、Req 8 要件定義
- **Findings**:
  - 既存 `PlaybackState` はデータモデル層（シリアライズ対応）の値であり、ランタイムの状態マシンとは異なる役割
  - 要件の終了状態4種（Concluded/Cancelled/Trimmed/Compressed）は `InterruptionPolicy` の5戦略に1対1対応（Never は終了状態を持たない — 中断拒否のため）
  - 既存 `PlaybackState::Idle` → 要件の `Created` に相当
  - 既存 `PlaybackState::Completed` → 要件の `Concluded` に相当
- **Implications**: ランタイム専用の `InstanceState` enum を新設。既存 `PlaybackState` は変更せず温存。将来的に `PlaybackState` を `InstanceState` に統合する場合は migration パスを用意

### 所有権モデル

- **Context**: 購読者（Subscriber）のライフサイクルと DolaRuntime の所有権関係
- **Sources Consulted**: Req 4（購読管理）、Req 5（Update差分配信）
- **Findings**:
  - シングルスレッド前提（wintf の ECS schedule から呼び出される想定）
  - `DolaRuntime` が全状態を所有。購読者は軽量ハンドル（`SubscriptionHandle`）を保持
  - `SubscriptionHandle` の `Drop` で自動 Unsubscribe（Req 4.4）
  - `SubscriptionHandle` は `DolaRuntime` への参照を保持 → ライフタイム制約 or ID ベースの間接参照
  - ID ベース（`subscriber_id: u64`）が最もシンプル。`Drop` 時に runtime に Unsubscribe メッセージを送信する方式はライフタイム問題を回避
- **Implications**: `SubscriptionHandle` は `subscriber_id` + `Weak<RefCell<DolaRuntimeInner>>` または callback 登録方式。設計は ID ベースの間接参照を採用し、`Drop` trait で自動解除を実現

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: dola 内全実装 | 全ランタイムコードを dola クレート内に配置 | 既存型への直接アクセス、単一クレート完結 | 責務拡大、プラットフォーム依存混入 | gap-analysis Option A |
| B: 別クレート | `dola-runtime` 新設、dola に依存 | 責務分離明確、dola のプラットフォーム非依存性維持 | クレート間 API 境界設計、型共有の調整 | gap-analysis Option B |
| **C: ハイブリッド** | **コアロジックは dola 内、時刻のみ feature gate** | **既存型と密結合テスト可能、時刻のみ分離** | **feature gate 設計が必要** | **gap-analysis 推奨、採用** |

**選定**: Option C（ハイブリッド）を採用。理由:
1. `CompiledStoryboard` / `InterruptionPolicy` 等の既存型との密結合が自然
2. 子仕様フェーズ分割において、同一クレート内のモジュール分割は子仕様間の型共有を容易にする
3. `interpolation` 依存は feature gate `runtime` で隔離可能（ランタイム不要なユーザーは有効化しない）
4. 時刻取得のみ `windows-clock` feature で分離すれば、プラットフォーム非依存性の実質的な犠牲はない

---

## Design Decisions

### Decision: ランタイムモジュール配置

- **Context**: dola クレート内のどこにランタイムコードを配置するか
- **Alternatives Considered**:
  1. `src/` 直下にフラットに配置（`runtime.rs`, `timetable.rs` 等）
  2. `src/runtime/` サブモジュールに集約
- **Selected Approach**: `src/runtime/` サブモジュール
- **Rationale**: 既存のデータモデル層（`document.rs`, `storyboard.rs` 等）とランタイム層を明確に分離。モジュールツリーで責務境界を視覚化
- **Trade-offs**: ファイル階層が1段深くなるが、可読性と保守性が向上
- **Follow-up**: `lib.rs` の `pub use` で主要型をクレートルートに re-export

### Decision: InstanceState 新設（PlaybackState 温存）

- **Context**: 既存 `PlaybackState` と要件の状態モデルが不一致
- **Alternatives Considered**:
  1. 既存 `PlaybackState` を破壊的変更で拡張
  2. 新 `InstanceState` をランタイム専用に定義、既存を温存
- **Selected Approach**: Option 2 — `InstanceState` を新設
- **Rationale**: 既存 `PlaybackState` はシリアライズ対応のデータモデル型。ランタイムの状態マシンは内部詳細であり、シリアライズ不要。破壊的変更は既存テストに影響
- **Trade-offs**: 2つの状態 enum が共存するが、役割が明確に異なるため混乱は限定的
- **Follow-up**: `InstanceState` はシリアライズを実装しない（ランタイム内部状態のため）

### Decision: 時間オフセット統一機構

- **Context**: Pause/Resume とループ再生の両方で時間調整が必要
- **Alternatives Considered**:
  1. Pause 用とループ用で別々のオフセット管理
  2. 統一した `pause_accumulated: f64` + ループ計算
- **Selected Approach**: Option 2 — 統一機構
- **Rationale**: Req 12.6 で「Pause/Resume と同じ仕組みを使用」と明示。有効時刻の計算式を統一:
  `effective_base_time = (current_time - instance.start_time - instance.pause_accumulated) * instance.time_scale`
  ループ時は周回完了で `pause_accumulated` にループ分を加算調整
- **Trade-offs**: ループオフセットと実際の pause 累積が混在するが、計算式が単一で保守性が高い
- **Follow-up**: 実装時にオーバーフローや精度問題のテストが必要

### Decision: 子仕様フェーズ分割計画

- **Context**: 実装ゴールとして子仕様方式が確定済み。設計フェーズで分割戦略を決定
- **Alternatives Considered**:
  1. 要件単位で1対1分割（12子仕様）
  2. アーキテクチャ層単位で分割（3-4子仕様）
  3. 機能クラスタ単位で分割（4-5子仕様）
- **Selected Approach**: Option 3 — 機能クラスタ単位（4子仕様）
- **Rationale**: 要件単位は粒度が細かすぎ管理コスト大。層単位は依存関係が直線的すぎて並行作業不可。機能クラスタ単位が最適バランス
- **Trade-offs**: 一部クラスタ（特に facade）が多数の要件をカバーし複雑になる
- **Follow-up**: 各子仕様の scope と tier を設計文書で明確化

---

## Risks & Mitigations

- **time_scale の解釈** ✅ **Resolved**: WAM `SetStoryboardPlaybackSpeed` と同じ乗算方式で確定。`time_scale=2.0` は2倍速（半分の時間で完了）。式: `effective_time = (current_time - start_time - pause_accumulated) * time_scale`
- **InstanceState 外部可視性** ✅ **Resolved**: ステートレス設計を採用。`InstanceState` は内部のみ。オーケストレーターは `end_time` で終了管理、購読者は `update()` の空 Vec で検知。`group_id` は同一ストーリーボードの複数実行インスタンスを個別制御するための一意識別子として使用
- **f64 精度**: 長時間再生（数時間以上）での f64 精度劣化。→ 差分基準（前回値との比較）で影響を緩和。実用上デスクトップマスコットでは問題にならない
- **循環参照**: `SubscriptionHandle` が `DolaRuntime` への参照を保持し循環参照の可能性。→ ID ベース間接参照で回避
- **Never ポリシーの延期キュー**: `InterruptionPolicy::Never` で延期されたストーリーボードの管理が複雑。→ 子仕様3（競合解決）で個別に設計。✅ **Resolved**: design.md に実装ノート追加済み

---

## References

- [interpolation 0.3.0 ドキュメント](https://docs.rs/interpolation/0.3.0/interpolation/) — イージング評価基盤
- [quanta 0.12.6 ドキュメント](https://docs.rs/quanta/latest/quanta/) — 高性能時刻取得（不採用、参考）
- [Win32 GetTickCount64](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64) — OS起動時からのミリ秒取得
- [Win32 QueryPerformanceCounter](https://learn.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter) — 高精度タイマー
- [CSS Transitions Level 1](https://www.w3.org/TR/css-transitions-1/) — cubic-bezier 仕様参考
- gap-analysis.md — 既存コードベース調査結果
