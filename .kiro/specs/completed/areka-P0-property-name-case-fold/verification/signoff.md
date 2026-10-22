# 実機確認の記録（タスク 3.2・要件 6.1）

- **実施日**: 2026-10-22（記録の時刻は UTC）
- **判定に使った版**: コミット `ef9b556b`（タスク 3.3 の後）。配布物の `BUILD-INFO.txt` は `version=0.0.3` `arch=x64` `commit=ef9b556` `dirty=0` `built=2026-10-22T14:56:32Z` `script=tools/package.ps1 2.1.0`
- **ビルド**: `pwsh -NoProfile -File tools/package.ps1`（全段が緑・終了コード 0）→ `target\package\areka-0.0.3-x64.zip`（10,429,962 バイト）
- **展開先**: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-property-case-fold-595ab3\target\signoff-case-fold\x\`（ワークツリーの `target\` の下・絶対パス）。記録は同じ `target\signoff-case-fold\logs\`（`run1.log`・`run1.stderr.log`・`curl1.txt`・`req301.json`〜`req303.json`）
- **起動のしかた**: `Start-Process -PassThru` で展開先の `areka.exe` を起こした（作業フォルダは展開先・標準出力を `run1.log`、標準エラーを `run1.stderr.log` へ）。終わらせ方は有界の自動終了 `AREKA_APP_SMOKE_EXIT_MS`。起こしたプロセスは pid 9272 の 1 つで、自分で起こしたプロセス以外は止めていない
- **環境変数**（付けたもの）:

  ```
  AREKA_APP_SMOKE_EXIT_MS=90000
  AREKA_NO_ALERT=1
  NO_COLOR=1
  RUST_LOG=info,areka_mcp=debug
  ```

- **起動のときに残っていた環境変数**: 起動の前に `AREKA_*`／`WINTF_*` を空にしようとしたが、`Set-Item env:<名前> ([NullString]::Value)` では消えず、直後に数えると 2 個残っていた。名前は `AREKA_IMPL_WATCH_HOME` と `AREKA_MCP_HOME`（どちらもセッションから受け継いだもの・値はこの走行では見ていない）。起動のときにこの 2 つがあったとして記録する。前者を読むのは `crates/areka-impl-watch`（机の道具）だけ、後者を読むのは `tools/mcp-bridge-install.ps1` だけで、`crates` の下の areka 本体のコードはどちらも読まない。`run1.log` にもこの 2 つの名前は出ていない（0 件）。`WINTF_*` は 0 個
- **SSP**: この走行の間は動いていなかった（起動前に 9801・9821 の待受は 0）。areka は既定の 9801 で待ち受けた:

  ```
  2026-10-22T14:58:24.470113Z  INFO areka_mcp::server: MCP: 待受を始めた url=http://127.0.0.1:9801/api/mcp/v1
  ```

- **使った道具**: curl（Git for Windows 同梱）。`mcp-get-property` の実機確認と同じ無状態版の形（`MCP-Protocol-Version: 2026-07-28`・`Mcp-Method: tools/call`・`Mcp-Name: get_property`・本文の `params._meta` に `io.modelcontextprotocol/protocolVersion="2026-07-28"`・`clientCapabilities={}`・`clientInfo`）。本文は UTF-8 のままファイルから送った（`--data-binary @…`）

## 判定

| # | 呼び方 | 期待 | 結果 |
|---|---|---|---|
| ⑴ | `property_name: "baseware.name"`（`ghost_name` なし） | `areka`・`isError: false` | **合** |
| ⑵ | `property_name: "BASEWARE.NAME"`（`ghost_name` なし） | `areka`・`isError: false` | **合** |
| ⑶ | `property_name: " baseware.name"`（先頭に半角の空白 1 つ・`ghost_name` なし） | `NG:Cannot find such property name.`・`isError: true` | **合** |
| 記録 | 各呼び出しに「MCP: ツールに答えた」の `debug!` が 1 件ずつ | 3 件 | **合**（3 件） |
| 記録 | `ERROR` の段の行 | 0 件 | **合**（`run1.log`・`run1.stderr.log` とも 0 件） |

呼んだのは起動の通知（`OnFirstBoot`。起動種別が台詞を返したので `OnBoot` は送られていない＝記録の `event="boot_type_script"`・14:58:30）と会話の開始（14:58:31）の後の 14:58:55〜14:58:56 で、起動直後の短い窓は避けた。

## 答え（`curl1.txt`）

```
## id=301 get_property  arguments={"property_name":"baseware.name"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":301,"result":{"resultType":"complete","content":[{"type":"text","text":"areka"}],"isError":false}}

## id=302 get_property  arguments={"property_name":"BASEWARE.NAME"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":302,"result":{"resultType":"complete","content":[{"type":"text","text":"areka"}],"isError":false}}

## id=303 get_property  arguments={"property_name":" baseware.name"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":303,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:Cannot find such property name."}],"isError":true}}
```

本文の全文（3 つとも `arguments` の値だけが違う）:

```
{"jsonrpc":"2.0","id":301,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{},"io.modelcontextprotocol/clientInfo":{"name":"signoff-curl","version":"1"}},"name":"get_property","arguments":{"property_name":"baseware.name"}}}
```

## 記録（`run1.log` から該当の行）

```
2026-10-22T14:58:55.229305Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_property" ghost="えも？？" is_error=false text=""
2026-10-22T14:58:55.230739Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=301 status=200 path=/api/mcp/v1
2026-10-22T14:58:55.779811Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_property" ghost="えも？？" is_error=false text=""
2026-10-22T14:58:55.780736Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=302 status=200 path=/api/mcp/v1
2026-10-22T14:58:56.187751Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_property" ghost="えも？？" is_error=true text="NG:Cannot find such property name."
2026-10-22T14:58:56.189032Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=303 status=200 path=/api/mcp/v1
```

- 橋の記録の `text` は `isError: false` の答えでは空になる（橋の既存の振る舞い・本 spec の範囲外）。
- 記録の `ghost` は、`ghost_name` を省いても解決した先の `えも？？` になっている。

`WARN` の段は 4 件。3 件は emo2 の素材とバルーンの定義についての既存の記録（`areka_emo_atlas` の全透明の element 2 件・`areka_emo_text` の折返し基準 1 件）、1 件は有界の自動終了の続きで kanade が出す `event="force_quit" reason="user"`（14:59:56）。どれも本 spec の処理とは関係しない。

## 終わり方

有界の自動終了（90 秒）で終わった。`smoke 自動 close ゲート有効 … delay_ms=90000`（14:58:26）→ `smoke 自動 close: ゴースト窓を despawn しました count=4`（14:59:56）→ kanade の `強制終了指示——終了系列（Forced）へ直行 event="force_quit"`（WARN）。記録の末尾は次の 3 行で、プロセスの終了コードは 0（起こしたシェルで `WaitForExit` して取った）。

```
2026-10-22T14:59:56.934466Z  INFO actor{actor=emo-text}: ghost-shutdown: ghost shutdown sequence completed
2026-10-22T14:59:56.940752Z  INFO actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"
2026-10-22T14:59:56.949740Z  INFO actor{actor=emo-text}: areka_mcp::server: MCP: 待受を閉じた addr=127.0.0.1:9801
```

`ERROR` の段の数え方: 段の欄が `ERROR` の行（`^<時刻> +ERROR `）を数えて 0 件（`run1.log` 289 行・`run1.stderr.log`）。`run1.stderr.log` は補助 exe の 1 行（`[helper] SHIORI 初期化の入口: loadu`）だけ。
