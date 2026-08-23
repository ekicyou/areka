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
  DEFAULT_BALLOON_SUBDIR          -BalloonRoot 省略時に補うバルーンの相対位置。
                                  **emo2 fixture 固有の値であり他ゴーストへ流用しないこと**
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
      -GhostRoot C:\絶対パス\emo2 [-BalloonRoot C:\絶対パス\emo2\emo2-kakukaku] `
      [-OutDir C:\出力先] -ConfirmQuiet

  起動せず前提の検証と実行条件の記録だけを行う（配管の確認用）:
  pwsh -File tools/perf/invoke-perf-run.ps1 -Profile short -Build dev -GhostRoot ... -DryRun

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
    # バルーンのルート（絶対パス。省略時は <GhostRoot>\emo2-kakukaku を補う＝emo2 固有）
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
$DEFAULT_BALLOON_SUBDIR         = 'emo2-kakukaku'
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

    Write-Problem "[invoke-perf-run] 失敗: $Message"

    if ($script:OutDirCreated -and $script:OutDirPath -and (Test-Path -LiteralPath $script:OutDirPath)) {
        try {
            $errText = @(
                "この採取は失敗しました。測定結果として使ってはいけません。",
                "時刻: $(Get-UtcStamp)",
                "終了コード: $Code",
                "理由: $Message"
            ) -join [Environment]::NewLine
            Set-Content -LiteralPath (Join-Path $script:OutDirPath 'FAILED.txt') -Value $errText -Encoding utf8

            $parent = Split-Path -Parent $script:OutDirPath
            $leaf   = (Split-Path -Leaf $script:OutDirPath) + '-FAILED'
            if (Test-Path -LiteralPath (Join-Path $parent $leaf)) {
                $leaf = $leaf + '-' + [DateTime]::UtcNow.ToString('HHmmssfff')
            }
            Rename-Item -LiteralPath $script:OutDirPath -NewName $leaf
            Write-Problem "[invoke-perf-run] 作りかけの出力は $(Join-Path $parent $leaf) へ退避しました（判定には使えません）。"
        } catch {
            Write-Problem "[invoke-perf-run] 作りかけの出力の退避に失敗しました: $($_.Exception.Message)"
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
        $facts['os_caption']        = "(取得できませんでした: $($_.Exception.Message))"
    }
    try {
        $cpu = @(Get-CimInstance -ClassName Win32_Processor -ErrorAction Stop)
        $facts['cpu_model']         = ($cpu | ForEach-Object { $_.Name.Trim() }) -join ' / '
        $facts['cpu_sockets']       = $cpu.Count
        $facts['cpu_cores_physical']= ($cpu | Measure-Object -Property NumberOfCores -Sum).Sum
        $facts['cpu_max_clock_mhz'] = ($cpu | Measure-Object -Property MaxClockSpeed -Maximum).Maximum
    } catch {
        $facts['cpu_model']         = "(取得できませんでした: $($_.Exception.Message))"
    }
    try {
        $power = & powercfg /getactivescheme 2>&1
        $facts['power_scheme']      = ($power | Out-String).Trim()
    } catch {
        $facts['power_scheme']      = "(取得できませんでした)"
    }
    return $facts
}

# ソース識別（同一手順の再測が同一コードに対して行われたかを後から突き合わせるため）
function Get-SourceFacts {
    param([string]$RepoRoot)
    $facts = [ordered]@{}
    try {
        $head = & git -C $RepoRoot rev-parse HEAD 2>&1
        if ($LASTEXITCODE -eq 0) {
            $facts['git_head'] = ($head | Out-String).Trim()
            $branch = & git -C $RepoRoot rev-parse --abbrev-ref HEAD 2>&1
            if ($LASTEXITCODE -eq 0) { $facts['git_branch'] = ($branch | Out-String).Trim() }
            $status = & git -C $RepoRoot status --porcelain 2>&1
            if ($LASTEXITCODE -eq 0) {
                $dirty = @($status | Where-Object { $_ -and $_.ToString().Trim().Length -gt 0 }).Count
                $facts['git_dirty_files'] = $dirty
            }
        } else {
            $facts['git_head'] = '(git から取得できませんでした)'
        }
    } catch {
        $facts['git_head'] = "(git を実行できませんでした: $($_.Exception.Message))"
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

    $quietArgs = @('-NoProfile', '-File', $ScriptPath, '-Stage', $Stage, '-OutDir', $TargetOutDir)
    if ($GoalFilePath) { $quietArgs += @('-GoalFile', $GoalFilePath) }
    $captured = & (Get-PwshPath) @quietArgs 2>&1
    $code     = $LASTEXITCODE
    foreach ($line in @($captured)) { Write-Info ([string]$line) }
    return $code
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
    Stop-Run $EXIT_BAD_ARGS "-ConfirmQuiet と -AutoQuiet は同時に指定できません。人が確認したのなら -ConfirmQuiet、機械に確認させるのなら -AutoQuiet のどちらか一方だけを指定してください。"
}
if (-not $ConfirmQuiet -and -not $AutoQuiet -and -not $DryRun) {
    Write-Problem @"
[invoke-perf-run] 実走を開始しませんでした。

この計測は「測定マシンに他の負荷がないこと（静寂状態）」を前提にしています。
並行して開発セッション・ビルド・テスト・他のアプリが動いていると、採取した CPU 時系列は
areka の性能ではなくマシンの混み具合を測ったものになり、判定が丸ごと誤りになります。

開発者ご本人が次を確認してください:
  ・他の開発セッション（並行するビルドやテスト）が動いていないこと
  ・重いアプリケーション（ブラウザの大量タブ・動画・仮想マシン等）を閉じたこと
  ・ウイルス対策ソフトの全体スキャンなど、定期処理が走っていないこと

確認できたら -ConfirmQuiet を付けて再実行してください。
起動せずに前提だけを確かめたい場合は -DryRun を付けてください（測定にはなりません）。
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
    Stop-Run $EXIT_BAD_ARGS "水準の指定 -Profile が不正です（受け取った値: '$Profile'）。short（7 分）か long（20 分超）のどちらかを指定してください。"
}

$buildDirName = switch ($runBuild) {
    'dev'     { 'debug' }
    'release' { 'release' }
    default   { '' }
}
if (-not $buildDirName) {
    Stop-Run $EXIT_BAD_ARGS "ビルド種別の指定 -Build が不正です（受け取った値: '$Build'）。dev か release のどちらかを指定してください。"
}

# 実行体の位置は tools/perf/ からリポジトリルートを 2 階層上へたどって決める。
# -BinDir があるときだけ、その所在で上書きする（A/B 比較で実行体を取り違えないため）。
$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if ($BinDir) {
    if (-not (Test-Path -LiteralPath $BinDir -PathType Container)) {
        Stop-Run $EXIT_BAD_ARGS "-BinDir のフォルダが存在しません: $BinDir"
    }
    $binDirFull = (Resolve-Path -LiteralPath $BinDir).Path.TrimEnd('\')
    $exePath    = Join-Path $binDirFull 'areka.exe'
    if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
        Stop-Run $EXIT_BAD_ARGS "areka の実行体が -BinDir にありません: $exePath — A/B 比較では実行体一式（areka.exe と 32bit SHIORI ヘルパ）を先にこのフォルダへ複製してください。"
    }
} else {
    $binDirFull = ''
    $exePath    = Join-Path $repoRoot (Join-Path 'target' (Join-Path $buildDirName 'areka.exe'))
    if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
        $buildHint = 'cargo build -p areka' + $(if ($runBuild -eq 'release') { ' --release' } else { '' })
        Stop-Run $EXIT_BAD_ARGS "areka の実行体が見つかりません: $exePath — 先に「$buildHint」を実行してください。"
    }
}

# 目標定義ファイル（-AutoQuiet の閾値と確かめ直しの回数の出どころ）。
# check-quiet.ps1 と同じ決まりで解決し、無ければここで止める（黙って既定値へ落とさない）。
if ($Goal -and $GoalFile) {
    Stop-Run $EXIT_BAD_ARGS "-Goal と -GoalFile は同時に指定できません。"
}
$goalFileFull    = ''
$quietRetryMax   = $QUIET_RETRY_MAX_DEFAULT
$quietRetryWait  = $QUIET_RETRY_WAIT_SEC_DEFAULT
if ($Goal -or $GoalFile) {
    $goalCandidate = if ($GoalFile) { $GoalFile } else { Join-Path $PSScriptRoot (Join-Path 'goals' "$Goal.toml") }
    if (-not (Test-Path -LiteralPath $goalCandidate -PathType Leaf)) {
        Stop-Run $EXIT_BAD_ARGS "目標定義ファイルが見つかりません: $goalCandidate"
    }
    $goalFileFull = (Resolve-Path -LiteralPath $goalCandidate).Path
    $retrySettings = Read-QuietRetrySettingsFromToml ([IO.File]::ReadAllText($goalFileFull))
    if ($retrySettings.ContainsKey('retry_max'))      { $quietRetryMax  = [int]$retrySettings['retry_max'] }
    if ($retrySettings.ContainsKey('retry_wait_sec')) { $quietRetryWait = [int]$retrySettings['retry_wait_sec'] }
    if (-not $AutoQuiet) {
        Write-Info "[invoke-perf-run] 注意: 目標定義ファイル $goalFileFull は -AutoQuiet のときだけ使います（今回は使いません）。"
    }
}
if ($quietRetryMax -lt 0)  { Stop-Run $EXIT_BAD_ARGS "確かめ直しの回数が不正です（0 以上が必要）: $quietRetryMax（目標定義ファイル $goalFileFull の [quiet] retry_max）" }
if ($quietRetryWait -lt 0) { Stop-Run $EXIT_BAD_ARGS "確かめ直しの待ち秒数が不正です（0 以上が必要）: $quietRetryWait（目標定義ファイル $goalFileFull の [quiet] retry_wait_sec）" }

# ゴーストのルート: 絶対パス必須（相対パスでは pasta.dll の読み込みに失敗する既知の要件）
if (-not $GhostRoot) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot が指定されていません。ゴースト一式のルートを絶対パスで指定してください。"
}
if (-not [System.IO.Path]::IsPathFullyQualified($GhostRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot は絶対パスで指定してください（受け取った値: '$GhostRoot'）。相対パスで起動すると SHIORI の読み込みに失敗します。"
}
if (-not (Test-Path -LiteralPath $GhostRoot -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-GhostRoot のフォルダが存在しません: $GhostRoot"
}
$ghostRootFull = [System.IO.Path]::GetFullPath($GhostRoot).TrimEnd('\')
$descriptPath  = Join-Path $ghostRootFull (Join-Path 'ghost' (Join-Path 'master' 'descript.txt'))
if (-not (Test-Path -LiteralPath $descriptPath -PathType Leaf)) {
    Stop-Run $EXIT_BAD_ARGS "ゴーストの起動起点 $descriptPath が見つかりません。-GhostRoot にはゴースト一式のルート（直下に ghost\master\descript.txt があるフォルダ）を指定してください。"
}

# バルーンのルート: 省略時は emo2 固有の既定位置を補う
if (-not $BalloonRoot) {
    $BalloonRoot = Join-Path $ghostRootFull $DEFAULT_BALLOON_SUBDIR
}
if (-not [System.IO.Path]::IsPathFullyQualified($BalloonRoot)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot は絶対パスで指定してください（受け取った値: '$BalloonRoot'）。"
}
if (-not (Test-Path -LiteralPath $BalloonRoot -PathType Container)) {
    Stop-Run $EXIT_BAD_ARGS "-BalloonRoot のフォルダが存在しません: $BalloonRoot（省略時は <GhostRoot>\$DEFAULT_BALLOON_SUBDIR を補います）"
}
$balloonRootFull = [System.IO.Path]::GetFullPath($BalloonRoot).TrimEnd('\')

# =============================================================================
# 3. 出力先の作成（毎回まっさらな新規フォルダ・既存を上書きしない）
# =============================================================================
if (-not $OutDir) {
    $localAppData = [Environment]::GetFolderPath('LocalApplicationData')
    if (-not $localAppData) {
        Stop-Run $EXIT_BAD_ARGS "出力先の既定位置（LOCALAPPDATA）を解決できませんでした。-OutDir で明示してください。"
    }
    $stamp  = [DateTime]::Now.ToString('yyyyMMdd-HHmmss', [Globalization.CultureInfo]::InvariantCulture)
    $leaf   = "perf-$stamp" + $(if ($DryRun) { '-dryrun' } else { '' })
    $OutDir = Join-Path $localAppData (Join-Path 'areka-diag' $leaf)
}
if (-not [System.IO.Path]::IsPathFullyQualified($OutDir)) {
    $OutDir = [System.IO.Path]::GetFullPath((Join-Path (Get-Location).Path $OutDir))
}
if (Test-Path -LiteralPath $OutDir) {
    Stop-Run $EXIT_BAD_ARGS "出力先 $OutDir は既に存在します。採取のたびに新しいフォルダを作る決まりです（過去の採取を上書きしません）。別の -OutDir を指定してください。"
}
try {
    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
} catch {
    Stop-Run $EXIT_BAD_ARGS "出力先 $OutDir を作成できませんでした: $($_.Exception.Message)"
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
        Stop-Run $EXIT_BAD_ARGS "自動静寂確認のスクリプトが見つかりません: $checkQuietPath — -AutoQuiet はこのスクリプトを必要とします（人が確認して進めるなら -ConfirmQuiet を使ってください）。"
    }
    $maxQuietAttempts = $quietRetryMax + 1
    while ($true) {
        $quietAttempts++
        Write-Info "[invoke-perf-run] 静寂状態の自動確認 $quietAttempts/$maxQuietAttempts 回目（$CHECK_QUIET_SCRIPT）…"
        try {
            $quietCode = Invoke-CheckQuietOnce -ScriptPath $checkQuietPath -TargetOutDir $script:OutDirPath `
                -Stage $QUIET_STAGE_BEFORE -GoalFilePath $goalFileFull
        } catch {
            Stop-Run $EXIT_BAD_ARGS "自動静寂確認のスクリプトを起動できませんでした（$checkQuietPath）: $($_.Exception.Message)"
        }
        if ($quietCode -eq 0) { break }
        if ($quietCode -eq 4) {
            Stop-Run $EXIT_COUNTER_FAILED "静寂状態の自動確認で性能カウンタを読めませんでした（$CHECK_QUIET_SCRIPT が 4 で終了）。判断の根拠は quiet-$QUIET_STAGE_BEFORE.txt に残しています。"
        }
        if ($quietCode -ne 2) {
            Stop-Run $EXIT_BAD_ARGS "静寂状態の自動確認を実行できませんでした（$CHECK_QUIET_SCRIPT が $quietCode で終了）。引数か前提が不正です。"
        }
        if ($quietAttempts -ge $maxQuietAttempts) {
            Stop-Run $EXIT_QUIET_UNCONFIRMED "静寂状態の自動確認に失敗しました（$maxQuietAttempts 回確かめて、いずれも「静かでない」でした）。他の負荷が動いているマシンで採った CPU 時系列は areka の性能ではなくマシンの混み具合を測ったものになるため、起動しません。判断の根拠は quiet-$QUIET_STAGE_BEFORE.txt に残しています（下記の退避先を見てください）。"
        }
        Write-Info "[invoke-perf-run] まだ静かではありません。$quietRetryWait 秒待って確かめ直します（残り $($maxQuietAttempts - $quietAttempts) 回）。"
        Start-Sleep -Seconds $quietRetryWait
    }
    Write-Info "[invoke-perf-run] 静寂状態を確認しました（$quietAttempts 回目・根拠は $quietPath）。"
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
    'spec'                   = 'areka-P0-recompose-budget（要件 2.1 / 2.3 / 2.6 / 2.7 / 4.5）'
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
    'DEFAULT_BALLOON_SUBDIR'         = "$DEFAULT_BALLOON_SUBDIR（emo2 fixture 固有・他ゴーストへ流用しないこと）"
    'LOG_MARKER_SMOKE_GATE'          = $LOG_MARKER_SMOKE_GATE
    'cpu_unit'                       = 'cpu_percent_1core = 1 コアを 100% とする換算値（複数コア使用時は 100 超もあり得る）。1 行は約 1 秒幅の瞬時値で、15 秒の平均ではない'
}

$metaHead = @(
    '# areka 性能計測 実行条件（invoke-perf-run.ps1 が生成）',
    '# 判定スクリプト（judge-perf.py）はこの内容を測定マシンの条件として併記する（要件 4.5）。',
    ''
) -join [Environment]::NewLine
$metaHead += Format-Section '実行' $runSection
$metaHead += Format-Section '対象' $targetSection
$metaHead += Format-Section '較正値' $calibrationSection
$metaHead += Format-Section 'マシン' (Get-MachineFacts)
$metaHead += Format-Section 'ソース' (Get-SourceFacts -RepoRoot $repoRoot)
Set-Content -LiteralPath $metaPath -Value $metaHead -Encoding utf8

Write-Info "[invoke-perf-run] 水準=$runProfile ビルド=$runBuild 有界終了=${smokeExitMs}ms"
Write-Info "[invoke-perf-run] 実行体   : $exePath"
Write-Info "[invoke-perf-run] ゴースト : $ghostRootFull"
Write-Info "[invoke-perf-run] バルーン : $balloonRootFull"
Write-Info "[invoke-perf-run] 出力先   : $($script:OutDirPath)"
if ($BinDir) {
    Write-Info "[invoke-perf-run] 実行体の所在は -BinDir で上書きしています（SHA-256 $exeSha256）。"
}
if ($RustLogExtra) {
    Write-Info "[invoke-perf-run] RUST_LOG : $rustLogValue（-RustLogExtra を連結した値）"
}
if (-not $helperExists) {
    Write-Info "[invoke-perf-run] 注意: 32bit SHIORI ヘルパ $helperPath がありません。実 SHIORI なしの走行になり、発話のない別条件の計測になります（run-meta.txt に記録済み）。"
}

# =============================================================================
# 5. -DryRun はここで終わり（起動しない＝測定ではない）
# =============================================================================
if ($DryRun) {
    $dryText = @(
        'これは -DryRun（起動しない前提確認）の出力です。',
        '実走していないため run.log も cpu.csv もありません。測定結果として扱ってはいけません。',
        "時刻: $(Get-UtcStamp)"
    ) -join [Environment]::NewLine
    Set-Content -LiteralPath (Join-Path $script:OutDirPath 'DRY-RUN.txt') -Value $dryText -Encoding utf8
    Write-Info "[invoke-perf-run] -DryRun のため起動しませんでした。前提の検証と実行条件の記録のみ完了しました（測定ではありません）。"
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
        Stop-Run $EXIT_RUN_FAILED "areka を起動できませんでした（$exePath）: $($_.Exception.Message)"
    }
    if ($null -eq $proc) {
        Stop-Run $EXIT_RUN_FAILED "areka を起動できませんでした（$exePath）: プロセスを取得できませんでした。"
    }
    Write-Info "[invoke-perf-run] 起動しました（プロセス ID $($proc.Id)）。有界自動終了まで約 $([math]::Round($smokeExitMs / 60000.0, 1)) 分です。"

    # --- 最初の 1 点を採る。ここで採れないならカウンタ名・権限の問題ゆえ即エラー終了 ---
    $sampleCount = 0
    $firstValue  = $null
    $graceUntil  = [DateTime]::UtcNow.AddSeconds($COUNTER_FIRST_SAMPLE_GRACE_SEC)
    $lastError   = ''
    while ($true) {
        if ($proc.HasExited) {
            Stop-Run $EXIT_RUN_FAILED "areka が CPU の初回採取より前に終了しました（終了コード $($proc.ExitCode)）。実行ログ $runLogPath と $errLogPath を確認してください。"
        }
        try {
            $firstValue = Get-ProcessCpuPercent1Core -TargetProcessId $proc.Id -InstanceFilter "$instanceName*" `
                -IdCounterTemplate $COUNTER_PATH_PROC_ID -CpuCounterTemplate $COUNTER_PATH_PROC_CPU
            if ($null -ne $firstValue) { break }
            $lastError = "対象プロセス（ID $($proc.Id)）の性能カウンタのインスタンスが見つかりません。"
        } catch {
            $lastError = $_.Exception.Message
        }
        if ([DateTime]::UtcNow -ge $graceUntil) {
            if ($proc -and -not $proc.HasExited) { try { $proc.Kill() } catch { } }
            Stop-Run $EXIT_COUNTER_FAILED "CPU の性能カウンタを取得できませんでした（$COUNTER_FIRST_SAMPLE_GRACE_SEC 秒待機）: $lastError — カウンタ名 '$($COUNTER_PATH_PROC_CPU -f $instanceName)' はこのマシンで有効ですか（日本語版などカウンタ名が翻訳されている環境では冒頭の較正値を書き換えてください）。"
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
            Stop-Run $EXIT_RUN_FAILED "areka が予定時刻（有界自動終了 ${smokeExitMs}ms ＋ 猶予 ${EXIT_WATCHDOG_MARGIN_SEC}秒）を過ぎても終了しませんでした。停止させました。実行ログ $runLogPath を確認してください。"
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
                    $sampleErr = "対象プロセス（ID $($proc.Id)）の性能カウンタのインスタンスが見つかりません。"
                } catch {
                    $sampleErr = $_.Exception.Message
                }
                if ($proc.HasExited) { break }   # 走行が終わっただけ＝異常ではない
                $attempt++
                if ($attempt -gt $COUNTER_RETRY_MAX) {
                    try { $proc.Kill() } catch { }
                    Stop-Run $EXIT_COUNTER_FAILED "CPU の採取に $COUNTER_RETRY_MAX 回連続で失敗しました: $sampleErr — 時系列が歯抜けのまま判定に回さないため、この採取を失敗として終了します。"
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
    Stop-Run $EXIT_RUN_FAILED "areka が異常終了しました（終了コード $exitCode）。実行ログ $runLogPath と $errLogPath を確認してください。"
}
$plannedSec = $smokeExitMs / 1000.0
if ($elapsedSec -lt ($plannedSec - $EARLY_EXIT_TOLERANCE_SEC)) {
    Stop-Run $EXIT_RUN_FAILED "areka が予定より早く終了しました（予定 $([math]::Round($plannedSec))秒 に対し実際 $([math]::Round($elapsedSec))秒）。有界自動終了ではない経路で終わっており、計測として成立しません。"
}
if (-not (Test-Path -LiteralPath $runLogPath -PathType Leaf) -or (Get-Item -LiteralPath $runLogPath).Length -eq 0) {
    Stop-Run $EXIT_RUN_FAILED "実行ログ $runLogPath が空です。ログが取れていない走行は判定に使えません。"
}
$logText = Get-Content -LiteralPath $runLogPath -Raw -Encoding utf8
if ($logText -notmatch [regex]::Escape($LOG_MARKER_SMOKE_GATE)) {
    Stop-Run $EXIT_RUN_FAILED "実行ログに有界自動終了の証跡『$LOG_MARKER_SMOKE_GATE』がありません。$SMOKE_EXIT_ENV_NAME が効いていない走行は同一手順の再測になりません。"
}
if ($sampleCount -lt 1) {
    Stop-Run $EXIT_COUNTER_FAILED "CPU 時系列が 1 点も採れていません。"
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
Add-Content -LiteralPath $metaPath -Value (Format-Section '結果' $resultSection) -Encoding utf8

Write-Info "[invoke-perf-run] 完了しました（$([math]::Round($elapsedSec))秒・CPU 採取 $sampleCount 点）。"
Write-Info "[invoke-perf-run] 出力: $($script:OutDirPath)"
Write-Info "[invoke-perf-run]   $FILE_RUN_LOG / $FILE_CPU_CSV / $FILE_RUN_META"
exit $EXIT_OK
