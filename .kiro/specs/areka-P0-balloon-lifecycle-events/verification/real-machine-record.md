# 実機での確認の記録（タスク 7.2）

- 日付: 2026-10-06（R1〜R3）・2026-10-07（R4）
- 対象: areka（debug・`HEAD` の `e9a5c945` から組んだもの）と適合ゴースト emo2
- 走らせ方の正本: `.kiro/specs/areka-P0-balloon-lifecycle-events/real-machine-run.ps1`
- 記録の置き場: ワークツリーの `target\ble-signoff\`（根は `target\ble-signoff\root`・走行ごとの記録は `run-Rn.log`・叩いた記録は `run-Rn.clicks.txt`・起動と終了の時刻は `runs.txt`）

## 検体

- `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が展開した emo2 を `target\ble-signoff\root` へ写し、areka.exe と 32bit の補助プロセス（`target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe`・機種 014C を確かめてから写す）を同じ根に置いた。リポジトリの検体には触れていない。
- 写しの辞書にだけ `ghost\emo2\ghost\master\dic\zz_balloon_lifecycle.pasta` を足した（中身は走らせ方のスクリプトの `$dic`）。
  - `OnBalloonBreak`: 選択肢を含まない 2 行の台詞。
  - `OnBalloonClose`: `\![set,balloontimeout,3000]` を含む、選択肢を含まない台詞。
  - `OnBalloonTimeout`: 時間切れになった台本（Reference0）に `balloontimeout` の文字が**無ければ** `\![set,balloontimeout,3000]` つきの台詞（「時間切れ短縮」）を、**あれば**指定の無い台詞（「時間切れ既定」）を返す。利用者の操作が無くても「3 秒で消える → 次のトークは既定へ戻る」が交互に起きるようにするための作りで、⑷ を無人で観測できたのはこの台詞による（`OnBalloonClose` の台詞が運んだ ⑷ は別に記す）。
- 起動は絶対パス（`<根>\areka.exe "<根>\ghost\emo2"`・作業フォルダも根）。記憶（profile）は毎回消してから起こすので、起動のトークは `OnFirstBoot` の自己紹介になる。
- 環境変数（スクリプトが設定する）: `RUST_LOG=info,kanade=trace,areka::input_events=trace`（可視性の相の info・kanade の trace・入力の判定の trace）、`AREKA_APP_SMOKE_EXIT_MS`（有界の自動終了）、`AREKA_BALLOON_TIMEOUT_MS`（既定の待ち時間の短縮）、`AREKA_ROOT`・`AREKA_PROFILE_DIR`・`TMP`/`TEMP`（すべて根の下）。他の `AREKA_*`・`WINTF_*` は消してから起こす。

## 走行の一覧

| 走行 | コマンド（`real-machine-run.ps1` の引数） | 終了コード | ERROR | WARN | 内容 |
|---|---|---|---|---|---|
| R1 | `-Run R1 -ExitMs 75000 -TimeoutMs 8000` | 1 | 3 | 3 | 失敗（検体の誤り）。足した辞書の最後の行に改行が無く、pasta が構文の誤りで LOAD を断った（`zz_balloon_lifecycle.pasta:29:63`）。SHIORI に繋がらず終了系列（Fault）へ。スクリプトを直して（最後の行にも改行を書く）根を作り直した。本仕様の振る舞いの確認には使っていない |
| R2 | `-Run R2 -ExitMs 75000 -TimeoutMs 8000` | 0 | 0 | 4 | 無人。⑵ と ⑷ を確かめた |
| R3 | `-Run R3 -Clicks Break,Close -TimeoutMs 15000 -ExitMs 45000` | 0 | 0 | 4 | 叩く手順の空運転。入力デスクトップが `Screen-saver` だったので叩かず、合図の行とバルーン窓の見分けだけを確かめた |

- R2・R3 の WARN 4 行の内訳: emo2 の素材由来の既知のもの 3 行（`purple/a/null.png` が全透明 ×2・`emo2-kakukaku` の折返し基準が描画範囲の外 ×1）と、自動終了のときの `force_quit reason="user"` 1 行。本仕様の記録の WARN（`balloon_timeout_notice_failed`・`StopTimeMissing`・控えが無い）は 0 行。
- R1 で見つけた本仕様と無関係のこと: SHIORI に繋がらないときに出る知らせの窓（題名「SHIORI が動かなくなりました」）が閉じられるまで、有界の自動終了（`origin=Smoke`）が記録されてもプロセスが終わらなかった。自分で起こした pid（38016・根の areka.exe であることを確かめた）の知らせの窓へ `WM_CLOSE` を送って終わらせた（終了コード 1）。窓は終了の処理の最後に出る作りで、自動の走行のための抑止口 `AREKA_NO_ALERT` があるので、完了時（2026-10-08）に走らせ方のスクリプトがこれを立てるように直した（抑えても記録 `error!(event="alert")` は残る）。

## 場面ごとの照合

ログの時刻は UTC。`talk_time`（`stopped_at`・`display_end`・`deadline`）はトークの開始からの秒。

### ⑵ 放っておくとバルーンが消えて、知らせと `OnBalloonTimeout` が出る（R2・既定の待ち時間 8 秒）

起動のトーク（talk_id=1・`OnFirstBoot`）が最後まで流れ、8 秒後に時間切れで消えた。

```
05:01:19.043012Z kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
05:01:19.046719Z [balloon-visibility] タイムアウト計測を開始（起点＝止まった時刻） origin="stopped_at" stopped_at=17.058844299997872 display_end=17.099999999999998 deadline=25.058844299997872
05:01:27.047223Z [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="timeout" visible=false
05:01:27.048148Z [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="timeout" visible=false
05:01:27.048449Z [balloon-visibility] 時間切れで隠したことを運行の側へ知らせた event="balloon_timeout_notified" talk_id=1
05:01:27.048482Z kanade: バルーンのイベントを送る（要件 7.1） event="balloon_event_sent" id="OnBalloonTimeout" cause="timeout" talk_id=1
05:01:27.048809Z kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBalloonTimeout references=["\\p[1]\\s[通常]\\1\\![move,-353,,,0,base,base]\\p[0]…（talk_id=1 の台本）", "0"]
05:01:27.051134Z kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=2 origin="OnBalloonTimeout"
```

- 送った記録: `balloon_event_sent id="OnBalloonTimeout" cause="timeout" talk_id=1`。Reference0 は talk_id=1 の台本、Reference1 は `0`。
- 起点の記録: `origin="stopped_at"`。計測の開始は `steady_talk_done` の 3.7 ミリ秒後。満了予定 25.0588 ＝ 止まった時刻 17.0588 ＋ 8 秒。隠れたのは計測の開始のちょうど 8.000 秒後。
- 止まった時刻（17.0588）が占有区間の終端（17.1000）より 41 ミリ秒早く読まれ、早い方として採られている。最後まで流れたトークでもこうなりうることは設計（「起点」の節・最後まで流れたトーク）に書かれているとおり。
- 送らなかった理由の記録（`balloon_event_not_sent`）: R2 全体で 0 行。
- 同じ型の 2 回目: talk_id=4（`OnSecondChange` のランダムトーク）も `steady_talk_done` 05:01:49.036251Z → 計測の開始 05:01:49.043376Z（`deadline=11.69628609999927`＝3.6963＋8）→ 05:01:57.055038Z に `trigger="timeout" visible=false` → `balloon_timeout_notified talk_id=4` → `balloon_event_sent id="OnBalloonTimeout" cause="timeout" talk_id=4`。

### ⑷ `\![set,balloontimeout,3000]` で表示が終わってから約 3 秒で消え、`OnBalloonTimeout`、次のトークは既定へ戻る（R2）

この走行で `\![set,balloontimeout,3000]` を運んだのは `OnBalloonTimeout` への応答の「時間切れ短縮」の台詞（talk_id=2 と talk_id=5）。`OnBalloonClose` の台詞が運ぶ ⑷ は ⑶ の走行で確かめる（下記）。

```
05:01:27.051495Z kanade: SHIORI 送出 event="shiori_request" method=GET id=OnTranslate references=["\\p[1]\\s[静観]\\![set,balloontimeout,3000]時間切れの知らせ、\\_w[450]届いたよ。\\_w[950]これは3秒で消えるよ。\\_w[950]\\e", …]
05:01:27.080350Z areka::emo2_boot::talk_lifecycle: BalloonLifecycleSink: 待ち時間の指定を読んだ event="balloon_timeout_set" timeout=Millis(3000)
05:01:27.090638Z [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="content" visible=true
05:01:30.721053Z kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
05:01:30.723499Z [balloon-visibility] タイムアウト計測を開始（起点＝止まった時刻） origin="stopped_at" stopped_at=3.5843534000014188 display_end=3.5999999999999996 deadline=6.584353400001419
05:01:33.723692Z [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="timeout" visible=false
05:01:33.727028Z [balloon-visibility] 時間切れで隠したことを運行の側へ知らせた event="balloon_timeout_notified" talk_id=2
05:01:33.727053Z kanade: バルーンのイベントを送る（要件 7.1） event="balloon_event_sent" id="OnBalloonTimeout" cause="timeout" talk_id=2
05:01:33.727342Z kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBalloonTimeout references=["\\p[1]\\s[静観]\\![set,balloontimeout,3000]時間切れの知らせ、…\\e", "0"] status=Some("balloon(1=0)")
05:01:33.729700Z kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=3 origin="OnBalloonTimeout"
05:01:38.931655Z kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
05:01:38.932808Z [balloon-visibility] タイムアウト計測を開始（起点＝止まった時刻） origin="stopped_at" stopped_at=5.094181500000559 display_end=5.1000000000000005 deadline=13.094181500000559
05:01:45.240117Z kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=4 origin="OnSecondChange"
05:01:45.363756Z [balloon-visibility] タイムアウト計測を破棄 reason="talk_started" deadline=13.094181500000559
```

- 待ち時間の指定の記録（要件 7.5）: `balloon_timeout_set timeout=Millis(3000)`。読めない値の WARN は 0 行。
- 約 3 秒で消えた: 満了予定 6.5844 ＝ 止まった時刻 3.5844 ＋ 3 秒。計測の開始（05:01:30.723499）から隠れるまで（05:01:33.723692）がちょうど 3.000 秒。
- 計測の開始の記録はトークの終わり（`steady_talk_done` 05:01:30.721053）の 2.4 ミリ秒後で、大きく遅れていない。
- 送った記録: `balloon_event_sent id="OnBalloonTimeout" cause="timeout" talk_id=2`（Reference0 は指定を含む台本そのもの）。
- 次のトークで既定へ戻った: 次の talk_id=3（「時間切れ既定」・指定なし）の満了予定は 13.0942 ＝ 5.0942 ＋ **8 秒**（既定）。talk_id=3 は満了の前に次のランダムトーク（talk_id=4）が始まったので計測が破棄され（`reason="talk_started"`）、時間切れの知らせも `OnBalloonTimeout` も出ていない（出ないのが正しい）。既定へ戻ったことは talk_id=4 が 8 秒で消えたこと（⑵ の 2 回目）でも確かめられる。
- 同じ型の 2 回目: talk_id=5（「時間切れ短縮」）は `balloon_timeout_set timeout=Millis(3000)`（05:01:57.092149Z）→ `steady_talk_done` 05:02:00.730648Z → 計測の開始 05:02:00.737858Z（7.2 ミリ秒後・`deadline=6.5885928999996395`＝3.5886＋3）→ 05:02:03.739154Z に `trigger="timeout"` で消え（計測の開始の 3.001 秒後）→ `balloon_timeout_notified talk_id=5` → `balloon_event_sent id="OnBalloonTimeout" cause="timeout" talk_id=5`。続く talk_id=6（指定なし）の満了予定は `deadline=13.093188200004079`＝5.0932＋8（既定）。
- 送らなかった理由の記録: 0 行。

R4 では `OnBalloonClose` の台詞が運んだ ⑷ も確かめた（⑶ の節の後半）。

### ⑴ 長い台詞の途中のダブルクリックで `OnBalloonBreak` が正しい scope で送られる（R4・2026-10-07）

起動のトーク（talk_id=1・`OnFirstBoot`）の途中で、むらさき（scope 0）のバルーンの中央（物理画素 2196,576）を左ダブルクリックした。

```
14:10:51.350401Z [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true
14:10:53.193464Z wintf: [handle_double_click_message] Double-click detected window_entity=24v0 target_entity=32v0 double_click=Left x=400 y=224
14:10:53.197320Z areka::input_events::user_break: バルーンの左ダブルクリックを検出 event="balloon_break_detected" scope=0
14:10:53.197683Z kanade: 利用者の中断を受け入れた——現行のトークを止める（要件 2.1・6.1） event="balloon_break_accepted" scope=0 talk_id=1 phase="Steady"
14:10:53.199070Z kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
14:10:53.199355Z [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="user_break" visible=false
14:10:53.199530Z kanade: バルーンのイベントを送る（要件 7.1） event="balloon_event_sent" id="OnBalloonBreak" cause="break" scope=0 talk_id=1
14:10:53.200249Z kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBalloonBreak references=["\\p[1]\\s[静観]\\1\\![move,-353,,,0,base,base]\\p[0]…（talk_id=1 の台本）", "0", ""]
14:10:53.205713Z kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=2 origin="OnBalloonBreak"
```

- 送った記録: `balloon_event_sent id="OnBalloonBreak" cause="break" scope=0 talk_id=1`。叩いたバルーンの scope（0）がそのまま Reference1 に入り、Reference0 は止めたトークの台本、Reference2 は空。
- 中断を受け入れてから送るまで 1.8 ミリ秒。中断で止まったトークに `OnBalloonTimeout`・`OnBalloonClose` は出ていない。
- 応答の台詞（talk_id=2）が続けて流れた。
- 起点の記録: 無い（正しい）。利用者の中断は見えている scope をすべて一度に隠すので（決定論のテスト `balloon_visibility_user_break_tests.rs` の `a_user_break_hides_every_visible_scope_in_one_action`。⑶ でも scope 0・1 が同時に隠れている）、中断で終わったトークには計測が始まらない。この走行では中断のとき見えていたのは scope 0 だけだった（scope 1 が初めて見えるのは 14:10:54.62）。
- 送らなかった理由の記録: 0 行。

### ⑶ 台詞が終わったバルーンのダブルクリックで `OnBalloonClose`（と、その台詞が運ぶ ⑷）（R4・2026-10-07）

⑴ の応答（talk_id=2）が最後まで流れた `steady_talk_done` の 1.15 秒後、残っているむらさきのバルーンを同じ点で左ダブルクリックした（既定の待ち時間は 15 秒にしてあるので、まだ出ている）。

```
14:10:56.609738Z kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
14:10:56.613274Z [balloon-visibility] タイムアウト計測を開始（起点＝止まった時刻） origin="stopped_at" stopped_at=3.348263899999438 display_end=3.3500000000000005 deadline=18.348263899999438
14:10:57.762972Z areka::input_events::user_break: バルーンの左ダブルクリックを検出 event="balloon_break_detected" scope=0
14:10:57.763765Z kanade: 中断の要求を受けたが再生中のトークが無い——何も止めない（要件 2.2） event="balloon_break_no_talk" scope=0 reason="not_playing" phase="Steady"
14:10:57.764405Z kanade: バルーンのイベントを送る（要件 7.1） event="balloon_event_sent" id="OnBalloonClose" cause="close" talk_id=2
14:10:57.764885Z kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBalloonClose references=["\\p[0]\\s[1000]…止められてもた。\\_w[950]\\p[1]\\s[静観]中断の知らせ、\\_w[450]届いたよ。\\_w[950]\\e"]
14:10:57.765597Z [balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="user_break" visible=false
14:10:57.766531Z [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="user_break" visible=false
14:10:57.767598Z [balloon-visibility] タイムアウト計測を破棄 reason="no_visible_scope" deadline=18.348263899999438
14:10:57.769390Z kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=3 origin="OnBalloonClose"
14:10:57.806026Z areka::emo2_boot::talk_lifecycle: BalloonLifecycleSink: 待ち時間の指定を読んだ event="balloon_timeout_set" timeout=Millis(3000)
14:11:00.509176Z kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
14:11:00.513068Z [balloon-visibility] タイムアウト計測を開始（起点＝会話の占有終端） origin="display_end" display_end=2.7 deadline=5.7
14:11:03.521997Z [balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="timeout" visible=false
14:11:03.523172Z [balloon-visibility] 時間切れで隠したことを運行の側へ知らせた event="balloon_timeout_notified" talk_id=3
14:11:03.523197Z kanade: バルーンのイベントを送る（要件 7.1） event="balloon_event_sent" id="OnBalloonTimeout" cause="timeout" talk_id=3
14:11:03.531099Z kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=4 origin="OnBalloonTimeout"
14:11:08.660999Z kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
14:11:08.662524Z [balloon-visibility] タイムアウト計測を開始（起点＝止まった時刻） origin="stopped_at" stopped_at=5.097116000000824 display_end=5.1000000000000005 deadline=20.097116000000824
```

- 送った記録: `balloon_event_sent id="OnBalloonClose" cause="close" talk_id=2`。再生中のトークが無いので中断は何も止めず（`balloon_break_no_talk reason="not_playing"`）、読み終えたバルーンを閉じた。Reference0 は閉じたバルーンに出ていた台本（talk_id=2 の台本）で、Reference は 1 つだけ。
- 閉じたことで時間切れの計測は破棄され（`reason="no_visible_scope"`）、talk_id=2 に `OnBalloonTimeout` は出ていない。
- その台詞が運ぶ ⑷: `OnBalloonClose` の台詞（talk_id=3）が `balloon_timeout_set timeout=Millis(3000)` を記録し、計測の開始（14:11:00.513068）からちょうど 3.009 秒後に `trigger="timeout"` で消えた（満了予定 5.7 ＝ 占有終端 2.7 ＋ 3 秒）→ `balloon_timeout_notified talk_id=3` → `balloon_event_sent id="OnBalloonTimeout" cause="timeout" talk_id=3`。
- 次のトークで既定へ戻った: 続く talk_id=4（指定なし）の満了予定は 20.0971 ＝ 5.0971 ＋ **15 秒**（この走行の既定）。
- talk_id=3 の起点は `origin="display_end"`（占有終端の方が早かった）で、`stop_time_missing` の WARN ではない。
- 送らなかった理由の記録（`balloon_event_not_sent`）・台本の控えが無い記録（`balloon_event_script_missing`）: R4 全体で 0 行。

### R4 に至るまでの叩く手順の直し（2026-10-07）

R4 は同じ名前で 3 回起こし、記録（`run-R4.log`・`run-R4.clicks.txt`）は 3 回目で上書きされている。上の照合はすべて 3 回目のもの。

1. 1 回目（23:06:40 起動）: 入力デスクトップは `Default` だったが `sent=0`。スクリプトの `INPUT` 構造体に余計な 8 バイトの詰め物があり（48 バイト・x64 の正しい大きさは 40）、SendInput が引数ごと断っていた。areka は何も受け取っていない。詰め物を消した。
2. 2 回目（23:08:41 起動）: ⑴ は取れた（`OnBalloonBreak scope=0 talk_id=1`）。⑶ は空振り。原因は 2 つ。Close の合図に中断そのものの `steady_talk_done` を拾い、中断への応答の台詞の最中に叩いた。押す 2 回の間にポインタが動き（2245,626 と 2256,653）、ダブルクリックにならなかった。Close の合図を「次のトークの開始（`steady_talk`）の後の `steady_talk_done`」に改め、各入力に絶対座標を載せた。
3. 3 回目（23:10:48 起動）: ⑴・⑶・⑶ の台詞が運ぶ ⑷ をすべて取れた（上記）。

## 走行の一覧（追記）

| 走行 | コマンド（`real-machine-run.ps1` の引数） | 終了コード | ERROR | WARN | 内容 |
|---|---|---|---|---|---|
| R4（3 回目） | `-Run R4 -Clicks Break,Close -TimeoutMs 15000 -ExitMs 90000` | 0 | 0 | 4 | ⑴・⑶ と、`OnBalloonClose` の台詞が運ぶ ⑷ を確かめた。WARN の内訳は R2・R3 と同じ（素材由来 3 行＋自動終了の `force_quit` 1 行） |

- 抜き出す行: `rg -e balloon_break_ -e balloon_event_ -e balloon_timeout_ -e "タイムアウト計測" -e "trigger=" -e steady_talk target\ble-signoff\run-R4.log`
