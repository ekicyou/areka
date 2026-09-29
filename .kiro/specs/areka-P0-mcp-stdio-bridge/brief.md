# Brief: areka-P0-mcp-stdio-bridge

> 2026-09-29 `/kiro-discovery` の続きで起票（開発者判断「SSP と同じ対応を行うには中継 exe を作らないとダメ。どうせ作りたくなるので spec を置いてロードマップに置く」）。SSP MCP 移植の **M2 の並走枠**。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（§1・§6）。

## Problem

- **Claude Desktop の利用者**が areka の MCP を使えない。Desktop の `claude_desktop_config.json` は `command` で起動する stdio のサーバしか書けず（`url` の欄が無い＝本体の検査関数で確認・survey §6）、コネクタ登録は Anthropic のクラウドからつなぎに来るので `127.0.0.1` へ届かない（公式の案内に明記）。
- 代用の `npx mcp-remote` は Node.js を入れさせる＝第三者に勧める形として重い。
- SSP は同じ理由で `data\mcp.exe`（stdio ⇔ HTTP の中継）を同梱している。

## Current State

- `mcp-server-core` が着地すれば、areka 本体は `127.0.0.1:<port>/api/mcp/v1` で HTTP を受ける（既定 9821・`AREKA_MCP_PORT`）。
- SSP の `mcp.exe` の実測（survey §6）: 引数なしで起動し、標準入力の 1 行 1 要求を `POST /api/mcp/v1`（`Host: 127.0.0.1:9801` 焼き込み）へ流し、応答を 1 行で標準出力へ返す。通知には何も返さない。JSON でない行には `-32700`・`id: null`。本体が居なければ「Server not available」「No response from server」、壊れた応答には「Invalid JSON response from server」（文字列で確認・振る舞いは未実測）。`Mcp-Method`／`Mcp-Name`／`MCP-Protocol-Version` のヘッダを付けて送る。

## Desired Outcome

- `areka-mcp-bridge.exe`（名前は要件で決める）を Claude Desktop の `mcpServers` に `command` として書けば、areka の MCP ツールが Desktop から使える。
- areka が起動していない・ポートが違う・応答が壊れている、のいずれでも Desktop 側に理由の分かる JSON-RPC のエラーが返り、中継は落ちない。
- `GET /api/mcp/help` に Desktop 用の設定例（中継の絶対パス入り）と HTTP 直結のコマンド例が並ぶ。
- 配布 zip に中継が入る。

## Approach

小さな bin クレートを 1 つ（外部依存なしを目標・JSON は `serde_json` を `mcp-server-core` と共有）。**rmcp も tokio も使わない**——中継は MCP の中身を作らず写すだけなので、本体の「MCP を自作しない」（2026-09-29 開発者判断＝rmcp 採用）には当たらない。写すだけで足りず MCP を解釈する必要が出たら（上の Boundary の問い）、そのときは rmcp の stdio と HTTP クライアントで組む。標準入力を 1 行ずつ読み、`127.0.0.1:<port>` へ HTTP/1.1 で投げ、応答本文を 1 行で書き出す。ポートは `AREKA_MCP_PORT`（Desktop の設定の `env` で渡せる）→ 既定 9821 の順（引数で渡すかは要件で決める）。

## Scope

- **In**: 中継の exe・ポートの決め方・本体不在と異常応答のエラー・通知の扱い・`/api/mcp/help` への Desktop 用の設定例・配布スクリプト（`tools/package-alpha.ps1` の後継）への同梱・決定論テスト（ループバックの偽サーバで）・SSP の `mcp.exe` と同じ入力に同じ出力を返すことの突き合わせ（要件の段で 1 度）。
- **Out**: 本体側のプロトコル（`mcp-server-core`）・ツール（M2 以降）・Desktop の拡張機能（`.mcpb`）の包み。

## Boundary Candidates

- 中継（stdio ⇔ HTTP の写し）と、本体（MCP の中身）の境。中継は MCP の中身を解釈しない（写すだけ）かどうかを要件で決める——SSP の `mcp.exe` は `initialize`・`tools/list`・`server/discover` の文字列を持つ＝何か解釈している可能性がある。

## Out of Boundary

- 127.0.0.1 以外への中継（リモートの areka）。
- SSP の `mcp.exe` と同じ 9801 の焼き込み（areka は既定 9821）。

## Upstream / Downstream

- **Upstream**: `mcp-server-core`。
- **Downstream**: `.mcpb` の包み（未起票・欲しくなったら）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `mcp-server-core`（`/api/mcp/help` の文面）・完了 `alpha-package`（配布スクリプトの同梱物）。

## Constraints

- x64＋arm64（32bit は要らない）。Desktop が起動するプロセスなので、標準出力へログを出さない（ログは標準エラー＝Desktop の `mcp-server-<名>.log` に残る）。
- 規模 S。
