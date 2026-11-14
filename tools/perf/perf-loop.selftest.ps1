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
