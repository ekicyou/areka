# 実装計画: wintf-dcomp-migration-1-d2d1-composition

## タスク概要

Phase 1 — D2D1 合成スタック構築。DComp パイプラインを温存しながら、新しい D2D1 合成描画スタックを独立モジュールとして構築する。world.rs への登録は Phase 2 で行うため、本フェーズでは独立テスト可能な状態を目指す。

---

## 実装タスク

### Phase 1A: COM層・コンポーネント基盤

- [ ] 1. com/ulw.rs モジュール新規作成
  - `transfer_to_hbitmap()` 関数を実装する
  - ステージング ID2D1Bitmap1 を Map() → DIBSection メモリへコピー → Unmap() のフローを実装する
  - pitch と stride の不一致時の行単位コピーを実装する
  - pitch==stride 時の一括コピー最適化を実装する
  - `com/mod.rs` に `pub mod ulw;` を追加する
  - _Requirements: Req 4 AC1-AC5_

- [ ] 2. WindowD3D11Compositor コンポーネント実装
  - `ecs/graphics/compositor.rs` を新規作成する
  - `WindowD3D11CompositorInner` 構造体を定義する（composition_bitmap, staging_bitmap, hbitmap, memory_dc, dib_bits, size）
  - `WindowD3D11Compositor` 構造体を定義する（inner: Option, generation: u32, SparseSet storage）
  - `new()`, `resize()`, `invalidate()`, `is_valid()`, アクセサメソッドを実装する
  - D2D1 Bitmap 作成（TARGET / CPU_READ）、CreateDIBSection、CreateCompatibleDC を実装する
  - `Drop` 実装で GDI リソースを解放する
  - `ecs/graphics/mod.rs` に `pub mod compositor;` を追加する
  - _Requirements: Req 1 AC1-AC6_

### Phase 1B: ECSシステム実装

- [ ] 3. compositor_init_system 実装
  - `ecs/graphics/compositor_systems.rs` を新規作成する
  - `Or<(Without<WindowD3D11Compositor>, Changed<HasGraphicsResources>)>` クエリで新規ウィンドウ・デバイスロスト復旧を検出する
  - `GraphicsCore` から DC を取得して `WindowD3D11Compositor::new()` を呼び出す
  - `Changed<HasGraphicsResources>` + `!is_valid()` によるデバイスロスト再作成ロジックを実装する（GraphicsCore に generation なし）
  - リサイズ検出ロジック（`cached_size` vs `WindowPos.size` 比較 → `resize()`）を実装する
  - エラーハンドリング（`tracing::error` + スキップ）を実装する
  - _Requirements: Req 3 AC1-AC7_
  - _Dependencies: Task 2_

- [ ] 4. composite_render_system 実装
  - `compositor_systems.rs` に `composite_render_system` を追加する
  - `CompositeContext` 構造体（`dc: &ID2D1DeviceContext`, `accumulated_opacity: f32`）を定義する
  - per-window `WindowD3D11Compositor` のイテレーションを実装する
  - `render_subtree()` 再帰関数を実装する:
    - `CompositeContext` で DC + 累積透明度を親→子に伝搬
    - `Visual.is_visible == false` でサブツリースキップ
    - `accumulated_opacity * Visual.opacity` で opacity 手動累積（clamp [0.0, 1.0]）
    - **PushLayer は不使用**（中間サーフェス確保による負荷のため）
  - 合成描画ループを実装する:
    - composition_bitmap を DC に SetTarget
    - BeginDraw → Clear transparent
    - `render_subtree()` で再帰走査: SetTransform → draw_with_opacity(accumulated_opacity) → 子に伝搬
    - EndDraw
  - ダーティ判定（Changed<GraphicsCommandList/GlobalArrangement/Visual>）を実装する
  - CopyFromBitmap（composition → staging）を実装する
  - _Requirements: Req 2 AC1-AC10_
  - _Dependencies: Task 2, Task 3_

### Phase 1C: テスト・検証

- [ ] 5. ユニットテスト作成
  - `WindowD3D11Compositor` ライフサイクルテスト（new/resize/invalidate）を作成する
  - `CompositeContext` による opacity 手動累積テスト（parent×child, is_visible=false, clamp）を作成する
  - `transfer_to_hbitmap` のpitch/stride テストを作成する
  - _Requirements: Req 5 AC1, AC3, AC4_
  - _Dependencies: Task 1, Task 2, Task 3, Task 4_

- [ ] 6. 統合テスト・E2E検証
  - `composite_render_system` の z-order + transform + opacity 合成テストを作成する
  - `compositor_init_system` + `composite_render_system` 統合テストを作成する
  - デバイスロスト→再初期化の統合テストを作成する
  - `taffy_flex_demo` 相当の独立テスト環境を構築し、新パイプラインでの描画検証を行う
  - `cargo test` で全テスト（既存+新規）がパスすることを確認する
  - _Requirements: Req 5 AC2, AC5, AC6_
  - _Dependencies: Task 3, Task 4_

---

## 依存関係サマリー

```
Task 1 (com/ulw.rs) ──────┐
Task 2 (Compositor) ───────┼──→ Task 3 (init_system) ──→ Task 5 (Unit Tests) ──→ Task 6 (Integration)
                           └──→ Task 4 (render_system) ─┘
```

## 要件カバレッジサマリー

| 要件 (v2) | タスク | 備考 |
|-----------|--------|------|
| Req 1 (WindowD3D11Compositor) | 2 | コンポーネント定義 + Drop |
| Req 2 (composite_render_system) | 4 | 合成描画 + opacity 累積 + ダーティ判定 |
| Req 3 (compositor_init_system) | 3 | 初期化 + リサイズ + デバイスロスト |
| Req 4 (transfer_to_hbitmap) | 1 | D2D→HBITMAP 転送 |
| Req 5 (検証基準) | 5, 6 | Unit + Integration テスト |

全6タスクで全5要件をカバー。旧 Req 4-8 は v2 で Req 1-5 に統合済み。
