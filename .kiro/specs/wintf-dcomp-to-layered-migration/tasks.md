# 実装計画: wintf-dcomp-to-layered-migration

## タスクの進め方

本仕様は4フェーズの段階的移行戦略に従い、各フェーズが前フェーズの完了を前提とする。並列実行可能なタスクには`(P)`マーカーを付与している。

### Phase 1: D2D1合成スタック構築
DCompパイプラインを維持したまま、新しいD2D1合成システムを並行追加する。新システムはworld.rsに登録せず独立テストを行う。

### Phase 2: DCompパイプライン置換
ECSスケジュールをDCompシステムから新D2D1システムに切り替える。DCompコードは残存するが呼び出しはゼロになる。

### Phase 3: UpdateLayeredWindow統合
WS_EX_LAYEREDウィンドウスタイルを適用し、UpdateLayeredWindowによる描画およびalpha=0クリックスルーを実現する。

### Phase 4: DCompコード削除
DComp関連のコード、コンポーネント、システムをすべて削除し、最終テストを実施する。

---

## タスク一覧

### Phase 1: D2D1合成スタック構築

- [ ] 1. Phase 1: D2D1合成スタック並行構築

- [ ] 1.1 (P) GlobalArrangementにOpacity累積機能を追加
  - `ecs/layout/arrangement.rs`のGlobalArrangement structに`global_opacity: f32`フィールドを追加（Default実装で1.0を設定）
  - `ecs/layout/systems.rs`のPropagate実装にOpacity累積ロジックを追加（`parent.global_opacity * child.Visual.opacity`を計算、`is_visible == false`の場合は0.0）
  - Visual.opacityとGlobalArrangement.global_opacityの統合テスト（parent 0.8 × child 0.5 = 0.4を検証）
  - _Requirements: 3.6_

- [ ] 1.2 (P) WindowD3D11Compositorコンポーネントを実装
  - `ecs/graphics/compositor.rs`を新規作成し、WindowD3D11Compositor structを定義（ID2D1Bitmap1×2, HBITMAP, HDC, DIBSection pointer, size, generationを保持）
  - `new()`, `resize()`, `invalidate()`, `is_valid()`メソッドおよびリソースアクセサ（composition_bitmap, staging_bitmap, hbitmap, memory_dc, dib_bits）を実装
  - ID2D1Bitmap1作成（RENDER_TARGET用とCPU_READ用）、CreateDIBSection呼び出し、MemoryDC作成のリソースライフサイクルテスト
  - _Requirements: 3.1, 3.5, 6.1_

- [ ] 1.3 (P) UlwTransferユーティリティモジュールを実装
  - `com/ulw.rs`を新規作成し、`transfer_to_hbitmap()`関数を実装（staging bitmap Map → stride考慮の行単位memcpy → Unmap）
  - `present_layered_window()`関数を実装（BLENDFUNCTION構築 → UpdateLayeredWindow呼び出し、window_pos: Option対応）
  - pitch/strideが異なるケースでの正しいピクセルコピー検証（unit test）
  - _Requirements: 4.1, 4.2, 4.4_

- [ ] 1.4 compositor_init_systemを実装
  - `ecs/graphics/compositor_systems.rs`を新規作成し、compositor_init_system関数を実装（Added<WindowHandle> && Without<WindowD3D11Compositor>をクエリ）
  - GraphicsCoreからID2D1DeviceContextを取得し、WindowD3D11Compositor::new()を呼び出してエンティティに挿入
  - デバイスロスト後の再初期化フローテスト（generation不一致検出 → 再作成）
  - _Requirements: 3.1, 6.1_
  - _Dependencies: Task 1.2_

- [ ] 1.5 composite_render_systemを実装
  - `ecs/graphics/compositor_systems.rs`にcomposite_render_system関数を実装（per-windowのWindowD3D11Compositorをイテレート）
  - Children関係のdepth-first pre-order走査でz-order順ソート、GlobalArrangementのtransform/global_opacity適用、GraphicsCommandListをDrawImage
  - BeginDraw → Clear transparent → 各エンティティ描画 → EndDraw → CopyFromBitmap(staging)の完全フロー実装
  - Opacity適用方法の選択（PushLayerまたはDrawImage composite mode）、複数エンティティの正しい合成および座標変換の統合テスト
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Dependencies: Task 1.1, Task 1.2_

- [ ] 1.6* Phase 1統合テスト（新パイプライン単体検証）
  - taffy_flex_demo相当の描画が新パイプラインで動作することを検証（DCompパイプラインは並行稼働のまま、新システムは独立テスト環境で実行）
  - z-order描画順序の正確性、transform累積の正確性、opacity累積の正確性を視覚的に確認
  - _Requirements: 10.1, 10.2_
  - _Dependencies: Task 1.4, Task 1.5_

### Phase 2: DCompパイプライン置換

- [ ] 2. Phase 2: DCompパイプライン無効化とスケジュール切替

- [ ] 2.1 (P) GraphicsCoreからDComp初期化を除去
  - `ecs/graphics/core.rs`のGraphicsCore::new()からDCompositionCreateDevice3およびIDCompositionDesktopDevice/IDCompositionDevice3の作成ステップを削除
  - GraphicsCoreInner structからdesktop/dcompフィールドを削除、dcomp()/desktop()アクセサメソッドを削除
  - `com/dcomp.rs`へのuse依存を除去、invalidate()→再初期化フローの簡素化（DCompステップ省略）
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [ ] 2.2 ECSスケジュールを新パイプラインに切り替え
  - `ecs/world.rs`のSchedule定義からDComp関連システム12個（visual_resource_management, visual_hierarchy_sync, init_window_graphics, window_visual_integration, deferred_surface_creation, cleanup_surface, render_surface, visual_property_sync, commit_composition等）の登録を削除
  - 新システム（compositor_init_system, composite_render_system）をGraphicsSetup/RenderSurfaceステージに登録
  - invalidate_dependent_componentsのコンポーネント型参照を新コンポーネントに更新
  - _Requirements: 2.3, 3.3_
  - _Dependencies: Task 2.1_

- [ ] 2.3 (P) on_visual_addフックからDCompコンポーネント挿入を除去
  - `ecs/graphics/components.rs`のon_visual_add関数からVisualGraphics::default(), SurfaceGraphics::default(), SurfaceGraphicsDirty::default()の挿入を削除
  - Arrangement::default()とBrushInheritマーカーの挿入は維持
  - _Requirements: 6.2, 6.3_

- [ ] 2.4 Phase 2統合テスト（DComp呼び出しゼロ検証）
  - 全既存example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）がD2D1合成パイプラインで動作することを検証
  - DComp API呼び出しがゼロであることをgrep検証（IDComposition*型参照の検出）
  - cargo test全テストパス確認
  - _Requirements: 10.1, 10.2_
  - _Dependencies: Task 2.2, Task 2.3_

### Phase 3: UpdateLayeredWindow統合

- [ ] 3. Phase 3: ULW統合とクリックスルー実現

- [ ] 3.1 (P) ulw_present_systemを実装しスケジュールに登録
  - `ecs/graphics/compositor_systems.rs`にulw_present_system関数を実装（WindowD3D11Compositorをイテレート、staging bitmap Map → UlwTransfer::transfer_to_hbitmap → UlwTransfer::present_layered_window → Unmap）
  - UpdateLayeredWindow失敗時のエラーログ記録と次フレーム再試行ロジック実装
  - `ecs/world.rs`のCommitCompositionステージにulw_present_systemを登録
  - _Requirements: 4.1, 4.4, 4.5_
  - _Dependencies: Task 1.3_

- [ ] 3.2 (P) WS_EX_LAYEREDウィンドウスタイルに切り替え
  - `ecs/window.rs`のWindowStyle::default()の`ex_style`をWS_EX_LAYEREDに変更
  - `areka/src/main.rs`のShell/Balloon生成箇所でWS_EX_NOREDIRECTIONBITMAPをWS_EX_LAYEREDに変更
  - _Requirements: 4.2_

- [ ] 3.3 (P) WM_PAINTハンドラをULW方式に更新
  - `ecs/window_proc/handlers.rs`のWM_PAINTハンドラでBeginPaint/EndPaintの最小ペアのみ実行（描画処理はUpdateLayeredWindowに委ねる）
  - WM_ERASEBKGNDハンドラのコメント更新（DComp前提 → ULW方式）
  - WM_SIZEハンドラに合成ビットマップリサイズトリガー（WindowD3D11Compositor::resize()呼び出し）を追加
  - _Requirements: 7.1, 7.2, 7.3_

- [ ] 3.4 WS_EX_LAYERED環境でのWM_PAINT発火動作を検証
  - 最小構成のWS_EX_LAYEREDウィンドウを作成し、WM_PAINTハンドラでログ出力して発火動作を確認
  - 検証結果に基づき、handlers.rsのWM_PAINTハンドラ実装を確定（発火する場合: BeginPaint/EndPaintペア必須、発火しない場合: ハンドラ不要）
  - research.md § Research Neededの項目を検証結果で更新
  - _Requirements: 7.1, 7.3_
  - _Dependencies: Task 3.2_

- [ ] 3.5 Phase 3統合テスト（クリックスルー検証）
  - UpdateLayeredWindowでの透過ウィンドウ表示が動作すること確認
  - alpha=0ピクセル領域のクリックスルーが動作すること確認（マウスクリックが背後のウィンドウに到達）
  - WM_SIZE時の合成ビットマップリサイズが正常動作すること確認
  - ULW失敗時のログ出力および次フレーム再試行が動作すること確認
  - _Requirements: 4.3, 10.1, 10.2_
  - _Dependencies: Task 3.1, Task 3.2, Task 3.3, Task 3.4_

### Phase 4: DCompコード削除とクリーンアップ

- [ ] 4. Phase 4: DComp関連コード全削除と最終検証

- [ ] 4.1 (P) DCompコンポーネント定義を削除
  - `ecs/graphics/components.rs`からVisualGraphics, SurfaceGraphics, SurfaceGraphicsDirty, SurfaceCreationStats structを削除
  - DCompコンポーネント参照を含むテストを修正または削除
  - _Requirements: 6.3_

- [ ] 4.2 (P) DCompシステム関数を削除
  - `ecs/graphics/systems.rs`からvisual_resource_management, visual_hierarchy_sync, deferred_surface_creation, cleanup_surface, render_surface, visual_property_sync, commit_compositionなど12個のRED分類システム関数を削除
  - `ecs/graphics/visual_manager.rs`ファイル全体を削除（170行）
  - _Requirements: 6.4_

- [ ] 4.3 (P) com/dcomp.rsモジュールを削除
  - `com/dcomp.rs`ファイル全体を削除（315行）
  - `ecs/graphics/core.rs`および他のモジュールからDComp関連use文を最終クリーンアップ
  - _Requirements: 2.5_

- [ ] 4.4 (P) dcomp_demo.rsサンプルを削除
  - `examples/dcomp_demo.rs`ファイルを削除
  - _Requirements: 8.4_

- [ ] 4.5 Phase 4最終検証（DComp参照ゼロ確認）
  - cargo test全テストパス確認
  - cargo build --examples全ビルドパス確認（dcomp_demo.rs削除済み）
  - ECSコード内のIDComposition*型参照がゼロであることをgrep検証
  - `com/dcomp.rs`が削除されていることを確認
  - _Requirements: 2.5, 10.1, 10.2_
  - _Dependencies: Task 4.1, Task 4.2, Task 4.3, Task 4.4_

---

## 完了基準（Definition of Done）

各フェーズの完了基準は以下の通り：

### Phase 1
- WindowD3D11Compositor::new()がID2D1Bitmap1 + HBITMAPリソースを正しく作成
- composite_render_systemがGraphicsCommandListをz-order + transform + opacityで合成描画
- global_opacity累積がunit testでパス
- 新パイプライン単体での描画結果がtaffy_flex_demo相当と視覚的に一致

### Phase 2
- 全既存exampleがD2D1合成パイプラインで動作
- DComp API呼び出しがゼロであること（grep検証）
- cargo test全テストパス

### Phase 3
- UpdateLayeredWindowでの透過ウィンドウ表示が動作
- alpha=0ピクセル領域のクリックスルーが動作
- WM_SIZE時のリサイズが正常動作
- ULW失敗時のログ出力+次フレーム再試行が動作

### Phase 4
- cargo test全テストパス
- cargo build --examples全ビルドパス（dcomp_demo.rs削除済み）
- ECSコード内のIDComposition*型参照がゼロ
- com/dcomp.rsが削除されている

---

## 要件カバレッジ

全10要件をタスクでカバー：
- Req 1 (影響範囲特定): design.md/research.mdで定義済み
- Req 2 (段階的移行): Phase 1-4構成で実現
- Req 3 (D2D1合成): Task 1.1-1.6でカバー
- Req 4 (ULW統合): Task 3.1-3.5でカバー
- Req 5 (GraphicsCore簡素化): Task 2.1でカバー
- Req 6 (ECSコンポーネント再設計): Task 1.2, 2.3, 4.1でカバー
- Req 7 (メッセージハンドリング): Task 3.3, 3.4でカバー
- Req 8 (既存仕様影響): design.md Migration Strategyで定義、Task 4.4でdcomp_demo削除
- Req 9 (子仕様構成): 4フェーズ構成で実現
- Req 10 (テスト戦略): Task 1.6, 2.4, 3.5, 4.5でカバー
