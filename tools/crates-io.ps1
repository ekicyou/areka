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
    7. 判定「公開の段の形」: WORKFLOW の on: の直下のきっかけが workflow_dispatch（必須）と push（任意・
       下は tags だけ）だけで、ファイルに secrets. の参照が無い（ファイルが無ければ失敗）
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
    WORKFLOW      公開の段の workflow のファイル

.PARAMETER Verify
  組み立てまでの形を選ぶ（cargo publish --dry-run・crates.io の索引を読む）。
  既に在る版は cargo が名前つきの警告で示し、失敗にしない。
  MSVC のリンカが見つかるよう、test-all.ps1 と同じ PATH の手当てをする。

.PARAMETER Version
  ワークスペースの版がこの値（v を付けない版・例 0.0.2）と違えば、二つの版を示して失敗する。
  -Pending と一緒に渡したときも、索引を読む前に判定する。

.PARAMETER Pending
  確認（3〜5・7〜9）はせず、較正・読み取り・（-Version のときは）版の判定の後、公開する一覧の
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
# 子の出力を端末の文字コードに依らず UTF-8 で読む関数 Invoke-Utf8Child
. (Join-Path $PSScriptRoot 'utf8-child.ps1')
Set-Location (Split-Path $PSScriptRoot -Parent)

# =============================================================================
# 較正値（説明の一覧と対応。変更はここだけ）
# =============================================================================
$PUBLISH   = @('dola', 'wintf')
$MAX_CRATE = 10485760
$INDEX_URL = 'https://index.crates.io/'
$WORK_DIR  = 'target/crates-io'
$WORKFLOW  = '.github/workflows/crates-io.yml'

# 人が読む行。別のプロセスから呼ぶと Write-Host も標準出力へ出るので、-Pending（標準出力は残りの名前だけ）では標準エラーへ
function Say([string]$Text, [ConsoleColor]$Color) {
    if ($Pending) { [Console]::Error.WriteLine($Text) } else { Write-Host $Text -ForegroundColor $Color }
}

function Fail([string]$Message) {
    Say "FAIL $Message" Red
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
    if ($extra) { "publishable but not in the list: $($extra -join ', ')" }
    if ($missing) { "in the list but not publishable: $($missing -join ', ')" }
}

function Test-Fields($Packages, [string[]]$Publish) {
    foreach ($pkg in @($Packages | Where-Object name -In $Publish)) {
        $empty = @('description', 'license', 'repository' | Where-Object { [string]::IsNullOrWhiteSpace($pkg.$_) })
        if ($empty) { "$($pkg.name) has empty fields: $($empty -join ', ')" }
    }
}

function Test-Reason([string]$Name, [string]$Manifest) {
    if ($Manifest -notmatch '(?m)^publish[ \t]*=[ \t]*false[ \t]*#[ \t]*\S') {
        "$Name Cargo.toml has no line 'publish = false # <reason>'"
    }
}

function Test-Version($Packages, [string[]]$Publish, [string]$Want) {
    foreach ($pkg in @($Packages | Where-Object name -In $Publish)) {
        if ($pkg.version -ne $Want) { "$($pkg.name) version $($pkg.version) differs from the given version $Want" }
    }
}

# $Sizes は 名前 → 包んだ .crate のバイト数（無ければ $null）
function Test-Size([hashtable]$Sizes, [long]$Max) {
    foreach ($name in $Sizes.Keys | Sort-Object) {
        $size = $Sizes[$name]
        if ($null -eq $size) { "$name .crate is missing" }
        elseif ($size -gt $Max) { "$name .crate is $size bytes, over the limit of $Max bytes" }
    }
}

# 索引を読んだ答え（HTTP の状態）。200 だけが読めた
function Test-Index([string]$Name, [int]$Status) {
    if ($Status -eq 404) { "$Name has no version on crates.io yet; publish it with the fallback procedure in the guide and set up Trusted Publishing" }
    elseif ($Status -ne 200) { "cannot read the index of $Name (HTTP $Status)" }
}

# 索引の本文（1 行 1 版の JSON）の vers にその版が無ければ、その文を返す（空なら在る）
function Test-Indexed([string]$Name, [string]$Want, [string]$Body) {
    $vers = @($Body -split '\r?\n' | Where-Object { $_.Trim() } | ForEach-Object { ($_ | ConvertFrom-Json -AsHashtable).vers })
    if ($Want -notin $vers) { "$Name $Want is not on crates.io" }
}

# 行の字下げの深さと、キーの名前（「名前:」の名前）と、キーと同じ行の値（コメントを除く）
function Indent([string]$Line) { ($Line -replace '\S.*$', '').Length }
function Key([string]$Line) { $Line.Trim() -replace '\s*:.*$', '' }
function Inline([string]$Line) { ($Line -replace '^[^:]*:', '' -replace '\s#.*$', '').Trim() }

# workflow のファイルの本文。YAML の読み手は使わず、行頭の on: の直下（字下げの最も浅い行）のキーと、
# その push: の直下のキーだけを見る。きっかけは workflow_dispatch（必須）と、tags だけを持つ push（任意）
function Test-Workflow([string]$Name, [string]$Text) {
    if ($Text -match 'secrets\.') { "$Name references secrets. (credentials are Trusted Publishing and github.token only)" }
    # 空行・コメントだけの行は読まない
    $lines = @($Text -split '\r?\n' | Where-Object { $_ -notmatch '^\s*(#.*)?$' })
    $at = @(for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -cmatch '^(on|"on"|''on'')[ \t]*:') { $i } })
    if ($at.Count -eq 0) { return "$Name has no top-level on:" }
    $inline = Inline $lines[$at[0]]
    if ($inline) {
        if ($inline -cne 'workflow_dispatch') { "$Name writes on: on one line (triggers must be workflow_dispatch and tag push only): $inline" }
        return
    }
    # 次の行頭の行までが on: の中
    $children = @(for ($i = $at[0] + 1; $i -lt $lines.Count -and $lines[$i] -match '^\s'; $i++) { $lines[$i] })
    if ($children.Count -eq 0) { return "$Name has no trigger under on:" }
    $depth = ($children | ForEach-Object { Indent $_ } | Measure-Object -Minimum).Minimum
    $top = @(for ($i = 0; $i -lt $children.Count; $i++) { if ((Indent $children[$i]) -eq $depth) { $i } })
    $keys = @($top | ForEach-Object { Key $children[$_] })
    if (@($keys | Where-Object { $_.StartsWith('-') }).Count) { return "$Name writes on: as a list (lines starting with -); write keys instead" }
    $other = @($keys | Where-Object { $_ -cnotin 'workflow_dispatch', 'push' })
    if ($other.Count) { "$Name has triggers other than workflow_dispatch and push under on: $($other -join ', ')" }
    if ('workflow_dispatch' -cnotin $keys) { "$Name has no workflow_dispatch (manual re-run entry) under on:" }
    $p = [array]::IndexOf($keys, 'push')
    if ($p -lt 0) { return }
    # push: の中は、次の同じ深さのキーまで
    if (Inline $children[$top[$p]]) { return "$Name writes push: on one line (put only tags: under it)" }
    $end = if ($p + 1 -lt $top.Count) { $top[$p + 1] } else { $children.Count }
    $sub = @(for ($i = $top[$p] + 1; $i -lt $end; $i++) { $children[$i] })
    $subDepth = ($sub | ForEach-Object { Indent $_ } | Measure-Object -Minimum).Minimum
    $subKeys = @($sub | Where-Object { (Indent $_) -eq $subDepth } | ForEach-Object { Key $_ })
    $bad = @($subKeys | Where-Object { $_ -cne 'tags' })
    if ($bad.Count) { "$Name has keys other than tags under push: $($bad -join ', ') (do not run on branch or path pushes)" }
    if ('tags' -cnotin $subKeys) { "$Name has no tags: under push: (run on tag push only)" }
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
    $text = $Got -join '; '
    if ($ShouldPass -and $Got) { Fail "calibration '$Judgment': good sample ($Sample) was rejected: $text" }
    if (-not $ShouldPass -and -not $Got) { Fail "calibration '$Judgment': bad sample ($Sample) was accepted" }
    foreach ($m in $Mention) {
        if (-not $text.Contains($m)) { Fail "calibration '$Judgment': failure text of bad sample ($Sample) lacks '$m': $text" }
    }
}

$p = Pkgs $good
Calibrate 'list' 'good sample' (Test-List $p $PUBLISH) $true
Calibrate 'list' 'crate outside the list publishes' (Test-List (Pkgs ($good -replace '"publish":\[\]', '"publish":null')) $PUBLISH) $false 'areka'
Calibrate 'list' 'crate outside the list names a registry' (Test-List (Pkgs ($good -replace '"publish":\[\]', '"publish":["crates-io"]')) $PUBLISH) $false 'areka'
Calibrate 'list' 'wintf does not publish' (Test-List (Pkgs ($good -replace '("wintf","version":"0.0.1","publish"):null', '$1:[]')) $PUBLISH) $false 'wintf'
Calibrate 'fields' 'good sample' (Test-Fields $p $PUBLISH) $true
Calibrate 'fields' 'empty description' (Test-Fields (Pkgs ($good -replace '("dola".*?"description"):"d"', '$1:""')) $PUBLISH) $false 'dola'
Calibrate 'reason' 'line with a reason' (Test-Reason 'areka' "[package]`r`nname = `"areka`"`r`npublish = false # reason`r`n") $true
Calibrate 'reason' 'bare publish = false line' (Test-Reason 'areka' "[package]`r`npublish = false`r`n") $false 'areka'
Calibrate 'reason' 'comment on another line' (Test-Reason 'areka' "[package]`r`n# reason`r`npublish = false`r`n") $false 'areka'
Calibrate 'reason' 'commented-out line' (Test-Reason 'areka' "[package]`r`n# publish = false # reason`r`n") $false 'areka'
Calibrate 'reason' 'empty comment' (Test-Reason 'areka' "[package]`r`npublish = false #`r`nname = `"areka`"`r`n") $false 'areka'
Calibrate 'version' 'same version' (Test-Version $p $PUBLISH '0.0.1') $true
Calibrate 'version' 'different version' (Test-Version $p $PUBLISH '9.9.9') $false '0.0.1', '9.9.9'
Calibrate 'size' 'exactly at the limit' (Test-Size @{ dola = 1; wintf = $MAX_CRATE } $MAX_CRATE) $true
Calibrate 'size' '1 byte over the limit' (Test-Size @{ dola = 1; wintf = $MAX_CRATE + 1 } $MAX_CRATE) $false 'wintf', "$($MAX_CRATE + 1)"
Calibrate 'size' '.crate missing' (Test-Size @{ dola = $null; wintf = 1 } $MAX_CRATE) $false 'dola'
# 索引の本文は 1 行 1 版の JSON。0.0.10 だけの本文は 0.0.1 を「在る」と読まない（文字の一部の一致で数えない）
$index = "{`"name`":`"dola`",`"vers`":`"0.0.1`",`"yanked`":false}`n{`"name`":`"dola`",`"vers`":`"0.0.2`",`"yanked`":false}`n"
Calibrate 'pending' 'version present' @((Test-Index 'dola' 200) + (Test-Indexed 'dola' '0.0.1' $index)) $true
Calibrate 'pending' 'version absent' (Test-Indexed 'dola' '0.0.1' ($index -replace '0\.0\.1"', '0.0.10"')) $false 'dola', '0.0.1'
Calibrate 'pending' 'index 404' (Test-Index 'dola' 404) $false 'dola', 'no version', 'fallback'
Calibrate 'pending' 'index 500' (Test-Index 'dola' 500) $false 'dola', '500'
# 実物の on: の形の写し。push の下の tags の値と、workflow_dispatch の下の inputs・version は、きっかけと読まない
$flow = "# description`r`nname: crates-io`r`n`r`non:`r`n  push:`r`n    tags: ['v*']`r`n  workflow_dispatch:`r`n    inputs:`r`n      version:`r`n        required: true`r`n`r`n# permissions`r`npermissions:`r`n  contents: read`r`n  id-token: write`r`n  actions: read`r`njobs:`r`n  publish:`r`n    env:`r`n      GH_TOKEN: `${{ github.token }}`r`n"
Calibrate 'workflow shape' 'good sample' (Test-Workflow 'w.yml' $flow) $true
Calibrate 'workflow shape' 'on: on the first line' (Test-Workflow 'w.yml' $flow.Substring($flow.IndexOf("`non:") + 1)) $true
Calibrate 'workflow shape' 'workflow_dispatch only' (Test-Workflow 'w.yml' ($flow -replace "  push:`r`n    tags: \['v\*'\]`r`n", '')) $true
Calibrate 'workflow shape' 'written as On:' (Test-Workflow 'w.yml' ($flow -creplace '(?m)^on:', 'On:')) $false 'w.yml', 'on:'
Calibrate 'workflow shape' 'pull_request added' (Test-Workflow 'w.yml' ($flow -replace '(?m)^  workflow_dispatch:', "  pull_request:`r`n  workflow_dispatch:")) $false 'w.yml', 'pull_request'
Calibrate 'workflow shape' 'workflow_run added' (Test-Workflow 'w.yml' ($flow -replace '(?m)^  workflow_dispatch:', "  workflow_run:`r`n    workflows: [release]`r`n    types: [completed]`r`n  workflow_dispatch:")) $false 'w.yml', 'workflow_run'
Calibrate 'workflow shape' 'schedule added' (Test-Workflow 'w.yml' ($flow -replace '(?m)^  workflow_dispatch:', "  schedule:`r`n    - cron: '0 0 * * *'`r`n  workflow_dispatch:")) $false 'w.yml', 'schedule'
Calibrate 'workflow shape' 'written as Push:' (Test-Workflow 'w.yml' ($flow -creplace '(?m)^  push:', '  Push:')) $false 'w.yml', 'Push'
Calibrate 'workflow shape' 'no workflow_dispatch' (Test-Workflow 'w.yml' ($flow -replace '(?ms)^  workflow_dispatch:.*?(?=^# permissions)', '')) $false 'w.yml', 'workflow_dispatch'
Calibrate 'workflow shape' 'branches under push' (Test-Workflow 'w.yml' ($flow -replace '(?m)^    tags:', "    branches: [main]`r`n    tags:")) $false 'w.yml', 'branches'
Calibrate 'workflow shape' 'paths under push' (Test-Workflow 'w.yml' ($flow -replace '(?m)^    tags:', "    paths: ['crates/**']`r`n    tags:")) $false 'w.yml', 'paths'
Calibrate 'workflow shape' 'empty push' (Test-Workflow 'w.yml' ($flow -replace "(?m)^    tags: .*`r`n", '')) $false 'w.yml', 'tags'
Calibrate 'workflow shape' 'push on one line' (Test-Workflow 'w.yml' ($flow -replace "(?m)^  push:`r`n    tags: .*`r`n", "  push: { tags: ['v*'] }`r`n")) $false 'w.yml', 'push'
Calibrate 'workflow shape' 'on: as a list' (Test-Workflow 'w.yml' ($flow -replace '(?ms)^on:.*?(?=^# permissions)', "on:`r`n  - push`r`n  - workflow_dispatch`r`n`r`n")) $false 'w.yml', 'list'
Calibrate 'workflow shape' 'on: on one line' (Test-Workflow 'w.yml' ($flow -replace '(?ms)^on:.*?(?=^# permissions)', "on: push`r`n")) $false 'w.yml', 'push'
Calibrate 'workflow shape' 'on: as a one-line list' (Test-Workflow 'w.yml' ($flow -replace '(?ms)^on:.*?(?=^# permissions)', "on: [push, workflow_dispatch]`r`n")) $false 'w.yml', 'push'
Calibrate 'workflow shape' 'no on:' (Test-Workflow 'w.yml' ($flow -replace '(?m)^on:', 'xon:')) $false 'w.yml', 'on:'
Calibrate 'workflow shape' 'contains secrets.' (Test-Workflow 'w.yml' ($flow -replace 'github\.token', 'secrets.GITHUB_TOKEN')) $false 'w.yml', 'secrets.'

# =============================================================================
# 2〜6. 実物の判定
# =============================================================================
# 括弧の中は子の標準エラーの最後の行。JSON として読めないときは捕まえない例外（終了コード 1）
$r = Invoke-Utf8Child cargo @('metadata', '--no-deps', '--locked', '--format-version', '1')
if ($r.Code) { Fail "read: cargo metadata exited with code $($r.Code) ($($r.ErrLines | Select-Object -Last 1))" }
$packages = @(($r.Out | ConvertFrom-Json).packages)

function Check([string]$Judgment, [string[]]$Got) {
    if ($Got) { Fail "check '$Judgment': $($Got -join '; ')" }
    Say "OK check '$Judgment'" Green
}

if (-not $Pending) {
    Check 'list' (Test-List $packages $PUBLISH)
    Check 'fields' (Test-Fields $packages $PUBLISH)
    Check 'reason' @($packages | Where-Object name -NotIn $PUBLISH | ForEach-Object {
            Test-Reason $_.name (Get-Content -Raw -LiteralPath $_.manifest_path)
        })
}
# -Pending でも索引を読む前に判定する
if ($Version) { Check 'version' (Test-Version $packages $PUBLISH $Version) }

# =============================================================================
# -Pending: ワークスペースの版がまだ crates.io に無いクレートの名前だけを標準出力へ
# =============================================================================
if ($Pending) {
    $ProgressPreference = 'SilentlyContinue'
    foreach ($name in $PUBLISH) {
        $pkg = $packages | Where-Object name -EQ $name
        if (-not $pkg) { Fail "pending: $name is not in the workspace" }
        # ponytail: 索引の置き場は 4 文字以上の名前の規則だけ。短い名前を一覧に足すときに 1〜3 文字の規則を足す
        $low = $name.ToLowerInvariant()
        if ($low.Length -lt 4) { Fail "pending: $name is shorter than 4 characters; its index path rule is not implemented" }
        $url = "$INDEX_URL$($low.Substring(0, 2))/$($low.Substring(2, 2))/$low"
        try { $res = Invoke-WebRequest -Uri $url -SkipHttpErrorCheck }
        catch { Fail "pending: cannot read the index of $name ($($_.Exception.Message))" }
        $err = Test-Index $name ([int]$res.StatusCode)
        if ($err) { Fail "pending: $err" }
        try { $missing = Test-Indexed $name $pkg.version ([string]$res.Content) }
        catch { Fail "pending: cannot read the index of $name (body: $($_.Exception.Message))" }
        if ($missing) {
            Say "absent $name $($pkg.version)"
            $name
        } else {
            Say "present $name $($pkg.version)"
        }
    }
    exit 0
}

# =============================================================================
# 8〜9. 包んで、大きさを判定する（.crate はどちらの形でも package\tmp-crate\ に出る）
# =============================================================================
# 7. 公開の段の形（公開の段の中では、タグのコミットに在る写しを見る）
if (-not (Test-Path -LiteralPath $WORKFLOW -PathType Leaf)) { Fail "check 'workflow shape': $WORKFLOW is missing" }
Check 'workflow shape' (Test-Workflow $WORKFLOW (Get-Content -Raw -LiteralPath $WORKFLOW))

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
if ($LASTEXITCODE) { Fail "package: $step exited with code $LASTEXITCODE ($($PUBLISH -join ', '))" }
Write-Host "OK package ($step)" -ForegroundColor Green

$sizes = @{}
foreach ($pkg in @($packages | Where-Object name -In $PUBLISH)) {
    $file = Get-Item -LiteralPath "$crateDir/$($pkg.name)-$($pkg.version).crate" -ErrorAction Ignore
    $sizes[$pkg.name] = $file ? $file.Length : $null
}
Check 'size' (Test-Size $sizes $MAX_CRATE)

Write-Host "green (publish list: $($PUBLISH -join ', '))" -ForegroundColor Green
exit 0
