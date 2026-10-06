# Brief: areka-P0-mcp-get-status

> 2026-10-05 棚卸㉒で `mcp-kanade-tools` から切り出した。理由は大きさでなく並走: `get_status` だけは kanade の運行（`schedule/` の下）に触らずに答えられるので、kanade の進行の列を待たずに走れる。SSP MCP の移植（roadmap「SSP MCP の移植」）の 1 本。

## Problem

MCP の `get_status` が今も `NG:not implemented yet` を返す（`crates/areka/src/mcp/get_status.rs` の `handle`）。エージェントは、ゴーストが話している最中か・選択待ちか・起動の途中かを知る手段が無く、台本を送る時機を選べない。

## Current State

- ツールの入口・名前の解決・UI スレッドへの橋は `mcp-tool-entrances` が作った。後から答える口 `mcp::later` は `dump_surface`・`dump_balloon` が使っている（手本は `dump_surface.rs` の `handle`）。
- 実行の状態は kanade が持つ: `crates/areka-kanade/src/status.rs` の `ExecutionState`（10 状態）と、`ExecutionStatus::derive(..).render()`（SHIORI へ渡す `Status` の文字列を作る・公開済み）。
- kanade の殻がその場で答える作りの手本は `KanadeMsg::ResourceQuery`（`msg.rs`・`actor.rs`）。
- 書式の正本は SSP 2.9.05 の `get_status` の答え（survey の表）。

## Desired Outcome

`get_status` が宛先のゴーストの今の実行の状態を、SSP と同じ書式で返す。ゴーストが居なければ SSP と同じ文言の NG。

## Approach

kanade の殻へ問い合わせの変種を 1 つ足し、`State::snapshot()` から `ExecutionStatus::derive(..).render()` を作って返す。運行の表（`schedule/`）には触らない。MCP の側は `mcp::later` で返事を待つ。

## Scope

- **In**: `get_status` の本物の答え・kanade の問い合わせの変種と殻の腕・決定論テスト・実機で Claude Code から 1 回確かめる。
- **Out**: `sakurascript`・`raise_event`（`mcp-kanade-tools`）・`reload`（`mcp-reload`）・`strict`（`mcp-strict-errors`）。

## Boundary Candidates

- MCP のツールのファイル（`get_status{,_tests}.rs`）
- kanade の殻の問い合わせ（`msg.rs` の変種・`actor.rs` の腕・新規 `actor_status.rs`）

## Out of Boundary

- 実行の状態の決め方そのもの（`status.rs`）。SSP の旗 `changing`（areka の 10 状態にも ukadoc にも無い）を足すかは要件の議題で、足すなら `status.rs` と切替の写しの通り道が範囲に入る。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`・`status-execution-states`（完了）。
- **Downstream**: `mcp-kanade-tools`（`msg.rs`・`actor.rs` を分け合う＝同じウェーブに置かない）。`mcp-user-response`・`mcp-author-tools` が状態を読むときの口。

## Existing Spec Touchpoints

- **Extends**: `mcp-kanade-tools` の範囲から `get_status` を引き取る。
- **Adjacent**: MCP の 3 段目の約束（`crates/areka/src/mcp/mod.rs`・`handler.rs` を触らない）。

## Constraints

- 触るファイル: `crates/areka/src/mcp/get_status.rs`・`get_status_tests.rs`・`crates/areka-kanade/src/msg.rs`（変種 1 つと名前の腕）・`actor.rs`（振り分けの腕 1 つ）・新規 `actor_status.rs`＋兄弟テスト。`lib.rs`・`schedule/` は触らない。
- `msg.rs` は 909 行・`actor.rs` は 876 行（上限 1,000）＝足すものは新しいファイルへ。

## 想定

- 規模 S（4〜6 タスク）。−（Opus で足りる）。

### 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）

- kanade の `schedule/`・`lib.rs` に触らない（`balloon-lifecycle-events` の持ち物）。MCP の `mod.rs`・`handler.rs` に触らない。
