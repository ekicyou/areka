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

  -SelfTest: the 8 checks of the fetch step without network (fake file:/// rows through the real
  curl.exe, in target\sample-fetch-selftest\<pid>\, removed at the end). Prints PASS/FAIL <name>
  per check; exit 0 only when all pass.
#>
#Requires -Version 7
param([switch]$SelfTest)

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

# All rows: per-row lines, then the count line; $ExitCode gets 0 (no failed row) or 1.
# -AnyScheme skips the https:// rule (only -SelfTest passes it, for its file:/// rows).
function Invoke-FetchRows {
    param(
        [Parameter(Mandatory)][AllowEmptyCollection()][object[]]$Rows,
        [Parameter(Mandatory)][ref]$ExitCode,
        [switch]$AnyScheme
    )
    $count = @{ fetched = 0; present = 0; failed = 0 }
    foreach ($c in $Rows) {
        $n = ConvertTo-AsciiText $c[0]
        if (-not $AnyScheme -and -not ($c[1].StartsWith('https://', [System.StringComparison]::Ordinal) -and $c[2].StartsWith('https://', [System.StringComparison]::Ordinal))) {
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
    $ExitCode.Value = [int]($count.failed -gt 0)
}

# --- -SelfTest: the 8 checks, through the real curl.exe on file:/// rows, no network ------
if ($SelfTest) {
    $script:fails = 0
    $script:all = [System.Collections.Generic.List[string]]::new()
    function Judge([string]$Name, [bool]$Ok) {
        if ($Ok) { "PASS $Name" } else { $script:fails++; "FAIL $Name" }
    }
    $ABC_SHA = 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'  # FIPS 180-2 SHA-256("abc")
    $base = Join-Path $Root 'target\sample-fetch-selftest'
    $place = Join-Path $base "$PID"

    # source folder <dir>: <name>.nar = $Text, <name>.nar.sha256 = "$Sha  <name>.nar" (sha256sum form)
    function Set-Source([string]$Dir, [string]$Name, [string]$Text, [string]$Sha) {
        New-Item -ItemType Directory -Force -Path $Dir | Out-Null
        [System.IO.File]::WriteAllText((Join-Path $Dir "$Name.nar"), $Text, [System.Text.Encoding]::ASCII)
        [System.IO.File]::WriteAllText((Join-Path $Dir "$Name.nar.sha256"), "$Sha  $Name.nar`n", [System.Text.Encoding]::ASCII)
    }
    function Get-FileUrl([string]$Path) { ([System.Uri]$Path).AbsoluteUri }
    # a fake 5-column row: source <SrcDir>, shelf <ShelfDir>; -NoChecksum points the checksum URL nowhere
    function New-Row([string]$Name, [string]$SrcDir, [string]$ShelfDir, [switch]$NoChecksum) {
        $nar = Join-Path $SrcDir "$Name.nar"
        $sum = if ($NoChecksum) { Join-Path $SrcDir "missing\$Name.nar.sha256" } else { "$nar.sha256" }
        , @($Name, (Get-FileUrl $nar), (Get-FileUrl $sum), (Join-Path $ShelfDir "$Name.nar"), (Join-Path $ShelfDir "$Name.nar.sha256"))
    }
    function Invoke-Row($Row) {
        $r = Sync-FetchedSample -Name $Row[0] -Url $Row[1] -ChecksumUrl $Row[2] -NarPath $Row[3] -ChecksumPath $Row[4]
        foreach ($l in $r.Lines) { $script:all.Add($l) }
        $r
    }
    function Get-Files([string]$Dir) {
        if (-not (Test-Path -LiteralPath $Dir)) { return @() }
        @(Get-ChildItem -LiteralPath $Dir -File | ForEach-Object Name | Sort-Object)
    }
    function Test-SameBytes([string]$A, [string]$B) {
        (Test-File $A) -and (Test-File $B) -and (Get-Sha256Hex $A) -eq (Get-Sha256Hex $B)
    }

    try {
        $src = Join-Path $place 'source'
        $shelf = Join-Path $place 'shelf'

        # 1. empty shelf + good source -> fetched, both files equal the source, no .part-*
        Set-Source $src 'one' 'abc' $ABC_SHA
        $row = New-Row 'one' $src $shelf
        $r = Invoke-Row $row
        Judge '1 empty shelf fetches the archive and the checksum file' (
            $r.Result -eq 'fetched' -and
            ((Get-Files $shelf) -join ',') -ceq 'one.nar,one.nar.sha256' -and
            (Test-SameBytes $row[3] (Join-Path $src 'one.nar')) -and
            (Test-SameBytes $row[4] (Join-Path $src 'one.nar.sha256')))

        # 2. second run with the source archive gone (only its .sha256 kept) -> present, no re-download
        Remove-Item -LiteralPath (Join-Path $src 'one.nar') -Force
        $r = Invoke-Row $row
        Judge '2 second run is present without fetching the archive again' (
            $r.Result -eq 'present' -and ((Get-Files $shelf) -join ',') -ceq 'one.nar,one.nar.sha256' -and
            (Get-Sha256Hex $row[3]) -eq $ABC_SHA)

        # 3. a new source version -> fetched again, the shelf pair is the new version
        $defPath = Join-Path $place 'def.tmp'
        [System.IO.File]::WriteAllText($defPath, 'def', [System.Text.Encoding]::ASCII)
        $defSha = Get-Sha256Hex $defPath
        Remove-Item -LiteralPath $defPath -Force
        Set-Source $src 'one' 'def' $defSha
        $r = Invoke-Row $row
        Judge '3 a new source version is fetched again' (
            $r.Result -eq 'fetched' -and ((Get-Files $shelf) -join ',') -ceq 'one.nar,one.nar.sha256' -and
            (Test-SameBytes $row[3] (Join-Path $src 'one.nar')) -and
            (Test-SameBytes $row[4] (Join-Path $src 'one.nar.sha256')))

        # 4. source archive not matching its .sha256 -> failed, nothing placed, names both SHA-256
        $shelf4 = Join-Path $place 'shelf4'
        Set-Source $src 'bad' 'xyz' $ABC_SHA
        $row4 = New-Row 'bad' $src $shelf4
        $r = Invoke-Row $row4
        $text = $r.Lines -join "`n"
        Judge '4 a mismatching archive fails and nothing is placed' (
            $r.Result -eq 'failed' -and @(Get-Files $shelf4).Count -eq 0 -and
            $text.Contains('bad') -and $text.Contains($row4[1]) -and
            $text.Contains("published $ABC_SHA") -and $text.Contains('got ' + (Get-Sha256Hex (Join-Path $src 'bad.nar'))))

        # 5. checksum unreachable + a consistent shelf pair -> present with one warning line
        $r = Invoke-Row (New-Row 'one' $src $shelf -NoChecksum)
        Judge '5 unreachable checksum with a consistent shelf is present with a warning' (
            $r.Result -eq 'present' -and
            @($r.Lines | Where-Object { $_.Contains('could not check for a newer archive') }).Count -eq 1)

        # 6. checksum unreachable + empty shelf -> failed, nothing left, name/URL/reason/remedy
        $shelf6 = Join-Path $place 'shelf6'
        $row6 = New-Row 'one' $src $shelf6 -NoChecksum
        $r = Invoke-Row $row6
        $text = $r.Lines -join "`n"
        Judge '6 unreachable checksum with an empty shelf fails' (
            $r.Result -eq 'failed' -and @(Get-Files $shelf6).Count -eq 0 -and
            $text.Contains('one') -and $text.Contains($row6[2]) -and $text.Contains('curl exit') -and $text.Contains('remedy:'))

        # 7. three rows (fetched, present, failed) -> 1, 1, 1 and exit 1; all success -> exit 0
        $src7 = Join-Path $place 'source7'
        $shelf7 = Join-Path $place 'shelf7'
        Set-Source $src7 'one' 'abc' $ABC_SHA
        Set-Source $src7 'two' 'abc' $ABC_SHA
        $okRows = @((New-Row 'one' $src7 $shelf7), (New-Row 'two' $src7 $shelf7))
        $code = -1
        $null = Invoke-FetchRows -Rows (, $okRows[0]) -ExitCode ([ref]$code) -AnyScheme   # 'one' on the shelf
        $code = -1
        $out = @(Invoke-FetchRows -Rows @($okRows[1], $okRows[0], (New-Row 'three' $src7 $shelf7 -NoChecksum)) -ExitCode ([ref]$code) -AnyScheme)
        foreach ($l in $out) { $script:all.Add($l) }
        $mixed = ($out[-1] -ceq 'sample-fetch: fetched 1, present 1, failed 1') -and $code -eq 1
        $code = -1
        $out = @(Invoke-FetchRows -Rows $okRows -ExitCode ([ref]$code) -AnyScheme)
        foreach ($l in $out) { $script:all.Add($l) }
        Judge '7 three rows count 1, 1, 1 with exit 1 and all success exits 0' (
            $mixed -and ($out[-1] -ceq 'sample-fetch: fetched 0, present 2, failed 0') -and $code -eq 0)

        # 8. every line above is ASCII only
        Judge '8 all output is ASCII only' (
            $script:all.Count -gt 0 -and @($script:all | Where-Object { $_ -notmatch '^[\x20-\x7E]*$' }).Count -eq 0)
    }
    catch {
        $script:fails++
        "FAIL selftest stopped: $(ConvertTo-AsciiText $_.Exception.Message)"
    }
    finally {
        if (Test-Path -LiteralPath $place) { Remove-Item -LiteralPath $place -Recurse -Force }
        if ((Test-Path -LiteralPath $base) -and @(Get-ChildItem -LiteralPath $base -Force).Count -eq 0) {
            Remove-Item -LiteralPath $base -Force
        }
    }
    if ($script:fails) { "selftest: $($script:fails) failure(s)"; exit 1 }
    'selftest: all passed'
    exit 0
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
$code = 1
Invoke-FetchRows -Rows $rows -ExitCode ([ref]$code)
exit $code
