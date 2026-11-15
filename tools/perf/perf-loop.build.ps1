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

# 32bit SHIORI ヘルパの作られる所（target\<i686>\release・隣へ写す元）。
function Get-MeasureHelperSource {
    return (Join-Path $repoRoot (Join-Path 'target' (Join-Path $MEASURE_HELPER_TARGET (Join-Path 'release' $MEASURE_HELPER_EXE))))
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
# -BinDir の無い走行（target\<release|debug> を直に使う）だけは、隣に在っても元と SHA-256 が違えば写し直す
# （古いヘルパで測らない・12）。-BinDir の走行（A/B の bin-A・bin-B、final -BinDir）は隣に無いときだけ写す
# （共有の元は B のビルドで変わるので、bin-A の BUILD.txt と食い違うヘルパを入れない）。元が無ければ隣のものを残す。
function Sync-MeasureShioriHelper {
    param([string]$BinDir, [Parameter(Mandatory = $true)][string]$Build)
    $dest = Join-Path (Get-MeasureBinDir -BinDir $BinDir -Build $Build) $MEASURE_HELPER_EXE
    $src = Get-MeasureHelperSource
    $destPresent = Test-Path -LiteralPath $dest -PathType Leaf
    if (-not (Test-Path -LiteralPath $src -PathType Leaf)) {
        if ($destPresent) { return }
        Write-Problem "[perf-loop] WARNING: 32bit SHIORI helper is in neither $dest nor $src. Run cargo build --release -p $MEASURE_HELPER_PACKAGE --target $MEASURE_HELPER_TARGET first (a pass/fail run stops with exit 4)."
        return
    }
    if ($destPresent -and ($BinDir -or (Get-MeasureSha256 -Path $src) -eq (Get-MeasureSha256 -Path $dest))) { return }
    # 小部屋の BUILD.txt が helper=missing と書く側には、共有の元（多くは相手の側のビルド）を入れない
    $build = if ($BinDir) { Join-Path $BinDir $MEASURE_FILE_BUILD }
    if ($build -and (Test-Path -LiteralPath $build -PathType Leaf) -and (Select-String -LiteralPath $build -Pattern '^helper=missing$' -Quiet)) {
        Write-Problem "[perf-loop] warning: $build says helper=missing, so the shared helper is not copied into $BinDir (the run has no real SHIORI)."
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
    # 32bit ヘルパも同じツリーから作り直す（隣へ写す元＝target\<i686>\release が古いままにならない）。作れなくても止めない
    $helper = Invoke-MeasureCargo -Label 'pre-run 32bit SHIORI helper build from the worktree' `
        -Arguments @('build', '--release', '-p', $MEASURE_HELPER_PACKAGE, '--target', $MEASURE_HELPER_TARGET, '--manifest-path', $manifest)
    if ($helper.Code -ne 0) {
        # 別のツリーの古いヘルパで測らない（A/B の側と同じ決まり）。元と隣の写しを消し、合否の走行は既にある関門（4）で止まる
        Remove-Item -LiteralPath (Get-MeasureHelperSource) -Force -ErrorAction Ignore
        Remove-Item -LiteralPath (Join-Path (Get-MeasureBinDir -BinDir '' -Build $Build) $MEASURE_HELPER_EXE) -Force -ErrorAction Ignore
        Write-Problem "[perf-loop] warning: could not rebuild the 32bit SHIORI helper (exit code $($helper.Code)): $($helper.LastLine). The old helper was removed, so this run has no real SHIORI (a pass/fail run stops with 4)."
    }
    $exe = Join-Path (Get-MeasureBinDir -BinDir '' -Build $Build) $MEASURE_AREKA_EXE
    $git = Get-MeasureGitFacts
    Write-Info "[perf-loop] executable to run: $exe sha256=$(Get-MeasureSha256 -Path $exe) git_head=$($git.Head) git_dirty=$($git.Dirty)"
}

# =============================================================================
# A/B のビルドと複製（prepare-ab／measure-ab）
# =============================================================================

# 揃った小部屋か。-DryRun で写しただけの一式（BUILD.txt の dry_run=true＝そのツリーから作っていない）は揃ったと見ない
function Test-MeasureBinDirComplete {
    param([Parameter(Mandatory = $true)][string]$Dir)
    if (-not (Test-Path -LiteralPath $Dir -PathType Container)) { return $false }
    if (-not (Test-Path -LiteralPath (Join-Path $Dir $MEASURE_AREKA_EXE) -PathType Leaf)) { return $false }
    $build = Join-Path $Dir $MEASURE_FILE_BUILD
    if (-not (Test-Path -LiteralPath $build -PathType Leaf)) { return $false }
    return -not (Select-String -LiteralPath $build -Pattern '^dry_run=true$' -Quiet)
}

# BUILD.txt に書く git の事実（どのツリーから作った実行体かを後から突き合わせるため）。
function Get-MeasureGitFacts {
    $git = Get-Command 'git' -ErrorAction SilentlyContinue
    if (-not $git) { return [pscustomobject]@{ Head = $PERF_LOOP_EMPTY; Dirty = $PERF_LOOP_EMPTY } }
    $head = Invoke-Child -Exe $git.Source -Arguments @('-C', $repoRoot, 'rev-parse', 'HEAD') -Quiet
    $stat = Invoke-Child -Exe $git.Source -Arguments @('-C', $repoRoot, 'status', '--porcelain') -Quiet
    # 標準出力の最後の行だけ（git の警告は標準エラーに出るので混ぜない）
    $headOut = @($head.Out | Where-Object { $_ })
    $headText = if ($head.Code -eq 0 -and $headOut.Count -gt 0) { $headOut[-1].Trim() } else { $PERF_LOOP_EMPTY }
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
    if (-not $cargo) { Stop-Run -Code $EXIT_MEASURE_FAILED -Message "cargo not found (cannot run the ${Label}: cargo $($Arguments -join ' '))" }
    $releaseDebug = Get-MeasureToml -Section 'levels' -Key 'release_debug_env' -Default $MEASURE_RELEASE_DEBUG_DEFAULT
    Write-Info "[perf-loop] ${Label}: cargo $($Arguments -join ' ') (CARGO_PROFILE_RELEASE_DEBUG=$releaseDebug)"
    # cargo の Compiling の行は呼ぶ側の会話へ流さない（トークンの負荷）。落ちたときだけ最後の数行を出す
    $r = Invoke-Child -Exe $cargo.Source -Arguments $Arguments -Environment @{ CARGO_PROFILE_RELEASE_DEBUG = $releaseDebug } -Quiet
    if ($r.Code -ne 0) { foreach ($line in @($r.Lines | Select-Object -Last 8)) { Write-Problem "    $line" } }
    return $r
}

# 片側（A＝変更前／B＝変更後）の実行体一式を作って小部屋へ複製する。
# 32bit ヘルパが作れないときは止めずに大きく警告して続ける——ヘルパの有無は BUILD.txt に残り、
# A と B で食い違えば measure-ab が走る前に止める（Assert-MeasureAbDistinct）。
function Invoke-MeasureBuildSide {
    param(
        [Parameter(Mandatory = $true)][ValidateSet('A', 'B')][string]$Side,
        [Parameter(Mandatory = $true)][string]$BinDir
    )
    # -Resume は揃った一式を使う。-DryRun は配管だけを確かめるので、揃った本物の一式を書き替えない（A の実行体を失う）
    if (($script:ResumeMode -or $script:DryRunMode) -and (Test-MeasureBinDirComplete -Dir $BinDir)) {
        Write-Info "[perf-loop] -Resume/-DryRun: not recreating the $Side side executable set: $BinDir"
        return $BinDir
    }
    $manifest    = Join-Path $repoRoot 'Cargo.toml'
    $releaseDir  = Join-Path $repoRoot 'target\release'
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
    # 作り直しの途中で止まったとき、新しい実行体と古い BUILD.txt を揃った一式と見ない（BUILD.txt は最後に書く）
    Remove-Item -LiteralPath (Join-Path $BinDir $MEASURE_FILE_BUILD) -Force -ErrorAction Ignore
    # 前の -DryRun が置いた印を本物の一式に残さない
    if (-not $script:DryRunMode) { Remove-Item -LiteralPath (Join-Path $BinDir $MEASURE_FILE_DRYRUN) -Force -ErrorAction Ignore }
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
        # 前の回の PDB を残さない（BUILD.txt の pdb=no_pdb と食い違う）
        Remove-Item -LiteralPath (Join-Path $BinDir $MEASURE_AREKA_PDB) -Force -ErrorAction Ignore
        Write-Problem "[perf-loop] warning: no PDB ($pdbSrc). Stage 3 (per-function attribution) cannot resolve symbols."
    }

    $helperSrc = Get-MeasureHelperSource
    $helperDest = Join-Path $BinDir $MEASURE_HELPER_EXE
    if ($helperState -eq 'missing') {
        # 前の回の（別のツリーの）ヘルパを残さない（BUILD.txt の helper=missing と食い違う）
        Remove-Item -LiteralPath $helperDest -Force -ErrorAction Ignore
        Write-Problem "[perf-loop] warning: the helper build failed, so it is not copied (BUILD.txt says helper=missing)."
    } elseif (Test-Path -LiteralPath $helperSrc -PathType Leaf) {
        Copy-Item -LiteralPath $helperSrc -Destination $helperDest -Force
        if ($helperState -eq 'skipped') { $helperState = 'copied' }
    } else {
        Remove-Item -LiteralPath $helperDest -Force -ErrorAction Ignore
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

# 15: bin-A と bin-B の BUILD.txt の exe_sha256 が同じなら、A と B は同じ実行体（変更が実行体に届いていない、
# または prepare-ab を変更の後に回した）。走る前に 3 で止める。どちらかが '-'（読めない・無い）なら比べない。
function Assert-MeasureAbDistinct {
    param([Parameter(Mandatory = $true)][string]$BinA, [Parameter(Mandatory = $true)][string]$BinB)
    $sha = @(); $helper = @()
    foreach ($dir in @($BinA, $BinB)) {
        $path = Join-Path $dir $MEASURE_FILE_BUILD
        $map = if (Test-Path -LiteralPath $path -PathType Leaf) { ConvertFrom-KeyValueLines -Lines @(Get-Content -LiteralPath $path -Encoding utf8) } else { @{} }
        $sha += $(if ($map['exe_sha256']) { $map['exe_sha256'].Trim() } else { $PERF_LOOP_EMPTY })
        $helper += $(if ($map['helper']) { $map['helper'].Trim() } else { $PERF_LOOP_EMPTY })
    }
    # 片側だけ 32bit ヘルパが無い（helper=missing）と、発話の有る側と無い側を比べることになる。走る前に 4 で止める
    if (($helper[0] -eq 'missing') -ne ($helper[1] -eq 'missing')) {
        Stop-Run -Code $EXIT_MEASURE_FAILED -Message "A and B differ in the 32bit SHIORI helper (helper=$($helper[0]) in $BinA, helper=$($helper[1]) in $BinB): one side would run without talk. Fix the helper build and prepare the side again. No run was started."
    }
    if ($sha[0] -eq $PERF_LOOP_EMPTY -or $sha[1] -eq $PERF_LOOP_EMPTY) { return }
    if ($sha[0] -eq $sha[1]) {
        Stop-Run -Code $EXIT_BAD_ARGS -Message "A and B are the same executable (exe_sha256=$($sha[0]) in both $BinA and $BinB): the change did not reach the executable, or prepare-ab ran after the change. No run was started."
    }
}
