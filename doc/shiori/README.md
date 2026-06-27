# doc/shiori — SHIORI 互換契約資産ルート

本ディレクトリは、areka-P0-shiori-protocol 仕様が定義する **SHIORI 正準 content プロトコル契約** の資産ルートである。

## 正本（SSOT）は `shiori_protocol.toml` ただ 1 枚

- **`shiori_protocol.toml` が契約の唯一の正本（single source of truth, SSOT）。** イベント／リソースカタログ、フィールドスキーマ、意味名⇔`ReferenceN` 対応表、json-rpc 封筒マッピング、予約 SHIORI ヘッダ集合、沈黙裁定ログ、バージョニング方針を、この 1 ファイルへ機械可読データとして符号化する。
- **契約定義を `shiori_protocol.toml` 以外のいかなるファイルにも分散させてはならない。** 契約は唯一の正本に集約し、host-32・reference・pasta・codegen・doc/Web のいずれにも二重定義を置かない（要件 3：単一正本対応表）。

## doc / Web は正本ではなく派生（generated FROM the TOML）

- 人間可読な doc 台帳・Web ページは、`shiori_protocol.toml` から **生成される派生レンダリング（derived / generated）** であり、**正本ではない**。
- doc/Web は常に正本と同値であること（生成のたびに正本から再構築されること）を保つ。正本に存在しない記述を doc/Web へ手書きしてはならない。
- TOML の `#` コメントはパース時に失われ生成に使えないため、人間可読説明は全テーブル・全エントリ・全フィールドの `description` キー（データ）として正本側に保持する（要件 11.4）。
- doc/Web 生成器の実装は本仕様のスコープ外（後続フェーズ）。本 README はアプローチ（入力＝正本、出力＝派生、同値保持）を宣言するに留める。

## doc/Web 生成アプローチの受け入れ基準（生成器は下流実装・基準の確定のみ）

doc/Web 派生レンダリングは、正本 `shiori_protocol.toml` から派生を生成する射 `generate: TOML(正本) → doc/Web(派生)` として定義する。生成器コードは本仕様のスコープ外（下流・後続フェーズ）であり、本節はその生成器が満たすべき **受け入れ基準** を確定する（要件 11.2 / 11.5）。

- **AC-G1（入力＝正本のみ）**: 生成器の唯一の入力は `shiori_protocol.toml` 正本とする。手書き doc 断片・別データ等の副入力を取らない（入力＝正本）。
- **AC-G2（description の全展開）**: 全テーブル（`[meta]`／`[envelope]`／`[reserved_headers]`／`[mapping]`／各 `[[silence_ruling]]`）・全 entry（446）・全 field（802）の `description` データを派生本文へ漏れなく展開する。`description` を持つ要素で派生に反映されないものがあってはならない（要件 11.4 を出力側で実体化）。
- **AC-G3（正本に無い記述の非付加）**: 派生 doc/Web は正本に存在しない契約記述を含まない。生成器は正本データの再表現のみを行い、新規の規範内容を加えない（出力＝派生）。
- **AC-G4（同値・冪等＝同値保持）**: 同一正本からの生成は毎回同値の派生を生む（決定的）。正本更新時は派生を再生成し、派生は常に正本と同値に保つ。契約の差分は正本側でのみ発生し、派生は追従する（同値保持）。
- **AC-G5（2投影の人間可読化）**: 意味名（canonical＝`name`）と `ReferenceN`（alias＝`reference`／`reference_variadic`）の 2 投影をいずれも派生へ表示でき、両者が同一 field 由来・同一値であることが読み取れる（要件 3.2 / 10.2 の人間可読化）。型は正本の小文字 Rust 準拠表記のまま表示する（要件 11.3）。
- **AC-G6（生成器の範囲外）**: 生成器コード自体は本仕様の成果物に含めない。Rust 型 / codegen と同様、下流（設計・実装フェーズ／下流クレート）で実装される（要件 11.5）。

**受け入れ判定の検証方法**: 派生生成後、(a) 正本の全 `description` 文字列が派生に出現する（被覆＝AC-G2）、(b) 派生中の契約記述がすべて正本データへ遡源できる（無正本記述ゼロ＝AC-G3）、(c) 同一正本から 2 回生成して同値である（決定性＝AC-G4）、の 3 点をもって受け入れとする。

## Rust 型 / codegen は下流生成（out of this spec's scope）

- 生成された Rust 型（event enum・フィールド struct 等）および codegen 機構は、`shiori_protocol.toml` から **下流（設計・実装フェーズ／下流クレート）で生成される** ものであり、本仕様の成果物には **含めない**（要件 11.5）。
- 正本側の型表記は小文字の Rust 準拠型名（`i32`/`u32`/`i64`/`bool`/`str` 等。大文字混在禁止・文字列は `str`）で統一する（要件 11.3）。

## 典拠スナップショット

- 契約抽出元の ukadoc ピン留めスナップショット（出典 URL・取得日・sha256 を含む `SOURCES.md`）は、`.kiro/specs/areka-P0-shiori-protocol/ukadoc/` に典拠資産として保持される（要件 7.3 / 11.6）。`shiori_protocol.toml` の `provenance` 列および `[[silence_ruling]]` はこのスナップショットを典拠として参照する。

## ファイル一覧

| ファイル | 役割 |
|----------|------|
| `shiori_protocol.toml` | 【正本／SSOT】契約の全符号化 |
| `README.md` | 本ファイル。正本／派生の関係の宣言 |
