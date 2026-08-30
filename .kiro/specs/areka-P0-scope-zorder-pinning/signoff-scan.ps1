<#
.SYNOPSIS
  areka-P0-scope-zorder-pinning 実機サインオフの機械判定（要件 6.1／6.2／6.4／9.4）。

.DESCRIPTION
  有界時間で自動終了させた実機走行のログ 1 本を読み、目視を使わずに 3 点を判定する。

    J1  グループ指定が実際に成立したこと          （-Mode grouped の走行）
    J2  グループ指定の無い走行では鎖の記録が 0 件 （-Mode default の走行）
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
#
# ⚠ 語は必ず**冠（角括弧）込み**で照合する。この作業ツリーの名前は
#    `areka-p0-balloon-visibility-…` のようにハイフン付きの語を含み、走行ログは毎行の
#    ように作業ツリーの絶対パスを吐く。冠を外した `linked` や `settled` で数えると
#    設定行やパス行を巻き込む（`real-machine-signoff.md` §1.1 の転記ミス 1 件目が実例）。
#
# ⚠ 冠込みでも `[zorder-chain] linked` は `[zorder-chain] unlinked` に、
#    `[zorder-chain] link-failed` は `[zorder-chain] unlink-failed` に当たらない
#    ——`] linked` / `] link-failed` という区切り込みの字面が別物だからである。
#
# 2026-08-30（task 6.1）: 判定モデルが「毎巡の観測＋是正」から「所有の鎖」へ変わったので、
# 是正モデルの 3 語（`fix` / `skip` / `verify-failed`）を鎖の 3 語へ差し替えた。
# 受理・拒否・ペア系 4 本・出所の別・終了コードの体系は**据え置き**である（要件 9.5）。
#
# design の差し替え表からの相違 2 点（どちらも判定語そのものではなく変数名・追加）:
#   ⑴ design は `$TAG_CHAIN_FAILED` と書いているが `$TAG_CHAIN_LINK_FAILED` にした。
#      鎖には失敗タグが 2 つ（`link-failed` / `unlink-failed`）あり、`FAILED` だけでは
#      どちらか読めない。**照合する字面は design のとおり `[zorder-chain] link-failed`。**
#   ⑵ `$TAG_CHAIN`（冠だけ）を足した。J2 の「0 件」を冠ごと主張するために要る——
#      語ごとに数えると、鎖に語が増えたとき数え漏らす。
# ---------------------------------------------------------------------------
$TAG_GROUP = '[zorder-group]'
$TAG_GROUP_APPLIED = '[zorder-group] applied'
$TAG_GROUP_REJECTED = '[zorder-group] rejected'
$TAG_CHAIN = '[zorder-chain]'
$TAG_CHAIN_LINKED = '[zorder-chain] linked'
$TAG_CHAIN_SETTLED = '[zorder-chain] settled'
$TAG_CHAIN_LINK_FAILED = '[zorder-chain] link-failed'
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
$groupRejected = Select-Tag $lines $TAG_GROUP_REJECTED
$chainAll = Select-Tag $lines $TAG_CHAIN
$chainLinked = Select-Tag $lines $TAG_CHAIN_LINKED
$chainSettled = Select-Tag $lines $TAG_CHAIN_SETTLED
$chainLinkFailed = Select-Tag $lines $TAG_CHAIN_LINK_FAILED
$pairAll = Select-Tag $lines $TAG_PAIR
$pairOwner = Select-Tag $lines $TAG_PAIR_OWNER
$pairFix = Select-Tag $lines $TAG_PAIR_FIX
$pairSkip = Select-Tag $lines $TAG_PAIR_SKIP

Write-Output ''
Write-Output '== 記録の件数（診断タグ別）=='
Write-Output ("  [zorder-group] 合計 {0}   applied {1} / rejected {2}" -f `
        $groupAll.Count, $groupApplied.Count, $groupRejected.Count)
Write-Output ("  [zorder-chain] 合計 {0}   linked {1} / settled {2} / link-failed {3}" -f `
        $chainAll.Count, $chainLinked.Count, $chainSettled.Count, $chainLinkFailed.Count)
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
#   ⑵ 繋いだ記録（linked）が 1 件以上あり、**すべての行が宣言どおりの位置**である
#      ——`pos=i/n` の n が収まった行の `declared=` の要素数と一致し、その列の
#      i 番目が `owned_hwnd=`、その 1 つ後ろが `owner_hwnd=` であること。
#      鎖は「手前の窓が 1 つ奥の窓を所有する」直線なので、宣言列と繋ぎは
#      この形でしか噛み合わない。位置を偽る繋ぎはここで赤くなる。
#   ⑶ 収まった記録（settled）があり、**宣言列ごとの最後の 1 行**で
#      **宣言と実測が一致している**（終状態で測る・量化子の裁定は下記）。
#      settled 行は宣言（declared）と直後の実測（measured）を同一行に持つので、
#      この一致は 1 行だけで閉じる（2 行の突合を要さない＝design の裁定）。
#   ⑷ 張り失敗（link-failed）が 1 件も無い
#
# ================== ⑶ の量化子の裁定（2026-08-30・review round 1）==================
#
# 採った形: **宣言列ごとの「最後の settled」がすべて一致**（＝終状態の全称）。
#
# ⚠ **「1 本でも一致すれば緑」にしてはならない**（存在量化）。`settled` は**結果の並びの
#    食い違いを報せる唯一の記録**である——`link-failed` が載せるのは Win32 呼出そのものの
#    失敗だけで、呼出が成功して並びが宣言どおりにならなかった場合は `settled` にしか出ない
#    （design の Error Strategy）。しかも 6.2 の走行は `settled` が複数本出るのが前提である
#    （`signoff-procedure.md` §4 が走行長を「繋ぎと収まりの機会が何度あったか」で測っている）。
#    存在量化にすると **N 本中 1 本当たれば緑**になり、初版を実機 NO-GO にした症状
#    ——宣言した並びが実機で成立しない——をそのまま見逃す。
#
# ⚠ **素の全称（不一致 0 本）にもしない。** 健全な走行でも良性の不一致が出る経路が
#    **構造として在る**（推測ではなくソースから辿れる）:
#      ・`zorder_chain_apply.rs:198-228` ——撤去が 1 本でも実行された巡は `acted` が立ち、
#        その巡でも `nudge_and_measure` が走って `settled` を 1 本出す。
#      ・`zorder_chain_apply.rs:512-525` `measure_chain_order` ——前面走査が**実際に出会った**
#        窓だけを返す。よって窓が去る途中の巡や走査が打切りになった巡では `measured` が
#        `declared` より**短く**なる。
#    つまり窓の出入りの過渡で不一致が 1 本出るのは異常ではない。素の全称はそこで赤を出し、
#    「健全な走行で必ず赤くなる門」＝常に赤い道具になる。
#
# 「収まった」は**落ち着いた先**の意である。よって宣言列ごとに**最後の 1 行**を見て、
# その終状態が宣言どおりであることを全称で要求する。過渡の不一致は通し、
# **収束しなかった宣言列は 1 つでも残っていれば赤**にする。
#
# ⚠ 良性の不一致に例外を与えるのは**実測を示してから**である。現時点で実測は無い
#    （鎖の語彙の実走は task 6.2）ので、上の 2 経路は「在りうる」までしか言っていない。
#    終状態の全称はその実測を待たずに安全側へ倒せる形なので、これを採った。
#
# ================== 被覆は主張しない（FINDING 4）==================
#
# ⑵ が見るのは「各 `linked` 行が宣言列のどこかの隣接対に当たること」だけであり、
# **宣言された横断 edge がすべて出たか（被覆）は見ていない**。同じ繋ぎが 3 本重複しても通る。
# 見ないのは、**繋ぎの本数がログから導けない**ためである——同一スコープの
# （バルーン, キャラ窓）対は既存ペア機構が張るので鎖の繋ぎに数えず（design DD-2）、
# その対がいくつ在るかは記録に出ないスコープ構造が決める。よって「宣言列の窓の枚数 − 1」は
# 期待本数にならない。**被覆の主張は決定論テスト側（`compose_chain` の分岐網羅）が担う。**
#
# ================== `nudge_ok=` は読まない（FINDING 5）==================
#
# `settled` の `nudge_ok=` は後押しそのものの成否である（design の Error Strategy——
# 後押しの失敗は記録して続行する）。**判定には使わない**: 後押しが失敗しても終状態が
# 宣言どおりなら走行は成立しており、失敗して並びが崩れたなら ⑶ が終状態で捕まえる。
# よってこの欄は**診断の材料**であって門ではない（`skipped` の件数を数えない禁・
# `origin=` を使わない禁と同じ筋の据え置きである）。**印字はする。**
#
# ⚠ 初版が持っていた「既に並んでいた（AlreadyOrdered）」「失敗続きで諦めた
#    （GaveUpAfterFailures）」の 2 つは**撤去した**。どちらも毎巡の観測と是正を前提とした
#    語であり、鎖の下には存在しない——鎖は OS が維持するので「以後の巡」が無い。
#
# ⚠ 見送り（`[zorder-chain] skipped`）の**件数**を判定条件にしてはならない。連呼の抑止は
#    真偽値 1 枚なので、前の待ちが解けないうちに始まった次の待ちは記録されないことがある
#    （tasks.md 申し送り 2.3）。存在を印字するのはよいが、数えて判定に使うと偽の赤が出る。
# ---------------------------------------------------------------------------

# 収まった行の切り出し（design.md「実機サインオフの改訂」が定める 4 欄の並び）。
#
# ⚠ 実際の行は 4 欄の**後ろ**に `nudge_ok=` を持つ。後ろに足す約束は
#    `zorder_chain_diag.rs` の兄弟テスト（4 欄の隣接を字面で固定）が守っているので、
#    この正規表現は 4 欄だけを見ればよい。欄を**間へ**割り込ませると静かに 0 件になる。
function Get-SettledLine([string]$line) {
    # 例: [zorder-chain] settled nudged_hwnd=0xA0 insert_after=0xB0 declared=0xA0,0xB0 measured=0xA0,0xB0 nudge_ok=true
    if ($line -notmatch 'nudged_hwnd=(\S+)\s+insert_after=(\S+)\s+declared=(\S+)\s+measured=(\S+)') { return $null }
    # ⚠ **4 欄を先に全部ローカルへ退避する。** `-match` は成功するたび `$Matches` を
    #    丸ごと差し替えるので、次の `-match` より後に `$Matches[n]` を読むと**別の正規表現の
    #    捕獲**を読む。実際に踏んだ（レビュー round 2 の FINDING 3）——`nudge_ok` を先に
    #    評価したせいで `Nudged` に `true` が入り `InsertAfter` が空になっていた。
    #    `insert_after` は DD-3 の錨であり、判定に出ていなくても壊れたままにはできない。
    $nudged = $Matches[1]
    $after = $Matches[2]
    $declared = $Matches[3]
    $measured = $Matches[4]
    # `nudge_ok=` は 4 欄の**後ろ**に在る別欄。判定には使わない（上の裁定）が、
    # 診断の材料として拾って印字する。無い行では番兵にする。
    $nudge = if ($line -match 'nudge_ok=(\S+)') { $Matches[1] } else { '-' }
    return [pscustomobject]@{
        Nudged       = $nudged
        InsertAfter  = $after
        Declared     = $declared
        Measured     = $measured
        NudgeOk      = $nudge
        DeclaredList = @($declared -split ',')
        # 番兵（`-`）は「その経路にはその値が無い」であって一致ではない。
        Ok           = ($declared -ne '-' -and $measured -ne '-' -and $declared -eq $measured)
    }
}

# 繋いだ行の切り出し（4 欄は `zorder_chain_diag.rs` の書式でこの順に隣り合う）。
function Get-LinkedLine([string]$line) {
    # 例: [zorder-chain] linked segment=g0 owned=23v0 owner=25v0 owned_hwnd=0xA0 owner_hwnd=0xB0 pos=2/3
    if ($line -notmatch 'owned_hwnd=(\S+)\s+owner_hwnd=(\S+)\s+pos=(\d+)/(\d+)') { return $null }
    return [pscustomobject]@{
        Owned = $Matches[1]
        Owner = $Matches[2]
        Pos   = [int]$Matches[3]
        Total = [int]$Matches[4]
    }
}

# その繋ぎが、宣言された鎖のどれか 1 本の「i 番目 → i+1 番目」に一致するか。
function Test-LinkedAgainstDeclared($edge, $declarations) {
    foreach ($d in $declarations) {
        $list = $d.DeclaredList
        if ($list.Count -ne $edge.Total) { continue }
        # 最も奥の窓は所有先を持たないので `pos=n` は出ない（1 ≤ i < n）。
        if ($edge.Pos -lt 1 -or $edge.Pos -ge $list.Count) { continue }
        if ($list[$edge.Pos - 1] -eq $edge.Owned -and $list[$edge.Pos] -eq $edge.Owner) { return $true }
    }
    return $false
}

if ($Mode -eq 'grouped') {
    Write-Output ''
    Write-Output '== J1（グループ指定が実際に成立したこと・要件 9.4）=='

    $appliedSet = @($groupApplied | Where-Object { $_.Contains('action=set') })
    $fromDescript = @($appliedSet | Where-Object { $_.Contains('source=Descript') })
    $fromTag = @($appliedSet | Where-Object { $_.Contains('source=Tag') })
    Write-Output ("  受理: action=set {0} 件（source=Descript {1} / source=Tag {2}）" -f $appliedSet.Count, $fromDescript.Count, $fromTag.Count)
    foreach ($l in $appliedSet) { Write-Output "    $l" }

    # 収まった行——宣言と実測の一致は 1 行の中で閉じる。
    # 判定は**宣言列ごとの最後の 1 行**（終状態）で行う。過渡の不一致は通し、
    # 収束しなかった宣言列が 1 つでも残っていれば赤にする（量化子の裁定は上記）。
    $declarations = @()
    $settledOk = 0
    $settledUnreadable = 0
    $terminal = [ordered]@{}     # 宣言列の字面 → その列の最後の settled
    foreach ($l in $chainSettled) {
        $sl = Get-SettledLine $l
        if ($null -eq $sl) { $settledUnreadable++; Write-Output "    (解釈不能な settled 行) $l"; continue }
        $declarations += $sl
        if ($sl.Ok) { $settledOk++ }
        $terminal[$sl.Declared] = $sl        # 同じ鍵への再代入で「最後の 1 行」が残る
        # 4 欄すべてを印字する——`nudged_hwnd` と `insert_after`（DD-3 の錨）は判定に出ないので、
        # 印字しないと切り出しが壊れても誰も気づかない（FINDING 3 が実際にそうなった）。
        Write-Output ("    settled 後押し={0} 錨={1} 宣言={2} 実測={3} 一致={4} nudge_ok={5}" -f `
                $sl.Nudged, $sl.InsertAfter, $sl.Declared, $sl.Measured, $sl.Ok, $sl.NudgeOk)
    }

    # 終状態の全称——宣言列ごとの最後の 1 行がすべて一致していること。
    $terminalOk = 0
    $terminalBad = 0
    foreach ($k in $terminal.Keys) {
        if ($terminal[$k].Ok) { $terminalOk++ }
        else {
            $terminalBad++
            Write-Output ("    ⚠ 終状態が宣言どおりでない宣言列: 宣言={0} 最後の実測={1}" -f $k, $terminal[$k].Measured)
        }
    }
    Write-Output ("  終状態（宣言列ごとの最後の settled）: 宣言列 {0} 本中 一致 {1} / 不一致 {2}" -f $terminal.Count, $terminalOk, $terminalBad)

    # 繋いだ行——宣言どおりの位置に出ているか（本数はここで数える）。
    $linkedCoherent = 0
    $linkedUnreadable = 0
    foreach ($l in $chainLinked) {
        $e = Get-LinkedLine $l
        if ($null -eq $e) { $linkedUnreadable++; Write-Output "    (解釈不能な linked 行) $l"; continue }
        $ok = Test-LinkedAgainstDeclared $e $declarations
        if ($ok) { $linkedCoherent++ }
        Write-Output ("    linked {0}→{1} pos={2}/{3} 宣言どおり={4}" -f $e.Owned, $e.Owner, $e.Pos, $e.Total, $ok)
    }
    Write-Output ("  繋いだ本数: {0} 件（うち宣言どおり {1} / 解釈不能 {2}）" -f $chainLinked.Count, $linkedCoherent, $linkedUnreadable)
    Write-Output ("  収まった本数: {0} 件（うち宣言と実測が一致 {1} / 解釈不能 {2}）——**件数は判定に使わない。判定は終状態の全称である**" -f $chainSettled.Count, $settledOk, $settledUnreadable)
    Write-Output ("  失敗と拒否の記録: link-failed {0} 件 / rejected {1} 件" -f $chainLinkFailed.Count, $groupRejected.Count)

    if ($appliedSet.Count -eq 0) {
        Write-Output '  → J1: 判定不能（受理の記録が 0 件＝指定がそもそも台帳へ届いていない）'
        $verdicts['J1'] = 'INCONCLUSIVE'
    }
    elseif ($chainLinked.Count -ge 1 -and $linkedUnreadable -eq 0 -and $linkedCoherent -eq $chainLinked.Count `
            -and $terminal.Count -ge 1 -and $terminalBad -eq 0 -and $settledUnreadable -eq 0 `
            -and $chainLinkFailed.Count -eq 0) {
        Write-Output '  → J1: PASS（繋いだ行が宣言どおりの本数出て、宣言列ごとの終状態がすべて宣言と実測で一致した）'
        $verdicts['J1'] = 'PASS'
    }
    else {
        Write-Output '  → J1: FAIL（繋ぎが宣言どおりに出ていないか、終状態が宣言の並びへ収束しなかった）'
        $verdicts['J1'] = 'FAIL'
    }
}

# ---------------------------------------------------------------------------
# J2: グループ指定の無い走行では鎖の記録が 0 件（要件 6.1／6.2／6.4）
#
# 「0 件」は**両方の冠**について主張する。片方だけを数えると、もう一方が出ている走行を
# 「何も起きていない」と読める。冠で数えるので、その冠の下の語が全部（保全 2 語・鎖 7 語）
# 一度に入る——語を足したときに数え漏らす形にならない。
#
# 既定状態では望む鎖そのものを組まない（`compose_chain` がグループ 0 で `None` を返す）ので、
# 鎖系の記録は 1 行も出ない。1 行でも出ていれば「指定が無いのに束縛した」である。
# ---------------------------------------------------------------------------
if ($Mode -eq 'default') {
    Write-Output ''
    Write-Output '== J2（既定＝非強制・グループ系と鎖系の記録が 0 件・要件 6.1／6.2／6.4）=='
    $leaked = @($groupAll) + @($chainAll)
    if ($leaked.Count -eq 0) {
        Write-Output '  → J2: PASS（[zorder-group] も [zorder-chain] も 1 件も無い＝受理も繋ぎも拒否も起きていない）'
        $verdicts['J2'] = 'PASS'
    }
    else {
        Write-Output ("  → J2: FAIL（グループ指定が無いのに記録が出ている: [zorder-group] {0} 件 / [zorder-chain] {1} 件）" -f $groupAll.Count, $chainAll.Count)
        foreach ($l in $leaked | Select-Object -First 10) { Write-Output "    $l" }
        $verdicts['J2'] = 'FAIL'
    }
}

# ---------------------------------------------------------------------------
# J3: 既存のペア機構の記録が従来どおり出ること（要件 9.5）
#
# ⚠ 判定に origin=zorder-pair の件数を使ってはならない。判定は [zorder-pair] の
#    診断タグだけで行う。初版でこの禁が要ったのは、グループ発行の指令が凍結済み
#    pair_fix_command 経由で書かれ、書込側の origin 欄でペア発行分に見えたからである
#    （tasks.md 申し送り 4.1）。鎖の下では後押しが指令キューを経由しない（design DD-4）
#    ので取り違えの経路そのものは消えたが、**禁は据え置く**——書込側の欄はどちらにせよ
#    ペア機構の記録ではなく、そこから 9.5 の保全を言うことはできない。
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
#   値の妥当性は J1 の settled 行（宣言と実測の突合）が担う。
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
