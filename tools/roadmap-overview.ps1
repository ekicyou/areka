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
    SPEC_PREFIX   spec 名の頭（外してから SPEC_NAME に当てる）
    SPEC_NAME     spec 名に見える語に当てる正規表現（当たらないバッククォートの語はファイル名などとして読み飛ばす）
    THEME_SPECS   テーマの節で読む #### 見出し（ほかの #### の下、Existing Spec Updates などは読まない）
    THEME_ROW_SEP テーマの行の名前の並びと残りの区切り
    THEME_NAME_SEP テーマの行の名前の並びの区切り
    THEME_DEPS    テーマの行の残りの、一言の元と前提の文の区切り（最後のもの）

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
$SPEC_PREFIX    = 'areka-P0-'
$SPEC_NAME      = '^[a-z0-9]+(-[a-z0-9]+)+$'
$THEME_SPECS    = '#### Specs (dependency order)'
$THEME_ROW_SEP  = ' -- '
$THEME_NAME_SEP = ', '
$THEME_DEPS     = 'Dependencies:'

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

# バッククォートの語が spec 名に見えれば、頭の SPEC_PREFIX を外した名前を返す。見えなければ $null
function Get-SpecName([string]$Word) {
    $w = $Word.StartsWith($SPEC_PREFIX) ? $Word.Substring($SPEC_PREFIX.Length) : $Word
    $w -cmatch $SPEC_NAME ? $w : $null
}

# 文の中のバッククォートの語のうち spec 名に見えるものを、出る順に返す（頭の SPEC_PREFIX は外す）
function Get-CodeNames([string]$Text) {
    @([regex]::Matches($Text, '`([^`]*)`') | ForEach-Object { Get-SpecName $_.Groups[1].Value } | Where-Object { $_ })
}

# 括弧の組（markers.brackets）の中を入れ子ごと外した、深さ 0 の文。
# バッククォートの中は塊として残し、中の括弧は数えない
function Get-TopText([string]$Text, [pscustomobject]$Markers) {
    $opens = @($Markers.brackets | ForEach-Object { [string]$_[0] })
    $closes = @($Markers.brackets | ForEach-Object { [string]$_[1] })
    $sb = [System.Text.StringBuilder]::new()
    $depth = 0
    $inCode = $false
    foreach ($c in $Text.ToCharArray()) {
        $ch = [string]$c
        if ($ch -eq '`') { $inCode = -not $inCode }
        elseif (-not $inCode) {
            if ($opens -ccontains $ch) { $depth++; continue }
            if ($closes -ccontains $ch) { if ($depth -gt 0) { $depth-- }; continue }
        }
        if ($depth -eq 0) { [void]$sb.Append($ch) }
    }
    $sb.ToString()
}

# 表の行の欄を、見出し行の欄の数（$Width）の長さ以上にそろえて返す。数が違えば table-columns の警告を
# $Warnings へ足す（足りない欄は空として読む・余りの欄は残す）。$Key は表の鍵（version-table など）
function Get-Cells($Row, [int]$Width, [string]$Key, $Warnings) {
    $cells = [string[]]$Row.Cells
    if ($cells.Count -ne $Width) {
        $Warnings.Add([pscustomobject]@{ Key = 'table-columns'; Detail = "$Key line $($Row.Line): $($cells.Count) cell(s), expected $Width" })
    }
    if ($cells.Count -lt $Width) { $cells = [string[]](@($cells) + @('') * ($Width - $cells.Count)) }
    $cells
}

# 版の段の表を帯の並びに読む。節か表が無ければ $null（欠け version-table は呼ぶ側が出す）。
# 返り値 @{ Bands = 帯（表の行の順）; Warnings = band-count の警告 }。カードは名前・連鎖・注だけを持つ
# （ほかの欄は台帳とテーマを読んだ後の組み立てで足す）
function Get-Bands([string[]]$Lines, [pscustomobject]$Markers) {
    $table = Get-Table $Lines $Markers.secVersions
    if ($null -eq $table) { return $null }
    $opens = @($Markers.brackets | ForEach-Object { [string]$_[0] })
    $closes = @($Markers.brackets | ForEach-Object { [string]$_[1] })
    $bands = [System.Collections.Generic.List[object]]::new()
    $warnings = [System.Collections.Generic.List[object]]::new()
    foreach ($row in $table.Rows) {
        $cells = Get-Cells $row $table.Width 'version-table' $warnings
        $spec = $cells[2]
        # 深さ 0 の字の印（括弧の組とバッククォートの中は深さ 0 にしない）
        $top = [bool[]]::new($spec.Length)
        $depth = 0; $inCode = $false
        for ($i = 0; $i -lt $spec.Length; $i++) {
            $chr = [string]$spec[$i]
            if ($chr -eq '`') { $inCode = -not $inCode; continue }
            if ($inCode) { continue }
            if ($opens -ccontains $chr) { $depth++; continue }
            if ($closes -ccontains $chr) { if ($depth -gt 0) { $depth-- }; continue }
            $top[$i] = $depth -eq 0
        }
        # 深さ 0 の最後の「＝**N 本**」が書かれた数。その前だけが並びの文
        $declared = $null
        $list = $spec
        $hit = @([regex]::Matches($spec, $Markers.countRegex) | Where-Object { $top[$_.Index] }) | Select-Object -Last 1
        if ($hit) { $declared = [int]$hit.Groups[1].Value; $list = $spec.Substring(0, $hit.Index) }

        $groups = [System.Collections.Generic.List[object]]::new()
        $read = 0
        foreach ($grp in Split-Top $list @($Markers.groupSep) $Markers) {
            $parts = @(Split-Top $grp.Text @($Markers.headSep) $Markers)
            $heading = $null
            $body = $grp.Text
            if ($parts.Count -gt 1) {
                $heading = ConvertTo-Plain $parts[0].Text
                $body = ($parts[1..($parts.Count - 1)] | ForEach-Object { $_.Sep + $_.Text }) -join ''
                $body = $body.Substring($Markers.headSep.Length)
            }
            $cards = [System.Collections.Generic.List[object]]::new()
            foreach ($el in Split-Top $body @($Markers.chainSep, $Markers.listSep) $Markers) {
                $t = $el.Text.TrimStart()
                if (-not $t.StartsWith('`')) { continue }
                $end = $t.IndexOf('`', 1)
                if ($end -lt 0) { continue }
                $name = Get-SpecName $t.Substring(1, $end - 1)
                if ($null -eq $name) { continue }
                $note = ConvertTo-Plain $t.Substring($end + 1)
                $cards.Add([pscustomobject][ordered]@{
                    name = $name; chained = $el.Sep -ceq $Markers.chainSep; note = $note -eq '' ? $null : $note
                })
            }
            $read += $cards.Count
            $groups.Add([pscustomobject][ordered]@{ heading = $heading; cards = [object[]]$cards })
        }
        $bands.Add([pscustomobject][ordered]@{
            name = ConvertTo-Plain $cells[0]; goal = ConvertTo-Plain $cells[1]
            declared = $declared; read = $read; notInBand = $false; groups = [object[]]$groups
        })
        if ($declared -ne $read) {
            $warnings.Add([pscustomobject]@{ Key = 'band-count'; Detail = "band $($bands.Count): declared $($declared ?? '?'), read $read" })
        }
    }
    [pscustomobject]@{ Bands = [object[]]$bands; Warnings = [object[]]$warnings }
}

# spec 台帳の表を行の並びに読む。節か表が無ければ $null（欠け ledger-table は呼ぶ側が出す）。
# 7 欄＝spec・段・何をするか・規模・並び・前提・Fable。1 欄目に spec 名に見えるバッククォートの語が無い行は読まない。
# 返り値 @{ Rows = @{ name; tier; cls; size; what; order; prereq（前提の全文の地の文）; prereqs（深さ 0 の名前） }; Warnings }
function Get-Ledger([string[]]$Lines, [pscustomobject]$Markers) {
    $table = Get-Table $Lines $Markers.secLedger
    if ($null -eq $table) { return $null }
    $rows = [System.Collections.Generic.List[object]]::new()
    $warnings = [System.Collections.Generic.List[object]]::new()
    foreach ($row in $table.Rows) {
        $c = Get-Cells $row $table.Width 'ledger-table' $warnings
        # 1 欄目の最初のバッククォートの語だけを名前と見る（2 つ目以降の語は注）
        $first = [regex]::Match($c[0], '`([^`]*)`')
        $name = $first.Success ? (Get-SpecName $first.Groups[1].Value) : $null
        if ($null -eq $name) { continue }
        $cls = [regex]::Match($c[1], $Markers.classRegex)
        $rows.Add([pscustomobject][ordered]@{
            name = $name
            tier = ConvertTo-Plain (Split-Top $c[1] @($Markers.listSep) $Markers)[0].Text
            cls = $cls.Success ? $cls.Groups[1].Value : $null
            size = ConvertTo-Plain $c[3]; what = ConvertTo-Plain $c[2]; order = ConvertTo-Plain $c[4]
            prereq = ConvertTo-Plain $c[5]; prereqs = [string[]]@(Get-CodeNames (Get-TopText $c[5] $Markers))
        })
    }
    [pscustomobject]@{ Rows = [object[]]$rows; Warnings = [object[]]$warnings }
}

# テーマ別の決めごとの節を読む。節が無ければ $null（欠け themes は呼ぶ側が出す）。
# ### 見出しごとに、THEME_SPECS の #### の下の「- [{印}] {名前の並び} -- {残り}」の行だけを読む。
# 返り値 @{ Themes = Specs の行を 1 つ以上持つ見出しの名前（出る順）; Rows = @{ names; done; theme（Themes の添字）;
#   source（一言の元）; prereqs（前提の文の深さ 0 の SPEC_PREFIX 付きの名前） } }
function Get-Themes([string[]]$Lines, [pscustomobject]$Markers) {
    $start = -1
    for ($i = 0; $i -lt $Lines.Count; $i++) {
        if ($Lines[$i].StartsWith($SECTION_PREFIX + $Markers.secThemes)) { $start = $i; break }
    }
    if ($start -lt 0) { return $null }
    $themes = [System.Collections.Generic.List[string]]::new()
    $rows = [System.Collections.Generic.List[object]]::new()
    $heading = ''
    $theme = -1
    $inSpecs = $false
    $prefixed = [regex]::Escape($SPEC_PREFIX) + '([a-z0-9]+(?:-[a-z0-9]+)*)'
    for ($i = $start + 1; $i -lt $Lines.Count -and -not $Lines[$i].StartsWith($SECTION_PREFIX); $i++) {
        $line = $Lines[$i]
        if ($line.StartsWith('### ')) {
            $h = $line.Substring(4)
            $cut = $h.IndexOf($Markers.themeCut)
            $heading = ConvertTo-Plain ($cut -ge 0 ? $h.Substring(0, $cut) : $h)
            $theme = -1; $inSpecs = $false; continue
        }
        if ($line.StartsWith('#### ')) { $inSpecs = $line.TrimEnd() -ceq $THEME_SPECS; continue }
        if (-not $inSpecs -or $line -notmatch '^- \[(.)\]\s*(.*)$') { continue }
        $mark, $body = $Matches[1], $Matches[2]
        $sep = $body.IndexOf($THEME_ROW_SEP)
        $list, $rest = $sep -ge 0 ? @($body.Substring(0, $sep), $body.Substring($sep + $THEME_ROW_SEP.Length)) : @($body, '')
        $d = $rest.LastIndexOf($THEME_DEPS)
        $source, $deps = $d -ge 0 ? @($rest.Substring(0, $d), $rest.Substring($d + $THEME_DEPS.Length)) : @($rest, '')
        $source = $source.Trim()
        if ($source.EndsWith($Markers.sentenceEnd)) { $source = $source.Substring(0, $source.Length - $Markers.sentenceEnd.Length).TrimEnd() }
        if ($theme -lt 0) { $themes.Add($heading); $theme = $themes.Count - 1 }
        $rows.Add([pscustomobject][ordered]@{
            names = [string[]]@($list -split [regex]::Escape($THEME_NAME_SEP) | ForEach-Object { Get-SpecName $_.Trim() } | Where-Object { $_ })
            done = $mark -ceq 'x'; theme = $theme; source = $source
            prereqs = [string[]]@([regex]::Matches((Get-TopText $deps $Markers), $prefixed) |
                ForEach-Object { Get-SpecName $_.Value } | Where-Object { $_ })
        })
    }
    [pscustomobject]@{ Themes = [string[]]$themes; Rows = [object[]]$rows }
}

# ウェーブ編成の表の最初の行（いま走っているウェーブ）を読む。節か表が無ければ $null（欠け wave-table は呼ぶ側が出す）。
# 返り値 @{ Name = 1 欄目の最初の **…** の中（無ければ欄の地の文・行が無ければ $null）; Names = 2 欄目の spec 名; Warnings }
function Get-Wave([string[]]$Lines, [pscustomobject]$Markers) {
    $table = Get-Table $Lines $Markers.secWaves
    if ($null -eq $table) { return $null }
    $warnings = [System.Collections.Generic.List[object]]::new()
    $name = $null
    $names = [string[]]@()
    if ($table.Rows.Count) {
        $c = Get-Cells $table.Rows[0] $table.Width 'wave-table' $warnings
        $bold = [regex]::Match($c[0], '\*\*(.+?)\*\*')
        $name = ConvertTo-Plain ($bold.Success ? $bold.Groups[1].Value : $c[0])
        $names = [string[]]@(Get-CodeNames $c[1])
    }
    [pscustomobject]@{ Name = $name; Names = $names; Warnings = [object[]]$warnings }
}

# 完了サマリの節の最初の表を読む（節の後の別の表は読まない）。節か表が無ければ $null（欠け summary-table は呼ぶ側が出す）。
# 4 欄＝Wave・完了日・ユニット（PR）・一行要約。返り値 @{ Rows = @{ wave; date; units; summary（地の文）;
#   names（ユニットの欄の、深さを問わないバッククォートの spec 名＝完了） }; Warnings }
function Get-Summary([string[]]$Lines, [pscustomobject]$Markers) {
    $table = Get-Table $Lines $Markers.secSummary
    if ($null -eq $table) { return $null }
    $rows = [System.Collections.Generic.List[object]]::new()
    $warnings = [System.Collections.Generic.List[object]]::new()
    foreach ($row in $table.Rows) {
        $c = Get-Cells $row $table.Width 'summary-table' $warnings
        $rows.Add([pscustomobject][ordered]@{
            wave = ConvertTo-Plain $c[0]; date = ConvertTo-Plain $c[1]; units = ConvertTo-Plain $c[2]
            summary = ConvertTo-Plain $c[3]; names = [string[]]@(Get-CodeNames $c[2])
        })
    }
    [pscustomobject]@{ Rows = [object[]]$rows; Warnings = [object[]]$warnings }
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

    # ---- 版の段の読み方（仮の判定。2.5 で見本の判定に置き替えて外す） ----
    # 版の段だけを持つ小さな断片。帯 1: 括弧の中の・＝→名前・ファイル名・地の文・最後の深さ 0 の数・数の後ろの名前と括弧の中の数
    # 帯 2: 小見出し（中に名前）・〔〕の入れ子・／の組・数が無い。帯 3: 数と読んだ数が違う
    $script:bandFrag = $null
    if ($null -ne $m) { try {
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $o2, $c2 = $m.brackets[1][0], $m.brackets[1][1]
        $l, $ch, $eq, $g = $m.listSep, $m.chainSep, $m.headSep, $m.groupSep
        $hon = [string][char]0x672C
        $script:bandFrag = Get-Bands @(
            '# t', "$SECTION_PREFIX$($m.secVersions)", 'text', '| v | g | s |', '|---|---|---|',
            "| **v1** | ``g`` one | ``a-one`` $ch ``areka-P0-b-two``${o}n$l m$eq``z-z``${ch}w$c$l ``c.rs``$l plain$l ``d-four``$eq**3 $hon**. tail ``e-five``$o$eq**7 $hon**$c |",
            "| v2 | g2 | h``q-q``$o``r-r``$c$eq``f-six``$ch``g-seven``$o$o2 note$c2$c$g ``h-eight`` |",
            "| v3 | g3 | ``i-nine``$eq**2 $hon** |",
            '', '## Next') $m
    } catch { } }
    function Show-Card($Card) { "$($Card.name)|$($Card.chained)|$($Card.note)" }
    Try-Judge 'bands-groups' {
        $b = $script:bandFrag.Bands
        $b.Count -eq 3 -and (Same ($b | ForEach-Object { "$($_.name)|$($_.goal)|$($_.notInBand)|$(@($_.groups).Count)" }) 'v1|g one|False|1', 'v2|g2|False|2', 'v3|g3|False|1') -and
        (Same ($b[1].groups[1].cards | ForEach-Object { $_.name }) 'h-eight') -and $null -eq $b[1].groups[1].heading
    }
    Try-Judge 'bands-heading' {
        $b = $script:bandFrag.Bands
        $null -eq $b[0].groups[0].heading -and $b[1].groups[0].heading -ceq "hq-q$($m.brackets[0][0])r-r$($m.brackets[0][1])" -and
        (Same ($b[1].groups[0].cards | ForEach-Object { $_.name }) 'f-six', 'g-seven')
    }
    Try-Judge 'bands-chain' {
        $b = $script:bandFrag.Bands
        (Same ($b[0].groups[0].cards | ForEach-Object { "$($_.name)|$($_.chained)" }) 'a-one|False', 'b-two|True', 'd-four|False') -and
        (Same ($b[1].groups[0].cards | ForEach-Object { "$($_.name)|$($_.chained)" }) 'f-six|False', 'g-seven|True')
    }
    Try-Judge 'bands-note' {
        $b = $script:bandFrag.Bands
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $o2, $c2 = $m.brackets[1][0], $m.brackets[1][1]
        (Same ($b[0].groups[0].cards | ForEach-Object { Show-Card $_ }) 'a-one|False|', "b-two|True|${o}n$($m.listSep) m$($m.headSep)z-z$($m.chainSep)w$c", 'd-four|False|') -and
        $null -eq $b[0].groups[0].cards[0].note -and (Show-Card $b[1].groups[0].cards[1]) -ceq "g-seven|True|$o${o2} note$c2$c"
    }
    Try-Judge 'bands-count' {
        $f = $script:bandFrag
        (Same ($f.Bands | ForEach-Object { "$($_.declared)|$($_.read)" }) '3|3', '|3', '2|1') -and $null -eq $f.Bands[1].declared -and
        (Same ($f.Warnings | ForEach-Object { "$($_.Key) $($_.Detail)" }) 'band-count band 2: declared ?, read 3', 'band-count band 3: declared 2, read 1') -and
        $null -eq (Get-Bands @('## Other', '| a |') $m)
    }

    # ---- 台帳・テーマ・ウェーブ・完了サマリの読み方（仮の判定。2.5 で見本の判定に置き替えて外す） ----
    function Show-Names($Names) { @($Names) -join ',' }
    Try-Judge 'ledger' {
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $l = $m.listSep
        $kubun = [string][char]0x533A + [char]0x5206
        $f = Get-Ledger @(
            '# t', "$SECTION_PREFIX$($m.secLedger)$o n$c", '> text', '| s | t | w | z | o | p | f |', '|---|---|---|---|---|---|---|',
            "| ``a-one``$o``x-x``$c | bug$l $kubun B | what **bold**$o y$c | 5 | ord | ``b-two``$l ``c.rs``$l ``areka-P0-c-three``$o``d-four``$c | - |",
            "| ``areka-P0-e-five`` | t2 | w2 |",
            '| plain | x | x | x | x | x | x |',
            '', '## Next') $m
        $r = $f.Rows
        $r.Count -eq 2 -and
        (Same ($r | ForEach-Object { "$($_.name)|$($_.tier)|$($_.cls)|$($_.size)|$($_.what)|$($_.order)" }) "a-one|bug|B|5|what bold$o y$c|ord", 'e-five|t2|||w2|') -and
        $r[0].prereq -ceq "b-two$l c.rs$l areka-P0-c-three${o}d-four$c" -and (Show-Names $r[0].prereqs) -ceq 'b-two,c-three' -and
        $null -eq $r[1].cls -and @($r[1].prereqs).Count -eq 0 -and
        $null -eq (Get-Ledger @('## Other', '| a |') $m) -and $null -eq (Get-Ledger @("$SECTION_PREFIX$($m.secLedger)", 'no table') $m)
    }
    Try-Judge 'themes' {
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $e, $ch = $m.sentenceEnd, $m.chainSep
        $f = Get-Themes @(
            '# t', "$SECTION_PREFIX$($m.secThemes)", 'text', "### Alpha$o x$c", 'note',
            '#### Existing Spec Updates', '- [ ] areka-P0-skip-me -- no. Dependencies: none',
            '#### Specs (dependency order)', '',
            "- [x] areka-P0-a-one, areka-P0-b-two -- first$e Dependencies: areka-P0-c-three$o areka-P0-z-z$c, areka-P0-d-four",
            "- [$ch] areka-P0-e-five -- moved Dependencies: none",
            "### **Beta** two$o y$c", '#### Specs (dependency order)',
            '- [ ] areka-P0-f-six -- say Dependencies: x then Dependencies: areka-P0-a-one',
            '### Empty', '#### Existing Spec Updates', '- [x] areka-P0-g-seven -- x',
            '## Next', '#### Specs (dependency order)', '- [x] areka-P0-h-eight -- out') $m
        (Same $f.Themes 'Alpha', 'Beta two') -and
        (Same ($f.Rows | ForEach-Object { "$(Show-Names $_.names)|$($_.done)|$($_.theme)|$($_.source)|$(Show-Names $_.prereqs)" }) `
            'a-one,b-two|True|0|first|c-three,d-four', 'e-five|False|0|moved|', 'f-six|False|1|say Dependencies: x then|a-one') -and
        $null -eq (Get-Themes @('## Other', '### x') $m)
    }
    Try-Judge 'wave' {
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $f = Get-Wave @(
            "$SECTION_PREFIX$($m.secWaves)", '> x', '| Wave | Units | Cmd | Why |', '|---|---|---|---|',
            "| **C9**$o 3$c | ``a-one``$o``b-two``$c ``c.rs`` ``areka-P0-c-three`` | x | y |",
            '| **Next** | `z-z` | x | y |') $m
        $g = Get-Wave @("$SECTION_PREFIX$($m.secWaves)", '| W | U | C | Y |', '|---|---|---|---|', '| plain `w` | `q-q` | x | y |') $m
        $f.Name -ceq 'C9' -and (Show-Names $f.Names) -ceq 'a-one,b-two,c-three' -and
        $g.Name -ceq 'plain w' -and (Show-Names $g.Names) -ceq 'q-q' -and
        $null -eq (Get-Wave @('## Other', '| a |') $m)
    }
    $script:summaryFrag = $null
    if ($null -ne $m) { try {
        $o, $c = $m.brackets[0][0], $m.brackets[0][1]
        $script:summaryFrag = Get-Summary @(
            "$SECTION_PREFIX$($m.secSummary)$o x$c", '| Wave | D | U | S |', '|---|---|---|---|',
            '| W1 | 07-01 | `a-one` 23/23 | sum **b** |',
            "| W2 | 08-01 | ``b-two``$o``x``$c$($m.listSep) ``c-three``${o}PR#1$c | s2",
            "| W3 | 09-01 | idle$($m.listSep) plain | s3 |",
            '', '### rel', '| v | d |', '|---|---|', '| `z-z` | x |') $m
    } catch { } }
    Try-Judge 'summary' {
        $r = $script:summaryFrag.Rows
        (Same ($r | ForEach-Object { "$($_.wave)|$($_.date)|$($_.units)|$($_.summary)|$(Show-Names $_.names)" }) `
            'W1|07-01|a-one 23/23|sum b|a-one', "W2|08-01|b-two$($m.brackets[0][0])x$($m.brackets[0][1])$($m.listSep) c-three$($m.brackets[0][0])PR#1$($m.brackets[0][1])|s2|b-two,c-three",
            "W3|09-01|idle$($m.listSep) plain|s3|") -and
        @($script:summaryFrag.Warnings).Count -eq 0 -and
        $null -eq (Get-Summary @('## Other', '| a |') $m)
    }
    Try-Judge 'table-columns' {
        $f = Get-Ledger @("$SECTION_PREFIX$($m.secLedger)", '| s | t | w | z | o | p | f |', '|---|---|---|---|---|---|---|',
                          '| `a-one` | t | w | z | o | p | f |', '| `b-two` | t |', '| `c-three` | t | w | z | o | p | f | extra |') $m
        $s = Get-Summary @("$SECTION_PREFIX$($m.secSummary)", '| W | D | U | S |', '|---|---|---|---|', '| W1 | 07-01 |') $m
        (Same ($f.Warnings + $s.Warnings | ForEach-Object { "$($_.Key) $($_.Detail)" }) `
            'table-columns ledger-table line 5: 2 cell(s), expected 7', 'table-columns ledger-table line 6: 8 cell(s), expected 7',
            'table-columns summary-table line 4: 2 cell(s), expected 4') -and
        (Same ($f.Rows | ForEach-Object { "$($_.name)|$($_.tier)|$($_.what)|$($_.prereq)" }) 'a-one|t|w|p', 'b-two|t||', 'c-three|t|w|p') -and
        $s.Rows[0].units -ceq '' -and $s.Rows[0].summary -ceq ''
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
