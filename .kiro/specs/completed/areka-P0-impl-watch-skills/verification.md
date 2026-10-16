# 確かめの記録（areka-P0-impl-watch-skills）

設計書「確かめの記録（`verification.md`）」と Testing Strategy に沿った実走の記録。

## 7.1 置き方の道具の実走（要件 7.1, 7.3, 7.4, 7.5, 7.8, 11.1, 11.2）

- 実施: 2026-10-16（PowerShell 7・作業木 `C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c`）
- 一時の置き場所: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\impl-watch-skills-trial\home`（以下 `<試し>`。始める前に無いことを確かめ、空から作った）
- 道具: `tools/impl-watch-install.ps1`（変更なし）

### 置き場所の守り

どの試しのコマンドも、頭に次の 2 文を置いた（以下の各コマンドでは `<守り>` と略す）。値が作業木の `target\` で始まらなければ何もせず 9 で止まる。

```powershell
$env:AREKA_IMPL_WATCH_HOME='C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\impl-watch-skills-trial\home'; if (-not $env:AREKA_IMPL_WATCH_HOME.StartsWith('C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\')) { 'ABORT guard'; exit 9 }
```

`ABORT guard` はどのコマンドでも出なかった。

### 本物の置き場所の前と後（読むだけ）

本物の置き場所 `C:\home\maz\store\.areka-impl-watch` を `cmd /c dir /a` と `Get-ChildItem -Force -Recurse` で読んだ。

| 時点 | 中身 | フォルダの更新時刻 | `C:\c` |
|---|---|---|---|
| 試しの前 | ファイル 0・フォルダ 0（空） | 2026/10/09 18:51 | 無い |
| 試しの後 | ファイル 0・フォルダ 0（空） | 2026/10/09 18:51（`2026-10-09T18:51:22.9857545+09:00`） | 無い |

前と後は同じで、`C:\c\` はできていない（試しの前から無かった）。

### 場合 1: 空の置き場所への初めての置き方

```powershell
<守り>; if (Test-Path $env:AREKA_IMPL_WATCH_HOME) { Remove-Item -Recurse -Force -LiteralPath $env:AREKA_IMPL_WATCH_HOME }; pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\tools\impl-watch-install.ps1; "exit=$LASTEXITCODE"; Get-ChildItem -Force -LiteralPath $env:AREKA_IMPL_WATCH_HOME | Select-Object Name,Length
```

出力:

```text
OK placed areka-impl-watch.exe
OK check: --help exit 0
exit=0

Name                 Length
----                 ------
areka-impl-watch.exe 557056
```

設計書の道具の表との突き合わせ: 順 3 でフォルダを作り、順 4 は前のものが無いので `OK placed areka-impl-watch.exe` だけ（`OK renamed previous …` は出ない）、順 6 の `OK check: --help exit 0`、終了コード 0 —— 合う。置き場所には `areka-impl-watch.exe` だけがあり、状態のファイルは無い（Testing Strategy 1）。

### 場合 2: 見張りを走らせたままの入れ替え

見張りを `<試し>` の実行ファイルでバックグラウンドに立てた。

```powershell
<守り>; & "$env:AREKA_IMPL_WATCH_HOME\areka-impl-watch.exe" watch --id trial-a --repo trial 2>&1; "exit=$LASTEXITCODE"
```

`status` で `trial-a trial working`・`waits=1`、プロセス（pid 16176・`<試し>\areka-impl-watch.exe`）が走っていることを見てから道具を走らせた。

```powershell
<守り>; pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\tools\impl-watch-install.ps1; "exit=$LASTEXITCODE"; Get-ChildItem -Force -LiteralPath $env:AREKA_IMPL_WATCH_HOME | Select-Object Name,Length
```

出力:

```text
OK renamed previous to areka-impl-watch.old-20261016T064730Z.exe
OK placed areka-impl-watch.exe
KEEP areka-impl-watch.old-20261016T064730Z.exe (in use; run again later)
OK check: --help exit 0
exit=0

Name                                      Length
----                                      ------
alive
areka-impl-watch.exe                      557056
areka-impl-watch.old-20261016T064730Z.exe 557056
impl-watch.log                            407
state.json                                668
state.lock                                0
status.md                                 634
```

道具の後に `Get-Process -Id 16176` で見張りがまだ走っていること（StartTime 15:47:18 のまま）を確かめた。

設計書の道具の表との突き合わせ: 順 4 で走っている実行ファイルの改名ができて `OK renamed previous to areka-impl-watch.old-<UTC>.exe`（`<UTC>` は `20261016T064730Z` の形）と `OK placed …`、順 5 で走っているため消せず `KEEP <名前> (in use; run again later)` で続け、順 6 の `OK check: --help exit 0`、終了コード 0 —— 合う（7.3・7.4）。見張りは走り続けた。

### 場合 3: 見張りを `leave` で終えた後の後始末

```powershell
<守り>; & "$env:AREKA_IMPL_WATCH_HOME\areka-impl-watch.exe" leave --id trial-a 2>&1; "exit=$LASTEXITCODE"; try { Wait-Process -Id 16176 -Timeout 30 -ErrorAction Stop; 'watch ended' } catch { "wait: $($_.Exception.Message)" }
```

```text
left
exit=0
watch ended
```

バックグラウンドの見張りの出力は `gone: removed`・`exit=3`（記録が消えて終わった）。続けて道具を走らせた。

```powershell
<守り>; pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\tools\impl-watch-install.ps1; "exit=$LASTEXITCODE"; Get-ChildItem -Force -Recurse -LiteralPath $env:AREKA_IMPL_WATCH_HOME | ForEach-Object { '{0} {1}' -f $_.FullName.Substring($env:AREKA_IMPL_WATCH_HOME.Length+1), $(if($_.PSIsContainer){'<dir>'}else{$_.Length}) }
```

出力:

```text
OK renamed previous to areka-impl-watch.old-20261016T064753Z.exe
OK placed areka-impl-watch.exe
OK removed areka-impl-watch.old-20261016T064730Z.exe
OK removed areka-impl-watch.old-20261016T064753Z.exe
OK check: --help exit 0
exit=0
alive <dir>
areka-impl-watch.exe 557056
impl-watch.log 547
state.json 249
state.lock 0
status.md 317
alive\trial-a.watch.lock 0
```

設計書の道具の表との突き合わせ: 順 4 は今のものが在るのでいつもどおり改名して置き、順 5 で場合 2 で残った古いもの（`…064730Z.exe`）とこの回に改名したもの（`…064753Z.exe`）をどちらも `OK removed <名前>` で消し、順 6 の確かめ、終了コード 0 —— 合う（7.4）。状態のファイル（`state.json`・`state.lock`・`alive\`・`status.md`・`impl-watch.log`）はどれも残っている（7.8）。

### 場合 4: 空と相対の値の断り

試しのフォルダ（`<試し>` の親）を今のフォルダにし、守りを通した後で、道具を呼ぶときだけ値を差し替えた。前と後に試しのフォルダ全体の一覧（名前・大きさ・更新時刻）を取って比べた。

```powershell
<守り>; $trial = $env:AREKA_IMPL_WATCH_HOME; $base = Split-Path $trial -Parent; Set-Location $base; $snap = { Get-ChildItem -Force -Recurse -LiteralPath $base | ForEach-Object { '{0}|{1}|{2:o}' -f $_.FullName, $(if($_.PSIsContainer){'d'}else{$_.Length}), $_.LastWriteTimeUtc } }; $before = & $snap; foreach ($v in @('', 'foo', '/c/home/maz/store/.areka-impl-watch')) { $env:AREKA_IMPL_WATCH_HOME = $v; "--- value=[$v]"; pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\tools\impl-watch-install.ps1; "exit=$LASTEXITCODE" }; $env:AREKA_IMPL_WATCH_HOME = $trial; $after = & $snap; "trial tree unchanged: $((Compare-Object $before $after) -eq $null)"; "foo exists in cwd: $(Test-Path (Join-Path $base 'foo'))"; "C:\c exists: $(Test-Path C:\c)"
```

**手順からの逸れ（記録として残す）:** この回の 3 つ目の値 `/c/home/maz/store/.areka-impl-watch` は本物の置き場所を `/c/…` の形で指していた。brief.md 68 行（試しは `target\` の下の Windows 形の絶対パスだけを指す・本物の置き場所はどの形でも使わない）からの逸れである。`/c/…` の分かれ道は試しの置き場所を指す `/c/…` の値（下の回）で同じく確かめられるので、避けられた。道具は順 1 で断り（終了コード 1）、何も書いていない（本物の置き場所は試しの後も 0 件・フォルダの更新時刻 `2026-10-09T18:51:22`・`C:\c` は無い。上の「本物の置き場所の前と後」）。7.2 以降の試しではこの形を繰り返さない。

出力:

```text
--- value=[]
FAIL home: AREKA_IMPL_WATCH_HOME is not set
exit=1
--- value=[foo]
FAIL home: AREKA_IMPL_WATCH_HOME is not an absolute path
exit=1
--- value=[/c/home/maz/store/.areka-impl-watch]
FAIL home: AREKA_IMPL_WATCH_HOME is not an absolute path
exit=1
trial tree unchanged: True
foo exists in cwd: False
C:\c exists: False
```

`/c/…` の形の場合 4 の証跡は、試しの置き場所を指す値で走らせた次の回とする。

```powershell
<守り>; $trial = $env:AREKA_IMPL_WATCH_HOME; $base = Split-Path $trial -Parent; Set-Location $base; $snap = { Get-ChildItem -Force -Recurse -LiteralPath $base | ForEach-Object { '{0}|{1}|{2:o}' -f $_.FullName, $(if($_.PSIsContainer){'d'}else{$_.Length}), $_.LastWriteTimeUtc } }; $before = & $snap; $v = '/c/home/maz/git/areka/.claude/worktrees/areka-p0-watch-skills-7d4d4c/target/impl-watch-skills-trial/home'; $env:AREKA_IMPL_WATCH_HOME = $v; "--- value=[$v]"; pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\tools\impl-watch-install.ps1; "exit=$LASTEXITCODE"; $env:AREKA_IMPL_WATCH_HOME = $trial; $after = & $snap; "trial tree unchanged: $((Compare-Object $before $after) -eq $null)"; "C:\c exists: $(Test-Path C:\c)"; "<試し>の親\c exists: $(Test-Path (Join-Path $base 'c'))"
```

```text
--- value=[/c/home/maz/git/areka/.claude/worktrees/areka-p0-watch-skills-7d4d4c/target/impl-watch-skills-trial/home]
FAIL home: AREKA_IMPL_WATCH_HOME is not an absolute path
exit=1
trial tree unchanged: True
C:\c exists: False
<試し>の親\c exists: False
```

設計書の道具の表との突き合わせ: 順 1 で空は `FAIL home: AREKA_IMPL_WATCH_HOME is not set`、`foo` と `/c/…`（証跡は試しの置き場所を指す `/c/…` の回）は `FAIL home: AREKA_IMPL_WATCH_HOME is not an absolute path`、どれも終了コード 1 —— 合う（7.5）。値は画面へ出ていない（7.7）。試しのフォルダの一覧は前後で同じで、`foo`・`c` のフォルダも `C:\c\` もできていない（何もできていない）。

### 終わりの状態

- 立てたバックグラウンドの見張り（pid 16176）は場合 3 の `leave` で終了コード 3 で終わった。`<試し>` の実行ファイルのプロセスは残っていない。
- 本物の置き場所の実行ファイルは走らせておらず、`clear` はどこでも呼んでいない。本物の置き場所を指した値は場合 4 の逸れの 1 回だけで、順 1 で断られ何も書いていない。

## 7.2 スキルに書いたとおりのコマンドの並び（要件 2.6〜2.10, 3.5〜3.8, 3.10, 4.4, 5.5, 11.1, 11.3, 12.1, 12.4）

- 実施: 2026-10-16 15:55〜16:01（Git Bash・作業木 `C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c`）
- 一時の置き場所: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\impl-watch-skills-trial\seq-home`（以下 `<並び>`。7.1 の `home` とは別。始める前に無いことを確かめた）
- 識別 `trial-a`（実装の側）・`trial-b`（負荷テストの側）、リポジトリ `trial`、名前 `trial-spec`
- 呼ぶ形はスキル（`kiro-impl` の「Call form」・`kiro-complete` の「呼ぶ形」）の字のまま: `"$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" <コマンド> <引数> 2>&1; echo "exit=$?"`。バックグラウンドと前景の分け方もスキルのとおり（`watch`・`merge --wait`・`loadtest --wait`・`stopped --wait` はバックグラウンド、`leave`・`merged`・`loaddone`・`cancel` は前景）。並びの間の様子を見るために、試しの側でだけ `status` を前景で呼んだ（スキルのふだんの流れでは呼ばない）。

### 置き場所の守り（Bash）

どの試しのコマンドも、頭に次の 1 行を置いた（以下 `<守り>` と略す）。値が作業木の `target\` で始まらなければ何もせず 9 で止まる。`ABORT guard` はどのコマンドでも出なかった。

```bash
export AREKA_IMPL_WATCH_HOME='C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\impl-watch-skills-trial\seq-home'; case "$AREKA_IMPL_WATCH_HOME" in 'C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\'*) ;; *) echo 'ABORT guard'; exit 9;; esac
```

並び 7 で別の値を指すときも、同じ形で `target\impl-watch-skills-trial\` の下の値を置いてから確かめた（`no-home` だけは守りを通した後に `unset` で空にし、本物の置き場所へは戻していない）。

値は Windows 形のまま実行ファイルへ渡った: `status` の行が `status: C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\impl-watch-skills-trial\seq-home\status.md` と出て、`watch` の 2 行目の `details:` も同じ形。Git Bash の `[ -f "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" ]` と `"$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe"` は `\` と `/` の混ざった道筋のまま動いた。`C:\c\` も `target\impl-watch-skills-trial\c` もできていない。

### 本物の置き場所の前と後（読むだけ）

本物の置き場所 `C:\home\maz\store\.areka-impl-watch` を PowerShell の `Get-ChildItem -Force -Recurse` と `Get-Item` で読んだ。

| 時点 | 中身 | フォルダの更新時刻 | `C:\c` |
|---|---|---|---|
| 試しの前（15:55:21） | 0 件 | `2026-10-09T18:51:22.9857545+09:00` | 無い |
| 試しの後（16:01:12） | 0 件 | `2026-10-09T18:51:22.9857545+09:00` | 無い |

前と後は同じで、`C:\c\` はできていない。本物の置き場所を指す値はどの形（`C:\…`・`/c/…`）でも使っていない。`clear`（全部）はどこでも呼んでいない。

### 用意: 置き方の道具で `<並び>` へ置く

```powershell
$env:AREKA_IMPL_WATCH_HOME='C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\impl-watch-skills-trial\seq-home'; if (-not $env:AREKA_IMPL_WATCH_HOME.StartsWith('C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\target\')) { 'ABORT guard'; exit 9 }; pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c\tools\impl-watch-install.ps1; "exit=$LASTEXITCODE"
```

```text
OK placed areka-impl-watch.exe
OK check: --help exit 0
exit=0
```

`<並び>` には `areka-impl-watch.exe` だけがあり、状態の無いところから始めた。続けて机の前置き（`kiro-impl`・`kiro-complete` の字のまま）を作業木で走らせ、`desk: id=areka-p0-watch-skills-7d4d4c repo=areka` を得た（前置きの `on` の枝。以下の並びは指示どおり識別 `trial-a`・`trial-b`・リポジトリ `trial` を字のまま書いた）。

### 並び 1: 見張り → 負荷テストの番 → 停止要請 → 止まった → 再開 → 立て直し

表の「種」は 背＝バックグラウンド・前＝前景。コマンドはどれも `<守り>; ` の後に呼ぶ形の字のまま呼んだ。

| 順 | コマンド | 種 | 出力 | 終わり方の表の行 |
|---|---|---|---|---|
| 1 | `watch --id trial-a --repo trial --name trial-spec` | 背 | （走り続ける） | — |
| 2 | `watch --id trial-b --repo trial --name trial-spec` | 背 | （走り続ける） | — |
| 3 | `loadtest --id trial-b --repo trial --purpose "trial load, about 1 minute" --wait` | 背 | （待つ） | — |
| 4 | 1 の見張りが終わる | — | `stop requested by trial-b`・`details: <並び>\status.md`・`exit=0` | 見張り `exit=0`・`stop requested by <X>` → `stopped --wait`（3.1, 3.3） |
| 5 | `stopped --id trial-a --wait` | 背 | （待つ） | — |
| 6 | 3 の待ちが終わる | — | `granted load; stopped: trial-a`・`exit=0` | 番が来た（kiro-impl の Desk timing 3） |
| 7 | `loaddone --id trial-b` | 前 | `load done`・`exit=0` | — |
| 8 | 5 の待ちが終わる | — | `resumed`・`exit=0` | 再開の待ち `exit=0`・`resumed` → 見張りを立て直す（3.5） |
| 9 | `watch --id trial-a --repo trial --name trial-spec` | 背 | （走り続ける。`status` で `trial-a trial working`・`trial-b trial working`・`waits=2`） | 立て直し（3.5） |

### 並び 2: 止まっている間の負荷テストの番と見張りの 1 回の立て直し

| 順 | コマンド | 種 | 出力 | 終わり方の表の行 |
|---|---|---|---|---|
| 1 | `loadtest --id trial-b --repo trial --purpose "trial load 2, about 1 minute" --wait` | 背 | （待つ） | — |
| 2 | 並び 1 の 9 の見張りが終わる | — | `stop requested by trial-b`・`details: …`・`exit=0` | 見張り `exit=0`・`stop requested by <X>`（3.1, 3.3） |
| 3 | `stopped --id trial-a --wait`（止まる切れ目で先に。3.9） | 背 | （待つ） | — |
| 4 | `loadtest --id trial-a --repo trial --purpose "trial load a, about 1 minute" --wait`（止まったまま申し込む） | 背 | （待つ） | — |
| 5 | 1 の待ちが終わる | — | `granted load; stopped: trial-a`・`exit=0`（`status`: `trial-a trial stopped absent`・`load: holder=trial-b queue=trial-a`） | — |
| 6 | `loaddone --id trial-b` | 前 | `load done`・`exit=0` | — |
| 7 | trial-b の見張り（並び 1 の 2）が終わる | — | `stop requested by trial-a`・`details: …`・`exit=0` | 見張り `exit=0`・`stop requested by <X>` |
| 8 | `stopped --id trial-b --wait` | 背 | （待つ） | — |
| 9 | 3 と 4 の待ちが続けて終わる | — | 3: `resumed`・`exit=0`／4: `granted load; stopped: trial-b`・`exit=0` | 再開の待ち `resumed` → 立て直し（3.5）と、止まっている間に来た番 → 見張りを 1 回だけ立て直す（4.4） |
| 10 | `watch --id trial-a --repo trial --name trial-spec`（立て直し 1 度目） | 背 | （走り続ける） | 3.5・4.4 |
| 11 | `watch --id trial-a --repo trial --name trial-spec`（同じ出来事で 2 度目を呼んだ場合） | 背 | `a watch wait for trial-a is already running`・`exit=1` | 見張り `exit=1`・`a watch wait for <ID> is already running` → 自分の見張り（10）が走っているので何もしない（2.7・表の下の「2 度目が `already running` で終わっても害の無いもの」・4.4） |
| 12 | `loaddone --id trial-a` | 前 | `load done`・`exit=0` | — |
| 13 | 8 の待ちが終わる | — | `resumed`・`exit=0` | 再開の待ち `resumed`（3.5） |
| 14 | `watch --id trial-b --repo trial --name trial-spec` | 背 | （走り続ける） | 立て直し（3.5） |

### 並び 3: マージの番 → マージの記録 → 見張りの終わり

| 順 | コマンド | 種 | 出力 | 終わり方の表の行 |
|---|---|---|---|---|
| 1 | `merge --id trial-a --repo trial --spec trial-spec --wait` | 背 | `granted merge repo=trial; last: none`・`exit=0` | kiro-complete の前置きステップ 6（番が来た・`last:` を報告に添える） |
| 2 | `merged --id trial-a --pr 1 --sha 0123456` | 前 | `merged repo=trial`・`exit=0` | kiro-complete ステップ 8 の「マージの机」（参加を終える） |
| 3 | 並び 2 の 10 の見張りが終わる | — | `gone: removed`・`exit=3` | 見張り `exit=3` → 今の見張りだが参加を終えた（`merged`）後なので何もしない（2.10） |

続く `status` に `merge trial: holder=none queue=0 last=PR#1 0123456 trial-spec 2026-10-16T06:57:58Z` が出た（記録が残る）。

### 並び 4: `already running`

| 順 | コマンド | 種 | 出力 | 終わり方の表の行 |
|---|---|---|---|---|
| 1 | `watch --id trial-a --repo trial --name trial-spec` | 背 | （走り続ける） | — |
| 2 | `watch --id trial-a --repo trial --name trial-spec`（同じ識別の 2 本目） | 背 | `a watch wait for trial-a is already running`・`exit=1` | 見張り `exit=1`・`a watch wait for <ID> is already running`（2.3, 2.7, 2.8。自分の見張りが走っているので何もしない） |
| 3 | `loadtest --id trial-b --repo trial --purpose "trial load 3, about 1 minute" --wait` | 背 | `granted load; stopped: trial-a`・`exit=0`（5 の後） | — |
| 4 | 1 の見張りが終わる | — | `stop requested by trial-b`・`details: …`・`exit=0` | 見張り `exit=0`・`stop requested by <X>` |
| 5 | `stopped --id trial-a --wait` | 背 | （待つ） | — |
| 6 | `stopped --id trial-a --wait`（再開の待ちの種類の 2 本目） | 背 | `a resume wait for trial-a is already running`・`exit=1` | 再開の待ち `exit=1`・`a resume wait for <ID> is already running` → 何もしない・6.4 の失敗に数えない（3.10） |
| 7 | `loaddone --id trial-b` | 前 | `load done`・`exit=0` | — |
| 8 | 5 の待ちが終わる | — | `resumed`・`exit=0` | 再開の待ち `resumed`（3.5） |

### 並び 5: 記録が消えたときの立て直し

| 順 | コマンド | 種 | 出力 | 終わり方の表の行 |
|---|---|---|---|---|
| 1 | `watch --id trial-a --repo trial --name trial-spec`（並び 4 の 8 の立て直し） | 背 | （走り続ける） | 3.5 |
| 2 | `merge --id trial-b --repo trial --spec trial-spec-b --wait` | 背 | `granted merge repo=trial; last: PR#1 0123456 trial-spec 2026-10-16T06:57:58Z`・`exit=0` | — |
| 3 | `merge --id trial-a --repo trial --spec trial-spec --wait`（b の後ろに並ぶ） | 背 | （待つ。`status`: `merge trial: holder=trial-b queue=1`） | — |
| 4 | `leave --id trial-a`（ほかの者が a の記録を消した場合に当たる） | 前 | `left`・`exit=0` | — |
| 5 | 1 の見張りが終わる | — | `gone: removed`・`exit=3` | 見張り `exit=3` → 今の見張りで、参加を終えていない → 1 行伝えて立て直し、持っていた申し込みを申し込み直す（2.9） |
| 6 | 3 の待ちが終わる | — | `gone: request gone`・`exit=3` | kiro-complete の前置きステップ 7（自分の `cancel` の後でない → 見張りを立ててから申し込み直す） |
| 7 | `watch --id trial-a --repo trial --name trial-spec`（立て直し） | 背 | （走り続ける。`status`: `trial-a trial working`・`waits=3`） | 2.9 |
| 8 | `merge --id trial-a --repo trial --spec trial-spec --wait`（申し込み直し） | 背 | 9 の後に `granted merge repo=trial; last: PR#1 0123456 trial-spec 2026-10-16T06:57:58Z`・`exit=0` | 2.9・前置きステップ 7 |
| 9 | `cancel --id trial-b`（b の机を外す） | 前 | `cancelled`・`exit=0` | — |
| 10 | `cancel --id trial-a`（並びの後始末。番を外す・参加は続ける） | 前 | `cancelled`・`exit=0` | — |

7 の立て直しは `already running` にならずに走り続けた（前の見張りは 5 で終わっていて錠を離していた）。

### 並び 6: `clear --repo`（kiro-next の Step 0）

`kiro-next` の Step 0 の Bash を字のまま、作業木で走らせた（`R` は作業木から求まる `areka`）。続けて呼ぶ形の字のまま `--repo trial` でも呼んだ。

```bash
<守り>
R=$(basename "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")")
if [ -z "$AREKA_IMPL_WATCH_HOME" ]; then echo "desk: skip no-home"
elif [ ! -f "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" ]; then echo "desk: skip no-exe"
elif ! [[ "$R" =~ ^[A-Za-z0-9._-]{1,100}$ ]]; then echo "desk: skip bad-name $R"
else "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" clear --repo "$R" 2>&1; echo "exit=$?"; fi
"$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" clear --repo trial 2>&1; echo "exit=$?"
```

```text
usage error: clear: --repo is not accepted; see areka-impl-watch --help
exit=2
usage error: clear: --repo is not accepted; see areka-impl-watch --help
exit=2
```

今の実行ファイル（`--help` の `clear` の行に `--repo` が無い版）は `--repo` を受けず、2 で断った。`kiro-next` の読み方の「`exit=2`（`clear --repo` をまだ持たない実行ファイル）→ 開発者へ 1 行伝えて棚卸を続ける」の行に当たる。断りの後の `status` は前と同じ（`trial-a`・`trial-b` が `working`・`last=PR#1 …` が残る）で、退避のファイルもできておらず、全部の `clear` には倒れていない。`--repo` を受ける版での `cleared repo=<名前>; left: …`・0 の行は、拾い残しの spec（`areka-P0-impl-watch-residue`）の後に確かめる。

### 並び 7: 前置きの 3 つの `skip`

机の前置き（`kiro-impl`・`kiro-complete` の字のまま）と `kiro-next` の Step 0 の Bash を、値と場所を変えて走らせた。

| 場合 | 用意（`<守り>` の後） | 机の前置きの出力 | kiro-next の Step 0 の出力 |
|---|---|---|---|
| 環境変数が空 | `unset AREKA_IMPL_WATCH_HOME`（作業木で） | `desk: skip no-home` | `desk: skip no-home` |
| 実行ファイルの無いフォルダ | 値を `…\target\impl-watch-skills-trial\seq-noexe`（空のフォルダ・中身 0 件）にして守りで確かめる | `desk: skip no-exe` | `desk: skip no-exe` |
| 形に合わない識別 | 値は `<並び>`。`…/target/impl-watch-skills-trial/bad name` に `git init` した試しのリポジトリへサブシェルで `cd` して走らせる（前置きは今のフォルダの git から名前を求めるため） | `desk: skip bad-name bad name bad name` | `desk: skip bad-name bad name` |

どれも机のコマンドを呼ばずに終わり、`seq-noexe` は後も中身 0 件のまま。`kiro-impl` の「`desk: skip no-home|no-exe|bad-name`」の行（別の値をこしらえず 1 行伝えて机の段を飛ばす・6.1, 6.2）と `kiro-next` の読み方の「`desk: skip …` → 1 行伝えて棚卸を続ける」の行に当たる。

### 並び 8: 残りの見張りの外し方

並び 5 の 7 の見張り（`trial-a`）を走らせたまま、それを前のセッションの残りの見張りに見立て、終わり方の表の下の 1 本を字のまま（`<ID>`=`trial-a`・`<R>`=`trial`・`<feature>`=`trial-spec`）バックグラウンドで走らせた。

```bash
<守り>; "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" leave --id trial-a 2>&1; sleep 3; "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" watch --id trial-a --repo trial --name trial-spec 2>&1; echo "exit=$?"
```

| 順 | 出来事 | 出力 |
|---|---|---|
| 1 | 1 本の中の `leave` | `left`（読まない行） |
| 2 | 古い見張り（並び 5 の 7）が終わる | `gone: removed`・`exit=3`（今の見張りではなくなったので読み流す） |
| 3 | `sleep 3` の後の新しい見張り | `already running` にならずに走り続けた（`status`: `trial-a trial working`・`waits=2`。1 本の出力は `left` の 1 行だけのまま） |

見張り `exit=1`・`a watch wait for <ID> is already running` の行の「覚えが無ければ残りの見張りの外し方の 1 本」（2.8）に当たる。この 1 本の `leave` は参加の終わりに数えず、この 1 本が今の見張りになった。

### 後始末（実行の終わりの `leave`）

```bash
<守り>; "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" leave --id trial-a 2>&1; echo "exit=$?"; "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" leave --id trial-b 2>&1; echo "exit=$?"
```

```text
left
exit=0
left
exit=0
```

走っていた 2 本（並び 8 の 1 本・並び 2 の 14 の trial-b の見張り）は、どちらも `gone: removed`・`exit=3` で終わった（参加を終えた後の 3 なので何もしない・2.10）。後で `Get-Process -Name areka-impl-watch` は 0 件。立てたバックグラウンドの見張りと待ちはどれも自分で終わり、止めたプロセスは無い。`<並び>` には `areka-impl-watch.exe`・`state.json`・`state.lock`・`status.md`・`impl-watch.log`・`alive\`（錠のファイル）が残る。

### まとめ

| 並び | 当たった行 | 結果 |
|---|---|---|
| 1 | 見張り `stop requested by <X>`・再開の待ち `resumed` | 合う |
| 2 | 見張り `stop requested by <X>`・再開の待ち `resumed`・止まっている間の番と 1 回の立て直し（4.4）・見張り `already running`（自分のもの） | 合う |
| 3 | 番（前置きステップ 6）・`merged`・見張り `exit=3`（参加を終えた後） | 合う |
| 4 | 見張り `already running`・再開の待ち `already running`（3.10） | 合う |
| 5 | 見張り `exit=3`（今の見張り・参加の中）・`merge --wait` の 3（前置きステップ 7） | 合う |
| 6 | kiro-next の読み方 `exit=2` | 合う（今の実行ファイルは `--repo` を受けない） |
| 7 | 前置きの `skip no-home`・`no-exe`・`bad-name`（kiro-next の `skip` も） | 合う |
| 8 | 見張り `already running`（覚えが無い → 2.8 の 1 本） | 合う |

スキルの文を字のまま走らせて食い違いは出ず、スキルの書き替えは要らなかった。

## 7.3 文書の静的な確かめ（要件 8.1, 8.2, 8.3, 8.4, 9.3, 11.4）

- 実施: 2026-10-16（Git Bash と PowerShell・作業木 `C:\home\maz\git\areka\.claude\worktrees\areka-p0-watch-skills-7d4d4c`・HEAD `e88a3b19`）
- 既定の枝: `origin/HEAD -> origin/main`。分かれ目（`git merge-base HEAD origin/main`）は `1e6590d8`
- 実行ファイル `areka-impl-watch.exe` は走らせていない（読むだけの確かめ）

### 1. 机の前置きの Bash が同じであること

`kiro-impl` の前置き（`.claude/skills/kiro-impl/SKILL.md` 261〜265 行の 5 行）を基準にし、各行を `git grep -c -F` で数えた。

```bash
git grep -c -F -e "<前置きの各行>" -- .claude/skills/kiro-impl/SKILL.md .claude/skills/kiro-complete/SKILL.md .kiro/specs/areka-P0-impl-watch-skills/design.md
```

| 前置きの行（先頭） | kiro-impl | kiro-complete | design.md |
|---|---|---|---|
| `ID=$(basename "$(git rev-parse --show-toplevel)"); R=…` | 1 | 1 | 1 |
| `if [ -z "$AREKA_IMPL_WATCH_HOME" ]; then echo "desk: skip no-home"` | 1 | 1 | 2 |
| `elif [ ! -f "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" ]; then echo "desk: skip no-exe"` | 1 | 1 | 2 |
| `elif ! [[ "$ID" =~ … && "$R" =~ … ]]; then echo "desk: skip bad-name $ID $R"` | 1 | 1 | 1 |
| `else echo "desk: id=$ID repo=$R"; fi` | 1 | 1 | 1 |

- design.md の 2 は、224〜225 行（机の前置き）と 357〜358 行（kiro-next の棚卸の Bash。同じ 2 行を使う）の 2 か所。
- 5 行のまとまりとしても比べた（CR を除き、基準の 1 行目から 5 行を取り出して `diff`）: kiro-complete（110〜114 行）・design.md（223〜227 行）とも差分なし。

kiro-next の `R=` の式:

```bash
git grep -c -F -e 'R=$(basename "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")")' -- .claude/skills/kiro-impl/SKILL.md .claude/skills/kiro-complete/SKILL.md .claude/skills/kiro-next/SKILL.md .kiro/specs/areka-P0-impl-watch-skills/design.md
```

```text
.claude/skills/kiro-complete/SKILL.md:1
.claude/skills/kiro-impl/SKILL.md:1
.claude/skills/kiro-next/SKILL.md:1
.kiro/specs/areka-P0-impl-watch-skills/design.md:2
```

kiro-next（46 行）の `R=` は前置きの `R=` と同じ字（前置きでは `ID=…; ` の後ろに続く部分として一致する）。design.md の 2 は前置き（223 行）と kiro-next の Bash（356 行）。→ 合格

### 2. 調停役のフォルダと今使う文書の文（8.1・8.2・8.3・8.4）

```bash
ls -d .claude/skills/kiro-watch; git ls-files .claude/skills/kiro-watch | wc -l
```

```text
ls: cannot access '.claude/skills/kiro-watch': No such file or directory
0
```

分かれ目との差分でも `D .claude/skills/kiro-watch/SKILL.md`・`kiro-watch.ps1`・`messages.json` の 3 つが消えている。

```bash
P=(.claude/skills CLAUDE.md .kiro/steering ':!.kiro/steering/roadmap.md' ':!.kiro/steering/roadmap-history.md' doc ':!doc/roadmap-overview-*.html')
git grep -n -e '【kiro-watch】' -e 'SUSPENDED' -e '一時停止中' -- "${P[@]}"
```

出力 0 行（当たり無し）。除いたのは roadmap.md・roadmap-history.md と、生成された俯瞰の HTML（`doc/roadmap-overview-2026-10-16.html`。roadmap の中身を写した過去の記録で、外さないとここに当たる）。完了した spec・本 spec の文書・`tools/release-notes-fixture/` は範囲（`.claude/skills`・`CLAUDE.md`・`.kiro/steering`・`doc`）の外。

調停役を探す文（同じ範囲）:

```bash
git grep -n -i -e 'list_sessions' -e 'kiro-watch' -e '調停役' -e 'coordinator' -- "${P[@]}"
```

| 当たった所 | 中身 | 判定 |
|---|---|---|
| `.claude/skills/kiro-watch-clear/SKILL.md` 2・3・8・15 行 | スキル名 `kiro-watch-clear`（机の全消しのスキル。調停役ではない） | 調停役を指さない |
| `CLAUDE.md` 66 行・`.kiro/steering/structure.md` 425 行 | `kiro-watch-clear` の案内とフォルダの道筋 | 調停役を指さない |
| `doc/impl-watch.md` 41〜53 行 | 「調停役のセッションとの違い」の節。「これまでは」で始まり「替えたときの違いの記録」とする過去の表（8.6 で残すもの） | 履歴。今使うものとして指していない |
| `doc/impl-watch.md` 55 行 | 「調停役のセッションはもう使わない」 | 今は使わないことを述べる文 |
| `doc/impl-watch.md` 416 行 | スキル `/kiro-watch-clear` の案内 | 調停役を指さない |
| `.claude/skills/kiro-next/SKILL.md` 42 行 | 「the merge-coordinator memory (who is in flight)」。10-06 の手作業の合流役のメモ（記憶のファイル）を読む文で、調停役 `kiro-watch` のセッションを探す文ではない（tasks.md の Implementation Notes 5.1 に範囲外として記載済み） | 調停役のセッションを探す文ではない |

`list_sessions` の当たりは 0。→ 合格

### 3. 終わり方の表と手順書 4 節・5 節の突き合わせ（9.3）

設計書「終わり方の表」の 16 行を、`doc/impl-watch.md` の 4 節（「流れ」・「スキルからの呼び方」の「終わったときの扱い」）と 5 節（「終了コードごとにセッションがすること」・終了コード 1 の表）に 1 行ずつ当てた。

| 終わり方の表の行 | 手順書の当たる所 | 結果 |
|---|---|---|
| 見張り `exit=0`・`stop requested by <X>` → 切れ目で `stopped --wait` | 4 節 流れ 4 の 1・2（177〜178 行）・5 節 `watch` の 0（352 行） | 食い違い無し |
| 見張り `exit=0`・`already stopped; …` → 記録し直さず `resume` | 5 節 331 行（出力）・352 行 | 食い違い無し |
| 見張り `exit=1`・`AREKA_IMPL_WATCH_HOME …` → 机の段を止めて 1 行・報告に残す | 4 節 265 行・5 節 366〜368 行と 377 行（立て直さずに変数を直す） | 食い違い無し（5 節は開発者のすること、4 節がスキルの足し分） |
| 見張り `exit=1`・`a watch wait for <ID> is already running` → 自分のなら何もしない・覚えが無ければ外し方の 1 本・申し込み直し | 4 節 266〜272 行・5 節 369 行 | 食い違い無し（外し方の Bash も字が同じ） |
| 見張り `exit=1`・上のほか → 直ちに 1 回だけ立て直す・続けば机の段を止める | 4 節 264 行・5 節 377 行 | 食い違い無し |
| 見張り `exit=3` → 今の見張りでない・参加を終えた後なら何もしない・それ以外は 1 行＋立て直し＋申し込み直し | 4 節 186 行・204 行・5 節 `watch` の 3（352 行） | 食い違い無し |
| 再開の待ち `exit=0`・`resumed` → 見張りを立て直して次へ | 4 節 流れ 4 の 3（179 行）・5 節 354 行 | 食い違い無し |
| 再開の待ち `exit=0`・`stop requested again by <X>` → もう一度 `stopped --wait` | 4 節 流れ 4 の 4（180 行）・206 行・5 節 354 行 | 食い違い無し |
| 再開の待ち `exit=3` → 見張りを立て直して続ける | 5 節 `stopped --wait`・`resume` の 3（354 行） | 食い違い無し |
| 再開の待ち `exit=1`・`a resume wait … already running` → 何もしない | 5 節 370 行 | 食い違い無し |
| `merge --wait`・`loadtest --wait` `exit=1`・`already running` → 自分のなら何もしない・覚えが無ければ `cancel` → 申し込み直す → 1 行 | 5 節 371 行 | 食い違い無し |
| `loaddone` `exit=3`・`not applied: not the load-test holder` → 返し直さず 1 行伝えて続ける | 5 節 336 行（出力）・358 行（行動は要らない） | 食い違い無し（1 行は 4 節 215 行の「1 つの出来事につき 1 行」） |
| `loadrunning` `exit=3`・`not applied: the load-test desk already has a holder` → 持ち主にならず 1 行・指示を待つ | 5 節 335 行・359 行 | 食い違い無し |
| `leave`・`merged`・`loaddone`・`cancel`・`loadrunning` `exit=1`・ほか → 1 行をそのまま伝え終了コード 1 の表に従う・直らなければ机の段を止める | 4 節 214 行・261 行・5 節 362〜375 行 | 食い違い無し |
| どれでも `exit=1`・`state.lock is busy …` → 間を置かずに 1 回だけ呼び直す | 5 節 374 行 | 食い違い無し |
| どれでも `exit=2` → 机の段を止めて報告に残す | 4 節 273 行・5 節 379 行（kiro-next の `clear --repo` の 2 は 259 行で別扱い。設計書の Error Categories と同じ） | 食い違い無し |

表の下の決まり（立て直しと申し込み直しは 1 回まで・2 度目の `already running` は害なし）も 4 節 263 行と同じ。→ 16 行とも合格

### 4. `tools/encoding-check.ps1`

```bash
pwsh -NoProfile -File tools/encoding-check.ps1; echo "exit=$?"
```

```text
PASS tools/impl-watch-install.ps1
（ほかの道具・ワークフロー・cp932 の子プロセスの確かめもすべて PASS）
encoding check: all passed
exit=0
```

→ 合格

### 5. 書き替えたファイルの改行

PowerShell でファイルを丸ごと読み、`\r\n`・`\r\r`・CR の無い `\n` を数えた。

```powershell
$t = [IO.File]::ReadAllText((Resolve-Path $f)); [regex]::Matches($t,"`r`n").Count; [regex]::Matches($t,"`r`r").Count; [regex]::Matches($t,"(?<!`r)`n").Count
```

| ファイル | `git ls-files --eol` | CRLF | `\r\r` | CR の無い改行 |
|---|---|---|---|---|
| `.claude/skills/kiro-impl/SKILL.md` | `i/lf w/crlf` | 425 | 0 | 0 |
| `.claude/skills/kiro-complete/SKILL.md` | `i/lf w/crlf` | 660 | 0 | 0 |
| `.claude/skills/kiro-watch-clear/SKILL.md` | `i/lf w/crlf` | 72 | 0 | 0 |
| `doc/impl-watch.md` | `i/lf w/crlf` | 486 | 0 | 0 |
| `.kiro/steering/structure.md` | `i/lf w/crlf` | 462 | 0 | 0 |
| `tools/impl-watch-install.ps1` | `i/lf w/crlf` | 135 | 0 | 0 |
| `.claude/skills/kiro-next/SKILL.md` | `i/lf w/lf` | 0 | 0 | 157 |

- kiro-next の SKILL.md は作業木でも LF で、もとからの形である: 分かれ目 `1e6590d8` の中身にも CR が 0 個（`git show 1e6590d8:.claude/skills/kiro-next/SKILL.md | tr -cd '\r' | wc -c` → `0`）で、LF のまま混ざりが無い（CRLF 0・`\r\r` 0）。
- Git Bash の `grep -c $'\r'` は CR を落として読むため数えに使わなかった（LF の kiro-next でも行数と同じ数を返した）。

→ 合格

### 6. アプリのソースと `Cargo.lock` の差分なし（11.4）

```bash
git diff --stat $(git merge-base HEAD origin/main) HEAD -- crates/ Cargo.lock | wc -l
git diff --stat origin/main...HEAD -- crates/ Cargo.lock | wc -l
git status --porcelain -- crates/ Cargo.lock | wc -l
```

```text
0
0
0
```

- 較正: 同じ分かれ目との差分は、範囲を外すと `.claude/skills/`・`.kiro/steering/structure.md`・`doc/impl-watch.md`・`tools/impl-watch-install.ps1` の変更を出す（`git diff --name-status 1e6590d8 HEAD`）ので、比べ方は効いている。`crates/` と `Cargo.lock` はどちらも実在する道筋。

→ 合格

### まとめ

| 確かめ | 結果 |
|---|---|
| 1. 前置きの Bash が kiro-impl・kiro-complete・design.md で同じ、kiro-next の `R=` も同じ | 合格 |
| 2. 調停役のフォルダが無く、今使う文書に調停役を今使うものとして指す文が無い | 合格 |
| 3. 終わり方の表の 16 行が手順書 4 節・5 節と食い違わない | 合格 |
| 4. `tools/encoding-check.ps1` | 合格 |
| 5. 書き替えたファイルの改行（CRLF のもの 6 つ・LF のもの 1 つ） | 合格 |
| 6. `crates/` と `Cargo.lock` の差分なし | 合格 |
