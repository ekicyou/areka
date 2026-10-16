<#
.SYNOPSIS
  areka-P0-balloon-text-bugs タスク 5.3（件 1）・5.4（件 2）・5.5（件 3）を実機で確かめる根づくりと、起動・操作・記録。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2 を nar-sample-path で展開して
             ワークツリーの target\btb-signoff\root（普通のバルーン）と target\btb-signoff\root-box（箱）に写す。
             写しの辞書から dispatch.pasta・hour.pasta を外してゴースト自身のトークを黙らせる。
             root-box はシェルの surface0 に箱 btbbox（吹き出しの描き込まれた位置）を書き足す。
             製品のコードとリポジトリの検体には触れない。
  -Run S   : 筋書き S（Chose・Timeout・Break・Box・Wait）の台本（boot.lua）を写しへ書き、areka を絶対パスで起こし、
             記録を読みながら SendInput で操作し、有界の自動終了まで待つ。記録は target\btb-signoff\run-S.log、
             操作の控えは run-S.steps.txt。
             台本: 1 本目のトークは「選んでね（1）」＋選択肢 2 行。時間切れで隠れて OnBalloonTimeout が来ると、
             2 本目（「選んでね（2）」）を 1 回だけ返す（Wait は 1 本だけ）。
             1 本目は起点の確かめ（済んだ後は触らない）、2 本目は済んだ行の確かめ（ポインタを乗せて押す）。
             選択肢の行の位置は、ポインタを縦に動かして強調の記録（choice_hover_inject／box_choice_hover_inject）
             で測ってから押す。入力デスクトップが Default でないときは操作せず止まる。
             Collapse（タスク 5.4・件 2）: root の同梱バルーンに足した 8×8 の面 balloons20.png で \b[20]→\b[20]→\b[0]→\b[20] の
             4 本を時間切れごとに流す。操作はせず、見えている間に areka の MCP の dump_balloon を dump-<名>.png に撮る。
             Underline・Vertical（タスク 5.5・件 3）: 最下行（縦書きは最後の列）に作者の下線とアンカーを置いた 4 行の台詞を
             1 本流し、終わってから dump_balloon で撮る。Vertical は root-tate（基層に vertical,1 と wordwrappoint.y,-56）。

.EXAMPLE
  pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\completed\areka-P0-balloon-text-bugs\real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-anchor-tag-canon-da1fe5\.kiro\specs\completed\areka-P0-balloon-text-bugs\real-machine-run.ps1 -Run Chose
#>
param(
    [switch]$Prepare,
    [ValidateSet('Chose', 'Timeout', 'Break', 'Box', 'Wait', 'Collapse', 'Underline', 'Vertical')]
    [string]$Run,
    [int]$ExitMs = 75000,
    [int]$TimeoutMs = 5000,
    [int]$Port = 39873,
    [string]$RustLog = 'info,kanade=trace,areka::input_events=debug'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (git -C $PSScriptRoot rev-parse --show-toplevel)).Path
$base = Join-Path $wt 'target\btb-signoff'
$utf8 = [Text.UTF8Encoding]::new($false)

function Machine([string]$exe) {
    $b = [IO.File]::ReadAllBytes($exe)
    $pe = [BitConverter]::ToInt32($b, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

# 写しの scripts\pasta\shiori\event\boot.lua をこれに差し替える（__SCEN__ を筋書きの名前に替える）。
# 台本はすべて生のさくらスクリプト（pasta の変換・ウェイトの挿入を通さない）。
$lua = @'
--- areka-P0-balloon-text-bugs 5.3 の実機の確かめ用（target\btb-signoff の写しだけに置く）。
local REG = require("pasta.shiori.event.register")
local SCEN = "__SCEN__"
local n = 0
local CHOICES = [==[\n\q[はい（いち）を選ぶ,OnBtbYes]\n\q[いいえ（に）を選ぶ,OnBtbNo]]==]

-- 件 2（Collapse）: 8×8 のバルーンの面 \b[20] で潰す → 隠れて同じ面でもう一度出す → \b[0] へ戻す → また \b[20]
local COLLAPSE = {
    [==[\0\s[0]\b[20]ちいさい面（1）\e]==],
    [==[\0\s[0]\b[20]ちいさい面（2）\e]==],
    [==[\0\s[0]\b[0]もどった面（3）です\e]==],
    [==[\0\s[0]\b[20]ちいさい面（4）\e]==],
}

-- 件 3（Underline・Vertical）: 文字の領域に 4 行（縦書きは 4 列）入る台詞。最下行（縦書きは最後の列）に
-- 作者の下線とアンカーを置き、字の下の部分（g・y・ぁ）を全部の行に入れる。
local UNDERLINE = [==[\0\s[0]\b[0]一行目 gyぁ\n二行目 gyぁ\n三行目 \f[underline,1]下線gyぁ\f[underline,0]\n]==]
    .. [==[\f[underline,1]下線gyぁ\f[underline,0] \_a[OnBtbAnchor]アンカgyぁ\_a\e]==]
local VERTICAL = [==[\0\s[0]\b[0]\f[underline,1]下線ぁ\f[underline,0]\n二列目\n三列目\n]==]
    .. [==[\f[underline,1]下線ぁ\f[underline,0]\_a[OnBtbAnchor]アンカ\_a\e]==]

local function talk()
    n = n + 1
    if SCEN == "Collapse" then return COLLAPSE[n] end
    if SCEN == "Underline" then return UNDERLINE end
    if SCEN == "Vertical" then return VERTICAL end
    local s = [==[\0\s[0]]==]
    if SCEN == "Timeout" then s = s .. [==[\![set,choicetimeout,8000]]==] end
    s = s .. "選んでね（" .. tostring(n) .. "）" .. CHOICES
    if SCEN == "Wait" then
        -- 選択肢の後ろに字と \w の待ち（約 9 秒）
        s = s .. [==[\nまだ待ってね]==] .. string.rep([==[\w9]==], 20)
    end
    return s .. [==[\e]==]
end

local function quiet(act) return nil end

REG.OnBoot = function(act) return talk() end
REG.OnFirstBoot = REG.OnBoot
REG.OnGhostChanged = REG.OnBoot
REG.OnBalloonTimeout = function(act)
    if SCEN == "Collapse" and n < #COLLAPSE then return talk() end
    if (SCEN == "Chose" or SCEN == "Timeout" or SCEN == "Break" or SCEN == "Box") and n < 2 then return talk() end
    return nil
end

for _, id in ipairs({ "OnBtbYes", "OnBtbNo", "OnBtbAnchor", "OnAnchorSelectEx", "OnAnchorSelect", "OnChoiceTimeout", "OnBalloonBreak", "OnBalloonClose",
    "OnMinuteChange", "OnHourTimeSignal", "OnMouseMove", "OnMouseClick", "OnMouseDoubleClick", "OnMouseWheel",
    "OnShellChanging", "OnShellChanged", "OnBalloonChange", "OnGhostChanging", "OnSurfaceChange",
    "OnSurfaceRestore", "OnInstallCompleteAll", "OnUpdateComplete" }) do
    REG[id] = quiet
end

return REG
'@

# root-box の surface0 に足す箱（ukadoc の語で: balloon.*ブレスと surface.append*ブレスの element定義）。
# surface0.png（434×687）の吹き出しの絵は x 30〜390・y 40〜160 が不透明。そこに箱を置く。
$boxBraces = @'
// ── areka-P0-balloon-text-bugs 5.3 実機の確かめで書き足した箱 ──
balloon.btbbox
{
size,340,110
font.height,20
}
surface.append0
{
element1,balloon,btbbox,40,50
}
'@

if ($Prepare) {
    Push-Location $wt
    try {
        cargo build -p areka --bin areka -j 4; if ($LASTEXITCODE) { throw "areka のビルドが失敗 ($LASTEXITCODE)" }
        cargo build -p shiori-host32-helper --target i686-pc-windows-msvc -j 4; if ($LASTEXITCODE) { throw "helper のビルドが失敗 ($LASTEXITCODE)" }
        $printed = cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2
        if ($LASTEXITCODE) { throw "nar-sample-path が失敗 ($LASTEXITCODE)" }
    } finally { Pop-Location }
    $manual = ($printed | Where-Object { $_ -like 'root=*' }) -replace '^root=', ''
    $helper = Join-Path $wt 'target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe'
    if ((Machine $helper) -ne '014C') { throw "helper が 32bit でない: $helper" }
    foreach ($name in 'root', 'root-box', 'root-tate') {
        $root = Join-Path $base $name
        New-Item -ItemType Directory -Force $root | Out-Null
        Get-ChildItem -Force $root | Remove-Item -Recurse -Force
        Copy-Item -Recurse (Join-Path $manual '*') $root
        Copy-Item (Join-Path $wt 'target\debug\areka.exe') $root
        Copy-Item $helper $root
        foreach ($f in 'dispatch.pasta', 'hour.pasta') { Remove-Item (Join-Path $root "ghost\emo2\ghost\master\dic\$f") }
        # 件 2: 余白（validrect の 36〜56 px）より小さい 8×8 のバルーンの面（\b[20]）を足す＝実機の再現の形
        Add-Type -AssemblyName System.Drawing
        $bmp = [Drawing.Bitmap]::new(8, 8); $g = [Drawing.Graphics]::FromImage($bmp); $g.Clear([Drawing.Color]::White); $g.Dispose()
        $bmp.Save((Join-Path $root 'balloon\emo2-kakukaku\balloons20.png'), [Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
        "prepared $root (areka=$(Machine (Join-Path $root 'areka.exe')) helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')))"
    }
    $surfaces = Join-Path $base 'root-box\ghost\emo2\shell\master\surfaces.txt'
    [IO.File]::AppendAllText($surfaces, (("`r`n" + $boxBraces.Trim() + "`r`n") -replace "(?<!`r)`n", "`r`n"), $utf8)
    # 件 3 の縦書き: root-tate の同梱バルーンの基層 descript.txt に正典キー vertical,1 を足す（書字方向は起動時に確定）。
    # emo2-kakukaku は横書き用で wordwrappoint.y,0 を持ち、縦書きでは 1 字ごとに列を折り返す（1 回目の撮影）。
    # 列を validrect の下端まで使えるよう、wordwrappoint.y を validrect.bottom と同じ -56 に上書きする（後勝ち）。
    [IO.File]::AppendAllText((Join-Path $base 'root-tate\balloon\emo2-kakukaku\descript.txt'), "`r`nvertical,1`r`nwordwrappoint.y,-56`r`n", $utf8)
    return
}

Add-Type @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public static class BtbInput {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] struct MOUSEINPUT { public int dx, dy; public uint data, flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] struct INPUT { public uint type; public MOUSEINPUT mi; }
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder b, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool GetCursorPos(out POINT p);
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
  // 見えている窓: 題名\t左,上,右,下（物理画素）
  public static List<string> Windows(uint pid) {
    PerMonitor();
    var o = new List<string>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p != pid || !IsWindowVisible(h)) return true;
      var t = new StringBuilder(256); GetWindowTextW(h, t, 256); RECT r; GetWindowRect(h, out r);
      o.Add(String.Format("{0}\t{1},{2},{3},{4}", t, r.L, r.T, r.R, r.B)); return true; }, IntPtr.Zero);
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
  public static uint Click(int x, int y) { PerMonitor(); return SendInput(2, new[] { Abs(0x2, x, y), Abs(0x4, x, y) }, Size()); }
  public static uint DoubleClick(int x, int y) {
    PerMonitor(); uint s = SendInput(2, new[] { Abs(0x2, x, y), Abs(0x4, x, y) }, Size());
    System.Threading.Thread.Sleep(60); return s + SendInput(2, new[] { Abs(0x2, x, y), Abs(0x4, x, y) }, Size());
  }
}
'@

if (-not $Run) { return }
$root = Join-Path $base $(switch ($Run) { 'Box' { 'root-box' } 'Vertical' { 'root-tate' } default { 'root' } })
if (-not (Test-Path (Join-Path $root 'areka.exe'))) { throw "根がない。先に -Prepare" }
$bootLua = Join-Path $root 'ghost\emo2\ghost\master\scripts\pasta\shiori\event\boot.lua'
[IO.File]::WriteAllText($bootLua, (($lua -replace '__SCEN__', $Run) -replace "(?<!`r)`n", "`r`n"), $utf8)
foreach ($p in 'profile', 'tmp', 'ghost\emo2\ghost\master\profile') {
    $d = Join-Path $root $p; if (Test-Path $d) { Remove-Item -Recurse -Force $d }
}
New-Item -ItemType Directory -Force (Join-Path $root 'tmp') | Out-Null
if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) { throw "port $Port is in use (pass -Port)" }
$desk0 = [BtbInput]::InputDesktop()
if ($desk0 -ne 'Default') { throw "入力デスクトップが Default でない（$desk0）。画面の錠を解いてから回し直す" }

Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
$env:NO_COLOR = '1'
$env:RUST_LOG = $RustLog
$env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
$env:AREKA_NO_ALERT = '1'
$env:AREKA_BALLOON_TIMEOUT_MS = "$TimeoutMs"
$env:AREKA_MCP_PORT = "$Port"
$env:AREKA_ROOT = $root
$env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
$env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
$log = Join-Path $base "run-$Run.log"
$note = Join-Path $base "run-$Run.steps.txt"
Set-Content $note "scenario=$Run desktop=$desk0 timeout_ms=$TimeoutMs exit_ms=$ExitMs rust_log=$RustLog"
$p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$(Join-Path $root 'ghost\emo2')`"" -WorkingDirectory $root `
    -RedirectStandardOutput $log -RedirectStandardError (Join-Path $base "run-$Run.err.log") -WindowStyle Hidden -PassThru
$null = $p.Handle
"$Run pid=$($p.Id) start=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')

$script:seen = 0
function Note([string]$s) { Add-Content -LiteralPath $note -Encoding utf8 "$(Get-Date -Format o) $s" }
function Lines { @(Get-Content -LiteralPath $log -Encoding utf8 -ErrorAction SilentlyContinue) }
# $script:seen より後の行で $pattern に合う最初の行を待つ。見つけたら seen をその次へ進める。
function WaitFor([string]$pattern, [int]$sec = 60) {
    $until = (Get-Date).AddSeconds($sec)
    while ((Get-Date) -lt $until -and -not $p.HasExited) {
        $lines = Lines
        for ($i = $script:seen; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match $pattern) { $script:seen = $i + 1; Note "hit [$pattern] line=$($i + 1)"; return $lines[$i] }
        }
        Start-Sleep -Milliseconds 100
    }
    Note "MISS [$pattern]"; throw "待ちの行が来なかった: $pattern"
}
# 題名が名前（scope 0＝むらさき）の見えている窓のうち、面積の小さい方（$Smallest）か大きい方。
function WindowRect([bool]$Smallest) {
    $ws = [BtbInput]::Windows([uint32]$p.Id) | Where-Object { ($_ -split "`t")[0] -eq 'むらさき' } |
        ForEach-Object { , ([int[]]((($_ -split "`t")[1]) -split ',')) } |
        Where-Object { $_[0] -gt -30000 -and $_[2] -gt $_[0] } |  # 最小化された窓（-32000 の位置）を除く
        Sort-Object { ($_[2] - $_[0]) * ($_[3] - $_[1]) }
    $r = if ($Smallest) { $ws | Select-Object -First 1 } else { $ws | Select-Object -Last 1 }
    if (-not $r) { throw "むらさきの窓が見えていない" }
    Note "window smallest=$Smallest rect=$($r -join ',')"
    , $r
}
# x を固めてポインタを y0..y1 で縦に動かし、強調の記録から選択肢の行ごとの y の範囲を測る。
function Calibrate([int]$x, [int]$y0, [int]$y1, [string]$event) {
    if ([BtbInput]::InputDesktop() -ne 'Default') { throw "入力デスクトップが Default でなくなった" }
    $rows = @{}; $current = $null
    for ($y = $y0; $y -le $y1; $y += 3) {
        $null = [BtbInput]::Move($x, $y); Start-Sleep -Milliseconds 45
        $lines = Lines
        for ($i = $script:seen; $i -lt $lines.Count; $i++) {
            if ($lines[$i] -match "event=`"?$event`"? .*ordinal=(None|Some\((\d+)\))") { $current = $Matches[2] }
        }
        $script:seen = $lines.Count
        if ($null -ne $current) {
            if (-not $rows.ContainsKey($current)) { $rows[$current] = @($y, $y) } else { $rows[$current][1] = $y }
        }
    }
    foreach ($k in $rows.Keys) { Note "row ordinal=$k y=$($rows[$k] -join '..') x=$x" }
    if (-not $rows.ContainsKey('0') -or -not $rows.ContainsKey('1')) { throw "選択肢の行を測れなかった" }
    $rows
}
function Mid($r) { [int](($r[0] + $r[1]) / 2) }
$home0 = [BtbInput]::Cursor()
function Home { $null = [BtbInput]::Move($home0.X, $home0.Y); Note "pointer home $($home0.X),$($home0.Y)" }
function Act([string]$what, [scriptblock]$do) {
    if ([BtbInput]::InputDesktop() -ne 'Default') { throw "入力デスクトップが Default でなくなった" }
    $sent = & $do; Note "$what sent=$sent"
}
# 選択肢の行の測り方（普通のバルーンは一番小さいむらさきの窓・箱はシェルの窓の箱の位置）。
function Rows {
    if ($Run -eq 'Box') {
        $r = WindowRect $false; $k = ($r[2] - $r[0]) / 434.0
        $x = [int]($r[0] + (40 + 30) * $k)
        return @{ x = $x; rows = (Calibrate $x ([int]($r[1] + 50 * $k)) ([int]($r[1] + 160 * $k)) 'box_choice_hover_inject') }
    }
    $r = WindowRect $true; $k = ($r[2] - $r[0]) / 400.0
    $x = [int]($r[0] + (36 + 40) * $k)
    @{ x = $x; rows = (Calibrate $x ([int]($r[1] + 40 * $k)) ([int]($r[1] + 175 * $k)) 'choice_hover_inject') }
}
# 済んだ行の確かめ: 2 行にポインタを乗せ、両方を 1 回ずつ押す。
function Probe($c) {
    Start-Sleep -Milliseconds 500
    Act 'probe hover yes' { [BtbInput]::Move($c.x, (Mid $c.rows['0'])) }; Start-Sleep -Milliseconds 400
    Act 'probe hover no' { [BtbInput]::Move($c.x, (Mid $c.rows['1'])) }; Start-Sleep -Milliseconds 400
    Act 'probe click no' { [BtbInput]::Click($c.x, (Mid $c.rows['1'])) }; Start-Sleep -Milliseconds 400
    Act 'probe click yes' { [BtbInput]::Click($c.x, (Mid $c.rows['0'])) }; Start-Sleep -Milliseconds 400
    Home
}

# areka 自身の MCP の dump_balloon（scope 0）を撮って dump-<名>.png に残す。
function Dump([string]$name) {
    $body = @{ jsonrpc = '2.0'; id = 1; method = 'tools/call'; params = @{ name = 'dump_balloon'; arguments = @{ scope = 0 } } } | ConvertTo-Json -Depth 6 -Compress
    try {
        $r = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$Port/api/mcp/v1" -ContentType 'application/json; charset=utf-8' `
            -Headers @{ Accept = 'application/json, text/event-stream' } -Body ([Text.Encoding]::UTF8.GetBytes($body)) -TimeoutSec 30
    } catch { Note "dump $name EXC $($_.Exception.Message)"; return }
    $img = $r.result.content | Where-Object { $_.type -eq 'image' } | Select-Object -First 1
    $text = ($r.result.content | Where-Object { $_.type -eq 'text' } | Select-Object -First 1).text
    if ($img) {
        $png = Join-Path $base "dump-$name.png"
        [IO.File]::WriteAllBytes($png, [Convert]::FromBase64String($img.data))
        Add-Type -AssemblyName System.Drawing; $b = [Drawing.Bitmap]::new($png); $wh = "$($b.Width)x$($b.Height)"; $b.Dispose()
        Note "dump $name isError=$([bool]$r.result.isError) png=$png size=$wh"
    } else { Note "dump $name isError=$([bool]$r.result.isError) no image text=$text" }
}

$ok = $true
try {
    switch ($Run) {
        { $_ -in 'Chose', 'Box' } {
            # 1 本目: 選んで 204 → 待ちが解けた時刻から既定の長さで隠れる
            WaitFor 'event="?choice_waiting_established' | Out-Null
            $c = Rows
            Act 'T1 click yes' { [BtbInput]::Click($c.x, (Mid $c.rows['0'])) }; Home
            WaitFor 'event="?choice_resolved' | Out-Null
            WaitFor 'id=OnBalloonTimeout' | Out-Null
            # 2 本目: 二度押し（ダブルクリック）で選び、済んだ行に乗せて押す
            WaitFor 'event="?choice_waiting_established' | Out-Null
            $c = Rows
            Act 'T2 double-click yes' { [BtbInput]::DoubleClick($c.x, (Mid $c.rows['0'])) }; Home
            WaitFor 'event="?choice_resolved' | Out-Null
            Probe $c
            WaitFor 'id=OnBalloonTimeout' | Out-Null
        }
        'Timeout' {
            WaitFor 'event="?choice_waiting_established' | Out-Null
            WaitFor 'event="?choice_timeout_cancelled' 30 | Out-Null
            WaitFor 'id=OnBalloonTimeout' | Out-Null
            WaitFor 'event="?choice_waiting_established' | Out-Null
            $c = Rows; Home
            WaitFor 'event="?choice_timeout_cancelled' 30 | Out-Null
            Probe $c
            WaitFor 'id=OnBalloonTimeout' | Out-Null
        }
        'Break' {
            WaitFor 'event="?choice_waiting_established' | Out-Null
            $c = Rows
            # 1 行目（選択肢でない字）をダブルクリックして中断する
            $y = (Mid $c.rows['0']) - ($c.rows['1'][0] - $c.rows['0'][0])
            Act 'T1 double-click line0 (break)' { [BtbInput]::DoubleClick($c.x, $y) }; Home
            WaitFor 'event="?balloon_break_accepted' | Out-Null
            # 利用者の中断は全部のバルーンをその場で隠す（完了 balloon-break 要件 4.1）。隠れた後に
            # OnBalloonTimeout が来ないこと（次のトークまで見送る）を、既定の長さの 3 倍待って確かめる。
            Start-Sleep -Milliseconds (3 * $TimeoutMs)
            $late = @(Lines | Select-Object -Skip $script:seen | Where-Object { $_ -match 'id=OnBalloonTimeout' })
            Note "OnBalloonTimeout after break: $($late.Count)"
        }
        { $_ -in 'Underline', 'Vertical' } {
            # 件 3: 操作はしない。台詞が出揃うのを待ち（トークの終わり＝steady_talk_done か boot_talk の完了）、
            # dump_balloon で撮る。撮った後は時間切れに任せる。
            WaitFor 'visible=true' | Out-Null
            WaitFor 'event="?(boot_talk_done|steady_talk_done|talk_done)' 30 | Out-Null
            Start-Sleep -Milliseconds 1000; Dump ($Run.ToLower())
        }
        'Collapse' {
            # 件 2: 操作はしない。面ごとに記録を待ち、見えている間に dump_balloon で今の絵を撮る。
            WaitFor '文字の描画範囲が潰れている' | Out-Null
            Start-Sleep -Milliseconds 1500; Dump 'c1'
            WaitFor 'id=OnBalloonTimeout' | Out-Null
            WaitFor 'visible=true' | Out-Null
            Start-Sleep -Milliseconds 1500; Dump 'c2'
            WaitFor 'id=OnBalloonTimeout' | Out-Null
            WaitFor '文字の描画範囲が戻った' | Out-Null
            WaitFor 'テキスト供給面を予約スロットへ装着した' | Out-Null
            Start-Sleep -Milliseconds 2000; Dump 'r3'
            WaitFor 'id=OnBalloonTimeout' | Out-Null
            WaitFor '文字の描画範囲が潰れている' | Out-Null
            Start-Sleep -Milliseconds 1500; Dump 'c4'
            WaitFor 'id=OnBalloonTimeout' | Out-Null
        }
        'Wait' {
            # 選択肢の行が出た後、\w の待ちの最中に押す（捨てられる）→ 待ちが明けた後に押し直す（選べる）
            WaitFor 'visible=true' | Out-Null
            Start-Sleep -Milliseconds 1500
            $c = Rows
            Act 'press yes during wait' { [BtbInput]::Click($c.x, (Mid $c.rows['0'])) }; Home
            WaitFor 'event="?choice_waiting_established' 30 | Out-Null
            # 人の手と同じく、先にポインタを行へ運んでから押す
            Act 'hover yes after wait' { [BtbInput]::Move($c.x, (Mid $c.rows['0'])) }; Start-Sleep -Milliseconds 400
            Act 'press yes after wait' { [BtbInput]::Click($c.x, (Mid $c.rows['0'])) }; Home
            WaitFor 'event="?choice_resolved' | Out-Null
            WaitFor 'id=OnBalloonTimeout' | Out-Null
        }
    }
} catch {
    $ok = $false; Note "STOP $($_.Exception.Message)"; Home
    "STOP: $($_.Exception.Message)"
}
Note "driver done ok=$ok"
$p.WaitForExit()
"$Run exit=$($p.ExitCode) ok=$ok end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')
