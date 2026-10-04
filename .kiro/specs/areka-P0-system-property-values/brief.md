# Brief: areka-P0-system-property-values

> 2026-10-04 棚卸㉑で `areka-P0-property-catalog-lists` から切り出し（開発者「負荷が高すぎる仕様は分割を検討せよ」）。元の spec は一覧（カタログ）の枝を持ち、本 spec は `system.*` を持つ。

## Problem

ゴーストの台本はプロパティ `system.*` で時刻・モニタ・カーソルの位置・OS・メモリ・CPU・電源・ディスク・テーマなどを照会し、環境に合わせた台詞（メモリ残量トーク・モニタの寸法の参照）を出す。areka の sylphya にはこれらの値が無く、値なし（NOT_FOUND）になる。

## Current State

- 網羅台帳 `doc/ukadoc-coverage/ledger/property.toml` の `system.*` の行は今 `property-catalog-lists` が持ち主（着地時に本 spec へ直す）。
- モニタの値は `crates/wintf/src/ecs/window/monitor.rs` の `Monitor`（`bounds`・`work_area`・`dpi`・`is_primary`）に在る。`bpp` だけ源が無い。
- 時計・カーソル・OS・メモリ・CPU・電源・ディスク・テーマ・ドラッグの様子は、Win32 から採る層がまだ無い（M2 の基盤に依存はしない）。
- 動く値を出す口は sylphya に無い（`SylphyaPublisher` は静的・SHIORI 照会・永続だけ・`BackingLayer::SystemEnv` は縮退のまま）。`currentghost-property-tree` が作る（その spec を口の持ち主に決めた）。時計は毎秒変わる＝口が「読む時に問い合わせる」形でないと出し直しが追いつかない。

## Desired Outcome

`system.*`（約 25 項目）が実際の値で読める。採る層は偽の値を差せる形で、決定論のテストで固定される。取れない値（`bpp` など）は値なしのまま（捏造しない）。

## Approach

Win32 の採り口を 1 か所に集め（偽の値を差せる継ぎ目つき）、`currentghost-property-tree` の動く値の口へつなぐ。モニタは wintf の値を読む。

## Scope

- **In**: `system.*` の値の導出・Win32 の採り口と偽の値の継ぎ目・決定論のテスト・網羅台帳の該当行・COMPAT §8。
- **Out**: 一覧の枝（`property-catalog-lists`）・`currentghost.*`（`currentghost-property-tree`・`currentghost-property-others`）・照会の経路（`property-query-channels`）・SSTP や FMO 経由の外からの照会。

## Boundary Candidates

- モニタ（wintf の値）／時計とカーソル／OS・メモリ・CPU・電源・ディスク・テーマ（Win32 の採り口）。

## Out of Boundary

- DPI 追従そのもの・モニタの列挙の仕組み（wintf）。

## Upstream / Downstream

- **Upstream**: `currentghost-property-tree`（動く値の口）・`property-query-channels`（照会の end-to-end）・DPI 追従の基盤（モニタの値）。
- **Downstream**: 環境に合わせた演出をする既存ゴーストの互換。

## Existing Spec Touchpoints

- **Extends**: `property-catalog-lists`（分割元）。
- **Adjacent**: `currentghost-property-tree`・`currentghost-property-others`・`property-catalog-lists`（同じ sylphya の語彙）。

## Constraints

- 正典はライブの ukadoc で、設計の前に突き合わせる。値の捏造禁止・決定論のテスト必達（OS の値は偽の値を差して固定する）。
- sylphya の語彙（`vocab/dotted.rs`）と `emo2_boot/mod.rs` を触る spec と同時に走らせない。

## 2026-10-04 棚卸㉑で切り出し

- 元の spec: `property-catalog-lists`（⒜）。
- 規模: M〜L（14〜18 タスク）。
- 前提: `currentghost-property-tree`（動く値の口・未）。
- 触るファイル: 新規の Win32 の採り口（例 `crates/areka/src/property/system_env.rs`）・`crates/wintf/src/ecs/window/monitor.rs`（読むだけ）・`crates/areka-sylphya/src/{vocab/dotted.rs, key.rs}`・`crates/areka-ghost/src/sylphya_wiring.rs`・`crates/areka/src/emo2_boot/mod.rs`・`doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 共有しうる相手: `currentghost-property-tree`・`currentghost-property-others`・`property-catalog-lists`・`mcp-get-property`。
- 議題: `system.cpu.*`・`memory.*` などの読みを毎回 Win32 へ問い合わせるか、間隔を置いて採るか（動く値の口の形しだい）。
