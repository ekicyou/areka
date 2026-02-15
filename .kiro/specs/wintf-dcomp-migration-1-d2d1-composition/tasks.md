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
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [ ] 2. WindowD3D11Compositor コンポーネント実装
  - `ecs/graphics/compositor.rs` を新規作成する
  - `WindowD3D11CompositorInner` 構造体を定義する（composition_bitmap, staging_bitmap, hbitmap, memory_dc, dib_bits, size）
  - `WindowD3D11Compositor` 構造体を定義する（inner: Option, generation: u32, SparseSet storage）
  - `new()`, `resize()`, `invalidate()`, `is_valid()`, アクセサメソッドを実装する
  - D2D1 Bitmap 作成（TARGET / CPU_READ）、CreateDIBSection、CreateCompatibleDC を実装する
  - `Drop` 実装で GDI リソースを解放する
  - `ecs/graphics/mod.rs` に `pub mod compositor;` を追加する
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

### Phase 1B: Layout層拡張

- [ ] 3. GlobalArrangement に global_opacity 追加
  - `GlobalArrangement` 構造体に `global_opacity: f32` フィールドを追加する（初期値 1.0）
  - `Default` 実装を更新する
  - `propagate_global_arrangements` に Opacity 累積ロジック（`parent.global_opacity * child.opacity`）を追加する
  - `Visual.is_visible == false` 時に `global_opacity = 0.0` を設定するロジックを追加する
  - `[0.0, 1.0]` 範囲クランプを適用する
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_
  - _Dependencies: None_

### Phase 1C: ECSシステム実装

- [ ] 4. compositor_init_system 実装
  - `ecs/graphics/compositor_systems.rs` を新規作成する
  - `Added<WindowHandle>` && `Without<WindowD3D11Compositor>` クエリで新規ウィンドウを検出する
  - `GraphicsCore` から DC を取得して `WindowD3D11Compositor::new()` を呼び出す
  - generation 不一致検出によるデバイスロスト再作成ロジックを実装する
  - リサイズ検出ロジック（WindowHandle サイズ変更 → `resize()`）を実装する
  - エラーハンドリング（`tracing::error` + スキップ）を実装する
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 7.1, 7.2, 7.3_
  - _Dependencies: Task 2_

- [ ] 5. composite_render_system 実装
  - `compositor_systems.rs` に `composite_render_system` を追加する
  - per-window `WindowD3D11Compositor` のイテレーションを実装する
  - `Children` 関係の depth-first pre-order 走査で z-order 描画順を決定する
  - 合成描画ループを実装する:
    - composition_bitmap を DC に SetTarget
    - BeginDraw → Clear transparent
    - 各エンティティ: SetTransform → PushLayer(opacity) → DrawImage(CommandList) → PopLayer
    - EndDraw
  - ダーティ判定（Changed<GraphicsCommandList/GlobalArrangement/Visual>）を実装する
  - CopyFromBitmap（composition → staging）を実装する
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_
  - _Dependencies: Task 2, Task 3_

### Phase 1D: テスト・検証

- [ ] 6. ユニットテスト作成
  - `WindowD3D11Compositor` ライフサイクルテスト（new/resize/invalidate）を作成する
  - `GlobalArrangement.global_opacity` 累積テスト（parent×child, is_visible=false, clamp）を作成する
  - `transfer_to_hbitmap` のpitch/stride テストを作成する
  - _Requirements: 8.1, 8.3_
  - _Dependencies: Task 1, Task 2, Task 3_

- [ ] 7. 統合テスト・E2E検証
  - `composite_render_system` の z-order + transform + opacity 合成テストを作成する
  - `compositor_init_system` + `composite_render_system` 統合テストを作成する
  - デバイスロスト→再初期化の統合テストを作成する
  - `taffy_flex_demo` 相当の独立テスト環境を構築し、新パイプラインでの描画検証を行う
  - `cargo test` で全テスト（既存+新規）がパスすることを確認する
  - _Requirements: 8.2, 8.4, 8.5_
  - _Dependencies: Task 4, Task 5_

---

## 依存関係サマリー

```
Task 1 (com/ulw.rs) ──────┐
Task 2 (Compositor) ───────┼──→ Task 4 (init_system) ──→ Task 6 (Unit Tests) ──→ Task 7 (Integration)
Task 3 (GlobalArrangement) ┘──→ Task 5 (render_system) ─┘
```

## 要件カバレッジサマリー

| 要件 | タスク |
|------|--------|
| Req 1 (WindowD3D11Compositor) | 2 |
| Req 2 (composite_render_system) | 5 |
| Req 3 (compositor_init_system) | 4 |
| Req 4 (Opacity累積) | 3 |
| Req 5 (D2D→HBITMAP転送) | 1 |
| Req 6 (リサイズ) | 2, 4 |
| Req 7 (デバイスロスト) | 4 |
| Req 8 (検証基準) | 6, 7 |

全8要件がタスクにマッピング済み。
