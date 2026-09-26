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
    SMOKE_EXIT_MS        = 10000    -Check の有界の自動終了（ミリ秒・-SmokeExitMs で上書き）
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
  pwsh -NoProfile -File tools/package-alpha.ps1 -Check -CheckDir C:\t -SmokeExitMs 10000
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
$SMOKE_EXIT_MS        = 10000   # 2026-09-26 実測: 挨拶はゲートから約 2.1 秒（release・展開直後の初回起動）。余裕 5 倍
$WATCHDOG_MARGIN_SEC  = 60
$EXPAND_DIR_MAX_CHARS = 160
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
$script:ZipTmp = $null       # 圧縮の段が .zip.tmp のパスを入れる
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

function Invoke-Cleanup {
    if ($script:ZipTmp -and (Test-Path -LiteralPath $script:ZipTmp)) {
        Remove-Item -LiteralPath $script:ZipTmp -Force
    }
    # 起動から番犬までの段で落ちたときに、自分が起こした子だけを止める（名前では探さない）
    if ($script:Child -and -not $script:Child.HasExited) { $script:Child.Kill() }
    Restore-Env
    $env:PATH = $script:OrigPath
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

$X64 = 'x86_64-pc-windows-msvc'; $I686 = 'i686-pc-windows-msvc'
$OUT_DIR = 'target/alpha'
$script:AppExe = "$OUT_DIR/$X64/release/areka.exe"
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

# RUSTFLAGS を置き換え、CARGO_ENCODED_RUSTFLAGS・CARGO_BUILD_RUSTFLAGS を外す（在ると cargo が RUSTFLAGS を無視する）。
# 差し替えはこの 2 段の間だけ。失敗時は Exit-Script の後始末が戻す。
Set-EnvTemp 'RUSTFLAGS' $script:RustFlags
Set-EnvTemp 'CARGO_ENCODED_RUSTFLAGS' $null
Set-EnvTemp 'CARGO_BUILD_RUSTFLAGS' $null
Step 'x64 本体ビルド' { cargo build --locked --release -p areka --target $X64 --target-dir $OUT_DIR }
Step 'i686 helper ビルド' { cargo build --locked --release -p shiori-host32-helper --target $I686 --target-dir $OUT_DIR }
Restore-Env

Step '静的リンクの確認' {
    $bad = $false
    foreach ($exe in @($script:AppExe, $script:HelperExe)) {
        if (-not (Test-Path -LiteralPath $exe)) { throw "ビルドの出力が無い: $exe" }
        $info = Read-PeInfo ([IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $exe)))
        $denied = @(Get-DeniedImports $info.Imports)
        Write-Host ('{0}: 機種 0x{1:x4}・取り込み {2}' -f $exe, $info.Machine, ($info.Imports -join ', '))
        if ($denied.Count) { Write-Host "  拒否表に当たる: $($denied -join ', ')"; $bad = $true }
    }
    if ($bad) { throw 'VC++ ランタイムの DLL を読んでいる（+crt-static が効いていない）' }
}

Step 'ライセンス検査' { cargo deny --locked check licenses }

Step '謝辞の生成' {
    if (Test-Path -LiteralPath $script:Notices) { Remove-Item -LiteralPath $script:Notices -Force }
    cargo about generate --locked --workspace --target $X64 --target $I686 about.hbs -o $script:Notices
    if ($LASTEXITCODE) { return }
    if (-not (Test-Path -LiteralPath $script:Notices)) { throw "謝辞の出力が無い: $script:Notices" }
}

# nar-sample-path の出力（1 行 1 組の key=value）から $Keys の値を読む。鍵が無い・パスが実在しなければ throw。
function Read-SamplePaths([string]$Sample, [string[]]$Keys) {
    $lines = @(cargo run -q --locked -p sample-ghost-kit --bin nar-sample-path -- $Sample)
    if ($LASTEXITCODE) { throw "nar-sample-path $Sample が終了コード $LASTEXITCODE" }
    $paths = @{}
    foreach ($key in $Keys) {
        $line = $lines | Where-Object { $_.StartsWith("$key=") } | Select-Object -First 1
        if (-not $line) { throw "nar-sample-path $Sample の出力に $key= が無い" }
        $path = $line.Substring($key.Length + 1)
        if (-not (Test-Path -LiteralPath $path -PathType Container)) { throw "nar-sample-path $Sample の $key= が実在しない: $path" }
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

$STAGE_DIR = "$OUT_DIR/stage"
Step '組み立て' {
    if (Test-Path -LiteralPath $STAGE_DIR) { Remove-Item -LiteralPath $STAGE_DIR -Recurse -Force }
    $null = New-Item -ItemType Directory -Path "$STAGE_DIR/ghost", "$STAGE_DIR/balloon"
    # 設計の「zip の中身」の 9 行だけ（写す元が無ければ Copy-Item が throw）
    Copy-Item -LiteralPath $script:AppExe -Destination $STAGE_DIR
    Copy-Item -LiteralPath $script:HelperExe -Destination $STAGE_DIR
    Copy-Item -LiteralPath $script:GhostDir -Destination "$STAGE_DIR/ghost/emo2" -Recurse
    Copy-Item -LiteralPath $script:KakukakuDir -Destination "$STAGE_DIR/balloon/emo2-kakukaku" -Recurse
    Copy-Item -LiteralPath $script:StayseeDir -Destination "$STAGE_DIR/balloon/StayseeBalloon" -Recurse
    Copy-Item -LiteralPath 'dist/README.txt' -Destination $STAGE_DIR
    Copy-Item -LiteralPath 'LICENSE-MIT' -Destination $STAGE_DIR
    Copy-Item -LiteralPath $script:Notices -Destination $STAGE_DIR
    $info = @(
        "commit=$script:Commit"
        "dirty=$script:Dirty"
        "built=$([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))"
        "script=tools/package-alpha.ps1 $SCRIPT_VERSION"
        "rustflags=$script:RustFlags"
    )
    [IO.File]::WriteAllLines((Join-Path (Resolve-Path -LiteralPath $STAGE_DIR) 'BUILD-INFO.txt'), $info)   # UTF-8（BOM 無し）
}

Step '圧縮' {
    $name = 'areka-alpha-x64-{0}-{1}{2}.zip' -f (Get-Date -Format 'yyyyMMdd'), $script:Commit, $(if ($script:Dirty) { '-dirty' } else { '' })
    $script:ZipFinal = Join-Path (Resolve-Path -LiteralPath $OUT_DIR) $name
    $script:ZipTmp = "$script:ZipFinal.tmp"
    if (Test-Path -LiteralPath $script:ZipTmp) { Remove-Item -LiteralPath $script:ZipTmp -Force }
    # includeBaseDirectory を $false にしないと stage/ が最上位に入る
    [IO.Compression.ZipFile]::CreateFromDirectory((Resolve-Path -LiteralPath $STAGE_DIR), $script:ZipTmp, [IO.Compression.CompressionLevel]::Optimal, $false)
    Write-Host $script:ZipTmp
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
function Test-ZipContent([string]$ZipPath) {
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
        $machines = [ordered]@{ 'areka.exe' = 0x8664; 'shiori-host32-helper.exe' = 0x014c; 'ghost/emo2/ghost/master/pasta.dll' = 0x014c }
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
            foreach ($want in @("commit=$script:Commit", "dirty=$script:Dirty")) {
                if ($info -cnotcontains $want) { $bad.Add("8 BUILD-INFO.txt に $want が無い") }
            }
        }
    } finally { $z.Zip.Dispose() }
    , $bad.ToArray()
}

Step '中身の判定' {
    $bad = Test-ZipContent $script:ZipTmp
    if (-not $bad.Count) { Write-Host '判定 1〜8 すべて合'; return }
    $bad | ForEach-Object { Write-Host "否 $_" }
    throw "中身の判定で否が $($bad.Count) 件"
}

Step '完成' {
    Move-Item -LiteralPath $script:ZipTmp -Destination $script:ZipFinal -Force
    $script:ZipTmp = $null
    Write-Host "zip: $script:ZipFinal"
    Write-Host "コミット $script:Commit・未コミットの変更 $script:Dirty 件"
}

# 記録の判定（要件 3.3・3.5）。2 つの記録を連結した文字列・子の終了コード・番犬で止めたかを受け、
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
    # 自動終了より後の挨拶は数えない（要件 3.3 の空振り）。両方とも run.log に出るので行の位置で比べる。
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
        $script:LogDir = "$script:ExpandDir-logs"
        if (Test-Path -LiteralPath $script:LogDir) { throw "記録の置き場が既に在る: $script:LogDir" }
        [IO.Compression.ZipFile]::ExtractToDirectory($script:ZipFinal, $script:ExpandDir)
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
}

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
