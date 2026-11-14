#Requires -Version 7.0
<#
================================================================================
perf-loop.measure.ps1 — perf-loop.ps1 が dot-source する計測サブコマンド 8 本の本体
  spec: areka-P0-draw-load-parity（要件 1.2 / 2.7 / 2.10 / 5.6・design「計測の道具
        （tools/perf/）→ C5 perf-loop.ps1 → Batch / Job Contract」・Flow 2）

このファイルは単体では何も実行しない。perf-loop.ps1 の先頭（較正値と共通部品の後）で

    . (Join-Path $PSScriptRoot 'perf-loop.measure.ps1')

として読み込むと、8 本のサブコマンド本体（measure-baseline／rank-run／rank／
prepare-ab／measure-ab／compare／followup／final）が呼び出し側の scope に入る。
どれが何をするかの一覧は perf-loop.ps1 の頭書にある（入口は 1 つ＝要件 2.10）。

なぜ別ファイルなのか: 1 ファイル 1,000 行の上限（要件・steering）。perf-loop.ps1 は
入口・引数・preflight・selftest を持ち、ここは「実際に測る 8 本」だけを持つ。

--------------------------------------------------------------------------------
どの走行にも共通する 4 つの決まり
--------------------------------------------------------------------------------
 ⒜ 実走は必ず invoke-perf-run.ps1（C7）を通す。**出力先は毎回まっさらな新規フォルダ**
    でなければならない（ランナー側の決まり）。失敗した走行の出力先はランナーが
    `<leaf>-FAILED` へ退避して `[invoke-perf-run] moved to: <path>` の 1 行で知らせるので、
    1 度だけの採り直しは同じ出力先で行える。その行が無く出力先が残っていれば採り直さずに 4。
 ⒝ 静寂の確認は前後 2 回。前（quiet-before.txt）はランナーの `-AutoQuiet` が書く。
    後（quiet-after.txt）は**ここの仕事**で、check-quiet.ps1 -Stage after を呼ぶ。
    「静かでない」なら目標定義 [quiet] の retry_max 回だけ待って確かめ直し、
    それでも静かでなければ 2（静寂でない）で終わる（要件 2.8・design C6）。
 ⒞ 実走の失敗は 1 度だけ採り直し、なお失敗なら 4（計測失敗＝MEASURE_FAILED）。
    引数・前提の不正は 3、静寂でないは 2 をそのまま返す（tasks.md 7.3）。
 ⒟ **絶対値の合否を出す走行（measure-baseline／final）は実 SHIORI を伴うこと。**
    32bit ヘルパが無いと発話が起きず、アイドルの絶対値は別条件の値になる。
    採取ランナーは警告だけで走ってしまう（invoke-perf-run.ps1 の shiori_helper_present の注記）ので、
    合否を出す前に run-meta.txt の shiori_helper_present を関門にして 4 で止める。
    相対比較（rank-run／measure-ab）は警告 1 行を残して続ける。

--------------------------------------------------------------------------------
-DryRun（試験専用。測定にはならない）
--------------------------------------------------------------------------------
  配管（出力先の階層・成果物の有無・-Resume・RESULT 行）だけを安く確かめるための旗。
  areka を起動せず、静寂確認・cargo build・サンプリング・追随チェックも行わない。
  **quiet-*.txt を偽造しない**（証跡は作らない）。順位表・比較・合否判定は入力が実在
  するときだけ回す。出力先には DRY-RUN.txt が残り、もう一度 -DryRun を掛けたときだけ
  作り直す（run.log がある出力先には絶対に触らない）。

--------------------------------------------------------------------------------
較正値・調整値の一覧（変更する場合はここだけを書き換える）
--------------------------------------------------------------------------------
  MEASURE_SAMPLE_NAME／MEASURE_SAMPLE_BALLOON_NAME  -GhostRoot／-BalloonRoot 省略時に
                               検体の窓口（nar-sample-path）へ尋ねる検体名とバルーン名
  MEASURE_AREKA_*／MEASURE_HELPER_*  A/B ビルドで作って複製するもの（実行体・PDB・
                               32bit SHIORI ヘルパの crate 名・実行体名・ターゲット）
  MEASURE_RUN_RETRY            実走の採り直し回数（1＝合計 2 回まで）
  MEASURE_RANK_RUSTLOG_EXTRA   順位付け走行で点灯させる target（Flow 2）
  MEASURE_*_DEFAULT            目標定義ファイルに節・鍵が無いときの既定値
  MEASURE_FOLLOWUP_CHECKS_ALL  見た目の追随チェックの固定語彙（[followup] required の検算）
================================================================================
#>

Set-StrictMode -Version 3.0

# =============================================================================
# 較正値・調整値（上の一覧と対応。変更はここだけ）
# =============================================================================
$MEASURE_SAMPLE_NAME           = 'emo2'
$MEASURE_SAMPLE_BALLOON_NAME   = 'emo2-kakukaku'
$MEASURE_AREKA_PACKAGE         = 'areka'
$MEASURE_AREKA_EXE             = 'areka.exe'
$MEASURE_AREKA_PDB             = 'areka.pdb'
$MEASURE_HELPER_PACKAGE        = 'shiori-host32-helper'
$MEASURE_HELPER_EXE            = 'shiori-host32-helper.exe'
$MEASURE_HELPER_TARGET         = 'i686-pc-windows-msvc'
$MEASURE_RUN_RETRY             = 1
$MEASURE_RANK_RUSTLOG_EXTRA    = 'wintf::tick=debug,areka::perf=debug'
$MEASURE_AB_SEQUENCE_DEFAULT   = @('A', 'B', 'A', 'B')
$MEASURE_BUILDS_FINAL_DEFAULT  = @('release', 'dev')
$MEASURE_SHORT_PROFILE_DEFAULT = 'short'
$MEASURE_LONG_PROFILE_DEFAULT  = 'long'
$MEASURE_ITER_BUILD_DEFAULT    = 'release'
$MEASURE_RELEASE_DEBUG_DEFAULT = 'line-tables-only'
$MEASURE_FOLLOWUP_EXIT_MS_DEFAULT = 120000
#: 見た目の追随チェックの固定語彙（invoke-followup-checks.ps1 の $CHECK_ALL・
#: judge-followup.py の CHECK_ALL と対。目標定義 [followup] required の検算に使う）
$MEASURE_FOLLOWUP_CHECKS_ALL      = @('clickthrough', 'drag', 'dpi', 'balloon_follow')
$MEASURE_QUIET_RETRY_MAX_DEFAULT  = 3
$MEASURE_QUIET_RETRY_WAIT_DEFAULT = 60

#: 走行の成果物の名前（ランナー・道具と揃える写し）
$MEASURE_FILE_RUN_LOG      = 'run.log'
$MEASURE_FILE_CPU_CSV      = 'cpu.csv'
$MEASURE_FILE_RUN_META     = 'run-meta.txt'
$MEASURE_FILE_QUIET_BEFORE = 'quiet-before.txt'
$MEASURE_FILE_QUIET_AFTER  = 'quiet-after.txt'
$MEASURE_FILE_RANK         = 'rank.txt'
$MEASURE_FILE_DUMP         = 'dump.txt'
$MEASURE_FILE_SAMPLING     = 'sampling.txt'
$MEASURE_FILE_VERDICT      = 'verdict.txt'
$MEASURE_FILE_COMPARE      = 'compare.txt'
$MEASURE_FILE_BUILD        = 'BUILD.txt'
$MEASURE_FILE_DRYRUN       = 'DRY-RUN.txt'
$MEASURE_FILE_FOLLOWUP     = 'followup.txt'      # 中止のときも書かれる（完了の印にしない）
$MEASURE_FILE_PROBE_LOG    = 'probe.log'
$MEASURE_FILE_FOLLOWUP_VERDICT = 'followup-verdict.txt'  # 判定まで届いた印
$MEASURE_TRACE_SUFFIX      = '-trace.etl'

#: A/B の側 → 出力先の小部屋（design C5 の配置）
$MEASURE_SIDE_LEAF = @{ 'A' = 'bin-A'; 'B' = 'bin-B' }

# 実行体の用意と出力先の扱い（自分の較正値 $MEASURE_* を置いた後で読み込む）
. (Join-Path $PSScriptRoot 'perf-loop.build.ps1')
. (Join-Path $PSScriptRoot 'perf-loop.rundir.ps1')

# =============================================================================
# 目標定義の引数（節・鍵そのものの読み口 Get-MeasureToml／-Int／-Array は
# perf-loop.common.ps1 に置いてある。本ファイルの行数上限のため＝要件 2.10）
# =============================================================================

function Get-MeasureGoalArgsPwsh {
    if ($script:GoalFilePath) { return @('-GoalFile', $script:GoalFilePath) }
    return @('-Goal', $script:GoalName)
}

function Get-MeasureGoalArgsPython {
    if ($script:GoalFilePath) { return @('--goal-file', $script:GoalFilePath) }
    return @('--goal', $script:GoalName)
}

# =============================================================================
# 場所（ゴースト・実行体・出力先）と前提
# =============================================================================
# 既定の在り処は検体の窓口に尋ねる（Get-NarSamplePath は perf-tool-base.ps1・1 検体 1 回・止めない）。
# 窓口が答えられなければ 4 で止める（空のパスで走らせない）。
function Get-MeasureSamplePath {
    param([Parameter(Mandatory = $true)][string]$Key)
    $r = Get-NarSamplePath -Sample $MEASURE_SAMPLE_NAME -Key $Key
    if (-not $r.Ok) { Stop-Run -Code $EXIT_MEASURE_FAILED -Message $r.Message }
    return $r.Value
}

function Get-MeasureGhostRoot {
    $path = if ($script:GhostRootArg) { $script:GhostRootArg } else { Get-MeasureSamplePath -Key 'folder' }
    return [System.IO.Path]::GetFullPath($path).TrimEnd('\')
}

function Get-MeasureBalloonRoot {
    $path = if ($script:BalloonRootArg) { $script:BalloonRootArg } else { Get-MeasureSamplePath -Key "balloon.$MEASURE_SAMPLE_BALLOON_NAME" }
    return [System.IO.Path]::GetFullPath($path).TrimEnd('\')
}


function Assert-MeasureIter {
    if ($script:IterArg -lt 1) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "-Iter must be 1 or more (got: $($script:IterArg)). The iteration output folder is iter-<n>."
    }
}

function Get-MeasurePython {
    $python = Get-PythonCommand
    if (-not $python) {
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message 'python not found (the rank table, comparison and verdict are made by python tools)'
    }
    return $python
}

# 子の道具の終了コードを perf-loop の体系へ写す（同じ体系なのでそのまま。語彙外は計測失敗）。
function Convert-MeasureChildExit {
    param([Parameter(Mandatory = $true)][int]$Code)
    if ($Code -eq $EXIT_BAD_ARGS)    { return $EXIT_BAD_ARGS }
    if ($Code -eq $EXIT_UNAVAILABLE) { return $EXIT_UNAVAILABLE }
    return $EXIT_MEASURE_FAILED
}


# =============================================================================
# 実走・静寂・合否・順位表（8 本が共有する 4 つの動作）
# =============================================================================

# 子の出力の決まった行から語を 1 つ読む（最後に合った行の 1 つ目の括り・無ければ $null）
function Get-MeasureLineWord {
    param([string[]]$Lines, [Parameter(Mandatory = $true)][string]$Pattern)
    $word = $null
    foreach ($line in @($Lines)) { if ($line -match $Pattern) { $word = $Matches[1] } }
    return $word
}

# 走行の出力先が空いているか（-DryRun の印だけのフォルダは作り直す）。在れば 3 で止める。
# ビルドより前に呼ぶ（既に在る出力先のためにビルドの数分と target\<build> の書き替えを使わない）。
function Assert-MeasureRunDirFree {
    param([Parameter(Mandatory = $true)][string]$Dir)
    Clear-MeasureDryRunDir -Dir $Dir
    if (Test-Path -LiteralPath $Dir) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "output folder already exists: $Dir (each capture makes a new folder. To capture again, delete it; to use the existing artifacts, add -Resume)"
    }
}

# 実走を 1 本採る。失敗は 1 度だけ採り直し、なお失敗なら 4 で止まる（tasks.md 7.3）。
# 静寂でない（ランナーの 2）は採り直さずそのまま 2 で止まる——待って確かめ直すのは
# ランナー側が目標定義の回数だけ既に行っているためである（design C6・C7）。
# 戻り: SamplingStarted（ランナーが '[invoke-perf-run] sampling start: code=0' を出した＝採取が動いたまま返った）と
# SamplingReason（その行の reason=・行が無ければ start_failed・-SampleEtl が無ければ '-'）。
# 採取を始めた後の失敗の道はランナーが -Cancel で止めて返るので、ここは 0 の道だけを返す（30）
function Invoke-MeasureRun {
    param(
        [Parameter(Mandatory = $true)][string]$OutDir,
        [Parameter(Mandatory = $true)][string]$RunProfile,
        [Parameter(Mandatory = $true)][string]$Build,
        [string]$BinDir,
        [string]$RustLogExtra,
        [string]$SampleEtl,
        [string]$Label = 'run'
    )
    $runnerPath = Join-Path $PSScriptRoot 'invoke-perf-run.ps1'
    $ghostRoot   = Get-MeasureGhostRoot
    $balloonRoot = Get-MeasureBalloonRoot
    $attempts = $MEASURE_RUN_RETRY + 1
    if (-not $script:DryRunMode) { Sync-MeasureShioriHelper -BinDir $BinDir -Build $Build }

    for ($attempt = 1; $attempt -le $attempts; $attempt++) {
        Assert-MeasureRunDirFree -Dir $OutDir
        $arguments = @(
            '-Profile', $RunProfile,
            '-Build', $Build,
            '-GhostRoot', $ghostRoot,
            '-BalloonRoot', $balloonRoot,
            '-OutDir', $OutDir
        )
        if ($script:DryRunMode) {
            $arguments += '-DryRun'
        } else {
            $arguments += '-AutoQuiet'
            $arguments += Get-MeasureGoalArgsPwsh
        }
        if ($BinDir)       { $arguments += @('-BinDir', $BinDir) }
        if ($RustLogExtra) { $arguments += @('-RustLogExtra', $RustLogExtra) }
        if ($SampleEtl)    { $arguments += @('-SampleEtl', $SampleEtl) }

        Write-Info "[perf-loop] capturing $Label ($attempt/$attempts, level $RunProfile, build $Build, out $OutDir)"
        $result = Invoke-PwshChild -ScriptPath $runnerPath -Arguments $arguments
        if ($result.Code -eq 0) {
            if (-not $SampleEtl) { return [pscustomobject]@{ SamplingStarted = $false; SamplingReason = $PERF_LOOP_EMPTY } }
            $startCode = Get-MeasureLineWord -Lines $result.Lines -Pattern '^\[invoke-perf-run\] sampling start: code=(-?\d+) '
            $startWhy  = Get-MeasureLineWord -Lines $result.Lines -Pattern '^\[invoke-perf-run\] sampling start: code=\S+ reason=(\S+)'
            return [pscustomobject]@{ SamplingStarted = ($startCode -eq '0'); SamplingReason = $(if ($startWhy) { $startWhy } else { 'start_failed' }) }
        }
        # ランナーの決まった 1 行 '[invoke-perf-run] moved to: <path>' から失敗した走行の退け先を読む（無ければ $null）
        $moved = Get-MeasureLineWord -Lines $result.Lines -Pattern '^\[invoke-perf-run\] moved to: (.+?)\s*$'
        if ($result.Code -eq 2) {
            $evidence = if ($moved) { Join-Path $moved $MEASURE_FILE_QUIET_BEFORE } else { "$MEASURE_FILE_QUIET_BEFORE in $OutDir-FAILED*" }
            Stop-Run -Code $EXIT_NOT_QUIET -Message "${Label}: the automatic quiet check did not pass (after waiting and rechecking as many times as goal [quiet] says). The evidence is $evidence"
        }
        if ($result.Code -eq 3) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "${Label}: the capture runner stopped on bad arguments or preconditions: $($result.LastLine)"
        }
        if (-not $moved -and (Test-Path -LiteralPath $OutDir)) {
            Stop-Run -Code $EXIT_MEASURE_FAILED -Message "${Label}: the failed run could not be moved aside, so it is not captured again: $OutDir (exit code $($result.Code)): $($result.LastLine)"
        }
        Write-Problem "[perf-loop] $Label failed (exit code $($result.Code)): $($result.LastLine)"
        if ($attempt -lt $attempts) { Write-Info "[perf-loop] capturing once more." }
    }
    Stop-Run -Code $EXIT_MEASURE_FAILED -Message "$Label failed all $attempts times (MEASURE_FAILED)"
}

# 走行の後の静寂確認。「静かでない」なら目標定義の回数だけ待って確かめ直す（design C6）。
function Invoke-MeasureQuietAfter {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    if ($script:DryRunMode) {
        Write-Info '[perf-loop] -DryRun: no quiet check after the run (and no quiet-after.txt).'
        return
    }
    $checkPath = Join-Path $PSScriptRoot 'check-quiet.ps1'
    $retryMax  = Get-MeasureTomlInt -Section 'quiet' -Key 'retry_max' -Default $MEASURE_QUIET_RETRY_MAX_DEFAULT
    $waitSec   = Get-MeasureTomlInt -Section 'quiet' -Key 'retry_wait_sec' -Default $MEASURE_QUIET_RETRY_WAIT_DEFAULT
    if ($retryMax -lt 0 -or $waitSec -lt 0) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [quiet] retry_max and retry_wait_sec must be 0 or more (got $retryMax / $waitSec)"
    }
    for ($i = 0; $i -le $retryMax; $i++) {
        $arguments = @('-Stage', 'after', '-OutDir', $RunDir) + (Get-MeasureGoalArgsPwsh)
        $result = Invoke-PwshChild -ScriptPath $checkPath -Arguments $arguments -Quiet
        if ($result.Code -eq 0) {
            Write-Info "[perf-loop] quiet check after the run: quiet ($RunDir\$MEASURE_FILE_QUIET_AFTER)"
            return
        }
        if ($result.Code -eq 3) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "quiet check after the run stopped on bad arguments or settings: $($result.LastLine)"
        }
        if ($result.Code -eq 4) {
            Stop-Run -Code $EXIT_MEASURE_FAILED -Message "quiet check after the run could not read the performance counters (measure failed): $($result.LastLine)"
        }
        # 静かでない（2）のほかは道具が回らなかった（落ちた・起こせない）。待って確かめ直さない
        if ($result.Code -ne 2) {
            Stop-Run -Code $EXIT_MEASURE_FAILED -Message "quiet check after the run did not run (exit code $($result.Code), measure failed): $($result.LastLine)"
        }
        Write-Info "[perf-loop] quiet check after the run: not quiet ($($i + 1)/$($retryMax + 1)): $($result.LastLine)"
        if ($i -lt $retryMax) {
            Write-Info "[perf-loop] waiting $waitSec s and checking again."
            Start-Sleep -Seconds $waitSec
        }
    }
    Stop-Run -Code $EXIT_NOT_QUIET -Message "quiet check after the run was not quiet all $($retryMax + 1) times (evidence $RunDir\$MEASURE_FILE_QUIET_AFTER)"
}

# 合否判定（judge-perf.py --mode verdict）。出力は verdict.txt に残し、判定語を返す。
# 判定が付かない（2 判定不能・3 引数不正）は計測失敗（4）——黙って合格にしない。
function Invoke-MeasureJudge {
    param([Parameter(Mandatory = $true)][string]$RunDir, [Parameter(Mandatory = $true)][string]$Build)
    if (-not (Test-Path -LiteralPath (Join-Path $RunDir $MEASURE_FILE_RUN_LOG) -PathType Leaf)) {
        if ($script:DryRunMode) {
            Write-Info '[perf-loop] -DryRun: no run artifacts, so no verdict.'
            return $PERF_LOOP_EMPTY
        }
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message "run log not found: $RunDir\$MEASURE_FILE_RUN_LOG"
    }
    # 合否を出す走行は実 SHIORI を伴っていなければならない（発話の無い別条件を判定しない）
    Assert-MeasureShioriHelper -RunDir $RunDir -Absolute -Label "verdict run ($Build)" | Out-Null
    $python = Get-MeasurePython
    $judgePath = if ($script:GoalConfig -and $script:GoalConfig.JudgeScriptPath) {
        $script:GoalConfig.JudgeScriptPath
    } else {
        Join-Path $PSScriptRoot 'judge-perf.py'
    }
    $arguments = @(
        $judgePath,
        (Join-Path $RunDir $MEASURE_FILE_RUN_LOG),
        (Join-Path $RunDir $MEASURE_FILE_CPU_CSV),
        '--mode', 'verdict',
        '--build', $Build,
        '--meta', (Join-Path $RunDir $MEASURE_FILE_RUN_META)
    )
    $result = Invoke-PythonChild -Python $python -Arguments $arguments -Quiet
    $verdictPath = Join-Path $RunDir $MEASURE_FILE_VERDICT
    Set-Content -LiteralPath $verdictPath -Value ((@($result.Lines) -join "`n") + "`n") -Encoding utf8 -NoNewline
    foreach ($line in $result.Lines) { Write-Info $line }
    if ($result.Code -eq 0) { Write-Info "[perf-loop] verdict PASS ($verdictPath)"; return 'PASS' }
    if ($result.Code -eq 1) { Write-Info "[perf-loop] verdict FAIL ($verdictPath)"; return 'FAIL' }
    Stop-Run -Code $EXIT_MEASURE_FAILED `
        -Message "no verdict (judge-perf.py exit code $($result.Code), $verdictPath): $($result.LastLine)"
}

# run-meta.txt の shiori_helper_present を読む（真／偽／読めなければ $null）。
function Get-MeasureShioriHelperPresent {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    $path = Join-Path $RunDir $MEASURE_FILE_RUN_META
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $null }
    foreach ($line in (Get-Content -LiteralPath $path -Encoding utf8)) {
        if ($line -match '^\s*shiori_helper_present\s*=\s*(\S+)\s*$') { return ($Matches[1] -imatch '^true$') }
    }
    return $null
}

# 実 SHIORI（32bit ヘルパ）を伴う走行だったかを確かめる。
#   -Absolute（measure-baseline／final＝**絶対値の合否**を出す走行）… 無ければ 4 で止める。
#     ヘルパが無いと SHIORI が居らず発話が起きない。発話の無い走行は「アイドル 3.0% 未満」
#     を満たしやすい**別条件**であり、その合否を台帳へ載せると静かに嘘をつく。
#     採取ランナーは警告を出すだけで走ってしまう（invoke-perf-run.ps1 の shiori_helper_present の注記）ので、
#     絶対値の合否を出す側でここを関門にする。
#   それ以外（rank-run／measure-ab＝**相対比較**）… 警告と 1 行の記録を残して続ける
#     （A と B が同じ条件なら差そのものは読める）。
function Assert-MeasureShioriHelper {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [switch]$Absolute,
        [string]$Label = 'run'
    )
    $present = Get-MeasureShioriHelperPresent -RunDir $RunDir
    if ($present -eq $true) { return $true }
    $word = if ($null -eq $present) { $PERF_LOOP_EMPTY } else { 'false' }
    if ($Absolute) {
        Stop-Run -Code $EXIT_MEASURE_FAILED -Dir $RunDir -Message ("${Label}: run without the 32bit SHIORI helper (shiori_helper_present=$word in run-meta.txt). " +
            "Without SHIORI there is no talk, so the idle absolute value comes from a different condition and cannot be used for the verdict. " +
            "Run cargo build --release -p $MEASURE_HELPER_PACKAGE --target $MEASURE_HELPER_TARGET and capture again.")
    }
    Write-Problem "[perf-loop] warning: ${Label} ran without the 32bit SHIORI helper (shiori_helper_present=$word). With no talk it is usable only for relative comparison."
    Write-Info "[perf-loop] shiori_helper_present=$word dir=$RunDir"
    return $false
}

# 段③（関数別の帰属）の可否を走行の出力先へ残す。rank が後から読む（二重に測らない）。
function Write-MeasureSamplingNote {
    param([Parameter(Mandatory = $true)][string]$RunDir, [bool]$Available, [Parameter(Mandatory = $true)][string]$Reason)
    if (-not (Test-Path -LiteralPath $RunDir -PathType Container)) { return }
    if ($script:DryRunMode) { return }   # 6: -DryRun の出力先に印のほかのファイルを置かない（次の本番が 3 で止まる）
    $lines = @("available=$(Format-LoopBool $Available)", "reason=$Reason", "time_utc=$(Get-UtcStamp)")
    Set-Content -LiteralPath (Join-Path $RunDir $MEASURE_FILE_SAMPLING) -Value (($lines -join "`n") + "`n") -Encoding utf8 -NoNewline
}

# sampling.txt の reason=（無ければ $null・行が読めなければ probe_failed）。可否は dump.txt の有無で決まるので読まない
function Get-MeasureSamplingReason {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    $path = Join-Path $RunDir $MEASURE_FILE_SAMPLING
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $null }
    $reason = 'probe_failed'
    foreach ($line in (Get-Content -LiteralPath $path -Encoding utf8)) {
        if ($line -match '^reason=(\S+)\s*$') { $reason = $Matches[1] }
    }
    return $reason
}

# 4 段の順位表（perf-rank.py）。dump.txt が無ければ段③を UNAVAILABLE として続ける
# （能力不足は停止の理由にしない＝design C5 の 5 の扱い）。
function Invoke-MeasureRank {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    if ($script:DryRunMode -and -not (Test-Path -LiteralPath (Join-Path $RunDir $MEASURE_FILE_RUN_LOG) -PathType Leaf)) {
        Write-Info '[perf-loop] -DryRun: no run artifacts, so no rank table.'
        return $PERF_LOOP_EMPTY
    }
    $outPath = Join-Path $RunDir $MEASURE_FILE_RANK
    $arguments = @((Join-Path $PSScriptRoot 'perf-rank.py'), $RunDir, '--out', $outPath)
    if (Test-Path -LiteralPath (Join-Path $RunDir $MEASURE_FILE_DUMP) -PathType Leaf) {
        $arguments += @('--sampling-available', 'auto')
    } else {
        $reason = Get-MeasureSamplingReason -RunDir $RunDir
        if (-not $reason) { $reason = 'no_record' }   # 24: 可否の試しは回さない（今の機械の可否は過去の走行の理由ではない）
        $arguments += @('--sampling-available', 'false', '--unavailable-reason', $reason)
        Write-Info "[perf-loop] building the rank table with stage 3 (per-function attribution) as UNAVAILABLE reason=$reason (continuing with stages 1, 2, 4)."
    }
    $result = Invoke-PythonChild -Python (Get-MeasurePython) -Arguments $arguments
    if ($result.Code -ne 0) {
        Stop-Run -Code (Convert-MeasureChildExit -Code $result.Code) `
            -Message "could not build the rank table (perf-rank.py exit code $($result.Code)): $($result.LastLine)"
    }
    Write-Info "[perf-loop] rank table: $outPath"
    return $outPath
}


# =============================================================================
# measure-baseline — 25 分 × 1 本（-Build release|dev）
# =============================================================================
# ベースラインは release 25 分・dev 25 分・順位付け 7 分を**別コマンド・別ターン**で
# 回す（1 コマンド 60 分にまとめると check-in が割り込む＝要件 1.11・design C5 Risks）。
function Invoke-SubMeasureBaseline {
    $build = $script:BuildKind
    $dir = Resolve-RunDir -RunDir $script:RunDirArg -Kind 'baseline' -Date $script:DateStamp -Leaf $build
    $script:CurrentDir = $dir
    $longProfile = Get-MeasureToml -Section 'levels' -Key 'long_profile' -Default $MEASURE_LONG_PROFILE_DEFAULT

    Write-Info "[perf-loop] measure-baseline goal $($script:GoalName) / build $build / level $longProfile / out $dir"

    if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $dir)) {
        Write-Info "[perf-loop] -Resume: using the existing run: $dir"
    } else {
        # 11: ビルドを先に回してから実行体を確かめる（走行の本体はビルドを呼ばない）
        Assert-MeasureRunDirFree -Dir $dir
        Sync-MeasureWorktreeBuild -Build $build -BinDir $script:BinDirArg
        if (-not (Test-MeasureExeReady -BinDir $script:BinDirArg -Build $build)) {
            Write-MeasureDryRunMark -Dir $dir -Reason "no executable, so the run was skipped (-DryRun)"
            Exit-WithResult -Code $EXIT_OK -Dir $dir
        }
        $null = Invoke-MeasureRun -OutDir $dir -RunProfile $longProfile -Build $build -BinDir $script:BinDirArg -Label "baseline ($build)"
        Invoke-MeasureQuietAfter -RunDir $dir
    }

    $verdict = Invoke-MeasureJudge -RunDir $dir -Build $build
    Write-Info "[perf-loop] baseline build=$build verdict=$verdict dir=$dir"
    Exit-WithResult -Code $EXIT_OK -Dir $dir
}

# =============================================================================
# rank-run — 順位付けの走行（7 分・点灯・サンプリング）→ 順位表
# =============================================================================
# 点灯した走行は観測行とサンプリングが CPU を押し上げるので**合否には使わない**
# （Flow 2）。段③が使えない機械では採取を省き、段①②④で順位表を作る。
# 順位付けの走行の出力先（rank-run と rank が共に使う）: -RunDir → -Date なら baseline-<date>\rank → iter-<n>\rank
function Get-MeasureRankDir {
    if ($script:RunDirArg) { return (Resolve-RunDir -RunDir $script:RunDirArg -Kind 'iter') }
    if ($script:DateStamp) { return (Get-LoopDir -Kind 'baseline' -Date $script:DateStamp -Leaf 'rank') }
    Assert-MeasureIter
    return (Get-LoopDir -Kind 'iter' -Iter $script:IterArg -Leaf 'rank')
}

function Invoke-SubRankRun {
    $build = $script:BuildKind
    $dir = Get-MeasureRankDir
    $script:CurrentDir = $dir
    $shortProfile = Get-MeasureToml -Section 'levels' -Key 'short_profile' -Default $MEASURE_SHORT_PROFILE_DEFAULT

    Write-Info "[perf-loop] rank-run goal $($script:GoalName) / build $build / level $shortProfile / out $dir"

    if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $dir)) {
        Write-Info "[perf-loop] -Resume: using the existing run: $dir"
        Invoke-MeasureRank -RunDir $dir | Out-Null
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }

    Assert-MeasureRunDirFree -Dir $dir
    Sync-MeasureWorktreeBuild -Build $build -BinDir $script:BinDirArg
    if (-not (Test-MeasureExeReady -BinDir $script:BinDirArg -Build $build)) {
        Write-MeasureDryRunMark -Dir $dir -Reason "no executable, so the run was skipped (-DryRun)"
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }

    # --- 段③（サンプリング）の可否を先に確かめる -----------------------------
    # 採取を始めるのはランナー（静かさの確かめの後・areka を起こす直前）。ここは -Start を呼ばない（1・9）
    $samplePath = Join-Path $PSScriptRoot 'invoke-cpu-sample.ps1'
    $etlPath = Join-Path (Split-Path -Parent $dir) ((Split-Path -Leaf $dir) + $MEASURE_TRACE_SUFFIX)
    $sampleEtl = ''
    $reason = 'dry_run'
    if ($script:DryRunMode) {
        Write-Info '[perf-loop] -DryRun: no stage 3 (sampling).'
    } else {
        $probe = Get-SamplingProbe
        $reason = $probe.Reason
        if ($probe.Available) {
            New-LoopDir -Path (Split-Path -Parent $etlPath) | Out-Null
            $sampleEtl = $etlPath
        } else {
            Write-Info "[perf-loop] stage 3 unavailable: reason=$reason. Skipping the capture and continuing with stages 1, 2, 4 (not a reason to stop)."
        }
    }
    $pdbDir = Get-MeasureBinDir -BinDir $script:BinDirArg -Build $build

    # --- 順位付けの走行（点灯・可否が真ならランナーが採取を始める） -----------
    $run = Invoke-MeasureRun -OutDir $dir -RunProfile $shortProfile -Build $build -BinDir $script:BinDirArg `
        -RustLogExtra $MEASURE_RANK_RUSTLOG_EXTRA -SampleEtl $sampleEtl -Label 'rank run'

    # --- 段③の停止と記号解決（採取が動いたまま返ったら次の 1 手で必ず止める） ---
    $sampling = $false
    if ($run.SamplingStarted) {
        $stopArgs = @('-Stop', '-Etl', $etlPath, '-Out', (Join-Path $dir $MEASURE_FILE_DUMP), '-PdbDir', $pdbDir)
        $stop = Invoke-PwshChild -ScriptPath $samplePath -Arguments $stopArgs
        if ($stop.Code -eq 0) {
            $sampling = $true
            $reason = 'ok'
        } elseif ($stop.Code -eq $EXIT_UNAVAILABLE) {
            $reason = Get-MeasureLineWord -Lines $stop.Lines -Pattern '^INVOKE-CPU-SAMPLE RESULT stop code=\S+ .*\breason=(\S+)'
            if (-not $reason) { $reason = 'no_xperf' }
            Write-Problem "[perf-loop] stage 3 symbol resolution not possible, capability missing (reason=$reason). Continuing with stages 1, 2, 4."
        } elseif ($stop.Code -eq $EXIT_BAD_ARGS) {
            # 3 は採取を止めた後の前提の不備（-PdbDir・記号の置き場）。そのまま 3（直して -Stop を打ち直せばまとめた物を読む）
            Stop-Run -Code $EXIT_BAD_ARGS -Message "stage 3 symbol resolution stopped on bad arguments or preconditions: $($stop.LastLine)"
        } elseif ($stop.Code -eq -1) {
            Write-Problem "[perf-loop] warning: the kernel CPU sampling may still be running. Run 'xperf -stop' in an elevated PowerShell by hand."
            Stop-Run -Code $EXIT_MEASURE_FAILED -Message "could not start invoke-cpu-sample.ps1 -Stop: $($stop.LastLine)"
        } else {
            # -Stop は -d が落ちても自分で -stop を打つが、それより前に落ちたなら採取が残っていることがある
            Write-Problem "[perf-loop] warning: if 'xperf -loggers' still lists NT Kernel Logger and no other tool is sampling, run 'xperf -stop' in an elevated PowerShell by hand."
            Stop-Run -Code $EXIT_MEASURE_FAILED `
                -Message "stage 3 symbol resolution failed (invoke-cpu-sample.ps1 exit code $($stop.Code)): $($stop.LastLine)"
        }
    } elseif ($sampleEtl) {
        $reason = $run.SamplingReason
        Write-Problem "[perf-loop] the runner could not start the stage 3 capture (reason=$reason). Continuing with stages 1, 2, 4."
    }
    Write-MeasureSamplingNote -RunDir $dir -Available $sampling -Reason $reason
    Assert-MeasureShioriHelper -RunDir $dir -Label 'rank run' | Out-Null

    Invoke-MeasureQuietAfter -RunDir $dir
    Invoke-MeasureRank -RunDir $dir | Out-Null
    Exit-WithResult -Code $EXIT_OK -Dir $dir
}

# =============================================================================
# rank — 既にある走行から順位表だけを作り直す
# =============================================================================
function Invoke-SubRank {
    $dir = Get-MeasureRankDir
    $script:CurrentDir = $dir
    Write-Info "[perf-loop] rank run $dir"
    if (-not (Test-Path -LiteralPath $dir -PathType Container)) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "run output folder not found: $dir (check -RunDir or -Iter)"
    }
    $rankPath = Join-Path $dir $MEASURE_FILE_RANK
    if (Test-ResumeArtifact -Path $rankPath -Resume:$script:ResumeMode) {
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }
    if ($script:DryRunMode -and -not (Test-Path -LiteralPath (Join-Path $dir $MEASURE_FILE_RUN_LOG) -PathType Leaf)) {
        Write-Info '[perf-loop] -DryRun: no run artifacts, so no rank table.'
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }
    Invoke-MeasureRank -RunDir $dir | Out-Null
    Exit-WithResult -Code $EXIT_OK -Dir $dir
}

# =============================================================================
# prepare-ab — A 側（変更前）の実行体一式を bin-A へ
# =============================================================================
# **変更を入れる前**に回すこと。ここで固めた実行体が A（変更前）になる。
function Invoke-SubPrepareAb {
    Assert-MeasureIter
    $binA = Get-LoopDir -Kind 'iter' -Iter $script:IterArg -Leaf $MEASURE_SIDE_LEAF['A']
    $script:CurrentDir = $binA
    Write-Info "[perf-loop] prepare-ab iter $($script:IterArg) / A side $binA"
    Invoke-MeasureBuildSide -Side 'A' -BinDir $binA | Out-Null
    Exit-WithResult -Code $EXIT_OK -Dir $binA
}

# =============================================================================
# measure-ab — B をビルドして A1 B1 A2 B2 を交互に採り、compare まで
# =============================================================================
# ばらつき＝|A1−A2|（同一形を 2 回）。差＝mean(B)−mean(A)（Flow 2・要件 1.7）。
# 交互に採るのは、機械の状態がゆっくり変わっても差に片寄りが乗らないようにするため。
function Invoke-SubMeasureAb {
    Assert-MeasureIter
    $iterDir = Get-LoopDir -Kind 'iter' -Iter $script:IterArg
    $script:CurrentDir = $iterDir
    $binA = Join-Path $iterDir $MEASURE_SIDE_LEAF['A']
    $binB = Join-Path $iterDir $MEASURE_SIDE_LEAF['B']
    $shortProfile = Get-MeasureToml -Section 'levels' -Key 'short_profile' -Default $MEASURE_SHORT_PROFILE_DEFAULT
    $abBuild = Get-MeasureAbBuild
    $sequence = Get-MeasureTomlArray -Section 'levels' -Key 'ab_sequence' -Default $MEASURE_AB_SEQUENCE_DEFAULT
    Write-Info "[perf-loop] measure-ab iter $($script:IterArg) / level $shortProfile / build $abBuild / order $($sequence -join ' ')"

    # 走行の名前（A→A1, B→B1, A→A2, B→B2）。名前は出力先の語彙に無ければ止める。B を作る前に確かめる。
    $counts = @{ 'A' = 0; 'B' = 0 }
    $runs = @()
    foreach ($side in $sequence) {
        $key = $side.ToUpperInvariant()
        if (-not $counts.ContainsKey($key)) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [levels] ab_sequence has a value other than A/B: '$side'"
        }
        $counts[$key] = $counts[$key] + 1
        $name = "$key$($counts[$key])"
        if ($PERF_LOOP_ITER_LEAVES -notcontains $name) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "run name is not in the output folder vocabulary: '$name' (allowed: $($PERF_LOOP_ITER_LEAVES -join ', '))"
        }
        $runs += [pscustomobject]@{ Name = $name; Side = $key; Dir = (Join-Path $iterDir $name) }
    }
    if ($counts['A'] -ne 2 -or $counts['B'] -ne 2) {
        # 34: 4 本の突き合わせは A と B が 2 本ずつのときだけ成り立つ
        Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [levels] ab_sequence must have A and B exactly twice each (got: $($sequence -join ' '))"
    }

    if (-not (Test-MeasureBinDirComplete -Dir $binA) -and -not $script:DryRunMode) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "A side executable set not found: $binA (run prepare-ab -Iter $($script:IterArg) before making the change)"
    }

    # 走る出力先が空いていることを B を作る前に確かめる（-Resume で使う済みの走行は除く）
    foreach ($run in $runs) {
        if (-not ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $run.Dir))) { Assert-MeasureRunDirFree -Dir $run.Dir }
    }

    # B 側（＝いまのツリー）を作り、A と同じ実行体なら走る前に止める（-DryRun では比べない・15）
    Invoke-MeasureBuildSide -Side 'B' -BinDir $binB | Out-Null
    if (-not $script:DryRunMode) { Assert-MeasureAbDistinct -BinA $binA -BinB $binB }

    foreach ($run in $runs) {
        if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $run.Dir)) {
            Write-Info "[perf-loop] -Resume: using the existing run: $($run.Dir)"
            continue
        }
        $binDir = if ($run.Side -eq 'A') { $binA } else { $binB }
        if (-not (Test-MeasureExeReady -BinDir $binDir -Build $abBuild)) {
            Write-MeasureDryRunMark -Dir $run.Dir -Reason "no $($run.Side) side executable, so the run was skipped (-DryRun)"
            continue
        }
        $null = Invoke-MeasureRun -OutDir $run.Dir -RunProfile $shortProfile -Build $abBuild -BinDir $binDir -Label "run $($run.Name)"
        Assert-MeasureShioriHelper -RunDir $run.Dir -Label "run $($run.Name)" | Out-Null
        Invoke-MeasureQuietAfter -RunDir $run.Dir
    }
    $script:CurrentDir = $iterDir

    Invoke-MeasureCompareCore -IterDir $iterDir
    Exit-WithResult -Code $EXIT_OK -Dir $iterDir
}

# =============================================================================
# compare — A B A B の 4 本から差とばらつきを出して採否を返す
# =============================================================================
# 目標定義 [levels] iteration_build を小文字にそろえて読む（判定へ渡る --build は dev／release だけを字のまま受ける）。
# A/B のビルドは release 固定（判定式⑷a が release の式＝要件 5.3）。別の値なら走る前に 3 で止める（measure-ab・compare とも）
function Get-MeasureAbBuild {
    $abBuild = (Get-MeasureToml -Section 'levels' -Key 'iteration_build' -Default $MEASURE_ITER_BUILD_DEFAULT).ToLowerInvariant()
    if ($abBuild -ne 'release') {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [levels] iteration_build accepts only release (got: '$abBuild'). A/B executables are built as release (formula (4)a is the release formula, requirement 5.3)."
    }
    return $abBuild
}

function Invoke-MeasureCompareCore {
    param([Parameter(Mandatory = $true)][string]$IterDir)
    $a1 = Join-Path $IterDir 'A1'
    $a2 = Join-Path $IterDir 'A2'
    $b1 = Join-Path $IterDir 'B1'
    $b2 = Join-Path $IterDir 'B2'
    foreach ($dir in @($a1, $a2, $b1, $b2)) {
        if (Test-Path -LiteralPath (Join-Path $dir $MEASURE_FILE_RUN_LOG) -PathType Leaf) { continue }
        if ($script:DryRunMode) {
            Write-Info "[perf-loop] -DryRun: no run artifacts, so no accept/reject (perf-compare.py) ($dir)."
            return
        }
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message "run artifacts not found: $dir\$MEASURE_FILE_RUN_LOG"
    }
    $abBuild = Get-MeasureAbBuild
    $arguments = @(
        (Join-Path $PSScriptRoot 'perf-compare.py'),
        '--a', $a1, $a2,
        '--b', $b1, $b2,
        '--build', $abBuild,
        '--repo-root', $repoRoot,
        '--out-dir', $IterDir
    ) + (Get-MeasureGoalArgsPython)
    $result = Invoke-PythonChild -Python (Get-MeasurePython) -Arguments $arguments
    if ($result.Code -ne 0) {
        Stop-Run -Code (Convert-MeasureChildExit -Code $result.Code) `
            -Message "could not decide accept/reject (perf-compare.py exit code $($result.Code)): $($result.LastLine)" -Dir $IterDir
    }
    Write-Info "[perf-loop] accept/reject: $IterDir\$MEASURE_FILE_COMPARE"
}

function Invoke-SubCompare {
    Assert-MeasureIter
    $iterDir = Get-LoopDir -Kind 'iter' -Iter $script:IterArg
    $script:CurrentDir = $iterDir
    Write-Info "[perf-loop] compare iter $($script:IterArg) / $iterDir"
    # 周の作業場そのものが無いのは -Iter の打ち間違い（計測の失敗の 4 にしない）
    if (-not (Test-Path -LiteralPath $iterDir -PathType Container)) { Stop-Run -Code $EXIT_BAD_ARGS -Message "iteration folder not found: $iterDir (check -Iter)" }
    Invoke-MeasureCompareCore -IterDir $iterDir
    Exit-WithResult -Code $EXIT_OK -Dir $iterDir
}

# =============================================================================
# followup — 見た目の追随 4 検査（design C13）
# =============================================================================
# 判定語（PASS／FAIL／INCONCLUSIVE）は**採否を決める側＝周の手順スキル**が読む。
# ここが終了コードで区別するのは「判定まで届いたか」だけである。
#
# **中止と判定を取り違えないこと**: invoke-followup-checks.ps1 は中止のときにも
#   FOLLOWUP RESULT overall=INCONCLUSIVE clickthrough=- drag=- dpi=- balloon_follow=-
# の 1 行を出す（invoke-followup-checks.ps1 の Stop-Run）。行が在ることだけを
# 見て 0 を返すと、**起動できなかった走行が「判定できた」ことになる**。
# 判定まで届いた印は次の 2 つが揃うこと:
#   ⒜ judge-followup.py が書く followup-verdict.txt が在る
#   ⒝ 報告行の 4 検査が全て `-` ではない（＝中止の形ではない）
# 対応:
#   子が 3      … 引数・前提の不正 → そのまま 3
#   ⒜⒝が揃う   … 判定語を印字して 0（PASS／FAIL／INCONCLUSIVE のどれでも 0）
#   それ以外    … 実走の失敗 → 1 度だけ回し直し、なお駄目なら 4
# -Resume の完了印も同じ 2 つ（probe.log と followup-verdict.txt）。followup.txt は
# 中止のときにも書かれるので、単独では完了の印にならない。
function Test-MeasureFollowupComplete {
    param([Parameter(Mandatory = $true)][string]$Dir)
    if (-not (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_PROBE_LOG) -PathType Leaf)) { return $false }
    return (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_FOLLOWUP_VERDICT) -PathType Leaf)
}

# 判定まで届いたか（上の ⒜⒝）。RESULT 行は子の全部の行の最後のもの（理由の行は標準エラーへ出る）。
# 戻り: Reached・ResultLine・Why（届かなかった理由の語・届いたら空）
function Test-FollowupReachedVerdict {
    param([string[]]$Lines = @(), [Parameter(Mandatory = $true)][string]$Dir)
    $resultLine = $null
    foreach ($line in $Lines) { if ($line -match '^FOLLOWUP RESULT ') { $resultLine = $line.Trim() } }
    $judged = Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_FOLLOWUP_VERDICT) -PathType Leaf
    $abortShape = ($resultLine -and ($resultLine -match 'clickthrough=-\s+drag=-\s+dpi=-\s+balloon_follow=-'))
    $why = if (-not $resultLine) { 'no result line' }
           elseif ($abortShape)  { 'the result line has the abort shape (all 4 checks are -)' }
           elseif (-not $judged) { "no verdict output ($MEASURE_FILE_FOLLOWUP_VERDICT)" }
           else                  { '' }
    return [pscustomobject]@{ Reached = ($why -eq ''); ResultLine = $resultLine; Why = $why }
}

# 順（13）: -Resume の済み → 目標定義の検査の集合 → -DryRun → 実行体（-BinDir／bin-B／無ければビルドして
# target\release）→ 検体の在り処 → 検査。済んだ -Resume と -DryRun は外を何も呼ばない。
function Invoke-SubFollowup {
    Assert-MeasureIter
    $iterDir = Get-LoopDir -Kind 'iter' -Iter $script:IterArg
    $dir = Join-Path $iterDir 'followup'
    $script:CurrentDir = $dir
    if (Test-MeasureFollowupComplete -Dir $dir) {
        if ($script:ResumeMode) {
            Write-Info "[perf-loop] -Resume: using the existing followup verdict: $dir"
            Exit-WithResult -Code $EXIT_OK -Dir $dir
        }
        # 済んだ判定を黙って消さない（ほかの走行の出力先と同じ決まり）
        Stop-Run -Code $EXIT_BAD_ARGS -Dir $dir -Message "output folder already has a followup verdict: $dir (to check again, delete it; to use it, add -Resume)"
    }
    $exitMs = Get-MeasureTomlInt -Section 'followup' -Key 'exit_ms' -Default $MEASURE_FOLLOWUP_EXIT_MS_DEFAULT
    # 必須の検査は目標定義 [followup] required が唯一の所在（要件 1.1）。ここで読んで
    # -Checks へ渡さないと、実走の集合は checker の既定（4 検査）のまま動かない
    # ——DPI の違うモニタが 1 面しか無い機械で dpi を外せず、判定不能のまま 1 周も
    # 採用できなくなる（README §17）。checker はこの集合を probe.log の
    # `check=session step=begin required=` へ書き、judge-followup.py はその行だけを
    # 「どれを必須とするか」の定義元として読むので、外した検査は総合判定に効かない。
    $checks = @(Get-MeasureTomlArray -Section 'followup' -Key 'required' -Default $MEASURE_FOLLOWUP_CHECKS_ALL)
    $unknown = @($checks | Where-Object { $MEASURE_FOLLOWUP_CHECKS_ALL -notcontains $_ })
    if ($unknown.Count -gt 0) {
        Stop-Run -Code $EXIT_BAD_ARGS -Dir $dir -Message ("goal [followup] required has unknown check names: " +
            "$($unknown -join ', ') (allowed: $($MEASURE_FOLLOWUP_CHECKS_ALL -join ', '))")
    }
    # 並びは固定語彙の順に揃える（目標定義の書き順で報告の並びが揺れないように）。
    $checksArg = (@($MEASURE_FOLLOWUP_CHECKS_ALL | Where-Object { $checks -contains $_ }) -join ',')
    $candidate = Join-Path $iterDir $MEASURE_SIDE_LEAF['B']
    # bin-B は揃った一式（BUILD.txt が在り、-DryRun の写しでない）のときだけ使う
    $useTarget = (-not $script:BinDirArg) -and -not (Test-MeasureBinDirComplete -Dir $candidate)
    $binDir = if ($script:BinDirArg) { [System.IO.Path]::GetFullPath($script:BinDirArg).TrimEnd('\') }
              elseif ($useTarget)    { Get-MeasureBinDir -Build 'release' }
              else                   { $candidate }
    # 渡された -BinDir に実行体が無ければ検査の子を起こさずに 3（打つべきビルドを示す。-DryRun では止めない）
    if ($script:BinDirArg) { $null = Test-MeasureExeReady -BinDir $binDir -Build 'release' }
    $arguments = @('-OutDir', $dir, '-BinDir', $binDir, '-ExitMs', "$exitMs", '-Checks', $checksArg)
    Write-Info "[perf-loop] followup iter $($script:IterArg) / exe $binDir / checks $checksArg / limit $exitMs ms / out $dir"
    if ($script:DryRunMode) {
        Write-MeasureDryRunMark -Dir $dir -Reason ('followup checks are not run under -DryRun (they need a real run and input injection). ' +
            "arguments to pass: invoke-followup-checks.ps1 $($arguments -join ' ') -GhostRoot <sample> -BalloonRoot <sample>")
        Write-Info '[perf-loop] -DryRun: followup checks were not run.'
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }
    if ($useTarget) { Sync-MeasureWorktreeBuild -Build 'release' }
    # 検査の子はヘルパを写さないので、実 SHIORI を伴うようここで隣へそろえる（走行の Invoke-MeasureRun と同じ口）
    Sync-MeasureShioriHelper -BinDir $(if ($useTarget) { '' } else { $binDir }) -Build 'release'
    $arguments += @('-GhostRoot', (Get-MeasureGhostRoot), '-BalloonRoot', (Get-MeasureBalloonRoot))
    $checkerPath = Join-Path $PSScriptRoot 'invoke-followup-checks.ps1'
    Write-Info "[perf-loop] followup checker call: invoke-followup-checks.ps1 $($arguments -join ' ')"
    $attempts = $MEASURE_RUN_RETRY + 1
    for ($attempt = 1; $attempt -le $attempts; $attempt++) {
        # 前の回の置き土産（中止したときの followup.txt・古い判定・-DryRun の印）を残さない。
        # 残すと「今回の判定」と取り違える。
        foreach ($stale in @($MEASURE_FILE_FOLLOWUP, $MEASURE_FILE_FOLLOWUP_VERDICT, $MEASURE_FILE_DRYRUN)) {
            $stalePath = Join-Path $dir $stale
            if (Test-Path -LiteralPath $stalePath -PathType Leaf) { Remove-Item -LiteralPath $stalePath -Force }
        }
        Write-Info "[perf-loop] running the followup checks ($attempt/$attempts)"
        $result = Invoke-PwshChild -ScriptPath $checkerPath -Arguments $arguments
        if ($result.Code -eq 3) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "followup checks stopped on bad arguments or preconditions: $($result.LastLine)" -Dir $dir
        }
        $verdict = Test-FollowupReachedVerdict -Lines $result.Lines -Dir $dir
        if ($verdict.Reached) {
            Write-Info "[perf-loop] $($verdict.ResultLine)"
            Exit-WithResult -Code $EXIT_OK -Dir $dir
        }
        Write-Problem "[perf-loop] followup checks did not reach a verdict (exit code $($result.Code), $($verdict.Why)): $($result.LastLine)"
        if ($attempt -lt $attempts) { Write-Info '[perf-loop] running once more.' }
    }
    Stop-Run -Code $EXIT_MEASURE_FAILED -Message "followup checks did not reach a verdict in all $attempts tries (MEASURE_FAILED)" -Dir $dir
}

# =============================================================================
# final — 25 分 × release／dev → judge-perf.py --mode verdict
# =============================================================================
# -Build を明示すると片方だけを採る。**2 本を別ターンで回すための形**であり
# （25 分 × 2 は check-in を跨ぐ＝要件 1.11）、2 本目は -Resume を付けて回すと
# 済んだ方を採り直さない。判定の出力は spec の results へも写す（要件 5.6）。
function Invoke-SubFinal {
    # 10: 1 つの実行体を release と dev の 2 つのビルドとして測らない
    if ($script:BinDirArg -and -not $script:BuildExplicit) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message 'final with -BinDir needs -Build (one executable cannot be measured as both builds)'
    }
    $dir = Resolve-RunDir -RunDir $script:RunDirArg -Kind 'final' -Date $script:DateStamp
    $script:CurrentDir = $dir
    $longProfile = Get-MeasureToml -Section 'levels' -Key 'long_profile' -Default $MEASURE_LONG_PROFILE_DEFAULT
    $builds = if ($script:BuildExplicit) {
        @($script:BuildKind)
    } else {
        # 4: 目標定義の語も小文字にそろえる（判定へ渡る --build は dev／release だけを受ける）
        @(Get-MeasureTomlArray -Section 'target' -Key 'builds_final' -Default $MEASURE_BUILDS_FINAL_DEFAULT | ForEach-Object { $_.ToLowerInvariant() })
    }
    Write-Info "[perf-loop] final goal $($script:GoalName) / level $longProfile / build $($builds -join ', ') / out $dir"

    # 語は 1 本目を測る前に全部確かめる（2 本目の打ち間違いで 1 本目の 25 分を捨てない）
    foreach ($build in $builds) {
        if ($BUILD_KINDS -notcontains $build) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [target] builds_final has a value other than dev/release: '$build'" -Dir $dir
        }
    }
    if (@($builds | Sort-Object -Unique).Count -ne @($builds).Count) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "goal [target] builds_final names a build twice: $($builds -join ', ')" -Dir $dir
    }
    # どのビルドの出力先も空いていることを 1 本目を測る前に確かめる（-Resume で使う済みの走行は除く）
    foreach ($build in $builds) {
        $buildDir = Join-Path $dir $build
        if (-not ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $buildDir))) { Assert-MeasureRunDirFree -Dir $buildDir }
    }

    $summary = @()
    foreach ($build in $builds) {
        $buildDir = Join-Path $dir $build
        if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $buildDir)) {
            Write-Info "[perf-loop] -Resume: using the existing run: $buildDir"
        } else {
            Assert-MeasureRunDirFree -Dir $buildDir
            Sync-MeasureWorktreeBuild -Build $build -BinDir $script:BinDirArg
            if (-not (Test-MeasureExeReady -BinDir $script:BinDirArg -Build $build)) {
                Write-MeasureDryRunMark -Dir $buildDir -Reason "no executable, so the run was skipped (-DryRun)"
                $summary += "final build=$build verdict=$PERF_LOOP_EMPTY dir=$buildDir"
                continue
            }
            $null = Invoke-MeasureRun -OutDir $buildDir -RunProfile $longProfile -Build $build -BinDir $script:BinDirArg -Label "final verdict ($build)"
            Invoke-MeasureQuietAfter -RunDir $buildDir
        }
        $verdict = Invoke-MeasureJudge -RunDir $buildDir -Build $build
        Copy-MeasureVerdictToResults -RunDir $buildDir -Build $build
        $summary += "final build=$build verdict=$verdict dir=$buildDir"
    }
    $script:CurrentDir = $dir
    foreach ($line in $summary) { Write-Info "[perf-loop] $line" }
    Exit-WithResult -Code $EXIT_OK -Dir $dir
}

# 判定の出力を spec の results/final-<date>/<build>/ へも写す（要件 5.6）。
function Copy-MeasureVerdictToResults {
    param([Parameter(Mandatory = $true)][string]$RunDir, [Parameter(Mandatory = $true)][string]$Build)
    if (-not $script:GoalConfig) { return }
    if (-not $script:GoalConfig.SpecDir -or -not $script:GoalConfig.ResultsDir) { return }
    $source = Join-Path $RunDir $MEASURE_FILE_VERDICT
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { return }
    # 写し先の日付は走行の出力先（final-<date>\<build>）の日付。-RunDir で前の final を読み直しても今日の日付にしない
    $finalLeaf = Split-Path -Leaf (Split-Path -Parent $RunDir)
    $stamp = if ($finalLeaf -match '^final-(\d{8})$') { $Matches[1] } else { Get-LoopDateStamp }
    $target = Join-Path (Join-Path (Join-Path $repoRoot $script:GoalConfig.SpecDir) $script:GoalConfig.ResultsDir) `
        (Join-Path "final-$stamp" $Build)
    New-LoopDir -Path $target | Out-Null
    Copy-Item -LiteralPath $source -Destination (Join-Path $target $MEASURE_FILE_VERDICT) -Force
    foreach ($name in @($MEASURE_FILE_RUN_META, $MEASURE_FILE_QUIET_AFTER)) {
        $extra = Join-Path $RunDir $name
        if (Test-Path -LiteralPath $extra -PathType Leaf) { Copy-Item -LiteralPath $extra -Destination (Join-Path $target $name) -Force }
    }
    Write-Info "[perf-loop] copied the verdict output to the spec: $target"
}
