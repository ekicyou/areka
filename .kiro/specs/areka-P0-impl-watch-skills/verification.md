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
