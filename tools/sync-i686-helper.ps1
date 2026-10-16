<#
.SYNOPSIS
  target\debug\ の areka.exe の隣の shiori-host32-helper.exe を i686（32bit）版に揃え、揃ったかだけで合否を返す。

.DESCRIPTION
  全体テスト（tools/test-all.ps1）の最後の段 `i686 helper beside areka.exe` から別のプロセスで呼ぶ。
  画面へ出す文は固定の英文（ASCII）だけ。端末の文字コードは読みも書きもしない。
  例外は本文を出さず、型の名前と HResult の 16 進だけを出す（本文は日本語に訳されて出ることがあるため）。

    （引数なし） [-Dest <path>] [-Source <path>]
        置き先が既に i686 版なら何も写さない。違えば写し元を一時名（<Dest>.<pid>.tmp）へ写してから
        Move-Item -Force で置き換える。失敗したら一時名を消す。相対パスはリポジトリの根から読む。
          helper is i686: {Dest}                                  揃っていた（写さない）
          placed i686 helper: {Source} -> {Dest}                  置いた
          cannot place i686 helper: source missing or not i686: {Source}
          cannot place i686 helper: {型} (HResult 0x........): {Dest} (is areka.exe running?)
          helper beside areka.exe is still not i686: {Dest}       置いた後も i686 でない
        終了コード: 最後にもう一度 Dest を判定した 1 か所だけで決める。0 i686 版／1 そうでない
    -SelfTest
        自分の検査。target\ の下のプロセス番号つきの作業場に偽の小さな PE を作り、本体を別のプロセスで回して
        6 つの分かれ道を終了コードと文の先頭で確かめ、「ok selftest {名前}」か「FAIL selftest {名前}」を出す。
          1 dest already i686   → 0・写さない
          2 dest x64            → 0・置き先が i686 になる
          3 dest missing        → 0・置き先が i686 になる
          4 source missing      → 1
          5 source x64          → 1
          6 dest locked         → 1・*.tmp が残らない
        作業場は終わりに消す。終了コード: 0 通った／1 落ちた判定が在る

  i686 版の判定（Test-I686Pe）は shiori-host32-host の classify_helper_image と同じ規則:
  先頭 4096 バイトの MZ・e_lfanew の先の PE\0\0・機械の種類 0x014C。無い・短い・読めないは偽。

.EXAMPLE
  pwsh -NoProfile -File tools/sync-i686-helper.ps1
  pwsh -NoProfile -File tools/sync-i686-helper.ps1 -SelfTest
#>
#Requires -Version 7
[CmdletBinding(DefaultParameterSetName = 'Sync')]
param(
    [Parameter(ParameterSetName = 'Sync')][string]$Dest = 'target\debug\shiori-host32-helper.exe',
    [Parameter(ParameterSetName = 'Sync')][string]$Source = 'target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe',
    [Parameter(ParameterSetName = 'SelfTest', Mandatory)][switch]$SelfTest
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent

function Test-I686Pe([string]$Path) {
    try {
        $fs = [IO.File]::OpenRead($Path)
        try {
            $head = [byte[]]::new(4096)
            $n = $fs.Read($head, 0, $head.Length)
        } finally { $fs.Dispose() }
        if ($n -lt 0x40 -or $head[0] -ne 0x4D -or $head[1] -ne 0x5A) { return $false }
        $e = [BitConverter]::ToUInt32($head, 0x3C)
        if ($e + 6 -gt $n) { return $false }
        if ($head[$e] -ne 0x50 -or $head[$e + 1] -ne 0x45 -or $head[$e + 2] -ne 0 -or $head[$e + 3] -ne 0) { return $false }
        return [BitConverter]::ToUInt16($head, [int]$e + 4) -eq 0x014C
    } catch { return $false }
}

if (-not $SelfTest) {
    $destPath = if ([IO.Path]::IsPathRooted($Dest)) { $Dest } else { Join-Path $root $Dest }
    $srcPath = if ([IO.Path]::IsPathRooted($Source)) { $Source } else { Join-Path $root $Source }
    $said = $false
    if (Test-I686Pe $destPath) {
        "helper is i686: $Dest"
        $said = $true
    } elseif (-not (Test-I686Pe $srcPath)) {
        "cannot place i686 helper: source missing or not i686: $Source"
        $said = $true
    } else {
        $tmp = "$destPath.$PID.tmp"
        try {
            Copy-Item -LiteralPath $srcPath -Destination $tmp -Force
            Move-Item -LiteralPath $tmp -Destination $destPath -Force
            "placed i686 helper: $Source -> $Dest"
        } catch {
            $x = $_.Exception
            'cannot place i686 helper: {0} (HResult 0x{1:X8}): {2} (is areka.exe running?)' -f $x.GetType().Name, $x.HResult, $Dest
            $said = $true
            Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
        }
    }
    # 終了コードを決めるのはここだけ（途中の出口を作らない）
    if (Test-I686Pe $destPath) { exit 0 }
    if (-not $said) { "helper beside areka.exe is still not i686: $Dest" }
    exit 1
}

# =============================================================================
# 自分の検査
# =============================================================================
$script:selfFails = 0
function Judge([string]$Name, [bool]$Ok) {
    if ($Ok) { "ok selftest $Name" } else { $script:selfFails++; "FAIL selftest $Name" }
}

# 偽の小さな PE（MZ・e_lfanew=0x40・PE\0\0・機械の種類）。$Tag で中身を変えて「写したか」を見分ける
function New-FakePe([string]$Path, [uint16]$Machine, [byte]$Tag = 0) {
    $b = [byte[]]::new(0x47)
    $b[0] = 0x4D; $b[1] = 0x5A
    $b[0x3C] = 0x40
    $b[0x40] = 0x50; $b[0x41] = 0x45
    $b[0x44] = $Machine -band 0xFF; $b[0x45] = $Machine -shr 8
    $b[0x46] = $Tag
    [IO.File]::WriteAllBytes($Path, $b)
}

# 本体を別のプロセスで回し、終了コードと文の 1 行目を返す
function Invoke-Sync([string]$Dir) {
    $out = @(& pwsh -NoProfile -File $PSCommandPath -Dest (Join-Path $Dir 'helper.exe') -Source (Join-Path $Dir 'src.exe') 2>&1)
    [pscustomobject]@{ Code = $LASTEXITCODE; Head = if ($out.Count) { [string]$out[0] } else { '' } }
}

$work = Join-Path $root "target\sync-i686-helper-selftest-$PID"
try {
    $null = New-Item -ItemType Directory -Path $work -Force
    $dirs = 1..6 | ForEach-Object { $d = Join-Path $work "b$_"; $null = New-Item -ItemType Directory -Path $d; $d }

    # 1 置き先が既に i686 → 0・写さない（写し元は中身の違う i686）
    $d = $dirs[0]
    New-FakePe (Join-Path $d 'helper.exe') 0x014C 1
    New-FakePe (Join-Path $d 'src.exe') 0x014C 2
    $r = Invoke-Sync $d
    Judge '1 dest already i686' ($r.Code -eq 0 -and $r.Head.StartsWith('helper is i686: ') -and
        [IO.File]::ReadAllBytes((Join-Path $d 'helper.exe'))[0x46] -eq 1)

    # 2 置き先が x64・写し元が i686 → 0・置き先が i686 になる
    $d = $dirs[1]
    New-FakePe (Join-Path $d 'helper.exe') 0x8664
    New-FakePe (Join-Path $d 'src.exe') 0x014C
    $r = Invoke-Sync $d
    Judge '2 dest x64' ($r.Code -eq 0 -and $r.Head.StartsWith('placed i686 helper: ') -and (Test-I686Pe (Join-Path $d 'helper.exe')))

    # 3 置き先が無い・写し元が i686 → 0・置き先が i686 になる
    $d = $dirs[2]
    New-FakePe (Join-Path $d 'src.exe') 0x014C
    $r = Invoke-Sync $d
    Judge '3 dest missing' ($r.Code -eq 0 -and $r.Head.StartsWith('placed i686 helper: ') -and (Test-I686Pe (Join-Path $d 'helper.exe')))

    # 4 写し元が無い → 1
    $d = $dirs[3]
    New-FakePe (Join-Path $d 'helper.exe') 0x8664
    $r = Invoke-Sync $d
    Judge '4 source missing' ($r.Code -eq 1 -and $r.Head.StartsWith('cannot place i686 helper: source missing or not i686: '))

    # 5 写し元が x64 → 1
    $d = $dirs[4]
    New-FakePe (Join-Path $d 'helper.exe') 0x8664
    New-FakePe (Join-Path $d 'src.exe') 0x8664
    $r = Invoke-Sync $d
    Judge '5 source x64' ($r.Code -eq 1 -and $r.Head.StartsWith('cannot place i686 helper: source missing or not i686: '))

    # 6 置き先を FileShare.None で開いて塞いだまま → 1・*.tmp が残らない
    $d = $dirs[5]
    New-FakePe (Join-Path $d 'helper.exe') 0x8664
    New-FakePe (Join-Path $d 'src.exe') 0x014C
    $lock = [IO.File]::Open((Join-Path $d 'helper.exe'), 'Open', 'Read', 'None')
    try { $r = Invoke-Sync $d } finally { $lock.Dispose() }
    Judge '6 dest locked' ($r.Code -eq 1 -and $r.Head.StartsWith('cannot place i686 helper: ') -and
        $r.Head.EndsWith('(is areka.exe running?)') -and @(Get-ChildItem -LiteralPath $d -Filter '*.tmp').Count -eq 0)
} catch {
    $script:selfFails++
    'FAIL selftest setup: {0} (HResult 0x{1:X8})' -f $_.Exception.GetType().Name, $_.Exception.HResult
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
if (Test-Path -LiteralPath $work) { $script:selfFails++; "FAIL selftest workspace removed: $work" }
if ($script:selfFails -gt 0) { exit 1 }
exit 0
