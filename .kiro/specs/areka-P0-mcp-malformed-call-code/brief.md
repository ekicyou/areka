# Brief: areka-P0-mcp-malformed-call-code

> 2026-10-11 `/kiro-discovery` で起票（`areka-P0-mcp-stdio-bridge` の完了時の棚卸）。出どころは同 spec のタスク 5.1（本物の受け口と中継ごしの応答の比較）の気付き。場所の記述は起票時（main `95b34055` を取り込んだ `mcp-stdio-bridge` のブランチ）＝着手時に引き直す。

## Problem

AI のクライアントが、形の違う `params` の `tools/call`（`name` が文字列でなく数、など）を送ると、areka の MCP の受け口は `-32602`（引数が正しくない）でなく `-32601`（そのメソッドは無い・文は `tools/call`）で答える。`tools/call` は在るので、答えの番号が事実と合っていない。クライアントは「このサーバーはツールを呼べない」と読みうる。

## Current State

- `mcp-tool-entrances` は「名前・必須の欄・型の誤りは `-32602`」と決めて作った（`crates/areka-mcp/src/handler.rs` の頭の説明）。これは `params` の**中の `arguments`** が `inputSchema` に合わないときと、未登録のツール名のときに効いている。
- 効いていないのは、`params` そのものの形が MCP の型に合わないとき（`{"name": 7}` など）。rmcp 3.5.0 は型の合わない要求を `CustomRequest`（型のない要求）として読み、`ArekaHandler` は `ServerHandler::on_custom_request` を上書きしていないので、rmcp の既定の `-32601` が返る。
- 中継（`areka-mcp-bridge`）は型のない要求も中身のまま届けるので、中継ごしでも同じ答えになる。
- この答えを固定している所が 2 つある（直すときに一緒に直す）:
  - `crates/areka-mcp-bridge/tests/real_receiver_test.rs` の比較の表の行 `{"id": 8, "method": "tools/call", "params": {"name": 7}, "at": "/error/code", "is": -32601}` と、その上の説明の注釈。
  - `doc/ssp-mcp/bridge-diff-areka.md` の「形の違う `params` の `tools/call`」の行（areka の欄に `-32601` と本 spec の名前が書いてある）。

## Desired Outcome

- areka が知っているメソッド（`tools/call`・`tools/list`・`initialize` など）に形の違う `params` が来たら `-32602` で答える。知らないメソッドは今のまま `-32601`。
- 5 つの版（`2024-11-05`〜`2026-07-28`）のどれでも同じ。中継ごしでも同じ（中継のコードは直さない見込み）。
- 上の 2 か所の固定が新しい答えに替わっている。

## Approach

`ArekaHandler` に `on_custom_request` を書き、メソッド名が areka の知っている名前なら `-32602`、そうでなければ既定の `-32601` を返す。知っている名前の一覧をどこから取るか（rmcp の型の名前の定数か、自分の表か）は設計で決める。

## Scope

- **In**: `crates/areka-mcp/src/handler.rs` の 1 か所・受け口の決定論テスト（形の違う `params` を版ごとに）・中継の比較テストの 1 行・差の記録の 1 行。
- **Out**: `arguments` の検査（今も `-32602`）。ツールの中身。中継の作り。SSP の `mcp.exe` がこの形にどう答えるか（未実測のまま・測らない）。

## Boundary Candidates

- 「型のない要求をどう読み分けるか」（受け口）と「その答えを固定する場所」（受け口のテスト・中継の比較テスト・差の記録）。

## Out of Boundary

- 通知（`id` の無いもの）の形の違い（今は 202・本文なしで、要件に合っている）。
- `id` の形の違い（`"id":{}` など。rmcp が通知として読む＝`mcp-stdio-bridge` のタスク 3.1 の記録）。

## Upstream / Downstream

- **Upstream**: `mcp-server-core`・`mcp-tool-entrances`・`mcp-stdio-bridge`（どれも着地済み）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（完了した `mcp-tool-entrances` の決まり「型の誤りは `-32602`」の穴を埋める）。
- **Adjacent**: `crates/areka-mcp/src/handler.rs` を触る spec（`mcp-kanade-tools`・`mcp-reload`・`mcp-shiori-query`・`mcp-user-response` など MCP の段）。同時に走らせるなら触る行を照合する。`mcp-stdio-bridge-residue`（中継の拾い残し・テストのファイルが重なる）。

## Constraints

- rmcp は `=3.5.0` の固定。版を上げる話は別（`tech.md`）。
- 受け口の答えを変えるので、中継の `real_receiver_test.rs`（本物の受け口との一致）を必ず一緒に通す。
- 区分 B（バグ）。規模 S（3〜5）。
