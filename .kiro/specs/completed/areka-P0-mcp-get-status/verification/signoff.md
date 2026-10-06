# 実機確認の記録（タスク 3.2・要件 4.3・4.4・5.6）

- **実施日**: 2026-10-06（記録の時刻は UTC の 2026-10-05 15:42〜15:52。下の呼び出しの時刻は日本時間）
- **判定に使った版**: コミット `750994fc`。配布物の `BUILD-INFO.txt` は `commit=750994f` `dirty=0`
- **ビルド**: `pwsh -NoProfile -File tools/package.ps1`（全段 緑）→ `target\package\areka-0.0.1-x64.zip`（配布用のビルド）
- **展開先**: `target\signoff-status\a\`（ワークツリーの `target\` の下・絶対パス）。記録は `target\signoff-status\logs\`、返った応答は `target\signoff-status\out\`
- **起動のしかた**: `AREKA_*`／`WINTF_*` を全部外し、`AREKA_APP_SMOKE_EXIT_MS=1800000`・`NO_COLOR=1`・`RUST_LOG=info,areka::mcp=debug,areka_mcp=debug,kanade=debug` を付けて `Start-Process -PassThru` で展開先の `areka.exe` を起こした（pid 23984）。止めるときは、その pid にだけ `taskkill`（強制なし）を送った。「きれいに終わったので起動中の印を消しました」と「待受を閉じた addr=127.0.0.1:9801」の記録が出ている
- **呼び出し**: curl（Git for Windows 同梱）で `initialize` の後に `tools/call` の `get_status` を送った（`target\signoff-status\mcp.sh`）。待受は `127.0.0.1:9801`。ゴーストの名前は JSON の `\u` の書き方で渡した（下の「気付いたこと」）
- **ゴースト**: 配布物に入っている既定のゴースト emo2（名前 `えも？？`）だけ。起動して約 10 分の間に、合わせて 400 回呼んだ（記録の「ツールに答えた」が 400 行）

## 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | 何も話していない間は `talking` を含まない答え（空、またはバルーンが見えていれば `balloon(…)` だけ） | **合**。`balloon(0=0/1=0)`・`isError=false`。emo2 は 15〜25 秒ごとに話し、バルーンが消えるまでの時間（30 秒あまり）より間が短いので、バルーンは消えないまま。空の答えは見ていない（下記） |
| ⑵ | 話している間（話し始めて 1 フレーム以上たってから）は `talking` と `balloon(…)` を含む答え | **合**。`talking,balloon(0=0/1=0)`。話し始めのすぐ後は `talking,balloon(0=0)`／`talking,balloon(1=0)` も返った（台本が片方のバルーンを消すため。下記） |
| ⑶ | 起動していない名前を渡すと `NG:Cannot find active ghost from specified name` | **合**。`isError=true`（下記） |
| ⑷ | 呼んでいる間も会話と描画が止まらず、warn 以上の記録が増えない | **合**。呼んでいた間（最初の答えの記録 15:42:16 から最後の答えの記録 15:51:22 まで・UTC）に、会話が 24 回始まり（`talk_id` 2〜25）、25 回「定常運転へ復帰」まで終わった（最初の呼び出しのとき再生中だった起動の挨拶 `talk_id=1` を含む）。同じ間にバルーンの見え方の切り替わりが 52 回記録された。WARN・ERROR は起動の直後の 3 件（どれも本 spec と関係ない既存のもの）から 1 件も増えなかった（下記） |
| 見たまま 1 | 切替の途中に届いたときの答え | **確かめられなかった**（下記） |
| 見たまま 2 | 話し始めの直後のバルーンの遅れ | 約 0.3 秒おきに呼んだ範囲では、遅れは見えなかった（下記） |

## ⑴ 何も話していない間

引数なしで呼んだ。話と話の間の答えは、どれも次のとおりだった。

```
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"balloon(0=0/1=0)"}],"isError":false}}
```

本文に `talking` は無く、`OK:` などの前置きも無い。見えているバルーンは、キャラクター 0・1 ともバルーン 0。

約 0.3 秒おきに 6 分間呼んだ 322 回の内訳:

| 答えの本文 | 回数 |
|---|---|
| `balloon(0=0/1=0)` | 249 |
| `talking,balloon(0=0/1=0)` | 50 |
| `talking,balloon(0=0)` | 18 |
| `talking,balloon(1=0)` | 5 |

空の本文は 1 回も返らなかった。emo2 は秒ごとのイベント（`OnSecondChange`）から 15〜25 秒おきに話し、バルーンが消えるまでの時間（記録の `deadline` で 30〜38 秒）が来る前に次の話が始まるため、バルーンが消えた場面が無かった。要件 5.6 ⑴ の「バルーンが見えていれば `balloon(…)` だけ」に当たる。

## ⑵ 話している間

起動の挨拶の再生中（呼び出し 00:42:16）と、その後の秒ごとのイベントの会話のたびに、次の答えが返った。

```
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"talking,balloon(0=0/1=0)"}],"isError":false}}
```

会話の始まりと終わり（kanade の記録）と答えの移り変わりが合っている。例（記録は UTC、呼び出しは日本時間で 9 時間の差）:

```
15:45:44.106  kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=10
15:45:44.175  [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="clear" visible=false
15:45:45.346  [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="content" visible=true
15:45:47.746  kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
```

```
00:45:43.395  balloon(0=0/1=0)
00:45:44.637  talking,balloon(0=0)
00:45:45.933  talking,balloon(0=0/1=0)
00:45:48.299  balloon(0=0/1=0)
```

話し始めのすぐ後の `talking,balloon(0=0)`（または `talking,balloon(1=0)`）は、台本が話し始めに片方のキャラクターのバルーンを消し（記録の `trigger="clear" visible=false`）、そのキャラクターが話し出すと出し直す（`trigger="content" visible=true`）ためで、答えは画面のバルーンの見え方のとおりだった。

`ghost_name` に emo2 の名前（`えも？？`）を渡しても、同じ答えが返った（12 回。`talking,balloon(1=0)`・`talking,balloon(0=0/1=0)`・`balloon(0=0/1=0)`）。ゴーストのフォルダのフルパスを渡したときも `balloon(0=0/1=0)` が返った。`ghost_name` に空の文字列を渡したときは、省略と同じく `balloon(0=0/1=0)` が返った。

## ⑶ 起動していない名前

```
ghost_name="存在しないゴースト"  → {"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}
ghost_name="emo2"                → {"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}
ghost_name="Emily/Phase4.5"      → {"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}
ghost_name="C:\home\nothing\ghost" → {"type":"text","text":"NG:Cannot find active ghost from specified name"}],"isError":true}
```

（`emo2` はフォルダの名前で、ゴーストの名前ではないので当たらない。名前の決まりは `mcp-tool-entrances` のまま。）

記録には `debug!` の 1 行ずつだけが出た（warn 以上は出ていない）:

```
DEBUG ... areka_mcp::tools::bridge: MCP: ツールに答えた tool="get_status" ghost="" is_error=true text="NG:Cannot find active ghost from specified name"
```

## ⑷ 会話と描画が止まらないこと・warn 以上の記録

- 呼んでいた間は、標準出力の記録の 110 行目（最初の「ツールに答えた」・15:42:16 UTC）から 4961 行目（最後の「ツールに答えた」・15:51:22 UTC）まで。この範囲で、会話の再生を起こした記録（`再生起動`）が 24 件（`talk_id` 2〜25）、「talk 完了——定常運転へ復帰」が 25 件だった。終わりの 25 件には、範囲の前（55 行目）に始まり最初の呼び出しのとき再生中だった起動の挨拶（`talk_id=1`・216 行目で完了）を含む。バルーンの見え方の切り替わり（表示の側の記録 `バルーンの可視状態が遷移`）は、この範囲で 52 件（走行の全体では 62 件）。表情の切り替え（`apply(ShowSurface)`）も呼び出しの間、毎分記録され続けた。
- 走行の全体では、`再生起動` が 28 件（起動の挨拶 `talk_id=1`・秒ごとのイベントの会話 `talk_id` 2〜27・終了の挨拶 `talk_id=28`）、「定常運転へ復帰」が 26 件（`talk_id` 1〜26）。`talk_id` 26・27 は最後の呼び出しの後に始まった。`talk_id=27` は、止める指示（`taskkill`）を受けて「talk 完了——保留 close を消化し握手開始」（`steady_talk_done_close`）で終わり、続けて終了の挨拶（`talk_id=28`）に移った。
- 1 回の呼び出しの curl の所要時間（322 回）: 最小 12 ms・中央 20 ms・90% 点 29 ms・最大 556 ms。最初の 60 回の中に 1 回だけ 1.58 秒があった。どちらも答えは正しく、上限（10 秒）の答えにはなっていない。
- WARN・ERROR の行は、標準出力・標準エラー出力を合わせて 3 件で、どれも呼び出しを始める前（起動の直後の 15:42:03）のもの。呼び出しの間とその後は 0 件。

```
WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" ...（2 件）
WARN actor{actor=emo-text}: areka_emo_text::actor::attach: 折返し基準が描画範囲の外に解決された ... balloon="emo2-kakukaku" ...（1 件）
```

## 見たまま 1: 切替の途中に届いたときの答え

**確かめられなかった。** 配布物にはゴーストが emo2 の 1 体しか無く、切り替えるには 2 体目を置いて右クリックのメニューから選ぶ操作が要る。画面の操作（computer-use）の許可を求めたが、許されなかった。MCP から切り替えを起こす口（`sakurascript`・`raise_event`）はまだ無い。したがって、切替の途中に届いた呼び出しの答え（要件 3.2 の「降りた」の `NG:`）と、その場面がどれくらいの長さ続くか（research.md 7 節）は、実機では見ていない。答えの文言は決定論テスト（`get_status_tests.rs`）で固定されている。

## 見たまま 2: 話し始めの直後のバルーンの遅れ

約 0.3 秒おきに呼んだ範囲では、遅れは見えなかった。話し始めの記録から最も早く届いた呼び出しは約 100 ms 後（`talk_id=11`: 15:46:08.093 に再生を起こし、15:46:08.152 に台本がキャラクター 0 のバルーンを消し、15:46:08.253 の答えが `talking,balloon(1=0)`）で、そのときには消えたバルーンがもう答えから外れていた。1 フレーム（約 16 ms）の遅れは、この呼び出しの間隔では捉えられない。`talking` が出ているのに `balloon(…)` が 1 つも無い答えは、400 回の中に 1 回も無かった。

## 気付いたこと（本 spec の範囲の外）

- Git Bash から curl の引数に日本語をそのまま書くと、areka は `fail to deserialize request body invalid unicode code point` で受け取らなかった（3 回）。Windows の引数の文字コードの変換で JSON が壊れたもので、areka の側の問題ではない。`\u3048` の形で渡すと正しく受け取った。
- `tools/package.ps1` を Git Bash から `pwsh` で起こすと、`cargo metadata` の出力（パッケージの説明の日本語）を読み違えて「版を読めない」で止まった（終了コード 3）。PowerShell から起こすと全段 緑だった。
