# measure-ssp-rects.ps1 — キャラ窓矩形の読み取り専用計測（areka-P0-scope-chain-gap R1 / R6.4）
# DPI aware (Per-Monitor v2) プロセスとして対象プロセスの全トップレベル可視窓の物理 px 矩形を取得する。
# 対象プロセス名は -ProcessName で指定する（既定 `ssp` ＝参照実装。引数なしの従来動作は不変）。
# 書き込み・操作は一切行わない（GetWindowRect / GetClassName / GetWindowText / GetDpiForWindow のみ）。
#
# 使い方:
#   スナップショット 1 回:  pwsh -File measure-ssp-rects.ps1
#   ポーリング（起動観測）: pwsh -File measure-ssp-rects.ps1 -PollSeconds 30 -IntervalMs 50 -LogPath rects.log
#     → 変化があった tick のみログへ追記。対象プロセス起動前に開始し、起動直後の初期配置→boot script の move までの時系列を捕捉する。
#   areka 本体を対象にする（実機受け入れ C5 の従判定・R6.4）:
#     pwsh -File measure-ssp-rects.ps1 -ProcessName areka
#     pwsh -File measure-ssp-rects.ps1 -ProcessName areka -PollSeconds 30 -IntervalMs 50
#     → -LogPath 省略時の既定ログ名は対象プロセス名を含む（例 areka-rects-poll-YYYYMMDD-HHMMSS.log）。
param(
    [int]$PollSeconds = 0,
    [int]$IntervalMs = 50,
    [string]$LogPath = "",
    # 窓矩形を採取する対象のプロセス名（拡張子なし・Get-Process -Name と同じ表記）
    [string]$ProcessName = "ssp"
)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public class W32 {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder sb, int max);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr hWnd, uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool GetMonitorInfo(IntPtr hMon, ref MONITORINFO mi);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MONITORINFO {
        public int cbSize; public RECT rcMonitor; public RECT rcWork; public uint dwFlags;
    }
}
"@

# DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4 （これ以降の GetWindowRect は物理 px）
[void][W32]::SetProcessDpiAwarenessContext([IntPtr]-4)

function Get-TargetWindows {
    # $Name: 対象プロセス名（既定は呼び出し元の -ProcessName ＝ ssp）
    param([string]$Name = $ProcessName)
    $pids = @(Get-Process -Name $Name -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
    if ($pids.Count -eq 0) { return @() }
    $result = New-Object System.Collections.Generic.List[object]
    $cb = [W32+EnumWindowsProc]{
        param($hWnd, $lParam)
        $winPid = [uint32]0
        [void][W32]::GetWindowThreadProcessId($hWnd, [ref]$winPid)
        if ($pids -contains [int]$winPid -and [W32]::IsWindowVisible($hWnd)) {
            $r = New-Object W32+RECT
            [void][W32]::GetWindowRect($hWnd, [ref]$r)
            $cls = New-Object System.Text.StringBuilder 256
            [void][W32]::GetClassName($hWnd, $cls, 256)
            $txt = New-Object System.Text.StringBuilder 256
            [void][W32]::GetWindowText($hWnd, $txt, 256)
            $dpi = [W32]::GetDpiForWindow($hWnd)
            $mi = New-Object W32+MONITORINFO
            $mi.cbSize = [System.Runtime.InteropServices.Marshal]::SizeOf($mi)
            $hMon = [W32]::MonitorFromWindow($hWnd, 2) # MONITOR_DEFAULTTONEAREST
            [void][W32]::GetMonitorInfo($hMon, [ref]$mi)
            $result.Add([pscustomobject]@{
                HWnd  = ('0x{0:X}' -f $hWnd.ToInt64())
                Class = $cls.ToString()
                Title = $txt.ToString()
                Dpi   = $dpi
                L = $r.Left; T = $r.Top; R = $r.Right; B = $r.Bottom
                W = $r.Right - $r.Left; H = $r.Bottom - $r.Top
                WorkArea = ('L={0} T={1} R={2} B={3}' -f $mi.rcWork.Left, $mi.rcWork.Top, $mi.rcWork.Right, $mi.rcWork.Bottom)
            })
        }
        return $true
    }
    [void][W32]::EnumWindows($cb, [IntPtr]::Zero)
    return $result
}

function Format-Snapshot($wins) {
    if ($wins.Count -eq 0) { return "(no visible $ProcessName windows)" }
    ($wins | Sort-Object Class, L | Format-Table HWnd, Class, Title, Dpi, L, T, W, H, WorkArea -AutoSize | Out-String -Width 400).TrimEnd()
}

if ($PollSeconds -le 0) {
    $ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fff'
    "=== snapshot $ts ==="
    Format-Snapshot (Get-TargetWindows)
} else {
    # 既定ログ名は対象プロセス名を含む（既定 ssp では従来どおり ssp-rects-poll-*.log）
    if (-not $LogPath) { $LogPath = Join-Path $PSScriptRoot ('{0}-rects-poll-{1}.log' -f $ProcessName, (Get-Date -Format 'yyyyMMdd-HHmmss')) }
    "polling $ProcessName ${PollSeconds}s @ ${IntervalMs}ms -> $LogPath"
    $deadline = [DateTime]::UtcNow.AddSeconds($PollSeconds)
    $prev = ""
    while ([DateTime]::UtcNow -lt $deadline) {
        $wins = Get-TargetWindows
        # 変化検知キー: 全窓の class+rect の連結
        $key = ($wins | Sort-Object Class, HWnd | ForEach-Object { "$($_.Class)|$($_.L),$($_.T),$($_.W),$($_.H)" }) -join ';'
        if ($key -ne $prev) {
            $ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fff'
            $block = "=== change $ts ===`r`n" + (Format-Snapshot $wins)
            Add-Content -Path $LogPath -Value $block -Encoding utf8
            $prev = $key
        }
        Start-Sleep -Milliseconds $IntervalMs
    }
    "done -> $LogPath"
}
