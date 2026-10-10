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

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M〜L（14〜18）のまま。切る: なし（一度切り出した spec）。
- 前提の状態: `currentghost-property-tree`（動く値の口）はまだ＝着手できない。読む道は `mcp-get-property`（✅ 10-04）でできた＝着地後は MCP の `get_property` で `system.monitor.count` などを実機で確かめられる。
- 崩れた前提／古くなった位置: なし。`crates/wintf/src/ecs/window/monitor.rs`（424）は C3 で変わっていない（`Monitor` の `bounds`・`work_area`・`dpi`・`is_primary`）。sylphya の `BackingLayer::SystemEnv` は縮退のまま。`system.dnd.mode` は投げ込み（ファイルのドラッグ）の様子で、`mouse-drag-events` の窓のドラッグとは別物。
- 触るファイル: 新規の Win32 の採り口（例 `crates/areka/src/property/system_env.rs`）・`crates/wintf/src/ecs/window/monitor.rs`（読むだけ）・`crates/areka-sylphya/src/{vocab/dotted.rs, key.rs}`・`crates/areka-ghost/src/sylphya_wiring.rs`・`crates/areka/src/emo2_boot/mod.rs`・`doc/ukadoc-coverage/ledger/property.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 議題（答えで作業が変わるものだけ）: 前回の 1 つ（毎回問い合わせるか間隔を置くか・動く値の口の形しだい）。
- 見つけた穴: なし。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: `currentghost-property-tree`（動く値の口）は未着手のまま＝着手できない。C4 は値の源に触れていない＝モニタの値 `crates/wintf/src/ecs/window/monitor.rs` は `f26aa1c1` からの変更 0（`wintf-tooltip` は wintf の別の所）・sylphya も変更 0（`BackingLayer::SystemEnv` は縮退のまま）。網羅台帳 `property.toml` の `system.*` は 25 行で、持ち主は今も `property-catalog-lists`（着地のときに本 spec へ直す）。
- **触るファイル**: 前回のまま＝新規の Win32 の採り口（例 `crates/areka/src/property/system_env.rs`）・`monitor.rs`（読むだけ）・`crates/areka-sylphya/src/{vocab/dotted.rs, key.rs}`（407・411）・`crates/areka-ghost/src/sylphya_wiring.rs`（415）・`crates/areka/src/emo2_boot/mod.rs`（**912**＝足すのは呼び出しの数行）・台帳 `property.toml`（25 行）・`doc/COMPAT_ARCHITECTURE.md` §8。**足す 1 本**: 根の `Cargo.toml`（下の穴）。
- **規模**: M〜L（14〜18）のまま。**分割の案**: なし（一度切り出した spec）。
- **先に要るもの**: `currentghost-property-tree`（その前に `property-name-case-fold`）。
- **ファイルの重なり**: `currentghost-property-others`・`property-catalog-lists`・`zorder-property`（`dotted.rs`・`emo2_boot/mod.rs`）・`property-name-case-fold`・`property-catalog-lists`（`key.rs`）・`emo2_boot` の結線の列の全員（`mod.rs`）。根の `Cargo.toml` に触るなら、版上げ（`release-cycle`）や依存を足す spec（`mcp-stdio-bridge`・`makoto-dll-host`）と同じウェーブに置けない。
- **優先度の区分**: C（ukadoc の `system.*` の拾い残し）。**要件定義のモデル**: Opus（議題は「毎回問い合わせるか間隔を置くか」の 1 つで、口の形は前の spec が決める）。
- **見つけた穴・古くなった記述**: 電源・ディスク・テーマの値を採るには、Windows の機能（電源＝`Win32_System_Power`・ディスクの空き＝`Win32_Storage_FileSystem`・テーマの設定＝`Win32_System_Registry` の見込み）が要るが、根の `Cargo.toml` の `windows` の機能の一覧に今は無い（在るのは `Win32_System_SystemInformation`・`Win32_System_Performance` など）。触るファイルの一覧に根の `Cargo.toml` が無かった。機能を足すだけなら `Cargo.lock` は変わらない見込みだが、設計の段で確かめる。
