# 実機の確かめ（real-run-log-noise）

タスク 5.2 の記録。直した後の areka を実機で 2 回走らせ、設計の「実機の数え直し」の表の文言ごとに行数を数えて、上限と比べた。

**結論**: 走行 A（同梱の emo2・`emo2-kakukaku`）も走行 B（どっとさくら・`bottle`）も `RESULT PASS`。毎フレーム出ていた件 1・件 2・件 3 の行は、装着のときの数件だけになった。`null.png` の `WARN` は 0 件。走行 B では本体（`actor=0`）と相方（`actor=1`）がどちらも話し、`bottle` の文字の面がどちらのスコープにも作られた。描画範囲の潰れの `warn` は 0 件（要件 6.4）。

## 1. 同定

| 項目 | 値 |
|---|---|
| コミット | `0e87e7b52a03f33b76e77803383af1ca3f958e9c`（2026-10-30 09:13:38 +0900・タスク 5.1 まで。走らせたときの作業ツリーは変更なし） |
| ビルド | debug 版。`cargo build -p areka`（`dev` の形）で作った `C:\home\maz\git\areka\.claude\worktrees\kiro-start-real-run-log-noise-58b428\target\debug\areka.exe` |
| 32 ビットの補助 | 走行の前に `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\kiro-start-real-run-log-noise-58b428\tools\sync-i686-helper.ps1` を回した。終了コード 0・出力は `helper is i686: target\debug\shiori-host32-helper.exe` |
| 検体 | `cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2` と `-- dot_sakura` が入れた根。`root=…\target\nar-samples\manual\emo2`・`…\manual\dot_sakura`。今の窓口の形では、バルーンはゴーストのフォルダの中に在る（`ghost\emo2\balloon\emo2-kakukaku`・`ghost\dot_sakura\balloon\bottle`） |
| 置き場 | どれもワークツリーの `target\real-run-log-noise-verify\` の下（`run.ps1`・`count.ps1`・`logs\`・`profile-emo2\runA1\`・`profile-dot-sakura\runB1\`）。git には残らない |
| 机 | 排他テストの机を、2 回を続けて走らせる間だけ借り、走り終えてすぐ返した |
| 操作 | 何もしていない（画面の操作・MCP の呼び出しなし）。終わりは自動終了（`AREKA_APP_SMOKE_EXIT_MS=50000`） |

## 2. 命令

### 起動の命令（`target\real-run-log-noise-verify\run.ps1`）

どっとさくらの確かめ（`completed/areka-P0-dot-sakura-specimen/verification.md` 4 節）と同じ形にした。`$wt` と置き場は絶対パスだけで組み、同じ呼び出しの中で終わりを待って終了コードを書き出す。環境変数は設計のとおり。違いは 1 点で、ゴーストのフォルダとバルーンのフォルダを位置の引数でも渡す（`real-run-kit` の `Launch::Sample` と同じ渡し方）。今の窓口ではバルーンがゴーストのフォルダの中に在るので、バルーンを確実に選ぶためにそうした。記録では両方の走行とも `ghost_resolved route=Argv`・`balloon_resolved route=Argv` で、ねらったゴーストとバルーンになった。

```powershell
param(
    [Parameter(Mandatory)][ValidateSet('A', 'B')][string]$Run,
    [Parameter(Mandatory)][string]$Tag
)
$ErrorActionPreference = 'Stop'
$wt = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-real-run-log-noise-58b428'
$v = "$wt\target\real-run-log-noise-verify"
if ($Run -eq 'A') {
    $root = "$wt\target\nar-samples\manual\emo2"
    $ghost = "$root\ghost\emo2"
    $balloon = "$ghost\balloon\emo2-kakukaku"
    $profile = "$v\profile-emo2\$Tag"
} else {
    $root = "$wt\target\nar-samples\manual\dot_sakura"
    $ghost = "$root\ghost\dot_sakura"
    $balloon = "$ghost\balloon\bottle"
    $profile = "$v\profile-dot-sakura\$Tag"
}
if (Test-Path $profile) { throw "profile already exists: $profile" }
New-Item -ItemType Directory -Force $profile, "$v\logs" | Out-Null
Remove-Item Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
$env:AREKA_ROOT = $root
$env:AREKA_PROFILE_DIR = $profile
$env:RUST_LOG = 'info,areka_seriko=debug,areka_emo_text=debug'
$env:AREKA_APP_SMOKE_EXIT_MS = '50000'
$env:NO_COLOR = '1'
$env:AREKA_NO_ALERT = '1'
$p = Start-Process -FilePath "$wt\target\debug\areka.exe" -ArgumentList $ghost, $balloon -WorkingDirectory $v -RedirectStandardOutput "$v\logs\$Tag.stdout.log" -RedirectStandardError "$v\logs\$Tag.stderr.log" -PassThru
$start = Get-Date
"pid=$($p.Id) start=$($start.ToString('yyyy-MM-ddTHH:mm:sszzz'))" | Set-Content "$v\logs\$Tag.pid.txt"
$p.WaitForExit()
$end = Get-Date
"$($p.ExitCode)" | Set-Content "$v\logs\$Tag.exitcode.txt"
"exit=$($p.ExitCode) end=$($end.ToString('yyyy-MM-ddTHH:mm:sszzz')) secs=$([int]($end - $start).TotalSeconds)" | Add-Content "$v\logs\$Tag.pid.txt"
"exit=$($p.ExitCode) secs=$([int]($end - $start).TotalSeconds)"
& "$v\count.ps1" -Log "$v\logs\$Tag.stdout.log" -Run $Run
```

走らせた命令（この順に 1 回ずつ）:

```text
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\kiro-start-real-run-log-noise-58b428\target\real-run-log-noise-verify\run.ps1" -Run A -Tag runA1
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\kiro-start-real-run-log-noise-58b428\target\real-run-log-noise-verify\run.ps1" -Run B -Tag runB1
```

最初の 1 回は、走行 A の命令のパスを引用符で囲まずに Git Bash から渡した。逆斜線が消えて `pwsh` が台本のファイルを見つけられず、areka を起こす前に止まった（`logs\` に何も残っていない）。引用符で囲んで打ち直したのが上の 2 行。areka を起こしたのは、走行ごとに 1 回だけ。

### 数える命令（`target\real-run-log-noise-verify\count.ps1`）

`run.ps1` が走行の後に呼ぶ。端末へは ASCII だけを出す。1 文言 1 行で `name count cap PASS|FAIL`、最後に `RESULT PASS|FAIL`。走行 B で話した記録が無ければ `RESULT NOVERDICT`。文言はソースから逐語で写した（要件 1.7 の 5 つは `writing.rs`・`wrap.rs`・`balloon_overrides.rs` の `SKIPPED`・`draw.rs`）。

```powershell
param(
    [Parameter(Mandatory)][string]$Log,
    [Parameter(Mandatory)][ValidateSet('A', 'B')][string]$Run
)
$ErrorActionPreference = 'Stop'
$lines = [System.IO.File]::ReadAllLines($Log, [System.Text.Encoding]::UTF8)
$levelRe = [regex]'^\S+\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s'
function Level([string]$l) { $m = $levelRe.Match($l); if ($m.Success) { $m.Groups[1].Value } else { '' } }
# Fields printed after the message (span prefixes like actor{actor=emo-text} come before it).
function After([string]$l, [string]$msg) { $i = $l.IndexOf($msg, [System.StringComparison]::Ordinal); if ($i -lt 0) { '' } else { $l.Substring($i + $msg.Length) } }
function Hits([string]$msg, [string[]]$levels) {
    @($lines | Where-Object { $_.Contains($msg) -and ($levels -contains (Level $_)) })
}
$any = 'DEBUG', 'INFO', 'WARN', 'ERROR'

# Messages copied verbatim from the source (crates/areka-emo-text, crates/areka-emo-atlas).
$M_RB = 'actor の装着先（予約スロット）を登録した'
$M_RX = '箱の置き場所を登録した（面は次の提示で作る）'
$M_G = '箱の束を受け取った（前の箱の面を片付け、表を差し替える）'
$M_ITEM1 = '未指定の origin 成分を書字開始角へ寄せる'
$M_ITEM2 = '同値のため再結線・再生成を行わない'
$M_ROP = 'cursor.blendmethod ROP 系は M1 未対応'
$M_UL = 'cursor.style underline 系は M1 未対応'
$M_EMPTY = '全透明（α=0）でトリム後 0 寸'
$M_EDGE = '未指定座標成分を既定辺へ縮退する'
$M_OUT = '宣言された origin 成分が文字を描いてよい範囲の外にある'
$M_VERT = 'vertical の値が 0／1 のいずれでもないため指定なしとして扱う（受理値: 0 / 1）'
$M_WM = '未知の writing_mode 値のため指定なしとして扱う（受理語彙: horizontal_tb / vertical_rl / vertical_lr）'
$M_WRAP = '未知の budoux_newline 値のため OFF（文字単位折返し）へフォールバックする（受理語彙: 1 / true / 0 / false）'
$M_OVR = 'バルーン定義の書体設定を適用できない——当該項目は既定のまま'
$M_FH = 'font.height が 0（DirectWrite fontsize は正値必須）のため既定値 12 へフォールバックする'
$M_COL = '文字の描画範囲が潰れている（幅か高さが 0 以下）'
$M_SURF = 'テキスト供給面を予約スロットへ装着した'
$M_BOOT = '起動グリーティングを再生起動'
$M_TEXT = 'Text cue 適用'

$rb = @(Hits $M_RB @("DEBUG")); $rx = @(Hits $M_RX @("DEBUG")); $g = @(Hits $M_G @("DEBUG"))
$R = $rb.Count + $rx.Count
$S = @($rb | ForEach-Object { if ((After $_ $M_RB) -match '\bactor=(\S+)') { $Matches[1] } } | Sort-Object -Unique).Count
$G = $g.Count
$N = 0; foreach ($l in $g) { if ((After $l $M_G) -match '\bboxes=(\d+)') { $N = [Math]::Max($N, [int]$Matches[1]) } }
"lines $($lines.Count) R_b $($rb.Count) R_x $($rx.Count) R $R S $S G $G N $N"

$fail = $false
function Check([string]$name, [int]$count, [int]$cap) {
    $ok = $count -le $cap
    if (-not $ok) { $script:fail = $true }
    "$name $count $cap $(if ($ok) { 'PASS' } else { 'FAIL' })"
}
$empty = @(Hits $M_EMPTY @("WARN"))
$nullRe = 'purple[\\/]+a[\\/]+null\.png'
$nullPng = @($empty | Where-Object { $_ -match $nullRe })
$other = @($empty | Where-Object { $_ -notmatch $nullRe })
$otherGroups = @($other | Group-Object { if ((After $_ $M_EMPTY) -match 'set=(\S+) rel_path=(\S+)') { "$($Matches[1])|$($Matches[2])" } else { $_ } })

Check 'resolves' $R 50
Check 'item1-origin-corner' (Hits $M_ITEM1 $any).Count (2 * $R)
Check 'item2-churn-quiet' (Hits $M_ITEM2 $any).Count 0
Check 'item3-rop' (Hits $M_ROP @('WARN')).Count ($S + $N * $G)
Check 'item3-underline' (Hits $M_UL @('WARN')).Count ($S + $N * $G)
Check 'item4-null-png' $nullPng.Count 0
$maxPer = ($otherGroups | ForEach-Object Count | Measure-Object -Maximum).Maximum
$o = Check 'item4-other' $other.Count (2 * $otherGroups.Count)
if ($maxPer -gt 2) { $script:fail = $true; $o = $o -replace 'PASS$', 'FAIL' }
$o
Check 'r15-default-edge' (Hits $M_EDGE @('DEBUG')).Count (5 * $R)
Check 'r15-origin-outside' (Hits $M_OUT @('DEBUG')).Count (2 * $R)
Check 'r17-vertical' (Hits $M_VERT @('WARN')).Count $R
Check 'r17-writing-mode' (Hits $M_WM @('WARN')).Count $R
Check 'r17-wrap' (Hits $M_WRAP @('WARN')).Count $R
Check 'r17-overrides' (Hits $M_OVR @('WARN')).Count (2 * $R)
Check 'r17-font-height' (Hits $M_FH @('WARN')).Count ($R + $N * $G)
Check 'r6-collapse' (Hits $M_COL @('WARN')).Count 0

# r6-surface: normal balloon place only (place=Balloon), per scope.
$surf = @(Hits $M_SURF @('INFO') | Where-Object { (After $_ $M_SURF) -match '\bplace=Balloon\b' -or $_ -match 'place=Balloon\b' })
$surfBy = @{}; foreach ($l in $surf) { if ($l -match 'actor=(\S+) place=Balloon') { $surfBy[$Matches[1]] = 1 + [int]$surfBy[$Matches[1]] } }
$asc = { param($s) ($s -replace '[^\x20-\x7E]', '?') }
$boot = (Hits $M_BOOT @('INFO')).Count
$spoken = @(@(Hits $M_TEXT @("DEBUG")) | ForEach-Object { if ((After $_ $M_TEXT) -match '\bactor=(\S+)') { $Matches[1] } } | Sort-Object -Unique)
"boot_greeting $boot spoken_scopes $(& $asc ($spoken -join ','))"
$noVerdict = $false
if ($Run -eq 'B') {
    if ($boot -eq 0 -or $spoken.Count -eq 0) {
        $noVerdict = $true
        'r6-surface NOSPEECH >=1 NOVERDICT'
    } else {
        foreach ($a in $spoken) {
            $c = [int]$surfBy[$a]
            $ok = $c -ge 1
            if (-not $ok) { $fail = $true }
            "r6-surface[actor=$(& $asc $a)] $c >=1 $(if ($ok) { 'PASS' } else { 'FAIL' })"
        }
    }
} else {
    foreach ($a in $surfBy.Keys) { "r6-surface[actor=$(& $asc $a)] $($surfBy[$a]) info -" }
}
if ($noVerdict) { 'RESULT NOVERDICT' } elseif ($fail) { 'RESULT FAIL' } else { 'RESULT PASS' }
```

- 行の水準は、行頭の時刻の次の語（`DEBUG`・`INFO`・`WARN` など）で読む。欄（`actor=`・`boxes=`・`place=`）は文言より後ろから読む。行頭のスパン `actor{actor=emo-text}` を拾わないため。
- 数えない: 折返し基準の `WARN`（討議 5）と、終わり際の `kanade: 強制終了指示`。どの名前の文言にも当たらないので、どの行数にも入らない。
- 数える命令の確かめ: 走らせる前に、直す前のどっとさくらの記録（`areka-p0-dot-sakura-specimen-0f24e9\target\dot-sakura-verify\logs\run1.stdout.log`）を読ませた。`item3-rop 20764 0 FAIL`・`r6-collapse 2 0 FAIL`・`RESULT NOVERDICT`（話していない）を出し、その記録の 7 節の数と一致した。

## 3. 上限の入力（R_b・R_x・R・S・G・N）

設計の決め方のまま、記録の中の数から求めた。走行の長さ（フレーム数）は使わない（要件 4.3）。

| 入力 | 求め方 | 走行 A | 走行 B |
|---|---|---|---|
| `R_b` | `DEBUG` の「actor の装着先（予約スロット）を登録した」の行数 | 2（`actor=0`・`actor=1`） | 2（`actor=0`・`actor=1`） |
| `R_x` | `DEBUG` の「箱の置き場所を登録した（面は次の提示で作る）」の行数 | 0 | 0 |
| `R` | `R_b` ＋ `R_x`（上限 50） | 2 | 2 |
| `S` | `R_b` の行の `actor=` の欄の種類の数 | 2 | 2 |
| `G` | `DEBUG` の「箱の束を受け取った（前の箱の面を片付け、表を差し替える）」の行数 | 1 | 1 |
| `N` | `G` の行の `boxes=` の欄の最大値（箱の名前の数） | 0 | 0 |

この値から上限は、件 1 と `r15-origin-outside` が 2×`R`＝4、件 3 の 2 つが `S`＋`N`×`G`＝2、`r15-default-edge` が 5×`R`＝10、`r17-*` が `R`＝2（`r17-overrides` は 2×`R`＝4、`r17-font-height` は `R`＋`N`×`G`＝2）。`item4-other` は絵ごとに 2（`set` と `rel_path` で分け、どの絵も 2 以下で、合計が 2×絵の数以下）。

## 4. 走行

| 項目 | 走行 A（同梱の emo2） | 走行 B（どっとさくら） |
|---|---|---|
| 記録の名前 | `runA1` | `runB1` |
| バルーン | `emo2-kakukaku` | `bottle` |
| 時刻（日本時間） | 2026-10-30 09:33:45〜09:34:42 | 2026-10-30 09:34:53〜09:35:46 |
| 長さ | 56 秒（自動終了 50 秒＋起動と終了） | 53 秒 |
| areka の番号 | 30388 | 20696 |
| 終了コード | 0（`logs\runA1.exitcode.txt`） | 0（`logs\runB1.exitcode.txt`） |
| 記録の全行数 | 6,187（ERROR 0・WARN 2・INFO 156・DEBUG 6,029） | 1,345（ERROR 0・WARN 47・INFO 440・DEBUG 858） |
| 標準エラー | 1 行（`[helper] SHIORI 初期化の入口: load`） | 1 行（同じ） |
| `panicked` | 0 | 0 |
| 話したか | 話した。`起動グリーティングを再生起動`（09:33:52 日本時間）・`Text cue 適用` 48 行・スコープ 0 と 1 | 話した。`起動グリーティングを再生起動`（09:34:57 日本時間）・`Text cue 適用` 82 行・スコープ 0 と 1 |
| 端末の結果 | `RESULT PASS` | `RESULT PASS` |

## 5. 文言ごとの行数・上限・合否（直す前と並べて）

直す前の数の出どころ:

- **2026-10-10**（brief.md の Current State・コミット `84ca70e6`・同じ `RUST_LOG=info,areka_seriko=debug,areka_emo_text=debug`）。同梱の emo2 のままの走行は約 46 秒・全 42,845 行で、件 1 と件 2 の合計は 32,008（内訳は記録に無い）・`null.png` の `WARN` は 2。検体つきの走行は約 51 秒・全 32,194 行で、件 1 が 20,484・件 2 が 10,240。
- **2026-10-23**（`completed/areka-P0-dot-sakura-specimen/verification.md`。走行は 2026-10-22 23:36:52〜23:39:06 日本時間・2 分 14 秒・全 21,198 行）。`RUST_LOG` は `info,areka::boot_resolve=debug,…` で、文字の層の `debug` を開けていない。そのため `debug` の行（件 1・件 2・`R`・`r15-*`）は「測っていない」とした。`WARN` と `INFO` の行はこの記録から数え直した。`r6-surface` は、同じ記録で「テキスト供給面を予約スロットへ装着した」が 0 行（`INFO` は開いていた）。

### 走行 A（同梱の emo2・`emo2-kakukaku`）

| 名前 | 直す前 2026-10-10（emo2 のまま・46 秒） | 直す前 2026-10-10（検体つき・51 秒） | 直した後（`runA1`・56 秒） | 上限 | 合否 |
|---|---|---|---|---|---|
| 記録の全行数 | 42,845 | 32,194 | 6,187 | — | — |
| `resolves`（`R`） | 測っていない | 測っていない | 2 | 50 | PASS |
| `item1-origin-corner`（件 1） | 件 2 と合わせて 32,008 | 20,484 | 4 | 4 | PASS |
| `item2-churn-quiet`（件 2） | （件 1 に合算） | 10,240 | 0 | 0 | PASS |
| `item3-rop`（件 3） | 測っていない | 測っていない | 0 | 2 | PASS |
| `item3-underline`（件 3） | 測っていない | 測っていない | 0 | 2 | PASS |
| `item4-null-png`（件 4） | 2 | 2 | 0 | 0 | PASS |
| `item4-other` | 測っていない | 測っていない | 0 | 0 | PASS |
| `r15-default-edge` | 測っていない | 測っていない | 0 | 10 | PASS |
| `r15-origin-outside` | 測っていない | 測っていない | 0 | 4 | PASS |
| `r17-vertical` | 測っていない | 測っていない | 0 | 2 | PASS |
| `r17-writing-mode` | 測っていない | 測っていない | 0 | 2 | PASS |
| `r17-wrap` | 測っていない | 測っていない | 0 | 2 | PASS |
| `r17-overrides` | 測っていない | 測っていない | 0 | 4 | PASS |
| `r17-font-height` | 測っていない | 測っていない | 0 | 2 | PASS |
| `r6-collapse` | 測っていない | 測っていない | 0 | 0 | PASS |
| `r6-surface`（合否に入れない） | 測っていない | 測っていない | `actor=0` 1・`actor=1` 1 | — | — |

- 件 1 の 4 行は、装着のときの 2 スコープ × 成分 2 つ（`origin.x`・`origin.y`）。その後の約 50 秒には 1 行も出ていない。
- `item4-null-png` が 0 になったのは、着せ替えの「無し」の絵と見分けて `debug` へ下げたため（要件 3.1）。この `debug` の行そのものは、この走行の記録に無い。`RUST_LOG` が `areka_emo_atlas` の `debug` を開けていないからで、見分けた `debug` の 1 件は決定論のテスト（`emo2_e2e.rs`・タスク 3.3）で固定している。
- 数えなかった `WARN` は 2 行。折返し基準の `WARN`（`balloon="emo2-kakukaku"`・討議 5）と、終わり際の `kanade: 強制終了指示`。

### 走行 B（どっとさくら・`bottle`）

| 名前 | 直す前 2026-10-23（2 分 14 秒） | 直した後（`runB1`・53 秒） | 上限 | 合否 |
|---|---|---|---|---|
| 記録の全行数 | 21,198 | 1,345 | — | — |
| `resolves`（`R`） | 測っていない | 2 | 50 | PASS |
| `item1-origin-corner`（件 1） | 測っていない | 0 | 4 | PASS |
| `item2-churn-quiet`（件 2） | 測っていない | 0 | 0 | PASS |
| `item3-rop`（件 3） | 20,764 | 2 | 2 | PASS（上限ちょうど） |
| `item3-underline`（件 3） | 0 | 0 | 2 | PASS |
| `item4-null-png`（件 4） | 0 | 0 | 0 | PASS |
| `item4-other` | 4 | 4 | 4 | PASS（上限ちょうど） |
| `r15-default-edge` | 測っていない | 0 | 10 | PASS |
| `r15-origin-outside` | 測っていない | 0 | 4 | PASS |
| `r17-vertical` | 0 | 0 | 2 | PASS |
| `r17-writing-mode` | 0 | 0 | 2 | PASS |
| `r17-wrap` | 0 | 0 | 2 | PASS |
| `r17-overrides` | 0 | 0 | 4 | PASS |
| `r17-font-height` | 0 | 0 | 2 | PASS |
| `r6-collapse` | 2 | 0 | 0 | PASS |
| `r6-surface`（`actor=0`・下限） | 0（話していない） | 1 | 1 以上 | PASS |
| `r6-surface`（`actor=1`・下限） | 0（話していない） | 1 | 1 以上 | PASS |

- **上限ちょうどの 2 つ**:
  - `item3-rop` の 2 行は、装着のときにスコープごとに 1 行（`balloon="bottle" blendmethod="mergepennot"`）。上限 `S`＋`N`×`G`＝2＋0 のちょうどで、余裕は無い。設計の「置き場ごとに値が決まったとき 1 件」のとおり。
  - `item4-other` の 4 行は、`surface12.png` と `surface22.png`（どちらも `set=0`・228×338）がそれぞれ 2 行。1 回の起動で焼き込みが 2 回走るため（設計）。絵ごとの上限 2 のちょうどで、作者の中身から出るものなので外さず数を見た。
- `r6-surface` の対象は話したスコープ 0 と 1（`Text cue 適用` の `actor` の欄）。どちらも `place=Balloon` の行が 1 行ずつ出た（`actor=0`: `physical_size=(652, 298)`、`actor=1`: `physical_size=(652, 164)`・`wrap=CharByChar`）。潰れた場所には供給面を作らないので、`bottle` の文字が両方のスコープで描かれた（要件 6.4）。直す前は 2 つのスコープとも潰れていた（`r6-collapse` 2）。
- 直す前の走行は SHIORI が全部の要求に 400 を返して話さなかった。直した後は `shiori2-protocol`（#373）の後のコードで、起動のグリーティングが再生された。
- 数えなかった `WARN` は 41 行で、どれも本 spec の文言ではない。合成メソッドの名前 27・シェルの描けない描画メソッド 6・2 人目のバルーンの面の縮退 6・`OnTranslate` のエラー応答 1・終わり際の `kanade: 強制終了指示` 1。数えた `WARN` 6 行（`item3-rop` 2・`item4-other` 4）と合わせて 47。

## 6. 記録の置き場

- `C:\home\maz\git\areka\.claude\worktrees\kiro-start-real-run-log-noise-58b428\target\real-run-log-noise-verify\logs\runA1.stdout.log`・`runA1.stderr.log`・`runA1.pid.txt`・`runA1.exitcode.txt`
- 同じフォルダの `runB1.*`
- 命令は `run.ps1`・`count.ps1`（同じ `target\real-run-log-noise-verify\` の直下）。使い捨てで、リポジトリには残さない（要件 4.6）。記録の量が戻ったことの見張りは、要件 5.1 の決定論のテストが担う。
