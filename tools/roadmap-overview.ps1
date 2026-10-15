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
