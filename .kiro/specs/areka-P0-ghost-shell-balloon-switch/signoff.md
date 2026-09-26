# 実機確認の記録（要件 3.8・4.9・10.11・10.12）

- 日時: 2026-09-27 07:00〜07:05 JST（ログの時刻は UTC で 2026-09-26 22:00〜22:05）
- HEAD: `1a41da42`（ブランチ `claude/areka-p0-ghost-shell-balloon-871462`）
- 環境: Windows 11 Pro 10.0.26200・モニタ 2 台（主 2880x1800・DPI 192／副 2560x1600・DPI 144。ログの `diag.monitor` の値）
- 実行体: `target\debug\areka.exe`（x64 debug）。`cargo build -p areka -j 4` が HEAD で「建て直し無し」で終わることを確かめてから複製した
- i686 の成果物: PowerShell で `cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc`。helper は PE の機械種別 `0x014C`、areka.exe は `0x8664` を確かめた。複製した 2 つのハッシュは `target\` の元と一致

## 検体（根は 1 つ・短い絶対パス）

配布物と同じ形に、実行体を根の直下に置いた。根は「実行ファイルのあるフォルダ」（`AREKA_ROOT` は外した）。

```
C:\tmp\areka-signoff-gsw\root\
  areka.exe                       … target\debug\areka.exe の複製
  shiori-host32-helper.exe        … i686 の helper
  ghost\emo2\                     … `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が配った emo2
  ghost\R_POST_and_KOMAINU\       … C:\home\maz\tmp\loadu\ascii\R_POST_and_KOMAINU を丸ごと複製（install.txt の directory,R_POST_and_KOMAINU・type,ghost）
  ghost\fail1\                    … SHIORI が失敗する 1 キャラのゴースト（下記）
  balloon\emo2-kakukaku\          … emo2 の同梱バルーン
  balloon\StayseeBalloon\         … 既定バルーン（`nar-sample-path -- StayseeBalloon` が配ったもの）
```

- R_POST_and_KOMAINU は複製元に `ghost\master\profile\areka\sylphya.toml`（`[boot] count = "1"`）が既にあり、起動記録のあるゴーストとして使った（① で `OnGhostChanged` が送られる条件）。SHIORI は里々（`satori.dll`・i686 の helper 経由）。バルーンの同梱は無く、既定バルーン StayseeBalloon へ決まる
- 失敗するゴースト `fail1`: emo2 の複製の `ghost\master\descript.txt` を `charset,UTF-8`／`name,fail-one`／`shiori,shiori_loadu.dll`／`seriko.defaultsurfacedirectoryname,master` の 4 行に差し替え（`sakura.name`・`kero.name` 無し＝1 キャラ）、i686 の `shiori_loadu.dll` を同じフォルダへ置いた（前回の `shiori-fault-notice` の実機確認と同じ組み立て）。走行 ④ だけ `HOST32_TESTDLL_LOADU_FAIL=1` を付けて LOAD を失敗させた
- 目録の並び（メニューの「ゴースト」枠の子）: `Ｒポストと狛犬`・`えも？？`・`fail-one`（3 つとも選べる）

## 全走行に共通の設定

- 起動は **argv なし**（ゴーストもバルーンも起動の解決に任せる）
- 環境変数（`C:\tmp\areka-signoff-gsw\tools\launch.ps1` が設定して起動し、終了を待って終了コードを残す）:
  - `NO_COLOR=1`
  - `AREKA_NO_ALERT=1`（告知が出ても窓で止まらないようにするため。告知の記録 `event="alert"` は抑止でも 1 件残るので、0 件であることで「告知が出ない」を判定した）
  - `AREKA_APP_SMOKE_EXIT_MS`（① ② ④ は 60000・③ は 45000）＝全走行が自分で終わる
  - `AREKA_PROFILE_DIR` は走行ごとに `C:\tmp\areka-signoff-gsw\` の下のフォルダ（開発者のアプリの記憶は読まず・書かない）。⑤＋② は ① と同じフォルダを使う
  - `RUST_LOG=info,kanade=trace,areka_kanade=trace,areka=debug,areka_emo_text=info,areka_emo_present=info,areka_ghost=debug,ghost-shutdown=debug,ghost-boot=debug,shiori-actor=debug`
    - kanade の切替の相（`change_*`・target `kanade`）と SHIORI の送出の記録（`shiori_request`・trace・Reference つき）、areka の切替の事象（`ghost_switch_*`）・`windows_closed_for_restart`・`app_exit`、降ろす処理の段ごとの記録（target `ghost-shutdown` の debug）まで開けた。`areka=debug` は `areka_emo_text`・`areka_emo_present` の debug まで拾って 1 走行 2 万行を超えたので、この 2 つだけ info に戻した（判定に使う記録は含まない）
- 標準出力と標準エラーは `C:\tmp\areka-signoff-gsw\logs\<走行>.out.log`／`.err.log`、終了コードと所要時間は `<走行>.exit.txt`
- UI の操作: **開発者の手ではなく、エージェントが Win32 と UI Automation で行った。**
  - キャラ窓の中の不透明な点は、`SetCursorPos` でカーソルを動かし、クリック透過の旗（`WS_EX_TRANSPARENT`）が外れる点を探して決めた
  - 右クリックとダブルクリックは `SendInput`（実際のマウス入力と同じ経路）
  - 右クリックメニュー（Win32 のポップアップメニュー）は UI Automation で項目の名前と位置を読み、「ゴースト」→ 子の項目の順に `SendInput` で左クリックした
- プロセス: 各走行の終了後に `areka.exe`・`shiori-host32-helper.exe` がどちらも 0 件であることを確かめた

## 走行 ①: emo2 → メニュー「ゴースト」→ R_POST_and_KOMAINU

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run1 -Profile C:\tmp\areka-signoff-gsw\prof1 -SmokeMs 60000
```

- 起動: `ghost_resolved route=Default`（emo2・記憶の無い新しい記憶の置き場）→ `OnBoot` の挨拶
- 挨拶の後、emo2（むらさき）のキャラ窓を右クリック → 「ゴースト」→「Ｒポストと狛犬」を選んだ（emo2 のメニュー: `ゴースト | 説明書 | 終了`）
- 流れ（時刻は UTC・秒）:

  | 時刻 | 事象 |
  |---|---|
  | 22:01:02.157 | `ghost_switch_requested`（from=えも？？ to=Ｒポストと狛犬 raise_event=true origin=manual）・`change_accepted` |
  | 02.157 | `OnGhostChanging` GET: `["ポスト", "manual", "Ｒポストと狛犬", "C:\tmp\areka-signoff-gsw\root\ghost\R_POST_and_KOMAINU"]` |
  | 02.158 | emo2 は 204 → `change_close_begin` → `OnClose` GET `["system"]` |
  | 02.160 | `change_talk_start`（phase=ChangeCloseTalkWait）＝emo2 の送り出しの台詞「えー、」「もう終わりなん？」「またすぐ会えるよ。」「ほな、」「またな！」 |
  | 06.604 | 台詞が `\-` に達して終わる（`talk_done_quit`）→ kanade が SHIORI を解放（`unload_clean` 06.650） |
  | 06.660 | `ghost_switch_down_ms` **ms=1** |
  | 06.664 | `windows_closed_for_restart` closed=4（`app_exit` は出ない） |
  | 06.800 | `last_used_recorded` ghost=R_POST_and_KOMAINU balloon=StayseeBalloon・`ghost_switch_booted`（attempt=Target） |
  | 06.839 | 切替先の窓が出た（「本物のゴースト窓を開きました」scopes=[0, 1]） |
  | 07.046 | `OnGhostChanged` GET: `["むらさき", "", "えも？？", "C:\tmp\areka-signoff-gsw\root\ghost\emo2", "", "", "", "master"]` |
  | 07.046 | R_POST が台本を返す（`boot_type_script`＝`OnBoot` は送らない）→ 交代の台詞「いらっしゃませ。」「違うだろ。」「……」「なんだよ。」 |
  | 07.051 | `ghost_switch_done`（ghost=R_POST_and_KOMAINU attempt=Target） |
  | 22:01:56.813 | 自動終了 `app_exit` origin=Smoke closed=4 |

- 終了コード: **0**（60.58 秒）
- 目印の件数:

  | 目印 | 件数 |
  |---|---|
  | `ghost_switch_requested` | 1 |
  | `change_accepted`／`change_close_begin`／`change_talk_start` | 1／1／1 |
  | `change_talk_done_unload`／`talk_done_quit` | 0／1（送り出しの台詞が `\-` で終わったため。下の「気付いたこと」1） |
  | `change_cancelled` | 0 |
  | `ghost_switch_down_ms` | 1（ms=1） |
  | `windows_closed_for_restart` | 1（closed=4） |
  | `ghost_switch_booted` | 1（Target） |
  | `ghost_switch_done`（切替を終える info） | 1 |
  | `ghost_switch_cancelled`／`ghost_switch_target_fault` | 0／0 |
  | `app_exit` | 1（origin=Smoke のみ） |
  | `ghost_quit`／`alert` | 0／0 |
  | `OnGhostChanging`／`OnGhostChanged`／`OnFirstBoot` | 1／1／0 |
  | ERROR 行／パニック | 0／0 |

- 判定（要件 10.11 ①）: **合格**。emo2 の送り出しの台詞のあと、同じプロセスのまま R_POST が起き、`OnGhostChanged`（Ref0＝直前の本体側の名前 むらさき・Ref2＝えも？？・Ref3＝emo2 の場所・Ref7＝master・Ref1 と Ref4〜6 は空）に里々が「他のゴーストから変更」の台本で答え、それが交代の台詞として流れた。里々のこの見出しは 2 つの台本から無作為に選び、今回は「いらっしゃませ。…なんだよ。」が出た（R_POST の `dic04_Change.txt` はどちらの台本でも `（Ｒ０）` を飛び先の行でしか使わないので、「むらさきから」の字面はどちらでも出ない）。終了の指示は自動終了の 1 件だけ。

## 走行 ⑤＋②: 再起動で R_POST（記憶）→ メニューで emo2 へ（往復）

① の終了後、同じ記憶の置き場（`prof1`・① の後の中身は `[last] ghost = "R_POST_and_KOMAINU"`）で argv なしに再起動し、そのままメニューで emo2 へ戻した。

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run5 -Profile C:\tmp\areka-signoff-gsw\prof1 -SmokeMs 60000
```

- 起動: `ghost_resolved route=Memory dir=...\ghost\R_POST_and_KOMAINU`・`balloon_resolved route=Memory ...\balloon\StayseeBalloon` → `OnBoot` `["master"]` → 「おはようございます。」
- 挨拶の後、R_POST（ポスト）のキャラ窓を右クリック → 「ゴースト」→「えも？？」（R_POST のメニュー: `ゴースト | Read me(R) | 終了`）
- 流れ（時刻は UTC・秒）:

  | 時刻 | 事象 |
  |---|---|
  | 22:02:11.673 | `ghost_switch_requested`（from=Ｒポストと狛犬 to=えも？？ origin=manual）・`change_accepted` |
  | 11.673 | `OnGhostChanging` GET: `["むらさき", "manual", "えも？？", "C:\tmp\areka-signoff-gsw\root\ghost\emo2"]` |
  | 11.674 | 里々が台本を返す → `change_talk_start`（phase=ChangeTalkWait）＝「チェインジ！」 |
  | 12.057 | `change_talk_done_unload`（reason=Ended）→ `unload_clean` 12.066 |
  | 12.068 | `ghost_switch_down_ms` **ms=1** |
  | 12.071 | `windows_closed_for_restart` closed=4 |
  | 12.520 | `last_used_recorded` ghost=emo2 balloon=emo2-kakukaku・`ghost_switch_booted`（Target） |
  | 12.563 | 切替先の窓が出た |
  | 12.750 | `OnGhostChanged` GET: `["ポスト", "\0\s[0]\1\s[10]\0チェインジ！\e", "Ｒポストと狛犬", "C:\tmp\areka-signoff-gsw\root\ghost\R_POST_and_KOMAINU", "", "", "", "master"]` |
  | 12.751 | emo2 は 204 → 続けて `OnBoot` GET → 挨拶「おっはよー！」… |
  | 12.759 | `ghost_switch_done`（ghost=emo2 attempt=Target） |
  | 22:03:07.453 | 自動終了 `app_exit` origin=Smoke |

- 終了コード: **0**（60.31 秒）。走行後の記憶: `[last] ghost = "emo2"`
- 目印の件数: `ghost_switch_requested` 1・`change_accepted` 1・`change_talk_start` 1・`change_talk_done_unload` 1・`talk_done_quit` 0・`change_cancelled` 0・`ghost_switch_down_ms` 1（ms=1）・`windows_closed_for_restart` 1・`ghost_switch_booted` 1・`ghost_switch_done`（info）1・`ghost_switch_cancelled` 0・`ghost_switch_target_fault` 0・`app_exit` 1（Smoke のみ）・`ghost_quit` 0・`alert` 0・`OnGhostChanging` 1・`OnGhostChanged` 1・`OnBoot` 2（起動時と、`OnGhostChanged` の 204 の後）・`OnFirstBoot` 0・ERROR 行 0・パニック 0
- 判定（要件 10.11 ⑤）: **合格**。① のあと終了して argv なしで起動すると、記憶の経路（route=Memory）で R_POST が出た。
- 判定（要件 10.11 ②）: **合格**。R_POST の `OnGhostChanging` の台本（「チェインジ！」）が流れてから降ろされ、emo2 が同じプロセスで起きた。`OnGhostChanged` の Ref1 に R_POST の送り出しの台本がそのまま載り、204 で `OnBoot` へ続いた（要件 4.1・4.2）。① と合わせて emo2 → R_POST → emo2 の往復。

## 走行 ③: ① の送り出しの台詞の途中でバルーンをダブルクリック → 中止

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run3 -Profile C:\tmp\areka-signoff-gsw\prof3 -SmokeMs 45000
```

- ① と同じく emo2 で起き、メニューで「Ｒポストと狛犬」を選んだ。`change_talk_start` を記録で見てから約 0.4 秒後、エモ（スコープ 1）のバルーン窓の不透明な点を `SendInput` で左ダブルクリックした（07:03:24.985 JST）
- 流れ（時刻は UTC・秒）:

  | 時刻 | 事象 |
  |---|---|
  | 22:03:24.472 | `ghost_switch_requested`・`change_accepted`・`OnGhostChanging`（204）→ `change_close_begin` → `OnClose` GET |
  | 24.475 | `change_talk_start`（phase=ChangeCloseTalkWait）＝「そろそろお時間だね。」 |
  | 25.216 | `talk_done_interrupted_as_non_quit`（台詞が利用者の中断で止まった） |
  | 25.217 | `change_cancelled` reason=user_break phase=ChangeCloseTalkWait |
  | 25.225 | `ghost_switch_cancelled` reason=UserBreak |
  | 以降 | emo2 の定常が続く（中止の後に `OnSecondChange` 39 件・22:03:49 に自発の台詞 1 件） |
  | 22:04:04.174 | 自動終了 `app_exit` origin=Smoke |

- 終了コード: **0**（45.67 秒）
- 目印の件数: `ghost_switch_requested` 1・`change_talk_start` 1・`change_cancelled` 1・`ghost_switch_cancelled` 1・`ghost_switch_down_ms` **0**・`windows_closed_for_restart` **0**・`ghost_switch_booted` **0**・`ghost_switch_done`（info）**0**・`OnGhostChanged` 0・`unload_clean` 1（自動終了のときだけ）・`talk_done_quit` 0・`app_exit` 1（Smoke のみ）・`ghost_quit` 0・`alert` 0・窓を開いた回数 1（起動時だけ）・ERROR 行 0・パニック 0
- 判定（要件 10.11 ③）: **合格**。送り出しの台詞（`OnGhostChanging` 204 の後の `OnClose` の台詞）をバルーンのダブルクリックで止めると切替は中止され、降ろさず（降ろした記録 0・窓の作り直し 0）、emo2 が定常のまま残った。

## 走行 ④: R_POST → SHIORI が失敗するゴースト → 既定ゴースト emo2 が Ref6/7 つきで起きる

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run4 -Profile C:\tmp\areka-signoff-gsw\prof4 -SmokeMs 60000 -LoaduFail
```

（`-LoaduFail` は `HOST32_TESTDLL_LOADU_FAIL=1` を付ける。emo2 は pasta.dll、R_POST は satori.dll なので影響を受けない）

- emo2 で起き、メニューで「Ｒポストと狛犬」へ（① と同じ流れ）。R_POST の挨拶（`OnGhostChanged` の台本）が終わってから、R_POST のキャラ窓のメニューで「fail-one」を選んだ
- 流れ（時刻は UTC・秒）:

  | 時刻 | 事象 |
  |---|---|
  | 22:04:19.988 | 1 回目の切替（emo2 → R_POST）: `ghost_switch_requested`・`OnGhostChanging`（204）→ `OnClose` の台詞 → `talk_done_quit` 24.405 |
  | 24.467 | `ghost_switch_down_ms` **ms=1**・`windows_closed_for_restart` 24.470 |
  | 24.601 | `ghost_switch_booted`（R_POST・Target）・窓 24.637・`OnGhostChanged` 24.686（Ref0＝むらさき 他 ① と同じ）→ 交代の台詞「……おや？」「ん。」・`ghost_switch_done` 24.691 |
  | 22:04:27.751 | 2 回目の切替（R_POST → fail-one）: `ghost_switch_requested`（from=Ｒポストと狛犬 to=fail-one）・`change_accepted` |
  | 27.752 | `OnGhostChanging` GET: `["", "manual", "fail-one", "C:\tmp\areka-signoff-gsw\root\ghost\fail1"]`（fail-one は `sakura.name` を持たないので Ref0 は空）→ 「返信！」「違うだろ。」 |
  | 28.255 | `change_talk_done_unload`（reason=Ended）→ `unload_clean` 28.266 |
  | 28.274 | `ghost_switch_down_ms` **ms=1**・`windows_closed_for_restart` 28.277 |
  | 28.854 | `last_used_recorded` ghost=fail1・`ghost_switch_booted`（fail1・Target） |
  | 28.884 | fail-one の窓が出た（scopes=[0, 1]） |
  | 28.915 | `connect_failed`（LOAD が ack [1] を返さない）・`shiori_failed` |
  | 28.927 | `ghost_switch_target_fault`（ghost=fail-one）＝既定ゴーストへ戻す |
  | 28.930 | `ghost_switch_down_ms` **ms=1**・`windows_closed_for_restart` 28.934（fail-one の窓 4 枚を閉じた） |
  | 29.391 | `last_used_recorded` ghost=emo2・`ghost_switch_booted`（emo2・**attempt=Default**） |
  | 29.402 | emo2 の窓が出た |
  | 29.619 | `OnBoot` GET: `["「コンフィズリー」＆「City-Pop'n」", "", "", "", "", "", "halt", "fail-one"]` |
  | 29.623 | `ghost_switch_done`（ghost=emo2 attempt=Default） |
  | 22:05:14.705 | 自動終了 `app_exit` origin=Smoke |

- 終了コード: **0**（60.65 秒）。走行後の記憶: `[last] ghost = "emo2"`
- 目印の件数:

  | 目印 | 件数 |
  |---|---|
  | `ghost_switch_requested` | 2 |
  | `change_accepted`／`change_talk_start` | 2／2 |
  | `change_talk_done_unload`／`talk_done_quit` | 1（R_POST の送り出し）／1（emo2 の送り出し） |
  | `ghost_switch_down_ms` | 3（ms=1・1・1） |
  | `windows_closed_for_restart` | 3 |
  | `ghost_switch_booted` | 3（Target・Target・Default） |
  | `ghost_switch_done`（切替を終える info） | 2（R_POST・emo2 の Default） |
  | `ghost_switch_target_fault` | 1 |
  | `ghost_switch_default_fault`／`ghost_switch_fatal` | 0／0 |
  | `ghost_windows_stale` | 0 |
  | `connect_failed`／`shiori_failed` | 1／1 |
  | `OnGhostChanged` | 1（R_POST へ切り替えたときだけ。既定へ戻したときは 0） |
  | `OnBoot`（Ref6＝halt・Ref7＝fail-one） | 1 |
  | `app_exit` | 1（origin=Smoke のみ） |
  | `ghost_quit`／`alert` | 0／0 |
  | ERROR 行 | 3（`connect_failed`・`shiori_failed`・`ghost_switch_target_fault` の各 1。いずれも意図した失敗） |
  | パニック | 0 |

- 判定（要件 10.11 ④）: **合格**。R_POST から SHIORI が失敗するゴーストへ切り替えると、失敗は切替先の窓が出た後に非同期で届き（`connect_failed` は窓の 30 ms 後）、`ghost_switch_target_fault` で既定ゴースト emo2 へ戻された。emo2 は `OnGhostChanged` を受けず `OnBoot` で起き、その Ref6＝`halt`・Ref7＝`fail-one`（落ちたゴーストの名前）だった。呼び出し元の R_POST ではなく既定の emo2 が起きた。告知は 0 件、終了の指示は自動終了の 1 件だけ。
- 判定（要件 4.9）: **合格**。`sakura.name`／`kero.name` を持たない 1 キャラのゴースト（fail-one）でも落ちず、窓は 2 人のゴーストと同じスコープ 0 と 1（scopes=[0, 1]）で作られ、既定へ戻すときに 4 枚とも閉じられた。

## 止まった時間（要件 3.8）

`ghost_switch_down_ms` は `GhostSession::shutdown` の所要時間。どの切替でも、送り出しの台詞の終わりで kanade が先に SHIORI を解放している（`unload_clean`）ため、降ろす処理は「kanade は既に止まっている」（`ghost-shutdown` の debug）を確かめるだけになり **1 ms** だった。

「降ろし始めから切替先の窓が出るまで」（`ghost_switch_down_ms` の記録時刻から所要 ms を引いた時刻 → 「本物のゴースト窓を開きました」の時刻）も記録から求めた。この区間は UI スレッドで同期に進む（降ろす → 窓を閉じる → 切替先の構成を読み、シェルとバルーンを焼き、kanade を起こす → 窓を作る）。

| 切替 | `ghost_switch_down_ms` | 降ろし始め → 切替先の窓 | 台詞の終わり（または失敗の到着＝`ghost_switch_target_fault` の時刻）→ 切替先の窓 |
|---|---|---|---|
| ① emo2 → R_POST | 1 ms | 180 ms | 236 ms |
| ② R_POST → emo2 | 1 ms | 496 ms | 506 ms |
| ④ emo2 → R_POST | 1 ms | 171 ms | 232 ms |
| ④ R_POST → fail-one | 1 ms | 611 ms | 629 ms |
| ④ fail-one → emo2（既定へ戻す） | 1 ms | 472 ms | 475 ms（`connect_failed` の時刻からは 487 ms） |

- **すべて 1 秒以内**。設計討議へ上げる議題は無い。
- 最大は emo2 のシェルを焼く切替（fail-one は emo2 のシェルの複製）で、時間の大半は `windows_closed_for_restart` から `ghost_switch_booted` まで（構成の読み取りと焼き）。降ろす処理そのもの（kanade・dispatcher・SHIORI・relay・ticker・sylphya・seriko の join）は 1 ms で支配していない。

## まとめ

| 走行 | 終了コード | 切替 | 目印（主なもの） | 判定 |
|---|---|---|---|---|
| ① emo2 → R_POST | 0（自動終了） | 成立 | requested 1・down_ms 1（1 ms）・windows_closed 1・booted 1・done 1・`OnGhostChanged` 1・app_exit 1（Smoke） | 合格 |
| ⑤ 再起動で R_POST | 0（自動終了） | —（route=Memory） | `ghost_resolved route=Memory`（R_POST） | 合格 |
| ② R_POST → emo2 | （⑤ と同じ走行） | 成立 | requested 1・change_talk_done_unload 1・down_ms 1（1 ms）・booted 1・done 1・`OnGhostChanged` 1（Ref1＝送り出しの台本）→ 204 → `OnBoot` | 合格 |
| ③ 台詞の途中でダブルクリック | 0（自動終了） | 中止 | change_cancelled 1（user_break）・ghost_switch_cancelled 1・down_ms 0・windows_closed 0・booted 0 | 合格 |
| ④ R_POST → fail-one → emo2 | 0（自動終了） | 既定へ戻す | target_fault 1・booted 3（最後が Default）・`OnBoot` Ref6＝halt・Ref7＝fail-one・`OnGhostChanged`（戻し）0・alert 0 | 合格 |

- 要件 10.11: 5 走行とも**合格**。どの走行でも切替の経路から終了の指示は出ず（`app_exit` は自動終了の 1 件だけ）、`ghost_quit`（停止通知からの終了）は 0 件、パニックは 0 件、プロセスは残らなかった。
- 要件 10.12: 5 走行のコマンド・終了コード・目印の件数・止まった時間を本書に残した。
- 要件 3.8: 降ろし始めから切替先の窓まで最大 611 ms（1 秒以内）。
- 要件 4.9: 1 キャラのゴーストで落ちない（④）。

## 気付いたこと

1. **送り出しの台詞が `\-` で終わる経路の記録の名前**: emo2 の `OnGhostChanging` は 204 で、続く `OnClose` の別れの台詞は `\-` で終わる。このとき kanade の記録は `change_talk_done_unload` ではなく、既存の `talk_done_quit`（「reason=Quit——終了系列（Quit）へ」）になる（① と ④ の 1 回目）。切替の中身は停止通知に載ったまま届き、areka は「切替による停止」として降ろして切替先を起こした（`app_exit`・`ghost_quit` は 0）。要件 2.3〜2.5・衝突表 1 の「`\-` 入りでも終了ではなく降ろすことで終わる」どおりの振る舞いで、記録の読み手は `change_talk_done_unload` と `talk_done_quit` の両方を「送り出しの台詞の終わり」として数える必要がある。
2. **失敗するゴーストの名前が一度だけ「最後に使ったゴースト」に書かれる**: ④ で fail1 を起こした時点（起動の呼び出しが返った時点）で `last_used_recorded ghost=fail1` が書かれ、SHIORI の失敗が届いて既定へ戻した 537 ms 後に `ghost=emo2` で上書きされた（走行後の記憶は emo2）。要件 4.6 の「起動に成功したら書く」の「成功」を起動の呼び出しの成功と読んだ振る舞い（design の `on_boot_ok`）で、要件 6.2 の最終状態（既定ゴーストに書き換わる）も満たす＝要件違反ではない。ただしこの間にプロセスが異常終了すると（強制終了・クラッシュ・電源断。LOAD の応答が遅いゴーストでは間が延びる）、次の起動で壊れたゴーストがまた起き、単独起動の失敗の経路で告知と終了コード 1 が 1 回出て、その次の起動で emo2 が Ref6/7 付きで起きる。**開発者判断の議題**: 切替先の記憶を書く時点を定常到達（`ghost_switch_done`）へ移すか（要件 3.5 が「SHIORI の失敗は起動の呼び出しが返った後に非同期で届く」とするのと同じ理屈）。
3. 里々の「他のゴーストから変更」は 2 つの台本から無作為に選ぶ。① では「いらっしゃませ。…なんだよ。」、④ の 1 回目では「……おや？」「ん。」が出た。どちらも台詞の字面に「むらさき」を綴らないので、「むらさきから交代」という字面そのものは画面に出ていない。`OnGhostChanged` の Ref0＝むらさき が送られていることは記録で確かめた。
4. （記録の審査で気付いた端のケース・優先度は低い）既定ゴースト自身の起動が同期に失敗して致命（`GhostFallbackFailed`）で終わるときは、既定ゴーストの `on_boot_ok` が走らないので、記憶は 2. の形で書かれた壊れた切替先のまま残る（`should_record_halt` は致命では書き換えない前提＝既定の起動が記憶を書き済み、が成り立たない）。配布物の同梱の既定ゴーストが壊れているときに限る。2. の議題と一緒に扱うとよい。
5. WARN は検体と既定バルーンに由来するもの（emo2 の全透明の要素 `purple/a/null.png`、StayseeBalloon のスコープ 1 の面の縮退、折返し基準）と、自動終了の `force_quit` だけだった。

## 本走行の前に行ったこと

- 手順の確かめの走行 `probe`（`prof-probe`・自動終了 25 秒・操作なし）: emo2 が初めて起きる走行で、`OnFirstBoot` の挨拶が流れ、emo2 の起動記録がここで書かれた。以後の走行の emo2 は起動記録のあるゴースト（起動は `OnBoot`）。
- 1 回目の ① の試み（記録は `logs\try1-nosendinput.*`）: エージェントの道具の不具合（`SendInput` に渡す構造体の大きさの誤り）で右クリックが届かず、切替は起きなかった。自動終了で終わった（終了コード 0）。道具を直してから記憶の置き場を作り直し、① をやり直した。areka 側の欠陥ではない。

## 開発者が自分の目で見直す手順

1. i686 の成果物を建て（`cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc`）、`cargo build -p areka`。上の「検体」の形で `C:\tmp\` の下に根を組む（areka.exe と i686 の helper を根の直下へ）。
2. PowerShell で `$env:AREKA_PROFILE_DIR='<空のフォルダ>'; Remove-Item Env:AREKA_ROOT -ErrorAction SilentlyContinue` として、根の `areka.exe` を argv なしで起動する。emo2 が出たら右クリック →「ゴースト」→「Ｒポストと狛犬」。emo2 の別れの台詞のあと R_POST が交代の台詞で起きることを見る（①）。
3. R_POST を右クリック →「ゴースト」→「えも？？」で emo2 へ戻る（②）。もう一度「Ｒポストと狛犬」を選び、emo2 の別れの台詞の途中でバルーンをダブルクリックすると、切り替わらずに emo2 が残る（③）。
4. R_POST にしてからメニューの「終了」で閉じ、同じ `AREKA_PROFILE_DIR` で argv なしに起動し直すと R_POST が出る（⑤）。
5. `$env:HOST32_TESTDLL_LOADU_FAIL='1'` を付けて起動し、R_POST から「fail-one」を選ぶと、一瞬 fail-one の窓が出てから emo2 が起きる（④）。`$env:RUST_LOG='info,kanade=trace'` で起動すれば、`OnBoot` の送出の記録の Reference の 7 番目と 8 番目に `halt` と `fail-one` が見える。
