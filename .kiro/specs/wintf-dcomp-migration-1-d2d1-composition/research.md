# ギャップ分析: wintf-dcomp-migration-1-d2d1-composition (v2)

## 概要

本分析は、Phase 1「D2D1合成スタック構築」の要件 v2 と既存コードベースとのギャップを調査し、実装戦略を評価する。

### v2 改訂のコンテキスト

以下の子仕様が実装完了し、Phase 1 の前提条件が全て充足されたため、ギャップ分析を全面的に再評価した:

| 子仕様 | 完了による影響 |
|--------|--------------|
| `wintf-dcomp-migration-0-visual-opacity-dataflow` | `Visual.opacity` / `Visual.is_visible` のデータフロー確立。v1 で「デッドフィールド」「初ワイヤリング」と記載していた項目が全て解消 |
| `wintf-taffy-child-order-fix` | `Children` コンポーネントが Z-order の権威的ソースとして確立。v1 で未確認だった兄弟順序の保証が確定 |

---

## 1. 現状調査

### 1.1 関連モジュール構成

| モジュール | パス | 役割 | 本仕様との関係 |
|-----------|------|------|---------------|
| `graphics/mod.rs` | `ecs/graphics/mod.rs` | モジュール定義・再エクスポート | 新モジュール登録先（`mod compositor; mod compositor_systems;` 追加） |
| `graphics/core.rs` | `ecs/graphics/core.rs` | `GraphicsCore` リソース（D3D11, D2D1, DComp 初期化） | `ID2D1DeviceContext` 取得元。**generation フィールドなし** |
| `graphics/components.rs` | `ecs/graphics/components.rs` | `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics`, `Visual` | `Option<Inner>` パターン参考。`Visual` は Phase 0 で完全ワイヤリング済み |
| `graphics/command_list.rs` | `ecs/graphics/command_list.rs` | `GraphicsCommandList`（`ID2D1CommandList` ラッパー） | 合成描画の入力（変更なし） |
| `graphics/systems.rs` | `ecs/graphics/systems.rs` | DComp描画・合成システム群（1461行） | `init_window_graphics` パターン参考。`draw_recursive` は dead code だが走査ロジック参考 |
| `layout/arrangement.rs` | `ecs/layout/arrangement.rs` | `Arrangement`, `GlobalArrangement` | 座標変換参照元（変更なし）。`transform: Matrix3x2` |
| `layout/metrics.rs` | `ecs/layout/metrics.rs` | `Opacity` コンポーネント | Phase 0 で `#[deprecated]` 済み。Phase 1 では参照しない |
| `layout/systems.rs` | `ecs/layout/systems.rs` | `propagate_global_arrangements` | 座標伝播参照（変更なし） |
| `common/tree_system.rs` | `ecs/common/tree_system.rs` | ジェネリック階層伝播アルゴリズム | `Mul<L, Output=G>` trait 制約（opacity を Layout 層に入れない根拠） |
| `common/tree_iter.rs` | `ecs/common/tree_iter.rs` | `DepthFirstReversePostOrder` イテレータ | 合成走査のパターン参考 |
| `com/d2d/mod.rs` | `com/d2d/mod.rs` | D2D1 拡張 trait 群（292行） | `draw_image`, `set_transform`, `clear` 等 — 合成描画で直接使用 |
| `com/mod.rs` | `com/mod.rs` | COM モジュール定義 | `pub mod ulw;` 追加先 |
| `window.rs` | `ecs/window.rs` | `WindowPos`, `WindowHandle` 等 | `WindowPos.size: Option<SIZE>` — リサイズ検出のサイズ取得元 |
| `world.rs` | `ecs/world.rs` | スケジュール・システム登録 | Phase 2 で変更（Non-Goal） |

### 1.2 既存パターンと規約

#### GPU リソースコンポーネントの `Option<Inner>` パターン

既存の `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` は全て同一パターン:

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

**`WindowD3D11Compositor` はこのパターンに完全に従える。** `generation` フィールドと `increment_generation()` メソッドは `WindowGraphics` に実装済みで、そのまま踏襲する。

#### デバイスロスト復旧フロー（v2: 詳細確認済み）

```
GraphicsCore::invalidate()  ← GraphicsCore 自体には generation フィールドなし
  → init_graphics_core: GraphicsCore 再作成（Option<GraphicsCoreInner> を Some に）
    → HasGraphicsResources.set_changed() を全エンティティにトリガー
      → init_window_graphics: Or<(Without<WindowGraphics>, Changed<HasGraphicsResources>)> で検出
        → !is_valid() なら re-initialize（generation を wrapping_add(1)）
        → None なら新規作成
```

**v1 からの修正**: `GraphicsCore` には `generation` フィールドが**存在しない**。デバイスロスト検出は `HasGraphicsResources` マーカーの `Changed` 検出と `is_valid()` の組み合わせで行われる。`WindowD3D11Compositor` の `compositor_init_system` も同一の `Or<(Without<WindowD3D11Compositor>, Changed<HasGraphicsResources>)>` パターンで実装すればよい。

#### 階層伝播の `Mul` trait 制約

`propagate_parent_transforms<L, G, M>` は以下の制約を要求:

```rust
L: Component + Copy + Into<G>,
G: Component<Mutability = Mutable> + Copy + PartialEq + Mul<L, Output = G>,
M: Component<Mutability = Mutable>,
```

**設計決定（v1 から維持）: GlobalArrangement には opacity を追加しない。** `Arrangement` に opacity フィールドがなく `Mul` trait での累積が不自然であることも、Opacity を Layout 層から分離する決定を支持する。代わりに `CompositeContext` で `composite_render_system` 内の `render_subtree()` 再帰走査中に手動累積する。

### 1.3 Visual 描画属性の現状（v2: Phase 0 完了反映）

**Visual 構造体**: 描画属性（opacity, is_visible, transform_origin）を保持する ECS コンポーネント

| 名前 | 型 | 場所 | 状態 (v2) |
|------|------|------|-----------|
| `Visual.opacity` | `f32` フィールド | `components.rs` | ✅ **アクティブ** — Widget 層から `set_opacity()` で書き込み。`clamped_opacity()` で 0.0..=1.0 クランプ取得。`visual_property_sync_system` が `Changed<Visual>` で検出し DComp に同期 |
| `Visual.is_visible` | `bool` フィールド | `components.rs` | ✅ **アクティブ** — Widget 層から `set_visible()` で書き込み。`visual_property_sync_system` が `Changed<Visual>` で検出し DComp に同期 |
| `Opacity` | `Component` | `metrics.rs` | ⚠️ `#[deprecated]` 済み — Phase 0 で廃止マーク。Phase 1 では一切参照しない。完全削除は Phase 4 |

**処理の伝搬層（v2 で確立済み）**:
```
Widget
  → Visual.set_opacity() / Visual.set_visible()  【Phase 0 で確立】
    → Changed<Visual> 検出
      → visual_property_sync_system: DComp Visual に同期（Phase 2 で除去）
      → composite_render_system: D2D1 合成で直接参照（Phase 1 で新規）
```

**v1 からの変更**: v1 では「Visual.opacity は現在デッドフィールド。初ワイヤリング必要」「visual_property_sync_system が Opacity コンポーネント（誤実装）を参照」と記載していたが、Phase 0 でこれらは全て解消済み。Phase 1 の `composite_render_system` は `Visual.clamped_opacity()` と `Visual.is_visible` を読み取るだけでよい。

### 1.4 合成ターゲット方式の差異（v2 新規セクション）

**現行パイプライン（DComp 方式）**:
```
render_surface(): 
  IDCompositionSurface::BeginDraw() → ID2D1DeviceContext を取得（Surface 固有の DC）
  → 各エンティティの GraphicsCommandList を DrawImage
  → IDCompositionSurface::EndDraw()
  → IDCompositionDevice3::Commit() でハードウェア合成
```

**新パイプライン（D2D1 合成方式）**:
```
composite_render_system():
  ID2D1DeviceContext::SetTarget(composition_bitmap)  ← ★新規パターン
  → ID2D1DeviceContext::BeginDraw()
  → 全エンティティを depth-first pre-order 走査
    → SetTransform(GlobalArrangement.transform)
    → DrawImage(entity.GraphicsCommandList)
  → ID2D1DeviceContext::EndDraw()
```

**重要な発見**: 現行コードに `SetTarget` の使用実績は**ゼロ**。`render_surface` は DComp `BeginDraw` が返す DC に描画するため、明示的なターゲット設定が不要だった。新 `composite_render_system` では `GraphicsCore` の共有 DC に対して `SetTarget(composition_bitmap)` を呼び出す**新規パターン**が必要。

**影響**:
- `D2D1DeviceContextExt` trait に `set_target()` / `get_target()` の追加が必要になる可能性
- 共有 DC のターゲット復元（描画完了後に元のターゲットに戻す）の考慮が必要

### 1.5 D2D1 拡張 trait の現状（v2 新規セクション）

`D2D1DeviceContextExt` trait（`com/d2d/mod.rs`, 292行）の合成描画で使える既存 API:

| メソッド | 用途 | 合成での利用 |
|---------|------|-------------|
| `set_transform(&Matrix3x2)` | 変換行列設定 | `GlobalArrangement.transform` の適用 |
| `clear(Option<&D2D1_COLOR_F>)` | サーフェスクリア | 合成ビットマップ初期化（透明クリア） |
| `draw_image(&ID2D1Image, ...)` | イメージ描画 | `GraphicsCommandList` の合成描画 |
| `fill_rectangle(...)` | 矩形塗り | デバッグ用途 |
| `create_bitmap_from_wic_bitmap(...)` | WIC → D2D Bitmap | **利用しない**（`CreateBitmap` を直接使用） |

**ギャップ**:
- `set_target()` / `get_target()` — 未実装（`ID2D1DeviceContext::SetTarget` / `GetTarget` のラッパー）
- `begin_draw()` / `end_draw()` — 未実装（`ID2D1RenderTarget::BeginDraw` / `EndDraw` のラッパー）
- `create_bitmap()` with `D2D1_BITMAP_PROPERTIES1` — 未実装（`CreateBitmap` の TARGET/CPU_READ 版）

> 注: `begin_draw` / `end_draw` は現行コードでは `unsafe { dc.BeginDraw() }` / `unsafe { dc.EndDraw() }` で直接呼び出されている（`render_surface` 内）。ラッパー追加は任意。

---

## 2. 要件ごとのフィージビリティ分析

### Req 1: WindowD3D11Compositor コンポーネント

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: 4リソース管理 | `WindowGraphics` の `Option<Inner>` パターン | `ID2D1Bitmap1` 作成（TARGET/CPU_READ）、`CreateDIBSection`、`CreateCompatibleDC` — 全て新規 API | Medium |
| AC2: ライフサイクル API | `WindowGraphics` と同パターン（`new`/`resize`/`invalidate`/`is_valid`/`generation`） | `is_dirty()`/`set_dirty()` は新規追加（v2 要件で昇格） | Low |
| AC3: 同一サイズ保証 | なし | `WindowD3D11CompositorInner` でサイズを1か所で管理すれば自然に保証 | Low |
| AC4: SparseSet | `#[component(storage = "SparseSet")]` パターン既存 | 属性追加のみ | Low |
| AC5: `Option<Inner>` + `Drop` | `WindowGraphics` の `Option<WindowGraphicsInner>` パターン | GDI リソース（`DeleteObject`, `DeleteDC`）の `Drop` 実装が新規 | Low |
| AC6: ファイル配置 | `graphics/mod.rs` に登録パターン既存 | `mod compositor;` 追加のみ | Low |

**新規 Win32/D2D API 呼び出し一覧**:

| API | 用途 | 既存コードでの使用実績 |
|-----|------|----------------------|
| `ID2D1DeviceContext::CreateBitmap()` with `D2D1_BITMAP_OPTIONS_TARGET` | 合成先ビットマップ | ❌ なし（WIC 経由の `CreateBitmapFromWicBitmap` のみ） |
| `ID2D1DeviceContext::CreateBitmap()` with `D2D1_BITMAP_OPTIONS_CPU_READ \| CANNOT_DRAW` | ステージングビットマップ | ❌ なし |
| `CreateDIBSection()` | PBGRA32 top-down DIB | ❌ なし（全て DComp 経由） |
| `CreateCompatibleDC()` | メモリ DC | ❌ なし |
| `SelectObject()` | HBITMAP → MemoryDC | ❌ なし |
| `DeleteObject()` / `DeleteDC()` | GDI リソース解放（`Drop`） | ❌ なし |

### Req 2: composite_render_system（v2: 難度引き下げ）

| AC | 既存資産 | ギャップ | 難度 (v2) |
|----|---------|--------|-----------|
| AC1: 深さ優先走査 | `draw_recursive()` パターン（dead code）| `Children` の pre-order 走査ロジック流用。`wintf-taffy-child-order-fix` で兄弟順序保証済み | Low |
| AC2: Transform + DrawImage | `D2D1DeviceContextExt::set_transform()` + `draw_image()` | パターンは完全に既存。`GlobalArrangement.transform: Matrix3x2` をそのまま適用 | Low |
| AC3: is_visible スキップ | `Visual.is_visible` ✅ Phase 0 で確立済み | `Visual.is_visible` を読み取るだけ。**v1 の Medium → v2 で Low に引き下げ** | **Low** ⬇️ |
| AC4: opacity 累積計算 | `Visual.clamped_opacity()` ✅ Phase 0 で確立済み | `CompositeContext` で `accumulated * clamped_opacity()` を再帰渡し | **Low** ⬇️ |
| AC5: opacity 適用描画 | なし | D2D Effect or pre-multiplied alpha で累積 opacity を適用。**PushLayer 不使用** | Medium |
| AC6: opacity==0 スキップ | `Visual.clamped_opacity()` | 累積 opacity が 0.0 ならサブツリースキップ（早期脱出） | Low |
| AC7: CopyFromBitmap | なし | `ID2D1Bitmap1::CopyFromBitmap()` + `set_dirty(true)` — 新規 API | Low |
| AC8: ダーティ判定 | `Changed<GraphicsCommandList>`, `Changed<GlobalArrangement>`, `Changed<Visual>` | ウィンドウレベルの集約判定（3つの Changed の OR） | Medium |
| AC9: 既存システム非侵襲 | `GraphicsCommandList` が DComp 非依存設計 | 入力側完全既存。合成システムは消費側のみ | Low |
| AC10: ファイル配置 | — | `compositor_systems.rs` 新規作成 | Low |

**v1 → v2 の難度変更理由**:
- AC3 (is_visible): v1 では「デッドフィールド、初ワイヤリング」で Medium → Phase 0 でワイヤリング確立済みのため Low
- AC4 (opacity 累積): v1 では「Visual.opacity 未使用のためワイヤリング含む」で Medium → Phase 0 で `set_opacity()` / `clamped_opacity()` 確立済みのため、累積計算ロジック自体は Low。**描画適用**（D2D Effect 等）は新 AC5 として Medium を維持

**合成ターゲット切替（v2 詳細）**:

現行の `render_surface` は `IDCompositionSurface::BeginDraw()` が返す DC に描画する（`SetTarget` 不使用）。新 `composite_render_system` は `GraphicsCore` の共有 `ID2D1DeviceContext` に対して:

1. `SetTarget(composition_bitmap)` — 合成ビットマップをターゲット設定 ★新規パターン
2. `BeginDraw()` → 走査 + 描画 → `EndDraw()`
3. `SetTarget(null)` or 元のターゲットに復元

**SetTarget の windows crate API** (`windows 0.62.2`):
```rust
// ID2D1DeviceContext は ID2D1RenderTarget を継承
// SetTarget は ID2D1DeviceContext のメソッド
unsafe { dc.SetTarget(Some(&composition_bitmap.cast::<ID2D1Image>()?)) };
```

### Req 3: compositor_init_system

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: WindowHandle 検出 | `init_window_graphics` の `Or<(Without<Xxx>, Changed<HasGraphicsResources>)>` パターン | `Without<WindowD3D11Compositor>` に置換するだけ | Low |
| AC2: GraphicsCore DC 取得 | `GraphicsCore::device_context() → Option<&ID2D1DeviceContext>` 既存 | そのまま利用可能 | Low |
| AC3: リサイズ検出 | `WindowPos.size: Option<SIZE>` でサイズ取得可能 | `WindowD3D11Compositor` 内部の `cached_size` との比較（推奨方式 A） | Low |
| AC4: デバイスロスト復旧 | `init_window_graphics` の `!is_valid()` + `wrapping_add(1)` generation パターン | パターン流用。**GraphicsCore 自体に generation なし** — `HasGraphicsResources` の `Changed` + `is_valid()` で検出 | Low |
| AC5: エラーハンドリング | `tracing::error` パターン既存 | `invalidate()` → ログ → 次フレーム再試行 | Low |
| AC6: 0×0 ガード | `calculate_surface_size_from_global_arrangement` にサイズ0チェック既存 | `WindowPos.size` で同様のガード | Low |
| AC7: ファイル配置 | — | `compositor_systems.rs` に同居 | Low |

**v1 → v2 の変更**:
- AC3: v1 で Medium → `WindowPos.size` の存在を確認済みのため Low に引き下げ
- AC4: v1 で「generation 不一致検出」と記載 → `GraphicsCore` に generation フィールドが**ない**ことを確認。正確には `Changed<HasGraphicsResources>` + `is_valid()` パターン

**リサイズ検出のアプローチ（v2: 確定）**:
- **Option A（採用）**: `WindowD3D11Compositor` 内部に `cached_size: (u32, u32)` を保持し、`WindowPos.size` と比較。不一致なら `resize()` 呼び出し
- ~~Option B: `Changed<WindowPos>` フィルタ~~ — レイアウト変更でも発火するため過剰再作成のリスクあり

### Req 4: transfer_to_hbitmap

| AC | 既存資産 | ギャップ | 難度 |
|----|---------|--------|------|
| AC1: Map/Copy/Unmap | なし | `ID2D1Bitmap1::Map(D2D1_MAP_OPTIONS_READ)` / `Unmap()` は完全新規 | Medium |
| AC2: pitch≠stride 行単位コピー | なし | 標準的な行単位メモリコピーロジック（`for row in 0..height`) | Low |
| AC3: 単一 memcpy 最適化 | なし | `std::ptr::copy_nonoverlapping` — pitch == stride の場合 | Low |
| AC4: com/ulw.rs 配置 | `com/mod.rs` に `pub mod xxx;` パターン | `pub mod ulw;` 追加 | Low |
| AC5: エラーハンドリング | `windows::core::Result` パターン既存 | そのまま利用 | Low |

**D2D1 Map API 詳細（v2 調査済み）**:
```rust
// ID2D1Bitmap1::Map (windows 0.62.2)
unsafe fn Map(
    &self,
    options: D2D1_MAP_OPTIONS,  // D2D1_MAP_OPTIONS_READ
    mappedRect: *mut D2D1_MAPPED_RECT,  // { pitch: u32, bits: *mut u8 }
) -> Result<()>;

unsafe fn Unmap(&self) -> Result<()>;
```

**制約**:
- `D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW` でビットマップを作成する必要がある
- Map 中は他の D2D 操作がブロックされる可能性あり（GPU → CPU 同期）
- `D2D1_MAPPED_RECT.pitch` は GPU ドライバ依存で `width * 4` と一致しない場合がある

### Req 5: Phase 1 検証基準

| AC | テスト種別 | 既存パターン | 難度 |
|----|-----------|-------------|------|
| AC1: リソース作成 | unit test | `crates/wintf/tests/` に多数のテスト | Low（GPU 依存） |
| AC2: 合成描画 | integration test | `dcomp_demo.rs` 等の examples | Medium |
| AC3: opacity 累積 | integration test | Phase 0 で `Visual.opacity` テスト基盤あり | Low |
| AC4: transfer_to_hbitmap | unit test | — | Medium（Map/Unmap）|
| AC5: 回帰なし | `cargo test` | CI 相当 | Low |
| AC6: 共存ビルド | `cargo build` | — | Low |

**制約**: D3D11/D2D1 依存のテストは GPU がない CI 環境では実行できない可能性あり。既存テストも同様の制約を持つため、プロジェクト全体として受容されている。

---

## 3. 実装アプローチ評価

### Option A: 全て新規コンポーネント（推奨 — v1 から維持）

要件の通り、`compositor.rs`, `compositor_systems.rs`, `ulw.rs` を新規作成し、既存ファイルは最小限の `mod` 追加のみ。

**新規ファイル:**
- `ecs/graphics/compositor.rs` — `WindowD3D11Compositor` コンポーネント
- `ecs/graphics/compositor_systems.rs` — `compositor_init_system`, `composite_render_system`
- `com/ulw.rs` — `transfer_to_hbitmap`

**拡張ファイル（mod 追加のみ）:**
- `ecs/graphics/mod.rs` — `pub mod compositor; pub mod compositor_systems;`
- `com/mod.rs` — `pub mod ulw;`

**潜在的な追加変更:**
- `com/d2d/mod.rs` — `set_target()` / `get_target()` を `D2D1DeviceContextExt` trait に追加（任意: `unsafe` 直接呼び出しでも可）

**Trade-offs:**
- ✅ DComp パイプラインに一切触れない（共存保証）
- ✅ 新規ファイルが中心で回帰リスク最小
- ✅ 既存パターンを忠実に踏襲
- ✅ Phase 0 完了により `Visual` データフローのワイヤリング不要（v1 比でスコープ縮小）

### Option B: 既存システムを拡張 → **却下（v1 から維持）**

- ❌ Non-Goal 違反（DComp パイプラインへの干渉リスク）
- ❌ 既存の 1461 行 `systems.rs` がさらに肥大化
- ❌ ロールバック困難

### Option C: opacity 先送り → **不要化（v2）**

v1 では「opacity 累積のみ Phase 1 から除外」を検討候補としていたが、Phase 0 で `Visual.opacity` データフローが確立されたことにより:

- `composite_render_system` での opacity 累積は `Visual.clamped_opacity()` を呼ぶだけ（データフロー構築不要）
- 実装コストが大幅に低下したため先送りの動機が消失
- **Option C は不要化。Option A で opacity 累積を含めて Phase 1 に実装する。**

---

## 4. 実装複雑度とリスク（v2 再評価）

### 全体

| 指標 | 評価 (v2) | v1 比 | 根拠 |
|------|----------|-------|------|
| **工数** | **S（1-3日）** | ⬇️ S-M → S | Phase 0 完了で Visual ワイヤリング不要。パターン流用率向上 |
| **リスク** | **Low** | ⬇️ Low-Medium → Low | 唯一の新規 API 群（D2D Bitmap 作成/Map/SetTarget）は標準的な Win32/D2D パターン |

### 要件別

| 要件 | 工数 (v2) | リスク (v2) | v1 比変更 | 備考 |
|------|----------|------------|-----------|------|
| Req 1 | S | Low | — | パターン流用、新規 API は標準的 |
| Req 2 | S-M | Low | ⬇️ M→S-M, Medium→Low | Phase 0 完了で opacity/visibility ワイヤリング不要 |
| Req 3 | S | Low | — | `init_window_graphics` 完全流用 |
| Req 4 | S | Low | — | 標準的なメモリ操作 |
| Req 5 | S | Low | — | GPU テスト制約は既存と同様 |

### リスク項目

| リスク | 影響度 | 発生確率 | 緩和策 |
|--------|--------|---------|--------|
| `SetTarget` 後の共有 DC 状態復元忘れ | High | Low | `SetTarget(null)` を `EndDraw()` 後に必ず呼ぶ。デザインで RAII ガードを検討 |
| `ID2D1Bitmap1::Map` の GPU → CPU 同期レイテンシ | Medium | Medium | Phase 1 では毎フレーム Map は非推奨。ダーティフラグで必要時のみ転送 |
| GDI リソース（HBITMAP/DC）のリーク | High | Low | `Drop` 実装で保証。テストで検証 |
| `D2D1_BITMAP_OPTIONS_TARGET` と `D2D1_BITMAP_OPTIONS_CPU_READ` の排他制約 | High | Low | 設計で2つの別ビットマップとして管理（要件 AC1 の通り） |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option A（全て新規コンポーネント）** を推奨（v1 から維持）。Phase 0 完了により実装コストがさらに低下。

### 設計フェーズで確定が必要な項目（v2 更新）

| 項目 | 状態 | 推奨 |
|------|------|------|
| DeviceContext 共有 vs 専用 | **要確定** | 共有 DC（`GraphicsCore::device_context()`）を使用し、`SetTarget` でターゲット切替。専用 DC は Phase 1 では不要 |
| ダーティ判定粒度 | **要確定** | Phase 1 ではウィンドウ全体再合成（差分更新は Phase 2 以降で最適化） |
| `D2D1DeviceContextExt` への追加 | **要確定** | `set_target()` ラッパーを trait に追加するか、`unsafe` 直接呼び出しか |
| opacity 適用方式 | **要確定** | D2D Effect（`D2D1_OPACITY_METADATA`）vs pre-multiplied alpha 操作 vs `DrawImage` の opacity パラメータ |

### 解消済み Research Items（v2）

v1 で「Research Needed」だった項目のうち、Phase 0 完了で解消されたもの:

| 項目 | v1 状態 | v2 状態 |
|------|---------|---------|
| `Visual.opacity` のワイヤリング方法 | Research Needed | ✅ `set_opacity()` / `clamped_opacity()` で確立済み |
| `Visual.is_visible` のワイヤリング方法 | Research Needed | ✅ `set_visible()` で確立済み |
| `visual_property_sync_system` の修正方法 | Research Needed | ✅ `Changed<Visual>` に移行済み。Phase 1 では非変更 |
| `Children` の兄弟順序保証 | 暗黙の前提 | ✅ `wintf-taffy-child-order-fix` で明示的に保証 |

### 残存 Research Items（設計フェーズで調査）

1. `ID2D1DeviceContext::CreateBitmap()` に `D2D1_BITMAP_OPTIONS_TARGET` を指定する際の具体的な `D2D1_BITMAP_PROPERTIES1` 構成（pixelFormat, dpiX/dpiY, bitmapOptions）
2. opacity 適用の D2D API 選択: `DrawImage` の interpolationMode/compositeMode パラメータで opacity 指定可能か、または `ID2D1Effect`（`CLSID_D2D1Opacity`）が必要か
3. `ID2D1Bitmap1::Map()` / `Unmap()` の制約（Map 中の DC ロック範囲、GPU 同期タイミング）
4. `ID2D1Bitmap::CopyFromBitmap()` の制約（TARGET → CPU_READ ビットマップ間のコピー可否）
5. `CreateDIBSection` で PBGRA32 top-down DIB を作成する際の `BITMAPINFO` 構成（`biHeight` を負値にする）

---

## 6. 要件-資産マップ（v2 更新）

| 要件 | 既存資産（再利用可能） | ギャップ（新規実装） | 制約 |
|------|----------------------|---------------------|------|
| Req 1 | `Option<Inner>` パターン、SparseSet、`generation` パターン | D2D Bitmap 作成（TARGET/CPU_READ）、DIBSection/DC 作成、GDI `Drop` | GPU 依存 |
| Req 2 | `draw_recursive` 走査ロジック、`DrawImage`/`SetTransform` 既存、**`Visual.clamped_opacity()` / `is_visible` 確立済み** ✅ | `SetTarget` 新規パターン、opacity 適用描画、ダーティ集約判定 | DComp 非干渉 |
| Req 3 | `init_window_graphics` パターン完全流用、`WindowPos.size` 取得 | `cached_size` 比較、0×0 ガード | — |
| Req 4 | `windows::core::Result` パターン | Map/Copy/Unmap 全体、pitch/stride 分岐 | — |
| Req 5 | `crates/wintf/tests/` パターン、Phase 0 のテスト基盤 | GPU 依存 integration test | CI 制約 |

---

## 変更履歴

### v2 (2026-02-16): 子仕様完了に伴う全面再評価

**Phase 0 完了反映**:
- セクション 1.3: `Visual.opacity` / `Visual.is_visible` の状態を「デッドフィールド」→「✅ アクティブ（Phase 0 で確立済み）」に更新
- セクション 1.3: `visual_property_sync_system` の「誤った実装」記述を削除（`Changed<Visual>` に移行済み）
- Req 2 AC3/AC4: 難度を Medium → Low に引き下げ（データフロー構築が不要になったため）
- セクション 5: 「解消済み Research Items」セクションを追加

**taffy-child-order-fix 完了反映**:
- セクション 1.1: `Children` の Z-order 権威的ソース保証を明記
- Req 2 AC1: 兄弟順序保証を「暗黙の前提」→「明示的保証」に昇格

**新規調査結果のセクション追加**:
- セクション 1.4「合成ターゲット方式の差異」: 現行 DComp 方式と新 D2D1 方式の `SetTarget` パターン差異を文書化
- セクション 1.5「D2D1 拡張 trait の現状」: 合成描画で必要な既存/不足 API を一覧化

**v1 からの修正**:
- `GraphicsCore` に generation フィールドが「ある」前提 → 「**ない**」に修正。デバイスロスト検出は `HasGraphicsResources` + `Changed` + `is_valid()` パターン
- Req 3 AC3: リサイズ検出方式を Option A に確定
- Req 4: `ID2D1Bitmap1::Map` API シグネチャを調査結果として記載
- Option C（opacity 先送り）: Phase 0 完了により不要化を明記

**全体評価の変更**:
- 工数: S-M（2-5日） → **S（1-3日）** に引き下げ
- リスク: Low-Medium → **Low** に引き下げ
