# 実装計画: wintf-dcomp-migration-1-d2d1-composition

## タスク概要

Phase 1 — D2D1 合成スタック構築。DComp パイプラインを温存しながら、新しい D2D1 合成描画スタックを独立モジュールとして構築する。world.rs への登録は Phase 2 で行うため、本フェーズでは独立テスト可能な状態を目指す。

---

## 実装タスク

### Phase 1A: COM層・コンポーネント基盤

- [ ] 1. (P) transfer_to_hbitmap ユーティリティ実装
- [ ] 1.1 (P) com/ulw.rs モジュール作成と基本実装
  - `com/ulw.rs` を新規作成し、`transfer_to_hbitmap()` 関数を実装する
  - ステージング ID2D1Bitmap1 を `Map(D2D1_MAP_OPTIONS_READ)` でマップする
  - `D2D1_MAPPED_RECT` から pitch と bits を取得する
  - pitch と stride（`width * 4`）を比較し、一致時は `std::ptr::copy_nonoverlapping` で一括コピー、不一致時は行単位コピーを実装する
  - `Unmap()` でマッピングを解除する
  - Map 失敗時は `windows::core::Result::Err` を返す
  - `com/mod.rs` に `pub mod ulw;` を追加する
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 2. (P) WindowD3D11Compositor コンポーネント実装
- [ ] 2.1 (P) compositor.rs 作成とコンポーネント構造定義
  - `ecs/graphics/compositor.rs` を新規作成する
  - `WindowD3D11CompositorInner` 構造体を定義する（composition_bitmap, staging_bitmap, hbitmap, memory_dc, dib_bits フィールド）
  - `WindowD3D11Compositor` 構造体を定義する（inner: Option<Inner>, generation: u32, dirty: bool, cached_size: (u32, u32)）
  - `#[derive(Component)]` と `#[component(storage = "SparseSet")]` を適用する
  - `unsafe impl Send` と `unsafe impl Sync` を実装する
  - _Requirements: 1.1, 1.4_

- [ ] 2.2 (P) リソース作成とライフサイクル管理
  - `new(dc: &ID2D1DeviceContext, w: u32, h: u32)` を実装する:
    - composition_bitmap を `D2D1_BITMAP_OPTIONS_TARGET`、PBGRA32、96dpi で作成
    - staging_bitmap を `D2D1_BITMAP_OPTIONS_CPU_READ | CANNOT_DRAW`、PBGRA32、96dpi で作成
    - `CreateDIBSection` で top-down DIB（biHeight 負数）、PBGRA32、BI_RGB の HBITMAP を作成
    - `CreateCompatibleDC` で memory_dc を作成
    - `SelectObject(memory_dc, hbitmap)` で HBITMAP を DC に関連付ける
    - dib_bits ポインタを保存
    - generation を 0、dirty を false、cached_size を (w, h) で初期化
  - `resize(dc, w, h)` を実装する（全リソース再作成、generation インクリメント）
  - `invalidate()` を実装する（inner を None に設定）
  - `Drop` を実装する（`DeleteObject(hbitmap)`, `DeleteDC(memory_dc)` を呼び出す）
  - _Requirements: 1.1, 1.2, 1.3, 1.5_

- [ ] 2.3 (P) アクセサメソッドと状態管理
  - `is_valid()`, `composition_bitmap()`, `staging_bitmap()`, `hbitmap()`, `memory_dc()`, `dib_bits()` を実装する
  - `cached_size()`, `generation()` を実装する
  - `is_dirty()`, `set_dirty(v: bool)` を実装する
  - `ecs/graphics/mod.rs` に `pub mod compositor;` を追加する
  - _Requirements: 1.2, 1.6_

### Phase 1B: ECSシステム実装

- [ ] 3. compositor_init_system 実装
- [ ] 3.1 compositor_systems.rs 作成とシステム骨格
  - `ecs/graphics/compositor_systems.rs` を新規作成する
  - `compositor_init_system` 関数を定義する
  - `GraphicsCore` リソースから `device_context()` を取得し、None なら early return する
  - `Or<(Without<WindowD3D11Compositor>, Changed<HasGraphicsResources>)>` クエリを実装する
  - _Requirements: 3.2, 3.7_

- [ ] 3.2 新規ウィンドウとデバイスロスト復旧ロジック
  - `WindowPos.size` が None または幅/高さ 0 の場合はスキップする
  - `Option<WindowD3D11Compositor> = None` の場合、`WindowD3D11Compositor::new(dc, w, h)` を呼び出し、成功時は `commands.entity(entity).insert(compositor)` する
  - `Some(compositor)` + `!is_valid()` の場合、`new()` で再作成し、成功時は旧 generation を引き継ぎ increment する
  - 失敗時は `tracing::error!` でエラー出力し、`invalidate()` を呼び出す
  - _Requirements: 3.1, 3.4, 3.5, 3.6_

- [ ] 3.3 リサイズ検出と処理
  - `is_valid()` かつ `cached_size != (w, h)` の場合、`resize(dc, w, h)` を呼び出す
  - 失敗時は `tracing::error!` でエラー出力する（旧サイズ維持）
  - _Requirements: 3.3_

- [ ] 4. composite_render_system 実装
- [ ] 4.1 DcTargetGuard RAII 構造体実装
  - `compositor_systems.rs` に `DcTargetGuard` 構造体を定義する（dc: &ID2D1DeviceContext, prev_target: Option<ID2D1Image>）
  - `new(dc, new_target)` を実装する（`GetTarget()` で保存、`SetTarget(new_target)` で切替）
  - `Drop` を実装する（`SetTarget(prev_target.as_ref())` で復元）
  - _Requirements: 2.2, 2.7_

- [ ] 4.2 CompositeContext と render_subtree 骨格
  - `CompositeContext` 構造体を定義する（dc: &ID2D1DeviceContext, accumulated_opacity: f32）
  - `render_subtree(ctx, entity, query)` 関数を定義する
  - `Visual.is_visible == false` の場合、サブツリー全体をスキップする
  - `accumulated_opacity * Visual.clamped_opacity()` で opacity を累積計算し、[0.0, 1.0] に clamp する
  - `accumulated_opacity == 0.0` の場合、サブツリー全体をスキップする
  - _Requirements: 2.3, 2.4, 2.6_

- [ ] 4.3 draw_with_opacity 関数実装
  - `draw_with_opacity(dc, command_list, opacity) -> windows::core::Result<()>` を実装する
  - opacity == 1.0（f32::EPSILON 比較）の場合、`DrawImage` で直接描画する
  - opacity < 1.0 の場合:
    - `CreateEffect(&CLSID_D2D1ColorMatrix)` で ColorMatrix Effect を作成（失敗時は Err 返却）
    - `SetInput(0, command_list)` で入力設定
    - alpha チャネル乗算用の 5×4 行列を作成（M[3][3] = opacity）
    - `SetValue(0, &matrix)` でプロパティ設定（失敗時は Err 返却）
    - `GetOutput()` で出力取得（失敗時は Err 返却）
    - `DrawImage(&output)` で描画
  - `render_subtree` で Result を受け取り、失敗時は `tracing::error!` でログ出力して当該エンティティをスキップする
  - _Requirements: 2.5_

- [ ] 4.4 render_subtree でのトランスフォームと描画
  - `GlobalArrangement.transform` を `SetTransform` で DC に適用する
  - `GraphicsCommandList.command_list()` が Some の場合、`draw_with_opacity` を呼び出す
  - 子エンティティへの再帰（`CompositeContext` で accumulated_opacity を伝搬）を実装する
  - _Requirements: 2.1, 2.2_

- [ ] 4.5 composite_render_system メインループ実装
  - `compositor_query: Query<(Entity, &mut WindowD3D11Compositor, &Children)>` を定義する
  - `added_query: Query<Entity, Added<WindowD3D11Compositor>>` を定義する
  - `changed_query`, `children_query` を定義する
  - `is_window_dirty(window_entity, window_children, &changed_query, &children_query, &added_query)` 関数を実装する:
    - `added_query.contains(window_entity)` で初回フレーム検出
    - サブツリー内の `Changed<GraphicsCommandList/GlobalArrangement/Visual>` 検出
  - メインループで DC 取得、`is_valid()` チェック、`is_window_dirty()` 判定を実装する
  - _Requirements: 2.8, 2.9_

- [ ] 4.6 合成描画パイプライン実装
  - `DcTargetGuard::new(dc, composition_bitmap)` で DC ターゲット切替
  - `BeginDraw()` → `Clear(transparent)` を実装する
  - `CompositeContext { dc, accumulated_opacity: 1.0 }` を作成し、`Children` を depth-first pre-order で走査する
  - `EndDraw()` を呼び出す（ターゲット復元は `DcTargetGuard` の Drop で自動）
  - `staging.CopyFromBitmap(None, composition_bitmap, None)` を実装する
  - `transfer_to_hbitmap(staging, dib_bits, w, h)` を呼び出す
  - `compositor.set_dirty(true)` を設定する
  - _Requirements: 2.7, 2.10_

### Phase 1C: テスト・検証

- [ ] 5. ユニットテスト作成
- [ ] 5.1 WindowD3D11Compositor ライフサイクルテスト
  - `new()` が全4リソースを正しく作成することをテストする
  - `resize()` がリソースを再作成し、generation をインクリメントすることをテストする
  - `invalidate()` が `is_valid() == false` にすることをテストする
  - _Requirements: 5.1_

- [ ] 5.2 (P) CompositeContext opacity 累積テスト
  - parent opacity 0.8 × child opacity 0.5 = final 0.4 となることをテストする
  - `is_visible == false` でサブツリーがスキップされることをテストする
  - opacity が [0.0, 1.0] に clamp されることをテストする
  - _Requirements: 5.3_

- [ ] 5.3 (P) transfer_to_hbitmap 転送テスト
  - pitch == stride 時の一括コピーをテストする
  - pitch != stride 時の行単位コピーをテストする
  - Map 失敗時の Result::Err 返却をテストする
  - _Requirements: 5.4_

- [ ] 6. 統合テスト・E2E検証
- [ ] 6.1 composite_render_system 合成テスト
  - 複数 `GraphicsCommandList` を z-order + transform で正しく合成描画できることをテストする
  - opacity 累積が正確に実行されることをテストする
  - _Requirements: 5.2, 5.3_

- [ ] 6.2 システム統合テスト
  - `compositor_init_system` → `composite_render_system` パイプラインの統合動作をテストする
  - デバイスロスト → 再初期化 → 正常描画再開のフローをテストする
  - _Requirements: 5.2_

- [ ] 6.3* E2E描画検証とリグレッションテスト
  - `taffy_flex_demo` 相当の独立テスト環境を構築し、新パイプラインでの描画を検証する
  - `cargo test` で全テスト（既存+新規）がパスすることを確認する
  - `cargo build` で DComp パイプラインとの共存ビルドが成功することを確認する
  - _Requirements: 5.5, 5.6_

---

## 依存関係サマリー

```
Task 1 (transfer_to_hbitmap) (P) ──┐
Task 2 (WindowD3D11Compositor) (P) ─┼──→ Task 3 (compositor_init_system) ──┐
                                     │                                      │
                                     └──→ Task 4 (composite_render_system) ─┤
                                                                             ├──→ Task 5 (Unit Tests) ──→ Task 6 (Integration)
                                          (Task 4.1-4.6 は順次実行)          │
```

## 要件カバレッジサマリー

| 要件 (v2.1) | タスク | 備考 |
|-------------|--------|------|
| Req 1 (WindowD3D11Compositor) | 2.1, 2.2, 2.3 | コンポーネント定義 + リソース管理 + Drop |
| Req 2 (composite_render_system) | 4.1, 4.2, 4.3, 4.4, 4.5, 4.6 | RAII guard + 合成描画 + opacity 累積 + ダーティ判定 |
| Req 3 (compositor_init_system) | 3.1, 3.2, 3.3 | 初期化 + リサイズ + デバイスロスト |
| Req 4 (transfer_to_hbitmap) | 1.1 | D2D→HBITMAP 転送 |
| Req 5 (検証基準) | 5.1, 5.2, 5.3, 6.1, 6.2, 6.3 | Unit + Integration テスト |

全6メジャータスク、18サブタスクで全5要件の34 AC をカバー。Task 1 と Task 2 は並列実行可能。
