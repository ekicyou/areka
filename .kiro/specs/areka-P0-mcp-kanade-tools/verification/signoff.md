# 実機確認の記録（タスク 6.3・要件 7.6・7.7）

- **実施日**: 2026-10-19（記録の時刻は UTC の 2026-10-18 18:00〜18:06）
- **判定に使った版**: コミット `d113f35d`（タスク 6.2 の後・作業ツリーは未変更）。`cargo build -p areka -j 4`（debug）の `target\debug\areka.exe`
- **helper**: `pwsh -NoProfile -File tools/sync-i686-helper.ps1` → `helper is i686: target\debug\shiori-host32-helper.exe`
- **ゴースト**: `cargo run -p sample-ghost-kit --bin nar-sample-path emo2` が展開した `target\nar-samples\manual\emo2` を `target\sk\r\` へ写し、その写しを使った（同梱の検体そのものと `target\` の外は書き替えていない）。起動の引数は絶対パスで、ゴースト `…\target\sk\r\ghost\emo2`・バルーン `…\target\sk\r\balloon\emo2-kakukaku`（実走のテスト `mcp_get_log_real_run.rs` と同じ渡し方）。ゴーストの名前は `えも？？`
- **置き場**: 根・検体・プロファイル・記録はすべてワークツリーの `target\sk\` の下（`r\`＝検体の写し・`profile\`＝`AREKA_PROFILE_DIR`・`logs\`＝`run1.log`／`run1.stderr.log`・`out\`＝要求と応答の JSON と復号した PNG・`mcp.ps1`＝呼び出しの道具）
- **起動のしかた**: PowerShell から `AREKA_*`／`WINTF_*` を全部外したうえで、次を付けて `Start-Process -PassThru` で起こした（pid 37604・18:00:49 UTC）。終わらせ方は有界の自動終了（900 秒）を掛けたうえで、確かめ終えた 18:06:24 に自分で起こした pid 37604 にだけ `taskkill`（強制なし）を送った。終了の挨拶（`Ghost:OnClose`）を再生してから `unload_clean`・`ghost shutdown sequence completed`・「MCP: 待受を閉じた addr=127.0.0.1:9871」まで出て終わった

  ```
  AREKA_MCP_PORT=9871
  AREKA_APP_SMOKE_EXIT_MS=900000
  AREKA_NO_ALERT=1
  NO_COLOR=1
  AREKA_PROFILE_DIR=<ワークツリー>\target\sk\profile
  AREKA_PERF_THREAD_REPORT_SEC=10
  RUST_LOG=info,areka::log::script=info,kanade=debug,areka::mcp=debug,areka_mcp=debug,areka::perf=debug
  ```

- **待受**: 起動前に 9800〜9899 の待受は 0 件（ほかのセッションの areka・SSP と当たらない）。areka は `http://127.0.0.1:9871/api/mcp/v1` で待ち受けた
- **呼び出し**: PowerShell の `Invoke-WebRequest` で JSON-RPC の `tools/call` を 1 本ずつ送った（`target\sk\mcp.ps1`・本文は UTF-8 のバイト列）。このセッションの `ssp` の MCP ツールは SSP 宛てなので使っていない。画面の操作（クリック・SendInput）は 1 回も使っていない
- **「台詞が表示される」の見方**: 画面を撮る代わりに、⑴ script 種別のログの 1 件（`areka::log::script`）、⑵ kanade の「新しいトークとして始める」と「talk 完了」の記録、⑶ `get_status` の `talking`、⑷ 直後の `dump_balloon` が返したバルーンの絵（復号して開き、字を目で読んだ）の 4 つで判定した

## 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | `sakurascript` で台本を送ると `OK` が返り、台詞が表示される | **合**（id 3。答えは `OK`・`isError=false`。バルーンの絵に送った字が出ていた。下記） |
| ⑵ | 長く待つ台本の再生中に 2 本目を送ると、1 本目が中断されて 2 本目が表示される | **合**（id 10→13。1 本目の `\_w[30000]` の後ろの字は出ず、2 本目の字がバルーンに出た。下記） |
| ⑶ | `sakurascript` で表情を変えた後に `dump_surface` を 1 回呼ぶと、変わった姿が返る | **合**（id 24→25→26。目を閉じて手を組んだ姿から、ジト目で手を下ろした姿へ変わった絵が返った。下記）。`mcp-dump-images` の要件 7.7 ⑵ の本来の手順 |
| ⑷ | `raise_event` で台本を返すイベントを起こすと、台本の文字列を含む答えが返り、台詞が表示される | **合**（id 30・`OnBalloonChange`。答えの 2 行目に `Script: <台本>`。両方のバルーンに台本の字が出た。下記） |
| ⑸ | ゴーストが知らない名前で `OK:the ghost returned no script.` が返る | **合**（id 34・`OnArekaSignoffUnknownEvent`。直前のトークはそのまま続いた。下記） |
| ⑹ | `get_log`（`log_type=script`）に、MCP の台本（`[SSTP(Local,Auth)]`）・`raise_event` の台本（`[Ghost:<イベント名>]`）・ゴーストの自発の会話の台本が読める | **合**（id 40。3 つとも読めた。下記）。ただし `[Ghost]`（SHIORI の往復から生まれていないトーク）の語は、実機では出していない（下の「確かめられなかったこと」） |
| ⑺ | 呼んでいる間も描画が止まらず、warn 以上の記録が増えない | **合**（呼んでいた 2 分 13 秒の間に、トークが 12 回始まり、表示の更新が 174 回、バルーンの見え方の切り替わりが 18 回記録された。WARN・ERROR は起動の直後の 3 件から 1 件も増えなかった。下記） |

## ⑴ 台本を送る

先に `get_status`（id 1）で `balloon(0=0/1=0)`（話していない）を確かめてから送った。

```
[18:01:53.781 UTC] id=3 sakurascript 33ms isError=False text=OK
  送った台本: \0\s[1000]MCP から送った台本、ひとつめや。\e
[18:01:55.459 UTC] id=5 dump_balloon → OK:balloon of scope 0 as last drawn (...)  b64len=16244 sha16=71DC7EDED17EEC86
```

記録（抜き出し）:

```
18:01:53.761525Z INFO kanade: 外からの台本を新しいトークとして始める event="sstp_send_started" talk_id=3 strict=None
18:01:53.764061Z INFO kanade: OnTranslate は 204——展開済みの元の台詞で進む event="translate_reply" source=sstp-send kind="no_content"
18:01:53.765381Z INFO areka::log::script: \0\s[1000]MCP から送った台本、ひとつめや。\e ghost=えも？？ label=SSTP(Local,Auth)
18:01:53.777598Z DEBUG areka_mcp::tools::bridge: MCP: ツールに答えた tool="sakurascript" ghost="えも？？" is_error=false
18:01:54.696117Z INFO kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
```

`dump_balloon`（id 5）が返した絵（`out\img5.png`・400×224）には「MCP から送った台本、ひとつめや。」と出ていた。翻訳は `source=sstp-send` で通っている（要件 4.4）。

## ⑵ 再生中の 2 本目で中断する

```
[18:02:09.467 UTC] id=10 sakurascript 75ms isError=False text=OK
  送った台本: \0\s[1000]長く待つ台本、いちばんめ。\_w[30000]この続きは出てはいけない。\e
[18:02:12.532 UTC] id=11 get_status → talking,balloon(0=0)
[18:02:12.642 UTC] id=12 dump_balloon → 絵に「長く待つ台本、いちばんめ。」
[18:02:12.739 UTC] id=13 sakurascript 15ms isError=False text=OK
  送った台本: \0\s[1000]割り込んだ、にばんめの台本や。\e
[18:02:14.422 UTC] id=14 dump_balloon → 絵に「割り込んだ、にばんめの台本や。」
[18:02:44.499 UTC] id=15 get_status → balloon(0=0/1=0)
[18:02:44.639 UTC] id=16 dump_balloon → 絵はゴーストの自発の会話の字（「この続きは出てはいけない。」は無い）
```

記録（kanade の行だけ）:

```
18:02:09.419076Z INFO kanade: 外からの台本を新しいトークとして始める event="sstp_send_started" talk_id=5
18:02:12.727633Z INFO kanade: 外からの台本を新しいトークとして始める event="sstp_send_started" talk_id=6
18:02:13.498856Z INFO kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
18:02:22.612531Z INFO kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=7 origin="OnSecondChange"
```

- 1 本目（`talk_id=5`）は再生中（id 11 の `talking`）に 2 本目（`talk_id=6`）で置き換わり、「talk 完了」は 2 本目の 1 回だけだった。
- 1 本目の待ちが終わるはずの 18:02:39 より前の 18:02:22 に、ゴーストの自発の会話（`talk_id=7`）が始まった＝1 本目はもう再生されていない。id 16 の絵にも 1 本目の後ろの字は出ていない。

## ⑶ 表情を変えてから撮る

emo2 のむらさき（スコープ 0）は、surface1000 のまま `\![bind,…]` の着せ替えで表情を変える作りなので、その形の台本で変えた。

```
[18:03:32.586 UTC] id=24 dump_surface → OK:scope 0, surface 1000 as currently shown (...)  b64len=162972 sha16=189FB6158126CD52
[18:03:32.697 UTC] id=25 sakurascript 23ms isError=False text=OK
  送った台本: \0\s[1000]\![bind,腕,伸び,1]\![bind,紅,差し,0]\![bind,口,口開,1]\![bind,眉,シュン,1]\![bind,目,ジトー,1]\![bind,まばたき,ジトー,1]ジト目に変えたで。\_w[5000]\e
[18:03:33.699 UTC] id=26 dump_surface → OK:scope 0, surface 1000 as currently shown (...)  b64len=178968 sha16=99B1CA41DB17FA57
```

- id 24 の絵（`out\img24.png`・382×547）は目を閉じて胸の前で手を組んだ姿、id 26 の絵（`out\img26.png`・382×547）はジト目・口を開き・手を下ろした姿で、送った着せ替えのとおりに変わっていた。id 25 から id 26 の間にほかのトークは始まっていない（kanade の記録）。
- 先に試した `\0\s[2102]…`（id 21→22）でも、答えが `surface 2102 as currently shown` に変わり、絵も変わった。ただし 2102 は emo2 ではエモ（スコープ 1）の「驚き」の絵で、むらさきの表情ではなかったので、判定は上の着せ替えの手順で行った。

## ⑷ 台本を返すイベント

emo2 の辞書 `dic\update.pasta` の `＊OnBalloonChange` は、引数なしで両方のキャラクターの台詞を返す。

```
[18:03:50.256 UTC] id=30 raise_event 130ms isError=False text=OK:the ghost returned a script and it is being played.
Script: \p[0]\s[1000]\![bind,腕,組み,1]\![bind,紅,差し,0]\![bind,口,にこっ,1]\![bind,眉,通常,1]\![bind,目,通常,1]\![bind,まばたき,通常,1]バルーン切り替えです。\_w[950]\w9\w9\p[1]\s[通常]ばるるるるん？\_w[450]\_w[600]\e
[18:03:52.822 UTC] id=31 get_status → talking,balloon(0=0/1=0)
[18:03:52.940 UTC] id=32 dump_balloon → 絵に「バルーン切り替えです。」
[18:03:53.125 UTC] id=33 dump_balloon {"scope":1} → 絵に「ばるるるるん？」
```

答えは 1 行目の文言と `Script: <台本>` の 2 行（要件 2.3）。script 種別のログにも同じ台本が `label=Ghost:OnBalloonChange` で 1 件出た。

## ⑸ ゴーストが知らない名前

```
[18:03:53.182 UTC] id=34 raise_event 17ms isError=False text=OK:the ghost returned no script.
  引数: {"event":"OnArekaSignoffUnknownEvent","references":["r0","r1"]}
[18:03:53.215 UTC] id=35 get_status → talking,balloon(0=0/1=0)
```

⑷ のトークの再生中に送った。答えの直後も `talking` のままで、今のトークは止まらなかった（要件 2.4）。

## ⑹ script 種別のログ

`get_log` `{"log_type":"script"}`（id 40・`isError=false`）の答えの抜き出し（台本は長いので頭だけ）:

```
#14 2026/10/19 03:00 [Ghost:OnFirstBoot] えも？？ : \p[0]\s[1000]…はじめましてや！！…
#15 2026/10/19 03:01 [Ghost:OnSecondChange] えも？？ : \p[0]\s[1000]…今日はほんまについてへん…
#16 2026/10/19 03:01 [SSTP(Local,Auth)] えも？？ : \0\s[1000]MCP から送った台本、ひとつめや。\e
#17 2026/10/19 03:02 [Ghost:OnSecondChange] えも？？ : …傘持ってきたのに晴れとるやん…
#18 2026/10/19 03:02 [SSTP(Local,Auth)] えも？？ : \0\s[1000]長く待つ台本、いちばんめ。\_w[30000]この続きは出てはいけない。\e
#19 2026/10/19 03:02 [SSTP(Local,Auth)] えも？？ : \0\s[1000]割り込んだ、にばんめの台本や。\e
…
#25 2026/10/19 03:03 [SSTP(Local,Auth)] えも？？ : \0\s[1000]\![bind,腕,伸び,1]…ジト目に変えたで。\_w[5000]\e
#26 2026/10/19 03:03 [Ghost:OnBalloonChange] えも？？ : \p[0]\s[1000]…バルーン切り替えです。…
#27 2026/10/19 03:03 [Ghost:OnSecondChange] えも？？ : …トリックオアトリート！…
```

- MCP の台本は `[SSTP(Local,Auth)]`、`raise_event` で返った台本は `[Ghost:OnBalloonChange]`、ゴーストの自発の会話（毎秒のイベントへの答え）は `[Ghost:OnSecondChange]`、起動の挨拶は `[Ghost:OnFirstBoot]` で読めた。名前の欄は `get_active_ghost_list` と同じ `えも？？`。
- 中断された 1 本目（#18）も、再生を始めたトークなので 1 件残っている（要件 4.1）。
- 走行の全体で、トークを始めた記録（「新しいトークとして始める」と「再生起動」）が 21 件、script 種別のログが 21 件で、トーク 1 つにつき 1 件だった。語の内訳は `SSTP(Local,Auth)` 5・`Ghost:OnSecondChange` 13・`Ghost:OnFirstBoot` 1・`Ghost:OnBalloonChange` 1・`Ghost:OnClose` 1。

## ⑺ 描画が止まらないこと・warn 以上の記録

- 呼んでいた間は、最初の答え（18:01:53.25・id 1）から最後の答え（18:04:06.39・id 40）まで。この範囲で、トークの始まりが 12 件、「talk 完了——定常運転へ復帰」が 10 件、バルーンの見え方の切り替わり（`バルーンの可視状態が遷移`）が 18 件、表示の更新（`apply(ShowSurface)`）が 174 件、SERIKO のループの発火が 42 件記録された。
- 表示のスレッド（`main`・role=ui）の CPU 時間は、10 秒ごとの報告で呼ぶ前も呼んでいる間も 10 秒あたり 3.7〜5.2 秒ずつ増え続けた（debug ビルド）。止まった区間は無い。
- WARN・ERROR の行は、標準出力・標準エラー出力を合わせて走行の全体で 3 件で、どれも最初の呼び出しの前（起動の直後の 18:00:52〜54）の、本 spec と関係の無い既存のもの。呼んでいる間とその後は 0 件（要件 6.5 の例外に当たる記録も出ていない）。

  ```
  WARN areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" ...（2 件）
  WARN areka_emo_text::actor::attach: 折返し基準が描画範囲の外に解決された ... balloon="emo2-kakukaku" ...（1 件）
  ```

- 1 回の呼び出しの所要時間（PowerShell で測った往復）: `sakurascript` 15〜75 ms（5 回）、`raise_event` 17 ms と 130 ms、`get_status` 15〜288 ms、`dump_surface` 141〜430 ms、`dump_balloon` 90〜149 ms（debug ビルド）。

## 確かめられなかったこと

- **表示の語 `[Ghost]`（SHIORI の往復から生まれていないトーク）**: 実機では出していない。この語が付くのは選択肢の `script:` の台本などで、出すには選択肢を選ぶ操作（クリック）が要り、MCP からは起こせない。本走行では画面の操作をしない決まりなので見送った。語の決め方は決定論テスト（`crates/areka-kanade/src/schedule/script_log_tests.rs` ほか・タスク 3.5）で固定されている。要件 7.6 ⑹ が挙げる 3 つ（MCP の台本・`raise_event` の台本・自発の会話）は上のとおり実機で読めた。

## 気付いたこと（本 spec の範囲の外）

- **トークごとのスレッド `sakura-talk-<番号>` が、トークが終わっても残り続ける。** 10 秒ごとの報告で、プロセスのスレッド数が起動直後の 15 から終了前の 35 まで増え、増えた分はすべて `sakura-talk-1`〜`sakura-talk-<番号>` だった（トーク 1 つにつき 1 本）。MCP を呼ぶ前に終わった起動の挨拶の `sakura-talk-1` も残っているので、MCP の呼び出しとは関係なく、`crates/areka-sakura/src/drive.rs` の `spawn_talk` が起こすスレッドの後始末の問題と見られる。本 spec の境界（kanade の SSTP の入口と MCP の 2 ツール）の外なので、ここでは直さず、起票へ回す。

## 要件 7.7（検体の README）

⑴ が通ったので、`crates/areka-seriko/tests/fixtures/trigger-intervals/README.md` の「実機の確かめ」の 3 から、「areka の MCP の `sakurascript` はまだ中身が無いので、写しの `scripts\pasta\shiori\event\boot.lua` の `OnBoot`／`OnFirstBoot` が生の台本 1 本を返すようにして流す」という読み替えの 1 文を外し、「areka の MCP の `sakurascript` で台本として送って流す」本来の手順へ戻した。
