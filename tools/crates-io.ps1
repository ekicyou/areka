<#
.SYNOPSIS
  crates.io へ出すクレート（公開する一覧）が、何も上げずに「この版で出せるか」を確かめる。

.DESCRIPTION
  順に行い、1 つ失敗したらそこで終わる。
    1. 判定の較正: 下の判定を、このスクリプトに埋めた正しい見本と誤った見本に当て、
       正しく通し・正しく落とすことを確かめる（道具が壊れたまま緑を出さない）
    2. 読み取り: cargo metadata --no-deps --format-version 1 --locked
    3. 判定「一覧」: 出せると扱われるクレート（publish が [] でない）の集まりが、公開する一覧とちょうど同じ
    4. 判定「欄」: 公開する一覧の各クレートの説明・ライセンス・リポジトリが空でない
    5. 判定「理由」: 出さない各クレートの Cargo.toml に、行頭から「publish = false # 理由」の行がある
    6. 判定「版」（-Version のときだけ）: 公開する一覧の各クレートの版が渡された値と同じ
  環境変数は読まない（手元と CI で同じ動き）。呼ばれた場所によらずリポジトリの根へ移る。
  追跡されたファイルは書き換えない。

  終了コード: 0 緑／1 失敗（どの判定・どのクレートかを 1 行で示す）

  較正値（変更はスクリプト冒頭の「較正値」の 1 か所だけ）:
    PUBLISH       公開する一覧（新しいクレートを出すときはここに足す）
    MAX_CRATE     1 クレートを包んだ大きさの上限（crates.io の上限・圧縮後 10 MB）
    INDEX_URL     crates.io の索引
    WORK_DIR      作業の置き場（cargo の --target-dir）

.PARAMETER Verify
  組み立てまでの形を選ぶ。

.PARAMETER Version
  ワークスペースの版がこの値（v を付けない版・例 0.0.2）と違えば、二つの版を示して失敗する。

.EXAMPLE
  pwsh -NoProfile -File tools/crates-io.ps1
  pwsh -NoProfile -File tools/crates-io.ps1 -Version 0.0.2
#>
#Requires -Version 7
# 知らない引数（綴り違いを含む）は黙って捨てずに断る
[CmdletBinding()]
param([switch]$Verify, [string]$Version)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
# cargo の出力（JSON・UTF-8）を文字化けさせずに読む
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Set-Location (Split-Path $PSScriptRoot -Parent)

# =============================================================================
# 較正値（説明の一覧と対応。変更はここだけ）
# =============================================================================
$PUBLISH   = @('dola', 'wintf')
$MAX_CRATE = 10485760
$INDEX_URL = 'https://index.crates.io/'
$WORK_DIR  = 'target/crates-io'

function Fail([string]$Message) {
    Write-Host "失敗 $Message" -ForegroundColor Red
    exit 1
}

# =============================================================================
# 判定（入力 → 失敗の文の一覧。空なら通る）。$Packages は cargo metadata の packages の形
# =============================================================================
# publish が null は「どこへでも出せる」、[] は「出さない」、名前の並びは「その置き場へ出せる」
function Test-List($Packages, [string[]]$Publish) {
    $can = @($Packages | Where-Object { $null -eq $_.publish -or @($_.publish).Count } | ForEach-Object name)
    $extra = @($can | Where-Object { $_ -notin $Publish })
    $missing = @($Publish | Where-Object { $_ -notin $can })
    if ($extra) { "一覧の外なのに出せる: $($extra -join ', ')" }
    if ($missing) { "一覧にあるのに出せない: $($missing -join ', ')" }
}

function Test-Fields($Packages, [string[]]$Publish) {
    foreach ($pkg in @($Packages | Where-Object name -In $Publish)) {
        $empty = @('description', 'license', 'repository' | Where-Object { [string]::IsNullOrWhiteSpace($pkg.$_) })
        if ($empty) { "$($pkg.name) の欄が空: $($empty -join ', ')" }
    }
}

function Test-Reason([string]$Name, [string]$Manifest) {
    if ($Manifest -notmatch '(?m)^publish[ \t]*=[ \t]*false[ \t]*#[ \t]*\S') {
        "$Name の Cargo.toml に「publish = false # 理由」の行が無い"
    }
}

function Test-Version($Packages, [string[]]$Publish, [string]$Want) {
    foreach ($pkg in @($Packages | Where-Object name -In $Publish)) {
        if ($pkg.version -ne $Want) { "$($pkg.name) の版 $($pkg.version) が渡された版 $Want と違う" }
    }
}

# =============================================================================
# 1. 判定の較正
# =============================================================================
function Pkgs([string]$Json) { @(($Json | ConvertFrom-Json).packages) }

# 正しい見本と、その一部を取り替えた誤った見本
$good = '{"packages":[
  {"name":"dola","version":"0.0.1","publish":null,"description":"d","license":"MIT","repository":"r"},
  {"name":"wintf","version":"0.0.1","publish":null,"description":"d","license":"MIT","repository":"r"},
  {"name":"areka","version":"0.0.1","publish":[],"description":null,"license":null,"repository":null}]}'

function Calibrate([string]$Judgment, [string]$Sample, [string[]]$Got, [bool]$ShouldPass, [string[]]$Mention = @()) {
    $text = $Got -join '・'
    if ($ShouldPass -and $Got) { Fail "較正「$Judgment」: 正しい見本（$Sample）を落とした: $text" }
    if (-not $ShouldPass -and -not $Got) { Fail "較正「$Judgment」: 誤った見本（$Sample）を通した" }
    foreach ($m in $Mention) {
        if (-not $text.Contains($m)) { Fail "較正「$Judgment」: 誤った見本（$Sample）の失敗の文に「$m」が無い: $text" }
    }
}

$p = Pkgs $good
Calibrate '一覧' '正しい見本' (Test-List $p $PUBLISH) $true
Calibrate '一覧' '一覧の外のクレートが出す' (Test-List (Pkgs ($good -replace '"publish":\[\]', '"publish":null')) $PUBLISH) $false 'areka'
Calibrate '一覧' '一覧の外のクレートが置き場を名指しで出す' (Test-List (Pkgs ($good -replace '"publish":\[\]', '"publish":["crates-io"]')) $PUBLISH) $false 'areka'
Calibrate '一覧' 'wintf が出さない' (Test-List (Pkgs ($good -replace '("wintf","version":"0.0.1","publish"):null', '$1:[]')) $PUBLISH) $false 'wintf'
Calibrate '欄' '正しい見本' (Test-Fields $p $PUBLISH) $true
Calibrate '欄' '説明が空' (Test-Fields (Pkgs ($good -replace '("dola".*?"description"):"d"', '$1:""')) $PUBLISH) $false 'dola'
Calibrate '理由' '理由つきの行' (Test-Reason 'areka' "[package]`r`nname = `"areka`"`r`npublish = false # 理由`r`n") $true
Calibrate '理由' 'publish = false だけの行' (Test-Reason 'areka' "[package]`r`npublish = false`r`n") $false 'areka'
Calibrate '理由' 'コメントが別の行' (Test-Reason 'areka' "[package]`r`n# 理由`r`npublish = false`r`n") $false 'areka'
Calibrate '理由' 'コメントにした行' (Test-Reason 'areka' "[package]`r`n# publish = false # 理由`r`n") $false 'areka'
Calibrate '理由' '中身の無いコメント' (Test-Reason 'areka' "[package]`r`npublish = false #`r`nname = `"areka`"`r`n") $false 'areka'
Calibrate '版' '同じ版' (Test-Version $p $PUBLISH '0.0.1') $true
Calibrate '版' '違う版' (Test-Version $p $PUBLISH '9.9.9') $false '0.0.1', '9.9.9'

# =============================================================================
# 2〜6. 実物の判定
# =============================================================================
$out = @(cargo metadata --no-deps --format-version 1 --locked 2>&1)
if ($LASTEXITCODE) { Fail "読み取り: cargo metadata が終了コード $LASTEXITCODE（$($out | Select-Object -Last 1)）" }
$packages = @((($out | Where-Object { $_ -isnot [Management.Automation.ErrorRecord] }) -join "`n" | ConvertFrom-Json).packages)

function Check([string]$Judgment, [string[]]$Got) {
    if ($Got) { Fail "判定「$Judgment」: $($Got -join '・')" }
    Write-Host "OK 判定「$Judgment」" -ForegroundColor Green
}

Check '一覧' (Test-List $packages $PUBLISH)
Check '欄' (Test-Fields $packages $PUBLISH)
Check '理由' @($packages | Where-Object name -NotIn $PUBLISH | ForEach-Object {
        Test-Reason $_.name (Get-Content -Raw -LiteralPath $_.manifest_path)
    })
if ($Version) { Check '版' (Test-Version $packages $PUBLISH $Version) }

Write-Host "緑（公開する一覧: $($PUBLISH -join '・')）" -ForegroundColor Green
exit 0
