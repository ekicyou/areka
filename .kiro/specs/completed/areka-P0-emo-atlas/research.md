# ギャップ分析 — areka-P0-emo-atlas

> フェーズ: gap-analysis（要件確定後）／言語: ja（spec.json）
> 対象: ⑥ emo トラック直列チェーン 1/3（emo-atlas → emo-compose → emo-present）の**素材基盤層**
> 方針正本: 合成は emo 自前・アトラス転写・1枚物（記憶 areka-emo-own-compositor-atlas／roadmap emo 節）
> 正典: ukadoc（`use_self_alpha`／`.pna`／キーカラー規則）・emo2 fixture は最小適合サンプル

本書は「情報提供であって決定ではない」原則に従い、既存コードベースの現況・要件充足に必要な技術要素・複数の実装アプローチ・設計フェーズへ持ち越す調査項目を整理する。**requirements.md／spec.json は確定済み・本書はそれらを一切改変しない。**

---

## 1. 既存コードベースの現況（Current State）

### 1.1 wintf の WIC デコード経路（既存・流用元）

- **場所**: `crates/wintf/src/com/wic.rs`（WIC の薄い ext トレイト群）＋ `crates/wintf/src/ecs/widget/bitmap_source/`（BitmapSource ウィジェット）。
- **WicCore**（`bitmap_source/wic_core.rs`）: `IWICImagingFactory2` を `CLSCTX_INPROC_SERVER` で生成し ECS Resource として保持。**MTA 前提**（`CoInitializeEx(COINIT_MULTITHREADED)`）で `unsafe impl Send + Sync`（WIC の thread-free marshaling に依拠）。
- **デコード関数** `load_bitmap_source(factory, path)`（`bitmap_source/systems.rs:75`）: `CreateDecoderFromFilename` → `GetFrame(0)` → **`IWICFormatConverter` で `GUID_WICPixelFormat32bppPBGRA` へ変換**して `IWICBitmapSource` を返す。**PBGRA（premultiplied BGRA）への正規化はここで既に行われている**が、これは「α チャンネルが有れば採用・無ければ 100% 不透明」という WIC の既定挙動であり、**`use_self_alpha`／キーカラー／`.pna` の伺か透過規則は一切解釈しない**。
- **CPU 画素アクセス**: `WICBitmapSourceExt::copy_pixels(rect, stride, buffer)`（`com/wic.rs:129`）と `get_size()` が既にある。`generate_alpha_mask_system`（`systems.rs:373`）は実際に `copy_pixels` で PBGRA バッファを取り出し、`AlphaMask::from_pbgra32(&buffer, w, h, stride)`（`bitmap_source/alpha_mask.rs:33`）でヒットテスト用マスクを生成している。**＝WIC ソース→CPU PBGRA バッファ抽出の前例が既に稼働している。**
- **α 走査の前例**: `AlphaMask::from_pbgra32` は行 stride を考慮しつつ全画素の α を走査する（trim ではなくビットパック 2 値化）。**「PBGRA バッファの α を stride 込みで走査する」パターンは実証済み**でトリム矩形算出に流用できる。

### 1.2 上流モデル（マニフェスト入力・areka-parsers・全て実装完了）

- **shell**（`crates/areka-parsers/src/shell/model.rs`）: `Shell { surfaces, appends, aliases }`。
  - `Surface { id, elements: Vec<Element>, collisions, animations }`。
  - `Element { layer, path: ElementPath, x, y }` — `ElementPath` は opaque NewType（`as_str()` のみ）で**無加工の画像パス**（サブディレクトリ区切り含む）を保持。
  - `Animation { id, interval, patterns }`／`Pattern { index, surface_id: i64, wait, x, y }` — **bind pattern が参照する `surface_id` は間接参照**（emo2 の surface1000 は静的 element ゼロで全パーツが `pattern0,overlay,<helper_surface_id>,0,0,0`）。負値はレイヤクリア/停止センチネル（要件 5.5・下流解釈）。
  - **重要**: shell モデルは「element の画像パス」と「pattern の参照先 surface_id」を持つが、**surface_id → その surface の element パスへの解決（間接参照の展開）は本層が行う**（parser は転記層・展開は下流／記憶 areka-parser-transcribes-tree-downstream）。
- **balloon**（`crates/areka-parsers/src/balloon/model.rs`）: `BalloonModel { windowposition, origin, wordwrappoint, validrect, font }`。
  - **⚠ balloon モデルは幾何＋フォントのみをモデル化し、`balloons0.png`/`balloonk0.png` 等の画像パスも `use_self_alpha` も一切保持しない**（`parse.rs` の「image」は s0s/k0s 差分マージ層の意で画像ファイルではない）。要件 5.5 で choice/scroll 系を非モデル化した設計方針の帰結。→ D1（設計で解決済・後述）。
- **package**（`crates/areka-parsers/src/package/model.rs`）: `MountModel { names, shiori: ShioriMount{dir,file}, shell: ShellMount{dir} }`。**`shell.dir`（物理存在確認済み）は element パス解決の基準になる**が、**balloon dir は MountModel に含まれない**（balloon 所在解決は baseware 共有・スコープ外／記憶 areka-ghost-boot-descript-not-install）。

### 1.3 packing クレート（新規依存候補）

- ワークスペースに `rectangle-pack`/`rect_packer`/`texture_packer` は**未導入**（`workspace.dependencies` に該当なし）。
- brief 済み調査（2026-07-03）: 本命 **`rectangle-pack`**（zero-dep・MIT/Apache・静的バッチ packing・複数 bin＝頁対応・活発維持）。padding 非内蔵＝**矩形を 1〜2px 拡げて渡す自前ラップ**（自明）。対抗 `rect_packer`（zero-dep・MIT・padding 内蔵だが単頁 API＝複数頁 DIY）。棄却: `texture_packer`（`image` 強制依存＋休眠）・`guillotiere`/`etagere`（動的アロケータ＝bake-once に過剰）・`crunch`（回転不要）。
- **新規依存＝開発者承認必要**（encoding_rs 前例に倣う・design で正式申請＝本書 §承認）。

### 1.4 emo2 fixture 実測（透過腕の確定）

- 場所: `crates/pilot/examples/shiori-host-32/fixtures/emo2/`。
- shell descript（`shell/master/descript.txt`）: `charset,UTF-8`・`seriko.use_self_alpha,1`・`.pna` ファイル**無し**（`shell/master/**/*.pna` glob 0 件）→ **主実装腕＝PNG 自身の α チャンネル。キーカラー腕・`.pna` 腕は型シームのみ**（要件 3.5 に整合）。
- element PNG は `surface0.png`/`surface10.png`（トップ）＋ `CityPop/`（surfaceNNNN・ゼロ詰め variant）＋ `purple/<0..4,a>/` 配下（`base1.png`/`niko.png`/`ribbon.png` 等・サブディレクトリパス）。**ElementPath のまま素通し**（要件 1.5）。
- balloon（`emo2-kakukaku/`）: `balloons0.png`・`balloonk0.png`・`balloonc1..4.png` が実在。**α 規則は surface と同一**（`use_self_alpha,1`・balloon descript 側の同名キー）。
- surface1000 は `pattern0,overlay,1100..1800台,0,0,0` で helper surface を bind 参照（**間接参照の実例**）。element オフセットは全て `0,0`。

### 1.5 コード規約・配置（structure.md）

- クレート分割・責務分離。CPU 非依存純粋層は `XxxResource` 系（デバイスロスト非対応）、GPU 依存は `XxxGraphics`。**本層の成果物（アトラス表・頁バッファ）は CPU 保持の純粋データ＝`XxxResource` 系寄りだが、通信非依存・channel 非依存の独立層**。
- ワークスペース members は `crates/*` glob＝**新クレート `crates/areka-emo-atlas/` は自動参加**。最小依存分離の前例＝`shiori-abi`／`areka-parsers`。
- テスト: in-source `#[cfg(test)]` 主軸＋fixture スモーク。**WIC を使うテストは COM 初期化（`CoInitializeEx`）が必要**（wintf の既存テスト慣行）。
- 依存方向厳守・unsafe は COM ラッパー層に集約。

---

## Summary（設計フェーズ更新）

- **Feature**: `areka-P0-emo-atlas`
- **Discovery Scope**: Complex Integration（既存 wintf WIC 経路＋areka-parsers モデルへの統合・新規純粋クレート追加）
- **Key Findings**:
  - 既存 WIC 経路（`com/wic.rs` ext ＋ `load_bitmap_source`）が PBGRA 抽出まで実証済で、デコード腕として再利用可能。ただし ECS（`bitmap_source`）密結合部は流用不可。
  - workspace は `crates/*` glob ゆえ新クレート `crates/areka-emo-atlas/` が自然。WIC ext は wintf 内ゆえ、デコード腕のみ wintf WIC ユーティリティを参照し純粋コアは wintf 非依存に保つ（ヘキサゴナル軽量版）。
  - ukadoc（`seriko.use_self_alpha`／balloon `use_self_alpha`）が 3 値（1/true, full, 0）× `.pna` 有無の透過規則を正典として確定。emo2 は `1`＋`.pna` 無＝α 採用腕のみ実装、他はシーム。

---

## 2. 要件→資産マップ（Requirement-to-Asset Map・ギャップ種別: Missing / Unknown / Constraint）

| 要件 | 必要な技術要素 | 既存資産 | ギャップ |
|---|---|---|---|
| **R1 マニフェスト導出** | shell/balloon モデルから element パス列挙・間接 bind 参照解決・重複排除 | shell `Element.path`/`Pattern.surface_id`・package `shell.dir` | **Missing**: 列挙器そのもの。**間接参照解決（surface_id→element パス）は本層新規**。balloon 画像パスは入力に無い（D1・設計で解決） |
| **R2 デコード** | パス→画素バッファ・差替え可能 trait・エラー継続 | `load_bitmap_source`（PBGRA 化）・`copy_pixels`/`get_size` | **Constraint**: 既存は PBGRA 化を WIC 内で行い raw バッファ API を公開しない。**「デコード手段の trait 薄切り」は新規**。エラー継続も新規 |
| **R3 透過正規化** | `use_self_alpha` 解釈・α>`.pna`>キーカラー優先・premultiplied BGRA 統一 | WIC PBGRA 変換（α有→採用/α無→不透明） | **Missing**: 伺か透過規則の解釈層。emo2 は α 腕のみ実装（他はシーム） |
| **R4 α トリミング** | α>0 タイト矩形算出・trim_offset/trimmed/original 記録・全透明→空 | `AlphaMask` の stride 込み α 走査パターン | **Missing**: トリム矩形算出・オフセット記録（走査パターンは流用可） |
| **R5 packing** | 静的バッチ配置・padding・複数頁・決定性・重複排除 | なし | **Missing**: `rectangle-pack` 新規導入（**承認要**）＋padding 自前ラップ |
| **R6 索引表 API** | AtlasKey→AtlasEntry・頁 premultiplied BGRA バッファ（stride 明示）・`Send`/`Arc` 共有・channel 非依存 | `XxxResource` 命名規約・`Arc` 手渡し規約 | **Missing**: 型そのもの。**emo-compose と共有契約＝本層が正本定義（D3）** |

**非機能**: 決定性・premultiplied 一貫性・スレッド安全所有（`Send`＋`Arc`）。既存基盤（WIC MTA・Arc 規約）と整合。

---

## 3. 実装アプローチ（Option A / B / C）

### Option A: wintf の bitmap_source を拡張して載せる — 棄却
- ✅ WIC 再結線ゼロ。❌ 本層は通信/ECS/表示非依存の純粋層＝ECS ウィジェット（Component＋on_add＋WintfTaskPool 前提）とは責務も依存形も異なる。純粋オフスクリーンテストと相反。

### Option B: 新規クレート／モジュールとして分離（純粋層）— 採用の骨格
- emo 素材基盤を独立配置。WIC デコードは `com/wic.rs` ext を再利用するが `bitmap_source` ECS 機構には依存しない。

### Option C: ハイブリッド（WIC 薄ラッパー共有・上位は新規純粋層）— 採用（B に内包）
- デコード最小面（decoder→PBGRA raw）を trait ポートに隔離、既定腕＝WIC。テスト腕＝メモリ PBGRA で正規化以降は COM 非依存。**要件 2.3 と純粋テスト方針を両立する最有力＝採用。**

**採用**: B の骨格＋C のデコード trait 隔離。新クレート `crates/areka-emo-atlas/`・WIC 既定腕は `com/wic.rs` ext を再利用（D2）。

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| bitmap_source 拡張（A） | 既存 ECS ウィジェットへ atlas を追加 | WIC 再結線ゼロ | ECS/D2D/TaskPool 密結合＝純粋テスト不能 | 棄却 |
| 純粋コア＋デコードポート（B+C・採用） | ヘキサゴナル軽量版。COM 依存を trait ポートに隔離 | 純粋オフスクリーンテスト可・要件 2.3 直合致・emo-compose と契約共有容易 | WIC ext 所在調整（D2） | **採用** |
| 全部自前デコード | image クレート等で独自デコード | wintf 完全非依存 | 新規デコード依存増（要件は「新規デコード依存ゼロ」）・PBGRA 再実装 | 棄却（要件違反） |

---

## Design Decisions

### Decision D2: クレート境界と WIC ext の所在
- **Context**: 本層を新クレートにするか wintf 内モジュールにするか。WIC ext（`com/wic.rs`）と `load_bitmap_source`（`bitmap_source/systems.rs`）は wintf 内。
- **Alternatives Considered**:
  1. 新クレートが wintf 全体へ依存 — ECS/D2D/GraphicsCore を巻き込み純粋性喪失。
  2. WIC 薄ラッパーを独立共有クレートへ切出し — emo-compose/present も WIC を要さないため過剰。
  3. wintf 内 COM 層（`com/wic.rs`）へ `load_bitmap_source` 相当を移設・公開し、新クレートはデコード腕のみが最小 feature の `windows`＋その WIC ユーティリティを参照。
- **Selected Approach**: (3)。新クレート `crates/areka-emo-atlas/` を追加（`crates/*` glob 自動参加）。純粋コア（normalize/trim/pack/table/manifest）は wintf 非依存。既定 WIC デコード腕のみが wintf の WIC ユーティリティを参照。`load_bitmap_source` を ECS 非依存の `com/wic.rs` へ移設（ECS 依存部は非移設・挙動不変リファクタ）。
- **Rationale**: 最小依存分離の前例（`shiori-abi`/`areka-parsers`）に整合。COM/unsafe をデコード腕へ集約する steering 規律を満たす。切出し新クレートより変更面が小さい。
- **Trade-offs**: ✅ 純粋性・テスト隔離・契約共有容易。❌ wintf の WIC ユーティリティ移設（挙動不変の小リファクタ）が必要。
- **Follow-up**: 移設後 `bitmap_source/systems.rs` が移設先を参照して既存テストが緑であること。

### Decision D3: AtlasEntry/AtlasKey 共有契約の正本
- **Context**: 要件 6 の索引表型を本層が正本定義（emo-compose が再定義しない）。空エントリ表現の確定。
- **Selected Approach**: `AtlasKey(String)`（正規化 element パス・無改変）。`AtlasEntry{ original: Size, placement: Option<Placement> }`＝**空エントリは `placement: None`**（全透明・転写スキップ）。`Placement{ page, uv_rect, trim_offset }`。頁は `AtlasPage{ width, height, stride, bytes: Arc<[u8]> }`（premultiplied BGRA・stride 明示・Arc 共有）。`AtlasTable{ entries: HashMap, pages: Arc<[AtlasPage]> }`・`Clone`。
- **Rationale**: `Option<Placement>` が「原寸は常に持つが焼付は無い」空エントリを型で自然表現（`Option<AtlasEntry>` の二重 Option を回避）。`Arc<[u8]>` で `Send+Sync`＋安価 Clone（roadmap 並行モデル・大型データ Arc 手渡し）。
- **Trade-offs**: ✅ 空/未知/既知の 3 分岐を型で区別（`get`→`None`＝未知、`Some(entry{placement:None})`＝空）。❌ フィールド変更は下流破壊（Revalidation Trigger）ゆえ最小に保つ。

### Decision D4: デコード trait と COM 隔離度
- **Context**: 要件 2.3「差替可能・既定手段を上位非露出」＋純粋テスト範囲の確定。
- **Selected Approach**: `trait ElementDecoder { fn decode(&self, path) -> Result<DecodedImage, DecodeError>; fn probe_pna(&self, path) -> bool }`。既定腕＝WIC（COM 必要）／テスト腕＝メモリ PBGRA。`DecodedImage` は BGRA8＋`has_alpha`。正規化以降（normalize/trim/pack/table）は `DecodedImage` を入力とし **COM 非依存**。
- **Rationale**: COM/unsafe を WIC 腕に隔離＝正規化以降が COM init 不要でテスト可能。要件 2.3 に忠実（上位は trait のみ見る）。
- **Trade-offs**: ✅ テスト隔離・差替容易。❌ ポート境界の型（`DecodedImage`）を 1 つ増やす。

### Decision D5: use_self_alpha × .pna 動作表（ukadoc 正典）
- **Context**: 透過解釈の正典確定（ukadoc MCP `get_doc`/`search_docs`）。
- **Sources Consulted**:
  - `ukadoc:descript_shell:seriko.use_self_alpha_2c_5024:1`（shell）: 「1/true→α付PNG および .pna のある画像は α 参照。α も .pna も無ければ左上キー色（従来挙動）。full→加えて α 無し画像も全て不透明（キー色透過しない）。0＝既定。」
  - `ukadoc:descript_balloon:use_self_alpha_2c_5024:1`（balloon）: shell と同一セマンティクス（balloon 全体一括・オーバーライド不可）。
  - `.pna` 命名規則（`surfaceN.png`⇔`surfaceN.pna`）は ukadoc 記載薄＝SSP de-facto（`probe_pna` で吸収・emo2 は常に無し）。
- **Selected Approach**: 2×2＋α 有無の動作表（design §Normalizer に掲載）。**emo2 実装腕＝`use_self_alpha=1`（On）かつ α チャンネル有り→α 採用**のみ実装。`.pna`/keycolor/full/opaque は `AlphaSource` enum の variant＝型シーム（実装本体は `NormalizeError::Unsupported`）。
- **Rationale**: emo2 fixture 実測（`use_self_alpha,1`・`.pna` 無）に一致。過剰実装禁止（emo2 使用分のみ）。ukadoc 正典で拡張時の腕を型で予約。
- **Trade-offs**: ✅ 最小実装＋拡張シーム明示。❌ 将来の pna/keycolor 実装時に premultiply 自前が必要（D8）。

### Decision D6: 間接 bind 参照解決の走査規約
- **Context**: surface1000 の `Pattern.surface_id` → helper surface（1100 等）の element を列挙。
- **Selected Approach**: surface id 索引（`HashMap<u32, &Surface>`）を作り、各 surface の直接 element を列挙しつつ `Animation.patterns[].surface_id` を辿って参照先 surface の element も列挙。**除外規約**: 負 `surface_id`（レイヤクリア/停止センチネル）・索引に存在しない id・`Range` append・alias（画像を持たない）は列挙対象外。**循環検出**: 訪問済み surface id 集合で bind ループを打ち切る（emo2 は 1 段だが構造として保証）。
- **Rationale**: parser 転記層の非展開方針（記憶 areka-parser-transcribes-tree-downstream）に沿い展開を本層が担う。負値・不在の意味解釈はせず単に除外（要件 5.5 の下流解釈尊重）。
- **Trade-offs**: ✅ emo2 の 30 本 bind を安全に解決。❌ 多段 bind の意味（overlay 合成順）は本層のスコープ外（列挙のみ）。

### Decision D7: padding・頁サイズ・決定性の具体値
- **Context**: bleed 防止・複数頁・golden 固定。
- **Selected Approach**: padding=1px（矩形を全周 +1px 拡張＝`size + 2*padding` で packer 登録し UV は非包含の実矩形）。頁サイズ既定 2048（単頁超過矩形は自頁へ退避）。**golden 入力ソート順＝正規化パス昇順**（packer 呼出前に固定）で同一入力→同一出力を保証。
- **Rationale**: 1px padding で線形補間 bleed を防ぎつつ面積効率を保つ。2048 は emo2 規模に十分（4096 は将来）。ソート固定で `rectangle-pack` の内部順序非依存性を担保。
- **Trade-offs**: ✅ 決定性を golden で固定可。❌ fallback `rect_packer` は単頁 API ゆえ複数頁ループを自前実装（padding 内蔵は利用せず自前ラップに統一）。

### Decision D8: premultiplied 統一の実施点
- **Context**: WIC は既に PBGRA を返すが、シーム腕実装時の premultiply。
- **Selected Approach**: 統一点＝正規化段の出力（`NormalizedImage.pbgra`）。α チャンネル採用腕（emo2）は WIC 出力が既に premultiplied ゆえ実質恒等。シーム腕（keycolor/.pna/full）実装時は正規化段末尾で premultiply。契約は「Normalizer 出力は常に premultiplied BGRA」を型で明示。
- **Rationale**: 単一責務（premultiplied 保証を正規化段に集約）。straight α 混入（にじみ/暗縁）を段境界で排除。
- **Trade-offs**: ✅ 一貫性を段で担保。❌ シーム腕の premultiply は将来実装。

### 承認: 新規依存 `rectangle-pack`
- **Context**: 要件 5 の静的バッチ packing（複数頁・決定的）。
- **Selected**: `rectangle-pack`（zero-dep・MIT/Apache・複数 bin＝頁対応）を `[workspace.dependencies]` へ追加申請。padding は非内蔵ゆえ +1px 自前ラップ。
- **Rationale**: build-vs-adopt で adopt（車輪の再発明回避）。zero-dep で純粋性を損なわない。`encoding_rs` の意図的依存追加前例に倣い design で正式申請。
- **Fallback**: `rect_packer`（zero-dep・MIT・単頁 API＝複数頁を自前ループ）。
- **Follow-up**: **未承認の場合は着手不可**。承認後 `Cargo.toml` へ追加。

### D1（解決済・2026-07-03 要件ディスカッション #1）: balloon は surface システムで描画
- 開発者判断: balloon は内部設計上 surface システムで描画（surface＝内部に element を持ち element 合成で画像生成）。本層は balloon を shell surface と**区別せず**同一機構（要件 1.1）で扱う。要件 1.4 を「surface として表現された balloon の element を shell surface と同一機構で列挙」へ是正済。本層は `BalloonModel` の画像パスを直接消費せず、surface 表現（`SurfaceSet`）を受ける（要件 3.6 維持）。
- **残余（→ 隣接ユニット・design.md Open Questions #1 に記録）**: balloon 画像（`balloons*.png` 等）を surface 表現へ適合させる責務の所有者（balloon parser 拡張／package-mount の balloon dir 解決／専用アダプタ）と surface 表現型（shell 共通か balloon 専用か）。**本層設計・要件には影響しない**（本層は完成 surface 表現を入力として受ける契約で閉じている）。

---

## Design Synthesis（3 レンズ）

### 1. Generalization
- R1（直接 element 列挙）・R1.3（間接 bind 参照）・R1.4（balloon surface）は「**surface 集合から element パスを導出する**」単一の一般問題の変種。`ManifestDeriver::derive(SurfaceSet)` を一般インターフェイスとし、balloon を shell と同一 `Surface` として流し込むことで A（shell 直接）・B（間接）・C（balloon）を自然に包含。実装は emo2 が要する範囲（1 段 bind）に留める。

### 2. Build vs. Adopt
- **packing**: adopt `rectangle-pack`（成熟・zero-dep）。build 却下理由＝矩形詰めは既解決問題で自前は非決定性/バグ源。
- **デコード**: adopt 既存 WIC（`load_bitmap_source`）。新規デコード依存ゼロの要件に合致（`image` クレート等は却下）。
- **α 走査**: 既存 `AlphaMask::from_pbgra32` の stride 込み走査パターンを踏襲（コード流用でなくパターン流用）。

### 3. Simplification
- デコード trait は単一実装（WIC）＋テスト腕のみ＝ポート抽象は要件 2.3 が明示要求ゆえ保持（投機でない）。
- 透過腕は emo2 の 1 本のみ実装、他は enum variant の型シーム（要件 3.5 が明示要求）。実装本体を作らないことで最小化。
- 幾何プリミティブ（Point/Size/Rect）は純粋層自前定義（wintf::types への依存を避け純粋性維持）。`#[repr(C)]` は本層には不要（Win32 相互変換しない）。
- 頁サイズ/padding は単一 config・可変頁数のみ（回転・多戦略 packer は不要＝emo2 は回転不要）。

---

## Risks & Mitigations
- **`rectangle-pack` 承認ゲート** — 未承認なら着手不可。fallback `rect_packer`（複数頁自前ループ）を design に明記。
- **premultiplied 一貫性・トリム座標不変** — バグ源。golden（画素一致・配置座標＋trim_offset 等価）で固定。
- **WIC テストの COM init 依存** — デコード trait 隔離で正規化以降を COM 非依存化し緩和。純粋テストは COM 不要。
- **間接参照解決の誤り（素材欠落）** — golden 列挙テスト（emo2 surface1000→helper 全解決）で固定。循環検出で無限ループ回避。
- **WIC ユーティリティ移設リグレッション（D2）** — 挙動不変リファクタとして既存 bitmap_source テスト緑を維持。

---

## 5. 複雑度・リスク（Effort / Risk）

- **Effort: M（3〜7 日）**。既存 WIC 経路・α 走査パターン・Arc 規約を流用、emo2 実装腕は α のみ。新規要素＝間接参照解決・デコード trait 隔離・`rectangle-pack` 結線＋padding ラップ＋複数頁・emo-compose 共有契約。
- **Risk: Medium**。packing 承認ゲート・premultiplied/トリム意味論（テストで固定可）・WIC 移設（挙動不変）。D1 は設計で解決済（残余は隣接ユニット）。

---

## 6. 設計フェーズへの推奨（Recommendations）— 反映済

- 採用アプローチ: 純粋コア＋デコードポート（新クレート `areka-emo-atlas`・WIC 既定腕再利用）。三層直列（列挙／正規化+トリム／packing+表）を純粋関数で構成、成果物は `Send`＋`Arc` 共有。
- 設計冒頭で確定した鍵決定: D3 契約正本・D2 クレート境界/WIC 所在・D1（設計解決・残余は隣接）・D4 デコード trait・D5 use_self_alpha×.pna 動作表。
- 承認事項: `rectangle-pack`（zero-dep・MIT/Apache）を design で正式申請。
- 持ち越し（隣接ユニット）: balloon→surface 適合の所有者（design.md Open Questions #1）。

---

## References
- ukadoc `seriko.use_self_alpha`（shell）: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#seriko.use_self_alpha_2c_5024:1
- ukadoc `use_self_alpha`（balloon）: https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#use_self_alpha_2c_5024:1
- `rectangle-pack`（crates.io・zero-dep・MIT/Apache・複数 bin packing）
- 主要参照ファイル: `crates/wintf/src/com/wic.rs`・`crates/wintf/src/ecs/widget/bitmap_source/{systems.rs,alpha_mask.rs}`・`crates/areka-parsers/src/shell/model.rs`・`crates/areka-parsers/src/balloon/model.rs`・`crates/pilot/examples/shiori-host-32/fixtures/emo2/`
- 適合スコープ正本: `doc/emo2-conformance-scope.md`
- steering: `.kiro/steering/{tech.md,structure.md,roadmap.md}`
