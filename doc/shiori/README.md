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
