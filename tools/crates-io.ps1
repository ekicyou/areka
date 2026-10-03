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
    8. 包む: 前回の .crate を消してから、公開する一覧を包む（cargo の出力はそのまま見せる）
         引数なし: cargo package --no-verify --allow-dirty --locked --offline（ネットを使わない）
         -Verify : cargo publish --dry-run --allow-dirty --locked（索引を読み、組み立てまで）
    9. 判定「大きさ」: WORK_DIR\package\tmp-crate\{名前}-{版}.crate が在り、MAX_CRATE 以下
  --dry-run の無い cargo publish は呼ばない。作業の物は WORK_DIR の下だけ。
  環境変数は読まない（手元と CI で同じ動き）。呼ばれた場所によらずリポジトリの根へ移る。
  追跡されたファイルは書き換えない。

  終了コード: 0 緑／1 失敗（どの判定・どのクレートかを 1 行で示す）

  較正値（変更はスクリプト冒頭の「較正値」の 1 か所だけ）:
    PUBLISH       公開する一覧（新しいクレートを出すときはここに足す）
    MAX_CRATE     1 クレートを包んだ大きさの上限（crates.io の上限・圧縮後 10 MB）
    INDEX_URL     crates.io の索引
    WORK_DIR      作業の置き場（cargo の --target-dir）

.PARAMETER Verify
  組み立てまでの形を選ぶ（cargo publish --dry-run・crates.io の索引を読む）。
  既に在る版は cargo が名前つきの警告で示し、失敗にしない。
  MSVC のリンカが見つかるよう、test-all.ps1 と同じ PATH の手当てをする。

.PARAMETER Version
  ワークスペースの版がこの値（v を付けない版・例 0.0.2）と違えば、二つの版を示して失敗する。
  -Pending と一緒に渡したときも、索引を読む前に判定する。

.PARAMETER Pending
  確認（3〜5・8〜9）はせず、較正・読み取り・（-Version のときは）版の判定の後、公開する一覧の
  各クレートの索引（INDEX_URL{先頭2字}/{次の2字}/{名前}・4 文字以上の名前だけ）を 1 回読み、
  ワークスペースの版がまだ無いクレートの名前だけを標準出力へ 1 行ずつ出す。
  人が読む行（在る・無い・OK・失敗）は標準エラーへ。索引が 404（1 つも版が無い）・読めないときは名前つきで失敗する。

.EXAMPLE
  pwsh -NoProfile -File tools/crates-io.ps1
  pwsh -NoProfile -File tools/crates-io.ps1 -Verify -Version 0.0.2
  pwsh -NoProfile -File tools/crates-io.ps1 -Pending -Version 0.0.2
#>
#Requires -Version 7
# 知らない引数（綴り違いを含む）は黙って捨てずに断る
[CmdletBinding()]
param([switch]$Verify, [string]$Version, [switch]$Pending)

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

# 人が読む行。別のプロセスから呼ぶと Write-Host も標準出力へ出るので、-Pending（標準出力は残りの名前だけ）では標準エラーへ
function Say([string]$Text, [ConsoleColor]$Color) {
    if ($Pending) { [Console]::Error.WriteLine($Text) } else { Write-Host $Text -ForegroundColor $Color }
}

function Fail([string]$Message) {
    Say "失敗 $Message" Red
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

# $Sizes は 名前 → 包んだ .crate のバイト数（無ければ $null）
function Test-Size([hashtable]$Sizes, [long]$Max) {
    foreach ($name in $Sizes.Keys | Sort-Object) {
        $size = $Sizes[$name]
        if ($null -eq $size) { "$name の .crate が無い" }
        elseif ($size -gt $Max) { "$name の .crate が $size バイトで上限 $Max バイトを超える" }
    }
}

# 索引を読んだ答え（HTTP の状態）。200 だけが読めた
function Test-Index([string]$Name, [int]$Status) {
    if ($Status -eq 404) { "$Name は crates.io に 1 つも版が無い・手順書の予備の手順で出して Trusted Publishing を設定する" }
    elseif ($Status -ne 200) { "$Name の索引を読めない（HTTP $Status）" }
}

# 索引の本文（1 行 1 版の JSON）の vers にその版が無ければ、その文を返す（空なら在る）
function Test-Indexed([string]$Name, [string]$Want, [string]$Body) {
    $vers = @($Body -split '\r?\n' | Where-Object { $_.Trim() } | ForEach-Object { ($_ | ConvertFrom-Json -AsHashtable).vers })
    if ($Want -notin $vers) { "$Name の $Want が crates.io に無い" }
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
Calibrate '大きさ' '上限ちょうど' (Test-Size @{ dola = 1; wintf = $MAX_CRATE } $MAX_CRATE) $true
Calibrate '大きさ' '上限を 1 バイト超え' (Test-Size @{ dola = 1; wintf = $MAX_CRATE + 1 } $MAX_CRATE) $false 'wintf', "$($MAX_CRATE + 1)"
Calibrate '大きさ' '.crate が無い' (Test-Size @{ dola = $null; wintf = 1 } $MAX_CRATE) $false 'dola'
# 索引の本文は 1 行 1 版の JSON。0.0.10 だけの本文は 0.0.1 を「在る」と読まない（文字の一部の一致で数えない）
$index = "{`"name`":`"dola`",`"vers`":`"0.0.1`",`"yanked`":false}`n{`"name`":`"dola`",`"vers`":`"0.0.2`",`"yanked`":false}`n"
Calibrate '残り' 'その版が在る' @((Test-Index 'dola' 200) + (Test-Indexed 'dola' '0.0.1' $index)) $true
Calibrate '残り' 'その版が無い' (Test-Indexed 'dola' '0.0.1' ($index -replace '0\.0\.1"', '0.0.10"')) $false 'dola', '0.0.1'
Calibrate '残り' '索引が 404' (Test-Index 'dola' 404) $false 'dola', '1 つも版が無い', '予備の手順'
Calibrate '残り' '索引が 500' (Test-Index 'dola' 500) $false 'dola', '500'

# =============================================================================
# 2〜6. 実物の判定
# =============================================================================
$out = @(cargo metadata --no-deps --format-version 1 --locked 2>&1)
if ($LASTEXITCODE) { Fail "読み取り: cargo metadata が終了コード $LASTEXITCODE（$($out | Select-Object -Last 1)）" }
$packages = @((($out | Where-Object { $_ -isnot [Management.Automation.ErrorRecord] }) -join "`n" | ConvertFrom-Json).packages)

function Check([string]$Judgment, [string[]]$Got) {
    if ($Got) { Fail "判定「$Judgment」: $($Got -join '・')" }
    Say "OK 判定「$Judgment」" Green
}

if (-not $Pending) {
    Check '一覧' (Test-List $packages $PUBLISH)
    Check '欄' (Test-Fields $packages $PUBLISH)
    Check '理由' @($packages | Where-Object name -NotIn $PUBLISH | ForEach-Object {
            Test-Reason $_.name (Get-Content -Raw -LiteralPath $_.manifest_path)
        })
}
# -Pending でも索引を読む前に判定する
if ($Version) { Check '版' (Test-Version $packages $PUBLISH $Version) }

# =============================================================================
# -Pending: ワークスペースの版がまだ crates.io に無いクレートの名前だけを標準出力へ
# =============================================================================
if ($Pending) {
    $ProgressPreference = 'SilentlyContinue'
    foreach ($name in $PUBLISH) {
        $pkg = $packages | Where-Object name -EQ $name
        if (-not $pkg) { Fail "残り: $name がワークスペースに無い" }
        # ponytail: 索引の置き場は 4 文字以上の名前の規則だけ。短い名前を一覧に足すときに 1〜3 文字の規則を足す
        $low = $name.ToLowerInvariant()
        if ($low.Length -lt 4) { Fail "残り: $name は 4 文字未満で、索引の置き場の規則を実装していない" }
        $url = "$INDEX_URL$($low.Substring(0, 2))/$($low.Substring(2, 2))/$low"
        try { $res = Invoke-WebRequest -Uri $url -SkipHttpErrorCheck }
        catch { Fail "残り: $name の索引を読めない（$($_.Exception.Message)）" }
        $err = Test-Index $name ([int]$res.StatusCode)
        if ($err) { Fail "残り: $err" }
        try { $missing = Test-Indexed $name $pkg.version ([string]$res.Content) }
        catch { Fail "残り: $name の索引を読めない（本文: $($_.Exception.Message)）" }
        if ($missing) {
            Say "無い $name $($pkg.version)"
            $name
        } else {
            Say "在る $name $($pkg.version)"
        }
    }
    exit 0
}

# =============================================================================
# 8〜9. 包んで、大きさを判定する（.crate はどちらの形でも package\tmp-crate\ に出る）
# =============================================================================
$crateDir = "$WORK_DIR/package/tmp-crate"
Remove-Item -Path "$crateDir/*.crate", "$WORK_DIR/package/*.crate" -Force -ErrorAction Ignore
$pArgs = $PUBLISH | ForEach-Object { '-p', $_ }
if ($Verify) {
    # test-all.ps1 と同じ手当て: MSVC 以外の link.exe（Git Bash の coreutils）を PATH から外す
    $env:PATH = ($env:PATH -split ';' | Where-Object {
            -not $_ -or -not (Test-Path (Join-Path $_ 'link.exe')) -or (Test-Path (Join-Path $_ 'cl.exe'))
        }) -join ';'
    $step = 'cargo publish --dry-run'
    cargo publish --dry-run --allow-dirty --locked --target-dir $WORK_DIR @pArgs
} else {
    $step = 'cargo package'
    cargo package --no-verify --allow-dirty --locked --offline --target-dir $WORK_DIR @pArgs
}
if ($LASTEXITCODE) { Fail "包む: $step が終了コード $LASTEXITCODE（$($PUBLISH -join '・')）" }
Write-Host "OK 包む（$step）" -ForegroundColor Green

$sizes = @{}
foreach ($pkg in @($packages | Where-Object name -In $PUBLISH)) {
    $file = Get-Item -LiteralPath "$crateDir/$($pkg.name)-$($pkg.version).crate" -ErrorAction Ignore
    $sizes[$pkg.name] = $file ? $file.Length : $null
}
Check '大きさ' (Test-Size $sizes $MAX_CRATE)

Write-Host "緑（公開する一覧: $($PUBLISH -join '・')）" -ForegroundColor Green
exit 0
