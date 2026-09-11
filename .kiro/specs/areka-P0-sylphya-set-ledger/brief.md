# Brief: areka-P0-sylphya-set-ledger

> 起票: 2026-09-11（棚卸⑬・`areka-P0-property-query-channels` の分割 ⑶ を独立 spec に切り出し）。親 brief の「Scope In: sylphya 台帳の正典追随（`SET_EFFECTIVE` 21→26・サウンド語彙族の登記・件数檻の更新）」を丸ごと本 spec が引き継ぐ。親 spec はスクリプト経路（1〜4）に専念し、台帳には触れない。
> **三重所有の仮裁定（棚卸⑬）を本 spec が台帳側で履行する**: `currentghost.seriko.zorder`／`seriko.sticky-window` の SET 台帳行は本 spec が持ち、値の導出（parse／serialize）は `areka-P0-zorder-property` 単独、`areka-P0-currentghost-property-tree` は `seriko.*` から `zorder` を除外する。裁定の出典は roadmap の「棚卸⑬の仮裁定」節。

## Problem

sylphya の SET 有効一覧 `SET_EFFECTIVE`（`crates/areka-sylphya/src/vocab/dotted.rs`・件数檻 `assert_eq!(SET_EFFECTIVE.len(), 21)`）が ukadoc スナップショット比で **5 件古い**。`seriko.zorder`・`seriko.sticky-window`（SSP 2.8.78）とサウンド系 SET 3 葉（2.8.72）が未登記で、サウンド語彙 ≈18 葉は族ごと不在。`\![set,property]` の経路（親 spec）が着地しても、台帳が古いままでは正典どおりに書ける項目が 21 で止まる。

## Current State

2026-09-11 実測（サブエージェント再測定・全命中）:
- `SET_EFFECTIVE` は 21 項・`zorder`／`sticky-window` は grep 0 件・件数檻が 21 を固定。
- `property.get`／`property.set` の名前は予約済み（`dotted.rs` の予約表）。
- `GENERIC_PROP_NAMES` 17 種の件数檻も同ファイルに現存。
- ukadoc 調査の台帳 `doc/ukadoc-coverage/ledger/property.toml`（188 件・完了 spec `ukadoc-survey-property`）が SET 有効項目の正典 URL と版番号を持つ＝本 spec の照合元。同 spec のブリーフィング（`doc/ukadoc-coverage/briefing-property.md`「三重所有」節）は本仮裁定と同じ案（案 甲）を推奨している。

## Desired Outcome

`SET_EFFECTIVE` が ukadoc の SET 有効項目と一致し（21→26・照合元は `ledger/property.toml`）、サウンド語彙族が語彙表に載り、件数檻が新しい数を固定している。実行時挙動は不変（値の導出は行わない＝`NotFound`／`NotSettable` 縮退は既定のまま）。

## Approach

台帳の追随のみ。`dotted.rs` の配列と件数檻を更新し、各項目の定義行に `/// ukadoc: <url>` を 1 行置く（`ukadoc-survey-toolkit` 規則⑴）。`doc/ukadoc-coverage/ledger/property.toml` の該当行の `status`／`owner` を本 spec で埋める（台帳は `doc/` 配下の生きた文書・完了 spec の成果物だが編集可）。

## Scope

- **In**: `SET_EFFECTIVE` 21→26／サウンド語彙族の登記（≈18 葉・語彙のみ）／件数檻の更新／`ledger/property.toml` の `owner` 記入／COMPAT §8 の `currentghost.seriko.zorder` 行へ「台帳行は本 spec・値は `zorder-property`」の相互参照 1 行。
- **Out**: 値の導出（`zorder-property`・`currentghost-property-tree`・`property-catalog-lists`）／照会経路（親 spec）／`.ext.*` 逆方向イベント（`property-ipc-transport`）。

## Boundary Candidates

- 単一スライス（S）。分割しない。

## Out of Boundary

- `dotted.rs` の M1 縮退宣言文の改訂（`currentghost-property-tree` が行う）。

## Upstream / Downstream

- **Upstream**: 完了 `ukadoc-survey-property`（照合元台帳）・完了 `scope-zorder-pinning`（`zorder` の値源）。前提 spec なし＝**即着手可**。
- **Downstream**: `property-query-channels`（`\![set,property]` が本台帳を読む）・`zorder-property`・`currentghost-property-tree`・`property-catalog-lists`（いずれも `dotted.rs` を後で触る＝本 spec が先に着地して rebase 源を消す）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-property-query-channels`（分割元・⑶ を引き継ぐ）。
- **Adjacent**: `zorder-property`（値の導出）・`currentghost-property-tree`（`seriko.*` 一括から `zorder` を除外）。

## Constraints

- 編集集合は `crates/areka-sylphya/src/vocab/dotted.rs`（＋兄弟テスト）・`doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8 のみ。W13 の他 spec と共有ファイル 0（実測・2026-09-11）。
- 決定論テスト: 件数檻＋各新規項目の名前解決（`NotSettable` にならないこと）。
- 規模 S・要件定義は Opus で足りる。
