#Requires -Version 7.0
<#
================================================================================
invoke-perf-run.ps1 — 有界実走＋CPU 時系列採取ランナー
  spec: areka-P0-recompose-budget（要件 2.1 / 2.3 / 2.6 / 2.7・design「計測資産
        （tools/perf/）→ invoke-perf-run.ps1 → Batch / Job Contract」・Key decision D7）
  版 1.1.0 の追加分の spec: areka-P0-draw-load-parity（要件 2.2 / 2.8・design
        「計測の道具（tools/perf/）→ C7 invoke-perf-run.ps1 1.1.0（追加のみ）」）

何をするか:
  1. 測定マシンが静寂状態であることの確認（-ConfirmQuiet、または -AutoQuiet による
     check-quiet.ps1 の自動確認）を関門として要求する（要件 2.7・2.8）
  2. 前提（ゴースト一式・実行体・出力先）を起動前に検証する
  3. areka 実行体を有界自動終了つきで直接起動し、標準出力・標準エラーをログへ収める
  4. 並行して対象プロセスの CPU（1 コア換算）を一定間隔で CSV へ採取する（要件 2.3）
  5. 実行条件（ビルド種別・水準・開始終了時刻・マシン識別）を run-meta.txt へ収める（要件 4.5 の素材）

判定は一切行わない。集計・合否判定は judge-perf.py の仕事であり、本スクリプトは
「同一手順で再現できる採取」だけを担う（要件 2.4・4.1）。

--------------------------------------------------------------------------------
較正値・調整値の一覧（変更する場合はここだけを書き換える。judge-perf.py と README.md は
この一覧を参照先として引用する）
--------------------------------------------------------------------------------
  SCRIPT_VERSION                  本スクリプトの版（run-meta.txt へ記録される）
  SMOKE_EXIT_MS_SHORT   = 420000  短時間水準（7 分）の有界自動終了ミリ秒
  SMOKE_EXIT_MS_LONG    = 1500000 長時間水準（25 分＝20 分超）の有界自動終了ミリ秒
  SMOKE_EXIT_ENV_NAME             有界自動終了を有効化する環境変数名
                                  （実装: crates/areka/src/main.rs の SMOKE_EXIT_ENV）
  RUST_LOG_VALUE                  実走時のログフィルタ（段階別計時ログを debug で出す）
  NO_COLOR_VALUE                  実行ログの色付けを止める（run.log を人が grep できる形で残す）
  CPU_SAMPLE_INTERVAL_SEC = 15    CPU 時系列の採取間隔（秒）
  COUNTER_PATH_PROC_ID            プロセス PID のカウンタ名（英語版 Windows の名称）
  COUNTER_PATH_PROC_CPU           プロセス CPU のカウンタ名（英語版 Windows の名称）
  COUNTER_FIRST_SAMPLE_GRACE_SEC  起動直後にカウンタのインスタンスが現れるまでの猶予（秒）
  COUNTER_RETRY_MAX               採取失敗時の再試行回数（超えたら即エラー終了）
  COUNTER_RETRY_WAIT_SEC          再試行の間隔（秒）
  EXIT_WATCHDOG_MARGIN_SEC = 180  有界終了予定時刻からの猶予（超えたら停止させて失敗扱い）
  EARLY_EXIT_TOLERANCE_SEC = 30   予定より早く終わった場合に失敗とみなす差（秒）
  POLL_INTERVAL_MS = 250          終了監視のポーリング間隔（ミリ秒）
  LOG_MARKER_SMOKE_GATE           有界自動終了が有効化された証跡のログ文言。
                                  これが実行ログに無い実走は「有界でない」＝採取失敗とみなす
  CSV_HEADER                      CPU 時系列 CSV のヘッダ行（design「CPU 時系列 CSV スキーマ」）
  CHECK_QUIET_SCRIPT              -AutoQuiet が呼ぶ自動静寂確認スクリプトのファイル名
                                  （同じフォルダに置く。実装: tools/perf/check-quiet.ps1）
  QUIET_STAGE_BEFORE  = 'before'  自動静寂確認の段階名。出力は quiet-before.txt（-OutDir 直下）
  QUIET_RETRY_MAX_DEFAULT   = 3   「静かでない」ときに待って確かめ直す回数の既定。
                                  目標定義ファイルの [quiet] retry_max があればそちらを使う
  QUIET_RETRY_WAIT_SEC_DEFAULT=60 確かめ直すまでの待ち秒数の既定。
                                  目標定義ファイルの [quiet] retry_wait_sec があればそちらを使う

CPU の「1 コア換算」について:
  採取値は Windows のプロセス性能カウンタ `% Processor Time` の値をそのまま記録する。
  この値は「1 コアを 100% とする換算値」であり、複数コアを同時に使うプロセスでは
  100 を超え得る（例: 2 コア分を使い切れば 200）。CPU コア数で割った「マシン全体に対する
  割合」ではない。要件 4.4 の「アイドル CPU（1 コア換算）3% 未満」と同じ土俵の値であり、
  brief の実測値（dev 45.3% / release 13.4%→21.6%）とも同じ土俵である。
  1 行は「その時刻に取った約 1 秒幅の瞬時値」であり、15 秒間の平均ではない
  （性能カウンタの仕組み上、1 回の採取に約 1 秒かかる）。judge-perf.py 側はこの前提で扱う。

終了コード:
  0 … 正常終了（採取完了、または -DryRun の検証完了）
  1 … 実走の失敗（起動できない・異常終了・予定より早い終了・応答なし）
  2 … 静寂状態の確認がない（-ConfirmQuiet 欠落）＝起動拒否（要件 2.7）。
      -AutoQuiet のときは、待って確かめ直しても静かにならなかった場合もこの 2 を返す（要件 2.8）
  3 … 引数・前提の不正（水準/ビルド種別の指定誤り・パス不正・出力先の作成不能。
      -AutoQuiet と -ConfirmQuiet の同時指定・-BinDir の不正もここ）
  4 … CPU カウンタの取得失敗（-AutoQuiet の自動静寂確認がカウンタを読めなかった場合を含む）
  ※ judge-perf.py の終了コード（0=合格 / 1=不合格 / 2=判定不能 / 3=引数不正）とは
     別体系である。本スクリプトは合否を判定しない。

使い方:
  pwsh -File tools/perf/invoke-perf-run.ps1 `
      -Profile short -Build dev `
      -GhostRoot <検体の窓口の folder= の値> -BalloonRoot <同じく balloon.emo2-kakukaku= の値> `
      [-OutDir C:\出力先] -ConfirmQuiet

  起動せず前提の検証と実行条件の記録だけを行う（配管の確認用）:
  pwsh -File tools/perf/invoke-perf-run.ps1 -Profile short -Build dev -GhostRoot ... -BalloonRoot ... -DryRun

--------------------------------------------------------------------------------
版 1.1.0 で足した引数（既存の呼び方は何も変わらない）
--------------------------------------------------------------------------------
  -AutoQuiet
      静寂状態の確認を人に求めず、check-quiet.ps1 で機械的に行う（要件 2.8）。
      -ConfirmQuiet とは同時に指定できない（同時指定は 3 で終わる）。
      起動の前に check-quiet.ps1 -Stage before を呼び、報告を -OutDir 直下の
      quiet-before.txt に残す。静かでなければ待って確かめ直し（既定 3 回・60 秒間隔。
      待つたびに quiet-before.txt は最新の 1 回で上書きされる）、それでも静かでなければ
      「静寂状態の自動確認に失敗」として 2 で終わる。
      閾値・採取秒数・重いプロセス名・確かめ直しの回数と待ち秒数は目標定義ファイル
      （-Goal <名前> なら tools/perf/goals/<名前>.toml、-GoalFile <パス> なら直接指定）の
      [quiet] 節から読む。どちらも省いたときは check-quiet.ps1 と本スクリプトの既定値。
      -Goal / -GoalFile は -AutoQuiet のときだけ使う。
      -DryRun と併せたときも確認そのものは実際に行う（配管の確認用）。
  -BinDir <dir>
      areka の実行体と 32bit SHIORI ヘルパの所在を target/<ビルド種別> から上書きする。
      A/B 比較で「どちらの実行体を測ったのか」を取り違えないための引数であり、
      run-meta.txt には所在（bin_dir）と実行体の SHA-256（exe_sha256）を記録する。
      SHA-256 は -BinDir の有無にかかわらず常に記録する。
  -RustLogExtra <str>
      実走時の RUST_LOG の既定値の末尾へ「,」で連結する（例: wintf::tick=debug）。
      run-meta.txt の env_RUST_LOG は連結したあとの実際の値になる。
      較正値 RUST_LOG_VALUE 自体は変えない。

  自動静寂確認つき・実行体の所在を指定・追加のログ指定つきで採る:
  pwsh -File tools/perf/invoke-perf-run.ps1 `
      -Profile short -Build release `
      -GhostRoot C:\絶対パス\emo2 -OutDir C:\出力先\A1 `
      -AutoQuiet -Goal draw-load-parity `
      -BinDir C:\出力先\bin-A -RustLogExtra 'wintf::tick=debug,areka::perf=debug'
================================================================================
#>

[CmdletBinding()]
param(
    # 水準: short（7 分）／long（20 分超）。design の 2 水準に対応する。
    # ※ PowerShell の自動変数 $PROFILE と同名だが、design が定めた引数名であり変更しない
    #    （スクリプトスコープで隠れるだけで、本スクリプトは $PROFILE を使わない）。
    [string]$Profile,
    # ビルド種別: dev（target/debug）／release（target/release）
    [string]$Build,
    # ゴーストのルート（絶対パス必須。相対パスは pasta.dll の読み込みに失敗する既知の要件）
    [string]$GhostRoot,
    # バルーンのルート（絶対パス必須。既定は無い＝ゴーストのルートの下には無いので継ぎ足せない。
    # 得方は nar-sample-path の `balloon.<バルーン名>=` の行）
    [string]$BalloonRoot,
    # 出力先（省略時は %LOCALAPPDATA%\areka-diag\perf-<日時>）
    [string]$OutDir,
    # 測定マシンが静寂状態であることの確認（要件 2.7）。これが無ければ実走を起動しない
    [switch]$ConfirmQuiet,
    # 静寂状態の確認を check-quiet.ps1 で機械的に行う（要件 2.8）。-ConfirmQuiet と排他
    [switch]$AutoQuiet,
    # 目標定義の名前。tools/perf/goals/<Goal>.toml の [quiet] を使う（-AutoQuiet のときだけ）
    [string]$Goal,
    # 目標定義ファイルを直接指定する（-Goal の代わり。試験・別所在用）
    [string]$GoalFile,
    # 実行体と 32bit SHIORI ヘルパの所在（省略時は target/<ビルド種別>）
    [string]$BinDir,
    # 実走時の RUST_LOG の既定値の末尾へ「,」で連結する追加指定
    [string]$RustLogExtra,
    # 起動せず、前提検証と実行条件の記録だけを行う（測定にはならない）
    [switch]$DryRun
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
# 外部コマンド（git）の標準エラー出力で例外を投げさせない。終了コードは自前で見る。
$PSNativeCommandUseErrorActionPreference = $false
# 子の出力を端末の文字コードに依らず UTF-8 で読む（Invoke-Utf8Child）・子の pwsh の引数を UTF-8 の書き手の前置き付きで組む（Get-PwshChildArguments）
. (Join-Path (Split-Path -Parent $PSScriptRoot) 'utf8-child.ps1')
. (Join-Path $PSScriptRoot 'perf-pwsh-child.ps1')

# =============================================================================
# 較正値・調整値（上部の一覧と対応。変更はここだけ）
# =============================================================================
$SCRIPT_VERSION                 = '1.1.0'
$SMOKE_EXIT_MS_SHORT            = 420000
$SMOKE_EXIT_MS_LONG             = 1500000
$SMOKE_EXIT_ENV_NAME            = 'AREKA_APP_SMOKE_EXIT_MS'
$RUST_LOG_VALUE                 = 'info,areka_emo_present=debug'
# 実行ログの色付け（ANSI エスケープ）を止める。tracing-subscriber 0.3.23 は出力先が端末か
# どうかを見ておらず、この変数が未設定だとファイルへ落とした run.log にも色コードが入る
# （fmt_layer.rs:739-755）。人が run.log を直接 grep できる形で残すために設定する。
# judge-perf.py 側の色除去は撤去しない——手で起動した走行や、この設定を入れる前に採った
# 走行の run.log には色が入っており、それも読めなければならない（fixture でも固定済み）。
$NO_COLOR_VALUE                 = '1'
$CPU_SAMPLE_INTERVAL_SEC        = 15
$COUNTER_PATH_PROC_ID           = '\Process({0})\ID Process'
$COUNTER_PATH_PROC_CPU          = '\Process({0})\% Processor Time'
$COUNTER_FIRST_SAMPLE_GRACE_SEC = 30
$COUNTER_RETRY_MAX              = 3
$COUNTER_RETRY_WAIT_SEC         = 2
$EXIT_WATCHDOG_MARGIN_SEC       = 180
$EARLY_EXIT_TOLERANCE_SEC       = 30
$POLL_INTERVAL_MS               = 250
$LOG_MARKER_SMOKE_GATE          = 'smoke 自動 close ゲート有効'
$CSV_HEADER                     = 'timestamp,cpu_percent_1core'
$CHECK_QUIET_SCRIPT             = 'check-quiet.ps1'
$QUIET_STAGE_BEFORE             = 'before'
$QUIET_RETRY_MAX_DEFAULT        = 3
$QUIET_RETRY_WAIT_SEC_DEFAULT   = 60

# 終了コード
$EXIT_OK                = 0
$EXIT_RUN_FAILED        = 1
$EXIT_QUIET_UNCONFIRMED = 2
$EXIT_BAD_ARGS          = 3
$EXIT_COUNTER_FAILED    = 4

# 出力ファイル名
$FILE_RUN_LOG    = 'run.log'
$FILE_RUN_ERRLOG = 'run.stderr.log'
$FILE_CPU_CSV    = 'cpu.csv'
$FILE_RUN_META   = 'run-meta.txt'

# =============================================================================
# 補助関数
# =============================================================================

# 出力先の後始末に使う状態（Stop-Run が参照する）
$script:OutDirPath    = ''
$script:OutDirCreated = $false

function Write-Info {
    param([string]$Message)
    [Console]::Out.WriteLine($Message)
}

function Write-Problem {
    param([string]$Message)
    [Console]::Error.WriteLine($Message)
}

function Get-UtcStamp {
    [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.fff', [Globalization.CultureInfo]::InvariantCulture) + 'Z'
}

# 失敗して終了する。理由を必ず表示し（黙って失敗しない）、作りかけの出力先は
# 測定結果と取り違えられない名前へ退避する（部分成果を成果として残さない）。
function Stop-Run {
    param([int]$Code, [string]$Message)

    Write-Problem "[invoke-perf-run] failed: $Message"

    if ($script:OutDirCreated -and $script:OutDirPath -and (Test-Path -LiteralPath $script:OutDirPath)) {
        try {
            $errText = @(
                "This run failed. Do not use it as a measurement result.",
                "time: $(Get-UtcStamp)",
                "exit code: $Code",
                "reason: $Message"
            ) -join [Environment]::NewLine
            Set-Content -LiteralPath (Join-Path $script:OutDirPath 'FAILED.txt') -Value $errText -Encoding utf8

            $parent = Split-Path -Parent $script:OutDirPath
            $leaf   = (Split-Path -Leaf $script:OutDirPath) + '-FAILED'
            if (Test-Path -LiteralPath (Join-Path $parent $leaf)) {
                $leaf = $leaf + '-' + [DateTime]::UtcNow.ToString('HHmmssfff')
            }
            Rename-Item -LiteralPath $script:OutDirPath -NewName $leaf
            Write-Problem "[invoke-perf-run] moved the partial output to $(Join-Path $parent $leaf) (not usable for judging)."
        } catch {
            Write-Problem "[invoke-perf-run] could not move the partial output: $($_.Exception.Message)"
        }
    }
    exit $Code
}

# 対象プロセスの CPU（1 コア換算）を 1 回採取する。
# 同名プロセスが複数あるとき、性能カウンタは areka / areka#1 / areka#2 … と枝番で区別する。
# 枝番の割り当ては起動順で変わるため、名前ではなく PID（ID Process カウンタ）で対象を選ぶ。
#
# 【重要な落とし穴】枝番は CounterSample の Path にしか現れない。InstanceName プロパティは
# どの枝番でも同じ素の名前（例: 'areka'）を返すため、InstanceName を突き合わせの鍵にすると
# 別プロセスの CPU を対象の値として黙って記録してしまう（2026-08-14 に実測で確認）。
# ゆえに Path から末尾のカウンタ名を落とした部分（例 '\\HOST\process(areka#1)'）を鍵にする。
#
# 対象インスタンスが見つからない場合は $null を返す（呼び手が再試行・終了判定を行う）。
function Get-ProcessCpuPercent1Core {
    param(
        [int]$TargetProcessId,
        [string]$InstanceFilter,
        [string]$IdCounterTemplate,
        [string]$CpuCounterTemplate
    )

    $paths = @(($IdCounterTemplate -f $InstanceFilter), ($CpuCounterTemplate -f $InstanceFilter))
    $set = Get-Counter -Counter $paths -MaxSamples 1 -ErrorAction Stop

    $idByKey  = @{}
    $cpuByKey = @{}
    foreach ($s in $set.CounterSamples) {
        $path = [string]$s.Path
        $cut  = $path.LastIndexOf('\')
        if ($cut -lt 0) { continue }
        $key = $path.Substring(0, $cut)          # 例: \\HOST\process(areka#1)
        if ($path -like '*\id process') {
            $idByKey[$key] = [int]$s.CookedValue
        } else {
            $cpuByKey[$key] = [double]$s.CookedValue
        }
    }
    foreach ($kv in $idByKey.GetEnumerator()) {
        if ($kv.Value -eq $TargetProcessId -and $cpuByKey.ContainsKey($kv.Key)) {
            return [double]$cpuByKey[$kv.Key]
        }
    }
    return $null
}

# マシン識別（要件 4.5 の「測定マシンの条件」素材）。取得できない項目は理由を値として残す。
function Get-MachineFacts {
    $facts = [ordered]@{}
    $facts['hostname']              = [Environment]::MachineName
    $facts['logical_processors']    = [Environment]::ProcessorCount
    $facts['os_version']            = [Environment]::OSVersion.VersionString
    $facts['powershell_version']    = $PSVersionTable.PSVersion.ToString()
    $facts['dotnet_version']        = [Environment]::Version.ToString()

    try {
        $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
        $facts['os_caption']        = $os.Caption
        $facts['os_build']          = $os.BuildNumber
        $facts['total_memory_mb']   = [math]::Round($os.TotalVisibleMemorySize / 1024)
    } catch {
        $facts['os_caption']        = "(unavailable: $($_.Exception.Message))"
    }
    try {
        $cpu = @(Get-CimInstance -ClassName Win32_Processor -ErrorAction Stop)
        $facts['cpu_model']         = ($cpu | ForEach-Object { $_.Name.Trim() }) -join ' / '
        $facts['cpu_sockets']       = $cpu.Count
        $facts['cpu_cores_physical']= ($cpu | Measure-Object -Property NumberOfCores -Sum).Sum
        $facts['cpu_max_clock_mhz'] = ($cpu | Measure-Object -Property MaxClockSpeed -Maximum).Maximum
    } catch {
        $facts['cpu_model']         = "(unavailable: $($_.Exception.Message))"
    }
    $facts['power_scheme']          = Get-ActivePowerScheme
    return $facts
}

# 今の電源設定を powrprof の API から '<GUID> (<名前>)' の形で取る。名前は UTF-16 で来るので
# 端末の文字コードに依らない（powercfg は端末の文字コードで書き、437 の端末では日本語の名前が ? に落ちる）。
# 取れないときは '(unavailable: <理由>)' を返す（走行は止めない）。
function Get-ActivePowerScheme {
    try {
        if (-not ('ArekaPerfRun.PowerScheme' -as [type])) {
            Add-Type -Namespace ArekaPerfRun -Name PowerScheme -MemberDefinition @'
[DllImport("powrprof.dll")]
public static extern uint PowerGetActiveScheme(IntPtr userRootPowerKey, out IntPtr activePolicyGuid);
[DllImport("powrprof.dll")]
public static extern uint PowerReadFriendlyName(IntPtr rootPowerKey, ref Guid schemeGuid, IntPtr subGroupGuid, IntPtr settingGuid, byte[] buffer, ref uint bufferSize);
[DllImport("kernel32.dll")]
public static extern IntPtr LocalFree(IntPtr mem);
'@
        }
        $api = [ArekaPerfRun.PowerScheme]
        $guidPtr = [IntPtr]::Zero
        $rc = $api::PowerGetActiveScheme([IntPtr]::Zero, [ref]$guidPtr)
        if ($rc -ne 0) { return "(unavailable: PowerGetActiveScheme returned $rc)" }
        try {
            $guid = [System.Runtime.InteropServices.Marshal]::PtrToStructure($guidPtr, [type][guid])
        } finally {
            [void]$api::LocalFree($guidPtr)
        }
        # 1 回目で大きさを聞き、2 回目で名前（UTF-16・終端の NUL 付き）を読む
        $size = [uint32]0
        $rc = $api::PowerReadFriendlyName([IntPtr]::Zero, [ref]$guid, [IntPtr]::Zero, [IntPtr]::Zero, $null, [ref]$size)
        if ($rc -ne 0 -or $size -eq 0) { return "$guid (unavailable: PowerReadFriendlyName returned $rc)" }
        $buffer = [byte[]]::new($size)
        $rc = $api::PowerReadFriendlyName([IntPtr]::Zero, [ref]$guid, [IntPtr]::Zero, [IntPtr]::Zero, $buffer, [ref]$size)
        if ($rc -ne 0) { return "$guid (unavailable: PowerReadFriendlyName returned $rc)" }
        $name = [System.Text.Encoding]::Unicode.GetString($buffer, 0, [int]$size).TrimEnd([char]0)
        return "$guid ($name)"
    } catch {
        return "(unavailable: $($_.Exception.Message))"
    }
}

# ソース識別（同一手順の再測が同一コードに対して行われたかを後から突き合わせるため）
function Get-SourceFacts {
    param([string]$RepoRoot)
    $facts = [ordered]@{}
    try {
        # git は UTF-8 で書くので、端末の文字コードに依らず UTF-8 で読む（Invoke-Utf8Child）
        $head = Invoke-Utf8Child git @('-C', $RepoRoot, 'rev-parse', 'HEAD')
        if ($head.Code -eq 0) {
            $facts['git_head'] = $head.Out.Trim()
            $branch = Invoke-Utf8Child git @('-C', $RepoRoot, 'rev-parse', '--abbrev-ref', 'HEAD')
            if ($branch.Code -eq 0) { $facts['git_branch'] = $branch.Out.Trim() }
            $status = Invoke-Utf8Child git @('-C', $RepoRoot, 'status', '--porcelain')
            if ($status.Code -eq 0) {
                $dirty = @($status.Out -split '\r?\n' | Where-Object { $_.Trim().Length -gt 0 }).Count
                $facts['git_dirty_files'] = $dirty
            }
        } else {
            $facts['git_head'] = '(unavailable: git rev-parse HEAD failed)'
        }
    } catch {
        $facts['git_head'] = "(unavailable: cannot run git: $($_.Exception.Message))"
    }
    return $facts
}

# --- 版 1.1.0 で足した補助（自動静寂確認・実行体の所在・追加ログ指定） -------------

# 目標定義ファイルの [quiet] 節から「確かめ直しの回数と待ち秒数」だけを読む最小の解釈器。
# 閾値・採取秒数・重いプロセス名は check-quiet.ps1 が自分で同じファイルから読む（二重に
# 解釈して食い違わせないため、本スクリプトはこの 2 つ以外を読まない）。
# 他の節・他のキーは黙って読み飛ばす。引用符の外の # 以降は注釈として落とす。
function Read-QuietRetrySettingsFromToml {
    param([string]$Text)
    $out = @{}
    $inQuiet = $false
    foreach ($raw in ($Text -split "`r?`n")) {
        $line = $raw.Trim()
        if ($line -eq '' -or $line.StartsWith('#')) { continue }
        if ($line -match '^\[([^\]]+)\]\s*$') { $inQuiet = ($Matches[1].Trim() -eq 'quiet'); continue }
        if (-not $inQuiet) { continue }
        if ($line -notmatch '^([A-Za-z0-9_]+)\s*=\s*(.+)$') { continue }
        $key = $Matches[1]
        $val = ($Matches[2]).Trim()

        $inStr = $false
        $cut = -1
        for ($i = 0; $i -lt $val.Length; $i++) {
            $ch = $val[$i]
            if ($ch -eq '"') { $inStr = -not $inStr }
            elseif ($ch -eq '#' -and -not $inStr) { $cut = $i; break }
        }
        if ($cut -ge 0) { $val = $val.Substring(0, $cut).Trim() }

        if ($key -eq 'retry_max' -or $key -eq 'retry_wait_sec') {
            $parsed = 0
            if ([int]::TryParse($val, [Globalization.NumberStyles]::Integer, [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)) {
                $out[$key] = $parsed
            }
        }
    }
    return $out
}

# 本スクリプトを走らせている pwsh 自身を探す（子スクリプトも同じ版で走らせる）。
function Get-PwshPath {
    if ($PSHOME) {
        $candidate = Join-Path $PSHOME 'pwsh.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    return 'pwsh'
}

# check-quiet.ps1 を 1 回呼ぶ。報告は quiet-<Stage>.txt として OutDir に残り、
# 同じ内容を本スクリプトの標準出力にも書き写す（何を根拠に静かと判定したかを実行ログに残す）。
# 返り値は check-quiet.ps1 の終了コード（0 静か / 2 静かでない / 3 引数 / 4 計測失敗）。
#
# 【落とし穴】子スクリプトの出力を関数の中でそのまま流すと、それが関数の戻り値に混ざって
# 終了コードだけを受け取ったつもりの呼び手が配列を掴む（2026-08-23 に実測で踏んだ）。
# ゆえに出力は必ず変数へ受け、書き写してから終了コードだけを返す。
function Invoke-CheckQuietOnce {
    param([string]$ScriptPath, [string]$TargetOutDir, [string]$Stage, [string]$GoalFilePath)

    $quietArgs = @('-Stage', $Stage, '-OutDir', $TargetOutDir)
    if ($GoalFilePath) { $quietArgs += @('-GoalFile', $GoalFilePath) }
    # 子の pwsh は前置きで標準出力・標準エラーを UTF-8 で書くので、UTF-8 で読む。
    # 写す順は標準出力の行、続いて標準エラーの行（交互の順は保たない）。
    $result = Invoke-Utf8Child (Get-PwshPath) (Get-PwshChildArguments -ScriptPath $ScriptPath -Arguments $quietArgs)
    foreach ($line in @($result.Out -split '\r?\n' | Where-Object { $_ -ne '' })) { Write-Info $line }
    foreach ($line in $result.ErrLines) { Write-Info $line }
    return $result.Code
}

function Format-Section {
    param([string]$Title, [System.Collections.IDictionary]$Items)
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("[$Title]")
    foreach ($k in $Items.Keys) {
        $lines.Add(("{0} = {1}" -f $k, $Items[$k]))
    }
    $lines.Add('')
    return ($lines -join [Environment]::NewLine)
}

# =============================================================================
# 1. 静寂状態の確認関門（要件 2.7・2.8）— 何よりも先に見る。既定値で素通りしない
#    -ConfirmQuiet は人の確認、-AutoQuiet は check-quiet.ps1 による機械の確認。
#    どちらの経路を通ったかは run-meta.txt の quiet_mode に残る。
# =============================================================================
if ($ConfirmQuiet -and $AutoQuiet) {
    Stop-Run $EXIT_BAD_ARGS "-ConfirmQuiet and -AutoQuiet cannot be used together. Use only one: -ConfirmQuiet if a person checked, -AutoQuiet to let the machine check."
}
if (-not $ConfirmQuiet -and -not $AutoQuiet -and -not $DryRun) {
    Write-Problem @"
[invoke-perf-run] run not started.

This measurement assumes the machine has no other load (quiet state).
If dev sessions, builds, tests or other apps run at the same time, the CPU series
measures how busy the machine is, not areka, and the verdict is wrong as a whole.

Please check yourself that:
  - no other dev session (parallel build or test) is running
  - heavy apps (many browser tabs, video, virtual machines, etc.) are closed
  - no scheduled job such as a full antivirus scan is running

When confirmed, run again with -ConfirmQuiet.
To check only the preconditions without starting areka, use -DryRun (not a measurement).
"@
    exit $EXIT_QUIET_UNCONFIRMED
}

# =============================================================================
# 2. 引数の検証
# =============================================================================
$runProfile = if ($null -eq $Profile) { '' } else { $Profile.Trim().ToLowerInvariant() }
$runBuild   = if ($null -eq $Build)   { '' } else { $Build.Trim().ToLowerInvariant() }

$smokeExitMs = switch ($runProfile) {
    'short' { $SMOKE_EXIT_MS_SHORT }
    'long'  { $SMOKE_EXIT_MS_LONG }
    default { -1 }
}
if ($smokeExitMs -lt 0) {
    Stop-Run $EXIT_BAD_ARGS "invalid -Profile (got: '$Profile'). Use short (7 min) or long (over 20 min)."
}

$buildDirName = switch ($runBuild) {
    'dev'     { 'debug' }
    'release' { 'release' }
    default   { '' }
}
if (-not $buildDirName) {
    Stop-Run $EXIT_BAD_ARGS "invalid -Build (got: '$Build'). Use dev or release."
}

# 実行体の位置は tools/perf/ からリポジトリルートを 2 階層上へたどって決める。
# -BinDir があるときだけ、その所在で上書きする（A/B 比較で実行体を取り違えないため）。
$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if ($BinDir) {
    if (-not (Test-Path -LiteralPath $BinDir -PathType Container)) {
        Stop-Run $EXIT_BAD_ARGS "-BinDir folder does not exist: $BinDir"
    }
    $binDirFull = (Resolve-Path -LiteralPath $BinDir).Path.TrimEnd('\')
    $exePath    = Join-Path $binDirFull 'areka.exe'
    if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
        Stop-Run $EXIT_BAD_ARGS "areka executable not found in -BinDir: $exePath - for A/B compare, first copy the executable set (areka.exe and the 32bit SHIORI helper) into this folder."
    }
} else {
    $binDirFull = ''
    $exePath    = Join-Path $repoRoot (Join-Path 'target' (Join-Path $buildDirName 'areka.exe'))
    if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
        $buildHint = 'cargo build -p areka' + $(if ($runBuild -eq 'release') { ' --release' } else { '' })
        Stop-Run $EXIT_BAD_ARGS "areka executable not found: $exePath - run '$buildHint' first."
    }
}

# 目標定義ファイル（-AutoQuiet の閾値と確かめ直しの回数の出どころ）。
# check-quiet.ps1 と同じ決まりで解決し、無ければここで止める（黙って既定値へ落とさない）。
if ($Goal -and $GoalFile) {
    Stop-Run $EXIT_BAD_ARGS "-Goal and -GoalFile cannot be used together."
}
$goalFileFull    = ''
$quietRetryMax   = $QUIET_RETRY_MAX_DEFAULT
$quietRetryWait  = $QUIET_RETRY_WAIT_SEC_DEFAULT
if ($Goal -or $GoalFile) {
    $goalCandidate = if ($GoalFile) { $GoalFile } else { Join-Path $PSScriptRoot (Join-Path 'goals' "$Goal.toml") }
    if (-not (Test-Path -LiteralPath $goalCandidate -PathType Leaf)) {
        Stop-Run $EXIT_BAD_ARGS "goal file not found: $goalCandidate"
    }
    $goalFileFull = (Resolve-Path -LiteralPath $goalCandidate).Path
    $retrySettings = Read-QuietRetrySettingsFromToml ([IO.File]::ReadAllText($goalFileFull))
    if ($retrySettings.ContainsKey('retry_max'))      { $quietRetryMax  = [int]$retrySettings['retry_max'] }
    if ($retrySettings.ContainsKey('retry_wait_sec')) { $quietRetryWait = [int]$retrySettings['retry_wait_sec'] }
    if (-not $AutoQuiet) {
        Write-Info "[invoke-perf-run] note: goal file $goalFileFull is used only with -AutoQuiet (not used this time)."
    }
}
if ($quietRetryMax -lt 0)  { Stop-Run $EXIT_BAD_ARGS "invalid retry count (must be 0 or more): $quietRetryMax ([quiet] retry_max in goal file $goalFileFull)" }
if ($quietRetryWait -lt 0) { Stop-Run $EXIT_BAD_ARGS "invalid retry wait seconds (must be 0 or more): $quietRetryWait ([quiet] retry_wait_sec in goal file $goalFileFull)" }

# ゴーストのルート: 絶対パス必須（相対パスでは pasta.dll の読み込みに失敗する既知の要件）
if (-not $GhostRoot) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot is not given. Pass the root of the ghost set as an absolute path."
}
if (-not [System.IO.Path]::IsPathFullyQualified($GhostRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot must be an absolute path (got: '$GhostRoot'). With a relative path, loading SHIORI fails."
}
if (-not (Test-Path -LiteralPath $GhostRoot -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot folder does not exist: $GhostRoot"
}
$ghostRootFull = [System.IO.Path]::GetFullPath($GhostRoot).TrimEnd('\')
$descriptPath  = Join-Path $ghostRootFull (Join-Path 'ghost' (Join-Path 'master' 'descript.txt'))
if (-not (Test-Path -LiteralPath $descriptPath -PathType Leaf)) {
    Stop-Run $EXIT_BAD_ARGS "ghost boot file $descriptPath not found. -GhostRoot must be the root of the ghost set (the folder with ghost\master\descript.txt under it)."
}

# バルーンのルート: 絶対パス必須。既定は置かない——検体の窓口は同梱バルーンを
# `<根>/ghost/<名前>/balloon/<directory>` へ展開する（areka-P0-ghost-inner-balloon 要件 7.1）。
# 置き場は検体ごとに窓口が決めるので、ここで名前を継ぎ足して組まず、窓口の印字した値を受け取る。
if (-not $BalloonRoot) {
    # 逐語で出す文なので単引用符で書く（二重引用符だとバッククォートが逃がし記号として食われる）
    Stop-Run $EXIT_BAD_ARGS '-BalloonRoot is not given. Pass the balloon root as an absolute path (use the value of the balloon.emo2-kakukaku= line from cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2).'
}
if (-not [System.IO.Path]::IsPathFullyQualified($BalloonRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot must be an absolute path (got: '$BalloonRoot')."
}
if (-not (Test-Path -LiteralPath $BalloonRoot -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot folder does not exist: $BalloonRoot"
}
$balloonRootFull = [System.IO.Path]::GetFullPath($BalloonRoot).TrimEnd('\')

# =============================================================================
# 3. 出力先の作成（毎回まっさらな新規フォルダ・既存を上書きしない）
# =============================================================================
if (-not $OutDir) {
    $localAppData = [Environment]::GetFolderPath('LocalApplicationData')
    if (-not $localAppData) {
        Stop-Run $EXIT_BAD_ARGS "cannot resolve the default out dir (LOCALAPPDATA). Pass -OutDir."
    }
    $stamp  = [DateTime]::Now.ToString('yyyyMMdd-HHmmss', [Globalization.CultureInfo]::InvariantCulture)
    $leaf   = "perf-$stamp" + $(if ($DryRun) { '-dryrun' } else { '' })
    $OutDir = Join-Path $localAppData (Join-Path 'areka-diag' $leaf)
}
if (-not [System.IO.Path]::IsPathFullyQualified($OutDir)) {
    $OutDir = [System.IO.Path]::GetFullPath((Join-Path (Get-Location).Path $OutDir))
}
if (Test-Path -LiteralPath $OutDir) {
    Stop-Run $EXIT_BAD_ARGS "out dir $OutDir already exists. Each run makes a new folder (past runs are never overwritten). Pass another -OutDir."
}
try {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
} catch {
    Stop-Run $EXIT_BAD_ARGS "cannot create out dir ${OutDir}: $($_.Exception.Message)"
}
$script:OutDirPath    = [System.IO.Path]::GetFullPath($OutDir)
$script:OutDirCreated = $true

$runLogPath  = Join-Path $script:OutDirPath $FILE_RUN_LOG
$errLogPath  = Join-Path $script:OutDirPath $FILE_RUN_ERRLOG
$cpuCsvPath  = Join-Path $script:OutDirPath $FILE_CPU_CSV
$metaPath    = Join-Path $script:OutDirPath $FILE_RUN_META
$quietPath   = Join-Path $script:OutDirPath ("quiet-$QUIET_STAGE_BEFORE.txt")

# =============================================================================
# 3.5 静寂状態の自動確認（-AutoQuiet・要件 2.8）— 起動の直前に、機械で確かめる
#     出力先が出来てからでないと報告を置けないので、ここで行う。
#     静かでなければ待って確かめ直し、上限を超えたら 2 で終わる（人には問わない）。
# =============================================================================
$quietMode     = if ($AutoQuiet) { 'auto' } elseif ($ConfirmQuiet) { 'confirmed' } else { '-' }
$quietAttempts = 0

if ($AutoQuiet) {
    $checkQuietPath = Join-Path $PSScriptRoot $CHECK_QUIET_SCRIPT
    if (-not (Test-Path -LiteralPath $checkQuietPath -PathType Leaf)) {
        Stop-Run $EXIT_BAD_ARGS "quiet-check script not found: $checkQuietPath - -AutoQuiet needs it (use -ConfirmQuiet if a person checked instead)."
    }
    $maxQuietAttempts = $quietRetryMax + 1
    while ($true) {
        $quietAttempts++
        Write-Info "[invoke-perf-run] quiet check $quietAttempts/$maxQuietAttempts ($CHECK_QUIET_SCRIPT)..."
        try {
            $quietCode = Invoke-CheckQuietOnce -ScriptPath $checkQuietPath -TargetOutDir $script:OutDirPath `
                -Stage $QUIET_STAGE_BEFORE -GoalFilePath $goalFileFull
        } catch {
            Stop-Run $EXIT_BAD_ARGS "cannot start the quiet-check script ($checkQuietPath): $($_.Exception.Message)"
        }
        if ($quietCode -eq 0) { break }
        if ($quietCode -eq 4) {
            Stop-Run $EXIT_COUNTER_FAILED "quiet check could not read the performance counters ($CHECK_QUIET_SCRIPT exited with 4). The evidence is in quiet-$QUIET_STAGE_BEFORE.txt."
        }
        if ($quietCode -ne 2) {
            Stop-Run $EXIT_BAD_ARGS "quiet check could not run ($CHECK_QUIET_SCRIPT exited with $quietCode). Bad arguments or preconditions."
        }
        if ($quietAttempts -ge $maxQuietAttempts) {
            Stop-Run $EXIT_QUIET_UNCONFIRMED "quiet check failed (checked $maxQuietAttempts times, never quiet). A CPU series taken while other load runs measures the machine, not areka, so areka is not started. The evidence is in quiet-$QUIET_STAGE_BEFORE.txt (see the moved folder below)."
        }
        Write-Info "[invoke-perf-run] not quiet yet. Waiting $quietRetryWait s and checking again ($($maxQuietAttempts - $quietAttempts) left)."
        Start-Sleep -Seconds $quietRetryWait
    }
    Write-Info "[invoke-perf-run] machine is quiet (attempt $quietAttempts, evidence in $quietPath)."
}

# =============================================================================
# 4. 実行条件の記録（run-meta.txt の前半・起動前に書く＝失敗しても条件は残る）
# =============================================================================
$exeItem      = Get-Item -LiteralPath $exePath
$workingDir   = Split-Path -Parent $exePath
$helperPath   = Join-Path $workingDir 'shiori-host32-helper.exe'
$helperExists = Test-Path -LiteralPath $helperPath -PathType Leaf
$profileDir   = if ($env:AREKA_PROFILE_DIR) { $env:AREKA_PROFILE_DIR } else { Join-Path $workingDir (Join-Path 'profile' 'areka') }
$instanceName = [System.IO.Path]::GetFileNameWithoutExtension($exePath)
$startedUtc   = Get-UtcStamp
$commandLine  = '"{0}" "{1}" "{2}"' -f $exePath, $ghostRootFull, $balloonRootFull
# 実行体の中身の指紋。A/B 比較で「どちらの実行体を測ったのか」を後から突き合わせるため、
# -BinDir の有無にかかわらず必ず記録する（要件 2.2）。
$exeSha256    = (Get-FileHash -LiteralPath $exePath -Algorithm SHA256).Hash
$binDirShown  = if ($binDirFull) { $binDirFull } else { '-' }
# -RustLogExtra は既定値の末尾へ「,」で連結する。較正値 RUST_LOG_VALUE 自体は変えない。
$rustLogValue = if ($RustLogExtra) { "$RUST_LOG_VALUE,$RustLogExtra" } else { $RUST_LOG_VALUE }

$runSection = [ordered]@{
    'script'                 = 'tools/perf/invoke-perf-run.ps1'
    'script_version'         = $SCRIPT_VERSION
    'spec'                   = 'areka-P0-recompose-budget (requirements 2.1 / 2.3 / 2.6 / 2.7 / 4.5)'
    'dry_run'                = [bool]$DryRun
    'quiet_confirmed'        = [bool]$ConfirmQuiet
    'profile'                = $runProfile
    'build'                  = $runBuild
    'started_utc'            = $startedUtc
    'out_dir'                = $script:OutDirPath
    'quiet_mode'             = $quietMode
    'quiet_attempts'         = $quietAttempts
}
$targetSection = [ordered]@{
    'exe_path'               = $exePath
    'exe_size_bytes'         = $exeItem.Length
    'exe_last_write_utc'     = $exeItem.LastWriteTimeUtc.ToString('yyyy-MM-ddTHH:mm:ssZ', [Globalization.CultureInfo]::InvariantCulture)
    'working_directory'      = $workingDir
    'ghost_root'             = $ghostRootFull
    'balloon_root'           = $balloonRootFull
    'shiori_helper_path'     = $helperPath
    'shiori_helper_present'  = $helperExists
    'areka_profile_dir'      = $profileDir
    'command_line'           = $commandLine
    'env_smoke_exit_name'    = $SMOKE_EXIT_ENV_NAME
    'env_smoke_exit_ms'      = $smokeExitMs
    'env_RUST_LOG'           = $rustLogValue
    'env_NO_COLOR'           = $NO_COLOR_VALUE
    'bin_dir'                = $binDirShown
    'exe_sha256'             = $exeSha256
}
$calibrationSection = [ordered]@{
    'SMOKE_EXIT_MS_SHORT'            = $SMOKE_EXIT_MS_SHORT
    'SMOKE_EXIT_MS_LONG'             = $SMOKE_EXIT_MS_LONG
    'CPU_SAMPLE_INTERVAL_SEC'        = $CPU_SAMPLE_INTERVAL_SEC
    'COUNTER_PATH_PROC_ID'           = ($COUNTER_PATH_PROC_ID -f ($instanceName + '*'))
    'COUNTER_PATH_PROC_CPU'          = ($COUNTER_PATH_PROC_CPU -f ($instanceName + '*'))
    'COUNTER_FIRST_SAMPLE_GRACE_SEC' = $COUNTER_FIRST_SAMPLE_GRACE_SEC
    'COUNTER_RETRY_MAX'              = $COUNTER_RETRY_MAX
    'COUNTER_RETRY_WAIT_SEC'         = $COUNTER_RETRY_WAIT_SEC
    'EXIT_WATCHDOG_MARGIN_SEC'       = $EXIT_WATCHDOG_MARGIN_SEC
    'EARLY_EXIT_TOLERANCE_SEC'       = $EARLY_EXIT_TOLERANCE_SEC
    'LOG_MARKER_SMOKE_GATE'          = $LOG_MARKER_SMOKE_GATE
    'cpu_unit'                       = 'cpu_percent_1core = percent of one core (can exceed 100 when several cores are used). Each row is an instant value over about 1 s, not a 15 s average'
}

$metaHead = @(
    '# areka perf run conditions (written by invoke-perf-run.ps1)',
    '# judge-perf.py shows this content as the conditions of the measuring machine (requirement 4.5).',
    ''
) -join [Environment]::NewLine
$metaHead += Format-Section "`u{5B9F}`u{884C}" $runSection
$metaHead += Format-Section "`u{5BFE}`u{8C61}" $targetSection
$metaHead += Format-Section "`u{8F03}`u{6B63}`u{5024}" $calibrationSection
$metaHead += Format-Section "`u{30DE}`u{30B7}`u{30F3}" (Get-MachineFacts)
$metaHead += Format-Section "`u{30BD}`u{30FC}`u{30B9}" (Get-SourceFacts -RepoRoot $repoRoot)
Set-Content -LiteralPath $metaPath -Value $metaHead -Encoding utf8

Write-Info "[invoke-perf-run] profile=$runProfile build=$runBuild smoke_exit=${smokeExitMs}ms"
Write-Info "[invoke-perf-run] exe      : $exePath"
Write-Info "[invoke-perf-run] ghost    : $ghostRootFull"
Write-Info "[invoke-perf-run] balloon  : $balloonRootFull"
Write-Info "[invoke-perf-run] out dir  : $($script:OutDirPath)"
if ($BinDir) {
    Write-Info "[invoke-perf-run] exe location overridden by -BinDir (SHA-256 $exeSha256)."
}
if ($RustLogExtra) {
    Write-Info "[invoke-perf-run] RUST_LOG : $rustLogValue (with -RustLogExtra appended)"
}
if (-not $helperExists) {
    Write-Info "[invoke-perf-run] note: 32bit SHIORI helper $helperPath not found. The run has no real SHIORI and no talk, so it measures a different condition (recorded in run-meta.txt)."
}

# =============================================================================
# 5. -DryRun はここで終わり（起動しない＝測定ではない）
# =============================================================================
if ($DryRun) {
    $dryText = @(
        'This is the output of -DryRun (checks preconditions without starting areka).',
        'Nothing was run, so there is no run.log and no cpu.csv. Do not treat this as a measurement result.',
        "time: $(Get-UtcStamp)"
    ) -join [Environment]::NewLine
    Set-Content -LiteralPath (Join-Path $script:OutDirPath 'DRY-RUN.txt') -Value $dryText -Encoding utf8
    Write-Info "[invoke-perf-run] -DryRun: areka not started. Only checked preconditions and recorded run conditions (not a measurement)."
    exit $EXIT_OK
}

# =============================================================================
# 6. 実走の起動と CPU 時系列の採取
# =============================================================================
Set-Content -LiteralPath $cpuCsvPath -Value $CSV_HEADER -Encoding utf8

$proc = $null
$prevSmokeEnv = [Environment]::GetEnvironmentVariable($SMOKE_EXIT_ENV_NAME)
$prevRustLog  = $env:RUST_LOG
$prevNoColor  = $env:NO_COLOR

try {
    # 子プロセスへ渡す環境変数（本スクリプト自身のプロセスに設定して継承させる）。
    [Environment]::SetEnvironmentVariable($SMOKE_EXIT_ENV_NAME, "$smokeExitMs")
    $env:RUST_LOG = $rustLogValue
    $env:NO_COLOR = $NO_COLOR_VALUE

    $launchedAt = [DateTime]::UtcNow
    try {
        $proc = Start-Process -FilePath $exePath `
            -ArgumentList @("`"$ghostRootFull`"", "`"$balloonRootFull`"") `
            -WorkingDirectory $workingDir `
            -RedirectStandardOutput $runLogPath `
            -RedirectStandardError $errLogPath `
            -NoNewWindow -PassThru
    } catch {
        Stop-Run $EXIT_RUN_FAILED "cannot start areka ($exePath): $($_.Exception.Message)"
    }
    if ($null -eq $proc) {
        Stop-Run $EXIT_RUN_FAILED "cannot start areka ($exePath): no process object returned."
    }
    Write-Info "[invoke-perf-run] started (process ID $($proc.Id)). About $([math]::Round($smokeExitMs / 60000.0, 1)) min until the smoke auto-exit."

    # --- 最初の 1 点を採る。ここで採れないならカウンタ名・権限の問題ゆえ即エラー終了 ---
    $sampleCount = 0
    $firstValue  = $null
    $graceUntil  = [DateTime]::UtcNow.AddSeconds($COUNTER_FIRST_SAMPLE_GRACE_SEC)
    $lastError   = ''
    while ($true) {
        if ($proc.HasExited) {
            Stop-Run $EXIT_RUN_FAILED "areka exited before the first CPU sample (exit code $($proc.ExitCode)). Check the run logs $runLogPath and $errLogPath."
        }
        try {
            $firstValue = Get-ProcessCpuPercent1Core -TargetProcessId $proc.Id -InstanceFilter "$instanceName*" `
                -IdCounterTemplate $COUNTER_PATH_PROC_ID -CpuCounterTemplate $COUNTER_PATH_PROC_CPU
            if ($null -ne $firstValue) { break }
            $lastError = "performance counter instance for the target process (ID $($proc.Id)) not found."
        } catch {
            $lastError = $_.Exception.Message
        }
        if ([DateTime]::UtcNow -ge $graceUntil) {
            if ($proc -and -not $proc.HasExited) { try { $proc.Kill() } catch { } }
            Stop-Run $EXIT_COUNTER_FAILED "cannot read the CPU performance counter (waited $COUNTER_FIRST_SAMPLE_GRACE_SEC s): $lastError - is the counter name '$($COUNTER_PATH_PROC_CPU -f $instanceName)' valid on this machine? (where counter names are localized, edit the calibration values at the top)"
        }
        Start-Sleep -Seconds $COUNTER_RETRY_WAIT_SEC
    }
    Add-Content -LiteralPath $cpuCsvPath -Value ("{0},{1}" -f (Get-UtcStamp), $firstValue.ToString('F2', [Globalization.CultureInfo]::InvariantCulture)) -Encoding utf8
    $sampleCount = 1

    # --- 一定間隔での採取と終了監視 ---
    $nextSampleAt = [DateTime]::UtcNow.AddSeconds($CPU_SAMPLE_INTERVAL_SEC)
    $deadline     = $launchedAt.AddMilliseconds($smokeExitMs).AddSeconds($EXIT_WATCHDOG_MARGIN_SEC)

    while (-not $proc.HasExited) {
        $now = [DateTime]::UtcNow
        if ($now -ge $deadline) {
            try { $proc.Kill() } catch { }
            Stop-Run $EXIT_RUN_FAILED "areka did not exit by the planned time (smoke auto-exit ${smokeExitMs}ms + margin ${EXIT_WATCHDOG_MARGIN_SEC}s). Killed it. Check the run log $runLogPath."
        }
        if ($now -ge $nextSampleAt) {
            $value    = $null
            $attempt  = 0
            $sampleErr = ''
            while ($true) {
                try {
                    $value = Get-ProcessCpuPercent1Core -TargetProcessId $proc.Id -InstanceFilter "$instanceName*" `
                        -IdCounterTemplate $COUNTER_PATH_PROC_ID -CpuCounterTemplate $COUNTER_PATH_PROC_CPU
                    if ($null -ne $value) { break }
                    $sampleErr = "performance counter instance for the target process (ID $($proc.Id)) not found."
                } catch {
                    $sampleErr = $_.Exception.Message
                }
                if ($proc.HasExited) { break }   # 走行が終わっただけ＝異常ではない
                $attempt++
                if ($attempt -gt $COUNTER_RETRY_MAX) {
                    try { $proc.Kill() } catch { }
                    Stop-Run $EXIT_COUNTER_FAILED "CPU sampling failed $COUNTER_RETRY_MAX times in a row: $sampleErr - a series with gaps must not be judged, so this run ends as failed."
                }
                Start-Sleep -Seconds $COUNTER_RETRY_WAIT_SEC
            }
            if ($null -ne $value) {
                Add-Content -LiteralPath $cpuCsvPath -Value ("{0},{1}" -f (Get-UtcStamp), $value.ToString('F2', [Globalization.CultureInfo]::InvariantCulture)) -Encoding utf8
                $sampleCount++
            }
            # 採取自体に約 1 秒かかるため、次回時刻は絶対時刻で積み上げてずれを溜めない。
            $nextSampleAt = $nextSampleAt.AddSeconds($CPU_SAMPLE_INTERVAL_SEC)
            while ($nextSampleAt -lt [DateTime]::UtcNow) {
                $nextSampleAt = $nextSampleAt.AddSeconds($CPU_SAMPLE_INTERVAL_SEC)
            }
        }
        Start-Sleep -Milliseconds $POLL_INTERVAL_MS
    }

    $proc.WaitForExit()
    $exitCode   = $proc.ExitCode
    $endedUtc   = Get-UtcStamp
    $elapsedSec = ([DateTime]::UtcNow - $launchedAt).TotalSeconds
} finally {
    # 環境変数を元に戻す（このプロセス限りだが、取り違えを残さない）。
    [Environment]::SetEnvironmentVariable($SMOKE_EXIT_ENV_NAME, $prevSmokeEnv)
    if ($null -eq $prevRustLog) {
        Remove-Item Env:\RUST_LOG -ErrorAction SilentlyContinue
    } else {
        $env:RUST_LOG = $prevRustLog
    }
    if ($null -eq $prevNoColor) {
        Remove-Item Env:\NO_COLOR -ErrorAction SilentlyContinue
    } else {
        $env:NO_COLOR = $prevNoColor
    }
    if ($proc -and -not $proc.HasExited) { try { $proc.Kill() } catch { } }
}

# =============================================================================
# 7. 採取結果の健全性確認（黙って壊れた成果を残さない）
# =============================================================================
if ($exitCode -ne 0) {
    Stop-Run $EXIT_RUN_FAILED "areka exited abnormally (exit code $exitCode). Check the run logs $runLogPath and $errLogPath."
}
$plannedSec = $smokeExitMs / 1000.0
if ($elapsedSec -lt ($plannedSec - $EARLY_EXIT_TOLERANCE_SEC)) {
    Stop-Run $EXIT_RUN_FAILED "areka exited earlier than planned (planned $([math]::Round($plannedSec)) s, actual $([math]::Round($elapsedSec)) s). It did not end by the smoke auto-exit, so this is not a valid measurement."
}
if (-not (Test-Path -LiteralPath $runLogPath -PathType Leaf) -or (Get-Item -LiteralPath $runLogPath).Length -eq 0) {
    Stop-Run $EXIT_RUN_FAILED "run log $runLogPath is empty. A run without a log cannot be judged."
}
$logText = Get-Content -LiteralPath $runLogPath -Raw -Encoding utf8
if ($logText -notmatch [regex]::Escape($LOG_MARKER_SMOKE_GATE)) {
    Stop-Run $EXIT_RUN_FAILED "run log has no smoke auto-exit marker (LOG_MARKER_SMOKE_GATE). A run where $SMOKE_EXIT_ENV_NAME did not take effect is not a repeat of the same procedure."
}
if ($sampleCount -lt 1) {
    Stop-Run $EXIT_COUNTER_FAILED "no CPU sample was taken."
}

# =============================================================================
# 8. 実行条件の記録（run-meta.txt の後半）と完了報告
# =============================================================================
$resultSection = [ordered]@{
    'ended_utc'            = $endedUtc
    'elapsed_sec'          = [math]::Round($elapsedSec, 1)
    'areka_exit_code'      = $exitCode
    'cpu_sample_count'     = $sampleCount
    'run_log'              = $runLogPath
    'run_stderr_log'       = $errLogPath
    'cpu_csv'              = $cpuCsvPath
}
Add-Content -LiteralPath $metaPath -Value (Format-Section "`u{7D50}`u{679C}" $resultSection) -Encoding utf8

Write-Info "[invoke-perf-run] done ($([math]::Round($elapsedSec)) s, $sampleCount CPU samples)."
Write-Info "[invoke-perf-run] output: $($script:OutDirPath)"
Write-Info "[invoke-perf-run]   $FILE_RUN_LOG / $FILE_CPU_CSV / $FILE_RUN_META"
exit $EXIT_OK
