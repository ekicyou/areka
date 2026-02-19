# 実装計画

## タスク一覧

- [ ] 1. `CompositionMode` enum と `Window` 構造体フィールドの基盤実装
- [ ] 1.1 `CompositionMode` enum の定義と公開
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]` で `ULW`（デフォルト）/ `DComp` の2バリアントを定義
  - `#[default] ULW` により `Default::default()` が `ULW` を返すことを保証
  - `window.rs` 内（または `window_types.rs` 等の既存モジュール）に配置
  - `pub use` で crate 外から参照可能にする
  - _Requirements: 1.1, 1.3_
- [ ] 1.2 `Window` 構造体への `composition_mode` プライベートフィールド追加
  - `Window { title, parent, composition_mode: CompositionMode }` — フィールドはプライベート
  - `pub fn composition_mode(&self) -> CompositionMode` ゲッターのみ公開
  - `impl Default for Window` で `composition_mode: CompositionMode::default()` を設定
  - 既存の `Window { title: ..., parent: None }` 構文で破壊的変更が発生するため、既存の全呼び出し箇所を `Window { ..Default::default() }` 構文か明示フィールド指定に更新
  - _Requirements: 1.2, 1.3_

- [ ] 2. `DCompGraphicsResource` ECS リソースの実装
- [ ] 2.1 (P) `DCompGraphicsResource` の構造体定義とリソース登録
  - `DCompGraphicsResourceInner { desktop: IDCompositionDesktopDevice, dcomp: IDCompositionDevice3 }` を定義
  - `#[derive(Resource)] pub struct DCompGraphicsResource { inner: Option<DCompGraphicsResourceInner> }` を定義
  - `Option<Inner>` パターンで `is_valid()` → `inner.is_some()` を実装
  - `dcomp() -> Option<&IDCompositionDevice3>` / `desktop() -> Option<&IDCompositionDesktopDevice>` アクセサ実装
  - _Requirements: 4.1, 4.3, 4.4_
- [ ] 2.2 (P) `DCompGraphicsResource::new()` と `invalidate()` の実装
  - `pub fn new(d2d_device: &ID2D1Device) -> windows::core::Result<Self>` — `IDCompositionDesktopDevice` → `IDCompositionDevice3` の順で COM 初期化
  - `pub fn invalidate(&mut self)` — `inner = None` で全 DComp COM オブジェクトをドロップ
  - COM 初期化失敗時は `windows::core::Result` のエラーを呼び出し元に伝播（ログは呼び出し側）
  - _Requirements: 4.1, 4.2, 4.5_

- [ ] 3. (P) `GraphicsCore` から DComp フィールドを除去し共通基盤に純化
  - `GraphicsCoreInner` から `desktop: IDCompositionDesktopDevice` と `dcomp: IDCompositionDevice3` を削除
  - `GraphicsCore::new()` の DComp 初期化ステップ（ステップ 7, 8）を除去
  - `GraphicsCore::dcomp()` / `GraphicsCore::desktop()` アクセサを削除
  - `GraphicsCore::invalidate()` は変更なし（共通リソースの無効化のみ継続）
  - `Res<GraphicsCore>` を参照する DComp システムは `Option<Res<DCompGraphicsResource>>` を別途取得する構造に変更（後続タスク対応）
  - _Requirements: 4.3, 6.5_

- [ ] 4. (P) `on_visual_add` フックへの DComp コンポーネント条件付き挿入
- [ ] 4.1 (P) `find_owner_window_composition_mode` ヘルパー関数の実装
  - `fn find_owner_window_composition_mode(world: &DeferredWorld, entity: Entity) -> Option<CompositionMode>` を実装
  - エンティティ自身が `Window` の場合は即座に `Some(w.composition_mode())` を返す
  - `ChildOf` チェーンを辿って祖先 `Window` を探す（既存 `find_owner_window` と同等ロジック）
  - 祖先 `Window` が見つからない場合（orphan Visual）は `None`
  - _Requirements: 2.2, 3.3_
- [ ] 4.2 (P) `on_visual_add` における DComp コンポーネント挿入ロジックの追加
  - `find_owner_window_composition_mode` で `CompositionMode::DComp` を確認した場合のみ `VisualGraphics::default()`, `SurfaceGraphics::default()`, `SurfaceGraphicsDirty::default()` を `world.commands()` で挿入
  - `CompositionMode::ULW` または `None`（orphan）の場合は挿入せず既存動作を維持（後方互換性）
  - _Requirements: 2.2, 3.3_

- [ ] 5. (P) `compositor_init_system` の ULW モード限定化
  - クエリに `&Window`（または `&CompositionMode` に相当するゲッター経由参照）を追加するか、イテレーション内で `window.composition_mode() == CompositionMode::ULW` チェックを追加
  - `CompositionMode::DComp` の Window はスキップ（`continue`）する
  - 既存の `Or<(Without<WindowD3D11Compositor>, Changed<HasGraphicsResources>, Changed<WindowPos>)>` フィルタはそのまま維持
  - _Requirements: 2.1_

- [ ] 6. `init_window_graphics` の DComp 遅延初期化対応
- [ ] 6.1 `init_window_graphics` への `DCompGraphicsResource` 遅延初期化ロジック追加
  - システムパラメータに `Option<ResMut<DCompGraphicsResource>>` を追加
  - DComp モードの Window を検出し、かつ `DCompGraphicsResource` がまだ存在しない（`dcomp_res.is_none()` または `!dcomp_res.as_ref().map_or(false, |r| r.is_valid())`）場合に `DCompGraphicsResource::new(gc.d2d_device())` を呼び出す
  - 成功時は `commands.insert_resource(new_resource)` で ECS World に登録
  - 失敗時は `tracing::error!` ログを出力してフレームスキップ（次フレームで再試行）
  - _Requirements: 4.2, 7.1_
- [ ] 6.2 `init_window_graphics` への `CompositionMode::DComp` ランタイムフィルタ追加
  - クエリに `&Window` を含め、`window.composition_mode() == CompositionMode::DComp` の場合のみ処理する
  - ULW モードの Window は `WindowGraphics` が未挿入でもクエリにヒットしうるため、ランタイムチェックで除外
  - `DCompGraphicsResource.desktop()` を `IDCompositionTarget` 作成に使用
  - _Requirements: 2.2, 4.2_

- [ ] 7. (P) `invalidate_dependent_components` への `DCompGraphicsResource` 無効化連動
  - システムパラメータに `Option<ResMut<DCompGraphicsResource>>` を追加
  - `!gc.is_valid()` ブランチ内で `if let Some(ref mut dcr) = dcomp_resource { dcr.invalidate(); }` を追加
  - `WindowD3D11Compositor` / `BitmapSourceGraphics` と同じパターンで実装（既存コードの追加のみ、既存行は変更なし）
  - _Requirements: 4.5_

- [ ] 8. ウィンドウ生成・WndProc の `CompositionMode` 連動
- [ ] 8.1 (P) `create_windows` システムの `CompositionMode` 連動ウィンドウスタイル実装
  - `create_windows` クエリに `&Window`（`composition_mode()` ゲッター経由）を追加
  - `CompositionMode::ULW` → `WS_EX_LAYERED` を適用（現状維持）
  - `CompositionMode::DComp` → `WS_EX_NOREDIRECTIONBITMAP` を適用（`WS_EX_LAYERED` は付与しない）
  - `WindowStyle` コンポーネントとの整合性を確認
  - _Requirements: 5.1, 5.2, 5.3_
- [ ] 8.2 (P) `handlers.rs` の WM_PAINT `CompositionMode` 分岐実装
  - `hwnd_to_entity` で Entity を解決し `world.get::<Window>(entity).map(|w| w.composition_mode())` で取得
  - `CompositionMode::DComp` の場合は `DefWindowProcW` に委譲（DComp は OS 管理）
  - `CompositionMode::ULW` または取得失敗の場合は既存の `BeginPaint` / `EndPaint` 最小ペアを維持（フォールバック）
  - WM_ERASEBKGND / WM_WINDOWPOSCHANGED は両モード共通動作のため変更不要
  - _Requirements: 5.4_

- [ ] 9. `world.rs` への DComp システムスケジュール再登録
- [ ] 9.1 5ステージへの DComp システム追加登録
  - `GraphicsSetup`: `init_window_graphics` を `compositor_init_system` の**前**に追加
  - `PreRenderSurface`: `visual_resource_management_system`, `deferred_surface_creation_system`, `mark_dirty_surfaces`, `cleanup_surface_on_commandlist_removed`, `window_visual_integration_system` を追加
  - `RenderSurface`: `render_surface` を追加
  - `Composition`: `visual_hierarchy_sync_system` → `visual_property_sync_system` を `composite_render_system` の前に追加
  - `CommitComposition`: `commit_composition` を追加（`ulw_present_system` と独立）
  - _Requirements: 3.1, 3.4_
- [ ] 9.2 既存 ULW スケジュール順序・依存関係の保持確認
  - 既存 ULW システム（`compositor_init_system`, `composite_render_system`, `ulw_present_system`）のステージ登録順を変更しない
  - DComp システム追加後も `cargo build` でコンパイルが通ることを確認
  - スケジュール内に不要な同期ポイント（`before`/`after` 制約の過剰追加）が発生しないよう確認
  - _Requirements: 3.2, 6.6_

- [ ] 10. `cargo test` 全パスと既存後方互換性の確認
- [ ] 10.1 `Window` 構造体変更に伴うコンパイルエラーの修正
  - `Window { title: ..., parent: ... }` の既存構文が `composition_mode` フィールド追加により失敗する箇所を `..Default::default()` 構文で修正
  - テストコード内の `Window` インスタンス生成を含む全ファイルを確認・修正
  - `GraphicsCore` からのアクセサ（`dcomp()`, `desktop()`）削除に伴う呼び出し側コードを `DCompGraphicsResource` 参照に切り替え
  - _Requirements: 9.1_
- [ ] 10.2 既存 ULW パイプライン後方互換テスト実行
  - `cargo test` を実行し、全テストがパスすることを確認
  - `CompositionMode::ULW`（デフォルト）の動作が変更されていないことを確認
  - _Requirements: 9.1, 9.2_

- [ ] 11. DComp パイプライン向けユニットテストの実装
- [ ] 11.1 (P) `CompositionMode` と `Window` の単体テスト
  - `CompositionMode::default() == CompositionMode::ULW` を検証するテストを追加
  - `Window::default().composition_mode() == CompositionMode::ULW` を検証
  - `Window { composition_mode: CompositionMode::DComp, ..Default::default() }.composition_mode() == CompositionMode::DComp` を検証
  - _Requirements: 1.1, 1.3_
- [ ] 11.2 (P) `find_owner_window_composition_mode` の単体テスト
  - Window エンティティ自身に対して正しいモードを返すケース
  - 子 Visual エンティティから祖先 Window を辿るケース
  - orphan Visual（Window 祖先なし）が `None` を返すケース
  - _Requirements: 2.2_
- [ ] 11.3 (P) `DCompGraphicsResource` の状態遷移テスト（ユニット）
  - `DCompGraphicsResource { inner: None }` の初期状態で `is_valid() == false` を検証
  - `invalidate()` 後に `is_valid() == false` かつアクセサが `None` を返すことを検証（GPU 不要のモック構造体テスト）
  - _Requirements: 4.1, 4.4, 4.5_

- [ ] 12. DComp パイプライン向けインテグレーションテストの実装
- [ ] 12.1 コンポーネント自動挿入の統合テスト
  - ULW Window 生成 → `WindowD3D11Compositor` が挿入され `WindowGraphics` は挿入されないことを検証
  - DComp Window 生成 → `WindowGraphics` が挿入され `WindowD3D11Compositor` は挿入されないことを検証
  - DComp Window 配下の Visual 生成 → `VisualGraphics` + `SurfaceGraphics` + `SurfaceGraphicsDirty` が挿入されること
  - ULW Window 配下の Visual 生成 → 上記 DComp コンポーネントが挿入されないこと
  - _Requirements: 2.1, 2.2, 2.4, 8.1, 8.2_
- [ ] 12.2* `GraphicsCore.invalidate()` → `DCompGraphicsResource` 連動無効化の統合テスト
  - `GraphicsCore` を無効化後、`invalidate_dependent_components` 実行で `DCompGraphicsResource.is_valid() == false` になることを検証
  - 既存の `WindowD3D11Compositor` 連動テストと同パターンで実装
  - _Requirements: 4.5_

- [ ] 13. `dcomp_taffy_demo` サンプルの実装
  - 既存 `taffy_flex_demo.rs` を参考に `CompositionMode::DComp` を指定した Window で taffy レイアウトを描画するサンプルを新規作成
  - DComp パイプライン（`WindowGraphics` → `VisualGraphics` → `SurfaceGraphics`）を通じた描画が正常に行われることを目視確認
  - Rectangle / Label / BitmapSource の表示をカバーし、DComp Surface への正常描画を検証
  - `Cargo.toml` に `[[example]]` エントリを追加
  - _Requirements: 7.2, 7.4, 9.3_

- [ ] 14. `multi_backend_demo` サンプルの実装
  - `CompositionMode::ULW` ウィンドウと `CompositionMode::DComp` ウィンドウを1つの ECS World に同時 spawn するサンプルを新規作成
  - ULW ウィンドウ: 透過クリックスルー + マスコット系ウィジェット表示
  - DComp ウィンドウ: 通常 UI ウィジェット（ボタン/テキスト等）表示
  - 各々が独立して描画され、一方の操作が他方に影響しないことを目視確認
  - `Cargo.toml` に `[[example]]` エントリを追加
  - _Requirements: 8.2, 8.3, 9.4_
