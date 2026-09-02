# Brief: areka-P0-ukadoc-survey-property

> 起票: 2026-09-02（`/kiro-discovery` Path D・ukadoc 網羅調査 6 本の 5 本目）。同日の開発者追記で当初の `ukadoc-survey-script-property` を分割し、さくらスクリプトを `ukadoc-survey-sakura-script` へ、プロパティを本 spec へ分けた（並行実施のため）。
> **種別**: 調査 spec（台帳＋ブリーフィング節・実行時コード非接触）。`ukadoc-survey-toolkit` が凍結した台帳形式で `doc/ukadoc-coverage/ledger/property.toml` を書く。
> **所有範囲＝「プロパティシステムの木」**: list_propertysystem **188 項目**（版番号付き 98＝全ページ中で世代差が最も濃い）。
> **本ドメインは既に 3 本の M2 ゲート brief（2026-08-27・`property-query-channels`／`currentghost-property-tree`／`property-catalog-lists`）＋`zorder-property` が所有宣言している。本 spec の主務は「新規の数え直し」ではなく、正典 id 単位で所有宣言を突合し、無所有・二重所有・新旧書式を仕訳すること。**

## Problem

2026-08-27 の手作業サーベイ（≈180 項目）は brief ごとに数え方が違い、正典 id との対応が機械で追えない。スナップショット実測は 188＝差分 8 前後の正体が不明。版番号付きが 98 項目もある＝同じ値へ到達する名前が世代ごとに増えており（例: `currentghost.balloon.scope(ID).*` と旧 `balloon.*` 系・`system.monitor.index(ID).*` の追加葉）、**最新仕様を優先し旧名をエイリアスとして仕訳**しないと 3 本の brief が同じ値を別名で二重実装しかねない。また `zorder-property` ⇄ `currentghost-property-tree` の二重所有裁定が未決（記憶 scope-zorder-pinning 残件）。

## Current State

### ukadoc 側（2026-09-02 実測）

- 188 項目・版番号付き 98。2026-08-27 サーベイの内訳（`currentghost` ≈65〔balloon.scope 族 19・scope 幾何 17・seriko 14・mousecursor 6 他〕・`system` 25・カタログ/履歴 8 根・汎用 17 葉・サウンド語彙）と概ね一致。
- 照会経路（`\![get,property,...]`・`\![set,property,...]`・`%property[...]`・SSTP・`property.get`/`property.set` イベント）は sakura／shiori 台帳側の id を `links` で参照する（本 spec は木の側）。

### areka 側（2026-09-02 実測・file:line は着手時に再検証すること）

- sylphya `vocab/`（981 行）＝状態付き語彙台帳: `FLAT_VOCAB` 26（実導出 4＝`username`／`selfname`／`selfname2`／`keroname`・`flat.rs:32`）・`DOTTED_ROOTS` 10（`dotted.rs:21`）・`GENERIC_PROP_NAMES` 17（`dotted.rs:44`）・`SET_EFFECTIVE` 21（`dotted.rs:80`）・`EXT_EVENT_GET/SET` 2（予約のみ・`dotted.rs:107,110`）。状態型 `M1Status::{Derived, Degraded}`／`DegradePolicy::{PassThroughRaw, ConsumerDefault, NotFound}`（`vocab/mod.rs:11-55`）。**実導出は `baseware.*` のみ・他の根枝は NotFound 縮退**（`dotted.rs:4-6`）。件数固定テスト `ledger_key_determinism_tests.rs:201-204`。
- 所有宣言（転記元）: `property-query-channels`（照会経路）／`currentghost-property-tree`（`currentghost.*` ≈65）／`property-catalog-lists`（`system.*` 25・カタログ 5 根・`history`・`rateofuselist`・`currentghost.sound.*`・`.ext.*`）／`zorder-property`（`currentghost.seriko.zorder`）／`balloon-vertical-canon` 完了時の縮退登記（`.vertical`）。

## Desired Outcome

- 188 項目すべてに status・根拠・担当 spec・世代・優先度が付き、`unclassified` 0・**二重所有 0・無所有 0**。
- 新旧名の仕訳: 同じ値へ到達する名前群ごとに正典 1 つと `alias` 群（`alias_of`）。sylphya の語彙表へ「alias は正典へ写像して 1 か所で導出」という方針が台帳から引ける。
- `zorder-property` ⇄ `currentghost-property-tree` の二重所有について、台帳の id 単位で裁定案（どちらが `currentghost.seriko.zorder` を持つか）を出す。
- 関連の検索: 各葉の値の源（descript キー・OS メトリクス・イベント）と、SET 有効 21 葉の書込先が `links` に登記される。
- ブリーフィング節（`doc/ukadoc-coverage/briefing-property.md`）: 3 本の brief の所有範囲を id 一覧へ書き換えるための是正候補と、無所有項目の優先度。

## Approach

- 3 本の brief＋zorder-property の項目名→catalog id 対応表を先に作る（対応が付かない名前は表記揺れとして記録）。
- toolkit の evidence スキャン（sylphya 語彙表の件数固定テストと `M1Status`）で status 候補を機械転記し、人手で確定。
- 仕訳規則は toolkit 凍結のものを適用（最新優先・新名正典・旧名 alias・版番号＝世代・版番号なしは世代不明）。
- 実装済みの証拠は toolkit の規則 7（ソースの ukadoc URL）＝sylphya の語彙表は**表の頭にページ URL 1 つ**（`list_propertysystem.html`）を置き、個々の名前は catalog の title で対応付ける。実行時挙動を変えない doc コメントのみ・本 spec の唯一のコード接触。
- **着手条件**: toolkit の要件確定後・実装完了を待たない。他 survey と並走。

## Scope

- **In**: 188 項目の台帳・所有突合表・alias 仕訳・二重所有裁定案・関連登記・ブリーフィング節。
- **Out**: 実装・照会経路のタグ／イベント側の台帳（sakura／shiori spec）・既存 brief の書き換え。

## Boundary Candidates

- 「照会経路／木」の既存分割線に従う（brief 3 本の境界は再定義しない・裁定案を出すだけ）。

## Out of Boundary

- 既存 M2 ゲート brief の優先順位変更（`ukadoc-coverage-roadmap`）。

## Upstream / Downstream

- **Upstream**: `ukadoc-survey-toolkit`（台帳形式・仕訳規則の凍結＝**要件確定後に着手可**）。既存 brief 4 本・sylphya 語彙表。
- **Downstream**: `ukadoc-coverage-roadmap`。既存 brief 4 本（是正候補の受け手）。

## Existing Spec Touchpoints

- **Extends**: なし（brief は書き換えない）。
- **Adjacent**: `ukadoc-survey-sakura-script`（`%property[...]`・`\![get/set,property]` はタグとして向こう・木として本 spec＝二重計上しない）／`ukadoc-survey-shiori`（`property.get`/`property.set` イベント）／`emo2-conformance-e2e`（W12・共有ファイル 0）。

## Constraints

- 台帳は `doc/ukadoc-coverage/ledger/property.toml` 1 ファイルのみ。
- 件数の食い違いは catalog（id）を正とし brief 側の差を記録する。
- 「所有者なし」は憶測で既存 brief に押し込まない。
