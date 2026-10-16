<#
.SYNOPSIS
  Encoding check for the release tools: calibration, then static rules over the tool scripts
  and the workflow run bodies. Exit code 0 = all passed, 1 = at least one failure.

.DESCRIPTION
  This file is ASCII only (it is one of its own targets). Japanese text in the samples is built
  with `u{...} escapes, so the source text of every token stays ASCII.

  1. Calibration: every rule is run on built-in samples that must pass and samples that must fail.
     Samples are plain strings (single quotes or concatenation), so they are never code here.
  2. Static rules over tools/*.ps1 and tools/perf/*.ps1 (direct children of each folder only) and
     .claude/skills/**/*.ps1 (scripts kept inside a skill; rules
     A-D, the same for both) and the run: bodies of .github/workflows/*.yml
     (a step's `run: |` block or one-line scalar; a mapping-valued run: such as defaults.run is
     not a body). Each body is parsed as pwsh; a parse error is a failure.
       A  a non-comment token whose source text has a char outside tab, CR, LF, 0x20-0x7E
          (except inside the right-hand side of an assignment to $LOG_MARKER_*)
       B  a $LOG_MARKER_* variable inside an expandable string or here-string
       C  an assignment to [Console]::OutputEncoding / InputEncoding ([System.Console] too) or
          $OutputEncoding, or a chcp / chcp.com command (only exception: inside the function
          Set-OwnConsoleCp932 in tools/encoding-check.ps1); the string after -Command / -c of a
          child pwsh / powershell is parsed and checked the same way, and so is every string that
          mentions Encoding or chcp (only exceptions, in tools/encoding-check.ps1: the strings of
          $calib and $chcpWords in Get-ConsoleRewrites); [Console]::SetOut / SetError are
          not rewrites
       D  a cargo / cargo.exe command (bare or with &) with the element metadata or nar-sample-path,
          also through a variable argument ($x, @x): the right-hand sides of = / += to that name
          (case-insensitive, any scope) are followed through string constants, arrays, +,
          parentheses and other variables (each once); pipelines, command results, members and
          method values are not followed
       E  tools/package.ps1 (metadata and nar-sample-path), tools/crates-io.ps1 (metadata) and the
          run bodies of .github/workflows/release.yml (metadata) each have an Invoke-Utf8Child
          call with that element
  3. cp932 child: this script starts itself with -Cp932Child as a windowless child that has its
     own console (Invoke-Utf8Child -OwnConsole). Only that child sets its own console to code
     page 932 (Set-OwnConsoleCp932), then checks with the fixed sample {"description":"<U+7248>",
     "version":"0.0.1"} written as raw UTF-8 bytes by a child pwsh:
       calibration  the plain call (@(& pwsh ... 2>&1) | ConvertFrom-Json) must break the sample
                    (else this console cannot detect a regression)
       production   Invoke-Utf8Child must read the sample exactly
       real         Invoke-Utf8Child cargo metadata must give exactly one areka package with a
                    version, printed as version=<v>
     then, in its own scope (perf-loop.common.ps1 sets StrictMode 3.0), with the real
     Invoke-PwshChild of tools/perf and the sample <U+7248><U+2460><U+1F600> printed by this script
     run with -PerfSampleChild (out1= Write-Output, out2= Write-Host, err1= stderr):
       perf calibration  Invoke-Child with a plain -File (no prefix) must break the sample
       perf real         Invoke-PwshChild must deliver the 3 lines exactly
       perf console      the child's console must still be 932 / 932 (read from the OS)
     The parent copies the child's lines, fails on a non-zero exit, reads the version itself and
     fails unless the child's version=<v> line is the same, and fails if its own two console code
     pages (read from the OS, not the .NET cache) changed. The parent never sets its own console.
  All failures are collected and printed, then the exit code is decided once.
  Writes nothing, creates no temporary files, prints ASCII only, never prints the environment.

  Run: pwsh -NoProfile -File tools/encoding-check.ps1
  (-Cp932Child and -PerfSampleChild are internal; run by hand from a terminal they change nothing,
  print one ASCII line and exit with 1.)
#>
using namespace System.Management.Automation.Language
#Requires -Version 7
param(
    [switch]$Cp932Child,      # internal: only the windowless child started by this script uses it
    [switch]$PerfSampleChild  # internal: the sample writer started by that child
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# Invoke-Utf8Child: read a child's output as UTF-8 regardless of the console code page
. (Join-Path $PSScriptRoot 'utf8-child.ps1')

$NonAscii = '[^\t\r\n\x20-\x7E]'
$SelfWhere = 'tools/encoding-check.ps1'
$CallWordSet = @('metadata', 'nar-sample-path')
# Rule E: where an Invoke-Utf8Child call must carry each element
$RequiredCalls = [ordered]@{
    'tools/package.ps1'             = @('metadata', 'nar-sample-path')
    'tools/crates-io.ps1'           = @('metadata')
    '.github/workflows/release.yml' = @('metadata')
}

# Printable form: non-ASCII and control chars become \uXXXX, text longer than $Max is cut
# ($Max 0 = never cut).
function Show-Ascii([string]$s, [int]$Max = 80) {
    $t = -join ($s.ToCharArray() | ForEach-Object {
            if ([int]$_ -ge 0x20 -and [int]$_ -le 0x7E) { $_ } else { '\u{0:X4}' -f [int]$_ }
        })
    if ($Max -gt 0 -and $t.Length -gt $Max) { $t.Substring(0, $Max - 3) + '...' } else { $t }
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

# Rule D's value tracing: the string constants an expression is built from. Follows string
# constants, arrays (@(...) and , lists), +, parentheses and variables (through $Assigned: lower-case
# name -> right-hand sides of = and +=, each name once via $Seen). Pipelines, command results,
# members and method values are not followed.
function Get-TracedWords([Ast]$Node, [hashtable]$Assigned, [System.Collections.Generic.HashSet[string]]$Seen) {
    if ($Node -is [PipelineAst]) {
        $el = $Node.PipelineElements
        if ($el.Count -eq 1 -and $el[0] -is [CommandExpressionAst]) { Get-TracedWords $el[0].Expression $Assigned $Seen }
    }
    elseif ($Node -is [CommandExpressionAst]) { Get-TracedWords $Node.Expression $Assigned $Seen }
    elseif ($Node -is [StringConstantExpressionAst]) { $Node.Value }
    elseif ($Node -is [ArrayLiteralAst]) { foreach ($e in $Node.Elements) { Get-TracedWords $e $Assigned $Seen } }
    elseif ($Node -is [ArrayExpressionAst]) { foreach ($s in $Node.SubExpression.Statements) { Get-TracedWords $s $Assigned $Seen } }
    elseif ($Node -is [ParenExpressionAst]) { Get-TracedWords $Node.Pipeline $Assigned $Seen }
    elseif ($Node -is [BinaryExpressionAst] -and $Node.Operator -eq [TokenKind]::Plus) {
        Get-TracedWords $Node.Left $Assigned $Seen
        Get-TracedWords $Node.Right $Assigned $Seen
    }
    elseif ($Node -is [VariableExpressionAst]) {
        $name = $Node.VariablePath.UserPath.Split(':')[-1].ToLowerInvariant()
        if ($Assigned.ContainsKey($name) -and $Seen.Add($name)) {
            foreach ($r in $Assigned[$name]) { Get-TracedWords $r $Assigned $Seen }
        }
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
    $chcpWords = @('chcp', 'chcp.com')
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
        if ($leaf -in $chcpWords -and -not ($Where -eq $SelfWhere -and (Test-InFunction $c 'Set-OwnConsoleCp932'))) {
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
    # Every string (not a bare word) that mentions Encoding or chcp is parsed and checked the same
    # way (a prefix built in a variable and passed to a child). Only exceptions, in this file: the
    # strings of the calibration samples ($calib = ...) and $chcpWords above.
    foreach ($s in $Ast.FindAll({ param($n)
                ($n -is [StringConstantExpressionAst] -and $n.StringConstantType -ne [StringConstantType]::BareWord) -or
                $n -is [ExpandableStringExpressionAst] }, $true)) {
        if ($s.Value -notmatch 'Encoding' -and $s.Value -notmatch $chcpWords[0]) { continue }
        if ($Where -eq $SelfWhere -and (Test-SelfExemptString $s)) { continue }
        $tk = $null; $er = $null
        $inner = [Parser]::ParseInput($s.Value, [ref]$tk, [ref]$er)
        if (@(Get-ConsoleRewrites $inner '').Count) {
            [pscustomobject]@{ Line = $s.Extent.StartLineNumber; Text = $s.Extent.Text }
        }
    }
}

# Rule C's own exceptions in tools/encoding-check.ps1, by name: a string inside the right-hand side
# of $calib = ..., or of $chcpWords = ... inside the function Get-ConsoleRewrites.
function Test-SelfExemptString([Ast]$Str) {
    for ($p = $Str.Parent; $null -ne $p; $p = $p.Parent) {
        if (-not ($p -is [AssignmentStatementAst] -and $p.Left -is [VariableExpressionAst])) { continue }
        $name = $p.Left.VariablePath.UserPath
        return ($name -eq 'calib' -or ($name -eq 'chcpWords' -and (Test-InFunction $p 'Get-ConsoleRewrites')))
    }
    $false
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
    foreach ($h in @(Get-ConsoleRewrites $ast $Where) | Sort-Object Line -Unique) { & $add 'C' $h.Line $h.Text }

    # D, and the Invoke-Utf8Child elements for E
    $callWords = [System.Collections.Generic.List[string]]::new()
    $assigned = @{}   # rule D: lower-case variable name -> right-hand sides of = and +=
    foreach ($n in $ast.FindAll({ param($n) $n -is [AssignmentStatementAst] -and
                    $n.Operator -in @([TokenKind]::Equals, [TokenKind]::PlusEquals) }, $true)) {
        $l = $n.Left
        if ($l -is [ConvertExpressionAst]) { $l = $l.Child }
        if ($l -isnot [VariableExpressionAst]) { continue }
        $name = $l.VariablePath.UserPath.Split(':')[-1].ToLowerInvariant()
        if (-not $assigned.ContainsKey($name)) { $assigned[$name] = [System.Collections.Generic.List[Ast]]::new() }
        $assigned[$name].Add($n.Right)
    }
    foreach ($c in $ast.FindAll({ param($n) $n -is [CommandAst] }, $true)) {
        $leaf = Get-CommandLeaf $c
        if ($null -eq $leaf) { continue }
        $words = @(Get-ArgWords $c | Where-Object { $_ -in $CallWordSet })
        if ($leaf -in @('cargo', 'cargo.exe') -and -not $words.Count) {
            $seen = [System.Collections.Generic.HashSet[string]]::new()
            $words = @(foreach ($e in @($c.CommandElements | Select-Object -Skip 1)) {
                    Get-TracedWords $e $assigned $seen | Where-Object { $_ -in $CallWordSet }
                })
        }
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

# ---- 3. cp932 child: shared parts ----
$root = Split-Path -Parent $PSScriptRoot
$pwsh = [Environment]::ProcessPath
$VersionPattern = '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$'

# The console code pages read from the OS now: @(<output>, <input>). [Console]::OutputEncoding
# is cached by .NET and misses a rewrite by a child. The type is built in memory, once.
function Get-ConsoleCodePages {
    if (-not ('ArekaEncodingCheck.ConsoleCp' -as [type])) {
        Add-Type -Namespace ArekaEncodingCheck -Name ConsoleCp -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("kernel32.dll")] public static extern uint GetConsoleOutputCP();
[System.Runtime.InteropServices.DllImport("kernel32.dll")] public static extern uint GetConsoleCP();
'@
    }
    @([ArekaEncodingCheck.ConsoleCp]::GetConsoleOutputCP(), [ArekaEncodingCheck.ConsoleCp]::GetConsoleCP())
}

# The only console rewrite rule C allows: called only in the windowless child (-Cp932Child), on
# the console that child owns alone.
function Set-OwnConsoleCp932 {
    $cp932 = [System.Text.Encoding]::GetEncoding(932)
    [Console]::OutputEncoding = $cp932
    [Console]::InputEncoding = $cp932
}

# Version of the areka package read with Invoke-Utf8Child cargo metadata. Throws (ASCII message).
function Read-ArekaVersion {
    $r = Invoke-Utf8Child cargo @('metadata', '--no-deps', '--locked', '--format-version', '1',
        '--manifest-path', (Join-Path $root 'Cargo.toml'))
    if ($r.Code) { throw "cargo metadata exited with code $($r.Code): $(Show-Ascii ([string]($r.ErrLines | Select-Object -Last 1)))" }
    try { $meta = $r.Out | ConvertFrom-Json } catch { throw 'cargo metadata output is not JSON' }
    $pkgs = @($meta.packages | Where-Object { $_.name -ceq 'areka' })
    if ($pkgs.Count -ne 1) { throw "expected exactly one areka package, got $($pkgs.Count)" }
    $v = [string]$pkgs[0].version
    if ($v -cnotmatch $VersionPattern) { throw "areka version is not a version: $(Show-Ascii $v)" }
    $v
}

if ($PerfSampleChild) {
    # Run by hand from a terminal: print no sample.
    if (-not ([Console]::IsOutputRedirected -and [Console]::IsErrorRedirected)) {
        '-PerfSampleChild is internal; run tools/encoding-check.ps1 without it'
        exit 1
    }
    $s = "`u{7248}`u{2460}`u{1F600}"
    Write-Output "out1=$s"
    Write-Host "out2=$s"
    [Console]::Error.WriteLine("err1=$s")
    exit 0
}

if ($Cp932Child) {
    # Run by hand from a terminal: do not touch the developer's console.
    if (-not ([Console]::IsOutputRedirected -and [Console]::IsErrorRedirected)) {
        '-Cp932Child is internal; run tools/encoding-check.ps1 without it'
        exit 1
    }
    $bad = [System.Collections.Generic.List[string]]::new()
    $same = { param([string]$a, [string]$b) [string]::Equals($a, $b, [StringComparison]::Ordinal) }

    try {
        Set-OwnConsoleCp932
        $cp = [Console]::OutputEncoding.CodePage
        if ($cp -eq 932) { 'PASS cp932 child: own console code page 932' }
        else { $bad.Add("FAIL cp932 child: own console code page is $cp, not 932") }
    }
    catch { $bad.Add("FAIL cp932 child: cannot set own console to 932: $(Show-Ascii $_.Exception.Message)") }

    # The sample is written as raw UTF-8 bytes by a child pwsh. U+7248 ends with 0x88, a lead byte
    # in 932, so a 932 decoder swallows the following double quote.
    $desc = "`u{7248}"
    $sample = '{"description":"' + $desc + '","version":"0.0.1"}'
    $b64 = [Convert]::ToBase64String([System.Text.UTF8Encoding]::new($false).GetBytes($sample))
    $emit = "[Console]::OpenStandardOutput().Write([Convert]::FromBase64String('$b64'))"

    # calibration: the plain call must break the sample on this console
    $raw = @(& $pwsh -NoProfile -NonInteractive -Command $emit 2>&1) -join "`n"
    if ($LASTEXITCODE) { $bad.Add("FAIL cp932 child: calibration: sample writer exited with code $LASTEXITCODE") }
    else {
        $readOk = $false
        try { $readOk = & $same ($raw | ConvertFrom-Json).description $desc } catch { }
        if ($readOk) { $bad.Add('FAIL cp932 child: calibration: the plain call read the sample intact; cannot detect a regression on this console') }
        else { "PASS cp932 child: calibration (the plain call breaks the sample: $(Show-Ascii $raw))" }
    }

    # production: Invoke-Utf8Child must read the sample exactly
    try {
        $r = Invoke-Utf8Child $pwsh @('-NoProfile', '-NonInteractive', '-Command', $emit)
        if ($r.Code) { throw "sample writer exited with code $($r.Code)" }
        $exact = $false
        try { $j = $r.Out | ConvertFrom-Json; $exact = (& $same $j.description $desc) -and (& $same $j.version '0.0.1') } catch { }
        if ($exact) { 'PASS cp932 child: production (Invoke-Utf8Child reads the sample exactly)' }
        else { $bad.Add("FAIL cp932 child: production: Invoke-Utf8Child read the sample as $(Show-Ascii $r.Out 0)") }
    }
    catch { $bad.Add("FAIL cp932 child: production: $(Show-Ascii $_.Exception.Message)") }

    # real: the version of the current workspace
    try { "version=$(Read-ArekaVersion)" }
    catch { $bad.Add("FAIL cp932 child: real: $(Show-Ascii $_.Exception.Message)") }

    # perf: the real Invoke-PwshChild of tools/perf on this 932 console. The grandchild is this
    # script with -PerfSampleChild. perf-loop.common.ps1 sets StrictMode 3.0, so it is loaded only
    # inside this scope, after the 3 checks above.
    & {
        . (Join-Path $root 'tools/perf/perf-loop.common.ps1')
        $s = "`u{7248}`u{2460}`u{1F600}"
        $want = @("out1=$s", "out2=$s", "err1=$s") -join "`n"

        # perf calibration: the plain -File call (no prefix) must break the sample
        $r = Invoke-Child -Exe $pwsh -Arguments @('-NoProfile', '-NonInteractive', '-File', $PSCommandPath, '-PerfSampleChild') -Quiet
        $got = (@($r.Out) + @($r.Err)) -join "`n"
        if ($r.Code) { $bad.Add("FAIL cp932 child: perf calibration: sample writer exited with code $($r.Code)") }
        elseif (& $same $got $want) { $bad.Add('FAIL cp932 child: perf calibration: the plain call read the sample intact; cannot detect a regression on this console') }
        else { "PASS cp932 child: perf calibration (the plain call breaks the sample: $(Show-Ascii $got 0))" }

        # perf real: Invoke-PwshChild must deliver the 3 lines exactly
        $r = Invoke-PwshChild -ScriptPath $PSCommandPath -Arguments @('-PerfSampleChild') -Quiet
        $got = (@($r.Out) + @($r.Err)) -join "`n"
        $exact = $r.Code -eq 0 -and @($r.Out).Count -eq 2 -and @($r.Err).Count -eq 1 -and (& $same $got $want)
        if ($exact) { 'PASS cp932 child: perf real (Invoke-PwshChild delivers the 3 sample lines exactly)' }
        else { $bad.Add("FAIL cp932 child: perf real: code $($r.Code), Invoke-PwshChild read $(Show-Ascii $got 0)") }

        # perf console: the child must leave this console at 932 (a rewrite prefix would change it)
        $cps = Get-ConsoleCodePages
        if ($cps[0] -eq 932 -and $cps[1] -eq 932) { 'PASS cp932 child: perf console still 932 after Invoke-PwshChild' }
        else { $bad.Add("FAIL cp932 child: perf console changed by Invoke-PwshChild: output $($cps[0]), input $($cps[1])") }
    }

    $bad
    if ($bad.Count) { exit 1 }
    exit 0
}

$fails = [System.Collections.Generic.List[string]]::new()
$cpBefore = Get-ConsoleCodePages

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
    @{ Name = 'rewrite prefix built in a variable and passed'; Text = '$inner = ''[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); '' + $x; $psi.ArgumentList.Add($inner)'; Expect = 'C' }
    @{ Name = 'chcp inside an argument array'; Text = '$a = @(''-Command'', ''chcp 65001'')'; Expect = 'C' }
    @{ Name = 'bare cargo with metadata passed as an array variable'; Text = '$a = @(''metadata'', ''--no-deps''); cargo @a'; Expect = 'D' }
    @{ Name = 'bare cargo with nar-sample-path through two variables'; Text = '$NAR_ARGS = @(''run'', ''--bin'', ''nar-sample-path''); $cargoArgs = @($nar_args) + @(''--'', ''x''); & cargo @cargoArgs'; Expect = 'D' }
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
    @{ Name = 'UTF-8 writer prefix for a child'; Text = '$pre = ''$u = [System.Text.UTF8Encoding]::new($false); $w = [System.IO.StreamWriter]::new([Console]::OpenStandardOutput(), $u); $w.AutoFlush = $true; [Console]::SetOut($w); $e = [System.IO.StreamWriter]::new([Console]::OpenStandardError(), $u); $e.AutoFlush = $true; [Console]::SetError($e); '''; Expect = $null }
    @{ Name = 'bare cargo with a pipeline value'; Text = '$PUBLISH = @(''metadata''); $pArgs = $PUBLISH | ForEach-Object { ''-p'', $_ }; cargo publish @pArgs'; Expect = $null }
    @{ Name = 'bare cargo with only other words in variables'; Text = '$x = @(''build'', ''--locked''); $x += $x; cargo @x $y'; Expect = $null }
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
$targets = @(Get-ChildItem -LiteralPath (Join-Path $root 'tools') -Filter '*.ps1' -File) +
@(Get-ChildItem -LiteralPath (Join-Path $root 'tools/perf') -Filter '*.ps1' -File) +
@(Get-ChildItem -LiteralPath (Join-Path $root '.claude/skills') -Filter '*.ps1' -File -Recurse) +
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

# ---- 3. cp932 child ----
$child = $null
try { $child = Invoke-Utf8Child $pwsh @('-NoProfile', '-NonInteractive', '-File', $PSCommandPath, '-Cp932Child') -OwnConsole }
catch { $fails.Add("FAIL cp932 child: cannot start: $(Show-Ascii $_.Exception.Message)") }
if ($null -ne $child) {
    $childLines = @($child.Out -split '\r?\n' | Where-Object { $_ -ne '' } | ForEach-Object { Show-Ascii $_ 0 })
    $childFails = @($childLines | Where-Object { $_.StartsWith('FAIL ') })
    $childLines | Where-Object { -not $_.StartsWith('FAIL ') }
    foreach ($e in $child.ErrLines) { "cp932 child stderr: $(Show-Ascii $e 0)" }
    foreach ($f in $childFails) { $fails.Add($f) }
    if ($child.Code -and $childFails.Count -eq 0) { $fails.Add("FAIL cp932 child: exit code $($child.Code)") }
    $childVersion = @($childLines | Where-Object { $_.StartsWith('version=') })
    if ($childVersion.Count -eq 1) {
        try {
            $mine = 'version=' + (Read-ArekaVersion)
            if ([string]::Equals($childVersion[0], $mine, [StringComparison]::Ordinal)) { "PASS cp932 child: $mine is the version this process read" }
            else { $fails.Add("FAIL cp932 child: child printed $($childVersion[0]) but this process read $mine") }
        }
        catch { $fails.Add("FAIL cp932 child: this process cannot read the version: $(Show-Ascii $_.Exception.Message)") }
    }
    elseif ($childFails.Count -eq 0) { $fails.Add("FAIL cp932 child: expected one version= line, got $($childVersion.Count)") }
}
$cpAfter = Get-ConsoleCodePages
if (($cpBefore -join ',') -ceq ($cpAfter -join ',')) { "PASS own console code pages unchanged (output $($cpAfter[0]), input $($cpAfter[1]))" }
else { $fails.Add("FAIL own console code pages changed: output $($cpBefore[0]) -> $($cpAfter[0]), input $($cpBefore[1]) -> $($cpAfter[1])") }

$fails
if ($fails.Count) { "encoding check: $($fails.Count) failure(s)"; exit 1 }
'encoding check: all passed'
exit 0
