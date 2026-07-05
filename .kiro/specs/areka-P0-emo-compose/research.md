# ギャップ分析 (research.md) — areka-P0-emo-compose

> 対象: emo 三段直列チェーン **2/3**（emo-atlas → **emo-compose** → emo-present）＝合成コア。
> 前提: requirements.md・spec.json は確定済み（本分析では変更しない）。
> 本書は**情報提供**であり最終決定ではない。design フェーズと要件ディスカッションの判断材料。

## 1. サマリ（3-5 bullet）

- **新設クレート**: `crates/areka-emo-compose` は**未存在**（greenfield）。上流 `areka-emo-atlas`（✅ 完了）と `areka-parsers::shell`（✅ 完了）の実 API を**正本として消費**し、下流 `emo-present`/`seriko`/`collision-geometry` へ正規化 Surface 定義と `ComposedSurface` を供給する。既存コンポーネントの拡張ではなく **Option B（新設）** が基調。
- **上流契約は実在・安定**: `AtlasTable::{resolve, entry, page, pages}` と `Placement{page, uv_rect, trim_offset}`／`AtlasEntry{original, placement: Option<Placement>}`（None=転写スキップ）は table.rs で確定。`Shell{surfaces, appends, aliases}` と各転記型（`Surface`/`Element`/`Animation`/`Pattern`/`Collision`/`SurfaceAppend`/`AppendTarget`/`SurfaceAlias`）も model.rs で確定。**これらを再定義しない**（要件 Adjacent expectations）。
- **主な欠落能力**: (1)実サーフェスツリー構築（疎 id 解決・append 範囲展開・alias 解決）(2)合成プラン導出（レイヤ順・変換行列・合成メソッド）(3)アトラス転写による premultiplied BGRA 1枚物合成(4)入れ子再帰＋循環検出(5)合成メソッド写像表（全量列挙・emo2 使用分=overlay のみ実装）(6)`ComposedSurface` 出力契約(7)オフスクリーン pixel golden テスト。**いずれも現行コードベースに存在しない**。
- **テスト経路は確立済み**: `MemoryDecoder`（WIC/COM 不要インメモリデコーダ）＋`bake()` で `AtlasTable` を headless に構築でき、emo2 fixture（`crates/pilot/examples/shiori-host-32/fixtures/emo2/`）を `areka_parsers::shell::parse` で読める。**pixel golden テストは COM 非依存に組める**（brief 記載どおり実証）。
- **バックエンド選定が最大の design 判断**: CPU ピクセル演算 vs D2D オフスクリーン。emo2 使用メソッド=`overlay`（SourceOver 相当）のみ実装ゆえ **CPU 開始が有力**だが、合成コア API をバックエンド非依存に切ることが要件（brief Approach 5）。

## 2. 上流契約の実シグネチャ（正本・design はこれを消費し再定義しない）

### 2.1 areka-emo-atlas（crates/areka-emo-atlas/src/table.rs, lib.rs）

公開 re-export（lib.rs L140-141）:

    pub use table::{ AtlasEntry, AtlasKey, AtlasPage, AtlasTable, ElementId, Placement, Point, Rect, SetId, Size };

- `struct ElementId(pub u32)` — ランタイム密 index（O(1) 引き）。
- `struct SetId(pub u32)` — 出所セット序数。
- `struct AtlasKey { set: SetId, rel_path: String }` — ソースキー（無改変相対パス）。
- `struct Point { x: i32, y: i32 }` / `struct Size { w: u32, h: u32 }` / `struct Rect { x: u32, y: u32, w: u32, h: u32 }` — 幾何プリミティブ（**wintf 非依存の自前型**）。
- `struct AtlasEntry { original: Size, placement: Option<Placement> }` — **placement=None が全透明＝転写スキップ**（要件 6.3）。
- `struct Placement { page: u32, uv_rect: Rect, trim_offset: Point }` — **brief の簡略表記 `page,x,y` ではなく uv_rect（頁内 UV・padding 非包含）＋trim_offset（原画像内 bbox 左上）**。転写元は page 頁の uv_rect、転写先座標は「element 配置座標＋trim_offset」（要件 6.2）。
- `struct AtlasPage { width: u32, height: u32, stride: u32, bytes: Arc<[u8]> }` — **premultiplied BGRA・stride 明示・Arc 共有**。

AtlasTable の消費 API（メソッド）:
- `fn resolve(&self, set: SetId, rel_path: &str) -> Option<ElementId>` — **構築時一度きり**（未知=None）。emo-compose はツリー構築時に resolve し以後 ElementId を保持。
- `fn entry(&self, id: ElementId) -> &AtlasEntry` — **毎フレーム O(1)**（範囲外 id は panic＝密 index 契約違反）。
- `fn page(&self, index: u32) -> Option<&AtlasPage>` / `fn pages(&self) -> &[AtlasPage]`。
- `fn key(&self, id) -> &AtlasKey` / `len()` / `is_empty()`。
- AtlasTable/AtlasPage は `Send + Sync`・`Clone`（Arc 共有ゆえ安価）を実証済（table.rs test）。

AtlasTable の**構築経路**（emo-compose がアトラスをどう得るか）:
- `pub fn bake(sets: &[SurfaceSet], decoder: &impl ElementDecoder, cfg: PackConfig) -> BakeResult { table: AtlasTable, errors: Vec<BakeError> }`（lib.rs L56）。
- `struct SurfaceSet<'a> { surfaces: &'a [shell::Surface], base_dir: &'a Path, alpha_params: AlphaParams }`（manifest.rs L21）。
- **テスト用**: `MemoryDecoder`（decode.rs L76）に `insert(path, w, h, stride, bgra, has_alpha)` で画像を積む→bake で AtlasTable を **COM/WIC 不要**に得る。emo-compose の pixel golden test はこの経路を使える。

### 2.2 areka-parsers::shell（crates/areka-parsers/src/shell/model.rs, mod.rs）

公開 re-export（mod.rs L42-46）:

    pub use model::{ Animation, AliasKey, AppendTarget, Collision, CollisionName, Element, ElementPath, Interval, Pattern, Shell, Surface, SurfaceAlias, SurfaceAppend };
    pub use parse::parse;   // pub fn parse(input: &str) -> Shell

- `struct Shell { surfaces: Vec<Surface>, appends: Vec<SurfaceAppend>, aliases: Vec<SurfaceAlias> }` — **surface.append と alias は転記のまま**（未展開）。展開は emo-compose の責務。
- `struct Surface { id: u32, elements: Vec<Element>, collisions: Vec<Collision>, animations: Vec<Animation> }` — **この公開形をそのまま正規化の入力にできる**（既に collisions/animations を保持）。
- `struct Element { layer: u32, path: ElementPath, x: i64, y: i64 }` — **x/y は i64**（注: atlas Point/trim_offset は i32。転写先座標算出で型合わせが要る＝design 判断）。
- `struct Animation { id: u32, interval: Interval, patterns: Vec<Pattern> }`。
- `enum Interval { Bind, Random{k}, BindRandom{k} }`（`#[non_exhaustive]`）— **Bind が MAYUNA bind 判定の鍵**（surface1000 の全パーツ）。
- `struct Pattern { index: u32, surface_id: i64, wait: u32, x: i64, y: i64 }` — **surface_id < 0 はレイヤクリア/停止センチネル**（下流解釈）。pattern0 overlay を bind 静的合成に使う。
- `struct Collision { index: u32, left/top/right/bottom: i64, name: CollisionName }`（矩形のみ・要件は enum 全量シームだが型は現状矩形単独）。
- `struct SurfaceAppend { targets: Vec<AppendTarget>, collisions: Vec<Collision>, animations: Vec<Animation> }`。
- `enum AppendTarget { Single(u32), Range{start, end} }`（`#[non_exhaustive]`・**両端含む**・展開は下流=emo-compose）。
- `struct SurfaceAlias { key: AliasKey, ids: Vec<u32> }` — **kero.surface.alias のみ**（decode.rs 実装確認）。

## 3. 要件→資産マップ（Missing / Constraint / Reusable）

| 要件 | 必要能力 | 現状 | タグ |
|---|---|---|---|
| R1 実ツリー構築 | Shell→正規化 Surface 定義（疎 id 解決・collision/animation 保持） | 未存在。Surface 公開形は流用可 | Missing |
| R2 append 範囲展開 | AppendTarget::Range{start,end} を実 id へ展開・出現順適用 | SurfaceAppend/AppendTarget 転記型のみ・展開ロジックなし | Missing |
| R3 alias 解決 | SurfaceAlias{key,ids} を順序付き id へ・重複/欠落を決定的に | Shell.aliases 保持のみ・解決なし | Missing |
| R4 合成プラン | レイヤ順・変換行列・合成メソッド・アトラス/入れ子参照の命令列 | 未存在 | Missing |
| R5 bind 静的合成 | compose(surface_id, active_binds)・pattern0 overlay を animation ID 昇順 | 未存在。Interval::Bind/Pattern は流用 | Missing |
| R6 アトラス転写合成 | AtlasTable 頁→合成先バッファへ premultiplied SourceOver・trim_offset 転写 | 未存在。AtlasPage.bytes/Placement は流用 | Missing |
| R7 入れ子再帰＋循環検出 | 訪問集合で非パニック打ち切り | 未存在（atlas manifest に**類似実装 resolve_indirect+visited** が参考例として在る） | Missing (参考あり) |
| R8 合成メソッド写像表 | ukadoc 全量列挙・overlay のみ実装・他は型シーム | 未存在。正典は ukadoc MCP | Missing + Research |
| R9 ComposedSurface 出力 | premultiplied BGRA・size・stride・Send 所有 | 未存在。AtlasPage が形の参考 | Missing |
| R10 決定性・純粋層 | バイト等価・整数/固定小数・バッファ再利用・通信非依存 | atlas が同規律で実装済（踏襲元） | Constraint |
| R11 pixel 観測 | emo2 fixture のオフスクリーン golden | fixture 実在・MemoryDecoder+parse 経路確立 | Reusable |
| R12 制約遵守 | Rust 2024・tokio 禁止・新規依存なし・drift 修正 | ワークスペース規律確認済・drift 1 箇所特定 | Constraint |

## 4. 実装アプローチ選択肢

### Option A: 既存クレート拡張（areka-emo-atlas に合成段を足す）
- **却下寄り**。atlas は「素材基盤（bake まで）」の単一責務が明確で、合成段を混ぜると責務肥大。brief/roadmap は**三段直列を別ユニット**と明示。emo-atlas は完了済で不変が望ましい。
- 利点: 依存追加不要。 欠点: 責務混濁・完了済クレートへの侵襲・テスト境界の崩れ。

### Option B: 新設クレート areka-emo-compose（推奨基調）
- `crates/areka-emo-compose` を新設し Cargo.toml に areka-parsers（path）＋areka-emo-atlas（path）＋tracing（workspace）を依存。**新規外部依存なし**（要件 12.2）。合成コアは std のみ。
- 内部三層（brief Boundary Candidates）: (1)ツリー構築（Shell→正規化 Surface 定義・純粋）(2)プラン生成（定義→命令列・純粋）(3)転写実行（命令列＋AtlasTable→画素）。
- 利点: 責務分離・独立テスト容易・上流不変。 欠点: ファイル増・公開 API 設計を要する（が要件で境界が既に明確）。
- **Effort: L（1-2週）／Risk: Medium**（新パターンだが上流契約が確定・ガイドが厚い）。

### Option C: ハイブリッド（段階導入）
- Phase1: CPU バックエンドで overlay のみ実装＋ツリー構築＋pixel golden 緑化。Phase2: 必要時に D2D バックエンドへ差替え（API 非依存を最初から）。写像表/変換行列/入れ子/循環検出の**構造**は Phase1 から保持（要件 12.3）。
- 利点: 最小実装ファースト・M1 規律適合・リスク逓減。 欠点: バックエンド抽象の口を最初に正しく切る計画が要る。
- **本フィーチャの現実解は B の器の中で C の段階戦略**（新設クレート＋CPU 先行＋バックエンド抽象シーム）。

## 5. Research Needed（design フェーズへ持ち越す調査項目）

1. **合成式の de-facto 確定（ukadoc MCP 必読）**: overlay/overlayfast/replace/base/reduce/asis/interpolate/add/bind/blend-* の透明度合成式。ukadoc は式未明文の箇所あり＝**SSP 実挙動が de-facto**。写像表に「合成式（確定/de-facto/未確定）」列を設け、emo2 使用分（overlay）のみ実測確定。get_doc 対象: descript_shell_surfaces の element*/animation*.pattern*/animation-sort/各描画メソッド個別ページ。
2. **animation-sort（ascend/descend）の扱い** — **【議題1で解決済み】**: bind 静的合成の順序は「animation-sort → ID 順」の2段。ukadoc `descript_shell_surfaces` で **`animation-sort` は実在キー・既定 `descend`**（兄弟に `collision-sort`・既定 none）と確定。現行 parser は passthrough 吸収で未転記（decode.rs L33）＝**転記層の欠落**。emo2 surfaces.txt は `animation-sort` 未指定＝既定 descend 適用。**決定: 本チェーン内で areka-parsers::shell へ animation-sort/collision-sort の転記を追加し（要件 12.5）、emo-compose は 2段規則で消費（要件 5.3/5.6・1.6）。** descend/ascend が画素積層へ効く de-facto 方向のみ design 冒頭の ukadoc 実測へ持ち越し。
3. **バックエンド選定（CPU vs D2D）**: 判断材料＝(1)ピクセル忠実性（reduce 等の生ピクセル演算要否・emo2 は overlay のみゆえ CPU 素直）(2)毎フレーム再合成コスト（M-life seriko-loop・O(elements)）(3)スレッド制約（D2D device context 単一スレッド・WUC upload は UI スレッド）(4)golden 安定性（headless・整数演算）。合成コア API はバックエンド非依存に切る。
4. **座標型の整合**: Element.x/y: i64・Pattern.x/y: i64 vs atlas Point.x/y: i32・trim_offset: i32・Rect: u32。転写先座標算出（配置座標＋trim_offset）と合成先バッファ境界クリップでの型変換・オーバーフロー/負座標クリップ方針を design で確定。
5. **アトラスをどう受け取るか（API 境界）**: compose() の入力に AtlasTable を借用で渡すか、SurfaceSet→bake を emo-compose 内で行うか。R6.1 は「転写命令列とアトラス（AtlasTable）が与えられたとき」＝**AtlasTable を借用入力とするのが素直**（bake は呼び手/統合層の責務）。pixel test では test 内で MemoryDecoder+bake して渡す。
6. **surface.append の正確な意味論** — **【議題2で解決済み】**: ukadoc/satori 実例で確定——(a) `surface.append` は **plain `surfaceN` と異なり「既存 surface のみに追記・非存在 id を新設しない」存在条件付き**（plain `surface1-3` は 1/2/3 を全新設）。(b) ターゲットは単一・カンマ列挙・範囲・**除外 `!`**。(c) append ブロックは **element も持てる**（現行 `SurfaceAppend` は elements 欠落＝転記ギャップ）。(d) 素の `surface1,3`／`surface1-3` も正典書式だが**現行 parser は単一 `parse::<u32>` で破損**（`surface1-3`→id=0 化・`surface1,3`→surface3 黙殺）。**決定: parser は記述のまま忠実転記（多 id ヘッダ・append-element・除外記述子・登場順の単一ストリーム）、意味論（create/append・存在条件・展開）は emo の single-pass fold が担う（要件 2・12.5・Boundary）。**
7. **入れ子 surface 参照 vs bind 参照の区別**: R7 の「入れ子 surface 参照」と R5 の「bind の pattern0 参照」の合成上の関係整理。atlas の resolve_indirect（Pattern.surface_id を辿る）が参考。循環検出の訪問集合実装は atlas manifest.rs L97-126 が既存参考例。

## 5b. アーキテクチャ決定（議題2追加・要 design 詳細化）

8. **データ管理＝`bevy_ecs` 基盤** — **【議題2で決定】**: `bevy_ecs` は `[workspace.dependencies]` 正式メンバーで **wintf（`src/ecs/` 全域＝window/widget/graphics/clickthrough）と areka 本体が既採用**＝新規依存ではない。emo は wintf UI スレッド常駐ゆえ ECS World の住人。**旧「純粋 std-only 層」前提は失効**し、内部サーフェス合成ツリーを entity/component で管理して大規模サーフェスデータを高速処理する（要件 1.8・10.4/10.6・12.2）。**World 所有＝【議題2で決定】emo 専用 per-ghost World（wintf 本体 World とは分離）**——根拠: wintf アーキタイプ汚染回避／per-ghost lifecycle（World drop で一掃）／headless テスト容易（R11.4・wintf 無しで World 構築）／層境界補強（wintf World/component 非依存）／emo→表示は `ComposedSurface` 値ハンドオフゆえ共有 entity 不要。**design 詳細**: (a) component スキーマ（surface/element/bind/animation/collision と layer/transform/method/sort/atlas-ref）、(b) single-pass 構築を system 化するか、(c) 2 World（emo/wintf）の UI スレッド上 tick 協調・handoff 機構（→ wintf `bitmap_source` widget）、(d) seriko（毎フレーム変異）との相互作用（emo World 内 system か message か）。
9. **スケール規律（破綻回避の要）**: World は**定義・構造 component のみ保持**、**合成済みビットマップ（1 surface 数百KB〜数MB×多数）は永続保持しない**（要件 10.6）。materialize はオンデマンド、キャッシュ/無効化は emo-present（要件 9.4）。持たせると N surface×MB×ゴーストでメモリ破綻。
10. **並行モデルへの影響（memory 追随要）**: memory `areka-concurrency-model` は「actor＋channel・大型データ Arc 手渡し」を基調とするが、emo のサーフェス状態は **UI スレッド共有 ECS World** に載る方向＝emo/seriko 間は「World 共有」寄り。design で actor 境界と ECS World の役割分担を確定し、確定後に memory を更新すること。

## 6. 制約・既知ドリフト

- **Rust 2024・tokio 禁止**（要件 12.1）。**依存は workspace 既存基盤（`bevy_ecs`・tracing・areka-parsers・areka-emo-atlas）のみ**——旧「std-only」は bevy_ecs 既採用により失効（要件 12.2・議題2）。
- **失敗経路のログ規律**（記憶 areka-log-first-no-silent-failure）: 欠落 id/未解決 alias/未実装メソッド/循環は warn 以上、合成失敗は error＋Err 戻り値、panic は致命限定＋直前ログ（要件 1.4/3.3/7.3/8.4/10.5）。
- **premultiplied 一貫性**: SourceOver `dst = src + dst*(1-src_a)`・straight α 混在禁止（要件 6.4）。atlas 頁は既に premultiplied BGRA。
- **DPI 非持込**: 合成はピクセル等倍＝surface 原寸（要件 6.5）。拡縮は wintf 側。
- **キャッシュ非保持**: surface id→合成結果のキャッシュ/無効化は emo-present の責務（要件 9.4）。
- **既知ドリフト（要件 12.4）**: `crates/areka-parsers/src/balloon/model.rs:6` の doc コメントに旧エンジン名 `areka-P0-text-layer`/`areka-P0-surface-engine` 参照が残存（grep 実確認）。現行エンジン固有名（emo-text-layer 等）へ本チェーン着手時に追随修正。

## 7. design フェーズ推奨（優先アプローチと持ち越し）

- **優先**: Option B（新設 areka-emo-compose）の器で Option C の段階戦略（CPU バックエンド先行＋バックエンド抽象シーム＋写像表/行列/入れ子/循環検出の構造を最初から）。
- **上流契約は §2 の実シグネチャを正本として消費**（再定義しない）。特に `Placement{page, uv_rect, trim_offset}` は brief の簡略表記 `page,x,y` と異なる点を design で明示追随。
- **持ち越し研究**: §5 の 7 項目（特に 1・2・6 は ukadoc MCP get_doc/search_docs を design 冒頭で実行）。

---

## 8. Design フェーズ discovery（2026-07-05 追記・design.md 生成時）

### 8.1 ukadoc 実測（MCP get_doc/search_docs・持ち越し §5-1/2/6 の解決）

- **animation-sort の画素積層方向＝解決**: ukadoc 明文「手前から奥にどの順で表示するか。ascend で昇順(1,2,3...)、descend で降順(10,9,8...)。既定 descend」。よって **descend（既定）＝大 ID が手前＝画家のアルゴリズムでは animation ID 昇順に描画**、ascend＝小 ID が手前＝ID 降順に描画。brief の「ID 昇順合成」は既定 descend と一致（design.md 決定5）。
- **collision-sort**: 既定 **none**＝「ID によらず先に書かれている方が手前」。ascend/descend で ID 順ソート可。compose の画素経路には不使用・正規化結果へ引き継ぎ（R1.6）。
- **animation*.pattern***: サーフェス番号 **-1＝そのアニメ停止・-2＝全アニメ停止**のセンチネル（描画なし・メソッド/XY 無視）。パターンは番号の小さい方から積み重なる。alias 名指定可（SSP 2.8.25+）。旧書式 `*pattern*,ID,ウェイト,メソッド,X,Y` あり（現行転記層は吸収）。オプション `alpha,値`（2.8.37+）。
- **element***: element0 は surface*.png を破棄して置換・element1 以降は上に順次合成。オプション clipping（2.8.15+）/alpha（2.8.37+）/source＋auto（2.8.41+）。
- **各描画メソッド**（写像表は design.md に全量・式ステータス列付きで確定）: overlay=単純重ね（式未明文→premultiplied SourceOver を de-facto 式に採用）／add・bind=**overlay と同義が明文**（確定）→ Overlay へ写像／overlay-fast=ベース不透明度変調・interpolate=その対（ベース透明度変調＝DestOver 相当）／replace=src 範囲内αごと上書き・**範囲外は無操作**（透明扱いでない）／asis=src 透過無視（不透明扱い・透過域は黒挙動）／base=全置換＋collision 更新・XY 無視・先頭以外 overlay 読替／reduce=不透明度乗算（切り抜き）・RGB 無視・**src 範囲外は透明相当＝消去**／blend-* 群=Photoshop 相当（add/multiply/screen/overlay/darken/lighten/darker-color/lighter-color/color-burn/color-dodge/hard-light/hard-mix/difference/exclusion/divide/hue/color 等＋各 -fast 変種・旧称 overlaymultiply=blend-multiply-fast・overlayscreen=blend-screen-fast）。
- **collisionex タイプ**: rect/ellipse/circle/polygon/region（M1 転記層は矩形 collision のみ＝現行契約・型多様化は将来）。

### 8.2 コードベース実測（design 時の追加確認）

- **転記層は非 overlay の element/pattern 行を吸収する**（validation_tests.rs 要件 4.5/5.7 で固定済みの既存契約）。`Element`/`Pattern` に描画メソッドのフィールドは**存在しない**。→ 写像表・未実装 warn 経路（R8.4）は **compose 側の型シーム＋plan 直接構築テスト**で担保し、M1 の plan 命令は常に Overlay。parser へのメソッド転記追加は将来の転記層拡張＝revalidation trigger として design.md に記録（12.5 の4ギャップに含めない＝スコープ遵守）。
- plain ヘッダ破損箇所の実体: decode.rs **L127** `rest.parse::<u32>().unwrap_or(0)`。TopLevel 破棄: decode.rs **L87**。append 内 element: append 経路が element 行を処理しない（黙殺）。3 Vec は各々出現順保持だが種別間 interleaving なし——4ギャップの実在を確認。
- **emo2 fixture 実測**: surface 定義 59 本・charset UTF-8・`animation-sort`/`collision-sort` **未指定**（既定適用の実証系）・surface.append 5 ブロック（`surface.append10,2100-2110,2200-2210` の多ターゲット範囲あり）・alias 重複キーあり（`100,[2100]` が2回）・surface1000 は collision 2件（Head/Bust）＋bind 系 animation（1100〜1801）のみで静的 element ゼロ。
- **bevy_ecs**: workspace 0.18.0（features: std/multi_threaded/serialize 等）。wintf は単一 World を `Rc<RefCell<EcsWorld>>` で UI スレッド所有。第2 World の前例はないが障害なし。
- **atlas キー整合**: bake は `ElementPath.as_str()` を無加工で `AtlasKey.rel_path` に用いる（正規化なし）→ emo-compose の resolve キーも `ElementPath` 無加工でよい（不一致の余地なし）。

### 8.3 Design Decisions（design.md 本文の決定の記録）

#### Decision: バックエンド＝CPU 整数演算・Backend trait は設けない
- **Alternatives**: (A) D2D オフスクリーン (B) CPU＋Backend trait 抽象 (C) CPU＋plan/execute データ分離。
- **Selected**: (C)。plan（`Vec<BlitOp>`）が既にバックエンド非依存データであり、trait は「実装1つの投機的抽象」（synthesis 簡素化レンズ）。D2D は golden の決定性（ドライバ丸め差）と headless 性で不利、emo2=overlay のみで CPU が素直。
- **Trade-offs**: 将来 D2D 化時は別 executor を追加（plan は不変）。

#### Decision: 入れ子参照は plan 時 flatten（中間バッファなし）
- **Rationale**: SourceOver は結合的なので overlay のみの M1 では「入れ子を合成してから転写」＝「入れ子の各層をオフセット累積して直接転写」が画素等価。中間バッファ・再帰実行を排し R10.3（途中アロケーションなし）を構造的に満たす。
- **Trade-offs**: 非 overlay の入れ子参照メソッドが実装される際は中間バッファ方式の再導入が必要（シームとして記録・revalidation trigger）。循環検出は flatten 再帰の visited 集合（atlas manifest.rs の前例踏襲）。

#### Decision: EmoWorld は受動データストア（Schedule/System 非所有）
- **Rationale**: R1.8 の「system として実装し得る」は component スキーマの担保で満たし、fold/compose は `&mut World`/`&World` を取る決定的な同期関数とする。wintf schedule との tick 協調は本 spec では発生しない（emo-present が UI スレッド上で同期呼び出し）。seriko 統合時の system 化・協調設計は seriko 側 spec の領分（§5b-8(c)(d) の残課題をそちらへ送る）。
- **ECS 粒度**: entity＝surface 1件（`SurfaceId`＋`SurfaceMaster`＋`AtlasBinding`）。animation/element の entity 分解はアーキタイプ増だけでスケールに寄与しないため不採用。将来の per-animation 動的状態は同 entity への component 追加または子 entity で拡張可能。

#### Decision: de-facto 規則の確定（ukadoc 無明文箇所）
- **plain surface 重複定義**: 後の定義が**全置換**（後勝ち）＋warn。ukadoc 明文なし＝de-facto。
- **append の同一 animation id**: 後勝ち置換＋warn。element/collision は末尾連結。
- **alias 重複キー**: 後勝ち（workspace KV 規約と整合）。
- **キャンバス外形**: 原点(0,0)固定・全描画層の `offset+original` の和集合・負オフセットはクリップ。element0 が(0,0)原寸の通常形では base 原寸と厳密一致・全 bind surface では bind 群の外形。SSP のはみ出し挙動（切るか広げるか）は未実測＝emo2 では差が生じないため de-facto 採用。
- **除外 `!` の適用**: R2.5 の shall に従い fold の展開時に減算を**実装**する（記述子保持は parser・「型シームに留めてよい」の許容より shall の直接充足を選択。実装コストは数行）。

#### Decision: 転記ギャップ(d)＝`Shell.definitions: Vec<DefRef>`（index 参照ストリーム）
- **Alternatives**: (A) Shell を単一 `Vec<Definition>` へ再構成 (B) 3 Vec 維持＋登場順 index ストリーム追加。
- **Selected**: (B)。(A) は emo-atlas の `&[shell::Surface]` スライス消費を壊す。(B) はデータ重複ゼロ・非破壊・登場順を完全保持。
- **合わせて**: `Surface.targets: Vec<AppendTarget>` 追加（`id` は記述子先頭の代表 id＝単一形は完全互換）、`SurfaceAppend.elements` 追加、`AppendTarget` に `Exclude`/`ExcludeRange` variant 追加（`#[non_exhaustive]` ゆえ非破壊）、`Shell.animation_sort`/`collision_sort: Option<SortOrder>`（未指定 None・既定解釈は下流）。

#### Decision: 合成先バッファの所有形＝`compose_into`（再利用）＋`compose`（便宜）
- **Rationale**: R9（Send 所有の値ハンドオフ）と R10.3（バッファ再利用）の両立。毎フレーム経路は emo-present が `ComposedSurface` を回して `compose_into`、定常状態ゼロアロケーション（初回/拡大時のみ確保＝運用解釈として記録）。

### 8.4 Risks & Mitigations（design 時点）

- **SSP 実挙動との de-facto 乖離**（重複定義規則・キャンバス外形）— warn ログで観測可能化＋研究記録済み。emo2 では差が生じない構成のため M1 リスク低。実ゴースト多様化時に実測で更新。
- **`Surface.id`＝代表 id への意味拡張** — 単一形は完全互換・多 id 形は現行が破損（id=0）ゆえ改善のみ。parser テストで固定。
- **メソッド転記の将来追加**（転記層拡張）— `ComposeMethod::Unknown` と registry の全量列挙で受け口確保済み。plan/blit の dispatch は追加 variant に match 網羅で追随。
- **並行モデル memory の追随**（§5b-10）— 本 design で「emo World＝UI スレッド共有データストア・emo→present は値ハンドオフ」と確定。**実装完了後に memory `areka-concurrency-model` を更新すること**（follow-up）。

### 8.5 References（design 追記分）

- ukadoc `descript_shell_surfaces`: animation-sort／collision-sort／element*／animation*.pattern*／overlay／overlay-fast／overlayfast／interpolate／replace／asis／base／add／bind／reduce／blend-* 各アンカー（MCP get_doc 実測・2026-07-05）
- satori wiki「簡単なシェルの揚げ方」（surface.append の実例・plain 多 id の実例）
- `crates/areka-parsers/src/shell/{model,decode}.rs`・`validation_tests.rs`（転記契約の実測）
- `crates/areka-emo-atlas/src/{table,manifest,decode}.rs`（正本契約・visited 前例・MemoryDecoder）
