<#
.SYNOPSIS
  .kiro/steering/roadmap.md を読み、版の段ごとの帯とカードの 1 枚の HTML（doc/roadmap-overview-{updated_at}.html）を作る。

.DESCRIPTION
  roadmap.md は読むだけで、書き替えない。画面へ出す文は ASCII だけ。
  端末の文字コードは読みも書きもしない。今のフォルダを変えない。呼ばれた場所によらず、このスクリプトの在るリポジトリを見る。
  日本語の読み取りの目印と HTML の決まった文言は、文言のファイル（STRINGS_FILE）の 1 か所だけに置く。

    （引数なし） [-Open] [-OutFile <path>]
        roadmap.md から HTML を作る。-Open を付けると、書けたときだけ OS の既定のアプリで開く。
        -OutFile を付けると doc/ の日付つきの名前でなく、その場所へ書く（相対なら今のフォルダから）。
        ウェーブ編成の節の最初の ``` の塊（次のウェーブの開始コマンド）は、頁の上にコピーのボタンつきで出す（無ければ出さない）。
          FAIL strings: {理由}                   文言のファイルが読めない・鍵が欠けるか空
          FAIL template: {理由}                  ひな形が読めない・約束を破る
          FAIL missing roadmap                   .kiro/steering/roadmap.md が読めない
          FAIL missing {鍵}                      読めない所（欠けごとに 1 行・何も書かない）
          WARN {鍵} ...                          読めたが食い違う所（警告ごとに 1 行・書く）
          WARN git: blob unknown                 中身の印（git hash-object の先頭 12 字）を求められない
          FAIL write {path}: {理由}              HTML を書けない
          OK wrote {path} ({帯} band(s), {spec} spec(s), {警告} warning(s))
        終了コード: 0 書けた（警告が在っても 0）／1 FAIL の行を出した
    -SelfTest
        自分の検査。判定を 1 つずつ「ok selftest {名前}」か「FAIL selftest {名前}」で印字する。
        判定の番号は design の SelfTest の番号。
        見本はこのスクリプトの隣の roadmap-overview-fixture/（roadmap.md＝見本の断片・expected-data.json＝期待の値）。
          1 strings  文言のファイルが読めて、markers と labels の鍵がそろい、どれも空でない
                     （鍵を 1 つ外した写しと、1 つを空にした写しが落ちることも確かめる）
          2 template ひな形（TEMPLATE_FILE）が約束を守る: 差し込み口がちょうど 1 つ・CHARSET を持つ・
                     EXTERNAL に当たる書き方が無い・ASCII だけ（1 か所ずつ崩した写しがその約束だけで落ちることも確かめる）
          3 fixture data equals the expected data
                     見本から作ったデータの JSON（字下げつき・LF・中身の印は 0123456789ab）が期待の値と 1 字違わず同じ
          4 fixture has no missing and no warning
          5 fixture broken for {鍵} warns with {鍵} only
                     見本を 1 か所ずつ壊した文（band-count・not-in-ledger・not-in-band・duplicate・table-columns）が
                     その警告の鍵だけを出す。壊した文が見本と同じなら落とす
          6 fixture without {所} of {鍵} is missing {鍵} only
                     前置きの updated_at と 5 つの節（themes 以外は「節は在るが表が無い」形も）を 1 つずつ外した文が
                     その欠けの鍵だけを出し、データを作らない（外し方が 10 でなければ「fixture removals cover every missing key」が落ちる）
          7 fixture html is the same twice and escapes the script end tag
                     見本から 2 回作った HTML が 1 字違わず同じで、台帳の文の </script> が JSON の逃がしになっている

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
    FRONT_MATTER  先頭の前置きの区切りの行
    UPDATED_AT    前置きの更新日の鍵（値は YYYY-MM-DD。無ければ欠け updated-at）
    TEMPLATE_FILE HTML・CSS・JS のひな形（このスクリプトと同じフォルダ）
    PLACEHOLDER   ひな形のデータの差し込み口
    CHARSET       ひな形が持つ文字コードの宣言
    EXTERNAL      ひな形に在ってはならない、外の物を読む書き方に当てる正規表現

.PARAMETER Open
  書けたときだけ、作った HTML を OS の既定のアプリ（ブラウザ）で開く。

.PARAMETER OutFile
  書く先のファイル。省くと doc/roadmap-overview-{updated_at}.html。/kiro-next は棚卸の最後に
  scratch の roadmap-overview-latest.html へ書いて、セッションで公開する。

.PARAMETER SelfTest
  自分の検査だけを回す。何も書かない。

.EXAMPLE
  pwsh -NoProfile -File .claude/skills/kiro-next/overview/roadmap-overview.ps1 -Open
  pwsh -NoProfile -File .claude/skills/kiro-next/overview/roadmap-overview.ps1 -OutFile C:\some\folder\roadmap-overview-latest.html
  pwsh -NoProfile -File .claude/skills/kiro-next/overview/roadmap-overview.ps1 -SelfTest
#>
#Requires -Version 7
# 知らない引数（綴り違いを含む）は黙って捨てずに断る
[CmdletBinding(DefaultParameterSetName = 'Build')]
param(
    [Parameter(ParameterSetName = 'Build')][switch]$Open,
    [Parameter(ParameterSetName = 'Build')][string]$OutFile,
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
                'colWave', 'colDate', 'colUnits', 'colSummary', 'completedExtra', 'what', 'note',
                'waveCommands', 'copy', 'copied'
$SECTION_PREFIX = '## '
$TABLE_RULE     = '^\|\s*:?-'
$ONE_LINER_MAX  = 60
$SPEC_PREFIX    = 'areka-P0-'
$SPEC_NAME      = '^[a-z0-9]+(-[a-z0-9]+)+$'
$THEME_SPECS    = '#### Specs (dependency order)'
$THEME_ROW_SEP  = ' -- '
$THEME_NAME_SEP = ', '
$THEME_DEPS     = 'Dependencies:'
$FRONT_MATTER   = '---'
$UPDATED_AT     = 'updated_at'
$TEMPLATE_FILE  = 'roadmap-overview.template.html'
$PLACEHOLDER    = '@@ROADMAP_DATA@@'
$CHARSET        = '<meta charset="utf-8">'
$EXTERNAL       = '(?i)<script\b[^>]*\ssrc\s*=|<link\b|@import|url\(|https?://'

# リポジトリの根＝このスクリプトから上へ辿って最初に .kiro の在るフォルダ（スキルのフォルダの中に置くので、深さに依らない）。
# 無ければスクリプトのフォルダ（roadmap.md が読めずに FAIL missing roadmap になる）
$root = $PSScriptRoot
for ($d = $PSScriptRoot; $d; $d = Split-Path $d -Parent) {
    if ([System.IO.Directory]::Exists((Join-Path $d '.kiro'))) { $root = $d; break }
}

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
    if ($null -eq $text) { throw "cannot read $STRINGS_FILE" }
    try { $s = $text | ConvertFrom-Json } catch { throw "$STRINGS_FILE is not valid JSON" }
    $bad = @(Test-Strings $s)
    if ($bad.Count) { throw "missing or empty key(s) in ${STRINGS_FILE}: $($bad -join ', ')" }
    $s
}

# ひな形の約束の破れを鍵（placeholder・charset・external・ascii）で全部返す（空なら通った）。
# 差し込み口がちょうど 1 つ・文字コードの宣言が在る・外の物を読む書き方が無い・ASCII だけ
function Test-Template([string]$Template) {
    if (([regex]::Matches($Template, [regex]::Escape($PLACEHOLDER))).Count -ne 1) { 'placeholder' }
    if (-not $Template.Contains($CHARSET)) { 'charset' }
    if ($Template -match $EXTERNAL) { 'external' }
    if ($Template -match '[^\x00-\x7F]') { 'ascii' }
}

# 文言とデータを HTML の字を逃がす JSON にし、ひな形のただ 1 つの差し込み口を文字列の置き換え（正規表現でない）で替える。
# 改行は LF。時刻・乱数・環境の値は入れない
function Format-Html([string]$Template, [pscustomobject]$Data, [pscustomobject]$Labels) {
    if (([regex]::Matches($Template, [regex]::Escape($PLACEHOLDER))).Count -ne 1) { throw 'template must have exactly one placeholder' }
    $json = [ordered]@{ labels = $Labels; data = $Data } | ConvertTo-Json -Depth 10 -Compress -EscapeHandling EscapeHtml
    $Template.Replace("`r`n", "`n").Replace($PLACEHOLDER, $json)
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
#   source（一言の元）; prereqs（前提の文の深さ 0 の spec 名・頭の SPEC_PREFIX は外す） } }
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
    # 未着手の spec は 2026-10-23 から接頭辞なし（kiro-spec-schema.md の命名規則）なので、頭の SPEC_PREFIX は有っても無くてもよい
    $prefixed = '(?<![A-Za-z0-9-])(?:' + [regex]::Escape($SPEC_PREFIX) + ')?([a-z0-9]+(?:-[a-z0-9]+)+)(?![A-Za-z0-9-])'
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

# ウェーブ編成の節の最初のコードの塊（``` で囲んだ行・/kiro-next が書く開始コマンドの一覧）を、囲みを外して返す。
# 節か塊が無ければ $null（任意の欄＝欠けにも警告にもしない）
function Get-WaveCommands([string[]]$Lines, [pscustomobject]$Markers) {
    $i = 0
    while ($i -lt $Lines.Count -and -not $Lines[$i].StartsWith($SECTION_PREFIX + $Markers.secWaves)) { $i++ }
    for ($i++; $i -lt $Lines.Count -and -not $Lines[$i].StartsWith($SECTION_PREFIX); $i++) {
        if (-not $Lines[$i].StartsWith('```')) { continue }
        $body = [System.Collections.Generic.List[string]]::new()
        for ($i++; $i -lt $Lines.Count -and -not $Lines[$i].StartsWith('```'); $i++) { $body.Add($Lines[$i]) }
        return ($body -join "`n")
    }
    $null
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

# roadmap.md の全文（LF）から、HTML に埋めるデータ・警告・欠けを作る。ファイルにも git にも触らず、時刻・乱数・環境を読まない。
# 更新日は前置きの UPDATED_AT から読み、$Source からは blob（中身の印・無ければ $null）だけを使う。
# 返り値 @{ Missing = 欠けの鍵（在れば Data は $null）; Warnings = @{ Key; Detail }; Data = データの形（design）}。
# 並びはすべて roadmap.md に出る順で、1 要素でも配列として持つ
function Read-Roadmap([string]$Text, [pscustomobject]$Markers, [pscustomobject]$Source) {
    $lines = [string[]]$Text.Split("`n")
    $missing = [System.Collections.Generic.List[string]]::new()
    $updatedAt = $null
    if ($lines.Count -and $lines[0] -ceq $FRONT_MATTER) {
        for ($i = 1; $i -lt $lines.Count -and $lines[$i] -cne $FRONT_MATTER; $i++) {
            if ($lines[$i] -cmatch "^$UPDATED_AT\s*:\s*(\d{4}-\d{2}-\d{2})\s*$") { $updatedAt = $Matches[1]; break }
        }
    }
    if ($null -eq $updatedAt) { $missing.Add('updated-at') }
    $bands = Get-Bands $lines $Markers;    if ($null -eq $bands) { $missing.Add('version-table') }
    $ledger = Get-Ledger $lines $Markers;  if ($null -eq $ledger) { $missing.Add('ledger-table') }
    $themes = Get-Themes $lines $Markers;  if ($null -eq $themes) { $missing.Add('themes') }
    $wave = Get-Wave $lines $Markers;      if ($null -eq $wave) { $missing.Add('wave-table') }
    $summary = Get-Summary $lines $Markers; if ($null -eq $summary) { $missing.Add('summary-table') }
    $warnings = [System.Collections.Generic.List[object]]::new()
    foreach ($r in @($bands, $ledger, $wave, $summary) | Where-Object { $_ }) { foreach ($w in $r.Warnings) { $warnings.Add($w) } }
    if ($missing.Count) { return [pscustomobject]@{ Missing = [string[]]$missing; Warnings = [object[]]$warnings; Data = $null } }

    # 引き当て（並びは作らない）。台帳とテーマは最初に出た行を使う
    $ledgerBy = @{}
    foreach ($r in $ledger.Rows) { if (-not $ledgerBy.ContainsKey($r.name)) { $ledgerBy[$r.name] = $r } }
    $themeBy = @{}
    $done = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($r in $themes.Rows) {
        foreach ($n in $r.names) { if (-not $themeBy.ContainsKey($n)) { $themeBy[$n] = $r }; if ($r.done) { [void]$done.Add($n) } }
    }
    $inSummary = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($r in $summary.Rows) { foreach ($n in $r.names) { [void]$inSummary.Add($n); [void]$done.Add($n) } }
    $waveNames = [System.Collections.Generic.HashSet[string]]::new([string[]]$wave.Names)
    $noTheme = @($themes.Themes).Count

    function New-Card([string]$Name, [bool]$Chained, $Note) {
        $lr = $ledgerBy[$Name]; $tr = $themeBy[$Name]
        [pscustomobject][ordered]@{
            name = $Name
            oneLiner = $null -ne $tr ? (Get-OneLiner $tr.source $Markers) : ($null -ne $lr ? (Get-OneLiner $lr.what $Markers) : '')
            theme = $null -ne $tr ? $tr.theme : $noTheme
            tier = $null -ne $lr ? $lr.tier : $null; cls = $null -ne $lr ? $lr.cls : $null; size = $null -ne $lr ? $lr.size : $null
            chained = $Chained; note = $Note
            wave = $waveNames.Contains($Name) ? $wave.Name : $null
            done = $done.Contains($Name); notInLedger = $null -eq $lr
            otherBands = [System.Collections.Generic.List[string]]::new()
            prereqs = [object[]]@()
            ledger = $null -ne $lr ? [pscustomobject][ordered]@{ what = $lr.what; order = $lr.order; prereq = $lr.prereq } : $null
        }
    }

    # 帯に置く。2 つ目以降に出た名前はカードにせず、先のカードの otherBands に帯の名前を足す
    $placed = [ordered]@{}   # 名前 -> @{ Band = 置いた帯の名前; Card }
    $seen = [ordered]@{}     # 名前 -> 出た帯の番号（1 から）
    $notInLedger = [System.Collections.Generic.List[string]]::new()
    $outBands = [System.Collections.Generic.List[object]]::new()
    for ($bi = 0; $bi -lt $bands.Bands.Count; $bi++) {
        $b = $bands.Bands[$bi]
        $groups = foreach ($g in $b.groups) {
            $cards = foreach ($k in $g.cards) {
                if (-not $seen.Contains($k.name)) { $seen[$k.name] = [System.Collections.Generic.List[int]]::new() }
                $seen[$k.name].Add($bi + 1)
                if ($placed.Contains($k.name)) {
                    $p = $placed[$k.name]
                    if ($p.Band -cne $b.name -and -not $p.Card.otherBands.Contains($b.name)) { $p.Card.otherBands.Add($b.name) }
                    continue
                }
                $card = New-Card $k.name $k.chained $k.note
                $placed[$k.name] = @{ Band = $b.name; Card = $card }
                if ($card.notInLedger) { $notInLedger.Add($k.name) }
                $card
            }
            [pscustomobject][ordered]@{ heading = $g.heading; cards = [object[]]@($cards) }
        }
        $outBands.Add([pscustomobject][ordered]@{
            name = $b.name; goal = $b.goal; declared = $b.declared; read = $b.read; notInBand = $false; groups = [object[]]@($groups)
        })
    }
    foreach ($n in $seen.Keys) {
        if ($seen[$n].Count -gt 1) { $warnings.Add([pscustomobject]@{ Key = 'duplicate'; Detail = "${n}: bands $($seen[$n] -join ',')" }) }
    }
    foreach ($n in $notInLedger) { $warnings.Add([pscustomobject]@{ Key = 'not-in-ledger'; Detail = $n }) }

    # 台帳に在ってどの帯にも無い spec は、末尾の帯（名前と目標は空・小見出し無しの組 1 つ・台帳の順）へ
    $rest = foreach ($r in $ledger.Rows) {
        if ($placed.Contains($r.name)) { continue }
        $card = New-Card $r.name $false $null
        $placed[$r.name] = @{ Band = ''; Card = $card }
        $warnings.Add([pscustomobject]@{ Key = 'not-in-band'; Detail = $r.name })
        $card
    }
    $rest = [object[]]@($rest)
    if ($rest.Count) {
        $outBands.Add([pscustomobject][ordered]@{
            name = ''; goal = ''; declared = $null; read = $rest.Count; notInBand = $true
            groups = [object[]]@([pscustomobject][ordered]@{ heading = $null; cards = $rest })
        })
    }

    # 前提＝台帳の前提 → Specs の前提の順に重なりを外す。居場所は置いた帯の名前と完了か（どちらも無い＝roadmap に無い）
    foreach ($p in $placed.Values) {
        $card = $p.Card
        $names = [System.Collections.Generic.List[string]]::new()
        $lr = $ledgerBy[$card.name]; $tr = $themeBy[$card.name]
        foreach ($n in @(@(if ($null -ne $lr) { $lr.prereqs }) + @(if ($null -ne $tr) { $tr.prereqs }))) {
            if ($n -and -not $names.Contains($n)) { $names.Add($n) }
        }
        $card.prereqs = [object[]]@(foreach ($n in $names) {
            [pscustomobject][ordered]@{ name = $n; band = $placed.Contains($n) ? $placed[$n].Band : $null; done = $done.Contains($n) }
        })
        $card.otherBands = [string[]]@($card.otherBands)
    }

    $extra = [System.Collections.Generic.List[object]]::new()
    $extraNames = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($r in $themes.Rows) {
        if (-not $r.done) { continue }
        foreach ($n in $r.names) {
            if ($placed.Contains($n) -or $inSummary.Contains($n) -or -not $extraNames.Add($n)) { continue }
            $tr = $themeBy[$n]
            $extra.Add([pscustomobject][ordered]@{ name = $n; oneLiner = Get-OneLiner $tr.source $Markers; theme = $tr.theme })
        }
    }
    $rows = foreach ($r in $summary.Rows) {
        [pscustomobject][ordered]@{
            wave = $r.wave; date = $r.date; units = $r.units; summary = $r.summary; oneLiner = Get-OneLiner $r.summary $Markers
        }
    }

    $data = [pscustomobject][ordered]@{
        source = [pscustomobject][ordered]@{ updatedAt = $updatedAt; blob = $Source.blob }
        themes = [string[]]@($themes.Themes)
        bands = [object[]]$outBands
        waveCommands = Get-WaveCommands $lines $Markers
        completed = [pscustomobject][ordered]@{ rows = [object[]]@($rows); extra = [object[]]$extra }
        warningCount = $warnings.Count
    }
    [pscustomobject]@{ Missing = [string[]]@(); Warnings = [object[]]$warnings; Data = $data }
}

if ($SelfTest) {
    $script:selfFails = 0
    function Judge([string]$Name, [bool]$Ok) {
        if ($Ok) { "ok selftest $Name" } else { $script:selfFails++; "FAIL selftest $Name" }
    }
    function Try-Judge([string]$Name, [scriptblock]$Body) { $ok = $false; try { $ok = [bool](& $Body) } catch { }; Judge $Name $ok }

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

    # ---- 2. ひな形の約束（差し込み口が 1 つ・文字コードの宣言・外の物を読む書き方が無い・ASCII だけ） ----
    # 1 か所ずつ崩した写しが、その約束の鍵だけで落ちることも確かめる（判定の空振りを防ぐ）
    $template = Read-Lf (Join-Path $PSScriptRoot $TEMPLATE_FILE)
    Try-Judge 'template' {
        $null -ne $template -and @(Test-Template $template).Count -eq 0 -and
        (@(Test-Template ($template + $PLACEHOLDER)) -join ',') -ceq 'placeholder' -and
        (@(Test-Template $template.Replace($PLACEHOLDER, '')) -join ',') -ceq 'placeholder' -and
        (@(Test-Template $template.Replace($CHARSET, '')) -join ',') -ceq 'charset' -and
        (@(Test-Template ($template + '<link rel="x">')) -join ',') -ceq 'external' -and
        (@(Test-Template ($template + '<script src="x.js"></script>')) -join ',') -ceq 'external' -and
        (@(Test-Template ($template + "/* $([char]0x3042) */")) -join ',') -ceq 'ascii'
    }

    # ---- 3〜6. 見本の読み取り（見本はこのスクリプトの隣の roadmap-overview-fixture/。中身の印は固定の値） ----
    $fxDir = Join-Path $PSScriptRoot 'roadmap-overview-fixture'
    $fixture = Read-Lf (Join-Path $fxDir 'roadmap.md')
    $expected = Read-Lf (Join-Path $fxDir 'expected-data.json')
    $m = ${strings}?.markers
    $src = [pscustomobject]@{ blob = '0123456789ab' }
    function Read-Fixture([string]$Text) { Read-Roadmap $Text $m $src }
    $base = $null
    if ($null -ne $m -and $null -ne $fixture) { try { $base = Read-Fixture $fixture } catch { } }

    # 3. 見本から作ったデータの JSON（字下げつき・LF）が期待の値と 1 字違わず同じ
    Try-Judge 'fixture data equals the expected data' {
        $null -ne $expected -and (($base.Data | ConvertTo-Json -Depth 10).Replace("`r`n", "`n") + "`n") -ceq $expected
    }
    # 4. 見本は欠け 0・警告 0
    Try-Judge 'fixture has no missing and no warning' {
        $null -ne $base.Data -and @($base.Missing).Count -eq 0 -and @($base.Warnings).Count -eq 0 -and $base.Data.warningCount -eq 0
    }

    # 5. 見本を 1 か所ずつ壊した文が、決まった警告の鍵だけを出す（壊した文が見本と同じなら空振りなので落とす）。
    # 壊す所は「いつか」の帯の「`omega-last`＝**1 本**」と、台帳の omega-last の行（行の頭で当てる＝ウェーブの表の名前に当てない）
    $broken = [ordered]@{}
    if ($null -ne $m -and $null -ne $fixture) {
        $l, $eq = $m.listSep, $m.headSep
        $last = "``omega-last``$eq**1 "
        $row = "`n| ``omega-last`` |"
        $broken['band-count']    = $fixture.Replace($last, "``omega-last``$eq**2 ")
        $broken['not-in-ledger'] = $fixture.Replace($last, "``omega-last``$l ``zz-new``$eq**2 ")
        $broken['not-in-band']   = $fixture.Replace($row, "`n| ``zz-led`` | t | w | z | o | - | - |$row")
        $broken['duplicate']     = $fixture.Replace($last, "``omega-last``$l ``alpha-base``$eq**2 ")
        $broken['table-columns'] = $fixture.Replace($row, "$row x |")
    }
    foreach ($k in 'band-count', 'not-in-ledger', 'not-in-band', 'duplicate', 'table-columns') {
        Try-Judge "fixture broken for $k warns with $k only" {
            $t = $broken[$k]
            $r = Read-Fixture $t
            $t -cne $fixture -and @($r.Missing).Count -eq 0 -and $r.Data.warningCount -eq @($r.Warnings).Count -and
            (@($r.Warnings | ForEach-Object Key) -join ',') -ceq $k
        }
    }

    # 6. 見本から 1 か所ずつ外した文が、決まった欠けの鍵だけを出し、Data が無い。
    # 節の見出しの行を別の名前に替える（節が無い）か、見出しの直後に別の節を挟む（節は在るが表が無い）。
    # themes は表を持たない節なので「節が無い」形だけ。外し方は合わせて 10（数が違えば判定は空振りなので落とす）
    function Edit-Heading([string]$Marker, [scriptblock]$To) {
        $lines = $fixture.Split("`n")
        for ($i = 0; $i -lt $lines.Count; $i++) {
            if ($lines[$i].StartsWith($SECTION_PREFIX + $Marker)) { $lines[$i] = & $To $lines[$i]; break }
        }
        $lines -join "`n"
    }
    $cut = [System.Collections.Generic.List[object]]::new()
    if ($null -ne $m -and $null -ne $fixture) {
        $cut.Add(@('updated-at', 'the updated_at line', ([regex]"(?m)^$UPDATED_AT\s*:.*\n").Replace($fixture, '', 1)))
        $places = [ordered]@{ 'version-table' = $m.secVersions; 'ledger-table' = $m.secLedger; 'themes' = $m.secThemes
                              'wave-table' = $m.secWaves; 'summary-table' = $m.secSummary }
        foreach ($k in $places.Keys) {
            $cut.Add(@($k, 'the section', (Edit-Heading $places[$k] { "${SECTION_PREFIX}gone" })))
            if ($k -cne 'themes') { $cut.Add(@($k, 'the table', (Edit-Heading $places[$k] { param($h) "$h`n`n${SECTION_PREFIX}gone" }))) }
        }
    }
    Judge 'fixture removals cover every missing key' ($cut.Count -eq 10)
    foreach ($c in $cut) {
        Try-Judge "fixture without $($c[1]) of $($c[0]) is missing $($c[0]) only" {
            $r = Read-Fixture $c[2]
            $c[2] -cne $fixture -and (@($r.Missing) -join ',') -ceq $c[0] -and $null -eq $r.Data
        }
    }

    # 7. 見本から 2 回作った HTML（labels は文言のファイルのもの）が 1 字違わず同じで、
    # 台帳の文の </script> が \u003c/script\u003e に逃がされている（HTML の </script> の数はひな形と同じ）
    Try-Judge 'fixture html is the same twice and escapes the script end tag' {
        $h1 = Format-Html $template (Read-Fixture $fixture).Data $strings.labels
        $h2 = Format-Html $template (Read-Fixture $fixture).Data $strings.labels
        $escEnd = '\' + 'u003c/script' + '\' + 'u003e'
        $ends = { param($t) ([regex]::Matches($t, '</script>')).Count }
        $h1 -ceq $h2 -and $h1.Contains($escEnd) -and -not $h1.Contains($PLACEHOLDER) -and
        (& $ends $h1) -eq (& $ends $template) -and -not $h1.Contains("`r")
    }

    if ($script:selfFails) { "selftest: $($script:selfFails) failure(s)"; exit 1 }
    'selftest: all passed'
    exit 0
}

# ---- 作る呼び方 ----
try { $strings = Read-Strings }
catch { "FAIL strings: $($_.Exception.Message -replace '[^\x20-\x7E]', '?')"; exit 1 }
function ConvertTo-Ascii([string]$Text) { $Text -replace '[^\x20-\x7E]', '?' }

$template = Read-Lf (Join-Path $PSScriptRoot $TEMPLATE_FILE)
if ($null -eq $template) { "FAIL template: cannot read $TEMPLATE_FILE"; exit 1 }
$bad = @(Test-Template $template)
if ($bad.Count) { "FAIL template: $TEMPLATE_FILE breaks: $($bad -join ', ')"; exit 1 }

# 読むのは根の roadmap.md だけ（今のフォルダによらない）
$roadmapRel = '.kiro/steering/roadmap.md'
$text = Read-Lf (Join-Path $root $roadmapRel)
if ($null -eq $text) { 'FAIL missing roadmap'; exit 1 }

# 中身の印＝git hash-object の先頭 12 字（ファイルの中身だけで決まる。git log --find-object で辿れる）
$blob = $null
try {
    # 出力は 16 進の ASCII だけなので、端末の文字コードに依らず同じに読める（リポジトリの道具の関数に頼らない）
    $g = (& git -C $root hash-object -- $roadmapRel 2>$null) -join ''
    if ($LASTEXITCODE -eq 0 -and $g.Trim() -cmatch '^[0-9a-f]{40,64}$') { $blob = $g.Trim().Substring(0, 12) }
} catch { }

$r = Read-Roadmap $text $strings.markers ([pscustomobject]@{ blob = $blob })
if (@($r.Missing).Count) { foreach ($k in $r.Missing) { "FAIL missing $k" }; exit 1 }

$warnLines = @(foreach ($w in $r.Warnings) { ConvertTo-Ascii "WARN $($w.Key) $($w.Detail)" })
if ($null -eq $blob) { $warnLines += 'WARN git: blob unknown'; $r.Data.warningCount++ }
$warnLines

if ($OutFile) {
    $outPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutFile)
    $outRel = ConvertTo-Ascii $outPath
} else {
    $outRel = "doc/roadmap-overview-$($r.Data.source.updatedAt).html"
    $outPath = Join-Path $root $outRel
}
try { Write-Utf8 $outPath (Format-Html $template $r.Data $strings.labels) }
catch { "FAIL write ${outRel}: $(ConvertTo-Ascii $_.Exception.GetBaseException().Message)"; exit 1 }

# 帯の数は版の段の表の行の数、spec の数は置いたカードの数（「帯に無い」の帯を含む）
$bandN = @($r.Data.bands | Where-Object { -not $_.notInBand }).Count
$specN = 0
foreach ($b in $r.Data.bands) { foreach ($gr in $b.groups) { $specN += @($gr.cards).Count } }
"OK wrote $outRel ($bandN band(s), $specN spec(s), $($r.Data.warningCount) warning(s))"
if ($Open) { Invoke-Item -LiteralPath $outPath }
exit 0
