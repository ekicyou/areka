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
4. **clip（角丸）等価**: DComp `IDCompositionRectangleClip`（角丸 8 半径・DPI スケール）の WUC 写像（`InsetClip`＋`CompositionRoundedRectangleGeometry`/`CompositionGeometricClip` 等）が「新能力導入禁止（要件 9.4）」に抵触しない等価範囲か。
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
