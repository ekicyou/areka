<#
.SYNOPSIS
  Self-test of the shared install steps (tools/exe-install.ps1). Exit code 0 = all passed, 1 = a failure.

.DESCRIPTION
  pwsh -NoProfile -File tools/exe-install.selftest.ps1

  Runs every branch of Test-InstallHome, Install-ToolExe and Get-StateStatusAction in fake install
  folders under target\exe-install-selftest\<pid>\. The fake built exe is a small text file
  ('good' / 'bad'); the fake check reads that text to decide. No real exe, no cargo, never reads or
  sets AREKA_IMPL_WATCH_HOME / AREKA_MCP_HOME. Held file handles and the folder are released and
  removed in finally. Prints one line per check, ASCII only:
    ok selftest <name>  /  FAIL selftest <name>: <reason>
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'exe-install.ps1')

$top = Join-Path (Split-Path -Parent $PSScriptRoot) 'target/exe-install-selftest'
$base = Join-Path $top "$PID"
$held = [System.Collections.Generic.List[System.IDisposable]]::new()
$n = 't'

function Assert([bool]$Cond, [string]$Why) { if (-not $Cond) { throw $Why } }

function New-Home([string]$Case, [string]$Current) {
    $h = Join-Path $base $Case
    [void][System.IO.Directory]::CreateDirectory($h)
    if ($Current) { [System.IO.File]::WriteAllText((Join-Path $h "$n.exe"), $Current) }
    $h
}

function Get-Built([string]$Text) {
    $p = Join-Path $base "built-$Text.exe"
    [System.IO.File]::WriteAllText($p, $Text)
    $p
}

# Opens without FileShare.Delete, so rename and delete of that file fail until finally.
function Lock-File([string]$Path) {
    $held.Add([System.IO.File]::Open($Path, 'Open', 'Read', 'Read'))
}

function Read-Exe([string]$HomeDir) { [System.IO.File]::ReadAllText((Join-Path $HomeDir "$n.exe")) }

function Get-Olds([string]$HomeDir) {
    (@([System.IO.Directory]::GetFiles($HomeDir, "$n.old*.exe") | ForEach-Object { [System.IO.Path]::GetFileName($_) } | Sort-Object)) -join ','
}

function Assert-Lines($R, [int]$Code, [string[]]$Like) {
    $l = @($R.Lines)
    $all = $l -join ' | '
    Assert ($R.Code -eq $Code) "code $($R.Code): $all"
    Assert ($l.Count -eq $Like.Count) "lines: $all"
    for ($i = 0; $i -lt $Like.Count; $i++) { Assert ($l[$i] -like $Like[$i]) "line ${i}: $all" }
}

$check = {
    param([string]$ExePath)
    $t = [System.IO.File]::ReadAllText($ExePath)
    if ($t -eq 'good') { @{ Ok = $true; Line = 'OK check: good' } } else { @{ Ok = $false; Line = "FAIL check: content $t" } }
}

$checks = [ordered]@{
    'home checks'                     = {
        $v = 'AREKA_X_HOME'
        foreach ($x in @('', '   ')) { Assert ((Test-InstallHome $v $x) -eq "FAIL home: $v is not set") "blank '$x'" }
        foreach ($x in @('rel\x', '/c/x', 'C:foo')) {
            $r = Test-InstallHome $v $x
            Assert ($r -eq "FAIL home: $v is not an absolute path") "relative '$x': $r"
            Assert (-not $r.Contains($x)) "value printed: $r"
        }
        Assert ($null -eq (Test-InstallHome $v 'C:\x')) 'absolute path refused'
    }
    'first place'                     = {
        $h = New-Home 'first'
        $r = Install-ToolExe -HomeDir $h -Name $n -Built (Get-Built 'good') -Check $check
        Assert-Lines $r 0 @('OK check: good', "OK placed $n.exe")
        Assert ((Read-Exe $h) -eq 'good') 'placed exe content'
        Assert (-not (Test-Path -LiteralPath (Join-Path $h "$n.new.exe"))) '.new.exe left'
    }
    'good check places and sweeps'    = {
        $h = New-Home 'good' 'current'
        foreach ($o in 1, 2) { [System.IO.File]::WriteAllText((Join-Path $h "$n.old-$o.exe"), 'old') }
        $r = Install-ToolExe -HomeDir $h -Name $n -Built (Get-Built 'good') -Check $check
        Assert-Lines $r 0 @('OK check: good', "OK renamed previous to $n.old-*Z.exe", "OK placed $n.exe",
            "OK removed $n.old*.exe", "OK removed $n.old*.exe", "OK removed $n.old*.exe")
        $renamed = $r.Lines[1].Substring('OK renamed previous to '.Length)
        $removed = @($r.Lines[3..5] | ForEach-Object { $_.Substring('OK removed '.Length) } | Sort-Object)
        Assert (($removed -join ',') -eq ((@("$n.old-1.exe", "$n.old-2.exe", $renamed) | Sort-Object) -join ',')) "removed: $($removed -join ',')"
        Assert ((Read-Exe $h) -eq 'good') 'placed exe content'
        Assert ((Get-Olds $h) -eq '') "old copies: $(Get-Olds $h)"
    }
    'bad check keeps the current exe' = {
        $h = New-Home 'bad' 'current'
        foreach ($o in 1, 2) { [System.IO.File]::WriteAllText((Join-Path $h "$n.old-$o.exe"), 'old') }
        $r = Install-ToolExe -HomeDir $h -Name $n -Built (Get-Built 'bad') -Check $check
        Assert-Lines $r 1 @('FAIL check: content bad')
        Assert ((Read-Exe $h) -eq 'current') 'current exe changed'
        Assert (-not (Test-Path -LiteralPath (Join-Path $h "$n.new.exe"))) '.new.exe left'
        Assert ((Get-Olds $h) -eq "$n.old-1.exe,$n.old-2.exe") "old copies: $(Get-Olds $h)"
    }
    'check throws'                    = {
        $h = New-Home 'throws' 'current'
        # The held .new.exe also fails the cleanup; neither reason may print the install folder.
        $r = Install-ToolExe -HomeDir $h -Name $n -Built (Get-Built 'good') -Check {
            param([string]$ExePath)
            Lock-File $ExePath
            throw "boom $([char]0x7248) $ExePath"
        }
        Assert-Lines $r 1 @('FAIL check: cannot start: boom \u7248 *', "FAIL cleanup: cannot remove $n.new.exe: *")
        foreach ($l in $r.Lines) {
            Assert ($l -cmatch '^[\x20-\x7E]*$') "not ASCII: $l"
            Assert ($l.IndexOf($h, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) "install folder printed: $l"
        }
        Assert ((Read-Exe $h) -eq 'current') 'current exe changed'
    }
    'rename fails and restores'       = {
        $h = New-Home 'rename' 'current'
        $r = Install-ToolExe -HomeDir $h -Name $n -Built (Get-Built 'good') -Check {
            param([string]$ExePath)
            Lock-File $ExePath
            @{ Ok = $true; Line = 'OK check: held' }
        }
        Assert-Lines $r 1 @('OK check: held', 'FAIL place: rename')
        Assert ((Read-Exe $h) -eq 'current') 'current exe not restored'
        Assert ((Get-Olds $h) -eq '') "old copies: $(Get-Olds $h)"
    }
    'keep in use'                     = {
        $h = New-Home 'keep' 'current'
        foreach ($o in 1, 2) { [System.IO.File]::WriteAllText((Join-Path $h "$n.old-$o.exe"), 'old') }
        Lock-File (Join-Path $h "$n.old-1.exe")
        $r = Install-ToolExe -HomeDir $h -Name $n -Built (Get-Built 'good') -Check $check
        Assert-Lines $r 0 @('OK check: good', "OK renamed previous to $n.old-*Z.exe", "OK placed $n.exe", '*', '*', '*')
        $sweep = @($r.Lines[3..5])
        Assert (@($sweep -ceq "KEEP $n.old-1.exe (in use; run again later)").Count -eq 1) "sweep: $($sweep -join ' | ')"
        Assert (@($sweep -clike "OK removed $n.old*.exe").Count -eq 2) "sweep: $($sweep -join ' | ')"
        Assert ((Get-Olds $h) -eq "$n.old-1.exe") "old copies: $(Get-Olds $h)"
    }
    'copy fails'                      = {
        $h = New-Home 'copy' 'current'
        [void][System.IO.Directory]::CreateDirectory((Join-Path $h "$n.new.exe"))
        $r = Install-ToolExe -HomeDir $h -Name $n -Built (Get-Built 'good') -Check $check
        Assert-Lines $r 1 @('FAIL place: copy')
        Assert ((Read-Exe $h) -eq 'current') 'current exe changed'
    }
    'status action'                   = {
        Assert ((Get-StateStatusAction 0 @('anything')) -eq 'none') '0'
        Assert ((Get-StateStatusAction 1 @('state file version mismatch: 2 != 3')) -eq 'clear') 'mismatch'
        Assert ((Get-StateStatusAction 1 @('state.json is broken')) -eq 'fail') 'other'
        Assert ((Get-StateStatusAction 1 @()) -eq 'fail') 'empty'
    }
}

$failed = 0
try {
    [void][System.IO.Directory]::CreateDirectory($base)
    foreach ($name in $checks.Keys) {
        try { $null = & $checks[$name]; "ok selftest $name" }
        catch { $failed++; "FAIL selftest ${name}: $(Show-Ascii $_.Exception.Message 0)" }
    }
}
finally {
    foreach ($s in $held) { $s.Dispose() }
    try {
        if ([System.IO.Directory]::Exists($base)) { [System.IO.Directory]::Delete($base, $true) }
        if ([System.IO.Directory]::Exists($top) -and -not [System.IO.Directory]::EnumerateFileSystemEntries($top).GetEnumerator().MoveNext()) {
            [System.IO.Directory]::Delete($top)
        }
    }
    catch { $failed++; "FAIL selftest cleanup: $(Show-Ascii $_.Exception.Message 0)" }
}
exit ([int]($failed -gt 0))
