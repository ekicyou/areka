# 実機の観測の記録（areka-P0-status-execution-states・タスク 7.3）

要件 2.2・3.4・4.7・5.2・7.2 の実機の確認。本物の emo2（32bit の pasta.dll を補助プロセス経由）を起こし、SHIORI へ送ったリクエストの記録（`shiori_request` の `status`）で 3 つの状態を確かめた。**3 状態とも、それぞれの区間でだけ載っていた。**

## 走らせ方

- 日時: 2026-10-03 16:16〜16:20 JST（記録の時刻は UTC）・HEAD `ff018838`（`cargo build -p areka` と i686 の `shiori-host32-helper` を組み直した `target\debug\areka.exe`）
- 根: `target\so\root`（`ghost\emo2`＝`target\nar-samples\manual\emo2` の写し・`balloon\emo2-kakukaku`）。置き場はすべてワークツリーの `target\so\` の下
- 環境変数: `NO_COLOR=1`・`RUST_LOG=info,areka=debug,areka::input_events=trace,kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS=90000`（有界の自動終了）・`AREKA_ROOT`／`AREKA_PROFILE_DIR`＝上の根（絶対パス）・`TMP`／`TEMP`＝`target\so\tmp`（URL の取得の一時フォルダ `%TEMP%\areka\download` をここへ向ける）
- 起動: 引数なし（根の列挙で emo2）。2 回とも自動終了で `app_exit origin=Smoke`・終了コード 0
- 台本: 製品のコードは変えず、**写したゴーストの辞書だけ**を書き換えた。`dic/boot.pasta` の `OnFirstBoot` と時刻帯の起動の場面（`起動朝`〜`起動halt`）を次の 1 本に差し替えた（`\w9` は 0.45 秒・行ごとに 8〜24 個）

  ```
  むらさき：観測の前半やで。\w9…
  エモ：\![enter,nouserbreakmode]ここから中断できない区間。\w9…
  むらさき：まだ中断できへんで。\w9…\![leave,nouserbreakmode]
  エモ：ここから中断できる。\w9…
  むらさき：\![execute,install,url,http://localhost:18765/obs,nar]ダウンロードするで。\w9…
  ```

- 取得先: 自分で起こした `127.0.0.1:18765` の小さな HTTP サーバー（Python）。`/obs` を 1 KB ずつ 0.15 秒おきに 40 回返す（約 6〜8 秒かかる＝取得の間に OnSecondChange が何度か入る）。中身は書庫ではないので、取得の後のインストールは失敗する（`install_failed`・`OnInstallFailure`＝想定どおり。`online` の観測には関係しない）。サーバーは走行の後に止めた
- 1 回目は台本の綴りを `\![install,url,…]` と誤り（正しくは `\![execute,install,url,…]`）、取得が起きなかった（`install_cue_skip name="install"`）。`balloon`・`nouserbreak` は 1 回目でも同じ形で観測できた。下の抜き書きは 2 回目

## 数（2 回目・`shiori_request` 98 件）

| 状態 | 載った件数 | 載った区間 | 区間の外で載った件数 |
|---|---|---|---|
| `balloon(…)` | 93 | バルーンが出ている間 | 0（出る前の `OnInitialize`〜`homeurl` の 5 件は載っていない） |
| `nouserbreak` | 14 | 旗が上がってから（03.624）下りるまで（17.422） | 0 |
| `online` | 8 | 取得の始まり（26.117）から終わり（34.008）まで | 0 |

- 届けの失敗 `no_user_break_send_failed`・`balloon_status_send_failed` は 0 件。置き場にゴーストが居ない `balloon_status_no_ghost` も 0 件
- `ERROR` は 2 件で、どちらも書庫でない検体を入れようとした `install_failed`（想定どおり）
- 1 回目（96 件）も `balloon(…)` 90 件・`nouserbreak` 14 件（区間の外 0 件）・`online` 0 件（取得が起きていない）

## 抜き書き（2 回目・UTC・`references` は省略）

### (a) バルーンが出ると `balloon(…)`

```
07:18:54.973 shiori_request GET    homeurl        status=Some("talking")
07:18:55.057 balloon_status_reported bindings=[0=0]
07:18:55.057 execution_state_updated kind="balloons" changed=true
07:18:55.362 shiori_request NOTIFY OnSecondChange status=Some("talking,balloon(0=0)")
07:19:03.625 balloon_status_reported bindings=[0=0, 1=0]          （\1 が喋り始めた）
07:19:04.350 shiori_request NOTIFY OnSecondChange status=Some("talking,nouserbreak,balloon(0=0/1=0)")
07:19:34.064 balloon_status_reported bindings=[1=0]               （OnInstallFailure の台本が \0 を消して \1 から始まった）
07:19:34.353 shiori_request NOTIFY OnSecondChange status=Some("talking,balloon(1=0)")
07:19:35.873 balloon_status_reported bindings=[0=0, 1=0]
07:19:36.355 shiori_request NOTIFY OnSecondChange status=Some("talking,balloon(0=0/1=0)")
07:19:39.355 shiori_request GET    OnSecondChange status=Some("balloon(0=0/1=0)")   （トークは終わり、バルーンは残っている）
```

### (b) `\![enter,nouserbreakmode]`〜`\![leave,nouserbreakmode]` の区間だけ `nouserbreak`

```
07:19:03.355 shiori_request NOTIFY OnSecondChange status=Some("talking,balloon(0=0)")
07:19:03.606 （台本の enter の cue が配られた）
07:19:03.623979 no_user_break_changed value=true                 （画面の側の旗）
07:19:03.624211 execution_state_updated kind="no_user_break" changed=true   （kanade の写し）
07:19:04.350 shiori_request NOTIFY OnSecondChange status=Some("talking,nouserbreak,balloon(0=0/1=0)")
   … 1 秒ごとに 14 件、すべて nouserbreak あり …
07:19:17.354775 shiori_request NOTIFY OnSecondChange status=Some("talking,nouserbreak,balloon(0=0/1=0)")
07:19:17.415 （台本の leave の cue が配られた）
07:19:17.421816 no_user_break_changed value=false
07:19:17.421974 execution_state_updated kind="no_user_break" changed=true
07:19:18.353 shiori_request NOTIFY OnSecondChange status=Some("talking,balloon(0=0/1=0)")
```

### (c) URL からの取得の間だけ `online`

```
07:19:25.362 shiori_request NOTIFY OnSecondChange status=Some("talking,balloon(0=0/1=0)")
07:19:26.116605 online_begin what="install-fetch" count=1
07:19:26.116792 install_fetch_begin url="http://localhost:18765/obs" dir=…\target\so\tmp\areka\download
07:19:26.356883 online_changed online=true
07:19:26.357143 shiori_request NOTIFY OnSecondChange status=Some("talking,online,balloon(0=0/1=0)")
   … 1 秒ごとに 8 件、すべて online あり …
07:19:33.354118 shiori_request NOTIFY OnSecondChange status=Some("talking,online,balloon(0=0/1=0)")
07:19:34.006609 install_fetch_done path=…\target\so\tmp\areka\download\29752-0-obs
07:19:34.007541 online_end what="install-fetch" count=0
07:19:34.022087 online_changed online=false
07:19:34.022290 shiori_request GET    OnInstallBegin   status=Some("talking,balloon(0=0/1=0)")
07:19:34.030386 shiori_request GET    OnInstallFailure status=Some("talking,balloon(0=0/1=0)")
```

## 旗の変化と送ったリクエストの時刻（要件 3.4 の運搬の間・要件 5.2）

| 変化 | 画面の側の旗 | kanade の写し | 運搬の間 | 直前に送った要求 | 直後に送った要求 |
|---|---|---|---|---|---|
| enter（2 回目） | 03.623979 | 03.624211 | 0.23 ms | 03.355（なし・正しい） | 04.350（あり・変化から 0.73 秒） |
| leave（2 回目） | 17.421816 | 17.421974 | 0.16 ms | 17.355（あり・旗はまだ上がっていた） | 18.353（なし・変化から 0.93 秒） |
| enter（1 回目） | 42.921017 | 42.921261 | 0.24 ms | 42.351（なし） | 43.355（あり） |
| leave（1 回目） | 56.762721 | 56.762935 | 0.21 ms | 56.363（あり） | 57.354（なし） |

- 画面の側の旗と kanade の写しが食い違うのは 0.16〜0.24 ms で、その間に送られた要求は 0 件。どの要求も、送った時点の画面の側の旗と同じ値を載せていた（「載っているのに中断できる」「載っていないのに中断できない」に当たる要求は 0 件）
- 台本の cue が配られてから画面の側の旗が変わるまでの 6〜18 ms は、既存の中断の旗の取り出し（画面のフレームの巡）で、中断を受け付けるかどうかの判定も同じ旗を読む
- `online` は取得のスレッドが数を立ててから、kanade が次のメッセージを受けた時点で写しへ移す（26.117 → 26.357・34.008 → 34.022）。どちらも写しを移した直後の要求から反映され、立っている間の要求には必ず載り、下ろした後の要求には載らなかった
- 変化から次の要求への反映はどれも 1 秒以内（`balloon` 0.29〜0.95 秒・`nouserbreak` 0.73〜0.93 秒・`online` 0.24 秒・取得の後の `OnInstallBegin` は 0.015 秒）。OnSecondChange 以外（`OnInstallBegin`・`OnInstallFailure` の GET）にも同じ規則で載っていた（要件 5.1 の傍証）

## 確かめていないこと

- バルーンが画面に見えている形との突き合わせ（要件 4.7）は記録の上だけ。GPU の合成窓はスクリーンショットで読めないため、表示層が報告した組（`balloon_status_reported`）と要求の `status` が一致することを見た

## 中断の拒否の目視（開発者・2026-10-03 17:47 JST）

開発者がバルーンを左ダブルクリックし、区間の中では台詞が続き、区間の後では止まることを目で確かめた。根は `target\so\eye`（辞書の起動の場面を「区間 約 30 秒 → 区間の後 約 27 秒」の台本に差し替えた写し）。記録（UTC）:

```
08:47:47.548 no_user_break_changed value=true
08:47:50.406 balloon_break_detected scope=0 → balloon_break_rejected reason="no_user_break"
08:47:52.39  OnSecondChange status=Some("talking,nouserbreak,balloon(0=0/1=0)")
08:47:59.164 balloon_break_detected scope=1 → balloon_break_rejected reason="no_user_break"
08:48:17.072 balloon_break_detected scope=0 → balloon_break_rejected reason="no_user_break"
08:48:20.40  OnSecondChange status=Some("talking,nouserbreak,balloon(0=0/1=0)")
08:48:21.007 no_user_break_changed value=false
08:48:21.38  OnSecondChange status=Some("talking,balloon(0=0/1=0)")
08:48:24.230 balloon_break_detected scope=0 → balloon_break_accepted talk_id=1（kanade）
08:48:24.39  OnSecondChange status=None（トークが止まり、バルーンも消えた）
```

- 区間の中の 3 回（` `・`` の両方のバルーン）はすべて断られ、そのときの要求には `nouserbreak` が載っていた。区間の後の 1 回は受け入れられ、トークが止まった。「載っているのに中断できる」「載っていないのに中断できない」は 0 件（要件 3.4）

## 後片付け

- 置き場: `target\so\`（根・一時フォルダ・サーバーの台本・生のログ `run1.log`／`run2.log`）。git の追跡外で、ワークツリーの片付けで消える
- 自分で起こしたプロセス（サーバー 1・areka 2）はすべて終了を確かめた
