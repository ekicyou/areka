# 検証の記録（session-crate-split）

## 着手のときの証跡

### テストの一覧（1.1）

- コマンド: `cargo test --workspace --no-fail-fast -- --list`（標準出力を `target/session-crate-split/tests-before.txt` に残した。14755 行・終了コード 0）。
- 基点のコミット: `ba887213c`（枝 `claude/kiro-start-session-crate-split-56c35d`・コードの変更なし）。
- `: test$` の行は 14443 行。`: test$` の行から `: test` と先頭の `パス::` を落として名前の末尾の多重集合を作れる形である（異なる末尾 14330 種・合計 14443。同名が 113 件重なる）。段 6 では同じ手順で作って比べる。

### 兄弟との照らし（1.1）

兄弟 2 本の作業の木（読むだけ）で、触るファイルを設計の「同じウェーブとの重なり」と照らした。

- `test-name-work-tags`（木 `kiro-start-pilot-windows-rs-next-86646c`・tasks.md あり）: 名前の付け替えが中心。本体の `runtime_tests/zorder_wiring_tests.rs` は範囲から外れており、この仕様の `include_str!` と `t_zwi05` の字面の書き換えと重ならない（⑴）。名前の行を直す `doc/COMPAT_ARCHITECTURE.md` §8 はこの仕様がパスの行だけを直す許された重なり（⑷）で、後から着地する側が手で解く。`.kiro/specs/audit-areka-app/brief.md` は向こうが 1 項目を足し、こちらも書き換えるので、同じファイルの別の箇所として競合しうる。`crates/areka-runtime/src/consumer_ledger_test_support.rs` の 28 行・`window_state_cue_tests.rs` の 324 行は名のコメントだけで、この仕様は触らない。
- `dola-cue-test-builder`（木 `kiro-start-pilot-windows-rs-next-86646c`・brief.md のみで tasks.md は無い）: brief の触るファイルは `crates/dola/src/cue/command.rs`・`crates/dola/Cargo.toml`・`areka-ghost` の `prop_sink.rs`・`prop_get_sink_tests.rs`、`areka-seriko` の `actor_test_support.rs`、`areka-runtime` の `consumer_ledger_test_support.rs`・`frame_window_link_drain_tests.rs`・使い手の `Cargo.toml`。呼び手の名（`carrier_cue` ほか）を残す形なら、この仕様が移す・書き換えるファイルに触らない（⑵）。
- 結果: 設計が足したファイル（`tools/package.ps1`・`tools/perf/judge-perf.py`・`tools/load-flake.ps1`・`doc/ukadoc-coverage/`・`doc/ideas/`・`session_tests/` ほか）は兄弟 2 本の触るファイルと重ならない。重なりうるのは上の `doc/COMPAT_ARCHITECTURE.md` と `audit-areka-app/brief.md` の 2 件だけで、前者は設計が許した重なり、後者は後から着地する側が両方を残して解く。

### 実機の 6 つの道（1.2）

- 手順は作業の木の `target\session-crate-split\real\six-paths.ps1`（使い捨て・`tools/` には足さない）。走らせ方は `pwsh -NoProfile -File <作業の木>\target\session-crate-split\real\six-paths.ps1 -OutDir <作業の木>\target\session-crate-split\real-before`。段 6 は同じ手順を `-OutDir <作業の木>\target\session-crate-split\real-after` で回す。
- 根は走りごとに作り直す（`run1`・`run2` の下の `root`・`profile`・`tmp`）。`root\ghost` に emo2 と、`descript.txt` の `name` だけを `emo2-switch-target` に替えた写し `emo2b`（切替の先）を置く。emo2 は `root\ghost\emo2` を絶対パスで渡して起こす。`RUST_LOG=info`。areka の tracing は標準出力に出るので、標準出力と標準エラーを行ごとに 1 つのファイルへまとめる。
- 1 回目の走り（`run1.full.log`）で 起動 → メニューからの切替 → MCP の `reload`（`target=ghost`）→ `.nar` の投げ込み → メニューの終了 を順に回し、各段の始まりの行で切って `boot.log`・`switch.log`・`reload.log`・`nar-drop.log`・`menu-quit.log` に分ける。2 回目の走り（`smoke-exit.log`）は `AREKA_APP_SMOKE_EXIT_MS=40000` で起こして自動終了を待つ。1 回目の `AREKA_APP_SMOKE_EXIT_MS=900000` は安全のための上限だけ。
- メニューの項目は `#32768` の窓から `MN_GETHMENU`・`GetMenuItemRect`・`GetMenuStringW` で矩形と名を読み、`tools/mouse-input.ps1` の `click` で押す（切替は先頭の項目＝ゴースト → 子の `emo2-switch-target`、終了は最後の項目）。投げ込みはエクスプローラーの窓（`explorer.exe /select`）の `areka-balloon.nar` からキャラ窓の当たる点へ `drag` で落とす。投げ込みの書庫は `vendors\sample_ghost\areka-balloon.nar` の写し。
- 道ごとの記録の置き場: `target/session-crate-split/real-before/`（段の切れ目と押した座標は `steps.txt`）。

#### 走りの結果（2026-11-22）

- 2026-11-22 04:47:41〜04:49:59 UTC（約 2 分 20 秒）、排他テストの机を取り、開発者の SendInput の許しの下で 1 回取った。この記録の前に 2 回、手順の不具合でゴーストの切替の段で止まった（1 回目は StrictMode で `.Count` が引けない、2 回目は UI Automation でメニューの項目が見えない）。どちらも右クリックとメニューの表示（`menu_shown`）までは届き、止まった後は手順を直して `real-before` を消した。直したのは手順だけで、areka には触れていない。上の手順はこの 3 回目のものである。
- 終了コード: 1 回目の走り（起動〜メニューの終了）は 0、2 回目の走り（煙の自動終了）も 0。手順そのものも 0 で終わった。
- 記録（`target/session-crate-split/real-before/`）: `boot.log` 111 行・`switch.log` 136 行・`reload.log` 97 行・`nar-drop.log` 45 行・`menu-quit.log` 37 行（以上は `run1.full.log` 426 行を段の始まりで切ったもの）・`smoke-exit.log` 148 行・`steps.txt`（押した座標・メニューの読み取り・切れ目の行の範囲）。
- 道ごとの目印の行（行番号は各記録のもの・出所の名前は記録のまま）:
  - 起動（`boot.log`）: 19 `areka::smoke_exit` 自動 close のゲート有効（1 回目は安全のための上限 900000 ms）→ 110 `kanade` `event="steady_talk_done"`。
  - メニューからの切替（`switch.log`）: 3 `areka::menu::trigger` `event="menu_shown"`（`scope=0 items=13`）→ 4 `event="menu_selected"`（`frame=Ghost` の子）→ 26 `ghost-shutdown` `ghost shutdown sequence completed`（前のゴーストを降ろした）→ 71 `areka::session::ghost_switch` `event="ghost_switch_done"` → 135 `kanade` `event="steady_talk_done"`。メニューは上の段が 15 項目（区切り 3 本を含む）、子は `えも？？`（印つき＝今のゴースト）と `emo2-switch-target` の 2 つで、後者を押した。
  - MCP の `reload`（`reload.log`）: MCP の答えは 1 回目で `OK:reload requested.…`。2 `areka::session::reload` `event="reload_started"` → 12 `ghost-shutdown` `ghost shutdown sequence completed` → 68 `event="reload_done"` → 69 `areka::session::ghost_switch` `event="ghost_switch_done"` → 95 `kanade` `event="steady_talk_done"`。
  - `.nar` の投げ込み（`nar-drop.log`）: 手順が開いたエクスプローラーの窓（40,40〜940,640・キャラ窓と重ならない）の `areka-balloon.nar` を (567,344) から (2446,1157) へ 60 段でドラッグした。5 `areka::file_drop` `event="file_drop_received"` → 15 `install` `event="install_done"` → 44 `kanade` `event="steady_talk_done"`。エクスプローラーの窓は WM_CLOSE で閉じた。
  - メニューの終了（`menu-quit.log`）: 3 `event="menu_shown"`（`scope=0 items=14`。切替のときより 1 つ多い。投げ込みで入れたバルーンが「バルーン」の子に増えた見込み）→ 4 `event="menu_selected"`（`frame=Close id=14`）→ 6 `kanade` `event="close_handshake_begin"`（`reason="user"`・OnClose の台詞を再生）→ 32 `areka::session::app_exit` `event="app_exit" origin=KanadeStopped(Quit) closed=4` → 36 `ghost-shutdown` `ghost shutdown sequence completed` → 終了コード 0。
  - 煙の自動終了（`smoke-exit.log`・`AREKA_APP_SMOKE_EXIT_MS=40000`）: 19 自動 close のゲート有効 → 120・136 `kanade` `event="steady_talk_done"` → 139 `areka::session::app_exit` `event="app_exit" origin=Smoke` → 141 `areka::smoke_exit` `ゴースト窓を despawn しました count=4` → 143 `kanade` `event="force_quit"`（WARN）→ 147 `ghost-shutdown` `ghost shutdown sequence completed` → 終了コード 0。
- メニューの終了の `app_exit` の `origin` は `Menu` でなく `KanadeStopped(Quit)` である。メニューの「終了」は OnClose の握手を通り、kanade が止まったことを受けて終わるためで、今の振る舞いのまま記録した（この仕様では変えない）。段 6 の記録も同じ値になることを比べる。
- WARN・ERROR の行は 1 回目の走りに 8 行、2 回目に 3 行で、どれも起動と切替のたびに出る次の 4 種類だけ（ERROR は 0 行）:
  - `areka_emo_text::actor::attach` 折返しの基準が描画範囲の外に解決された（emo2 のバルーンの定義の粗さ）。ゴーストを起こすたびに 1 行。
  - `shiori-host32` `[helper] SHIORI 初期化の入口: loadu event="helper_stderr"`。helper を起こすたびに 1 行。
  - `areka_placement::follow::window_move` 移動対象の窓の WindowHandle が未付与のため移動しない（`reload` の起こし直しの直後に 2 行）。
  - `kanade` `event="force_quit"`（煙の自動終了の終了系列）。
- 段の切れ目のあたりには、毎フレームの行が前後どちらの段に入るかの揺れがありうる。比べは 7.5 で、出所の名前を除いた行の順と中身で行う。

#### 手順 `target\session-crate-split\real\six-paths.ps1`

```powershell
<#
  Throwaway (session-crate-split D15): run the six paths once each against emo2 and keep one
  record per path in -OutDir.
    run 1: boot -> ghost switch from the menu -> MCP reload -> .nar drop from an Explorer window
           onto the character window -> menu quit. The merged log is cut at the step boundaries
           into boot.log / switch.log / reload.log / nar-drop.log / menu-quit.log.
    run 2: boot and wait for AREKA_APP_SMOKE_EXIT_MS -> smoke-exit.log.
  Menu clicks and the drag go through tools/mouse-input.ps1 (SendInput). Needs the exclusive-test
  desk and the developer's SendInput permission, except with -DryCheck (no areka, no window, no input).
  Usage: pwsh -NoProfile -File <this> -OutDir <absolute folder> [-DryCheck]
#>
param([Parameter(Mandatory)] [string]$OutDir, [switch]$DryCheck)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$Here = $PSScriptRoot
$Wt = (Resolve-Path (Join-Path $Here '..\..\..')).Path
$Exe = Join-Path $Wt 'target\debug\areka.exe'
$Mouse = Join-Path $Wt 'tools\mouse-input.ps1'
$Nar = Join-Path $Wt 'vendors\sample_ghost\areka-balloon.nar'
$SwitchName = 'emo2-switch-target'
$OutDir = [System.IO.Path]::GetFullPath($OutDir)
$null = New-Item -ItemType Directory -Force $OutDir
$Steps = Join-Path $OutDir 'steps.txt'
Set-Content -LiteralPath $Steps -Value @() -Encoding utf8NoBOM
function Note([string]$Text) {
    $line = "$((Get-Date).ToUniversalTime().ToString('HH:mm:ss.fff')) $Text"
    Add-Content -LiteralPath $Steps -Value $line -Encoding utf8NoBOM
    # Host, not the pipeline: Note runs inside functions whose output is their return value.
    Write-Host $line
}

$paths = @{}
foreach ($line in (& (Join-Path $Wt 'target\debug\nar-sample-path.exe') emo2)) {
    $k, $v = $line -split '=', 2; $paths[$k] = $v
}
if ($LASTEXITCODE -ne 0) { throw "nar-sample-path failed ($LASTEXITCODE)" }
foreach ($f in $Exe, $Mouse, $Nar, (Join-Path $Wt 'target\debug\shiori-host32-helper.exe')) {
    if (-not (Test-Path -LiteralPath $f)) { throw "missing: $f" }
}

# A fresh root with emo2 and a renamed copy (the switch target), empty profile and temp.
function New-RunRoot([string]$Name) {
    $work = Join-Path $Here $Name
    if (Test-Path $work) { Remove-Item -Recurse -Force $work }
    $root = New-Item -ItemType Directory -Force (Join-Path $work 'root\ghost')
    Copy-Item -Recurse $paths['folder'] (Join-Path $root 'emo2')
    Copy-Item -Recurse $paths['folder'] (Join-Path $root 'emo2b')
    $descript = Join-Path $root 'emo2b\ghost\master\descript.txt'
    $text = [System.IO.File]::ReadAllText($descript, [System.Text.Encoding]::UTF8)
    $renamed = ([regex]'(?m)^name,[^\r\n]*').Replace($text, "name,$SwitchName", 1)
    if ($renamed -eq $text) { throw "no name line in $descript" }
    [System.IO.File]::WriteAllText($descript, $renamed, [System.Text.UTF8Encoding]::new($false))
    foreach ($d in 'profile', 'tmp') { $null = New-Item -ItemType Directory -Force (Join-Path $work $d) }
    [pscustomobject]@{
        Work = $work; Root = (Join-Path $work 'root')
        Ghost = (Join-Path $root 'emo2'); Balloon = (Join-Path $root 'emo2\balloon\emo2-kakukaku')
    }
}

function Set-RunEnv($Run, [string]$SmokeMs) {
    $l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $l.Start()
    $port = $l.LocalEndpoint.Port; $l.Stop()
    Remove-Item Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
    $env:AREKA_ROOT = $Run.Root
    $env:AREKA_PROFILE_DIR = Join-Path $Run.Work 'profile'
    $env:TMP = Join-Path $Run.Work 'tmp'; $env:TEMP = $env:TMP
    $env:AREKA_MCP_PORT = "$port"
    $env:AREKA_NO_ALERT = '1'
    $env:NO_COLOR = '1'
    $env:PASTA_DEBUG = '0'
    $env:AREKA_APP_SMOKE_EXIT_MS = $SmokeMs
    $env:RUST_LOG = 'info'
    $port
}

$run1 = New-RunRoot 'run1'
$run2 = New-RunRoot 'run2'
$dropDir = Join-Path $Here 'drop-src'
if (Test-Path $dropDir) { Remove-Item -Recurse -Force $dropDir }
$null = New-Item -ItemType Directory -Force $dropDir
$dropNar = Join-Path $dropDir 'areka-balloon.nar'
Copy-Item $Nar $dropNar
Note "roots ready: $($run1.Root) $($run2.Root) nar=$dropNar"

& pwsh -NoProfile -File $Mouse -SelfTest
if ($LASTEXITCODE -ne 0) { throw 'mouse-input selftest failed' }

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class AkSixRun {
    // areka's tracing goes to stdout; both streams go into the one file, line by line.
    public static Process Start(string exe, string args, string outPath) {
        var w = new StreamWriter(new FileStream(outPath, FileMode.Create, FileAccess.Write, FileShare.ReadWrite), new UTF8Encoding(false));
        w.AutoFlush = true;
        var psi = new ProcessStartInfo(exe, args) {
            UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true,
            StandardOutputEncoding = Encoding.UTF8, StandardErrorEncoding = Encoding.UTF8 };
        var p = new Process { StartInfo = psi, EnableRaisingEvents = true };
        DataReceivedEventHandler h = (s, e) => { if (e.Data != null) lock (w) w.WriteLine(e.Data); };
        p.OutputDataReceived += h; p.ErrorDataReceived += h;
        p.Exited += (s, e) => { p.WaitForExit(); lock (w) w.Dispose(); };
        p.Start(); p.BeginOutputReadLine(); p.BeginErrorReadLine();
        return p;
    }

    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern int GetMenuItemCount(IntPtr m);
    [DllImport("user32.dll")] public static extern bool GetMenuItemRect(IntPtr h, IntPtr m, uint i, out RECT r);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetMenuStringW(IntPtr m, uint id, StringBuilder s, int max, uint flags);
    [DllImport("user32.dll")] public static extern uint GetMenuState(IntPtr m, uint id, uint flags);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr h, StringBuilder s, int max);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int max);
    public delegate bool EnumProc(IntPtr h, IntPtr l);

    public static string ClassOf(IntPtr h) { var sb = new StringBuilder(256); GetClassNameW(h, sb, 256); return sb.ToString(); }
    public static string TextOf(IntPtr h) { var sb = new StringBuilder(512); GetWindowTextW(h, sb, 512); return sb.ToString(); }
    // Visible top-level windows of one class, top of the z-order first (a submenu comes before its parent menu).
    public static List<IntPtr> TopWindowsOfClass(string cls) {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => { if (IsWindowVisible(h) && ClassOf(h) == cls) found.Add(h); return true; }, IntPtr.Zero);
        return found;
    }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }

    // Character windows: visible top-level windows of the process that own another of its windows (the balloons).
    public static List<IntPtr> CharactersOf(uint pid) {
        var mine = new List<IntPtr>();
        EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid) mine.Add(h); return true; }, IntPtr.Zero);
        var owners = new HashSet<IntPtr>();
        foreach (var h in mine) owners.Add(GetWindow(h, 4));
        var chars = new List<IntPtr>();
        foreach (var h in mine) if (owners.Contains(h) && IsWindowVisible(h) && GetWindow(h, 4) == IntPtr.Zero) chars.Add(h);
        return chars;
    }
}
"@
# PER_MONITOR_AWARE_V2: window rects, UIA rects and mouse-input all speak physical pixels.
$null = [AkSixRun]::SetThreadDpiAwarenessContext([IntPtr](-4))
$AE = [System.Windows.Automation.AutomationElement]
$Scope = [System.Windows.Automation.TreeScope]

if ($DryCheck) {
    foreach ($r in $run1, $run2) {
        foreach ($p in $r.Ghost, $r.Balloon, (Join-Path $r.Root 'ghost\emo2b')) {
            if (-not (Test-Path -LiteralPath $p)) { throw "dry: missing $p" }
        }
        $n = Select-String -LiteralPath (Join-Path $r.Root 'ghost\emo2b\ghost\master\descript.txt') -Pattern '^name,' -Encoding utf8
        Note "dry: $($r.Root) switch target line: $($n.Line)"
    }
    Note "dry: uia root class=$($AE::RootElement.Current.ClassName) port=$(Set-RunEnv $run1 '900000') areka=$Exe"
    Note 'dry: ok (areka not started, no window, no input)'
    exit 0
}

if ([AkSixRun]::GetForegroundWindow() -eq [IntPtr]::Zero) { throw 'no foreground window (screen saver or locked desktop?)' }

function Invoke-Mouse([string[]]$MouseArgs) {
    $out = & pwsh -NoProfile -File $Mouse @MouseArgs
    Note "mouse-input $($MouseArgs -join ' ') -> $out"
    if ($LASTEXITCODE -ne 0) { throw "mouse-input failed: $out" }
}

function Read-Log([string]$Path) {
    $fs = [System.IO.File]::Open($Path, 'Open', 'Read', 'ReadWrite')
    try { (New-Object System.IO.StreamReader($fs, [System.Text.Encoding]::UTF8)).ReadToEnd() } finally { $fs.Dispose() }
}
function Get-LineCount([string]$Path) { ([regex]::Matches((Read-Log $Path), "`n")).Count }
function Get-Count([string]$Path, [string]$Needle) { ([regex]::Matches((Read-Log $Path), [regex]::Escape($Needle))).Count }
function Wait-Count($Proc, [string]$Path, [string]$Needle, [int]$AtLeast, [int]$Seconds, [switch]$Optional) {
    $end = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $end) {
        if ($Proc.HasExited) { throw "areka exited (code $($Proc.ExitCode)) before: $Needle" }
        if ((Get-Count $Path $Needle) -ge $AtLeast) { Note "seen x$AtLeast $Needle"; return $true }
        Start-Sleep -Milliseconds 300
    }
    if ($Optional) { Note "not seen (optional) x$AtLeast $Needle"; return $false }
    throw "timeout waiting for x${AtLeast}: $Needle"
}

# Same point search as the input-crate-split escape procedure: areka keeps its windows click-through
# where the cursor is not, so put the cursor on each point (9x9 grid, nearest the centre first),
# wait 300 ms, then check the window under it. Rightmost character window first (the sakura side).
function Find-CharacterPoint([int]$ProcId) {
    $wins = @(foreach ($h in [AkSixRun]::CharactersOf([uint32]$ProcId)) {
        $r = New-Object AkSixRun+RECT; $null = [AkSixRun]::GetWindowRect($h, [ref]$r)
        [pscustomobject]@{ H = $h; R = $r }
    })
    foreach ($c in ($wins | Sort-Object { $_.R.R } -Descending)) {
        $r = $c.R; $w = $r.R - $r.L; $hh = $r.B - $r.T; $cx = $r.L + [int]($w / 2); $cy = $r.T + [int]($hh / 2)
        $pts = foreach ($i in 1..9) { foreach ($j in 1..9) { , @(($r.L + [int][math]::Floor($w * $i / 10)), ($r.T + [int][math]::Floor($hh * $j / 10))) } }
        foreach ($xy in ($pts | Sort-Object { [math]::Pow($_[0] - $cx, 2) + [math]::Pow($_[1] - $cy, 2) })) {
            Invoke-Mouse @('-Action', 'move', '-X', "$($xy[0])", '-Y', "$($xy[1])")
            Start-Sleep -Milliseconds 300
            $pt = New-Object AkSixRun+POINT; $pt.X = $xy[0]; $pt.Y = $xy[1]
            if ([AkSixRun]::WindowFromPoint($pt) -eq $c.H) {
                Note "character hwnd=0x$($c.H.ToString('X')) rect=$($r.L),$($r.T),$($r.R),$($r.B) at x=$($xy[0]) y=$($xy[1])"
                return [pscustomobject]@{ X = $xy[0]; Y = $xy[1]; Rects = @($wins | ForEach-Object { $_.R }) }
            }
        }
    }
    throw "no point on a character window of pid $ProcId"
}

# areka's menu is a plain HMENU shown by TrackPopupMenuEx: read it through its #32768 window.
# A separator has no string (GetMenuState's flag bits are not usable on submenu items).
function Get-MenuWindows {
    @(foreach ($h in [AkSixRun]::TopWindowsOfClass('#32768')) {
        $wp = [uint32]0; $null = [AkSixRun]::GetWindowThreadProcessId($h, [ref]$wp)
        $hm = [AkSixRun]::SendMessageW($h, 0x01E1, [IntPtr]::Zero, [IntPtr]::Zero)  # MN_GETHMENU
        $n = if ($hm -ne [IntPtr]::Zero) { [AkSixRun]::GetMenuItemCount($hm) } else { -1 }
        $items = @(for ($i = 0; $i -lt $n; $i++) {
            $sb = [System.Text.StringBuilder]::new(256)
            $null = [AkSixRun]::GetMenuStringW($hm, [uint32]$i, $sb, 256, 0x400)  # MF_BYPOSITION
            $st = [AkSixRun]::GetMenuState($hm, [uint32]$i, 0x400)
            $r = New-Object AkSixRun+RECT; $ok = [AkSixRun]::GetMenuItemRect($h, $hm, [uint32]$i, [ref]$r)
            [pscustomobject]@{ Index = $i; Name = $sb.ToString(); Sep = ($sb.Length -eq 0); Checked = (($st -band 0x8) -ne 0); RectOk = $ok; R = $r }
        })
        [pscustomobject]@{ H = $h; Pid = $wp; Items = $items }
    })
}
function Wait-Menu([int]$ProcId, [IntPtr[]]$Exclude, [scriptblock]$Pick, [int]$Seconds) {
    $end = (Get-Date).AddSeconds($Seconds); $seen = $null
    while ((Get-Date) -lt $end) {
        $menus = @(Get-MenuWindows)
        $summary = ($menus | ForEach-Object { "0x$($_.H.ToString('X')) pid=$($_.Pid) n=$($_.Items.Count) [$(($_.Items | ForEach-Object { $_.Name + $(if ($_.Checked) { '*' } else { '' }) }) -join ' | ')]" }) -join ' ; '
        if ($summary -ne $seen) { Note "menu scan: $(if ($summary) { $summary } else { 'no #32768 window' })"; $seen = $summary }
        foreach ($m in ($menus | Where-Object { $_.Pid -eq $ProcId -and $Exclude -notcontains $_.H })) {
            $real = @($m.Items | Where-Object { -not $_.Sep })
            $hit = & $Pick $real
            if ($hit) { return [pscustomobject]@{ Menu = $m.H; Item = $hit } }
        }
        Start-Sleep -Milliseconds 200
    }
    throw 'menu item not found'
}
function Invoke-ClickMenuItem($Found) {
    $r = $Found.Item.R
    $x = [int](($r.L + $r.R) / 2); $y = [int](($r.T + $r.B) / 2)
    $pt = New-Object AkSixRun+POINT; $pt.X = $x; $pt.Y = $y
    $under = [AkSixRun]::WindowFromPoint($pt)
    Note "menu item '$($Found.Item.Name)' index=$($Found.Item.Index) rectOk=$($Found.Item.RectOk) rect=$($r.L),$($r.T),$($r.R),$($r.B) under=0x$($under.ToString('X')) menu=0x$($Found.Menu.ToString('X'))"
    if ($under -ne $Found.Menu) { throw 'the item centre is not on the menu window (coordinates off?)' }
    Invoke-Mouse @('-Action', 'click', '-X', "$x", '-Y', "$y")
}
function Open-Menu($Proc) {
    $pt = Find-CharacterPoint $Proc.Id
    Invoke-Mouse @('-Action', 'rightclick', '-X', "$($pt.X)", '-Y', "$($pt.Y)")
}

function Invoke-Mcp([int]$Port, [string]$Tool, [hashtable]$Arguments, [string]$RefusedText) {
    $body = @{ jsonrpc = '2.0'; id = 1; method = 'tools/call'; params = @{ name = $Tool; arguments = $Arguments } } | ConvertTo-Json -Depth 5
    for ($i = 0; $i -lt 15; $i++) {
        $r = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/api/mcp/v1" -Method Post -Body $body `
            -ContentType 'application/json' -Headers @{ Accept = 'application/json, text/event-stream' }
        Note "mcp $Tool attempt=$i reply=$($r.Content)"
        if (-not $r.Content.Contains($RefusedText)) { return }
        Start-Sleep -Seconds 1
    }
    throw "MCP $Tool refused 15 times"
}

function Save-Segments([string]$Log, [object[]]$Cuts) {
    $lines = @((Read-Log $Log) -split "`r?`n")
    if ($lines.Count -gt 0 -and $lines[-1] -eq '') { $lines = @($lines | Select-Object -First ($lines.Count - 1)) }
    for ($i = 0; $i -lt $Cuts.Count; $i++) {
        $from = $Cuts[$i].From
        $to = if ($i + 1 -lt $Cuts.Count) { $Cuts[$i + 1].From } else { $lines.Count }
        $part = @(if ($to -gt $from) { $lines[$from..($to - 1)] })
        [System.IO.File]::WriteAllLines((Join-Path $OutDir "$($Cuts[$i].Name).log"), [string[]]$part, [System.Text.UTF8Encoding]::new($false))
        Note "segment $($Cuts[$i].Name): lines $from..$($to - 1) ($($part.Count))"
    }
}

$steady = 'event="steady_talk_done"'

# ---- run 1: boot, switch, reload, drop, menu quit -------------------------------------------
# Safety exit only (origin=Smoke in run 1 would mean the menu quit did not land).
$port = Set-RunEnv $run1 '900000'
$log1 = Join-Path $OutDir 'run1.full.log'
$p = [AkSixRun]::Start($Exe, "`"$($run1.Ghost)`" `"$($run1.Balloon)`"", $log1)
Note "run1 started pid=$($p.Id) port=$port"
$cuts = @(@{ Name = 'boot'; From = 0 })
$eh = $null
try {
    $null = Wait-Count $p $log1 $steady 1 90
    Start-Sleep -Seconds 2

    $cuts += @{ Name = 'switch'; From = (Get-LineCount $log1) }
    Open-Menu $p
    $top = Wait-Menu $p.Id @() { param($items) if ($items.Count -gt 1) { $items[0] } } 10
    Invoke-ClickMenuItem $top
    # The submenu child: by its label, else the one ghost that is not the current (checked) one.
    Invoke-ClickMenuItem (Wait-Menu $p.Id @($top.Menu) { param($items)
            $byName = @($items | Where-Object { $_.Name -like "*$SwitchName*" })
            if ($byName.Count -gt 0) { $byName[0] } else { @($items | Where-Object { -not $_.Checked }) | Select-Object -First 1 } } 10)
    $null = Wait-Count $p $log1 'event="ghost_switch_done"' 1 90
    $null = Wait-Count $p $log1 $steady 2 90
    Start-Sleep -Seconds 2

    $cuts += @{ Name = 'reload'; From = (Get-LineCount $log1) }
    $before = Get-Count $log1 $steady
    Invoke-Mcp $port 'reload' @{ target = 'ghost' } 'Cannot reload now'
    $null = Wait-Count $p $log1 'event="reload_done"' 1 90
    $null = Wait-Count $p $log1 $steady ($before + 1) 30 -Optional
    Start-Sleep -Seconds 2

    $cuts += @{ Name = 'nar-drop'; From = (Get-LineCount $log1) }
    $target = Find-CharacterPoint $p.Id
    $known = @([AkSixRun]::TopWindowsOfClass('CabinetWClass'))
    Start-Process explorer.exe "/select,`"$dropNar`""
    $end = (Get-Date).AddSeconds(20)
    while (-not $eh -and (Get-Date) -lt $end) {
        Start-Sleep -Milliseconds 300
        $eh = @([AkSixRun]::TopWindowsOfClass('CabinetWClass') | Where-Object { [AkSixRun]::TextOf($_) -like '*drop-src*' }) | Select-Object -First 1
    }
    if (-not $eh) {
        throw "explorer window for drop-src not found; CabinetWClass: $((@([AkSixRun]::TopWindowsOfClass('CabinetWClass')) | ForEach-Object { "0x$($_.ToString('X')) '$([AkSixRun]::TextOf($_))'" }) -join ' ; ')"
    }
    # Explorer may open the folder as a tab of a window the developer already has: never move or close that one.
    if ($known -contains $eh) { $eh = $null; throw 'drop-src opened in an existing explorer window (tab); close it by hand' }
    $explorer = $AE::FromHandle($eh)
    # Top-left of the primary screen, clear of the character windows (checked below).
    $null = [AkSixRun]::SetWindowPos($eh, [IntPtr]::Zero, 40, 40, 900, 600, 0x0004)
    Start-Sleep -Milliseconds 800
    $er = New-Object AkSixRun+RECT; $null = [AkSixRun]::GetWindowRect($eh, [ref]$er)
    foreach ($c in $target.Rects) {
        if ($er.L -lt $c.R -and $c.L -lt $er.R -and $er.T -lt $c.B -and $c.T -lt $er.B) { throw "explorer $($er.L),$($er.T),$($er.R),$($er.B) overlaps a character window" }
    }
    $itemCond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
    $item = $null; $end = (Get-Date).AddSeconds(10)
    while (-not $item -and (Get-Date) -lt $end) {
        $item = @($explorer.FindAll($Scope::Descendants, $itemCond) | Where-Object { $_.Current.Name -like 'areka-balloon*' }) | Select-Object -First 1
        if (-not $item) { Start-Sleep -Milliseconds 300 }
    }
    if (-not $item) {
        # Fallback: the name on any element type (the view may not expose the row as ListItem).
        $any = @($explorer.FindAll($Scope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -like 'areka-balloon*' })
        $rows = @($explorer.FindAll($Scope::Descendants, $itemCond)) | Select-Object -First 20 | ForEach-Object { $_.Current.Name }
        Note "no ListItem match; ListItems: $($rows -join ' | '); any-type matches: $(($any | ForEach-Object { "$($_.Current.ControlType.ProgrammaticName) '$($_.Current.Name)'" }) -join ' | ')"
        $item = $any | Select-Object -First 1
    }
    if (-not $item) { throw 'areka-balloon item not found in explorer' }
    $b = $item.Current.BoundingRectangle
    # Details view rows are wide: grab near the icon and name, not the middle of the row.
    $sx = if ($b.Width -gt 3 * $b.Height) { [int]($b.X + [math]::Min(60, $b.Width / 3)) } else { [int]($b.X + $b.Width / 2) }
    $sy = [int]($b.Y + $b.Height / 2)
    Note "explorer hwnd=0x$($eh.ToString('X')) rect=$($er.L),$($er.T),$($er.R),$($er.B) item '$($item.Current.Name)' at x=$sx y=$sy"
    Invoke-Mouse @('-Action', 'drag', '-X', "$sx", '-Y', "$sy", '-X2', "$($target.X)", '-Y2', "$($target.Y)", '-Steps', '60')
    $null = Wait-Count $p $log1 'event="file_drop_received"' 1 15
    $null = Wait-Count $p $log1 'event="install_done"' 1 60
    $null = Wait-Count $p $log1 $steady ((Get-Count $log1 $steady) + 1) 30 -Optional
    $null = [AkSixRun]::PostMessageW($eh, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero); $eh = $null  # WM_CLOSE
    Start-Sleep -Seconds 2

    $cuts += @{ Name = 'menu-quit'; From = (Get-LineCount $log1) }
    Open-Menu $p
    Invoke-ClickMenuItem (Wait-Menu $p.Id @() { param($items) if ($items.Count -gt 1) { $items[-1] } } 10)
    if (-not $p.WaitForExit(60000)) { throw 'areka did not exit within 60s after the menu quit' }
    Note "run1 exit code=$($p.ExitCode)"
}
finally {
    if ($eh) { $null = [AkSixRun]::PostMessageW($eh, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero); Note "closed own explorer 0x$($eh.ToString('X'))" }
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force; Note "killed own pid=$($p.Id)" }
    $null = $p.WaitForExit(10000)
    Save-Segments $log1 $cuts
}

# ---- run 2: smoke auto-exit -------------------------------------------------------------------
$port = Set-RunEnv $run2 '40000'
$log2 = Join-Path $OutDir 'smoke-exit.log'
$p = [AkSixRun]::Start($Exe, "`"$($run2.Ghost)`" `"$($run2.Balloon)`"", $log2)
Note "run2 started pid=$($p.Id) port=$port"
try {
    if (-not $p.WaitForExit(150000)) { throw 'areka did not exit within 150s with AREKA_APP_SMOKE_EXIT_MS=40000' }
    Note "run2 exit code=$($p.ExitCode)"
}
finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force; Note "killed own pid=$($p.Id)" }
}
Note 'done'
```

## 段 1

### 2.1

- 口の束 `BodySystems` を `crates/areka/src/session/body_systems.rs` へ移し（`Resource`・`Clone`・`Copy`）、`after_char_pointer`・`after_balloon_pointer`・`after_runtime_wired` を足した。`of`（無ければ `#[cfg(not(test))]` で `error!(event = "body_systems_missing")`）と `NO_BODY`（3 欄とも何もしない）を置いた。
- `register_systems` は `install_input_hooks` の直後で `world.insert_resource(body)` を 1 回据える。`menu_poll`・`after_input` の呼び方と登録の順は変えていない。
- C1 の 3 件は窓を作るクロージャの中で `attach_char_pointer_handlers` → `after_char_pointer` → `attach_balloon_pointer_handlers` → `after_balloon_pointer` の順。C2 の 10 件は `wire_mouse_input` の後・`on_boot_ok` の前の `after_runtime_wired` の 1 行。本体の `BODY_SYSTEMS` に 3 欄と `attach_body_receivers`・`wire_body_after_runtime` を足した（呼ぶ順・引数・説明の行は前の集めた層の行のまま。`wire_menu` の説明の「上の入力の結線」だけ、置き場が変わって「上」が指せないので「先に済んだ入力の結線（`wire_mouse_input`）」とした）。
- 型を移したので、`log_history_tests.rs` の `moved_modules_keep_their_kinds_by_real_module_path` が `ghost_session` の出所の見本に使っていた `BodySystems` を、同じモジュールに残る `GhostSlot` へ替えた（期待の出所 `areka::session::ghost_session` と種別は変えていない）。

確かめ（2026-11-22）:

- RED: `BODY_SYSTEMS` を書き換える前の `cargo check -p areka --bin areka` が `E0063 missing fields after_balloon_pointer, after_char_pointer and after_runtime_wired`・`E0603`（旧パス）で組めない。
- `cargo check -p areka --bin areka`: 警告 0。
- `cargo test -p areka --bin areka`: 1837 passed・0 failed・3 ignored。
- `cargo fmt --all -- --check`: 差分なし。

本体の効き目を失って赤になったテスト（2.5 へ渡す一覧）: **0 件**（上の 1837 本がすべて緑）。

`git grep -n -e menu -e file_drop -e install -- crates/areka/src/session/ghost_session.rs crates/areka/src/session/ghost_session_runtime.rs` の残り（本体のモジュールを名指すコードは 0 件）:

| 行 | 中身 | 種類 |
|---|---|---|
| `ghost_session.rs:47` | `crate::session::hooks::install(world);` | 集めた層の関数名（install のモジュールではない） |
| `ghost_session.rs:48` | `crate::session::app_exit::install_input_hooks(world);` | 同上 |
| `ghost_session.rs:54` | `crate::session::write_failure::install(world);` | 同上 |
| `ghost_session.rs:61` | `(body.menu_poll)(world);` | 束の欄の名 |
| `ghost_session.rs:288・289・295・296` | 本体の `menu::attach_release_handlers`・`menu::wire_menu`・`file_drop::attach_file_drop_receivers`・`menu::shortcut::attach_shortcut_receivers` | 説明の地の文（リンクでない） |

`ghost_session_runtime.rs` は 0 件。

### 2.2

- 束に `on_switch_end`・`on_steady`・`overwrite_between`・`update_busy`・`install_busy`・`cue_sinks` の 6 欄を足し、`NO_BODY` に同じ欄（何もしない 3 つ・偽 2 つ・`areka_ghost::sink::DiscardSink` を 2 つ）を足した。
- C3: `ghost_switch.rs` の `on_switch_end` 2 件（定常到達の `attempt == WelcomeAttempt::Target`・中止の `false`、引数は前のまま）・`on_steady` の 1 行（前の install → update の 2 行）・`overwrite_between`（`close_windows_for_restart` の直後・`boot_origin_override` の前のまま）。
- C4: `reload.rs` の `overlap_or_context` の先で `let body = BodySystems::of(world);`、`update_busy` → `install_busy` の順（`Updating` → `Installing`）は前のまま。`shell_balloon_switch.rs` は `update_busy` だけ。
- C5: `runtime_boot.rs` は列を作る前に `let [install_sink, update_sink] = (BodySystems::of(world).cue_sinks)(world);`、列の 9・10 番目に `install_sink, update_sink,`。`use` の 2 行（`InstallCueSink`・`UpdateCueSink`）を外した。
- 本体の `BODY_SYSTEMS` に 6 欄と `tell_desks_steady`（`install::desk::on_steady` → `update::desk::on_steady`）・`body_cue_sinks`（`InstallCueSink(install::desk::raw_sender)` → `UpdateCueSink(update::desk::raw_sender)`）を足した。説明の行は前の集めた層の行を写した。
- 節目のログ（`ghost_switch_done`・`ghost_switch_cancelled`・`reload_refused`・`skin_switch_updating` ほか）の本文と順は触っていない（呼び出しの行だけを替えた）。
- `t_zwi05` は受け口の列の字面の 9・10 番目を `install_sink, update_sink,` に替え、`main.rs` の字面に `[ Box::new(InstallCueSink::new(install::desk::raw_sender(world))), Box::new(UpdateCueSink::new(update::desk::raw_sender(world))), ]` が在ることを同じテストに足した（名前は変えていない）。

`BODY_SYSTEMS` の 11 行と設計の BodySlots の照らし（同じ型の欄は取り違えても型で止まらないので 1 行ずつ）:

| 欄 | 型 | 本体の中身 | 設計の BodySlots | 一致 |
|---|---|---|---|---|
| `menu_poll` | `fn(&mut World)` | `menu::register_menu_poll` | `menu::register_menu_poll` | ○ |
| `after_input` | `fn(&mut World)` | `register_body_after_input` | `register_body_after_input` | ○ |
| `after_char_pointer` | `fn(&mut World)` | `menu::attach_release_handlers` | `menu::attach_release_handlers` | ○ |
| `after_balloon_pointer` | `fn(&mut World)` | `attach_body_receivers`（file_drop → menu::shortcut） | 同じ | ○ |
| `after_runtime_wired` | `fn(&mut World, &GhostRuntime)` | `wire_body_after_runtime` | 同じ | ○ |
| `on_switch_end` | `fn(&mut World, bool)` | `update::desk::on_switch_end` | 同じ | ○ |
| `on_steady` | `fn(&mut World)` | `tell_desks_steady`（install::desk::on_steady → update::desk::on_steady） | 同じ | ○ |
| `overwrite_between` | `fn(&mut World)` | `install::desk::run_overwrite_between` | 同じ | ○ |
| `update_busy` | `fn(&World) -> bool` | `update::desk::is_busy` | 同じ | ○ |
| `install_busy` | `fn(&World) -> bool` | `install::desk::is_busy` | 同じ | ○ |
| `cue_sinks` | `fn(&World) -> [Box<dyn BootCueSink>; 2]` | `body_cue_sinks`（InstallCueSink → UpdateCueSink） | 同じ | ○ |

確かめ（2026-11-22）:

- RED: `t_zwi05` の字面を先に替えた時点で `cargo test -p areka --bin areka t_zwi05` が赤（受け口の列の字面が無い・1 failed）。
- `cargo check -p areka --bin areka`: 警告 0。
- `cargo test -p areka --bin areka`: 1832 passed・5 failed・3 ignored。赤の 5 本は、`BodySystems::of` の倒れ方を一時に `crate::BODY_SYSTEMS` へ替えて回すと 5 本とも緑（書き換えは戻した）＝本体の効き目を失った赤。`runtime_boot_tests.rs` は中身を変えずに緑（`sinks_are_thirteen_with_reload_then_window_state_last` ほか）。`t_zwi05` は緑。
- `cargo fmt --all -- --check`: 差分なし。

本体の効き目を失って赤になったテスト（2.1 の一覧に足して 2.5 へ渡す・`session/` の下は ※）:

| テスト | ファイル |
|---|---|
| `install::desk::tests::resend_goes_at_once_when_steady_arrived_since_the_last_send` | `crates/areka/src/install/desk_tests.rs` |
| `install::desk::tests::a_cancelled_overwrite_switch_goes_back_to_held_and_retries_on_the_next_steady` | `crates/areka/src/install/desk_tests.rs` |
| `update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time` | `crates/areka/src/update/desk_resolve_tests.rs` |
| ※ `session::reload::tests::refusals_in_order_leave_one_record_each` | `crates/areka/src/session/reload_tests.rs` |
| ※ `session::shell_balloon_switch::tests::refusals_in_order_leave_one_warning_each_and_send_nothing` | `crates/areka/src/session/shell_balloon_switch_tests.rs` |

`git grep -n -E "(install|update)::|InstallCueSink|UpdateCueSink" -- ghost_switch.rs reload.rs shell_balloon_switch.rs runtime_boot.rs`（`crates/areka/src/session/` の下）: 本体のモジュールを名指すコードは 0 件。残りは説明の地の文 2 行（`ghost_switch.rs:504` の `update::desk::on_switch_end`・`:677` の `install::desk::run_overwrite_between`、リンクでない）。`install`・`update` の語だけで grep すると、ほかに束の欄の名（`update_busy`・`install_busy`・`cue_sinks` の受け手の局所名 `install_sink`・`update_sink`）・集めた層の名（`record_last_installed`・`last_installed_recorded`・`lastinstalled`・`RefusedReason::Installing`・`installed_for`・`install_lines`）が当たる。

### 2.3

- `crates/areka/src/smoke_exit.rs` と `smoke_exit_tests.rs` を `git mv` で `crates/areka/src/session/` の兄弟へ移した。`session/mod.rs` に `pub(crate) mod smoke_exit;` を足し、本体の `main.rs` の宣言を外した。
- 中身の書き換えは `use crate::session::app_exit;` → `use super::app_exit;` の 1 行だけ。`arm`・`smoke_exit_ms` は元から `pub(crate)` で、可視性は変えていない。
- 呼び出しの向き先: `main.rs` の `session::smoke_exit::arm`、`session/alert.rs` と `install/startup.rs` の `crate::session::smoke_exit::smoke_exit_ms`。
- 申し送り: 7.6 へ。移した `smoke_exit.rs` の `//!` の「`main.rs` の 1,000 行の上限のため外へ出した」は、集めた層に移った後は古くなる（直していない）。

### 2.4

- `crates/areka/src/session/body_systems_tests.rs`（231 行・`body_systems.rs` から `#[cfg(test)] #[path = "body_systems_tests.rs"] mod tests;` で宣言）に、記録する偽物の組 `RECORDING`（各欄が欄の名前と、`PROBE` が読むそのときの World の印をスレッドの記録へ足す。忙しさの答えは `UPDATE_BUSY`・`INSTALL_BUSY` のスレッドの値）と 3 本のテストを足した。
  - `busy_slots_are_asked_update_first`: 空の World に `RECORDING` を据え、`request_reload`（MCP・ゴースト）と `request_skin_switch_with`（シェル・台本）を更新が忙しい・暇の 2 通りで呼ぶ。読み直しは忙しいとき `Refused(Updating)` で問うのは `update_busy` だけ、暇なとき `update_busy` → `install_busy` を問うて `NoContext`。シェル・バルーンの切替はどちらも `update_busy` だけ（`Updating`・`NoContext`）。
  - `menu_poll_and_after_input_sit_in_the_registration_order`: `register_systems(…, RECORDING)` で 2 欄が 1 回ずつ `menu_poll` → `after_input` の順。系の名は bevy の `debug` の機能が無いと `<Enable the debug feature to see the name>` になって読めなかったので、系の数と型で代えた: `menu_poll` のときの `Input` の段に `user_break` の汲み出し（`drain_no_user_break_signals`・公開の関数なので型で照らす）が在り、2 欄の間に載った系の数が、`balloon` の離脱から `balloon_wheel` までの 5 つの登録を空の World で回したときの数（5）と同じ（`balloon` の離脱が `menu_poll` より前に来るか、`after_input` が `balloon_wheel` より前に来ると数が減る）。
  - `window_slots_sit_between_the_input_attachments`: `include_str!("ghost_session.rs")` を `code_only`・`squeeze`（`areka_runtime::source_text_test_support`）にかけ、`let spawn: BoxedCommand = Box::new(move |world: &mut World| {` から `Ok(PreparedWindows {` までの中で `attach_char_pointer_handlers` → `after_char_pointer` → `attach_balloon_pointer_handlers` → `after_balloon_pointer` の位置が昇順。
- 書き換えて赤を確かめた（どれも `git checkout --` で戻し、コミットしていない）:
  1. `ghost_session.rs` の `register_systems` で `(body.menu_poll)(world);` と `areka_input::balloon::register_balloon_leave_system(world);` の 2 行を入れ替え → `menu_poll_and_after_input_sit_in_the_registration_order` だけが赤: `(menu_poll のとき user_break の汲み出しが在る, 2 欄の間に載った系の数) が違う` `left: (true, 4)` `right: (true, 5)`。
  2. `ghost_session.rs` の窓を作るクロージャで `(BodySystems::of(world).after_char_pointer)(world);` と `areka_input::balloon::attach_balloon_pointer_handlers(world);` の 2 行を入れ替え → `window_slots_sit_between_the_input_attachments` が赤（`窓を作るクロージャの中の差し込みと口の順が違う`）。
  3. `reload.rs` の `overlap_or_context` で `update_busy` と `install_busy` の判定の 2 つの `if` を入れ替え（2 と同じ回で回した）→ `busy_slots_are_asked_update_first` が赤: `left: ((Refused(Updating), ["install_busy", "update_busy"]), (Refused(NoContext), ["install_busy", "update_busy"]))` `right: ((Refused(Updating), ["update_busy"]), (Refused(NoContext), ["update_busy", "install_busy"]))`。
- `cargo test -p areka --bin areka`: 1835 通過・5 赤・3 無視。赤は 2.1〜2.3 から持ち越しの 5 本（`install::desk::tests::resend_goes_at_once_when_steady_arrived_since_the_last_send`・`install::desk::tests::a_cancelled_overwrite_switch_goes_back_to_held_and_retries_on_the_next_steady`・`update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time`・`session::reload::tests::refusals_in_order_leave_one_record_each`・`session::shell_balloon_switch::tests::refusals_in_order_leave_one_warning_each_and_send_nothing`。2.5 で直す）だけで、増えていない。
- 各テストの頭で `reset()` が `PROBE` を既定へ戻し、記録を捨て、`UPDATE_BUSY`・`INSTALL_BUSY` を偽へ戻す（同じスレッドで順に走ったとき、前のテストの印の読み手が `Schedules` の無い World で落ちないように）。`cargo test -p areka --bin areka body_systems -- --test-threads 1` と既定の並列の両方で 3 本緑。
- `cargo check -p areka --bin areka --tests` 警告 0・`cargo clippy -p areka --bin areka --tests` で新しいファイルへの指摘 0・`cargo fmt --all -- --check` 差分なし。

### 2.5

本体の効き目を失って赤になったテストに、World を組んだ直後（足場の `rig`・`world()` を作った次の行）の `insert_resource(crate::BODY_SYSTEMS);` の 1 行だけを足した（表明は変えていない）。2.1〜2.4 で渡された 5 本がそのまま赤で、ほかに増えていない。

足したテスト（`session/` の下は ※。集めた層は最後に `crate::BODY_SYSTEMS` を名指せないので、3.2・3.4 で本体へ移すか、`NO_BODY` に替えて理由を記録する）:

| テスト | ファイル:足した行 |
|---|---|
| `install::desk::tests::resend_goes_at_once_when_steady_arrived_since_the_last_send` | `crates/areka/src/install/desk_tests.rs:385` |
| `install::desk::tests::a_cancelled_overwrite_switch_goes_back_to_held_and_retries_on_the_next_steady` | `crates/areka/src/install/desk_tests.rs:554` |
| `update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time` | `crates/areka/src/update/desk_resolve_tests.rs:558` |
| ※ `session::reload::tests::refusals_in_order_leave_one_record_each` | `crates/areka/src/session/reload_tests.rs:183` |
| ※ `session::shell_balloon_switch::tests::refusals_in_order_leave_one_warning_each_and_send_nothing` | `crates/areka/src/session/shell_balloon_switch_tests.rs:287` |

確かめ（2026-11-22）:

- RED（足す前）: `cargo test -p areka --bin areka` が 1835 passed・5 failed・3 ignored（赤は上の 5 本）。
- `cargo test -p areka --bin areka`: 1840 passed・0 failed・3 ignored。
- `cargo check -p areka --bin areka --tests`: 警告 0。
- `cargo fmt --all -- --check`: 差分なし。

#### 1.10 の数

式（設計の Verification の 1.10）を本番のファイル（`*_tests.rs`・`*test_support*.rs` を除く 36 本）にかけた:

```
git grep -nE "crate::(menu|install|update|file_drop|mcp|cli|offline_dump|roots|shiori_host|shiori_session|shiori_demo|reference_brain|thread_roles|tick_gate_config|png_image|perf_thread_report|smoke_exit|BODY_SYSTEMS|after_run|finish_after_run|boot_first_ghost|install_boot_context)\b|use crate::\{" -- crates/areka/src/session ':!*_tests.rs' ':!*test_support*'
```

当たりは 1 行（`boot_config.rs:209` の `&crate::cli::parse::parse(args).options.boot,`）で、`#[cfg(test)]` の `resolve_boot_from`（199 行の印から関数の終わりまで）の中。`//` の説明の行の当たりは 0。式が除く `#[cfg(test)]` の塊と `//` の行を除くと、**本番 0 行**。`crate::session::roots` は式に当たらない（`crate::roots` の前が `session::` のため）。

`#[cfg(test)]` の塊と `//` の行を除く数え方は、行ごとに `#[cfg(test)]` の印の次の項目（複数行の署名は最初の `{` か `;` まで）を波括弧の深さで飛ばし、`//` で始まる行を飛ばす使い捨ての python で数えた（36 本・当たり 0 行。同じ数え方で `#[cfg(test)]` の中の当たりは上の 1 行）。

#### テストだけに残る参照（50 行・21 ファイル）

上の式をテストのファイル（`crates/areka/src/session/*_tests.rs`・`*test_support*`）にかけた結果（`git grep -c`）と、ほかに本番のファイルの `#[cfg(test)]` の中の 1 行:

| ファイル | 行の数 | 名指す本体の項目 |
|---|---|---|
| `app_exit_escape_tests.rs` | 1 | `crate::BODY_SYSTEMS` |
| `boot_config_callghost_tests.rs` | 1 | `crate::cli::parse` |
| `boot_config_module_policy_tests.rs` | 1 | `crate::install_boot_context` |
| `boot_resolve_mark_text_tests.rs` | 1 | `crate::install::desk::remember_balloon` |
| `ghost_session_reload_tests.rs` | 2 | `crate::BODY_SYSTEMS` |
| `ghost_session_restart_tests.rs` | 3 | `crate::menu`（`Frame` ほか・`register`）・`crate::BODY_SYSTEMS` |
| `ghost_session_scope_rect_boot_tests.rs` | 2 | `crate::mcp::{install, drain}` |
| `ghost_session_scope_scale_boot_tests.rs` | 3 | `crate::shiori_host::ShioriHostSink`・`crate::mcp::{install, drain}` |
| `ghost_session_strict_tests.rs` | 1 | `crate::menu::MenuWiring` |
| `ghost_session_switch_fallback_tests.rs` | 1 | `crate::after_run` |
| `ghost_session_switch_memory_tests.rs` | 1 | `crate::after_run` |
| `ghost_session_switch_tests.rs` | 1 | `crate::menu::{Frame, MenuWiring}` |
| `ghost_switch_test_support.rs` | 1 | `crate::BODY_SYSTEMS`（`SwitchRig` の `register_systems`。3.1 で `body` に替える） |
| `readonly_run_tests.rs` | 8 | `crate::after_run`・`crate::install`・`crate::update`・`crate::roots::add_root` |
| `readonly_sites_tests.rs` | 4 | `crate::boot_first_ghost`・`crate::roots::add_root`・`crate::install::desk::remember_balloon` |
| `reload_tests.rs` | 3 | `crate::install::desk`・`crate::update::desk`・※ `crate::BODY_SYSTEMS`（2.5 で足した行） |
| `session_end_tests.rs` | 1 | `use crate::{after_run, finish_after_run}` |
| `shell_balloon_switch_session_tests.rs` | 7 | `crate::menu`（`Frame`・`MenuAction`・`MenuContext` ほか） |
| `shell_balloon_switch_session_update_tests.rs` | 2 | `crate::update::desk`・`crate::update` |
| `shell_balloon_switch_tests.rs` | 2 | `crate::update::desk`・※ `crate::BODY_SYSTEMS`（2.5 で足した行） |
| `write_failure_sites_tests.rs` | 4 | `crate::boot_first_ghost`・`crate::roots::add_root`・`crate::install::desk::remember_balloon` |
| `boot_config.rs`（`#[cfg(test)] resolve_boot_from` の中） | 1 | `crate::cli::parse::parse` |

## 段 2

### 3.1

- `ghost_switch_test_support.rs` に `RigSample`（`root`・`ghost`・`bundled_balloon`・寿命を持つ `keep`）と `RigSample::of(SampleRoot)`・`SwitchRig::with(sample, body, scripts)` を足した。欄 `sample` は `RigSample`、検体を読む所は欄の読み、`register_systems` へは `body` を渡す。
- 集めた層の `SwitchRig::new(scripts)` は `SampleRoot::acquire("emo2")` → `RigSample::of` → `with(…, crate::BODY_SYSTEMS, …)` の数行（偽物の組へ替えるのは 3.2）。足場は `crate::sample_test_support::acquire_emo2` を引かなくなった。
- 本体の `sample_test_support.rs` に `switch_rig(scripts)`（`acquire_emo2()` と `crate::BODY_SYSTEMS`）を足した。

#### `git grep -c "SwitchRig::new("` の数

| | 本体 | `session/` の本体へ移すファイル | `session/` の集めた層に残すファイル | 計 |
|---|---|---|---|---|
| 前（`64a434f46`） | 22（16 ファイル） | 17（11 ファイル） | 18（13 ファイル） | 57 |
| 後 | 0 | 0 | 18（13 ファイル） | 18 |

定義（`pub(crate) fn new(scripts`）は字面が `SwitchRig::new(` でないので数に入らない。

- `switch_rig(` へ替えた本体の 16 ファイル（22 か所）: `install/desk_overwrite_tests.rs` 2・`install/desk_record_tests.rs` 1・`install/names_tests.rs` 1・`install/worker_tests.rs` 1・`main_session_mark_tests.rs` 2・`main_session_mark_write_tests.rs` 1・`mcp/check_script_tests.rs` 1・`mcp/dump_surface_gpu_test_support.rs` 1・`mcp/dump_surface_tests.rs` 1・`mcp/get_expression_table_tests.rs` 1・`mcp/get_property_tests.rs` 2・`mcp/get_status_tests.rs` 1・`mcp/mcp_tests.rs` 3・`runtime_tests/frame_stay_on_top_lifecycle_tests.rs` 1・`update/desk_reload_tests.rs` 2・`update/worker_path_tests.rs` 1
- `switch_rig(` へ替えた `session/` の 11 ファイル（17 か所・TestPlacement の表で本体へ移すもの）: `session_end_tests.rs` 1・`ghost_session_switch_fallback_tests.rs` 1・`readonly_run_tests.rs` 1・`readonly_sites_tests.rs` 1・`write_failure_sites_tests.rs` 1・`ghost_session_strict_tests.rs` 1・`ghost_session_switch_tests.rs` 4・`shell_balloon_switch_session_tests.rs` 4・`ghost_session_scope_rect_boot_tests.rs` 1・`ghost_session_scope_scale_boot_tests.rs` 1・`boot_resolve_mark_text_tests.rs` 1
- `SwitchRig::new(` が残る集めた層の 13 ファイル（18 か所）: `boot_shell_tests.rs` 2・`ghost_quit_switch_tests.rs` 1・`ghost_session_runtime_tests.rs` 1・`ghost_session_switch_translate_tests.rs` 1・`ghost_switch_balloon_tests.rs` 1・`ghost_switch_boot_event_tests.rs` 1・`ghost_switch_reload_tests.rs` 4・`ghost_switch_rig_wait_tests.rs` 2・`ghost_switch_talk_clock_tests.rs` 1・`reload_skin_tests.rs` 1・`session_end_deadline_tests.rs` 1（集めた層に残す `boot_a`）・`session_end_sync_send_tests.rs` 1・`shell_balloon_switch_session_lap_tests.rs` 1

#### 確かめ

- 赤: 呼び出しを `switch_rig(` へ替え、`switch_rig` を足す前の `cargo check -p areka --bin areka --tests` が `error[E0432]: unresolved import crate::sample_test_support::switch_rig` 29 件で止まる。
- 緑: `cargo test -p areka --bin areka` 1840 passed・0 failed・3 ignored（2.5 の終わりと同じ）。`cargo check -p areka --bin areka --tests` 0 warnings。`cargo fmt --all -- --check` 差分なし。

### 3.2

#### 比べ方

- 比べる組: `git grep -lE "SwitchRig|ghost_switch_test_support" -- crates/areka/src/session` は足場そのもの・`mod.rs` を除いて 31 ファイル。うち 3 本（`runtime_boot_tests.rs`・`readme_script_e2e_tests.rs`・`script_security_e2e_tests.rs`）は定数か `standard_script` を借りるだけで足場を組まない。足場を組むのは 28 本で、16 本が集めた層の `SwitchRig::new` に届き（直に 13 本・`shell_balloon_switch_session_lap_tests.rs` の `lap_rig`／`lap_rig_of` を通して `_abort`・`_balloon`・`_update` の 3 本）、12 本は 3.1 で本体の `switch_rig` に替えたファイル 11 本と、`ghost_session_switch_fallback_tests.rs` の `fallback_rig` を借りる `ghost_session_switch_memory_tests.rs`。後の 12 本は `SwitchRig::new` を通らず、表で既に本体と決まっているので比べから外した。設計の「30 本」とは数え方が違い、ここでは `SwitchRig::new` に届くかを走らせた記録で決めた（下の 52 本）。
- 一時の仕掛け（コミットしない・比べた後に `git checkout` で戻した）: `SwitchRig::new` へ数える組（11 の欄がどれも World の資源の数えを 1 つ足してから、環境の変数で選んだ `crate::BODY_SYSTEMS` か `NO_BODY` の同じ欄を呼ぶ）を渡し、`SwitchRig` の `Drop` でテストの名・作り口の回数・欄ごとの回数・起こした回ごとの偽の SHIORI の呼出列（状態の問い合わせを除き、`homeurl` の照会を含む）を 1 行ずつ書き出す。本体の `switch_rig` の足場は数える組の資源が無いので書き出さない。
- 回したもの: 同じビルドで `cargo test -p areka --bin areka session::` を本物の組・偽物の組で 2 回ずつ（並びは既定のまま）。`session::` の 650 本のうち `SwitchRig::new` に届いたのは 16 ファイル 52 本。比べたのは、呼出列から `homeurl` の照会を除いた列（`SwitchRig::calls` と同じ見え方）・`homeurl` の照会の数・作り口の回数・欄ごとの回数。
- 合否: 本物の組は 2 回とも 647 passed・0 failed・3 ignored。偽物の組は 2 回とも 646 passed・1 failed・3 ignored（赤は `session::ghost_session::shell_balloon_switch_session_update_tests::a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon`。`crates/areka/src/update/desk.rs:556:10` で「Requested non-send resource … does not exist in the `World`」の panic）。同じ組の 2 回どうしの差は、本物の組の `remembered_hidden_shell_reaches_all_resolutions_decided_once` の 1 本だけ（定常の知らせを汲む前後で終わる回による）。

#### 比べた結果の表

合否は「本物の組／偽物の組」。記録の差は「本物の組 → 偽物の組」で、2 回が違うときは回ごとに書く。差の無い欄は書かない。

| ファイル（旧 `session/`） | テスト | 合否 | 記録の差 | 決めた置き場 |
|---|---|---|---|---|
| `ghost_quit_switch_tests.rs` | `stop_with_handoff_under_reservation_switches_without_exit` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `boot_shell_tests.rs` | `animated_hidden_shell_boot_emits_each_load_record_once` | 緑／緑 | `on_steady` 0→1 | 集めた層（乙） |
| `boot_shell_tests.rs` | `each_switch_adds_one_more_set_of_load_records` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `boot_shell_tests.rs` | `missing_remembered_shell_boots_default_and_rewrites_memory` | 緑／緑 | なし | 集めた層（丙） |
| `boot_shell_tests.rs` | `production_boot_emits_each_load_record_once` | 緑／緑 | なし | 集めた層（丙） |
| `boot_shell_tests.rs` | `remembered_hidden_shell_reaches_all_resolutions_decided_once` | 緑／緑 | 1 回目 `homeurl` 1→0／2 回目 `on_steady` 0→1 | 集めた層（乙） |
| `ghost_session_runtime_tests.rs` | `boot_failure_returns_the_unit_without_runtime_to_the_slot` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_session_runtime_tests.rs` | `rebuilt_runtime_is_attached_to_the_same_unit` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_abort_tests.rs` | `close_request_while_waiting_drops_the_switch_and_exits_as_today` | 緑／緑 | なし | 集めた層（丙・門） |
| `shell_balloon_switch_session_abort_tests.rs` | `ghost_switch_after_a_shell_swap_unloads_the_session` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `shell_balloon_switch_session_abort_tests.rs` | `ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `shell_balloon_switch_session_abort_tests.rs` | `marked_talk_break_cancels_the_shell_switch_without_quitting` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_abort_tests.rs` | `undecodable_shell_keeps_the_old_mount_and_records_one_error` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_balloon_tests.rs` | `script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_lap_tests.rs` | `menu_switch_to_the_current_shell_rebuilds_and_raises_both_events` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_lap_tests.rs` | `plain_script_shell_switch_skips_changing_and_waits_for_the_script_end` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_lap_tests.rs` | `script_shell_switch_round_trips_with_raise_event` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_update_tests.rs` | `a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon` | 緑／赤 | 作り口 2→1・`after_runtime_wired` 2→1・`on_switch_end` 1→0・`on_steady` 1→0・`overwrite_between` 1→0・`cue_sinks` 2→1・`calls` の列が違う・`homeurl` 1→0 | 本体（ファイルごと） |
| `shell_balloon_switch_session_update_tests.rs` | `update_targets_follow_the_swapped_shell_and_balloon` | 緑／緑 | なし | 本体（ファイルごと） |
| `ghost_session_switch_translate_tests.rs` | `switch_sends_each_on_translate_to_its_own_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `a_memory_written_before_this_spec_finds_the_ghost_inner_balloon` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `a_remembered_ghost_inner_balloon_that_is_gone_moves_on_to_the_next_route` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_does_not_see_the_balloon_inside_the_previous_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_to_a_ghost_whose_descript_names_a_balloon_uses_it_and_records_the_descript_route` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_to_a_ghost_with_its_own_balloon_uses_and_remembers_it_by_folder_name` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_to_a_ghost_with_numbered_companions_uses_the_first_and_records_the_companion_route` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_boot_event_tests.rs` | `a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_boot_event_tests.rs` | `a_switch_with_a_boot_event_boots_the_target_with_that_event_instead_of_changed_or_boot` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_boot_event_tests.rs` | `a_switch_without_a_boot_event_boots_the_target_with_on_ghost_changed_as_today` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_failed_ghost_reload_hands_no_carry_to_the_default_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_failed_shiori_rebuild_falls_back_to_the_default_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_failed_shiori_rebuild_of_the_default_ghost_is_fatal` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_reload_tests.rs` | `a_foreign_switch_under_a_reload_mark_boots_as_before_and_drops_the_mark_as_overtaken` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_ghost_reload_hands_the_shown_surfaces_to_the_new_windows_but_a_switch_does_not` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_ghost_reload_keeps_the_ghost_inner_balloon_through_the_memory` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_ghost_reload_reboots_with_no_boot_notice_but_with_the_information_notices` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_shiori_rebuild_that_cannot_write_the_drop_memory_is_fatal` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_reload_tests.rs` | `a_shiori_reload_keeps_the_sticky_ledger_and_anchor_source` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_shiori_reload_keeps_the_windows_and_rebuilds_only_the_runtime` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_switch_without_the_mark_boots_with_on_ghost_changed_as_before` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_rig_wait_tests.rs` | `a_rig_wait_that_never_arrives_names_the_callers_place` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_rig_wait_tests.rs` | `the_rig_progress_ignores_status_and_counts_every_factory_call` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_talk_clock_tests.rs` | `a_rig_clock_far_ahead_of_a_stalled_handshake_leaves_the_switch_unchanged` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `reload_skin_tests.rs` | `descript_ghost_replaces_names_and_readme_then_reserves_the_current_shell` | 緑／緑 | なし | 集めた層（丙） |
| `session_end_deadline_tests.rs` | `answered_within_limit_is_not_cut` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `cut_while_in_flight_get_is_held` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `cut_while_on_close_notify_is_held` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `cut_while_unload_is_held` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `deadline_fires_without_manual_cut` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `self_timeout_shorter_than_limit_is_not_cut` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `unbounded_shutdown_never_cuts` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_sync_send_tests.rs` | `teardown_join_does_not_wait_for_a_sync_send_to_the_ui_window` | 緑／緑 | なし | 集めた層（丙・門） |
理由:

- 甲（`homeurl` の照会だけが減る・24 本）: 本物の組では定常の知らせで本体の `tell_desks_steady` → 更新の窓口が `homeurl` の照会を偽の SHIORI へ 1 回送る。`SwitchRig::calls` はこの照会をもともと除くので、表明が見る列は同じ。照会が増やすのは進みの目印（`progress_probe`）の 1 だけで、偽物の組でも待ちは打ち切られていない（2 回とも緑）。照会そのものは本体の `update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time` が見るので、集めた層に残しても守りは失わない。
- 乙（`on_steady` の回数・2 本）: 定常の知らせを汲んだ回数で、1 回起こすだけのテストは汲む前後で終わるので回による（`remembered_hidden_shell_…` は本物の組でも 1 回目 1・2 回目 0）。表明は読み込みの記録の数だけを見て、`on_steady` の先の本体の窓口を見ない。
- 丙（差なし・24 本）: 呼出列・照会・欄の回数・作り口の回数が同じ。
- 門（`exit_wait` の門を通るもの）: `session_end_deadline_tests.rs` の 7 本・`session_end_sync_send_tests.rs` の 1 本（`end_session`／`end_session_within`）と `close_request_while_waiting_drops_the_switch_and_exits_as_today`（`on_ghost_os_close`）。偽物の組では本体の install・update の門が据わらないが、本物の組でも門は空で待たず、記録も欄の回数も差が無い（`update_busy`・`install_busy` はどちらの組でも 0 回）。仕事のある門の待ちは本体の `install/worker_tests.rs`・`install/desk_exit_tests.rs`・`update/worker_tests.rs` ほかが見る。集めた層の `reload`・`shell_balloon_switch` の門は集めた層の登録で据わるので組によらない。
- `UpdateDesk` の照会（`update_busy`）が呼ばれたのは `_update` の赤の 1 本（どちらの組でも 2 回）だけ。照会が真のときの断りを見るのは `reload_tests.rs`・`shell_balloon_switch_tests.rs`（下の ※）で、どちらも本体へ移す。
- 本体（`shell_balloon_switch_session_update_tests.rs`・ファイルごと）: 赤の 1 本は更新の窓口の読み直し（`ask_reload_for_test`）を通すので本体の窓口が要る。足場を集めた層の `lap_rig` → `SwitchRig::new` から受けていて、3.1 の `SwitchRig::new(` の字面の数えに入らなかった。もう 1 本は差なしだが足場 `steady_rig` を分け合う。表の既にある行（install・update の本物を見る）のとおりファイルごと移す。

#### 決めたことと直したこと

- 集めた層の `SwitchRig::new` は `NO_BODY` を渡す（`ghost_switch_test_support.rs`）。
- `app_exit_escape_tests.rs` の `register_systems` へ渡す `crate::BODY_SYSTEMS` を `crate::session::body_systems::NO_BODY` へ替えた（表明は変えていない）。
- ※ 2.5 の 2 行（`reload_tests.rs:183`・`shell_balloon_switch_tests.rs:287` の `insert_resource(crate::BODY_SYSTEMS)`）: どちらのファイルも TestPlacement の表でファイルごと本体へ移す（D8 ⑵）ので本体へ移すと決め、1 行はそのまま 3.4 で移す。
- 本体へ移すと決めて偽物の組で赤になる `shell_balloon_switch_session_update_tests.rs` は、3.4 で移すまで本体の `switch_rig` で走らせる: `shell_balloon_switch_session_lap_tests.rs` の `lap_rig_of` の後半を `lap_rig_on(rig, shells)`（呼び手が組んだ足場を受ける形）に分け、`steady_rig` は `lap_rig_on(switch_rig(…), &[SECOND])` で組む。3.4 の「本体へ移すと決めたファイルの `SwitchRig::new(` を `switch_rig(` へ」と同じ書き換えで、このファイルには `SwitchRig::new(` の字面が無いので足場の受け口を分けた。3.4 で移すとき、このファイルは `shell_balloon_switch_session_lap_tests.rs` の `lap_rig_on` ほかの小道具を引くので、4.5 の開け方の対象になる。
- 設計の TestPlacement の表に、上の本体の行・甲・乙・門・※ の行を足した。
- `SwitchRig::new(` の字面の数は 18（13 ファイル）のまま。

#### 確かめ

- 赤（比べそのもの）: 偽物の組で `a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon` が 2 回とも赤（上）。
- 緑: `cargo test -p areka --bin areka` 1840 passed・0 failed・3 ignored（`SwitchRig::new` が `NO_BODY` を渡す形で）。`cargo check -p areka --bin areka --tests` 警告 0。`cargo fmt --all -- --check` 差分なし。
- 一時の仕掛けが残っていない: `git diff` の差は `ghost_switch_test_support.rs`（`NO_BODY` の 3 行）・`app_exit_escape_tests.rs`（1 行）・`shell_balloon_switch_session_lap_tests.rs`（`lap_rig_on`）・`shell_balloon_switch_session_update_tests.rs`（`steady_rig`・`use`・`//!` の文）だけ。

### 3.3

#### 足場の道の確かめ（どちらの枝を取ったか）

- 取った枝: **C1 の 2 欄（`after_char_pointer`・`after_balloon_pointer`）を表明に含める**（字面で呼び手の数を数える形へは替えない）。
- 理由: `SwitchRig::boot` は `boot_ghost` だけで窓を開かないが、切替の道（`ghost_switch.rs` の起こし直し）は `reopen_ghost_windows` → `boot_ghost_strict` → `commit_ghost_windows` で窓を作るクロージャを作業プールへ投函する。足場に `WintfTaskPool::with_threads(1)` を据え、`Input` の段に `drain_task_pool_commands` を足すと、`pump_input_until` が回す `Input` の段で投函された閉包がこの World に当たる（既にある `ghost_session_switch_fallback_tests.rs` の `windows_rig` と同じ形）。A の窓は `fn main` と同じく `open_ghost_windows` を `boot` の前に呼んで開く。走らせて、窓を開くたびに 2 欄が 1 回ずつ呼ばれることを記録で見た。

#### 足したテスト

- `session/body_systems_tests.rs` に `body_slots_follow_the_boot_and_switch_order`。`SwitchRig::with(RigSample::of(SampleRoot::acquire("emo2")), RECORDING, …)` に台詞を返さない A・B（起動記録を植える）を渡し、A を窓ごと起こして定常、B へ切り替えて B の窓と定常まで回す。先頭で `reset()`。
- B の窓の閉包と B の定常の知らせはどちらが先に着くかが決まらないので、全体の並びは比べず、窓の欄の並び・窓の外の欄の並び・窓の欄の位置（起こした回の `after_runtime_wired` の後・A の窓は `overwrite_between` の前）を比べる。

#### 記録の中で数えた回数（1 回の走りの記録）

`["menu_poll", "after_input", "cue_sinks", "after_runtime_wired", "after_char_pointer", "after_balloon_pointer", "on_steady", "overwrite_between", "cue_sinks", "after_runtime_wired", "after_char_pointer", "after_balloon_pointer", "on_switch_end(true)", "on_steady"]`

| 欄 | 登録 | A の起動（窓を開く） | B への切替（起こし直し） | 計 |
|---|---|---|---|---|
| `menu_poll`・`after_input` | 1・1 | 0 | 0 | 1・1 |
| `cue_sinks` → `after_runtime_wired` | 0 | 1 → 1 | 1 → 1 | 2・2 |
| `after_char_pointer` → `after_balloon_pointer` | 0 | 1 → 1 | 1 → 1 | 2・2 |
| `overwrite_between` | 0 | 0 | 1（B の `cue_sinks` より前） | 1 |
| `on_switch_end(true)` → `on_steady` | 0 | `on_steady` だけ 1（予約の外の定常） | 1 → 1 | 1・2 |
| `update_busy`・`install_busy`・`on_switch_end(false)` | 0 | 0 | 0 | 0 |

#### 確かめ

- 赤（一時の摂動・コミットしない）: `ghost_switch.rs` の `(BodySystems::of(world).overwrite_between)(world);` をコメントに替えると、このテストが `assertion left == right failed` で赤（窓の外の欄から `overwrite_between` が消え、窓の欄の位置の確かめが `false`）。`git checkout crates/areka/src/session/ghost_switch.rs` で戻し、`git status` で差が `body_systems_tests.rs` だけであることを見た。
- 緑: `cargo test -p areka --bin areka body_systems -- --test-threads 1` 4 passed。`cargo check -p areka --bin areka --tests` 警告 0。`cargo fmt --all -- --check` 差分なし。`cargo test -p areka --bin areka` 1841 passed・0 failed・3 ignored。`body_systems_tests.rs` は 358 行。

### 3.4

#### 移したファイル（`git mv`・名前は変えない・24 本）

`crates/areka/src/session/` → `crates/areka/src/session_tests/`: `session_end_tests.rs`・`ghost_session_switch_tests.rs`（子の `ghost_session_switch_fallback_tests.rs`・`ghost_session_switch_memory_tests.rs` は `#[path]` の親子のまま）・`ghost_session_restart_tests.rs`・`ghost_session_reload_tests.rs`・`ghost_session_strict_tests.rs`・`ghost_session_scope_rect_boot_tests.rs`・`ghost_session_scope_scale_boot_tests.rs`・`shell_balloon_switch_session_tests.rs`・`shell_balloon_switch_session_update_tests.rs`（3.2 で本体と決めた行）・`boot_resolve_mark_text_tests.rs`・`readonly_run_tests.rs`・`readonly_sites_tests.rs`・`write_failure_sites_tests.rs`（`session_tests/mod.rs` で `#[path]` つきの `sites_tests` の名で宣言・`readonly_sites_tests.rs` は `super::sites_tests` から引く）・`boot_config_module_policy_tests.rs`・`boot_config_{balloon,copy,input,option,roots,callghost}_tests.rs`・`reload_tests.rs`・`shell_balloon_switch_tests.rs`。

- `main.rs` に `#[cfg(test)] mod session_tests;`。`session_tests/mod.rs` は宣言の並びと `resolve_boot_from`（`boot_config.rs` の `#[cfg(test)]` の口を移した。使い手は `boot_config_{balloon,copy,input,option,roots}_tests.rs` と `main_session_mark_tests.rs`〔子の `main_session_mark_fallback_tests.rs` は `use super::*` で引く〕）。
- 集めた層の親（`session_end.rs`・`readonly.rs`・`write_failure.rs`・`boot_config.rs`・`ghost_session.rs`・`ghost_session_scope_rect_tests.rs`・`ghost_session_scope_scale_tests.rs`・`boot_resolve.rs`・`reload.rs`・`shell_balloon_switch.rs`）から移したテストの宣言を外した。
- `use super::*`（`ghost_session_scope_*_boot_tests.rs` は `use super::super::*`）を元の親のモジュールの glob（`use crate::session::<元の親>::*;`）に替え、親の `use` の行のうち子が使う名だけを写した。名を指す `use super::X` は `crate::session::<元の親>::X` へ。
- `git status` で 24 本とも `R`（移動）。`git diff HEAD -M --summary` の rename が 24。中の書き換えは `use` の行だけ（最大は `reload_tests.rs` の 65 行で、大半は下の小道具の切り出し）。
- 3.2 で本体と決めた `shell_balloon_switch_session_update_tests.rs` には `SwitchRig::new(` の字面が無い（3.2 で `lap_rig_on(switch_rig(…))` に替え済み）。`git grep -n "SwitchRig::new(" -- crates/areka/src/session_tests/` は 0 件。

#### `use super::*` で引いていた私有の項目の数

移した後の最初の `cargo check -p areka --bin areka --tests`（宣言と `use` を直す前・赤）が 186 件のエラーで止まった（`target/session-crate-split/3.4-red.txt`）。エラーをファイルと名で数えた:

| 移したファイル | 親が定める私有の項目 | 親の `use` から借りていた名 |
|---|---|---|
| `ghost_session_reload_tests.rs` | 欄 `GhostSession::seriko`・`GhostSession::loop_ticker` | 11（`BalloonDecision`・`BootOrigin`・`GhostDecision`・`GhostNames`・`KanadeMsg`・`KanadeNotice`・`PathBuf`・`Schedules`・`Sender`・`World`・`mpsc`） |
| `ghost_session_restart_tests.rs` | 欄 `PreparedWindows::spawn` | 13（`BalloonDecision`・`CommandSender`・`ConfigInputs`・`FrameFinalize`・`GhostDecision`・`KanadeNotice`・`PathBuf`・`Schedules`・`Sender`・`WintfTaskPool`・`World`・`app_exit`・`mpsc`） |
| `ghost_session_strict_tests.rs` | ― | 7（`BalloonDecision`・`ConfigInputs`・`GhostDecision`・`KanadeNotice`・`PathBuf`・`World`・`mpsc`） |
| `reload_tests.rs` | 関数 `reload::drain` | 7（`DescriptNotes`・`GhostSlot`・`ReloadTarget`・`ScriptReload`・`SkinSwitchInFlight`・`SwitchInFlight`・`TARGET_ERROR`） |
| `write_failure_sites_tests.rs` | 関数 `write_failure::drain_system` | 3（`ExitOrigin`・`PersistFailure`・`World`〔`bevy_ecs::prelude::*`〕） |
| `shell_balloon_switch_tests.rs` | ― | 1（`SkinCandidate`） |
| `ghost_session_scope_rect_boot_tests.rs` | 親のテストのモジュール `tests` とその `LEFT_SCREEN`・`wiring::{fake_handle, placement}` | 1（`Arc`） |
| `ghost_session_scope_scale_boot_tests.rs` | 親のテストのモジュール `tests` とその `applied` | 1（`TargetWindow`） |
| `shell_balloon_switch_session_update_tests.rs` | 兄弟のテストのモジュール `shell_balloon_switch_session_lap_tests` とその小道具 | 0 |
| ほかの 15 本 | ― | 0（名を指す `use` だけ・`boot_config_*` は `resolve_boot_from` の置き場の書き換えだけ） |

- 親が定める私有の項目（`use super::*` の glob と親のモジュールの中の見え方で引いていたもの）は 5（関数 2・欄 3）。親の `use` から借りていた名はのべ 44 で、どれも親と同じ行を子に写した（私有の項目ではないので開けていない）。テストのモジュールの小道具は下の表。

#### `pub(crate)` で開けた項目と使う本体のテスト（段 3 で `test-support` の条件へ替える）

| 開けた項目（集めた層） | 前 | 使う本体のテストのファイル |
|---|---|---|
| `ghost_session.rs` の `GhostSession::seriko`・`GhostSession::loop_ticker`（欄） | 私有 | `ghost_session_reload_tests.rs` |
| `ghost_session.rs` の `PreparedWindows::spawn`（欄） | 私有 | `ghost_session_restart_tests.rs` |
| `reload.rs` の `drain` | 私有 | `reload_tests.rs` |
| `write_failure.rs` の `drain_system` | 私有 | `write_failure_sites_tests.rs` |
| `ghost_session_scope_rect.rs` の `mod tests`・`ghost_session_scope_rect_tests.rs` の `LEFT_SCREEN`・`mod wiring`・`wiring::fake_handle`・`wiring::placement` | 私有・`pub(super)` | `ghost_session_scope_rect_boot_tests.rs`（3.5 で `ghost_session_scope_rect_test_support.rs` へ移す） |
| `ghost_session_scope_scale.rs` の `mod tests`・`ghost_session_scope_scale_tests.rs` の `applied` | 私有 | `ghost_session_scope_scale_boot_tests.rs`（3.5 で `ghost_session_scope_scale_test_support.rs` へ移す） |
| `ghost_session.rs` の `mod shell_balloon_switch_session_lap_tests`・その `SECOND`・`UNBOUNDED`・`Ticks`・`LapRig`・`LapRig::rig`（欄）・`LapRig::frames_until`・`lap_rig_on`・`copy_tree`・`kinds`・`got`・`idle` | 私有・`pub(super)` | `shell_balloon_switch_session_update_tests.rs` |
| `reload.rs` の `#[cfg(test)] pub(crate) mod test_support`（新設 `reload_test_support.rs`・`boot_context`・`plant_skin_switch`） | `reload_tests.rs` の `pub(super)` | `reload_tests.rs`（集めた層の `reload_skin_tests.rs` も使う） |

- `reload_test_support.rs`（設計に無い・申し送り）: 集めた層に残す `reload_skin_tests.rs` が、本体へ移す `reload_tests.rs` の小道具 `boot_context`・`plant_skin_switch` を `super::tests::` で引いていた（TestPlacement の表の D8 ⑵ は本体へ移すファイルの足場が番人の crate に頼ることだけを見ていて、逆向きの使い手を数えていなかった）。2 つは番人の crate に頼らないので、D8 ⑵ の前者のとおり集めた層の兄弟 `reload_test_support.rs`（`#[test]` を持たない）へ写しを作らずに移し、両方から使う。`boot_context` の 1 つ目の引数は `&TempPath` から `&Path` に替え（中で使っていたのは `temp.path()` だけ）、呼び手 2 か所を `temp.path()` を渡す形にした（`test-support` には `temp-path-kit` を入れられないため）。4.1 で `lib.rs` に `ghost_session_scope_{rect,scale}_test_support` と同じ `#[cfg(any(test, feature = "test-support"))] pub mod` で宣言する対象に足す。
- `shell_balloon_switch_session_lap_tests.rs` の小道具（3.2 の申し送り）は、この段ではテストのモジュールのまま `pub(crate)` で開けた。4.2 の公開面で、集めた層の `#[cfg(test)]` のモジュールは本体から見えなくなるので、置き場（`test-support` の兄弟へ移すか）を決める。足場の `LapRig` は `GpuPermit`（runtime の `test-support`）と `SwitchRig` に頼り、番人の crate に頼るか（`TempPath`・`capture`）はそのとき確かめる。

#### 名前の末尾の比べ

- コマンド: `cargo test -p areka --bin areka -- --list`（移す前 `target/session-crate-split/3.4-list-before.txt`・移した後 `3.4-list-after.txt`）。`: test` の行の最後の `::` の後を多重集合にして比べた（`3.4-tails-compare.txt`）。
- 結果: 前 1844 行・後 1844 行。全体の末尾の多重集合は等しい（差 0）。道が変わったテストは 128 本で、前後の末尾の多重集合は等しい（例: `session::boot_config::option_tests::…` 17 本 → `session_tests::boot_config_option_tests::…` 17 本・`session::write_failure::sites_tests::…` 1 本 → `session_tests::sites_tests::…` 1 本）。

#### 確かめ

- 赤: 移して宣言を外しただけの `cargo check -p areka --bin areka --tests` が 186 件のエラー（上）。
- 緑: `cargo test -p areka --bin areka` 1841 passed・0 failed・3 ignored（3.3 の終わりと同じ）。`cargo check -p areka --bin areka --tests` と `cargo check -p areka --bin areka` が警告 0。`cargo fmt --all -- --check` 差分なし。`git grep "SwitchRig::new(" -- crates/areka/src/session_tests/` 0 件。

### 3.5

#### 本体へ分けたテストと小道具（名前と中身は変えない・`session_tests/` に宣言）

| 元（集めた層・`session/`） | 本体の新ファイル | 移したテスト | 一緒に移した小道具 | 元に残った数 → 分けた後の数 |
|---|---|---|---|---|
| `readonly_tests.rs`（`session::readonly::tests`） | `readonly_body_tests.rs` | `engage_is_called_only_from_main`・`main_calls_engage_exactly_once_before_resolve_boot`・`withhold_everything_is_called_only_from_readonly` | `all_sources`・`files_calling`（D8 ⑴・残るテストは使わない） | 8 → 5 + 3 |
| `readme_tests.rs`（`session::readme::readme_tests`） | `readme_body_tests.rs` | `only_the_os_port_spells_shell_execute` | なし | 17 → 16 + 1 |
| `session_end_deadline_tests.rs`（`session::session_end::deadline_tests`） | `session_end_deadline_body_tests.rs` | `only_session_end_calls_the_bounded_shutdown`・`deadline_call_scan_turns_red_on_one_line_removed_or_added` | `DEADLINE_CALL_TOKENS`・`ALLOWED_DEADLINE_CALLS`・`production_sources`・`deadline_call_sites`・`deadline_call_mismatch` | 10 → 8 + 2 |
| `log_history_convention_tests.rs`（`session::log_history::convention_tests`） | `log_history_convention_body_tests.rs` | `every_rule_target_names_a_module_or_literal_that_exists`・`locate_turns_red_for_missing_and_underivable_targets` | `crate_of`・`locate`（`RULES` は `crate::session::log_history::RULES` から引く） | 11 → 9 + 2 |

- 切り出しは行の範囲の移動だけ。移した側の頭に `//!` の 1 行と必要な `use` を足し、元の側は使われなくなった `use`（`session_end_deadline_tests.rs` の `BTreeSet`・`Path`）を外した。本体のファイルの中の根の並び・鍵（`areka/session/…`）は今のまま（4.4 で替える）。

#### 集めた層の兄弟へ移した小道具（写しなし・両側から使う）

- `session/ghost_session_scope_rect_test_support.rs`（新設・`#[test]` なし）: `LEFT_SCREEN`・`fake_handle`・`placement`（元は `ghost_session_scope_rect_tests.rs` の定数と `wiring` の中の 2 関数）。
- `session/ghost_session_scope_scale_test_support.rs`（新設・`#[test]` なし）: `ratio`・`applied`（`applied` が引く `ratio` も、`ratio` を他のテストが使うので一緒に移して両側が引く）。
- 親（`ghost_session_scope_{rect,scale}.rs`）は `#[cfg(test)] #[path = "…_test_support.rs"] pub(crate) mod test_support;` を足し、3.4 で `pub(crate)` に開けた `mod tests` を私有へ戻した。`ghost_session_scope_rect_tests.rs` の `wiring` も私有へ戻し、`LEFT_SCREEN` の `pub(crate)` は小道具の側に在る。本体の `*_boot_tests.rs` は `…::test_support::{LEFT_SCREEN, fake_handle, placement}`・`…::test_support::applied` を引く。ほかに `tests` を開けたままにする使い手は無い。
- 4.1 の `lib.rs` で `reload_test_support` と同じく `test-support` の条件で宣言する対象に、この 2 つの `test_support` を足す（tasks.md 4.1 に既に記載）。

#### 名前の末尾の比べ

- コマンド: `cargo test -p areka --bin areka -- --list`（前 `target/session-crate-split/3.5-list-before.txt`・後 `3.5-list-after.txt`）。`: test` の行の最後の `::` の後を多重集合にして比べた（`3.5-tails-before.txt`・`3.5-tails-after.txt`）。
- 結果: 前 1844 行・後 1844 行・末尾の多重集合は等しい（差 0）。分けた 4 ファイルの両側の合計は上の表のとおり前と同じ（8・17・10・11）。

#### 確かめ

- 赤: 分けただけでは壊れず（`use` は本体の側に持たせて組めた）、`cargo check` は元の側の未使用の `use` 2 件の警告だけが出て外した。RED は該当なし（振る舞いを変えない移動で、名前の末尾の比べが最初の網）。
- 緑: `cargo test -p areka --bin areka` 1841 passed・0 failed・3 ignored。`cargo check -p areka --bin areka --tests` 警告 0。`cargo fmt --all -- --check` 差分なし。

### 3.6

#### 追加したテスト（本体の `session_tests/body_systems_tests.rs`・`session_tests/mod.rs` に宣言）

- `register_systems_installs_body_systems`: `register_systems(…, crate::BODY_SYSTEMS)` の後に `BodySystems` の資源が在る。
- `after_input_registers_install_update_mcp_in_order`: 同じ登録の後、`Input` の段の系のうち `install::desk::drain`・`update::desk::drain`・`mcp::drain` を `system_type()` で引いて、この順に 1 つずつ並ぶ（系の名は bevy の `debug` の機能が無いと読めないので型で引く）。スレッドの局所の状態は使わない（`--test-threads 1` でも緑）。

#### 赤（一時の変異・戻して未コミット）

- `register_systems` の `world.insert_resource(body);` を消す → `register_systems_installs_body_systems` が赤（`BodySystems の資源が無い`）。
- `main.rs` の `register_body_after_input` の `update::register`・`mcp::register` を入れ替える → `after_input_registers_install_update_mcp_in_order` が赤。

#### 集めた層が本体の項目を名指す数

- コマンド: `grep -rnE "crate::(BODY_SYSTEMS|register_body_after_input|attach_body_receivers|wire_body_after_runtime|tell_desks_steady|body_cue_sinks|finish_after_run|install_boot_context|first_boot_origin|boot_first_ghost|after_run|main|cli|menu|install|update|mcp|shiori_host|runtime_tests)\b" crates/areka/src/session | wc -l`（`main.rs` の根の関数は上の名前で網羅）
- 結果: 0 件（`crate_source_test_support`・`sample_test_support` は 4.1 で解く）。

#### 確かめ

- `cargo test -p areka --bin areka` 1843 passed・0 failed・3 ignored（1841 + 2）。2 本は `-- --test-threads 1` でも緑。
- `cargo check -p areka --bin areka --tests` 警告 0。`cargo fmt --all -- --check` 差分なし。

## 段 3

### 4.1

#### 移したもの（`git mv`・行の単位の書き換えだけ）

- `crates/areka/src/session/` の 89 ファイル（`mod.rs` → `crates/areka-session/src/lib.rs`・`readme/` 11・`roots/` 7 は同じ並び）と本体の `crate_source_test_support.rs` を `crates/areka-session/src/` へ。`crates/areka/src/session/` は無くなった。
- 新しく足したのは `Cargo.toml` と `src/crash_report_tests.rs` の 2 つだけ。`git diff --cached -M --name-status` は A 2・R 106（R100 34・R099 18・R098 20・ほか R086〜R097）。一番低い類似度は `reload_test_support.rs`（R086・小さいファイルで `#[cfg]` の条件と `crate::session::` の行を替えた）。
- `git log --follow`（コミット前の木を `git commit-tree` で一時のコミットにして見た・枝は動かしていない）:
  - `crates/areka-session/src/ghost_switch.rs` → `3bea60ed8`（2.2）・`d4db9eb08`（session-module-gather #494）・`cc1c263fa`（#488）
  - `crates/areka-session/src/lib.rs` → `42aca0c28`（2.3）・`4bf30123b`（2.1）
  - `crates/areka-session/src/crate_source_test_support.rs` → `cc1c263fa`（#488）・`972d18abf`（#476）

#### 本番の依存（19 個＋optional 1 つ）

| 依存 | 理由（本番のコードが名指す所） |
|---|---|
| wintf | 窓・ECS の土台（11 ファイル） |
| dola | 説明書の台詞の受け口 `readme/readme_cue.rs` の `CueSink` |
| areka-ghost・areka-kanade・areka-sylphya・areka-parsers・areka-seriko・areka-actor | 起動と切替が名指す型（19・15・13・6・2・3 ファイル） |
| areka-user-data | 根の並び・切替の写し（4 ファイル） |
| areka-emo-present | 本番のコードは名指さない。`test-support` で立つ `ghost_session_scope_scale_test_support.rs` が `AppliedScale`・`ScaleRatio` を名指す。runtime が本番で既に引くので出荷の閉包は増えない（設計の 20 個のとおり残す） |
| areka-placement・areka-runtime・areka-input | 下の層（4・13・4 ファイル） |
| bevy_ecs・thiserror・tracing・tracing-subscriber・async-io | ワークスペースの表から（`thiserror` は `ghost_session.rs`・`async-io` は `smoke_exit.rs`・`tracing-subscriber` は `log_history.rs`） |
| windows | ワークスペースの表＋crate の側で `Win32_System_Registry` を上乗せ（`readme/os_port.rs` の `ShellExecuteExW`・`SHELLEXECUTEINFOW` がこの機能の下にある。本体ではほかの crate の機能の合わせで付いていた）。根の表は変えない |
| shiori-host32-host（optional） | `test-support` で付く（切替の足場の偽の SHIORI の戻り値の型） |

- 設計との違い: `areka-update` を外した。集めた層のコードが名指すのは `log_history.rs` の決まりの文字列（`"areka_update::winhttp"` ほか）だけで、型も関数も使わない（3.6「今の集めた層が本番で使う crate に収め」）。
- 番人の 3 crate（log-capture-kit・sample-ghost-kit・temp-path-kit）は本番の表に無い（開発用だけ）。`cargo tree -p areka-session -e normal` に `areka`・番人の 3 crate は出ない。
- `test-support = ["dep:shiori-host32-host", "areka-runtime/test-support"]`（設計のとおり）。

#### 開発用の依存

log-capture-kit・temp-path-kit・sample-ghost-kit（番人の注を写す）・shiori-host32-host・areka-emo-text・areka-emo-compose・areka-sakura（テストの World の型）・areka-runtime／areka-placement／areka-input（`test-support`）。設計の見込みの `areka-emo-atlas`・`image` は集めた層のテストが使わないので足していない。`cargo check -p areka-session --tests` が組めることで足りていることを確かめた。

#### `Cargo.lock`

差分は `areka-session` の 1 項目（33 行の追加）だけ。

#### `lib.rs` の宣言

- `pub mod body_systems;`・`pub mod smoke_exit;`（`pub(crate)` から）
- `#[cfg(any(test, feature = "test-support"))] pub mod ghost_switch_test_support;`（`#[cfg(test)] pub(crate)` から）
- `#[cfg(any(test, feature = "test-support"))] pub mod crate_source_test_support;`（新しい宣言）
- `#[cfg(test)] mod crash_report_tests;`（新しい宣言・前例 `areka-input` と同じ中身で `areka_runtime::crash_report_test_support::install` を `.CRT$XCU` に置き、`crash_filter_is_installed_before_the_tests_start` 1 本）
- ほかのモジュールの宣言は今のまま（`pub(crate) mod`・可視性は 4.2 の公開面で決める）。

#### 中の支えの宣言（本体のテストが使うもの）

| 支え | 置き場 | 条件 | 本体の使い手 |
|---|---|---|---|
| `reload::test_support`（`reload_test_support.rs`） | `reload.rs` | `#[cfg(any(test, feature = "test-support"))] pub mod`（`#[cfg(test)] pub(crate)` から） | `session_tests/reload_tests.rs` |
| `ghost_session_scope_rect::test_support` | `ghost_session_scope_rect.rs` | 同上 | `session_tests/ghost_session_scope_rect_boot_tests.rs` |
| `ghost_session_scope_scale::test_support` | `ghost_session_scope_scale.rs` | 同上。`use super::*` を `use areka_emo_present::{AppliedScale, ScaleRatio};` に替え、親の `#[cfg(test)]` の同じ `use` を外した（親のテストはこの `use` を使っていなかった） | `session_tests/ghost_session_scope_scale_boot_tests.rs` |
| `ghost_session::shell_balloon_switch_session_lap_tests` | `ghost_session.rs` | `#[cfg(test)]` のまま | `session_tests/shell_balloon_switch_session_update_tests.rs`（`lap_rig_on` ほか） |

- lap_tests を `test-support` に出せない理由: ファイルが `log_capture_kit`・`sample_ghost_kit` を使い、本体が使う `lap_rig_on` も `add_rpost_shell` で `SampleRoot::acquire("R_POST_and_KOMAINU")` を呼ぶ（設計 D8 ⑵ の後者・番人の crate は `test-support` に入れられない）。4.2 で本体の `shell_balloon_switch_session_update_tests.rs` の小道具の置き場を決める（申し送り）。

#### `test-support` で組むための書き換え

- `ghost_switch_test_support.rs`: `use sample_ghost_kit::SampleRoot;`・`RigSample::of`・`SwitchRig::new`・`use crate::body_systems::NO_BODY`（`new` だけが使う）・子のテストのモジュール 2 つ（`rig_wait_tests`・`talk_clock_tests`）に `#[cfg(test)]` を付けた。親が `#[cfg(test)]` だった間は要らなかった。
- `RigSample::of` が `#[cfg(test)]` になったので、本体の `sample_test_support.rs` の `switch_rig` は 4.2 で `RigSample { root, ghost, bundled_balloon, keep }` を欄で組む（Implementation Notes 3.1 の「`test-support` が sample-ghost-kit を引けないなら `#[cfg(test)]`」の枝）。
- `GhostSession::dispatcher` を `#[cfg(test)]` から `#[cfg(any(test, feature = "test-support"))]` へ（足場が使う・可視性 `pub(crate)` は 4.2）。

#### パスの書き換え

- `crate::session::` → `crate::`（68 ファイル）。`pub(in crate::session)` は 0 件。`"areka::session::…"` の文字列は 4.3 が持つので触っていない。
- `runtime_boot_tests.rs` の `use crate::sample_test_support::acquire_emo2;` を外し、`acquire_emo2()` の 3 か所（`runtime_boot_tests.rs`・子の `script_security_e2e_tests.rs`・`readme_script_e2e_tests.rs`。子は `use super::*` で引いていた）を `SampleRoot::acquire("emo2").expect("emo2 は登記済みの検体")` へ。設計は「1 行」と書くが、`use super::*` の子 2 本も同じ関数を呼んでいた。
- `crate_source_test_support.rs`: `crate_source!` に `#[macro_export]`・`pub(crate) use crate_source;` を外す・`crate_path` を `pub use`・`mod tests` に `#[cfg(test)]`。

#### 赤（移した直後）

- `cargo check -p areka-session --lib`: `readme\os_port.rs: error[E0432]: unresolved imports windows::Win32::UI::Shell::SHELLEXECUTEINFOW, ShellExecuteExW` → `windows` に `Win32_System_Registry` を上乗せして解いた。
- `cargo check -p areka-session --lib --features test-support`: 7 件（`sample_ghost_kit` の E0432・子のテストの `log_capture_kit` の E0432・`ScaleRatio`／`AppliedScale` の E0425／E0433／E0422・`GhostSession::dispatcher` の E0599）→ 上の書き換えで解いた。

#### 確かめ

- `cargo check -p areka-session --lib`・`--lib --features test-support`・`--tests`: エラー 0。
- 警告: `--lib` 439・`--lib --features test-support` 466・`--tests` 32。どれも死んだコード（`… is never used`／`never constructed`）で、`--tests` の 1 件だけ `shell_balloon_switch.rs` の `pub(crate) use areka_runtime::skin_switch::SwapFinish` の使われない `use`（設計の PublicSurface の 8 つの 1 つ・本体のテストだけが使う）。本体が名指す項目が `pub(crate)` のまま lib になったためで、公開面（4.2）を決めるまで 0 にならない。
- 門: 4.1 の門は「エラー 0」とし、警告 0 の門は 4.2（本体のコンパイラの指摘で公開面を決める段）へ移した（tasks.md の Implementation Notes に記録）。
- `cargo fmt -p areka-session -- --check` 差分なし（`crate::session::` を外して並びが変わった `use` を `cargo fmt -p areka-session` で整えた）。
- テストは走らせていない（4.4）。本体の crate はこの時点で組めない（4.2 でつなぎ替える）。

### 4.2

#### つなぎ替え

- 本体の `Cargo.toml`: 本番の表に `areka-session = { path = "../areka-session" }`、開発用の表に `areka-session = { path = "../areka-session", features = ["test-support"] }`（`areka-input` の 2 行の形と理由の注）。
- `main.rs` から `mod session;`・`#[cfg(test)] mod crate_source_test_support;` を外した（`mod smoke_exit;` は 2.3 で既に無い）。出し直しは置いていない。
- 本体の 153 ファイル（`src/` 152・`tests/smoke_boot_loop_exit.rs` の説明の 1 行）: `crate::session::` → `areka_session::`、`session::`（前が識別子でも `:` でもないもの）→ `areka_session::`、`crate::crate_source_test_support::crate_source!` → `areka_session::crate_source!`、`use crate::crate_source_test_support::crate_source;` → `use areka_session::crate_source;`、`crate::crate_source_test_support::crate_path` → `areka_session::crate_source_test_support::crate_path`。`"areka::session::…"` の文字列（4.3）と `tests/emo2_mcp_real_run.rs`（5.1）は触っていない。
- `runtime_tests/zorder_wiring_tests.rs`: 33・74 行の `include_str!("../session/…")` を `crate_source!("areka-session", "src/runtime_boot.rs")`・`crate_source!("areka-session", "src/ghost_session.rs")` へ（ほかは `use` の 1 行だけ）。
- 本体の `placement_tests/` の 4 ファイル（`measure_bake_count_tests.rs`・`measure_template_tests.rs`・`zorder_group_branch_coverage_tests.rs`・`zorder_inversion_branch_coverage_tests.rs`）は、読み口の `use`／パスの 1 行ずつだけが変わった（`mod crate_source_test_support;` を外すと組めないため。同じウェーブの約束の範囲の外の変更・申し送り）。
- `sample_test_support.rs` の `switch_rig`: `RigSample::of`（集めた層の `#[cfg(test)]`）の代わりに `RigSample { root, ghost, bundled_balloon, keep }` を欄で組む（`RigSample` の 4 欄を `pub` に）。

#### 公開面の決め方

1. 本番の本体（`cargo check -p areka --bin areka`）のエラーを 1 件ずつ見て `pub` にした（E0603・E0616・E0451・`private_interfaces`。繰り返しで 0 まで）。
2. 続けて本体のテストのビルド（`--tests`）のエラーを同じく見た。glob（`use areka_session::<mod>::*;`）の 9 ファイルは私有の名を黙って落とすので、`cannot find …` を glob の先のモジュールの定義へ引き当てて開けた。
3. 集めた層の `--tests` の `private_interfaces` で、開けた項目の署名が名指す型を開けた（連れて開けた型）。
- 一覧（ファイルごと・項目の名・本番の本体が使うか）: `target/session-crate-split/4.2-surface.txt`（P＝本番の本体も使う・T＝本体のテストだけ〔と連れて開けた型〕）。数: 本番の本体が使う 25 ファイル・本体のテストだけ 29 ファイル（T の `shell_balloon_switch_session_lap_tests.rs` の行は `use` の書き換えで、開けた項目ではない）。
- 使われない `pub(crate)` は消していない（`cargo check -p areka-session --lib` が警告 0 で、残る `pub(crate)` は集めた層の中で使われている）。

#### 本体のテストだけが使う `#[cfg(test)]` の項目（`#[cfg(any(test, feature = "test-support"))] pub`）

| 項目 | 前 | 使う本体のテスト |
|---|---|---|
| `GhostSession::{for_test, with_memory_publisher, with_seriko_sink}` | `#[cfg(test)] pub(crate)` | `session_tests/`・`runtime_tests/`・`install`・`update`・`mcp` のテスト |
| `GhostSession::dispatcher` | `#[cfg(any(test, feature = "test-support"))] pub(crate)`（4.1） | 同上 |
| `GhostSession` の欄 `memory_publisher_for_test` とその読み・組み立て 4 か所 | `#[cfg(test)]`（欄は私有のまま） | `with_memory_publisher` が立てる |
| `roots::single_env` | `#[cfg(test)] pub(crate)` | 本体のテスト 23 か所 |
| `shell_balloon_switch::SwapFinish` の出し直し | `#[cfg(test)] pub(crate) use` | `runtime_tests/frame/switch_finish_tests.rs` |
| `log_history::hold` | `log_history_capture_tests.rs` の `pub(crate) fn hold`＋`static WINDOW` | `offline_dump/offline_dump_tests.rs`（`use` を `areka_session::log_history::hold` へ） |
| `ghost_switch_test_support`（モジュールは 4.1） | 中の項目が `pub(crate)` | 本体の 17 ファイル＋`session_tests/`（中の項目を `pub` に） |
| `ReadmeWiring::path`（設計の 8 つの外） | `#[cfg(test)] pub(crate)` | `session_tests/ghost_session_restart_tests.rs`・`ghost_session_switch_tests.rs` |
| `ghost_session::shell_balloon_switch_session_lap_test_support`（新設・下） | ― | `session_tests/shell_balloon_switch_session_update_tests.rs` |

- `hold` の `static WINDOW` は `log_history.rs` の写しの窓そのものの `static WINDOW`（`Mutex<Window>`）と名がぶつかるので、`hold` の関数の中に置いた（名と中身は前のまま）。テストのモジュール `capture_tests` は `#[cfg(test)]` のまま。使われなくなった `use std::sync::Mutex;` を外した。
- `ghost_switch_test_support` の中で集めた層のテストだけが使う `Stall::{at, entered, release}`・`SwitchRig::{add_balloon_copy, plant_last_balloon}` は、`--lib --features test-support` で死んだコードの警告になるので `pub` にした（モジュールが `test-support` の条件の下にある）。

#### 本番の項目のうち本体のテストだけが使うもの（素の `pub`）

- 3.4 で開けた私有の項目（`GhostSession::seriko`・`GhostSession::loop_ticker`・`PreparedWindows::spawn` の欄、`reload::drain`、`write_failure::drain_system`）は、集めた層の本番のコードも使う欄と関数で、可視性だけを機能で切り替えると項目を 2 つ書くことになる。`#[cfg(any(test, feature = "test-support"))] pub` にできないので素の `pub` にした（設計の「本番の公開面に中の関数を出さない」との違い・申し送り）。
- `boot_seam::insert_persist_wiring` は `#[cfg(test)]` ではない本番の関数で、本体の `session_tests/readonly_run_tests.rs`・`readonly_sites_tests.rs`・`write_failure_sites_tests.rs` が使う（設計の「本体が使わない（0 件）」は 3.4 で 3 本が本体へ移る前の数）。素の `pub`。
- ほかは一覧の T の行（`boot_resolve::{read_session_mark, clear_session_mark, keep_session_mark, …}`・`reload::{ReloadInFlight, ReloadStage, register, …}` ほか）。

#### 連れて開けた型（`private_interfaces`）

- 本番の本体の段: `boot_resolve::BalloonRoute`・`exit_wait::ClosingWaits`・`ghost_session::OpenWindowsError`・`ghost_switch::{SwitchTarget, PrevGhost, SwitchStage, WelcomeAttempt}`・`switch_copy::SwitchCopy`・`log_history::{Stamp, Hms}`・`reload::RefusedReason`・`roots::RootSkip`・`session_mark::MarkVerdict`。
- 本体のテストの段: `ghost_session_runtime::{RuntimeRebuild, rebuild_runtime_in_place}`（`ghost_session.rs` の `pub(crate) use` を `pub use` にした E0364・E0365）・`reload::SkinStep`・`log_history::Rule`・`readme::destination::{Destination, Target, Store}`・`boot_resolve::NoBalloon`・`app_exit::WindowsClosed`・`ghost_session::BootWiringFailed`・`ghost_switch_test_support::Stall`。
- 出し直しの広げ: `shell_balloon_switch.rs` の `areka_runtime::skin_switch::{SkinKind, …}`・`ghost_switch.rs` の `areka_runtime::special_name::{LastInstalledGhost, …}` を `pub(crate) use` → `pub use`。

#### lap_tests の小道具の置き場

- 本体の `shell_balloon_switch_session_update_tests.rs` が使う `lap_rig_on`・`SECOND`・`UNBOUNDED`・`Ticks`・`LapRig`・`LapRig::frames_until`・`copy_tree`・`kinds`・`got`・`idle` は、集めた層の `shell_balloon_switch_session_lap_tests.rs` の中と兄弟の `_abort_tests.rs`・`_balloon_tests.rs` も使う（両側の使い手）。番人の crate に頼るのは `add_rpost_shell`（`SampleRoot::acquire("R_POST_and_KOMAINU")`）だけで、残りは頼らない。
- 決め（D8 ⑵ の前者）: 番人に頼らない小道具を、行の範囲の移動で兄弟の `shell_balloon_switch_session_lap_test_support.rs`（`#[cfg(any(test, feature = "test-support"))] pub mod`・`ghost_session.rs` で宣言・`#[test]` を持たない）へ移した。移したのは上の 10 個と、それが引く `NO_TICKS`・`CREEP`・`ACCEPT`（`Ticks` の私有の欄を組むので一緒に）・`WORK_AREA`・`TICK_EVERY`・`UiMarks`・`UiSeen`（`ui_seen_counts_only_ui_side_progress` が使うので `pub(crate)`）・`spawn_windows`・`PUMP_MAX`・`pump_messages`・`refs_of`・`calls_a`。lap_tests は `pub(crate) use super::shell_balloon_switch_session_lap_test_support::*;` で引き、兄弟 2 本の `use super::shell_balloon_switch_session_lap_tests::{…}` は変えていない。テストの名前と中身は変えていない。
- 番人に頼る `add_rpost_shell` は分けられないので、`lap_rig_on` は 2 つ目のシェルを足す関数を 3 つ目の引数 `add_shell: fn(&Path, &str)` で受ける形にした。集めた層の `lap_rig_of` は lap_tests の `add_rpost_shell` を、本体の `steady_rig` は本体のテストのファイルに置いた `add_rpost_shell`（同じ 9 行・理由の注つき）を渡す。この 9 行は両側の写し（`test-support` に `sample-ghost-kit` を入れられないため・申し送り）。ほかの写しは無い。

#### 赤

- `mod session;`・`mod crate_source_test_support;` を外しただけ: `cargo check -p areka --bin areka` が 106 件のエラー（E0433 100・E0432 6）。
- パスを書き換えた直後: 同じく 122 件（`target/session-crate-split/4.2-red.txt`。E0603 の私有のモジュール 117 ほか）。公開面を開けて 0 へ。

#### 確かめ

- `cargo check --workspace --all-targets`: 警告 0・エラー 0。
- `cargo check -p areka-session --lib`・`--lib --features test-support`・`--tests`: どれも警告 0（4.1 の 439・466・32 から）。
- `cargo test -p areka --bin areka --no-run`・`cargo test -p areka-session --no-run`: 組めた（テストは 4.4 で走らせる）。
- `cargo doc -p areka-session --no-deps`: 解けないリンクの警告に本体の名（`menu`・`install`・`update`・`mcp`・`cli`・`BODY_SYSTEMS`・根の関数・`areka::`）は 0 件。残る解けないリンク（`quit_app`・`fatal`・`request_ghost_switch` ほか集めた層の中の名）は集めた層の項目を指すもので、4.6 の対象の外。
- 4.7 の式: `crates/areka-session/Cargo.toml` に `areka =` の行 0・`cargo tree -p areka-session -e normal,dev` に `areka v` 0・`crates/areka-session/src` の `//` の行を除き文字列の中を除いた `areka::` 0（文字列の中に残る `"areka::install"`・`"areka::session::…"` ほか 87 行はログの出所の語彙で、`"areka::session::…"` は 4.3 が替える）。
- `Cargo.lock`（`main` との差）: `areka-session` の 1 項目と本体の依存の 1 行だけ（33 行の追加）。
- `cargo fmt --all -- --check`: 差分なし。
- `cargo clippy -p areka-session --all-targets`: 警告は移したコードに前からある形の指摘（`type_complexity` 33・`collapsible_if` 19 ほか）で、新しく足した `shell_balloon_switch_session_lap_test_support.rs`・`log_history::hold` には無い。

### 4.3

RED は n/a（文字列の target の書き換えのみ。期待値の側も同時に直した）。

- `log_history.rs` の規則 5 行（boot_config, boot_resolve, ghost_session, ghost_switch, readme）を `areka_session::…` へ。種別・照合は据え置き。
- 期待値: `log_history_tests.rs`, `log_history_convention_tests.rs`, `reload_skin_tests.rs`（crate）、`session_tests/reload_tests.rs`（本体）。
- `doc/ssp-mcp/log-convention.md` の規則表 5 行。
- `git grep -n '"areka::session' -- crates/` の残りは 5.1 の持ち物の 3 件のみ（`emo2_mcp_real_run.rs`, `real-run-kit/src/launch.rs`, `real-run-kit/src/bin/real-run-fake-areka.rs`）。real-run-kit と `emo2_mcp_real_run.rs` は引用符つき・なしとも 5.1 に任せる。`cargo check -p areka-session --tests` / `cargo check -p areka --bin areka --tests` は警告 0、fmt 差分なし。

### 4.4

#### 赤（直す前）

- `cargo test -p areka-session --lib`: テストの exe が起動の時点で `0xc0000139`（STATUS_ENTRYPOINT_NOT_FOUND）で落ち、1 本も走らない。
- `cargo test -p areka --bin areka`: 1327 緑・10 赤。
  - 見張り 6 本: `readme_body_tests::only_the_os_port_spells_shell_execute`（当たり 0）・`session_end_deadline_body_tests` の 2 本（許可表の 3 行が当たり無し）・`readonly_body_tests::engage_is_called_only_from_main`（較正の `mod session;` が `main.rs` に無い）・`readonly_body_tests::withhold_everything_is_called_only_from_readonly`（`areka/session/readonly.rs` が無い）・`log_history_convention_body_tests::every_rule_target_names_a_module_or_literal_that_exists`（`areka_session` の 5 行のフォルダを導けない）。
  - 字面の対照 1 本: `zorder_wiring_tests::t_zwi08_the_entry_point_carries_the_shell_setting_into_the_wiring`（4.2 で `open_ghost_windows` を `pub` にしたので、対照の字面 `pub(crate) fn open_ghost_windows(` が本文に無い）。
  - 口を据えずに口の道を通り `body_systems_missing` の記録が 1 件増えた 3 本（D13）: 下の表。

#### テストの exe の申告（設計との違い）

- 原因: 告知の `alert.rs`（とその DPI のテスト）が `TaskDialogIndirect`・`TaskDialog` を取り込む。`dumpbin /imports` で areka-session の lib のテストの exe の `comctl32.dll` の取り込みはこの 2 つだけで、どちらも comctl32 の版 6 にしか無い。版 6 の申告を持つのは本体の exe（`crates/areka/build.rs` の `rustc-link-arg-bins`・bin のユニットテストの exe にも届く）と wintf のサンプル（`rustc-link-arg-examples`）だけで、crate を分けた後の areka-session のテストの exe には申告が無い。
- 直し: `crates/areka-session/build.rs` と `crates/areka-session/tests.manifest`（中身は `areka.manifest` と同じ申告）を足し、MSVC のときだけ `cargo:rustc-link-arg=/MANIFEST:EMBED`・`/MANIFESTINPUT:<tests.manifest>` を出す。`rustc-link-arg-tests` は結合テスト（`tests/`）の無い crate では Cargo が「does not have a test target」で受け付けなかった。`rustc-link-arg` はこの package のリンクする成果物（ここでは lib のテストの exe だけ）にしか届かず、依存する側には伝わらない。
- 確かめ:
  - `cargo test -p areka-session --lib -- --list`: 510 本が並ぶ（直す前は 0xc0000139）。
  - `EnumResourceNamesW(RT_MANIFEST)`: `target/debug/areka.exe`・areka-session の lib のテストの exe・本体の bin のテストの exe がどれも申告 1 つ（id 1）。`mt.exe -inputresource:…;#1` で取り出した areka.exe と areka-session のテストの exe の申告にどちらも `Microsoft.Windows.Common-Controls` が在る。
  - `cargo build -p areka --bin areka -v` の areka の rustc の行の `/MANIFESTINPUT:` は `crates/areka/areka.manifest` の 1 つだけ（areka-session の `tests.manifest` は届かない）。
- 設計の中身は変えず、設計の「新しい crate」のファイルの木に 2 行を足した。

#### 見張りの付け替え（SourceGuards）

- `readme_body_tests.rs`・`session_end_deadline_body_tests.rs`: 根の並びに `("areka-session", crate_path("areka-session", "src"))` を足した（3 → 4）。鍵 `"areka/session/…"` → `"areka-session/…"`（`readme/os_port.rs`・`ghost_session.rs`・`session_end.rs`）。許可表と期待値は鍵の字面のほか変えていない。説明の 3 つの木の列挙に `crates/areka-session/src` を足した。
- `log_history_convention_body_tests.rs`: `crate_of` に `"areka_session"` → `(crate_path("areka-session", "src"), "lib.rs")` の行を足した。
- `readonly_body_tests.rs`: `engage_is_called_only_from_main` は本体の `src` と `crate_path("areka-session", "src")`（相対パスに `areka-session/` を前置）の 2 つを歩く。名前だけで呼べる所の絞りを `areka-session/readonly.rs`・`areka-session/readonly_*` へ。較正の字面は、`main.rs` から消えた `mod session;` の代わりに `use areka_session::`。`withhold_everything_is_called_only_from_readonly` の除き方を `areka-session` の `readonly.rs` へ。`main.rs` を読む 1 本は変えていない。
- `zorder_wiring_tests.rs` の `t_zwi08`: 対照の字面 `pub(crate) fn open_ghost_windows(` → `pub fn open_ghost_windows(`（4.2 の公開面に合わせる）。
- 見張りの新しい当たりは無かった（許可表は変えていない）。

#### 口が無いときの記録（OrderTests 6）

- 本体の `session_tests/body_systems_tests.rs` に `missing_body_systems_logs_and_answers_like_no_desk` を足した。口の無い World で `BodySystems::of` を呼び、`body_systems_missing` の行が `ERROR` 1 つ・`update_busy`・`install_busy` が偽・`cue_sinks` が 2 つ。本体のテストのビルドでは areka-session は `cfg(test)` でないので記録が出ることを、このテストの緑で確かめた。
- 本体のテストから `of` を呼ぶため、`BodySystems::of` を `#[cfg(any(test, feature = "test-support"))] pub` にした（それ以外では今のまま `pub(crate)`・中身は私有の `installed_or_none` 1 つ）。4.5 の「本体のテストだけが使う項目」の一覧に足す（`audit-areka-app` への申し送り）。

#### `world.insert_resource(crate::BODY_SYSTEMS);` の 1 行を足したテスト（D13・表明は変えていない）

| テスト | 赤の形 |
|---|---|
| `session_tests::reload_tests::missing_ghost_or_kanade_is_no_context` | warn 以上の総数 2（期待 1） |
| `session_tests::shell_balloon_switch_tests::acceptance_starts_one_wait_marked_only_for_menu_and_raise_event_shells` | warn＋error の数 1（期待 0）・5 件とも |
| `session_tests::shell_balloon_switch_tests::send_failure_leaves_one_error_and_no_marker` | error の総数 2（期待 1） |

- 前からあった 1 行（`reload_tests.rs` の `refusals_in_order_leave_one_record_each`・`shell_balloon_switch_tests.rs` の `refusals_in_order_leave_one_warning_each_and_send_nothing`）と合わせ、本体で 1 行を持つテストは 5 本。

#### 確かめ

- `cargo test -p areka-session`: 507 緑・3 無視・0 赤。
- `cargo test -p areka --bin areka`: 赤の 10 本と足した 1 本を含む 15 本を回し直して 15 緑（赤のテストだけ回し直す）。
- `cargo test -p log-capture-kit --test file_length_guard_test`: 7 緑（例外表は変えていない）。
- `cargo check --workspace --all-targets`: 警告 0。`cargo fmt --all -- --check`: 差分なし。
- `cargo clippy -p areka-session --all-targets`・`-p areka --bin areka --tests`: 触った所（`body_systems.rs`・`build.rs`・足したテスト）に指摘なし（残る指摘は移したコードに前からある形）。

### 5.2

- 変更: `tools/load-flake.ps1` の `-Binary` に `areka-session-lib`（`-p areka-session --lib`）を足した。足した場所は ValidateSet・`Get-CargoTestTarget` の switch・自分の検査の表 `$want`・説明の列挙（コメントと `.DESCRIPTION` 側の 2 か所）。`areka-input-lib` と同じ書き方。
- RED: 該当なし（mechanical。値の足しだけで、新しい分岐の判断は無い）。
- 他の .ps1 がこの道具の出力の文を字面で読んでいないか: 足した文字列は `areka-input-lib` と同じ並びで、出力する文は変えていない。
- `pwsh -NoProfile -File tools/load-flake.ps1 -SelfTest`: 5 本とも ok（`cargo args` を含む）。-SelfTest は cargo も負荷の走行も起こさない（約 1 秒の偽の親と負荷の子 1 本のみ）。

### 5.1

- 変更: 実走の道具と実機テストの log target を `areka::session::…` から `areka_session::…` へ替えた。対象は `crates/real-run-kit/src/launch.rs`（`RELIED_TARGETS`）・`launch_tests.rs`・`close_tests.rs`・`bin/real-run-fake-areka.rs` と `crates/areka/tests/emo2_mcp_real_run.rs`（`rust_log`・`BOOT_CONFIG_PREFIX`・コメント）。`launch_tests.rs` の「似て非なる」出所は `areka_session::app` と `areka_session::app_exits` で、前方一致の境界は同じ。runbook の文書は触っていない。
- RED: 該当なし（文字列の付け替えのみ。既存テストは付け替えと同時に新しい名前で緑）。
- `git grep -n "areka::session" -- crates/real-run-kit crates/areka/tests/emo2_mcp_real_run.rs`: 0 件。
- `cargo test -p real-run-kit`: 全緑。`cargo check -p areka --tests`: 警告 0。`cargo fmt --all -- --check`: 差分なし。

### 6.2

- 変更: 古い `crates/areka/src/session/<名>.rs` を移した先へ、出所 `areka::session…` を `areka_session…` へ書き換えた。移した先は `git ls-files` で実在を確かめて決めた（`crates/areka-session/src/<名>.rs`、本体に残ったテストは `crates/areka/src/session_tests/<名>.rs`）。実在しない名は 0 件。`ukadoc: https://…` の行は触っていない。
- 触ったファイル: `doc/COMPAT_ARCHITECTURE.md`（パスの行のみ。§8 の名の行は触らない）・`doc/ukadoc-coverage/ledger/{sakura-script,property,shiori,assets}.toml`・`doc/ukadoc-coverage/briefing-assets.md`・`doc/testing.md`・`doc/ssp-mcp/log-convention.md`（表の外）・`doc/pasta-real-run.md`（127・141 行の `RUST_LOG`）・`doc/ideas/novel-redesign-2026-11-20/` の 8 ファイル・`tools/package.ps1`（3 か所）・`tools/perf/judge-perf.py`（1 か所）。`crate::session::…` の 3 か所（COMPAT の `pick_index`・log-convention の `last_id`・ideas 03 の `session/runtime_boot.rs`）も新しい場所へ替えた。
- 台帳の変えた行数: 38 行（sakura-script 18・property 10・shiori 9・assets 1。行数は別々の行の置き換えで増減なし）。
- 網羅の報告: `ukadoc-survey report` と `report-summary` で作り直した（手では直していない）。
- RED: 該当なし（mechanical。`check` は書き換えの後に回し、書き換え前の赤は測っていない）。
- `cargo run -p ukadoc-survey -- check`: 食い違い 0 件（緑）。
- 道具の出力の文: `tools/package.ps1`・`judge-perf.py` は注釈・説明の中のパスだけで、出力する文字列は変えていない。
- `git grep -E "crates/areka/src/session/|areka::session|crate::session"`（`doc`・`tools`、除外は `roadmap*`・overview html・`report/`）: 0 件。
- レビューの差し戻しの直し:
  - `doc/COMPAT_ARCHITECTURE.md` の OS のセッションの終了の行: `only_session_end_calls_the_bounded_shutdown`・`deadline_call_scan_turns_red_on_one_line_removed_or_added` は本体の `crates/areka/src/session_tests/session_end_deadline_body_tests.rs` へ移ったので、その 2 つの前にこのファイルの名を入れて分けた（残りの 8 つは `crates/areka-session/src/session_end_deadline_tests.rs` のまま）。§8 の名の行は触っていない。
  - `doc/ideas/novel-redesign-2026-11-20/92-validation-code.md:173`: `crates/` の付かない `areka/src/session/runtime_boot.rs` を `areka-session/src/runtime_boot.rs` へ（この文書の書き方に合わせて `crates/` は付けない）。
  - 本体の `src/` 直下から移した `smoke_exit.rs`（D5）の古いパス `crates/areka/src/smoke_exit.rs` を `crates/areka-session/src/smoke_exit.rs` へ: `tools/package.ps1`（148・152・694 行の注釈）・`tools/perf/invoke-perf-run.ps1`（29 行の説明）。どちらも注釈・説明の中で、出力する文字列や照合の字面は変えていない。
- 広げた形の掃き出し（`doc`・`tools`、除外は `roadmap*`・`doc/roadmap-overview-2026-10-16.html`・`doc/ukadoc-coverage/report/`）:
  - `git grep -nE "areka/src/session/|src/session/|areka::session::|crate::session::|\bsession::[a-z_]"`: 0 件。
  - `git grep -nE "session/[a-z_0-9]+(\.rs|/)|session_tests/"` から `crates/areka/src/session_tests/` と `areka-session/` を除いたもの: 0 件。
  - `crates/areka-session/src/` の各ファイルで、本体に同名が無いものについて `areka/src/<名>` と `crates/areka/src/<名>` を `git grep -F`: 直した後 0 件（直す前は `smoke_exit.rs` の 4 件）。
  - `git grep -nE "smoke_exit|crate_source_test_support|sample_test_support|areka::session"` で古い場所を指すもの: 0 件（残るのは `invoke-perf-run.ps1` の変数名 3 件で、パスではない）。
  - 文書が「`<テストのファイル>` の `<名>`・…」の形で挙げるテストの名を、そのファイルに実在するかで全部照らした（使い捨ての python。`*_body_tests.rs` へ移った名を含む）: 集めた層と `session_tests/` の名の食い違いは上の 1 行の 2 件だけで、直した後 0 件。集めた層の名で残る 2 件（`app_exit_tests.rs` の `fault_of_returns_the_fault_only_for_kanade_stopped_fault`・`ghost_switch_tests.rs` の `matching_is_case_sensitive`）は `main` の `crates/` にも無く、移動の前からの食い違いなので触っていない。
  - 台帳は変えていないので報告は作り直していない。`cargo run -p ukadoc-survey -- check`: 食い違い 0 件（緑・直しの後に回し直し）。

### 6.3

- `.kiro/steering/structure.md`: 「Session Crate（areka-session）」の節を `areka-input` の節の後（`Parser Crate` の前）に足した（役・出所の名前・`BodySystems`・`test-support`・テストの置き場・依存の向き・使い手）。本体の節の `session/` は「集めた層は `areka-session`・本体の本物どうしをつなぐテストは `session_tests/`」へ替え、`session/<名>.rs`・`crates/areka/src/session/…` の言及を `areka-session` の相当へ付け替えた。出所の名前 2 か所（`areka::session::roots::user_root`・`…::layout()`）は `areka_session::…`。`areka-session` を、本体の依存の一覧・配置と実行系と入力の使い手・`log-capture-kit`（22 → 23 crate）・`temp-path-kit`・クラッシュ報告の支えの使い手へ数え入れた。
- `.kiro/steering/tech.md`: 集めた層のパス 1 か所（`readme/os_port.rs`）を `crates/areka-session/src/` へ替えた。
- `audit-areka-app`・`audit-areka-install-update` の brief: 「受け持つファイル」の 1 行だけを、`crates/areka-session/` の該当ファイル・本体の `session_tests/`・`main.rs` の `BODY_SYSTEMS` へ書き換えた。所見の文の古いパスは触っていない。
- レビューの差し戻しで、パスを替えた文の周りが事実と合わなくなった所を直した:
  - `structure.md` の入力の節の Tests: 強制退避の枝のテストは「集めた層を引くテストだけは `areka-session` に置く」へ（このテストは `areka_input` の支えを引き、`areka-session` の `app_exit.rs` の子）。
  - `structure.md` の実行系の節の約束と本体の `runtime_tests/` の行: `session_hooks` の向き先は「上の層」、据えるのは「`areka-session` が `hooks.rs` で」（`crates/areka-session/src/hooks.rs` の `install`）。
  - `structure.md` の Session の節の Purpose と本体の `session_tests/` の行: 「本体に頼る」を外した（`areka-session` は本体に依存しない）。
  - `tech.md`: `ShellExecute` を綴るのは `crates/areka/src`・`crates/areka-session/src` で `readme/os_port.rs` だけ（`git grep -l ShellExecute -- crates/areka/src crates/areka-session/src` が `crates/areka-session/src/readme/os_port.rs` の 1 件）。
  - `audit-areka-app` の brief: 受け持つファイルを本体（`crates/areka/`）と集めた層（`crates/areka-session/`）の 2 組に分け、重なりを無くした。集めた層は `src/readme*`・`src/readme/`・`src/roots/`（`audit-areka-install-update` の持ち物）を除く全部で、`src/lib.rs`・`Cargo.toml`・`build.rs`・`tests.manifest` を含む。本体の `session_tests/` は `readme_body_tests.rs` を除く。`git ls-files crates/areka-session crates/areka/src/session_tests` の全ファイルがどちらか 1 本の brief に入ることを見た。
  - `audit-areka-install-update` の brief: 本体の `crates/areka/src/` の分と `crates/areka-session/src/` の分を 2 つに分けて書いた。
  - `structure.md` の実行系の節の cue の受け口: 「本体に頼る受け口」を「上の層に頼る受け口」へ替え、`readme/readme_cue.rs` は `areka-session` の側（`crates/areka-session/src/readme/readme_cue.rs`）、`install_cue.rs`・`update_cue.rs` は本体の側と書き分けた。
  - 直した文の周り（Session の節の依存・Features・使い手、`log-capture-kit` の 23 crate、`areka-user-data` の依存の向き）を `Cargo.toml` と `git grep` で突き合わせ、合っていることを見た。
- 2 回目の差し戻しで直した所:
  - `audit-areka-app` の brief: 集めた層から `src/ghost_switch*`・`src/shell_balloon_switch*`（`audit-emo2-boot-switch` の持ち物）を除き、本体の `session_tests/` からも `shell_balloon_switch*` を除いた。
  - `audit-emo2-boot-switch` の brief: 「受け持つファイル」の行の `crates/areka/src/session/` を `crates/areka-session/src/` の `ghost_switch*`・`shell_balloon_switch*` と本体の `crates/areka/src/session_tests/` の `shell_balloon_switch*` へ替えた。所見の文・申し送りの節の古いパスは触っていない。
  - `crates/areka-session/src/readme_script_e2e_tests.rs` は、移す前のどの brief の「受け持つファイル」にも名が無く（`audit-areka-install-update` が挙げていたのは `readme.rs`・`readme/`・`readme_tests.rs` だけ）、本体の残りを受け持つ `audit-areka-app` の持ち物だった。`runtime_boot_tests.rs` の子でもあるので `audit-areka-app` に置き、`audit-areka-install-update` の行から外した。これに合わせ、`audit-areka-app` の除く所は `src/readme*` でなく `src/readme.rs`・`src/readme/`・`src/readme_tests.rs`・`src/roots/` と書いた。
  - 3 本の brief の突き合わせ: `git ls-files crates/areka-session crates/areka/src/session_tests` の 141 ファイルを 3 本の brief の書き方で振り分け、どれも 1 本だけに入ることを見た（`audit-areka-app` 100・`audit-areka-install-update` 21・`audit-emo2-boot-switch` 20・どこにも入らない／2 本に入るもの 0）。`audit-areka-app` の集めた層の分は、括弧の中の名の並びだけで 100 件を覆う。
  - steering の残り: `logging.md` の `log_history.rs`・`product.md` の `boot_resolve.rs`（定数 `DEFAULT_BALLOON_FOLDER`）を `crates/areka-session/src/` へ替えた（どちらも移した先に在ることを見た）。`git grep -E "crates/areka/src/session/|areka::session|session/<名>.rs" -- .kiro/steering/` は `roadmap.md`・`roadmap-history.md`（除外の 2 本）を除いて 0 件。
  - `structure.md` の Session の節の `BodySystems`: 据える所を「`register_systems` が系を登録する前に 1 回据える」へ（`ghost_session.rs` の `register_systems` は `hooks::install`・`install_input_hooks` の後、系を足す前に `insert_resource(body)` する）。
- RED: 該当なし（文書のみ）。`git grep -E "crates/areka/src/session/|areka::session|session/<名>.rs" -- structure.md tech.md`: 0 件。

### 6.1

コードの説明（コメント）だけを移した先のパスへ書き換えた。コードは変えていない（`areka-runtime`・`areka-kanade` の差分はコメントの行だけ）。

書き換えた所:
- `areka-runtime`: `talk_clock.rs`・`user_break_cue.rs`・`lib.rs`（`runtime_boot.rs` の 3 か所と `areka_session::ghost_quit::wire_kanade_stop` の 2 か所）・`strict_judges.rs`・`placement_tests/transition_judge_offset_signoff_tests.rs`・`session_hooks.rs`（`areka_session::hooks`）・`skin_switch.rs`（`areka_session::shell_balloon_switch`）
- `areka-kanade`: `schedule/script_log.rs`・`schedule/events.rs`（`areka_session::reload`）
- 本体: `tests/emo2_real_run.rs`・`Cargo.toml`（81 行付近）・`runtime_tests/spine_move_cue_tests.rs`・`session_tests/boot_config_copy_tests.rs`
- `areka-session` の中: `alert_dpi_tests.rs`・`boot_config.rs`（`session::runtime_boot` の字面）

除外した所（書き換えていない）:
- 4.3 の持ち分: `log_history*`・`reload*_tests.rs`／5.1 の持ち分: `tests/emo2_mcp_real_run.rs`
- `areka-placement` の 4 か所（古い字面のまま）: `diag.rs:46`・`persist.rs:330`・`resolver.rs:97`・`resolver_resolve_balloon_tests.rs:123`（いずれも `session/boot_seam.rs`）
- `crates/pilot/examples/pilot-windows-rs-next/REWRITE_TABLE.md:66`（過去の書き換えの記録の引用）
- 触らない約束の `crates/areka/src/placement_tests/`・`crates/dola/`・`crates/areka-ghost/`・`crates/areka-seriko/`

確かめ: `cargo check -p areka-runtime -p areka-kanade -p areka -p areka-session --all-targets` 警告 0・`cargo fmt --all -- --check` 差分なし。

### 6.4

古いパスと古い出所の名前を `git grep` で数えた。範囲は `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md`（`P` とする）。`roadmap-history.md`・`roadmap.md`・`.kiro/specs/completed/`・`doc/roadmap-overview-2026-10-16.html` は範囲の外（初めから数えない）。

コマンドと件数:
- `git grep -n -F 'crates/areka/src/session/' -- $P` → 1 件（`crates/pilot/examples/pilot-windows-rs-next/REWRITE_TABLE.md:66`・除外 B）
- `git grep -n -E 'areka/src/session/|(^|[^A-Za-z0-9_/-])session/[A-Za-z0-9_]+\.rs' -- $P` → 5 件（`areka-placement` の 4 か所と `REWRITE_TABLE.md:66`・すべて除外）。前に `-` が付く `areka-session/<名>.rs`（新しい名・本体の `session_tests/` 6 件）は移した先なので数えない
- `git grep -n -E 'areka::session(::|=|$)' -- $P` → 0 件
- `git grep -n -F 'areka::session' -- $P`（字面すべて） → 0 件

除外した所（残りの 5 件はすべてここに含まれる）:
- `crates/areka-placement/src/diag.rs:46`・`persist.rs:330`・`resolver.rs:97`・`resolver_resolve_balloon_tests.rs:123`（`session/boot_seam.rs` を名指す・触らない約束・`audit-areka-app` へ申し送り）
- `crates/pilot/examples/pilot-windows-rs-next/REWRITE_TABLE.md:66`（過去の書き換えの記録の引用）
- `crates/areka-placement/examples/window-placement.rs:37`（古い `crates/areka/src/smoke_exit.rs`・上の 3 つのパターンには当たらないが申し送りの一覧に載せる）

除外のほかに当たった所: 0 件。

RED: 該当なし（数えるだけ）。

## 段 6

### 7.1

構造の証跡（RED 該当なし）。2026-11-22 に取った。

#### テストの名前の末尾の多重集合（8.2・8.3）

- コマンド: `cargo test --workspace --no-fail-fast -- --list`（標準出力を `target/session-crate-split/tests-after.txt` に残した・終了コード 0）。`: test$` の行から `: test` と先頭の `パス::` を落として並べ（段 0 の `tests-before.txt` と同じ手順）、`diff` で比べた。
- `: test$` の行: 前 14443 → 後 14451（差 +8）。
- 枝の基点は `ba887213c`。作業の枝は `main` を取り込んでいない（`git log --merges ba887213c..HEAD` は 0 件）。`main` は着地済みの兄弟（`test-name-work-tags`・`dola-cue-test-builder`）で進んでいるが、枝には入っていないので、この差に混ざらない。

| 増えた末尾 | 件数 | 説明 |
|---|---|---|
| `busy_slots_are_asked_update_first` | +1 | OrderTests（集めた層の `body_systems_tests.rs`） |
| `menu_poll_and_after_input_sit_in_the_registration_order` | +1 | OrderTests（集めた層の `body_systems_tests.rs`） |
| `window_slots_sit_between_the_input_attachments` | +1 | OrderTests（集めた層の `body_systems_tests.rs`） |
| `body_slots_follow_the_boot_and_switch_order` | +1 | OrderTests（集めた層の `body_systems_tests.rs`） |
| `register_systems_installs_body_systems` | +1 | OrderTests（本体の `session_tests/body_systems_tests.rs`） |
| `after_input_registers_install_update_mcp_in_order` | +1 | OrderTests（本体の `session_tests/body_systems_tests.rs`） |
| `missing_body_systems_logs_and_answers_like_no_desk` | +1 | OrderTests（本体の `session_tests/body_systems_tests.rs`） |
| `crash_filter_is_installed_before_the_tests_start` | +1 | 集めた層の `crash_report_tests.rs`（新しいテスト exe の TestBinaryGuard）（同名は本体・runtime・input にも在る） |

- 減った末尾: 0。移した・分けたテストは名前を変えていないので差に出ない。説明のつかない差は 0。見込み（+8）と一致した。

#### 警告と触らない約束（8.4・3.7）

- `cargo check --workspace --all-targets`: 警告 0・エラー 0。
- 基点は `git merge-base main HEAD` = `7cb95c3e8`（`main` の先頭は後続の着地で進んでいる。比べは枝が持つ範囲）。`git diff --stat 7cb95c3e8 HEAD -- <場所>`:
  - `crates/areka-placement/`・`crates/dola/`・`crates/areka-ghost/`・`crates/areka-seriko/`: 差分なし（空）。
  - 本体の `crates/areka/src/placement_tests/`: 4 ファイル・各 1 行の import の書き換え（開発者の裁定で約束の例外とした・2026-11-22）。4.2 で集めた層の支え（`crate_source_test_support`）の出し先が替わったための付け替えで、design.md の約束の文と確かめの表に例外として書き足した:
    - `measure_bake_count_tests.rs`: `-use crate::crate_source_test_support::crate_source;` → `+use areka_session::crate_source;`
    - `measure_template_tests.rs`: `-use crate::crate_source_test_support::crate_source;` → `+use areka_session::crate_source;`
    - `zorder_group_branch_coverage_tests.rs`: `-use crate::crate_source_test_support::crate_source;` → `+use areka_session::crate_source;`
    - `zorder_inversion_branch_coverage_tests.rs`: `-    let fixture = crate::crate_source_test_support::crate_path(` → `+    let fixture = areka_session::crate_source_test_support::crate_path(`
  - `crates/areka-runtime`・`crates/areka-kanade`: 9 ファイル・13 行ずつの入れ替え。差分の `+`／`-` の行をすべて取り出し、`//` で始まる行以外を数えると 0 行（コメントだけ）。

#### 本体を引かないこと（3.4・4.7）

- `cargo tree -p areka-session -e normal`: `areka v` の行 0 件。`log-capture-kit`・`temp-path-kit`・`sample-ghost-kit` の行 0 件（本番の依存は下の crate だけ）。
- 4.7 の式: ⑴ `crates/areka-session/Cargo.toml` に `areka =` の行 0 件。⑵ `cargo tree -p areka-session -e normal,dev` に `areka v` の行 0 件（番人の 3 crate は `[dev-dependencies]` にだけ現れる・5.9 の決まりどおり）。⑶ `crates/areka-session/src` の `areka::` の字面は 46 行（`log_history.rs`・`log_history_tests.rs`・`log_history_convention_tests.rs`）だが、すべて `"areka::install"` などの本体が持つ出所の名前の文字列で、設計が「語彙の直書きは変えない」とした所。引用符の外の Rust のパス・`//` を除いたコードの行の `areka::` は 0 件。

#### テストの置き場の表（5.1・設計の TestPlacement）

| テストのファイル（旧 `session/`） | 置き場 | 理由 |
|---|---|---|
| `session_end_tests.rs`・`ghost_session_switch_fallback_tests.rs`・`ghost_session_switch_memory_tests.rs`・`readonly_run_tests.rs`・`readonly_sites_tests.rs`・`write_failure_sites_tests.rs`・`boot_config_module_policy_tests.rs` | 本体 | 根の関数（`after_run`・`finish_after_run`・`boot_first_ghost`・`install_boot_context`）を引く（5.3） |
| `boot_config_{balloon,copy,input,option,roots,callghost}_tests.rs` | 本体 | `resolve_boot_from` を通して `cli::parse` を引く（5.3・D12） |
| `ghost_session_restart_tests.rs`・`ghost_session_reload_tests.rs` | 本体 | `SpineHarness`・`BODY_SYSTEMS`・menu を引く（5.6） |
| `ghost_session_strict_tests.rs`・`ghost_session_switch_tests.rs`・`shell_balloon_switch_session_tests.rs` | 本体 | menu の本物（`MenuWiring` ほか）を見る（5.2） |
| `ghost_session_scope_rect_boot_tests.rs`・`ghost_session_scope_scale_boot_tests.rs` | 本体 | mcp・`shiori_host` の本物を見る（5.2） |
| `boot_resolve_mark_text_tests.rs`・`shell_balloon_switch_session_update_tests.rs` | 本体 | install・update の本物を見る（5.2） |
| `reload_tests.rs`（ファイルごと） | 本体 | `refusals_in_order_leave_one_record_each` が本物の `InstallDesk`・`UpdateDesk` を据えて断りの順を見る。足場（`rig`・`refuse`）が `TempPath`・`capture` に頼るので分けない（D8 ⑵） |
| `shell_balloon_switch_tests.rs`（ファイルごと） | 本体 | `refusals_in_order_leave_one_warning_each_and_send_nothing` が本物の `UpdateDesk` を据える。同じ理由で分けない |
| `readonly_tests.rs` の `engage_is_called_only_from_main`・`main_calls_engage_exactly_once_before_resolve_boot`・`withhold_everything_is_called_only_from_readonly`（小道具 `all_sources`・`files_calling` も） | 分ける → 本体 `readonly_body_tests.rs` | 本体の `main.rs`・`src` を読む（5.13）。3 本目は同じ小道具でワークスペースを歩くので一緒に移す |
| `readme_tests.rs` の `only_the_os_port_spells_shell_execute` | 分ける → 本体 `readme_body_tests.rs` | 本体の `src` を歩く（5.13） |
| `session_end_deadline_tests.rs` の `only_session_end_calls_the_bounded_shutdown`・`deadline_call_scan_turns_red_on_one_line_removed_or_added` | 分ける → 本体 `session_end_deadline_body_tests.rs` | 本体の `src` を歩く（5.13） |
| `log_history_convention_tests.rs` の `every_rule_target_names_a_module_or_literal_that_exists`・`locate_turns_red_for_missing_and_underivable_targets` | 分ける → 本体 `log_history_convention_body_tests.rs` | 決まりの出所（`areka::install` ほか本体のもの）の在りかを本体の `src` で確かめる（5.13） |
| `app_exit_escape_tests.rs` | 集めた層 | 見るのは集めた層の `quit_app` と `InputHooks` だけ。`crate::BODY_SYSTEMS` の 1 行を `NO_BODY` へ（中身の表明は変えない） |
| `runtime_boot_tests.rs` | 集めた層 | 口の無い World で 13 個（D4）。`acquire_emo2` を `SampleRoot::acquire("emo2")` へ |
| `boot_shell_tests.rs`・ほか `SwitchRig` を通る集めた層の 30 本と残りの全部 | 集めた層 | 本体の項目を名指さない。`SwitchRig::new` が `NO_BODY` を渡す |
| `shell_balloon_switch_session_update_tests.rs`（D9 の比べ） | 本体 | 偽物の組で `a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon` が赤（更新の窓口が据わらず、`update/desk.rs` が無い非 Send の資源を読んで panic）。足場を集めた層の `lap_rig` → `SwitchRig::new` から受けていたので、`SwitchRig::new(` の字面を数えた書き換えから漏れていた。`lap_rig_of` の後半を `lap_rig_on(rig, shells)` に分け、このファイルは本体の `switch_rig` で組んだ足場を渡す。もう 1 本（`update_targets_follow_the_swapped_shell_and_balloon`）は差なしだが、足場 `steady_rig` を分け合うのでファイルごと移す |
| 偽物の組で `homeurl` の照会だけが減る 24 本（`ghost_switch_reload_tests.rs` 9・`ghost_switch_balloon_tests.rs` 6・`ghost_switch_boot_event_tests.rs` 3・`shell_balloon_switch_session_abort_tests.rs` 2・`ghost_quit_switch_tests.rs`・`boot_shell_tests.rs`・`ghost_session_switch_translate_tests.rs`・`ghost_switch_talk_clock_tests.rs` 各 1） | 集めた層 | 本物の組では定常の知らせで本体の更新の窓口が `homeurl` の照会を 1 回送る。`SwitchRig::calls` はこの照会をもともと除くので、表明が見る列は同じ。照会が増やすのは進みの目印の 1 だけで、偽物の組でも待ちは打ち切られない（2 回とも緑）。照会そのものは本体の `update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time` が見る |
| `boot_shell_tests.rs` の `animated_hidden_shell_boot_emits_each_load_record_once`・`remembered_hidden_shell_reaches_all_resolutions_decided_once` | 集めた層 | 違うのは `on_steady` の回数（本物の組 0・偽物の組 1。後者は本物の組でも回により 0／1）。1 回起こすだけのテストは定常の知らせを汲む前後で終わるので回による。表明は読み込みの記録の数だけを見て、`on_steady` の先の本体の窓口を見ない |
| 終わりの道で `exit_wait` の門を通る集めた層のテスト（`session_end_deadline_tests.rs` 7・`session_end_sync_send_tests.rs` 1・`shell_balloon_switch_session_abort_tests.rs` の `close_request_while_waiting_drops_the_switch_and_exits_as_today`） | 集めた層 | 偽物の組では install・update の門が据わらないが、本物の組でも門は空で待たない（記録・口の回数とも差なし）。仕事のある門の待ちは本体の `install/worker_tests.rs`・`install/desk_exit_tests.rs`・`update/worker_tests.rs` ほかが見る |
| `reload_tests.rs`・`shell_balloon_switch_tests.rs` の `insert_resource(crate::BODY_SYSTEMS)` の 1 行（D13） | 本体 | 上の行のとおりファイルごと本体へ移すので、1 行はそのまま移す。`update_busy`・`install_busy` の照会が真のときの断りを見るのはこの 2 本で、偽物の組の照会は常に偽 |

- 本物の組と偽物の組で合否か記録が違う集めた層のテストは、1 本ずつ置き場と理由を決めてこの表に足す（D9）。静かに読んだ見込み（走らせていない）: 本体の `update::register` が据える `UpdateDesk` は定常のたびに `homeurl` の照会を偽の SHIORI へ送るが、`SwitchRig` の記録はもともとこの照会を除いて見る（`ghost_switch_test_support.rs:446〜458`）。進みの目印（`progress_probe`）は照会も数えるので、待ちの打ち切りの時点が変わりうる。`exit_wait` の門（install・update）も据わらなくなる。この 2 つに当たるテストが赤になるかは段 2 で走らせて決める。走らせた結果（2026-11-22・本物の組と偽物の組を 2 回ずつ）: `SwitchRig::new` に届く集めた層のテストは 16 ファイル 52 本で、赤は 1 本（上の `shell_balloon_switch_session_update_tests.rs` の行）。緑で記録が違うのは `homeurl` の照会 24 本と `on_steady` の回数 2 本で、上の行のとおり集めた層に残す。比べた表は検証の記録の 3.2。
- 分けたファイルのテストは、名前と中身を変えない。小道具は D8 の 2 通りで扱い、写しを作らない。`test-support` へ移した小道具は 4.5 の記録に入れる。
- この表を検証の記録に写す（5.1）。

#### 比べた結果の表（3.2・本物の組と偽物の組・SwitchRig を通る 52 本）


合否は「本物の組／偽物の組」。記録の差は「本物の組 → 偽物の組」で、2 回が違うときは回ごとに書く。差の無い欄は書かない。

| ファイル（旧 `session/`） | テスト | 合否 | 記録の差 | 決めた置き場 |
|---|---|---|---|---|
| `ghost_quit_switch_tests.rs` | `stop_with_handoff_under_reservation_switches_without_exit` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `boot_shell_tests.rs` | `animated_hidden_shell_boot_emits_each_load_record_once` | 緑／緑 | `on_steady` 0→1 | 集めた層（乙） |
| `boot_shell_tests.rs` | `each_switch_adds_one_more_set_of_load_records` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `boot_shell_tests.rs` | `missing_remembered_shell_boots_default_and_rewrites_memory` | 緑／緑 | なし | 集めた層（丙） |
| `boot_shell_tests.rs` | `production_boot_emits_each_load_record_once` | 緑／緑 | なし | 集めた層（丙） |
| `boot_shell_tests.rs` | `remembered_hidden_shell_reaches_all_resolutions_decided_once` | 緑／緑 | 1 回目 `homeurl` 1→0／2 回目 `on_steady` 0→1 | 集めた層（乙） |
| `ghost_session_runtime_tests.rs` | `boot_failure_returns_the_unit_without_runtime_to_the_slot` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_session_runtime_tests.rs` | `rebuilt_runtime_is_attached_to_the_same_unit` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_abort_tests.rs` | `close_request_while_waiting_drops_the_switch_and_exits_as_today` | 緑／緑 | なし | 集めた層（丙・門） |
| `shell_balloon_switch_session_abort_tests.rs` | `ghost_switch_after_a_shell_swap_unloads_the_session` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `shell_balloon_switch_session_abort_tests.rs` | `ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `shell_balloon_switch_session_abort_tests.rs` | `marked_talk_break_cancels_the_shell_switch_without_quitting` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_abort_tests.rs` | `undecodable_shell_keeps_the_old_mount_and_records_one_error` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_balloon_tests.rs` | `script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_lap_tests.rs` | `menu_switch_to_the_current_shell_rebuilds_and_raises_both_events` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_lap_tests.rs` | `plain_script_shell_switch_skips_changing_and_waits_for_the_script_end` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_lap_tests.rs` | `script_shell_switch_round_trips_with_raise_event` | 緑／緑 | なし | 集めた層（丙） |
| `shell_balloon_switch_session_update_tests.rs` | `a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon` | 緑／赤 | 作り口 2→1・`after_runtime_wired` 2→1・`on_switch_end` 1→0・`on_steady` 1→0・`overwrite_between` 1→0・`cue_sinks` 2→1・`calls` の列が違う・`homeurl` 1→0 | 本体（ファイルごと） |
| `shell_balloon_switch_session_update_tests.rs` | `update_targets_follow_the_swapped_shell_and_balloon` | 緑／緑 | なし | 本体（ファイルごと） |
| `ghost_session_switch_translate_tests.rs` | `switch_sends_each_on_translate_to_its_own_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `a_memory_written_before_this_spec_finds_the_ghost_inner_balloon` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `a_remembered_ghost_inner_balloon_that_is_gone_moves_on_to_the_next_route` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_does_not_see_the_balloon_inside_the_previous_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_to_a_ghost_whose_descript_names_a_balloon_uses_it_and_records_the_descript_route` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_to_a_ghost_with_its_own_balloon_uses_and_remembers_it_by_folder_name` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_balloon_tests.rs` | `switching_to_a_ghost_with_numbered_companions_uses_the_first_and_records_the_companion_route` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_boot_event_tests.rs` | `a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_boot_event_tests.rs` | `a_switch_with_a_boot_event_boots_the_target_with_that_event_instead_of_changed_or_boot` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_boot_event_tests.rs` | `a_switch_without_a_boot_event_boots_the_target_with_on_ghost_changed_as_today` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_failed_ghost_reload_hands_no_carry_to_the_default_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_failed_shiori_rebuild_falls_back_to_the_default_ghost` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_failed_shiori_rebuild_of_the_default_ghost_is_fatal` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_reload_tests.rs` | `a_foreign_switch_under_a_reload_mark_boots_as_before_and_drops_the_mark_as_overtaken` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_ghost_reload_hands_the_shown_surfaces_to_the_new_windows_but_a_switch_does_not` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_ghost_reload_keeps_the_ghost_inner_balloon_through_the_memory` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_ghost_reload_reboots_with_no_boot_notice_but_with_the_information_notices` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_shiori_rebuild_that_cannot_write_the_drop_memory_is_fatal` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_reload_tests.rs` | `a_shiori_reload_keeps_the_sticky_ledger_and_anchor_source` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_shiori_reload_keeps_the_windows_and_rebuilds_only_the_runtime` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_reload_tests.rs` | `a_switch_without_the_mark_boots_with_on_ghost_changed_as_before` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `ghost_switch_rig_wait_tests.rs` | `a_rig_wait_that_never_arrives_names_the_callers_place` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_rig_wait_tests.rs` | `the_rig_progress_ignores_status_and_counts_every_factory_call` | 緑／緑 | なし | 集めた層（丙） |
| `ghost_switch_talk_clock_tests.rs` | `a_rig_clock_far_ahead_of_a_stalled_handshake_leaves_the_switch_unchanged` | 緑／緑 | `homeurl` 1→0 | 集めた層（甲） |
| `reload_skin_tests.rs` | `descript_ghost_replaces_names_and_readme_then_reserves_the_current_shell` | 緑／緑 | なし | 集めた層（丙） |
| `session_end_deadline_tests.rs` | `answered_within_limit_is_not_cut` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `cut_while_in_flight_get_is_held` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `cut_while_on_close_notify_is_held` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `cut_while_unload_is_held` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `deadline_fires_without_manual_cut` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `self_timeout_shorter_than_limit_is_not_cut` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_deadline_tests.rs` | `unbounded_shutdown_never_cuts` | 緑／緑 | なし | 集めた層（丙・門） |
| `session_end_sync_send_tests.rs` | `teardown_join_does_not_wait_for_a_sync_send_to_the_ui_window` | 緑／緑 | なし | 集めた層（丙・門） |
理由:

- 甲（`homeurl` の照会だけが減る・24 本）: 本物の組では定常の知らせで本体の `tell_desks_steady` → 更新の窓口が `homeurl` の照会を偽の SHIORI へ 1 回送る。`SwitchRig::calls` はこの照会をもともと除くので、表明が見る列は同じ。照会が増やすのは進みの目印（`progress_probe`）の 1 だけで、偽物の組でも待ちは打ち切られていない（2 回とも緑）。照会そのものは本体の `update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time` が見るので、集めた層に残しても守りは失わない。
- 乙（`on_steady` の回数・2 本）: 定常の知らせを汲んだ回数で、1 回起こすだけのテストは汲む前後で終わるので回による（`remembered_hidden_shell_…` は本物の組でも 1 回目 1・2 回目 0）。表明は読み込みの記録の数だけを見て、`on_steady` の先の本体の窓口を見ない。
- 丙（差なし・24 本）: 呼出列・照会・欄の回数・作り口の回数が同じ。
- 門（`exit_wait` の門を通るもの）: `session_end_deadline_tests.rs` の 7 本・`session_end_sync_send_tests.rs` の 1 本（`end_session`／`end_session_within`）と `close_request_while_waiting_drops_the_switch_and_exits_as_today`（`on_ghost_os_close`）。偽物の組では本体の install・update の門が据わらないが、本物の組でも門は空で待たず、記録も欄の回数も差が無い（`update_busy`・`install_busy` はどちらの組でも 0 回）。仕事のある門の待ちは本体の `install/worker_tests.rs`・`install/desk_exit_tests.rs`・`update/worker_tests.rs` ほかが見る。集めた層の `reload`・`shell_balloon_switch` の門は集めた層の登録で据わるので組によらない。
- `UpdateDesk` の照会（`update_busy`）が呼ばれたのは `_update` の赤の 1 本（どちらの組でも 2 回）だけ。照会が真のときの断りを見るのは `reload_tests.rs`・`shell_balloon_switch_tests.rs`（下の ※）で、どちらも本体へ移す。
- 本体（`shell_balloon_switch_session_update_tests.rs`・ファイルごと）: 赤の 1 本は更新の窓口の読み直し（`ask_reload_for_test`）を通すので本体の窓口が要る。足場を集めた層の `lap_rig` → `SwitchRig::new` から受けていて、3.1 の `SwitchRig::new(` の字面の数えに入らなかった。もう 1 本は差なしだが足場 `steady_rig` を分け合う。表の既にある行（install・update の本物を見る）のとおりファイルごと移す。


#### 開けた項目の一覧

##### 3.4 で `pub(crate)` にした項目と使う本体のテスト

| 開けた項目（集めた層） | 前 | 使う本体のテストのファイル |
|---|---|---|
| `ghost_session.rs` の `GhostSession::seriko`・`GhostSession::loop_ticker`（欄） | 私有 | `ghost_session_reload_tests.rs` |
| `ghost_session.rs` の `PreparedWindows::spawn`（欄） | 私有 | `ghost_session_restart_tests.rs` |
| `reload.rs` の `drain` | 私有 | `reload_tests.rs` |
| `write_failure.rs` の `drain_system` | 私有 | `write_failure_sites_tests.rs` |
| `ghost_session_scope_rect.rs` の `mod tests`・`ghost_session_scope_rect_tests.rs` の `LEFT_SCREEN`・`mod wiring`・`wiring::fake_handle`・`wiring::placement` | 私有・`pub(super)` | `ghost_session_scope_rect_boot_tests.rs`（3.5 で `ghost_session_scope_rect_test_support.rs` へ移す） |
| `ghost_session_scope_scale.rs` の `mod tests`・`ghost_session_scope_scale_tests.rs` の `applied` | 私有 | `ghost_session_scope_scale_boot_tests.rs`（3.5 で `ghost_session_scope_scale_test_support.rs` へ移す） |
| `ghost_session.rs` の `mod shell_balloon_switch_session_lap_tests`・その `SECOND`・`UNBOUNDED`・`Ticks`・`LapRig`・`LapRig::rig`（欄）・`LapRig::frames_until`・`lap_rig_on`・`copy_tree`・`kinds`・`got`・`idle` | 私有・`pub(super)` | `shell_balloon_switch_session_update_tests.rs` |
| `reload.rs` の `#[cfg(test)] pub(crate) mod test_support`（新設 `reload_test_support.rs`・`boot_context`・`plant_skin_switch`） | `reload_tests.rs` の `pub(super)` | `reload_tests.rs`（集めた層の `reload_skin_tests.rs` も使う） |

- `reload_test_support.rs`（設計に無い・申し送り）: 集めた層に残す `reload_skin_tests.rs` が、本体へ移す `reload_tests.rs` の小道具 `boot_context`・`plant_skin_switch` を `super::tests::` で引いていた（TestPlacement の表の D8 ⑵ は本体へ移すファイルの足場が番人の crate に頼ることだけを見ていて、逆向きの使い手を数えていなかった）。2 つは番人の crate に頼らないので、D8 ⑵ の前者のとおり集めた層の兄弟 `reload_test_support.rs`（`#[test]` を持たない）へ写しを作らずに移し、両方から使う。`boot_context` の 1 つ目の引数は `&TempPath` から `&Path` に替え（中で使っていたのは `temp.path()` だけ）、呼び手 2 か所を `temp.path()` を渡す形にした（`test-support` には `temp-path-kit` を入れられないため）。4.1 で `lib.rs` に `ghost_session_scope_{rect,scale}_test_support` と同じ `#[cfg(any(test, feature = "test-support"))] pub mod` で宣言する対象に足す。
- `shell_balloon_switch_session_lap_tests.rs` の小道具（3.2 の申し送り）は、この段ではテストのモジュールのまま `pub(crate)` で開けた。4.2 の公開面で、集めた層の `#[cfg(test)]` のモジュールは本体から見えなくなるので、置き場（`test-support` の兄弟へ移すか）を決める。足場の `LapRig` は `GpuPermit`（runtime の `test-support`）と `SwitchRig` に頼り、番人の crate に頼るか（`TempPath`・`capture`）はそのとき確かめる。


##### 4.2 で `test-support` の条件つきで `pub` にした項目

#### 本体のテストだけが使う `#[cfg(test)]` の項目（`#[cfg(any(test, feature = "test-support"))] pub`）

| 項目 | 前 | 使う本体のテスト |
|---|---|---|
| `GhostSession::{for_test, with_memory_publisher, with_seriko_sink}` | `#[cfg(test)] pub(crate)` | `session_tests/`・`runtime_tests/`・`install`・`update`・`mcp` のテスト |
| `GhostSession::dispatcher` | `#[cfg(any(test, feature = "test-support"))] pub(crate)`（4.1） | 同上 |
| `GhostSession` の欄 `memory_publisher_for_test` とその読み・組み立て 4 か所 | `#[cfg(test)]`（欄は私有のまま） | `with_memory_publisher` が立てる |
| `roots::single_env` | `#[cfg(test)] pub(crate)` | 本体のテスト 23 か所 |
| `shell_balloon_switch::SwapFinish` の出し直し | `#[cfg(test)] pub(crate) use` | `runtime_tests/frame/switch_finish_tests.rs` |
| `log_history::hold` | `log_history_capture_tests.rs` の `pub(crate) fn hold`＋`static WINDOW` | `offline_dump/offline_dump_tests.rs`（`use` を `areka_session::log_history::hold` へ） |
| `ghost_switch_test_support`（モジュールは 4.1） | 中の項目が `pub(crate)` | 本体の 17 ファイル＋`session_tests/`（中の項目を `pub` に） |
| `ReadmeWiring::path`（設計の 8 つの外） | `#[cfg(test)] pub(crate)` | `session_tests/ghost_session_restart_tests.rs`・`ghost_session_switch_tests.rs` |
| `ghost_session::shell_balloon_switch_session_lap_test_support`（新設・下） | ― | `session_tests/shell_balloon_switch_session_update_tests.rs` |

- `hold` の `static WINDOW` は `log_history.rs` の写しの窓そのものの `static WINDOW`（`Mutex<Window>`）と名がぶつかるので、`hold` の関数の中に置いた（名と中身は前のまま）。テストのモジュール `capture_tests` は `#[cfg(test)]` のまま。使われなくなった `use std::sync::Mutex;` を外した。
- `ghost_switch_test_support` の中で集めた層のテストだけが使う `Stall::{at, entered, release}`・`SwitchRig::{add_balloon_copy, plant_last_balloon}` は、`--lib --features test-support` で死んだコードの警告になるので `pub` にした（モジュールが `test-support` の条件の下にある）。

#### 本番の項目のうち本体のテストだけが使うもの（素の `pub`）

- 3.4 で開けた私有の項目（`GhostSession::seriko`・`GhostSession::loop_ticker`・`PreparedWindows::spawn` の欄、`reload::drain`、`write_failure::drain_system`）は、集めた層の本番のコードも使う欄と関数で、可視性だけを機能で切り替えると項目を 2 つ書くことになる。`#[cfg(any(test, feature = "test-support"))] pub` にできないので素の `pub` にした（設計の「本番の公開面に中の関数を出さない」との違い・申し送り）。
- `boot_seam::insert_persist_wiring` は `#[cfg(test)]` ではない本番の関数で、本体の `session_tests/readonly_run_tests.rs`・`readonly_sites_tests.rs`・`write_failure_sites_tests.rs` が使う（設計の「本体が使わない（0 件）」は 3.4 で 3 本が本体へ移る前の数）。素の `pub`。
- ほかは一覧の T の行（`boot_resolve::{read_session_mark, clear_session_mark, keep_session_mark, …}`・`reload::{ReloadInFlight, ReloadStage, register, …}` ほか）。

#### 連れて開けた型（`private_interfaces`）

- 本番の本体の段: `boot_resolve::BalloonRoute`・`exit_wait::ClosingWaits`・`ghost_session::OpenWindowsError`・`ghost_switch::{SwitchTarget, PrevGhost, SwitchStage, WelcomeAttempt}`・`switch_copy::SwitchCopy`・`log_history::{Stamp, Hms}`・`reload::RefusedReason`・`roots::RootSkip`・`session_mark::MarkVerdict`。
- 本体のテストの段: `ghost_session_runtime::{RuntimeRebuild, rebuild_runtime_in_place}`（`ghost_session.rs` の `pub(crate) use` を `pub use` にした E0364・E0365）・`reload::SkinStep`・`log_history::Rule`・`readme::destination::{Destination, Target, Store}`・`boot_resolve::NoBalloon`・`app_exit::WindowsClosed`・`ghost_session::BootWiringFailed`・`ghost_switch_test_support::Stall`。
- 出し直しの広げ: `shell_balloon_switch.rs` の `areka_runtime::skin_switch::{SkinKind, …}`・`ghost_switch.rs` の `areka_runtime::special_name::{LastInstalledGhost, …}` を `pub(crate) use` → `pub use`。

#### lap_tests の小道具の置き場

- 本体の `shell_balloon_switch_session_update_tests.rs` が使う `lap_rig_on`・`SECOND`・`UNBOUNDED`・`Ticks`・`LapRig`・`LapRig::frames_until`・`copy_tree`・`kinds`・`got`・`idle` は、集めた層の `shell_balloon_switch_session_lap_tests.rs` の中と兄弟の `_abort_tests.rs`・`_balloon_tests.rs` も使う（両側の使い手）。番人の crate に頼るのは `add_rpost_shell`（`SampleRoot::acquire("R_POST_and_KOMAINU")`）だけで、残りは頼らない。
- 決め（D8 ⑵ の前者）: 番人に頼らない小道具を、行の範囲の移動で兄弟の `shell_balloon_switch_session_lap_test_support.rs`（`#[cfg(any(test, feature = "test-support"))] pub mod`・`ghost_session.rs` で宣言・`#[test]` を持たない）へ移した。移したのは上の 10 個と、それが引く `NO_TICKS`・`CREEP`・`ACCEPT`（`Ticks` の私有の欄を組むので一緒に）・`WORK_AREA`・`TICK_EVERY`・`UiMarks`・`UiSeen`（`ui_seen_counts_only_ui_side_progress` が使うので `pub(crate)`）・`spawn_windows`・`PUMP_MAX`・`pump_messages`・`refs_of`・`calls_a`。lap_tests は `pub(crate) use super::shell_balloon_switch_session_lap_test_support::*;` で引き、兄弟 2 本の `use super::shell_balloon_switch_session_lap_tests::{…}` は変えていない。テストの名前と中身は変えていない。
- 番人に頼る `add_rpost_shell` は分けられないので、`lap_rig_on` は 2 つ目のシェルを足す関数を 3 つ目の引数 `add_shell: fn(&Path, &str)` で受ける形にした。集めた層の `lap_rig_of` は lap_tests の `add_rpost_shell` を、本体の `steady_rig` は本体のテストのファイルに置いた `add_rpost_shell`（同じ 9 行・理由の注つき）を渡す。この 9 行は両側の写し（`test-support` に `sample-ghost-kit` を入れられないため・申し送り）。ほかの写しは無い。


##### 4.4 で据えた `insert_resource(crate::BODY_SYSTEMS)` の 1 行を足したテスト

2.5 で足した 5 本のうち、移した先でも 1 行を持つのは本体の `reload_tests.rs`・`shell_balloon_switch_tests.rs` の 2 本で、4.4 で 3 本が増えた（本体で 1 行を持つのは計 5 本）。

足したテスト（`session/` の下は ※。集めた層は最後に `crate::BODY_SYSTEMS` を名指せないので、3.2・3.4 で本体へ移すか、`NO_BODY` に替えて理由を記録する）:

| テスト | ファイル:足した行 |
|---|---|
| `install::desk::tests::resend_goes_at_once_when_steady_arrived_since_the_last_send` | `crates/areka/src/install/desk_tests.rs:385` |
| `install::desk::tests::a_cancelled_overwrite_switch_goes_back_to_held_and_retries_on_the_next_steady` | `crates/areka/src/install/desk_tests.rs:554` |
| `update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time` | `crates/areka/src/update/desk_resolve_tests.rs:558` |
| ※ `session::reload::tests::refusals_in_order_leave_one_record_each` | `crates/areka/src/session/reload_tests.rs:183` |
| ※ `session::shell_balloon_switch::tests::refusals_in_order_leave_one_warning_each_and_send_nothing` | `crates/areka/src/session/shell_balloon_switch_tests.rs:287` |

#### `world.insert_resource(crate::BODY_SYSTEMS);` の 1 行を足したテスト（D13・表明は変えていない）

| テスト | 赤の形 |
|---|---|
| `session_tests::reload_tests::missing_ghost_or_kanade_is_no_context` | warn 以上の総数 2（期待 1） |
| `session_tests::shell_balloon_switch_tests::acceptance_starts_one_wait_marked_only_for_menu_and_raise_event_shells` | warn＋error の数 1（期待 0）・5 件とも |
| `session_tests::shell_balloon_switch_tests::send_failure_leaves_one_error_and_no_marker` | error の総数 2（期待 1） |

- 前からあった 1 行（`reload_tests.rs` の `refusals_in_order_leave_one_record_each`・`shell_balloon_switch_tests.rs` の `refusals_in_order_leave_one_warning_each_and_send_nothing`）と合わせ、本体で 1 行を持つテストは 5 本。


### 7.2 THIRD-PARTY-NOTICES.md の再生成（要件 8.11）

- 実行: `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（`tools/test-all.ps1` の `-License` の段と同じ呼び出し・cargo-about 0.9.3）。手では編集していない。
- 差分: `git diff --stat` は 1 file, 2 insertions(+), 1 deletion(-)。中身は 2 か所だけ。
  - `- MIT License (265 crate)` → `- MIT License (266 crate)`
  - MIT 節の末尾に `- areka-session 0.1.0` を 1 行追加
- 版のずれなどそのほかの差分は 0 件。
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok。

### 7.3

- 排他テストの机を取り、`pwsh -NoProfile -File tools/test-all.ps1` を直さずに 1 回回した（コミット `f467358e6`）。出力は `target/session-crate-split/7.3-test-all.log`。
- 要約は `all steps green`。21 の段がすべて OK（fmt --check 6 s・x64 workspace tests 410 s・dola toml feature tests・i686 tests (host-32)・crates.io pre-publish check・encoding check・load flake check・perf loop selftest ほか）。`test result: ok` の行は 180。
- 含まれていることを出力で確かめたもの: `areka_session` の lib のテストの実行ファイル（`Running unittests src\lib.rs (… areka_session-….exe)`）・1,000 行の番人 `file_length_guard_test`・番人の決まり `with_default_guard_test`・`sample_path_guard_test`・`temp_path_guard_test`。
- 赤は無く、回し直しはしていない。走らせ終えてすぐ机を返した。

### 7.4

- 排他テストの机を取り、`cargo test -p areka --test emo2_mcp_real_run -- --nocapture` を今の手順で 1 回走らせた（コミット `0b1413ca8`）。出力は `target/session-crate-split/7.4-emo2-mcp.log`。
- 結果は緑（1 passed・25.31 秒）。証拠は 19.4 秒で集め終え、利用者と同じ閉じ方（`UserClose`）で終了コード 0。
- 新しい出所の名前で行を拾えたことは、テストの表明そのものが示す。`BOOT_CONFIG_PREFIX = "areka_session::boot_config: "` と、`rust_log` の `areka_session::boot_config=info` ほかで、標準出力の行を拾えなければこのテストは赤になる。
- 拾った行の数の比べ: 移す前（着手のとき）にこのテストの行の数を記録していなかったので、数どうしは比べられない。テストは拾う行を `ghost_resolved` の 1 行と出どころの記録に絞って表明しているので、緑であることは同じ行を拾えたことを意味する。
- 走らせ終えてすぐ机を返した。

### 7.5

#### 走り（8.7・2.5）

- 手順は 1.2 と同じ `target\session-crate-split\real\six-paths.ps1`（1.2 の節に写した中身から 1 バイトも変えていない。走らせる前に写しとバイトで一致することを確かめた）。違いは `-OutDir` を `<作業の木>\target\session-crate-split\real-after` にしたことだけ。ビルドはコミット `bb68918d3` で `cargo build -p areka -p sample-ghost-kit --bin areka --bin nar-sample-path`・i686 の helper（`helper is i686`）。
- 2026-11-22 08:31:52〜08:34:06 UTC（約 2 分 15 秒）、排他テストの机を取り、開発者の SendInput の許しの下で 1 回取った。終了コードは 1 回目の走り（起動〜メニューの終了）・2 回目の走り（煙の自動終了）・手順そのものとも 0。
- この前に 1 回（08:27:57〜08:28:22 UTC）、点を探す途中で外からカーソルが動かされ（`mouse-input.ps1` の読み戻しが狙い (2522,1157) に対して (2564,1191)）、手順が自分で止まった。起動の段までで止まり、記録は捨てて `real-after` を消してから、開発者がマウスから手を離した状態でこの 1 回を取った。手順も areka も変えていない。
- 押した所は 1.2 と同じ: 当たる点 (2446,1157)・メニューの「ゴースト」(2622,609)・子の `emo2-switch-target` (2264,653)・投げ込みは (567,344) → (2446,1157)・「終了」(2622,1132)。メニューの並び（上の段 15 項目・子 2 つ）も同じ。エクスプローラーの窓は 40,40〜1240,840 で 1.2（40,40〜940,640）より大きく開いたが、項目の位置は同じで、キャラ窓とも重ならなかった。
- 記録（`target/session-crate-split/real-after/`）: `boot.log` 120 行・`switch.log` 128 行・`reload.log` 91 行・`nar-drop.log` 38 行・`menu-quit.log` 39 行（`run1.full.log` 416 行を段の始まりで切ったもの）・`smoke-exit.log` 132 行・`steps.txt`。
- 道ごとの節目: 切替は `menu_selected scope=0` → `ghost_switch_done ghost=Some("emo2b")`、読み直しは `reload_done target=Ghost` → `ghost_switch_done ghost=Some("emo2b")`、投げ込みは `file_drop_received scope=0` → `install_done`、メニューの終了は `menu_selected`（`frame=Close`）→ `app_exit origin=KanadeStopped(Quit)`、煙の自動終了は `app_exit origin=Smoke`。値は 1.2 と同じ。

#### 比べ方

`target\session-crate-split\real\compare.py`（使い捨て・`tools/` には足さない）で、道ごとの 6 つの記録と、切れ目の揺れを分けるために `run1.full.log` の全体も比べた。走らせ方は `python <作業の木>\target\session-crate-split\real\compare.py <作業の木>\target\session-crate-split\real-before <作業の木>\target\session-crate-split\real-after stable events`（`PYTHONIOENCODING=utf-8`）。結果は `target/session-crate-split/compare-7.5.txt` に残した。自分どうし（`real-before` と `real-before`）で比べると、どの道もどの見方でも差が 0 になることを先に確かめた。

- 行ごとに、先頭の時刻を落とし、`a::b::c: ` の形の出所の名前を落とす（これが「出所の名前を除く」）。走りごとに変わる値（pid・待受の番号・窓の番号 `0x…`・entity `69v0`・時間・`id=`・`talk_id=`・`since_id=`）を伏せる。
- 見方は 3 つ。`all` は全部の行、`stable` は乱数と壁時計で決まる 3 つの組（下の「伏せた組」）を除いた行、`events` は `event="…"` の名前だけを順に並べたもの。
- 出所の名前の付け替えは 1 対 1 で、数も同じだった（`run1.full.log` と `smoke-exit.log` の合計）: `areka::session::app_exit` 4 → `areka_session::app_exit` 4・`boot_config` 4・`boot_resolve` 10・`ghost_quit` 1・`ghost_session` 12・`ghost_switch` 10・`reload` 2・`roots::report` 2・`areka::smoke_exit` 3 → `areka_session::smoke_exit` 3。ほかの出所の名前は前後で同じ。

伏せた組（`stable` で除く行）:

- `seriko`: SERIKO の再生（`actor{actor=seriko}`）・ループの刻み（`actor{actor=loop-ticker}` の追いつき）・乱数の種の行。抽選は走りごとに実の entropy で、追いつきは壁時計で決まる。
- `talk-text`: 台本そのもの（`\p[`・`\s[` を含む行）。emo2 は台詞と `\![bind,…]` を乱数で選ぶ（例: 起動の挨拶で前は `\![bind,腕,伸び,1]`、後は `\![bind,腕,組み,1]`。読み直しの後の台詞は前が「紅葉の季節！」、後が「今日のお昼、」）。
- `balloon-timeout`: バルーンのタイムアウトの計り（`[balloon-visibility] タイムアウト計測…`・`抑止が解けた…`）。秒の小数と、「止まった時刻」と「会話の占有終端」のどちらが先に来たか（`origin="stopped_at"`／`"display_end"`）が壁時計で決まる。

#### 比べた結果の表

| 道 | 行（前/後） | `stable` の行（前/後） | `events`（前/後） | `all` の差 | `stable` の差 | `events` の差 | 本当の差 |
|---|---|---|---|---|---|---|---|
| 起動 | 111/120 | 80/80 | 18/18 | 21 | 2 | 2 | 0 |
| メニューからの切替 | 136/128 | 86/87 | 29/29 | 32 | 1 | 0 | 0 |
| MCP の `reload` | 97/91 | 81/79 | 29/29 | 28 | 6 | 0 | 0 |
| `.nar` の投げ込み | 45/38 | 30/27 | 19/19 | 19 | 5 | 0 | 0 |
| メニューの終了 | 37/39 | 28/28 | 13/13 | 12 | 0 | 0 | 0 |
| 煙の自動終了 | 148/132 | 99/92 | 25/22 | 40 | 9 | 3 | 0 |
| （1 回目の全体 `run1.full.log`） | 426/416 | 305/301 | 108/108 | 138 | 16 | 2 | 0 |

`all` の差は全部、伏せた組の 3 つの行（行の数の差もこの組の数の差: `seriko` は前後で 29/38・45/36・14/10・8/6・6/8・44/38）と、下の `stable` の差で説明がつく。

`stable` と `events` に残った差の 1 件ずつの中身（行番号は各記録のもの）:

- 起動（`stable` 2・`events` 2）: kanade の `event="boot_start"` の行が、前は 33 行目（本体の `実 sink 結線が成立`・`boot_resolve` の 2 行の間）、後は 29 行目（`ghost boot sequence completed` の直後）に出た。kanade の actor のスレッドと本体のスレッドが同じ 0.3 ms のうちに書いた行の並びで、どちらの側の行の中身と順も変わっていない。`run1.full.log` の `events` の差 2 もこの 1 件。
- 切替（`stable` 1）: 後に `apply(ShowSurface) target_id=TargetId(0) surface_id=1000 cache_hit=false` が 1 行多い（後の 88 行目）。台本の `\![bind,…]` の選び（乱数）が変えた面の組み直しの数で、切替の節目（`menu_selected`→ゴーストを降ろす→`ghost_switch_done`→`steady_talk_done`）の中身と順は同じ。
- 読み直し（`stable` 6）: ⑴ helper の標準エラーの行（`shiori-host32` `[helper] SHIORI 初期化の入口: loadu event="helper_stderr"`）が、前は 61 行目、後は 57 行目に出た。別のプロセス（32bit の helper）の標準エラーを中継した行で、本体の行との前後はプロセスの間で決まらない。⑵ `テキスト供給面を予約スロットへ装着した` と `apply(ShowSurface) … surface_id=1000` の 2 行の前後が入れ替わり、`ShowSurface` は前が 1 行多い。描画の側の並びと、読み直しの後の台詞（乱数・前の台詞は bind を 2 回替える）による組み直しの数。`reload_started`→`reload_done`→`ghost_switch_done`→`steady_talk_done` の中身と順は同じ。
- 投げ込み（`stable` 5）: 前だけに `[balloon-visibility] バルーンの可視状態が遷移した scope=1 trigger="clear"`・`scope=1 trigger="content"` と `apply(ShowSurface) target_id=TargetId(3) surface_id=0 cache_hit=true` があり、`scope=0 trigger="clear"` の位置が違う。投げ込みの時点では前後とも両方のバルーンが開いていた（読み直しの台詞の後に `scope=0`・`scope=1` の `trigger="content" visible=true` が出て、投げ込みまで閉じの行が無い。`run1.full.log` の前は 324・329 行目＝04:48:49.115・04:48:51.469、後は 326・331 行目＝08:32:58.280・08:33:00.384）。`OnInstallBegin` の台詞（`\p[1]` の「荷物が来たよ。」）の再生の始まり（前 04:49:04.328、後 08:33:11.560）から数えて、前は台詞の始まりの消去（`scope=1 trigger="clear"`・+30 ms）と相方のバルーンへの中身（`scope=1 trigger="content"`・+42 ms）が別のフレームに入ったので相方の遷移の行が 2 行出た。後は同じフレームに入ったので相方のバルーンの可視は変わらず、遷移の行は 0 行で、`scope=0 trigger="clear"`（+17 ms）だけが出た。どちらになるかはフレームの巡りで決まり、移したコードの外である（`areka-runtime` はこの仕様でコメントだけの変更）。`file_drop_received`→`OnInstallBegin`→`install_done`→`OnInstallComplete` の台詞→`steady_talk_done` の中身と順は同じ（`events` の差 0）。
- メニューの終了: 差 0。`app_exit origin=KanadeStopped(Quit) closed=4` も 1.2 と同じ。
- 煙の自動終了（`stable` 9・`events` 3）: ⑴ 前だけに、待つ間の随時のトーク 1 回（`steady_talk origin="OnSecondChange"`・`translate_reply`・`steady_talk_done` とその台詞・面とバルーンの行）がある。`OnSecondChange` に台本を返すかは emo2 の乱数なので、40 秒の間に来るかは走りによる。これが `events` の差 3。⑵ 後だけに wintf の `[WinApp::run] exit requested while windows remained open — destroying them before returning` が 1 行ある。煙の自動終了は tick の外の非同期の仕事から窓を閉じるので、`run()` が返る前に窓の片付け（reconcile）が回ったかどうかで出る行である（wintf の `runtime/mod.rs` の残った窓を壊す所の説明のとおり）。移す前の本体でも出ていた: 別の作業の木の `target\package\check-210911-logs\run.log`（2026-11-17・出所が `areka::smoke_exit`・`areka::app_exit` の頃）の 88 行目に同じ行がある。`app_exit origin=Smoke closed=4` → `ゴースト窓を despawn しました count=4` → `force_quit` → `ghost shutdown sequence completed` の中身と順は同じ。
- 1 回目の全体（`run1.full.log`）: 全体の記録には段の切れ目が無いので、切れ目の揺れはここに入らない。`stable` の差 16 は、上の起動・切替・読み直し・投げ込みで挙げたのと同じ行である。読み直しの所だけ、差を並べる側（difflib）の対応の取り方が段の比べと違い、テキスト供給面の行と `ShowSurface` の行の入れ替わりが 2 行多く数えられた。`events` は 108/108 で、差は起動の `boot_start` の 1 件だけ。

本当の差（乱数・壁時計・スレッドとプロセスの間の並びのどれでも説明できない、行の有無と順の違い）は 6 つの道のどれにも無く、0 である。

#### 実機で回したもの（8.8・8.9）

- OS のセッションの終わり（`WM_ENDSESSION`）は実機では起こせないので、決定論のテストで足りるとした（8.8）。受け持つテストは `crates/areka-session/src/session_end_deadline_tests.rs`（8 本）・`session_end_sync_send_tests.rs`（4 本）で、7.1 の全体テストで緑である。
- この仕様で実機を使ったのは、ログの出所を名前で読む実機テスト（7.4・8.6）と、この 6 つの道（1.2 と 7.5・8.7）だけである（8.9）。ほかは構造の証跡（テストの名前の一覧の比べ・警告・全体テスト・grep・決定論のテスト）で足りる。

#### 比べの手順 `target\session-crate-split\real\compare.py`

```python
"""Throwaway (session-crate-split 7.5): compare the six path records of real-before and real-after.

Per line: drop the timestamp, drop module-path targets (`a::b::c: `, the source names the move renames),
mask values that differ run to run (pids, ports, hwnds, entities, ids, durations).
Three views per path (and for the whole run1.full.log):
  all     every normalized line
  stable  without the classes driven by randomness or wall-clock timing (NOISE below)
  events  only the `event="..."` names, in order
Usage: python compare.py <before dir> <after dir>
"""
import difflib
import re
import sys

PATHS = ['boot', 'switch', 'reload', 'nar-drop', 'menu-quit', 'smoke-exit', 'run1.full']
RULES = [
    (re.compile(r'^\d{4}-\d\d-\d\dT[\d:.]+Z\s+'), ''),
    (re.compile(r'\b[a-z][a-z0-9_]*(?:::[a-z0-9_]+)+: '), ''),
    (re.compile(r'0x[0-9A-Fa-f]+'), '0x#'),
    (re.compile(r'\b\d+v\d+\b'), '#v#'),
    (re.compile(r'\d+(?:\.\d+)?(?:ms|µs|ns|s)\b'), '#t'),
    (re.compile(r'\b(pid|port|elapsed_ms|ms|took_ms|since_id|id|talk_id|frame_no|tick)=\d+'), r'\1=#'),
    (re.compile(r'127\.0\.0\.1:\d+'), '127.0.0.1:#'),
]
BS = chr(92)
NOISE = {
    # SERIKO animation and its ticker: random draws and wall-clock catch-up.
    'seriko': lambda l: 'actor{actor=seriko}' in l or 'actor{actor=loop-ticker}' in l or 'SERIKO ループ乱数 seed' in l,
    # The ghost's talk text: emo2 picks lines and \![bind,...] at random.
    'talk-text': lambda l: (BS + 'p[') in l or (BS + 's[') in l,
    # Balloon timeout bookkeeping: float clock values and which of two moments came first.
    'balloon-timeout': lambda l: '[balloon-visibility] タイムアウト計測' in l or '[balloon-visibility] 抑止が解けた' in l,
}
EVENT = re.compile(r'event="([a-z_]+)"')


def norm(line):
    for rx, to in RULES:
        line = rx.sub(to, line)
    return line


def diff(a, b):
    d = [x for x in difflib.unified_diff(a, b, n=0, lineterm='') if not x.startswith(('---', '+++'))]
    return d, sum(1 for x in d if x[:1] in '+-')


def main(before, after, show):
    for name in PATHS:
        a = [norm(l) for l in open(f'{before}/{name}.log', encoding='utf-8').read().splitlines()]
        b = [norm(l) for l in open(f'{after}/{name}.log', encoding='utf-8').read().splitlines()]
        sa = [l for l in a if not any(f(l) for f in NOISE.values())]
        sb = [l for l in b if not any(f(l) for f in NOISE.values())]
        ea = [m.group(1) for l in a for m in [EVENT.search(l)] if m]
        eb = [m.group(1) for l in b for m in [EVENT.search(l)] if m]
        counts = {k: (sum(map(f, a)), sum(map(f, b))) for k, f in NOISE.items()}
        views = {'all': diff(a, b), 'stable': diff(sa, sb), 'events': diff(ea, eb)}
        print(f'{name}: lines {len(a)}/{len(b)} stable {len(sa)}/{len(sb)} events {len(ea)}/{len(eb)} '
              + ' '.join(f'{v}={n}' for v, (_, n) in views.items())
              + ' noise ' + ' '.join(f'{k}={x}/{y}' for k, (x, y) in counts.items()))
        for v in show:
            for x in views[v][0]:
                print(f'  [{v}] {x[:240]}')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2], sys.argv[3:])
```

## 申し送り（7.6）

この spec では直さず、受け手へ渡す項目の一覧。どの項目にも受け手を書いた。

### 受け手: `audit-areka-app`

1. 古いパス（コメント・触らない約束の crate）
   - `crates/areka-placement/src/diag.rs:46`・`persist.rs:330`・`resolver.rs:97`・`resolver_resolve_balloon_tests.rs:123`: 古い `session/boot_seam.rs` を名指す（移した先は `crates/areka-session/src/boot_seam.rs`）。
   - `crates/areka-placement/examples/window-placement.rs:37`: 古い `crates/areka/src/smoke_exit.rs` を名指す（移した先は `crates/areka-session/src/smoke_exit.rs`）。
   - 6.4 の数えで除いた `crates/pilot/examples/pilot-windows-rs-next/REWRITE_TABLE.md:66` は過去の書き換えの記録の引用で、直す必要は無い。
2. 開けた項目の一覧（4.2・全件は `target/session-crate-split/4.2-surface.txt`・P＝本番の本体も使う・T＝本体のテストだけ）
   - 本体のテストだけが使う `#[cfg(any(test, feature = "test-support"))] pub`: `GhostSession::{for_test, with_memory_publisher, with_seriko_sink, dispatcher}`・`roots::single_env`・`shell_balloon_switch::SwapFinish` の出し直し・`log_history::hold`・`ghost_switch_test_support`（モジュールと中の項目）・`ReadmeWiring::path`・`ghost_session::shell_balloon_switch_session_lap_test_support`（新設）・`reload::test_support`・`ghost_session_scope_{rect,scale}::test_support`・`crate_source_test_support`・`BodySystems::of`（4.4。それ以外では `pub(crate)`。定義が条件ごとに 2 つある）。
   - 設計の方針（本番の公開面に中の関数を出さない）と違い、条件で切り替えず素の `pub` にした項目: `GhostSession::{seriko, loop_ticker}`・`PreparedWindows::spawn`・`reload::drain`・`write_failure::drain_system`（集めた層の本番のコードも使うため）・`boot_seam::insert_persist_wiring`（本体の `session_tests/` の 3 本が使う）。ほかは 4.2 の一覧の T の行（`boot_resolve::{read_session_mark, clear_session_mark, keep_session_mark, …}`・`reload::{ReloadInFlight, ReloadStage, register, …}` ほか）と、連れて開けた型（`private_interfaces`・4.2 の「連れて開けた型」）。
   - 使われなくなった `pub(crate)` は消していない。公開面を絞る・素の `pub` を条件つきへ戻せるかは `audit-areka-app` の判断。
3. `world.insert_resource(crate::BODY_SYSTEMS);` の 1 行を足した本体のテスト（表明は変えていない・8 本: 2.5 の 5 本・4.4 の 3 本）
   - 2.5: `install::desk::tests::resend_goes_at_once_when_steady_arrived_since_the_last_send`・`install::desk::tests::a_cancelled_overwrite_switch_goes_back_to_held_and_retries_on_the_next_steady`・`update::desk::resolve_tests::steady_notice_sends_one_homeurl_query_each_time`・`session::reload::tests::refusals_in_order_leave_one_record_each`・`session::shell_balloon_switch::tests::refusals_in_order_leave_one_warning_each_and_send_nothing`（後ろ 2 本は 3.4 で本体の `session_tests/` へ移った）。
   - 4.4（D13）でさらに次の 3 本に足した（口を据えずに口の道を通り `body_systems_missing` の記録が増えて赤になった）。合わせて 8 本: `session_tests::reload_tests::missing_ghost_or_kanade_is_no_context`・`session_tests::shell_balloon_switch_tests::acceptance_starts_one_wait_marked_only_for_menu_and_raise_event_shells`・`session_tests::shell_balloon_switch_tests::send_failure_leaves_one_error_and_no_marker`。
   - 今後、口を据えずに口の道を通るテストを足すと同じ赤になる。
4. `crates/areka-session/src/smoke_exit.rs` の `//!`: 「`main.rs` の 1,000 行の上限のため外へ出した」は、集めた層へ移った後は古い（2.3）。
5. 理由の文が古くなる `#[allow]` は 0 件。`crates/areka-session` の `#[allow]` は 3 つ（`boot_resolve_option_tests.rs:41` の `clippy::too_many_arguments`・`ghost_switch_test_support.rs:212` の `dead_code`〔理由「読まず、捨てると木が消える寿命だけを持つ」〕・`shell_balloon_switch_session_lap_tests.rs:112` の `clippy::type_complexity`）、本体の `session_tests/` は 0 件で、どれも理由の文がパスや本体の項目を名指さず、移した後も合っている。
6. 作業の途中で見つけた不具合・古い説明（この spec では直していない）
   - `crates/areka-session/src/crate_source_test_support.rs` の `//!`: 「本体と `areka-runtime` のテストが…」「本体が依存する crate」「本体自身のファイルは…」は、この支えが `areka-session` の中に在る今は主語が合わない。4.1 のレビューで見つけた。
   - `crates/areka-session/src/roots/user_root.rs:199` の doc `resolve_boot_from` へ渡す口: `resolve_boot_from` は 3.4 で本体の `session_tests/mod.rs` へ移ったので、集めた層から見ると指す先が無い（`roots/` は `audit-areka-install-update` の持ち物でもあるので、同じ項目をそちらにも書いた）。
   - `add_rpost_shell` の 9 行は 2 か所の写し（集めた層の `shell_balloon_switch_session_lap_tests.rs` と本体の `session_tests/` のテストのファイル）。`test-support` に `sample-ghost-kit`（番人）を入れられないため。1 つに束ねるかは `audit-areka-app` の判断。
   - `crates/areka-session/Cargo.toml` の設計との違い: `areka-update` を本番の依存から外した（集めた層は決まりの文字列 `"areka_update::winhttp"` ほかを持つだけ・4.1）。`areka-emo-present` は本番のコードが名指さず、`test-support` で立つ `ghost_session_scope_scale_test_support.rs` だけが使う（runtime が本番で既に引くので出荷の閉包は増えない・4.1）。`windows` に `Win32_System_Registry` を crate の側で上乗せしている（`readme/os_port.rs` の `ShellExecuteExW`）。
   - `crates/areka-session/build.rs`・`tests.manifest`（設計に無い・4.4）: lib のテストの exe に comctl32 の版 6 の申告が無く `0xc0000139` で落ちたため足した。MSVC のときだけ `/MANIFEST:EMBED`・`/MANIFESTINPUT` を出す。`tests.manifest` は `crates/areka/areka.manifest` と同じ申告の写しで、片方だけ直すと食い違う。
   - `crates/areka-session` の clippy には、移したコードに前からある形の指摘（`type_complexity` 33・`collapsible_if` 19 ほか）が残る（4.2・4.4）。
   - 前からある文書の食い違い（6.2・この spec では直していない）: `doc/COMPAT_ARCHITECTURE.md` の `fault_of_returns_the_fault_only_for_kanade_stopped_fault`（`app_exit_tests.rs`）・`matching_is_case_sensitive`（`ghost_switch_tests.rs`）は `main` の `crates/` にも無い名。`doc/ukadoc-coverage/briefing-assets.md:262` の実在しない `crates/areka-parsers/src/package/resolve.rs` ほか約 12 件。

### 受け手: `audit-areka-install-update`

1. `NO_BODY` の受け口と、本体が今まで使っていた「窓口が無い」ときの受け口の差（設計の危うさ 3）
   - `NO_BODY`（`crates/areka-session/src/body_systems.rs`）の `cue_sinks` は `areka_ghost::sink::DiscardSink` を 2 つ返す。
   - 本体が窓口の無いときに使う受け口は、受信端の無い送出端を持つ `InstallCueSink`。`url` の腕で取得のスレッドを起こすかが違う（`DiscardSink` は起こさない）。
   - `NO_BODY` を通るのは結線の誤りのとき（`body_systems_missing` の `error!` が出る）と、集めた層のテスト（`SwitchRig::new` が `NO_BODY` を渡す）だけで、本番の道では起こらない。振る舞いの変化には数えていない。
   - 本体の `session_tests/body_systems_tests.rs` の `missing_body_systems_logs_and_answers_like_no_desk` が、口の無い World の答え（`error!` 1 つ・`update_busy`・`install_busy` は偽・`cue_sinks` は 2 つ）を固定している。
2. `crates/areka-session/src/roots/user_root.rs:199` の doc が指す `resolve_boot_from` は本体の `session_tests/` へ移った（`audit-areka-app` の 6 と同じ項目・`roots/` はこの受け手の持ち物）。

### 受け手: 棚卸のセッション

C25 の行の「触るファイル」に無く、この spec で足したファイル。兄弟 2 本（`test-name-work-tags`・`dola-cue-test-builder`）の触るファイルとは、`doc/COMPAT_ARCHITECTURE.md` §8 のパスの行（設計が許した重なり）と `.kiro/specs/audit-areka-app/brief.md` の 2 件を除いて重ならない（着手のとき・1.1）。

1. 道具: `tools/package.ps1`（3 か所・注釈）・`tools/perf/judge-perf.py`（1 か所）・`tools/perf/invoke-perf-run.ps1`（29 行の説明）・`tools/load-flake.ps1`（`areka-session-lib` を足した・5.2）
2. 文書: `doc/ukadoc-coverage/`（`briefing-assets.md`・`ledger/{sakura-script,property,shiori,assets}.toml`・作り直した `report/`）・`doc/ideas/novel-redesign-2026-11-20/` の 8 ファイル・`doc/COMPAT_ARCHITECTURE.md`・`doc/testing.md`・`doc/ssp-mcp/log-convention.md`・`doc/pasta-real-run.md`・`THIRD-PARTY-NOTICES.md`（`areka-session` の 1 行・7.2）
3. steering: `.kiro/steering/structure.md`・`tech.md`・`logging.md`・`product.md`
4. 兄弟の brief: `.kiro/specs/audit-areka-app/brief.md`・`.kiro/specs/audit-areka-install-update/brief.md`・`.kiro/specs/audit-emo2-boot-switch/brief.md`（3 本目）。いずれも「受け持つファイル」の行の書き換え。`audit-areka-app` の brief は `test-name-work-tags` も 1 項目を足すので、後から着地する側が両方を残して解く。
5. 新しい crate と本体: `crates/areka-session/`（`Cargo.toml`・`build.rs`・`tests.manifest`・`src/crash_report_tests.rs`・`src/body_systems.rs`・`src/shell_balloon_switch_session_lap_test_support.rs`・`src/reload_test_support.rs`・`src/ghost_session_scope_{rect,scale}_test_support.rs`）・本体の `crates/areka/src/session_tests/`（新設）・`crates/areka/src/sample_test_support.rs`・`crates/areka/src/install/startup.rs`・`crates/areka/Cargo.toml`・`Cargo.lock`
6. 実機の道具: `crates/real-run-kit/`（`launch.rs`・`launch_tests.rs`・`close_tests.rs`・`bin/real-run-fake-areka.rs`）・`crates/areka/tests/emo2_mcp_real_run.rs`（5.1・ログの出所の名前）
7. コメントだけの書き換え: `crates/areka-runtime/src/`（`lib.rs`・`strict_judges.rs`・`talk_clock.rs`・`user_break_cue.rs`・`session_hooks.rs`・`skin_switch.rs`・`placement_tests/transition_judge_offset_signoff_tests.rs`）・`crates/areka-kanade/src/schedule/`（`script_log.rs`・`events.rs`）
8. 約束の例外（開発者の裁定・2026-11-22）: 本体の `crates/areka/src/placement_tests/` の 4 ファイル（`measure_bake_count_tests.rs`・`measure_template_tests.rs`・`zorder_group_branch_coverage_tests.rs`・`zorder_inversion_branch_coverage_tests.rs`）の読み口の import 1 行ずつ。`crates/areka-placement/`・`crates/dola/`・`crates/areka-ghost/`・`crates/areka-seriko/` の差分は 0。
