#Requires -Version 7.0
<#
================================================================================
perf-loop.build.ps1 — perf-loop の実行体の用意（作る・写す・確かめる・A と B）
  spec: audit-tools-perf-run（直し 20・design「perf-loop.build」）

読み込み用。perf-loop.measure.ps1 が自分の較正値（$MEASURE_*）を置いた後で
    . (Join-Path $PSScriptRoot 'perf-loop.build.ps1')
として読み込む。単独で回しても何もしない。
頼ってよいのは perf-loop.common.ps1 と perf-loop.measure.ps1 の定数・関数だけ
（perf-loop.ps1 の変数には頼らない）。
================================================================================
#>

Set-StrictMode -Version 3.0

# =============================================================================
# 実行体の在り処・作り直し・32bit ヘルパ
# =============================================================================

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
        "Set CARGO_PROFILE_RELEASE_DEBUG=$releaseDebug in the environment from PowerShell and run cargo build --release -p $MEASURE_AREKA_PACKAGE first (stage 3 needs the symbols)"
    } else {
        "Run cargo build -p $MEASURE_AREKA_PACKAGE first"
    }
    if ($script:DryRunMode) {
        Write-Info "[perf-loop] -DryRun: executable not found ($exe). Skipping this run. $hint"
        return $false
    }
    Stop-Run -Code $EXIT_BAD_ARGS -Message "executable not found: $exe. $hint"
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
        Write-Problem "[perf-loop] WARNING: 32bit SHIORI helper is in neither $dest nor $src. Run cargo build --release -p $MEASURE_HELPER_PACKAGE --target $MEASURE_HELPER_TARGET first (a pass/fail run stops with exit 4)."
        return
    }
    Copy-Item -LiteralPath $src -Destination $dest -Force
    Write-Info "[perf-loop] copied 32bit SHIORI helper next to the executable: $src -> $dest"
}

# 走行前に作業ツリーから実行体を作り直す（-BinDir 無しの走行＝followup／rank-run／measure-baseline／final）。
# measure-ab は B 側を target\release に作って残すので、次の周の rank-run が前の周の B を黙って測り、
# 逆に followup は prepare-ab が残した A（HEAD）を測っていた（2026-08-23 周 1〜3 で実際に起きた＝周 1 の
# 「門 ON がドラッグを壊す」は A 実行体での失敗だった）。-BinDir が明示された走行と -DryRun は触らない。
function Sync-MeasureWorktreeBuild {
    param([Parameter(Mandatory = $true)][string]$Build, [string]$BinDir)
    if ($BinDir) { return }
    if ($script:DryRunMode) { return }
    $manifest = Join-Path $repoRoot 'Cargo.toml'
    $cargoArgs = @('build'); if ($Build -eq 'release') { $cargoArgs += '--release' }
    $cargoArgs += @('-p', $MEASURE_AREKA_PACKAGE, '--manifest-path', $manifest)
    $r = Invoke-MeasureCargo -Label "pre-run $Build build from the worktree" -Arguments $cargoArgs
    if ($r.Code -ne 0) { Stop-Run -Code $EXIT_MEASURE_FAILED -Message "pre-run $Build build failed (exit code $($r.Code)): $($r.LastLine)" }
    $exe = Join-Path (Get-MeasureBinDir -BinDir '' -Build $Build) $MEASURE_AREKA_EXE
    $git = Get-MeasureGitFacts
    Write-Info "[perf-loop] executable to run: $exe sha256=$(Get-MeasureSha256 -Path $exe) git_head=$($git.Head) git_dirty=$($git.Dirty)"
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
    if (-not $cargo) { Stop-Run -Code $EXIT_MEASURE_FAILED -Message 'cargo not found (cannot build A/B)' }
    $releaseDebug = Get-MeasureToml -Section 'levels' -Key 'release_debug_env' -Default $MEASURE_RELEASE_DEBUG_DEFAULT
    Write-Info "[perf-loop] ${Label}: cargo $($Arguments -join ' ') (CARGO_PROFILE_RELEASE_DEBUG=$releaseDebug)"
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
        Write-Info "[perf-loop] -Resume: not recreating the $Side side executable set: $BinDir"
        return $BinDir
    }
    $manifest    = Join-Path $repoRoot 'Cargo.toml'
    $releaseDir  = Join-Path $repoRoot 'target\release'
    $helperDir   = Join-Path $repoRoot (Join-Path 'target' (Join-Path $MEASURE_HELPER_TARGET 'release'))
    $helperState = 'skipped'

    if ($script:DryRunMode) {
        Write-Info "[perf-loop] -DryRun: no cargo build for the $Side side (copies the existing target\release if it can)."
    } else {
        $build = Invoke-MeasureCargo -Label "$Side side release build" `
            -Arguments @('build', '--release', '-p', $MEASURE_AREKA_PACKAGE, '--manifest-path', $manifest)
        if ($build.Code -ne 0) {
            Stop-Run -Code $EXIT_MEASURE_FAILED -Message "$Side side release build failed (exit code $($build.Code)): $($build.LastLine)"
        }
        $helper = Invoke-MeasureCargo -Label "$Side side 32bit SHIORI helper build" `
            -Arguments @('build', '--release', '-p', $MEASURE_HELPER_PACKAGE, '--target', $MEASURE_HELPER_TARGET, '--manifest-path', $manifest)
        if ($helper.Code -eq 0) {
            $helperState = 'built'
        } else {
            # 古い（別ツリーの）ヘルパを黙って混ぜない。missing と記して複製もしない
            # ——絶対値の合否を出す走行（measure-baseline／final）はこれを見て拒む。
            $helperState = 'missing'
            Write-Problem "[perf-loop] warning: could not build the 32bit SHIORI helper (exit code $($helper.Code)): $($helper.LastLine)"
            Write-Problem "[perf-loop] warning: a run without the helper has no real SHIORI and no talk, a different condition (check rustup target add $MEASURE_HELPER_TARGET). If A and B share the condition, the comparison itself still holds."
        }
    }

    New-LoopDir -Path $BinDir | Out-Null
    $exeSrc = Join-Path $releaseDir $MEASURE_AREKA_EXE
    if (Test-Path -LiteralPath $exeSrc -PathType Leaf) {
        Copy-Item -LiteralPath $exeSrc -Destination (Join-Path $BinDir $MEASURE_AREKA_EXE) -Force
    } elseif ($script:DryRunMode) {
        Write-MeasureDryRunMark -Dir $BinDir -Reason "target\release\$MEASURE_AREKA_EXE does not exist yet (not built under -DryRun)"
        Write-Info "[perf-loop] -DryRun: placed only the mark for the $Side side copy: $BinDir"
        return $BinDir
    } else {
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message "build passed but the executable is missing: $exeSrc"
    }

    $pdbSrc = Join-Path $releaseDir $MEASURE_AREKA_PDB
    $pdbState = 'no_pdb'
    if (Test-Path -LiteralPath $pdbSrc -PathType Leaf) {
        Copy-Item -LiteralPath $pdbSrc -Destination (Join-Path $BinDir $MEASURE_AREKA_PDB) -Force
        $pdbState = $MEASURE_AREKA_PDB
    } else {
        Write-Problem "[perf-loop] warning: no PDB ($pdbSrc). Stage 3 (per-function attribution) cannot resolve symbols."
    }

    $helperSrc = Join-Path $helperDir $MEASURE_HELPER_EXE
    if ($helperState -eq 'missing') {
        Write-Problem "[perf-loop] warning: the helper build failed, so it is not copied (BUILD.txt says helper=missing)."
    } elseif (Test-Path -LiteralPath $helperSrc -PathType Leaf) {
        Copy-Item -LiteralPath $helperSrc -Destination (Join-Path $BinDir $MEASURE_HELPER_EXE) -Force
        if ($helperState -eq 'skipped') { $helperState = 'copied' }
    } else {
        Write-Problem "[perf-loop] warning: no 32bit SHIORI helper ($helperSrc). The run will have no real SHIORI."
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
    Write-Info "[perf-loop] $Side side executable set: $BinDir"
    return $BinDir
}
