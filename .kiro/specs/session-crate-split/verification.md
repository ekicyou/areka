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
