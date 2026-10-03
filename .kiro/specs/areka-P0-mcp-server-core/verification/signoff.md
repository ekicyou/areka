# 実機確認の記録（タスク 6.2・要件 9.5・9.6）

- **実施日**: 2026-10-03
- **判定に使った版**: コミット `ce8ba54`（直しのコミット `52c9136f` を含む）。配布物の `BUILD-INFO.txt` は `commit=ce8ba54` `dirty=0`
- **ビルド**: `pwsh -NoProfile -File tools/package-alpha.ps1`（全段 緑・終了コード 0）→ `target\alpha\areka-alpha-x64-20261003-ce8ba54.zip`
  （組む間だけ、追跡外のこの記録を `target\` へ退けて、未コミットの変更 0 件で組んだ）
- **展開先**: `target\signoff-mcp\b\`（ワークツリーの `target\` の下・絶対パス）。記録は `target\signoff-mcp\logs\`
- **起動のしかた**: `AREKA_*`／`WINTF_*` を全部外してから、`Start-Process -PassThru` で展開先の `areka.exe` を起こした（作業フォルダは展開先）。終わらせ方は有界の自動終了 `AREKA_APP_SMOKE_EXIT_MS`。自分で起こしたプロセス以外は止めていない
- **使った道具**: Claude Code `2.1.283`・curl（Git for Windows 同梱）

## 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | `claude mcp add` で登録し、Claude Code から接続できる | **合**（`✔ Connected`・ツールの一覧の取得も通った） |
| ⑵ | `curl` で `initialize`→`tools/list`→`ping` | **合** |
| ⑶ | `AREKA_MCP_PORT=0` で接続できず `info!` が出る | **合** |
| ⑷ | SSP が 9801 で動いている机で 9821 を同時に待ち受ける | **未確認**（この日は SSP が動いていなかった。下記） |
| 記録 | 要求ごとに `debug!` 1 件・悪い `Origin` に `warn!` 1 件 | **合** |
| 記録 | 性能の報告に `actor:mcp` が出る | **合** |
| 記録 | 待受の開始・閉じたときの `info!`、終了後に 9821 が閉じている | **合** |
| 大きさ | リリースの `areka.exe` の増分 | +1,674,752 バイト（下記） |

**ERROR の行は 0 件**（下の走行 3・走行 4 とも、標準出力・標準エラー出力の両方）。

## 1 度目の走行で見つかった失敗（経緯）

コミット `5a9e1d4` の配布物で 1 度目の確認をしたとき、⑴ が通らなかった:

```
> claude mcp get areka
  Status: ! Connected · tools fetch failed
  Issue: Invalid result for tools/list: [ { "expected": "number", "code": "invalid_type", "path": [ "ttlMs" ], ... }, { "code": "invalid_value", "values": [ "public", "private" ], "path": [ "cacheScope" ], ... } ]
```

Claude Code 2.1.283 は `initialize` を送らず、無状態版（`2026-07-28`）の `server/discover` → `tools/list` を送っていた。
`server/discover` の答えには `ttlMs`・`cacheScope` があったが、`tools/list` の答え
（`{"resultType":"complete","tools":[]}`）には無く、Claude Code がこれを受け付けなかった。

直し: コミット `52c9136f`（無状態版の `tools/list` の答えに `ttlMs: 0` と `cacheScope: "private"` を足す。
旧式の版の答えは変えない。要件 3.15・設計 B-12・差の一覧の「困るので直した」1 行）。以下はすべて直した後の版での結果である。

## 走行 3（既定の番号 9821・記録を細かく）

環境変数:

```
AREKA_APP_SMOKE_EXIT_MS=180000
AREKA_NO_ALERT=1
NO_COLOR=1
RUST_LOG=info,areka_mcp=debug,areka::perf=debug
AREKA_PERF_THREAD_REPORT_SEC=5
```

`areka::perf=debug` は性能の報告を点けるために足した（報告は target `areka::perf` の DEBUG で点く）。
起こしたプロセス番号 6816。起動の前は 9801・9821 とも待受が無かった。起動の 4 秒後:

```
> netstat -ano | grep -E ":9801 |:9821 "
  TCP         127.0.0.1:9821         0.0.0.0:0              LISTENING       6816
```

### ⑵ curl で initialize → tools/list → ping

共通の付け足し: `-H "Content-Type: application/json" -H "Accept: application/json, text/event-stream"`（設計 B-8）。宛先 `http://127.0.0.1:9821/api/mcp/v1`。

```
## initialize  --data '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"curl-signoff","version":"0"}}}'
HTTP/1.1 200 OK
content-type: application/json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"areka-mcp-server","version":"0.0.1"},"instructions":"This server controls areka, ..."}}

## notifications/initialized  --data '{"jsonrpc":"2.0","method":"notifications/initialized"}'
HTTP/1.1 202 Accepted

## tools/list  --data '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":2,"result":{"tools":[]}}

## ping  --data '{"jsonrpc":"2.0","id":3,"method":"ping"}'
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":3,"result":{}}

## 悪い Origin（-H "Origin: http://evil.example" を足した ping）
HTTP/1.1 403 Forbidden
content-type: text/plain; charset=utf-8
Forbidden
```

旧式の版の `tools/list` の答えは直す前と同じ（`ttlMs`・`cacheScope` は付かない）。

無状態版の `tools/list`（Claude Code と同じ形。`-H "MCP-Protocol-Version: 2026-07-28" -H "Mcp-Method: tools/list"`、
本文の `params._meta` に `io.modelcontextprotocol/protocolVersion="2026-07-28"`・`clientCapabilities={}`・`clientInfo`）:

```
HTTP/1.1 200 OK
content-type: application/json
{"jsonrpc":"2.0","id":12,"result":{"resultType":"complete","ttlMs":0,"cacheScope":"private","tools":[]}}
```

### ⑴ Claude Code での登録

利用者の設定を変えないため、`--scope project` で `target\signoff-mcp\proj\` の中だけに登録した（そこに `.mcp.json` ができる）。
プロジェクトの `.mcp.json` に書いたサーバーは、使う許しが出るまで Claude Code が接続を試さないので、
同じフォルダの `.claude\settings.local.json` に `{"enabledMcpjsonServers": ["areka"]}` を書いて許した。
登録済みの他のサーバーへ接続を試さないよう、`claude mcp list` ではなく `claude mcp get areka` で見た。

```
> cd target\signoff-mcp\proj
> claude mcp add --transport http --scope project areka http://127.0.0.1:9821/api/mcp/v1
Added HTTP MCP server areka with URL: http://127.0.0.1:9821/api/mcp/v1 to project config

> claude mcp get areka
areka:
  Scope: Project config (shared via .mcp.json)
  Status: ✔ Connected
  Type: http
  URL: http://127.0.0.1:9821/api/mcp/v1
```

1 度目にあった「tools fetch failed」と「Issue:」の行は出なかった。このとき areka が受けた要求（記録より）:

```
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=server/discover id="server-discover-probe-1" status=200 path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/list id=0 status=200 path=/api/mcp/v1
```

後始末: `claude mcp remove areka -s project` で登録を外し（`.mcp.json` は `{"mcpServers": {}}`）、
`.claude\settings.local.json` とそのフォルダを消した。利用者の設定 `~/.claude.json` に `signoff-mcp` の記録が無いことも確かめた（該当 0 件）。

### 記録（要件 9.6）

`areka_mcp` の行の全部（走行 3）:

```
INFO  areka_mcp::server: MCP: 待受を始めた url=http://127.0.0.1:9821/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=initialize id=1 status=200 path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=notifications/initialized id=- status=202 path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/list id=2 status=200 path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=ping id=3 status=200 path=/api/mcp/v1
WARN  actor{actor=mcp}: areka_mcp::dispatch: MCP: Origin を拒んだ origin=http://evil.example path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=- id=- status=403 path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/list id=12 status=200 path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=server/discover id="server-discover-probe-1" status=200 path=/api/mcp/v1
DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/list id=0 status=200 path=/api/mcp/v1
INFO  actor{actor=emo-text}: areka_mcp::server: MCP: 待受を閉じた addr=127.0.0.1:9821
```

- 送った要求は 8 本（curl 6 本・Claude Code 2 本）で、`debug!` も 8 件。1 本に 1 件。
- 悪い `Origin` の `warn!` は 1 件（値つき）。`areka_mcp` の `WARN` は全体でこの 1 件だけ。
- 閉じた `info!` に付く `actor{actor=emo-text}` は、閉じる処理が走ったスレッドの文脈の名前で、
  同じ終わり際に出る既存の行（`ghost-shutdown: ...` など）にも同じものが付いている。

性能の報告（5 秒ごと・全 18 回の全部に mcp の行がある）:

```
DEBUG areka::perf: perf(thread): スレッド別 CPU snap=1 t_s=5 tid=20476 name=mcp role=actor:mcp cpu_us=0 kernel_us=0 user_us=0
DEBUG areka::perf: perf(thread): スレッド別 CPU snap=3 t_s=15 tid=20476 name=mcp role=actor:mcp cpu_us=15625 kernel_us=15625 user_us=0
DEBUG areka::perf: perf(thread): スレッド別 CPU snap=18 t_s=89 tid=20476 name=mcp role=actor:mcp cpu_us=15625 kernel_us=15625 user_us=0
```

要求の無い間の CPU は増えていない（snap=3 から終了直前の snap=18 まで 15,625 µs のまま）。

### 終わり方と、終了後に 9821 が閉じていること

走行 3 は自動終了（180 秒）より前の 89 秒で終わった。記録によると、机の前の人がゴーストを右クリックし、
メニューの「終了」を選んでいた（自分は操作していない）:

```
INFO  areka::menu::trigger: [menu] selected event="menu_selected" scope=1 frame=Close id=8
INFO  kanade: OnClose 応答スクリプト——close talk を再生起動し完了を待機 event="close_talk_start" talk_id=5
INFO  kanade: reason=Quit——終了系列（Quit）へ event="talk_done_quit" talk_id=5
INFO  areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4
INFO  areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"
INFO  actor{actor=emo-text}: areka_mcp::server: MCP: 待受を閉じた addr=127.0.0.1:9821
```

利用者の通常の終わり方で、待受も閉じた。プロセスは終了コード 0。その後:

```
> netstat -ano | Select-String ':9821 '
  TCP    127.0.0.1:62333        127.0.0.1:9821         TIME_WAIT       0
  ...（curl 側の閉じた接続の名残 6 本だけ・LISTENING の行は無い）

> curl -sS -m 5 http://127.0.0.1:9821/api/mcp/help
curl: (7) Failed to connect to 127.0.0.1:9821 after 2005 ms: Could not connect to server
```

## 走行 4（`AREKA_MCP_PORT=0`）— ⑶

環境変数: `AREKA_APP_SMOKE_EXIT_MS=20000`・`AREKA_NO_ALERT=1`・`NO_COLOR=1`・`RUST_LOG=info,areka_mcp=debug`・`AREKA_MCP_PORT=0`。
プロセス番号 33360・20 秒後に自動終了で終わり、終了コード 0。起動の 6 秒後（動いている間）に:

```
> netstat -ano | Select-String ':9821 .*LISTEN'
（出力なし）

> curl.exe -sS -m 5 -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" --data '{"jsonrpc":"2.0","id":1,"method":"ping"}' http://127.0.0.1:9821/api/mcp/v1
curl: (7) Failed to connect to 127.0.0.1:9821 after 2012 ms: Could not connect to server
```

記録（`areka_mcp` の行はこの 1 件だけ）:

```
INFO  areka_mcp::server: MCP: 待ち受けない（AREKA_MCP_PORT が 0）
```

ゴーストはいつもどおり起動し、自動終了の行（`smoke 自動 close: ゴースト窓を despawn しました count=4`）まで進んだ。

## ⑷ SSP との同時の待受 — 未確認

この日は SSP が動いていなかった（`tasklist` に `ssp.exe` が無い・9801 の待受も無い）。
指示に従い SSP を起こしていない。確かめられたのは「9801 が無い机で 9821 が待ち受ける」ことだけ（走行 3 の netstat）。
**SSP が 9801 で待ち受けている机での同時の待受は確かめていない。**

確かめ方（利用者が行う）:

1. SSP を起動し、MCP の待受（9801）を有効にしておく。
2. `target\signoff-mcp\b\areka.exe` を起動する。
3. `netstat -ano | findstr ":9801 :9821"` を打ち、`127.0.0.1:9801` と `127.0.0.1:9821` の両方に `LISTENING` が出ることを見る。
4. areka をメニューの「終了」で閉じる。

## リリースの `areka.exe` の増分

| | コミット | `areka.exe` の大きさ |
|---|---|---|
| 比べる元 | `7393c30` | 6,594,048 バイト |
| 本 spec | `ce8ba54` | 8,268,800 バイト |
| 増分 | | **+1,674,752 バイト（約 +1.60 MiB・+25.4%）** |

測り方: どちらも `tools/package-alpha.ps1`（同じ版 1.0.0・`rustflags=-C target-feature=+crt-static`）が作った配布 zip の
中の `areka.exe` を比べた。比べる元は、別のワークツリーに残っていた
`areka-alpha-x64-20261003-7393c30.zip`（`BUILD-INFO.txt` に `dirty=0`）。`7393c30` は本 spec の分かれ目 `1ce4c74e` の
すぐ後の文書だけのコミットで、`git diff --stat 1ce4c74e 7393c30 -- . ':!.kiro' ':!doc'` が空
＝ソースは分かれ目と同じ。本 spec と分かれ目の差（`.kiro`・`doc` を除く）は `crates/areka-mcp/` の新設と
`crates/areka/Cargo.toml`・`main.rs`・`Cargo.lock` 等の 21 ファイルなので、増分はこの spec の分とみてよい。
配布 zip の大きさは 7,564,054 → 8,214,690 バイト（+650,636 バイト）。

## 確かめられなかったこと

- ⑷ SSP が 9801 で動いている机での同時の待受: SSP が動いていなかったため確かめていない（確かめ方は上記）。
- `claude mcp list` での表示: 登録済みの他のサーバーへ接続を試すのを避けるため、直した後は `claude mcp get areka` だけで見た（`get` は同じ接続の確かめを 1 つのサーバーにだけ行う）。
