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
  SAMPLE_NAME           既定で使う検体の名前（検体の窓口へ渡す名前）
  SAMPLE_BALLOON_NAME   その検体が同梱するバルーンの名前（窓口の `balloon.<名前>=` の鍵）
  SHOW_READY_TIMEOUT_MS 表示成立点を待つ上限
  SETTLE_MS             1 手ごとの反映待ち（クリック透過の再評価・窓書込の flush）
  DRAG_DX_PX            ドラッグで動かす距離（px・judge-followup.py と対）
  DRAG_STEPS            ドラッグ中に送る中間移動の回数
  DPI_WAIT_MS           DPI 変更の受理と表示成立を待つ上限
  EXIT_MARGIN_SEC       有界自動終了を過ぎてから強制終了するまでの猶予
  KILL_WAIT_MS          強制終了の後に子孫まで終わるのを待つ上限（ミリ秒）
  TRANSPARENT_INSET_PX  透明とみなす点（窓の左上角からの内寄せ）
  OPAQUE_BOTTOM_PX      不透明とみなす点（下端中央からの上寄せ）

終了コード（judge-followup.py の 0・1・2 はそのまま、判定器の 3＝読取不能とほかは 1 に写す）:
  0 … 総合 PASS  1 … FAIL あり  2 … FAIL 無しで判定不能あり  3 … 本スクリプトの引数不正
  1 は実走そのものに失敗した場合（起動できない・表示が成立しない・窓が先に閉じた）にも返す。
  -SelfTest は自分の検査と判定器の較正のどちらかが赤なら非ゼロ。

標準出力の末尾には必ず次の 1 行を出す（実走のとき。-SelfTest は終了コードだけ・背景実行でも会話へ届く形・要件 1.9）:
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

    # バルーンのルート（絶対パス。省略時は検体の窓口が教える emo2 同梱バルーンの場所）
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
# shortcut: 必須の引数の欠けは PowerShell の束ねの段で RESULT 行の無い 1 になる（script の中から捕まえる口が無い）。
# 呼び手（perf-loop）が引数を欠く形で呼ぶようになったら、必須を外して自分で確かめて 3 にする

# 想定外の失敗は理由の 1 行と中止の形の FOLLOWUP RESULT 行で 1（perf-loop は 1 度だけ採り直す）
trap {
    # 共通部品より前の失敗（読み込めない）でも元の原因と中止の形の RESULT 行で 1
    if (-not (Get-Command -Name 'Stop-Run' -CommandType Function -ErrorAction Ignore)) {
        [Console]::Error.WriteLine("[invoke-followup-checks] stopped: unexpected failure before the shared helpers loaded: $($_.Exception.Message -replace '\s*\r?\n\s*', ' ')")
        [Console]::Out.WriteLine('FOLLOWUP RESULT overall=INCONCLUSIVE clickthrough=- drag=- dpi=- balloon_follow=- code=1 dir=-')
        exit 1
    }
    Stop-Run $EXIT_FAIL (Format-UnexpectedError -ErrorRecord $_)
}

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# trap の Stop-Run が出力先の決まる前でも読める値
$script:NoteLines  = @()
$script:OutDirPath = '-'

# =============================================================================
# 較正値
# =============================================================================
$SCRIPT_VERSION        = '0.2.0'
$RUST_LOG_VALUE        = 'info,wintf::ecs::clickthrough=debug,wintf::transition=debug,areka_emo_present=debug'
$SMOKE_EXIT_ENV_NAME   = 'AREKA_APP_SMOKE_EXIT_MS'
$SAMPLE_NAME            = 'emo2'
$SAMPLE_BALLOON_NAME    = 'emo2-kakukaku'
$SHOW_READY_TIMEOUT_MS = 60000
$SETTLE_MS             = 400
$DRAG_DX_PX            = 80
$DRAG_STEPS            = 8
$DPI_WAIT_MS           = 8000
$EXIT_MARGIN_SEC       = 20
$KILL_WAIT_MS          = 10000
$TRANSPARENT_INSET_PX  = 2
$OPAQUE_BOTTOM_PX      = 10

#: 検査の固定語彙（judge-followup.py の CHECK_ALL と対）。
$CHECK_ALL = @('clickthrough', 'drag', 'dpi', 'balloon_follow')

#: 中止するときの終了コード。判定まで辿り着けた場合の 0／1／2 は judge-followup.py が
#: そのまま返すので、ここには本スクリプト自身が使う 2 つだけを置く。
$EXIT_FAIL     = 1
$EXIT_BAD_ARGS = 3

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$judgePath = Join-Path $PSScriptRoot 'judge-followup.py'

#: 表示成立点の実行ログの目印（本番のログの文言と字で突き合わせる）。
$LOG_MARKER_SHOW_READY = 'apply\(ShowSurface\): 表示・マスクを更新'

# 表示・時刻・子の起こし方・検体の在り処・python 探しは共通部品から
. (Join-Path $PSScriptRoot 'perf-tool-base.ps1')

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

    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
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
    // On x64, INPUT is 40 bytes (type 4 + padding 4 + MOUSEINPUT 32). Adding the padding by hand
    // makes it 48 bytes and SendInput returns ERROR_INVALID_PARAMETER(87). Do not add it.
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public MOUSEINPUT mi; }

    public static int InputSize() { return Marshal.SizeOf(typeof(INPUT)); }
}
"@
}

# DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4（以降の座標はすべて物理 px）
$null = [AkFollowupW32]::SetThreadDpiAwarenessContext([IntPtr](-4))

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
function Get-ProbeStamp {
    # run.log（tracing の RFC3339・UTC・小数 6 桁）と同じ形。判定側が両者を同じ物差しで
    # 並べるため、ここを崩してはならない。
    return [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture) + 'Z'
}

function Write-Probe {
    # probe.log へ 1 行足す（1 手 1 行・key=value・値に空白を入れない）
    param([string]$Check, [string]$Step, [string[]]$Fields)
    $line = "probe: check=$Check step=$Step t=$(Get-ProbeStamp)"
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
    Write-Problem "[invoke-followup-checks] stopped: $Reason"
    if ($script:NoteLines) {
        Write-Note "stopped: $Reason"
        Set-Content -LiteralPath $script:NotePath -Value ($script:NoteLines -join [Environment]::NewLine) -Encoding utf8
    }
    Write-Host ("FOLLOWUP RESULT overall=INCONCLUSIVE clickthrough=- drag=- dpi=- balloon_follow=- code={0} dir={1}" -f $Code, $script:OutDirPath)
    exit $Code
}

# 検体の窓口の値（読みは共通部品の Get-NarSamplePath）。取れなければ止める——同梱バルーンを
# 持たない検体では `balloon.<名前>=` の行そのものが出ず、空を掴んだまま進むとパス無しで起動してしまう。
function Get-NarSampleValue {
    param([Parameter(Mandatory = $true)][string]$Sample, [Parameter(Mandatory = $true)][string]$Key)
    $r = Get-NarSamplePath -Sample $Sample -Key $Key
    if (-not $r.Ok) { Stop-Run $EXIT_FAIL $r.Message }
    return $r.Value
}

function Invoke-Judge {
    # 判定の本文は必ず標準出力へ出す（会話へ届く形）。関数の戻り値で本文を持ち帰ると
    # 終了コードと混ざるため、終了コードはスクリプト変数へ置く。
    param([pscustomobject]$Python, [string[]]$Arguments)
    # judge-followup.py は標準出力と標準エラーを UTF-8 に寄せて書くので、UTF-8 で読み、
    # 標準出力の行・標準エラーの行の順に行のまま写す（末尾の改行 1 つだけを落とす）
    $result = Invoke-PythonChild -Python $Python -Arguments $Arguments -Quiet
    foreach ($outLine in $result.Out) { Write-Info $outLine }
    foreach ($errLine in $result.Err) { if ($errLine -ne '') { Write-Info $errLine } }
    $script:JudgeExitCode = $result.Code
}

# balloon_follow が比べる前後の操作（drag・dpi のうち求められたもの）のうち、status=done に届かなかったもの（純関数）。
# 戻り: Sources（求められた操作・必ず配列）・NotDone（届かなかった操作）・Ready（操作が 1 つ以上あり全部届いた）
function Get-BalloonFollowSources {
    param([string[]]$Requested, [string[]]$ProbeLines)
    $sources = @(@('drag', 'dpi') | Where-Object { $Requested -contains $_ })
    $notDone = @($sources | Where-Object { $s = $_; -not @($ProbeLines | Where-Object { $_ -match "check=$s step=status .*status=done" }).Count })
    return [pscustomobject]@{ Sources = $sources; NotDone = $notDone; Ready = ($sources.Count -gt 0 -and $notDone.Count -eq 0) }
}

# 判定器（judge-followup.py）の終了コードを返す値へ写す（純関数）: 0・1・2 はそのまま、
# 3（読取不能）とほか（起こせない -1 を含む）は 1＝実走の失敗（perf-loop が 1 度だけ採り直す）
function Convert-FollowupJudgeExit {
    param([int]$Code)
    if ($Code -in 0, 1, 2) { return $Code }
    return $EXIT_FAIL
}

# run.log の増えた分だけを読む読み手。位置と、改行の来ていない最後の 1 行のバイトを覚える
# （バイトで持つので、UTF-8 の 1 文字が読みの間で割れても崩れない）。
function New-RunLogTail {
    param([string]$Path)
    return [pscustomobject]@{ Path = $Path; Position = [long]0; Pending = [byte[]]::new(0) }
}

# 前の読みの後に書き終わった行を返す（書きかけの最後の 1 行は次に回す・同じ行を 2 度返さない）。
# 子プロセスが書いている最中のファイルを共有読みする（Get-Content は共有違反になり得る）。
function Read-RunLogTail {
    param([pscustomobject]$Tail)
    if (-not (Test-Path -LiteralPath $Tail.Path)) { return @() }
    try {
        $share = [System.IO.FileShare]::ReadWrite -bor [System.IO.FileShare]::Delete
        $stream = [System.IO.File]::Open($Tail.Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, $share)
        try {
            $count = [int][Math]::Max(0, $stream.Length - $Tail.Position)
            $buf = [byte[]]::new($Tail.Pending.Length + $count)
            [Array]::Copy($Tail.Pending, $buf, $Tail.Pending.Length)
            $null = $stream.Seek($Tail.Position, [System.IO.SeekOrigin]::Begin)
            $read = 0
            while ($read -lt $count) {
                $n = $stream.Read($buf, $Tail.Pending.Length + $read, $count - $read)
                if ($n -le 0) { break }
                $read += $n
            }
        } finally { $stream.Dispose() }
    } catch { return @() }
    $end = $Tail.Pending.Length + $read
    # 先頭の BOM はバッファの頭がファイルの頭のときだけ落とす（1 度目に BOM の途中までしか見えなくても次で落ちる）
    $start = if (($Tail.Position - $Tail.Pending.Length) -eq 0 -and $end -ge 3 -and $buf[0] -eq 0xEF -and $buf[1] -eq 0xBB -and $buf[2] -eq 0xBF) { 3 } else { 0 }
    $Tail.Position += $read
    $lastNl = if ($end -gt 0) { [Array]::LastIndexOf($buf, [byte]10, $end - 1) } else { -1 }
    $lines = @()
    $from = $start
    if ($lastNl -ge $start) {
        $text = [System.Text.Encoding]::UTF8.GetString($buf, $start, $lastNl - $start)
        $lines = @($text -split "`n" | ForEach-Object { $_.TrimEnd("`r") })
        $from = $lastNl + 1
    }
    $Tail.Pending = [byte[]]::new($end - $from)
    [Array]::Copy($buf, $from, $Tail.Pending, 0, $end - $from)
    return $lines
}

function Get-RunLogLinesSince {
    # 読み手の増えた分のうち、行頭の時刻が $Since 以降の行だけを返す（観測窓の内側だけを見るため）
    param([pscustomobject]$Tail, [datetime]$Since)
    $out = New-Object System.Collections.Generic.List[string]
    foreach ($trimmed in @(Read-RunLogTail -Tail $Tail)) {
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
    # -Process が終わったら待たずに $false（起動直後に落ちた areka を上限まで待たない）
    param([string]$Path, [datetime]$Since, [string]$Pattern, [int]$TimeoutMs, [System.Diagnostics.Process]$Process)
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    $tail = New-RunLogTail -Path $Path
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($Process -and $Process.HasExited) {
            foreach ($line in (Get-RunLogLinesSince -Tail $tail -Since $Since)) { if ($line -match $Pattern) { return $true } }
            return $false
        }
        foreach ($line in (Get-RunLogLinesSince -Tail $tail -Since $Since)) {
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

# 操作の対象の窓の矩形。取れない＝窓が先に閉じたので、理由の 1 行で 1（実走の失敗）
function Get-TargetRect {
    param([pscustomobject]$Window)
    $rect = Get-WindowRectObject -Hwnd $Window.Hwnd
    if (-not $rect) { Stop-Run $EXIT_FAIL "the $($Window.Kind) window $($Window.Key) closed before the checks finished (GetWindowRect failed)." }
    return $rect
}

function Format-Hwnd { param([IntPtr]$Hwnd) return ('0x{0:X}' -f [int64]$Hwnd) }

function Test-ExTransparent {
    param([IntPtr]$Hwnd)
    $ex = [int64][AkFollowupW32]::GetWindowLongPtr($Hwnd, $GWL_EXSTYLE)
    return [pscustomobject]@{ Ex = ('0x{0:X}' -f $ex); Transparent = (($ex -band $WS_EX_TRANSPARENT) -ne 0) }
}

# 自分の検査（-SelfTest で判定器の較正の前に回す）。1 検査 1 行を ASCII で出し、赤の数を返す。
function Invoke-FollowupSelfChecks {
    $checks = [ordered]@{
        'balloon-follow-sources' = {
            # 求められた操作が 1 つでも 0 でも配列のまま数える（StrictMode で 1 つの文字列の .Count を引かない）
            $done = @('check=drag step=status status=done reason=-')
            foreach ($case in @(@{ Req = @('clickthrough', 'drag', 'balloon_follow'); Ready = $true; NotDone = '' }
                                @{ Req = @('drag', 'dpi', 'balloon_follow'); Ready = $false; NotDone = 'dpi' }
                                @{ Req = @('dpi', 'balloon_follow'); Ready = $false; NotDone = 'dpi' }
                                @{ Req = @('balloon_follow'); Ready = $false; NotDone = '' })) {
                $r = Get-BalloonFollowSources -Requested $case.Req -ProbeLines $done
                if ($r.Ready -ne $case.Ready -or ($r.NotDone -join ',') -ne $case.NotDone) {
                    throw "$($case.Req -join ','): ready=$($r.Ready) not_done='$($r.NotDone -join ',')' (want $($case.Ready) / '$($case.NotDone)')"
                }
            }
        }
        'judge-exit-map' = {
            foreach ($pair in @(@(0, 0), @(1, 1), @(2, 2), @(3, 1), @(4, 1), @(-1, 1), @(255, 1))) {
                $got = Convert-FollowupJudgeExit -Code $pair[0]
                if ($got -ne $pair[1]) { throw "judge exit $($pair[0]) mapped to $got (want $($pair[1]))" }
            }
        }
        'incremental-log-reader' = {
            $dir = Join-Path $repoRoot ('target\perf-loop-tmp\followup-selftest-' + [Guid]::NewGuid().ToString('N'))
            $null = New-Item -ItemType Directory -Force -Path $dir
            try {
                $path = Join-Path $dir 'run.log'
                $tail = New-RunLogTail -Path $path
                $nihon = "$([char]0x65E5)$([char]0x672C)$([char]0x8A9E)"
                $jp = [Text.Encoding]::UTF8.GetBytes($nihon)
                $add = {
                    param([byte[]]$Bytes)
                    $s = [IO.File]::Open($path, [IO.FileMode]::Append, [IO.FileAccess]::Write, [IO.FileShare]::ReadWrite)
                    try { $s.Write($Bytes, 0, $Bytes.Length) } finally { $s.Dispose() }
                }
                $expect = {
                    param([string]$Label, [string[]]$Want)
                    $got = @(Read-RunLogTail -Tail $tail)
                    if (($got.Count -ne $Want.Count) -or (($got -join "`n") -cne ($Want -join "`n"))) {
                        throw "${Label}: got $($got.Count) lines [$($got -join '|')]"
                    }
                }
                & $expect 'no file yet' @()
                # BOM, one whole line, then a partial line without a newline
                & $add ([byte[]](@(0xEF, 0xBB, 0xBF) + [Text.Encoding]::UTF8.GetBytes("line1`npar")))
                & $expect 'first read' @('line1')
                & $expect 'no growth' @()
                # the partial line completes; a Japanese word is split inside one character
                & $add ([byte[]]([Text.Encoding]::UTF8.GetBytes("tial`n") + $jp[0..3]))
                & $expect 'partial completed' @('partial')
                & $add ([byte[]]($jp[4..($jp.Length - 1)] + [Text.Encoding]::UTF8.GetBytes("`r`nx`n")))
                & $expect 'utf-8 split across reads' @($nihon, 'x')
                & $expect 'nothing returned twice' @()
            } finally { Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue }
        }
    }
    $ng = 0
    foreach ($name in $checks.Keys) {
        try { & $checks[$name]; Write-Info "selftest ${name}: ok" }
        catch { $ng++; Write-Info ("selftest ${name}: NG " + ($_.Exception.Message -replace '[^\x20-\x7E]', '?')) }
    }
    return $ng
}

# =============================================================================
# -SelfTest（判定そのものの較正）
# =============================================================================
if ($SelfTest) {
    $python = Get-PythonCommand
    if (-not $python) {
        Write-Problem '[invoke-followup-checks] python not found (the selftest calls judge-followup.py).'
        exit $EXIT_BAD_ARGS
    }
    $selfNg = Invoke-FollowupSelfChecks
    Invoke-Judge -Python $python -Arguments @($judgePath, '--selftest')
    # 最後の行が判定器の緑の行のまま赤で終わらないよう、自分の検査の赤をその後に 1 行で言う
    if ($selfNg -gt 0) { Write-Info "selftest own checks: ng=$selfNg (see the NG lines above)" }
    if ($script:JudgeExitCode -ne 0) { exit $script:JudgeExitCode }
    exit $(if ($selfNg -gt 0) { $EXIT_FAIL } else { 0 })
}

# =============================================================================
# 引数の検証
# =============================================================================
$script:ProbeLines = @()

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
# 同じ出力先の前の回の判定を残さない（判定器が落ちた回に古い判定を RESULT 行へ写さない）
Remove-Item -LiteralPath (Join-Path $script:OutDirPath 'followup-verdict.txt'), $script:NotePath -Force -ErrorAction Ignore

# -Checks は配列でも 'a,b,c' の 1 文字列でも受ける（perf-loop.measure.ps1 は子 pwsh へ引用した
# 1 文字列で渡す＝2026-08-23 周 1 の followup が exit 3「未知の検査名 'clickthrough,drag,…'」で止まった穴）。
$requested = @()
foreach ($name in @($Checks | ForEach-Object { [string]$_ -split ',' })) {
    $trimmed = $name.Trim()
    if (-not $trimmed) { continue }
    if ($CHECK_ALL -notcontains $trimmed) {
        Stop-Run $EXIT_BAD_ARGS "-Checks has an unknown check name: '$trimmed' (allowed: $($CHECK_ALL -join ', '))"
    }
    if ($requested -notcontains $trimmed) { $requested += $trimmed }
}
if ($requested.Count -eq 0) {
    Stop-Run $EXIT_BAD_ARGS '-Checks is empty. With 0 checks the run would pass without checking anything.'
}
# 判定表の並びは固定語彙の順に揃える（出力の並びが実行順で揺れないように）
$requested = @($CHECK_ALL | Where-Object { $requested -contains $_ })

if (-not $BinDir) { $BinDir = Join-Path $repoRoot 'target\debug' }
if (-not [System.IO.Path]::IsPathFullyQualified($BinDir)) { $BinDir = Join-Path $repoRoot $BinDir }
$binDirFull = [System.IO.Path]::GetFullPath($BinDir).TrimEnd('\')
$exePath = Join-Path $binDirFull 'areka.exe'
if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
    Stop-Run $EXIT_FAIL "executable not found: $exePath (check -BinDir, or run cargo build -p areka first)"
}

if (-not $GhostRoot) { $GhostRoot = Get-NarSampleValue -Sample $SAMPLE_NAME -Key 'folder' }
if (-not [System.IO.Path]::IsPathFullyQualified($GhostRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot must be an absolute path (a relative path makes SHIORI loading fail): '$GhostRoot'"
}
$ghostRootFull = [System.IO.Path]::GetFullPath($GhostRoot).TrimEnd('\')
if (-not (Test-Path -LiteralPath $ghostRootFull -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot folder not found: $ghostRootFull"
}
if (-not $BalloonRoot) { $BalloonRoot = Get-NarSampleValue -Sample $SAMPLE_NAME -Key "balloon.$SAMPLE_BALLOON_NAME" }
if (-not [System.IO.Path]::IsPathFullyQualified($BalloonRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot must be an absolute path: '$BalloonRoot'"
}
$balloonRootFull = [System.IO.Path]::GetFullPath($BalloonRoot).TrimEnd('\')
if (-not (Test-Path -LiteralPath $balloonRootFull -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot folder not found: $balloonRootFull"
}
if ($ExitMs -lt 20000) {
    Stop-Run $EXIT_BAD_ARGS "-ExitMs is too short ($ExitMs ms). It is not enough for the show-ready point and the 4 checks (use 20000 ms or more)."
}

$python = Get-PythonCommand
if (-not $python) {
    Stop-Run $EXIT_BAD_ARGS 'python not found (judge-followup.py does the judging).'
}

Write-Info "[invoke-followup-checks] version $SCRIPT_VERSION / executable $exePath"
Write-Info "[invoke-followup-checks] output $($script:OutDirPath) / checks $($requested -join ',') / auto-exit ${ExitMs}ms"

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
Write-Note "executable: $exePath"
Write-Note "checks    : $($requested -join ', ')"

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
        Stop-Run $EXIT_FAIL "could not start areka ($exePath): $($_.Exception.Message)"
    }
    if ($null -eq $proc) { Stop-Run $EXIT_FAIL "could not start areka ($exePath)." }
    Write-Info "[invoke-followup-checks] started (process ID $($proc.Id)). Waiting for the show-ready point."

    # --- 表示成立点を待つ（ここから先が観測の前提）------------------------------
    $showReady = Wait-RunLogPattern -Path $runLogPath -Since $launchedAt -Process $proc `
        -Pattern $LOG_MARKER_SHOW_READY -TimeoutMs $SHOW_READY_TIMEOUT_MS
    if (-not $showReady) {
        Write-Probe -Check 'session' -Step 'show_ready' -Fields @("waited_ms=$SHOW_READY_TIMEOUT_MS", 'result=timeout')
        if ($proc.HasExited) { Stop-Run $EXIT_FAIL "areka exited before the show-ready point (exit code $($proc.ExitCode)). Check $errLogPath and $runLogPath." }
        Stop-Run $EXIT_FAIL "the show-ready point (apply(ShowSurface)) did not appear within $SHOW_READY_TIMEOUT_MS ms. Check $runLogPath."
    }
    Start-Sleep -Milliseconds 1500   # 2 キャラ分の初期配置が落ち着くまで
    Write-Probe -Check 'session' -Step 'show_ready' -Fields @("waited_ms=$SHOW_READY_TIMEOUT_MS", 'result=ok')

    # --- 操作を注入できるかを実際に試す（推測しない）---------------------------
    $cursorBefore = New-Object AkFollowupW32+POINT
    $null = [AkFollowupW32]::GetCursorPos([ref]$cursorBefore)
    # 右下の角にカーソルが居ると +7,+5 は画面の端で止められて動かない。動かなければ逆向きでもう 1 度だけ試す
    foreach ($delta in @(@(7, 5), @(-7, -5))) {
        $setRet = [AkFollowupW32]::SetCursorPos($cursorBefore.X + $delta[0], $cursorBefore.Y + $delta[1])
        Start-Sleep -Milliseconds 60
        $cursorAfter = New-Object AkFollowupW32+POINT
        $null = [AkFollowupW32]::GetCursorPos([ref]$cursorAfter)
        $cursorMoved = ($cursorAfter.X -ne $cursorBefore.X) -or ($cursorAfter.Y -ne $cursorBefore.Y)
        if ($cursorMoved) { break }
    }
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
    Write-Note "input injection: $injection (SetCursorPos ret=$setRet moved=$cursorMoved / SendInput sent=$sent lasterr=$sendErr)"
    Write-Info "[invoke-followup-checks] input injection: $injection (cursor=$cursorOk input=$inputOk)"

    # --- 対象の窓を決める -------------------------------------------------------
    # キャラ窓とバルーン窓はクラス名も表題も同じ（実測: どちらも wintf-winmsg-executor）。
    # 見分けは本番のログしか持っていないので、[transition] の hwnd と win_kind の対で引く。
    $identity = @{}
    foreach ($line in (Get-RunLogLinesSince -Tail (New-RunLogTail -Path $runLogPath) -Since $launchedAt)) {
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
    Write-Note "PID $targetPid has $($windows.Count) visible windows ($(@($windows | Where-Object { $_.Kind -ne '-' }).Count) of known kind)"

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
        Write-Note 'could not identify the character window and balloon window pair (no win_kind in [transition]).'
    } else {
        Write-Probe -Check 'windows' -Step 'target' -Fields @(
            "scope=$($charWin.Scope)", "char_hwnd=$($charWin.Key)", "balloon_hwnd=$($balloonWin.Key)"
        )
        Write-Note "target: character window $($charWin.Key) / balloon window $($balloonWin.Key) (scope=$($charWin.Scope))"

        # =====================================================================
        # 検査 1: clickthrough
        # =====================================================================
        if ($requested -contains 'clickthrough') {
            if (-not $cursorOk) {
                Write-Probe -Check 'clickthrough' -Step 'status' -Fields @('status=unavailable', 'reason=cursor_injection_unavailable')
                Write-Note 'clickthrough: inconclusive, the cursor cannot be moved (SetCursorPos was refused).'
            } else {
                $begin = [DateTime]::UtcNow
                $rect = Get-TargetRect -Window $charWin
                $points = @(
                    @{ Name = 'transparent'; X = $rect.X + $TRANSPARENT_INSET_PX; Y = $rect.Y + $TRANSPARENT_INSET_PX; Expect = 'true' },
                    @{ Name = 'opaque'; X = $rect.X + [int]($rect.W / 2); Y = $rect.Y + $rect.H - $OPAQUE_BOTTOM_PX; Expect = 'false' }
                )
                $allLanded = $true
                foreach ($point in $points) {
                    $null = [AkFollowupW32]::SetCursorPos($point.X, $point.Y)
                    Start-Sleep -Milliseconds 80
                    $now = New-Object AkFollowupW32+POINT
                    $null = [AkFollowupW32]::GetCursorPos([ref]$now)
                    $landed = ($now.X -eq $point.X) -and ($now.Y -eq $point.Y)
                    if (-not $landed) { $allLanded = $false }
                    Write-Probe -Check 'clickthrough' -Step 'probe_point' -Fields @(
                        "hwnd=$($charWin.Key)", "point=$($point.Name)", "x=$($point.X)", "y=$($point.Y)",
                        "moved=$($landed.ToString().ToLower())"
                    )
                    Start-Sleep -Milliseconds $SETTLE_MS
                    # 窓が先に閉じていれば読んだ様式は 0（＝不透明）になり、本物の不一致に見える。閉じていれば 1 で止める
                    $null = Get-TargetRect -Window $charWin
                    $ex = Test-ExTransparent -Hwnd $charWin.Hwnd
                    $observed = $ex.Transparent.ToString().ToLower()
                    $result = if ($observed -eq $point.Expect) { 'match' } else { 'mismatch' }
                    Write-Probe -Check 'clickthrough' -Step 'read' -Fields @(
                        "hwnd=$($charWin.Key)", "point=$($point.Name)", "ex=$($ex.Ex)",
                        "transparent=$observed", "expected=$($point.Expect)", "result=$result"
                    )
                    Write-Note "clickthrough $($point.Name) point ($($point.X),$($point.Y)): ex=$($ex.Ex) transparent=$observed expect=$($point.Expect) -> $result"
                }
                $null = [AkFollowupW32]::SetCursorPos($cursorBefore.X, $cursorBefore.Y)
                Start-Sleep -Milliseconds $SETTLE_MS
                Write-Probe -Check 'clickthrough' -Step 'window' -Fields @(
                    "begin_t=$($begin.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture))Z",
                    "end_t=$(Get-ProbeStamp)"
                )
                # 点が画面の外で止められたら、別の画素を読んだ結果を不一致と言わない（drag と同じ決まり）
                if ($allLanded) { Write-Probe -Check 'clickthrough' -Step 'status' -Fields @('status=done', 'reason=-') }
                else { Write-Probe -Check 'clickthrough' -Step 'status' -Fields @('status=unavailable', 'reason=cursor_did_not_reach_target') }
            }
        }

        # =====================================================================
        # 検査 2: drag
        # =====================================================================
        if ($requested -contains 'drag') {
            if (-not $inputOk) {
                Write-Probe -Check 'drag' -Step 'status' -Fields @('status=unavailable', 'reason=input_injection_unavailable')
                Write-Note 'drag: inconclusive, input cannot be injected (SendInput was refused). Faked window messages do not take the same path as a real drag, so they are not used instead.'
            } else {
                $begin = [DateTime]::UtcNow
                foreach ($pair in @(@($charWin, 'char'), @($balloonWin, 'balloon'))) {
                    $rect = Get-TargetRect -Window $pair[0]
                    Write-Probe -Check 'drag' -Step 'rect' -Fields @(
                        'phase=before', "hwnd=$($pair[0].Key)", "win_kind=$($pair[1])",
                        "x=$($rect.X)", "y=$($rect.Y)", "w=$($rect.W)", "h=$($rect.H)"
                    )
                }
                $charRect = Get-TargetRect -Window $charWin
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
                    $rect = Get-TargetRect -Window $pair[0]
                    Write-Probe -Check 'drag' -Step 'rect' -Fields @(
                        'phase=after', "hwnd=$($pair[0].Key)", "win_kind=$($pair[1])",
                        "x=$($rect.X)", "y=$($rect.Y)", "w=$($rect.W)", "h=$($rect.H)"
                    )
                }
                Write-Probe -Check 'drag' -Step 'window' -Fields @(
                    "begin_t=$($begin.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture))Z",
                    "end_t=$(Get-ProbeStamp)"
                )
                if ($landed) {
                    Write-Probe -Check 'drag' -Step 'status' -Fields @('status=done', 'reason=-')
                } else {
                    Write-Probe -Check 'drag' -Step 'status' -Fields @('status=unavailable', 'reason=cursor_did_not_reach_target')
                }
                Write-Note "drag: ($fromX,$fromY) -> ($toX,$toY) cursor_reached=$landed"
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
            Write-Note "monitors $($monitors.Count), distinct DPI $($distinctDpi -join ',')"

            if ($distinctDpi.Count -lt 2) {
                Write-Probe -Check 'dpi' -Step 'status' -Fields @('status=unavailable', 'reason=single_dpi')
                Write-Note 'dpi: inconclusive, there are not 2 or more monitors with different DPI.'
            } else {
                $begin = [DateTime]::UtcNow
                $homeRect = Get-TargetRect -Window $charWin
                foreach ($pair in @(@($charWin, 'char'), @($balloonWin, 'balloon'))) {
                    $rect = Get-TargetRect -Window $pair[0]
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
                    $rect = Get-TargetRect -Window $pair[0]
                    Write-Probe -Check 'dpi' -Step 'rect' -Fields @(
                        'phase=after', "hwnd=$($pair[0].Key)", "win_kind=$($pair[1])",
                        "x=$($rect.X)", "y=$($rect.Y)", "w=$($rect.W)", "h=$($rect.H)"
                    )
                }
                Write-Probe -Check 'dpi' -Step 'window' -Fields @(
                    "begin_t=$($begin.ToString('yyyy-MM-ddTHH:mm:ss.ffffff', [Globalization.CultureInfo]::InvariantCulture))Z",
                    "end_t=$(Get-ProbeStamp)"
                )
                Write-Probe -Check 'dpi' -Step 'status' -Fields @('status=done', 'reason=-')
                Write-Note "dpi: moved $homeDpi -> $($away.Dpi) and back (out=$moveOk back=$backOk)"
            }
        }

        # =====================================================================
        # 検査 4: balloon_follow（drag / dpi の前後の矩形を使う・自前の操作は無い）
        # =====================================================================
        if ($requested -contains 'balloon_follow') {
            $follow = Get-BalloonFollowSources -Requested $requested -ProbeLines $script:ProbeLines
            if ($follow.Sources.Count -eq 0) {
                Write-Probe -Check 'balloon_follow' -Step 'status' -Fields @('status=unavailable', 'reason=no_required_mover')
                Write-Note 'balloon_follow: inconclusive, neither drag nor dpi is in the required checks (nothing moves the windows to compare).'
            } elseif (-not $follow.Ready) {
                Write-Probe -Check 'balloon_follow' -Step 'status' -Fields @('status=unavailable', 'reason=depends_on_drag_dpi')
                Write-Note "balloon_follow: inconclusive, the operations to compare before and after ($($follow.NotDone -join ', ')) did not complete."
            } else {
                Write-Probe -Check 'balloon_follow' -Step 'status' -Fields @('status=done', 'reason=-')
            }
        }
    }

    # --- 有界自動終了を待つ（正規の終了経路を使い、ログを最後まで書かせる）-------
    Write-Info '[invoke-followup-checks] checks done. Waiting for the bounded auto-exit.'
    $deadline = $launchedAt.AddMilliseconds($ExitMs).AddSeconds($EXIT_MARGIN_SEC)
    while (-not $proc.HasExited) {
        if ([DateTime]::UtcNow -ge $deadline) {
            $stopped = Stop-ChildProcess -Process $proc -WaitMs $KILL_WAIT_MS
            Write-Note "warning: killed the process tree because the bounded auto-exit (${ExitMs}ms + margin ${EXIT_MARGIN_SEC}s) passed (all exited=$stopped)."
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
    # 途中で止まった道でも、子孫（32bit の SHIORI ヘルパ）まで止めて終わるまで待つ
    if ($proc -and -not (Stop-ChildProcess -Process $proc -WaitMs $KILL_WAIT_MS)) {
        Write-Problem "[invoke-followup-checks] areka (process ID $($proc.Id)) or its descendants did not exit within $KILL_WAIT_MS ms."
    }
}

# =============================================================================
# 判定
# =============================================================================
Set-Content -LiteralPath $script:NotePath -Value ($script:NoteLines -join [Environment]::NewLine) -Encoding utf8

Invoke-Judge -Python $python -Arguments @($judgePath, $script:OutDirPath)
$overallCode = Convert-FollowupJudgeExit -Code $script:JudgeExitCode

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
