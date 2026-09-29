# Brief: areka-P0-mcp-server-core

> 2026-09-29 `/kiro-discovery` で起票（開発者指示「ssp mcp tool の完全移植のための spec 群を立ち上げて。実装は α リリースの後。areka の 127.0.0.1 の適当なポートでサーバを開く形。基本実装 → 空のダミー関数を置いて入り口だけ全部整備 → 個別のコマンド実装。平行開発しやすいように分割」）。SSP MCP 移植の **1 段目（基本実装）**。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。
> **事実の正本**は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 への実測）。本文の file:line は起票時（main `c3876110`）の実測＝**着手時に引き直すこと**。

## Problem

- **AI エージェントでゴーストを作る人**（Claude Code・Cursor など）: SSP では内蔵 MCP サーバ経由で台本を流す・イベントを起こす・画像を撮る・ログを読むことができ、ukadoc の「その他の機能」がこれを「AI エージェントでゴーストを作りたい（Vibe Coding）」の土台として案内している。areka には受け口が無く、areka 上で同じ開発の回し方ができない。
- **areka の開発そのもの**: 実機の確かめは今 `AREKA_APP_SMOKE_EXIT_MS`＋ログ grep に頼る。MCP があれば動いている areka へ直接問い合わせられる。

## Current State

- サーバの類は 0（`std::net`・tokio・hyper・tiny_http いずれも無し）。SSTP（9801）も未実装（roadmap「α 後」の予約・brief なし）。
- `serde_json` は lockfile にある（ukadoc-survey・dola 経由）が app は使っていない。tokio は非同期ランタイムとしては入っていない（wasm-bindgen-futures 経由の推移依存だけ）。app の非同期は `async-io`／`async-channel`（`areka-actor`）。
- **2026-09-29 開発者判断「tokio 依存は入れてよい。MCP を自作するのは避けた方がよさそう」**＝公式 Rust SDK `rmcp`（`modelcontextprotocol/rust-sdk`・MIT・tokio 必須・`2026-07-28` 準拠で `2025-11-25` 以前と互換・Streamable HTTP のサーバは Tower のサービス・無状態の既定と `with_json_response(true)`）を使う。依存の承認はこの判断で済んでいる（`deny.toml` の検査は要件の段で通す）。
- `doc/CONSTITUTION.md`「MCP 採用方針」は `.kiro/specs/areka-P0-mcp-server/`（実在しない）へリンクしている。本 spec 群が「プラットフォームは MCP サーバー」の側を満たす（同節の「ゴーストは MCP クライアント」は本 spec 群の外＝Out of Boundary）。

## Desired Outcome

- areka が起動すると `127.0.0.1:<port>` で HTTP を受け、`POST /api/mcp/v1` が MCP サーバとして応える。プロトコルの細部は rmcp（＝MCP の規格）に従い、SSP の癖（survey §2）は**参考**とする＝SSP と違っても、クライアントから見えるツールの名前・引数・結果の文字列が同じなら可。ツールはまだ 0 本（`tools/list` は空）。
- クライアントは `claude mcp add --transport http areka http://127.0.0.1:<port>/api/mcp/v1` で登録でき、`initialize`→`tools/list`→`ping` が通る。Claude Desktop は HTTP を直接書けないので、SSP の `mcp.exe` 相当の中継は `mcp-stdio-bridge`（M2 の並走枠）が作る。
- 待受の失敗（ポート使用中など）はログに理由を 1 行出してアプリは動き続ける（ログ無しの失敗の禁止）。

## Approach

- **新しい葉クレート `areka-mcp`**: 専用スレッドに tokio のランタイムを 1 つ立て、rmcp の Streamable HTTP サーバ（無状態・JSON 応答）を `127.0.0.1` で待ち受ける。**tokio はこのスレッドの中に閉じ込める**（app のほかの部分は `async-io` の世界のまま・rmcp の型を `areka-mcp` の外へ出さない）。
- rmcp の `ServerHandler` を実装し、ツールの一覧と呼び出しは「ツール表」へ委ねる形にしておき、表への登録口だけを公開する（表の中身は `mcp-tool-entrances`）。
- rmcp が持たないものだけを足す: `Origin` 検査（rmcp に同等の設定があればそれを使う）・`GET /api/mcp/help`・`GET /api/mcp/v1` の手打ちフォーム（要るかは要件で決める）。
- **ポート（起票時の決め）**: 既定は `9821`、`AREKA_MCP_PORT` で変更、`0` で待ち受けない。9801 を避けた理由＝開発者の机では SSP が 9801 を使っていて同時に動かす・9801 は将来の SSTP の口。**既定で有効**（SSP と同じ＝常に待ち受ける）。この 2 点は要件の段で開発者が覆してよい。

## Scope

- **In**:
  - 待受（`127.0.0.1` 限定・ポートの決定と環境変数・失敗の記録・アプリの終了でスレッドを畳む）。
  - rmcp の組み込み（tokio のスレッド・Streamable HTTP の無状態と JSON 応答・`ServerHandler`・`capabilities: {tools:{}}`・`instructions`・`serverInfo`〔名前と版は要件で決める〕）と `POST /api/mcp/v1` への取り付け。
  - `GET /api/mcp/help`（登録手順。HTTP 直結のコマンド例。Desktop 用の例は `mcp-stdio-bridge` が足す）。手打ちフォームは要件で要否を決める。
  - `Origin` 検査（localhost 以外は拒否）。
  - SSP との差の記録: survey §2 の各行について rmcp の振る舞いを実測し、違う行を一覧にする（直すのは「クライアントが困る」行だけ）。
  - 決定論テスト（実ソケットを 127.0.0.1 のエフェメラルポートで開き、`initialize`〔5 版〕→`tools/list`→`ping`、無状態版の `server/discover`、悪い `Origin` を当てる）。
- **Out**: ツールの定義と中身（後続の spec）・rmcp の中身に手を入れること・認証・TLS・リモート接続。

## Boundary Candidates

- rmcp（輸送とプロトコル・既製）と、ツール表（登録口・自前）の 2 層。rmcp の型は `areka-mcp` の中に留める。
- アプリ本体（`crates/areka`）側は「起動時にサーバを立て、終了時に畳む」の配線だけ。

## Out of Boundary

- ゴーストが MCP クライアントになる構想（`doc/CONSTITUTION.md`）。
- SSTP（9801）とその HTTP 経路。MCP を SSTP の口へ同居させるかは SSTP の spec が決める。
- ツールの引数検査・`ghost_name` の解決・アプリ本体への問い合わせの橋（`mcp-tool-entrances`）。

## Upstream / Downstream

- **Upstream**: なし（α 完成宣言のあと）。
- **Downstream**: `mcp-tool-entrances` → 各ツールの spec 7 本・`mcp-stdio-bridge`（roadmap「SSP MCP の移植」）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: SSTP（予約・brief なし）。`doc/CONSTITUTION.md` の MCP 節のリンク（本起票で直した）。

## Constraints

- 外部依存（`rmcp`・`tokio`・HTTP の土台〔hyper か axum〕・`serde_json` とその推移依存）は 2026-09-29 に開発者が承認済み。`deny.toml`（ライセンスと重複の検査）と `THIRD-PARTY-NOTICES.md` の更新は本 spec の仕事。
- rmcp の版は固定し、上げるときは本 spec の決定論テストを通す（MCP の新しい版への追随は rmcp の版上げで行う）。
- 常時テストはネットへ出ない（ループバックのみ）。ポートはテストごとにエフェメラル。
- ログは `tracing`、失敗の経路は必ず `error!`/`warn!` を出す。
