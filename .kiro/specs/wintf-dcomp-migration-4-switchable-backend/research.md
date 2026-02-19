# ギャップ分析: wintf-dcomp-migration-4-switchable-backend

## 概要

本分析は、Phase 4「切り替え式バックエンド実装」の要件と既存コードベースとのギャップを調査し、実装戦略を評価する。Phase 1〜3 で DComp → ULW 移行が完了し、現在は ULW パイプラインのみがアクティブ。DComp コードは全て残存するがスケジュール未登録の状態。

### 調査の前提

| Phase   | 仕様名                                     | 状態 | 本分析への影響                                                                                   |
| ------- | ------------------------------------------ | ---- | ------------------------------------------------------------------------------------------------ |
| Phase 1 | `wintf-dcomp-migration-1-d2d1-composition` | 完了 | ULW合成スタック（compositor, compositor_systems）構築済み                                        |
| Phase 2 | `wintf-dcomp-migration-2-pipeline-switch`  | 完了 | DCompシステム群をスケジュールから除去済み。`on_visual_add` からDCompコンポーネント自動挿入を除去 |
| Phase 3 | `wintf-dcomp-migration-3-ulw-integration`  | 完了 | ULW描画＋クリックスルー動作確認済み。`WS_EX_LAYERED` デフォルト化                                |

---

## 1. 現状調査

### 1.1 要件-資産マッピング

| 要件                             | 既存資産                                                                          | ギャップ                                                       |
| -------------------------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Req 1: CompositionMode enum      | **Missing** — コードベースに概念なし                                              | enum定義、コンポーネント化、デフォルト値設定が必要             |
| Req 2: ECSクエリ分岐             | **Constraint** — bevy_ecs 0.18 には enum variant ベースのクエリフィルタなし       | マーカーコンポーネント or ランタイムフィルタの設計判断が必要   |
| Req 3: DCompスケジュール再登録   | 全DCompシステム関数がコードとして残存（`systems.rs`, `visual_manager.rs`）        | world.rs のスケジュール登録を復元 + クエリにモードフィルタ追加 |
| Req 4: GraphicsCore条件付きDComp | `GraphicsCoreInner` にDCompフィールドが非Option直接格納                           | Inner構造体の分割 or DCompフィールドのOption化                 |
| Req 5: ウィンドウスタイル連動    | `WindowStyle::default()` が `WS_EX_LAYERED` 固定                                  | `create_windows` でCompositionMode参照→スタイル自動決定        |
| Req 6: パフォーマンス最適化      | ULWシステムはDComp非依存。DCompシステムはWindow検索不要                           | 空クエリスキップの検証、DComp遅延初期化の実装                  |
| Req 7: DComp動作検証             | `dcomp_demo.rs` が存在（ECS非使用の独立デモ）                                     | ECSベースのDComp描画テスト/exampleが必要                       |
| Req 8: 共存動作                  | 両パイプラインのコードが独立して存在                                              | 混在時の共有リソース競合・デバイスロスト独立復旧の検証         |
| Req 9: WinRT拡張準備             | パイプラインコードがモジュール分離済み（`compositor_systems.rs` vs `systems.rs`） | `CompositionMode` の拡張設計、モジュール境界の明確化           |
| Req 10: テスト戦略               | 既存テスト + example が ULW 前提                                                  | DComp example、混在 example の新規作成                         |

### 1.2 関連モジュール構成

| モジュール                       | パス                                 | 現在の役割                              | 本仕様での変更                                 |
| -------------------------------- | ------------------------------------ | --------------------------------------- | ---------------------------------------------- |
| `graphics/core.rs`               | `ecs/graphics/core.rs`               | GraphicsCore（D3D/D2D/DComp一括初期化） | DComp遅延初期化への分離                        |
| `graphics/components.rs`         | `ecs/graphics/components.rs`         | DComp/ULW両コンポーネント定義           | CompositionMode追加、on_visual_addのモード分岐 |
| `graphics/systems.rs`            | `ecs/graphics/systems.rs`            | DCompシステム群（未登録）               | クエリにモードフィルタ追加、スケジュール再登録 |
| `graphics/visual_manager.rs`     | `ecs/graphics/visual_manager.rs`     | DComp Visual管理                        | クエリにモードフィルタ追加                     |
| `graphics/compositor.rs`         | `ecs/graphics/compositor.rs`         | WindowD3D11Compositor（ULW）            | 変更なし                                       |
| `graphics/compositor_systems.rs` | `ecs/graphics/compositor_systems.rs` | ULWシステム群（アクティブ）             | クエリにモードフィルタ追加                     |
| `graphics/mod.rs`                | `ecs/graphics/mod.rs`                | モジュール定義                          | 必要に応じてモジュール整理                     |
| `ecs/window.rs`                  | `ecs/window.rs`                      | ウィンドウ管理、create_windows          | CompositionMode参照→スタイル自動決定           |
| `ecs/world.rs`                   | `ecs/world.rs`                       | スケジュール定義                        | DCompシステム再登録                            |
| `ecs/window_proc/handlers.rs`    | `ecs/window_proc/handlers.rs`        | WndProcハンドラ                         | WM_PAINT/WM_ERASEBKGNDのモード分岐             |
| `com/dcomp.rs`                   | `com/dcomp.rs`                       | DComp APIラッパー                       | 変更なし                                       |
| `com/ulw.rs`                     | `com/ulw.rs`                         | ULW APIラッパー                         | 変更なし                                       |

### 1.3 既存パターンと規約

#### bevy_ecs クエリフィルタリングパターン

コードベースで使われている主なパターン:

```rust
// 存在フィルタ（最も一般的）
With<SurfaceGraphics>, Without<WindowHandle>, Without<ChildOf>

// 変更検出
Changed<HasGraphicsResources>, Changed<GlobalArrangement>

// 複合条件
Or<(Without<WindowD3D11Compositor>, Changed<HasGraphicsResources>, Changed<WindowPos>)>

// has マクロ（ブール判定）
Has<Window>  // visual_property_sync_system で使用
```

**重要**: enum variant ベースのフィルタは存在しない。`With<T>` はコンポーネントの**存在**のみチェック。

#### GPU リソースの Option<Inner> パターン

```rust
struct XxxInner { /* COM objects */ }

#[derive(Component)]
pub struct Xxx {
    inner: Option<XxxInner>,
    generation: u32,
}
// invalidate() で inner = None, generation++
```

`GraphicsCore`, `WindowGraphics`, `WindowD3D11Compositor` 全てがこのパターンに従う。

#### DComp システムのWindow非依存性

DComp システムは**Window エンティティを探索しない**:
- `render_surface`: 自エンティティの `SurfaceGraphics` のみ参照
- `visual_resource_management_system`: `Res<GraphicsCore>` のみ参照
- `visual_hierarchy_sync_system`: 直接の `ChildOf.parent()` のみ参照、Window まで遡らない

→ DComp モードフィルタをウィンドウレベルではなく**エンティティレベル**で適用する場合、子エンティティにもモード情報が必要になる可能性がある。

#### ULW システムのWindow中心性

ULW システムは**Window エンティティを起点**として子ツリーを走査:
- `composite_render_system`: `Query<(Entity, &mut WindowD3D11Compositor, &Children, ...)>` でWindowを起点にサブツリー再帰描画

→ ULW は `WindowD3D11Compositor` の存在がモードフィルタとして機能している（`Without<WindowD3D11Compositor>` が暗黙のDCompフィルタ）。

---

## 2. 主要技術課題

### 2.1 CompositionMode のフィルタリング戦略（Research Needed）

bevy_ecs 0.18 では enum variant によるクエリフィルタが不可能。以下の3アプローチを検討:

#### アプローチ A: マーカーコンポーネント方式

```rust
#[derive(Component)] pub struct UlwMode;
#[derive(Component)] pub struct DCompMode;
```

- ULW システム: `With<UlwMode>`, DComp システム: `With<DCompMode>`
- **課題**: 子エンティティへのモード伝播が必要（`on_visual_add` 等）
- **利点**: ECSネイティブのクエリフィルタで最高効率、空クエリ時は即スキップ

#### アプローチ B: enum コンポーネント + ランタイムフィルタ方式

```rust
#[derive(Component)]
pub enum CompositionMode { ULW, DComp }
```

- システム内で `iter().filter(|(.., mode)| matches!(mode, CompositionMode::DComp))`
- **課題**: 全エンティティをイテレートしてからフィルタ（パフォーマンス懸念）
- **利点**: シンプルな定義、WinRT拡張が容易

#### アプローチ C: ハイブリッド方式（enum + マーカー併用）

```rust
#[derive(Component)]
#[non_exhaustive]
pub enum CompositionMode { ULW, DComp }

// 内部的にマーカーを自動管理
#[derive(Component)] struct UlwPipeline;
#[derive(Component)] struct DCompPipeline;
```

- `CompositionMode` 追加時に対応マーカーを自動挿入（Observerパターン）
- **課題**: 管理の二重化
- **利点**: 公開APIの拡張性 + 内部フィルタリング効率

### 2.2 モード情報の伝播スコープ

#### 問題

- DComp システムの多くは `Window` エンティティではなく**子エンティティ**（ウィジェット）を直接クエリする
  - `visual_resource_management_system`: `Query<(Entity, &Visual, &mut VisualGraphics, ...)>`
  - `render_surface`: `Query<(Entity, &SurfaceGraphics, ...)>`
  - `deferred_surface_creation_system`: `Query<(Entity, &VisualGraphics, &GraphicsCommandList, ...)>`
- ULW システムは Window を起点にサブツリー走査するので、Window にのみモードがあれば十分

#### 選択肢

| 選択肢                                                | 説明                                                                                                            | トレードオフ                                                                                                                           |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| (a) Windowのみにモード                                | DCompシステムは暗黙的フィルタ: `With<VisualGraphics>` / `With<SurfaceGraphics>` の存在がDCompモードの証         | DCompコンポーネントの挿入管理が重要。既に Phase 2 で `on_visual_add` から除去されているため、DCompモードWindow生成時に明示的挿入が必要 |
| (b) 全エンティティにモード伝播                        | 親→子に `CompositionMode` を伝播                                                                                | オーバーヘッドが大きい。Common Infrastructure の伝播システム活用は可能だが、全エンティティにコンポーネント追加はメモリコスト           |
| (c) DCompコンポーネントの存在自体をフィルタとして利用 | DComp固有コンポーネント(`VisualGraphics`, `SurfaceGraphics`)が挿入されているエンティティのみDCompシステムが処理 | 追加のモードコンポーネント不要。ただし`on_visual_add`のフック復元が必要で、CompositionModeをどこで判定するかが課題                     |

### 2.3 GraphicsCore の DComp 遅延初期化

#### 現状

```rust
struct GraphicsCoreInner {
    d3d: ID3D11Device,          // 共通
    dxgi: IDXGIDevice4,         // 共通
    d2d_factory: ID2D1Factory,  // 共通
    d2d: ID2D1Device,           // 共通
    d2d_device_context: ID2D1DeviceContext,  // 共通
    dwrite_factory: IDWriteFactory2,         // 共通
    desktop: IDCompositionDesktopDevice,     // DComp専用
    dcomp: IDCompositionDevice3,             // DComp専用
}
```

DComp初期化ステップ（7, 8）は最後に位置し、D2Dデバイスへの単方向依存のみ → **分離容易**。

#### 選択肢

| 選択肢                                   | 説明                                                                                 | トレードオフ                                                                                                      |
| ---------------------------------------- | ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| (a) Inner内のOptionフィールド            | `desktop: Option<IDCompositionDesktopDevice>`, `dcomp: Option<IDCompositionDevice3>` | 最小変更。ただし `GraphicsCore::new()` 呼び出し時にDComp不要なら初期化スキップするフラグが必要                    |
| (b) Inner分割（共通Inner + DComp Inner） | `common: CommonInner`, `dcomp: Option<DCompInner>`                                   | 構造が明確。遅延初期化で `dcomp` を後付け可能。`invalidate()` は `common = None; dcomp = None;`                   |
| (c) 別Resourceとして分離                 | `GraphicsCore`（共通）+ `DCompGraphicsCore`（DComp専用Resource）                     | bevy_ecs的に自然（リソース分離）。DCompシステムは `Option<Res<DCompGraphicsCore>>` で参照。存在しなければスキップ |

### 2.4 on_visual_add フックの DComp コンポーネント復元

#### 現状

Phase 2 で `on_visual_add` から以下を除去:
- `VisualGraphics` 自動挿入
- `SurfaceGraphics` 自動挿入
- `SurfaceGraphicsDirty` 自動挿入

#### 課題

DComp モードの Window 配下に Visual を追加する場合、これらのコンポーネントが必要。しかし `on_add` フック内では**親のCompositionModeを参照できない**（DeferredWorld でのクエリ制約）。

#### 選択肢

| 選択肢                                         | 説明                                                                                                | トレードオフ                                                                             |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| (a) on_visual_add にモード判定を復元           | DeferredWorld を使って parent の CompositionMode を参照し、DCompモードなら DComp コンポーネント挿入 | `DeferredWorld` の制約調査が必要（Research Needed）                                      |
| (b) 専用システムで後付け挿入                   | DCompモードWindow配下で `Without<VisualGraphics>` なVisualエンティティを検索→挿入                   | 1フレーム遅延するが確実。既存の `visual_resource_management_system` がこのパターンに近い |
| (c) Window生成時にサブツリー全体にまとめて挿入 | DCompモードWindow spawn時に子エンティティにもDCompコンポーネントをバッチ挿入                        | spawn順序への依存が強い。動的な子追加に対応できない                                      |

### 2.5 ウィンドウメッセージハンドラのモード分岐

#### 現状

```rust
// WM_ERASEBKGND — ULW固定前提
pub(super) fn WM_ERASEBKGND(...) -> HandlerResult {
    Some(LRESULT(1)) // 背景消去スキップ
}

// WM_PAINT — ULW固定前提
pub(super) fn WM_PAINT(hwnd: HWND, ...) -> HandlerResult {
    // BeginPaint/EndPaint最小ペア
    Some(LRESULT(0))
}
```

#### 課題

WndProcハンドラは `hwnd` ベースで、ECS World への直接アクセスは限定的。Entity→CompositionMode の逆引きが必要。

#### 現行パターン

`handlers.rs` は `AppState` から `World` にアクセスし、`hwnd_to_entity` マップで Entity を検索するパターンが確立されている（ポインタイベント処理で使用）。同様のパターンで CompositionMode を取得可能。

---

## 3. 実装アプローチ評価

### Option A: マーカーコンポーネント方式（最小変更・最高効率）

**戦略**: DComp 固有コンポーネントの存在がモードフィルタとして機能する設計を活用。`CompositionMode` はWindowエンティティにのみ持たせ、DCompコンポーネントの挿入/未挿入でシステムが自然にフィルタリングされる。

**変更箇所**:
1. `CompositionMode` enum 定義（Window専用コンポーネント）
2. `GraphicsCoreInner` の DComp フィールドを `Option` 化 or 分離
3. `on_visual_add`: DCompモードWindow配下でのみDCompコンポーネント挿入を復元
4. `create_windows`: CompositionMode参照→スタイル自動決定
5. DCompシステムをworld.rsに再登録（クエリは既存の`With<VisualGraphics>`等がフィルタとして機能）
6. ULWシステムは`WindowD3D11Compositor`の存在がフィルタ（変更なし）
7. `handlers.rs`: モード分岐追加

**トレードオフ**:
- ✅ DComp システムのクエリ変更が最小（`With<VisualGraphics>` / `With<SurfaceGraphics>` が暗黙フィルタ）
- ✅ ULW システムは変更なし（`WindowD3D11Compositor` の有無がフィルタ）
- ✅ ECS ネイティブフィルタで空クエリ時の即スキップが保証される
- ❌ DComp コンポーネント挿入タイミングの管理が必要（`on_visual_add` or 専用システム）
- ❌ WinRT Compositor 追加時に新しいコンポーネントセットとon_addロジックが必要

### Option B: 明示的モード伝播方式（明確・拡張容易）

**戦略**: `CompositionMode` を全エンティティに伝播し、各システムがモードを明示的にチェック。

**変更箇所**:
1. `CompositionMode` enum 定義（`#[non_exhaustive]`）
2. Common Infrastructure の伝播システムに `CompositionMode` 伝播を追加
3. 全パイプライン固有システムに `CompositionMode` フィルタ追加
4. `GraphicsCore` 分割
5. `create_windows`, `handlers.rs` 変更

**トレードオフ**:
- ✅ モードの明示性が最高（どのエンティティがどのパイプラインか一目瞭然）
- ✅ WinRT Compositor 追加時に enum variant 追加のみ
- ❌ 全エンティティへの伝播オーバーヘッド（メモリ + 伝播システムの実行コスト）
- ❌ 既存システムの大量変更（DComp: 8システム、ULW: 3システムにフィルタ追加）
- ❌ ランタイムフィルタ（enum variant マッチ）でクエリ効率が低下

### Option C: ハイブリッド方式（推奨候補）

**戦略**: `CompositionMode` はWindowエンティティにのみ持たせる。DCompモードWindow配下では**DComp固有コンポーネント（`VisualGraphics`, `SurfaceGraphics`）の挿入を復元**し、ULWモードWindow配下ではULW固有コンポーネント（`WindowD3D11Compositor`）の挿入を維持。各パイプラインのシステムは既存のコンポーネント存在フィルタで自然に分岐。

具体的には:
- DComp システム: 既存のクエリ（`With<VisualGraphics>` 等）がそのまま機能
- ULW システム: 既存のクエリ（`With<WindowD3D11Compositor>`）がそのまま機能
- `compositor_init_system`: DCompモードWindowにはスキップ → `Without<WindowD3D11Compositor>` + `CompositionMode` チェック or DCompモードWindowには `WindowD3D11Compositor` を挿入しない
- `init_window_graphics`: ULWモードWindowにはスキップ → 同様

**変更箇所**:
1. `CompositionMode` enum 定義（Windowエンティティ専用）
2. `GraphicsCore` の DComp 部分を分離（遅延初期化）
3. `on_visual_add`: `CompositionMode` に応じた条件付きDCompコンポーネント挿入を復元（もしくは専用システムで後付け）
4. `compositor_init_system`: `CompositionMode::ULW` のWindowのみ `WindowD3D11Compositor` を生成
5. `init_window_graphics`: `CompositionMode::DComp` のWindowのみ `WindowGraphics` を生成
6. `create_windows`: `CompositionMode` → スタイル自動決定
7. `world.rs`: DCompシステム再登録
8. `handlers.rs`: モード分岐

**トレードオフ**:
- ✅ 既存の DComp/ULW システムクエリの変更が最小限
- ✅ コンポーネント存在フィルタは ECS ネイティブで最高効率
- ✅ 空クエリ時の即スキップが保証される（パフォーマンス要件 Req 6 充足）
- ✅ DComp 遅延初期化で ULW のみ使用時のコスト排除（Req 6.4 充足）
- ❌ `on_visual_add` 内でのモード判定が技術的に challenge（DeferredWorld制約）
- ❌ WinRT Compositor 追加時に新しいコンポーネントセットと初期化ロジックが必要

---

## 4. 努力レベルとリスク評価

### 努力レベル: **M（3-7日）**

**根拠**:
- DComp システムのコードは全て残存しており、新規実装ではなく**再有効化+フィルタ追加**
- GraphicsCore の分割は構造が明確（DComp 初期化ステップが末尾に集中）
- `CompositionMode` enum 自体はシンプルな定義
- 主要な変更点: `world.rs`（スケジュール再登録）、`core.rs`（遅延初期化）、`components.rs`（CompositionMode定義 + on_visual_add）、`handlers.rs`（モード分岐）、`window.rs`（スタイル連動）

### リスク: **Medium**

| リスク要因                                | レベル | 対策                                                                                                        |
| ----------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------- |
| `on_visual_add` 内の CompositionMode 参照 | Medium | DeferredWorld でのクエリ制約を調査。不可の場合は専用システムで後付け（1フレーム遅延は許容範囲）             |
| DComp パイプライン再起動後の描画品質      | Medium | `dcomp_demo.rs` をリファレンスに段階的検証。Phase 2 除去前のコミットと diff 比較                            |
| 共有リソース（D2D DeviceContext）の競合   | Low    | ULW と DComp は同一フレーム内で異なるステージで実行。DeviceContext は排他アクセス。スケジュール順序保証あり |
| デバイスロスト時の復旧                    | Low    | `GraphicsCore.invalidate()` で一括破棄・再初期化。D3Dデバイスロストは全パイプラインに波及するため、独立復旧は不要（Req 8.5 削除済み） |
| WndProc ハンドラのモード分岐              | Low    | `hwnd_to_entity` マップ + Entity→CompositionMode 取得のパターンが確立済み                                   |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C（ハイブリッド方式）

既存のコンポーネント存在フィルタリングを最大限活用し、システムクエリの変更を最小化する方式を推奨。パフォーマンス要件（Req 6）の充足と実装コストのバランスが最も良い。

### 設計フェーズでの決定事項

1. **CompositionMode の定義方式**: `#[non_exhaustive]` enum vs sealed enum + extension trait
2. **DComp コンポーネント挿入タイミング**: `on_visual_add` フック内 vs 専用後付けシステム
3. **GraphicsCore 分割粒度**: Inner 内 Option vs Inner 分割 vs 別 Resource

### Research Needed（設計フェーズで調査）

- [ ] `DeferredWorld` で祖先エンティティの `CompositionMode` コンポーネントを参照可能か（bevy_ecs 0.18 の on_add フック制約）
- [ ] bevy_ecs 0.18 の Observer パターンによるコンポーネント連動挿入の実現性
- [ ] DComp と ULW が同一フレーム内で同一 `ID2D1DeviceContext` を使用する際の排他制御要件
- [ ] `IDCompositionDesktopDevice::CreateTargetForHwnd` が `WS_EX_LAYERED` ウィンドウに対して動作するか（万が一の混同防止）
