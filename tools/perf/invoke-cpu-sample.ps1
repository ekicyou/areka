#Requires -Version 7.0
<#
================================================================================
invoke-cpu-sample.ps1 — CPU サンプリング（呼出スタック付き）の採取・停止・記号解決を 1 コマンドで
  spec: areka-P0-draw-load-parity（要件 2.4 / 2.11 / 8.6・design「計測の道具（tools/perf/）
        → C8 invoke-cpu-sample.ps1 → Batch / Job Contract」）

何をするか（4 つのモードのどれか 1 つを指定する）:
  -Probe     … 採れるかどうかだけを確かめる。昇格しているか・xperf.exe が在るか・
               実際に 5 秒だけ採って止められるかを見て、1 行で報告する（必ず exit 0）:
                 available=<真偽> reason=<語> elevated=<真偽> xperf_path=<パス|->
               xperf_path は空白や括弧を含み得るので行の最後に置く（perf-loop が読む）
  -Start     … サンプリングと呼出スタックの採取を始める（管理者権限が要る）
  -Stop      … 採取を止めてトレースをまとめ（まとめが通ったら元の ETL を消す）、記号を解決して
               テキストの dump を書き出す
  -Cancel    … 採取を止めて ETL を消す（記号は解かない。採取を始めた道具が失敗の道で使う）
  -SelfTest  … 道具そのものの較正。同梱の dump 断片で「areka.exe! のフレームを数える」
               関門が正しく数えられるか（合格側）と、フレームが無い入力で 0 を返すか
               （不合格側）を確かめ、続けて -Probe と同じ確認を行う（採取は試さない）

「関門」について（要件 2.11 ＝ 黙って続けない）:
  記号が解決できていない dump は、見た目は正常でも中身が空である。そこで -Stop は
  書き出した dump を必ず数え直し、areka.exe! のフレームが 0 なら計測失敗（exit 4）で
  止める。-SelfTest はこの数え方そのものを、既知の合格側・不合格側の両方で毎回試す。

昇格していない環境について:
  段③（関数別の帰属）は管理者権限が無いと採れない。これは「計測の失敗」ではなく
  「能力の不足」なので exit 5（UNAVAILABLE）で区別する。呼び出し側は段③だけを省いて
  段①②④で続行してよい（design「Error Categories and Responses」）。

--------------------------------------------------------------------------------
較正値・調整値の一覧（変更する場合はここだけを書き換える）
--------------------------------------------------------------------------------
  SCRIPT_VERSION            本スクリプトの版
  XPERF_DEFAULT_PATH        PATH に xperf が無いときに見に行く既定の位置
  PROF_INTERVAL_UNITS       サンプリング周期（-SetProfInt の単位。1221 ≒ 8kHz）
  PROBE_CAPTURE_SEC         -Probe が実際に採ってみる秒数
  XPERF_KERNEL_FLAGS        採取するカーネルプロバイダ
  XPERF_STACKWALK           呼出スタックを採る対象イベント
  BUFFER_SIZE_KB / MAX_BUFFERS  採取バッファ
  SYMBOL_SERVER_URL         公開シンボルサーバ
  DEFAULT_SYMCACHE_SUBDIR   記号キャッシュの既定位置（LOCALAPPDATA 配下）
  FIXTURE_DUMP_RELPATH      -SelfTest が読む同梱 dump 断片
  FIXTURE_EXPECT_*          同梱 dump 断片の既知の内訳（-SelfTest がこの値と厳密に突き合わせる。
                            fixture を実採取で差し替えたら、この値も同時に書き換える）
  TARGET_MODULE             関門が数える対象モジュール名

記号（要件 8.6 ＝ Cargo.toml 非接触）:
  release ビルドの記号は、ビルド時の環境変数
      $env:CARGO_PROFILE_RELEASE_DEBUG = 'line-tables-only'
  で付与する。Cargo.toml には触れない。なお本ワークスペースの release は
  lto=true・opt-level='z' であり、インライン化でスタックが浅くなる（呼び元が
  1 段に潰れて見える）。詳しくは tools/perf/README.md を参照。

終了コード（design「C5 の終了コード体系」に合わせる）:
  0 … 正常終了（-Probe は採れない場合も 0。可否は標準出力の 1 行で報告する）
  1 … 実行の失敗（xperf が非ゼロを返した・自己較正が赤）
  3 … 引数・前提の不正
  4 … 計測失敗（dump が空・記号解決ゼロ）
  5 … 能力不足 ＝ UNAVAILABLE（昇格していない・xperf.exe が無い）

標準出力の末尾には必ず次の 1 行を出す（背景実行でも会話へ届く形）:
  INVOKE-CPU-SAMPLE RESULT <mode> code=<n> <キー=値 …>

使い方:
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -Probe
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -SelfTest
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -Start -Etl C:\出力\cpu.etl
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -Cancel -Etl C:\出力\cpu.etl
  pwsh -File tools/perf/invoke-cpu-sample.ps1 -Stop  -Etl C:\出力\cpu.etl `
      -Out C:\出力\dump.txt -PdbDir C:\repo\target\release
================================================================================
#>

[CmdletBinding(DefaultParameterSetName = 'None')]
param(
    # 採れるかどうかだけを確かめる（必ず exit 0・1 行で報告）
    [Parameter(ParameterSetName = 'Probe', Mandatory = $true)]
    [switch]$Probe,

    # 採取を始める
    [Parameter(ParameterSetName = 'Start', Mandatory = $true)]
    [switch]$Start,

    # 採取を止めて記号解決まで行う
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [switch]$Stop,

    # 採取を止めて ETL を消す
    [Parameter(ParameterSetName = 'Cancel', Mandatory = $true)]
    [switch]$Cancel,

    # 道具そのものの較正
    [Parameter(ParameterSetName = 'SelfTest', Mandatory = $true)]
    [switch]$SelfTest,

    # 採取先／読み取り元のトレースファイル
    [Parameter(ParameterSetName = 'Start', Mandatory = $true)]
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [Parameter(ParameterSetName = 'Cancel', Mandatory = $true)]
    [string]$Etl,

    # 書き出すテキスト dump
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [string]$Out,

    # 実行体の PDB が在る場所（例: target\release）
    [Parameter(ParameterSetName = 'Stop', Mandatory = $true)]
    [string]$PdbDir,

    # 記号キャッシュ（省略時は LOCALAPPDATA\areka-diag\symcache）
    [Parameter(ParameterSetName = 'Stop')]
    [string]$SymCache,

    # サンプリング周期（-SetProfInt の単位）。0 なら較正値を使う
    [Parameter(ParameterSetName = 'Start')]
    [int]$ProfIntervalUnits = 0
)
# shortcut: 引数なしは None の組で 3 と RESULT 行にするが、モードの必須の引数の欠け（-Stop で -Out 無し）と
# 2 つのモードの同時指定は PowerShell の束ねの段で RESULT 行の無い 1 になる（script の中から捕まえる口が無い）。
# 呼び手（perf-loop・ランナー）が引数を欠く形で呼ぶようになったら、必須を外して自分で確かめる形にする

# 想定外の失敗は理由の 1 行と RESULT 行で 1
trap {
    # 共通部品より前の失敗（読み込めない）でも元の原因と RESULT 行で 1
    if (-not (Get-Command -Name 'Stop-Run' -CommandType Function -ErrorAction Ignore)) {
        [Console]::Error.WriteLine("error: unexpected failure before the shared helpers loaded: $($_.Exception.Message -replace '\s*\r?\n\s*', ' ')")
        [Console]::Out.WriteLine("INVOKE-CPU-SAMPLE RESULT $($PSCmdlet.ParameterSetName.ToLowerInvariant()) code=1")
        exit 1
    }
    Stop-Run -Mode $PSCmdlet.ParameterSetName.ToLowerInvariant() -Code $EXIT_RUN_FAILED -Message (Format-UnexpectedError -ErrorRecord $_)
}

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'perf-tool-base.ps1')

# ---------------------------------------------------------------- 較正値 ----
$SCRIPT_VERSION           = '1.1.0'
$XPERF_DEFAULT_PATH       = 'C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe'
$PROF_INTERVAL_UNITS      = 1221
$PROBE_CAPTURE_SEC        = 5
$XPERF_KERNEL_FLAGS       = 'PROC_THREAD+LOADER+PROFILE'
$XPERF_STACKWALK          = 'Profile'
$BUFFER_SIZE_KB           = 1024
$MAX_BUFFERS              = 512
$SYMBOL_SERVER_URL        = 'https://msdl.microsoft.com/download/symbols'
$DEFAULT_SYMCACHE_SUBDIR  = 'areka-diag\symcache'
$FIXTURE_DUMP_RELPATH     = 'fixtures-loop\rank\sample_ok\dump.txt'
# 同梱 dump 断片の既知の内訳（fixtures-loop/rank/sample_ok/README.md に公表してある値）。
# -SelfTest はこの値と厳密に突き合わせる。「1 つ以上」で済ませると、たとえば
# ThreadStartImage!Function 列まで数えてしまう実装（16 ではなく 32 になる）を見逃す。
$FIXTURE_EXPECT_SAMPLE_COUNT      = 16
$FIXTURE_EXPECT_STACK_COUNT       = 8
$FIXTURE_EXPECT_AREKA_FRAMES      = 16
$FIXTURE_EXPECT_RESOLVED_FRAMES   = 22
$FIXTURE_EXPECT_UNRESOLVED_FRAMES = 2
$FIXTURE_EXPECT_TIDS              = @('18332', '18420', '18512')
$TARGET_MODULE            = 'areka.exe'

# 終了コード（design C5）
$EXIT_OK             = 0
$EXIT_RUN_FAILED     = 1
$EXIT_BAD_ARGS       = 3
$EXIT_MEASURE_FAILED = 4
$EXIT_UNAVAILABLE    = 5

$SCRIPT_DIR = Split-Path -Parent $PSCommandPath
# 一時ファイルと作業用のフォルダはワークツリーの target\perf-loop-tmp\ の下だけに置く（OS の一時フォルダは使わない）。
$PERF_TMP_DIR = Join-Path (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)) 'target\perf-loop-tmp'

# --------------------------------------------------------------- 補助関数 ----
# 表示（Write-Info／Write-Problem＝失敗の文は標準エラーへ）は perf-tool-base から

# 標準出力の末尾に置く終端行。走行固有のトークンを添えて、背景終了でも読み取れるようにする。
function Write-ResultLine {
    param([string]$Mode, [int]$Code, [string]$Extra = '')
    $line = "INVOKE-CPU-SAMPLE RESULT $Mode code=$Code version=$SCRIPT_VERSION"
    if ($Extra) { $line = "$line $Extra" }
    Write-Host $line
}

function Stop-Run {
    param([string]$Mode, [int]$Code, [string]$Message, [string]$Extra = '')
    if ($Message) { Write-Problem "error: $Message" }
    Write-ResultLine -Mode $Mode -Code $Code -Extra $Extra
    exit $Code
}

# xperf.exe の所在。PATH を先に見て、無ければ Windows Kits の既定位置を見る。
function Get-XperfPath {
    $cmd = Get-Command 'xperf' -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($cmd) { return $cmd.Source }
    if (Test-Path -LiteralPath $XPERF_DEFAULT_PATH -PathType Leaf) { return $XPERF_DEFAULT_PATH }
    return $null
}

function Test-Elevated {
    $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [System.Security.Principal.WindowsPrincipal]::new($id)
    return $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-StartArguments {
    param([string]$EtlPath)
    $interval = if ($ProfIntervalUnits -gt 0) { $ProfIntervalUnits } else { $PROF_INTERVAL_UNITS }
    return @(
        '-on', $XPERF_KERNEL_FLAGS,
        '-stackwalk', $XPERF_STACKWALK,
        '-SetProfInt', "$interval",
        '-BufferSize', "$BUFFER_SIZE_KB",
        '-MaxBuffers', "$MAX_BUFFERS",
        '-f', $EtlPath
    )
}

# ------------------------------------------------------------------ 関門 ----
# dump のテキストから「解決できた areka.exe! のフレーム」を数える。
# -Stop の関門と -SelfTest の合格側・不合格側は、必ずこの同じ関数を通す（＝関門を較正する）。
#
# 列は必ず列名行（ヘッダ）から引く（xperf の版で列が動くため・design C9 の Risks）。
# 列名行の実物（Windows Performance Toolkit の perf_nt_c.dll に埋め込まれた書式そのもの）:
#   SampledProfile,  TimeStamp,  Process Name ( PID),  ThreadID,  PrgrmCtr, CPU,
#                    ThreadStartImage!Function, Image!Function, Count, SampledProfile type
#   Stack,  TimeStamp,  ThreadID, No.,  Address,  Image!Function
# 数えるのは Image!Function 列だけである。ThreadStartImage!Function 列はスレッドの
# 起点であって帰属先ではないので数えない（素朴な文字列検索だと二重に数えてしまう）。
# 行は 1 行ずつ受ける（-Stop は [IO.File]::ReadLines を渡す＝大きな dump を丸ごと持たない）。
# 7 分の採取の dump は数百万行になる。PowerShell の行ごとの処理は 1 行あたり 100 マイクロ秒を越えて
# 関門だけで何時間もかかるので、数える本体は .NET の標準だけの C#（Add-Type・1 回だけ組む）に置く。
# 比べ方は PowerShell の -eq と同じく大小を区別しない（列名の位置だけは字のまま探す）。
# C# の中の注釈は ASCII（encoding check の規則 A は here-string の中身も字句として見る）。
function Measure-ArekaFrames {
    param([System.Collections.Generic.IEnumerable[string]]$Lines)
    if (-not ('PerfToolDumpCounter' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;

public static class PerfToolDumpCounter {
    sealed class Header { public int Columns; public int ImageIndex; public int TidIndex; }
    static readonly Regex Hex = new Regex("^0x[0-9a-fA-F]+$", RegexOptions.CultureInvariant);
    static bool Same(string a, string b) { return string.Equals(a, b, StringComparison.OrdinalIgnoreCase); }

    // Returns SampleCount, StackCount, ArekaFrames, ResolvedFrames, UnresolvedFrames, ThreadIds (sorted), Problem, SeenLine
    public static Dictionary<string, object> Count(IEnumerable<string> lines, string target) {
        long samples = 0, stacks = 0, areka = 0, resolved = 0, unresolved = 0;
        bool seen = false;
        string problem = "";
        var tids = new SortedSet<string>(StringComparer.Ordinal);
        var headers = new Dictionary<string, Header>(StringComparer.OrdinalIgnoreCase);
        foreach (string raw in lines) {
            if (raw == null) continue;
            string line = raw.Trim();
            if (line.Length == 0) continue;
            seen = true;
            if (Same(line, "BeginHeader") || Same(line, "EndHeader")) continue;
            if (line.StartsWith("//", StringComparison.Ordinal) || line.StartsWith("#", StringComparison.Ordinal)) continue;
            if (line.IndexOf(',') < 0) continue;
            string[] fields = line.Split(',');
            string name = fields[0].Trim();
            bool isSample = Same(name, "SampledProfile");
            if (!isSample && !Same(name, "Stack")) continue;
            // Header row: the 2nd column is the word TimeStamp (data rows have a number there)
            if (fields.Length >= 2 && Same(fields[1].Trim(), "TimeStamp")) {
                string[] names = new string[fields.Length];
                for (int i = 0; i < fields.Length; i++) names[i] = fields[i].Trim();
                var h = new Header { Columns = names.Length, ImageIndex = Array.LastIndexOf(names, "Image!Function"), TidIndex = Array.IndexOf(names, "ThreadID") };
                headers[name] = h;
                if (h.ImageIndex < 0) problem = "Image!Function column missing in the " + name + " header row";
                continue;
            }
            Header header;
            if (!headers.TryGetValue(name, out header)) continue;   // events without a header row are not read
            if (isSample) samples++; else stacks++;
            // Rust symbols may hold commas inside generics or fn types: re-split only rows with too many columns
            if (fields.Length > header.Columns) fields = SplitOutside(line, header.Columns);
            if (header.TidIndex >= 0 && header.TidIndex < fields.Length) {
                string tid = fields[header.TidIndex].Trim();
                if (tid.Length > 0) tids.Add(tid);
            }
            if (header.ImageIndex < 0) continue;
            int extra = fields.Length - header.Columns;   // columns still left over go to Image!Function
            if (extra < 0) continue;
            string value = (extra == 0 ? fields[header.ImageIndex] : string.Join(",", fields, header.ImageIndex, extra + 1)).Trim();
            int bang = value.IndexOf('!');
            if (bang < 0) continue;
            string module = value.Substring(0, bang).Trim();
            string func = value.Substring(bang + 1).Trim();
            // only resolved frames count for the gate (areka.exe!0x... means the PDB did not match)
            if (Same(module, "Unknown") || module == "??" || Hex.IsMatch(func)) unresolved++;
            else { resolved++; if (Same(module, target)) areka++; }
        }
        // Name the missing known header rows (design C9 Risks: exit 4). A dump with SampledProfile but no Stack
        // would pass silently although no call stack was read
        var missing = new List<string>();
        foreach (string known in new[] { "SampledProfile", "Stack" }) { if (!headers.ContainsKey(known)) missing.Add(known); }
        if (missing.Count > 0) problem = "known header rows missing from dump: " + string.Join("/", missing);
        var r = new Dictionary<string, object>();
        r["SampleCount"] = samples; r["StackCount"] = stacks; r["ArekaFrames"] = areka;
        r["ResolvedFrames"] = resolved; r["UnresolvedFrames"] = unresolved;
        r["ThreadIds"] = new List<string>(tids).ToArray(); r["Problem"] = problem; r["SeenLine"] = seen;
        return r;
    }

    // Split only at commas outside brackets (<> () [], the '>' of '->' does not count); plain split if the count is not want
    static string[] SplitOutside(string line, int want) {
        var parts = new List<string>();
        int depth = 0, from = 0;
        for (int i = 0; i < line.Length; i++) {
            char ch = line[i];
            if (ch == '<' || ch == '(' || ch == '[') depth++;
            else if ((ch == '>' && (i == 0 || line[i - 1] != '-')) || ch == ')' || ch == ']') { if (depth > 0) depth--; }
            else if (ch == ',' && depth == 0) { parts.Add(line.Substring(from, i - from)); from = i + 1; }
        }
        parts.Add(line.Substring(from));
        return parts.Count == want ? parts.ToArray() : line.Split(',');
    }
}
'@
    }
    $c = [PerfToolDumpCounter]::Count($Lines, $TARGET_MODULE)
    $result = [pscustomobject]@{
        SampleCount = $c['SampleCount']; StackCount = $c['StackCount']; ArekaFrames = $c['ArekaFrames']
        ResolvedFrames = $c['ResolvedFrames']; UnresolvedFrames = $c['UnresolvedFrames']
        ThreadIds = @($c['ThreadIds']); Problem = $c['Problem']
    }
    if (-not $c['SeenLine']) { $result.Problem = 'dump is empty' }
    return $result
}

# 昇格・xperf の実在・5 秒の実採取までを確かめる。返すのは Available・Reason・Elevated・XperfPath と、
# それを -Probe の 1 行にした Line（perf-loop の Get-SamplingProbe が読む形）。
function Get-ProbeResult {
    param([switch]$SkipCapture)
    $elevated = [bool](Test-Elevated)
    $xperf = Get-XperfPath
    $tail = "elevated=$(if ($elevated) { 'true' } else { 'false' }) xperf_path=$(if ($xperf) { $xperf } else { '-' })"
    $answer = { param([bool]$Ok, [string]$Why)
        [pscustomobject]@{ Available = $Ok; Reason = $Why; Elevated = $elevated; XperfPath = $xperf
                           Line = "available=$(if ($Ok) { 'true' } else { 'false' }) reason=$Why $tail" } }

    if (-not $elevated) { return & $answer $false 'not_elevated' }
    if (-not $xperf) { return & $answer $false 'no_xperf' }
    if ($SkipCapture) { return & $answer $true 'ok' }

    $tempDir = Join-Path $PERF_TMP_DIR ('areka-cpu-probe-' + [guid]::NewGuid().ToString('N'))
    $null = New-Item -ItemType Directory -Path $tempDir -Force
    $probeEtl  = Join-Path $tempDir 'probe.etl'
    $mergedEtl = Join-Path $tempDir 'probe-merged.etl'
    try {
        $r = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments (Get-StartArguments -EtlPath $probeEtl)
        if ($r.Code -ne 0) { return & $answer $false 'start_failed' }
        Start-Sleep -Seconds $PROBE_CAPTURE_SEC
        $d = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments @('-d', $mergedEtl)
        if ($d.Code -ne 0) {
            $null = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments @('-stop')
            return & $answer $false 'start_failed'
        }
        return & $answer $true 'ok'
    } catch {
        $null = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments @('-stop')
        return & $answer $false 'start_failed'
    } finally {
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# ------------------------------------------------------------- 各モード ----
function Invoke-ProbeMode {
    $probe = Get-ProbeResult
    Write-Info $probe.Line
    Write-ResultLine -Mode 'probe' -Code $EXIT_OK -Extra $probe.Line
    exit $EXIT_OK
}

function Invoke-StartMode {
    # 実採取までは試さない（本番の採取をこれから始めるため）。前提だけを見る。
    $probe = Get-ProbeResult -SkipCapture
    if (-not $probe.Available) {
        Write-Info "available=false reason=$($probe.Reason)"
        Stop-Run -Mode 'start' -Code $EXIT_UNAVAILABLE `
            -Message "cannot start CPU sampling (reason=$($probe.Reason)). Stage 3 (per-function attribution) needs an elevated PowerShell. This is a missing capability, not a measurement failure." `
            -Extra "available=false reason=$($probe.Reason)"
    }
    $etlFull = [System.IO.Path]::GetFullPath($Etl)
    $parent = Split-Path -Parent $etlFull
    if ($parent -and -not (Test-Path -LiteralPath $parent -PathType Container)) {
        try { $null = New-Item -ItemType Directory -Path $parent -Force }
        catch { Stop-Run -Mode 'start' -Code $EXIT_BAD_ARGS -Message "could not create the -Etl output folder ${parent}: $($_.Exception.Message)" }
    }
    $r = Invoke-Child -Exe $probe.XperfPath -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments (Get-StartArguments -EtlPath $etlFull)
    if ($r.Code -ne 0) {
        Stop-Run -Mode 'start' -Code $EXIT_RUN_FAILED `
            -Message "xperf failed to start capture (exit code $($r.Code)): $($r.Lines -join ' ') - if a kernel logger is still running from an earlier capture (xperf -loggers lists NT Kernel Logger) and no other tool is sampling, run 'xperf -stop' in an elevated PowerShell."
    }
    Write-Info "capture started: $etlFull"
    Write-ResultLine -Mode 'start' -Code $EXIT_OK -Extra "etl=$etlFull"
    exit $EXIT_OK
}

# 採取を止めて ETL を消す（記号は解かない）。呼ぶのは自分で採取を始めた道具だけ（カーネルの採取は 1 つ）。
function Invoke-CancelMode {
    $etlFull = [System.IO.Path]::GetFullPath($Etl)
    $xperf = Get-XperfPath
    if (-not $xperf) {
        Stop-Run -Mode 'cancel' -Code $EXIT_UNAVAILABLE -Message 'xperf.exe not found.' -Extra 'available=false reason=no_xperf'
    }
    $r = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments @('-stop')
    if ($r.Code -ne 0) {
        Stop-Run -Mode 'cancel' -Code $EXIT_RUN_FAILED -Message "xperf -stop failed (exit code $($r.Code)): $($r.Lines -join ' ')" -Extra "etl=$etlFull"
    }
    if (Test-Path -LiteralPath $etlFull -PathType Leaf) { Remove-Item -LiteralPath $etlFull -Force }
    Write-Info "capture cancelled: $etlFull"
    Write-ResultLine -Mode 'cancel' -Code $EXIT_OK -Extra "etl=$etlFull"
    exit $EXIT_OK
}

function Invoke-StopMode {
    $etlFull = [System.IO.Path]::GetFullPath($Etl)
    $outFull = [System.IO.Path]::GetFullPath($Out)
    $pdbFull = [System.IO.Path]::GetFullPath($PdbDir)
    $xperf = Get-XperfPath
    if (-not $xperf) {
        Stop-Run -Mode 'stop' -Code $EXIT_UNAVAILABLE -Message 'xperf.exe not found.' -Extra 'available=false reason=no_xperf'
    }

    # ⒜ 何より先に採取を止めてトレースをまとめる（前提の不備で止まっても、カーネルの採取を残さない）
    $merged = [System.IO.Path]::ChangeExtension($etlFull, '.merged.etl')
    $d = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments @('-d', $merged)
    # 0 でもまとめた先が無ければ通らなかった扱い（元を消すと測った物が残らない）
    if ($d.Code -ne 0 -or -not (Test-Path -LiteralPath $merged -PathType Leaf)) {
        # -d が落ちたなら採取が止まっていないことがある。-Probe と同じく -stop で必ず止めてから読む
        if ($d.Code -ne 0) { $null = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) -Arguments @('-stop') }
        if (Test-Path -LiteralPath $etlFull -PathType Leaf) {
            Write-Info "stopping capture (-d) did not produce $merged (exit code $($d.Code)). Reading the existing $etlFull as is."
            $merged = $etlFull
        } elseif (Test-Path -LiteralPath $merged -PathType Leaf) {
            # 前の -Stop がまとめて元を消した後に前提の不備で 3 で止まり、直して打ち直した形。まとめた物を読む
            Write-Info "no capture to stop and $etlFull is gone. Reading the earlier merged trace $merged."
        } else {
            Stop-Run -Mode 'stop' -Code $EXIT_RUN_FAILED `
                -Message "xperf failed to stop capture and $etlFull does not exist (exit code $($d.Code)): $($d.Lines -join ' ')"
        }
    } elseif (Test-Path -LiteralPath $etlFull -PathType Leaf) {
        Remove-Item -LiteralPath $etlFull -Force   # まとめが通ったら元は要らない（大きい）
    }

    # 前提の確かめは止めた後
    if (-not (Test-Path -LiteralPath $pdbFull -PathType Container)) {
        Stop-Run -Mode 'stop' -Code $EXIT_BAD_ARGS -Message "-PdbDir folder not found: $pdbFull (build release with the environment variable CARGO_PROFILE_RELEASE_DEBUG=line-tables-only)"
    }
    if (-not $SymCache) {
        $localApp = [Environment]::GetFolderPath('LocalApplicationData')
        if (-not $localApp) { Stop-Run -Mode 'stop' -Code $EXIT_BAD_ARGS -Message 'could not resolve the default symbol cache location (LOCALAPPDATA). Pass -SymCache explicitly.' }
        $SymCache = Join-Path $localApp $DEFAULT_SYMCACHE_SUBDIR
    }
    try { $null = New-Item -ItemType Directory -Path $SymCache -Force -ErrorAction Stop }
    catch { Stop-Run -Mode 'stop' -Code $EXIT_BAD_ARGS -Message "could not create the -SymCache folder ${SymCache}: $($_.Exception.Message)" }

    # ⒝ 記号解決の道筋を子プロセスへ渡し、テキスト dump を書き出す
    $savedSymbolPath = $env:_NT_SYMBOL_PATH
    $env:_NT_SYMBOL_PATH = "srv*$SymCache*$SYMBOL_SERVER_URL;$pdbFull"
    try {
        $s = Invoke-Child -Exe $xperf -Quiet -ReadEncoding ([Console]::OutputEncoding) `
            -Arguments @('-i', $merged, '-symbols', '-a', 'dumper', '-o', $outFull)
        if ($s.Code -ne 0) {
            Stop-Run -Mode 'stop' -Code $EXIT_RUN_FAILED -Message "xperf symbol resolution (-a dumper) failed (exit code $($s.Code)): $($s.Lines -join ' ')"
        }
    } finally {
        $env:_NT_SYMBOL_PATH = $savedSymbolPath
    }

    # ⒞ 関門: 書き出した dump を数え直す（要件 2.11 ＝ 黙って続けない）
    if (-not (Test-Path -LiteralPath $outFull -PathType Leaf)) {
        Stop-Run -Mode 'stop' -Code $EXIT_MEASURE_FAILED -Message "dump was not written: $outFull"
    }
    $stats = Measure-ArekaFrames -Lines ([IO.File]::ReadLines($outFull))
    if ($stats.Problem) {
        Stop-Run -Mode 'stop' -Code $EXIT_MEASURE_FAILED `
            -Message "cannot read dump ($($stats.Problem)) - dump=$outFull. The xperf version may have changed the column order." `
            -Extra "dump=$outFull sample_count=$($stats.SampleCount) stack_rows=$($stats.StackCount)"
    }
    if ($stats.ArekaFrames -lt 1) {
        Stop-Run -Mode 'stop' -Code $EXIT_MEASURE_FAILED `
            -Message "no symbols resolved (0 areka.exe! frames) - dump=$outFull sample_count=$($stats.SampleCount). Build release with the environment variable CARGO_PROFILE_RELEASE_DEBUG=line-tables-only and check that -PdbDir has the PDB." `
            -Extra "dump=$outFull sample_count=$($stats.SampleCount) areka_frames=0"
    }
    # 関門を通ったらまとめたトレース（大きい）は要らない。読んだのが元の ETL なら残す（まとめに失敗した証拠）
    if ($merged -ne $etlFull) { Remove-Item -LiteralPath $merged -Force -ErrorAction Ignore }
    $summary = "sample_count=$($stats.SampleCount) areka_frames=$($stats.ArekaFrames) dump=$outFull"
    Write-Info $summary
    Write-ResultLine -Mode 'stop' -Code $EXIT_OK `
        -Extra "$summary stack_rows=$($stats.StackCount) resolved=$($stats.ResolvedFrames) unresolved=$($stats.UnresolvedFrames)"
    exit $EXIT_OK
}

# ------------------------------------------------------ 自己較正の助け ----
# 偽の xperf.cmd を置いた部屋（target\perf-loop-tmp\ の下）。本物の xperf は回さない。
# 偽物は呼ばれるたびに 'call'・'sym=<_NT_SYMBOL_PATH>'・引数を 1 行ずつ書き、-o の先へ同梱の dump を写す。
# 偽物の置き場は空白の無いフォルダ（cmd の引用の規則）、データは空白を含むフォルダに置く。
function New-FakeXperfRoom {
    param([int]$ExitCode = 0)
    $root = Join-Path $PERF_TMP_DIR ('cpu-sample-selftest-' + [guid]::NewGuid().ToString('N'))
    $room = [pscustomobject]@{
        Root = $root; Bin = (Join-Path $root 'bin'); Data = (Join-Path $root 'dir with space')
        Log  = (Join-Path $root 'xperf-calls.txt')
    }
    $null = New-Item -ItemType Directory -Path $room.Bin, $room.Data -Force
    $fixture = Join-Path $SCRIPT_DIR $FIXTURE_DUMP_RELPATH
    $cmd = @(
        '@echo off',
        ">>""$($room.Log)"" echo call",
        ">>""$($room.Log)"" echo sym=%_NT_SYMBOL_PATH%",
        'set "out="',
        'set "mergedTo="',
        ':loop',
        'if "%~1"=="" goto done',
        ">>""$($room.Log)"" echo(%~1",
        'if /i "%~1"=="-o" set "out=%~2"',
        'if /i "%~1"=="-d" set "mergedTo=%~2"',
        'shift',
        'goto loop',
        ':done',
        "if defined out copy /y ""$fixture"" ""%out%"" >nul",
        'if defined mergedTo type nul > "%mergedTo%"',
        "exit /b $ExitCode"
    )
    [IO.File]::WriteAllLines((Join-Path $room.Bin 'xperf.cmd'), $cmd, [Text.Encoding]::ASCII)
    return $room
}

# 偽物の記録を呼び出しごとに読み、記録を消す。戻り: @( @{ Sym; Args } … )
function Read-FakeXperfCalls {
    param([pscustomobject]$Room)
    $calls = [System.Collections.Generic.List[object]]::new()
    if (-not (Test-Path -LiteralPath $Room.Log -PathType Leaf)) { return ,@() }
    foreach ($line in [IO.File]::ReadAllLines($Room.Log)) {
        if ($line -eq 'call') { $calls.Add([pscustomobject]@{ Sym = ''; Args = [System.Collections.Generic.List[string]]::new() }); continue }
        if ($calls.Count -eq 0) { continue }
        $last = $calls[$calls.Count - 1]
        if ($line.StartsWith('sym=') -and $last.Args.Count -eq 0 -and -not $last.Sym) { $last.Sym = $line.Substring(4); continue }
        $last.Args.Add($line)
    }
    Remove-Item -LiteralPath $Room.Log -Force
    return ,@($calls)
}

# 自分を子として起こす（偽の xperf を子の PATH の先頭へ）
function Invoke-SelfWithFakeXperf {
    param([pscustomobject]$Room, [string[]]$Arguments)
    $argv = Get-PwshChildArguments -ScriptPath $PSCommandPath -Arguments $Arguments
    return Invoke-Child -Exe (Get-PwshPath) -Arguments $argv -Quiet -Environment @{ Path = "$($Room.Bin);$env:Path" }
}

function Get-AsciiLastLine {
    param($Result)
    return [string]($Result.Out | Select-Object -Last 1) -replace '[^\x20-\x7E]', '?'
}

# 段③の止める順・引数の渡し方・取り消しを偽の xperf で確かめる（stop-merges-first・args-with-spaces・cancel-mode）
function Test-FakeXperfModes {
    $failures = @()
    $room = New-FakeXperfRoom
    $badRoom = New-FakeXperfRoom -ExitCode 7
    try {
        $etl = Join-Path $room.Data 'cpu.etl'
        $merged = Join-Path $room.Data 'cpu.merged.etl'
        $out = Join-Path $room.Data 'dump out.txt'
        $pdb = Join-Path $room.Data 'pdb dir'
        $symCache = Join-Path $room.Data 'sym cache'

        # stop-merges-first ⒜: -PdbDir が無くても、まず -d でまとめてから 3
        [IO.File]::WriteAllText($etl, 'etl')
        $r = Invoke-SelfWithFakeXperf -Room $room -Arguments @('-Stop', '-Etl', $etl, '-Out', $out, '-PdbDir', $pdb, '-SymCache', $symCache)
        $calls = Read-FakeXperfCalls -Room $room
        $first = if ($calls.Count -gt 0) { $calls[0].Args -join ' ' } else { '(fake xperf not called)' }
        if ($r.Code -ne $EXIT_BAD_ARGS -or $calls.Count -ne 1 -or $calls[0].Args[0] -ne '-d') {
            $failures += "stop-merges-first: missing -PdbDir gave exit code $($r.Code) (want $EXIT_BAD_ARGS) after $($calls.Count) xperf call(s), first '$first' (want -d first)"
        }

        # stop-merges-first ⒝: -PdbDir が在れば -d → -i … -o、dump が関門を通り、元の ETL が消えている
        $null = New-Item -ItemType Directory -Path $pdb -Force
        [IO.File]::WriteAllText($etl, 'etl')
        $r = Invoke-SelfWithFakeXperf -Room $room -Arguments @('-Stop', '-Etl', $etl, '-Out', $out, '-PdbDir', $pdb, '-SymCache', $symCache)
        $calls = Read-FakeXperfCalls -Room $room
        $last = Get-AsciiLastLine $r
        $order = ($calls | ForEach-Object { $_.Args[0] }) -join ','
        if ($r.Code -ne $EXIT_OK -or $order -ne '-d,-i' -or (Test-Path -LiteralPath $etl) -or $last -notlike "INVOKE-CPU-SAMPLE RESULT stop code=0 *areka_frames=$FIXTURE_EXPECT_AREKA_FRAMES*") {
            $failures += "stop-merges-first: exit code $($r.Code), xperf calls '$order' (want -d,-i), etl left=$(Test-Path -LiteralPath $etl), last line '$last'"
        }

        # args-with-spaces: 空白を含む ETL・-Out は 1 つの引数のまま、-PdbDir は記号の道筋の最後に 1 つのまま
        $wantD = @('-d', $merged) -join '|'
        $wantI = @('-i', $merged, '-symbols', '-a', 'dumper', '-o', $out) -join '|'
        $gotD = if ($calls.Count -gt 0) { $calls[0].Args -join '|' } else { '' }
        $gotI = if ($calls.Count -gt 1) { $calls[1].Args -join '|' } else { '' }
        $gotSym = if ($calls.Count -gt 1) { $calls[1].Sym } else { '' }
        if ($gotD -ne $wantD -or $gotI -ne $wantI -or -not $gotSym.EndsWith(";$pdb")) {
            $failures += "args-with-spaces: got -d '$gotD' / -i '$gotI' / sym '$gotSym' (want '$wantD' / '$wantI' / ending ';$pdb')"
        }

        # cancel-mode: -stop が記録され ETL が消える。xperf が非ゼロなら 1 で ETL は残る
        [IO.File]::WriteAllText($etl, 'etl')
        $r = Invoke-SelfWithFakeXperf -Room $room -Arguments @('-Cancel', '-Etl', $etl)
        $calls = Read-FakeXperfCalls -Room $room
        $got = ($calls | ForEach-Object { $_.Args -join ' ' }) -join ','
        $last = Get-AsciiLastLine $r
        if ($r.Code -ne $EXIT_OK -or $got -ne '-stop' -or (Test-Path -LiteralPath $etl) -or $last -notlike 'INVOKE-CPU-SAMPLE RESULT cancel code=0 *') {
            $failures += "cancel-mode: exit code $($r.Code), xperf calls '$got' (want -stop), etl left=$(Test-Path -LiteralPath $etl), last line '$last'"
        }
        $badEtl = Join-Path $badRoom.Data 'cpu.etl'
        [IO.File]::WriteAllText($badEtl, 'etl')
        $r = Invoke-SelfWithFakeXperf -Room $badRoom -Arguments @('-Cancel', '-Etl', $badEtl)
        $calls = Read-FakeXperfCalls -Room $badRoom
        $last = Get-AsciiLastLine $r
        if ($r.Code -ne $EXIT_RUN_FAILED -or $calls.Count -ne 1 -or -not (Test-Path -LiteralPath $badEtl) -or $last -notlike 'INVOKE-CPU-SAMPLE RESULT cancel code=1 *' -or -not ($r.Err -match 'xperf')) {
            $failures += "cancel-mode: failing xperf -stop gave exit code $($r.Code) (want $EXIT_RUN_FAILED), $($calls.Count) call(s), etl kept=$(Test-Path -LiteralPath $badEtl), last line '$last'"
        }

        # stop-merge-fails: -d が落ちたら -stop で採取を止めてから元の ETL を読みにいく（ここでは -i も落ちて 1）
        $badPdb = Join-Path $badRoom.Data 'pdb dir'
        $null = New-Item -ItemType Directory -Path $badPdb -Force
        $r = Invoke-SelfWithFakeXperf -Room $badRoom -Arguments @('-Stop', '-Etl', $badEtl, '-Out', (Join-Path $badRoom.Data 'dump.txt'), '-PdbDir', $badPdb, '-SymCache', (Join-Path $badRoom.Data 'sym'))
        $calls = Read-FakeXperfCalls -Room $badRoom
        $order = ($calls | ForEach-Object { $_.Args[0] }) -join ','
        $readFrom = if ($calls.Count -gt 2) { $calls[2].Args[1] } else { '' }
        if ($r.Code -ne $EXIT_RUN_FAILED -or $order -ne '-d,-stop,-i' -or $readFrom -ne $badEtl) {
            $failures += "stop-merge-fails: exit code $($r.Code) (want $EXIT_RUN_FAILED), xperf calls '$order' (want -d,-stop,-i), -i read '$readFrom' (want $badEtl)"
        }
        # 元の ETL が無く前のまとめが残っていれば（直して打ち直した -Stop）、まとめた物を読む
        Remove-Item -LiteralPath $badEtl -Force
        $badMerged = [System.IO.Path]::ChangeExtension($badEtl, '.merged.etl')
        [IO.File]::WriteAllText($badMerged, 'merged')
        $r = Invoke-SelfWithFakeXperf -Room $badRoom -Arguments @('-Stop', '-Etl', $badEtl, '-Out', (Join-Path $badRoom.Data 'dump.txt'), '-PdbDir', $badPdb, '-SymCache', (Join-Path $badRoom.Data 'sym'))
        $calls = Read-FakeXperfCalls -Room $badRoom
        $readFrom = if ($calls.Count -gt 2) { $calls[2].Args[1] } else { '' }
        if ($readFrom -ne $badMerged) { $failures += "stop-merge-fails: retry without the ETL read '$readFrom' (want $badMerged), exit code $($r.Code)" }
    } finally {
        Remove-Item -LiteralPath $room.Root, $badRoom.Root -Recurse -Force -ErrorAction SilentlyContinue
    }
    return $failures
}

function Invoke-SelfTestMode {
    $failures = @()

    # 期待値と実測が食い違ったら、どちらの数字も残す（あとで台帳から追える形）。
    function Test-Expected {
        param([string]$Side, [string]$Label, $Expected, $Actual)
        if ($Expected -eq $Actual) { return $null }
        return "${Side}: $Label differs from the expected value (expected $Expected / actual $Actual)"
    }

    # ⒜-1 合格側: 同梱の dump 断片の内訳が、README に公表した既知の値と厳密に一致する。
    #      「1 つ以上」ではなく厳密一致にしてある。素朴に areka.exe! を数える実装は
    #      ThreadStartImage!Function 列まで拾って 16 ではなく 32 になるので、
    #      厳密一致でなければその取り違えを見逃す。
    $fixture = Join-Path $SCRIPT_DIR $FIXTURE_DUMP_RELPATH
    $okFrames = 0
    $okSamples = 0
    if (-not (Test-Path -LiteralPath $fixture -PathType Leaf)) {
        $failures += "bundled dump fixture not found: $fixture"
    } else {
        # stream-counts: -Stop と同じ 1 行ずつ読む道（[IO.File]::ReadLines）で数える
        $okStats = Measure-ArekaFrames -Lines ([IO.File]::ReadLines($fixture))
        $okFrames = $okStats.ArekaFrames
        $okSamples = $okStats.SampleCount
        if ($okStats.Problem) { $failures += "pass side: $($okStats.Problem) ($fixture)" }
        $failures += @(
            (Test-Expected 'pass side' 'SampledProfile row count' $FIXTURE_EXPECT_SAMPLE_COUNT     $okStats.SampleCount),
            (Test-Expected 'pass side' 'Stack row count'         $FIXTURE_EXPECT_STACK_COUNT       $okStats.StackCount),
            (Test-Expected 'pass side' 'areka.exe frame count'  $FIXTURE_EXPECT_AREKA_FRAMES      $okStats.ArekaFrames),
            (Test-Expected 'pass side' 'resolved frame count'   $FIXTURE_EXPECT_RESOLVED_FRAMES   $okStats.ResolvedFrames),
            (Test-Expected 'pass side' 'unresolved frame count' $FIXTURE_EXPECT_UNRESOLVED_FRAMES $okStats.UnresolvedFrames),
            (Test-Expected 'pass side' 'ThreadID list'          ($FIXTURE_EXPECT_TIDS -join ',')  ($okStats.ThreadIds -join ','))
        ) | Where-Object { $_ }
        Write-Info "selftest pass side: sample_count=$($okStats.SampleCount) stack_rows=$($okStats.StackCount) areka_frames=$($okStats.ArekaFrames) resolved=$($okStats.ResolvedFrames) unresolved=$($okStats.UnresolvedFrames) tids=$($okStats.ThreadIds -join ',')"
        Write-Info "selftest pass side expected: sample_count=$FIXTURE_EXPECT_SAMPLE_COUNT stack_rows=$FIXTURE_EXPECT_STACK_COUNT areka_frames=$FIXTURE_EXPECT_AREKA_FRAMES resolved=$FIXTURE_EXPECT_RESOLVED_FRAMES unresolved=$FIXTURE_EXPECT_UNRESOLVED_FRAMES tids=$($FIXTURE_EXPECT_TIDS -join ',')"
    }

    # ⒜-2 不合格側: areka.exe を含まない dump では必ず 0 と数える（毎回赤も作る）
    $redText = @(
        'BeginHeader',
        '         SampledProfile,  TimeStamp,     Process Name ( PID),   ThreadID,           PrgrmCtr, CPU, ThreadStartImage!Function,            Image!Function, Count, SampledProfile type',
        '                Stack,  TimeStamp,   ThreadID, No.,            Address,            Image!Function',
        'EndHeader',
        '         SampledProfile,    1052388,      notepad.exe ( 900),       1234, 0x00007ffb0d21c4a0,   1,     ntdll.dll!RtlUserThreadStart,       win32u.dll!NtUserPeekMessage,     1,     1',
        '         SampledProfile,    1053509,      notepad.exe ( 900),       1234, 0x00007ffb1c2d3e4f,   1,     ntdll.dll!RtlUserThreadStart,          Unknown!0x00007ffb1c2d3e4f,     1,     1',
        '                  Stack,    1053509,       1234,   1, 0x00007ffb0d21c4a0,       win32u.dll!NtUserPeekMessage'
    ) -join "`n"
    $redStats = Measure-ArekaFrames -Lines ($redText -split "`n")
    $failures += @(
        (Test-Expected 'fail side' 'areka.exe frame count'  0 $redStats.ArekaFrames),
        (Test-Expected 'fail side' 'SampledProfile row count' 2 $redStats.SampleCount),
        (Test-Expected 'fail side' 'Stack row count'         1 $redStats.StackCount),
        (Test-Expected 'fail side' 'unresolved frame count' 1 $redStats.UnresolvedFrames)
    ) | Where-Object { $_ }
    if ($redStats.Problem) { $failures += "fail side: both header rows exist but it reported the dump unreadable ($($redStats.Problem))" }
    Write-Info "selftest fail side: sample_count=$($redStats.SampleCount) stack_rows=$($redStats.StackCount) areka_frames=$($redStats.ArekaFrames) (expected areka_frames=0)"

    # ⒜-3 空の入力は「dump が空」と言い、黙って 0 を返さない
    $emptyStats = Measure-ArekaFrames -Lines ([string[]]@())
    if (-not $emptyStats.Problem) { $failures += 'empty input returned no reason' }

    # ⒜-4 列名行が欠けた入力は、どのイベントの分が無いのかまで言う（-Stop はこれで exit 4）
    $noStackText = @(
        '         SampledProfile,  TimeStamp,     Process Name ( PID),   ThreadID,           PrgrmCtr, CPU, ThreadStartImage!Function,            Image!Function, Count, SampledProfile type',
        '         SampledProfile,    1052388,        areka.exe (23140),      18332, 0x00007ff6c4a91b30,   6,        areka.exe!mainCRTStartup,        areka.exe!wintf::ecs::world::EcsWorld::try_tick_world,     1,     1'
    ) -join "`n"
    $noStackStats = Measure-ArekaFrames -Lines ($noStackText -split "`n")
    if ($noStackStats.Problem -notmatch 'Stack') {
        $failures += "missing header row: input without the Stack header row was accepted (Problem='$($noStackStats.Problem)')"
    }
    Write-Info "selftest missing header row: problem=$($noStackStats.Problem)"

    # ⒜-4b 記号のカンマ: 総称型のカンマを含む ThreadStartImage!Function と Image!Function でも、数えるのは Image!Function 列。
    #      列名行に Image!Function が無ければ、記号が無いせいにせず列名行の不備と言う
    $commaHead = '         SampledProfile,  TimeStamp,     Process Name ( PID),   ThreadID,           PrgrmCtr, CPU, ThreadStartImage!Function,            Image!Function, Count, SampledProfile type'
    $commaStats = Measure-ArekaFrames -Lines ([string[]]@(
        $commaHead,
        '                Stack,  TimeStamp,   ThreadID, No.,            Address,            Image!Function',
        '         SampledProfile,    1052388,        areka.exe (23140),      18332, 0x00007ff6c4a91b30,   6,   areka.exe!std::thread::spawn<closure$0, ()>,   areka.exe!drop_in_place<(u32, u64)>,     1,     1',
        '                  Stack,    1052388,      18332,   1, 0x00007ff6c4a91b30,       areka.exe!core::iter::Map<A, fn(u8, u8) -> u8>::next',
        # 括りが閉じない記号（C++ の operator<）の行は素直に割り直し、余りを Image!Function へ寄せて数える
        '         SampledProfile,    1052390,        areka.exe (23140),      18332, 0x00007ff6c4a91b30,   6,   foo.dll!operator<,   areka.exe!f<A, B>,     1,     1'
    ))
    if ($commaStats.ArekaFrames -ne 3 -or $commaStats.ResolvedFrames -ne 3 -or ($commaStats.ThreadIds -join ',') -ne '18332' -or $commaStats.Problem) {
        $failures += "symbol commas: areka_frames=$($commaStats.ArekaFrames) resolved=$($commaStats.ResolvedFrames) tids=$($commaStats.ThreadIds -join ',') problem='$($commaStats.Problem)' (want 3 / 3 / 18332 / none)"
    }
    # 記号の解けていない areka.exe!0x… は関門に数えない（PDB の食い違いで 0 件のまま通さない）
    $hexStats = Measure-ArekaFrames -Lines ([string[]]@($commaHead,
        '                Stack,  TimeStamp,   ThreadID, No.,            Address,            Image!Function',
        '         SampledProfile,    1052388,        areka.exe (23140),      18332, 0x00007ff6c4a91b30,   6,   areka.exe!0x00007ff6c4a90000,   areka.exe!0x00007ff6c4a91b30,     1,     1'))
    if ($hexStats.ArekaFrames -ne 0 -or $hexStats.UnresolvedFrames -ne 1) { $failures += "unresolved areka.exe!0x frame: areka_frames=$($hexStats.ArekaFrames) unresolved=$($hexStats.UnresolvedFrames) (want 0 / 1)" }
    $noImageStats = Measure-ArekaFrames -Lines ([string[]]@(($commaHead -replace ' Image!Function,', ' Image Function,'), '                Stack,  TimeStamp,   ThreadID, No.,            Address,            Image!Function'))
    if ($noImageStats.Problem -notmatch 'Image!Function column missing') { $failures += "header without Image!Function: problem='$($noImageStats.Problem)'" }

    # ⒜-5 no-args: 引数なしは必須の引数を問わずに 3 と RESULT 行 none で終わる（-NonInteractive の子でも）
    $noArgs = Invoke-PwshChild -ScriptPath $PSCommandPath -Quiet
    $noArgsLast = [string]($noArgs.Out | Select-Object -Last 1) -replace '[^\x20-\x7E]', '?'
    if ($noArgs.Code -ne $EXIT_BAD_ARGS -or $noArgsLast -notlike "INVOKE-CPU-SAMPLE RESULT none code=$EXIT_BAD_ARGS *") {
        $failures += "no-args: exit code $($noArgs.Code) (want $EXIT_BAD_ARGS), last line '$noArgsLast'"
    }

    # ⒜-6 stop-merges-first・args-with-spaces・cancel-mode（偽の xperf を子の PATH の先頭へ）
    $failures += @(Test-FakeXperfModes)

    # ⒜-7 probe-skip-capture: この関数の可否の確かめは、どれも採取を飛ばす形（-SkipCapture）で呼ぶ（構文木で見る）
    $selfAst = [System.Management.Automation.Language.Parser]::ParseFile($PSCommandPath, [ref]$null, [ref]$null)
    $selfFn = $selfAst.Find({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Invoke-SelfTestMode' }, $true)
    $probeCalls = @($selfFn.Body.FindAll({ param($n) $n -is [System.Management.Automation.Language.CommandAst] -and $n.GetCommandName() -eq 'Get-ProbeResult' }, $true))
    $noSkip = @($probeCalls | Where-Object { -not ($_.CommandElements | Where-Object { $_ -is [System.Management.Automation.Language.CommandParameterAst] -and $_.ParameterName -eq 'SkipCapture' }) })
    if ($probeCalls.Count -eq 0 -or $noSkip.Count -gt 0) {
        $failures += "probe-skip-capture: Invoke-SelfTestMode calls Get-ProbeResult $($probeCalls.Count) time(s), $($noSkip.Count) without -SkipCapture (want at least 1, all with -SkipCapture)"
    }

    # ⒝ -Probe と同じ前提の確かめ（採取は試さない＝preflight の本物の 5 秒の採取は -Probe の 1 回だけ。採れないことは赤ではない）
    $probe = Get-ProbeResult -SkipCapture
    $available = if ($probe.Available) { 'true' } else { 'false' }
    Write-Info "available=$available reason=$($probe.Reason)"

    if ($failures.Count -gt 0) {
        foreach ($f in $failures) { Write-Problem "error: $f" }
        Stop-Run -Mode 'selftest' -Code $EXIT_RUN_FAILED -Message 'self-test is red (the tool itself is broken).' `
            -Extra "failures=$($failures.Count) available=$available reason=$($probe.Reason)"
    }
    Write-Info 'self-test: pass (pass side matches the known counts exactly / fail side 0 frames / empty input and missing header row give a reason)'
    Write-ResultLine -Mode 'selftest' -Code $EXIT_OK `
        -Extra "fixture_areka_frames=$okFrames fixture_sample_count=$okSamples red_areka_frames=$($redStats.ArekaFrames) available=$available reason=$($probe.Reason)"
    exit $EXIT_OK
}

# ------------------------------------------------------------------ 本体 ----
switch ($PSCmdlet.ParameterSetName) {
    'Probe'    { Invoke-ProbeMode }
    'Start'    { Invoke-StartMode }
    'Stop'     { Invoke-StopMode }
    'Cancel'   { Invoke-CancelMode }
    'SelfTest' { Invoke-SelfTestMode }
    'None'     { Stop-Run -Mode 'none' -Code $EXIT_BAD_ARGS -Message 'specify exactly one of -Probe / -Start / -Stop / -Cancel / -SelfTest.' }
}
