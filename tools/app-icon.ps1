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

  終了コード: 0 3 つとも書けた／1 FAIL の行を出した
#>
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# 較正値（資源の番号の表・完了の後は変えない）
$SIZES = @(16, 20, 24, 32, 40, 48, 64, 256)
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

# 16 枚を検査する。戻り値: @{ Fails = 行の配列; Png = @{ '<名>-<寸法>.png' = bytes } }
function Test-IconInput([string]$Dir) {
    $fails = [System.Collections.Generic.List[string]]::new()
    $png = @{}
    $sig = [byte[]](0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A)
    foreach ($g in $GROUPS) {
        foreach ($n in $SIZES) {
            $file = "$($g.Name)-$n.png"
            $path = Join-Path $Dir $file
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { $fails.Add("FAIL input ${file}: missing"); continue }
            $b = [System.IO.File]::ReadAllBytes($path)
            $isPng = $b.Length -ge 33 -and
                [System.Linq.Enumerable]::SequenceEqual([byte[]]$b[0..7], $sig) -and
                [System.Text.Encoding]::ASCII.GetString($b, 12, 4) -eq 'IHDR'
            if (-not $isPng) { $fails.Add("FAIL input ${file}: not a PNG"); continue }
            $w = Get-BE32 $b 16
            $h = Get-BE32 $b 20
            if ($w -ne $n -or $h -ne $n) { $fails.Add("FAIL input ${file}: size ${w}x${h}, want ${n}x${n}"); continue }
            if ($b[24] -ne 8 -or $b[25] -ne 6) { $fails.Add("FAIL input ${file}: not 8-bit RGBA"); continue }
            $png[$file] = $b
        }
    }
    @{ Fails = $fails; Png = $png }
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

# 3 つを一時ファイルへ書けてから置き換える。失敗は一時ファイルを消して FAIL の行を返す。
function Write-IconOutputs([string]$Dir, $Outputs) {
    $temps = [ordered]@{}
    $current = $null
    try {
        foreach ($name in $Outputs.Keys) {
            $current = $name
            $tmp = Join-Path $Dir "$name.tmp"
            $temps[$name] = $tmp
            [System.IO.File]::WriteAllBytes($tmp, $Outputs[$name])
        }
        foreach ($name in $temps.Keys) {
            $current = $name
            [System.IO.File]::Move($temps[$name], (Join-Path $Dir $name), $true)
        }
    } catch {
        foreach ($t in $temps.Values) { if (Test-Path -LiteralPath $t -PathType Leaf) { Remove-Item -LiteralPath $t -Force -ErrorAction SilentlyContinue } }
        return , @("FAIL write ${current}: $(ConvertTo-Ascii $_.Exception.GetBaseException().Message)")
    }
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

$lines = Invoke-Pack $IconDir
$lines
if ($lines -like 'FAIL *') { exit 1 }
exit 0
