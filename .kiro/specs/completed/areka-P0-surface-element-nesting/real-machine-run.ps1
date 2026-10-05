<#
.SYNOPSIS
  areka-P0-surface-element-nesting タスク 8.4（実機の確かめ）の根づくり・起動・判定。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2 を nar-sample-path で展開して
             ワークツリーの target\nest に写す。写したシェル（ghost\emo2\shell\master）の中身を
             リポジトリの検体 crates\areka-emo-compose\tests\fixtures\surface-nesting\ の写しに入れ替え、
             検体に無い descript.txt を最小の形で足し、写したゴーストの台本
             （scripts\pasta\shiori\event\second_change.lua）を確かめ用に差し替える。
             製品のコードとリポジトリの検体には触れない。
  -Run     : target\nest で areka を起こし、有界の自動終了（AREKA_APP_SMOKE_EXIT_MS）まで待ち、
             続けて -Check の判定を行う。記録は target\nest\run.log。
  -Check   : target\nest\run.log を検索して判定だけを行う（走らせ直さない）。不合格があれば終了コード 1。

  台本（起動の台詞 1 本）: \0 の面を 8 秒ごとに 0 → 1 → 2 → 30 → 0 → 2 → 1 → 0 と切り替える。
  0・1・2 は子 10（まばたき animation0・random,2）を置く面、30 は子 10 を置かない面（3 段の入れ子の頭）。
  ゴースト自身の台詞（ランダムトーク・マウス・シェル切替など）はこの回のあいだ黙らせる。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/completed/areka-P0-surface-element-nesting/real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File .kiro/specs/completed/areka-P0-surface-element-nesting/real-machine-run.ps1 -Run
  pwsh -NoProfile -File .kiro/specs/completed/areka-P0-surface-element-nesting/real-machine-run.ps1 -Check
#>
param(
    [switch]$Prepare,
    [switch]$Run,
    [switch]$Check,
    [int]$ExitMs = 85000,
    [string]$RustLog = 'info,areka_seriko=info,areka_emo_present::shell_target=info'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
$root = Join-Path $wt 'target\nest'
$shell = Join-Path $root 'ghost\emo2\shell\master'
$log = Join-Path $root 'run.log'
$utf8 = [Text.UTF8Encoding]::new($false)
$segMs = 8000
# 台本で切り替える面の並び（最初の 0 は起動の台詞の頭）。子 10 を置く面と置かない面。
$faces = @(0, 1, 2, 30, 0, 2, 1, 0)
$withChild = @(0, 1, 2)

function Machine([string]$path) {
    $b = [IO.File]::ReadAllBytes($path)
    $pe = [BitConverter]::ToInt32($b, 0x3c)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

function Crlf([string]$text) { $text -replace "(?<!`r)`n", "`r`n" }

$boot = '\0\s[0]入れ子の確かめ。' + (($faces | Select-Object -Skip 1 | ForEach-Object { "\_w[$segMs]\s[$_]" }) -join '') + "\_w[$segMs]\e"

$lua = @"
--- areka-P0-surface-element-nesting タスク 8.4（実機の確かめ）用に、target\nest の写しの中だけで差し替えた台本。
--- 起動の台詞で \0 の面を $segMs ms ごとに切り替える。ゴースト自身の台詞はこの回のあいだ黙らせる。
local REG = require("pasta.shiori.event.register")

local BOOT = [==[$boot]==]
local function quiet(act) return nil end

REG.OnBoot = function(act) return BOOT end
REG.OnFirstBoot = REG.OnBoot
REG.OnGhostChanged = REG.OnBoot
for _, id in ipairs({ "OnTalk", "OnSecondChange", "OnMinuteChange", "OnHourTimeSignal", "OnMouseMove",
    "OnMouseClick", "OnMouseDoubleClick", "OnMouseWheel", "OnChoiceTimeout", "OnShellChanging",
    "OnShellChanged", "OnBalloonChange", "OnGhostChanging", "OnSurfaceChange", "OnSurfaceRestore" }) do
    REG[id] = quiet
end

return REG
"@

$descript = @'
charset,UTF-8
type,shell
name,surface-nesting
// areka-P0-surface-element-nesting タスク 8.4: 検体 surface-nesting を emo2 のシェルとして読ませるための最小の descript.txt
seriko.use_self_alpha,1
seriko.alignmenttodesktop,bottom
'@

if ($Prepare) {
    $free = (Get-PSDrive C).Free
    if ($free -lt 3GB) { throw "C: の空きが $([math]::Round($free / 1GB, 1)) GB しかない（3 GB 未満）——ビルドしない" }
    $env:CARGO_INCREMENTAL = '0'
    Push-Location $wt
    try {
        cargo build -p areka --bin areka -j 4; if ($LASTEXITCODE) { throw "areka のビルドが失敗 ($LASTEXITCODE)" }
        cargo build -p shiori-host32-helper --target i686-pc-windows-msvc -j 4; if ($LASTEXITCODE) { throw "helper のビルドが失敗 ($LASTEXITCODE)" }
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
    # シェルを検体に入れ替える（emo2 の絵と surfaces.txt は残さない）。
    Get-ChildItem -Force $shell | Remove-Item -Recurse -Force
    Copy-Item (Join-Path $wt 'crates\areka-emo-compose\tests\fixtures\surface-nesting\*') $shell
    [IO.File]::WriteAllText((Join-Path $shell 'descript.txt'), (Crlf $descript), $utf8)
    [IO.File]::WriteAllText((Join-Path $root 'ghost\emo2\ghost\master\scripts\pasta\shiori\event\second_change.lua'), (Crlf $lua), $utf8)
    "prepared $root (helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')) areka=$(Machine (Join-Path $root 'areka.exe')))"
    "shell: $((Get-ChildItem -Name $shell) -join ' ')"
    "boot: $boot"
}

if ($Run) {
    if (-not (Test-Path (Join-Path $root 'areka.exe'))) { throw "根が無い——先に -Prepare" }
    $ghost = Join-Path $root 'ghost\emo2'
    # 前の回の記憶を消す（最後に使ったシェルはゴースト側の記憶にも残る）。
    foreach ($p in 'profile', 'tmp', 'ghost\emo2\ghost\master\profile\areka') { $d = Join-Path $root $p; if (Test-Path $d) { Remove-Item -Recurse -Force $d } }
    New-Item -ItemType Directory -Force (Join-Path $root 'tmp') | Out-Null
    Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
    $env:NO_COLOR = '1'
    $env:RUST_LOG = $RustLog
    $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
    $env:AREKA_ROOT = $root
    $env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
    $env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
    # デバッグ版はコンソールの実行ファイル。出力はファイルへ回すので、空のコンソール窓は隠す。
    $p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$ghost`"" -WorkingDirectory $root `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $root 'run.err.log') -WindowStyle Hidden -PassThru
    "pid=$($p.Id) start=$(Get-Date -Format o) exit_ms=$ExitMs rust_log=$RustLog" | Tee-Object -Append (Join-Path $root 'runs.txt')
    $p.WaitForExit()
    "exit=$($p.ExitCode) end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $root 'runs.txt')
    Set-Content (Join-Path $root 'exit.txt') "$($p.ExitCode)"
    $Check = $true
}

if ($Check) {
    if (-not (Test-Path $log)) { throw "記録が無い: $log" }
    $lines = [IO.File]::ReadAllLines($log)
    $results = [Collections.Generic.List[object]]::new()
    function Judge([string]$name, [bool]$ok, [string]$detail) {
        $results.Add([pscustomobject]@{ Ok = $ok; Name = $name; Detail = $detail })
    }
    function Stamp([string]$line) {
        if ($line -match '^(\S+)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s') { [DateTimeOffset]::Parse($Matches[1]) }
    }
    function Has([string]$line, [string[]]$parts) { foreach ($x in $parts) { if (-not $line.Contains($x)) { return $false } }; $true }

    # J1 有界の自動終了で終わった。
    $exitCode = if (Test-Path (Join-Path $root 'exit.txt')) { (Get-Content (Join-Path $root 'exit.txt')).Trim() } else { '?' }
    $smoke = @($lines | Where-Object { (Has $_ @('event="app_exit"', 'origin=Smoke')) })
    Judge 'J1 有界の自動終了（app_exit origin=Smoke・終了コード 0）' (($smoke.Count -ge 1) -and ($exitCode -eq '0')) "app_exit(Smoke)=$($smoke.Count) 行・終了コード=$exitCode"

    # J2 読み込みの回数 N（検体のシェルの一覧の info! が読み込み 1 回に 1 行・採寸の経路でも出る＝起動 1 回で 2 回ありうる）。
    $loads = @($lines | Where-Object { (Has $_ @('shell: シェルの面の画像の一覧が終わった', 'nest\ghost\emo2\shell\master')) })
    $n = $loads.Count
    Judge 'J2 検体のシェルを読み込んだ（N ≥ 1）' ($n -ge 1) "N=$n"

    # J3 警告 3 種が、仕込んだ件ごとに読み込み 1 回につき 1 行ずつ（＝ちょうど N 行）。種類の総数も 6N。
    $missing = 'shell: element定義が指したサーフェスが無いので置かない'
    $cycle = 'shell: element定義の参照が循環するので、先祖へ戻る参照を置かない'
    $inChild = 'shell: 子として置かれたサーフェスの箱は親の中に置かない'
    $expected = @(
        @($missing, 'surface=50', 'element=1', 'target="9999"'),
        @($missing, 'surface=50', 'element=2', 'target="4294967296"'),
        @($cycle, 'surface=60', 'element=1', 'target=60'),
        @($cycle, 'surface=61', 'element=1', 'target=62'),
        @($cycle, 'surface=62', 'element=1', 'target=61'),
        @($inChild, 'parent=71', 'child=70', 'name="fuda"')
    )
    foreach ($e in $expected) {
        $c = @($lines | Where-Object { $_.Contains(' WARN ') -and (Has $_ $e) }).Count
        Judge "J3 warn 1 読み込み 1 行: $($e -join ' ')" (($n -ge 1) -and ($c -eq $n)) "$c 行（期待 N=$n）"
    }
    $all = @($lines | Where-Object { $_.Contains($missing) -or $_.Contains($cycle) -or $_.Contains($inChild) }).Count
    Judge 'J3 入れ子の警告 3 種の総数（6N・仕込んでいない件が 0）' (($n -ge 1) -and ($all -eq 6 * $n)) "$all 行（期待 $(6 * $n)）"

    # J4 \0 の面の切り替えの並び（apply(ShowSurface) の TargetId(0)・続く同じ番号はまとめる）。
    $segs = [Collections.Generic.List[object]]::new()
    foreach ($l in $lines) {
        if ((Has $l @('apply(ShowSurface): 表示・マスクを更新', 'target_id=TargetId(0)')) -and $l -match 'surface_id=(\d+)') {
            $sid = [int]$Matches[1]
            if ($segs.Count -eq 0 -or $segs[$segs.Count - 1].Surface -ne $sid) { $segs.Add([pscustomobject]@{ Surface = $sid; Start = (Stamp $l) }) }
        }
    }
    $seq = ($segs | ForEach-Object { $_.Surface }) -join ','
    Judge 'J4 面の切り替えの並び' ($seq -eq ($faces -join ',')) "実際=$seq 期待=$($faces -join ',')"

    # 子 10 の部品の記録（スコープ 0）。
    $part = { param($kind) @($lines | Where-Object { (Has $_ @("seriko: part $kind", 'scope="0"')) -and $_ -match 'part=10(\s|$)' } | ForEach-Object { Stamp $_ }) }
    $fires = & $part '抽選発火'
    $stops = & $part '停止'
    $end = Stamp ($lines | Where-Object { Stamp $_ } | Select-Object -Last 1)
    if ($seq -eq ($faces -join ',')) {
        for ($i = 0; $i -lt $segs.Count; $i++) {
            $s = $segs[$i]; $next = if ($i + 1 -lt $segs.Count) { $segs[$i + 1].Start } else { $end }
            if ($withChild -contains $s.Surface) {
                # J5 子 10 を置く面のあいだ、部品の抽選発火が続く（切り替えをまたいで途切れない）。
                $c = @($fires | Where-Object { $_ -ge $s.Start -and $_ -lt $next }).Count
                Judge "J5 区間 $($i + 1)（面 $($s.Surface)）で part=10 の抽選発火が 1 行以上" ($c -ge 1) "$c 行"
            } else {
                # J6 子 10 を置かない面のあいだは抽選しない（戻る刻みの記録が絵の差し替えより先に出うるので末尾 500ms は数えない）。
                $c = @($fires | Where-Object { $_ -ge $s.Start -and $_ -lt $next.AddMilliseconds(-500) }).Count
                Judge "J6 区間 $($i + 1)（面 $($s.Surface)）で part=10 の抽選発火が 0 行" ($c -eq 0) "$c 行"
            }
        }
    }
    # J7 発火した再生は停止まで進む（自動終了で切れた最後の 1 回だけ欠けてよい）。
    Judge 'J7 part=10 の停止 ≥ 抽選発火 − 1' (($fires.Count -ge 1) -and ($stops.Count -ge $fires.Count - 1)) "抽選発火=$($fires.Count) 停止=$($stops.Count)"

    # J8 ERROR と、部品の負の番号の warn が 0 行。
    $err = @($lines | Where-Object { $_ -match '^\S+\s+ERROR\s' })
    $neg = @($lines | Where-Object { $_.Contains('seriko: part `-1` 以外の負 surface') })
    Judge 'J8 ERROR 0 行・部品の負の番号の warn 0 行' (($err.Count -eq 0) -and ($neg.Count -eq 0)) "ERROR=$($err.Count) 負の番号=$($neg.Count)"

    $results | ForEach-Object { '{0} {1} — {2}' -f $(if ($_.Ok) { 'PASS' } else { 'FAIL' }), $_.Name, $_.Detail }
    $segs | ForEach-Object { "  区間 面 $($_.Surface) 開始 $($_.Start.ToString('HH:mm:ss.fff'))" }
    $err | Select-Object -First 10 | ForEach-Object { "  ERROR: $_" }
    if ($results | Where-Object { -not $_.Ok }) { 'RESULT: FAIL'; exit 1 } else { 'RESULT: PASS' }
}
