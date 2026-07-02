# ギャップ分析: areka-P0-balloon-parse

> 種別: gap-analysis（kiro-validate-gap 成果）。要件 (`requirements.md`) と既存コードベースの差分を分析し、設計フェーズの実装戦略判断に供する情報を提供する。**判断ではなく選択肢**を提示する。
> 入力: `spec.json`（language=ja）／`requirements.md`（確定済・R1〜R5）／`brief.md`／steering・memory・ukadoc（`descript_balloon`）／`doc/emo2-conformance-scope.md` §4。
> 正典: ukadoc（`descript_balloon`）＞ `doc/emo2-conformance-scope.md` §4 ＞ emo2-kakukaku fixture（最小適合サンプルであって書式の聖典ではない）。

---

## 1. 現状調査（接ぎ木先の解剖）

### 1.1 クレート構成と接ぎ木点

`areka-parsers`（`crates/areka-parsers/`）は「純粋・std 中心・host 非依存」なパーサーファミリ。`lib.rs` は兄弟モジュールを列挙するだけの薄い集約層で、次を公開する:

```
pub mod charset;  // areka-P0-parser-foundation（完了）
pub mod kv;       // areka-P0-parser-foundation（完了）
pub mod sakura;   // areka-P0-sakura-parse（完了・規律テンプレート）
```

`lib.rs` の doc コメントに **「兄弟モジュール（shell / balloon / package 等）は各 spec が追加する」** と明記されており、`balloon` モジュール追加は設計として予定済みの拡張点。本 spec は `pub mod balloon;` を 1 行追加し `balloon/` サブツリーを新設する接ぎ木となる（既存モジュールへの侵襲なし）。

依存: `encoding_rs`（foundation 経由）＋ `tracing` のみ（`Cargo.toml`）。**新規外部依存は不要**（本 spec は純粋なマップ→型写像＋整数パースのみで std だけで足りる。`tracing` は寛容無視トークンの観測ログに任意で使える）。

### 1.2 確立済みパーサー規律（`sakura` / foundation から抽出）

| 規律 | 具体例（既存コード） | 本 spec での適用 |
|---|---|---|
| **公開 facade は `parse(...) -> Model`（`Result` 無し・寛容）** | `sakura::parse(input: &str) -> Vec<Instruction>`（`parse.rs`）／`kv::parse_kv(text: &str) -> BTreeMap<String,String>`（`parse.rs`） | R1.1/R1.2: `Result` 無しで常に `BalloonModel` を返す |
| **モデル型 = 別モジュール `model.rs`・クロスエンジン I/O 契約の片側（本クレートが正本所有）** | `sakura/model.rs`（`Instruction` enum ＋ 値型）／`charset/model.rs`（`DefaultEncoding`） | バルーンモデル型を `balloon/model.rs` に定義 |
| **`#[non_exhaustive]` で将来拡張を後方互換に開く** | `Instruction`・`DefaultEncoding` が両方 `#[non_exhaustive]` | R2.8: モデル型（enum/struct）に `#[non_exhaustive]` シームを残す |
| **NewType＋不透明 inner＋read-only accessor（dola `ActorKey` 流儀）** | `SurfaceArg(String)` → `new`/`as_str`／`NewLineRatio(f32)` → `new`/`ratio` | R2.8: 各幾何・フォント値へ read-only アクセサを提供 |
| **最小派生のみ（値に応じ `Clone`/`Debug`/`PartialEq`、必要なら `Copy`/`Eq`）** | `DefaultEncoding` は `Copy,Eq` 付き／`Instruction` は `f32` 含むため `Eq` 無し | 座標は整数のみゆえ `Copy,Eq` 可能（3.3 参照） |
| **in-source `#[cfg(test)]` テスト（別モジュール・公開パス経由で検証）** | `sakura/model_tests.rs` は `use crate::sakura::{...}` で公開面のみを叩き I/O 契約を固定 | R5.4: fixture ベースの `*_tests.rs` を同居 |
| **mod.rs は `mod` 宣言＋`pub use` 集約のみ** | `sakura/mod.rs`・`kv/mod.rs` | `balloon/mod.rs` で公開面を集約 |

### 1.3 上流 foundation の出力仕様（消費対象）

- `charset::decode(bytes, DefaultEncoding) -> String`（バイト→デコード済み文字列）。
- `kv::parse_kv(&str) -> BTreeMap<String,String>`: **最初のカンマ 1 個で `key,value` 分割・trim・後勝ち・空行/カンマ無し行スキップ・値は無加工文字列・順序非保持・panic/`Result` 無し**（`kv/parse.rs`）。
  - 含意 A: **値は必ず文字列**。`origin.x,0` は `"0"`、`wordwrappoint.x,-34` は `"-34"` として得られる。本 spec が整数パース・符号解釈を担う。
  - 含意 B: **後勝ちマージは 1 ファイル内のみ**。ファイル間（画像別＞descript）の 2 層マージは foundation 非所有 → **本 spec が所有**（R3）。
  - 含意 C: `BTreeMap` はキー階層を持たない **フラット**マップ。`font.color.r` は 1 個の文字列キー。本 spec が `font.color.{r,g,b}` の 3 キーを RGB へ束ねる。

### 1.4 emo2-kakukaku fixture 実測（採取元）

3 ファイルとも UTF-8（charset 行は descript のみ `charset,UTF-8`・画像別層は charset 行なし＝foundation 既定で通る）。要件 R5 の期待値と fixture は一致（照合済）:

| キー | descript.txt | balloons0s.txt | balloonk0s.txt |
|---|---|---|---|
| `windowposition.x/y` | （なし） | `266` / `-129` | `-190` / `-75` |
| `origin.x/y` | `0` / `0` | （なし） | （なし） |
| `wordwrappoint.x` | `-34` | `-49` | （なし・descript 継承 -34） |
| `wordwrappoint.y` | `0` | （なし） | （なし） |
| `validrect.top/bottom/left/right` | `0/0/0/0` | `46/-56/36/-44` | `40/-70/24/-48` |
| `font.name` | `Yu Gothic UI` | （なし） | （なし） |
| `font.height` | `28` | （なし） | （なし） |
| `font.color.r/g/b` | `0/0/0` | （なし） | （なし） |

- 画像別層に **存在するがモデル化しないキー**: `arrow0/1.x/y`・`number.xr/y`・`onlinemarker.*`・`sstpmarker/message.*`（R2.7 で明示除外）。foundation マップに残置され本層は消費しない（寛容 passthrough）。
- **観測すべき挙動**: R5.2/R5.3 のマージ期待値は「画像別に無いキーは descript を継承」を要求する（例: `balloonk0s.txt` に `wordwrappoint` 無し → descript の `-34` を採る／`origin`/`font` は両画像別層に無く descript のみ由来）。

### 1.5 ukadoc（正典）による符号意味の裏付け

- `windowposition.x,座標`: 「数値指定の場合シェル側が+、シェルから離れる側が-」→ R4.2 と一致。
- `windowposition.y,座標`: 「ピクセル単位で数値で指定する。**下が+で、上が-なので注意**」→ R4.3 と一致。
- `validrect.{top,bottom,left,right}` は `座標 *1` 注記（＝負値は反対辺基準の SSP 慣行）／`wordwrappoint.x,-34`＝右端基準（`emo2-conformance-scope.md` §4 落とし穴）→ R4.1 と一致。
- **結論**: 要件の符号解釈は ukadoc・conformance-scope・fixture の三者と整合。設計での符号意味の再定義は不要（保持のみ・R4.4 のとおりピクセル解決は消費側）。

---

## 2. 要件実現可能性分析（Requirement → 資産マップ）

| 要件 | 技術的必要物 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| R1 入力受理・寛容パース | `parse(...) -> Model`（`Result` 無し・未知/非数値を無視継続） | `sakura`/`kv` に寛容 facade 前例あり | **Constraint**（規律踏襲）＋一部 **Missing**（balloon 固有の非数値→未指定処理） |
| R2 モデル定義（幾何＋フォント subset） | `windowposition`/`origin`/`wordwrappoint`/`validrect`/`font.{name,height,color}` の型・「未指定」表現・read-only accessor・`#[non_exhaustive]` | モデル型なし（brief「出力モデル型は未存在」）／NewType＋accessor 前例あり | **Missing**（型は本 spec で新規定義） |
| R3 3段優先度・ファイル間マージ | 画像別層＞descript 層の 2 層 KV マージ＋未指定表現 | `kv` は 1 ファイル内後勝ちのみ（ファイル間非所有） | **Missing**（2 層マージは本 spec 所有） |
| R4 座標符号解釈 | 負値=反対辺オフセットを符号付き整数で保持（ピクセル解決せず） | foundation が明示的に非所有／`NewLineRatio` に符号付き保持前例 | **Missing**（符号解釈は本 spec 所有） |
| R5 emo2-kakukaku 適合 | fixture ベース in-source 単体テスト（host 不要・純粋関数） | `sakura` に fixture テスト前例／emo2 fixture 存在（foundation validation でも採取済） | **Constraint**（規律踏襲）＋テスト新規作成 |

**複雑度シグナル**: アルゴリズム的ロジックは軽量（フラットマップからのキー写像＋整数パース＋2 層優先度選択）。外部統合なし・I/O なし・並行なし。難所は「型形状の設計判断」に集中（下記 §6）。

---

## 3. 実装アプローチの選択肢

### Option A: 既存 `areka-parsers` へ `balloon` モジュールを追加（brief 既定・推奨候補）

- **拡張対象**: `lib.rs` に `pub mod balloon;` 1 行追加。`balloon/{mod.rs, model.rs, parse.rs, *_tests.rs}` を新設。`Cargo.toml` は変更不要（新規依存なし）。
- **互換性**: 既存モジュール（`charset`/`kv`/`sakura`）へ非侵襲。`lib.rs` の追加行のみが `shell-parse`/`package-mount` と共有シーム（brief「lib.rs/Cargo.toml の共有シームはマージ順に留意」）。並走安全（同クレート別モジュール・非衝突）。
- **トレードオフ**: ✅ 既存規律・インフラ（foundation の `kv`/`charset`）を最大限再利用・新規ファイル最小・並走安全。❌ `lib.rs` の共有行が並列 spec とマージ競合し得る（軽微・順序留意で回避可）。

### Option B: 独立クレート化（非推奨）

- **合理性**: バルーンモデルが将来 baseware 全体で共有される I/O 契約になる場合、独立クレートで境界を明示できる。
- **トレードオフ**: ✅ 責務分離が明快。❌ foundation の `charset`/`kv` を pub 依存として引き回す必要・クレート増でナビゲーション負荷・brief/roadmap の「同クレート別モジュール」方針および `sakura` 前例（同一クレート同居）と不整合。**M1 スコープでは過剰**。

### Option C: ハイブリッド（`balloon` モジュール内で層分割・部分的に推奨）

- **合理性**: `balloon/` 内部を `sakura` 同様に責務分割: `model.rs`（型）／`map.rs` or `decode.rs`（KV マップ→モデル写像＋符号解釈）／`merge.rs`（2 層優先度解決）／`parse.rs`（公開 facade）。単一責務・テスト容易。
- **トレードオフ**: ✅ `sakura` の `model←lexer←decode←parse` 分割規律と一貫・各層を独立テスト可。❌ 本 spec のロジックは `sakura` より軽量ゆえ、過剰分割は YAGNI リスク（`kv` は単一責務ゆえ `parse` 1 本で済ませた前例あり）。**分割粒度は設計判断**（§6-7）。

**総合**: brief は Option A を既定とし `sakura`/foundation パターン踏襲を指示。内部構造は Option C の軽量版（`model` ＋ facade、マージ/写像は 1〜2 ファイル）が妥当な出発点。分割の細かさは設計フェーズで確定。

---

## 4. Research Needed（設計フェーズへ持ち越す不確実性）

1. **`wordwrappoint.y` の扱い**: fixture では descript に `wordwrappoint.y,0` があるが画像別層には無い。要件 R2.3 は「x, および存在すれば y」と表現＝y は optional。モデルで `wordwrappoint` を `{x, y: Option}` とするか、`y` も x と同格に持つかは設計判断（fixture の y は常に 0）。**過剰実装禁止規律との兼ね合いで要検討**。
2. **「未指定」の型表現**: R2.6/R3.4 は「組込み既定値で埋めない・未指定として表現」。`Option<T>` が自然だが、NewType 不透明化（`sakura` の NewType 流儀）と `Option` の組合せ方（`Option<Windowposition>` か、各成分 `Option` か）は型形状判断。
3. **座標の内部整数型**: 符号付きゆえ `i32` が素直（emo2 実測値は数百 px 範囲）。ukadoc の `center`/`top`/`bottom` キーワード指定（windowposition.x）は emo2 未使用ゆえ M1 非対応で良いが、非数値トークン到来時の「未指定扱い」（R1.4）と衝突しないことの確認。
4. **RGB 束ね方**: `font.color.{r,g,b}` の 3 キーを 1 色型へ束ねる際、3 個中一部欠落時の扱い（全て揃って初めて色、または部分未指定）。fixture は 3 個揃っているため、欠落時挙動は R1.4/R2.6 準拠（欠落成分は未指定）で足りるが型形状に影響。
5. **入力インターフェースの形**: R1.1 は「デコード済み文字列 **または** foundation の KV マップ」を受理と述べる。`parse(&str)` と `parse_from_kv(&BTreeMap)` の 2 入口を出すか、`&str` 入口が内部で `kv::parse_kv` を呼ぶ単一入口かは API 設計判断（マージは KV マップ 2 個を受ける形が自然）。

これらは **深掘りせず設計フェーズで確定**（gap-analysis は選択肢提示に留める）。

---

## 5. 工数・リスク

- **工数**: **S（1〜3 日）**。既存 `areka-parsers` 規律・foundation 出力を再利用し、ロジックはフラットマップ→型写像＋整数パース＋2 層優先度選択のみ。外部統合・I/O・並行なし。fixture・期待値・正典（ukadoc）が既に揃い、参照テンプレート（`sakura`）も完成済。
- **リスク**: **Low**。確立済みパターンの踏襲・馴染みのある技術（std のみ）・スコープ明確（emo2 幾何＋フォント subset のみ・過剰実装禁止）・統合最小（`lib.rs` 1 行）。唯一の不確実性は型形状の設計判断（§4）で、いずれも要件・fixture・ukadoc により境界が確定しており技術的未知はない。

---

## 6. 設計判断項目（設計フェーズ／要件ディスカッションへ供給）

> 以下は **判断ではなく論点**。要件は確定済ゆえ本項は要件を変更せず、design/discussion での確定を促す。

1. **【依存 D1】クレート edition の記載差異（brief vs 実体）**: brief.md は「Rust 2024」と記す。`crates/areka-parsers/Cargo.toml` は `edition.workspace = true`、ルート `Cargo.toml` の `[workspace.package]` は `edition = "2024"` → **実体は 2024 で brief と整合**。指示で言及された「crate が edition=2021 を報告」という状態は**現行 Cargo.toml では観測されず**（既に 2024）。設計では「2024 前提で確定・edition 記述の齟齬なし」を明記するか、指示の前提と実体の差を注記する（**本 gap-analysis では解決せず・設計判断として surface**）。
2. **【D2】バルーンモデルの型形状**: (a) 単一 struct `BalloonModel` に全フィールドを持たせるか、(b) `windowposition`/`origin`/`wordwrappoint`/`validrect`/`font` を各 NewType/sub-struct に分けるか。`sakura` は enum＋値型、本件は「1 モデル値」ゆえ struct 集約が自然。`#[non_exhaustive]`（R2.8）を struct に付す場合、別クレートからの構築が制限される点（テストは同クレートゆえ問題なし・下流消費は accessor のみ）を確認。
3. **【D3】「未指定」の表現方式**: `Option<T>` 直持ちか、NewType でラップした上での `Option` か。R2.6「既定値で埋めない」を満たす最小形は各モデル化キーを `Option` にすること。座標成分（x/y や t/b/l/r）を個別 `Option` にするか、幾何単位でまとめて `Option` にするか（fixture では validrect は 4 成分同時に現れる）。
4. **【D4】2 層マージの実装位置と単位**: マージを「KV マップ 2 個（画像別・descript）を先に後勝ち合成してから 1 回写像」するか、「両者を各々モデル化してからモデル同士を優先度合成」するか。前者は foundation の後勝ちマップ流儀と一貫（画像別を descript に `insert` で上書き）・実装最小。R3 の期待挙動はどちらでも満たせるが、前者を推奨候補として提示。
5. **【D5】入力インターフェース（§4-5）**: `&str`×N と `&BTreeMap`×N のどちらを主入口にするか、マージ用に「画像別層・descript 層」の 2 引数（各 optional）を受ける facade 形状。R3.5（descript のみ）を満たすため画像別層は optional。
6. **【D6】内部数値型と非数値耐性**: 座標=`i32`・font.height=`u32`（非負）・color 成分=`u8`（0–255）を素直な候補とし、パース失敗時は R1.4 に従い「未指定」へ落とす（`Result` を漏らさない）。`u8`/`u32` 範囲外値（emo2 では出ない）到来時も未指定へ落とすかは設計で確定。
7. **【D7】`wordwrappoint.y` の要否（§4-1）**: fixture の y は常に 0・画像別層に無い。過剰実装禁止規律（R5.5）と R2.3「存在すれば y」の両立として、`y: Option` で保持し emo2 では常に `Some(0)`/未指定になる形を推奨候補として提示。
8. **【D8】内部モジュール分割粒度（Option C）**: `balloon/` を `model` ＋ facade の 2 ファイルに留めるか、写像・マージを別ファイルに割るか。`kv`（単一責務→1 本）と `sakura`（多層→分割）の中間規模ゆえ、`model.rs` ＋ `parse.rs`（写像・マージ内包）＋ `*_tests.rs` を出発点に提示。
9. **【D9】`tracing` 使用可否**: 寛容無視した未知キー/非数値トークンを `tracing` で観測ログするか（依存は許可済・純粋関数性は保つ）。任意・観測目的のみ。

---

## 7. 設計フェーズへの推奨

- **優先アプローチ**: Option A（`areka-parsers` に `balloon` モジュール追加）＋内部は Option C の軽量版（`model.rs` ＋ facade、マージ/写像は最小分割）。`sakura`/foundation 規律を全面踏襲。
- **主要判断**: 型形状（D2/D3/D7）とマージ位置（D4）・入力インターフェース（D5）を design で確定。edition 差異（D1）は明示的に注記。
- **持ち越す Research 項目**: §4 の 1〜5（`wordwrappoint.y`・未指定表現・整数型・RGB 束ね・入力形）。いずれも要件・fixture・ukadoc で境界確定済＝技術的未知なし・型形状の設計選択。
- **規律の再確認**: `Result` 無し寛容パス・NewType＋opaque＋accessor・`#[non_exhaustive]`・in-source テスト・過剰実装禁止（emo2 subset のみ・2 例目の実物まで抽象を足さない）。
