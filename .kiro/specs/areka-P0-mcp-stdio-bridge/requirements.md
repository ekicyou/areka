# Requirements Document

> 本文の実物の確認は **2026-10-10・本ブランチ**（main `414d43eb`）のもの。SSP の中継の実測も同じ日（開発者の机の `C:\wintools\ssp\data\mcp.exe`・94,208 バイト・2026-09-24 の日付）。コードは「何の定義か」（関数名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「開発者の裁定」の 2 件は、要件を書く途中で brief と実測が食い違った点・brief が決めていなかった点に、開発者が 2026-10-10 に答えたもの。
> 「要件の段で決めた細部」の表は、brief が「要件で決める」とした細部と、書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: Claude Desktop で areka の MCP ツールを使いたい人。Desktop の設定（`claude_desktop_config.json`）は `command` で起動する stdio のサーバーしか書けず、コネクタの登録は Anthropic のクラウドからつなぎに来るので `127.0.0.1` の areka へ届かない。代わりの `npx mcp-remote` は Node.js を入れさせるので、第三者に勧める形として重い。

**今の状態**: areka 本体は `127.0.0.1` の 1 つのポート（既定は 9801 → 9821 の早い者勝ち・どちらも使用中なら隣の番号・`AREKA_MCP_PORT` で 1 つを指定）で HTTP の MCP を受ける（完了 `mcp-server-core`）。stdio との中継は無く、help の Claude Desktop の節は「中継は今後の版で用意します」の 1 文だけ。SSP は同じ理由で中継 `data\mcp.exe` を同梱している。

**何を変えるか**: stdio と HTTP の中継の exe を配布 zip に入れる。Desktop の設定の `command` に書けば、areka の MCP ツールが Desktop から使える。Desktop を areka より先に起動していても、前に areka から受け取った答えの控えでツールの一覧が並び、areka を起動すればそのまま使える（SSP の中継と同じ）。areka に届かない・応答が壊れているときは、理由の分かる JSON-RPC のエラーが Desktop に返り、中継は落ちない。SSP など areka でないものが同じ番号で待ち受けていても、そこへはつながない。`GET /api/mcp/help` に Desktop 用の設定例（中継の絶対パス入り）が載る。

> 起票: 2026-09-29 `/kiro-discovery`（開発者判断「SSP と同じ対応を行うには中継 exe を作らないとダメ。どうせ作りたくなるので spec を置いてロードマップに置く」）。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md) §1・§6 と [doc/ssp-mcp/transport-diff-areka.md](../../../doc/ssp-mcp/transport-diff-areka.md)。

## Introduction

### 誰が困っているか

- **Claude Desktop の利用者**: Claude Code と Cursor は areka の URL を直接登録できるが、Desktop は登録の手段が無い。SSP なら同梱の中継を設定に書けば済む。
- **areka を第三者に勧める人**: 「Desktop で使うには Node.js を入れて `npx mcp-remote`」という案内は、zip を展開して起動するだけの配布の形に合わない。

### いま何が起きているか（2026-10-10 に実物で確認）

- **中継は無い。** `crates/` に中継のクレートは無く、配布スクリプト `tools/package.ps1` が zip に入れてよいとする実行ファイルは `areka.exe`・`shiori-host32-helper.exe`・同梱ゴーストの `pasta.dll` の 3 つ（`$ALLOWED_EXECUTABLES`）。
- **help の Desktop の節は 1 文。** `crates/areka-mcp/src/help.rs` の `help_html` は「中継は今後の版で用意します」と書くだけで、テスト `help_tests.rs` は設定ファイルの名前（`claude_desktop_config`）が載っていないことを確かめている。`help_html` が受け取るのは待ち受けている番号だけで、呼び手は `dispatch.rs` の 1 か所。
- **areka の受け口の約束**（`doc/ssp-mcp/transport-diff-areka.md`）: セッションを持たない・応答は JSON 1 件・通知には本文なしの 202・`Accept` に `application/json` と `text/event-stream` の両方が無い要求は 406・`Host` がループバックでない要求は 403・**壊れた JSON の本文には 415 と平文**（JSON-RPC のエラーではない）・版の札 `MCP-Protocol-Version` が無い要求は旧式として通す・`initialize` を使わない版 `2026-07-28` は札 `Mcp-Method`（`tools/call` では `Mcp-Name` も）が要る・名乗りは `serverInfo` の `name` が `areka-mcp-server`（SSP は `ssp-mcp-server`）。
- **待受の候補**は `crates/areka-mcp/src/port.rs` の `DEFAULT_PORTS`（9801・9821）と `FALLBACK_STEPS`（9）から組む 20 個（9801・9821・9802・9822 … 9810・9830）。
- **説明書 `dist/README.txt` に MCP の節も `AREKA_MCP_PORT` の説明も無い**（完了 `mcp-server-core` が本 spec へ送った宿題）。
- **Desktop がどの版で話すかは未確認。** Claude Code 2.1.283 は `initialize` を送らず `2026-07-28` の `server/discover` でつなぐ（`mcp-server-core` の実機確認）。

### SSP の中継 `mcp.exe` の実測（2026-10-10・brief の「要件の段で 1 度の突き合わせ」）

9801 に記録を取る偽のサーバーを立てた場合と、何も居ない場合（答えの控えがある・無いの 2 通り）で、標準入力へ要求を流して出力を記録した。

**SSP（に見立てた偽のサーバー）が応えるとき**

| 見たもの | SSP の中継の振る舞い |
|---|---|
| 送り方 | 1 要求ごとに接続 1 本。`POST /api/mcp/v1 HTTP/1.1`・`Host: 127.0.0.1:9801`・`Content-Type: application/json`・`Accept: application/json, text/event-stream`・`Content-Length`・`Connection: close`。`initialize`（`2025-06-18`）で始めた並びでは、札 `Mcp-Method`・`Mcp-Name`・`MCP-Protocol-Version` は 1 つも付かなかった |
| 入力の切り方 | **行ではなく、いちばん外の `{`〜`}` の対応で切る。** 配列 `[{…},{…}]` は 2 つの要求に割れて応答が 2 つ出た。JSON でない行は、次の `{…}` と 1 つの本文にまとめて送られた |
| 応答 | 本文をそのまま書く。改行入りの JSON は改行入りのまま複数行で出た。応答の `id` は確かめない（違う `id` もそのまま出た）。文字列の `id` はそのまま返る |
| 通知 | 本文なしの 202 → 出力なし |
| 本文が JSON でない（200 の平文・500・415） | `{"jsonrpc":"2.0","id":<要求の id>,"error":{"code":-1,"message":"Invalid JSON response from server"}}` |
| 本文が空・応答なしで接続が閉じた | 同じ形で `"No response from server"` |
| 答えの控え | `initialize` の答えを受け取るたびに SSP のフォルダの `data\profile\mcp_initialize.dat` へ書く。`tools/list` の答えの控えは同じ場所の `mcp_tools.dat` |
| 終わり | 標準入力が閉じると終了コード 0。標準エラーは空 |

**SSP が居ないとき（9801 に何も居ない）**

| 入力 | SSP の中継の振る舞い |
|---|---|
| `ping` | 自分で `{"jsonrpc":"2.0","id":<id>,"result":{}}` と答える（控えの有無に関わらず） |
| `initialize`（控えがある） | 控え（`mcp_initialize.dat`）の答えを、`id` だけ差し替えて返す（要求した版に関わらず控えの版のまま） |
| `initialize`（控えが無い） | `{"jsonrpc":"2.0","id":<id>,"error":{"code":-1,"message":"Server not available"}}`。控えの置き場には何も書かれない |
| `tools/list`（控えがある） | 控え（`mcp_tools.dat`）の答えを、`id` だけ差し替えて返す |
| 通知 | 出力なし |
| ほか（`tools/call`・`server/discover`・未知のメソッド） | `{"jsonrpc":"2.0","id":<id>,"error":{"code":-1,"message":"Server not available"}}` |
| JSON でない入力 | 自分で `{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}` と答える |
| 続き | どの場合も終了せず次を読む（1 要求あたり約 0.1 秒）。終了コード 0・標準エラーは空 |

- **未実測**: 接続を受けたまま返事をしない相手をどれだけ待つか・`initialize` を使わない版（`2026-07-28`）での送り方・控えが無いときの `tools/list`。
- **この実測の副作用と後始末**: 偽のサーバーの `initialize` の答えで、開発者の SSP の `data\profile\mcp_initialize.dat` を上書きした。上書きした控えは開発者の了承を得て取り除き、「控えが無い」場合の実測に使った。SSP が起動している状態で SSP の中継に `initialize` が 1 回通れば、本物の答えで作り直される。本 spec のテストは利用者の本物の控えに触れない（要件 8.1）。

### brief の記述を実物で引き直して改めた点

1. brief の本文の「既定 9821」は古い。今の候補は 9801 が先で、20 個ある（brief の 10-03 の注記と棚卸の追記のとおり）。SSP が 9801 を持っている机では、候補を順に当たるだけの中継は SSP につながる（裁定 2 で塞ぐ）。
2. brief の「標準入力の 1 行 1 要求」は SSP の中継の実際とは違う（波かっこの対応で切る）。areka の中継は brief のとおり 1 行 1 要求にする（細部 5）。
3. brief の「JSON でない行には `-32700`・`id: null`」は、SSP が居るときは SSP 本体の答えを写したもの。areka 本体は壊れた JSON に JSON-RPC でない 415 を返すので、areka の中継は自分で答える必要がある（要件 1.6）。
4. brief の「本体が居なければ Server not available」は、`initialize`・`tools/list`・`ping` には当たらない。SSP の中継はこの 3 つに控えと自前の答えで応える。brief の望む姿（「areka が起動していないとき、理由の分かるエラーが返る」）と brief の範囲（「SSP の `mcp.exe` と同じ入力に同じ出力」）がこの 3 つについて食い違っていたので、裁定 1 で SSP と同じ側に決めた。
5. brief の「`Mcp-Method`／`Mcp-Name`／`MCP-Protocol-Version` のヘッダを付けて送る」は、`initialize` で始める並びでは起きなかった。
6. survey §6 の「JSON でない行の直後の `no/such` に応答が出なかった（終了したのか握りつぶしたのかは未確認）」は、JSON でない行と次の要求が 1 つの本文として送られ、1 つのエラーで答えられたため。終了も握りつぶしもしていない。
7. brief の「中継は MCP の中身を作らず写すだけ」は、裁定 1・2 で 3 か所だけ外れる: 控えを返す・`ping` に自分で答える・相手が areka かを確かめる問い合わせを自分で送る。加えて、`initialize` を使わない版（`2026-07-28`）を通すには、行の中身（`method`・`params.name`・`params._meta` の版）を読んで HTTP の札を 3 つ作る必要がある（ギャップ分析で rmcp 3.5.0 のソースから確認・`research.md` 2.5）。これらは手書きで持たず、公式 SDK（rmcp）で組む（裁定 3）。

### 開発者の裁定（2026-10-10）

| # | 問い | 裁定 | 載せた要件 |
|---|---|---|---|
| 1 | areka が起動していないとき、`initialize`・`tools/list`・`ping` にどう答えるか | **SSP の中継と同じ。** 中継は、areka から中継した `initialize` と `tools/list` の答えを控えておき、areka が居ないときは控えを `id` だけ差し替えて返す。`ping` は自分で答える。ほかの要求は「areka が居ない」のエラー。控えがまだ無いときの `initialize` はエラー（SSP の実測と同じ）。`server/discover` は SSP と同じく控えず、エラーで答える。**控えの置き場と形は設計で決める**（要件では決めない）。※「`id` だけ差し替えて返す」と「`server/discover` は控えない」は、同じ日の裁定 3 で「控えの中身で、どの版の最初の問い合わせにも答える」に改めた | 3.1〜3.11 |
| 2 | 候補を当たっていて、areka でないもの（SSP など）が応えたらどうするか | **areka の MCP サーバーだと名乗った相手にだけつなぐ。** 違う相手は飛ばし、どの候補も areka でなければ「areka が見つからない」のエラー。`AREKA_MCP_PORT` で番号を指定したときも同じく確かめる。**確かめるのにどの要求を使うかは設計で決める** | 2.2〜2.4・2.11・3.6 |
| 3 | 行の中身を読んで HTTP の札を作る・控えで答える・名乗りを確かめる中継を、手書きで持つか公式 SDK で組むか | **公式 SDK（rmcp）で組む。** Desktop の側（stdio）も areka の側（HTTP）も rmcp で受ける。`Cargo.lock` に無い外部のクレートは足さない（付属の `reqwest` は使わず、HTTP を送る部分だけ自前で書く＝実測で増える名前 0 個）。これに伴い、中継は「字をそのまま写す」から「意味を変えずに届ける」になり、裁定 1 の「`id` だけ差し替えて返す」「`server/discover` は控えない」は「控えの中身で、どの版の最初の問い合わせにも答える」に改まる。設計の最初に 3 点（自前の HTTP の送り手で rmcp の口を満たせるか・確かめた相手と同じ接続で送れるか・exe の大きさ）を確かめ、崩れたら止めて報告する | 1.1・1.2・1.10・3.1〜3.9・Boundary Context |

### 開発者の裁定（2026-10-11）

実機確認（`verification/signoff.md`）の後に、実機で分かった 1 点と、要件の文と実装の間に幅があった読み 4 点に、開発者が答えたもの。コードを変えたのは 1 だけ（help の文）。2〜5 は実装をそのままにして、要件の文が実装の形を言い表すように改めた。

| # | 問い | 裁定 | 載せた要件 |
|---|---|---|---|
| 1 | Claude Desktop は窓を閉じるだけでは終了しない（タスクバーの通知領域に残る）ので、「起動し直す」だけでは設定が読み直されないことがある | help と説明書の「起動し直す」に、窓を閉じるだけでは終了しないことと、通知領域のアイコンから終了してから起動し直すことを書き添える | 6.5・7.4 |
| 2 | 「本文が JSON である」応答とは何か | 札 `Content-Type` が `application/json` で、本文が JSON-RPC のメッセージとして読める応答。ほかの札の応答は、本文が JSON として読めても「JSON でない応答」のエラーで答える（コードは変えていない） | 4.2・4.3 |
| 3 | 「areka が見つからない」のエラーに、どの相手の番号と名乗りを続けるか | 名乗りを読めた相手の分だけ。名乗りを読めなかった相手は標準エラーにだけ残す。届ける前の失敗が 2 度続いた要求には、番号と名乗りを続けないエラーで答える（コードは変えていない） | 3.6 |
| 4 | つながっている間の「areka の今のサーバーの情報」とは何か | 中継がつなぎを作ったときの握手で受け取った情報。そのつなぎが生きている間は、areka が同じ番号で起動し直しても同じ。ツールの一覧と呼び出しは毎回 areka の今の答え（コードは変えていない） | 3.9 |
| 5 | 旧式の話し方（`_meta` なし）の `server/discover` は、中継ごしでは結果・直接では `-32602` で、答えが違う | 今のまま。差は `doc/ssp-mcp/bridge-diff-areka.md` の記録のとおり（コードも記録も変えていない） | 1.2 |

### 要件の段で決めた細部（要件ディスカッションで覆せる）

| # | 細部（brief の議題） | 決めたこと | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | exe の名前 | **`areka-mcp-bridge.exe`**（Desktop の設定の名前は `areka`） | brief と roadmap が使ってきた名前 | 6.1・7.1 |
| 2 | ポートを引数でも渡せるか | **渡せない。`AREKA_MCP_PORT` だけ** | Desktop の設定は `env` を書ける。SSP の中継も引数なし。手段が 2 つあると食い違ったときの順を決める必要が出る | 2.8 |
| 3 | arm64 の zip に入れるか | **入れる**（arm64 の zip には arm64 の中継） | brief の制約「x64＋arm64」 | 7.1 |
| 4 | 写すだけか、版の札を解釈するか | **意味を変えずに届ける。Desktop がどの版で話しても、areka へ HTTP で直接つないだときと同じ意味の結果を返す**（裁定 3。版の交渉と HTTP の札は rmcp が持つ。字の並び〔欄の順・空白〕は同じとは限らない） | Desktop がどの版で話すかは未確認で、どの版でも使えることが望む姿。Claude Code は既に `initialize` を使わない版でつなぐ | 1.1・1.2・1.10 |
| 5 | 入力の切り方 | **1 行 1 要求**（SSP の中継の「波かっこの対応で切る」は写さない） | MCP の stdio は改行区切りで、brief も 1 行 1 要求。JSON でない行が次の要求を巻き込まない | 1.1・1.6 |
| 6 | 中継が作るエラーの番号と文 | **番号は SSP と同じ `-1`。文は SSP と同じ書き出し（`Server not available`・`No response from server`・`Invalid JSON response from server`）に、理由を英語で続ける** | SSP と同じ文で始めれば SSP の中継に慣れた利用者とエージェントが読める。続きで「理由の分かる」を満たす | 3.6・4.1・4.2 |
| 7 | `AREKA_MCP_PORT` が `0`・数でない | **`0` は「つなぐ先が無い」として、控えも使わずに各要求へエラー。数でない・範囲外は、標準エラーに 1 件残して既定の 20 候補** | areka 本体の読み方（`0` は待ち受けない・読めない値は既定）と同じ意味にする。待ち受けないと決めた設定で、控えのツールの一覧を並べない | 2.5・2.6 |
| 8 | 改行入りの応答 | **意味を変えずに 1 行にして返す**（SSP の中継は改行入りのまま出す） | stdio の 1 メッセージは 1 行。areka は今は改行なしで返すが、決まりとして固定する | 1.4 |
| 9 | SSP の中継との差の記録の置き場 | **`doc/ssp-mcp/` の下に新しい文書 1 つ**（上の実測と、areka の中継の答えを行ごとに並べる）。survey §6 にはそれを指す 1 行を足す | `mcp-server-core` の差の一覧と同じ置き方 | 8.4 |
| 10 | winget の別名（`PortableCommandAlias`）・コード署名 | **本 spec では決めない** | それぞれ `winget-manifest-submission`・`release-code-signing` の持ち物 | Boundary Context |
| 11 | 控えに取る答え | **エラーでない結果だけ**（`initialize`・`tools/list` の `result` を持つ応答）。版ごとに分けず、最後に通った 1 つずつ（SSP と同じ） | エラーを控えると、areka が居ないときにエラーを返し続ける。版ごとに分ける必要は、`initialize` を使わない版が控えの対象でない（裁定 1）ので出ない | 3.1・3.2 |
| 12 | 控えが無いときの `tools/list` | **`initialize` と同じくエラー**（SSP では未実測） | 控えが無いときの `initialize` の実測に揃える | 3.5 |
| 13 | areka が見つからないエラーの文の続き | **areka でない相手が応えた番号があれば、その番号と相手の名乗りを続きに入れる** | 「areka は起動していないが SSP が 9801 に居る」が Desktop の側で読める | 3.6 |
| 14 | 届けた要求に応答が無かったとき | **送り直さない** | 台本を流す・イベントを起こす要求が 2 回届くと、ゴーストが同じことを 2 回する | 4.5 |
| 15 | Desktop が `initialize` を使わない版で話す場合 | **どの版の最初の問い合わせ（`initialize`・`server/discover`）にも、控えのサーバーの情報で答える**（SSP の中継は `server/discover` にエラーを返す＝差の記録に載せる） | 裁定 3。Desktop の話す版が未実測でも「Desktop を先に起動してもツールが並ぶ」が効く | 3.4・8.5 |

### 設計へ送る議題（要件では決めない）

- 控えの置き場と形（中継を起動し直しても残ること・2 つの中継が同時に書いても壊れないことは要件 3 が求める）。
- 相手が areka かを確かめるのにどの要求を使うか、いつ確かめ直すか（要件 2.11 を満たす接続の持ち方）。
- 候補の並びを areka 本体と共有する手段（共有するか、写して一致を検査するか）。
- help が中継の置き場を知る手段（help の中で求めるか、areka 本体が自分の場所に使っている決まり〔リンクを辿る〕を通して起動の配線から渡すか＝`research.md` 議題 6）。
- 待つ時間の値（候補を見切る時間・応答を待つ上限）。
- rmcp での組み方（裁定 3）: 自前の HTTP の送り手で rmcp の口（`StreamableHttpClient`）を満たす形・要求を種類ごとでなく 1 つの口で areka へ渡す形・tokio を中継のどこに置くか・exe の大きさ。
- rmcp の stdio の輸送が、JSON でない行・オブジェクトでない行をどう扱うか（要件 1.6・1.7 を満たす手段）。
- rmcp の版を上げるときに中継のテストも通す決まりの置き場（`.kiro/steering/tech.md` の rmcp の項）。

## Boundary Context

- **In scope**:
  - 中継の exe（標準入力の 1 行を areka の MCP の受け口へ届け、応答を 1 行で標準出力へ返す）。
  - つなぐ先の決め方（既定の候補・`AREKA_MCP_PORT`・相手が areka かの確かめ）。
  - areka が居ないときの答え（控え・自前の `ping`・エラー）と、応答がおかしいときのエラー。
  - `GET /api/mcp/help` の Claude Desktop の節（設定例）。
  - 配布 zip（x64・arm64）への同梱と、配布スクリプトの検査・説明書 `dist/README.txt`。
  - 決定論テスト（ループバックの偽のサーバー）・SSP の中継との差の記録・実機確認（Claude Desktop から）。
- **Out of scope**:
  - areka 本体の MCP の受け口とツール（完了 `mcp-server-core` と各ツールの spec）。本 spec は受け口の振る舞いを変えない。
  - `127.0.0.1` 以外への中継（ほかの機械の areka）・認証・TLS。
  - SSP へつなぐこと（中継のつなぐ先は areka だけ）。
  - areka が後から起動したことを Desktop へ知らせること（中継から Desktop へ通知を送らない＝0 件。Desktop の持つツールの一覧は、Desktop が次に `tools/list` を送るまで前のまま）。
  - Claude Desktop の拡張機能（`.mcpb`）の包み。
  - Claude Code・Cursor 向けの案内（今のまま HTTP で直接つなぐ）。
  - サーバーからクライアントへ向かう要求・通知の中継（areka は出さない＝0 件）。
  - winget のマニフェスト・コード署名（細部 10）。
- **Adjacent expectations**:
  - 完了 `mcp-server-core` の受け口の約束（上の「areka の受け口の約束」）に頼る。本 spec は help の Claude Desktop の節だけを変え、ほかの節（URL・Claude Code・Cursor・ポートの決まり方・areka 独自のツール）は変えない。
  - 同じウェーブの約束（roadmap のウェーブ C5）で本 spec が触ると届けてあるのは、新しいクレート・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（作り直し）・`help.rs`・`help_tests.rs`・`tools/package.ps1`・`dist/README.txt`（自分の行）。約束の一覧に無い新しい文書 1 つ（細部 9）と survey §6 の 1 行も触る。**約束は設計の形を縛らない**（開発者の方針 2026-10-05「設計は本質の形で。触らない約束を理由に回避の形へ逃げない」）: 設計が本質の形を選んだ結果、約束の外のファイル（`crates/areka/src/main.rs`・`crates/areka-mcp/src/` の `dispatch.rs`・`server.rs`・`lib.rs`・`port.rs`・`handler.rs` など）に触るなら、設計の文書にそのファイルを挙げ、設計ディスカッションで開発者に諮って、同じファイルを触る並走の spec（`main.rs` は `baseware-root-list`）と調整する。
  - 中継は公式 SDK（rmcp・`Cargo.lock` に既にある版）で組み、`Cargo.lock` に無い外部のクレートは足さない（裁定 3。`reqwest` は使わない）。rmcp の機能（クライアント・stdio の輸送）を足すので、`Cargo.lock` と謝辞は作り直す。設計の最初に確かめる 3 点（裁定 3）のどれかが崩れ、外部のクレートを足さないと組めないと分かったら、止めて報告する（依存を足すのは開発者の判断）。
  - 版上げ（`v0.0.3`）は本 spec の PR が開いていないときに行う（roadmap の決まり）。先に版上げが着地したら、取り込んで `Cargo.lock` と謝辞を作り直す。
  - 完了のときに `release-code-signing`（署名する exe の一覧）と `winget-manifest-submission`（入れ子の実行ファイルの一覧）へ、中継が増えたことを申し送る。

## Requirements

### Requirement 1: 標準入力の 1 行を areka へ届け、応答を 1 行で返す

**Objective:** As a Claude Desktop の利用者, I want Desktop の設定に中継を書くだけで areka の MCP ツールが使えること, so that Node.js などを別に入れずに済む

#### Acceptance Criteria

1. When 標準入力に JSON のオブジェクト 1 つの行（UTF-8・行末は LF でも CRLF でもよい）が届く, the 中継 shall その要求を、意味を変えずに、つないだ areka の MCP の受け口（`POST /api/mcp/v1`）へ届け、areka の応答を、意味を変えずに 1 行（UTF-8・BOM なし・行末は LF 1 つ）で標準出力へ書く。
2. The 中継 shall 同じ要求を areka へ HTTP で直接送ったときと同じ意味の応答を返す（欄と値が同じ。欄の順と空白は問わない。`initialize`・`tools/list`・`tools/call`・`ping`・areka が返す JSON-RPC のエラー・`isError: true` の結果を含む。`id` は数でも文字列でも要求のまま返る）。例外は 1 つ: 旧式の話し方（`_meta` なし）の `server/discover` には、中継は areka へ送らずに areka のサーバーの情報で答え、areka へ直接送ると `-32602` が返る（5 つの版のクライアントは送らない要求。`doc/ssp-mcp/bridge-diff-areka.md` に記録＝2026-10-11 の裁定 5）。
3. When 届いた行が通知（`id` を持たない要求）で、areka が本文の無い応答で受け取る, the 中継 shall 標準出力へ何も書かない。
4. The 中継 shall どの応答も 1 行で書く（areka の応答の本文が途中に改行を含んでいても、行を分けない）。
5. When 標準入力に空の行（空白だけの行を含む）が届く, the 中継 shall areka へ何も送らず、標準出力へ何も書かず、次の行を読む。
6. If 届いた行が JSON として読めない, then the 中継 shall areka へ送らずに、`id` が `null`・番号 `-32700` の JSON-RPC のエラーを 1 行で返し、次の行を読み続ける（その行が次の行の扱いを変えない）。
7. If 届いた行が JSON として読めるがオブジェクトでない（配列・数・文字列など）, then the 中継 shall areka へ送らずに、`id` が `null`・番号 `-32600` の JSON-RPC のエラーを 1 行で返し、次の行を読み続ける。
8. The 中継 shall 行の長さと応答の長さに自分の上限を設けない（areka が受ける大きさの要求と、areka が返す大きさの応答〔画像を含む `dump_surface`・`dump_balloon` の結果〕を欠けずに通す）。
9. When 標準入力が閉じる, the 中継 shall 扱っている途中の要求の応答を 5 秒を上限に書いてから（rmcp の受け手が標準入力の閉じで持つ掃き出しの上限・設定で変えられない）、終了コード 0 で終わる。5 秒を越えて続く要求の応答は書かれない（標準入力を閉じた Desktop に読み手は居ない＝設計ディスカッション議題 2 の裁定 2026-10-10）。
10. The 中継 shall areka が受ける 5 つの版（`initialize` で始める `2025-11-25`・`2025-06-18`・`2025-03-26`・`2024-11-05` と、`server/discover` で始める `2026-07-28`）のどれで話すクライアントにも、areka につながっている間は、areka へ HTTP で直接つないだときと同じ意味の結果を返す（ツールの一覧・ツールの呼び出しの結果・areka の JSON-RPC のエラー）。

### Requirement 2: つなぐ先の areka を決める

**Objective:** As a Claude Desktop の利用者, I want areka がどの番号で待ち受けていても、SSP が同じ番号を使っていても、設定を書き換えずに areka へつながること, so that 知らないうちに SSP のゴーストを操作してしまうことがなく、areka を起動し直してもそのまま使える

#### Acceptance Criteria

1. While `AREKA_MCP_PORT` が設定されていない, the 中継 shall areka 本体が待受に試すのと同じ候補を同じ順（9801 → 9821 → 9802 → 9822 → … → 9810 → 9830 の 20 個）で当たり、`127.0.0.1` のその番号で areka を探す。
2. When 当たった番号で応えた相手が areka の MCP サーバーだと名乗る（`serverInfo` の `name` が `areka-mcp-server`）, the 中継 shall その相手をつなぐ先に決め、残りの候補を当たらない。つなぐ先を決めた後は、その相手が areka として応える間、候補を先頭から当たり直さない（2.1 の順は、つなぐ先が決まっていないときの探し方）。「当たらない」は要求を書かないことを指し、探索を速くするために残りの候補へ TCP の接続を開くだけ（1 バイトも書かずに閉じる）は許す（設計 D-5・机では居ない番号への接続が断られるまで約 2 秒かかるため）。
3. If 当たった番号で応えた相手が areka の MCP サーバーだと名乗らない（`ssp-mcp-server` と名乗る SSP・MCP でない HTTP のサーバー・HTTP でない相手を含む）, then the 中継 shall Desktop から届いた要求をその相手へ 1 つも届けず、標準エラーにその番号と理由を残して、次の候補へ進む。
4. When `AREKA_MCP_PORT` に 1〜65535 の番号（前後の空白は許す）が設定されている, the 中継 shall その番号だけを当たり（ほかの候補を当たる回数 0）、その番号の相手にも 2.2・2.3 と同じ確かめを行う。
5. If `AREKA_MCP_PORT` が数として読めない・範囲の外である, then the 中継 shall 標準エラーに理由を 1 件残し、既定の 20 候補で探す。
6. If `AREKA_MCP_PORT` が `0` である, then the 中継 shall どこへもつながず、控えも使わずに、`id` を持つどの要求にも「areka が MCP を待ち受けない設定になっている」旨の JSON-RPC のエラー（番号 `-1`・`Server not available` で始まる英文）で答える。
7. The 中継 shall `127.0.0.1` 以外へつながない（宛先の機械を変える設定 0 個）。
8. The 中継 shall 起動の引数なしで動き、つなぐ先を変える手段を `AREKA_MCP_PORT` の 1 つだけにする。
9. If 当たった番号に居る相手が、接続は受けるが返事をしない, then the 中継 shall 設計で決めて記録する限りのある時間でその候補を見切り、次の候補へ進む。
10. When つないでいた areka が終了し、同じ番号または別の候補の番号で起動し直した, the 中継 shall 中継を起動し直さなくても、その後に届いた要求を新しい areka へ届ける。areka が終了したままなら、その後に届いた要求には要件 3 の決まりで答える。
11. If つないでいた areka が終了した後に、同じ番号を areka でないものが取った, then the 中継 shall Desktop から届いた要求をその相手へ 1 つも届けず、2.1〜2.4 の決まりで areka を探し直す。

### Requirement 3: areka が居ないときの答え（控え・自前の答え・エラー）

**Objective:** As a Claude Desktop の利用者, I want Desktop を areka より先に起動していても areka のツールが並び、areka を起動すればそのまま使えること, so that 起動の順番を気にせずに済み、areka が居ない間は理由が Desktop の側で読める

ここで「areka が見つからない」は、要件 2 の決まりで当たった番号のどこにも areka の MCP サーバーが居ないこと（何も居ない・areka でないものだけが居る、の両方）を指す。`AREKA_MCP_PORT` が `0` のときは要件 2.6 が先に立ち、この要件は当たらない。

#### Acceptance Criteria

1. When 中継が areka からサーバーの情報（名乗り・できること・説明の文）を受け取る, the 中継 shall それを控えに取り、前のサーバーの情報の控えを置き換える。
2. When areka が `tools/list` に返した結果（エラーでないもの）を中継が Desktop へ返す, the 中継 shall その結果を控えに取り、前のツールの一覧の控えを置き換える。
3. The 中継 shall 控えを、中継が終了して起動し直された後も読める形で持つ（Desktop が次に起動する中継が、前の中継の取った控えで答えられる）。
4. While areka が見つからず、サーバーの情報の控えがある, when 最初の問い合わせ（`initialize`、または `initialize` を使わない版の `server/discover`）が届く, the 中継 shall 控えのサーバーの情報（名乗り・できること・説明の文）で答える（版は Desktop と中継の間で決まった版）。`tools/list` には、ツールの一覧の控えで同じに答える。
5. If areka が見つからず、届いた最初の問い合わせ（または `tools/list`）に当たる控えが無い, then the 中継 shall 3.6 と同じエラーで答える。
6. If areka が見つからないときに、最初の問い合わせ・`tools/list`・`ping` 以外の `id` を持つ要求（`tools/call`・未知のメソッドを含む）が届く, then the 中継 shall その要求の `id`・番号 `-1`・`Server not available` で始まり areka が見つからない旨が続く英文の `message` を持つ JSON-RPC のエラーを 1 行で返す。areka でない相手が応えた番号のうち、名乗りを読めた相手の分は、その番号と名乗りを `message` の続きに含める。名乗りを読めない答えを返した相手（MCP でない HTTP のサーバー・HTTP でない相手）と返事をしない相手の番号は、標準エラーにだけ残す。つないでいた areka へ届ける前に失敗し、探し直して見つけた areka へも届ける前に失敗した要求（2 度とも areka へ書いていない）には、番号と名乗りを続けない「areka が見つからない」のエラーで答える（2026-10-11 の裁定 3）。
7. While areka が見つからない, when `ping` が届く, the 中継 shall 自分で、その要求の `id` と空の `result`（`{}`）を持つ応答を 1 行で返す。
8. If areka が見つからないときに通知が届く, then the 中継 shall 標準出力へ何も書かない。
9. While areka につながっている, the 中継 shall 控えで答えない（最初の問い合わせには areka の今のサーバーの情報で、`tools/list` と `tools/call` には areka の今の答えで応える）。ここで「areka の今のサーバーの情報」は、中継がその areka へのつなぎを作ったときの握手で受け取った情報を指し、そのつなぎが生きている間は同じ（areka が同じ番号で起動し直しても変わらない。`tools/list` と `tools/call` は毎回 areka の今の答え＝2026-10-11 の裁定 4）。areka が居ない間に控えで答えた後で areka が見つかったときも、その後の要求には areka の今の答えで応え、3.1・3.2 の決まりで控えを新しい中身に置き換える。
10. If 控えを読めない（壊れている・途中まで）・書けない, then the 中継 shall 標準エラーに理由を残し、読めない控えは無いものとして扱い、要求の中継とエラーの応答を続ける（終了しない）。
11. While 2 つ以上の中継が同時に動いている, when それぞれが同じ種類の控えを書く, the 中継 shall 控えをどれか 1 つの応答の全体にする（混ざった中身・途中までの中身にしない）。

### Requirement 4: areka の応答がおかしいとき

**Objective:** As a Claude Desktop の利用者（と Desktop の上で動くエージェント）, I want areka とのやりとりが途中で壊れたときも、理由が Desktop の側で読めて中継が止まらないこと, so that 次に何をすればよいかが分かり、Desktop を起動し直さずに続けられる

#### Acceptance Criteria

1. If つないだ areka が、`id` を持つ要求に本文の無い応答を返す、または応答の途中で接続を閉じる, then the 中継 shall その要求の `id`・番号 `-1`・`No response from server` で始まる英文の `message` を持つ JSON-RPC のエラーを 1 行で返す。
2. If areka の応答が 4.3 の言う「本文が JSON である」応答でない（札 `Content-Type` が `application/json` でない応答〔HTTP のエラーに付く平文を含む。本文がたまたま JSON として読めても同じ〕と、札が `application/json` でも本文が JSON-RPC のメッセージとして読めない応答。本文の無い応答は 4.1）, then the 中継 shall その要求の `id`・番号 `-1`・`Invalid JSON response from server` で始まる英文の `message` を持つ JSON-RPC のエラーを 1 行で返し、HTTP の状態と本文の頭を標準エラーに残す。
3. When areka の応答の本文が JSON である（札 `Content-Type` が `application/json` で、本文が JSON-RPC のメッセージとして読める応答を指す。ほかの札の応答は、本文が JSON として読めても 4.2 で扱う＝2026-10-11 の裁定 2）, the 中継 shall HTTP の状態（200・400 など）に関わらず、その中身を返す（areka の JSON-RPC のエラーは、番号・文・付いているデータを同じにして返し、中継のエラーに作り替えない）。
4. If areka が要求に応えないまま、設計で決めて記録する上限の時間が過ぎる, then the 中継 shall その要求に 4.1 と同じ形のエラーで答え、次の要求を扱える状態に戻る。上限は、areka のツールが答えるまでに掛けうる最も長い時間より長くする。
5. The 中継 shall areka へ届けた要求を、応答が得られなかったことを理由に送り直さない（同じ要求が areka に 2 回届く回数 0）。
6. The 中継 shall どのエラーの後も終了せず、次の行を読み続ける。

### Requirement 5: Desktop に起動されるプロセスとしての作法

**Objective:** As a Claude Desktop の利用者, I want 中継が Desktop とのやりとりを汚さず、調べたいときは記録が残っていること, so that つながらないときに Desktop の記録（`mcp-server-<名>.log`）で原因を追える

#### Acceptance Criteria

1. The 中継 shall 標準出力へ JSON-RPC の応答の行だけを書く（記録・進みの表示・空の行 0 件）。
2. The 中継 shall 記録（つないだ先の番号・飛ばした候補とその理由・控えで答えたこと・エラーの詳しい中身）を標準エラーへ書く。
3. The 中継 shall 窓・ダイアログ・通知を出さない。
4. While 中継が 2 つ以上同時に動いている, the 中継 shall 互いの要求と応答に影響しない（1 つしか動けない仕組みを持たない）。

### Requirement 6: help の Claude Desktop 用の設定例

**Objective:** As a はじめて areka を Claude Desktop につなぐ人, I want help の 1 ページから設定を写せること, so that 中継の置き場を自分で調べずに済む

#### Acceptance Criteria

1. When `GET /api/mcp/help` が届く, the areka shall Claude Desktop の節に、設定ファイル `claude_desktop_config.json` へそのまま貼れる `mcpServers` の例を載せる。例の `command` は、動いている `areka.exe` と同じフォルダにある中継 `areka-mcp-bridge.exe` の絶対パスにする。
2. The areka shall 設定例を、パスに `\`・空白・`&` などが含まれていても、ページに表示された文字を貼れば正しい JSON になる形で載せる。
3. While areka が既定の 20 候補のどれかで待ち受けている, the areka shall 設定例に `AREKA_MCP_PORT` を載せない（中継が自分で見つける）。
4. While areka が既定の 20 候補の外の番号で待ち受けている, the areka shall 設定例の `env` に `AREKA_MCP_PORT` をその番号で載せる。
5. The areka shall Claude Desktop の節に、設定を書くファイルの名前と、書いた後に Desktop を起動し直すこと（窓を閉じるだけでは Desktop は終了しないので、タスクバーの通知領域のアイコンから終了してから起動し直す旨を添える＝2026-10-11 の裁定 1）を載せ、「中継は今後の版で用意します」の文を載せない。
6. The areka shall help のほかの節（待ち受けている URL・Claude Code の登録コマンド・Cursor の設定の断片・ポートの決まり方と変え方・areka 独自のツール）を今のまま載せる。

### Requirement 7: 配布物への同梱

**Objective:** As a areka を zip で受け取る人, I want 展開したフォルダに中継が入っていて、そのまま Desktop から起動できること, so that 別に何かを入手したり入れたりせずに済む

#### Acceptance Criteria

1. The 配布スクリプト shall x64 の zip に x64 の中継を、arm64 の zip に arm64 の中継を、`areka.exe` と同じ階層に `areka-mcp-bridge.exe` の名前で入れる。
2. If zip に中継が無い、または中継の CPU 種別が zip の種別と合わない, then the 配布スクリプトの検査 shall その zip を赤にする（`areka.exe` に対する今の検査と同じ形で中継を見る）。
3. The 中継 shall 単体で動く（Node.js などの実行環境・追加のインストール・`areka.exe` と同じ確認で分かる追加のランタイムを要しない）。
4. The 説明書 `dist/README.txt` shall 本体の同梱物の見出しに中継を載せ、Claude Desktop からつなぐ手順の節（areka を起動して help のページを開き、設定例を写すこと・Desktop を起動し直すときは窓を閉じるだけでは終了しないこと〔2026-10-11 の裁定 1〕・Desktop を先に起動していても前の答えの控えでツールが並ぶこと）と、`AREKA_MCP_PORT` の意味（番号の指定・`0` で待ち受けない）と、中継の控えの置き場（設計で決めた場所）を載せる。
5. When 新しいクレートを足した後に全体テスト（`tools/test-all.ps1 -Format -License`）を回す, the ワークスペース shall 緑で終わる（ライセンスの検査・謝辞の一覧の作り直し・crates.io へ出さない印と理由の行を含む）。

### Requirement 8: 確かめ

**Objective:** As a 後で中継や受け口を直す開発者, I want 中継の振る舞いが実行のたびに同じ結果になるテストと記録で固定されていること, so that 変更で Desktop からの使い勝手が壊れたら赤で分かる

#### Acceptance Criteria

1. The 中継 shall ループバックの偽のサーバー（OS が割り当てる空きポート・固定の番号を束ねるテスト 0 本・ネットへ出ない）を相手に、控えをテストごとの一時の置き場（ワークツリーの `target` の下）に置いて（利用者の本物の控えを読み書きする回数 0・机の本物の 9801〜9830 へ接続するテスト 0 本）、次を決定論テストで固定する:
   - **中継の基本**（要件 1）: 要求と応答の往復（数の `id`・文字列の `id`）・通知で出力なし・改行入りの応答が 1 行になる・空の行・JSON でない行の `-32700` とその次の行が通ること・オブジェクトでない行の `-32600`・大きな応答が欠けないこと・標準入力が閉じて終了コード 0・標準出力に応答の行しか無いこと。
   - **つなぐ先**（要件 2）: areka と名乗る相手へつなぐ・areka と名乗らない相手を飛ばして次の候補の areka へつなぐ（飛ばした相手に Desktop の要求が届いた数 0）・areka と名乗らない相手だけのときのエラーに番号と名乗りが入る・`AREKA_MCP_PORT` の 4 通り（番号・`0`・読めない値・未設定）・指定した番号の相手が areka と名乗らないときのエラー・返事をしない相手の見切り・起動し直した areka へ届くこと・areka が去った番号を areka でないものが取ったときに要求が届かないこと。
   - **areka が居ないとき**（要件 3）: `initialize` と `tools/list` の応答が控えに取られる・別に起動した中継が控えの中身で答える（`initialize` で始める版と `server/discover` で始める版の両方）・控えが無いときの最初の問い合わせと `tools/list` のエラー・自前の `ping`・`tools/call` のエラー・通知で出力なし・areka が見つかった後は areka の応答が返り控えが置き換わる・エラーの応答は控えに取られない・壊れた控えは無いものとして扱われる・控えを書けなくても中継が続く・同時に書いても控えが混ざらない。
   - **応答がおかしいとき**（要件 4）: 壊れた応答・本文の無い応答・途中で閉じる接続・応答の上限の時間・JSON の本文が HTTP の状態に関わらず返る・送り直しが 0 回・エラーの後も続くこと。
2. If 中継の候補の並びが areka 本体の待受の候補の並びと食い違う、または中継が areka と見なす名乗りが areka 本体の名乗りと食い違う, then the 全体テスト shall 赤になる。
3. The areka shall help の決定論テストで、Claude Desktop の節の設定例（中継の絶対パス・JSON として読めること・`env` の有無の 2 通り）と、「今後の版で用意します」の文が無いことを固定する（設定ファイルの名前が載っていないことを確かめていた今の期待は書き換える）。
4. The 本 spec shall SSP の中継と areka の中継が同じ入力に何を返すかを行ごとに並べた記録を `doc/ssp-mcp/` の下に 1 つ持ち、違う行には違う理由を書く（入力の切り方・改行入りの応答・エラーの文の続き・areka でない相手の扱い・控えの置き場・`server/discover` に控えで答えること・`AREKA_MCP_PORT` が `0` のときを含む）。比べるのは字の並びではなく意味（欄と値）。
5. The 本 spec shall 実機で確かめ、結果を `.kiro/specs/areka-P0-mcp-stdio-bridge/verification/signoff.md` に残す: ⑴ 配布スクリプトで作った zip を展開して `areka.exe` を起動し、help の設定例を Claude Desktop の設定に写して Desktop を起動し直すと、Desktop に areka のツールが並び、`get_status` など 1 本以上が呼べる、⑵ そのとき Desktop が話した版を記録する、⑶ areka を止めた状態で Desktop からツールを呼ぶと、理由の読めるエラーが Desktop に出て、areka を起動し直すと Desktop を起動し直さずに呼べる、⑷ areka を止めたまま Desktop を起動し直すと、控えで areka のツールが並び、その後に areka を起動すると Desktop を起動し直さずに呼べる、⑸ SSP が 9801 で待ち受けている机で areka を起動すると、Desktop から呼んだ `get_active_ghost_list` が areka のゴーストを返す（SSP のゴーストを返さない）、⑹ 配布スクリプトの検査（`-Check`）が緑。
