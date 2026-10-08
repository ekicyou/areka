# areka 独自の MCP ツール（2026-10-08 時点）

SSP の MCP サーバに無く、areka だけが出すツールの約束の正本。今は `check_script` の 1 本。ツールの名前・引数・結果の形・診断の種類と文面・SSP との違い・あとからツールを 1 本足す手順を書く。SSP と同じ 10 本の約束は [survey.md](survey.md)、輸送の差は [transport-diff-areka.md](transport-diff-areka.md) にある。

- **areka の根拠**: ツールの定義・引数・結果の組み立ては `crates/areka-mcp/src/tools/check_script.rs`、独自のツールの表と登録は `crates/areka-mcp/src/tools/mod.rs` の `OWN_TABLE` と `all_entrances`。答えを作る処理は `crates/areka/src/mcp/check_script.rs`（事実を集めて別のスレッドへ渡す）と `crates/areka/src/mcp/check_script_judge.rs`（診断を決める純粋な関数と、文面の定数）
- **文面の正本**: 下の ⑶ の文面が正本で、`check_script_judge.rs` の文面の定数はこれを写したもの。文面を変えるときは、この文書と定数を同じ変更で直す（両者の一致はテストで見ていない）

## ⑴ 独自のツールとは

- SSP に無い、areka だけのツール。名前に接頭辞は付けない（`check_script` のまま）。
- `tools/list` では、SSP と同じ 10 本の**後ろ**に並ぶ（今は 11 本で、11 本目が `check_script`）。先頭の 10 本の定義と並びは SSP 2.9.05 の `tools/list`（[tools-list-ssp-2.9.05.json](tools-list-ssp-2.9.05.json)）と同じのまま変えない。
- 引数の検査・`ghost_name` の解決・答えの待ちの上限・記録は、SSP と同じ 10 本と同じ規則で動く。
  - 引数が `inputSchema` に合わない（必須が無い・型が違う）→ JSON-RPC の `-32602`。11 本に無い名前も `-32602`。
  - `ghost_name` を省くと起動中の 1 体が宛先。解決できないときは `isError: true` で、`ghost_name` を省いて起動中のゴーストが居なければ `NG:Specified ghost is not active`、`ghost_name` を渡してそれに当たるゴーストが居なければ（起動中のゴーストが居ないときも）`NG:Cannot find active ghost from specified name`。
  - 10 秒のうちに答えが出なければ `NG:areka did not respond within 10 seconds`、終了の途中なら `NG:areka is shutting down`（どちらも `isError: true`）。
- 接続の時点で分かるようにしてある: サーバーの指示文（`initialize` の `instructions`）に `check_script` が SSP に無い areka 独自のツールで、ゴーストに何もさせずに台本を確かめるので `sakurascript` の前に使う、と英文で書いてある（`crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS`）。登録案内のページ（`GET /api/mcp/help`）にも「areka 独自のツール」の節があり、名前と日本語の 1 行が出る（`crates/areka-mcp/src/help.rs`。名前は登録と同じ定義から読む）。

## ⑵ `check_script` の引数と結果

台本を**再生せずに**読み、宛先のゴーストが今使っているシェルとバルーンに照らして、areka で効かない箇所を位置つきで返す。ゴーストは喋らず、動かず、表情も変わらない（台本の再生・SHIORI への要求・表示の変化・ファイルの書き込み・ネットへの送信のどれもしない）。タグ 1 つだけの台本を渡せば、そのタグが areka で効くかを確かめる用途にも使える。

引数:

| 名前 | 型 | 必須 | 中身 |
|---|---|---|---|
| `script` | string | 必須 | 確かめる台本 |
| `ghost_name` | string | 任意 | 宛先のゴーストの名前か、ゴーストのフォルダのフルパス。省くと起動中の 1 体 |

結果:

| 場合 | `isError` | 本文 |
|---|---|---|
| 診断 0 件 | `false` | 1 行だけ: `OK:0 diagnostics` |
| 診断 n 件（n ≥ 1） | `false` | 1 行目 `OK:<n> diagnostics`、続けて診断 1 件につき JSON の 1 行 |
| 窓の無いゴースト（表示の準備に失敗し、記録だけで動いている） | `false` | 1 行目の末尾に ` (surface and balloon IDs were not checked: this ghost has no window)` が付く。無い surface・無いバルーンの診断は出ない |
| `ghost_name` を解決できない・時間切れ・終了の途中 | `true` | ⑴ の `NG:` の文言 |

- 誤りが在っても、検査そのものは成功なので `isError: false`。`isError: true` は検査ができなかったときだけ。
- 誤りの有無は 1 行目の数で機械的に見分ける（`^OK:(\d+) diagnostics`）。
- 診断の 1 行は JSON のオブジェクトで、欄は次の 5 つ。欄の並びは約束しない。綴りに改行や引用符が在っても、エスケープされて 1 件が 1 行に収まる。

| 欄 | 中身 |
|---|---|
| `kind` | 診断の種類（⑶ の表の名前） |
| `start` | 該当箇所の先頭の位置 |
| `end` | 該当箇所の末尾の位置（この位置の文字は含まない） |
| `text` | 該当箇所の綴りそのまま |
| `message` | 何が起きているか・再生でどうなるかの英文 1 文（⑶ の文面） |

- **位置の単位は文字**。台本の先頭を 0 として数えた文字の数（Unicode の符号位置の数）で、バイトでも UTF-16 の単位でもない。例: `あいう\xえお` の `\x` は `start: 3`・`end: 5`。主な手がかりは `text` で、位置は同じ綴りが 2 度出るときの区別に使う。
- 診断は台本の順に並ぶ。1 つの命令に 2 件付くことがあり、そのときは 2 件が同じ位置になる（例: `\![*]\q[題]` は `ignored` と `unreadable_argument` の 2 件）。
- 影響の大きさ（どのくらい困るか）の欄は無い。

例（宛先のシェルに surface 99999 が無い場合）:

```
OK:3 diagnostics
{"end":2,"kind":"unknown_tag","message":"areka does not know this tag; playback drops it","start":0,"text":"\\x"}
{"end":11,"kind":"missing_surface","message":"no such surface in the current shell; playback does not change the surface","start":2,"text":"\\s[99999]"}
{"end":20,"kind":"ignored","message":"areka accepts this but it has no effect yet","start":11,"text":"\\f[sub,1]"}
```

## ⑶ 診断の種類と文面

種類は 6 つ。文面の形は「何が起きているか; 再生でどうなるか」の英文 1 文（小文字で始め、句点なし）。

| `kind` | 場合 | `message` |
|---|---|---|
| `unknown_tag` | — | `areka does not know this tag; playback drops it` |
| `unknown_command` | — | `no part of areka handles this \! command; playback ignores it` |
| `missing_surface` | — | `no such surface in the current shell; playback does not change the surface` |
| `missing_balloon` | — | `no such balloon ID in the current balloon; playback does not change the balloon` |
| `unreadable_argument` | 閉じていない | `the bracket or quote is not closed; playback drops everything from here to the end` |
| `unreadable_argument` | 既定の値へ落とす | `the argument is missing or unreadable; playback uses the default value` |
| `ignored` | — | `areka accepts this but it has no effect yet` |

何を診るか（全数）。どの行も、判定は再生が実際に使う関数の答えをそのまま使う（検査のための別の判定を持たない）ので、検査と再生の扱いは食い違わない。

| # | `kind` | 何を言うか | 出る台本の例 | 出ない台本の例 |
|---|---|---|---|---|
| 1 | `unknown_tag` | 角括弧なしの知らないタグ | `\x` | `\e` |
| 2 | `unknown_tag` | 角括弧つきの知らないタグ（`\i`・`\&` を含む） | `\i[5]`・`\&[amp]` | `\s[0]` |
| 3 | `unknown_tag` | 旧い 2 連の `\q` | `\q[ID][題]` | `\q[題,ID]` |
| 4 | `unknown_command` | 誰も拾わない `\!` の名前 | `\![nosuch]`・`\![raise,OnX]` | `\![open,readme]`・`\j[http://a/]` |
| 5 | `unknown_command` | 名前は在るが、その第 1 引数を誰も拾わない | `\![set,nosuch]` | `\![set,zorder,0,1]` |
| 6 | `missing_surface` | 今のシェルに無い数の ID・範囲外の数 | `\s[99999]`・`\s[-2]` | `\s[0]`・`\s[-1]` |
| 7 | `missing_surface` | 今のシェルの別名の表に無い名前 | `\s[無い名前]` | 表に在る名前 |
| 8 | `missing_balloon` | 今のバルーンに無い ID・範囲外の数 | `\b[99]`・`\b[-2]` | `\b[0]`・`\b[-1]`・`\b[名前]` |
| 9 | `unreadable_argument` | 閉じていない `[`／`"`（そこから台本の末尾までが捨てられる） | `\s[0` | `\s[0]` |
| 10 | `unreadable_argument` | 台本を読む段が既定の値へ落とした引数 | `\_w[abc]`・`\n[abc]`・`\p[x]`・`\q[題]` | `\_w[500]`・`\n[half]`・`\p[1]`・`\q[題,ID]` |
| 11 | `unreadable_argument` | `\![set,choicetimeout,…]` の読めない時間 | `\![set,choicetimeout,abc]` | `\![set,choicetimeout,5000]` |
| 12 | `ignored` | 選択肢マーカー | `\![*]` | `\q[題,ID]` |
| 13 | `ignored` | 受け取るが表示を変えない `\f` のキー | `\f[sub,1]`・`\f[align,center]`・`\f[height,large]` | `\f[bold,1]` |
| 14 | `unknown_tag` | 知らない `\f` のキー・キーの無い `\f`（値の誤りは診ない） | `\f[colour,red]`・`\f[]` | `\f[color,red]`・`\f[bold,abc]` |

- 「誰も拾わない `\!`」は、名前と第 1 引数の組で決まる。拾う組の一覧は `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical`（今 23 組）で、各機能の処理が実際に拾う組と一致することをテスト（`consumer_ledger_agreement_tests.rs`）で固定している。
- `\s`・`\b` は、その場の話し手（`\0`・`\1`・`\h`・`\u`・`\p[n]`）のシェルとバルーンで判定する。
- 表の各行の「出る台本」「出ない台本」は `crates/areka/src/mcp/check_script_judge_tests.rs` の `row01`〜`row14` が固定している。

**名前だけ予約した種類**（今は出さない）:

| `kind` | 出すようになるとき | 今の扱い |
|---|---|---|
| `missing_animation` | areka が `\i` に対応した後（今のシェルに無いアニメーション） | `\i[…]` は表の 2 行目の `unknown_tag` で返る |
| `unknown_entity` | areka が `\&` に対応した後（知らない実体参照） | `\&[…]` は表の 2 行目の `unknown_tag` で返る |

## ⑷ 診ていないもの

`check_script` は、台本の字面だけで決まることを診る。誤って「無い」と答えることを避けるため、次は診ない（診断が出なくても、効くとは限らない）。

- **各機能の処理が自分で読む引数**。`\!` は「その名前と第 1 引数を誰かが拾うか」までを診て、拾った後の引数の誤りは診ない。
  - `\![move,…]`・`\![set,zorder,…]`／`\![reset,zorder,…]`・`\![bind,…]`
  - `\![change,ghost|shell|balloon,…]`（`\+`・`\_+` を含む）
  - `\![open,readme|file|browser|explorer|editor|mailer,…]` と `\j[…]`（例: `\![open,readme,種類,名前]` の後ろの引数）
  - `\![execute,install,…]`・更新の 3 つ（`\![updatebymyself]`・`\![update,…]`・`\![updateother,…]`）
  - `\_l[x,y]` の座標
- **`\f` の値の誤り**（`\f[bold,abc]` など）。知らないキーとキーの無い `\f` だけを `unknown_tag` で返す（⑶ の 14 行目）。
- **`%` の変数**（`%username` など）。未対応の名前も診ない。SHIORI が置き換える中身も診ない。
- **台本の字面で決まらないもの**
  - `\![change,shell,…]`・`\![change,ghost,…]`（`\+`・`\_+` を含む）の後ろの `\s`、`\![change,balloon,…]`・`\![change,ghost,…]` の後ろの `\b`（切り替えた先のシェルとバルーンは字面で分からない）。ほかの種類は切替の後ろも診る。
  - 名前の形の `\b[名前]`（その時に表示している絵の中の箱に依る）。
  - 絵やバルーンを持たない話し手での `\s`・`\b`（例: 窓の無い `\p[2]`）。
  - `\![raise,…]` で呼んだ先の台本の中身（`\![raise]` 自体は、今は誰も拾わないので `unknown_command` になる）。
  - 文字そのもの。

あわせて知っておくこと:

- **`\e`・`\-` の後ろも診る**。再生はそこで止まるので後ろは再生されないが、書いてある誤りは知らせる。
- **答えは、呼んだ時点のシェルとバルーンに対するもの**。areka は呼ばれた時点のシェルとバルーンの事実を写し取ってから診るので、診ている間にシェルが切り替わっても、答えは写し取った時点のシェルに対するものになる。
- **10 秒の上限に当たる場合がある**。起動の直後で表示の準備がまだ済んでいない間は、済むのを待ってから答える。準備がいつまでも済まないときと、診るのに 10 秒を超えるほど長い台本のときは、⑴ の時間切れ（`NG:areka did not respond within 10 seconds`・`isError: true`）になる。台本の長さの上限は別に置いていない（MCP の本文の上限 4 MiB だけ。超えると HTTP の 413）。

## ⑸ SSP との違い

| 項目 | areka | SSP |
|---|---|---|
| `check_script` | 在る（`tools/list` の 11 本目） | 無い |
| 診断の基準 | **areka で**効くかどうか。SSP では効くタグでも、areka が対応していなければ診断が出る（例: `\i[…]`・`\&[…]` は `unknown_tag`、`\![raise,…]` は `unknown_command`） | — |
| 台本の誤りを知る方法 | 再生する前に `check_script` で知る（再生中の記録は `mcp-strict-errors` が作る予定） | `sakurascript` などの `strict` で、再生中の誤りをエラーログへ残す |
| 種類の名前と文面 | ⑶ のとおり。再生中の記録（`mcp-strict-errors`）も同じ名前と文面に揃える予定 | エラーログの文面は測っていない |

- SSP と同じ 10 本の答えは、`check_script` が増えても変わらない（SSP 向けの道具からは、11 本目が増えたことだけが見える）。

## ⑹ あとからツールを 1 本足す手順

独自のツールを足す spec（`mcp-user-response`・`mcp-shiori-query` など）が触るファイルは次のとおり。SSP と同じ 10 本の表と、そのテストには触らない。

| 区分 | ファイル | すること |
|---|---|---|
| 新規 | `crates/areka-mcp/src/tools/<ツール>.rs`（＋兄弟のテスト） | 定義（`DEFINITION`）・型の付いた引数（`Args`）・詰め替え（`parse`）・help に載せる日本語の 1 行（`SUMMARY_JA`。`<`・`>`・`&` を書かない） |
| 新規 | `crates/areka/src/mcp/<ツール>.rs`（＋兄弟のテスト） | 答えを作る処理（`handle`） |
| 追記 | `crates/areka-mcp/src/tools/mod.rs` | `pub mod`・`ToolCall` の変種と `name` の腕・`OWN_TABLE` の 1 行（表の長さの数も 1 増やす） |
| 追記 | `crates/areka/src/mcp/mod.rs` | `mod`・`dispatch` の腕 1 本（`ghost_name` を解決するなら `resolved!` を通す） |
| 追記 | `crates/areka-mcp/src/handler.rs` | `INSTRUCTIONS` にツールの名前を足す |
| 追記 | `crates/areka-mcp/src/tools/tools_own_tests.rs`・`tools_own_socket_tests.rs` | 本数（11 → 12）と名前の並び |
| 追記 | `doc/ssp-mcp/areka-tools.md`（この文書） | ツールの節 |
| 触らない | `TABLE`・`entrances`・`tools_tests.rs`・`tools_socket_tests.rs`・`registry.rs`・`bridge.rs`・`help.rs`・`dispatch.rs`・`server.rs` | — |

- 登録の順は `TABLE` の 10 行 → `OWN_TABLE` の行で、これがそのまま `tools/list` の並びになる。`all_entrances` も help も `OWN_TABLE` の同じ行を読むので、名前を手で 2 か所に書く所は無い。
- 名前が SSP の 10 本と重なると 10 本の側が置き換わる。`tools_own_tests.rs` の名前の重複 0 の検査が赤で知らせる。
- `ToolCall` に変種を足すと、`dispatch` の網羅の `match` がコンパイルで足し忘れを知らせる。
