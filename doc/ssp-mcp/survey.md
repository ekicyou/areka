# SSP 内蔵 MCP サーバの実測記録（2026-09-29・SSP 2.9.05）

areka へ SSP の MCP サーバを移植する spec 群（`.kiro/steering/roadmap.md`「SSP MCP の移植」）の**事実の正本**。全て開発者の机の SSP 2.9.05（`C:\wintools\ssp`・ゴースト Emily/Phase4.5 起動中）へ `curl` と MCP クライアントで当てて得た。仕様文書は ukadoc の「その他の機能」（<https://ssp.shillest.net/ukadoc/ssphelp/other.html>）と SSTP 仕様（<https://ssp.shillest.net/ukadoc/manual/spec_sstp.html>）。**ukadoc MCP（仕様検索）には MCP の節が索引されていない**（`search_docs("MCP")` は 0 件）＝仕様の出典は上の 2 ページと本実測。

`tools/list` の応答全文は同じフォルダの [tools-list-ssp-2.9.05.json](tools-list-ssp-2.9.05.json)（名前・title・description・inputSchema を逐語で保存）。

## 1. 構成

- SSP 本体が **`127.0.0.1:9801`（SSTP over HTTP と同じ口）** で HTTP を受ける。MCP は `POST /api/mcp/v1`。
- `GET /api/mcp/help` は設定手順の HTML（`claude_desktop_config.json` の断片）、`GET /api/mcp/v1` は手で JSON-RPC を打つフォーム（`enctype="text/plain"`・`__dummy__mcp__form__flag__=1` と `mcp=<JSON>`）。フォーム送信も通る（実測で `ping` が 200）。
- `data\mcp.exe`（94 KB）は **stdio ⇔ HTTP の橋**。`127.0.0.1:9801` と `POST /api/mcp/v1` を焼き込んでいる（文字列で確認）＝ポートは変えられない。
- **橋が要るのは Claude Desktop**: Claude Code・Cursor は HTTP の MCP サーバを直接登録できる（`claude mcp add --transport http <名> http://127.0.0.1:<port>/api/mcp/v1`）。Claude Desktop は書けない（§6）＝areka も中継を持つ（`mcp-stdio-bridge`）。
- 能力は `tools` のみ。`resources/list`・`resources/templates/list`・`prompts/list` は空配列で 200（`instructions` の「read-only resources」は実体が無い）。

## 2. 輸送とプロトコル

> **areka での扱い（2026-09-29 開発者判断）**: areka はプロトコルを自作せず公式 Rust SDK `rmcp` を使う。本節の表は SSP の振る舞いの記録であり、areka の要件ではない（rmcp の振る舞いとの差は `mcp-server-core` が一覧にし、クライアントが困る行だけ直す）。SSP と一致させるのは §3 のツールの名前・引数・結果の文字列。

| 項目 | SSP の振る舞い（実測） |
|---|---|
| 応答 | 常に `application/json` の単発応答。SSE・セッション ID（`Mcp-Session-Id`）は使わない＝無状態 |
| 通知（id なし） | `202 Accepted`・本文なし |
| 版の交渉 | `supportedVersions` = `2026-07-28`・`2025-11-25`・`2025-06-18`・`2025-03-26`・`2024-11-05`。`initialize` に既知の版を渡せばその版を返し、未知の版なら最新の旧式版 `2025-11-25` を返す |
| `MCP-Protocol-Version` ヘッダ | 旧式 4 版と無しは素通し。それ以外（`2026-07-28` や未知の値）は**無状態版の検査**に入り、`params._meta` に `io.modelcontextprotocol/protocolVersion` と `io.modelcontextprotocol/clientCapabilities` が無ければ 400・`-32602`「Invalid params: _meta requires protocolVersion and clientCapabilities」 |
| 無状態版（`2026-07-28`） | `server/discover` が `supportedVersions`・`capabilities`・`instructions`・`ttlMs: 3600000`・`cacheScope: "private"`・`resultType: "complete"`・`_meta.io.modelcontextprotocol/serverInfo` を返す。`Mcp-Method` ヘッダ（と `tools/call` では `Mcp-Name`）が本文と食い違うと 400・`-32020`「Header mismatch: Mcp-Method／Mcp-Name」。結果に `resultType: "complete"` と `_meta.serverInfo` が付く。無状態版で `initialize` を送ると 400（`_meta` 必須） |
| `ping` | `{}` |
| 未知メソッド | 200・`-32601`「Method not found」 |
| JSON でない本文 | 400・`-32700`「Parse error」・`id: null` |
| バッチ（配列） | 400・`-32600`「Invalid Request」 |
| 未知のツール名・必須引数の欠落 | 200・`-32602`「Invalid params」（`isError` の結果でなく JSON-RPC エラー） |
| エラーの形 | `{"code","message","data"}`・`data` は `message` と同じ文字列 |
| `Origin` | 無し・`http://localhost:*` は通す。`http://evil.example`・`null` は **403**・`-32600`「Forbidden: invalid origin」・`id` なし |
| `Host` | 検査しない（`Host: evil.example` でも 200）＝DNS 再束縛への備えは Origin だけ |
| serverInfo | `{"name":"ssp-mcp-server","version":"2.9.05"}` |

## 3. ツール 10 本（結果の形）

結果は全て `content: [{type:"text", text}]`（dump 系はこれに `{type:"image", data:<base64 PNG>, mimeType:"image/png"}` が続く）＋`isError`。**本文の先頭の約束**: 成功で付言があるときは `OK:…`、失敗は `NG:…` かつ `isError: true`。値を返すツールは素の値（`OK:` なし）。

| ツール | 引数（必須は太字） | 成功の本文（実測） | 失敗の本文（実測） | ukadoc の対応物 |
|---|---|---|---|---|
| `get_active_ghost_list` | なし | `Emily/Phase4.5`（1 行 1 体と推定・多重起動は未確認） | — | — |
| `get_status` | `ghost_name` | 空＝idle。例 `balloon(0=0/1=0/2=0)`（名前なしの呼び出し）。旗は talking, choosing, minimizing, changing, induction, passive, timecritical, nouserbreak, online, opening(...), balloon(...) | `NG:Cannot find active ghost from specified name` | SSTP `EXECUTE GetStatus`＝SHIORI/3.0 の `Status` ヘッダ・`currentghost.status` と同じ内容 |
| `get_expression_table` | **`ghost_name`** | Markdown 表 `\|scope \0,\1,\p[2]...\|character name\|description\|surface number : \s[]\|`・行区切り `\r\n`・`\0`／`\1` ごとに surface ID と説明 | 名前なし: `NG:Specified ghost is not active` | surfaces.txt の説明・`descript`（要件で確定） |
| `get_property` | **`property_name`**, `ghost_name` | 素の値（`currentghost.name` → `Emily/Phase4.5`・`currentghost.scope(0).surface.num` → `3`） | `NG:Cannot find such property name.` | プロパティシステム |
| `get_log` | `log_type`（error 既定／script／network／update／status）, `ghost_name`, `since_id`, `max_count` | `#<id> <yyyy/mm/dd hh:mm> [<種別>] <名> : <本文>` を `\r\n` 区切り・古い順・継続行はタブ始まり。0 件は `(no log entries)`。id は全種別で通し番号（status #12・network #30・script #35 が混在） | `NG:Unknown log_type (error / script / network / update / status)` | 無し（SSP の開発者ツールのログ窓） |
| `sakurascript` | **`script`**, `ghost_name`, `strict` | `OK`。strict 時は `OK:strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=<n>)` | — | SSTP SEND の Owned SSTP 相当（「スクリプトは対象ゴースト自身の処理（Owned SSTP）と同様に扱われ、`\![reload,...]` なども実行できる」）。script ログの種別は `[SSTP(Local,Auth)]` |
| `raise_event` | **`event`**, `references[]`, `ghost_name`, `strict` | `OK:the ghost returned no script.`（返り台本があれば本文に含む＝description） | — | SSTP NOTIFY（`Event`＋`Reference*`） |
| `reload` | **`target`**（ghost／shiori／shell／balloon／makoto／descript）, `ghost_name` | （未実測＝副作用のため） | `NG:Unknown target (ghost / shiori / shell / balloon / makoto / descript)` | `\![reload,ghost／shiori／shell／balloon／makoto／descript]` |
| `dump_surface` | `scope`, `surface`, `ghost_name` | `OK:scope 0, surface 3 as currently shown (with running animations and dressups, before scaling and transparency)`＋PNG | `NG:No such surface ID. Check get_expression_table tool`・`NG:No such scope in this ghost` | `\![execute,dumpsurface,...]` |
| `dump_balloon` | `scope`, `ghost_name` | `OK:balloon of scope 0 as last drawn (before scaling and transparency; kept even if the balloon is hidden now)`＋PNG | （scope 1 も成功） | `\![execute,dumpballoon,...]` |

`ghost_name` は「ゴースト名またはルートフォルダのフルパス」。`strict` は SSTP の `Option: strict` と同じ＝存在しないサーフェス（`\s`）・アニメーション（`\i`）・バルーン（`\b`）・未知のタグ・`\![コマンド]`・`\&[実体参照]` をエラーログに記録する。

## 4. SSP の欠陥（移植しない）

1. **`get_expression_table` の文字化け**: 説明の一部の字が壊れる（例「通常」が `�\uf8f0`、キャラクタ名「ディ…」が `�\udc86�\udc87ィ`）。SJIS→UTF-8 の変換で多バイト字を割っている様子。areka は正しい文字で返す。
2. **`get_log`（script）の JSON が不正**: 台本中の逆斜線がエスケープされず `Invalid \escape` で JSON として読めない（生の `\C\0\![set,trayicon,...]`）。areka は正しくエスケープする。

## 5. 未確認（要件の段で確かめる）

- `reload` 各対象の成功の本文・`raise_event` で台本が返ったときの本文の形（副作用があるため本調査では打っていない）。
- `get_active_ghost_list` の複数体の区切り・`get_status` の各旗の出る条件の実例。
- `get_log` の `update` 種別と `ghost_name` 絞り込みの実例。

## 6. 中継 `mcp.exe` と Claude Desktop（2026-09-29 追記）

- **`mcp.exe` の振る舞い（実測）**: 引数なし。標準入力の 1 行 1 要求を SSP へ流し、応答を 1 行で返す。`initialize`（2025-06-18）→ SSP の応答そのまま・`notifications/initialized` → 出力なし・`ping` → `{}`・`tools/call get_status` → `talking,balloon(0=0/1=0)`・JSON でない行 → `-32700`・`id: null`。その直後の `no/such` には応答が出なかった（終了したのか握りつぶしたのかは未確認）。終了コード 0。
- **`mcp.exe` の中の文字列**: `POST /api/mcp/v1 HTTP/1.1`・`Host: 127.0.0.1:9801`・`Mcp-Method:`・`Mcp-Name:`・`MCP-Protocol-Version:`・`Server not available`・`No response from server`・`Invalid JSON response from server`・`Unsupported protocol version`・`result.supportedVersions`・`server/discover`＝SSP へは無状態版のヘッダを付けて送り、版の交渉に `server/discover` を使っている様子。
- **Claude Desktop の JSON 設定は stdio だけ**: Desktop 2.9939.4.0（`C:\Program Files\WindowsApps\Claude_2.9939.4.0_x64__pzs8sxrjxfjjc\app\resources\app.asar`）の検査関数は、`mcpServers` の各項目に `command`（文字列）を必須とし、ほかに `args`・`env`・`extensionId` だけを許す。`url` だけの項目は検査に落ちる。
- **Claude Desktop のコネクタは 127.0.0.1 へ届かない**: 公式の案内（support.claude.com「Getting started with custom connectors using remote MCP」）は、コネクタの接続が Anthropic のクラウドから行われ、公開インターネットから到達できるサーバが要ると書く。ukadoc の「ブラウザ上で動く AI（claude.ai など）からは 127.0.0.1 に接続できない」と同じ理由。
