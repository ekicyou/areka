<#
.SYNOPSIS
  areka-P0-shell-balloon タスク 12.5（実機の確かめ）の根づくりと起動。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2 を nar-sample-path で展開して
             ワークツリーの target\sbx\A と target\sbx\B に写し、シェルへ箱の定義を書き足し、
             写したゴーストの台本（scripts\pasta\shiori\event\second_change.lua）を確かめ用に差し替える。
             製品のコードとリポジトリの検体（.nar）には触れない。
  -Run A|B : 根 A か B で areka を起こし、有界の自動終了まで待つ。記録は <根>\run.log。
             -Item を付けないと、起動の台詞と 3 つの段を 1 回で流す最初の通し（12.5 の記録の 1〜3 章）。
  -Item n  : -Run と一緒に使う。1 回の起動で項目 n（1〜10・real-machine-check.md 4 章）だけを確かめる。
             台本はその項目の台詞 1 本だけに差し替え、ゴースト自身の台詞（ランダムトーク・なでなで・
             メニュー・選択肢の時間切れ・シェル切替の台詞）はこの回のあいだ黙らせる。箱は吹き出しの
             描き込まれていない絵（surface1110〜1112）に置く。項目 5 は根 B、ほかは根 A。
             自動終了の時間・バルーンの待ち時間・ログの水準は項目ごとの既定を使う（-ExitMs・-RustLog を
             明示したときはそちらが勝つ）。
  -Watch   : 走行中 1 秒ごとに、起こした areka の見えている窓の四角を <根>\windows.txt に書き、
             その四角だけを <根>\shots\ に画像で残す（窓の外の画面は撮らない）。画面を撮れない
             実行環境では撮影だけをやめ、四角の記録は続ける。

  根 A: shell\master の surface0 に縦書きの箱 2 つ（tate1・tate2）、surface1100 に tate1（別の位置）と
        font.follow,balloon の箱 fuda、surface1101 は箱なし。shell\second は surface0 の箱の位置だけ違う。
        項目ごとの確かめ用に、吹き出しの描き込まれていない絵（purple\0\base1.png・base2.png）で
        surface1110（tate1 が右肩・tate2 が左肩）・surface1111（tate1 が左上・fuda が足元）・
        surface1112（箱なし）を足す。surface0.png には吹き出しが絵として描き込まれているので、
        項目ごとの確かめでは使わない。
  根 B: A の master と同じ箱に、surface0 と surface1110 からはみ出す箱 hami を足したもの。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/areka-P0-shell-balloon/real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File .kiro/specs/areka-P0-shell-balloon/real-machine-run.ps1 -Run A -Watch
  pwsh -NoProfile -File .kiro/specs/areka-P0-shell-balloon/real-machine-run.ps1 -Run A -Item 7   # 項目 7 だけを確かめる回
#>
param(
    [switch]$Prepare,
    [ValidateSet('A', 'B')][string]$Run,
    [ValidateRange(1, 10)][int]$Item,
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
surface.append1110
{
element3,balloon,hami,200,480
}
'@
# 項目ごとの確かめ用の絵（吹き出しの描き込まれていない base1.png・base2.png は 382×547）。
$itemPlaces = @'
// ── 項目ごとの確かめ用（吹き出しの描き込まれていない絵） ──
surface1110
{
element0,overlay,purple/0/base1.png,0,0
}
surface1111
{
element0,overlay,purple/0/base2.png,0,0
}
surface1112
{
element0,overlay,purple/0/base1.png,0,0
}
surface.append1110
{
element1,balloon,tate1,330,40
element2,balloon,tate2,10,40
}
surface.append1111
{
element1,balloon,tate1,10,80
element2,balloon,fuda,70,470
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

# ── 項目ごとの台本（-Item）。説明は相方（\1）の普通のバルーンに、触る相手は紫の子（\0）の箱に出す ──
# 置き場所: surface1110 は tate1（青・縦書き）が右肩（330,40）・tate2（赤・縦書き）が左肩（10,40）、
#           surface1111 は tate1 が左上（10,80）・fuda（緑・横書き）が足元（70,470）、surface1112 は箱なし。
$red = '\0\s[1110]\b[tate2]立ち絵へのダブルクリックが届いた。\_w[5000]\e'
$items = @{
    1  = @{ Root = 'A'; ExitMs = 120000; TimeoutMs = 5000; Talk =
        '\1\s[10]項目 1。紫の子の右肩に青い縦書き、左肩に赤い縦書きが出ます。腕の形が替わると青い文字が左上へ付いて回り、足元の緑の札だけが赤い文字になります。最後に紫の子の文字が普通の吹き出しへ移り、また箱へ戻ります。' +
        '\0\s[1110]\b[tate1]一の箱（青）です。\_w[3000]\b[tate2]二の箱（赤）です。\_w[3000]\b[nai]\_w[500]\s[1111]付いて回る。\_w[3000]\b[fuda]\f[color,255,0,0]札は赤。\_w[3000]\b[tate1]一は青のまま。\_w[3000]\s[1112]普通の吹き出しへ。\_w[3000]\s[1110]箱へ戻る。\_w[5000]\e' }
    2  = @{ Root = 'A'; ExitMs = 180000; TimeoutMs = 5000; Talk =
        '\1\s[10]項目 2。紫の子が 90 秒話し続けます。そのあいだに紫の子の体をつかんでドラッグしてください。拡大率の違うモニタがあれば、そちらへ持っていって戻してください。青と赤の文字が絵と一緒に動いてずれなければ合格です。' +
        '\0\s[1110]\b[tate1]青の文字\b[tate2]赤の文字\_w[90000]\e' }
    3  = @{ Root = 'A'; ExitMs = 150000; TimeoutMs = 5000; Talk =
        '\1\s[10]項目 3。紫の子の箱に文字が出る・消える・腕の形が替わる・普通の吹き出しへ移る、を 8 回くり返します。そのあいだ立ち絵がちらつかない（一瞬消える・白くなる・跳ねるが無い）ことを見てください。' +
        '\0\s[1110]' + ('\b[tate1]出る\_w[1500]\c\_w[1000]\s[1111]替わる\_w[1500]\s[1112]吹き出し\_w[1500]\s[1110]\b[tate2]戻る\_w[1500]\c\_w[1000]' * 8) + '\e' }
    4  = @{ Root = 'A'; ExitMs = 120000; TimeoutMs = 5000; Talk =
        '\1\s[10]項目 4。紫の子の腕の形が 3 秒ごとに替わり、そのたびに青い文字が右肩と左上を行き来します。替わる瞬間に、前の位置に文字が一瞬残って見えるかを見てください。' +
        '\0\s[1110]\b[tate1]行き来する文字' + ('\_w[3000]\s[1111]\_w[3000]\s[1110]' * 8) + '\_w[2000]\e' }
    5  = @{ Root = 'B'; ExitMs = 90000; TimeoutMs = 5000; Talk =
        '\1\s[10]項目 5。紫の子の右下に、絵からはみ出す箱へ長い文字が出ます。はみ出した部分が窓の端で切れて見え、立ち絵の窓が大きくならなければ合格です。' +
        '\0\s[1110]\b[hami]はみ出す箱です。右と下へはみ出すほど長い文字を並べています。あいうえおかきくけこさしすせそたちつてとなにぬねのはひふへほ。\_w[30000]\e' }
    6  = @{ Root = 'A'; ExitMs = 150000; TimeoutMs = 5000; Talk =
        '\1\s[10]項目 6。紫の子の後ろにメモ帳などの窓を置いてください。右肩の青い文字の下（同じ縦の列の、何も描かれていない所）をクリックして、後ろの窓が選ばれれば合格です。そのあと青い文字そのものを 1 回だけクリックしてください（こちらは後ろへ抜けません）。' +
        '\0\s[1110]\b[tate1]文字\_w[90000]\e' }
    7  = @{ Root = 'A'; ExitMs = 150000; TimeoutMs = 5000; Talk =
        '\1\s[10]項目 7。紫の子の足元の緑の札に選択肢が 2 つ出ます。ポインタを重ねると強調され、クリックすると返事が同じ札に出れば合格です。' +
        '\0\s[1111]\![set,choicetimeout,0]\b[fuda]選んで\n\q[はい,OnSbYes]　\q[いいえ,OnSbNo]\e' }
    8  = @{ Root = 'A'; ExitMs = 150000; TimeoutMs = 5000; Trace = $true; Talk =
        '\1\s[10]項目 8。紫の子が 60 秒話し続けます。そのあいだに右肩の青い文字を左ダブルクリックしてください。台詞が止まって文字が消え、左肩に赤い文字が出なければ合格です。' +
        '\0\s[1110]\b[tate1]ここをダブルクリック\_w[60000]\b[tate2]止まらなかった\e'; Double = $red }
    9  = @{ Root = 'A'; ExitMs = 150000; TimeoutMs = 60000; Trace = $true; Talk =
        '\1\s[10]項目 9。右肩に青い文字が出たら、紫の子は話し終わっています。その青い文字を左ダブルクリックしてください。左肩に赤い文字「立ち絵へのダブルクリックが届いた。」が出れば合格です（文字は 60 秒で消えます）。' +
        '\0\s[1110]\b[tate1]ここをダブルクリック\e'; Double = $red }
    10 = @{ Root = 'A'; ExitMs = 120000; TimeoutMs = 8000; Talk =
        '\1\s[10]項目 10。右肩に青い文字が出たら、その文字の上にポインタを置いて動かさずに待ってください。台詞が終わって 8 秒たっても消えなければ合格です。そのあとポインタを立ち絵からも吹き出しからも外すと、8 秒で消えます。' +
        '\0\s[1110]\b[tate1]ここに置く\_w[5000]\e' }
}

function Item-Lua([int]$n, [hashtable]$it) {
    $double = if ($it.Double) { "[==[$($it.Double)]==]" } else { 'nil' }
    @"
--- areka-P0-shell-balloon 実機の確かめ（項目 $n）用に、target\sbx の写しの中だけで差し替えた台本。
--- 起動の台詞 1 本だけを出す。ゴースト自身の台詞（ランダムトーク・なでなで・メニュー・選択肢の時間切れ・
--- シェル切替の台詞）は、この回のあいだ黙らせる。
local REG = require("pasta.shiori.event.register")

local TALK = [==[$($it.Talk)]==]
local DOUBLE = $double
local function quiet(act) return nil end

REG.OnBoot = function(act) return TALK end
REG.OnFirstBoot = REG.OnBoot
REG.OnSecondChange = quiet
REG.OnTalk = quiet
REG.OnMouseMove = quiet
REG.OnChoiceTimeout = quiet
REG.OnShellChanging = quiet
REG.OnShellChanged = quiet
REG.OnBalloonChange = quiet
REG.OnGhostChanging = quiet
REG.OnMouseDoubleClick = function(act) return DOUBLE end
REG.OnSbYes = function(act) return [==[\0\s[1111]\b[fuda]はいを選んだ。\_w[5000]\e]==] end
REG.OnSbNo = function(act) return [==[\0\s[1111]\b[fuda]いいえを選んだ。\_w[5000]\e]==] end

return REG
"@
}

function Write-Talk([string]$root, [string]$lua) {
    $lua = $lua -replace "(?<!`r)`n", "`r`n"
    [IO.File]::WriteAllText((Join-Path $root 'ghost\emo2\ghost\master\scripts\pasta\shiori\event\second_change.lua'), $lua, $utf8)
}

function Legacy-Lua([string]$x) {
    if ($x -eq 'A') { Lua-Script $bootA $stagesA $shellChangedA } else { Lua-Script $bootB @() $shellChangedA }
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
        # 根のフォルダそのものは消さず中身だけを消す（シェルの今いる場所が根の中だと、フォルダは消せない）。
        New-Item -ItemType Directory -Force $root | Out-Null
        Get-ChildItem -Force $root | Remove-Item -Recurse -Force
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
        Add-Text $surfaces ($braces + "`n" + $masterPlaces + "`n" + $itemPlaces)
        if ($x -eq 'B') { Add-Text $surfaces $overflow }
        Write-Talk $root (Legacy-Lua $x)
        "prepared $root (helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')) areka=$(Machine (Join-Path $root 'areka.exe')))"
    }
}

if ($Item -and -not $Run) { throw "-Item は -Run と一緒に使う（項目 5 は -Run B、ほかは -Run A）" }

if ($Run) {
    $root = Join-Path $base $Run
    $ghost = Join-Path $root 'ghost\emo2'
    $timeoutMs = 5000
    if ($Item) {
        $it = $items[$Item]
        if ($it.Root -ne $Run) { throw "項目 $Item は根 $($it.Root) で確かめる（-Run $($it.Root) -Item $Item）" }
        if (-not $PSBoundParameters.ContainsKey('ExitMs')) { $ExitMs = $it.ExitMs }
        if (-not $PSBoundParameters.ContainsKey('RustLog') -and $it.Trace) { $RustLog += ',kanade=trace' }
        $timeoutMs = $it.TimeoutMs
        Write-Talk $root (Item-Lua $Item $it)
    } else {
        Write-Talk $root (Legacy-Lua $Run)
    }
    # 前の回の記憶を消す。最後に使ったシェルはゴースト側の記憶（ghost\master\profile\areka）にも残るので、
    # 消さないと前の回で替えた先のシェル（second）で起きる。
    foreach ($p in 'profile', 'tmp', 'shots', 'ghost\emo2\ghost\master\profile\areka') { $d = Join-Path $root $p; if (Test-Path $d) { Remove-Item -Recurse -Force $d } }
    New-Item -ItemType Directory -Force (Join-Path $root 'tmp') | Out-Null
    Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
    $env:NO_COLOR = '1'
    $env:RUST_LOG = $RustLog
    $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
    $env:AREKA_BALLOON_TIMEOUT_MS = "$timeoutMs"
    $env:AREKA_ROOT = $root
    $env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
    $env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
    $p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$ghost`"" -WorkingDirectory $root `
        -RedirectStandardOutput (Join-Path $root 'run.log') -RedirectStandardError (Join-Path $root 'run.err.log') -PassThru
    "$Run item=$Item pid=$($p.Id) start=$(Get-Date -Format o) exit_ms=$ExitMs timeout_ms=$timeoutMs" | Tee-Object -Append (Join-Path $base 'runs.txt')
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
