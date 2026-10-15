<#
.SYNOPSIS
  .kiro/steering/roadmap.md を読み、版の段ごとの帯とカードの 1 枚の HTML（doc/roadmap-overview-{updated_at}.html）を作る。

.DESCRIPTION
  roadmap.md は読むだけで、書き替えない。画面へ出す文は ASCII だけ。
  端末の文字コードは読みも書きもしない。今のフォルダを変えない。呼ばれた場所によらず、このスクリプトの在るリポジトリを見る。
  日本語の読み取りの目印と HTML の決まった文言は、文言のファイル（STRINGS_FILE）の 1 か所だけに置く。

    （引数なし） [-Open]
        roadmap.md から HTML を作る。-Open を付けると、書けたときだけ OS の既定のアプリで開く。
          FAIL strings: {理由}                   文言のファイルが読めない・鍵が欠けるか空
    -SelfTest
        自分の検査。判定を 1 つずつ「ok selftest {名前}」か「FAIL selftest {名前}」で印字する。
          strings  文言のファイルが読めて、markers と labels の鍵がそろい、どれも空でない
                   （鍵を 1 つ外した写しと、1 つを空にした写しが落ちることも確かめる）

  終了コード: 0 通った／1 文言のファイルが読めない・落ちた判定が在る

  較正値（変更はスクリプト冒頭の「較正値」の 1 か所だけ）:
    STRINGS_FILE  文言のファイル（このスクリプトと同じフォルダ）
    MARKER_KEYS   読み取りの目印（markers）の鍵
    LABEL_KEYS    HTML の決まった文言（labels）の鍵
    SECTION_PREFIX 節の見出しの頭（「## 」）
    TABLE_RULE    表の区切り行（|---）に当てる正規表現
    ONE_LINER_MAX カードの一言の上限の字数（越えれば切って「…」を足す）

.PARAMETER Open
  書けたときだけ、作った HTML を OS の既定のアプリ（ブラウザ）で開く。

.PARAMETER SelfTest
  自分の検査だけを回す。何も書かない。

.EXAMPLE
  pwsh -NoProfile -File tools/roadmap-overview.ps1 -Open
  pwsh -NoProfile -File tools/roadmap-overview.ps1 -SelfTest
#>
#Requires -Version 7
# 知らない引数（綴り違いを含む）は黙って捨てずに断る
[CmdletBinding(DefaultParameterSetName = 'Build')]
param(
    [Parameter(ParameterSetName = 'Build')][switch]$Open,
    [Parameter(ParameterSetName = 'SelfTest', Mandatory)][switch]$SelfTest
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# =============================================================================
# 較正値（説明の一覧と対応。変更はここだけ）
# =============================================================================
$STRINGS_FILE = 'roadmap-overview.strings.json'
$MARKER_KEYS  = 'secVersions', 'secLedger', 'secThemes', 'secWaves', 'secSummary', 'groupSep', 'headSep', 'chainSep',
                'listSep', 'brackets', 'countRegex', 'classRegex', 'themeCut', 'sentenceEnd', 'ellipsis'
$LABEL_KEYS   = 'title', 'sourceUpdated', 'sourceBlob', 'sourceBlobUnknown', 'bandCount', 'bandCountMismatch',
                'warningCount', 'wave', 'done', 'notInLedger', 'notInBand', 'otherBands', 'prereqs', 'notInRoadmap',
                'tier', 'cls', 'size', 'order', 'theme', 'noTheme', 'clearFilter', 'completedHeading',
                'colWave', 'colDate', 'colUnits', 'colSummary', 'completedExtra', 'what', 'note'
$SECTION_PREFIX = '## '
$TABLE_RULE     = '^\|\s*:?-'
$ONE_LINER_MAX  = 60

$root = Split-Path $PSScriptRoot -Parent

# ファイルを UTF-8 で読み、改行を LF にそろえて返す。無ければ $null
function Read-Lf([string]$Path) {
    if (-not [System.IO.File]::Exists($Path)) { return $null }
    [System.IO.File]::ReadAllText($Path, [System.Text.UTF8Encoding]::new($false)).Replace("`r`n", "`n")
}

# BOM なしの UTF-8 で書く（親のフォルダが無ければ作る）
function Write-Utf8([string]$Path, [string]$Text) {
    [void][System.IO.Directory]::CreateDirectory((Split-Path $Path -Parent))
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

# 文言（ConvertFrom-Json の結果）の欠けを「markers.secLedger」の形で全部返す（空なら通った）。
# 値は空でない文字列か、空でない文字列だけの空でない並び
function Test-Strings($Strings) {
    foreach ($group in @{ markers = $MARKER_KEYS; labels = $LABEL_KEYS }.GetEnumerator() | Sort-Object Key) {
        $obj = $Strings -is [pscustomobject] ? $Strings.PSObject.Properties[$group.Key]?.Value : $null
        foreach ($k in $group.Value) {
            $v = $obj -is [pscustomobject] ? $obj.PSObject.Properties[$k]?.Value : $null
            $items = @($v)
            # 1 要素の並びの真偽は中身で決まる（@('') は偽）ので、数で見る
            $ok = $null -ne $v -and $items.Count -gt 0 -and
                  @($items | Where-Object { $_ -isnot [string] -or $_ -eq '' }).Count -eq 0
            if (-not $ok) { "$($group.Key).$k" }
        }
    }
}

# 文言を UTF-8 で読んで返す。読めない・鍵が欠けるか空のときは throw（文は ASCII）
function Read-Strings {
    $text = Read-Lf (Join-Path $PSScriptRoot $STRINGS_FILE)
    if ($null -eq $text) { throw "cannot read tools/$STRINGS_FILE" }
    try { $s = $text | ConvertFrom-Json } catch { throw "tools/$STRINGS_FILE is not valid JSON" }
    $bad = @(Test-Strings $s)
    if ($bad.Count) { throw "missing or empty key(s) in tools/${STRINGS_FILE}: $($bad -join ', ')" }
    $s
}

# 表の 1 行を欄に割る。先頭と末尾の | は在っても無くてもよい。バッククォートの中の | と \| は区切りにしない
# （\| は | に直す）。欄は前後の空白を外す
function Split-Row([string]$Line) {
    $s = $Line.Trim()
    $cells = [System.Collections.Generic.List[string]]::new()
    $cur = [System.Text.StringBuilder]::new()
    $inCode = $false
    $endsWithSep = $false
    for ($i = 0; $i -lt $s.Length; $i++) {
        $ch = $s[$i]
        $endsWithSep = $false
        if ($ch -eq '\' -and $i + 1 -lt $s.Length -and $s[$i + 1] -eq '|') { [void]$cur.Append('|'); $i++; continue }
        if ($ch -eq '`') { $inCode = -not $inCode }
        if ($ch -eq '|' -and -not $inCode) {
            $cells.Add($cur.ToString().Trim()); [void]$cur.Clear(); $endsWithSep = $true; continue
        }
        [void]$cur.Append($ch)
    }
    if (-not $endsWithSep) { $cells.Add($cur.ToString().Trim()) }
    if ($s.StartsWith('|') -and $cells.Count) { $cells.RemoveAt(0) }
    [string[]]$cells
}

# 括弧の深さ 0 の所だけで、$Seps のどれかの字で区切る。括弧の組は markers.brackets。
# バッククォートの中は 1 つの塊として扱い、括弧も区切りも数えない。
# 返り値は順序どおりの @{ Sep = 前の区切りの字（最初は ''）; Text = 区切りの間の文 }
function Split-Top([string]$Text, [string[]]$Seps, [pscustomobject]$Markers) {
    $opens = @($Markers.brackets | ForEach-Object { [string]$_[0] })
    $closes = @($Markers.brackets | ForEach-Object { [string]$_[1] })
    $parts = [System.Collections.Generic.List[object]]::new()
    $cur = [System.Text.StringBuilder]::new()
    $sep = ''
    $depth = 0
    $inCode = $false
    foreach ($c in $Text.ToCharArray()) {
        $ch = [string]$c
        if ($ch -eq '`') { $inCode = -not $inCode }
        elseif (-not $inCode) {
            if ($opens -ccontains $ch) { $depth++ }
            elseif ($closes -ccontains $ch) { if ($depth -gt 0) { $depth-- } }
            elseif ($depth -eq 0 -and $Seps -ccontains $ch) {
                $parts.Add([pscustomobject]@{ Sep = $sep; Text = $cur.ToString() })
                [void]$cur.Clear(); $sep = $ch; continue
            }
        }
        [void]$cur.Append($ch)
    }
    $parts.Add([pscustomobject]@{ Sep = $sep; Text = $cur.ToString() })
    [object[]]$parts
}

# 「## {marker}」で始まる節（次の「## 」の前まで）の最初の表を返す。節か表が無ければ $null。
# 見出し行と区切り行（|---）は Rows に入れない。表は「|」で始まる行が続く所まで。Line は 1 から数えた行番号
function Get-Table([string[]]$Lines, [string]$SectionMarker) {
    $start = -1
    for ($i = 0; $i -lt $Lines.Count; $i++) {
        if ($Lines[$i].StartsWith($SECTION_PREFIX + $SectionMarker)) { $start = $i; break }
    }
    if ($start -lt 0) { return $null }
    $i = $start + 1
    while ($i -lt $Lines.Count -and -not $Lines[$i].StartsWith($SECTION_PREFIX) -and -not $Lines[$i].StartsWith('|')) { $i++ }
    if ($i -ge $Lines.Count -or -not $Lines[$i].StartsWith('|')) { return $null }
    $width = @(Split-Row $Lines[$i]).Count
    $i++
    if ($i -lt $Lines.Count -and $Lines[$i] -match $TABLE_RULE) { $i++ }
    $rows = [System.Collections.Generic.List[object]]::new()
    for (; $i -lt $Lines.Count -and $Lines[$i].StartsWith('|'); $i++) {
        $rows.Add([pscustomobject]@{ Line = $i + 1; Cells = [string[]](Split-Row $Lines[$i]) })
    }
    [pscustomobject]@{ Width = $width; Rows = [object[]]$rows }
}

# ** とバッククォートを外した地の文（前後の空白も外す）
function ConvertTo-Plain([string]$Text) { ($Text -replace '\*\*', '' -replace '`', '').Trim() }

# 一言: （）の組を入れ子ごと外す → ConvertTo-Plain → 深さ 0 の最初の「。」の前まで → 前後の空白を外す →
# ONE_LINER_MAX 字（書記素で数える）を越えれば切って「…」を足す
function Get-OneLiner([string]$Text, [pscustomobject]$Markers) {
    $open, $close = [string]$Markers.brackets[0][0], [string]$Markers.brackets[0][1]
    $sb = [System.Text.StringBuilder]::new()
    $depth = 0
    foreach ($c in $Text.ToCharArray()) {
        $ch = [string]$c
        if ($ch -ceq $open) { $depth++ }
        elseif ($ch -ceq $close -and $depth -gt 0) { $depth-- }
        elseif ($depth -eq 0) { [void]$sb.Append($ch) }
    }
    $t = (Split-Top (ConvertTo-Plain $sb.ToString()) @($Markers.sentenceEnd) $Markers)[0].Text.Trim()
    $info = [System.Globalization.StringInfo]::new($t)
    if ($info.LengthInTextElements -le $ONE_LINER_MAX) { return $t }
    $info.SubstringByTextElements(0, $ONE_LINER_MAX) + $Markers.ellipsis
}

if ($SelfTest) {
    $script:selfFails = 0
    function Judge([string]$Name, [bool]$Ok) {
        if ($Ok) { "ok selftest $Name" } else { $script:selfFails++; "FAIL selftest $Name" }
    }

    # ---- 文言の鍵がそろう ----
    $strings = $null
    try { $strings = Read-Strings } catch { }
    # 鍵を 1 つ外した写しと 1 つを空にした写しが、その鍵だけで落ちる（判定の空振りを防ぐ）
    $caught = $false
    if ($null -ne $strings) {
        $cut = $strings | ConvertTo-Json -Depth 10 | ConvertFrom-Json
        $cut.markers.PSObject.Properties.Remove('secLedger')
        $empty = $strings | ConvertTo-Json -Depth 10 | ConvertFrom-Json
        $empty.labels.title = ''
        $caught = ((@(Test-Strings $cut) -join ',') -ceq 'markers.secLedger') -and
                  ((@(Test-Strings $empty) -join ',') -ceq 'labels.title')
    }
    Judge 'strings' $caught

    # ---- 共通の割り方（仮の判定。2.5 で見本の判定に置き替えて外す） ----
    # 日本語の字は文言の目印から組み立てる（このファイルを ASCII に保つ）
    function Same($Got, [string[]]$Want) { $g = @($Got); $g.Count -eq $Want.Count -and ($g -join "`n") -ceq ($Want -join "`n") }
    function Try-Judge([string]$Name, [scriptblock]$Body) { $ok = $false; try { $ok = [bool](& $Body) } catch { }; Judge $Name $ok }
    $m = ${strings}?.markers

    Try-Judge 'split-row' {
        (Same (Split-Row '| a | `x|y` | b\|c |') 'a', '`x|y`', 'b|c') -and
        (Same (Split-Row 'a | b') 'a', 'b') -and
        (Same (Split-Row '| a | b') 'a', 'b') -and
        (Same (Split-Row '| a |  |') 'a', '') -and
        (Same (Split-Row '| a | b\|') 'a', 'b|')
    }
    Try-Judge 'split-top' {
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $o2, $c2 = $m.brackets[1][0], $m.brackets[1][1]
        $q, $q2 = $m.brackets[2][0], $m.brackets[2][1]
        $l, $ch = $m.listSep, $m.chainSep
        $text = "a$l b$o c$l d$o2$q e$ch f$q2$c2$c$ch ``g$l h$o``$l i"
        $got = Split-Top $text @($l, $ch) $m | ForEach-Object { "$($_.Sep)|$($_.Text)" }
        Same $got "|a", "$l| b$o c$l d$o2$q e$ch f$q2$c2$c", "$ch| ``g$l h$o``", "$l| i"
    }
    Try-Judge 'get-table' {
        $lines = '# t', '## Other', '| A | B | C |', '|---|---|---|', '| no | no | no |', '## Sec', 'text',
                 '| A | B | C |', '|---|---|---|', '| 1 | 2 | 3 |', '| 4 | 5', 'after', '| z | z | z |',
                 '### sub', '## Empty', 'nothing', '## Next'
        $t = Get-Table $lines 'Sec'
        $t.Width -eq 3 -and $t.Rows.Count -eq 2 -and $t.Rows[0].Line -eq 10 -and $t.Rows[1].Line -eq 11 -and
        (Same $t.Rows[0].Cells '1', '2', '3') -and (Same $t.Rows[1].Cells '4', '5') -and
        $null -eq (Get-Table $lines 'Empty') -and $null -eq (Get-Table $lines 'Gone') -and
        (Get-Table $lines 'Other').Rows[0].Cells[0] -ceq 'no'
    }
    Try-Judge 'convert-plain' { (ConvertTo-Plain ' **a** `b` c ') -ceq 'a b c' }
    Try-Judge 'one-liner' {
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $q, $q2 = $m.brackets[2][0], $m.brackets[2][1]
        $e, $dots = $m.sentenceEnd, $m.ellipsis
        (Get-OneLiner " ab$o x$o y$c z$c **c** ``d``$e tail" $m) -ceq 'ab c d' -and
        (Get-OneLiner "p$q q$e r$q2 s$e t" $m) -ceq "p$q q$e r$q2 s" -and
        (Get-OneLiner ('x' * 61) $m) -ceq (('x' * 60) + $dots) -and
        (Get-OneLiner ('x' * 60) $m) -ceq ('x' * 60)
    }

    if ($script:selfFails) { "selftest: $($script:selfFails) failure(s)"; exit 1 }
    'selftest: all passed'
    exit 0
}

# ---- 作る呼び方 ----
try { $strings = Read-Strings }
catch { "FAIL strings: $($_.Exception.Message -replace '[^\x20-\x7E]', '?')"; exit 1 }
# 読み手と HTML の組み立ては後のタスクで入る。それまでは黙って通さず落ちる
'FAIL build: the reader and the html assembler are not implemented yet'
exit 1
