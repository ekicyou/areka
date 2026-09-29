# Brief: areka-P0-mcp-get-property

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントがゴーストを作るとき、プロパティシステム（`currentghost.*`・`system.*` など）を読んで状態を確かめたい。SSP の `get_property` がこれを担う。

## Current State

- `SylphyaReader::resolve_dotted_str`（`crates/areka-sylphya/src/reader.rs` 90 行目付近）は同期・ロック無しで、どのスレッドからも呼べる。今の呼び手は SHIORI 側の `GetProperty`（`crates/areka/src/shiori_host.rs` 247〜256 行目）。
- ランタイムの reader は私有の欄（`crates/areka-ghost/src/runtime.rs` 163 行目付近）で、外へは `GhostParts` からしか出ていない＝取り出し口が要る。
- 値の網羅（`currentghost.*` の多くが未実装）は別の spec の持ち物（下の Existing Spec Touchpoints）。

## Desired Outcome

- `get_property(property_name, ghost_name?)` が、SHIORI の `GetProperty` と**同じ解決**で値を素の文字列で返す（survey §3: `currentghost.name` → `Emily/Phase4.5`）。
- 無い名前は `NG:Cannot find such property name.`（`isError: true`）。
- 本 spec の後にプロパティの網羅が進めば、MCP 側は何もせずに答えが増える。

## Approach

`mcp-tool-entrances` が置いたダミーの中身を書く。reader の取り出し口を `GhostRuntime`／`GhostSession` に足し、UI スレッドの処理で引く（切替の後も今のゴーストの reader を引く）。

## Scope

- **In**: reader の取り出し口・`get_property` の中身・「無い名前」と「値が空」の区別（SHIORI の `GetProperty` の既存の区別に合わせる）・決定論テスト。
- **Out**: プロパティの値そのものの追加。

## Boundary Candidates

- sylphya の読み手（既存）と MCP のツール（新）の間の取り出し口 1 つ。

## Out of Boundary

- `currentghost.*`・一覧系・`status`・`zorder` などの値の実装（下の既存 spec）。
- 書き込み（SSP の MCP に `set_property` は無い）。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `property-query-channels`・`property-ipc-transport`・`currentghost-property-tree`・`property-catalog-lists`（いずれも α 後・brief のみ）。これらが値を足すと本ツールの答えが増える。

## Constraints

- 規模 S。`crates/areka-ghost/src/runtime.rs` を触る＝同じウェーブの `mcp-reload` と重なりうる（干渉台帳を見よ）。
