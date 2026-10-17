# 検証の記録（areka-P0-emily-ghost-verification）

えみり（`emily4.nar`）を areka の実機で起こした検証の記録。各節は担当のタスクが埋める（同定・機械の構成・環境変数は 5.1、走行と崩れた所の表は 5.2〜6.2、数の洗い出しは 4.2）。

## 同定

準備（タスク 5.1）は 2026-10-17 22:21（日本時間）に行った。

| 項目 | 値 |
|---|---|
| コミット | `2af9e369f8449069bbb5219a1246b0a2292c2b26`（準備のときの `git rev-parse HEAD`。走行の前にコミットが進んだら、走行の節にその時のコミットを書く） |
| ビルド | debug 版（`cargo build -p areka`・`dev` の形）。`C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\debug\areka.exe` |
| 検体 | `vendors\sample_ghost\emily4.nar`（2,677,923 バイト）。sha256 `ebc87c6daa108116b9afdc6608059aa35ffbe183e0da83eb1b01888c824d39c0`（PowerShell の `Get-FileHash -Algorithm SHA256` で測った。`vendors/sample_ghost/README.md` に書かれた値と同じ） |

走らせる前の確かめ:

- 自動テスト: `cargo test -p areka-nar -p sample-ghost-kit -p log-capture-kit` は 436 本が通り、落ちた 0・無視 2（終了コード 0）。`cargo test -p areka` は 3,099 本が通り、落ちた 0・無視 2（終了コード 0）。
- 32 ビットの補助プログラム（要件 3.3）: `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` の後に `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\tools\sync-i686-helper.ps1` を回し、終了コード 0・出力は `helper is i686: target\debug\shiori-host32-helper.exe`。`areka.exe` の隣の `target\debug\shiori-host32-helper.exe` は i686 版。全体テストの後などで x64 版に替わることがあるので、走行の直前にもう一度回す。

## 機械の構成

| 項目 | 値 | 測り方 |
|---|---|---|
| OS | Windows 11 Pro・版 26H2・ビルド 10.0.26300.9711 | `cmd /c ver` と、レジストリ `HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion` の `DisplayVersion`・`CurrentBuild`・`UBR`（同じキーの `ProductName` は古い名残で「Windows 10 Pro」と出るが、ビルド 26300 は Windows 11） |
| 画面 1（主） | 2880×1800・拡大率 200%（DPI 192）・左上 (0, 0) | PowerShell の中で、プロセスをモニターごとの DPI に合わせる形（`SetProcessDpiAwarenessContext(-4)`）にしてから `EnumDisplayMonitors` と `GetDpiForMonitor` で読んだ。`HKCU:\Control Panel\Desktop\WindowMetrics` の `AppliedDPI` も 192 |
| 画面 2 | 2560×1600・拡大率 150%（DPI 144）・左上 (-2560, 195)（主の画面の左） | 同上 |

## 環境変数

areka が読む環境変数と、その値。パスはどれも `C:\` から始まる絶対パスで、ワークツリーの `target\` の下だけを使う（`C:\` の直下・`C:\tmp` は使わない・要件 3.1）。

| 名前 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT` | `C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\emily-verify\root` | 置き場の根。配布物と同じ形（下の木）。在ると書ける根（`%LOCALAPPDATA%\areka`）は並びに入らない |
| `AREKA_PROFILE_DIR` | `C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\emily-verify\profile` | areka の記憶の置き場（`crates/areka-user-data/src/lib.rs` の `PROFILE_DIR_ENV`）。準備の時点で空 |
| `RUST_LOG` | `info,areka::install=debug,areka::boot_resolve=debug,areka_nar=debug` | 判定に使う分岐（インストールの飛ばし・起動のゴーストとバルーンの決め方）の記録が出る水準（要件 3.4） |
| `AREKA_APP_SMOKE_EXIT_MS` | `1800000`（30 分） | 自動で終わる上限。1 回の走行の所要より長くし、安全弁としてだけ置く（要件 3.4） |
| `NO_COLOR` | `1` | 記録に色の制御文字を混ぜず、検索しやすい平文にする（`crates/areka/tests/smoke_boot_loop_exit.rs` と同じ扱い） |
| `AREKA_NO_ALERT` | 設定しない（起動の前に消す） | 設定すると警告のダイアログが出ない。インストールの警告（要件 1.12）を確かめるため |
| `AREKA_USER_ROOT` | 設定しない（起動の前に消す） | 書ける根を強いる変数。`target\debug\` は書けて winget の控えも無いので、設定しなければ書ける根は使わない形になり、本物の `%LOCALAPPDATA%\areka` に触らない（`crates/areka-user-data/src/lib.rs` の `decide`） |

記録の行き先: areka は記録を標準出力へ出す（`crates/areka/src/log_history.rs` の `init`）。起動の命令で `target\emily-verify\logs\` の下のファイルへ受ける。

根の形（準備の時点・上の 2 段）:

```text
target\emily-verify\
├─ profile\                  （空）
└─ root\
   ├─ ghost\
   │  └─ emo2\               （93 ファイル）
   └─ balloon\
      ├─ emo2-kakukaku\      （21 ファイル）
      └─ StayseeBalloon\     （29 ファイル）
```

`emo2` と `emo2-kakukaku` は `cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2`、`StayseeBalloon` は `cargo run -q -p sample-ghost-kit --bin nar-sample-path -- StayseeBalloon` が開いた `target\nar-samples\manual\` の下から写した（`tools/package.ps1` の組み立てと同じ 3 つ）。えみりの `.nar` は根に写していない（名前に `emily` を含む物は 0 件）。走行の中で、メニューの「インストール…」から `C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\vendors\sample_ghost\emily4.nar` を入れる。

## 走行

### 起動の命令

1 回目の走行で、下の命令をそのまま回した（2026-10-17 22:29:48）。PowerShell から、どのフォルダで打っても同じに動く形。`$v` と `$w` は絶対パスだけから組む。起動したプロセスの番号を出すので、止めるときはこの番号だけを使う（要件 3.7）。

```powershell
$w = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680'
$v = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\emily-verify'
pwsh -NoProfile -File "$w\tools\sync-i686-helper.ps1"; "sync exit=$LASTEXITCODE"
$null = New-Item -ItemType Directory -Force -Path "$v\logs"
Remove-Item Env:AREKA_NO_ALERT, Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
$env:AREKA_ROOT = "$v\root"
$env:AREKA_PROFILE_DIR = "$v\profile"
$env:RUST_LOG = 'info,areka::install=debug,areka::boot_resolve=debug,areka_nar=debug'
$env:AREKA_APP_SMOKE_EXIT_MS = '1800000'
$env:NO_COLOR = '1'
$p = Start-Process -FilePath "$w\target\debug\areka.exe" -WorkingDirectory $v -RedirectStandardOutput "$v\logs\run1.stdout.log" -RedirectStandardError "$v\logs\run1.stderr.log" -PassThru
"areka pid=$($p.Id)"
```

2 回目の走行（直した後・要件 5.6）は、根と記憶を作り直してから、記録のファイル名を `run2.*` にして同じ命令で回す。1 回目は終了コードを取り損ねた（下の 1 回目の走行の「終了」）ので、2 回目は最後の行を次の 2 行に替え、同じ呼び出しの中で終わりを待って終了コードを書き出す。

```powershell
$p = Start-Process -FilePath "$w\target\debug\areka.exe" -WorkingDirectory $v -RedirectStandardOutput "$v\logs\run2.stdout.log" -RedirectStandardError "$v\logs\run2.stderr.log" -PassThru
"areka pid=$($p.Id)"; $p.WaitForExit(); "areka exit=$($p.ExitCode)" | Set-Content "$v\logs\run2.exitcode.txt"
```

### 1 回目の走行（2026-10-17 22:29〜22:46・日本時間）

- コミット: `48a10de6`（ビルドしたコードは準備のときの `2af9e369` と同じ。間のコミットは文書だけ）。areka の番号 23564。
- 机: 負荷テストの机を取ってから回した（22:29 に番が来た・他のセッション 1 つが止まって待った）。終わってすぐ返した。
- 操作: AI が行った。areka の窓はクリックが透けて道具の右クリックが効かないので、開発者の許可（2026-10-17・この spec の走行のあいだ）を得て、PowerShell から SendInput で右クリック・撫で（マウスを左右に動かす）・ダブルクリックを送った。メニューの項目・ファイルを選ぶ画面・警告のダイアログは道具のクリックと文字の入力で操作した。
- 撮った物: ワークツリーの `target\emily-verify\shots\`（`01-install-warning.jpg`〜`10-dump-surface-scope1-s10.png`）。記録は `target\emily-verify\logs\run1.stdout.log`（1,961 行）と `run1.stderr.log`。

#### 流れ（時刻は記録の UTC・日本時間は +9 時間）

| 時刻 | 起きたこと |
|---|---|
| 13:29:49 | emo2 で起動。台詞が出た |
| 13:38:34 | メニューの「インストール…」→ ファイルを選ぶ画面のファイル名の欄に `emily4.nar` の絶対パスを入れて Enter。`[areka_nar] companion balloon skipped`（鍵 `balloon.source.directory`・値 `balloon`・置き場 `emily4`）の WARN が 1 件、続いて `[areka_nar] installed installed=1 skipped=1`。根に `ghost\emily4` が在り、`balloon\emily4` は無い（確かめた） |
| 13:38:34 | 警告のダイアログが 1 つ出た（`01-install-warning.jpg`）。題名「インストールの警告 - Emily/Phase4」、本文は書庫・入った物・入らなかったバルーン（`emily4`・取り出し元 `balloon`）・代わりのバルーンの 4 行。記録 `install_warning_dialog`（`suppressed=false`）。OK で閉じた |
| 13:39:09 | メニューの「ゴースト」→「Emily/Phase4.5」で切り替え |
| 13:39:14 | バルーンの決め方の記録: descript の `balloon,emily4` も同梱の `emily4` も根に無く、次の候補へ進んで `StayseeBalloon` を使った（`descript_balloon_not_found`・`companion_balloon_not_found`） |
| 13:39:15 | 起動の台詞「はじめまして！ボク、エミリっていいます。…」（`02`〜`04`）。1 人目の面 0 は見えた。2 人目（テディ）は面 10 で出た後、13:39:20 の面 11 で消えた（崩れた所 1） |
| 13:41:15 | 何もせず待って、時間で出る話（`OnSecondChange`・talk 2）。このとき 1 人目は面 5・6 になり姿が消えた（`05-timed-talk-sakura-gone.jpg`・崩れた所 1） |
| 13:43:15 | 時間で出る話（talk 3） |
| 13:44:12 | テディを撫でた → `OnMouseMove` に台詞が返って再生された（talk 4）。13:44:21 にダブルクリック → `OnMouseDoubleClick` の台詞（talk 5） |
| 13:45:32 | テディを右クリック → メニューが出た（`07-menu.jpg`。えみりの見出しで「説明書」は「よんでね」、「ネットワーク更新」は「こうしん」）。「よんでね」を選ぶと、メモ帳で `readme.txt` が開いた（AI が閉じた） |
| 13:46:01 | もう一度メニューを出して「終了」→ `OnClose` の台詞（talk 7）→ `talk_done_quit` → SHIORI をきれいに降ろし（`unload_clean`）→ 全窓を閉じ `app_exit origin=KanadeStopped(Quit)` → `session_mark_cleared`（起動中の印を消した）。プロセスは終わった |

#### 要件 4 の項目ごとの結果

| 要件 | 結果 | 判定した者 |
|---|---|---|
| 4.1 起動して 1・2 人目に面と台詞 | 台詞は出た。面は起動のときは 1 人目が見えたが、時間で出る話で 1 人目が、面 11 で 2 人目が消える（崩れた所 1）。**直す前は不合格の見込み** | AI の数え（記録・撮った物）／見た目は開発者の目視待ち |
| 4.2 待っても台詞が出続け落ちない | 13:41:15・13:43:15 に時間で出る話が始まって終わった。落ちていない | AI の数え |
| 4.3 撫でが SHIORI へ届く | `OnMouseMove` に台詞が返って再生された（13:44:12） | AI の数え |
| 4.4 右クリックメニューと選んだ働き | メニューが出て、「よんでね」で説明書が開いた。「終了」で終わった | AI の数え／見た目は開発者の目視待ち |
| 4.5 終了の台詞の後にきれいに終わる | `OnClose` の台詞の後に `KanadeStopped(Quit)` で終わり、起動中の印も消えた。**終了コードは取り損ねた**（起動と待ちを別の呼び出しにしたため・2 回目で取る） | AI の数え（終了コードは未） |
| 4.6 panic と致命が 0 | `panicked` 0 件。致命で止まった記録 0 件。ERROR は 47 件（下の「ERROR と WARN の数」） | AI の数え |
| 4.7 3 人目の台詞は出なくてよい | 3 人目（`\p[2]`）への面の指示は「未装着ターゲット」で読み捨てられ、会話は止まらなかった | AI の数え |
| 4.8 目視の項目は開発者が決める | 撮った物（警告のダイアログ・起動の台詞・消えた姿・メニュー）を渡したが、開発者は 1 回目を見ていない（2026-10-17「見てない。」）。目視の判定は、直した後の 2 回目の走行でまとめて受ける | 開発者（2 回目で） |
| 4.9 見た目の違いだけで不合格にしない | `.pna` を使わないことなどは不合格の理由にしていない | AI |

#### ERROR と WARN の数（切り替えの後・要件 5.5）

| 件数 | 水準 | 中身 | 数から外すか |
|---|---|---|---|
| 135 | WARN | `surface id 重複: 既存定義を全置換する（後勝ち）` | 外さない（崩れた所 1 の原因そのもの） |
| 39 + 4 | ERROR | `apply(ShowSurface)`・`apply(Hide)`: 未装着ターゲット（`TargetId(4)`・`TargetId(6)`＝3 人目・4 人目） | 外す。3 人目からの窓は作らない決め（タスク 3.1）の結果で、3 人目の窓は `extra-character-windows` の持ち場（崩れた所 2） |
| 4 + 4 | ERROR・WARN | 面 11 が「定義層が皆無で外形 0×0」→ 窓を隠す | 外さない（崩れた所 1） |
| 6 | WARN | バルーンの面 2・3 が本体側の系列へ縮退（`StayseeBalloon` に 2 人目用の面 2・3 の絵が無い） | 外す。バルーンの中身から出るもので、決まった縮退の道 |
| 2 | WARN | `.pna` を使わない（27 枚） | 外す。裁定（2026-10-05・`.pna` は使わない） |
| 2 | WARN | descript と同梱のバルーンが根に見つからない（`emily4`） | 外す。同梱を飛ばしたので当然出る（見立て 9） |
| 1 | WARN | 3 人目の文字の装着先が未解決（`actor=2`） | 外す（崩れた所 2 と同じ） |
| 2 | WARN | 飛ばした同梱の記録・警告のダイアログの記録 | 外す（本 spec の決まった記録） |

外した残りは ERROR 4・WARN 139 で、どれも崩れた所 1 から出ている。

#### 研究の記録の見立て（5 節）との照合

| # | 見立て | 当たり外れ |
|---|---|---|
| 1 | 根にバルーンが 0 個で起動が止まる | 当たらなかった（根に `StayseeBalloon` を並べたので、`StayseeBalloon` に落ちて起動した） |
| 2 | 3 人目の窓で panic | 実機では当たらなかった（タスク 3.1 で先に直した。panic 0） |
| 3 | 3 人目の台詞で会話が止まる | 当たらなかった（止まらない。3 人目の面の指示は読み捨て） |
| 4 | YAYA が辞書を読めない | 当たらなかった（起動の台詞・時間の話・撫での応えが出た） |
| 5 | 起動の知らせの順で分かれ道が変わる | 目に見える崩れは無かった |
| 6 | SSP 専用の命令が多い | 今回の走行の範囲では、命令で落ちることは無かった |
| 7 | 見た目の違い | `.pna` の WARN は当たった。ほかは開発者の目視待ち |
| 8 | descript の最初の面を読まない | 表に出なかった（3 人目の窓が無い） |
| 9 | 起動の記録の警告がえみりの中身から出る | 当たった（同梱のバルーンが見つからない 2 件） |
| — | （見立てに無かった）面の番号が 2 回書かれたときに前の定義が消える | 新しく見つかった（崩れた所 1） |

#### MCP で撮った物

- `dump_surface` の scope 0（面 5）は白紙だった（`09-dump-surface-scope0-s5.png`・崩れた所 1 と同じ原因）。scope 1（面 10）はテディの絵（`10`）。
- `get_status` は `talking,balloon(0=0/1=0)`。`sakurascript` は `NG:not implemented yet`（崩れた所 3）。`dump_balloon` は今回は撮っていない。

### 2 回目の走行（直した後・2026-10-17 23:52〜23:58・日本時間・タスク 6.2）

- コミット: `5f574ae8`（タスク 6.1 の直しを含む）。直した後に `cargo build -p areka` で debug 版を作り直し、i686 の補助プログラムを作り直して `tools/sync-i686-helper.ps1` を回した（終了コード 0・`placed i686 helper`）。areka の番号 39104。
- 根と記憶: `target\emily-verify\root\` と `profile\` を消して、5.1 と同じ 3 つ（`ghost\emo2`・`balloon\emo2-kakukaku`・`balloon\StayseeBalloon`）から作り直した。記憶の置き場は空から。
- 机: 負荷テストの机を取ってから回し（23:52 に番が来た・他のセッション 1 つが止まって待った）、areka が終わった直後に返した。走行の外（直し・記録）では机も画面の操作も持たない（2026-10-17 開発者「タスクの区切りを超えて占有は許可しない」）。
- 起動の命令: 上の「起動の命令」の 2 回目の形（同じ呼び出しの中で `WaitForExit` し、終了コードを `run2.exitcode.txt` に書く）。記録は `target\emily-verify\logs\run2.stdout.log`（1,356 行）・`run2.stderr.log`（補助プログラムの 2 行だけ）。
- 撮った物: `target\emily-verify\shots\r2-01-install-warning.jpg`〜`r2-08-after-quit.jpg`。
- 操作: 1 回目と同じ（SendInput と道具のクリック）。

#### 流れ（時刻は記録の UTC・日本時間は +9 時間）

| 時刻 | 起きたこと |
|---|---|
| 14:52:31 | emo2 で起動（空の記憶なので初めての挨拶） |
| 14:53:06 | メニューの「インストール…」で `emily4.nar` を入れた。`companion balloon skipped` の WARN 1 件・警告のダイアログ 1 回（`r2-01`・1 回目と同じ文面）。OK で閉じた |
| 14:53:32 | メニューの「ゴースト」→「Emily/Phase4.5」で切り替え |
| 14:53:4x | 起動の台詞（`r2-02`・`r2-03`）。1 人目もテディも見えた。テディは面 11（顔の向きを変えた絵）でも姿を保った（1 回目はここで消えた） |
| 14:55:35 | 何もせず待って、時間で出る話（`OnSecondChange`）。1 人目は面 5・6 でも見えた（`r2-04`・1 回目はここで消えた） |
| 14:56:13 | 1 人目の頭を撫でた → `OnMouseMove` に台詞が返って再生された（`r2-05`） |
| 14:56:29〜36 | 1 人目を右クリック → メニュー（`r2-06`）→「よんでね」→ メモ帳で `readme.txt` が開いた（AI が閉じた） |
| 14:56:48〜14:58:00 | 「終了」を選ぼうとしたが、メニューが何も選ばれずに閉じ、その後の右クリックでは「メニューを出した」の記録は出るのに画面に見えなかった（崩れた所 6） |
| 14:57:37 | 時間で出る話（別の面・`r2-07`）。1 人目もテディも見えた |
| 14:58:40 | SendInput だけで右クリック → メニューが見え、SendInput のクリックで「終了」→ `OnClose` の台詞（talk 5）→ `talk_done_quit` → `unload_clean` → `app_exit origin=KanadeStopped(Quit)` → `session_mark_cleared` |
| 14:58:42 | プロセスが終わった。**終了コード 0**（`run2.exitcode.txt`「areka exit=0 at 23:58:42」） |

#### 要件 4 の項目ごとの結果

| 要件 | 結果 | 判定した者 |
|---|---|---|
| 4.1 起動して 1・2 人目に面と台詞 | 合格。起動・時間で出る話・撫での応えのどれでも、1 人目とテディの面と台詞が出た。空の絵（`EmptyComposition`）は 0 件 | AI の数え（記録）＋開発者の目視（2026-10-18「合格」） |
| 4.2 待っても台詞が出続け落ちない | 合格。14:55:35・14:57:37 に時間で出る話 | AI の数え |
| 4.3 撫でが SHIORI へ届く | 合格。`OnMouseMove` に台詞が返った（14:56:13） | AI の数え |
| 4.4 右クリックメニューと選んだ働き | 合格。メニューが出て、「よんでね」で説明書が開き、「終了」で終わった。途中でメニューが見えなくなる件は崩れた所 6 | AI の数え＋開発者の目視（メニュー・2026-10-18「合格」） |
| 4.5 終了の台詞の後にきれいに終わる | 合格。`OnClose` の台詞の後に `KanadeStopped(Quit)`、起動中の印を消し、終了コード 0 | AI の数え |
| 4.6 panic と致命が 0 | 合格。`panicked` 0 件・致命 0 件 | AI の数え |
| 4.7 3 人目の台詞は出なくてよい | 3 人目への面の指示は読み捨てで、会話は止まらなかった | AI の数え |
| 4.8 目視の項目は開発者が決める | 開発者が撮った物（`r2-01`〜`r2-07`）を見て「合格」（2026-10-18） | 開発者 |
| 4.9 見た目の違いだけで不合格にしない | 守った（バルーンが `StayseeBalloon` に落ちること・`.pna` を使わないことは理由にしていない） | AI |

#### ERROR と WARN の数（要件 5.5）

| 件数 | 水準 | 中身 | 数から外すか |
|---|---|---|---|
| 47 | ERROR | 未装着ターゲット（`TargetId(4)` 45・`TargetId(5)` 1・`TargetId(6)` 1＝3 人目・4 人目） | 外す（崩れた所 2・`extra-character-windows`） |
| 6 | WARN | バルーンの面 2・3 が本体側の系列へ縮退 | 外す（`StayseeBalloon` の中身・決まった縮退の道） |
| 2 | WARN | `.pna` を使わない | 外す（2026-10-05 裁定） |
| 2 | WARN | descript と同梱のバルーンが根に見つからない | 外す（同梱を飛ばしたので当然出る） |
| 1 | WARN | 3 人目の文字の装着先が未解決 | 外す（崩れた所 2） |
| 2 | WARN | 飛ばした同梱の記録・警告のダイアログの記録 | 外す（本 spec の決まった記録） |
| 3 | WARN | emo2 の側（全透明の element 2・emo2-kakukaku の折返し基準 1） | 外す（切り替え前の emo2 とそのバルーンの中身） |

外した残りは ERROR 0・WARN 0。1 回目に 135 件あった `surface id 重複` は DEBUG に下げたので WARN には出ない（足し合わせは決めた振る舞いで失敗ではない）。

## 崩れた所の表

| 番号 | 症状 | 根拠 | 原因 | 扱い | 起票した spec か裁定 |
|---|---|---|---|---|---|
| 1 | 1 人目が面 5・6 で、2 人目が面 11 で姿が消える。MCP の `dump_surface`（面 5）も白紙 | 13:39:20・13:41:15 の記録（`EmptyComposition surface_id=11`・`全透明退化 → Hide`）、`05-timed-talk-sakura-gone.jpg`・`06-both-gone.jpg`・`09-dump-surface-scope0-s5.png`。WARN `surface id 重複` 135 件 | えみりの `surfaces.txt` は、面ごとの `surface5` などのブレスに element定義（絵）を書き、`surface0,3,5,6,8,21,28` などの共有のブレスに当たり判定と着せ替えを書く。areka は同じ番号の 2 つ目のブレスで前の定義を丸ごと置き換える（`crates/areka-emo-compose/src/fold.rs` の `upsert_surface`・「後勝ち全置換」）ので、絵が消える。面 11 は自分の絵の画像ファイルが無く、全部消えて外形 0×0 になる | 直した（要件 4.1 を妨げていた・タスク 6.1）。直し方は開発者の決め（2026-10-17）で「足し合わせる」 | 2026-10-17 開発者の決め（下の「直しの見積もりと 20 本の線」の「直した結果」） |
| 2 | 3 人目・4 人目への面の指示が ERROR「未装着ターゲット」（43 件）。3 人目の文字が装着先未解決で溜まる | 13:39:15〜の記録（`TargetId(4)`・`TargetId(6)`・`actor=2`） | 3 人目からの窓を作らない決め（タスク 3.1）。areka は 3 人目からのキャラクターに対応していない | 起票済み（範囲の外） | `extra-character-windows` |
| 3 | MCP の `sakurascript` が `NG:not implemented yet` | 13:42 の MCP の答え | areka が MCP の `sakurascript` にまだ対応していない | 起票済み（範囲の外） | `mcp-kanade-tools` |
| 4 | `.pna` を使わない | WARN 2 件（27 枚） | 裁定どおり | 方針で受けない | 2026-10-05 裁定（areka は α 付き画像だけを正しく使う・`.pna` は無視） |
| 5 | 終了コードを取り損ねた | 起動の命令の形 | 手順の不備（起動と待ちを別の呼び出しにした） | 直した（2 回目の走行で終了コード 0 を取った） | — |
| 6 | メニューで「終了」を道具のクリックで選ぶと、何も選ばれずにメニューが閉じた。その後の右クリック 3 回は「メニューを出した」（`menu_shown`）の記録が出るのに画面に見えず、選んだ記録も閉じた記録も無い。SendInput だけで出し直すと見えて、「終了」を選べた | 2 回目の 14:56:48・14:57:20・14:57:31・14:57:58 の `menu_shown`（対になる `menu_selected` が無い）、23:56:56・23:57:23・23:57:33 の撮った物 | 未確定。メニューが前面を取れない（メモ帳を閉じた直後や、道具が別の窓を前に出した後）と、Win32 のメニューが見えずに閉じる見込み。選ばれずに閉じたときの記録は DEBUG（`crates/areka/src/menu/trigger.rs` の `menu_dismissed`）で、今回の記録の水準（`info`・メニューは開けていなかった）では出ないので、見えずに閉じたのか、見えないまま開いているのかを記録から区別できなかった | 起票の候補（範囲の外・要件 4.4 は満たした）。`/kiro-complete` の棚卸で起票する。再現を測るときは `RUST_LOG` に `areka::menu=debug` を足す | — |

崩れた所のうち、areka が対応していないためにうまくいかなかった件（2・3）は、どちらも既に起票されていた spec の持ち場なので、新しい起票は 0 本。`.kiro/specs/areka-P0-extra-character-windows/`・`.kiro/specs/areka-P0-mcp-kanade-tools/` が在ることを確かめた。

### 直しの見積もりと 20 本の線（タスク 5.3・要件 6.4）

- 「直す」は崩れた所 1 の 1 件だけ。
- 直す場所: `crates/areka-emo-compose/src/fold.rs` の `upsert_surface`（298 行あたり・同じ番号の 2 つ目のブレスで前の定義を消して作り直す所）と、そこへ定義を渡す畳み込み。
- 直し方（開発者の決め・2026-10-17「足し合わせる」）: 同じ番号のブレスが 2 つ以上あれば、前の定義に後のブレスの行を足す。同じ名前の行（`element1` と `element1` など）が重なったときは後の行が勝つ。上書きするのは、完了 spec `areka-P0-emo-compose` の要件 2.1（既存の番号は「重複定義規則」で扱う＝全置換として実装）と、`areka-P0-shell-implicit-surface` の設計が `doc/COMPAT_ARCHITECTURE.md` §8 に書いた行（`surface.append` の後に来た同じ番号の波括弧は全置換）。この 2 つの番号を `doc/COMPAT_ARCHITECTURE.md` に書く。
- 見積もり: タスク 1 本（6.1・直す前に赤のテスト → 直す → 文書）。直す場所が 1 か所なので、6.1 を件ごとのタスクに分けない。
- 20 本の線: いま決まっている 13 本（1.1〜6.2）から 6.1 を除いた 12 本＋直しのタスク 1 本＝13 本。**20 本を超えない。**
- SHIORI へ渡すバイト列を全部のゴーストで変える直しは含まない（要件 6.3 の聞き取りは不要）。

#### 直した結果（タスク 6.1）

- 直した場所:
  - `crates/areka-emo-compose/src/fold.rs` の `fold_plain_surface`（同じ番号が既に在れば作り直さず、前の定義へ足す）と `merge_brace`（足し方。element定義は element番号・当たり判定は collision番号・アニメーションは animation番号と pattern番号で、同じ名前の行は後の行が勝つ）。
  - `crates/areka-emo-compose/src/boxes.rs` の `distribute`（シェル内バルーン `elementN,balloon,…` の置き場所の行も同じ規則で足す）。箱の行は絵の行と同じ見出しの展開で配っているので、絵だけを足して箱を全置換のままにすると、同じ `surfaces.txt` から組んだ絵と箱の畳み込みが食い違う。両方を同じ規則に揃えた。
- 直す前に赤・直した後に緑のテスト（`crates/areka-emo-compose/src/fold_merge_tests.rs`）: `shared_brace_adds_to_the_per_face_brace_instead_of_replacing_it`（えみりと同じ形・ファイル名は作り物）と `same_named_line_in_a_later_brace_wins`（同じ名前の行の後勝ち）。
- 直す前の赤（直す前の `fold.rs` に上の 2 本だけを足して `cargo test -p areka-emo-compose --lib merge_tests`）: 2 本とも赤——`assertion left == right failed: 面 5 の絵が共有のブレスで消えない  left: []  right: [(0, "body5.png"), (1, "face5.png")]`、もう 1 本は `left: [(1, "b1.png", (2, 2)), (2, "b2.png", (3, 3))]  right: [(0, "a0.png", (0, 0)), (1, "b1.png", (2, 2)), (2, "b2.png", (3, 3))]`（element0 が消えた）。
- 全置換を前提に書かれていたので、足す規則の判定へ書き直したテスト: `fold_tests.rs` の `duplicate_id_same_layer_line_is_last_wins`・`interleaved_redefinition_is_appearance_order_deterministic`、`log_firing_tests.rs` の `duplicate_surface_id_merge_logs_debug_not_warn`、`boxes_tests.rs` の `later_surface_brace_adds_to_the_placements`、`base_image_tests.rs` の `brace_after_append_merges_into_the_image_only_face`。
- 記録の重さ: 同じ番号の 2 つ目のブレスの記録を `warn!`「既存定義を全置換する（後勝ち）」から `debug!`「前の定義へ後のブレスの行を足す（同じ名前の行は後勝ち）」へ下げた。足すのは定めた振る舞いで失敗ではなく、えみりでは 1 本のシェルで 135 件当たる（走行の WARN の数）。
- 上書きした完了 spec の要件（`doc/COMPAT_ARCHITECTURE.md` §8 に書いた）: `areka-P0-emo-compose` 要件 2.1（既存の番号は全置換）と、`areka-P0-shell-implicit-surface` の設計が §8 に書いた行（`surface.append` の後に来た同じ番号の波括弧は全置換）。`surface.append` の規則（`areka-P0-emo-compose` 要件 2.4）は変えていない。

#### 既知の限界（完了のときに開発者へ聞く）

- 後のブレスが、ある animation番号の pattern を書くのにその `animationN.interval` の行を書かないと、足した後の interval は `bind` になる。転記層（parser）は interval の行が無いアニメーションを `bind` で埋めるので、「書かれていない」と「`bind` と書かれた」を見分けられない。「同じ名前の行は後の行が勝つ」を字面どおりに読むと、書かれていない行は前の行を上書きしないはずなので、この点だけ規則に合わない。
- 登記した検体での件数: **0 件**。登記したゴースト 5 体（`emo2`・`R_POST_and_KOMAINU`・`konnoyayame`・`claudia`・`emily4`）のシェルは各 1 本（`shell/master`）の計 5 本で、6 本ではない。数え方: `target\nar-samples\cache\` に展開済みの各検体の `ghost\<名>\shell\master\surfaces.txt` を、小さな数え上げの道具（使い捨て・ワークツリーの外）で読んだ。`surface.append` でないブレスごとに、pattern の行（`animationN.patternM`・旧書式 `NpatternM`）はあるが同じ N の interval の行（`animationN.interval`・旧書式 `Ninterval`）が無いブレスを数え、5 本とも 0（ブレスの数は `emo2` 59（ほかに `surface.append` のブレス 5）・`R_POST_and_KOMAINU` 9・`konnoyayame` 10・`claudia` 13・`emily4` 60）。較正: `claudia` の `animation0.interval` の 1 行と、えみりの最初の `animation50.interval` の 1 行をそれぞれ消した写しでは 1 件と数えた。
- 開発者へ残す問い: 「起票するか、方針で受けないか」（完了のときに聞く）。

## 網羅台帳との突き合わせ（タスク 6.2・要件 8.4）

2026-10-18 に、えみりが使う項目と、この記録が触れる項目の網羅台帳の行を、この記録と今のコードに照らして読み直した。読み直しの中で、タスク 3.1（3 人目以降の窓を作らない）の後も「`charN.` の綴りでキャラクタが立つ」と書いたままの行が見つかったので、それも直した（3.1 で消した `placement::config::char_scope_of` の名前を挙げている行が 20 行あった）。

### 直した行

| ファイル | 行（id の見出し） | 直したこと |
|---|---|---|
| `ledger/assets.toml` | `descript_ghost` の `char*.name` | 状態を実装済み → 縮退。名前は読むが 3 人目以降の窓を作らないので画面に出ない、と書き直した（崩れた所 2）。束の行の下位区分も縮退に揃えた。宛先の欄は空のまま（`areka-P0-extra-character-windows` は網羅の道筋の spec の表に載っておらず、宛先に書くと検査が赤になる）で、担当の spec の名前は備考に書いた |
| `ledger/assets.toml` | `descript_ghost` の `char*.balloon.defaultsurface`・`char*.seriko.defaultsurface`、`descript_shell` の `char*.` で始まる 18 行（`balloon.dontmove`・`balloon.syncscale`・`bindgroup*.addid`・`bindgroup*.default`・`bindgroup*.name`・`bindoption*.group`・`defaultleft`・`defaulttop`・`defaultx`・`defaulty`・`icon.frame.color.r/g/b`・`menu`・`menuitem*`・`menuitemex*`・`name`・`seriko.alignmenttodesktop`） | 「`charN.` の番号が 2 以上ならそのキャラクタを立てる（`char_scope_of`）」を、「番号が 2 以上でもキャラクタを立てない（2026-10-17・タスク 3.1。絞りを外すのは `areka-P0-extra-character-windows`）」へ。窓の置き方の 4 行と `seriko.alignmenttodesktop` は「番号が 2 以上の値は使われない」を足した。えみりの `char2.balloon.offsetx` は台帳に行が無い（見出しが無い）ので、直す行も無い。`char3`・`char4.seriko.alignmenttodesktop` はこの直しに入る。状態はどれも未対応のまま |
| `ledger/assets.toml` | `descript_shell` の `sakura.` で始まる 16 行（`balloon.dontmove`・`balloon.offsetx/y`・`balloon.syncscale`・`defaultleft`・`defaulttop`・`defaultx`・`defaulty`・`icon.frame.color.r/g/b`・`menu`・`menuitem*`・`menuitemex*`・`name`・`name2`） | 「人数を決める材料は `kero.` と `charN.` の綴り」を「`kero.` の綴りだけ」へ |
| `ledger/assets.toml` | `descript_shell_surfaces` の `element*` | 2026-10-17 の注記を足した。同じ番号の 2 つ目の `surface*`ブレスは前の定義へ足す（同じ名前の行は後勝ち・記録は debug!）、全置換だったころはえみりの絵が消えた（崩れた所 1）。状態と担当は変えていない |
| `ledger/assets.toml` | `descript_shell_surfaces` の `animation*.interval` | 2026-10-17 の注記を足した。後のブレスが pattern定義だけを書くと間隔が `bind` になる既知の限界（上の「既知の限界」・登記した検体では 0 件） |
| `ledger/sakura-script.toml` | `\p[ID番号]` | 状態を実装済み → 縮退。番号が 2 以上の話し手は窓が無く、文字は出ず面の指示は読み捨てられ、会話は止まらない（タスク 3.2）と書いた。「ログ: 出ない」を、番号が 2 以上では error!（未装着ターゲット）と warn!（装着先が未解決）が出る、へ直した（2 回目の走行で ERROR 47 件・WARN 1 件）。宛先の欄は空のまま（理由は `char*.name` と同じ） |

状態を 2 行動かしたので、機械が作る報告（`doc/ukadoc-coverage/report/assets.md`・`sakura-script.md`・`summary.md`）を `report` と `report-summary` で作り直し、`doc/ukadoc-coverage/briefing.md` の 4-2 の `descript_ghost` の数（実装済み 17 → 16・縮退 1 → 2）とその説明の 1 文を直した。

### 読んで、直さなかった行

| ファイル | 行 | 直さなかった理由 |
|---|---|---|
| `ledger/assets.toml` | `descript_install` の `*.source.directory` | 4.2 で直した文（取り出し元の無い同梱は飛ばして warn!・警告のダイアログ 1 つ・起動のバルーンは他に当たらなければ既定）が、2 回の走行（`companion balloon skipped` 1 件・ダイアログ 1 回・`StayseeBalloon` が着いた）と合う |
| `ledger/assets.toml` | `descript_install` の `*.directory` | 同梱の最初の 1 個が標準のバルーンになる、は同梱が置き場に在るときの話で、えみりでは `companion_balloon_not_found` を残して次の段へ進んだ（`boot_resolve.rs` の同梱の段どおり） |
| `ledger/assets.toml` | `descript_ghost` の `balloon` | 当たらないとき `descript_balloon_not_found` を残して次へ進む、が走行の記録と合う |
| `ledger/assets.toml` | `descript_shell` の `seriko.use_self_alpha` | `.pna` は使わず、添えてあった数を warn! で残す、が走行の WARN（`.pna` を使わない 2 件）と合う（裁定 2026-10-05） |
| `ledger/assets.toml` | `descript_ghost` の `readme` | メニューの「説明書」（えみりの見出しは「よんでね」）が既定の関連付けで `readme.txt` を開く、が走行と合う |
| `ledger/shiori.toml` | `OnMouseMove`・`OnMouseDoubleClick`・`OnSecondChange`・`OnClose`・`OnGhostChanged` | どれも実際に SHIORI へ送る、と書いてあり、走行で台詞が返った（撫で・ダブルクリック・時間で出る話・終了の台詞・切り替え） |
| `ledger/shiori.toml` | `OnInstallCompleteEx`・`OnInstallComplete` | 入れた物だけを並べる（飛ばした同梱は入れた物に入らない）。`judge.rs` の `complete_ex_refs`・`complete_legacy_refs` が入れた物の列から組むので、書いてあることと食い違わない。`OnInstallFailure` の「拒否の 13 種」は 4.2 で直してある |
| `ledger/shiori.toml` | `char*.popupmenu.type`・`char*.popupmenu.visible` | 「2026-09-18 の時点で起票は 0 本」と時点を付けて書いた文で、偽にはならない。3 人目以降の窓が無いことは今も同じ |
| `ledger/assets.toml` | `descript_shell_surfaces` の `collision*`・`animation*.pattern*` | 書いてあるのは一つの行の読み方（矩形だけ・`overlay` だけ）で、同じ番号のブレスの重なり方には触れていない。足し方は `element*` と `animation*.interval` の注記と COMPAT §8 にある |

`surface*`ブレスそのもの（`surface.append` を含む）は、ukadoc で項目の見出しになっていないので台帳に行が無い（`manual_shell` の行の備考のとおり）。同じ番号のブレスを足す決めごとと、追記の後に来た波括弧の扱いは `doc/COMPAT_ARCHITECTURE.md` §8 にある。

台帳に「同じ番号の 2 つ目のブレスで前の定義を全置換する」と書いた行は、直す前から 0 件だった（台帳の 4 本を「全置換」「後勝ち」で検索して確かめた）。直した後に、台帳の 4 本で `char_scope_of` を検索して 0 件、「`kero.` と `charN.` の綴りだけ」も 0 件。**えみりが使う項目と、この記録が触れる項目で、記録や今のコードと食い違う記述（偽の記述）は 0 件。**

網羅台帳の検査: `cargo test -p ukadoc-survey` は 601＋6＋113＋5 本が通り、落ちた 0（終了コード 0）。

## 数の洗い出し

タスク 4.2 で、ソースと文書の全体を `14 種`・`14 変種`・`fourteen`・`7 つ`・`7 本`・`seven` で検索した（2026-10-17・`git grep -n -I -E "14 種|14 変種|fourteen|7 つ|7 本|seven"`）。本 spec で変わった数は 2 つ——拒否の閉じた語彙（14 種 → 13 種）と、登記した検体の数（7 → 8）。外した場所を除いて 183 行が当たり、全部を下のどれかに片付けた（直した 2 行は直した後は当たらないので、183 行はどれも、この数とは別の数か、変わったことそのものを書いた行）。**片付いていない行は 0 件。**

### 検索から外した場所と理由

- `.kiro/specs/completed/` の下: 終わった spec の当時の記録で、書き直すと当時の事実が消える。上書きした完了 spec の要件は、書き直す代わりに `doc/COMPAT_ARCHITECTURE.md` §8 に番号つきで書いた（要件 6.5・8.2）。
- 本 spec のフォルダ（`.kiro/specs/areka-P0-emily-ghost-verification/`）: 「14 種 → 13 種」「7 → 8」という変化そのものを書いている文書なので、古い数が出るのが正しい。
- `target/`: ビルドの出力で、追跡していない（`git grep` は見ない）。
- `vendors/` の下: 当たりは 0 件だった（検体の README も含めて）。

### 直した行

- `doc/COMPAT_ARCHITECTURE.md` §8「インストールの失敗理由の語の写し」の行: 「拒否の 14 種」と「14 種を 1 つずつ組んで」を 13 種へ。`invalid type` の並びから `CompanionSourceMissing` を外した。
- `doc/ukadoc-coverage/ledger/shiori.toml` の `OnInstallFailure` の備考: 「areka-nar の拒否の 14 種」を 13 種へ。
- タスク 1.2・2・4.1 で直した分（今は当たらない）: 語彙のテスト・`error_tests.rs`・`judge_tests.rs` の 14 → 13、`procedure.rs` の口の数 7 → 8、`sample-ghost-kit` の登記のテストの 7 → 8。検索で、これらの古い数が残っていないことを確かめた。

### この数とは別の数（残した行と理由）

どれも、拒否の語彙の数でも、登記した検体の数でもない。

**ソース（`crates/`・`tools/`・`.claude/`）**

| 場所 | 数えているもの |
|---|---|
| `crates/areka-nar/src/error_tests.rs:372`（`manifest_warnings_are_seven_and_carry_their_key`） | 記録だけを残す警告（`ManifestWarning`）の種類 7。拒否の語彙とは別の型で、本 spec は警告の種類を足していない（飛ばした同梱は結果の別の欄に載せた）ので 7 のまま |
| `crates/areka/src/install/procedure_branch_tests.rs:336`・`crates/areka/src/install/fetch_url_tests.rs:144`（`..._seven_days`） | 日数 7 日（残った中身を消すまでの日数・古い落とし物を消す日数） |
| `crates/areka/src/update/refs_tests.rs:421`（`unsent_seven_words_never_appear`） | 更新の知らせで送らない語 7 つ |
| `crates/areka/src/log_history_tests.rs:139`（`..._fourteen_rows_in_order`） | 記録の履歴の規則の表の 14 行 |
| `crates/areka-parsers/src/balloon/parse_tests.rs:861`（`font_base_key_table_has_fourteen_entries...`） | バルーンの descript の書体の鍵 14 |
| `crates/sample-ghost-kit/src/devroot_sweep_tests.rs:501` | 同じ検体を取る 8 本のスレッドのうち、残りの 7 本（検体の数ではない） |
| `crates/areka/src/mcp/mcp_tests.rs:27・143・146` | MCP の道具のうち、相手を省略できる 7 本 |
| `crates/areka/src/mcp/get_property_tests.rs:6` | 聞くプロパティの名前 7 つ |
| `crates/areka/src/mcp/get_expression_table_tests.rs:343`（「検体 7」・上の 6 語では当たらず、検体の数の言い回しを広げて探したときに出た行） | 表情の表のテストの見本の 7 番目（登記した検体ではない） |
| `crates/areka/src/emo2_boot/spine_conformance_script.rs:618・694・714`・`crates/areka-kanade/src/schedule/events_tests.rs:350・443` | マウスのイベントの Reference 7 つ |
| `crates/areka/src/ghost_session_switch_fallback_tests.rs:420`（`Ref6/7`） | Reference の番号 6 と 7 |
| `crates/areka/src/emo2_boot/user_break_cue.rs:70` | 起動の受け口の 7 本目 |
| `crates/areka/src/emo2_boot/balloon_visibility_phase_reappear_tests.rs:10・267`・`crates/areka/src/emo2_boot/frame_balloon_offset_follow_tests.rs:42・47`・`crates/areka/src/placement/spawn_zorder_pair_deferred_tests.rs:9`・`crates/wintf/src/ecs/window/zorder_chain.rs:355` | そのファイルの中のテストの本数 |
| `crates/areka/src/placement/placement_prepare_tests.rs:343・344・487・491`（`seven_sixths`） | 拡大率 7/6 を入れた変数の名前 |
| `crates/areka-emo-atlas/src/animated_tests.rs:432` | 動く絵を縮める理由 7 つ |
| `crates/areka-emo-compose/src/nesting_report_tests.rs:80` | アニメーションの ID の語 7 つ |
| `crates/areka-emo-text/src/state_cluster_tests.rs:34` | 絵文字の部品の数 7 |
| `crates/areka-emo-text/tests/decoration_readback_test.rs:102`・`crates/areka-emo-text/tests/decoration_readback/mod.rs:78` | DirectWrite が範囲で持てる飾りの鍵 7 つ |
| `crates/areka-ghost/src/catalog_tests.rs:69` | ゴーストの素性の項目 7 つ |
| `crates/areka-mcp-bridge/src/service_tests.rs:188・189` | 要求の id に使った文字列 `"seven"` |
| `crates/areka-seriko/src/looper_film_count_tests.rs:66` | 16 ミリ秒の刻み 7 回 |
| `crates/wintf/src/ecs/window/command_batch_tests.rs:206`・`command_transition_tests.rs:140`・`zorder_chain_diag_tests.rs:101` | 窓の位置の引数 7 つ・窓を作る引数 7 つ・重なりの順の札 7 つ |
| `crates/ukadoc-survey/` の 17 行（`catalog/mod.rs:5`・`documents/derive_tests.rs:445`・`evidence/candidates.rs:512`・`evidence/resolve.rs:307`・`ledger/read_tests.rs:439`・`model.rs:238・278`・`model_tests.rs:127`・`report/tally_tests.rs:112・197`・`tests/consistency/briefing_arms.rs:717`・`checks.rs:765・771`・`linkage_checks.rs:6・23・508`・`spec_checks.rs:39`） | 網羅台帳の道具の数（状態の綴り 7 つ・検査の腕 7 つ・必須の欄 7 つ・正典の URL 7 つ・逃がし文字 7 つ・完了 spec の持ち主の宛先 14 種） |
| `tools/perf/judge-perf.py:260・1287・1431` | 「267 本」の一部（間隔の本数） |
| `.claude/agents/perf-analyze.md:39` | スケジュールの 7 本 |

**文書**

| 場所 | 数えているもの |
|---|---|
| `doc/COMPAT_ARCHITECTURE.md:287・290・300` | テストの名前の中の 7 日（`..._seven_days`）と、送らない語 7 つ（`unsent_seven_words_never_appear`） |
| `doc/COMPAT_ARCHITECTURE.md:333`（本 spec で足した行） | 「14 種 → 13 種」と、変わったことそのものを書いている |
| `README.md:95`・`doc/CONSTITUTION.md:76` | areka のエンジン 7 つ |
| `README.md:107`・`doc/release-notes/v0.0.2.md:21` | MCP の道具 10 本のうち動く 7 本 |
| `doc/impl-watch.md:27・43` | 机の決まり 7 つ |
| `doc/ssp-mcp/areka-tools.md:65` | 台本の診断の種類 7 つ |
| `doc/roadmap-overview-2026-10-16.html:70` | 10-16 時点のロードマップの写し（檻 7 本・Reference 7 つ・申し送り 7 本） |
| `doc/ukadoc-coverage/README.md:144` | 台帳の状態の綴り 7 つ |
| `doc/ukadoc-coverage/briefing-assets.md:381〜485`（「実例に使う 7 本のファイル」） | emo2 の系統の検体の中にある、正典の書式で書かれたファイル 7 本（検体の数ではない。同じ節が「検体 `R_POST_and_KOMAINU.nar` は数えていない」と断っている） |
| `doc/ukadoc-coverage/briefing-assets.md:814・816` | バルーンの書体の正典の見出し 14 種 |
| `doc/ukadoc-coverage/briefing-property.md:955` | 「7,007 本」の一部（テストの本数） |
| `doc/ukadoc-coverage/briefing-sakura-script.md:8` | 説明書の節 7 つ |
| `doc/ukadoc-coverage/briefing-shiori.md:59`・`doc/ukadoc-coverage/ledger/shiori.toml:30` | 優先度が空の群 7 つ |
| `doc/ukadoc-coverage/briefing.md:1725・1746・2435・2592`・`doc/ukadoc-coverage/roadmap-draft.md:1489・1494` | 「27 本」の一部（spec の本数） |
| `doc/ukadoc-coverage/ledger/property.toml` の 13 行（177・192・1496・1526・1559・1729・1764・2277・2292・2658・2673・2688・2703） | プロパティの名前のルートの枝 7 本 |
| `doc/ukadoc-coverage/linkage.md:153・2406` | 束の残り 7 本・束の欄 7 つ |
| `.kiro/steering/product.md:63`・`.kiro/steering/roadmap.md` の 13 行 | ウェーブ C8 の 7 本・spec の本数（107 本 など）・メニューの枠 7 つ・Reference 7 つなど |
| `.kiro/steering/roadmap-history.md` の 31 行 | 過去のウェーブの記録（その時点の spec・テスト・ファイルの本数）。拒否の語彙と検体の数を言う行は無い |
| ほかの spec の `brief.md` の 28 行（`check-script-arg-checks`・`coverage-roadmap-refresh`・`property-name-case-fold` ほか、19 本の spec） | その spec の数（診断の種類 7 つ・メニューの枠 7 つ・「バルーンのリンクと OS の連携」の 7 本・テストの本数など）。拒否の語彙と検体の数を言う行は無い |

### 配布物の許可表

`tools/package.ps1` の許可表は `ghost/` の下が `emo2` だけ、`balloon/` の下が `emo2-kakukaku` と `StayseeBalloon` だけで、`emily` の綴りは 0 件（大小を無視して数えた）。えみりは配布物に入らない（要件 2.7・変更 0）。
