#Requires -Version 7.0
<#
================================================================================
load-flake.ps1 — CPU の負荷をかけて areka のテストを回し、回ごとの赤を記録する
  spec: areka-P0-ghost-session-test-load-flake（要件 1.1〜1.3・7.2・design「LoadRepro」）

何をするか（design「LoadRepro」の進み方 ⑴〜⑸）:
  ⑴ 負荷を起こす前に `cargo test -p areka --bin areka --no-run` で実行ファイルを作る
     （失敗したら負荷を起こさずに summary.txt へ書いて終わる）
  ⑵ マシン全体の CPU（`\Processor(_Total)\% Processor Time`）を 5 秒採って記録する
  ⑶ CPU を回すだけの子（pwsh の空の繰り返し）を -Burners 本起こし、番号を控え、
     負荷の最中の CPU をもう一度 5 秒採る
  ⑷ 回ごとに `cargo test -p areka --bin areka -- <Filter> [--test-threads N] [--nocapture]` を回し、
     標準出力と標準エラーを 1 つのファイル round-N.log へバイトのまま保存する
     （cmd の `> file 2>&1`。コンソールの文字コードを通さない）。
     -RoundTimeoutMin を越えたら、その回に自分で起こしたプロセスの木だけを止めて「上限越え」と書く
  ⑸ finally で、控えた負荷の子だけを番号で止める（名前で探して止めない）。
     止めた後に 1 つも残っていないことを確かめて summary.txt に書く

出力（すべてワークツリーの target\load-flake\<Label>-<yyyyMMdd-HHmmss>\ の下だけ）:
  conditions.txt  引数・論理 CPU の数・コミット・負荷の前と最中の CPU・負荷の子の番号
  build.log       ⑴ の cargo の出力
  round-N.log     回ごとの cargo test の出力（標準出力と標準エラーを合わせたもの）
  summary.txt     回ごとの終了コード・所要時間・上限越え・`test result:` の行と本数・
                  赤のテストの名前・`待ちの打ち切り` の行・`sample-ghost-kit:` を含む
                  後片付けの失敗の行（os error 5 の数）と、全回のまとめ（中央値・最大の所要時間）
  ファイルは UTF-8 で書く。端末へ出す文は ASCII だけ（どのコードページでも同じに読める）。

-Burners 0 は負荷なし。静かな机の所要時間の測定（design「Performance」）に使う。

-NoCapture は libtest に `--nocapture` を渡す。libtest は緑のテストの標準エラーを捨てるので、
これが無いと `sample-ghost-kit:` の後片付けの失敗は赤のテストの分しか残らない（要件 5.1 は
赤に依らず数える）。付けても赤の名前は末尾の `failures:` の一覧から取れる（失敗の文言は
`---- 名前 stdout ----` の囲みでなく、その場の panic の行として出る）。直す前と後で揃えること。

終了コード:
  0 … 手順が最後まで走った（テストが赤でも 0。赤は summary.txt で読む）
  2 … ⑴ のビルドに失敗した
  3 … 負荷の子を起こせなかった
  1 … その他、スクリプト自身の失敗
  どの場合も summary.txt を書く。

使い方:
  pwsh -NoProfile -File tools/load-flake.ps1 -Label before -NoCapture
  pwsh -NoProfile -File tools/load-flake.ps1 -Label smoke -Rounds 1 -Burners 2 -Filter some::test
  pwsh -NoProfile -File tools/load-flake.ps1 -Label quiet -Burners 0 -Rounds 3
================================================================================
#>

[CmdletBinding()]
param(
    # 出力のフォルダの名前（before／after／任意）。ファイル名に使える文字だけ
    [Parameter(Mandatory)][ValidatePattern('^[A-Za-z0-9._-]+$')][string]$Label,
    [ValidateRange(1, 1000)][int]$Rounds = 5,
    # 既定は論理 CPU の数 × 2。0 なら負荷なし
    [ValidateRange(0, 4096)][int]$Burners = -1,
    # テストの名前の絞り込み（空なら実行ファイルの全部）
    [string]$Filter = '',
    # --test-threads にそのまま渡す（0 なら渡さない）
    [ValidateRange(0, 4096)][int]$TestThreads = 0,
    [ValidateRange(1, 1440)][int]$RoundTimeoutMin = 30,
    # --nocapture を渡す（緑のテストの標準エラー＝`sample-ghost-kit:` の後片付けの失敗も round-N.log に残す）
    [switch]$NoCapture
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# 待ちの部品の打ち切りの文言の頭「待ちの打ち切り」（tools の字句は ASCII だけ・encoding-check の規則 A）
$WaitCutPrefix = "`u{5F85}`u{3061}`u{306E}`u{6253}`u{3061}`u{5207}`u{308A}"

$COUNTER_PATH_MACHINE_CPU = '\Processor(_Total)\% Processor Time'
$CPU_SAMPLE_SEC = 5
$UTF8 = [System.Text.UTF8Encoding]::new($false)

$logicalCpus = [Environment]::ProcessorCount
if ($Burners -lt 0) { $Burners = $logicalCpus * 2 }

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$outDir = Join-Path $repoRoot ("target\load-flake\{0}-{1}" -f $Label, (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# 端末へは ASCII だけ（例外の文言など ASCII 以外は ? に置き換える。全文は summary.txt に残る）
function Say([string]$msg) { Write-Host ("[load-flake] " + ($msg -replace '[^\x20-\x7E]', '?')) }

function Add-Text([string]$file, [string[]]$lines) {
    [IO.File]::AppendAllLines((Join-Path $outDir $file), $lines, $UTF8)
}

# cmd の `> file 2>&1` で子の出力をバイトのまま 1 つのファイルへ。返すのは cmd の Process
function Start-Logged([string]$cargoArgs, [string]$logPath) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new('cmd.exe')
    $psi.Arguments = "/d /s /c `"cargo $cargoArgs > `"$logPath`" 2>&1`""
    $psi.WorkingDirectory = $repoRoot
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    return [System.Diagnostics.Process]::Start($psi)
}

# 自分で起こしたプロセスの木だけを番号で止める
function Stop-Tree([System.Diagnostics.Process]$p) {
    if ($null -eq $p -or $p.HasExited) { return }
    & taskkill.exe /T /F /PID $p.Id *> $null
    $null = $p.WaitForExit(30000)
}

function Measure-Cpu {
    try {
        $set = Get-Counter -Counter $COUNTER_PATH_MACHINE_CPU -SampleInterval 1 -MaxSamples ($CPU_SAMPLE_SEC + 1) -ErrorAction Stop
        # 1 本目は前回からの差分が無く無意味なので捨てる（check-quiet.ps1 と同じ）
        $v = @($set | Select-Object -Skip 1 | ForEach-Object { [double]$_.CounterSamples[0].CookedValue })
        $s = $v | Measure-Object -Average -Maximum
        return ('avg {0:F1}% max {1:F1}% ({2} samples)' -f $s.Average, $s.Maximum, $v.Count)
    } catch {
        return "n/a ($($_.Exception.Message))"
    }
}

# Measure-Object -Sum は 0 件だと Sum が無く StrictMode で落ちるので自前で足す
function Get-Sum($xs) { $t = 0; foreach ($x in $xs) { $t += $x }; return $t }

function Get-Median([double[]]$xs) {
    if ($xs.Count -eq 0) { return 0 }
    $s = @($xs | Sort-Object)
    $m = [int][math]::Floor($s.Count / 2)
    if ($s.Count % 2) { return $s[$m] }
    return ($s[$m - 1] + $s[$m]) / 2
}

# round-N.log を UTF-8 で読み、まとめに要るものを取り出す
function Read-Round([string]$logPath) {
    $lines = if (Test-Path $logPath) { [IO.File]::ReadAllLines($logPath, $UTF8) } else { @() }
    $r = [ordered]@{ ResultLines = @(); Passed = 0; Failed = 0; FailedNames = @(); WaitLines = @(); CleanupLines = @(); OsError5 = 0 }
    $inFailList = $false
    $names = [System.Collections.Generic.List[string]]::new()
    foreach ($l in $lines) {
        if ($l -match '^test result: \w+\. (\d+) passed; (\d+) failed;') {
            $r.ResultLines += $l; $r.Passed += [int]$Matches[1]; $r.Failed += [int]$Matches[2]
        }
        if ($l -match '^test (\S+) \.\.\. FAILED') { $names.Add($Matches[1]) }
        # 末尾の `failures:` の名前の一覧（4 字下げの 1 語の行）
        if ($l -eq 'failures:') { $inFailList = $true; continue }
        if ($inFailList) {
            if ($l -match '^    (\S+)$') { $names.Add($Matches[1]) }
            elseif ($l -ne '') { $inFailList = $false }
        }
        if ($l.Contains($WaitCutPrefix)) { $r.WaitLines += $l }
        # 行の途中も探す（--nocapture では他のテストの出力が行の頭に割り込む）
        if ($l.Contains('sample-ghost-kit:')) {
            $r.CleanupLines += $l
            if ($l.Contains('os error 5')) { $r.OsError5++ }
        }
    }
    $r.FailedNames = @($names | Sort-Object -Unique)
    return $r
}

$exitCode = 0
$status = 'ok'
$burnerProcs = [System.Collections.Generic.List[System.Diagnostics.Process]]::new()
$roundResults = [System.Collections.Generic.List[object]]::new()
$current = $null
$startedAt = Get-Date

$testArgs = '-p areka --bin areka --'
if ($Filter) { $testArgs += " `"$Filter`"" }
if ($TestThreads -gt 0) { $testArgs += " --test-threads $TestThreads" }
if ($NoCapture) { $testArgs += ' --nocapture' }

$commit = (& git -C $repoRoot rev-parse HEAD 2>$null) -join ''
$dirty = if ((& git -C $repoRoot status --porcelain 2>$null)) { 'yes' } else { 'no' }

Add-Text 'conditions.txt' @(
    "label: $Label"
    "rounds: $Rounds"
    "burners: $Burners"
    "filter: $(if ($Filter) { $Filter } else { '(none: whole binary)' })"
    "test_threads: $(if ($TestThreads -gt 0) { $TestThreads } else { '(not passed)' })"
    "round_timeout_min: $RoundTimeoutMin"
    "nocapture: $(if ($NoCapture) { 'yes' } else { 'no' })"
    "command: cargo test $testArgs"
    "logical_cpus: $logicalCpus"
    "commit: $commit (dirty: $dirty)"
    "started: $($startedAt.ToString('yyyy-MM-dd HH:mm:ss'))"
)
Say "out: $outDir"

try {
    # ⑴ 負荷の前に実行ファイルを作る
    Say 'build: cargo test -p areka --bin areka --no-run'
    $b = Start-Logged 'test -p areka --bin areka --no-run' (Join-Path $outDir 'build.log')
    $b.WaitForExit()
    if ($b.ExitCode -ne 0) {
        $status = "build failed (exit $($b.ExitCode)); see build.log"
        $exitCode = 2
        throw $status
    }

    # ⑵ 負荷の前の CPU
    $cpuBefore = Measure-Cpu
    Add-Text 'conditions.txt' "cpu_before_load: $cpuBefore"
    Say "cpu before load: $cpuBefore"

    # ⑶ 負荷の子を起こし、番号を控える
    if ($Burners -gt 0) {
        $pwshExe = (Get-Process -Id $PID).Path
        try {
            for ($i = 0; $i -lt $Burners; $i++) {
                $burnerProcs.Add((Start-Process -FilePath $pwshExe -ArgumentList '-NoProfile', '-NonInteractive', '-Command', 'while($true){}' -WindowStyle Hidden -PassThru))
            }
        } catch {
            $status = "burner start failed after $($burnerProcs.Count) of ${Burners}: $($_.Exception.Message)"
            $exitCode = 3
            throw $status
        }
        Add-Text 'conditions.txt' "burner_pids: $(($burnerProcs | ForEach-Object Id) -join ',')"
        Start-Sleep -Seconds 1
        $cpuDuring = Measure-Cpu
        Add-Text 'conditions.txt' "cpu_during_load: $cpuDuring"
        Say "started $Burners burners; cpu during load: $cpuDuring"
    } else {
        Add-Text 'conditions.txt' @('burner_pids: (none)', 'cpu_during_load: (no load)')
    }

    # ⑷ 回ごとに回す
    for ($n = 1; $n -le $Rounds; $n++) {
        $log = Join-Path $outDir "round-$n.log"
        Say "round $n/${Rounds}: cargo test $testArgs"
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $current = Start-Logged "test $testArgs" $log
        $timedOut = -not $current.WaitForExit($RoundTimeoutMin * 60 * 1000)
        if ($timedOut) { Stop-Tree $current }
        $current.WaitForExit()
        $sw.Stop()
        $r = Read-Round $log
        $r.Round = $n
        $r.ExitCode = if ($timedOut) { 'killed' } else { $current.ExitCode }
        $r.WallSec = [math]::Round($sw.Elapsed.TotalSeconds, 1)
        $r.TimedOut = $timedOut
        $roundResults.Add([pscustomobject]$r)
        $current = $null
        Say ("round {0}: exit {1}, {2}s, passed {3}, failed {4}, wait-failure lines {5}, cleanup lines {6}{7}" -f `
                $n, $r.ExitCode, $r.WallSec, $r.Passed, $r.Failed, $r.WaitLines.Count, $r.CleanupLines.Count, $(if ($timedOut) { ', TIMED OUT' } else { '' }))
    }
} catch {
    if ($exitCode -eq 0) { $exitCode = 1; $status = "script failed: $($_.Exception.Message)" }
    Say "error: $status"
} finally {
    # 途中で止められた回の木（自分で起こしたもの）
    Stop-Tree $current
    # ⑸ 控えた負荷の子だけを番号で止める
    foreach ($p in $burnerProcs) {
        if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
    }
    foreach ($p in $burnerProcs) { $null = $p.WaitForExit(10000) }
    $remaining = @($burnerProcs | Where-Object { -not $_.HasExited } | ForEach-Object Id)
    if ($burnerProcs.Count -gt 0) { Say "stopped burners; remaining: $($remaining.Count)" }
    if ($remaining.Count -gt 0 -and $exitCode -eq 0) { $exitCode = 1; $status = "burners still running: $($remaining -join ',')" }

    # summary.txt
    $s = [System.Collections.Generic.List[string]]::new()
    $s.Add("load-flake summary: $Label")
    $s.Add("status: $status")
    $s.Add("started: $($startedAt.ToString('yyyy-MM-dd HH:mm:ss'))  finished: $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))")
    $s.Add("burners: $($burnerProcs.Count) started (pids: $(if ($burnerProcs.Count) { ($burnerProcs | ForEach-Object Id) -join ',' } else { 'none' })); remaining after stop: $($remaining.Count)$(if ($remaining.Count) { ' (' + ($remaining -join ',') + ')' })")
    foreach ($r in $roundResults) {
        $s.Add('')
        $s.Add("== round $($r.Round) ==")
        $s.Add("exit_code: $($r.ExitCode)")
        $s.Add("wall_sec: $($r.WallSec)")
        $s.Add("timed_out: $(if ($r.TimedOut) { "yes (> $RoundTimeoutMin min)" } else { 'no' })")
        $s.Add("test_result: $(if ($r.ResultLines.Count) { $r.ResultLines -join ' | ' } else { '(no test result line)' })")
        $s.Add("passed: $($r.Passed)  failed: $($r.Failed)")
        $s.Add("failed_tests: $($r.FailedNames.Count)")
        $r.FailedNames | ForEach-Object { $s.Add("  $_") }
        $s.Add("wait_cut_lines: $($r.WaitLines.Count)")
        $r.WaitLines | ForEach-Object { $s.Add("  $_") }
        $s.Add("cleanup_lines (sample-ghost-kit:): $($r.CleanupLines.Count)  os_error_5: $($r.OsError5)")
        $r.CleanupLines | ForEach-Object { $s.Add("  $_") }
    }
    $walls = @($roundResults | ForEach-Object { [double]$_.WallSec })
    $redRounds = @($roundResults | Where-Object { $_.TimedOut -or $_.ExitCode -ne 0 }).Count
    $s.Add('')
    $s.Add('== aggregate ==')
    $s.Add("rounds_run: $($roundResults.Count) / $Rounds")
    $s.Add("rounds_red: $redRounds")
    $s.Add("rounds_timed_out: $(@($roundResults | Where-Object TimedOut).Count)")
    $s.Add("passed_total: $(Get-Sum ($roundResults | ForEach-Object Passed))  failed_total: $(Get-Sum ($roundResults | ForEach-Object Failed))")
    $s.Add("wait_failure_lines_total: $(Get-Sum ($roundResults | ForEach-Object { $_.WaitLines.Count }))")
    $s.Add("cleanup_lines_total: $(Get-Sum ($roundResults | ForEach-Object { $_.CleanupLines.Count }))  os_error_5_total: $(Get-Sum ($roundResults | ForEach-Object OsError5))")
    $s.Add("wall_sec: median $(Get-Median $walls)  max $(if ($walls.Count) { ($walls | Sort-Object)[-1] } else { 0 })  all [$($walls -join ', ')]")
    $s.Add('failed_tests_by_name (rounds):')
    $roundResults | ForEach-Object { $_.FailedNames } | Group-Object | Sort-Object Count -Descending |
        ForEach-Object { $s.Add("  $($_.Count)  $($_.Name)") }
    [IO.File]::WriteAllLines((Join-Path $outDir 'summary.txt'), $s, $UTF8)
    Say "summary: $(Join-Path $outDir 'summary.txt') (status: $(if ($exitCode) { "exit $exitCode" } else { 'ok' }), red rounds: $redRounds)"
}
exit $exitCode
