# Requirements Document

## Project Description (Input)

`doc/shiori/shiori_protocol.toml` が 10,685 行の単一ファイルへ肥大化し、保守・レビュー・diff が困難になっている。1 イベントの追加・修正でも巨大ファイル全体を読み込む必要があり、LLM 支援・人手レビューの双方でコンテキスト効率が悪い。一方、拡張元の完了仕様 `areka-P0-shiori-protocol`（completed）は「契約の正本は `shiori_protocol.toml` ただ 1 枚／契約定義を他ファイルへ分散させてはならない（要件3・要件11）」を中核不変条件として置いており、素朴に物理分割すると当該不変条件に literal に抵触する。

本仕様は **契約内容そのものを一切変更しない非破壊の物理リファクタ**である。`shiori_protocol.toml` を Event/Resource 単位の keyed table フラグメント群へ再編し、その **フラグメント群そのものを契約の正本（SSOT）** とする。正準契約は分割フラグメントから決定的・冪等に再構成（merge）でき、再構成結果が現行 `shiori_protocol.toml` と **意味的に同値**であることを証明できる。SSOT 保証（単一権威・二重定義禁止・全 `description` のデータ保持・provenance 維持・派生 doc/Web との同値/冪等）は物理分割後も維持する。あわせて完了仕様 `areka-P0-shiori-protocol` の要件3・11（単一ファイル正本）を「論理 SSOT＝フラグメント結合結果」へ改訂する。

上位設計の正本は `doc/COMPAT_ARCHITECTURE.md` §5。契約典拠は ukadoc ピン留めスナップショット（`.kiro/specs/completed/areka-P0-shiori-protocol/ukadoc/` 等）。

## Introduction

本仕様は、完了仕様 `areka-P0-shiori-protocol` が確定した正準 SHIORI content 契約の **物理ソース編成と TOML 符号化形のみ**を再編する。契約セマンティクス（どの event/resource が・どの field を・どの `ReferenceN` 位置で・どの型/必須/応答意味/provenance/description を持つか、封筒マッピング、予約ヘッダ集合、沈黙裁定、バージョニング方針）は **一切変更しない**。

変更は 2 点。(1) 単一の巨大 TOML を Event/Resource 単位の keyed table フラグメント群へ物理分割し、フラグメント群を契約の論理 SSOT とする。(2) TOML 符号化形を `[[entry]]` 配列から id キーの連想配列へ、`[[entry.field]]` 配列から意味名キーの inline table へ刷新し、id/意味名の一意性をパーサ自身に機械担保させる。非破壊の証拠は「現行正本の parse 結果」と「フラグメント結合の parse 結果」の意味的同値ゲートで与える。

本仕様の成果物は **再編後のフラグメント群・共有フラグメント・決定的再構成（マニフェスト/順序）の契約と受け入れ基準・改訂された完了仕様要件・改訂された `doc/shiori/README.md`** までである。再構成機構・バリデータ・doc/Web 生成器・Rust codegen の **実装コード**は本仕様のスコープ外（後続フェーズ）。

## Boundary Context

- **In scope**:
  - `shiori_protocol.toml` の Event/Resource 単位フラグメントへの物理分割（≤600 行・カテゴリ純度・超過カテゴリの entry 境界でのサブ分割）
  - TOML 符号化形の刷新（entry/silence_ruling の id キー連想配列化、field の意味名キー inline table 化、共有テーブルの単一共有フラグメントへの集約）
  - フラグメント → 正準契約の決定的再構成機構の **契約・受け入れ基準**（マニフェスト/順序の固定、意味的同値ゲートの判定基準）
  - 現行正本との意味的同値（無損失・冪等）の証明と provenance/description の保持
  - 完了仕様 `areka-P0-shiori-protocol` 要件3・11 の改訂（論理 SSOT の再定義）
  - `doc/shiori/README.md` の改訂（SSOT＝fragments の宣言・派生の地位）
- **Out of scope**:
  - 契約内容そのものの変更（event/resource の追加削除・field 意味/型/`ReferenceN` 位置・封筒マッピング・予約ヘッダ集合・沈黙裁定・バージョニング方針はすべて不変）
  - 再構成機構・バリデータ・doc/Web 生成器・Rust codegen の **実装コード**（HOW・後続フェーズ／下流）
  - COM ABI（`IShiori`/`IShioriHost`）・トランスポート・さくらスクリプト/SAORI 解釈（隣接仕様の領分）
- **Adjacent expectations**:
  - 上流 `areka-P0-shiori-protocol`（completed）が本契約の正本所有者であり、本仕様はその要件3・11 を改訂継承する（completed の履歴は不変のまま系譜を残す）。
  - 下流 consumer（`areka-P0-shiori-host-32`・`areka-P0-shiori-reference`・pasta native 脳・後続の doc/Web 生成器・Rust codegen）は、入力が「単一ファイル `shiori_protocol.toml`」から「フラグメント群（または再構成された正準ビュー）」へ移ることを lockstep で取り込む。契約セマンティクスは不変ゆえ、影響は符号化形・入力ソースのみに限定される。
  - **未決の設計判断（Q1）**: `shiori_protocol.toml` を「非権威の生成物として tree に残置（暫定推奨・下流無改修）」とするか「tree から削除しオンデマンド結合に一本化」するかは、本要件では確定せず設計フェーズで裁定する。Requirement 7 は処遇の選択肢に依らず満たすべき不変条件（正本でない・残す場合は正本との同値・編集禁止表示）を規定する。

## Requirements

### Requirement 1: フラグメント群の正本化と二重定義禁止

**Objective:** As a 互換契約の管理者, I want 契約の正本を Event/Resource 単位のフラグメント群とし各契約データを唯一の場所に置きたい, so that 単一権威と二重定義禁止を保ったまま巨大単一ファイルを解体できる

#### Acceptance Criteria

1. The 分割後フラグメント群 shall 正準 SHIORI content 契約の論理 SSOT（単一権威）となり、`shiori_protocol.toml` 単一ファイルに代わって本契約の正本としての地位を持つ。
2. The 分割後フラグメント群 shall 各契約データ（各 entry・各 field・各 silence_ruling・各共有テーブル）を、フラグメント群全体で唯一の場所にのみ保持する（二重定義禁止の維持）。
3. The 分割後フラグメント群 shall 各 entry を、その `kind`（`event`／`resource`）に依らず単一のフラグメント形式で表現する。
4. If 同一の entry id・field 意味名・silence_ruling id がフラグメント群内に複数存在する場合, then the 受け入れ検証 shall それを二重定義違反として検出し、再構成を不合格とする。

### Requirement 2: フラグメントのサイズ不変条件と分割境界

**Objective:** As a 契約のレビュアー（LLM・人手の双方）, I want 各フラグメントをレビューに優しいサイズに収めたい, so that 1 イベントの追加・修正で巨大ファイル全体を読まずに済む

#### Acceptance Criteria

1. The 各フラグメント shall 600 行以下に収まる。
2. The フラグメント分割 shall カテゴリ純度を基本とし、各フラグメントを ukadoc カテゴリ単位で区切る。
3. Where あるカテゴリが単一フラグメントで 600 行を超える場合, the フラグメント分割 shall 当該カテゴリを entry 境界で順序付きサブフラグメントへ分割する。
4. The フラグメント分割 shall 単一 entry（その全 field を含む）を複数フラグメントへまたがって分割しない。

### Requirement 3: 決定的・冪等な再構成

**Objective:** As a 下流 consumer の実装者, I want フラグメントから正準契約を決定的に再構成したい, so that どの環境でも一意な正準ビューが得られ契約のドリフトを防げる

#### Acceptance Criteria

1. The 再構成（merge）契約 shall フラグメント群の結合順を、明示マニフェストまたはファイル名の数値接頭辞によって決定的に固定する。
2. When フラグメント群を再構成する場合, the 再構成契約 shall 同一入力に対し常に同一の正準ビューを生成する（冪等）。
3. The 再構成契約 shall 再構成された正準ビューを、現行 `shiori_protocol.toml` の parse 結果と **意味的に同値**（無損失一致）とすることを受け入れ基準として規定する。
4. The 意味的同値の判定 shall entry 集合・各 entry の field 集合・共有テーブル・silence_ruling・全 description・全 provenance・封筒マッピング・予約ヘッダ集合の各データが、順序非依存で過不足なく一致することを条件とする。

### Requirement 4: TOML 符号化形の刷新と一意性のパーサ機械担保

**Objective:** As a 契約データの編集者, I want id と field 意味名の一意性をパーサ自身に強制させたい, so that 重複が機械検出され DRY な符号化で行数を圧縮できる

#### Acceptance Criteria

1. The 符号化形 shall 各 entry を id をキーとする連想テーブル（quote 付きキー）として表現し、entry id の一意性を TOML パーサに機械担保させる。
2. The 符号化形 shall 各 field を entry 配下の inline table（1 field = 1 行・snake_case の意味名をキー）として表現し、field 意味名の entry 内一意性を TOML パーサに機械担保させる。
3. The 符号化形 shall 各 silence_ruling を id をキーとする連想テーブルとして表現し、silence_ruling id の一意性を TOML パーサに機械担保させる。
4. The 符号化形 shall keyed table のキーを常に quote し、ドット・アスタリスクを含む id（例 `OnUpdate.OnDownloadBegin`・`char*.defaultx`）を破綻なく表現する。
5. While 符号化形を inline table で表現する間, the 符号化形 shall 各 field の `ReferenceN` 位置を `reference`（および可変長末尾は `reference_variadic`）キーで保持し、配列順序の消失が契約に影響しないことを保証する。
6. The 符号化形刷新 shall 完了仕様の `[mapping]` 記述データ（`canonical_key`/`alias_key` 等）を新しいテーブルキー表現へ更新し、その意味（値キーとテーブルキーの対応）を変えない。

### Requirement 5: 共有テーブルの集約

**Objective:** As a 契約の管理者, I want 横断参照される共有データを単一フラグメントへ集約したい, so that meta/封筒/予約ヘッダ/沈黙裁定が一意な中央位置に保たれる

#### Acceptance Criteria

1. The 共有フラグメント shall `[meta]`・`[mapping]`・`[envelope]`・`[reserved_headers]` の各共有テーブルを単一の共有フラグメントへ集約する。
2. The 共有フラグメント shall entry/field からカテゴリ横断で参照される全 `silence_ruling` を当該共有フラグメントへ集約する。
3. The 符号化形 shall entry/field の `silence_ref` による silence_ruling への文字列参照を、共有フラグメントへの集約後も解決可能なまま保持する。

### Requirement 6: 説明文と典拠の無損失保持

**Objective:** As a doc/Web 生成と典拠追跡の利用者, I want 全 description と provenance を分割後も失わせたくない, so that 派生レンダリングと互換進捗の可視性を保てる

#### Acceptance Criteria

1. The 分割後フラグメント群 shall 全 entry・全 field・全 silence_ruling・全共有テーブルの `description`（コメントでなくデータ）を無損失で保持する。
2. The 分割後フラグメント群 shall 全 entry・全 field・全 silence_ruling の provenance を無損失で保持する。
3. The 分割後フラグメント群 shall ukadoc ピン留めスナップショット（出典 URL・取得日・sha256 等の典拠資産）への参照整合を維持する。

### Requirement 7: 旧単一ファイルの派生レンダリングへの降格

**Objective:** As a 移行期の下流 consumer, I want `shiori_protocol.toml` を正本でなく派生（または廃止）として扱いたい, so that 正本がフラグメント群へ一本化されつつ移行期の互換が保てる

#### Acceptance Criteria

1. The `shiori_protocol.toml` shall 本仕様の完了後において契約の正本ではなく、フラグメント群が正本となる。
2. Where `shiori_protocol.toml` を非権威の生成物として tree に残置する場合, the `shiori_protocol.toml` shall フラグメント群からの再構成結果と意味的に同値であり、doc/Web と同格の派生レンダリングとして扱われる。
3. Where `shiori_protocol.toml` を非権威の生成物として残置する場合, the `shiori_protocol.toml` shall その先頭に「fragments から生成・直接編集禁止」である旨をデータとして明示する。
4. The 本仕様 shall `shiori_protocol.toml` を tree に残置するか削除してオンデマンド結合へ一本化するかの処遇（Q1）を、設計フェーズで裁定する未決の設計判断として明示する。

### Requirement 8: 完了仕様の SSOT 要件の改訂

**Objective:** As a 互換契約の管理者, I want 完了仕様 `areka-P0-shiori-protocol` の単一ファイル正本要件を論理 SSOT へ改訂したい, so that フラグメント分割が完了仕様の不変条件と整合する

#### Acceptance Criteria

1. The 本仕様 shall 完了仕様 `areka-P0-shiori-protocol` の要件3・要件11 における「単一ファイル正本（single source of truth＝1 枚の TOML）」を、「論理 SSOT＝フラグメント群およびその決定的結合結果」へ改訂する。
2. The 改訂 shall 二重定義禁止・全 description のデータ保持・provenance 維持・派生 doc/Web との同値/冪等という不変条件（精神）を維持する。
3. The 改訂 shall 完了仕様の `completed/` 配下の履歴を不変のまま残し、改訂が本仕様側で系譜（拡張改訂）として追跡可能であることを保証する。
4. The 本仕様 shall `doc/shiori/README.md` を、SSOT がフラグメント群であり `shiori_protocol.toml` が派生（または廃止）である旨へ改訂する。

### Requirement 9: 契約セマンティクスの非破壊保証

**Objective:** As a 下流 consumer と契約の裁定者, I want 物理編成・符号化形のみが変わり契約挙動が一切変わらないことを保証したい, so that 下流が符号化形の取り込みだけで済み再検証範囲を限定できる

#### Acceptance Criteria

1. The 本仕様 shall event/resource の追加・削除・改名を行わず、現行正本の entry 集合を不変に保つ。
2. The 本仕様 shall 各 field の意味名・型・必須/任意・`ReferenceN` 位置・応答意味・provenance を不変に保つ。
3. The 本仕様 shall 封筒マッピング・予約ヘッダ集合・沈黙裁定・バージョニング方針を不変に保つ。
4. If 意味的同値ゲートが現行正本と再構成結果の間に差分を検出する場合, then the 受け入れ検証 shall 本仕様の成果物を不合格とし、契約セマンティクスが変化したものとして扱う。
