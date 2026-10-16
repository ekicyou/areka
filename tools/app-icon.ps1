<#
.SYNOPSIS
  areka.exe のアイコンの寸法ごとの PNG 16 枚から、app.ico・tray.ico・icons.res を作り直す。

.DESCRIPTION
  読み書きするのは、このスクリプトの在るリポジトリの crates/areka/icon/ だけ（呼ばれた場所によらない・今のフォルダを変えない）。
  端末へ出す文は ASCII の英語だけ。端末の文字コードは読みも書きもしない。ファイルに時刻を書かない。

    （引数なし）
        先に入力を全部検査する。16 枚の <名>-<寸法>.png（名は app・tray、寸法は 16 20 24 32 40 48 64 256）が
        在り、PNG の印と IHDR で始まり、幅と高さがその寸法、ビットの深さ 8、色の型 6（RGBA）であること。
        欠けや違いはファイルごとに 1 行ずつ出す（最初の 1 つで止めない）。
          FAIL input <file>: missing
          FAIL input <file>: not a PNG
          FAIL input <file>: size <w>x<h>, want <n>x<n>
          FAIL input <file>: not 8-bit RGBA
        揃っていれば 16 枚のバイトをそのまま並べて（縮めない）3 つを組み、一時ファイルへ書けてから置き換える。
          OK wrote <file> (<n> images)            .res は (2 groups, 16 images)
          FAIL write <file>: <reason>             一時ファイルを消し、今の 3 つは触らない

  形（設計の「Data Models」）:
    .ico  頭 6 バイト（0・1・8）＋目録 16 バイト×8（寸法の小さい順・256 は 0・深さ 32）＋PNG のバイト
    .res  空の項目 32 バイト、RT_ICON 1-16（app 1-8・tray 9-16、記憶の印 0x1010）、
          RT_GROUP_ICON 1（app）・2（tray）（記憶の印 0x1030）。言語 0・4 バイト境まで 0 で詰める。
          RT_MANIFEST は書かない。

    -Seed（原画を変えたときだけ・組みはしない）
        原画 app.png・tray.png を同じ決まり（1024x1024・8 ビット RGBA）で検査し、無い <名>-<寸法>.png だけを
        GDI+ で縮めて書く（32 ビット ARGB の画板・高品質の双三次・縁の折り返し）。在るファイルは上書きしない。
          OK seeded <file>
          FAIL input <file>: ...                  原画の欠けや違い。何も書かない
          FAIL write <file>: <reason>

  終了コード: 0 3 つとも書けた（-Seed は FAIL なし）／1 FAIL の行を出した
#>
param([switch]$Seed)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# 較正値（資源の番号の表・完了の後は変えない）
$SIZES = @(16, 20, 24, 32, 40, 48, 64, 256)
$MASTER_SIZE = 1024
$GROUPS = @(
    @{ Name = 'app';  GroupId = 1; FirstIconId = 1 },
    @{ Name = 'tray'; GroupId = 2; FirstIconId = 9 }
)
$RT_ICON = 3
$RT_GROUP_ICON = 14
$FLAGS_ICON = 0x1010
$FLAGS_GROUP = 0x1030
$RES_FILE = 'icons.res'

$IconDir = Join-Path (Split-Path $PSScriptRoot -Parent) 'crates/areka/icon'

function ConvertTo-Ascii([string]$s) { $s -replace '[^\x20-\x7E]', '?' }

function Get-BE32([byte[]]$b, [int]$at) {
    ([uint32]$b[$at] -shl 24) -bor ([uint32]$b[$at + 1] -shl 16) -bor ([uint32]$b[$at + 2] -shl 8) -bor [uint32]$b[$at + 3]
}

# 1 枚を検査する（在る・PNG の印と IHDR・n x n・8 ビット RGBA）。戻り値: FAIL の行（文字列）か、通ればバイト
function Test-PngFile([string]$Dir, [string]$File, [int]$N) {
    $path = Join-Path $Dir $File
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return "FAIL input ${File}: missing" }
    $b = [System.IO.File]::ReadAllBytes($path)
    $sig = [byte[]](0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)
    $isPng = $b.Length -ge 33 -and
        [System.Linq.Enumerable]::SequenceEqual([byte[]]$b[0..7], $sig) -and
        [System.Text.Encoding]::ASCII.GetString($b, 12, 4) -eq 'IHDR'
    if (-not $isPng) { return "FAIL input ${File}: not a PNG" }
    $w = Get-BE32 $b 16
    $h = Get-BE32 $b 20
    if ($w -ne $N -or $h -ne $N) { return "FAIL input ${File}: size ${w}x${h}, want ${N}x${N}" }
    if ($b[24] -ne 8 -or $b[25] -ne 6) { return "FAIL input ${File}: not 8-bit RGBA" }
    , $b
}

# 16 枚を検査する。戻り値: @{ Fails = 行の配列; Png = @{ '<名>-<寸法>.png' = bytes } }
function Test-IconInput([string]$Dir) {
    $fails = [System.Collections.Generic.List[string]]::new()
    $png = @{}
    foreach ($g in $GROUPS) {
        foreach ($n in $SIZES) {
            $file = "$($g.Name)-$n.png"
            $r = Test-PngFile $Dir $file $n
            if ($r -is [string]) { $fails.Add($r) } else { $png[$file] = $r }
        }
    }
    @{ Fails = $fails; Png = $png }
}

# 種まき: 原画 2 枚を検査し、無い <名>-<寸法>.png だけを縮めて書く。戻り値: 端末へ出す行
function Invoke-Seed([string]$Dir) {
    $masters = @{}
    $fails = @(foreach ($g in $GROUPS) {
            $r = Test-PngFile $Dir "$($g.Name).png" $MASTER_SIZE
            if ($r -is [string]) { $r } else { $masters[$g.Name] = $r }
        })
    if ($fails.Count) { return , $fails }

    Add-Type -AssemblyName System.Drawing
    $lines = [System.Collections.Generic.List[string]]::new()
    foreach ($g in $GROUPS) {
        $src = [System.Drawing.Bitmap]::new([System.IO.MemoryStream]::new($masters[$g.Name]))
        try {
            foreach ($n in $SIZES) {
                $file = "$($g.Name)-$n.png"
                $path = Join-Path $Dir $file
                if (Test-Path -LiteralPath $path) { continue }
                $tmp = "$path.tmp"
                $dst = [System.Drawing.Bitmap]::new($n, $n, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
                $gr = [System.Drawing.Graphics]::FromImage($dst)
                $attr = [System.Drawing.Imaging.ImageAttributes]::new()
                try {
                    $gr.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
                    $gr.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                    $gr.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
                    $gr.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
                    $attr.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
                    $gr.DrawImage($src, [System.Drawing.Rectangle]::new(0, 0, $n, $n), 0, 0, $src.Width, $src.Height, [System.Drawing.GraphicsUnit]::Pixel, $attr)
                    $dst.Save($tmp, [System.Drawing.Imaging.ImageFormat]::Png)
                    # 上書きしない Move: 間に誰かが置いたファイルも守る
                    [System.IO.File]::Move($tmp, $path, $false)
                    $lines.Add("OK seeded $file")
                } catch {
                    Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
                    $lines.Add("FAIL write ${file}: $(ConvertTo-Ascii $_.Exception.GetBaseException().Message)")
                } finally {
                    $attr.Dispose(); $gr.Dispose(); $dst.Dispose()
                }
            }
        } finally { $src.Dispose() }
    }
    , @($lines)
}

# 検査を通った 16 枚から 3 つのファイルのバイトを組む。戻り値: 順序つきの @{ ファイル名 = bytes }
function New-IconOutputs([hashtable]$Png) {
    $out = [ordered]@{}
    $resIcons = [System.Collections.Generic.List[object]]::new()
    $resGroups = [System.Collections.Generic.List[object]]::new()
    foreach ($g in $GROUPS) {
        $images = @($SIZES | ForEach-Object { , $Png["$($g.Name)-$_.png"] })

        # .ico
        $ms = [System.IO.MemoryStream]::new()
        $w = [System.IO.BinaryWriter]::new($ms)
        $w.Write([uint16]0); $w.Write([uint16]1); $w.Write([uint16]$SIZES.Count)
        $offset = 6 + 16 * $SIZES.Count
        for ($i = 0; $i -lt $SIZES.Count; $i++) {
            $dim = [byte]($SIZES[$i] % 256)
            $w.Write($dim); $w.Write($dim); $w.Write([byte]0); $w.Write([byte]0)
            $w.Write([uint16]1); $w.Write([uint16]32)
            $w.Write([uint32]$images[$i].Length); $w.Write([uint32]$offset)
            $offset += $images[$i].Length
        }
        foreach ($img in $images) { $w.Write($img) }
        $w.Flush()
        $out["$($g.Name).ico"] = $ms.ToArray()

        # .res の RT_ICON と RT_GROUP_ICON の中身
        $gms = [System.IO.MemoryStream]::new()
        $gw = [System.IO.BinaryWriter]::new($gms)
        $gw.Write([uint16]0); $gw.Write([uint16]1); $gw.Write([uint16]$SIZES.Count)
        for ($i = 0; $i -lt $SIZES.Count; $i++) {
            $id = $g.FirstIconId + $i
            $dim = [byte]($SIZES[$i] % 256)
            $gw.Write($dim); $gw.Write($dim); $gw.Write([byte]0); $gw.Write([byte]0)
            $gw.Write([uint16]1); $gw.Write([uint16]32)
            $gw.Write([uint32]$images[$i].Length); $gw.Write([uint16]$id)
            $resIcons.Add(@{ Id = $id; Data = $images[$i] })
        }
        $gw.Flush()
        $resGroups.Add(@{ Id = $g.GroupId; Data = $gms.ToArray() })
    }

    # .res
    $ms = [System.IO.MemoryStream]::new()
    $w = [System.IO.BinaryWriter]::new($ms)
    $writeEntry = {
        param([int]$type, [int]$id, [int]$flags, [byte[]]$data)
        $w.Write([uint32]$data.Length); $w.Write([uint32]32)
        $w.Write([uint16]0xFFFF); $w.Write([uint16]$type)
        $w.Write([uint16]0xFFFF); $w.Write([uint16]$id)
        $w.Write([uint32]0); $w.Write([uint16]$flags); $w.Write([uint16]0)
        $w.Write([uint32]0); $w.Write([uint32]0)
        $w.Write($data)
        $pad = (4 - ($data.Length % 4)) % 4
        if ($pad) { $w.Write([byte[]]::new($pad)) }
    }
    & $writeEntry 0 0 0 ([byte[]]::new(0))
    foreach ($e in $resIcons) { & $writeEntry $RT_ICON $e.Id $FLAGS_ICON $e.Data }
    foreach ($e in $resGroups) { & $writeEntry $RT_GROUP_ICON $e.Id $FLAGS_GROUP $e.Data }
    $w.Flush()
    $out[$RES_FILE] = $ms.ToArray()
    $out
}

# 3 つを一時ファイルへ書けてから置き換える。今のファイルは .bak へ退かし、途中で失敗したら
# 置き換え済みのものを .bak から戻す（3 つとも前のまま）。一時ファイルと .bak は最後に消す。
function Write-IconOutputs([string]$Dir, $Outputs) {
    $current = $null
    $swapped = [System.Collections.Generic.List[string]]::new()
    $leftovers = @($Outputs.Keys | ForEach-Object { $p = Join-Path $Dir $_; "$p.tmp"; "$p.bak" })
    try {
        foreach ($name in $Outputs.Keys) {
            $current = $name
            [System.IO.File]::WriteAllBytes((Join-Path $Dir "$name.tmp"), $Outputs[$name])
        }
        foreach ($name in $Outputs.Keys) {
            $current = $name
            $dest = Join-Path $Dir $name
            if (Test-Path -LiteralPath $dest -PathType Leaf) { [System.IO.File]::Replace("$dest.tmp", $dest, "$dest.bak") }
            else { [System.IO.File]::Move("$dest.tmp", $dest) }
            $swapped.Add($name)
        }
    } catch {
        $msg = ConvertTo-Ascii $_.Exception.GetBaseException().Message
        foreach ($name in $swapped) {
            $dest = Join-Path $Dir $name
            try {
                if (Test-Path -LiteralPath "$dest.bak" -PathType Leaf) { [System.IO.File]::Move("$dest.bak", $dest, $true) }
                else { [System.IO.File]::Delete($dest) }
            } catch { $msg += "; restore $name failed: $(ConvertTo-Ascii $_.Exception.GetBaseException().Message)" }
        }
        foreach ($t in $leftovers) { Remove-Item -LiteralPath $t -Force -ErrorAction SilentlyContinue }
        return , @("FAIL write ${current}: $msg")
    }
    foreach ($t in $leftovers) { Remove-Item -LiteralPath $t -Force -ErrorAction SilentlyContinue }
    , @($Outputs.Keys | ForEach-Object {
            if ($_ -eq $RES_FILE) { "OK wrote $_ ($($GROUPS.Count) groups, $($GROUPS.Count * $SIZES.Count) images)" }
            else { "OK wrote $_ ($($SIZES.Count) images)" }
        })
}

# 検査して組んで書く。戻り値: 端末へ出す行（FAIL を含めば失敗）
function Invoke-Pack([string]$Dir) {
    $in = Test-IconInput $Dir
    if ($in.Fails.Count) { return , @($in.Fails) }
    Write-IconOutputs $Dir (New-IconOutputs $in.Png)
}

$lines = if ($Seed) { Invoke-Seed $IconDir } else { Invoke-Pack $IconDir }
$lines
if ($lines -like 'FAIL *') { exit 1 }
exit 0
