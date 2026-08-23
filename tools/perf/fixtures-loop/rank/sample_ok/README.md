# fixtures-loop/rank/sample_ok — CPU サンプリング dump の合格側 fixture

`dump.txt` は、`xperf -i <etl> -symbols -a dumper -o <dump.txt>` が書き出すテキストの
**書式に合わせて手で書き起こした断片**である。実機のトレースから採ったものではない。

- 使う側: `tools/perf/invoke-cpu-sample.ps1 -SelfTest`（合格側＝`areka.exe` のフレームが
  1 つ以上数えられること）と、後続タスクの `tools/perf/perf-rank.py`（段③の順位表と
  記号解決率）。
- 対になる不合格側は `invoke-cpu-sample.ps1` の中に直書きしてある（`areka.exe` を
  1 つも含まない同じ書式の 2 行。関門が 0 と数えることを毎回確かめる）。

## 差し替えの義務（重要）

この断片は**手書きであり、実機の採取で置き換えて検証しなければならない**。
昇格した PowerShell で最初の実採取が成立した時点（タスク 7.2 の preflight／タスク 9.1）で、
実物の dump の冒頭と本ファイルを突き合わせ、列の並び・空白の詰め方・未解決フレームの
書き方が食い違っていたら**実物に合わせて本ファイルを差し替える**こと。差し替えたら本節に
採取日と採取環境（Windows の版・Windows Performance Toolkit の版）を書き足す。

- 採取状況: **未（2026-08-23 時点）。本ファイルは手書きの断片である。**

**差し替えるときは、下の「内容」の表の数値と `tools/perf/invoke-cpu-sample.ps1` の
`FIXTURE_EXPECT_*` 定数（`FIXTURE_EXPECT_SAMPLE_COUNT` / `FIXTURE_EXPECT_STACK_COUNT` /
`FIXTURE_EXPECT_AREKA_FRAMES` / `FIXTURE_EXPECT_RESOLVED_FRAMES` /
`FIXTURE_EXPECT_UNRESOLVED_FRAMES` / `FIXTURE_EXPECT_TIDS`）を必ず同時に書き換えること。**
`-SelfTest` はこの定数と**厳密一致**で突き合わせる（「1 つ以上」ではない）ので、
片方だけ変えると自己較正が赤になる。逆に言えば、厳密一致だからこそ下の
ThreadStartImage の罠が効く——数える列を取り違えた実装は 16 ではなく 32 を返して赤になる。

## 内容

| 種別 | 件数 | 備考 |
|---|---|---|
| `SampledProfile` 行 | 16 | スレッド 3 本（18332＝UI／18420＝vblank 検出／18512＝カーソル監視） |
| `Stack` 行 | 8 | 2 つの呼出スタック（UI 側 6 段・vblank 側 2 段） |
| `Image!Function` 列が `areka.exe` のフレーム | 16 | 合格側の判定に使う |
| 記号未解決のフレーム | 2 | `Unknown!0x00007ffb1c2d3e4f`（解決率を測れるように必ず 1 つ以上入れる） |
| `areka.exe` 以外の解決済みモジュール | `win32u.dll` / `ntoskrnl.exe` / `dwmapi.dll` | 帰属の切り分けが効くことを見るため |

`ThreadStartImage!Function` 列にも `areka.exe!…` が入っている。これは**わざと**である
——素朴に `areka.exe!` を数える実装だと 16 ではなく 32 と数えてしまうので、列名行から
`Image!Function` 列を引いているかどうかがこの fixture で分かる。

## 依拠した列の意味（どこまで裏を取ったか）

列名行の字面は、測定マシンに実在する Windows Performance Toolkit の
`perf_nt_c.dll` に埋め込まれている書式文字列から取った（推測ではない）。

**逐語で確認できたもの**

- `SampledProfile` の列名行（64bit 版）:
  `%23s,  TimeStamp,     Process Name ( PID),   ThreadID,           PrgrmCtr, CPU, ThreadStartImage!Function,            Image!Function, Count, SampledProfile type`
- `Stack` の列名行:
  `                Stack,  TimeStamp,   ThreadID, No.,    Address,            Image!Function`
  （64bit 版は `Address` 列の幅が広い）
- 行の書式の一部: イベント名は `%23s`（右詰め 23 桁）、プロセスは `%16ws (%4d)`、
  ThreadID は `%10d`、CPU は `%3u`、モジュールと関数は `%16ws!%ws`（`!` で連結）
- `Stack` 行の書式: `%23s, %10I64d, %10d, %3d, 0x%016I64x, %16ws!%ws`
  （イベント名・TimeStamp・ThreadID・No.・Address・Image!Function の 6 列）
- ヘッダの囲み `BeginHeader` / `EndHeader`、未解決の語 `Unknown`
- `dumper` というアクションが在ること（`xperf -help processing` の Available Actions）

**書式文字列からは確定できず、列名行から補ったもの**

- `SampledProfile` 行の末尾 2 列（`Count` と `SampledProfile type`）の書式。
  逐語で取れたのは `Image!Function` までの 8 列分の書式であり、末尾 2 列は列名行に
  合わせて `%5d` 相当の右詰め数値として書いた。
- 未解決フレームの書き方。`Unknown` という語と `%16ws!%ws` の連結は確認できたが、
  両者を組み合わせた `Unknown!0x…` という実物の行は見ていない。
- 各列の実際の詰め幅（実物は値の桁数で変わる）。**読む側はカンマで分割したあと必ず
  trim すること。** `Stack` の列名行は先頭が 16 桁詰めで、`%23s` の行と揃っていない。

## 読む側への注意

- Rust の記号は総称型の中に `,` を含み得る。カンマ分割した結果が列名行より多いときは、
  余りを `Image!Function` 側へ寄せて連結する（`invoke-cpu-sample.ps1` の
  `Measure-ArekaFrames` はそうしている）。
- release ビルドは `lto=true`・`opt-level='z'` なのでインライン化でスタックが浅くなる。
  記号は `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`（環境変数・`Cargo.toml` 非接触）で付与する。
