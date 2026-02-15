# 要件定義書: wintf-dcomp-to-layered-migration

## 導入

マウスクリックのクリックスルーがDirectComposition描画では実現不可能であることが判明し、デスクトップマスコット描画の根幹要件を満たせないと結論付けた。本仕様は描画パイプラインをDirectComposition（DComp）ベースからD2D1＋UpdateLayeredWindow（ULW）ベースへ全面移行するための**実装指針ドキュメント**および**段階的子仕様群**を策定する親仕様である。

### 移行の動機

- **致命的制約**: DComp描画ではクロスプロセスのクリックスルー（alpha=0ピクセルの透過クリック）が不可能
- **解決策**: `UpdateLayeredWindow()` + `WS_EX_LAYERED` + `ULW_ALPHA` 方式では、alpha=0ピクセルがOSレベルで自動的にクリックスルーとなる
- **副次効果**: `SetWindowRgn` ベースのクリックスルー仕様（wintf-P0-click-through-rgn）の大部分が不要化

### 本仕様のスコープ

本仕様（親仕様）は**実装そのものは行わない**。以下を成果物とする：

1. **実装指針ドキュメント**: 影響範囲、再利用可能資産、廃止対象、移行戦略の包括的定義
2. **段階的子仕様群**: フェーズ番号付きの子仕様ドキュメント（各子仕様は実装指針を参照）

### 現行パイプラインの構成

```
D3D11Device → DXGIDevice → D2D1Device → D2D1DeviceContext
                                       → DCompositionDesktopDevice → DCompositionDevice3
                                           ├─ IDCompositionTarget (per-window)
                                           ├─ IDCompositionVisual3 (per-entity)
                                           └─ IDCompositionSurface (per-entity)
```

### 目標パイプラインの構成

```
D3D11Device → DXGIDevice → D2D1Device → D2D1DeviceContext
                                       → ID2D1Bitmap1 (per-window合成ターゲット)
                                           └─ UpdateLayeredWindow(hwnd, ..., ULW_ALPHA)
```

## Project Description (Input)

DirectCompositionベース⇒UpdateLayeredWindowベースへ変更。マウスクリックのクリックスルーが出来ないことが判明し、DirectComposition描画ではデスクトップマスコット描画は不可能と結論付けた。そのため、描画をD3D11⇒D2D1⇒UpdateLayeredWindow()+WS_EX_LAYEREDレンダリングへと変更する。

順序としては、先にD3D11＋D2D1ベースの新しい合成スタックやシステムを作成し、DCompベースの合成パイプラインと同じ程度の実装が出来たことを確認してから、DCompパイプラインからD2D1パイプラインへ変更、最後にUpdateLayeredWindow()+WS_EX_LAYEREDレンダリングの実装を行う案がある。旧実装を参照しつつ、新しい実装を検討し、最後にまとめて削除するのが望ましい。本仕様設計段階で最適な置き換えプランを策定し、子仕様を決定する。

本仕様のゴールは実装指針ドキュメントの作成と、フェーズ番号を振った子仕様ドキュメントの作成とする。子仕様は実装指針ドキュメントを参照するように作成する。

---

## Requirements

### Requirement 1: 影響範囲の特定と分類

**Objective:** 開発者として、DComp依存コードと非依存コードを明確に分類し、移行の正確なスコープを把握したい。

#### Acceptance Criteria

1. The wintf crate shall DComp依存コードを以下の3カテゴリに分類する定義を含む：
   - **廃止対象（RED）**: DComp固有のコンポーネント・システム（IDCompositionTarget, IDCompositionVisual3, IDCompositionSurface, IDCompositionDevice3関連）
   - **書き換え対象（YELLOW）**: DComp前提で構築されているが、D2D1合成方式に置換可能なシステム（render_surface, commit_composition, visual_property_sync, deferred_surface_creation等）
   - **再利用可能（GREEN）**: DComp非依存のコンポーネント・システム（D2D1, DirectWrite, WIC, Layout, Widget, Input全般）

2. The wintf crate shall 以下のファイル群をDComp廃止対象として識別する：
   - `com/dcomp.rs` — DComp APIラッパー全体の廃止（315行）
   - `ecs/graphics/core.rs` — GraphicsCoreからDComp初期化を除去
   - `ecs/graphics/components.rs` — WindowGraphics, VisualGraphics, SurfaceGraphicsの全面置換
   - `ecs/graphics/systems.rs` — DComp合成・描画システムの置換
   - `ecs/graphics/visual_manager.rs` — Visual階層管理の廃止
   - `ecs/world.rs` — スケジュール定義の更新
   - `ecs/window.rs` — WS_EX_NOREDIRECTIONBITMAPからWS_EX_LAYEREDへの変更
   - `ecs/window_proc/handlers.rs` — WM_PAINT/WM_ERASEBKGNDハンドラの更新

3. The wintf crate shall 以下をDComp非依存の再利用可能資産として保証する：
   - `com/d2d/` モジュール全体（D2D APIラッパー）
   - `com/dwrite.rs`（DirectWrite）
   - `com/wic.rs`（Windows Imaging Component）
   - `com/d2d/command.rs`（D2Dコマンド抽象化、GraphicsCommandList生成）
   - `ecs/layout/` 全体（Taffyレイアウトエンジン統合）
   - `ecs/widget/` 全体（Label, Rectangle, BitmapSource等のウィジェット描画）
   - `ecs/pointer/`, `ecs/drag/`（入力システム全般）

### Requirement 2: 段階的移行戦略の定義

**Objective:** 開発者として、旧DCompパイプラインを参照しながら新パイプラインを段階的に構築し、安全に切り替えたい。

#### Acceptance Criteria

1. The 実装指針 shall 以下の4フェーズ段階的移行戦略を定義する：
   - **フェーズ1**: D2D1ベースの新合成スタック構築（DComp並行稼働、旧コード温存）
   - **フェーズ2**: DCompパイプラインからD2D1合成パイプラインへの切り替え（旧コード参照可能な状態で新パイプライン有効化）
   - **フェーズ3**: UpdateLayeredWindow統合（WS_EX_LAYERED適用、ULW呼出、クリックスルー検証）
   - **フェーズ4**: 旧DCompコード削除と最終クリーンアップ

2. When フェーズ1が完了した時, the 新パイプライン shall DCompパイプラインと同等の描画結果を達成する（矩形描画、テキスト描画、画像描画、透過表示の各機能）

3. When フェーズ2が完了した時, the wintf crate shall DCompベースの合成パイプラインを無効化し、D2D1合成パイプラインで全描画を実行する

4. When フェーズ3が完了した時, the wintf crate shall UpdateLayeredWindowによるウィンドウ更新を実装し、alpha=0ピクセルのクリックスルーが動作すること

5. When フェーズ4が完了した時, the wintf crate shall DComp関連コード（com/dcomp.rs、DCompコンポーネント、DCompシステム）をECSコードから完全に除去し、cargo test全テストがパスすること

### Requirement 3: 新描画パイプライン（D2D1合成方式）

**Objective:** 開発者として、DComp COMリソースをD2D1ベースに一新しつつ、Visual/Surface のComposition概念を継承した合成描画パイプラインが欲しい。

#### Acceptance Criteria

1. The 新パイプライン shall ウィンドウごとに1つの合成ビットマップ（ID2D1Bitmap1 or WIC Bitmap）を確保し、全ウィジェットのGraphicsCommandListを座標オフセット＋不透明度を適用しながら合成描画する

2. The 新パイプライン shall 既存のComposition概念（Visual階層、z-order、親子関係）をD2D1合成方式で引き継ぎ、DComp Visual階層同期システム（visual_hierarchy_sync_system）をD2D1ベースの合成描画ループに置換する

3. The 新パイプライン shall 以下のDCompスケジュールステージを置換する：
   - `PreLayout`のVisual作成 → 合成レイヤー管理
   - `GraphicsSetup`のWindowGraphics初期化 → 合成ビットマップ初期化
   - `Draw`のdeferred_surface_creation → 不要（合成ビットマップに直接描画）
   - `RenderSurface`のBeginDraw/EndDraw → D2D RenderTarget上への直接描画
   - `Composition`のvisual_property_sync → 合成描画時のtransformオフセット適用
   - `CommitComposition`のCommit → 合成ビットマップの確定

4. The 新パイプライン shall GraphicsCommandList（ID2D1CommandList）を生成するウィジェットシステム群を一切変更せずに再利用する

5. While ウィンドウサイズが変更された時, the 新パイプライン shall 合成ビットマップを適切にリサイズし、次フレームで正しい描画を行う

6. The 新パイプライン shall 合成描画時に親→子のOpacity階層累積を適用する（DComp方式でVisual.SetOpacity()が自動処理していた機能の自前実装）。具体的な累積方法（GlobalArrangement拡張 or 合成ループ内動的計算）は設計フェーズで確定する

### Requirement 4: UpdateLayeredWindow統合

**Objective:** 開発者として、合成ビットマップをUpdateLayeredWindowでウィンドウに転送し、alpha透過とクリックスルーを実現したい。

#### Acceptance Criteria

1. The wintf crate shall 合成ビットマップ（PARGB32形式）をHBITMAP/MemoryDC経由で`UpdateLayeredWindow(hwnd, hdcDst, &ptDst, &size, hdcSrc, &ptSrc, 0, &blend, ULW_ALPHA)`を呼び出してウィンドウ描画を行う

2. The wintf crate shall ウィンドウスタイルを`WS_EX_NOREDIRECTIONBITMAP`から`WS_EX_LAYERED`に変更する（`ecs/window.rs`のWindowStyle::default()および`areka/src/main.rs`）

3. When alpha=0のピクセルが描画された時, the OS shall 当該ピクセル領域をクリックスルーとして処理する（ULW_ALPHA方式の標準動作）

4. The wintf crate shall commit_compositionシステムを、IDCompositionDevice3::Commit()からUpdateLayeredWindow()呼び出しに置換する

5. If UpdateLayeredWindow呼び出しが失敗した場合, the wintf crate shall エラーをトレースログに記録し、次フレームで再試行する

### Requirement 5: GraphicsCore初期化の簡素化

**Objective:** 開発者として、DComp初期化を除去しD2D1デバイス中心のシンプルな初期化フローにしたい。

#### Acceptance Criteria

1. The GraphicsCore shall 初期化フローからDCompositionCreateDevice3およびIDCompositionDesktopDevice/IDCompositionDevice3の作成を除去する

2. The GraphicsCore shall 以下のデバイスチェーンを維持する：
   - D3D11CreateDevice → ID3D11Device
   - Cast → IDXGIDevice4
   - D2D1CreateFactory → ID2D1Factory
   - D2D1CreateDevice(dxgi) → ID2D1Device
   - CreateDeviceContext → ID2D1DeviceContext（共有）
   - DWriteCreateFactory → IDWriteFactory2

3. The GraphicsCore shall DComp関連フィールド（`desktop: IDCompositionDesktopDevice`, `dcomp: IDCompositionDevice3`）をstructから除去する

4. If デバイスロストが発生した場合, the GraphicsCore shall 既存のinvalidate()→再初期化フローを維持しつつ、DComp再初期化ステップを省略する

### Requirement 6: ECSコンポーネント再設計

**Objective:** 開発者として、論理コンポーネント（Visual概念）を継承しつつ、DComp COMリソースコンポーネントをD2D1合成方式に適したものに一新したい。

#### Acceptance Criteria

1. The wintf crate shall WindowGraphicsコンポーネントからIDCompositionTargetを除去し、以下に置換する：
   - 合成ビットマップ（ID2D1Bitmap1）: ウィンドウ全体の合成描画先
   - MemoryDC/HBITMAP: UpdateLayeredWindow転送用

2. The wintf crate shall Visual（論理コンポーネント）のComposition概念（階層、z-order、親子関係）を継承する。既存のVisualコンポーネントがそのまま利用可能かを設計フェーズで調査し、利用不可と判明した場合は仮名（Visual2等）で新コンポーネントを並行作成する

3. The wintf crate shall VisualGraphicsコンポーネント（IDCompositionVisual3保持）およびSurfaceGraphicsコンポーネント（IDCompositionSurface保持）を、D2D1合成方式に適した新リソースコンポーネントに一新する

4. The wintf crate shall visual_manager.rsのDComp固有リソース管理（IDCompositionVisual作成等）を、D2D1合成方式のリソース管理に置換する

5. The wintf crate shall コンポーネント命名規則（GPUリソースは`XxxGraphics`サフィックス）を維持し、新しいWindowGraphicsおよびリソースコンポーネントの設計に適用する

### Requirement 7: ウィンドウメッセージハンドリングの更新

**Objective:** 開発者として、ULW方式に適合したウィンドウメッセージ処理に変更したい。

#### Acceptance Criteria

1. The wintf crate shall WM_ERASEBKGND/WM_PAINTハンドラを、DComp前提の「何もしない」からULW方式に適した処理に更新する

2. The wintf crate shall WM_SIZEメッセージハンドラで合成ビットマップのリサイズをトリガーする

3. While WS_EX_LAYERED が設定されている時, the wintf crate shall WM_PAINTに対してBeginPaint/EndPaintの最小ペアのみを実行し、描画はUpdateLayeredWindowに委ねる

### Requirement 8: 既存仕様への影響評価

**Objective:** 開発者として、本移行が他の既存仕様に与える影響を明確にし、必要な調整を計画したい。

#### Acceptance Criteria

1. The 実装指針 shall wintf-P0-click-through-rgn仕様との関係を以下のように定義する：
   - 両仕様は**競争的並走**とする。click-through-rgn（SetWindowRgn+DComp方式）の実験結果が十分な性能を示した場合、本仕様（ULW移行）が凍結される可能性がある
   - 逆にclick-through-rgnが性能要件を満たせない場合、本仕様が優先され、click-through-rgn仕様の大部分はULW方式のalpha=0自動クリックスルーにより不要化する
   - 両仕様の実装は互いに依存せず、独立して進行可能とする

2. The 実装指針 shall wintf-P0-animation-system仕様への影響を評価する：
   - DComp Animation APIからの切り替え必要性の判断
   - dolaスケジュール駆動アニメーションとの統合方針

3. The 実装指針 shall wintf-P0-balloon-system仕様への影響を評価する：
   - バルーンウィンドウの描画パイプライン変更の影響

4. The 実装指針 shall dcomp_demo.rs（ECS非使用の独立DCompデモ）をフェーズ4（DCompコード削除・クリーンアップ）で削除対象とする

### Requirement 9: 子仕様の構成定義

**Objective:** 開発者として、移行作業をフェーズ番号付きの子仕様に分割し、依存関係と実装順序を明確にしたい。

#### Acceptance Criteria

1. The 実装指針 shall 以下の子仕様構成を定義する（最終的な子仕様構成は設計フェーズで確定）：
   - **子仕様1**: D2D1合成スタック構築（新GraphicsCore、合成ビットマップ、合成描画システム）
   - **子仕様2**: DCompパイプライン置換（ECSシステム切り替え、スケジュール更新）
   - **子仕様3**: UpdateLayeredWindow統合（WS_EX_LAYERED、ULW呼び出し、クリックスルー検証）
   - **子仕様4**: DCompコード削除と最終クリーンアップ

2. The 各子仕様 shall 実装指針ドキュメントを参照し、自仕様の担当範囲・依存する前提条件・完了基準を明記する

3. The 各子仕様 shall 前フェーズの完了を前提条件として記載し、段階的な検証が可能な構成とする

4. The 子仕様 shall DCompパイプラインとの並行稼働期間（フェーズ1〜2）を考慮し、旧コードを参照可能な状態で新実装を進められる設計とする

### Requirement 10: テスト・検証戦略

**Objective:** 開発者として、各フェーズで移行の正しさを検証できるテスト戦略を持ちたい。

#### Acceptance Criteria

1. The 各子仕様 shall 以下の検証基準を含む：
   - フェーズ1: 新パイプラインでtaffy_flex_demo相当の描画が動作すること
   - フェーズ2: 全既存exampleが新パイプラインで動作すること
   - フェーズ3: UpdateLayeredWindowでの透過表示＋クリックスルーが動作すること
   - フェーズ4: `cargo test` 全テストパス＋DComp参照がECSコードから除去されていること

2. The 実装指針 shall 各フェーズの完了基準（Definition of Done）を明記する

3. If 新パイプラインでの描画品質がDComp方式と異なる場合, the 実装指針 shall 許容範囲と対処方針を定義する
