# ギャップ分析: audit-tools-perf-run

> 2026-11-14 `kiro-validate-gap`（`/kiro-start` の中のサブエージェント）。範囲 `tools/perf/*.ps1`・`*.psm1` だけに `/ponytail:ponytail-audit` の作法をかけた。作法のうち 2 つは brief のとおり外した＝「最大 20 件」を外して全件を出す・「読まなかった所（Not checked）」を残さない（8 ファイル 5,456 行を全部、頭から終わりまで読んだ）。番号は audit の通し番号で、以後の段はこの番号で指す。

## 1. 要約

- 範囲は性能改善ループを走らせる PowerShell の道具 8 本（5,456 行・テスト 0 行）。指摘は **36 件**＝必ず直す 6・直したほうがよい 18・できれば 12。
- 重いのは「走行を途中で止めたときの後始末」と「`-Resume` の完了の見方」。段③のカーネルの採取（xperf）が止まらずに残る道（1）と、途中で切れた走行や走行の後の静かさの確認が済んでいない走行を `-Resume` が「済み」として使う道（2）がある。どちらも、スキル `perf-loop-iteration` が背景の走行を見失ったときに `-Resume` で起動し直す手順と重なる。
- 外への約束（終了コード・RESULT 行・`key=value` の語・`Invoke-PwshChild` ほか）を変える直しは 14 件、変えずに守るよう気を付ける直しは 3 件。実際の計測の走行でしか確かめられないものは 4 件、一部だけそうなものが 4 件。
- テストの置き場が今は無い（`perf-loop.ps1 selftest` は Python の 5 本と PowerShell の 3 本の自分の検査を回すが、`perf-loop.ps1` 自身と `invoke-perf-run.ps1` の判断の分かれ目は 1 つも試していない。全体テストは `tools/perf` の自分の検査を回していない）。どこで回すかは要件の討議で決める（要件 4.2）。
- 直す対象のタスクは 18〜22 本の見立てで、上限 20 の境目にある。分け方の案を 10 節に置いた（決めるのは要件の討議）。

## 2. 測り直し（要件 1.2）

- 取り込んだ `main`: `30a343eb`（このブランチの `HEAD~1`・`origin/main` と同じ）。`tools/perf/` の差分は無い。
- 範囲: `tools/perf/` 直下の `*.ps1` 8 ファイル・`*.psm1` 0 ファイル。本番のコード 5,456 行・テスト 0 行（brief と同じ数え方＝`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた。各道具の中にある `-SelfTest` の部分はコードに数えた）。

| ファイル | 行数 |
|---|---:|
| `check-quiet.ps1` | 662 |
| `invoke-cpu-sample.ps1` | 606 |
| `invoke-followup-checks.ps1` | 865 |
| `invoke-perf-run.ps1` | 926 |
| `perf-loop.common.ps1` | 678 |
| `perf-loop.measure.ps1` | 993 |
| `perf-loop.ps1` | 685 |
| `perf-pwsh-child.ps1` | 41 |
| 計 | 5,456 |

- 物差しの版: brief は ponytail 5.3.2 と書くが、今入っているのは 5.3.3（`~/.claude/plugins/installed_plugins.json`・2026-11-06 更新）。audit の作法（見る順・3 つの組・1 件 4 部・`shortcut:` の扱い）は 5.3.3 の `skills/ponytail-audit/SKILL.md` に従った。
- 範囲の中の `shortcut:`・`ponytail:` の注釈: 0 件（grep で確かめた）。

## 3. この道具が何をするか・想定した負荷

**何をするか**: 開発者 1 人の Windows の机で、areka を有界の自動終了つきで起動して CPU の時系列を採り（`invoke-perf-run.ps1`）、前後の静かさを確かめ（`check-quiet.ps1`）、管理者権限があれば xperf で関数ごとの採取をし（`invoke-cpu-sample.ps1`）、見た目の追随を実際の入力で確かめる（`invoke-followup-checks.ps1`）。入口は `perf-loop.ps1` の 10 本のサブコマンドで、スキル `perf-loop-iteration` と役割別のエージェントが背景の 1 本ずつとして呼ぶ。判定と台帳は Python の道具（`audit-tools-perf-judge` の持ち物）。

**想定した負荷**: 1 台の静かな机で、1 度に 1 本の走行（7 分か 25 分）。並行の走行は無い。1 ゴールで周は最大 30（`[stop] max_iterations`）。ファイルの大きさは、run.log が数十 MB まで・段③の採取と dump は機械の CPU 数と採取の時間で決まり、実物の大きさはこの分析では測っていない（走行を回さない約束＝要件 7.1）。

## 4. 範囲の外の読み手と外への約束（いま確かめた事実）

| 外への約束 | 読み手 | 確かめた所 |
|---|---|---|
| サブコマンド名・引数・末尾の `PERF-LOOP RESULT <sub> code=<n> dir=<path>` | スキル `perf-loop-iteration`（`SKILL.md` 25〜339 行・走行を見失ったら同じコマンドに `-Resume` を付けて起動し直す＝331〜339 行）・`.claude/agents/perf-measure.md`（36・56・70 行） | grep |
| `preflight` の `capabilities=…` 行（そのまま台帳へ写す） | スキル（80〜82 行）・台帳の検体 | grep |
| `perf-loop.common.ps1` を単独で読み込み、`Invoke-Child`・`Invoke-PwshChild` を呼ぶ | `tools/encoding-check.ps1` 399〜425 行（932 の端末で 3 行が字のとおりに届くか・端末が 932 のままか） | 読んだ |
| `Get-PwshChildArguments`（`perf-pwsh-child.ps1`） | 範囲の中だけ（`invoke-perf-run.ps1`・`perf-loop.common.ps1`） | grep |
| `CHECK_ALL`（追随の 4 検査の語） | `invoke-followup-checks.ps1:122`・`perf-loop.measure.ps1:84`・`judge-followup.py` | 読んだ |
| `SCRIPT_VERSION` | `invoke-perf-run.ps1` は `run-meta.txt` へ書く・`perf-loop.common.ps1:467` は `judge-perf.py` の版を読む | 読んだ |
| `judge-perf.py --build` は `dev`・`release` だけ（大文字小文字を区別） | `judge-perf.py:4082`・`perf-compare.py:929` | grep |
| 範囲の外の Python の道具は、`run-meta.txt` の `ended_utc` など終わりの節を読まない。長い走行だけは `LONG_RUN_MIN_SPAN_SEC = 1200` で途中切れを弾く | `judge-perf.py:459` | grep |
| 32bit の SHIORI ヘルパは、areka が閉じると一緒に落ちる（ジョブに入れている） | `crates/shiori-host32-host/src/job.rs` | grep |

## 5. 指摘の一覧（36 件）

旗の読み方:
- **(a) 外への約束**: 直すと、範囲の外の読み手（`audit-tools-perf-judge` の Python・スキル・エージェント・`README.md`・`goals/`・`encoding check`）が前提にしている形が変わるか。「変える」＝形が変わる・「守る」＝形は変えないが、そこを壊さないよう直し方に気を付ける・「なし」。
- **(b) 実走でしか**: 実際の計測の走行（areka を立てる・画面を操作する・昇格して xperf を回す）でしか確かめられないか。「はい」「一部」「いいえ」。
- **(c) 残す候補**: brief の「直さずに残してよい 3 種類」に当たりうるか（⑴ 開発者の裁定・完了した spec の要件や設計に反する／⑵ わざと置いた先送りの印／⑶ SSP・ukadoc の互換）。候補を挙げるだけで、決めない。
- **(d) テストの置き場**: 壊れたら落ちる決定論のテストを置ける場所。T1＝既にある `-SelfTest`（`check-quiet`・`invoke-cpu-sample`・`invoke-followup-checks`）へ足す／T2＝`perf-loop.ps1` 側の新しい自分の検査（新しい `.ps1`・名前は設計で決める・`SELFTEST_TOOLS` に足す）／T3＝`invoke-perf-run.ps1` に新しく `-SelfTest` を持たせる。

### 必ず直す（Must fix）

**1. 段③のカーネルの採取が止まらずに残る** （`perf-loop.measure.ps1:632-673`・`invoke-cpu-sample.ps1:428-450`・`:454-462`）
- **何か**: `rank-run` は採取を `-Start` で始めてから走行を回し、最後に `-Stop` で止める。
- **困る場面**: 採取を始めた後で走行が止まると（静かでない＝2・引数の不正＝3・2 度失敗＝4・走行の前の `cargo build` の失敗・例外）、`Stop-Run` がそのまま `exit` して `-Stop` が呼ばれない。`-Stop` の側も、`-PdbDir` が無い・`LOCALAPPDATA` が無い・`.wpaProfile` が無い（`wpaexporter` を選んだとき）と、採取を止める前に 3 で終わる。すると xperf のカーネルの採取（バッファ 512 MB まで）が再起動まで動き続け、次の `rank-run` の `-Start` は「既に動いている」で失敗し（`start_failed`）、段③が毎回抜け、机は重いままになる（後の静かさの確認も落ちやすくなる）。
- **直し方**: `rank-run` の採取の開始から停止までを `try`/`finally` にして、どの道でも止める（`-Stop` が無理なら `xperf -stop`）。`-Stop` の中の前提の確かめは、止めた後へ回す。
- **直さないと**: 1 度でも途中で止まると、昇格した机で段③が以後ずっと取れず、計測の机も汚れたまま。
- **旗**: (a) なし ／ (b) 一部（止める道の筋は偽の `xperf`〔小さな `.cmd`〕で決定論に試せる。本物の xperf で確かめるのは昇格した机だけ） ／ (c) なし ／ (d) T2（偽の `xperf` と偽の走行で「途中で止まっても `-stop` が呼ばれた」を見る）＋T1（`-Stop` の前提の確かめの順）

**2. `-Resume` が、途中で切れた走行と、後の静かさの確認が済んでいない走行を「済み」として使う** （`perf-loop.measure.ps1:189-195`・`perf-loop.common.ps1:667-672`・`invoke-perf-run.ps1:735`・`:769`・`:784-789`）
- **何か**: `-Resume` の完了の印は「`run.log`・`cpu.csv`・`run-meta.txt` が在る」こと＋「`quiet-after.txt` が無いか QUIET」。
- **困る場面**: `run-meta.txt` と `cpu.csv` は走行の起動の前に、`run.log` は起動のときに作られる。背景の走行が途中で消えると（セッションが切れた・背景のタスクが止められた）、3 つとも在るまま残る。スキルは見失った走行を同じコマンドに `-Resume` を付けて起動し直す（`SKILL.md:331-339`）ので、7 分の走行の 3 分ぶんだけの A1 がそのまま比べに使われる（25 分の走行は `judge-perf.py` が 1200 秒で弾くが、7 分の走行には弾く所が無い）。また `quiet-after.txt` は確認の最後にしか書かれない（`check-quiet.ps1:658`）ので、後の確認の途中で消えた走行は「後ろの確認を持たない」と読まれて通る。注釈は「-DryRun 等」と書くが、`-DryRun` の出力先には `run.log` が無いので、この分かれ目には来ない。
- **直し方**: 完了の印を、ランナーが最後にだけ書く `run-meta.txt` の終わりの節（`ended_utc`）の有無に替え、`quiet-after.txt` が無い本物の走行は未完了と読む。未完了の出力先は、今の NOT_QUIET と同じく名前を替えて脇へ退ける（測ったものは消さない）。
- **直さないと**: 短い走行の切れ端で採否が決まり、台帳に誤った採用・不採用が残る。
- **旗**: (a) 変える（軽い：退ける出力先の名前が 1 つ増える。スキルの文「完了済みの成果物は再利用され、無ければ走る」はそのまま正しい） ／ (b) いいえ ／ (c) なし ／ (d) T2（`ended_utc` の無い偽の出力先・`quiet-after.txt` の無い偽の出力先を作って「未完了」と読むこと）

**3. 想定していない失敗で落ちると、RESULT 行が出ず、終了コードが 1 になる** （`perf-loop.ps1` 全体〔`trap` が無い〕・例 `perf-loop.ps1:523`・`perf-loop.measure.ps1:514`・`:536`・`perf-loop.common.ps1:675`・`invoke-followup-checks.ps1` 全体・例 `:608`・`invoke-cpu-sample.ps1:71-118`・`:605`・`check-quiet.ps1` 全体）
- **何か**: 各道具は「標準出力の末尾に必ず RESULT 行を出す」と約束し、`perf-loop.ps1` は「終わるのは必ず `Exit-WithResult` を通る」と書く。どれも `$ErrorActionPreference = 'Stop'` だが、捕まえる所が無い。
- **困る場面**: `python` が無いのに台帳が在る机で `preflight` を回すと、`Invoke-PythonChild -Python $null` の引数の結び付けで例外になり（`:523`）、RESULT 行なしの 1 で落ちる。`bin-B\areka.exe` を前の走行の areka がまだ握っているときの `Copy-Item`（`:514`）、脇へ退ける `Move-Item`（`common:675`）も同じ。1 は表では「実走の失敗」なので、スキルとエージェントは道具の不具合（4）と読めない。`invoke-followup-checks.ps1` は窓が先に閉じると `$rect` が空のまま `.X` を読んで落ち（`StrictMode`）、`FOLLOWUP RESULT` 行が出ない。`invoke-cpu-sample.ps1` は引数なしだと必須の引数を問う所で落ち（`-NonInteractive`）、`default` の枝（`:605`）には来ない。
- **直し方**: 各スクリプトの頭に `trap` を 1 つ置き、理由を 1 行出してから、決まった終了コードと RESULT 行で終える（`perf-loop.ps1` は `Stop-Run` を通す）。届かない `default` の枝は消す。
- **直さないと**: 背景の走行の終わりに RESULT 行が無く、スキルは「進行中」と「落ちた」を見分けられない。
- **旗**: (a) 変える（落ちたときの終了コードを何にするか・RESULT 行が出るようになる。読み手はスキルとエージェント。終了コードの体系は完了した spec `areka-P0-draw-load-parity` の design C5） ／ (b) いいえ ／ (c) ⑴候補（終了コードの体系そのものは完了した spec の設計。新しい語を足さず「4＝道具の不具合」に寄せるなら体系の中） ／ (d) T2・T1（わざと例外を起こす小さな入口で、RESULT 行と終了コードを見る）

**4. `-Build Release` のように大文字が混ざると、25 分走った後で判定が引数を拒む** （`perf-loop.ps1:636-638`・`perf-loop.measure.ps1:953`・`:333-336`・`perf-loop.ps1:632`・`:225`）
- **何か**: `-Build` と目標定義の `builds_final` は `-notcontains`（大文字小文字を区別しない）で確かめる。
- **困る場面**: `measure-baseline -Build Release` は確かめを通り、ランナーは小文字にして 25 分走る。最後に `judge-perf.py --build Release` を呼ぶと、`--build` は `dev`・`release` だけ（`judge-perf.py:4082`・区別する）なので判定が付かず 4（計測失敗）で終わる＝25 分を失う。サブコマンドも同じで、`Rank-Run` は通り、RESULT 行に語彙の外の綴り `Rank-Run` がそのまま出る（頭書は「語彙外の字面を受け取らないように」と書く）。
- **直し方**: 入口で `$Sub` と `$Build` を小文字にそろえてから確かめる（1 行ずつ）。
- **直さないと**: 打ち間違い 1 字で 25 分を失い、台帳に MEASURE_FAILED が残る。
- **旗**: (a) 変える（軽い：RESULT 行の `<sub>` が必ず小文字になる。読み手はスキル） ／ (b) いいえ ／ (c) なし ／ (d) T2（`-DryRun` の入口で `Release` を渡し、判定へ渡る語が `release` であること）

**5. xperf への引数が空白を含むと割れる** （`invoke-cpu-sample.ps1:212-213`・`:227-234`・`:354`・`:359`・`:454`・`:474`・`:480`）
- **何か**: `Invoke-NativeCommand` は `Start-Process -ArgumentList <配列>` で xperf と wpaexporter を呼ぶ。
- **困る場面**: `Start-Process` は配列の要素を空白でつなぐだけで括らない（この机の pwsh 7.7.0 で `'C:\dir with space\x.etl'` が `C:\dir` と `with` の 2 つに割れるのを確かめた）。利用者のフォルダの名前に空白がある机（出力先の既定は `%LOCALAPPDATA%` の下）や、`-OutRoot`・`-RunDir` に空白を含むパスを渡すと、`-f <etl>`・`-d <merged>`・`-o <dump>`・`-i <etl>` が割れ、段③は `start_failed` で黙って抜けるか、`-Stop` が 1 で落ちる。今の机のパスには空白が無いので、まだ踏んでいない。ランナーの方は要素を自分で `"` で括っている（`invoke-perf-run.ps1:785`）＝同じことを 2 か所で別のやり方でしている。
- **直し方**: `ProcessStartInfo.ArgumentList` に 1 つずつ入れる形（`perf-loop.common.ps1` の `Invoke-Child` と同じ）にする。読みの文字コードだけは今のまま端末の文字コードにする（19 と一緒に）。
- **直さないと**: 空白のあるパスの机では段③が取れず、理由も「start_failed」としか残らない。
- **旗**: (a) なし ／ (b) いいえ（偽の実行体で引数の数を数えれば決定論に試せる） ／ (c) なし ／ (d) T1（`invoke-cpu-sample.ps1 -SelfTest` で、空白を含む引数を偽の実行体へ渡して 1 つのまま届くこと）

**6. `-DryRun` の後の本番の走行が、利用者が渡したフォルダを丸ごと消す** （`perf-loop.measure.ps1:199-208`・`:211-220`・`:571`・`:583`）
- **何か**: `Clear-MeasureDryRunDir` は「`DRY-RUN.txt` が在って `run.log` が無い出力先」を `Remove-Item -Recurse -Force` で作り直す。
- **困る場面**: 実行体がまだ無い机で `measure-baseline -DryRun -RunDir D:\perfruns` を回すと、`Write-MeasureDryRunMark` が既に在る `D:\perfruns` へ `DRY-RUN.txt` を置く。続けて `measure-baseline -RunDir D:\perfruns` を回すと、`D:\perfruns` の中身（ほかの走行の成果を含む）が丸ごと消える。
- **直し方**: 消すのは「中身が `-DryRun` の置いたファイル（`DRY-RUN.txt`・ランナーの `-DryRun` が書く `run-meta.txt`）だけ」のときに限る。それ以外が在れば止めて理由を出す。
- **直さないと**: 測った成果が戻せない形で消えうる。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T2（`DRY-RUN.txt` とほかのファイルを置いた偽の出力先が消えないこと・`DRY-RUN.txt` だけのものは作り直されること）

### 直したほうがよい（Should fix）

**7. 子の pwsh が想定外の失敗で落ちると、理由の行が色の符号と化けた字になる** （`perf-pwsh-child.ps1:30-39`・`perf-loop.common.ps1:252-261`）
- **何か**: 子の pwsh は前置きで自分の標準出力・標準エラーだけを UTF-8 の書き手に替える。
- **困る場面**: 子のスクリプトが捕まえていない例外で落ちると、PowerShell が書くエラーの表示は色の符号（ESC `[31;1m` など）付きで来て、地域語の文（日本語の `.NET` の文）は化けて届く。`Invoke-Child` の「最後の 1 行」は色の符号だけの行になる（この机で、存在しないファイルの `Copy-Item` で落ちる小さなスクリプトを `Invoke-PwshChild` で回して確かめた。LastLine＝`ESC[36;1mESC[31;1m…ESC[0m`）。`perf-loop` の失敗の文（「failed (exit code 1): <最後の 1 行>」）の理由が全部読めなくなる。
- **直し方**: 前置きで、子のスクリプトの呼び出しを `try`/`catch` で包み、例外の文を `[Console]::Error`（UTF-8 の書き手）へ素の文字で書いてから 1 で終える。あわせて「最後の 1 行」を選ぶ所で色の符号を落とす。端末の文字コードには触らない。
- **直さないと**: 想定外の失敗ほど理由が残らない（「記録の無い失敗の道」に近い）。
- **旗**: (a) 守る（`encoding check` は `Invoke-PwshChild` が子の 3 行を字のとおりに届けることと、端末が 932 のままであることを確かめている＝前置きを変えても、その 2 つを崩さない） ／ (b) いいえ ／ (c) なし ／ (d) T2（わざと落ちる子を回し、最後の 1 行が理由の文で、色の符号を含まないこと）。`encoding check` に同じ確かめを足すかは `audit-tools-scripts` の持ち物なので、足すなら申し合わせ。

**8. areka を強制終了した直後に、終わるのを待たずに出力先の名前を替える・次の走行を起こす** （`invoke-perf-run.ps1:816`・`:831`・`:850`・`:885`・`:236-245`・`invoke-followup-checks.ps1:829`・`:844`・`perf-loop.measure.ps1:247-249`）
- **何か**: 時間切れのときは `$proc.Kill()` してすぐ `Stop-Run` で出力先を `<leaf>-FAILED` へ替える。`Kill()` は終わるのを待たない。
- **困る場面**: まだ消えきっていない areka が `run.log` を握っていると、名前の替えが「使用中」で失敗し、出力先は元の名前のまま残る。`perf-loop` の 2 回目の採り直しは「出力先が既に在る」で 3（引数の不正）で止まる＝採り直しが効かず、理由も違う。追随の検査でも、`finally` の `Kill()` の直後に `perf-loop` が 2 回目を起こし、同じ `run.log` へ書こうとして起動に失敗しうる。
- **直し方**: `Kill()` の後に `WaitForExit(<数秒>)` を置く（2 つのファイルで同じ 1 行）。
- **直さないと**: 応答の無い走行が 1 度出ると、採り直しの代わりに「引数の不正」で止まる。
- **旗**: (a) なし ／ (b) 一部（取り合いの起きやすさは本物の走行でしか分からない。「待ってから替える」順は、ファイルを握るだけの偽のプロセスで決定論に試せる） ／ (c) なし ／ (d) T3（偽のプロセスに `run.log` を握らせ、`Stop-Run` の後に `-FAILED` へ替わっていること）

**9. `rank-run` の段③の採取が、走行の前の `cargo build` と走行の前の静かさの確認まで含んでしまう** （`perf-loop.measure.ps1:632-651`・`:242`・`invoke-perf-run.ps1:630-659`）
- **何か**: 採取の `-Start`（`:638`）の後で `Invoke-MeasureRun` を呼び、その中で作業ツリーからの `cargo build`（`:242`）と、ランナーの `-AutoQuiet` の静かさの確認（最大 4 回・待ちを入れて数分）が走る。
- **困る場面**: 採取の窓にビルドと待ちが入り、ETL がその分だけ太る。静かさの確認は xperf の採取（約 8 kHz・全 CPU・呼出スタックつき）が動いている机の CPU を測るので、採取そのものの重さで「静かでない」に倒れうる。倒れると 2 で止まり、1 の漏れにもつながる。
- **直し方**: 走行の前のビルドを採取の開始より前へ移す。静かさの確認と採取の順をどうするか（ランナーに採取の開始を任せる・確認を採取の前に済ませる）は設計で決める。
- **直さないと**: 段③の採取の度に静かさの確認が揺れ、ETL も余分に大きい。
- **旗**: (a) なし ／ (b) はい（xperf の重さが静かさの確認を倒すかは、昇格した机の本物の走行でしか分からない） ／ (c) ⑴候補（採取と走行の順は完了した spec の設計 Flow 2 の形。順を入れ替えるのは設計の中の直しで、裁定に反するかは要確認） ／ (d) T2（偽の採取と偽の走行で、呼ばれる順が「ビルド → 採取の開始 → 走行 → 採取の停止」であること）

**10. `final` が `-BinDir` を黙って無視する** （`perf-loop.measure.ps1:959-960`・`perf-loop.ps1:142-144`・`tools/perf/README.md:553`）
- **何か**: 入口の引数の説明は「`-BinDir` 省略時は target」とし、README も「`-BinDir` を渡さない走行（followup／rank-run／measure-baseline／final）は作り直す」と書く。
- **困る場面**: `final -BinDir C:\x` と渡しても、`final` は `Test-MeasureExeReady -Build` と `Invoke-MeasureRun`（`-BinDir` なし）を呼ぶので、作業ツリーから作り直した `target\<build>` を測る。渡した実行体の合否ではないのに、合否が spec の `results` へ写る。
- **直し方**: `final` でも `$script:BinDirArg` を `Test-MeasureExeReady` と `Invoke-MeasureRun` へ渡す（2 か所）。逆に「`final` は `-BinDir` を取らない」と決めるなら、渡されたら 3 で止める。
- **直さないと**: 別の実行体の合否を取り違えて記録しうる。
- **旗**: (a) 変える（軽い：`final -BinDir` の振る舞い。README は `audit-tools-perf-judge` の持ち物で、今の文はむしろ直した後に合う） ／ (b) いいえ ／ (c) なし ／ (d) T2（`-DryRun final -BinDir <偽>` で、ランナーへ渡る `-BinDir` を見る）

**11. 「実行体が在るか」の確かめが、走行の前のビルドより先にある** （`perf-loop.measure.ps1:148-164`・`:579`・`:618`・`:959`・`perf-loop.common.ps1:650-662`）
- **何か**: `measure-baseline`・`rank-run`・`final` は、実行体が無いと「先に `cargo build` を」と 3 で止める。ところが走行の直前に `Sync-MeasureWorktreeBuild` が同じ実行体を必ず作り直す。
- **困る場面**: 作ったばかりのワークツリーで周 0 の `measure-baseline` を回すと、道具が自分で作れるのに 3 で止まり、1 ターンを失う。
- **直し方**: `-BinDir` の無い走行では、確かめをビルドの後へ回す（ビルドが確かめを兼ねる）。
- **直さないと**: 新しい机で毎回 1 度、無駄な 3 を踏む。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T2（偽の `cargo` で、実行体の無い木でもビルドの後に確かめが通ること）

**12. 32bit ヘルパの写しが、無いときだけで、古くなっても替わらない** （`perf-loop.common.ps1:633-644`）
- **何か**: `Sync-MeasureShioriHelper` は `target\<release|debug>\shiori-host32-helper.exe` が無いときだけ、`target\i686-pc-windows-msvc\release\` から写す。
- **困る場面**: 一度写した後に SHIORI の中継を直してヘルパを作り直しても、実行体の隣は古いヘルパのまま。`measure-baseline`・`rank-run`・`final` は古いヘルパで測る。
- **直し方**: 元が新しい（書き込み時刻か SHA-256 が違う）ときも写す。
- **直さないと**: ヘルパを直した周の測りが、直す前のヘルパで行われる。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T2（`target` の下に偽の 2 つのファイルを置き、元が新しければ写ること）

**13. `followup` が、`-Resume` で済みを確かめる前にビルドと検体の展開をする・使わない実行体まで作る** （`perf-loop.measure.ps1:858-897`）
- **何か**: `followup` は最初に `Sync-MeasureWorktreeBuild`（`cargo build --release`）を回し、引数を組むときに `Get-MeasureGhostRoot`／`Get-MeasureBalloonRoot`（`cargo run … nar-sample-path`＝呼ぶたびに検体を展開し直す）を呼び、その後で `-Resume` の済みを見る。`bin-B` が在れば、作り直した `target\release` ではなく `bin-B` を使う。
- **困る場面**: 済んだ `followup -Resume`（スキルが RESULT 行を取り直すために呼ぶ）でも、ビルドと検体の展開を毎回やる。`bin-B` を使う周でも、使わない `target\release` を作る。
- **直し方**: `-Resume` の済みの確かめを先頭へ移し、ビルドは `target\release` を使うときだけにする。
- **直さないと**: RESULT 行を取り直すだけの呼び出しが数十秒〜数分かかる。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T2（済みの偽の出力先で `-Resume` を呼び、`cargo` が呼ばれないこと）

**14. 追随の判定が「ログが読めない」（3）で終わると、`perf-loop` は「引数の不正」として採り直さない** （`perf-loop.measure.ps1:914-916`・`invoke-followup-checks.ps1:55-57`・`:852-865`）
- **何か**: `invoke-followup-checks.ps1` は自分の引数の不正に 3 を使い、判定まで来たら `judge-followup.py` の終了コードをそのまま返す。`judge-followup.py` の 3 は「読取不能」（`EXIT_BAD_INPUT`）。
- **困る場面**: areka が起動直後に落ちて `run.log` に要る行が無いと、判定は 3 を返す。`perf-loop` は子の 3 を「引数・前提の不正」として 3 で止め、1 度の採り直しもしない。実際は走行の失敗（4 に当たる）。
- **直し方**: `invoke-followup-checks.ps1` が判定の 3 を自分の 3 と分けて返す（例: 判定の 3 は 1＝実走の失敗に写す）か、`perf-loop` が `followup-verdict.txt` の有無で見分ける。
- **直さないと**: 一過性の起動失敗で、追随の相が「引数の不正」として止まる。
- **旗**: (a) 変える（`followup` の終了コード。読み手はスキル。`judge-followup.py` の終了コードは `audit-tools-perf-judge` の持ち物＝こちらで写し方を変えるだけなら相手のファイルは触らない） ／ (b) いいえ ／ (c) ⑴候補（「終了コードは judge-followup.py の体系をそのまま返す」は完了した spec の設計 C13） ／ (d) T2（偽の判定が 3 を返したとき、採り直しに入ること）

**15. `measure-ab` が、A と B が同じ実行体でも比べ始める** （`perf-loop.measure.ps1:742-747`・`:543-558`）
- **何か**: `BUILD.txt` に `exe_sha256` を書くが、A と B を突き合わせる所が無い。`prepare-ab` は `-Resume` なしで回すと `bin-A` を今の木で作り直す。
- **困る場面**: 変更を入れた後に `prepare-ab` を `-Resume` なしで回し直すと、A も変更後の実行体になる。`measure-ab` は同じ実行体を 4 本 7 分ずつ比べ、「差なし」として台帳に残す。
- **直し方**: `measure-ab` の頭で `bin-A` と `bin-B` の `exe_sha256` を比べ、同じなら走行の前に止める。
- **直さないと**: 30 分かけて同じものを比べ、変更の効き目が無かったと誤って記録する。
- **旗**: (a) 変える（軽い：`measure-ab` に止まる道が 1 つ増える。何番で止めるかは設計） ／ (b) いいえ ／ (c) 候補ではないが、要件 3.4（新しい検査を足さない）に当たるかを要件の討議で確かめる ／ (d) T2（同じ中身の偽の `bin-A`・`bin-B` で止まること）

**16. `wpaexporter` の道は使えないまま残っている** （`invoke-cpu-sample.ps1:106-113`・`:126`・`:145`・`:187-193`・`:373-387`・`:443-450`・`:468-477`・`perf-loop.common.ps1:398`・`:421`・`perf-loop.ps1:598`）
- **何か**: 記号の解決の代わりの道として `wpaexporter` を選べる（`-Backend`・目標定義の `[sampling] backend`）。
- **困る場面**: 要る `tools/perf/wpa/cpu-sampled.wpaProfile` は一度も作られていない（`results/reconcile-2026-08-23.md:43`「計画にあって未作成」・フォルダも無い）。選ぶと必ず 3 で止まり、その止まり方は 1 の漏れの道でもある。仮にファイルを置いても、`wpaexporter` は書き出すファイルの名前を `-Out` に合わせないので、直後の「dump が書かれていない」で必ず 4 になる。
- **直し方**: `wpaexporter` の道を消す（`-Backend`・`Get-BackendFromGoalFile`・`Get-WpaExporterPath`・定数 2 つ・`perf-loop` の `SamplingBackend` と `preflight` の `sampling_backend=` の行）。消す前にツリー全体で参照を確かめる。
- **直さないと**: 使えない道が約 40 行と引数 1 つ、目標定義の節 1 つとして残り、選んだ人を必ず止める。
- **旗**: (a) 変える（`-Backend` の引数・`preflight.txt` の `sampling_backend=`・目標定義の `[sampling]`〔`goals/` は `audit-tools-perf-judge` の持ち物〕・README の説明） ／ (b) いいえ ／ (c) ⑴候補（代わりの道は完了した spec の設計 C8・research の DD-5 に書かれている） ／ (d) 消すだけなら新しいテストは要らない（`invoke-cpu-sample.ps1 -SelfTest` が通ること）

**17. 目標定義を読む小さな解釈器が 4 つあり、すでに読み方が食い違っている** （`perf-loop.common.ps1:317-346`・`:449-462`・`invoke-perf-run.ps1:391-421`・`check-quiet.ps1:248-305`・`invoke-cpu-sample.ps1:374-387`）
- **何か**: 同じ TOML を、`Read-TomlSections`・`Read-QuietRetrySettingsFromToml`・`Read-QuietSettingsFromToml`・`Get-BackendFromGoalFile` が別々に読む。
- **困る場面**: 節の見出しの見方が 2 通り（`^\[([^\]]+)\]\s*$` と `^\[(.+)\]$`）・注釈の落とし方が 3 通り（1 つは落とさない）。どれも単引用符の文字列（TOML として正しい `['cargo']`）を読めず、黙って既定値へ落ちる。Python の道具は `tomllib` で同じファイルを正しく読むので、同じ目標定義を PowerShell と Python で違う値として使う。直すたびに 4 か所を同時に直す必要がある。
- **直し方**: 読み口を `perf-loop.common.ps1` の 1 つに寄せ、ほかの 3 本はそれを使う（`check-quiet.ps1`・`invoke-perf-run.ps1`・`invoke-cpu-sample.ps1` が共通のファイルを読み込む形にするか、読み口だけの小さなファイルにするかは設計）。
- **直さないと**: 目標定義を書き換えたとき、どれか 1 本だけが黙って既定値を使う。
- **旗**: (a) なし（読める値は今と同じか広がるだけ） ／ (b) いいえ ／ (c) なし ／ (d) T2（`check-quiet.ps1 -SelfTest` の TOML の検査を、寄せた読み口の検査として 1 か所に）

**18. 検体の在り処の問い合わせと `python` の探し方が、2 本に同じ形で書かれている** （`invoke-followup-checks.ps1:132-134`・`:243-293`・`perf-loop.common.ps1:122-170`）
- **何か**: `Get-NarSampleMap`／`Get-NarSamplePath`／`Get-PythonCommand` は両方のファイルにあり、注釈が自ら「同形（Stop-Run の形だけが違う）」と書く。
- **困る場面**: 検体の窓口の出力の形が変わると、2 か所を同時に直す必要がある。片方だけ直すと、`perf-loop` からの走行と単独の `followup` で検体の場所が食い違う。
- **直し方**: 片方を消して 1 つを使う（止め方の違いは呼び手が渡す）。`perf-loop` からの呼び出しは常に `-GhostRoot`・`-BalloonRoot` を渡しているので、`invoke-followup-checks.ps1` 側の問い合わせは単独で呼ぶときだけに使われる。
- **直さないと**: 同じ規則の写しがずれる。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T2（表を作る部分を、偽の窓口の出力から作る検査）

**19. 子のプロセスの起こし方が 3 通りある** （`perf-loop.common.ps1:192-269`・`invoke-perf-run.ps1:446`・`invoke-followup-checks.ps1:252`・`:302`・`invoke-cpu-sample.ps1:205-222`）
- **何か**: `Invoke-Child`（`ProcessStartInfo`・UTF-8 で読む）、`Invoke-Utf8Child`（`tools/utf8-child.ps1`）、`Start-Process`＋受け皿のファイル（端末の文字コードで読む）。
- **困る場面**: 5 の空白の割れは 3 つ目だけにある。終了コードの持ち帰り方・最後の 1 行の選び方・文字コードも 3 通りで、直すたびに 3 か所を見る。
- **直し方**: `invoke-cpu-sample.ps1` の分は、読む文字コードを選べる形で `Invoke-Child` に寄せる（5 の直しと同じ）。`Invoke-Utf8Child` は範囲の外（`tools/`）の道具で、範囲の外の道具も使っているので残す。
- **直さないと**: 子の扱いの直しが 1 か所だけに入る。
- **旗**: (a) 守る（`encoding check` が `Invoke-Child -Exe -Arguments -Quiet` を呼んでいる＝今の引数の形を崩さない。足すなら引数を足すだけ） ／ (b) いいえ ／ (c) なし ／ (d) T2（読む文字コードを選んだときの決定論の検査）

**20. 「共通の部分」が、呼び手にしか無い変数と関数に頼っていて、単独では読み込めず、テストもできない** （`perf-loop.common.ps1:125-158`・`:629-678`・`perf-loop.ps1:208-214`・`:246`・`:625-685`）
- **何か**: `perf-loop.common.ps1` の `Get-NarSampleMap`・`Sync-MeasureShioriHelper`・`Sync-MeasureWorktreeBuild`・`Test-MeasureQuietAfterOk` は、`perf-loop.ps1` が持つ `$EXIT_MEASURE_FAILED`・`$repoRoot` と、`perf-loop.measure.ps1` が持つ `$MEASURE_*`・`Invoke-MeasureCargo`・`Get-MeasureBinDir` などに頼る。置き場は「行数の上限のため」（注釈）。一方 `perf-loop.ps1` は頭から引数の確かめ・振り分け・`exit` まで一続きで、読み込むだけで走る。
- **困る場面**: `encoding check` は `perf-loop.common.ps1` を単独で読み込んでいる。そこでこれらの関数を呼ぶと、変数が無いので `StrictMode` で落ちる。また、直したバグのテスト（要件 4）を書こうとしても、`perf-loop.ps1` を読み込めないので、測る側の関数を呼ぶには定数を写して並べ直すしかない。
- **直し方**: 終了コードなど共通の定数を `perf-loop.common.ps1` へ移し、測る側だけが使う関数は測る側のファイルへ戻す。`perf-loop.measure.ps1` は 993 行で上限 1,000 に近いので、戻すなら新しいファイルに分ける（仕事ごとに分ける）。
- **直さないと**: 2〜15 の直しのテストが書きにくく、写した定数がずれる。
- **旗**: (a) 守る（`encoding check` が単独で読み込める形と、`Invoke-Child`・`Invoke-PwshChild` の名前を保つ） ／ (b) いいえ ／ (c) なし ／ (d) この直しが T2 の前提（テスト用の入口から関数だけを読み込めるようにする）

**21. `perf-loop.ps1 selftest` は、`perf-loop` 自身の判断の分かれ目を 1 つも試していない** （`perf-loop.ps1:194-203`・対象 `perf-pwsh-child.ps1:23-41`・`perf-loop.common.ps1:317-346`・`:449-462`・`:497-534`・`:667-678`・`perf-loop.measure.ps1:181-208`・`:847-851`・`:917-924`）
- **何か**: 自分の検査は Python の 5 本、PowerShell の 3 本の `-SelfTest` と、条件文の一致の検査だけ。
- **困る場面**: 引数の括り方（`Get-PwshChildArguments`）、TOML の読み、出力先の語彙、`-Resume` の完了の見方（2）、出力先を退ける・消す所（2・6）、追随の「判定まで届いたか」の見方、子の終了コードの写し方に、壊れたら落ちるテストが無い。どれも過去に実際に踏んだ穴の直し（注釈に 2026-08-23 の日付）で、また戻っても気付けない。
- **直し方**: 20 の後で、これらを決定論に試す自分の検査を 1 本足し、`SELFTEST_TOOLS` に入れる。
- **直さないと**: 直した穴が黙って戻る。
- **旗**: (a) 変える（軽い：`selftest` の `ok=` の数と `preflight` の `selftest=ok ok=<n>` の数が増える。語は同じ） ／ (b) いいえ ／ (c) なし ／ (d) T2

**22. `invoke-perf-run.ps1` には自分の検査が無い** （`invoke-perf-run.ps1:260-290`・`:391-421`・`:891-907`・`perf-loop.ps1:194-203`）
- **何か**: CPU のカウンタを PID で選ぶ所（注釈が「InstanceName を鍵にすると別のプロセスの値を黙って記録する」と書く 2026-08-14 の穴）、`[quiet]` の回数の読み、早すぎる終わり・目印の無いログの見分けに、テストが無い。
- **困る場面**: カウンタの選び方が `InstanceName` 鍵へ戻っても、別の areka の CPU を測ったまま緑で走る。
- **直し方**: カウンタの結果から値を選ぶ部分を、入力を受けて値を返す関数にし（`Get-Counter` を呼ぶ所と分ける）、`-SelfTest` で偽の `CounterSamples` を与えて試す。`SELFTEST_TOOLS` に足す。
- **直さないと**: 測った値の取り違えが黙って戻る。
- **旗**: (a) 変える（軽い：21 と同じく `selftest` の数が増える） ／ (b) いいえ ／ (c) なし ／ (d) T3

**23. 段③の dump を丸ごとメモリへ読んで数える** （`invoke-cpu-sample.ps1:493`・`:267`）
- **何か**: `-Stop` の関門は `Get-Content -Raw` で dump を 1 つの文字列にし、さらに行の配列に割る。
- **困る場面**: dump はすべての CPU の採取（約 8 kHz）と呼出スタックの行を含み、採取の時間（7 分＋9 のビルドと待ち）に比例して大きくなる。文字列と行の配列の 2 つ分がメモリに載り、数 GB の dump なら遅くなるか落ちる。
- **直し方**: `[IO.File]::ReadLines` で 1 行ずつ数える（関数の中身はそのまま・入力を行の列にするだけ）。
- **直さないと**: 長い採取の後の関門で、道具がメモリ不足で落ちうる。
- **旗**: (a) なし ／ (b) はい（実物の dump の大きさは昇格した机の本物の走行でしか分からない） ／ (c) なし ／ (d) T1（今の検体の dump で同じ数になること）

**24. 5 秒の本物の採取の試しを、要らない所でも回す** （`perf-loop.ps1:457`・`:534`・`invoke-cpu-sample.ps1:584`・`perf-loop.measure.ps1:420`）
- **何か**: `preflight` は `Get-SamplingProbe`（`-Probe`＝昇格した机では xperf で 5 秒採って止める）を回し、続く自分の検査の中の `invoke-cpu-sample.ps1 -SelfTest` がもう 1 度同じ試しをする。`rank`（既にある走行から表を作り直すだけ）は、`sampling.txt` が無いと、今の机で試しを回して理由の語を埋める。
- **困る場面**: 昇格した机の `preflight` は 10 秒とカーネルの採取 2 回を余分に使う。`rank` は、古い走行の理由の欄に「今日の机」の可否を書く（走ったときの机とは限らない）。
- **直し方**: `preflight` の試しは 1 回の結果を使い回す。`rank` は `sampling.txt` が無ければ `probe_failed` ではなく「記録が無い」と書く（語は設計）か、試しを回さない。
- **直さないと**: 遅さと、古い走行への誤った理由の記入。
- **旗**: (a) なし（`rank` の理由の語を新しく作るなら、`perf-rank.py` の受け付ける語との申し合わせ） ／ (b) 一部（昇格した机でだけ差が出る） ／ (c) なし ／ (d) T2（偽の `-Probe` が何回呼ばれたかを数える）

### できれば（Nice to have）

**25. 落ちることの無い確かめ・使われない値・二重の既定値** （`perf-loop.ps1:167`・`:172`・`:228`・`:480-486`・`:590-591`・`:681`・`perf-loop.measure.ps1:696`・`invoke-cpu-sample.ps1:605`）
- **何か**: `#Requires -Version 7.0` があるので `pwshOk` は必ず真。`pythonOk` は作るが合否に使わない（3.10 でも `preflight` は 0 で通りうる）。`$DEFAULT_GOAL` は引数の既定値の写し。`rank` の `-Resume` は同じことを 2 度確かめる。`switch` の `default` の枝は来ない。
- **困る場面**: 読む人が「版の確かめで止まる」と誤解する。値を変えるとき 2 か所を見る。
- **直し方**: 使わない値は消すか、`pythonOk` は合否に使う（どちらかは設計）。
- **旗**: (a) 変える（軽い：`preflight.txt` の `pwsh=… ok=`・`python=… ok=` の行） ／ (b) いいえ ／ (c) ⑴候補（Python／PowerShell の版を見ることは完了した spec の設計 C5 の preflight の項目） ／ (d) T2（`pythonOk` を合否に使うなら、版の低い偽の `python` で 4 になること）

**26. `-Goal` と `-GoalFile` を両方渡したときの扱いが、道具ごとに違う** （`perf-loop.ps1:655`・`perf-loop.common.ps1:373-377`・`invoke-perf-run.ps1:537-539`・`check-quiet.ps1:556`）
- **何か**: ランナーと静かさの確認は 3 で拒むが、`perf-loop` は黙って `-GoalFile` を使う。
- **困る場面**: `preflight -Goal a -GoalFile b.toml` は `a` を無視して `b` で進み、出力先の名前も `b` の中の名前になる。
- **直し方**: `perf-loop` も 3 で拒む（1 行）。
- **旗**: (a) 変える（軽い：止まる道が 1 つ増える） ／ (b) いいえ ／ (c) なし ／ (d) T2

**27. 相対パスの解き方が 2 通りある** （`perf-loop.common.ps1:483`・`perf-loop.measure.ps1:130`・`:135`・`:141`・`:860` と `perf-loop.common.ps1:556-561`）
- **何か**: `-RunDir` は PowerShell の今の場所につないで解くが、`-OutRoot`・`-BinDir`・`-GhostRoot`・`-BalloonRoot` は `[IO.Path]::GetFullPath`（プロセスの今の場所）で解く。
- **困る場面**: 対話の pwsh で `cd` した後に `& tools/perf/perf-loop.ps1 … -BinDir .\bin` と呼ぶと、`-BinDir` だけが起動したときの場所を基に解かれ、別のフォルダを見る（`pwsh -File` で新しく起こすスキルの呼び方では起きない）。
- **直し方**: 1 つの解き方（`Resolve-RunDir` と同じ）に寄せる。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T2

**28. 「静かでない」の証拠の場所の案内が、別のフォルダを指しうる** （`perf-loop.measure.ps1:270`・`invoke-perf-run.ps1:238-240`）
- **何か**: `perf-loop` は「証拠は `<OutDir>-FAILED` の `quiet-before.txt`」と書くが、ランナーはその名前が既に在ると時刻を足した名前にする。
- **困る場面**: 同じ出力先で 2 度目の「静かでない」が出ると、案内は 1 度目の古い証拠を指す。
- **直し方**: ランナーが替えた後の名前を 1 行出し、`perf-loop` はそれを写す（または案内を「`<OutDir>-FAILED*`」とする）。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T3

**29. 段③の生の ETL が、まとめた ETL の隣にずっと残る** （`perf-loop.measure.ps1:625`・`invoke-cpu-sample.ps1:453`）
- **何か**: `-Stop` は `-d` でまとめた `<name>.merged.etl` を作るが、元の `<name>.etl` は消さない。どちらも周の出力先に残り、周ごとに増える。
- **困る場面**: 同じ採取が 2 つの大きなファイルとして残り、30 周で 60 個になる。
- **直し方**: まとめが成功したら元の ETL を消す（記号の解決はまとめた方だけを読む）。
- **旗**: (a) なし（Python の道具は dump だけを読む） ／ (b) はい（大きさは本物の採取でしか分からない） ／ (c) なし ／ (d) T1（偽の `xperf` で、まとめの後に元が消えること）

**30. 段③の「能力不足（5）」の理由の語を、子の出力を読まずに決め打ちしている** （`perf-loop.measure.ps1:643`・`:662-664`）
- **何か**: 開始が 5 なら必ず `not_elevated`、停止が 5 なら必ず `no_xperf` と書く。子は `reason=…` を出している。
- **困る場面**: 開始の 5 が「xperf が無い」でも、`sampling.txt` と順位表に `not_elevated` と残る。
- **直し方**: 子の `available=false reason=<語>` の行から語を取る（`Get-SamplingProbe` と同じ読み方）。
- **旗**: (a) なし（語の集まりは同じ） ／ (b) いいえ ／ (c) なし ／ (d) T2

**31. ランナーが目印を探すために run.log を丸ごと読む** （`invoke-perf-run.ps1:901-902`）
- **何か**: 有界の自動終了の目印 1 行を探すのに、`Get-Content -Raw` で全体を読み、正規表現で探す。目印は起動の直後に出る。
- **困る場面**: 点灯した `rank-run` の run.log（`wintf::tick=debug`）は大きく、読むのに時間とメモリを使う。
- **直し方**: `Select-String -SimpleMatch -List`（最初の 1 件で止まる）にする。
- **旗**: (a) なし ／ (b) はい（実物の大きさは本物の走行でしか分からない） ／ (c) なし ／ (d) T3

**32. 追随の検査が、待つ間ずっと run.log の全体を読み直し、全行の時刻を解き直す** （`invoke-followup-checks.ps1:310-352`）
- **何か**: `Wait-RunLogPattern` は 200 ms ごとに `Read-RunLogText` で全体を読み、`Get-RunLogLinesSince` で全行の時刻を解く。
- **困る場面**: 表示の成立を最大 60 秒待つ間に、伸びていくログを 300 回読み直す。
- **直し方**: 読んだ位置を覚えて、増えた分だけ読む。
- **旗**: (a) なし ／ (b) 一部（遅さが効くかは本物の走行のログの大きさ次第） ／ (c) なし ／ (d) T1

**33. 条件文の写しの字数の確かめが、字数が読めないと黙って飛ばされる** （`perf-loop.ps1:313-329`）
- **何か**: 本文が一致したときに、`goal-text` が報告する字数と写しの頭書の字数を比べる。どちらかが読めなければ比べずに緑にする。
- **困る場面**: `goal-text` の報告の文の形が変わって字数が読めなくなると、字数の確かめがずっと効かないまま緑が続く。
- **直し方**: `goal-text` 側の字数が読めないときは赤にする（写しの頭書に字数が無いときだけ飛ばす）。
- **旗**: (a) なし ／ (b) いいえ ／ (c) なし ／ (d) T2

**34. `ab_sequence` に 4 本の並べ替え以外を書くと、走り終わってから止まる** （`perf-loop.measure.ps1:733-763`・`:788-801`）
- **何か**: 並びは目標定義から読めるが、比べる側は A1・A2・B1・B2 の 4 本を決め打ちで要る。
- **困る場面**: `["A", "B"]` と書くと、7 分 × 2 本を走らせてから「A2 が無い」で 4 になる。
- **直し方**: 並びを読んだ所で、A と B がちょうど 2 回ずつかを確かめ、走る前に 3 で止める。
- **旗**: (a) 変える（軽い：止まる番号と時期） ／ (b) いいえ ／ (c) なし ／ (d) T2

**35. 表示と時刻の小さな関数が 5 本のファイルに写してあり、時刻の書き方も 2 通り** （`perf-loop.common.ps1:78-90`・`invoke-perf-run.ps1:205-217`・`check-quiet.ps1:133-138`・`invoke-cpu-sample.ps1:160-161`・`invoke-followup-checks.ps1:209-215`）
- **何か**: `Write-Info`・`Write-Problem`・`Get-UtcStamp` がファイルごとにあり、`preflight.txt`・`BUILD.txt`・`DRY-RUN.txt`・`sampling.txt` の `time_utc` は末尾に `Z` が無く、`run-meta.txt`・`quiet-*.txt` は `Z` がある。`invoke-cpu-sample.ps1` は失敗の文も標準出力へ書く。
- **困る場面**: 同じ「UTC の時刻」の欄が、ファイルによって UTC と読めたり読めなかったりする。
- **直し方**: 時刻の書き方を 1 つにそろえる（`Z` 付き）。関数の写しを寄せるかは 17 の置き場と一緒に決める。
- **旗**: (a) 変える（`preflight.txt`・`BUILD.txt` の `time_utc` の字面。今それを解いて読む Python は見つからなかったが、申し合わせの対象） ／ (b) いいえ ／ (c) なし ／ (d) T2

**36. `preflight` が昇格と xperf を自分でも確かめ、試しでももう 1 度確かめる** （`perf-loop.ps1:173`・`:453-457`・`perf-loop.common.ps1:594-611`）
- **何か**: `Test-Elevated`・`Find-XperfPath`・xperf の既定の場所（`invoke-cpu-sample.ps1` の写し）を `perf-loop` も持つ。`-Probe` も同じことを見る。
- **困る場面**: xperf の既定の場所を変えるとき、2 か所を同時に直す必要がある。片方だけ直すと、`capabilities=` の `xperf:` と `function_stage` が食い違う。
- **直し方**: `-Probe` の出力（理由の語）から `elevated`・`xperf` を出すか、`-Probe` に在り処も出させて 1 か所にする。
- **旗**: (a) 変える（`preflight.txt` と `capabilities=` の `elevated`・`xperf`・`xperf_path` の値の出どころ。語と並びは保つ） ／ (b) いいえ ／ (c) ⑴候補（`preflight` の項目は完了した spec の設計 C5） ／ (d) T2

**判定（Verdict）**: 動くが、途中で止まったときの後始末（1・3・8）と `-Resume` の完了の見方（2）を先に直す。

**減らせる量（Lean）**: 約 −150 行（16 の `wpaexporter` の道 約 40 行・17 の解釈器 3 本 約 90 行・18 の写し 約 50 行・25 の小さな消し。代わりにテストが増える）・依存の削減 0。

**読まなかった所（Not checked）**: なし（範囲の 8 ファイル 5,456 行を全部読んだ）。

## 6. 指摘の数と旗の集計

| 組 | 件数 | 番号 |
|---|---:|---|
| 必ず直す | 6 | 1〜6 |
| 直したほうがよい | 18 | 7〜24 |
| できれば | 12 | 25〜36 |
| 計 | 36 | |

| 旗 | 件数 | 番号 |
|---|---:|---|
| (a) 外への約束を変える | 14 | 2・3・4・10・14・15・16・21・22・25・26・34・35・36 |
| (a) 変えないよう守る | 3 | 7・19・20 |
| (b) 実走でしか確かめられない | 4 | 9・23・29・31 |
| (b) 一部だけ実走が要る | 4 | 1・8・24・32 |
| (c) 残す候補（⑴ 完了した spec の設計・開発者の裁定） | 6 | 3・9・14・16・25・36 |
| (c) ⑵ わざと置いた先送りの印 | 0 | （範囲に `shortcut:`・`ponytail:` は無い） |
| (c) ⑶ SSP・ukadoc の互換 | 0 | （開発の道具で、ゴーストやバルーンの振る舞いに関わらない） |

## 7. いまの自分の検査と全体テスト（要件 4.2・8.5 の前提）

- **`perf-loop.ps1 selftest` が回すもの**（`perf-loop.ps1:194-203`・`:356-403`）: `judge-perf.py`・`perf-rank.py`・`perf-compare.py`・`perf-ledger.py`・`judge-followup.py` の `--selftest`、`invoke-cpu-sample.ps1 -SelfTest`・`check-quiet.ps1 -SelfTest`・`invoke-followup-checks.ps1 -SelfTest`（中身は `judge-followup.py --selftest` を呼ぶだけ）、最後に条件文の写しの一致（作業のフォルダはワークツリーの `target\perf-loop-tmp\` の下で、終わったら消す）。
- **要るもの**: pwsh 7・Python 3.11 以上（`tomllib`）。**昇格・xperf・画面の操作は要らない**。ただし昇格した机で xperf が在ると、`invoke-cpu-sample.ps1 -SelfTest` の中の試し（`Get-ProbeResult`）が本物のカーネルの採取を 5 秒する（机の全体の ETW の状態に触る・一時のファイルは `target\perf-loop-tmp\` の下で消す）。合否は試しの結果に左右されない。
- **範囲の外の副作用**: Python の 4 本の自分の検査（`judge_followup_selftest.py:130`・`perf-rank.py:862`・`perf_compare_selftest.py:273`・`perf_ledger_selftest.py:222`）は OS の一時フォルダ（`tempfile.TemporaryDirectory`）を使う＝「一時フォルダはワークツリーの `target` の下」の決まりから外れる。これは `audit-tools-perf-judge` の持ち物なので、この spec では直さず、相手へ知らせる材料として残す。このため、このギャップ分析では `perf-loop.ps1 selftest` は回していない（副作用がワークツリーの `target` の外に出るため）。
- **この分析で回したもの**: `check-quiet.ps1 -SelfTest`（ファイルを書かない純粋な検査）＝緑（`CHECK-QUIET RESULT stage=selftest verdict=SELFTEST_OK code=0`）。`invoke-cpu-sample.ps1 -SelfTest`（この机は昇格していないので試しは `not_elevated` で何も起こさない）＝緑（`fixture_areka_frames=16`・`available=false reason=not_elevated`）。ほかに、7 と 5 を確かめるため、スクラッチパッドの小さなスクリプトで `Invoke-PwshChild` と `Start-Process` の振る舞いを見た（計測の走行は回していない・`%LOCALAPPDATA%\areka-diag\perf-loop\` は一覧を 1 度見ただけで、中のファイルは読んでも書き替えてもいない）。
- **全体テスト**: `tools/test-all.ps1` は `tools/perf` の自分の検査を回していない。触れるのは段 `encoding check`（`tools/encoding-check.ps1`）だけで、`tools/perf/*.ps1` の構文と端末へ出す文の ASCII と、`perf-loop.common.ps1` を単独で読み込んでの `Invoke-Child`／`Invoke-PwshChild` の 3 行の届き方を見る。

## 8. 要件と今の資産の対応

| 要件 | 今ある資産 | ギャップ |
|---|---|---|
| 1 洗い出し | この文書（36 件・全部読んだ） | 済み |
| 2 対象の確定 | 6 節の (c) の候補 6 件 | 要件の討議で裁定（Constraint） |
| 3 直し切る | 8 ファイル＋範囲の外の読み手（4 節） | 3・4・14・16・35・36 は範囲の外の読み手まで直しが及ぶ（Constraint：申し合わせ） |
| 4 テストで縛る | 既にある `-SelfTest` 3 本・`perf-loop.ps1 selftest` | `perf-loop` 自身と `invoke-perf-run` のテストの置き場が無い（Missing）・20 が前提。全体テストに段を足すかは未定（Unknown） |
| 5 守る決まり | `perf-loop.measure.ps1` 993 行 | 直しを足すと 1,000 行を越える＝新しいファイルが要る（Constraint）。3 の `trap`・7 の前置きは「記録の無い失敗の道を作らない」に当たる |
| 6 範囲の外・C20 | 4 節の読み手 | `audit-tools-perf-judge` の持ち物（README・`goals/`・Python）へ及ぶのは 10（README の文）・14（終了コードの写し）・16（`[sampling]`・README）・24（理由の語）・35（`time_utc`）。`encoding check`（`audit-tools-scripts`・C21）へは及ばない形で直せる（7・19・20 は守る） |
| 7 走行を回さない | — | 9・23・29・31 は本物の走行でしか確かめられない（Unknown：扱いは裁定） |
| 8 仕上げ | `perf-loop.ps1 selftest` | 8.5 は回せる（昇格・画面は要らない）。ただし Python の自分の検査が OS の一時フォルダを使う点は 7 節のとおり |

## 9. 直し方の選択肢

### 案 A: 今のファイルの中で直す（足すテストも今の `-SelfTest` の中へ）
- 必ず直す 6 件と、ほとんどの直したほうがよいは、今の関数を数行ずつ直すだけで済む。
- ✅ ファイルが増えない。❌ `perf-loop.measure.ps1` は 993 行で、2・6・9・11・13・15 の直しと `perf-loop` 側のテストを足すと 1,000 行の番人を越える。`perf-loop.ps1` は読み込むだけで走るので、測る側の関数をテストから呼べない（20）。

### 案 B: 仕事ごとに新しいファイルを足す
- 測る側の「走行の出力先を扱う関数」（完了の見方・退け方・消し方＝2・6）を新しいファイルへ移し、`perf-loop` 側の自分の検査を新しいファイルとして足す。目標定義の読み口を 1 つにする（17）。
- ✅ 1,000 行の番人の下に収まり、テストから関数だけを読み込める。❌ ファイルが 2〜3 本増え、`encoding check` が直下の `*.ps1` を全部見るので、新しいファイルも ASCII の決まりに従う必要がある。

### 案 C: 組み合わせ（推しではなく、比べる材料）
- 必ず直す 6 件と 7・8 は今のファイルの中で直す（数行ずつ）。テストを置くための分け直し（20）と、写しの寄せ（17・18・19・35・36）と、テストの置き場（21・22）だけを新しいファイルで受ける。
- ✅ 危ない所の直しが先に小さく入る。❌ 分け直しが後になると、先に入れた直しのテストを 2 度書き直しうる（20 の前にテストを書けない所がある）。

## 10. 規模とリスク・タスクの見立て

- **規模: M（3〜7 日）**。1 件ずつは数行〜数十行だが、テストの置き場を作る分け直し（20）と、写しの寄せ（17〜19）が道具のつなぎ目に触る。
- **リスク: 中**。本物の走行で確かめられない直し（9・23・29・31）があり、外への約束を変える直し（14 件）は `audit-tools-perf-judge` との申し合わせとスキル・README の追随が要る。
- **タスクの見立て（直しのまとまりごと）**:

| まとまり | 指摘 | タスクの見立て |
|---|---|---:|
| 段③の採取の始めと終わり | 1・9・29・30 | 2 |
| 走行の出力先と `-Resume` | 2・6・8・28 | 2 |
| 終わり方と終了コード・RESULT 行 | 3・4・14・26・34 | 2 |
| 子のプロセスと失敗の文 | 5・7・19 | 2 |
| 実行体の作り方と写し | 10・11・12・13・15 | 2 |
| 共通の部分の分け直し・写しの寄せ | 17・18・20・35・36・25 | 3 |
| 使えない道を消す | 16 | 1 |
| 遅い所 | 23・24・31・32 | 1〜2 |
| テストの置き場と足すテスト | 21・22・33 と各バグの回帰のテスト | 2 |
| 相対パス | 27 | （上のどれかに入れる） |
| 検証（audit のかけ直し・全体テスト・`selftest`・既定のブランチとの差分の確かめ） | — | 2〜3 |
| 計 | 36 件 | **18〜22** |

- **分け方の案**（要件 2.6・決めるのは要件の討議）:
  - 案 1: 1 本のまま（上限 20 をわずかに越えうる。タスクを「まとまり」単位で数えれば 20 に収まる見込み）。
  - 案 2: 2 本目を「できれば」の組（25〜36）と、写しの寄せ（17〜19）で切る。1 本目は必ず直す・直したほうがよいの残りとテストの置き場（約 14〜16）、2 本目は約 5〜7。ただし 17〜19 の寄せは 20 の分け直しと同じファイルに触るので、同じウェーブで並べられない（2 本目は 1 本目の着地を待つ）。

## 11. 設計へ持ち越す調べもの（Research Needed）

1. 7 の直しで、子の pwsh の想定外の失敗の表示（色の符号・地域語）がどの書き手を通って出ているか（`[Console]::SetError` の後でも化ける道）。前置きの `try`/`catch` で足りるか、`$PSStyle.OutputRendering` も要るか。`encoding check` の 3 行の確かめを崩さないこと。
2. 1 と 9 の直しで、昇格した机で `rank-run` が途中で止まったとき、`xperf -stop` で確実に止まるか（ほかの道具〔WPR など〕のカーネルの採取を止めてしまわない形）。
3. 2 の直しで、ランナーが書く終わりの節（`[結果]` の `ended_utc`）を完了の印にしてよいか。完了した走行の印が今の出力先にすでに揃っているか（`%LOCALAPPDATA%` の既にある成果には触らない約束なので、コードと検体から確かめる）。
4. 3 の直しで、落ちたときに返す終了コード（1 のままか 4 か）を、スキル・エージェントの読み方と合わせる。
5. 20 の分け直しで、`encoding check` が `perf-loop.common.ps1` を単独で読み込む形を保てるか（`StrictMode 3.0` を立てること・読み込むだけで副作用が無いこと）。
6. 23・29・31 の大きさは本物の走行でしか分からない。決定論のテストで「同じ数になる」「元が消える」までを縛り、効き目の確かめは要件 7.3 の扱いに従う。

## 12. 要件の討議で決めること（設計の判断の項目）

1. **直さずに残す指摘があるか**（要件 2.2）: 候補は 3（終了コードの体系）・9（採取と走行の順）・14（`judge-followup.py` の体系をそのまま返す形）・16（代わりの道 `wpaexporter`）・25（Python／PowerShell の版の確かめ）・36（`preflight` の項目）。どれも完了した spec `areka-P0-draw-load-parity` の設計に書かれた形で、⑴ に当たるかの裁定が要る。⑵・⑶ の候補は無い。
2. **外への約束を変える直し 14 件の扱い**（要件 6.5）: スキル・エージェントが読む形（3・4・14・15・26・34）は要件の討議で、`audit-tools-perf-judge` の持ち物に及ぶ形（10 の README・16 の `[sampling]` と README・24 の理由の語・35 の `time_utc`）はそのセッションとの申し合わせで、扱いを決める。
3. **テストの置き場**（要件 4.2）: `perf-loop` 側の新しい自分の検査（T2）と `invoke-perf-run.ps1` の `-SelfTest`（T3）を足すか。全体テストに `perf-loop.ps1 selftest` の段を足すか（足すなら、Python の自分の検査が OS の一時フォルダを使う点を `audit-tools-perf-judge` が先に直すかを含む）。
4. **本物の走行でしか確かめられない直しの扱い**（要件 7.3）: 9・23・29・31（と一部 1・8・24・32）を、決定論のテストで縛れる所までで止めるか、静かな机で 1 本だけ回すか、ウェーブの外へ回すか。
5. **分け方**（要件 2.6）: 10 節の案 1（1 本のまま）か案 2（2 本目を切る）。
6. **15 の「A と B が同じなら止める」が、要件 3.4 の「新しい検査を足さない」に当たるか**。当たるなら直し方を「`BUILD.txt` の値を報告に並べるだけ」にするかを含む。
7. **20 の分け直しの形**（案 A・B・C のどれか）: 測る側の関数を新しいファイルへ移すか、テストの入口だけを足すか。

### 要件の討議で設計へ回した事項（2026-11-14）

- 12 の 6（15 が「新しい機能」に当たるか）は、要件 3.4 に「確定した指摘の直しとして足す確かめ・止まる道は新しい機能に数えない」と書き足して解いた（議題にしない）。
- 12 の 7（20 の分け直しの形＝9 節の案 A・B・C）は設計で決める。
- 3 の想定外の失敗で返す終了コード（1 のままか、今の語彙の中の 4＝道具の不具合か）は、11 節の 4 のとおり設計でスキル・エージェントの読み方と合わせて決める。
- 9 の採取と静かさの確認の順（ランナーに採取の開始を任せるか、確認を採取の前に済ませるか）は設計で決める。
- 17 の読み口の置き場（共通のファイルを読み込むか、読み口だけの小さなファイルか）・24 の `rank` の理由の語は設計で決める（24 の語は `perf-rank.py` との申し合わせ）。
- 11 節の調べもの 1〜6 は設計の段で行う。

## 13. 範囲の外で気付いたこと（その範囲の spec の持ち物として残す）

- `audit-tools-perf-judge`: Python の 4 本の自分の検査が OS の一時フォルダを使う（7 節）。`README.md:553` の「`-BinDir` を渡さない走行（…final）」の文は、今の `final` の振る舞いと合わない（10）。`goals/draw-load-parity.toml` の `[sampling]` の注釈は、使えない `wpaexporter` を選べるように書いている（16）。
- `audit-tools-scripts`（C21）: `encoding check` は `Invoke-PwshChild` の 3 行の届き方を見るが、子が想定外の失敗で落ちたときの表示（7）は見ていない。足すかは相手の持ち物。
