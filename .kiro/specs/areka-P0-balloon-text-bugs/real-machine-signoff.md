# 実機の確かめの記録（areka-P0-balloon-text-bugs・タスク 5.3 件 1・5.4 件 2・5.5 件 3）

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


## 6. 件 2（タスク 5.4）: 文字の描画範囲が潰れたバルーン

設計「テストの方針 › 実機」の 2 を、同じ道具・同じ版の areka（HEAD `7e996b33`・製品のコードは 5.3 と同じ）で走らせた。**結論: 要件 2.3・2.4・2.5 はすべて成立**（潰れに入ったときに「潰れている」が 1 行、隠す・出すでは増えない、面を戻すと「戻った」が 1 行出て、その後に文字が描かれる）。ERROR は 0 行。

### 6.1 走らせ方

| 項目 | 値 |
|---|---|
| 日時 | 2026-10-16 19:32〜19:33 JST（ログは UTC の 10:32〜10:33） |
| 根 | `target\btb-signoff\root`（`-Prepare` で作り直した）。同梱バルーンの写し `balloon\emo2-kakukaku` に、8×8 の白一色の面 `balloons20.png` を足した。brief の実機の再現（10-06・8×8 の面に emo2-kakukaku の余白 36〜56 px を当てる）と同じ形 |
| 台本 | 4 本を時間切れごとに順に返す（`OnBalloonTimeout` で次を返す）: ⑴ `\0\s[0]\b[20]ちいさい面（1）\e` ⑵ `\0\s[0]\b[20]ちいさい面（2）\e`（同じ潰れた面のまま、隠れた後にもう一度出す） ⑶ `\0\s[0]\b[0]もどった面（3）です\e`（普通の面へ戻す） ⑷ `\0\s[0]\b[20]ちいさい面（4）\e`（一度描いた後にまた潰す） |
| 環境変数 | 5.3 と同じ（`RUST_LOG=info,kanade=trace,areka::input_events=debug`。潰れの `warn!`・戻りの `info!`・再追従の `info!`・装着の `info!` はどれも info 以上で、`areka_emo_text` の info が出る水準）・`AREKA_BALLOON_TIMEOUT_MS=5000`・`AREKA_MCP_PORT=39873` |
| 操作 | なし（`SendInput` は使っていない）。見えている間に areka 自身の MCP の `dump_balloon`（scope 0）で今の絵を撮った（`dump-c1.png`・`dump-c2.png`・`dump-r3.png`・`dump-c4.png`） |
| コマンド | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\areka-P0-balloon-text-bugs\real-machine-run.ps1 -Prepare` の後に、同じファイルを `-Run Collapse` で実行 |
| 走行 | pid 10444・自動終了・終了コード 0（`runs.txt`）。記録は `target\btb-signoff\run-Collapse.log`、操作の控えは `run-Collapse.steps.txt` |

### 6.2 判定

| # | 確かめる項目（要件） | 結果 | 根拠（`run-Collapse.log` の行） |
|---|---|---|---|
| 1 | 潰れに入ったとき、どのバルーンのどの値で潰れたかが読める警告が 1 回だけ出る（2.3） | **成立**（⑴ で 1 行。名前・4 辺・画像の大きさが載っている） | 80 |
| 2 | 潰れたままの間はエラーを 1 行も出さず、毎フレームの作り直しもしない（2.2） | **成立**（ERROR 0 行。⑴〜⑵ の約 11 秒の間、文字の層の記録は 80・81 行の 2 行だけ） | 80〜132 |
| 3 | 潰れたまま隠す・出すで記録が増えない（2.5） | **成立**（⑴ が隠れ（94）、⑵ で出し直しても（105）「潰れている」は増えない） | 94・105 |
| 4 | 面を切り替えて戻すと「戻った」が 1 回出て、文字が描かれる（2.4） | **成立**（⑶ で「戻った」1 行（133）→ 再追従（134）→ 装着（135・文は変わっていない＝2.8）。`dump-r3.png`（400×224）に「もどった面（3）です」が描かれている） | 133〜135 |
| 5 | 一度描いた後にまた潰すと、前の文字が残らない（2.9） | **成立**（⑷ で「潰れている」がもう 1 回（新しい潰れの入り・160）。`dump-c4.png` は 8×8 の全部の画素が `FFFFFFFF`（白一色）で、⑶ の文字は残っていない） | 160・161 |
| 6 | 実機の記録で上が確かめられる（4.4） | **成立** | — |

「潰れている」の警告（`文字の描画範囲が潰れている（幅か高さが 0 以下）`）は 2 行（80・160）で、どちらも潰れの入り。「戻った」は 1 行（133）。`grep -c '潰れている'` が 4 になるのは、再追従の記録の文（81・161）にも「描画範囲が潰れているので」の語が入っているため。

### 6.3 根拠（逐語の抜き書き）

時刻は UTC。行頭の `actor{actor=emo-text}:` と長い Reference は省いた。

```
79   10:32:06.988731 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true
80   10:32:06.989506 WARN  areka_emo_text::actor::attach: 文字の描画範囲が潰れている（幅か高さが 0 以下）——このバルーンの文字は描かない balloon="emo2-kakukaku" left=36.0 top=46.0 right=-36.0 bottom=-48.0 image_size=(8.0, 8.0)
81   10:32:06.990325 INFO  areka_emo_text::actor::attach: 文字層の再追従: binding と文字描画領域を再構築した——描画範囲が潰れているので描画資源は残し、同じフレームの提示が外す（戻るまで再生成しない・リビール状態は保存） actor=0 k_old=2.0 k_new=2.0 image_size=(8, 8) surface_size=(16, 16)
94   10:32:12.343059 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
97   10:32:12.346029 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout … status=Some("balloon(0=20)")
105  10:32:12.393551 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true      ← ⑵ 同じ潰れた面で出し直す（記録は増えない）
120  10:32:17.792990 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
123  10:32:17.795969 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout … status=Some("balloon(0=20)")
132  10:32:17.844105 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true
133  10:32:17.844460 INFO  areka_emo_text::actor::attach: 文字の描画範囲が戻った——次のフレームから文字を描く balloon="emo2-kakukaku" width=320.0 height=122.0
134  10:32:17.844714 INFO  areka_emo_text::actor::attach: 文字層の再追従: binding と文字描画領域を再構築し描画資源を破棄した（次フレームで新しい物理寸へ再生成・リビール状態は保存） actor=0 k_old=2.0 k_new=2.0 image_size=(400, 224) surface_size=(800, 448)
135  10:32:17.850910 INFO  areka_emo_text::actor::present: テキスト供給面を予約スロットへ装着した（ActorRender 不在時のみ・以降は Present のみ） actor=0 place=Balloon slot=32v0 physical_size=(640, 244) wrap=BudouxWordWrap
147  10:32:23.343033 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
150  10:32:23.345925 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout … status=Some("balloon(0=0)")
160  10:32:23.394181 WARN  areka_emo_text::actor::attach: 文字の描画範囲が潰れている（幅か高さが 0 以下）——このバルーンの文字は描かない balloon="emo2-kakukaku" left=36.0 top=46.0 right=-36.0 bottom=-48.0 image_size=(8.0, 8.0)
161  10:32:23.394438 INFO  areka_emo_text::actor::attach: 文字層の再追従: …描画範囲が潰れているので描画資源は残し、同じフレームの提示が外す… image_size=(8, 8) surface_size=(16, 16)
175  10:32:28.793478 INFO  [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
178  10:32:28.796881 TRACE kanade: event="shiori_request" method=GET id=OnBalloonTimeout … status=Some("balloon(0=20)")
```

潰れた値（`left=36 top=46 right=-36 bottom=-48`・`image_size=(8,8)`）は、brief の 10-06 の実機の記録の値と同じ。直す前に毎フレーム出ていた `create_composition_swap_chain`（0x887A0001）と `present_frame が失敗` の ERROR は 0 行で、`解決後の validrect が退化している` の毎フレームの WARN も 0 行。

`dump_balloon` の読み戻し（`run-Collapse.steps.txt`）:

| 撮った時 | 絵 | 大きさ | 中身 |
|---|---|---|---|
| ⑴ 潰れた面（`dump-c1.png`） | `\b[20]` | 8×8 | 全部の画素が `FFFFFFFF`（文字なし） |
| ⑵ 出し直し（`dump-c2.png`） | `\b[20]` | 8×8 | 全部の画素が `FFFFFFFF` |
| ⑶ 戻した面（`dump-r3.png`） | `\b[0]` | 400×224 | 「もどった面（3）です」が文字の領域の 1 行目に描かれている（目で確かめた） |
| ⑷ また潰した面（`dump-c4.png`） | `\b[20]` | 8×8 | 全部の画素が `FFFFFFFF`（⑶ の文字は残っていない） |

### 6.4 そのほかの記録

- WARN は 9 行: 件 2 の「潰れている」2 行（80・160）、5.3 と同じ自明の 4 行（`null.png` の 11・29 行、`折返し基準` の 59 行、`force_quit` の 241 行）、それに `balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=20 prefix=balloons file=balloons20.png` の 3 行（15・20・34 行）。最後の 3 行は、足した `balloons20.png` に scope 1 用の `balloonk20.png` が無いための、検体が原因の正しい縮退の知らせで、件 2 の判定には関わらない。`解決後の validrect が退化している`・`create_composition_swap_chain`・`present_frame が失敗` は 0 行。
- 箱（要件 2.7）・ほかのバルーンを妨げないこと（要件 2.6）は、タスク 5.4 の範囲外（決定論と実 GPU のテスト `actor_region_collapse_tests.rs` が固定する）。

## 7. 件 3（タスク 5.5）: 最下行の下線の撮り直し

設計「テストの方針 › 実機」の 3 を、同じ道具・同じ版の areka（HEAD `bc298852`・製品のコードは 5.3 と同じ）で走らせた。**結論: 最下行の作者の下線とアンカーの下線は、どちらも面の中に見えている（要件 3.1）。縦書きのバルーンでも下線が出ている（要件 3.4 の見た目の確かめ）。ただし、最下行の `g`・`y` の字の下の部分は面の下端で欠けている**（7.4・範囲外。起票は親が行う）。

### 7.1 走らせ方

| 項目 | 値 |
|---|---|
| 日時 | 2026-10-16 19:37〜19:40 JST（ログは UTC の 10:37〜10:40） |
| 根 | 横書き: `target\btb-signoff\root`。縦書き: `target\btb-signoff\root-tate`（同梱バルーンの写しの基層 `descript.txt` の末尾に `vertical,1` と `wordwrappoint.y,-56` を足した。理由は 7.3） |
| 形 | 同梱バルーン emo2-kakukaku の面 0（400×224・validrect 36/46/-44/-56＝文字の領域 320×122）・Yu Gothic UI 28 画素・拡大率 2.0（プライマリ DPI 192）＝brief の実機の形と同じ |
| 台本（横書き） | `\0\s[0]\b[0]一行目 gyぁ\n二行目 gyぁ\n三行目 \f[underline,1]下線gyぁ\f[underline,0]\n\f[underline,1]下線gyぁ\f[underline,0] \_a[OnBtbAnchor]アンカgyぁ\_a\e`（4 行目＝最下行に作者の下線とアンカー。全部の行に `g`・`y`・`ぁ`） |
| 台本（縦書き） | `\0\s[0]\b[0]\f[underline,1]下線ぁ\f[underline,0]\n二列目\n三列目\n\f[underline,1]下線ぁ\f[underline,0]\_a[OnBtbAnchor]アンカ\_a\e` |
| 環境変数 | 5.3 と同じ。ただし `AREKA_BALLOON_TIMEOUT_MS=30000`（撮る前に隠れないように）・`AREKA_APP_SMOKE_EXIT_MS=25000` |
| 撮り方 | 操作はしない。トークの終わり（`steady_talk_done`）の 1 秒後に、areka 自身の MCP の `dump_balloon`（scope 0）で撮った |
| コマンド | `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\areka-P0-balloon-text-bugs\real-machine-run.ps1 -Prepare` の後、同じファイルを `-Run Underline -TimeoutMs 30000 -ExitMs 25000` と `-Run Vertical -TimeoutMs 30000 -ExitMs 25000` で実行 |
| 走行 | Underline pid 25284・Vertical（1 回目）pid 34692・Vertical pid 33448。どれも自動終了・終了コード 0。ERROR は 0 行 |

### 7.2 残した画像（git で残す）

| 画像 | 元（`target\btb-signoff\`） | 写っているもの |
|---|---|---|
| `images/underline-bottom-row.png`（400×224） | `dump-underline.png` | 横書き・4 行。3 行目の作者の下線、4 行目（最下行）の作者の下線「下線gyぁ」、アンカーの下線「アンカgyぁ」が、すべて面の中に見えている |
| `images/underline-bottom-row-zoom4x.png`（1000×260） | `zoom-underline-bottom.png` | 上の画像の 3〜4 行目（x 30〜280・y 110〜175）を最近傍で 4 倍にしたもの。最下行の下線と、字の下の部分の欠け（7.4）が見える |
| `images/underline-vertical.png`（400×224） | `dump-vertical.png` | 縦書き・5 列（右から「下線ぁ」「二列目」「三列目」「下線ぁ」「アンカ」）。1 列目と 4 列目の作者の下線、5 列目のアンカーの下線が、列の左側に縦の線で見えている |

### 7.3 判定

| # | 確かめる項目（要件） | 結果 | 根拠 |
|---|---|---|---|
| 1 | 最下行の作者の下線が欠けずに見える（3.1） | **成立** | `underline-bottom-row.png`。y=166 の行の暗い画素（R＜100）の連なりは x 0〜33（バルーンの枠）・**x 36〜140（105 画素）＝作者の下線「下線gyぁ」**・x 149〜260（112 画素）＝アンカーの下線。作者の下線の範囲 x 36〜140 の暗い画素は、y=166 が 105・y=167 が 7（字の筆画）・y=168 以降が 0。文字の領域の下端は 46＋122＝168 なので、線は面の中にある |
| 2 | 最下行のアンカーの下線が欠けずに見える（3.1） | **成立** | 同じ画像の y=166 の **x 149〜260（112 画素）＝アンカー「アンカgyぁ」の下線**。この範囲の暗い画素は y=167 が 5（字の筆画）・y=168 以降が 0。x=200 では y=166 が `000000`、y=167 が `949494`（線の下端のにじみ） |
| 3 | 下線の位置は行ごとに同じ決め方（3.10） | **成立**（見た目） | 3 行目の作者の下線は y=136 の x 128〜232（105 画素）、4 行目は y=166。行送り 30 ちょうど。4 行目の線は面に収まるので、内側への寄せは掛かっていない |
| 4 | 拡大率が 100% でない（3.3） | **成立**（拡大率 2.0 で撮った） | 起動の記録の `k_shell=2.0 k_balloon=2.0` |
| 5 | 縦書きのバルーンでも下線が出る（3.4） | **成立**（見た目） | `underline-vertical.png`。線は列の左側で、今の描き方の側のまま。この台詞は 5 列なので、文字の領域の左端の列までは届いていない。端の列で面に収まらないときの寄せは、実 GPU の檻（タスク 2.6）が固定している |
| 6 | 最下行の字の下の部分（`g`・`y`・`ぁ`）が欠けていないか | **欠けている（`g`・`y`）**。`ぁ` は字の下の部分が無いので欠けない | 7.4 |

縦書きの 1 回目（`run-Vertical-1.log`・`dump-vertical-1.png`）は、基層に `vertical,1` だけを足した形で撮った。emo2-kakukaku は横書き用のバルーンで `wordwrappoint.y,0` を持ち、縦書きにすると 1 字ごとに列が折り返して 1 列 1 字になり、あふれて最後の 10 字だけが残った（右から「目 三 列 目 下 線 ぁ ア ン カ」、どの字の左にも下線の縦線）。2 回目は `wordwrappoint.y,-56`（validrect.bottom と同じ）で上書きして、列を文字の領域の下端まで使わせた。

### 7.4 最下行の字の下の部分の欠け（範囲外・完了の棚卸で起票）

- **見たこと**: `underline-bottom-row-zoom4x.png` で、3 行目の「gy」は字の下の部分が下線（y=136）を越えて y=142 まで描かれている（3 行目の作者の下線の範囲 x 128〜232 の暗い画素: 137〜142 行に 7・4・4・6・13・9、143 行は 0）。4 行目（最下行）は、作者の下線とアンカーの範囲 x 36〜260 で、167 行に 12 画素（字の筆画）があり、168〜169 行は 0。170〜172 行の 1〜2 画素は x=36・37 のバルーンの枠。そのため最下行の `g` は `q`、`y` は `v` のように見える（作者の下線の「下線gyぁ」とアンカーの「アンカgyぁ」の両方）。最下行の字は全部下線の区間の中にあるので、下線の無い字が最下行で欠けるかは、今回の台詞では確かめていない。
- **見立て**: 文字の面は validrect ちょうどの大きさ（高さ 122・下端 y=168）で作られ、4 行目のベースラインは y≈166。Yu Gothic UI 28 画素の字の下の部分は約 6 画素下（3 行目で 136→142）まで伸びるので、172 付近まで要るところが 168 で切れている。行の矩形（字の大きさの箱）で数えると 4 行目は収まるとされ、スクロールも起きない（brief の件 3 の「原因の見立て」の `visible_window` と同じ数え方）。
- **範囲**: 本 spec の件 3 は下線の問題で、字の形は変えない（要件 3.6）。要件 3.11 の寄せは下線にだけ掛かる。この欠けは下線の有無と関係なく起きる見込みで、本 spec の範囲の外。
- **起票先**: 本 spec の完了の棚卸（`/kiro-complete`）で `/kiro-discovery` によって起票する。`tasks.md` の Implementation Notes にも範囲外として載せる（本タスクでは起票していない）。

### 7.5 そのほか

- WARN は 5.4 と同じ形（`null.png` 2 行・`balloons20.png` の scope 1 の縮退 3 行・`force_quit` 1 行）に加えて、`折返し基準が描画範囲の外に解決された` が Underline（`axis="x" wrap_threshold=254.0`・59 行）と Vertical の 2 回目（`axis="y" wrap_threshold=147.0`・59 行。足した `wordwrappoint.y,-56` による）に 1 行ずつ。ERROR はどの回も 0 行。
- 撮った画像では、アンカーの字と線は黒（`000000`）で描かれていた。下線の確かめには関わらないので、色の決まりは本 spec では見ていない。
