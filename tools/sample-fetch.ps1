<#
.SYNOPSIS
  Fetch step for the sample ghost archives of the URL rows: fetch, verify, place, report counts.

.DESCRIPTION
  This file is ASCII only. Run from any directory:
    pwsh -NoProfile -File <repository>\tools\sample-fetch.ps1

  The rows come from `nar-sample-path --fetch-list` (one place for the URL, the checksum URL and
  the two shelf paths; this script never builds them). For each row, every run fetches the small
  published checksum file <URL>.sha256 and fetches the archive only when the shelf has none or the
  shelf archive does not match the latest checksum. Only bytes that match the published SHA-256
  are placed (archive first, then the checksum file).

  Output (ASCII only; outside text is written as \uXXXX):
    sample-fetch: <name>: present | fetched | FAILED ...
    sample-fetch: fetched <n>, present <n>, failed <n>
  Exit code: 0 = no failed row, 1 = a failed row, or the row list could not be read.
  Never reads or writes the console code page. Temporary files are <file>.part-<pid> next to the
  shelf files and are removed on every path except a successful move.
#>
#Requires -Version 7
param()

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# Invoke-Utf8Child: read a child's output as UTF-8 regardless of the console code page
. (Join-Path $PSScriptRoot 'utf8-child.ps1')

$Root = Split-Path -Parent $PSScriptRoot
$Curl = Join-Path $env:SystemRoot 'System32\curl.exe'
$FetchCommand = "pwsh -NoProfile -File `"$(Join-Path $Root 'tools\sample-fetch.ps1')`""

# Non-ASCII and control chars become \uXXXX (the idea of Show-Ascii in tools/encoding-check.ps1).
function ConvertTo-AsciiText([string]$s) {
    -join ($s.ToCharArray() | ForEach-Object {
            if ([int]$_ -ge 0x20 -and [int]$_ -le 0x7E) { $_ } else { '\u{0:X4}' -f [int]$_ }
        })
}

function Get-Sha256Hex([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

# sha256sum form, same rule as parse_checksum in crates/sample-ghost-kit/src/fetched.rs:
# the first word (after a BOM) is 64 hex digits; returned in lower case, else $null.
function Read-Checksum([string]$Path) {
    $text = [System.IO.File]::ReadAllText($Path).TrimStart([char]0xFEFF)
    $word = @($text -split '\s+' | Where-Object { $_ -ne '' } | Select-Object -First 1)
    if ($word.Count -eq 1 -and $word[0] -match '^[0-9a-fA-F]{64}$') { $word[0].ToLowerInvariant() } else { $null }
}

function Test-File([string]$Path) { Test-Path -LiteralPath $Path -PathType Leaf }

# Remove <file>.part-<pid> left by earlier runs whose process is gone; live runs' files are kept.
function Remove-StalePart([string]$Path) {
    $dir = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $dir -PathType Container)) { return }
    $leaf = Split-Path -Leaf $Path
    foreach ($f in @(Get-ChildItem -LiteralPath $dir -File | Where-Object { $_.Name.StartsWith("$leaf.part-", [System.StringComparison]::OrdinalIgnoreCase) })) {
        $tail = $f.Name.Substring($leaf.Length + '.part-'.Length)
        if ($tail -notmatch '^\d+$') { continue }
        $owner = [int]$tail
        if ($owner -eq $PID -or $null -ne (Get-Process -Id $owner -ErrorAction SilentlyContinue)) { continue }
        try { Remove-Item -LiteralPath $f.FullName -Force } catch { }
    }
}

# curl exit code + stderr in one ASCII string, or $null on success.
function Invoke-Curl([string]$Url, [string]$OutFile) {
    $r = Invoke-Utf8Child $Curl @('--fail', '--location', '--silent', '--show-error',
        '--connect-timeout', '30', '--speed-limit', '1', '--speed-time', '60',
        '--output', $OutFile, $Url)
    if ($r.Code -eq 0) { return $null }
    ConvertTo-AsciiText ("curl exit $($r.Code): " + ($r.ErrLines -join ' | '))
}

function Get-Remedy([string]$NarPath, [string]$ChecksumPath) {
    'remedy: run again: ' + (ConvertTo-AsciiText $FetchCommand) +
    ' ; or place both files by hand: ' + (ConvertTo-AsciiText $NarPath) + ' and ' +
    (ConvertTo-AsciiText $ChecksumPath) + ' (see vendors/sample_ghost/README.md)'
}

# One row of --fetch-list (5 columns). Returns Result = present | fetched | failed and the Lines to print.
# Takes any URL (the https:// rule is the caller's), so -SelfTest can pass file:/// rows.
function Sync-FetchedSample {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Url,
        [Parameter(Mandatory)][string]$ChecksumUrl,
        [Parameter(Mandatory)][string]$NarPath,
        [Parameter(Mandatory)][string]$ChecksumPath
    )
    $lines = [System.Collections.Generic.List[string]]::new()
    $n = ConvertTo-AsciiText $Name
    $u = ConvertTo-AsciiText $Url
    $cu = ConvertTo-AsciiText $ChecksumUrl
    $narTemp = "$NarPath.part-$PID"
    $sumTemp = "$ChecksumPath.part-$PID"
    function Done([string]$Result) { [pscustomobject]@{ Result = $Result; Lines = [string[]]$lines } }

    # 1. stale temporary files of dead runs
    Remove-StalePart $NarPath
    Remove-StalePart $ChecksumPath
    try {
        foreach ($d in @((Split-Path -Parent $NarPath), (Split-Path -Parent $ChecksumPath))) {
            New-Item -ItemType Directory -Force -Path $d | Out-Null
        }

        # 2. the latest checksum, every run
        $reason = Invoke-Curl $ChecksumUrl $sumTemp
        if ($null -ne $reason) {
            # 6. checksum unreachable: a consistent shelf pair is still present
            if ((Test-File $NarPath) -and (Test-File $ChecksumPath)) {
                $shelf = Read-Checksum $ChecksumPath
                if ($null -ne $shelf -and (Get-Sha256Hex $NarPath) -eq $shelf) {
                    $lines.Add("sample-fetch: ${n}: warning: could not check for a newer archive: $cu ($reason)")
                    $lines.Add("sample-fetch: ${n}: present")
                    return Done 'present'
                }
            }
            $lines.Add("sample-fetch: ${n}: FAILED could not fetch the checksum file $cu ($reason); $(Get-Remedy $NarPath $ChecksumPath)")
            return Done 'failed'
        }
        $latest = Read-Checksum $sumTemp
        if ($null -eq $latest) {
            $lines.Add("sample-fetch: ${n}: FAILED the checksum file $cu is not in sha256sum form (first word 64 hex digits)")
            return Done 'failed'
        }

        # 3. the shelf archive already matches: refresh only the checksum file
        if (Test-File $NarPath) {
            $have = Get-Sha256Hex $NarPath
            if ($have -eq $latest) {
                if (-not (Test-File $ChecksumPath) -or (Get-Sha256Hex $ChecksumPath) -ne (Get-Sha256Hex $sumTemp)) {
                    [System.IO.File]::Move($sumTemp, $ChecksumPath, $true)
                }
                $lines.Add("sample-fetch: ${n}: present")
                return Done 'present'
            }
            $lines.Add("sample-fetch: ${n}: shelf archive does not match the latest checksum (published $latest, shelf $have); fetching again")
        }

        # 4. fetch the archive
        $reason = Invoke-Curl $Url $narTemp
        if ($null -ne $reason) {
            $lines.Add("sample-fetch: ${n}: FAILED could not fetch $u ($reason); $(Get-Remedy $NarPath $ChecksumPath)")
            return Done 'failed'
        }

        # 5. verify, then move the archive first and the checksum file second
        $got = Get-Sha256Hex $narTemp
        if ($got -ne $latest) {
            $lines.Add("sample-fetch: ${n}: FAILED SHA-256 mismatch for $u (published $latest, got $got); not placed")
            return Done 'failed'
        }
        [System.IO.File]::Move($narTemp, $NarPath, $true)
        [System.IO.File]::Move($sumTemp, $ChecksumPath, $true)
        $lines.Add("sample-fetch: ${n}: fetched")
        return Done 'fetched'
    }
    finally {
        foreach ($t in @($narTemp, $sumTemp)) {
            if (Test-File $t) { Remove-Item -LiteralPath $t -Force }
        }
    }
}

# --- the row list -------------------------------------------------------------------------
Push-Location -LiteralPath $Root
try {
    $list = Invoke-Utf8Child cargo @('run', '--quiet', '-p', 'sample-ghost-kit', '--bin', 'nar-sample-path', '--', '--fetch-list')
}
catch {
    "sample-fetch: could not run nar-sample-path --fetch-list: $(ConvertTo-AsciiText $_.Exception.Message)"
    exit 1
}
finally {
    Pop-Location
}
if ($list.Code -ne 0) {
    "sample-fetch: nar-sample-path --fetch-list exited with code $($list.Code): $(ConvertTo-AsciiText ($list.ErrLines -join ' | '))"
    exit 1
}
$rows = @()
foreach ($line in @($list.Out -split '\r?\n' | Where-Object { $_ -ne '' })) {
    $cols = $line -split "`t"
    if ($cols.Count -ne 5) {
        "sample-fetch: nar-sample-path --fetch-list gave a line without 5 tab-separated columns: $(ConvertTo-AsciiText $line)"
        exit 1
    }
    $rows += , $cols
}

# --- each row -----------------------------------------------------------------------------
$count = @{ fetched = 0; present = 0; failed = 0 }
foreach ($c in $rows) {
    $n = ConvertTo-AsciiText $c[0]
    if (-not ($c[1].StartsWith('https://', [System.StringComparison]::Ordinal) -and $c[2].StartsWith('https://', [System.StringComparison]::Ordinal))) {
        "sample-fetch: ${n}: FAILED only https:// URLs are fetched: $(ConvertTo-AsciiText $c[1]) $(ConvertTo-AsciiText $c[2])"
        $count.failed++
        continue
    }
    try {
        $res = Sync-FetchedSample -Name $c[0] -Url $c[1] -ChecksumUrl $c[2] -NarPath $c[3] -ChecksumPath $c[4]
        $res.Lines
        $count[$res.Result]++
    }
    catch {
        "sample-fetch: ${n}: FAILED unexpected error for $(ConvertTo-AsciiText $c[1]): $(ConvertTo-AsciiText $_.Exception.Message); $(Get-Remedy $c[3] $c[4])"
        $count.failed++
    }
}
"sample-fetch: fetched $($count.fetched), present $($count.present), failed $($count.failed)"
exit ([int]($count.failed -gt 0))
