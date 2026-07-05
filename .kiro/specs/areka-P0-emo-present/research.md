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

- `balloons*.png` を fixture 直指定で同一表示機構へ。**入力経路の選択肢**: (a) balloon も emo-atlas/emo-compose を通して `ComposedSurface` 化してから同経路 / (b) 枠は単一 PNG ゆえ直 WIC ロードで簡略化。emo2 実測（brief）では balloon は `.pna` 無し・PNG α のみ・overlay のみ——**(a) がシェルと機構統一**だが (b) の方が M-boot 最小。design 判断。
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

_本ドキュメントは情報提供であり最終決定を含まない。設計判断は要件ディスカッション／design フェーズへ委ねる。_
