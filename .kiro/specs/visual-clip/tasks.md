# Implementation Plan

## Task Format Template

Use whichever pattern fits the work breakdown:

### Major task only
- [ ] {{NUMBER}}. {{TASK_DESCRIPTION}}{{PARALLEL_MARK}}
  - {{DETAIL_ITEM_1}}
  - _Requirements: {{REQUIREMENT_IDS}}_

### Major + Sub-task structure
- [ ] {{MAJOR_NUMBER}}. {{MAJOR_TASK_SUMMARY}}
- [ ] {{MAJOR_NUMBER}}.{{SUB_NUMBER}} {{SUB_TASK_DESCRIPTION}}{{SUB_PARALLEL_MARK}}
  - {{DETAIL_ITEM_1}}
  - {{DETAIL_ITEM_2}}
  - _Requirements: {{REQUIREMENT_IDS}}_

---

## Implementation Tasks

- [ ] 1. ClipShape 型定義と Visual への統合
- [ ] 1.1 ClipShape enum を定義する
  - `ecs/graphics/clip.rs` を新規作成（新規モジュール）
  - 3バリアント実装: `Rectangle`, `RoundedRectangle { radius: f32 }`, `RoundedRectangleIndividual { top_left, top_right, bottom_left, bottom_right: f32 }`
  - `Debug`, `Clone`, `PartialEq` を derive
  - `RoundedRectangle::new(radius)` と `RoundedRectangleIndividual::new(...)` コンストラクタで負値を 0.0 にクランプ
  - クランプ時は `warn!` ログを出力（`Visual::set_opacity` パターンに準拠）
  - `mod.rs` に `mod clip; pub use clip::*;` を追加してエクスポート
  - _Requirements: 1, 8_

- [ ] 1.2 Visual コンポーネントに clip フィールドを追加する
  - `Visual` struct に `pub clip: Option<ClipShape>` フィールドを追加
  - `Visual::default()` で `clip: None` を設定
  - `set_clip(&mut self, clip: Option<ClipShape>)` セッターを実装（既存パターンに準拠）
  - `Changed<Visual>` で bevy_ecs により自動検知される（追加実装不要）
  - _Requirements: 2_

---

- [ ] 2. COM API ラッパー拡張
- [ ] 2.1 (P) DCompositionDeviceExt に create_rectangle_clip を実装する
  - `com/dcomp.rs` の `impl DCompositionDeviceExt for IDCompositionDevice3` ブロック内に追加
  - `fn create_rectangle_clip(&self) -> Result<IDCompositionRectangleClip>` を実装
  - `#[inline(always)]` + `unsafe { self.CreateRectangleClip() }` パターンを使用
  - _Requirements: 6_

- [ ] 2.2 (P) DCompositionVisualExt に set_clip を実装する
  - `com/dcomp.rs` の `impl DCompositionVisualExt for IDCompositionVisual3` ブロック内に追加
  - `fn set_clip<P0>(&self, clip: P0) -> Result<()> where P0: Param<IDCompositionClip>` を実装
  - `set_content` と同一パターンを使用（`None` は null ポインタとして渡される）
  - _Requirements: 6_

- [ ] 2.3 (P) D2D1FactoryExt に create_rounded_rectangle_geometry を実装する
  - `com/d2d/mod.rs` の `impl D2D1FactoryExt` ブロック内に追加
  - `fn create_rounded_rectangle_geometry(&self, rounded_rect: &D2D1_ROUNDED_RECT) -> Result<ID2D1RoundedRectangleGeometry>` を実装
  - `#[inline(always)]` + `unsafe { self.CreateRoundedRectangleGeometry(...) }` パターンを使用
  - _Requirements: 6_

---

- [ ] 3. DComp モード — clip_sync_system 実装
- [ ] 3.1 clip_sync_system の基本構造を実装する
  - `ecs/graphics/systems/clip_sync.rs` を新規作成
  - `systems/mod.rs` に `mod clip_sync; pub use clip_sync::*;` を追加
  - `clip_sync_system(dcomp_resource, query)` 関数を定義
  - クエリ: `(&Arrangement, &GlobalArrangement, &Visual, &VisualGraphics)` + `Or<(Changed<Arrangement>, Changed<GlobalArrangement>, Changed<Visual>)>`
  - DComp デバイス取得（`dcomp_resource.dcomp()`、None なら早期 return）
  - 各エンティティで `VisualGraphics::visual()` から `IDCompositionVisual3` を取得（DComp モード判定）
  - `Arrangement.size` が (0, 0) の場合はクリップ適用をスキップ
  - `GlobalArrangement` のスケール値を適用して物理座標に変換（`physical_right = width * scale_x`, `physical_bottom = height * scale_y`）
  - `visual.clip` が `None` または size が (0, 0) の場合は `set_clip(None::<IDCompositionClip>)` でクリップ解除
  - エラー時は `error!` ログを出力して処理継続（Graceful Degradation）
  - _Requirements: 3, 4, 5, 7_

- [ ] 3.2 ClipShape バリアント別の RectangleClip 設定を実装する
  - `visual.clip` が `Some(clip_shape)` の場合、`create_rectangle_clip()` で `IDCompositionRectangleClip` を作成
  - `SetLeft(0.0)`, `SetTop(0.0)`, `SetRight(physical_right)`, `SetBottom(physical_bottom)` を呼び出し
  - `match clip_shape` で分岐:
    - `Rectangle`: すべての角の半径を 0.0 に設定（`SetTopLeftRadiusX/Y`, `SetTopRightRadiusX/Y`, `SetBottomLeftRadiusX/Y`, `SetBottomRightRadiusX/Y` すべて 0.0）
    - `RoundedRectangle { radius }`: すべての角に同一半径を設定（`physical_radius = radius * scale_x`）
    - `RoundedRectangleIndividual { tl, tr, bl, br }`: 各角に個別半径を設定（各値に `scale_x` を乗算）
  - `set_clip(rectangle_clip)` で適用
  - _Requirements: 4_

- [ ] 3.3 clip_sync_system をスケジュールに統合する
  - `ecs/graphics/systems/mod.rs` または `world.rs` のスケジュール定義箇所で `clip_sync_system` を登録
  - `Composition` スケジュールフェーズに配置
  - `visual_property_sync_system` の後に実行されるよう `.after(visual_property_sync_system)` を設定
  - _Requirements: 7_

---

- [ ] 4. ULW モード — render_subtree クリップ拡張
- [ ] 4.1 ClipGuard RAII 構造を実装する
  - `ecs/graphics/compositor_systems/render.rs` に `ClipGuard` struct を private として定義
  - `ClipType` enum を定義（`AxisAligned`, `Layer`）
  - `ClipGuard<'a>` フィールド: `dc: &'a ID2D1DeviceContext`, `clip_type: ClipType`
  - `unsafe fn push(dc, clip_shape, size) -> Result<Self>` を実装（Push 処理、成功時に Self を返す）
  - `impl Drop for ClipGuard<'_>` で `match self.clip_type` により適切な Pop メソッド（`PopAxisAlignedClip` または `PopLayer`）を呼び出し
  - 既存の `DcTargetGuard` パターンに準拠
  - _Requirements: 9_

- [ ] 4.2 Rectangle クリップ実装（PushAxisAlignedClip）
  - `ClipGuard::push` 内で `ClipShape::Rectangle` の場合、`dc.PushAxisAlignedClip(&D2D1_RECT_F { left: 0.0, top: 0.0, right: width, bottom: height }, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE)` を呼び出し
  - `ClipType::AxisAligned` を設定して `ClipGuard` を返す
  - Push 失敗時は `error!` ログを出力し、`Err` を返す（呼び出し側で `None` に変換される）
  - _Requirements: 3, 9_

- [ ] 4.3 RoundedRectangle クリップ実装（PushLayer + Geometry）
  - `ClipShape::RoundedRectangle { radius }` の場合、`dc.GetFactory()` で `ID2D1Factory` を取得
  - `factory.create_rounded_rectangle_geometry(&D2D1_ROUNDED_RECT { rect: (0,0,w,h), radiusX: radius, radiusY: radius })` で `ID2D1RoundedRectangleGeometry` を作成
  - `dc.PushLayer(&D2D1_LAYER_PARAMETERS1 { ... geometricMask: Some(geometry), ... }, None)` を呼び出し
  - `ClipType::Layer` を設定して `ClipGuard` を返す
  - エラー時は `error!` ログ + `Err` 返却
  - **Dependency Note**: Task 2.3 の `create_rounded_rectangle_geometry` が必要
  - _Requirements: 9_

- [ ] 4.4 RoundedRectangleIndividual クリップ実装（PathGeometry）
  - `ClipShape::RoundedRectangleIndividual { tl, tr, bl, br }` の場合、既存の `factory.create_path_geometry()` で `ID2D1PathGeometry` を作成
  - `geo.Open()` で `ID2D1GeometrySink` を取得
  - 各角に `AddArc` で個別半径の円弧を描画（4辺 + 4角の計8セグメント）
  - `sink.Close()` で PathGeometry を確定
  - `dc.PushLayer` で geometric_mask として PathGeometry を指定
  - `ClipType::Layer` を設定して `ClipGuard` を返す
  - _Requirements: 9_

- [ ] 4.5 render_subtree クエリに Arrangement 追加とクリップフロー統合
  - `render_subtree` 関数のクエリに `&Arrangement` を追加: `Query<(&Arrangement, &GlobalArrangement, Option<&GraphicsCommandList>, &Visual, Option<&Children>)>`
  - クリップフロー（SetTransform 後、draw_with_opacity 前）:
    1. `let _clip_guard = if let Some(clip_shape) = &visual.clip { ... }` で ClipGuard を生成
    2. `arrangement.size` が (0, 0) より大きい場合のみ `ClipGuard::push(ctx.dc, clip_shape, arrangement.size).ok()` を呼び出し
    3. `_clip_guard` をスコープに保持（Drop 時に自動 Pop）
  - draw と children 再帰を実行（エラー時も Drop により Pop が保証される）
  - _Requirements: 3, 5, 7, 9_

---

- [ ] 5. クリッピング検証デモ
- [ ] 5.1 clip_demo の基本構造を実装する
  - `examples/clip_demo.rs` を新規作成
  - `multi_backend_demo.rs` のデュアルウィンドウパターンをテンプレートとして使用
  - ULW モードウィンドウと DComp モードウィンドウの2つを生成
  - `cargo run --example clip_demo` で実行可能な構成
  - 基本的なイベントループとウィンドウ管理を実装
  - _Requirements: 10_

- [ ] 5.2 全3バリアントのレイアウト構築
  - 各ウィンドウに3つの領域を配置（Rectangle, RoundedRectangle, RoundedRectangleIndividual）
  - 各領域は親要素（クリップ適用）+ 子要素（はみ出すサイズ）で構成
  - flex grow によるサイズ可変レイアウトを使用
  - 視覚的にクリップ効果が確認できる配色・サイズ設定（例: 親は半透明背景、子は濃色で親より大きい）
  - _Requirements: 10_

- [ ] 5.3 ウィンドウリサイズ対応
  - ウィンドウサイズ変更時にレイアウトが再計算されるよう設定（flex grow が自動対応）
  - `Arrangement` サイズ変更が `Changed<Arrangement>` で検知され、クリップ領域も自動追従することを確認
  - デモ実行時にウィンドウをリサイズして、クリップ領域が動的に更新されることを視覚検証
  - _Requirements: 10_

---

- [ ] 6. テスト実装
- [ ]* 6.1 Unit tests（ClipShape と Visual）
  - `ecs/graphics/clip.rs` に `#[cfg(test)] mod tests` セクションを追加
  - `ClipShape` 各バリアントの作成テスト（Rectangle, RoundedRectangle, RoundedRectangleIndividual）
  - `PartialEq` による比較テスト
  - 負の radius 値が 0.0 にクランプされることのテスト（`RoundedRectangle::new(-5.0)` → `radius == 0.0`）
  - `Visual::default()` の `clip` が `None` であることのテスト
  - _Requirements: 1, 2_

- [ ]* 6.2 Integration tests（clip_sync_system と render_subtree）
  - `tests/graphics/clip_test.rs` を新規作成（または既存 graphics テストに追加）
  - `clip_sync_system` テスト: DComp モードで `Visual.clip` 変更時に `SetClip` が呼ばれることを検証（モック可能な場合）
  - `render_subtree` テスト: ULW モードで clip 付き Visual の描画時に Push/Pop が正しく呼ばれることを検証
  - `Changed<Arrangement>` によるクリップ再計算のテスト（Arrangement サイズ変更 → クリップ更新）
  - **Note**: COM API のモックが困難な場合、E2E デモ（`clip_demo`）での視覚検証を優先
  - _Requirements: 4, 9_

---

## Task Summary

- **Major tasks**: 6
- **Sub-tasks**: 20（うちオプショナル2: 6.1*, 6.2*）
- **Parallel-capable tasks**: 3（Task 2.1, 2.2, 2.3）
- **Requirements coverage**: 全10要件（Requirement 1〜10）をカバー
- **Average task size**: 1-3 hours per sub-task

## Execution Notes

- **Recommended Start**: Task 1.1（ClipShape 型定義、他のすべてのタスクの基礎）
- **Parallel Execution**: Task 2（COM API ラッパー）の3サブタスクは Task 1 と並行実行可能
- **Critical Path**: Task 1 → Task 3/4（並列） → Task 5 → Task 6*
- **Task 3 と 4 の並列性**: Task 1 と 2 完了後、Task 3（DComp）と Task 4（ULW）は互いに独立しており並列実行可能
- **Optional Tests**: Task 6（特に 6.2）は COM API のモック困難により、Task 5（clip_demo）での視覚検証で代替可能
