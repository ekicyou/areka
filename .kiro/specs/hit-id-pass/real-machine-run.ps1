<#
.SYNOPSIS
  hit-id-pass の実機の確かめ（クローディアの楕円と多角形の名前が Reference4 に載ること）の根づくりと起動。
  完了 collisionex-regions の同名の道具の写し（根の場所を target\hip-signoff に変え、-Run で起動の済みを待って撫での道具まで動かす）。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 claudia を nar-sample-path で展開して
             ワークツリーの target\hip-signoff\root に写す。製品のコードとリポジトリの検体（.nar）には触れない。
  -Run R1  : 127.0.0.1:9276 を握るものが無いことを確かめ、記憶を消して根で areka を起こす。記録に OnFirstBoot か OnBoot が出たら、
             pid のプロセス名が areka であることを確かめて real-machine-drive.ps1 を動かし、自動終了まで待つ。
             記録は target\hip-signoff\run-R1.log・道具の出力は drive-R1.txt。
#>
param(
    [switch]$Prepare,
    [ValidateSet('R1')]
    [string]$Run,
    [int]$ExitMs = 120000,
    [string]$RustLog = 'info,areka=debug,kanade=trace,areka_emo_present::presenter::hit=debug'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$base = Join-Path $wt 'target\hip-signoff'
$root = Join-Path $base 'root'

function Machine([string]$exe) {
    $b = [IO.File]::ReadAllBytes($exe)
    $pe = [BitConverter]::ToInt32($b, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

if ($Prepare) {
    Push-Location $wt
    try {
        cargo build -p areka --bin areka -j 4; if ($LASTEXITCODE) { throw "areka のビルドが失敗 ($LASTEXITCODE)" }
        cargo build -p shiori-host32-helper --target i686-pc-windows-msvc -j 4; if ($LASTEXITCODE) { throw "helper のビルドが失敗 ($LASTEXITCODE)" }
        $printed = cargo run -q -p sample-ghost-kit --bin nar-sample-path -- claudia
        if ($LASTEXITCODE) { throw "nar-sample-path が失敗 ($LASTEXITCODE)" }
    } finally { Pop-Location }
    $manual = ($printed | Where-Object { $_ -like 'root=*' }) -replace '^root=', ''
    $helper = Join-Path $wt 'target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe'
    if ((Machine $helper) -ne '014C') { throw "helper が 32bit でない: $helper" }
    # 根のフォルダそのものは消さず中身だけを消す（シェルの今いる場所が根の中だと、フォルダは消せない）。
    New-Item -ItemType Directory -Force $root | Out-Null
    Get-ChildItem -Force $root | Remove-Item -Recurse -Force
    Copy-Item -Recurse (Join-Path $manual '*') $root
    Copy-Item (Join-Path $wt 'target\debug\areka.exe') $root
    Copy-Item $helper $root
    "prepared $root (helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')) areka=$(Machine (Join-Path $root 'areka.exe')))"
}

if ($Run) {
    if (-not (Test-Path (Join-Path $root 'areka.exe'))) { throw "根がない。先に -Prepare" }
    $held = netstat -ano | Select-String '127\.0\.0\.1:9276\s'
    if ($held) { throw "127.0.0.1:9276 を握るものがある（$($held -join ' / ')）。起こさずに止める" }
    $ghost = Join-Path $root 'ghost\claudia'
    foreach ($p in 'profile', 'tmp', 'ghost\claudia\ghost\master\profile\areka') {
        $d = Join-Path $root $p; if (Test-Path $d) { Remove-Item -Recurse -Force $d }
    }
    New-Item -ItemType Directory -Force (Join-Path $root 'tmp') | Out-Null
    Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
    $env:NO_COLOR = '1'
    $env:RUST_LOG = $RustLog
    $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
    $env:AREKA_ROOT = $root
    $env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
    $env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
    $log = Join-Path $base "run-$Run.log"
    $p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$ghost`"" -WorkingDirectory $root `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $base "run-$Run.err.log") -WindowStyle Hidden -PassThru
    "$Run pid=$($p.Id) start=$(Get-Date -Format o) exit_ms=$ExitMs" | Tee-Object -Append (Join-Path $base 'runs.txt')
    # 記憶を消すので OnFirstBoot が来て OnBoot は飛ばされる（collisionex-regions の 2 回目）。
    $deadline = (Get-Date).AddSeconds(60)
    while (-not ((Test-Path $log) -and (Select-String -Path $log -Pattern 'id=OnFirstBoot|id=OnBoot' -Quiet))) {
        if ($p.HasExited -or (Get-Date) -gt $deadline) { break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $p.HasExited -and (Get-Date) -le $deadline) {
        Start-Sleep -Seconds 6
        $name = (Get-Process -Id $p.Id).ProcessName
        if ($name -ne 'areka') { throw "pid=$($p.Id) のプロセス名が $name（areka でない）。送らずに止める" }
        & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'real-machine-drive.ps1') -AppPid $p.Id *>&1 | Tee-Object (Join-Path $base "drive-$Run.txt")
    } else { "boot not seen before exit/deadline; drive skipped" | Tee-Object (Join-Path $base "drive-$Run.txt") }
    $p.WaitForExit()
    "$Run exit=$($p.ExitCode) end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')
}
