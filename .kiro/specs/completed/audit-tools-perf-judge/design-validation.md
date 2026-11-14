# 設計の検証: audit-tools-perf-judge

> 2026-11-14・枝 `claude/kiro-start-audit-tools-perf-judge-e20c21`（HEAD `224eb971f`）・`/kiro-validate-design`（対話なしで回した）。
> 読んだもの: `spec.json`（language=ja）・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C20 の行）・`tools/perf/` の実物（下の「確かめたこと」）。

## 設計の検証のまとめ

81 件の指摘は、設計の「指摘ごとの直し」の表に 1 件ずつ全部載っていて、直し方・テスト・タスクの番号が付いている。共通モジュールの境目・1,000 行の決まりの当て方・C20 の申し合わせの扱いも筋が通っている。ただ、実物と照らすと、一時フォルダの根の取り方の 1 つずれと、版を 0.5.0 に上げたときに直す所の書き漏らしが見つかった。どちらも設計の文の直しで済むので、設計の討議で直してからタスクへ進めてよい。

## 確かめたこと（実物と照らした結果）

| 確かめたこと | 結果 |
|---|---|
| `research.md` §3 の #1〜#81 が設計の直しに 1 件ずつ当たるか | 当たる。「指摘ごとの直し」の表に 81 行あり、どの行にもタスクの番号がある（「—」のテストは Nice to have だけで、#1〜#27 には T1〜T27 がある） |
| `judge-perf.py`（4,159 行）と `fixtures/generate.py`（1,244 行）を増やさない決まり | D11 で「どのタスクの終わりでも越えない・同じタスクの削りと差し引き」と決めてある。タスク 1 で `perf_lines` へ移す分（約 −120 行）と、タスク 3 の消し（#36 約 −110 行ほか）が、増える #2・#6 より先に来る順になっている。`generate.py` はタスク 1 で `import sys`・`sys.path` の 1 行・`from perf_lines import` と `generate_more` の読み込み（約 +4 行）が増え、`_ANSI_RE` と `strip_ansi`（約 −5 行）が減るので、ほぼ差し引き 0。タスク 4 の #68（約 −29 行）でゆとりができる |
| 共通モジュールが判定式と較正値を持たないか | `perf_lines` の定数は `ANSI_RE`・`TIMESTAMP_RE`・`FIELD_KEY_RE` だけで、`J_CPU_CSV_HEADER`・`WARMUP_EXCLUDE_SEC`・終了コードは道具に残る。ただし `percentile` の中身は較正値 `PERCENTILE_METHOD`（`judge-perf.py:499`）の実装そのもの（下の重大な指摘 3） |
| `.ps1`・Rust・`Cargo.*`・`vendors/` を変えないか | 「Modified Files」の表に `.ps1`・`crates/`・`Cargo.*`・`vendors/` は 1 つも無い。`.ps1` が Python の出力を日本語の字面で読む所（`-match`・`-like`・`Select-String`）も 0 件（`tools/perf/*.ps1` を全部見た）。D9 で文を英語にしても `.ps1` は壊れない |
| D6 の dump の列名の行 | 検体 `fixtures-loop/rank/*/dump.txt` の列名の行は、設計の書くとおり `SampledProfile` に `Process Name ( PID)` があり、`Stack` には無い |
| 一時フォルダの根（D1） | ずれている（下の重大な指摘 1） |
| 版 0.5.0 の届く先（D10） | 書き漏らしがある（下の重大な指摘 2） |

## 重大な指摘（3 件）

🔴 **重大な指摘 1**: 一時フォルダの根が `tools\` になる（D1 の `parents[1]`）

- **問題**: D1 と `perf_selftest_common.REPO_ROOT` は、リポジトリの根を `Path(__file__).resolve().parents[1]` と書き、「`tools/perf/` の 2 つ上」と説明している。実際に Python で確かめると、`tools/perf/perf_selftest_common.py` の `parents[1]` は `tools` で、根は `parents[2]`。このまま作ると、一時フォルダは `<ワークツリー>\tools\target\perf-loop-tmp\` にできる。
- **影響**: `.gitignore` の `target` がどの深さにも当たるので（`git check-ignore` で確かめた）、`git status` は汚れず、誰も気づかない。T1 も同じ定数 `WORK_ROOT` と比べるだけなので緑のまま。要件 7.4（一時フォルダはワークツリーの `target` の下）に反したまま着地し、`perf-loop.ps1` と同じ `target\perf-loop-tmp\` を分け合うという D1 の前提も崩れる。
- **直し方**: `parents[2]` に直す。さらに、根の取り違えが黙って通らないように、`REPO_ROOT / "Cargo.toml"` があることを読み込みのときに確かめる（無ければ理由を出して exit 3）。T1 は `WORK_ROOT` と比べるだけでなく、`WORK_ROOT.parent.parent` に `Cargo.toml` があることも見る形にする。
- **要件**: 7.4・4.7
- **設計の箇所**: 「設計の決め」D1・「自己較正の支え」の `REPO_ROOT`・「新しいテスト」の T1（`research.md` §9.1 の #1 の行も同じ書き方）

🔴 **重大な指摘 2**: 版を 0.5.0 に上げると、書いていない 3 か所の自己較正が赤になる（D10）

- **問題**: D10 は `SCRIPT_VERSION` を 0.4.0 → 0.5.0 に上げ、直す所を `goals/draw-load-parity.toml` の `judge_version` だけと書く。実物では、ほかに次の所が 0.4.0 を持つ。
  - `fixtures-loop/compare/goal.toml:15` と `fixtures-loop/compare/na_secondary/goal.toml:13` の `judge_version = "0.4.0"`。perf-compare の自己較正は本物の `tools/perf/judge-perf.py` を呼ぶ（`judge_script = "tools/perf/judge-perf.py"`）ので、版の門（`perf-compare.py:429-435`）で全部のケースが判定不能になる。
  - `fixtures-loop/compare/*/expected_compare.txt` の 8 本の「判定スクリプト  : judge-perf.py 0.4.0」の行（逐語で比べる）。
  - `goals/draw-load-parity.goal.md:56` の「judge-perf.py 0.4.0」。`goal-text` は toml の `judge_version` から文を作るので、`perf-loop.ps1 selftest` の `goal-text-vs-goal-md` が赤になる。
- **影響**: タスク 6 の完了条件（触った道具の `--selftest` が緑）は judge-perf だけを見るので通るが、perf-compare と perf-loop の自己較正が赤になる。しかもタスクの見取り図では、タスク 7（`fixtures-loop/compare/` を触る）は 3〜6 と、タスク 10（`goals/draw-load-parity.toml` を触る）は 6 と並べてよいことになっていて、同じファイルを 2 つのタスクが同時に触る。
- **直し方**: D10 とタスク 6 の触るファイルに、上の 2 本の `goal.toml`・`expected_compare.txt` 8 本・`goal.md:56` を足す（`goal.md` の字数の頭書き 1,012 字は「0.4.0」と「0.5.0」が同じ長さなので変わらない）。タスク 7 とタスク 10 は、タスク 6 の後に回すか、版を上げる所だけを 1 つのタスクに切り出して 7・10 より先に置く。「並べてよいか」の欄の根拠を「触るファイルと、自己較正が読むファイルが重ならない」に改める。
- **要件**: 2.3・4.6・6.1
- **設計の箇所**: 「設計の決め」D10・「Revalidation Triggers」の 2 つ目・「タスクの見取り図」のタスク 6・7・10

🔴 **重大な指摘 3**: `perf_lines` の振る舞いを変えても、judge-perf の版を上げる決まりに掛からない

- **問題**: D2 は「`perf_lines` は判定式も較正値も持たない」とするが、`percentile` は較正値 `PERCENTILE_METHOD`（「nearest-rank・ceil(q*n) 番目」・`judge-perf.py:499`）の実装そのもの。`parse_fields`・`read_cpu_csv`（壊れた行の決め）も judge-perf の合否を直に決める。judge-perf の決まりは「較正値を変えたら `SCRIPT_VERSION` を上げる」で、perf-compare の門と preflight はこの版で「合否の意味が同じか」を見ている。ところが Revalidation Triggers は、`perf_lines` を変えたときに「自己較正を全部回す」としか書いていない。
- **影響**: 後の誰かが `perf_lines` の分位点や壊れた行の扱いを変えると、judge-perf の版は上がらないまま合否の意味が変わり、版で古い走行と新しい走行を分ける仕組みが働かない。議題 4 で共通モジュールに寄せてよいとした理由（判定に関わるものを持たない）も弱くなる。
- **直し方**: Revalidation Triggers に「`perf_lines` の関数の振る舞い（分位点・`parse_fields` の切り方・`read_cpu_csv` の壊れた行・時刻の読み）を変えるときは、judge-perf の較正値の変更として `SCRIPT_VERSION` と `judge_version` を同じコミットで上げる」を 1 行足し、`perf_lines.percentile` の説明に「`PERCENTILE_METHOD` の実装・変えるなら judge-perf の版を上げる」と 1 行書く。コードは増えない（タスク 1 の #7・#13・#51 の変更はタスク 6 の 0.5.0 に含まれる）。
- **要件**: 6.1・議題 4（要件「確定した直す対象」）
- **設計の箇所**: 「設計の決め」D2・「Revalidation Triggers」の 1 つ目・「行の読み方（`perf_lines.py`）」

## 小さな気づき（重大ではない・設計の討議で一緒に直すとよい）

- #74 のうち `fixtures/generate.py:195` の行番号の直しが、どのタスクにも入っていない（#74 の行はタスク 5・7・8・13 で、`generate.py` を触るのはタスク 4）。タスク 4 に足すとよい。
- 名前の重なり: テストの番号 T2 と既存の検体 `T2_tick_lines_absent`、設計の決め D1 と検体 `D1_dev_not_applicable`、#32 の「P1・P2」（compare のケース）と検体 `P1_boot_talk_excluded`。タスクの文で取り違えやすい。
- D9 で `perf-ledger.py`・`perf_ledger_goal.py` の失敗の文を日本語のまま残す（スキルの「台帳が無い」の見分けが日本語の語を読むため）。`tech.md` の「道具が端末へ出す文は ASCII だけ」とは違うので、かけ直しの audit（タスク 14）で指摘として出たときの扱い（範囲の外の読み手 `SKILL.md` に懸かる＝要件 5.6 で残す）を先に書いておくと、かけ直しが長引かない。

## 設計のよい所

1. **81 件を 1 件ずつ表で追える**: どの指摘にも、直し方・壊れたら落ちるテスト（#1〜#27 は T1〜T27）・タスクの番号が並び、「直したのに戻すと赤になるか」を表の「直しを戻すと」の欄で決めてある。要件 4.7 を 1 件ずつ確かめられる形になっている。
2. **共通モジュールの境目と並走の約束が具体的**: `perf_lines` と `perf_selftest_common` に何を置き、何を道具に残すかを関数の形まで決め、`parse_fields` を同じ名前で読み込むので Rust の注記 10 ファイルがずれない、と実物で詰めてある。`audit-tools-perf-run`・`audit-placement-follow` との申し合わせの範囲（変わる口は #6・#55・#67 だけ）もはっきりしている。

## 最終の判定

- **判定: GO**
- **理由**: 設計の骨組み（共通モジュール 2 本・検体の組み立ての置き場・1,000 行の決まりの当て方・15 本のタスク）は要件と steering に合い、81 件の受け皿もそろっている。重大な指摘 3 件はどれも設計の文の数行の直しで済み、作り方の向きを変えるものではない。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-tools-perf-judge`）で、重大な指摘 1〜3 と小さな気づきを design.md に入れる（1 は `parents[2]` と `Cargo.toml` の確かめ・2 は D10 とタスク 6・7・10 の直し・3 は Revalidation Triggers の 1 行）。
  2. そのあと `/kiro-spec-tasks audit-tools-perf-judge` でタスクを作る。
