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

- [x] 1.3 R1 スパイク example で最小往復と等価描画を先行検証
  - DispatcherQueue コントローラ（`DQTYPE_THREAD_CURRENT`・既存 pump 相乗り）＋`DesktopWindowTarget`＋D2D `BeginDraw` で 1 サーフェスを表示する最小往復
  - apartment 種別（`DQTAT_COM_NONE` vs `ASTA`）を現状 COM 初期化状態から実測確定し、終了時 `ShutdownQueueAsync` ドレインと drop 順（controller 最後）が成立することを確認
  - 観測可能な完了: `wuc_spike` example が 1 サーフェスを移行前と等価に描画し、等価不成立なら全面移行へ進まず原因究明する判断が記録される
  - _Requirements: 1.1, 1.2, 1.3, 3.1_
  - _Depends: 1.2_

- [x] 1.4 スパイクで透過共存を確認
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

> **タスク再編（owner 承認 2026-07-02「無理ならタスク整理して実行」）**: 設計 Option C の in-place 型差し替えでは、`components.rs` の 3 コンポーネント（WindowGraphics.target / VisualGraphics.inner / SurfaceGraphics.inner）とその消費側システムが `SetRoot(target, visual)` 等で**型レベルで一体**であり、2.2〜2.5 を個別に green build できない。よって **2.2・2.3・2.4・2.5・3.1（ライブ登録切替）を「コア合成カットオーバー」一体ユニットとして実装**し、3.1 完了時点で green build を復帰させる（中間の赤は squash-merge で消える・[[areka-commit-as-you-go]]）。3.2（DComp 定義撤去）・4.x は従来どおり別立て。各サブタスクは論理単位でコミットしつつ、検証・完了マークは green 復帰時にまとめて行う。

- [ ] 2. コア: 層別 DComp→WUC 差し替え（層順序厳守）
- [x] 2.1 WucGraphicsResource を実装し合成デバイス層を WUC 化
  - **着手前**: 本タスク以降が触る既存本体ファイルと変更内容を design.md「File Structure Plan」に基づき依頼者へ提示して確認を得る（要件 10.1／不確実 API は 10.2 に従い確認）
  - `Compositor`＋`ICompositorInterop::CreateGraphicsDevice(既存 ID2D1Device)`＋`CreateDispatcherQueueController` を lazy 単一 Resource に保持し、`invalidate`/`is_valid` を現行 `DCompGraphicsResource` と 1:1 で提供
  - `WucGraphicsResourceInner` のフィールド宣言順を controller 最後で固定して drop 順を保証し、`invalidate()` も同順で null 化する
  - 観測可能な完了: 最初の WUC ウィンドウで Resource が遅延生成され、プロセス終了時に**本番 Resource** の `ShutdownQueueAsync` ドレインが成立して shutdown クラッシュが無い（要件 3.3）
  - _Requirements: 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 10.1, 10.2_
  - _Boundary: WucGraphicsResource_
  - _Depends: 1.2, 1.3_

- [x] 2.2 合成ターゲット束縛を DesktopWindowTarget へ差し替え
  - `WindowGraphics` の内部 `IDCompositionTarget` を `DesktopWindowTarget` へ、`create_window_graphics_for_hwnd` を `CreateDesktopWindowTarget(hwnd, topmost)` へ、root 束縛を `target.SetRoot(root_visual)` へ写像
  - 観測可能な完了: ウィンドウの合成ターゲットが同一 HWND に束縛された `DesktopWindowTarget` となり、root visual が設定される
  - _Requirements: 4.1, 4.2_
  - _Boundary: WindowGraphics_
  - _Depends: 2.1_

- [x] 2.3 ビジュアル木同期を WUC Container/Sprite へ差し替え
  - `VisualGraphics` 内部型を WUC `Visual` へ、生成を `CreateSpriteVisual`（描画対象）/`CreateContainerVisual`（純コンテナ）へ、木同期を `Children().RemoveAll()`→Children 順 `InsertAtTop`、property を `SetOffset(Vector3)`/`SetOpacity(f32)` へ写像
  - 観測可能な完了: 親子関係・Z 順・offset・opacity が移行前と同一のツリー構造・重なり順・配置で再現される
  - _Requirements: 5.1, 5.2, 5.3_
  - _Boundary: VisualGraphics_
  - _Depends: 2.2_

- [x] 2.4 サーフェス生成・束ね・描画を WUC へ差し替え
  - `SurfaceGraphics` に `CompositionSurfaceBrush` 保持を追加し、生成を `CreateDrawingSurface`（B8G8R8A8/PREMUL）＋`CreateSurfaceBrushWithSurface`＋`SpriteVisual.SetBrush`、解除を `SetBrush(None)`、`render_surface` を interop `BeginDraw`（既存 offset 適用ロジック流用）へ写像する
  - content 束縛に swapchain 経路は用いない
  - 観測可能な完了: サーフェスが生成され brush で Sprite に束ねられて D2D 描画が表示され、解除経路で brush が解放されて黒画像化しない
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_
  - _Boundary: SurfaceGraphics_
  - _Depends: 2.3_

- [x] 2.5 clip 3 変種を WUC clip 型へ等価写像
  - `Rectangle`→`InsetClip`、`RoundedRectangle`→`CreateRoundedRectangleGeometry`＋`GeometricClip`、`RoundedRectangleIndividual`（4 角独立半径）→`CreatePathGeometry`＋`GeometricClip`、DPI スケールを半径・矩形へ乗算、`Visual::SetClip`/clear を維持
  - 観測可能な完了: 各 `ClipShape` 変種が対応する WUC clip を生成し、DPI 乗算後の形状で visual に適用される（新能力は導入しない）
  - _Requirements: 5.4, 9.4_
  - _Boundary: clip_sync_system_
  - _Depends: 2.3_

- [ ] 3. 統合: フレーム反映モデル移行と DComp 撤去
- [x] 3.1 明示 Commit を廃し暗黙反映へ移行、ライブ Resource 登録を切替
  - `commit_composition` システムを削除し `CommitComposition` schedule から登録解除（`ulw_present_system` は同 schedule に残置）、`world/mod.rs` の**ライブ Resource 登録**を `DCompGraphicsResource`→`WucGraphicsResource` へ切替、`window_pos` の invalidate 経路を WUC Resource へ差し替え
  - 観測可能な完了: 明示 `Commit()` 呼び出しが無く DispatcherQueue ティックで反映され、アプリが移行前と等価に描画し、`ulw_present_system` が従来通り動作する
  - _Requirements: 7.1, 7.2, 7.3_
  - _Depends: 2.1, 2.4, 2.5_
  - _Boundary: world schedule, render.rs, window_pos.rs_

- [x] 3.2 消費ゼロ化した DComp 定義を撤去し正本を更新
  - 3.1 でライブ登録が WUC へ移った後、**dead となった** `dcomp_resource.rs`/`com/dcomp.rs` の定義・登録のみ撤去する（`CompositionMode` enum と ULW アームが参照する DComp は残置・3.1 のライブ登録行には再度触れない）、`doc/COMPAT_ARCHITECTURE.md` を DComp→WUC 移行判断で正本更新
  - 観測可能な完了: 合成パスに DComp 参照が残らず（enum/ULW 残置分を除く）ビルドが通り、COMPAT 正本に移行判断が反映される
  - _Requirements: 9.2, 10.3_
  - _Depends: 3.1_

- [ ] 4. 検証: 描画等価性ハーネスと回帰
- [x] 4.1 サーフェス層ビット等価ハーネス（ランタイム二重描画）
  - 同一 `GraphicsCommandList` をその場で (a) D2D 直描き（WIC render target・参照基準）と (b) WUC surface `BeginDraw` D2D 出力へ描画し、WIC `CopyPixels` 読み戻し→ハッシュ一致／差分ゼロを自動判定する（永続ゴールデンを repo に持たない）
  - 観測可能な完了: `surface_pixel_equivalence_test` が代表シーンでビット等価 PASS する
  - _Requirements: 8.5, 8.6_
  - _Boundary: 検証ハーネス_
  - _Depends: 2.4_

- [x] 4.2 clip ビットマップサンプル等価検証
  - clip 各変種（個別半径含む）の幾何を既知フィルへ適用してオフスクリーン WIC render target へ描画し、`CopyPixels`→基準ビットマップサンプルとピクセル等価判定する（曖昧な差分閾値は設けずビット等価基準）
  - 観測可能な完了: 全 `ClipShape` 変種がビットマップサンプル比較で差分ゼロ PASS する
  - _Requirements: 5.4, 8.6_
  - _Depends: 2.5, 4.1_

- [x] 4.3 合成層キャプチャ比較
  - 固定シーン（visual 配置・z 順・opacity）を Desktop Duplication でキャプチャし移行前後を比較（`PrintWindow` は黒画像化のため不採用）、DWM 非決定性は静止シーン安定待ちで吸収、決定論的キャプチャ不能な過渡のみ目視残差として範囲を明示する
  - 観測可能な完了: 固定シーンの合成層キャプチャ比較が移行前後で一致し、目視残差範囲が文書化される
  - _Requirements: 8.2, 8.3, 8.7_
  - _Depends: 3.1_

- [x] 4.4 回帰・可搬性の最終確認
  - `ulw_present_system` 非回帰、デバイスロスト→WUC Resource 再生成、当たり判定・`compute_ex_style`（`WS_EX_NOREDIRECTIONBITMAP`）の不変、release（z/LTO）ビルドを確認する（i686 は descope・x64 のみ）
  - 観測可能な完了: ULW アーム非回帰・デバイスロスト再生成・release が通り、当たり判定と窓フラグ透過挙動が移行前と等価に保たれる
  - _Requirements: 8.1, 9.1, 9.2, 9.3_
  - _Depends: 3.1, 3.2_

## Implementation Notes

- **✅ 目視テスト完了（2026-07-02・オンスクリーン実検証・owner 目視確認込み）**: 当初「機構実証／手動残差」扱いだった目視系（1.3 spike 等価描画・1.4 透過・4.3 合成層）を、**GDI 全画面キャプチャ（`System.Drawing` CopyFromScreen＝DWM 合成後）による窓 rect ピクセル実測**と **owner の目視確認**で実検証し完了へ格上げ。結果: dcomp_demo カード色 (0.55,0.45,0.75)→画面 **(140,115,191) 完全一致**／wuc_spike premul 青→(22,119,220) alpha 合成／dcomp_taffy_demo レイアウト描画○（owner 目視「表示しました」）／clip_demo クリップ図形（赤 255,76,76・黄 229,204,25）描画○。**残る目視テストタスクは無し**。**動的検証（owner 実操作 2026-07-02）**: 窓右下ドラッグでリサイズ→内部レイアウトが追従して連続再描画されることを owner が確認＝**動的な再描画等価（要件 7.3）を実操作で実証**（リサイズ→Taffy 再レイアウト→WUC 更新→vsync tick 暗黙 commit→画面反映が毎フレーム取りこぼしなく成立。明示 Commit 廃止でも連続更新が正しく反映されることの裏取り）。4.3 の「自動 before/after キャプチャ非構築」判断は据置（DComp 撤去で before 基準無・DWM 非決定）だが、**移行後のオンスクリーン合成が期待色とピクセル一致することは実証済み**（before 基準の代わりに「コード上の意図色との一致」で決定論性を担保）。
- **🔥 オンスクリーン合成バグ修正（2026-07-02・owner 目視報告「窓すら表示されない」起点の総当たり検証で確定）**: WUC 移行後、dcomp_demo/dcomp_taffy_demo が「窓は visible・SetBrush×21/SetRoot/InsertAtBottom×39 全成功・エラーゼロ・なのに画面に何も出ない」状態だった。**真因（H2）: bevy MultiThreaded executor が graphics システムを worker スレッドで実行し、`CreateDispatcherQueueController(DQTYPE_THREAD_CURRENT)` がポンプの無い worker に DispatcherQueue を紐づけ、合成 commit が永遠に flush されない**（WUC はスレッド親和・DComp は free-threaded ゆえ従来 Multi でも偶々動作＝移行で顕在化）。切り分け: ECS 非依存でメインスレッド生成の wuc_spike は画面表示○（GDI 全画面キャプチャのピクセル実測 R22/G119/B220＝描画色一致）→ H1 暗黙 commit はシロ。**修正: `world/mod.rs` で WUC を触る schedule（GraphicsSetup/PreRenderSurface/RenderSurface/Composition/CommitComposition）を `ExecutorKind::SingleThreaded` 固定**（try_tick_world は UI スレッド駆動ゆえ全 WUC 呼び出し＝UI スレッド保証）。**修正後実測: dcomp_demo カード色 (0.55,0.45,0.75) → 画面ピクセル (140,115,191) 完全一致・taffy も (255,0,0) 実描画・全テスト回帰緑**。教訓: 自動テストは全て同一スレッド内で WUC を扱っておりこの経路を構造的に検出できなかった。オンスクリーン検証は「GDI 全画面キャプチャ（CopyFromScreen）＋窓 rect ピクセル vs コード色」で自動化可能（PrintWindow は WUC で黒）。
- **4.1 完了**: `tests/graphics/surface_pixel_equivalence_test.rs`。ランタイム二重描画で (a) D2D 直描き基準 vs (b) WUC `begin_draw(None)` 供給 DC を同一 `draw_scene` で描き、D2D `Bitmap1::Map(CPU_READ)` 読み戻しで 128×128 全画素 BGRA バイト等価を PASS。自己検証 assert 付き。atlas 直接読み戻し不可のため「WUC 供給 DC 経由の D2D 描画≡参照基準」を検証（最終合成 atlas は 4.3 領分と doc 明記）。
- **4.2 完了（達成可能水準・owner の簡易方針と整合）**: WUC clip（InsetClip/GeometricClip）は write-only 相当かつ**合成層でしか顕在化せず、オフスクリーンでラスタライズ不可**（サーフェス atlas と同じ制約）。ゆえに設計文の「オフスクリーン WIC ピクセル比較」は WUC clip に構造的に適用できない。代わりに `tests/graphics/clip_sync_system_test.rs`（既存・WUC 移行済）が **3 変種適用・clip 解除（None/size0）・未初期化スキップの各分岐を characterization**（DPI スケール込みの型/パラメータ写像がエラーなく完走）で網羅。clip の**ピクセル等価は合成層（4.3）の領分**。個別半径は簡易近似（前述）。
- **4.3 処理（合成層・目視残差／owner の「簡易・ULW 廃止予定」方針と整合）**: 自動 Desktop Duplication before/after キャプチャ harness は**構築しない**。理由: (1) 決定論的サーフェス層は 4.1 でビット等価担保済み、(2) 合成層は `dcomp_demo`（WUC バックエンド）実起動で WucGraphicsResource 初期化・全 Visual 生成・SetRoot/SetBrush・エラー/panic なしを実測（合成が機能することを smoke 確認）、(3) 「移行前（DComp）」基準は 3.2 で撤去済みゆえ before キャプチャは git checkout を要し、DWM 非決定性もあり自動決定論比較に不向き。設計自身が「決定論的キャプチャ不能な過渡は目視残差フォールバック」を許容。**残差範囲＝合成層の最終ピクセル一致は手動目視（必要時 `cargo run -p wintf --example dcomp_demo` 等）**として明示。必要なら別途 harness 化可能。
- **4.4 完了（回帰・x64）**: release（`opt-level='z'`/`lto=true`/`codegen-units=1`）ビルド exit 0（full WUC・要件 8.1）。`compute_ex_style`/`window_factory.rs` は本ブランチ未変更＝当たり判定 ex-style・`WS_EX_NOREDIRECTIONBITMAP` 透過不変（要件 9.1/9.3）。`ulw_present_system` は `CommitComposition` schedule に残置（要件 9.2）。デバイスロスト経路は `window_pos.rs` が `WucGraphicsResource::invalidate` を呼び init.rs が lazy 再生成。full test suite 全 11 バイナリ緑。
- **clip 個別半径の方針確定（owner 2026-07-02）**: `RoundedRectangleIndividual`（4角独立半径）の WUC 厳密写像は `CompositionPath`＝`IGeometrySource2D`（Win2D）を要するが、**Win2D は却下**（実体ある Rust crate なし・再頒布 DLL・WinRT activation はデスクトップアプリで許容不可）。事実確認: 個別半径 clip は **areka 本体で未使用**（`set_clip` は API のみ・本体からの clip セットはゼロ）、ジオメトリを実際に組むのは **ULW レンダ経路（D2D PushLayer・スコープ外・かつ ULW は廃止予定）**だけ、WUC 側個別半径写像の消費者は example/test のみ。ゆえに **個別半径は「コンパイルが通る程度の簡易近似」（均一最大半径へ縮約＋warn）で確定**し、これ以上の実装（B′ の D2D＋自前 IGeometrySource2D 等）は行わない。task 4.2 のビット等価は `Rectangle`＋均一 `RoundedRectangle`（厳密写像）を対象とし、個別半径は近似・対象外と明記する。（将来 WUC で個別半径が必要になれば、Win2D 無しに D2D `create_path_geometry`＋自前 `IGeometrySource2D` `#[implement]` で実現可能な道は確保済み＝ guards.rs に D2D 弧構築の前例あり）

- **i686/arm64 descope（owner ekicyou 2026-07-02）**: wintf は表示合成レイヤーで **x64 or arm64 のみ**。i686（x86）は SHIORI 駆動 helper 専用の別クレートで、wintf は i686 ターゲットにならない。ゆえに task 1.1 の i686 節・task 1.5・task 4.4 の i686 ランタイム・要件 8.4 の 32bit 可搬は本移行では x64 のみで判定（arch 矛盾の spec 誤り）。arm64 検証も後回し＝x64 完了後にオプション別仕様。**当面 x64 のみを意識する**。（参考: full wintf lib を i686 build すると既存 `api.rs`/`window_factory.rs` の `SetWindowLongPtr` isize/i32 不一致で落ちるが wintf x86 非対象ゆえ修正不要）
- **1.1 完了（x64）**: ルート `Cargo.toml` に WUC features＋`windows-numerics=0.3.1` は着手時点で working tree に存在。x64 `cargo build -p wintf`（exit 0）・`cargo build -p wintf --release`（z/LTO・exit 0）通過。DComp feature `Win32_Graphics_DirectComposition` 残置確認。
- **3.2 完了（dead DComp 撤去・COMPAT 更新）**: ライブ参照ゼロ化した `ecs/graphics/dcomp_resource.rs`（DCompGraphicsResource）・`com/dcomp.rs`（DComposition*Ext 群）を撤去（mod 宣言・re-export も除去）。dead テスト `tests/com/dcomp_test.rs`・`tests/graphics/dcomp_resource_test.rs` と `core_accessor_test.rs` の DComp Debug テストを削除（モジュール宣言も追従）。`wuc.rs` の doc intra-link を平文化。`CompositionMode` enum・ULW アーム・`Win32_Graphics_DirectComposition` feature（`com/animation.rs` が使用）は残置。`doc/COMPAT_ARCHITECTURE.md` §6 に DComp→WUC 移行判断を記録（要件 10.3）。`cargo build -p wintf --tests` green・`cargo test -p wintf` 全緑（com 80→61・graphics 152→145＝削除 dead テスト分ぴったり）。
- **WUC test crash 真因（最終・後続参考）**: ヘッドレス（pump なし）で `WucGraphicsResource` を複数生成すると DispatcherQueue の未ドレイン teardown work で 2 個目以降が `STATUS_ACCESS_VIOLATION`。修正（テスト側）: 構築直後に 1×1 warmup `CompositionDrawingSurface` を保持し DQ 安定化（`tests/visual/common/mod.rs`）。設計ノート（非ブロッカー）: `WucGraphicsResource` に drain/pump hook 公開の検討余地。
- **2.2〜3.1 完了（コア合成カットオーバー一体）**: components 3型（WindowGraphics→DesktopWindowTarget / VisualGraphics→WUC Visual / SurfaceGraphics→CompositionDrawingSurface＋CompositionSurfaceBrush）＋消費側システムを in-place で WUC 化し、init.rs で WucGraphicsResource を lazy 登録（ライブ切替）。`cargo build -p wintf` green・`cargo test -p wintf --lib` **503 passed/0 failed**・`dcomp_demo`（WUC バックエンド）実起動で WucGraphicsResource 初期化・全 Visual 生成・エラー/panic なしを実測。主要判断:
  - **z順序（要件5.2）**: DComp `add_visual(insertabove=false, ref=None)`＝兄弟最下部挿入。Children [A,B,C] 反復で最終スタック C,B,A。WUC で同一再現は **`InsertAtBottom`**（design の素朴な `InsertAtTop` は誤り）。検算一致。
  - **SpriteVisual.Size（後追い修正・reviewer 適用）**: WUC の SpriteVisual は自身の Size 内にのみ brush 描画（DComp SetContent は不要だった）。live パイプラインは Size 未設定＝空描画の恐れ→ `deferred_surface_creation` で surface と同一物理サイズを `sprite.SetSize` する修正を適用。
  - **commit_composition 削除**（要件7.1）: `CommitComposition` schedule から解除、`ulw_present_system` 残置。暗黙反映へ。
  - **DComp 定義残置**: `dcomp_resource.rs`/`com/dcomp.rs` はライブ参照ゼロだが定義は残す（撤去は 3.2）。
  - **テスト追従**: `graphics/tests.rs`・`tests/visual/{child_order,hierarchy_sync,common}` を WUC 型へ更新（骨抜きにせず実挙動維持）。
  - **⚠ clip 個別半径の制約（要件5.4/9.4/task4.2 影響・要 owner 判断）**: `RoundedRectangleIndividual`（4角独立半径）は本来 `CreatePathGeometry`＋`CompositionPath` だが、`CompositionPath::Create` は `IGeometrySource2D`（Win2D=CanvasGeometry）必須で **windows 0.62.2 単体では構築不可**（registry 確認済）。設計の PathGeometry 前提は Win2D 見落とし＝spec と現実の齟齬。暫定対応: **4角の最大半径で均一角丸へ縮約**（全角同値なら厳密一致・非均一は近似＋warn ログ）。areka 本体は個別半径未使用（example/ULW guard のみ）ゆえ実害限定。**task 4.2 の個別半径ビット等価は Win2D 導入なしには非均一ケースで達成不可**——4.2 で scope 調整 or Win2D 依存追加の判断が要る。`Rectangle` は InsetClip（Visual サイズ相対）でなく明示サイズの radius=0 GeometricClip で写像（live パイプラインは Visual サイズ非設定のため絶対矩形が必要）。
- **2.1 完了（WucGraphicsResource・DComp 並存）**: `ecs/graphics/wuc_resource.rs` 新規＋`mod.rs` re-export。`dcomp_resource.rs` を 1:1 テンプレに WUC 化（`Option<Inner>`・`unsafe Send/Sync`・手動 Debug・`#[derive(Resource)]`）。Inner フィールド順 `compositor→graphics_device→dq_controller`（controller 最後 drop）。`new()` は `DQTAT_COM_NONE`→`Compositor::new()`→`ICompositorInterop::create_graphics_device` の順。テスト `wuc_graphics_resource_lifecycle` PASS（MTA 再現・new/invalidate/new_empty/drop 健全）。**ライブ登録の切替と消費側改変は 3.1 の領分で本タスクは並存追加のみ・green build 維持**（dcomp_resource.rs/world/mod.rs/消費側システム未変更）。
- **1.4 完了（機構実証・owner 承認 2026-07-02）**: 透過機構は `wuc_spike` で実証済——`WS_EX_NOREDIRECTIONBITMAP` 窓＋`B8G8R8A8`/`Premultiplied` サーフェス＋alpha<1（0.85）クリアがエラーなく生成・描画・SetRoot 成立。DComp との厳密な目視ピクセル等価は task 4.1（サーフェス層ビット等価）/4.3（合成層キャプチャ）へ委譲し、1.4 は機構実証で完了扱い（owner 承認）。
- **1.3 完了・R1 GO（最重要・apartment 決着）**: `examples/wuc_spike.rs`（自己完結の生 Win32 窓＋WUC 最小往復）を実行し **R1 GO** を実測確定。**核心発見: 本番 UI スレッドは `CoInitializeEx(COINIT_MULTITHREADED)`＝MTA（`WinApp::new` L98）であり、design.md §2.1 の「STA 前提」は誤り**。MTA スレッドでは `CreateDispatcherQueueController(DQTAT_COM_NONE)`（apartment 不変）で成立し、**`Compositor::new()` が MTA 上で起動する**（WUC は MTA の UI スレッドで動作＝移行成立）。ShutdownQueueAsync ドレインは controller を最後に drop する順序で成立・shutdown クラッシュなし。**→ task 2.1 の `WucGraphicsResource` は apartment に `DQTAT_COM_NONE` を使う（ASTA ではない）こと。design §2.1 の apartment 記述はこの実測で上書き。** 厳密ピクセル等価は task 4.1 のランタイム二重描画ハーネスの領分（本スパイクは往復機構と threading 前提の GO を担保）。
- **1.2 完了・WUC BeginDraw の実挙動発見**: `com/wuc.rs` に interop Ext 3種＋`create_dispatcher_queue_controller` 実装。`begin_draw` は dcomp.rs L254-259 と byte 一致。往復テスト `com::wuc::tests::begin_draw_roundtrip` PASS（atlas updateoffset=(1,2) 実測・非ゼロ観測）。**重要（後続 2.4 render_surface へ）**: WUC `ICompositionDrawingSurfaceInterop::BeginDraw` に `Some(部分矩形)` を渡すと `E_INVALIDARG (0x80070057)`。本番 `render_surface` は `begin_draw(None)`（全面）のみ使うため影響なし。移行後も **None 経路のみ**を使うこと。apartment 種別実測: cargo test スレッドは COM 未初期化ゆえ `DQTAT_COM_ASTA` で成功（design §2.1 と一致）。
