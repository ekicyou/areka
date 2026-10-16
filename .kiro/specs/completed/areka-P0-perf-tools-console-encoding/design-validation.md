# 設計の検証: areka-P0-perf-tools-console-encoding

- 日付: 2026-10-16
- 対象: `design.md`（確定版）・`requirements.md`・`research.md`・steering（`tech.md` ほか）
- 突き合わせたコード: `tools/perf/*.ps1` の 7 本・`tools/encoding-check.ps1`・`tools/utf8-child.ps1`・`tools/perf/judge-followup.py`・`tools/perf/judge-perf.py`・`tools/perf/perf-rank.py`・スキル `perf-loop-iteration`
- 回し方: 読むだけ（構文解析で数えた・端末の文字コードには触れていない・一時ファイルは作っていない）

## 設計の検証の要約

設計は、書く側（子の PowerShell）にそのプロセスの中だけの UTF-8 の書き手を頼む形で根から直し、前置きの在り処を 1 か所（`Get-PwshChildArguments`）に絞り、呼ぶ側 7 か所と `Invoke-Child` の契約を変えない。設計に書かれた行番号・関数名・行数はコードと合っており、名前のぶつかりも無い。直すべき点は 3 つあるが、どれも設計の文を少し足せば済み、作りの向きを変えるものではない。

## コードとの突き合わせ（確かめた事実）

| 確かめたこと | 結果 |
|---|---|
| 行番号（`perf-loop.common.ps1` 17〜25・188・270・281〜311、`perf-loop.ps1` 246・272・313・315、`invoke-followup-checks.ps1` 245・290・478、`invoke-perf-run.ps1` 316・329〜334・407・864、`invoke-cpu-sample.ps1` 202〜205・342、`judge-followup.py` 894〜899、`tech.md` 93・97、スキル 29 行） | すべて設計の記述どおり |
| 行数 | `perf-loop.measure.ps1` 993・`invoke-perf-run.ps1` 887・`invoke-followup-checks.ps1` 849・ほか 700 未満。上限 1,000 の内 |
| 名前のぶつかり | `encoding-check.ps1` の関数（`Show-Ascii`・`Get-ConsoleRewrites`・`Set-OwnConsoleCp932` ほか）と `perf-loop.common.ps1` の関数・頭の変数（`$PERF_LOOP_*`・`$script:NarSampleMaps`）は重ならない。`invoke-perf-run.ps1`・`invoke-followup-checks.ps1` と `Invoke-Utf8Child`・`Get-PwshChildArguments` も重ならない。`perf-loop.common.ps1` の `Set-StrictMode -Version 3.0` は `encoding-check.ps1` と同じ |
| 子と孫の出力を受け取る所の棚卸 | 構文解析で数えた素の呼び出し・`&`・`Start-Process` は、設計の表の行と 1 対 1 で合う（漏れなし） |
| 規則 A に当たる字句 | 7 本で 395 個（`research.md` の数と同じ） |
| 規則 D の名前の追跡を今の対象に当てた見込み | `package.ps1`・`crates-io.ps1`・`test-all.ps1` の素の `cargo` の変数（`$I686`・`$OUT_DIR`・`$t`・`$pArgs`・`$WORK_DIR`・`$i686`）の代入に `metadata`・`nar-sample-path` は無い。今の対象の合否は変わらない見込み（5.5） |
| `run-meta.txt` を読む道具 | `judge-perf.py` と `perf-rank.py` は見出しを報告に使うだけで、値は鍵で引く。値や `#` の行を英文にしても形は変わらない（4.5） |

## 直すべき点（3 つ）

### 1. 端末にもファイルにも出ない日本語の字句の扱いが文面の規則（R0〜R3）に無い

- **何が**: 規則 A は注記の外の字句をすべて見るので、端末へ出す文でもファイルへ出す文でもない字句も赤になる。該当は 2 つ。`invoke-followup-checks.ps1` 140 行の `Add-Type -TypeDefinition` のヒア文字列（C# の中の日本語の注記・172〜173 行）と、`check-quiet.ps1` 503〜520 行の自己較正用の TOML の見本のヒア文字列（日本語の `#` の注記）。設計の「変更するファイル」は `check-quiet.ps1` を「文面だけ」としか書いておらず、R0〜R3 のどれにも当てはまらない。
- **なぜ困るか**: 直した後に判定が緑にならず（5.2）、実装係がその場で書き方を決めることになる。TOML の見本は「注記を読み飛ばす」ことを確かめる自己較正の入力なので、日本語の注記を消すだけだと確かめたいことが変わるおそれがある。
- **直し方**: 規則に R4「出力されない字句（埋め込みの C# の注記・自己較正の見本）」を足す。C# の注記はヒア文字列の外の PowerShell の注記へ移すか英文にする。TOML の見本は、ASCII の外の字の注記を残したいなら展開するヒア文字列の中で `` `u{…} `` で書き、確かめる中身を変えない。
- **要件**: 4.1・5.2・5.4
- **設計の箇所**: 「端末へ出す文の規則（要件 4）」・「Modified Files」の `check-quiet.ps1` と `invoke-followup-checks.ps1`

### 2. 2 つ目の子の起こし方（`Invoke-CheckQuietOnce`）を確かめる手が無く、手元の確かめ（7.2）の前提の実行体の出どころも決まっていない

- **何が**: `invoke-perf-run.ps1` の `Invoke-CheckQuietOnce` は `Invoke-Utf8Child`＋`Get-PwshChildArguments` に替わり、出力の順・終了コードの持ち帰り・`throw` の扱いが変わる。ところが設計の確かめ（判定の 932 の確かめ・`perf-loop.ps1 selftest`・`rank-run -DryRun`）はどれもこの道を通らない。`perf-loop` の `-DryRun` は子へ `-AutoQuiet` を渡さない（`perf-loop.measure.ps1` 257〜262 行）ので、`check-quiet.ps1` は呼ばれない。さらに `rank-run` ほか子の PowerShell を起こす計測のサブコマンドは、実行体が無いと `-DryRun` で子を起こさずに終わる（`Test-MeasureExeReady`）。このワークツリーには `target\` 自体がまだ無い。
- **なぜ困るか**: 本番の計測はすべて `-AutoQuiet` で走る（同 260 行）。この道が壊れていると、最初の本物の計測（静かな机の時間）で初めて分かる。実行体の出どころが決まっていないと、7.2 のために release のビルドを回す（重い）ことになりかねない。
- **直し方**: 手元の確かめに `invoke-perf-run.ps1 -AutoQuiet -DryRun`（頭書 95 行のとおり、`-DryRun` でも静寂の確認は本当に回る「配管の確認」）を 932 の自分だけの端末で 1 回足し、`CHECK-QUIET RESULT` の行が写されることと終了コードを記録する。7.2 の実行体は、`-Build dev` で `cargo build -p areka`（回す前に所要時間を伝える）か、既にある実行体を `-BinDir` で読むだけで使うか、どちらかを設計に書く。
- **要件**: 2.1・2.2・7.2・7.3
- **設計の箇所**: 「Testing Strategy」の「手元での確かめ」・「InvokePerfRun」の `Invoke-CheckQuietOnce`

### 3. 規則 C の「文字列の中身の検査」の自分の例外が、`encoding-check.ps1` のファイル全体に広すぎる

- **何が**: 設計は `tools/encoding-check.ps1` の全部の文字列を中身の検査から外す。今の規則 C の例外は関数の名前（`Set-OwnConsoleCp932` の中だけ）で狭く限ってあるのに対し、ファイル単位の例外になる。`encoding-check.ps1` は子の pwsh へ変数で文を渡す所（`$emit` を `-Command` へ）を既に持ち、この spec でさらに子を起こす所が増える。変数で渡す形は今の `-Command` の検査に掛からないので、例外が広いと、判定自身に書き替えの文が入っても誰も捕まえない。
- **なぜ困るか**: 判定が自分を見張れない穴ができる。名前つきの狭い例外という今の作りとも揃わない。
- **直し方**: 例外を、当たると分かっている所だけに狭める。例: 較正の見本の並び（`$calib` への代入の右辺）と、`Get-ConsoleRewrites` の中の語の一覧（`@('chcp', 'chcp.com')`）の文字列だけを外す。ほかの文字列（`$emit` など）は中身を検査する。あわせて規則 D の名前の追跡は大文字小文字を区別しない（PowerShell の変数名と同じ）と明記する（`package.ps1` は `$I686`、`test-all.ps1` は `$i686` と書く）。
- **要件**: 5.3・5.5・5.6・5.7
- **設計の箇所**: 「EncodingCheck」の「規則 C の広がり」の「自分の例外」・「規則 D の広がり」

## 設計のよい所

1. **根から直していて、前置きの在り処が 1 つ**: 端末を書き替える代わりに、子の PowerShell の中だけで `[Console]::SetOut`／`SetError` に UTF-8 の書き手を渡す。Python の子へ `PYTHONIOENCODING` を渡すのと同じ考えで、実測で全部の出力の経路と 932 に無い字まで届くことを確かめてある。前置きは `Get-PwshChildArguments` だけに置き、`Invoke-Child`・`Invoke-PwshChild` の契約と呼ぶ側 7 か所を変えない。`invoke-perf-run.ps1` が `perf-loop.common.ps1` を読み込めない（`Write-Info`・`Stop-Run`・`Get-PwshPath` がぶつかる）ことも先に見つけて、新しい部品を小さなファイルに切り出している。
2. **戻りを捕まえる判定が偶然に頼らない**: 932 の判定は、まず前置きの無い形で見本が壊れることを確かめ（壊れなければ赤）、次に本物の `Invoke-PwshChild` で 3 行が 1 字違わず届くことを見て、最後に端末の値を OS から読んで 932 のままかを見る。.NET が端末の値を覚えてしまうので今の「own console code pages unchanged」が子の書き替えに気付けない穴まで、`GetConsoleOutputCP` で塞いでいる。棚卸の表も行番号まで正しく、漏れが無い。

## 最終の判断

- **判断: GO**
- **理由**: 作りの向き・境界・依存の向きは steering（`tech.md` の「道具が端末へ出す文は ASCII だけ・端末を書き替えない」）と合っており、要件 1〜10 のすべてに部品と確かめが付いている。上の 3 点はどれも設計の文を少し足せば済み、作り直しは要らない。
- **次にすること**:
  1. 設計の議論で上の 3 点を決め、`design.md` に書き足す（R4 の追加・手元の確かめに `-AutoQuiet -DryRun` と実行体の出どころ・規則 C の例外を狭める＋規則 D の大文字小文字）。
  2. その後 `/kiro-spec-tasks areka-P0-perf-tools-console-encoding` でタスクを作る。

## 細かい覚え書き（直さなくても進める）

- `Get-ConsoleCodePages` の `Add-Type` は始めと終わりの 2 回呼ばれるので、型が既にあれば組み直さない形にする（同じ名前の型を別の中身で 2 度組むと失敗する）。
- `Invoke-Judge` を `Invoke-Utf8Child` に替えると、Python の出力は終わってからまとめて写る（今は 1 行ずつ）。判定器は速く終わるので困らないが、README の新しい節に一言あるとよい。
- `invoke-cpu-sample.ps1` の受け皿を端末の出力の文字コードで読む形は、xperf が ASCII だけを書く今は結果に差が出ない。読み方の理由は注記に残しておく。
