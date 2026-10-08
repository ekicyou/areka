# 実機確認の記録（タスク 6.2・要件 5.5）

- **実施日**: 2026-10-08（記録の時刻は UTC の 2026-10-07 17:55:40〜18:01:07。下の呼び出しの時刻は、断りが無ければ記録と同じ UTC）
- **判定に使った版**: コミット `f2f3f432`。配布物の `BUILD-INFO.txt` は `version=0.0.2` `commit=f2f3f43` `dirty=0`
- **ビルド**: PowerShell から `pwsh -NoProfile -File tools/package.ps1`（全段 緑・「all steps green」）→ `target\package\areka-0.0.2-x64.zip`（配布用のビルド）
- **展開先**: `target\signoff-check\a\`（ワークツリーの `target\` の下・絶対パス）。記録は `target\signoff-check\logs\`（`stdout.log`・`stderr.log`・`package.log`・`pid.txt`）、送った本文は `target\signoff-check\in\`、返った応答は `target\signoff-check\out\`
- **起動のしかた**: `AREKA_*`／`WINTF_*` を全部外し、`AREKA_APP_SMOKE_EXIT_MS=1800000`・`NO_COLOR=1`・`RUST_LOG=info,areka::mcp=debug,areka_mcp=debug,kanade=debug` を付けて `Start-Process -PassThru` で展開先の `areka.exe` を起こした（pid 1204）。処理の `debug!`「[mcp] 台本を確かめた」の記録の出どころは `areka::mcp::check_script` で、`areka::mcp=debug` に含まれる。止めるときは、その pid が展開先の `areka.exe` であることを確かめてから、その pid にだけ `taskkill`（強制なし）を送った。「きれいに終わったので起動中の印を消しました」と「待受を閉じた addr=127.0.0.1:9801」の記録が出ている（下の「止め方」）
- **呼び出し**: curl（Git for Windows 同梱）で `initialize` → `notifications/initialized` → 本文 1 件を送った（`target\signoff-check\mcp.sh`）。待受は記録の「MCP: 待受を始めた url=http://127.0.0.1:9801/api/mcp/v1」から読んだ `127.0.0.1:9801`。台本の日本語は JSON の `\u` の書き方で渡した（本文は `target\signoff-check\gen.py` で作った）
- **ゴースト**: 配布物に入っている既定のゴースト emo2（名前 `えも？？`）だけ。`check_script` は合わせて 131 回、`get_status` は 73 回呼んだ（記録の「ツールに答えた」が 204 行・`is_error=true` は 0 行）

## 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | `tools/list` に 11 本が出て、11 本目が `check_script` | **合**。先頭 10 本は今までどおりの並び（下記） |
| ⑵ | `\x`・`\s[99999]`・`\f[sub,1]` を含む台本で 3 件の診断が返り、ゴーストは喋らず表情も変わらない | **合**。`OK:3 diagnostics`・`isError=false`。呼んだ前後の約 10 秒、会話の再生・表情の切り替え（`bind 適用`・面の替わる `ShowSurface`）・バルーンの見え方の切り替わりは 1 件も無く、`get_status` は `talking` を含まない答えのまま（下記） |
| ⑶ | 誤りの無い台本で `OK:0 diagnostics` | **合**。`isError=false`（下記） |
| ⑷ | 普通の台本と 1 MiB 前後の長い台本の両方で、呼んでいる間も描画と会話が止まらない・処理の `debug!` の時間 | **合**。約 3 分の間に両方を 64 回ずつ呼び、その間に会話が 9 回始まり 8 回終わり、バルーンの見え方の切り替わりが 20 回あった。WARN・ERROR は起動の直後の 3 件から増えなかった。UI スレッドの時間は両方とも中央 90 µs 前後、別スレッドの時間は普通の台本で中央 48 µs・長い台本で中央 53 ms（下記） |
| ⑸ | help のページに「areka 独自のツール」の節 | **合**。節の見出しと `check_script` と日本語の 1 行が出た（下記） |

## ⑴ `tools/list`

`out/list.body`（`http=200`）の `result.tools` の名前の並び:

```
1 get_active_ghost_list
2 get_status
3 get_expression_table
4 get_property
5 get_log
6 sakurascript
7 raise_event
8 reload
9 dump_surface
10 dump_balloon
11 check_script
```

11 本目の定義（抜粋）:

```
{"name": "check_script", "title": "Check SakuraScript Without Playing", "description": "areka's own tool (SSP does not have it). Checks SakuraScript without playing it: the ghost does not speak, move or change. ...", "inputSchema": {"properties": {"ghost_name": {...}, "script": {"description": "SakuraScript to check", "type": "string"}}, "required": ["script"], "type": "object"}}
```

`initialize` の答え（`out/list.init.body`）の `instructions` にも、`check_script is an areka-only tool that SSP does not have: it checks SakuraScript without playing it (the ghost does nothing), so use it before sakurascript.` の 1 文と、今までの 2 つの案内（`call get_active_ghost_list first`・`NG:not implemented yet`）が出ていた。

## ⑵ 誤りを含む台本

ゴーストが話していない間に呼ぶため、会話が終わった記録（`steady_talk_done`）を待って 1 秒後に、`get_status` → `check_script`（誤りを含む台本）→ `check_script`（誤りの無い台本・⑶）→ `get_status` を約 1 秒おきに 8 回、の順に呼んだ（`target\signoff-check\quiet.sh`・結果は `out/quiet.txt`）。

渡した台本（`in/err.json`）: `\x\s[99999]\f[sub,1]この文は喋らない\e`

答え（`out/err.body`・本文を開いたもの）:

```
OK:3 diagnostics
{"end":2,"kind":"unknown_tag","message":"areka does not know this tag; playback drops it","start":0,"text":"\\x"}
{"end":11,"kind":"missing_surface","message":"no such surface in the current shell; playback does not change the surface","start":2,"text":"\\s[99999]"}
{"end":20,"kind":"ignored","message":"areka accepts this but it has no effect yet","start":11,"text":"\\f[sub,1]"}
```

`isError` は `false`。`doc/ssp-mcp/areka-tools.md` ⑵ の例と 1 字も違わない。

記録（UTC）:

```
17:56:52.051  kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
17:56:54.029  MCP: ツールに答えた tool="get_status" ghost="えも？？" is_error=false
17:56:54.663  areka::mcp::check_script: [mcp] 台本を確かめた tool="check_script" diagnostics=3 script_bytes=46 ui_us=177 worker_us=70
17:56:54.663  MCP: ツールに答えた tool="check_script" ghost="えも？？" is_error=false
17:56:55.197  areka::mcp::check_script: [mcp] 台本を確かめた tool="check_script" diagnostics=0 script_bytes=47 ui_us=115 worker_us=55
17:56:55.679  MCP: ツールに答えた tool="get_status" ...（17:57:02.730 まで 8 回）
17:57:02.147  kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=4 origin="OnSecondChange"
```

ゴーストが喋らず、表情も変わらなかったことの裏付け:

- **会話**: 会話が終わった 17:56:52.051 から次の会話が始まる 17:57:02.147 までの約 10 秒、kanade の記録は 1 行も無い。次の会話（`talk_id=4`）は `origin="OnSecondChange"` で、emo2 がふだん 15〜25 秒おきに秒ごとのイベントから話すもの。呼び出し（17:56:54.663）の 7.5 秒後で、台本の文（「この文は喋らない」）を話したものではない。呼び出しの後に `get_status` を約 1 秒おきに 8 回呼び、7 回は `balloon(0=0/1=0)`（`talking` 無し）、8 回目（17:57:02.730・`talk_id=4` の始まりの後）だけが `talking,balloon(0=0)` だった。
- **表情**: 同じ約 10 秒の間に、`bind 適用`（表情の部品の切り替え）は 0 件。表示の更新（`apply(ShowSurface)`）は 18 件あり、どれも SERIKO のくり返しのアニメーション（まばたき等。scope 0 の `animation_id=1400` と scope 1 の `id=0` の 2 つのループが 3 回ずつ回った＝`seriko: loop 抽選発火` 6 回・`loop 末尾残留` 3 回・`loop 停止` 3 回）によるもので、面は scope 0 が `surface_id=1000`、scope 1 が `surface_id=2100` のまま替わっていない。`\s[99999]` に当たる面の記録も無い。
- **バルーン**: 同じ間に `バルーンの可視状態が遷移` は 0 件。
- **SHIORI**: areka の記録は SHIORI への要求を 1 件ずつは残さない（秒ごとのイベントの要求も、台本の返らないものは記録に出ない）ので、要求の数を実機の記録で数えることはできない。記録に出た kanade の行は、呼び出しの前後とも秒ごとのイベント（`origin="OnSecondChange"`）から起きたものだけだった。「答えの前後で SHIORI の呼出の記録が増えない」ことは、決定論テスト（タスク 5.4・`crates/areka/src/mcp/check_script_tests.rs`）で固定している。

## ⑶ 誤りの無い台本

渡した台本（`in/ok.json`）: `\0\s[0]こんにちは。\w9\1\s[10]やあ。\e`（emo2 のシェルには `surface0`・`surface10` が在る）

```
{"jsonrpc":"2.0","id":11,"result":{"content":[{"type":"text","text":"OK:0 diagnostics"}],"isError":false}}
```

記録は `[mcp] 台本を確かめた tool="check_script" diagnostics=0 script_bytes=47 ui_us=115 worker_us=55`（17:56:55.197）。

## ⑷ 呼んでいる間も描画と会話が止まらないこと・処理の時間

渡した台本:

- **普通の台本**（`in/normal.json`・154 バイト）: `\0\s[0]今日はいい天気だね。\w9\1\s[10]そうだね、散歩でも行こうか。\w9\0\s[0]\x うん、行こう。\_w[500]\n\f[sub,1]小さく\e`。診断は `\x` と `\f[sub,1]` の 2 件
- **長い台本**（`in/big.json`・1,048,600 バイト・604,668 文字）: `\0\s[0]長い台本の一節です。\w5\1\s[10]こちらも長い台本の一節です。\n\x\s[99999]\f[sub,1]ここまで。\_w[100]\n`（137 バイト）を 7,654 回くり返し、末尾に `\e`。診断は 1 回につき 3 件で、合わせて 22,962 件

約 3 分（17:57:25.7〜18:00:27.9）の間、普通の台本 → `get_status` → 長い台本 → 1 秒休む、をくり返した（`target\signoff-check\load.sh`・64 周・192 回の呼び出し）。どの呼び出しも `http=200`。記録の「[mcp] 台本を確かめた」は、普通の台本の 64 回がどれも `diagnostics=2`、長い台本の 64 回がどれも `diagnostics=22962` だった。

処理の `debug!` の時間（64 回ずつ・µs）:

各 64 件を小さい順に並べ、中央は 33 番目、90% 点は 57 番目の値（どちらも実際に出た値）。

| 台本 | UI スレッド（`ui_us`） | 別スレッド（`worker_us`） |
|---|---|---|
| 普通（154 バイト） | 最小 66・中央 90・90% 点 110・最大 162 | 最小 34・中央 48・90% 点 62・最大 114 |
| 長い（1 MiB） | 最小 76・中央 95・90% 点 131・最大 185 | 最小 40,545・中央 53,113・90% 点 67,519・最大 81,910 |

UI スレッドの時間は台本の長さに依らず 0.2 ms 未満だった（設計どおり、UI スレッドでするのは事実の写し取りだけ）。長い台本の別スレッドの時間も最大 82 ms で、橋の上限（10 秒）よりずっと短い。curl で測った 1 回の所要時間は、普通の台本が中央 9 ms・最大 79 ms、長い台本が中央 155 ms・最大 244 ms（答えの本文は 3,558,440 バイト）。この 3 分の前に 1 度だけ長い台本を呼んだときは `ui_us=88 worker_us=41918`（17:57:16.472）。

呼んでいた間の会話と描画（記録の範囲 17:57:25.7〜18:00:27.9）:

- 会話の再生を起こした記録（`再生起動`）が 9 件（`talk_id` 6〜14・どれも `origin="OnSecondChange"`）、「talk 完了——定常運転へ復帰」が 8 件（`talk_id` 6〜13。14 は範囲の終わりの 18:00:26.152 に始まった）。
- バルーンの見え方の切り替わり（`バルーンの可視状態が遷移`）が 20 件、表示の更新（`apply(ShowSurface)`）が 192 件、表情の部品の切り替え（`bind 適用`）が 41 件。
- WARN・ERROR の行は、標準出力・標準エラー出力を合わせて走行の全体で 3 件で、どれも呼び出しを始める前（起動の直後の 17:55:41）の既存のもの（下記）。呼び出しの間とその後は 0 件。

会話の途中に長い台本を呼んだ例（`talk_id=9` の間・UTC）:

```
17:58:48.147  kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=9 origin="OnSecondChange"
17:58:48.211  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="clear" visible=false
17:58:49.106  [mcp] 台本を確かめた tool="check_script" diagnostics=2 script_bytes=154 ui_us=106 worker_us=44
17:58:50.539  [mcp] 台本を確かめた tool="check_script" diagnostics=22962 script_bytes=1048600 ui_us=98 worker_us=73630
17:58:50.704  seriko: bind 適用 scope=0 category=腕 part=伸び id=1100 on=true（ほか 3 件）
17:58:51.206  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true
17:58:52.222  [mcp] 台本を確かめた tool="check_script" diagnostics=2 script_bytes=154 ui_us=80 worker_us=49
17:58:53.342  [mcp] 台本を確かめた tool="check_script" diagnostics=22962 script_bytes=1048600 ui_us=86 worker_us=53022
17:58:54.931  [mcp] 台本を確かめた tool="check_script" diagnostics=2 script_bytes=154 ui_us=86 worker_us=45
17:58:55.845  seriko: bind 適用 scope=0 category=腕 part=組み id=1101 on=true（ほか 3 件）
17:58:56.060  [mcp] 台本を確かめた tool="check_script" diagnostics=22962 script_bytes=1048600 ui_us=131 worker_us=53607
17:58:56.848  kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
```

起動の直後の WARN（本 spec と関係ない既存のもの）:

```
WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" ...（2 件）
WARN actor{actor=emo-text}: areka_emo_text::actor::attach: 折返し基準が描画範囲の外に解決された ... balloon="emo2-kakukaku" ...（1 件）
```

## ⑸ help のページ

同じ待受の `GET http://127.0.0.1:9801/api/mcp/help`（`http=200`・`content-type: text/html; charset=utf-8`・`out/help.html`）の末尾:

```
<h2>areka 独自のツール</h2>
<p>SSP と同じツールに加えて、SSP に無い次のツールを出しています。</p>
<ul>
<li><code>check_script</code> — 台本を再生せずに確かめ、areka で効かないタグ・コマンド、今のシェルとバルーンに無い ID を位置つきで返す（ゴーストには何もさせない）</li>
</ul>
```

## 止め方

18:01 ごろ、pid 1204 が展開先の `areka.exe`（`target\signoff-check\a\areka.exe`）であることを確かめてから `taskkill /PID 1204`（強制なし）を送った。終了の挨拶（`close_talk_start talk_id=17`・18:01:02.655）の後、次の記録で終わった。

```
18:01:07.201712  shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)） event="unload_clean"
18:01:07.209857  [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4
18:01:07.225828  ghost-shutdown: ghost shutdown sequence completed
18:01:07.230077  [boot_resolve] きれいに終わったので起動中の印を消しました event="session_mark_cleared"
18:01:07.236508  MCP: 待受を閉じた addr=127.0.0.1:9801
```

止めた後、pid 1204 のプロセスは残っていない。ほかのプロセスには触れていない。

## 気付いたこと

- `tools/list` と `initialize` には、橋の「MCP: ツールに答えた」の記録は出ない（出るのは `tools/call` だけ）。要求ごとの「MCP: 要求に応えた method=…」は `debug!` で出る。
- この走行の待受は、要求ごとにセッションを作って閉じる形で、`initialize` の答えに `Mcp-Session-Id` は付かなかった（記録の `rmcp::service: Service initialized` → `serve finished quit_reason=Closed` が 1 本の要求ごとに出る）。呼び出しには差し支えない。
- 画面は撮っていない。⑵ の「喋らない・表情が変わらない」は、上の記録（会話・`bind 適用`・面の ID・バルーンの見え方）と `get_status` の答えで判定した。
- 約 3 分の呼び出しの手順（`load.sh`）では、答えの 1 行目を書き出す部分が Git Bash の `/c/...` の形のパスを Windows の Python に渡したため読めず、`out/load.txt` に答えの 1 行目が残らなかった（呼び出しそのものは 192 回とも `http=200`）。答えの中身は、サーバー側の記録の「[mcp] 台本を確かめた … diagnostics=…」と「ツールに答えた … is_error=false」（192 回ぶん・`is_error=true` は 0 回）で確かめた。長い台本の答えの 1 行目が `OK:22962 diagnostics` であることは、その前の 1 回（`out/big1.body`）で見ている。
