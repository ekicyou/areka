<#
.SYNOPSIS
    areka-P0-file-slimming: 移設前の本番ファイル内テストモジュール本体と、
    移設後のテストファイル群の本文一致を、行頭空白非依存・項目単位で検証する。

.DESCRIPTION
    design §検証 / EvidencePipeline（証跡パイプライン）の判定契約:

      「移設前コミットの元ファイルのモジュールブロック内部」と「移設後のテストファイル群」を
       行頭空白正規化（lstrip）で突合し、テーマ分割分は項目単位
      （テスト関数はキー＝識別子で 1:1、ヘルパ項目は正規化本文の多重集合）で一致判定する。
       出力ゼロ＝一致。

    Requirement 2.4 が許容する差分だけを吸収する:
      - 一律 4 スペース de-indent      → 行頭空白の除去
      - `use` の調整                   → `use` 項目を比較対象から外す
      - 可視性の調整                   → 行頭 `pub` / `pub(...)` を除去
      - モジュール接続の調整           → `mod X;` 宣言を比較対象から外す／入れ子 `mod` を平坦化
    アサーション・入力値・期待値・属性・コメント・識別子は一切正規化しない。

.PARAMETER OriginalPath
    移設前の本番ファイル。`-Commit` を与えた場合はリポジトリ相対パス（`/` 区切り）として
    `git show <commit>:<path>` で読む。`-Commit` を省略した場合は作業ツリーのファイルとして読む。

.PARAMETER Commit
    移設前の状態を持つコミット（sha / ref）。実運用では必ず指定する
    （移設後の作業ツリーには既にテストモジュールが無いため）。読み取り専用の git のみを使う。

.PARAMETER RelocatedPath
    移設後のテストファイル。テーマ分割の場合は複数指定する（順序は不問）。

.PARAMETER ModuleName
    元ファイルに複数のテストモジュールがあるとき、比較対象を名前で絞る。省略時は全ブロックの合算。

.OUTPUTS
    一致: 出力ゼロ・exit 0
    不一致: 差分の逐条レポート・exit 1
    引数不正等: エラーメッセージ・exit 2

.EXAMPLE
    pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Compare-RelocatedTests.ps1 `
        -Commit f05537e `
        -OriginalPath crates/areka/src/placement/follow.rs `
        -RelocatedPath crates/areka/src/placement/follow_test_support.rs, `
                       crates/areka/src/placement/follow_anchor_tests.rs, `
                       crates/areka/src/placement/follow_drag_tests.rs

.NOTES
    Requirements: 1.8, 2.3, 2.4, 2.9
    Design:       §検証 / EvidencePipeline（証跡パイプライン）, §実装 / TestRelocation, §実装 / ThemeSplit
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OriginalPath,
    [string]$Commit,
    [Parameter(Mandatory)][string[]]$RelocatedPath,
    [string[]]$ModuleName,
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path,
    [switch]$Detail
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'RustParse.ps1')

$RepoRoot = (Resolve-Path $RepoRoot).Path

# `pwsh -File` は引数を常に単一文字列として渡すため、カンマ区切りの複数指定を自前で展開する
# （`pwsh -Command` から呼ぶ場合は既に配列で届くので、この展開は恒等になる）。
function Expand-ListArg([string[]]$v) {
    $o = New-Object System.Collections.Generic.List[string]
    foreach ($x in $v) {
        if ($null -eq $x) { continue }
        foreach ($y in ($x -split ',')) {
            $t = $y.Trim()
            if ($t -ne '') { $o.Add($t) }
        }
    }
    return $o.ToArray()
}
$RelocatedPath = Expand-ListArg $RelocatedPath
if ($ModuleName) { $ModuleName = Expand-ListArg $ModuleName }

function Resolve-Input([string]$p) {
    if ([System.IO.Path]::IsPathRooted($p)) { return $p }
    if (Test-Path -LiteralPath $p) { return (Resolve-Path -LiteralPath $p).Path }
    $j = Join-Path $RepoRoot ($p -replace '/', '\')
    if (Test-Path -LiteralPath $j) { return (Resolve-Path -LiteralPath $j).Path }
    return $p
}

# ---------------------------------------------------------------- 元ファイルの取得
$originalLines = $null
if ($Commit) {
    $spec = ('{0}:{1}' -f $Commit, ($OriginalPath -replace '\\', '/'))
    $prevOut = [Console]::OutputEncoding
    try {
        [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
        $originalLines = @(& git -C $RepoRoot show $spec)
    }
    finally { [Console]::OutputEncoding = $prevOut }
    if ($LASTEXITCODE -ne 0) {
        Write-Error "git show '$spec' が失敗しました (exit $LASTEXITCODE)" -ErrorAction Continue
        exit 2
    }
}
else {
    $op = Resolve-Input $OriginalPath
    if (-not (Test-Path -LiteralPath $op)) { Write-Error "元ファイルが見つかりません: $op" -ErrorAction Continue; exit 2 }
    $originalLines = Read-RustLines -Path $op
}
$originalLines = [string[]]$originalLines

# ------------------------------------------------- 元ファイルのテストモジュール本体を抽出
$blocks = [RustItemScan]::FindTestModules($originalLines)
if ($ModuleName) {
    $blocks = @($blocks | Where-Object { $ModuleName -contains $_.Name })
}
if (@($blocks).Count -eq 0) {
    Write-Error "元ファイルに比較対象の #[cfg(test)] mod ブロックがありません: $OriginalPath" -ErrorAction Continue
    exit 2
}

function Merge-Digest($acc, $d) {
    foreach ($k in $d.TestBodies.Keys) {
        if (-not $acc.TestBodies.ContainsKey($k)) {
            $acc.TestBodies[$k] = New-Object System.Collections.Generic.List[object]
        }
        foreach ($r in $d.TestBodies[$k]) { $acc.TestBodies[$k].Add($r) }
    }
    foreach ($r in $d.Helpers) { $acc.Helpers.Add($r) }
}

function New-EmptyDigest {
    [pscustomobject]@{
        TestBodies = (New-Object 'System.Collections.Generic.Dictionary[string, System.Collections.Generic.List[object]]')
        Helpers    = (New-Object System.Collections.Generic.List[object])
    }
}

$before = New-EmptyDigest
foreach ($b in $blocks) {
    # ブロック内部 = `{` の次の行 〜 `}` の前の行（0-based [BraceLine, End-1)）
    $d = Get-ItemDigest -Lines $originalLines -From $b.BraceLine -To ($b.End - 1) `
        -Origin ('{0}:mod {1}' -f $OriginalPath, $b.Name)
    Merge-Digest $before $d
}

# ------------------------------------------------------------ 移設後ファイル群の取得
$after = New-EmptyDigest
$relResolved = @()
foreach ($rp in $RelocatedPath) {
    $p = Resolve-Input $rp
    if (-not (Test-Path -LiteralPath $p)) { Write-Error "移設後ファイルが見つかりません: $p" -ErrorAction Continue; exit 2 }
    $relResolved += $p
    $lines = Read-RustLines -Path $p
    $d = Get-ItemDigest -Lines $lines -From 0 -To $lines.Length -Origin $rp
    Merge-Digest $after $d
}

# ------------------------------------------------------------------------ 突合
$report = New-Object System.Collections.Generic.List[string]

function Get-FirstDiffLine([string]$a, [string]$b) {
    $la = $a -split "`n"
    $lb = $b -split "`n"
    $n = [Math]::Max($la.Count, $lb.Count)
    for ($k = 0; $k -lt $n; $k++) {
        $x = if ($k -lt $la.Count) { $la[$k] } else { '<行なし>' }
        $y = if ($k -lt $lb.Count) { $lb[$k] } else { '<行なし>' }
        if ($x -cne $y) { return ('    正規化 {0} 行目:{1}      before: {2}{1}      after : {3}' -f ($k + 1), [Environment]::NewLine, $x, $y) }
    }
    return '    （差分行を特定できませんでした）'
}

# --- テスト関数: 識別子キーで 1:1（同名重複は多重集合として突合）
$allKeys = New-Object System.Collections.Generic.HashSet[string]
foreach ($k in $before.TestBodies.Keys) { [void]$allKeys.Add($k) }
foreach ($k in $after.TestBodies.Keys) { [void]$allKeys.Add($k) }
$sortedKeys = [string[]]@($allKeys)
if ($sortedKeys.Count -gt 1) { [Array]::Sort($sortedKeys, [System.StringComparer]::Ordinal) }

foreach ($k in $sortedKeys) {
    $hasB = $before.TestBodies.ContainsKey($k)
    $hasA = $after.TestBodies.ContainsKey($k)
    if (-not $hasA) {
        $report.Add(('[TEST-MISSING] テスト関数 `{0}` が移設後に存在しません（移設前 {1} 箇所）' -f $k, $before.TestBodies[$k].Count))
        continue
    }
    if (-not $hasB) {
        $report.Add(('[TEST-EXTRA]   テスト関数 `{0}` が移設前に存在しません（移設後 {1} 箇所: {2}）' -f $k, $after.TestBodies[$k].Count, (($after.TestBodies[$k] | ForEach-Object { $_.Origin }) -join ', ')))
        continue
    }

    $bl = @($before.TestBodies[$k])
    $al = @($after.TestBodies[$k])
    if ($bl.Count -ne $al.Count) {
        $report.Add(('[TEST-COUNT]   テスト関数 `{0}` の出現数が違います: 移設前 {1} / 移設後 {2}' -f $k, $bl.Count, $al.Count))
    }
    # 同名多重集合の突合（本文一致でペアリング）
    $remaining = New-Object System.Collections.Generic.List[object]
    foreach ($r in $al) { $remaining.Add($r) }
    foreach ($rb in $bl) {
        $hit = -1
        for ($q = 0; $q -lt $remaining.Count; $q++) {
            if ($remaining[$q].Text -ceq $rb.Text) { $hit = $q; break }
        }
        if ($hit -ge 0) { $remaining.RemoveAt($hit); continue }
        if ($remaining.Count -gt 0) {
            $ra = $remaining[0]
            $report.Add(('[TEST-BODY]    テスト関数 `{0}` の本文が一致しません（before: {1}:{2}-{3} / after: {4}:{5}-{6}）' -f `
                        $k, $rb.Origin, $rb.Start, $rb.End, $ra.Origin, $ra.Start, $ra.End))
            $report.Add((Get-FirstDiffLine $rb.Text $ra.Text))
            $remaining.RemoveAt(0)
        }
        else {
            $report.Add(('[TEST-BODY]    テスト関数 `{0}` の移設前本文に対応する移設後の実体がありません（before: {1}:{2}-{3}）' -f `
                        $k, $rb.Origin, $rb.Start, $rb.End))
        }
    }
    foreach ($ra in $remaining) {
        $report.Add(('[TEST-BODY]    テスト関数 `{0}` の移設後本文に対応する移設前の実体がありません（after: {1}:{2}-{3}）' -f `
                    $k, $ra.Origin, $ra.Start, $ra.End))
    }
}

# --- ヘルパ項目: 正規化本文の多重集合
$afterHelpers = New-Object System.Collections.Generic.List[object]
foreach ($r in $after.Helpers) { $afterHelpers.Add($r) }
$missingHelpers = New-Object System.Collections.Generic.List[object]
foreach ($rb in $before.Helpers) {
    $hit = -1
    for ($q = 0; $q -lt $afterHelpers.Count; $q++) {
        if ($afterHelpers[$q].Text -ceq $rb.Text) { $hit = $q; break }
    }
    if ($hit -ge 0) { $afterHelpers.RemoveAt($hit) } else { $missingHelpers.Add($rb) }
}
foreach ($rb in $missingHelpers) {
    $head = ($rb.Text -split "`n")[0]
    $report.Add(('[ITEM-MISSING] 非テスト項目が移設後に見当たりません（before: {0}:{1}-{2}）: {3}' -f $rb.Origin, $rb.Start, $rb.End, $head))
}
foreach ($ra in $afterHelpers) {
    $head = ($ra.Text -split "`n")[0]
    $report.Add(('[ITEM-EXTRA]   移設前に対応の無い非テスト項目があります（after: {0}:{1}-{2}）: {3}' -f $ra.Origin, $ra.Start, $ra.End, $head))
}

# ------------------------------------------------------------------------ 判定
if ($report.Count -gt 0) {
    foreach ($line in $report) { Write-Output $line }
    # 先頭が空白の行は直前の指摘に対する詳細（差分行の提示）なので件数に数えない
    $findingCount = @($report | Where-Object { $_ -notmatch '^\s' }).Count
    Write-Output ('MISMATCH: {0} 件の差分 ({1} -> {2})' -f $findingCount, $OriginalPath, ($RelocatedPath -join ', '))
    exit 1
}

if ($Detail) {
    $bCount = 0; foreach ($k in $before.TestBodies.Keys) { $bCount += $before.TestBodies[$k].Count }
    $aCount = 0; foreach ($k in $after.TestBodies.Keys) { $aCount += $after.TestBodies[$k].Count }
    Write-Output ('MATCH: test fn {0}={1} / helper item {2}={3} / mod block {4} / files {5}' -f `
                  $bCount, $aCount, $before.Helpers.Count, $after.Helpers.Count, @($blocks).Count, @($relResolved).Count)
}
exit 0
