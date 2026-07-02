# 実装計画: wintf-dcomp-to-wuc-migration

> 本移行は設計上 **層順序を厳守する逐次移行**（features→スパイク→device→target→tree→surface→clip→frame→検証）。層の順序依存を守るため並行実行余地は無く、`(P)` マーカーは付与しない（research.md §B-Decision-1）。
> **変更前提示ゲート（要件 10.1/10.2）**: 既存本体コードを触る各タスク（2.1 以降）は、着手前に design.md「File Structure Plan」の対象ファイルと変更内容を依頼者へ提示して確認を得る。不確実な Win32/WinRT API・クレート仕様は推測せず確認する。

- [ ] 1. 基盤: features・interop・スパイク検証
- [x] 1.1 WUC features と windows-numerics をルート Cargo.toml へ追加しビルド疎通
  - `windows` に `UI_Composition`/`UI_Composition_Desktop`/`Win32_System_WinRT`/`Win32_System_WinRT_Composition`/`System`/`Foundation`/`Graphics_DirectX` を追加し、`windows-numerics = "0.3"` を依存追加
  - 既存 DComp features（`Win32_Graphics_DirectComposition`）は残置（ULW/`CompositionMode` enum が参照）
  - 観測可能な完了: `cargo build` と `cargo build --release`（`opt-level='z'`・`lto=true`）が x64 で通過する
  - _Requirements: 8.1_
  - _Descoped (owner 2026-07-02): i686 ビルドは対象外。wintf は表示合成レイヤーで x64/arm64 のみ、i686 は helper 専用クレート。要件 8.4 の 32bit 可搬節は wintf にはアーキ矛盾ゆえ x64 のみで判定。arm64 検証も後回し（x64 完了後の別仕様）。_

- [x] 1.2 com/wuc interop Ext ラッパー群を実装し往復を単体テスト
  - `ICompositorInterop::CreateGraphicsDevice`・`ICompositorDesktopInterop::CreateDesktopWindowTarget`・`ICompositionDrawingSurfaceInterop::BeginDraw`/`EndDraw` を `Result` 返却の安全ラッパーへ包み、unsafe を局所化する
  - `begin_draw` は IID＋void** out-param から D2D DC を cast し `(ID2D1DeviceContext3, POINT)` を返す（現行 com/dcomp.rs の signature と byte 一致・下流 render_surface 差分ゼロ）
  - 観測可能な完了: BeginDraw wrapper の往復単体テスト（atlas offset 非ゼロケース含む）が通る
  - _Requirements: 2.1, 4.1, 6.1, 6.2, 6.3_
  - _Boundary: com/wuc Ext_

- [ ] 1.3 R1 スパイク example で最小往復と等価描画を先行検証
  - DispatcherQueue コントローラ（`DQTYPE_THREAD_CURRENT`・既存 pump 相乗り）＋`DesktopWindowTarget`＋D2D `BeginDraw` で 1 サーフェスを表示する最小往復
  - apartment 種別（`DQTAT_COM_NONE` vs `ASTA`）を現状 COM 初期化状態から実測確定し、終了時 `ShutdownQueueAsync` ドレインと drop 順（controller 最後）が成立することを確認
  - 観測可能な完了: `wuc_spike` example が 1 サーフェスを移行前と等価に描画し、等価不成立なら全面移行へ進まず原因究明する判断が記録される
  - _Requirements: 1.1, 1.2, 1.3, 3.1_
  - _Depends: 1.2_

- [ ] 1.4 スパイクで透過共存を確認
  - `WS_EX_NOREDIRECTIONBITMAP`＋`DesktopWindowTarget` で per-pixel alpha 透過が DComp 時と同一に成立することをスパイク上で確認
  - 観測可能な完了: 透明ピクセルを含むスパイクサーフェスが DComp 時と同じ per-pixel alpha 透過表示になる
  - _Requirements: 9.3_
  - _Depends: 1.3_

- [ ] 1.5 スパイクを i686 ランタイムで実証
  - スパイク example を i686 ターゲットで実行し、WUC/DispatcherQueue のランタイム動作を確認する
  - 観測可能な完了: i686 ビルドのスパイクが x64 と同一の 1 サーフェス等価描画を表示する
  - _Requirements: 8.4_
  - _Depends: 1.3_
  - _Descoped (owner 2026-07-02): wintf は i686 非対象（表示合成は x64/arm64 のみ・i686 は helper 専用）。本タスクは実施しない。_

- [ ] 2. コア: 層別 DComp→WUC 差し替え（層順序厳守）
- [ ] 2.1 WucGraphicsResource を実装し合成デバイス層を WUC 化
  - **着手前**: 本タスク以降が触る既存本体ファイルと変更内容を design.md「File Structure Plan」に基づき依頼者へ提示して確認を得る（要件 10.1／不確実 API は 10.2 に従い確認）
  - `Compositor`＋`ICompositorInterop::CreateGraphicsDevice(既存 ID2D1Device)`＋`CreateDispatcherQueueController` を lazy 単一 Resource に保持し、`invalidate`/`is_valid` を現行 `DCompGraphicsResource` と 1:1 で提供
  - `WucGraphicsResourceInner` のフィールド宣言順を controller 最後で固定して drop 順を保証し、`invalidate()` も同順で null 化する
  - 観測可能な完了: 最初の WUC ウィンドウで Resource が遅延生成され、プロセス終了時に**本番 Resource** の `ShutdownQueueAsync` ドレインが成立して shutdown クラッシュが無い（要件 3.3）
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 10.1, 10.2_
  - _Boundary: WucGraphicsResource_
  - _Depends: 1.2, 1.3_

- [ ] 2.2 合成ターゲット束縛を DesktopWindowTarget へ差し替え
  - `WindowGraphics` の内部 `IDCompositionTarget` を `DesktopWindowTarget` へ、`create_window_graphics_for_hwnd` を `CreateDesktopWindowTarget(hwnd, topmost)` へ、root 束縛を `target.SetRoot(root_visual)` へ写像
  - 観測可能な完了: ウィンドウの合成ターゲットが同一 HWND に束縛された `DesktopWindowTarget` となり、root visual が設定される
  - _Requirements: 4.1, 4.2_
  - _Boundary: WindowGraphics_
  - _Depends: 2.1_

- [ ] 2.3 ビジュアル木同期を WUC Container/Sprite へ差し替え
  - `VisualGraphics` 内部型を WUC `Visual` へ、生成を `CreateSpriteVisual`（描画対象）/`CreateContainerVisual`（純コンテナ）へ、木同期を `Children().RemoveAll()`→Children 順 `InsertAtTop`、property を `SetOffset(Vector3)`/`SetOpacity(f32)` へ写像
  - 観測可能な完了: 親子関係・Z 順・offset・opacity が移行前と同一のツリー構造・重なり順・配置で再現される
  - _Requirements: 5.1, 5.2, 5.3_
  - _Boundary: VisualGraphics_
  - _Depends: 2.2_

- [ ] 2.4 サーフェス生成・束ね・描画を WUC へ差し替え
  - `SurfaceGraphics` に `CompositionSurfaceBrush` 保持を追加し、生成を `CreateDrawingSurface`（B8G8R8A8/PREMUL）＋`CreateSurfaceBrushWithSurface`＋`SpriteVisual.SetBrush`、解除を `SetBrush(None)`、`render_surface` を interop `BeginDraw`（既存 offset 適用ロジック流用）へ写像する
  - content 束縛に swapchain 経路は用いない
  - 観測可能な完了: サーフェスが生成され brush で Sprite に束ねられて D2D 描画が表示され、解除経路で brush が解放されて黒画像化しない
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Boundary: SurfaceGraphics_
  - _Depends: 2.3_

- [ ] 2.5 clip 3 変種を WUC clip 型へ等価写像
  - `Rectangle`→`InsetClip`、`RoundedRectangle`→`CreateRoundedRectangleGeometry`＋`GeometricClip`、`RoundedRectangleIndividual`（4 角独立半径）→`CreatePathGeometry`＋`GeometricClip`、DPI スケールを半径・矩形へ乗算、`Visual::SetClip`/clear を維持
  - 観測可能な完了: 各 `ClipShape` 変種が対応する WUC clip を生成し、DPI 乗算後の形状で visual に適用される（新能力は導入しない）
  - _Requirements: 5.4, 9.4_
  - _Boundary: clip_sync_system_
  - _Depends: 2.3_

- [ ] 3. 統合: フレーム反映モデル移行と DComp 撤去
- [ ] 3.1 明示 Commit を廃し暗黙反映へ移行、ライブ Resource 登録を切替
  - `commit_composition` システムを削除し `CommitComposition` schedule から登録解除（`ulw_present_system` は同 schedule に残置）、`world/mod.rs` の**ライブ Resource 登録**を `DCompGraphicsResource`→`WucGraphicsResource` へ切替、`window_pos` の invalidate 経路を WUC Resource へ差し替え
  - 観測可能な完了: 明示 `Commit()` 呼び出しが無く DispatcherQueue ティックで反映され、アプリが移行前と等価に描画し、`ulw_present_system` が従来通り動作する
  - _Requirements: 7.1, 7.2, 7.3_
  - _Depends: 2.1, 2.4, 2.5_
  - _Boundary: world schedule, render.rs, window_pos.rs_

- [ ] 3.2 消費ゼロ化した DComp 定義を撤去し正本を更新
  - 3.1 でライブ登録が WUC へ移った後、**dead となった** `dcomp_resource.rs`/`com/dcomp.rs` の定義・登録のみ撤去する（`CompositionMode` enum と ULW アームが参照する DComp は残置・3.1 のライブ登録行には再度触れない）、`doc/COMPAT_ARCHITECTURE.md` を DComp→WUC 移行判断で正本更新
  - 観測可能な完了: 合成パスに DComp 参照が残らず（enum/ULW 残置分を除く）ビルドが通り、COMPAT 正本に移行判断が反映される
  - _Requirements: 9.2, 10.3_
  - _Depends: 3.1_

- [ ] 4. 検証: 描画等価性ハーネスと回帰
- [ ] 4.1 サーフェス層ビット等価ハーネス（ランタイム二重描画）
  - 同一 `GraphicsCommandList` をその場で (a) D2D 直描き（WIC render target・参照基準）と (b) WUC surface `BeginDraw` D2D 出力へ描画し、WIC `CopyPixels` 読み戻し→ハッシュ一致／差分ゼロを自動判定する（永続ゴールデンを repo に持たない）
  - 観測可能な完了: `surface_pixel_equivalence_test` が代表シーンでビット等価 PASS する
  - _Requirements: 8.5, 8.6_
  - _Boundary: 検証ハーネス_
  - _Depends: 2.4_

- [ ] 4.2 clip ビットマップサンプル等価検証
  - clip 各変種（個別半径含む）の幾何を既知フィルへ適用してオフスクリーン WIC render target へ描画し、`CopyPixels`→基準ビットマップサンプルとピクセル等価判定する（曖昧な差分閾値は設けずビット等価基準）
  - 観測可能な完了: 全 `ClipShape` 変種がビットマップサンプル比較で差分ゼロ PASS する
  - _Requirements: 5.4, 8.6_
  - _Depends: 2.5, 4.1_

- [ ] 4.3 合成層キャプチャ比較
  - 固定シーン（visual 配置・z 順・opacity）を Desktop Duplication でキャプチャし移行前後を比較（`PrintWindow` は黒画像化のため不採用）、DWM 非決定性は静止シーン安定待ちで吸収、決定論的キャプチャ不能な過渡のみ目視残差として範囲を明示する
  - 観測可能な完了: 固定シーンの合成層キャプチャ比較が移行前後で一致し、目視残差範囲が文書化される
  - _Requirements: 8.2, 8.3, 8.7_
  - _Depends: 3.1_

- [ ] 4.4 回帰・可搬性の最終確認
  - `ulw_present_system` 非回帰、デバイスロスト→WUC Resource 再生成、当たり判定・`compute_ex_style`（`WS_EX_NOREDIRECTIONBITMAP`）の不変、release（z/LTO）ビルドを確認する（i686 は descope・x64 のみ）
  - 観測可能な完了: ULW アーム非回帰・デバイスロスト再生成・release が通り、当たり判定と窓フラグ透過挙動が移行前と等価に保たれる
  - _Requirements: 8.1, 9.1, 9.2, 9.3_
  - _Depends: 3.1, 3.2_

## Implementation Notes

- **i686/arm64 descope（owner ekicyou 2026-07-02）**: wintf は表示合成レイヤーで **x64 or arm64 のみ**。i686（x86）は SHIORI 駆動 helper 専用の別クレートで、wintf は i686 ターゲットにならない。ゆえに task 1.1 の i686 節・task 1.5・task 4.4 の i686 ランタイム・要件 8.4 の 32bit 可搬は本移行では x64 のみで判定（arch 矛盾の spec 誤り）。arm64 検証も後回し＝x64 完了後にオプション別仕様。**当面 x64 のみを意識する**。（参考: full wintf lib を i686 build すると既存 `api.rs`/`window_factory.rs` の `SetWindowLongPtr` isize/i32 不一致で落ちるが wintf x86 非対象ゆえ修正不要）
- **1.1 完了（x64）**: ルート `Cargo.toml` に WUC features＋`windows-numerics=0.3.1` は着手時点で working tree に存在。x64 `cargo build -p wintf`（exit 0）・`cargo build -p wintf --release`（z/LTO・exit 0）通過。DComp feature `Win32_Graphics_DirectComposition` 残置確認。
- **1.2 完了・WUC BeginDraw の実挙動発見**: `com/wuc.rs` に interop Ext 3種＋`create_dispatcher_queue_controller` 実装。`begin_draw` は dcomp.rs L254-259 と byte 一致。往復テスト `com::wuc::tests::begin_draw_roundtrip` PASS（atlas updateoffset=(1,2) 実測・非ゼロ観測）。**重要（後続 2.4 render_surface へ）**: WUC `ICompositionDrawingSurfaceInterop::BeginDraw` に `Some(部分矩形)` を渡すと `E_INVALIDARG (0x80070057)`。本番 `render_surface` は `begin_draw(None)`（全面）のみ使うため影響なし。移行後も **None 経路のみ**を使うこと。apartment 種別実測: cargo test スレッドは COM 未初期化ゆえ `DQTAT_COM_ASTA` で成功（design §2.1 と一致）。
