# 調査・設計判断ログ: wintf-dcomp-to-wuc-migration

> 本書は gap 分析（後半「§ ギャップ分析」に原文保持）に加え、design フェーズの discovery 発見と設計判断を記録する。
> 言語: ja（spec.json）。更新日: 2026-07-01（design フェーズ追記）。

## Summary（design 追記）

- **Feature**: `wintf-dcomp-to-wuc-migration`
- **Discovery Scope**: Extension（既存 wintf 表示バックエンドの純粋等価移行 / light discovery）
- **Key Findings**:
  1. `windows` 0.62.2 のディスク上 crate ソースで WUC 型・interop trait の実 API 形状を実確認（下記 §A）。全必須型・全メソッドが存在。numerics は別 crate `windows-numerics` 0.3 に分離（`Foundation_Numerics` feature 経由ではない）。
  2. サーフェス束ね方が唯一の構造変化: DComp `visual.SetContent(surface)`（直付け）→ WUC `sprite_visual.SetBrush(CreateSurfaceBrushWithSurface(surface))`（brush 一段挟む）。生成・解除の両経路（surface.rs L174 / L259）に波及。
  3. clip の `RoundedRectangleIndividual`（4 角独立半径）は WUC `CompositionRoundedRectangleGeometry`（`CornerRadius` 単一 Vector2）に 1:1 が無い。→ 個別半径は `CompositionPathGeometry`（カスタムパス）で等価写像する設計判断で解決（下記 §B-Decision-7）。areka 本体での個別半径構築は無く example/`clip_sync`/ULW guard のみ利用のため、ビルド・挙動等価目的で写像を実装する（要件破綻ではない）。
  4. DispatcherQueue は `DQTYPE_THREAD_CURRENT` で既存 pump に相乗り（公式 Win32 チュートリアル実証）。pump 非差し替えの要件 3.2 が成立。
  5. サーフェス D2D 層のビット等価キャプチャは既存 `com/wic.rs`（`IWICBitmapSource::CopyPixels`）＋ D2D の WIC render target で自己完結可能（R8.6 の主受け入れ手段）。合成層は Desktop Duplication を採る（R8.7・下記 §B-Decision-8）。

---

## A. `windows` 0.62.2 WUC 実 API 確認（crate ソース実読）

**ソース**: `C:\Users\maz-o\.cargo\registry\src\index.crates.io-6f17d22bba15001f\windows-0.62.2\`、numerics は `windows-numerics-0.3.0`。

| グループ | Rust パス | 主要シグネチャ（抜粋） | gating feature |
|---|---|---|---|
| Compositor | `windows::UI::Composition::Compositor` | `new()`, `CreateContainerVisual()`, `CreateSpriteVisual()`, `CreateSurfaceBrushWithSurface(P0: Param<ICompositionSurface>)`, `CreateInsetClip()`, `CreateRoundedRectangleGeometry()`, `CreateGeometricClipWithGeometry(P0: Param<CompositionGeometry>)`, `CreatePathGeometry(...)` | `UI_Composition` |
| DesktopWindowTarget | `windows::UI::Composition::Desktop::DesktopWindowTarget` | `SetRoot(P0: Param<Visual>)`, `Root()`, `IsTopmost()` | `UI_Composition_Desktop` |
| GraphicsDevice | `windows::UI::Composition::CompositionGraphicsDevice` | `CreateDrawingSurface(size: Foundation::Size, fmt: DirectXPixelFormat, alpha: DirectXAlphaMode)` | `UI_Composition` |
| Visual/Sprite/Container | `windows::UI::Composition::{Visual, ContainerVisual, SpriteVisual, VisualCollection, CompositionSurfaceBrush}` | `Visual::SetOffset(Vector3)`, `SetOpacity(f32)`, `SetSize(Vector2)`, `SetClip(P0: Param<CompositionClip>)`; `ContainerVisual::Children()->VisualCollection`; `VisualCollection::{InsertAtTop, InsertAtBottom, InsertAbove, InsertBelow, Remove, RemoveAll, Count}`; `SpriteVisual::SetBrush(P0: Param<CompositionBrush>)` | `UI_Composition` |
| Interop | `windows::Win32::System::WinRT::Composition::{ICompositorInterop, ICompositorDesktopInterop, ICompositionDrawingSurfaceInterop, ICompositionGraphicsDeviceInterop}` | `ICompositorInterop::CreateGraphicsDevice(P0: Param<IUnknown>)` [pass ID2D1Device]; `ICompositorDesktopInterop::CreateDesktopWindowTarget(hwnd: HWND, istopmost: BOOL)`; `ICompositionDrawingSurfaceInterop::BeginDraw(updaterect: *const RECT, iid: *const GUID, updateobject: *mut *mut c_void, updateoffset: *mut POINT)` / `EndDraw()` / `Resize(POINT)` | `Win32_System_WinRT_Composition` |
| DispatcherQueue | `windows::Win32::System::WinRT::{CreateDispatcherQueueController, DispatcherQueueOptions, DISPATCHERQUEUE_THREAD_TYPE, DISPATCHERQUEUE_THREAD_APARTMENTTYPE}` ＋ `windows::System::DispatcherQueueController` | `CreateDispatcherQueueController(options, *mut Option<DispatcherQueueController>)`; `DispatcherQueueOptions{ dwSize:u32, threadType, apartmentType }`; `DQTYPE_THREAD_CURRENT=2`, `DQTAT_COM_NONE=0`/`DQTAT_COM_ASTA=1`; `DispatcherQueueController::ShutdownQueueAsync()->IAsyncAction` | `Win32_System_WinRT`（関数・options）＋`System`（WinRT controller） |
| Numerics | `windows_numerics::{Vector2, Vector3}`（別 crate 0.3） | `Vector3{X,Y,Z:f32}`, `Vector2{X,Y:f32}`。**`Foundation_Numerics` feature からは供給されない** | crate `windows-numerics = "0.3"` を追加 |

**pixel format 写像**: `DXGI_FORMAT_B8G8R8A8_UNORM` → `DirectXPixelFormat::B8G8R8A8UIntNormalized`、`DXGI_ALPHA_MODE_PREMULTIPLIED` → `DirectXAlphaMode::Premultiplied`（`Graphics_DirectX` feature）。

**BeginDraw の要点**: DComp と同じ atlas offset 意味論（`updateoffset` を D2D の `SetTransform` M31/M32 に反映する既存ロジックがそのまま流用可）。`iid=&ID2D1DeviceContext::IID` を渡し `updateobject` に返る raw ポインタを `com/wuc.rs` の wrapper が cast する。返る D2D DC は `CreateGraphicsDevice` に渡した既存 `GraphicsCore.d2d` 由来＝デバイス共有。

**最終 features（ルート Cargo.toml `windows` に追加）**: `UI_Composition`, `UI_Composition_Desktop`, `Win32_System_WinRT`, `Win32_System_WinRT_Composition`, `System`, `Foundation`, `Graphics_DirectX`。加えて `windows-numerics = "0.3"` を dependency に追加。既存 `Win32_System_Com`/`Win32_Foundation`/`Win32_Graphics_Direct2D` 系は流用。

---

## B. 設計判断（design.md へ反映）

### Decision-1: 実装アプローチ = Option C（混成）
- **Context**: 純粋等価移行。schedule 構造と消費側改修を最小化しつつ、スパイク（R1）を段階分離したい。
- **Alternatives**: A（in-place 全差し替え）、B（WUC 専用コンポーネント新設・DComp 並存）、C（Ext/Resource 新設＋コンポーネント内部型 in-place）。
- **Selected**: C。新規は `com/wuc.rs`（interop Ext）と `WucGraphicsResource`（Compositor＋CompositionGraphicsDevice＋DispatcherQueueController 保持）に限定。既存 `WindowGraphics`/`VisualGraphics`/`SurfaceGraphics` の内部保持型のみ WUC 型へ差し替え、コンポーネント名・アクセサ形は維持。schedule 構造据え置き（`commit_composition` のみ除去）。
- **Rationale**: 消費 6 システムをアクセサ経由の最小改修に抑え、等価性を守りやすい。DComp 二重化（B）の冗長・schedule 複雑化を回避。
- **Trade-offs**: 層の順序依存（features→スパイク→device→target→tree→surface→frame）を計画で管理する必要。

### Decision-2: デバイス層 — Compositor ＋ CompositionGraphicsDevice
- `Compositor::new()` → `cast::<ICompositorInterop>()` → `CreateGraphicsDevice(graphics.d2d_device())` → `CompositionGraphicsDevice`。`WucGraphicsResource`（lazy 単一）に両者を保持。DComp の `IDCompositionDesktopDevice`/`Device3` は廃止。lazy-init・単一インスタンスのライフサイクル方針は現行踏襲（要件 2.2）。

### Decision-3: DispatcherQueue — DQTYPE_THREAD_CURRENT で pump 相乗り
- `Compositor` 生成前に `CreateDispatcherQueueController(DispatcherQueueOptions{ dwSize, DQTYPE_THREAD_CURRENT, DQTAT_COM_* })`。controller は Compositor より長寿命に `WucGraphicsResource` へ保持し、終了時 `ShutdownQueueAsync` でドレイン（要件 3.3）。既存 `wintf-winmsg-executor` の `GetMessage`/`DispatchMessage` pump を差し替えない（要件 3.2）。
- **apartment 種別の確定**: areka の現状 COM 初期化状況（`CoInitializeEx`/`RoInitialize` の有無・STA/MTA）に依存。既に STA 初期化済みなら `DQTAT_COM_NONE`、未初期化なら `DQTAT_COM_ASTA`。**R1 スパイクで実測して確定**（design では選択規則を提示、値は spike が決める）。

### Decision-4: ターゲット束縛 — DesktopWindowTarget
- `Compositor.cast::<ICompositorDesktopInterop>()::CreateDesktopWindowTarget(hwnd, istopmost)` → `DesktopWindowTarget`。`WindowGraphics` の内部 `IDCompositionTarget` を `DesktopWindowTarget` へ差し替え。root 束縛は `target.SetRoot(root_visual)`（`window_visual_integration_system` の `SetRoot` を WUC 型で維持）。HWND・ライフサイクル対応は不変（要件 4.2）。

### Decision-5: ビジュアル木 — Container/Sprite ＋ VisualCollection
- 生成: `CreateContainerVisual`/`CreateSpriteVisual`（surface を持つ描画対象は Sprite、純コンテナは Container）。
- z 順写像: 現行は `remove_all_visuals()` → Children 順に `add_visual(child,false,None)`。WUC 等価は `Children().RemoveAll()` → Children 順に `InsertAtTop(child)`（逐次 InsertAtTop で反復順＝最終 z 順が一致）。offset は `SetOffset(Vector3{x,y,0})`、opacity は `SetOpacity(f32)`（要件 5.2/5.3）。
- **注意**: DComp `SetOffsetX2/Y2` は個別軸 setter だったが WUC は `SetOffset(Vector3)` の一括。既存 `visual_property_sync_system` が両軸を同時計算しているため写像は自然。

### Decision-6: サーフェス — CompositionDrawingSurface ＋ SurfaceBrush（構造変化）
- 生成: `CompositionGraphicsDevice::CreateDrawingSurface(Size, B8G8R8A8UIntNormalized, Premultiplied)`。
- 描画: `cast::<ICompositionDrawingSurfaceInterop>()::BeginDraw(null_rect, &ID2D1DeviceContext::IID, &out_dc, &out_offset)` → D2D DC＋offset → 既存の `SetTransform`/`Clear`/`DrawImage`/`EndDraw` をそのまま（要件 6.2・offset 適用ロジック流用）。
- **束ね方（唯一の構造変化）**: `visual.SetContent(surface)` → `sprite_visual.SetBrush(compositor.CreateSurfaceBrushWithSurface(surface))`（要件 6.3）。解除は `sprite_visual.SetBrush(None)`。`SurfaceGraphics` に `CompositionSurfaceBrush` 保持を追加（brush ライフタイム管理）。波及: `deferred_surface_creation_system`（生成＋束ね）・`cleanup_surface_on_commandlist_removed`（解除）。B8G8R8A8/PREMUL は WUC でも同指定で画素等価（要件 6.4）。swapchain 経路は非採用（要件 6.5）。

### Decision-7: clip 等価写像（3 変種）
- **Rectangle**（半径 0）→ `Compositor.CreateInsetClip()`（inset 0）を rect 範囲へ。角丸なしゆえ inset で等価。
- **RoundedRectangle { radius }**（全角統一）→ `CreateRoundedRectangleGeometry()`（`SetCornerRadius(Vector2{r,r})`＋`SetSize`）→ `CreateGeometricClipWithGeometry(geometry)`。
- **RoundedRectangleIndividual**（4 角独立）→ WUC に単一 clip 型の直接等価が無い。`Compositor.CreatePathGeometry(path)` で角ごとの弧を組んだ `CompositionPath` を構築 → `CreateGeometricClipWithGeometry`。areka 本体は個別半径を構築しないが `clip_sync.rs` が enum 全変種を扱うため、ビルド・挙動等価目的で写像を実装（要件 5.4・9.4「既存機能の等価移行」に含む・新能力ではない）。
- DPI スケール（`scale_x`/`scale_y` を半径・矩形へ乗算）は現行 `clip_sync_system` と同一計算を WUC 側で維持（要件 5.4）。
- `SetClip(clip)` / `SetClip(None)`（clear）を WUC `Visual::SetClip` で維持。

### Decision-8: フレーム反映 — Commit 廃止・暗黙反映
- `commit_composition` システムと `dcomp.commit()` を除去。WUC は DispatcherQueue が pump 上で tick する際に暗黙反映（要件 7.1）。`CommitComposition` schedule slot は `ulw_present_system` が残るため保持し、DComp commit システムのみ登録解除。フレーム境界のデータフロー（1 フレームで適用される変更集合）は不変（要件 7.2）。観測等価性は R8 の受け入れハーネスで担保（要件 7.3）。

### Decision-9: 描画等価性検証ハーネス（R8）
- **主受け入れ手段 = 自動ピクセル差分**（要件 8.5）。
- **サーフェス（D2D）層**（要件 8.6・決定論的）: D2D 描画コードは移行前後で不変。オフスクリーン WIC ビットマップ（`IWICImagingFactory2` の `CreateBitmap` ＋ D2D WIC render target、または既存 `com/wic.rs` の `CopyPixels` 読み戻し）へ同一 CommandList を描画し、ハッシュ一致／差分ゼロで自動比較。既存 `image` crate（examples/tests で使用実績）でハッシュ・PNG 出力。
- **合成層**（配置・z 順・不透明度・clip／要件 8.7）: `PrintWindow` は DComp/WUC content で黒画像化するため不採用。**Desktop Duplication API**（`IDXGIOutputDuplication`）で合成後フレームをキャプチャし、固定シーンで移行前後を比較。DWM タイミング非決定性は「静止シーンで安定するまで待機してからキャプチャ」で吸収。決定論的キャプチャ不能な範囲（例: DWM アニメ過渡）のみ目視を残差フォールバック（要件 8.7）。
- **32bit 可搬**（要件 8.4）＋**release z/LTO 疎通**（要件 8.1）はビルドマトリクスで検証。

### Decision-10: スコープ境界（隣接非侵）
- `compute_ex_style` の DComp 分岐（`WS_EX_NOREDIRECTIONBITMAP`）は不変で流用（要件 9.3・DWM 合成は DComp/WUC 共通ゆえ透過等価成立見込み・R1 で確認）。当たり判定・ULW アーム・`CompositionMode` enum は非改変（要件 9.1/9.2）。WUC 新能力・投機的抽象は非導入（要件 9.4）。

## Architecture Pattern Evaluation

| Option | 説明 | 長所 | リスク | 判定 |
|---|---|---|---|---|
| A in-place 全差し替え | Ext/Resource/コンポーネント全て内部差し替え | 新規最小 | SurfaceBrush 構造変化が in-place に収まらぬ箇所 | 却下 |
| B WUC 専用新設・並存 | 型で DComp/WUC 分離 | 段階移行安全 | 二重コンポーネントで schedule/クエリ複雑化・冗長 | 却下 |
| **C 混成** | Ext/Resource 新設＋コンポーネント内部 in-place | スパイク分離＋消費側改修最小の両立 | 層順序依存の計画管理 | **採用** |

## Risks & Mitigations（design 追記）

- **R-High（サーフェス束ね）**: SetContent→SurfaceBrush の構造変化 → `SurfaceGraphics` に brush 保持追加・生成/解除両経路を単一システムで対称化。R1 スパイクで 1 surface 表示を先行検証。
- **R-Med（DispatcherQueue apartment）**: `DQTAT_COM_NONE` vs `ASTA` の選択が現状 COM 初期化に依存 → R1 スパイクで実測確定。
- **R-Med（clip 個別半径）**: WUC 直接等価なし → PathGeometry 写像。areka 本体未使用ゆえ実害小だがビルド等価のため実装。
- **R-Med（32bit×WUC runtime）**: i686 ビルドは実績あるが WUC ランタイム動作は未実証 → R1 スパイクを i686 でも走らせる。
- **R-Med（合成層キャプチャの非決定性）**: DWM タイミング → 静止シーン安定待ちキャプチャ＋残差目視フォールバック。

## References（design 追記）

- [Using the Visual Layer with Win32 — Microsoft Learn](https://learn.microsoft.com/en-us/windows/uwp/composition/using-the-visual-layer-with-win32) — DispatcherQueue の DQTYPE_THREAD_CURRENT 相乗り、DesktopWindowTarget 束縛の正準パターン。
- [CreateDispatcherQueueController — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/dispatcherqueue/nf-dispatcherqueue-createdispatcherqueuecontroller) — options 構造・apartment 種別。
- [ICompositionDrawingSurfaceInterop::BeginDraw — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/windows.ui.composition.interop/nf-windows-ui-composition-interop-icompositiondrawingsurfaceinterop-begindraw) — D2D DC ＋ offset out-param 意味論。
- `windows` 0.62.2 crate ソース（ディスク実読）・`windows-numerics` 0.3.0。

---

## § ギャップ分析（kiro-validate-gap 由来・原文保持）

# ギャップ分析: wintf-dcomp-to-wuc-migration

> 対象: 表示合成バックエンドを DirectComposition（DComp）から Windows.UI.Composition（WUC）へ**純粋等価移行**する。
> 本書は要件（requirements.md）と既存コードベースの差分を分析し、設計判断の材料を提示する（決定はしない）。
> 言語: ja（spec.json）。日付: 2026-07-01。

## 1. サマリ（3-5 点）

- **DComp パスは概ね隔離されているが、brief の見出し 8 ファイルより広い**。COM ラッパー（`com/dcomp.rs`）と 4 コンポーネント種（`WindowGraphics`/`VisualGraphics`/`SurfaceGraphics`＋`DCompGraphicsResource`）に DComp 型が集中し、それを消費する ECS システムは **brief 記載外の 3 ファイルにも及ぶ**（`visual_manager.rs`・`clip_sync.rs`・`window_pos.rs`）。加えて schedule 登録（`ecs/world/mod.rs`）が置換対象システムを跨いで結線している。ULW アーム（`compositor.rs`/`com/ulw.rs`/`compositor_systems/`）とは COM 型レベルで完全独立で、**隔離クレーム自体は成立**する。
- **不足能力は 3 点**: ①WUC features が `Cargo.toml`（workspace）に**一切未追加**（現状 `Win32_Graphics_DirectComposition` のみ）、②DispatcherQueue 初期化がコードベースに**存在しない**（`CreateDispatcherQueueController` の grep ヒットはドキュメントのみ）、③`Compositor`/`DesktopWindowTarget`/`CompositionDrawingSurface`＋interop trait 群の利用実績ゼロ。message pump は `wintf-winmsg-executor` の `block_on`/`MessageLoop::run` に委譲済みで、**差し替え不要（相乗り前提が成立）**。
- **候補アプローチは「隔離層の内側差し替え（Option A 変種）」が本命**。COM 型を保持する 4 コンポーネント＋Resource と `com/dcomp.rs` の Ext trait を WUC 版へ置換し、消費側システムはコンポーネントのアクセサ経由で最小改修する。新規 `com/wuc.rs`＋`wuc_resource.rs` を作る Option B 混成が現実的（詳細 §4）。
- **主要リスク（Medium〜High）**: (a) DComp は Surface を Visual へ**直付け**（`SetContent`）するが WUC は **SurfaceBrush が一段挟まる**ため、`deferred_surface_creation`/`window_visual_integration`/`clip_sync` の束ね方が構造変化する。(b) `Commit()` 廃止＝暗黙反映への状態モデル変更でフレーム境界の観測等価性を担保する必要（要件 7）。(c) DispatcherQueue の apartment 種別（ASTA/NONE）と `RoInitialize`/`CoInitializeEx` 前提の確定は**要検証**。
- **研究フラグ（設計フェーズ持ち越し）**: `windows` 0.62.2 の WUC 型・interop trait の実 API 形状（brief は discovery で存在確認済みだが本ギャップ分析では crate ソース未確認）、32bit（i686）での WUC/DispatcherQueue 動作、`WS_EX_NOREDIRECTIONBITMAP`＋`DesktopWindowTarget` の透過共存、`clip`（`IDCompositionRectangleClip`）の WUC 等価（`InsetClip`/`GeometricClip`）。

## 2. 現状調査（Current State）

### 2.1 brief の DComp ファイルマップ検証（実コードと突合）

brief 記載の各ファイル・接続先を実在確認した。**全て実在し、記述は正確**。ただし DComp 型の消費範囲は見出しより広い。

| 層 | brief 記載ファイル | 実在 | 補足（実コード確認） |
|---|---|---|---|
| デバイス | `ecs/graphics/dcomp_resource.rs`, `com/dcomp.rs` | ✅ | `DCompGraphicsResource`（`IDCompositionDesktopDevice`＋`IDCompositionDevice3` の lazy-init 単一 Resource）。`com/dcomp.rs` は Ext trait 群（Device/DesktopDevice/Target/Visual/Surface/Rotate/Matrix）。 |
| ターゲット | `ecs/graphics/systems/init.rs`, `components.rs`（`WindowGraphics`） | ✅ | `create_window_graphics_for_hwnd` が `desktop.create_target_for_hwnd(hwnd, true)`。`WindowGraphics` が `IDCompositionTarget`＋`ID2D1DeviceContext` を保持。 |
| ビジュアル木 | `ecs/graphics/systems/visual_sync.rs`, `components.rs`（`VisualGraphics`） | ✅ | `visual_hierarchy_sync_system` が `remove_all_visuals`＋`add_visual`（Children 順・深さソート）。`visual_property_sync_system` が Offset/Opacity。`VisualGraphics` が `IDCompositionVisual3`＋parent キャッシュ。 |
| サーフェス | `ecs/graphics/systems/surface.rs`, `render.rs` | ✅ | `deferred_surface_creation_system` が `dcomp.create_surface(w,h,B8G8R8A8,PREMUL)`＋`visual.SetContent(&surface)`（**直付け**）。`render_surface` が `begin_draw`→D2D DC→`end_draw`。 |
| フレーム反映 | `ecs/graphics/systems/render.rs`（`commit_composition`） | ✅ | `commit_composition` が毎フレーム末 `dcomp.commit()`。 |
| 窓フラグ | `runtime/window_factory.rs`（`compute_ex_style`） | ✅ | `compute_ex_style`: DComp→`(ex_style & !WS_EX_LAYERED) \| WS_EX_NOREDIRECTIONBITMAP`。純関数＋単体テスト 3 本あり。 |
| モード選択 | `ecs/window/components.rs`（`CompositionMode`） | ✅ | `CompositionMode` enum（`ULW` 既定／`DComp`）。`Window.composition_mode()` で参照。生成時固定。 |

### 2.2 brief 見出しに**無い** DComp 依存ファイル（追加発見・要注意）

以下は DComp 型を直接参照するが brief のファイルマップには載っていない。**移行の実対象**であり、design のタスク分解でカバーが必要。

- `ecs/graphics/visual_manager.rs`: `create_visual_only`（`dcomp.create_visual()`→`IDCompositionVisual3`）と `window_visual_integration_system`（`target.SetRoot(visual)`）。**Visual 生成と Root 束縛の核**。brief の「ビジュアル木」層の一部だが未列挙。
- `ecs/graphics/systems/clip_sync.rs`: `clip_sync_system` が `dcomp.create_rectangle_clip()`→`IDCompositionRectangleClip`→`visual.SetClip`。**クリップ（角丸含む）は DComp 固有 API**で WUC では別型（`CompositionClip`/`InsetClip`）へ写像が必要。等価維持の要件 5.3/8.3（見た目等価）に絡む。**設計判断項目**。
- `ecs/graphics/systems/window_pos.rs`: `invalidate_dependent_components` が `DCompGraphicsResource::invalidate()` を呼ぶ（デバイスロスト経路）。DComp 型は直接触らないが Resource ライフサイクルに関与。
- `ecs/graphics/components.rs` の `SurfaceGraphics` は `IDCompositionSurface` を保持（brief は `components.rs` を「WindowGraphics/VisualGraphics」で挙げるが Surface も同ファイル）。
- `ecs/world/mod.rs`（schedule 登録）: 置換対象システムを `GraphicsSetup`/`PreRenderSurface`/`RenderSurface`/`Composition`/`CommitComposition`/`PreLayout` の各 schedule に結線。**move/rename ではなく中身差し替えなら schedule 構造は据え置き可**だが、`commit_composition` 廃止（要件 7）は `CommitComposition` schedule の該当システム除去または no-op 化を要する。
- `com/animation.rs`: `IDCompositionAnimation` を Param に取る Ext（`GetCurve`）を含むが、これは **UIAnimation（`Win32_UI_Animation`）系**で DComp アニメの周辺。本 spec は WUC 新能力（アニメ）を扱わない（要件 9.4）ため、**この経路が現状 areka で稼働しているかを design で確認**（未使用なら触らない）。

### 2.3 隔離クレームの評価（brief「DComp パスは綺麗に隔離」）

- **成立**: ULW アーム（`compositor.rs`/`com/ulw.rs`/`compositor_systems/{init,render}`）は D3D11 CPU ビットマップ＋`UpdateLayeredWindow` 経路で、DComp COM 型を一切共有しない。`invalidate_dependent_components` が両者を同一システムで無効化するが、これは `GraphicsCore`（共通 D2D/D3D11）失効時の一括処理であり型の結合ではない。
- **境界の実体**: DComp 型（`IDCompositionTarget`/`Visual3`/`Surface`/`Device3`/`DesktopDevice`/`RectangleClip`）は **`com/dcomp.rs`＋4 コンポーネント種＋6 システムファイル**に閉じる。`grep DirectComposition` のヒット 18 ファイル中、テスト・schedule・型 re-export を除く**実装コアは上記に収束**。→ 移行のブラスト半径は限定的だが brief の 8 ファイルより広い（実質 11〜12 ファイル＋schedule）。

### 2.4 message pump / スレッド構成（要件 3 の前提確認）

- `runtime/message_loop.rs`: 自作 `PeekMessageW` ポンプは**既に撤去済み**で、`wintf-winmsg-executor` の `block_on`/`MessageLoop::run` に委譲。filter は常時 `Forward`。→ 要件 3.2「既存 pump を差し替えず相乗り」は、**ライブラリの pump に DispatcherQueue を相乗りさせる**意味になる。DispatcherQueue の tick は WM_ 経由でこの pump に配送される想定（brief の「相乗り」）。
- UI スレッド固定モデルは `GraphicsCore`/`DCompGraphicsResource`/各 `*Graphics` コンポーネントの `unsafe impl Send/Sync`＋「同一 COM オブジェクトへ並行アクセスしない ECS スケジュール配置」で担保。WUC オブジェクトも同じスレッドアフィニティ規律に載せる必要（要件 3.4・brief Constraints）。

### 2.5 Cargo features（不足の核）

- workspace `windows` 0.62.2 features に **WUC 関連が皆無**。現状: `Win32_Graphics_DirectComposition` を含むが、`UI_Composition`/`UI_Composition_Desktop`/`Win32_System_WinRT_Composition`/`Win32_System_WinRT`/`System`/`Foundation`/`Foundation_Numerics` はいずれも**未追加**。→ 移行の**最初の必須変更**（要件 2/3/4/5/6 の全 API 前提）。features は workspace 集中管理（ルート `Cargo.toml`）なので追加はそこ。
- release 最適化（`opt-level='z'`, `lto=true`, `codegen-units=1`）と 32bit 可搬性は要件 8.1/8.4 の受け入れ条件。WUC/WinRT features 追加後もこれらでビルドが通ることは **design/impl での検証項目**（features 追加はバイナリサイズ・LTO へ影響しうる）。

## 3. 要件→アセット対応表（ギャップ タグ: Missing / Unknown / Constraint）

| 要件 | 必要技術要素 | 既存アセット | ギャップ |
|---|---|---|---|
| R1 スパイク検証 | DispatcherQueue＋DesktopWindowTarget＋D2D BeginDraw の最小往復（1 surface） | なし（新規） | **Missing**: スパイク用の最小 example/テストハーネス。`examples/` に多数の DComp demo あり（雛形流用可）。 |
| R2 デバイス層 | `Compositor`＋`ICompositorInterop::CreateGraphicsDevice(d2d)` → `CompositionGraphicsDevice`（lazy 単一） | `DCompGraphicsResource`（lazy 単一の同型ライフサイクル） | **Missing** API 利用実績。**Constraint**: lazy-init・単一インスタンス方針を維持（要件 2.2）。 |
| R3 DispatcherQueue | `CreateDispatcherQueueController`（現在スレッド）＋pump 相乗り＋長寿命ドレイン | pump は `wintf-winmsg-executor` 委譲済み | **Missing** DQ 初期化。**Unknown**: apartment 種別（ASTA vs NONE）、`RoInitialize`/`CoInitializeEx` 前段の要否と現状の初期化状況。 |
| R4 ターゲット束縛 | `ICompositorDesktopInterop::CreateDesktopWindowTarget(hwnd, isTopmost)` → `DesktopWindowTarget` | `WindowGraphics`（`IDCompositionTarget` 保持・HWND 束縛） | **Missing** API。`WindowGraphics` の内部型差し替え＋アクセサ改名で吸収可能。 |
| R5 ビジュアル木 | `CreateContainerVisual`/`CreateSpriteVisual`＋子リスト操作（`Children`/`InsertAtTop` 等） | `visual_hierarchy_sync`（remove_all＋add_visual・Children 順）／`VisualGraphics` | **Constraint**: DComp `AddVisual/RemoveVisual/RemoveAllVisuals` と WUC `Visuals` コレクション API の**意味写像**（Z 順・親子）。offset/opacity も `Visual.Offset`/`Opacity`（Vector3/float）へ写像。 |
| R6 サーフェス | `CompositionGraphicsDevice.CreateDrawingSurface`＋`ICompositionDrawingSurfaceInterop::BeginDraw`（D2D DC 直返し）＋`CreateSurfaceBrush`→`SpriteVisual.Brush` | `deferred_surface_creation`（`create_surface`＋`SetContent` 直付け）／`render_surface`（begin/end draw） | **Missing** API＋**構造変化**: 直付け→**SurfaceBrush 一段挟む**。`SurfaceGraphics` に brush 保持追加が要る可能性。B8G8R8A8/PREMUL は WUC でも同指定（要件 6.4）。 |
| R7 フレーム反映 | `Commit()` 廃止＋DispatcherQueue ティックの暗黙反映 | `commit_composition`（毎フレーム `Commit()`） | **Constraint**: `commit_composition` を除去/no-op 化。フレーム境界のデータフロー等価性を観測で担保（要件 7.2/7.3）。 |
| R8 描画等価受入 | ビルド通過（z/LTO）＋起動同一描画＋32bit 可搬 | 既存 release profile／CI なし（未確認） | **Unknown**: 等価性の**検証手段**（ピクセル比較 or 目視 E2E）。既存に自動描画回帰テスト基盤があるか design で要確認。 |
| R9 スコープ境界 | 当たり判定・ULW・`WS_EX_NOREDIRECTIONBITMAP`・新能力抽象を触らない | `compute_ex_style`（NOREDIRECTIONBITMAP 維持）／ULW アーム独立 | **Constraint**: `compute_ex_style` DComp 分岐は原則不変。`clip`（角丸）の WUC 写像が「新能力」に踏み込まない範囲かを design で線引き。 |
| R10 変更前提示 | 対象ファイル・変更内容の事前提示、不確実 API は質問、`doc/COMPAT_ARCHITECTURE.md` 更新 | プロセス要件（コード非依存） | **Constraint**: 実装フローの規律。ギャップ分析の本表がその素材。 |

## 4. 実装アプローチ候補（A / B / C）

### Option A: 隔離層内側の in-place 差し替え（コンポーネント/Ext を WUC 化）
- **対象**: `com/dcomp.rs` の Ext trait 群を WUC 版 API へ書き換え、`DCompGraphicsResource`／`WindowGraphics`／`VisualGraphics`／`SurfaceGraphics` の内部保持型を WUC 型へ差し替え、消費 6 システムはアクセサ経由の最小改修。schedule 構造は据え置き（`commit_composition` のみ除去/no-op）。
- **トレードオフ**: ✅ 新規ファイル最小・既存 schedule/システム順序を温存し等価性を守りやすい。 ✅ ブラスト半径がコンポーネント境界に閉じる。 ❌ `dcomp_*` 命名のまま WUC 実体になる違和感（rename か命名維持かの判断）。 ❌ SurfaceBrush 一段挟みで `SurfaceGraphics`／`visual.SetContent` 経路の構造が変わり、in-place では収まらない箇所が出る。
- **効ort/Risk**: L（1–2 週）／Medium。

### Option B: WUC 専用モジュール新設（`com/wuc.rs`＋`wuc_resource.rs`）＋コンポーネント新設
- **対象**: `com/wuc.rs`（Compositor/Interop の Ext）と `WucGraphicsResource` を新規作成し、`WindowGraphics`/`VisualGraphics`/`SurfaceGraphics` に相当する WUC コンポーネント（例 `WucWindowGraphics` 等）を新設。DComp 版は移行完了まで残置し、システムを WUC 版へ切替。
- **トレードオフ**: ✅ DComp と WUC を型で分離しスパイク（R1）→全面移行の段階実施が安全。 ✅ 命名が実体と一致。 ❌ 「純粋等価移行」で並存は冗長・二重コンポーネントが schedule/クエリを複雑化。 ❌ 最終的に DComp 側を消す作業（ULW 除去 spec と別に）が発生。**本 spec は ULW を残す方針だが DComp 二重化は別問題**。
- **効ort/Risk**: L〜XL／Medium。

### Option C: 混成（Ext/Resource は新設、コンポーネントは in-place 差し替え）【推奨の叩き台】
- **戦略**: (1) features 追加＋`com/wuc.rs`（interop Ext）＋`WucGraphicsResource`（Compositor＋CompositionGraphicsDevice＋DispatcherQueue controller 保持）を**新設**。(2) R1 スパイクを新設 example で先行検証。(3) 既存 `WindowGraphics`/`VisualGraphics`/`SurfaceGraphics` の**内部型のみ WUC へ差し替え**（コンポーネント名・アクセサ形は極力維持し消費側改修を最小化）、Surface は brush 保持を追加。(4) `commit_composition` を除去し `CommitComposition` schedule から外す。(5) `clip_sync` を WUC clip へ写像。
- **トレードオフ**: ✅ スパイク段階分離（安全）＋消費側の改修最小（等価維持）を両立。 ✅ 新規は Resource/Ext に限定、コンポーネントは温存で schedule 不変。 ❌ 計画の粒度管理が必要（スパイク→デバイス→ターゲット→木→サーフェス→反映の順序依存）。
- **効ort/Risk**: L／Medium。

## 5. 効ort / Risk（層別）

| 移行層 | 効ort | Risk | 一言根拠 |
|---|---|---|---|
| features 追加＋ビルド疎通 | S | Low | workspace features 追記のみ。ただし z/LTO/32bit 疎通確認込みなら Low〜Medium。 |
| R1 スパイク（DQ＋DWT＋D2D 往復） | S〜M | Medium | 新規だが公式チュートリアル実証あり（brief）。DQ apartment 種別の確定が肝。 |
| デバイス層（R2） | S | Low | lazy 単一の既存ライフサイクルに写像。`CreateGraphicsDevice(d2d)` は既存 D2D デバイス流用。 |
| DispatcherQueue（R3） | M | Medium | pump 相乗り・長寿命・ドレイン・`RoInitialize`/`CoInitializeEx` 前提が Unknown。 |
| ターゲット束縛（R4） | S | Low | `WindowGraphics` 内部型差し替え。 |
| ビジュアル木（R5） | M | Medium | Children 順・Z 順・offset/opacity の写像。既存ロジック踏襲可だが API 形状差あり。 |
| サーフェス（R6） | M〜L | **High** | 直付け→**SurfaceBrush 一段挟む**構造変化。`SetContent`/`clip`/`begin_draw` 経路の再結線。 |
| フレーム反映（R7） | S | Medium | `Commit()` 除去は容易だが**観測等価性の担保**が難所。 |
| clip 写像（clip_sync） | M | Medium〜High | DComp `RectangleClip`（角丸 8 半径）→WUC clip の等価が未確認。 |
| 等価性検証（R8） | M | Medium | 検証手段（ピクセル/目視）と回帰基盤の有無が Unknown。 |

## 6. 設計フェーズへの持ち越し（Research Needed）＋推奨

### Research Needed（設計で確定）
1. **`windows` 0.62.2 の実 API 形状**: `Compositor`/`ICompositorInterop::CreateGraphicsDevice`/`ICompositorDesktopInterop::CreateDesktopWindowTarget`/`ICompositionDrawingSurfaceInterop::BeginDraw`/`CompositionGraphicsDevice::CreateDrawingSurface`/`CreateSurfaceBrush` のシグネチャと必要 features の**crate ソース実確認**（brief は discovery で存在確認済み・本ギャップ分析では未確認）。
2. **DispatcherQueue 詳細**: `DQTYPE_THREAD_CURRENT`＋apartment 種別（`DQTAT_COM_NONE` vs `ASTA`）の選択、`RoInitialize`/`CoInitializeEx(APARTMENTTHREADED?)` の前段要否、現状 areka の COM 初期化状態、pump 相乗り時の tick 配送機序。
3. **SurfaceBrush 束ね方**: `SpriteVisual.Brush = CreateSurfaceBrush(surface)` により `SurfaceGraphics` に brush を保持すべきか、`SetContent`→`Brush` 経路変更が `deferred_surface_creation`/`cleanup_surface_on_commandlist_removed`/`window_visual_integration`/`clip_sync` へ及ぼす波及。
4. **clip（角丸）等価**: DComp `IDCompositionRectangleClip`（角丸 8 半径・DPI スケール）の WUC 写像（`InsetClip`＋`CompositionRoundedRectangleGeometry`/`CompositionGeometricClip` 等）の具体型選定。**スコープはディスカッション議題 2 で決定済み**——clip 等価写像は「既存機能の等価移行」として **in scope**（R5.4 追加・R9.4 の「新能力」線引き明記）。`clip_sync.rs` は DComp 型を直接参照するため移行しないとビルド不通＝必須対象。design が確定するのは WUC 側の具体クリップ型と DPI スケール等価のみ。
5. **32bit（i686）× WUC/DispatcherQueue**: WinRT interop の 32bit 動作（memory では host-32 で i686 ビルド実績あり・WUC は別）。
6. **透過共存**: `WS_EX_NOREDIRECTIONBITMAP`＋`DesktopWindowTarget` の透過が DComp 時と同一に成立するか（DWM 合成は共通で成立見込みだが要検証）。
7. **等価性検証手段（方針決定済み・機構は design で確定）**: ディスカッション議題 1 で **自動ピクセル差分ハーネスを主たる受け入れ手段とする（R8.5/8.6）** と決定。design で確定すべきは**キャプチャ機構**——(a) サーフェス（D2D）層は WIC ビットマップ等へのレンダバックでビット等価比較可能（D2D 描画コード不変ゆえ確実な回帰ガード）、(b) 合成層（配置・Z 順・不透明度・clip）のキャプチャは DComp/WUC content で `PrintWindow` が黒画像化する等の癖があり、Desktop Duplication / DWM サムネイル等の決定論的キャプチャ手段の選定と DWM タイミング非決定性の扱いが要検証。決定論的キャプチャ不能な範囲のみ目視を残差フォールバック（R8.7）。

### 設計への推奨
- **推奨アプローチ**: Option C（混成）を叩き台に。features 追加→R1 スパイク先行→デバイス/ターゲット/木/サーフェス/反映の順で層ごとに等価確認しながら in-place 差し替え。新規は Resource（`WucGraphicsResource`）と Ext（`com/wuc.rs`）に限定し、コンポーネント名と schedule 構造は温存して等価性を守る。
- **brief のファイルマップを design で拡張**: 見出し 8 ファイルに `visual_manager.rs`・`clip_sync.rs`・`window_pos.rs`（invalidate 経路）・`ecs/world/mod.rs`（schedule）・`components.rs` の `SurfaceGraphics` を明示追加し、要件 10.1 の「対象ファイル事前提示」を漏れなく満たす。
- **`commit_composition` と `CommitComposition` schedule**: 反映モデル変更（R7）で該当システムを除去/no-op 化する設計判断を明記（ULW 側 `ulw_present_system` は同 schedule に残す）。
- **正本更新**: 設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本として更新（要件 10.3）。

---

## 付記: 検証済みファイル一覧（絶対パス）

- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\ecs\graphics\dcomp_resource.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\com\dcomp.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\ecs\graphics\components.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\ecs\graphics\systems\init.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\ecs\graphics\systems\visual_sync.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\ecs\graphics\systems\surface.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\ecs\graphics\systems\render.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\runtime\window_factory.rs`
- `C:\home\maz\git\areka\.claude\worktrees\vibrant-mirzakhani-901faa\crates\wintf\src\ecs\window\components.rs`
- 追加発見: `ecs\graphics\visual_manager.rs`, `ecs\graphics\systems\clip_sync.rs`, `ecs\graphics\systems\window_pos.rs`, `ecs\graphics\core.rs`, `ecs\world\mod.rs`, `runtime\message_loop.rs`, ルート `Cargo.toml`（workspace windows features）
