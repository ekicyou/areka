# 設計検証レポート: areka-P0-shiori-protocol

> 検証プロセス: `kiro-validate-design`（design-review.md の Analysis → Critical Issues → Strengths → GO/NO-GO）。
> 非対話実行（NON-INTERACTIVE）。本レポートは設計ディスカッション（`/kiro-design-discussion`）の入力。
> 入力: spec.json（language=ja, phase=design-generated）／requirements.md（11 要件）／design.md／research.md（§8 設計追補）／steering（tech.md・structure.md ほか）。
> スコープ前提: 本仕様は SHIORI **content 契約**（単一 TOML 正本 `doc/shiori/shiori_protocol.toml` ＋生成 doc/Web）を定義する。`IShiori` COM ABI（`areka-P0-shiori-com` 完了・content は不透明 HSTRING）には触れない。Rust codegen・投影/パーサ実装は明示的にスコープ外であり、本検証はそれらの欠落を欠陥として扱わない（要件の実スコープ＝WHAT/契約に対して判定）。

## 設計レビュー サマリー

本設計は「単一 TOML 正本＋派生 doc/Web」という明快なパターンを採り、全 11 要件を Requirements Traceability 表で TOML の table/キーまたは封筒・doc 生成方針へ一意に対応づけている。境界規律（ABI 不変、codegen・投影機構・翻訳の下流分離）が一貫し、steering（最小依存・32bit 可搬性・既存 `toml` 採用）とも整合する。実装可能性は高く、残る論点は **契約スキーマが一部要件（charset 対応・複数沈黙裁定・予約集合の確定性）を完全に表現しきれているか** という細部のスキーマ充足性に限られる。

## Critical Issues（最大 3）

### 🔴 Critical Issue 1: charset 規約が単一スカラーで `Charset` ヘッダ対応を表現しきれない懸念
- **Concern**: R8.2 は「content 文字列のエンコーディング/charset 規約を定義し、`Charset` ヘッダ等の文字集合情報との対応を規定する」ことを要求するが、設計は `[meta].charset`（`str` 1 個）と `description` への対応規則記載に留める（Data Models §`[meta]`）。レガシー SHIORI/3.0 ではリクエスト単位に `Charset` が変動しうるため、グローバル 1 スカラー＋散文では「対応規則」が契約データとして検証可能な形に符号化されない恐れがある。
- **Impact**: host-32 がレガシー wire を byte 化する際に依拠すべき「内部表現（HSTRING/UTF-16）⇔ wire charset」の対応が散文依存になり、契約の機械可読性（R11-1/-4）と再現性が弱まる。
- **Suggestion**: `[meta]` に既定 charset と「`Charset` ヘッダ ⇔ 内部表現の対応規則」を構造化キー（例: `charset_default` ＋ `charset_header_policy`）として明示するか、対応規則を `[[silence_ruling]]`/専用キーで data 化する旨を 1 段落で確約する。
- **Traceability**: 8.2（一部 8.1/8.3）
- **Evidence**: design.md §Data Models 「`[meta]` — 契約メタ…」`charset` 行／Requirements Traceability 8.2。

### 🔴 Critical Issue 2: `entry.silence_ref` がスカラーのため 1 エントリ複数裁定を表現できない
- **Concern**: 設計は `entry.silence_ref` を任意の `str` 1 個（`[[silence_ruling]]` の `id` 参照）とする（Data Models §`[[entry]]`）。しかし research §8.2 が示すとおり、同一イベントが「GET/NOTIFY 分類（dispatch_class）」と「意味割り当て（meaning_assignment）」など複数トピックで同時に沈黙裁定対象になりうる。スカラー参照ではエントリ単位で 1 裁定しか結べない。
- **Impact**: R7.1（沈黙する裁定を対応表へ記録）の網羅が構造的に制約され、複数裁定が絡むエントリで追跡漏れ（反証不能化）が生じうる。COMPAT §2 の「進捗を可視・反証可能に」という方針に対する穴。
- **Suggestion**: `silence_ref` を `array of str`（複数 `id` 参照可）とするか、`field` レベルにも `silence_ref` を許す旨を明記して、エントリ/フィールド双方が複数裁定へ紐づけられるようにする。
- **Traceability**: 7.1, 7.2（1.3 の沈黙裁定経路にも波及）
- **Evidence**: design.md §Data Models 「`[[entry]]`」`silence_ref` 行／research.md §8.2（`[NOTIFY/他GET]` 等の文脈依存）。

### 🔴 Critical Issue 3: 予約ヘッダ集合が「例」止まりで、非衝突保証の判定基盤が確定していない
- **Concern**: R6.1 は意味名が予約 SHIORI ヘッダと「衝突しないことを保証する」を要求し、Testing Strategy も「全 `entry.field.name` が `[reserved_headers]` と衝突しない」検証を契約とする。だが設計の `[reserved_headers].request/response` は値が `例 ["ID","Sender",…]` と例示に留まる（Data Models §`[reserved_headers]`）。衝突検証の母集合（＝確定した予約集合）が未確定だと、保証の判定そのものが空回りしうる。
- **Impact**: R6.1 の「保証」が検証可能性を欠き、R6.2 の是正（`collision_policy`）も判定基準が定まらない。host-32/reference が依拠する非衝突契約の信頼性に直結。
- **Suggestion**: 予約集合の「確定」は本仕様所有の必達物である旨を Boundary Commitments で言明し（典拠＝ukadoc SHIORI/3.0 ＋沈黙裁定）、カタログ全列挙（下流）と異なり予約集合は実装作業に委譲しない、と明記する。あるいは委譲する場合は委譲先と確定責任を Requirements Traceability 6.1 に追記する。
- **Traceability**: 6.1, 6.2
- **Evidence**: design.md §Data Models 「`[reserved_headers]`」`request`/`response` 行／§Testing Strategy「予約非衝突」。

## 設計の強み（Strengths）

1. **「field 行 1 枚＝R3 の単一スキーマ→2 投影」を構造で担保**: 意味名（canonical）と `ReferenceN`（alias）を別テーブルへ二重化せず `[[entry.field]]` 1 行に集約することで、R3.2/10.2 のドリフトを設計レベルで原理的に抑止している。canonical 優先規則（3.4）も「正本が field 行 1 枚」という構造から自然に導かれており、要件と設計の対応が緊密。
2. **境界規律と steering 整合が一貫**: ABI 不変・封筒は既存 `areka-P0-shiori-com` 意味論（`Request`/`Complete`/`Raise`・`CorrelationToken`・`SHIORI_S_PENDING`）への「被せ」に徹し（research §1.2 表を design に再掲）、成果物を静的データ（TOML）に限定して最小依存・32bit 可搬性（tech.md/structure.md）を崩さない。codegen・投影機構・翻訳・生成器を明示的に下流へ分離し、WHAT/契約スコープを逸脱していない。

## 最終判定（Final Assessment）

### Decision: **GO**

### Rationale
全 11 要件が TOML 正本の table/キーまたは封筒・doc 生成方針へ一意トレースされ、既存 ABI 意味論・steering との整合が取れ、実装（下流 codegen/投影は対象外）への明快な経路がある。指摘 3 件はいずれもスキーマ表現の精緻化（charset 対応の構造化・複数沈黙裁定の許容・予約集合確定責任の明文化）であり、契約の骨格を覆す致命的欠陥ではない。設計ディスカッションで調整可能なレベルのため GO 相当。

### Next Steps
1. 設計ディスカッション（`/kiro-design-discussion areka-P0-shiori-protocol`）で Critical Issue 1〜3 を解消（charset 対応の符号化・`silence_ref` の複数許容・予約集合の確定責任）。
2. 合意後、`/kiro-spec-tasks areka-P0-shiori-protocol` でタスク生成へ進む。
