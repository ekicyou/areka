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
  - **⚠ balloon モデルは幾何＋フォントのみをモデル化し、`balloons0.png`/`balloonk0.png` 等の画像パスも `use_self_alpha` も一切保持しない**（`parse.rs` の「image」は s0s/k0s 差分マージ層の意で画像ファイルではない）。要件 5.5 で choice/scroll 系を非モデル化した設計方針の帰結。→ **要件 1.4「balloon モデルが balloon 画像を含む」を満たす入力が現状の `BalloonModel` に存在しない（後述 §4 の主要ギャップ）**。
- **package**（`crates/areka-parsers/src/package/model.rs`）: `MountModel { names, shiori: ShioriMount{dir,file}, shell: ShellMount{dir} }`。**`shell.dir`（物理存在確認済み）は element パス解決の基準になる**が、**balloon dir は MountModel に含まれない**（balloon 所在解決は baseware 共有・スコープ外／記憶 areka-ghost-boot-descript-not-install）。

### 1.3 packing クレート（新規依存候補）

- ワークスペースに `rectangle-pack`/`rect_packer`/`texture_packer` は**未導入**（`Cargo.lock` はサブモジュール未展開の worktree ゆえ不在だが、`workspace.dependencies` にも該当なし）。
- brief 済み調査（2026-07-03）: 本命 **`rectangle-pack`**（zero-dep・MIT/Apache・静的バッチ packing・複数 bin＝頁対応・活発維持）。padding 非内蔵＝**矩形を 1〜2px 拡げて渡す自前ラップ**（自明）。対抗 `rect_packer`（zero-dep・MIT・padding 内蔵だが単頁 API＝複数頁 DIY）。棄却: `texture_packer`（`image` 強制依存＋休眠）・`guillotiere`/`etagere`（動的アロケータ＝bake-once に過剰）・`crunch`（回転不要）。
- **新規依存＝開発者承認必要**（encoding_rs 前例に倣う・要件確定済だが承認は design 議題）。

### 1.4 emo2 fixture 実測（透過腕の確定）

- 場所: `crates/pilot/examples/shiori-host-32/fixtures/emo2/`。
- shell descript（`shell/master/descript.txt`）: `charset,UTF-8`・`seriko.use_self_alpha,1`・`.pna` ファイル**無し**（`shell/master/**/*.pna` glob 0 件）→ **主実装腕＝PNG 自身の α チャンネル。キーカラー腕・`.pna` 腕は型シームのみ**（要件 3.5 に整合）。
- element PNG は `surface0.png`/`surface10.png`（トップ）＋ `CityPop/`（surfaceNNNN・ゼロ詰め variant）＋ `purple/<0..4,a>/` 配下（`base1.png`/`niko.png`/`ribbon.png` 等・サブディレクトリパス）。**ElementPath のまま素通し**（要件 1.5）。
- balloon（`emo2-kakukaku/`）: `balloons0.png`・`balloonk0.png`・`balloonc1..4.png` が実在。**α 規則は surface と同一**（`use_self_alpha,1`・balloon descript 側の同名キー）。
- surface1000 は `pattern0,overlay,1100..1800台,0,0,0` で helper surface を bind 参照（**間接参照の実例**）。element オフセットは全て `0,0`。

### 1.5 コード規約・配置（structure.md）

- クレート分割・責務分離。CPU 非依存純粋層は `XxxResource` 系（デバイスロスト非対応）、GPU 依存は `XxxGraphics`。**本層の成果物（アトラス表・頁バッファ）は CPU 保持の純粋データ＝`XxxResource` 系寄りだが、通信非依存・channel 非依存の独立層**。
- テスト: in-source `#[cfg(test)]` 主軸＋fixture スモーク。**WIC を使うテストは COM 初期化（`CoInitializeEx`）が必要**（wintf の既存テスト慣行）。
- 依存方向厳守・unsafe は COM ラッパー層に集約。

---

## 2. 要件→資産マップ（Requirement-to-Asset Map・ギャップ種別: Missing / Unknown / Constraint）

| 要件 | 必要な技術要素 | 既存資産 | ギャップ |
|---|---|---|---|
| **R1 マニフェスト導出** | shell/balloon モデルから element パス列挙・間接 bind 参照解決・重複排除 | shell `Element.path`/`Pattern.surface_id`・package `shell.dir` | **Missing**: 列挙器そのもの。**間接参照解決（surface_id→element パス）は本層新規**。balloon 画像パスは入力に無い（§4-D1） |
| **R2 デコード** | パス→画素バッファ・差替え可能 trait・エラー継続 | `load_bitmap_source`（PBGRA 化）・`copy_pixels`/`get_size` | **Constraint**: 既存は PBGRA 化を WIC 内で行い raw バッファ API を公開しない。**「デコード手段の trait 薄切り」は新規**。エラー継続（他エントリ処理継続）も新規 |
| **R3 透過正規化** | `use_self_alpha` 解釈・α>`.pna`>キーカラー優先・premultiplied BGRA 統一 | WIC PBGRA 変換（α有→採用/α無→不透明） | **Missing**: 伺か透過規則の解釈層。既存は透過規則を知らない。emo2 は α 腕のみ実装（他はシーム） |
| **R4 α トリミング** | α>0 タイト矩形算出・trim_offset/trimmed/original 記録・全透明→空 | `AlphaMask` の stride 込み α 走査パターン | **Missing**: トリム矩形算出・オフセット記録（走査パターンは流用可） |
| **R5 packing** | 静的バッチ配置・padding・複数頁・決定性・重複排除 | なし | **Missing**: `rectangle-pack` 新規導入（**承認要**）＋padding 自前ラップ |
| **R6 索引表 API** | AtlasKey→AtlasEntry・頁 premultiplied BGRA バッファ（stride 明示）・`Send`/`Arc` 共有・channel 非依存 | `XxxResource` 命名規約・`Arc` 手渡し規約（roadmap 並行モデル） | **Missing**: 型そのもの。**emo-compose と共有契約＝design 冒頭で確定必要** |

**非機能**: 決定性（同一入力→同一 packing）・premultiplied 一貫性（straight α 混入＝にじみ/暗縁）・スレッド安全な所有形（`Send`＋`Arc`）。いずれも**新規保証項目**だが既存基盤（WIC MTA・Arc 規約）と整合。

---

## 3. 実装アプローチ（Option A / B / C）

### Option A: wintf の bitmap_source を拡張して載せる

- **内容**: 既存 `bitmap_source/` に atlas モジュールを追加、`load_bitmap_source`/`copy_pixels` を直接呼ぶ。
- **トレードオフ**: ✅ WIC 経路の再結線ゼロ。❌ **本層は「通信非依存の純粋層・ECS 非依存・表示非依存」＝ECS ウィジェット（`bitmap_source` は Component＋on_add フック＋WintfTaskPool 前提）とは責務も依存形も異なる**。bitmap_source は ECS/D2D/GraphicsCore に密結合しており、純粋オフスクリーンテスト（要件の pass/fail 定義）と相反。**採らない筋が濃厚。**

### Option B: 新規クレート／モジュールとして分離（純粋層）— **推奨の第一候補**

- **内容**: emo 素材基盤を独立配置（新クレート `areka-emo-atlas` もしくは emo 系クレート内の `atlas` モジュール）。WIC デコードは `com/wic.rs` の ext トレイト（`create_decoder_from_filename`/`create_format_converter`/`copy_pixels`/`get_size`）を**再利用**するが、`bitmap_source` の ECS 機構には依存しない。三層直列（デコード＋正規化／トリミング／packing＋表）を純粋関数で構成。
- **配置判断（design 議題）**: (a) 新クレート `crates/areka-emo-atlas/`（クレート境界で純粋性を担保・parsers/shiori-abi の分離流儀に整合）か、(b) 既存 wintf 内の新モジュールか。**wic ext トレイトが wintf 内にあるため、新クレートにすると WIC 薄ラッパーの再配置／公開が必要**（§4-D2）。
- **トレードオフ**: ✅ 責務分離明快・純粋オフスクリーンテスト可・emo-compose と契約共有しやすい。✅ requirements の「純粋層」「channel 非依存」に直合致。❌ WIC ext トレイトの所在調整が要る（wintf 依存 or 薄ラッパー切出し）。

### Option C: ハイブリッド（WIC 薄ラッパーは共有・上位は新規純粋層）

- **内容**: WIC デコード最小面（decoder→PBGRA raw バッファ抽出）だけを共有可能な薄いユーティリティに保ち、その上の**正規化・トリミング・packing・表を新規純粋層**として積む。デコード trait の「既定腕＝WIC」を薄ラッパー越しに実装し、テスト時はモック腕（メモリ上 PBGRA）に差替え可能にする。
- **トレードオフ**: ✅ 要件 2.3（デコード手段差替え可・既定手段を上位に露出しない）に最も忠実。✅ COM/WIC 依存をデコード腕に隔離＝正規化以降のテストが COM init 不要になる。❌ 薄ラッパーの境界設計に一手間。**要件 2.3 と純粋テスト方針を両立する最有力構成＝B と C は実質連続（B の内部にデコード trait 隔離を入れると C）。**

**推奨の方向性（決定ではない）**: **Option B/C 連続体**——新規純粋層としてデコード trait 隔離（C）を内蔵しつつ、WIC 既定腕は `com/wic.rs` の ext を再利用。クレート境界（新クレート化）か wintf 内モジュールかは §4-D2 で design 決定。

---

## 4. 設計フェーズへ持ち越す設計判断・調査項目（Research Needed / 要件ディスカッションへ供給）

- **D1（最重要・要件↔既存モデルの不整合）balloon 画像パスの入力源**: 要件 1.4 は「balloon モデルが balloon 画像を含む」前提だが、**現状の `areka_parsers::balloon::BalloonModel` は画像パスも `use_self_alpha` も保持しない**（幾何＋フォントのみ）。balloon の element/画像列挙をどの入力から得るか未確定。候補: (a) balloon descript の生 KV／別途 balloon 画像列挙器を上流で用意し本層へ注入、(b) 本層が balloon 画像名を規約（`balloons*.png`/`balloonk*.png`）で受け取る薄い入力型を定義、(c) balloon 側 spec の拡張。**「本層は元ファイルを読みに行かない」（要件 1.1/3.6）ゆえ、balloon 画像パス列挙の責務境界を design 冒頭で確定必要。**
- **D2 本層の配置（クレート境界）と WIC ext の所在**: 新クレート `crates/areka-emo-atlas/` か wintf 内モジュールか。純粋性・emo チェーン共有・parsers 流儀からは新クレートが自然だが、WIC ext トレイト（`com/wic.rs`）は wintf 内。新クレート化する場合、(a) wintf へ依存、(b) WIC 薄ラッパーを共有クレートへ切出し、(c) デコード腕だけ areka 側実装、のいずれか。emo-compose/emo-present の配置とも整合させる。
- **D3 AtlasEntry/AtlasKey 契約の正本確定**: 要件 6 は `AtlasKey(path)→AtlasEntry{page, uv_rect, trim_offset, original_size}`＋頁バッファ（premultiplied BGRA・stride 明示）を規定。**この型は emo-compose と共有＝本層が正本を定義（compose 側で再定義しない）**。design 冒頭で両ユニット共通型として確定。空エントリ（全透明・転写スキップ）の表現（`Option`／専用 variant）も決定。
- **D4 デコード trait の形と COM 隔離度**: 要件 2.3「差替え可能・既定手段を上位に露出しない」を満たす trait 署名。既定腕＝WIC（COM init 必要）、テスト腕＝メモリ PBGRA。正規化以降を COM 非依存にできるか（＝COM init 不要な純粋テストの範囲）を design で確定。
- **D5 ukadoc 正典参照（design 着手時に MCP `get_doc`/`search_docs`）**:
  - `descript_shell` の `seriko.use_self_alpha`（0/1/full の値域）・`seriko.paint_transparent_region_black`（0/1・既定は pna 系=1/α系=0）。
  - balloon 側同名キーは `descript_balloon`（shell と別定義・両方読む）。
  - `.pna` の命名対応規則（`surfaceN.png`⇔`surfaceN.pna`・ukadoc 記載薄＝SSP de-facto 確認）。
  - element サブディレクトリ配置の正当性（`CityPop/`/`purple/`・de-facto 有効・ukadoc 明文なし→寛容に受ける）。
  - file-only surface（`surfaceN.png` 直参照・surfaces.txt 未定義）は ukadoc 上有効＝**シームのみ**（emo2 は全 surface 定義済で不要）。
  - `surfacetable.txt`＝表示名定義＝アトラス対象外の確認。`seriko.dpi`＝M1 等倍・キー存在のみ記録。
  - **design 冒頭で `use_self_alpha` × `.pna` 有無の 2×2 動作表（0/1/full × pna 有/無）**を作り、emo2 実測（=1・pna 無）以外はシームと明示。
- **D6 間接 bind 参照解決の走査規約**: surface1000 の `pattern.surface_id` → helper surface（1100 等）の element パスを解決する走査。負 surface_id（レイヤクリア）・`Range` append・alias は列挙対象外（画像を持たない）として除外する規約を確定。循環参照検出（surface→surface）の要否（emo2 は 1 段だが構造として）。
- **D7 padding・頁サイズ・決定性の具体値**: 頁サイズ 2048（必要時 4096）・padding 1〜2px・矩形拡張ラップの UV は padding 非包含。`rectangle-pack` の決定性を golden テストで固定する入力ソート順（path 昇順等）を確定。
- **D8 premultiplied 統一の実施点**: WIC は既に PBGRA を返すが、**キーカラー腕・`.pna` 腕を実装する場合の premultiply は自前**。emo2 は α 腕のみゆえ WIC 出力が既に premultiplied＝追加処理不要だが、シーム型の premultiply 契約を型で明示。

---

## 5. 複雑度・リスク（Effort / Risk）

- **Effort: M（3〜7 日）**。既存 WIC 経路・α 走査パターン・Arc 規約を流用でき、emo2 実装腕は α のみ（キーカラー/pna はシーム）。ただし (a) マニフェスト導出の間接参照解決、(b) デコード trait 隔離、(c) `rectangle-pack` 結線＋padding ラップ＋複数頁、(d) emo-compose 共有契約の設計、で新規要素が複数。
- **Risk: Medium**。
  - packing 新規依存（`rectangle-pack`）＝**承認ゲート**（未承認なら着手不可・fallback `rect_packer`）。
  - **D1（balloon 画像パスの入力源不在）が最大の未知**——要件と既存 parser モデルの不整合ゆえ、責務境界の設計判断が必要（コード規模は小だが設計影響あり）。
  - premultiplied 一貫性・トリム意味論（配置不変）はバグ源だが、テストで固定可能（golden／画素一致）。
  - WIC テストの COM init 依存＝デコード trait 隔離で正規化以降を COM 非依存化すれば緩和。

---

## 6. 設計フェーズへの推奨（Recommendations）

- **推奨アプローチ**: Option B/C 連続体（新規純粋層＋デコード trait 隔離、WIC 既定腕は `com/wic.rs` ext 再利用）。三層直列（デコード＋正規化／トリミング／packing＋表）を純粋関数で構成し、成果物は `Send`＋`Arc` 共有形。
- **設計冒頭で確定すべき鍵決定**: ①AtlasEntry/AtlasKey 共有契約（D3・emo-compose 正本）②本層のクレート境界と WIC ext 所在（D2）③balloon 画像パスの入力責務（D1）④デコード trait 署名と COM 隔離度（D4）⑤`use_self_alpha`×`.pna` 2×2 動作表（D5）。
- **承認事項**: `rectangle-pack` 新規依存（zero-dep・MIT/Apache）を design で正式申請（encoding_rs 前例）。
- **持ち越し調査**: D5 の ukadoc 正典確認（MCP）・D6 間接参照解決規約・D7 padding/頁/決定性の具体値。

---

## 付録: 主要参照ファイル（絶対パス）

- WIC 経路: `crates/wintf/src/com/wic.rs`・`crates/wintf/src/ecs/widget/bitmap_source/{wic_core.rs,systems.rs,resource.rs,alpha_mask.rs}`
- 上流モデル: `crates/areka-parsers/src/shell/model.rs`・`crates/areka-parsers/src/balloon/model.rs`・`crates/areka-parsers/src/package/model.rs`
- emo2 fixture: `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/{descript.txt,surfaces.txt}`・同 `emo2-kakukaku/`（balloon 画像）
- 適合スコープ正本: `doc/emo2-conformance-scope.md`
- steering: `.kiro/steering/{tech.md,structure.md,roadmap.md}`
</content>
</invoke>
