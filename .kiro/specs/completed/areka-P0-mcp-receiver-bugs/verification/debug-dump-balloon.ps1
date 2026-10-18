# debug-dump-balloon.ps1 -- areka-P0-mcp-receiver-bugs 件 2 と要件 2.7 の実機の確かめ（手で回す台本）。
#
# areka のコードでもテストでもない（全体テスト・配布に入らない・要件 3.2）。
# 起こし方（PowerShell から・絶対パス・どのフォルダからでも動く）:
#   pwsh -File <worktree>\.kiro\specs\areka-P0-mcp-receiver-bugs\verification\debug-dump-balloon.ps1 [-Ghost claudia,emo2] [-Calls 5] [-Tool dump_balloon]
#   -Tool dump_surface は要件 3.6 のときだけ使う。
# 置き場はすべてワークツリーの target\ の下（要件 3.8）:
#   target\mcp-receiver-bugs\build.log           建てるときの出力
#   target\mcp-receiver-bugs\<ghost>\            ゴーストごとの作業の置き場（毎回作り直す）
#     profile\ bridge-profile\ areka-stdout.log areka-stderr.log bridge-stderr.log results.json
# 終了コード: 0 = 測る回がすべて 絵 か 断り で絵が 1 回以上
#             1 = 上限の答え・中継の時間切れ・返らなかった が 1 回でもある（待ちの回・2.7 の回も含む）
#             2 = 条件を作れなかった（引数・建てる・検体・起動の失敗・60 秒で絵が出ない・測る回の絵が 0 回）
#   ゴーストが複数のときは 1 があれば 1、無くて 2 があれば 2、ほかは 0。
# 端末へ出す文は ASCII だけ。

param(
    [string[]]$Ghost = @('claudia', 'emo2'),
    [int]$Calls = 5,
    [string]$Tool = 'dump_balloon'
)

$ErrorActionPreference = 'Stop'

# 想定外の例外は ASCII の 1 行を出して 2 で終わる（1 は上限の答え・中継の時間切れ・返らなかった だけに取っておく）。
trap {
    Write-Host ("error: unexpected: " + ("$_" -replace '[^\x20-\x7E]', '?'))
    exit 2
}

# 環境の変数を書く。$null は空にせず消す（PowerShell は string の引数へ $null を "" で渡すため NullString を使う）。
function Set-EnvVar([string]$name, $value) {
    if ($null -eq $value) { [Environment]::SetEnvironmentVariable($name, [NullString]::Value) }
    else { [Environment]::SetEnvironmentVariable($name, [string]$value) }
}

# ゴースト -> バルーン（nar-sample-path の balloon.<名>= の名）。
$Balloons = @{ claudia = 'claudia'; emo2 = 'emo2-kakukaku' }
$Tools = @('dump_balloon', 'dump_surface')

$ReadWaitMs = 40000      # 答えの行を待つ上限（中継の 30 秒より長く）
$FirstPictureMs = 60000  # バルーンが出るのを待つ上限
$AutoExitMs = 360000     # AREKA_APP_SMOKE_EXIT_MS
$DeadlineMs = 420000     # 自動終了 + 60 秒（areka の起動から数える）

function Say([string]$text) {
    # 端末へは ASCII だけ。
    Write-Host ($text -replace '[^\x20-\x7E]', '?')
}

# ---- 引数を建てるより前に確かめる ----
$ghosts = @($Ghost | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
if ($ghosts.Count -eq 0) { Say 'error: -Ghost is empty (known: claudia, emo2)'; exit 2 }
foreach ($g in $ghosts) {
    if (-not $Balloons.ContainsKey($g)) { Say "error: unknown -Ghost '$g' (known: claudia, emo2)"; exit 2 }
}
if ($Tools -notcontains $Tool) { Say "error: unknown -Tool '$Tool' (known: dump_balloon, dump_surface)"; exit 2 }
if ($Calls -lt 1) { Say "error: -Calls must be 1 or more (got $Calls)"; exit 2 }

# ---- ワークツリーの根は台本の場所から ----
$top = & git -C $PSScriptRoot rev-parse --show-toplevel 2>$null
if ($LASTEXITCODE -ne 0 -or -not $top) { Say "error: not inside a git worktree: $PSScriptRoot"; exit 2 }
$Root = [IO.Path]::GetFullPath($top)
$Target = Join-Path $Root 'target'
$Manifest = Join-Path $Root 'Cargo.toml'
$RunBase = Join-Path $Target 'mcp-receiver-bugs'
New-Item -ItemType Directory -Force $RunBase | Out-Null
$BuildLog = Join-Path $RunBase 'build.log'
Set-Content -Path $BuildLog -Value '' -NoNewline

# ---- 建てる（debug の areka と中継、続けて i686 の helper）----
Say "build: areka + areka-mcp-bridge (debug), log: $BuildLog"
& cargo build --manifest-path $Manifest -p areka -p areka-mcp-bridge *>> $BuildLog
if ($LASTEXITCODE -ne 0) { Say "error: cargo build (areka, bridge) failed, see $BuildLog"; exit 2 }
Say 'build: shiori-host32-helper (i686 debug)'
& cargo build --manifest-path $Manifest -p shiori-host32-helper --target i686-pc-windows-msvc *>> $BuildLog
if ($LASTEXITCODE -ne 0) { Say "error: cargo build (i686 helper) failed, see $BuildLog"; exit 2 }

$Debug = Join-Path $Target 'debug'
$ArekaExe = Join-Path $Debug 'areka.exe'
$BridgeExe = Join-Path $Debug 'areka-mcp-bridge.exe'
foreach ($p in @($ArekaExe, $BridgeExe)) {
    if (-not (Test-Path $p)) { Say "error: built exe not found: $p"; exit 2 }
}

function Test-I686Pe([string]$path) {
    if (-not (Test-Path $path)) { return $false }
    $b = [IO.File]::ReadAllBytes($path)
    if ($b.Length -lt 0x40) { return $false }
    $pe = [BitConverter]::ToInt32($b, 0x3c)
    if ($pe -lt 0 -or $pe + 6 -gt $b.Length) { return $false }
    return [BitConverter]::ToUInt16($b, $pe + 4) -eq 0x014c
}

# i686 の helper を areka.exe の隣へ最後に写す（x64 版が隣に残る罠を避ける）。
$HelperSrc = Join-Path $Target 'i686-pc-windows-msvc\debug\shiori-host32-helper.exe'
$HelperDst = Join-Path $Debug 'shiori-host32-helper.exe'
try { Copy-Item -Force $HelperSrc $HelperDst } catch { Say "error: cannot copy i686 helper to $HelperDst"; exit 2 }
if (-not (Test-I686Pe $HelperDst)) { Say "error: helper beside areka.exe is not i386 (0x014c): $HelperDst"; exit 2 }
Say "helper: i686 (0x014c) beside areka.exe: $HelperDst"

function Get-FreePort {
    $l = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    $l.Start(); $port = $l.LocalEndpoint.Port; $l.Stop()
    return $port
}

function Test-Listening([int]$port) {
    $c = [Net.Sockets.TcpClient]::new()
    try { return $c.ConnectAsync('127.0.0.1', $port).Wait(500) -and $c.Connected } catch { return $false } finally { $c.Dispose() }
}

# 中継の標準出力から、id の合う答えの行を待つ。読みかけの非同期の読みは次の呼び出しへ持ち越す。
$script:pending = $null
function Read-Answer($proc, [int]$id, [int]$timeoutMs) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($true) {
        if ($null -eq $script:pending) { $script:pending = $proc.StandardOutput.ReadLineAsync() }
        $left = $timeoutMs - [int]$sw.ElapsedMilliseconds
        if ($left -le 0 -or -not $script:pending.Wait($left)) { return @{ Eof = $false; Json = $null } }
        $line = $script:pending.Result
        $script:pending = $null
        if ($null -eq $line) { return @{ Eof = $true; Json = $null } }
        try { $j = $line | ConvertFrom-Json -Depth 64 } catch { continue }
        if ($j.PSObject.Properties['id'] -and "$($j.id)" -eq "$id") { return @{ Eof = $false; Json = $j } }
    }
}

# 答えを 5 つ（と JSON-RPC のエラー・その他）に分ける。
function Get-Class($ans) {
    $r = [ordered]@{ class = ''; data_length = $null; reason = $null; code = $null; message = $null }
    if ($ans.Eof) { $r.class = 'no_return'; $r.reason = 'bridge stdout closed'; return $r }
    if ($null -eq $ans.Json) { $r.class = 'no_return'; $r.reason = "no line within $($ReadWaitMs / 1000) s"; return $r }
    $j = $ans.Json
    if ($j.PSObject.Properties['error']) {
        $r.code = $j.error.code; $r.message = [string]$j.error.message
        $r.class = if ($r.code -eq -1 -and $r.message.StartsWith('No response from server')) { 'bridge_timeout' } else { 'error' }
        return $r
    }
    $content = @($j.result.content | Where-Object { $_ })
    $img = $content | Where-Object { $_.type -eq 'image' } | Select-Object -First 1
    $text = ($content | Where-Object { $_.type -eq 'text' } | Select-Object -First 1).text
    if ($img) { $r.class = 'image'; $r.data_length = ([string]$img.data).Length; $r.reason = $text }
    elseif ($text -match 'did not respond within') { $r.class = 'timeout_answer'; $r.reason = $text }
    elseif ($j.result.isError) { $r.class = 'refusal'; $r.reason = $text }
    else { $r.class = 'other'; $r.reason = $text }
    return $r
}

function New-Record($n, $phase, $tool, $ms, $c) {
    $rec = [ordered]@{ n = $n; phase = $phase; tool = $tool; ms = $ms }
    foreach ($k in $c.Keys) { $rec[$k] = $c[$k] }
    return $rec
}

function Invoke-Ghost([string]$g) {
    $balloonName = $Balloons[$g]
    $work = Join-Path $RunBase $g
    $result = [ordered]@{
        ghost = $g; balloon = $balloonName; tool = $Tool; calls = $Calls; port = $null
        ghost_folder = $null; balloon_folder = $null
        limits = [ordered]@{ receiver_s = 10; bridge_s = 30; read_s = $ReadWaitMs / 1000; first_picture_s = $FirstPictureMs / 1000; auto_exit_ms = $AutoExitMs; deadline_ms = $DeadlineMs }
        records = @(); measured_images = 0; areka_exit = $null; bridge_exit = $null; killed = @(); exit_code = 2; note = $null
    }
    $resultsPath = Join-Path $work 'results.json'
    $save = { $result | ConvertTo-Json -Depth 8 | Set-Content -Path $resultsPath -Encoding utf8 }
    $fail = { param($why) $result.note = $why; Say "$g error: $why"; & $save; return 2 }

    # 検体（nar-sample-path は毎回作り直す）。
    $lines = & cargo run -q --manifest-path $Manifest -p sample-ghost-kit --bin nar-sample-path -- $g 2>> $BuildLog
    $sampleOk = $LASTEXITCODE -eq 0
    # 作業の置き場を作り直す。
    if (Test-Path $work) { Remove-Item -Recurse -Force $work }
    New-Item -ItemType Directory -Force (Join-Path $work 'profile'), (Join-Path $work 'bridge-profile') | Out-Null
    if (-not $sampleOk) { return (& $fail "nar-sample-path $g failed, see $BuildLog") }
    foreach ($l in @($lines)) {
        if ($l -like 'folder=*') { $result.ghost_folder = $l.Substring(7) }
        if ($l -like "balloon.$balloonName=*") { $result.balloon_folder = $l.Substring("balloon.$balloonName=".Length) }
    }
    if (-not $result.ghost_folder -or -not $result.balloon_folder) { return (& $fail "nar-sample-path $g printed no folder or balloon.$balloonName") }

    $port = Get-FreePort
    $result.port = $port
    $areka = $null; $bridge = $null; $bridgeErr = $null

    try {
        # ---- areka を起こす（環境は子へ引き継ぐため、この間だけ自分の環境を書き替えて戻す）----
        $set = [ordered]@{
            AREKA_MCP_PORT = "$port"; AREKA_PROFILE_DIR = (Join-Path $work 'profile')
            RUST_LOG = 'warn,areka::mcp=debug,areka_mcp=debug'; NO_COLOR = '1'; AREKA_NO_ALERT = '1'
            AREKA_APP_SMOKE_EXIT_MS = "$AutoExitMs"; AREKA_ROOT = $null; AREKA_USER_ROOT = $null
        }
        $saved = @{}
        foreach ($k in $set.Keys) { $saved[$k] = [Environment]::GetEnvironmentVariable($k); Set-EnvVar $k $set[$k] }
        try {
            $areka = Start-Process -FilePath $ArekaExe -ArgumentList "`"$($result.ghost_folder)`"", "`"$($result.balloon_folder)`"" `
                -WorkingDirectory $work -NoNewWindow -PassThru `
                -RedirectStandardOutput (Join-Path $work 'areka-stdout.log') -RedirectStandardError (Join-Path $work 'areka-stderr.log')
        } finally {
            foreach ($k in $saved.Keys) { Set-EnvVar $k $saved[$k] }
        }
        $null = $areka.Handle  # 終了コードを後で読めるように握る
        $arekaStart = [Diagnostics.Stopwatch]::StartNew()
        Say "$g areka started pid=$($areka.Id) port=$port"

        # 待ち受けが開くまで待つ（起動の失敗は 2）。
        while (-not (Test-Listening $port)) {
            if ($areka.HasExited) { return (& $fail "areka exited before listening (exit $($areka.ExitCode))") }
            if ($arekaStart.ElapsedMilliseconds -ge 120000) { return (& $fail "areka did not listen on $port within 120 s") }
            Start-Sleep -Milliseconds 250
        }

        # ---- 中継を起こす（同じポート・別の記憶・標準入出力をつなぐ）----
        $psi = [Diagnostics.ProcessStartInfo]::new($BridgeExe)
        $psi.UseShellExecute = $false
        $psi.WorkingDirectory = $work
        $psi.RedirectStandardInput = $true; $psi.RedirectStandardOutput = $true; $psi.RedirectStandardError = $true
        $psi.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
        $psi.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
        $psi.StandardErrorEncoding = [Text.UTF8Encoding]::new($false)
        $psi.Environment['AREKA_MCP_PORT'] = "$port"
        $psi.Environment['AREKA_PROFILE_DIR'] = (Join-Path $work 'bridge-profile')
        [void]$psi.Environment.Remove('AREKA_ROOT'); [void]$psi.Environment.Remove('AREKA_USER_ROOT')
        $bridge = [Diagnostics.Process]::Start($psi)
        $bridgeErr = $bridge.StandardError.ReadToEndAsync()
        $script:pending = $null
        Say "$g bridge started pid=$($bridge.Id)"

        $send = { param($obj) $bridge.StandardInput.WriteLine(($obj | ConvertTo-Json -Depth 8 -Compress)); $bridge.StandardInput.Flush() }
        $call = {
            param($phase, $n, $name, $arguments)
            $script:id = $script:id + 1
            $reqId = $script:id
            $sw = [Diagnostics.Stopwatch]::StartNew()
            & $send ([ordered]@{ jsonrpc = '2.0'; id = $reqId; method = 'tools/call'; params = [ordered]@{ name = $name; arguments = $arguments } })
            $ans = Read-Answer $bridge $reqId $ReadWaitMs
            $ms = [int]$sw.ElapsedMilliseconds
            $c = Get-Class $ans
            $rec = New-Record $n $phase "$name" $ms $c
            $result.records += $rec
            & $save
            $detail = if ($c.class -eq 'image') { "data_length=$($c.data_length)" } elseif ($c.code) { "code=$($c.code) $($c.message)" } else { "$($c.reason)" }
            Say ("{0} {1}#{2} {3} {4}ms {5} {6}" -f $g, $phase, $n, $name, $ms, $c.class, $detail)
            return $rec
        }
        # initialize（2025-06-18）と notifications/initialized。
        $script:id = 1
        & $send ([ordered]@{ jsonrpc = '2.0'; id = 1; method = 'initialize'; params = [ordered]@{ protocolVersion = '2025-06-18'; capabilities = @{}; clientInfo = [ordered]@{ name = 'debug-dump-balloon'; version = '0' } } })
        $init = Read-Answer $bridge 1 $ReadWaitMs
        if ($null -eq $init.Json -or $init.Json.PSObject.Properties['error']) { return (& $fail 'initialize got no result') }
        & $send ([ordered]@{ jsonrpc = '2.0'; method = 'notifications/initialized' })
        Say "$g initialize ok ($($init.Json.result.protocolVersion))"

        # 要件 2.7: 形の違う params の tools/call を 1 回（期待は -32602）。
        $script:id = $script:id + 1
        $reqId = $script:id
        $sw = [Diagnostics.Stopwatch]::StartNew()
        & $send ([ordered]@{ jsonrpc = '2.0'; id = $reqId; method = 'tools/call'; params = @{ name = 7 } })
        $c = Get-Class (Read-Answer $bridge $reqId $ReadWaitMs)
        $rec = New-Record 1 'req2.7' 'tools/call {"name":7}' ([int]$sw.ElapsedMilliseconds) $c
        $result.records += $rec; & $save
        Say ("{0} req2.7#1 tools/call(name=7) {1}ms {2} code={3} {4}" -f $g, $rec.ms, $c.class, $c.code, $c.message)

        # バルーンが出るのを待つ: dump_balloon を 1 秒おきに、絵が返るか 60 秒まで。
        $waitSw = [Diagnostics.Stopwatch]::StartNew()
        $seen = $false; $n = 0
        while (-not $seen -and $waitSw.ElapsedMilliseconds -lt $FirstPictureMs) {
            $n++
            $r = & $call 'wait' $n 'dump_balloon' @{ scope = 0 }
            if ($r.class -eq 'image') { $seen = $true; break }
            if ($r.class -in @('timeout_answer', 'bridge_timeout', 'no_return')) { break }
            Start-Sleep -Milliseconds 1000
        }

        if ($seen) {
            # 測る回。
            for ($i = 1; $i -le $Calls; $i++) {
                $r = & $call 'measured' $i $Tool @{ scope = 0 }
                if ($r.class -eq 'image') { $result.measured_images++ }
            }
        }

        $bad = @($result.records | Where-Object { $_.class -in @('timeout_answer', 'bridge_timeout', 'no_return') }).Count
        $measured = @($result.records | Where-Object { $_.phase -eq 'measured' })
        $allOk = $measured.Count -eq $Calls -and @($measured | Where-Object { $_.class -notin @('image', 'refusal') }).Count -eq 0
        if ($bad -gt 0) { $result.exit_code = 1 }
        elseif (-not $seen) { $result.exit_code = 2; $result.note = "no picture within $($FirstPictureMs / 1000) s" }
        elseif ($allOk -and $result.measured_images -ge 1) { $result.exit_code = 0 }
        else { $result.exit_code = 2; $result.note = 'measured calls had no picture or an unexpected answer' }
        Say "$g measured_images=$($result.measured_images)/$Calls exit=$($result.exit_code)"

        # ---- 畳む: 中継の標準入力を閉じ、areka は自動終了を待つ ----
        $bridge.StandardInput.Close()
        if (-not $bridge.WaitForExit(15000)) { $bridge.Kill(); $result.killed += 'bridge' }
        Say "$g waiting for areka auto exit (deadline $($DeadlineMs / 1000) s from start)"
        $left = $DeadlineMs - [int]$arekaStart.ElapsedMilliseconds
        if ($left -lt 0) { $left = 0 }
        if (-not $areka.WaitForExit($left)) { $areka.Kill(); $areka.WaitForExit(); $result.killed += 'areka' }
        return $result.exit_code
    } finally {
        # 異常の道でも、自分で起こした子だけを止める。
        if ($bridge -and -not $bridge.HasExited) { try { $bridge.Kill(); $bridge.WaitForExit(5000) | Out-Null; $result.killed += 'bridge' } catch {} }
        if ($areka -and -not $areka.HasExited) { try { $areka.Kill(); $areka.WaitForExit(5000) | Out-Null; $result.killed += 'areka' } catch {} }
        if ($bridge) { try { $result.bridge_exit = $bridge.ExitCode } catch {} }
        if ($areka) { try { $result.areka_exit = $areka.ExitCode } catch {} }
        if ($bridgeErr) { try { Set-Content -Path (Join-Path $work 'bridge-stderr.log') -Value $bridgeErr.Result -Encoding utf8 } catch {} }
        & $save
        if ($result.killed.Count -gt 0) { Say "$g killed: $($result.killed -join ',')" }
        Say "$g results: $resultsPath"
    }
}

# ゴーストは 1 体ずつ順に起こす。
$codes = @()
foreach ($g in $ghosts) { $codes += [int](Invoke-Ghost $g | Select-Object -Last 1) }
$final = if ($codes -contains 1) { 1 } elseif ($codes -contains 2) { 2 } else { 0 }
Say "done: exit=$final"
exit $final
