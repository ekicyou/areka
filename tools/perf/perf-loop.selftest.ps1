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

決まり（骨組みは perf-loop.selftest-base.ps1・2 本目と共に読む）:
  - perf-loop.common.ps1 と perf-loop.measure.ps1 を読み込み、その関数をそのまま試す。
  - 偽物は読み込んだ後で同じ名前の関数を定義し直して作り、呼ばれた順と引数を
    Add-SelfTestCall で $script:SelfTestCalls に残す（検査ごとに空にする）。
  - Exit-WithResult は定義し直してあり、本物の exit の代わりに終了コードを持った例外を投げる。
  - 作業のフォルダは <リポジトリ>\target\perf-loop-tmp\selftest-<GUID>\ の下だけ。終わったら消す。
  - 本物の cargo・python・xperf を呼ばない。子として起こすのは pwsh だけ。

検査を足すには: $SelfTestChecks に Name と Body（失敗なら throw する scriptblock）の 1 組を足す。
出力先と -Resume の束・入口の束は perf-loop.selftest2.ps1（1,000 行の番人のため束ごとに分けた）。
================================================================================
#>

. (Join-Path $PSScriptRoot 'perf-loop.selftest-base.ps1')

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
        # 見出しの後ろの注釈: 下の鍵はその節に入り、前の節へ紛れない
        $h = Read-TomlSections -Text "[stop]`nmax = 1`n[quiet] # note`nretry = 3`n"
        Assert-SelfTest ((Get-TomlInt $h 'quiet' 'retry') -eq 3 -and $null -eq (Get-TomlInt $h 'stop' 'retry')) 'heading with a trailing comment'
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
            # ready は書き終えてから名前を付ける（読み手が空のファイルを掴まない）
            "[System.IO.File]::WriteAllText('$($ready -replace "'", "''").tmp', [string]`$PID); [System.IO.File]::Move('$($ready -replace "'", "''").tmp', '$($ready -replace "'", "''")')"
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
        $gc = $null
        try {
            $deadline = [DateTime]::UtcNow.AddSeconds(20)
            while (-not (Test-Path -LiteralPath $ready) -and [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited) {
                Start-Sleep -Milliseconds 100
            }
            Assert-SelfTest (Test-Path -LiteralPath $ready) 'child did not start holding the file'
            # 待つ相手に孫が入ることを固定する（孫を待たない抜けは Kill が速いので実の走行では再現しにくい）
            $gcPidSeen = [int](Get-Content -LiteralPath $ready -Raw)
            # 後始末用に孫の Handle を開いておく（終わった後に PID が使い回されても別物を止めない）
            $gc = [System.Diagnostics.Process]::GetProcessById($gcPidSeen); $null = $gc.Handle
            Assert-SelfTest (@(Get-DescendantProcess -Process $process).Id -contains $gcPidSeen) 'Get-DescendantProcess did not find the grandchild'
            $stopped = Stop-ChildProcess -Process $process -WaitMs 10000
            Assert-SelfTest $stopped 'Stop-ChildProcess returned false'
            Assert-SelfTest $process.HasExited 'Stop-ChildProcess returned before the child exited'
            Assert-SelfTest $gc.HasExited 'Stop-ChildProcess returned before the grandchild exited'
            try { Rename-Item -LiteralPath $held -NewName 'held-moved' -ErrorAction Stop }
            catch { throw "folder still held after Stop-ChildProcess: $($_.Exception.Message)" }
        } finally {
            if (-not $process.HasExited) { $process.Kill($true); $null = $process.WaitForExit(10000) }
            $process.Dispose()
            # 赤のときに孫が残ってフォルダを握り続けないよう、開いておいた孫を止める
            if ($gc) {
                if (-not $gc.HasExited) { $gc.Kill(); $null = $gc.WaitForExit(10000) }
                $gc.Dispose()
            }
        }
    } }

    [pscustomobject]@{ Name = 'kill-after-parent-exit'; Body = {
        # 子が先に終わり、子が起こした孫がファイルを握って残る（areka が落ちて 32bit ヘルパが残る形）。それでも孫まで止めて待つ
        $held = Join-Path $script:SelfTestRoot 'held2'
        $null = New-Item -ItemType Directory -Path $held
        $ready = Join-Path $script:SelfTestRoot 'held2-ready.txt'
        $grandchild = New-SelfTestScript -Name 'hold2.ps1' -Text (@(
            "`$f = [System.IO.File]::Open('$((Join-Path $held 'lock.txt') -replace "'", "''")', 'Create', 'ReadWrite', 'None')"
            # ready は書き終えてから名前を付ける（読み手が空のファイルを掴まない）
            "[System.IO.File]::WriteAllText('$($ready -replace "'", "''").tmp', [string]`$PID); [System.IO.File]::Move('$($ready -replace "'", "''").tmp', '$($ready -replace "'", "''")')"
            'Start-Sleep -Seconds 60'
        ) -join "`n")
        $child = New-SelfTestScript -Name 'spawn-hold2.ps1' -Text (@(
            '$psi = [System.Diagnostics.ProcessStartInfo]::new([Environment]::ProcessPath)'
            "foreach (`$a in @('-NoProfile', '-NonInteractive', '-File', '$($grandchild -replace "'", "''")')) { `$psi.ArgumentList.Add(`$a) }"
            '$psi.UseShellExecute = $false'
            '$null = [System.Diagnostics.Process]::Start($psi)'
            "while (-not (Test-Path -LiteralPath '$($ready -replace "'", "''")')) { Start-Sleep -Milliseconds 100 }"
        ) -join "`n")
        $psi = [System.Diagnostics.ProcessStartInfo]::new((Get-PwshPath))
        foreach ($a in @('-NoProfile', '-NonInteractive', '-File', $child)) { $psi.ArgumentList.Add($a) }
        $psi.UseShellExecute = $false
        $psi.CreateNoWindow = $true
        $process = [System.Diagnostics.Process]::Start($psi)
        $gc = $null
        try {
            Assert-SelfTest ($process.WaitForExit(20000)) 'child did not exit after starting the grandchild'
            $gc = [System.Diagnostics.Process]::GetProcessById([int](Get-Content -LiteralPath $ready -Raw)); $null = $gc.Handle
            Assert-SelfTest (Stop-ChildProcess -Process $process -WaitMs 10000) 'Stop-ChildProcess returned false'
            Assert-SelfTest $gc.HasExited 'the grandchild of an exited child was not stopped'
            try { Rename-Item -LiteralPath $held -NewName 'held2-moved' -ErrorAction Stop }
            catch { throw "folder still held after Stop-ChildProcess: $($_.Exception.Message)" }
        } finally {
            if (-not $process.HasExited) { $process.Kill($true); $null = $process.WaitForExit(10000) }
            $process.Dispose()
            if ($gc) {
                if (-not $gc.HasExited) { $gc.Kill(); $null = $gc.WaitForExit(10000) }
                $gc.Dispose()
            }
        }
    } }

    [pscustomobject]@{ Name = 'kill-snapshot-fails'; Body = {
        # 子孫の一覧取りが投げても子は止め、そのことを 1 行で知らせる
        $script:SelfTestProblems = [System.Collections.Generic.List[string]]::new()
        function Write-Problem { param([string]$Message) $script:SelfTestProblems.Add($Message) }
        function Get-DescendantProcess { param($Process) throw 'snapshot boom' }
        $psi = [System.Diagnostics.ProcessStartInfo]::new((Get-PwshPath))
        foreach ($a in @('-NoProfile', '-NonInteractive', '-Command', 'Start-Sleep -Seconds 60')) { $psi.ArgumentList.Add($a) }
        $psi.UseShellExecute = $false
        $psi.CreateNoWindow = $true
        $process = [System.Diagnostics.Process]::Start($psi)
        try {
            $stopped = Stop-ChildProcess -Process $process -WaitMs 3000
            Assert-SelfTest $stopped 'Stop-ChildProcess returned false'
            Assert-SelfTest $process.HasExited 'child not killed when the snapshot threw'
            $msg = @($script:SelfTestProblems) -join ' | '
            Assert-SelfTest ($script:SelfTestProblems.Count -eq 1 -and $msg.Contains('descendant snapshot failed') -and $msg.Contains('snapshot boom')) "warning: $(ConvertTo-SelfTestAscii $msg)"
        } finally {
            if (-not $process.HasExited) { $process.Kill($true); $null = $process.WaitForExit(10000) }
            $process.Dispose()
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

    [pscustomobject]@{ Name = 'trap-before-base'; Body = {
        # 共通部品が読めない写しの部屋: 子の道具 4 本の trap は元の原因の 1 行と決まった終了コード・最後の行で終わる
        $dir = Join-Path $script:SelfTestRoot 'base-broken\tools\perf'
        $null = New-Item -ItemType Directory -Path $dir -Force
        Copy-Item -Path (Join-Path $PSScriptRoot '*.ps1') -Destination $dir
        [System.IO.File]::WriteAllText((Join-Path $dir 'perf-tool-base.ps1'), "throw 'base broken'`n")
        $cases = @(
            @{ Name = 'invoke-perf-run.ps1'; Args = @('-SelfTest'); Code = 1; Last = $null }
            @{ Name = 'invoke-cpu-sample.ps1'; Args = @('-Probe'); Code = 1; Last = 'INVOKE-CPU-SAMPLE RESULT probe code=1' }
            @{ Name = 'check-quiet.ps1'; Args = @('-SelfTest'); Code = 1; Last = 'CHECK-QUIET RESULT stage=selftest verdict=SELFTEST_NG code=1 file=-' }
            @{ Name = 'check-quiet.ps1'; Args = @('-Stage', 'after'); Code = 4; Last = 'CHECK-QUIET RESULT stage=after verdict=MEASURE_FAILED code=4 file=-' }
            @{ Name = 'invoke-followup-checks.ps1'; Args = @('-SelfTest'); Code = 1; Last = 'FOLLOWUP RESULT overall=INCONCLUSIVE clickthrough=- drag=- dpi=- balloon_follow=- code=1 dir=-' }
        )
        foreach ($case in $cases) {
            $r = Invoke-PwshChild -ScriptPath (Join-Path $dir $case.Name) -Arguments $case.Args -Quiet
            $label = "$($case.Name) $($case.Args -join ' ')"
            Assert-SelfTest ($r.Code -eq $case.Code) "${label}: exit code $($r.Code) (want $($case.Code)): $(ConvertTo-SelfTestAscii $r.LastLine)"
            if ($case.Last) { Assert-SelfTest (($r.Out | Select-Object -Last 1) -eq $case.Last) "${label}: last line: $(ConvertTo-SelfTestAscii (($r.Out | Select-Object -Last 1)))" }
            Assert-SelfTest ([bool]((@($r.Err) -join "`n") -match 'before the shared helpers loaded: base broken')) "${label}: the original cause is lost: $(ConvertTo-SelfTestAscii (@($r.Err) -join ' | '))"
        }
    } }

    [pscustomobject]@{ Name = 'quiet-after-codes'; Body = {
        # 走行の後の静かさの確かめ: 3 は 3、4 と回らなかった子（1・-1）は 4 でそのまま止まる（確かめ直さない）。2 は既定の回数だけ確かめて 2
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
        # 目標定義の負の回数・待ち秒数は確かめずに 3
        foreach ($bad in @('retry_max = -1', 'retry_wait_sec = -1')) {
            $script:SelfTestCalls.Clear()
            $script:GoalConfig = [pscustomobject]@{ Toml = (Read-TomlSections -Text "[quiet]`n$bad`n") }
            $code = Get-SelfTestExitCode { Invoke-MeasureQuietAfter -RunDir (Join-Path $script:SelfTestRoot 'run') }
            $calls = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'Invoke-PwshChild' }).Count
            Assert-SelfTest ($code -eq 3 -and $calls -eq 0) "${bad}: exit $code with $calls check-quiet calls (want 3 and none)"
        }
        $script:GoalConfig = $null
        foreach ($case in @(@(3, 3, 1), @(4, 4, 1), @(1, 4, 1), @(-1, 4, 1), @(2, 2, ($MEASURE_QUIET_RETRY_MAX_DEFAULT + 1)))) {
            $script:SelfTestCalls.Clear()
            $childCode = $case[0]
            $code = Get-SelfTestExitCode { Invoke-MeasureQuietAfter -RunDir (Join-Path $script:SelfTestRoot 'run') }
            Assert-SelfTest ($code -eq $case[1]) "check-quiet $childCode -> exit $code (want $($case[1]))"
            $calls = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'Invoke-PwshChild' }).Count
            Assert-SelfTest ($calls -eq $case[2]) "check-quiet $childCode was called $calls times (want $($case[2]))"
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

    [pscustomobject]@{ Name = 'not-quiet-evidence'; Body = {
        # ランナーの 2: 証拠の場所は moved to: の行から読んだ退け先。読めなければ <OutDir>-FAILED*
        $script:SelfTestProblems = [System.Collections.Generic.List[string]]::new()
        function Write-Problem { param([string]$Message) $script:SelfTestProblems.Add($Message) }
        function Write-Info { param([string]$Message) }
        function Get-MeasureGhostRoot { 'G' }
        function Get-MeasureBalloonRoot { 'B' }
        function Sync-MeasureWorktreeBuild { param($Build, $BinDir) }
        function Clear-MeasureDryRunDir { param($Dir) }
        function Invoke-PwshChild {
            param([string]$ScriptPath, [string[]]$Arguments, [switch]$Quiet)
            return (New-ChildResult -Code 2 -Out $runnerOut -Err @('[invoke-perf-run] failed: not quiet') -LastLine 'not quiet')
        }
        $script:DryRunMode = $true
        $outDir = Join-Path $script:SelfTestRoot 'nq-run'
        $moved = "$outDir-FAILED-123456789"
        foreach ($case in @(@{ Out = @('x', "[invoke-perf-run] moved to: $moved"); Want = "$moved\quiet-before.txt" },
                            @{ Out = @('x'); Want = "$outDir-FAILED*" })) {
            $script:SelfTestProblems.Clear()
            $runnerOut = $case.Out
            $code = Get-SelfTestExitCode { Invoke-MeasureRun -OutDir $outDir -RunProfile 'short' -Build 'release' }
            Assert-SelfTest ($code -eq 2) "runner 2 -> exit $code (want 2)"
            $msg = @($script:SelfTestProblems) -join ' | '
            Assert-SelfTest ($msg.Contains($case.Want)) "evidence '$($case.Want)' not in: $(ConvertTo-SelfTestAscii $msg)"
        }
    } }

    [pscustomobject]@{ Name = 'run-not-moved'; Body = {
        # ランナーの 1 で moved to: が無く出力先が残っている: 採り直さずに 4。退けが読めれば今のまま 1 度採り直す
        $script:SelfTestProblems = [System.Collections.Generic.List[string]]::new()
        function Write-Problem { param([string]$Message) $script:SelfTestProblems.Add($Message) }
        function Write-Info { param([string]$Message) }
        function Get-MeasureGhostRoot { 'G' }
        function Get-MeasureBalloonRoot { 'B' }
        function Sync-MeasureWorktreeBuild { param($Build, $BinDir) }
        function Clear-MeasureDryRunDir { param($Dir) }
        function Invoke-PwshChild {
            param([string]$ScriptPath, [string[]]$Arguments, [switch]$Quiet)
            Add-SelfTestCall -Name 'Invoke-PwshChild' -Arguments @($ScriptPath)
            if ($leaveDir) { $null = New-Item -ItemType Directory -Path $outDir -Force }
            return (New-ChildResult -Code 1 -Out $runnerOut -Err @('[invoke-perf-run] failed: killed') -LastLine 'killed')
        }
        $script:DryRunMode = $true
        $outDir = Join-Path $script:SelfTestRoot 'nm-run'
        foreach ($case in @(@{ Out = @('x'); Leave = $true; Calls = 1; Want = 'could not be moved aside' },
                            @{ Out = @("[invoke-perf-run] moved to: $outDir-FAILED"); Leave = $false; Calls = 2; Want = 'failed all 2 times' })) {
            $script:SelfTestCalls.Clear()
            $script:SelfTestProblems.Clear()
            $runnerOut = $case.Out
            $leaveDir = $case.Leave
            $code = Get-SelfTestExitCode { Invoke-MeasureRun -OutDir $outDir -RunProfile 'short' -Build 'release' }
            Remove-Item -LiteralPath $outDir -Recurse -Force -ErrorAction SilentlyContinue
            Assert-SelfTest ($code -eq 4) "leave=$($case.Leave): exit $code (want 4)"
            $calls = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'Invoke-PwshChild' }).Count
            Assert-SelfTest ($calls -eq $case.Calls) "leave=$($case.Leave): runner was called $calls times (want $($case.Calls))"
            $msg = @($script:SelfTestProblems) -join ' | '
            Assert-SelfTest ($msg.Contains($case.Want)) "leave=$($case.Leave): '$($case.Want)' not in: $(ConvertTo-SelfTestAscii $msg)"
            if ($case.Leave) { Assert-SelfTest ($msg.Contains($outDir)) "the out dir path not in: $(ConvertTo-SelfTestAscii $msg)" }
        }
    } }

    # -------------------------------------------------------------------------
    # 実行体の用意の束（10・11・12）
    # -------------------------------------------------------------------------

    [pscustomobject]@{ Name = 'exe-after-build'; Body = {
        # -BinDir の無い 3 つの走行は ビルド -> 確かめ の順。走行の本体はビルドを呼ばない
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        function Sync-MeasureWorktreeBuild { param($Build, $BinDir) Add-SelfTestCall -Name 'build' -Arguments @($BinDir) }
        function Test-MeasureExeReady { param($BinDir, $Build) Add-SelfTestCall -Name 'check' -Arguments @($BinDir); $false }
        function Write-MeasureDryRunMark { param($Dir, $Reason) }
        function Get-MeasureGhostRoot { 'G' }
        function Get-MeasureBalloonRoot { 'B' }
        function Clear-MeasureDryRunDir { param($Dir) }
        function Invoke-PwshChild { param([string]$ScriptPath, [string[]]$Arguments, [switch]$Quiet) New-ChildResult -Code 0 -LastLine 'ok' }
        $script:DryRunMode = $true
        $script:ResumeMode = $false
        $script:BuildKind = 'release'
        $script:BuildExplicit = $true
        $script:BinDirArg = ''
        $script:DateStamp = $null
        $script:GoalConfig = $null
        $script:GoalName = 'selftest'
        foreach ($sub in @('Invoke-SubMeasureBaseline', 'Invoke-SubRankRun', 'Invoke-SubFinal')) {
            $script:SelfTestCalls.Clear()
            $script:RunDirArg = Join-Path $script:SelfTestRoot "eab-$sub"
            $null = Get-SelfTestExitCode { & $sub }
            $order = @($script:SelfTestCalls | Where-Object { $_.Name -in 'build', 'check' } | ForEach-Object { $_.Name }) -join ','
            Assert-SelfTest ($order -ceq 'build,check') "${sub}: call order '$order' (want 'build,check')"
        }
        $script:SelfTestCalls.Clear()
        Invoke-MeasureRun -OutDir (Join-Path $script:SelfTestRoot 'eab-run') -RunProfile 'short' -Build 'release'
        $n = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'build' }).Count
        Assert-SelfTest ($n -eq 0) "Invoke-MeasureRun called the build $n times (want 0)"
        # 出力先が既に在れば、ビルドする前に 3（既に在る走行のためにビルドしない）
        foreach ($sub in @('Invoke-SubMeasureBaseline', 'Invoke-SubRankRun', 'Invoke-SubFinal')) {
            $script:SelfTestCalls.Clear()
            $script:RunDirArg = Join-Path $script:SelfTestRoot "eab2-$sub"
            $runDir = if ($sub -eq 'Invoke-SubFinal') { Join-Path $script:RunDirArg 'release' } else { $script:RunDirArg }
            $null = New-Item -ItemType Directory -Path $runDir -Force
            $code = Get-SelfTestExitCode { & $sub }
            $n = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'build' }).Count
            Assert-SelfTest ($code -eq 3 -and $n -eq 0) "${sub} with an existing out dir: exit $code with $n builds (want 3 and 0)"
        }
    } }

    [pscustomobject]@{ Name = 'final-bindir'; Body = {
        # final: -BinDir には -Build が要る（無ければ 3・何も呼ばない）。在れば確かめと走行へ渡す
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        function Sync-MeasureWorktreeBuild { param($Build, $BinDir) Add-SelfTestCall -Name 'build' -Arguments @($BinDir) }
        function Test-MeasureExeReady { param($BinDir, $Build) Add-SelfTestCall -Name 'check' -Arguments @($BinDir); $true }
        function Invoke-MeasureRun { param($OutDir, $RunProfile, $Build, $BinDir, $Label) Add-SelfTestCall -Name 'run' -Arguments @($BinDir) }
        function Invoke-MeasureQuietAfter { param($RunDir) }
        function Invoke-MeasureJudge { param($RunDir, $Build) 'PASS' }
        function Copy-MeasureVerdictToResults { param($RunDir, $Build) }
        $script:DryRunMode = $true
        $script:ResumeMode = $false
        $script:GoalConfig = $null
        $script:GoalName = 'selftest'
        $script:DateStamp = $null
        $bin = Join-Path $script:SelfTestRoot 'fb-bin'
        $script:BinDirArg = $bin
        $script:RunDirArg = Join-Path $script:SelfTestRoot 'fb-final'
        $script:BuildKind = 'release'
        $script:BuildExplicit = $false
        $code = Get-SelfTestExitCode { Invoke-SubFinal }
        Assert-SelfTest ($code -eq 3) "final -BinDir without -Build: exit $code (want 3)"
        $n = @($script:SelfTestCalls | Where-Object { $_.Name -in 'build', 'check', 'run' }).Count
        Assert-SelfTest ($n -eq 0) "final -BinDir without -Build made $n build/check/run calls (want 0)"
        $script:SelfTestCalls.Clear()
        $script:BuildExplicit = $true
        $code = Get-SelfTestExitCode { Invoke-SubFinal }
        Assert-SelfTest ($code -eq 0) "final -BinDir -Build: exit $code (want 0)"
        foreach ($name in @('check', 'run')) {
            $got = @($script:SelfTestCalls | Where-Object { $_.Name -eq $name } | ForEach-Object { $_.Arguments[0] })
            Assert-SelfTest ($got.Count -eq 1 -and $got[0] -eq $bin) "final $name got -BinDir '$(ConvertTo-SelfTestAscii ($got -join ','))' (want '$(ConvertTo-SelfTestAscii $bin)')"
        }
        # builds_final の語は 1 本目を測る前に全部確かめる（2 本目の打ち間違いで 1 本目を捨てない）
        $script:SelfTestCalls.Clear()
        $script:BinDirArg = ''
        $script:BuildExplicit = $false
        $script:RunDirArg = Join-Path $script:SelfTestRoot 'fb-final-typo'
        $script:GoalConfig = [pscustomobject]@{ Toml = (Read-TomlSections -Text "[target]`nbuilds_final = [`"release`", `"dvv`"]`n") }
        $code = Get-SelfTestExitCode { Invoke-SubFinal }
        $n = @($script:SelfTestCalls | Where-Object { $_.Name -in 'build', 'check', 'run' }).Count
        Assert-SelfTest ($code -eq 3 -and $n -eq 0) "builds_final with a bad word: exit $code with $n build/check/run calls (want 3 and 0)"
        $script:SelfTestCalls.Clear()
        $script:GoalConfig = [pscustomobject]@{ Toml = (Read-TomlSections -Text "[target]`nbuilds_final = [`"release`", `"Release`"]`n") }
        $code = Get-SelfTestExitCode { Invoke-SubFinal }
        $n = @($script:SelfTestCalls | Where-Object { $_.Name -in 'build', 'check', 'run' }).Count
        Assert-SelfTest ($code -eq 3 -and $n -eq 0) "builds_final naming release twice: exit $code with $n build/check/run calls (want 3 and 0)"
        $script:GoalConfig = $null
        # 2 本目（dev）の出力先が既に在れば、1 本目を測る前に 3
        $script:SelfTestCalls.Clear()
        $script:RunDirArg = Join-Path $script:SelfTestRoot 'fb-final-taken'
        $null = New-Item -ItemType Directory -Path (Join-Path $script:RunDirArg 'dev') -Force
        $code = Get-SelfTestExitCode { Invoke-SubFinal }
        $n = @($script:SelfTestCalls | Where-Object { $_.Name -in 'build', 'check', 'run' }).Count
        Assert-SelfTest ($code -eq 3 -and $n -eq 0) "final with dev already there: exit $code with $n build/check/run calls (want 3 and 0)"
    } }

    [pscustomobject]@{ Name = 'shiori-gate'; Body = {
        # 合否を出す走行は実 SHIORI が関門（偽・無いは 4）。相対比較の走行は警告して続ける。判定器の 0/1 は PASS/FAIL・ほかは 4
        $script:SelfTestProblems = [System.Collections.Generic.List[string]]::new()
        function Write-Problem { param([string]$Message) $script:SelfTestProblems.Add($Message) }
        function Write-Info { param([string]$Message) }
        $run = Join-Path $script:SelfTestRoot 'sg-run'
        $null = New-Item -ItemType Directory -Path $run -Force
        $meta = Join-Path $run 'run-meta.txt'
        foreach ($case in @(@{ Line = 'shiori_helper_present = True'; Abs = 'True'; Rel = $true }, @{ Line = 'shiori_helper_present = false'; Abs = 'exit4'; Rel = $false },
                            @{ Line = 'other = 1'; Abs = 'exit4'; Rel = $false })) {
            [System.IO.File]::WriteAllText($meta, "[run]`n$($case.Line)`n")
            $abs = try { "$(Assert-MeasureShioriHelper -RunDir $run -Absolute)" } catch { if ($_.Exception.Data['ExitCode'] -eq 4) { 'exit4' } else { throw } }
            Assert-SelfTest ($abs -eq $case.Abs) "'$($case.Line)' -Absolute: $abs (want $($case.Abs))"
            $script:SelfTestProblems.Clear()
            $rel = Assert-MeasureShioriHelper -RunDir $run
            Assert-SelfTest ($rel -eq $case.Rel -and ($script:SelfTestProblems.Count -gt 0) -ne $case.Rel) "'$($case.Line)' relative: $rel with $($script:SelfTestProblems.Count) warnings"
        }
        [System.IO.File]::WriteAllText($meta, "[run]`nshiori_helper_present = true`n")
        [System.IO.File]::WriteAllText((Join-Path $run 'run.log'), "x`n")
        $script:DryRunMode = $false
        $script:GoalConfig = $null
        function Get-MeasurePython { [pscustomobject]@{ Exe = 'python'; Prefix = @() } }
        function Invoke-PythonChild { param($Python, $Arguments, [switch]$Quiet) New-ChildResult -Code $script:FakeJudge -Out @('judge line') -LastLine 'judge line' }
        foreach ($pair in @(@(0, 'PASS'), @(1, 'FAIL'), @(2, 'exit4'), @(3, 'exit4'))) {
            $script:FakeJudge = $pair[0]
            $got = try { Invoke-MeasureJudge -RunDir $run -Build 'release' } catch { if ($_.Exception.Data['ExitCode'] -eq 4) { 'exit4' } else { throw } }
            Assert-SelfTest ($got -eq $pair[1]) "judge-perf.py $($pair[0]) -> $got (want $($pair[1]))"
        }
    } }

    [pscustomobject]@{ Name = 'helper-refresh'; Body = {
        # 32bit ヘルパ: 隣に無い・SHA-256 が違うなら写す。同じなら触らない。元が無ければ隣のものを残す。
        # -BinDir の走行（A/B の bin-A など）は隣に無いときだけ写し、SHA-256 が違っても書き換えない
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) Add-SelfTestCall -Name 'Write-Info' -Arguments @($Message) }
        $repoRoot = Join-Path $script:SelfTestRoot 'hr-repo'
        $srcDir = Join-Path $repoRoot (Join-Path 'target' (Join-Path $MEASURE_HELPER_TARGET 'release'))
        $bin = Join-Path $repoRoot (Join-Path 'target' 'release')
        $null = New-Item -ItemType Directory -Path $srcDir, $bin -Force
        $src = Join-Path $srcDir $MEASURE_HELPER_EXE
        $dest = Join-Path $bin $MEASURE_HELPER_EXE
        [System.IO.File]::WriteAllText($src, 'new')
        foreach ($case in @(@{ Name = 'missing'; Dest = $null; Copied = $true }, @{ Name = 'differs'; Dest = 'old'; Copied = $true },
                            @{ Name = 'same'; Dest = 'new'; Copied = $false })) {
            $script:SelfTestCalls.Clear()
            Remove-Item -LiteralPath $dest -Force -ErrorAction SilentlyContinue
            if ($null -ne $case.Dest) { [System.IO.File]::WriteAllText($dest, $case.Dest) }
            Sync-MeasureShioriHelper -BinDir '' -Build 'release'
            $text = if (Test-Path -LiteralPath $dest) { [System.IO.File]::ReadAllText($dest) } else { '<none>' }
            Assert-SelfTest ($text -ceq 'new') "$($case.Name): helper next to the exe is '$text' (want 'new')"
            $copied = [bool](@($script:SelfTestCalls | Where-Object { $_.Name -eq 'Write-Info' -and $_.Arguments[0] -like '*copied 32bit*' }).Count)
            Assert-SelfTest ($copied -eq $case.Copied) "$($case.Name): copied=$copied (want $($case.Copied))"
        }
        Remove-Item -LiteralPath $src -Force
        [System.IO.File]::WriteAllText($dest, 'old')
        Sync-MeasureShioriHelper -BinDir '' -Build 'release'
        Assert-SelfTest ([System.IO.File]::ReadAllText($dest) -ceq 'old') 'source missing: the helper next to the exe was changed'
        [System.IO.File]::WriteAllText($src, 'new')
        $side = Join-Path $script:SelfTestRoot 'hr-bin-A'
        $null = New-Item -ItemType Directory -Path $side -Force
        $sideDest = Join-Path $side $MEASURE_HELPER_EXE
        Sync-MeasureShioriHelper -BinDir $side -Build 'release'
        Assert-SelfTest ((Test-Path -LiteralPath $sideDest) -and [System.IO.File]::ReadAllText($sideDest) -ceq 'new') 'A/B side folder (-BinDir): a missing helper was not copied'
        [System.IO.File]::WriteAllText($sideDest, 'old')
        Sync-MeasureShioriHelper -BinDir $side -Build 'release'
        Assert-SelfTest ([System.IO.File]::ReadAllText($sideDest) -ceq 'old') 'A/B side folder (-BinDir): a helper whose SHA-256 differs was overwritten'
    } }

    [pscustomobject]@{ Name = 'ab-sequence'; Body = {
        # 34: ab_sequence が A と B ちょうど 2 回ずつでなければ 3。B のビルドもランナーも呼ばれない
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        function Test-MeasureBinDirComplete { param($Dir) $true }
        function Invoke-MeasureBuildSide { param($Side, $BinDir) Add-SelfTestCall -Name 'build'; throw 'stop-after-build' }
        function Invoke-MeasureRun { param($OutDir, $RunProfile, $Build, $BinDir, $Label) Add-SelfTestCall -Name 'run' }
        $script:DryRunMode = $false
        $script:ResumeMode = $false
        $script:GoalConfig = $null
        $script:GoalName = 'selftest'
        $script:OutRootPath = Join-Path $script:SelfTestRoot 'abs-out'
        $script:IterArg = 1
        foreach ($seq in @(@('A', 'B'), @('A', 'B', 'A', 'B', 'A'), @('A', 'A', 'A', 'B'), @('A', 'C', 'B', 'B'))) {
            $script:SelfTestCalls.Clear()
            $script:SelfTestSequence = $seq
            function Get-MeasureTomlArray { param($Section, $Key, $Default) $script:SelfTestSequence }
            $code = Get-SelfTestExitCode { Invoke-SubMeasureAb }
            Assert-SelfTest ($code -eq 3) "ab_sequence $($seq -join ','): exit $code (want 3)"
            $n = @($script:SelfTestCalls | Where-Object { $_.Name -in 'build', 'run' }).Count
            Assert-SelfTest ($n -eq 0) "ab_sequence $($seq -join ','): $n build/run calls (want 0)"
        }
        # 正しい並び（A B B A も）は B のビルドまで進む。iteration_build の語は大小を問わず release だけ
        $script:SelfTestSequence = @('a', 'B', 'B', 'A')
        $script:GoalConfig = [pscustomobject]@{ Toml = (Read-TomlSections -Text "[levels]`niteration_build = `"Release`"`n") }
        $msg = try { Invoke-SubMeasureAb; '<returned>' } catch { $_.Exception.Message }
        Assert-SelfTest ($msg -eq 'stop-after-build') "ab_sequence a,B,B,A / iteration_build Release did not reach the B build: $(ConvertTo-SelfTestAscii $msg)"
        Assert-SelfTest ((Get-MeasureAbBuild) -ceq 'release') 'iteration_build Release is not passed on as release'
        $script:GoalConfig = [pscustomobject]@{ Toml = (Read-TomlSections -Text "[levels]`niteration_build = `"dev`"`n") }
        $code = Get-SelfTestExitCode { Invoke-SubMeasureAb }
        Assert-SelfTest ($code -eq 3) "iteration_build dev: exit $code (want 3)"
        $script:GoalConfig = $null
        # 走る出力先が既に在れば、B を作る前に 3
        $script:SelfTestCalls.Clear()
        $null = New-Item -ItemType Directory -Path (Get-LoopDir -Kind 'iter' -Iter 1 -Leaf 'A1') -Force
        $code = Get-SelfTestExitCode { Invoke-SubMeasureAb }
        $n = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'build' }).Count
        Assert-SelfTest ($code -eq 3 -and $n -eq 0) "existing A1: exit $code with $n B builds (want 3 and 0)"
    } }

    [pscustomobject]@{ Name = 'ab-same-exe'; Body = {
        # 15: B を作った直後に bin-A と bin-B の exe_sha256 が同じなら走る前に 3。'-' のときと -DryRun では比べない
        function Write-Problem { param([string]$Message) }
        function Write-Info { param([string]$Message) }
        function Test-MeasureBinDirComplete { param($Dir) $true }
        function Invoke-MeasureBuildSide {
            param($Side, $BinDir)
            $null = New-Item -ItemType Directory -Path $BinDir -Force
            [System.IO.File]::WriteAllText((Join-Path $BinDir $MEASURE_FILE_BUILD), "side=B`nexe_sha256=$($script:SelfTestShaB)`nhelper=$($script:SelfTestHelperB)`n")
            return $BinDir
        }
        function Test-MeasureExeReady { param($BinDir, $Build) $true }
        function Invoke-MeasureRun { param($OutDir, $RunProfile, $Build, $BinDir, $Label) Add-SelfTestCall -Name 'run'; throw 'stop-at-run' }
        $script:ResumeMode = $false
        $script:GoalConfig = $null
        $script:GoalName = 'selftest'
        $script:OutRootPath = Join-Path $script:SelfTestRoot 'ase-out'
        $script:IterArg = 1
        $binA = Get-LoopDir -Kind 'iter' -Iter 1 -Leaf 'bin-A'
        $null = New-Item -ItemType Directory -Path $binA -Force
        $cases = @(
            @{ A = 'aa11'; B = 'aa11'; Dry = $false; Want = 'exit' }
            @{ A = 'aa11'; B = 'bb22'; Dry = $false; Want = 'run' }
            @{ A = '-'; B = '-'; Dry = $false; Want = 'run' }
            @{ A = 'aa11'; B = 'aa11'; Dry = $true; Want = 'run' }
            @{ A = $null; B = 'bb22'; Dry = $false; Want = 'run' }
            @{ A = 'aa11'; B = 'bb22'; Dry = $false; Want = 'exit4'; HelperB = 'missing' }
        )
        foreach ($case in $cases) {
            $script:SelfTestCalls.Clear()
            $script:DryRunMode = $case.Dry
            $script:SelfTestShaB = $case.B
            $script:SelfTestHelperB = if ($case.ContainsKey('HelperB')) { $case.HelperB } else { 'built' }
            # A が $null なら exe_sha256 の行そのものが無い BUILD.txt（比べずに進む）
            [System.IO.File]::WriteAllText((Join-Path $binA $MEASURE_FILE_BUILD), $(if ($null -eq $case.A) { "side=A`nhelper=built`n" } else { "side=A`nexe_sha256=$($case.A)`nhelper=built`n" }))
            $label = "A=$($case.A) B=$($case.B) dry=$($case.Dry)"
            $got = try { $c = Get-SelfTestExitCode { Invoke-SubMeasureAb }; "exit$c" } catch { $_.Exception.Message }
            $runs = @($script:SelfTestCalls | Where-Object { $_.Name -eq 'run' }).Count
            if ($case.Want -eq 'exit4') {
                # 片側だけヘルパが無ければ走る前に 4
                Assert-SelfTest ($got -eq 'exit4' -and $runs -eq 0) "${label} helper mismatch: got '$(ConvertTo-SelfTestAscii $got)' with $runs runs (want exit4 and 0 runs)"
            } elseif ($case.Want -eq 'exit') {
                Assert-SelfTest ($got -eq 'exit3' -and $runs -eq 0) "${label}: got '$(ConvertTo-SelfTestAscii $got)' with $runs runs (want exit3 and 0 runs)"
            } else {
                Assert-SelfTest ($got -eq 'stop-at-run') "${label}: got '$(ConvertTo-SelfTestAscii $got)' (want the first run)"
            }
        }
    } }
)

# =============================================================================
# 回す
# =============================================================================

Invoke-SelfTestChecks -Checks $SelfTestChecks -Label 'PERF-LOOP-SELFTEST'
