# 実機確認の記録（要件 7.7・7.8）

- 日付: 2026-09-24（走行は 23:56〜00:02 JST。ログの時刻は UTC で 14:56〜15:02）
- 対象: ブランチ `claude/areka-p0-shiori-fault-a338be` の HEAD `bd82a280`
- 環境: Windows 11 Pro 10.0.26200・モニタ 2 台（2880x1800 主・3413x2133）
- 実行体: `target\debug\areka.exe`（x64 debug・`cargo build -p areka -j 4`）。隣の `shiori-host32-helper.exe` は i686（PE の機械種別 `0x014C` を確かめた）
- 検体（短い絶対パスへ複製。長いパスでは emo2 の pasta が読めなくなるため）:
  - 動く検体: `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が配った emo2 を `C:\tmp\areka-signoff\ok\` へ複製
  - 失敗する検体: 同じ複製を `C:\tmp\areka-signoff\fail\` へ置き、smoke ④ と同じ組み立てにした（`ghost\emo2\ghost\master\descript.txt` を `charset,UTF-8`／`name,emo2-fault`／`shiori,shiori_loadu.dll`／`seriko.defaultsurfacedirectoryname,master` の 4 行に差し替え、i686 の `shiori_loadu.dll` を同じフォルダへ置く）
- 全走行に共通の環境変数:
  - `NO_COLOR=1`
  - `RUST_LOG=info,kanade=trace,areka_kanade=trace,areka=debug,shiori-actor=debug,ghost-boot=debug`（判定の分岐 `shiori_failed`・`shiori_down`（kanade・error）、`connect_failed`（shiori-actor・error）、`ghost_quit`（info）と `ghost_quit_extra`（debug）、`app_exit`（info）と `app_exit_again`（debug）、`alert`（error）がすべて出る水準）
  - `AREKA_PROFILE_DIR` は走行ごとに `C:\tmp\areka-signoff\` の下の空のフォルダ（開発者のアプリの記憶を読まず・書かない）。`AREKA_ROOT` は外した
- 告知の窓の扱い: **窓は開発者の目視ではなく、エージェントが UI Automation と Win32 で読んで閉じた。** 題名と本文は UI Automation で窓から読み取った字面そのまま。OK は窓の OK ボタン（コントロール ID 2）へ `BM_CLICK` を送って押した（UI Automation の Invoke はこのボタンで使えなかった）。告知が出ている間のそのプロセスの見えている最上位の窓は `EnumWindows` で数えた。

## 走行 ①: 失敗する検体・抑止なし

コマンド（PowerShell）:

```
$env:HOST32_TESTDLL_LOADU_FAIL = '1'
$env:AREKA_PROFILE_DIR = 'C:\tmp\areka-signoff\prof1b'
target\debug\areka.exe "C:\tmp\areka-signoff\fail\ghost\emo2" "C:\tmp\areka-signoff\fail\balloon\emo2-kakukaku"
```

- 告知の窓が出た時刻: 起動から 2.92 秒
- 題名: `SHIORI が動かなくなりました`
- 本文（窓から読んだまま）:

  ```
  ゴースト: emo2-fault（C:\tmp\areka-signoff\fail\ghost\emo2）
  失敗の種類: SHIORI に接続できなかった
  理由: shiori handshake failure: SHIORI の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: [0]
  ```

- 告知の間に見えていた最上位の窓（要件 1.11）: 告知の窓（`#32770`・917x404）と `PseudoConsoleWindow`（0x0・出力の受け口の付属物で画面には何も出ない）の 2 つだけ。ゴースト窓・バルーン窓は 0 個
- OK を押したのは起動から 3.50 秒。プロセスは 0.15 秒後に終わった
- 終了コード: **1**
- 起動から終了まで: 3.65 秒
- プロセスは残っていない（`Get-Process` で 0 件・子プロセス（helper）も 0 件）
- 目印の件数:

  | 目印 | 件数 |
  |---|---|
  | `event="alert"` で題名あり | 1（`suppressed=false`: 1・`suppressed=true`: 0） |
  | `event="app_exit"` | 1（`origin=KanadeStopped(Fault(ShioriFault { kind: ConnectFailed, .. }))`） |
  | `event="app_exit_again"` | 0 |
  | `event="ghost_quit"` | 1（`cause=Fault(ShioriFault { kind: ConnectFailed, .. })`） |
  | `event="ghost_quit_extra"` | 0 |
  | `event="connect_failed"` | 1 |
  | `event="shiori_failed"` | 1 |
  | `event="shiori_down"` | 0 |
  | `event="shiori_error_response"` | 0 |
  | `event="stop_cause_unknown"` | 0 |
  | 「本物のゴースト窓を開きました」 | 1（窓は出てから閉じた） |

- 判定（要件 7.7 ①）: **合格**。題名・ゴースト名・種類「SHIORI に接続できなかった」・理由が出て、OK で閉じると終了コード 1 で終わり、プロセスは残らなかった。
- 別に見つかった欠陥: 標準エラーに bevy のシステムのパニック（`Non-send data not found`）が 1 件出た。スタックには `areka::alert::raise` → `MessageBoxW` → その中のメッセージの回し → wintf の `tick_one_frame` → `FrameFinalize` のスケジュールが並ぶ。つまり**告知の窓が出ている間、窓の裏のメッセージの回しが wintf の 1 フレームを回し、既に無い NonSend の資源を引くシステムがパニックしている**。プロセスの見え方（告知・終了コード 1・プロセスが残らない）には影響していないが、告知を出す走行（抑止なし）でだけ毎回起きる（走行 ② でも 1 件、抑止ありの走行 ③ では 0 件）。

## 走行 ②: 動く検体（emo2）・短い応答期限・抑止なし

コマンド（PowerShell）:

```
$env:AREKA_SHIORI_REQUEST_TIMEOUT_MS = '1'
$env:AREKA_PROFILE_DIR = 'C:\tmp\areka-signoff\prof2'
target\debug\areka.exe "C:\tmp\areka-signoff\ok\ghost\emo2" "C:\tmp\areka-signoff\ok\balloon\emo2-kakukaku"
```

- 使った値: **`AREKA_SHIORI_REQUEST_TIMEOUT_MS=1`**。値を当てた結果は下の表（2〜50 ms と 300・1000 ms は抑止と自動終了を付けて別に確かめた）。
- 告知の窓が出た時刻: 起動から 4.19 秒
- 題名: `SHIORI が動かなくなりました`
- 本文（窓から読んだまま）:

  ```
  ゴースト: えも？？（C:\tmp\areka-signoff\ok\ghost\emo2）
  失敗の種類: SHIORI との通信が切れた
  理由: shiori ipc failure: ipc transport failure
  ```

- 告知の間に見えていた最上位の窓: 告知の窓（`#32770`・819x373）と `PseudoConsoleWindow`（0x0）の 2 つだけ。ゴースト窓は 0 個
- OK を押したのは起動から 4.76 秒
- 終了コード: **1**
- 起動から終了まで: 4.89 秒
- プロセスは残っていない（子プロセスも 0 件）
- どこで切れたか: `OnInitialize`（NOTIFY）は通り、`username` の照会（失敗しても起動は続く・warn 1 件）の次の `OnFirstBoot`（GET）で `shiori_failed` になった＝起動の挨拶の問い合わせで切れた
- 目印の件数:

  | 目印 | 件数 |
  |---|---|
  | `event="alert"` で題名あり | 1（`suppressed=false`: 1） |
  | `event="app_exit"` | 1（`origin=KanadeStopped(Fault(ShioriFault { kind: Disconnected, reason: "shiori ipc failure: ipc transport failure" }))`） |
  | `event="app_exit_again"` | 0 |
  | `event="ghost_quit"` | 1（`kind: Disconnected`） |
  | `event="ghost_quit_extra"` | 0 |
  | `event="shiori_failed"` | 1 |
  | `event="shiori_down"` | 0 |
  | `event="connect_failed"` | 0 |
  | `event="shiori_error_response"` | 0 |
  | `event="stop_cause_unknown"` | 0 |

- 値の当たり（いずれも emo2・`AREKA_NO_ALERT=1`・`AREKA_APP_SMOKE_EXIT_MS` 付き）:

  | 値（ms） | 結果 |
  |---|---|
  | 1 | 起動の挨拶で失敗・種類「SHIORI との通信が切れた」・終了コード 1（本走行） |
  | 2 | `OnBoot`（GET）で失敗・種類「SHIORI との通信が切れた」・終了コード 1 |
  | 3・5・10・20 | 起動が通り、自動終了 20 秒まで失敗なし・`origin=Smoke`・終了コード 0 |
  | 50・300・1000 | 起動が通り、30 秒まで失敗なし（自動終了 30 秒の直前で、エージェントが自分で起こしたプロセスを止めた） |

- 判定（要件 7.7 ②）: **種類は要件どおりでない**。告知が出て、OK で終了コード 1・プロセスが残らない、までは合格。しかし種類は「SHIORI の応答が期限内に返らなかった」ではなく「SHIORI との通信が切れた」になった。どの値でも「期限内に返らなかった」は出なかった。
- 理由（静的に確かめた）: `crates/shiori-host32-ipc/src/lib.rs` の `send_copydata` は `SendMessageTimeoutW` が 0 を返したとき（期限切れも含む）を一律 `IpcError::SendFailed` にする。`IpcError::Timeout` になるのは `send_request` で送出は成功したのに応答の受け皿が空だったときだけで、helper は受けた窓手続きの中で応答を 1 通返してから戻る（`crates/shiori-host32-helper/src/main.rs` の冒頭の説明）ので、遅い SHIORI の期限切れは必ず `SendFailed` → `RequestError::Ipc` →「通信が切れた」へ写る。design の「応答の期限切れ → `map_error`（`Timeout`）→ 応答が期限内に返らなかった」の行は、i686 の実経路では届かない。要件 1.9（期限切れの告知の種類）の扱いは本走行の外で決める必要がある。

## 走行 ③: 失敗する検体・抑止あり・自動終了 20 秒

コマンド（PowerShell）:

```
$env:HOST32_TESTDLL_LOADU_FAIL = '1'
$env:AREKA_NO_ALERT = '1'
$env:AREKA_APP_SMOKE_EXIT_MS = '20000'
$env:AREKA_PROFILE_DIR = 'C:\tmp\areka-signoff\prof3'
target\debug\areka.exe "C:\tmp\areka-signoff\fail\ghost\emo2" "C:\tmp\areka-signoff\fail\balloon\emo2-kakukaku"
```

- 告知の窓: **出なかった**（UI Automation で題名の窓を 100 ms ごとに探し、プロセスが終わるまで 0 件）
- 終了コード: **1**
- 起動から終了まで: 1.97 秒（失敗は自動終了 20 秒より十分先に起きた。`app_exit` の `origin` は `Smoke` ではなく `KanadeStopped(Fault(..))`）
- プロセスは残っていない（子プロセスも 0 件）
- 標準エラーのパニック: 0 件
- 目印の件数:

  | 目印 | 件数 |
  |---|---|
  | `event="alert"` で題名あり | 1（`suppressed=true`: 1・`suppressed=false`: 0） |
  | `event="app_exit"` | 1（`kind: ConnectFailed`） |
  | `event="app_exit_again"` | 0 |
  | `event="ghost_quit"` | 1 |
  | `event="ghost_quit_extra"` | 0 |
  | `event="connect_failed"` | 1 |
  | `event="shiori_failed"` | 1 |
  | `event="shiori_down"` | 0 |
  | `event="shiori_error_response"` | 0 |
  | `event="stop_cause_unknown"` | 0 |

- 判定（要件 7.7 ③）: **合格**。窓が出ずに終了コード 1 で終わり、プロセスは残らず、告知の記録は抑止の印付きで 1 件だけ残った。

## まとめ

| 走行 | 終了コード | 告知の窓 | 種類 | プロセス | 判定 |
|---|---|---|---|---|---|
| ① 失敗する検体・抑止なし | 1 | 出た・OK で閉じた | SHIORI に接続できなかった | 残らない | 合格（ただし告知中のパニックあり） |
| ② emo2・期限 1 ms・抑止なし | 1 | 出た・OK で閉じた | SHIORI との通信が切れた | 残らない | 種類が要件と違う（「期限内に返らなかった」は実経路で出ない） |
| ③ 失敗する検体・抑止・自動終了 20 秒 | 1 | 出ない | （記録のみ）SHIORI に接続できなかった | 残らない | 合格 |

残った課題 2 件:

1. 告知の窓が出ている間に wintf の 1 フレームが回り、bevy のシステムが `Non-send data not found` でパニックする（①② で各 1 件・③ で 0 件）。
2. 応答の期限切れは実経路で「SHIORI との通信が切れた」になり、「SHIORI の応答が期限内に返らなかった」は届かない（②）。

## 開発者が自分の目で見直す手順

1. i686 の helper と `shiori_loadu.dll` を建て（`cargo build -p shiori-host32-helper -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc`）、helper を `target\debug\` へ複製し、上の「失敗する検体」を `C:\tmp\` の下へ組む。
2. PowerShell で `$env:NO_COLOR='1'; $env:HOST32_TESTDLL_LOADU_FAIL='1'; $env:AREKA_PROFILE_DIR='<空のフォルダ>'` を設定し、`target\debug\areka.exe <fail\ghost\emo2 の絶対パス> <fail\balloon\emo2-kakukaku の絶対パス>` を起動して、告知の題名・本文と、背後にゴースト窓が無いことを目で見る。
3. OK を押したあと `$LASTEXITCODE` が 1 であること、タスクマネージャに `areka.exe` と `shiori-host32-helper.exe` が残っていないことを見る（② は `HOST32_TESTDLL_LOADU_FAIL` の代わりに `AREKA_SHIORI_REQUEST_TIMEOUT_MS='1'` と `ok\` の検体で同じ手順）。
