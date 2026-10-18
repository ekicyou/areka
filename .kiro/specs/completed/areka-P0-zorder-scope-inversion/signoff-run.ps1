<#
.SYNOPSIS
  areka-P0-zorder-scope-inversion の実機の確かめの走行（要件 8.4・8.5・8.6）。手順は signoff-procedure.md。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2（vendors\sample_ghost\emo2.nar）を
             nar-sample-path で展開して、走行の種類ごとに target\zorder-inversion-signoff\<Case>\root へ写す。
             起動の台本（boot.pasta の起動の各場面の最初のむらさきの発話）へ種類ごとの指定を差し込む。
             検体は派生物でコミットしない（このスクリプトでの再生成が正本）。
  -Run <Case> : その種類の根で areka を有界の時間で起こし、ログを <Case>\run.log、前面走査を
             <Case>\probe.txt、操作の控えを <Case>\steps.txt に採る。
  -Press   : （-Run inverted のときだけ・開発者の許可を得て走行の間だけ）SendInput で s0 を押す・b0 を押す・
             s0 を掴んで動かす・他のアプリ（このプロセスが出す窓）を押す・s1 を押して戻る。各操作の後に前面走査を採る。
  入力デスクトップが Default でないときは操作せず止まる。

.EXAMPLE
  pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Prepare
  pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-zorder-scope-87951c\.kiro\specs\areka-P0-zorder-scope-inversion\signoff-run.ps1 -Run inverted -Press
#>
param(
    [switch]$Prepare,
    [ValidateSet('inverted', 'explicit', 'numeric', 'default')]
    [string]$Run,
    [switch]$Press,
    [int]$ExitMs = 0,
    [int]$Port = 39875,
    [string]$RustLog = 'info,wintf::ecs::window=debug'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (git -C $PSScriptRoot rev-parse --show-toplevel)).Path
$base = Join-Path $wt 'target\zorder-inversion-signoff'

# 種類ごとの起動の台本への差し込み（ぱすたと同じく解除してから指定する）。default は差し込まない。
$TAGS = @{
    inverted = '\![reset,zorder]\![set,zorder,s0,b0,s1]'
    explicit = '\![reset,zorder]\![set,zorder,b0,s0,b1,s1]'
    numeric  = '\![reset,zorder]\![set,zorder,0,1]'
    default  = ''
}

function Machine([string]$exe) {
    $b = [IO.File]::ReadAllBytes($exe)
    $pe = [BitConverter]::ToInt32($b, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

if ($Prepare) {
    Push-Location $wt
    try {
        cargo build -p areka --bin areka -j 4; if ($LASTEXITCODE) { throw "areka build failed ($LASTEXITCODE)" }
        cargo build -p shiori-host32-helper --target i686-pc-windows-msvc -j 4; if ($LASTEXITCODE) { throw "helper build failed ($LASTEXITCODE)" }
        $printed = cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2
        if ($LASTEXITCODE) { throw "nar-sample-path failed ($LASTEXITCODE)" }
    } finally { Pop-Location }
    $manual = ($printed | Where-Object { $_ -like 'root=*' }) -replace '^root=', ''
    $helper = Join-Path $wt 'target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe'
    if ((Machine $helper) -ne '014C') { throw "helper is not 32bit: $helper" }
    foreach ($case in 'inverted', 'explicit', 'numeric', 'default') {
        $root = Join-Path $base "$case\root"
        if (Test-Path (Join-Path $base $case)) { Remove-Item -Recurse -Force (Join-Path $base $case) }
        New-Item -ItemType Directory -Force $root | Out-Null
        Copy-Item -Recurse (Join-Path $manual '*') $root
        Copy-Item (Join-Path $wt 'target\debug\areka.exe') $root
        Copy-Item $helper $root
        $dic = Join-Path $root 'ghost\emo2\ghost\master\dic'
        # ゴースト自身のトーク（ランダムトーク・時報）を黙らせ、窓の出入りを起動の場面と操作の結果だけにする。
        foreach ($f in 'dispatch.pasta', 'hour.pasta') { Remove-Item (Join-Path $dic $f) }
        $n = 0
        if ($TAGS[$case]) {
            # 起動の各場面（＊起動朝〜＊起動halt・＊OnFirstBoot。＊OnClose より前）の最初のむらさきの発話の頭へ。
            $f = Join-Path $dic 'boot.pasta'
            $lines = [IO.File]::ReadAllText($f) -split "`r`n"
            $pending = $false
            for ($i = 0; $i -lt $lines.Count; $i++) {
                if ($lines[$i] -eq '＊OnClose') { break }
                if ($lines[$i] -match '^＊') { $pending = $true; continue }
                if ($pending -and $lines[$i] -match '^　むらさき：＠[^　]*　') {
                    $lines[$i] = $lines[$i] -replace '^(　むらさき：＠[^　]*　)', ('$1' + $TAGS[$case].Replace('$', '$$'))
                    $n++; $pending = $false
                }
            }
            # 起動の 4 帯 × 3 場面＋halt 3＋OnFirstBoot。時刻帯にも前回の終わり方にも依らず指定が出るように全部へ入れる。
            if ($n -lt 12) { throw "boot.pasta: only $n scenes got the tag (expected >= 12)" }
            [IO.File]::WriteAllText($f, ($lines -join "`r`n"), [Text.UTF8Encoding]::new($false))
        }
        "prepared $case root=$root tagged_scenes=$n areka=$(Machine (Join-Path $root 'areka.exe')) helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe'))"
    }
    "nar=$((Get-Item (Join-Path $wt 'vendors\sample_ghost\emo2.nar')).LastWriteTime.ToString('o'))"
    return
}
if (-not $Run) { return }
if ($Press -and $Run -ne 'inverted') { throw "-Press is for -Run inverted only" }

Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public static class ZiInput {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] struct MOUSEINPUT { public int dx, dy; public uint data, flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] struct INPUT { public uint type; public MOUSEINPUT mi; }
  [DllImport("user32.dll")] static extern IntPtr GetTopWindow(IntPtr h);
  [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr h, uint cmd);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", SetLastError=true)] static extern uint SendInput(uint n, INPUT[] i, int size);
  [DllImport("user32.dll", SetLastError=true)] static extern IntPtr OpenInputDesktop(uint f, bool inh, uint acc);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern bool GetUserObjectInformationW(IntPtr h, int idx, StringBuilder b, int len, out int need);
  [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr h);
  [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
  [DllImport("user32.dll")] static extern int GetSystemMetrics(int i);
  static void PerMonitor() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
  public static string InputDesktop() {
    var h = OpenInputDesktop(0, false, 0x0100); if (h == IntPtr.Zero) return "<none>";
    var sb = new StringBuilder(256); int n; GetUserObjectInformationW(h, 2, sb, 512, out n); CloseDesktop(h); return sb.ToString();
  }
  // 前面走査: 手前から奥へ（GetTopWindow → GW_HWNDNEXT）。見えている窓だけ。pid の窓は 0xHEX、他は連なりを * に畳む。
  public static string Scan(uint pid) {
    var o = new List<string>();
    for (IntPtr h = GetTopWindow(IntPtr.Zero); h != IntPtr.Zero; h = GetWindow(h, 2)) {
      if (!IsWindowVisible(h)) continue;
      uint p; GetWindowThreadProcessId(h, out p);
      string t = p == pid ? String.Format("0x{0:X}", h.ToInt64()) : "*";
      if (t == "*" && o.Count > 0 && o[o.Count - 1] == "*") continue;
      o.Add(t);
    }
    return String.Join(",", o);
  }
  public static uint OwnerPid(IntPtr h) { uint p; GetWindowThreadProcessId(h, out p); return p; }
  public static int[] Rect(IntPtr h) { PerMonitor(); RECT r; GetWindowRect(h, out r); return new[] { r.L, r.T, r.R, r.B }; }
  static INPUT Abs(uint f, int x, int y) {
    int vx = GetSystemMetrics(76), vy = GetSystemMetrics(77), vw = GetSystemMetrics(78), vh = GetSystemMetrics(79);
    var i = new INPUT(); i.type = 0; i.mi.flags = f | 0x1 | 0x4000 | 0x8000;
    i.mi.dx = (int)((x - vx) * 65535L / (vw - 1)); i.mi.dy = (int)((y - vy) * 65535L / (vh - 1)); return i;
  }
  static int Size() { return Marshal.SizeOf(typeof(INPUT)); }
  public static uint Move(int x, int y) { PerMonitor(); return SendInput(1, new[] { Abs(0, x, y) }, Size()); }
  public static uint Down(int x, int y) { PerMonitor(); return SendInput(1, new[] { Abs(0x2, x, y) }, Size()); }
  public static uint Up(int x, int y) { PerMonitor(); return SendInput(1, new[] { Abs(0x4, x, y) }, Size()); }
}
'@

$root = Join-Path $base "$Run\root"
if (-not (Test-Path (Join-Path $root 'areka.exe'))) { throw "no root for $Run. run -Prepare first" }
$dir = Join-Path $base $Run
foreach ($f in 'run.log', 'run.err.log', 'probe.txt', 'steps.txt') { Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $dir $f) }
$lad = Join-Path $dir 'localappdata'
if (Test-Path $lad) { Remove-Item -Recurse -Force $lad }
New-Item -ItemType Directory -Force $lad, (Join-Path $dir 'tmp') | Out-Null
if ($ExitMs -le 0) { $ExitMs = if ($Press) { 90000 } else { 30000 } }
if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) { throw "port $Port is in use (pass -Port)" }
$desk0 = [ZiInput]::InputDesktop()
if ($desk0 -ne 'Default') { throw "input desktop is not Default ($desk0). unlock the screen and rerun" }

Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
$env:NO_COLOR = '1'
$env:RUST_LOG = $RustLog
$env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
$env:AREKA_NO_ALERT = '1'
$env:AREKA_MCP_PORT = "$Port"
$env:AREKA_ROOT = $root
# 書ける根（%LOCALAPPDATA%\areka）と一時フォルダは子プロセスだけ target\ の下へ向ける。
$env:LOCALAPPDATA = $lad
$env:TMP = Join-Path $dir 'tmp'; $env:TEMP = $env:TMP
$log = Join-Path $dir 'run.log'
$probe = Join-Path $dir 'probe.txt'
$note = Join-Path $dir 'steps.txt'
Set-Content $note "run=$Run press=$Press desktop=$desk0 exit_ms=$ExitMs rust_log=$RustLog root=$root"
$p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$(Join-Path $root 'ghost\emo2')`"" -WorkingDirectory $root `
    -RedirectStandardOutput $log -RedirectStandardError (Join-Path $dir 'run.err.log') -WindowStyle Hidden -PassThru
$null = $p.Handle
"$Run pid=$($p.Id) start=$(Get-Date -Format o)"

function Note([string]$s) { Add-Content -LiteralPath $note -Encoding utf8 "$(Get-Date -Format o) $s" }
function Lines { @(Get-Content -LiteralPath $log -Encoding utf8 -ErrorAction SilentlyContinue) }
function WaitFor([string]$literal, [int]$sec = 60) {
    $until = (Get-Date).AddSeconds($sec)
    while ((Get-Date) -lt $until -and -not $p.HasExited) {
        $hit = Lines | Where-Object { $_.Contains($literal) } | Select-Object -Last 1
        if ($hit) { Note "hit $literal : $hit"; return $hit }
        Start-Sleep -Milliseconds 200
    }
    throw "line did not come: $literal"
}
function Pump { [System.Windows.Forms.Application]::DoEvents() }
function Probe([string]$label) {
    Pump; Start-Sleep -Milliseconds 800; Pump
    $fg = [ZiInput]::GetForegroundWindow()
    $line = "probe label=$label order=$([ZiInput]::Scan([uint32]$p.Id)) foreground=0x$('{0:X}' -f $fg.ToInt64())"
    Add-Content -LiteralPath $probe -Encoding ascii $line; Note $line
}
function Guard { if ([ZiInput]::InputDesktop() -ne 'Default') { throw "input desktop is no longer Default" } }
function Field([string]$line, [string]$name) { if ($line -match ('(?<![\w])' + [regex]::Escape($name) + '=(\S+)')) { $Matches[1] } }
# 窓の中で押下が届く点を探して押す（透過の画素は下の窓へ抜けるので、前面の窓が狙った窓になるまで点を替える）。
function PressOn([string]$hex, [string]$what, [switch]$Drag) {
    $h = [IntPtr][Convert]::ToInt64($hex, 16)
    $r = [ZiInput]::Rect($h)
    foreach ($f in @(@(0.5, 0.5), @(0.5, 0.35), @(0.45, 0.6), @(0.5, 0.2), @(0.4, 0.45), @(0.6, 0.45))) {
        Guard
        $x = [int]($r[0] + ($r[2] - $r[0]) * $f[0]); $y = [int]($r[1] + ($r[3] - $r[1]) * $f[1])
        $null = [ZiInput]::Move($x, $y); Start-Sleep -Milliseconds 300
        $null = [ZiInput]::Down($x, $y); Start-Sleep -Milliseconds 120
        if ($Drag) {
            for ($k = 1; $k -le 10; $k++) { $null = [ZiInput]::Move($x - 6 * $k, $y); Start-Sleep -Milliseconds 30 }
            $x -= 60
        }
        $null = [ZiInput]::Up($x, $y); Start-Sleep -Milliseconds 400; Pump
        $fg = [ZiInput]::GetForegroundWindow()
        Note "$what at $x,$y foreground=0x$('{0:X}' -f $fg.ToInt64())"
        if ($fg -eq $h) { return }
    }
    throw "the press did not reach $what ($hex)"
}

$ok = $true
$form = $null
try {
    if ($Run -eq 'default') { WaitFor '[zorder-pair] owner-established' | Out-Null } else { WaitFor '[zorder-chain] settled' | Out-Null }
    Start-Sleep -Milliseconds 3000
    Probe 'boot'
    if ($Press) {
        $rel = WaitFor '[zorder-chain] pair-released' 10
        $set = Lines | Where-Object { $_.Contains('[zorder-chain] settled') } | Select-Object -Last 1
        if ($set -notmatch 'declared=(\S+)') { throw "no declared list" }
        $decl = @($Matches[1] -split ',')
        if ($decl.Count -lt 3) { throw "declared list is short: $($decl -join ',')" }
        # 外した組のバルーンが b0。逆転した並びの頭 2 つが s0・b0、末尾が s1（signoff-judge.ps1 が照合し直す）。
        $b0 = Field $rel 'owned_hwnd'; $s0 = $decl[0]; $s1 = $decl[-1]
        if ($decl[1] -ne $b0) { throw "declared[1]=$($decl[1]) is not the released balloon $b0" }
        Note "s0=$s0 b0=$b0 s1=$s1 declared=$($decl -join ',')"
        PressOn $s0 'press s0'; Probe 'press-s0'
        PressOn $b0 'press b0'; Probe 'press-b0'
        PressOn $s0 'drag s0' -Drag; Probe 'drag-s0'
        # 他のアプリ: このプロセスが出す窓（areka とは別のプロセス）。
        $form = New-Object System.Windows.Forms.Form
        $form.Text = 'zorder-inversion-signoff other app'; $form.StartPosition = 'Manual'
        $form.Location = New-Object System.Drawing.Point(40, 40); $form.Size = New-Object System.Drawing.Size(480, 320)
        $form.Show(); Pump
        PressOn ('{0:X}' -f $form.Handle.ToInt64()) 'press other app'; Probe 'other-app'
        PressOn $s1 'press s1 (return)'; Probe 'returned'
    }
} catch {
    $ok = $false; Note "STOP $($_.Exception.Message)"
    "STOP: $($_.Exception.Message)"
} finally {
    if ($form) { $form.Close(); $form.Dispose() }
}
Note "driver done ok=$ok"
$p.WaitForExit()
"$Run exit=$($p.ExitCode) ok=$ok end=$(Get-Date -Format o)"
"log=$log"
"probe=$probe"
