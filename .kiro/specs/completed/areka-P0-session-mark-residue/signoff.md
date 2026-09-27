# 実機確認の記録（要件 7.7）

- 日時: 2026-09-27 23:48〜23:55 JST（ログの時刻は UTC で 2026-09-27 14:48〜14:55）
- HEAD: `5ef27138`（ブランチ `claude/areka-p0-session-mark-residue-0e9e89`）
- 環境: Windows 11 Pro 10.0.26200
- 実行体: `target\debug\areka.exe`（x64 debug）。`cargo build -p areka -j 4` を HEAD で通してから複製した。i686 の helper は `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` の成果物。複製した 2 つのハッシュが `target\` の元と一致することを確かめた

## 検体（根は 1 つ・短い絶対パス）

emo2 は絶対パスかつ短いパスで起こす必要がある（相対だと pasta.dll の読み込みが 0x8007007E で失敗する）ので、根だけ短いパスに置いた。配布物と同じ形に、実行体を根の直下に置いた（`AREKA_ROOT` は外した）。

```
C:\tmp\areka-signoff-smr\root\
  areka.exe                   … target\debug\areka.exe の複製
  shiori-host32-helper.exe    … i686 の helper
  ghost\emo2\                 … `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が配った emo2
  ghost\R_POST_and_KOMAINU\   … C:\home\maz\tmp\loadu\ascii\R_POST_and_KOMAINU を丸ごと複製（SHIORI は里々・i686 の helper 経由）
  balloon\emo2-kakukaku\      … emo2 の同梱バルーン
  balloon\StayseeBalloon\     … 既定バルーン（`nar-sample-path -- StayseeBalloon` が配ったもの）
```

リポジトリの中のゴースト・バルーン・配布物には触れていない。壊したのは上の根の中の StayseeBalloon の複製だけで、走行の後に元へ戻した。

## 全走行に共通の設定

- 手順のスクリプトはセッションの作業フォルダ（以下 `<作業>`＝このセッションの scratchpad の `smr\`）に置いた。スクリプトは 3 つ:
  - `<作業>\tools\launch.ps1 -Run <走行> -Profile <記憶の置き場> -SmokeMs <ms>`: 環境変数を設定して areka を `Start-Process` で起こし、自分が起こした PID を `logs\<走行>.pid.txt` に残し、終わるまで待って終了コードと所要を `logs\<走行>.exit.txt` に残す。自動終了の時刻＋60 秒を過ぎても終わらなければ、その PID の実行ファイルが根の `areka.exe` であることを確かめてから止める（今回は一度も働いていない）
  - `<作業>\tools\endsession.ps1 -ProcId <PID> -Out <記録>`: OS のセッションの終了の再現。指定した PID のトップレベル窓を `EnumWindows`＋`GetWindowThreadProcessId` で全部（見えない窓も）列挙し、各窓へ `SendMessageTimeout`（`SMTO_NORMAL`）で `WM_QUERYENDSESSION`（wParam＝0・lParam＝0・上限 5 秒）→ 全窓が TRUE を返したことを確かめてから、各窓へ順に `WM_ENDSESSION`（wParam＝TRUE・lParam＝0・上限 15 秒）。前の spec（`areka-P0-ghost-shell-balloon-switch` の走行 ⑧）と同じもの
  - `<作業>\tools\suspend-helper.ps1 -ProcId <PID> -Out <記録>`: 指定した areka の子の `shiori-host32-helper.exe`（親 PID で絞る）がちょうど 1 つであることを確かめて、`NtSuspendProcess` で一時停止する（SHIORI が応答しない形を作る）
- 環境変数（`launch.ps1` が設定する）:
  - `NO_COLOR=1`・`AREKA_NO_ALERT=1`（告知が出ても窓で止まらないように。告知の記録 `event="alert"` は抑止でも 1 件残るので、0 件であることで「告知が出ない」を判定した）
  - `AREKA_APP_SMOKE_EXIT_MS`（走行ごとに下に記す）＝全走行が自分で終わる
  - `AREKA_PROFILE_DIR` は走行ごとに `<作業>` の下の新しいフォルダ（開発者のアプリの記憶は読まず・書かない）。次の起動は前の走行と同じフォルダを使う
  - `RUST_LOG=info,kanade=trace,areka_kanade=trace,areka=debug,areka_emo_text=info,areka_emo_present=info,areka_ghost=debug,ghost-shutdown=debug,ghost-boot=debug,shiori-actor=trace,wintf::ecs::window_proc::lifecycle=debug`
    - 印の事象（`session_mark_*`）・OS のセッションの終了の受け手（`os_session_end_begin`／`_again`／`_done`）・倒れた（`session_mark_pinned_by_fallback`）は target `areka::*` の info／warn／debug、上限切れ（`shiori_wait_cut`）は target `shiori-actor` の warn、`OnBoot`／`OnClose` の送出は kanade の trace `shiori_request`（method と Reference つき）で拾える。design の最低水準（`shiori-actor=warn,areka=info`）より広い
- プロセス: 各走行の後に `areka.exe`・`shiori-host32-helper.exe` がどちらも 0 件であることを `tasklist` で確かめた。止めたり一時停止したりしたのは、`launch.ps1` が残した自分の PID と、その子の helper だけ

## 走行 A: 定常の emo2 へ `WM_QUERYENDSESSION` → `WM_ENDSESSION`（TRUE）

コマンド（PowerShell）:

```
pwsh -NoProfile -File <作業>\tools\launch.ps1 -Run runA -Profile <作業>\profA -SmokeMs 120000   # 裏で起動
# 記録に steady_talk_done が出たら（挨拶が終わって定常）
pwsh -NoProfile -File <作業>\tools\endsession.ps1 -ProcId <runA の PID> -Out <作業>\logs\runA.endsession.txt
# 続けて次の起動
pwsh -NoProfile -File <作業>\tools\launch.ps1 -Run runA2 -Profile <作業>\profA -SmokeMs 20000
```

- 起動: `ghost_resolved route=Default`（emo2）→ `session_mark_written ghost="えも？？"` → `OnFirstBoot` → 挨拶が終わって定常（`steady_talk_done` 14:48:20.832）。送る前の記憶は `ghost = "emo2"`・`running = "えも？？"`
- 送信の記録（`runA.endsession.txt`）: トップレベル窓 5 枚（キャラ窓とバルーン窓 4 枚＋既定の IME 窓）。`WM_QUERYENDSESSION` は 5 枚とも TRUE。`WM_ENDSESSION` は 1 枚目が 75 ms で戻り、2 枚目と 4 枚目は 0〜9 ms で戻り（手続きへ配られた記録は無い）、3 枚目と 5 枚目は窓が既に無い（`lasterr=1400`）
- 記録の抜き書き（`runA.out.log`）:

  ```
  14:48:39.217317Z INFO wintf::ecs::window_proc::lifecycle: [WM_ENDSESSION] セッションの終了を利用側の関数へ渡す event="os_session_end" entity=24v0 lparam=0
  14:48:39.217983Z INFO areka::session_end: … event="os_session_end_begin"
  14:48:39.223888Z INFO areka::app_exit: … event="app_exit" origin=SessionEnd closed=4
  14:48:39.224983Z TRACE kanade: SHIORI 送出 event="shiori_request" method=NOTIFY id=OnClose references=["system"]
  14:48:39.274924Z INFO shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)） event="unload_clean"
  14:48:39.284156Z INFO areka::boot_resolve: きれいに終わったので起動中の印を消しました event="session_mark_cleared"
  14:48:39.284680Z INFO areka::session_end: … event="os_session_end_done" ms=66 down_ok=true shiori_cut=false
  14:48:39.310961Z INFO areka: [main] OS のセッションの終了で後始末は済んでいる——告知を出さず、起動中の印にも触れません event="session_end_already_handled" first=Some(SessionEnd)
  ```

- 終了コード **0**（38.62 秒・`WM_ENDSESSION` の後にプロセスが自分で終わった）。記憶: 後 `running = ""`
- 目印の件数: `os_session_end`（wintf）1・`os_session_end_begin` **1**・`os_session_end_again` 0・`os_session_end_done` 1（**ms=66・shiori_cut=false**）・`shiori_wait_cut` **0**・`OnClose`（Ref0＝system）1・`session_mark_cleared` **1**・`session_mark_kept` 0・`alert` 0・ERROR 0・パニック 0
  - `app_exit` は 2 件: 1 件目が出所 `SessionEnd`（窓 4 枚を閉じた）、2 件目は後から届いた kanade の停止通知（`Forced`）で `closed=0`。2 件目の直前に `app_exit_again`（最初の出所を残し、この出所は記録だけ）が出ており、最初の出所は `SessionEnd` のまま（`session_end_already_handled first=Some(SessionEnd)`）。`quit_app` は呼ばれるたびに `app_exit` を info で残す作りなので、害は無い
- 次の起動 A2: `ghost_resolved route=Memory`（emo2）→ `OnBoot` GET `["「コンフィズリー」＆「City-Pop'n」"]`（**Ref6/7 なし**）→ 自動終了 → `session_mark_cleared`。`session_mark_found` 0。終了コード 0（20.68 秒）
- 判定: **合格**。後始末は 1 回（`os_session_end_begin` 1・`_again` 0）、印は消え、後始末の所要は **66 ms** で上限 3 秒より 2 桁近く短く、打ち切りは無い（`shiori_cut=false`・`shiori_wait_cut` 0）。次の起動に Ref6/7 は付かない

## 走行 B: バルーンの根を壊して LogSink へ倒した回を `WM_ENDSESSION` で終え、次の起動を見る

壊し方: 根の中の `StayseeBalloon` の `balloons1.png`（scope 0 の面 1）を、PNG でない 17 バイトの文字列で上書きした（元は根の外へ退避し、走行の後に戻した）。

- 最初に `balloons0.png`・`balloonk0.png`（面 0）を抜いてみた回（`runB-try1`）は、起動窓の採寸（面 0 だけを使う）で失敗して起動の失敗の告知（`event="alert" scene=StartupWindow`）で終わった（終了コード 1・0.15 秒・印を書く前）。面 0 は起動前に確かめられるので LogSink へは届かない
- 面 1 だけを壊すと、採寸（面 0 だけ）は通り、起動の結線の組み立て（全部の面を焼く）で失敗して LogSink へ倒れる。これが要件の「バルーンの根が壊れて LogSink へ倒れる」回で、決定論テストの `FakeShiori::BalloonMissing` と同じ帰結（結線は成立せず、LogSink の起動は成功する）

起こすゴーストは R_POST にした（記憶の置き場に `[last] ghost = "R_POST_and_KOMAINU"` を書いてから起こす）。R_POST は同梱のバルーンを持たないので既定の StayseeBalloon に決まる。次の起動が `LastGhost` を読まずに既定の emo2 を起こすことが、ゴーストの違いで見分けられる。

コマンド（PowerShell）:

```
# <作業>\profB\sylphya.toml に format-version = 1 / [last] ghost = "R_POST_and_KOMAINU" を書く
pwsh -NoProfile -File <作業>\tools\launch.ps1 -Run runB -Profile <作業>\profB -SmokeMs 120000   # 裏で起動
# 記録に steady_talk_done が出たら
pwsh -NoProfile -File <作業>\tools\endsession.ps1 -ProcId <runB の PID> -Out <作業>\logs\runB.endsession.txt
# 続けて次の起動（バルーンは壊したまま。emo2 は自分のバルーン emo2-kakukaku を使う）
pwsh -NoProfile -File <作業>\tools\launch.ps1 -Run runB2 -Profile <作業>\profB -SmokeMs 25000
```

- B の記録の抜き書き（`runB.out.log`）:

  ```
  14:51:26.964348Z INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Memory dir=C:\tmp\areka-signoff-smr\root\ghost\R_POST_and_KOMAINU
  14:51:26.966149Z INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Default dir=C:\tmp\areka-signoff-smr\root\balloon\StayseeBalloon
  14:51:27.048091Z INFO areka::boot_resolve: 起動中の印を書きました event="session_mark_written" ghost="Ｒポストと狛犬"
  14:51:27.098074Z ERROR areka_emo_present::balloon: balloon: 面画像の bake に失敗 … balloons1.png: コンポーネントが見つかりません。 (0x88982F50)
  14:51:27.099404Z ERROR areka::emo2_boot: emo2-boot: 構築入力の組立に失敗（実 sink 結線は不成立・帰結は呼び手が記録・R7.3） error=バルーン表示対象の構築に失敗
  14:51:27.101721Z INFO areka::ghost_session: LogSink フォールバックで起動しました（emo2-boot wire 不成立）
  14:51:27.102141Z WARN areka: [main] 起動が LogSink へ倒れた——このプロセスは終わり方によらず起動中の印を残す（次の起動は既定のゴーストで Ref6/7 付き） event="session_mark_pinned_by_fallback"
  14:51:27.415446Z TRACE kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBoot references=["master"]
  （挨拶は LogSink へ流れる: ghost-sink の event="emit" が続き、14:51:29.732830Z steady_talk_done）
  14:51:50.242329Z INFO wintf::ecs::window_proc::lifecycle: [WM_ENDSESSION] … event="os_session_end" entity=24v0 lparam=0
  14:51:50.242875Z INFO areka::session_end: … event="os_session_end_begin"
  14:51:50.247798Z TRACE kanade: SHIORI 送出 event="shiori_request" method=NOTIFY id=OnClose references=["system"]
  14:51:50.256046Z INFO shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)） event="unload_clean"
  14:51:50.258941Z INFO areka: [main] きれいに終わらなかったので起動中の印を残します（次の起動は既定のゴーストで Ref6/7 付き） event="session_mark_kept" reason="logsink_fallback" first=Some(SessionEnd)
  14:51:50.259082Z INFO areka::session_end: … event="os_session_end_done" ms=16 down_ok=true shiori_cut=false
  ```

- B の終了コード **0**（23.42 秒）。記憶: 起動前 `ghost = "R_POST_and_KOMAINU"` → 後 `ghost = "R_POST_and_KOMAINU"`・`running = "Ｒポストと狛犬"`（印が残った）
- B の目印: `session_mark_pinned_by_fallback`（warn）**1**・`os_session_end_begin` 1・`os_session_end_again` 0・`os_session_end_done` 1（ms=16・down_ok=true・shiori_cut=false）・`session_mark_kept` **1（reason=logsink_fallback）**・`session_mark_cleared` **0**・`shiori_wait_cut` 0・`alert` 0・ERROR 2（上のバルーンの焼きの失敗と結線の組み立ての失敗＝意図した失敗）・パニック 0。`app_exit` は走行 A と同じく 2 件（最初の出所は `SessionEnd`）
- 次の起動 B2（記録 `runB2.out.log`）:

  ```
  14:52:13.513174Z INFO areka::boot_config: 前回はきれいに終わらなかったので、最後のゴーストの記憶を読まずに起動するゴーストを決めます event="session_mark_found" ghost=Ｒポストと狛犬
  14:52:13.514637Z INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Default dir=C:\tmp\areka-signoff-smr\root\ghost\emo2
  14:52:13.761236Z INFO areka::boot_resolve: 起動中の印を書きました event="session_mark_written" ghost="えも？？"
  14:52:14.201370Z TRACE kanade: SHIORI 送出 event="shiori_request" method=GET id=OnBoot references=["「コンフィズリー」＆「City-Pop'n」", "", "", "", "", "", "halt", "Ｒポストと狛犬"]
  14:52:38.984566Z INFO areka::app_exit: … event="app_exit" origin=Smoke closed=4
  14:52:39.055176Z INFO areka::boot_resolve: きれいに終わったので起動中の印を消しました event="session_mark_cleared"
  ```

  終了コード 0（25.64 秒）。記憶 `ghost = "emo2"`・`running = ""`。`alert` 0・ERROR 0・パニック 0
- 判定: **合格**。LogSink へ倒れた回は、倒れた時点で warn が 1 件出て、OS のセッションの終了できれいに終えても印が残った（`session_mark_kept reason="logsink_fallback"`）。次の起動は `LastGhost`（R_POST）を読まずに既定の emo2 を起こし、`OnBoot` の Ref6＝`halt`・Ref7＝倒れたゴーストの名前（Ｒポストと狛犬）だった。告知は 0 件

### 走行 C: 同じ LogSink へ倒れた回を自動終了（後始末の側の経路）で終える

OS のセッションの終了ではなく、自動終了（`run()` の後の後始末の側）でも印が残ることを確かめた。記憶の置き場は新しく作り、B と同じく R_POST を記憶に書いてから起こした（バルーンは B と同じく壊したまま）。

```
pwsh -NoProfile -File <作業>\tools\launch.ps1 -Run runC -Profile <作業>\profC -SmokeMs 15000
```

- 記録: `session_mark_written ghost="Ｒポストと狛犬"` → バルーンの焼きの失敗 → `LogSink フォールバックで起動しました` → `session_mark_pinned_by_fallback`（warn）→ `OnBoot` `["master"]` → 14:53:07.775555Z `app_exit origin=Smoke closed=4` → `OnClose` NOTIFY `["user"]` → 14:53:07.801709Z `session_mark_kept reason="logsink_fallback" first=Some(Smoke)`
- 終了コード **0**（15.24 秒）。記憶: 後 `running = "Ｒポストと狛犬"`。`session_mark_cleared` 0・`alert` 0・パニック 0
- 判定: **合格**。自動終了という「きれいな終わり」でも、倒れた回は印を残す（理由は同じ `logsink_fallback`）。次の起動は B2 と同じ形になる（B2 で見たので繰り返していない）

## 走行 D: SHIORI が応答しない状態で OS のセッションの終了を送る（上限 3 秒の打ち切り）

i686 のテスト DLL には「固まる」応答が無いが、本物の SHIORI（emo2 の pasta）を載せた helper を一時停止すれば、SHIORI が応答しない形を実機で作れる。定常の emo2 の子の helper を一時停止してすぐ、走行 A と同じ手順で `WM_ENDSESSION` を送った。

コマンド（PowerShell）:

```
pwsh -NoProfile -File <作業>\tools\launch.ps1 -Run runD -Profile <作業>\profD -SmokeMs 120000   # 裏で起動
# 記録に steady_talk_done が出たら
pwsh -NoProfile -File <作業>\tools\suspend-helper.ps1 -ProcId <runD の PID> -Out <作業>\logs\runD.suspend.txt
pwsh -NoProfile -File <作業>\tools\endsession.ps1 -ProcId <runD の PID> -Out <作業>\logs\runD.endsession.txt
# 続けて次の起動
pwsh -NoProfile -File <作業>\tools\launch.ps1 -Run runD2 -Profile <作業>\profD -SmokeMs 20000
```

- 一時停止: 14:54:12.508 に areka（PID 25260）の子の helper（PID 20840）を一時停止した。その直後の 14:54:12.779 に kanade が毎秒の `OnSecondChange` を送り、その往復が返らないまま止まった
- 送信の記録: `WM_QUERYENDSESSION` は 5 枚とも TRUE（UI スレッドが答えるので SHIORI を待たない）。1 枚目の `WM_ENDSESSION` の往復は **3014 ms**（後始末が上限まで待った分）、残りは 0〜6 ms か窓が既に無い
- 記録の抜き書き（`runD.out.log`）:

  ```
  14:54:12.779729Z TRACE kanade: SHIORI 送出 event="shiori_request" method=GET id=OnSecondChange references=["13", "0", "0", "1"]
  14:54:13.393613Z INFO wintf::ecs::window_proc::lifecycle: [WM_ENDSESSION] … event="os_session_end" entity=24v0 lparam=0
  14:54:13.394740Z INFO areka::session_end: … event="os_session_end_begin"
  14:54:13.400017Z INFO areka::app_exit: … event="app_exit" origin=SessionEnd closed=4
  14:54:16.395064Z WARN shiori-actor: SHIORI の待ちを上限で打ち切った——補助プロセスを終わらせて待ちを解く event="shiori_wait_cut" stage="in_flight_request" id=Some("OnSecondChange") limit_ms=3000 elapsed_ms=3000 unblocked=true
  14:54:16.395712Z ERROR kanade: SHIORI 呼出失敗——終了系列（Fault）へ event="shiori_failed" error=shiori request timeout: wire timeout
  14:54:16.398739Z ERROR shiori-actor: helper の異常終了を検出——死活報告（ShioriDown）を送出（以後は再報告しない） event="helper_exited" exit=Abnormal(1)
  14:54:16.400585Z INFO ghost-shutdown: ghost shutdown sequence completed
  14:54:16.400916Z INFO areka: [main] きれいに終わらなかったので起動中の印を残します（次の起動は既定のゴーストで Ref6/7 付き） event="session_mark_kept" reason="session_end_deadline" first=Some(SessionEnd)
  14:54:16.401072Z INFO areka::session_end: … event="os_session_end_done" ms=3006 down_ok=true shiori_cut=true
  14:54:16.415581Z INFO areka: [main] OS のセッションの終了で後始末は済んでいる——告知を出さず、起動中の印にも触れません event="session_end_already_handled" first=Some(SessionEnd)
  ```

- 終了コード **0**（12.69 秒）。記憶: 後 `running = "えも？？"`（印が残った）。走行後に `areka.exe`・`shiori-host32-helper.exe` は 0 件（一時停止した helper は見張りが終わらせた）
- 目印: `os_session_end_begin` 1・`os_session_end_again` 0・`os_session_end_done` 1（**ms=3006・shiori_cut=true**）・`shiori_wait_cut`（warn）**1**（`unblocked=true`）・`shiori_unblock_unavailable` 0・`session_mark_kept` **1（reason=session_end_deadline）**・`session_mark_cleared` 0・`OnClose` 0（kanade は `OnSecondChange` の往復で止まっていたので、`OnClose` の段へは進まなかった）・`alert` **0**・ERROR 2（`shiori_failed`・`helper_exited`＝打ち切りの帰結）・パニック 0。`app_exit` は 2 件（2 件目は後から届いた kanade の停止通知 `Fault(Timeout)` で `closed=0`・最初の出所は `SessionEnd` のまま、告知なし）
- 次の起動 D2: `session_mark_found ghost=えも？？` → `ghost_resolved route=Default`（emo2）→ `OnBoot` GET `["「コンフィズリー」＆「City-Pop'n」", "", "", "", "", "", "halt", "えも？？"]` → 自動終了 → `session_mark_cleared`。終了コード 0（20.87 秒）。記憶 `running = ""`。`alert` 0・ERROR 0
- 判定: **合格**。SHIORI が応答しない状態でも、OS のセッションの終了の後始末は上限 3 秒で見張りが helper を終わらせて待ちを解き（`shiori_wait_cut`・`unblocked=true`）、3006 ms で戻った。打ち切った回は印を残し（`session_end_deadline`）、次の起動は既定のゴーストで Ref6＝`halt`・Ref7 付きになった。告知は出ず、終了コードは 0
- 見た段は「在来の往復」（`in_flight_request`）だけ。「`OnClose` の通知」「降ろす」の段で固まる形は、一時停止の時刻を段に合わせる手段が無いので実機では作っていない。段の語 4 通りは見張り部品のテスト（`crates/areka-kanade/src/shiori/probe_tests.rs`）、段ごとの後始末の振る舞いは下の決定論テストで固定している

## 決定論テストと前提のテスト（同じ HEAD で実行）

実機で作れなかった段（`OnClose` の通知・降ろす）と、helper を終わらせると同期の送信がすぐ戻るという前提は、次のテストで足りる。

```
cargo test -j 4 -p areka --bin areka deadline_tests
cargo test -j 4 -p shiori-host32-host --lib terminator
```

- `session_end::deadline_tests`: 10 件すべて ok（`cut_while_in_flight_get_is_held`・`cut_while_on_close_notify_is_held`・`cut_while_unload_is_held`・`deadline_fires_without_manual_cut`・`answered_within_limit_is_not_cut`・`self_timeout_shorter_than_limit_is_not_cut`・`unbounded_shutdown_never_cuts`・`session_end_limit_is_three_seconds`・`only_session_end_calls_the_bounded_shutdown`・`deadline_call_scan_turns_red_on_one_line_removed_or_added`）
- `terminator::terminator_tests`: 6 件すべて ok（前提のテスト `terminate_mid_send_releases_synchronous_send` を含む）

## まとめ

| 走行 | 終わらせ方 | 終了コード | 後始末 | 印 | 次の起動 | 判定 |
|---|---|---|---|---|---|---|
| A 定常の emo2 | `WM_QUERYENDSESSION` → `WM_ENDSESSION`（TRUE） | 0 | 1 回・**66 ms**・`shiori_cut=false` | 消えた（`session_mark_cleared`） | route=Memory・Ref6/7 なし | 合格 |
| B LogSink へ倒れた R_POST | 同上 | 0 | 1 回・16 ms・`shiori_cut=false` | 残った（`reason=logsink_fallback`・倒れた時点で warn 1 件） | route=Default（emo2）・Ref6＝`halt`・Ref7＝Ｒポストと狛犬 | 合格 |
| C LogSink へ倒れた R_POST | 自動終了 | 0 | — | 残った（`reason=logsink_fallback`） | （B2 と同じ形・繰り返していない） | 合格 |
| D helper を一時停止した emo2 | `WM_QUERYENDSESSION` → `WM_ENDSESSION`（TRUE） | 0 | 1 回・**3006 ms**・`shiori_cut=true`・`shiori_wait_cut` 1 | 残った（`reason=session_end_deadline`） | route=Default（emo2）・Ref6＝`halt`・Ref7＝えも？？ | 合格 |

- 要件 7.7: 定常のゴーストへの OS のセッションの終了（A）、LogSink へ倒れた回の後始末と次の起動（B・C）、上限の打ち切り（D・実機では在来の往復の段のみ）を実機で確かめ、残りの段と前提は決定論テストで確かめた。告知は全走行で 0 件（最初に面 0 を抜いた試行 `runB-try1` の起動の失敗の告知を除く。これは印を書く前の起動の失敗で、本件の対象外）
- 気付いたこと: OS のセッションの終了の後、kanade の停止通知が遅れて届き `app_exit` の info がもう 1 件出る（`closed=0`）。前の spec の走行 ⑧ では 1 件だった。最初の出所は `SessionEnd` のまま残り、告知・終了コード・印の判定には影響しない（`quit_app` が 2 度目も `app_exit` を info で残す作り）。コードの変更は要らない
