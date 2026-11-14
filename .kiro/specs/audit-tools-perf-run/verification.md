# 検証の記録（audit-tools-perf-run）

## 8.1 版の上げと自分の検査

- 振る舞いの変わる 5 本の `SCRIPT_VERSION` を小さい版の数で 1 つ上げた（申し合わせ済み B）: `perf-loop.ps1` 1.0.0 → 1.1.0・`invoke-perf-run.ps1` 1.1.0 → 1.2.0・`invoke-cpu-sample.ps1` 1.0.0 → 1.1.0・`check-quiet.ps1` 1.0.1 → 1.1.0・`invoke-followup-checks.ps1` 0.1.1 → 0.2.0。
- `pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest`: 毎回のかけ直しの直しの後に回し、最後（7 回目の直しの後）も `SELFTEST RESULT ok=12 ng=0`・`PERF-LOOP RESULT selftest code=0`。内訳の数は T2 1 本目 `ok=23`（着手の 20）・2 本目 `ok=27`（着手の 16）・T3 `ok=7`（着手の 6）（検査を足した分だけ増えた。道具の数 12 は同じ）。
- `pwsh -NoProfile -File tools/encoding-check.ps1`: `encoding check: all passed`。
- 1,000 行の番人: 長い順に `invoke-followup-checks.ps1` 967 行・`invoke-perf-run.ps1` 917 行・`perf-loop.measure.ps1` 902 行で、どのファイルも 1,000 行の下（16 本で 8,172 行）。
- 範囲に 1 本足した: `tools/perf/perf-loop.selftest-base.ps1`（T2 の 2 本が写しで持っていた骨組み＝偽物の記録・終了の口・回し方を 1 か所へ寄せた。1 回目の指摘 B8）。

## 8.1 同じ範囲への audit のかけ直し

- 範囲: `tools/perf/` 直下の `*.ps1` の全部（16 本＝着手の 8 本・設計で足した 7 本・このタスクで足した `perf-loop.selftest-base.ps1`）。`*.psm1` は 0 本。
- やり方: 回ごとに 4 組（組 A＝入口と共通部品・実行体・出力先の 6 本／組 B＝測る側と T2 の 4 本／組 C＝ランナー・T3・静かさの確かめの 4 本／組 D＝段③の採取・追随の検査の 2 本）へ別々の読み係を当て、`/ponytail:ponytail-audit`（5.3.3）の見方を、件数の上限なし・全部の行を読む作法（要件 1）でかけた。どの回も「読まなかった所」は 0。指摘は全部この中で直し、直しごとに自分の検査を回して緑を確かめてから次の回へ進んだ。
- 数え方: 組をまたいで同じ指摘は 1 件に数えた（例: 1 回目の申し送り 3.1 は 3 組が挙げた）。前の回に裁定した・設計が受け入れた形は数えない（要件 8.2）。

### 回ごとの件数

| 回 | 日付 | 組 A | 組 B | 組 C | 組 D | 重複を除いた計 | 必ず直す | うち振る舞いに効く |
|---|---|---|---|---|---|---|---|---|
| 1 | 2026-11-15 | 14 | 8 | 5 | 10 | 31 | 1 | 0（D1＝関門の速さ。測る値は同じ） |
| 2 | 2026-11-15 | 4 | 6 | 3 | 3 | 15 | 0 | 0 |
| 3 | 2026-11-15 | 4 | 2 | 2 | 2 | 10＋隣の spec からの持ち込み 1 | 1 | 1（D1 balloon_follow の StrictMode） |
| 4 | 2026-11-15 | 2 | 3 | 3 | 2 | 10 | 0 | 0 |
| 5 | 2026-11-15 | 3 | 3 | 2 | 5 | 13 | 1 | 1（A1 -Ledger の打ち間違いで preflight が緑） |
| 6 | 2026-11-15 | 5 | 3 | 2 | 2 | 12 | 1 | 1（A1 A と B でヘルパの有無が違う比較） |
| 7 | 2026-11-15 | 4 | 5 | 2 | 2 | 13 | 1 | 1（A1 -DryRun が揃った bin-A を書き替える） |

- 回ごとの指摘の中身は、1 回目が想定外の失敗の道・終了コードの写し・関門の速さ、2〜4 回目が 1 回目の直しの取り残し（同じ形の別の所）と試しの抜け、5〜7 回目が打ち間違いの扱い・端の場合（画面の端・PID の使い回し・空の配列）・試しの抜けへ移った。
- **5 回目で打ち切れなかった**: 5 回目の指摘に「必ず直す」かつ道具の振る舞い（終了コード）に関わるもの（-Ledger）が 1 件あったので、要件 8.7 の打ち切りの決まりに当たらず、6 回目・7 回目をかけた。6・7 回目にも「必ず直す」が 1 件ずつ出た（どちらも直した）。**7 回目の指摘は全部直したが、7 回目の後のかけ直し（8 回目）はまだかけていない**（進行役の判断を仰ぐ。下の「残りの扱い」）。

### 直した指摘（回ごと・指摘 → 直し → 試し）

1 回目（31 件・直し 29・指摘に当たらない 1・裁定待ち 1）:
- 段③の採取の関門（dump の数え直し）が 1 行 400 マイクロ秒で、7 分の採取の dump で何時間もかかる（必ず直す）→ 数える本体を .NET 標準だけの C#（Add-Type）へ移し約 5 マイクロ秒／行（48,021 行で 20.8 秒 → 0.2 秒・数は同じ）→ T1 の既存の厳密な数の検査が緑のまま。
- 記号の総称型のカンマで数える列がずれる → 列が多すぎる行だけ括りの外のカンマで割り直す → T1「symbol commas」。列名行に Image!Function が無いと記号の無いせいにする → 列名行の不備と言う → T1。
- 子の道具 4 本の trap が共通部品より前の失敗で元の原因を失い RESULT 行の無い 1 になる（申し送り 3.1）→ trap に共通部品より前の受け皿を置き、元の原因の 1 行と決まった終了コード・RESULT 行で終える → T2「trap-before-base」。
- `-Stop` の `xperf -d` が落ちても採取を止めない（申し送り 6.4）→ -d が落ちたら `xperf -stop` を打ってから読む・perf-loop は 0・5 のほかの `-Stop` の終わりで手で止める警告を出す → T1「stop-merge-fails」・T2「stop-failed」。
- 走行の後の静かさの確かめが落ちた子（1・-1）を「静かでない」と読み 3 分待って 2 → 2 のほかは 4 で止める → T2「quiet-after-codes」。
- followup の実行体の選び（申し送り 7.2）に試しが無い・-DryRun で写しただけの bin-B を使う → 揃った一式だけを使う → T2「followup-exe-choice」。
- -DryRun の写しの bin-A を本番が揃った一式と見る・作り直しで写さなかったヘルパと PDB が前の回のまま残る・helper=missing の一式へ共有のヘルパを写す・走行前のビルドがヘルパを作り直さない → それぞれ直した → T2「bin-set-stale」「worktree-build-helper」。
- 日付をまたぐ final で走行と結果の写しが別の日付 → 1 回の呼び出しの中で日付を 1 度だけ決める → T2「loopdir-vocab」の日付の項。
- 入口の読めない `-Iter`・知らない引数が RESULT 行の無い 1 → 字で受けて入口で確かめ、受け皿の引数で 3 と RESULT 行 → T2「bad-params」。
- 子が先に終わると孫（ヘルパ）を止めない → 子孫を必ず集めて 1 つずつ止める → T2「kill-after-parent-exit」。
- 可否の 1 行が読めない訳を黙って落とす → 1 行の警告 → T2「probe-parse」。
- python の版・git の HEAD に標準エラーの警告が混ざる → 標準出力の最後の行だけ → （6 回目に T2「python-read」）。
- 目標定義の [quiet] の読めない値が黙って既定へ落ちる（ランナー・静かさの確かめ）→ 3 で止める → T1・T3「quiet-retry-settings」。静かさの確かめの答えから次の手を決める所に試しが無い → 純関数 `Get-QuietAttemptStep` → T3「quiet-attempt-step」。
- T3・T2 の後始末が使い回された PID の別のプロセスを止め得る → 握った者の Handle を開いて持ち帰る。
- 写しと要らないコード: T2 の 2 本の骨組みの写し（`perf-loop.selftest-base.ps1` へ）・`Get-MeasureMovedDir`（`Get-MeasureLineWord` へ）・rank の出力先の 2 つの写し（`Get-MeasureRankDir`）・使われない目標定義の値 3 つ・`Get-LoopDateStamp -Date`・死んだ枝・同じ在り処の 2 度の探し・`Invoke-Child` の戻り値の手組み・`$PERF_LOOP_EMPTY` の二重の字面。
- そのほか: カウンタの失敗の回数の文（3 → 4 回）・check-in の文（正の整数）・採取を始められない訳（`xperf -stop` の手引き）・followup の前の回の判定を残さない・Store の別名の python・`dryrun-clear` が `-DryRun` の印を持ち越す（申し送り 4.1）。
- 指摘に当たらない 1 件: 「measure-ab -Resume が B を作り直して 2 つの B を混ぜる」は、揃った bin-B を -Resume では作り直さない（`Invoke-MeasureBuildSide` が早く戻る）ので起きない。
- 裁定待ち 1 件: 「`invoke-followup-checks.ps1` が `-BinDir` の実行体・検体の窓口が無いとき 3 でなく 1」は、頭書が「起動できない」を 1 と書く外への約束（終了コードの意味）なので、この中では直さず進行役の裁定に回した（下の「残りの扱い」）。

2 回目（15 件・全部直した）: Store から入れた python を捨てない（最後に回す）・followup が検査の子の隣へヘルパを写す・根が終わった後に PID の使い回しの別物の子を止めない・`-Iter` の束ねの残りの穴を `shortcut:` で明記・T3 の ready のファイルの書きかけを掴まない・空の `heavy_process_names = []` を「名前を置かない」と読む・[quiet] の鍵の一覧を 1 か所に・builds_final の語を 1 本目の前に全部確かめる・既に在る出力先をビルドの前に 3 で止める・SHIORI の関門と判定器の写しの試し・followup の前の -DryRun の印を消す・注釈の位置・使われない `HeaderFound`・括りの閉じない記号の試し・followup の -SelfTest の最後の行が自分の検査の赤を言う。試し: T2「exe-after-build」「final-bindir」「shiori-gate」「followup-exe-choice」、T1 の symbol commas。

3 回目（10 件＋隣の spec `audit-tools-perf-judge` からの持ち込み 4 件のうち直すもの 1 件・全部直した）: balloon_follow の StrictMode で 1 つだけの要素の `.Count` が落ちる（必ず直す）→ 純関数 `Get-BalloonFollowSources` → T1「balloon-follow-sources」・`selftest -GoalFile` の無いファイルで既定の目標を緑にする → 3・TOML の見出しの後ろの注釈で鍵が前の節へ入る・`Assert-MeasureAbDistinct` の空の添字・cargo の無い訳の文・`iteration_build` の大小・measure-ab の出力先の確かめの試し・`-HeavyProcessNames a,b` が割れない・静かさの確かめの 4 の訳の文・二重の return・ランナーの使い方の例に `-BalloonRoot` が無い（持ち込み）。持ち込みのうち DRAG_DX_PX の注釈はこの枝ではまだ正しい・.py を行番号で指す注釈は 0 件・スキルの `--iteration` の文は下の「範囲の外」。

4 回目（10 件・全部直した）: 段③の関門が記号の解けない `areka.exe!0x…` を数える・カーソルが画面の右下の角に居ると注入を不可と言う・静かさの確かめの古い報告を証拠と言う・NaN の閾値が素通り・ランナーの頭書の 4・走行前のヘルパの作り直しが落ちたとき古いヘルパで測る・揃った一式に -DryRun の印が残る・済んだ followup を -Resume 無しで黙って消す・builds_final の同じ語の 2 度・結果の写しの日付を走行の出力先から。試し: T1 の unresolved、T2「worktree-build-helper」「followup-resume-first」「final-bindir」「final-results-date」。

5 回目（13 件・直し 12・設計が受け入れた形 1）: `-Ledger` の打ち間違い（必ず直す）→ 3 → T2「bad-params」・`$script:IterArg` を整数に・子の作業の場所を PSDrive の名でなく OS の道で・sampling.txt の使わない値・結果の写しの日付と followup -DryRun の試し・走行が先に終わった失敗の道でも子孫を止める・`-Stop` の打ち直しで前のまとめを読む・必須の引数の欠けを `shortcut:` で明記（2 本）・クリック透過の窓が先に閉じたら 1・前の回の followup.txt を残さない・`-Stop` の読み先の試し。受け入れ済み: 理由の行に入る OS の例外の文が地域語になり得る点（design の「想定外の失敗の終わり方」の ASCII の冠＋例外の文）。

6 回目（12 件・全部直した）: measure-ab で A と B の一方だけヘルパが無い（必ず直す）→ 走る前に 4 → T1「ab-same-exe」・目標定義の在るのに読めない整数・配列・空の配列を 3 → T2「goal-values-strict」・作り直しの途中で止まった一式を揃ったと見ない・cargo の出力を会話へ流さない（落ちたときだけ最後の 8 行）・python の在り処と版の試し → T2「python-read」（`Get-PythonVersionText` を共通部品へ移した）・final の 2 本目の出力先を 1 本目の前に・measure-ab -Resume の試し → T2「ab-resume」・採取を始められない訳の行 → T3・静かさの確かめの頭書・記号の置き場が作れない → 3・followup の頭書の RESULT 行は実走のとき。

7 回目（13 件・全部直した）: -DryRun の prepare-ab が揃った bin-A を書き替えて A の実行体を失う（必ず直す）→ 揃った一式は -DryRun でも書き替えない → T2「dryrun-keeps-real-set」・`checkin_minutes` の読めない値を 3・`Assert-MeasureAbDistinct` の写しのループを 1 つに・ヘルパの元の道の 3 つの写しを `Get-MeasureHelperSource` に・T2 を本番と同じ `$ErrorActionPreference = 'Stop'` で回す・`-Checks` の渡し方の試し → T2「followup-checks-arg」・`-Stop` の 3 を 3 のまま・関門を通ったらまとめたトレースを消す・受け入れの判定の試し → T2「compare-core」・届かない死んだ確かめ・T3 の -DryRun -SampleEtl の試しが理由の語まで見る・クリック透過の点が画面の外で止められたら判定不能・balloon_follow の動かす操作が無いときの理由の語 `no_required_mover`。

### 赤の確かめ（直しの前の形で落ちることを見た）

- trap-before-base: HEAD の `check-quiet.ps1`・`invoke-cpu-sample.ps1` と壊した共通部品で、RESULT 行の無い 1（元の原因は「Format-UnexpectedError が無い」「$EXIT_RUN_FAILED が無い」）。直した後は元の原因の 1 行と `CHECK-QUIET RESULT stage=after verdict=MEASURE_FAILED code=4 file=-`。
- 段③の関門: HEAD の数え方で総称型のカンマの行が 0 フレーム（直した後 1）、48,021 行に 20.8 秒（直した後 0.2 秒）。
- HEAD の `perf-loop.measure.ps1`・`perf-loop.build.ps1`・`perf-tool-base.ps1`・`perf-loop.ps1`・`perf-loop.common.ps1` を写しの部屋で戻して T2 を回し、`quiet-after-codes`・`stop-failed`・`followup-exe-choice`・`bin-set-stale`・`worktree-build-helper`・`kill-after-parent-exit`・`loopdir-vocab`（日付）・`bad-params`・`probe-parse`・`exe-after-build`・`final-bindir`・`followup-retry-on-1` が赤になることを見た。

## 残す指摘・回す指摘

- **残す指摘: 0 件**（要件 2.2 の 3 種類に当たる指摘は出なかった）。ただし 1 回目の 1 件（followup の実行体が無いときの終了コード 1）は外への約束に当たるため、この中では直さず**進行役の裁定待ち**にした。
- **回す指摘: 0 件**（後の spec へ回した指摘は無い）。

## 消す前の grep の件数（要件 3.3）

ツリー全体（`target`・`vendors`・完了した spec の文書を除く・`*.ps1`・`*.py`・`*.md`・`*.toml`・`*.json`）で数えた。

- 1.3（写しの消し・4 本の外）: `Get-PwshPath`・`Get-NarSampleMap`・`NAR_SAMPLE_ARGS`・`NarSampleMaps` は 0 件。`Write-Info`・`Write-Problem`・`Get-UtcStamp`・`Get-PythonCommand`・`Get-NarSamplePath` は共通部品を読む `perf-loop*.ps1` だけ。4 本の `Invoke-Utf8Child` の参照 9 → 0。
- 6.2（`wpaexporter` の道）: HEAD で `invoke-cpu-sample.ps1` 22・`perf-loop.common.ps1` 4・`perf-loop.ps1` 5・`goals` 1 → 範囲の中 0（`invoke-cpu-sample.ps1` の `XPERF_DEFAULT_PATH`・`Test-Elevated` の 5 件は段③の採取の唯一の持ち場として残す）。範囲の外に残る `[sampling] backend`（goals の toml・`perf_ledger_goal.py` の必須の鍵・`fixtures-loop` の 5 本）は `audit-tools-perf-judge` の受け持ち。
- 8.1（このタスクで消した名前）: `Get-MeasureMovedDir` 2（どちらも `perf-loop.measure.ps1`）→ 0。`Get-GoalConfig` の値 `LedgerRel`・`JudgeScript`・`IdleCpuMaxPct` は `perf-loop.common.ps1` の中だけ（読む所 0）→ 0（目標定義の鍵 `idle_cpu_release_max_pct` そのものは Python とスキルが読むので残る）。`Get-LoopDateStamp -Date` を渡す呼び出し 0。`HeaderFound` を読む所は `invoke-cpu-sample.ps1` の外に 0。`Get-MeasureSamplingNote` 1（`Invoke-MeasureRank`）→ `Get-MeasureSamplingReason` へ替えて 0。`Split-DumpFields` はこのタスクの中で足して消した（HEAD に 0）。

## 範囲の外

- スキル `perf-loop-iteration` の `SKILL.md` 45〜46 行の「`--iteration` を省くと n+1」は、`audit-tools-perf-judge` の枝では 3 で止まる形に変わる（相手の持ち物・まだ `main` に無い）。この spec の `.claude` の差分は S1・S2 だけなので触らない。
- `judge-perf.py` 0.5.0 の最後の行 `JUDGE-PERF RESULT code=N`（相手の枝・未着地）は合わせていない。着地すれば `perf-loop.measure.ps1` の止まりの文に埋まる `LastLine` がその行になるだけで、終了コードの写しは変わらない。

## 残りの扱い（進行役へ）

- 7 回目の指摘は全部直し、自分の検査と `encoding check` は緑。8 回目のかけ直しはかけていない。要件 8.7 の打ち切りは 5 回目の形で、5 回目に「必ず直す」かつ振る舞いに関わる指摘があったため当たらなかった。続けて 8 回目をかけるか、7 回目で区切るかの判断を仰ぐ。
- 裁定待ち 1 件（followup の実行体が無いときの終了コード 1 か 3 か）。
