# Brief: areka-P0-currentghost-property-others

> 2026-10-04 棚卸㉑で `areka-P0-currentghost-property-tree` から切り出し（開発者「負荷が高すぎる仕様は分割を検討せよ」）。元の spec は「動く値を出す口」と `currentghost.balloon.scope(ID).*` の 19 項目を持ち、本 spec は `currentghost.*` の残りを持つ。

## Problem

ゴーストの台本はプロパティ `currentghost.*` で自分の状態（面の番号・窓の位置と大きさ・モニタ・シェルの一覧・当たり判定の名前…）を照会する。areka の sylphya には `currentghost.*` の値が無く、照会しても値なし（NOT_FOUND）になる。

## Current State

- 網羅台帳 `doc/ukadoc-coverage/ledger/property.toml` の `currentghost.*` の行（64）は今 `currentghost-property-tree` が持ち主。そのうち `balloon.scope(ID).*`・`balloon.汎用`・`balloon.count` 以外が本 spec の範囲（着地時に持ち主を本 spec へ直す）。
- 値の源: 面と窓は表示の側（`crates/areka/src/emo2_boot/`・`placement/`）、シェルの一覧は `crates/areka-ghost/src/catalog.rs` の `list_shells`・`list_all_shells`、運行の状態は kanade の `ExecutionStatus::render`（kanade のスレッドの `State` から毎回導かれる）。
- 動く値を出す口は sylphya に無い（`SylphyaPublisher` は静的・SHIORI 照会・永続だけ）。`currentghost-property-tree` が作る（その spec を口の持ち主に決めた）。
- `currentghost.seriko.zorder` は `zorder-property` の持ち主（本 spec の範囲から除く・棚卸⑬の裁定）。

## Desired Outcome

`currentghost.scope(ID).*`＋`.scope.count`・`currentghost.mousecursor.*`・`currentghost.seriko.*`（`zorder` を除く 13）・`currentghost.shelllist.*`・`currentghost.status`・`currentghost.汎用` が実際の状態から導かれ、切替のたびに出し直され、照会で読める。未導出の値は値なしのまま（捏造しない）。

## Approach

`currentghost-property-tree` の動く値の口に乗り、枝ごとに値を集める所を足す（scope の幾何・seriko・mousecursor・shelllist・status）。SET 有効の項目は値の保持と読みまで（書き込みの効果は各機能の spec）。

## Scope

- **In**: 上の枝の値の導出と出し直し（起動・切替・更新の読み直し）・scope ID の列挙・決定論のテスト（scope 2 体）・網羅台帳の該当行。
- **Out**: 動く値の口と `balloon.scope` の 19 項目（`currentghost-property-tree`）・`seriko.zorder`（`zorder-property`）・`currentghost.sound.*`（`property-catalog-lists`）・照会の経路（`property-query-channels`）・SET の書き込みの効果。

## Boundary Candidates

- scope の幾何（面・窓・モニタ）／seriko と mousecursor（当たり判定の名前の選び手）／shelllist と status。

## Out of Boundary

- 書字方向やバルーンの寸法の解決（bvc・`currentghost-property-tree`）・窓配置の規則（placement 系）。

## Upstream / Downstream

- **Upstream**: `currentghost-property-tree`（動く値の口・scope の列挙の決まり）・`property-query-channels`（照会の end-to-end）・完了 `scope-zorder-pinning`。
- **Downstream**: 自分の状態を照会する実ゴーストの適合。

## Existing Spec Touchpoints

- **Extends**: `currentghost-property-tree`（分割元）。
- **Adjacent**: `zorder-property`・`system-property-values`・`property-catalog-lists`（同じ sylphya の口）・`mcp-get-property`（読む側）。

## Constraints

- 正典はライブの ukadoc（MCP のスナップショットのプロパティ節は 2.8.80 で古い）。値の捏造禁止・決定論のテスト必達。
- sylphya の語彙（`vocab/dotted.rs`）と `emo2_boot/mod.rs` を触る spec と同時に走らせない。

## 2026-10-04 棚卸㉑で切り出し

- 元の spec: `currentghost-property-tree`（⒝）。
- 規模: M〜L（14〜18 タスク）。
- 前提: `currentghost-property-tree`（未）。
- 触るファイル: `crates/areka-sylphya/src/vocab/dotted.rs`・`crates/areka-ghost/src/{sylphya_wiring.rs, catalog.rs（読むだけ）}`・`crates/areka/src/emo2_boot/{mod.rs, ghost_switch.rs, shell_balloon_switch.rs}`・`currentghost-property-tree` が作る値を集める所・`doc/ukadoc-coverage/ledger/property.toml`。
- 共有しうる相手: `currentghost-property-tree`・`system-property-values`・`property-catalog-lists`・`zorder-property`・`mcp-get-property`。
- 議題: `currentghost.status` を kanade のスレッドから UI の側へどう写すか（kanade に知らせを足すなら kanade の進行の列に入る）。
