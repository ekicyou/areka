#Requires -Version 7.0
<#
================================================================================
invoke-perf-run.selftest.ps1 — ランナーの自分の検査（T3）
  spec: audit-tools-perf-run（要件 4.1〜4.5・design「Testing Strategy」の T3）

呼び方: invoke-perf-run.ps1 -SelfTest が静かさの関門より前で読み込み、Invoke-PerfRunSelfTest を呼ぶ
        （ランナーの関数と較正値がそのまま見える）。単独では回さない。
出力:   赤の検査だけ 1 行ずつ 'NG <検査の名前>: <理由>'（ASCII）。結果の 1 行はランナーが出す。
戻り値: 赤の数。緑の数は $script:SelfTestOk に置く。

決まり（T2 と同じ）:
  - 作業のフォルダは <リポジトリ>\target\perf-loop-tmp\selftest-<GUID>\ の下だけ。終わったら消す。
  - 本物の areka・性能カウンタ・xperf を使わない。

検査を足すには: $PerfRunSelfTestChecks に Name と Body（失敗なら throw する scriptblock）の 1 組を足す。
================================================================================
#>

function Assert-SelfTest {
    param([bool]$Condition, [string]$Reason)
    if (-not $Condition) { throw $Reason }
}

# 理由を 1 行の ASCII にする（例外の文は地域語のことがある）
function ConvertTo-SelfTestAscii {
    param([string]$Text)
    return (($Text -replace '\s*\r?\n\s*', ' ') -replace '[^\x20-\x7E]', '?')
}

# 偽の CounterSample（Path と CookedValue。InstanceName は枝番によらず素の名前＝本物と同じ落とし穴）
function New-FakeCounterSample {
    param([string]$Instance, [string]$Counter, [double]$Value)
    return [pscustomobject]@{ Path = "\\host\process($Instance)\$Counter"; InstanceName = 'areka'; CookedValue = $Value }
}

# ファイルを握って眠る pwsh を起こす（-Grandchild なら子がさらに起こした孫が握る）。
# 握った者は自分の PID を ready に書く。その Process の Handle を開いて持ち帰る（終わった後に PID が使い回されても別物を止めない）。
# 戻り: [pscustomobject]@{ Process; Ready; Held（握った者・読めなければ $null） }
function Start-SelfTestHolder {
    param([Parameter(Mandatory)][string]$Dir, [Parameter(Mandatory)][string]$Tag, [switch]$Grandchild)
    $ready = Join-Path $script:SelfTestRoot "$Tag-ready.txt"
    $q = { param($t) $t -replace "'", "''" }
    $hold = Join-Path $script:SelfTestRoot "$Tag-hold.ps1"
    [System.IO.File]::WriteAllText($hold, (@(
        "`$f = [System.IO.File]::Open('$(& $q (Join-Path $Dir 'lock.txt'))', 'Create', 'ReadWrite', 'None')"
        # ready は書き終えてから名前を付ける（読み手が空のファイルを掴まない）
        "[System.IO.File]::WriteAllText('$(& $q $ready).tmp', [string]`$PID); [System.IO.File]::Move('$(& $q $ready).tmp', '$(& $q $ready)')"
        'Start-Sleep -Seconds 60'
    ) -join "`n"), [System.Text.UTF8Encoding]::new($false))
    $entry = $hold
    if ($Grandchild) {
        $entry = Join-Path $script:SelfTestRoot "$Tag-spawn.ps1"
        [System.IO.File]::WriteAllText($entry, (@(
            '$psi = [System.Diagnostics.ProcessStartInfo]::new([Environment]::ProcessPath)'
            "foreach (`$a in @('-NoProfile', '-NonInteractive', '-File', '$(& $q $hold)')) { `$psi.ArgumentList.Add(`$a) }"
            '$psi.UseShellExecute = $false'
            '$null = [System.Diagnostics.Process]::Start($psi)'
            'Start-Sleep -Seconds 60'
        ) -join "`n"), [System.Text.UTF8Encoding]::new($false))
    }
    $psi = [System.Diagnostics.ProcessStartInfo]::new((Get-PwshPath))
    foreach ($a in @('-NoProfile', '-NonInteractive', '-File', $entry)) { $psi.ArgumentList.Add($a) }
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $process = [System.Diagnostics.Process]::Start($psi)
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while (-not (Test-Path -LiteralPath $ready) -and [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
        Start-Sleep -Milliseconds 100
    }
    $held = $null
    $heldPid = 0
    if ((Test-Path -LiteralPath $ready) -and [int]::TryParse((Get-Content -LiteralPath $ready -Raw), [ref]$heldPid)) {
        try { $held = [System.Diagnostics.Process]::GetProcessById($heldPid); $null = $held.Handle } catch { $held = $null }
    }
    return [pscustomobject]@{ Process = $process; Ready = $ready; Held = $held }
}

# 赤のときに握った者が残ってフォルダを握り続けないよう、子と、握った者（開いておいた Process）を止める
function Stop-SelfTestHolder {
    param([Parameter(Mandatory)][pscustomobject]$Holder)
    foreach ($p in @($Holder.Process, $Holder.Held)) {
        if (-not $p) { continue }
        if (-not $p.HasExited) { $p.Kill($true); $null = $p.WaitForExit(10000) }
        $p.Dispose()
    }
}

$PerfRunSelfTestChecks = @(
    [pscustomobject]@{ Name = 'counter-select'; Body = {
        # 同じ素の名前で枝番の違う 2 つ。対象（PID 200）は後ろに並ぶ areka#1
        $samples = @(
            (New-FakeCounterSample 'areka' 'id process' 100)
            (New-FakeCounterSample 'areka' '% processor time' 11.5)
            (New-FakeCounterSample 'areka#1' 'id process' 200)
            (New-FakeCounterSample 'areka#1' '% processor time' 22.5)
        )
        $v = Select-ProcessCpuValue -Samples $samples -TargetProcessId 200
        Assert-SelfTest ($v -eq 22.5) "pid 200: expected 22.5, got $v"
        $v = Select-ProcessCpuValue -Samples $samples -TargetProcessId 100
        Assert-SelfTest ($v -eq 11.5) "pid 100: expected 11.5, got $v"
        $v = Select-ProcessCpuValue -Samples $samples -TargetProcessId 300
        Assert-SelfTest ($null -eq $v) "pid 300: expected null, got $v"
        # ID は在るが CPU の値が無い枝番は選ばない
        $v = Select-ProcessCpuValue -Samples @($samples[0], $samples[1], $samples[2]) -TargetProcessId 200
        Assert-SelfTest ($null -eq $v) "pid 200 without cpu sample: expected null, got $v"
    } }

    [pscustomobject]@{ Name = 'quiet-retry-settings'; Body = {
        $text = @(
            '[other]', 'retry_max = 99', 'retry_wait_sec = 98',
            '[quiet]', 'retry_max = 5', "retry_wait_sec = 7  # comment", 'cpu_threshold = 3',
            '[after]', 'retry_wait_sec = 97'
        ) -join "`n"
        $r = Read-QuietRetrySettingsFromToml $text
        Assert-SelfTest ($r.Count -eq 2) "both keys: expected 2 keys, got $($r.Count) ($(@($r.Keys) -join ','))"
        Assert-SelfTest ($r['retry_max'] -eq 5) "retry_max: expected 5, got $($r['retry_max'])"
        Assert-SelfTest ($r['retry_wait_sec'] -eq 7) "retry_wait_sec: expected 7, got $($r['retry_wait_sec'])"
        $r = Read-QuietRetrySettingsFromToml (@('[quiet]', 'retry_max = 0', '[x]', 'retry_wait_sec = 9') -join "`n")
        Assert-SelfTest ($r.Count -eq 1 -and $r['retry_max'] -eq 0) "only retry_max = 0: got $(@($r.Keys) -join ',') / $($r['retry_max'])"
        $r = Read-QuietRetrySettingsFromToml (@('[other]', 'retry_max = 4') -join "`n")
        Assert-SelfTest ($r.Count -eq 0) "no [quiet]: expected 0 keys, got $($r.Count)"
        # 鍵は在るのに整数で読めない値は既定へ落とさず投げる
        foreach ($bad in @('retry_wait_sec = 60s', 'retry_max = 3.0')) {
            $threw = $false
            try { $null = Read-QuietRetrySettingsFromToml "[quiet]`n$bad" } catch { $threw = $true }
            Assert-SelfTest $threw "'$bad' fell back to the default"
        }
    } }

    [pscustomobject]@{ Name = 'quiet-attempt-step'; Body = {
        # 静かさの確かめの答え: 0 は進む・3 は引数・ほかの想定外（1・-1 など）は 4 と同じカウンタ（走行の後の確かめと同じ 4）・2 は retry_max + 1 回目で尽きる
        foreach ($case in @(@(0, 1, 3, 'go'), @(0, 4, 3, 'go'), @(4, 1, 3, 'counter'), @(1, 1, 3, 'counter'), @(3, 1, 3, 'bad'), @(-1, 1, 3, 'counter'),
                            @(2, 1, 3, 'wait'), @(2, 3, 3, 'wait'), @(2, 4, 3, 'unconfirmed'), @(2, 1, 0, 'unconfirmed'))) {
            $got = Get-QuietAttemptStep -Code $case[0] -Attempt $case[1] -RetryMax $case[2]
            Assert-SelfTest ($got -eq $case[3]) "code $($case[0]) attempt $($case[1]) retry_max $($case[2]): got $got (want $($case[3]))"
        }
    } }

    [pscustomobject]@{ Name = 'smoke-marker'; Body = {
        $filler = @(1..5000 | ForEach-Object { "2026-11-15T00:00:00Z INFO line $_" })
        $utf8 = [System.Text.UTF8Encoding]::new($false)
        $atEnd = Join-Path $script:SelfTestRoot 'marker-at-end.log'
        [System.IO.File]::WriteAllLines($atEnd, $filler + @('INFO ' + $LOG_MARKER_SMOKE_GATE + ' ms=420000'), $utf8)
        Assert-SelfTest (Test-SmokeMarker -Path $atEnd) 'marker on the last line not found'
        $inMiddle = Join-Path $script:SelfTestRoot 'marker-in-middle.log'
        [System.IO.File]::WriteAllLines($inMiddle, $filler + @('INFO ' + $LOG_MARKER_SMOKE_GATE) + $filler, $utf8)
        Assert-SelfTest (Test-SmokeMarker -Path $inMiddle) 'marker in the middle not found'
        $none = Join-Path $script:SelfTestRoot 'marker-none.log'
        [System.IO.File]::WriteAllLines($none, $filler, $utf8)
        Assert-SelfTest (-not (Test-SmokeMarker -Path $none)) 'marker found in a log without it'
        # 目印は字面のまま比べる（正規表現として読まない）
        Assert-SelfTest (-not (Test-SmokeMarker -Path $none -Marker 'line.5000$')) 'marker read as a regex'
    } }
    [pscustomobject]@{ Name = 'failed-dir-name'; Body = {
        $parent = Join-Path $script:SelfTestRoot 'names'
        $null = New-Item -ItemType Directory -Path $parent
        $n = Get-FailedDirName -Parent $parent -Leaf 'run'
        Assert-SelfTest ($n -eq 'run-FAILED') "nothing there: expected run-FAILED, got $n"
        $null = New-Item -ItemType Directory -Path (Join-Path $parent 'run-FAILED')
        $n = Get-FailedDirName -Parent $parent -Leaf 'run'
        Assert-SelfTest ($n -match '^run-FAILED-\d{9}$') "run-FAILED there: expected run-FAILED-<time>, got $n"
        Assert-SelfTest (-not (Test-Path -LiteralPath (Join-Path $parent $n))) "the second name $n already exists"
    } }

    [pscustomobject]@{ Name = 'move-after-kill'; Body = {
        # 子が握る場合と、子が起こした孫が握る場合。強制終了して待った後、Move-FailedOutDir が名前を替えられる
        foreach ($grand in @($false, $true)) {
            $tag = if ($grand) { 'grandchild' } else { 'child' }
            $dir = Join-Path $script:SelfTestRoot "out-$tag"
            $null = New-Item -ItemType Directory -Path $dir
            $holder = Start-SelfTestHolder -Dir $dir -Tag $tag -Grandchild:$grand
            try {
                Assert-SelfTest (Test-Path -LiteralPath $holder.Ready) "${tag}: the holder did not start holding the file"
                $script:ArekaNotStopped = $false
                Stop-ArekaProcess -Process $holder.Process
                Assert-SelfTest (-not $script:ArekaNotStopped) "${tag}: Stop-ArekaProcess reported a timeout"
                Assert-SelfTest $holder.Process.HasExited "${tag}: Stop-ArekaProcess returned before the process exited"
                try { $moved = Move-FailedOutDir -Path $dir }
                catch { throw "${tag}: folder still held after Stop-ArekaProcess: $($_.Exception.Message)" }
                Assert-SelfTest ($moved -eq "$dir-FAILED" -and (Test-Path -LiteralPath $moved) -and -not (Test-Path -LiteralPath $dir)) "${tag}: moved to '$moved'"
            } finally {
                Stop-SelfTestHolder -Holder $holder
            }
        }
    } }

    [pscustomobject]@{ Name = 'sampling-cancel-on-stop'; Body = {
        $realChild = ${function:Invoke-PwshChild}
        # 偽の段③の採取: 呼ばれた引数を残し、決まった結果を返す
        $calls = [System.Collections.Generic.List[string]]::new()
        $info = [System.Collections.Generic.List[string]]::new()
        $script:FakeSample = @{ Code = 0; Out = @('INVOKE-CPU-SAMPLE RESULT start code=0 version=x etl=e') }
        function Invoke-PwshChild { param([string]$ScriptPath, [string[]]$Arguments, [switch]$Quiet)
            $calls.Add("$(Split-Path -Leaf $ScriptPath) $($Arguments -join ' ')")
            return [pscustomobject]@{ Code = $script:FakeSample.Code; Out = $script:FakeSample.Out; Err = @(); LastLine = 'x' } }
        function Write-Info { param([string]$Message) $info.Add($Message) }
        $problems = [System.Collections.Generic.List[string]]::new()
        function Write-Problem { param([string]$Message) $problems.Add($Message) }
        $etl = Join-Path $script:SelfTestRoot 'cpu.etl'

        # 印が立っていなければ（-SampleEtl なし・始められなかった）取り消さない
        $script:SamplingEtl = ''
        Stop-RunSampling
        Assert-SelfTest ($calls.Count -eq 0) "no mark: expected no call, got '$($calls -join ' | ')'"
        foreach ($case in @(
                @{ Code = 5; Out = @('INVOKE-CPU-SAMPLE RESULT start code=5 version=x available=false reason=not_elevated'); Reason = 'not_elevated' }
                @{ Code = 1; Out = @('INVOKE-CPU-SAMPLE RESULT start code=1 version=x'); Reason = 'start_failed' })) {
            $script:FakeSample = $case; $calls.Clear(); $info.Clear()
            Start-RunSampling -Etl $etl
            Stop-RunSampling
            Assert-SelfTest ($calls.Count -eq 1 -and $calls[0] -eq "invoke-cpu-sample.ps1 -Start -Etl $etl") "start code $($case.Code): calls '$($calls -join ' | ')'"
            Assert-SelfTest ($info -contains "[invoke-perf-run] sampling start: code=$($case.Code) reason=$($case.Reason)") "start code $($case.Code): line '$($info -join ' | ')'"
            # 語の無い失敗は子の訳の行を 1 行出す（語の有る 5 は出さない）
            Assert-SelfTest (($problems.Count -gt 0) -eq ($case.Reason -eq 'start_failed')) "start code $($case.Code): $($problems.Count) reason lines"
            $problems.Clear()
        }

        # 始められたら印が立ち、取り消しは -Cancel を 1 回だけ呼んで印を下ろす
        $script:FakeSample = @{ Code = 0; Out = @('INVOKE-CPU-SAMPLE RESULT start code=0 version=x etl=e') }
        $calls.Clear(); $info.Clear()
        Start-RunSampling -Etl $etl
        Assert-SelfTest ($info -contains '[invoke-perf-run] sampling start: code=0 reason=ok') "start ok: line '$($info -join ' | ')'"
        Stop-RunSampling
        Stop-RunSampling
        Assert-SelfTest ($calls.Count -eq 2 -and $calls[1] -eq "invoke-cpu-sample.ps1 -Cancel -Etl $etl") "mark set: calls '$($calls -join ' | ')'"

        # 配線: Stop-Run は出力先を退ける前に取り消し、採取を始めるのは -SampleEtl のときだけ
        $ast = [System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'invoke-perf-run.ps1'), [ref]$null, [ref]$null)
        $cmds = @($ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] }, $true))
        $stopRun = $ast.Find({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Stop-Run' }, $true)
        $inStopRun = @($cmds | Where-Object { $_.Extent.StartOffset -ge $stopRun.Extent.StartOffset -and $_.Extent.EndOffset -le $stopRun.Extent.EndOffset })
        $cancelAt = @($inStopRun | Where-Object { $_.GetCommandName() -eq 'Stop-RunSampling' } | ForEach-Object { $_.Extent.StartOffset })
        $moveAt = @($inStopRun | Where-Object { $_.GetCommandName() -eq 'Move-FailedOutDir' } | ForEach-Object { $_.Extent.StartOffset })
        Assert-SelfTest ($cancelAt.Count -eq 1 -and $moveAt.Count -eq 1 -and $cancelAt[0] -lt $moveAt[0]) "Stop-Run: Stop-RunSampling at '$cancelAt', Move-FailedOutDir at '$moveAt'"
        $starts = @($cmds | Where-Object { $_.GetCommandName() -eq 'Start-RunSampling' })
        Assert-SelfTest ($starts.Count -eq 1) "Start-RunSampling called $($starts.Count) times in the runner (want 1)"
        $guard = $starts[0].Parent
        while ($guard -and -not ($guard -is [System.Management.Automation.Language.IfStatementAst])) { $guard = $guard.Parent }
        Assert-SelfTest ($guard -and $guard.Clauses[0].Item1.Extent.Text -eq '$SampleEtl') 'Start-RunSampling is not under if ($SampleEtl)'

        # -SampleEtl と -DryRun は一緒に使えない（3・出力先を作る前に止まる）。ほかの引数は -DryRun が 0 で通る形
        $bin = Join-Path $script:SelfTestRoot 'bin'
        $ghost = Join-Path $script:SelfTestRoot 'ghost'
        $null = New-Item -ItemType Directory -Path (Join-Path $ghost 'ghost\master'), $bin
        $null = New-Item -ItemType File -Path (Join-Path $ghost 'ghost\master\descript.txt'), (Join-Path $bin 'areka.exe')
        $out = Join-Path $script:SelfTestRoot 'dry-out'
        $r = & $realChild -ScriptPath (Join-Path $PSScriptRoot 'invoke-perf-run.ps1') -Quiet -Arguments @('-DryRun', '-SampleEtl', $etl,
            '-OutDir', $out, '-Profile', 'short', '-Build', 'dev', '-BinDir', $bin, '-GhostRoot', $ghost, '-BalloonRoot', $bin)
        Assert-SelfTest ($r.Code -eq 3 -and -not (Test-Path -LiteralPath $out) -and $r.LastLine -match '-SampleEtl') "-DryRun -SampleEtl: exit code $($r.Code) (want 3), out dir made: $(Test-Path -LiteralPath $out)"
    } }
)

function Invoke-PerfRunSelfTest {
    $repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $script:SelfTestRoot = Join-Path $repo ('target\perf-loop-tmp\selftest-' + [Guid]::NewGuid().ToString('N'))
    $script:SelfTestOk = 0
    $ng = 0
    $null = New-Item -ItemType Directory -Path $script:SelfTestRoot -Force
    try {
        foreach ($check in $PerfRunSelfTestChecks) {
            try {
                $null = & $check.Body
                $script:SelfTestOk++
            } catch {
                $ng++
                Write-Info "NG $($check.Name): $(ConvertTo-SelfTestAscii $_.Exception.Message)"
            }
        }
    } finally {
        Remove-Item -LiteralPath $script:SelfTestRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    return $ng
}
