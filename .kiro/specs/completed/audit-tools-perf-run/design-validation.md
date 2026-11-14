# 設計の検証: audit-tools-perf-run

- 日付: 2026-11-14
- 対象: `design.md`（749 行）・`requirements.md`・`research.md`（5 節の 36 件と 14 節の決め）・`spec.json`（`language: ja`・`phase: design-generated`）
- steering: この検証では `.kiro/steering/` を読み直していない（C20 の約束は、要件の Boundary Context・要件 6 と呼び手が示した触らないファイルの一覧で突き合わせた）
- 突き合わせたコード: `tools/perf/*.ps1` の 8 本・`tools/encoding-check.ps1`（読むだけ）・`.claude/skills/perf-loop-iteration/SKILL.md`（読むだけ）
- 回していないもの: 実際の計測のサブコマンド・xperf・`%LOCALAPPDATA%\areka-diag\perf-loop\` の下には触っていない

## 設計の要約の評価

36 件の指摘が全部「直し方・主なファイル・テストの名前・実走で確かめるか」の 1 行ずつに落ちており、範囲の外へ出る直しも、行と書く側と「申し合わせが決まらないときの形」まで書かれている。コードと照らした結果、関数の名前・今の行数・`SELFTEST_TOOLS` の数（今 8 本＋goal-text＝9 → 10 本＋goal-text＝11）・`encoding check` が `perf-loop.common.ps1` を単独で読み込んで `Invoke-Child`／`Invoke-PwshChild` を呼ぶ形は、設計の記述と合っている。直すべき所は 3 つあるが、どれもタスクの段で書き足せば済む大きさで、形の作り直しは要らない。

## 確かめた事実（コードとの突き合わせ）

| 確かめたこと | 結果 |
|---|---|
| 今の行数（番人 1,000 行） | `perf-loop.measure.ps1` 993・`invoke-perf-run.ps1` 926・`invoke-followup-checks.ps1` 865・`perf-loop.ps1` 685・`perf-loop.common.ps1` 678・`check-quiet.ps1` 662・`invoke-cpu-sample.ps1` 606・`perf-pwsh-child.ps1` 41。設計の見立ての前提と同じ。`measure` から `build`・`rundir` へ移る関数（`Get-MeasureBinDir`〜`Invoke-MeasureBuildSide` ほか）は約 200 行で、見立ての約 880 行は筋が通る |
| 設計が名指しする関数 | `Test-MeasureRunComplete`・`Clear-MeasureDryRunDir`・`Write-MeasureDryRunMark`・`Test-MeasureQuietAfterOk`・`Sync-Measure*`・`Get-NarSampleMap`・`Invoke-NativeCommand`・`Get-BackendFromGoalFile`・`Get-WpaExporterPath`・`Read-QuietRetrySettingsFromToml`・`Invoke-Utf8Child` の使い所・`Get-SamplingProbe`（今は入口に在る）は、どれも設計の書いた場所に在る |
| `encoding check` の形 | `tools/encoding-check.ps1` の 403 行で `perf-loop.common.ps1` を自分の範囲に読み込み、408 行で `Invoke-Child -Exe -Arguments -Quiet`、415 行で `Invoke-PwshChild -ScriptPath -Arguments -Quiet` を呼ぶ。設計は `common` を単独で読み込める形と 2 つの名前・引数を残し、足すのは任意の `-ReadEncoding` だけ＝崩れない。T2 `load-standalone` で毎回試す |
| 36 件のテストの置き場 | 1〜36 の全部に T1／T2／T3 の検査の名前がある。16 だけは「既にある T1 が緑・消す前の grep」で、消す直しなので妥当 |
| C20 の約束 | `tools/test-all.ps1`・`tools/encoding-check.ps1`・`Cargo.*`・`tools/crates-io.ps1`・`goals/`・`tools/perf/README.md`・`.claude/agents/perf-*.md` は Out of Boundary に入り、検証の段で既定のブランチとの差分 0 を確かめる形。範囲の外で書くのは `SKILL.md` の 2 か所だけ（要件の討議の裁定の範囲） |
| xperf の偽物が効くか | `invoke-cpu-sample.ps1` の `Get-XperfPath` は `PATH` を先に見て、既定の在り処はその後。子の `PATH` の先頭へ偽の `xperf.cmd` を置く T1 の形で本物は呼ばれない |

## 直すべき所（3 件）

### 1. 強制終了の後の「待つ」が areka 1 つだけで、退けが失敗したときの道が決まっていない

- **何が問題か**: ランナーは areka を `Start-Process -RedirectStandardOutput <run.log>` で起こし（`invoke-perf-run.ps1` 784〜788 行）、areka は同じフォルダの `shiori-host32-helper.exe` を子として起こす。設計の `Stop-ChildProcess` は `Kill()` した 1 つのプロセスが終わるのを待つだけで、ヘルパが areka から標準出力（＝`run.log`）を受け継いでいれば、areka が消えた後もヘルパが `run.log` を握ったまま残りうる。そのとき `-FAILED` への名前の替えは今と同じく失敗し、`perf-loop` の採り直しは「出力先が既に在る」で 3 になる。これは指摘 8 が挙げた害そのもので、設計は起きる見込みを下げるだけで、起きたときの道を決めていない。待ちが時間切れ（`$false`）のときの扱いも書かれていない。
- **なぜ困るか**: 応答の無い走行が 1 度出ると、採り直しの代わりに「引数の不正」で止まり、スキルは呼び方の誤りと読んで TOOLFIX へ回す。
- **直し方の案**: ⑴ `Stop-ChildProcess` は `Kill($true)`（子孫まで止める・.NET の標準）にしてから待つ。⑵ `Invoke-MeasureRun` は、ランナーが 0 以外で返り、`moved to:` の行が無く、出力先が元の名前のまま残っているとき、採り直しへ入らずに「失敗した走行を退けられなかった」の理由で 4 にする（または `Move-MeasureRunAside` に `FAILED` の組を足して perf-loop が退けてから採り直す）。⑶ T3 `move-after-kill` に「孫のプロセスがファイルを握る」場合を 1 つ足す。
- **要件**: 3.1（指摘 8）・5.2・5.3
- **設計の箇所**: 「直し方の一覧」の 8 の行・Components の perf-tool-base の `Stop-ChildProcess`・invoke-perf-run の節・perf-loop.measure の `Invoke-MeasureRun`

### 2. T2 が `perf-loop.ps1` を子として起こす検査は、`-DryRun` だけでは本物の python へ届きうる

- **何が問題か**: T2 の決まりは「`perf-loop.ps1` を子として起こすのは `-DryRun` と引数の確かめで終わる呼び方だけ」とする。しかし今のコードでは、`-DryRun` でも実行体が在れば先へ進む（`Test-MeasureExeReady` が `$true`）。`measure-baseline -DryRun` はランナーを `-DryRun` で回した後に `Invoke-MeasureJudge`（本物の `judge-perf.py`）を、`rank-run -DryRun` は `Invoke-MeasureRank`（本物の `perf-rank.py`）を呼ぶ。設計の `case-normalize` は `rank-run` には `-BinDir <空の一時>` を付けるが、`measure-baseline -Build Release -DryRun ...` は省略の形で書かれており、ワークツリーに `target\release\areka.exe` が在る机では python が走る。
- **なぜ困るか**: 結果がワークツリーにビルドが在るかどうかで変わる（要件 4.3 の決定論）。T2 の「`cargo`・`python`・`xperf` の本物を呼ばない」の決まりも破れる。
- **直し方の案**: T2 の決まりに「`perf-loop.ps1` を子として起こすときは、必ず `-BinDir <中身の無い一時のフォルダ>` と `-OutRoot <一時>` を渡す（実行体が無い道で終わらせる）」を 1 行足し、`case-normalize` の `measure-baseline` の例にも `-BinDir` を明記する。
- **要件**: 4.3・4.4・7.1
- **設計の箇所**: Testing Strategy の「T2 の検査」の **入口** の束・Components の perf-loop.selftest の決まり

### 3. README の 4 か所（R1〜R4）は相手が書く分担だが、この spec の検証でそろったかを確かめる段が無い

- **何が問題か**: 直しで偽になる README の文（R1＝`-Resume` の完了の見方と関数名・R2＝子の読み方・R3＝`-Cancel` とランナーが採取を始める形・R4＝`no_record`）は、`audit-tools-perf-judge` が書く分担になっている（README は相手の持ち物なので、分担そのものは要件 6.1・6.6 に合う）。ただ、検証（要件 8）は「この spec の側の README の差分が 0」を見るだけで、相手が書いたかどうかを見ない。この spec が先に `main` へ入ると、外への約束の説明の正本（要件の Introduction）が、相手が入るまで偽のまま残る。R2 には「決まらないときの形」があるが、R1・R3 には無い。
- **なぜ困るか**: README を読んで道具を呼ぶ人とエージェントが、無くなった関数名や古い採取の流れを正しいと読む。要件 3.2「壊れる文書を全部直し切る」が、記録の上で閉じない。
- **直し方の案**: 検証の段に 1 項足す: 「R1〜R4 が `main` に入っているか、まだなら共有の覚え書き `audit-tools-perf-run-to-perf-judge.md` に相手が引き受けた記録があり、完了の報告に『README の R1〜R3 は相手の PR 待ち』と書く」。あわせて R1・R3 にも、相手が書けないときの形（この spec の完了を相手の着地の後へ回す、など）を一言書く。
- **要件**: 3.2・6.6・8.6
- **設計の箇所**: 「範囲の外で直す文（申し合わせ用）」の R1〜R4・「検証（要件 8）」

## 設計のよい所

- **36 件の追い方が閉じている**: 指摘ごとに直し方・ファイル・テストの名前・実走の要否が 1 行にあり、範囲の外へ出る直しは行番号と書く側と相手の予定との重なりまで表になっている。申し合わせ待ち A・B には「決まらないときの形」があり、道具の直しが相手の返事で止まらない。
- **読み込みの向きと `encoding check` の形を先に固めている**: `perf-tool-base.ps1` を最下流に置き、`common` は単独で読み込めて副作用が無い、子の道具 4 本は `common` を読まない（`Stop-Run` の名前のぶつかりを避ける）、という決まりを図と文で示し、T2 `load-standalone` で毎回試す。採取の持ち主（始めるのと失敗の後の取り消しはランナー・成功の後に止めるのは perf-loop）も 1 か所に決まっている。

## 判定

- **判定: GO**
- **理由**: 要件・裁定・C20 の約束と食い違う所は無く、36 件すべてに直し方とテストの置き場がある。上の 3 件は、どれも決まりやテストの一文を足せば済む大きさで、設計の形（案 B の分け直し・採取の流れ・自分の検査の置き場）は変わらない。
- **次の段**: 設計の討議（`/kiro-design-discussion audit-tools-perf-run`）で上の 3 件を議題にし、決まった分を `design.md` に書き足してから `/kiro-spec-tasks audit-tools-perf-run` へ進む。
