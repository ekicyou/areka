# Brief: wintf-dcomp-to-wuc-migration（本坑 / main）

> **種別**: 本坑（main）。通常の kiro ライフサイクル（requirements → design → tasks → impl → complete）。PR ベース squash マージで `main` へ統合。**pilot は切らない**（`/kiro-discovery` 調査で windows 0.62.2 の WUC 相互運用が **GO-with-caveats** と確定・耐力壁級の Unknown 無し）。
> **位置づけ**: M1（emo2-boot）とは別軸の **wintf 基盤層**。表示バックエンドの差し替え一本。ULW 除去はこの spec に**含めない**（別 spec `wintf-ulw-removal`・下記 Downstream）。

## Problem

表示レイヤーの合成は現在 **DirectComposition**（`IDCompositionDevice3` / `IDCompositionTarget` / `IDCompositionVisual3` / `IDCompositionSurface`）に依存している。これを WinRT の **Windows.UI.Composition**（`Compositor` / `DesktopWindowTarget` / `CompositionDrawingSurface` 等）へ移行し、DirectComposition への依存を廃して表示合成基盤を WUC へ寄せたい。移行の狙いは **DComp 依存の廃止・純粋等価移行**であり、WUC の新能力（アニメ/エフェクト）活用は本 spec の目的ではない（拡張は「2 例目の実物」が要求してから）。

## Current State（調査済み・DComp パスは綺麗に隔離されている）

| 層 | 現状（DComp） | 接続先ファイル |
|---|---|---|
| デバイス | `IDCompositionDesktopDevice`/`Device3`（lazy-init 単一） | `ecs/graphics/dcomp_resource.rs`, `com/dcomp.rs` |
| ターゲット | `CreateTargetForHwnd(hwnd)` → `IDCompositionTarget` | `ecs/graphics/systems/init.rs`, `ecs/graphics/components.rs`（`WindowGraphics`） |
| ビジュアル木 | `IDCompositionVisual3` ＋ `AddVisual`/`RemoveVisual`/`RemoveAllVisuals`（ChildOf→Z順） | `ecs/graphics/systems/visual_sync.rs`, `components.rs`（`VisualGraphics`） |
| サーフェス | `CreateSurface(w,h,B8G8R8A8,PREMUL)` ＋ `BeginDraw`→D2D DC→`EndDraw` | `ecs/graphics/systems/surface.rs`, `render.rs` |
| フレーム反映 | 毎フレーム末 `Commit()`（全変更を原子的に適用） | `ecs/graphics/systems/render.rs`（`commit_composition`） |
| 窓フラグ | `WS_EX_NOREDIRECTIONBITMAP`（DComp モード時） | `runtime/window_factory.rs`（`compute_ex_style`） |
| モード選択 | `CompositionMode` enum（ULW 既定 / DComp・生成時固定） | `ecs/window/components.rs` |

- **ULW アーム（別軸・本 spec 対象外）**: `ecs/graphics/compositor.rs` / `com/ulw.rs` / `compositor_systems/`（CPU ビットマップ＋`UpdateLayeredWindow`）。DComp とは独立経路。

## Desired Outcome

表示バックエンドが Windows.UI.Composition で再構成され、**描画結果・再描画挙動が移行前と完全等価**。ビルドが通り、起動して従来と同一の描画結果が得られる。**当たり判定・ウィンドウ管理・スレッド構成は不変**（要件）。

## Approach

`windows` 0.62.2 の `Windows::UI::Composition` ＋ `Win32::System::WinRT::Composition` interop trait 群を使用（調査で全型・全 interop trait の存在確認済み）。DComp→WUC の写像:

- **デバイス**: `Compositor`（WinRT）＋ `ICompositorInterop::CreateGraphicsDevice(d2d/d3d11 device)` → `CompositionGraphicsDevice`。
- **ターゲット**: `Compositor.cast::<ICompositorDesktopInterop>()::CreateDesktopWindowTarget(hwnd, isTopmost)` → `DesktopWindowTarget`（HWND 束縛）。
- **ビジュアル木**: `IDCompositionVisual3` → `ContainerVisual` / `SpriteVisual`（`CreateContainerVisual`/`CreateSpriteVisual`・親子と Z 順は現行ロジック踏襲）。
- **サーフェス**: `CreateSurface`+`BeginDraw` → `CompositionDrawingSurface` ＋ `ICompositionDrawingSurfaceInterop::BeginDraw`（**D2D `ID2D1DeviceContext` を直接返す＝現状の描画コードそのまま**）→ `EndDraw`。サーフェスは `SpriteVisual.Brush = Compositor.CreateSurfaceBrush(surface)` で束ねる（DComp のサーフェス直付けに対し **brush が一段挟まる**）。
- **フレーム反映**: `Commit()` は **廃止**（WUC は DispatcherQueue ティックで暗黙反映）。バッチ→暗黙反映へ状態モデルが変わるが、データフローは等価。
- **DispatcherQueue（唯一の新規初期化）**: `Compositor` 生成前に UI スレッドで `CreateDispatcherQueueController(DispatcherQueueOptions{ threadType: DQTYPE_THREAD_CURRENT, apartmentType: DQTAT_COM_NONE|ASTA })`。**既存の message pump に相乗りし、ポンプは差し替えない**（＝スレッド構成不変の要件を満たす。公式 Win32 チュートリアルで実証済み）。`RoInitialize`/`CoInitializeEx` を前段に確認。controller は compositor より長寿命・終了時にドレイン。
- **窓フラグ**: `WS_EX_NOREDIRECTIONBITMAP` を DComp パスからそのまま流用（透過挙動は DComp と同一）。
- **content binding**: **CompositionDrawingSurface（BeginDraw D2D）パス**を採る（現状の per-frame D2D 再描画に一致）。swapchain パス（`CreateCompositionSurfaceForSwapChain`）は**採らない**。
- **先頭タスク＝スパイク検証**: DispatcherQueue 統合＋`DesktopWindowTarget`＋D2D `BeginDraw` の最小往復（1 サーフェスを表示）を**本 spec 内のスパイク**で先に走らせ、等価描画を確認してから全面移行（別途 pilot は切らない）。

**既存コードに触れる前に、対象ファイルと変更内容を依頼者へ提示して確認を取る**（推測で書き換えない）。features 追加（`UI_Composition`, `UI_Composition_Desktop`, `Win32_System_WinRT_Composition`, `Win32_System_WinRT`, `System`, `Foundation`, `Foundation_Numerics` 等）は最小限。

## Scope

- **In**: 上表 DComp パス各ファイルの WUC 差し替え（device / target / visual tree / surface / frame-apply）。`CreateDispatcherQueueController` 初期化の UI スレッドへの組み込み（pump 非差し替え）。`Cargo.toml` の WUC features 追加（最小）。**描画等価性の検証**（見た目・再描画挙動が移行前と一致・ビルド通過・起動同一結果）。
- **Out**: **ULW 一式の除去**（別 spec `wintf-ulw-removal`・クリックスルー本坑完了後）。`CompositionMode` enum の撤去/整理（ULW 除去 spec の領分）。当たり判定・ウィンドウ管理・スレッド構成の変更。WUC 新能力（`CompositionAnimation`/エフェクトグラフ）の活用。投機的抽象・拡張シームの追加。swapchain content パス。

## Boundary Candidates

- デバイス層（`Compositor` ＋ `CompositionGraphicsDevice` ＋ DispatcherQueue 初期化）
- ターゲット束縛（`DesktopWindowTarget` ← HWND・`WindowGraphics` 相当）
- ビジュアル木同期（`ContainerVisual` AddVisual/RemoveVisual 相当・`VisualGraphics` 相当）
- サーフェス描画（`CompositionDrawingSurface` BeginDraw/EndDraw ＋ `CreateSurfaceBrush`）
- フレーム反映（`Commit()` 廃止 → 暗黙反映への状態モデル調整）

## Out of Boundary

- ULW 除去・`CompositionMode` enum の collapse（別 spec）。
- クリックスルー機構（`wintf-clickthrough-alpha-toggle`・表示層と独立）。
- M1 emo2-boot の各エンジントラック（別軸）。

## Upstream / Downstream

- **Upstream**: 既存 DComp 切替基盤（`wintf-dcomp-migration-*` 系 completed が築いた土台）／D2D・D3D11 デバイス／`wintf-winmsg-executor`（UI スレッド message pump・DispatcherQueue はこれに相乗り）／`windows` 0.62.2 の WUC bindings。
- **Downstream**:
  - `wintf-clickthrough-alpha-toggle`（本坑・未着手）: **意味論的に非衝突**＝表示層（合成 visual/surface）と当たり判定層（HWND ex-style）は独立（pilot REPORT の核心原理）。本移行は表示層だけ、クリックスルーは当たり判定層だけを触る。**実質の重なりは `runtime/window_factory.rs::compute_ex_style` 1 関数のみ**（本移行は `WS_EX_NOREDIRECTIONBITMAP` 維持でほぼ変更なし・クリックスルーは `WS_EX_TRANSPARENT` トグル＋`WS_EX_LAYERED` 同伴を追加＝別関心・NOREDIRECTIONBITMAP は一致）＝**テキストマージ点**。後から入る方が rebase。DispatcherQueue（本移行・UI スレッド）とカーソル監視ワーカ（クリックスルー・別スレッド）は別機構で無衝突。α源（per-widget `AlphaMask`）も本移行は触らない。**順序任意・致命的手戻りなし**（本移行を先に入れればクリックスルーは最終バックエンド上で一度検証で済む微利）。
    - **design 時の検証項目（後から同居する側で確認）**: WUC `DesktopWindowTarget` ＋ `WS_EX_LAYERED`（同伴フラグ・非 ULW 描画）＋ `WS_EX_NOREDIRECTIONBITMAP` の共存。pilot は DComp で実証済み・DWM 合成は DComp/WUC 共通ゆえ成立見込みだが、Win32/WinRT 相互作用は推測せず検証する。
  - `wintf-ulw-removal`（**未作成・要 just-in-time**）: 本移行完了 **かつ** `wintf-clickthrough-alpha-toggle` 完了（＝ULW 無しで別プロセス透過が成立）を前提に、ULW 一式除去＋`CompositionMode` を WUC 単独へ collapse。
  - WUC ベースの将来アニメ/エフェクト（M2 以降・本 spec では足がかりも残さない）。

## Existing Spec Touchpoints

- **Extends**: `wintf-dcomp-migration-0..4` / `wintf-dcomp-to-layered-migration`（completed）が築いた DComp 表示基盤を置換。
- **Adjacent**: `wintf-clickthrough-alpha-toggle`（表示層 vs 当たり判定層の二層分離ゆえ独立）／`wintf-ulw-removal`（未作成・ULW 側の後続）。

## Constraints

- Rust 2024・`windows` 0.62.2 系・**tokio 禁止**。32bit 可搬性を崩さない。
- **DispatcherQueue は UI スレッドの既存 pump に相乗り**（`GetMessage`/`DispatchMessage` ループを差し替えない＝スレッド構成不変の要件）。全 WUC オブジェクトは UI スレッドにスレッドアフィニティ（既存 render/window UI スレッド固定モデルと整合・他 actor は channel で marshal）。
- **描画等価が受け入れ基準**（見た目・再描画挙動が移行前と変わらない）。
- 既存リリース最適化（`opt-level='z'`, `lto=true`）と互換。
- 既存本体コードは推測で書き換えない（変更前に対象と内容を依頼者へ提示）。
- 不確実な Win32/WinRT API・クレート仕様は推測で進めず質問する。
- 設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本として更新。
