<#
.SYNOPSIS
  areka-P0-zorder-scope-inversion の実機の確かめの判定（要件 8.4・8.5・8.6）。

.DESCRIPTION
  有界の時間で自動で終わった走行のログ 1 本（と、同じ走行で取った前面走査の控え）を読み、
  走行の種類ごとに判定する。手順は signoff-procedure.md（同じフォルダ）が正本。

    inverted  `\![reset,zorder]\![set,zorder,s0,b0,s1]`（ぱすたと同じ指定・要件 8.4・8.5）
    explicit  `\![reset,zorder]\![set,zorder,b0,s0,b1,s1]`（逆転の無い明示の記法・要件 8.6）
    numeric   `\![reset,zorder]\![set,zorder,0,1]`（数字だけの記法・要件 8.6）
    default   指定なし（要件 8.6）

  判定の語はすべて冠（角括弧）込みで照合する（パスや設定の行を巻き込まないため）。
  端末へ出す文は ASCII だけ。

.PARAMETER Log
  走行のログ（signoff-run.ps1 が <case>\run.log に採ったもの）。

.PARAMETER Case
  inverted / explicit / numeric / default。

.PARAMETER Probe
  前面走査の控え（signoff-run.ps1 が <case>\probe.txt に採ったもの）。inverted では必須。
  1 行 1 回: `probe label=<名> order=<tok>,<tok>,...`（手前から奥。tok は areka の見えている窓の
  `0xHEX`、または areka 以外の見えている窓の連なりを 1 つに畳んだ `*`）。

.OUTPUTS
  終了コード 0=すべて PASS / 1=いずれかが FAIL / 2=判定不能（材料が無い）/ 3=引数不正
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Log,
    [Parameter(Mandatory = $true)][ValidateSet('inverted', 'explicit', 'numeric', 'default')][string]$Case,
    [string]$Probe
)
$ErrorActionPreference = 'Stop'

# 判定の語（冠込み）。出典: crates/wintf/src/ecs/window/zorder_chain_diag.rs と zorder_pair_diag.rs の定数。
# ⚠ `[zorder-chain] linked` は `[zorder-chain] unlinked` に当たらない（`] linked` の区切りが別物）。
$TAG_GROUP = '[zorder-group]'
$TAG_APPLIED = '[zorder-group] applied'
$TAG_REJECTED = '[zorder-group] rejected'
$TAG_CHAIN = '[zorder-chain]'
$TAG_LINKED = '[zorder-chain] linked'
$TAG_SETTLED = '[zorder-chain] settled'
$TAG_RELEASED = '[zorder-chain] pair-released'
$TAG_FAILS = @('[zorder-chain] link-failed', '[zorder-chain] unlink-failed', '[zorder-chain] link-refused')
$TAG_PAIR = '[zorder-pair]'
$TAG_OWNER = '[zorder-pair] owner-established'

# 受理の行が持つべき字面（members= と normalized= は行の中で隣り合う）。
# 出典: zorder_drain_written_order_tests.rs（inverted・explicit）と t_zdw08・t_zdr04（numeric）。
$EXPECT_APPLIED = @{
    inverted = 'members=s0,b0,b1,s1 normalized=0:inverted,1:false+b1'
    explicit = 'members=b0,s0,b1,s1 normalized=0:false,1:false'
    numeric  = 'members=b0,s0,b1,s1 normalized=-'
}
# inverted の前面走査で揃っていなければならない場面（signoff-procedure.md の走行の段）。
$REQUIRED_PROBES = @('boot', 'press-s0', 'press-b0', 'drag-s0', 'other-app', 'returned')

if (-not (Test-Path -LiteralPath $Log)) { Write-Output "BAD-ARGS: log not found: $Log"; exit 3 }
if ($Probe -and -not (Test-Path -LiteralPath $Probe)) { Write-Output "BAD-ARGS: probe not found: $Probe"; exit 3 }
try { $lines = [IO.File]::ReadAllLines((Resolve-Path -LiteralPath $Log).Path) }
catch { Write-Output "BAD-ARGS: cannot read log: $Log"; exit 3 }

function Tagged([string]$tag) { @($lines | Where-Object { $_.Contains($tag) }) }
function Field([string]$line, [string]$name) {
    # 語の頭から照合する（`owned_hwnd=` が `xowned_hwnd=` に当たらないように）。
    if ($line -match ('(?<![\w])' + [regex]::Escape($name) + '=(\S+)')) { $Matches[1] } else { $null }
}

function HwndOf([string]$entity, [string]$hwnd) {
    if ($hwnd -and $hwnd -ne '-') { return $hwnd }
    foreach ($l in $linked) {
        if ((Field $l 'owned') -eq $entity) { return (Field $l 'owned_hwnd') }
        if ((Field $l 'owner') -eq $entity) { return (Field $l 'owner_hwnd') }
    }
    return $null
}

$verdicts = [ordered]@{}
function Verdict([string]$k, [string]$v, [string]$why) { $verdicts[$k] = $v; Write-Output "  -> $k $v ($why)" }

$pair = Tagged $TAG_PAIR
$owners = Tagged $TAG_OWNER
$group = Tagged $TAG_GROUP
$chain = Tagged $TAG_CHAIN
$applied = Tagged $TAG_APPLIED
$rejected = Tagged $TAG_REJECTED
$linked = Tagged $TAG_LINKED
$settled = Tagged $TAG_SETTLED
$released = Tagged $TAG_RELEASED
$fails = @($TAG_FAILS | ForEach-Object { Tagged $_ })

Write-Output "log: $Log"
Write-Output "case: $Case  lines: $($lines.Count)"
Write-Output ("counts: group={0} applied={1} rejected={2} chain={3} linked={4} settled={5} pair-released={6} failures={7} pair={8} owner-established={9}" -f `
        $group.Count, $applied.Count, $rejected.Count, $chain.Count, $linked.Count, $settled.Count, $released.Count, $fails.Count, $pair.Count, $owners.Count)

# 走行が窓を作るところまで届いていない記録は、合格とも不合格とも読まない。
if ($pair.Count -eq 0) {
    Write-Output 'INCONCLUSIVE: no [zorder-pair] line. check RUST_LOG has wintf::ecs::window=debug and the run reached the windows.'
    exit 2
}

# 収まった行: 宣言と直後の実測が同じ行に載る（4 欄の並びは兄弟テストが固定）。
$settledRows = @(foreach ($l in $settled) {
        if ($l -notmatch 'declared=(\S+)\s+measured=(\S+)') { [pscustomobject]@{ Bad = $true; Line = $l }; continue }
        $d = $Matches[1]; $m = $Matches[2]
        [pscustomobject]@{ Bad = $false; Declared = $d; List = @($d -split ','); Ok = ($d -ne '-' -and $d -eq $m); Measured = $m }
    })
# 正の宣言列: 宣言と実測が一致した行のうち最も長いもの（同じ長さなら後のもの）。前面走査の期待にも使う。
$canon = $null
foreach ($r in $settledRows) { if (-not $r.Bad -and $r.Ok -and ($null -eq $canon -or $r.List.Count -ge $canon.List.Count)) { $canon = $r } }

$s0 = $null; $b0 = $null
if ($Case -ne 'default') {
    Write-Output ''
    Write-Output '== A applied =='
    $want = $EXPECT_APPLIED[$Case]
    $hit = @($applied | Where-Object { $_.Contains('action=set') -and $_.Contains($want) })
    # 「書いた順を採らなかった」の旧語 `true` が normalized= 欄に 1 つも出ないこと（要件 1.4）。
    $oldWord = @($applied | Where-Object { (Field $_ 'normalized') -match '(^|,)\d+:true' })
    Write-Output "  expect: $want   hit: $($hit.Count)   old-word(true): $($oldWord.Count)   rejected: $($rejected.Count)"
    if ($applied.Count -eq 0) { Verdict 'A' 'INCONCLUSIVE' 'no applied line: the setting did not reach the ledger' }
    elseif ($hit.Count -ge 1 -and $oldWord.Count -eq 0 -and $rejected.Count -eq 0) { Verdict 'A' 'PASS' 'the applied line carries the expected words' }
    else { Verdict 'A' 'FAIL' 'applied line differs, old word true seen, or rejected' }

    Write-Output ''
    Write-Output '== R pair-released =='
    if ($Case -eq 'inverted') {
        # 逆転したスコープ 0 の組（バルーン b0 がキャラクター s0 の持ち主の結び目）を鎖のために外す。
        # 結び目がまだ張られていなかった組では owner_hwnd が番兵 `-` になる（zorder_chain_apply.rs の
        # 外しの手順）。そのときは同じ実体を名指す linked の行から窓を引く（entity=バルーン・peer=キャラクター）。
        $r0 = $released | Select-Object -First 1
        if ($r0) {
            $b0 = HwndOf (Field $r0 'entity') (Field $r0 'owned_hwnd')
            $s0 = HwndOf (Field $r0 'peer') (Field $r0 'owner_hwnd')
        }
        Write-Output "  s0=$s0 b0=$b0 (from the first pair-released line)"
        if ($released.Count -ge 1 -and $s0 -and $b0 -and $s0 -ne '-' -and $b0 -ne '-') { Verdict 'R' 'PASS' 'the inverted pair was released for the chain' }
        else { Verdict 'R' 'FAIL' 'no pair-released line with both handles' }
    }
    else {
        # 逆転の無い指定では組は隣り合うので外さない（本機能の前と同じ）。
        if ($released.Count -eq 0) { Verdict 'R' 'PASS' 'no pair is released (same as before the feature)' }
        else { Verdict 'R' 'FAIL' 'a pair was released though every pair is adjacent' }
    }

    Write-Output ''
    Write-Output '== L linked =='
    $coherent = 0
    foreach ($l in $linked) {
        $ok = $false
        if ($l -match 'owned_hwnd=(\S+)\s+owner_hwnd=(\S+)\s+pos=(\d+)/(\d+)') {
            $o = $Matches[1]; $w = $Matches[2]; $pos = [int]$Matches[3]; $tot = [int]$Matches[4]
            # 鎖は「手前の窓が 1 つ奥の窓に持たれる」一直線: 宣言列の pos 番目 → pos+1 番目。
            foreach ($r in $settledRows) {
                if ($r.Bad -or $r.List.Count -ne $tot -or $pos -lt 1 -or $pos -ge $tot) { continue }
                if ($r.List[$pos - 1] -eq $o -and $r.List[$pos] -eq $w) { $ok = $true; break }
            }
        }
        if ($ok) { $coherent++ } else { Write-Output "  not as declared: $l" }
    }
    Write-Output "  linked: $($linked.Count)  as declared: $coherent"
    if ($linked.Count -ge 1 -and $coherent -eq $linked.Count) { Verdict 'L' 'PASS' 'every link matches a declared neighbour' }
    else { Verdict 'L' 'FAIL' 'no link, or a link off the declared chain' }

    Write-Output ''
    Write-Output '== S settled (declared column == measured column) =='
    # 宣言列ごとの最後の settled で判定する（窓の出入りの過渡は通し、収まらなかった列は赤）。
    $terminal = [ordered]@{}
    $bad = 0
    foreach ($r in $settledRows) {
        if ($r.Bad) { $bad++; Write-Output "  unreadable: $($r.Line)"; continue }
        $terminal[$r.Declared] = $r
    }
    $off = @($terminal.Values | Where-Object { -not $_.Ok })
    foreach ($r in $off) { Write-Output "  not settled: declared=$($r.Declared) last-measured=$($r.Measured)" }
    Write-Output "  settled: $($settled.Count)  declared lists: $($terminal.Count)  not settled: $($off.Count)  unreadable: $bad  failures: $($fails.Count)"
    foreach ($f in $fails) { Write-Output "  failure: $f" }
    if ($null -ne $canon) { Write-Output "  canonical declared: $($canon.Declared)" }
    $front = $true
    if ($Case -eq 'inverted') {
        # 要件 8.4: 手前から s0 -> b0 -> ...(b1) -> s1。宣言列の頭 2 つが外した組の s0・b0 であること。
        $front = ($null -ne $canon -and $canon.List.Count -ge 3 -and $canon.List[0] -eq $s0 -and $canon.List[1] -eq $b0)
        Write-Output "  front: s0,b0 lead the canonical list: $front"
    }
    if ($terminal.Count -ge 1 -and $off.Count -eq 0 -and $bad -eq 0 -and $fails.Count -eq 0 -and $null -ne $canon -and $canon.List.Count -ge 3 -and $front) {
        Verdict 'S' 'PASS' 'every declared list settled as declared'
    }
    else { Verdict 'S' 'FAIL' 'a list did not settle, a failure was logged, or the front order is wrong' }
}
else {
    Write-Output ''
    Write-Output '== D no setting =='
    # 指定が無ければ受理も鎖も 1 行も出ない（本機能の前と同じ）。
    if ($group.Count -eq 0 -and $chain.Count -eq 0) { Verdict 'D' 'PASS' 'no [zorder-group] and no [zorder-chain] line' }
    else { Verdict 'D' 'FAIL' 'group or chain lines appear without a setting' }
}

# 前面走査: 宣言列の窓が宣言の順に並び、間に他のアプリの窓（`*`）が挟まらない（塊のまま）。
if ($Probe) {
    Write-Output ''
    Write-Output '== P front scan =='
    $probes = @([IO.File]::ReadAllLines((Resolve-Path -LiteralPath $Probe).Path) | Where-Object { $_ -match '^probe ' })
    $labels = @()
    $pbad = 0
    if ($Case -eq 'default') {
        # 組ごとにバルーン（owned）がキャラクター（owner）のすぐ手前（間に何も挟まらない）。
        $pairs = @($owners | ForEach-Object { , @((Field $_ 'owned_hwnd'), (Field $_ 'owner_hwnd')) })
    }
    foreach ($p in $probes) {
        $label = Field $p 'label'; $order = Field $p 'order'; $labels += $label
        $toks = if ($order) { @($order -split ',') } else { @() }
        $ok = $true; $why = ''
        if ($Case -eq 'default') {
            $seen = 0
            foreach ($pr in $pairs) {
                $i = [array]::IndexOf($toks, $pr[0]); $j = [array]::IndexOf($toks, $pr[1])
                if ($i -lt 0 -or $j -lt 0) { continue }
                $seen++
                if ($j -ne $i + 1) { $ok = $false; $why = "pair $($pr[0])/$($pr[1]) not adjacent" }
            }
            if ($seen -eq 0) { $ok = $false; $why = 'no pair visible' }
        }
        elseif ($null -eq $canon) { $ok = $false; $why = 'no canonical declared list' }
        else {
            $idx = @($canon.List | ForEach-Object { [array]::IndexOf($toks, $_) } | Where-Object { $_ -ge 0 })
            $sorted = @($idx | Sort-Object)
            if ($idx.Count -lt 2) { $ok = $false; $why = "only $($idx.Count) declared window visible" }
            elseif (($idx -join ',') -ne ($sorted -join ',')) { $ok = $false; $why = 'order differs from declared' }
            elseif (@($toks[$sorted[0]..$sorted[-1]] | Where-Object { $_ -eq '*' }).Count -gt 0) { $ok = $false; $why = 'another app window sits inside the chain' }
            if ($ok -and $Case -eq 'inverted') {
                # 要件 5.2: s0 を押してもバルーン b0 を s0 より手前へ出さない（両方見えていること）。
                $i = [array]::IndexOf($toks, $s0); $j = [array]::IndexOf($toks, $b0)
                if ($i -lt 0 -or $j -lt 0) { $ok = $false; $why = 's0 or b0 not visible' }
                elseif ($i -gt $j) { $ok = $false; $why = 'b0 is in front of s0' }
            }
        }
        if (-not $ok) { $pbad++ }
        Write-Output ("  {0,-10} {1}  {2}  {3}" -f $label, $(if ($ok) { 'ok' } else { 'NG' }), $order, $why)
    }
    $missing = @()
    if ($Case -eq 'inverted') { $missing = @($REQUIRED_PROBES | Where-Object { $labels -notcontains $_ }) }
    if ($probes.Count -eq 0) { Verdict 'P' 'INCONCLUSIVE' 'no probe line' }
    elseif ($missing.Count -gt 0) { Verdict 'P' 'INCONCLUSIVE' "missing probes: $($missing -join ',')" }
    elseif ($pbad -eq 0) { Verdict 'P' 'PASS' 'every scan keeps the declared order as one block' }
    else { Verdict 'P' 'FAIL' "$pbad scan(s) break the order" }
}
elseif ($Case -eq 'inverted') {
    Write-Output ''
    Write-Output '== P front scan =='
    Verdict 'P' 'INCONCLUSIVE' 'the inverted case needs -Probe'
}

Write-Output ''
Write-Output ('total: ' + (($verdicts.Keys | ForEach-Object { "$_=$($verdicts[$_])" }) -join ' '))
if ($verdicts.Values -contains 'FAIL') { exit 1 }
if ($verdicts.Values -contains 'INCONCLUSIVE') { exit 2 }
exit 0
