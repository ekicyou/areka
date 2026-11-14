#Requires -Version 7.0
<#
================================================================================
invoke-perf-run.selftest.ps1 — ランナーの自分の検査（T3）
  spec: audit-tools-perf-run（要件 4.1〜4.5・design「Testing Strategy」の T3）

呼び方: invoke-perf-run.ps1 -SelfTest が静かさの関門より前で読み込み、Invoke-PerfRunSelfTest を呼ぶ
        （ランナーの関数と較正値がそのまま見える）。単独では回さない。
出力:   赤の検査だけ 1 行ずつ 'NG <検査の名前>: <理由>'（ASCII）。結果の 1 行はランナーが出す。
戻り値: 赤の数。緑の数は $script:SelfTestOk に置く。

決まり（T2 と同じ）:
  - 作業のフォルダは <リポジトリ>\target\perf-loop-tmp\selftest-<GUID>\ の下だけ。終わったら消す。
  - 本物の areka・性能カウンタ・xperf を使わない。

検査を足すには: $PerfRunSelfTestChecks に Name と Body（失敗なら throw する scriptblock）の 1 組を足す。
================================================================================
#>

function Assert-SelfTest {
    param([bool]$Condition, [string]$Reason)
    if (-not $Condition) { throw $Reason }
}

# 理由を 1 行の ASCII にする（例外の文は地域語のことがある）
function ConvertTo-SelfTestAscii {
    param([string]$Text)
    return (($Text -replace '\s*\r?\n\s*', ' ') -replace '[^\x20-\x7E]', '?')
}

# 偽の CounterSample（Path と CookedValue。InstanceName は枝番によらず素の名前＝本物と同じ落とし穴）
function New-FakeCounterSample {
    param([string]$Instance, [string]$Counter, [double]$Value)
    return [pscustomobject]@{ Path = "\\host\process($Instance)\$Counter"; InstanceName = 'areka'; CookedValue = $Value }
}

$PerfRunSelfTestChecks = @(
    [pscustomobject]@{ Name = 'counter-select'; Body = {
        # 同じ素の名前で枝番の違う 2 つ。対象（PID 200）は後ろに並ぶ areka#1
        $samples = @(
            (New-FakeCounterSample 'areka' 'id process' 100)
            (New-FakeCounterSample 'areka' '% processor time' 11.5)
            (New-FakeCounterSample 'areka#1' 'id process' 200)
            (New-FakeCounterSample 'areka#1' '% processor time' 22.5)
        )
        $v = Select-ProcessCpuValue -Samples $samples -TargetProcessId 200
        Assert-SelfTest ($v -eq 22.5) "pid 200: expected 22.5, got $v"
        $v = Select-ProcessCpuValue -Samples $samples -TargetProcessId 100
        Assert-SelfTest ($v -eq 11.5) "pid 100: expected 11.5, got $v"
        $v = Select-ProcessCpuValue -Samples $samples -TargetProcessId 300
        Assert-SelfTest ($null -eq $v) "pid 300: expected null, got $v"
        # ID は在るが CPU の値が無い枝番は選ばない
        $v = Select-ProcessCpuValue -Samples @($samples[0], $samples[1], $samples[2]) -TargetProcessId 200
        Assert-SelfTest ($null -eq $v) "pid 200 without cpu sample: expected null, got $v"
    } }

    [pscustomobject]@{ Name = 'quiet-retry-settings'; Body = {
        $text = @(
            '[other]', 'retry_max = 99', 'retry_wait_sec = 98',
            '[quiet]', 'retry_max = 5', "retry_wait_sec = 7  # comment", 'cpu_threshold = 3',
            '[after]', 'retry_wait_sec = 97'
        ) -join "`n"
        $r = Read-QuietRetrySettingsFromToml $text
        Assert-SelfTest ($r.Count -eq 2) "both keys: expected 2 keys, got $($r.Count) ($(@($r.Keys) -join ','))"
        Assert-SelfTest ($r['retry_max'] -eq 5) "retry_max: expected 5, got $($r['retry_max'])"
        Assert-SelfTest ($r['retry_wait_sec'] -eq 7) "retry_wait_sec: expected 7, got $($r['retry_wait_sec'])"
        $r = Read-QuietRetrySettingsFromToml (@('[quiet]', 'retry_max = 0', '[x]', 'retry_wait_sec = 9') -join "`n")
        Assert-SelfTest ($r.Count -eq 1 -and $r['retry_max'] -eq 0) "only retry_max = 0: got $(@($r.Keys) -join ',') / $($r['retry_max'])"
        $r = Read-QuietRetrySettingsFromToml (@('[other]', 'retry_max = 4') -join "`n")
        Assert-SelfTest ($r.Count -eq 0) "no [quiet]: expected 0 keys, got $($r.Count)"
    } }

    [pscustomobject]@{ Name = 'smoke-marker'; Body = {
        $filler = @(1..5000 | ForEach-Object { "2026-11-15T00:00:00Z INFO line $_" })
        $utf8 = [System.Text.UTF8Encoding]::new($false)
        $atEnd = Join-Path $script:SelfTestRoot 'marker-at-end.log'
        [System.IO.File]::WriteAllLines($atEnd, $filler + @('INFO ' + $LOG_MARKER_SMOKE_GATE + ' ms=420000'), $utf8)
        Assert-SelfTest (Test-SmokeMarker -Path $atEnd) 'marker on the last line not found'
        $inMiddle = Join-Path $script:SelfTestRoot 'marker-in-middle.log'
        [System.IO.File]::WriteAllLines($inMiddle, $filler + @('INFO ' + $LOG_MARKER_SMOKE_GATE) + $filler, $utf8)
        Assert-SelfTest (Test-SmokeMarker -Path $inMiddle) 'marker in the middle not found'
        $none = Join-Path $script:SelfTestRoot 'marker-none.log'
        [System.IO.File]::WriteAllLines($none, $filler, $utf8)
        Assert-SelfTest (-not (Test-SmokeMarker -Path $none)) 'marker found in a log without it'
        # 目印は字面のまま比べる（正規表現として読まない）
        Assert-SelfTest (-not (Test-SmokeMarker -Path $none -Marker 'line.5000$')) 'marker read as a regex'
    } }
)

function Invoke-PerfRunSelfTest {
    $repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $script:SelfTestRoot = Join-Path $repo ('target\perf-loop-tmp\selftest-' + [Guid]::NewGuid().ToString('N'))
    $script:SelfTestOk = 0
    $ng = 0
    $null = New-Item -ItemType Directory -Path $script:SelfTestRoot -Force
    try {
        foreach ($check in $PerfRunSelfTestChecks) {
            try {
                $null = & $check.Body
                $script:SelfTestOk++
            } catch {
                $ng++
                Write-Info "NG $($check.Name): $(ConvertTo-SelfTestAscii $_.Exception.Message)"
            }
        }
    } finally {
        Remove-Item -LiteralPath $script:SelfTestRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    return $ng
}
