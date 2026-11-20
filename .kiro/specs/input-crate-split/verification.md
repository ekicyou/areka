# 検証の記録（input-crate-split）

## 着手のときの証跡

### テストの一覧（1.1）

- `cargo test --workspace --no-fail-fast -- --list` を基点 `80028ea01` で取り、`target/input-crate-split/tests-before.txt` に残した。`: test$` の行は 14389 行。

### 実機の強制退避（1.2・D13）

- 2026-11-20 14:27 UTC、排他テストの机を取り、開発者の SendInput の許しの下で 1 回取った。記録は `target/input-crate-split/escape-before.stderr.log`（166 行）。
- areka の tracing は標準出力に出るので、この記録は名前のままで標準出力と標準エラーを行ごとにまとめたもの。
- `log_stay_on_top_discard` の行は「常に手前」がオンのときだけ出るため、強制退避の前に MCP の `sakurascript` で `\![set,windowstate,stayontop]\e` を送る。7.3 も同じ台本で取る。
- `run()` の復帰を示す専用の行は無い。`main.rs` で `app.run()` の後に呼ぶ `GhostSession::shutdown` の最初の行（`seriko: loop ticker を Close しました`）を目印にする。
- 順（行番号は記録のもの）:
  - 153 `areka::input_events` … `event="mouse_escape_close" wired=true shift=true`
  - 154 `areka_runtime::frame::window_state` `[window_state] 常に手前のまま降ろした（状態は捨てる） reason="exit"`
  - 156 `areka::app_exit` `[quit_app] … event="app_exit" origin=Escape closed=4`
  - 160 `areka::ghost_session` `seriko: loop ticker を Close しました`（`run()` の復帰の目印）
  - 165 `ghost-shutdown` `ghost shutdown sequence completed` → 終了コード 0（`origin=Smoke` は無い）

#### 手順 `target\input-crate-split\escape\escape.ps1`

```powershell
<#
  Throwaway (input-crate-split D13): start areka with emo2 (absolute paths), turn stay-on-top on
  over MCP so quit_app writes its log_stay_on_top_discard line, then put one Ctrl+Shift+left
  double-click on the character window with SendInput and wait for areka to exit.
  Needs the exclusive-test desk and the developer's SendInput permission.
  Usage: pwsh -NoProfile -File <this> -Out <stderr log path>
#>
param([Parameter(Mandatory)] [string]$Out)
$ErrorActionPreference = 'Stop'

$Here = $PSScriptRoot
$Wt = (Resolve-Path (Join-Path $Here '..\..\..')).Path
$Exe = Join-Path $Wt 'target\debug\areka.exe'
$paths = @{}
foreach ($line in (& (Join-Path $Wt 'target\debug\nar-sample-path.exe') emo2)) {
    $k, $v = $line -split '=', 2; $paths[$k] = $v
}
if ($LASTEXITCODE -ne 0) { throw "nar-sample-path failed ($LASTEXITCODE)" }
$Ghost = $paths['folder']; $Balloon = $paths['balloon.emo2-kakukaku']

$Work = Join-Path $Here 'run'
if (Test-Path $Work) { Remove-Item -Recurse -Force $Work }
$Root = New-Item -ItemType Directory -Force (Join-Path $Work 'root')
$Prof = New-Item -ItemType Directory -Force (Join-Path $Work 'profile')
$l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $l.Start()
$Port = $l.LocalEndpoint.Port; $l.Stop()

Remove-Item Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
$env:AREKA_ROOT = $Root.FullName
$env:AREKA_PROFILE_DIR = $Prof.FullName
$env:AREKA_MCP_PORT = "$Port"
$env:AREKA_NO_ALERT = '1'
$env:NO_COLOR = '1'
$env:PASTA_DEBUG = '0'
# Safety exit only (origin=Smoke would mean the escape did not land).
$env:AREKA_APP_SMOKE_EXIT_MS = '240000'
$env:RUST_LOG = 'info,kanade=debug,areka::input_events=debug,areka::app_exit=debug,areka_runtime::frame::window_state=debug,wintf=info,wintf::ecs::clickthrough=debug'

$OutFull = [System.IO.Path]::GetFullPath($Out)
# areka's tracing goes to stdout; both streams go into the one -Out file, line by line.
Add-Type -TypeDefinition @"
using System.Diagnostics;
using System.IO;
using System.Text;
public static class AkMergedRun {
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
}
"@
$p = [AkMergedRun]::Start($Exe, "`"$Ghost`" `"$Balloon`"", $OutFull)
Write-Output "started pid=$($p.Id) port=$Port"

function Read-Log {
    $fs = [System.IO.File]::Open($OutFull, 'Open', 'Read', 'ReadWrite')
    try { (New-Object System.IO.StreamReader($fs, [System.Text.Encoding]::UTF8)).ReadToEnd() } finally { $fs.Dispose() }
}
function Wait-Log([string]$Needle, [int]$Seconds) {
    $end = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $end) {
        if ($p.HasExited) { throw "areka exited (code $($p.ExitCode)) before: $Needle" }
        if ((Read-Log).Contains($Needle)) { return }
        Start-Sleep -Milliseconds 300
    }
    throw "timeout waiting for: $Needle"
}

try {
    Wait-Log 'wrap=BudouxWordWrap' 60
    # The ghost refuses scripts while the boot greeting plays (about 17 s for emo2).
    Wait-Log 'event="steady_talk_done"' 60
    Start-Sleep -Seconds 1

    $body = @{ jsonrpc = '2.0'; id = 1; method = 'tools/call'
        params = @{ name = 'sakurascript'; arguments = @{ script = '\![set,windowstate,stayontop]\e' } } } | ConvertTo-Json -Depth 5
    $accepted = $false
    for ($i = 0; $i -lt 15; $i++) {
        $r = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/api/mcp/v1" -Method Post -Body $body `
            -ContentType 'application/json' -Headers @{ Accept = 'application/json, text/event-stream' }
        Write-Output "mcp attempt=$i reply=$($r.Content)"
        if (-not $r.Content.Contains('cannot accept a script now')) { $accepted = $true; break }
        Start-Sleep -Seconds 1
    }
    if (-not $accepted) { throw 'MCP sakurascript refused 15 times' }
    Wait-Log '[window_state] 常に手前の状態が変わった' 10
    Start-Sleep -Milliseconds 800

    & (Join-Path $Here 'send-escape.ps1') -ProcessId $p.Id

    if (-not $p.WaitForExit(60000)) {
        throw "areka did not exit within 60s after the escape (mouse_escape_close in log: $((Read-Log).Contains('mouse_escape_close')))"
    }
    Write-Output "exit code=$($p.ExitCode)"
}
finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force; Write-Output "killed own pid=$($p.Id)" }
}
```

#### 手順 `target\input-crate-splitscape\send-escape.ps1`

```powershell
<#
  Throwaway (input-crate-split D13): one Ctrl+Shift+left double-click on the character window of
  the given areka process, by SendInput. Releases everything in finally.
  The point is checked with WindowFromPoint + GetWindowThreadProcessId before pressing.
#>
param([Parameter(Mandatory)] [int]$ProcessId)
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public class AkEscapeW32 {
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll", SetLastError=true)] public static extern uint SendInput(uint count, [In] INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
    [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
    public delegate bool EnumProc(IntPtr h, IntPtr l);

    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT {
        public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo;
    }
    [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT {
        public ushort wVk, wScan; public uint dwFlags, time; public IntPtr dwExtraInfo;
    }
    // x64 INPUT: type at 0, union at 8, 40 bytes.
    [StructLayout(LayoutKind.Explicit, Size=40)] public struct INPUT {
        [FieldOffset(0)] public uint type;
        [FieldOffset(8)] public MOUSEINPUT mi;
        [FieldOffset(8)] public KEYBDINPUT ki;
    }
    public static int InputSize() { return Marshal.SizeOf(typeof(INPUT)); }

    // Character windows: visible top-level windows of the process that own another of its windows (the balloons).
    public static List<IntPtr> CharactersOf(uint pid) {
        var mine = new List<IntPtr>();
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid) mine.Add(h);
            return true;
        }, IntPtr.Zero);
        var owners = new HashSet<IntPtr>();
        foreach (var h in mine) owners.Add(GetWindow(h, 4));
        var chars = new List<IntPtr>();
        foreach (var h in mine) if (owners.Contains(h) && IsWindowVisible(h) && GetWindow(h, 4) == IntPtr.Zero) chars.Add(h);
        return chars;
    }
}
"@

# PER_MONITOR_AWARE_V2: GetWindowRect and the SendInput targets are physical pixels.
$null = [AkEscapeW32]::SetThreadDpiAwarenessContext([IntPtr](-4))
if ([AkEscapeW32]::GetForegroundWindow() -eq [IntPtr]::Zero) { throw 'no foreground window (screen saver or locked desktop?)' }

$vx = [AkEscapeW32]::GetSystemMetrics(76); $vy = [AkEscapeW32]::GetSystemMetrics(77)
$vw = [AkEscapeW32]::GetSystemMetrics(78); $vh = [AkEscapeW32]::GetSystemMetrics(79)

function New-Key([uint16]$Vk, [bool]$Up) {
    $i = New-Object AkEscapeW32+INPUT; $i.type = 1
    $k = New-Object AkEscapeW32+KEYBDINPUT; $k.wVk = $Vk; if ($Up) { $k.dwFlags = 0x0002 }
    $i.ki = $k; $i
}
function New-Mouse([uint32]$Flags) {
    $i = New-Object AkEscapeW32+INPUT; $i.type = 0
    $m = New-Object AkEscapeW32+MOUSEINPUT
    $m.dx = [int][math]::Floor((([long]($target.X - $vx)) * 65536 + $vw - 1) / $vw)
    $m.dy = [int][math]::Floor((([long]($target.Y - $vy)) * 65536 + $vh - 1) / $vh)
    $m.dwFlags = $Flags -bor 0x0001 -bor 0x8000 -bor 0x4000  # MOVE | ABSOLUTE | VIRTUALDESK
    $i.mi = $m; $i
}
function Send-One($Item) {
    if ([AkEscapeW32]::SendInput(1, @($Item), [AkEscapeW32]::InputSize()) -ne 1) {
        throw "SendInput refused (lasterr=$([System.Runtime.InteropServices.Marshal]::GetLastWin32Error()))"
    }
    Start-Sleep -Milliseconds 40
}

# Same point search as crates/areka/tests/stay_on_top_real_run.rs `hit_point`: areka keeps its windows
# click-through where the cursor is not, so put the cursor on each point (9x9 grid, nearest the centre
# first), wait 300 ms, then check the window under it.
$target = $null
foreach ($h in [AkEscapeW32]::CharactersOf([uint32]$ProcessId)) {
    $r = New-Object AkEscapeW32+RECT; $null = [AkEscapeW32]::GetWindowRect($h, [ref]$r)
    $w = $r.R - $r.L; $hh = $r.B - $r.T; $cx = $r.L + [int]($w / 2); $cy = $r.T + [int]($hh / 2)
    $pts = foreach ($i in 1..9) { foreach ($j in 1..9) { ,@(($r.L + [int][math]::Floor($w * $i / 10)), ($r.T + [int][math]::Floor($hh * $j / 10))) } }
    $pts = $pts | Sort-Object { [math]::Pow($_[0] - $cx, 2) + [math]::Pow($_[1] - $cy, 2) }
    foreach ($xy in $pts) {
        $target = New-Object AkEscapeW32+POINT; $target.X = $xy[0]; $target.Y = $xy[1]
        Send-One (New-Mouse 0)
        Start-Sleep -Milliseconds 300
        if ([AkEscapeW32]::WindowFromPoint($target) -eq $h) { break }
        $target = $null
    }
    if ($target) { Write-Output "target hwnd=0x$($h.ToString('X')) rect=$($r.L),$($r.T),$($r.R),$($r.B) at x=$($target.X) y=$($target.Y)"; break }
}
if (-not $target) { throw "no point on a character window of pid $ProcessId" }

$CTRL = 0x11; $SHIFT = 0x10; $LEFTDOWN = 0x0002; $LEFTUP = 0x0004
try {
    Send-One (New-Mouse 0)
    Start-Sleep -Milliseconds 200
    Send-One (New-Key $CTRL $false)
    Send-One (New-Key $SHIFT $false)
    Send-One (New-Mouse $LEFTDOWN); Send-One (New-Mouse $LEFTUP)
    Send-One (New-Mouse $LEFTDOWN); Send-One (New-Mouse $LEFTUP)
    Write-Output 'sent Ctrl+Shift+left double-click'
}
finally {
    foreach ($item in (New-Mouse $LEFTUP), (New-Key $SHIFT $true), (New-Key $CTRL $true)) {
        $null = [AkEscapeW32]::SendInput(1, @($item), [AkEscapeW32]::InputSize())
    }
}
```

## 古いパスと古い出所の名前の数え（6.4）

2026-11-21、範囲 `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で `git grep` した。

| 数えたもの | 範囲 | 当たり |
|---|---|---|
| 古いパス `crates/areka/src/input_events/` | 範囲の全部 | 0 件 |
| `src/` を付けない書き方 `input_events/<名>.rs` | 範囲の全部 | 0 件（除いた所の 3 か所だけ） |
| モジュールの形 `input_events::<名>` | `doc/` と本体の外の crate・`tools/` | 0 件（除いた所の 1 か所だけ） |
| 古い出所の名前 `areka::input_events` | 範囲の全部 | 0 件 |
| 古いカーソルの絵の置き場 `crates/areka/cursor` | 範囲の全部 | 0 件 |

- 本体の中の呼び手 `crate::input_events::…`（出し直し `use areka_input as input_events;` で通るもの）は当たりにしない。
- 除いた所:
  - `crates/areka-runtime/src/frame/wiring.rs` の説明 3 か所（295・319・326 行。同じウェーブの約束で触らない。`audit-emo2-boot-frame` はまだ直していない）
  - `doc/roadmap-overview-2026-10-16.html`（日付つきの写し）
  - 範囲の外: `roadmap-history.md`・`.kiro/specs/completed/`・`roadmap.md`（新旧のパスの対応表を持つ）

## 構造の証跡（7.1）

### テストの名前の多重集合（8.2・8.3）

- 完了のときの一覧 `target/input-crate-split/tests-after.txt`（`cargo test --workspace --no-fail-fast -- --list`）の `: test$` の行は 14392 行。着手のときは 14389 行。
- 名前の末尾（最後の `::` から後ろ）の多重集合の差は増えた 3 件だけで、減った名前は 0 件:

| 増えた名前 | 置き場 | 理由 |
|---|---|---|
| `crash_filter_is_installed_before_the_tests_start` | `crates/areka-input/src/crash_report_tests.rs` | 新しい crate のテストの実行ファイルに落ちの報告を据えた確かめ（同名は本体・runtime ほかにも在る） |
| `escape_without_hooks_logs_and_still_requests_exit` | `crates/areka-input/src/hooks_tests.rs` | 口が無いときの倒れ方 |
| `register_systems_installs_the_escape_hook` | `crates/areka/src/app_exit_escape_tests.rs` | システムの組み立てを通す口の据え付けの道 |

- 移したテストは名前を変えていないので差に出ない。

### 警告・触らない約束・依存・履歴

- `cargo check --workspace --all-targets` の警告は `shiori-abi` の `unused dependency windows`（`crates/shiori-abi/Cargo.toml:16`）の 1 件だけ。この差分は `shiori-abi` に触っておらず、main でも同じに出る（範囲の外・完了の棚卸で起票する）。この spec が足した警告は 0 件。
- 触らない約束のファイル（`crates/areka-runtime/src/frame*`・`crates/areka/src/runtime_tests/`・`session/runtime_boot.rs`・`session/runtime_boot_tests.rs`・`session/ghost_quit*`）の `git diff --stat main...HEAD` は空。
- `cargo tree -p areka-input -e normal` に `areka` と番人の crate（`log-capture-kit`・`real-run-kit`・`temp-path-kit`）は出ない。
- `Cargo.lock` の差分は `areka-input` の 1 項目と、本体の依存の 1 行（`"areka-input"`）だけ。
- `git log --follow` で移したファイルの履歴が追える（例 `crates/areka-input/src/lib.rs` は `areka-P0-input-events` の頃まで、`crates/areka-input/cursor/hand.cur` は `shell-mouse-cursor` まで）。

### テストの置き場（5.1）

| テスト | 置き場 | 理由 |
|---|---|---|
| 移したファイルの中のテスト（下の 3 本を除く） | `areka-input` | 本体を引かない（`crate::` の本体の項目 0） |
| 強制退避の 3 本と据え付けの確かめ 1 本 | 本体 `app_exit_escape_tests.rs` | 本物の `quit_app` を見る |
| `shortcut_tests.rs` | 本体 `menu/` | `crate::ghost_session`・`crate::menu` を引く |
| `file_drop_tests.rs`・`file_drop_mime_tests.rs`・`file_drop_wiring_tests.rs` | 本体 src 直下 | `crate::ghost_session`・`crate::install`・`crate::crate_source_test_support` を引く |
| `spawn_assembly_tests.rs`（旧 `placement_tests/` の下） | `areka-input` | 配置と入力だけを使う |
| `hooks_tests.rs`・`crash_report_tests.rs` | `areka-input` | 口が無いときの倒れ方・落ちの報告の据え付け |
| 本体の `ghost_session_*_tests.rs`・`shell_balloon_switch_session_*_tests.rs`・`app_exit_tests.rs`・`menu/*_tests.rs`・`runtime_tests/` の 8 本・`session/runtime_boot_tests.rs` | 本体（変えない） | 出し直しで通る |

- どの行も本体の部品を偽物に替えていない。
