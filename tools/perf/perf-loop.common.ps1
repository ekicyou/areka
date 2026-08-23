#Requires -Version 7.0
<#
================================================================================
perf-loop.common.ps1 — perf-loop.ps1 が dot-source する共通部品
  spec: areka-P0-draw-load-parity（要件 1.2 / 2.10 / 2.11・design「計測の道具
        （tools/perf/）→ C5 perf-loop.ps1 → Batch / Job Contract」）

このファイルは単体では何も実行しない。perf-loop.ps1 の先頭で

    . (Join-Path $PSScriptRoot 'perf-loop.common.ps1')

として読み込むことで、次の 4 群の部品が呼び出し側の scope に入る。

  ⒜ 表示と終了      … Write-Info / Write-Problem / Exit-WithResult / Stop-Run
                       終了は必ず Exit-WithResult を通る。ここが標準出力の末尾へ
                       `PERF-LOOP RESULT <sub> code=<n> dir=<path>` の 1 行を出す
                       唯一の場所である（背景実行の終了で会話へ届く形＝要件 1.11）
  ⒝ 子プロセスの起動 … Get-PythonCommand / Get-PwshPath / Invoke-Child
                       出力を 1 行ずつ持ち帰り、終了コードを握り潰さない。
                       **子の出力は端末のコードページに依らず必ず UTF-8 として読む**
                       （python には PYTHONIOENCODING、pwsh には子の中での
                       [Console]::OutputEncoding 切り替えを渡し、読む側は
                       ProcessStartInfo の StandardOutputEncoding を UTF-8 に固定する）。
                       素の `& $exe` 捕捉は親の端末のコードページで復号するため、
                       CP932 の端末から回すと日本語が化けて字面の比較が偽の不一致になる
  ⒞ 目標定義の読み   … Read-TomlSections / Get-GoalConfig / Get-JudgeScriptVersion
                       Get-MeasureToml / Get-MeasureTomlInt / Get-MeasureTomlArray
                       TOML は最小の解釈器（外部モジュールを使わない）。
                       Get-Measure* は $script:GoalConfig.Toml を引く節・鍵の読み口で、
                       呼び手は計測の本体（perf-loop.measure.ps1）＝置き場だけがここ
  ⒟ 出力先の配置     … Get-LoopDir / Resolve-RunDir / New-LoopDir / Test-ResumeArtifact
                       %LOCALAPPDATA%\areka-diag\perf-loop\<goal>\… の唯一の所在

なぜ別ファイルなのか:
  同じ入口が計測サブコマンド 8 本（measure-baseline／rank-run／rank／prepare-ab／
  measure-ab／compare／followup／final）まで持つ。1 ファイル 1,000 行の上限
  （要件・steering）に収めるため、入口（perf-loop.ps1）・共通部品（本ファイル）・
  計測の本体（perf-loop.measure.ps1）の 3 枚に分けてある。

呼び出し側が事前に置いておく script scope の変数（Exit-WithResult ほかが読む）:
  $script:CurrentSub   … 実行中のサブコマンド名（RESULT 行の <sub>）
  $script:CurrentDir   … そのサブコマンドの出力先（無ければ '-'）
  $script:OutRootPath  … 出力先の根（既定は %LOCALAPPDATA%\areka-diag\perf-loop）
  $script:GoalName     … 目標の名前（出力先の第 2 階層）
================================================================================
#>

Set-StrictMode -Version 3.0

# =============================================================================
# 較正値・調整値（perf-loop.ps1 の一覧と対応。変更はここだけ）
# =============================================================================

#: RESULT 行の冠。読む側（perf-loop-iteration スキル・台帳）はこの字面に依存する。
$PERF_LOOP_RESULT_PREFIX = 'PERF-LOOP RESULT'

#: 出力先の既定の根。%LOCALAPPDATA% が無い環境では TEMP へ落ちる。
$PERF_LOOP_OUTROOT_SUBDIR = 'areka-diag\perf-loop'

#: 周（iteration）の出力先に置く小部屋の語彙（design C5 の配置そのもの）。
$PERF_LOOP_ITER_LEAVES = @('rank', 'A1', 'B1', 'A2', 'B2', 'bin-A', 'bin-B', 'followup')

#: 値が無いことを表す字面（台帳・RESULT 行と同じ）。
$PERF_LOOP_EMPTY = '-'

#: 子プロセスの出力を読む文字符号。**端末のコードページに依らず必ず UTF-8**。
#  BOM 無し（[Text.UTF8Encoding]::new($false)）で、書き出し側にも同じものを使う。
$PERF_LOOP_CHILD_ENCODING = [System.Text.UTF8Encoding]::new($false)

# =============================================================================
# ⒜ 表示と終了
# =============================================================================

function Write-Info {
    param([string]$Message)
    [Console]::Out.WriteLine($Message)
}

function Write-Problem {
    param([string]$Message)
    [Console]::Error.WriteLine($Message)
}

function Get-UtcStamp {
    [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.fff', [Globalization.CultureInfo]::InvariantCulture)
}

# 終了する唯一の口。標準出力の末尾へ必ず RESULT 行を出してから exit する。
# どの経路（正常・引数不正・計測失敗・能力不足）もここを通ること。
function Exit-WithResult {
    param(
        [Parameter(Mandatory = $true)][int]$Code,
        [string]$Sub,
        [string]$Dir
    )
    if (-not $Sub) { $Sub = if ($script:CurrentSub) { $script:CurrentSub } else { $PERF_LOOP_EMPTY } }
    if (-not $Dir) { $Dir = if ($script:CurrentDir) { $script:CurrentDir } else { $PERF_LOOP_EMPTY } }
    Write-Info "$PERF_LOOP_RESULT_PREFIX $Sub code=$Code dir=$Dir"
    exit $Code
}

# 失敗して終了する。理由を必ず表示する（黙って失敗しない＝要件 2.11）。
function Stop-Run {
    param(
        [Parameter(Mandatory = $true)][int]$Code,
        [Parameter(Mandatory = $true)][string]$Message,
        [string]$Sub,
        [string]$Dir
    )
    Write-Problem "[perf-loop] 失敗: $Message"
    Exit-WithResult -Code $Code -Sub $Sub -Dir $Dir
}

# =============================================================================
# ⒝ 子プロセスの起動
# =============================================================================

# python の在り処。単一要素の配列を返すと PowerShell が文字列へ展開するため、
# 実行体と前置引数を別の項として持つオブジェクトで返す（invoke-followup-checks.ps1 と同形）。
function Get-PythonCommand {
    foreach ($candidate in @('python', 'python3')) {
        $found = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($found) { return [pscustomobject]@{ Exe = $found.Source; Prefix = @() } }
    }
    $py = Get-Command 'py' -ErrorAction SilentlyContinue
    if ($py) { return [pscustomobject]@{ Exe = $py.Source; Prefix = @('-3') } }
    return $null
}

# 自分と同じ pwsh。子の .ps1 は必ず別プロセスで回す（Set-StrictMode や
# $ErrorActionPreference の設定・exit の意味が混ざらないようにするため）。
function Get-PwshPath {
    $candidate = Join-Path $PSHOME 'pwsh.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    $found = Get-Command 'pwsh' -ErrorAction SilentlyContinue
    if ($found) { return $found.Source }
    return $null
}

# 子プロセスを回して、出力（標準出力・標準エラー）と終了コードを持ち帰る。
# 終了コードは握り潰さない——非ゼロは呼び手が読んで判断する。
#
# **子の出力は必ず UTF-8 として読む**（$PERF_LOOP_CHILD_ENCODING）。
# `& $exe` の素の捕捉は [Console]::OutputEncoding（＝親の端末のコードページ）で復号するため、
# 既定が CP932 の端末から回すと、UTF-8 で書く道具（perf-ledger.py は自分で UTF-8 へ
# 切り替える＝perf-ledger.py:930-932）の出力が化けて、字面の比較が偽の不一致になる。
# ここは端末のコードページに依らない形（ProcessStartInfo の StandardOutputEncoding）で
# 読み、共有物である端末のコードページには触らない。
# 戻り値: Code（int）・Out（string[]）・Err（string[]）・Lines（Out＋Err）・LastLine
function Invoke-Child {
    param(
        [Parameter(Mandatory = $true)][string]$Exe,
        [string[]]$Arguments = @(),
        [hashtable]$Environment,
        [switch]$Quiet
    )
    if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
        return [pscustomobject]@{
            Code     = -1
            Out      = @()
            Err      = @("実行体がありません: $Exe")
            Lines    = @("実行体がありません: $Exe")
            LastLine = "実行体がありません: $Exe"
        }
    }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName               = $Exe
    $startInfo.UseShellExecute        = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError  = $true
    $startInfo.StandardOutputEncoding = $PERF_LOOP_CHILD_ENCODING
    $startInfo.StandardErrorEncoding  = $PERF_LOOP_CHILD_ENCODING
    $startInfo.WorkingDirectory       = (Get-Location).Path
    foreach ($argument in $Arguments) { $startInfo.ArgumentList.Add([string]$argument) }
    if ($Environment) {
        foreach ($key in $Environment.Keys) { $startInfo.Environment[[string]$key] = [string]$Environment[$key] }
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    try {
        $null = $process.Start()
        # 先に非同期で読み始める（バッファが埋まって子が止まるのを避ける）
        $outTask = $process.StandardOutput.ReadToEndAsync()
        $errTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $outText = $outTask.GetAwaiter().GetResult()
        $errText = $errTask.GetAwaiter().GetResult()
        $code = $process.ExitCode
    } catch {
        return [pscustomobject]@{
            Code     = -1
            Out      = @()
            Err      = @("起動できません: $($_.Exception.Message)")
            Lines    = @("起動できません: $($_.Exception.Message)")
            LastLine = "起動できません: $($_.Exception.Message)"
        }
    } finally {
        $process.Dispose()
    }

    $outLines = @()
    if ($outText) { $outLines = @(($outText -replace "`r`n", "`n").TrimEnd("`n") -split "`n") }
    $errLines = @()
    if ($errText) { $errLines = @(($errText -replace "`r`n", "`n").TrimEnd("`n") -split "`n") }
    $lines = @($outLines) + @($errLines)
    if ($null -eq $code) { $code = -1 }
    if (-not $Quiet) { foreach ($line in $lines) { Write-Info $line } }
    # 「最後の 1 行」は報告に使うので、罫線（= や - だけの行）は飛ばして中身のある行を採る
    $last = $PERF_LOOP_EMPTY
    for ($i = $lines.Count - 1; $i -ge 0; $i--) {
        if (-not $lines[$i]) { continue }
        $trimmed = $lines[$i].Trim()
        if ($trimmed -eq '') { continue }
        if ($trimmed -match '^[=\-_*\s]+$') { continue }
        $last = $trimmed
        break
    }
    return [pscustomobject]@{
        Code     = [int]$code
        Out      = $outLines
        Err      = $errLines
        Lines    = $lines
        LastLine = $last
    }
}

# python スクリプトを回す（Get-PythonCommand の戻りを受ける）。
# PYTHONIOENCODING／PYTHONUTF8 を渡して、子が必ず UTF-8 で書くようにする
# （既定では端末のコードページで書く道具があり、CP932 の端末だと化ける）。
function Invoke-PythonChild {
    param(
        [Parameter(Mandatory = $true)][pscustomobject]$Python,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [switch]$Quiet
    )
    $argv = @($Python.Prefix) + $Arguments
    return Invoke-Child -Exe $Python.Exe -Arguments $argv -Quiet:$Quiet `
        -Environment @{ PYTHONIOENCODING = 'utf-8'; PYTHONUTF8 = '1' }
}

# .ps1 を子 pwsh で回す。
# -File ではなく -Command で呼び、子の中で [Console]::OutputEncoding を UTF-8 へ
# 切り替えてからスクリプトを起動する。PowerShell は出力先が渡された handle でも
# [Console]::OutputEncoding（＝その端末のコードページ）で書くため、-File のままだと
# CP932 の端末から回した子の出力が CP932 で来る。親（Invoke-Child）は UTF-8 で読むので、
# ここを揃えないと日本語が化ける。端末そのもののコードページは変えない（子の中だけ）。
function Invoke-PwshChild {
    param(
        [Parameter(Mandatory = $true)][string]$ScriptPath,
        [string[]]$Arguments = @(),
        [switch]$Quiet
    )
    $pwshPath = Get-PwshPath
    if (-not $pwshPath) {
        $msg = 'pwsh が見つかりません'
        return [pscustomobject]@{ Code = -1; Out = @(); Err = @($msg); Lines = @($msg); LastLine = $msg }
    }
    if (-not (Test-Path -LiteralPath $ScriptPath -PathType Leaf)) {
        $msg = "スクリプトがありません: $ScriptPath"
        return [pscustomobject]@{ Code = -1; Out = @(); Err = @($msg); Lines = @($msg); LastLine = $msg }
    }
    $parts = @("& '" + ($ScriptPath -replace "'", "''") + "'")
    foreach ($argument in $Arguments) {
        $text = [string]$argument
        # -SelfTest のような引数名はそのまま（引用すると値として渡ってしまう形もあるため
        # 引数名の形をしているものだけ素で渡し、それ以外は単引用符で括る）
        if ($text -match '^-[A-Za-z][A-Za-z0-9]*$') { $parts += $text }
        else { $parts += ("'" + ($text -replace "'", "''") + "'") }
    }
    $inner = "[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(`$false); " +
             ($parts -join ' ') + "; exit `$LASTEXITCODE"
    $argv = @('-NoProfile', '-NonInteractive', '-Command', $inner)
    return Invoke-Child -Exe $pwshPath -Arguments $argv -Quiet:$Quiet
}

# =============================================================================
# ⒞ 目標定義（TOML）の読み
# =============================================================================

# TOML を「節 → 鍵 → 値（文字列のまま）」の入れ子ハッシュへ。外部モジュールを使わない。
# 解するのは 1 行に収まった素の値と 1 行の配列だけ（目標定義ファイルはこの制約で書く＝
# tasks.md Implementation Notes (5.1)）。引用符の外の # 以降は注釈として落とす。
function Read-TomlSections {
    param([Parameter(Mandatory = $true)][string]$Text)
    $result = @{}
    $section = ''
    foreach ($raw in ($Text -split "`r?`n")) {
        $line = $raw.Trim()
        if ($line -eq '' -or $line.StartsWith('#')) { continue }
        if ($line -match '^\[([^\]]+)\]\s*$') {
            $section = $Matches[1].Trim()
            if (-not $result.ContainsKey($section)) { $result[$section] = @{} }
            continue
        }
        if ($section -eq '') { continue }
        if ($line -notmatch '^([A-Za-z0-9_]+)\s*=\s*(.+)$') { continue }
        $key = $Matches[1]
        $val = ($Matches[2]).Trim()

        # 引用符の外にある # 以降を落とす
        $inStr = $false
        $cut = -1
        for ($i = 0; $i -lt $val.Length; $i++) {
            $ch = $val[$i]
            if ($ch -eq '"') { $inStr = -not $inStr }
            elseif ($ch -eq '#' -and -not $inStr) { $cut = $i; break }
        }
        if ($cut -ge 0) { $val = $val.Substring(0, $cut).Trim() }
        $result[$section][$key] = $val
    }
    return $result
}

# 節・鍵を引いて文字列で返す（引用符は外す）。無ければ $null。
function Get-TomlString {
    param([hashtable]$Toml, [string]$Section, [string]$Key)
    if (-not $Toml.ContainsKey($Section)) { return $null }
    if (-not $Toml[$Section].ContainsKey($Key)) { return $null }
    $val = [string]$Toml[$Section][$Key]
    if ($val.Length -ge 2 -and $val.StartsWith('"') -and $val.EndsWith('"')) {
        return $val.Substring(1, $val.Length - 2)
    }
    return $val
}

# 節・鍵を整数で引く。読めなければ $null。
function Get-TomlInt {
    param([hashtable]$Toml, [string]$Section, [string]$Key)
    $raw = Get-TomlString -Toml $Toml -Section $Section -Key $Key
    if ($null -eq $raw) { return $null }
    $parsed = 0
    $ok = [int]::TryParse($raw, [Globalization.NumberStyles]::Integer,
        [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)
    if ($ok) { return $parsed }
    return $null
}

# 目標定義ファイルの所在。-GoalFile が最優先、無ければ goals/<Goal>.toml。
function Resolve-GoalFile {
    param([string]$Goal, [string]$GoalFile, [Parameter(Mandatory = $true)][string]$ScriptRoot)
    if ($GoalFile) { return $GoalFile }
    return (Join-Path $ScriptRoot (Join-Path 'goals' "$Goal.toml"))
}

# 目標定義を読み、道具が要る値だけを取り出す。
# 返す: Path・Name・SpecDir・LedgerRel・LedgerPath・ResultsDir・JudgeScript・JudgeScriptPath・
#       JudgeVersion・CheckinMinutes・IdleCpuMaxPct・SamplingBackend・Toml
function Get-GoalConfig {
    param(
        [Parameter(Mandatory = $true)][string]$GoalFilePath,
        [Parameter(Mandatory = $true)][string]$RepoRoot
    )
    $text = Get-Content -LiteralPath $GoalFilePath -Raw -Encoding utf8
    $toml = Read-TomlSections -Text $text

    $name        = Get-TomlString -Toml $toml -Section 'goal' -Key 'name'
    $specDir     = Get-TomlString -Toml $toml -Section 'goal' -Key 'spec_dir'
    $ledgerRel   = Get-TomlString -Toml $toml -Section 'goal' -Key 'ledger'
    $resultsDir  = Get-TomlString -Toml $toml -Section 'goal' -Key 'results_dir'
    $judgeScript = Get-TomlString -Toml $toml -Section 'goal' -Key 'judge_script'
    $judgeVer    = Get-TomlString -Toml $toml -Section 'goal' -Key 'judge_version'
    $checkin     = Get-TomlInt    -Toml $toml -Section 'goal_runtime' -Key 'checkin_minutes'
    $idleMax     = Get-TomlString -Toml $toml -Section 'target' -Key 'idle_cpu_release_max_pct'
    $backend     = Get-TomlString -Toml $toml -Section 'sampling' -Key 'backend'

    $ledgerPath = $null
    if ($specDir -and $ledgerRel) {
        $ledgerPath = [System.IO.Path]::GetFullPath((Join-Path (Join-Path $RepoRoot $specDir) $ledgerRel))
    }
    $judgePath = $null
    if ($judgeScript) {
        $judgePath = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $judgeScript))
    }

    return [pscustomobject]@{
        Path            = $GoalFilePath
        Name            = $name
        SpecDir         = $specDir
        LedgerRel       = $ledgerRel
        LedgerPath      = $ledgerPath
        ResultsDir      = $resultsDir
        JudgeScript     = $judgeScript
        JudgeScriptPath = $judgePath
        JudgeVersion    = $judgeVer
        CheckinMinutes  = $checkin
        IdleCpuMaxPct   = $idleMax
        SamplingBackend = $backend
        Toml            = $toml
    }
}

# =============================================================================
# 目標定義の読み（節・鍵が無ければ既定値。目標定義が唯一の所在＝要件 1.1）
# =============================================================================
# 読む対象は $script:GoalConfig.Toml（入口の perf-loop.ps1 が Get-GoalConfig で入れる）。
# 計測サブコマンド（perf-loop.measure.ps1）から呼ばれるが、置き場はここ（行数上限）。
function Get-MeasureToml {
    param([Parameter(Mandatory = $true)][string]$Section, [Parameter(Mandatory = $true)][string]$Key, [string]$Default)
    if (-not $script:GoalConfig) { return $Default }
    $value = Get-TomlString -Toml $script:GoalConfig.Toml -Section $Section -Key $Key
    if ($null -eq $value -or [string]::IsNullOrWhiteSpace($value)) { return $Default }
    return $value
}

function Get-MeasureTomlInt {
    param([Parameter(Mandatory = $true)][string]$Section, [Parameter(Mandatory = $true)][string]$Key, [int]$Default)
    if (-not $script:GoalConfig) { return $Default }
    $value = Get-TomlInt -Toml $script:GoalConfig.Toml -Section $Section -Key $Key
    if ($null -eq $value) { return $Default }
    return $value
}

# 1 行の配列（["A", "B", "A", "B"]）を文字列の配列へ。読めなければ既定値を返す
# （目標定義ファイルは配列を 1 行に保つ決まり＝tasks.md Implementation Notes (5.1)）。
function Get-MeasureTomlArray {
    param([Parameter(Mandatory = $true)][string]$Section, [Parameter(Mandatory = $true)][string]$Key, [string[]]$Default)
    $raw = Get-MeasureToml -Section $Section -Key $Key
    if (-not $raw) { return $Default }
    $text = $raw.Trim()
    if (-not ($text.StartsWith('[') -and $text.EndsWith(']'))) { return $Default }
    $items = @()
    foreach ($part in ($text.Substring(1, $text.Length - 2) -split ',')) {
        $piece = $part.Trim().Trim('"').Trim()
        if ($piece) { $items += $piece }
    }
    if ($items.Count -eq 0) { return $Default }
    return $items
}

# judge-perf.py の SCRIPT_VERSION の先頭語（例 "0.4.0"）を、スクリプトを起動せずに読む。
# 判定スクリプトは複数行の括弧つき文字列で版を持つ（judge-perf.py:150）ため、
# 最初の引用符の中の最初の空白までを版と見る。読めなければ $null（呼び手が計測失敗にする）。
function Get-JudgeScriptVersion {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    $text = Get-Content -LiteralPath $Path -Raw -Encoding utf8
    $match = [regex]::Match($text, '(?m)^SCRIPT_VERSION\s*=\s*\(?\s*"([^"\s]+)')
    if ($match.Success) { return $match.Groups[1].Value }
    return $null
}

# =============================================================================
# ⒟ 出力先の配置（design C5 の配置の唯一の所在）
# =============================================================================

# 出力先の根。-OutRoot が最優先、無ければ %LOCALAPPDATA%\areka-diag\perf-loop。
function Resolve-OutRoot {
    param([string]$OutRoot)
    if ($OutRoot) { return [System.IO.Path]::GetFullPath($OutRoot).TrimEnd('\') }
    $base = $env:LOCALAPPDATA
    if (-not $base) { $base = $env:TEMP }
    if (-not $base) { return $null }
    return [System.IO.Path]::GetFullPath((Join-Path $base $PERF_LOOP_OUTROOT_SUBDIR)).TrimEnd('\')
}

# 出力先の 1 か所を組む（作りはしない）。
#   -Kind goal      … <root>\<goal>                      （preflight.txt の置き場）
#   -Kind baseline  … <root>\<goal>\baseline-<date>       （1 周目の 25 分 × 2 本）
#   -Kind iter      … <root>\<goal>\iter-<n>              （周の作業場）
#   -Kind final     … <root>\<goal>\final-<date>          （最終判定）
# -Leaf を付けると小部屋（rank／A1／B1／A2／B2／bin-A／bin-B／followup）まで下る。
# 小部屋は -Kind iter のときだけ語彙を検査する（打ち間違いを黙って通さない）。
function Get-LoopDir {
    param(
        [Parameter(Mandatory = $true)][ValidateSet('goal', 'baseline', 'iter', 'final')][string]$Kind,
        [int]$Iter = 0,
        [string]$Date,
        [string]$Leaf,
        [string]$OutRoot,
        [string]$Goal
    )
    $root = if ($OutRoot) { $OutRoot } else { $script:OutRootPath }
    $goalName = if ($Goal) { $Goal } else { $script:GoalName }
    if (-not $root) { throw '出力先の根が決まっていません（Resolve-OutRoot を先に呼ぶこと）。' }
    if (-not $goalName) { throw '目標の名前が決まっていません。' }

    $path = Join-Path $root $goalName
    switch ($Kind) {
        'goal' { }
        'iter' {
            if ($Iter -lt 1) { throw "-Iter は 1 以上が要ります（受け取った値: $Iter）。" }
            $path = Join-Path $path "iter-$Iter"
        }
        'baseline' {
            $stamp = if ($Date) { $Date } else { Get-LoopDateStamp }
            $path = Join-Path $path "baseline-$stamp"
        }
        'final' {
            $stamp = if ($Date) { $Date } else { Get-LoopDateStamp }
            $path = Join-Path $path "final-$stamp"
        }
    }
    if ($Leaf) {
        if ($Kind -eq 'iter' -and $PERF_LOOP_ITER_LEAVES -notcontains $Leaf) {
            throw "周の小部屋の名前が語彙にありません: '$Leaf'（使えるのは $($PERF_LOOP_ITER_LEAVES -join '・')）"
        }
        $path = Join-Path $path $Leaf
    }
    return $path
}

# 出力先の名前に使う日付（既定は実行日のローカル時刻・yyyyMMdd）。
# -Date で上書きできるのは、同じ日付の出力先を後から作り直す／試験で決定論にするため。
function Get-LoopDateStamp {
    param([string]$Date)
    if ($Date) { return $Date }
    if ($script:DateStamp) { return $script:DateStamp }
    return (Get-Date).ToString('yyyyMMdd', [Globalization.CultureInfo]::InvariantCulture)
}

# 走行の出力先を決める唯一の口。-RunDir が与えられていればそれを使い（既定の配置の外へ
# 出したいとき・既にある走行を読み直すとき）、無ければ Get-LoopDir の配置に従う。
# 計測サブコマンドは自分で Join-Path せず、必ずこれを通すこと。
function Resolve-RunDir {
    param(
        [string]$RunDir,
        [Parameter(Mandatory = $true)][ValidateSet('goal', 'baseline', 'iter', 'final')][string]$Kind,
        [int]$Iter = 0,
        [string]$Date,
        [string]$Leaf
    )
    if ($RunDir) {
        $path = $RunDir
        if (-not [System.IO.Path]::IsPathFullyQualified($path)) {
            $path = Join-Path (Get-Location).Path $path
        }
        return [System.IO.Path]::GetFullPath($path).TrimEnd('\')
    }
    return (Get-LoopDir -Kind $Kind -Iter $Iter -Date $Date -Leaf $Leaf)
}

# 出力先を作る（既にあれば何もしない）。
function New-LoopDir {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Force -Path $Path | Out-Null
    }
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
}

# 冪等（-Resume）の判定。同じ出力先に成果物が既にあり、-Resume が付いていれば
# 「作り直さず再利用する」と答える。-Resume が無ければ常に $false（採り直す）。
# 何を成果物と見るかは呼び手が渡す（各サブコマンドが自分の完了印を渡す）。
function Test-ResumeArtifact {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [switch]$Resume
    )
    if (-not $Resume) { return $false }
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    Write-Info "[perf-loop] -Resume: 既にある成果物を再利用します: $Path"
    return $true
}

# =============================================================================
# 環境の見立て（preflight が読む）
# =============================================================================

# 管理者権限で走っているか（段③＝関数別の帰属に要る）。
function Test-Elevated {
    try {
        $identity  = [Security.Principal.WindowsIdentity]::GetCurrent()
        $principal = New-Object Security.Principal.WindowsPrincipal($identity)
        return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    } catch {
        return $false
    }
}

# xperf.exe の所在（PATH → Windows Kits の既定位置）。無ければ $null。
function Find-XperfPath {
    param([string]$DefaultPath)
    $found = Get-Command 'xperf.exe' -ErrorAction SilentlyContinue
    if ($found) { return $found.Source }
    if ($DefaultPath -and (Test-Path -LiteralPath $DefaultPath -PathType Leaf)) { return $DefaultPath }
    return $null
}

# 真偽を字面へ（capabilities 行・preflight.txt の値）。
function Format-LoopBool {
    param([bool]$Value)
    if ($Value) { return 'true' }
    return 'false'
}

# 値が無いときは '-'（台帳・RESULT 行と同じ字面）。
function Format-LoopValue {
    param($Value)
    if ($null -eq $Value) { return $PERF_LOOP_EMPTY }
    $text = [string]$Value
    if ($text.Trim() -eq '') { return $PERF_LOOP_EMPTY }
    return $text
}

# 32bit SHIORI ヘルパを実行体の隣へそろえる（合否判定の走行は shiori_helper_present=true が関門）。
# measure-baseline／rank-run／final は target\<release|debug> の areka.exe を直に使うが、ヘルパは
# --target i686 で別ディレクトリに出るので、在るのに隣に無い＝実 SHIORI 無しの走行になって exit 4 で
# 25 分を失う（2026-08-23 baseline-20260823\release で実際に起きた）。無ければ何もしない（既存の関門が止める）。
function Sync-MeasureShioriHelper {
    param([Parameter(Mandatory = $true)][string]$BinDir)
    $dest = Join-Path $BinDir $MEASURE_HELPER_EXE
    if (Test-Path -LiteralPath $dest -PathType Leaf) { return }
    $src = Join-Path $repoRoot (Join-Path 'target' (Join-Path $MEASURE_HELPER_TARGET (Join-Path 'release' $MEASURE_HELPER_EXE)))
    if (-not (Test-Path -LiteralPath $src -PathType Leaf)) {
        Write-Problem "[perf-loop] 警告: 32bit SHIORI ヘルパが $dest にも $src にもありません。cargo build --release -p $MEASURE_HELPER_PACKAGE --target $MEASURE_HELPER_TARGET を先に回してください（合否判定の走行は exit 4 で止まります）。"
        return
    }
    Copy-Item -LiteralPath $src -Destination $dest -Force
    Write-Info "[perf-loop] 32bit SHIORI ヘルパを実行体の隣へ複製しました: $src -> $dest"
}
