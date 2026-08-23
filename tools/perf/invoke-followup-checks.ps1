#Requires -Version 7.0
<#
================================================================================
invoke-followup-checks.ps1 — 見た目の追随をエージェント自身が実走で確かめる
  spec: areka-P0-draw-load-parity（要件 1.5 / 4.7・design.md「計測の道具（tools/perf/）
        → C13 追随チェック」）

何をするか:
  有界の自動終了つきで areka を起動し、表示が成立するのを待ってから、対象プロセスの
  ウィンドウへ実際に操作を当てて 4 つの追随を確かめる。操作は 1 手ずつ時刻つきで
  probe.log へ書き、判定は judge-followup.py が run.log と probe.log の両方を読んで行う。

  clickthrough    透明な画素の上と不透明な画素の上へカーソルを置き、WS_EX_TRANSPARENT の
                  有無が期待どおりに切り替わるか（本番のトグル記録が両方向で出ているか）
  drag            不透明な点から右へ 80px ドラッグし、キャラ窓とバルーン窓が同じだけ動くか
  dpi             DPI の違うモニタへキャラ窓を移して戻し、DPI 変更の受理と拡大率 k の
                  変化が出るか（DPI の違うモニタが無ければ判定不能）
  balloon_follow  上の 2 つの前後で、バルーンのキャラ窓相対位置が変わっていないか

  この 3 つの出力を -OutDir へ残す:
      run.log             実走の標準出力（本番のログ）
      probe.log           操作の記録（1 行 1 手・key=value）
      followup.txt        人が読む観測の要約
  さらに judge-followup.py が followup-verdict.txt を同じ場所へ書く。

--------------------------------------------------------------------------------
操作を注入できない環境について（黙って緑にしない）
--------------------------------------------------------------------------------
  対話していないセッション（サービス起動・別セッションからの実行・入力が遮断された
  デスクトップ）では SetCursorPos / SendInput が拒否される。このとき本スクリプトは
  「確かめた」と言わずに、その検査を判定不能（INCONCLUSIVE）として理由つきで記録する。
  注入が効くかどうかは推測せず、起動直後に実際に 1 回試して確かめる:
      SetCursorPos … 呼んだあと GetCursorPos でカーソルが本当に動いたかを見る
                     （戻り値だけでは足りない。偽の成功を弾くのはこの読み戻しである）
      SendInput    … 送れた件数と GetLastError を見る（0 件かつ 5＝アクセス拒否なら不可）
  判定不能は採用の根拠にならない（judge-followup.py が exit 2 を返す）。

--------------------------------------------------------------------------------
較正値・調整値の一覧（変更する場合はここだけを書き換える）
--------------------------------------------------------------------------------
  SCRIPT_VERSION        本スクリプトの版
  RUST_LOG_VALUE        実走時のログフィルタ（design C13 の指定）
  SMOKE_EXIT_ENV_NAME   有界自動終了の環境変数名
  DEFAULT_GHOST_ROOT    ゴースト（emo2）の既定ルート（絶対パス）
  DEFAULT_BALLOON_SUBDIR  -BalloonRoot 省略時に補うバルーンの相対位置
  SHOW_READY_TIMEOUT_MS 表示成立点を待つ上限
  SETTLE_MS             1 手ごとの反映待ち（クリック透過の再評価・窓書込の flush）
  DRAG_DX_PX            ドラッグで動かす距離（px・judge-followup.py と対）
  DRAG_STEPS            ドラッグ中に送る中間移動の回数
  DPI_WAIT_MS           DPI 変更の受理と表示成立を待つ上限
  EXIT_MARGIN_SEC       有界自動終了を過ぎてから強制終了するまでの猶予
  TRANSPARENT_INSET_PX  透明とみなす点（窓の左上角からの内寄せ）
  OPAQUE_BOTTOM_PX      不透明とみなす点（下端中央からの上寄せ）

終了コード（judge-followup.py の体系をそのまま返す）:
  0 … 総合 PASS  1 … FAIL あり  2 … FAIL 無しで判定不能あり  3 … 引数不正・読取不能
  1 は実走そのものに失敗した場合（起動できない・表示が成立しない）にも返す。

標準出力の末尾には必ず次の 1 行を出す（背景実行でも会話へ届く形・要件 1.9）:
  FOLLOWUP RESULT overall=<…> clickthrough=<…> drag=<…> dpi=<…> balloon_follow=<…> code=<n> dir=<path>

使い方:
  pwsh -NoProfile -File tools/perf/invoke-followup-checks.ps1 -OutDir C:\出力先
  pwsh -NoProfile -File tools/perf/invoke-followup-checks.ps1 -OutDir C:\出力先 `
      -BinDir C:\repo\iter-3\bin-B -ExitMs 120000
  pwsh -NoProfile -File tools/perf/invoke-followup-checks.ps1 -SelfTest
================================================================================
#>

[CmdletBinding(DefaultParameterSetName = 'Run')]
param(
    # 出力先ディレクトリ（run.log・probe.log・followup.txt を書く）
    [Parameter(ParameterSetName = 'Run', Mandatory = $true)]
    [string]$OutDir,

    # 実行体（areka.exe）と 32bit ヘルパの所在。省略時はリポジトリの target\debug
    [Parameter(ParameterSetName = 'Run')]
    [string]$BinDir,

    # ゴースト一式のルート（絶対パス）
    [Parameter(ParameterSetName = 'Run')]
    [string]$GhostRoot,

    # バルーンのルート（絶対パス。省略時は <GhostRoot>\emo2-kakukaku）
    [Parameter(ParameterSetName = 'Run')]
    [string]$BalloonRoot,

    # 有界自動終了までの時間（ms）
    [Parameter(ParameterSetName = 'Run')]
    [int]$ExitMs = 120000,

    # 実施する検査（既定は 4 つすべて）
    [Parameter(ParameterSetName = 'Run')]
    [string[]]$Checks = @('clickthrough', 'drag', 'dpi', 'balloon_follow'),

    # 判定そのものの較正（judge-followup.py --selftest を呼んでその終了コードを返す）
    [Parameter(ParameterSetName = 'SelfTest', Mandatory = $true)]
    [switch]$SelfTest
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# =============================================================================
# 較正値
# =============================================================================
$SCRIPT_VERSION        = '0.1.1'
$RUST_LOG_VALUE        = 'info,wintf::ecs::clickthrough=debug,wintf::transition=debug,areka_emo_present=debug'
$SMOKE_EXIT_ENV_NAME   = 'AREKA_APP_SMOKE_EXIT_MS'
$DEFAULT_BALLOON_SUBDIR = 'emo2-kakukaku'
$SHOW_READY_TIMEOUT_MS = 60000
$SETTLE_MS             = 400
$DRAG_DX_PX            = 80
$DRAG_STEPS            = 8
$DPI_WAIT_MS           = 8000
$EXIT_MARGIN_SEC       = 20
$TRANSPARENT_INSET_PX  = 2
$OPAQUE_BOTTOM_PX      = 10

#: 検査の固定語彙（judge-followup.py の CHECK_ALL と対）。
$CHECK_ALL = @('clickthrough', 'drag', 'dpi', 'balloon_follow')

#: 中止するときの終了コード。判定まで辿り着けた場合の 0／1／2 は judge-followup.py が
#: そのまま返すので、ここには本スクリプト自身が使う 2 つだけを置く。
$EXIT_FAIL     = 1
$EXIT_BAD_ARGS = 3

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$DEFAULT_GHOST_ROOT = Join-Path $repoRoot 'crates\pilot\examples\shiori-host-32\fixtures\emo2'
$judgePath = Join-Path $PSScriptRoot 'judge-followup.py'

# =============================================================================
# Win32（DPI 対応で読む・列挙・入力注入・窓の移動）
# =============================================================================
if (-not ('AkFollowupW32' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class AkFollowupW32 {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    public delegate bool MonitorEnumProc(IntPtr hMon, IntPtr hdc, IntPtr rect, IntPtr data);

    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll", EntryPoint="GetWindowLongPtrW")] public static extern IntPtr GetWindowLongPtr(IntPtr hWnd, int index);
    [DllImport("user32.dll", SetLastError=true)] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll", SetLastError=true)] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT pt);
    [DllImport("user32.dll", SetLastError=true)] public static extern uint SendInput(uint count, [In] INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr hWnd, uint flags);
    [DllImport("user32.dll")] public static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, MonitorEnumProc cb, IntPtr data);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool GetMonitorInfoW(IntPtr hMon, ref MONITORINFO info);
    [DllImport("shcore.dll")] public static extern int GetDpiForMonitor(IntPtr hMon, int type, out uint dx, out uint dy);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);

    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MONITORINFO {
        public int cbSize; public RECT rcMonitor; public RECT rcWork; public uint dwFlags;
    }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT {
        public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo;
    }
    // x64 の INPUT は 40 バイト（type 4 + 詰め 4 + MOUSEINPUT 32）。詰め物を自分で足すと
    // 48 バイトになり SendInput が ERROR_INVALID_PARAMETER(87) を返す。足さないこと。
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public MOUSEINPUT mi; }

    public static int InputSize() { return Marshal.SizeOf(typeof(INPUT)); }
}
"@
}

# DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4（以降の座標はすべて物理 px）
$null = [AkFollowupW32]::SetProcessDpiAwarenessContext([IntPtr](-4))

$GWL_EXSTYLE        = -20
$WS_EX_TRANSPARENT  = 0x20
$SWP_NOSIZE         = 0x0001
$SWP_NOZORDER       = 0x0004
$SWP_NOACTIVATE     = 0x0010
$MONITOR_DEFAULTTONEAREST = 0x2
$MOUSEEVENTF_MOVE     = 0x0001
$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP   = 0x0004
$MOUSEEVENTF_ABSOLUTE = 0x8000
$MOUSEEVENTF_VIRTUALDESK = 0x4000
$SM_XVIRTUALSCREEN = 76
$SM_YVIRTUALSCREEN = 77
$SM_CXVIRTUALSCREEN = 78
$SM_CYVIRTUALSCREEN = 79

# =============================================================================
# 小さな道具
# =============================================================================
function Get-UtcStamp {
    # run.log（tracing の RFC3339・UTC・小数 6 桁）と同じ形。判定側が両者を同じ物差しで
    # 並べるため、ここを崩してはならない。
    return [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture) + 'Z'
}

function Write-Info { param([string]$Text) Write-Host $Text }

function Write-Probe {
    # probe.log へ 1 行足す（1 手 1 行・key=value・値に空白を入れない）
    param([string]$Check, [string]$Step, [string[]]$Fields)
    $line = "probe: check=$Check step=$Step t=$(Get-UtcStamp)"
    if ($Fields) { $line += ' ' + ($Fields -join ' ') }
    Add-Content -LiteralPath $script:ProbeLogPath -Value $line -Encoding utf8
    $script:ProbeLines += $line
}

function Write-Note {
    # followup.txt へ人が読む 1 行を足す
    param([string]$Text)
    $script:NoteLines += $Text
}

function Stop-Run {
    param([int]$Code, [string]$Reason)
    Write-Host "[invoke-followup-checks] 中止: $Reason" -ForegroundColor Red
    if ($script:NoteLines) {
        Write-Note "中止: $Reason"
        Set-Content -LiteralPath $script:NotePath -Value ($script:NoteLines -join [Environment]::NewLine) -Encoding utf8
    }
    Write-Host ("FOLLOWUP RESULT overall=INCONCLUSIVE clickthrough=- drag=- dpi=- balloon_follow=- code={0} dir={1}" -f $Code, $script:OutDirPath)
    exit $Code
}

function Get-PythonCommand {
    # 単一要素の配列を返すと PowerShell が文字列へ展開してしまうため、
    # 実行体と前置引数を別の項として持つオブジェクトで返す。
    foreach ($candidate in @('python', 'python3')) {
        $found = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($found) { return [pscustomobject]@{ Exe = $found.Source; Prefix = @() } }
    }
    $py = Get-Command 'py' -ErrorAction SilentlyContinue
    if ($py) { return [pscustomobject]@{ Exe = $py.Source; Prefix = @('-3') } }
    return $null
}

function Invoke-Judge {
    # 判定の本文は必ず標準出力へ出す（会話へ届く形）。関数の戻り値で本文を持ち帰ると
    # 終了コードと混ざるため、終了コードはスクリプト変数へ置く。
    param([pscustomobject]$Python, [string[]]$Arguments)
    $argv = @($Python.Prefix) + $Arguments
    & $Python.Exe @argv | ForEach-Object { Write-Host $_ }
    $script:JudgeExitCode = $LASTEXITCODE
}

function Read-RunLogText {
    # 子プロセスが書いている最中のファイルを共有読みする（Get-Content は共有違反になり得る）
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return '' }
    try {
        $share = [System.IO.FileShare]::ReadWrite -bor [System.IO.FileShare]::Delete
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, $share)
        try {
            $reader = New-Object System.IO.StreamReader($stream, [System.Text.Encoding]::UTF8)
            return $reader.ReadToEnd()
        } finally { $stream.Dispose() }
    } catch { return '' }
}

function Get-RunLogLinesSince {
    # 行頭の時刻が $Since 以降の行だけを返す（観測窓の内側だけを見るため）
    param([string]$Path, [datetime]$Since)
    $text = Read-RunLogText -Path $Path
    if (-not $text) { return @() }
    $out = New-Object System.Collections.Generic.List[string]
    foreach ($line in ($text -split "`n")) {
        $trimmed = $line.TrimEnd("`r")
        if ($trimmed -match '^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?)Z\s') {
            $stamp = [DateTime]::Parse($Matches[1], [Globalization.CultureInfo]::InvariantCulture,
                [Globalization.DateTimeStyles]::AssumeUniversal -bor [Globalization.DateTimeStyles]::AdjustToUniversal)
            if ($stamp -ge $Since) { $out.Add($trimmed) }
        }
    }
    return $out.ToArray()
}

function Wait-RunLogPattern {
    # $Since 以降の行に正規表現が現れるまで待つ。現れたら $true
    param([string]$Path, [datetime]$Since, [string]$Pattern, [int]$TimeoutMs)
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while ([DateTime]::UtcNow -lt $deadline) {
        foreach ($line in (Get-RunLogLinesSince -Path $Path -Since $Since)) {
            if ($line -match $Pattern) { return $true }
        }
        Start-Sleep -Milliseconds 200
    }
    return $false
}

function Get-WindowRectObject {
    param([IntPtr]$Hwnd)
    $rect = New-Object AkFollowupW32+RECT
    if (-not [AkFollowupW32]::GetWindowRect($Hwnd, [ref]$rect)) { return $null }
    return [pscustomobject]@{
        X = $rect.Left; Y = $rect.Top
        W = $rect.Right - $rect.Left; H = $rect.Bottom - $rect.Top
    }
}

function Format-Hwnd { param([IntPtr]$Hwnd) return ('0x{0:X}' -f [int64]$Hwnd) }

function Test-ExTransparent {
    param([IntPtr]$Hwnd)
    $ex = [int64][AkFollowupW32]::GetWindowLongPtr($Hwnd, $GWL_EXSTYLE)
    return [pscustomobject]@{ Ex = ('0x{0:X}' -f $ex); Transparent = (($ex -band $WS_EX_TRANSPARENT) -ne 0) }
}

# =============================================================================
# -SelfTest（判定そのものの較正）
# =============================================================================
if ($SelfTest) {
    $python = Get-PythonCommand
    if (-not $python) {
        Write-Host '[invoke-followup-checks] python が見つかりません（自己較正は judge-followup.py を呼びます）。' -ForegroundColor Red
        exit $EXIT_BAD_ARGS
    }
    Invoke-Judge -Python $python -Arguments @($judgePath, '--selftest')
    exit $script:JudgeExitCode
}

# =============================================================================
# 引数の検証
# =============================================================================
$script:ProbeLines = @()
$script:NoteLines = @()

if (-not [System.IO.Path]::IsPathFullyQualified($OutDir)) {
    $OutDir = Join-Path (Get-Location).Path $OutDir
}
$script:OutDirPath = [System.IO.Path]::GetFullPath($OutDir).TrimEnd('\')
New-Item -ItemType Directory -Force -Path $script:OutDirPath | Out-Null
$script:ProbeLogPath = Join-Path $script:OutDirPath 'probe.log'
$script:NotePath     = Join-Path $script:OutDirPath 'followup.txt'
$runLogPath          = Join-Path $script:OutDirPath 'run.log'
$errLogPath          = Join-Path $script:OutDirPath 'run-stderr.log'
Set-Content -LiteralPath $script:ProbeLogPath -Value '' -Encoding utf8 -NoNewline

# -Checks は配列でも 'a,b,c' の 1 文字列でも受ける（perf-loop.measure.ps1 は子 pwsh へ引用した
# 1 文字列で渡す＝2026-08-23 周 1 の followup が exit 3「未知の検査名 'clickthrough,drag,…'」で止まった穴）。
$requested = @()
foreach ($name in @($Checks | ForEach-Object { [string]$_ -split ',' })) {
    $trimmed = $name.Trim()
    if (-not $trimmed) { continue }
    if ($CHECK_ALL -notcontains $trimmed) {
        Stop-Run $EXIT_BAD_ARGS "-Checks に未知の検査名があります: '$trimmed'（使えるのは $($CHECK_ALL -join '・')）"
    }
    if ($requested -notcontains $trimmed) { $requested += $trimmed }
}
if ($requested.Count -eq 0) {
    Stop-Run $EXIT_BAD_ARGS '-Checks が空です。確かめる検査が 0 件では、何も確かめずに合格を返してしまいます。'
}
# 判定表の並びは固定語彙の順に揃える（出力の並びが実行順で揺れないように）
$requested = @($CHECK_ALL | Where-Object { $requested -contains $_ })

if (-not $BinDir) { $BinDir = Join-Path $repoRoot 'target\debug' }
if (-not [System.IO.Path]::IsPathFullyQualified($BinDir)) { $BinDir = Join-Path $repoRoot $BinDir }
$binDirFull = [System.IO.Path]::GetFullPath($BinDir).TrimEnd('\')
$exePath = Join-Path $binDirFull 'areka.exe'
if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
    Stop-Run $EXIT_FAIL "実行体がありません: $exePath（-BinDir を確かめるか cargo build -p areka を先に実行してください）"
}

if (-not $GhostRoot) { $GhostRoot = $DEFAULT_GHOST_ROOT }
if (-not [System.IO.Path]::IsPathFullyQualified($GhostRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot は絶対パスで指定してください（相対だと SHIORI の読み込みに失敗します）: '$GhostRoot'"
}
$ghostRootFull = [System.IO.Path]::GetFullPath($GhostRoot).TrimEnd('\')
if (-not (Test-Path -LiteralPath $ghostRootFull -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot のフォルダがありません: $ghostRootFull"
}
if (-not $BalloonRoot) { $BalloonRoot = Join-Path $ghostRootFull $DEFAULT_BALLOON_SUBDIR }
if (-not [System.IO.Path]::IsPathFullyQualified($BalloonRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot は絶対パスで指定してください: '$BalloonRoot'"
}
$balloonRootFull = [System.IO.Path]::GetFullPath($BalloonRoot).TrimEnd('\')
if (-not (Test-Path -LiteralPath $balloonRootFull -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot のフォルダがありません: $balloonRootFull"
}
if ($ExitMs -lt 20000) {
    Stop-Run $EXIT_BAD_ARGS "-ExitMs が短すぎます（$ExitMs ms）。表示成立と 4 検査に足りません（20000 ms 以上）。"
}

$python = Get-PythonCommand
if (-not $python) {
    Stop-Run $EXIT_BAD_ARGS 'python が見つかりません（判定は judge-followup.py が行います）。'
}

Write-Info "[invoke-followup-checks] 版 $SCRIPT_VERSION / 実行体 $exePath"
Write-Info "[invoke-followup-checks] 出力先 $($script:OutDirPath) / 検査 $($requested -join ',') / 自動終了 ${ExitMs}ms"

# =============================================================================
# 実走の起動
# =============================================================================
Write-Probe -Check 'session' -Step 'begin' -Fields @(
    "script_version=$SCRIPT_VERSION",
    "required=$($requested -join ',')",
    "exit_ms=$ExitMs",
    "bin_dir=$($binDirFull -replace '\s','_')"
)
Write-Note "invoke-followup-checks.ps1 $SCRIPT_VERSION"
Write-Note "実行体: $exePath"
Write-Note "検査  : $($requested -join ', ')"

$proc = $null
$prevSmoke = [Environment]::GetEnvironmentVariable($SMOKE_EXIT_ENV_NAME)
$prevRustLog = $env:RUST_LOG
$prevNoColor = $env:NO_COLOR

try {
    [Environment]::SetEnvironmentVariable($SMOKE_EXIT_ENV_NAME, "$ExitMs")
    $env:RUST_LOG = $RUST_LOG_VALUE
    $env:NO_COLOR = '1'

    $launchedAt = [DateTime]::UtcNow
    try {
        $proc = Start-Process -FilePath $exePath `
            -ArgumentList @("`"$ghostRootFull`"", "`"$balloonRootFull`"") `
            -WorkingDirectory $binDirFull `
            -RedirectStandardOutput $runLogPath `
            -RedirectStandardError $errLogPath `
            -NoNewWindow -PassThru
    } catch {
        Stop-Run $EXIT_FAIL "areka を起動できませんでした（$exePath）: $($_.Exception.Message)"
    }
    if ($null -eq $proc) { Stop-Run $EXIT_FAIL "areka を起動できませんでした（$exePath）。" }
    Write-Info "[invoke-followup-checks] 起動しました（プロセス ID $($proc.Id)）。表示成立点を待ちます。"

    # --- 表示成立点を待つ（ここから先が観測の前提）------------------------------
    $showReady = Wait-RunLogPattern -Path $runLogPath -Since $launchedAt `
        -Pattern 'apply\(ShowSurface\): 表示・マスクを更新' -TimeoutMs $SHOW_READY_TIMEOUT_MS
    if (-not $showReady) {
        Write-Probe -Check 'session' -Step 'show_ready' -Fields @("waited_ms=$SHOW_READY_TIMEOUT_MS", 'result=timeout')
        Stop-Run $EXIT_FAIL "表示成立点（apply(ShowSurface)）が $SHOW_READY_TIMEOUT_MS ms 以内に出ませんでした。$runLogPath を確認してください。"
    }
    Start-Sleep -Milliseconds 1500   # 2 キャラ分の初期配置が落ち着くまで
    Write-Probe -Check 'session' -Step 'show_ready' -Fields @("waited_ms=$SHOW_READY_TIMEOUT_MS", 'result=ok')

    # --- 操作を注入できるかを実際に試す（推測しない）---------------------------
    $cursorBefore = New-Object AkFollowupW32+POINT
    $null = [AkFollowupW32]::GetCursorPos([ref]$cursorBefore)
    $setRet = [AkFollowupW32]::SetCursorPos($cursorBefore.X + 7, $cursorBefore.Y + 5)
    Start-Sleep -Milliseconds 60
    $cursorAfter = New-Object AkFollowupW32+POINT
    $null = [AkFollowupW32]::GetCursorPos([ref]$cursorAfter)
    $cursorMoved = ($cursorAfter.X -ne $cursorBefore.X) -or ($cursorAfter.Y -ne $cursorBefore.Y)
    if ($cursorMoved) { $null = [AkFollowupW32]::SetCursorPos($cursorBefore.X, $cursorBefore.Y) }

    $probeInput = New-Object AkFollowupW32+INPUT
    $probeInput.type = 0
    $probeMouse = New-Object AkFollowupW32+MOUSEINPUT
    $probeMouse.dwFlags = $MOUSEEVENTF_MOVE
    $probeInput.mi = $probeMouse
    $sent = [AkFollowupW32]::SendInput(1, @($probeInput), [AkFollowupW32]::InputSize())
    $sendErr = [Runtime.InteropServices.Marshal]::GetLastWin32Error()

    $cursorOk = $cursorMoved
    $inputOk = ($sent -ge 1)
    $injection = if ($cursorOk -and $inputOk) { 'available' } else { 'unavailable' }
    Write-Probe -Check 'env' -Step 'injection' -Fields @(
        "setcursorpos_ret=$($setRet.ToString().ToLower())",
        "cursor_moved=$($cursorMoved.ToString().ToLower())",
        "sendinput_sent=$sent",
        "sendinput_lasterr=$sendErr",
        "cursor_injection=$(if ($cursorOk) { 'available' } else { 'unavailable' })",
        "input_injection=$(if ($inputOk) { 'available' } else { 'unavailable' })",
        "injection=$injection"
    )
    Write-Note "操作の注入: $injection（SetCursorPos ret=$setRet 実移動=$cursorMoved / SendInput 送出=$sent lasterr=$sendErr）"
    Write-Info "[invoke-followup-checks] 操作の注入: $injection（カーソル=$cursorOk 入力=$inputOk）"

    # --- 対象の窓を決める -------------------------------------------------------
    # キャラ窓とバルーン窓はクラス名も表題も同じ（実測: どちらも wintf-winmsg-executor）。
    # 見分けは本番のログしか持っていないので、[transition] の hwnd と win_kind の対で引く。
    $identity = @{}
    foreach ($line in (Get-RunLogLinesSince -Path $runLogPath -Since $launchedAt)) {
        if ($line -match '\[transition\].*\bhwnd=(0x[0-9A-Fa-f]+)\b.*\bscope=(\S+)\s+win_kind=(char|balloon)\b') {
            $key = ('0x{0:X}' -f [Convert]::ToInt64($Matches[1], 16))
            $identity[$key] = [pscustomobject]@{ Scope = $Matches[2]; Kind = $Matches[3] }
        }
    }

    $windows = New-Object System.Collections.Generic.List[object]
    $targetPid = $proc.Id
    $callback = [AkFollowupW32+EnumWindowsProc]{
        param($hWnd, $lParam)
        $winPid = [uint32]0
        $null = [AkFollowupW32]::GetWindowThreadProcessId($hWnd, [ref]$winPid)
        if (([int]$winPid -eq $targetPid) -and [AkFollowupW32]::IsWindowVisible($hWnd)) {
            $rect = Get-WindowRectObject -Hwnd $hWnd
            if ($rect -and $rect.W -gt 0 -and $rect.H -gt 0) {
                $key = Format-Hwnd -Hwnd $hWnd
                $known = if ($identity.ContainsKey($key)) { $identity[$key] } else { $null }
                $windows.Add([pscustomobject]@{
                    Hwnd = $hWnd; Key = $key
                    Kind = if ($known) { $known.Kind } else { '-' }
                    Scope = if ($known) { $known.Scope } else { '-' }
                    X = $rect.X; Y = $rect.Y; W = $rect.W; H = $rect.H
                })
            }
        }
        return $true
    }
    $null = [AkFollowupW32]::EnumWindows($callback, [IntPtr]::Zero)

    foreach ($win in $windows) {
        $ex = Test-ExTransparent -Hwnd $win.Hwnd
        Write-Probe -Check 'windows' -Step 'enumerate' -Fields @(
            "hwnd=$($win.Key)", "win_kind=$($win.Kind)", "scope=$($win.Scope)",
            "x=$($win.X)", "y=$($win.Y)", "w=$($win.W)", "h=$($win.H)", "ex=$($ex.Ex)"
        )
    }
    Write-Note "PID $targetPid の可視窓 $($windows.Count) 枚（うち種別の判った窓 $(@($windows | Where-Object { $_.Kind -ne '-' }).Count) 枚）"

    # 対になっているキャラ窓とバルーン窓のうち、最も小さい scope を対象にする（決定論）
    $charWin = $null; $balloonWin = $null
    foreach ($scope in (@($windows | Where-Object { $_.Scope -ne '-' } | ForEach-Object { $_.Scope }) | Sort-Object -Unique)) {
        $c = $windows | Where-Object { $_.Scope -eq $scope -and $_.Kind -eq 'char' } | Select-Object -First 1
        $b = $windows | Where-Object { $_.Scope -eq $scope -and $_.Kind -eq 'balloon' } | Select-Object -First 1
        if ($c -and $b) { $charWin = $c; $balloonWin = $b; break }
    }

    if (-not $charWin -or -not $balloonWin) {
        # 種別を決められないときは、操作を当てる先が定まらない＝道具の側の不足。
        # 本番を赤にせず、すべて判定不能として記録する。
        foreach ($name in $requested) {
            Write-Probe -Check $name -Step 'status' -Fields @('status=unavailable', 'reason=window_classification_failed')
        }
        Write-Note 'キャラ窓とバルーン窓の対を特定できませんでした（[transition] の win_kind が出ていない）。'
    } else {
        Write-Probe -Check 'windows' -Step 'target' -Fields @(
            "scope=$($charWin.Scope)", "char_hwnd=$($charWin.Key)", "balloon_hwnd=$($balloonWin.Key)"
        )
        Write-Note "対象: キャラ窓 $($charWin.Key) / バルーン窓 $($balloonWin.Key)（scope=$($charWin.Scope)）"

        # =====================================================================
        # 検査 1: clickthrough
        # =====================================================================
        if ($requested -contains 'clickthrough') {
            if (-not $cursorOk) {
                Write-Probe -Check 'clickthrough' -Step 'status' -Fields @('status=unavailable', 'reason=cursor_injection_unavailable')
                Write-Note 'clickthrough: カーソルを動かせないため判定不能（SetCursorPos が拒否された）。'
            } else {
                $begin = [DateTime]::UtcNow
                $rect = Get-WindowRectObject -Hwnd $charWin.Hwnd
                $points = @(
                    @{ Name = 'transparent'; X = $rect.X + $TRANSPARENT_INSET_PX; Y = $rect.Y + $TRANSPARENT_INSET_PX; Expect = 'true' },
                    @{ Name = 'opaque'; X = $rect.X + [int]($rect.W / 2); Y = $rect.Y + $rect.H - $OPAQUE_BOTTOM_PX; Expect = 'false' }
                )
                foreach ($point in $points) {
                    $null = [AkFollowupW32]::SetCursorPos($point.X, $point.Y)
                    Start-Sleep -Milliseconds 80
                    $now = New-Object AkFollowupW32+POINT
                    $null = [AkFollowupW32]::GetCursorPos([ref]$now)
                    $landed = ($now.X -eq $point.X) -and ($now.Y -eq $point.Y)
                    Write-Probe -Check 'clickthrough' -Step 'probe_point' -Fields @(
                        "hwnd=$($charWin.Key)", "point=$($point.Name)", "x=$($point.X)", "y=$($point.Y)",
                        "moved=$($landed.ToString().ToLower())"
                    )
                    Start-Sleep -Milliseconds $SETTLE_MS
                    $ex = Test-ExTransparent -Hwnd $charWin.Hwnd
                    $observed = $ex.Transparent.ToString().ToLower()
                    $result = if ($observed -eq $point.Expect) { 'match' } else { 'mismatch' }
                    Write-Probe -Check 'clickthrough' -Step 'read' -Fields @(
                        "hwnd=$($charWin.Key)", "point=$($point.Name)", "ex=$($ex.Ex)",
                        "transparent=$observed", "expected=$($point.Expect)", "result=$result"
                    )
                    Write-Note "clickthrough $($point.Name) 点 ($($point.X),$($point.Y)): ex=$($ex.Ex) transparent=$observed 期待=$($point.Expect) → $result"
                }
                $null = [AkFollowupW32]::SetCursorPos($cursorBefore.X, $cursorBefore.Y)
                Start-Sleep -Milliseconds $SETTLE_MS
                Write-Probe -Check 'clickthrough' -Step 'window' -Fields @(
                    "begin_t=$($begin.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture))Z",
                    "end_t=$(Get-UtcStamp)"
                )
                Write-Probe -Check 'clickthrough' -Step 'status' -Fields @('status=done', 'reason=-')
            }
        }

        # =====================================================================
        # 検査 2: drag
        # =====================================================================
        if ($requested -contains 'drag') {
            if (-not $inputOk) {
                Write-Probe -Check 'drag' -Step 'status' -Fields @('status=unavailable', 'reason=input_injection_unavailable')
                Write-Note 'drag: 入力を注入できないため判定不能（SendInput が拒否された）。窓メッセージの偽装では実ドラッグと同じ経路を通らないので代替しない。'
            } else {
                $begin = [DateTime]::UtcNow
                foreach ($pair in @(@($charWin, 'char'), @($balloonWin, 'balloon'))) {
                    $rect = Get-WindowRectObject -Hwnd $pair[0].Hwnd
                    Write-Probe -Check 'drag' -Step 'rect' -Fields @(
                        'phase=before', "hwnd=$($pair[0].Key)", "win_kind=$($pair[1])",
                        "x=$($rect.X)", "y=$($rect.Y)", "w=$($rect.W)", "h=$($rect.H)"
                    )
                }
                $charRect = Get-WindowRectObject -Hwnd $charWin.Hwnd
                $fromX = $charRect.X + [int]($charRect.W / 2)
                $fromY = $charRect.Y + $charRect.H - $OPAQUE_BOTTOM_PX
                $toX = $fromX + $DRAG_DX_PX
                $toY = $fromY

                $vx = [AkFollowupW32]::GetSystemMetrics($SM_XVIRTUALSCREEN)
                $vy = [AkFollowupW32]::GetSystemMetrics($SM_YVIRTUALSCREEN)
                $vw = [AkFollowupW32]::GetSystemMetrics($SM_CXVIRTUALSCREEN)
                $vh = [AkFollowupW32]::GetSystemMetrics($SM_CYVIRTUALSCREEN)
                function Send-MouseAbsolute {
                    param([int]$X, [int]$Y, [uint32]$Flags)
                    $item = New-Object AkFollowupW32+INPUT
                    $item.type = 0
                    $mouse = New-Object AkFollowupW32+MOUSEINPUT
                    $mouse.dx = [int](($X - $vx) * 65535 / [math]::Max(1, $vw - 1))
                    $mouse.dy = [int](($Y - $vy) * 65535 / [math]::Max(1, $vh - 1))
                    $mouse.dwFlags = $Flags
                    $item.mi = $mouse
                    return [AkFollowupW32]::SendInput(1, @($item), [AkFollowupW32]::InputSize())
                }

                $null = [AkFollowupW32]::SetCursorPos($fromX, $fromY)
                Start-Sleep -Milliseconds 120
                $null = Send-MouseAbsolute -X $fromX -Y $fromY -Flags ($MOUSEEVENTF_MOVE -bor $MOUSEEVENTF_ABSOLUTE -bor $MOUSEEVENTF_VIRTUALDESK -bor $MOUSEEVENTF_LEFTDOWN)
                Start-Sleep -Milliseconds 80
                for ($step = 1; $step -le $DRAG_STEPS; $step++) {
                    $x = $fromX + [int]($DRAG_DX_PX * $step / $DRAG_STEPS)
                    $null = Send-MouseAbsolute -X $x -Y $toY -Flags ($MOUSEEVENTF_MOVE -bor $MOUSEEVENTF_ABSOLUTE -bor $MOUSEEVENTF_VIRTUALDESK)
                    Start-Sleep -Milliseconds 40
                }
                $null = Send-MouseAbsolute -X $toX -Y $toY -Flags ($MOUSEEVENTF_MOVE -bor $MOUSEEVENTF_ABSOLUTE -bor $MOUSEEVENTF_VIRTUALDESK -bor $MOUSEEVENTF_LEFTUP)
                Start-Sleep -Milliseconds 300
                $landedPoint = New-Object AkFollowupW32+POINT
                $null = [AkFollowupW32]::GetCursorPos([ref]$landedPoint)
                $landed = [math]::Abs($landedPoint.X - $toX) -le 2 -and [math]::Abs($landedPoint.Y - $toY) -le 2
                Write-Probe -Check 'drag' -Step 'gesture' -Fields @(
                    "hwnd=$($charWin.Key)", "from_x=$fromX", "from_y=$fromY", "to_x=$toX", "to_y=$toY",
                    "dx=$DRAG_DX_PX", 'dy=0', "moved=$($landed.ToString().ToLower())"
                )
                Start-Sleep -Milliseconds $SETTLE_MS
                foreach ($pair in @(@($charWin, 'char'), @($balloonWin, 'balloon'))) {
                    $rect = Get-WindowRectObject -Hwnd $pair[0].Hwnd
                    Write-Probe -Check 'drag' -Step 'rect' -Fields @(
                        'phase=after', "hwnd=$($pair[0].Key)", "win_kind=$($pair[1])",
                        "x=$($rect.X)", "y=$($rect.Y)", "w=$($rect.W)", "h=$($rect.H)"
                    )
                }
                Write-Probe -Check 'drag' -Step 'window' -Fields @(
                    "begin_t=$($begin.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture))Z",
                    "end_t=$(Get-UtcStamp)"
                )
                if ($landed) {
                    Write-Probe -Check 'drag' -Step 'status' -Fields @('status=done', 'reason=-')
                } else {
                    Write-Probe -Check 'drag' -Step 'status' -Fields @('status=unavailable', 'reason=cursor_did_not_reach_target')
                }
                Write-Note "drag: ($fromX,$fromY) → ($toX,$toY) カーソル到達=$landed"
            }
        }

        # =====================================================================
        # 検査 3: dpi
        # =====================================================================
        if ($requested -contains 'dpi') {
            $monitors = New-Object System.Collections.Generic.List[object]
            $monCallback = [AkFollowupW32+MonitorEnumProc]{
                param($hMon, $hdc, $rect, $data)
                $info = New-Object AkFollowupW32+MONITORINFO
                $info.cbSize = [Runtime.InteropServices.Marshal]::SizeOf([type]([AkFollowupW32+MONITORINFO]))
                if ([AkFollowupW32]::GetMonitorInfoW($hMon, [ref]$info)) {
                    $dx = [uint32]0; $dy = [uint32]0
                    $null = [AkFollowupW32]::GetDpiForMonitor($hMon, 0, [ref]$dx, [ref]$dy)
                    $monitors.Add([pscustomobject]@{
                        Handle = $hMon; Dpi = [int]$dx
                        Wa = $info.rcWork
                    })
                }
                return $true
            }
            $null = [AkFollowupW32]::EnumDisplayMonitors([IntPtr]::Zero, [IntPtr]::Zero, $monCallback, [IntPtr]::Zero)

            $index = 0
            foreach ($mon in $monitors) {
                Write-Probe -Check 'dpi' -Step 'monitors' -Fields @(
                    "mon=$index", "hmon=$('0x{0:X}' -f [int64]$mon.Handle)", "dpi=$($mon.Dpi)",
                    "wa=$($mon.Wa.Left),$($mon.Wa.Top),$($mon.Wa.Right),$($mon.Wa.Bottom)"
                )
                $index++
            }
            $distinctDpi = @($monitors | ForEach-Object { $_.Dpi } | Sort-Object -Unique)
            Write-Note "モニタ $($monitors.Count) 面・DPI の種類 $($distinctDpi -join ',')"

            if ($distinctDpi.Count -lt 2) {
                Write-Probe -Check 'dpi' -Step 'status' -Fields @('status=unavailable', 'reason=single_dpi')
                Write-Note 'dpi: DPI の異なるモニタが 2 面以上ないため判定不能。'
            } else {
                $begin = [DateTime]::UtcNow
                $homeRect = Get-WindowRectObject -Hwnd $charWin.Hwnd
                foreach ($pair in @(@($charWin, 'char'), @($balloonWin, 'balloon'))) {
                    $rect = Get-WindowRectObject -Hwnd $pair[0].Hwnd
                    Write-Probe -Check 'dpi' -Step 'rect' -Fields @(
                        'phase=before', "hwnd=$($pair[0].Key)", "win_kind=$($pair[1])",
                        "x=$($rect.X)", "y=$($rect.Y)", "w=$($rect.W)", "h=$($rect.H)"
                    )
                }
                $homeMon = [AkFollowupW32]::MonitorFromWindow($charWin.Hwnd, $MONITOR_DEFAULTTONEAREST)
                $homeDpi = (@($monitors | Where-Object { $_.Handle -eq $homeMon }) + @($monitors[0]))[0].Dpi
                $away = @($monitors | Where-Object { $_.Dpi -ne $homeDpi })[0]
                $awayX = $away.Wa.Left + 40
                $awayY = $away.Wa.Top + 40

                $moveOk = [AkFollowupW32]::SetWindowPos($charWin.Hwnd, [IntPtr]::Zero, $awayX, $awayY, 0, 0,
                    ($SWP_NOSIZE -bor $SWP_NOZORDER -bor $SWP_NOACTIVATE))
                Write-Probe -Check 'dpi' -Step 'move' -Fields @(
                    'phase=out', "hwnd=$($charWin.Key)", "x=$awayX", "y=$awayY",
                    "from_dpi=$homeDpi", "to_dpi=$($away.Dpi)",
                    "result=$(if ($moveOk) { 'ok' } else { 'failed' })"
                )
                $null = Wait-RunLogPattern -Path $runLogPath -Since $begin -Pattern 'kind=msg msg=WM_DPICHANGED' -TimeoutMs $DPI_WAIT_MS
                Start-Sleep -Milliseconds 1200

                $backOk = [AkFollowupW32]::SetWindowPos($charWin.Hwnd, [IntPtr]::Zero, $homeRect.X, $homeRect.Y, 0, 0,
                    ($SWP_NOSIZE -bor $SWP_NOZORDER -bor $SWP_NOACTIVATE))
                Write-Probe -Check 'dpi' -Step 'move' -Fields @(
                    'phase=back', "hwnd=$($charWin.Key)", "x=$($homeRect.X)", "y=$($homeRect.Y)",
                    "from_dpi=$($away.Dpi)", "to_dpi=$homeDpi",
                    "result=$(if ($backOk) { 'ok' } else { 'failed' })"
                )
                Start-Sleep -Milliseconds 1800
                foreach ($pair in @(@($charWin, 'char'), @($balloonWin, 'balloon'))) {
                    $rect = Get-WindowRectObject -Hwnd $pair[0].Hwnd
                    Write-Probe -Check 'dpi' -Step 'rect' -Fields @(
                        'phase=after', "hwnd=$($pair[0].Key)", "win_kind=$($pair[1])",
                        "x=$($rect.X)", "y=$($rect.Y)", "w=$($rect.W)", "h=$($rect.H)"
                    )
                }
                Write-Probe -Check 'dpi' -Step 'window' -Fields @(
                    "begin_t=$($begin.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture))Z",
                    "end_t=$(Get-UtcStamp)"
                )
                Write-Probe -Check 'dpi' -Step 'status' -Fields @('status=done', 'reason=-')
                Write-Note "dpi: $homeDpi → $($away.Dpi) へ移して戻した（出=$moveOk 戻=$backOk）"
            }
        }

        # =====================================================================
        # 検査 4: balloon_follow（drag / dpi の前後の矩形を使う・自前の操作は無い）
        # =====================================================================
        if ($requested -contains 'balloon_follow') {
            $sources = @('drag', 'dpi') | Where-Object { $requested -contains $_ }
            $notDone = @()
            foreach ($source in $sources) {
                $done = @($script:ProbeLines | Where-Object { $_ -match "check=$source step=status .*status=done" }).Count -gt 0
                if (-not $done) { $notDone += $source }
            }
            if ($notDone.Count -gt 0 -or $sources.Count -eq 0) {
                Write-Probe -Check 'balloon_follow' -Step 'status' -Fields @('status=unavailable', 'reason=depends_on_drag_dpi')
                Write-Note "balloon_follow: 前後で比べる操作（$($notDone -join '・')）が成立しなかったため判定不能。"
            } else {
                Write-Probe -Check 'balloon_follow' -Step 'status' -Fields @('status=done', 'reason=-')
            }
        }
    }

    # --- 有界自動終了を待つ（正規の終了経路を使い、ログを最後まで書かせる）-------
    Write-Info '[invoke-followup-checks] 検査を終えました。有界自動終了を待ちます。'
    $deadline = $launchedAt.AddMilliseconds($ExitMs).AddSeconds($EXIT_MARGIN_SEC)
    while (-not $proc.HasExited) {
        if ([DateTime]::UtcNow -ge $deadline) {
            try { $proc.Kill() } catch { }
            Write-Note "警告: 有界自動終了（${ExitMs}ms ＋猶予 ${EXIT_MARGIN_SEC}秒）を過ぎたため強制終了しました。"
            break
        }
        Start-Sleep -Milliseconds 250
    }
    try { $proc.WaitForExit(5000) | Out-Null } catch { }
    Write-Probe -Check 'session' -Step 'end' -Fields @(
        "exit_code=$(if ($proc.HasExited) { $proc.ExitCode } else { '-' })",
        "elapsed_sec=$([math]::Round(([DateTime]::UtcNow - $launchedAt).TotalSeconds, 1))"
    )
} finally {
    [Environment]::SetEnvironmentVariable($SMOKE_EXIT_ENV_NAME, $prevSmoke)
    if ($null -eq $prevRustLog) { Remove-Item Env:\RUST_LOG -ErrorAction SilentlyContinue } else { $env:RUST_LOG = $prevRustLog }
    if ($null -eq $prevNoColor) { Remove-Item Env:\NO_COLOR -ErrorAction SilentlyContinue } else { $env:NO_COLOR = $prevNoColor }
    if ($proc -and -not $proc.HasExited) { try { $proc.Kill() } catch { } }
}

# =============================================================================
# 判定
# =============================================================================
Set-Content -LiteralPath $script:NotePath -Value ($script:NoteLines -join [Environment]::NewLine) -Encoding utf8

Invoke-Judge -Python $python -Arguments @($judgePath, $script:OutDirPath)
$overallCode = $script:JudgeExitCode

$verdictPath = Join-Path $script:OutDirPath 'followup-verdict.txt'
$summary = 'overall=- clickthrough=- drag=- dpi=- balloon_follow=-'
if (Test-Path -LiteralPath $verdictPath) {
    $verdictLine = @(Get-Content -LiteralPath $verdictPath -Encoding utf8 |
        Where-Object { $_ -like 'FOLLOWUP VERDICT *' }) | Select-Object -Last 1
    if ($verdictLine) {
        $summary = ($verdictLine -replace '^FOLLOWUP VERDICT\s+', '') -replace '\s+code=\d+\s*$', ''
    }
}
Write-Host ("FOLLOWUP RESULT {0} code={1} dir={2}" -f $summary, $overallCode, $script:OutDirPath)
exit $overallCode
