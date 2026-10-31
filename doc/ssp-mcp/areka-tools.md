# areka 独自の MCP ツール（2026-10-10 時点）

SSP の MCP サーバに無く、areka だけが出すツールの約束の正本。今は `check_script` の 1 本。ツールの名前・引数・結果の形・診断の種類と文面・SSP との違い・あとからツールを 1 本足す手順を書く。あわせて、SSP と同じツールの `strict` で残る再生中の記録の約束も ⑺ に書く。SSP と同じ 10 本の約束は [survey.md](survey.md)、輸送の差は [transport-diff-areka.md](transport-diff-areka.md) にある。

- **areka の根拠**: ツールの定義・引数・結果の組み立ては `crates/areka-mcp/src/tools/check_script.rs`、独自のツールの表と登録は `crates/areka-mcp/src/tools/mod.rs` の `OWN_TABLE` と `all_entrances`。答えを作る処理は `crates/areka/src/mcp/check_script.rs`（事実を集めて別のスレッドへ渡す）と `crates/areka/src/mcp/check_script_judge.rs`（診断を決める純粋な関数と、文面の定数）
- **文面の正本**: 下の ⑶ の文面が正本で、文面の定数（`crates/areka-sakura/src/strict.rs` の `MSG_*`。`check_script_judge.rs` の定数はこれを指す別名）はこれを写したもの。文面を変えるときは、この文書と定数を同じ変更で直す。⑶ の 2 つの表と ⑺ の検出点の表は、strict の記録のテスト（`crates/areka/src/emo2_boot/strict_judges_tests.rs`）がこの文書から読んで記録と比べる。表は `<!-- …:begin -->`〜`<!-- …:end -->` の印で挟んであるので、印と表の列の並びを崩さない

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

種類は 7 つ。文面の形は「何が起きているか; 再生でどうなるか」の英文 1 文（小文字で始め、句点なし）。strict の記録（⑺）も同じ種類と文面を使う。

<!-- diagnostic-kinds:begin -->

| `kind` | 場合 | `message` |
|---|---|---|
| `unknown_tag` | — | `areka does not know this tag; playback drops it` |
| `unknown_command` | — | `no part of areka handles this \! command; playback ignores it` |
| `missing_surface` | — | `no such surface in the current shell; playback does not change the surface` |
| `missing_balloon` | — | `no such balloon ID in the current balloon; playback does not change the balloon` |
| `unreadable_argument` | 閉じていない | `the bracket or quote is not closed; playback drops everything from here to the end` |
| `unreadable_argument` | 既定の値へ落とす | `the argument is missing or unreadable; playback uses the default value` |
| `unreadable_argument` | 出せない文字の番号 | `areka cannot show this character number; playback drops it` |
| `ignored` | — | `areka accepts this but it has no effect yet` |
| `unpaired_tag` | 閉じが無い | `the anchor is not closed; playback extends it to the end of the script` |
| `unpaired_tag` | 開いている間の新しい開き | `a new anchor opens before the previous one closes; playback closes the previous one here` |
| `unpaired_tag` | 開いていない所の閉じ | `no anchor is open; playback ignores this close` |

<!-- diagnostic-kinds:end -->

何を診るか（全数）。どの行も、判定は再生が実際に使う関数の答えをそのまま使う（検査のための別の判定を持たない）ので、検査と再生の扱いは食い違わない。

<!-- diagnostic-rows:begin -->

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
| 15 | `unpaired_tag` | 閉じ `\_a` の無いアンカーの開き（アンカーは台本の終わりまで続く） | `\_a[x]` | `\_a[x]あ\_a` |
| 16 | `unpaired_tag` | アンカーが開いている間の新しい開き（診断は新しい開きに付く） | `\_a[x]あ\_a[y]い\_a` | `\_a[x]あ\_a\_a[y]い\_a` |
| 17 | `unpaired_tag` | アンカーが開いていない所の閉じ | `\_a` | `\_a[x]あ\_a` |
| 18 | `unreadable_argument` | `\_u[…]`・`\_m[…]` の出せない番号（0・制御の文字・片割れのサロゲート・U+10FFFF より上）と読めない番号（再生はその埋め込みを捨てる） | `\_u[0]`・`\_u[0x09]`・`\_u[0xD83D]`・`\_u[0x110000]`・`\_u[abc]`・`\_m[0]` | `\_u[0x3000]`・`\_m[65]`・`\_u[ 0X22ee,1]` |

<!-- diagnostic-rows:end -->

- 「誰も拾わない `\!`」は、名前と第 1 引数の組で決まる。拾う組の一覧は `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical`（今 23 組）で、各機能の処理が実際に拾う組と一致することをテスト（`consumer_ledger_agreement_tests.rs`）で固定している。
- `\s`・`\b` は、その場の話し手（`\0`・`\1`・`\h`・`\u`・`\p[n]`）のシェルとバルーンで判定する。
- 表の各行の「出る台本」「出ない台本」は `crates/areka/src/mcp/check_script_judge_tests.rs` の `row01`〜`row17` が固定している。strict の再生でも、18 行の「出る台本」で記録が残り「出ない台本」で残らないことを、`strict_judges_tests.rs` がこの表を読んで確かめる（8 行目の「出ない台本」の `\b[名前]` は、再生では今のサーフェスに在る箱の名前で確かめる。⑺）。
- `unpaired_tag`（15〜17 行目）は、再生が使うのと同じ判定（`areka_sakura::pair_anchors`）の答えを開き・閉じの位置に付けたもので、再生の警告と同じ所に同じ内容で出る。重なった開きが閉じられないまま終わる台本（`\_a[x]あ\_a[y]い`）では、2 つ目の開きに 16 行目・15 行目の順で 2 件付く。`\e`・`\-` の後ろの `\_a` には出さない（再生はそこで読むのをやめるので、開きも閉じも数えない）。

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

- **`\e`・`\-` の後ろも診る**。再生はそこで止まるので後ろは再生されないが、書いてある誤りは知らせる。アンカーの対応の崩れ（`unpaired_tag`）だけは別で、後ろの `\_a` には出さない（⑶）。
- **答えは、呼んだ時点のシェルとバルーンに対するもの**。areka は呼ばれた時点のシェルとバルーンの事実を写し取ってから診るので、診ている間にシェルが切り替わっても、答えは写し取った時点のシェルに対するものになる。
- **10 秒の上限に当たる場合がある**。起動の直後で表示の準備がまだ済んでいない間は、済むのを待ってから答える。準備がいつまでも済まないときと、診るのに 10 秒を超えるほど長い台本のときは、⑴ の時間切れ（`NG:areka did not respond within 10 seconds`・`isError: true`）になる。台本の長さの上限は別に置いていない（MCP の本文の上限 4 MiB だけ。超えると HTTP の 413）。

## ⑸ SSP との違い

| 項目 | areka | SSP |
|---|---|---|
| `check_script` | 在る（`tools/list` の 11 本目） | 無い |
| 診断の基準 | **areka で**効くかどうか。SSP では効くタグでも、areka が対応していなければ診断が出る（例: `\i[…]`・`\&[…]` は `unknown_tag`、`\![raise,…]` は `unknown_command`） | — |
| 台本の誤りを知る方法 | 再生する前に `check_script` で知る。再生中の誤りは、`sakurascript`・`raise_event` の `strict` でエラーログへ残す（⑺） | `sakurascript` などの `strict` で、再生中の誤りをエラーログへ残す |
| 種類の名前と文面 | ⑶ のとおり。strict の記録（⑺）も同じ名前と文面 | strict の記録は `[GHOST/Script] surface not found (99999) at position 2 : <その位置から台本の終わりまで>` の形で、areka と違う（⑺ の「SSP の strict との違い」） |

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

## ⑺ strict の記録

`sakurascript` に `strict: true` を付けて台本を流すか、`raise_event` に `strict: true` を付けてイベントを送りゴーストが台本を返すと、areka はその台本を再生しながら、再生が誤りに当たるたびにエラーログへ記録を 1 件ずつ残す。記録は `get_log`（`log_type=error`）で読む。`check_script` が再生の前に字面で診るのに対し、こちらは実際に再生した所で当たった誤りを残す。

### 答えの文

| ツール | 場合 | 答えの本文（`isError`） |
|---|---|---|
| `sakurascript` | `strict` 真で受け付けた | `OK:strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=<n>)`（`false`） |
| `sakurascript` | `strict` 偽・省略で受け付けた | `OK`（`false`） |
| `raise_event` | `strict` 真で、ゴーストが台本を返した | 1 行目 `OK:the ghost returned a script and it is being played. strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=<n>)`、続けて `Script: <台本>` の行 |
| `raise_event` | `strict` 偽・省略で台本が返った・台本が返らなかった | `strict` を付けないときと同じ（`OK:the ghost returned a script and it is being played.` と `Script: ` の行、または `OK:the ghost returned no script.`） |
| どちらも | 受け付けない・ゴーストが居ない・答えの前にゴーストが降りた など | 今までの `NG:` のまま（記録も残らない） |

- `<n>` は 10 進の数で、台本を送る前にログで最後に振られた番号（エラーに限らない全種類の通しの番号。何も無ければ 0）。再生が終わった後に `get_log` を `log_type=error`・`since_id=<n>` で呼ぶと、その台本の strict の記録が全部入る（同じ間に出たほかの記録も混じる）。

### 記録すること

- 再生が実際に通った所で当たった誤り 1 件につき 1 件。種類は ⑶ の 7 つで、種類の名前と文面は ⑶ の表と 1 字も違わない。名前だけ予約した 2 つ（`missing_animation`・`unknown_entity`）は出さない。
- 誤りに 1 つも当たらなければ記録は 0 件（答えは上の表のまま）。
- 台本が次の 5 つをすべて満たすとき、残る記録は、同じ台本を受け付けた時点で `check_script` に渡したときの診断と同じ集まりになる（種類・位置・綴り・文面・順が同じ）。
  - 中でシェル・バルーン・ゴーストを切り替えない。
  - `\e`・`\-` の後ろに誤りを持たない。
  - 翻訳（`OnTranslate`）で書き換わらない。
  - 最後まで再生される。
  - 名前の形の `\b[名前]` で、箱が見つからない箇所を持たない。
- `check_script` が診ない名前の形の `\b[名前]`（⑷）も、再生で今の絵に箱が見つからなければ `missing_balloon` で記録する。
- 翻訳を通った台本は、翻訳の後の台本を strict のまま再生し、位置も翻訳の後の台本に対して数える（書き換わらなかった台本では、渡した台本の位置と同じ）。

### 記録の形

`get_log` の 1 行は次の形。

```
#<番号> <日時> [Error] <ゴースト名> : <kind> <start>-<end> <message>: <text> point="<点>"
```

例（今のシェルに surface 99999 が無いゴーストに `\s[99999]\x` を流した場合）:

```
#41 2026/10/30 12:00 [Error] 窓際のぱすたさん : missing_surface 0-9 no such surface in the current shell; playback does not change the surface: \s[99999] point="SurfaceAbsent"
#42 2026/10/30 12:00 [Error] 窓際のぱすたさん : unknown_tag 9-11 areka does not know this tag; playback drops it: \x point="ReadUnknownTag"
```

- 表示の語は `Error`。名は台本を再生しているゴーストの名前で、`get_active_ghost_list` が返す値と同じ（`get_log` の `ghost_name` で絞り込むと当たる）。記録は tracing の target `areka::log::error` へ書く。
- `<start>`・`<end>` は再生した台本の先頭を 0 とした文字の数（⑵ と同じ数え方）。`<text>` は該当箇所の綴りそのまま。
- 文面はどれも `:` を含まないので、最初の `: ` までが「種類 位置 文面」、その後ろから最後の ` point="` の手前までが綴り。
- `point` の欄は、下の検出点の表の「点」の名前。
- 本文は 4,096 文字で切られ、末尾に ` ...(truncated)` が付く（とても長い綴りのとき。`point` の欄も切られて落ちる）。`<end>` は切られても元の位置を示す。

### 記録しないこと

- `strict` が偽か省略の台本。
- `NG:` で答えたとき（受け付けない・ゴーストが居ない・答えの前にゴーストが降りた）。
- `raise_event` で、ゴーストが台本を返さなかった・空白だけの台本を返した・イベントを受け付けなかった・送れなかったとき。
- 再生されなかった所の誤り: `\e`・`\-` の後ろと、別の台本・利用者の操作・終了などで途中で打ち切られた後ろ。打ち切られる前に残した記録は消えない。
- strict の台本の中から呼ばれて始まった別の台本（`\![raise,…]` に返った台本など）。記録は strict で受け付けたその台本だけ。
- 窓の無いゴースト（表示の準備に失敗し、記録だけで動いている）での、無い surface・無いバルーン。ほかの種類は記録する。
- `strict` の有無で、台本の再生のしかた・`script` の種類の記録・再生の側が今出している警告の記録は変わらない。

### `[SYSTEM]` の記録との見分け

再生の側は、`strict` に関わらず、一部の誤りを警告としてエラーログに残す（名は `[SYSTEM]`）。同じ誤りが strict の記録と `[SYSTEM]` の記録の両方に出ることがある。strict の記録だけを読むには、`get_log` の `ghost_name` にゴーストの名前を渡して絞る。

### `ignored` について

`ignored` は、areka がまだ対応していないもので、台本の誤りではない。`ignored` を除いて読みたいときは、種類の名前で外せる（本文が `ignored ` で始まる記録を除く）。

### 切替の後ろの `\s`・`\b`

areka のシェル・バルーンの切替は、台詞の切れ目まで待ってから効く。同じ台本の中で切り替えた後ろの `\s`・`\b` は、再生でも切替が効く前に届けば切替の前のシェル・バルーンで扱われ、記録もそれで確かめる。切替が効いた後に届いたものは、切替の後のシェル・バルーンで確かめる。どちらも、再生が実際に使う定義と同じものに照らす。

### 記録の順

記録は再生が誤りに当たった順（台本の順）に並ぶ。ただし名前の形の `\b[名前]` で箱が見つからない記録（`BalloonBoxMissing`）だけは、文字の層が表示の時に決めるので、ほかの記録との前後が台本の順と違うことがある。どの記録にも位置が付くので、位置で並べ直せる。

### SSP の strict との違い

SSP の欄は [survey.md](survey.md) 8 節の実測（2026-10-29）。

| 項目 | areka | SSP |
|---|---|---|
| 本文の形 | `<kind> <start>-<end> <message>: <text> point="<点>"`（⑶ の種類と文面） | `[GHOST/Script] surface not found (99999) at position 2 : <その位置から台本の終わりまで>` |
| 表示の語・名・位置の数え方 | `Error`・ゴースト名・文字の数 | 同じ |
| 知らないタグ（`\y` など） | `unknown_tag` として記録する | 記録しなかった |
| 答えの `since_id=<n>` | 全種類の通しの最後の番号 | エラーログの最後の番号（どちらで読んでも、その台本の記録の読み落としは無い） |
| 呼んだ先の台本（`\![raise,…]` に返った台本など） | 記録しない | 測っていない |
| 同じ台本の中の切替の後ろの `\s`・`\b` | 切替が効く前に届いたものは、切替の前のシェル・バルーンで確かめる | 測っていない |

### 検出点の表

再生の側で strict の記録を書く所の全部。列「点」は記録の `point` の欄と同じ綴り。この表は `strict_judges_tests.rs` が読み、実装の検出点の一覧と名前の集まり・数が同じこと、各行の赤の台本でその点の記録がちょうど 1 件、緑の台本で 0 件であることを確かめる。

<!-- strict-points:begin -->

| 点 | 種類 | 層と分岐 | 記録する条件 | 赤の台本 | 緑の台本 |
|---|---|---|---|---|---|
| ReadUnknownTag | `unknown_tag` | 台本の解釈: `compile_strict` の命令の頭（読みの印 `UnknownTag`） | 再生がその命令に届いた | `\x` | `\e` |
| ReadUnclosed | `unreadable_argument`（閉じていない） | 同（印 `Unclosed`） | 同 | `\s[0` | `\s[0]` |
| ReadDefaulted | `unreadable_argument`（既定の値へ落とす） | 同（印 `ArgumentDefaulted`） | 同 | `\_w[abc]` | `\_w[500]` |
| ReadMarkerIgnored | `ignored` | 同（印 `MarkerIgnored`） | 同 | `\![*]` | `\q[題,ID]` |
| CommandUnhandled | `unknown_command` | 台本の解釈: `GenericCommand`・`Move` の腕（`\!` の受け取り手の台帳 `consumer_of` が None。単独の `\![*]` を除く） | 同 | `\![nosuch]` | `\![open,readme]` |
| ChoiceTimeoutUnreadable | `unreadable_argument`（既定の値へ落とす） | 台本の解釈: `GenericCommand` の腕（`parse_choice_timeout` が `Unreadable`） | 同 | `\![set,choicetimeout,abc]` | `\![set,choicetimeout,5000]` |
| CharCodeDropped | `unreadable_argument`（出せない文字の番号） | 台本の解釈: `CharCodeDropped` の腕（読み手が `\_u[…]`・`\_m[…]` の番号を捨てた） | 同 | `\_u[0]` | `\_u[0x3000]` |
| FontIgnored | `ignored` | 台本の解釈: `Font` の腕（文字の層の `classify_font_tag` が受け取るだけ） | 同 | `\f[sub,1]` | `\f[bold,1]` |
| FontUnknownKey | `unknown_tag` | 同（知らないキー・キーなし） | 同 | `\f[colour,red]` | `\f[color,red]` |
| AnchorReopened | `unpaired_tag`（新しい開き） | 台本の解釈: `Anchor` の腕（`pair_anchors` が `Reopened`） | 同 | `\_a[x]あ\_a[y]い\_a` | `\_a[x]あ\_a\_a[y]い\_a` |
| AnchorUnclosed | `unpaired_tag`（閉じが無い） | 同（`Unclosed`） | 同 | `\_a[x]` | `\_a[x]あ\_a` |
| AnchorStrayClose | `unpaired_tag`（開いていない所の閉じ） | 台本の解釈: `AnchorEnd` の腕（`StrayClose`） | 同 | `\_a` | `\_a[x]あ\_a` |
| SurfaceUnresolved | `missing_surface` | 表情: `SerikoSink::emit_with` の `Emote`（解決器が `Unresolved`） | 合図が配られた・話し手のスコープにシェルが在る | `\s[-2]` | `\s[-1]` |
| SurfaceAbsent | `missing_surface` | 同（数が今のシェルの ID に無い） | 同 | `\s[99999]` | `\s[0]` |
| BalloonInvalid | `missing_balloon` | 表情: `SerikoSink::emit_with` の `BalloonSurface`（`resolve_balloon_key` が `Invalid`） | 合図が配られた・話し手のスコープにバルーンが在る | `\b[-2]` | `\b[-1]` |
| BalloonAbsent | `missing_balloon` | 同（ID が今のバルーンに無い） | 同 | `\b[99]` | `\b[0]` |
| BalloonBoxMissing | `missing_balloon` | 文字の層: `route_select`（名前の形の `\b[名前]` の箱が今のサーフェスに無い・空回しの写しでない） | UI が合図を捌いた | `\b[無い箱]` | 今のサーフェスに在る箱の名前 |

<!-- strict-points:end -->

誤りを見つけるが、strict の記録は書かない所（同じ誤りを上の点が先に記録するか、台本の誤りでないもの）:

| 分岐 | 書かない理由 |
|---|---|
| seriko 本体 `handle_message` の `Unresolved`・`BalloonResolve::Invalid` の `error!` | 同じ解決器で `SurfaceUnresolved`・`BalloonInvalid` が記録済み |
| 表示の層 `apply_show` の「合成失敗」（`presenter/show.rs`） | 同じ ID の集まりで `SurfaceAbsent`・`BalloonAbsent` が記録済み |
| 文字の層 `apply_font_args` の `warn!` | 同じ分類で `FontIgnored`・`FontUnknownKey` が記録済み |
| `compile` の除外の腕（`Raw` の `debug!`） | 読みの印で `ReadUnknownTag`・`ReadUnclosed` が記録済み |
| seriko の `log_shell_fallback`（切替で今の面が無い） | 台本の誤りでない（切替の戻り先） |
| seriko の `EntityRef`・知らないシェル系の合図 | 到来しない守りの枝 |
| 表示の層の装着前の宛先（`presenter/show.rs`） | 台本の誤りでない |
| `\f` の値の誤り（`BadValue`） | `check_script` と同じく診ない |
