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
    `<leaf>-FAILED` へ退避するので、1 度だけの採り直しは同じ出力先で行える。
 ⒝ 静寂の確認は前後 2 回。前（quiet-before.txt）はランナーの `-AutoQuiet` が書く。
    後（quiet-after.txt）は**ここの仕事**で、check-quiet.ps1 -Stage after を呼ぶ。
    「静かでない」なら目標定義 [quiet] の retry_max 回だけ待って確かめ直し、
    それでも静かでなければ 2（静寂でない）で終わる（要件 2.8・design C6）。
 ⒞ 実走の失敗は 1 度だけ採り直し、なお失敗なら 4（計測失敗＝MEASURE_FAILED）。
    引数・前提の不正は 3、静寂でないは 2 をそのまま返す（tasks.md 7.3）。
 ⒟ **絶対値の合否を出す走行（measure-baseline／final）は実 SHIORI を伴うこと。**
    32bit ヘルパが無いと発話が起きず、アイドルの絶対値は別条件の値になる。
    採取ランナーは警告だけで走ってしまう（invoke-perf-run.ps1:709-710）ので、
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
  MEASURE_GHOST_ROOT_RELPATH／MEASURE_BALLOON_SUBDIR  -GhostRoot／-BalloonRoot 省略時
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
$MEASURE_GHOST_ROOT_RELPATH    = 'crates\pilot\examples\shiori-host-32\fixtures\emo2'
$MEASURE_BALLOON_SUBDIR        = 'emo2-kakukaku'
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
function Get-MeasureGhostRoot {
    $path = if ($script:GhostRootArg) { $script:GhostRootArg } else { Join-Path $repoRoot $MEASURE_GHOST_ROOT_RELPATH }
    return [System.IO.Path]::GetFullPath($path).TrimEnd('\')
}

function Get-MeasureBalloonRoot {
    $path = if ($script:BalloonRootArg) { $script:BalloonRootArg } else { Join-Path (Get-MeasureGhostRoot) $MEASURE_BALLOON_SUBDIR }
    return [System.IO.Path]::GetFullPath($path).TrimEnd('\')
}

# 実行体の在り処（-BinDir が最優先・無ければ target\<release|debug>）。
function Get-MeasureBinDir {
    param([string]$BinDir, [Parameter(Mandatory = $true)][string]$Build)
    if ($BinDir) { return [System.IO.Path]::GetFullPath($BinDir).TrimEnd('\') }
    $sub = if ($Build -eq 'release') { 'release' } else { 'debug' }
    return [System.IO.Path]::GetFullPath((Join-Path $repoRoot (Join-Path 'target' $sub))).TrimEnd('\')
}

# 実行体が在るか。無いときは何を打てばよいかを必ず示す（黙って落とさない）。
# -DryRun のときだけは止めずに $false を返す（配管の確認は実行体が無くてもできる）。
function Test-MeasureExeReady {
    param([string]$BinDir, [Parameter(Mandatory = $true)][string]$Build)
    $dir = Get-MeasureBinDir -BinDir $BinDir -Build $Build
    $exe = Join-Path $dir $MEASURE_AREKA_EXE
    if (Test-Path -LiteralPath $exe -PathType Leaf) { return $true }
    $releaseDebug = Get-MeasureToml -Section 'levels' -Key 'release_debug_env' -Default $MEASURE_RELEASE_DEBUG_DEFAULT
    $hint = if ($Build -eq 'release') {
        "PowerShell から CARGO_PROFILE_RELEASE_DEBUG=$releaseDebug を環境変数に置いて cargo build --release -p $MEASURE_AREKA_PACKAGE を先に回してください（記号が無いと段③が使えません）"
    } else {
        "cargo build -p $MEASURE_AREKA_PACKAGE を先に回してください"
    }
    if ($script:DryRunMode) {
        Write-Info "[perf-loop] -DryRun: 実行体がありません（$exe）。この走行は省きます。$hint"
        return $false
    }
    Stop-Run -Code $EXIT_BAD_ARGS -Message "実行体がありません: $exe。$hint"
}

function Assert-MeasureIter {
    if ($script:IterArg -lt 1) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "-Iter は 1 以上が要ります（受け取った値: $($script:IterArg)）。周の出力先は iter-<n> です。"
    }
}

function Get-MeasurePython {
    $python = Get-PythonCommand
    if (-not $python) {
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message 'python が見つかりません（順位表・比較・合否判定は python の道具が行います）'
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

# 走行の成果物が揃っているか（-Resume の完了印）。
function Test-MeasureRunComplete {
    param([Parameter(Mandatory = $true)][string]$Dir)
    foreach ($name in @($MEASURE_FILE_RUN_LOG, $MEASURE_FILE_CPU_CSV, $MEASURE_FILE_RUN_META)) {
        if (-not (Test-Path -LiteralPath (Join-Path $Dir $name) -PathType Leaf)) { return $false }
    }
    return (Test-MeasureQuietAfterOk -Dir $Dir)
}

# -DryRun の置き土産（DRY-RUN.txt があって run.log が無い出力先）だけを作り直す。
# 実走の成果物がある出力先には触らない——測ったものを黙って消さないため。
function Clear-MeasureDryRunDir {
    param([Parameter(Mandatory = $true)][string]$Dir)
    if (-not (Test-Path -LiteralPath $Dir)) { return }
    $isDry  = Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_DRYRUN) -PathType Leaf
    $hasRun = Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_RUN_LOG) -PathType Leaf
    if ($isDry -and -not $hasRun) {
        Write-Info "[perf-loop] 前回の -DryRun の出力先を作り直します: $Dir"
        Remove-Item -LiteralPath $Dir -Recurse -Force
    }
}

# 走行を省いたことを出力先に残す（-DryRun 専用。測定結果と取り違えないための札）。
function Write-MeasureDryRunMark {
    param([Parameter(Mandatory = $true)][string]$Dir, [Parameter(Mandatory = $true)][string]$Reason)
    New-LoopDir -Path $Dir | Out-Null
    $lines = @(
        'これは -DryRun（試験専用）の出力です。測定結果として扱ってはいけません。',
        "理由: $Reason",
        "時刻: $(Get-UtcStamp)"
    )
    Set-Content -LiteralPath (Join-Path $Dir $MEASURE_FILE_DRYRUN) -Value (($lines -join "`n") + "`n") -Encoding utf8 -NoNewline
}

# =============================================================================
# 実走・静寂・合否・順位表（8 本が共有する 4 つの動作）
# =============================================================================

# 実走を 1 本採る。失敗は 1 度だけ採り直し、なお失敗なら 4 で止まる（tasks.md 7.3）。
# 静寂でない（ランナーの 2）は採り直さずそのまま 2 で止まる——待って確かめ直すのは
# ランナー側が目標定義の回数だけ既に行っているためである（design C6・C7）。
function Invoke-MeasureRun {
    param(
        [Parameter(Mandatory = $true)][string]$OutDir,
        [Parameter(Mandatory = $true)][string]$RunProfile,
        [Parameter(Mandatory = $true)][string]$Build,
        [string]$BinDir,
        [string]$RustLogExtra,
        [string]$Label = '走行'
    )
    $runnerPath = Join-Path $PSScriptRoot 'invoke-perf-run.ps1'
    $ghostRoot   = Get-MeasureGhostRoot
    $balloonRoot = Get-MeasureBalloonRoot
    $attempts = $MEASURE_RUN_RETRY + 1
    Sync-MeasureWorktreeBuild -Build $Build -BinDir $BinDir
    if (-not $script:DryRunMode) { Sync-MeasureShioriHelper -BinDir (Get-MeasureBinDir -BinDir $BinDir -Build $Build) }

    for ($attempt = 1; $attempt -le $attempts; $attempt++) {
        Clear-MeasureDryRunDir -Dir $OutDir
        if (Test-Path -LiteralPath $OutDir) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "出力先が既にあります: $OutDir（採取のたびに新しいフォルダを作る決まりです。採り直すなら消すか、既にある成果物を使うなら -Resume を付けてください）"
        }
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

        Write-Info "[perf-loop] $Label を採ります（$attempt/$attempts・水準 $RunProfile・ビルド $Build・出力先 $OutDir）"
        $result = Invoke-PwshChild -ScriptPath $runnerPath -Arguments $arguments
        if ($result.Code -eq 0) { return }
        if ($result.Code -eq 2) {
            Stop-Run -Code $EXIT_NOT_QUIET -Message "$Label：静寂状態の自動確認が通りませんでした（目標定義 [quiet] の回数だけ待って確かめ直した後です）。根拠は退避先 $OutDir-FAILED の $MEASURE_FILE_QUIET_BEFORE に残っています。"
        }
        if ($result.Code -eq 3) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "$Label：採取ランナーが引数・前提の不正で止まりました: $($result.LastLine)"
        }
        Write-Problem "[perf-loop] $Label が失敗しました（終了コード $($result.Code)）: $($result.LastLine)"
        if ($attempt -lt $attempts) { Write-Info "[perf-loop] もう 1 度だけ採り直します。" }
    }
    Stop-Run -Code $EXIT_MEASURE_FAILED -Message "$Label が $attempts 回とも失敗しました（計測失敗＝MEASURE_FAILED）"
}

# 走行の後の静寂確認。「静かでない」なら目標定義の回数だけ待って確かめ直す（design C6）。
function Invoke-MeasureQuietAfter {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    if ($script:DryRunMode) {
        Write-Info '[perf-loop] -DryRun: 走行後の静寂確認は行いません（証跡 quiet-after.txt も作りません）。'
        return
    }
    $checkPath = Join-Path $PSScriptRoot 'check-quiet.ps1'
    $retryMax  = Get-MeasureTomlInt -Section 'quiet' -Key 'retry_max' -Default $MEASURE_QUIET_RETRY_MAX_DEFAULT
    $waitSec   = Get-MeasureTomlInt -Section 'quiet' -Key 'retry_wait_sec' -Default $MEASURE_QUIET_RETRY_WAIT_DEFAULT
    for ($i = 0; $i -le $retryMax; $i++) {
        $arguments = @('-Stage', 'after', '-OutDir', $RunDir) + (Get-MeasureGoalArgsPwsh)
        $result = Invoke-PwshChild -ScriptPath $checkPath -Arguments $arguments -Quiet
        if ($result.Code -eq 0) {
            Write-Info "[perf-loop] 走行後の静寂確認: 静か（$RunDir\$MEASURE_FILE_QUIET_AFTER）"
            return
        }
        if ($result.Code -eq 4) {
            Stop-Run -Code $EXIT_MEASURE_FAILED -Message "走行後の静寂確認が性能カウンタを読めませんでした（計測失敗）: $($result.LastLine)"
        }
        Write-Info "[perf-loop] 走行後の静寂確認: 静かでない（$($i + 1)/$($retryMax + 1)）: $($result.LastLine)"
        if ($i -lt $retryMax) {
            Write-Info "[perf-loop] $waitSec 秒待って確かめ直します。"
            Start-Sleep -Seconds $waitSec
        }
    }
    Stop-Run -Code $EXIT_NOT_QUIET -Message "走行後の静寂確認が $($retryMax + 1) 回とも「静かでない」でした（根拠 $RunDir\$MEASURE_FILE_QUIET_AFTER）"
}

# 合否判定（judge-perf.py --mode verdict）。出力は verdict.txt に残し、判定語を返す。
# 判定が付かない（2 判定不能・3 引数不正）は計測失敗（4）——黙って合格にしない。
function Invoke-MeasureJudge {
    param([Parameter(Mandatory = $true)][string]$RunDir, [Parameter(Mandatory = $true)][string]$Build)
    if (-not (Test-Path -LiteralPath (Join-Path $RunDir $MEASURE_FILE_RUN_LOG) -PathType Leaf)) {
        if ($script:DryRunMode) {
            Write-Info '[perf-loop] -DryRun: 実走の成果物が無いので合否判定は行いません。'
            return $PERF_LOOP_EMPTY
        }
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message "実行ログがありません: $RunDir\$MEASURE_FILE_RUN_LOG"
    }
    # 合否を出す走行は実 SHIORI を伴っていなければならない（発話の無い別条件を判定しない）
    Assert-MeasureShioriHelper -RunDir $RunDir -Absolute -Label "合否判定の走行（$Build）" | Out-Null
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
    if ($result.Code -eq 0) { Write-Info "[perf-loop] 合否 PASS（$verdictPath）"; return 'PASS' }
    if ($result.Code -eq 1) { Write-Info "[perf-loop] 合否 FAIL（$verdictPath）"; return 'FAIL' }
    Stop-Run -Code $EXIT_MEASURE_FAILED `
        -Message "合否が付きませんでした（judge-perf.py 終了コード $($result.Code)・$verdictPath）: $($result.LastLine)"
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
#     採取ランナーは警告を出すだけで走ってしまう（invoke-perf-run.ps1:709-710）ので、
#     絶対値の合否を出す側でここを関門にする。
#   それ以外（rank-run／measure-ab＝**相対比較**）… 警告と 1 行の記録を残して続ける
#     （A と B が同じ条件なら差そのものは読める）。
function Assert-MeasureShioriHelper {
    param(
        [Parameter(Mandatory = $true)][string]$RunDir,
        [switch]$Absolute,
        [string]$Label = '走行'
    )
    $present = Get-MeasureShioriHelperPresent -RunDir $RunDir
    if ($present -eq $true) { return $true }
    $word = if ($null -eq $present) { $PERF_LOOP_EMPTY } else { 'false' }
    if ($Absolute) {
        Stop-Run -Code $EXIT_MEASURE_FAILED -Dir $RunDir -Message ("${Label}：32bit SHIORI ヘルパ無しの走行です（run-meta.txt の shiori_helper_present=$word）。" +
            "SHIORI が居ないと発話が起きず、アイドルの絶対値は別条件の値になります——合否には使えません。" +
            "cargo build --release -p $MEASURE_HELPER_PACKAGE --target $MEASURE_HELPER_TARGET を通してから採り直してください。")
    }
    Write-Problem "[perf-loop] 警告: ${Label} は 32bit SHIORI ヘルパ無しの走行です（shiori_helper_present=$word）。発話が無いので相対比較にだけ使えます。"
    Write-Info "[perf-loop] shiori_helper_present=$word dir=$RunDir"
    return $false
}

# 段③（関数別の帰属）の可否を走行の出力先へ残す。rank が後から読む（二重に測らない）。
function Write-MeasureSamplingNote {
    param([Parameter(Mandatory = $true)][string]$RunDir, [bool]$Available, [Parameter(Mandatory = $true)][string]$Reason)
    if (-not (Test-Path -LiteralPath $RunDir -PathType Container)) { return }
    $lines = @("available=$(Format-LoopBool $Available)", "reason=$Reason", "time_utc=$(Get-UtcStamp)")
    Set-Content -LiteralPath (Join-Path $RunDir $MEASURE_FILE_SAMPLING) -Value (($lines -join "`n") + "`n") -Encoding utf8 -NoNewline
}

function Get-MeasureSamplingNote {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    $path = Join-Path $RunDir $MEASURE_FILE_SAMPLING
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $null }
    $available = $false
    $reason = 'probe_failed'
    foreach ($line in (Get-Content -LiteralPath $path -Encoding utf8)) {
        if ($line -match '^available=(true|false)\s*$') { $available = ($Matches[1] -eq 'true') }
        elseif ($line -match '^reason=(\S+)\s*$')       { $reason = $Matches[1] }
    }
    return [pscustomobject]@{ Available = $available; Reason = $reason }
}

# 4 段の順位表（perf-rank.py）。dump.txt が無ければ段③を UNAVAILABLE として続ける
# （能力不足は停止の理由にしない＝design C5 の 5 の扱い）。
function Invoke-MeasureRank {
    param([Parameter(Mandatory = $true)][string]$RunDir)
    if ($script:DryRunMode -and -not (Test-Path -LiteralPath (Join-Path $RunDir $MEASURE_FILE_RUN_LOG) -PathType Leaf)) {
        Write-Info '[perf-loop] -DryRun: 実走の成果物が無いので順位表は作りません。'
        return $PERF_LOOP_EMPTY
    }
    $outPath = Join-Path $RunDir $MEASURE_FILE_RANK
    $arguments = @((Join-Path $PSScriptRoot 'perf-rank.py'), $RunDir, '--out', $outPath)
    if (Test-Path -LiteralPath (Join-Path $RunDir $MEASURE_FILE_DUMP) -PathType Leaf) {
        $arguments += @('--sampling-available', 'auto')
    } else {
        $note = Get-MeasureSamplingNote -RunDir $RunDir
        $reason = if ($note) { $note.Reason } else { (Get-SamplingProbe).Reason }
        $arguments += @('--sampling-available', 'false', '--unavailable-reason', $reason)
        Write-Info "[perf-loop] 段③（関数別の帰属）は UNAVAILABLE reason=$reason として順位表を作ります（段①②④で続けます）。"
    }
    $result = Invoke-PythonChild -Python (Get-MeasurePython) -Arguments $arguments
    if ($result.Code -ne 0) {
        Stop-Run -Code (Convert-MeasureChildExit -Code $result.Code) `
            -Message "順位表を作れませんでした（perf-rank.py 終了コード $($result.Code)）: $($result.LastLine)"
    }
    Write-Info "[perf-loop] 順位表: $outPath"
    return $outPath
}

# =============================================================================
# A/B のビルドと複製（prepare-ab／measure-ab）
# =============================================================================

function Test-MeasureBinDirComplete {
    param([Parameter(Mandatory = $true)][string]$Dir)
    if (-not (Test-Path -LiteralPath $Dir -PathType Container)) { return $false }
    if (-not (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_AREKA_EXE) -PathType Leaf)) { return $false }
    return (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_BUILD) -PathType Leaf)
}

# BUILD.txt に書く git の事実（どのツリーから作った実行体かを後から突き合わせるため）。
function Get-MeasureGitFacts {
    $git = Get-Command 'git' -ErrorAction SilentlyContinue
    if (-not $git) { return [pscustomobject]@{ Head = $PERF_LOOP_EMPTY; Dirty = $PERF_LOOP_EMPTY } }
    $head = Invoke-Child -Exe $git.Source -Arguments @('-C', $repoRoot, 'rev-parse', 'HEAD') -Quiet
    $stat = Invoke-Child -Exe $git.Source -Arguments @('-C', $repoRoot, 'status', '--porcelain') -Quiet
    $headText = if ($head.Code -eq 0) { $head.LastLine } else { $PERF_LOOP_EMPTY }
    $dirty = if ($stat.Code -ne 0) { $PERF_LOOP_EMPTY }
             elseif ((@($stat.Out) -join '').Trim()) { 'true' }
             else { 'false' }
    return [pscustomobject]@{ Head = $headText; Dirty = $dirty }
}

function Get-MeasureSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $PERF_LOOP_EMPTY }
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

# cargo を回す（Cargo.toml には触らない＝記号は環境変数で付ける・要件 2.4／8.6）。
function Invoke-MeasureCargo {
    param([Parameter(Mandatory = $true)][string[]]$Arguments, [Parameter(Mandatory = $true)][string]$Label)
    $cargo = Get-Command 'cargo' -ErrorAction SilentlyContinue
    if (-not $cargo) { Stop-Run -Code $EXIT_MEASURE_FAILED -Message 'cargo が見つかりません（A/B のビルドができません）' }
    $releaseDebug = Get-MeasureToml -Section 'levels' -Key 'release_debug_env' -Default $MEASURE_RELEASE_DEBUG_DEFAULT
    Write-Info "[perf-loop] ${Label}: cargo $($Arguments -join ' ')（CARGO_PROFILE_RELEASE_DEBUG=$releaseDebug）"
    return Invoke-Child -Exe $cargo.Source -Arguments $Arguments -Environment @{ CARGO_PROFILE_RELEASE_DEBUG = $releaseDebug }
}

# 片側（A＝変更前／B＝変更後）の実行体一式を作って小部屋へ複製する。
# 32bit ヘルパが作れないときは止めずに大きく警告して続ける——A と B を同じ手順で
# 作る限り条件は揃っており、ヘルパの有無は run-meta.txt と BUILD.txt に残るためである。
function Invoke-MeasureBuildSide {
    param(
        [Parameter(Mandatory = $true)][ValidateSet('A', 'B')][string]$Side,
        [Parameter(Mandatory = $true)][string]$BinDir
    )
    if ($script:ResumeMode -and (Test-MeasureBinDirComplete -Dir $BinDir)) {
        Write-Info "[perf-loop] -Resume: $Side 側の実行体一式を作り直しません: $BinDir"
        return $BinDir
    }
    $manifest    = Join-Path $repoRoot 'Cargo.toml'
    $releaseDir  = Join-Path $repoRoot 'target\release'
    $helperDir   = Join-Path $repoRoot (Join-Path 'target' (Join-Path $MEASURE_HELPER_TARGET 'release'))
    $helperState = 'skipped'

    if ($script:DryRunMode) {
        Write-Info "[perf-loop] -DryRun: $Side 側の cargo build は行いません（既にある target\release を複製できるなら複製します）。"
    } else {
        $build = Invoke-MeasureCargo -Label "$Side 側 release ビルド" `
            -Arguments @('build', '--release', '-p', $MEASURE_AREKA_PACKAGE, '--manifest-path', $manifest)
        if ($build.Code -ne 0) {
            Stop-Run -Code $EXIT_MEASURE_FAILED -Message "$Side 側の release ビルドが失敗しました（終了コード $($build.Code)）: $($build.LastLine)"
        }
        $helper = Invoke-MeasureCargo -Label "$Side 側 32bit SHIORI ヘルパのビルド" `
            -Arguments @('build', '--release', '-p', $MEASURE_HELPER_PACKAGE, '--target', $MEASURE_HELPER_TARGET, '--manifest-path', $manifest)
        if ($helper.Code -eq 0) {
            $helperState = 'built'
        } else {
            # 古い（別ツリーの）ヘルパを黙って混ぜない。missing と記して複製もしない
            # ——絶対値の合否を出す走行（measure-baseline／final）はこれを見て拒む。
            $helperState = 'missing'
            Write-Problem "[perf-loop] 警告: 32bit SHIORI ヘルパを作れませんでした（終了コード $($helper.Code)）: $($helper.LastLine)"
            Write-Problem "[perf-loop] 警告: ヘルパ無しの走行は実 SHIORI 無し＝発話のない別条件になります（rustup target add $MEASURE_HELPER_TARGET を確かめてください）。A と B の両側で同じ条件なら比較そのものは成立します。"
        }
    }

    New-LoopDir -Path $BinDir | Out-Null
    $exeSrc = Join-Path $releaseDir $MEASURE_AREKA_EXE
    if (Test-Path -LiteralPath $exeSrc -PathType Leaf) {
        Copy-Item -LiteralPath $exeSrc -Destination (Join-Path $BinDir $MEASURE_AREKA_EXE) -Force
    } elseif ($script:DryRunMode) {
        Write-MeasureDryRunMark -Dir $BinDir -Reason "target\release\$MEASURE_AREKA_EXE がまだありません（-DryRun なので作りません）"
        Write-Info "[perf-loop] -DryRun: $Side 側の複製は札だけ置きました: $BinDir"
        return $BinDir
    } else {
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message "ビルドは通ったのに実行体がありません: $exeSrc"
    }

    $pdbSrc = Join-Path $releaseDir $MEASURE_AREKA_PDB
    $pdbState = 'no_pdb'
    if (Test-Path -LiteralPath $pdbSrc -PathType Leaf) {
        Copy-Item -LiteralPath $pdbSrc -Destination (Join-Path $BinDir $MEASURE_AREKA_PDB) -Force
        $pdbState = $MEASURE_AREKA_PDB
    } else {
        Write-Problem "[perf-loop] 警告: PDB がありません（$pdbSrc）。段③（関数別の帰属）は記号を解決できません。"
    }

    $helperSrc = Join-Path $helperDir $MEASURE_HELPER_EXE
    if ($helperState -eq 'missing') {
        Write-Problem "[perf-loop] 警告: ヘルパのビルドに失敗したので複製しません（BUILD.txt は helper=missing）。"
    } elseif (Test-Path -LiteralPath $helperSrc -PathType Leaf) {
        Copy-Item -LiteralPath $helperSrc -Destination (Join-Path $BinDir $MEASURE_HELPER_EXE) -Force
        if ($helperState -eq 'skipped') { $helperState = 'copied' }
    } else {
        Write-Problem "[perf-loop] 警告: 32bit SHIORI ヘルパがありません（$helperSrc）。実 SHIORI 無しの走行になります。"
        $helperState = 'missing'
    }

    $git = Get-MeasureGitFacts
    $lines = @(
        "side=$Side",
        "time_utc=$(Get-UtcStamp)",
        "goal=$($script:GoalName)",
        "git_head=$($git.Head)",
        "git_dirty=$($git.Dirty)",
        "release_debug_env=$(Get-MeasureToml -Section 'levels' -Key 'release_debug_env' -Default $MEASURE_RELEASE_DEBUG_DEFAULT)",
        "exe=$MEASURE_AREKA_EXE",
        "exe_sha256=$(Get-MeasureSha256 -Path (Join-Path $BinDir $MEASURE_AREKA_EXE))",
        "pdb=$pdbState",
        "helper=$helperState",
        "helper_sha256=$(Get-MeasureSha256 -Path (Join-Path $BinDir $MEASURE_HELPER_EXE))",
        "dry_run=$(Format-LoopBool $script:DryRunMode)"
    )
    Set-Content -LiteralPath (Join-Path $BinDir $MEASURE_FILE_BUILD) -Value (($lines -join "`n") + "`n") -Encoding utf8 -NoNewline
    foreach ($line in $lines) { Write-Info "  $line" }
    Write-Info "[perf-loop] $Side 側の実行体一式: $BinDir"
    return $BinDir
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

    Write-Info "[perf-loop] measure-baseline 目標 $($script:GoalName) / ビルド $build / 水準 $longProfile / 出力先 $dir"

    if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $dir)) {
        Write-Info "[perf-loop] -Resume: 既にある走行を使います: $dir"
    } elseif (Test-MeasureExeReady -BinDir $script:BinDirArg -Build $build) {
        Invoke-MeasureRun -OutDir $dir -RunProfile $longProfile -Build $build -BinDir $script:BinDirArg -Label "ベースライン（$build）"
        Invoke-MeasureQuietAfter -RunDir $dir
    } else {
        Write-MeasureDryRunMark -Dir $dir -Reason "実行体が無いので走行を省きました（-DryRun）"
        Exit-WithResult -Code $EXIT_OK -Dir $dir
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
function Invoke-SubRankRun {
    $build = $script:BuildKind
    $dir = if ($script:RunDirArg) {
        Resolve-RunDir -RunDir $script:RunDirArg -Kind 'iter'
    } elseif ($script:DateStamp) {
        Get-LoopDir -Kind 'baseline' -Date $script:DateStamp -Leaf 'rank'
    } else {
        Assert-MeasureIter
        Get-LoopDir -Kind 'iter' -Iter $script:IterArg -Leaf 'rank'
    }
    $script:CurrentDir = $dir
    $shortProfile = Get-MeasureToml -Section 'levels' -Key 'short_profile' -Default $MEASURE_SHORT_PROFILE_DEFAULT

    Write-Info "[perf-loop] rank-run 目標 $($script:GoalName) / ビルド $build / 水準 $shortProfile / 出力先 $dir"

    if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $dir)) {
        Write-Info "[perf-loop] -Resume: 既にある走行を使います: $dir"
        Invoke-MeasureRank -RunDir $dir | Out-Null
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }

    if (-not (Test-MeasureExeReady -BinDir $script:BinDirArg -Build $build)) {
        Write-MeasureDryRunMark -Dir $dir -Reason "実行体が無いので走行を省きました（-DryRun）"
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }

    # --- 段③（サンプリング）の可否を先に確かめる -----------------------------
    $samplePath = Join-Path $PSScriptRoot 'invoke-cpu-sample.ps1'
    $etlPath = Join-Path (Split-Path -Parent $dir) ((Split-Path -Leaf $dir) + $MEASURE_TRACE_SUFFIX)
    $sampling = $false
    $reason = 'ok'
    if ($script:DryRunMode) {
        $reason = 'dry_run'
        Write-Info '[perf-loop] -DryRun: 段③（サンプリング）は行いません。'
    } else {
        $probe = Get-SamplingProbe
        if (-not $probe.Available) {
            $reason = $probe.Reason
            Write-Info "[perf-loop] 段③は使えません: reason=$reason。採取を省いて段①②④で続けます（停止の理由にしません）。"
        } else {
            New-LoopDir -Path (Split-Path -Parent $etlPath) | Out-Null
            $start = Invoke-PwshChild -ScriptPath $samplePath -Arguments @('-Start', '-Etl', $etlPath)
            if ($start.Code -eq 0) {
                $sampling = $true
                Write-Info "[perf-loop] 段③の採取を始めました: $etlPath"
            } else {
                $reason = if ($start.Code -eq $EXIT_UNAVAILABLE) { 'not_elevated' } else { 'start_failed' }
                Write-Problem "[perf-loop] 段③の採取を始められませんでした（終了コード $($start.Code)・reason=$reason）: $($start.LastLine)"
            }
        }
    }

    # --- 順位付けの走行（点灯） ----------------------------------------------
    Invoke-MeasureRun -OutDir $dir -RunProfile $shortProfile -Build $build -BinDir $script:BinDirArg `
        -RustLogExtra $MEASURE_RANK_RUSTLOG_EXTRA -Label '順位付けの走行'
    Assert-MeasureShioriHelper -RunDir $dir -Label '順位付けの走行' | Out-Null

    # --- 段③の停止と記号解決 -------------------------------------------------
    if ($sampling) {
        $pdbDir = Get-MeasureBinDir -BinDir $script:BinDirArg -Build $build
        $stopArgs = @('-Stop', '-Etl', $etlPath, '-Out', (Join-Path $dir $MEASURE_FILE_DUMP), '-PdbDir', $pdbDir)
        if ($script:GoalFilePath) { $stopArgs += @('-GoalFile', $script:GoalFilePath) }
        $stop = Invoke-PwshChild -ScriptPath $samplePath -Arguments $stopArgs
        if ($stop.Code -eq 0) {
            $reason = 'ok'
        } elseif ($stop.Code -eq $EXIT_UNAVAILABLE) {
            $sampling = $false
            $reason = 'no_xperf'
            Write-Problem "[perf-loop] 段③の記号解決は能力不足で行えませんでした（reason=$reason）。段①②④で続けます。"
        } else {
            Stop-Run -Code $EXIT_MEASURE_FAILED `
                -Message "段③の記号解決に失敗しました（invoke-cpu-sample.ps1 終了コード $($stop.Code)）: $($stop.LastLine)"
        }
    }
    Write-MeasureSamplingNote -RunDir $dir -Available $sampling -Reason $reason

    Invoke-MeasureQuietAfter -RunDir $dir
    Invoke-MeasureRank -RunDir $dir | Out-Null
    Exit-WithResult -Code $EXIT_OK -Dir $dir
}

# =============================================================================
# rank — 既にある走行から順位表だけを作り直す
# =============================================================================
function Invoke-SubRank {
    $dir = if ($script:RunDirArg) {
        Resolve-RunDir -RunDir $script:RunDirArg -Kind 'iter'
    } elseif ($script:DateStamp) {
        Get-LoopDir -Kind 'baseline' -Date $script:DateStamp -Leaf 'rank'
    } else {
        Assert-MeasureIter
        Get-LoopDir -Kind 'iter' -Iter $script:IterArg -Leaf 'rank'
    }
    $script:CurrentDir = $dir
    Write-Info "[perf-loop] rank 走行 $dir"
    if (-not (Test-Path -LiteralPath $dir -PathType Container)) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "走行の出力先がありません: $dir（-RunDir か -Iter を確かめてください）"
    }
    $rankPath = Join-Path $dir $MEASURE_FILE_RANK
    if ($script:ResumeMode -and (Test-ResumeArtifact -Path $rankPath -Resume)) {
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }
    if ($script:DryRunMode -and -not (Test-Path -LiteralPath (Join-Path $dir $MEASURE_FILE_RUN_LOG) -PathType Leaf)) {
        Write-Info '[perf-loop] -DryRun: 実走の成果物が無いので順位表は作りません。'
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
    Write-Info "[perf-loop] prepare-ab 周 $($script:IterArg) / A 側 $binA"
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
    $abBuild = Get-MeasureToml -Section 'levels' -Key 'iteration_build' -Default $MEASURE_ITER_BUILD_DEFAULT
    $sequence = Get-MeasureTomlArray -Section 'levels' -Key 'ab_sequence' -Default $MEASURE_AB_SEQUENCE_DEFAULT

    if ($abBuild -ne 'release') {
        # A/B のビルドは release 固定（判定式⑷a が release の式＝要件 5.3）。
        # 目標定義が別の値を書いていたら、黙って release を作らずここで止める。
        Stop-Run -Code $EXIT_BAD_ARGS -Message "目標定義 [levels] iteration_build は release だけを受け付けます（受け取った値: '$abBuild'）。A/B の実行体は release で作ります（判定式⑷a が release の式＝要件 5.3）。"
    }
    Write-Info "[perf-loop] measure-ab 周 $($script:IterArg) / 水準 $shortProfile / ビルド $abBuild / 順序 $($sequence -join ' ')"

    if (-not (Test-MeasureBinDirComplete -Dir $binA) -and -not $script:DryRunMode) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "A 側の実行体一式がありません: $binA（変更を入れる前に prepare-ab -Iter $($script:IterArg) を回してください）"
    }

    # B 側（＝いまのツリー）を作る
    Invoke-MeasureBuildSide -Side 'B' -BinDir $binB | Out-Null

    # 走行の名前（A→A1, B→B1, A→A2, B→B2）。名前は出力先の語彙に無ければ止める。
    $counts = @{ 'A' = 0; 'B' = 0 }
    $runs = @()
    foreach ($side in $sequence) {
        $key = $side.ToUpperInvariant()
        if (-not $counts.ContainsKey($key)) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "目標定義 [levels] ab_sequence に A／B 以外があります: '$side'"
        }
        $counts[$key] = $counts[$key] + 1
        $name = "$key$($counts[$key])"
        if ($PERF_LOOP_ITER_LEAVES -notcontains $name) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "走行の名前が出力先の語彙にありません: '$name'（使えるのは $($PERF_LOOP_ITER_LEAVES -join '・')）"
        }
        $runs += [pscustomobject]@{ Name = $name; Side = $key; Dir = (Join-Path $iterDir $name) }
    }

    foreach ($run in $runs) {
        if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $run.Dir)) {
            Write-Info "[perf-loop] -Resume: 既にある走行を使います: $($run.Dir)"
            continue
        }
        $binDir = if ($run.Side -eq 'A') { $binA } else { $binB }
        if (-not (Test-MeasureExeReady -BinDir $binDir -Build $abBuild)) {
            Write-MeasureDryRunMark -Dir $run.Dir -Reason "$($run.Side) 側の実行体が無いので走行を省きました（-DryRun）"
            continue
        }
        Invoke-MeasureRun -OutDir $run.Dir -RunProfile $shortProfile -Build $abBuild -BinDir $binDir -Label "走行 $($run.Name)"
        Assert-MeasureShioriHelper -RunDir $run.Dir -Label "走行 $($run.Name)" | Out-Null
        Invoke-MeasureQuietAfter -RunDir $run.Dir
    }
    $script:CurrentDir = $iterDir

    Invoke-MeasureCompareCore -IterDir $iterDir
    Exit-WithResult -Code $EXIT_OK -Dir $iterDir
}

# =============================================================================
# compare — A B A B の 4 本から差とばらつきを出して採否を返す
# =============================================================================
function Invoke-MeasureCompareCore {
    param([Parameter(Mandatory = $true)][string]$IterDir)
    $a1 = Join-Path $IterDir 'A1'
    $a2 = Join-Path $IterDir 'A2'
    $b1 = Join-Path $IterDir 'B1'
    $b2 = Join-Path $IterDir 'B2'
    foreach ($dir in @($a1, $a2, $b1, $b2)) {
        if (Test-Path -LiteralPath (Join-Path $dir $MEASURE_FILE_RUN_LOG) -PathType Leaf) { continue }
        if ($script:DryRunMode) {
            Write-Info "[perf-loop] -DryRun: 実走の成果物が無いので採否（perf-compare.py）は行いません（$dir）。"
            return
        }
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message "走行の成果物がありません: $dir\$MEASURE_FILE_RUN_LOG"
    }
    $abBuild = Get-MeasureToml -Section 'levels' -Key 'iteration_build' -Default $MEASURE_ITER_BUILD_DEFAULT
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
            -Message "採否を出せませんでした（perf-compare.py 終了コード $($result.Code)）: $($result.LastLine)" -Dir $IterDir
    }
    Write-Info "[perf-loop] 採否: $IterDir\$MEASURE_FILE_COMPARE"
}

function Invoke-SubCompare {
    Assert-MeasureIter
    $iterDir = Get-LoopDir -Kind 'iter' -Iter $script:IterArg
    $script:CurrentDir = $iterDir
    Write-Info "[perf-loop] compare 周 $($script:IterArg) / $iterDir"
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
# の 1 行を出す（invoke-followup-checks.ps1:222-231 の Stop-Run）。行が在ることだけを
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

function Invoke-SubFollowup {
    Assert-MeasureIter
    $iterDir = Get-LoopDir -Kind 'iter' -Iter $script:IterArg
    $dir = Join-Path $iterDir 'followup'
    $script:CurrentDir = $dir
    Sync-MeasureWorktreeBuild -Build 'release' -BinDir $script:BinDirArg
    $binDir = if ($script:BinDirArg) {
        [System.IO.Path]::GetFullPath($script:BinDirArg).TrimEnd('\')
    } else {
        $candidate = Join-Path $iterDir $MEASURE_SIDE_LEAF['B']
        if (Test-Path -LiteralPath (Join-Path $candidate $MEASURE_AREKA_EXE) -PathType Leaf) { $candidate }
        else { Get-MeasureBinDir -Build 'release' }
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
        Stop-Run -Code $EXIT_BAD_ARGS -Dir $dir -Message ("目標定義 [followup] required に未知の検査名があります: " +
            "$($unknown -join '・')（使えるのは $($MEASURE_FOLLOWUP_CHECKS_ALL -join '・')）")
    }
    # 並びは固定語彙の順に揃える（目標定義の書き順で報告の並びが揺れないように）。
    $checksArg = (@($MEASURE_FOLLOWUP_CHECKS_ALL | Where-Object { $checks -contains $_ }) -join ',')

    $checkerPath = Join-Path $PSScriptRoot 'invoke-followup-checks.ps1'
    $arguments = @(
        '-OutDir', $dir,
        '-BinDir', $binDir,
        '-ExitMs', "$exitMs",
        '-Checks', $checksArg,
        '-GhostRoot', (Get-MeasureGhostRoot),
        '-BalloonRoot', (Get-MeasureBalloonRoot)
    )
    Write-Info "[perf-loop] followup 周 $($script:IterArg) / 実行体 $binDir / 検査 $checksArg / 上限 $exitMs ms / 出力先 $dir"
    Write-Info "[perf-loop] 追随チェックの呼び出し行: invoke-followup-checks.ps1 $($arguments -join ' ')"

    if ($script:ResumeMode -and (Test-MeasureFollowupComplete -Dir $dir)) {
        Write-Info "[perf-loop] -Resume: 既にある追随チェックの判定を使います: $dir"
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }
    if ($script:DryRunMode) {
        Write-MeasureDryRunMark -Dir $dir -Reason ('追随チェックは -DryRun では行いません（実走と操作注入が要ります）。' +
            "渡す引数: invoke-followup-checks.ps1 $($arguments -join ' ')")
        Write-Info '[perf-loop] -DryRun: 追随チェックは行いませんでした。'
        Exit-WithResult -Code $EXIT_OK -Dir $dir
    }
    $attempts = $MEASURE_RUN_RETRY + 1
    for ($attempt = 1; $attempt -le $attempts; $attempt++) {
        # 前の回の置き土産（中止したときの followup.txt・古い判定）を残さない。
        # 残すと「今回の判定」と取り違える。
        foreach ($stale in @($MEASURE_FILE_FOLLOWUP, $MEASURE_FILE_FOLLOWUP_VERDICT)) {
            $stalePath = Join-Path $dir $stale
            if (Test-Path -LiteralPath $stalePath -PathType Leaf) { Remove-Item -LiteralPath $stalePath -Force }
        }
        Write-Info "[perf-loop] 追随チェックを回します（$attempt/$attempts）"
        $result = Invoke-PwshChild -ScriptPath $checkerPath -Arguments $arguments
        if ($result.Code -eq 3) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "追随チェックが引数・前提の不正で止まりました: $($result.LastLine)" -Dir $dir
        }
        $resultLine = $null
        foreach ($line in $result.Lines) { if ($line -match '^FOLLOWUP RESULT ') { $resultLine = $line.Trim() } }
        $judged = Test-Path -LiteralPath (Join-Path $dir $MEASURE_FILE_FOLLOWUP_VERDICT) -PathType Leaf
        $abortShape = ($resultLine -and ($resultLine -match 'clickthrough=-\s+drag=-\s+dpi=-\s+balloon_follow=-'))
        if ($resultLine -and $judged -and -not $abortShape) {
            Write-Info "[perf-loop] $resultLine"
            Exit-WithResult -Code $EXIT_OK -Dir $dir
        }
        $why = if (-not $resultLine) { '報告行がありません' }
               elseif ($abortShape)  { '中止の形の報告行です（4 検査とも -）' }
               else                  { "判定の出力（$MEASURE_FILE_FOLLOWUP_VERDICT）がありません" }
        Write-Problem "[perf-loop] 追随チェックが判定まで届きませんでした（終了コード $($result.Code)・$why）: $($result.LastLine)"
        if ($attempt -lt $attempts) { Write-Info '[perf-loop] もう 1 度だけ回します。' }
    }
    Stop-Run -Code $EXIT_MEASURE_FAILED -Message "追随チェックが $attempts 回とも判定まで届きませんでした（計測失敗＝MEASURE_FAILED）" -Dir $dir
}

# =============================================================================
# final — 25 分 × release／dev → judge-perf.py --mode verdict
# =============================================================================
# -Build を明示すると片方だけを採る。**2 本を別ターンで回すための形**であり
# （25 分 × 2 は check-in を跨ぐ＝要件 1.11）、2 本目は -Resume を付けて回すと
# 済んだ方を採り直さない。判定の出力は spec の results へも写す（要件 5.6）。
function Invoke-SubFinal {
    $dir = Resolve-RunDir -RunDir $script:RunDirArg -Kind 'final' -Date $script:DateStamp
    $script:CurrentDir = $dir
    $longProfile = Get-MeasureToml -Section 'levels' -Key 'long_profile' -Default $MEASURE_LONG_PROFILE_DEFAULT
    $builds = if ($script:BuildExplicit) {
        @($script:BuildKind)
    } else {
        Get-MeasureTomlArray -Section 'target' -Key 'builds_final' -Default $MEASURE_BUILDS_FINAL_DEFAULT
    }
    Write-Info "[perf-loop] final 目標 $($script:GoalName) / 水準 $longProfile / ビルド $($builds -join '・') / 出力先 $dir"

    $summary = @()
    foreach ($build in $builds) {
        if ($BUILD_KINDS -notcontains $build) {
            Stop-Run -Code $EXIT_BAD_ARGS -Message "目標定義 [target] builds_final に dev／release 以外があります: '$build'" -Dir $dir
        }
        $buildDir = Join-Path $dir $build
        if ($script:ResumeMode -and (Test-MeasureRunComplete -Dir $buildDir)) {
            Write-Info "[perf-loop] -Resume: 既にある走行を使います: $buildDir"
        } elseif (Test-MeasureExeReady -Build $build) {
            Invoke-MeasureRun -OutDir $buildDir -RunProfile $longProfile -Build $build -Label "最終判定（$build）"
            Invoke-MeasureQuietAfter -RunDir $buildDir
        } else {
            Write-MeasureDryRunMark -Dir $buildDir -Reason "実行体が無いので走行を省きました（-DryRun）"
            $summary += "final build=$build verdict=$PERF_LOOP_EMPTY dir=$buildDir"
            continue
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
    $stamp = Get-LoopDateStamp
    $target = Join-Path (Join-Path (Join-Path $repoRoot $script:GoalConfig.SpecDir) $script:GoalConfig.ResultsDir) `
        (Join-Path "final-$stamp" $Build)
    New-LoopDir -Path $target | Out-Null
    Copy-Item -LiteralPath $source -Destination (Join-Path $target $MEASURE_FILE_VERDICT) -Force
    foreach ($name in @($MEASURE_FILE_RUN_META, $MEASURE_FILE_QUIET_AFTER)) {
        $extra = Join-Path $RunDir $name
        if (Test-Path -LiteralPath $extra -PathType Leaf) { Copy-Item -LiteralPath $extra -Destination (Join-Path $target $name) -Force }
    }
    Write-Info "[perf-loop] 判定の出力を spec へ写しました: $target"
}
