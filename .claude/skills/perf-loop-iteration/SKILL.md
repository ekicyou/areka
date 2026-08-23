---
name: perf-loop-iteration
description: 自走改善ループの 1 ターンを回す。台帳の相から続きを進め、背景コマンドを起動したところでターンを終え、最後に PERF-LOOP STATUS 行を印字する。USE FOR: /goal の毎ターン, perf-loop-iteration, 改善ループを 1 相進める, 引数 <goal-name>。DO NOT USE FOR: 単発の計測（tools/perf/perf-loop.ps1 を直に呼ぶ）, 目標定義の新設。
argument-hint: <goal-name>
allowed-tools: Bash, Read, Grep, Glob, Agent, Edit, Write
---

# perf-loop-iteration — 自走改善ループの 1 ターン

## このターンで必ず守る 4 つ

1. **開発者へ質問しない。** 裁定を仰がない。判断はこの文書の規則と台帳と道具の出力だけで決める。
2. **記憶は台帳だけ。** 会話の記憶・要約・前のターンの印象を判断に使わない。毎ターン `state` を読み直す。
3. **背景コマンドを起動したら、その場でターンを終える。** 起動した相を `WAIT_<相>` として台帳へ書き、STATUS 行を出して黙る。
4. **返答の最後の行は、道具が出した STATUS 行（または FINAL 行）そのもの。** 一字も足さず、引用符・コードブロック・語尾で囲まず、その後ろに何も書かない。

## 入力

引数は goal 名 1 つだけ（例 `draw-load-parity`）。以下 `<goal>` と書く。他は何も受け取らない。

## 使う道具（この 2 つだけ。個別の道具を直に呼ばない）

```
python tools/perf/perf-ledger.py <sub> --goal <goal> …
pwsh -NoProfile -File tools/perf/perf-loop.ps1 <sub> -Goal <goal> …
```

`perf-loop.ps1` は標準出力の末尾に必ず `PERF-LOOP RESULT <sub> code=<n> dir=<path>` を出す。
**判断はこの行の `code=` で行う**（説明文の日本語は端末の文字コードで化けることがある）。

| code | 意味 | このターンの扱い |
|---|---|---|
| 0 | 完了 | 次へ進む |
| 1 | 実走の失敗 | 同じ引数で 1 回だけやり直す。なお失敗なら計測失敗と同じ扱い |
| 2 | 静かでない | `set-phase <今の相> --goal <goal> --not-quiet-retries <n+1>`（`n` は台帳の `not_quiet_retries`。`-`／空なら 0）を書いてから、**次のターンで同じコマンドを 1 回やり直す**。`not_quiet_retries` が 1 を超えたら計測失敗（TOOLFIX へ）。周の記録の `reason` には `not_quiet` を残し、TOOLFIX に入ったら `--not-quiet-retries 0` で戻す |
| 3 | 引数・前提の不正 | 自分の呼び方の誤り。引数を直して 1 回だけやり直す。直らなければ TOOLFIX |
| 4 | 計測失敗（MEASURE_FAILED） | TOOLFIX へ |
| 5 | 能力不足（UNAVAILABLE） | **止まらない。** 段③を欠いたまま続け、理由語を台帳へ書く |

## 毎ターンの冒頭手順（例外なく、この順）

**`iteration` の意味**——台帳の `iteration` は「**今まわしている周の番号**」である（`0` は周 0＝
準備中で、まだ 1 周も始まっていない）。周 `<n>` の作業場は `iter-<n>`、記録は `## 周 <n>`、
結果の複製先は `results\iter-<n>` で、**この 3 つと `iteration` の値は必ず同じ数**になる。
だから `append` には**必ず `--iteration <n>` を渡す**（省くと道具は「次の周」と読んで `<n>+1` で
書き、周番号が 1 つずれる）。周を進めるのは RECORD の最後の 1 か所だけ。

1. `python tools/perf/perf-ledger.py state --goal <goal>` を実行する。
   - **成功したら** `phase` `iteration` `pending_run` `streak_no_gain` `previous_phase`
     `toolfix_used` `not_quiet_retries` `run` `baseline_idle_cpu_pct` `best_idle_cpu_pct` を控える。以下 `<n>` ＝
     `iteration`（今の周）。「相ごとの手順」へ。
   - **台帳が無い（exit 3・「台帳の在り処」「ありません」）** → 下の「周 0」へ。
2. 周 0（台帳が無いときだけ）:
   ```
   python tools/perf/perf-ledger.py init --goal <goal>
   pwsh -NoProfile -File tools/perf/perf-loop.ps1 preflight -Goal <goal>
   python tools/perf/perf-ledger.py state --goal <goal>
   ```
   `preflight` は台帳があれば `goal-check` を呼ぶので、走行トークンはここで作られる。
   `state` の `run` が 8 桁の数字でなければ `python tools/perf/perf-ledger.py goal-check --goal <goal>`
   を 1 回呼び、それでも埋まらなければ**止めずに**周の記録へ `reason: run-token-missing` を残す。
   ただし `/goal` に貼られた条件文のトークンと台帳のトークンが違うと終端行が判定に届かないので、
   **`run` が埋まったら STATUS 行の前に 1 行だけ「トークン `<8 桁>`。条件文と一致しているか確かめること」と書く。**
3. 相を進める。**背景コマンドを起動する相に達したら、そこでターンを終える。** それ以外の相は
   同じターンで続けて進めてよい（節約の対象は推論トークンであって計測時間ではない・設計 C2）。
4. **相の境界ごとに** `set-phase` で台帳を更新し、`status` を印字する。最後の `status` を返答の最後の行に置く。

```
python tools/perf/perf-ledger.py status --goal <goal>
```

## 相ごとの手順

相の一覧と行き先は `python tools/perf/perf-ledger.py next-phase --table` が持つ。
**行き先を自分で決めない**——必ず `next-phase --phase <相> --event <出来事> --goal <goal>` に聞く。
`WAIT_` を冠した相は「その相をやり直す」と読む（未完の相は何度やっても同じ結果になる）。

### PREFLIGHT

`preflight` の標準出力（周 0 で既に走らせていればその出力）から `capabilities=…` の行を採り、
`set-phase PREFLIGHT --goal <goal> --capabilities "<その行の中身>"` で台帳へ写す。
`function_stage=UNAVAILABLE reason=<…>` はそのまま写す（空欄にしない・推測で埋めない）。
- `code=0` → `next-phase --phase PREFLIGHT --event ok`（＝BASELINE）。**この枝だけ**、そのまま
  同じターンで BASELINE へ進み、1 本目の背景コマンドを起動してターンを終える。
- `code=4`（自己較正が赤・版不一致）→ `--event toolfix_needed`（＝TOOLFIX）。BASELINE へは進まない。

### BASELINE（3 本・各 1 ターン）

今日の日付を `yyyyMMdd` で決め（`<date>`）、**3 本を順に、1 ターン 1 本**で背景起動する。
どこまで進んだかは `pending_run` の末尾の語で分かる（`\release` → 次は dev、`\dev` → 次は rank）。

```
pwsh -NoProfile -File tools/perf/perf-loop.ps1 measure-baseline -Goal <goal> -Build release -Date <date>
pwsh -NoProfile -File tools/perf/perf-loop.ps1 measure-baseline -Goal <goal> -Build dev     -Date <date>
pwsh -NoProfile -File tools/perf/perf-loop.ps1 rank-run          -Goal <goal> -Date <date>
```

起動は `Bash` の `run_in_background: true`。起動したら直ちに

```
python tools/perf/perf-ledger.py set-phase WAIT_BASELINE --goal <goal> --pending-run "<出力先>\<release|dev|rank>"
python tools/perf/perf-ledger.py status --goal <goal>
```

を出してターンを終える。出力先の根は `%LOCALAPPDATA%\areka-diag\perf-loop\<goal>\baseline-<date>`。
3 本目（`rank-run`）の完了を回収したら、release の `verdict.txt` から定常アイドル CPU を読み
`set-phase RANK --goal <goal> --baseline <x.xx> --best <x.xx> --iteration 1` として RANK へ
（ここで周 0 が終わり、**周 1 が始まる**）。

### RANK

順位表は**その周の走行**から作る。まず `rank.txt` が既にあるかを見る。

- 周 1 は BASELINE の 3 本目が作った `baseline-<date>\rank\rank.txt` をそのまま使う。
- 周 2 以降で `<iter>\rank\rank.txt` が無ければ、7 分の順位付け走行を背景起動して**ターンを終える**。
  ```
  pwsh -NoProfile -File tools/perf/perf-loop.ps1 rank-run -Goal <goal> -Iter <n>
  python tools/perf/perf-ledger.py set-phase WAIT_RANK --goal <goal> --pending-run "<iter>\rank"
  ```

`rank.txt` が揃ったら `perf-measure` エージェントへ `mode: rank`・`run_dir: <rank の走行
ディレクトリ>` を渡す（テンプレート `templates/agent-prompts.md`。エージェント側が
`perf-loop.ps1 rank -RunDir <dir>` を回して読む）。返ってきた `## Measure` の `RANK_TOP` と
4 段の上位を控える。`STAGE3_FUNCTION` が `UNAVAILABLE` でも続ける。
→ `next-phase --phase RANK --event ok`。

### SELECT

`perf-analyze` エージェントへ順位表・台帳・設計のパスを渡す。返ってきた `## Analysis` から
`HYPOTHESIS` `CANDIDATE` `FILES` `PLAN` `TESTS` `SIZE` `SKIPPED` `HANDOFF` を控え、
`templates/entry.json` を写した `<iter>\entry.json` の `hypothesis` `candidate`
`files_changed` `skipped_candidates` へ書く（`<iter>` ＝ `…\perf-loop\<goal>\iter-<n>`）。

- **選ぶのは 1 周 1 つ**。最上位から順に、Out of scope／担当 spec が稼働中／既に試して差なし・
  悪化／信号が弱い、のいずれにも当たらない最初の 1 つ。当たったものは理由つきで
  `skipped_candidates` へ（要件 3.1）。
- `CANDIDATE: none`（＝全部除外）→ `next-phase --phase SELECT --event no_candidate` で FINAL へ
  （`reason=plateau`）。
- `SIZE: large` でも**開発者に問わない**。下の「大きい変更の 3 条件」で決める。

### IMPLEMENT

先に A 側（変更前）の実行体を退避する。ビルドを伴うので背景起動し、ターンを終える。

```
pwsh -NoProfile -File tools/perf/perf-loop.ps1 prepare-ab -Goal <goal> -Iter <n>
python tools/perf/perf-ledger.py set-phase WAIT_IMPLEMENT --goal <goal> --pending-run "<iter>\bin-A"
```

回収したターンで `perf-implement` エージェントへ `## Analysis` をそのまま渡す。

- `STATUS: DONE` → `FILES_CHANGED` を `entry.json` の `files_changed` へ写す。**`(new)` の印を
  外さずにそのまま書く**（例 `crates/a/src/b.rs, crates/a/src/b_tests.rs(new)`）。この印は
  「戻すときに消すファイル」の唯一の記憶で、次のターンの会話には残っていない（要件 1.10）。
  → `next-phase --phase IMPLEMENT --event ok`（＝TEST）。
- `STATUS: BLOCKED` → **実装をやり直させない。**
  `next-phase --phase IMPLEMENT --event implement_blocked`（＝RECORD）へ進み、`entry.json` は
  `verdict: NA`・`reason: implement_blocked: <BLOCKER>` とする。戻し方は RECORD の 2 と同じ。

### TEST

1. `perf-review` エージェントを呼ぶ。`VERDICT: REJECTED` → RECORD（verdict `TESTS_RED`・
   `reason` に `review_rejected: <FINDINGS の 1 行>`）。出来事は `review_rejected`。
2. `APPROVED` なら全テストを背景で回す。完了の目印を自分で書き足す。**必ず PowerShell で回す**
   ——素の `$?` は PowerShell では真偽値なので、`code=True` と書かれて毎周テストが赤に見える。
   終了コードは `$LASTEXITCODE` から採り、パスの区切りは `\` にそろえる。
   **`$LASTEXITCODE` は呼び出し側のシェルに食われない引用で渡すこと**——`-Command` の引数を
   二重引用符で囲むと、bash は `$LASTEXITCODE` を空にして `+ )` の構文誤りにし（ログが 1 バイトも
   出来ず、テストが走らないまま待ち続ける）、PowerShell は**自分の** `$LASTEXITCODE` を埋める
   （内側が 7 で落ちても `code=0` と記録される＝赤が緑に見える）。**外を単引用符・中を二重引用符**にする。
   ```
   pwsh -NoProfile -Command 'cargo test --workspace *> "<iter>\tests\workspace.log"; ("PERF-LOOP TESTS code=" + $LASTEXITCODE) | Add-Content "<iter>\tests\workspace.log"'
   ```
   `set-phase WAIT_TEST --goal <goal> --pending-run "<iter>\tests"` を書いてターンを終える。
3. 回収したターンで `PERF-LOOP TESTS code=` を読む。`code=0` 以外 → RECORD（`tests_red`）。
4. 緑なら見た目の追随を背景で回す。
   ```
   pwsh -NoProfile -File tools/perf/perf-loop.ps1 followup -Goal <goal> -Iter <n>
   ```
   `set-phase WAIT_TEST --goal <goal> --pending-run "<iter>\followup"` でターンを終える。
5. 回収したターンで `followup\followup.txt` の総合を読む。`PASS` → `tests_green_followup_pass`
   （＝REMEASURE）。`FAIL` → `followup_fail`。**`INCONCLUSIVE` も採用しない**（安全側・設計 C13）
   ——`followup_fail` として RECORD へ進め、`reason` に `followup_inconclusive: <検査名>` を残す。
   > 対話デスクトップのセッション以外では、カーソルとキー入力の注入が拒まれて
   > `clickthrough`／`drag`／`balloon_follow` が `INCONCLUSIVE` になる。**その環境では 1 周も
   > 採用できない。** 台帳にそう残るので、読む人がセッションを取り替えられるようにする。

### REMEASURE

```
pwsh -NoProfile -File tools/perf/perf-loop.ps1 measure-ab -Goal <goal> -Iter <n>
python tools/perf/perf-ledger.py set-phase WAIT_REMEASURE --goal <goal> --pending-run "<iter>"
```

背景起動してターンを終える。回収したら `next-phase --phase REMEASURE --event ok`（＝DECIDE）。
`code=4` → `measure_failed`（＝TOOLFIX）。

### DECIDE

`perf-measure` エージェントへ `mode: compare`・`iteration: <n>` を渡す（内部で
`perf-loop.ps1 compare` が走る）。次に `<iter>\compare.json` を**自分で読み**、
**次の 6 つの鍵だけ**を `entry.json` へ写す（綴りは同じ。他の鍵は台帳の語彙に無い）。

```
before_idle_cpu_pct  after_idle_cpu_pct  delta_pct  noise_pct  secondary  verdict
```

あわせて `runs`（4 走行のディレクトリ名）・`tests`・`followup`・`duration_min`（この周に
かかった分）を埋める。`compare.json` の `reason` は `entry.json` の `reason` へ足す。
出来事は verdict をそのまま小文字にしたもの（`adopted`／`no_diff`／`worse`／`measure_failed`）。
**DECIDE の `measure_failed` は RECORD へ行く**（TOOLFIX ではない・遷移表どおり）。

### RECORD

`files_changed` は `<パス>` と `<パス>(new)` が混ざった 1 行である。git へ渡すときは**印を外した
パス**を使い、`(new)` が付いていたものだけを「新規」として扱う。`git status --short` の `??` の
行とも突き合わせる（食い違ったら作業ツリー＝`??` の側を採る）。判断はこの 1 ターンの中で
**台帳と作業ツリーだけ**から決まる。

1. **採用（`ADOPTED`）** — 選択的に足してコミットする。
   ```
   git add <files_changed の各パス（(new) を外したもの）>
   git commit -m "perf(<goal>): iter <n> <hypothesis>"
   git rev-parse --short HEAD
   ```
   短い SHA を `entry.json` の `commit` へ。`--streak 0`。
2. **不採用（それ以外すべて）** — 元へ戻す。
   ```
   git restore --source=HEAD -- <(new) の付いていないパスを 1 つずつ>
   rm -- <(new) の付いていたパス>            # 台帳の印。git status --short の ?? と一致するはず
   git status --short                        # 何も残っていないことを確かめる
   ```
   `--streak <前の値 + 1>`。`git restore` が失敗して差分が残るなら
   **`FINAL`（`STOPPED reason=safety`）**へ行く（要件 1.4 ⒞）。
3. 台帳へ追記し、判定の出力を spec へ複製する。**`--iteration <n>` を必ず渡す**（`<n>` ＝ 今の
   `state.iteration`。省くと `<n>+1` で書かれて周番号が 1 つずれる）。
   ```
   python tools/perf/perf-ledger.py append --goal <goal> --from-json "<iter>\entry.json" --iteration <n>
   ```
   `<spec_dir>\results\iter-<n>\` を作り、`compare.txt`・`compare.json`・`rank.txt`・
   `followup.txt` を複製する（要件 5.6・7.6）。
4. 次の行き先を `next-phase --phase RECORD --event <…>` で決める。
   - `streak_no_gain + 1 >= [stop].max_no_gain_streak` → `plateau`
   - `<n> >= [stop].max_iterations` → `iteration_cap`
   - 採用して `after_idle_cpu_pct < [target].idle_cpu_release_max_pct` → `goal_met_candidate`
   - それ以外 → `ok`（＝RANK）。**ここが周を進める唯一の場所**——
     `set-phase RANK --goal <goal> --iteration <n+1>` とする。
5. `--best` は `after_idle_cpu_pct` が今までの最良より小さいときだけ更新する。

### TOOLFIX（1 周につき 1 回だけ）

入るときに戻り先と回数を台帳へ書く。**ここを書かないと次のターンが戻り先を見失う。**

```
python tools/perf/perf-ledger.py set-phase TOOLFIX --goal <goal> --previous-phase <計測が失敗した相> --toolfix-used <toolfix_used + 1> --not-quiet-retries 0
```

`toolfix_used`（更新後）が `[stop].toolfix_retry` を超えていたら、直さずに
`next-phase --phase TOOLFIX --event toolfix_fail` → FINAL（`STOPPED reason=measure_failed`）。

超えていなければ `perf-implement` エージェントへ「失敗したサブコマンドと終了コードと
標準出力の末尾 20 行」を渡して道具を直させ（実行体のコードは触らせない）、

```
pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest -Goal <goal>
```

を回す。`code=0` → `next-phase --phase TOOLFIX --event toolfix_ok --goal <goal>`
（`--previous` は省く＝台帳の `previous_phase` を読む）→ 出た相を `set-phase` して続ける。
`code` が 0 以外 → `toolfix_fail` → FINAL。

### FINAL

1. 25 分 × 2 本を 1 ターン 1 本で背景起動する（`pending_run` の末尾で進み具合が分かる）。
   ```
   pwsh -NoProfile -File tools/perf/perf-loop.ps1 final -Goal <goal> -Build release -Date <date>
   pwsh -NoProfile -File tools/perf/perf-loop.ps1 final -Goal <goal> -Build dev -Date <date> -Resume
   ```
   起動ごとに `set-phase WAIT_FINAL --goal <goal> --pending-run "<final-<date> の下>\<build>"`。
2. 2 本とも回収したら対比表を作る。
   ```
   python tools/perf/perf-ledger.py summary --goal <goal>
   ```
3. 両ビルドの `verdict.txt` を読む。
   - **判定式⑴〜⑷b が全て PASS** →
     ```
     python tools/perf/perf-ledger.py final --goal <goal> --outcome GOAL_MET --idle-cpu <x.xx>
     ```
   - **PASS でない**:
     - `goal_met_candidate` で来ていて、かつ頭打ちにも周数上限にも達していない → **止まらない**。
       結果を台帳へ残し `set-phase RANK --goal <goal> --iteration <n+1>` で周へ戻り、STATUS 行で
       終える（要件 1.4 に無い理由でループを止めないため。ここだけは `next-phase` に行がない）。
     - それ以外 → 未達を登記して止める。
       ```
       python tools/perf/perf-ledger.py final --goal <goal> --outcome STOPPED --reason <plateau|safety|measure_failed|iteration_cap> --top-remaining <stage:item:share>
       ```
       あわせて `<spec_dir>\requirements.md` の `## 改訂欄` へ 1 項を `Edit` で足す（要件 5.7）:
       日付・**未達の判定式**（⑵／⑷a など）・到達値と目標値・**残る最大項**（順位表の
       最上位＝`top_remaining` と同じもの）・**引受先の spec 名**。引受先は
       `.kiro/specs/` を実際に見て、担当ファイル集合が当たる**生存している** spec を挙げる
       （完了済みは引き受けられない）。無ければ「引受先なし・新規 spec が要る」と書く。
4. FINAL 行を**返答の最後の行**として出す。これで `/goal` が止まる。

### `WAIT_<相>` のとき（背景コマンドの完了待ち）

1. 背景タスクの出力の**末尾**を読む。`PERF-LOOP RESULT <sub> code=<n> dir=<path>`
   （`cargo test` なら `PERF-LOOP TESTS code=<n>`）が出ていれば**完了**。
2. 出ていなければ `pending_run` の中に、その相が書くはずの成果物があるかを見る。

   | 起動したもの | 見る成果物 | 無いとき |
   |---|---|---|
   | `measure-baseline`／`final` | `verdict.txt` | 進行中 |
   | `rank-run` | `rank.txt` | 進行中 |
   | `measure-ab` | `compare.json` | 進行中 |
   | `prepare-ab` | `bin-A\BUILD.txt` | 進行中 |
   | `followup` | `followup.txt` | 進行中 |
   | `cargo test` | `tests\workspace.log` の中の `PERF-LOOP TESTS code=` の行 | 下の※ |

   ※ **`cargo test` だけは「ファイルが無い」と「ファイルはあるが `code=` の行が無い」を分ける。**
   ログが**在るのに** `code=` の行が無い（0 バイトを含む）なら、それは進行中ではなく
   **起動そのものの失敗**である（引用の崩れ・pwsh の構文誤り）。待っても永久に終わらないので、
   `tests_red` として RECORD へ進め、`entry.json` の `tests` へ `launch_failed: <ログの末尾 1 行>`
   と残す。ログが**無い**ときだけ進行中と読む。

   進行中なら台帳を触らず `python tools/perf/perf-ledger.py status --goal <goal>` を出して
   ターンを終える（待つ）。
   **例外——背景タスクがこのセッションに存在しない場合。** 起動した背景コマンドの完了通知が
   このセッションに無い（別のセッションで再開した・要約で失われた・前回の起動が `code=2`
   で終わっていた）のに成果物も無いときは、進行中ではなく**起動し直す**。`code=2`
   で終わっていた場合は、起動し直す前に上の `code=2` の規則（`not_quiet_retries` を
   1 つ増やし、1 を超えていれば TOOLFIX）を先に当てる。同じコマンドに
   `-Resume` を付けて背景起動する（完了済みの成果物は再利用され、無ければ走る）。`cargo test`
   だけは `-Resume` が無いので同じコマンドをもう一度起動する。
3. 完了していれば、同じコマンドに `-Resume` を付けて 1 回だけ呼び、成果物を作り直さずに
   RESULT 行を取り直してから次の相へ進む。**成果物が無いのに `-Resume` で呼び直さない**
   （25 分の計測がもう 1 本走る）。**`cargo test` にこの手順は当たらない**——`-Resume` を持たない
   唯一の背景コマンドで、やり直すときは同じコマンドをもう一度起動する。
4. check-in（様子うかがい）が届いたターンも手順は同じ。進行中なら「待つ」と答え、
   最後に STATUS 行（`phase` は `WAIT_…` のまま）を置く。

## 共通の規則

### 大きい変更の 3 条件（要件 3.6）

規模が大きくても開発者に問わない。⒜ 全テスト緑 ⒝ 見た目の追随が全て PASS
⒞ 交互比較の差が計測のばらつきを超える（`perf-compare.py` が `ADOPTED` と言う）——
**3 つ揃えば採用、1 つでも欠ければ戻す**。規模とリスクの見立て（`SIZE`・`RISK`）は
`entry.json` の `reason` に残す。

### 候補を選ばない理由（要件 3.1・8.5）

`skipped_candidates` に語をそろえて書く: `out_of_scope`／`spec_active:<spec 名>`／
`already_tried`／`no_signal`。`already_tried` は台帳の過去の周の `candidate` と `verdict`
（`NO_DIFF`・`WORSE`）を読んで判断する（`perf-ledger.py entries --goal <goal>`）。
稼働していない spec の担当ファイルを触ったら、その spec の `brief.md` へ申し送る旨を
`entry.json` の `reason` へ `handoff:<spec 名>:<ファイル>` として残す。

### git の規則（このスキルだけが git を動かす）

- 採用は **選択的な `git add <ファイル>` ＋ 1 周 1 コミット**。
- 不採用は `git restore --source=HEAD -- <ファイル>` と新規ファイルの削除。
- **`git add -A` と `git reset --hard` は使わない。** ブランチを作らない・切り替えない・押さない。
- サブエージェントに git の破壊的操作をさせない（定義で禁じてある。破っていたら
  `perf-review` が `FINDINGS` に挙げる）。
- `Cargo.toml` は触らない・触らせない（記号付与はビルド時の環境変数）。

### サブエージェントの呼び方

`templates/agent-prompts.md` の文をそのまま埋めて `Agent` ツールへ渡す。返答は所定の見出しと
鍵の行だけを読む。**最初の 1 行 `[agent-model] <名前>` に `opus` が含まれなければ**、その周の
`reason` へ `agent-model-warning:<エージェント名>=<名前>` を足して**続行する**（止めない）。
自分のモデルが Fable 系、またはモデル名の行が見つからないときは `model: "opus"` も渡す。

### 印字の規則

- 相の境界ごとに `status` を印字する（途中の行は判定に使われないが、切れたときの手がかりになる）。
- **返答の最後の行は最後の `status`（FINAL のときは `final`）の出力そのもの。**
- 文書中の `PERF-LOOP …` の見本はすべて山括弧つきで、実出力とは一致しない。判定は実物の
  トークン込みの字面でしか成立しない。**見本を返答へ貼らない。**

## 観測可能な完了状態

台帳の無い状態でこのスキルを 1 回呼ぶと、PREFLIGHT 相が回り、台帳に状態ブロックができ、
返答の最後の行に `PERF-LOOP STATUS iter=0 phase=…` の 1 行が出ている。
