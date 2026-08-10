<#
.SYNOPSIS
    areka-P0-file-slimming: 移設前テスト名リストへ対応表を適用した結果と、
    移設後テスト名リストを多重集合として突合する。

.DESCRIPTION
    design §検証 / EvidencePipeline の判定契約:

        リスト (1)(2): `対応表を旧リストへ適用した結果 == 新リスト`
        （テーマ分割が無いクレートは素の完全一致）。
        行数一致が 2.2 の総数一致を、名前一致が 2.9 を同時に担保する。

    notes.md §10.2 の採取規約に合わせる:
      - 行の形式は `<FQN>: test`
      - 整列は序数（ordinal）比較 `[Array]::Sort($arr, [StringComparer]::Ordinal)`
        （`Sort-Object` の既定はカルチャ依存で不可）
      - **重複行は除去しない**。比較は集合ではなく多重集合（行数込み）で行う
        （実測: before_default.txt は 4,790 行 / 相異なる名前 4,787）

.PARAMETER Before
    移設前リスト（`before_default.txt` / `before_alltargets.txt`）。

.PARAMETER After
    移設後リスト（`after_default.txt` / `after_alltargets.txt`）。

.PARAMETER Mapping
    旧->新の対応表 CSV、またはフラグメントディレクトリ。複数指定可。省略時は対応表なし（素の完全一致を要求）。

.PARAMETER StrictUnusedMappings
    Before に一度も現れない対応表の行を FAIL 扱いにする。
    既定では INFO 止まり——examples のテストは `_alltargets` 側にしか現れず、
    `_default` 側の照合では未使用になるのが正常であるため。
    最終照合（タスク 7.1）では 2 本のリストを跨いだ未使用行がゼロであることを目視で確認する。

.OUTPUTS
    PASS: 要約・exit 0
    FAIL: 行数と対称差の逐条レポート・exit 1
    引数不正: エラーメッセージ・exit 2

.EXAMPLE
    pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Compare-TestLists.ps1 `
        -Before .kiro/specs/areka-P0-file-slimming/verification/before_default.txt `
        -After  .kiro/specs/areka-P0-file-slimming/verification/after_default.txt `
        -Mapping .kiro/specs/areka-P0-file-slimming/verification/test_name_mapping.csv

.NOTES
    Requirements: 1.8, 2.2, 2.3, 2.9
    Design:       §検証 / EvidencePipeline（証跡パイプライン）, §Data Models
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Before,
    [Parameter(Mandatory)][string]$After,
    [string[]]$Mapping = @(),
    [switch]$StrictUnusedMappings
)

$ErrorActionPreference = 'Stop'

$SUFFIX = ': test'

function Read-TestList([string]$p, [string]$label) {
    if (-not (Test-Path -LiteralPath $p)) { Write-Error "${label} が見つかりません: $p" -ErrorAction Continue; exit 2 }
    $lines = @([System.IO.File]::ReadAllLines($p))
    $out = New-Object System.Collections.Generic.List[string]
    $bad = 0
    foreach ($l in $lines) {
        if ($l -eq '') { continue }
        if ($l -cmatch ': test$') { $out.Add($l) } else { $bad++ }
    }
    if ($bad -gt 0) {
        Write-Error "${label} に `: test` で終わらない行が ${bad} 件あります: $p（notes.md §10.2 の抽出手順で採取し直すこと）" -ErrorAction Continue
        exit 2
    }
    return $out
}

# `pwsh -File` は引数を常に単一文字列として渡すため、カンマ区切りの複数指定を自前で展開する
function Expand-ListArg([string[]]$v) {
    $o = New-Object System.Collections.Generic.List[string]
    foreach ($x in $v) {
        if ($null -eq $x) { continue }
        foreach ($y in ($x -split ',')) { $t = $y.Trim(); if ($t -ne '') { $o.Add($t) } }
    }
    return $o.ToArray()
}
$Mapping = Expand-ListArg $Mapping

# --------------------------------------------------------------------- 対応表の読込
$map = New-Object 'System.Collections.Generic.Dictionary[string,string]' ([System.StringComparer]::Ordinal)
$mapFiles = New-Object System.Collections.Generic.List[string]
foreach ($p in $Mapping) {
    if (-not (Test-Path -LiteralPath $p)) { Write-Error "対応表が見つかりません: $p" -ErrorAction Continue; exit 2 }
    $it = Get-Item -LiteralPath $p
    if ($it.PSIsContainer) {
        foreach ($f in @(Get-ChildItem -LiteralPath $it.FullName -Filter '*.csv' -File | Sort-Object Name)) { $mapFiles.Add($f.FullName) }
    }
    else { $mapFiles.Add($it.FullName) }
}
foreach ($f in $mapFiles) {
    foreach ($r in @(Import-Csv -LiteralPath $f)) {
        $o = ([string]$r.old_fqn).Trim()
        $n = ([string]$r.new_fqn).Trim()
        if ($o -eq '' -or $n -eq '') { continue }
        if ($map.ContainsKey($o)) {
            Write-Error "対応表の old_fqn が重複しています: `$o`（先に Test-MappingBijection.ps1 で検証すること）" -ErrorAction Continue
            exit 2
        }
        $map[$o] = $n
    }
}

# ------------------------------------------------------------------------- 読込
$beforeLines = Read-TestList $Before 'Before'
$afterLines = Read-TestList $After 'After'

# ------------------------------------------------------------------- 対応表の適用
$applied = New-Object System.Collections.Generic.List[string]
$usedKeys = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::Ordinal)
$mappedCount = 0
foreach ($l in $beforeLines) {
    $fqn = $l.Substring(0, $l.Length - $SUFFIX.Length)
    if ($map.ContainsKey($fqn)) {
        $applied.Add($map[$fqn] + $SUFFIX)
        [void]$usedKeys.Add($fqn)
        $mappedCount++
    }
    else {
        $applied.Add($l)
    }
}

# ------------------------------------------------------- 序数整列（notes.md §10.2 手順 3）
$appliedArr = [string[]]@($applied)
$afterArr = [string[]]@($afterLines)
if ($appliedArr.Count -gt 1) { [Array]::Sort($appliedArr, [System.StringComparer]::Ordinal) }
if ($afterArr.Count -gt 1) { [Array]::Sort($afterArr, [System.StringComparer]::Ordinal) }

# ------------------------------------------------------------ 多重集合としての突合
function Get-Counts([string[]]$arr) {
    $h = New-Object 'System.Collections.Generic.Dictionary[string,int]' ([System.StringComparer]::Ordinal)
    foreach ($x in $arr) {
        if ($h.ContainsKey($x)) { $h[$x] = $h[$x] + 1 } else { $h[$x] = 1 }
    }
    return $h
}
$cb = Get-Counts $appliedArr
$ca = Get-Counts $afterArr

$keys = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::Ordinal)
foreach ($k in $cb.Keys) { [void]$keys.Add($k) }
foreach ($k in $ca.Keys) { [void]$keys.Add($k) }
$keyArr = [string[]]@($keys)
if ($keyArr.Count -gt 1) { [Array]::Sort($keyArr, [System.StringComparer]::Ordinal) }

$diffs = New-Object System.Collections.Generic.List[string]
foreach ($k in $keyArr) {
    $nb = 0; if ($cb.ContainsKey($k)) { $nb = $cb[$k] }
    $na = 0; if ($ca.ContainsKey($k)) { $na = $ca[$k] }
    if ($nb -eq $na) { continue }
    if ($na -eq 0) { $diffs.Add(('  - 移設後に無い (before x{0}): {1}' -f $nb, $k)) }
    elseif ($nb -eq 0) { $diffs.Add(('  + 移設前に無い (after  x{0}): {1}' -f $na, $k)) }
    else { $diffs.Add(('  ! 出現数が違う (before x{0} / after x{1}): {2}' -f $nb, $na, $k)) }
}

$unused = @()
if ($map.Count -gt 0) {
    $unused = @($map.Keys | Where-Object { -not $usedKeys.Contains($_) })
    if ($unused.Count -gt 1) { $u = [string[]]@($unused); [Array]::Sort($u, [System.StringComparer]::Ordinal); $unused = $u }
}

# ------------------------------------------------------------------------- 報告
Write-Output ('BEFORE      : {0}  ({1} 行 / 相異なる {2})' -f (Split-Path -Leaf $Before), $beforeLines.Count, $cb.Count)
Write-Output ('AFTER       : {0}  ({1} 行 / 相異なる {2})' -f (Split-Path -Leaf $After), $afterLines.Count, $ca.Count)
Write-Output ('MAPPING     : {0} 行 ({1} ファイル) / 適用 {2} 行 / 未使用 {3} 行' -f $map.Count, $mapFiles.Count, $mappedCount, $unused.Count)
Write-Output ('LINE COUNT  : before {0} / after {1} -> {2}' -f $beforeLines.Count, $afterLines.Count, ($(if ($beforeLines.Count -eq $afterLines.Count) { '一致 (Requirement 2.2)' } else { '不一致' })))

$fail = $false
if ($beforeLines.Count -ne $afterLines.Count) { $fail = $true }
if ($diffs.Count -gt 0) {
    $fail = $true
    Write-Output ('SYMMETRIC DIFF: {0} 名' -f $diffs.Count)
    foreach ($d in $diffs) { Write-Output $d }
}
if ($unused.Count -gt 0) {
    Write-Output ('UNUSED MAPPING ROWS: {0}' -f $unused.Count)
    foreach ($u in $unused) { Write-Output ('  ? before に現れない old_fqn: {0}' -f $u) }
    if ($StrictUnusedMappings) { $fail = $true }
}

if ($fail) { Write-Output 'RESULT: FAIL'; exit 1 }
Write-Output 'RESULT: PASS'
exit 0
