<#
.SYNOPSIS
    areka-P0-file-slimming: 検証ツール 3 本（Compare-RelocatedTests / Test-MappingBijection /
    Compare-TestLists）を、既知の一致ケース・不一致ケースの双方で実行し、期待どおりに
    判定することを確認する自己検証ドライバ（タスク 1.4 の完了状態そのもの）。

.DESCRIPTION
    各ケースは「コマンド」「期待 exit code」「実 exit code」を持ち、全ケース一致で exit 0。
    1 件でも期待から外れたら該当ケースの出力を添えて exit 1。

    ケース TOKENIZER-EQUIV は、`RustParse.ps1` が `Measure-TestModules.ps1`（タスク 1.1）の
    トークナイザを弱めていないことを機械検証する——`scan_raw.csv` の `modules` 列
    （タスク 1.1 が実測したテストモジュール行範囲）をリポジトリ全ファイルで再現できることを要求する。

    ケース LIST-SMOKE-* は、フィクスチャではなく実データ（4,790 行 / 4,105 行）で
    Compare-TestLists.ps1 が動くことを示す。

.OUTPUTS
    ケース一覧の判定マトリクスと総括。全件期待どおり: exit 0 / それ以外: exit 1

.EXAMPLE
    pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Test-VerificationTools.ps1

.NOTES
    Requirements: 1.8, 2.3, 2.4, 2.9
    Design:       §検証 / EvidencePipeline（証跡パイプライン）, §Data Models
#>
[CmdletBinding()]
param(
    [switch]$ShowOutput
)

$ErrorActionPreference = 'Stop'
$V = $PSScriptRoot
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path

$results = New-Object System.Collections.Generic.List[object]

function Invoke-Case {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Script,
        [Parameter(Mandatory)][string[]]$Args,
        [Parameter(Mandatory)][int]$Expect
    )
    $out = & pwsh -NoProfile -File (Join-Path $V $Script) @Args 2>&1
    $code = $LASTEXITCODE
    $results.Add([pscustomobject]@{
            Name    = $Name
            Command = ('{0} {1}' -f $Script, ($Args -join ' '))
            Expect  = $Expect
            Actual  = $code
            Ok      = ($code -eq $Expect)
            Output  = ($out | Out-String).TrimEnd()
        })
}

# ===================================================================== 本文一致検証
$orig = 'fixtures/relocate/original.rs'
Invoke-Case -Name 'RELOC-OK-DEINDENT' -Script 'Compare-RelocatedTests.ps1' -Expect 0 -Args @(
    '-OriginalPath', (Join-Path $V $orig), '-RelocatedPath', (Join-Path $V 'fixtures/relocate/ok_deindent.rs'), '-Detail')
Invoke-Case -Name 'RELOC-BAD-ASSERT' -Script 'Compare-RelocatedTests.ps1' -Expect 1 -Args @(
    '-OriginalPath', (Join-Path $V $orig), '-RelocatedPath', (Join-Path $V 'fixtures/relocate/bad_assert.rs'))
Invoke-Case -Name 'RELOC-BAD-DROPPED' -Script 'Compare-RelocatedTests.ps1' -Expect 1 -Args @(
    '-OriginalPath', (Join-Path $V $orig), '-RelocatedPath', (Join-Path $V 'fixtures/relocate/bad_dropped.rs'))
Invoke-Case -Name 'RELOC-BAD-RENAMED' -Script 'Compare-RelocatedTests.ps1' -Expect 1 -Args @(
    '-OriginalPath', (Join-Path $V $orig), '-RelocatedPath', (Join-Path $V 'fixtures/relocate/bad_renamed.rs'))
Invoke-Case -Name 'RELOC-OK-THEMESPLIT' -Script 'Compare-RelocatedTests.ps1' -Expect 0 -Args @(
    '-OriginalPath', (Join-Path $V $orig),
    '-RelocatedPath', ((@(
                (Join-Path $V 'fixtures/relocate/split_gamma_tests.rs'),
                (Join-Path $V 'fixtures/relocate/split_test_support.rs'),
                (Join-Path $V 'fixtures/relocate/split_alpha_tests.rs'))) -join ','),
    '-Detail')

# ===================================================================== 全単射検証
Invoke-Case -Name 'MAP-OK' -Script 'Test-MappingBijection.ps1' -Expect 0 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/map_ok.csv'))
Invoke-Case -Name 'MAP-DUP-OLD' -Script 'Test-MappingBijection.ps1' -Expect 1 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/map_dup_old.csv'))
Invoke-Case -Name 'MAP-DUP-NEW' -Script 'Test-MappingBijection.ps1' -Expect 1 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/map_dup_new.csv'))
Invoke-Case -Name 'MAP-BAD-IDENT' -Script 'Test-MappingBijection.ps1' -Expect 1 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/map_bad_ident.csv'))
Invoke-Case -Name 'MAP-BAD-REASON' -Script 'Test-MappingBijection.ps1' -Expect 1 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/map_bad_reason.csv'))
Invoke-Case -Name 'MAP-FRAG-OK' -Script 'Test-MappingBijection.ps1' -Expect 0 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/frag_ok'))
Invoke-Case -Name 'MAP-FRAG-COLLIDE' -Script 'Test-MappingBijection.ps1' -Expect 1 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/frag_collide'))
Invoke-Case -Name 'MAP-EMPTY-CONTAINER' -Script 'Test-MappingBijection.ps1' -Expect 0 -Args @(
    '-Path', (Join-Path $V 'test_name_mapping.csv'))
Invoke-Case -Name 'MAP-FRAGDIR-EMPTY' -Script 'Test-MappingBijection.ps1' -Expect 0 -Args @(
    '-Path', (Join-Path $V 'mapping'))

# フラグメント結合（タスク 7.1 の運用）: 結合出力が単体でも全単射検証を通ること
$mergeOut = Join-Path ([System.IO.Path]::GetTempPath()) ('fs_merge_{0}.csv' -f ([guid]::NewGuid().ToString('N')))
Invoke-Case -Name 'MAP-MERGE-OUT' -Script 'Test-MappingBijection.ps1' -Expect 0 -Args @(
    '-Path', (Join-Path $V 'fixtures/mapping/frag_ok'), '-Out', $mergeOut)
Invoke-Case -Name 'MAP-MERGE-ROUNDTRIP' -Script 'Test-MappingBijection.ps1' -Expect 0 -Args @(
    '-Path', $mergeOut)

# ===================================================================== リスト照合
$bs = Join-Path $V 'fixtures/lists/before_small.txt'
Invoke-Case -Name 'LIST-OK-IDENTICAL' -Script 'Compare-TestLists.ps1' -Expect 0 -Args @(
    '-Before', $bs, '-After', (Join-Path $V 'fixtures/lists/after_identical.txt'))
Invoke-Case -Name 'LIST-BAD-MISSING' -Script 'Compare-TestLists.ps1' -Expect 1 -Args @(
    '-Before', $bs, '-After', (Join-Path $V 'fixtures/lists/after_missing.txt'))
Invoke-Case -Name 'LIST-BAD-RENAMED-NOMAP' -Script 'Compare-TestLists.ps1' -Expect 1 -Args @(
    '-Before', $bs, '-After', (Join-Path $V 'fixtures/lists/after_renamed.txt'))
Invoke-Case -Name 'LIST-OK-RENAMED-WITHMAP' -Script 'Compare-TestLists.ps1' -Expect 0 -Args @(
    '-Before', $bs, '-After', (Join-Path $V 'fixtures/lists/after_renamed.txt'),
    '-Mapping', (Join-Path $V 'fixtures/mapping/map_ok.csv'))
Invoke-Case -Name 'LIST-BAD-DUPCOLLAPSE' -Script 'Compare-TestLists.ps1' -Expect 1 -Args @(
    '-Before', $bs, '-After', (Join-Path $V 'fixtures/lists/after_dupcollapse.txt'))

# 実データスモーク（フィクスチャではなく移設前スナップショットそのもの）
Invoke-Case -Name 'LIST-SMOKE-REALDATA-DEFAULT' -Script 'Compare-TestLists.ps1' -Expect 0 -Args @(
    '-Before', (Join-Path $V 'before_default.txt'), '-After', (Join-Path $V 'before_default.txt'))
Invoke-Case -Name 'LIST-SMOKE-REALDATA-ALLTARGETS' -Script 'Compare-TestLists.ps1' -Expect 0 -Args @(
    '-Before', (Join-Path $V 'before_alltargets.txt'), '-After', (Join-Path $V 'before_alltargets.txt'))

# ============================================ トークナイザ持ち上げの等価性（対 タスク 1.1）
. (Join-Path $V 'RustParse.ps1')

$scanRaw = Join-Path $V 'scan_raw.csv'
$tokMsg = ''
$tokOk = $false
if (-not (Test-Path -LiteralPath $scanRaw)) {
    $tokMsg = "scan_raw.csv が見つかりません: $scanRaw"
}
else {
    $rows = @(Import-Csv -LiteralPath $scanRaw)
    $checked = 0; $withBlocks = 0; $bad = 0
    $badSamples = New-Object System.Collections.Generic.List[string]
    foreach ($r in $rows) {
        $full = Join-Path $RepoRoot ($r.path -replace '/', '\')
        if (-not (Test-Path -LiteralPath $full)) { continue }
        $checked++
        $lines = [System.IO.File]::ReadAllLines($full)
        $blocks = [RustItemScan]::FindTestModules($lines)
        $desc = (@($blocks | ForEach-Object { '{0}:{1}-{2}({3})' -f $_.Name, $_.Start, $_.End, $_.Lines }) -join '|')
        $expected = [string]$r.modules
        if ($desc -cne $expected) {
            $bad++
            if ($badSamples.Count -lt 5) { $badSamples.Add(('  {0}{1}    expected: {2}{1}    actual  : {3}' -f $r.path, [Environment]::NewLine, $expected, $desc)) }
        }
        if ($expected -ne '') { $withBlocks++ }
    }
    $tokOk = ($bad -eq 0 -and $checked -gt 0)
    $tokMsg = ('走査 {0} ファイル / テストモジュールを持つ {1} ファイル / 不一致 {2}' -f $checked, $withBlocks, $bad)
    if ($bad -gt 0) { $tokMsg += [Environment]::NewLine + ($badSamples -join [Environment]::NewLine) }
}
$results.Add([pscustomobject]@{
        Name    = 'TOKENIZER-EQUIV'
        Command = 'RustParse.ps1 [RustItemScan]::FindTestModules vs scan_raw.csv (modules 列)'
        Expect  = 0
        Actual  = $(if ($tokOk) { 0 } else { 1 })
        Ok      = $tokOk
        Output  = $tokMsg
    })

# ===================================================================== 判定マトリクス
Write-Output ''
Write-Output '=== 判定マトリクス（期待 exit code / 実 exit code） ==='
Write-Output ('{0,-32} {1,7} {2,7}  {3}' -f 'case', 'expect', 'actual', 'verdict')
foreach ($r in $results) {
    Write-Output ('{0,-32} {1,7} {2,7}  {3}' -f $r.Name, $r.Expect, $r.Actual, $(if ($r.Ok) { 'OK' } else { '*** NG ***' }))
}

$ng = @($results | Where-Object { -not $_.Ok })
if ($ShowOutput -or $ng.Count -gt 0) {
    foreach ($r in $(if ($ng.Count -gt 0) { $ng } else { $results })) {
        Write-Output ''
        Write-Output ('--- {0} : {1}' -f $r.Name, $r.Command)
        Write-Output $r.Output
    }
}

if (Test-Path -LiteralPath $mergeOut) { Remove-Item -LiteralPath $mergeOut -Force }

Write-Output ''
if ($ng.Count -gt 0) {
    Write-Output ('RESULT: FAIL  ({0}/{1} ケースが期待どおりでない)' -f $ng.Count, $results.Count)
    exit 1
}
Write-Output ('RESULT: PASS  ({0}/{0} ケースが期待どおり)' -f $results.Count)
exit 0
