# 実機確認の記録（タスク 5・要件 6）

- **実施日**: 2026-10-06
- **判定に使った版**: コミット `1da1e04e`（タスク 4 の後）。配布物の `BUILD-INFO.txt` は `commit=1da1e04` `dirty=0` `script=tools/package.ps1 2.0.0` `built=2026-10-05T23:29:23Z`
- **ビルド**: `pwsh -NoProfile -File tools/package.ps1`（全段 緑・終了コード 0）→ `target\package\areka-0.0.1-x64.zip`（8,630,013 バイト）
- **展開先**: `target\signoff-ghost-name\x\`（ワークツリーの `target\` の下・絶対パス）。記録は `target\signoff-ghost-name\logs\`（`run1.log`・`run1.stderr.log`・`curl1.txt`・`req300.json`〜`req348.json`・`package.log`）
- **ゴースト**: 展開先の `ghost\emo2\ghost\master\descript.txt`（`charset,UTF-8`・CRLF）の `name` の行だけを `name,えも？？` → `name,えも2DEBUG` に Edit で書き替えた（要件 6.2）。書き替え後のほかの行は配布物のまま:

  | 欄 | 値 |
  |---|---|
  | `name` | `えも2DEBUG`（書き替えた） |
  | `sakura.name` | `むらさき` |
  | `kero.name` | `エモ` |
  | `sakura.name2` | 無い（emo2 の descript に行が無い） |
  | フォルダ | `ghost\emo2`（フルパス `C:\home\maz\git\areka\.claude\worktrees\areka-p0-mcp-ghost-name-20ecf8\target\signoff-ghost-name\x\ghost\emo2`） |

  survey §7.4 の実測のゴーストと、名前・本体側名・相方の名前・フォルダ名がすべて同じ。

- **起動のしかた**: `AREKA_*`／`WINTF_*` は起動前に 0 個。`Start-Process -PassThru` で展開先の `areka.exe` を絶対パスで起こした（作業フォルダは展開先・pid 30932・08:30:18 JST）。終わらせ方は有界の自動終了 `AREKA_APP_SMOKE_EXIT_MS`。自分で起こしたプロセス以外は止めていない（自分の areka も止めず、自動終了に任せた）
- **環境変数**:

  ```
  AREKA_APP_SMOKE_EXIT_MS=300000
  AREKA_NO_ALERT=1
  NO_COLOR=1
  RUST_LOG=info,areka_mcp=debug
  ```

- **SSP**: この走行の間は動いていなかった（起動前に 9801〜9899 の待受は 0・SSP のプロセスも無い。走行中の待受は areka の 9801 だけ）。areka は既定の 9801 で待ち受けた:

  ```
  2026-10-05T23:30:18.811682Z  INFO areka_mcp::server: MCP: 待受を始めた url=http://127.0.0.1:9801/api/mcp/v1
  ```

- **使った道具**: curl（Git for Windows 同梱）。`mcp-get-property` の実機確認と同じ無状態版の形（`MCP-Protocol-Version: 2026-07-28`・`Mcp-Method: tools/call`・`Mcp-Name: <道具の名前>`・本文の `params._meta` に `io.modelcontextprotocol/protocolVersion="2026-07-28"`・`clientCapabilities={}`・`clientInfo`）。本文は UTF-8 のままファイル（`req<id>.json`）に書き、`--data-binary @…` で送った。応答はすべて `HTTP/1.1 200 OK`

呼んだのは起動の通知（`OnFirstBoot`。起動種別が台詞を返したので `OnBoot` は送られていない＝記録の `event="boot_type_script"`・23:30:23）から 35 秒後（23:30:58 以降）で、起動直後の短い窓は避けた。

## 判定

「見つかる」＝答えが `NG:Cannot find active ghost from specified name` でも `NG:Specified ghost is not active` でもないこと。`get_status` は解決を通った後の処理がまだ `NG:not implemented yet`（`isError: true`）を返す版なので、その答えを「見つかる」に数える（design の「実機確認の手順」）。外れ＝本文 `NG:Cannot find active ghost from specified name`・`isError: true`。

### survey §7.4 の各行

`C:/…/ghost/emo2` と `C:\…\ghost\emo2\` は上の「ゴースト」のフルパス（前者は `\` を `/` に替えた形）。

| # | `ghost_name` | survey §7.4（SSP 2.9.07） | `get_status` | `get_expression_table` | `get_log` | 判定 |
|---|---|---|---|---|---|---|
| ① | `えも2DEBUG`（名前） | 見つかる | 見つかる（id 301） | 見つかる（id 317） | 見つかる（id 333） | **合** |
| ② | `えも2debug`（英字の大小違い） | 見つかる | 見つかる（302） | 見つかる（318） | 見つかる（334） | **合** |
| ③ | `むらさき`（本体側名＝`sakura.name`） | 見つかる | 見つかる（303） | 見つかる（319） | 見つかる（335） | **合** |
| ④ | `エモ`（相方の名前＝`kero.name`） | Cannot find | Cannot find（304） | Cannot find（320） | Cannot find（336） | **合** |
| ⑤ | `ムラサキ`（かなの違い） | Cannot find | Cannot find（305） | Cannot find（321） | Cannot find（337） | **合** |
| ⑥ | ` えも2DEBUG`（名前の前に空白） | 見つかる | 見つかる（306） | 見つかる（322） | 見つかる（338） | **合** |
| ⑦ | `えも2DEBUG   `（名前の後ろに空白） | 見つかる | 見つかる（307） | 見つかる（323） | 見つかる（339） | **合** |
| ⑧ | `   `（空白だけ） | Cannot find | Cannot find（308） | Cannot find（324） | Cannot find（340） | **合** |
| ⑨ | 空文字 | Cannot find | Cannot find（309） | Cannot find（325） | Cannot find（341） | **合** |
| ⑩ | `emo2`（フォルダ名だけ） | Cannot find | Cannot find（310） | Cannot find（326） | Cannot find（342） | **合** |
| ⑪ | `C:/…/ghost/emo2`（`/` 区切り・末尾の区切りなし） | 見つかる | 見つかる（311） | 見つかる（327） | 見つかる（343） | **合** |
| ⑫ | ` C:\…\ghost\emo2\ `（パスの前後に空白） | Cannot find | Cannot find（312） | Cannot find（328） | Cannot find（344） | **合** |
| ⑬ | `えも2DEBUG`（`get_active_ghost_list` の答え〔id 300〕をそのまま） | — | 見つかる（316） | 見つかる（332） | 見つかる（348） | **合**（`mcp-tool-entrances` 要件 4.4 を保つ） |

「Cannot find」の行はどれも本文 `NG:Cannot find active ghost from specified name`・`isError: true`（18 件）。⑧⑨ の `get_expression_table` は、着地の前の `NG:Specified ghost is not active` でなく `Cannot find` になった（要件 3.1・3.2）。

### 要件の段で決めた細部（areka 側・記録）

| # | `ghost_name` | 要件 | `get_status` | `get_expression_table` | `get_log` | 判定 |
|---|---|---|---|---|---|---|
| 追A | `　えも2DEBUG\t`（全角の空白とタブで挟んだ名前） | 見つかる（2.1） | 見つかる（313） | 見つかる（329） | 見つかる（345） | **合** |
| 追B | `えも2ｄｅｂｕｇ`（`DEBUG` を全角の小文字に） | Cannot find（1.3） | Cannot find（314） | Cannot find（330） | Cannot find（346） | **合** |
| 追C | `null`（記録だけ） | — | `NG:not implemented yet`（315＝省略と同じく起動中の 1 体へ） | `NG:Specified ghost is not active`（331＝省略と同じ） | 全種別の記録 3 件（347＝省略と同じく絞らない） | 記録だけ |

- 追B は全角と半角の違いと大小の違いが重なった指定。全角の英字どうしの大小違いだけを試すには全角の英字を含む名前のゴーストが要るので、実機では当てていない（決定論テストの `resolve_tests.rs` が受け持つ）。
- `sakura.name2` は emo2 の descript に無いので実機では当てていない（要件 1.4 は本物の単位を起こす振り分けのテスト〔タスク 3〕が固定している）。
- `null` は、プロトコル側の `optional_string` が省略と同じ `None` にしたとおりの答え。

### 記録

| 項目 | 期待 | 結果 |
|---|---|---|
| 各呼び出しに「MCP: ツールに答えた」の `debug!` が 1 件ずつ | 49 件（一覧 1・3 本 × 16） | **合**（49 件） |
| 見つかった行の記録の欄 `ghost` | 綴りに関係なく一覧の値（要件 1.5・4.4） | **合**（`get_status`・`get_expression_table` の見つかった 17 件〔`get_status` の `null` を含む〕すべて `ghost="えも2DEBUG"`。外れの行は `ghost=""`） |
| `ERROR` の段の行 | 0 件 | **合**（`run1.log`・`run1.stderr.log` とも 0 件） |

- `get_log` の行の記録の `ghost` は、見つかった行でも `""`。`get_log` は振り分けでなく自分の `answer` で解決する既存の経路で、本 spec は変えていない。
- `get_log` の見つかった行（①②③⑥⑦⑪⑬・追A）の答えは、どれも `(no log entries)`。この版にはゴーストの名前を付けて記録を残す出し手がまだ無く（取り決めの target `areka::log::script`・`areka::log::error` を出すコードは本番に 0 か所）、履歴の 3 件はどれも名 `[SYSTEM]` の記録（追C の答え）。名前で指しても、英字の大小違い・本体側名・空白付きで指しても同じ答えが返ったことは確かめたが、中身のある記録が同じになることは実機では見られない。それは `get_log_tests.rs` の名前で絞るテスト（タスク 2.1 で英字の大小違いの行を足した）が固定している（要件 1.5）。

## SSP の列（要件 6.3）

SSP はこの机で動いていなかった（起動前・走行中とも 9801〜9899 に SSP の待受が無く、SSP のプロセスも無い）。SSP を起こすことはしていないので、SSP の列は取れていない。未実測の細部（全角の英字の大小・全角の空白とタブで挟んだ名前・`sakura.name2`・`null`）は SSP では未実測のまま。要件の段で決めた細部 1・2・4 と `null` の扱いは、SSP の追試ができたときに design の Revalidation Triggers のとおり見直す。

## 答え（`curl1.txt` から）

`get_active_ghost_list`:

```
## id=300 [一覧] get_active_ghost_list  arguments={}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":300,"result":{"resultType":"complete","content":[{"type":"text","text":"えも2DEBUG"}],"isError":false}}
```

見つかる行の答えは道具ごとに 1 種類だけだった（`get_status` の 9 件〔`null` を含む〕・`get_expression_table` の 8 件・`get_log` の 8 件〔`null` を除く〕。`get_expression_table` の 8 件の結果は JSON の文字列として全部同じ）:

```
## id=302 [②英字の大小違い] get_status  arguments={"ghost_name": "えも2debug"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":302,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:not implemented yet"}],"isError":true}}

## id=319 [③本体側名] get_expression_table  arguments={"ghost_name": "むらさき"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":319,"result":{"resultType":"complete","content":[{"type":"text","text":"|scope \\0,\\1,\\p[2]...|character name|description|surface number : \\s[]|\r\n|-----|-----|-----|-----|\r\n|\\0|0|素|\\s[0]|\r\n…（emo2 の表情の表・898 文字）"}],"isError":false}}

## id=338 [⑥前に空白] get_log  arguments={"ghost_name": " えも2DEBUG"}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":338,"result":{"resultType":"complete","content":[{"type":"text","text":"(no log entries)"}],"isError":false}}
```

外れの行（18 件とも同じ本文）:

```
## id=309 [⑨空文字] get_status  arguments={"ghost_name": ""}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":309,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}}

## id=324 [⑧空白だけ] get_expression_table  arguments={"ghost_name": "   "}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":324,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}}

## id=344 [⑫前後に空白のあるフルパス] get_log  arguments={"ghost_name": " C:\\home\\maz\\git\\areka\\.claude\\worktrees\\areka-p0-mcp-ghost-name-20ecf8\\target\\signoff-ghost-name\\x\\ghost\\emo2\\ "}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":344,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}}
```

`null`（記録だけ）:

```
## id=331 [追Cnull] get_expression_table  arguments={"ghost_name": null}
HTTP/1.1 200 OK
{"jsonrpc":"2.0","id":331,"result":{"resultType":"complete","content":[{"type":"text","text":"NG:Specified ghost is not active"}],"isError":true}}
```

## 記録（`run1.log` から該当の行）

```
2026-10-05T23:30:58.538924Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_active_ghost_list" ghost="" is_error=false text=""
2026-10-05T23:31:17.022206Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも2DEBUG" is_error=true text="NG:not implemented yet"
2026-10-05T23:31:17.173959Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="えも2DEBUG" is_error=true text="NG:not implemented yet"
2026-10-05T23:31:17.305488Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="" is_error=true text="NG:Cannot find active ghost from specified name"
2026-10-05T23:31:19.364182Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_expression_table" ghost="えも2DEBUG" is_error=false text=""
2026-10-05T23:31:20.230257Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_expression_table" ghost="" is_error=true text="NG:Cannot find active ghost from specified name"
2026-10-05T23:31:21.364886Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_expression_table" ghost="" is_error=true text="NG:Specified ghost is not active"
2026-10-05T23:31:22.330515Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_log" ghost="" is_error=false text=""
2026-10-05T23:31:22.613770Z DEBUG actor{actor=mcp}:serve_inner: areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_log" ghost="" is_error=true text="NG:Cannot find active ghost from specified name"
```

（順に id 300・302〈英字の大小違い〉・303〈本体側名〉・304〈相方の名前〉・319〈本体側名〉・324〈空白だけ〉・331〈`null`〉・338〈前に空白〉・341〈空文字〉）

`WARN` の段は 4 件。3 件は emo2 の素材とバルーンの定義についての既存の記録（`areka_emo_atlas` の全透明の element 2 件・`areka_emo_text` の折返し基準 1 件）、1 件は有界の自動終了の続きで kanade が出す `event="force_quit" reason="user"`（23:35:21）。どれも本 spec の処理とは関係しない。

## 終わり方

有界の自動終了（300 秒）で 23:35:21（08:35:21 JST）に終わった。kanade の `強制終了指示——終了系列（Forced）へ直行 event="force_quit"`（WARN）の後、記録の末尾は「ghost shutdown sequence completed」→「きれいに終わったので起動中の印を消しました event="session_mark_cleared"」→「MCP: 待受を閉じた addr=127.0.0.1:9801」。pid 30932 のプロセスが消えたことを確かめた（終了コードは取っていない）。

`ERROR` の段の数え方: 段の欄が `ERROR` の行を数えて 0 件（`run1.log`・`run1.stderr.log`）。大小を区別しない `error` の一致は 49 件あるが、どれも橋の記録の欄の名前 `is_error` で、段ではない。`run1.stderr.log` は補助 exe の 1 行（`[helper] SHIORI 初期化の入口: loadu`）だけ。
