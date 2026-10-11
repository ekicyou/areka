<#
.SYNOPSIS
  リリースの見どころのファイル（doc/release-notes/v{版}.md）の形を検査し、詳しいリリースノートを作る。

.DESCRIPTION
  -Check と -SelfTest は読むだけで、何も書かない。-Notes が書くのは -Out のファイル 1 つだけ。
  画面へ出す文は ASCII だけで、見どころの行の中身も題名も印字しない。
  端末の文字コードは読みも書きもしない。今のフォルダを変えない。呼ばれた場所によらず、このスクリプトの在るリポジトリを見る。
  見どころの書き方の決まりは doc/release-notes/README.md に在る。

    -Check -Version {版}
        その版の名前の見どころのファイルだけを読み、欠けを全部集めて 1 行ずつ印字する。
          FAIL {欠けの名前} line {行}: {説明}      行 0 はファイルの全体を指す
          OK highlights v{版} ({見どころの数} item(s))
    -Notes -Version {版} -Repository {owner/name} [-PrevTag {タグ}] [-To {先}] -Out {ファイル}
        本線（git log --first-parent）の {PrevTag}..{To} の題名から、詳しいリリースノートを -Out に書く
        （LF・BOM なしの UTF-8。日付や時刻は書かないので、同じ入力からは同じバイトになる）。
        -PrevTag が空なら最初のコミットから。-To の既定は HEAD。-Out の相対パスは、呼んだ側の今のフォルダから数える。
          OK release notes v{版} ({行の数} change(s))
          FAIL git log (exit {コード}): {git の標準エラーの最後の行}
          FAIL strings: ...                        定型文が読めない
        1 つの題名の扱い（上から順に）:
          行末の「 (#番号)」を外し、PR へのリンクにする（無ければ番号なしで載せる）
          「種類(スコープ): 文」の形で、スコープが spec 名の印で始まれば製品の変更。
            種類が feat＝新しい機能／fix＝不具合の修正／perf＝性能の改善／ほか＝そのほかの変更。載せるのは「文」
          形は合うがスコープが spec 名でない（スコープが無いものを含む）＝内部の作業。載せるのは「文」
          形が合わない＝内部の作業。載せるのは題名の全体
          載せる文から spec 名の印をすべて外す（外した跡にまた現れたら、無くなるまで外す）
        見出しの中は git log の順（新しい順）。0 件の見出しは出さない。
    -SelfTest
        自分の検査。判定を 1 つずつ「ok selftest {名前}」か「FAIL selftest {名前}」で印字する。
          定型文のファイルが読めて、鍵がそろっている
          見本の題名（tools/release-notes-fixture/subjects.txt）から作った文が expected-notes.md と 1 字違わず同じ
          見本から fix と perf の製品の変更を除き、一つ前のタグを空にして作った文が expected-notes-from-start.md と同じ
          見本（tools/release-notes-fixture/highlights-ok.md）が通る
          見本から 1 か所ずつ外した文が、それぞれ決まった欠けの名前だけで落ちる
          見出しの番号の後ろに字を続けた文が落ちる／大文字と小文字の違う spec 名の印は通る／欠けを重ねた文が全部を返す
          書き方の決まりの文書が、較正値の出典の URL を含む
          doc/release-notes/v*.md の実物が全部通る

  欠けの名前:
    missing-file           その版の見どころのファイルが無い
    no-highlight           「### 」で始まる行が 0
    heading-no-pr          「### 」の行の末尾が (#番号) か (#番号, #番号) でない
    spec-name              spec 名の印が現れる
    no-source-link         出典のリンクが無い
    first-name-not-linked  出典のリンクより前に「Claudia」が在る

  終了コード: 0 通った／1 欠けが在る（-Notes は git の失敗か定型文が読めない・-SelfTest は落ちた判定が在る）

  較正値（変更はスクリプト冒頭の「較正値」の 1 か所だけ）:
    HIGHLIGHTS_DIR  見どころのフォルダ
    SOURCE_URL      出典の URL（doc/release-notes/README.md と同じ値）
    SOURCE_LINK     出典のリンク（最初の「Claudia」の書き方）
    SPEC_MARK       spec 名の印
    STRINGS_FILE    日本語の定型文のファイル（このスクリプトと同じフォルダ）
    NOTES_KEYS      定型文の鍵。{version}＝版・{prev}＝一つ前のタグ・{count}＝件数 に置き換わるのは題と範囲の文だけ

.PARAMETER Version
  v を付けない版（例 0.0.3）。

.PARAMETER Repository
  owner/name（例 ekicyou/areka）。PR へのリンクの URL を作る。

.PARAMETER PrevTag
  一つ前のタグ（例 v0.0.2）。空なら最初のコミットから。

.PARAMETER To
  一覧の先（タグの名前かコミット）。既定は HEAD。

.PARAMETER Out
  書き出す先のファイル。親のフォルダが無ければ作る。

.EXAMPLE
  pwsh -NoProfile -File tools/release-notes.ps1 -Check -Version 0.0.3
  pwsh -NoProfile -File tools/release-notes.ps1 -Notes -Version 0.0.3 -Repository ekicyou/areka -PrevTag v0.0.2 -Out target/release-notes/release-notes-v0.0.3.md
  pwsh -NoProfile -File tools/release-notes.ps1 -SelfTest
#>
#Requires -Version 7
# 知らない引数（綴り違いを含む）は黙って捨てずに断る
[CmdletBinding(DefaultParameterSetName = 'Check')]
param(
    [Parameter(ParameterSetName = 'Check', Mandatory)][switch]$Check,
    [Parameter(ParameterSetName = 'Notes', Mandatory)][switch]$Notes,
    [Parameter(ParameterSetName = 'SelfTest', Mandatory)][switch]$SelfTest,

    # ファイルの名前と印字に使うので、版に現れる ASCII の字だけを通す
    [Parameter(ParameterSetName = 'Check', Mandatory)]
    [Parameter(ParameterSetName = 'Notes', Mandatory)]
    [ValidatePattern('^[0-9A-Za-z.+-]+$')][string]$Version,

    [Parameter(ParameterSetName = 'Notes', Mandatory)][string]$Repository,
    [Parameter(ParameterSetName = 'Notes')][string]$PrevTag = '',
    [Parameter(ParameterSetName = 'Notes')][string]$To = 'HEAD',
    [Parameter(ParameterSetName = 'Notes', Mandatory)][string]$Out
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# =============================================================================
# 較正値（説明の一覧と対応。変更はここだけ）
# =============================================================================
$HIGHLIGHTS_DIR = 'doc/release-notes'
$SOURCE_URL     = 'https://github.com/ponapalt/claudia'
$SOURCE_LINK    = "[Claudia]($SOURCE_URL)"
$SPEC_MARK      = 'areka-P0-'
$STRINGS_FILE   = 'release-notes.strings.json'
$NOTES_KEYS     = 'notesTitle', 'notesRange', 'notesRangeFromStart', 'headFeat', 'headFix', 'headPerf', 'headOther', 'headInternal'

$root = Split-Path $PSScriptRoot -Parent

# 子の出力を端末の文字コードに依らず UTF-8 で読む関数 Invoke-Utf8Child
. (Join-Path $PSScriptRoot 'utf8-child.ps1')

# ファイルを UTF-8 で読み、改行を LF にそろえて返す。無ければ $null
function Read-Lf([string]$Path) {
    if (-not [System.IO.File]::Exists($Path)) { return $null }
    [System.IO.File]::ReadAllText($Path, [System.Text.UTF8Encoding]::new($false)).Replace("`r`n", "`n")
}

# 見どころの文（改行は LF）の欠けを全部返す（空なら通った）。$Text が $null ならファイルが無い扱い。
# 引数に [string] を付けない: 付けると PowerShell が $null を空の文字列に変え、無いファイルと空のファイルを分けられない
function Test-Highlights([AllowNull()]$Text) {
    $bad = { param($key, $line, $detail) [pscustomobject]@{ Key = $key; Line = $line; Detail = $detail } }
    if ($null -eq $Text) { return & $bad 'missing-file' 0 'the highlights file of this version does not exist' }

    $lines = $Text -split "`n"
    $heads = 0
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -cmatch '^### ') {
            $heads++
            if ($lines[$i].TrimEnd() -cnotmatch '\(#[0-9]+(, #[0-9]+)*\)$') {
                & $bad 'heading-no-pr' ($i + 1) 'the heading does not end with (#N) or (#N, #N)'
            }
        }
        if ($lines[$i].Contains($SPEC_MARK)) { & $bad 'spec-name' ($i + 1) "the line contains $SPEC_MARK" }
    }
    if ($heads -eq 0) { & $bad 'no-highlight' 0 'no line starts with "### "' }

    # 出典のリンクは「Claudia」を含むので、最初の「Claudia」がリンクより前に在るときだけが「リンクでない」
    $link = $Text.IndexOf($SOURCE_LINK, [StringComparison]::Ordinal)
    $first = $Text.IndexOf('Claudia', [StringComparison]::Ordinal)
    if ($link -lt 0) { & $bad 'no-source-link' 0 "no $SOURCE_LINK" }
    elseif ($first -lt $link) {
        & $bad 'first-name-not-linked' @($Text.Substring(0, $first) -split "`n").Count "the first Claudia comes before $SOURCE_LINK"
    }
}

# 定型文を UTF-8 で読んで返す。読めない・鍵が欠けるか空のときは throw
function Read-Strings {
    $s = Read-Lf (Join-Path $PSScriptRoot $STRINGS_FILE) | ConvertFrom-Json
    foreach ($k in $NOTES_KEYS) { if (-not $s.$k) { throw "strings: no $k" } }
    $s
}

# 題名の並び（新しい順）から詳しいリリースノートの全文（改行は LF）を返す。ファイルにも git にも触らない
function Format-ReleaseNotes([string[]]$Subjects, [string]$Version, [string]$PrevTag, [string]$Repository, [pscustomobject]$Strings) {
    $kind = @{ feat = 'headFeat'; fix = 'headFix'; perf = 'headPerf' }
    $sections = [ordered]@{ headFeat = @(); headFix = @(); headPerf = @(); headOther = @(); headInternal = @() }
    foreach ($subject in $Subjects) {
        $text = $subject; $pr = ''; $head = 'headInternal'
        if ($text -cmatch '^(?<text>.*) \(#(?<pr>[0-9]+)\)$') { $text = $Matches.text; $pr = $Matches.pr }
        if ($text -cmatch '^(?<type>[a-z]+)(\((?<scope>[^)]*)\))?!?: (?<rest>.+)$') {
            $text = $Matches.rest
            # スコープの無い題名では、$Matches に scope が無い
            if (([string]$Matches['scope']).StartsWith($SPEC_MARK, [StringComparison]::Ordinal)) {
                $head = $kind[$Matches.type] ?? 'headOther'
            }
        }
        # 外した跡に印がまた現れても残さない
        while ($text.Contains($SPEC_MARK)) { $text = $text.Replace($SPEC_MARK, '') }
        $line = "- $text"
        if ($pr) { $line += " ([#$pr](https://github.com/$Repository/pull/$pr))" }
        $sections[$head] += $line
    }
    # 置き換えるのは題と範囲の文だけ（題名の中の字は置き換えない）
    $fill = { param($t) $t.Replace('{version}', $Version).Replace('{prev}', $PrevTag).Replace('{count}', [string]$Subjects.Count) }
    $blocks = @(
        & $fill $Strings.notesTitle
        & $fill ($PrevTag ? $Strings.notesRange : $Strings.notesRangeFromStart)
        foreach ($h in $sections.Keys) {
            if ($sections[$h].Count) { "## $($Strings.$h)`n`n$($sections[$h] -join "`n")" }
        }
    )
    ($blocks -join "`n`n") + "`n"
}

if ($SelfTest) {
    $script:selfFails = 0
    function Judge([string]$Name, [bool]$Ok) {
        if ($Ok) { "ok selftest $Name" } else { $script:selfFails++; "FAIL selftest $Name" }
    }
    # 欠けの名前をコンマでつないだ並び（通れば空）
    $keys = { param($t) @(Test-Highlights $t | ForEach-Object Key) -join ',' }

    # ---- 見どころの検査 ----
    $ok = [string](Read-Lf (Join-Path $PSScriptRoot 'release-notes-fixture/highlights-ok.md'))
    Judge 'highlights fixture passes' ((& $keys $ok) -ceq '')
    # 見本から 1 か所ずつ外した文。名前＝出るはずの欠けの名前（それだけが出る）
    $broken = [ordered]@{
        'missing-file'          = $null
        'no-highlight'          = (($ok -split "`n") -cnotmatch '^### ') -join "`n"
        'heading-no-pr'         = ([regex]'\(#[0-9]+\)\n').Replace($ok, "`n", 1)
        'spec-name'             = $ok + "`n${SPEC_MARK}x`n"
        'no-source-link'        = $ok.Replace($SOURCE_LINK, '')
        'first-name-not-linked' = $ok.Replace($SOURCE_LINK, "Claudia $SOURCE_LINK")
    }
    foreach ($k in $broken.Keys) {
        # 外したつもりの文が見本と同じなら、判定は空振りなので落とす
        Judge "highlights broken fixture fails with $k" (($broken[$k] -cne $ok) -and ((& $keys $broken[$k]) -ceq $k))
    }
    # 見出しの番号の括弧は行の最後に在ること
    Judge 'highlights heading with text after the pr number fails' (
        (& $keys ([regex]'(\(#[0-9]+\))\n').Replace($ok, "`$1 x`n", 1)) -ceq 'heading-no-pr')
    # spec 名の印は大文字と小文字を区別する
    Judge 'highlights spec mark in another case passes' ((& $keys ($ok + "`n$($SPEC_MARK.ToUpperInvariant())x`n")) -ceq '')
    # 欠けは 1 つ目で止めず、全部を返す（番号の無い見出し 2 つと spec 名）
    Judge 'highlights with three defects lists all three' (
        (& $keys (($ok -creplace '\(#[0-9]+(, #[0-9]+)*\)\n', "`n") + "`n${SPEC_MARK}x`n")) -ceq 'heading-no-pr,heading-no-pr,spec-name')
    Judge 'rules document has the source url' ([string](Read-Lf (Join-Path $root "$HIGHLIGHTS_DIR/README.md"))).Contains($SOURCE_URL)
    $real = @(Get-ChildItem -LiteralPath (Join-Path $root $HIGHLIGHTS_DIR) -Filter 'v*.md' -File)
    "selftest real highlights: $($real.Count) file(s)"
    foreach ($f in $real) {
        Judge "real highlights $($f.Name -replace '[^\x20-\x7E]', '?') passes" ((& $keys (Read-Lf $f.FullName)) -ceq '')
    }

    # ---- 詳しいリリースノートの整形 ----
    $fx = { param($n) [string](Read-Lf (Join-Path $PSScriptRoot "release-notes-fixture/$n")) }
    $strings = $null
    try { $strings = Read-Strings } catch { }
    Judge 'notes strings file has every key' ($null -ne $strings)
    $subjects = @((& $fx 'subjects.txt') -split "`n" -cne '')
    Judge 'notes fixture has subjects' ($subjects.Count -gt 0)
    if ($null -ne $strings) {
        Judge 'notes of the fixture subjects equal the expected notes' (
            (Format-ReleaseNotes $subjects '1.2.3' 'v1.2.2' 'owner/name' $strings) -ceq (& $fx 'expected-notes.md'))
        # 一つ前のタグが空のときの範囲の文と、0 件の見出し（不具合の修正・性能の改善）を出さないこと
        $noFixPerf = @($subjects -cnotmatch "^(fix|perf)\($([regex]::Escape($SPEC_MARK))")
        Judge 'notes from the start without fix and perf equal the expected notes' (
            ($noFixPerf.Count -lt $subjects.Count) -and
            ((Format-ReleaseNotes $noFixPerf '1.2.3' '' 'owner/name' $strings) -ceq (& $fx 'expected-notes-from-start.md')))
    }

    if ($script:selfFails) { "selftest: $($script:selfFails) failure(s)"; exit 1 }
    'selftest: all passed'
    exit 0
}

if ($Notes) {
    try { $strings = Read-Strings }
    catch { "FAIL strings: cannot read tools/$STRINGS_FILE, or a key is missing or empty"; exit 1 }
    $range = $PrevTag ? "$PrevTag..$To" : $To
    $r = Invoke-Utf8Child git @('-C', $root, 'log', '--first-parent', '--format=%s', '--end-of-options', $range, '--')
    if ($r.Code) {
        "FAIL git log (exit $($r.Code)): $([string]($r.ErrLines | Select-Object -Last 1) -replace '[^\x20-\x7E]', '?')"
        exit 1
    }
    # 1 行 1 題名。最後の改行だけを外す（空の題名の行を落とさない）
    $log = $r.Out.Replace("`r`n", "`n") -replace '\n\z', ''
    $subjects = $log -ne '' ? @($log -split "`n") : @()
    $path = $PSCmdlet.GetUnresolvedProviderPathFromPSPath($Out)
    [void][System.IO.Directory]::CreateDirectory((Split-Path $path -Parent))
    [System.IO.File]::WriteAllText($path, (Format-ReleaseNotes $subjects $Version $PrevTag $Repository $strings), [System.Text.UTF8Encoding]::new($false))
    "OK release notes v$Version ($($subjects.Count) change(s))"
    exit 0
}

# ---- -Check ----
$text = Read-Lf (Join-Path $root "$HIGHLIGHTS_DIR/v$Version.md")
$found = @(Test-Highlights $text)
foreach ($f in $found) { "FAIL $($f.Key) line $($f.Line): $($f.Detail)" }
if ($found.Count) { exit 1 }
"OK highlights v$Version ($(@($text -split "`n" -cmatch '^### ').Count) item(s))"
exit 0
