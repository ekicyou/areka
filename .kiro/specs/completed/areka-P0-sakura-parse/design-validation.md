# 設計バリデーションレポート: areka-P0-sakura-parse

> 本書は設計フェーズの品質レビュー結果（非対話・自動実行）。design.md / requirements.md / research.md / spec.json は FINALIZED であり本レビューでは変更しない。判定は GO / NO-GO の一者択一。重大論点は最大 3 件、強み 1〜2 件。

## レビューサマリー

設計は emo2 subset さくらスクリプトパーサを「2 層パイプライン（Lexer 一般構文 ／ Decode emo2 subset 限定）＋フラット命令モデル」として明快に分解し、areka を `lib.rs` 化して下流 `areka-P0-sakura-engine` と型契約を共有する配置判断を根拠付きで確定している。全 numeric requirement ID（1.1〜13.8）が Requirements Traceability 表に出現し、境界 4 節・依存方向・interface 契約（signature ＋ pre/post/invariant）・寛容パススルー方針・`\q` 旧形除外がいずれも具体的に明示されている。実装着手に十分な readiness を備えており、未確定 3 件はすべて型契約に影響しない内部値定数のみで、設計ブロッカーではない。

## 分析（Analysis）

### 要件トレーサビリティ（全 ID 1.1〜13.8）

requirements.md の受入基準を機械的に全列挙し、design.md の Requirements Traceability 表と突合した結果、**欠落 ID ゼロ**を確認した。

- 要件 1（純粋関数契約）: 1.1〜1.5 すべて表に出現（1.4 のみ Flows 欄が `—` だが「副作用なし」を Interfaces 欄で明示しており妥当）。
- 要件 2〜9（emo2 subset デコード）: 2.1〜2.3 / 3.1〜3.4 / 4.1〜4.2 / 5.1〜5.4 / 6.1〜6.4 / 7.1〜7.3 / 8.1〜8.2 / 9.1〜9.2 すべて出現。component（decode/model/lexer）と interface（`Instruction` variant）まで割付済み。
- 要件 10（寛容パススルー）: 10.1〜10.3 出現。`Raw` / `GenericCommand` 2 シームへの吸収先が Error Handling 表でも再掲され整合。
- 要件 11（拡張シーム）: 11.1（`#[non_exhaustive]`）/ 11.2（例外テーブル外 → Raw/Generic）出現。
- 要件 12（純粋性・テスト）: 12.1〜12.4 出現。12.4「boot script 1 本に限定せずタグ個別検証」が Testing Strategy で明示的に担保。
- 要件 13（構文一般パース・エスケープ）: 13.1〜13.8 すべて出現。各エスケープ／クォート／角括弧が lexer に割付済み。

→ **トレーサビリティは完全**。design.md §8.4 の自己申告（全 ID 出現）と独立突合の結果が一致した。

### 2 層境界（Lexer 一般 ／ Decode emo2 subset 限定）

責務分界が design 全体で一貫している。Lexer は「`[` がワード終端・`]` が引数終端を機械的に決める」ため未知タグも構文区切り可能（要件 13 の頑健性の核心）、Decode は emo2 subset のみ意味デコードし subset 外は Raw/Generic へ。Boundary Commitments・Architecture・依存方向（§8.3: `model ← lexer ← decode ← parse`、上方向 import 禁止）・Components 表のすべてでこの分界が崩れていない。Lexer の `Token` 型を `pub(crate)` に閉じ `Instruction` のみ公開する点も I/O 契約の漏れを防いでおり妥当。

### フラット命令リスト（入れ子 AST 無し）契約

`Instruction` を単一フラット enum（`#[non_exhaustive]`）とし、「さくらスクリプト文法は線形」を根拠に入れ子構造を持たない設計判断が明示（model 節 Responsibilities）。research §3.3 の Option ii（カテゴリ分割入れ子 enum）を YAGNI として却下した経緯も §8.1 synthesis #1 に記録。dola `CueCommand` 前例とも整合。**要件には「構造化 AST」表記があるが、design.md は Overview 冒頭で「フラットな構造化命令の Vec」と一貫定義し直しており、用語の不整合は解消されている**。

### `\q` 旧 2 連ブラケット除外

要件 5.3・13.8、Non-Goals、Out of Boundary、Error Handling 表、Testing Strategy のすべてで「旧 `\q[ID][タイトル]` は Choice 化せず、宙に浮く 2 個目 `[...]` を Raw 保持し隣接命令を壊さない」が一貫明示。research §7.3 で「さくらスクリプト唯一の `[...][...]` 連続形・ukadoc 明記の旧仕様」と根拠付け済み。テストでも「旧形が隣接命令を壊さないこと」を固定対象に挙げており、検証可能。

### 寛容パススルー・エラー方針

戻り値 `Vec<Instruction>` 直返し（`Result` 不使用・`thiserror` エラー型を定義しない）が Error Strategy・facade 契約・依存規律で一貫。要件 10.2（エラー送出しない）/ 10.3（前後の正常命令を欠落させない）と完全整合。`tracing` は logging.md 規約どおり「発行のみ・subscriber 初期化なし」。Error Categories 表が状況→応答→命令→要件 ID を網羅しており実装指針として十分。

### areka lib.rs 配置判断（bin-only → lib 面）

現状の bin-only 構造（`crates/areka/src/` に `lib.rs` 無し）を実地確認済み（src 配下は `main.rs` ＋ `shiori_*` のみ）。Cargo.toml も `[[bin]] name = "areka"` のみで `[lib]` 不在を確認。design は `[lib]`（`name = "areka"`, `path = "src/lib.rs"`）追加＋ bin 温存の二面クレート化を選択。Option B（bin mod・I/O 契約共有不能）/ Option C2（別クレート・YAGNI）を却下する根拠が research §3.1・§8.2 に明示され、brief 制約（areka 内モジュール・新規クレート回避）と「下流と型を共有する公開面」要件を両立する唯一解として妥当。既存 `shiori_*` bin モジュールと lib 側 `sakura` の責務非重複も明記済み。

### 未確定 3 件の性質確認

OPEN QUESTION #1（`\w`/`\wN` 基準 ms）・#2（素 `\n` 既定比率）・#3（boot script フィクスチャ取り込み）はいずれも「型の外形（variant 名・構造）に影響しない内部値定数 ／ テスト素材調達」であり、`Wait(Duration)` / `NewLineRatio(f32)` という型契約は確定済み。タスク指示どおり**設計ブロッカーとして扱わない**。

## 重大論点（Critical Issues）

なし（設計ブロッカーに該当する論点は検出されなかった）。

下記は設計ディスカッションで触れてもよい軽微な観察（いずれも GO を妨げない・任意）:

- （軽微・任意）要件側の「構造化 AST」という語と design の「フラット命令列」は、design 内で再定義により整合済みだが、tasks.md 生成時に実装者が要件文言だけ読むと入れ子 AST を期待しうる。tasks 段階で「フラット Vec・入れ子なし」を 1 行明記すると齟齬を確実に防げる。

## 設計の強み（Strengths）

1. **トレーサビリティと境界の二重の堅牢性**: 全 numeric ID（1.1〜13.8）が欠落なく component/interface/flow まで割付され、かつ 2 層境界（Lexer 一般 ／ Decode subset 限定）と依存方向（単方向 import 禁止規律）が design 全体で一切ブレずに貫かれている。実装・レビュー時の判定基準が機械的に検証可能。

2. **YAGNI 規律に沿った設計判断の明示性**: serde 派生除外・`Result`/`thiserror` 不採用・外部 parser 依存ゼロ（手書き線形スキャナ）・別クレート化却下のいずれもが research の選択肢比較と steering（roadmap 実装規律・logging.md・structure.md）に紐付いて根拠付けられており、「最小実装＋薄い拡張シーム」を体現している。dola `CueCommand` 前例の踏襲も既存パターン整合を担保。

## 最終判定（Final Assessment）

### 判定: **GO**

**根拠**: 要件トレーサビリティが完全（1.1〜13.8 欠落ゼロ）であり、2 層境界・フラット命令契約・`\q` 旧形除外・寛容パススルー・lib 配置判断のすべてが根拠付きで確定し相互整合している。未確定 3 件は型契約に影響しない内部値定数のみで設計ブロッカーではない。設計は実装着手に十分な readiness を備える。

### 次フェーズ

- `/kiro-spec-tasks areka-P0-sakura-parse` で実装タスクを生成する。
- tasks 生成時、OPEN QUESTION #1/#2（値定数）#3（フィクスチャ）の裁定を実装タスク内に組み込む。
- （任意）tasks に「`Instruction` はフラット Vec・入れ子 AST なし」を 1 行明記し、要件「構造化 AST」表記との齟齬を予防する。
