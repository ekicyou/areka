#Requires -Version 7.0
<#
.SYNOPSIS
  同じテストを規定回数くり返し実行し、各回の出力をファイルへ保存して要約表を作る。

.DESCRIPTION
  要件 9.1-9.4 の反復証跡を作るための道具。手順と読み方は同じフォルダの repeat-tests.md にある。

  判定は 6 値で、緑と数えるのは「緑」だけ:
    打ち切り   … 1 回の上限秒に達したのでプロセス木を止めた（ハングの止め木）
    ビルド失敗 … test result: 行が 1 本も出なかった（コンパイルが通っていない）
    赤         … failed が 1 件以上、または終了コードが 0 でない
    空振り     … passed が 0（フィルタの綴り誤りは終了コード 0 で通るのでここで捕まえる）
    件数不一致 … passed が期待値と違う（期待値を渡したときだけ）
    緑         … 上のどれでもない

  待機はすべて有界（各回・事前ビルド・停止確認）。無界の待機は置かない。
  終了コード: 全回が「緑」なら 0、そうでなければ 1。
#>
[CmdletBinding()]
param(
    # 対象の名前。既定表は下の $Targets。'custom' のときは -CargoArgs を渡す。
    [Parameter(Mandatory = $true)][string]$Target,
    # 走らせる総回数（同時に走るぶんも 1 回と数える）。
    [int]$Times = 1,
    # 1 巡で同時に起動するプロセス数。これが「負荷」の定義（既定 1 = 負荷なし）。
    [int]$Parallel = 1,
    # 1 回あたりの期待 passed 件数。-1 は「指定なし」。
    [int]$ExpectPassed = -1,
    # ログ名と要約の見出しに使う札。既定は対象名。
    [string]$Tag,
    # 'custom' のときの cargo 引数（-- より前）。
    [string[]]$CargoArgs,
    # -- より後ろに渡す引数（テストフィルタ等）。
    [string[]]$TestArgs,
    [string]$OutDir,
    [string]$RedDir,
    [string]$SummaryPath,
    # 走らせるリポジトリのルート。既定は本スクリプトの位置から上へ探した先。
    # 移行前後の所要時間比較（R-3）で、別ワークツリー（git worktree add）を測るときに渡す。
    [string]$Root,
    # 要約に 1 行そのまま載る自由記述（何のための走行か）。
    [string]$Note,
    # 事前ビルドと実行体の刻印採取を省く（推奨しない）。
    [switch]$SkipPrebuild,
    # 1 回あたりの上限秒。0 = 自動（対象表の単独実測 × 同時プロセス数 × 10、下限 120 秒）。
    # 上限に達した回はプロセス木を止めて「打ち切り」として記録する（緑にはしない）。
    [int]$TimeoutSec = 0,
    # i686 成果物の在否検査を省く。
    [switch]$SkipI686Check
)

$ErrorActionPreference = 'Stop'
$LF = [string][char]10
$BQ = [string][char]96
$TAB = [string][char]9

# ---- 対象表 -------------------------------------------------------------
# Expect は「その時点の実測値」であって不変量ではない。テストが増減したら
# cargo test <対象> -- --list で採り直して更新する（repeat-tests.md §2）。
# Solo は -Parallel 1 での実測所要秒（2026-08-24）。上限秒の自動算出にだけ使う。
$Targets = @{
    'workspace' = @{ Cargo = @('test', '--workspace'); Test = @(); Expect = 5865; Solo = 36.8; Desc = 'ワークスペース全体' }
    'seriko'    = @{ Cargo = @('test', '-p', 'areka-seriko', '--lib'); Test = @(); Expect = 200; Solo = 0.4; Desc = 'areka-seriko の lib テスト（要件 3.7 の存在主張を含む）' }
    'wait'      = @{ Cargo = @('test', '-p', 'areka', '--bins'); Test = @('spine_e2e_sakura_blink_default_off_emits_nothing', 'spine_s4_balloon_free_onboot_completes_without_balloon_face_switch'); Expect = 2; Solo = 1.7; Desc = '有界化した待機 2 テスト（要件 4）' }
    'wintf'     = @{ Cargo = @('test', '-p', 'wintf', '--lib'); Test = @(); Expect = 842; Solo = 1.6; Desc = 'wintf の lib テスト（錠を退役させた crate・要件 7.2）' }
    'kit'       = @{ Cargo = @('test', '-p', 'log-capture-kit'); Test = @(); Expect = 79; Solo = 2.4; Desc = '共有 crate log-capture-kit の全テスト（試走用の小さい対象）' }
}
# 上限秒の下限と、単独実測に掛ける係数。上限は性能の合否ではなく「ハングの止め木」なので
# 大きめに取る（既定は期待所要のおよそ 10 倍）。
$TimeoutFloorSec = 120
$TimeoutFactor = 10
# 対象表に無い custom は所要の見当が付かないので、明示指定が無ければこの値。
$TimeoutCustomSec = 1800

if ($Target -ne 'custom' -and -not $Targets.ContainsKey($Target)) {
    throw "対象 '$Target' は未定義。使えるのは: $(($Targets.Keys | Sort-Object) -join ', '), custom"
}
if ($Times -lt 1) { throw '-Times は 1 以上' }
if ($Parallel -lt 1) { throw '-Parallel は 1 以上' }

if ($Target -eq 'custom') {
    if (-not $CargoArgs -or $CargoArgs.Count -eq 0) {
        throw '-Target custom には -CargoArgs が要る（例: -CargoArgs test,-p,wintf,--lib）'
    }
    $cargoPart = @($CargoArgs)
    $testPart = if ($TestArgs) { @($TestArgs) } else { @() }
    $desc = 'custom'
    $soloSec = 0.0
    $timeoutBasis = "custom の既定 $TimeoutCustomSec 秒（単独実測が無いため）"
}
else {
    $entry = $Targets[$Target]
    $cargoPart = @($entry.Cargo)
    $testPart = if ($TestArgs) { @($TestArgs) } else { @($entry.Test) }
    $desc = $entry.Desc
    $soloSec = [double]$entry.Solo
    # 明示的に渡されたときは（-1 = 指定なし を含めて）そちらを優先する。
    # 移行前ツリーの計測（repeat-tests.md §7）は -ExpectPassed -1 で表の値を無効にする。
    if (-not $PSBoundParameters.ContainsKey('ExpectPassed')) { $ExpectPassed = [int]$entry.Expect }
}
if (-not $Tag) { $Tag = $Target }

# ---- 1 回あたりの上限（ハングの止め木・要件 4 と同じ規律を検証側にも当てる）----
if ($TimeoutSec -gt 0) {
    $timeoutBasis = "-TimeoutSec で明示指定"
}
elseif ($soloSec -gt 0) {
    $auto = [int][Math]::Ceiling($soloSec * $Parallel * $TimeoutFactor)
    $TimeoutSec = [Math]::Max($TimeoutFloorSec, $auto)
    $timeoutBasis = "自動＝単独実測 $soloSec 秒 × 同時 $Parallel × $TimeoutFactor（下限 $TimeoutFloorSec 秒）"
}
else {
    $TimeoutSec = $TimeoutCustomSec
}
$timeoutMs = $TimeoutSec * 1000

# ---- リポジトリのルートを探す -------------------------------------------
# 完了アーカイブ（completed/ への移動）で階層が 1 段変わるので、相対の段数を
# 固定せず Cargo.toml と crates/ を持つ祖先まで登る。
if ($Root) {
    $root = (Resolve-Path $Root).Path
    if (-not ((Test-Path (Join-Path $root 'Cargo.toml')) -and (Test-Path (Join-Path $root 'crates')))) {
        throw "-Root '$root' に Cargo.toml と crates/ が無い"
    }
}
else {
    $root = $PSScriptRoot
    while ($root -and -not ((Test-Path (Join-Path $root 'Cargo.toml')) -and (Test-Path (Join-Path $root 'crates')))) {
        $parent = Split-Path -Parent $root
        if ($parent -eq $root) { break }
        $root = $parent
    }
    if (-not $root -or -not (Test-Path (Join-Path $root 'crates'))) {
        throw "リポジトリのルートが見つからない（$PSScriptRoot から上へ Cargo.toml と crates/ を探した）"
    }
}

if (-not $OutDir) { $OutDir = Join-Path $PSScriptRoot 'logs' }
if (-not $RedDir) { $RedDir = Join-Path $PSScriptRoot 'red' }
if (-not $SummaryPath) { $SummaryPath = Join-Path $PSScriptRoot 'summary.md' }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# ---- 前提の検査 ---------------------------------------------------------
# ワークスペース全体のテストは 32bit 側の成果物が要る（無いと当該テストが落ちる）。
if (-not $SkipI686Check) {
    $i686 = Join-Path $root 'target/i686-pc-windows-msvc/debug'
    $need = @('shiori-host32-helper.exe', 'shiori.dll')
    $miss = @($need | Where-Object { -not (Test-Path (Join-Path $i686 $_)) })
    if ($miss.Count -gt 0) {
        $msg = "i686 の成果物が無い: $($miss -join ', ')（$i686）" + $LF +
        '先に次を実行すること:' + $LF +
        '  cargo build -p shiori-host32-helper --target i686-pc-windows-msvc' + $LF +
        '  cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc' + $LF +
        '検査を外すなら -SkipI686Check（外した事実は要約に残る）。'
        throw $msg
    }
}

$env:CARGO_TERM_COLOR = 'never'

$headSha = (& git -C $root rev-parse --short HEAD 2>$null)
$dirty = @(& git -C $root status --porcelain 2>$null)
$dirtyNote = if ($dirty.Count -eq 0) { 'clean' } else { "dirty（$($dirty.Count) 件）" }
$cargoVer = (& cargo --version 2>$null)

# 要約は履歴へ残るので、記録する実行コマンドから環境固有の接頭辞を伏せる。
# （OS アカウント名やその場限りの一時ディレクトリ名がリポジトリへ入らないようにする）
function Format-Portable([string]$s) {
    if (-not $s) { return $s }
    $pairs = @()
    foreach ($k in 'TEMP', 'TMP', 'USERPROFILE') {
        $v = [Environment]::GetEnvironmentVariable($k)
        if ($v) { $pairs += , @($v, ('<' + $k.ToLowerInvariant() + '>')) }
    }
    # 長い接頭辞から先に当てる（TEMP は USERPROFILE の下にあることが多い）
    $pairs = @($pairs | Sort-Object { - $_[0].Length })
    foreach ($p in $pairs) {
        $s = $s.Replace($p[0], $p[1])
        $s = $s.Replace($p[0].Replace('\', '/'), $p[1])
    }
    return $s
}

$runArgs = @($cargoPart)
if ($testPart.Count -gt 0) { $runArgs += '--'; $runArgs += $testPart }
$cmdText = Format-Portable ('cargo ' + ($runArgs -join ' '))

# ---- 事前ビルドと実行体の刻印 -------------------------------------------
# 目的は 2 つ。⑴ 各回の所要時間にコンパイル時間を混ぜない。
# ⑵ どの実行体を測ったかをパス／サイズ／更新時刻で残す（glob で拾った古い実行体を
#    黙って測る事故・変異を戻しても mtime が据え置きで再ビルドされない事故の検出）。
$prebuildSec = $null
$binLines = @()
$binCountResolved = 0
if (-not $SkipPrebuild) {
    $jsonPath = Join-Path $OutDir "$Tag-prebuild.json"
    $prebuildErr = Join-Path $OutDir "$Tag-prebuild.err.log"
    $pbArgs = @($cargoPart) + @('--no-run', '--message-format=json')
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $pb = Start-Process -FilePath 'cargo' -ArgumentList $pbArgs -WorkingDirectory $root -NoNewWindow -PassThru -RedirectStandardOutput $jsonPath -RedirectStandardError $prebuildErr
    # 事前ビルドも有界で待つ（ここが無界だと 1 回のハングで全体が止まり記録も残らない）
    if (-not $pb.WaitForExit($timeoutMs)) {
        try { $pb.Kill($true) } catch { }
        throw "事前ビルドが上限 $TimeoutSec 秒（$timeoutBasis）に達したので打ち切った。$prebuildErr を見ること。"
    }
    $sw.Stop()
    $prebuildSec = [Math]::Round($sw.Elapsed.TotalSeconds, 1)
    if ($pb.ExitCode -ne 0) {
        throw "事前ビルドが失敗した（終了コード $($pb.ExitCode)）。$prebuildErr を見ること。"
    }
    foreach ($line in [IO.File]::ReadLines($jsonPath)) {
        if (-not $line.StartsWith('{')) { continue }
        try { $obj = $line | ConvertFrom-Json } catch { continue }
        if ($obj.PSObject.Properties.Name -notcontains 'executable') { continue }
        if (-not $obj.executable) { continue }
        if (-not $obj.profile -or -not $obj.profile.test) { continue }
        $binLines += $obj.executable
    }
    $binLines = @($binLines | Sort-Object -Unique)
    $binCountResolved = $binLines.Count
    $stampPath = Join-Path $OutDir "$Tag-binaries.txt"
    $stamp = @(
        "# $Tag の事前ビルドで解決したテスト実行体（cargo test --no-run --message-format=json 由来）",
        "# 採取 $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')・HEAD $headSha・$binCountResolved 本",
        '# 列: サイズ<TAB>更新時刻(UTC)<TAB>パス'
    )
    foreach ($b in $binLines) {
        if (Test-Path $b) {
            $fi = Get-Item $b
            $stamp += ($fi.Length.ToString() + $TAB + $fi.LastWriteTimeUtc.ToString('s') + $TAB + $b)
        }
        else { $stamp += ('-' + $TAB + '-' + $TAB + $b + '（不在）') }
    }
    [IO.File]::WriteAllText($stampPath, (($stamp -join $LF) + $LF))
}

# ---- 反復 ---------------------------------------------------------------
$resultRe = [regex]'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out'
$failedRe = [regex]'^test (.+) \.\.\. FAILED\s*$'
$stdoutRe = [regex]'^---- (.+) stdout ----\s*$'

# 打ち切りの直後は、止めたプロセスのリダイレクト先ハンドルが OS 側でまだ開いていることがある。
# 有界（10 回 × 200ms）で読み直し、それでも読めなければ読めなかった旨を本文に残す。
# 読んだ本文は Format-Portable を通す＝要約と red/ に入るのは環境固有の接頭辞を伏せた形になる
# （一時ディレクトリ名や OS アカウント名を履歴へ入れない）。生ログ logs/ は無加工のまま残る。
function Read-OneLog([string]$path) {
    if (-not (Test-Path $path)) { return '' }
    for ($k = 0; $k -lt 10; $k++) {
        try { return (Format-Portable ([IO.File]::ReadAllText($path))) }
        catch { Start-Sleep -Milliseconds 200 }
    }
    return "（このログはハンドルが解放されず読めなかった: $(Format-Portable $path)）"
}

function Read-RunText([string]$outPath, [string]$errPath) {
    $t = Read-OneLog $outPath
    $t += ([string][char]10) + (Read-OneLog $errPath)
    return $t
}

function Get-RunFacts([string]$text) {
    $nl = [string][char]10
    $lines = $text -split '\r?\n'
    $passed = 0; $failed = 0; $ignored = 0; $filtered = 0; $bins = 0
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($l in $lines) {
        $m = $script:resultRe.Match($l)
        if ($m.Success) {
            $bins++
            $passed += [int]$m.Groups[2].Value
            $failed += [int]$m.Groups[3].Value
            $ignored += [int]$m.Groups[4].Value
            $filtered += [int]$m.Groups[6].Value
            continue
        }
        $f = $script:failedRe.Match($l)
        if ($f.Success) { $names.Add($f.Groups[1].Value.Trim()) }
    }
    # 失敗の本文（---- <名前> stdout ---- から次の区切りまで）を最大 60 行で抜く
    $blocks = New-Object System.Collections.Generic.List[object]
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $s = $script:stdoutRe.Match($lines[$i])
        if (-not $s.Success) { continue }
        $name = $s.Groups[1].Value.Trim()
        $body = New-Object System.Collections.Generic.List[string]
        for ($j = $i + 1; $j -lt $lines.Count; $j++) {
            $lj = $lines[$j]
            if ($lj -like '---- *' -or $lj -eq 'failures:' -or $lj.StartsWith('test result:')) { break }
            $body.Add($lj)
        }
        $trimmed = $false
        $keep = @($body)
        if ($keep.Count -gt 60) { $keep = @($keep[0..59]); $trimmed = $true }
        $blocks.Add([pscustomobject]@{ Name = $name; Body = (($keep -join $nl).TrimEnd()); Trimmed = $trimmed })
    }
    return [pscustomobject]@{
        Passed = $passed; Failed = $failed; Ignored = $ignored; Filtered = $filtered
        Bins   = $bins; Names = @($names | Sort-Object -Unique); Blocks = $blocks
    }
}

$runs = New-Object System.Collections.Generic.List[object]
$runIndex = 0
$remaining = $Times
Write-Host "[$Tag] $cmdText を $Times 回（同時 $Parallel プロセス）"
while ($remaining -gt 0) {
    $n = [Math]::Min($Parallel, $remaining)
    $batch = New-Object System.Collections.Generic.List[object]
    for ($i = 0; $i -lt $n; $i++) {
        $runIndex++
        $outPath = Join-Path $OutDir ('{0}-r{1:d3}.out.log' -f $Tag, $runIndex)
        $errPath = Join-Path $OutDir ('{0}-r{1:d3}.err.log' -f $Tag, $runIndex)
        $p = Start-Process -FilePath 'cargo' -ArgumentList $runArgs -WorkingDirectory $root -NoNewWindow -PassThru -RedirectStandardOutput $outPath -RedirectStandardError $errPath
        $batch.Add([pscustomobject]@{
                Index   = $runIndex; Proc = $p; Out = $outPath; Err = $errPath
                Start   = (Get-Date); Sw = [Diagnostics.Stopwatch]::StartNew()
                TimedOut = $false; KillNote = ''
            })
    }
    foreach ($b in $batch) {
        # 有界待機。上限に達したらプロセス木ごと止めて「打ち切り」として記録する
        # （自分が起こしたプロセスだけを止める。無界の待機はこの spec の主題そのものなので、
        #   検証ハーネス側にも同じ規律を当てる）。
        if (-not $b.Proc.WaitForExit($timeoutMs)) {
            $b.TimedOut = $true
            try {
                $b.Proc.Kill($true)
                $b.KillNote = '上限に達したのでプロセス木を停止した'
            }
            catch {
                $b.KillNote = "上限に達したが停止に失敗した: $($_.Exception.Message)"
            }
            # 停止の完了だけをさらに有界で待つ（ここも無界にしない）
            if (-not $b.Proc.WaitForExit(30000)) {
                $b.KillNote += '／停止後 30 秒たっても終了を確認できなかった'
            }
        }
        $b.Sw.Stop()
        $facts = Get-RunFacts (Read-RunText $b.Out $b.Err)
        $exit = try { $b.Proc.ExitCode } catch { -1 }
        $verdict =
        if ($b.TimedOut) { '打ち切り' }
        elseif ($facts.Bins -eq 0) { 'ビルド失敗' }
        elseif ($facts.Failed -gt 0 -or $exit -ne 0) { '赤' }
        elseif ($facts.Passed -eq 0) { '空振り' }
        elseif ($ExpectPassed -ge 0 -and $facts.Passed -ne $ExpectPassed) { '件数不一致' }
        else { '緑' }
        $runs.Add([pscustomobject]@{
                Index    = $b.Index; Start = $b.Start; Sec = [Math]::Round($b.Sw.Elapsed.TotalSeconds, 1)
                Exit     = $exit; Passed = $facts.Passed; Failed = $facts.Failed; Ignored = $facts.Ignored
                Filtered = $facts.Filtered; Bins = $facts.Bins; Verdict = $verdict
                Names    = $facts.Names; Blocks = $facts.Blocks
                Out      = $b.Out; Err = $b.Err
                TimedOut = $b.TimedOut; KillNote = $b.KillNote
            })
        Write-Host ('  r{0:d3} {1} 終了{2} passed={3} failed={4} {5}s' -f $b.Index, $verdict, $exit, $facts.Passed, $facts.Failed, [Math]::Round($b.Sw.Elapsed.TotalSeconds, 1))
        # 赤・打ち切りの回は生ログを追跡対象の red/ へ複写する（logs/ は非追跡なので消えても残る）
        if ($verdict -eq '赤' -or $verdict -eq 'ビルド失敗' -or $verdict -eq '打ち切り') {
            New-Item -ItemType Directory -Force -Path $RedDir | Out-Null
            # 打ち切り直後はハンドルが残ることがあるので、読み直した本文を書き出す
            # （Copy-Item だと同じ理由で失敗し得る）。
            [IO.File]::WriteAllText((Join-Path $RedDir (Split-Path -Leaf $b.Out)), (Read-OneLog $b.Out))
            $errText = Read-OneLog $b.Err
            if ($errText.Trim().Length -gt 0) {
                [IO.File]::WriteAllText((Join-Path $RedDir (Split-Path -Leaf $b.Err)), $errText)
            }
        }
    }
    $remaining -= $n
}

# ---- 要約の追記 ---------------------------------------------------------
$counts = @{}
foreach ($v in '緑', '赤', '空振り', '件数不一致', 'ビルド失敗', '打ち切り') {
    $counts[$v] = @($runs | Where-Object { $_.Verdict -eq $v }).Count
}
$secs = @($runs | ForEach-Object { $_.Sec } | Sort-Object)
$median =
if ($secs.Count -eq 0) { 0 }
elseif ($secs.Count % 2 -eq 1) { $secs[[int][Math]::Floor($secs.Count / 2)] }
else { [Math]::Round(($secs[[int]($secs.Count / 2) - 1] + $secs[[int]($secs.Count / 2)]) / 2, 1) }

$expectText = if ($ExpectPassed -ge 0) { "$ExpectPassed" } else { '指定なし' }
$prebuildText =
if ($SkipPrebuild) { '省略（-SkipPrebuild）' }
else { "$prebuildSec 秒・テスト実行体 $binCountResolved 本（刻印 logs/$Tag-binaries.txt）" }
$i686Text = if ($SkipI686Check) { '**省略（-SkipI686Check）**' } else { '実施' }

$out = New-Object System.Collections.Generic.List[string]
$out.Add('')
$out.Add("## $Tag — $desc ×$Times（同時 $Parallel プロセス）")
$out.Add('')
$out.Add('| 項目 | 値 |')
$out.Add('|---|---|')
$out.Add("| 実行日時 | $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') |")
$out.Add("| 走行ルート | $BQ$root$BQ |")
$out.Add("| HEAD | $BQ$headSha$BQ（作業ツリー $dirtyNote） |")
$out.Add("| 実行コマンド | $BQ$cmdText$BQ |")
$out.Add("| 回数 / 同時プロセス | $Times / $Parallel |")
$out.Add("| 期待 passed | $expectText |")
$out.Add("| 1 回の上限 | $TimeoutSec 秒（$timeoutBasis） |")
$out.Add("| 事前ビルド | $prebuildText |")
$out.Add("| i686 成果物の検査 | $i686Text |")
$out.Add("| cargo | $cargoVer |")
if ($Note) { $out.Add("| 備考 | $Note |") }
$out.Add('')
$out.Add('| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |')
$out.Add('|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|')
foreach ($r in $runs) {
    $row = '| {0} | {1} | {2} | {3} | {4} | {5} | {6} | {7} | {8} | {9} | {10}{11}{10} |' -f
    $r.Index, $r.Start.ToString('HH:mm:ss'), $r.Sec, $r.Exit, $r.Passed, $r.Failed, $r.Ignored,
    $r.Filtered, $r.Bins, $r.Verdict, $BQ, (Split-Path -Leaf $r.Out)
    $out.Add($row)
}
$out.Add('')
$minSec = if ($secs.Count) { $secs[0] } else { 0 }
$maxSec = if ($secs.Count) { $secs[-1] } else { 0 }
$out.Add("**$Times 回走らせて 緑 $($counts['緑'])・赤 $($counts['赤'])・空振り $($counts['空振り'])・件数不一致 $($counts['件数不一致'])・ビルド失敗 $($counts['ビルド失敗'])・打ち切り $($counts['打ち切り'])**（所要秒 中央値 $median / 最小 $minSec / 最大 $maxSec）")

$bad = @($runs | Where-Object { $_.Verdict -ne '緑' })
if ($bad.Count -gt 0) {
    $out.Add('')
    $out.Add('### 緑でなかった回の内訳')
    foreach ($r in $bad) {
        $out.Add('')
        $out.Add("- **回 $($r.Index)・判定 $($r.Verdict)**（終了コード $($r.Exit)・passed $($r.Passed)・failed $($r.Failed)・filtered out $($r.Filtered)・ログ $BQ$(Split-Path -Leaf $r.Out)$BQ）")
        if ($r.TimedOut) {
            $out.Add("  - **上限 $TimeoutSec 秒に達したので打ち切った**（$timeoutBasis）。$($r.KillNote)。")
            $out.Add('  - 打ち切りの回の出力は途中までしか無い。上限が短すぎたのか本当にハングしたのかは生ログの最終行で判断すること（理由の分からない打ち切りを残さない）。')
        }
        if ($r.Names.Count -gt 0) {
            $out.Add("  - 失敗したテスト（$($r.Names.Count) 件）:")
            foreach ($nm in $r.Names) { $out.Add("    - $BQ$nm$BQ") }
        }
        elseif ($r.Verdict -eq '赤') {
            $out.Add('  - 失敗したテスト名が出力から採れなかった（終了コードだけが非 0）。生ログを直接読むこと。')
        }
        foreach ($blk in $r.Blocks) {
            $out.Add('')
            $out.Add("  失敗内容 $BQ$($blk.Name)${BQ}:")
            $out.Add('')
            $out.Add('  ' + ($BQ * 3))
            foreach ($bl in ($blk.Body -split '\n')) { $out.Add('  ' + $bl) }
            if ($blk.Trimmed) { $out.Add('  …（60 行で切った。全文は生ログ）') }
            $out.Add('  ' + ($BQ * 3))
        }
    }
}
$out.Add('')

if (-not (Test-Path $SummaryPath)) {
    $header = @(
        '# 反復実行の記録（要件 9.1-9.4）',
        '',
        'repeat-tests.ps1 が 1 回の走行につき 1 節を追記する。読み方と負荷の定義は repeat-tests.md。',
        '生ログは logs/（非追跡・再生成できる）、赤の回の生ログだけ red/（追跡）へ複写される。',
        ''
    ) -join $LF
    [IO.File]::WriteAllText($SummaryPath, $header + $LF)
}
# 追記の改行は既存本文に合わせる。core.autocrlf のチェックアウトで summary.md が CRLF に
# なっている場合に LF を足すと、混在した改行のファイルになる（本 spec が繰り返し踏んだ罠）。
$existing = [IO.File]::ReadAllBytes($SummaryPath)
$crlfSeen = 0
$bareLfSeen = 0
for ($i = 0; $i -lt $existing.Length; $i++) {
    if ($existing[$i] -eq 10) {
        if ($i -gt 0 -and $existing[$i - 1] -eq 13) { $crlfSeen++ } else { $bareLfSeen++ }
    }
}
$sep = if ($crlfSeen -gt $bareLfSeen) { [string][char]13 + $LF } else { $LF }
[IO.File]::AppendAllText($SummaryPath, (($out -join $sep) + $sep))

Write-Host ''
Write-Host "[$Tag] $Times 回: 緑 $($counts['緑'])・赤 $($counts['赤'])・空振り $($counts['空振り'])・件数不一致 $($counts['件数不一致'])・ビルド失敗 $($counts['ビルド失敗'])・打ち切り $($counts['打ち切り'])"
Write-Host "[$Tag] 要約を追記: $SummaryPath"
if ($counts['緑'] -eq $Times) { exit 0 } else { exit 1 }
