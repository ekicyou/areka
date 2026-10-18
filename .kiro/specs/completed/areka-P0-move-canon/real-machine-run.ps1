<#
.SYNOPSIS
  areka-P0-move-canon タスク 6.2 の実機の確かめ（emo2 の「エモの位置調整」と「シェル位置初期化」）。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2（vendors\sample_ghost\emo2.nar）を
             nar-sample-path で展開して target\move-canon-signoff\root に写す。製品のコードと検体には触れない。
  -Run Adjust : 書ける根（子プロセスだけ LOCALAPPDATA を target\move-canon-signoff\<Tag>\localappdata へ向ける）を
             空にし、ゴーストの位置の記憶（ghost\emo2\ghost\master\profile\areka）を消してエモ（\1）だけ
             x=-SeedEmoX（既定 1000）へずらした記憶を置いて起こす。むらさきをダブルクリック → 「エモの位置調整」→「調整」を SendInput で選び、
             move_applied（scope=1）と窓の矩形を控える。次に右クリックのメニューの「シェル位置初期化」を
             SendInput で押し、reset_pos_menu_outcome・reset_window_pos と窓の矩形を控える。有界の自動終了まで待つ。
  -Run Reboot : 書ける根を消さずに起こし直し、窓の矩形を控えて有界の自動終了まで待つ。
  記録は target\move-canon-signoff\<Tag>\run-<Run>.log、操作の控えは run-<Run>.steps.txt。
  入力デスクトップが Default でないときは操作せず止まる。

.EXAMPLE
  pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\areka-P0-move-canon\real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\areka-P0-move-canon\real-machine-run.ps1 -Run Adjust -Tag scale1
  pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\areka-P0-move-canon\real-machine-run.ps1 -Run Reboot -Tag scale1
#>
param(
    [switch]$Prepare,
    [ValidateSet('Adjust', 'Reboot')]
    [string]$Run,
    [string]$Tag = 'scale1',
    [int]$ExitMs = 0,
    [int]$Port = 39874,
    [int]$SeedEmoX = 1000,
    [string]$RustLog = 'info,kanade=trace,areka::input_events=debug'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (git -C $PSScriptRoot rev-parse --show-toplevel)).Path
$base = Join-Path $wt 'target\move-canon-signoff'
$root = Join-Path $base 'root'

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
    New-Item -ItemType Directory -Force $root | Out-Null
    Get-ChildItem -Force $root | Remove-Item -Recurse -Force
    Copy-Item -Recurse (Join-Path $manual '*') $root
    Copy-Item (Join-Path $wt 'target\debug\areka.exe') $root
    Copy-Item $helper $root
    # ゴースト自身のトーク（ランダムトーク・時報）を黙らせ、記録と窓を操作の結果だけにする。
    foreach ($f in 'dispatch.pasta', 'hour.pasta') { Remove-Item (Join-Path $root "ghost\emo2\ghost\master\dic\$f") }
    "prepared $root (areka=$(Machine (Join-Path $root 'areka.exe')) helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')))"
    "nar=$((Get-Item (Join-Path $wt 'vendors\sample_ghost\emo2.nar')).LastWriteTime.ToString('o'))"
    return
}

Add-Type @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public static class McInput {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] struct MOUSEINPUT { public int dx, dy; public uint data, flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] struct INPUT { public uint type; public MOUSEINPUT mi; }
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder b, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr h, StringBuilder b, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll", SetLastError=true)] static extern uint SendInput(uint n, INPUT[] i, int size);
  [DllImport("user32.dll", SetLastError=true)] static extern IntPtr OpenInputDesktop(uint f, bool inh, uint acc);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern bool GetUserObjectInformationW(IntPtr h, int idx, StringBuilder b, int len, out int need);
  [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr h);
  [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
  [DllImport("user32.dll")] static extern int GetSystemMetrics(int i);
  [DllImport("user32.dll")] static extern IntPtr SendMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] static extern int GetMenuItemCount(IntPtr m);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetMenuStringW(IntPtr m, uint id, StringBuilder b, int n, uint f);
  [DllImport("user32.dll")] static extern bool GetMenuItemRect(IntPtr h, IntPtr m, uint i, out RECT r);
  static void PerMonitor() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
  public static string InputDesktop() {
    var h = OpenInputDesktop(0, false, 0x0100); if (h == IntPtr.Zero) return "<none>";
    var sb = new StringBuilder(256); int n; GetUserObjectInformationW(h, 2, sb, 512, out n); CloseDesktop(h); return sb.ToString();
  }
  // 見えている窓: 題名\t左,上,右,下（物理画素）\tDPI\t窓の型の名前
  public static List<string> Windows(uint pid) {
    PerMonitor();
    var o = new List<string>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p != pid || !IsWindowVisible(h)) return true;
      var t = new StringBuilder(256); GetWindowTextW(h, t, 256); var c = new StringBuilder(256); GetClassNameW(h, c, 256); RECT r; GetWindowRect(h, out r);
      o.Add(String.Format("{0}\t{1},{2},{3},{4}\t{5}\t{6}", t, r.L, r.T, r.R, r.B, GetDpiForWindow(h), c)); return true; }, IntPtr.Zero);
    return o;
  }
  // 開いているポップアップメニュー（#32768）の項目: 添字\t文字\t左,上,右,下（物理画素）
  public static List<string> PopupItems() {
    PerMonitor();
    var o = new List<string>();
    EnumWindows((h, l) => { var c = new StringBuilder(64); GetClassNameW(h, c, 64);
      if (c.ToString() != "#32768" || !IsWindowVisible(h)) return true;
      IntPtr m = SendMessageW(h, 0x01E1, IntPtr.Zero, IntPtr.Zero); if (m == IntPtr.Zero) return true;
      int n = GetMenuItemCount(m);
      for (uint i = 0; i < n; i++) { var s = new StringBuilder(256); GetMenuStringW(m, i, s, 256, 0x400); RECT r; GetMenuItemRect(IntPtr.Zero, m, i, out r);
        o.Add(String.Format("{0}\t{1}\t{2},{3},{4},{5}", i, s, r.L, r.T, r.R, r.B)); }
      return true; }, IntPtr.Zero);
    return o;
  }
  static INPUT Abs(uint f, int x, int y) {
    int vx = GetSystemMetrics(76), vy = GetSystemMetrics(77), vw = GetSystemMetrics(78), vh = GetSystemMetrics(79);
    var i = new INPUT(); i.type = 0; i.mi.flags = f | 0x1 | 0x4000 | 0x8000;
    i.mi.dx = (int)((x - vx) * 65535L / (vw - 1)); i.mi.dy = (int)((y - vy) * 65535L / (vh - 1)); return i;
  }
  static int Size() { return Marshal.SizeOf(typeof(INPUT)); }
  public static POINT Cursor() { PerMonitor(); POINT p; GetCursorPos(out p); return p; }
  public static uint Move(int x, int y) { PerMonitor(); return SendInput(1, new[] { Abs(0, x, y) }, Size()); }
  // 押下と離しを別のフレームに届ける（同じフレームに押下・離し・離脱が重なると押下が拾われないことがある）。
  public static uint Click(int x, int y) {
    PerMonitor(); uint s = SendInput(1, new[] { Abs(0x2, x, y) }, Size());
    System.Threading.Thread.Sleep(80); return s + SendInput(1, new[] { Abs(0x4, x, y) }, Size());
  }
  public static uint RightClick(int x, int y) { PerMonitor(); return SendInput(2, new[] { Abs(0x8, x, y), Abs(0x10, x, y) }, Size()); }
  public static uint DoubleClick(int x, int y) {
    PerMonitor(); uint s = SendInput(2, new[] { Abs(0x2, x, y), Abs(0x4, x, y) }, Size());
    System.Threading.Thread.Sleep(60); return s + SendInput(2, new[] { Abs(0x2, x, y), Abs(0x4, x, y) }, Size());
  }
}
'@

if (-not $Run) { return }
if (-not (Test-Path (Join-Path $root 'areka.exe'))) { throw "no root. run -Prepare first" }
$dir = Join-Path $base $Tag
$lad = Join-Path $dir 'localappdata'
if ($Run -eq 'Adjust') {
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    # 前の走行の位置の記憶を消し、エモ（\1）だけを横へずらした記憶を置く（「エモの位置調整」で動く距離を作る）。
    $prof = Join-Path $root 'ghost\emo2\ghost\master\profile\areka'
    if (Test-Path $prof) { Remove-Item -Recurse -Force $prof }
    New-Item -ItemType Directory -Force $prof | Out-Null
    [IO.File]::WriteAllText((Join-Path $prof 'sylphya.toml'), "format-version = 1`r`n`r`n[window.1]`r`nx = `"$SeedEmoX`"`r`ny = `"904`"`r`n", [Text.UTF8Encoding]::new($false))
}
New-Item -ItemType Directory -Force $lad, (Join-Path $dir 'tmp') | Out-Null
if ($ExitMs -le 0) { $ExitMs = if ($Run -eq 'Adjust') { 90000 } else { 25000 } }
if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) { throw "port $Port is in use (pass -Port)" }
$desk0 = [McInput]::InputDesktop()
if ($desk0 -ne 'Default') { throw "input desktop is not Default ($desk0). unlock the screen and rerun" }

Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
$env:NO_COLOR = '1'
$env:RUST_LOG = $RustLog
$env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
$env:AREKA_NO_ALERT = '1'
$env:AREKA_MCP_PORT = "$Port"
$env:AREKA_ROOT = $root
# 書ける根（%LOCALAPPDATA%\areka）は子プロセスだけ target\ の下へ向ける（このプロセスの中だけの環境変数）。
$env:LOCALAPPDATA = $lad
$env:TMP = Join-Path $dir 'tmp'; $env:TEMP = $env:TMP
$log = Join-Path $dir "run-$Run.log"
$note = Join-Path $dir "run-$Run.steps.txt"
Set-Content $note "run=$Run tag=$Tag desktop=$desk0 exit_ms=$ExitMs rust_log=$RustLog localappdata=$lad"
$p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$(Join-Path $root 'ghost\emo2')`"" -WorkingDirectory $root `
    -RedirectStandardOutput $log -RedirectStandardError (Join-Path $dir "run-$Run.err.log") -WindowStyle Hidden -PassThru
$null = $p.Handle
"$Run pid=$($p.Id) start=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')

$script:seen = 0
function Note([string]$s) { Add-Content -LiteralPath $note -Encoding utf8 "$(Get-Date -Format o) $s" }
function Lines { @(Get-Content -LiteralPath $log -Encoding utf8 -ErrorAction SilentlyContinue) }
function WaitFor([string]$pattern, [int]$sec = 60) {
    $until = (Get-Date).AddSeconds($sec)
    while ((Get-Date) -lt $until -and -not $p.HasExited) {
        $lines = Lines
        for ($i = $script:seen; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match $pattern) { $script:seen = $i + 1; Note "hit [$pattern] line=$($i + 1): $($lines[$i])"; return $lines[$i] }
        }
        Start-Sleep -Milliseconds 100
    }
    Note "MISS [$pattern]"; throw "line did not come: $pattern"
}
function Snapshot([string]$label) {
    foreach ($w in [McInput]::Windows([uint32]$p.Id)) { Note "rect $label $w" }
}
# 題名が $title の見えている窓のうち、面積の小さい方（$Smallest＝バルーン）か大きい方（キャラクター）。
function WindowRect([string]$title, [bool]$Smallest) {
    $ws = [McInput]::Windows([uint32]$p.Id) | Where-Object { ($_ -split "`t")[0] -eq $title } |
        ForEach-Object { , ([int[]]((($_ -split "`t")[1]) -split ',')) } |
        Where-Object { $_[0] -gt -30000 -and $_[2] -gt $_[0] } |
        Sort-Object { ($_[2] - $_[0]) * ($_[3] - $_[1]) }
    $r = if ($Smallest) { $ws | Select-Object -First 1 } else { $ws | Select-Object -Last 1 }
    if (-not $r) { throw "window '$title' is not visible" }
    Note "window $title smallest=$Smallest rect=$($r -join ',')"
    , $r
}
function Act([string]$what, [scriptblock]$do) {
    if ([McInput]::InputDesktop() -ne 'Default') { throw "input desktop is no longer Default" }
    $sent = & $do; Note "$what sent=$sent"
}
$home0 = [McInput]::Cursor()
function Home { $null = [McInput]::Move($home0.X, $home0.Y); Note "pointer home $($home0.X),$($home0.Y)" }
# エモのバルーン（288×203・validrect.left 24・top 40）で、ポインタを縦に動かして選択肢の行を測る。
# 戻り値: y の小さい順に並べた行（ordinal と y の範囲）。
function Rows {
    $r = WindowRect 'エモ' $true; $k = ($r[2] - $r[0]) / 288.0
    $x = [int]($r[0] + 60 * $k)
    $rows = @{}; $current = $null
    for ($y = [int]($r[1] + 30 * $k); $y -le [int]($r[1] + 175 * $k); $y += 3) {
        if ([McInput]::InputDesktop() -ne 'Default') { throw "input desktop is no longer Default" }
        $null = [McInput]::Move($x, $y); Start-Sleep -Milliseconds 45
        $lines = Lines
        for ($i = $script:seen; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match 'event="?choice_hover_inject"? .*ordinal=(None|Some\((\d+)\))') { $current = $Matches[2] }
        }
        $script:seen = $lines.Count
        if ($null -ne $current) {
            if (-not $rows.ContainsKey($current)) { $rows[$current] = @($y, $y) } else { $rows[$current][1] = $y }
        }
    }
    foreach ($k2 in $rows.Keys) { Note "row ordinal=$k2 y=$($rows[$k2] -join '..') x=$x" }
    if ($rows.Count -lt 1) { throw "could not measure choice rows" }
    $sorted = $rows.GetEnumerator() | Sort-Object { $_.Value[0] } | ForEach-Object { @{ ord = $_.Key; y = [int](($_.Value[0] + $_.Value[1]) / 2); x = $x } }
    , @($sorted)
}
# むらさきのキャラクターの窓の中で、当たりの記録が出る点を探してダブルクリックする。
function DoubleClickSakura {
    $r = WindowRect 'むらさき' $false
    foreach ($f in @(@(0.5, 0.45), @(0.42, 0.43), @(0.5, 0.3), @(0.5, 0.6), @(0.45, 0.2))) {
        $x = [int]($r[0] + ($r[2] - $r[0]) * $f[0]); $y = [int]($r[1] + ($r[3] - $r[1]) * $f[1])
        Act "hover sakura $x,$y" { [McInput]::Move($x, $y) }; Start-Sleep -Milliseconds 400
        Act "double-click sakura $x,$y" { [McInput]::DoubleClick($x, $y) }
        try { WaitFor 'id=OnMouseDoubleClick' 4 | Out-Null; Home; return @($x, $y) } catch { Note "no OnMouseDoubleClick at $x,$y" }
    }
    throw "double-click did not reach sakura"
}
# 人の手と同じく、先にポインタを行へ運んで強調させてから押す（いきなり押すと選ばれない）。
function PressRow($row, [string]$what) {
    Act "hover $what" { [McInput]::Move($row.x, $row.y) }; Start-Sleep -Milliseconds 400
    Act "click $what" { [McInput]::Click($row.x, $row.y) }; Start-Sleep -Milliseconds 400; Home
}
function PickMenuItem([string]$needle) {
    $until = (Get-Date).AddSeconds(5)
    while ((Get-Date) -lt $until) {
        $items = [McInput]::PopupItems()
        if ($items.Count -gt 0) { break }
        Start-Sleep -Milliseconds 100
    }
    foreach ($it in $items) { Note "menu item $it" }
    $hit = $items | Where-Object { ($_ -split "`t")[1] -like "*$needle*" } | Select-Object -First 1
    if (-not $hit) { throw "menu item '$needle' not found" }
    $rc = [int[]]((($hit -split "`t")[2]) -split ',')
    $x = [int](($rc[0] + $rc[2]) / 2); $y = [int](($rc[1] + $rc[3]) / 2)
    Act "hover menu item $needle $x,$y" { [McInput]::Move($x, $y) }; Start-Sleep -Milliseconds 300
    Act "click menu item $needle $x,$y" { [McInput]::Click($x, $y) }
}

$ok = $true
try {
    WaitFor 'visible=true' | Out-Null
    Start-Sleep -Milliseconds 4000
    Snapshot 'boot'
    if ($Run -eq 'Adjust') {
        $pt = DoubleClickSakura
        WaitFor 'event="?choice_waiting_established' 30 | Out-Null
        $rows = Rows
        # メインメニュー: 1 列目の縦の並びは「おしゃべり頻度」「エモの位置調整」（「閉じる」は 5em の右）
        if ($rows.Count -lt 2) { throw "main menu rows < 2" }
        PressRow $rows[1] "main row ordinal=$($rows[1].ord) (エモの位置調整)"
        WaitFor 'event="?choice_selected' | Out-Null
        WaitFor 'event="?choice_waiting_established' 30 | Out-Null
        $rows = Rows
        # 調整メニュー: 1 列目は「調整」だけ（「もどる」は 5em の右）
        PressRow $rows[0] "adjust row ordinal=$($rows[0].ord) (調整)"
        WaitFor 'event="?choice_selected' | Out-Null
        WaitFor 'event="?move_applied' 30 | Out-Null
        Start-Sleep -Milliseconds 1500
        Snapshot 'after_move'
        # 右クリックのメニューの「シェル位置初期化」
        Act "hover sakura $($pt[0]),$($pt[1])" { [McInput]::Move($pt[0], $pt[1]) }; Start-Sleep -Milliseconds 400
        Act "right-click sakura $($pt[0]),$($pt[1])" { [McInput]::RightClick($pt[0], $pt[1]) }
        PickMenuItem 'シェル位置初期化'
        Home
        WaitFor 'event="?reset_pos_menu_outcome' 30 | Out-Null
        Start-Sleep -Milliseconds 2000
        Snapshot 'after_reset'
    }
} catch {
    $ok = $false; Note "STOP $($_.Exception.Message)"; Home
    "STOP: $($_.Exception.Message)"
}
Note "driver done ok=$ok"
$p.WaitForExit()
"$Run exit=$($p.ExitCode) ok=$ok end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')
