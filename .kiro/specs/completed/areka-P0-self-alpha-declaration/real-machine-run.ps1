<#
.SYNOPSIS
  areka-P0-self-alpha-declaration タスク 5（実機での確かめ）の根づくり・起動・判定。

.DESCRIPTION
  -Prepare : 今の版（HEAD）の areka と 32bit の補助プロセスを組み、本仕様の最初のコードのコミット
             （4db4b666）の親を git archive で target\sad-rm\baseline-src へ取り出して「今まで」の
             areka を target\sad-rm\baseline-target に組む（作業ツリーと git の状態には触れない）。
             検体（emo2・R_POST_and_KOMAINU・konnoyayame・claudia・StayseeBalloon）を nar-sample-path で
             展開して target\sad-rm\pristine へ写し、確かめ用の写しを足す:
               - emo2 の台本 boot.lua を、起動の台詞 1 本（固定の文）だけを話し、ほかは黙る形に差し替える
                 （バルーンの絵を 2 つの版で同じ文字にして画素で見比べるため）
               - 画像だけのシェル imageonly（descript.txt と surface0.png だけ・surfaces.txt なし）
               - Staysee の写し 2 つ（use_self_alpha を 0／full に書き換え）
               - Staysee の balloon*.png を赤紫（255,0,255）の上に合成して α を持たない絵にした写し 2 つ（1／full）
             製品のコードとリポジトリの検体（.nar）には触れない。
  -Run     : 指定の場合を指定の版（-Build head|base）で起動し、MCP（dump_surface・dump_balloon）で
             面とバルーンの絵を撮り、有界の自動終了（AREKA_APP_SMOKE_EXIT_MS）まで待つ。
             場合ごとに target\sad-rm\run\<場合>-<版>\ を作り直す（root・out・run.log）。
  -Check   : 記録と撮った絵を判定する（走らせ直さない）。端末へは ASCII だけを出し、記録の行の
             中身（日本語）は target\sad-rm\check-detail.txt へ書く。不合格があれば終了コード 1。

.EXAMPLE
  pwsh -NoProfile -File .kiro/specs/completed/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Prepare
  pwsh -NoProfile -File .kiro/specs/completed/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Run all -Build head
  pwsh -NoProfile -File .kiro/specs/completed/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Run all -Build base
  pwsh -NoProfile -File .kiro/specs/completed/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Check
#>
param(
    [switch]$Prepare,
    [string[]]$Run,
    [ValidateSet('head', 'base')]
    [string]$Build = 'head',
    [switch]$Check,
    [int]$ExitMs = 100000,
    [int]$Port = 39871,
    [int]$TalkWaitSec = 20,
    [string]$RustLog = 'info,areka_emo_atlas=debug,areka::mcp=debug'
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
$base = Join-Path $wt 'target\sad-rm'
$pristine = Join-Path $base 'pristine'
$baselineRev = '4db4b666^'
Add-Type -AssemblyName PresentationCore, System.Drawing

# 場合の一覧。Base=$true は「今まで」の版でも走らせ、撮った絵を画素で見比べる。
$cases = [ordered]@{
    'emo2-kakukaku'    = @{ Ghost = 'emo2'; Balloon = 'emo2-kakukaku'; Base = $true }
    'emo2-staysee'     = @{ Ghost = 'emo2'; Balloon = 'StayseeBalloon'; Base = $true }
    'emo2-claudia'     = @{ Ghost = 'emo2'; Balloon = 'claudia'; Base = $true }
    'emo2-claudia_v'   = @{ Ghost = 'emo2'; Balloon = 'claudia_vertical'; Base = $true }
    'rpost'            = @{ Ghost = 'R_POST_and_KOMAINU'; Balloon = 'StayseeBalloon'; Base = $true }
    'konnoyayame'      = @{ Ghost = 'konnoyayame'; Balloon = 'StayseeBalloon'; Base = $true }
    'claudia'          = @{ Ghost = 'claudia'; Balloon = 'claudia'; Base = $true }
    'imageonly'        = @{ Ghost = 'imageonly'; Balloon = 'StayseeBalloon'; Base = $false }
    'staysee-0'        = @{ Ghost = 'emo2'; Balloon = 'staysee-0'; Base = $false }
    'staysee-full'     = @{ Ghost = 'emo2'; Balloon = 'staysee-full'; Base = $false }
    'staysee-rgb-1'    = @{ Ghost = 'emo2'; Balloon = 'staysee-rgb-1'; Base = $false }
    'staysee-rgb-full' = @{ Ghost = 'emo2'; Balloon = 'staysee-rgb-full'; Base = $false }
}

# 画素を数える小さな道具（PowerShell の繰り返しでは遅いので C# で）。並びは BGRA。
Add-Type -TypeDefinition @'
public static class SadPx {
    // [α=0 の数, 0<α<255 の数, α=255 の数]
    public static long[] Stats(byte[] p) {
        long[] r = new long[3];
        for (int i = 3; i < p.Length; i += 4) { byte a = p[i]; if (a == 0) r[0]++; else if (a == 255) r[2]++; else r[1]++; }
        return r;
    }
    // 期待の α: mode 0 = 元の α / 1 = 全面不透明 / 2 = 赤緑青が左上と同じなら 0・ほかは 255（α を捨てて抜く）
    // / 3 = 4 バイトが左上と同じなら 0・ほかは元の α を 255 にしたもの（α なしの絵の抜き色）
    public static byte[] Expect(byte[] s, int mode) {
        byte[] e = new byte[s.Length / 4];
        for (int i = 0, j = 0; i < s.Length; i += 4, j++) {
            if (mode == 0) e[j] = s[i + 3];
            else if (mode == 1) e[j] = 255;
            else if (mode == 2) e[j] = (byte)((s[i] == s[0] && s[i + 1] == s[1] && s[i + 2] == s[2]) ? 0 : 255);
            else e[j] = (byte)((s[i] == s[0] && s[i + 1] == s[1] && s[i + 2] == s[2] && s[i + 3] == s[3]) ? 0 : 255);
        }
        return e;
    }
    public static long Mismatch(byte[] dump, byte[] expectAlpha) {
        long n = 0;
        for (int i = 3, j = 0; i < dump.Length; i += 4, j++) if (dump[i] != expectAlpha[j]) n++;
        return n;
    }
    public static long PixelDiff(byte[] a, byte[] b) {
        if (a.Length != b.Length) return -1;
        long n = 0;
        for (int i = 0; i < a.Length; i += 4) if (a[i] != b[i] || a[i + 1] != b[i + 1] || a[i + 2] != b[i + 2] || a[i + 3] != b[i + 3]) n++;
        return n;
    }
}
'@

# PNG を色の管理なし・乗算なしの BGRA で読む（WIC。GDI+ のガンマ補正を通さない）。
function Read-Bgra([string]$path) {
    $fs = [IO.File]::OpenRead($path)
    try {
        $dec = [Windows.Media.Imaging.BitmapDecoder]::Create($fs,
            [Windows.Media.Imaging.BitmapCreateOptions]::IgnoreColorProfile -bor [Windows.Media.Imaging.BitmapCreateOptions]::PreservePixelFormat,
            [Windows.Media.Imaging.BitmapCacheOption]::OnLoad)
        $f = [Windows.Media.Imaging.FormatConvertedBitmap]::new($dec.Frames[0], [Windows.Media.PixelFormats]::Bgra32, $null, 0)
        $buf = [byte[]]::new($f.PixelWidth * $f.PixelHeight * 4)
        $f.CopyPixels($buf, $f.PixelWidth * 4, 0)
        [pscustomobject]@{ W = $f.PixelWidth; H = $f.PixelHeight; Px = $buf }
    } finally { $fs.Dispose() }
}

function Machine([string]$exe) {
    $b = [IO.File]::ReadAllBytes($exe)
    $pe = [BitConverter]::ToInt32($b, 0x3C)
    '{0:X4}' -f [BitConverter]::ToUInt16($b, $pe + 4)
}

function Copy-Fresh([string]$from, [string]$to) {
    if (Test-Path $to) { Remove-Item -Recurse -Force $to }
    New-Item -ItemType Directory -Force (Split-Path $to) | Out-Null
    Copy-Item -Recurse $from $to
}

# descript.txt の `use_self_alpha,<値>` の行の値だけをバイトのまま書き換える（Shift_JIS の行を壊さない）。
function Set-BalloonAlpha([string]$dir, [string]$value) {
    $p = Join-Path $dir 'descript.txt'
    $t = [Text.Encoding]::Latin1.GetString([IO.File]::ReadAllBytes($p))
    $n = [regex]::Matches($t, '(?m)^use_self_alpha,[^\r\n]*').Count
    if ($n -ne 1) { throw "use_self_alpha line count is $n in $p" }
    $t = [regex]::Replace($t, '(?m)^use_self_alpha,[^\r\n]*', "use_self_alpha,$value")
    [IO.File]::WriteAllBytes($p, [Text.Encoding]::Latin1.GetBytes($t))
}

# balloon*.png を赤紫の上に合成し、α を持たない 24 bit の PNG（色の型 2）にする。
function Flatten-Balloon([string]$dir) {
    foreach ($f in Get-ChildItem $dir -Filter 'balloon*.png') {
        $src = [Drawing.Image]::FromFile($f.FullName)
        try {
            $dst = [Drawing.Bitmap]::new($src.Width, $src.Height, [Drawing.Imaging.PixelFormat]::Format24bppRgb)
            $g = [Drawing.Graphics]::FromImage($dst)
            $g.Clear([Drawing.Color]::FromArgb(255, 255, 0, 255))
            $g.DrawImage($src, [Drawing.Rectangle]::new(0, 0, $src.Width, $src.Height))
            $g.Dispose()
        } finally { $src.Dispose() }
        $dst.Save($f.FullName, [Drawing.Imaging.ImageFormat]::Png)
        $dst.Dispose()
        $ct = [IO.File]::ReadAllBytes($f.FullName)[25]
        if ($ct -ne 2) { throw "flattened png color type is $ct (want 2): $($f.FullName)" }
    }
}

# シェルのフォルダの面の番号（surfaces.txt の見出し `surface<番号の並び>` と surface<数字>.png）。
function Surface-Ids([string]$shellDir) {
    $ids = [Collections.Generic.SortedSet[int]]::new()
    $st = Join-Path $shellDir 'surfaces.txt'
    if (Test-Path $st) {
        $t = [Text.Encoding]::Latin1.GetString([IO.File]::ReadAllBytes($st))
        foreach ($m in [regex]::Matches($t, '(?m)^\s*surface([0-9][0-9,\-! ]*)')) {
            foreach ($part in $m.Groups[1].Value.Split(',')) {
                $x = $part.Trim()
                if ($x -match '^\d+$') { [void]$ids.Add([int]$x) }
                elseif ($x -match '^(\d+)-(\d+)$' -and ([int]$Matches[2] - [int]$Matches[1]) -le 2000) {
                    for ($i = [int]$Matches[1]; $i -le [int]$Matches[2]; $i++) { [void]$ids.Add($i) }
                }
            }
        }
    }
    foreach ($f in Get-ChildItem $shellDir -File) { if ($f.Name -match '^surface(\d{1,9})\.png$') { [void]$ids.Add([int]$Matches[1]) } }
    @($ids)
}

function Mcp([string]$tool, [hashtable]$arguments) {
    $body = @{ jsonrpc = '2.0'; id = 1; method = 'tools/call'; params = @{ name = $tool; arguments = $arguments } } | ConvertTo-Json -Depth 6 -Compress
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$Port/api/mcp/v1" -ContentType 'application/json' `
        -Headers @{ Accept = 'application/json, text/event-stream' } -Body $body -TimeoutSec 30
}

# 1 回の撮影。画像は PNG で保存し、本文は answers.txt へ 1 行。
function Dump([string]$out, [string]$name, [string]$tool, [hashtable]$arguments) {
    try { $r = Mcp $tool $arguments } catch { Add-Content -LiteralPath (Join-Path $out 'answers.txt') "$name`tEXC`t$($_.Exception.Message)"; return }
    if ($r.error) { Add-Content -LiteralPath (Join-Path $out 'answers.txt') "$name`tRPC`t$($r.error.message)"; return }
    $text = ($r.result.content | Where-Object { $_.type -eq 'text' } | Select-Object -First 1).text
    $img = $r.result.content | Where-Object { $_.type -eq 'image' } | Select-Object -First 1
    if ($img) { [IO.File]::WriteAllBytes((Join-Path $out "$name.png"), [Convert]::FromBase64String($img.data)) }
    Add-Content -LiteralPath (Join-Path $out 'answers.txt') "$name`t$([bool]$r.result.isError)`t$text"
}

if ($Prepare) {
    $free = (Get-PSDrive C).Free
    if ($free -lt 10GB) { throw "C: free is $([math]::Round($free / 1GB, 1)) GB (< 10 GB); not building" }
    $env:CARGO_INCREMENTAL = '0'
    Push-Location $wt
    try {
        cargo build -p areka --bin areka -j 2; if ($LASTEXITCODE) { throw "areka build failed ($LASTEXITCODE)" }
        cargo build -p shiori-host32-helper --target i686-pc-windows-msvc -j 2; if ($LASTEXITCODE) { throw "helper build failed ($LASTEXITCODE)" }
        # 補助プロセスは本仕様で変えていない（差分 0 を確かめて、両方の版で同じものを使う）。
        $d = git diff --name-only $baselineRev HEAD -- crates/shiori-host32-helper crates/shiori-host32-ipc crates/shiori-abi
        if ($d) { throw "helper sources differ between baseline and HEAD: $d" }
        $src = Join-Path $base 'baseline-src'
        if (-not (Test-Path (Join-Path $src 'Cargo.toml'))) {
            New-Item -ItemType Directory -Force $src | Out-Null
            $tar = Join-Path $base 'baseline.tar'
            git archive --format=tar -o $tar $baselineRev; if ($LASTEXITCODE) { throw "git archive failed" }
            # Windows の tar は日本語の名前の .pdn 1 つで終了コード 1 を返す（ビルドに使わない）。中身の在処で確かめる。
            tar -xf $tar -C $src 2>$null
            Remove-Item $tar
            if (-not (Test-Path (Join-Path $src 'crates\areka\Cargo.toml'))) { throw "baseline source extraction failed" }
        }
        "baseline rev: $(git rev-parse $baselineRev)"
    } finally { Pop-Location }
    Push-Location (Join-Path $base 'baseline-src')
    try {
        cargo build -p areka --bin areka -j 2 --target-dir (Join-Path $base 'baseline-target'); if ($LASTEXITCODE) { throw "baseline areka build failed ($LASTEXITCODE)" }
    } finally { Pop-Location }
    $helper = Join-Path $wt 'target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe'
    if ((Machine $helper) -ne '014C') { throw "helper is not 32-bit: $helper" }
    foreach ($b in @(@('head', (Join-Path $wt 'target\debug\areka.exe')), @('base', (Join-Path $base 'baseline-target\debug\areka.exe')))) {
        $dir = Join-Path $base "bin\$($b[0])"
        New-Item -ItemType Directory -Force $dir | Out-Null
        Copy-Item $b[1] $dir -Force
        Copy-Item $helper $dir -Force
        "bin $($b[0]): areka=$(Machine (Join-Path $dir 'areka.exe')) helper=$(Machine (Join-Path $dir 'shiori-host32-helper.exe'))"
    }

    # 検体を展開して pristine へ写す（nar-sample-path は呼ぶたびに展開先を作り直す・1 本ずつ順に）。
    if (Test-Path $pristine) { Remove-Item -Recurse -Force $pristine }
    foreach ($n in 'emo2', 'R_POST_and_KOMAINU', 'konnoyayame', 'claudia', 'StayseeBalloon') {
        Push-Location $wt
        try { $printed = cargo run -q -p sample-ghost-kit --bin nar-sample-path -- $n; if ($LASTEXITCODE) { throw "nar-sample-path $n failed" } } finally { Pop-Location }
        $manual = ($printed | Where-Object { $_ -like 'root=*' }) -replace '^root=', ''
        foreach ($kind in 'ghost', 'balloon') {
            $k = Join-Path $manual $kind
            if (Test-Path $k) { foreach ($sub in Get-ChildItem $k -Directory) { Copy-Fresh $sub.FullName (Join-Path $pristine "$kind\$($sub.Name)") } }
        }
    }

    # emo2 の台本: 起動の台詞 1 本だけを話し、ほかは黙る（バルーンの文字を 2 つの版で同じにする）。
    $boot = '\0\s[0]self alpha check. scope zero.\1\s[10]scope one.\e'
    $lua = @"
--- areka-P0-self-alpha-declaration タスク 5（実機の確かめ）用に、target\sad-rm の写しの中だけで差し替えた台本。
--- 起動の台詞 1 本（固定の文）だけを話し、ゴースト自身の台詞はこの回のあいだ黙らせる。
local REG = require("pasta.shiori.event.register")

local BOOT = [==[$boot]==]
local function quiet(act) return nil end

REG.OnBoot = function(act) return BOOT end
REG.OnFirstBoot = REG.OnBoot
REG.OnGhostChanged = REG.OnBoot
for _, id in ipairs({ "OnTalk", "OnSecondChange", "OnMinuteChange", "OnHourTimeSignal", "OnMouseMove",
    "OnMouseClick", "OnMouseDoubleClick", "OnMouseWheel", "OnChoiceTimeout", "OnShellChanging",
    "OnShellChanged", "OnBalloonChange", "OnGhostChanging", "OnSurfaceChange", "OnSurfaceRestore",
    "OnInstallCompleteAll", "OnUpdateComplete" }) do
    REG[id] = quiet
end

return REG
"@
    $bootLua = Join-Path $pristine 'ghost\emo2\ghost\master\scripts\pasta\shiori\event\boot.lua'
    if (-not (Test-Path $bootLua)) { throw "emo2 boot.lua not found: $bootLua" }
    [IO.File]::WriteAllText($bootLua, ($lua -replace "(?<!`r)`n", "`r`n"), [Text.UTF8Encoding]::new($false))

    # 画像だけのシェル: emo2 の写しのシェルを descript.txt と surface0.png（R_POST の surface0000.png・α なしの絵）だけにする。
    Copy-Fresh (Join-Path $pristine 'ghost\emo2') (Join-Path $pristine 'ghost\imageonly')
    $io = Join-Path $pristine 'ghost\imageonly\shell\master'
    Get-ChildItem -Force $io | Remove-Item -Recurse -Force
    Copy-Item (Join-Path $pristine 'ghost\R_POST_and_KOMAINU\shell\master\surface0000.png') (Join-Path $io 'surface0.png')
    [IO.File]::WriteAllText((Join-Path $io 'descript.txt'), "charset,UTF-8`r`ntype,shell`r`nname,image-only`r`nseriko.use_self_alpha,1`r`n", [Text.UTF8Encoding]::new($false))

    # バルーンの写し 4 つ。
    $stay = Join-Path $pristine 'balloon\StayseeBalloon'
    foreach ($v in @(@('staysee-0', '0', $false), @('staysee-full', 'full', $false), @('staysee-rgb-1', '1', $true), @('staysee-rgb-full', 'full', $true))) {
        $dir = Join-Path $pristine "balloon\$($v[0])"
        Copy-Fresh $stay $dir
        Set-BalloonAlpha $dir $v[1]
        if ($v[2]) { Flatten-Balloon $dir }
    }
    "prepared $pristine"
    Get-ChildItem (Join-Path $pristine 'ghost'), (Join-Path $pristine 'balloon') -Directory | ForEach-Object { "  $($_.Parent.Name)\$($_.Name)" }
}

if ($Run) {
    $names = if ($Run -contains 'all') { @($cases.Keys | Where-Object { $Build -eq 'head' -or $cases[$_].Base }) } else { $Run }
    foreach ($name in $names) {
        $c = $cases[$name]
        if (-not $c) { throw "unknown case: $name" }
        if ($Build -eq 'base' -and -not $c.Base) { throw "case $name has no baseline run" }
        $exe = Join-Path $base "bin\$Build\areka.exe"
        if (-not (Test-Path $exe)) { throw "no build at $exe; run -Prepare first" }
        if (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue) { throw "port $Port is in use" }
        $dir = Join-Path $base "run\$name-$Build"
        if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
        $root = Join-Path $dir 'root'
        $out = Join-Path $dir 'out'
        New-Item -ItemType Directory -Force $out, (Join-Path $root 'tmp') | Out-Null
        $ghost = Join-Path $root "ghost\$($c.Ghost)"
        $balloon = Join-Path $root "balloon\$($c.Balloon)"
        Copy-Fresh (Join-Path $pristine "ghost\$($c.Ghost)") $ghost
        Copy-Fresh (Join-Path $pristine "balloon\$($c.Balloon)") $balloon
        $ghost = (Resolve-Path $ghost).Path; $balloon = (Resolve-Path $balloon).Path

        Get-ChildItem env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "env:$($_.Name)" }
        $env:NO_COLOR = '1'
        $env:RUST_LOG = $RustLog
        $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
        $env:AREKA_NO_ALERT = '1'
        $env:AREKA_MCP_PORT = "$Port"
        $env:AREKA_ROOT = $root
        $env:AREKA_PROFILE_DIR = Join-Path $root 'profile'
        $env:TMP = Join-Path $root 'tmp'; $env:TEMP = $env:TMP
        $p = Start-Process $exe -ArgumentList "`"$ghost`" `"$balloon`"" -WorkingDirectory $root `
            -RedirectStandardOutput (Join-Path $dir 'run.log') -RedirectStandardError (Join-Path $dir 'run.err.log') -WindowStyle Hidden -PassThru
        "$name-$Build pid=$($p.Id) start=$(Get-Date -Format o) exit_ms=$ExitMs" | Tee-Object -Append (Join-Path $base 'runs.txt')

        # 待受が答えるまで待つ（最大 60 秒）。
        $ready = $false
        for ($i = 0; $i -lt 120 -and -not $p.HasExited; $i++) {
            try { $null = Mcp 'get_status' @{}; $ready = $true; break } catch { Start-Sleep -Milliseconds 500 }
        }
        if ($ready) {
            Start-Sleep -Seconds $TalkWaitSec
            $shellDir = Join-Path $ghost 'shell\master'
            foreach ($id in Surface-Ids $shellDir) { Dump $out "surface-$id" 'dump_surface' @{ scope = 0; surface = $id } }
            foreach ($s in 0, 1) { Dump $out "balloon-$s" 'dump_balloon' @{ scope = $s } }
        } else { "  MCP did not answer" }
        $p.WaitForExit()
        "$name-$Build exit=$($p.ExitCode) end=$(Get-Date -Format o) dumps=$(@(Get-ChildItem $out -Filter *.png).Count)" | Tee-Object -Append (Join-Path $base 'runs.txt')
        Set-Content (Join-Path $dir 'exit.txt') "$($p.ExitCode)"
    }
}

if ($Check) {
    $results = [Collections.Generic.List[object]]::new()
    $detail = [Collections.Generic.List[string]]::new()
    function Judge([string]$n, [bool]$ok, [string]$d) { $results.Add([pscustomobject]@{ Ok = $ok; Name = $n; Detail = $d }) }
    function Field([string]$line, [string]$key) { if ($line -match "(?:^|\s)$key=(`"[^`"]*`"|\S+)") { $Matches[1].Trim('"') } }
    $keyLine = 'bake: element を抜き色（左上の 1 画素と同じ色）で透過しました'

    $runs = @{}
    # 走り終えた（exit.txt の在る）回だけを読む。
    foreach ($d in Get-ChildItem (Join-Path $base 'run') -Directory | Where-Object { Test-Path (Join-Path $_.FullName 'exit.txt') }) {
        $lines = @([IO.File]::ReadAllLines((Join-Path $d.FullName 'run.log'))) + @([IO.File]::ReadAllLines((Join-Path $d.FullName 'run.err.log')))
        $runs[$d.Name] = [pscustomobject]@{
            Dir = $d.FullName; Lines = $lines
            Exit = if (Test-Path (Join-Path $d.FullName 'exit.txt')) { (Get-Content (Join-Path $d.FullName 'exit.txt')).Trim() } else { '?' }
        }
    }

    foreach ($name in $cases.Keys) {
        $c = $cases[$name]
        foreach ($b in 'head', 'base') {
            $r = $runs["$name-$b"]
            if (-not $r) { if ($b -eq 'head' -or $c.Base) { Judge "$name-$b ran" $false 'no run directory' }; continue }
            $L = $r.Lines
            $smoke = @($L | Where-Object { $_.Contains('event="app_exit"') -and $_.Contains('origin=Smoke') }).Count
            Judge "$name-$b bounded auto-exit (app_exit origin=Smoke, exit 0)" (($smoke -ge 1) -and ($r.Exit -eq '0')) "smoke=$smoke exit=$($r.Exit)"
            $errs = @($L | Where-Object { $_ -match '^\S+\s+ERROR\s' })
            $warns = @($L | Where-Object { $_ -match '^\S+\s+WARN\s' })
            $keys = @($L | Where-Object { $_.Contains($keyLine) })
            $sa = @($L | Where-Object { $_.Contains('self_alpha: 透過の扱いを決めた') })
            $saWarn = @($L | Where-Object { $_.Contains('self_alpha: 透過の宣言の値が読めない') })
            $pna = @($L | Where-Object { $_.Contains('ignored_pna=') })
            $imgOnly = @($L | Where-Object { $_.Contains('shell: surfaces.txt が面を定義しないので') })
            $answers = Join-Path $r.Dir 'out\answers.txt'
            $ok = if (Test-Path $answers) { @(Get-Content $answers | Where-Object { $_ -match "`tFalse`t" }).Count } else { 0 }
            $ng = if (Test-Path $answers) { @(Get-Content $answers | Where-Object { $_ -notmatch "`tFalse`t" }).Count } else { 0 }
            "{0,-24} exit={1} ERROR={2} WARN={3} keycolor_debug={4} self_alpha_info={5} self_alpha_warn={6} ignored_pna_warn={7} image_only_info={8} dumps_ok={9} dumps_ng={10}" -f "$name-$b", $r.Exit, $errs.Count, $warns.Count, $keys.Count, $sa.Count, $saWarn.Count, $pna.Count, $imgOnly.Count, $ok, $ng
            $detail.Add("=== $name-$b")
            foreach ($l in $sa + $saWarn + $pna + $imgOnly) { $detail.Add("  $l") }
            foreach ($g in ($errs + $warns | ForEach-Object { ($_ -replace '^\S+\s+', '') -replace '\d{4}-\d\d-\d\dT[\d:.]+Z?', '' } | Group-Object | Sort-Object Count -Descending)) { $detail.Add("  [$($g.Count)x] $($g.Name)") }
            if (Test-Path $answers) { foreach ($a in Get-Content $answers | Where-Object { $_ -notmatch "`tFalse`t" }) { $detail.Add("  NG-dump: $a") } }
            $detail.Add("  keycolor rel_paths: $((@($keys | ForEach-Object { Field $_ 'rel_path' }) | Sort-Object -Unique) -join ' ')")

            if ($name -eq 'imageonly') {
                # emo2 の台本と着せ替えは面 0 しか無いシェルに無い面・部品を指すので、その「無いので読み飛ばす」
                # 系の ERROR だけは出る（シェルとゴーストの組み合わせの帰結）。それ以外の ERROR が 0 であることを見る。
                $absent = 'bind の \(カテゴリ, パーツ\) を名前解決できず|surface を解決できず|SurfaceNotFound|not found'
                $other = @($errs | Where-Object { $_ -notmatch $absent })
                Judge "$name-$b ERROR lines other than 'absent surface/part requested by the emo2 script' = 0" ($other.Count -eq 0) "ERROR=$($errs.Count) other=$($other.Count)"
            } else {
                Judge "$name-$b ERROR lines = 0" ($errs.Count -eq 0) "ERROR=$($errs.Count)"
            }
            Judge "$name-$b ignored_pna warn = 0" ($pna.Count -eq 0) "ignored_pna=$($pna.Count)"
            if ($b -eq 'head') {
                # 宣言の info: シェル・バルーンとも起動と採寸で 1 行ずつ（=2）。宣言の値と declared を見る。
                foreach ($kind in 'shell', 'balloon') {
                    $k = @($sa | Where-Object { (Field $_ 'kind') -eq $kind })
                    $tv = ($k | ForEach-Object { "$(Field $_ 'treatment')/$(Field $_ 'declared')" } | Sort-Object -Unique) -join ','
                    Judge "$name-head self_alpha info kind=$kind = 2 (boot + measure)" ($k.Count -eq 2) "count=$($k.Count) treatment/declared=$tv"
                }
                Judge "$name-head self_alpha warn = 0" ($saWarn.Count -eq 0) "count=$($saWarn.Count)"
            } else {
                Judge "$name-base has no self_alpha lines (pre-spec build)" ($sa.Count -eq 0) "count=$($sa.Count)"
            }
        }

        # 今の版と今までの版の画素の見比べ（撮れた絵すべて）。
        if ($c.Base -and $runs["$name-head"] -and $runs["$name-base"]) {
            $h = Join-Path $runs["$name-head"].Dir 'out'; $o = Join-Path $runs["$name-base"].Dir 'out'
            $hn = @(Get-ChildItem $h -Filter *.png | ForEach-Object Name); $on = @(Get-ChildItem $o -Filter *.png | ForEach-Object Name)
            $same = 0; $diff = [Collections.Generic.List[string]]::new()
            foreach ($f in $hn | Where-Object { $on -contains $_ }) {
                $a = Read-Bgra (Join-Path $h $f); $z = Read-Bgra (Join-Path $o $f)
                $n = if ($a.W -ne $z.W -or $a.H -ne $z.H) { -1 } else { [SadPx]::PixelDiff($a.Px, $z.Px) }
                if ($n -eq 0) { $same++ } else { $diff.Add("$f($n)") }
            }
            $only = @($hn | Where-Object { $on -notcontains $_ }) + @($on | Where-Object { $hn -notcontains $_ })
            Judge "$name head vs base: every dumped picture is pixel-identical" (($diff.Count -eq 0) -and ($only.Count -eq 0) -and ($same -ge 1)) "same=$same diff=$($diff.Count) $($diff -join ' ') one_side_only=$($only.Count) $($only -join ' ')"
            $kh = @($runs["$name-head"].Lines | Where-Object { $_.Contains($keyLine) } | ForEach-Object { "$(Field $_ 'rel_path') $(Field $_ 'b') $(Field $_ 'g') $(Field $_ 'r') $(Field $_ 'a')" } | Sort-Object)
            $kb = @($runs["$name-base"].Lines | Where-Object { $_.Contains($keyLine) } | ForEach-Object { "$(Field $_ 'rel_path') $(Field $_ 'b') $(Field $_ 'g') $(Field $_ 'r') $(Field $_ 'a')" } | Sort-Object)
            Judge "$name head vs base: same key-colour bake records" (($kh -join '|') -eq ($kb -join '|')) "head=$($kh.Count) base=$($kb.Count)"
        }
    }

    # バルーンの宣言の描き分け（画素）。元の絵を同じ大きさの候補から選び、期待の α と食い違う画素を数える。
    function Balloon-Judge([string]$run, [int]$scope, [string]$balloonDir, [int]$mode, [string]$label) {
        $png = Join-Path $base "run\$run\out\balloon-$scope.png"
        if (-not (Test-Path $png)) { Judge "$run balloon scope $scope $label" $false 'no dump'; return }
        $d = Read-Bgra $png
        $st = [SadPx]::Stats($d.Px)
        $best = $null
        foreach ($f in Get-ChildItem $balloonDir -Filter $(if ($scope -eq 0) { 'balloons*.png' } else { 'balloonk*.png' })) {
            $s = Read-Bgra $f.FullName
            if ($s.W -ne $d.W -or $s.H -ne $d.H) { continue }
            $m = [SadPx]::Mismatch($d.Px, [SadPx]::Expect($s.Px, $mode))
            if (-not $best -or $m -lt $best.M) { $best = [pscustomobject]@{ F = $f.Name; M = $m; Src = [SadPx]::Stats($s.Px); TL = '{0},{1},{2},{3}' -f $s.Px[2], $s.Px[1], $s.Px[0], $s.Px[3] } }
        }
        $tl = $d.Px[3]
        $info = "size=$($d.W)x$($d.H) alpha0=$($st[0]) semi=$($st[1]) opaque=$($st[2]) topleft_alpha=$tl src=$($best.F) src_semi=$($best.Src[1]) src_alpha0=$($best.Src[0]) src_topleft_rgba=$($best.TL) mismatch=$($best.M)"
        $script:balloonInfo.Add("$run scope=$scope $label : $info")
        $okMode = switch ($mode) {
            2 { ($st[1] -eq 0) -and ($tl -eq 0) }          # 0: 半透明なし・左上が抜ける
            1 { ($st[0] -eq 0) -and ($st[1] -eq 0) }      # full × α なし: 全面不透明
            3 { ($tl -eq 0) }                              # 1 × α なし: 左上の色が抜ける
            default { $true }                              # α をそのまま
        }
        Judge "$run balloon scope $scope $label" ($best -and $okMode -and ($best.M -le [math]::Max(16, $d.W * $d.H / 1000))) $info
    }
    $balloonInfo = [Collections.Generic.List[string]]::new()
    foreach ($s in 0, 1) {
        Balloon-Judge 'emo2-staysee-head' $s (Join-Path $pristine 'balloon\StayseeBalloon') 0 'decl=1 alpha src: uses own alpha (reference)'
        Balloon-Judge 'staysee-0-head' $s (Join-Path $pristine 'balloon\staysee-0') 2 'decl=0 alpha src: no semi, top-left colour keyed'
        Balloon-Judge 'staysee-full-head' $s (Join-Path $pristine 'balloon\staysee-full') 0 'decl=full alpha src: uses own alpha (same as 1)'
        Balloon-Judge 'staysee-rgb-1-head' $s (Join-Path $pristine 'balloon\staysee-rgb-1') 3 'decl=1 rgb src: top-left colour keyed'
        Balloon-Judge 'staysee-rgb-full-head' $s (Join-Path $pristine 'balloon\staysee-rgb-full') 1 'decl=full rgb src: fully opaque, not keyed'
    }
    # full × α ありは 1 と同じ絵（文字も同じ台本なので画素まで一致する）。
    foreach ($s in 0, 1) {
        $a = Join-Path $base "run\staysee-full-head\out\balloon-$s.png"; $z = Join-Path $base "run\emo2-staysee-head\out\balloon-$s.png"
        if ((Test-Path $a) -and (Test-Path $z)) { $n = [SadPx]::PixelDiff((Read-Bgra $a).Px, (Read-Bgra $z).Px); Judge "staysee-full vs staysee(1) balloon scope $s pixel-identical" ($n -eq 0) "diff_pixels=$n" }
    }
    # 宣言の値の記録（treatment）が書き換えどおり。
    foreach ($v in @(@('staysee-0-head', '0'), @('staysee-full-head', 'full'), @('staysee-rgb-1-head', '1'), @('staysee-rgb-full-head', 'full'), @('emo2-staysee-head', '1'))) {
        $r = $runs[$v[0]]
        if ($r) {
            $k = @($r.Lines | Where-Object { $_.Contains('self_alpha: 透過の扱いを決めた') -and (Field $_ 'kind') -eq 'balloon' })
            $bad = @($k | Where-Object { (Field $_ 'treatment') -ne $v[1] -or (Field $_ 'declared') -ne 'true' })
            Judge "$($v[0]) balloon self_alpha treatment=$($v[1]) declared=true" (($k.Count -eq 2) -and ($bad.Count -eq 0)) "lines=$($k.Count) mismatching=$($bad.Count)"
        }
    }
    # 画像だけのシェル: 記録 1 回の読み込みにつき 1 行・面 1 つ・surfaces_txt=missing、面 0 が抜き色で撮れる。
    $r = $runs['imageonly-head']
    if ($r) {
        $io = @($r.Lines | Where-Object { $_.Contains('shell: surfaces.txt が面を定義しないので') })
        $bad = @($io | Where-Object { (Field $_ 'surfaces_txt') -ne 'missing' -or (Field $_ 'surfaces') -ne '1' })
        Judge 'imageonly-head image-only info (surfaces_txt=missing surfaces=1) once per load (boot + measure = 2)' (($io.Count -eq 2) -and ($bad.Count -eq 0)) "lines=$($io.Count) mismatching=$($bad.Count)"
        $png = Join-Path $r.Dir 'out\surface-0.png'
        if (Test-Path $png) {
            $d = Read-Bgra $png; $s = Read-Bgra (Join-Path $pristine 'ghost\imageonly\shell\master\surface0.png')
            $m = if ($d.W -eq $s.W -and $d.H -eq $s.H) { [SadPx]::Mismatch($d.Px, [SadPx]::Expect($s.Px, 3)) } else { -1 }
            $st = [SadPx]::Stats($d.Px)
            Judge 'imageonly-head surface 0 drawn from the image alone, top-left colour keyed (decl=1, RGB png)' ($m -eq 0) "size=$($d.W)x$($d.H) alpha0=$($st[0]) semi=$($st[1]) opaque=$($st[2]) mismatch=$m"
        } else { Judge 'imageonly-head surface 0 dumped' $false 'no dump' }
    }

    $detail.Add('=== balloon alpha')
    foreach ($x in $balloonInfo) { $detail.Add("  $x") }
    [IO.File]::WriteAllLines((Join-Path $base 'check-detail.txt'), $detail, [Text.UTF8Encoding]::new($false))
    ''
    $results | ForEach-Object { '{0} {1} -- {2}' -f $(if ($_.Ok) { 'PASS' } else { 'FAIL' }), $_.Name, $_.Detail }
    "detail: $(Join-Path $base 'check-detail.txt')"
    if ($results | Where-Object { -not $_.Ok }) { 'RESULT: FAIL'; exit 1 } else { 'RESULT: PASS' }
}
