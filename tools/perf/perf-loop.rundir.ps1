#Requires -Version 7.0
<#
================================================================================
perf-loop.rundir.ps1 — perf-loop の走行の出力先の扱い（完了の見方・脇へ退ける・-DryRun の印）
  spec: audit-tools-perf-run（直し 20・design「perf-loop.rundir」）

読み込み用。perf-loop.measure.ps1 が自分の較正値（$MEASURE_*）を置いた後で
    . (Join-Path $PSScriptRoot 'perf-loop.rundir.ps1')
として読み込む。単独で回しても何もしない。
頼ってよいのは perf-loop.common.ps1 と perf-loop.measure.ps1 の定数・関数だけ
（perf-loop.ps1 の変数には頼らない）。
================================================================================
#>

Set-StrictMode -Version 3.0

# -Resume の完了の印（2）。完了なら $true。run.log の在る未完了の走行は -INCOMPLETE、静かでない走行は
# -NOTQUIET へ脇へ退けて $false（採り直させる。測ったものは消さない）。run.log の無い出力先のうち、
# -DryRun の置き土産（DRY-RUN.txt が在る）と在らない出力先は触らずに $false。DRY-RUN.txt も run.log も無い出力先は
# 静かさの確かめの間に止まった走行なので -INCOMPLETE へ退ける（-Resume で「既に在る」の 3 にしない）。
# ended_utc はランナーが成功の最後にだけ書く。
function Test-MeasureRunComplete {
    param([Parameter(Mandatory = $true)][string]$Dir)
    if (-not (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_RUN_LOG) -PathType Leaf)) {
        if ((Test-Path -LiteralPath $Dir -PathType Container) -and -not (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_DRYRUN) -PathType Leaf)) {
            $aside = Move-MeasureRunAside -Dir $Dir -Tag 'INCOMPLETE'
            Write-Problem "[perf-loop] run stopped before $MEASURE_FILE_RUN_LOG was written; moved aside: $aside (will rerun)"
        }
        return $false
    }
    $meta  = Join-Path $Dir $MEASURE_FILE_RUN_META
    $after = Join-Path $Dir $MEASURE_FILE_QUIET_AFTER
    $verdict = @()
    if (Test-Path -LiteralPath $after -PathType Leaf) {
        $verdict = @(Get-Content -LiteralPath $after -Encoding utf8 | Where-Object { $_ -match '^verdict=' } | Select-Object -Last 1)
    }
    $complete = (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_CPU_CSV) -PathType Leaf) -and
        (Test-Path -LiteralPath $meta -PathType Leaf) -and
        [bool](@(Get-Content -LiteralPath $meta -Encoding utf8) -match '^ended_utc = ') -and
        $verdict.Count -eq 1
    if (-not $complete) {
        $aside = Move-MeasureRunAside -Dir $Dir -Tag 'INCOMPLETE'
        Write-Problem "[perf-loop] run is incomplete (missing $MEASURE_FILE_CPU_CSV, ended_utc in $MEASURE_FILE_RUN_META, or a verdict in $MEASURE_FILE_QUIET_AFTER); run not reused, moved aside: $aside (will rerun)"
        return $false
    }
    if ($verdict[0] -notmatch '^verdict=QUIET$') {
        $aside = Move-MeasureRunAside -Dir $Dir -Tag 'NOTQUIET'
        Write-Problem "[perf-loop] post-run quiet check was $($verdict[0]); run not reused, moved aside: $aside (will rerun)"
        return $false
    }
    return $true
}

# 脇へ退ける唯一の口。'<Dir>-<Tag>-<UTC の数字だけの時刻>' へ名前を替える。戻り: 退けた先。
function Move-MeasureRunAside {
    param([Parameter(Mandatory = $true)][string]$Dir, [Parameter(Mandatory = $true)][ValidateSet('INCOMPLETE', 'NOTQUIET')][string]$Tag)
    $aside = "$Dir-$Tag-$((Get-UtcStamp) -replace '[^0-9]', '')"
    Move-Item -LiteralPath $Dir -Destination $aside
    return $aside
}

# -DryRun の置き土産を作り直す（6）。DRY-RUN.txt の在る出力先で、中身が印のファイル（DRY-RUN.txt と
# ランナーの -DryRun が書く run-meta.txt）だけ（下位のフォルダ無し）のときに限って消す。ほかが在れば
# 3 で止めて何も消さない（利用者が渡したフォルダを丸ごと消さないため）。DRY-RUN.txt が無ければ触らない。
function Clear-MeasureDryRunDir {
    param([Parameter(Mandatory = $true)][string]$Dir)
    if (-not (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_DRYRUN) -PathType Leaf)) { return }
    $others = @(Get-ChildItem -LiteralPath $Dir -Force |
        Where-Object { $_.PSIsContainer -or @($MEASURE_FILE_DRYRUN, $MEASURE_FILE_RUN_META) -notcontains $_.Name } |
        ForEach-Object Name)
    if ($others.Count -gt 0) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "output folder holds a -DryRun mark and other things ($($others -join ', ')): $Dir (nothing was deleted. Use a new folder, or remove $MEASURE_FILE_DRYRUN yourself)"
    }
    Write-Info "[perf-loop] recreating the output folder of the previous -DryRun: $Dir"
    Remove-Item -LiteralPath $Dir -Recurse -Force
}

# 走行を省いたことを出力先に残す（-DryRun 専用。測定結果と取り違えないための札）。
function Write-MeasureDryRunMark {
    param([Parameter(Mandatory = $true)][string]$Dir, [Parameter(Mandatory = $true)][string]$Reason)
    New-LoopDir -Path $Dir | Out-Null
    $lines = @(
        'This is -DryRun (test only) output. Do not treat it as a measurement result.',
        "reason: $Reason",
        "time: $(Get-UtcStamp)"
    )
    Set-Content -LiteralPath (Join-Path $Dir $MEASURE_FILE_DRYRUN) -Value (($lines -join "`n") + "`n") -Encoding utf8 -NoNewline
}
