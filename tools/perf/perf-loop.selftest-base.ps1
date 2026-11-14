#Requires -Version 7.0
<#
================================================================================
perf-loop.selftest-base.ps1 — perf-loop 側の自分の検査（T2）の骨組み
  spec: audit-tools-perf-run（要件 4.1〜4.5・5.5・design「Testing Strategy」の T2）

読み込み用（perf-loop.selftest.ps1・perf-loop.selftest2.ps1 が . で読み込む）。単独で回しても何もしない。
perf-loop.common.ps1 と perf-loop.measure.ps1 を読み込み、その後で終了の口 Exit-WithResult を
偽物へ定義し直す（本物の exit の代わりに終了コードを持った例外を投げる）。

  Add-SelfTestCall       … 偽物が呼ばれた順と引数を $script:SelfTestCalls に残す（検査ごとに空にする）
  Assert-SelfTest        … 偽なら理由で throw
  ConvertTo-SelfTestAscii … 理由を 1 行の ASCII に
  New-SelfTestScript     … 子の pwsh に回させる .ps1 を作業のフォルダに書く
  Get-SelfTestExitCode   … 偽の Exit-WithResult が投げた終了コード（終わらなければ $null）
  Invoke-SelfTestChecks  … 検査を回し、赤だけ 'NG <名前>: <理由>'、最後に '<Label> RESULT ok=<n> ng=<m> code=<0|1>' で終える
================================================================================
#>

. (Join-Path $PSScriptRoot 'perf-loop.common.ps1')
. (Join-Path $PSScriptRoot 'perf-loop.measure.ps1')

# 本番（perf-loop.ps1）と同じく、コマンドの失敗で止まる（続けて緑に見せない）
$ErrorActionPreference = 'Stop'

$script:SelfTestCalls = [System.Collections.Generic.List[object]]::new()

# 偽物が呼ばれた順と引数を残す
function Add-SelfTestCall {
    param([Parameter(Mandatory = $true)][string]$Name, [object[]]$Arguments = @())
    $script:SelfTestCalls.Add([pscustomobject]@{ Name = $Name; Arguments = @($Arguments) })
}

# 終了の口の偽物: 本物の exit で検査が終わらないよう、終了コードを持った例外を投げる
function Exit-WithResult {
    param([Parameter(Mandatory = $true)][int]$Code, [string]$Sub, [string]$Dir)
    Add-SelfTestCall -Name 'Exit-WithResult' -Arguments @($Code, $Sub, $Dir)
    $ex = [System.Exception]::new("Exit-WithResult code=$Code")
    $ex.Data['ExitCode'] = $Code
    throw $ex
}

function Assert-SelfTest {
    param([bool]$Condition, [string]$Reason)
    if (-not $Condition) { throw $Reason }
}

# 理由を 1 行の ASCII にする（例外の文は地域語のことがある）
function ConvertTo-SelfTestAscii {
    param([string]$Text)
    return (($Text -replace '\s*\r?\n\s*', ' ') -replace '[^\x20-\x7E]', '?')
}

# 子の pwsh に回させる .ps1 を作業のフォルダに書く（BOM の無い UTF-8）
function New-SelfTestScript {
    param([Parameter(Mandatory = $true)][string]$Name, [Parameter(Mandatory = $true)][string]$Text)
    $path = Join-Path $script:SelfTestRoot $Name
    [System.IO.File]::WriteAllText($path, $Text, [System.Text.UTF8Encoding]::new($false))
    return $path
}

# 偽の Exit-WithResult が投げた終了コードを返す（終わらなければ $null）
function Get-SelfTestExitCode {
    param([Parameter(Mandatory = $true)][scriptblock]$Script)
    try { $null = & $Script }
    catch {
        if ($_.Exception.Data.Contains('ExitCode')) { return [int]$_.Exception.Data['ExitCode'] }
        throw
    }
    return $null
}

# 検査を回して終える。作業のフォルダは <リポジトリ>\target\perf-loop-tmp\selftest-<GUID>\ の下だけで、終わったら消す。
function Invoke-SelfTestChecks {
    param([Parameter(Mandatory = $true)][object[]]$Checks, [Parameter(Mandatory = $true)][string]$Label)
    $script:SelfTestRoot = Join-Path $repoRoot ('target\perf-loop-tmp\selftest-' + [Guid]::NewGuid().ToString('N'))
    $okCount = 0
    $ngCount = 0
    $null = New-Item -ItemType Directory -Path $script:SelfTestRoot -Force
    try {
        foreach ($check in $Checks) {
            $script:SelfTestCalls.Clear()
            try {
                $null = & $check.Body
                $okCount++
            } catch {
                $ngCount++
                Write-Info "NG $($check.Name): $(ConvertTo-SelfTestAscii $_.Exception.Message)"
            }
        }
    } finally {
        Remove-Item -LiteralPath $script:SelfTestRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    $code = if ($ngCount -eq 0) { 0 } else { 1 }
    Write-Info "$Label RESULT ok=$okCount ng=$ngCount code=$code"
    exit $code
}
