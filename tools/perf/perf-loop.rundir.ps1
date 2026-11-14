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

# 走行の成果物が揃っているか（-Resume の完了印）。
function Test-MeasureRunComplete {
    param([Parameter(Mandatory = $true)][string]$Dir)
    foreach ($name in @($MEASURE_FILE_RUN_LOG, $MEASURE_FILE_CPU_CSV, $MEASURE_FILE_RUN_META)) {
        if (-not (Test-Path -LiteralPath (Join-Path $Dir $name) -PathType Leaf)) { return $false }
    }
    return (Test-MeasureQuietAfterOk -Dir $Dir)
}

# 走行後の静寂確認（quiet-after.txt）が NOT_QUIET だった走行は -Resume で再利用しない（設計: 静寂の確認は
# 前後 2 回＝後ろが落ちた走行は疑わしい）。2026-08-23 周 3 で A1/B1（走行後 21.4%/20.5%）がそのまま
# compare に使われた穴。NOT_QUIET の走行は `<走行>-NOTQUIET-<時刻>` へ退避し（測ったものは消さない）、$false を返す。
function Test-MeasureQuietAfterOk {
    param([Parameter(Mandatory = $true)][string]$Dir)
    $path = Join-Path $Dir $MEASURE_FILE_QUIET_AFTER
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $true }   # 後ろの確認を持たない走行（-DryRun 等）は対象外
    $verdict = @(Get-Content -LiteralPath $path -Encoding utf8 | Where-Object { $_ -match '^verdict=' } | Select-Object -Last 1)
    if ($verdict.Count -eq 0 -or $verdict[0] -match '^verdict=QUIET$') { return $true }
    $stamp = (Get-UtcStamp) -replace '[^0-9]', ''
    $aside = "$Dir-NOTQUIET-$stamp"
    Move-Item -LiteralPath $Dir -Destination $aside -Force
    Write-Problem "[perf-loop] post-run quiet check was $($verdict[0]); run not reused, moved aside: $aside (will rerun)"
    return $false
}

# -DryRun の置き土産（DRY-RUN.txt があって run.log が無い出力先）だけを作り直す。
# 実走の成果物がある出力先には触らない——測ったものを黙って消さないため。
function Clear-MeasureDryRunDir {
    param([Parameter(Mandatory = $true)][string]$Dir)
    if (-not (Test-Path -LiteralPath $Dir)) { return }
    $isDry  = Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_DRYRUN) -PathType Leaf
    $hasRun = Test-Path -LiteralPath (Join-Path $Dir $MEASURE_FILE_RUN_LOG) -PathType Leaf
    if ($isDry -and -not $hasRun) {
        Write-Info "[perf-loop] recreating the output folder of the previous -DryRun: $Dir"
        Remove-Item -LiteralPath $Dir -Recurse -Force
    }
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
