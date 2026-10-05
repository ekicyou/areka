<#
.SYNOPSIS
  Encoding check for the release tools: calibration, then static rules over the tool scripts
  and the workflow run bodies. Exit code 0 = all passed, 1 = at least one failure.

.DESCRIPTION
  This file is ASCII only (it is one of its own targets). Japanese text in the samples is built
  with `u{...} escapes, so the source text of every token stays ASCII.

  1. Calibration: every rule is run on built-in samples that must pass and samples that must fail.
     Samples are plain strings (single quotes or concatenation), so they are never code here.
  2. Static rules over tools/*.ps1 (not tools/perf/) and the run: bodies of .github/workflows/*.yml
     (a step's `run: |` block or one-line scalar; a mapping-valued run: such as defaults.run is
     not a body). Each body is parsed as pwsh; a parse error is a failure.
       A  a non-comment token whose source text has a char outside tab, CR, LF, 0x20-0x7E
          (except inside the right-hand side of an assignment to $LOG_MARKER_*)
       B  a $LOG_MARKER_* variable inside an expandable string or here-string
       C  an assignment to [Console]::OutputEncoding / InputEncoding ([System.Console] too) or
          $OutputEncoding, or a chcp / chcp.com command (only exception: inside the function
          Set-OwnConsoleCp932 in tools/encoding-check.ps1); the string after -Command / -c of a
          child pwsh / powershell is parsed and checked the same way
       D  a cargo / cargo.exe command (bare or with &) with the element metadata or nar-sample-path
       E  tools/package.ps1 (metadata and nar-sample-path), tools/crates-io.ps1 (metadata) and the
          run bodies of .github/workflows/release.yml (metadata) each have an Invoke-Utf8Child
          call with that element
  All failures are collected and printed, then the exit code is decided once.
  Writes nothing, creates no temporary files, prints ASCII only, never prints the environment.

  Run: pwsh -NoProfile -File tools/encoding-check.ps1
#>
using namespace System.Management.Automation.Language
#Requires -Version 7

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$NonAscii = '[^\t\r\n\x20-\x7E]'
$SelfWhere = 'tools/encoding-check.ps1'
$CallWordSet = @('metadata', 'nar-sample-path')
# Rule E: where an Invoke-Utf8Child call must carry each element
$RequiredCalls = [ordered]@{
    'tools/package.ps1'             = @('metadata', 'nar-sample-path')
    'tools/crates-io.ps1'           = @('metadata')
    '.github/workflows/release.yml' = @('metadata')
}

# Printable form: non-ASCII and control chars become \uXXXX, long text is cut.
function Show-Ascii([string]$s) {
    $t = -join ($s.ToCharArray() | ForEach-Object {
            if ([int]$_ -ge 0x20 -and [int]$_ -le 0x7E) { $_ } else { '\u{0:X4}' -f [int]$_ }
        })
    if ($t.Length -gt 80) { $t.Substring(0, 77) + '...' } else { $t }
}

# Leaf name of a command (cargo, & 'cargo.exe', C:\x\chcp.com), or $null.
function Get-CommandLeaf([CommandAst]$Cmd) {
    $name = $Cmd.GetCommandName()
    if ($null -eq $name) { return $null }
    [System.IO.Path]::GetFileName($name)
}

# String constants among the arguments of a command (not its name), e.g. metadata in @('metadata').
function Get-ArgWords([CommandAst]$Cmd) {
    foreach ($e in @($Cmd.CommandElements | Select-Object -Skip 1)) {
        foreach ($c in $e.FindAll({ param($n) $n -is [StringConstantExpressionAst] }, $true)) { $c.Value }
    }
}

function Test-InFunction([Ast]$Node, [string]$Name) {
    for ($p = $Node.Parent; $null -ne $p; $p = $p.Parent) {
        if ($p -is [FunctionDefinitionAst] -and $p.Name -eq $Name) { return $true }
    }
    $false
}

# Rule C on one syntax tree: console encoding assignments and chcp commands, plus the same check on
# the string a child PowerShell gets after -Command / -c (parsed, not searched as text; reported at
# the line of the child's command). Returns objects with Line and Text.
function Get-ConsoleRewrites([Ast]$Ast, [string]$Where) {
    foreach ($n in $Ast.FindAll({ param($n) $n -is [AssignmentStatementAst] }, $true)) {
        $l = $n.Left
        if ($l -is [ConvertExpressionAst]) { $l = $l.Child }   # [Text.Encoding]$OutputEncoding = ...
        $hit = ($l -is [VariableExpressionAst] -and $l.VariablePath.UserPath.Split(':')[-1] -eq 'OutputEncoding') -or
            ($l -is [MemberExpressionAst] -and $l.Static -and $l.Expression -is [TypeExpressionAst] -and
            $l.Expression.TypeName.FullName -in @('Console', 'System.Console') -and
            $l.Member -is [StringConstantExpressionAst] -and $l.Member.Value -in @('OutputEncoding', 'InputEncoding'))
        if ($hit -and -not ($Where -eq $SelfWhere -and (Test-InFunction $n 'Set-OwnConsoleCp932'))) {
            [pscustomobject]@{ Line = $n.Extent.StartLineNumber; Text = $n.Extent.Text }
        }
    }
    foreach ($c in $Ast.FindAll({ param($n) $n -is [CommandAst] }, $true)) {
        $leaf = Get-CommandLeaf $c
        if ($leaf -in @('chcp', 'chcp.com') -and -not ($Where -eq $SelfWhere -and (Test-InFunction $c 'Set-OwnConsoleCp932'))) {
            [pscustomobject]@{ Line = $c.Extent.StartLineNumber; Text = $c.Extent.Text }
        }
        if ($leaf -notin @('pwsh', 'pwsh.exe', 'powershell', 'powershell.exe')) { continue }
        $el = $c.CommandElements
        for ($i = 1; $i -lt $el.Count - 1; $i++) {
            $arg = $el[$i + 1]
            if (-not ($el[$i] -is [CommandParameterAst] -and $el[$i].ParameterName -in @('Command', 'c'))) { continue }
            if (-not ($arg -is [StringConstantExpressionAst] -or $arg -is [ExpandableStringExpressionAst])) { continue }
            $tk = $null; $er = $null
            $inner = [Parser]::ParseInput($arg.Value, [ref]$tk, [ref]$er)
            if (@(Get-ConsoleRewrites $inner '').Count) {
                [pscustomobject]@{ Line = $c.Extent.StartLineNumber; Text = $c.Extent.Text }
            }
        }
    }
}

# Rules A-D (and parse errors) on one pwsh text. Returns the findings and the elements of
# Invoke-Utf8Child calls (for rule E).
function Get-Findings {
    param([string]$Text, [string]$Where, [int]$LineOffset = 0)
    $tokens = $null; $errors = $null
    $ast = [Parser]::ParseInput($Text, [ref]$tokens, [ref]$errors)
    $found = [System.Collections.Generic.List[object]]::new()
    $add = { param($rule, $line, $what) $found.Add([pscustomobject]@{ Rule = $rule; Line = $line + $LineOffset; What = $what }) }

    foreach ($e in $errors) { & $add 'parse' $e.Extent.StartLineNumber $e.Message }

    # A
    $exempt = @($ast.FindAll({ param($n)
                $n -is [AssignmentStatementAst] -and $n.Left -is [VariableExpressionAst] -and
                $n.Left.VariablePath.UserPath.Split(':')[-1] -like 'LOG_MARKER_*' }, $true) | ForEach-Object { $_.Right.Extent })
    foreach ($t in $tokens) {
        if ($t.Kind -eq [TokenKind]::Comment -or $t.Extent.Text -notmatch $NonAscii) { continue }
        $inMarker = @($exempt | Where-Object { $t.Extent.StartOffset -ge $_.StartOffset -and $t.Extent.EndOffset -le $_.EndOffset }).Count -gt 0
        if (-not $inMarker) { & $add 'A' $t.Extent.StartLineNumber $t.Extent.Text }
    }

    # B
    foreach ($v in $ast.FindAll({ param($n) $n -is [VariableExpressionAst] -and $n.VariablePath.UserPath.Split(':')[-1] -like 'LOG_MARKER_*' }, $true)) {
        for ($p = $v.Parent; $null -ne $p; $p = $p.Parent) {
            if ($p -is [ExpandableStringExpressionAst]) { & $add 'B' $v.Extent.StartLineNumber $v.Extent.Text; break }
        }
    }

    # C
    foreach ($h in Get-ConsoleRewrites $ast $Where) { & $add 'C' $h.Line $h.Text }

    # D, and the Invoke-Utf8Child elements for E
    $callWords = [System.Collections.Generic.List[string]]::new()
    foreach ($c in $ast.FindAll({ param($n) $n -is [CommandAst] }, $true)) {
        $leaf = Get-CommandLeaf $c
        if ($null -eq $leaf) { continue }
        $words = @(Get-ArgWords $c | Where-Object { $_ -in $CallWordSet })
        if ($leaf -in @('cargo', 'cargo.exe') -and $words.Count) { & $add 'D' $c.Extent.StartLineNumber $c.Extent.Text }
        if ($leaf -eq 'Invoke-Utf8Child') { foreach ($w in $words) { $callWords.Add($w) } }
    }

    [pscustomobject]@{ Findings = $found.ToArray(); CallWords = $callWords.ToArray() }
}

# run: bodies of a workflow: Line = 1-based workflow line of the body's first line.
function Get-RunBodies([string]$Yaml) {
    $lines = $Yaml -split '\r?\n'
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -notmatch '^(?<lead>\s*(-\s+)?)run:[ \t]*(?<val>.*?)[ \t]*$') { continue }
        $col = $Matches['lead'].Length
        $val = $Matches['val']
        if ($val -eq '' -or $val.StartsWith('#')) { continue }   # mapping value (defaults.run): not a body
        if ($val -notmatch '^\|[-+]?[ \t]*(#.*)?$') {
            [pscustomobject]@{ Line = $i + 1; Text = $val }
            continue
        }
        # block: lines deeper than the run: key (and blank lines), de-indented like YAML does
        $body = [System.Collections.Generic.List[string]]::new()
        $j = $i + 1
        while ($j -lt $lines.Count -and ($lines[$j] -match '^\s*$' -or ($lines[$j].Length - $lines[$j].TrimStart().Length) -gt $col)) {
            $body.Add($lines[$j]); $j++
        }
        $first = $body | Where-Object { $_ -notmatch '^\s*$' } | Select-Object -First 1
        $indent = if ($first) { $first.Length - $first.TrimStart().Length } else { 0 }
        $text = ($body | ForEach-Object { if ($_.Length -ge $indent) { $_.Substring($indent) } else { '' } }) -join "`n"
        [pscustomobject]@{ Line = $i + 2; Text = $text }
        $i = $j - 1
    }
}

# All findings of one target (ps1 text or workflow), plus its Invoke-Utf8Child elements.
function Get-TargetResult([string]$Text, [string]$Where, [bool]$IsYaml) {
    if (-not $IsYaml) { return Get-Findings $Text $Where }
    $f = [System.Collections.Generic.List[object]]::new()
    $w = [System.Collections.Generic.List[string]]::new()
    foreach ($b in Get-RunBodies $Text) {
        $r = Get-Findings $b.Text $Where ($b.Line - 1)
        foreach ($x in $r.Findings) { $f.Add($x) }
        foreach ($x in $r.CallWords) { $w.Add($x) }
    }
    [pscustomobject]@{ Findings = $f.ToArray(); CallWords = $w.ToArray() }
}

$fails = [System.Collections.Generic.List[string]]::new()

# ---- 1. calibration ----
$jp = "`u{65E5}`u{672C}"
$nl = "`n"
$calib = @(
    # must fail
    @{ Name = 'bare cargo metadata'; Text = '$x = @(cargo metadata --no-deps 2>&1)'; Expect = 'D' }
    @{ Name = 'bare cargo with & and nar-sample-path'; Text = "& 'cargo.exe' run --bin nar-sample-path"; Expect = 'D' }
    @{ Name = 'japanese Write-Host'; Text = "Write-Host '" + $jp + "'"; Expect = 'A' }
    @{ Name = 'japanese expandable string'; Text = '"' + $jp + ' $y"'; Expect = 'A' }
    @{ Name = 'Console OutputEncoding assignment'; Text = '[Console]::OutputEncoding = [Text.Encoding]::UTF8'; Expect = 'C' }
    @{ Name = 'System.Console InputEncoding assignment'; Text = '[System.Console]::InputEncoding = [Text.Encoding]::UTF8'; Expect = 'C' }
    @{ Name = 'OutputEncoding assignment'; Text = '$OutputEncoding = [Text.Encoding]::UTF8'; Expect = 'C' }
    @{ Name = 'chcp'; Text = 'chcp 65001'; Expect = 'C' }
    @{ Name = 'rewrite inside child pwsh -Command'; Text = 'pwsh -NoProfile -Command ''[Console]::OutputEncoding = [Text.Encoding]::UTF8; & ./tools/package.ps1'''; Expect = 'C' }
    @{ Name = 'chcp inside child pwsh -c'; Text = 'pwsh -c "chcp 65001"'; Expect = 'C' }
    @{ Name = 'Set-OwnConsoleCp932 outside encoding-check'; Where = 'tools/package.ps1'; Text = 'function Set-OwnConsoleCp932 { chcp 932 }'; Expect = 'C' }
    @{ Name = 'marker inside expandable string'; Text = '"$LOG_MARKER_A ' + $jp + '"'; Expect = 'B' }
    @{ Name = 'parse error'; Text = 'if ('; Expect = 'parse' }
    @{ Name = 'missing Invoke-Utf8Child call'; Text = 'cargo metadata'; Need = @('metadata'); Expect = 'E' }
    @{ Name = 'Invoke-Utf8Child without metadata'; Text = "Invoke-Utf8Child cargo @('run', '--bin', 'nar-sample-path')"; Need = @('metadata', 'nar-sample-path'); Expect = 'E' }
    @{ Name = 'yaml run block with japanese'; Yaml = $true; Text = "jobs:${nl}  a:${nl}    steps:${nl}      - name: x${nl}        run: |${nl}          Write-Host '" + $jp + "'${nl}"; Expect = 'A' }
    @{ Name = 'yaml one-line run with japanese'; Yaml = $true; Text = "steps:${nl}  - run: Write-Host '" + $jp + "'${nl}"; Expect = 'A' }
    # must pass
    @{ Name = 'comment'; Text = '# ' + $jp; Expect = $null }
    @{ Name = 'block comment'; Text = '<# ' + $jp + ' #>'; Expect = $null }
    @{ Name = 'marker assignment'; Text = '$LOG_MARKER_A = ''' + $jp + ''''; Expect = $null }
    @{ Name = 'marker array assignment'; Text = '$LOG_MARKER_B = @(''' + $jp + ''', ''x'')'; Expect = $null }
    @{ Name = 'child pwsh -File'; Text = 'pwsh -NoProfile -NonInteractive -File ./tools/package.ps1 -Arch all'; Expect = $null }
    @{ Name = 'Invoke-Utf8Child metadata'; Text = "Invoke-Utf8Child cargo @('metadata')"; Need = @('metadata'); Expect = $null }
    @{ Name = 'Set-OwnConsoleCp932 in encoding-check'; Where = $SelfWhere; Text = 'function Set-OwnConsoleCp932 { [Console]::OutputEncoding = [Text.Encoding]::GetEncoding(932); chcp 932 }'; Expect = $null }
    @{ Name = 'yaml name and comment only'; Yaml = $true; Text = "# " + $jp + "${nl}steps:${nl}  - name: " + $jp + "${nl}"; Expect = $null }
    @{ Name = 'yaml block ends at the next key'; Yaml = $true; Text = "steps:${nl}  - run: |${nl}      Write-Host 'ok'${nl}    name: " + $jp + "${nl}  - name: " + $jp + "${nl}"; Expect = $null }
    @{ Name = 'yaml defaults run mapping'; Yaml = $true; Text = "defaults:${nl}  run:${nl}    shell: pwsh${nl}    working-directory: " + $jp + "${nl}"; Expect = $null }
)
foreach ($s in $calib) {
    $where = if ($s.ContainsKey('Where')) { $s.Where } else { 'calibration' }
    $r = Get-TargetResult $s.Text $where ($s.ContainsKey('Yaml'))
    $rules = @($r.Findings | ForEach-Object Rule)
    if ($s.ContainsKey('Need') -and @($s.Need | Where-Object { $_ -notin $r.CallWords }).Count) { $rules += 'E' }
    $ok = if ($null -eq $s.Expect) { $rules.Count -eq 0 } else { $rules -contains $s.Expect }
    if (-not $ok) {
        $exp = if ($null -eq $s.Expect) { 'pass' } else { $s.Expect }
        $fails.Add("FAIL calibration ${exp}: $($s.Name) (got: $(if ($rules.Count) { $rules -join ',' } else { 'none' }))")
    }
}
if ($fails.Count -eq 0) { "PASS calibration ($($calib.Count) samples)" }

# ---- 2. static rules over the real files ----
$root = Split-Path -Parent $PSScriptRoot
$targets = @(Get-ChildItem -LiteralPath (Join-Path $root 'tools') -Filter '*.ps1' -File) +
@(Get-ChildItem -LiteralPath (Join-Path $root '.github/workflows') -Filter '*.yml' -File)
$callWordsOf = @{}
foreach ($file in $targets) {
    $where = [System.IO.Path]::GetRelativePath($root, $file.FullName).Replace('\', '/')
    $r = Get-TargetResult (Get-Content -LiteralPath $file.FullName -Raw -Encoding utf8) $where ($file.Extension -eq '.yml')
    $callWordsOf[$where] = $r.CallWords
    foreach ($x in $r.Findings) { $fails.Add("FAIL ${where}:$($x.Line): $($x.Rule) $(Show-Ascii $x.What)") }
    if ($r.Findings.Count -eq 0) { "PASS $where" }
}
foreach ($where in $RequiredCalls.Keys) {
    $have = if ($callWordsOf.ContainsKey($where)) { $callWordsOf[$where] } else { @() }
    foreach ($w in $RequiredCalls[$where]) {
        if ($w -in $have) { "PASS ${where}: E Invoke-Utf8Child $w" }
        else { $fails.Add("FAIL ${where}: E no Invoke-Utf8Child call with $w") }
    }
}

$fails
if ($fails.Count) { "encoding check: $($fails.Count) failure(s)"; exit 1 }
'encoding check: all passed'
exit 0
