<#
.SYNOPSIS
  α の配布物（zip）を今のソースから組み、中身を判定する（-Check で展開して起動も確かめる）。

.DESCRIPTION
  段を名前つきで直列に回し、どれかの段が失敗したらその段の名前と外部コマンドの出力の末尾を
  印字して、後始末（.zip.tmp の削除・差し替えた環境変数の復元）をしてから即終了する
  （tools/test-all.ps1 の「最後まで回す」とは逆）。
  最後の段「git status 不変の確認」で、始めと終わりの git status --porcelain が同じことを確かめる
  （追跡しているファイルを 1 つも書き換えない。zip と途中物は追跡外の target/alpha/ に置く）。

  注意: 同じ検体で実機を回している最中に実行すると、その走行の展開した木を消す（target/nar-samples/manual/<検体>/）。
  検体の展開は nar-sample-path が同じ manual/<検体>/ を消してから作り直すため。実機の走行が終わってから実行すること。

  終了コード:
    0 … 全段が緑
    1 … 組む段の失敗（段の名前を印字する。git status が変わったときもここ）
    2 … 起動確認（-Check）の否
    3 … 引数・前提の不正（何を入れるか／どう直すかを 1 行で印字する）

  較正値・調整値の一覧（変更するときはスクリプト冒頭の「較正値」の 1 か所だけを書き換える）:
    SCRIPT_VERSION                  本スクリプトの版（BUILD-INFO.txt の script= に書く）
    SMOKE_EXIT_MS        = 20000    -Check の有界の自動終了（ミリ秒・-SmokeExitMs で上書き）
    WATCHDOG_MARGIN_SEC  = 60       自動終了の予定から番犬が子を止めるまでの猶予（秒）
    EXPAND_DIR_MAX_CHARS = 160      -Check の展開先のフルパスの長さの上限（文字）
    DLL_DENY_PREFIXES               取り込み表に在ってはならない DLL 名の前方一致（大文字小文字を区別しない）
    ALLOWED_EXECUTABLES             zip に入れてよい .exe／.dll の項目名
    LOG_MARKER_*                    -Check の記録の判定に使う目印の文言
    OUTPUT_TAIL_LINES    = 40       段の失敗のときに印字する外部コマンドの出力の末尾の行数

.PARAMETER Check
  zip を組んだあと、短いパスへ展開して areka.exe を有界で起動し、記録を判定する。

.PARAMETER CheckDir
  -Check の展開先を作る親フォルダ（絶対パス）。省略時は一時フォルダ。
  その下に areka-alpha-check-<HHmmss> を作る。そのフルパスが EXPAND_DIR_MAX_CHARS を超えると終了コード 3。

.PARAMETER SmokeExitMs
  -Check の有界の自動終了（ミリ秒・正の整数）。省略時は SMOKE_EXIT_MS。

.EXAMPLE
  pwsh -NoProfile -File tools/package-alpha.ps1

.EXAMPLE
  pwsh -NoProfile -File tools/package-alpha.ps1 -Check -CheckDir C:\t -SmokeExitMs 20000
#>
# 使い方の説明（上）は Get-Help がファイルの先頭でしか読まないので、#Requires はここに置く
#Requires -Version 7
param(
    [switch]$Check,
    [string]$CheckDir,
    # 数でない値も終了コード 3 で断るため、文字列で受けて自分で読む
    [string]$SmokeExitMs
)

Set-StrictMode -Version 3.0
$SmokeExitMsGiven = $PSBoundParameters.ContainsKey('SmokeExitMs')
$ErrorActionPreference = 'Stop'
# 外部コマンドの標準エラー出力で例外を投げさせない。終了コードは自前で見る。
$PSNativeCommandUseErrorActionPreference = $false

# =============================================================================
# 較正値（説明の一覧と対応。変更はここだけ）
# =============================================================================
$SCRIPT_VERSION       = '1.0.0'
$SMOKE_EXIT_MS        = 20000
$WATCHDOG_MARGIN_SEC  = 60
$EXPAND_DIR_MAX_CHARS = 160
$DLL_DENY_PREFIXES    = @('vcruntime', 'msvcp', 'msvcr', 'api-ms-win-crt-', 'ucrtbase', 'concrt')
$ALLOWED_EXECUTABLES  = @('areka.exe', 'shiori-host32-helper.exe', 'ghost/emo2/ghost/master/pasta.dll')
$LOG_MARKER_SMOKE_GATE     = 'smoke 自動 close ゲート有効'          # crates/areka/src/main.rs の SMOKE_EXIT_ENV の info!
$LOG_MARKER_WINDOWS        = '本物のゴースト窓を開きました'          # crates/areka/src/ghost_session.rs
$LOG_MARKER_FAULTS         = @('SHIORI が動かなくなりました', 'event="connect_failed"', 'event="helper_exited"')
$LOG_MARKER_GREETING       = '起動グリーティングを再生起動'          # areka-kanade/src/schedule/boot.rs（204 側の文言は数えない）
$LOG_MARKER_BALLOON        = 'バルーンを決めました'                  # crates/areka/src/boot_config.rs の balloon_resolved
$LOG_MARKER_BALLOON_ROUTE  = 'route=Companion'
$LOG_MARKER_BALLOON_DIR    = '\balloon\emo2-kakukaku'
$OUTPUT_TAIL_LINES    = 40

$EXIT_OK = 0; $EXIT_STAGE_FAILED = 1; $EXIT_CHECK_FAILED = 2; $EXIT_BAD_ARGS = 3

Set-Location (Split-Path $PSScriptRoot -Parent)

# Git Bash から呼ばれると GNU coreutils の link.exe が MSVC の link.exe を遮蔽し i686 のリンクが落ちる。
# link.exe を持つが cl.exe を持たないフォルダ（＝MSVC 以外の link.exe）を、このプロセスの PATH から外す。
# （tools/test-all.ps1 と同じ）
$env:PATH = ($env:PATH -split ';' | Where-Object {
        -not $_ -or -not (Test-Path (Join-Path $_ 'link.exe')) -or (Test-Path (Join-Path $_ 'cl.exe'))
    }) -join ';'

# =============================================================================
# 後始末
# =============================================================================
$script:ZipTmp = $null       # 圧縮の段が .zip.tmp のパスを入れる
$script:SavedEnv = @{}       # Set-EnvTemp が差し替える前の値（$null は「無かった」）

# 環境変数をこのプロセスで差し替える。元の値は最初の 1 回だけ覚え、後始末で戻す。
function Set-EnvTemp([string]$Name, [AllowNull()][string]$Value) {
    if (-not $script:SavedEnv.ContainsKey($Name)) {
        $script:SavedEnv[$Name] = [Environment]::GetEnvironmentVariable($Name)
    }
    [Environment]::SetEnvironmentVariable($Name, $Value)
}

function Restore-Env {
    foreach ($name in @($script:SavedEnv.Keys)) {
        [Environment]::SetEnvironmentVariable($name, $script:SavedEnv[$name])
    }
    $script:SavedEnv.Clear()
}

function Invoke-Cleanup {
    if ($script:ZipTmp -and (Test-Path -LiteralPath $script:ZipTmp)) {
        Remove-Item -LiteralPath $script:ZipTmp -Force
    }
    Restore-Env
}

function Exit-Script([int]$Code, [string]$Message) {
    Invoke-Cleanup
    if ($Message) { Write-Host $Message -ForegroundColor Red }
    exit $Code
}

# =============================================================================
# 段の直列
# =============================================================================
# 段は外部コマンドの終了コードが非 0 か、throw（理由の文）で失敗する。失敗したら段の名前と
# 出力の末尾を印字し、後始末をして終了コード 1。前提の不正は段の中で Exit-Script 3 を呼ぶ。
function Step([string]$Name, [scriptblock]$Command) {
    Write-Host "`n==> $Name" -ForegroundColor Cyan
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $global:LASTEXITCODE = 0
    $out = [Collections.Generic.List[string]]::new()
    $reason = $null
    try {
        & $Command 2>&1 | ForEach-Object { $line = "$_"; $out.Add($line); Write-Host $line }
        if ($LASTEXITCODE) { $reason = "終了コード $LASTEXITCODE" }
    } catch {
        $reason = "$_"
    }
    $sec = [int]$sw.Elapsed.TotalSeconds
    if ($null -eq $reason) {
        Write-Host "OK   $Name（$sec 秒）" -ForegroundColor Green
        return
    }
    if ($out.Count) {
        Write-Host "`n---- 出力の末尾（最大 $OUTPUT_TAIL_LINES 行） ----"
        $out | Select-Object -Last $OUTPUT_TAIL_LINES | ForEach-Object { Write-Host $_ }
    }
    Exit-Script $EXIT_STAGE_FAILED "FAIL $Name（$sec 秒・$reason）"
}

# =============================================================================
# 段
# =============================================================================
Step '前提の確認' {
    # 引数
    if ($SmokeExitMsGiven) {
        $ms = 0
        if (-not [int]::TryParse($SmokeExitMs, [ref]$ms) -or $ms -le 0) {
            Exit-Script $EXIT_BAD_ARGS "-SmokeExitMs は正の整数で指定する（受け取った値: '$SmokeExitMs'）"
        }
        $script:SmokeMs = $ms
    } else {
        $script:SmokeMs = $SMOKE_EXIT_MS
    }
    if ($CheckDir -and -not [IO.Path]::IsPathFullyQualified($CheckDir)) {
        Exit-Script $EXIT_BAD_ARGS "-CheckDir は絶対パスで指定する（受け取った値: '$CheckDir'）"
    }
    $parent = if ($CheckDir) { $CheckDir } else { [IO.Path]::GetTempPath() }
    $script:ExpandDir = Join-Path $parent ('areka-alpha-check-' + (Get-Date -Format 'HHmmss'))
    if (($Check -or $CheckDir) -and $script:ExpandDir.Length -gt $EXPAND_DIR_MAX_CHARS) {
        Exit-Script $EXIT_BAD_ARGS ("展開先が長すぎる（{0} 文字・上限 {1}）: {2} — -CheckDir に短いパスを指定する" -f $script:ExpandDir.Length, $EXPAND_DIR_MAX_CHARS, $script:ExpandDir)
    }

    # git（コミットは 7 桁固定。--short だけだと曖昧なとき 8 桁以上を返す）
    $script:Commit = git rev-parse --short=7 HEAD 2>$null
    if ($LASTEXITCODE -or -not $script:Commit) { Exit-Script $EXIT_BAD_ARGS 'git が動かない（git を入れて PATH に通す）' }
    $script:StatusBefore = @(git status --porcelain)
    $script:Dirty = $script:StatusBefore.Count

    # 道具
    $null = cargo about --version 2>&1
    if ($LASTEXITCODE) { Exit-Script $EXIT_BAD_ARGS 'cargo about が無い（cargo install cargo-about --version 0.9.2 --locked で入れる）' }
    $null = cargo deny --version 2>&1
    if ($LASTEXITCODE) { Exit-Script $EXIT_BAD_ARGS 'cargo deny が無い（cargo install cargo-deny --version 0.20.2 --locked で入れる）' }
    if (-not (Test-Path -LiteralPath 'Cargo.lock')) { Exit-Script $EXIT_BAD_ARGS 'Cargo.lock が無い（追跡している Cargo.lock を git から戻す）' }

    Write-Host "コミット $script:Commit・未コミットの変更 $script:Dirty 件"
}

# 組む段（ビルド・ライセンス・謝辞・展開・組み立て・圧縮・中身の判定・完成）と
# -Check の段（展開・起動・番犬・記録の判定）はこの位置へ Step で並べる。

Step 'git status 不変の確認' {
    $after = @(git status --porcelain)
    if ($LASTEXITCODE) { throw 'git status が失敗した' }
    if (($script:StatusBefore -join "`n") -ceq ($after -join "`n")) { return }
    $script:StatusBefore | Where-Object { $after -cnotcontains $_ } | ForEach-Object { Write-Host "消えた $_" }
    $after | Where-Object { $script:StatusBefore -cnotcontains $_ } | ForEach-Object { Write-Host "増えた $_" }
    throw 'git status --porcelain が始めと違う'
}

Invoke-Cleanup
Write-Host "`n全段 緑" -ForegroundColor Green
exit $EXIT_OK
