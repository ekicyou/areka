# Brief: areka-P0-shiori-protocol-split

> 拡張元（完了）: `areka-P0-shiori-protocol`（`doc/shiori/shiori_protocol.toml` を単一正本 SSOT として確定。要件3・11 が「契約定義を 1 ファイルへ集約・他ファイルへ分散禁止」を中核不変条件とする）。
> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。契約典拠: ukadoc ピン留めスナップショット（`.kiro/specs/areka-P0-shiori-protocol/ukadoc/`）。
> 本仕様は **契約内容そのものを一切変更しない**。物理ソースの編成と TOML 符号化形のみを再編する（挙動・契約は非破壊）。

## Problem
`doc/shiori/shiori_protocol.toml` が **10,685 行の単一ファイル**へ肥大化し、保守・レビュー・diff が困難。1 イベントの追加・修正でも巨大ファイル全体を読み込む必要があり、LLM 支援・人手レビューの双方でコンテキスト効率が悪い。一方、拡張元の完了仕様 `areka-P0-shiori-protocol` は「契約の正本は `shiori_protocol.toml` ただ 1 枚／契約定義を他ファイルへ分散させてはならない（要件3・要件11）」を中核不変条件として置いており、**素朴に物理分割すると当該不変条件に literal に抵触**する。SSOT 保証（単一権威・二重定義禁止・全 `description` のデータ保持・provenance 維持・派生 doc/Web との同値/冪等）を壊さずに物理ソースのみを分割する設計が要る。

## Current State
- 正本 `doc/shiori/shiori_protocol.toml`：実測 **10,685 行**。内訳は `[[entry]]` **446**（`kind=event` **287** ＋ `kind=resource` **159**）、`[[entry.field]]` **802**（`reference` 付き 796・`reference_variadic` 28・`reference` 無し 6）、`[[silence_ruling]]` **9**、共有テーブル `[meta]`/`[mapping]`/`[envelope]`/`[reserved_headers]` 各 1。
- entry は既に **36 カテゴリ順**（`lifecycle`→`time`→…→`shortcut_key`→`tooltip`）で整列済み。カテゴリ別件数は偏りがあり最大は `shortcut_key` 93・`ghost_info` 40。
- `description` 行長は中央値 76・p90 146 文字。突出した長文は共有テーブル 4 件（1,132〜1,601 文字）で、これらは field ではない。
- 現行 id に重複は無い。resource id にはドット・アスタリスク混じり（`OnUpdate.OnDownloadBegin`／`property.get`／`sakura.defaultx`／`char*.defaultx` 等）が実在する。
- `silence_ref` は entry/field から `[[silence_ruling]]` を横断参照する（OS 状態系 8 件を 1 裁定が束ねる等、カテゴリ横断）。
- doc/Web 生成器・Rust codegen は未実装（後続フェーズ）。現状 `shiori_protocol.toml` を読む実装上の consumer はほぼ存在しない。

## Desired Outcome
`shiori_protocol.toml` を **Event/Resource 単位で keyed table 化したフラグメント群**へ再編し、その **フラグメント群そのものが契約の正本（SSOT）** となる。正準契約は分割フラグメントから **決定的に再構成（merge）** され、再構成結果が現行 `shiori_protocol.toml` と **意味的に同値**であることを証明できる。契約内容（どの event/resource が・どの field を・どの `ReferenceN` 位置で・どの型/必須/応答意味/provenance/description を持つか、封筒マッピング、予約ヘッダ集合、沈黙裁定、バージョニング方針）は **一切変更しない**。各フラグメントは LLM/人手レビューに優しいサイズに収まる。

## Approach
**単一正本データの物理分割＋決定的再構成**。完了仕様の「論理 SSOT＝単一権威・二重定義禁止」の精神を維持したまま、物理レイアウトと符号化形のみを刷新する。

### A. 分割粒度（サイズ駆動）
- **1 フラグメント ≤ 600 行**（LLM が破綻し始める閾値）を不変条件とする。
- 継ぎ目は **カテゴリ純度**を基本（diff・捜索が意味単位になる）。
- 600 行を超えるカテゴリ（`shortcut_key`・`ghost_info` 等）は **entry 境界で順序付きサブ分割**（例 `shortcut_key.01.toml` / `shortcut_key.02.toml`）。entry 途中では割らない。
- event と resource は同形（`kind` 判別子付き）ゆえ同一フラグメント形式に収める（必要なら `events/`・`resources/` のディレクトリ分離は設計フェーズで判断）。

### B. SSOT の所在の転換
- **フラグメント群＝物理かつ論理の SSOT（権威）。** `shiori_protocol.toml` は **もはや正本ではない**。
- `shiori_protocol.toml` は doc/Web と同格の **派生レンダリング（generated・非権威）** へ降格、または廃止。**暫定推奨＝非権威の生成物として残置**（先頭に「fragments から生成・編集禁止」banner をデータで明記）し、移行時に下流を一斉改修せずに済ませる。将来 consumer を fragments 直読みへ寄せた後に廃止可（要件フェーズで裁定＝下記 Q1）。
- 二重定義禁止は「各契約データが唯一のフラグメントにのみ存在する」ことで担保。後述の keyed table 化により **パーサ自身が重複を機械検出**する。

### C. TOML 符号化形の刷新（keyed / inline）
- **entry**：`[[entry]]` 配列 → `[entry."OnFirstBoot"]` **連想配列**（id をキー化、quote 必須）。`id =` 行が消え DRY、id 一意をパーサが強制。
- **field**：`[[entry.field]]` 配列 → `[entry."OnFirstBoot".field]` 下の **inline table（1 field = 1 行、snake_case の意味名をキー）**。例: `shell_name = { reference = 0, type = "str", required = false, provenance = "ukadoc", description = "…" }`。`name =` 行が消え、field 名の event 内一意をパーサが強制（要件2.3 を機械担保）。`reference N` が位置を担うため順序消失は無害。
- **silence_ruling**：`[[silence_ruling]]` → `[silence_ruling."sr_…"]` 連想配列で一貫（id 一意担保）。`silence_ref` の文字列参照はそのまま機能。
- **共有テーブル**：`[meta]`/`[mapping]`/`[envelope]`/`[reserved_headers]` は通常テーブルのまま `_shared.toml` へ集約（横断参照される `[silence_ruling.*]` も中央集約）。
- **行数効果**：field 1 件が約 7 行 → 1 行へ圧縮（802 field で約 6,000 行削減）。全体は 10,685 → 推定 3,000 行台へ。600 行フラグメントに詰める entry が増え分割数を抑制。
- **これは符号化スキーマ形状の変更**（design.md DP1 の `array of entry` 規定の改訂・Revalidation Trigger 該当）であり、本仕様の明示的設計判断として畳み込む。`[mapping]` の `canonical_key`/`alias_key` 記述データは「値キー → テーブルキー」へ更新（意味は不変）。

### D. 決定的再構成と非破壊ゲート
- フラグメントの結合順を **明示マニフェスト**（`_manifest.toml`）または **ファイル名 NN. 数値接頭辞**で決定的に固定（冪等）。
- 受け入れゲート＝`parse(現行 shiori_protocol.toml)` と `parse(merge(fragments))` を **データ構造として同値**と検証（entry/field/共有テーブル/silence_ruling/全 description/全 provenance が無損失一致）。これが「契約を一切変えていない」非破壊の証拠。keyed 化により順序非依存の map 等価比較が可能。
- 派生 doc/Web 生成・Rust codegen は引き続きスコープ外（後続）。入力が `shiori_protocol.toml` 単一ファイルから「fragments（または再構成された正準ビュー）」へ移ることのみ宣言する。

### E. 完了仕様の要件改訂（拡張）
- 本仕様は完了済み `areka-P0-shiori-protocol` の要件3・11 を **「論理 SSOT＝フラグメント群（およびその決定的結合結果）／二重定義禁止は維持／全 description データ保持・provenance 維持・派生同値は不変」と上書き継承**する。`completed/` の履歴は不変のまま系譜を残す。

## 要件の種（Requirement Seeds → 要件フェーズで EARS 化）
> 以下は WHAT。`/kiro-start` の要件生成で requirements.md（EARS）へ昇格させる。

- **R-種1 物理分割の正本化**: 契約の正本は Event/Resource 単位のフラグメント群とし、各契約データは唯一のフラグメントにのみ存在する（二重定義禁止の維持）。
- **R-種2 サイズ不変条件**: 各フラグメントは ≤600 行。カテゴリ純度を基本とし、超過カテゴリは entry 境界で順序付きサブ分割する。
- **R-種3 決定的再構成**: フラグメントから正準契約を決定的・冪等に再構成でき、結果は現行正本と意味的に同値（無損失）。
- **R-種4 符号化形**: entry/silence_ruling は id キーの連想配列、field は意味名キーの inline table。id/意味名の一意をパーサが機械担保する。
- **R-種5 共有テーブル集約**: `[meta]`/`[mapping]`/`[envelope]`/`[reserved_headers]` と横断参照される `[silence_ruling.*]` を単一の共有フラグメントへ集約する。
- **R-種6 典拠・説明の無損失**: 全 `description`（データ）と provenance を分割後も無損失で保持する。
- **R-種7 派生の地位**: `shiori_protocol.toml` は正本ではなく派生レンダリング（または廃止）。doc/Web と同格の generated 物として正本との同値を保つ。
- **R-種8 完了仕様の改訂**: `areka-P0-shiori-protocol` 要件3・11 の「単一ファイル正本」を「論理 SSOT＝フラグメント結合結果」へ改訂する（精神は維持）。

### 未決の要件ノブ（要件フェーズで決定）
- **Q1 `shiori_protocol.toml` の処遇**: 非権威の生成物として残置（暫定推奨・下流無改修）か、tree から削除しオンデマンド結合に一本化するか。**契約面の WHAT**として裁定する。

## Scope
- **In**:
  - `shiori_protocol.toml` の Event/Resource 単位フラグメントへの物理分割（≤600 行・カテゴリ純度・超過時サブ分割）
  - TOML 符号化形の刷新（entry/silence_ruling の連想配列化、field の inline table 化、共有テーブルの `_shared.toml` 集約）
  - フラグメント → 正準契約の決定的再構成機構の **契約・受け入れ基準**（マニフェスト順序・同値ゲート）
  - 現行正本との意味的同値の証明（無損失・冪等）と provenance/description の保持
  - 完了仕様 `areka-P0-shiori-protocol` 要件3・11 の改訂（論理 SSOT 再定義）
  - `doc/shiori/README.md` の改訂（SSOT＝fragments の宣言・派生の地位）
- **Out**:
  - 契約内容そのものの変更（event/resource の追加削除・field 意味/型/`ReferenceN` 位置・封筒マッピング・予約ヘッダ集合・沈黙裁定・バージョニング方針はすべて不変）
  - 再構成機構・バリデータ・doc/Web 生成器・Rust codegen の **実装コード**（HOW・下流／後続フェーズ）
  - COM ABI（`IShiori`/`IShioriHost`）・トランスポート・さくらスクリプト/SAORI 解釈（隣接仕様）

## Boundary Candidates
- 分割レイアウト（ディレクトリ構成・フラグメント命名・サイズ規律・サブ分割規則）
- TOML 符号化スキーマ（keyed entry／inline field／keyed silence_ruling／共有テーブル）と `[mapping]` 記述更新
- 決定的再構成の契約（マニフェスト順序・冪等性・同値ゲートの判定基準）
- 完了仕様の要件改訂と README/典拠参照の整合

## Out of Boundary
- 契約セマンティクスの一切（非破壊が大前提）
- 再構成・検証・生成・codegen の実装コード（後続・下流）
- ABI 面・トランスポート・content 解釈（隣接仕様の領分）

## Upstream / Downstream
- **Upstream**: `areka-P0-shiori-protocol`（完了・本仕様が拡張改訂する正本所有者）／ukadoc ピン留めスナップショット（典拠・読み取りのみ）
- **Downstream**:
  - 後続の doc/Web 生成器・Rust codegen（入力が単一ファイル→fragments/正準ビューへ移行）
  - `areka-P0-shiori-host-32`・`areka-P0-shiori-reference`・pasta native 脳（正準契約の consumer。契約セマンティクス不変ゆえ影響は符号化形のみ・D7 lockstep）

## Existing Spec Touchpoints
- **Extends**: `areka-P0-shiori-protocol`（completed。要件3・11 を改訂し論理 SSOT を再定義。`shiori_protocol.toml`・`README.md`・design DP1 を更新対象とする）
- **Adjacent**: `areka-P0-shiori-host-32`・`areka-P0-shiori-reference`（正準契約の消費者。符号化形変更を lockstep で取り込む）

## Constraints
- **契約非破壊が絶対**: 物理編成・符号化形のみ変更し、契約内容と挙動を一切変えない。意味的同値ゲートで担保する。
- **SSOT 精神の維持**: 単一権威・二重定義禁止・全 description のデータ保持・provenance 維持・派生 doc/Web との同値/冪等を、物理分割後も保証する（keyed 化でパーサが一意を機械担保）。
- **TOML v1.0.0 構文**: inline table は単一行限定（design の技術選定と整合）。長文 description を持つ少数 field は 1 行が長くなるが許容（形式統一を優先）。
- **キー quote 必須**: ドット/アスタリスク混じり id（`char*.defaultx` 等）に対応するため keyed table のキーは常に quote する。
- **典拠**: ukadoc を互換契約の正典とし、provenance と `[silence_ruling.*]` を無損失で引き継ぐ。
