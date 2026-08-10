<#
.SYNOPSIS
    areka-P0-file-slimming: 旧->新テスト名対応表が全単射であることを機械検証する。

.DESCRIPTION
    design §Data Models の不変量と Requirement 2.9 を逐条で検査する:

      1. 列が `old_fqn` / `new_fqn` / `reason` の 3 列であること
      2. `old_fqn` に重複が無いこと
      3. `new_fqn` に重複が無いこと          （2+3 で全単射）
      4. 各行の完全修飾名の**末尾セグメント**（テスト関数の識別子）が old/new で同一であること
         — 2.9 が許容するのは所属モジュールパスの変化のみで、関数の改名は許容しない
      5. `reason` が `theme_split` であること（design §Data Models の enum）
      6. `old_fqn` と `new_fqn` が同一の行が無いこと（対応表に載せる意味が無い＝誤登録）
      7. フラグメント結合時、フラグメント間でキー衝突が無いこと（どのファイル同士かを報告する）

.PARAMETER Path
    対応表 CSV、またはフラグメントを収めたディレクトリ。複数指定可。
    ディレクトリを指定した場合、その直下の `*.csv` を全て結合対象とする（再帰しない）。

.PARAMETER Out
    検証 PASS のときだけ、結合結果を単一の対応表 CSV として書き出す（タスク 7.1 の結合運用）。

.OUTPUTS
    PASS: 検査結果の要約・exit 0
    FAIL: 違反の逐条レポート・exit 1
    引数不正: エラーメッセージ・exit 2

.EXAMPLE
    pwsh -File .kiro/specs/areka-P0-file-slimming/verification/Test-MappingBijection.ps1 `
        -Path .kiro/specs/areka-P0-file-slimming/verification/mapping `
        -Out  .kiro/specs/areka-P0-file-slimming/verification/test_name_mapping.csv

.NOTES
    Requirements: 1.8, 2.9
    Design:       §Data Models, §実装 / ThemeSplit, §検証 / EvidencePipeline
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string[]]$Path,
    [string]$Out
)

$ErrorActionPreference = 'Stop'

$ALLOWED_REASONS = @('theme_split')
$REQUIRED_COLUMNS = @('old_fqn', 'new_fqn', 'reason')

# `pwsh -File` は引数を常に単一文字列として渡すため、カンマ区切りの複数指定を自前で展開する
function Expand-ListArg([string[]]$v) {
    $o = New-Object System.Collections.Generic.List[string]
    foreach ($x in $v) {
        if ($null -eq $x) { continue }
        foreach ($y in ($x -split ',')) { $t = $y.Trim(); if ($t -ne '') { $o.Add($t) } }
    }
    return $o.ToArray()
}
$Path = Expand-ListArg $Path

# ------------------------------------------------------------------ 入力ファイルの展開
$files = New-Object System.Collections.Generic.List[string]
foreach ($p in $Path) {
    if (-not (Test-Path -LiteralPath $p)) { Write-Error "パスが見つかりません: $p" -ErrorAction Continue; exit 2 }
    $item = Get-Item -LiteralPath $p
    if ($item.PSIsContainer) {
        $found = @(Get-ChildItem -LiteralPath $item.FullName -Filter '*.csv' -File | Sort-Object Name)
        foreach ($f in $found) { $files.Add($f.FullName) }
    }
    else {
        $files.Add($item.FullName)
    }
}
if ($files.Count -eq 0) {
    Write-Output 'PASS: 対応表フラグメントが 0 件（テーマ分割による改名なし）'
    if ($Out) { Set-Content -LiteralPath $Out -Value 'old_fqn,new_fqn,reason' -Encoding utf8NoBOM }
    exit 0
}

# ------------------------------------------------------------------------ 読み込み
$violations = New-Object System.Collections.Generic.List[string]
$rows = New-Object System.Collections.Generic.List[object]

foreach ($f in $files) {
    $header = (Get-Content -LiteralPath $f -TotalCount 1)
    if ($null -eq $header) {
        $violations.Add(('[HEADER] {0}: ファイルが空です（少なくともヘッダ行が必要）' -f $f))
        continue
    }
    $cols = @($header -split ',' | ForEach-Object { $_.Trim().Trim('"') })
    if (($cols -join ',') -cne ($REQUIRED_COLUMNS -join ',')) {
        $violations.Add(('[HEADER] {0}: 列が `{1}` です（期待: `{2}`）' -f $f, ($cols -join ','), ($REQUIRED_COLUMNS -join ',')))
        continue
    }

    $n = 0
    foreach ($r in @(Import-Csv -LiteralPath $f)) {
        $n++
        $rows.Add([pscustomobject]@{
                OldFqn   = ([string]$r.old_fqn).Trim()
                NewFqn   = ([string]$r.new_fqn).Trim()
                Reason   = ([string]$r.reason).Trim()
                Source   = $f
                RowIndex = $n
            })
    }
}

if ($violations.Count -gt 0) {
    foreach ($v in $violations) { Write-Output $v }
    Write-Output ('FAIL: {0} 件の違反' -f $violations.Count)
    exit 1
}

# ------------------------------------------------------------------------ 検査
function Get-LastSegment([string]$fqn) {
    $parts = $fqn -split '::'
    return $parts[$parts.Count - 1]
}

$byOld = @{}
$byNew = @{}

foreach ($r in $rows) {
    $loc = ('{0}#{1}' -f (Split-Path -Leaf $r.Source), $r.RowIndex)

    if ([string]::IsNullOrWhiteSpace($r.OldFqn) -or [string]::IsNullOrWhiteSpace($r.NewFqn)) {
        $violations.Add(('[EMPTY] {0}: old_fqn / new_fqn は空にできません (old=`{1}` new=`{2}`)' -f $loc, $r.OldFqn, $r.NewFqn))
        continue
    }
    if ($ALLOWED_REASONS -cnotcontains $r.Reason) {
        $violations.Add(('[REASON] {0}: reason=`{1}` は許容外です（許容: {2}）' -f $loc, $r.Reason, ($ALLOWED_REASONS -join ' / ')))
    }
    if ($r.OldFqn -ceq $r.NewFqn) {
        $violations.Add(('[NOOP] {0}: old_fqn と new_fqn が同一です（FQN 不変の移設は行を持たない）: `{1}`' -f $loc, $r.OldFqn))
    }

    $oldId = Get-LastSegment $r.OldFqn
    $newId = Get-LastSegment $r.NewFqn
    if ($oldId -cne $newId) {
        $violations.Add(('[IDENT] {0}: 関数識別子が変わっています（2.9 違反・許容はモジュールパスの変化のみ）: `{1}` -> `{2}`  ({3} -> {4})' -f `
                    $loc, $oldId, $newId, $r.OldFqn, $r.NewFqn))
    }

    if ($byOld.ContainsKey($r.OldFqn)) {
        $prev = $byOld[$r.OldFqn]
        $violations.Add(('[DUP-OLD] old_fqn が重複しています: `{0}`  ({1} / {2})' -f $r.OldFqn, ('{0}#{1}' -f (Split-Path -Leaf $prev.Source), $prev.RowIndex), $loc))
    }
    else { $byOld[$r.OldFqn] = $r }

    if ($byNew.ContainsKey($r.NewFqn)) {
        $prev = $byNew[$r.NewFqn]
        $violations.Add(('[DUP-NEW] new_fqn が重複しています: `{0}`  ({1} / {2})' -f $r.NewFqn, ('{0}#{1}' -f (Split-Path -Leaf $prev.Source), $prev.RowIndex), $loc))
    }
    else { $byNew[$r.NewFqn] = $r }
}

# ------------------------------------------------------------------------ 判定
if ($violations.Count -gt 0) {
    foreach ($v in $violations) { Write-Output $v }
    Write-Output ('FAIL: {0} 件の違反 / 行数 {1} / ファイル {2}' -f $violations.Count, $rows.Count, $files.Count)
    exit 1
}

Write-Output ('PASS: 全単射 OK / 行数 {0} / 相異なる old_fqn {1} / 相異なる new_fqn {2} / フラグメント {3}' -f `
              $rows.Count, $byOld.Count, $byNew.Count, $files.Count)
foreach ($f in $files) {
    $c = @($rows | Where-Object { $_.Source -eq $f }).Count
    Write-Output ('  - {0}: {1} 行' -f (Split-Path -Leaf $f), $c)
}

if ($Out) {
    # old_fqn は全単射検証済みで一意・カンマを含まないため、CSV 行そのものを序数整列すれば決定的になる
    # （notes.md §10.2 と同じ `[StringComparer]::Ordinal`。`Sort-Object` はカルチャ依存のため使わない）
    $body = [string[]]@($rows | ForEach-Object { '{0},{1},{2}' -f $_.OldFqn, $_.NewFqn, $_.Reason })
    if ($body.Count -gt 1) { [Array]::Sort($body, [System.StringComparer]::Ordinal) }
    $all = , ($REQUIRED_COLUMNS -join ',') + $body
    Set-Content -LiteralPath $Out -Value $all -Encoding utf8NoBOM
    Write-Output ('wrote: {0} ({1} 行)' -f $Out, $body.Count)
}
exit 0
