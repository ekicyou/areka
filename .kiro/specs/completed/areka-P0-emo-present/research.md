# ギャップ分析: areka-P0-emo-present

> 対象仕様: `.kiro/specs/areka-P0-emo-present/`（requirements.md 確定済み・spec.json language=ja）
> 種別: 本坑（main）。⑥ emo 直列チェーン 3/3（emo-atlas ✅ → emo-compose ✅ → **emo-present**）
> 実施日: 2026-07-05 / 手法: 既存コードベースの Grep/Glob/Read 精査 + ukadoc カテゴリ確認
> 位置づけ: 情報提供（設計判断は要件ディスカッション／design へ委ねる）

---

## 1. 要旨（3–5 行）

- **emo-present は新設クレート**（`crates/areka-emo-present` は未存在）。上流 emo-compose（`ComposedSurface` premultiplied BGRA・`Composer::compose_into`）と emo-atlas（`AtlasTable`）は実シンボルとして完成・再定義禁止。表示・当たり判定・クリックスルー基盤は wintf に実在するが、**いずれもファイル読込（WIC）起点**であり、本ユニットが要求する**メモリ供給経路が存在しない**——ここが最大のギャップ。
- **表示口の欠落**: 既存 `BitmapSource` は `path: String` → `IWICBitmapSource` → `ID2D1Bitmap1` → D2D CommandList → WUC `CompositionDrawingSurface`（B8G8R8A8 premultiplied）という一本道で、**メモリバッファ（`&[u8]`）から surface を埋める入口が無い**。WIC にもメモリ構築ヘルパ（`CreateBitmapFromMemory` 相当）は未ラップ。要件 1（メモリ供給）実現には新経路が要る。
- **AlphaMask 生成は流用可能**: `AlphaMask::from_pbgra32(pixels, w, h, stride)` が premultiplied BGRA から直接ビットマスクを生成でき、`ComposedSurface` の出力形式と完全一致。ただし**既存の生成システムは `BitmapSourceResource` 前提**で、当たり判定（`hit_test_entity` の `HitTestMode::AlphaMask`）も `BitmapSourceResource` から α マスクを読む。emo-present の合成結果を hit-test に届ける結線は新規に要る（要件 2）。
- **クリックスルー基盤は所有権不要で載る**: `ClickThroughRegistryHandle`（NonSend）へ窓 Entity + HWND を登録すれば、`WS_EX_TRANSPARENT` 動的トグルが `hit_test_in_window`→AlphaMask 経由で機能する。**mock-shell が donor**として窓生成・登録の完全な手本を提供済み（`crates/areka/examples/mock-shell.rs`）。実 DPI は `screen_to_client_point`（OS `ScreenToClient` 委譲）で既に担保。
- **指令 API・キャッシュは純粋な emo ランタイム層の新設**（wintf 非依存で書ける部分）。seriko-engine（並走）が呼び手で、`\s[-1]` 非表示の意味論が両 design の突合点。

---

## 2. 現状調査（Requirement-to-Asset マップ）

### 2.1 上流 emo 実シンボル（再定義禁止・消費のみ）

| 資産 | 場所 | 契約 | emo-present での用途 |
|---|---|---|---|
| `ComposedSurface` | `crates/areka-emo-compose/src/composed.rs` | opaque・`width/height/stride(=w*4)/bytes` の accessor・`bytes()`/`into_bytes()`・premultiplied BGRA・`Send`・`Clone`・`Default` | 表示バッファ源・AlphaMask 源（キャッシュが所有） |
| `Composer::compose_into` | `crates/areka-emo-compose/src/lib.rs:111` | `(&mut ComposedSurface, &EmoWorld, &AtlasTable, surface_id:u32, &BindSet) -> Result<(),ComposeError>`・**out 再利用でゼロアロケーション**・結果を保持しない | 指令適用時の再合成（キャッシュミス時） |
| `Composer::compose` | 同上:141 | 値返し版（初回・便宜） | 初回合成 |
| `BindSet` | `crates/areka-emo-compose/src/bind.rs` | `from_ids(impl IntoIterator<Item=u32>)`・昇順整列 dedup・`Send`・`Clone` | 指令 API の bind ペイロード |
| `EmoWorld` / `AliasMap` / `AtlasBinding` | `crates/areka-emo-compose/src/world.rs`（`lib.rs:40` で再export） | `EmoWorld::build(&Shell)`＋`bind_atlas(&AtlasTable)`（resolve は構築時一度きり） | 合成入力の常駐 World（本層 or 上位が所有） |
| `AtlasTable` | `crates/areka-emo-atlas/src/table.rs`（`lib.rs:140` 再export） | `resolve/entry/key/pages/len`・premultiplied BGRA 頁 | 合成入力（不変・キャッシュ無効化のトリガ源） |

**設計的含意**: emo-compose のドキュメントが明示的に「surface→結果のキャッシュ・無効化は下流 emo-present の責務」と記す（`composed.rs` 冒頭・`lib.rs:59`）。**out バッファ再利用経路がキャッシュ層の設計に好適**（本層が out を所有すれば定常状態ゼロアロケーション）。

### 2.2 wintf 表示・当たり判定・クリックスルー基盤

| 資産 | 場所 | 現状 | ギャップ |
|---|---|---|---|
| `BitmapSource`（表示ウィジェット） | `crates/wintf/src/ecs/widget/bitmap_source/` | `path:String` 起点・on_add で `Visual+BitmapSourceGraphics+HitTest::alpha_mask()` を自動挿入・非同期 WIC ロード | **メモリ供給の入口が無い**（要件 1・最重要ギャップ） |
| WIC ロード | `crates/wintf/src/com/wic.rs:28` | `load_bitmap_source(factory, &Path)`→`(IWICBitmapSource, has_alpha)`・PBGRA32 変換 | ファイル専用。`CreateBitmapFromMemory` 等のメモリ構築ラッパー未実装 |
| D2D 描画 | `bitmap_source/systems.rs:99` `draw_bitmap_sources` | `IWICBitmapSource`→`ID2D1Bitmap1`（`CreateBitmapFromWicBitmap`）→CommandList→WUC surface | メモリバッファ直描の経路が無い |
| WUC surface 作成 | `crates/wintf/src/ecs/graphics/systems/surface.rs:81` | `CreateDrawingSurface(size, B8G8R8A8UIntNormalized, Premultiplied)`＋`CreateSurfaceBrushWithSurface`＋`SpriteVisual.SetBrush/SetSize` | 形式は `ComposedSurface` と完全一致。**流用の的**。ただし現状は CommandList 経由でしか埋まらない |
| `AlphaMask` | `bitmap_source/alpha_mask.rs:33` | `from_pbgra32(&[u8], w, h, stride)`・`is_hit(x,y)`・`Send+Sync` | **そのまま流用可**（`ComposedSurface.bytes()` を直接渡せる） |
| α マスク生成システム | `bitmap_source/systems.rs:332` `generate_alpha_mask_system` | `BitmapSourceResource` + `HitTestMode::AlphaMask` 前提・WIC から copy_pixels | emo-present の合成結果を α マスク化して hit-test へ届ける新結線が要る |
| 当たり判定 | `crates/wintf/src/ecs/layout/hit_test/mod.rs:164` `hit_test_entity` | `HitTestMode::AlphaMask` は **`BitmapSourceResource.alpha_mask()` からのみ**マスクを読む・bounds 相対座標→マスク座標に変換 | emo-present の α マスクを供給する型が `BitmapSourceResource` でない場合、hit-test 側の読み口が要る（設計判断） |
| クリックスルー機構 | `crates/wintf/src/ecs/clickthrough/` | `ClickThroughRegistryHandle`（**pub**・NonSend）で窓登録→`evaluate_targets` が `hit_test_in_window`→AlphaMask で `WS_EX_TRANSPARENT` トグル・`WS_EX_LAYERED` 同伴・OS `ScreenToClient`（実 DPI 対応） | **所有権不要で載る**。窓登録は mock-shell が手本 |
| donor（窓生成・登録） | `crates/areka/examples/mock-shell.rs` | WS_POPUP 透過窓・`register_click_through_windows`（`Added<WindowHandle>` で 1 回登録）・ドラッグ／ダブルクリック終了 | example の窓生成テンプレートとして流用（要件 6.6） |

### 2.3 規約・構造

- **クレート命名**: `areka-*`（`areka-emo-atlas`/`areka-emo-compose` に倣い `areka-emo-present`）。ワークスペース分割は責務ごと（structure.md）。
- **依存規約**: emo-compose の Cargo.toml が範型——`areka-parsers`/`areka-emo-atlas`/`bevy_ecs`/`tracing`/`thiserror`。emo-present は加えて **wintf 依存が新規に発生**（表示口が wintf を知る唯一の層）。これは brief の「表示口＝wintf を知る唯一の層」と整合。
- **ログ規律**: silent failure 禁止・失敗は `error!`/`warn!`＋`Err`／skip・panic は致命限定（記憶 areka-log-first・要件 3.4）。
- **UI スレッド固定**: WUC 更新は UI スレッド（MTA＋`DQTAT_COM_NONE`・記憶 areka-wuc-runs-on-mta-thread・要件 7）。合成（CPU）を worker で行う場合は channel/queue で UI スレッドへ（要件 7.2・既存 `WintfTaskPool`＋`CommandSender`（mpsc）が範型）。
- **premultiplied 一貫**: `ComposedSurface`・`AlphaMask::from_pbgra32`・WUC surface（`Premultiplied`）・D2D bitmap（`D2D1_ALPHA_MODE_PREMULTIPLIED`）が全て premultiplied BGRA で一致（要件 1.2・途中変換不要）。

---

## 3. 実現可能性分析（要件別ギャップ）

| 要件 | 技術的必要物 | ギャップ種別 | メモ |
|---|---|---|---|
| R1 メモリ供給表示 | ComposedSurface bytes → WUC surface へアップロードする経路 | **Missing** | 最大の新規実装点。§4 に候補 3 案 |
| R1.4 最小 visual 構成＋text-layer 予約口 | 窓あたり surface visual 1 枚＋独立 text 層スロット | Missing（構造） | brief クロスユニット契約「text-layer スロット予約」。VisualManager の子挿入で表現可（`visual_manager.rs` 既存） |
| R1.5 原寸追随 | surface サイズ変化時の窓/visual リサイズ | Constraint | 既存 `deferred_surface_creation_system` はサイズ不一致で再作成する（surface.rs:139）が、それは `GlobalArrangement` 駆動。メモリ供給経路では明示リサイズ規則が要る |
| R1.6 / R2.5 実 DPI 等倍 | 合成＝物理 px 等倍・表示側の論理/物理帰属確定 | **Unknown（Research）** | wintf 座標契約（`Monitor.work_area`/`WindowPos`=物理・`BoxStyle`=論理・記憶 dpi-coordinate-defect）に乗せる。design で確定・実 DPI 実行必須 |
| R2 AlphaMask 生成・同期 | ComposedSurface → AlphaMask・hit-test 供給 | Partial（`from_pbgra32` 流用可・供給結線 Missing） | 型は完成。読み口（`BitmapSourceResource` か新型か）が設計判断 |
| R2.4 表示/マスク原子性 | バッファとマスクを対で入替 | Missing（構造） | キャッシュエントリに (bytes, mask) を同梱すれば構造で担保 |
| R3 指令 API | scope+surface_id+BindSet を運ぶ Send 所有データ・reply 口 | Missing | wintf 非依存で書ける純粋部。`areka-actor` の envelope／`spawn_ui`/`UiSender` に将来載る形（借用なし・enum 1 variant 転写可・reply Sender 同梱） |
| R3.3 `\s[-1]` 非表示 | 非表示遷移の意味論 | **Unknown（Research）** | seriko-engine brief と突合（両 design）。ukadoc `\s[-1]`=非表示サーフェス明記 |
| R4 合成キャッシュ・無効化 | surface_id→(結果) の保持＋無効化口 | Missing | emo2 規模は全保持で可（brief）。out バッファ所有と好相性 |
| R5 バルーン枠表示 | balloons*.png を同一機構で表示 | Partial（同一表示口・入力経路が別） | fixture 直指定。PNG α 尊重（`ComposedSurface` 経由 or 直 WIC ロードかは設計判断）。offsetx/y 配置は R5.4 |
| R5.3 枠のみ（arrow/marker/online 除外） | 役割分担 | Constraint | M-boot は枠のみ。ukadoc `descript_balloon` 参照は design 冒頭で（brief 必読指示） |
| R6 観測 example（実 DPI） | mock-shell donor・golden 一致・クリック透過・切替観測 | Missing（example） | `main.rs` 不変・`examples/` に新設。donor 完備 |
| R7 更新スレッド規律 | UI スレッド更新・worker→UI channel 引渡し | Constraint | 既存 `WintfTaskPool`/`spawn_ui`/`UiSender` が範型 |

---

## 4. 実装アプローチの選択肢（最重要ギャップ = メモリ供給表示口）

emo-present の本質的新規点は「`ComposedSurface`（`&[u8]` premultiplied BGRA）を WUC surface へ載せる」経路である。3 案を提示する。

### Option A: 既存 BitmapSource 経路を拡張（メモリ源を注入）

`BitmapSource` / `BitmapSourceResource` / `draw_bitmap_sources` を「ファイル源 or メモリ源」の両対応に拡張する。メモリ源は `IWICBitmap`（`IWICImagingFactory::CreateBitmapFromMemory`）を新設ラッパで作り、以降は既存の `CreateBitmapFromWicBitmap`→CommandList→WUC 経路へ合流させる。

- 変更対象: `crates/wintf/src/ecs/widget/bitmap_source/`（`resource.rs`・`systems.rs`・`bitmap_source.rs`）＋`com/wic.rs`（メモリ構築ヘルパ追加）。
- ✅ WUC surface 作成・SpriteVisual・α マスク生成・hit-test 供給を**丸ごと再利用**（結線コストが最小）。
- ✅ AlphaMask も既存 `generate_alpha_mask_system` に相乗り。
- ❌ wintf 本体（他 spec の資産）を emo 都合で改変＝レイヤ侵食。BitmapSource は「画像パスウィジェット」の単一責務が濁る。
- ❌ D2D bitmap への一段（`CreateBitmapFromWicBitmap`）が挟まり、emo が既に premultiplied で持つバッファを WIC→D2D と二度写す無駄。
- 適合度: 「最小実装で早く golden を出す」には有効。ただし brief の「表示口＝emo 側の層」思想とややズレる。

### Option B: emo-present に専用のメモリ供給ウィジェット/システムを新設

emo-present クレート内に、`ComposedSurface` を受けて **WUC surface へ直接書く**（`CompositionDrawingSurface`→interop で `ID2D1DeviceContext`／`ID2D1Bitmap1` を得て `CopyFromMemory`／`DrawBitmap`）専用の表示コンポーネント＋システムを持つ。窓 Entity へ「surface 本体 visual＋text-layer 予約スロット」を最小構成で組む。

- 新規: `crates/areka-emo-present/`（wintf の graphics/visual/surface 資産を **利用**するが、BitmapSource には触れない）。
- ✅ brief の層境界に忠実（表示口が emo の唯一の wintf 接触層・BitmapSource 不侵）。
- ✅ premultiplied バッファを WIC を経ずに surface へ直接（`ID2D1Bitmap1::CopyFromMemory` or memory WIC bitmap を一度だけ）——変換段を削れる。
- ✅ キャッシュ（surface_id→(bytes, mask)）を本層で所有し、指令 API・原子入替・無効化を自然に同居。
- ❌ WUC surface interop（`CompositionDrawingSurface`→`ICompositionDrawingSurfaceInterop::BeginDraw`/`EndDraw` で `ID2D1DeviceContext` 取得）の新規結線が要る＝wintf 内部 API の露出／再利用範囲を design で確定する必要（**Research**）。
- ❌ 既存 α マスク生成・hit-test 供給を BitmapSource から切り離す分、その結線を自前で持つ。
- 適合度: 方針正本（emo 自前・層分離）に最も忠実。中〜やや高コスト。

### Option C: ハイブリッド（wic メモリ構築ヘルパは wintf/com、表示層は emo-present）

低レベルの「メモリ→表示可能リソース」ヘルパ（`com/wic.rs` に `create_bitmap_from_memory` 相当・または WUC surface interop の薄いユーティリティ）だけを wintf の COM 層へ足し（汎用・emo 非依存）、その上の**表示コンポーネント・キャッシュ・指令 API・AlphaMask 同期は emo-present に新設**する。

- ✅ COM ボイラープレートは wintf の既存 COM 層規約に収め（unsafe 隔離・structure.md「unsafe隔離」）、emo-present は安全 API のみ触る。
- ✅ 層分離（Option B の利点）＋既存 COM 資産流用（Option A の一部利点）。
- ✅ 将来 seriko/window-placement 結線時に emo-present の指令 API が独立して安定。
- ❌ 2 クレート（wintf COM 層＋emo-present）に跨る変更＝計画が最も緻密に要る。
- 適合度: 「最小の wintf 改変＋emo 側に責務集約」で brief 思想と現実的コストの均衡点。**有力候補**。

> いずれの案でも共通の設計確定事項: ① WUC surface のリサイズ規則（原寸追随・R1.5）② AlphaMask を hit-test へ届ける型（`BitmapSourceResource` 再利用か emo 専用 α 供給コンポーネントか）③ text-layer 予約スロットの visual 構成上の位置。

### Option D: コンポジター供給面を自前 swap chain にする（読み戻し可能ルート・調査 2026-07-05）

現行 wintf は `CompositionDrawingSurface`（`CreateGraphicsDevice`→`CreateDrawingSurface`→`ICompositionDrawingSurfaceInterop::begin_draw`）でコンポジターへ供給する。これは **WUC 内部アトラステクスチャへ書く書き込み専用経路**で、`begin_draw` が返す `updateoffset` がアトラス内配置＝**最終合成面を直接読み戻せない**（`tests/graphics/surface_pixel_equivalence_test.rs` が明言：「atlas から直接読み戻すことはできない」「検証していない: compositor の atlas 上の最終合成結果」）。

**代替**: `ICompositorInterop`（wintf が既に `wuc.rs` で cast・使用）は自前コンテンツ供給の入口を2つ持つ:
- `CreateCompositionSurfaceForSwapChain(swapchain) -> ICompositionSurface`（同一プロセス・本命）
- `CreateCompositionSurfaceForHandle(handle) -> ICompositionSurface`（クロスプロセス・不要）

`IDXGIFactory2::CreateSwapChainForComposition`（HWND 無し・flip model・`DXGI_ALPHA_MODE_PREMULTIPLIED`）で合成用 swap chain を作り、backbuffer に `ComposedSurface` を載せ、`CreateSurfaceBrushWithSurface` で SpriteVisual へ貼る。材料は `GraphicsCore::d3d()`（`ID3D11Device`）/`dxgi()`（`IDXGIDevice4`）で完備。

**利点**:
- **表示面を自前所有＝読み戻し可能**（`CopyResource`→CPU_READ staging→`Map`）。表示とヒットテストが**単一の真実源**になり得る。
- **将来の「画像を GPU へ直読み（CPU バイトを経ない）」ヒットテストルートを確保**——`CompositionDrawingSurface`（書き込み専用）だとその場合ヒットテスト用の別 CPU コピーが要る（2度手間）が、swap chain 所有なら読み戻し1本で済む。開発者の主目的。
- WUC 内部アトラスを迂回（読み戻し阻害要因の除去）。

**アトラスの切り分け（重要）**: 読み戻しを阻むのは **WUC 内部アトラス**であって **emo-atlas（`AtlasTable`）ではない**。emo-atlas は合成の上流・CPU 側素材アトラスで、合成出力 `ComposedSurface` は既に平坦化済みの1枚物（アトラス解決済み・offset 問題なし）。よって emo-atlas はこのルートを妨げない。

**留意点**: (1) flip model backbuffer は直接 Map 不可＝自前アップロード元テクスチャを読むか staging へ `CopyResource`。(2) リサイズは `ResizeBuffers`（R1.5）。(3) swap chain は面ごとだが emo は**窓あたり1枚物**ゆえ窓あたり1本（element 単位でない）＝軽微。(4) swap chain の alpha mode を premultiplied で一致。

**位置づけ**: **ディスカッション #3 で要件化（R8・必達）**——emo-present はコンポジター供給面を書き込み専用の `CompositionDrawingSurface` ではなく**自前所有・読み戻し可能なオフスクリーン面（swap chain 相当）**とし、その CPU 読み戻し経路を確保する。将来のオフスクリーン面直読みヒットテスト経路の基盤（当たり判定の実導出は後続・M-boot は R2 の CPU バイト経由）。Option C（ハイブリッド）と両立。design 冒頭で小 spike（swap chain 供給＋readback 往復）で実装可能性を実証してから本実装へ。記憶 [[gpu-draw-verification-offscreen-d2d-target]]（議論#1）とも整合——議論#1 のオフスクリーン D2D 検証（R6.7）はこのルートでは「自前所有面の読み戻し」に自然統合される。

---

## 5. 指令 API・キャッシュ・バルーン（wintf 非依存部の選択肢）

### 指令 API の形（R3）

- **単一メッセージ enum に転写可能な Send 所有データ**とする（brief・記憶 areka-concurrency-model）。`show_surface(scope, surface_id: u32, binds: BindSet)` 級。非表示は別 variant か `surface_id` の番兵値（`\s[-1]`）——**seriko-engine と両 design で突合必須**（seriko 側 brief も同じ突合を要求）。
- 応答要否は `reply_channel`（`ReplySender` 同梱）を許容する形（R3.6）。M-boot は直接呼出で開始し channel 化は結線時（kanade/seriko）。
- Option: enum を emo-present が定義し、seriko が消費（emo-present が契約正本＝brief 明記）。

### キャッシュ（R4）

- **全保持（`HashMap<u32, CacheEntry>`）**が emo2 規模で妥当（brief）。`CacheEntry { composed: ComposedSurface, mask: AlphaMask }` で表示バッファと当たり判定マスクを**同一エントリに束ね原子入替**（R2.4）。
- 無効化は「全破棄」の口だけ（アトラス再構築・ghost 再読込用の**構造**）。M-boot ではアトラス不変ゆえ実質未使用（brief）。
- emo-compose の `compose_into`（out 再利用）とキャッシュの out バッファ所有が好相性。

### バルーン枠（R5）

- `balloons*.png` を fixture 直指定で同一表示機構へ。**入力経路の選択肢**: (a) balloon も emo-atlas/emo-compose を通して `ComposedSurface` 化してから同経路 / (b) 枠は単一 PNG ゆえ直 WIC ロードで簡略化。emo2 実測（brief）では balloon は `.pna` 無し・PNG α のみ・overlay のみ。
  - **ディスカッション決定（#2）**: **(a) を採用**（統一グラフィック原則を M-boot でも貫く・記憶 areka-unified-shell-balloon-graphics）。R5.1 に「枠も `ComposedSurface` 化して同一経路・直 WIC バイパスは用いない」を明記。残る「atlas 登録の具体（balloon をどう atlas/EmoWorld へ載せるか）」は design（Research 5・descript_balloon 3 分類と併せて）で確定。
- 配置は `sakura.balloon.offsetx/offsety`（R5.4）——window-placement のバルーン追従 offset と同座標系か照合（brief）。

---

## 6. Research Needed（design へ持ち越す未確定事項）

1. **WUC surface へのメモリアップロード API**: `CompositionDrawingSurface` interop（`ICompositionDrawingSurfaceInterop::BeginDraw`→`ID2D1DeviceContext`→`ID2D1Bitmap1::CopyFromMemory` or `DrawBitmap`）の具体手順と、wintf 既存 graphics 資産（`surface.rs`/`wuc_resource.rs`）のどこまでを再利用/露出するか。in-memory `IWICBitmap`（`CreateBitmapFromMemory`）経由か bitmap 直コピーか。
   - **ディスカッション決定（#1）**: golden 検証は **描画のレンダリング先をコンポジター surface と分離**し、通常の D2D オフスクリーン描画先（自前 `ID2D1Bitmap1` ターゲット）へ描いて `Map` readback → emo-compose golden とバイト一致を**決定論 assert**する（要件 R6.2/R6.7）。GPU 合成窓の backbuffer readback（記憶 areka-gpu-window-screenshot-readback）は不要。emo-present の新規責務＝「メモリバッファ→D2D 描画可能形」の正しさはこのオフスクリーン検証で担保され、コンポジター提示の pixel 検証は wintf 既存経路（`BitmapSource` の `CommandList→WUC`）の責務として emo-present では行わない。design はこの分離を検証シームとして設ける。
2. **実 DPI での等倍表示契約**: 合成＝物理 px 等倍として、表示側の論理/物理変換の帰属（`WindowPos`/`BoxStyle`/`GlobalArrangement` のどれが物理でどれが論理か）を wintf 座標契約に整合させる。dpi≠96 実行での AlphaMask クリック座標一致（R2.5）。記憶 areka-window-placement-dpi-coordinate-defect の教訓に従い**実 DPI 実行で証明**。
3. **`\s[-1]` 非表示の意味論**: 指令 API に非表示を別 variant で持つか surface_id 番兵で持つか。seriko-engine（並走）brief の発行側表現と突合。ukadoc `sakurascript` の `\s[ID番号]` 正典参照。
4. **AlphaMask を hit-test へ届ける型**: 既存 hit-test は `BitmapSourceResource` からのみ α マスクを読む。emo-present の合成結果マスクを供給するため (a) `BitmapSourceResource` を再利用（Option A）か (b) hit-test に emo 専用の α 供給読み口を足すか。
5. **バルーン descript キーの正典**: `descript_balloon` 全文（`use_self_alpha`〔shell と別定義〕・`paint_transparent_region_black`・`overlay_outside_balloon`・有効描画領域/テキスト領域系）を design 冒頭で ukadoc `get_doc` 参照し「枠描画に効くキー／テキスト領域キー（→emo-text-layer 引継ぎ）／M1 対象外」の 3 分類表を作る（brief 必読指示）。カテゴリ id は `descript`。
6. **text-layer 予約スロットの visual 構成**: surface 本体 visual の上に独立 text 層を差し込む seam（M1 独立レイヤ／M2 合成パス内レイヤの二者を吸収）。予約しないと emo-text-layer 着手時に visual 構成の作り直し（brief）。

---

## 7. 工数・リスク（要件全体）

- **工数**: **L（1〜2 週）**。純粋部（指令 API・キャッシュ・AlphaMask 同期）は既存型流用で軽いが、**メモリ供給表示口（WUC surface interop）が新規 COM 結線**であり、実 DPI 観測 example まで含めると中〜大。
- **リスク**: **Medium**。
  - 低下要因: 上流契約（emo-compose/atlas）完成・premultiplied 形式一致・クリックスルー/AlphaMask/donor が実在・pilot で別プロセス透過が実証済み。
  - 上昇要因: WUC surface へのメモリ直アップロードが未踏経路（Research 1）・実 DPI 座標契約の確定（Research 2・過去に window-placement がこの取り違えでリジェクト）・seriko との `\s[-1]` 契約突合（Research 3）。
  - GPU 合成窓はスクショ不可（記憶 areka-gpu-window-screenshot-readback）だが、**ディスカッション #1 で解消**——golden 検証は**コンポジター surface を検証対象にせず**、通常の D2D オフスクリーン描画先へ描いて readback する検証シーム（R6.7）で決定論的に行う。よって backbuffer readback リスクは要件範囲から外れた。

---

## 8. 設計フェーズへの推奨

- **推奨アプローチ**: **Option C（ハイブリッド）を第一候補**。低レベル「メモリ→表示リソース」ヘルパのみ wintf の COM 層（unsafe 隔離規約）へ足し、表示コンポーネント・キャッシュ・指令 API・AlphaMask 同期は emo-present に集約。brief の層分離思想（表示口＝emo の唯一の wintf 接触層・BitmapSource 不侵）と現実コストの均衡。Option A は「最短で golden を出す」際の代替として保持。
- **キー決定事項**（design 冒頭で確定）: (1) WUC surface メモリアップロードの具体経路（Research 1）(2) 実 DPI 等倍の座標帰属（Research 2）(3) `\s[-1]` 非表示の API 表現（seriko 突合・Research 3）(4) AlphaMask の hit-test 供給型（Research 4）(5) `descript_balloon` 3 分類表（Research 5）(6) text-layer 予約スロット位置（Research 6）。
- **観測前提**: 実 DPI（dpi≠96）実行を経ないと座標正しさは未証明（記憶 areka-placement-real-ghost-first・window-placement-dpi-coordinate-defect）。golden 一致は GPU 合成窓ゆえ readback が要る可能性。
- **持ち越し Research**: §6 の 1〜6 を design で解消。ukadoc は `descript`（バルーン）・`sakurascript`（`\s[-1]`）カテゴリを `get_doc` で正典参照。

---

_本ドキュメント（§1〜§8）はギャップ分析であり、ディスカッション決定 #1/#2/#3 を含む。以降は design フェーズの Discovery ログと設計決定。_

---

# 設計フェーズ Discovery & Synthesis（2026-07-06・design.md 生成時）

## Summary

- **Feature**: `areka-P0-emo-present`
- **Discovery Scope**: Extension（既存 wintf/emo 資産への統合・integration-focused）＋ ukadoc 正典参照
- **Key Findings**:
  - wintf に swap chain 使用は皆無（`CreateSwapChainForComposition`/`IDXGISwapChain` 全文検索ゼロ）＝ R8 は純増設。材料（`GraphicsCore::d3d()/dxgi()`・`ICompositorInterop` cast 済み・`CompositorInteropExt` trait）は公開 API で完備
  - `hit_test_entity` の AlphaMask 読みは `BitmapSourceResource` ハードコード（フォールバック＝矩形）→ 汎用 `AlphaMaskResource` コンポーネント追加＋優先読みが最小増分（既存挙動完全後方互換）
  - バルーン枠は emo-atlas の新 API なしで載る: `bake` は手組み `SurfaceSet`（`Surface`/`Element` 手構築）を受理（emo2_e2e.rs で実証済みパターン）。synthetic surfaces.txt→`shell::parse` なら公開 parse API のみで完結
  - `\s[-1]`＝非表示サーフェスの正典確認（`list_sakura_script` \s[ID番号]）。alias/name 文字列も \s に指定可＝解決は surface 状態所有者（seriko）側
  - `GlobalArrangement.bounds`＝物理 px（実装コメント明記）・`WindowPos`＝物理・`BoxStyle`＝論理 → 表示経路から BoxStyle を排除し「窓クライアント寸=surface 原寸（物理）」で恒等写像化するのが DPI 事故（placement 欠陥の教訓）の構造的排除

## Research Log（ギャップ分析 §6 の解消）

### Research 1: WUC surface へのメモリアップロード経路
- **Findings**: R8（ディスカッション #3）により `CompositionDrawingSurface` interop 経路は不採用が確定済み。採用経路は `IDXGIFactory2::CreateSwapChainForComposition`（flip・`DXGI_ALPHA_MODE_PREMULTIPLIED`・B8G8R8A8・BufferCount 2）→ `ICompositorInterop::CreateCompositionSurfaceForSwapChain` → `CreateSurfaceBrushWithSurface` → SpriteVisual。アップロードは `UpdateSubresource(source_tex)`→`CopyResource(backbuffer)`→`Present(0)`、readback は `CopyResource(staging)`→`Map(READ)`（flip backbuffer は直接 Map 不可のため source_tex を単一真実源とする）。**D2D はピクセル経路に一切介在しない**＝純バイト転送で golden 決定論が最強化。wintf の `surface_pixel_equivalence_test` が CPU_READ staging＋Map の readback パターンを既に確立
- **Implications**: 議論 #1 のオフスクリーン D2D 検証（R6.7）は「自前所有面の readback」に統合（ギャップ分析 §4 Option D の予告どおり）。in-memory WIC/`CopyFromMemory` 案は全て不要になり廃案

### Research 2: 実 DPI 等倍表示の座標帰属
- **Findings**: `GlobalArrangement.bounds`＝物理 px（スクリーン座標・実装コメント明記）・`Monitor.work_area`/`WindowPos`＝物理・`BoxStyle`＝論理（taffy）。hit-test のマスク座標変換は bounds 相対比例
- **Decision**: emo-present は **BoxStyle（論理）を表示経路に使わない**。WindowPos サイズ＝surface 原寸（物理）を直接設定し、SetSize も物理。bounds 寸＝マスク原寸＝surface 原寸 → rel→mask 変換が恒等＝任意 DPI でクリック一致（R2.5）。契約文は design.md「DPI 表示契約」が正本（window-placement 引継ぎ・Revalidation Trigger）

### Research 3: `\s[-1]` 非表示の API 表現（seriko 突合）
- **Findings**: 正典（ukadoc `list_sakura_script`）: 「\s[ID番号]…現スコープ側のサーフェスを ID 番号のサーフェスに変更する。\s[-1]で非表示サーフェス。surfaces.txt の surface.alias または name で定義された文字列を ID の代わりに使用できる」。短縮形 `\sID` は 0〜9 のみ
- **Decision**: **`Hide` 専用 variant**（番兵不採用）。理由: emo-compose 契約の surface_id は `u32`（-1 を型が表せない）・番兵はスクリプト層の語彙でありスクリプト解釈者（seriko）が `\s[-1]`→`Hide`、alias→u32 解決を担うのが層として正しい。突合結果として「発行側が解決済み u32＋Hide を送る」を Data Contracts に固定（seriko design の Revalidation Trigger）

### Research 4: AlphaMask を hit-test へ届ける型
- **Findings**: `hit_test_entity` は `HitTestMode::AlphaMask` 時 `BitmapSourceResource.alpha_mask()` のみ読む。他の供給コンポーネントは存在しない
- **Decision**: wintf hit_test モジュールへ汎用 `AlphaMaskResource`（CPU リソース命名規約準拠・Component）を新設し**最優先読み**＋既存フォールバック維持。(a) `BitmapSourceResource` 再利用案は WIC source 必須フィールドが空になる歪みで棄却。emo 専用型を wintf に置く案は層汚染で棄却（汎用＝メモリ供給ウィジェット一般が使える）

### Research 5: バルーンの atlas/EmoWorld 載せ方（`descript_balloon` 3 分類）
- **Findings**: `bake(sets: &[SurfaceSet])` は `Shell` パース結果に限らず手組み `Surface` 配列を受理（emo2_e2e.rs の hand_surface パターン実証済み）。ただし `EmoWorld::build(&Shell)` は `Shell` を要求
- **Decision**: **synthetic surfaces.txt 生成→`shell::parse`→`Shell`** の一本道（公開 API のみ・emo-atlas 新 API 不要・Shell 内部構造への依存も回避）。3 分類表は design.md に掲載（(a) use_self_alpha/paint_transparent_region_black/dpi、(b) origin/validrect/wordwrappoint/font→text-layer、(c) marker/arrow/online/balloonc/cursor/anchor/windowposition）。emo2 kakukaku は PNG α のみ・pna 無し → `AlphaParams`＝self-alpha 相当。低確度 1 点: `sakura.balloon.alignment` 値域脚注（none/left/right）は MCP スナップショット外＝実装時に実ページ確認
- **バルーンファイル役割**（`manual_balloon` 全文確認）: balloons\*=本体側（偶数左/奇数右の 2 枚組）・balloonk\*=相方（省略時本体代用）・balloonc0-4=入力ボックス・arrow0/1=スクロール矢印・online\*=受信アニメ・sstp/marker/clickwait=各マーカー・balloons\*s.txt=ID 別上書き設定

### Research 6: text-layer 予約スロットの visual 構成
- **Decision**: surface entity の**兄弟・上位 z** に空 entity（`Name("emo-text-layer-slot")`＋`Visual` のみ）を予約。M1 独立レイヤ描画＝この entity に描く／M2 合成パス内レイヤ化＝この entity を畳んで合成へ移す、の二者を「slot entity の差し替え」で吸収。入れ子 Visual 合成は導入しない（R1.4）

### 追加確認: 上流実シンボル・donor・actor
- emo-compose: `compose_into(&mut ComposedSurface, &EmoWorld, &AtlasTable, u32, &BindSet) -> Result<(), ComposeError>`・`compose` 値返し・`ComposeError { SurfaceNotFound(u32), EmptyComposition(u32) }`（全透明は Err にしない）・`ComposedSurface`（width/height/stride=w*4/bytes/into_bytes・Send/Clone/Default）・rustdoc が「キャッシュは emo-present の責務」明記
- emo-atlas: `bake(&[SurfaceSet], &impl ElementDecoder, PackConfig) -> BakeResult`・`AtlasPage { width, height, stride, bytes: Arc<[u8]> }`（premultiplied BGRA）
- golden: emo-compose はバイト等値（`assert_eq!(out.bytes(), expected)`）。surface1000 の全 bind＝`BindSet::from_ids([1100, 1200, 1302])`（descript default の動的解決ヘルパは無し＝呼び手構築）
- donor: mock-shell（WS_POPUP＋`WS_EX_LAYERED|TOOLWINDOW|TOPMOST`・`register_click_through_windows`＝`Added<WindowHandle>` で `ClickThroughRegistryHandle::register(entity, hwnd)`・`FrameFinalize` 登録）
- areka-actor: `spawn_ui<M, E>(name, handler) -> (UiSender<M>, JoinHandle)`・`UiSender::send`（unbounded・Closed のみエラー）・`ReplySender`/`reply_channel`。envelope 規約（Send 所有・大型データは Arc 手渡し・Close 必須 variant）→ `PresentCommand` は転写可能形で設計

## Architecture Pattern Evaluation（最終）

| Option | 判定 | 根拠 |
|--------|------|------|
| A: BitmapSource 拡張 | 棄却 | レイヤ侵食・WIC→D2D 二度写し・R8（書込専用面不可）と両立しない |
| B: emo 専用 widget（CompositionDrawingSurface interop） | 棄却 | R8 で CompositionDrawingSurface（WUC 内部アトラス・読み戻し不能）自体が不採用 |
| **C＋D: COM ヘルパのみ wintf・表示層は emo-present・供給面は自前 swap chain** | **採用** | R8 必達・層分離（表示口＝emo の唯一の wintf 接触層）・readback＝検証と将来ヒットテストの単一真実源 |

## Design Decisions（design.md へ反映済みの要約）

1. **供給面＝自前 swap chain＋source_tex 単一真実源**（R8）。D2D 非経由の純バイト転送。readback は source_tex→staging→Map
2. **検証シーム統合**: R6.7 のオフスクリーン検証＝R8.3 readback（議論 #1 の意図を Option D 経路で充足・コンポジター提示の pixel 検証はしない）
3. **`Hide` 専用 variant**（番兵不採用）・alias/`\s[-1]` 解釈は seriko 側
4. **`AlphaMaskResource`（wintf 汎用増分）**＋hit_test 優先読み・後方互換
5. **バルーン＝synthetic surfaces.txt→parse→bake→EmoWorld**（公開 API のみ・統一経路・R5.1）
6. **DPI＝全物理 px 経路**（BoxStyle 不使用・窓クライアント寸=surface 原寸・恒等写像で R2.5）
7. **キャッシュ＝target ごと全保持 HashMap・CacheEntry{composed, mask} 対**（原子入替の構造的担保）・invalidate は全破棄の口のみ
8. **text-layer slot＝surface visual の兄弟・上位 z の空 entity 予約**
9. **UI スレッド強制＝EmoPresenter を NonSend**（型で担保）・M-boot 直接呼出→将来 spawn_ui handler へ無改変移行
10. **0x0 退化合成は Hide 相当へ縮退**（warn・swap chain は 0 寸不可）

## Synthesis 記録

- **Generalization**: シェルとバルーンは「PresentTarget」1 機構の 2 インスタンス（統一グラフィック原則の構造化）。AlphaMask 供給は emo 専用でなく wintf 汎用コンポーネント化
- **Build vs Adopt**: wintf 既存資産（SpriteVisual 装着型・clickthrough・hit-test・readback パターン）と emo 実シンボルを全面採用。新造は swap chain 供給・AlphaMaskResource・synthetic balloon 適合の 3 点のみ。emo-atlas への新 API 追加は回避（公開 parse で代替）
- **Simplification**: D2D/WIC をピクセル経路から排除（変換段ゼロ）・LRU 不採用（全保持）・部分キャッシュ無効化なし（全破棄のみ）・デバイスロストは invalidate＋次回再作成の最小規律・番兵値なし

## Risks & Mitigations

- **swap chain×WUC brush の未踏結線** → 実装フェーズ先頭で spike（供給＋readback 往復＋リサイズ）を統合テストとして先行・GO 確認後に本実装（WARP 可＝CI 決定論）
- **wintf `Visual` on_add の自動挿入（SurfaceGraphics 等）と emo brush 装着の干渉** → spike で確認・干渉時は最小構成挿入へ切替（BitmapSource 系不侵の境界は不変）
- **実 DPI 未実行のまま GO 判定**（placement リジェクトの教訓） → example rustdoc に dpi≠96 手順明記・検証記録に実 DPI 実行を必須化
- **`ResizeBuffers` の未解放参照エラー** → backbuffer は転送中のみ取得しスコープ解放の規約
- **`sakura.balloon.alignment` 値域の低確度** → 実装時に ukadoc 実ページ確認（M-boot は offsetx/y 直読みのため影響軽微）

## References

- ukadoc: `list_sakura_script`（\s[ID番号]・\sID）・`descript_balloon` 各キー・`descript_shell`/`descript_shell_surfaces`（sakura.balloon.offsetx/y・alignment）・`descript_ghost`（balloon.defaultsurface）・`manual_balloon`（ファイル構成全文）
- 実装正本: `crates/areka-emo-compose/src/{lib,composed,bind,world,error}.rs`・`crates/areka-emo-atlas/src/{lib,table,manifest,decode}.rs`・`crates/wintf/src/{com/{wuc,d3d11},ecs/graphics/{core,visual,visual_manager,systems/surface,wuc_resource},ecs/layout/{arrangement,hit_test/mod},ecs/widget/bitmap_source/{alpha_mask,resource},ecs/clickthrough/controller}.rs`・`crates/wintf/tests/graphics/surface_pixel_equivalence_test.rs`・`crates/areka/examples/mock-shell.rs`・`crates/areka-actor/src/{lib,ui,reply}.rs`
- 記憶正本: areka-emo-own-compositor-atlas・areka-wuc-runs-on-mta-thread・areka-clickthrough-hittest-config・areka-window-placement-dpi-coordinate-defect・areka-log-first-no-silent-failure・areka-concurrency-model
