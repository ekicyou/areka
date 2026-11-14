#Requires -Version 7.0
<#
================================================================================
perf-tool-base.ps1 — tools/perf の道具どうしの共通部品
  spec: audit-tools-perf-run（直し 5・7・8・17・18・19・35）

読み込み用（. で読み込む）。単独で回しても何もしない。
    . (Join-Path $PSScriptRoot 'perf-tool-base.ps1')

置くのは関数と定数だけ。読み込んでも何も走らない。Set-StrictMode を自分で切り替えず、
呼び手の変数に頼らない（所在は自分の $PSScriptRoot＝tools/perf から決める）。
端末へ出す文の字句は ASCII だけで、端末の文字コードには触れない。

  表示と時刻     … Write-Info / Write-Problem / Get-UtcStamp（末尾に Z）/ Format-UnexpectedError（trap の理由の文）
  子の起こし方   … Invoke-Child / Get-PwshPath / Get-PythonCommand / Invoke-PythonChild / Invoke-PwshChild
                   子の出力は既定で BOM の無い UTF-8 として読む（-ReadEncoding で選べる）
  強制終了       … Stop-ChildProcess（子孫まで止めて、終わるまで待つ）
  目標定義の読み … Read-TomlSections / Get-TomlString / Get-TomlInt / Get-TomlDouble / Get-TomlStringArray
  key=value      … ConvertFrom-KeyValueLines
  検体の在り処   … Get-NarSamplePath（止めない。止め方は呼び手が戻り値を見て決める）
================================================================================
#>

# 子の PowerShell の引数を UTF-8 の書き手の前置き付きで組む Get-PwshChildArguments（前置きの唯一の所在）
. (Join-Path $PSScriptRoot 'perf-pwsh-child.ps1')

#: 子の出力を読む既定の文字符号（端末の文字コードに依らない。BOM 無し）。
$PERF_TOOL_CHILD_ENCODING = [System.Text.UTF8Encoding]::new($false)

#: 値が無いことを表す字面（RESULT 行・台帳と同じ）。
$PERF_TOOL_EMPTY = '-'

#: リポジトリの根（tools/perf の 2 つ上）。
$PERF_TOOL_REPO_ROOT = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))

#: 色の符号（ESC [ … 終端字）。「最後の 1 行」を選ぶ前に落とす。
$PERF_TOOL_ANSI_PATTERN = '\x1b\[[0-?]*[ -/]*[@-~]'

#: 検体の窓口の表（1 検体 1 回だけ呼ぶ）。
$script:PerfToolNarSampleMaps = @{}

# =============================================================================
# 表示と時刻
# =============================================================================

function Write-Info    { param([string]$Message) [Console]::Out.WriteLine($Message) }
function Write-Problem { param([string]$Message) [Console]::Error.WriteLine($Message) }

function Get-UtcStamp {
    [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.fffZ', [Globalization.CultureInfo]::InvariantCulture)
}

# 各入口の trap が出す理由の文: '<Label>: <例外の文を 1 行に> (<スクリプト名>:<行番号>)'
function Format-UnexpectedError {
    param([Parameter(Mandatory = $true)][System.Management.Automation.ErrorRecord]$ErrorRecord, [string]$Label = 'unexpected failure')
    $info = $ErrorRecord.InvocationInfo
    $where = if ($info -and $info.ScriptName) { "$([System.IO.Path]::GetFileName($info.ScriptName)):$($info.ScriptLineNumber)" } else { $PERF_TOOL_EMPTY }
    return "${Label}: $($ErrorRecord.Exception.Message -replace '\s*\r?\n\s*', ' ') ($where)"
}

# =============================================================================
# 子の起こし方
# =============================================================================

function New-ChildResult {
    param([int]$Code, [string[]]$Out = @(), [string[]]$Err = @(), [string]$LastLine)
    return [pscustomobject]@{ Code = $Code; Out = @($Out); Err = @($Err); Lines = @($Out) + @($Err); LastLine = $LastLine }
}

# 子を起こして出力と終了コードを持ち帰る。引数は ArgumentList に 1 つずつ（空白で割れない）。
# 終了コードは握り潰さない。起こせないとき Code=-1。
# LastLine は色の符号を落としてから、罫線（= - _ * だけの行）でない最後の行。
function Invoke-Child {
    param(
        [Parameter(Mandatory = $true)][string]$Exe,
        [string[]]$Arguments = @(),
        [hashtable]$Environment,
        [switch]$Quiet,
        [System.Text.Encoding]$ReadEncoding   # 省けば BOM の無い UTF-8
    )
    if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
        $msg = "executable not found: $Exe"
        return New-ChildResult -Code -1 -Err @($msg) -LastLine $msg
    }
    $encoding = if ($ReadEncoding) { $ReadEncoding } else { $PERF_TOOL_CHILD_ENCODING }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName               = $Exe
    $startInfo.UseShellExecute        = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError  = $true
    $startInfo.StandardOutputEncoding = $encoding
    $startInfo.StandardErrorEncoding  = $encoding
    $startInfo.WorkingDirectory       = (Get-Location).Path
    foreach ($argument in $Arguments) { $startInfo.ArgumentList.Add([string]$argument) }
    if ($Environment) {
        foreach ($key in $Environment.Keys) { $startInfo.Environment[[string]$key] = [string]$Environment[$key] }
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    try {
        $null = $process.Start()
        # 先に非同期で読み始める（バッファが埋まって子が止まるのを避ける）
        $outTask = $process.StandardOutput.ReadToEndAsync()
        $errTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $outText = $outTask.GetAwaiter().GetResult()
        $errText = $errTask.GetAwaiter().GetResult()
        $code = $process.ExitCode
    } catch {
        $msg = "cannot start: $($_.Exception.Message)"
        return New-ChildResult -Code -1 -Err @($msg) -LastLine $msg
    } finally {
        $process.Dispose()
    }

    $outLines = @()
    if ($outText) { $outLines = @(($outText -replace "`r`n", "`n").TrimEnd("`n") -split "`n") }
    $errLines = @()
    if ($errText) { $errLines = @(($errText -replace "`r`n", "`n").TrimEnd("`n") -split "`n") }
    $lines = @($outLines) + @($errLines)
    if ($null -eq $code) { $code = -1 }
    if (-not $Quiet) { foreach ($line in $lines) { Write-Info $line } }
    $last = $PERF_TOOL_EMPTY
    for ($i = $lines.Count - 1; $i -ge 0; $i--) {
        if (-not $lines[$i]) { continue }
        $trimmed = ($lines[$i] -replace $PERF_TOOL_ANSI_PATTERN, '').Trim()
        if ($trimmed -eq '') { continue }
        if ($trimmed -match '^[=\-_*\s]+$') { continue }
        $last = $trimmed
        break
    }
    return [pscustomobject]@{ Code = [int]$code; Out = $outLines; Err = $errLines; Lines = $lines; LastLine = $last }
}

# 自分と同じ pwsh の在り処（無ければ $null）。子の .ps1 は必ず別プロセスで回す。
function Get-PwshPath {
    if ($PSHOME) {
        $candidate = Join-Path $PSHOME 'pwsh.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    $found = Get-Command 'pwsh' -ErrorAction SilentlyContinue
    if ($found) { return $found.Source }
    return $null
}

# python の在り処。単一要素の配列を返すと PowerShell が文字列へ展開するため、
# 実行体と前置引数を別の項として持つオブジェクトで返す（無ければ $null）。
function Get-PythonCommand {
    foreach ($candidate in @('python', 'python3')) {
        $found = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($found) { return [pscustomobject]@{ Exe = $found.Source; Prefix = @() } }
    }
    $py = Get-Command 'py' -ErrorAction SilentlyContinue
    if ($py) { return [pscustomobject]@{ Exe = $py.Source; Prefix = @('-3') } }
    return $null
}

# python スクリプトを回す。PYTHONIOENCODING／PYTHONUTF8 で子が必ず UTF-8 で書くようにする。
function Invoke-PythonChild {
    param(
        [Parameter(Mandatory = $true)][pscustomobject]$Python,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [switch]$Quiet
    )
    $argv = @($Python.Prefix) + $Arguments
    return Invoke-Child -Exe $Python.Exe -Arguments $argv -Quiet:$Quiet `
        -Environment @{ PYTHONIOENCODING = 'utf-8'; PYTHONUTF8 = '1' }
}

# .ps1 を子の pwsh で回す。引数は Get-PwshChildArguments（UTF-8 の書き手の前置き付き）で組む。
function Invoke-PwshChild {
    param(
        [Parameter(Mandatory = $true)][string]$ScriptPath,
        [string[]]$Arguments = @(),
        [switch]$Quiet
    )
    $pwshPath = Get-PwshPath
    if (-not $pwshPath) {
        return New-ChildResult -Code -1 -Err @('pwsh not found') -LastLine 'pwsh not found'
    }
    if (-not (Test-Path -LiteralPath $ScriptPath -PathType Leaf)) {
        $msg = "script not found: $ScriptPath"
        return New-ChildResult -Code -1 -Err @($msg) -LastLine $msg
    }
    $argv = Get-PwshChildArguments -ScriptPath $ScriptPath -Arguments $Arguments
    return Invoke-Child -Exe $pwshPath -Arguments $argv -Quiet:$Quiet
}

# =============================================================================
# 強制終了
# =============================================================================

# 子孫まで強制終了して（Kill($true)＝32bit の SHIORI ヘルパが run.log を受け継いでいても止める）、
# 子と子孫が全部終わるまで待つ（孫が握ったファイルが返った後も残らないように）。
# 戻り: 全部終わったら $true・時間切れなら $false（$null や既に終わったものは $true）。
function Stop-ChildProcess {
    param([System.Diagnostics.Process]$Process, [int]$WaitMs = 10000)
    if (-not $Process) { return $true }
    $deadline = [DateTime]::UtcNow.AddMilliseconds($WaitMs)
    $tree = @($Process)
    try {
        if ($Process.HasExited) { return $true }
    } catch { $null = $_ }
    # 一覧が取れなくても子は止める（待つ相手が子だけに減ることを 1 行で知らせる）
    try { $tree += @(Get-DescendantProcess -Process $Process) }
    catch { Write-Problem "[perf-tool-base] descendant snapshot failed: $($_.Exception.Message)" }
    try {
        $Process.Kill($true)
    } catch {
        # 止める間に自分で終わった（または止める権限が無い）。終わったかは下の待ちで決める。
        $null = $_
    }
    $allExited = $true
    foreach ($p in $tree) {
        $left = [int][Math]::Max(0, ($deadline - [DateTime]::UtcNow).TotalMilliseconds)
        try { if (-not $p.WaitForExit($left)) { $allExited = $false } } catch { $allExited = $false }
    }
    return $allExited
}

# 子孫の Process を止める前に集める。各々の Handle を開いておき、終わった後に PID が使い回されても別物を待たない。
# 親より先に生まれたものは、親の PID が使い回される前の古い親子関係なので外す。
function Get-DescendantProcess {
    param([System.Diagnostics.Process]$Process)
    $all = @(Get-CimInstance -ClassName Win32_Process -Property ProcessId, ParentProcessId, CreationDate -ErrorAction Stop)
    $found = @()
    $queue = [System.Collections.Generic.Queue[object]]::new()
    $queue.Enqueue([pscustomobject]@{ Id = $Process.Id; Start = $Process.StartTime })
    while ($queue.Count -gt 0) {
        $parent = $queue.Dequeue()
        foreach ($c in $all) {
            if ($c.ParentProcessId -ne $parent.Id -or $c.ProcessId -eq $parent.Id -or $c.CreationDate -lt $parent.Start) { continue }
            try {
                $p = [System.Diagnostics.Process]::GetProcessById([int]$c.ProcessId)
                $null = $p.Handle
                # 一覧を取った後で PID が使い回された別物は外す（CIM の時刻はマイクロ秒止まりなので 1ms の幅で比べる）
                if ([Math]::Abs(($p.StartTime - $c.CreationDate).TotalMilliseconds) -gt 1) { $p.Dispose(); continue }
                $found += $p
                $queue.Enqueue([pscustomobject]@{ Id = $p.Id; Start = $c.CreationDate })
            } catch { $null = $_ }
        }
    }
    return $found
}

# =============================================================================
# 目標定義（TOML）の読み
# =============================================================================

# 「節 → 鍵 → 値（引用符を外す前の字面）」の入れ子の表へ。外部モジュールを使わない。
# 解するのは 1 行に収まった値と 1 行の配列だけ（目標定義ファイルはこの制約で書く）。
# 節の見出しの見方は 1 通り。注釈の # は二重引用符と単引用符の外だけで落とす。
function Read-TomlSections {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Text)
    $result = @{}
    $section = ''
    foreach ($raw in ($Text -split "`r?`n")) {
        $line = $raw.Trim()
        if ($line -eq '' -or $line.StartsWith('#')) { continue }
        if ($line -match '^\[([^\]]+)\]\s*$') {
            $section = $Matches[1].Trim()
            if (-not $result.ContainsKey($section)) { $result[$section] = @{} }
            continue
        }
        if ($section -eq '') { continue }
        if ($line -notmatch '^([A-Za-z0-9_-]+)\s*=\s*(.+)$') { continue }
        $key = $Matches[1]
        $val = ($Matches[2]).Trim()

        $quote = [char]0   # 今いる文字列の引用符（0 なら文字列の外）
        $cut = -1
        for ($i = 0; $i -lt $val.Length; $i++) {
            $ch = $val[$i]
            if ($quote -eq [char]0) {
                if ($ch -eq '"' -or $ch -eq "'") { $quote = $ch }
                elseif ($ch -eq '#') { $cut = $i; break }
            } elseif ($quote -eq '"' -and $ch -eq '\') {
                $i++   # 二重引用符の中の逃がし（\" を閉じと読まない）。単引用符の中に逃がしは無い
            } elseif ($ch -eq $quote) {
                $quote = [char]0
            }
        }
        if ($cut -ge 0) { $val = $val.Substring(0, $cut).Trim() }
        $result[$section][$key] = $val
    }
    return $result
}

# 節・鍵の字面（無ければ $null）
function Get-TomlRaw {
    param([hashtable]$Toml, [string]$Section, [string]$Key)
    if (-not $Toml -or -not $Toml.ContainsKey($Section)) { return $null }
    if (-not $Toml[$Section].ContainsKey($Key)) { return $null }
    return [string]$Toml[$Section][$Key]
}

# 文字列で引く。二重引用符か単引用符で括られていれば外す。無ければ $null。
# shortcut: 二重引用符の中の逃がし（\n・\" など）は解かない。逃がしを使う値が要るようになったら解く。
function Get-TomlString {
    param([hashtable]$Toml, [string]$Section, [string]$Key)
    $val = Get-TomlRaw -Toml $Toml -Section $Section -Key $Key
    if ($null -eq $val) { return $null }
    if ($val.Length -ge 2 -and (($val.StartsWith('"') -and $val.EndsWith('"')) -or ($val.StartsWith("'") -and $val.EndsWith("'")))) {
        return $val.Substring(1, $val.Length - 2)
    }
    return $val
}

# 整数で引く。無い・読めなければ $null。
function Get-TomlInt {
    param([hashtable]$Toml, [string]$Section, [string]$Key)
    $raw = Get-TomlRaw -Toml $Toml -Section $Section -Key $Key
    if ($null -eq $raw) { return $null }
    $parsed = 0
    if ([int]::TryParse($raw.Replace('_', ''), [Globalization.NumberStyles]::AllowLeadingSign,
            [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)) { return $parsed }
    return $null
}

# 小数で引く（整数の字面も読む）。無い・読めなければ $null。
function Get-TomlDouble {
    param([hashtable]$Toml, [string]$Section, [string]$Key)
    $raw = Get-TomlRaw -Toml $Toml -Section $Section -Key $Key
    if ($null -eq $raw) { return $null }
    $parsed = 0.0
    $styles = [Globalization.NumberStyles]::AllowLeadingSign -bor [Globalization.NumberStyles]::AllowDecimalPoint -bor [Globalization.NumberStyles]::AllowExponent
    if ([double]::TryParse($raw.Replace('_', ''), $styles, [Globalization.CultureInfo]::InvariantCulture, [ref]$parsed)) { return $parsed }
    return $null
}

# 1 行の文字列の配列（["a", 'b']）を [string[]] で引く。'[]' は空の配列。無い・読めなければ $null。
# 要素の中の ',' は引用符ごと読むので割れない。
function Get-TomlStringArray {
    param([hashtable]$Toml, [string]$Section, [string]$Key)
    $raw = Get-TomlRaw -Toml $Toml -Section $Section -Key $Key
    if ($null -eq $raw) { return $null }
    $item = '(?:"[^"]*"|''[^'']*'')'
    if ($raw -notmatch ('^\[\s*(?:' + $item + '(?:\s*,\s*' + $item + ')*\s*,?)?\s*\]$')) { return $null }
    $items = [System.Collections.Generic.List[string]]::new()
    foreach ($m in [regex]::Matches($raw, $item)) { $items.Add($m.Value.Substring(1, $m.Value.Length - 2)) }
    return , $items.ToArray()
}

# =============================================================================
# key=value の行と検体の在り処
# =============================================================================

# 'key=value' の行を表へ（最初の '=' だけで割る＝値は Windows の絶対パスで、後ろの '=' を壊さない）
function ConvertFrom-KeyValueLines {
    param([AllowEmptyCollection()][AllowNull()][string[]]$Lines)
    $map = @{}
    foreach ($line in @($Lines)) {
        $text = "$line".Trim()
        $sep = $text.IndexOf('=')
        if ($sep -lt 1) { continue }
        $map[$text.Substring(0, $sep)] = $text.Substring($sep + 1)
    }
    return $map
}

# 検体の窓口（cargo run -p sample-ghost-kit --bin nar-sample-path -- <名前>）を 1 検体 1 回だけ呼び、
# 鍵の値を返す。止めない。戻り: [pscustomobject]@{ Ok; Value; Message }
# 窓口は呼ぶたびに検体を展開し直すので、表は取っておく。同梱バルーンを持たない検体では
# 'balloon.<名前>=' の行そのものが出ない＝鍵が無いのは Ok=$false（空のパスで進ませない）。
function Get-NarSamplePath {
    param([Parameter(Mandatory = $true)][string]$Sample, [Parameter(Mandatory = $true)][string]$Key)
    if (-not $script:PerfToolNarSampleMaps.ContainsKey($Sample)) {
        $cargo = Get-Command 'cargo' -ErrorAction SilentlyContinue
        if (-not $cargo) {
            return [pscustomobject]@{ Ok = $false; Value = $null; Message = 'cargo not found (cannot ask where the sample is)' }
        }
        $cargoArgs = @('run', '-q', '-p', 'sample-ghost-kit', '--bin', 'nar-sample-path',
                       '--manifest-path', (Join-Path $PERF_TOOL_REPO_ROOT 'Cargo.toml'), '--', $Sample)
        $r = Invoke-Child -Exe $cargo.Source -Arguments $cargoArgs -Quiet
        if ($r.Code -ne 0) {
            return [pscustomobject]@{ Ok = $false; Value = $null
                Message = "sample lookup failed (exit code $($r.Code), cargo $($cargoArgs -join ' ')): $($r.LastLine)" }
        }
        $script:PerfToolNarSampleMaps[$Sample] = ConvertFrom-KeyValueLines -Lines $r.Out
    }
    $map = $script:PerfToolNarSampleMaps[$Sample]
    if (-not $map.ContainsKey($Key) -or -not $map[$Key]) {
        return [pscustomobject]@{ Ok = $false; Value = $null
            Message = "sample lookup output has no '$Key=' line (sample '$Sample', keys found: $(($map.Keys | Sort-Object) -join ', '))" }
    }
    return [pscustomobject]@{ Ok = $true; Value = $map[$Key]; Message = '' }
}
