# ギャップ分析: areka-P0-sakura-parse

> 本書はギャップ分析（情報提供）であり、最終的な実装方針の決定ではない。設計フェーズ（`/kiro-spec-design`）および要件ディスカッションの入力として用いる。
> 実物スコープの正本: `doc/emo2-conformance-scope.md` §3。設計判断の正本: `doc/COMPAT_ARCHITECTURE.md`。配置文脈: `.kiro/steering/roadmap.md`（M-boot・parsers トラック）。
> 隣接（並走・別境界）: `areka-P0-shell-parse` / `areka-P0-balloon-parse` / `areka-P0-package-mount`（兄弟 parser）。下流: `areka-P0-sakura-engine`（命令列を消費・再パースしない）。

## 0. 分析サマリー（概要）

- **完全なグリーンフィールド（実装 0 行）**: `crates/*` 全域を走査した結果、さくらスクリプトの字句解析・命令化を行うコードは**現存しない**。`sakura` のヒットは cue テストのゴースト名（さくら/うにゅう）と `dola::ActorKey` のドキュメントコメントのみで、parser は皆無。過去 completed の `areka-P0-script-engine`（pest/Rune ベースの Pasta DSL エンジン・「Phase 6 Sakura Script」を含む ~5,000 行）は**現在のソースツリーに一切存在しない**（`pest`/`PastaEngine`/`ScriptEvent`/`transpil` で grep 全ゼロ）。よって再利用可能な既存 parser 資産はなく、流用する依存も無い。
- **配置の最大論点＝areka は bin-only クレート**: `crates/areka` には `lib.rs` が無く、全モジュールが `main.rs` の `mod xxx;` 宣言で束ねられている（`shiori_host` / `shiori_session` / `reference_brain` ほか）。純粋・host 非依存・単体テスト可能な parser を「`crates/areka` 内のモジュール」（brief 制約）として置くには、(A) areka に `lib.rs` を新設してライブラリ面を生やす、(B) bin の `mod` として `main.rs` 配下に置く、(C) 新規クレート、の三択になり、これが本仕様最大の設計判断（後述 Option / 設計判断 #1）。
- **整合すべき既存パターンは明快**: 型付き enum は dola の `CueCommand` / `BarrierKind` / `RoutingCommand`（`crates/dola/src/cue/command.rs`）が範。`Custom { command, params }` 的な「汎用エスケープ枠」、NewType（`ActorKey(String)`）、`#[derive(Clone, Debug, PartialEq, ...)]` が確立。エラー型は全クレート共通で `thiserror`（areka `SessionError`/`DemoError`、dola `DolaError` の手書き `Display`）。ロギングは `tracing`（ライブラリは発行のみ・subscriber はアプリ層）。Rust 2024 / resolver=2 / `crates/*` ワークスペース。
- **下流との I/O 契約（共有シーム）が要設計**: 出力の型付き命令 enum は `areka-P0-sakura-engine` と共有する片側契約。エンジンは再パースせずタイムライン再生する前提。`\s[...]` 不透明文字列は surface 層、`\q` の target は SHIORI `Reference0`（`OnChoiceSelectEx`）、`\![move]` 引数は window-placement へそれぞれ流れる「未解釈で保持」シーム。命令モデルの所有者・公開クレート・variant 粒度の確定が設計の中心。
- **規模/リスク見立て**: 単一純粋関数＋型定義＋単体テストで、外部依存ゼロ・アルゴリズムは線形スキャンの素直な字句解析。**Effort = S〜M（タグ約12種＋`\![move]`＋寛容パススルー、命令モデル設計込み）／ Risk = Low〜Medium**。難所はコードでなく (1) 命令モデル/I/O 契約の正本性、(2) 各タグの正規化規約（待ち時間 ms 換算・`\n[percent]` 比率基準・`\wN` 短縮の桁解釈・エスケープ `\\`）の確定。これらは要件ディスカッションでの裁定が前提。

---

## 1. 現状調査（Current State）

### 1.1 ワークスペース構成と既存資産

- マルチクレート `crates/*`（`wintf` / `dola` / `areka` / `shiori-abi` / `pilot`）＋ ベンダリング `vendors/pasta`（git サブモジュール）。Rust 2024 Edition・resolver=2・`members = ["crates/*"]`。
- **`crates/areka` の現状（最重要）**:
  - `src/main.rs` — `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` の **bin エントリ**。`mod shiori_host; mod shiori_session; mod reference_brain; mod shiori_demo; mod shiori_*_e2e_tests; mod tests;` を `main.rs` で宣言。
  - **`lib.rs` は存在しない**（bin-only クレート）。既存モジュールはいずれも host/SHIORI 寄りで、純粋 parser の置き場所として直接の前例は無い。
  - 依存: `wintf` / `shiori-abi` / `windows` / `bevy_ecs` / `thiserror` / `tracing` / `tracing-subscriber` / `async-io`。**parser は新規にこのいずれにも依存しない純粋ロジックで足りる**（std のみで可能）。
- **`crates/dola`** — プラットフォーム非依存の演出定義ライブラリ。型付きコマンド enum・NewType・`thiserror` 風エラーの**範例の宝庫**（後述 1.2）。
- **`crates/pilot`** — 使い捨て feasibility 用の examples 置き場（`src/lib.rs` ＋ `examples/`）。本仕様の恒久成果物の置き場としては不適。

### 1.2 整合すべき既存パターン（命名・型・エラー・ログ）

- **型付き命令 enum の範**: `crates/dola/src/cue/command.rs`
  - `CueCommand`（6 バリアント・データ系のみ）: `Text(String)` / `Clear` / `Emote { key } ` / `Choice { id, text }` / `EntityRef(u64)` / **`Custom { command: String, params: DynamicValue }`**。
    → 本 parser の **汎用 `\!` コマンド命令**・**raw/unknown 命令**は、この `Custom { command, raw_args }` 型の「汎用枠」パターンと直結する有力な範。
  - `BarrierKind` の `WaitForInput { timeout: Option<f64> }` 等 → **待ち命令（Duration 正規化）**の表現先例。
  - `ActorKey(String)`（NewType ＋ `From<&str>`/`Display`）→ **話者スコープ `\p[n]`** の番号・**`\s[...]` 不透明文字列**の NewType ラップ候補。
  - 共通 derive: `#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]`（値系）／ `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]`（f64 含む系は Eq/Hash 外す）。**serde 派生がデフォルト**な点に注意（命令モデルにも付すか否かは設計判断 #4）。
- **エラー型の規約**: 全クレート共通で `thiserror`。
  - areka: `SessionError` / `DemoError`（`#[derive(thiserror::Error, Debug)]` ＋ `#[error(...)]` ＋ `#[error(transparent)]`/`#[from]`）。
  - dola: `DolaError`（手書き `impl Display`・21 バリアント・`std::error::Error` 実装・Display 文字列を回帰テスト）。
  - ただし本 parser は **寛容パススルー（要件 10）＝エラーを送出せず raw/unknown 命令で吸収**が基本方針。よって「解析全体を失敗させる Result<…, E>」は最小（空入力等は失敗ではない）か、あるいは**不要**かもしれない（設計判断 #5）。
- **ロギング**: `.kiro/steering/logging.md` — ライブラリ（wintf/dola）は `tracing` マクロ発行のみ・subscriber 初期化はアプリ層。スコーププレフィックス `[function_name]`・構造化フィールド優先。純粋 parser に過剰ログは不要だが、寛容パススルー時に `warn!`（未知タグ遭遇）を 1 点入れる程度が steering 整合（任意）。
- **命名規約**（structure.md）: ファイル `snake_case.rs`・型 `PascalCase`・関数 `snake_case`・定数 `SCREAMING_SNAKE_CASE`。単体テストは in-source `#[cfg(test)] mod tests` か `{module}/tests.rs`。肥大化時は `{module}/mod.rs`＋サブモジュール分割（dola `runtime/` が代表）。

### 1.3 下流・隣接シーム（コード依存は無いが I/O 契約あり）

- **下流 `areka-P0-sakura-engine`**（roadmap: さくらスクリプト再生エンジン）: 命令列を消費し timeline 再生（`\w/\_w` wait・`\s` で shell-engine へ surface 指令・text を text-layer へ）。**命令モデル型を共有する I/O 契約**。本 parser はこの型の生成者、engine は消費者。型の所有クレートを両者が依存できる位置に置く必要（設計判断 #1/#3）。
- **`\s[...]` の不透明中身** → surface 層 / `areka-P0-shell-parse`（`kero.surface.alias` の日本語エイリアス解決）。parser は中身を**改変・数値解釈せず素通し**（要件 2.3）。
- **`\q[disp,target]` の target** → SHIORI `OnChoiceSelectEx` の `Reference0`（emo2-conformance §1）。`crates/areka/src/reference_brain.rs` が ReferenceN 経路の正解見本を持つが、本 parser は target 文字列を分離保持するのみ（解決しない）。
- **`\![move,...]` 引数** → render / window-placement で実行（parser は decode のみ・実行しない）。
- **`%username`** → 実行時コンテキストで展開（parser はトークン化のみ）。

### 1.4 過去 spec との関係

- **`areka-P0-script-engine`（completed・現ツリー不在）**: Pasta DSL（脳側 DSL）のエンジンであり、SHIORI 応答 Value のさくらスクリプト解析（本仕様）とは**別物**。コードも残存しないため流用不可・参照は概念上のみ。
- **旧 `areka-P0-sakura-script`（履歴）**: 「全タグ網羅」志向 → 本仕様は emo2 実需の**約12タグ＋`\![move]`**へ rescope（emo2-conformance §6・brief「Existing Spec Touchpoints」）。

---

## 2. 要件→資産マッピング（Missing / Unknown / Constraint）

| 要件 | 必要な技術要素 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| R1 純粋関数契約・順序保持・空入力 | 字句スキャナ＋命令 Vec 生成・順序保持・`""→[]` | 無 | **Missing**（新規・std のみ） |
| R2 完全 decode（未デコード断片を残さない） | タグ→値正規化済み命令への完全変換 | 無 | **Missing** |
| R2/R9 テキストラン | タグ間プレーンテキストの Text run 化・連続結合規約 | `CueCommand::Text(String)` の範 | Missing（パターンは既存） |
| R2 話者スコープ `\p[n]` | 番号付き話者スコープ命令 | `ActorKey` NewType の範 | Missing |
| R2/R2.3 サーフェス `\s[...]` 不透明 | 中身を不透明文字列で保持・無改変 | NewType ラップ範 | Missing；**Constraint**=数値解釈/エイリアス解決禁止 |
| R3 待ち時間正規化 `\w[n]`/`\wN`/`\_w[ms]` | 3 表記→単一 Duration。ms 換算規約 | `BarrierKind` の `f64`/`Duration` 範 | **Missing**；**Unknown**=`\w[n]`/`\wN` の単位・基準 ms（設計判断 #6） |
| R4 改行/割合改行 `\n[percent]`/`\n` | percent→比率 decode・素 `\n` 既定比率 | 無 | Missing；**Unknown**=基準（150=1.5 は確定、内部表現は f32/有理数か） |
| R5 選択肢 `\q[disp,target]`＋`\![*]` | disp/target 分離・選択肢マーカー消化 | `CueCommand::Choice { id, text }` の範 | Missing；**Unknown**=`\![*]` マーカーと `\q` の結合規約 |
| R6 カーソル/制御 `\_l[x,y]`/`\e`/`\c`/`\-` | 各型付き命令（x,y 単位 em/lh は保持のみ） | 単純 unit variant 範 | Missing |
| R7 Move と汎用 `\!` | `\![move,...]` のみ引数 decode・他は汎用保持 | `CueCommand::Custom { command, params }` の範 | Missing；**Unknown**=move 引数列の構造（dx,dy,...,base,base） |
| R8 システム変数 `%username` | 展開なしトークン化 | 無 | Missing |
| R10 寛容パススルー | 未知タグ/不正トークンを raw/unknown 命令で吸収・継続 | `Custom` 汎用枠の範 | Missing；**Constraint**=エラー送出禁止・前後の正常命令を欠落させない |
| R11 拡張シーム | variant 追加に開いた命令種別（`#[non_exhaustive]`／汎用 raw variant） | dola enum 群が前例 | Missing；**設計判断 #2** |
| R12 純粋性・UTF-8・テスト可能 | 決定的純粋関数・UTF-8 前提・host 非依存単体テスト | テスト規約あり | Missing（実装規律）；**Constraint**=Shift_JIS 変換なし |

凡例: Missing=新規実装が必要 / Unknown=設計フェーズで確定すべき規約 / Constraint=既存方針・スコープ境界による制約。

---

## 3. 実装アプローチの選択肢

### 3.1 配置（クレート/モジュール構造）— 本仕様の主論点

#### Option A: areka に `lib.rs` を新設し、parser をライブラリモジュールとして公開
- 構成: `crates/areka/src/lib.rs` を新設 → `pub mod sakura;`（または `sakura_parse`）。`main.rs` は `use areka::sakura::...;`。命令モデルもここに置き `pub` で下流へ公開。
- ✅ brief 制約「`crates/areka` 内のモジュール・新規クレートを作らない」に最も忠実。
- ✅ 純粋関数を `lib` 側へ寄せ、bin（main）から分離＝単体テストが素直（`cargo test -p areka` で lib テスト）。
- ✅ 下流 `sakura-engine` が `areka::sakura::Instruction` を直接 import できる（I/O 契約の共有が自然）。
- ❌ bin-only だった areka に lib 面を生やす構造変更（Cargo の `[lib]` 追加・`main.rs` の依存方向整理）。既存 `shiori_*` モジュールとの責務分界の明確化が要る。
- ❌ areka は将来 wintf/window/COM へ依存する重いアプリ層。純粋 parser がその依存グラフに同居する（コンパイル単位の肥大・並走テストでの巻き込み）。

#### Option B: bin の `mod` として `main.rs` 配下に置く（最小変更）
- 構成: `crates/areka/src/sakura/mod.rs` ＋ `main.rs` に `mod sakura;`。テストは in-source `#[cfg(test)]`。
- ✅ 既存 `shiori_host` 等と同じ idiom・構造変更ゼロ・最小。
- ❌ **下流 `sakura-engine` が命令モデル型を import できない**（bin の mod は外部から見えない）。I/O 契約の共有（要件「共有 I/O 契約の片側」）が成立しない致命的弱点。
- ❌ 純粋ロジックが bin に閉じ込められ、再利用・分離テストがしにくい。
- → **I/O 契約共有要件と矛盾**するため、単独では不適。Option A/C への踏み台としてのみ。

#### Option C: 命令モデル型を共有可能な位置（areka lib もしくは別の薄いクレート/dola 隣接）へ、parser ロジックと分離して配置
- 構成例 C1: areka `lib.rs`＋`sakura/`（Option A）に**揃えつつ、命令モデル（型）と parser（関数）をモジュール分割**（`sakura/model.rs`＝型＝engine 共有、`sakura/parse.rs`＝関数）。
- 構成例 C2: 命令モデルを下流 engine と共有しやすい中立クレート（例: 新規 `sakura-script` 軽量クレート、または dola 隣接）に置き、parser はそれを使う。
- ✅ I/O 契約（型）と実装（関数）を分離 → engine は型クレートのみ依存・parser 実装に巻き込まれない。クロスエンジン契約の正本が明確。
- ✅ R11 拡張シーム（variant 追加）を型クレート側で一元管理。
- ❌ C2 は「新規クレートを作らない」brief 制約と緊張（roadmap 規律「abstraction は 2 例目の実物が要求してから」）。現時点で消費者は engine 1 つのみ → **YAGNI の可能性**。
- ❌ C1 でも areka lib 化（Option A の構造変更）は必要。

**配置に関する所見**: brief 制約（areka 内モジュール・新規クレート回避・最小実装）を厳守するなら **Option A（または C1 のモジュール内分割）が素直**。ただし areka の bin-only 性と「下流と型を共有する I/O 契約」要件の両立には areka を lib 化する判断が必須で、これは brief「配置は着手時に確定」に該当する**未裁定の構造判断**（→ 設計判断 #1）。Option B は I/O 契約要件と矛盾、Option C2 は YAGNI 緊張ありで、いずれも要件ディスカッション/設計での裁定対象。

### 3.2 字句解析アルゴリズム

- **Option α: 手書き線形スキャナ（文字カーソル走査）**
  - `\` を見たらタグ開始、`[` で角括弧引数収集、`,` 分割、`%` でシステム変数、それ以外は Text run に蓄積。
  - ✅ 依存ゼロ・std のみ・UTF-8（`char_indices`）で素直・寛容パススルー（不正は raw で吸収）と相性良。最小実装規律に合致。
  - ✅ `\\` エスケープ・`\wN` 短縮（1 桁直後）等の文脈依存も手書きが扱いやすい。
  - ❌ タグが増えると分岐が膨らむ（ただし emo2 subset は約12種で収まる）。
- **Option β: 正規表現/パーサコンビネータ（nom/winnow 等）/PEG（pest）**
  - ✅ 宣言的・拡張容易。
  - ❌ **新規外部依存の導入**（現 areka/dola 依存に parser ライブラリは無い）。最小実装・依存最小規律に反する。`\wN` 短縮や寛容パススルーの「途中不正を飲み込んで継続」は PEG だと逆に書きにくい。emo2 subset 規模に対し過剰。
- **所見**: 手書き線形スキャナ（Option α）が規模・依存・寛容性すべてで素直。コンビネータ導入は YAGNI。

### 3.3 命令モデルの形（型設計）

- **Option i: フラットな単一 enum `Instruction`（`#[non_exhaustive]`）＋汎用 raw variant**
  - 例: `Text(String)` / `SpeakerScope { n }` / `Surface(OpaqueSurface)` / `Wait(Duration)` / `NewLine { ratio }` / `Choice { disp, target }` / `Cursor { x, y }` / `End` / `Clear` / `Quit` / `Move(MoveArgs)` / `GenericCommand { name, raw_args }` / `SystemVar(...)` / `Raw(String)` / `Unknown { … }`。
  - dola `CueCommand` と同型で最も素直。`#[non_exhaustive]` ＋ 汎用枠で R11 拡張シームを満たす。
- **Option ii: カテゴリ分割（待ち/描画/制御/未知の入れ子 enum）**
  - 早すぎる構造化。emo2 subset 規模では過剰（YAGNI）。
- **所見**: Option i（フラット enum＋`#[non_exhaustive]`＋汎用 raw/generic variant）が dola 前例・最小実装・拡張シーム要件に合致。`Duration` は std `std::time::Duration` か独自正規化型か（設計判断 #6）。

---

## 4. 設計フェーズへ持ち越す論点（Research Needed / 設計判断）

> 以下は**要件ディスカッション・設計フェーズの入力**。本ギャップ分析では決定しない。

1. **【配置・最重要】**: areka を `lib.rs` 新設でライブラリ化し parser を公開モジュールにするか（Option A/C1）、命令モデルのみ別の共有位置へ出すか（C2）。下流 `sakura-engine` との型共有 I/O 契約をどのコンパイル単位が所有するか。bin-only 構造変更の是非。brief「配置は着手時に確定」の裁定点。
2. **【拡張シームの形】**: R11 を `#[non_exhaustive] enum` で満たすか、汎用 `Raw`/`GenericCommand` variant で満たすか、両方か。variant 追加の互換性方針。
3. **【I/O 契約の所有と公開範囲】**: 命令モデル型の `pub` 範囲・serde 派生の要否（dola はデフォルト serde 派生。本命令列をシリアライズ/スナップショットテストする計画があるか）。`PartialEq`/`Eq`/`Hash` の付与方針（Duration/f64 を含むと Eq/Hash 不可）。
4. **【テスト資産】**: emo2 実 boot script を単体テストのフィクスチャとして同梱するか（`crates/areka/src/.../tests` か `tests/` か）。`C:\…\ghost_dev\…\emo2` 由来の実スクリプトをリポジトリに取り込む可否・ライセンス。スナップショット（insta 等）導入の是非（新規依存）。
5. **【エラー方針の具体化】**: 寛容パススルー（R10）の下で parser の戻り値型を `Vec<Instruction>`（失敗なし・全て命令で吸収）にするか、`Result` を残すか。`thiserror` エラー型を定義する必要があるか（送出しないなら不要かもしれない）。
6. **【値正規化規約の確定】**（Unknown 群）:
   - `\w[n]` / `\wN`（短縮）の単位・基準 ms（ukadoc 既定の wait 量・`\wN` の 1 桁解釈）。`\_w[ms]` は絶対 ms で確定。3 表記を統一する内部型（`std::time::Duration` 推奨か独自）。
   - `\n[percent]` の比率内部表現（`150→1.5` は確定。f32/f64/有理数）と素 `\n` の既定比率値。
   - `\![move,dx,dy,...,base,base]` の引数列構造（base の意味・可変長の扱い）。decode 後の `MoveArgs` 型形。
   - `\q[disp,target]` と選択肢マーカー `\![*]` の結合規約（マーカーをどの命令へ畳むか）。
   - `\\`（バックスラッシュエスケープ）・`%` リテラル・未閉じ `[` 等の境界ケースの寛容処理規約。
7. **【roadmap pass/fail との整合】**: roadmap の本ユニット完了条件は「boot script を **token 化**」だが、確定 requirements は「**完全 decode 済み型付き命令列（AST）**」を要求。token 化より一段深い（値正規化込み）。両者の差分は意図的な要件昇格と解されるが、roadmap 文言の更新要否を設計フェーズで確認（steering 領分・本仕様外だが整合確認は必要）。

---

## 5. 規模・リスク

- **Effort: S〜M（1〜7 日）**
  - 字句スキャナ（手書き・線形）＝小。命令モデル enum 定義＝小。各タグ decode（約12種＋move）＝小〜中。寛容パススルー＝小。emo2 boot script 単体テスト＝中（フィクスチャ整備）。
  - areka lib 化（Option A 採用時）の構造変更が加われば +S。
- **Risk: Low〜Medium**
  - アルゴリズム的難所なし（線形スキャン・外部依存ゼロ・純粋関数）＝ Low 寄り。
  - Medium 要因: (1) 命令モデル/I/O 契約の正本性（下流 engine との型共有・配置判断）、(2) 値正規化規約（待ち時間 ms・比率・move 引数・エスケープ）の確定。コードでなく**仕様の確定**が主リスク。
  - 配置（設計判断 #1）と値正規化（#6）の裁定が遅れると手戻りの可能性 → 要件ディスカッションで優先裁定推奨。

---

## 6. 設計フェーズへの推奨

- **推奨アプローチ（暫定・裁定前提）**: 字句解析は手書き線形スキャナ（3.2 α）、命令モデルはフラット `#[non_exhaustive] enum`＋汎用 raw/generic variant（3.3 i・dola `CueCommand` 準拠）、配置は areka を `lib.rs` 化して `sakura` モジュールを公開（3.1 A、型と関数はモジュール内分割＝C1）。これが brief 制約（areka 内・新規クレート回避・最小実装）と I/O 契約共有要件を最もよく両立する。ただし**配置（#1）は要裁定**。
- **設計で確定すべき主要決定**: (1) 配置/lib 化と I/O 契約の所有者、(2) 命令モデルの全 variant と拡張シームの形、(3) 値正規化規約（#6 の各項）、(4) 戻り値型とエラー方針、(5) テストフィクスチャ（emo2 実 boot script）の取り込み。
- **持ち越すリサーチ項目**: ukadoc の `\w[n]`/`\wN` 既定 wait 量・`\![move]` 引数仕様の一次確認、`\n[percent]` 比率基準、`\\`/未閉じ角括弧の寛容処理境界。emo2 実 boot script の所在（`ghost_dev/.../emo2`）からのフィクスチャ抽出。

---

## 7. 確定した構文モデル（要件ディスカッション結論・設計入力）

> 要件ディスカッション（議題 #2 構文/エスケープ・#3 `\q` 旧仕様除外）と ukadoc 一次確認で確定したパース対象の構文モデル。設計フェーズの実装指針。**意味デコードは emo2 subset 限定**、**構文パースは全さくらスクリプト**（要件 13）。

### 7.1 正準形（現代タグ・汎用に区切れる）

- `\` ＋ **コマンドワード** ＋ `[` 引数（`,` 区切り）`]`。`[` がワード終端を、`]` が引数終端を機械的に決めるため、**未知タグでも構文として区切れる**（R13 の頑健性の核心・線形スキャナで素直）。
- `\![...]` は word=`!`・**第 1 引数が実コマンド**（例: `\![open,sliderinput,...]` / `\![move,...]`）。`\!` の意味分岐は第 1 引数で行う（②意味層）。

### 7.2 正準形で切れない例外（既知テーブル／別形で扱う）

1. **bare タグ**（ブラケット省略・`\e` `\c` `\-` `\n`）— 既知の小テーブルで引く（汎用には区切れない）。
2. **`\wN` 短縮形**（`\w` ＋ 1 桁数字・ブラケットなし）。内部的には `\w[N]` と同一の Wait へ正規化。
3. **`%keyword` システム変数**（`%username` 等・`\`/`[]` ではない**並列の別形**）。`\%` エスケープのみが構文層で関係。
4. **エスケープ**: `\\`→`\` ／ `\%`→`%` ／ 角内 `\]`→`]` ／ 引数クォート `"..."`（`,` 内包・`""`=リテラル `"`）。これが「ブラケット内 `,` 区切り」を**正確化**する規則。

### 7.3 除外（旧仕様・areka 対象外）

- **`\q` の 2 連ブラケット形式 `\q[ID][タイトル]` / `\q*[ID][タイトル]`**（ukadoc 明記の「[旧仕様]」・`*`=通し番号）。これは**さくらスクリプトで唯一の `[...][...]` 連続形**。
- areka は**現行の `\q[タイトル,ID]`**（カンマ区切り単一ブラケット・第 1=タイトル=disp、第 2=ID=target）のみ Choice として decode する（要件 5.1）。
- 旧 2 連形は**意味デコードせず**、寛容パススルー（R10/R13.8）で吸収——①構文層は落とさず、宙に浮く 2 個目の `[...]` は Raw/Text として保持し隣接命令を壊さない。「無視（=②非対象）」と「クラッシュ」は別物。emo2 は現行形のみ使用。

### 7.4 簡約 EBNF（設計の出発点）

```ebnf
element   = tag | bareTag | shorthand | sysvar | text ;
tag       = "\" , word , "[" , [ arg , { "," , arg } ] , "]" ;   (* 正準・汎用 *)
bareTag   = "\" , knownChar ;                                     (* \e \c \- \n  既知テーブル *)
shorthand = "\w" , DIGIT ;                                        (* \wN *)
sysvar    = "%" , keyword ;                                       (* %username *)
arg       = quoted | raw ;                                        (* "…"(, 内包) / \] エスケープ *)
text      = { textchar | "\\" | "\%" } ;
(* 除外: tag が "]" 後にさらに "[" を持つ旧 2 連形（\q 旧仕様のみ）→ ②非対象・①は Raw/Text で吸収 *)
```

---

## 8. 設計フェーズ決定（design.md 生成時・確定）

> `/kiro-spec-design` 実行で確定した設計判断。§4 の持ち越し論点に対する裁定結果と synthesis 結論。design.md が自己完結の正本であり、本節はその根拠ログ。

### 8.1 discovery 種別と synthesis 結論

- **Discovery 種別 = light（拡張・統合フォーカス）**: ロジックはグリーンフィールドだが、外部リサーチ不要（構文モデルは §7 で確定済み・外部依存ゼロ）。整合対象は areka 構造・dola `CueCommand` パターン・steering 規約で、Grep/Read による既存パターン分析が中心。新規外部依存の WebSearch 検証は不要（手書き線形スキャナ＋std のみ）。
- **Generalization（synthesis #1）**: 待ち時間 3 表記（`\w[n]`/`\wN`/`\_w[ms]`）を単一 `Wait(Duration)` へ一般化（要件 3.4 が明示要求）。寛容パススルーは「区切れたが意味未対応（`GenericCommand`/`Raw`）」と「区切れない不正（`Raw`）」の 2 シームへ一般化。これ以上のカテゴリ分割（待ち/描画/制御の入れ子 enum）は YAGNI ゆえ却下＝フラット enum 維持（§3.3 i）。
- **Build vs Adopt（synthesis #2）**: 字句解析は **build（手書き線形スキャナ）**。nom/pest/winnow/正規表現は emo2 subset 規模（約 12 種）に対し過剰＋新規依存ゆえ却下（§3.2）。待ち時間正規化型は **adopt（std `std::time::Duration`）**。命令モデルは dola `CueCommand` の**パターンを adopt（型は再定義）**＝直接の型依存はしない（純粋・並走安全維持）。
- **Simplification（synthesis #3）**: `thiserror` エラー型を**定義しない**（寛容パススルーゆえ送出しない・§4 #5 裁定）。戻り値は `Vec<Instruction>` 直返し（`Result` 不要）。serde 派生を**外す**（シリアライズ計画なし・YAGNI・§4 #3 裁定）。Option C2（別クレート）は消費者 1 つ＝YAGNI ゆえ却下。

### 8.2 §4 持ち越し論点の裁定

| # | 論点 | 裁定 |
|---|------|------|
| 1 | 配置（最重要） | **Option A / C1**: areka に `lib.rs` 新設＋`sakura` モジュール公開。型（`model.rs`）と関数（`lexer`/`decode`/`parse`）をモジュール内分割。下流 engine は `areka::sakura::Instruction` を直接 import。Option B（bin mod）は I/O 契約共有不能ゆえ却下、C2（別クレート）は YAGNI ゆえ却下。 |
| 2 | 拡張シームの形 | `#[non_exhaustive] enum Instruction` **＋** 汎用 `Raw` / `GenericCommand` variant の**両方**（variant 追加は後方互換・消費側 `match` は `_ =>` 必須）。 |
| 3 | I/O 契約の公開と serde | `pub` は `parse` ＋ `Instruction` ＋値型（`SurfaceArg`/`NewLineRatio`/`Choice`/`MoveArgs`）。lexer の `Token` は `pub(crate)`（非公開）。**serde 派生なし**。`#[derive(Clone, Debug, PartialEq)]`（`f32`/`Duration` ゆえ `Eq`/`Hash` なし）。 |
| 4 | テスト資産 | in-source `#[cfg(test)] mod tests`（`sakura/tests.rs`）。emo2 実 boot script のリポジトリ同梱可否は未確定（OPEN QUESTION #3）だが、タグ個別の手書きフィクスチャで done の網羅性（要件 12.4）を担保可能ゆえ**非ブロッキング**。insta 等スナップショット依存は導入しない。 |
| 5 | エラー方針 | `Vec<Instruction>` 直返し（失敗なし）。`thiserror` エラー型は定義しない。 |
| 6 | 値正規化規約 | 型の**外形は確定**（`Wait(Duration)` / `NewLineRatio(f32)` / `Choice{disp,target,references}` / `MoveArgs{args:Vec<String>}` / `Cursor{x,y}` は文字列保持）。内部定数（`\w`/`\wN` 基準 ms・素 `\n` 既定比率）は実装時裁定（OPEN QUESTION #1/#2）＝型契約に影響せず。`Cursor`/`MoveArgs` は意味割当をせず生引数保持＝「decode = 構文区切り＋引数分割」と Out of Boundary（実行・意味割当は別 spec）を両立。 |
| 7 | roadmap pass/fail 整合 | roadmap L86「boot script を token 化」に対し確定要件は「完全 decode 済み型付き命令列」＝**意図的な要件昇格**（§4 #7）。design.md はこの昇格を反映。roadmap 文言更新は steering 領分・本 spec 外（kiro-complete 時に整合確認）。 |

### 8.3 依存方向（design.md 確定・実装/レビューで違反はエラー扱い）

`model`（型・依存なし） ← `lexer`（model のみ） ← `decode`（model + lexer） ← `parse`（lexer + decode）。各層は左の層のみを import し、上方向（右→左）の import は禁止。`tracing` は `decode` の任意発行のみ。host-32/conductor/wintf/dola/shiori-abi へのコード依存は全層で禁止（純粋・並走安全）。

### 8.4 設計レビューゲート結果

- **Mechanical checks**: 全 numeric requirement ID（1.1〜13.8）が traceability 表に出現 ✓ / Boundary 4 節（Owns・Out・Allowed・Revalidation）populated ✓ / File Structure Plan に具体パス ✓ / boundary ↔ file 整合 ✓ / orphan component なし（model/lexer/decode/parse/tests は全て file マップ済み）✓。
- **Judgment review**: 境界明示・依存方向明示・interface 具体（signature ＋ pre/post/invariant）・`\q` 旧形除外と「Move decode = 構文区切り」で要件 7.1 と Out of Boundary の緊張を解消。OPEN QUESTION 3 件は値定数のみで型契約に影響せず＝真の spec gap ではない。
- **結果**: **1 パス目で通過**（修復パスなし）。

### 8.5 設計ディスカッション裁定（§8.2 #1・§8.1 synthesis #3 を上書き）

> 設計ディスカッション（議題1）で配置判断を覆した。本節が §8.2 #1（Option A/C1）および §8.1 synthesis #3 の「Option C2 却下」に**優先する**。

- **配置 = Option C2（パーサー専用クレート）に確定**: 新クレート **`areka-parsers`**（純粋・`std` のみ・host 非依存）を新設し、`sakura` モジュールを置く。当初の areka lib 化（A/C1）は破棄。
- **覆した根拠**:
  1. **重依存の漏れ**: パーサーは純粋・std のみ。重い `areka`（windows/bevy_ecs/wintf/D2D）に同居させると、下流 `sakura-engine` が型1個のために areka 全体へ依存する＝アーキテクチャ臭。専用クレートなら下流は軽量依存で済む。
  2. **議題1（フィールド可視性）の根治**: 共有契約を重バイナリに埋めたことが `SurfaceArg`/`NewLineRatio` の「別クレートから読めない」穴の遠因。公開前提の専用クレートへ出せば問題が消える（＋NewType は `as_str()`/`ratio()` アクセサで読み取り公開＝dola `ActorKey` 流儀）。
  3. **役割集約**: M-boot のパーサー4兄弟（sakura/shell/balloon/package-mount）は全員が同一素性（純粋/std/host 非依存）。役割で1クレートに束ねるのが自然でクレート乱立も防ぐ。roadmap「横断構造は最初から正しく持つ」と整合。brief も配置を「着手時に確定」と open にしていた（制約違反でない）。
- **YAGNI 規律**: 本 spec が作るのは `areka-parsers` クレート＋`sakura` モジュール**のみ**。兄弟（shell/balloon/package）の空スタブは作らない＝各 spec が着手時に自分のモジュールを追加（balloon-system の空 spec 工場を踏まない）。
- **areka 影響**: なし（bin のまま・lib 化しない）。
- **議題1（アクセサ）はこの裁定に吸収**: 専用クレート化＋NewType アクセサ追加で I/O 契約が機能する形に確定。
