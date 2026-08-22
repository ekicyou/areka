#Requires -Version 7.0
<#
================================================================================
perf-loop.ps1 — 自走改善ループの 1 入口（10 サブコマンド）
  spec: areka-P0-draw-load-parity（要件 1.2 / 2.7 / 2.10 / 2.11 / 5.6・design「計測の
        道具（tools/perf/）→ C5 perf-loop.ps1 → Batch / Job Contract」・Flow 2）

何をするか:
  採取・順位表・比較・追随・最終判定・自己較正を **1 つの入口**から回す（要件 2.10）。
  ループを回すスキル（perf-loop-iteration）と役割別エージェントは、道具を個別に
  呼ばずこのスクリプトのサブコマンドだけを呼ぶ。

  入口（本ファイル）が持つのは引数の検証・出力先の決定・preflight・selftest。
  実際に測る 8 本の本体は perf-loop.measure.ps1（1 ファイル 1,000 行の上限のため）。

サブコマンド（`<sub>` は RESULT 行にそのまま出る。所要は目安）:
  preflight        能力確認（昇格・xperf・PDB・判定スクリプトの版一致・Python／
                   PowerShell の版・CLAUDE_CODE_GOAL_CHECKIN_MINUTES の実効値・selftest）
  selftest         道具の自己較正（下の一覧を順に回す）                    約 1 分
  measure-baseline 25 分 × 1 本（-Build release|dev）→ verdict.txt         約 26 分
  rank-run         順位付け 7 分＋点灯（wintf::tick／areka::perf）＋段③の
                   サンプリング（使えない機械では省く）→ rank.txt          約 8 分
  rank             既にある走行から rank.txt を作り直す                    数秒
  prepare-ab       A 側（変更前）の実行体・PDB・32bit helper を bin-A へ   ビルド次第
  measure-ab       B をビルドし A1 B1 A2 B2 を交互に採り compare まで      約 30 分
  compare          perf-compare.py → compare.txt／compare.json             数十秒
  followup         invoke-followup-checks.ps1（見た目の追随 4 検査）        約 2 分
                   ＝判定まで届けば 0（PASS／FAIL／INCONCLUSIVE の別は報告行で読む）
  final            25 分 × release／dev → judge-perf.py --mode verdict     約 26 分 × 本数

各サブコマンドが何を書くか（出力先は下の「出力先」の配置）:
  measure-baseline  run.log／cpu.csv／run-meta.txt／quiet-before.txt／quiet-after.txt／
                    verdict.txt
  rank-run          上と同じ 5 つ＋sampling.txt＋rank.txt（段③が使えれば dump.txt も）
  prepare-ab        bin-A/{areka.exe, areka.pdb, shiori-host32-helper.exe, BUILD.txt}
  measure-ab        bin-B/…＋A1／B1／A2／B2 の各走行＋compare.txt／compare.json
  followup          followup/{run.log, probe.log, followup.txt, followup-verdict.txt}
  final             <build>/ ごとに走行 5 つ＋verdict.txt（spec の results へも写す）

子の道具の出力は**必ず UTF-8 として読む**（perf-loop.common.ps1 の Invoke-Child）。
端末の既定コードページが CP932 の環境でも、道具が UTF-8 で書いた文字が化けないため。
これは字面を比べる検査（goal-text と goals/*.goal.md の一致）の前提である。

段③の可否語（`function_stage=UNAVAILABLE reason=…`）:
  not_elevated / no_xperf / start_failed … invoke-cpu-sample.ps1 -Probe が返す語（design C8）
  no_pdb                                 … 記号解決に要る PDB が無い（preflight が足す語）
  probe_failed                           … -Probe そのものが回らなかった（道具の異常。
                                           C8 の語彙には無く、preflight が足す語）
  dry_run                                … -DryRun なので採らなかった（C8 の語彙には
                                           無く、rank-run が足す語。測定ではない）

出力先（design C5 の配置。所在の唯一の定義は perf-loop.common.ps1 の Get-LoopDir）:
  %LOCALAPPDATA%\areka-diag\perf-loop\<goal>\preflight.txt
  %LOCALAPPDATA%\areka-diag\perf-loop\<goal>\baseline-<date>\
  %LOCALAPPDATA%\areka-diag\perf-loop\<goal>\iter-<n>\{rank,A1,B1,A2,B2,bin-A,bin-B,followup}\
  %LOCALAPPDATA%\areka-diag\perf-loop\<goal>\final-<date>\
  同じ出力先に -Resume を付けると、既にある成果物を作り直さず再利用する（冪等）。

1 周の回し方（背景実行 1 本＝1 ターン。要件 1.11＝check-in を跨がせない）:
  周 0   preflight → measure-baseline -Build release → measure-baseline -Build dev
         → rank-run -Date <日付>（それぞれ別ターン）
  周 n   rank-run -Iter n → prepare-ab -Iter n →（変更を実装）→ measure-ab -Iter n
         → followup -Iter n
  最後   final -Build release → final -Build dev -Resume

--------------------------------------------------------------------------------
較正値・調整値の一覧（変更する場合はここだけを書き換える）
--------------------------------------------------------------------------------
  SCRIPT_VERSION              本スクリプトの版
  DEFAULT_GOAL                -Goal を省いたときの目標の名前
  SELFTEST_TOOLS              自己較正を回す道具の一覧と順序（下の表）
  CHECKIN_DEFAULT_MINUTES     CLAUDE_CODE_GOAL_CHECKIN_MINUTES 未設定時の実効値（30 分・
                              Claude Code の既定。公式文書 code.claude.com/docs/en/goal.md）
  CHECKIN_WARN_BELOW_MINUTES  これ未満なら警告（25 分水準の計測に check-in が割り込む）
  PYTHON_MIN_VERSION          tomllib が要る最低の Python（3.11）
  PWSH_MIN_MAJOR              最低の PowerShell（7）
  XPERF_DEFAULT_PATH          PATH に xperf が無いときに見に行く既定の位置
                              （invoke-cpu-sample.ps1 の同名定数の写し）
  RELEASE_PDB_RELPATH         段③の記号解決に要る PDB の在り処

終了コード（design C5 の体系。全てのサブコマンドで同じ意味）:
  0 … 完了（段③が UNAVAILABLE でも preflight は 0 で続行を許す）
  1 … 実走の失敗
  2 … 静寂でない（再試行の上限を超えた）
  3 … 引数・前提の不正（未知のサブコマンド・目標定義ファイルが無い・未実装の受け口）
  4 … 計測失敗＝MEASURE_FAILED（自己較正が赤・判定スクリプトの版不一致・空採取・
      記号解決ゼロ）。台帳へは MEASURE_FAILED として記録される
  5 … 能力不足＝UNAVAILABLE（昇格なし・xperf なし・PDB なし）。**停止の理由にしない**。
      preflight は段③を UNAVAILABLE と記して 0 で返す（順位表は段①②④で続く）

標準出力の末尾には必ず次の 1 行を出す（背景実行の終了で会話へ届く形＝要件 1.11）:
  PERF-LOOP RESULT <sub> code=<n> dir=<path>
  <sub> は上の一覧の語。語彙にないサブコマンド（打ち間違い）のときは '-' を出す
  ——読む側が語彙外の字面を受け取らないようにするため。<path> は出力先が無ければ '-'。

使い方:
  pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest
  pwsh -NoProfile -File tools/perf/perf-loop.ps1 preflight -Goal draw-load-parity
  pwsh -NoProfile -File tools/perf/perf-loop.ps1 rank-run -Iter 3
  pwsh -NoProfile -File tools/perf/perf-loop.ps1 measure-ab -Iter 3 -Resume

-DryRun（**試験専用**・測定にはならない）:
  areka を起動せず、cargo build・サンプリング・追随チェック・静寂確認も行わずに、
  出力先の階層と -Resume と RESULT 行だけを安く確かめる。証跡（quiet-*.txt）は
  偽造しない。詳しくは perf-loop.measure.ps1 の頭書を見ること。
================================================================================
#>

[CmdletBinding()]
param(
    # サブコマンド（上の一覧のいずれか）。未知の語は 3 で止める
    [Parameter(Position = 0)]
    [string]$Sub,

    # 目標の名前。tools/perf/goals/<Goal>.toml を読む
    [string]$Goal = 'draw-load-parity',

    # 目標定義ファイルを直接指定する（-Goal の代わり。試験・別所在用）
    [string]$GoalFile,

    # 周番号（iter-<n> の <n>）。周の作業場を使うサブコマンドが読む
    [int]$Iter = 0,

    # 走行の出力先を直接指定する（既定の配置を使わないとき）
    [string]$RunDir,

    # 既にある成果物を作り直さず再利用する（冪等）
    [switch]$Resume,

    # 計測するビルド（measure-baseline／final が使う）
    [string]$Build = 'release',

    # 出力先の根（既定は %LOCALAPPDATA%\areka-diag\perf-loop）
    [string]$OutRoot,

    # 台帳のパス（既定は目標定義の [goal].spec_dir + ledger）
    [string]$Ledger,

    # 出力先の名前に使う日付（yyyyMMdd）。既定は実行日
    [string]$Date,

    # 実行体と 32bit ヘルパの所在（省略時は target\<release|debug>）。
    # measure-ab は bin-A／bin-B を自分で決めるので、ここは使わない
    [string]$BinDir,

    # ゴースト一式のルート（絶対パス。省略時は emo2 fixture）
    [string]$GhostRoot,

    # バルーンのルート（絶対パス。省略時は <GhostRoot>\emo2-kakukaku）
    [string]$BalloonRoot,

    # **試験専用**。areka を起動せず、配管（出力先・-Resume・RESULT 行）だけを確かめる。
    # 静寂確認・cargo build・サンプリング・追随チェックも行わない（測定にはならない）
    [switch]$DryRun
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
# 子プロセス（python・pwsh・xperf）の非ゼロ終了で例外を投げさせない。
# 自己較正は「赤も作って道具を較正する」ので、非ゼロは正常な観測結果である。
$PSNativeCommandUseErrorActionPreference = $false

# =============================================================================
# 較正値・調整値（上部の一覧と対応。変更はここだけ）
# =============================================================================
$SCRIPT_VERSION             = '1.0.0'
$DEFAULT_GOAL               = 'draw-load-parity'
$CHECKIN_DEFAULT_MINUTES    = 30
$CHECKIN_WARN_BELOW_MINUTES = 25
$CHECKIN_ENV_NAME           = 'CLAUDE_CODE_GOAL_CHECKIN_MINUTES'
$PYTHON_MIN_VERSION         = [Version]'3.11'
$PWSH_MIN_MAJOR             = 7
$XPERF_DEFAULT_PATH         = 'C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe'
$RELEASE_PDB_RELPATH        = 'target\release\areka.pdb'
$PDB_BUILD_HINT             = 'CARGO_PROFILE_RELEASE_DEBUG=line-tables-only を付けて cargo build --release すると出る'
$PREFLIGHT_FILE             = 'preflight.txt'
$SELFTEST_TOKEN             = '12345678'

# サブコマンドの全面（RESULT 行の <sub> はこの語のいずれか）
$SUB_PREFLIGHT = 'preflight'
$SUB_SELFTEST  = 'selftest'
#: 入口（本ファイル）が持つ 2 本
$SUBCOMMANDS_ENTRY   = @($SUB_PREFLIGHT, $SUB_SELFTEST)
#: perf-loop.measure.ps1 が持つ 8 本（実際に測る側）
$SUBCOMMANDS_MEASURE = @(
    'measure-baseline', 'rank-run', 'rank', 'prepare-ab', 'measure-ab', 'compare', 'followup', 'final'
)
$SUBCOMMANDS = $SUBCOMMANDS_ENTRY + $SUBCOMMANDS_MEASURE
$BUILD_KINDS = @('dev', 'release')

# 自己較正を回す道具の一覧（順序も較正値＝この順で回す）。
#   Task  … その道具を作ったタスク（tasks.md 5.1〜6.4）
#   Kind  … py＝python スクリプト／ps＝PowerShell スクリプト
$SELFTEST_TOOLS = @(
    [pscustomobject]@{ Task = '6.1'; Kind = 'py'; File = 'judge-perf.py';             Arguments = @('--selftest') }
    [pscustomobject]@{ Task = '6.2'; Kind = 'py'; File = 'perf-rank.py';              Arguments = @('--selftest') }
    [pscustomobject]@{ Task = '6.3'; Kind = 'py'; File = 'perf-compare.py';           Arguments = @('--selftest') }
    [pscustomobject]@{ Task = '5.5'; Kind = 'py'; File = 'perf-ledger.py';            Arguments = @('--selftest') }
    [pscustomobject]@{ Task = '6.4'; Kind = 'py'; File = 'judge-followup.py';         Arguments = @('--selftest') }
    [pscustomobject]@{ Task = '5.3'; Kind = 'ps'; File = 'invoke-cpu-sample.ps1';     Arguments = @('-SelfTest') }
    [pscustomobject]@{ Task = '5.1'; Kind = 'ps'; File = 'check-quiet.ps1';           Arguments = @('-SelfTest') }
    [pscustomobject]@{ Task = '6.4'; Kind = 'ps'; File = 'invoke-followup-checks.ps1'; Arguments = @('-SelfTest') }
)

#: 自己較正の最後に回す自前の検査（道具ではなく文書との一致を見る＝tasks.md (7.1)）。
$SELFTEST_GOAL_TEXT_NAME = 'goal-text-vs-goal-md'

# 終了コード
$EXIT_OK             = 0
$EXIT_RUN_FAILED     = 1
$EXIT_NOT_QUIET      = 2
$EXIT_BAD_ARGS       = 3
$EXIT_MEASURE_FAILED = 4
$EXIT_UNAVAILABLE    = 5

# 判定語（読む側＝perf-loop-iteration スキル・台帳と突き合わせる字面）
$STAGE_AVAILABLE     = 'AVAILABLE'
$STAGE_UNAVAILABLE   = 'UNAVAILABLE'
$VERDICT_PREFLIGHT_OK = 'PREFLIGHT_OK'
$VERDICT_PREFLIGHT_NG = 'PREFLIGHT_NG'

# =============================================================================
# script scope の状態（共通部品が読む。dot-source より先に置くこと）
# =============================================================================
$script:CurrentSub  = if ($Sub) { $Sub } else { '-' }
$script:CurrentDir  = '-'
$script:OutRootPath = $null
$script:GoalName    = if ($Goal) { $Goal } else { $DEFAULT_GOAL }
$script:DateStamp   = $Date

# 計測サブコマンド（perf-loop.measure.ps1）が読む引数。**引数を読む所在はここだけ**に
# しておく——8 本の本体が $PSBoundParameters や param 変数を直に触ると、どの引数が
# 効くのかが 8 か所に散るためである。
$script:IterArg        = $Iter
$script:RunDirArg      = $RunDir
$script:BuildKind      = $Build
$script:BuildExplicit  = $PSBoundParameters.ContainsKey('Build')
$script:BinDirArg      = $BinDir
$script:GhostRootArg   = $GhostRoot
$script:BalloonRootArg = $BalloonRoot
$script:ResumeMode     = [bool]$Resume
$script:DryRunMode     = [bool]$DryRun

. (Join-Path $PSScriptRoot 'perf-loop.common.ps1')

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

# 計測サブコマンド 8 本の本体（共通部品と較正値の後・$repoRoot の後に読むこと）
. (Join-Path $PSScriptRoot 'perf-loop.measure.ps1')

# =============================================================================
# 自己較正（selftest）
# =============================================================================

# `goal-text` の出力（走行トークンを <token> に置換）が goals/<goal>.goal.md の
# `---` 以降の本文と一字も違わないことを確かめる。文面の所在は 1 箇所（要件 1.6）で、
# テンプレート（perf_ledger_goal.py の GOAL_TEXT_TEMPLATE）を直すと写しが黙ってずれる
# ——ずれを見つけるのがこの検査である（tasks.md Implementation Notes (7.1)）。
# 台帳は使い捨ての一時ファイルへ作る（本物の台帳の走行トークンには触れない）。
function Test-GoalTextMatchesGoalMd {
    param(
        [Parameter(Mandatory = $true)][pscustomobject]$Python,
        [Parameter(Mandatory = $true)][string]$GoalName,
        [string]$GoalFilePath
    )
    $ledgerScript = Join-Path $PSScriptRoot 'perf-ledger.py'
    $goalMdPath   = Join-Path $PSScriptRoot (Join-Path 'goals' "$GoalName.goal.md")
    if (-not (Test-Path -LiteralPath $goalMdPath -PathType Leaf)) {
        return [pscustomobject]@{ Ok = $false; Code = -1; Note = "条件文の写しがありません: $goalMdPath"; Lines = @() }
    }

    $temp = Join-Path ([System.IO.Path]::GetTempPath()) ("perf-loop-selftest-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $temp | Out-Null
    try {
        $tempLedger = Join-Path $temp 'loop-ledger.md'
        $common = @('--ledger', $tempLedger)
        if ($GoalFilePath) { $common += @('--goal-file', $GoalFilePath) } else { $common += @('--goal', $GoalName) }

        $init = Invoke-PythonChild -Python $Python -Arguments (@($ledgerScript, 'init') + $common) -Quiet
        if ($init.Code -ne 0) {
            return [pscustomobject]@{ Ok = $false; Code = $init.Code; Note = "台帳を作れません: $($init.LastLine)"; Lines = $init.Lines }
        }
        $check = Invoke-PythonChild -Python $Python -Arguments (@($ledgerScript, 'goal-check') + $common + @('--token', $SELFTEST_TOKEN)) -Quiet
        if ($check.Code -ne 0) {
            return [pscustomobject]@{ Ok = $false; Code = $check.Code; Note = "goal-check が通りません: $($check.LastLine)"; Lines = $check.Lines }
        }
        # 条件文の本文（標準出力）と報告行（標準エラー）は分けて受け取る。
        # Invoke-PythonChild は端末のコードページに依らず UTF-8 で読む——ここを親の
        # コードページ任せにすると、CP932 の端末では「⑴〜⑷b」などが化けて、
        # 本文が一致していても不一致と報告してしまう（実際に踏んだ）。
        $text = Invoke-PythonChild -Python $Python -Arguments (@($ledgerScript, 'goal-text') + $common) -Quiet
        if ($text.Code -ne 0) {
            return [pscustomobject]@{ Ok = $false; Code = $text.Code; Note = 'goal-text が通りません'; Lines = $text.Lines }
        }

        $generated = ((@($text.Out) -join "`n") -replace $SELFTEST_TOKEN, '<token>')
        $mdText    = (Get-Content -LiteralPath $goalMdPath -Raw -Encoding utf8) -replace "`r`n", "`n"
        $marker    = "`n---`n"
        $at = $mdText.IndexOf($marker)
        if ($at -lt 0) {
            return [pscustomobject]@{ Ok = $false; Code = -1; Note = "写しに区切り線（---）がありません: $goalMdPath"; Lines = @() }
        }
        $body = $mdText.Substring($at + $marker.Length)

        # 先頭の空行と末尾の改行だけを無視して比べる（本文は一字も違ってはならない）
        $left  = $generated.Trim()
        $right = $body.Trim()
        if ($left -ceq $right) {
            # 本文が一致していても、写しのヘッダが書いている字数（「本文は 1,012 字」）は
            # テンプレートを直すと黙ってずれる。goal-text 自身が報告した字数と突き合わせる。
            $reported = $null
            $errText = (@($text.Err) -join "`n")
            if ($errText -match '(\d+)\s*文字') { $reported = [int]$Matches[1] }
            $declared = $null
            if ($mdText -match '本文は\s*([\d,]+)\s*字') { $declared = [int](($Matches[1]) -replace ',', '') }
            if ($null -ne $reported -and $null -ne $declared -and $reported -ne $declared) {
                $detail = @(
                    "$goalMdPath のヘッダが書いている字数が goal-text の報告と違います",
                    "  goal-text : $reported 字",
                    "  goal.md   : $declared 字",
                    '  本文は一致しているので、直すのはヘッダの数字だけです。'
                )
                return [pscustomobject]@{ Ok = $false; Code = 1; Note = "本文は一致・ヘッダの字数が違います（$declared ≠ $reported）"; Lines = $detail }
            }
            $note = "本文 $(Format-LoopValue $reported) 字が一致"
            if ($null -ne $declared) { $note += "（写しのヘッダの $declared 字とも一致）" }
            return [pscustomobject]@{ Ok = $true; Code = 0; Note = $note; Lines = @() }
        }

        # 食い違いは黙って通さない。最初に違う行を名指しする。
        $leftLines  = @($left -split "`n")
        $rightLines = @($right -split "`n")
        $maxCount = [Math]::Max($leftLines.Count, $rightLines.Count)
        $diffAt = -1
        for ($i = 0; $i -lt $maxCount; $i++) {
            $a = if ($i -lt $leftLines.Count) { $leftLines[$i] } else { '(行なし)' }
            $b = if ($i -lt $rightLines.Count) { $rightLines[$i] } else { '(行なし)' }
            if ($a -cne $b) { $diffAt = $i; break }
        }
        $detail = @(
            "goal-text の出力と $goalMdPath の本文が違います（$($diffAt + 1) 行目）",
            "  goal-text : $(if ($diffAt -lt $leftLines.Count) { $leftLines[$diffAt] } else { '(行なし)' })",
            "  goal.md   : $(if ($diffAt -lt $rightLines.Count) { $rightLines[$diffAt] } else { '(行なし)' })",
            '  直すのは perf_ledger_goal.py の GOAL_TEXT_TEMPLATE か goals/*.goal.md のどちらか一方です。'
        )
        return [pscustomobject]@{ Ok = $false; Code = 1; Note = "本文が食い違います（$($diffAt + 1) 行目）"; Lines = $detail }
    } finally {
        Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# 全ての道具の自己較正を順に回す。1 つでも赤なら計測失敗（要件 2.11＝黙って続けない）。
# 戻り値: Ok（bool）・OkCount・NgCount・Lines（1 道具 1 行の報告）
function Invoke-SelfTestAll {
    param(
        [Parameter(Mandatory = $true)][string]$GoalName,
        [string]$GoalFilePath
    )
    $report = @()
    $ngCount = 0
    $okCount = 0

    $python = Get-PythonCommand
    if (-not $python) {
        $line = "selftest python NG code=-1 | python が見つかりません（道具の 5 本は python で動きます）"
        Write-Info $line
        return [pscustomobject]@{ Ok = $false; OkCount = 0; NgCount = 1; Lines = @($line) }
    }

    foreach ($tool in $SELFTEST_TOOLS) {
        $path = Join-Path $PSScriptRoot $tool.File
        if ($tool.Kind -eq 'py') {
            $result = Invoke-PythonChild -Python $python -Arguments (@($path) + $tool.Arguments) -Quiet
        } else {
            $result = Invoke-PwshChild -ScriptPath $path -Arguments $tool.Arguments -Quiet
        }
        $ok = ($result.Code -eq 0)
        if ($ok) { $okCount++ } else { $ngCount++ }
        $line = "selftest $($tool.File) $(if ($ok) { 'ok' } else { 'NG' }) code=$($result.Code) | $($result.LastLine)"
        Write-Info $line
        $report += $line
        if (-not $ok) {
            # 赤のときだけ子の出力をそのまま見せる（何が壊れたかを人と台帳へ残すため）
            foreach ($childLine in $result.Lines) { Write-Info "    $childLine" }
        }
    }

    $goalText = Test-GoalTextMatchesGoalMd -Python $python -GoalName $GoalName -GoalFilePath $GoalFilePath
    if ($goalText.Ok) { $okCount++ } else { $ngCount++ }
    $line = "selftest $SELFTEST_GOAL_TEXT_NAME $(if ($goalText.Ok) { 'ok' } else { 'NG' }) code=$($goalText.Code) | $($goalText.Note)"
    Write-Info $line
    $report += $line
    if (-not $goalText.Ok) {
        foreach ($childLine in $goalText.Lines) { Write-Info "    $childLine" }
    }

    $summary = "SELFTEST RESULT ok=$okCount ng=$ngCount"
    Write-Info $summary
    $report += $summary
    return [pscustomobject]@{ Ok = ($ngCount -eq 0); OkCount = $okCount; NgCount = $ngCount; Lines = $report }
}

function Invoke-SubSelfTest {
    Write-Info "[perf-loop] 版 $SCRIPT_VERSION / 自己較正 $($SELFTEST_TOOLS.Count) 道具＋$SELFTEST_GOAL_TEXT_NAME"
    $result = Invoke-SelfTestAll -GoalName $script:GoalName -GoalFilePath $script:GoalFilePath
    if ($result.Ok) { Exit-WithResult -Code $EXIT_OK }
    Stop-Run -Code $EXIT_MEASURE_FAILED -Message "道具の自己較正が $($result.NgCount) 件赤です（計測失敗＝MEASURE_FAILED。直してから回すこと）"
}

# =============================================================================
# 能力確認（preflight）
# =============================================================================

# python の版を「3.13.15」の形で得る（読めなければ $null）。
function Get-PythonVersionText {
    param([pscustomobject]$Python)
    if (-not $Python) { return $null }
    $result = Invoke-Child -Exe $Python.Exe -Arguments (@($Python.Prefix) + @('-c', 'import sys;print("%d.%d.%d" % sys.version_info[:3])')) -Quiet
    if ($result.Code -ne 0) { return $null }
    if ($result.LastLine -match '^\d+\.\d+') { return $result.LastLine }
    return $null
}

# invoke-cpu-sample.ps1 -Probe の 1 行（available=… reason=…）を読む。
function Get-SamplingProbe {
    $path = Join-Path $PSScriptRoot 'invoke-cpu-sample.ps1'
    $result = Invoke-PwshChild -ScriptPath $path -Arguments @('-Probe') -Quiet
    $available = $false
    $reason = 'probe_failed'
    foreach ($line in $result.Lines) {
        if ($line -match '^available=(true|false)(\s+reason=(\S+))?') {
            $available = ($Matches[1] -eq 'true')
            if ($Matches.Count -ge 4 -and $Matches[3]) { $reason = $Matches[3] }
            elseif ($available) { $reason = 'ok' }
        }
    }
    if ($result.Code -ne 0 -and -not $available) {
        # -Probe は必ず 0 で返る約束（design C8）。非ゼロは道具自身の異常。
        $reason = 'probe_failed'
    }
    return [pscustomobject]@{ Available = $available; Reason = $reason; Code = $result.Code; Lines = $result.Lines }
}

function Invoke-SubPreflight {
    $dir = New-LoopDir -Path (Get-LoopDir -Kind goal)
    $script:CurrentDir = $dir

    Write-Info "[perf-loop] 版 $SCRIPT_VERSION / preflight 目標 $($script:GoalName) / 出力先 $dir"

    # --- 環境の見立て ---------------------------------------------------------
    $elevated  = Test-Elevated
    $xperfPath = Find-XperfPath -DefaultPath $XPERF_DEFAULT_PATH
    $pdbPath   = Join-Path $repoRoot $RELEASE_PDB_RELPATH
    $pdbFound  = Test-Path -LiteralPath $pdbPath -PathType Leaf
    $probe     = Get-SamplingProbe

    # 段③（関数別の帰属）の可否。順に見て、最初に足りないものを理由にする。
    $stage = $STAGE_AVAILABLE
    $stageReason = 'ok'
    if (-not $probe.Available) {
        $stage = $STAGE_UNAVAILABLE
        $stageReason = $probe.Reason
    } elseif (-not $pdbFound) {
        $stage = $STAGE_UNAVAILABLE
        $stageReason = 'no_pdb'
    }

    # --- 判定スクリプトの版一致 ----------------------------------------------
    $goalConfig = $script:GoalConfig
    $judgeDeclared = Format-LoopValue $goalConfig.JudgeVersion
    $judgeActual = $null
    if ($goalConfig.JudgeScriptPath) { $judgeActual = Get-JudgeScriptVersion -Path $goalConfig.JudgeScriptPath }
    $judgeMatch = ($judgeActual -and $goalConfig.JudgeVersion -and ($judgeActual -eq $goalConfig.JudgeVersion))

    # --- Python / PowerShell の版 --------------------------------------------
    $python = Get-PythonCommand
    $pythonVersion = Get-PythonVersionText -Python $python
    $pythonOk = $false
    if ($pythonVersion) {
        $parsed = [Version]$pythonVersion
        $pythonOk = ($parsed -ge $PYTHON_MIN_VERSION)
    }
    $pwshVersion = $PSVersionTable.PSVersion.ToString()
    $pwshOk = ($PSVersionTable.PSVersion.Major -ge $PWSH_MIN_MAJOR)

    # --- check-in 間隔の実効値 -----------------------------------------------
    # 環境変数が未設定なら Claude Code の既定 30 分が効いている。25 分未満だと
    # 25 分水準の背景計測に check-in が割り込む（要件 1.11）。
    $checkinRaw = [Environment]::GetEnvironmentVariable($CHECKIN_ENV_NAME)
    $checkinSource = 'default'
    $checkinMinutes = $CHECKIN_DEFAULT_MINUTES
    $checkinNote = ''
    if ($checkinRaw) {
        $parsedCheckin = 0
        if ([int]::TryParse($checkinRaw, [ref]$parsedCheckin) -and $parsedCheckin -gt 0) {
            $checkinMinutes = $parsedCheckin
            $checkinSource = 'env'
        } else {
            $checkinNote = "環境変数 $CHECKIN_ENV_NAME の値 '$checkinRaw' が整数として読めません（既定 $CHECKIN_DEFAULT_MINUTES 分として扱います）"
            $checkinSource = 'default'
        }
    }
    $checkinWarn = ($checkinMinutes -lt $CHECKIN_WARN_BELOW_MINUTES)
    $checkinRecommended = if ($goalConfig.CheckinMinutes) { $goalConfig.CheckinMinutes } else { $CHECKIN_DEFAULT_MINUTES }

    # --- 台帳（あれば版一致まで goal-check で確かめる） ------------------------
    # goal-check は必須キー・判定スクリプトの版・閾値・台帳の goal 名を突き合わせ、
    # 走行トークンが無ければ作る／既にあれば変えない（perf_ledger_goal.py:430-447）。
    # 台帳が無い状態では呼べない（周 0 の手順は init → goal-check → goal-text）ので、
    # -Ledger で明示されたか、目標定義から導いた台帳が実在するときだけ呼ぶ。
    # 実在すれば呼んで構わない——トークンは書き換わらず、まだ無ければここで作られた 1 つを
    # 後の goal-text がそのまま読む（条件文の字面はずれない）。
    $ledgerPath = if ($Ledger) { $Ledger } else { $goalConfig.LedgerPath }
    $ledgerExists = ($ledgerPath -and (Test-Path -LiteralPath $ledgerPath -PathType Leaf))
    $goalCheck = 'skipped'
    $goalCheckNote = '台帳がまだありません（perf-ledger.py init → goal-check → goal-text の順で作ること）'
    if ($ledgerExists) {
        $goalCheckArgs = @((Join-Path $PSScriptRoot 'perf-ledger.py'), 'goal-check', '--ledger', $ledgerPath)
        if ($script:GoalFilePath) { $goalCheckArgs += @('--goal-file', $script:GoalFilePath) }
        else { $goalCheckArgs += @('--goal', $script:GoalName) }
        $result = Invoke-PythonChild -Python $python -Arguments $goalCheckArgs -Quiet
        $goalCheck = if ($result.Code -eq 0) { 'ok' } else { 'ng' }
        $goalCheckNote = $result.LastLine
        if ($goalCheck -eq 'ng') {
            # 何が食い違ったかを黙って落とさない（版・閾値・goal 名のどれかが名指しされる）
            Write-Info "[perf-loop] goal-check が通りません（終了コード $($result.Code)）:"
            foreach ($childLine in $result.Lines) { Write-Info "    $childLine" }
        }
    }

    # --- 道具の自己較正 -------------------------------------------------------
    $selftest = Invoke-SelfTestAll -GoalName $script:GoalName -GoalFilePath $script:GoalFilePath
    $selftestWord = if ($selftest.Ok) { 'ok' } else { 'ng' }

    # --- 合否 -----------------------------------------------------------------
    # 段③が使えないこと（5＝能力不足）は preflight を止めない。止めるのは
    # 「道具が壊れている」＝自己較正の赤と判定スクリプトの版不一致だけ（要件 2.11）。
    $code = $EXIT_OK
    $failReason = ''
    if (-not $judgeMatch) {
        $code = $EXIT_MEASURE_FAILED
        $failReason = "判定スクリプトの版が目標定義と違います（目標定義 $judgeDeclared / $(Format-LoopValue $judgeActual)）"
    } elseif (-not $selftest.Ok) {
        $code = $EXIT_MEASURE_FAILED
        $failReason = "道具の自己較正が $($selftest.NgCount) 件赤です"
    } elseif ($goalCheck -eq 'ng') {
        $code = $EXIT_MEASURE_FAILED
        $failReason = "台帳と目標定義の突合（goal-check）が通りません: $goalCheckNote"
    }
    $verdict = if ($code -eq $EXIT_OK) { $VERDICT_PREFLIGHT_OK } else { $VERDICT_PREFLIGHT_NG }

    # --- 報告（preflight.txt と標準出力へ同じ文面） ---------------------------
    $capabilities = @(
        "elevated:$(Format-LoopBool $elevated)",
        "xperf:$(Format-LoopBool ([bool]$xperfPath))",
        "pdb:$(Format-LoopBool $pdbFound)",
        "function_stage:$stage",
        "reason:$stageReason",
        "judge:$(Format-LoopValue $judgeActual)",
        "python:$(Format-LoopValue $pythonVersion)",
        "pwsh:$pwshVersion",
        "checkin_min:$checkinMinutes",
        "selftest:$selftestWord"
    ) -join ';'

    $lines = @(
        "[perf-loop] version=$SCRIPT_VERSION",
        "sub=$SUB_PREFLIGHT",
        "goal=$($script:GoalName)",
        "goal_file=$($goalConfig.Path)",
        "time_utc=$(Get-UtcStamp)",
        "repo_root=$repoRoot",
        "out_root=$($script:OutRootPath)",
        "out_dir=$dir",
        "elevated=$(Format-LoopBool $elevated)",
        "xperf=$(Format-LoopBool ([bool]$xperfPath))",
        "xperf_path=$(Format-LoopValue $xperfPath)",
        "pdb=$(Format-LoopBool $pdbFound)",
        "pdb_path=$pdbPath",
        "pdb_hint=$PDB_BUILD_HINT",
        "probe_available=$(Format-LoopBool $probe.Available)",
        "probe_reason=$($probe.Reason)",
        "function_stage=$stage reason=$stageReason",
        "judge_script=$(Format-LoopValue $goalConfig.JudgeScriptPath)",
        "judge_version_goal=$judgeDeclared",
        "judge_version_script=$(Format-LoopValue $judgeActual)",
        "judge_version_match=$(Format-LoopBool $judgeMatch)",
        "python=$(Format-LoopValue $pythonVersion) min=$($PYTHON_MIN_VERSION.ToString()) ok=$(Format-LoopBool $pythonOk)",
        "pwsh=$pwshVersion min=$PWSH_MIN_MAJOR ok=$(Format-LoopBool $pwshOk)",
        "checkin_minutes=$checkinMinutes source=$checkinSource recommended=$checkinRecommended warn_below=$CHECKIN_WARN_BELOW_MINUTES",
        "checkin_warn=$(Format-LoopBool $checkinWarn)",
        "ledger=$(Format-LoopValue $ledgerPath)",
        "ledger_exists=$(Format-LoopBool $ledgerExists)",
        "goal_check=$goalCheck",
        "selftest=$selftestWord ok=$($selftest.OkCount) ng=$($selftest.NgCount)",
        "sampling_backend=$(Format-LoopValue $goalConfig.SamplingBackend)",
        "capabilities=$capabilities",
        "verdict=$verdict",
        "code=$code"
    )
    if ($checkinNote) { $lines += "checkin_note=$checkinNote" }
    if ($failReason) { $lines += "fail_reason=$failReason" }

    $text = ($lines -join "`n") + "`n"
    $reportPath = Join-Path $dir $PREFLIGHT_FILE
    Set-Content -LiteralPath $reportPath -Value $text -Encoding utf8 -NoNewline
    foreach ($line in $lines) { Write-Info $line }
    Write-Info "preflight_file=$reportPath"

    # 黙って続けない——人が読む語でも一度書く。
    if ($stage -eq $STAGE_UNAVAILABLE) {
        Write-Info "[perf-loop] 段③（関数別の帰属）は使えません: reason=$stageReason。段①②④で続行します（停止の理由にしません）。"
    }
    if ($checkinWarn) {
        Write-Problem "[perf-loop] 警告: check-in 間隔の実効値が $checkinMinutes 分です（$CHECKIN_WARN_BELOW_MINUTES 分未満）。25 分水準の背景計測に割り込みます。$CHECKIN_ENV_NAME=$checkinRecommended を勧めます。"
    }
    if ($checkinNote) { Write-Problem "[perf-loop] 警告: $checkinNote" }

    if ($code -ne $EXIT_OK) { Stop-Run -Code $code -Message $failReason }
    Exit-WithResult -Code $EXIT_OK
}

# =============================================================================
# 引数の検証
# =============================================================================
if (-not $Sub) {
    Write-Problem "[perf-loop] サブコマンドがありません。使えるのは: $($SUBCOMMANDS -join '・')"
    Stop-Run -Code $EXIT_BAD_ARGS -Message 'サブコマンドを 1 つ指定してください' -Sub '-'
}
if ($SUBCOMMANDS -notcontains $Sub) {
    Write-Problem "[perf-loop] 使えるサブコマンド: $($SUBCOMMANDS -join '・')"
    Stop-Run -Code $EXIT_BAD_ARGS -Message "未知のサブコマンドです: '$Sub'" -Sub '-'
}
if ($BUILD_KINDS -notcontains $Build) {
    Stop-Run -Code $EXIT_BAD_ARGS -Message "-Build は $($BUILD_KINDS -join ' か ') です（受け取った値: '$Build'）"
}
if ($Iter -lt 0) {
    Stop-Run -Code $EXIT_BAD_ARGS -Message "-Iter は 0 以上です（受け取った値: $Iter）"
}
if ($Date -and $Date -notmatch '^\d{8}$') {
    Stop-Run -Code $EXIT_BAD_ARGS -Message "-Date は yyyyMMdd の 8 桁です（受け取った値: '$Date'）"
}

$script:OutRootPath = Resolve-OutRoot -OutRoot $OutRoot
if (-not $script:OutRootPath) {
    Stop-Run -Code $EXIT_BAD_ARGS -Message '出力先の根が決まりません（%LOCALAPPDATA% も %TEMP% も無いので -OutRoot を指定してください）'
}

# 目標定義。selftest は道具の較正が主目的なので、目標定義ファイルが無くても
# そこで止めず「goal-text の検査が赤」として 4 で返す（道具の不具合と同じ扱い）。
$script:GoalFilePath = $null
$script:GoalConfig = $null
$goalCandidate = Resolve-GoalFile -Goal $script:GoalName -GoalFile $GoalFile -ScriptRoot $PSScriptRoot
if (Test-Path -LiteralPath $goalCandidate -PathType Leaf) {
    $script:GoalFilePath = [System.IO.Path]::GetFullPath($goalCandidate)
    $script:GoalConfig = Get-GoalConfig -GoalFilePath $script:GoalFilePath -RepoRoot $repoRoot
    if ($script:GoalConfig.Name -and $script:GoalConfig.Name -ne $script:GoalName) {
        # -GoalFile で別名のファイルを渡したときは、出力先は定義側の名前に従う
        $script:GoalName = $script:GoalConfig.Name
    }
} elseif ($Sub -ne $SUB_SELFTEST) {
    Stop-Run -Code $EXIT_BAD_ARGS -Message "目標定義ファイルがありません: $goalCandidate（-Goal か -GoalFile を確かめてください）"
}

# =============================================================================
# 振り分け（入口の 2 本は本ファイル・測る 8 本は perf-loop.measure.ps1）
# =============================================================================
switch ($Sub) {
    'preflight'        { Invoke-SubPreflight }
    'selftest'         { Invoke-SubSelfTest }
    'measure-baseline' { Invoke-SubMeasureBaseline }
    'rank-run'         { Invoke-SubRankRun }
    'rank'             { Invoke-SubRank }
    'prepare-ab'       { Invoke-SubPrepareAb }
    'measure-ab'       { Invoke-SubMeasureAb }
    'compare'          { Invoke-SubCompare }
    'followup'         { Invoke-SubFollowup }
    'final'            { Invoke-SubFinal }
    default            { Stop-Run -Code $EXIT_BAD_ARGS -Message "未知のサブコマンドです: '$Sub'" -Sub '-' }
}

# ここへは来ない（各サブコマンドは必ず Exit-WithResult で終わる）。来たら道具の不具合。
Stop-Run -Code $EXIT_MEASURE_FAILED -Message "サブコマンド '$Sub' が終了コードを返しませんでした（道具の不具合）"
