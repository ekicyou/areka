<#
.SYNOPSIS
  collisionex-regions タスク 6.1（実機でクローディアの矩形の名前が届くことを確かめる）の根づくりと起動。
  完了 areka-P0-mouse-drag-events の同名の道具の写し（根の場所・走行の名前・自動終了・RUST_LOG だけを変えた）。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 claudia を nar-sample-path で展開して
             ワークツリーの target\cxr-signoff\root に写す。製品のコードとリポジトリの検体（.nar）には触れない。
  -Run Rn  : 根で areka を起こし、終わるまで待つ。記録は target\cxr-signoff\run-Rn.log。
             起動の前に記憶（profile）を消す。RUST_LOG は当たり判定の名前を決める hit_region_client の debug! まで開ける。
             自動終了は既定 5 分（途中でやめるときはメニューの「終了」でよい）。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/completed/collisionex-regions/real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File .kiro/specs/completed/collisionex-regions/real-machine-run.ps1 -Run R1
#>
param(
    [switch]$Prepare,
    [ValidateSet('R1')]
    [string]$Run,
    [int]$ExitMs = 300000,
    [string]$RustLog = 'info,areka=debug,kanade=trace,wintf::ecs::drag=debug,areka_emo_present::presenter::hit=debug'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$base = Join-Path $wt 'target\cxr-signoff'
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
    # デバッグ版はコンソールの実行ファイル。出力はファイルへ回すので、空のコンソール窓は隠す。
    $log = Join-Path $base "run-$Run.log"
    $p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$ghost`"" -WorkingDirectory $root `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $base "run-$Run.err.log") -WindowStyle Hidden -PassThru
    "$Run pid=$($p.Id) start=$(Get-Date -Format o) exit_ms=$ExitMs" | Tee-Object -Append (Join-Path $base 'runs.txt')
    $p.WaitForExit()
    "$Run exit=$($p.ExitCode) end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')
}
