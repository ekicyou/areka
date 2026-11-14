#Requires -Version 7.0
<#
================================================================================
perf-loop.selftest.ps1 — perf-loop 側の自分の検査（T2）
  spec: audit-tools-perf-run（要件 4.1〜4.5・5.5・design「Testing Strategy」の T2）

呼び方: pwsh -NoProfile -File tools/perf/perf-loop.selftest.ps1（引数なし）
        perf-loop.ps1 selftest が SELFTEST_TOOLS の 1 本として Invoke-PwshChild で回す。
出力:   赤の検査だけ 1 行ずつ 'NG <検査の名前>: <理由>'、最後に
        'PERF-LOOP-SELFTEST RESULT ok=<n> ng=<m> code=<0|1>'（全部 ASCII）。
終了:   0＝全部緑・1＝1 件でも赤。

決まり:
  - perf-loop.common.ps1 と perf-loop.measure.ps1 を読み込み、その関数をそのまま試す。
  - 偽物は読み込んだ後で同じ名前の関数を定義し直して作り、呼ばれた順と引数を
    Add-SelfTestCall で $script:SelfTestCalls に残す（検査ごとに空にする）。
  - Exit-WithResult は定義し直してあり、本物の exit の代わりに終了コードを持った例外を投げる。
  - 作業のフォルダは <リポジトリ>\target\perf-loop-tmp\selftest-<GUID>\ の下だけ。終わったら消す。
  - 本物の cargo・python・xperf を呼ばない。子として起こすのは pwsh だけ。

検査を足すには: $SelfTestChecks に Name と Body（失敗なら throw する scriptblock）の 1 組を足す。
1 ファイル 1,000 行の番人に近づいたら（約 950 行）、束ごとに perf-loop.selftest2.ps1 へ分ける。
================================================================================
#>

. (Join-Path $PSScriptRoot 'perf-loop.common.ps1')
. (Join-Path $PSScriptRoot 'perf-loop.measure.ps1')

# =============================================================================
# 骨組み（偽物の記録・終了の口・作業のフォルダ・判定）
# =============================================================================

$script:SelfTestCalls = [System.Collections.Generic.List[object]]::new()

# 偽物が呼ばれた順と引数を残す
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

# 理由を 1 行の ASCII にする（例外の文は地域語のことがある）
function ConvertTo-SelfTestAscii {
    param([string]$Text)
    return (($Text -replace '\s*\r?\n\s*', ' ') -replace '[^\x20-\x7E]', '?')
}

# 子の pwsh に回させる .ps1 を作業のフォルダに書く（BOM の無い UTF-8）
function New-SelfTestScript {
    param([Parameter(Mandatory = $true)][string]$Name, [Parameter(Mandatory = $true)][string]$Text)
    $path = Join-Path $script:SelfTestRoot $Name
    [System.IO.File]::WriteAllText($path, $Text, [System.Text.UTF8Encoding]::new($false))
    return $path
}

# 偽の Exit-WithResult が投げた終了コードを返す（終わらなければ $null）
function Get-SelfTestExitCode {
    param([Parameter(Mandatory = $true)][scriptblock]$Script)
    try { $null = & $Script }
    catch {
        if ($_.Exception.Data.Contains('ExitCode')) { return [int]$_.Exception.Data['ExitCode'] }
        throw
    }
    return $null
}

#: 頭に trap を持つ入口 5 本
$SelfTestEntryScripts = @('perf-loop.ps1', 'invoke-perf-run.ps1', 'invoke-cpu-sample.ps1', 'check-quiet.ps1', 'invoke-followup-checks.ps1')

# 入口の写しを作業のフォルダの tools\perf\ に置き、最後の関数の定義の直後に throw を差し込む
# （trap が受け止める想定外の失敗。areka もカウンタも触る前に落ちる）
function New-SelfTestBrokenCopy {
    param([Parameter(Mandatory = $true)][string]$Name)
    $dir = Join-Path $script:SelfTestRoot 'tools\perf'
    if (-not (Test-Path -LiteralPath $dir)) {
        $null = New-Item -ItemType Directory -Path $dir
        Copy-Item -Path (Join-Path $PSScriptRoot '*.ps1') -Destination $dir
    }
    $text = [System.IO.File]::ReadAllText((Join-Path $PSScriptRoot $Name))
    $ast = [System.Management.Automation.Language.Parser]::ParseInput($text, [ref]$null, [ref]$null)
    $lastFn = @($ast.EndBlock.Statements | Where-Object { $_ -is [System.Management.Automation.Language.FunctionDefinitionAst] })[-1]
    $at = $lastFn.Extent.EndOffset
    $path = Join-Path $dir $Name
    $broken = $text.Substring(0, $at) + "`nthrow 'injected failure'`n" + $text.Substring($at)
    [System.IO.File]::WriteAllText($path, $broken, [System.Text.UTF8Encoding]::new($false))
    return $path
}

# =============================================================================
# 土台の束
# =============================================================================

$SelfTestChecks = @(
    [pscustomobject]@{ Name = 'load-standalone'; Body = {
        # 新しい pwsh で common だけを読み込み、Invoke-Child と Invoke-PwshChild が呼べて、読み込みは何も出さない
        $inner = New-SelfTestScript -Name 'exit5.ps1' -Text 'exit 5'
        $common = (Join-Path $PSScriptRoot 'perf-loop.common.ps1') -replace "'", "''"
        $innerQ = $inner -replace "'", "''"
        $outer = New-SelfTestScript -Name 'load-standalone.ps1' -Text (@(
            ". '$common'"
            '$pwsh = Get-PwshPath'
            '$a = Invoke-Child -Exe $pwsh -Arguments @(''-NoProfile'', ''-NonInteractive'', ''-Command'', ''exit 7'') -Quiet'
            "`$b = Invoke-PwshChild -ScriptPath '$innerQ' -Quiet"
            'if ($a.Code -eq 7 -and $b.Code -eq 5) { exit 0 }'
            'exit 9'
        ) -join "`n")
        $r = Invoke-PwshChild -ScriptPath $outer -Quiet
        Assert-SelfTest ($r.Code -eq 0) "child exit code $($r.Code) (want 0): $($r.LastLine)"
        Assert-SelfTest ($r.Lines.Count -eq 0) "loading common printed $($r.Lines.Count) lines: $($r.LastLine)"
    } }

    [pscustomobject]@{ Name = 'toml-reader'; Body = {
        $text = @(
            '# top comment'
            '[target]'
            "name = 'cargo'   # single quotes"
            'dq = "a # not a comment"'
            "sq = 'b # not a comment'"
            "arr = [`"x`", 'y,z']"
            'empty = []'
            'ratio = 1.5'
            'count = 1_000'
            '[ quiet ]'
            'neg = -3'
        ) -join "`r`n"
        $t = Read-TomlSections -Text $text
        Assert-SelfTest ((Get-TomlString $t 'target' 'name') -eq 'cargo') 'single-quoted string with comment'
        Assert-SelfTest ((Get-TomlString $t 'target' 'dq') -eq 'a # not a comment') '# inside double quotes'
        Assert-SelfTest ((Get-TomlString $t 'target' 'sq') -eq 'b # not a comment') '# inside single quotes'
        $arr = Get-TomlStringArray $t 'target' 'arr'
        Assert-SelfTest ($null -ne $arr -and $arr.Count -eq 2 -and $arr[0] -eq 'x' -and $arr[1] -eq 'y,z') 'string array'
        $empty = Get-TomlStringArray $t 'target' 'empty'
        Assert-SelfTest ($null -ne $empty -and $empty.Count -eq 0) "'[]' is an empty array, not missing"
        Assert-SelfTest ((Get-TomlDouble $t 'target' 'ratio') -eq 1.5) 'double'
        Assert-SelfTest ((Get-TomlInt $t 'target' 'count') -eq 1000) 'int with _'
        Assert-SelfTest ($null -eq (Get-TomlInt $t 'target' 'ratio')) 'int of a double must be null'
        Assert-SelfTest ((Get-TomlInt $t 'quiet' 'neg') -eq -3) 'heading with spaces / negative int'
        Assert-SelfTest ($null -eq (Get-TomlString $t 'target' 'missing')) 'missing key must be null'
    } }

    [pscustomobject]@{ Name = 'keyvalue-parse'; Body = {
        $m = ConvertFrom-KeyValueLines -Lines @('a=1', 'ghost=C:\x=y\z', 'noeq', '=nokey', '')
        Assert-SelfTest ($m['a'] -eq '1') 'a=1'
        Assert-SelfTest ($m['ghost'] -eq 'C:\x=y\z') "'=' inside the value"
        Assert-SelfTest ($m.Count -eq 2) "lines without a key must be skipped (got $($m.Count) keys)"
    } }

    [pscustomobject]@{ Name = 'child-args-and-encoding'; Body = {
        # 子は引数の数と中身を ASCII で、続けて U+00E9 の UTF-8 の 2 バイトを素のバイトで書く
        $child = New-SelfTestScript -Name 'echo-args.ps1' -Text (@(
            '$head = [System.Text.Encoding]::ASCII.GetBytes("n=$($args.Count);" + ($args -join ";") + ";")'
            '$bytes = [byte[]]($head + [byte[]](0xC3, 0xA9, 0x0A))'
            '$o = [Console]::OpenStandardOutput(); $o.Write($bytes, 0, $bytes.Length); $o.Flush()'
        ) -join "`n")
        $argv = @('-NoProfile', '-NonInteractive', '-File', $child, 'a b', 'c')
        $r = Invoke-Child -Exe (Get-PwshPath) -Arguments $argv -Quiet
        Assert-SelfTest ($r.Code -eq 0) "child exit code $($r.Code)"
        Assert-SelfTest ($r.LastLine -eq ('n=2;a b;c;' + [char]0xE9)) "default UTF-8 read / args: $(ConvertTo-SelfTestAscii $r.LastLine)"
        $r = Invoke-Child -Exe (Get-PwshPath) -Arguments $argv -Quiet -ReadEncoding ([System.Text.Encoding]::Latin1)
        Assert-SelfTest ($r.LastLine -eq ('n=2;a b;c;' + [char]0xC3 + [char]0xA9)) "-ReadEncoding Latin1 not applied: $(ConvertTo-SelfTestAscii $r.LastLine)"
    } }

    [pscustomobject]@{ Name = 'pwsh-child-args'; Body = {
        $argv = Get-PwshChildArguments -ScriptPath "C:\a b\it's.ps1" -Arguments @('-SelfTest', 'a b', "it's", '-1')
        Assert-SelfTest ($argv.Count -eq 4 -and $argv[0] -eq '-NoProfile' -and $argv[2] -eq '-Command') 'argv head'
        $want = "& 'C:\a b\it''s.ps1' -SelfTest 'a b' 'it''s' '-1'"
        Assert-SelfTest ($argv[3].Contains($want)) "command does not contain: $want"
        # 実際に回して、名前の形は引数名・ほかは値として 1 つずつ届く
        $child = New-SelfTestScript -Name 'bind.ps1' -Text (@(
            'param([switch]$Flag, [string]$Value, [string]$Other)'
            '[Console]::Out.WriteLine("flag=$Flag value=$Value other=$Other")'
        ) -join "`n")
        $r = Invoke-PwshChild -ScriptPath $child -Arguments @('-Flag', '-Value', "a b'c", '-Other', '-1') -Quiet
        Assert-SelfTest ($r.Code -eq 0) "child exit code $($r.Code): $($r.LastLine)"
        Assert-SelfTest ($r.LastLine -eq "flag=True value=a b'c other=-1") "binding: $(ConvertTo-SelfTestAscii $r.LastLine)"
    } }

    [pscustomobject]@{ Name = 'pwsh-child-error'; Body = {
        # わざと落ちる子（複数行の文）: 最後の 1 行が理由の文で、色の符号を含まない
        $child = New-SelfTestScript -Name 'throw.ps1' -Text 'throw "boom one`nboom two"'
        $r = Invoke-PwshChild -ScriptPath $child -Quiet
        Assert-SelfTest ($r.Code -eq 1) "child exit code $($r.Code) (want 1)"
        Assert-SelfTest ($r.LastLine.StartsWith('[pwsh-child] uncaught error: ') -and $r.LastLine.Contains('boom one') -and $r.LastLine.Contains('boom two')) "last line is not the reason: $(ConvertTo-SelfTestAscii $r.LastLine)"
        Assert-SelfTest (-not (@($r.Lines) -join "`n").Contains([string][char]27)) 'output contains ESC'
        # 色の符号つきで理由を書く子: Invoke-Child は符号を落としてから最後の 1 行を選ぶ
        $child = New-SelfTestScript -Name 'colored.ps1' -Text '[Console]::Error.WriteLine("$([char]27)[31mcolored reason$([char]27)[0m"); exit 1'
        $r = Invoke-PwshChild -ScriptPath $child -Quiet
        Assert-SelfTest ($r.Code -eq 1) "colored child exit code $($r.Code) (want 1)"
        Assert-SelfTest ($r.LastLine -eq 'colored reason') "colour codes not stripped from the last line: $(ConvertTo-SelfTestAscii $r.LastLine)"
    } }

    [pscustomobject]@{ Name = 'kill-wait'; Body = {
        # 子が起こした孫がファイルを握って眠る。Stop-ChildProcess で子孫まで止めた後、そのフォルダの名前を替えられる
        $held = Join-Path $script:SelfTestRoot 'held'
        $null = New-Item -ItemType Directory -Path $held
        $ready = Join-Path $script:SelfTestRoot 'held-ready.txt'
        $grandchild = New-SelfTestScript -Name 'hold.ps1' -Text (@(
            "`$f = [System.IO.File]::Open('$((Join-Path $held 'lock.txt') -replace "'", "''")', 'Create', 'ReadWrite', 'None')"
            "[System.IO.File]::WriteAllText('$($ready -replace "'", "''")', [string]`$PID)"
            'Start-Sleep -Seconds 60'
        ) -join "`n")
        $child = New-SelfTestScript -Name 'spawn-hold.ps1' -Text (@(
            '$psi = [System.Diagnostics.ProcessStartInfo]::new([Environment]::ProcessPath)'
            "foreach (`$a in @('-NoProfile', '-NonInteractive', '-File', '$($grandchild -replace "'", "''")')) { `$psi.ArgumentList.Add(`$a) }"
            '$psi.UseShellExecute = $false'
            '$null = [System.Diagnostics.Process]::Start($psi)'
            'Start-Sleep -Seconds 60'
        ) -join "`n")
        $psi = [System.Diagnostics.ProcessStartInfo]::new((Get-PwshPath))
        foreach ($a in @('-NoProfile', '-NonInteractive', '-File', $child)) { $psi.ArgumentList.Add($a) }
        $psi.UseShellExecute = $false
        $psi.CreateNoWindow = $true
        $process = [System.Diagnostics.Process]::Start($psi)
        try {
            $deadline = [DateTime]::UtcNow.AddSeconds(20)
            while (-not (Test-Path -LiteralPath $ready) -and [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
                Start-Sleep -Milliseconds 100
            }
            Assert-SelfTest (Test-Path -LiteralPath $ready) 'child did not start holding the file'
            $stopped = Stop-ChildProcess -Process $process -WaitMs 10000
            Assert-SelfTest $stopped 'Stop-ChildProcess returned false'
            Assert-SelfTest $process.HasExited 'Stop-ChildProcess returned before the child exited'
            try { Rename-Item -LiteralPath $held -NewName 'held-moved' -ErrorAction Stop }
            catch { throw "folder still held after Stop-ChildProcess: $($_.Exception.Message)" }
        } finally {
            if (-not $process.HasExited) { $process.Kill($true); $null = $process.WaitForExit(10000) }
            $process.Dispose()
            # 赤のときに孫が残ってフォルダを握り続けないよう、孫が自分で書いた PID で止める
            $gcPid = 0
            if ((Test-Path -LiteralPath $ready) -and [int]::TryParse((Get-Content -LiteralPath $ready -Raw), [ref]$gcPid)) {
                $gc = Get-Process -Id $gcPid -ErrorAction SilentlyContinue
                if ($gc) { $gc.Kill(); $null = $gc.WaitForExit(10000); $gc.Dispose() }
            }
        }
    } }

    [pscustomobject]@{ Name = 'utc-stamp'; Body = {
        $stamp = Get-UtcStamp
        Assert-SelfTest ($stamp -cmatch '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$') "shape: $(ConvertTo-SelfTestAscii $stamp)"
        $parsed = [DateTime]::ParseExact($stamp, 'yyyy-MM-ddTHH:mm:ss.fffZ', [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::AdjustToUniversal -bor [Globalization.DateTimeStyles]::AssumeUniversal)
        Assert-SelfTest ([Math]::Abs(([DateTime]::UtcNow - $parsed).TotalSeconds) -lt 60) "not UTC now: $stamp"
    } }

    # -------------------------------------------------------------------------
    # 終わり方の束（3・21）
    # -------------------------------------------------------------------------

    [pscustomobject]@{ Name = 'exit-trap'; Body = {
        # common と trap だけの子: 想定外の失敗は理由の 1 行と RESULT 行で 4。finally は throw でも本物の exit でも回る
        $common = (Join-Path $PSScriptRoot 'perf-loop.common.ps1') -replace "'", "''"
        $mark = Join-Path $script:SelfTestRoot 'finally.txt'
        $markQ = $mark -replace "'", "''"
        $child = New-SelfTestScript -Name 'exit-trap.ps1' -Text (@(
            '$script:CurrentSub = ''exit-trap''; $script:CurrentDir = ''-'''
            ". '$common'"
            'trap { Exit-OnUnexpectedError $_ }'
            '$ErrorActionPreference = ''Stop'''
            "if (`$args -contains 'exit') { try { Exit-WithResult -Code 2 } finally { [System.IO.File]::WriteAllText('$markQ', 'exit') } }"
            "try { throw 'boom' } finally { [System.IO.File]::WriteAllText('$markQ', 'throw') }"
        ) -join "`n")
        $r = Invoke-PwshChild -ScriptPath $child -Quiet
        Assert-SelfTest ($r.Code -eq 4) "unexpected failure: exit code $($r.Code) (want 4): $(ConvertTo-SelfTestAscii $r.LastLine)"
        Assert-SelfTest (($r.Out | Select-Object -Last 1) -eq 'PERF-LOOP RESULT exit-trap code=4 dir=-') "last stdout line: $(ConvertTo-SelfTestAscii (($r.Out | Select-Object -Last 1)))"
        Assert-SelfTest ([bool](@($r.Err) -match '^\[perf-loop\] FAILED: unexpected failure \(tool bug\): boom \(exit-trap\.ps1:\d+\)$')) "no reason line: $(ConvertTo-SelfTestAscii (@($r.Err) -join ' | '))"
        Assert-SelfTest ((Test-Path -LiteralPath $mark) -and (Get-Content -LiteralPath $mark -Raw) -eq 'throw') 'finally did not run on throw'
        Remove-Item -LiteralPath $mark
        $r = Invoke-PwshChild -ScriptPath $child -Arguments @('exit') -Quiet
        Assert-SelfTest ($r.Code -eq 2) "real exit: exit code $($r.Code) (want 2)"
        Assert-SelfTest ((Test-Path -LiteralPath $mark) -and (Get-Content -LiteralPath $mark -Raw) -eq 'exit') 'finally did not run on a real exit'

        # 共通部品を読み込む前の失敗（perf-loop.ps1 だけを置いたフォルダ）も RESULT 行を自分で出して 4
        $alone = Join-Path $script:SelfTestRoot 'alone'
        $null = New-Item -ItemType Directory -Path $alone
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'perf-loop.ps1') -Destination $alone
        $r = Invoke-PwshChild -ScriptPath (Join-Path $alone 'perf-loop.ps1') -Arguments @('rank') -Quiet
        Assert-SelfTest ($r.Code -eq 4 -and ($r.Out | Select-Object -Last 1) -eq 'PERF-LOOP RESULT - code=4 dir=-') "before common: code $($r.Code), last stdout line: $(ConvertTo-SelfTestAscii (($r.Out | Select-Object -Last 1)))"

        # 5 本の入口の写しに想定外の失敗を差し込む: 決まった終了コードと最後の行で終わる
        $cases = @(
            @{ Name = 'perf-loop.ps1'; Args = @('rank'); Code = 4; Last = 'PERF-LOOP RESULT rank code=4 dir=-' }
            @{ Name = 'invoke-perf-run.ps1'; Args = @('-SelfTest'); Code = 1; Last = '`[invoke-perf-run`] failed: unexpected failure: injected failure *' }
            @{ Name = 'invoke-cpu-sample.ps1'; Args = @('-Probe'); Code = 1; Last = 'INVOKE-CPU-SAMPLE RESULT probe code=1 *' }
            @{ Name = 'check-quiet.ps1'; Args = @('-SelfTest'); Code = 1; Last = 'CHECK-QUIET RESULT stage=selftest verdict=SELFTEST_NG code=1 file=-' }
            @{ Name = 'check-quiet.ps1'; Args = @('-Stage', 'after'); Code = 4; Last = 'CHECK-QUIET RESULT stage=after verdict=MEASURE_FAILED code=4 file=-' }
            @{ Name = 'invoke-followup-checks.ps1'; Args = @('-SelfTest'); Code = 1; Last = 'FOLLOWUP RESULT overall=INCONCLUSIVE * code=1 dir=-' }
        )
        foreach ($case in $cases) {
            $r = Invoke-PwshChild -ScriptPath (New-SelfTestBrokenCopy -Name $case.Name) -Arguments $case.Args -Quiet
            $label = "$($case.Name) $($case.Args -join ' ')"
            Assert-SelfTest ($r.Code -eq $case.Code) "${label}: exit code $($r.Code) (want $($case.Code)): $(ConvertTo-SelfTestAscii $r.LastLine)"
            # RESULT 行を持たない道具（ランナー）は理由の行が最後（標準エラー）
            $last = if ($case.Name -eq 'invoke-perf-run.ps1') { ($r.Err | Select-Object -Last 1) } else { ($r.Out | Select-Object -Last 1) }
            Assert-SelfTest ([bool]($last -like $case.Last)) "${label}: last line: $(ConvertTo-SelfTestAscii $last)"
            Assert-SelfTest ([bool]((@($r.Lines) -join "`n") -match 'unexpected failure( \(tool bug\))?: injected failure \(')) "${label}: no reason line"
        }
    } }

    [pscustomobject]@{ Name = 'trap-present'; Body = {
        # 5 本の入口とも、param の直後（ほかのどの文より前）に trap が 1 つだけ在る
        foreach ($name in $SelfTestEntryScripts) {
            $ast = [System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $name), [ref]$null, [ref]$null)
            $traps = @($ast.EndBlock.Traps | Where-Object { $_ })
            Assert-SelfTest ($traps.Count -eq 1) "${name}: $($traps.Count) top-level traps (want 1)"
            $first = @($ast.EndBlock.Statements)[0]
            Assert-SelfTest ($traps[0].Extent.StartOffset -lt $first.Extent.StartOffset) "${name}: the trap is not the first statement after param"
        }
    } }

    [pscustomobject]@{ Name = 'quiet-after-codes'; Body = {
        # 走行の後の静かさの確かめ: 3 は 3、4 は 4 でそのまま止まる（確かめ直さない）
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        function Start-Sleep { Add-SelfTestCall -Name 'Start-Sleep' }
        function Invoke-PwshChild {
            param([string]$ScriptPath, [string[]]$Arguments, [switch]$Quiet)
            Add-SelfTestCall -Name 'Invoke-PwshChild' -Arguments @($ScriptPath)
            return (New-ChildResult -Code $childCode -LastLine 'fake check-quiet')
        }
        $script:DryRunMode = $false
        $script:GoalConfig = $null
        $script:GoalFilePath = $null
        $script:GoalName = 'selftest'
        foreach ($childCode in @(3, 4)) {
            $script:SelfTestCalls.Clear()
            $code = Get-SelfTestExitCode { Invoke-MeasureQuietAfter -RunDir (Join-Path $script:SelfTestRoot 'run') }
            Assert-SelfTest ($code -eq $childCode) "check-quiet $childCode -> exit $code (want $childCode)"
            $calls = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'Invoke-PwshChild' }).Count
            Assert-SelfTest ($calls -eq 1) "check-quiet $childCode was called $calls times (want 1)"
        }
    } }

    [pscustomobject]@{ Name = 'child-exit-map'; Body = {
        # 子の道具の終了コードの写し方: 3 と 5 はそのまま、ほかは 4
        foreach ($pair in @(@(3, 3), @(5, 5), @(1, 4), @(2, 4), @(4, 4), @(-1, 4))) {
            $got = Convert-MeasureChildExit -Code $pair[0]
            Assert-SelfTest ($got -eq $pair[1]) "Convert-MeasureChildExit $($pair[0]) -> $got (want $($pair[1]))"
        }
        # ランナーの終了コード: 0 は続ける・2 と 3 はそのまま 1 回で止める・ほかは 1 度だけ採り直して 4
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        function Get-MeasureGhostRoot { 'G' }
        function Get-MeasureBalloonRoot { 'B' }
        function Sync-MeasureWorktreeBuild { param($Build, $BinDir) }
        function Clear-MeasureDryRunDir { param($Dir) }
        function Invoke-PwshChild {
            param([string]$ScriptPath, [string[]]$Arguments, [switch]$Quiet)
            Add-SelfTestCall -Name 'Invoke-PwshChild' -Arguments @($ScriptPath)
            return (New-ChildResult -Code $runnerCode -LastLine 'fake runner')
        }
        $script:DryRunMode = $true
        $outDir = Join-Path $script:SelfTestRoot 'no-such-run'
        foreach ($case in @(@{ Runner = 0; Exit = $null; Calls = 1 }, @{ Runner = 2; Exit = 2; Calls = 1 }, @{ Runner = 3; Exit = 3; Calls = 1 },
                            @{ Runner = 1; Exit = 4; Calls = 2 }, @{ Runner = 4; Exit = 4; Calls = 2 })) {
            $script:SelfTestCalls.Clear()
            $runnerCode = $case.Runner
            $code = Get-SelfTestExitCode { Invoke-MeasureRun -OutDir $outDir -RunProfile 'short' -Build 'release' }
            Assert-SelfTest ($code -eq $case.Exit) "runner $runnerCode -> exit $code (want $($case.Exit))"
            $calls = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'Invoke-PwshChild' }).Count
            Assert-SelfTest ($calls -eq $case.Calls) "runner $runnerCode was called $calls times (want $($case.Calls))"
        }
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
        function Write-MeasureDryRunMark { param($Dir, $Reason) }
        $script:BuildExplicit = $false
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
Write-Info "PERF-LOOP-SELFTEST RESULT ok=$okCount ng=$ngCount code=$code"
exit $code
