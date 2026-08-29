<#
.SYNOPSIS
  areka-P0-scope-zorder-pinning 実機サインオフの機械判定（要件 6.1／6.2／6.4／9.4）。

.DESCRIPTION
  有界時間で自動終了させた実機走行のログ 1 本を読み、目視を使わずに 3 点を判定する。

    J1  グループ指定が実際に成立したこと          （-Mode grouped の走行）
    J2  グループ指定の無い走行では是正の記録が 0 件（-Mode default の走行）
    J3  既存のペア機構の記録が従来どおり出ること  （両モード共通）

  判定語はすべて本番の記録行から逐語で取った（`signoff-procedure.md` §3 が出典を持つ）。
  行の字面が変われば本スクリプトは黙って 0 件を返すのではなく「判定不能」を返す——
  沈黙を合格と読ませないためである。

.PARAMETER Log
  走行ログのパス（`areka.exe ... *> <path>` で採ったもの）。

.PARAMETER Mode
  grouped: グループ指定のある走行（J1＋J3 を判定・J2 は対象外）
  default: グループ指定の無い走行（J2＋J3 を判定・J1 は対象外）

.OUTPUTS
  終了コード  0=判定した項目がすべて PASS / 1=いずれかが FAIL /
              2=判定不能（ログ水準の誤り・走行が窓生成まで届いていない等）/ 3=引数不正
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Log,
    [Parameter(Mandatory = $true)][ValidateSet('grouped', 'default')][string]$Mode
)

$ErrorActionPreference = 'Stop'

# ---------------------------------------------------------------------------
# 判定語（本番の記録行から逐語・出典は signoff-procedure.md §3）
#
# ⚠ `action=set source=Descript` のような**連結**を書いてはならない。実際の字面は
#    `action=set group_id=<N> source=Descript` で、間に group_id が挟まる。
# ---------------------------------------------------------------------------
$TAG_GROUP = '[zorder-group]'
$TAG_GROUP_APPLIED = '[zorder-group] applied'
$TAG_GROUP_FIX = '[zorder-group] fix'
$TAG_GROUP_SKIP = '[zorder-group] skip'
$TAG_GROUP_VERIFY_FAILED = '[zorder-group] verify-failed'
$TAG_GROUP_REJECTED = '[zorder-group] rejected'
$TAG_PAIR = '[zorder-pair]'
$TAG_PAIR_OWNER = '[zorder-pair] owner-established'
$TAG_PAIR_FIX = '[zorder-pair] fix'
$TAG_PAIR_SKIP = '[zorder-pair] skip'

if (-not (Test-Path -LiteralPath $Log)) {
    Write-Output "引数不正: ログが見つかりません: $Log"
    exit 3
}

# 「存在するが読めない」（書込中のロック等）を FAIL へ落とさない——判定の失敗と
# 道具の失敗を同じ終了コードにすると、赤を見た人が製品の欠陥だと読む。
try {
    $lines = [System.IO.File]::ReadAllLines((Resolve-Path -LiteralPath $Log).Path)
}
catch {
    Write-Output "引数不正: ログを読めません: $Log"
    Write-Output "  $($_.Exception.Message)"
    exit 3
}
Write-Output "ログ: $Log"
Write-Output "行数: $($lines.Count)   モード: $Mode"

function Select-Tag([string[]]$src, [string]$tag) {
    # -SimpleMatch 相当を自前で行う（角括弧を正規表現として解釈させない）。
    @($src | Where-Object { $_.Contains($tag) })
}

$groupAll = Select-Tag $lines $TAG_GROUP
$groupApplied = Select-Tag $lines $TAG_GROUP_APPLIED
$groupFix = Select-Tag $lines $TAG_GROUP_FIX
$groupSkip = Select-Tag $lines $TAG_GROUP_SKIP
$groupVerifyFailed = Select-Tag $lines $TAG_GROUP_VERIFY_FAILED
$groupRejected = Select-Tag $lines $TAG_GROUP_REJECTED
$pairAll = Select-Tag $lines $TAG_PAIR
$pairOwner = Select-Tag $lines $TAG_PAIR_OWNER
$pairFix = Select-Tag $lines $TAG_PAIR_FIX
$pairSkip = Select-Tag $lines $TAG_PAIR_SKIP

Write-Output ''
Write-Output '== 記録の件数（診断タグ別）=='
Write-Output ("  [zorder-group] 合計 {0}   applied {1} / fix {2} / skip {3} / verify-failed {4} / rejected {5}" -f `
        $groupAll.Count, $groupApplied.Count, $groupFix.Count, $groupSkip.Count, $groupVerifyFailed.Count, $groupRejected.Count)
Write-Output ("  [zorder-pair]  合計 {0}   owner-established {1} / fix {2} / skip {3}" -f `
        $pairAll.Count, $pairOwner.Count, $pairFix.Count, $pairSkip.Count)

# ---------------------------------------------------------------------------
# 走行そのものの成立確認（沈黙を合格と読ませない）
#
# ペア機構の owner-established は、ゴースト窓が 2 スコープ分できた走行なら必ず出る。
# 1 件も無いログは「グループが出なかった」のではなく「そもそも測れていない」——
# ログ水準の誤り・fixture 不在・起動失敗のいずれかである。
# ---------------------------------------------------------------------------
if ($pairAll.Count -eq 0) {
    Write-Output ''
    Write-Output '判定不能: [zorder-pair] の記録が 1 件も無い。'
    Write-Output '  → RUST_LOG に wintf::ecs::window=debug が入っているか、走行が窓生成まで届いたかを確かめること。'
    exit 2
}

$verdicts = @{}

# ---------------------------------------------------------------------------
# J1: グループ指定が実際に成立したこと（-Mode grouped）
#
#   ⑴ 受理の記録がある（action=set・出所は Descript か Tag）
#   ⑵ 是正の記録（fix）があり、その行の中で**指令と実測が一致している**
#      期待列 = head ＋ moves の「動かした窓」を順に並べたもの。
#      fix 行は指令（head/moves）と実測（measured）を同一行に持つので、
#      この一致は 1 行だけで閉じる（2 行の突合を要さない＝design の裁定）。
#   ⑶ 以後の巡で AlreadyOrdered かつ order_ok=true に落ち着いている
#   ⑷ GaveUpAfterFailures が 1 件も無い
# ---------------------------------------------------------------------------
function Test-FixLineCoherent([string]$line) {
    # 例: [zorder-group] fix group_id=0 head=0xA moves=0xB@0xA,0xC@0xB measured=0xA,0xB,0xC
    if ($line -notmatch 'head=(\S+)\s+moves=(\S+)\s+measured=(\S+)') { return $null }
    $head = $Matches[1]
    $moves = $Matches[2]
    $measured = $Matches[3]
    if ($moves -eq '-' -or $measured -eq '-') { return $null }
    $moved = @($moves -split ',' | ForEach-Object { ($_ -split '@')[0] })
    $expected = @($head) + $moved
    return [pscustomobject]@{
        Expected = ($expected -join ',')
        Measured = $measured
        Ok       = (($expected -join ',') -eq $measured)
    }
}

if ($Mode -eq 'grouped') {
    Write-Output ''
    Write-Output '== J1（グループ指定が実際に成立したこと・要件 9.4）=='

    $appliedSet = @($groupApplied | Where-Object { $_.Contains('action=set') })
    $fromDescript = @($appliedSet | Where-Object { $_.Contains('source=Descript') })
    $fromTag = @($appliedSet | Where-Object { $_.Contains('source=Tag') })
    Write-Output ("  受理: action=set {0} 件（source=Descript {1} / source=Tag {2}）" -f $appliedSet.Count, $fromDescript.Count, $fromTag.Count)
    foreach ($l in $appliedSet) { Write-Output "    $l" }

    $coherent = 0
    foreach ($l in $groupFix) {
        $c = Test-FixLineCoherent $l
        if ($null -eq $c) { Write-Output "    (解釈不能な fix 行) $l"; continue }
        if ($c.Ok) { $coherent++ }
        Write-Output ("    fix 指令={0} 実測={1} 一致={2}" -f $c.Expected, $c.Measured, $c.Ok)
    }

    $settled = @($groupSkip | Where-Object { $_.Contains('reason=AlreadyOrdered') -and $_.Contains('order_ok=true') })
    $gaveUp = @($groupSkip | Where-Object { $_.Contains('reason=GaveUpAfterFailures') })
    Write-Output ("  以後の巡: AlreadyOrdered かつ order_ok=true が {0} 件 / GaveUpAfterFailures が {1} 件" -f $settled.Count, $gaveUp.Count)
    Write-Output ("  不一致の記録: verify-failed {0} 件 / 拒否 rejected {1} 件" -f $groupVerifyFailed.Count, $groupRejected.Count)

    if ($appliedSet.Count -eq 0) {
        Write-Output '  → J1: 判定不能（受理の記録が 0 件＝指定がそもそも台帳へ届いていない）'
        $verdicts['J1'] = 'INCONCLUSIVE'
    }
    elseif ($coherent -ge 1 -and $settled.Count -ge 1 -and $gaveUp.Count -eq 0) {
        Write-Output '  → J1: PASS（指令と実測が同一行で一致し、以後の巡が成立側で落ち着いた）'
        $verdicts['J1'] = 'PASS'
    }
    else {
        Write-Output '  → J1: FAIL（指令は出たが、実測が宣言どおりの並びに一致しなかった）'
        $verdicts['J1'] = 'FAIL'
    }
}

# ---------------------------------------------------------------------------
# J2: グループ指定の無い走行では是正の記録が 0 件（要件 6.1／6.2／6.4）
#
# 「0 件」は 5 つの診断タグ**すべて**について主張する。片方だけを数えると、
# 見送りや拒否が出ている走行を「何も起きていない」と読める。
# ---------------------------------------------------------------------------
if ($Mode -eq 'default') {
    Write-Output ''
    Write-Output '== J2（既定＝非強制・グループ系の記録が 0 件・要件 6.1／6.2／6.4）=='
    if ($groupAll.Count -eq 0) {
        Write-Output '  → J2: PASS（[zorder-group] の記録が 1 件も無い＝是正も見送りも拒否も起きていない）'
        $verdicts['J2'] = 'PASS'
    }
    else {
        Write-Output '  → J2: FAIL（グループ指定が無いのにグループ系の記録が出ている）'
        foreach ($l in $groupAll | Select-Object -First 10) { Write-Output "    $l" }
        $verdicts['J2'] = 'FAIL'
    }
}

# ---------------------------------------------------------------------------
# J3: 既存のペア機構の記録が従来どおり出ること（要件 9.5）
#
# ⚠ 判定に origin=zorder-pair の件数を使ってはならない。グループ発行の指令は
#    凍結済み pair_fix_command 経由で書かれるため、書込側の origin 欄では
#    グループ発行分がペア発行分に見える（tasks.md 申し送り 4.1）。
#    判定は [zorder-pair] の診断タグだけで行う。
#
# 「従来どおり」の錨は本 spec 導入**前**の記録である
#   .kiro/specs/completed/areka-P0-ghost-window-zorder/verification/plan-a-gate.md:51-54
# ——欄立て（entity= / peer= / owned_hwnd= / owner_hwnd= / measured_prev=）が現行と同一。
# 件数だけを数えると欄が 1 つ落ちても素通りするので、欄立ても機械で照合する。
#
# ⚠ 欄名は**語の頭から**照合する。素の部分一致だと `entity=` が `char_entity=` にも当たり、
#   改名を素通りさせる——`char_entity=` / `balloon_entity=` は導入前の
#   `[zorder-pair] declared`（plan-a-gate.md:49-50）が実際に使っていた語なので仮想の話ではない。
#   よって `\b` を付けた正規表現で見る（`_` は `\w` なので `char_entity=` は `\bentity=` に当たらない）。
#
# ⚠ 照合するのは**欄名だけ**であって、値は見ていない（`measured_prev=GARBAGE` は通る）。
#   値の妥当性は J1 の fix 行（指令と実測の突合）が担う。
# ---------------------------------------------------------------------------
$PAIR_OWNER_FIELDS = @('entity=', 'peer=', 'owned_hwnd=', 'owner_hwnd=', 'measured_prev=')

Write-Output ''
Write-Output '== J3（既存ペア機構の記録が従来どおり・要件 9.5）=='
$decision = $pairFix.Count + $pairSkip.Count
Write-Output ("  owner-established {0} 件（2 スコープ分＝2 が既定形）/ fix+skip {1} 件" -f $pairOwner.Count, $decision)
foreach ($l in $pairOwner) { Write-Output "    $l" }

# 欄立ての照合（本 spec 導入前の記録と同じ 5 欄が全行に揃っているか）
$fieldsOk = $true
foreach ($l in $pairOwner) {
    $lost = @($PAIR_OWNER_FIELDS | Where-Object { $l -notmatch ('\b' + [regex]::Escape($_)) })
    if ($lost.Count -gt 0) {
        $fieldsOk = $false
        Write-Output ("    ⚠ 欄が欠けている: {0}" -f ($lost -join ' '))
    }
}
Write-Output ("  欄立て（導入前 plan-a-gate.md:51-54 と同一の 5 欄）: {0}" -f $(if ($fieldsOk) { '一致' } else { '不一致' }))

if ($pairOwner.Count -eq 2 -and $decision -ge 1 -and $fieldsOk) {
    Write-Output '  → J3: PASS'
    $verdicts['J3'] = 'PASS'
}
elseif ($pairOwner.Count -eq 0) {
    Write-Output '  → J3: 判定不能（ペア確立の記録が無い）'
    $verdicts['J3'] = 'INCONCLUSIVE'
}
else {
    Write-Output '  → J3: FAIL'
    $verdicts['J3'] = 'FAIL'
}

Write-Output ''
Write-Output ('総合: ' + (($verdicts.Keys | Sort-Object | ForEach-Object { "$_=$($verdicts[$_])" }) -join '  '))

if ($verdicts.Values -contains 'FAIL') { exit 1 }
if ($verdicts.Values -contains 'INCONCLUSIVE') { exit 2 }
exit 0
