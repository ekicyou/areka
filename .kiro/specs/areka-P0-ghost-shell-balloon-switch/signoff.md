# 実機確認の記録（要件 3.8・4.9・10.11・10.12・12.13）

> 走行 ①〜⑤ は 2026-09-27 07:00 の記録（HEAD `1a41da42`）。裁定 13（要件 12）の走行 ⑥〜⑨ は末尾の「2026-09-27 追記」（HEAD `7a496e14`）。

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
   - **解消（2026-09-27・裁定 13・要件 12.6〜12.7）**: 切替で降ろした直後に `LastGhost`＝既定・印＝切替先を書き（`switch_drop_recorded`）、切替先の `LastGhost` は定常到達まで書かない（`last_used_deferred`）形に改めた。実機では末尾の走行 ⑦ で、fail-one の迎え入れの途中に強制終了した時点の記憶が `ghost = "emo2"`・`running = "fail-one"` で、次の起動が emo2＋Ref6＝`halt`・Ref7＝`fail-one` になることを確かめた（壊れたゴーストの名前は `LastGhost` に一度も載らない）。
3. 里々の「他のゴーストから変更」は 2 つの台本から無作為に選ぶ。① では「いらっしゃませ。…なんだよ。」、④ の 1 回目では「……おや？」「ん。」が出た。どちらも台詞の字面に「むらさき」を綴らないので、「むらさきから交代」という字面そのものは画面に出ていない。`OnGhostChanged` の Ref0＝むらさき が送られていることは記録で確かめた。
4. （記録の審査で気付いた端のケース・優先度は低い）既定ゴースト自身の起動が同期に失敗して致命（`GhostFallbackFailed`）で終わるときは、既定ゴーストの `on_boot_ok` が走らないので、記憶は 2. の形で書かれた壊れた切替先のまま残る（`should_record_halt` は致命では書き換えない前提＝既定の起動が記憶を書き済み、が成り立たない）。配布物の同梱の既定ゴーストが壊れているときに限る。2. の議題と一緒に扱うとよい。
   - **解消（2026-09-27・裁定 13・要件 12.8）**: 降ろした直後に `LastGhost` は既定になっており、既定への戻しも失敗した致命では記憶を `LastGhost`＝既定・印＝切替先のまま残す（`should_record_halt` は起動中の印へ置き換えた）。この端のケースは実機で既定ゴーストを壊して起こしていない（決定論テスト 要件 12.12 ⑸ で固定）。実機では同じ「戻しの途中で落ちる」形を末尾の走行 ⑦ の試み 2（既定 emo2 の迎え入れの途中で強制終了）で見て、次の起動が Ref7＝`fail-one` になった。
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

---

## 2026-09-27 追記: 走行 ⑥〜⑨（要件 12.13・裁定 13）

- 日時: 2026-09-27 12:35〜12:48 JST（ログの時刻は UTC で 03:35〜03:48）
- HEAD: `7a496e14`（ブランチ `claude/areka-p0-ghost-shell-balloon-871462`・作業ツリーに差分なし）
- 実行体: `cargo build -p areka -j 4` を HEAD で回し（areka だけ建て直し）、もう一度回して「建て直し無し」を確かめてから根へ複製した。i686 の helper と `shiori_loadu.dll` も `cargo build -p shiori-host32-helper -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` が「建て直し無し」で、根の複製と一致

  | ファイル | SHA-256（`target\` の元と根の複製で一致） |
  |---|---|
  | `areka.exe`（x64 debug） | `92877D0F8C6E3592C266016185C82A0432831A78E744A537BF4791378A577AC6` |
  | `shiori-host32-helper.exe`（i686） | `8905FE98D6182F67D43A14B4D12E1C0027427FAA1230CDC4C6457EE598327365` |
  | `ghost\fail1\ghost\master\shiori_loadu.dll`（i686） | `F47D40FDB834A11A1F3049206D63EB31C26E16617C2B4B4DD325B346315993C5` |

### 検体と設定（前回からの差分だけ）

- 根は前回と同じ `C:\tmp\areka-signoff-gsw\root\`。argv なしで起動（`launch.ps1`）。`AREKA_NO_ALERT=1`・`NO_COLOR=1` も同じ
- 検体を 1 つ足した: `ghost\rpost_auto\`＝R_POST_and_KOMAINU の丸ごとの複製で、`descript.txt` の `name` を `Ｒポスト自動切替` に、`dic02_Event.txt` の `＊OnBoot` の本文を `：\![change,ghost,fail-one]` の 1 行に差し替えたもの（起動の挨拶の台本がそのまま fail-one への切替を出す。走行 ⑦ だけで使う・理由は ⑦ に書く）
- `RUST_LOG` に wintf の窓の手続きの記録を足した: `info,kanade=trace,areka_kanade=trace,areka=debug,areka_emo_text=info,areka_emo_present=info,areka_ghost=debug,ghost-shutdown=debug,ghost-boot=debug,shiori-actor=debug,wintf::ecs::window_proc::lifecycle=debug`
  - 印の事象（`session_mark_*`・`switch_drop_recorded`・`last_used_*`）とセッションの終了の受け手（`os_session_end_begin`／`_again`／`_done`）は target `areka::*` の info／debug、wintf の `os_session_end`（info）と破棄済みの打ち切り `[despawn-skip]`（debug）は target `wintf::ecs::window_proc::lifecycle`、`OnClose`／`OnBoot` の送出は kanade の trace `shiori_request`（method と Reference つき）で拾える
- 記憶の置き場は走行ごとに新しいフォルダ（`prof6`〜`prof9`）。「次の起動」は同じフォルダで起動した。最初の起動を R_POST にするため、置き場の `sylphya.toml` を前回の ① の後と同じ中身（`[last] ghost = "R_POST_and_KOMAINU"`）で用意した（⑦ だけ `ghost = "rpost_auto"`）
- 有界: 強制終了しない走行は `AREKA_APP_SMOKE_EXIT_MS`（最初の起動 120000・次の起動 30000）で自分で終わる。強制終了は `logs\<走行>.pid.txt` に `launch.ps1` が残した、自分が起こした areka の PID だけに `taskkill /F /PID` を打った（打つ前に実行ファイルのパスが根の `areka.exe` であることを確かめた）
- 記録の数え方: `C:\tmp\areka-signoff-gsw\tools\summary.sh <走行>` が `event="…"` の鍵ごとに件数を数え、判定に使う行を並べる
- 走行ごとの記憶の中身は `logs\prof<N>-*.sylphya.toml` に写しを残した
- **UI の操作ができなかった**: この時間帯は画面がスクリーンセーバー（入力デスクトップが `Screen-saver`・`Mystify.scr`）で、`SendInput` が拒否され（Win32 エラー 5）、`GetCursorPos` も失敗した（開発者のスクリーンセーバーは止めていない）。このため ⑦ はメニューを使わない形に、⑨ はメニューの「終了」と同じ終了要求を窓へ送る形に代えた（各走行に書く）。⑥・⑧ は UI を要しない
- プロセス: 全走行の後に `areka.exe`・`shiori-host32-helper.exe` がどちらも 0 件であることを確かめた。強制終了した areka の子の helper は、親を落とすと自分で終わった（落とした 1〜2 秒後に存在しない）

### 走行 ⑥: R_POST の定常で強制終了 → 次の起動は emo2＋Ref6＝halt・Ref7＝Ｒポストと狛犬

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run6a -Profile C:\tmp\areka-signoff-gsw\prof6 -SmokeMs 120000   # 裏で起動
taskkill /F /PID 22576                                                                                                              # run6a.pid.txt の PID
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run6b -Profile C:\tmp\areka-signoff-gsw\prof6 -SmokeMs 30000
```

- 6a: `ghost_resolved route=Memory`（R_POST）→ 03:35:52.620 `session_mark_written ghost="Ｒポストと狛犬"` → `OnBoot ["master"]` → 挨拶の台詞が終わって定常（`steady_talk_done` 03:35:54.286）→ 03:36:03.94 頃に taskkill・最後の記録 04.082・プロセスの終了 04.154。**終了コード 1**（`taskkill /F`・13.32 秒）。後始末は走らないので `session_mark_cleared`／`session_mark_kept`／`app_exit`／`OnClose` はどれも 0 件
- 記憶: 起動前 `ghost = "R_POST_and_KOMAINU"` → 強制終了の後 `ghost = "R_POST_and_KOMAINU"`・`running = "Ｒポストと狛犬"`（印が残った）
- 6b（次の起動）の流れ（UTC）:

  | 時刻 | 事象 |
  |---|---|
  | 03:36:13.583 | `session_mark_found ghost=Ｒポストと狛犬` |
  | 13.584 | `ghost_resolved route=Default`（emo2。`LastGhost` の R_POST は読まない） |
  | 13.988 | `session_mark_written ghost="えも？？"`（印を起こすゴーストの名前へ書き換え） |
  | 14.220 | `last_used_recorded ghost="emo2"` |
  | 15.480 | `OnBoot` GET: `["「コンフィズリー」＆「City-Pop'n」", "", "", "", "", "", "halt", "Ｒポストと狛犬"]` |
  | 44.226 | 自動終了 `app_exit origin=Smoke` → `OnClose` NOTIFY `["user"]` |
  | 44.294 | `session_mark_cleared` |

- 6b の終了コード **0**（30.81 秒）。記憶: `ghost = "emo2"`・`running = ""`
- 6b の目印: `session_mark_found` 1・`session_mark_written` 1・`session_mark_cleared` 1・`session_mark_kept` 0・`OnBoot`（halt つき）1・`app_exit` 1（Smoke のみ）・`alert` 0・ERROR 0・パニック 0
- 判定（要件 12.13 ⑥・12.1・12.3・12.4）: **合格**。強制終了では後始末が走らず印が残り、次の起動は `LastGhost` を読まずに既定の emo2 を起こし、`OnBoot` の Ref6＝`halt`・Ref7＝落としたゴーストの名前（`descript.txt` の `name`）だった。告知は 0 件。印は起こしたゴーストの名前へ書き換わり、きれいな終わりで消えた。

### 走行 ⑦: 切替先 fail-one の迎え入れの途中で強制終了 → 次の起動は Ref7＝fail-one・`LastGhost` は fail-one でない

時間の窓の取り方: 切替先の失敗は窓が出た数十 ms 後に届く（前回の ④ で `ghost_switch_booted` → `ghost_switch_target_fault` が 73 ms）。記録を見てから `taskkill` では間に合わないので、**切替先の SHIORI を載せる helper を、現れた直後にエージェントが一時停止した**（`NtSuspendProcess`。helper は自分が起こした areka の子で、`switch_drop_recorded` の記録の後に初めて現れたものだけを止める＝`tools\suspend-target-helper.ps1`）。helper が止まると areka は HELLO を待ち、上限の 5 秒（試み 1 で `ghost_switch_booted` 03:43:33.666 → `connect_failed` 38.705）まで切替先の迎え入れ（`Welcoming{Target}`）に留まる。その間に `tools\kill-in-welcome.ps1` が `attempt=Target` の記録と失敗の記録 0 件を確かめてから強制終了した。

切替の起こし方: UI が使えないので、メニューの代わりに検体 `rpost_auto`（起動の挨拶の台本が `\![change,ghost,fail-one]`）を起こした。台本の切替は `origin=automatic`・`raise_event=false`（`OnGhostChanging` は送らず黙って降ろす）で、降ろしてから切替先を起こす以降の経路（`switch_to`＝要件 12.6 の書き込み・迎え入れ）はメニューの切替と同じ。`HOST32_TESTDLL_LOADU_FAIL=1`（`-LoaduFail`）は前回の ④ と同じく付けた（一時停止が外れた場合に失敗として見えるように）。

コマンド（PowerShell・3 つを並べて起動）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\suspend-target-helper.ps1 -Run run7a                                                 # 裏で見張る
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run7a -Profile C:\tmp\areka-signoff-gsw\prof7 -SmokeMs 120000 -LoaduFail   # 裏で起動
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\kill-in-welcome.ps1 -Run run7a                                                       # 迎え入れの途中で taskkill /F
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run7b -Profile C:\tmp\areka-signoff-gsw\prof7 -SmokeMs 30000
```

- 7a の流れ（UTC）:

  | 時刻 | 事象 |
  |---|---|
  | 03:46:11.570 | `session_mark_written ghost="Ｒポスト自動切替"`・11.673 `last_used_recorded ghost="rpost_auto"` |
  | 11.783 | `OnBoot ["master"]` → 台本の `\![change,ghost,fail-one]` |
  | 11.842 | `ghost_switch_requested`（to=fail-one・origin=automatic・raise_event=false）・`change_accepted` |
  | 11.860 | `unload_clean`（`OnClose` は送らない＝黙って降ろす）→ 11.891 `ghost_switch_down_ms` ms=3 |
  | 11.895 | **`switch_drop_recorded last_ghost="emo2" mark="fail-one"`** |
  | 11.903 | `windows_closed_for_restart` |
  | 13.042 | `last_used_deferred ghost=fail1`（切替先の記憶は定常到達まで書かない）・`ghost_switch_booted ghost=fail1 attempt=Target` |
  | 13.070 | fail-one の helper（pid 20856）を一時停止 |
  | 14.229 | 強制終了（それまでの `connect_failed`／`ghost_switch_target_fault` は 0 件） |

- 7a の終了コード **1**（`taskkill /F`・3.04 秒）。一時停止した helper は親を落とした直後に消えていた
- 記憶: 起動前 `ghost = "rpost_auto"` → 強制終了の時点 **`ghost = "emo2"`・`running = "fail-one"`**（`LastGhost` は一度も fail1 にならない）
- 7a の目印: `switch_drop_recorded` 1・`last_used_recorded` 1（rpost_auto の起動時だけ）・`last_used_deferred` 1・`ghost_switch_booted` 1（Target）・`ghost_switch_target_fault` 0・`connect_failed` 0・`session_mark_cleared`／`kept` 0／0・ERROR 0
- 7b（次の起動）: 03:46:41.682 `session_mark_found ghost=fail-one` → `ghost_resolved route=Default`（emo2）→ 41.940 `session_mark_written ghost="えも？？"` → 42.166 `last_used_recorded ghost="emo2"` → 42.481 `OnBoot` GET `["「コンフィズリー」＆「City-Pop'n」", "", "", "", "", "", "halt", "fail-one"]` → 自動終了 → `OnClose` NOTIFY `["user"]` → 03:47:12.248 `session_mark_cleared`。終了コード **0**（30.66 秒）。記憶 `ghost = "emo2"`・`running = ""`。`alert` 0・ERROR 0
- 判定（要件 12.13 ⑦・12.6・12.7）: **合格**。前のゴーストを降ろした直後に `LastGhost`＝既定・印＝切替先が 1 回で書かれ、切替先の `LastGhost` は書かれない（`last_used_deferred`）。迎え入れの途中で落ちると、次の起動は emo2 で Ref7＝切替先の名前（fail-one）になり、壊れた切替先はまた起きない（「気付いたこと」2 の解消）。

⑦ の前の試み（記録は残した・判定には使っていない走行も含めて書く）:

- 試み 1（`logs\run7-try1.*`）: 一時停止は正しく fail-one の helper に掛かったが、強制終了を別の手順で打ったため HELLO の上限 5 秒に間に合わなかった。そのまま**記憶の時系列の実例**になった: `switch_drop_recorded last_ghost=emo2 mark=fail-one` → `last_used_deferred ghost=fail1` → `ghost_switch_booted`（Target）→ 5.04 秒後 `connect_failed`（HELLO を受領できなかった）→ `ghost_switch_target_fault` → `last_used_recorded ghost=emo2`・`ghost_switch_booted`（Default）→ `OnBoot` Ref6＝halt・Ref7＝fail-one → `last_used_recorded ghost=emo2`・`session_mark_steady ghost=えも？？` → `ghost_switch_done`（Default）。前回の ④ にあった `last_used_recorded ghost=fail1`（「気付いたこと」2）は出ない。その後 emo2 の定常で強制終了（終了コード 1）
- 試み 2（`logs\run7-try2.*`・次の起動 `run7-try2b`）: 見張りの開始が遅れて起動したゴーストの helper を見逃し、1 つ目に見えた fail-one の helper を止めずに、既定へ戻すときの **emo2 の helper を止めた**（見張りを `switch_drop_recorded` の後に現れた helper を止める形へ直したのが上の 7a）。fail-one は LOAD の失敗（`connect_failed`）で既定へ戻され、その**既定の迎え入れ（`Welcoming{Default}`）の途中**で強制終了した（終了コード 1）。強制終了の時点の記憶は `ghost = "emo2"`・`running = "fail-one"`、次の起動（終了コード 0）は `session_mark_found ghost=fail-one` → emo2 の `OnBoot` Ref6＝halt・Ref7＝fail-one → `session_mark_cleared`。要件 12.7 の「既定への戻しの途中で落ちても Ref7＝切替先」の実例
- 試み 1 だけ、既定へ戻した emo2 の窓で `chain_finalize: 初期配置の確定が続けて見送られている`（deferrals=600・scope 0）の WARN が 1 件出た（切替で起こしたゴーストの初期配置の欠陥の副作用・下の「気付いたこと」6）

### 走行 ⑧: OS のセッションの終了の再現（`WM_QUERYENDSESSION` → `WM_ENDSESSION`（TRUE））

OS のシャットダウンは開発機で行えないので、OS が各トップレベル窓へ送るのと同じ形を `tools\endsession.ps1` で再現した: 定常の R_POST の PID のトップレベル窓を `EnumWindows`＋`GetWindowThreadProcessId` で全部（見えない窓も）列挙し、各窓へ `SendMessageTimeout`（`SMTO_NORMAL`）で `WM_QUERYENDSESSION`（wParam＝0・lParam＝0・上限 5 秒）→ 全窓が TRUE を返したことを確かめてから、各窓へ順に `WM_ENDSESSION`（wParam＝TRUE・lParam＝0・上限 15 秒）。

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run8a -Profile C:\tmp\areka-signoff-gsw\prof8 -SmokeMs 120000   # 裏で起動
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\endsession.ps1 -ProcId 21828 -Out C:\tmp\areka-signoff-gsw\logs\run8a.endsession.txt
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run8b -Profile C:\tmp\areka-signoff-gsw\prof8 -SmokeMs 30000
```

- 送った側の記録（`run8a.endsession.txt`・UTC）: トップレベル窓は 5 枚（`wintf-winmsg-executor` の 狛犬 ×2・ポスト ×2、見えない `IME`「Default IME」×1）

  | 窓 | `WM_QUERYENDSESSION` | `WM_ENDSESSION`（TRUE） |
  |---|---|---|
  | `0xC0CAA` 狛犬（バルーン） | ret=1・result=1（TRUE） | **ret=1・result=0・34 ms**（受け手が走った） |
  | `0xA0CAE` 狛犬 | TRUE | ret=1・result=0・4 ms（下の注） |
  | `0xB0D14` ポスト | TRUE | 送信の失敗 ret=0・`ERROR_INVALID_WINDOW_HANDLE`（1400）＝窓は既に壊れていた |
  | `0x3F099A` ポスト | TRUE | 送信の失敗（1400） |
  | `0x3409CE` Default IME | TRUE | 送信の失敗（1400） |

- areka の記録（UTC）:

  | 時刻 | 事象 |
  |---|---|
  | 03:39:29.093 | wintf `os_session_end`（entity=24v0・lparam=0）→ `os_session_end_begin` |
  | 29.098 | `app_exit origin=SessionEnd closed=4` |
  | 29.099 | kanade `force_quit reason="system"` → `OnClose` **NOTIFY `["system"]`** |
  | 29.109 | `unload_clean`（helper 正常終了）→ 29.111 `persist flush confirmed`・`ghost shutdown sequence completed` |
  | 29.115 | `session_mark_cleared` |
  | 29.116 | **`os_session_end_done ms=22 down_ok=true`** |
  | 29.116〜29.117 | `quit_app` の窓の破棄が投函した `WM_CLOSE` 4 件は `[despawn-skip] WM_CLOSE`（破棄済みの打ち切り） |
  | 29.118 | `[WinApp::run] exit requested while windows remained open — destroying them before returning` |
  | 29.132 | `session_end_already_handled first=Some(SessionEnd)`（告知も印の判定もしない）・`ghost_slot_empty` |

- 8a の終了コード **0**（5.39 秒・`WM_ENDSESSION` の後にプロセスが自分で終わった）。記憶: 送る前 `ghost = "R_POST_and_KOMAINU"`・`running = "Ｒポストと狛犬"` → 後 `running = ""`
- 8a の目印: `os_session_end`（wintf）1・`os_session_end_begin` **1**・`os_session_end_again` **0**・`os_session_end_done` 1（ms=22）・`os_session_end_world_busy` 0・`os_session_end_down_failed` 0・`OnClose`（Ref0＝system・NOTIFY）**1**・`session_mark_cleared` **1**・`session_mark_kept` 0・`app_exit` 1（SessionEnd のみ）・`ghost_quit` 0・`alert` 0・ERROR 0・パニック 0
- 残りの窓への `WM_ENDSESSION`: 3 枚は送信の失敗（窓が既に壊れていた）。2 枚目（`0xA0CAE`）だけ `SendMessageTimeout` が 4 ms で成功（ret=1・result=0）を返したが、areka・wintf 側にこのメッセージの記録が 1 件も無い（`[despawn-skip] WM_ENDSESSION` の debug も、World の借用中の `os_session_end_world_busy` の warn も、「関数を持たない窓」の debug も 0）。送った時刻（29.119〜29.123）は、終了の指示を受けた wintf が World を借りたまま残りの窓を壊している最中（29.118〜）だった。窓の手続きへ配られる前に窓が壊され、OS が保留の送信を 0 で完了させたと推定する（手続きが呼ばれていれば借用中の warn が出るはずなので）。どの形でも受け手の処理は 1 回だけ（`os_session_end_begin` 1・`os_session_end_again` 0）で、要件 12.10 の意図（2 通目以降で後始末を繰り返さない）は満たす。要件の字面の 2 形（破棄済みの打ち切り／送信の失敗）に「送信の待ちの間に窓が壊れて 0 で返る」が加わる形（「気付いたこと」7）
- 8b（次の起動）: `ghost_resolved route=Memory`（R_POST・記憶どおり）→ `session_mark_written ghost="Ｒポストと狛犬"` → `OnBoot` GET `["master"]`（**Ref6/7 なし**）→ 自動終了 → `OnClose` NOTIFY `["user"]` → `session_mark_cleared`。`session_mark_found` 0。終了コード **0**（30.29 秒）。記憶 `ghost = "R_POST_and_KOMAINU"`・`running = ""`
- 所要: 受け手の処理（`os_session_end_begin` → `os_session_end_done`）は **22 ms**、1 通目の `WM_ENDSESSION` の送信の往復は 34 ms。OS の既定の猶予（数秒）より 2 桁小さいので、設計討議へ上げる議題は無い
- 判定（要件 12.13 ⑧・12.9〜12.11）: **合格**。

### 走行 ⑨: 正常な終了のあとの起動は Ref6/7 なし・記憶どおりのゴースト

メニューの「終了」の代わり: UI が使えないので、R_POST の本体側のキャラ窓へ `PostMessage(WM_CLOSE)` を送った（Alt＋F4 と同じ OS の閉鎖要求）。areka はこれを `on_ghost_os_close`（`app_exit.rs`）で受け、メニューの「終了」の動作 `request_close`（`menu/mod.rs`）と**同じ** `MouseWiring::send_close_request(CloseReason::User { scope })` を kanade へ 1 件送る（窓は消さない）。kanade から先（`OnClose` の台詞 → `\-` → 終了系列 → `quit_app`）はメニューの「終了」と同一の経路。

コマンド（PowerShell）:

```
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run9a -Profile C:\tmp\areka-signoff-gsw\prof9 -SmokeMs 120000   # 裏で起動
# 定常に入ってから、題名「ポスト」の背の高い窓（0x160D42）へ PostMessage(hwnd, WM_CLOSE, 0, 0)
pwsh -NoProfile -File C:\tmp\areka-signoff-gsw\tools\launch.ps1 -Run run9b -Profile C:\tmp\areka-signoff-gsw\prof9 -SmokeMs 30000
```

- 9a の流れ（UTC）: 03:47:42.303 `session_mark_written ghost="Ｒポストと狛犬"` → 定常 → 43.565 `WM_CLOSE` を投函 → 43.572 wintf `os_close_request`（entity=23v0）→ 43.576 areka `os_close_request scope=0 kind="ghost"` → 43.577 `OnClose` GET `["user", "0", "0"]` → R_POST の別れの台詞 → 44.242 `talk_done_quit` → 44.257 `ghost_quit cause=Quit` → 44.263 `app_exit origin=KanadeStopped(Quit)` → 44.295 `session_mark_cleared`
- 9a の終了コード **0**（2.19 秒）。記憶: 前 `running = "Ｒポストと狛犬"` → 後 `ghost = "R_POST_and_KOMAINU"`・`running = ""`。目印: `session_mark_cleared` 1・`session_mark_kept` 0・`app_exit` 1・`ghost_quit` 1・`alert` 0・ERROR 0
- 9b（次の起動）: `ghost_resolved route=Memory`（R_POST）→ `OnBoot` GET `["master"]`（**Ref6/7 なし**）→ 自動終了 → `session_mark_cleared`。`session_mark_found` 0。終了コード **0**（30.32 秒）。記憶 `ghost = "R_POST_and_KOMAINU"`・`running = ""`
- 判定（要件 12.13 ⑨・12.2）: **合格**（終了の操作はメニューの「終了」と同じ終了要求を OS の閉鎖要求で送る形で代えた）。きれいな終わりで印が消え、次の起動は `LastGhost` どおり R_POST で、Ref6/7 は付かない。なお ⑥〜⑧ の次の起動（6b・7b・8b・9b）も自動終了（出所 Smoke＝きれいな終わり）で `session_mark_cleared` 1 件・記憶 `running = ""` で終わっている。

### 前回の記録の走行で使ったが判定に使わなかったもの

- `logs\try9-locked.*`: ⑨ の最初の試み。メニューを開く右クリックの `SendInput` がスクリーンセーバーの入力デスクトップで拒否され（エラー 5）、何も操作できなかった。自動終了（出所 Smoke・終了コード 0）で終わった。areka 側の欠陥ではない

### まとめ（⑥〜⑨）

| 走行 | 終了コード | 最初の起動の記憶の後 | 次の起動 | 判定 |
|---|---|---|---|---|
| ⑥ R_POST の定常で `taskkill /F` | 1（強制終了）→ 次 0 | `running = "Ｒポストと狛犬"` のまま | `session_mark_found` 1・route=Default（emo2）・`OnBoot` Ref6＝halt・Ref7＝Ｒポストと狛犬 | 合格 |
| ⑦ fail-one の迎え入れの途中で `taskkill /F` | 1（強制終了）→ 次 0 | `ghost = "emo2"`・`running = "fail-one"` | `session_mark_found` 1・emo2・Ref6＝halt・Ref7＝fail-one | 合格 |
| ⑧ `WM_QUERYENDSESSION` → `WM_ENDSESSION`（TRUE） | 0 → 次 0 | `running = ""`（`os_session_end_begin` 1・`_again` 0・`OnClose` Ref0＝system 1・`session_mark_cleared` 1・22 ms） | route=Memory（R_POST）・Ref6/7 なし | 合格 |
| ⑨ 終了要求（メニューの「終了」と同じ要求） | 0 → 次 0 | `running = ""`（`session_mark_cleared` 1） | route=Memory（R_POST）・Ref6/7 なし | 合格 |

- 要件 12.13: ⑥〜⑨ とも**合格**（⑦ はメニューでなく台本の切替と helper の一時停止で時間の窓を取った・⑨ はメニューの「終了」を同じ終了要求の OS の閉鎖要求で代えた＝UI の入力ができなかったため）。告知 0 件・パニック 0 件・プロセスは残らなかった。
- 要件 12.1〜12.11 のうち実機で通ったもの: 12.1（起こす前に印を書く・既定の定常で既定の名前へ書き換え）、12.2（きれいな終わりで消す）、12.3（強制終了で残る）、12.4（印が在れば `LastGhost` を読まず既定＋Ref6/7）、12.6（降ろした直後の 1 回の書き込み・切替先の `LastGhost` は定常到達まで書かない）、12.7（迎え入れと既定への戻しの途中で落ちても Ref7＝切替先）、12.9〜12.11（セッションの終了の受け手・1 回だけ・所要の記録・告知なし）。12.5（argv）と 12.8（既定も壊れた致命）は実機で起こしていない（決定論テスト 要件 12.12 ⑶⑸）。

### 気付いたこと（⑥〜⑨）

6. **切替・既定への戻しで起こしたゴーストでは、初期配置の連鎖の再解決が走らない（見て分かる欠陥・要件 4.10 違反・新しいタスク 11.9 で直す）**:
   - 根: 初期配置の確定の一発の印 `ChainFinalized` と見送りの数 `ChainFinalizeStall`（どちらも World 全体の資源・`crates/areka/src/placement/chain_finalize.rs`）を、起こし直しのために全窓を閉じる `close_windows_for_restart`（`crates/areka/src/app_exit.rs`）が取り除かない。初回の起動で一度確定すると印が残り続けるので、切替先（と既定への戻し）の窓では「実表示寸で連鎖を再解決」が一度も走らない。要件 4.10（切替先の窓を初回の起動と同じ手順で作る）に反する
   - 証拠（前回の記録）: 初回の起動の emo2 は `chain_finalize: 実表示寸で連鎖を再解決 scope=1 from_x=1340 to_x=1392` → `初期配置を確定`（run1・run4 の起動時）。これに対し run5（② R_POST → emo2）と run4（④ の R_POST への切替・fail-one → emo2 の既定への戻し）では、`ghost_switch_booted` の後の `chain_finalize` の記録が 0 件で、emo2 の 2 人目のキャラ窓は `char_x=1340` のまま（初回の起動の 1392 へ動かない）。DPI 192 の画面で 2 人のキャラの間に 52 px の隙間が開き、2 人目のバルーンもそれに付いてこない
   - ⑦ の試み 1 の `chain_finalize: 初期配置の確定が続けて見送られている（deferrals=600 scope=Some(0) reason=実表示寸が未確定）` の WARN は同じ根の副作用: rpost_auto が初期配置の確定より前に（起動の挨拶の台本で）切り替えたので確定の印が立たず、見送りの数だけが fail-one の窓（HELLO 待ちの 5 秒間表示されない）をまたいで引き継がれ、既定へ戻した emo2 の窓で上限に達した。この走行でだけ emo2 の初期配置が 2.2 秒後に確定したのも、印が立っていなかったため
   - 本書の要件 12.13 の判定（⑥〜⑨）は印と終了の経路だけを見ており、この欠陥の影響を受けない。修正はコントローラが足すタスク 11.9 で行う
7. **⑧ の残りの窓の 3 つ目の形**: 要件 12.10・design Flow 7 は残りの窓への `WM_ENDSESSION` を「破棄済みの打ち切り（`[despawn-skip]`）か送信の失敗」とするが、実機では「送信の待ちの間に窓が壊れて `SendMessageTimeout` が 0 で成功を返す（手続きへ配られない）」が 1 枚あった（上の ⑧）。受け手の処理が 1 回だけという振る舞いは変わらないので、コードの変更は要らない。字面を合わせるなら要件 12.10 と design の 2 形に「待ちの間に窓が壊れる」を足す
   - 根拠: `WinApp::run` の手順 4.5（`crates/wintf/src/runtime/mod.rs`・残存窓の破棄）は World を借りたまま残りの窓を壊す。その間に窓の手続きが呼ばれていれば、橋渡し（`wndproc_bridge.rs`）の World の借用中の腕が `os_session_end_world_busy` の warn を 1 件残すはずだが、その warn は 0 件だった＝メッセージが手続きへ配られる前に窓が壊された。害は無い
8. 所要（22 ms）は OS の猶予より十分小さい。11.3 の申し送り（受け手の中の `GhostSession::shutdown` の join は送られてきたメッセージを配らない＝受け手の処理中に別スレッドから UI の窓へ同期の送信が重なると止まりうる）は、この再現では 1 通ずつ返りを待って送ったので重なる形が起きておらず、確かめていない

### 開発者が自分の目で見直す手順（⑥〜⑨）

1. ⑥: `AREKA_PROFILE_DIR` を空のフォルダにして起動し、メニューで R_POST へ切り替えてから、タスクマネージャーの「タスクの終了」ではなく `taskkill /F /PID <areka の PID>` で落とす。同じ `AREKA_PROFILE_DIR` で起動すると emo2 が出る（`$env:RUST_LOG='info,kanade=trace'` なら `OnBoot` の Reference の末尾 2 つが `halt`・`Ｒポストと狛犬`）
2. ⑦: `$env:HOST32_TESTDLL_LOADU_FAIL='1'` で起動し、R_POST から「fail-one」を選んだ瞬間（fail-one の窓が一瞬出る間）に `taskkill /F` するのは人の手では難しい。記憶の中身（`<AREKA_PROFILE_DIR>\sylphya.toml` の `[last]`）が切替の途中で `ghost = "emo2"`・`running = "fail-one"` になることは、切替の直後に開けば見える
3. ⑧: Windows をシャットダウン（または サインアウト）し、次のサインインで areka を起動すると、前回のゴーストが Ref6/7 なしで出る（`running` が空）
4. ⑨: メニューの「終了」で閉じ、起動し直すと前回のゴーストが Ref6/7 なしで出る
