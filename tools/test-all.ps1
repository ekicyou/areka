<#
.SYNOPSIS
  ワークスペースのフルテストを 1 本で回す（kiro-complete の Format／Test／License／移動後ゲートを兼ねる 1 回の正本）。

.DESCRIPTION
  1. sample fetch: URL の行の検体の取り寄せ（tools/sample-fetch.ps1 を別のプロセスで・通信するのはこの段だけ）。
     失敗したら cargo test を回す段（5・7）を skipped (sample fetch failed) として回さない（ほかの段は回す）
  2. i686 ターゲットの導入（入っていれば何もしない）
  3. i686 の成果物（helper・偽 DLL 2 つ）のビルド＝x64 の host32 e2e が spawn／load する
  4. cargo fmt --check（-Format のときは先に cargo fmt --all で整形する）
  5. x64 のワークスペース全テスト（--no-fail-fast・-j 4＝ページング不足 os error 1455 の回避）
  6. 機能 toml を有効にした dola のテスト（cargo test -p dola --features toml・検体を使わないので取り寄せが赤でも回す）
  7. i686 でしか走らないテスト（host-32 系）
  8. crates.io 公開前の確認（tools/crates-io.ps1 の包むだけの形・ネットを使わない）
  9. 道具の文字コードの判定（tools/encoding-check.ps1 を別のプロセスで・ネットを使わない・規則 F でこの台本の Step が見つからない命令の段を赤に記録することも見る）
  10. リリースノートの道具の自分の検査（tools/release-notes.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  11. 俯瞰の HTML の道具の自分の検査（.claude/skills/kiro-next/overview/roadmap-overview.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  12. 隣の helper を揃える道具の自分の検査（tools/sync-i686-helper.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  13. sample fetch check: 取り寄せの段の自分の検査（tools/sample-fetch.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  14. package check: 梱包の道具の自分の検査（tools/package.ps1 -SelfTest を別のプロセスで・コミット済みの検体を読むだけ・ネットを使わない）
  15. exe install check: exe を机へ置く道具の自分の検査（tools/exe-install.selftest.ps1 を別のプロセスで・偽の置き場で回す）
  16. load flake check: 揺れるテストの道具の自分の検査（tools/load-flake.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  17. mouse input check: マウス入力の道具の自分の検査（tools/mouse-input.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  18. app icon check: アプリアイコンの道具の自分の検査（tools/app-icon.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  19. cursor art check: カーソルの絵の道具の自分の検査（tools/cursor-art.ps1 -SelfTest を別のプロセスで・ネットを使わない）
  20. perf loop selftest: 性能改善ループの道具の自分の検査（tools/perf/perf-loop.ps1 selftest を別のプロセスで・いちばん長いので最後）
  21. -License のときだけ cargo deny check と cargo about generate（テストと同時に回すと rustc がメモリ不足で落ちるので最後に直列で）
  22. 最後の段: target\debug の隣の helper を i686 版に揃える（tools/sync-i686-helper.ps1・途中の段が x64 版で上書きしても、終わった後は areka.exe の隣が i686 版）
  段が赤でも止めずに最後まで回し、段ごとの合否を一覧にして、1 つでも赤なら終了コード 1 で終わる。
  一覧には検査したコミットと、始めた時点の未コミットの変更の件数も出す（「直近の緑」で再実行を省けるかの証拠）。

.EXAMPLE
  pwsh -NoProfile -File tools/test-all.ps1
  pwsh -NoProfile -File tools/test-all.ps1 -Format -License   # kiro-complete（アーカイブ後の 1 回）
#>
param([switch]$Format, [switch]$License)

Set-Location (Split-Path $PSScriptRoot -Parent)

# Git Bash から呼ばれると GNU coreutils の link.exe が MSVC の link.exe を遮蔽し i686 のリンクが落ちる。
# link.exe を持つが cl.exe を持たないフォルダ（＝MSVC 以外の link.exe）を、このプロセスの PATH から外す（共通の下回り）。
. (Join-Path $PSScriptRoot 'tool-common.ps1')
$env:PATH = Get-MsvcLinkPath $env:PATH

$i686 = 'i686-pc-windows-msvc'
$results = [ordered]@{}
$head = git rev-parse --short HEAD
$dirty = @(git status --porcelain).Count

function Step([string]$Name, [scriptblock]$Command) {
    Write-Host "`n==> $Name" -ForegroundColor Cyan
    $sw = [Diagnostics.Stopwatch]::StartNew()
    # 見つからない命令は $LASTEXITCODE を書かないので、前の段の値を持ち越さない（0 だと緑・$null だと SKIP になる）
    $global:LASTEXITCODE = -1
    & $Command
    $results[$Name] = [pscustomobject]@{ Code = $LASTEXITCODE; Seconds = [int]$sw.Elapsed.TotalSeconds }
}

# cargo test を回す段だけ: 取り寄せの段が赤なら回さず skipped として載せる（Code = $null）
function TestStep([string]$Name, [scriptblock]$Command) {
    if ($fetchFailed) {
        Write-Host "`n==> $Name skipped (sample fetch failed)" -ForegroundColor Yellow
        $results[$Name] = [pscustomobject]@{ Code = $null; Seconds = 0 }
        return
    }
    Step $Name $Command
}

Step 'sample fetch' { pwsh -NoProfile -File tools/sample-fetch.ps1 }
$fetchFailed = $results['sample fetch'].Code -ne 0
Step 'add i686 target' { rustup target add $i686 }
Step 'build i686 artifacts' { cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target $i686 }
if ($Format) { Step 'cargo fmt (format)' { cargo fmt --all } }
Step 'fmt --check' { cargo fmt --all -- --check }
TestStep 'x64 workspace tests' { cargo test --workspace --no-fail-fast -j 4 }
Step 'dola toml feature tests' { cargo test -p dola --features toml --no-fail-fast -j 4 }
TestStep 'i686 tests (host-32)' { cargo test -p shiori-host32-helper -p shiori-host32-ipc --target $i686 --no-fail-fast }
Step 'crates.io pre-publish check (package only)' { pwsh -NoProfile -File tools/crates-io.ps1 }
Step 'encoding check' { pwsh -NoProfile -File tools/encoding-check.ps1 }
Step 'release notes check' { pwsh -NoProfile -File tools/release-notes.ps1 -SelfTest }
Step 'roadmap overview check' { pwsh -NoProfile -File .claude/skills/kiro-next/overview/roadmap-overview.ps1 -SelfTest }
Step 'i686 helper sync check' { pwsh -NoProfile -File tools/sync-i686-helper.ps1 -SelfTest }
Step 'sample fetch check' { pwsh -NoProfile -File tools/sample-fetch.ps1 -SelfTest }
Step 'package check' { pwsh -NoProfile -File tools/package.ps1 -SelfTest }
Step 'exe install check' { pwsh -NoProfile -File tools/exe-install.selftest.ps1 }
Step 'load flake check' { pwsh -NoProfile -File tools/load-flake.ps1 -SelfTest }
Step 'mouse input check' { pwsh -NoProfile -File tools/mouse-input.ps1 -SelfTest }
Step 'app icon check' { pwsh -NoProfile -File tools/app-icon.ps1 -SelfTest }
Step 'cursor art check' { pwsh -NoProfile -File tools/cursor-art.ps1 -SelfTest }
Step 'perf loop selftest' { pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest }
if ($License) {
    Step 'cargo deny check' { cargo deny check }
    Step 'cargo about generate' { cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md }
}
Step 'i686 helper beside areka.exe' { pwsh -NoProfile -File tools/sync-i686-helper.ps1 }

Write-Host "`n==== results (commit $head, uncommitted changes at start: $dirty) ====" -ForegroundColor Cyan
$failed = 0
foreach ($name in $results.Keys) {
    $r = $results[$name]
    if ($null -eq $r.Code) { Write-Host "SKIP  $name (skipped (sample fetch failed))" -ForegroundColor Yellow; continue }
    $ok = $r.Code -eq 0
    if (-not $ok) { $failed++ }
    Write-Host ('{0}  {1} ({2} s, exit code {3})' -f ($(if ($ok) { 'OK  ' } else { 'FAIL' }), $name, $r.Seconds, $r.Code)) -ForegroundColor $(if ($ok) { 'Green' } else { 'Red' })
}
if ($License) { git diff --quiet -- THIRD-PARTY-NOTICES.md 2>$null }
if ($License -and $LASTEXITCODE) {
    Write-Host "`nTHIRD-PARTY-NOTICES.md has changes (dependencies changed; include it in the commit)" -ForegroundColor Yellow
}
if ($failed) { Write-Host "`nred: $failed step(s)" -ForegroundColor Red; exit 1 }
Write-Host "`nall steps green" -ForegroundColor Green
exit 0
