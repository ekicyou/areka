<#
.SYNOPSIS
  areka-P0-shell-balloon タスク 12.5（実機の確かめ）の根づくりと起動。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2 を nar-sample-path で展開して
             ワークツリーの target\sbx\A と target\sbx\B に写し、シェルへ箱の定義を書き足し、
             写したゴーストの台本（scripts\pasta\shiori\event\second_change.lua）を確かめ用に差し替える。
             製品のコードとリポジトリの検体（.nar）には触れない。
  -Run A|B : 根 A か B で areka を起こし、有界の自動終了まで待つ。記録は <根>\run.log。
  -Watch   : 走行中 1 秒ごとに、起こした areka の見えている窓の四角を <根>\windows.txt に書き、
             その四角だけを <根>\shots\ に画像で残す（窓の外の画面は撮らない）。画面を撮れない
             実行環境では撮影だけをやめ、四角の記録は続ける。

  根 A: shell\master の surface0 に縦書きの箱 2 つ（tate1・tate2）、surface1100 に tate1（別の位置）と
        font.follow,balloon の箱 fuda、surface1101 は箱なし。shell\second は surface0 の箱の位置だけ違う。
  根 B: A の master と同じ箱に、surface0 からはみ出す箱 hami を足したもの。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/areka-P0-shell-balloon/real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File .kiro/specs/areka-P0-shell-balloon/real-machine-run.ps1 -Run A -Watch
  pwsh -NoProfile -File .kiro/specs/areka-P0-shell-balloon/real-machine-run.ps1 -Run A -ExitMs 600000 -RustLog 'info,areka=debug,areka_emo_text=debug,areka_kanade=debug,kanade=trace'   # 目視・手で触る回
#>
param(
    [switch]$Prepare,
    [ValidateSet('A', 'B')][string]$Run,
    [int]$ExitMs = 80000,
    [string]$RustLog = 'info,areka=debug,areka_emo_text=debug,areka_emo_present=debug,areka_kanade=debug',
    [switch]$Watch
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$base = Join-Path $wt 'target\sbx'
$utf8 = [Text.UTF8Encoding]::new($false)

function Machine([string]$path) {
    $b = [IO.File]::ReadAllBytes($path)
    $pe = [BitConverter]::ToInt32($b, 0x3c)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

function Add-Text([string]$path, [string]$text) {
    # 検体の surfaces.txt は UTF-8（BOM なし）・CRLF。足す行も CRLF にそろえる。
    $crlf = ("`r`n" + $text.Trim() + "`r`n") -replace "(?<!`r)`n", "`r`n"
    [IO.File]::AppendAllText($path, $crlf, $utf8)
}

# ── 書き足す箱の定義（ukadoc の語で: balloon.*ブレスと、surface.append*ブレスの element定義） ──
$braces = @'
// ── areka-P0-shell-balloon 実機の確かめ（タスク 12.5）で書き足した箱 ──
balloon.tate1
{
size,48,320
vertical,1
font.height,20
font.color.r,0
font.color.g,0
font.color.b,160
}
balloon.tate2
{
size,48,320
vertical,1
font.height,20
font.color.r,160
font.color.g,0
font.color.b,0
}
balloon.fuda
{
size,240,64
font.follow,balloon
font.height,18
font.color.r,0
font.color.g,96
font.color.b,0
}
'@
$masterPlaces = @'
surface.append0
{
element1,balloon,tate1,370,40
element2,balloon,tate2,310,40
}
surface.append1100
{
element1,balloon,tate1,20,80
element2,balloon,fuda,70,460
}
'@
$secondPlaces = @'
surface.append0
{
element1,balloon,tate1,30,40
element2,balloon,tate2,90,40
}
'@
$overflow = @'
balloon.hami
{
size,320,120
font.height,22
}
surface.append0
{
element3,balloon,hami,250,620
}
'@

# ── 台本（Lua の長い文字列で書くので \ はそのまま） ──
$W = '\w9\w9\w9\w9'
$bootA = "\0\s[0]\b[tate1]一の箱です。$W\b[tate2]二の箱です。$W\b[nai]$W\s[1100]付いて回る。$W\b[fuda]\f[color,255,0,0]札は赤。$W\b[tate1]一は赤くない。$W\s[9999]無い番号でも箱。$W\s[1101]普通のバルーンへ。$W\s[0]箱へ戻る。$W\1\s[10]相方です。$W\e"
$stagesA = @(
    "\0\s[0]\b[tate2]シェルを替える。$W\![change,shell,second]\e",
    "\0\s[0]\b[tate1]選んで\n\q[はい,OnSbYes]\n\q[いいえ,OnSbNo]\e",
    ("\0\s[0]\b[tate1]長い台詞。" + ($W * 5) + "ダブルクリックで止まる。" + ($W * 5) + "\e")
)
$shellChangedA = "\0新しいシェルの箱。$W$W\e"
$bootB = "\0\s[0]\b[hami]はみ出す箱です。右と下へはみ出すほど長い文字を並べています。あいうえおかきくけこさしすせそたちつてと。$W$W$W\e"

function Lua-Script([string]$boot, [string[]]$stages, [string]$shellChanged) {
    $list = ($stages | ForEach-Object { "    [==[$_]==]," }) -join "`n"
    @"
--- areka-P0-shell-balloon タスク 12.5（実機の確かめ）用に、target\sbx の写しの中だけで差し替えた台本。
--- 起動の台詞のあと、話していない秒（OnSecondChange の GET）が GAP 回たまるごとに次の段を 1 つ出す。
local REG = require("pasta.shiori.event.register")
local log = require "@pasta_log"

local BOOT = [==[$boot]==]
local SHELL_CHANGED = [==[$shellChanged]==]
local STAGES = {
$list
}
local GAP = 8
local idle = 0
local stage = 0

REG.OnBoot = function(act) return BOOT end
REG.OnFirstBoot = function(act) return BOOT end
REG.OnShellChanging = function(act) return nil end
REG.OnShellChanged = function(act) return SHELL_CHANGED end
REG.OnSbYes = function(act) return [==[\0\s[0]\b[tate1]はいを選んだ。\w9\w9\w9\w9\e]==] end
REG.OnSbNo = function(act) return [==[\0\s[0]\b[tate1]いいえを選んだ。\w9\w9\w9\w9\e]==] end
REG.OnSecondChange = function(act)
    if string.lower(act.req.method or "") ~= "get" then return nil end
    idle = idle + 1
    if idle < GAP then return nil end
    idle = 0
    stage = stage + 1
    local s = STAGES[stage]
    if s then log.info({ fn = "signoff_stage", stage = stage }) end
    return s
end

return REG
"@
}

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
    foreach ($x in 'A', 'B') {
        $root = Join-Path $base $x
        if (Test-Path $root) { Remove-Item -Recurse -Force $root }
        New-Item -ItemType Directory -Force $root | Out-Null
        Copy-Item -Recurse (Join-Path $manual '*') $root
        Copy-Item (Join-Path $wt 'target\debug\areka.exe') $root
        Copy-Item $helper $root
        $shell = Join-Path $root 'ghost\emo2\shell'
        if ($x -eq 'A') {
            # シェルの写し（add_shell_copy と同じ手順: フォルダを写し、descript.txt の name の行だけ替える）
            Copy-Item -Recurse (Join-Path $shell 'master') (Join-Path $shell 'second')
            $d = Join-Path $shell 'second\descript.txt'
            $named = [IO.File]::ReadAllText($d, $utf8) -replace '(?m)^name,[^\r\n]*', 'name,second'
            [IO.File]::WriteAllText($d, $named, $utf8)
            Add-Text (Join-Path $shell 'second\surfaces.txt') ($braces + "`n" + $secondPlaces)
        }
        $surfaces = Join-Path $shell 'master\surfaces.txt'
        Add-Text $surfaces ($braces + "`n" + $masterPlaces)
        if ($x -eq 'B') { Add-Text $surfaces $overflow }
        $lua = if ($x -eq 'A') { Lua-Script $bootA $stagesA $shellChangedA } else { Lua-Script $bootB @() $shellChangedA }
        $lua = $lua -replace "(?<!`r)`n", "`r`n"
        [IO.File]::WriteAllText((Join-Path $root 'ghost\emo2\ghost\master\scripts\pasta\shiori\event\second_change.lua'), $lua, $utf8)
        "prepared $root (helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')) areka=$(Machine (Join-Path $root 'areka.exe')))"
    }
}

if ($Run) {
    $root = Join-Path $base $Run
    $ghost = Join-Path $root 'ghost\emo2'
    # 前の回の記憶を消す。最後に使ったシェルはゴースト側の記憶（ghost\master\profile\areka）にも残るので、
    # 消さないと前の回で替えた先のシェル（second）で起きる。
    foreach ($p in 'profile', 'tmp', 'shots', 'ghost\emo2\ghost\master\profile\areka') { $d = Join-Path $root $p; if (Test-Path $d) { Remove-Item -Recurse -Force $d } }
    New-Item -ItemType Directory -Force (Join-Path $root 'tmp') | Out-Null
    Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
    $env:NO_COLOR = '1'
    $env:RUST_LOG = $RustLog
    $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
    $env:AREKA_BALLOON_TIMEOUT_MS = '5000'
    $env:AREKA_ROOT = $root
    $env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
    $env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
    $p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$ghost`"" -WorkingDirectory $root `
        -RedirectStandardOutput (Join-Path $root 'run.log') -RedirectStandardError (Join-Path $root 'run.err.log') -PassThru
    "$Run pid=$($p.Id) start=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')
    if ($Watch) {
        Add-Type -AssemblyName System.Drawing
        Add-Type @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices;
public static class SbxWin {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
  public static List<int[]> Of(uint pid) {
    var list = new List<int[]>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) { RECT r; GetWindowRect(h, out r); list.Add(new[] { (int)h, r.L, r.T, r.R, r.B }); }
      return true; }, IntPtr.Zero);
    return list;
  }
}
'@
        [void][SbxWin]::SetProcessDpiAwarenessContext([IntPtr]::new(-4))
        $shots = New-Item -ItemType Directory -Force (Join-Path $root 'shots')
        while (-not $p.HasExited) {
            $t = Get-Date -Format 'HH-mm-ss'
            foreach ($w in [SbxWin]::Of([uint32]$p.Id)) {
                $width = $w[3] - $w[1]; $height = $w[4] - $w[2]
                "$(Get-Date -Format o) hwnd=$($w[0]) rect=($($w[1]),$($w[2]),$($w[3]),$($w[4])) size=${width}x${height}" | Add-Content (Join-Path $root 'windows.txt')
                if ($shots -and $width -gt 0 -and $height -gt 0) {
                    try {
                        $bmp = [Drawing.Bitmap]::new($width, $height)
                        $g = [Drawing.Graphics]::FromImage($bmp)
                        $g.CopyFromScreen($w[1], $w[2], 0, 0, $bmp.Size)
                        $bmp.Save((Join-Path $shots "$t-$($w[0]).png"))
                        $g.Dispose(); $bmp.Dispose()
                    } catch {
                        # 画面を撮れない席（対話の画面を持たない実行環境）では撮影だけをやめ、窓の四角の記録は続ける。
                        "$(Get-Date -Format o) 撮影できない: $($_.Exception.InnerException.Message)" | Add-Content (Join-Path $root 'windows.txt')
                        $shots = $null
                    }
                }
            }
            Start-Sleep -Milliseconds 1000
        }
    }
    $p.WaitForExit()
    "$Run exit=$($p.ExitCode) end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')
}
