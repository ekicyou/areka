---
inclusion: always
updated_at: 2026-07-01
---

# Project Structure

## Organization Philosophy

**責務ごとのクレート分割** - wintfがWindows向けUI基盤、dolaが演出データモデル、arekaがアプリ統合を担う。脳（SHIORI）との内部唯一ABIは独立した最小依存クレート shiori-abi（`IShiori`/`IShioriHost` のカスタムCOM定義＋エルゴノミック変換層）として分離し、UI基盤（wintf）に依存させない（下流32bitホスト/pastaが同ABIを共有するため）。wintf内ではCOMラッパー、ECSサブシステム、Win32メッセージ境界を分け、dolaは定義層から実行時層までを段階的なモジュールで構成する。

## Directory Patterns

### Workspace Root
**Location**: `/`  
**Purpose**: Cargoワークスペース設定、横断ドキュメント、開発ルール  
**Example**: `Cargo.toml`, `README.md`, `doc/`, `.kiro/steering/`

### Library Crate
**Location**: `/crates/wintf/`  
**Purpose**: メインライブラリの実装  
**Structure**:
- `src/` - ライブラリソースコード
- `examples/` - 手動検証用サンプルアプリケーション
- `tests/` - ドメイン別に整理された統合テスト

### COM Wrapper Layer
**Location**: `/crates/wintf/src/com/`  
**Purpose**: Windows COMインターフェイスのRustラッパー  
**Contains**:
- `dcomp.rs` - DirectComposition API
- `d3d11.rs` - Direct3D11 API
- `dwrite.rs` - DirectWrite API（縦書き対応）
- `dxgi.rs` - DXGIインターフェイス
- `wic.rs` - Windows Imaging Component
- `ulw.rs` - UpdateLayeredWindow API
- `animation.rs` - Windows Animation API
- `d2d/` - Direct2D関連

> **移行中の注意（2026-07-01・方針確定・実装移行中）**: `dcomp.rs`（DirectComposition）は **Windows.UI.Composition（`Compositor`/`DesktopWindowTarget`）へ移行決定**、`ulw.rs`（ULW）は**除去予定**（別プロセス透過は `WS_EX_TRANSPARENT` 動的トグル方式へ）。`ecs/graphics/` の `compositor.rs`（`WindowD3D11Compositor`）/`compositor_systems/` は ULW 専用ゆえ ULW 除去で撤去対象。briefed specs: `wintf-dcomp-to-wuc-migration`／`wintf-ulw-removal`／`wintf-clickthrough-alpha-toggle`。正本は `roadmap.md`／`doc/COMPAT_ARCHITECTURE.md`。

### ECS Component Layer
**Location**: `/crates/wintf/src/ecs/`  
**Purpose**: ECSアーキテクチャのコンポーネント定義  
**Structure**:
- `common/` - 共通インフラ（階層伝播システム）
- `window/` - ウィンドウ管理とWin32状態同期
- `graphics/` - グラフィックスリソースと描画システム
- `layout/` - taffy統合、配置計算、ヒット判定
- `widget/` - UIウィジェット、テキスト、画像、ブラシ
- `pointer/` - ポインター入力のバッファリングと配信
- `drag/` - ドラッグ状態管理とディスパッチ
- `cue/` - CueQueueとCueSheet配送のECS統合
- `dola/` - DolaRuntimeのECS Component化
- `world/` - schedule labels、vsync、フレーム進行
- `app.rs` - アプリケーション状態管理（ウィンドウカウント、ディスプレイ構成変更）
- `window_proc/` - Win32メッセージ種別ごとのECSブリッジ
- `types.rs` - 共通幾何プリミティブ型（`Point`/`Size`/`Rect` 等）の唯一の定義箇所。`#[repr(C)]` でWin32/D2D1型とゼロコスト相互変換

#### ECS機能グループ詳細

**1. Common Infrastructure** (`common/`)
- 責務: ECS階層システムの汎用的な伝播ロジック
- 代表的な関数: `sync_simple_transforms<L,G,M>()`, `propagate_parent_transforms<L,G,M>()`
- 特徴: 完全ジェネリック化、`Arrangement`/`Transform`両対応

**2. Window Management** (`window/`, `window_proc/`)
- 責務: Win32ウィンドウのライフサイクル管理とECS統合
- 代表的なコンポーネント: `Window`, `WindowHandle`, `WindowPos`, `WindowStyle`, `ZOrder`
- 特徴: HWNDとEntityの双方向マッピング、マルチスレッド対応

**3. Graphics Resources** (`graphics/`)
- 責務: Direct2D/DirectCompositionリソースのライフサイクル管理
- 代表的なコンポーネント: `GraphicsCore`, `WindowGraphics`, `Visual`, `Surface`, `DeviceContext`
- サブモジュール: `compositor.rs`（`WindowD3D11Compositor` — レンダリングパイプライン）, `compositor_systems/`（コンポジットinit/render）, `visual_manager.rs`（Visualの挿入・管理API）, `command_list.rs`（D2Dコマンドリスト）
- 特徴: デバイスロスト対応、遅延初期化、階層的描画

**4. Layout System** (`layout/`)
- 責務: taffyレイアウトエンジン統合と配置計算
- サブモジュール: `taffy.rs`, `metrics.rs`, `arrangement.rs`, `rect.rs`, `systems/`, `hit_test/`, `hit_region/`
- 代表的なコンポーネント: `TaffyStyle`, `TaffyComputedLayout`, `Arrangement`, `GlobalArrangement`, `Size`, `Offset`
- 特徴: 軸平行変換最適化、Common Infrastructure活用、Surface生成最適化

**5. Input Systems** (`pointer/`, `drag/`)
- 責務: ポインターイベントの収集、ヒットテスト、ドラッグ状態遷移
- 代表的な型: ポインターバッファー、ドラッグコンテキスト、キャプチャガード
- 特徴: Win32メッセージ境界とECSイベント処理を分離

**6. Cue System** (`cue/`)
- 責務: 離散コマンド配信のECS統合レイヤー
- 代表的なコンポーネント: `CueQueue`（`dola::TimedSchedule<CueCommand>` を内包）, `CueSheetTracker`
- 型の再エクスポート: `dola::cue::*`（`CueCommand`, `BarrierKind`, `RoutingCommand`, ドメイン型）を `pub use` で提供
- 主要システム: `dispatch_pending_cue_sheets`
- 特徴: `Entity::to_bits()` / `from_bits()` 変換をECS境界で実行、`EntityRef(u64)` のラウンドトリップ

**7. Dola Animator** (`dola/`)
- 責務: `DolaRuntime` のエンティティごとのECS Component化
- 代表的なコンポーネント: `DolaAnimator`（`DolaRuntime` を内部所有、`unsafe impl Send + Sync`）
- 主要システム: `tick_dola_animators`（`Query<&mut DolaAnimator>` + `Res<FrameTime>` で全エンティティ一括tick）
- 安全性保証: `Query<&mut>` の排他アクセスにより1 tick 1回・単一スレッドでの更新を型レベルで保証
- 消費パターン: 後続システムが `Query<&DolaAnimator>` の `last_result()` で `UpdateResult` を読み取る

**8. World Scheduling** (`world/`)
- 責務: schedule label、vsync、フレーム時間管理
- 特徴: グラフィックス更新、入力処理、アニメーション処理順序を明示する

**9. Application State** (`app.rs`)
- 責務: アプリケーション全体の状態管理
- 代表的な構造体: `App`（ウィンドウカウント、ディスプレイ構成変更検出、メッセージウィンドウ管理）
- 特徴: ECS Resourceとしてワールドに導入、ウィンドウの作成・破棄を追跡

### UI スレッド基盤 / Message Handling
**Location**: `/crates/wintf/src/runtime/`（新 facade）＋ `/crates/wintf/src/`（ルート補助）  
**Purpose**: UI スレッド基盤（メッセージループ・ウィンドウ生成・UI スレッド async・60Hz tick・終了規律）  
**Contains（runtime/ = 新 facade `WinApp`・spec `wintf-winmsg-executor` で外部クレ化完了）**:
- `runtime/mod.rs` - 公開 facade `WinApp`（`new`/`world`/`run`/`spawn_ui_local`）。COM/DPI 初期化・World 所有・全結線
- `runtime/message_loop.rs` - `MessageLoopDriver`（`block_on`/`MessageLoop::run` 委譲）＋ `ShutdownPolicy`（`event_listener::Event` 終了規律）
- `runtime/tick_bridge.rs` - `VsyncEventBridge`（DwmFlush→event_listener notify）＋ `AsyncTickTask`（13 schedule tick）
- `runtime/wndproc_bridge.rs` - `WndState`/`make_wndproc`（ライブラリ `Window::new_ex` クロージャ→`dispatch_window_message` 配送・GWLP 不使用）
- `runtime/window_registry.rs` - `WindowRegistry`（NonSend・`Window<S>` 所有・reconcile で寿命/終了管理）
- `runtime/window_factory.rs` - `EcsWindowFactory`（`util::Window::new_ex` 生成・style/pos/title 反映）

**ルート補助モジュール**:
- `win_state.rs` - ウィンドウ状態管理
- `win_style.rs` - ウィンドウスタイル定義
- `api.rs` - Windows API safeラッパー

> **注意**: 旧 deprecated モジュール（`winproc.rs` / `win_message_handler.rs` / `win_thread_mgr.rs` / `process_singleton.rs`）は spec `wintf-winmsg-executor` の完了に伴い**撤去済み**。メッセージ配送は `ecs/window_proc/` 配下の `dispatch_window_message` ＋ 種別別ハンドラ、UI スレッド基盤は `runtime/` の `WinApp` facade を用いること。クラス登録・HINSTANCE はライブラリ（`wintf-winmsg-executor`）が担う。

## Naming Conventions

- **Files**: `snake_case.rs`（Rust標準）
- **Modules**: `snake_case`
- **Types**: `PascalCase` (structs, enums, traits)
- **Functions**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`

### Test Naming Conventions

#### Integration Tests (`tests/` directory)
- **File name**: `{feature}_{type}_test.rs` or `{feature}_test.rs`
- **Entry point**: `tests/{domain}.rs` — `#[path]` による `mod` 宣言のみ、テストロジックは含まない
- **Common helpers**: `tests/{domain}/common/mod.rs`
- **Domain prefix removal**: ドメインサブディレクトリに配置する際、ドメイン名と重複するプレフィックスを除去する
  - 例: `compile_error_test.rs` → `compile/error_test.rs`
  - 例: `visual_child_order_test.rs` → `visual/child_order_test.rs`
  - ただし `taffy_` のようなサブドメインプレフィックスは維持する

#### Unit Tests (in-source `#[cfg(test)]`)
- **Inline**: 小規模テストはソースファイル内に `mod tests { ... }` として記述
- **Separated**: `{module}/tests.rs` — ディレクトリモジュール化パターン（`bitmap_source/` を参照）

### Component Naming Conventions

COMオブジェクトをラップするECSコンポーネントは、以下の命名規則に従う：

#### GPUリソース (`XxxGraphics`)
- **特性**: Direct3D/Direct2D/DirectCompositionデバイスに依存
- **デバイスロスト対応**: `invalidate()`メソッドと`generation`フィールドを実装
- **命名**: `XxxGraphics`サフィックス
- **例**:
  - `WindowGraphics` - ウィンドウレベルGPU資源
  - `VisualGraphics` - ウィジェットレベルGPU資源
  - `SurfaceGraphics` - ウィジェットレベルGPU資源
  - 将来: `BrushGraphics`, `BitmapGraphics`

#### CPUリソース (`XxxResource`)
- **特性**: デバイス非依存、永続的
- **デバイスロスト対応**: 不要（通常の参照カウント管理のみ）
- **命名**: `XxxResource`サフィックス
- **例**:
  - `TextLayoutResource` - テキストレイアウト（Label、TextBlock等で再利用）
  - 将来: `TextFormatResource`, `PathGeometryResource`

#### レベル分類
- **ウィンドウレベル**: Windowエンティティに配置（例: `WindowGraphics`）
- **ウィジェットレベル**: 個別ウィジェットエンティティに配置（例: `VisualGraphics`, `TextLayoutResource`）
- **共有リソース**: 複数ウィジェットで再利用（例: 将来の`BrushGraphics`、`GeometryResource`）

#### 非COMコンポーネント
- **論理コンポーネント**: サフィックスなし（例: `Label`, `Rectangle`, `Button`）
- **マーカーコンポーネント**: 用途に応じた名前（例: `HasGraphicsResources`, `GraphicsNeedsInit`）

#### COMアクセスメソッド命名
COMリソースコンポーネント内部のアクセスメソッドは、COMインターフェイス型に対応：
- `WindowGraphics::target()` → `Option<&IDCompositionTarget>`
- `VisualGraphics::visual()` → `Option<&IDCompositionVisual3>`
- `SurfaceGraphics::surface()` → `Option<&IDCompositionSurface>`
- `TextLayoutResource::get()` → `Option<&IDWriteTextLayout>`

### Animation Definition Crate
**Location**: `/crates/dola/`  
**Purpose**: 宣言的アニメーション定義フォーマット（Declarative Orchestration for Live Animation）  
**Structure**:
- `src/` - ライブラリソースコード
  - `document.rs` - ルートドキュメント定義
  - `storyboard.rs` - ストーリーボード定義
  - `transition.rs` - トランジション定義
  - `easing.rs` - イージング関数
  - `variable.rs` - アニメーション変数定義
  - `value.rs` - 動的値
  - `builder.rs` - Builder API
  - `playback.rs` - 再生状態
  - `validate/` - バリデーション（`rules.rs` 等）
  - `error.rs` - エラー型
  - `compile/` - 解決・型変換（`resolve.rs`, `types.rs`）
  - `cue/` - CueSheet/TimedScheduleモデル
  - `runtime/` - 実行系。ファサード、インスタンス管理、補間、購読管理、タイムライン、ループ制御をサブディレクトリモジュールに分割（`instance_manager/`, `interpolator/`, `subscription_manager/`, `timeline_manager/`, `loop_controller/` 等）
- `tests/` - テスト

> **モジュール分割パターン**: 600行リファクタ（`oversized-file-refactor`）以降、肥大化したファイルは `{module}/mod.rs` + サブモジュールのディレクトリ形式へ分割する方針。dola `runtime/` がその代表例。

**Dependencies**: `serde` + feature flags (`json`, `toml`, `yaml`) ＋ `interpolation`, `rand`, `pasta_core`

### Application Binary Crate
**Location**: `/crates/areka/`  
**Purpose**: デスクトップマスコット・プラットフォーム本体  
**Status**: 試作実装（シェル+バルーン2ウィンドウ表示、ドラッグ移動、ダブルクリック終了）  
**Dependencies**: wintf, human-panic, thiserror, tracing, tracing-subscriber, async-io, bevy_ecs, windows

### Parser Crate
**Location**: `/crates/areka-parsers/`
**Purpose**: 伺か資産（さくらスクリプト / surfaces.txt / balloon descript / install.txt）の**純粋パーサ群**。UI・COM 非依存（`std` ＋ `tracing` のみ）。
**Pattern**（`areka-P0-sakura-parse` が確立・M1 の `shell`/`balloon`/`package` parser もこれに接ぎ木）:
- モジュール分割: `src/sakura/`（既存）＋今後 `shell/`・`balloon/`・`package/`
- API: `pub fn parse(&str) -> Vec<Model>`（**`Result` 無しの寛容パース**・未知は `Raw` 変種へ吸収）
- 値型: NewType＋opaque inner＋read-only accessor、enum は `#[non_exhaustive]`（拡張シームのみ）
- テスト: in-source `#[cfg(test)]`、emo2 実 fixture で検証・**過剰実装禁止**（emo2 使用分のみ）
**Dependencies**: `tracing` のみ（外部パーサ非依存）

### SHIORI ABI Crate
**Location**: `/crates/shiori-abi/`
**Purpose**: 脳（SHIORI）との**内部唯一 ABI**。`IShiori`/`IShioriHost` のカスタム COM 定義（HSTRING/UTF-16・IID 既定義）＋エルゴノミック変換層。UI 基盤（wintf）に依存させない最小依存クレート（下流 32bit ホスト/pasta が同 ABI を共有）。x64 native 脳は in-proc COM、過去互換は 32bit Rust ホスト（host-32）が IPC 越しに同 ABI を実装。

### Pilot (Two-Tunnel Knowledge) Crate
**Location**: `/crates/pilot/`
**Purpose**: 二坑モデルの**先進坑（pilot・使い捨て）知見クレート**。**空（最小）`lib.rs` ＋ 探索コードは `examples/<spec-name>/` のみ**という構造で葉ノード隔離を担保（出荷グラフから被依存しない＝可逆性の構造的担保）。完了 pilot はここへ隔離保全（例: `examples/pilot-clickthrough-alpha-toggle/`・`examples/shiori-host-32/`）。規律の正本は `.kiro/steering/two-tunnel.md`。

### Vendored: pasta DSL Engine
**Location**: `/vendors/pasta/`（git サブモジュール）  
**Repository**: [https://github.com/ekicyou/pasta](https://github.com/ekicyou/pasta)  
**Purpose**: 里々インスパイアの会話記述DSLスクリプトエンジン  
**Integration**: `[patch.crates-io]` で `pasta_core` をローカルパスへ差し替え、ワークスペース内で協調開発する。dola が直接依存し、areka は wintf/dola 経由で利用する。クローン時は `git submodule update --init` が必要

## Import Organization

```rust
// 標準ライブラリ
use std::sync::Arc;

// 外部クレート（アルファベット順）
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::DirectComposition::*;

// 内部モジュール（相対パス）
use crate::com::dcomp::*;
use crate::ecs::window::*;
```

## Code Organization Principles

- **レイヤー分離**: COM→ECS→Message Handlingの依存方向を厳守
- **COMライフタイム**: `windows-rs`提供のスマートポインターで直接管理
- **unsafe隔離**: `unsafe`ブロックはCOMラッパー層に集約し、安全なAPIを上位層に提供
- **モジュール独立性**: 各モジュールは独立してテスト可能な単位として設計
- **テスト入口の固定化**: `tests/{domain}.rs` は束ね役に留め、実テストはドメイン配下へ寄せる

---
Workspace構成により将来的な機能拡張（別クレート追加）が容易。
