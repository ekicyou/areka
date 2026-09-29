# Brief: areka-P0-mcp-server-core

> 2026-09-29 `/kiro-discovery` で起票（開発者指示「ssp mcp tool の完全移植のための spec 群を立ち上げて。実装は α リリースの後。areka の 127.0.0.1 の適当なポートでサーバを開く形。基本実装 → 空のダミー関数を置いて入り口だけ全部整備 → 個別のコマンド実装。平行開発しやすいように分割」）。SSP MCP 移植の **1 段目（基本実装）**。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。
> **事実の正本**は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 への実測）。本文の file:line は起票時（main `c3876110`）の実測＝**着手時に引き直すこと**。

## Problem

- **AI エージェントでゴーストを作る人**（Claude Code・Cursor など）: SSP では内蔵 MCP サーバ経由で台本を流す・イベントを起こす・画像を撮る・ログを読むことができ、ukadoc の「その他の機能」がこれを「AI エージェントでゴーストを作りたい（Vibe Coding）」の土台として案内している。areka には受け口が無く、areka 上で同じ開発の回し方ができない。
- **areka の開発そのもの**: 実機の確かめは今 `AREKA_APP_SMOKE_EXIT_MS`＋ログ grep に頼る。MCP があれば動いている areka へ直接問い合わせられる。

## Current State

- サーバの類は 0（`std::net`・tokio・hyper・tiny_http いずれも無し）。SSTP（9801）も未実装（roadmap「α 後」の予約・brief なし）。
- `serde_json` は lockfile にある（ukadoc-survey・dola 経由）が app は使っていない。**新しい外部依存は開発者の承認が要る**（`areka-actor/src/lib.rs` 74〜77 行目付近のコメントの慣行）。
- `doc/CONSTITUTION.md`「MCP 採用方針」は `.kiro/specs/areka-P0-mcp-server/`（実在しない）へリンクしている。本 spec 群が「プラットフォームは MCP サーバー」の側を満たす（同節の「ゴーストは MCP クライアント」は本 spec 群の外＝Out of Boundary）。

## Desired Outcome

- areka が起動すると `127.0.0.1:<port>` で HTTP を受け、`POST /api/mcp/v1` が SSP と同じ振る舞いの MCP サーバとして応える（survey §2 の表の全行）。ツールはまだ 0 本（`tools/list` は空）。
- クライアントは `claude mcp add --transport http areka http://127.0.0.1:<port>/api/mcp/v1` で登録でき、`initialize`→`tools/list`→`ping` が通る。SSP の `mcp.exe`（stdio 橋・9801 焼き込み）は要らない。
- 待受の失敗（ポート使用中など）はログに理由を 1 行出してアプリは動き続ける（ログ無しの失敗の禁止）。

## Approach

- **新しい葉クレート `areka-mcp`**: 専用スレッドで `std::net::TcpListener` を `127.0.0.1` に束ね、HTTP/1.1 の最小限（`Content-Length` の本文・keep-alive）を自前で読む。JSON は `serde_json`（承認を取る）。非同期ランタイムは入れない。
- 要求の処理は「ツール表」へ委ねる形にしておき、表への登録口だけを公開する（表の中身は `mcp-tool-entrances`）。
- **ポート（起票時の決め）**: 既定は `9821`、`AREKA_MCP_PORT` で変更、`0` で待ち受けない。9801 を避けた理由＝開発者の机では SSP が 9801 を使っていて同時に動かす・9801 は将来の SSTP の口。**既定で有効**（SSP と同じ＝常に待ち受ける）。この 2 点は要件の段で開発者が覆してよい。

## Scope

- **In**:
  - 待受（`127.0.0.1` 限定・ポートの決定と環境変数・失敗の記録・アプリの終了でスレッドを畳む）。
  - HTTP: `POST /api/mcp/v1`（JSON）、`GET /api/mcp/v1`（手打ちフォーム・`text/plain` のフォーム送信の受理）、`GET /api/mcp/help`（登録手順。areka では HTTP 登録のコマンド例）。それ以外は 404。
  - JSON-RPC 2.0: 単発の要求・通知（202）・エラー（`-32700`／`-32600`／`-32601`／`-32602`、`data` は `message` と同文）・バッチの拒否。
  - MCP: `initialize`（5 版の交渉・未知なら `2025-11-25`）・`ping`・`tools/list`・`tools/call` の振り分け・`resources/list`／`resources/templates/list`／`prompts/list` の空応答・`capabilities: {tools:{}}`・`instructions`・`serverInfo`（名前と版は要件で決める）。
  - 無状態版 `2026-07-28`: `server/discover`・`_meta` の必須検査・`Mcp-Method`／`Mcp-Name` の食い違い（`-32020`）・結果の `resultType` と `_meta.serverInfo`。
  - `Origin` 検査（localhost 以外は 403）。
  - 決定論テスト（実ソケットを 127.0.0.1 のエフェメラルポートで開いて当てる）。
- **Out**: ツールの定義と中身（後続の spec）・SSE・セッション ID・認証・TLS・リモート接続。

## Boundary Candidates

- 輸送（HTTP の読み書き）と、プロトコル（JSON-RPC＋MCP の版と検査）と、ツール表（登録口）の 3 層。
- アプリ本体（`crates/areka`）側は「起動時にサーバを立て、終了時に畳む」の配線だけ。

## Out of Boundary

- ゴーストが MCP クライアントになる構想（`doc/CONSTITUTION.md`）。
- SSTP（9801）とその HTTP 経路。MCP を SSTP の口へ同居させるかは SSTP の spec が決める。
- ツールの引数検査・`ghost_name` の解決・アプリ本体への問い合わせの橋（`mcp-tool-entrances`）。

## Upstream / Downstream

- **Upstream**: なし（α 完成宣言のあと）。
- **Downstream**: `mcp-tool-entrances` → 各ツールの spec 7 本（roadmap「SSP MCP の移植」）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: SSTP（予約・brief なし）。`doc/CONSTITUTION.md` の MCP 節のリンク（本起票で直した）。

## Constraints

- 外部依存の追加（`serde_json`）は開発者の承認を要件の段で取る。
- 常時テストはネットへ出ない（ループバックのみ）。ポートはテストごとにエフェメラル。
- ログは `tracing`、失敗の経路は必ず `error!`/`warn!` を出す。
