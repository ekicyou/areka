<#
.SYNOPSIS
  ワークスペースのフルテストを 1 本で回す（kiro-complete の Format／Test／License ゲートの正本）。

.DESCRIPTION
  1. i686 ターゲットの導入（入っていれば何もしない）
  2. i686 の成果物（helper・偽 DLL 2 つ）のビルド＝x64 の host32 e2e が spawn／load する
  3. cargo fmt --check（-Format のときは先に cargo fmt --all で整形する）
  4. x64 のワークスペース全テスト（--no-fail-fast・-j 4＝ページング不足 os error 1455 の回避）
  5. i686 でしか走らないテスト（host-32 系）
  6. -License のときだけ cargo deny check と cargo about generate（テストと同時に回すと rustc がメモリ不足で落ちるので最後に直列で）
  段が赤でも止めずに最後まで回し、段ごとの合否を一覧にして、1 つでも赤なら終了コード 1 で終わる。
  一覧には検査したコミットと、始めた時点の未コミットの変更の件数も出す（「直近の緑」で再実行を省けるかの証拠）。

.EXAMPLE
  pwsh -NoProfile -File tools/test-all.ps1
  pwsh -NoProfile -File tools/test-all.ps1 -Format -License   # kiro-complete の DoD ゲート
#>
param([switch]$Format, [switch]$License)

Set-Location (Split-Path $PSScriptRoot -Parent)

# Git Bash から呼ばれると GNU coreutils の link.exe が MSVC の link.exe を遮蔽し i686 のリンクが落ちる。
# link.exe を持つが cl.exe を持たないフォルダ（＝MSVC 以外の link.exe）を、このプロセスの PATH から外す。
$env:PATH = ($env:PATH -split ';' | Where-Object {
        -not $_ -or -not (Test-Path (Join-Path $_ 'link.exe')) -or (Test-Path (Join-Path $_ 'cl.exe'))
    }) -join ';'

$i686 = 'i686-pc-windows-msvc'
$results = [ordered]@{}
$head = git rev-parse --short HEAD
$dirty = @(git status --porcelain).Count

function Step([string]$Name, [scriptblock]$Command) {
    Write-Host "`n==> $Name" -ForegroundColor Cyan
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & $Command
    $results[$Name] = [pscustomobject]@{ Code = $LASTEXITCODE; Seconds = [int]$sw.Elapsed.TotalSeconds }
}

Step 'i686 ターゲット導入' { rustup target add $i686 }
Step 'i686 成果物ビルド' { cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target $i686 }
if ($Format) { Step 'cargo fmt（整形）' { cargo fmt --all } }
Step 'fmt --check' { cargo fmt --all -- --check }
Step 'x64 ワークスペース全テスト' { cargo test --workspace --no-fail-fast -j 4 }
Step 'i686 テスト（host-32 系）' { cargo test -p shiori-host32-helper -p shiori-host32-ipc --target $i686 --no-fail-fast }
if ($License) {
    Step 'cargo deny check' { cargo deny check }
    Step 'cargo about generate' { cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md }
}

Write-Host "`n==== 結果（検査したコミット $head・開始時の未コミットの変更 $dirty 件） ====" -ForegroundColor Cyan
$failed = 0
foreach ($name in $results.Keys) {
    $r = $results[$name]
    $ok = $r.Code -eq 0
    if (-not $ok) { $failed++ }
    Write-Host ('{0}  {1}（{2} 秒・終了コード {3}）' -f ($(if ($ok) { 'OK  ' } else { 'FAIL' }), $name, $r.Seconds, $r.Code)) -ForegroundColor $(if ($ok) { 'Green' } else { 'Red' })
}
if ($License) { git diff --quiet -- THIRD-PARTY-NOTICES.md 2>$null }
if ($License -and $LASTEXITCODE) {
    Write-Host "`nTHIRD-PARTY-NOTICES.md に差分あり（依存が変わった証跡・コミットに含める）" -ForegroundColor Yellow
}
if ($failed) { Write-Host "`n赤 $failed 段" -ForegroundColor Red; exit 1 }
Write-Host "`n全段 緑" -ForegroundColor Green
exit 0
