<#
.SYNOPSIS
  areka-P0-char-position-save-on-exit タスク 6.2（実機 emo2 で確かめる）の根づくり・起動・判定。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2 を nar-sample-path で展開して
             ワークツリーの target\cpsoe に写す。製品のコードとリポジトリの検体には触れない。
  -Run Rn  : target\cpsoe で areka を起こし、終わるまで待ってから -Check Rn を行う。記録は target\cpsoe\run-Rn.log。
             R1 は起動の前に記憶（profile とゴーストの profile\areka）を消す。R2・R3 は前の回の記憶を残す。
             終わると記憶ファイル sylphya.toml を target\cpsoe\after-Rn.toml に写す。
             自動終了は 15 分（ふつうはメニューの「終了」で閉じる）。
  -Check Rn: 記録と記憶ファイルを検索して判定だけを行う（走らせ直さない）。不合格があれば終了コード 1。

  開発者の操作:
    R1: 起動して落ち着いたら、本体（\0）だけをドラッグして離す → メニューの「終了」。
    R2: 起動したら、相方が R1 で閉じたときの位置に立っているかを目で見る → 「終了」。
    R3: R2 と同じ（並びが変わらないかを見る）→ 「終了」。
  拡大率（DPI）は変えない（拡大率の詰め直しの後退は areka-P0-dpi-realign-remembered-chain の持ち分）。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/areka-P0-char-position-save-on-exit/real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File .kiro/specs/areka-P0-char-position-save-on-exit/real-machine-run.ps1 -Run R1
#>
param(
    [switch]$Prepare,
    [ValidateSet('R1', 'R2', 'R3')]
    [string]$Run,
    [ValidateSet('R1', 'R2', 'R3')]
    [string]$Check,
    [int]$ExitMs = 900000,
    [string]$RustLog = 'info,areka::persist::save=info,areka::persist::restore=info'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$root = Join-Path $wt 'target\cpsoe'
$ghost = Join-Path $root 'ghost\emo2'
$memory = Join-Path $ghost 'ghost\master\profile\areka\sylphya.toml'

function Machine([string]$path) {
    $b = [IO.File]::ReadAllBytes($path)
    $pe = [BitConverter]::ToInt32($b, 0x3c)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

if ($Prepare) {
    $free = (Get-PSDrive C).Free
    if ($free -lt 3GB) { throw "C: の空きが $([math]::Round($free / 1GB, 1)) GB しかない（3 GB 未満）——ビルドしない" }
    Push-Location $wt
    try {
        cargo build -p areka --bin areka -j 2; if ($LASTEXITCODE) { throw "areka のビルドが失敗 ($LASTEXITCODE)" }
        cargo build -p shiori-host32-helper --target i686-pc-windows-msvc -j 2; if ($LASTEXITCODE) { throw "helper のビルドが失敗 ($LASTEXITCODE)" }
        $printed = cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2
        if ($LASTEXITCODE) { throw "nar-sample-path が失敗 ($LASTEXITCODE)" }
    } finally { Pop-Location }
    $manual = ($printed | Where-Object { $_ -like 'root=*' }) -replace '^root=', ''
    $helper = Join-Path $wt 'target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe'
    if ((Machine $helper) -ne '014C') { throw "helper が 32bit でない: $helper" }
    # 根のフォルダそのものは消さず中身だけを消す。
    New-Item -ItemType Directory -Force $root | Out-Null
    Get-ChildItem -Force $root | Remove-Item -Recurse -Force
    Copy-Item -Recurse (Join-Path $manual '*') $root
    Copy-Item (Join-Path $wt 'target\debug\areka.exe') $root
    Copy-Item $helper $root
    "prepared $root (helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')) areka=$(Machine (Join-Path $root 'areka.exe')))"
}

if ($Run) {
    if (-not (Test-Path (Join-Path $root 'areka.exe'))) { throw "根が無い——先に -Prepare" }
    if ($Run -eq 'R1') {
        foreach ($p in 'profile', 'tmp', 'ghost\emo2\ghost\master\profile\areka') { $d = Join-Path $root $p; if (Test-Path $d) { Remove-Item -Recurse -Force $d } }
    } elseif (-not (Test-Path $memory)) {
        throw "前の回の記憶が無い: $memory（R1 から回す）"
    }
    New-Item -ItemType Directory -Force (Join-Path $root 'tmp') | Out-Null
    if (Test-Path $memory) { Copy-Item -Force $memory (Join-Path $root "before-$Run.toml") }
    Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
    $env:NO_COLOR = '1'
    $env:RUST_LOG = $RustLog
    $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
    $env:AREKA_ROOT = $root
    $env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
    $env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
    # デバッグ版はコンソールの実行ファイル。出力はファイルへ回すので、空のコンソール窓は隠す。
    $p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$ghost`"" -WorkingDirectory $root `
        -RedirectStandardOutput (Join-Path $root "run-$Run.log") -RedirectStandardError (Join-Path $root "run-$Run.err.log") -WindowStyle Hidden -PassThru
    "$Run pid=$($p.Id) start=$(Get-Date -Format o) exit_ms=$ExitMs" | Tee-Object -Append (Join-Path $root 'runs.txt')
    $p.WaitForExit()
    "$Run exit=$($p.ExitCode) end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $root 'runs.txt')
    if (Test-Path $memory) { Copy-Item -Force $memory (Join-Path $root "after-$Run.toml") }
    $Check = $Run
}

if ($Check) {
    $log = Join-Path $root "run-$Check.log"
    if (-not (Test-Path $log)) { throw "記録が無い: $log" }
    $lines = [IO.File]::ReadAllLines($log)
    $results = [Collections.Generic.List[object]]::new()
    function Judge([string]$name, [bool]$ok, [string]$detail) { $results.Add([pscustomobject]@{ Ok = $ok; Name = $name; Detail = $detail }) }
    function Count([string]$text) { @($lines | Where-Object { $_.Contains($text) }).Count }
    function Scopes([string]$text) { @($lines | Where-Object { $_.Contains($text) -and $_ -match 'scope=(\d+)' } | ForEach-Object { [int]$Matches[1] } | Sort-Object -Unique) -join ',' }
    function Bytes([string]$path) { if (Test-Path $path) { [Convert]::ToBase64String([IO.File]::ReadAllBytes($path)) } else { '' } }

    $wrote = '並べ終えた時点の保存: 書いた'
    $kept = '並べ終えた時点の保存: 記憶に位置があるので書かない'
    $chain = 'chain_finalize: 実表示寸で連鎖を再解決'
    $drag = 'char DragEnd 保存'
    $after = Join-Path $root "after-$Check.toml"

    if ($Check -eq 'R1') {
        Judge 'R1-1 並べ終えた時点で本体と相方の 2 行が書かれる' ((Count $wrote) -eq 2 -and (Scopes $wrote) -eq '0,1') "書いた=$(Count $wrote) 行・scope=$(Scopes $wrote)"
        Judge 'R1-2 ドラッグの確定は本体だけ' ((Count $drag) -ge 1 -and (Scopes $drag) -eq '0') "DragEnd 保存=$(Count $drag) 行・scope=$(Scopes $drag)"
        Judge 'R1-3 記憶に本体と相方の位置がある' ((Test-Path $after) -and ((Get-Content -Raw $after) -match '\[window\.0\]') -and ((Get-Content -Raw $after) -match '\[window\.1\]')) $after
    } else {
        $before = Join-Path $root "before-$Check.toml"
        Judge "$Check-1 並べ直しの行が出ない（相方は前回の位置）" ((Count $chain) -eq 0) "並べ直し=$(Count $chain) 行"
        Judge "$Check-2 並べ終えた時点で書かない（2 スコープとも記憶あり）" ((Count $wrote) -eq 0 -and (Scopes $kept) -eq '0,1') "書いた=$(Count $wrote) 行・記憶あり scope=$(Scopes $kept)"
        Judge "$Check-3 記憶が起動の前後で同じ" ((Bytes $before) -ne '' -and (Bytes $before) -eq (Bytes $after)) "$before ⇔ $after"
    }
    $err = @($lines | Where-Object { $_ -match '^\S+\s+ERROR\s' })
    Judge "$Check-E ERROR 0 行" ($err.Count -eq 0) "ERROR=$($err.Count)"

    $results | ForEach-Object { '{0} {1} — {2}' -f $(if ($_.Ok) { 'PASS' } else { 'FAIL' }), $_.Name, $_.Detail }
    $lines | Where-Object { $_.Contains('merge_scope restore') -or $_.Contains($wrote) -or $_.Contains($kept) -or $_.Contains($drag) -or $_.Contains($chain) } | ForEach-Object { "  $_" }
    $err | Select-Object -First 10 | ForEach-Object { "  ERROR: $_" }
    if ($results | Where-Object { -not $_.Ok }) { 'RESULT: FAIL'; exit 1 } else { 'RESULT: PASS' }
}
