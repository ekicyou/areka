<#
.SYNOPSIS
  配布物の zip（areka-{版}-{arch}.zip）と SHA256 の検証ファイル（areka-{版}-{arch}.zip.sha256）を
  今のソースから target/package/ に組み、中身を判定する（-Check で展開して起動も確かめる）。

.DESCRIPTION
  作る物と置き場:
    target/package/areka-{版}-{arch}.zip         配布物（{版} は Cargo.toml の [workspace.package] version）
    target/package/areka-{版}-{arch}.zip.sha256  「SHA256（16 進 64 字）・空白 2 つ・zip のファイル名」の 1 行
                                                  （sha256sum -c でそのまま検証できる形）
    {arch} は CPU 種別で x64 と arm64（-Arch で選ぶ・既定は x64）。補助 exe と pasta.dll は arm64 の zip でも
    32 ビット（i686）のまま。CI はこの 2 つの名前をこの置き場から拾う。
    完成品の名前の zip と .sha256 は、全段が緑（終了コード 0）のときだけ現れる。途中の段は仮の名前
    （.zip.tmp・.zip.sha256.tmp）で作り、最後の段「完成」で改名する。

  段は名前つきで直列に回す:
    前提の確認 → ターゲット導入 → ビルド（i686 の補助 exe・CPU 種別ごとの本体）→ 静的リンクの確認
    → ライセンス検査 → 謝辞の生成 → 検体の展開
    → CPU 種別ごとに「組み立て → 圧縮 → 中身の判定 → SHA256」
    → （-Check のとき）短いパスへ展開 → 起動 → 番犬 → 記録の判定 → 後片付け
    → git status 不変の確認 → 完成
  どれかの段が失敗したらその段の名前と外部コマンドの出力の末尾を印字し、後始末をしてから即終了する
  （tools/test-all.ps1 の「最後まで回す」とは逆）。後始末は、仮の名前の物を必ず消し、終了コードが 0 で
  ないときは今回改名した完成品も消し、自分が起こした子だけを止め、差し替えた環境変数と PATH を戻す。
  -Check の展開した木は失敗の経路では消さない（調べる手がかりとして残す）。
  段「git status 不変の確認」で、始めと終わりの git status --porcelain が同じことを確かめる
  （追跡しているファイルを 1 つも書き換えない。zip と途中物は追跡外の target/package/ に置く）。

  -Check は（-CheckDir を付けないとき）x64 の zip を target\package\check-<HHmmss> へ展開して areka.exe を有界で起動し、記録を
  target\package\check-<HHmmss>-logs に書いて判定する。窓を出すので CI では付けない。
  判定が合格なら展開した木を消し（-KeepExpanded で残す）、否なら木を残して置き場を印字する。記録はどちらでも残す。
  -CheckDir を付けなければ %TEMP% などの target\ の外には何も作らない（-CheckDir にリポジトリの外を指定したときはそこに作る）。

  注意: 同じ検体で実機を回している最中に実行すると、その走行の展開した木を消す（target/nar-samples/manual/<検体>/）。
  検体の展開は nar-sample-path が同じ manual/<検体>/ を消してから作り直すため。実機の走行が終わってから実行すること。

  終了コード:
    0 … 全段が緑（CPU 種別ごとに zip と .sha256 の絶対パスと版を印字する）
    1 … 段の失敗（段の名前を印字する。展開した木を消せなかった「後片付け」と、git status が変わったときもここ）
    2 … 起動確認（-Check）の記録の判定の否（展開した木と記録を残し、置き場を印字する）
    3 … 引数・前提の不正（版を読めない・道具が無いときも。何を入れるか／どう直すかを 1 行で印字する）

  較正値・調整値の一覧（変更するときはスクリプト冒頭の「較正値」の 1 か所だけを書き換える）:
    SCRIPT_VERSION                  本スクリプトの版（BUILD-INFO.txt の script= に書く）
    OUT_DIR              = target/package  cargo の --target-dir・stage・zip・.sha256・-Check の既定の展開先（check-*）の置き場
    ARCHS                           CPU 種別ごとのビルドのターゲットと機械種別（x64・arm64）
    HELPER_TARGET／HELPER_MACHINE   補助 exe と pasta.dll のターゲットと機械種別（i686・0x014c）
    REMOVE_RETRY = 5／REMOVE_RETRY_WAIT_SEC = 1  後片付けで展開した木を消す再試行の回数と間隔（秒）
    VERSION_PATTERN                 受け付ける版の形（+ の付記は受けない）
    SMOKE_EXIT_MS        = 10000    -Check の有界の自動終了（ミリ秒・-SmokeExitMs で上書き）
    WATCHDOG_MARGIN_SEC  = 60       自動終了の予定から番犬が子を止めるまでの猶予（秒）
    EXPAND_DIR_MAX_CHARS = 160      -Check の展開先のフルパスの長さの上限（文字）
    ARM64_VS_COMPONENT              arm64 のリンクに要る VS の部品名（vswhere -requires に渡す）
    VSWHERE_PATH                    vswhere.exe の固定の置き場（無ければ PATH の vswhere を試す）
    DLL_DENY_PREFIXES               取り込み表に在ってはならない DLL 名の前方一致（大文字小文字を区別しない）
    ALLOWED_EXECUTABLES             zip に入れてよい .exe／.dll の項目名
    LOG_MARKER_*                    -Check の記録の判定に使う目印の文言
    OUTPUT_TAIL_LINES    = 40       段の失敗のときに印字する外部コマンドの出力の末尾の行数

.PARAMETER Arch
  作る CPU 種別。x64・arm64・all（両方）のどれか。既定は x64。
  他の値は何もビルドせずに終了コード 3（受け付ける値を印字する）。
  arm64 を作るには Visual Studio の ARM64 の道具（ARM64_VS_COMPONENT）が要る。無ければ終了コード 3。

.PARAMETER Check
  zip を組んだあと、x64 の zip を展開して areka.exe を有界で起動し、記録を判定する。窓を出すので CI では付けない。
  x64 の zip が要るので -Arch arm64 とは組み合わせられない（終了コード 3）。

.PARAMETER CheckDir
  -Check の展開先 check-<HHmmss> と記録 check-<HHmmss>-logs を作る親フォルダ（絶対パス）。
  省略時は <リポジトリ>\target\package。リポジトリの中なら <リポジトリ>\target\ の下だけ受け付ける。
  展開先のフルパスが EXPAND_DIR_MAX_CHARS を超えると終了コード 3。

.PARAMETER KeepExpanded
  判定の合否にかかわらず、展開した木を消さずに置き場を印字する（-Check のとき）。

.PARAMETER SmokeExitMs
  -Check の有界の自動終了（ミリ秒・正の整数）。省略時は SMOKE_EXIT_MS。正の整数でなければ終了コード 3。

.EXAMPLE
  pwsh -NoProfile -File tools/package.ps1 -Arch all

  CI の形。x64 と arm64 の zip と .sha256 を target/package/ に作る（起動確認はしない）。

.EXAMPLE
  pwsh -NoProfile -File tools/package.ps1 -Check

  x64 の zip を作り、既定の置き場 target\package\check-<HHmmss> へ展開して起動を確かめる。
  合格なら展開した木を消し、記録 target\package\check-<HHmmss>-logs は残す。

.EXAMPLE
  pwsh -NoProfile -File tools/package.ps1 -Check -KeepExpanded

  起動確認の後も展開した木を残す（areka が書いた記憶などを見たいとき）。
#>

# 使い方の説明（上）は Get-Help がファイルの先頭でしか読まないので、#Requires はここに置く
#Requires -Version 7
param(
    # 受け付ける値以外も終了コード 3 で断るため、ValidateSet でなく文字列で受けて自分で読む
    [string]$Arch = 'x64',
    [switch]$Check,
    [string]$CheckDir,
    [switch]$KeepExpanded,
    # 数でない値も終了コード 3 で断るため、文字列で受けて自分で読む
    [string]$SmokeExitMs
)

Set-StrictMode -Version 3.0
$SmokeExitMsGiven = $PSBoundParameters.ContainsKey('SmokeExitMs')
$ErrorActionPreference = 'Stop'
# 外部コマンドの標準エラー出力で例外を投げさせない。終了コードは自前で見る。
$PSNativeCommandUseErrorActionPreference = $false
# 判定に使う子の出力（版・検体のパス）は端末の文字コードを通さず UTF-8 で読む
. (Join-Path $PSScriptRoot 'utf8-child.ps1')

# =============================================================================
# 較正値（説明の一覧と対応。変更はここだけ）
# =============================================================================
$SCRIPT_VERSION       = '2.0.0'
$OUT_DIR              = 'target/package'
# CPU 種別ごとの値の唯一の出どころ（ビルドのターゲット・zip の名前の {arch}・機械種別の検査）。順序つき
$ARCHS = [ordered]@{
    x64   = @{ Target = 'x86_64-pc-windows-msvc'; Machine = 0x8664 }
    arm64 = @{ Target = 'aarch64-pc-windows-msvc'; Machine = 0xAA64 }
}
# 補助 exe と pasta.dll は CPU 種別に依らず 32 ビット
$HELPER_TARGET        = 'i686-pc-windows-msvc'
$HELPER_MACHINE       = 0x014c
$REMOVE_RETRY         = 5
$REMOVE_RETRY_WAIT_SEC = 1
# 版の形（+ のビルドの付記はファイル名と URL で困るので受けない）
$VERSION_PATTERN      = '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$'
$SMOKE_EXIT_MS        = 10000   # 2026-09-26 実測: 挨拶はゲートから約 2.1 秒（release・展開直後の初回起動）。余裕 5 倍
$WATCHDOG_MARGIN_SEC  = 60
$EXPAND_DIR_MAX_CHARS = 160
# arm64 のリンクに要る VS の部品と、それを問う vswhere.exe の固定の置き場（VS Installer 同梱・PATH には載らない）
$ARM64_VS_COMPONENT   = 'Microsoft.VisualStudio.Component.VC.Tools.ARM64'
$VSWHERE_PATH         = "$([Environment]::GetFolderPath('ProgramFilesX86'))\Microsoft Visual Studio\Installer\vswhere.exe"
$DLL_DENY_PREFIXES    = @('vcruntime', 'msvcp', 'msvcr', 'api-ms-win-crt-', 'ucrtbase', 'concrt')
$ALLOWED_EXECUTABLES  = @('areka.exe', 'shiori-host32-helper.exe', 'ghost/emo2/ghost/master/pasta.dll')
$LOG_MARKER_SMOKE_GATE     = 'smoke 自動 close ゲート有効'          # crates/areka/src/main.rs の SMOKE_EXIT_ENV の info!
$LOG_MARKER_WINDOWS        = '本物のゴースト窓を開きました'          # crates/areka/src/ghost_session.rs
$LOG_MARKER_FAULTS         = @('SHIORI が動かなくなりました', 'event="connect_failed"', 'event="helper_exited"')
$LOG_MARKER_GREETING       = '起動グリーティングを再生起動'          # areka-kanade/src/schedule/boot.rs（204 側の文言は数えない）
$LOG_MARKER_SMOKE_EXIT     = 'smoke 自動 close: ゴースト窓を despawn しました'  # crates/areka/src/main.rs の有界の自動終了が発火した行（quit_app の直後）
$LOG_MARKER_BALLOON        = 'バルーンを決めました'                  # crates/areka/src/boot_config.rs の balloon_resolved
$LOG_MARKER_BALLOON_ROUTE  = 'route=Companion'
$LOG_MARKER_BALLOON_DIR    = '\balloon\emo2-kakukaku'
$OUTPUT_TAIL_LINES    = 40

$EXIT_OK = 0; $EXIT_STAGE_FAILED = 1; $EXIT_CHECK_FAILED = 2; $EXIT_BAD_ARGS = 3

Set-Location (Split-Path $PSScriptRoot -Parent)

# Git Bash から呼ばれると GNU coreutils の link.exe が MSVC の link.exe を遮蔽し i686 のリンクが落ちる。
# link.exe を持つが cl.exe を持たないフォルダ（＝MSVC 以外の link.exe）を、このプロセスの PATH から外す。
# （tools/test-all.ps1 と同じ）。& で呼ばれても親に残さないよう、後始末で元へ戻す。
$script:OrigPath = $env:PATH
$script:Child = $null
$env:PATH =($env:PATH -split ';' | Where-Object {
        -not $_ -or -not (Test-Path (Join-Path $_ 'link.exe')) -or (Test-Path (Join-Path $_ 'cl.exe'))
    }) -join ';'

# =============================================================================
# 後始末
# =============================================================================
$script:TmpFiles = [Collections.Generic.List[string]]::new()   # 圧縮と SHA256 の段が CPU 種別ごとの .zip.tmp・.sha256.tmp のパスを入れる
$script:SavedEnv = @{}       # Set-EnvTemp が差し替える前の値（$null は「無かった」）

# 環境変数をこのプロセスで差し替える。元の値は最初の 1 回だけ覚え、後始末で戻す。
function Set-EnvTemp([string]$Name, [AllowNull()][string]$Value) {
    if (-not $script:SavedEnv.ContainsKey($Name)) {
        $script:SavedEnv[$Name] = [Environment]::GetEnvironmentVariable($Name)
    }
    Set-EnvValue $Name $Value
}

# $null（と空）は変数を消す。[Environment]::SetEnvironmentVariable に $null を渡すと PowerShell が '' に
# 変えて「空の値を持つ変数」が残る（CARGO_ENCODED_RUSTFLAGS が空で残ると cargo は RUSTFLAGS を無視する）
function Set-EnvValue([string]$Name, [AllowNull()][string]$Value) {
    if ($Value) { Set-Item -LiteralPath "Env:$Name" -Value $Value }
    else { Remove-Item -LiteralPath "Env:$Name" -ErrorAction SilentlyContinue }
}

function Restore-Env {
    foreach ($name in @($script:SavedEnv.Keys)) {
        Set-EnvValue $name $script:SavedEnv[$name]
    }
    $script:SavedEnv.Clear()
}

$script:Finalized = [Collections.Generic.List[string]]::new()  # 段「完成」が改名の済んだ zip・.sha256 から順に入れる

# 仮の名前の物は常に、今回の完成品は終了コードが 0 でないときだけ消す（完成品が在る ⇔ 終了コード 0）
function Invoke-Cleanup([int]$Code) {
    $remove = @($script:TmpFiles) + ($Code ? @($script:Finalized) : @())
    foreach ($p in $remove) {
        if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force }
    }
    # 起動から番犬までの段で落ちたときに、自分が起こした子だけを止める（名前では探さない）
    if ($script:Child -and -not $script:Child.HasExited) { $script:Child.Kill() }
    Restore-Env
    $env:PATH = $script:OrigPath
}

function Exit-Script([int]$Code, [string]$Message) {
    Invoke-Cleanup $Code
    if ($Message) { Write-Host $Message -ForegroundColor Red }
    exit $Code
}

# CPU 種別 1 つ分の成果物の 4 つの絶対パス（名前の決め方はここだけ。版は前提の確認で読んだ $script:Version）
function Get-ArtifactNames([string]$Arch) {
    $zip = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../$OUT_DIR/areka-$script:Version-$Arch.zip"))
    [pscustomobject]@{ Zip = $zip; Sha = "$zip.sha256"; ZipTmp = "$zip.tmp"; ShaTmp = "$zip.sha256.tmp" }
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
    $accepted = @($ARCHS.Keys) + 'all'
    if ($accepted -cnotcontains $Arch) {
        Exit-Script $EXIT_BAD_ARGS ("-Arch は {0} のどれかで指定する（受け取った値: '{1}'）" -f ($accepted -join '・'), $Arch)
    }
    $script:BuildArchs = ($Arch -eq 'all') ? @($ARCHS.Keys) : @($Arch)
    if ($Check -and $script:BuildArchs -cnotcontains 'x64') {
        Exit-Script $EXIT_BAD_ARGS "-Check の起動確認には x64 の zip が要る（-Arch x64 か all と組み合わせる・受け取った値: '$Arch'）"
    }
    if ($CheckDir -and -not [IO.Path]::IsPathFullyQualified($CheckDir)) {
        Exit-Script $EXIT_BAD_ARGS "-CheckDir は絶対パスで指定する（受け取った値: '$CheckDir'）"
    }
    # リポジトリの中なら target\ の下だけ（一時フォルダは target\ の下に限る）。外は受け付ける
    $repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\') + '\'
    if ($CheckDir) {
        $full = [IO.Path]::GetFullPath($CheckDir).TrimEnd('\') + '\'
        if ($full.StartsWith($repoRoot, [StringComparison]::OrdinalIgnoreCase) -and
            -not $full.StartsWith("${repoRoot}target\", [StringComparison]::OrdinalIgnoreCase)) {
            Exit-Script $EXIT_BAD_ARGS "-CheckDir はリポジトリの中なら ${repoRoot}target\ の下を指定する（受け取った値: '$CheckDir'）"
        }
    }
    # 既定の親はワークツリーの target\package（%TEMP% には何も作らない）
    $parent = if ($CheckDir) { [IO.Path]::GetFullPath($CheckDir) } else { [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../$OUT_DIR")) }
    $script:ExpandDir = Join-Path $parent ('check-' + (Get-Date -Format 'HHmmss'))
    $script:LogDir = "$script:ExpandDir-logs"
    if (($Check -or $CheckDir) -and $script:ExpandDir.Length -gt $EXPAND_DIR_MAX_CHARS) {
        Exit-Script $EXIT_BAD_ARGS ("展開先が長すぎる（{0} 文字・上限 {1}）: {2} — -CheckDir に短いパス（リポジトリの外か ${repoRoot}target\ の下）を指定するか、ワークツリーを短いパスに置く" -f $script:ExpandDir.Length, $EXPAND_DIR_MAX_CHARS, $script:ExpandDir)
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
    # arm64 のリンクに要る VS の部品（rustup のターゲットは欠けても後の段で足すので、ここでは見ない）
    if ($script:BuildArchs -ccontains 'arm64') {
        # vswhere.exe は PATH に載らないので固定の置き場を先に見る
        $vswhere = if (Test-Path -LiteralPath $VSWHERE_PATH -PathType Leaf) { $VSWHERE_PATH }
                   else { Get-Command vswhere -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source }
        if (-not $vswhere) { Exit-Script $EXIT_BAD_ARGS "vswhere.exe が無い（探した場所: $VSWHERE_PATH と PATH。Visual Studio か Build Tools を入れる）" }
        $vs = & $vswhere -latest -products '*' -requires $ARM64_VS_COMPONENT -property installationPath 2>$null
        if ($LASTEXITCODE -or -not $vs) { Exit-Script $EXIT_BAD_ARGS "arm64 のリンクに要る VS の部品が無い（VS Installer で $ARM64_VS_COMPONENT を追加する・vswhere: $vswhere）" }
    }

    # 版（正本は Cargo.toml の [workspace.package] version。areka パッケージ経由で読む）。3 で止まる検査の最後
    $r = Invoke-Utf8Child cargo @('metadata', '--no-deps', '--locked', '--format-version', '1')
    if ($r.Code) {
        Exit-Script $EXIT_BAD_ARGS ('cannot read the version (cargo metadata exited with code {0}: {1})' -f $r.Code, ($r.ErrLines | Select-Object -Last 1))
    }
    try {
        $meta = $r.Out | ConvertFrom-Json
    } catch {
        Exit-Script $EXIT_BAD_ARGS "版を読めない（cargo metadata の出力が JSON として読めない: $_）"
    }
    $pkg = @($meta.packages | Where-Object { $_.name -ceq 'areka' })
    if ($pkg.Count -ne 1) { Exit-Script $EXIT_BAD_ARGS "版を読めない（cargo metadata に areka のパッケージが $($pkg.Count) 件）" }
    $script:Version = "$($pkg[0].version)"
    if (-not $script:Version) { Exit-Script $EXIT_BAD_ARGS '版を読めない（areka の version が空。Cargo.toml の [workspace.package] version を書く）' }
    if ($script:Version -cnotmatch $VERSION_PATTERN) {
        Exit-Script $EXIT_BAD_ARGS "版の形が違う（'$script:Version'・受け付ける形 $VERSION_PATTERN。+ の付記は付けない）"
    }

    # 前回の同名の組をビルドの前に消す（今回が失敗しても前回の物が完成品に見えて残らない）
    foreach ($a in $script:BuildArchs) {
        $n = Get-ArtifactNames $a
        foreach ($p in @($n.Zip, $n.Sha, $n.ZipTmp, $n.ShaTmp)) {
            if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force; Write-Host "前回の物を消した: $p" }
        }
    }

    Write-Host "コミット $script:Commit・未コミットの変更 $script:Dirty 件・版 $script:Version・CPU 種別 $($script:BuildArchs -join '・')"
}

$I686 = $HELPER_TARGET
$script:HelperExe = "$OUT_DIR/$I686/release/shiori-host32-helper.exe"
$script:Notices = "$OUT_DIR/THIRD-PARTY-NOTICES.md"
# 開発者のシェルの値に継ぎ足さない（-C target-cpu=native 等が混ざると開発機でしか動かない exe になる）
$script:RustFlags = '-C target-feature=+crt-static'

# PE のバイト列から機種と取り込み表の DLL 名を読む（PE32 と PE32+ の両方）。
# 読めない形なら throw。遅延読み込みの表は読まない（拒否表の DLL を遅延で読むことは無い前提）。
function Read-PeInfo([byte[]]$Bytes) {
    $u16 = { param($o) [BitConverter]::ToUInt16($Bytes, $o) }
    $u32 = { param($o) [BitConverter]::ToUInt32($Bytes, $o) }
    $pe = & $u32 0x3c
    if ((& $u32 $pe) -ne 0x4550) { throw 'PE の署名が無い' }
    $machine = & $u16 ($pe + 4)
    $sections = & $u16 ($pe + 6)
    $opt = $pe + 24
    $magic = & $u16 $opt
    $dirs = switch ($magic) { 0x10b { $opt + 96 } 0x20b { $opt + 112 } default { throw ('知らない optional header の magic 0x{0:x}' -f $magic) } }
    $importRva = & $u32 ($dirs + 8)   # データディレクトリの 1 番＝import
    $secTable = $opt + (& $u16 ($pe + 20))
    $toOffset = {
        param($rva)
        for ($i = 0; $i -lt $sections; $i++) {
            $s = $secTable + 40 * $i
            $va = & $u32 ($s + 12); $size = [Math]::Max((& $u32 ($s + 8)), (& $u32 ($s + 16)))
            if ($rva -ge $va -and $rva -lt $va + $size) { return $rva - $va + (& $u32 ($s + 20)) }
        }
        throw ('RVA 0x{0:x} がどの節にも無い' -f $rva)
    }
    $names = [Collections.Generic.List[string]]::new()
    if ($importRva) {
        for ($d = & $toOffset $importRva; ($nameRva = & $u32 ($d + 12)) -ne 0; $d += 20) {
            $n = & $toOffset $nameRva; $end = [Array]::IndexOf($Bytes, [byte]0, $n)
            $names.Add([Text.Encoding]::ASCII.GetString($Bytes, $n, $end - $n))
        }
    }
    [pscustomobject]@{ Machine = $machine; Imports = $names.ToArray() }
}

# 取り込み表の名前のうち拒否表（前方一致・大文字小文字を区別しない）に当たるもの（呼ぶ側は @() で包む）
function Get-DeniedImports([string[]]$Imports) {
    $Imports | Where-Object { $n = $_; $DLL_DENY_PREFIXES | Where-Object { $n.StartsWith($_, [StringComparison]::OrdinalIgnoreCase) } }
}

Step 'i686 ターゲット導入' { rustup target add $I686 }
# arm64 のターゲットは作るときだけ足す（-Arch x64 では道具の無い機械でも止まらない）
if ($script:BuildArchs -ccontains 'arm64') {
    Step 'arm64 ターゲット導入' { rustup target add $ARCHS['arm64'].Target }
}

# RUSTFLAGS を置き換え、CARGO_ENCODED_RUSTFLAGS・CARGO_BUILD_RUSTFLAGS を外す（在ると cargo が RUSTFLAGS を無視する）。
# 差し替えはビルドの段の間だけ。失敗時は Exit-Script の後始末が戻す。
Set-EnvTemp 'RUSTFLAGS' $script:RustFlags
Set-EnvTemp 'CARGO_ENCODED_RUSTFLAGS' $null
Set-EnvTemp 'CARGO_BUILD_RUSTFLAGS' $null
Step 'i686 helper ビルド' { cargo build --locked --release -p shiori-host32-helper --target $I686 --target-dir $OUT_DIR }
foreach ($a in $script:BuildArchs) {
    $t = $ARCHS[$a].Target
    Step "$a 本体ビルド" { cargo build --locked --release -p areka --target $t --target-dir $OUT_DIR }
}
Restore-Env

Step '静的リンクの確認' {
    # 作った exe 全部（helper＋作る CPU 種別の本体）と、期待する機械種別
    $exes = [ordered]@{ $script:HelperExe = $HELPER_MACHINE }
    foreach ($a in $script:BuildArchs) { $exes["$OUT_DIR/$($ARCHS[$a].Target)/release/areka.exe"] = $ARCHS[$a].Machine }
    $bad = $false
    foreach ($exe in $exes.Keys) {
        if (-not (Test-Path -LiteralPath $exe)) { throw "ビルドの出力が無い: $exe" }
        $info = Read-PeInfo ([IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $exe)))
        $denied = @(Get-DeniedImports $info.Imports)
        Write-Host ('{0}: 機種 0x{1:x4}（期待 0x{2:x4}）・取り込み {3}' -f $exe, $info.Machine, $exes[$exe], ($info.Imports -join ', '))
        if ($info.Machine -ne $exes[$exe]) { Write-Host '  機種が違う'; $bad = $true }
        if ($denied.Count) { Write-Host "  拒否表に当たる: $($denied -join ', ')"; $bad = $true }
    }
    if ($bad) { throw '機種が違うか、VC++ ランタイムの DLL を読んでいる（+crt-static が効いていない）' }
}

Step 'ライセンス検査' { cargo deny --locked check licenses }

# -Arch に依らず全ターゲット（ARCHS の行＋helper）で 1 回。どの CPU 種別の zip にも同じ物を入れる
Step '謝辞の生成' {
    if (Test-Path -LiteralPath $script:Notices) { Remove-Item -LiteralPath $script:Notices -Force }
    $targets = @($ARCHS.Values | ForEach-Object Target) + $HELPER_TARGET | ForEach-Object { '--target', $_ }
    cargo about generate --locked --workspace @targets about.hbs -o $script:Notices
    if ($LASTEXITCODE) { return }
    if (-not (Test-Path -LiteralPath $script:Notices)) { throw "謝辞の出力が無い: $script:Notices" }
}

# nar-sample-path の出力（1 行 1 組の key=value）から $Keys の値を読む。鍵が無い・パスが実在しなければ throw。
function Read-SamplePaths([string]$Sample, [string[]]$Keys) {
    $r = Invoke-Utf8Child cargo @('run', '-q', '--locked', '-p', 'sample-ghost-kit', '--bin', 'nar-sample-path', '--', $Sample)
    # 子の標準エラーは書き替えずにそのまま写す（Step の出力の末尾には入らないので、失敗の文に最後の行を添える）
    foreach ($e in $r.ErrLines) { Write-Host $e }
    if ($r.Code) { throw "nar-sample-path $Sample exited with code $($r.Code): $($r.ErrLines | Select-Object -Last 1)" }
    $lines = @($r.Out -split '\r?\n')
    $paths = @{}
    foreach ($key in $Keys) {
        $line = $lines | Where-Object { $_.StartsWith("$key=") } | Select-Object -First 1
        if (-not $line) { throw "nar-sample-path $Sample output has no $key=" }
        $path = $line.Substring($key.Length + 1)
        if (-not (Test-Path -LiteralPath $path -PathType Container)) { throw "nar-sample-path $Sample ${key}= does not exist: $path" }
        $paths[$key] = $path
    }
    $paths
}

Step '検体の展開' {
    $emo2 = Read-SamplePaths 'emo2' @('folder', 'balloon.emo2-kakukaku')
    $script:GhostDir = $emo2['folder']
    $script:KakukakuDir = $emo2['balloon.emo2-kakukaku']
    $script:StayseeDir = (Read-SamplePaths 'StayseeBalloon' @('folder'))['folder']
}

# zip の項目名（区切りを / に揃える・大文字小文字を区別する）→ 項目。呼ぶ側が .Zip を Dispose する。
function Get-ZipEntryMap([string]$Path) {
    $zip = [IO.Compression.ZipFile]::OpenRead($Path)
    $map = [hashtable]::new([StringComparer]::Ordinal)
    foreach ($e in $zip.Entries) { $map[$e.FullName.Replace('\', '/')] = $e }
    [pscustomobject]@{ Zip = $zip; Map = $map }
}

function Read-ZipEntry($Entry) {
    $ms = [IO.MemoryStream]::new(); $s = $Entry.Open()
    try { $s.CopyTo($ms) } finally { $s.Dispose() }
    , $ms.ToArray()
}

# zip を読み戻して設計の「zip の中身の判定」1〜8 を全部行い、否の項目の文を全部返す（空なら合）。
# $Arch は ARCHS のキー（本体の機械種別と BUILD-INFO.txt の arch= の期待値）。
function Test-ZipContent([string]$ZipPath, [string]$Arch) {
    $bad = [Collections.Generic.List[string]]::new()
    $same = { param([byte[]]$a, [byte[]]$b) [Linq.Enumerable]::SequenceEqual($a, $b) }
    $z = Get-ZipEntryMap $ZipPath
    try {
        $map = $z.Map
        $names = @($map.Keys | Sort-Object)

        # 1. 必須の項目
        foreach ($r in @('areka.exe', 'shiori-host32-helper.exe', 'ghost/emo2/ghost/master/descript.txt', 'ghost/emo2/install.txt',
                'balloon/emo2-kakukaku/descript.txt', 'balloon/StayseeBalloon/descript.txt',
                'README.txt', 'LICENSE-MIT', 'THIRD-PARTY-NOTICES.md', 'BUILD-INFO.txt')) {
            if (-not $map.ContainsKey($r)) { $bad.Add("1 必須の項目が無い: $r") }
        }
        # 2. 最上位・ghost/ 直下・balloon/ 直下の許可表
        $allowed = @{
            ''         = @('areka.exe', 'shiori-host32-helper.exe', 'ghost', 'balloon', 'README.txt', 'LICENSE-MIT', 'THIRD-PARTY-NOTICES.md', 'BUILD-INFO.txt')
            'ghost/'   = @('emo2')
            'balloon/' = @('emo2-kakukaku', 'StayseeBalloon')
        }
        foreach ($prefix in $allowed.Keys) {
            $kids = $names | Where-Object { $_.StartsWith($prefix, [StringComparison]::Ordinal) } |
                ForEach-Object { $_.Substring($prefix.Length).Split('/')[0] } | Where-Object { $_ } | Sort-Object -Unique
            $kids | Where-Object { $allowed[$prefix] -cnotcontains $_ } | ForEach-Object { $bad.Add("2 許可表に無い項目: $prefix$_") }
        }
        # 3. 起動記録
        $prof = @($names | Where-Object { $_ -match '(^|/)profile/' })
        if ($prof.Count) { $bad.Add("3 profile/ を含む項目が $($prof.Count) 件（最初: $($prof[0])）") }
        # 4. 実行ファイルと DLL は許可表の 3 本だけ
        $execs = @($names | Where-Object { $_ -match '\.(exe|dll)$' })
        $execs | Where-Object { $ALLOWED_EXECUTABLES -cnotcontains $_ } | ForEach-Object { $bad.Add("4 許可表に無い実行ファイル: $_") }
        $ALLOWED_EXECUTABLES | Where-Object { $execs -cnotcontains $_ } | ForEach-Object { $bad.Add("4 実行ファイルが無い: $_") }
        # 5. PE の機種・6. 依存 DLL（本体と helper だけ）
        $machines = [ordered]@{ 'areka.exe' = $ARCHS[$Arch].Machine; 'shiori-host32-helper.exe' = $HELPER_MACHINE; 'ghost/emo2/ghost/master/pasta.dll' = $HELPER_MACHINE }
        foreach ($n in $machines.Keys) {
            if (-not $map.ContainsKey($n)) { continue }   # 無いことは 1／4 が言う
            try { $info = Read-PeInfo (Read-ZipEntry $map[$n]) } catch { $bad.Add("5 PE として読めない: $n（$_）"); continue }
            Write-Host ('{0}: 機種 0x{1:x4}・取り込み {2}' -f $n, $info.Machine, ($info.Imports -join ', '))
            if ($info.Machine -ne $machines[$n]) { $bad.Add(('5 機種が違う: {0} は 0x{1:x4}（期待 0x{2:x4}）' -f $n, $info.Machine, $machines[$n])) }
            if ($n -like '*.exe') {
                $denied = @(Get-DeniedImports $info.Imports)
                if ($denied.Count) { $bad.Add("6 拒否表の DLL を読む: $n → $($denied -join ', ')") }
            }
        }
        # 7. 説明書 2 本は emo2.nar の中身とバイトが同じ
        $nar = Get-ZipEntryMap (Resolve-Path -LiteralPath 'vendors/sample_ghost/emo2.nar')
        try {
            foreach ($pair in @(@('ghost/emo2/readme.txt', 'readme.txt'), @('ghost/emo2/shell/master/readme.txt', 'shell/master/readme.txt'))) {
                if (-not $map.ContainsKey($pair[0])) { $bad.Add("7 説明書が無い: $($pair[0])"); continue }
                if (-not $nar.Map.ContainsKey($pair[1])) { $bad.Add("7 emo2.nar に $($pair[1]) が無い"); continue }
                if (-not (& $same (Read-ZipEntry $map[$pair[0]]) (Read-ZipEntry $nar.Map[$pair[1]]))) { $bad.Add("7 emo2.nar の $($pair[1]) とバイトが違う: $($pair[0])") }
            }
        } finally { $nar.Zip.Dispose() }
        # 8. README・ライセンス・謝辞は写す元とバイトが同じ・BUILD-INFO.txt の値
        foreach ($pair in @(@('README.txt', 'dist/README.txt'), @('LICENSE-MIT', 'LICENSE-MIT'), @('THIRD-PARTY-NOTICES.md', $script:Notices))) {
            if (-not $map.ContainsKey($pair[0])) { continue }   # 無いことは 1 が言う
            if (-not (& $same (Read-ZipEntry $map[$pair[0]]) ([IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $pair[1]))))) { $bad.Add("8 $($pair[1]) とバイトが違う: $($pair[0])") }
        }
        if ($map.ContainsKey('BUILD-INFO.txt')) {
            $info = [Text.Encoding]::UTF8.GetString((Read-ZipEntry $map['BUILD-INFO.txt'])) -split "`r?`n"
            foreach ($want in @("version=$script:Version", "arch=$Arch", "commit=$script:Commit", "dirty=$script:Dirty")) {
                if ($info -cnotcontains $want) { $bad.Add("8 BUILD-INFO.txt に $want が無い") }
            }
        }
    } finally { $z.Zip.Dispose() }
    , $bad.ToArray()
}

foreach ($a in $script:BuildArchs) {
    $stage = "$OUT_DIR/stage-$a"
    Step "$a 組み立て" {
        if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
        $null = New-Item -ItemType Directory -Path "$stage/ghost", "$stage/balloon"
        # 設計の「zip の中身」の 9 行だけ（写す元が無ければ Copy-Item が throw）。areka.exe だけ CPU 種別ごと
        Copy-Item -LiteralPath "$OUT_DIR/$($ARCHS[$a].Target)/release/areka.exe" -Destination $stage
        Copy-Item -LiteralPath $script:HelperExe -Destination $stage
        Copy-Item -LiteralPath $script:GhostDir -Destination "$stage/ghost/emo2" -Recurse
        Copy-Item -LiteralPath $script:KakukakuDir -Destination "$stage/balloon/emo2-kakukaku" -Recurse
        Copy-Item -LiteralPath $script:StayseeDir -Destination "$stage/balloon/StayseeBalloon" -Recurse
        Copy-Item -LiteralPath 'dist/README.txt' -Destination $stage
        Copy-Item -LiteralPath 'LICENSE-MIT' -Destination $stage
        Copy-Item -LiteralPath $script:Notices -Destination $stage
        $info = @(
            "version=$script:Version"
            "arch=$a"
            "commit=$script:Commit"
            "dirty=$script:Dirty"
            "built=$([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))"
            "script=tools/package.ps1 $SCRIPT_VERSION"
            "rustflags=$script:RustFlags"
        )
        [IO.File]::WriteAllLines((Join-Path (Resolve-Path -LiteralPath $stage) 'BUILD-INFO.txt'), $info)   # UTF-8（BOM 無し）
    }

    Step "$a 圧縮" {
        $tmp = (Get-ArtifactNames $a).ZipTmp
        $script:TmpFiles.Add($tmp)
        if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force }
        # includeBaseDirectory を $false にしないと stage-{arch}/ が最上位に入る
        [IO.Compression.ZipFile]::CreateFromDirectory((Resolve-Path -LiteralPath $stage), $tmp, [IO.Compression.CompressionLevel]::Optimal, $false)
        Write-Host $tmp
    }

    Step "$a 中身の判定" {
        $bad = Test-ZipContent (Get-ArtifactNames $a).ZipTmp $a
        if (-not $bad.Count) { Write-Host '判定 1〜8 すべて合'; return }
        $bad | ForEach-Object { Write-Host "否 $_" }
        throw "中身の判定で否が $($bad.Count) 件"
    }

    Step "$a SHA256" {
        $n = Get-ArtifactNames $a
        $script:TmpFiles.Add($n.ShaTmp)
        # Get-FileHash は大文字を返す。ファイル名は完成後の名前（sha256sum -c を完成品の隣でそのまま通す）
        $hash = (Get-FileHash -LiteralPath $n.ZipTmp -Algorithm SHA256).Hash.ToLowerInvariant()
        [IO.File]::WriteAllText($n.ShaTmp, ("{0}  {1}`n" -f $hash, (Split-Path $n.Zip -Leaf)))   # UTF-8（BOM 無し）・LF
        Write-Host $n.ShaTmp
    }
}

# 記録の判定（完了 alpha-package の要件 3.3・3.5）。2 つの記録を連結した文字列・子の終了コード・番犬で止めたかを受け、
# 条件ごとに { Name; Ok; Detail } を返す。全部見てから呼び手が 1 回主張する（1 つ目で止めない）。
function Test-RunLog([AllowEmptyString()][string]$Text, [int]$ExitCode, [bool]$WatchdogKilled) {
    $lines = $Text -split "`r?`n"
    $count = { param($m) @($lines | Where-Object { $_.Contains($m) }).Count }
    $row = { param($name, $ok, $detail) [pscustomobject]@{ Name = $name; Ok = [bool]$ok; Detail = $detail } }
    $gate = & $count $LOG_MARKER_SMOKE_GATE
    & $row '番犬で止めていない' (-not $WatchdogKilled) ($WatchdogKilled ? '番犬で止めた' : '自分で終わった')
    & $row '有界で走った' ($gate -gt 0) "「$LOG_MARKER_SMOKE_GATE」$gate 件"
    & $row '終了コード 0' ($ExitCode -eq 0) "終了コード $ExitCode"
    $win = & $count $LOG_MARKER_WINDOWS
    & $row 'ゴーストの窓が立った' ($win -gt 0) "「$LOG_MARKER_WINDOWS」$win 件"
    $faults = @($LOG_MARKER_FAULTS | ForEach-Object { $n = & $count $_; if ($n) { "「$_」$n 件" } })
    & $row 'SHIORI の接続の失敗が無い' (-not $faults.Count) ($faults.Count ? ($faults -join '・') : '失敗の目印 0 件')
    # 204 側の「epilogue-only 起動記録トーク…」は同じ event="boot_talk" だが文言が違うので数えない
    # 自動終了より後の挨拶は数えない（完了 alpha-package の要件 3.3 の空振り）。両方とも run.log に出るので行の位置で比べる。
    # 自動終了の目印が無ければ全行を見る（そのときは「有界で走った」側か終了コードで落ちる）
    $exitAt = [array]::FindIndex([string[]]$lines, [Predicate[string]]{ param($l) $l.Contains($LOG_MARKER_SMOKE_EXIT) })
    $before = ($exitAt -ge 0) ? @($lines | Select-Object -First $exitAt) : $lines
    $greet = @($before | Where-Object { $_.Contains($LOG_MARKER_GREETING) }).Count
    $all = & $count $LOG_MARKER_GREETING
    & $row '会話が始まった' ($greet -gt 0) "「$LOG_MARKER_GREETING」$greet 件（自動終了より前）・全体 $all 件"
    # 初回のバルーン: 最初の「バルーンを決めました」の行。dir= は行末までの値（パスに空白が入り得る）
    $b = @($lines | Where-Object { $_.Contains($LOG_MARKER_BALLOON) }) | Select-Object -First 1
    if (-not $b) {
        & $row '初回のバルーンは同梱' $false "「$LOG_MARKER_BALLOON」の行が無い"
    } else {
        $dir = ($b -match 'dir=(.*)$') ? $Matches[1].Trim() : ''
        $ok = $b.Contains($LOG_MARKER_BALLOON_ROUTE) -and $dir.EndsWith($LOG_MARKER_BALLOON_DIR, [StringComparison]::OrdinalIgnoreCase)
        & $row '初回のバルーンは同梱' $ok ($b.Substring($b.IndexOf($LOG_MARKER_BALLOON)))
    }
}

$script:WatchdogKilled = $false   # 番犬が子を止めたか（記録の判定の合否の一覧に畳み込む）
if ($Check) {
    Step '短いパスへ展開' {
        # 展開先の長さは「前提の確認」で確かめ済み
        if (Test-Path -LiteralPath $script:ExpandDir) { throw "展開先が既に在る: $script:ExpandDir" }
        if (Test-Path -LiteralPath $script:LogDir) { throw "記録の置き場が既に在る: $script:LogDir" }
        # 完成の改名より前なので x64 の仮の名前の zip を展開する（.tmp のままでも zip として読める）
        [IO.Compression.ZipFile]::ExtractToDirectory((Get-ArtifactNames 'x64').ZipTmp, $script:ExpandDir)
        if ($script:BuildArchs -ccontains 'arm64') { Write-Host 'arm64 の zip は作ったが起動確認はしていない（起動確認は x64 の zip だけ）' }
        $null = New-Item -ItemType Directory -Path $script:LogDir
        $script:RunLog = Join-Path $script:LogDir 'run.log'
        $script:RunErrLog = Join-Path $script:LogDir 'run.stderr.log'
        Write-Host "展開先: $script:ExpandDir"
    }

    Step '起動' {
        # AREKA_*／WINTF_* を全部外し、決めた 4 つだけ入れる。子へは継承で渡し、起こした直後に戻す。
        Get-ChildItem Env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } |
            ForEach-Object { Set-EnvTemp $_.Name $null }
        Set-EnvTemp 'AREKA_APP_SMOKE_EXIT_MS' "$script:SmokeMs"   # crates/areka/src/main.rs の SMOKE_EXIT_ENV
        Set-EnvTemp 'AREKA_NO_ALERT' '1'                          # crates/areka/src/alert.rs の NO_ALERT_ENV
        Set-EnvTemp 'RUST_LOG' 'info'
        Set-EnvTemp 'NO_COLOR' '1'
        try {
            $script:Child = Start-Process -FilePath (Join-Path $script:ExpandDir 'areka.exe') -WorkingDirectory $script:ExpandDir `
                -RedirectStandardOutput $script:RunLog -RedirectStandardError $script:RunErrLog -NoNewWindow -PassThru
        } finally { Restore-Env }
        $null = $script:Child.Handle   # 取っ手を先に掴まないと終了後に ExitCode が読めない
        Write-Host "子のプロセス番号: $($script:Child.Id)"
    }

    Step '番犬' {
        $limitMs = $script:SmokeMs + $WATCHDOG_MARGIN_SEC * 1000
        if (-not $script:Child.WaitForExit($limitMs)) {
            $script:WatchdogKilled = $true
            Write-Host "番犬: $limitMs ミリ秒を超えたので自分が起こした子（プロセス番号 $($script:Child.Id)）だけを止めた"
            $script:Child.Kill()
            $null = $script:Child.WaitForExit(10000)
        }
        Write-Host "子の終了コード: $($script:Child.ExitCode)"
        # 番犬で止めたことは次の段「記録の判定」の合否の一覧に畳み込む（ここでは止めない）
    }

    Step '記録の判定' {
        $text = (Get-Content -LiteralPath $script:RunLog -Raw) + "`n" + (Get-Content -LiteralPath $script:RunErrLog -Raw)
        $rows = Test-RunLog $text $script:Child.ExitCode $script:WatchdogKilled
        $rows | ForEach-Object { Write-Host ('{0} {1}（{2}）' -f ($_.Ok ? '合' : '否'), $_.Name, $_.Detail) }
        Write-Host "記録: $script:RunLog"
        Write-Host "記録: $script:RunErrLog"
        Write-Host "展開先: $script:ExpandDir"
        $failed = @($rows | Where-Object { -not $_.Ok })
        if ($failed.Count) {
            Exit-Script $EXIT_CHECK_FAILED ("起動確認の否（{0}）" -f (($failed | ForEach-Object Name) -join '・'))
        }
    }

    # 判定の合格の後にだけ来る（否は上で 2 で終わり、木と記録を残す）。番犬の後なので自分の子は終わっている。
    # 掴みが残る間（本体の終了に道連れの補助 exe など）は再試行で待つ。プロセスは止めない。
    Step '後片付け' {
        if ($KeepExpanded) {
            Write-Host "展開した木を残した（-KeepExpanded）: $script:ExpandDir"
        } else {
            $last = $null
            for ($i = 1; $i -le $REMOVE_RETRY; $i++) {
                try { Remove-Item -LiteralPath $script:ExpandDir -Recurse -Force; $last = $null; break }
                catch { $last = $_; if ($i -lt $REMOVE_RETRY) { Start-Sleep -Seconds $REMOVE_RETRY_WAIT_SEC } }
            }
            if ($last) { throw "展開した木を消せなかった（段「後片付け」・$REMOVE_RETRY 回試した）: $script:ExpandDir — $($last.Exception.Message)" }
            Write-Host "展開した木を消した: $script:ExpandDir"
        }
        Write-Host "記録（残す）: $script:LogDir"
    }
}

Step 'git status 不変の確認' {
    $after = @(git status --porcelain)
    if ($LASTEXITCODE) { throw 'git status が失敗した' }
    if (($script:StatusBefore -join "`n") -ceq ($after -join "`n")) { return }
    $script:StatusBefore | Where-Object { $after -cnotcontains $_ } | ForEach-Object { Write-Host "消えた $_" }
    $after | Where-Object { $script:StatusBefore -cnotcontains $_ } | ForEach-Object { Write-Host "増えた $_" }
    throw 'git status --porcelain が始めと違う'
}

# 起動確認が仮の名前の zip を展開するので、改名は起動確認と git status の確認の後
Step '完成' {
    foreach ($a in $script:BuildArchs) {
        $n = Get-ArtifactNames $a
        # 1 本ずつ改名の直後に登録する（途中で落ちたら済んだ分だけを後始末が消す）
        Move-Item -LiteralPath $n.ZipTmp -Destination $n.Zip -Force
        $script:Finalized.Add($n.Zip)
        Move-Item -LiteralPath $n.ShaTmp -Destination $n.Sha -Force
        $script:Finalized.Add($n.Sha)
    }
    # 印字は全組の改名が済んでから（途中で落ちたら完成品の名前を 1 つも印字しない）
    foreach ($a in $script:BuildArchs) {
        $n = Get-ArtifactNames $a
        Write-Host "zip: $($n.Zip)"
        Write-Host "sha256: $($n.Sha)"
        Write-Host "version: $script:Version"
    }
    Write-Host "コミット $script:Commit・未コミットの変更 $script:Dirty 件"
}

Invoke-Cleanup $EXIT_OK
Write-Host "`n全段 緑" -ForegroundColor Green
exit $EXIT_OK
