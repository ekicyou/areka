<#
.SYNOPSIS
  Shared tool helpers (load only): Show-Ascii, Get-ArekaVersion, Get-MsvcLinkPath, $AREKA_VERSION_PATTERN.

.DESCRIPTION
  Dot-source from a tool script:
    . (Join-Path $PSScriptRoot 'tool-common.ps1')   # from tools/
    . ./tools/tool-common.ps1                       # from the repo root (release.yml)

  Loads utf8-child.ps1 (Invoke-Utf8Child). Has no param block and no side effect besides
  defining functions and a constant: prints nothing, changes no location or environment,
  never reads or writes the console code page. Safe under Set-StrictMode -Version Latest.
#>

. (Join-Path $PSScriptRoot 'utf8-child.ps1')

# Accepted version shape (no + suffix).
$AREKA_VERSION_PATTERN = '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$'

# Printable form: non-ASCII and control chars become \uXXXX, text longer than $Max is cut
# ($Max 0 = never cut).
function Show-Ascii([string]$s, [int]$Max = 80) {
    $t = -join ($s.ToCharArray() | ForEach-Object {
            if ([int]$_ -ge 0x20 -and [int]$_ -le 0x7E) { $_ } else { '\u{0:X4}' -f [int]$_ }
        })
    if ($Max -gt 0 -and $t.Length -gt $Max) { $t.Substring(0, $Max - 3) + '...' } else { $t }
}

# Version of the areka package read with Invoke-Utf8Child cargo metadata. Throws (ASCII message).
# The repo root comes from this file's location, not from the caller's current location.
function Get-ArekaVersion {
    $root = Split-Path -Parent $PSScriptRoot
    $r = Invoke-Utf8Child cargo @('metadata', '--no-deps', '--locked', '--format-version', '1',
        '--manifest-path', (Join-Path $root 'Cargo.toml'))
    if ($r.Code) { throw "cargo metadata exited with code $($r.Code): $(Show-Ascii ([string]($r.ErrLines | Select-Object -Last 1)))" }
    try { $meta = $r.Out | ConvertFrom-Json } catch { throw 'cargo metadata output is not JSON' }
    $pkgs = @($meta.packages | Where-Object { $_.name -ceq 'areka' })
    if ($pkgs.Count -ne 1) { throw "expected exactly one areka package, got $($pkgs.Count)" }
    $v = [string]$pkgs[0].version
    if ($v -cnotmatch $AREKA_VERSION_PATTERN) {
        throw "areka version is not a version: $(Show-Ascii $v) (accepted: $AREKA_VERSION_PATTERN; no + suffix)"
    }
    $v
}

# Under Git Bash, GNU coreutils link.exe shadows the MSVC link.exe and i686 links fail.
# Returns the PATH string without folders that hold link.exe but not cl.exe.
function Get-MsvcLinkPath([string]$Path) {
    ($Path -split ';' | Where-Object {
            -not $_ -or -not (Test-Path (Join-Path $_ 'link.exe')) -or (Test-Path (Join-Path $_ 'cl.exe'))
        }) -join ';'
}
