#Requires -Version 7.0
<#
================================================================================
perf-loop.selftest2.ps1 — perf-loop 側の自分の検査（T2 の 2 本目＝出力先と -Resume の束・入口の束・段③の束）
  spec: audit-tools-perf-run（要件 4.1〜4.5・5.5・design「Testing Strategy」の T2）

呼び方・出力・終了・決まりは perf-loop.selftest.ps1 と同じ（1,000 行の番人のため束ごとに分けた）:
  pwsh -NoProfile -File tools/perf/perf-loop.selftest2.ps1（引数なし）
  赤の検査だけ 'NG <検査の名前>: <理由>'、最後に 'PERF-LOOP-SELFTEST2 RESULT ok=<n> ng=<m> code=<0|1>'。
  骨組みの助けの関数は 1 本目の写し（このファイルだけで独立して回る）。
================================================================================
#>

. (Join-Path $PSScriptRoot 'perf-loop.common.ps1')
. (Join-Path $PSScriptRoot 'perf-loop.measure.ps1')

# =============================================================================
# 骨組み（perf-loop.selftest.ps1 の写し・使う分だけ）
# =============================================================================

$script:SelfTestCalls = [System.Collections.Generic.List[object]]::new()

function Add-SelfTestCall {
    param([Parameter(Mandatory = $true)][string]$Name, [object[]]$Arguments = @())
    $script:SelfTestCalls.Add([pscustomobject]@{ Name = $Name; Arguments = @($Arguments) })
}

# 終了の口の偽物: 本物の exit で検査が終わらないよう、終了コードを持った例外を投げる
function Exit-WithResult {
    param([Parameter(Mandatory = $true)][int]$Code, [string]$Sub, [string]$Dir)
    Add-SelfTestCall -Name 'Exit-WithResult' -Arguments @($Code, $Sub, $Dir)
    $ex = [System.Exception]::new("Exit-WithResult code=$Code")
    $ex.Data['ExitCode'] = $Code
    throw $ex
}

function Assert-SelfTest {
    param([bool]$Condition, [string]$Reason)
    if (-not $Condition) { throw $Reason }
}

function ConvertTo-SelfTestAscii {
    param([string]$Text)
    return (($Text -replace '\s*\r?\n\s*', ' ') -replace '[^\x20-\x7E]', '?')
}

function Get-SelfTestExitCode {
    param([Parameter(Mandatory = $true)][scriptblock]$Script)
    try { $null = & $Script }
    catch {
        if ($_.Exception.Data.Contains('ExitCode')) { return [int]$_.Exception.Data['ExitCode'] }
        throw
    }
    return $null
}

# 段③の束の偽物（検査の中で . $SelfTestRankRunFakes として読む）。部品の呼ばれた順を Add-SelfTestCall に残す。
# ランナーの答えは $script:FakeRunner の頭から 1 つずつ（最後の 1 つは使い回す）、-Stop の答えは $script:FakeStop
$SelfTestRankRunFakes = {
    $script:SelfTestProblems = [System.Collections.Generic.List[string]]::new()
    function Write-Problem { param([string]$Message) $script:SelfTestProblems.Add($Message) }
    function Write-Info { param([string]$Message) }
    function Sync-MeasureWorktreeBuild { param($Build, $BinDir) Add-SelfTestCall -Name 'build' }
    function Test-MeasureExeReady { param($BinDir, $Build) Add-SelfTestCall -Name 'check'; $true }
    function Get-SamplingProbe {
        Add-SelfTestCall -Name 'probe'
        [pscustomobject]@{ Available = $script:FakeProbeOk; Reason = $(if ($script:FakeProbeOk) { 'ok' } else { 'probe_word' }) }
    }
    function Get-MeasureGhostRoot { 'G' }
    function Get-MeasureBalloonRoot { 'B' }
    function Sync-MeasureShioriHelper { param($BinDir, $Build) }
    function Clear-MeasureDryRunDir { param($Dir) }
    function New-LoopDir { param($Path) }
    function Write-MeasureSamplingNote { param($RunDir, [bool]$Available, $Reason) Add-SelfTestCall -Name 'note' -Arguments @($Available, $Reason) }
    function Invoke-MeasureRank { param($RunDir) Add-SelfTestCall -Name 'rank' }
    function Invoke-PwshChild {
        param([string]$ScriptPath, [string[]]$Arguments, [switch]$Quiet)
        $leaf = Split-Path -Leaf $ScriptPath
        if ($leaf -eq 'invoke-perf-run.ps1') {
            Add-SelfTestCall -Name 'runner' -Arguments $Arguments
            $r = $script:FakeRunner[0]
            if ($script:FakeRunner.Count -gt 1) { $script:FakeRunner = @($script:FakeRunner | Select-Object -Skip 1) }
            return $r
        }
        if ($leaf -eq 'invoke-cpu-sample.ps1') { Add-SelfTestCall -Name "sample$($Arguments[0])" -Arguments $Arguments; return $script:FakeStop }
        if ($leaf -eq 'check-quiet.ps1') { Add-SelfTestCall -Name 'quiet-after'; return (New-ChildResult -Code $script:FakeQuiet -LastLine 'fake quiet') }
        throw "unexpected child $leaf"
    }
    $script:FakeProbeOk = $true
    $script:FakeQuiet = 0
    $script:DryRunMode = $false
    $script:ResumeMode = $false
    $script:BuildKind = 'release'
    $script:BinDirArg = ''
    $script:DateStamp = $null
    $script:GoalConfig = $null
    $script:GoalFilePath = $null
    $script:GoalName = 'selftest'
    $script:RunDirArg = Join-Path $script:SelfTestRoot 'rr-run'
}

# ランナーの偽の答え: 採取の開始の行（'<code> <reason>'・空なら行なし）と、非ゼロなら退け先の行
function New-SelfTestRunnerResult {
    param([int]$Code, [string]$Start)
    $out = @('[invoke-perf-run] fake')
    if ($Start) { $c, $w = $Start -split ' ', 2; $out += "[invoke-perf-run] sampling start: code=$c reason=$w" }
    if ($Code -ne 0) { $out += "[invoke-perf-run] moved to: $($script:RunDirArg)-FAILED" }
    return (New-ChildResult -Code $Code -Out $out -LastLine $out[-1])
}

# 呼ばれた部品の名前を順に並べる（終了の口は除く）
function Get-SelfTestCallOrder { return (@($script:SelfTestCalls | Where-Object { $_.Name -ne 'Exit-WithResult' } | ForEach-Object { $_.Name }) -join ',') }

$SelfTestChecks = @(
    # -------------------------------------------------------------------------
    # 出力先と -Resume の束（2・6・21）
    # -------------------------------------------------------------------------

    [pscustomobject]@{ Name = 'resume-incomplete'; Body = {
        # 5 つの状態の出力先: 完了は使われ、未完了・静かでないは脇へ退き（中身はそのまま）、run.log の無いものと無いものは触られない
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        $root = Join-Path $script:SelfTestRoot 'resume'
        $null = New-Item -ItemType Directory -Path $root
        $full = @{
            'run.log' = 'log'; 'cpu.csv' = 'csv'
            'run-meta.txt' = "[run]`ndry_run = False`n`n[result]`nended_utc = 2026-01-01T00:00:00.000Z`n"
            'quiet-after.txt' = "verdict=NOT_QUIET`nverdict=QUIET`n"
        }
        $cases = @(
            @{ Name = 'complete'; Over = @{}; Want = $true; Tag = $null }
            @{ Name = 'no-ended'; Over = @{ 'run-meta.txt' = "[run]`nstarted_utc = x`n" }; Want = $false; Tag = 'INCOMPLETE' }
            @{ Name = 'no-after'; Over = @{ 'quiet-after.txt' = $null }; Want = $false; Tag = 'INCOMPLETE' }
            @{ Name = 'no-verdict'; Over = @{ 'quiet-after.txt' = "mean=1.0`n" }; Want = $false; Tag = 'INCOMPLETE' }
            @{ Name = 'no-csv'; Over = @{ 'cpu.csv' = $null }; Want = $false; Tag = 'INCOMPLETE' }
            @{ Name = 'no-meta'; Over = @{ 'run-meta.txt' = $null }; Want = $false; Tag = 'INCOMPLETE' }
            @{ Name = 'not-quiet'; Over = @{ 'quiet-after.txt' = "verdict=QUIET`nverdict=NOT_QUIET`n" }; Want = $false; Tag = 'NOTQUIET' }
            @{ Name = 'no-runlog'; Over = @{ 'run.log' = $null; 'cpu.csv' = $null; 'quiet-after.txt' = $null; 'DRY-RUN.txt' = 'dry' }; Want = $false; Tag = $null }
        )
        foreach ($case in $cases) {
            $dir = Join-Path $root $case.Name
            $null = New-Item -ItemType Directory -Path $dir
            $files = $full.Clone()
            foreach ($k in $case.Over.Keys) { $files[$k] = $case.Over[$k] }
            $names = @($files.Keys | Where-Object { $null -ne $files[$_] } | Sort-Object)
            foreach ($n in $names) { [System.IO.File]::WriteAllText((Join-Path $dir $n), $files[$n]) }
            $got = Test-MeasureRunComplete -Dir $dir
            Assert-SelfTest ($got -eq $case.Want) "$($case.Name): Test-MeasureRunComplete $got (want $($case.Want))"
            $aside = @(Get-ChildItem -LiteralPath $root -Directory -Filter "$($case.Name)-*")
            $asideNames = @($aside | ForEach-Object Name) -join ','
            if ($case.Tag) {
                Assert-SelfTest (-not (Test-Path -LiteralPath $dir)) "$($case.Name): folder was not moved aside"
                Assert-SelfTest ($aside.Count -eq 1 -and $aside[0].Name -cmatch "^$([regex]::Escape($case.Name))-$($case.Tag)-\d{17}$") "$($case.Name): moved aside as '$asideNames' (want $($case.Name)-$($case.Tag)-<17 digits>)"
                $dir = $aside[0].FullName
            } else {
                Assert-SelfTest ($aside.Count -eq 0) "$($case.Name): moved aside as '$asideNames' (want untouched)"
            }
            $left = @(Get-ChildItem -LiteralPath $dir -File | ForEach-Object Name | Sort-Object)
            Assert-SelfTest (($left -join ',') -eq ($names -join ',')) "$($case.Name): files '$($left -join ',')' (want '$($names -join ',')')"
        }
        $missing = Join-Path $root 'missing'
        Assert-SelfTest (-not (Test-MeasureRunComplete -Dir $missing)) 'missing folder must not be complete'
        Assert-SelfTest (-not (Test-Path -LiteralPath $missing)) 'missing folder was created'
    } }

    [pscustomobject]@{ Name = 'dryrun-clear'; Body = {
        # -DryRun の印だけの出力先は作り直し、ほかが在れば 3 で止めて何も消さない
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        $root = Join-Path $script:SelfTestRoot 'dryrun'
        $null = New-Item -ItemType Directory -Path $root
        $cases = @(
            @{ Name = 'marks'; Files = @('DRY-RUN.txt', 'run-meta.txt'); Sub = $false; Exit = $null; Kept = $false }
            @{ Name = 'mark-only'; Files = @('DRY-RUN.txt'); Sub = $false; Exit = $null; Kept = $false }
            @{ Name = 'other-file'; Files = @('DRY-RUN.txt', 'run-meta.txt', 'perf-x.csv'); Sub = $false; Exit = 3; Kept = $true }
            @{ Name = 'subfolder'; Files = @('DRY-RUN.txt'); Sub = $true; Exit = 3; Kept = $true }
            @{ Name = 'mark-and-run'; Files = @('DRY-RUN.txt', 'run.log'); Sub = $false; Exit = 3; Kept = $true }
            @{ Name = 'real-run'; Files = @('run.log', 'run-meta.txt'); Sub = $false; Exit = $null; Kept = $true }
        )
        foreach ($case in $cases) {
            $dir = Join-Path $root $case.Name
            $null = New-Item -ItemType Directory -Path $dir
            foreach ($n in $case.Files) { [System.IO.File]::WriteAllText((Join-Path $dir $n), 'x') }
            if ($case.Sub) { $null = New-Item -ItemType Directory -Path (Join-Path $dir 'A1') }
            $code = Get-SelfTestExitCode { Clear-MeasureDryRunDir -Dir $dir }
            Assert-SelfTest ($code -eq $case.Exit) "$($case.Name): exit $code (want $($case.Exit))"
            if ($case.Kept) {
                $left = @(Get-ChildItem -LiteralPath $dir -File | ForEach-Object Name | Sort-Object)
                Assert-SelfTest (($left -join ',') -eq (@($case.Files | Sort-Object) -join ',')) "$($case.Name): files '$($left -join ',')' left (want all kept)"
            } else {
                Assert-SelfTest (-not (Test-Path -LiteralPath $dir)) "$($case.Name): marks-only folder was not recreated"
            }
        }
        $code = Get-SelfTestExitCode { Clear-MeasureDryRunDir -Dir (Join-Path $root 'missing') }
        Assert-SelfTest ($null -eq $code) "missing folder: exit $code (want none)"
        # -DryRun の rank-run は段③の札を書かない（書くと次の本番が印のほかのファイルで 3 になる）
        $script:DryRunMode = $true
        $note = Join-Path $root 'note'
        $null = New-Item -ItemType Directory -Path $note
        Write-MeasureSamplingNote -RunDir $note -Available $false -Reason 'dry_run'
        Assert-SelfTest (-not (Test-Path -LiteralPath (Join-Path $note 'sampling.txt'))) '-DryRun wrote sampling.txt'
    } }

    [pscustomobject]@{ Name = 'loopdir-vocab'; Body = {
        # 出力先の配置は Kind ごとに決まり、周の小部屋は語彙の外を受けない
        $o = @{ OutRoot = 'C:\out'; Goal = 'g' }
        $want = @(
            @{ Args = @{ Kind = 'goal' }; Path = 'C:\out\g' }
            @{ Args = @{ Kind = 'baseline'; Date = '20260101' }; Path = 'C:\out\g\baseline-20260101' }
            @{ Args = @{ Kind = 'baseline'; Date = '20260101'; Leaf = 'release' }; Path = 'C:\out\g\baseline-20260101\release' }
            @{ Args = @{ Kind = 'iter'; Iter = 2; Leaf = 'A1' }; Path = 'C:\out\g\iter-2\A1' }
            @{ Args = @{ Kind = 'final'; Date = '20260102' }; Path = 'C:\out\g\final-20260102' }
        )
        foreach ($w in $want) {
            $a = $w.Args.Clone()
            $got = Get-LoopDir @a @o
            Assert-SelfTest ($got -eq $w.Path) "Get-LoopDir $($w.Args.Kind): '$got' (want '$($w.Path)')"
        }
        Assert-SelfTest (($PERF_LOOP_ITER_LEAVES -join ',') -ceq 'rank,A1,B1,A2,B2,bin-A,bin-B,followup') "iter leaves changed: $($PERF_LOOP_ITER_LEAVES -join ',')"
        foreach ($leaf in @($PERF_LOOP_ITER_LEAVES) + @($MEASURE_SIDE_LEAF.Values)) {
            $null = Get-LoopDir -Kind 'iter' -Iter 1 -Leaf $leaf @o
        }
        foreach ($bad in @('A3', 'rank-run', 'bin-C', 'final')) {
            $threw = $false
            try { $null = Get-LoopDir -Kind 'iter' -Iter 1 -Leaf $bad @o } catch { $threw = $true }
            Assert-SelfTest $threw "iteration subfolder '$bad' outside the vocabulary was accepted"
        }
        $threw = $false
        try { $null = Get-LoopDir -Kind 'iter' -Iter 0 @o } catch { $threw = $true }
        Assert-SelfTest $threw '-Iter 0 was accepted'
        Assert-SelfTest ((Resolve-RunDir -RunDir 'C:\x\y\' -Kind 'iter') -eq 'C:\x\y') '-RunDir must be used as it is (no trailing \)'
    } }

    # -------------------------------------------------------------------------
    # 入口の束（4・25・26・27・33）。入口を子として起こすときは必ず空の -BinDir と一時の -OutRoot（-DryRun）
    # -------------------------------------------------------------------------

    [pscustomobject]@{ Name = 'case-normalize'; Body = {
        $entry = Join-Path $PSScriptRoot 'perf-loop.ps1'
        $bin = Join-Path $script:SelfTestRoot 'empty-bin'
        $null = New-Item -ItemType Directory -Path $bin -Force
        $out = Join-Path $script:SelfTestRoot 'out-case'
        $r = Invoke-PwshChild -ScriptPath $entry -Arguments @('Rank-Run', '-Build', 'Release', '-Iter', '1', '-DryRun', '-BinDir', $bin, '-OutRoot', $out) -Quiet
        $last = ($r.Out | Select-Object -Last 1)
        Assert-SelfTest ($r.Code -eq 0 -and $last -clike 'PERF-LOOP RESULT rank-run code=0 dir=*') "Rank-Run: code $($r.Code), last line: $(ConvertTo-SelfTestAscii $last)"
        $r = Invoke-PwshChild -ScriptPath $entry -Arguments @('measure-baseline', '-Build', 'Release', '-DryRun', '-BinDir', $bin, '-OutRoot', $out) -Quiet
        Assert-SelfTest ($r.Code -eq 0) "measure-baseline: code $($r.Code): $(ConvertTo-SelfTestAscii $r.LastLine)"
        Assert-SelfTest ([bool](@($r.Lines) -cmatch '/ build release /')) "measure-baseline did not use build 'release': $(ConvertTo-SelfTestAscii (@($r.Lines) -join ' | '))"
        # final の builds_final の各語も小文字にそろえる
        function Write-Info { param([string]$Message) Add-SelfTestCall -Name 'Write-Info' -Arguments @($Message) }
        function Write-Problem { param([string]$Message) }
        function Get-MeasureTomlArray { param($Section, $Key, $Default) @('Release', 'DEV') }
        function Test-MeasureExeReady { param($BinDir, $Build) Add-SelfTestCall -Name 'Test-MeasureExeReady' -Arguments @($Build); $false }
        function Sync-MeasureWorktreeBuild { param($Build, $BinDir) }
        function Write-MeasureDryRunMark { param($Dir, $Reason) }
        $script:BuildExplicit = $false
        $script:BinDirArg = ''
        $script:DateStamp = $null
        $script:GoalConfig = $null
        $script:GoalName = 'selftest'
        $script:ResumeMode = $false
        $script:RunDirArg = Join-Path $script:SelfTestRoot 'final'
        $code = Get-SelfTestExitCode { Invoke-SubFinal }
        $builds = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'Test-MeasureExeReady' } | ForEach-Object { $_.Arguments[0] })
        Assert-SelfTest ($code -eq 0 -and ($builds -join ',') -ceq 'release,dev') "final builds_final: code $code, builds '$($builds -join ',')' (want 'release,dev')"
    } }

    [pscustomobject]@{ Name = 'goal-and-goalfile'; Body = {
        $entry = Join-Path $PSScriptRoot 'perf-loop.ps1'
        $bin = Join-Path $script:SelfTestRoot 'empty-bin'
        $null = New-Item -ItemType Directory -Path $bin -Force
        $out = Join-Path $script:SelfTestRoot 'out-goal'
        $goalFile = Join-Path $PSScriptRoot 'goals\draw-load-parity.toml'
        $base = @('rank-run', '-Iter', '1', '-DryRun', '-BinDir', $bin, '-OutRoot', $out, '-GoalFile', $goalFile)
        $r = Invoke-PwshChild -ScriptPath $entry -Arguments $base -Quiet
        Assert-SelfTest ($r.Code -eq 0) "-GoalFile alone: code $($r.Code) (want 0): $(ConvertTo-SelfTestAscii $r.LastLine)"
        $r = Invoke-PwshChild -ScriptPath $entry -Arguments ($base + @('-Goal', 'draw-load-parity')) -Quiet
        Assert-SelfTest ($r.Code -eq 3) "explicit -Goal with -GoalFile: code $($r.Code) (want 3): $(ConvertTo-SelfTestAscii $r.LastLine)"
        Assert-SelfTest ([bool](@($r.Err) -match '-Goal and -GoalFile')) "no reason line naming -Goal and -GoalFile: $(ConvertTo-SelfTestAscii (@($r.Err) -join ' | '))"
    } }

    [pscustomobject]@{ Name = 'relative-paths'; Body = {
        # PowerShell の今の場所（プロセスの今の場所ではない）につなぐ
        $here = Join-Path $script:SelfTestRoot 'cwd'
        $null = New-Item -ItemType Directory -Path $here -Force
        Push-Location -LiteralPath $here
        try {
            $got = Resolve-LoopPath 'a\b'
            Assert-SelfTest ($got -eq (Join-Path $here 'a\b')) "relative: $(ConvertTo-SelfTestAscii $got)"
            $got = Resolve-LoopPath '..\c'
            Assert-SelfTest ($got -eq (Join-Path $script:SelfTestRoot 'c')) "parent: $(ConvertTo-SelfTestAscii $got)"
        } finally { Pop-Location }
        Assert-SelfTest ((Resolve-LoopPath 'C:\x\y') -eq 'C:\x\y') 'absolute path must stay as it is'
        Assert-SelfTest ([string]::IsNullOrEmpty((Resolve-LoopPath ''))) 'empty must stay empty'
        # 入口は 7 つのパスの引数をどれも一度だけ Resolve-LoopPath に通す
        $text = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot 'perf-loop.ps1'))
        foreach ($name in @('OutRoot', 'RunDir', 'BinDir', 'GhostRoot', 'BalloonRoot', 'GoalFile', 'Ledger')) {
            $n = [regex]::Matches($text, "Resolve-LoopPath\s+\`$$name\b").Count
            Assert-SelfTest ($n -eq 1) "perf-loop.ps1 passes `$$name to Resolve-LoopPath $n times (want 1)"
        }
    } }

    [pscustomobject]@{ Name = 'preflight-code'; Body = {
        $ok = @{ JudgeMatch = $true; JudgeNote = 'j'; PythonOk = $true; PythonNote = 'p'; SelftestOk = $true; SelftestNgCount = 0; GoalCheck = 'ok'; GoalCheckNote = 'g' }
        $r = Get-PreflightCode @ok
        Assert-SelfTest ($r.Code -eq 0 -and -not $r.Reason) "all ok: code $($r.Code) reason '$($r.Reason)'"
        $r = Get-PreflightCode @ok -GoalCheck 'skipped'
        Assert-SelfTest ($r.Code -eq 0) "goal-check skipped: code $($r.Code)"
        # 順: 判定スクリプトの版 -> python の版 -> 自分の検査 -> goal-check（最初に落ちたものが理由）
        $cases = @(
            @{ Over = @{ JudgeMatch = $false; PythonOk = $false; SelftestOk = $false; SelftestNgCount = 2; GoalCheck = 'ng' }; Want = 'judge' }
            @{ Over = @{ PythonOk = $false; PythonNote = 'python not found'; SelftestOk = $false; SelftestNgCount = 2; GoalCheck = 'ng' }; Want = 'python not found' }
            @{ Over = @{ PythonOk = $false; PythonNote = '3.10.4' }; Want = '3.10.4' }
            @{ Over = @{ SelftestOk = $false; SelftestNgCount = 2; GoalCheck = 'ng' }; Want = '2 red' }
            @{ Over = @{ GoalCheck = 'ng'; GoalCheckNote = 'version differs' }; Want = 'version differs' }
        )
        foreach ($case in $cases) {
            $a = $ok.Clone()
            foreach ($k in $case.Over.Keys) { $a[$k] = $case.Over[$k] }
            $r = Get-PreflightCode @a
            Assert-SelfTest ($r.Code -eq 4 -and $r.Reason.Contains($case.Want)) "want code 4 with '$($case.Want)': code $($r.Code) reason '$(ConvertTo-SelfTestAscii $r.Reason)'"
        }
    } }

    [pscustomobject]@{ Name = 'goal-text-count'; Body = {
        $report = "goal-text: 1012 `u{6587}`u{5B57}"
        $header = "# copy`n`u{672C}`u{6587}`u{306F} 1,012 `u{5B57}`n---`nbody"
        $r = Test-GoalTextCharCount -ErrText $report -MdText $header
        Assert-SelfTest $r.Ok "same count must be ok: $(ConvertTo-SelfTestAscii $r.Note)"
        $r = Test-GoalTextCharCount -ErrText 'goal-text: no count here' -MdText $header
        Assert-SelfTest (-not $r.Ok) 'count unreadable from goal-text must be red'
        $r = Test-GoalTextCharCount -ErrText ($report -replace '1012', '1013') -MdText $header
        Assert-SelfTest (-not $r.Ok) 'different count must be red'
        $r = Test-GoalTextCharCount -ErrText 'goal-text: no count here' -MdText "# copy`n---`nbody"
        Assert-SelfTest $r.Ok "no count in the copy header must be skipped (ok): $(ConvertTo-SelfTestAscii $r.Note)"
    } }

    [pscustomobject]@{ Name = 'probe-parse'; Body = {
        # -Probe の 1 行から可否・昇格・xperf の在り処を読む（在り処は行の最後で、空白と括弧を含み得る）
        $xperf = 'C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe'
        $cases = @(
            @{ Lines = @("available=true reason=ok elevated=true xperf_path=$xperf", "INVOKE-CPU-SAMPLE RESULT probe code=0 version=1.0.0 available=true reason=ok elevated=true xperf_path=$xperf"); Code = 0
               Want = @{ Available = $true; Reason = 'ok'; Elevated = $true; XperfPath = $xperf } }
            @{ Lines = @("available=false reason=not_elevated elevated=false xperf_path=$xperf"); Code = 0
               Want = @{ Available = $false; Reason = 'not_elevated'; Elevated = $false; XperfPath = $xperf } }
            @{ Lines = @('available=false reason=no_xperf elevated=true xperf_path=-'); Code = 0
               Want = @{ Available = $false; Reason = 'no_xperf'; Elevated = $true; XperfPath = $null } }
            @{ Lines = @('error: boom'); Code = 1
               Want = @{ Available = $false; Reason = 'probe_failed'; Elevated = $false; XperfPath = $null } }
        )
        foreach ($case in $cases) {
            $script:ProbeFake = $case
            function Invoke-PwshChild { param($ScriptPath, $Arguments, [switch]$Quiet)
                Add-SelfTestCall -Name 'Invoke-PwshChild' -Arguments @($Arguments)
                [pscustomobject]@{ Code = $script:ProbeFake.Code; Lines = $script:ProbeFake.Lines; Out = $script:ProbeFake.Lines; Err = @(); LastLine = $script:ProbeFake.Lines[-1] } }
            $p = Get-SamplingProbe
            foreach ($k in $case.Want.Keys) {
                Assert-SelfTest ($p.$k -eq $case.Want[$k]) "line '$(ConvertTo-SelfTestAscii $case.Lines[0])': $k = '$($p.$k)' (want '$($case.Want[$k])')"
            }
        }
        $args = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'Invoke-PwshChild' } | ForEach-Object { $_.Arguments -join ' ' })
        Assert-SelfTest (($args | Sort-Object -Unique) -join ',' -eq '-Probe') "Get-SamplingProbe passed '$($args -join ',')' (want -Probe)"
    } }

    # -------------------------------------------------------------------------
    # 段③の束（1・9・24・30）
    # -------------------------------------------------------------------------

    [pscustomobject]@{ Name = 'sampling-order'; Body = {
        # ビルド -> 確かめ -> 可否 -> ランナー（-SampleEtl 付き）-> -Stop -> sampling.txt -> 走行の後の静かさ -> 順位表。-Start は呼ばない
        . $SelfTestRankRunFakes
        $script:FakeRunner = @(New-SelfTestRunnerResult -Code 0 -Start '0 ok')
        $script:FakeStop = New-ChildResult -Code 0 -Out @('INVOKE-CPU-SAMPLE RESULT stop code=0 version=1') -LastLine 'stop ok'
        $code = Get-SelfTestExitCode { Invoke-SubRankRun }
        Assert-SelfTest ($code -eq 0) "rank-run exit $code (want 0)"
        $order = Get-SelfTestCallOrder
        Assert-SelfTest ($order -ceq 'build,check,probe,runner,sample-Stop,note,quiet-after,rank') "call order '$order'"
        $runArgs = @(@($script:SelfTestCalls | Where-Object { $_.Name -eq 'runner' })[0].Arguments)
        $i = [array]::IndexOf($runArgs, '-SampleEtl')
        Assert-SelfTest ($i -ge 0 -and $runArgs[$i + 1] -like '*-trace.etl') "runner args lack -SampleEtl <etl>: $(ConvertTo-SelfTestAscii ($runArgs -join ' '))"
        $stopArgs = @(@($script:SelfTestCalls | Where-Object { $_.Name -eq 'sample-Stop' })[0].Arguments)
        Assert-SelfTest ($stopArgs[2] -eq $runArgs[$i + 1] -and $stopArgs -notcontains '-GoalFile') "-Stop args: $(ConvertTo-SelfTestAscii ($stopArgs -join ' '))"
        # 可否が偽なら -SampleEtl を渡さず、-Stop も呼ばない
        $script:SelfTestCalls.Clear()
        $script:FakeProbeOk = $false
        $script:RunDirArg += '-b'
        $null = Get-SelfTestExitCode { Invoke-SubRankRun }
        $order = Get-SelfTestCallOrder
        Assert-SelfTest ($order -ceq 'build,check,probe,runner,note,quiet-after,rank') "unavailable: call order '$order'"
        $runArgs = @(@($script:SelfTestCalls | Where-Object { $_.Name -eq 'runner' })[0].Arguments)
        Assert-SelfTest ($runArgs -notcontains '-SampleEtl') 'unavailable: the runner got -SampleEtl'
    } }

    [pscustomobject]@{ Name = 'sampling-stop-after-run'; Body = {
        # ランナーが 0 で採取の開始を報告した道では、後で何が落ちても -Stop がランナーの直後に必ず 1 回。
        # ランナーが失敗で返った道はランナー自身が -Cancel するので perf-loop は -Stop も -Start も呼ばない
        . $SelfTestRankRunFakes
        $stopOk = New-ChildResult -Code 0 -Out @('INVOKE-CPU-SAMPLE RESULT stop code=0 version=1') -LastLine 'stop ok'
        $cases = @(
            @{ Name = 'quiet-after-3'; Runner = @('0|0 ok'); Stop = $stopOk; Quiet = 3; Exit = 3; Stops = 1; Runs = 1 }
            @{ Name = 'quiet-after-4'; Runner = @('0|0 ok'); Stop = $stopOk; Quiet = 4; Exit = 4; Stops = 1; Runs = 1 }
            @{ Name = 'retake'; Runner = @('1|0 ok', '0|0 ok'); Stop = $stopOk; Quiet = 0; Exit = 0; Stops = 1; Runs = 2 }
            @{ Name = 'stop-not-started'; Runner = @('0|0 ok'); Stop = (New-ChildResult -Code -1 -Err @('pwsh not found') -LastLine 'pwsh not found'); Quiet = 0; Exit = 4; Stops = 1; Runs = 1; Want = 'xperf -stop' }
            @{ Name = 'stop-failed'; Runner = @('0|0 ok'); Stop = (New-ChildResult -Code 1 -Err @('error: boom') -LastLine 'error: boom'); Quiet = 0; Exit = 4; Stops = 1; Runs = 1 }
            @{ Name = 'runner-failed'; Runner = @('1|0 ok'); Stop = $stopOk; Quiet = 0; Exit = 4; Stops = 0; Runs = 2 }
            @{ Name = 'not-started'; Runner = @('0|5 not_elevated'); Stop = $stopOk; Quiet = 0; Exit = 0; Stops = 0; Runs = 1 }
        )
        foreach ($case in $cases) {
            $script:SelfTestCalls.Clear()
            $script:SelfTestProblems.Clear()
            $script:RunDirArg = Join-Path $script:SelfTestRoot "ssr-$($case.Name)"
            $script:FakeRunner = @($case.Runner | ForEach-Object { $c, $st = $_ -split '\|', 2; New-SelfTestRunnerResult -Code ([int]$c) -Start $st })
            $script:FakeStop = $case.Stop
            $script:FakeQuiet = $case.Quiet
            $code = Get-SelfTestExitCode { Invoke-SubRankRun }
            Assert-SelfTest ($code -eq $case.Exit) "$($case.Name): exit $code (want $($case.Exit))"
            $names = @($script:SelfTestCalls | ForEach-Object { $_.Name })
            $stops = @($names | Where-Object { $_ -eq 'sample-Stop' }).Count
            Assert-SelfTest ($stops -eq $case.Stops) "$($case.Name): -Stop called $stops times (want $($case.Stops))"
            $runs = @($names | Where-Object { $_ -eq 'runner' }).Count
            Assert-SelfTest ($runs -eq $case.Runs) "$($case.Name): runner called $runs times (want $($case.Runs))"
            $others = @($names | Where-Object { $_ -like 'sample-*' -and $_ -ne 'sample-Stop' })
            Assert-SelfTest ($others.Count -eq 0) "$($case.Name): perf-loop called $($others -join ',')"
            if ($case.Stops) {
                $at = [array]::LastIndexOf($names, 'runner')
                Assert-SelfTest ($names[$at + 1] -eq 'sample-Stop') "$($case.Name): -Stop is not right after the runner: $($names -join ',')"
            }
            if ($case.ContainsKey('Want')) {
                $msg = @($script:SelfTestProblems) -join ' | '
                Assert-SelfTest ($msg.Contains($case.Want)) "$($case.Name): '$($case.Want)' not in: $(ConvertTo-SelfTestAscii $msg)"
            }
        }
    } }

    [pscustomobject]@{ Name = 'sampling-reason'; Body = {
        # 段③の理由の語は子の出力から: 開始はランナーの 1 行の reason=、停止は -Stop の RESULT 行の reason=（無ければ no_xperf）
        . $SelfTestRankRunFakes
        $stop5 = 'INVOKE-CPU-SAMPLE RESULT stop code=5 version=1 available=false reason=stop_word'
        $cases = @(
            @{ Name = 'start-5'; Start = '5 start_word'; Stop = $null; Want = 'False start_word' }
            @{ Name = 'start-1'; Start = '1 other_word'; Stop = $null; Want = 'False other_word' }
            @{ Name = 'no-start-line'; Start = ''; Stop = $null; Want = 'False start_failed' }
            @{ Name = 'stop-0'; Start = '0 ok'; Stop = (New-ChildResult -Code 0 -Out @('x') -LastLine 'x'); Want = 'True ok' }
            @{ Name = 'stop-5'; Start = '0 ok'; Stop = (New-ChildResult -Code 5 -Out @($stop5) -Err @('error: x') -LastLine 'error: x'); Want = 'False stop_word' }
            @{ Name = 'stop-5-bare'; Start = '0 ok'; Stop = (New-ChildResult -Code 5 -Err @('error: x') -LastLine 'error: x'); Want = 'False no_xperf' }
        )
        foreach ($case in $cases) {
            $script:SelfTestCalls.Clear()
            $script:RunDirArg = Join-Path $script:SelfTestRoot "sr-$($case.Name)"
            $script:FakeRunner = @(New-SelfTestRunnerResult -Code 0 -Start $case.Start)
            $script:FakeStop = $case.Stop
            $code = Get-SelfTestExitCode { Invoke-SubRankRun }
            Assert-SelfTest ($code -eq 0) "$($case.Name): exit $code (want 0)"
            $note = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'note' })
            Assert-SelfTest ($note.Count -eq 1) "$($case.Name): sampling note written $($note.Count) times (want 1)"
            $got = "$($note[0].Arguments[0]) $($note[0].Arguments[1])"
            Assert-SelfTest ($got -eq $case.Want) "$($case.Name): note '$got' (want '$($case.Want)')"
        }
    } }

    [pscustomobject]@{ Name = 'rank-no-record'; Body = {
        # sampling.txt も dump も無い走行は可否の試しを回さず no_record。sampling.txt が在ればその理由
        function Write-Info { param([string]$Message) }
        function Get-SamplingProbe { Add-SelfTestCall -Name 'probe'; [pscustomobject]@{ Available = $false; Reason = 'probe_word' } }
        function Get-MeasurePython { 'python' }
        function Invoke-PythonChild { param($Python, $Arguments, [switch]$Quiet) Add-SelfTestCall -Name 'rank' -Arguments $Arguments; New-ChildResult -Code 0 -LastLine 'ok' }
        $script:DryRunMode = $false
        $dir = Join-Path $script:SelfTestRoot 'rnr-run'
        $null = New-Item -ItemType Directory -Path $dir -Force
        [System.IO.File]::WriteAllText((Join-Path $dir 'run.log'), "x`n")
        foreach ($case in @(@{ Note = $null; Want = 'no_record' }, @{ Note = "available=false`nreason=note_word`n"; Want = 'note_word' })) {
            $script:SelfTestCalls.Clear()
            if ($case.Note) { [System.IO.File]::WriteAllText((Join-Path $dir 'sampling.txt'), $case.Note) }
            $null = Invoke-MeasureRank -RunDir $dir
            $probes = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'probe' }).Count
            Assert-SelfTest ($probes -eq 0) "$($case.Want): the probe ran $probes times (want 0)"
            $a = @(@($script:SelfTestCalls | Where-Object { $_.Name -eq 'rank' })[0].Arguments)
            $i = [array]::IndexOf($a, '--unavailable-reason')
            Assert-SelfTest ($i -ge 0 -and $a[$i + 1] -eq $case.Want) "rank args: $(ConvertTo-SelfTestAscii ($a -join ' ')) (want --unavailable-reason $($case.Want))"
        }
    } }
)

# =============================================================================
# 回す
# =============================================================================

$script:SelfTestRoot = Join-Path $repoRoot ('target\perf-loop-tmp\selftest-' + [Guid]::NewGuid().ToString('N'))
$okCount = 0
$ngCount = 0
$null = New-Item -ItemType Directory -Path $script:SelfTestRoot -Force
try {
    foreach ($check in $SelfTestChecks) {
        $script:SelfTestCalls.Clear()
        try {
            $null = & $check.Body
            $okCount++
        } catch {
            $ngCount++
            Write-Info "NG $($check.Name): $(ConvertTo-SelfTestAscii $_.Exception.Message)"
        }
    }
} finally {
    Remove-Item -LiteralPath $script:SelfTestRoot -Recurse -Force -ErrorAction SilentlyContinue
}

$code = if ($ngCount -eq 0) { 0 } else { 1 }
Write-Info "PERF-LOOP-SELFTEST2 RESULT ok=$okCount ng=$ngCount code=$code"
exit $code
