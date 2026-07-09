# Implementation Plan

## 1. 基盤: クレート雛形と swap chain 供給面の実証

- [x] 1.1 (P) areka-emo-present クレート雛形の作成とワークスペース結線
  - Cargo.toml 作成（依存: areka-emo-compose, areka-emo-atlas, areka-parsers, areka-actor, wintf, bevy_ecs, windows, thiserror, tracing）
  - lib.rs に公開面 re-export と規約 rustdoc（指令 API 契約正本の宣言）
  - crates/Cargo.toml（workspace）にメンバー追加、crates/areka/Cargo.toml に dev-dependency 追加
  - 観測完了: `cargo check -p areka-emo-present` が成功する
  - _Boundary: crate scaffold_

- [x] 1.2 (P) wintf COM層: composition swap chain ヘルパの増分
  - `crates/wintf/src/com/dxgi.rs` に `create_composition_swap_chain(d3d, dxgi, width, height) -> Result<IDXGISwapChain1>` を追加（`IDXGIDevice4`→`GetParent`→`IDXGIFactory2`→`CreateSwapChainForComposition`。flip model・premultiplied alpha・B8G8R8A8・BufferCount=2 固定値）
  - `crates/wintf/src/com/wuc.rs` の `CompositorInteropExt` に `create_composition_surface_for_swap_chain` メソッドを追加（`ICompositorInterop::CreateCompositionSurfaceForSwapChain` の安全ラッパ）
  - unsafe をこの2ヘルパ（wintf COM層）に隔離する
  - 観測完了: 両ヘルパの単体テストが swap chain / `ICompositionSurface` を実際に生成できることを確認する
  - _Boundary: wintf com layer (dxgi.rs, wuc.rs)_

- [x] 1.3 実証: swap chain 供給面の生成→アップロード→リサイズ→readback 往復（GO ゲート）
  - 統合テストとして先行実装する（WARP デバイス可・CI 決定論）
  - 供給面生成→ソーステクスチャへ既知バイト列アップロード→Present→readback で全画素バイト一致を assert する
  - `ResizeBuffers` 実行後の再アップロード→readback 一致も確認する
  - 観測完了: テストが GO（成功）し、以降の本実装（3.1）へ安全に進めることが確認される
  - _Requirements: 6.7, 8.1, 8.3, 8.5_
  - _Depends: 1.1, 1.2_
  - _Boundary: spike integration test_

## 2. コア: 純粋層と wintf 汎用増分

- [x] 2.1 (P) PresentCommand 契約と PresentError の実装
  - `TargetId`・`PresentCommand`（`ShowSurface`/`Hide`/`InvalidateCache`・`#[non_exhaustive]`）・`PresentOutcome`・`PresentError`（thiserror）を `command.rs` に実装する
  - 非表示は `Hide` 専用 variant とし、`surface_id` の番兵値（`-1`）は導入しない
  - `PresentCommand: Send + 'static` の静的アサートテストを追加する
  - 観測完了: `PresentCommand::Hide` 等の各 variant が構築でき、Send 境界の static assert がコンパイルを通る
  - _Requirements: 3.1, 3.3, 3.5, 3.6, 4.3, 7.2_
  - _Boundary: PresentCommand (command.rs)_

- [x] 2.2 (P) ComposeCache の実装
  - `CacheEntry { composed: ComposedSurface, mask: AlphaMask }` と、合成入力（`surface_id`＋`BindSet`）完全一致キーの容量1メモ化スロット `slot: Option<(ComposeKey, CacheEntry)>` を実装する（2026-07-09 改訂: 旧 `HashMap<u32, CacheEntry>` 全保持は bind 差分で古い合成に衝突する仕様バグ。詳細は Implementation Notes）
  - エントリ挿入時に `AlphaMask::from_pbgra32` でマスクを1回だけ生成する（表示のたび再生成しない）
  - `invalidate_all()` を実装する
  - 観測完了: 完全一致 hit 時に `Composer` が呼ばれないこと・同一 surface id でも bind 集合が異なれば必ず再合成されること（呼出カウンタと bind 差分ミスの回帰檻）・`invalidate_all` 後は再合成されることを単体テストで確認する
  - _Requirements: 2.1, 2.4, 4.1, 4.2, 4.4_
  - _Boundary: ComposeCache (cache.rs)_

- [x] 2.3 (P) wintf hit-test への AlphaMaskResource 優先読み込み増分
  - `AlphaMaskResource`（`Component`・`AlphaMask` 内包・`set`/`mask()` アクセサ）を `hit_test/mod.rs` に新設する
  - `hit_test_entity` と `hit_test_entity_ex` の両 `HitTestMode::AlphaMask` 分岐で、共有ヘルパへ抽出した読み出しを介し `AlphaMaskResource` を最優先し、なければ `BitmapSourceResource` へフォールバックする（既存挙動は完全後方互換）
  - 観測完了: 「`AlphaMaskResource` あり→優先・なし→既存経路」を `hit_test_entity` 直接呼びと `hit_test_in_window` 経由（`hit_test_entity_ex`）の両方で確認する単体テストが通る
  - _Requirements: 2.2, 2.3, 2.5_
  - _Boundary: wintf hit_test module (AlphaMaskResource)_

- [x] 2.4 (P) emo-atlas の 0 寸/全透明 element 検出時の警告ログ増分
  - bake 時に、トリム後 0 寸または元画像 0 寸の element を検出したら `warn!` を発火する（動作・bake 結果は不変・ログのみの増分）
  - 観測完了: tracing capture で、全透明/0 寸 element を含む bake 実行時に warn ログが1回以上発火することを確認する単体テスト
  - _Boundary: emo-atlas bake（設計ディスカッション#1 由来の増分。対応する数値要件なし）_

## 3. コア: 表示層コンポーネント

- [x] 3.1 (P) SwapChainPresenter の本実装
  - swap chain 生成（`CreateSwapChainForComposition`・flip model・premultiplied・B8G8R8A8・BufferCount=2）を実装する
  - `source_tex`（単一の真実源）経由のアップロード（`UpdateSubresource`→`CopyResource`→`Present`）と readback（`CopyResource`→staging→`Map`）を実装する（D2D 非経由の純バイト転送）
  - `ResizeBuffers` によるリサイズ規則（backbuffer 参照解放後に実行）を実装する
  - 観測完了: `upload` 直後の `read_back()` が同じ `ComposedSurface.bytes()` とバイト一致する統合テストが通る
  - _Requirements: 1.1, 1.2, 1.5, 8.1, 8.2, 8.3, 8.4, 8.5_
  - _Depends: 1.2, 1.3_
  - _Boundary: SwapChainPresenter (chain.rs)_

- [x] 3.2 (P) VisualMount の実装
  - 窓 Entity への最小 visual 構成（surface entity + SpriteVisual + `HitTest::alpha_mask()` + `AlphaMaskResource`）と text-layer 予約スロット（兄弟・上位 z の空 entity）の装着を実装する
  - surface entity の `Arrangement`/`GlobalArrangement.bounds` を物理 px で直接確立する（`BoxStyle`/taffy 非経由）
  - 非表示切替（`Visual::set_visible(false)` + `HitTest::none()`）を実装する
  - 観測完了: 装着後に text-layer スロットが surface visual の兄弟・上位 z として存在し、非表示切替で `HitTest::none()` へ切り替わることを単体テストで確認する
  - _Requirements: 1.3, 1.4, 1.6, 3.3_
  - _Depends: 2.3_
  - _Boundary: VisualMount (mount.rs)_

- [x] 3.3 (P) BalloonFrameSource の実装
  - `balloons{N}.png` から synthetic surfaces.txt テキストを生成する
  - `areka_parsers::shell::parse` → `SurfaceSet`（`use_self_alpha,1` 相当の `AlphaParams`）→ `areka_emo_atlas::bake` → `EmoWorld::build`+`bind_atlas` の経路を実装する（シェルと同一機構・直 WIC バイパスなし）
  - M-boot の入力を枠画像のみに限定する（`balloonc*`/`arrow*`/`marker`/`online*` は列挙対象外）
  - 観測完了: synthetic テキスト→parse の往復で element path/surface id が転記一致することを単体テストで確認する
  - _Requirements: 5.1, 5.2, 5.3_
  - _Boundary: BalloonFrameSource (balloon.rs)_

## 4. 統合: 指令適用の統括と example 結線

- [x] 4.1 EmoPresenter の実装（統括・EmptyComposition 処理）
  - `PresentTarget`（world/atlas/composer/cache/mount/chain/visible）の管理と `attach_target`/`apply`/`read_back` を実装する
  - `ComposeError::EmptyComposition`（全透明退化）を `warn!` + Hide 相当への縮退 + reply `Ok` として一意に処理する（skip 解釈は採らない）
  - 解決不能な `surface_id` は `error!` + 当該指令 skip + 表示不変 + reply `Err` とする
  - NonSend 資源として UI スレッド専有を型で強制する
  - 観測完了: `attach_target`→`ShowSurface`→`read_back` が golden bytes と一致し、不正 `surface_id` 指定時は表示 bytes が不変のまま `Err` が返る統合テストが通る
  - _Requirements: 2.1, 2.4, 3.2, 3.4, 7.1, 7.2_
  - _Depends: 2.1, 2.2, 3.1, 3.2_
  - _Boundary: EmoPresenter (presenter.rs)_

- [x] 4.2 example クレート結線と mock-shell donor からの窓生成移植
  - `crates/areka/examples/emo-present.rs` を新設し、mock-shell の窓生成（WS_POPUP・透過 ex-style）・`register_click_through_windows`（`Added<WindowHandle>`）を移植する
  - シェル窓（target 0・emo2 surface0）とバルーン窓（target 1・balloons0.png、`BalloonFrameSource` 経由）の2窓構成を実装する
  - `main.rs` は変更しない
  - 観測完了: example 実行でシェル surface0 とバルーン枠の2窓が表示される
  - _Requirements: 6.1, 6.6_
  - _Depends: 4.1, 3.3_
  - _Boundary: example (emo-present.rs)_

- [x] 4.3 指令切替とアンカーオフセット配置の実装
  - タイマー駆動で surface0 ⇄ surface1000（`BindSet::from_ids([1100,1200,1302])`）⇄ `Hide` を巡回する切替ロジックを実装する
  - shell descript の `sakura.balloon.offsetx/offsety` を `areka_parsers::kv` で読み、バルーン窓位置に反映する（無指定時は既定整列）
  - 観測完了: example 実行で数秒周期の surface 切替と非表示化が視認でき、バルーンが指定オフセット位置に配置される
  - _Requirements: 3.2, 5.4, 6.4_
  - _Depends: 4.2_
  - _Boundary: example (emo-present.rs)_

## 5. 検証: E2E 観測と実 DPI 確認

- [x] 5.1 起動時 golden バイト一致 assert の実装
  - `apply(ShowSurface)` 直後に `read_back(target)` し、`ComposedSurface.bytes()` とのバイト一致を `assert_eq!` で検証する実装をシェル・バルーン両 target に組み込む
  - 観測完了: example 起動時に golden 不一致があれば即 panic する assert が実装され、一致時は正常起動する
  - _Requirements: 6.2, 6.7, 8.2, 8.3_
  - _Depends: 4.2_
  - _Boundary: example (emo-present.rs)_

- [x] 5.2 クリック透過の実挙動確認
  - 不透明域クリックのログ/視覚反応、透明域クリックの背後プロセスへの透過を example 上で確認する
  - 観測完了: 不透明域クリックはログに捕捉が記録され、透明域クリックは背後アプリのウィンドウが反応することを手動観測で確認する
  - _Requirements: 2.2, 2.3, 6.3_
  - _Depends: 4.3_
  - _Boundary: example (emo-present.rs)_
  - _Verified (2026-07-09・開発者手動観測@125% DPI): 不透明域クリック→捕捉ログ2回発火（`client(226,278)`・`(220,425)`・αマスク有効域着地）／透明域クリック→当アプリはログ無し・背後ウィンドウが反応（クリックスルー成立）を確認。カーソル形状の region 別変化は仕様外（本 spec は機能透過を保証）。_

- [x] 5.3 実 DPI（dpi≠96）実行での確認と記録
  - dpi≠96 のモニタ/スケーリング設定で 5.1/5.2/4.3 の巡回・クリックを再実施する
  - 実 DPI 実行手順を example の rustdoc に明記する
  - 観測完了: 実 DPI 環境での表示等倍・クリック座標一致を実機確認記録として残す（dpi=96 のみでは完了と見なさない）
  - _Requirements: 1.6, 2.5, 6.5_
  - _Depends: 5.1, 5.2_
  - _Boundary: example (emo-present.rs)_
  - _Verified (2026-07-09・開発者手動観測@**125% DPI＝dpi≠96**): (a)表示等倍＝surface0 焼き込みテキスト「アヒルやアヒル！…」がボケ/にじみ無くくっきり描画（DPI 仮想化なし）・実行ログの窓 bounds が surface 物理 px と厳密一致（surface0=434×687→(400,200)-(834,887)・surface1000=382×547→(400,200)-(782,747)）。(b)起動時 golden assert が両 target で 125% でも非 panic 通過。(c)クリック捕捉/透過が見た目の絵と座標一致（R2.5 恒等写像）。dpi=96 のみでない実 DPI 記録として成立。_

- [x] 5.4 (P) EmoPresenter のエッジケース回帰テスト
  - 不正 `surface_id` 指定時の skip+表示不変、`EmptyComposition`→Hide 縮退+reply `Ok`、`Hide`→再 `ShowSurface` での復帰、をそれぞれ統合テストとして実装する
  - 観測完了: 3ケースすべてが期待どおりの表示/reply/当たり判定状態になることを統合テストで確認する
  - _Requirements: 3.3, 3.4_
  - _Depends: 4.1_
  - _Boundary: EmoPresenter (presenter.rs)_

## Implementation Notes

- 1.2: 設計/タスク本文の「`IDXGIDevice4::GetParent::<IDXGIFactory2>()` でファクトリ取得」は誤り（デバイスの `GetParent` はアダプタを返し `E_NOINTERFACE`）。正準経路は `dxgi.GetAdapter()` → `adapter.GetParent::<IDXGIFactory2>()`。3.1 の SwapChainPresenter はこのヘルパ経由で生成するため同経路。wintf テストは WARP でなく実 HW `GraphicsCore` デバイスを使う既存 fixture 流儀（`begin_draw_roundtrip`）に追随。

- 3.x（3.1/3.2/3.3）: 表示層コンポーネントは `pub(crate)`。非 test 消費者（`EmoPresenter`＝4.1）着地まで `cargo check` に dead_code 警告が出る（各タスク緑判定では既知・許容）。4.1 完了で解消。
- 3.2: wintf の z 順に矛盾あり（レンダリング `visual_sync.rs`＝Children 先頭が最前 / hit-test `tree_iter.rs` DepthFirstReversePostOrder＝Children 末尾が最前）。VisualMount は「text 層は surface の上に描画」の設計意図＝**レンダリング権威**に従い text-slot を先頭子（＝描画上位）に置く。slot は `HitTest` 非搭載ゆえ hit-test 巡回順の差異は現状無害。emo-text-layer が slot に実 HitTest を載せる際はこの矛盾を要再確認（wintf 側の課題・本 spec 境界外）。brush 衝突は `GraphicsCommandList` 不挿入で `deferred_surface_creation_system` が発火せず・有効 `VisualGraphics` 同梱で on_add 上書きなし＝生存確認済み。
- 3.3: `build_balloon_target` の失敗経路（read_dir I/O 失敗・枠なし・`baked.errors` 非空）は全て `tracing::error!`＋`Err(PresentError::Compose(EmptyComposition(0)))` へ写像（`PresentError` に専用 I/O/decode variant がないため）。握り潰しなし・誤成功なし・真因はログ。build-time 呼びゆえ 4.2 の呼び手は Err を受けたら error ログを確認すること。`baked.errors` 非空は hard-fail（emo-atlas の寛容 survivor 方針より厳格・M-boot の固定小枠集合前提）。
- 4.2: example の UI スレッド駆動シーム＝`EmoBoot`（NonSend 資源・UI スレッドの `CommandSender` closure 内で presenter/assets 生成）＋`boot_present_system`（`FrameFinalize` の exclusive `&mut World` system）。`GraphicsCore` 存在＋`WucGraphicsResource::is_valid()` を待って一度だけ attach_target＋apply（`attached` フラグで one-shot）。GPU 資源は複数フレーム遅延で着地するため即時 apply は不可＝この待機が必須。4.3/5.1 はこの system を拡張する。fixture は `env!("CARGO_MANIFEST_DIR")`＋`../pilot/examples/shiori-host-32/fixtures/emo2/`。emo2 shell の `purple/a/null.png`（α無 PNG）は normalize seam で warn 継続（surface0 未使用）。**「2窓表示」は開発者の `cargo run -p areka --example emo-present` 実機確認が必要（headless 検証外・5.x で実 DPI 込み確認）**。
- 3.2/validate: emo-present 依存に `windows-numerics`（workspace pin）を追加（`SpriteVisual::SetSize(Vector2)` 用・wintf と同用途）。design「Allowed Dependencies」明示リスト外だが境界違反でない良性追加＝横断監査で GO 確認済み。design 追補候補（非ブロッキング）。
- validate-impl（2026-07-09）: フィーチャレベル GO（自動検証範囲）。機械: emo-present 21 lib+1 spike / emo-atlas 全 / wintf 550 lib 緑（exit 0）・example build 緑・marker grep clean。横断監査 CRITICAL NONE（依存方向 OK・File Structure MATCH・境界 OK・unsafe 隔離 OK・要件 8/8 群網羅）。**残: 5.2/5.3 の実機ランタイム観測（手動・dpi≠96 込み）のみ＝MANUAL_VERIFY_REQUIRED**。dead_code 3件は予約シーム（`SwapChainPresenter::size`・`VisualMount::text_slot`）で許容。
- 手動観測完了（2026-07-09・開発者@125% DPI）: 5.2/5.3 とも実機確認済み→`[x]`。**feature 全 17 サブタスク完了**。fixture 知見: emo2 shell の `surface0.png`（434×687）は**バルーン焼き込みのサンプル立ち絵**（キャラ＋セリフ入り吹き出し「アヒルやアヒル！…」が1枚に一体化）＝伺かの挨拶用デフォルト立ち絵の慣習。emo-present は当画像を忠実表示するのみ（テキスト描画機能ではない・emo-text-layer 別 spec は不変）。cycle が surface1000（bind 合成・焼き込みバルーン無し）へ移ると当該バルーンは消える。この「焼き込みバルーン」と emo-present の別 balloon 窓（`balloons0.png`＝空枠・内側 A=255 不透明白）は別物。
- キャッシュ仕様バグ是正（2026-07-09・開発者指摘）: R4.1 初版「surface id をキーに」は**要求仕様そのものの欠陥**だった。合成結果は合成入力（surface id＋BindSet）の純粋関数なのに surface id 単独キーでは bind 差分（着せ替え・まばたき）が古い合成にヒットし表示が更新されない（まばたきデモ「開きっぱなし」で顕在化。example に `InvalidateCache` を挟む応急処置を一時導入したが恒久策として棄却・撤去）。是正: `ComposeCache` を **合成入力（surface_id＋BindSet）完全一致キーの容量 1 メモ化スロット**へ再設計（`slot: Option<(ComposeKey, CacheEntry)>`）。多エントリ全保持は「将来 seriko がアニメ pattern 状態を合成入力へ加えると状態空間が膨張＝原寸ビットマップのメモリ堆積×低ヒット率」ゆえ不採用（開発者方針「状態が変わらないなら前回画像を継続・変わったら必ず再合成」）。将来 pattern 状態を合成入力へ加える際は `ComposeKey` へ追加する（キー＝合成入力の全体、の不変条件維持）。回帰檻: cache 単体 `different_binds_on_same_surface_must_miss`／実表示 `bind_change_on_same_surface_updates_display`（往復 golden 一致）。requirements R4・design（境界/フロー/トレサビ/コンポーネント/Domain Model/テスト戦略）改訂済み。emo-present 24 lib＋1 spike 緑。
- validate-impl 再検証（2026-07-09・キャッシュ改訂後）: **GO**。機械: emo-present 24 lib＋1 spike／emo-atlas 75(1 ignored)／emo-compose 135／wintf 550 lib すべて緑（exit 0）・example build 緑・TODO/FIXME/todo! 残渣 CLEAN・秘密 grep CLEAN。独立監査（横断）: 要件カバレッジ MATCH（R1〜R8 全群充足・改訂 R4.1/4.2 がコードで実現）・設計整合 MATCH・依存方向/境界 OK（禁止依存 tokio/dola/kanade/sakura 皆無・windows-numerics は既知の良性宣言外）・クロスタスク結線 OK（cache 新 API `get(id,&binds)`/`insert(id,binds,..)` 全呼び出し整合・example の InvalidateCache 応急処置は完全撤去）。CRITICAL NONE。task 2.2 本文の旧設計記述残存（唯一の指摘・documentation-only）は本再検証で是正済み。フィーチャレベル統合に阻害要因なし。
