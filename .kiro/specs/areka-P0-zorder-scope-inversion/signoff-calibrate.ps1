<#
.SYNOPSIS
  signoff-judge.ps1 の較正（合格と不合格の両方を正しく返すこと）。手順は signoff-procedure.md。

.DESCRIPTION
  指定の無い実走のログ 1 本（-RealLog）をそのまま使い、その上に本番の行の組立（兄弟テストが逐語で
  固定している書式）で組んだ行を足して標本を作り、判定の道具へ当てて終了コードを期待と比べる。
  窓のハンドルは実走のログの `[zorder-pair] owner-established` の 2 行から取る（手で決めた値を使わない）。
  標本は target\zorder-inversion-signoff\calibration\ に作る（再生成が正本・コミットしない）。

.OUTPUTS
  終了コード 0=すべての標本で期待どおり / 1=食い違いがある / 3=実走のログが使えない
#>
param(
    [string]$RealLog
)
$ErrorActionPreference = 'Stop'
$wt = (Resolve-Path (git -C $PSScriptRoot rev-parse --show-toplevel)).Path
$out = Join-Path $wt 'target\zorder-inversion-signoff\calibration'
if (-not $RealLog) { $RealLog = Join-Path $out 'real-default.log' }
if (-not (Test-Path -LiteralPath $RealLog)) { Write-Output "BAD-ARGS: real log not found: $RealLog"; exit 3 }
$judge = Join-Path $PSScriptRoot 'signoff-judge.ps1'
New-Item -ItemType Directory -Force $out | Out-Null

$real = [IO.File]::ReadAllLines((Resolve-Path -LiteralPath $RealLog).Path)
$owners = @($real | Where-Object { $_.Contains('[zorder-pair] owner-established') })
if ($owners.Count -lt 2 -or @($real | Where-Object { $_.Contains('[zorder-group]') -or $_.Contains('[zorder-chain]') }).Count -gt 0) {
    Write-Output "BAD-ARGS: the real log must be a run without a setting with 2 owner-established lines: $RealLog"; exit 3
}
function Pick([string]$l, [string]$n) { if ($l -match ('(?<![\w])' + $n + '=(\S+)')) { $Matches[1] } }
$B0 = Pick $owners[0] 'owned_hwnd'; $S0 = Pick $owners[0] 'owner_hwnd'; $EB0 = Pick $owners[0] 'entity'; $ES0 = Pick $owners[0] 'peer'
$B1 = Pick $owners[1] 'owned_hwnd'; $S1 = Pick $owners[1] 'owner_hwnd'; $EB1 = Pick $owners[1] 'entity'; $ES1 = Pick $owners[1] 'peer'
Write-Output "real log: $RealLog"
Write-Output "handles: s0=$S0 b0=$B0 s1=$S1 b1=$B1"

# 本番の出力先と書式（zorder_chain_diag.rs の組立・zorder_drain の受理の本文）どおりの行。
$PD = '2026-10-19T00:00:00.000000Z DEBUG actor{actor=emo-text}: wintf::ecs::window::zorder_chain_diag: '
$PC = '2026-10-19T00:00:00.000001Z DEBUG actor{actor=emo-text}: wintf::ecs::window::zorder_chain: '
$PE = '2026-10-19T00:00:00.000002Z ERROR actor{actor=emo-text}: wintf::ecs::window::zorder_chain: '
$RESET = "$PD[zorder-group] applied action=reset groups=0 base=-"
function Applied([string]$tail) { "$PD[zorder-group] applied action=set group_id=0 source=Tag $tail" }
$RELEASED = "$PC[zorder-chain] pair-released entity=$EB0 peer=$ES0 owned_hwnd=$B0 owner_hwnd=$S0 unlinked=true"
# 結び目がまだ張られていなかった組の外し（owner_hwnd は番兵）——s0 は linked の行から引き直す。
$RELEASED_UNTIED = "$PC[zorder-chain] pair-released entity=$EB0 peer=$ES0 owned_hwnd=$B0 owner_hwnd=- unlinked=false"
# 逆転 s0,b0,b1,s1: 繋ぎは s0→b0（pos 1）と b0→b1（pos 2）。b1→s1 は組の結び目なので鎖は繋がない。
$INV_LINKS = @(
    "$PC[zorder-chain] linked segment=g0 owned=$ES0 owner=$EB0 owned_hwnd=$S0 owner_hwnd=$B0 pos=1/4",
    "$PC[zorder-chain] linked segment=g0 owned=$EB0 owner=$EB1 owned_hwnd=$B0 owner_hwnd=$B1 pos=2/4")
$INV_DECL = "$S0,$B0,$B1,$S1"
function Settled([string]$d, [string]$m) { "$PC[zorder-chain] settled nudged_hwnd=$($d.Split(',')[0]) insert_after=$($d.Split(',')[1]) declared=$d measured=$m nudge_ok=true" }
$UNLINKED = "$PC[zorder-chain] unlinked segment=g0 owned=$ES0 owned_hwnd=$S0 owner_hwnd=$B0 reason=Teardown"
# 組の並び b0,s0,b1,s1（逆転の無い明示・数字だけ）: 繋ぎは s0→b1（pos 2）だけ。
$PAIR_LINKS = @("$PC[zorder-chain] linked segment=g0 owned=$ES0 owner=$EB1 owned_hwnd=$S0 owner_hwnd=$B1 pos=2/4")
$PAIR_DECL = "$B0,$S0,$B1,$S1"

$invGood = @($RESET, (Applied 'members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1'), $RELEASED) + $INV_LINKS + @((Settled $INV_DECL "$S0,$B1"), (Settled $INV_DECL $INV_DECL), $UNLINKED)
$probeGood = @(
    "probe label=boot order=$INV_DECL,*",
    "probe label=press-s0 order=$INV_DECL,*",
    "probe label=press-b0 order=*,$INV_DECL,*",
    "probe label=drag-s0 order=$INV_DECL,*",
    "probe label=other-app order=*,$INV_DECL,*",
    "probe label=returned order=$INV_DECL,*")

$samples = [ordered]@{
    'inv-good.log'          = $real + $invGood
    'inv-untied.log'        = $real + @($RESET, (Applied 'members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1'), $RELEASED_UNTIED) + $INV_LINKS + @((Settled $INV_DECL $INV_DECL))
    'inv-oldword.log'       = $real + @($RESET, (Applied 'members=s0,b0,b1,s1 normalized=0:true,1:false+b1'), $RELEASED) + $INV_LINKS + @((Settled $INV_DECL $INV_DECL))
    'inv-noreleased.log'    = $real + @($RESET, (Applied 'members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1')) + $INV_LINKS + @((Settled $INV_DECL $INV_DECL))
    'inv-unsettled.log'     = $real + @($RESET, (Applied 'members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1'), $RELEASED) + $INV_LINKS + @((Settled $INV_DECL $INV_DECL), (Settled $INV_DECL "$B0,$S0,$B1,$S1"))
    'inv-refused.log'       = $real + $invGood + @("$PE[zorder-chain] link-refused owned=$ES0 owner=$EB0 owned_hwnd=$S0 owner_hwnd=$B0 reason=OwnerLoop")
    'inv-noapplied.log'     = $real + @($RELEASED) + $INV_LINKS + @((Settled $INV_DECL $INV_DECL))
    # 逆転の語で受理したのに、収まった並びが組の並び（本機能の前の形）になっている。
    'inv-pairorder.log'     = $real + @($RESET, (Applied 'members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1'), $RELEASED) + $PAIR_LINKS + @((Settled $PAIR_DECL $PAIR_DECL))
    'exp-good.log'          = $real + @($RESET, (Applied 'members=b0,s0,b1,s1 normalized=0:false,1:false')) + $PAIR_LINKS + @((Settled $PAIR_DECL $PAIR_DECL))
    'exp-released.log'      = $real + @($RESET, (Applied 'members=b0,s0,b1,s1 normalized=0:false,1:false'), $RELEASED) + $PAIR_LINKS + @((Settled $PAIR_DECL $PAIR_DECL))
    'num-good.log'          = $real + @($RESET, (Applied 'members=b0,s0,b1,s1 normalized=-')) + $PAIR_LINKS + @((Settled $PAIR_DECL $PAIR_DECL))
    'num-wrongword.log'     = $real + @($RESET, (Applied 'members=b0,s0,b1,s1 normalized=0:false,1:false')) + $PAIR_LINKS + @((Settled $PAIR_DECL $PAIR_DECL))
    'def-chainleak.log'     = $real + @($INV_LINKS[0])
    'nopair.log'            = @($real | Where-Object { -not $_.Contains('[zorder-pair]') })
    'probe-good.txt'        = $probeGood
    'probe-press-flip.txt'  = @($probeGood | ForEach-Object { if ($_ -like '*label=press-s0 *') { "probe label=press-s0 order=$B0,$S0,$B1,$S1,*" } else { $_ } })
    'probe-split.txt'       = @($probeGood | ForEach-Object { if ($_ -like '*label=other-app *') { "probe label=other-app order=$S0,$B0,*,$B1,$S1" } else { $_ } })
    'probe-missing.txt'     = @($probeGood | Where-Object { $_ -notlike '*label=returned *' })
    'probe-pairs-good.txt'  = @("probe label=boot order=*,$B0,$S0,*,$B1,$S1,*")
    'probe-pairs-flip.txt'  = @("probe label=boot order=$S0,$B0,$B1,$S1")
}
foreach ($k in $samples.Keys) { [IO.File]::WriteAllLines((Join-Path $out $k), [string[]]$samples[$k]) }
Copy-Item -LiteralPath $RealLog (Join-Path $out 'real.log') -Force

# 標本・種類・前面走査・期待の終了コード。
$rows = @(
    @('real.log', 'default', '', 0, 'real run without a setting passes'),
    @('real.log', 'default', 'probe-pairs-good.txt', 0, 'pairs adjacent in the scan pass'),
    @('real.log', 'default', 'probe-pairs-flip.txt', 1, 'a char in front of its balloon fails'),
    @('real.log', 'inverted', 'probe-good.txt', 1, 'no inversion in the log fails'),
    @('real.log', 'explicit', '', 1, 'no chain in the log fails'),
    @('def-chainleak.log', 'default', '', 1, 'a chain line without a setting fails'),
    @('inv-good.log', 'inverted', 'probe-good.txt', 0, 'pasta setting with good scans passes'),
    @('inv-untied.log', 'inverted', 'probe-good.txt', 0, 'untied pair: s0 taken from the linked line'),
    @('inv-good.log', 'inverted', 'probe-press-flip.txt', 1, 'b0 in front of s0 after press fails'),
    @('inv-good.log', 'inverted', 'probe-split.txt', 1, 'another app inside the chain fails'),
    @('inv-good.log', 'inverted', 'probe-missing.txt', 2, 'a missing scene is inconclusive'),
    @('inv-good.log', 'inverted', '', 2, 'no probe is inconclusive'),
    @('inv-oldword.log', 'inverted', 'probe-good.txt', 1, 'old word true fails'),
    @('inv-noreleased.log', 'inverted', 'probe-good.txt', 1, 'no pair-released fails'),
    @('inv-unsettled.log', 'inverted', 'probe-good.txt', 1, 'last settled differs fails'),
    @('inv-refused.log', 'inverted', 'probe-good.txt', 1, 'link-refused fails'),
    @('inv-noapplied.log', 'inverted', 'probe-good.txt', 2, 'no applied is inconclusive'),
    @('inv-pairorder.log', 'inverted', '', 1, 'settled in the old pair order fails even before the scans'),
    @('exp-good.log', 'explicit', '', 0, 'explicit pair order passes'),
    @('exp-released.log', 'explicit', '', 1, 'a released pair in explicit fails'),
    @('num-good.log', 'numeric', '', 0, 'numeric pair order passes'),
    @('num-wrongword.log', 'numeric', '', 1, 'numeric with explicit words fails'),
    @('inv-good.log', 'default', '', 1, 'a grouped log judged as default fails'),
    @('nopair.log', 'default', '', 2, 'no pair line is inconclusive'),
    @('missing.log', 'default', '', 3, 'a missing log is bad args'))

$wrong = 0
foreach ($r in $rows) {
    $a = @('-NoProfile', '-File', $judge, '-Log', (Join-Path $out $r[0]), '-Case', $r[1])
    if ($r[2]) { $a += @('-Probe', (Join-Path $out $r[2])) }
    $null = & pwsh @a
    $got = $LASTEXITCODE
    $mark = if ($got -eq $r[3]) { 'ok' } else { $wrong++; 'WRONG' }
    Write-Output ("{0,-5} exit={1} want={2}  {3} {4} {5}  - {6}" -f $mark, $got, $r[3], $r[0], $r[1], $r[2], $r[4])
}
Write-Output ("calibration: {0} rows, {1} wrong" -f $rows.Count, $wrong)
foreach ($f in Get-ChildItem $out -File | Sort-Object Name) { Write-Output ("md5 {0} {1}" -f (Get-FileHash -Algorithm MD5 $f.FullName).Hash, $f.Name) }
if ($wrong) { exit 1 }
exit 0
