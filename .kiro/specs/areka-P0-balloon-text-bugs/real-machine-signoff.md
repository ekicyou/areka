# 実機の確かめの記録（areka-P0-balloon-text-bugs・タスク 5.3・件 1）

設計「テストの方針 › 実機」の 1（件 1）を、本物の areka と emo2（同梱バルーン emo2-kakukaku・実 pasta.dll）で走らせ、記録で確かめた。操作（ポインタの移動・押下）は人の手を使わず、走らせる道具が `SendInput` で行った。

**結論**: ⑴ 選んで 204・⑵ 選択肢の時間切れで `OnChoiceTimeout` 204・箱の選択肢・⑷ `\w` の待ちの前後の押下は、すべて成立。⑶ 中断（利用者のダブルクリック）は、完了 `balloon-break` の決まり（要件 4.1）どおりバルーンがその場で隠れ、`OnBalloonBreak`（204）が届き、`OnBalloonTimeout` は来ない——要件 1.4 の前提「バルーンが表示されたまま残った」が実機の中断の道では起きない（5 章）。

## 1. 走らせ方

| 項目 | 値 |
|---|---|
| 日時 | 2026-10-16 19:10〜19:25 JST（ログの時刻は UTC の 10:10〜10:25） |
| 版 | HEAD `c74da005`（製品のコードの未コミットの変更 0 件）。`cargo build -p areka --bin areka`（debug）と `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`（PE の機種 `014C` を確かめた） |
| 根 | ワークツリーの `target\btb-signoff\root`（普通のバルーン）と `target\btb-signoff\root-box`（箱）。どちらも `nar-sample-path emo2` が展開した `target\nar-samples\manual\emo2` の写しに `areka.exe` と 32bit の補助プロセスを置いたもの。写しの辞書から `dispatch.pasta`・`hour.pasta` を外してゴースト自身のトークを黙らせた |
| 台本 | 写しの `ghost\master\scripts\pasta\shiori\event\boot.lua` だけを差し替えた（生のさくらスクリプト）。1 本目 `\0\s[0]選んでね（1）\n\q[はい（いち）を選ぶ,OnBtbYes]\n\q[いいえ（に）を選ぶ,OnBtbNo]\e`。時間切れで `OnBalloonTimeout` が来たら 2 本目（「選んでね（2）」）を 1 回だけ返す。`OnBtbYes`・`OnBtbNo`・`OnChoiceTimeout`・`OnBalloonBreak` などは何も返さない（204）。Timeout は頭に `\![set,choicetimeout,8000]`、Wait は選択肢の後ろに `\nまだ待ってね` と `\w9`×20（約 9 秒） |
| 箱 | `root-box` の `shell\master\surfaces.txt` の末尾に、balloon.*ブレス `balloon.btbbox`（`size,340,110`・`font.height,20`）と、surface.append*ブレス `surface.append0` の element定義 `element1,balloon,btbbox,40,50`（surface0.png の吹き出しの絵の不透明な所）を足した |
| 環境変数 | `NO_COLOR=1`・`RUST_LOG=info,kanade=trace,areka::input_events=debug`（判定の分岐の記録まで開ける: 押下の棄却 `choice_click_rejected`・強調 `choice_hover_inject`／`box_choice_hover_inject`・箱の押下の結論 `box_press` は debug、SHIORI の送出 `shiori_request` は trace）・`AREKA_BALLOON_TIMEOUT_MS=5000`（既定 30 秒を 5 秒に短縮）・`AREKA_APP_SMOKE_EXIT_MS=75000`（有界の自動終了）・`AREKA_NO_ALERT=1`・`AREKA_MCP_PORT=39873`（開発者の SSP の 9801〜9830 を避ける）・`AREKA_ROOT`／`AREKA_PROFILE_DIR`＝根・`TMP`／`TEMP`＝根の `tmp`。起動の前に記憶（profile）を消した |
| 起動 | `<根>\areka.exe "<根>\ghost\emo2"`（絶対パス） |
| 道具 | 本フォルダの `real-machine-run.ps1`。`-Prepare` で根を作り、`-Run <筋書き>` で起こして記録を読みながら操作する。選択肢の行の位置は、ポインタを縦に 3 画素ずつ動かして強調の記録が `ordinal=Some(0)`／`Some(1)` に変わる y の範囲で測り、その真ん中を押す。押した後はポインタを元の位置へ戻す（バルーンの上に残すと時間切れが抑止される）。入力デスクトップが `Default` でないときは操作せず止まる（どの回も `Default`） |
| 画面 | プライマリ DPI 192（k=2.0）。普通のバルーンの窓 `1744,72,2544,520`（物理画素）、シェルの窓 `2012,330,2880,1704` |

コマンド（どのディレクトリからでも動く）:

```powershell
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\areka-P0-balloon-text-bugs\real-machine-run.ps1 -Prepare
pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\areka-P0-balloon-text-bugs\real-machine-run.ps1 -Run Chose    # Timeout・Break・Box・Wait も同じ
```

走行の一覧（`target\btb-signoff\runs.txt`）。生の記録は `target\btb-signoff\run-<筋書き>.log`、操作の控えは `run-<筋書き>.steps.txt`。

| 回 | プロセス番号 | 終わり方 | 判定に使う |
|---|---|---|---|
| Chose | 19888 | 自動終了・終了コード 0 | ✅ `run-Chose.log` |
| Timeout（1 回目） | 20712 | 自動終了・終了コード 0。2 本目の測りの直前に窓が画面の外へ動いた（4 章）ため操作が止まった | ✗（`run-Timeout-1.log` に改名して残した） |
| Break（1 回目） | 26132 | 自動終了・終了コード 0。操作の道具は途中で止まった（`runs.txt` の `ok=False`）。当時の道具は中断の後に `OnBalloonTimeout` を 60 秒待つ作りで、中断でバルーンがその場で隠れて `OnBalloonTimeout` が来なかったため、待ちが切れて止まった（走行中に読んだ記録では `balloon_break_accepted`→`trigger="user_break" visible=false`→`OnBalloonBreak`）。この回のログと操作の控えは 2 回目で上書きしたので、残ったファイルからは確かめられない。道具を「15 秒待って 0 件を数える」に直して 2 回目を回した | ✗（ファイルなし） |
| Timeout | 23352 | 自動終了・終了コード 0 | ✅ `run-Timeout.log` |
| Break | 20444 | 自動終了・終了コード 0 | ✅ `run-Break.log` |
| Box | 31068 | 自動終了・終了コード 0 | ✅ `run-Box.log` |
| Wait（1 回目） | 25912 | 自動終了・終了コード 0。明けた後の押下がどの記録も残さなかった（4 章） | ✗（`run-Wait-1.log`） |
| Wait | 25612 | 自動終了・終了コード 0。明けた後は先にポインタを行へ運んでから押した | ✅ `run-Wait.log` |

自分で起こしたプロセス以外は止めていない。走行の後に上の番号の areka と、根の補助プロセスが残っていないことを確かめた。

## 2. 判定の一覧

| # | 確かめる項目（要件） | 結果 | 根拠 |
|---|---|---|---|
| 1 | 選んで応答が 204 → 待ちが解けた時刻から既定の長さで隠れ `OnBalloonTimeout`（1.1・1.2） | **成立**（解けてから 5.04 秒） | 3.1 |
| 2 | 選択肢の時間切れで `OnChoiceTimeout` が 204 → 同じく隠れ `OnBalloonTimeout`（1.1・1.3・1.9） | **成立**（解けてから 5.00 秒） | 3.2 |
| 3 | 中断（利用者のダブルクリック）（1.4） | **前提が起きない**: その場で隠れ（`trigger="user_break"`）`OnBalloonBreak` 204・`OnBalloonTimeout` 0 件（15 秒待った）。完了 `balloon-break` 要件 4.1 のとおり | 3.3・5 章 |
| 4 | 済んだ行にポインタを乗せても光らない（1.10） | **成立**（選んだ・時間切れ・箱の 3 回とも、済んだ後の `ordinal=Some(..)` が 0 件） | 3.1・3.2・3.4 |
| 5 | 済んだ行を押しても選択の知らせも警告も出ない・二度押しも（1.11） | **成立**（済んだ後の `choice_selected` 0 件・WARN 0 件・押下は `choice_click_rejected reason="no_hit"`／箱は `selected_now=false`） | 3.1・3.2・3.4 |
| 6 | 箱の選択肢でも 1〜5（1.13） | **成立**（解けてから 5.05 秒・済んだ後の強調 0 件・選択 0 件・WARN 0 件） | 3.4 |
| 7 | 選択肢の後ろに字と `\w` の待ち: 明ける前の押下は今のまま捨てられ、明けた後の押し直しで選べる | **成立**（前: `choice_rejected_no_wait`・後: `choice_accepted`→`choice_resolved`→5.05 秒で隠れて `OnBalloonTimeout`） | 3.5 |
| 8 | 判定の分岐の記録まで開けた記録で上が確かめられる（4.4） | **成立**（棄却・強調・押下の結論の debug と SHIORI 送出の trace が出ている） | 1 章の環境変数 |

`OnBalloonTimeout` の後の 2 本目は、1 本目と同じ形の選択肢つきのトークとして始まった（`steady_talk origin="OnBalloonTimeout"`・要件 1.7 の「新しい台詞は今までの決まり」）。

## 3. 根拠（逐語の抜き書き）

時刻は UTC。行頭の `actor{actor=…}:` と長い Reference は省いた。行番号は各ログの行。

### 3.1 選んで 204（`run-Chose.log`）

1 本目（起点の確かめ・済んだ後は触らない）:

```
78   10:10:12.272037 INFO kanade: event="choice_waiting_established" talk_id=1 choice_count=2 deadline_ms=Some(86406709) display_end_ms=86376709
84   10:10:13.800412 DEBUG areka::input_events::balloon::moved: event="choice_hover_inject" scope=0 ordinal=Some(0)     ← 済む前は光る（行の測り）
97   10:10:16.982032 INFO areka::input_events::balloon::pressed: event="choice_selected" scope=0 id=OnBtbYes label=はい（いち）を選ぶ
98   10:10:16.982361 INFO kanade: event="choice_accepted" choice_id=OnBtbYes plan=Named talk_id=1
100  10:10:16.982977 TRACE kanade: event="shiori_request" method=GET id=OnBtbYes references=[] status=Some("talking,choosing,balloon(0=0)")
101  10:10:16.983673 INFO kanade: event="choice_resolved" talk_id=1 choice_id=OnBtbYes outcome="no_content"      ← 204
107  10:10:17.026967 INFO [balloon-visibility] タイムアウト計測を開始（選択肢つきのトーク・起点＝選択の待ちが解けて止まった時刻） origin="choice_wait_ended_at" stopped_at=0.3510479 display_end=0.35 deadline=5.3510479
113  10:10:22.026009 INFO [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
115  10:10:22.028980 INFO kanade: event="balloon_event_sent" id="OnBalloonTimeout" cause="timeout" talk_id=1
116  10:10:22.029331 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout references=[…選んでね（1）…, "0"] status=Some("balloon(0=0)")
```

`deadline − stopped_at` ＝ 5.0 秒（`AREKA_BALLOON_TIMEOUT_MS=5000`）。`choice_resolved`（16.98）から隠れる（22.03）まで 5.04 秒。待っている間（78〜101 行）は隠れていない。

2 本目（済んだ行の確かめ）: ダブルクリックで選び（1 打目で選び、2 打目は二度押し）、0.5 秒後に「はい」「いいえ」の行へポインタを乗せ、「いいえ」「はい」の順に 1 回ずつ押した。

```
148  10:10:27.000594 INFO  …pressed: event="choice_selected" scope=0 id=OnBtbYes
152  10:10:27.002161 INFO  kanade: event="choice_resolved" talk_id=2 choice_id=OnBtbYes outcome="no_content"
155  10:10:27.010618 INFO  [balloon-visibility] タイムアウト計測を開始（…） origin="choice_wait_ended_at" stopped_at=4.8812518 display_end=0.35 deadline=9.8812518
160  10:10:27.067059 DEBUG …pressed: event="choice_click_rejected" scope=0 reason="no_hit"          ← 二度押しの 2 打目
161  10:10:27.067349 DEBUG …user_break: event="balloon_break_ignored" scope=0 reason="selection"     ← 中断にもしない
169  10:10:28.467058 DEBUG …pressed: event="choice_click_rejected" scope=0 reason="no_hit"          ← 済んだ「いいえ」
176  10:10:28.875404 DEBUG …pressed: event="choice_click_rejected" scope=0 reason="no_hit"          ← 済んだ「はい」
177  10:10:29.292788 INFO  [balloon-visibility] 抑止が解けたのでタイムアウト計測をやり直した talk_time=7.1707655 deadline=12.1707655
183  10:10:34.300949 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
186  10:10:34.301761 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout …
```

153〜186 行の数え（`sed -n 153,186p run-Chose.log | grep -c …`）: `choice_hover_inject … ordinal=Some` **0**・`event="choice_selected"` **0**・` WARN ` **0**・`choice_click_rejected` **3**。ポインタを乗せている間は時間切れが止まり、離れてから測り直した（要件 1.6 の今までの抑止）。

### 3.2 選択肢の時間切れで `OnChoiceTimeout` 204（`run-Timeout.log`）

```
78   10:18:02.132184 INFO  kanade: event="choice_waiting_established" talk_id=1 choice_count=2 deadline_ms=Some(86854600) display_end_ms=86846600
95   10:18:10.524522 INFO  kanade: event="choice_timeout_fired" talk_id=1 deadline_ms=86854600 now_ms=86855000
96   10:18:10.526814 TRACE kanade: event="shiori_request" method=GET id=OnChoiceTimeout … status=Some("talking,choosing,balloon(0=0)")
97   10:18:10.527492 INFO  kanade: event="choice_timeout_cancelled" talk_id=1 outcome="no_content"      ← 204
100  10:18:10.528849 INFO  [balloon-visibility] タイムアウト計測を開始（…） origin="choice_wait_ended_at" stopped_at=8.7462753 display_end=0.35 deadline=13.7462753
107  10:18:15.529084 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
110  10:18:15.529873 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout … status=Some("balloon(0=0)")
```

選択肢の時間切れ（95 行）そのものでは `OnBalloonTimeout` を送らず（要件 1.9）、待ちが解けて 5.00 秒で隠れた。待っている 8 秒の間（78〜97 行）は隠れていない（要件 1.1）。

2 本目: 時間切れ（141〜143 行・`outcome="no_content"`）の後、2 行に乗せて 1 回ずつ押した。

```
146  10:18:24.537649 INFO  [balloon-visibility] タイムアウト計測を開始（…） origin="choice_wait_ended_at" stopped_at=8.9072821 deadline=13.9072821
151  10:18:25.901673 DEBUG …pressed: event="choice_click_rejected" scope=0 reason="no_hit"
159  10:18:26.320578 DEBUG …pressed: event="choice_click_rejected" scope=0 reason="no_hit"
167  10:18:31.745682 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
170  10:18:31.746420 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout …
```

144〜170 行: 強調 `ordinal=Some` **0**・`choice_selected` **0**・WARN **0**・棄却 **2**（トークの終わりから済む＝要件 1.10・1.11）。

### 3.3 中断（`run-Break.log`）

1 行目（選択肢でない字）を左ダブルクリックした。

```
80   10:19:19.585956 INFO  kanade: event="choice_waiting_established" talk_id=1 choice_count=2
95   10:19:24.295317 DEBUG …pressed: event="choice_click_rejected" scope=0 reason="no_hit"      ← 1 打目（選択肢の外）
102  10:19:24.354923 DEBUG …pressed: event="choice_click_rejected" scope=0 reason="no_hit"      ← 2 打目
103  10:19:24.355278 INFO  kanade: event="balloon_break_accepted" scope=0 talk_id=1 phase="Steady"
105  10:19:24.355929 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="user_break" visible=false
109  10:19:24.356913 TRACE kanade: event="shiori_request" method=GET id=OnBalloonBreak … status=Some("balloon(0=0)")
```

中断の受け入れと同じフレームで隠れ（`trigger="user_break"`）、その後 15 秒（既定の長さ 5 秒の 3 倍）待って `id=OnBalloonTimeout` は **0** 件（`run-Break.steps.txt` の `OnBalloonTimeout after break: 0`）。WARN は自明の 4 行（4 章）だけ。隠れた後なので、済んだ行へポインタを乗せる・押す確かめはできない。

### 3.4 箱の選択肢（`run-Box.log`）

shell の窓の箱 `btbbox` に「選んでね（1）」と選択肢 2 行が出た。

```
80   10:20:38.839348 INFO  kanade: event="choice_waiting_established" talk_id=1 choice_count=2
100  10:20:39.872040 DEBUG …shell_box_handler: event="box_choice_hover_inject" scope=0 box="btbbox" ordinal=Some(0)   ← 済む前は光る
146  10:20:42.884403 INFO  …shell_box_handler: event="choice_selected" scope=0 id=OnBtbYes … box="btbbox"
147  10:20:42.884573 DEBUG …shell_box_handler: event="box_press" scope=0 box="btbbox" x=140.0 y=163.0 double_click=None selected_now=true verdict=ConsumedBySelection
151  10:20:42.886374 INFO  kanade: event="choice_resolved" talk_id=1 choice_id=OnBtbYes outcome="no_content"
157  10:20:42.939564 INFO  [balloon-visibility] タイムアウト計測を開始（…） origin="choice_wait_ended_at" stopped_at=0.3505598 display_end=0.35 deadline=5.3505598
163  10:20:47.938190 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
166  10:20:47.941311 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout …
```

解けてから 5.05 秒で隠れた（箱だけに文字が出ている scope・要件 1.13）。2 本目（ダブルクリックで選び、2 行に乗せて 1 回ずつ押す）:

```
244  10:20:52.254934 INFO  …shell_box_handler: event="choice_selected" scope=0 id=OnBtbYes … box="btbbox"
249  10:20:52.256521 INFO  kanade: event="choice_resolved" talk_id=2 choice_id=OnBtbYes outcome="no_content"
254  10:20:52.263964 INFO  [balloon-visibility] タイムアウト計測を開始（…） origin="choice_wait_ended_at" stopped_at=4.2738135 deadline=9.2738135
259  10:20:52.313072 DEBUG …shell_box_handler: event="box_press" … y=163.0 double_click=Left selected_now=false verdict=ConsumedBySelection   ← 二度押し
271  10:20:53.696379 DEBUG …shell_box_handler: event="box_press" … y=208.0 double_click=None selected_now=false verdict=ShellOp           ← 済んだ「いいえ」
277  10:20:54.113075 DEBUG …shell_box_handler: event="box_press" … y=163.0 double_click=None selected_now=false verdict=ShellOp           ← 済んだ「はい」
287  10:20:59.539491 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
290  10:20:59.540315 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout …
```

250〜290 行: `box_choice_hover_inject … ordinal=Some` **0**・`choice_selected` **0**・WARN **0**・`selected_now=false` **3**。済んだ行の押下は選択にならず、シェルへの操作の道（`ShellOp`）へ回った。

### 3.5 選択肢の後ろに字と `\w` の待ち（`run-Wait.log`）

```
96   10:23:46.273666 INFO  …pressed: event="choice_selected" scope=0 id=OnBtbYes                                   ← 待ちの最中に押す
97   10:23:46.273991 WARN  kanade: event="choice_rejected_no_wait" reason="no_choice_wait" choice_id=OnBtbYes scope=0   ← 今のまま捨てられる
110  10:23:49.775641 INFO  kanade: event="choice_waiting_established" talk_id=1 choice_count=2 deadline_ms=Some(87224212) display_end_ms=87194212
111  10:23:49.872894 DEBUG …moved: event="choice_hover_inject" scope=0 ordinal=Some(0)                              ← 捨てられた押下では済んでいない（光る）
117  10:23:50.289384 INFO  …pressed: event="choice_selected" scope=0 id=OnBtbYes                                   ← 明けた後に押し直す
118  10:23:50.289880 INFO  kanade: event="choice_accepted" choice_id=OnBtbYes plan=Named talk_id=1
121  10:23:50.291737 INFO  kanade: event="choice_resolved" talk_id=1 choice_id=OnBtbYes outcome="no_content"
125  10:23:50.339807 INFO  [balloon-visibility] タイムアウト計測を開始（…） origin="choice_wait_ended_at" stopped_at=9.6505008 display_end=9.65 deadline=14.6505008
131  10:23:55.339497 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
134  10:23:55.342477 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout …
```

このログの WARN の 5 行は、自明の 4 行（4 章）と 97 行の `choice_rejected_no_wait`（今のままの棄却の記録）だけ。

## 4. 自明の記録と、判定に使わなかった回

- どの回も WARN が 4 行ある: `bake: element が全透明（α=0）`（`purple/a/null.png`・2 行・検体の絵）、`折返し基準が描画範囲の外に解決された`（emo2-kakukaku の `wordwrappoint.x,-34`・1 行）、`force_quit reason="user"`（`AREKA_APP_SMOKE_EXIT_MS` の自動終了・1 行）。ERROR はどの回も 0 行。
- Timeout（1 回目・`run-Timeout-1.log`）: 1 本目の選択待ちの最中（10:11:49・操作の道具は何も送っていない時間）に、`[balloon-limit] Clamp runtime 関門` と `[visibility-guard] OffscreenPull／NearestFallback／ClampX` が 565 行続いた（バルーンの窓の書込位置 `x=-46`→`-323` を作業領域へ引き戻す、が繰り返された）。直前に `loop ticker catch-up: skipped multiple boundaries`（PC が詰まった印）。2 本目の測りでは、むらさきの窓の 1 枚が最小化の位置（`-32768`）にあり、道具が測る窓を取り違えて止まった。道具を「最小化の位置の窓を除く」に直して 2 回目を回し、再現しなかった。件 1 の判定には関わらない。
- Wait（1 回目・`run-Wait-1.log`）: 待ちの最中の押下（99〜100 行・`choice_rejected_no_wait`）までは 2 回目と同じ。明けた後、ポインタを画面の別の場所から行の上へ動かすのと同時に押した（移動と押下を 1 回の `SendInput` で送った）ところ、areka の記録に移動も押下も 1 行も残らなかった。選択待ちは 30 秒の既定で時間切れ（173〜175 行）、その後 5 秒で隠れて `OnBalloonTimeout`（178〜186 行）。2 回目は先にポインタを行へ運び 0.4 秒置いてから押して通った。

## 5. 中断（要件 1.4）について

要件 1.4 は「選択の待ちが中断され（利用者の割り込み・台本の中断）、バルーンが表示されたまま残った」ときの決まりである。実機の利用者の中断（バルーンの左ダブルクリック）は、完了 `balloon-break` 要件 4.1 のとおり**出ている全部のバルーンをその場で隠し**（`decide_user_break`）、`OnBalloonBreak` を送り、次のトークまで時間切れの計測をしない。kanade で選択の待ちを畳む道（`Action::CancelChoice`）は、利用者の中断（`schedule/user_break.rs`）と選択肢の時間切れの 204（`schedule/steady.rs`）の 2 つだけで、時間切れは ⑵ で確かめた。したがって「中断の後にバルーンが表示されたまま残る」場面は今の areka の実機では起きず、⑶ は「中断ではその場で隠れ、`OnBalloonTimeout` を送らない（今までの決まりのまま）」を確かめた形になる。中断の後の起点（止まった時刻）は、決定論のテスト（`balloon_visibility_choice_wait_tests.rs` の中断の行）が固定している。
