# Brief: areka-P0-perf-tools-console-encoding

> 2026-10-06 `/kiro-discovery` で起票。出どころは `tools-utf8-child-output` の完了時の棚卸（design の Non-Goals に「`tools/perf/` の `Invoke-PwshChild` の注記と実際のずれは別に起票する候補」と残したもの）。段＝**その他**（中身はバグ寄りだが、性能改善ループは開発者の手元でしか回さない）。

## Problem

性能改善ループの道具（`tools/perf/`）が、子の PowerShell の中で `[Console]::OutputEncoding` を UTF-8 へ書き替えてからスクリプトを起動している。注記は「端末そのもののコードページは変えない（子の中だけ）」と言うが、子は親と同じ端末を共有しているので、この代入は共有の端末の出力のコードページを変え得る（.NET の `[Console]::OutputEncoding` の代入は端末のコードページを書き替える）。スクリプトが終わった後も開発者の端末に残りうる。

開発者の方針（2026-10-05）「文字化けはコンソールの文字コードを変えて逃げず、読む側を直す・道具が端末へ出す文は ASCII だけ」は、`tools-utf8-child-output` で `tools/*.ps1` と CI の段に当てた。`tools/perf/` は範囲外として残した。

## Current State

- `tools/perf/perf-loop.common.ps1` の `Invoke-PwshChild`（「.ps1 を子 pwsh で回す」の注記の関数）: 子へ `-Command` で `[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false); & '<script>' …` を渡す。
- 同じファイルの `Invoke-Child` は `ProcessStartInfo` で起動し、`UseShellExecute=$false`・標準出力と標準エラーを差し替え・`CreateNoWindow` は立てていない＝子は親の端末を引き継ぐ。読む側は `StandardOutputEncoding` を UTF-8 に固定している（読む側の形は正しい）。
- 書き替えが要る理由（注記より）: PowerShell は渡された handle へも `[Console]::OutputEncoding` で書くので、`-File` のままだと 932 の端末から回した子の出力が 932 で来る＝親の UTF-8 の読みと食い違う。
- 呼び出し元: `perf-loop.measure.ps1`（5 か所）・`perf-loop.ps1`（2 か所）。
- `tools/perf/*.ps1` の 7 本は、端末へ出す文の多くが日本語（例: `Invoke-PwshChild` の「pwsh が見つかりません」）。
- `tools/encoding-check.ps1`（`tools-utf8-child-output` の判定）は `tools/perf/` を対象から外している。

## Desired Outcome

- `tools/perf/` のどの道具も、親の端末のコードページを書き替えない（子の中の書き替えも、共有の端末に効くなら不可）。
- 932 の端末から回しても、子の出力の日本語が化けずに親へ届く（読む側で直す）。
- 端末へ出す文の扱いを `tools/*.ps1` と揃えるか、揃えない理由を決めて書く。
- `tools/encoding-check.ps1` の対象に `tools/perf/` を入れるかを決める（入れるなら判定が緑）。

## Approach

候補（要件・設計の段で決める）:
1. 子を自分だけの端末で起こす（`tools/utf8-child.ps1` の `-OwnConsole` と同じ考え）。子の中の書き替えはその端末にしか効かない。変更は `Invoke-Child` の起動の形だけで済むが、「端末の書き替え」を道具に残す形は方針の字面に沿わない。
2. 子の PowerShell の出力の文字コードを端末に依らず決める（例: 子を `-File` で起こし、子のスクリプト側で `$OutputEncoding` ではなく出力先を直に UTF-8 の writer にする・あるいは子の出力を構造化してファイル経由で渡す）。書き替えは消えるが、子のスクリプト 7 本に手が入る。
3. 子が端末へ出す文を ASCII だけにし、日本語は値（ファイル・JSON）でだけ渡す。`tools/*.ps1` と同じ規則になり `encoding-check.ps1` の対象に入れられる。文面の書き直しが大きい（7 本・数千行に日本語）。

推奨は要件の段で、手間（3 が最大）と方針への沿い方（1 が最小）を比べて決める。

## Scope
- **In**: `tools/perf/*.ps1` の子の起こし方と読み方・端末の書き替えの撤去・端末へ出す文の扱いの決定・`tools/encoding-check.ps1` の対象の決定・`tools/perf/README.md` の注記。
- **Out**: 性能改善ループの判定・計測の中身（`judge-*.py`・`perf-*.py` の計算）・`tools/*.ps1`（`tools/perf/` の外・済み）・CI。

## Boundary Candidates
- 子の起こし方（`Invoke-Child`・`Invoke-PwshChild`・`Invoke-PythonChild`）
- 端末へ出す文の文面（7 本の `.ps1`）
- 判定（`tools/encoding-check.ps1` の対象と規則）

## Out of Boundary
- `tools/utf8-child.ps1` の `Invoke-Utf8Child` を perf から読み込むかどうかは設計で決めてよいが、`Invoke-Utf8Child` の形（引数・戻り値）は変えない（`tools/*.ps1` と CI が頼っている）。
- Python の子（`PYTHONIOENCODING`・`PYTHONUTF8` の環境変数で UTF-8 にしている）は端末を書き替えないので対象外。

## Upstream / Downstream
- **Upstream**: 完了 `tools-utf8-child-output`（`tools/utf8-child.ps1`・`tools/encoding-check.ps1`・steering `tech.md` の「端末へ出す文は ASCII だけ・端末の文字コードを書き替えない」の決まり）・完了 `draw-load-parity`（性能改善ループの元）。
- **Downstream**: なし（性能改善ループを回す開発者だけ）。

## Existing Spec Touchpoints
- **Extends**: なし（新しい境界。`tools-utf8-child-output` は完了済み）。
- **Adjacent**: `release-cycle`（`tools/package.ps1`・`tools/crates-io.ps1` を使う・`tools/perf/` には触れない）・`test-all` の段 `encoding check`（対象を広げるなら判定が変わる）。

## Constraints
- 端末の文字コードを書き替えない（開発者の方針・`tech.md`）。
- `tools/perf/` の外の道具の外から見える形（`package.ps1 -Check` ほか）を変えない。
- 一時ファイルはワークツリーの `target\` の下だけ。
