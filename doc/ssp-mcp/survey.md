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

## 7. `get_property` の詳しい実測（2026-10-04・SSP 2.9.07・`get_property` spec の要件の段）

SSP 2.9.07（`baseware.version` → `SSP/2.9.07 (20261001-0; Windows NT 10.0.26300)`）・ゴースト「えも2DEBUG」（emo2）1 体の起動中に、Claude Code から SSP の MCP サーバへ直接打った。`tools/list` の `get_property` の定義（説明文・2 つの引数の説明・必須は `property_name` だけ）は 2.9.05 と同じで、areka の定義（`crates/areka-mcp` の get_property の定義）とも一字違わない。

### 7.1 名前ごとの答え

| 送った `property_name` | `ghost_name` | 答え | `isError` |
|---|---|---|---|
| `currentghost.name` | なし | `えも2DEBUG` | false |
| `baseware.name` | なし | `SSP` | false |
| `currentghost.status` | なし | （空の本文）＝idle | false |
| `currentghost.scope(0).surface.num` | なし | `1000` | false |
| `currentghost.scope(5).surface.num`（無いスコープ） | なし | `-1` | false |
| `currentghost.seriko.surfacelist.all` | なし | `0,10,1000,…,2210`（カンマ区切りの 1 行） | false |
| `currentghost.path` | なし | `C:\…\emo2\ghost\emo2\`（末尾の区切りあり） | false |
| `ghostlist.index(0).name` | なし | `Emily/Phase4.5` | false |
| `ghostlist(えも2DEBUG).path`／`ghostlist(えも2debug).path` | なし | 同じパス | false |
| `ghostlist(0).name`／`ghostlist(0).path` | なし | `NG:Cannot find such property name.` | true |
| `ghostlist(99).name`・`ghostlist(nonexistent).path` | なし | `NG:Cannot find such property name.` | true |
| `ghostlist`（一覧そのもの） | なし | `NG:Cannot find such property name.` | true |
| `currentghost.icon`（descript に無い） | なし | `NG:Cannot find such property name.` | true |
| `no.such.thing` | なし | `NG:Cannot find such property name.` | true |
| （空文字） | なし | `NG:Cannot find such property name.` | true |
| `baseware..name`・`baseware.name.` | なし | `NG:Cannot find such property name.` | true |
| ` baseware.name `（前後に空白） | なし | `NG:Cannot find such property name.` | true |
| `BASEWARE.NAME` | なし | `SSP` | false |

### 7.2 `ghost_name` の効き方

| `ghost_name` | `baseware.name` への答え |
|---|---|
| `えも2DEBUG` と同じ綴り | `SSP` |
| `えも2debug`（英字の大小違い） | `SSP` |
| ルートフォルダのフルパス `…\emo2\ghost\emo2\` | `SSP` |
| ゴーストのフォルダでない上位のパス `…\project\emo2` | `NG:Cannot find active ghost from specified name` |
| `nobody` | `NG:Cannot find active ghost from specified name` |
| 空文字 | `NG:Cannot find active ghost from specified name` |

全体の名前（`baseware.*`）を引くときも、`ghost_name` が外れていれば先に名前の解決で落ちる。

### 7.3 読み取れる約束（areka との突き合わせ）

1. **値は素のまま**: `OK:` を付けず、空白も改行も足さない。空の値（idle の `status`）は空の本文で `isError: false`。→ areka の要件と同じ。
2. **「無い」はすべて 1 つの文言**: 名前の誤り・書式の誤り・範囲外の番号・選んだ名前の不在・一覧そのものの名前・descript に無い項目は、どれも `NG:Cannot find such property name.`。→ areka の `DottedResolution::NotFound` 1 つに写せば足りる。
3. **名前は手直ししない（空白）**: 前後の空白は削らず「無い名前」になる。→ areka の要件と同じ。
4. **名前の英字の大小は区別しない（SSP）**: `BASEWARE.NAME` が通る。areka の読み手（`areka-sylphya` の点付きの名前の解釈 `parse_dotted` と鏡像の引き当て）は大小を区別する＝`BASEWARE.NAME` は「無い名前」になる。SHIORI の `GetProperty`・`%property[]` も同じ読み手なので、直すなら読み手の側で全経路をそろえる話になる（get_property だけ畳むと SHIORI 側と食い違う）。
5. **数字の括弧（SSP）**: `ghostlist(0).name` は SSP では「名前が 0 のゴースト」と読まれて無い名前になり、番号で引くのは `ghostlist.index(0).name` の形だけ。areka の `parse_dotted` は数字だけの括弧を番号として読む。値の網羅の spec（`property-catalog-lists`）の持ち物。
6. **`ghost_name` の綴り（SSP）**: 英字の大小を区別せず、空文字は「省略」でなく「外れ」。areka の宛先の解決（`crates/areka/src/mcp/resolve.rs` の `resolve`）は名前を完全一致で比べ、空文字を省略と同じに扱う。全ツール共通の振る舞いなので `mcp-tool-entrances` の持ち物（get_property だけの話ではない）。本体側名・前後の空白も含めた全体は 7.4。

### 7.4 `ghost_name` の照合（全ツール共通・2026-10-04 追記）

`get_status`・`get_expression_table`・`get_log` でも同じ照合だった（ゴースト「えも2DEBUG」・本体側名 `むらさき`・相方の名前 `エモ`・フォルダ `emo2`）。

| `ghost_name` | 答え |
|---|---|
| `えも2DEBUG`・`えも2debug`（英字の大小違い） | 見つかる |
| `むらさき`（本体側名＝`sakura.name`） | 見つかる |
| `エモ`（相方の名前＝`kero.name`）・`ムラサキ`（かなの違い） | `NG:Cannot find active ghost from specified name` |
| ` えも2DEBUG`・`えも2DEBUG   `（名前の前後に空白） | 見つかる |
| `   `（空白だけ）・空文字 | `NG:Cannot find active ghost from specified name`（`get_status`・`get_log`・`get_expression_table` のどれも） |
| `emo2`（フォルダ名だけ） | `NG:Cannot find active ghost from specified name` |
| `C:/…/ghost/emo2`（`/` 区切り・末尾の区切りなし） | 見つかる |
| ` C:\…\ghost\emo2\ `（パスの前後に空白） | `NG:Cannot find active ghost from specified name` |

areka の宛先の解決（`crates/areka/src/mcp/resolve.rs` の `resolve`・`mcp-tool-entrances` の要件 3.2〜3.5）との違いは 4 つ: ⑴ 名前の英字の大小（areka は区別する）、⑵ 本体側名（areka は照合しない）、⑶ 名前の前後の空白（areka は削らない）、⑷ 空文字（areka は省略と同じに扱い、`get_expression_table` では `NG:Specified ghost is not active` と答える）。パスの照合（大小・区切り・末尾の区切りを同じとみなす・フォルダ名だけは不一致）は同じ。
