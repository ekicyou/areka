# 実機確認の記録（タスク 4・要件 4.6）

- **実施日**: 2026-10-04
- **判定に使った版**: コミット `584feaa1`（タスク 2.2 の処理の後）。配布物の `BUILD-INFO.txt` は `commit=584feaa` `dirty=0` `script=tools/package.ps1 2.0.0`
- **ビルド**: `pwsh -NoProfile -File tools/package.ps1`（全段 緑・終了コード 0）→ `target\package\areka-0.0.1-x64.zip`（8,384,623 バイト）
- **展開先**: `target\signoff-prop\x\`（ワークツリーの `target\` の下・絶対パス）。記録は `target\signoff-prop\logs\`（`run1.log`・`run1.stderr.log`・`curl1.txt`・`req201.json`〜`req204.json`）
- **起動のしかた**: `AREKA_*`／`WINTF_*` は起動前に 0 個。`Start-Process -PassThru` で展開先の `areka.exe` を起こした（作業フォルダは展開先）。終わらせ方は有界の自動終了 `AREKA_APP_SMOKE_EXIT_MS`。自分で起こしたプロセス以外は止めていない
- **環境変数**:

  ```
  AREKA_APP_SMOKE_EXIT_MS=300000
  AREKA_NO_ALERT=1
  NO_COLOR=1
  RUST_LOG=info,areka_mcp=debug
  ```

- **SSP**: この走行の間は動いていなかった（起動前に 9801・9821 の待受は 0）。areka は既定の 9801 で待ち受けた:

  ```
  2026-10-04T06:20:11.851484Z  INFO areka_mcp::server: MCP: 待受を始めた url=http://127.0.0.1:9801/api/mcp/v1
  ```

- **使った道具**: curl（Git for Windows 同梱）。`mcp-tool-entrances` の実機確認と同じ無状態版の形（`MCP-Protocol-Version: 2026-07-28`・`Mcp-Method: tools/call`・`Mcp-Name: <道具の名前>`・本文の `params._meta`）。本文は UTF-8 のままファイルから送った（`--data-binary @…`）

## 判定

| # | 呼び方 | 期待 | 結果 |
|---|---|---|---|
| ⑴ | `property_name: "baseware.name"`（`ghost_name` なし） | `areka`・`isError: false` | **合** |
| ⑵ | `property_name: "no.such.thing"`（`ghost_name` なし） | `NG:Cannot find such property name.`・`isError: true` | **合** |
| ⑶ | `property_name: "baseware.name"`・`ghost_name: "えも？？"`（`get_active_ghost_list` の答えをそのまま） | `areka`・`isError: false`（記録だけ） | `areka`・`isError: false` |
| 記録 | 各呼び出しに「MCP: ツールに答えた」の `debug!` が 1 件ずつ | 4 件（⑴⑵⑶＋`get_active_ghost_list`） | **合**（4 件） |
| 記録 | `ERROR` の段の行 | 0 件 | **合**（`run1.log`・`run1.stderr.log` とも 0 件） |

呼んだのは起動の通知（`OnFirstBoot`。起動種別が台詞を返したので `OnBoot` は送られていない＝記録の `event="boot_type_script"`）と会話の開始の後（06:20:30 以降）で、起動直後の短い窓（design の決定 5）は避けた。

## 答え（`curl1.txt`）

```
## id=201 get_property  arguments={"property_name":"baseware.name"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":201,"result":{"resultType":"complete","content":[{"type":"text","text":"areka"}],"isError":false}}

## id=202 get_property  arguments={"property_name":"no.such.thing"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":202,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:Cannot find such property name."}],"isError":true}}

## id=203 get_active_ghost_list  arguments={}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":203,"result":{"resultType":"complete","content":[{"type":"text","text":"えも？？"}],"isError":false}}

## id=204 get_property  arguments={"property_name":"baseware.name","ghost_name":"えも？？"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":204,"result":{"resultType":"complete","content":[{"type":"text","text":"areka"}],"isError":false}}
```

## 記録（`run1.log` から該当の行）

```
2026-10-04T06:20:30.810621Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_property" ghost="えも？？" is_error=false text=""
2026-10-04T06:20:30.810938Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=201 status=200 path=/api/mcp/v1
2026-10-04T06:20:30.868911Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_property" ghost="えも？？" is_error=true text="NG:Cannot find such property name."
2026-10-04T06:20:30.869192Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=202 status=200 path=/api/mcp/v1
2026-10-04T06:20:30.919212Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_active_ghost_list" ghost="" is_error=false text=""
2026-10-04T06:20:30.919479Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=203 status=200 path=/api/mcp/v1
2026-10-04T06:20:37.127648Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_property" ghost="えも？？" is_error=false text=""
2026-10-04T06:20:37.127826Z DEBUG actor{actor=mcp}: areka_mcp::dispatch: MCP: 要求に応えた method=tools/call id=204 status=200 path=/api/mcp/v1
```

- 橋の記録の `text` は `isError: false` の答えでは空になる（`get_active_ghost_list` の行も同じ）。橋の既存の振る舞いで、本 spec は変えていない（要件 4.5・橋の記録は本 spec の範囲外）。
- 記録の `ghost` は、`ghost_name` を省いた ⑴⑵ でも解決した先の `えも？？` になっている。
- `get_property` の処理が出す `warn!`（`mcp_get_property_unavailable`）は 0 件（実行系のある本番の経路では来ない道）。

`WARN` の段は 4 件。3 件は emo2 の素材とバルーンの定義についての既存の記録（`areka_emo_atlas` の全透明の element 2 件・`areka_emo_text` の折返し基準 1 件）、1 件は有界の自動終了の続きで kanade が出す `event="force_quit" reason="user"`（06:25:12）。どれも本 spec の処理とは関係しない。

## 終わり方

有界の自動終了（300 秒）で 06:25:12 に終わった。kanade の `強制終了指示——終了系列（Forced）へ直行 event="force_quit"`（WARN）の後、記録の末尾は「ghost shutdown sequence completed」→「きれいに終わったので起動中の印を消しました event="session_mark_cleared"」→「MCP: 待受を閉じた addr=127.0.0.1:9801」。終了コードは、起こしたシェルとは別のシェルから待ったため取れていない（プロセスの終わりと記録の正常終了の行で確かめた）。

`ERROR` の段の数え方: 段の欄が `ERROR` の行を数えて 0 件（`run1.log`・`run1.stderr.log`）。大小を区別しない `error` の一致は 4 件あるが、どれも橋の記録の欄の名前 `is_error` で、段ではない。`run1.stderr.log` は補助 exe の 1 行（`[helper] SHIORI 初期化の入口: loadu`）だけ。
