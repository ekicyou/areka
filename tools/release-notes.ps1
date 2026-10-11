<#
.SYNOPSIS
  リリースの見どころのファイル（doc/release-notes/v{版}.md）の形を検査する。

.DESCRIPTION
  どの形も読むだけで、何も書き換えない。画面へ出す文は ASCII だけで、見どころの行の中身は印字しない。
  端末の文字コードは読みも書きもしない。呼ばれた場所によらず、このスクリプトの在るリポジトリを見る。
  見どころの書き方の決まりは doc/release-notes/README.md に在る。

    -Check -Version {版}
        その版の名前の見どころのファイルだけを読み、欠けを全部集めて 1 行ずつ印字する。
          FAIL {欠けの名前} line {行}: {説明}      行 0 はファイルの全体を指す
          OK highlights v{版} ({見どころの数} item(s))
    -SelfTest
        自分の検査。判定を 1 つずつ「ok selftest {名前}」か「FAIL selftest {名前}」で印字する。
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

  終了コード: 0 通った／1 欠けが在る（-SelfTest は、落ちた判定が在る）

  較正値（変更はスクリプト冒頭の「較正値」の 1 か所だけ）:
    HIGHLIGHTS_DIR  見どころのフォルダ
    SOURCE_URL      出典の URL（doc/release-notes/README.md と同じ値）
    SOURCE_LINK     出典のリンク（最初の「Claudia」の書き方）
    SPEC_MARK       spec 名の印

.PARAMETER Version
  v を付けない版（例 0.0.3）。

.EXAMPLE
  pwsh -NoProfile -File tools/release-notes.ps1 -Check -Version 0.0.3
  pwsh -NoProfile -File tools/release-notes.ps1 -SelfTest
#>
#Requires -Version 7
# 知らない引数（綴り違いを含む）は黙って捨てずに断る
[CmdletBinding(DefaultParameterSetName = 'Check')]
param(
    [Parameter(ParameterSetName = 'Check', Mandatory)][switch]$Check,
    [Parameter(ParameterSetName = 'SelfTest', Mandatory)][switch]$SelfTest,

    # ファイルの名前と印字に使うので、版に現れる ASCII の字だけを通す
    [Parameter(ParameterSetName = 'Check', Mandatory)][ValidatePattern('^[0-9A-Za-z.+-]+$')][string]$Version
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

$root = Split-Path $PSScriptRoot -Parent

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

    if ($script:selfFails) { "selftest: $($script:selfFails) failure(s)"; exit 1 }
    'selftest: all passed'
    exit 0
}

# ---- -Check ----
$text = Read-Lf (Join-Path $root "$HIGHLIGHTS_DIR/v$Version.md")
$found = @(Test-Highlights $text)
foreach ($f in $found) { "FAIL $($f.Key) line $($f.Line): $($f.Detail)" }
if ($found.Count) { exit 1 }
"OK highlights v$Version ($(@($text -split "`n" -cmatch '^### ').Count) item(s))"
exit 0
