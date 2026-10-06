# 実機での確認の記録（タスク 7.2）

- 日付: 2026-10-06
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
- R1 で見つけた本仕様と無関係のこと: SHIORI に繋がらないときに出る知らせの窓（題名「SHIORI が動かなくなりました」）が閉じられるまで、有界の自動終了（`origin=Smoke`）が記録されてもプロセスが終わらなかった。自分で起こした pid（38016・根の areka.exe であることを確かめた）の知らせの窓へ `WM_CLOSE` を送って終わらせた（終了コード 1）。

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

### ⑴ 長い台詞の途中のダブルクリックで `OnBalloonBreak` が正しい scope で送られる — 未確認（入力を送れない）

### ⑶ 台詞が終わったバルーンのダブルクリックで `OnBalloonClose`（と、その台詞が運ぶ ⑷）— 未確認（入力を送れない）

- 2 つとも左ダブルクリックが要る。走らせ方のスクリプトに、記録を読みながら SendInput でバルーンを叩く手順（`-Clicks Break,Close`）を用意し、R3 で空運転した。
  - 合図の行は取れている: Break は最初の `visible=true`（R3 では 05:02:55.342026Z の `scope=0 trigger="content" visible=true`）から 1.5 秒後、Close は次の `steady_talk_done`（05:03:12.363557Z）から 0.7 秒後に叩く手順まで進んだ。
  - バルーン窓の見分けも取れている: 窓は 4 枚とも題名がキャラクターの名前（scope 0＝むらさき・scope 1＝エモ）で、同じ題名の 2 枚のうち面積の小さい方がバルーン。R3 では むらさき のバルーン窓が物理画素で `1796,352,2596,800`（800×448）で、記録の `balloon_size: SizePx { w: 800, h: 448 }` と一致した。
- 叩けなかった理由: 入力デスクトップが確認の間ずっと `Screen-saver` だった（13:5x の最初の確認から 14:33:47 まで 30 分待ったが `Default` に戻らなかった）。スクリーンセーバー（またはロック）の間は SendInput がアプリへ届かないので、叩かずに記録だけを残した（`run-R3.clicks.txt` の「入力デスクトップが Default でないので叩かない」）。無理に叩いた結果や作った記録は載せていない。
- 画面に戻ったら次の 1 本で ⑴ と ⑶（と `OnBalloonClose` の台詞が運ぶ ⑷）をまとめて取れる:

```
pwsh -NoProfile -File .kiro/specs/areka-P0-balloon-lifecycle-events/real-machine-run.ps1 -Run R4 -Clicks Break,Close -TimeoutMs 15000 -ExitMs 90000
```

  - Break: 起動のトーク（約 17 秒）の 1.5 秒地点で むらさき（scope 0）のバルーンを叩く → 期待は `balloon_break_accepted scope=0 talk_id=1` → `balloon_event_sent id="OnBalloonBreak" cause="break" scope=0 talk_id=1`。
  - Close: その応答（「止められてもた。」）が最後まで流れた `steady_talk_done` の 0.7 秒後に叩く（既定の待ち時間は 15 秒にしてあるので、まだ出ている）→ 期待は `balloon_break_no_talk` → `balloon_event_sent id="OnBalloonClose" cause="close"`。続く `OnBalloonClose` の台詞で `balloon_timeout_set timeout=Millis(3000)` → 約 3 秒で `trigger="timeout"` → `OnBalloonTimeout` → 次のトークの満了予定が 15 秒（既定）に戻る。
  - 自動で叩けないときは手で叩いてよい: 上のコマンドを `-Clicks` なしで起こし、起動のトークの途中で むらさき のバルーンを左ダブルクリック（⑴）、その応答が流れ終わって残っているバルーンを左ダブルクリックし、すぐポインタをバルーンから外す（⑶。バルーンの上にポインタを残すと時間切れが抑止される）。
- 抜き出す行: `rg -e balloon_break_ -e balloon_event_ -e balloon_timeout_ -e "タイムアウト計測" -e "trigger=" -e steady_talk target\ble-signoff\run-R4.log`
