<#
.SYNOPSIS
  collisionex-regions タスク 6.1: 起こした areka のクローディアの本体の窓へ、SendInput で撫でとドラッグを送る。

.DESCRIPTION
  -AppPid で渡した areka（real-machine-run.ps1 -Run R1 で起こしたもの）の見えている窓のうち、
  幅÷333 と高さ÷500 が同じ拡大率 k になる窓を本体と相方の候補にし、いちばん右を本体とする
  （相方は本体の左隣に並ぶ）。点はサーフェスの座標で先に決めてあり、画面の点 = 窓の左上 + 点 × k。
  送る前に、その画面の点が選んだ窓（-AppPid の窓）の矩形の中にあることを確かめ、外なら何も送らずに止まる。
  areka のキャラクター窓は WS_EX_TRANSPARENT 付きで、wintf の clickthrough の監視（GetCursorPos を 12ms ごと）が
  カーソルが不透明な画素の上に来たときだけそれを外す。なので WindowFromPoint は動かす前には areka の窓を返さない。
  点へ動かして 150ms 待った後に WindowFromPoint の最上位の窓が選んだ本体の窓（同じ pid のバルーンでもない）であることを確かめ、違えばボタンも撫でも送らずに止まる。
  プロセスは Per-Monitor v2 にして、座標はすべて物理の点で扱う。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/collisionex-regions/real-machine-drive.ps1 -AppPid 12345
#>
param([Parameter(Mandatory)][int]$AppPid)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class Cx {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] struct MOUSEINPUT { public int dx, dy; public uint data, flags, time; public IntPtr extra; }
  // x64 の INPUT は 40 バイト（type 4 + 詰め 4 + MOUSEINPUT 32・共用体は 8 の位置）。余分な欄を足すと SendInput が拒む。
  [StructLayout(LayoutKind.Sequential)] struct INPUT { public uint type; public MOUSEINPUT mi; }
  public static int InputSize() { return Marshal.SizeOf(typeof(INPUT)); }
  public static int UnionOffset() { return (int)Marshal.OffsetOf(typeof(INPUT), "mi"); }
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr h, uint f);
  [DllImport("user32.dll")] static extern int GetSystemMetrics(int i);
  [DllImport("user32.dll")] static extern uint SendInput(uint n, INPUT[] i, int size);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr c);
  [DllImport("user32.dll")] static extern IntPtr GetThreadDpiAwarenessContext();
  [DllImport("user32.dll")] static extern bool AreDpiAwarenessContextsEqual(IntPtr a, IntPtr b);
  [DllImport("user32.dll", SetLastError = true)] static extern IntPtr OpenInputDesktop(uint flags, bool inherit, uint access);
  [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern bool GetUserObjectInformationW(IntPtr h, int index, System.Text.StringBuilder buf, int len, out int need);
  // 入力を受けている desktop の名前。開けなければ null（スクリーンセーバー・ロック・UAC の画面の間など）。
  public static string InputDesktopName() {
    var h = OpenInputDesktop(0, false, 0x0001); // DESKTOP_READOBJECTS
    if (h == IntPtr.Zero) return null;
    try { var b = new System.Text.StringBuilder(256); int n; return GetUserObjectInformationW(h, 2, b, 512, out n) ? b.ToString() : null; } // UOI_NAME
    finally { CloseDesktop(h); }
  }
  public static bool IsPmV2() { return AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(), new IntPtr(-4)); }
  public static List<IntPtr> Windows(uint pid) {
    var list = new List<IntPtr>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) list.Add(h); return true; }, IntPtr.Zero);
    return list;
  }
  // 点の上の最上位の窓（GA_ROOT）。持ち主の pid も返す。
  public static IntPtr RootAt(int x, int y, out uint pid) {
    var h = GetAncestor(WindowFromPoint(new POINT { X = x, Y = y }), 2);
    GetWindowThreadProcessId(h, out pid); return h;
  }
  // 絶対座標（仮想デスクトップ）の 1 件。0x8000 ABSOLUTE | 0x4000 VIRTUALDESK | 0x0001 MOVE
  public static void Send(int x, int y, uint extraFlags) {
    int vx = GetSystemMetrics(76), vy = GetSystemMetrics(77), vw = GetSystemMetrics(78), vh = GetSystemMetrics(79);
    var i = new INPUT { type = 0 };
    i.mi.dx = (int)Math.Round((x - vx) * 65535.0 / (vw - 1));
    i.mi.dy = (int)Math.Round((y - vy) * 65535.0 / (vh - 1));
    i.mi.flags = 0x8000 | 0x4000 | 0x0001 | extraFlags;
    if (SendInput(1, new[] { i }, Marshal.SizeOf(typeof(INPUT))) != 1) throw new Exception("SendInput failed");
  }
}
'@
[Cx]::SetProcessDpiAwarenessContext([IntPtr]-4) | Out-Null  # PER_MONITOR_AWARE_V2（既に決まっていれば失敗するので、下で今の状態を確かめる）
if (-not [Cx]::IsPmV2()) { throw 'Per-Monitor v2 になっていない（座標が物理の点でない）。送らずに止める' }
if ([Cx]::InputSize() -ne 40 -or [Cx]::UnionOffset() -ne 8) { throw "INPUT の大きさ $([Cx]::InputSize())・共用体の位置 $([Cx]::UnionOffset())（x64 は 40・8）。送らずに止める" }
function Desk {
    # スクリーンセーバーの間・ロックの間は SendInput が通らない（4 回目）。どちらかなら何も送らずに止める。
    $scr = Get-Process | Where-Object { $_.Path -like '*.scr' -or $_.ProcessName -like '*.scr' }
    if ($scr) { throw "スクリーンセーバーが動いている（$(($scr | ForEach-Object { "$($_.ProcessName) pid=$($_.Id)" }) -join ', ')）。送らずに止める" }
    $d = [Cx]::InputDesktopName()
    if ($d -ne 'Default') { throw "入力の desktop が Default でない（$(if ($d) { $d } else { '開けない' })）。送らずに止める" }
}
Desk
# wintf のドラッグの閾値は既定 5 物理点（wintf ecs/drag/mod.rs の DragConfig::default・キャラ窓は areka-placement spawn.rs で既定のまま）。
$dragStep = 8; $dragSteps = 10
if ($dragStep * $dragSteps -le 5) { throw 'ドラッグの距離が閾値 5 を超えない' }

# クローディアの surface0（333x500）の点（サーフェスの座標）。どれも surface0.png の不透明な画素（A=255）。
$points = [ordered]@{
    Head  = @(165, 70)    # collisionex1 rect 106,31,225,108 の中
    Bust  = @(165, 213)   # collisionex3 rect 131,188,200,238 の中
    Skirt = @(160, 436)   # collisionex8 polygon だけ（rect は y<=288）
    Foot  = @(152, 483)   # collisionex6 ellipse 142,472,163,494 だけ
    Fan   = @(262, 232)   # collisionex0 polygon だけ（Hair rect 200..250 の右）
}

function Body {
    $c = foreach ($h in [Cx]::Windows($AppPid)) {
        $r = New-Object Cx+RECT; [Cx]::GetWindowRect($h, [ref]$r) | Out-Null
        $w = $r.R - $r.L; $hh = $r.B - $r.T
        if ($w -gt 0 -and [math]::Abs($w / 333 - $hh / 500) -lt 0.01) { [pscustomobject]@{ H = $h; L = $r.L; T = $r.T; K = $w / 333; W = $w; Ht = $hh } }
    }
    if (-not $c) { throw "333x500 の比の窓が pid=$AppPid に無い" }
    $c | ForEach-Object { "  candidate hwnd=$($_.H) rect=$($_.L),$($_.T) $($_.W)x$($_.Ht) k=$($_.K)" } | Write-Host
    $c | Sort-Object L -Descending | Select-Object -First 1
}
function Screen($b, [string]$name) {
    $p = $points[$name]
    @([int][math]::Round($b.L + $p[0] * $b.K), [int][math]::Round($b.T + $p[1] * $b.K))
}
function Guard($b, $s, [string]$name) {
    Desk
    # 1) 点が選んだ窓の矩形の中（窓は -AppPid の EnumWindows から選んだもの）
    if ($s[0] -lt $b.L -or $s[0] -ge $b.L + $b.W -or $s[1] -lt $b.T -or $s[1] -ge $b.T + $b.Ht) {
        throw "$name の点 $($s -join ',') が窓 hwnd=$($b.H) の矩形の外。送らずに止める"
    }
    # 2) 動かして待つと areka が WS_EX_TRANSPARENT を外し、点の上の最上位の窓が -AppPid になる
    [Cx]::Send($s[0], $s[1], 0); Start-Sleep -Milliseconds 150
    $o = 0; $h = [Cx]::RootAt($s[0], $s[1], [ref]$o)
    # 同じ pid のバルーンの窓が重なっていても通さない（選んだ本体の窓そのものであること）
    if ($o -ne $AppPid -or $h -ne $b.H) { throw "$name の点 $($s -join ',') へ動かした後の最上位の窓が hwnd=$h pid=$o（本体 hwnd=$($b.H) pid=$AppPid でない）。ボタンも撫でも送らずに止める" }
}
function Stroke([string]$name) {
    $b = Body; $s = Screen $b $name; Guard $b $s $name
    Write-Host "stroke $name surface=$($points[$name] -join ',') screen=$($s -join ',') k=$($b.K) $(Get-Date -Format HH:mm:ss.fff)"
    for ($i = 0; $i -lt 30; $i++) { [Cx]::Send($s[0] + (($i % 2) * 2 - 1) * 8, $s[1], 0); Start-Sleep -Milliseconds 40 }
    [Cx]::Send($s[0], $s[1], 0); Start-Sleep -Milliseconds 1500
}
function Drag([string]$name, [int]$dir) {
    $b = Body; $s = Screen $b $name; Guard $b $s $name
    Write-Host "drag $name dir=$dir surface=$($points[$name] -join ',') screen=$($s -join ',') k=$($b.K) $(Get-Date -Format HH:mm:ss.fff)"
    [Cx]::Send($s[0], $s[1], 0); Start-Sleep -Milliseconds 300
    [Cx]::Send($s[0], $s[1], 0x0002); Start-Sleep -Milliseconds 200   # LEFTDOWN
    for ($i = 1; $i -le $dragSteps; $i++) { [Cx]::Send($s[0] + $dir * $dragStep * $i, $s[1], 0); Start-Sleep -Milliseconds 40 }
    Start-Sleep -Milliseconds 300
    [Cx]::Send($s[0] + $dir * $dragStep * $dragSteps, $s[1], 0x0004); Start-Sleep -Milliseconds 2000  # LEFTUP
}

Stroke Head
Stroke Skirt
Stroke Foot
Stroke Fan
# 左へ・右へを交互にして、本体の窓をもとの近くへ戻す。
Drag Bust -1
Drag Skirt 1
Drag Head -1
Drag Foot 1
# 終わったらカーソルを窓の外（本体の右上の外）へ逃がす。
$b = Body; [Cx]::Send($b.L + $b.W + 40, [math]::Max($b.T - 40, 0), 0)
"drive done $(Get-Date -Format HH:mm:ss.fff)"
