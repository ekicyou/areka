#Requires -Version 7.0
<#
================================================================================
invoke-cpu-sample.ps1 — CPU サンプリング（呼出スタック付き）の採取・停止・記号解決を 1 コマンドで
  spec: areka-P0-draw-load-parity（要件 2.4 / 2.11 / 8.6・design「計測の道具（tools/perf/）
        → C8 invoke-cpu-sample.ps1 → Batch / Job Contract」）

何をするか（4 つのモードのどれか 1 つを指定する）:
  -Probe     … 採れるかどうかだけを確かめる。昇格しているか・xperf.exe が在るか・
               実際に 5 秒だけ採って止められるかを見て、1 行で報告する（必ず exit 0）
  -Start     … サンプリングと呼出スタックの採取を始める（管理者権限が要る）
  -Stop      … 採取を止めてトレースをまとめ、記号を解決してテキストの dump を書き出す
  -SelfTest  … 道具そのものの較正。同梱の dump 断片で「areka.exe! のフレームを数える」
               関門が正しく数えられるか（合格側）と、フレームが無い入力で 0 を返すか
               （不合格側）を確かめ、続けて -Probe と同じ確認を行う

「関門」について（要件 2.11 ＝ 黙って続けない）:
  記号が解決できていない dump は、見た目は正常でも中身が空である。そこで -Stop は
  書き出した dump を必ず数え直し、areka.exe! のフレームが 0 なら計測失敗（exit 4）で
  止める。-SelfTest はこの数え方そのものを、既知の合格側・不合格側の両方で毎回試す。

昇格していない環境について:
  段③（関数別の帰属）は管理者権限が無いと採れない。これは「計測の失敗」ではなく
  「能力の不足」なので exit 5（UNAVAILABLE）で区別する。呼び出し側は段③だけを省いて
  段①②④で続行してよい（design「Error Categories and Responses」）。

--------------------------------------------------------------------------------
較正値・調整値の一覧（変更する場合はここだけを書き換える）
--------------------------------------------------------------------------------
  SCRIPT_VERSION            本スクリプトの版
  XPERF_DEFAULT_PATH        PATH に xperf が無いときに見に行く既定の位置
  PROF_INTERVAL_UNITS       サンプリング周期（-SetProfInt の単位。1221 ≒ 8kHz）
  PROBE_CAPTURE_SEC         -Probe が実際に採ってみる秒数
  XPERF_KERNEL_FLAGS        採取するカーネルプロバイダ
  XPERF_STACKWALK           呼出スタックを採る対象イベント
  BUFFER_SIZE_KB / MAX_BUFFERS  採取バッファ
  SYMBOL_SERVER_URL         公開シンボルサーバ
  DEFAULT_SYMCACHE_SUBDIR   記号キャッシュの既定位置（LOCALAPPDATA 配下）
  FIXTURE_DUMP_RELPATH      -SelfTest が読む同梱 dump 断片
  FIXTURE_EXPECT_*          同梱 dump 断片の既知の内訳（-SelfTest がこの値と厳密に突き合わせる。
                            fixture を実採取で差し替えたら、この値も同時に書き換える）
  WPA_PROFILE_RELPATH       代替 backend（wpaexporter）が使う版管理された .wpaProfile
  TARGET_MODULE             関門が数える対象モジュール名

記号（要件 8.6 ＝ Cargo.toml 非接触）:
  release ビルドの記号は、ビルド時の環境変数
      $env:CARGO_PROFILE_RELEASE_DEBUG = 'line-tables-only'
  で付与する。Cargo.toml には触れない。なお本ワークスペースの release は
  lto=true・opt-level='z' であり、インライン化でスタックが浅くなる（呼び元が
  1 段に潰れて見える）。詳しくは tools/perf/README.md を参照。

終了コード（design「C5 の終了コード体系」に合わせる）:
  0 … 正常終了（-Probe は採れない場合も 0。可否は標準出力の 1 行で報告する）
  1 … 実行の失敗（xperf が非ゼロを返した・自己較正が赤）
  3 … 引数・前提の不正
  4 … 計測失敗（dump が空・記号解決ゼロ）
  5 … 能力不足 ＝ UNAVAILABLE（昇格していない・xperf.exe が無い）

標準出力の末尾には必ず次の 1 行を出す（背景実行でも会話へ届く形）:
  INVOKE-CPU-SAMPLE RESULT <mode> code=<n> <キー=値 …>

使い方:
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -Probe
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -SelfTest
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -Start -Etl C:\出力\cpu.etl
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -Stop  -Etl C:\出力\cpu.etl `
      -Out C:\出力\dump.txt -PdbDir C:\repo\target\release
================================================================================
#>

[CmdletBinding(DefaultParameterSetName = 'Probe')]
param(
    # 採れるかどうかだけを確かめる（必ず exit 0・1 行で報告）
    [Parameter(ParameterSetName = 'Probe', Mandatory = $true)]
    [switch]$Probe,

    # 採取を始める
    [Parameter(ParameterSetName = 'Start', Mandatory = $true)]
    [switch]$Start,

    # 採取を止めて記号解決まで行う
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [switch]$Stop,

    # 道具そのものの較正
    [Parameter(ParameterSetName = 'SelfTest', Mandatory = $true)]
    [switch]$SelfTest,

    # 採取先／読み取り元のトレースファイル
    [Parameter(ParameterSetName = 'Start', Mandatory = $true)]
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [string]$Etl,

    # 書き出すテキスト dump
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [string]$Out,

    # 実行体の PDB が在る場所（例: target\release）
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [string]$PdbDir,

    # 記号キャッシュ（省略時は LOCALAPPDATA\areka-diag\symcache）
    [Parameter(ParameterSetName = 'Stop')]
    [string]$SymCache,

    # 記号解決の方式。目標定義ファイルの [sampling] backend より優先する
    [Parameter(ParameterSetName = 'Stop')]
    [ValidateSet('xperf-dumper', 'wpaexporter')]
    [string]$Backend,

    # 目標定義ファイル（[sampling] backend = "…" を読む）
    [Parameter(ParameterSetName = 'Stop')]
    [string]$GoalFile,

    # サンプリング周期（-SetProfInt の単位）。0 なら較正値を使う
    [Parameter(ParameterSetName = 'Start')]
    [int]$ProfIntervalUnits = 0
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------- 較正値 ----
$SCRIPT_VERSION           = '1.0.0'
$XPERF_DEFAULT_PATH       = 'C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe'
$WPAEXPORTER_DEFAULT_PATH = 'C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\wpaexporter.exe'
$PROF_INTERVAL_UNITS      = 1221
$PROBE_CAPTURE_SEC        = 5
$XPERF_KERNEL_FLAGS       = 'PROC_THREAD+LOADER+PROFILE'
$XPERF_STACKWALK          = 'Profile'
$BUFFER_SIZE_KB           = 1024
$MAX_BUFFERS              = 512
$SYMBOL_SERVER_URL        = 'https://msdl.microsoft.com/download/symbols'
$DEFAULT_SYMCACHE_SUBDIR  = 'areka-diag\symcache'
$FIXTURE_DUMP_RELPATH     = 'fixtures-loop\rank\sample_ok\dump.txt'
# 同梱 dump 断片の既知の内訳（fixtures-loop/rank/sample_ok/README.md に公表してある値）。
# -SelfTest はこの値と厳密に突き合わせる。「1 つ以上」で済ませると、たとえば
# ThreadStartImage!Function 列まで数えてしまう実装（16 ではなく 32 になる）を見逃す。
$FIXTURE_EXPECT_SAMPLE_COUNT      = 16
$FIXTURE_EXPECT_STACK_COUNT       = 8
$FIXTURE_EXPECT_AREKA_FRAMES      = 16
$FIXTURE_EXPECT_RESOLVED_FRAMES   = 22
$FIXTURE_EXPECT_UNRESOLVED_FRAMES = 2
$FIXTURE_EXPECT_TIDS              = @('18332', '18420', '18512')
$WPA_PROFILE_RELPATH      = 'wpa\cpu-sampled.wpaProfile'
$TARGET_MODULE            = 'areka.exe'

# 終了コード（design C5）
$EXIT_OK             = 0
$EXIT_RUN_FAILED     = 1
$EXIT_BAD_ARGS       = 3
$EXIT_MEASURE_FAILED = 4
$EXIT_UNAVAILABLE    = 5

$SCRIPT_DIR = Split-Path -Parent $PSCommandPath

# --------------------------------------------------------------- 補助関数 ----
function Write-Info    { param([string]$Message) Write-Host $Message }
function Write-Problem { param([string]$Message) Write-Host "エラー: $Message" }

# 標準出力の末尾に置く終端行。走行固有のトークンを添えて、背景終了でも読み取れるようにする。
function Write-ResultLine {
    param([string]$Mode, [int]$Code, [string]$Extra = '')
    $line = "INVOKE-CPU-SAMPLE RESULT $Mode code=$Code version=$SCRIPT_VERSION"
    if ($Extra) { $line = "$line $Extra" }
    Write-Host $line
}

function Stop-Run {
    param([string]$Mode, [int]$Code, [string]$Message, [string]$Extra = '')
    if ($Message) { Write-Problem $Message }
    Write-ResultLine -Mode $Mode -Code $Code -Extra $Extra
    exit $Code
}

# xperf.exe の所在。PATH を先に見て、無ければ Windows Kits の既定位置を見る。
function Get-XperfPath {
    $cmd = Get-Command 'xperf' -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($cmd) { return $cmd.Source }
    if (Test-Path -LiteralPath $XPERF_DEFAULT_PATH -PathType Leaf) { return $XPERF_DEFAULT_PATH }
    return $null
}

function Get-WpaExporterPath {
    $cmd = Get-Command 'wpaexporter' -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($cmd) { return $cmd.Source }
    if (Test-Path -LiteralPath $WPAEXPORTER_DEFAULT_PATH -PathType Leaf) { return $WPAEXPORTER_DEFAULT_PATH }
    return $null
}

function Test-Elevated {
    $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [System.Security.Principal.WindowsPrincipal]::new($id)
    return $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
}

# 外部コマンドを走らせ、終了コードと標準出力・標準エラーを揃えて返す。
function Invoke-NativeCommand {
    param([string]$FilePath, [string[]]$ArgumentList)
    $outFile = [System.IO.Path]::GetTempFileName()
    $errFile = [System.IO.Path]::GetTempFileName()
    try {
        $p = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -NoNewWindow -Wait -PassThru `
            -RedirectStandardOutput $outFile -RedirectStandardError $errFile
        return [pscustomobject]@{
            ExitCode = $p.ExitCode
            StdOut   = (Get-Content -LiteralPath $outFile -Raw -ErrorAction SilentlyContinue)
            StdErr   = (Get-Content -LiteralPath $errFile -Raw -ErrorAction SilentlyContinue)
        }
    } finally {
        Remove-Item -LiteralPath $outFile, $errFile -Force -ErrorAction SilentlyContinue
    }
}

function Get-StartArguments {
    param([string]$EtlPath)
    $interval = if ($ProfIntervalUnits -gt 0) { $ProfIntervalUnits } else { $PROF_INTERVAL_UNITS }
    return @(
        '-on', $XPERF_KERNEL_FLAGS,
        '-stackwalk', $XPERF_STACKWALK,
        '-SetProfInt', "$interval",
        '-BufferSize', "$BUFFER_SIZE_KB",
        '-MaxBuffers', "$MAX_BUFFERS",
        '-f', $EtlPath
    )
}

# ------------------------------------------------------------------ 関門 ----
# dump のテキストから「解決できた areka.exe! のフレーム」を数える。
# -Stop の関門と -SelfTest の合格側・不合格側は、必ずこの同じ関数を通す（＝関門を較正する）。
#
# 列は必ず列名行（ヘッダ）から引く（xperf の版で列が動くため・design C9 の Risks）。
# 列名行の実物（Windows Performance Toolkit の perf_nt_c.dll に埋め込まれた書式そのもの）:
#   SampledProfile,  TimeStamp,  Process Name ( PID),  ThreadID,  PrgrmCtr, CPU,
#                    ThreadStartImage!Function, Image!Function, Count, SampledProfile type
#   Stack,  TimeStamp,  ThreadID, No.,  Address,  Image!Function
# 数えるのは Image!Function 列だけである。ThreadStartImage!Function 列はスレッドの
# 起点であって帰属先ではないので数えない（素朴な文字列検索だと二重に数えてしまう）。
function Measure-ArekaFrames {
    param([string]$Text)

    $result = [pscustomobject]@{
        SampleCount      = 0    # SampledProfile 行の数
        StackCount       = 0    # Stack 行の数
        ArekaFrames      = 0    # Image!Function 列が areka.exe のフレームの数
        ResolvedFrames   = 0    # 記号が解いてある（関数名が 0x… でない）フレームの数
        UnresolvedFrames = 0    # 記号が解けていないフレームの数
        HeaderFound      = $false
        ThreadIds        = @()  # 行に出てきた ThreadID（重複なし・昇順）
        Problem          = ''
    }
    if ([string]::IsNullOrWhiteSpace($Text)) {
        $result.Problem = 'dump が空です'
        return $result
    }

    $headers = @{}   # イベント名 → @{ Names = 列名の配列; ImageIndex = Image!Function の位置 }
    foreach ($raw in ($Text -split "`r?`n")) {
        $line = $raw.Trim()
        if (-not $line) { continue }
        if ($line -eq 'BeginHeader' -or $line -eq 'EndHeader') { continue }
        if ($line.StartsWith('//') -or $line.StartsWith('#')) { continue }
        if ($line -notmatch ',') { continue }

        $fields = $line -split ','
        $name = $fields[0].Trim()
        if ($name -ne 'SampledProfile' -and $name -ne 'Stack') { continue }

        $trimmed = @($fields | ForEach-Object { $_.Trim() })
        # 列名行の判別: 2 列目が TimeStamp という語そのもの（データ行はここが数値）。
        if ($trimmed.Count -ge 2 -and $trimmed[1] -eq 'TimeStamp') {
            $imageIndex = -1
            for ($i = $trimmed.Count - 1; $i -ge 0; $i--) {
                if ($trimmed[$i] -eq 'Image!Function') { $imageIndex = $i; break }
            }
            $headers[$name] = @{ Names = $trimmed; ImageIndex = $imageIndex }
            $result.HeaderFound = $true
            continue
        }

        if (-not $headers.ContainsKey($name)) { continue }   # 列名行の無いイベントは読まない
        if ($name -eq 'SampledProfile') { $result.SampleCount++ } else { $result.StackCount++ }

        $tidIndex = [array]::IndexOf($headers[$name].Names, 'ThreadID')
        if ($tidIndex -ge 0 -and $tidIndex -lt $fields.Count) {
            $tid = $fields[$tidIndex].Trim()
            if ($tid -and $result.ThreadIds -notcontains $tid) { $result.ThreadIds += $tid }
        }

        $idx = $headers[$name].ImageIndex
        if ($idx -lt 0) { continue }
        $expected = $headers[$name].Names.Count
        # Rust の記号は総称型の中にカンマを含み得るので、余った分は Image!Function 側へ寄せる。
        $extra = $fields.Count - $expected
        if ($extra -lt 0) { continue }
        $value = (($fields[$idx..($idx + $extra)]) -join ',').Trim()
        if (-not $value) { continue }

        $bang = $value.IndexOf('!')
        if ($bang -lt 0) { continue }
        $module = $value.Substring(0, $bang).Trim()
        $func   = $value.Substring($bang + 1).Trim()
        if ($func -match '^0x[0-9a-fA-F]+$' -or $module -eq 'Unknown' -or $module -eq '??') {
            $result.UnresolvedFrames++
        } else {
            $result.ResolvedFrames++
        }
        if ($module -eq $TARGET_MODULE) { $result.ArekaFrames++ }
    }

    # 既知の列名行が欠けていたら、どのイベントの分が無いのかまで言う（design C9 の Risks:
    # 既知列が無ければ exit 4）。SampledProfile だけ在って Stack が無い dump は、
    # 呼出スタックが 1 本も読めていないのに黙って通ってしまうので、ここで捕まえる。
    $missingHeaders = @()
    foreach ($known in @('SampledProfile', 'Stack')) {
        if (-not $headers.ContainsKey($known)) { $missingHeaders += $known }
    }
    if ($missingHeaders.Count -gt 0) {
        $result.Problem = "既知の列名行が dump に在りません: $($missingHeaders -join '／')"
    }
    $result.ThreadIds = @($result.ThreadIds | Sort-Object)
    return $result
}

# 昇格・xperf の実在・5 秒の実採取までを確かめる。返すのは available と reason。
function Get-ProbeResult {
    param([switch]$SkipCapture)

    if (-not (Test-Elevated)) {
        return [pscustomobject]@{ Available = $false; Reason = 'not_elevated' }
    }
    $xperf = Get-XperfPath
    if (-not $xperf) {
        return [pscustomobject]@{ Available = $false; Reason = 'no_xperf' }
    }
    if ($SkipCapture) {
        return [pscustomobject]@{ Available = $true; Reason = 'ok' }
    }

    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ('areka-cpu-probe-' + [guid]::NewGuid().ToString('N'))
    $null = New-Item -ItemType Directory -Path $tempDir -Force
    $probeEtl  = Join-Path $tempDir 'probe.etl'
    $mergedEtl = Join-Path $tempDir 'probe-merged.etl'
    try {
        $r = Invoke-NativeCommand -FilePath $xperf -ArgumentList (Get-StartArguments -EtlPath $probeEtl)
        if ($r.ExitCode -ne 0) {
            return [pscustomobject]@{ Available = $false; Reason = 'start_failed' }
        }
        Start-Sleep -Seconds $PROBE_CAPTURE_SEC
        $d = Invoke-NativeCommand -FilePath $xperf -ArgumentList @('-d', $mergedEtl)
        if ($d.ExitCode -ne 0) {
            $null = Invoke-NativeCommand -FilePath $xperf -ArgumentList @('-stop')
            return [pscustomobject]@{ Available = $false; Reason = 'start_failed' }
        }
        return [pscustomobject]@{ Available = $true; Reason = 'ok' }
    } catch {
        $null = Invoke-NativeCommand -FilePath $xperf -ArgumentList @('-stop')
        return [pscustomobject]@{ Available = $false; Reason = 'start_failed' }
    } finally {
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# 目標定義ファイルの [sampling] backend を読む（この 1 キーだけの最小読み）。
function Get-BackendFromGoalFile {
    param([string]$Path)
    if (-not $Path) { return $null }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-Run -Mode 'stop' -Code $EXIT_BAD_ARGS -Message "-GoalFile が見つかりません: $Path"
    }
    $inSampling = $false
    foreach ($raw in (Get-Content -LiteralPath $Path)) {
        $line = $raw.Trim()
        if ($line -match '^\[(.+)\]$') { $inSampling = ($Matches[1].Trim() -eq 'sampling'); continue }
        if ($inSampling -and $line -match '^backend\s*=\s*"([^"]+)"') { return $Matches[1] }
    }
    return $null
}

# ------------------------------------------------------------- 各モード ----
function Invoke-ProbeMode {
    $probe = Get-ProbeResult
    $available = if ($probe.Available) { 'true' } else { 'false' }
    Write-Info "available=$available reason=$($probe.Reason)"
    Write-ResultLine -Mode 'probe' -Code $EXIT_OK -Extra "available=$available reason=$($probe.Reason)"
    exit $EXIT_OK
}

function Invoke-StartMode {
    # 実採取までは試さない（本番の採取をこれから始めるため）。前提だけを見る。
    $probe = Get-ProbeResult -SkipCapture
    if (-not $probe.Available) {
        Write-Info "available=false reason=$($probe.Reason)"
        Stop-Run -Mode 'start' -Code $EXIT_UNAVAILABLE `
            -Message "CPU サンプリングを始められません（reason=$($probe.Reason)）。段③（関数別の帰属）は昇格した PowerShell が要ります。これは計測の失敗ではなく能力の不足です。" `
            -Extra "available=false reason=$($probe.Reason)"
    }
    $etlFull = [System.IO.Path]::GetFullPath($Etl)
    $parent = Split-Path -Parent $etlFull
    if ($parent -and -not (Test-Path -LiteralPath $parent -PathType Container)) {
        try { $null = New-Item -ItemType Directory -Path $parent -Force }
        catch { Stop-Run -Mode 'start' -Code $EXIT_BAD_ARGS -Message "-Etl の出力先 $parent を作成できませんでした: $($_.Exception.Message)" }
    }
    $xperf = Get-XperfPath
    $r = Invoke-NativeCommand -FilePath $xperf -ArgumentList (Get-StartArguments -EtlPath $etlFull)
    if ($r.ExitCode -ne 0) {
        Stop-Run -Mode 'start' -Code $EXIT_RUN_FAILED `
            -Message "xperf の採取開始が失敗しました（終了コード $($r.ExitCode)）: $($r.StdErr)$($r.StdOut)"
    }
    Write-Info "採取を開始しました: $etlFull"
    Write-ResultLine -Mode 'start' -Code $EXIT_OK -Extra "etl=$etlFull"
    exit $EXIT_OK
}

function Invoke-StopMode {
    $etlFull = [System.IO.Path]::GetFullPath($Etl)
    $outFull = [System.IO.Path]::GetFullPath($Out)
    $pdbFull = [System.IO.Path]::GetFullPath($PdbDir)
    if (-not (Test-Path -LiteralPath $pdbFull -PathType Container)) {
        Stop-Run -Mode 'stop' -Code $EXIT_BAD_ARGS -Message "-PdbDir のフォルダが在りません: $pdbFull（release ビルドは環境変数 CARGO_PROFILE_RELEASE_DEBUG=line-tables-only で行ってください）"
    }
    $xperf = Get-XperfPath
    if (-not $xperf) {
        Stop-Run -Mode 'stop' -Code $EXIT_UNAVAILABLE -Message 'xperf.exe が見つかりません。' -Extra 'available=false reason=no_xperf'
    }
    if (-not $SymCache) {
        $localApp = [Environment]::GetFolderPath('LocalApplicationData')
        if (-not $localApp) { Stop-Run -Mode 'stop' -Code $EXIT_BAD_ARGS -Message '記号キャッシュの既定位置（LOCALAPPDATA）を解決できませんでした。-SymCache で明示してください。' }
        $SymCache = Join-Path $localApp $DEFAULT_SYMCACHE_SUBDIR
    }
    $null = New-Item -ItemType Directory -Path $SymCache -Force -ErrorAction SilentlyContinue

    # 記号解決の方式は採取を止める前に決める（引数の不備は仕事を始める前に言う）
    $backendName = $Backend
    if (-not $backendName) { $backendName = Get-BackendFromGoalFile -Path $GoalFile }
    if (-not $backendName) { $backendName = 'xperf-dumper' }
    $wpaProfile = Join-Path $SCRIPT_DIR $WPA_PROFILE_RELPATH
    if ($backendName -eq 'wpaexporter' -and -not (Test-Path -LiteralPath $wpaProfile -PathType Leaf)) {
        Stop-Run -Mode 'stop' -Code $EXIT_BAD_ARGS `
            -Message "profile file missing: $wpaProfile — 代替 backend（wpaexporter）は版管理された .wpaProfile が要ります。既定の backend（xperf-dumper）を使うか、.wpaProfile を追加してください。"
    }

    # ⒜ 採取を止めてトレースをまとめる（マージ済みの ETL を作る）
    $merged = [System.IO.Path]::ChangeExtension($etlFull, '.merged.etl')
    $d = Invoke-NativeCommand -FilePath $xperf -ArgumentList @('-d', $merged)
    if ($d.ExitCode -ne 0) {
        if (-not (Test-Path -LiteralPath $etlFull -PathType Leaf)) {
            Stop-Run -Mode 'stop' -Code $EXIT_RUN_FAILED `
                -Message "xperf の採取停止が失敗し、$etlFull も在りません（終了コード $($d.ExitCode)）: $($d.StdErr)$($d.StdOut)"
        }
        Write-Info "採取の停止（-d）が非ゼロを返しました。既に在る $etlFull をそのまま読みます。"
        $merged = $etlFull
    }

    # ⒝ 記号解決の道筋を子プロセスへ渡し、テキスト dump を書き出す
    $savedSymbolPath = $env:_NT_SYMBOL_PATH
    $env:_NT_SYMBOL_PATH = "srv*$SymCache*$SYMBOL_SERVER_URL;$pdbFull"
    try {
        if ($backendName -eq 'wpaexporter') {
            $wpa = Get-WpaExporterPath
            if (-not $wpa) {
                Stop-Run -Mode 'stop' -Code $EXIT_UNAVAILABLE -Message 'wpaexporter.exe が見つかりません。' -Extra 'available=false reason=no_xperf'
            }
            $outDir = Split-Path -Parent $outFull
            $w = Invoke-NativeCommand -FilePath $wpa -ArgumentList @('-i', $merged, '-profile', $wpaProfile, '-outputfolder', $outDir)
            if ($w.ExitCode -ne 0) {
                Stop-Run -Mode 'stop' -Code $EXIT_RUN_FAILED -Message "wpaexporter が失敗しました（終了コード $($w.ExitCode)）: $($w.StdErr)$($w.StdOut)"
            }
        } else {
            $s = Invoke-NativeCommand -FilePath $xperf `
                -ArgumentList @('-i', $merged, '-symbols', '-a', 'dumper', '-o', $outFull)
            if ($s.ExitCode -ne 0) {
                Stop-Run -Mode 'stop' -Code $EXIT_RUN_FAILED -Message "xperf の記号解決（-a dumper）が失敗しました（終了コード $($s.ExitCode)）: $($s.StdErr)$($s.StdOut)"
            }
        }
    } finally {
        $env:_NT_SYMBOL_PATH = $savedSymbolPath
    }

    # ⒞ 関門: 書き出した dump を数え直す（要件 2.11 ＝ 黙って続けない）
    if (-not (Test-Path -LiteralPath $outFull -PathType Leaf)) {
        Stop-Run -Mode 'stop' -Code $EXIT_MEASURE_FAILED -Message "dump が書き出されていません: $outFull"
    }
    $stats = Measure-ArekaFrames -Text (Get-Content -LiteralPath $outFull -Raw)
    if ($stats.Problem) {
        Stop-Run -Mode 'stop' -Code $EXIT_MEASURE_FAILED `
            -Message "dump を読めません（$($stats.Problem)）— dump=$outFull。xperf の版で列の並びが変わった可能性があります。" `
            -Extra "dump=$outFull sample_count=$($stats.SampleCount) stack_rows=$($stats.StackCount)"
    }
    if ($stats.ArekaFrames -lt 1) {
        $why = if ($stats.Problem) { "（$($stats.Problem)）" } else { '' }
        Stop-Run -Mode 'stop' -Code $EXIT_MEASURE_FAILED `
            -Message "記号解決ゼロ（areka.exe! フレーム 0）$why — dump=$outFull sample_count=$($stats.SampleCount)。release を環境変数 CARGO_PROFILE_RELEASE_DEBUG=line-tables-only でビルドし、-PdbDir に PDB が在ることを確かめてください。" `
            -Extra "dump=$outFull sample_count=$($stats.SampleCount) areka_frames=0"
    }
    $summary = "sample_count=$($stats.SampleCount) areka_frames=$($stats.ArekaFrames) dump=$outFull"
    Write-Info $summary
    Write-ResultLine -Mode 'stop' -Code $EXIT_OK `
        -Extra "$summary stack_rows=$($stats.StackCount) resolved=$($stats.ResolvedFrames) unresolved=$($stats.UnresolvedFrames) backend=$backendName"
    exit $EXIT_OK
}

function Invoke-SelfTestMode {
    $failures = @()

    # 期待値と実測が食い違ったら、どちらの数字も残す（あとで台帳から追える形）。
    function Test-Expected {
        param([string]$Side, [string]$Label, $Expected, $Actual)
        if ($Expected -eq $Actual) { return $null }
        return "${Side}: $Label が期待値と違います（期待 $Expected ／ 実際 $Actual）"
    }

    # ⒜-1 合格側: 同梱の dump 断片の内訳が、README に公表した既知の値と厳密に一致する。
    #      「1 つ以上」ではなく厳密一致にしてある。素朴に areka.exe! を数える実装は
    #      ThreadStartImage!Function 列まで拾って 16 ではなく 32 になるので、
    #      厳密一致でなければその取り違えを見逃す。
    $fixture = Join-Path $SCRIPT_DIR $FIXTURE_DUMP_RELPATH
    $okFrames = 0
    $okSamples = 0
    if (-not (Test-Path -LiteralPath $fixture -PathType Leaf)) {
        $failures += "同梱の dump 断片が在りません: $fixture"
    } else {
        $okStats = Measure-ArekaFrames -Text (Get-Content -LiteralPath $fixture -Raw)
        $okFrames = $okStats.ArekaFrames
        $okSamples = $okStats.SampleCount
        if ($okStats.Problem) { $failures += "合格側: $($okStats.Problem)（$fixture）" }
        $failures += @(
            (Test-Expected '合格側' 'SampledProfile 行の数'   $FIXTURE_EXPECT_SAMPLE_COUNT      $okStats.SampleCount),
            (Test-Expected '合格側' 'Stack 行の数'            $FIXTURE_EXPECT_STACK_COUNT       $okStats.StackCount),
            (Test-Expected '合格側' 'areka.exe のフレーム数'  $FIXTURE_EXPECT_AREKA_FRAMES      $okStats.ArekaFrames),
            (Test-Expected '合格側' '記号解決済みフレーム数'  $FIXTURE_EXPECT_RESOLVED_FRAMES   $okStats.ResolvedFrames),
            (Test-Expected '合格側' '記号未解決フレーム数'    $FIXTURE_EXPECT_UNRESOLVED_FRAMES $okStats.UnresolvedFrames),
            (Test-Expected '合格側' 'ThreadID の一覧'         ($FIXTURE_EXPECT_TIDS -join ',')  ($okStats.ThreadIds -join ','))
        ) | Where-Object { $_ }
        Write-Info "selftest 合格側: sample_count=$($okStats.SampleCount) stack_rows=$($okStats.StackCount) areka_frames=$($okStats.ArekaFrames) resolved=$($okStats.ResolvedFrames) unresolved=$($okStats.UnresolvedFrames) tids=$($okStats.ThreadIds -join ',')"
        Write-Info "selftest 合格側の期待値: sample_count=$FIXTURE_EXPECT_SAMPLE_COUNT stack_rows=$FIXTURE_EXPECT_STACK_COUNT areka_frames=$FIXTURE_EXPECT_AREKA_FRAMES resolved=$FIXTURE_EXPECT_RESOLVED_FRAMES unresolved=$FIXTURE_EXPECT_UNRESOLVED_FRAMES tids=$($FIXTURE_EXPECT_TIDS -join ',')"
    }

    # ⒜-2 不合格側: areka.exe を含まない dump では必ず 0 と数える（毎回赤も作る）
    $redText = @(
        'BeginHeader',
        '         SampledProfile,  TimeStamp,     Process Name ( PID),   ThreadID,           PrgrmCtr, CPU, ThreadStartImage!Function,            Image!Function, Count, SampledProfile type',
        '                Stack,  TimeStamp,   ThreadID, No.,            Address,            Image!Function',
        'EndHeader',
        '         SampledProfile,    1052388,      notepad.exe ( 900),       1234, 0x00007ffb0d21c4a0,   1,     ntdll.dll!RtlUserThreadStart,       win32u.dll!NtUserPeekMessage,     1,     1',
        '         SampledProfile,    1053509,      notepad.exe ( 900),       1234, 0x00007ffb1c2d3e4f,   1,     ntdll.dll!RtlUserThreadStart,          Unknown!0x00007ffb1c2d3e4f,     1,     1',
        '                  Stack,    1053509,       1234,   1, 0x00007ffb0d21c4a0,       win32u.dll!NtUserPeekMessage'
    ) -join "`n"
    $redStats = Measure-ArekaFrames -Text $redText
    $failures += @(
        (Test-Expected '不合格側' 'areka.exe のフレーム数'  0 $redStats.ArekaFrames),
        (Test-Expected '不合格側' 'SampledProfile 行の数'   2 $redStats.SampleCount),
        (Test-Expected '不合格側' 'Stack 行の数'            1 $redStats.StackCount),
        (Test-Expected '不合格側' '記号未解決フレーム数'    1 $redStats.UnresolvedFrames)
    ) | Where-Object { $_ }
    if ($redStats.Problem) { $failures += "不合格側: 両方の列名行が在るのに読めないと言いました（$($redStats.Problem)）" }
    Write-Info "selftest 不合格側: sample_count=$($redStats.SampleCount) stack_rows=$($redStats.StackCount) areka_frames=$($redStats.ArekaFrames)（areka_frames=0 が期待値）"

    # ⒜-3 空の入力は「dump が空」と言い、黙って 0 を返さない
    $emptyStats = Measure-ArekaFrames -Text ''
    if (-not $emptyStats.Problem) { $failures += '空入力に対して理由を返しませんでした' }

    # ⒜-4 列名行が欠けた入力は、どのイベントの分が無いのかまで言う（-Stop はこれで exit 4）
    $noStackText = @(
        '         SampledProfile,  TimeStamp,     Process Name ( PID),   ThreadID,           PrgrmCtr, CPU, ThreadStartImage!Function,            Image!Function, Count, SampledProfile type',
        '         SampledProfile,    1052388,        areka.exe (23140),      18332, 0x00007ff6c4a91b30,   6,        areka.exe!mainCRTStartup,        areka.exe!wintf::ecs::world::EcsWorld::try_tick_world,     1,     1'
    ) -join "`n"
    $noStackStats = Measure-ArekaFrames -Text $noStackText
    if ($noStackStats.Problem -notmatch 'Stack') {
        $failures += "列名行の欠落: Stack の列名行が無い入力を通してしまいました（Problem='$($noStackStats.Problem)'）"
    }
    Write-Info "selftest 列名行の欠落: problem=$($noStackStats.Problem)"

    # ⒝ -Probe（採れないことは赤ではなく UNAVAILABLE の報告）
    $probe = Get-ProbeResult
    $available = if ($probe.Available) { 'true' } else { 'false' }
    Write-Info "available=$available reason=$($probe.Reason)"

    if ($failures.Count -gt 0) {
        foreach ($f in $failures) { Write-Problem $f }
        Stop-Run -Mode 'selftest' -Code $EXIT_RUN_FAILED -Message '自己較正が赤です（道具そのものが壊れています）。' `
            -Extra "failures=$($failures.Count) available=$available reason=$($probe.Reason)"
    }
    Write-Info '自己較正: 合格（合格側は既知の内訳と厳密一致／不合格側 0 フレーム／空入力と列名行欠落に理由あり）'
    Write-ResultLine -Mode 'selftest' -Code $EXIT_OK `
        -Extra "fixture_areka_frames=$okFrames fixture_sample_count=$okSamples red_areka_frames=$($redStats.ArekaFrames) available=$available reason=$($probe.Reason)"
    exit $EXIT_OK
}

# ------------------------------------------------------------------ 本体 ----
switch ($PSCmdlet.ParameterSetName) {
    'Probe'    { Invoke-ProbeMode }
    'Start'    { Invoke-StartMode }
    'Stop'     { Invoke-StopMode }
    'SelfTest' { Invoke-SelfTestMode }
    default    { Stop-Run -Mode 'none' -Code $EXIT_BAD_ARGS -Message '-Probe / -Start / -Stop / -SelfTest のどれか 1 つを指定してください。' }
}
