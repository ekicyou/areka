#Requires -Version 7.0
<#
================================================================================
perf-loop.common.ps1 — perf-loop.ps1 が dot-source する共通部品
  spec: areka-P0-draw-load-parity（要件 1.2 / 2.10 / 2.11・design「計測の道具
        （tools/perf/）→ C5 perf-loop.ps1 → Batch / Job Contract」）

このファイルは単体では何も実行しない（関数と定数を置くだけ）。perf-loop.ps1 の先頭で

    . (Join-Path $PSScriptRoot 'perf-loop.common.ps1')

として読み込むことで、次の部品が呼び出し側の scope に入る。読み込むのは
perf-tool-base.ps1（表示・時刻・子の起こし方・目標定義の読み・検体の在り処）だけで、
単独で読み込んでも動く（tools/encoding-check.ps1 が Invoke-Child／Invoke-PwshChild を使う形）。

  終了と定数        … Exit-WithResult / Stop-Run / Exit-OnUnexpectedError（trap から）・終了コード（$EXIT_*）・$BUILD_KINDS・$repoRoot
                       終了は必ず Exit-WithResult を通る。ここが標準出力の末尾へ
                       `PERF-LOOP RESULT <sub> code=<n> dir=<path>` の 1 行を出す
                       唯一の場所である（背景実行の終了で会話へ届く形＝要件 1.11）
  目標定義の引き口   … Get-GoalConfig / Get-JudgeScriptVersion
                       Get-MeasureToml / Get-MeasureTomlInt / Get-MeasureTomlArray
                       Get-Measure* は $script:GoalConfig.Toml を引く節・鍵の読み口で、
                       呼び手は計測の本体（perf-loop.measure.ps1）＝置き場だけがここ
  出力先の配置       … Get-LoopDir / Resolve-RunDir / Resolve-LoopPath / New-LoopDir / Test-ResumeArtifact
  preflight の判定   … Get-PreflightCode / Test-GoalTextCharCount（純関数）/ Get-PythonVersionText
                       %LOCALAPPDATA%\areka-diag\perf-loop\<goal>\… の唯一の所在

なぜ別ファイルなのか:
  同じ入口が計測サブコマンド 8 本（measure-baseline／rank-run／rank／prepare-ab／
  measure-ab／compare／followup／final）まで持つ。1 ファイル 1,000 行の上限
  （要件・steering）に収めるため、入口（perf-loop.ps1）・共通部品（本ファイル）・
  計測の本体（perf-loop.measure.ps1。実行体の用意 perf-loop.build.ps1 と出力先の扱い
  perf-loop.rundir.ps1 を自分で読み込む）に分けてある。

呼び出し側が事前に置いておく script scope の変数（Exit-WithResult ほかが読む）:
  $script:CurrentSub   … 実行中のサブコマンド名（RESULT 行の <sub>）
  $script:CurrentDir   … そのサブコマンドの出力先（無ければ '-'）
  $script:OutRootPath  … 出力先の根（既定は %LOCALAPPDATA%\areka-diag\perf-loop）
  $script:GoalName     … 目標の名前（出力先の第 2 階層）
================================================================================
#>

Set-StrictMode -Version 3.0

# 表示・時刻・子の起こし方・目標定義の読み・検体の在り処（道具どうしの共通部品）
. (Join-Path $PSScriptRoot 'perf-tool-base.ps1')

# =============================================================================
# 較正値・調整値（perf-loop.ps1 の一覧と対応。変更はここだけ）
# =============================================================================

#: RESULT 行の冠。読む側（perf-loop-iteration スキル・台帳）はこの字面に依存する。
$PERF_LOOP_RESULT_PREFIX = 'PERF-LOOP RESULT'

#: 出力先の既定の根。%LOCALAPPDATA% が無い環境では TEMP へ落ちる。
$PERF_LOOP_OUTROOT_SUBDIR = 'areka-diag\perf-loop'

#: 周（iteration）の出力先に置く小部屋の語彙（design C5 の配置そのもの）。
$PERF_LOOP_ITER_LEAVES = @('rank', 'A1', 'B1', 'A2', 'B2', 'bin-A', 'bin-B', 'followup')

#: 値が無いことを表す字面（台帳・RESULT 行と同じ・所在は perf-tool-base）。
$PERF_LOOP_EMPTY = $PERF_TOOL_EMPTY

#: 終了コード（入口・測る側・実行体の用意・出力先の扱いが共に使う）
$EXIT_OK             = 0
$EXIT_RUN_FAILED     = 1
$EXIT_NOT_QUIET      = 2
$EXIT_BAD_ARGS       = 3
$EXIT_MEASURE_FAILED = 4
$EXIT_UNAVAILABLE    = 5

#: 計測するビルドの種類（-Build の語彙）
$BUILD_KINDS = @('dev', 'release')

#: リポジトリの根（tools/perf の 2 つ上）
$repoRoot = $PERF_TOOL_REPO_ROOT

# =============================================================================
# 終了
# =============================================================================

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
    Write-Problem "[perf-loop] FAILED: $Message"
    Exit-WithResult -Code $Code -Sub $Sub -Dir $Dir
}

# 想定外の失敗（入口の頭の trap から呼ぶ）。理由を 1 行出し、4（計測失敗の枠＝道具の不具合）で終える。
function Exit-OnUnexpectedError {
    param([Parameter(Mandatory = $true)][System.Management.Automation.ErrorRecord]$ErrorRecord)
    Stop-Run -Code $EXIT_MEASURE_FAILED -Message (Format-UnexpectedError -ErrorRecord $ErrorRecord -Label 'unexpected failure (tool bug)')
}

# =============================================================================
# 目標定義の引き口（節・鍵の読みそのものは perf-tool-base.ps1）
# =============================================================================

# 目標定義ファイルの所在。-GoalFile が最優先、無ければ goals/<Goal>.toml。
function Resolve-GoalFile {
    param([string]$Goal, [string]$GoalFile, [Parameter(Mandatory = $true)][string]$ScriptRoot)
    if ($GoalFile) { return $GoalFile }
    return (Join-Path $ScriptRoot (Join-Path 'goals' "$Goal.toml"))
}

# 目標定義を読み、道具が要る値だけを取り出す。
# 返す: Path・Name・SpecDir・LedgerPath・ResultsDir・JudgeScriptPath・JudgeVersion・CheckinMinutes・Toml
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
    # 書いた値を黙って既定へ落とさない（読めなければ 3・Get-MeasureTomlInt と同じ決まり）
    if ($null -eq $checkin -and $null -ne ($rawCheckin = Get-TomlRaw -Toml $toml -Section 'goal_runtime' -Key 'checkin_minutes')) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [goal_runtime] checkin_minutes cannot be read as an integer: $rawCheckin"
    }
    # 走行の後の静かさの確かめの回数・待ち秒数も、ビルドと走行の前に確かめる（-DryRun でも。後の Invoke-MeasureQuietAfter の確かめも残す）
    foreach ($quietKey in 'retry_max', 'retry_wait_sec') {
        $rawQuiet = Get-TomlRaw -Toml $toml -Section 'quiet' -Key $quietKey
        $quiet = Get-TomlInt -Toml $toml -Section 'quiet' -Key $quietKey
        if ($null -ne $rawQuiet -and ($null -eq $quiet -or $quiet -lt 0)) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [quiet] $quietKey must be an integer 0 or more: $rawQuiet"
        }
    }

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
        LedgerPath      = $ledgerPath
        ResultsDir      = $resultsDir
        JudgeScriptPath = $judgePath
        JudgeVersion    = $judgeVer
        CheckinMinutes  = $checkin
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

# 鍵が無ければ既定値。鍵は在るのに整数で読めなければ 3 で止める（書いた値を黙って既定へ落とさない）
function Get-MeasureTomlInt {
    param([Parameter(Mandatory = $true)][string]$Section, [Parameter(Mandatory = $true)][string]$Key, [int]$Default)
    if (-not $script:GoalConfig) { return $Default }
    $value = Get-TomlInt -Toml $script:GoalConfig.Toml -Section $Section -Key $Key
    if ($null -ne $value) { return $value }
    $raw = Get-TomlRaw -Toml $script:GoalConfig.Toml -Section $Section -Key $Key
    if ($null -ne $raw) { Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [$Section] $Key cannot be read as an integer: $raw" }
    return $Default
}

# 1 行の配列（["A", "B", "A", "B"]）を文字列の配列へ。鍵が無ければ既定値
# （目標定義ファイルは配列を 1 行に保つ決まり＝tasks.md Implementation Notes (5.1)）。
# 鍵は在るのに 1 行の配列として読めない・空の配列なら 3 で止める（どの呼び手も空の集合では回せない）。
function Get-MeasureTomlArray {
    param([Parameter(Mandatory = $true)][string]$Section, [Parameter(Mandatory = $true)][string]$Key, [string[]]$Default)
    if (-not $script:GoalConfig) { return $Default }
    $raw = Get-TomlRaw -Toml $script:GoalConfig.Toml -Section $Section -Key $Key
    if ($null -eq $raw) { return $Default }
    $items = Get-TomlStringArray -Toml $script:GoalConfig.Toml -Section $Section -Key $Key
    if ($null -eq $items -or $items.Count -eq 0) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [$Section] $Key must be a non-empty one-line array of strings: $raw"
    }
    return $items
}

# judge-perf.py の SCRIPT_VERSION の先頭語（例 "0.4.0"）を、スクリプトを起動せずに読む。
# 判定スクリプトは複数行の括弧つき文字列で版を持つ（judge-perf.py の SCRIPT_VERSION の定義）ため、
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
    if (-not $root) { throw 'output root is not set (call Resolve-OutRoot first).' }
    if (-not $goalName) { throw 'goal name is not set.' }

    $path = Join-Path $root $goalName
    switch ($Kind) {
        'goal' { }
        'iter' {
            if ($Iter -lt 1) { throw "-Iter must be 1 or more (got: $Iter)." }
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
            throw "iteration subfolder name is not in the vocabulary: '$Leaf' (allowed: $($PERF_LOOP_ITER_LEAVES -join ', '))"
        }
        $path = Join-Path $path $Leaf
    }
    return $path
}

# 出力先の名前に使う日付（既定は最初に聞かれた時のローカル時刻・yyyyMMdd）。1 回の呼び出しの中では
# 一度決めたら変えない（日付をまたぐ final の走行フォルダと結果の写し先が別の日付にならない）。
# 入口の -Date（$script:DateStamp）で上書きできるのは、同じ日付の出力先を後から作り直す／試験で決定論にするため。
# 決めた今日は $script:LoopDateToday に置く（$script:DateStamp は「-Date が渡された」の印でもあるので書き替えない）。
$script:LoopDateToday = $null
function Get-LoopDateStamp {
    if ($script:DateStamp) { return $script:DateStamp }
    if (-not $script:LoopDateToday) { $script:LoopDateToday = (Get-Date).ToString('yyyyMMdd', [Globalization.CultureInfo]::InvariantCulture) }
    return $script:LoopDateToday
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
    if ($RunDir) { return (Resolve-LoopPath $RunDir).TrimEnd('\') }
    return (Get-LoopDir -Kind $Kind -Iter $Iter -Date $Date -Leaf $Leaf)
}

# パスの引数を PowerShell の今の場所（プロセスの今の場所ではない）につないで絶対にする。空なら空を返す。
# 入口（perf-loop.ps1）がパスの引数ごとに一度だけ呼ぶ（27＝解き方を 1 通りにする）。
function Resolve-LoopPath {
    param([string]$Path)
    if (-not $Path) { return $Path }
    return $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
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
    Write-Info "[perf-loop] -Resume: reusing existing output: $Path"
    return $true
}

# =============================================================================
# 環境の見立て（preflight が読む）
# =============================================================================

# invoke-cpu-sample.ps1 -Probe の 1 行を読む（昇格・xperf の在り処・段③の可否の出どころはここだけ）:
#   available=<真偽> reason=<語> elevated=<真偽> xperf_path=<パス|->（在り処は行の最後で空白や括弧を含み得る）
# 戻り: Available・Reason・Elevated・XperfPath（無ければ $null）・Code・Lines。読めなければ reason=probe_failed。
function Get-SamplingProbe {
    $path = Join-Path $PSScriptRoot 'invoke-cpu-sample.ps1'
    $result = Invoke-PwshChild -ScriptPath $path -Arguments @('-Probe') -Quiet
    $probe = [pscustomobject]@{ Available = $false; Reason = 'probe_failed'; Elevated = $false; XperfPath = $null; Code = $result.Code; Lines = $result.Lines }
    # -Probe は必ず 0 で返る約束。非ゼロは道具自身の異常
    $read = $false
    if ($result.Code -eq 0) {
        foreach ($line in $result.Lines) {
            if ($line -match '^available=(true|false) reason=(\S+) elevated=(true|false) xperf_path=(.+)$') {
                $probe.Available = ($Matches[1] -eq 'true')
                $probe.Reason    = $Matches[2]
                $probe.Elevated  = ($Matches[3] -eq 'true')
                $probe.XperfPath = if ($Matches[4] -eq '-') { $null } else { $Matches[4] }
                $read = $true
            }
        }
    }
    # 昇格と xperf の値もこの 1 行が唯一の出どころなので、読めなかった訳を黙って落とさない
    if (-not $read) { Write-Problem "[perf-loop] warning: invoke-cpu-sample.ps1 -Probe failed (exit code $($result.Code)): $($result.LastLine) - elevated and xperf are reported as false." }
    return $probe
}

# python の版を「3.13.15」の形で得る（読めなければ $null）。
function Get-PythonVersionText {
    param([pscustomobject]$Python)
    if (-not $Python) { return $null }
    $result = Invoke-Child -Exe $Python.Exe -Arguments (@($Python.Prefix) + @('-c', 'import sys;print("%d.%d.%d" % sys.version_info[:3])')) -Quiet
    # 標準出力の最後の行だけ（python の警告は標準エラーに出るので混ぜない）
    $out = @($result.Out | Where-Object { $_ })
    if ($result.Code -ne 0 -or $out.Count -eq 0) { return $null }
    $last = $out[-1].Trim()
    if ($last -match '^\d+\.\d+') { return $last }
    return $null
}

# preflight の合否（純関数）。順に見て最初に落ちたものを理由にする:
# 判定スクリプトの版 → python の版 → 自分の検査 → goal-check。どれか落ちれば 4（計測失敗）。
# goal-check の 'skipped'（台帳がまだ無い）は落ちたことにしない。
function Get-PreflightCode {
    param(
        [bool]$JudgeMatch, [string]$JudgeNote,
        [bool]$PythonOk, [string]$PythonNote,
        [bool]$SelftestOk, [int]$SelftestNgCount,
        [string]$GoalCheck, [string]$GoalCheckNote
    )
    $reason = if (-not $JudgeMatch) { "judge script version differs from the goal definition ($JudgeNote)" }
    elseif (-not $PythonOk)         { "python check failed: $PythonNote" }
    elseif (-not $SelftestOk)       { "tool selftest has $SelftestNgCount red" }
    elseif ($GoalCheck -eq 'ng')    { "ledger vs goal definition check (goal-check) failed: $GoalCheckNote" }
    else                            { '' }
    $code = if ($reason) { $EXIT_MEASURE_FAILED } else { $EXIT_OK }
    return [pscustomobject]@{ Code = $code; Reason = $reason }
}

# goal-text の字数の突き合わせ（純関数）。写しの頭書（「本文は 1,012 字」）に字数が無いときだけ飛ばす（Ok）。
# 頭書に在るのに goal-text の報告（標準エラーの「1012 文字」）から読めないときと、数が違うときは赤。
# 突き合わせる日本語（「本文は」「字」「文字」）は字句を ASCII に保つため `u{} で書く（字は同じ）。
function Test-GoalTextCharCount {
    param([string]$ErrText, [string]$MdText)
    if ($MdText -notmatch "`u{672C}`u{6587}`u{306F}\s*([\d,]+)\s*`u{5B57}") {
        return [pscustomobject]@{ Ok = $true; Note = 'no char count in the copy header (count check skipped)' }
    }
    $declared = [int](($Matches[1]) -replace ',', '')
    if ($ErrText -notmatch "(\d+)\s*`u{6587}`u{5B57}") {
        return [pscustomobject]@{ Ok = $false; Note = "cannot read the char count from the goal-text report (the copy header says $declared chars)" }
    }
    $reported = [int]$Matches[1]
    if ($reported -ne $declared) {
        return [pscustomobject]@{ Ok = $false; Note = "the copy header says $declared chars but goal-text reports $reported (fix only the number in the header)" }
    }
    return [pscustomobject]@{ Ok = $true; Note = "$reported chars, also matches the copy header" }
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
