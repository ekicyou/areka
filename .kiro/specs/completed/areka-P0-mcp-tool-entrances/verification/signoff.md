# 実機確認の記録（タスク 7.3・要件 8.5・6.7）

- **実施日**: 2026-10-03
- **判定に使った版**: コミット `e9221437`。配布物の `BUILD-INFO.txt` は `commit=e922143` `dirty=0` `script=tools/package.ps1 2.0.0`
- **ビルド**: `pwsh -NoProfile -File tools/package.ps1`（全段 緑・終了コード 0）→ `target\package\areka-0.0.1-x64.zip`（中の `areka.exe` は 8,383,488 バイト）
- **展開先**: `target\signoff-ent\x\`（ワークツリーの `target\` の下・絶対パス）。記録は `target\signoff-ent\logs\`（`run1.log`・`run1.stderr.log`・`curl1.txt`・`claude-p.jsonl`）
- **起動のしかた**: `AREKA_*`／`WINTF_*` を全部外してから（外す前から 0 個だった）、`Start-Process -PassThru` で展開先の `areka.exe` を起こした（作業フォルダは展開先・プロセス番号 26260）。終わらせ方は有界の自動終了 `AREKA_APP_SMOKE_EXIT_MS`。自分で起こしたプロセス以外は止めていない
- **環境変数**:

  ```
  AREKA_APP_SMOKE_EXIT_MS=600000
  AREKA_NO_ALERT=1
  NO_COLOR=1
  RUST_LOG=info,areka_mcp=debug,areka::perf=debug
  AREKA_PERF_THREAD_REPORT_SEC=5
  ```

- **使った道具**: Claude Code `2.1.283`（CLI）・curl（Git for Windows 同梱）・この記録を書いたエージェント自身（Claude Code のセッション。下の「追加の 1 項目」）
- **SSP**: この走行の間は動いていなかった。areka は既定の 9801 で待ち受けた

## 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | 配布形の `areka.exe` を emo2 で起動し、`claude mcp add --transport http` で登録して `tools/list` に 10 本が出る | **合**（`claude mcp get areka` が `√ Connected`。Claude Code が受け取った道具に `mcp__areka__*` が 10 本。同じ形の `curl` の `tools/list` は 10 本で、保存してある SSP の JSON と並び・中身とも一致） |
| ⑵ | `get_active_ghost_list` が emo2 の descript の `name` を返す | **合**（`えも？？`・`isError: false`。descript の `name,えも？？` と同じ） |
| ⑶ | その名前とルートフォルダのフルパスの両方を `ghost_name` に渡した `get_status` が `NG:not implemented yet` | **合**（名前・フルパス・大文字にして `/` 区切りと末尾の `/` を付けたフルパスの 3 つとも） |
| ⑷ | 存在しない名前で `NG:Cannot find active ghost from specified name` | **合** |
| ⑸ | 呼んでいる間もゴーストの描画と会話が止まらない | **合**（会話の最中に 64 本を 25 秒続けて呼んだ。その間も表情のアニメーションと面の表示が続き、会話は始まって終わった。下記） |
| ⑹ | `RUST_LOG` を要件 6.7 の `debug!` が出る階層まで開け、記録に各呼び出しの行が残る | **合**（`tools/call` 74 本のうち、引数の検査を通った 73 本に「ツールに答えた」の `debug!` が 1 件ずつ・`tool`・`ghost`・`is_error`・`text` 付き。検査で拒まれた 1 本は handler の `debug!` 1 件） |
| 追加 | 必須の欄 `script` を抜いた `sakurascript` を Claude Code から呼び、エージェントに見えた文を書き残す | **記録した**（実機確認で使った Claude Code のセッション〔版未確認・2025-03-26 の形で接続〕が送る前に自分で inputSchema に照らして拒み、`script` が無いという理由の文がエージェントに見えた。areka には届いていない。2.1.283・無状態版の経路での見え方は未確認。差の一覧の該当行に記入済み。下記） |

**ERROR の段の行は 0 件**（標準出力・標準エラー出力の両方）。`areka_mcp` の `WARN` も 0 件。

## ⑴ 登録と 10 本の道具

### Claude Code での登録

前例（`mcp-server-core` の実機確認）と同じ手順。利用者の設定を変えないため、`--scope project` で `target\signoff-ent\proj\` の中だけに登録し、
同じフォルダの `.claude\settings.local.json` に `{"enabledMcpjsonServers": ["areka"]}` を書いた。見るのは `claude mcp get areka` だけ。
待受の番号は記録の行から読んだ:

```
2026-10-03T13:17:08.603976Z  INFO areka_mcp::server: MCP: 待受を始めた url=http://127.0.0.1:9801/api/mcp/v1
```

```
> claude mcp add --transport http --scope project areka http://127.0.0.1:9801/api/mcp/v1
Added HTTP MCP server areka with URL: http://127.0.0.1:9801/api/mcp/v1 to project config

> claude mcp get areka
areka:
  Scope: Project config (shared via .mcp.json)
  Status: √ Connected
  Type: http
  URL: http://127.0.0.1:9801/api/mcp/v1
```

このとき areka が受けた要求（記録より）:

```
2026-10-03T13:17:36.820000Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=server/discover id="server-discover-probe-1" status=200 path=/api/mcp/v1
2026-10-03T13:17:36.872032Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/list id=0 status=200 path=/api/mcp/v1
```

同じフォルダで、登録した `.mcp.json` だけを読ませて Claude Code を窓なしで起こした
（`claude -p --strict-mcp-config --mcp-config .mcp.json --no-session-persistence --output-format stream-json --verbose ...`）。
Claude Code の側のログインの期限が切れていて、モデルへの問い合わせは `Failed to authenticate: OAuth session expired and could not be refreshed` で止まった
（ログインし直すのは利用者の操作なので、ここでは行っていない）。ただし、問い合わせの前に Claude Code が出す最初の 1 行には、
areka に接続して受け取った道具が載っている（`logs\claude-p.jsonl`・関係する欄だけ抜き出し）:

```
"mcp_servers":[{"name":"areka","status":"connected","source":"dynamic"}]
"tools":[ ..., "mcp__areka__dump_balloon","mcp__areka__dump_surface","mcp__areka__get_active_ghost_list","mcp__areka__get_expression_table","mcp__areka__get_log","mcp__areka__get_property","mcp__areka__get_status","mcp__areka__raise_event","mcp__areka__reload","mcp__areka__sakurascript"]
"claude_code_version":"2.1.283"
```

`mcp__areka__` の道具は 10 本（並びは Claude Code の側で名前順に並べ替えたもの）。このときも areka は `server/discover` と `tools/list` に 200 で答えた（13:17:59）。

### curl で `tools/list`（Claude Code と同じ無状態版の形）

宛先 `http://127.0.0.1:9801/api/mcp/v1`。付け足し `-H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" -H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: tools/list"`、
本文の `params._meta` に `io.modelcontextprotocol/protocolVersion="2026-07-28"`・`clientCapabilities={}`・`clientInfo`。本文は UTF-8 のままファイルから送った（`--data-binary @…`）。

```
## id=101 tools/list
HTTP/1.1 200 OK
content-type: application/json
{"jsonrpc":"2.0","id":101,"result":{"resultType":"complete","ttlMs":0,"cacheScope":"private","tools":[{"name":"get_active_ghost_list",...},{"name":"get_status",...},...]}}
```

`tools` の名前の並び: `get_active_ghost_list`・`get_status`・`get_expression_table`・`get_property`・`get_log`・`sakurascript`・`raise_event`・`reload`・`dump_surface`・`dump_balloon`（10 本）。
受け取った `tools` の配列を、保存してある `doc/ssp-mcp/tools-list-ssp-2.9.05.json` の `tools` と比べ、並び・欄とも一致した（Python の `==` で `True`）。

## ⑵〜⑷ 呼び出し

### curl（無状態版の経路）

付け足しは `tools/list` と同じで、`Mcp-Method: tools/call` と `Mcp-Name: <道具の名前>` を足した。応答はどれも `HTTP/1.1 200 OK`（`id=107` だけ 400。「追加の 1 項目」）。

```
## id=102 get_active_ghost_list  arguments={}
{"jsonrpc":"2.0","id":102,"result":{"resultType":"complete","content":[{"type":"text","text":"えも？？"}],"isError":false}}

## id=103 get_status  arguments={"ghost_name":"えも？？"}
{"jsonrpc":"2.0","id":103,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:not implemented yet"}],"isError":true}}

## id=104 get_status  arguments={"ghost_name":"C:\\home\\maz\\git\\areka\\.claude\\worktrees\\areka-p0-emo-text-split-dc4e04\\target\\signoff-ent\\x\\ghost\\emo2"}
{"jsonrpc":"2.0","id":104,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:not implemented yet"}],"isError":true}}

## id=105 get_status  arguments={"ghost_name":"C:/HOME/MAZ/GIT/AREKA/.CLAUDE/WORKTREES/AREKA-P0-EMO-TEXT-SPLIT-DC4E04/TARGET/SIGNOFF-ENT/X/GHOST/EMO2/"}
{"jsonrpc":"2.0","id":105,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:not implemented yet"}],"isError":true}}

## id=106 get_status  arguments={"ghost_name":"no-such-ghost"}
{"jsonrpc":"2.0","id":106,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}}
```

emo2 の `ghost\master\descript.txt` は `charset,UTF-8`・`name,えも？？`（「？」は全角）。ルートフォルダは起動の記録の
`起動するゴーストを決めました event="ghost_resolved" route=Only dir=…\target\signoff-ent\x\ghost\emo2` と同じ。

### Claude Code から（エージェントに見えた結果）

窓なしの Claude Code はログインの期限切れで動かなかったので、この記録を書いたエージェント自身（Claude Code のセッション）から呼んだ。
このセッションには利用者が前から登録していた MCP サーバー `ssp` があり、その宛先が `127.0.0.1:9801` だった。SSP が動いていないこの時間は
9801 で待ち受けていたのは areka だけなので、`ssp` の名前の道具の呼び出しは areka に届いた（下の記録の行で確かめた）。登録は読んで使っただけで、変えていない。

エージェントに見えた結果（そのまま）:

```
mcp__ssp__get_active_ghost_list {}                                   → えも？？
mcp__ssp__get_status {"ghost_name":"えも？？"}                         → <error>NG:not implemented yet</error>
mcp__ssp__get_status {"ghost_name":"C:\home\maz\git\areka\.claude\worktrees\areka-p0-emo-text-split-dc4e04\target\signoff-ent\x\ghost\emo2"}
                                                                     → <error>NG:not implemented yet</error>
mcp__ssp__get_status {"ghost_name":"no-such-ghost"}                  → <error>NG:Cannot find active ghost from specified name</error>
```

`isError: true` の結果は、エージェントにはエラーとして本文の文言がそのまま見える。このときの記録:

```
2026-10-03T13:18:31.870020Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_active_ghost_list" ghost="" is_error=false text=""
2026-10-03T13:18:31.871051Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=38 status=200 path=/api/mcp/v1
2026-10-03T13:18:59.856838Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:18:59.860904Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=39 status=200 path=/api/mcp/v1
2026-10-03T13:18:59.912914Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:18:59.914017Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=40 status=200 path=/api/mcp/v1
2026-10-03T13:18:59.987896Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="" is_error=true text="NG:Cannot find active ghost from specified name"
2026-10-03T13:18:59.988882Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=41 status=200 path=/api/mcp/v1
```

このセッションの 4 本（`id=38`〜`41`）がどの形で届いたか（`logs\run1.log` で確かめた）:

- 4 本とも、直前の rmcp の `Service initialized as server` の行が `protocol_version: ProtocolVersion("2025-03-26")` だった（curl の要求の同じ行は `2026-07-28`。2 つの形を分けるのはこの値だけ）。
  無状態版（`2026-07-28`・本文に `_meta`）の形ではない。curl の無状態版の要求の同じ行は、どれも `ProtocolVersion("2026-07-28")` だった
  （同じ行の `client_info` は、どちらも rmcp が自分で埋めた `name: "rmcp"`・`version: "3.5.0"` で、呼び手の名前ではない）。
  `dispatch` の状態はどれも 200。
- この走行の間、このセッションは areka へ `tools/list` も `server/discover` も `initialize` も送っていない（記録の `tools/list` は
  curl の `id=101` と、`claude mcp get` と窓なしの Claude Code の `id=0` の 3 件だけ・`initialize` は 0 件）。
  このセッションが下の「追加の 1 項目」で照らした `inputSchema` は、この走行より前に受け取っていたものである。
  中身（`sakurascript` の `"required":["script"]` と各欄の説明）は、⑴ で比べた一覧（SSP の保存した JSON＝areka の `tools/list`）と同じ。

```
2026-10-03T13:18:31.863237Z  INFO actor{actor=mcp}:serve_inner: rmcp::service: Service initialized as server peer_info=Some(InitializeRequestParams { meta: None, protocol_version: ProtocolVersion("2025-03-26"), capabilities: ClientCapabilities { ... }, client_info: Implementation { name: "rmcp", title: None, version: "3.5.0", ... } })
```

## 追加の 1 項目: `script` を抜いた `sakurascript`

### Claude Code から（エージェントに見えた文）

`mcp__ssp__sakurascript` を `{"ghost_name":"えも？？"}` だけで呼んだ。エージェントに見えた文（そのまま）:

```
MCP error -32602: Input validation error: Invalid arguments for tool sakurascript: [
  {
    "expected": "string",
    "code": "invalid_type",
    "path": [
      "script"
    ],
    "message": "Invalid input: expected string, received undefined"
  }
]
```

- 欠けている欄の名前（`script`）と、文字列が要るのに無いという理由が、エージェントに見えた。
- この呼び出しは areka に届いていない。記録には、この時刻に `sakurascript` の行も `tools/call` の行も無い（13:18:31 の `id=38` の次は 13:18:59 の `id=39`＝上の `get_status`）。
  Claude Code が送る前に、受け取った `inputSchema`（`"required":["script"]`）に照らして自分で拒んでいる。
- したがって、実機確認で使った Claude Code のセッション（版未確認・2025-03-26 の形で接続）では、areka の HTTP 400 と `message`
  （`missing required argument: script`）はエージェントの目に入らない。Claude Code 2.1.283 が無状態版の経路でつないだときの見え方は確かめていない。
- この結果は、差の一覧（`doc/ssp-mcp/transport-diff-areka.md`）の「未知のツール名・必須引数の欠落」の行の判定の欄に書いた。
  書いた内容: 実機確認で使った Claude Code のセッション（版未確認・2025-03-26 の形で接続）は、必須の欄の欠落を送る前に自分で
  inputSchema に照らして拒み（`Input validation error`）、要求は areka に届かなかった。エージェントには Claude Code 自身の理由の文が見え、
  areka の 400 と `message` は見えない。2.1.283・無状態版の経路での見え方は未確認。

### 同じ要求を curl で送った場合（areka が返すもの）

```
## id=107 sakurascript  arguments={"ghost_name":"えも？？"}
HTTP/1.1 400 Bad Request
content-type: application/json
{"jsonrpc":"2.0","id":107,"error":{"code":-32602,"message":"missing required argument: script"}}
```

記録:

```
2026-10-03T13:17:27.790424Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::handler: MCP: 引数が inputSchema に合わない tool=sakurascript reason=missing required argument: script
2026-10-03T13:17:27.790862Z  WARN actor{actor=mcp}:serve_inner: rmcp::service: response error id=107 error=ErrorData { code: ErrorCode(-32602), message: "missing required argument: script", data: None }
2026-10-03T13:17:27.791260Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=107 status=400 path=/api/mcp/v1
```

値は 4.3 で測った値（差の一覧の「未知のツール名・必須引数の欠落」の行）と同じ。`WARN` は rmcp が出す行で、`areka_mcp` の行ではない。

## ⑸ 呼んでいる間も描画と会話が止まらない

### 起動の会話の最中の 6 本

起動のあいさつの会話（`talk_id=1`）は 13:17:12.83 に始まり 13:17:30.03 に終わった。curl の 6 本（`id=102`〜`107`）は 13:17:27.06〜27.79 で、この会話の最中だった。

```
2026-10-03T13:17:12.828132Z  INFO actor{actor=kanade}: kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1
2026-10-03T13:17:26.636999Z  INFO actor{actor=emo-text}: areka_emo_present::presenter::show: apply(ShowSurface): 表示・マスクを更新 target_id=TargetId(0) surface_id=1000 ...
2026-10-03T13:17:27.066789Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_active_ghost_list" ghost="" is_error=false text=""
  …（id=103〜107）
2026-10-03T13:17:30.025777Z  INFO actor{actor=kanade}: kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
```

### 25 秒続けて呼んだ間（64 本）

13:19:25.98〜13:19:50.43 に、`get_status`（`ghost_name` は `えも？？`）を間を空けずに 64 本続けて送った。64 本とも `NG:not implemented yet` が返った。

この 25 秒の間の記録:

| 見たもの | 件数 |
|---|---|
| 「ツールに答えた」の `debug!`（`get_status`） | 64 |
| 面の表示の更新（`apply(ShowSurface)`） | 22 |
| 表情のアニメーション（`areka_seriko`） | 17 |

この間に、emo2 の定時の会話（`OnSecondChange`・`talk_id=7`）が始まって終わった（13:19:33.59〜13:19:39.79・約 6.2 秒）。
その前後の会話（`talk_id=2`〜`6`）も 1 回 5〜7 秒で、長さは変わっていない。`talk_id=7` の間に areka は 10 本に答えた。

```
2026-10-03T13:19:33.590833Z  INFO actor{actor=kanade}: kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=7 origin="OnSecondChange"
2026-10-03T13:19:35.182566Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:19:35.574793Z  INFO actor{actor=seriko}: areka_seriko::looper: seriko: loop 抽選発火（再生開始・先頭コマから・要件 2.1/2.2） scope="1" slot=Shell ...
2026-10-03T13:19:35.575962Z  INFO actor{actor=emo-text}: areka_emo_present::presenter::show: apply(ShowSurface): 表示・マスクを更新 target_id=TargetId(2) surface_id=2100 ...
2026-10-03T13:19:35.707028Z  INFO actor{actor=seriko}: areka_seriko::looper: seriko: loop 停止（負 surface でベース復帰・要件 4.3） scope="1" slot=Shell ...
2026-10-03T13:19:35.874798Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:19:36.481036Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:19:36.988879Z  INFO actor{actor=emo-text}: areka_emo_present::presenter::show: apply(ShowSurface): 表示・マスクを更新 target_id=TargetId(2) surface_id=2110 ...
2026-10-03T13:19:37.032570Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:19:37.034039Z  INFO actor{actor=emo-text}: areka::emo2_boot::balloon_visibility::phase: [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="content" visible=true
2026-10-03T13:19:39.793712Z  INFO actor{actor=kanade}: kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
```

性能の報告（5 秒ごと）でも、この間に UI のスレッド（`role=ui`）の CPU 時間は 5 秒ごとに約 0.9〜1.3 秒ずつ増え続けている（止まっていない）。
MCP のスレッドは、送り始める前（snap=27・13:19:24）から送り終えた直後（snap=32・13:19:49）までで約 0.30 秒増えた。以後、終了まで増えていない:

```
DEBUG areka::perf: perf(thread): スレッド別 CPU snap=27 t_s=136 tid=1928 name=mcp role=actor:mcp cpu_us=62500 ...
DEBUG areka::perf: perf(thread): スレッド別 CPU snap=27 t_s=136 tid=2948 name=main role=ui cpu_us=30781250 ...
DEBUG areka::perf: perf(thread): スレッド別 CPU snap=32 t_s=161 tid=1928 name=mcp role=actor:mcp cpu_us=359375 ...
DEBUG areka::perf: perf(thread): スレッド別 CPU snap=32 t_s=161 tid=2948 name=main role=ui cpu_us=36484375 ...
```

画面を目で見ての確認はしていない（記録の行で判定した）。

## ⑹ 呼び出しごとの記録の行

`tools/call` は全部で 74 本（curl 6 本・続けて送った 64 本・エージェントのセッションから 4 本）。`areka_mcp::dispatch` の「要求に応えた」も
`method=tools/call` が 74 件。そのうち引数の検査を通った 73 本に、`areka_mcp::tools::bridge` の「ツールに答えた」の `debug!` が 1 件ずつある
（`tool`・`ghost`・`is_error`・`text` 付き）。残りの 1 本（`id=107`・`script` 抜き）は道具に届く前に拒まれ、`areka_mcp::handler` の
「引数が inputSchema に合わない」の `debug!` 1 件が残る。

`ghost` は解決したゴーストの名前で、名前を解決しない `get_active_ghost_list` と、解決に失敗した `no-such-ghost` では空（設計のとおり）。
`text` は `is_error` のときの本文で、`is_error=false` のときは空。curl の 6 本の行:

```
2026-10-03T13:17:27.066789Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_active_ghost_list" ghost="" is_error=false text=""
2026-10-03T13:17:27.216784Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:17:27.358605Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:17:27.508165Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=true text="NG:not implemented yet"
2026-10-03T13:17:27.650110Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="" is_error=true text="NG:Cannot find active ghost from specified name"
2026-10-03T13:17:27.790424Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::handler: MCP: 引数が inputSchema に合わない tool=sakurascript reason=missing required argument: script
```

フルパスで渡した `id=104`・`105` の行でも `ghost="えも？？"`（フルパスから emo2 に解決した）。

## 終わり方と後始末

- 自動終了（600 秒）で終わり、終了コード 0。終了の行:

  ```
  2026-10-03T13:27:10.532816Z  INFO actor{actor=emo-text}: areka: smoke 自動 close: ゴースト窓を despawn しました count=4
  2026-10-03T13:27:10.600815Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"
  2026-10-03T13:27:10.612241Z  INFO actor{actor=emo-text}: areka_mcp::server: MCP: 待受を閉じた addr=127.0.0.1:9801
  ```

  終了後は 9801〜9839 のどれにも待受が無い。
- `WARN` の段は全体で 5 件: `rmcp::service` の `response error id=107`（上記）・自動終了の `force_quit`・emo2 の画像とバルーン定義についての既存の 3 件（`areka_emo_atlas` 2 件・`areka_emo_text` 1 件）。`areka_mcp` の `WARN` は 0 件。
- 標準エラー出力は `[helper] SHIORI 初期化の入口: loadu` の 1 行だけ。
- Claude Code の登録: `claude mcp remove areka -s project` で外し（`.mcp.json` は `{"mcpServers": {}}`）、`target\signoff-ent\proj\` をフォルダごと消した。
  利用者の設定 `~/.claude.json` に `signoff-ent` の記録は 0 件、`~/.claude\projects\` に `signoff` を含むフォルダは 0 個（実施の前も 0 件・0 個）。
  窓なしの Claude Code は `--no-session-persistence` で起こした。

## 確かめられなかったこと

- 窓なしの Claude Code（CLI 2.1.283）にモデルを通して道具を呼ばせること: ログインの期限切れで止まった。⑵〜⑷ と追加の 1 項目は、
  この記録を書いたエージェントのセッション（Claude Code）から同じ areka を呼んで、エージェントに見えたものを書いた。このセッションの Claude Code の版は確かめていない。
- Claude Code 2.1.283 が無状態版（`2026-07-28`）の経路でつないだときにも、必須の欄の欠落を送る前に自分で拒むかどうか。
  拒むのを見たのは、2025-03-26 の形で接続していたこのエージェントのセッションだけである。
- 目で見ての描画の確認（⑸ は記録の行と性能の報告で判定した）。
