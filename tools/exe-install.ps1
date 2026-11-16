<#
.SYNOPSIS
  Shared install steps for the desk tools (load only): Test-InstallHome, Build-InstallExe,
  Install-ToolExe, Get-StateStatusAction.

.DESCRIPTION
  Dot-source from impl-watch-install.ps1 / mcp-bridge-install.ps1:
    . (Join-Path $PSScriptRoot 'exe-install.ps1')

  Loads tool-common.ps1 only. Has no param block, prints nothing on load, never prints or sets
  an environment variable. In the install folder it touches only <name>.exe, <name>.new.exe and
  <name>.old*.exe. Returned lines are ASCII only. Safe under Set-StrictMode -Version Latest.

  Install-ToolExe checks the new exe as <name>.new.exe BEFORE replacing the current one:
  a failed check leaves the current exe and the old copies as they were.
#>

. (Join-Path $PSScriptRoot 'tool-common.ps1')

# Step 1: the install folder from an environment variable. Returns a FAIL line or $null (never the value).
function Test-InstallHome {
    param([string]$VarName, [AllowNull()][string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) { return "FAIL home: $VarName is not set" }
    if (-not [System.IO.Path]::IsPathFullyQualified($Value)) { return "FAIL home: $VarName is not an absolute path" }
    $null
}

# Step 2: release build; the executable path comes from cargo's JSON messages (independent of CARGO_TARGET_DIR).
# Returns @{ Path = built exe or $null; Lines = cargo's stderr lines and the FAIL line }.
function Build-InstallExe {
    param([Parameter(Mandatory)][string]$Name, [Parameter(Mandatory)][string]$Root)
    $r = Invoke-Utf8Child cargo @('build', '--release', '-q', '-p', $Name,
        '--manifest-path', (Join-Path $Root 'Cargo.toml'), '--message-format=json-render-diagnostics')
    $lines = [System.Collections.Generic.List[string]]::new()
    foreach ($l in $r.ErrLines) { $lines.Add($l) }
    if ($r.Code -ne 0) {
        $lines.Add("FAIL build: cargo exit $($r.Code)")
        return [pscustomobject]@{ Path = $null; Lines = [string[]]$lines }
    }
    $built = $null
    foreach ($line in $r.Out -split '\r?\n') {
        if (-not $line.StartsWith('{')) { continue }
        $m = $line | ConvertFrom-Json
        if ($m.reason -eq 'compiler-artifact' -and $m.target.name -eq $Name -and $m.executable) {
            $built = [string]$m.executable
        }
    }
    if (-not $built) { $lines.Add('FAIL build: no executable in cargo output') }
    [pscustomobject]@{ Path = $built; Lines = [string[]]$lines }
}

# Steps 3-6: copy as .new.exe -> check it -> replace -> sweep old copies.
# $Check: param([string]$ExePath) -> @{ Ok = [bool]; Line = 'OK check: ...' | 'FAIL check: ...' }.
# A $Check that kills its child on a timeout waits for the child to end before it returns.
# Returns @{ Code = 0 | 1; Lines = [string[]] }.
function Install-ToolExe {
    param([Parameter(Mandatory)][string]$HomeDir, [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Built, [Parameter(Mandatory)][scriptblock]$Check)
    $lines = [System.Collections.Generic.List[string]]::new()
    $done = { param([int]$Code) [pscustomobject]@{ Code = $Code; Lines = [string[]]$lines } }
    $exe = Join-Path $HomeDir "$Name.exe"
    $new = Join-Path $HomeDir "$Name.new.exe"

    # 3 copy
    try {
        [void][System.IO.Directory]::CreateDirectory($HomeDir)
        [System.IO.File]::Copy($Built, $new, $true)
    }
    catch {
        $lines.Add('FAIL place: copy')
        return & $done 1
    }

    # 4 check the new exe before anything else changes
    try {
        $c = & $Check $new
        $ok = [bool]$c.Ok
        $checkLine = [string]$c.Line
    }
    catch {
        $ok = $false
        $checkLine = "FAIL check: cannot start: $(Show-Ascii $_.Exception.Message 0)"
    }
    $lines.Add($checkLine)
    if (-not $ok) {
        try { if ([System.IO.File]::Exists($new)) { [System.IO.File]::Delete($new) } }
        catch { $lines.Add("FAIL cleanup: cannot remove $Name.new.exe: $(Show-Ascii $_.Exception.Message 0)") }
        return & $done 1
    }

    # 5 rename current to .old-<UTC>.exe (works while it runs), then .new.exe to the real name
    $oldName = "$Name.old-$([DateTime]::UtcNow.ToString('yyyyMMdd\THHmmss\Z')).exe"
    $old = Join-Path $HomeDir $oldName
    $moved = $false
    try {
        if ([System.IO.File]::Exists($exe)) {
            [System.IO.File]::Move($exe, $old)
            $moved = $true
        }
        [System.IO.File]::Move($new, $exe)
    }
    catch {
        $fail = 'FAIL place: rename'
        try { if ($moved) { [System.IO.File]::Move($old, $exe) } } catch { $fail += '; restore failed' }
        try { [System.IO.File]::Delete($new) } catch { }
        $lines.Add($fail)
        return & $done 1
    }
    if ($moved) { $lines.Add("OK renamed previous to $oldName") }
    $lines.Add("OK placed $Name.exe")

    # 6 sweep old copies only after the replace; a running one cannot be deleted and stays for the next run
    foreach ($f in [System.IO.Directory]::GetFiles($HomeDir, "$Name.old*.exe")) {
        $leaf = [System.IO.Path]::GetFileName($f)
        try {
            [System.IO.File]::Delete($f)
            $lines.Add("OK removed $leaf")
        }
        catch {
            $lines.Add("KEEP $leaf (in use; run again later)")
        }
    }
    & $done 0
}

# impl-watch step 7 (pure): what to do after `status` exited with $Code.
# 'none' (exit 0) | 'clear' (state file of another version) | 'fail' (any other failure)
function Get-StateStatusAction {
    param([int]$Code, [string[]]$ErrLines)
    if ($Code -eq 0) { return 'none' }
    if (@($ErrLines) -match '^state file version mismatch') { return 'clear' }
    'fail'
}
