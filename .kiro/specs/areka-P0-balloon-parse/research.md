# ギャップ分析（Gap Analysis）— areka-P0-balloon-parse

> 生成: kiro-validate-gap / 2026-07-01
> 入力: requirements.md（確定）・brief.md・spec.json（language=ja）・steering（product/tech/structure/roadmap）・既存 `crates/areka-parsers/src/sakura/*`・emo2 fixture。
> 位置づけ: **情報提供であって決定ではない**。設計判断は requirements discussion / design phase へ持ち越す。

---

## 1. 現状調査（Current State Investigation）

### 1.1 接ぎ木先クレート `areka-parsers`

- **クレート構成**（`crates/areka-parsers/`）
  - `Cargo.toml`: 依存は `tracing`（workspace）のみ。`publish = false`・`edition.workspace`（Rust 2024）。**追加依存ゼロで balloon を実装できる前提が既に整っている**。
  - `src/lib.rs`: `pub mod sakura;` のみ。doc コメントに「兄弟モジュール（shell / balloon / package 等）は各 spec が追加する」と**明記済み**。balloon モジュールの追加口はここ 1 行の `pub mod balloon;`。

- **確立パターン（`sakura` モジュール）— balloon が踏襲すべき正本**
  - **多層ファイル分割**: `mod.rs`（公開面集約）/ `model.rs`（型）/ `lexer.rs`（構文）/ `decode.rs`（意味）/ `parse.rs`（公開 facade）＋各層 in-source テスト（`*_tests.rs`）＋横断 `validation_tests.rs`。依存方向 `model ← lexer ← decode ← parse`。
  - **公開面集約規約**: `mod.rs` が `mod model;` 等を private 宣言し、`pub use model::{...}; pub use parse::parse;` で最小の公開面のみを外部へ出す。テストは `#[cfg(test)] mod xxx_tests;` として同 `mod.rs` に並べる。
  - **モデル型の規律**（`model.rs`）:
    - `#[non_exhaustive]` enum（variant 追加を後方互換に）。
    - 派生は **最小限**（sakura は `Clone, Debug, PartialEq` のみ。`f32`/`Duration` を含むため `Eq`/`Hash`/`serde` は付さない）。
    - **不透明 NewType ＋ read-only アクセサ**（`SurfaceArg(String)` → `new()` / `as_str()`、`NewLineRatio(f32)` → `new()` / `ratio()`）。フィールドは非公開、別クレート下流は公開アクセサ経由でのみ読む。
    - コメントに要件番号を紐づける文化（`// 要件 2.2/2.3`）。
  - **寛容パース規律**（`parse.rs` / `decode.rs`）:
    - 公開 `parse` は `pub fn parse(input: &str) -> Vec<Instruction>` = **`Result` を返さない**。空入力→空 `Vec`、失敗しない・panic しない・エラー送出なし。
    - 未対応・不正トークンは情報を失わず `Raw(String)` へ吸収し、**後続解析を継続**（局所吸収・全域継続）。
    - `tracing` のみ（エラー型なし）。純粋・決定的・host 非依存・I/O なし。
  - **テスト文化**（`*_tests.rs`）: `#![cfg(test)]`・**公開パス経由**でのモデル構築/比較（`use crate::sakura::{...}`）＝別クレート下流視点で I/O 契約を固定。空入力・順序保持・境界値・純粋性を個別 `#[test]` で確認。

### 1.2 emo2 fixture（実データ・検証対象）

配置: `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`

- **`descript.txt`（84 行・base 共通既定）**: `charset,UTF-8` / `type,balloon` / `use_self_alpha,1` / `origin.x,0` `origin.y,0` / `wordwrappoint.x,-34` `wordwrappoint.y,0` / `validrect.{top,bottom,left,right},0` / `font.name,Yu Gothic UI` / `font.height,28` / `font.color.{r,g,b}` / `anchor.font.color.{r,g,b}` / `arrow0.{x,y}` `arrow1.{x,y}`。**未使用フィールド**（M1 対象外）も同居: `anchor.pen.color.*` / `cursor.*` / `number.*` / `onlinemarker.*` / `sstpmarker.*` / `sstpmessage.*` / `communicatebox.*`。
- **`balloons0s.txt`（27 行・sakura 側差分）**: `windowposition.x,266` `windowposition.y,-129`（**base に無い**）/ `wordwrappoint.x,-49`（**base を上書き**）/ `validrect.top,46` `validrect.bottom,-56` `validrect.left,36` `validrect.right,-44`（base を上書き・**負値含む**）/ `arrow0.x,15` `arrow1.x,15` `arrow0.y,90` `arrow1.y,-110`。＋未使用（`number.*` `onlinemarker.*` `sstpmarker.*` `sstpmessage.*`）。
- **`balloonk0s.txt`（19 行・kero 側差分）**: `windowposition.x,-190` `windowposition.y,-75` / `validrect.top,40` `validrect.bottom,-70` `validrect.left,24` `validrect.right,-48` / `number.xr,-58` `number.y,-56` / `arrow0.x,9` `arrow1.x,9` `arrow0.y,54` `arrow1.y,-125`。
- 画像: `balloons0.png`（400×224）/ `balloonk0.png`（288×203）＋ `arrow0.png` `arrow1.png` 他。

> **⚠ brief との食い違い（重要）**: brief.md と requirements の Boundary Context は「`balloonk0s.txt` は未 vendored / 実データ検証の主対象は s0s、k0s は構造対応のみ」を前提とするが、**実際には `balloonk0s.txt`（19 行の実データ）と `balloonk0.png` が fixture に存在する**。k0s も実データでマージ検証が可能。要件 6.3 は「k0s の差分が構造的に解析・マージされたモデルを生成」しか要求しないため要件矛盾ではないが、**k0s も実データ pass を検証対象に含めるか**は設計判断項目（後述 §5-4）。

### 1.3 参照した正本ドキュメント

- `doc/emo2-conformance-scope.md §4`（バルーン M1 実需の正本）: 必須フィールドと「負値=反対端基準」「s0s/k0s は descript ベース差分上書き＝マージ実装必須」を確定。sakura/kero の**左右配置は shell descript の `*.balloon.alignment` が決める**＝balloon 単体では決まらない cross-cutting seam（本 spec は所有しない）。
- steering `tech.md`: Rust 2024・`tracing` 全体規約。ただし全体の一般規約は `thiserror` を掲げるが、**areka-parsers は寛容パースゆえエラー型を持たない**（sakura が既にこの逸脱を確立済み。balloon も同流儀で問題なし）。

---

## 2. 要件→資産マップ（Requirement-to-Asset Map）

| 要件 | 技術的必要物 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| R1 公開面・純粋関数入口 | `balloon` モジュール＋公開 facade＋モデル型 | sakura の `mod.rs`/`parse.rs` パターンを複製 | **Missing**（新規モジュール）・パターンは既存 |
| R1.2 base＋s0s/k0s 入力で単一マージ済みモデル | facade 関数シグネチャ設計 | sakura は単一 `&str → Vec`。balloon は**複数入力→単一モデル**で形が異なる | **Missing**（新 API 形状）・§5-1 |
| R1.3/1.4 `Result` 無し・空/未知でも既定モデル | 寛容パース＋既定値付きモデル | sakura の寛容規律を踏襲 | Constraint（規律流用） |
| R1.5 不透明 NewType＋non_exhaustive＋最小派生 | モデル型設計 | `SurfaceArg`/`NewLineRatio` の NewType 流儀 | Constraint（規律流用）・§5-2 |
| R2 base descript フィールド解析（type/use_self_alpha/origin/font/font.color/anchor.font.color/画像/arrow） | `key,value` 行の kv パーサ | sakura は `\tag[args]` 構文で**別物**。kv 行パーサは新規 | **Missing**（新構文層）・§5-3 |
| R2.8 画像参照をサーフェス別に解決 | サーフェス別画像参照（`balloons0.png`/`balloonk0.png`） | descript に明示 png 参照行が無い（命名規約 `balloon{s,k}0.png` から導出）※ | **Unknown**・§5-5 |
| R3 座標の負値=反対端基準の符号保持 | 符号付き整数保持＋「負値の意味」を失わないモデル | sakura に座標系の前例なし（`Cursor{x,y}` は文字列保持のみ） | **Missing**＋Constraint・§5-6（最大の落とし穴） |
| R4 base→s0s/k0s overlay マージ | フィールド単位 overlay マージ器＋サーフェス区別 | sakura に**マージ概念なし**（完全新規） | **Missing**（新ロジック）・§5-1/§5-4 |
| R5 未使用フィールドの寛容取り扱い | 未知行を破綻させず取り込み（生保持等） | sakura の `Raw` 吸収パターン | Constraint（規律流用）・§5-7 |
| R6 emo2 fixture 単体テスト | fixture 文字列を入力にした in-source テスト | sakura の `*_tests.rs` 文化。ただし fixture の**文字列取り込み方法**は要決定 | **Missing**＋Unknown・§5-8 |

※ descript / s0s / k0s のいずれにも `balloons0.png` を指す明示行は**無い**。SSP 慣行では `balloon{s|k}{ID}.png` が命名規約で導出される。要件 2.8 の「参照しうる情報」は命名規約由来の導出を含意する可能性が高い（§5-5 で扱う）。

---

## 3. 複雑度シグナル

- **主体はアルゴリズム的ロジック**（kv 行パース＋符号保持＋overlay マージ）。外部統合・I/O・非同期は無し（純粋関数・単体テスト完結）。
- sakura の**構文が根本的に異なる**（さくらスクリプトのタグ列 vs. balloon の `key,value` 行）ため、lexer/decode をそのまま流用はできない。**分割思想・寛容規律・モデル規律・テスト文化は流用可、構文実装は新規**。
- マージという新概念が入る（sakura に前例なし）。ただし overlay = 「後勝ちで同キー上書き」の単純規則。

---

## 4. 実装アプローチ選択肢

### Option A: `sakura` パターンをそのまま多層複製（model/lexer/decode/parse/merge の分割）

- **内容**: balloon も `model.rs`/`lexer.rs`（kv 行スキャナ）/`decode.rs`（行→フィールド意味割当）/`merge.rs`（overlay）/`parse.rs`（facade）に厳密分割し、各層 in-source テスト。
- **トレードオフ**:
  - ✅ sakura と対称でナビゲーション容易・レビュアが規約整合を確認しやすい。
  - ✅ 各層独立テストで境界値（負値・空・未知行）を細かく固定できる。
  - ❌ balloon の構文は sakura より遥かに単純（`line.split_once(',')` レベル）。5 ファイルは**過剰分割の懸念**。lexer 層はほぼ trivial になりうる。

### Option B: 単一 `balloon` モジュール内で機能単位の小分割（model ＋ parse ＋ merge の 3 層程度）

- **内容**: 構文が単純なので lexer/decode を統合し、`model.rs`（型）/`parse.rs`（descript 文字列→中間フィールド集合）/`merge.rs`（base＋overlay→マージ済みモデル）/`mod.rs`（facade `pub fn ...`）＋ in-source テスト。
- **トレードオフ**:
  - ✅ 構文の実態（kv 1 行 = 1 フィールド）に見合った粒度。sakura の分割**思想**（依存方向・公開面集約・寛容規律・NewType）は維持。
  - ✅ マージという balloon 固有関心を独立ファイルで明示。
  - ❌ sakura と 1:1 のファイル対応にはならない（レビュー時に「なぜ lexer が無いか」の説明が要る＝doc コメントで吸収可能）。

### Option C: ハイブリッド（モデル＋facade は最小、内部は必要に応じて段階分割）

- **内容**: まず `model.rs`＋`mod.rs`（facade）で最小の型と入口を確立し、パース/マージは内部関数として置き、テストが求める粒度に応じて `parse.rs`/`merge.rs` へ切り出す。
- **トレードオフ**:
  - ✅ 過剰実装禁止の規律に最も忠実（構造を最初から作り込まない）。
  - ✅ 段階的にファイルを増やせる。
  - ❌ 「構造は最初から」を良しとする既存 sakura 文化（`mod.rs` に将来モジュールを予告するスタイル）とはやや逆。設計フェーズで分割方針を先に決めた方が一貫する。

> **設計フェーズへの示唆（決定ではない）**: balloon の構文が単純なため、sakura の分割**思想**（依存方向・公開面集約・寛容パース・不透明 NewType・in-source テスト）は必須踏襲だが、**ファイル数は sakura と機械的に揃える必要はない**。Option B（model / parse / merge / facade）が構文実態と規律の折衷として有力候補。最終判断は design phase。

---

## 5. 設計判断項目（requirements discussion / design へ持ち越す論点）

1. **facade の API 形状（マージ入口の設計）** — R1.2/R4。sakura は `parse(&str) -> Vec<Instruction>`。balloon は「base descript ＋ サーフェス別差分 → 単一マージ済みモデル」。候補: (a) `parse(descript, s0s, k0s) -> BalloonModel`（両サーフェスを 1 モデルに内包）/ (b) `parse_side(descript, overlay) -> BalloonSideModel` をサーフェスごとに 2 回呼ぶ / (c) `parse(descript) -> Base` ＋ `merge(base, overlay) -> Sided` の 2 段公開。R4.5「sakura/kero を区別して取り違えない」を型で表現する方法（サーフェス別 2 フィールド or サーフェス enum キー）と併せて決める。

2. **座標・色・フォントの値型設計（NewType 方針）** — R1.5/R3。sakura は `SurfaceArg`/`NewLineRatio` を不透明 NewType にした。balloon の座標（符号付き `i32`）・色（RGB）・font 高・origin をどこまで NewType 化するか。特に**座標の「負値=反対端基準」を型レベルで表現するか**（例: `Coord(i32)` に `is_from_far_edge()` アクセサ）、あるいは生 `i32` を保持し解釈は下流に委ねる（R3.4 は「下流が判定できる形で提供」＝符号保持で足りる）か。過剰実装禁止との兼ね合い。

3. **構文層の要否（lexer 分割の判断）** — §4。balloon の kv 行は `split_once(',')` で足りるため、sakura のような独立 lexer が必要か、decode/parse へ統合するか。Option A/B/C の選択に直結。

4. **k0s の検証深度** — Boundary Context / R6.3 と fixture 実態の食い違い（§1.2 ⚠）。brief は「k0s 未 vendored・構造対応のみ」を前提とするが、**実際は `balloonk0s.txt`（実データ）が存在**。k0s も実データ pass を検証対象に格上げするか、要件通り「構造対応のみ（解析・マージ経路が働く）」に留めるか。要件は後者しか要求しないが、実データがあるなら安価に前者も可能。

5. **バルーン本体画像参照の解決方式** — R2.8。**【Research 確定済み・2026-07-01 requirements discussion / ukadoc 正本】** ukadoc（`descript_balloon.html`）で確認: descript/s0s/k0s にバルーン本体画像ファイル名の**明示行は無く**、命名規約 `balloon{s|k}{ID}.png`（偶数=左向き／奇数=右向き）から導出される（ID はサーフェス設定ファイル名が担う）。→ **what は確定**: モデルはサーフェス種別（sakura／kero）＋サーフェス ID を保持し、下流がファイル I/O 無しに命名規約で解決する（R2.8 を本方針へ改訂済み）。**残る design 判断（how のみ）**: 導出ファイル名の型表現（(a) 命名規約から導出したファイル名文字列を NewType 保持 / (b) サーフェス種別＋ID の構造のみ保持し導出は下流）。※本 spec は host/I/O 非依存＝実ファイル存在チェックは領分外。

6. **未知行の保持粒度** — R5.2。sakura は `Raw(String)`（1 命令 1 生文字列）。balloon で未知 kv 行をどう保持するか（(a) `Vec<String>` の生行リスト / (b) `Vec<(String, String)>` の kv ペア / (c) 破棄せず保持する義務は R5.2「生保持等」で緩い）。未使用フィールド（cursor/number/communicatebox 等）は「意味解釈しない」だけで保持義務は R5.1 では課されていない点に注意。

7. **同一キー重複・行フォーマットの寛容度** — emo2 fixture では 1 キー 1 行だが、同一ファイル内で同キーが複数回出た場合（後勝ち？）、空値・余分な空白・CRLF/LF・BOM の扱いを decode でどう寛容化するか。sakura の「局所吸収・全域継続」に倣う。

8. **fixture のテスト取り込み方法** — R6。fixture は `crates/pilot/examples/.../emo2-kakukaku/` にあり、areka-parsers クレートからは**別クレートの相対パス**。候補: (a) `include_str!` で相対パス取り込み（クレート境界を跨ぐ相対パスの脆さ）/ (b) 検証に必要な最小の fixture 抜粋をテスト内リテラルとして持つ / (c) fixture を areka-parsers 側の `tests/fixtures/` へコピー/シンボリック。純粋関数・単体テストのみで観測可能（R6.4）を満たす取り込み方式を決める。

---

## 6. 工数・リスク（Effort / Risk）

- **Effort: S〜M（1〜5 日）**。既存 sakura パターンが強力なテンプレート＝分割思想・寛容規律・テスト文化・クレート雛形が揃い、追加依存ゼロ。新規性は「kv 行パース」「overlay マージ」「符号保持座標」の 3 点のみで、いずれもアルゴリズム的に単純。fixture も小さい（84/27/19 行）。
- **Risk: Low〜Medium**。
  - Low 要因: host/I/O 非依存・純粋関数・単体テスト完結・実データ fixture 有り・確立パターンの流用。
  - Medium 要因: **座標の「負値=反対端基準」の取り違えが最大の落とし穴**（emo2-conformance-scope §4・brief Constraints でも警告）。符号を情報として失わないモデル設計と、fixture の負値（`validrect.bottom,-56`・`wordwrappoint.x,-49`・`windowposition.y,-129`）を明示的に固定するテストが必須。マージ方向（base←overlay の後勝ち）とサーフェス（s0s/k0s）の取り違え防止も要注意。

---

## 7. 設計フェーズへの推奨

- **推奨アプローチ（暫定）**: Option B（model / parse / merge / facade ＋ in-source テスト）。sakura の分割思想・寛容規律・不透明 NewType・公開面集約・テスト文化を必須踏襲しつつ、単純な kv 構文に見合った粒度とし、balloon 固有の「マージ」を独立ファイルで明示。最終決定は design phase。
- **必須踏襲（非交渉）**: `Result` 無しの寛容パース／`#[non_exhaustive]`＋最小派生／不透明 NewType＋read-only アクセサ／`tracing` のみ・エラー型なし／in-source テスト・公開パス経由の契約固定／過剰・予測実装の禁止（emo2 使用フィールドのみ）。
- **持ち越す Research 項目**:
  - §5-5 バルーン本体画像参照の解決（descript に明示行が無い＝SSP 命名規約 `balloon{s|k}{ID}.png` が正本かの確認）。
  - §5-8 クレート境界を跨ぐ fixture のテスト取り込み方式。
- **要件との食い違い（design で解消要）**: brief/requirements は「k0s 未 vendored」を前提とするが、**実 fixture には k0s 実データが存在**（§1.2 ⚠・§5-4）。検証深度の再確認を推奨。

---

## 8. Next Steps

1. 本ギャップ分析を踏まえ requirements discussion で §5 の設計判断項目（特に 1・2・4・5）を詰める。
2. `/kiro-design areka-P0-balloon-parse` で技術設計へ進む（facade API 形状・モデル型・分割粒度・マージ規則・fixture 取り込みを設計文書化）。

---

## 9. 設計フェーズ記録（Design Phase Log）— 2026-07-01

> 生成: kiro-spec-design。design.md 生成に伴う discovery 種別・synthesis 結論・設計判断の確定を記録。

### 9.1 Discovery 種別

- **Light discovery（Extension）**。本 spec は既存 `areka-parsers` クレートへ `sakura` 確立パターンを踏襲した兄弟モジュールを追加する拡張であり、外部 API・新規依存・非同期・I/O が無い。深いコードベース調査は §1〜§7 のギャップ分析で既に完了済み。追加の WebSearch/WebFetch は不要（純粋 Rust・追加依存ゼロ・vendored fixture）。sakura モジュール実体（`mod.rs`/`model.rs`/`parse.rs`）と emo2 fixture 3 ファイルを再確認して確定値を採取した。

### 9.2 Synthesis（3 レンズ）

- **Generalization**: R2（base フィールド）と R4（サーフェス別上書き）は「フィールド集合の後勝ちマージ」という単一問題の変種。→ `RawFields`（フィールドスロット集合）を base/overlay 共通の中間表現とし、`merge_side(base, overlay, kind)` を 1 つのマージ器に一般化（サーフェスは `kind` 引数で差分吸収）。実装スコープは emo2 使用フィールドに限定（インタフェースのみ一般化）。
- **Build vs Adopt**: kv パースは `str::split_once(',')` で足り、外部パーサ／設定クレート（serde 等）を導入しない（過剰依存回避・sakura と同じ std のみ方針）。マージも「後勝ち overlay」の単純規則ゆえ既製ライブラリ不要。→ 全面 build（薄い自前実装）。
- **Simplification**: sakura の 5 層（model/lexer/decode/parse）を機械的に複製せず、**独立 lexer を廃し** kv 収集を `parse` に統合、balloon 固有の `merge` のみ追加（Option B）。画像参照はファイル名文字列を持たず `SurfaceKind`＋ID の構造のみ（不要な導出を下流へ）。座標は符号意味を計算せず符号付き `i32`＋型名分類で保持（`is_from_far_edge()` 等の述語を作らない＝過剰実装回避）。

### 9.3 設計判断の確定（§5 論点への回答）

| §5 論点 | 確定 |
|---|---|
| 5-1 facade API 形状 | 候補 (a) `parse(descript, s0s, k0s) -> Balloon`（両サーフェス内包・下流再マージ不要）。サーフェス区別は `Balloon::sakura()`/`kero()` の別 `BalloonSide` で型表現（R4.5）。 |
| 5-2 値型 NewType 方針 | 座標は符号付き `i32` を保持、符号意味は**型名**（`WindowPosition`/`WordWrapPoint`/`ValidRect`）＋doc で分類。述語アクセサ（`is_from_far_edge`）は作らない（R3.4「符号保持で足りる」・過剰実装禁止）。 |
| 5-3 lexer 分割 | 独立 lexer 不要。kv 収集は `parse` に統合（Option B）。 |
| 5-4 k0s 検証深度 | **両サーフェス実データ検証**を採用。確定版 requirements（Boundary Context・R6.3）が k0s vendored 済みへ更新済みゆえ矛盾なし。 |
| 5-5 画像参照解決 | how = (b) 構造のみ保持（`SurfaceKind`＋surface ID）。ファイル名導出は下流（命名規約 `balloon{s|k}{ID}.png`）。本 spec は I/O 非依存。 |
| 5-6 未知行保持粒度 | 未使用/未知行は診断目的の生保持（`RawFields` 内・非公開）に留め、モデル公開面へは出さない（R5.1 は保持義務を課さない）。 |
| 5-7 重複キー・行フォーマット寛容度 | 同一ファイル内同キーは後勝ち。CRLF/LF・BOM・前後空白・空値を parse で寛容化（sakura の局所吸収・全域継続に倣う）。 |
| 5-8 fixture 取り込み | (b) 検証最小抜粋をテスト内リテラルに直書き。クレート境界跨ぎ `include_str!` の脆さを回避し areka-parsers 単体で自己完結（R6.4）。実 fixture は正本として残す。 |

### 9.4 確定 fixture マージ値（テスト固定対象・符号確認済み）

- **sakura（s0s 起点→descript→既定）**: windowposition(266,-129) / wordwrappoint.x=-49（descript -34 を起点が上書き）・y=0 / validrect(top=46,bottom=-56,left=36,right=-44) / arrow0(15,90) / arrow1(15,-110) / font "Yu Gothic UI" h=28 / font.color(0,0,0) / anchor.font.color(180,40,40) / kind=Sakura,id=0。
- **kero（k0s 起点→descript→既定）**: windowposition(-190,-75) / wordwrappoint.x=-34（k0s に無し＝descript フォールバック）・y=0 / validrect(top=40,bottom=-70,left=24,right=-48) / arrow0(9,54) / arrow1(9,-125) / kind=Kero,id=0。

### 9.5 分割方針・依存方向（確定）

- ファイル: `balloon/{mod.rs, model.rs, model_tests.rs, parse.rs, parse_tests.rs, merge.rs, merge_tests.rs, validation_tests.rs}` ＋ `lib.rs` に `pub mod balloon;` 追加。
- 依存方向: `model ← parse ← merge ← facade(mod.rs)`（上方向依存禁止）。sakura を import しない独立モジュール。

### 9.6 設計レビューゲート結果

- Mechanical checks: 要件 ID 全網羅（1.1〜6.4）／Boundary 4 セクション充填／File Structure Plan 具体パス充填／Boundary↔File 整合／orphan component 無し — **全 pass**。
- Judgment review: 要件カバレッジ・アーキ準備性・境界明確性・実装可能性 — **全 pass**。
- 修復パス: **0 回**（初回で通過）。真の要件ギャップ・矛盾なし（§5-4 の brief 食い違いは確定版 requirements で解消済み）。

### 9.7 設計ディスカッション記録 — 2026-07-02

> 設計検証（GO）後の設計ディスカッションで確定した事項。design.md／requirements.md へ反映済み。

- **参照優先度の是正（開発者指摘・最重要）**: マージの概念モデルを **base 起点＋overlay 上書き** から、正しい **3段参照優先度** へ全面是正した。各フィールド値は **(1) サーフェス別テーブル `balloonsXXs`/`balloonkXXs`（起点・第1参照）→ (2) `descript.txt` 共通設定（第2参照）→ (3) 内部既定値（第3参照）** の順で解決する。**解決結果の確定値は不変**（§9.4 のテスト期待値は同一）で、変わったのは概念フレーミング・型モデル・ドキュメント記述。requirements R4（タイトル・Objective・全受入基準）／Introduction／Boundary Context／R6.2/6.3、design.md 全体（Overview/Architecture/System Flows/model/merge/Testing/Traceability）を是正。
- **§5-1 の型モデルを (b) 全フィールド per-surface へ確定（設計ディスカッション #1）**: 上記の是正により「共通 vs サーフェス別」を型で区別しないモデルが必然となった。`Balloon` は sakura/kero 各 `BalloonSide` を内包する器に徹し、各 `BalloonSide` が全フィールド（font/color/origin/type/use_self_alpha を含む）を3段参照優先度で解決した確定値として保持する。`descript` 由来の共通値は両サーフェスへ同値で解決・複製（コストはバルーン1定義ぶんで無視可能）。これにより検証レポート明確化1が指摘した「共通/別のフィールド配置ぶれ」リスク（R4.3/4.5）が構造的に消滅。`merge_side` は `resolve_side(surface, descript, kind)` に改称。
- **検証レポート明確化3（自明修正・カテゴリA）**: fixture テスト内リテラルに採取元の正本ファイル名・行のコメントを義務付け、自動照合は本 spec スコープ外と明記（design.md「Fixture 取り込み方式」）。
- **検証レポート明確化2**: 未使用フィールドの生保持は §5-6 の確定どおり（非公開 `RawFields` 内・診断目的・公開契約にしない）で据え置き。
