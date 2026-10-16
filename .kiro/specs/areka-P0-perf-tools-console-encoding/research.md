# ギャップ分析: areka-P0-perf-tools-console-encoding

- 日付: 2026-10-16（main `1e6590d8` の上・ブランチ `claude/areka-p0-perf-console-encoding-50106b`）
- 入力: `requirements.md`（要件 1〜9）・`brief.md`・steering `tech.md`（93 行・97 行）・`structure.md`
- 調べ方: `tools/perf/*.ps1` の 7 本・`tools/encoding-check.ps1`・`tools/utf8-child.ps1`・`tools/perf/README.md`・スキル `perf-loop-iteration`・エージェント `perf-*` を読んだ。加えて、判定の規則を `tools/perf/*.ps1` に当てる静的な数えと、**窓の無い自分だけの端末（932）の中だけで閉じる数秒の実測**を数回行った（開発者の端末には触れていない。実測の前後で外側の端末は 932 のまま）。重い計測は回していない。

---

## 1. まとめ

- **子の中の書き替えは、共有の端末を本当に 65001 に変え、子が終わっても残る**（実測で確かめた）。親の .NET は端末の文字コードを一度読むと覚えておくので、同じプロセスの `[Console]::OutputEncoding` を見ても変化に気付けない。要件 1.2・6.4・7 の「前と後で同じ」の確かめ方は、OS に直接尋ねる形か、新しい子で読む形にする必要がある。
- **書き替えの代わりの有力な形が実測で効いた**: 子の PowerShell の中で `[Console]::SetOut`／`SetError` に UTF-8 の書き手を渡すと（端末には触れない・そのプロセスの中だけ）、`Write-Host`・素の出力・`Write-Output`・`[Console]::Out`・`Write-Error`・`Write-Information` が全部 UTF-8 で来て、932 に無い字（絵文字）まで 1 字違わず親に届いた。端末の文字コードは 932 のまま。
- **書き替えを消すと、今は偶然うまくいっている所が崩れる**: 子のスクリプトが孫を素の `&` で起こしている所（`invoke-followup-checks.ps1` の python と cargo・`invoke-perf-run.ps1` の pwsh・git・powercfg）は、今は共有の端末が 65001 になっているから化けずに済んでいる。撤去と同時にこれらの読み方を直す必要がある（要件 2.2・3.2）。xperf・powercfg のように端末の文字コードで書く OS の道具は、撤去の後は 932 で書いてくる。
- **端末へ出す日本語は 7 本で 395 字句（約 390 行）**。大半は端末へ出す文だが、ファイルにだけ書く日本語（`run-meta.txt` の見出し・ログの目印・Python の出力と突き合わせる正規表現）も字句の規則（規則 A）に掛かるので、ASCII の字句で日本語を書き表す方法を設計で選ぶ必要がある。
- **今の判定の規則 C は、今ある書き替えを見逃す**: `tools/perf/` を対象に入れて規則を当てると、赤は規則 A（395 件）と規則 B（1 件）だけで、`Invoke-PwshChild` の書き替えは 0 件。書き替えの文が「`pwsh -Command '…'` の形」ではなく、変数に組んで `ProcessStartInfo` の引数に渡す形だから。要件 5.3 を満たすには規則 C を広げる必要がある。

---

## 2. 今の姿

### 2.1 子を起こす部品（`tools/perf/perf-loop.common.ps1`）

| 部品 | 行 | 起こし方 | 読み方 | 端末に依るか |
|---|---|---|---|---|
| `Invoke-Child` | 188–264 | `ProcessStartInfo`・`UseShellExecute=$false`・`CreateNoWindow` なし（親と端末を分け合う） | `StandardOutputEncoding`／`StandardErrorEncoding` を UTF-8 に固定 | 依らない（正しい形） |
| `Invoke-PythonChild` | 270–279 | `Invoke-Child` に `PYTHONIOENCODING=utf-8`・`PYTHONUTF8=1` を渡す | 同上 | 依らない |
| `Invoke-PwshChild` | 287–314 | `pwsh -NoProfile -NonInteractive -Command "[Console]::OutputEncoding=…UTF8…; & '<script>' …; exit $LASTEXITCODE"` | 同上 | **子が共有の端末を書き替えて合わせている**（本 spec の直す所） |

- 失敗のときの持ち帰りの文（`実行体がありません`・`起動できません`・`pwsh が見つかりません`・`スクリプトがありません`）は日本語。要件 4.1 の「子を起こせないときに持ち帰る文」に当たる。
- 頭書（17–24 行）と `Invoke-PwshChild` の注記（281–286 行）に「子の中での `[Console]::OutputEncoding` 切り替え」「端末そのもののコードページは変えない（子の中だけ）」と書いてある（要件 8.2 で消す記述）。`perf-loop.ps1` の頭書 40–42 行にも「子の道具の出力は必ず UTF-8 として読む」の説明がある（撤去の後も正しいが、書き方の見直し候補）。

### 2.2 `Invoke-PwshChild` の呼び出し 7 か所

| 呼ぶ所 | 子 | 親が子の出力をどう使うか |
|---|---|---|
| `perf-loop.measure.ps1:267` | `invoke-perf-run.ps1`（走行・`-DryRun` 可） | 端末へそのまま写す（`-Quiet` なし）・`Code`・`LastLine` を失敗の文に埋める |
| `perf-loop.measure.ps1:293` | `check-quiet.ps1 -Stage after` | `Code`・`LastLine` |
| `perf-loop.measure.ps1:638` | `invoke-cpu-sample.ps1 -Start` | 写す・`Code`・`LastLine` |
| `perf-loop.measure.ps1:659` | `invoke-cpu-sample.ps1 -Stop` | 写す・`Code`・`LastLine` |
| `perf-loop.measure.ps1:913` | `invoke-followup-checks.ps1` | 写す・`^FOLLOWUP RESULT ` の行を正規表現で拾う（ASCII） |
| `perf-loop.ps1:375` | 自己較正の 3 本（`-SelfTest`） | `Code`・`LastLine`・赤のときだけ全行を写す |
| `perf-loop.ps1:427` | `invoke-cpu-sample.ps1 -Probe` | `^available=… reason=…` の行を拾う（ASCII） |

親が字面で読む行はどれも ASCII。日本語が要るのは「写す」と「`LastLine` を文に埋める」所だけ。

### 2.3 子のスクリプトの中で孫を起こす所

| スクリプト:行 | 孫 | 今の読み方 | 撤去の後に起きること |
|---|---|---|---|
| `invoke-followup-checks.ps1:245` | `& cargo @cargoArgs`（`nar-sample-path`） | 素の `&`＝端末の文字コードで解く | 出力は ASCII の `key=value`＋パス。パスに日本語が入ると化ける |
| `invoke-followup-checks.ps1:290` | `& $Python.Exe @argv \| ForEach-Object { Write-Host $_ }`（`judge-followup.py`） | パイプ＝端末の文字コードで解く | `judge-followup.py` は自分で UTF-8 に寄せて書く（894–899 行）→ 932 で解かれて**化ける**（今は共有の端末が 65001 なので化けていない） |
| `invoke-followup-checks.ps1:464` | `Start-Process areka.exe -RedirectStandardOutput <file>` | ファイルを `-Encoding utf8` で読む | 端末を通らない（変化なし） |
| `invoke-perf-run.ps1:316` | `& powercfg /getactivescheme 2>&1` | 素の `&` | powercfg は端末の文字コードで書く道具と見られる。撤去の後は 932 で来て 932 で解く＝今と字は同じになる見込み。ただし 437 などの端末では日本語の名前が落ちる（要確認） |
| `invoke-perf-run.ps1:329–334` | `& git rev-parse`／`status --porcelain` | 素の `&` | HEAD は ASCII。ブランチ名に日本語があると化ける。`status` は行数だけ数える |
| `invoke-perf-run.ps1:407` | `& (Get-PwshPath) -NoProfile -File check-quiet.ps1 … 2>&1`（`Invoke-CheckQuietOnce`） | 素の `&`＝孫も親も端末の文字コード | 932 では字が合うが、932 に無い字は孫の側で落ちる。要件 2.1・2.2 に合わない |
| `invoke-perf-run.ps1:745` | `Start-Process areka.exe -RedirectStandardOutput <file>` | ファイルを `-Encoding utf8` で読む（862 行） | 端末を通らない |
| `invoke-cpu-sample.ps1:205` | `Start-Process xperf … -RedirectStandardOutput <一時ファイル>` | `Get-Content -Raw`（pwsh 7 の既定＝UTF-8） | xperf は端末か ANSI の文字コードで書くと見られる。今は共有の端末が 65001 なので UTF-8 で来ている可能性。撤去の後、xperf の日本語の失敗の文が化けうる（要確認） |
| `check-quiet.ps1` | 孫なし（`Get-Counter`・`Get-Process` はプロセスの中の API） | — | — |

補足: `invoke-cpu-sample.ps1:202–203` は一時ファイルを OS の一時フォルダ（`GetTempFileName`）に作っている。要件の範囲外の項には載っていない（brief の Constraints は「一時ファイルはワークツリーの `target\` の下だけ」）。

### 2.4 端末へ出す文の量（注記を除く字句で ASCII の外の字を含むもの）

| ファイル | 行数 | 該当の字句 | 主な中身 |
|---|---|---|---|
| `check-quiet.ps1` | 662 | 41 | 自己較正の失敗の文（`$ng +=`）・`Write-Host` 1 か所 |
| `invoke-cpu-sample.ps1` | 599 | 54 | `Write-Problem`（`エラー: …` を前置）・dump の点検の文 |
| `invoke-followup-checks.ps1` | 849 | 40 | 中止・失敗の文・ログの目印の正規表現 1 か所（478 行 `apply\(ShowSurface\): 表示・マスクを更新`） |
| `invoke-perf-run.ps1` | 887 | 81 | 進みと失敗の文・`run-meta.txt` の節の見出し（`実行`・`対象`・`較正値`・`マシン`・`ソース`・`結果`）・`FAILED.txt`／`DRY-RUN.txt` の本文・`$LOG_MARKER_SMOKE_GATE` |
| `perf-loop.common.ps1` | 684 | 23 | `Stop-Run` の `[perf-loop] 失敗:`・子を起こせないときの文・cargo のラベル |
| `perf-loop.measure.ps1` | **993** | 110 | 進みと失敗の文・ラベル（`'走行'` など）・`DRY-RUN` の本文 |
| `perf-loop.ps1` | 683 | 46 | 自己較正と preflight の文・**Python の出力と突き合わせる正規表現**（313 行・315 行） |
| 計 | | **395** | |

ファイルにだけ出す日本語の主な所:
- `run-meta.txt` の節の見出し: `judge-perf.py` の `read_run_meta`（1164 行）は見出しを読むが、値は鍵の名前で引く（`meta_lookup`）。見出しは報告に出るだけ。要件 4.5（ほかの道具が読むファイルの形を変えない）との兼ね合いで、日本語のまま残すのが安全。
- `$LOG_MARKER_SMOKE_GATE`（`invoke-perf-run.ps1:174`）と `invoke-followup-checks.ps1:478` の目印は、areka のログの文言との突き合わせ。中身を変えると突き合わせが壊れる。
- `perf-loop.ps1:313/315` は `perf-ledger.py goal-text` の標準エラーと `goals/*.goal.md` の日本語を正規表現で読む（要件 4.7）。

### 2.5 判定（`tools/encoding-check.ps1`）の今の規則を `tools/perf/*.ps1` に当てた結果

`encoding-check.ps1` の関数をそのまま取り出して `tools/perf/*.ps1` に当てた（静的・ファイルは書いていない）:

| ファイル | 規則 A | 規則 B | 規則 C | 規則 D |
|---|---|---|---|---|
| `check-quiet.ps1` | 41 | 0 | 0 | 0 |
| `invoke-cpu-sample.ps1` | 54 | 0 | 0 | 0 |
| `invoke-followup-checks.ps1` | 40 | 0 | 0 | 0 |
| `invoke-perf-run.ps1` | 80 | 1（864 行） | 0 | 0 |
| `perf-loop.common.ps1` | 23 | 0 | **0** | 0 |
| `perf-loop.measure.ps1` | 110 | 0 | 0 | 0 |
| `perf-loop.ps1` | 46 | 0 | 0 | 0 |

- 構文の誤りは 0。
- **規則 C は `Invoke-PwshChild` の書き替え（310 行）を見逃す**。規則 C が子の PowerShell の文を調べるのは「コマンド名が `pwsh` で、`-Command` の次に文字列の定数が来る形」だけ。perf は文字列を変数 `$inner` に組み、配列に入れて `Invoke-Child` → `ProcessStartInfo.ArgumentList` に渡すので、どの形にも当たらない。
- 規則 B の 1 件: `invoke-perf-run.ps1:864` が `$LOG_MARKER_SMOKE_GATE` を展開文字列（端末へ出す失敗の文）に埋めている。
- 規則 D も `invoke-followup-checks.ps1:245` の `& cargo @cargoArgs`（`nar-sample-path` は変数 `$NAR_SAMPLE_ARGS` の中）を見逃す（引数の定数しか見ないため）。要件 5 は規則 D の強化を求めていないが、要件 3 の棚卸で拾う所。
- 規則 A は `$LOG_MARKER_*` への代入の右辺だけを例外にしている。この例外はログの目印の日本語の置き場として使える（`invoke-followup-checks.ps1:478` の目印も `$LOG_MARKER_*` に移せば規則の内に収まる）。
- 判定の対象の取り方は `Get-ChildItem tools -Filter *.ps1`（下の階層を見ない・345 行）。`tools/perf/fixtures`・`fixtures-loop`・`goals` に `.ps1` は無い。

### 2.6 文書の古い記述

- `tools/perf/README.md`（1140 行）: 子の出力の受け取り方と端末へ出す文の決まりを書いた所が**無い**。要件 8.1 は「書き直す」でなく「新たに書く」になる。
- steering `tech.md:93`（段 `encoding check` の「`tools/perf/` を除く」）と `tech.md:97`（決まりの対象が「`tools/` のスクリプト」）。
- スキル `perf-loop-iteration` の `SKILL.md:29`「説明文の日本語は端末の文字コードで化けることがある」。エージェント `perf-*` の 4 本には文字コードに頼る記述は無い。

### 2.7 小さな実測（窓の無い自分だけの 932 の端末の中だけで閉じた）

外側の pwsh から `Invoke-Utf8Child -OwnConsole` で窓の無い子を起こし、その子が自分の端末だけを 932 にして、そこから孫の pwsh を `ProcessStartInfo`（UTF-8 で読む・端末を分け合う）で起こした。見本は `U+7248`・`U+2460`（932 にある）と `U+1F600`（932 に無い）。

| 試したこと | 結果 |
|---|---|
| 孫が何もせずに出す（今の前置きを外しただけの形） | 標準出力も標準エラーも化けた（`�`…）。**判定の「壊れることを先に確かめる」の見本として使える**（要件 6.3） |
| 孫が今の前置き（`[Console]::OutputEncoding=UTF8`）の後に出す | 字は届く。ただし孫が終わった後、**共有の端末は 65001 のまま**（`GetConsoleOutputCP` と新しい子の読みの両方で 65001）。同じプロセスの `[Console]::OutputEncoding.CodePage` は 932 のまま（.NET が覚えた値） |
| 孫が `[Console]::SetOut`／`SetError` に UTF-8 の書き手（`OpenStandardOutput`／`OpenStandardError` の上・`AutoFlush`）を渡してから出す | `Write-Host`・素の出力・`Write-Output`・`[Console]::Out.WriteLine`・`[Console]::Error.WriteLine`・`Write-Error`・`Write-Information` の全部が 1 字違わず届いた（932 に無い絵文字も）。端末は 932 のまま |
| `Write-Error` の色の符号 | 前置きなし・今の前置き・`SetError` のどれでも同じ ANSI の符号（`ESC[31;1m`）が標準エラーに乗る＝撤去で変わらない |

pwsh の版は 7.6.7。

---

## 3. 要件ごとの対応表

| 要件 | 今ある物 | ギャップ |
|---|---|---|
| 1.1 書き替えない | `Invoke-Child`・`Invoke-PythonChild` は書き替えない | **Missing**: `Invoke-PwshChild` の前置きの撤去。孫を起こす所に書き替えは無い |
| 1.2 回し終えた後も同じ | なし | **Constraint**: 同じプロセスの `[Console]::OutputEncoding` では確かめられない（2.7）。OS に尋ねる（`GetConsoleOutputCP`）か新しい子で読む |
| 1.3 どのコードページでも同じ結果 | 親の読みは UTF-8 固定 | **Unknown**: powercfg・xperf のように端末の文字コードで書く道具の出力（`run-meta.txt` の `power_scheme`・xperf の失敗の文）が端末の文字コードで変わる |
| 2.1 子の PowerShell の字を 1 字違わず | 親の読みは正しい | **Missing**: 子に UTF-8 で書かせる手（候補: `SetOut`／`SetError` の前置き・実測で効く） |
| 2.2 孫の字を化けさせない | なし | **Missing**: 2.3 の素の `&`・パイプを置き換える（`invoke-followup-checks.ps1` 2 か所・`invoke-perf-run.ps1` 3 種） |
| 2.3 Python・cargo・git の読みを後退させない | `Invoke-Child`・`Invoke-PythonChild` | 親の側は変えなくてよい |
| 2.4 写す所は中身を変えない | `Invoke-Child` の写し | 変えなくてよい |
| 3.1–3.3 棚卸 | なし | **Missing**: 文書に残す（4 章の下書きが土台） |
| 4.1–4.3 端末へ出す文は ASCII | なし（`tools/*.ps1` は済み） | **Missing**: 395 字句の書き直し |
| 4.4 機械が読む行を変えない | `PERF-LOOP RESULT`・`SELFTEST RESULT`・`FOLLOWUP RESULT`・`CHECK-QUIET RESULT`・`selftest … code=`・`available=` などは既に ASCII | **Constraint**: 書き直しの網に掛けない |
| 4.5 ほかの道具が読むファイルの形 | `run-meta.txt`・`quiet-*.txt`（ASCII の `key=value`） | **Constraint**: `run-meta.txt` の節の見出しの日本語を残すなら、字句を ASCII で書き表す手が要る |
| 4.6 端末とファイルの両方へ出す文 | `check-quiet.ps1` の報告（ASCII の `key=value`）・`invoke-followup-checks.ps1` の `Write-Note` | `Write-Note` の中身の確認が要る |
| 4.7 Python の日本語との突き合わせ | `perf-loop.ps1:313/315` の正規表現 | **Constraint**: 突き合わせの字は日本語のまま、字句は ASCII（`` `u{…} `` など） |
| 5.1 対象に入れる | `encoding-check.ps1:345` | **Missing**: 対象の取り方に `tools/perf/*.ps1` を足す |
| 5.3 子へ渡す文の中の書き替えを赤 | 規則 C（`pwsh -Command '…'` の形だけ） | **Missing**: 今の perf の形（変数に組んで `ProcessStartInfo` に渡す）を捕まえる規則と較正の見本 |
| 5.4 端末へ出す日本語を赤 | 規則 A | 足りる（ただしファイルにだけ出す日本語も赤になる＝4.5・4.7 の書き表し方と対） |
| 5.5・5.6 今の約束と較正を保つ | 較正 27 見本・cp932 child | 規則 C を広げるとき、`encoding-check.ps1` 自身の較正の見本（書き替えを文字列で持つ）を誤って赤にしない工夫が要る |
| 6.1–6.6 932 の判定 | `encoding-check.ps1` の cp932 child（`Invoke-Utf8Child` の読みを確かめる形） | **Missing**: perf の起こし方での見本の判定・壊れることの先の確かめ。置き場は未定（8 章 決めること 8） |
| 7.1–7.4 手元の確かめ | `perf-loop.ps1 selftest`（約 1 分）・`-DryRun`・`-OutRoot` | 道具は揃っている。前後の端末の文字コードの読み方だけ 1.2 と同じ注意 |
| 8.1–8.4 文書 | 2.6 | **Missing**: README に節を足す・注記 2 か所・`tech.md` 2 行・スキル 1 行 |
| 9.4 1,000 行 | `perf-loop.measure.ps1` が 993 行 | **Constraint**: このファイルには行を足さない（部品は `perf-loop.common.ps1` へ） |
| 9.5 依存を足さない | — | どの案も標準の .NET と PowerShell だけで済む |

---

## 4. 子の出力を受け取る所の棚卸（下書き・要件 3 の土台）

設計とタスクで確定させる前の下書き。「扱い」は今の姿から見た見込み。

| スクリプト | 受け取る所 | 見込みの扱い |
|---|---|---|
| `perf-loop.common.ps1` | `Invoke-Child`（ProcessStartInfo・UTF-8） | もとから端末に依らない |
| | `Invoke-PythonChild` | もとから端末に依らない |
| | `Invoke-PwshChild` | **直す**（前置きの差し替え） |
| | `Get-NarSampleMap`（`Invoke-Child` で cargo） | もとから端末に依らない |
| `perf-loop.measure.ps1` | `Invoke-PwshChild` 5 か所・`Invoke-PythonChild` 3 か所・`Invoke-Child`（git 2・cargo 1） | 部品の直しで足りる（呼ぶ側は変えない） |
| `perf-loop.ps1` | `Invoke-PythonChild` 4 か所・`Invoke-Child`（python -c）・`Invoke-PwshChild` 2 か所 | 同上 |
| `check-quiet.ps1` | 子を起こさない | 0 件（明示する） |
| `invoke-cpu-sample.ps1` | `Invoke-NativeCommand`（xperf をファイルへ） | 端末を通らない（ファイルを介す）。ただしファイルの中身の文字コードは xperf 次第（要確認） |
| `invoke-followup-checks.ps1` | `& cargo`（245）・`& python \| Write-Host`（290） | **直す** |
| | `Start-Process areka.exe`（ファイルへ・UTF-8 で読む） | 端末を通らない |
| `invoke-perf-run.ps1` | `& pwsh check-quiet.ps1`（407） | **直す** |
| | `& git`（329–334） | **直す**か「ASCII しか来ない」の見極め（ブランチ名） |
| | `& powercfg`（316） | 決めること 4 |
| | `Start-Process areka.exe`（ファイルへ・UTF-8 で読む） | 端末を通らない |

---

## 5. 実装の案

### 案 A: 今の部品を伸ばす（推す形の土台）

1. `Invoke-PwshChild` の前置きを `[Console]::OutputEncoding=…` から「`[Console]::SetOut`／`SetError` に UTF-8 の書き手を渡す」に差し替える（起こす側＝UTF-8 で読む側が、そのプロセスの中だけで UTF-8 を頼む）。呼ぶ側の 7 か所は変えない。
2. 子のスクリプトの中の素の `&` とパイプ（2.3）を、UTF-8 で読む形に置き換える。`invoke-perf-run.ps1`・`invoke-followup-checks.ps1` は `perf-loop.common.ps1` を読み込まない単独のスクリプトなので、`tools/utf8-child.ps1` の `Invoke-Utf8Child` を読み込んで使う（形は変えない・使うだけ）か、別の手を取る（決めること 3）。pwsh の孫（`check-quiet.ps1`）には 1 と同じ前置きを付ける。
3. 端末へ出す文を ASCII に書き直す。機械が読む行は網から外す。ファイルにだけ出す日本語は ASCII の字句で書き表す（決めること 5）。
4. `encoding-check.ps1` の対象に `tools/perf/*.ps1` を足し、規則 C を今の perf の形まで広げ、較正の見本を足す。932 の判定を足す（決めること 6・8）。
5. 文書（README に節・注記 2 か所・`tech.md` 2 行・スキル 1 行）。

- 良い点: 触るのは 7 本の中身と判定と文書だけ。呼ぶ側の形・出力先の配置・終了コードが変わらない（要件 9.1）。前置きが 1 か所にまとまるので判定で見張りやすい。実測で効くと分かっている。
- 気になる点: 「プロセスの中の書き手の差し替え」が開発者の方針（端末の文字コードを変えて逃げない）の字面にどう映るか。要件 1.1 が並べる「出力の文字コード・入力の文字コード・コードページ・PowerShell が子へ渡す文字コード」のどれにも当たらないが、確認しておきたい（決めること 1）。

### 案 B: 新しい受け渡しの形を作る

- B-1 ファイル渡し: 子は端末へ ASCII だけを出し、日本語を含む中身（写す行・`LastLine`）は UTF-8 のファイルに書き、親がそれを読んで写す。端末も書き手も触らない。
  - 良い点: 方針の字面に最も沿う。
  - 気になる点: 子のスクリプト 4 本と親の写し方に手が入る。ファイルの置き場が要る（既定の出力先は `%LOCALAPPDATA%`。出力先の配置を変えない要件 9.1・一時ファイルは `target\` の下の決まりとぶつかりうる）。自己較正・`-Probe` のように出力先を持たない呼び出しもある。規模が上がる。
- B-2 端末の今の文字コードで読む: 親が `Invoke-PwshChild` の子の出力だけを、共有の端末の今の出力の文字コード（読むだけ）で解く。
  - 良い点: 子に何もさせない。
  - 気になる点: 端末の文字コードに無い字は子の側で `?` に落ちる（437 の端末なら日本語が全部落ちる）。要件 1.3・2.1 に合わない。

### 案 C: 混ぜる

- 子の PowerShell には案 A の前置き、孫の OS の道具（powercfg・xperf）には「端末の今の文字コードで読む」か「その情報をプロセスの中の API（CIM など）で取る」、Python と cargo と git は今の `Invoke-Child` と同じ UTF-8 の読み、という形。読む相手ごとに正しい読み方を選ぶ。
- 良い点: 相手の書き方に合わせた読み方になる。
- 気になる点: 読み方の種類が増えるので、棚卸の表（要件 3.3）で所ごとの理由を残す必要がある。

---

## 6. 手間と危うさ

- **手間: M（3〜7 日）** — 子を起こす所の直しは小さい（部品 1 か所＋孫 5 か所ほど）が、端末へ出す文の書き直しが 7 本・395 字句にまたがり、判定の規則 C の拡張と較正、932 の判定の追加が乗る。要件の見立て（10〜14 タスク）と合う。
- **危うさ: 中** — 主な手（`SetOut`／`SetError`）は実測で効くと分かった。残る不確かさは、(1) 端末の文字コードで書く OS の道具（powercfg・xperf）が撤去の後にどう書くか、(2) 規則 C を広げたときの誤検出（`encoding-check.ps1` 自身の較正の見本）、(3) 395 字句の書き直しで機械が読む行やファイルの中身を誤って変えること（要件 4.4・4.5）。

---

## 7. 設計への申し送り

### 推す方向

案 A を土台に、OS の道具の読み方だけ案 C の考えを足す形。理由: 実測で効く・呼ぶ側を変えない・判定で見張る所が 1 か所にまとまる。

### 決めること（要件の議論・設計で答えが要るもの）

1. **子の PowerShell に UTF-8 で書かせる手**: `SetOut`／`SetError` の前置き（案 A）か、ファイル渡し（案 B-1）か。前置きの形は要件 1.1 の「書き替え」に当たらない、という読みでよいか。
2. **前置きの置き場**: `Invoke-PwshChild` の 1 か所（起こす側が頼む）か、子のスクリプトの頭（出力が渡された先のときだけ）か。後者は開発者が `pwsh -File <子> | …` と手で回したときの読みを変えてしまう。
3. **単独のスクリプトの中の孫の起こし方**: `invoke-perf-run.ps1`・`invoke-followup-checks.ps1` で `tools/utf8-child.ps1` の `Invoke-Utf8Child` を読み込むか、`perf-loop.common.ps1` を読み込むか、小さな共通の部品を `tools/perf/` に置くか。`Invoke-Utf8Child` は標準出力を 1 本の文字列で、標準エラーを別に返すので、`2>&1` の並び（標準出力と標準エラーの混ざる順）は保てない。
4. **端末の文字コードで書く OS の道具の読み方**: `powercfg /getactivescheme`（`run-meta.txt` の `power_scheme`）と xperf の出力（`invoke-cpu-sample.ps1` の失敗の文・dump）。今は共有の端末が 65001 になっているので UTF-8 で来ている可能性があり、撤去で 932 に戻る。端末の今の文字コードで読む・CIM などで取る・ASCII の部分（GUID）だけ使う、のどれにするか。要件 1.3 の「同じ結果」に `power_scheme` の値の字が含まれるか。
5. **ファイルにだけ出す日本語・ログの目印・突き合わせの字の書き表し方**: `` `u{…} ``（`encoding-check.ps1` の前例）・隣の `*.strings.json`（`release-notes.ps1`・`roadmap-overview.ps1` の前例）・`$LOG_MARKER_*` への代入（規則 A の例外・ログの目印向き）・英語に替えてよい所（人だけが読む `FAILED.txt`・`DRY-RUN.txt` など）の使い分け。`run-meta.txt` の節の見出しを日本語のまま残すか。
6. **規則 C の穴の塞ぎ方**: 文字列の定数を全部 PowerShell として構文解析して書き替えを探す／`-Command` の次の要素になる文字列だけを見る／文字列の中の字面を正規表現で見る、のどれか。`encoding-check.ps1` 自身の較正の見本（書き替えを文字列で持つ）の扱い（例外の所を名前で限る、など）。`SetOut`／`SetError` は規則 C の対象にしない（書き替えではない）と書き分けるか。
7. **規則 B の 1 件の直し方**: `invoke-perf-run.ps1:864` の失敗の文から目印の中身を外し、名前（`LOG_MARKER_SMOKE_GATE`）で言うなど。
8. **932 の判定（要件 6）の置き場**: `encoding-check.ps1` の cp932 child に足す（全体テストのたびに回る）／`perf-loop.ps1 selftest` に足す（性能改善ループを回すときだけ）／両方。「道具と同じ起こし方」を、本物の `Invoke-PwshChild`（`perf-loop.common.ps1` を読み込む）で確かめるか、同じ前置きの定数を分け合う形にするか。
9. **前後の端末の文字コードの確かめ方**: .NET は端末の文字コードを覚えるので、同じプロセスの `[Console]::OutputEncoding` では子の書き替えに気付けない（実測）。`GetConsoleOutputCP` を呼ぶ（`Add-Type` の P/Invoke・依存は増えない）か、新しい子の pwsh で読むか。`encoding-check.ps1` の今の「own console code pages unchanged」も同じ弱さを持つ（子が書き替えない今は実害なし）。要件 5.6 の範囲で直すか、そのままにするか。
10. **スキル `perf-loop-iteration` の注記の書き方**: 撤去の後、道具が自分で出す文は ASCII になるが、写す子の中身（`judge-perf.py` の日本語の報告）は、端末の文字コードと読む側（会話へ届ける仕組み）の組み合わせで今後も化けうる。注記を消すか、「写した子の中身は化けることがある」と狭めるか。
11. **範囲の外に残る物の書き残し**: `invoke-cpu-sample.ps1` の一時ファイルが OS の一時フォルダにある件、規則 D が `& cargo @cargoArgs` を見逃す件を、この spec で直すか、記すだけにするか。
12. **`perf-loop.measure.ps1`（993 行）**: 書き直しで行を増やさない書き方（長い英文を 1 行に収める・部品は `perf-loop.common.ps1` へ）を約束として置くか。

### 調べること（設計の段で）

- powercfg と xperf が、出力を渡された先（パイプ・ファイル）へ書くときに使う文字コード（端末の出力の文字コード・OEM・ANSI のどれか）。窓の無い自分だけの端末で数秒で確かめられる。
- `SetOut`／`SetError` の前置きが、`-Command` の中で `& '<script>'` を回して `exit $LASTEXITCODE` する今の形でも、終了コードと出力の順を保つこと（`AutoFlush` と、例外で止まったときの書き残し）。
- 規則 C を広げたときの、`tools/*.ps1` と workflow の `run:` の本文への誤検出が 0 であること（要件 5.5）。

## 8. 要件の議論で決めたこと（2026-10-16・全議題を推奨案で確定）

- 決めること 1 → 子の PowerShell には `[Console]::SetOut`／`SetError` に UTF-8 の書き手を渡す形（案 A）を採る。これは端末の書き替えに当たらない（要件 1.1 に明記）。ファイル渡し（案 B-1）は採らない。
- 決めること 4 → `run-meta.txt` の `power_scheme` のようにファイルに値として残る物は、端末に依らず字まで同じにする（手段は設計）。xperf の失敗の文のように写すだけの物は、読む側で化けさせなければ足りる（要件 1.3）。
- 決めること 10 → スキルの注記は消さずに、写した子の中身だけに狭めて書き直す（要件 8.4）。
- 決めること 11 → OS の一時フォルダの 3 か所（`perf-loop.ps1:272`・`invoke-cpu-sample.ps1:202-203`・`:342`）と規則 D の穴は、この spec で直す（要件 10・要件 5.7）。
- 残りの 2・3・5・6・7・8・9（確かめ方）・12 は設計の段で決める。
