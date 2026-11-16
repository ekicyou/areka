<#
.SYNOPSIS
  Self-test of the judging functions of tools/package.ps1. Exit code 0 = all passed, 1 = a failure.

.DESCRIPTION
  pwsh -NoProfile -File tools/package.ps1 -SelfTest

  Dot-sourced by package.ps1 -SelfTest after its functions are defined (never run on its own).
  No build, no cargo, no git, no launch check. The only child is package.ps1 itself with a bad
  -Arch value, which stops at the argument check. Writes only under target\package-selftest\<pid>\
  and removes it in finally. The two readme files are only read from vendors/sample_ghost/emo2.nar.
  Prints one line per check, ASCII only:
    ok selftest <name>  /  FAIL selftest <name>: <reason>
#>

$top = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/package-selftest'
$base = Join-Path $top "$PID"

function Assert([bool]$Cond, [string]$Why) { if (-not $Cond) { throw $Why } }

# Smallest PE that Read-PeInfo reads: header, 1 section at RVA 0x1000 (file offset 0x200),
# import table at its start naming $Dlls. $Plus = PE32+ (64-bit optional header).
function New-Pe([int]$Machine, [bool]$Plus, [string[]]$Dlls) {
    $b = [byte[]]::new(0x400)
    $put16 = { param($o, $v) [BitConverter]::GetBytes([uint16]$v).CopyTo($b, $o) }
    $put32 = { param($o, $v) [BitConverter]::GetBytes([uint32]$v).CopyTo($b, $o) }
    $pe = 0x80; $opt = $pe + 24; $optSize = $Plus ? 240 : 224
    & $put32 0x3c $pe
    & $put32 $pe 0x4550
    & $put16 ($pe + 4) $Machine
    & $put16 ($pe + 6) 1
    & $put16 ($pe + 20) $optSize
    & $put16 $opt ($Plus ? 0x20b : 0x10b)
    & $put16 ($opt + 68) 2
    & $put32 ($opt + ($Plus ? 112 : 96) + 8) 0x1000
    $s = $opt + $optSize
    & $put32 ($s + 8) 0x200; & $put32 ($s + 12) 0x1000; & $put32 ($s + 16) 0x200; & $put32 ($s + 20) 0x200
    for ($i = 0; $i -lt $Dlls.Count; $i++) {
        & $put32 (0x200 + 20 * $i + 12) (0x1100 + 0x40 * $i)
        [Text.Encoding]::ASCII.GetBytes($Dlls[$i]).CopyTo($b, 0x300 + 0x40 * $i)
    }
    , $b
}

function New-Zip([string]$Path, $Entries) {
    $z = [IO.Compression.ZipFile]::Open($Path, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($k in $Entries.Keys) {
            $s = $z.CreateEntry($k).Open()
            try { $s.Write($Entries[$k], 0, $Entries[$k].Length) } finally { $s.Dispose() }
        }
    } finally { $z.Dispose() }
}

function Read-NarEntry([string]$Name) {
    $m = Get-ZipEntryMap (Join-Path (Split-Path -Parent $PSScriptRoot) 'vendors/sample_ghost/emo2.nar')
    try { Read-ZipEntry $m.Map[$Name] } finally { $m.Zip.Dispose() }
}

# Names of the rows that Test-RunLog judged not Ok, joined with ','
function Get-RunLogFails([string[]]$Lines, [int]$Code = 0, [bool]$Killed = $false) {
    $rows = @(Test-RunLog ($Lines -join "`n") $Code $Killed)
    Assert ($rows.Count -eq 7) "rows: $($rows.Count)"
    (@($rows | Where-Object { -not $_.Ok } | ForEach-Object Name)) -join ','
}

function Get-ArgsError([hashtable]$P) {
    $d = @{ Arch = 'x64'; Check = $false; CheckDir = ''; KeepExpanded = $false; SmokeExitMsGiven = $false; SmokeExitMs = ''; RepoRoot = 'C:\repo\' }
    foreach ($k in $P.Keys) { $d[$k] = $P[$k] }
    Test-PackageArgs @d
}

$checks = [ordered]@{
    'run log'                  = {
        $balloon = $LOG_MARKER_BALLOON, $LOG_MARKER_BALLOON_ROUTE, ('dir=C:\x' + $LOG_MARKER_BALLOON_DIR) -join ' '
        $good = @($LOG_MARKER_SMOKE_GATE, $LOG_MARKER_WINDOWS, $balloon, $LOG_MARKER_GREETING, $LOG_MARKER_SMOKE_EXIT)
        Assert ((Get-RunLogFails $good) -eq '') "good: $(Get-RunLogFails $good)"
        $late = @($LOG_MARKER_SMOKE_GATE, $LOG_MARKER_WINDOWS, $balloon, $LOG_MARKER_SMOKE_EXIT, $LOG_MARKER_GREETING)
        Assert ((Get-RunLogFails $late) -eq 'talk started') "greeting after exit: $(Get-RunLogFails $late)"
        Assert ((Get-RunLogFails $good 1 $true) -eq 'not killed by watchdog,exit code 0') "killed: $(Get-RunLogFails $good 1 $true)"
        foreach ($f in $LOG_MARKER_FAULTS) {
            Assert ((Get-RunLogFails ($good + "x $f")) -eq 'no SHIORI connection failure') 'fault marker'
        }
        $other = @($LOG_MARKER_SMOKE_GATE, $LOG_MARKER_WINDOWS, ($LOG_MARKER_BALLOON, $LOG_MARKER_BALLOON_ROUTE, 'dir=C:\x\balloon\other' -join ' '),
            $balloon, $LOG_MARKER_GREETING, $LOG_MARKER_SMOKE_EXIT)
        Assert ((Get-RunLogFails $other) -eq 'first balloon is bundled') "first balloon: $(Get-RunLogFails $other)"
    }
    'pe read'                  = {
        foreach ($c in @(@(0x014c, $false), @(0x8664, $true))) {
            $i = Read-PeInfo (New-Pe $c[0] $c[1] @('KERNEL32.dll', 'USER32.dll'))
            Assert ($i.Machine -eq $c[0]) ('machine 0x{0:x}' -f $i.Machine)
            Assert ($i.Subsystem -eq 2) "subsystem $($i.Subsystem)"
            Assert (($i.Imports -join ',') -ceq 'KERNEL32.dll,USER32.dll') "imports $($i.Imports -join ',')"
        }
        $threw = $false
        try { $null = Read-PeInfo ([byte[]]::new(0x100)) } catch { $threw = "$_" -eq 'no PE signature' }
        Assert $threw 'no PE signature did not throw'
    }
    'deny list'                = {
        $d = @(Get-DeniedImports @('KERNEL32.dll', 'VCRUNTIME140.dll', 'api-ms-win-crt-runtime-l1-1-0.dll'))
        Assert (($d -join ',') -ceq 'VCRUNTIME140.dll,api-ms-win-crt-runtime-l1-1-0.dll') "denied: $($d -join ',')"
    }
    'zip content'              = {
        $root = Split-Path -Parent $PSScriptRoot
        $notices = Join-Path $base 'notices.md'
        [IO.File]::WriteAllText($notices, 'fake notices')
        $script:Version = '9.9.9'; $script:Commit = 'abc1234'; $script:Dirty = 0; $script:Notices = $notices
        $gui = New-Pe 0x8664 $true @('KERNEL32.dll')
        $x86 = New-Pe 0x014c $false @('KERNEL32.dll')
        $txt = [Text.Encoding]::ASCII.GetBytes('x')
        $e = [ordered]@{
            'areka.exe' = $gui; 'areka-mcp-bridge.exe' = $gui; 'shiori-host32-helper.exe' = $x86
            'ghost/emo2/ghost/master/pasta.dll' = $x86; 'ghost/emo2/ghost/master/descript.txt' = $txt; 'ghost/emo2/install.txt' = $txt
            'ghost/emo2/readme.txt' = (Read-NarEntry 'readme.txt'); 'ghost/emo2/shell/master/readme.txt' = (Read-NarEntry 'shell/master/readme.txt')
            'balloon/emo2-kakukaku/descript.txt' = $txt; 'balloon/StayseeBalloon/descript.txt' = $txt; 'balloon/areka-balloon/descript.txt' = $txt
            'README.txt' = [IO.File]::ReadAllBytes((Join-Path $root 'dist/README.txt'))
            'LICENSE-MIT' = [IO.File]::ReadAllBytes((Join-Path $root 'LICENSE-MIT'))
            'THIRD-PARTY-NOTICES.md' = [IO.File]::ReadAllBytes($notices)
            'BUILD-INFO.txt' = [Text.Encoding]::UTF8.GetBytes("version=9.9.9`narch=x64`ncommit=abc1234`ndirty=0`n")
        }
        $judge = {
            param($Name, $Entries)
            $p = Join-Path $base "$Name.zip"
            New-Zip $p $Entries
            $bad = Test-ZipContent $p 'x64' 6>$null   # returns one string[]
            $bad -join ' | '
        }
        $r = & $judge 'good' $e
        Assert ($r -eq '') "good: $r"
        $m = [ordered]@{} + $e; $m.Remove('README.txt')
        $r = & $judge 'missing' $m
        Assert ($r -ceq '1 required entry missing: README.txt') "missing: $r"
        $m = [ordered]@{} + $e; $m['ghost/emo2/ghost/master/profile/x.dat'] = $txt
        $r = & $judge 'profile' $m
        Assert ($r -clike '3 1 entries contain profile/ *') "profile: $r"
        $m = [ordered]@{} + $e; $m['areka.exe'] = New-Pe 0x014c $true @('KERNEL32.dll')
        $r = & $judge 'machine' $m
        Assert ($r -clike '5 wrong machine: areka.exe is 0x014c *') "machine: $r"
    }
    'args'                     = {
        $bad = [ordered]@{
            "-SmokeExitMs must be a positive integer (got: 'x')"                                     = @{ Check = $true; SmokeExitMsGiven = $true; SmokeExitMs = 'x' }
            "-SmokeExitMs must be a positive integer (got: '0')"                                     = @{ Check = $true; SmokeExitMsGiven = $true; SmokeExitMs = '0' }
            "-Arch must be one of x64, arm64, all (got: 'x86')"                                       = @{ Arch = 'x86' }
            "-Arch must be one of x64, arm64, all (got: 'X64')"                                       = @{ Arch = 'X64' }
            "-Check needs the x64 zip (combine with -Arch x64 or all; got: 'arm64')"                  = @{ Arch = 'arm64'; Check = $true }
            "-CheckDir must be an absolute path (got: 'rel\x')"                                       = @{ Check = $true; CheckDir = 'rel\x' }
            "-CheckDir inside the repository must be under C:\repo\target\ (got: 'C:\repo\src')"      = @{ Check = $true; CheckDir = 'C:\repo\src' }
            '-KeepExpanded, -SmokeExitMs and -CheckDir need -Check (got: -KeepExpanded)'              = @{ KeepExpanded = $true }
            '-KeepExpanded, -SmokeExitMs and -CheckDir need -Check (got: -SmokeExitMs)'               = @{ SmokeExitMsGiven = $true; SmokeExitMs = '500' }
            '-KeepExpanded, -SmokeExitMs and -CheckDir need -Check (got: -CheckDir)'                  = @{ CheckDir = 'C:\out' }
        }
        foreach ($want in $bad.Keys) {
            $got = (Get-ArgsError $bad[$want]).Error
            Assert ($got -ceq $want) "want '$want' got '$got'"
        }
        $r = Get-ArgsError @{ Arch = 'all' }
        Assert ($null -eq $r.Error -and ($r.BuildArchs -join ',') -eq 'x64,arm64' -and $r.SmokeMs -eq $SMOKE_EXIT_MS) "all: $($r.Error)"
        foreach ($ok in @(@{ Check = $true; KeepExpanded = $true }, @{ Check = $true; CheckDir = 'C:\repo\target\x' },
                @{ Check = $true; CheckDir = 'D:\out' })) {
            $r = Get-ArgsError $ok
            Assert ($null -eq $r.Error) "refused: $($r.Error)"
        }
        $r = Get-ArgsError @{ Check = $true; SmokeExitMsGiven = $true; SmokeExitMs = '500' }
        Assert ($null -eq $r.Error -and $r.SmokeMs -eq 500) "smoke ms: $($r.Error) $($r.SmokeMs)"
    }
    'ascii exit message'       = {
        $r = Invoke-Utf8Child pwsh @('-NoProfile', '-File', (Join-Path $PSScriptRoot 'package.ps1'), '-Arch', [string][char]0x65E5)
        $lines = @($r.Out -split '\r?\n' | Where-Object { $_ -ne '' }) + $r.ErrLines
        $all = Show-Ascii ($lines -join ' | ') 0
        Assert ($r.Code -eq 3) "code $($r.Code): $all"
        foreach ($l in $lines) { Assert ($l -cmatch '^[\x20-\x7E]*$') "not ASCII: $(Show-Ascii $l 0)" }
        Assert (@($lines | Where-Object { $_.Contains('\u65E5') }).Count -eq 1) "no \u65E5: $all"
        Assert (@($lines | Where-Object { $_ -clike '==> *' -and $_ -cne '==> preflight' }).Count -eq 0) "went past preflight: $all"
    }
    'cleanup idempotent'       = {
        $v = 'AREKA_PACKAGE_SELFTEST_VAR'
        $path = $env:PATH
        $f = Join-Path $base 'x.zip.tmp'
        [IO.File]::WriteAllText($f, 'x')
        $script:TmpFiles.Add($f)
        Set-EnvTemp $v 'changed'
        $env:PATH = "$path;C:\package-selftest"
        Invoke-Cleanup 1
        Invoke-Cleanup 1
        Assert ($null -eq [Environment]::GetEnvironmentVariable($v)) "$v left"
        Assert ($env:PATH -ceq $path) 'PATH not restored'
        Assert (-not (Test-Path -LiteralPath $f)) 'tmp file left'
    }
    'steps inside try finally' = {
        $ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'package.ps1'), [ref]$null, [ref]$null)
        $isCall = { param($n, $name) $n -is [Management.Automation.Language.CommandAst] -and $n.GetCommandName() -ceq $name }
        $inFunction = {
            param($n)
            for ($p = $n.Parent; $p; $p = $p.Parent) { if ($p -is [Management.Automation.Language.FunctionDefinitionAst]) { return $true } }
            $false
        }
        $steps = @($ast.FindAll({ param($n) (& $isCall $n 'Step') -and -not (& $inFunction $n) }, $true))
        Assert ($steps.Count -ge 10) "Step calls: $($steps.Count)"
        $tries = @($ast.FindAll({
                    param($n) $n -is [Management.Automation.Language.TryStatementAst] -and $n.Finally -and
                    $n.Finally.Find({ param($c) & $isCall $c 'Invoke-Cleanup' }, $true)
                }, $true))
        Assert ($tries.Count -eq 1) "try with Invoke-Cleanup in finally: $($tries.Count)"
        $body = $tries[0].Body.Extent
        foreach ($s in $steps) {
            Assert ($s.Extent.StartOffset -ge $body.StartOffset -and $s.Extent.EndOffset -le $body.EndOffset) "Step outside the try at line $($s.Extent.StartLineNumber)"
        }
        $exit = $ast.Find({ param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -ceq 'Exit-Script' }, $true)
        Assert ($null -ne $exit) 'no Exit-Script'
        Assert ($null -eq $exit.Find({ param($c) & $isCall $c 'Invoke-Cleanup' }, $true)) 'Exit-Script calls Invoke-Cleanup'
    }
}

$failed = 0
try {
    [void][IO.Directory]::CreateDirectory($base)
    foreach ($name in $checks.Keys) {
        try { $null = & $checks[$name]; "ok selftest $name" }
        catch { $failed++; "FAIL selftest ${name}: $(Show-Ascii $_.Exception.Message 0)" }
    }
}
finally {
    try {
        if ([IO.Directory]::Exists($base)) { [IO.Directory]::Delete($base, $true) }
        if ([IO.Directory]::Exists($top) -and -not [IO.Directory]::EnumerateFileSystemEntries($top).GetEnumerator().MoveNext()) {
            [IO.Directory]::Delete($top)
        }
    }
    catch { $failed++; "FAIL selftest cleanup: $(Show-Ascii $_.Exception.Message 0)" }
}
exit ([int]($failed -gt 0))
