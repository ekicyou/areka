# ギャップ分析: wintf-dcomp-migration-1-d2d1-composition

## 概要

本分析は、Phase 1「D2D1合成スタック構築」の要件と既存コードベースとのギャップを調査し、実装戦略を評価する。

---

## 1. 現状調査

### 1.1 関連モジュール構成

| モジュール | パス | 役割 | 本仕様との関係 |
|-----------|------|------|---------------|
| `graphics/mod.rs` | `ecs/graphics/mod.rs` | モジュール定義・再エクスポート | 新モジュール登録先 |
| `graphics/core.rs` | `ecs/graphics/core.rs` | `GraphicsCore` リソース（D3D11, D2D1, DComp 初期化） | `ID2D1DeviceContext` 取得元 |
| `graphics/components.rs` | `ecs/graphics/components.rs` | `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics`, `Visual` | パターン参考・`Visual.opacity` 確認 |
| `graphics/command_list.rs` | `ecs/graphics/command_list.rs` | `GraphicsCommandList`（`ID2D1CommandList` ラッパー） | 合成描画の入力 |
| `graphics/systems.rs` | `ecs/graphics/systems.rs` | DComp描画・合成システム群（1419行） | `draw_recursive` パターン参考 |
| `layout/arrangement.rs` | `ecs/layout/arrangement.rs` | `Arrangement`, `GlobalArrangement` | 座標変換参照元（変更なし） |
| `layout/metrics.rs` | `ecs/layout/metrics.rs` | `Opacity` コンポーネント | Phase 0 で廃止予定（Visual.opacity に統合） |
| `layout/systems.rs` | `ecs/layout/systems.rs` | `propagate_global_arrangements` | 座標伝播参照（変更なし） |
| `common/tree_system.rs` | `ecs/common/tree_system.rs` | ジェネリック階層伝播アルゴリズム | `Mul<L, Output=G>` trait 制約 |
| `common/tree_iter.rs` | `ecs/common/tree_iter.rs` | `DepthFirstReversePostOrder` イテレータ | 合成走査のパターン参考 |
| `com/d2d/mod.rs` | `com/d2d/mod.rs` | D2D1 拡張 trait 群 | `DrawImage`, `SetTransform` 等の API |
| `com/mod.rs` | `com/mod.rs` | COM モジュール定義 | `ulw.rs` 追加先 |
| `world.rs` | `ecs/world.rs` | スケジュール・システム登録 | Phase 2 で変更（Non-Goal） |

### 1.2 既存パターンと規約

#### GPU リソースコンポーネントの `Option<Inner>` パターン

既存の `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` は全て同一パターン：

```rust
struct XxxInner { /* COM objects */ }

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Xxx {
    inner: Option<XxxInner>,
    generation: u32,  // WindowGraphics のみ
}

impl Xxx {
    pub fn new(...) -> Self { ... }
    pub fn invalidate(&mut self) { self.inner = None; }
    pub fn is_valid(&self) -> bool { self.inner.is_some() }
}

unsafe impl Send for Xxx {}
unsafe impl Sync for Xxx {}
```

**`WindowD3D11Compositor` はこのパターンに完全に従える。**

#### デバイスロスト復旧フロー

```
GraphicsCore::invalidate()
  → init_graphics_core: GraphicsCore 再作成
    → HasGraphicsResources.set_changed() 全エンティティにトリガー
      → init_window_graphics: Changed<HasGraphicsResources> で検出、再初期化
      → visual_resource_management_system: 同上
```

**`WindowD3D11Compositor` は `init_window_graphics` と同じパターンで generation 不一致検出 + 再作成を実装すればよい。**

#### 階層伝播の `Mul` trait 制約

`propagate_parent_transforms<L, G, M>` は以下の制約を要求：

```rust
L: Component + Copy + Into<G>,
G: Component<Mutability = Mutable> + Copy + PartialEq + Mul<L, Output = G>,
M: Component<Mutability = Mutable>,
```

**設計決定: GlobalArrangement には opacity を追加しない。** `Arrangement` に opacity フィールドがなく `Mul` trait での累積が不自然であることも、Opacity を Layout 層から分離する決定を支持する。代わりに `CompositeContext` で `composite_render_system` 内の `render_subtree()` 再帰走査中に手動累積する。

### 1.3 Visual 描画属性の現状

**Visual 構造体の設計意図**: 描画属性（opacity, is_visible, transform_origin）を保持するコンポーネント

| 名前 | 型 | 場所 | 役割 |
|------|------|------|------|
| `Visual.opacity` | `f32` フィールド | `components.rs` | **描画属性（正式）** — 現在未使用だが Phase 1 で初ワイヤリング |
| `Visual.is_visible` | `bool` フィールド | `components.rs` | **描画属性（正式）** — 現在未使用だが Phase 1 で初ワイヤリング |
| `Opacity` | `Component` | `metrics.rs` | **設計ミス** — layout モジュールに配置されているが、opacity は描画属性であり layout 概念ではない |

**処理の伝搬層**:
```
Widget → GraphicsCommandList  （何を描くか）
Layout → Visual               （どこに、どう描くか）
         ↓
    Visual.opacity          （透明度 = 描画属性）
    Visual.is_visible       （描画実行の有無 = 描画属性）
```

**現状の誤った実装**: `visual_property_sync_system` が `Opacity` コンポーネント（metrics.rs）を参照しているが、これは **Visual.opacity を使うべき**。

**Phase 1 での修正方針**:
- `Visual.opacity` を正式な透明度ソースとして使用
- `Visual.is_visible` を正式な可視性制御として使用
- `Opacity` コンポーネントは Phase 2 以降で廃止候補（DComp 依存コード削除時）

---

## 2. 要件ごとのフィージビリティ分析

### Req 1: WindowD3D11Compositor コンポーネント

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: 4リソース管理 | `WindowGraphics` の `Option<Inner>` パターン | `ID2D1Bitmap1` 作成（TARGET/CPU_READ）、`CreateDIBSection`、`CreateCompatibleDC` — 全て新規 | Medium |
| AC2: ライフサイクル API | `WindowGraphics` と同パターン | new/resize/invalidate/is_valid/generation — 全てパターン流用 | Low |
| AC3: 同一サイズ保証 | なし | Inner 構造体でサイズを1か所で管理すれば自然に保証 | Low |
| AC4: SparseSet | `#[component(storage = "SparseSet")]` パターン既存 | 属性追加のみ | Low |
| AC5: ファイル配置 | `graphics/mod.rs` に登録パターン既存 | `mod compositor;` 追加のみ | Low |

**必要な新規 Win32/D2D API 呼び出し（Research Needed）:**

- `ID2D1DeviceContext::CreateBitmap()` with `D2D1_BITMAP_PROPERTIES1` (TARGET option)
- `ID2D1DeviceContext::CreateBitmap()` with `D2D1_BITMAP_PROPERTIES1` (CPU_READ option)
- `CreateDIBSection()` — PBGRA32, top-down
- `CreateCompatibleDC()`
- `SelectObject()` for HBITMAP into MemoryDC

現行コードに `CreateDIBSection`/`CreateCompatibleDC` の使用実績はない（全て DComp 経由）。

### Req 2: composite_render_system

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: 深さ優先走査 | `draw_recursive()` + `DepthFirstReversePostOrder` | `Children` の pre-order 走査は `draw_recursive` にほぼ同じロジックあり | Low |
| AC2: Transform + DrawImage | `render_surface` 内の `DrawImage` パターン | `SetTransform` + `DrawImage` パターンは完全に既存 | Low |
| AC3: is_visible == false スキップ | なし | `Visual.is_visible` は現在デッドフィールド。初ててのワイヤリング | Medium |
| AC4: Opacity 適用 | なし | **`CompositeContext` で手動累積、D2D Effect 等で個別適用（PushLayer不使用）** | Medium |
| AC5: CopyFromBitmap | なし | `ID2D1Bitmap1::CopyFromBitmap()` は新規 | Low |
| AC6: ダーティ判定 | `Changed<SurfaceGraphicsDirty>` パターン | ウィンドウレベルの集約判定が新規ロジック | Medium |
| AC7: 既存システム非侵襲 | `GraphicsCommandList` が DComp 非依存設計 | 入力側は完全に既存のまま。合成システムは消費側のみ | Low |
| AC8: ファイル配置 | — | `compositor_systems.rs` 新規作成 | Low |

**重要: 合成ターゲット切替**

現行の `render_surface` は各エンティティの `IDCompositionSurface` に `BeginDraw`/`EndDraw` で描画する。新しい `composite_render_system` は `WindowD3D11Compositor` の `composition_bitmap` に対して `ID2D1DeviceContext::SetTarget` → `BeginDraw`/`EndDraw`（レンダーターゲット方式）で描画する。

**Research Needed:**
- `CompositeContext` 手動累積方式における D2D Effect または pre-multiplied alpha での opacity 適用方法
- `ID2D1Bitmap1` に `SetTarget` して描画する際のデバイスコンテキスト要件（共有 DC vs 専用 DC）

### Req 3: compositor_init_system

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: Added\<WindowHandle\> | `init_window_graphics` が同パターン | `Without<WindowD3D11Compositor>` + `Changed<HasGraphicsResources>` の Or クエリ | Low |
| AC2: GraphicsCore DC 取得 | `GraphicsCore::device_context()` 既存 | そのまま利用可能 | Low |
| AC3: リサイズ検出 | `WindowHandle` にサイズ情報なし | **`WindowPos` からサイズ取得が必要** — 前フレームサイズの保存方法検討 | Medium |
| AC4: generation 不一致 | `init_window_graphics` が同パターン | パターン流用 | Low |
| AC5: ファイル配置 | — | AC8 と同ファイル | Low |

**リサイズ検出のアプローチオプション:**
- **Option A**: `WindowD3D11Compositor` 内部に `cached_size: (u32, u32)` を保持し、`WindowPos` と比較
- **Option B**: `Changed<WindowPos>` フィルタでサイズ変更を検出
- **推奨**: Option A（compositor 内部にサイズキャッシュ、シンプルで自己完結）

### Req 4: transfer_to_hbitmap

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: Map/Copy/Unmap | なし | `ID2D1Bitmap1::Map()` / `Unmap()` は完全新規 | Medium |
| AC2: pitch≠stride 行単位コピー | なし | 標準的なメモリコピーロジック | Low |
| AC3: 単一 memcpy 最適化 | なし | `std::ptr::copy_nonoverlapping` | Low |
| AC4: com/ulw.rs 配置 | `com/mod.rs` に `pub mod xxx;` パターン | `pub mod ulw;` 追加 | Low |
| AC5: エラーハンドリング | `windows::core::Result` パターン既存 | そのまま利用 | Low |

**Research Needed:**
- `ID2D1Bitmap1::Map()` の正確な API シグネチャ（`D2D1_MAP_OPTIONS_READ`）
- Map 中のスレッドセーフティ制約

### Req 5: Phase 1 検証基準

**注**: 旧 Req 5 (リサイズ対応) と旧 Req 6 (デバイスロスト対応) は Req 3 (compositor_init_system) の通常動作に包含されるため削除。AC は Req 3 に統合済み。

| AC | テスト種別 | 既存パターン | 難度 |
|----|-----------|-------------|------|
| AC1: リソース作成 | unit test | `crates/wintf/tests/` に多数のテスト | Low（ただし GPU 依存テストは CI で困難） |
| AC2: 合成描画 | integration test | 既存 integration test パターンあり | Medium（描画結果の検証方法要検討） |
| AC3: opacity 累積 | unit test | `Arrangement` / `GlobalArrangement` のテスト既存 | Low |
| AC4: transfer_to_hbitmap | unit test | — | Medium（Map/Unmap のモック困難） |
| AC5: 回帰なし | `cargo test` | CI 相当 | Low |
| AC6: 共存ビルド | `cargo build` | — | Low |

**制約**: D3D11/D2D1 依存のテストは GPU がない CI 環境では実行できない可能性あり。既存テストも同様の制約を持つため、プロジェクト全体として受容されている。

---

## 3. 実装アプローチ評価

### Option A: 全て新規コンポーネント（推奨）

要件の通り、`compositor.rs`, `compositor_systems.rs`, `ulw.rs` を新規作成し、`arrangement.rs` と `systems.rs` を最小限拡張する。

**新規ファイル:**
- `ecs/graphics/compositor.rs` — `WindowD3D11Compositor` コンポーネント
- `ecs/graphics/compositor_systems.rs` — `compositor_init_system`, `composite_render_system`
- `com/ulw.rs` — `transfer_to_hbitmap`

**拡張ファイル:**
- `ecs/graphics/mod.rs` — `mod compositor; mod compositor_systems;` 追加
- `com/mod.rs` — `pub mod ulw;` 追加

**Trade-offs:**
- ✅ DComp パイプラインに一切触れない（共存保証）
- ✅ 新規ファイルが中心で回帰リスク最小
- ✅ 既存パターンを忠実に踏襲
- ✅ Opacity 累積は composite_render_system 内の階層走査で実施（Layout 層への影響なし）

### Option B: 既存システムを拡張

`render_surface` を拡張して合成描画を追加する。`WindowGraphics` にビットマップを追加する。

**Trade-offs:**
- ❌ DComp パイプラインへの干渉リスク（Phase 1 の Non-Goal 違反）
- ❌ 既存の 1419 行 `systems.rs` がさらに肥大化
- ❌ ロールバックが困難
- ✅ ファイル数が増えない

**→ 却下: Non-Goal 違反のリスクが高い**

### Option C: ハイブリッド

Option A をベースに、opacity 累積のみ Phase 1 から除外し、合成時に常に opacity `1.0` としておく。Phase 2 で `visual_property_sync_system` の DComp 呼び出しを除去する際に opacity 累積も同時に実装。

> **注**: 設計決定により Option C の前提（GlobalArrangement.global_opacity）は却下された。Opacity 累積は `CompositeContext` 手動累積方式で Phase 1 の `composite_render_system` 内に含まれる。

**Trade-offs:**
- ✅ Phase 1 のスコープが大幅に縮小
- ✅ Opacity 累積の設計判断を先送りできる
- ❌ Phase 2 の負担増
- ❌ Phase 1 検証基準 AC2（opacity 合成テスト）が不完全

**→ 検討に値するが、要件合意が必要**

---

## 4. 実装複雑度とリスク

### 全体

| 指標 | 評価 | 根拠 |
|------|------|------|
| **工数** | **S-M（2-5日）** | 新規ファイル3つ、パターン流用が多いが D2D API 周りの実装・検証に時間要 |
| **リスク** | **Low-Medium** | D2D1 Bitmap 作成 / Map が新規 API、CompositeContext opacity 累積 |

### 要件別

| 要件 | 工数 | リスク | 備考 |
|------|------|--------|------|
| Req 1 | S | Low | パターン流用、API 調査のみ |
| Req 2 | M | Medium | CompositeContext opacity 累積、ダーティ判定ロジック |
| Req 3 | S | Low | `init_window_graphics` 流用 + サイズ変更/デバイスロスト検出 |
| Req 4 | S | Low | 標準的なメモリ操作 |
| Req 5 | S-M | Low | GPU テスト制約あり |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option A（全て新規コンポーネント）** を推奨。既存コードへの干渉最小で、Non-Goal の遵守が確実。

### 設計フェーズで確定が必要な項目

1. **DeviceContext 共有**: `GraphicsCore` の共有 DC を合成で使うか、WindowD3D11Compositor 用に専用 DC を作るか
2. **ダーティ判定粒度**: ウィンドウ全体再合成 vs 差分更新（Phase 1 ではウィンドウ全体再合成で十分と想定）

### Research Items（設計フェーズで調査）

1. `ID2D1DeviceContext::CreateBitmap()` に `D2D1_BITMAP_OPTIONS_TARGET` を指定する際の具体的な `D2D1_BITMAP_PROPERTIES1` 構成
2. `CompositeContext` 手動累積方式での opacity 適用 API（D2D Effect or pre-multiplied alpha）
3. `ID2D1Bitmap1::Map()` / `Unmap()` の制約（CPU_READ ビットマップに対するスレッドセーフティ）
4. `ID2D1Bitmap::CopyFromBitmap()` の制約（ターゲットビットマップからステージングへのコピー）
5. `CreateDIBSection` で PBGRA32 top-down DIB を作成する際の `BITMAPINFO` 構成

---

## 6. 要件-資産マップ

| 要件 | 既存資産（再利用可能） | ギャップ（新規実装） | 制約 |
|------|----------------------|---------------------|------|
| Req 1 | `Option<Inner>` パターン、SparseSet | D2D Bitmap 作成（TARGET/CPU_READ）、DIBSection/DC 作成 | GPU 依存 |
| Req 2 | `draw_recursive` ロジック、`DrawImage`/`SetTransform` パターン | CompositeContext opacity 手動累積、ダーティ集約判定 | DComp 非干渉 |
| Req 3 | `init_window_graphics` パターン、resize/generation パターン | `WindowPos` サイズ取得、エラーログ、0×0 ガード | — |
| Req 4 | なし | Map/Copy/Unmap 全体 | — |
| Req 5 | `crates/wintf/tests/` パターン | GPU 依存テスト | CI 制約 |
