# doc/shiori — SHIORI 互換契約資産ルート

本ディレクトリは、`areka-P0-shiori-protocol` 仕様が定義する **SHIORI 正準 content プロトコル契約** の資産ルートである。契約セマンティクスは完了仕様 `areka-P0-shiori-protocol` が確定したものであり、本契約の **物理ソース編成と TOML 符号化形** は後続仕様 `areka-P0-shiori-protocol-split` が再編した（契約内容そのものは非破壊・一切不変）。

## 正本（SSOT）は `fragments/` 群（論理 SSOT）

- **契約の唯一の正本（single source of truth, SSOT）は `fragments/` 配下のフラグメント群、およびその決定的・冪等な結合（merge）結果である。** すなわち「論理 SSOT＝フラグメント群および決定的結合結果」。イベント／リソースカタログ、フィールドスキーマ、意味名⇔`ReferenceN` 対応、json-rpc 封筒マッピング、予約 SHIORI ヘッダ集合、沈黙裁定ログ、バージョニング方針を、フラグメント群へ機械可読データとして符号化する。
- **旧単一ファイル `shiori_protocol.toml` は廃止（tree から削除）された。** 非権威の生成物としても残置しない。かつての「正本＝単一 TOML 1 枚」は、後続仕様で「論理 SSOT＝フラグメント群＋決定的結合結果」へ改訂継承された（経緯は後述「完了仕様 要件3・11 の改訂継承」）。
- **契約定義をフラグメント群以外のいかなるファイルにも分散させてはならない。** 契約は唯一の正本（フラグメント群）に集約し、host-32・reference・pasta・codegen・doc/Web のいずれにも二重定義を置かない（二重定義禁止の維持）。各契約データ（各 entry・各 field・各 silence_ruling・各共有テーブル）はフラグメント群全体で唯一の場所にのみ存在する。

## 正準ビューはオンデマンド merge で得る（常設しない）

- 単一ファイル形式の正準ビューが必要な場合、それは **フラグメント群からのオンデマンド再構成（merge）によってのみ** 得られる。tree に常設の単一ファイル正本は持たない。
- 結合順の単一真実源は `fragments/_manifest.toml` である。ファイル名の `NN.` 数値接頭辞は捜索・可読性の補助であって権威ではない（権威はマニフェスト単独）。
- 再構成（merge）はマニフェスト順で決定的・冪等であり、同一フラグメント群＋同一マニフェストから常に同一の正準ビューを生成する。
- 再構成機構・バリデータの **恒久的な実装コード** は本資産のスコープ外（下流／後続フェーズ）。本 README は正本＝フラグメント群・正準ビュー＝オンデマンド merge という契約を宣言するに留める。

## フラグメント符号化形（keyed/inline）

- 各 entry は id をキーとする連想テーブル `[entry."<id>"]` として、各 field は意味名をキーとする inline table（1 field = 1 行）として、各 silence_ruling は id をキーとする連想テーブル `[silence_ruling."<id>"]` として符号化する。id・意味名の一意性は TOML パーサ自身が機械担保する（重複はキー重複として機械検出）。
- キーは常に quote し、ドット・アスタリスクを含む id（`OnUpdate.OnDownloadBegin`・`char*.defaultx` 等）を破綻なく表現する。
- `ReferenceN` 位置は各 field の `reference`（および可変長末尾は `reference_variadic`）キーで保持し、配列順序の消失が契約に影響しない。
- 共有テーブル（`[meta]`／`[mapping]`／`[envelope]`／`[reserved_headers]`）と全 silence_ruling は単一の共有フラグメント `_shared.toml` へ集約する。`silence_ref` の文字列参照（id の配列）は集約後も解決可能なまま保持する。

## doc / Web は正本ではなく派生（generated FROM the fragments）

- 人間可読な doc 台帳・Web ページは、フラグメント群の正準ビュー（merge 結果）から **生成される派生レンダリング（derived / generated）** であり、**正本ではない**。
- doc/Web は常に正本と同値であること（生成のたびに正本＝フラグメント群から再構築されること）を保つ。正本に存在しない記述を doc/Web へ手書きしてはならない。
- TOML の `#` コメントはパース時に失われ生成に使えないため、人間可読説明は全テーブル・全エントリ・全フィールドの `description` キー（データ）として正本側に保持する。
- doc/Web 生成器の実装は本資産のスコープ外（後続フェーズ）。本 README はアプローチ（入力＝正本フラグメント群の正準ビュー、出力＝派生、同値保持）を宣言するに留める。

## doc/Web 生成アプローチの受け入れ基準（生成器は下流実装・基準の確定のみ）

doc/Web 派生レンダリングは、正本フラグメント群の正準ビューから派生を生成する射 `generate: merge(fragments)(正準ビュー) → doc/Web(派生)` として定義する。生成器コードはスコープ外（下流・後続フェーズ）であり、本節はその生成器が満たすべき **受け入れ基準** を確定する。

- **AC-G1（入力＝正本のみ）**: 生成器の唯一の入力は正本フラグメント群の正準ビュー（merge 結果）とする。手書き doc 断片・別データ等の副入力を取らない（入力＝正本）。
- **AC-G2（description の全展開）**: 全共有テーブル（`[meta]`／`[envelope]`／`[reserved_headers]`／`[mapping]`）・全 silence_ruling・全 entry（446）・全 field（802）の `description` データを派生本文へ漏れなく展開する。`description` を持つ要素で派生に反映されないものがあってはならない。
- **AC-G3（正本に無い記述の非付加）**: 派生 doc/Web は正本に存在しない契約記述を含まない。生成器は正本データの再表現のみを行い、新規の規範内容を加えない（出力＝派生）。
- **AC-G4（同値・冪等＝同値保持）**: 同一正本からの生成は毎回同値の派生を生む（決定的）。正本更新時は派生を再生成し、派生は常に正本と同値に保つ。契約の差分は正本側でのみ発生し、派生は追従する（同値保持）。
- **AC-G5（2投影の人間可読化）**: 意味名（canonical＝field の inline table キー）と `ReferenceN`（alias＝`reference`／`reference_variadic`）の 2 投影をいずれも派生へ表示でき、両者が同一 field 由来・同一値であることが読み取れる。型は正本の小文字 Rust 準拠表記のまま表示する。
- **AC-G6（生成器の範囲外）**: 生成器コード自体は本資産の成果物に含めない。Rust 型 / codegen と同様、下流（設計・実装フェーズ／下流クレート）で実装される。

**受け入れ判定の検証方法**: 派生生成後、(a) 正本の全 `description` 文字列が派生に出現する（被覆＝AC-G2）、(b) 派生中の契約記述がすべて正本データへ遡源できる（無正本記述ゼロ＝AC-G3）、(c) 同一正本から 2 回生成して同値である（決定性＝AC-G4）、の 3 点をもって受け入れとする。

## Rust 型 / codegen は下流生成（out of this asset's scope）

- 生成された Rust 型（event enum・フィールド struct 等）および codegen 機構は、正本フラグメント群の正準ビューから **下流（設計・実装フェーズ／下流クレート）で生成される** ものであり、本資産には **含めない**。
- 正本側の型表記は小文字の Rust 準拠型名（`i32`/`u32`/`i64`/`bool`/`str` 等。大文字混在禁止・文字列は `str`）で統一する。

## 非破壊移行の証拠（一回限り・最小エビデンス）

- 旧 `shiori_protocol.toml` からフラグメント群への移行は **契約内容を一切変更しない非破壊リファクタ**であり、その証拠は移行時の一回限りの意味的同値ゲートで与えられた。`parse(旧 TOML)` と `parse(merge(fragments))` が 8 要素（entry 集合／field 集合／共有テーブル／silence_ruling／全 description／全 provenance／封筒マッピング／予約ヘッダ集合）で順序非依存に過不足なく一致し、残差キーがゼロであることを確認した（PASS）。
- 合否結果と旧ファイルの pre-deletion blob への軽量ポインタを `equivalence_evidence.toml` に最小エビデンスとして残す。変換前 baseline は削除コミットが旧ファイル blob を git 履歴へ恒久保存するため、別途の正規化ダンプは同梱しない。
- 同値ゲートは移行時の一回限りゲート（削除を認可する関門）であり、完全移行後はフラグメント群が唯一の正本ゆえ旧形の再監査は想定しない。

## 典拠スナップショット

- 契約抽出元の ukadoc ピン留めスナップショット（出典 URL・取得日・sha256 を含む `SOURCES.md`）は、`.kiro/specs/completed/areka-P0-shiori-protocol/ukadoc/` に典拠資産として保持される。フラグメント群の `provenance` 列および各 `silence_ruling` はこのスナップショットを典拠として参照する。sha256 によりスナップショットの同一性を担保する。

## 完了仕様 要件3・11 の改訂継承

> （詳細は後続仕様 `areka-P0-shiori-protocol-split` の requirements.md 要件8・design.md DD-6 に記す。`completed/` 配下の履歴は不変のまま、改訂は本仕様側で系譜として追跡する。）

## ファイル一覧

| ファイル / ディレクトリ | 役割 |
|----------|------|
| `fragments/` | 【正本本体】論理 SSOT＝フラグメント群 |
| `fragments/_manifest.toml` | 再構成（merge）順の単一真実源（決定的・冪等） |
| `fragments/_shared.toml` | 共有フラグメント。`[meta]`／`[mapping]`／`[envelope]`／`[reserved_headers]`＋全 silence_ruling を集約 |
| `fragments/events/NN.{category}[.NN].toml` | kind=event のカテゴリ別フラグメント群（≤600 行・カテゴリ純度） |
| `fragments/resources/NN.{category}[.NN].toml` | kind=resource のカテゴリ別フラグメント群（≤600 行・カテゴリ純度） |
| `equivalence_evidence.toml` | 非破壊移行の最小エビデンス（同値ゲート合否＋旧ファイル blob ポインタ） |
| `README.md` | 本ファイル。正本（フラグメント群）／派生の関係の宣言 |

> 旧 `shiori_protocol.toml`（単一正本）は本仕様完了に伴い tree から削除された。正準ビューはフラグメント群からのオンデマンド merge で得る。
