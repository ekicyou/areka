#Requires -Version 7.0
<#
================================================================================
check-quiet.ps1 — 測定マシンが静かであることの自動確認
  spec: areka-P0-draw-load-parity（要件 1.5 / 2.8・design「計測の道具（tools/perf/）
        → C6 check-quiet.ps1」）

何をするか:
  1. マシン全体の CPU（`\Processor(_Total)\% Processor Time`）を指定秒数・1 秒刻みで採り、
     平均と最大を出す
  2. 既知の重いプロセス名（ビルド・テスト・別の areka）が動いていないかを調べる
     （測定対象の areka は -TargetPid で除外する。自分自身の PID も常に除外する）
  3. 平均が閾値以下 かつ 該当プロセス 0 件 のときだけ「静か」と判定する
  4. 判定と根拠を quiet-<Stage>.txt へ残し、同じ内容を標準出力にも出す

これは README.md §2 の「人が目で確かめる静寂確認」を機械の確認へ置き換えるものである
（要件 2.8＝人に確認を求めない）。**再試行は行わない**——静かでなければ 2 で終わるだけで、
retry_max 回まで待って確かめ直すのは呼び出し側（invoke-perf-run.ps1 の -AutoQuiet ／
perf-loop.ps1）の仕事である（design C6）。

判定に人の目は入らない。閾値は目標定義ファイル（-Goal / -GoalFile）か引数で与える。

--------------------------------------------------------------------------------
較正値・調整値の一覧（変更する場合はここだけを書き換える）
--------------------------------------------------------------------------------
  SCRIPT_VERSION                  本スクリプトの版（出力ファイルの先頭行へ記録される）
  DEFAULT_MACHINE_CPU_MAX_PCT     既定の閾値（マシン全体 CPU の平均・％）。
                                  目標定義 TOML の [quiet] machine_cpu_max_pct と同じ値
  DEFAULT_SAMPLE_SEC              既定の採取秒数（1 秒刻み）＝ [quiet] sample_sec と同じ値
  DEFAULT_HEAVY_PROCESS_NAMES     既定の「重いプロセス名」＝ [quiet] heavy_process_names と同じ
  COUNTER_PATH_MACHINE_CPU        マシン全体 CPU のカウンタ名（英語版 Windows の名称。
                                  名前が翻訳された環境では取得に失敗し 4 で終わる＝README §8）
  WARMUP_SAMPLE_COUNT = 1         捨てる先頭サンプル数。このカウンタの 1 本目は前回採取からの
                                  差分が無く 0 や無意味な値になるため、SampleSec+1 本採って
                                  先頭を捨て、本数ちょうどの実サンプルで平均と最大を出す

終了コード:
  0 … 静か（quiet-<Stage>.txt を残す）
  2 … 静かでない（quiet-<Stage>.txt を残す。CPU 超過 ／ 重いプロセス ／ その両方）
  3 … 引数・前提の不正（Stage/OutDir 欠落・目標定義ファイルが無い・値が不正）
  4 … 計測失敗（性能カウンタが読めない）。分かる範囲を書いた quiet-<Stage>.txt を残す
  ※ -SelfTest のときだけ 0（道具は健全）／1（道具が壊れている）を返す

使い方:
  pwsh -File tools/perf/check-quiet.ps1 -Stage before -OutDir C:\出力先 -Goal draw-load-parity
  pwsh -File tools/perf/check-quiet.ps1 -Stage after  -OutDir C:\出力先 `
      -MachineCpuMaxPct 10.0 -SampleSec 20 -HeavyProcessNames cargo,rustc -TargetPid 1234
  pwsh -File tools/perf/check-quiet.ps1 -SelfTest
================================================================================
#>

[CmdletBinding()]
param(
    # 段階名。出力ファイル名 quiet-<Stage>.txt に使う（例 before / after）
    [string]$Stage,
    # 出力先ディレクトリ（無ければ作る）
    [string]$OutDir,
    # 目標定義の名前。tools/perf/goals/<Goal>.toml の [quiet] を読む
    [string]$Goal,
    # 目標定義ファイルを直接指定する（-Goal の代わり。試験・別所在用）
    [string]$GoalFile,
    # 閾値（マシン全体 CPU の平均・％）。TOML より優先する
    [double]$MachineCpuMaxPct,
    # 採取秒数（1 秒刻み）。TOML より優先する
    [int]$SampleSec,
    # 重いプロセス名の一覧。TOML より優先する
    [string[]]$HeavyProcessNames,
    # 測定対象 areka の PID。重いプロセスの一覧から除外する
    [int]$TargetPid,
    # 道具そのものの点検（文面の決定論・書式・判定・TOML 解釈）
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# =============================================================================
# 較正値・調整値（上部の一覧と対応。変更はここだけ）
# =============================================================================
$SCRIPT_VERSION              = '1.0.0'
$DEFAULT_MACHINE_CPU_MAX_PCT = 10.0
$DEFAULT_SAMPLE_SEC          = 20
$DEFAULT_HEAVY_PROCESS_NAMES = @('cargo', 'rustc', 'rust-analyzer', 'msbuild', 'link', 'cl', 'areka', 'python')
$COUNTER_PATH_MACHINE_CPU    = '\Processor(_Total)\% Processor Time'
$WARMUP_SAMPLE_COUNT         = 1

# 終了コード
$EXIT_QUIET       = 0
$EXIT_SELFTEST_NG = 1
$EXIT_NOT_QUIET   = 2
$EXIT_BAD_ARGS    = 3
$EXIT_MEASURE     = 4

# 判定語・理由語（読む側＝perf-ledger.py などと突き合わせる字面。変更は下流と同時に）
$VERDICT_QUIET  = 'QUIET'
$VERDICT_NOT    = 'NOT_QUIET'
$VERDICT_FAILED = 'MEASURE_FAILED'
$REASON_OK      = 'ok'
$REASON_CPU     = 'cpu_mean_over_threshold'
$REASON_PROC    = 'heavy_process_present'
$REASON_BOTH    = 'both'
$REASON_COUNTER = 'counter_read_failed'

$INV = [Globalization.CultureInfo]::InvariantCulture

# =============================================================================
# 補助関数
# =============================================================================
function Write-Info    { param([string]$Message) [Console]::Out.WriteLine($Message) }
function Write-Problem { param([string]$Message) [Console]::Error.WriteLine($Message) }

function Get-UtcStamp {
    [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.fff', $INV) + 'Z'
}

# 失敗して終了する。理由を必ず表示する（黙って失敗しない）。
function Stop-Bad {
    param([int]$Code, [string]$Message)
    Write-Problem "[check-quiet] 失敗: $Message"
    exit $Code
}

# 小数 1 桁の固定書式。値が無い（計測できなかった）ときは n/a。
function Format-Pct {
    param($Value)
    if ($null -eq $Value) { return 'n/a' }
    return ([double]$Value).ToString('F1', $INV)
}

# 該当プロセス一覧を「pid:name;pid:name」へ。0 件なら「-」。順序は pid 昇順で決定論。
function Format-HeavyList {
    param($List)
    $items = @($List)
    if ($items.Count -eq 0) { return '-' }
    $sorted = @($items | Sort-Object -Property pid, name)
    return (($sorted | ForEach-Object { "$($_.pid):$($_.name)" }) -join ';')
}

# 判定（純関数）。静か＝平均が閾値以下 かつ 該当プロセス 0 件。
function Get-QuietVerdict {
    param($MeanPct, [double]$ThresholdPct, [int]$HeavyCount)
    if ($null -eq $MeanPct) {
        return [pscustomobject]@{ verdict = $VERDICT_FAILED; reason = $REASON_COUNTER; code = $EXIT_MEASURE }
    }
    $cpuOver = ([double]$MeanPct -gt $ThresholdPct)
    $procHit = ($HeavyCount -gt 0)
    if (-not $cpuOver -and -not $procHit) {
        return [pscustomobject]@{ verdict = $VERDICT_QUIET; reason = $REASON_OK; code = $EXIT_QUIET }
    }
    $reason = if ($cpuOver -and $procHit) { $REASON_BOTH } elseif ($cpuOver) { $REASON_CPU } else { $REASON_PROC }
    return [pscustomobject]@{ verdict = $VERDICT_NOT; reason = $reason; code = $EXIT_NOT_QUIET }
}

# 報告の文面を組む（純関数）。同じ入力から必ず同じ文面が出る＝ -SelfTest が固定する。
# 行は key=value・LF 区切り。読む側（perf-ledger.py・judge 系）はこの字面に依存する。
function Format-QuietReport {
    param([hashtable]$D)
    $lines = @(
        "[check-quiet] version=$($D.version)",
        "stage=$($D.stage)",
        "time_utc=$($D.time_utc)",
        "sample_sec=$($D.sample_sec)",
        "machine_cpu_mean_pct=$(Format-Pct $D.mean_pct)",
        "machine_cpu_max_pct=$(Format-Pct $D.max_pct)",
        "threshold_mean_pct=$(Format-Pct $D.threshold_pct)",
        "heavy_process_names=$(@($D.heavy_names) -join ',')",
        "heavy_processes_found=$(@($D.heavy_list).Count)",
        "heavy_process_list=$(Format-HeavyList $D.heavy_list)",
        "target_pid_excluded=$(if ([int]$D.target_pid -gt 0) { [int]$D.target_pid } else { '-' })",
        "verdict=$($D.verdict)",
        "reason=$($D.reason)"
    )
    return (($lines -join "`n") + "`n")
}

# TOML の [quiet] 節だけを読む最小の解釈器（外部モジュールを使わない）。
# 読むのは machine_cpu_max_pct（小数）・sample_sec（整数）・heavy_process_names（文字列配列）。
# 他の節・他のキーは黙って読み飛ばす。引用符の外の # 以降は注釈として落とす。
function Read-QuietSettingsFromToml {
    param([string]$Text)
    $out = @{}
    $inQuiet = $false
    foreach ($raw in ($Text -split "`r?`n")) {
        $line = $raw.Trim()
        if ($line -eq '' -or $line.StartsWith('#')) { continue }
        if ($line -match '^\[([^\]]+)\]\s*$') { $inQuiet = ($Matches[1].Trim() -eq 'quiet'); continue }
        if (-not $inQuiet) { continue }
        if ($line -notmatch '^([A-Za-z0-9_]+)\s*=\s*(.+)$') { continue }
        $key = $Matches[1]
        $val = ($Matches[2]).Trim()

        # 引用符の外にある # 以降を落とす
        $inStr = $false
        $cut = -1
        for ($i = 0; $i -lt $val.Length; $i++) {
            $ch = $val[$i]
            if ($ch -eq '"') { $inStr = -not $inStr }
            elseif ($ch -eq '#' -and -not $inStr) { $cut = $i; break }
        }
        if ($cut -ge 0) { $val = $val.Substring(0, $cut).Trim() }

        switch ($key) {
            'machine_cpu_max_pct' {
                $parsed = 0.0
                if ([double]::TryParse($val, [Globalization.NumberStyles]::Float, $INV, [ref]$parsed)) {
                    $out['machine_cpu_max_pct'] = $parsed
                }
            }
            'sample_sec' {
                $parsedI = 0
                if ([int]::TryParse($val, [Globalization.NumberStyles]::Integer, $INV, [ref]$parsedI)) {
                    $out['sample_sec'] = $parsedI
                }
            }
            'heavy_process_names' {
                $names = @([regex]::Matches($val, '"([^"]*)"') | ForEach-Object { $_.Groups[1].Value })
                if ($names.Count -gt 0) { $out['heavy_process_names'] = $names }
            }
            default { }
        }
    }
    return $out
}

# マシン全体 CPU を Seconds 秒・1 秒刻みで採る。先頭 Warmup 本（暖機）は捨てる。
# 失敗は握り潰さない——例外はそのまま呼び手（try/catch）へ返す。
function Measure-MachineCpu {
    param([int]$Seconds, [string]$CounterPath, [int]$Warmup)
    $set = Get-Counter -Counter $CounterPath -SampleInterval 1 -MaxSamples ($Seconds + $Warmup) -ErrorAction Stop
    $values = @($set | ForEach-Object { [double]$_.CounterSamples[0].CookedValue })
    if ($values.Count -le $Warmup) { throw "採取本数が足りません（$($values.Count) 本）" }
    return @($values[$Warmup..($values.Count - 1)])
}

# =============================================================================
# 道具そのものの点検（-SelfTest）
# =============================================================================
function Invoke-SelfTest {
    $ng = @()

    # (1) 文面の決定論＋書式：固定の入力から期待どおりの字面が出ること
    $fixture = @{
        version       = $SCRIPT_VERSION
        stage         = 'selftest'
        time_utc      = '2026-01-02T03:04:05.678Z'
        sample_sec    = 20
        mean_pct      = 4.24
        max_pct       = 9.87
        threshold_pct = 10.0
        heavy_names   = @('cargo', 'rustc')
        heavy_list    = @([pscustomobject]@{ pid = 4321; name = 'rustc' }, [pscustomobject]@{ pid = 1234; name = 'cargo' })
        target_pid    = 9999
        verdict       = $VERDICT_NOT
        reason        = $REASON_PROC
    }
    $expected = (@(
        "[check-quiet] version=$SCRIPT_VERSION",
        'stage=selftest',
        'time_utc=2026-01-02T03:04:05.678Z',
        'sample_sec=20',
        'machine_cpu_mean_pct=4.2',
        'machine_cpu_max_pct=9.9',
        'threshold_mean_pct=10.0',
        'heavy_process_names=cargo,rustc',
        'heavy_processes_found=2',
        'heavy_process_list=1234:cargo;4321:rustc',
        'target_pid_excluded=9999',
        'verdict=NOT_QUIET',
        'reason=heavy_process_present'
    ) -join "`n") + "`n"
    $actual = Format-QuietReport $fixture
    if ($actual -cne $expected) {
        $ng += "文面が期待と一致しません。`n--- 期待 ---`n$expected--- 実際 ---`n$actual"
    }
    if ((Format-QuietReport $fixture) -cne $actual) { $ng += '同じ入力から違う文面が出ました（決定論の破れ）。' }

    # (2) 該当 0 件・PID 未指定・丸めの表記
    $empty = @{
        version     = $SCRIPT_VERSION; stage = 'x'; time_utc = 'T'; sample_sec = 5
        mean_pct    = 0.04; max_pct = 0.1; threshold_pct = 1.0
        heavy_names = @('cargo'); heavy_list = @(); target_pid = 0
        verdict     = $VERDICT_QUIET; reason = $REASON_OK
    }
    $emptyText = Format-QuietReport $empty
    if ($emptyText -notmatch 'heavy_process_list=-\n')     { $ng += '該当 0 件が「-」になりません。' }
    if ($emptyText -notmatch 'target_pid_excluded=-\n')    { $ng += 'PID 未指定が「-」になりません。' }
    if ($emptyText -notmatch 'machine_cpu_mean_pct=0\.0\n'){ $ng += '小数 1 桁の丸めが期待と違います。' }
    if ($emptyText -notmatch 'heavy_processes_found=0\n')  { $ng += '該当件数 0 の表記が違います。' }

    # (3) 判定の全通り（境界＝閾値ちょうどは静か・計測失敗は MEASURE_FAILED）
    $cases = @(
        @{ mean = 4.0;  th = 10.0; n = 0; v = $VERDICT_QUIET; r = $REASON_OK;   c = $EXIT_QUIET },
        @{ mean = 40.0; th = 10.0; n = 0; v = $VERDICT_NOT;   r = $REASON_CPU;  c = $EXIT_NOT_QUIET },
        @{ mean = 4.0;  th = 10.0; n = 2; v = $VERDICT_NOT;   r = $REASON_PROC; c = $EXIT_NOT_QUIET },
        @{ mean = 40.0; th = 10.0; n = 2; v = $VERDICT_NOT;   r = $REASON_BOTH; c = $EXIT_NOT_QUIET },
        @{ mean = 10.0; th = 10.0; n = 0; v = $VERDICT_QUIET; r = $REASON_OK;   c = $EXIT_QUIET }
    )
    foreach ($c in $cases) {
        $got = Get-QuietVerdict $c.mean $c.th $c.n
        if ($got.verdict -cne $c.v -or $got.reason -cne $c.r -or $got.code -ne $c.c) {
            $ng += "判定が違います: mean=$($c.mean) th=$($c.th) n=$($c.n) → $($got.verdict)/$($got.reason)/$($got.code)"
        }
    }
    $failed = Get-QuietVerdict $null 10.0 0
    if ($failed.verdict -cne $VERDICT_FAILED -or $failed.code -ne $EXIT_MEASURE) { $ng += '計測失敗の判定が違います。' }

    # (4) TOML の [quiet] 節の解釈（他の節の同名キー・注釈行・行内注釈を混ぜる）
    $toml = @'
[goal]
name = "draw-load-parity"
sample_sec = 999                     # 別の節の同名キーは読まない

# 注釈行
[quiet]
machine_cpu_max_pct = 10.5           # 行内注釈
sample_sec = 7
heavy_process_names = ["cargo", "rustc", "rust-analyzer"]
retry_max = 3

[followup]
sample_sec = 111
'@
    $parsed = Read-QuietSettingsFromToml $toml
    if ($parsed['machine_cpu_max_pct'] -ne 10.5) { $ng += "TOML: machine_cpu_max_pct が $($parsed['machine_cpu_max_pct'])" }
    if ($parsed['sample_sec'] -ne 7)             { $ng += "TOML: sample_sec が $($parsed['sample_sec'])" }
    if ((@($parsed['heavy_process_names']) -join ',') -cne 'cargo,rustc,rust-analyzer') { $ng += 'TOML: heavy_process_names の解釈が違います。' }

    if ($ng.Count -gt 0) {
        Write-Problem "[check-quiet] 自己較正 NG（$($ng.Count) 件）"
        foreach ($m in $ng) { Write-Problem "  - $m" }
        Write-Info "CHECK-QUIET RESULT stage=selftest verdict=SELFTEST_NG code=$EXIT_SELFTEST_NG file=-"
        exit $EXIT_SELFTEST_NG
    }
    Write-Info '[check-quiet] 自己較正 OK（文面の決定論・書式・判定 6 通り・TOML [quiet] 解釈）'
    Write-Info "CHECK-QUIET RESULT stage=selftest verdict=SELFTEST_OK code=$EXIT_QUIET file=-"
    exit $EXIT_QUIET
}

if ($SelfTest) { Invoke-SelfTest }

# =============================================================================
# 引数・前提の検証
# =============================================================================
if ([string]::IsNullOrWhiteSpace($Stage)) {
    Stop-Bad $EXIT_BAD_ARGS '-Stage が必要です（出力ファイル名 quiet-<Stage>.txt に使います）。'
}
if ($Stage.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0) {
    Stop-Bad $EXIT_BAD_ARGS "-Stage にファイル名として使えない文字が含まれています: $Stage"
}
if ([string]::IsNullOrWhiteSpace($OutDir)) { Stop-Bad $EXIT_BAD_ARGS '-OutDir が必要です。' }
if ($Goal -and $GoalFile) { Stop-Bad $EXIT_BAD_ARGS '-Goal と -GoalFile は同時に指定できません。' }

# 既定値 → 目標定義ファイル → 明示引数、の順に上書きする。
# 【落とし穴】PowerShell の変数名は大小を区別しない。実効値を入れる変数に $SampleSec と
# 同じ綴りを使うと、既定値の代入がそのまま引数を上書きしてしまい、-SampleSec が黙って
# 効かなくなる（2026-08-23 に実測で踏んだ）。ゆえに実効値は eff* の名前で持つ。
$effThresholdPct = $DEFAULT_MACHINE_CPU_MAX_PCT
$effSampleSec    = $DEFAULT_SAMPLE_SEC
$effHeavyNames   = @($DEFAULT_HEAVY_PROCESS_NAMES)
$goalPath        = ''

if ($Goal -or $GoalFile) {
    $goalPath = if ($GoalFile) { $GoalFile } else { Join-Path $PSScriptRoot "goals\$Goal.toml" }
    if (-not (Test-Path -LiteralPath $goalPath -PathType Leaf)) {
        Stop-Bad $EXIT_BAD_ARGS "目標定義ファイルが見つかりません: $goalPath"
    }
    $goalPath = (Resolve-Path -LiteralPath $goalPath).Path
    $settings = Read-QuietSettingsFromToml ([IO.File]::ReadAllText($goalPath))
    if ($settings.ContainsKey('machine_cpu_max_pct')) { $effThresholdPct = [double]$settings['machine_cpu_max_pct'] }
    if ($settings.ContainsKey('sample_sec'))          { $effSampleSec    = [int]$settings['sample_sec'] }
    if ($settings.ContainsKey('heavy_process_names')) { $effHeavyNames   = @($settings['heavy_process_names']) }
}

if ($PSBoundParameters.ContainsKey('MachineCpuMaxPct'))  { $effThresholdPct = [double]$MachineCpuMaxPct }
if ($PSBoundParameters.ContainsKey('SampleSec'))         { $effSampleSec    = [int]$SampleSec }
if ($PSBoundParameters.ContainsKey('HeavyProcessNames')) { $effHeavyNames   = @($HeavyProcessNames) }

if ($effSampleSec -lt 1)      { Stop-Bad $EXIT_BAD_ARGS "採取秒数が不正です（1 以上が必要）: $effSampleSec" }
if ($effThresholdPct -le 0.0) { Stop-Bad $EXIT_BAD_ARGS "閾値が不正です（0 より大きい値が必要）: $effThresholdPct" }

try {
    if (-not (Test-Path -LiteralPath $OutDir)) { New-Item -ItemType Directory -Path $OutDir -Force | Out-Null }
} catch {
    Stop-Bad $EXIT_BAD_ARGS "出力先を作れません: $OutDir（$($_.Exception.Message)）"
}
$outFile = Join-Path (Resolve-Path -LiteralPath $OutDir).Path "quiet-$Stage.txt"

# =============================================================================
# 計測 → 判定 → 記録
# =============================================================================
$goalShown = if ($goalPath) { $goalPath } else { '(引数/既定)' }
Write-Info "[check-quiet] stage=$Stage 秒数=$effSampleSec 閾値=$(Format-Pct $effThresholdPct)% 目標定義=$goalShown"

$meanPct      = $null
$maxPct       = $null
$counterError = ''
try {
    $samples = Measure-MachineCpu $effSampleSec $COUNTER_PATH_MACHINE_CPU $WARMUP_SAMPLE_COUNT
    $stat    = $samples | Measure-Object -Average -Maximum
    $meanPct = [double]$stat.Average
    $maxPct  = [double]$stat.Maximum
} catch {
    $counterError = $_.Exception.Message
    Write-Problem "[check-quiet] 性能カウンタ '$COUNTER_PATH_MACHINE_CPU' の取得に失敗しました: $counterError"
}

# 既知の重いプロセス（測定対象 areka と自分自身は除外）
$heavyList = @()
if ($effHeavyNames.Count -gt 0) {
    $wanted = @($effHeavyNames | ForEach-Object { $_.ToLowerInvariant() })
    foreach ($p in (Get-Process -ErrorAction SilentlyContinue)) {
        if ($p.Id -eq $PID) { continue }
        if ($TargetPid -gt 0 -and $p.Id -eq $TargetPid) { continue }
        if ($wanted -contains $p.ProcessName.ToLowerInvariant()) {
            $heavyList += [pscustomobject]@{ pid = [int]$p.Id; name = [string]$p.ProcessName }
        }
    }
}

$v = Get-QuietVerdict $meanPct $effThresholdPct @($heavyList).Count
$report = Format-QuietReport @{
    version       = $SCRIPT_VERSION
    stage         = $Stage
    time_utc      = Get-UtcStamp
    sample_sec    = $effSampleSec
    mean_pct      = $meanPct
    max_pct       = $maxPct
    threshold_pct = $effThresholdPct
    heavy_names   = $effHeavyNames
    heavy_list    = $heavyList
    target_pid    = $TargetPid
    verdict       = $v.verdict
    reason        = $v.reason
}

[IO.File]::WriteAllText($outFile, $report, (New-Object Text.UTF8Encoding($false)))
[Console]::Out.Write($report)
if ($counterError) { Write-Info "counter_error=$counterError" }
Write-Info "CHECK-QUIET RESULT stage=$Stage verdict=$($v.verdict) code=$($v.code) file=$outFile"
exit $v.code
