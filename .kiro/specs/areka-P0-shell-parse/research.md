# ギャップ分析（research.md）— areka-P0-shell-parse

> 対象: emo2 `surfaces.txt`（SERIKO/2.0）→ 型付きシェルサーフェスモデル parser（`areka_parsers::shell` 新設）。
> 目的: 確定済み requirements.md と既存コードベースの差分を明らかにし、設計フェーズの判断材料を提供する（決定はしない）。
> 前提: requirements.md / spec.json は確定済（本書は変更しない）。spec language = `ja`。

## 0. 調査サマリ（3–5 bullets）

- **接ぎ木先は確立済**: `areka-parsers` クレートに `sakura`（`areka-P0-sakura-parse` 完了）と `charset`/`kv`（`areka-P0-parser-foundation` 完了）が既に在り、本 spec が踏襲すべき「`pub fn parse(&str)->Vec`／`Result` 無し寛容パス／NewType＋opaque＋accessor／`#[non_exhaustive]`／`tracing` のみ／in-source `#[cfg(test)]`」パターンは全て実物で観測可能。**依存の parser-foundation は既に completed**（ブロック要因なし）。
- **出力モデル型は未存在＝本 spec で新規定義**: surface 定義／element overlay／SERIKO animation・interval／collision 矩形／surface alias を表す型は codebase に無い（`wintf` の `SurfaceGraphics`/`VisualGraphics` は GPU リソースでありパーサ契約ではない）。これが本 spec の中核成果物。
- **surfaces.txt は sakura と構文クラスが異なる**: sakura はインライン走査（`\tag[args]`）だが surfaces.txt は**ブロック構造（`surfaceNNN { ... }`）＋ドット付きキー（`animation1100.interval`）＋行指向 CSV**。既存 `kv::parse_kv`（フラット `key,value` 後勝ちマップ）は**そのままでは不足**（ネスト・重複キー・`[id,...]` 配列値を潰す）。lexer/decode の二層は流用できるが lexer 実体は新規。
- **emo2 実物 fixture が唯一の適合基準**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt`（実読込済）に必要機能が全て現れる。`doc/emo2-conformance-scope.md` §2 が feature set の正本（interval 3種・overlay のみ・矩形 collision・全 offset `0,0`・alias 不透明）。過剰実装禁止。
- **要研究フラグは軽微**: 主に (a) fixture がクレート跨ぎのため in-source テストで `include_str!` を使えない規律（既存 kv/charset validation_tests が literal 直書きで回避済＝踏襲）、(b) `surface.append` 範囲展開の内部表現、(c) 重複 alias キーの保持表現、(d) surface ブロック内 collision と surface.append の統一表現。いずれも設計フェーズで解ける範囲。

## 1. 現状調査（Current State）

### 1.1 接ぎ木先クレート `areka-parsers`

- **Cargo.toml**（`crates/areka-parsers/Cargo.toml`）: `edition.workspace`（= Rust 2024）、依存は `encoding_rs`（foundation の charset デコード用）＋ `tracing` のみ。`publish = false`。
- **lib.rs**: `pub mod charset; pub mod kv; pub mod sakura;` の 3 兄弟。本 spec は `pub mod shell;` を 1 行追加する構造。
- **兄弟モジュールは非衝突**: 各モジュールは独立ディレクトリ・独立公開面。`shell` 追加は既存に触れない（要件 11.3・並走安全）。

### 1.2 参照パターン `sakura`（確立済・踏襲対象）

`crates/areka-parsers/src/sakura/` の構成と規律:

- **依存方向**: `model ← lexer ← decode ← parse`（`mod.rs` ヘッダに明記）。
- **`mod.rs`**（公開面集約）: 内部 `mod model/lexer/decode/parse` を private に持ち、`pub use model::{...}; pub use parse::parse;` で**公開面を一点集約**。テストは `#[cfg(test)] mod *_tests;` を各層に併置。
- **`model.rs`**（下流共有 I/O 契約）:
  - フラット単一 enum `Instruction`（`#[non_exhaustive]` + `#[derive(Clone, Debug, PartialEq)]`・`serde` 無し・`Eq`/`Hash` は `f32`/`Duration` を含むため付さない）。
  - 不透明 NewType（`SurfaceArg(String)` 等）: フィールド非公開・`new()` コンストラクタ＋`as_str()` read-only accessor。「dola `ActorKey` 流儀」と明記。
  - 寛容パススルー variant `Raw(String)`（意味未対応/不正を丸ごと吸収）。
- **`parse.rs`**（公開 facade）: `pub fn parse(input:&str)->Vec<Instruction> { decode(lex(input)) }` の一行合成。状態も I/O も持たない・`Result` を返さない・空入力で空 `Vec`・純粋決定的。
- **`lexer.rs`**（構文層）: `char_indices` の手書き線形スキャナ。内部トークン enum は `pub(crate)`（モジュール外非公開）。未閉じ境界は `Raw` 吸収し走査を中断しない。
- **`decode.rs`**（意味層）: 構文トークン→値正規化済み `Instruction`。emo2 subset のみを値正規化し、subset 外は passthrough シームへ委ねる。
- **テスト規律**: `validation_tests.rs` は公開 `parse` 経由の end-to-end アサーション。**期待値はリテラル直書き**（`include_str!` 不使用と明記＝クレート跨ぎ回避）。

### 1.3 parser-foundation（`charset` / `kv`・依存・完了済）

- **`kv::parse_kv(&str)->BTreeMap<String,String>`**: 行分割→最初のカンマで `split_once(',')`→trim→後勝ち→空行/カンマ無し行スキップ・値は文字列保持・**順序非保持**・`Result` 無し。素朴なフラット KV マップ。
- **`charset`**: charset 判定＋デコード（本 parser の**上流**。本 parser は UTF-8 デコード済 `&str` を入力に取り charset 判定は担わない・要件境界 §Out of scope）。
- validation_tests の採取元コメント（`fixtures/emo2/emo2-kakukaku/*`）から、**foundation は既に emo2 fixture ベースで検証済**という規律の前例がある。

### 1.4 emo2 fixture 解剖（`.../emo2/shell/master/surfaces.txt`・実読込済）

requirements の全機能が fixture に実在することを確認:

| 機能 | fixture 実例（行） |
|---|---|
| charset ヘッダ | `charset,UTF-8`（L1） |
| descript ブロック | `descript { version,1 }`（L2-5） |
| 単純 surface + 単一 element | `surface0 { element0,overlay,surface0.png,0,0 }`（L10-12） |
| surface 内 collision（矩形） | `collision0,93,62,271,130,Head` / `collision1,...,Bust`（L23-24） |
| animation interval `bind` | `animation1100.interval,bind`（L27） |
| animation pattern overlay | `animation1100.pattern0,overlay,1100,0,0,0`（L28） |
| interval `bind+random,K` | `animation1400.interval,bind+random,4`（L73） |
| 複数 element（2層） | `surface1410 { element0,...; element1,... }`（L211-215） |
| 負 ID overlay（層クリア） | `animation0.pattern3,overlay,-1,80,0,0`（L432） |
| interval `random,K` | `animation0.interval,random,4`（L429） |
| surface.append 単一 | `surface.append2200 { ... }`（L434） |
| surface.append 複数列挙＋範囲 | `surface.append10,2100-2110,2200-2210`（L415） |
| alias（数値キー・単値/複値） | `6,[2106,2206]`（L466） |
| alias（日本語キー） | `静観,[2106,2206]`（L478） |
| **重複 alias キー** | `100,[2100]` が L484 と L494 に 2 回（要件 8.4 の実根拠） |
| コメント・空行 | `//...`（L7 他多数）・空行 |

> **注記**: fixture は emo2 が使う範囲を全て含み、かつ subset 外機能（他 method/interval/collisionex/非 0 offset）は**一切現れない**。これが「emo2 使用分のみ実装」の実物担保。全 pattern の X/Y は末尾で、全 element offset は `0,0`。

## 2. 要件実現性分析（Requirements Feasibility）

### 2.1 要件 → 資産マップ（Missing / Unknown / Constraint タグ付き）

| 要件 | 技術的必要物 | 既存資産 | ギャップ |
|---|---|---|---|
| R1 モデル型定義 | surface/element/animation/interval/collision/alias 型・`#[non_exhaustive]`・opaque NewType | `sakura::model` が型付け流儀を提供 | **Missing**（型は本 spec で新規定義。流儀は流用可） |
| R2 寛容 facade | `pub fn parse(&str)->Model`（`Result` 無し・空入力空・純粋） | `sakura::parse` が template | **Missing**（実体新規・パターン既存） |
| R3 descript ヘッダ | `charset,V` 行 + `descript{version,N}` 解釈 | `kv::parse_kv` が単純 KV を提供 | **Constraint**（descript はブロック内 KV＝kv 部分流用可だが charset 行はブロック外） |
| R4 surface ブロック | `surfaceNNN{...}` 抽出 + element overlay 行 | 既存 lexer はインライン用 | **Missing**（ブロック lexer 新規） |
| R5 animation/interval | ドット付きキー `animationN.interval/.patternM` の集約 | 無 | **Missing**（`animationN` ごとに interval + pattern 群を束ねる集約が必要） |
| R6 collision 矩形 | `collisionN,l,t,r,b,name` 行 | 無 | **Missing**（CSV 行 decode・name は opaque） |
| R7 surface.append + 範囲 | `surface.appendNNN,tgt,range` のターゲット解決 | 無 | **Missing**（`2100-2110` 範囲展開ロジックが独自） |
| R8 alias 透過 | `KEY,[id,...]` の写像・キー opaque・重複保持 | `kv::parse_kv` は後勝ち＝**重複を潰す** | **Constraint**（kv 不適合。`[id,...]` 配列値も kv は非対応） |
| R9 コメント/空行/未知 | `//` 行・空行スキップ・未知吸収 | sakura lexer の Raw 吸収思想 | **Missing**（surfaces.txt 用に再実装） |
| R10 emo2 適合 | fixture pass の in-source テスト | validation_tests の literal 直書き前例 | **Constraint**（クレート跨ぎ＝`include_str!` 不可規律） |
| R11 クレート統合 | `pub mod shell` 追加・依存追加禁止・非衝突 | lib.rs 3 兄弟構造 | **既存で充足**（1 行追加のみ・依存追加不要） |

### 2.2 複雑度シグナル

- **アルゴリズム的ロジック中心**（外部統合なし・純粋関数・host 非依存）。CRUD でも workflow でもない。
- 難所は 3 点: (a) **ブロック＋ドット付きキーの構文層**（sakura のインライン走査とは別クラス）、(b) **`animationN` 集約**（`.interval` と複数 `.patternM` を 1 animation へ束ねる状態機械）、(c) **surface.append の範囲展開**（`a-b` を [a..=b] へ）。
- 単純な部分: element overlay 行・collision 行・alias 行はいずれも「1 行 = CSV split」で decode でき、opaque 保持なら加工不要。

## 3. 実装アプローチ選択肢（Options A/B/C）

### Option A: 既存 `kv`/`sakura` を最大流用（薄い shell 層）

`kv::parse_kv` を土台にし、ブロック境界だけ前処理で切り出して各ブロックを KV マップ化、その上に薄い意味写像を載せる。

- ✅ 新規コード最小・foundation 資産を直接活用。
- ❌ **致命的不適合**: `kv::parse_kv` は後勝ち（重複 alias キー R8.4 を潰す）・順序非保持（element/pattern 順序 R4.4 を失う）・`[id,...]` 配列値非対応・ドット付き複合キーの集約не対応。R8.4/R4.4/R5 の要件を満たせない。
- **評価**: **非推奨**。kv は「フラット KV」専用で surfaces.txt のネスト/重複/配列に構造的に合わない。descript ブロック内 KV の局所利用に留めるのが限界。

### Option B: `sakura` パターンを踏襲した独立 `shell` サブモジュール（新規 lexer+decode+model+parse）

`sakura` と同型の `model ← lexer ← decode ← parse` 四層を `src/shell/` に新設。lexer は surfaces.txt 用のブロック/行/CSV スキャナ、decode は `animationN` 集約と surface.append 範囲展開を担う意味層、model は下流共有型、parse は一行合成 facade。テストは in-source（fixture は literal 直書き＋代表抜粋）。

- ✅ 確立パターンと完全一貫（メンテナが即読める）。兄弟非衝突（R11.3）。各層独立テスト可。opaque/`#[non_exhaustive]`/`Result` 無し規律を自然に満たす。
- ✅ surfaces.txt 固有の構造（ブロック・ドットキー・範囲・重複）を decode 層で正しく表現できる。
- ❌ lexer 実体は新規（sakura の走査コードは流用不可・思想のみ流用）。
- **評価**: **推奨**。brief/structure.md steering が明示する接ぎ木方針そのもの。要件全項目を無理なく満たす。

### Option C: ハイブリッド（B の骨格 + descript ブロックのみ kv 流用）

Option B の四層を主軸としつつ、`descript { ... }` ブロック内の `version,N` のような素直な KV だけ `kv::parse_kv` へ委譲して重複実装を避ける。ブロック分割は shell 層、ブロック内 flat KV は kv、構造的部分（animation/alias/append）は shell decode。

- ✅ B の利点を保ちつつ、descript の些末な KV パースを既存資産で節約。foundation との連携を明示できる。
- ❌ descript は fixture 上 1 ブロック・2 フィールド（charset は別行）と極小のため、kv 委譲の節約効果は限定的。二重の入力経路（行 vs ブロック）で読み手の認知負荷が僅かに増す可能性。
- **評価**: **条件付き可**。descript の KV 委譲は設計フェーズの微小判断。効果が薄ければ B の純粋形で十分。

## 4. 要研究項目（Research Needed・設計フェーズ持ち越し）

1. **`surface.append` 範囲展開の内部表現**: `surface.append10,2100-2110,2200-2210` を「ターゲット ID 集合を parse 時に全展開して保持」するか「範囲記述子のまま保持し下流展開」か（要件 7.2 は「解決」と明記＝parse 時展開寄りだが、大量 ID 複製のコスト/表現を design で確定）。ukadoc SERIKO 仕様で範囲記法の端点包含（`a-b` は両端含む）を確認済とすべき。
2. **surface ブロック内 collision/animation と surface.append の統一表現**: 要件 7.3 は「通常 surface ブロックと同一のモデル表現」を要求。surface 定義と append 定義を同一 struct で表すか、append を「追記デルタ」として別型にするかの型設計。
3. **重複 alias キーの保持形**: 要件 8.4 は「出現をモデルに保持（衝突解決は下流委譲）」。`Vec<(Key, Vec<Id>)>`（順序保持・重複許容）か multimap か。kv の後勝ち BTreeMap は不可。
4. **`animationN` 集約の欠番 pattern index**: fixture に `pattern0` 欠落で `pattern1/2/3` のみの animation（L74-76・L432 `pattern3` のみ）あり。pattern index は連番前提でなく**疎（sparse）**。index を明示保持する表現（`Vec<(index, Pattern)>` 等）が必要。
5. **in-source テストの fixture 供給**: クレート跨ぎ（fixture は `crates/pilot/`・parser は `crates/areka-parsers/`）ゆえ `include_str!` は既存規律で不使用。emo2 の代表抜粋を literal 直書きするか、workspace 相対の別供給かを design で確定（既存 kv/charset validation_tests は literal 直書きで前例あり＝踏襲が素直）。要件 10.1 が「emo2 fixture を入力として」と書くため、抜粋の代表性を design で担保する。
6. **collision の surface 内スコープ vs グローバル**: fixture では collision は surface1000 ブロック内と surface.append の両方に現れる。所属 surface への結び付けを型でどう表すか（surface 定義に collision リストを持たせる形が素直）。

> いずれも**外部依存調査は不要**（新規外部 crate なし・`tracing` のみ）。ukadoc の SERIKO/2.0 仕様参照は範囲端点包含など数点の確認に留まり、正本は fixture＋`doc/emo2-conformance-scope.md`。

## 5. 実装複雑度・リスク

- **Effort: M（3–7 日）**
  - 根拠: パターン（sakura）が確立済で流用効くが、surfaces.txt 用の lexer/decode は新規。`animationN` 集約・surface.append 範囲展開という 2 つの非自明ロジックを含む。モデル型定義＋4 層＋in-source テストで中規模。単純 CRUD（S）ではなく、複数統合を要する L でもない。
- **Risk: Low〜Medium**
  - Low 要素: 既知技術（純粋 Rust パーサ）・確立パターン・明確スコープ・外部統合ゼロ・host 非依存・fixture という単一適合基準。
  - Medium 要素: surface.append 範囲展開と `animationN` 疎 pattern 集約は独自ロジックで設計判断を要する。alias 重複・collision スコープの型設計に選択肢がある。ただしいずれも fixture が正解を与えるため不確実性は限定的。

## 6. 設計フェーズへの申し送り

- **推奨アプローチ**: **Option B**（sakura パターン踏襲の独立 `shell` 四層）。descript KV の kv 委譲（Option C 差分）は効果が薄ければ不採用でよい微小判断。
- **主要決定事項**（design で確定すべき）:
  1. surface.append の範囲展開を parse 時に行うか（要件 7.2「解決」の解釈）。
  2. surface 定義と surface.append 追記を統一型で表すか別型か（要件 7.3）。
  3. 重複 alias キーの保持コンテナ（順序保持 `Vec` 系・kv BTreeMap 不可）。
  4. pattern index の疎保持表現（連番前提を置かない）。
  5. モデル最上位型の形（`ShellModel { descript, surfaces, aliases, ... }` のようなルート集約）。
  6. in-source テストの fixture 供給方式（literal 抜粋直書き＝既存規律踏襲を推奨）。
- **持ち越し研究**: §4 の 6 項目。外部依存調査は不要。ukadoc は範囲端点包含など数点の確認のみ。
- **過剰実装ガード**（要件 10.2/10.3）: emo2 未使用の method/interval/collisionex/非 0 offset は**実装しない**。拡張余地は `#[non_exhaustive]` シームのみ。2 例目の実物 fixture が要求するまで抽象を足さない。
