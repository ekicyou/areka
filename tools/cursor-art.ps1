<#
.SYNOPSIS
  同梱の手の形のカーソル（crates/areka/cursor/hand.svg・grip.svg）から hand.cur・grip.cur を作り直す。

.DESCRIPTION
  読み書きするのは、このスクリプトの在るリポジトリの crates/areka/cursor/ だけ（呼ばれた場所によらない）。
  組み立て（cargo build）では焼かない。絵を変えたときにだけ回し、出力の .cur をコミットする。
  端末へ出す文は ASCII の英語だけ。ファイルに時刻を書かない（同じ入力からは同じバイト）。

    （引数なし）
        先に 2 つの SVG を全部検査する。要素は svg の下の path だけ、path の属性は fill（#rrggbb）と d だけ、
        d の命令は絶対座標の M・L・C・Q・Z だけ（数の個数が命令に合うこと）。欠けや違いはファイルごとに 1 行。
          FAIL input <file>: <reason>
        通れば寸法ごとに焼く:
          1. path を GDI+ の GraphicsPath に写す（Q は同じ形の 3 次ベジェにする・塗りは nonzero）
          2. 白い塗り（輝度 50% 以上）の経路の外接矩形が、寸法 n の正方形の内側 1 px を残して
             縦横の比を保って収まる倍率と置き場所を決める
          3. 黒で埋めた画板に全部の経路を元の色で滑らかに塗る
          4. 画板の縁から、暗い画素（輝度 50% 未満）をたどって透明にする（手の中の黒い線は残る）
          5. 不透明な画素の 8 近傍の透明な画素を不透明の黒にする（1 px の黒い縁）
          6. 当たりの点 = 不透明な画素の外接矩形の中心（整数の画素・切り捨て）
        寸法 32 40 48 56 64 72 80 88 96 128 を 32 ビットの BMP（色の面＋1 ビットの覆い）の項目として
        .cur に詰め、一時ファイルへ書けてから置き換える。
          OK wrote <file> (10 images)
          FAIL write <file>: <reason>             一時ファイルを消し、今の .cur は触らない

    -SelfTest（道具を直したとき・本物のフォルダは書かない）
        SVG をワークツリーの target\cursor-art-selftest\ へ写し（毎回作り直す）、そこで判定を回す。
        判定: same bytes twice（2 回作って同じバイト）・unsupported command fails（相対の命令で止まる）
          OK selftest (2 checks)  /  FAIL selftest <name>

  終了コード: 0 全部書けた（-SelfTest は全部通った）／1 FAIL の行を出した
#>
[CmdletBinding()]
param([switch]$SelfTest)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# 較正値（Windows の拡大率 100-300% の 25% 刻みの寸法と 128）
$SIZES = @(32, 40, 48, 56, 64, 72, 80, 88, 96, 128)
$NAMES = @('hand', 'grip')
$ARITY = @{ M = 2; L = 2; C = 6; Q = 4; Z = 0 }

$CursorDir = Join-Path (Split-Path $PSScriptRoot -Parent) 'crates/areka/cursor'
Add-Type -AssemblyName System.Drawing

function ConvertTo-Ascii([string]$s) { $s -replace '[^\x20-\x7E]', '?' }

# 輝度 50% 未満か（整数で: 0.299R + 0.587G + 0.114B < 127.5）
function Test-Dark([int]$r, [int]$g, [int]$b) { (299 * $r + 587 * $g + 114 * $b) -lt 127500 }

# SVG を検査して読む。戻り値: FAIL の行（文字列）か、@( @{ R; G; B; Cmds = @( @{ C; P } ) } )
function Read-Svg([string]$Dir, [string]$File) {
    $path = Join-Path $Dir $File
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return "FAIL input ${File}: missing" }
    $settings = [System.Xml.XmlReaderSettings]::new()
    $settings.DtdProcessing = [System.Xml.DtdProcessing]::Prohibit
    $settings.XmlResolver = $null
    $doc = [System.Xml.XmlDocument]::new()
    try {
        $reader = [System.Xml.XmlReader]::Create($path, $settings)
        try { $doc.Load($reader) } finally { $reader.Dispose() }
    } catch { return "FAIL input ${File}: not XML ($(ConvertTo-Ascii $_.Exception.GetBaseException().Message))" }
    $root = $doc.DocumentElement
    if ($root.LocalName -ne 'svg') { return "FAIL input ${File}: root is not svg" }
    $paths = [System.Collections.Generic.List[object]]::new()
    foreach ($node in $root.ChildNodes) {
        if ($node.NodeType -ne [System.Xml.XmlNodeType]::Element) { continue }
        if ($node.LocalName -ne 'path') { return "FAIL input ${File}: unsupported element $($node.LocalName)" }
        foreach ($a in $node.Attributes) {
            if ($a.Name -notin @('fill', 'd')) { return "FAIL input ${File}: unsupported attribute $($a.Name)" }
        }
        $fill = $node.GetAttribute('fill')
        if ($fill -notmatch '^#[0-9a-fA-F]{6}$') { return "FAIL input ${File}: fill '$(ConvertTo-Ascii $fill)' is not #rrggbb" }
        $d = $node.GetAttribute('d')
        $tokens = [regex]::Matches($d, '[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?|[A-Za-z]')
        if (($d -replace '[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?|[A-Za-z]', '') -notmatch '^[\s,]*$') {
            return "FAIL input ${File}: unreadable path data"
        }
        $cmds = [System.Collections.Generic.List[object]]::new()
        $cur = $null
        foreach ($t in $tokens) {
            $v = $t.Value
            if ($v -cmatch '^[A-Za-z]$') {
                if ($v -cnotmatch '^[MLCQZ]$') { return "FAIL input ${File}: unsupported path command $v" }
                $cur = @{ C = $v; P = [System.Collections.Generic.List[double]]::new() }
                $cmds.Add($cur)
            } elseif ($null -eq $cur) {
                return "FAIL input ${File}: path data starts with a number"
            } else {
                $cur.P.Add([double]::Parse($v, [System.Globalization.CultureInfo]::InvariantCulture))
            }
        }
        foreach ($c in $cmds) {
            $n = $ARITY[$c.C]
            $ok = if ($n -eq 0) { $c.P.Count -eq 0 } else { $c.P.Count -gt 0 -and $c.P.Count % $n -eq 0 }
            if (-not $ok) { return "FAIL input ${File}: $($c.C) has $($c.P.Count) numbers" }
        }
        if ($cmds.Count -eq 0 -or $cmds[0].C -ne 'M') { return "FAIL input ${File}: path does not start with M" }
        $paths.Add(@{
                R = [Convert]::ToInt32($fill.Substring(1, 2), 16)
                G = [Convert]::ToInt32($fill.Substring(3, 2), 16)
                B = [Convert]::ToInt32($fill.Substring(5, 2), 16)
                Cmds = $cmds
            })
    }
    if ($paths.Count -eq 0) { return "FAIL input ${File}: no path" }
    if (-not @($paths | Where-Object { -not (Test-Dark $_.R $_.G $_.B) }).Count) { return "FAIL input ${File}: no white path" }
    , @($paths)
}

# 命令の列を GraphicsPath に写す（SVG の座標のまま）
function New-GdiPath($Cmds) {
    $gp = [System.Drawing.Drawing2D.GraphicsPath]::new([System.Drawing.Drawing2D.FillMode]::Winding)
    $x = 0.0; $y = 0.0; $sx = 0.0; $sy = 0.0
    $pt = { param($a, $b) [System.Drawing.PointF]::new([single]$a, [single]$b) }
    foreach ($c in $Cmds) {
        $p = $c.P
        switch ($c.C) {
            'M' {
                $gp.StartFigure()
                $x = $p[0]; $y = $p[1]; $sx = $x; $sy = $y
                for ($i = 2; $i -lt $p.Count; $i += 2) { $gp.AddLine((& $pt $x $y), (& $pt $p[$i] $p[$i + 1])); $x = $p[$i]; $y = $p[$i + 1] }
            }
            'L' {
                for ($i = 0; $i -lt $p.Count; $i += 2) { $gp.AddLine((& $pt $x $y), (& $pt $p[$i] $p[$i + 1])); $x = $p[$i]; $y = $p[$i + 1] }
            }
            'C' {
                for ($i = 0; $i -lt $p.Count; $i += 6) {
                    $gp.AddBezier((& $pt $x $y), (& $pt $p[$i] $p[$i + 1]), (& $pt $p[$i + 2] $p[$i + 3]), (& $pt $p[$i + 4] $p[$i + 5]))
                    $x = $p[$i + 4]; $y = $p[$i + 5]
                }
            }
            'Q' {
                for ($i = 0; $i -lt $p.Count; $i += 4) {
                    $qx = $p[$i]; $qy = $p[$i + 1]; $ex = $p[$i + 2]; $ey = $p[$i + 3]
                    $gp.AddBezier((& $pt $x $y), (& $pt ($x + 2 * ($qx - $x) / 3) ($y + 2 * ($qy - $y) / 3)),
                        (& $pt ($ex + 2 * ($qx - $ex) / 3) ($ey + 2 * ($qy - $ey) / 3)), (& $pt $ex $ey))
                    $x = $ex; $y = $ey
                }
            }
            'Z' { $gp.CloseFigure(); $x = $sx; $y = $sy }
        }
    }
    $gp
}

# 1 寸法を焼く。戻り値: @{ N; Hx; Hy; Px = BGRA の byte[]（上の行から） }
function New-CursorImage($Paths, $Gdi, [System.Drawing.RectangleF]$Bounds, [int]$N) {
    $s = ($N - 2) / [Math]::Max($Bounds.Width, $Bounds.Height)
    $dx = 1 + (($N - 2) - $Bounds.Width * $s) / 2 - $Bounds.X * $s
    $dy = 1 + (($N - 2) - $Bounds.Height * $s) / 2 - $Bounds.Y * $s
    $bmp = [System.Drawing.Bitmap]::new($N, $N, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $gr = [System.Drawing.Graphics]::FromImage($bmp)
    $m = [System.Drawing.Drawing2D.Matrix]::new([single]$s, 0, 0, [single]$s, [single]$dx, [single]$dy)
    try {
        $gr.Clear([System.Drawing.Color]::Black)
        $gr.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $gr.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $gr.Transform = $m
        for ($i = 0; $i -lt $Paths.Count; $i++) {
            $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, $Paths[$i].R, $Paths[$i].G, $Paths[$i].B))
            try { $gr.FillPath($brush, $Gdi[$i]) } finally { $brush.Dispose() }
        }
        $gr.Flush()
        $rect = [System.Drawing.Rectangle]::new(0, 0, $N, $N)
        $bd = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $px = [byte[]]::new($N * $N * 4)
        try {
            for ($y = 0; $y -lt $N; $y++) {
                [System.Runtime.InteropServices.Marshal]::Copy([IntPtr]::Add($bd.Scan0, $y * $bd.Stride), $px, $y * $N * 4, $N * 4)
            }
        } finally { $bmp.UnlockBits($bd) }
    } finally { $m.Dispose(); $gr.Dispose(); $bmp.Dispose() }

    # 4. 縁からたどれる暗い画素を透明にする
    $queue = [System.Collections.Generic.Queue[int]]::new()
    $seen = [bool[]]::new($N * $N)
    for ($i = 0; $i -lt $N; $i++) {
        foreach ($k in @($i, ($N * ($N - 1) + $i), ($i * $N), ($i * $N + $N - 1))) {
            if (-not $seen[$k]) { $seen[$k] = $true; $queue.Enqueue($k) }
        }
    }
    while ($queue.Count) {
        $k = $queue.Dequeue()
        $o = $k * 4
        if (-not (Test-Dark $px[$o + 2] $px[$o + 1] $px[$o])) { continue }
        $px[$o] = 0; $px[$o + 1] = 0; $px[$o + 2] = 0; $px[$o + 3] = 0
        $x = $k % $N; $y = [Math]::Floor($k / $N)
        if ($x -gt 0 -and -not $seen[$k - 1]) { $seen[$k - 1] = $true; $queue.Enqueue($k - 1) }
        if ($x -lt $N - 1 -and -not $seen[$k + 1]) { $seen[$k + 1] = $true; $queue.Enqueue($k + 1) }
        if ($y -gt 0 -and -not $seen[$k - $N]) { $seen[$k - $N] = $true; $queue.Enqueue($k - $N) }
        if ($y -lt $N - 1 -and -not $seen[$k + $N]) { $seen[$k + $N] = $true; $queue.Enqueue($k + $N) }
    }

    # 5. 1 px の黒い縁（4 の結果の不透明だけを見る）
    $opaque = [bool[]]::new($N * $N)
    for ($k = 0; $k -lt $N * $N; $k++) { $opaque[$k] = $px[$k * 4 + 3] -ne 0 }
    for ($y = 0; $y -lt $N; $y++) {
        for ($x = 0; $x -lt $N; $x++) {
            if ($opaque[$y * $N + $x]) { continue }
            $hit = $false
            for ($vy = [Math]::Max(0, $y - 1); $vy -le [Math]::Min($N - 1, $y + 1) -and -not $hit; $vy++) {
                for ($vx = [Math]::Max(0, $x - 1); $vx -le [Math]::Min($N - 1, $x + 1); $vx++) {
                    if ($opaque[$vy * $N + $vx]) { $hit = $true; break }
                }
            }
            if ($hit) { $o = ($y * $N + $x) * 4; $px[$o] = 0; $px[$o + 1] = 0; $px[$o + 2] = 0; $px[$o + 3] = 255 }
        }
    }

    # 6. 当たりの点
    $x0 = $N; $y0 = $N; $x1 = -1; $y1 = -1
    for ($k = 0; $k -lt $N * $N; $k++) {
        if ($px[$k * 4 + 3] -eq 0) { continue }
        $x = $k % $N; $y = [Math]::Floor($k / $N)
        if ($x -lt $x0) { $x0 = $x }; if ($x -gt $x1) { $x1 = $x }
        if ($y -lt $y0) { $y0 = $y }; if ($y -gt $y1) { $y1 = $y }
    }
    @{ N = $N; Hx = [int][Math]::Floor(($x0 + $x1) / 2); Hy = [int][Math]::Floor(($y0 + $y1) / 2); Px = $px }
}

# 寸法ごとの絵を .cur のバイトに詰める（目録は寸法の小さい順・項目は BITMAPINFOHEADER＋色の面（下の行から）＋覆い）
function ConvertTo-CurBytes($Images) {
    $ms = [System.IO.MemoryStream]::new()
    $w = [System.IO.BinaryWriter]::new($ms)
    $w.Write([uint16]0); $w.Write([uint16]2); $w.Write([uint16]$Images.Count)
    $offset = 6 + 16 * $Images.Count
    $blobs = foreach ($img in $Images) {
        $n = $img.N
        $stride = [int]([Math]::Floor(($n + 31) / 32)) * 4
        $bs = [System.IO.MemoryStream]::new()
        $b = [System.IO.BinaryWriter]::new($bs)
        $b.Write([uint32]40); $b.Write([int32]$n); $b.Write([int32](2 * $n))
        $b.Write([uint16]1); $b.Write([uint16]32); $b.Write([uint32]0)
        $b.Write([uint32]($n * $n * 4 + $stride * $n))
        $b.Write([int32]0); $b.Write([int32]0); $b.Write([uint32]0); $b.Write([uint32]0)
        for ($y = $n - 1; $y -ge 0; $y--) { $b.Write($img.Px, $y * $n * 4, $n * 4) }
        for ($y = $n - 1; $y -ge 0; $y--) {
            $row = [byte[]]::new($stride)
            for ($x = 0; $x -lt $n; $x++) {
                if ($img.Px[($y * $n + $x) * 4 + 3] -eq 0) { $row[$x -shr 3] = $row[$x -shr 3] -bor (0x80 -shr ($x -band 7)) }
            }
            $b.Write($row)
        }
        $b.Flush()
        , $bs.ToArray()
    }
    for ($i = 0; $i -lt $Images.Count; $i++) {
        $n = $Images[$i].N
        $w.Write([byte]($n % 256)); $w.Write([byte]($n % 256)); $w.Write([byte]0); $w.Write([byte]0)
        $w.Write([uint16]$Images[$i].Hx); $w.Write([uint16]$Images[$i].Hy)
        $w.Write([uint32]$blobs[$i].Length); $w.Write([uint32]$offset)
        $offset += $blobs[$i].Length
    }
    foreach ($blob in $blobs) { $w.Write($blob) }
    $w.Flush()
    , $ms.ToArray()
}

# 1 つの SVG から .cur のバイトを組む
function New-CurFromSvg($Paths) {
    $gdi = @($Paths | ForEach-Object { New-GdiPath $_.Cmds })
    try {
        $bounds = $null
        for ($i = 0; $i -lt $Paths.Count; $i++) {
            if (Test-Dark $Paths[$i].R $Paths[$i].G $Paths[$i].B) { continue }
            $r = $gdi[$i].GetBounds()
            $bounds = if ($null -eq $bounds) { $r } else { [System.Drawing.RectangleF]::Union($bounds, $r) }
        }
        $images = @(foreach ($n in $SIZES) { New-CursorImage $Paths $gdi $bounds $n })
        ConvertTo-CurBytes $images
    } finally { foreach ($g in $gdi) { $g.Dispose() } }
}

# 検査して組んで書く。戻り値: 端末へ出す行（FAIL を含めば失敗）
function Invoke-Pack([string]$Dir) {
    $parsed = [ordered]@{}
    $fails = @(foreach ($name in $NAMES) {
            $r = Read-Svg $Dir "$name.svg"
            if ($r -is [string]) { $r } else { $parsed[$name] = $r }
        })
    if ($fails.Count) { return , $fails }
    , @(foreach ($name in $NAMES) {
            $file = "$name.cur"
            $dest = Join-Path $Dir $file
            $tmp = "$dest.tmp"
            try {
                [System.IO.File]::WriteAllBytes($tmp, (New-CurFromSvg $parsed[$name]))
                [System.IO.File]::Move($tmp, $dest, $true)
                "OK wrote $file ($($SIZES.Count) images)"
            } catch {
                Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
                "FAIL write ${file}: $(ConvertTo-Ascii $_.Exception.GetBaseException().Message)"
            }
        })
}

# 自分の検査: SVG の写しで判定を回す。戻り値: 端末へ出す行
function Invoke-SelfTest {
    $root = Join-Path (Split-Path $PSScriptRoot -Parent) 'target/cursor-art-selftest'
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
    $script:caseNo = 0
    $newCase = {
        $script:caseNo++
        $d = Join-Path $root "case$script:caseNo"
        New-Item -ItemType Directory -Path $d -Force | Out-Null
        foreach ($name in $NAMES) { Copy-Item -LiteralPath (Join-Path $CursorDir "$name.svg") -Destination $d }
        $d
    }
    $readOut = { param($d) @($NAMES | ForEach-Object { [Convert]::ToBase64String([System.IO.File]::ReadAllBytes((Join-Path $d "$_.cur"))) }) -join '|' }
    $checks = [ordered]@{
        'same bytes twice'          = {
            $d = & $newCase
            $r1 = Invoke-Pack $d; $a = & $readOut $d
            $r2 = Invoke-Pack $d; $b = & $readOut $d
            -not ($r1 -like 'FAIL *') -and -not ($r2 -like 'FAIL *') -and $a -eq $b
        }
        'unsupported command fails' = {
            $d = & $newCase
            $p = Join-Path $d 'grip.svg'
            [System.IO.File]::WriteAllText($p, ([System.IO.File]::ReadAllText($p) -replace ' Z"', ' h 1 Z"'))
            $r = Invoke-Pack $d
            ($r -join '|') -eq 'FAIL input grip.svg: unsupported path command h' -and
                -not (Test-Path -LiteralPath (Join-Path $d 'hand.cur'))
        }
    }
    $fails = @(foreach ($name in $checks.Keys) {
            try { if (-not (& $checks[$name])) { "FAIL selftest $name" } }
            catch { "FAIL selftest ${name}: $(ConvertTo-Ascii $_.Exception.GetBaseException().Message)" }
        })
    if ($fails.Count) { , $fails } else { , @("OK selftest ($($checks.Count) checks)") }
}

$lines = if ($SelfTest) { Invoke-SelfTest } else { Invoke-Pack $CursorDir }
$lines
if ($lines -like 'FAIL *') { exit 1 }
exit 0
