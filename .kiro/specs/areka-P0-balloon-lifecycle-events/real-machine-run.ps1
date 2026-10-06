<#
.SYNOPSIS
  areka-P0-balloon-lifecycle-events タスク 7.2（実機での確認）の根づくりと起動。

.DESCRIPTION
  -Prepare : areka（debug）と 32bit の補助プロセスを組み、検体 emo2 を nar-sample-path で展開して
             ワークツリーの target\ble-signoff\root に写す。写しの辞書にだけ 3 つのイベントの応答
             （dic\zz_balloon_lifecycle.pasta）を足す。製品のコードとリポジトリの検体には触れない。
  -Run Rn  : 根で areka を絶対パスで起こし、終わるまで待つ（有界の自動終了）。記録は target\ble-signoff\run-Rn.log。
             起動の前に記憶（profile）を消す。既定の待ち時間は AREKA_BALLOON_TIMEOUT_MS で短くする。
  -Clicks  : 走行中に記録を読みながら、バルーンを左ダブルクリックする（SendInput）。順に 1 つずつ裁く。
             Break = 次にバルーンが現れてから -BreakDelayMs 後（台詞の途中）に叩く
             Close = 次に台詞が終わってから（steady_talk_done）-CloseDelayMs 後に叩く
             叩くのは -Scope のバルーン（0＝むらさき・1＝エモ）。叩いた直後にポインタを元の位置へ戻す（バルーンの上に残すと時間切れが抑止される）。
             入力デスクトップが Default でない（画面のロック・スクリーンセーバー）ときは叩かずに記録だけ残す。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/areka-P0-balloon-lifecycle-events/real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File .kiro/specs/areka-P0-balloon-lifecycle-events/real-machine-run.ps1 -Run R1
  pwsh -NoProfile -File .kiro/specs/areka-P0-balloon-lifecycle-events/real-machine-run.ps1 -Run R2 -Clicks Break,Close -TimeoutMs 15000
#>
param(
    [switch]$Prepare,
    [ValidatePattern('^R\d+$')]
    [string]$Run,
    [ValidatePattern('^((Break|Close)(,(Break|Close))*)?$')]
    [string]$Clicks = '',  # pwsh -File では配列を渡せないので、カンマ区切りの 1 つの文字列で受ける
    [int]$ExitMs = 90000,
    [int]$TimeoutMs = 8000,
    [int]$BreakDelayMs = 1500,
    [int]$CloseDelayMs = 700,
    [ValidateSet(0, 1)]
    [int]$Scope = 0,
    [string]$RustLog = 'info,kanade=trace,areka::input_events=trace'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$base = Join-Path $wt 'target\ble-signoff'
$root = Join-Path $base 'root'

function Machine([string]$exe) {
    $b = [IO.File]::ReadAllBytes($exe)
    $pe = [BitConverter]::ToInt32($b, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

# 写しの辞書にだけ足す応答。OnBalloonTimeout は、時間切れになった台本（Reference0）に
# balloontimeout の指定が無ければ 3 秒の指定つきの台詞を、あれば指定の無い台詞を返す
# （無人の走行でも ⑷「3 秒で消える → 次のトークは既定へ戻る」を交互に観測するため）。
$dic = @'
＃ zz_balloon_lifecycle.pasta - areka-P0-balloon-lifecycle-events 7.2 の実機確認のための応答（target の写しだけに置く）

＊OnBalloonBreak
　むらさき：＠驚き　止められてもた。
　　　エモ：＠静観　中断の知らせ、届いたよ。

＊OnBalloonClose
　　　エモ：＠通常　\![set,balloontimeout,3000]閉じたね。これは3秒で消えるよ。

＊OnBalloonTimeout
```lua
function SCENE.on_balloon_timeout(act)
    local save, var = act:init_scene(SCENE)
    local ref = act.req.reference or {}
    local script = ref[0] or ""
    if string.find(script, "balloontimeout", 1, true) then
        return act:call(SCENE.__global_name__, "時間切れ既定", {})
    end
    return act:call(SCENE.__global_name__, "時間切れ短縮", {})
end
```
　＞on_balloon_timeout

＊時間切れ既定
　むらさき：＠通常　放っとかれたから消えてもた。これは既定の待ち時間で消えるで。
　　　エモ：＠静観　次は3秒で消える台詞にするね。

＊時間切れ短縮
　　　エモ：＠通常　\![set,balloontimeout,3000]時間切れの知らせ、届いたよ。これは3秒で消えるよ。
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
    New-Item -ItemType Directory -Force $root | Out-Null
    Get-ChildItem -Force $root | Remove-Item -Recurse -Force
    Copy-Item -Recurse (Join-Path $manual '*') $root
    Copy-Item (Join-Path $wt 'target\debug\areka.exe') $root
    Copy-Item $helper $root
    $dicPath = Join-Path $root 'ghost\emo2\ghost\master\dic\zz_balloon_lifecycle.pasta'
    # 最後の行にも改行が要る（無いと pasta が構文の誤りで LOAD を断る）。
    [IO.File]::WriteAllText($dicPath, (($dic -replace "`r?`n", "`r`n") + "`r`n"), [Text.UTF8Encoding]::new($false))
    "prepared $root (helper=$(Machine (Join-Path $root 'shiori-host32-helper.exe')) areka=$(Machine (Join-Path $root 'areka.exe')))"
}

Add-Type @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public static class BleInput {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [StructLayout(LayoutKind.Sequential)] struct MOUSEINPUT { public int dx, dy; public uint data, flags, time; public IntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] struct INPUT { public uint type; public MOUSEINPUT mi; public long pad; }
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr h, StringBuilder b, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h, StringBuilder b, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll", SetLastError=true)] static extern uint SendInput(uint n, INPUT[] i, int size);
  [DllImport("user32.dll", SetLastError=true)] static extern IntPtr OpenInputDesktop(uint f, bool inh, uint acc);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern bool GetUserObjectInformationW(IntPtr h, int idx, StringBuilder b, int len, out int need);
  [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr h);
  [DllImport("user32.dll")] static extern uint GetDoubleClickTime();
  public static string InputDesktop() {
    var h = OpenInputDesktop(0, false, 0x0100); if (h == IntPtr.Zero) return "<none>";
    var sb = new StringBuilder(256); int n; GetUserObjectInformationW(h, 2, sb, 512, out n); CloseDesktop(h); return sb.ToString();
  }
  [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
  // 座標は物理画素で読み書きする（Per-Monitor v2）。DPI を意識しない既定のままだと縮めた座標が返る。
  static void PerMonitor() { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
  public static List<string> Windows(uint pid) {
    PerMonitor();
    var o = new List<string>();
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p != pid) return true;
      var c = new StringBuilder(256); GetClassNameW(h, c, 256); var t = new StringBuilder(256); GetWindowTextW(h, t, 256);
      RECT r; GetWindowRect(h, out r);
      o.Add(String.Format("{0}\t{1}\t{2}\t{3}\t{4},{5},{6},{7}", h.ToInt64(), IsWindowVisible(h) ? 1 : 0, c, t, r.L, r.T, r.R, r.B)); return true; }, IntPtr.Zero);
    return o;
  }
  static INPUT Btn(uint f) { var i = new INPUT(); i.type = 0; i.mi.flags = f; return i; }
  // (x,y) を左ダブルクリックし、ポインタを元の位置へ戻す。送れた件数を返す。
  public static uint DoubleClick(int x, int y) {
    PerMonitor(); POINT before; GetCursorPos(out before);
    SetCursorPos(x, y); System.Threading.Thread.Sleep(80);
    var size = Marshal.SizeOf(typeof(INPUT)); uint sent = 0;
    sent += SendInput(2, new[] { Btn(0x2), Btn(0x4) }, size); System.Threading.Thread.Sleep(60);
    sent += SendInput(2, new[] { Btn(0x2), Btn(0x4) }, size); System.Threading.Thread.Sleep(30);
    SetCursorPos(before.X, before.Y);
    return sent;
  }
}
'@

if ($Run) {
    if (-not (Test-Path (Join-Path $root 'areka.exe'))) { throw "根がない。先に -Prepare" }
    $ghost = Join-Path $root 'ghost\emo2'
    foreach ($p in 'profile', 'tmp', 'ghost\emo2\ghost\master\profile') {
        $d = Join-Path $root $p; if (Test-Path $d) { Remove-Item -Recurse -Force $d }
    }
    New-Item -ItemType Directory -Force (Join-Path $root 'tmp') | Out-Null
    Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
    $env:NO_COLOR = '1'
    $env:RUST_LOG = $RustLog
    $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
    $env:AREKA_BALLOON_TIMEOUT_MS = "$TimeoutMs"
    $env:AREKA_ROOT = $root
    $env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
    $env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
    $log = Join-Path $base "run-$Run.log"
    $note = Join-Path $base "run-$Run.clicks.txt"
    Set-Content $note "clicks=$Clicks desktop=$([BleInput]::InputDesktop())"
    $p = Start-Process (Join-Path $root 'areka.exe') -ArgumentList "`"$ghost`"" -WorkingDirectory $root `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $base "run-$Run.err.log") -WindowStyle Hidden -PassThru
    $null = $p.Handle  # 終了コードを読めるよう、起動直後に握っておく
    "$Run pid=$($p.Id) start=$(Get-Date -Format o) exit_ms=$ExitMs timeout_ms=$TimeoutMs clicks=$Clicks" | Tee-Object -Append (Join-Path $base 'runs.txt')

    # 記録を読み進め、各手順の合図の行を待ってから叩く。合図は前の手順より後の行だけを見る。
    $seen = 0
    foreach ($step in ($Clicks -split ',' | Where-Object { $_ })) {
        $pattern = if ($step -eq 'Break') { 'visible=true' } else { 'event="steady_talk_done"' }
        $hit = $null
        while (-not $p.HasExited -and -not $hit) {
            Start-Sleep -Milliseconds 100
            $lines = @(Get-Content -LiteralPath $log -Encoding utf8 -ErrorAction SilentlyContinue)
            for ($i = $seen; $i -lt $lines.Count; $i++) { if ($lines[$i] -match $pattern) { $hit = $lines[$i]; $seen = $i + 1; break } }
        }
        if (-not $hit) { Add-Content $note "${step}: areka が先に終わった"; break }
        Start-Sleep -Milliseconds $(if ($step -eq 'Break') { $BreakDelayMs } else { $CloseDelayMs })
        $desk = [BleInput]::InputDesktop()
        $wins = [BleInput]::Windows([uint32]$p.Id)
        Add-Content $note "$step $(Get-Date -Format o) cue=[$hit] desktop=$desk"
        $wins | ForEach-Object { Add-Content $note "  win $_" }
        if ($desk -ne 'Default') { Add-Content $note "  入力デスクトップが Default でないので叩かない"; continue }
        # バルーン窓の見分け方（R2 で確かめた）: キャラクターの窓とバルーンの窓はどちらも題名が
        # キャラクターの名前（scope 0＝むらさき・scope 1＝エモ）。同じ題名の見えている 2 枚のうち、
        # 面積の小さい方がバルーン。
        $who = @('むらさき', 'エモ')[$Scope]
        $target = $wins | Where-Object { $c = $_ -split "`t"; $c[1] -eq '1' -and $c[3] -eq $who } |
            Sort-Object { $r = (($_ -split "`t")[4] -split ',') | ForEach-Object { [int]$_ }; ($r[2] - $r[0]) * ($r[3] - $r[1]) } |
            Select-Object -First 1
        if (-not $target) { Add-Content $note "  見えているバルーン窓が無いので叩かない"; continue }
        $r = (($target -split "`t")[4] -split ',') | ForEach-Object { [int]$_ }
        $x = [int](($r[0] + $r[2]) / 2); $y = [int](($r[1] + $r[3]) / 2)
        $sent = [BleInput]::DoubleClick($x, $y)
        Add-Content $note "  double-click at $x,$y sent=$sent $(Get-Date -Format o)"
    }
    $p.WaitForExit()
    "$Run exit=$($p.ExitCode) end=$(Get-Date -Format o)" | Tee-Object -Append (Join-Path $base 'runs.txt')
}
