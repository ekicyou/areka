# ギャップ分析: audit-tools-perf-judge

> 2026-11-14・枝 `claude/kiro-start-audit-tools-perf-judge-e20c21`（HEAD `7827ee03`＝`main` の `30a343eb`＋spec の初期化 1 コミット）。
> 物差しは ponytail のプラグインの `ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.3/skills/ponytail-audit/SKILL.md`。brief は 5.3.2 と書くが、入っているのは 5.3.3。完了 `audit-impl-watch` の research.md の頭書きのとおり、5.3.2 と同じファイル）。
> 作法のうち 2 つを外した（要件 1.2・1.3）: 「最大 20 件」は外して全件を載せる。「読まなかった所（Not checked）」は残さない（範囲の 284 ファイルを全部読んだ・読み方は §1）。
> 読み方: 範囲を 7 本の読み手（読むだけのサブエージェント・A〜G）に分けて全行を読ませ、出た指摘をここでまとめて重なりを 1 件に寄せ、番号を振り直した。重い指摘（#1〜#4・#5・#15）はこちらでファイルと行を開いて中身を確かめた。

## 1. 範囲の測り直しと読み方（要件 1.1・1.3）

| まとまり | ファイル | 行（`wc -l`） |
|---|---|---|
| 直下の Python 11 本 | `judge-perf.py`・`judge_perf_catchup.py`・`judge-followup.py`・`judge_followup_selftest.py`・`perf-compare.py`・`perf_compare_selftest.py`・`perf-ledger.py`・`perf_ledger_goal.py`・`perf_ledger_selftest.py`・`perf-rank.py`・`perf_rank_dump.py` | 4,159・295・931・165・969・295・996・711・241・957・443 |
| `fixtures/` | 80（`generate.py` 1,244 行・`.gitattributes`・検体 **20 件**＝`case.txt` 20・`run.log` 20・`cpu.csv` 19・`run-meta.txt` 19） | 検体のデータ約 2.6 MB |
| `fixtures-loop/` | 190（`compare/`・`followup/`・`ledger/`・`rank/`。`judge-stub.py` 5 本は 13 行ずつ） | — |
| `goals/` | 2（`.toml` 124 行・`.goal.md` 70 行） | — |
| `README.md` | 1 | 1,178 |
| 合計 | **284**（要件の表と同じ） | Python 17 本・11,471 行 |

- 検体の数: README §9 の表と brief は 17 件と書くが、実物は 20 件（`C1_catchup_by_target_pass`・`C2_catchup_by_target_fail`・`T2_tick_lines_absent` が後から足された）。`--selftest` は 20 件を走らせ、期待 0／1／2／3 の内訳は 7／5／6／2（読み手 C）。表の古さは #78。
- 読み手と受け持ち（全員、受け持ちの全行を読んだ。照らし合わせに読んだ他の行は各報告に記録）:

| 読み手 | 受け持ち（全行） |
|---|---|
| A | `judge-perf.py` 1〜1500（頭書き・較正値のバナー 138〜716・ログ行の解析・`scan_log`・`read_cpu_csv`・`read_run_meta`・統計・窓 A／C） |
| B | `judge-perf.py` 1495〜2867（catch-up の集計・`--emit-metrics`・描画・終了コードの決め方・定常状態・判定式⑴） |
| C | `judge-perf.py` 2777〜4159（判定式⑵〜⑷b・入力の読み込み・`--selftest`・入口）・`judge_perf_catchup.py` 全行・`fixtures/*/case.txt` 20 本 |
| D | `judge-followup.py`・`judge_followup_selftest.py`・`perf-compare.py`・`perf_compare_selftest.py`・`fixtures-loop/followup/` 16 ファイル・`fixtures-loop/compare/` 全部 |
| E | `perf-ledger.py`・`perf_ledger_goal.py`・`perf_ledger_selftest.py`・`goals/` 2 本・`fixtures-loop/ledger/` 78 ファイル |
| F | `perf-rank.py`・`perf_rank_dump.py`・`fixtures-loop/rank/` 41 ファイル（照らし合わせに `crates/areka/src/perf_thread_report.rs` と同テストも全行） |
| G | `fixtures/generate.py` 全行・`fixtures/` の検体・`README.md` 全行 |

- 検体のデータの読み方（中身がバイト単位で同じものは 1 つを全文読み、残りはバイトの一致で確かめた）:
  - `fixtures/` の `run.log`（17 KB〜514 KB）は `generate.py` が作るもの。`generate.py` を `target\audit-perf-judge-tmp\G\` へ写して回し（`generate.py` は自分の隣へ書くので、その場では回していない）、作り直した 78 ファイルが登録済みのものと**全部バイトで一致**（差 0・余分 0・Python 3.13.17）。さらに登録済みの `run.log` 20 本の全行を、数字を伏せた形ごとに数え（19 種類・全部読んだ）、各本の先頭 150 行と末尾 50 行を読んだ。`generate.py` は全行読んでいるので、中身は作り方から全部決まる。
  - `run-meta.txt` 19 本は同じ md5（1 本を全文）。`cpu.csv` は 4 種類（各 1 本を全文・どの検体がどれかを md5 で確かめた）。
  - `fixtures-loop/compare/runs/` の `run.log` 12 本は 1 本を全文読み、残りはスクリプトで差分を取って違う行を全部読んだ（時刻だけ・catch-up 2 行の足し・表示 2 行の削り）。`cpu.csv`・`run-meta.txt` も同じ方法。
  - `fixtures-loop/ledger/*/judge-stub.py` 5 本は git の blob が同じ（`1bee5934…`）。1 本を全文。
- 実験の一時ファイルは全部ワークツリーの `target\audit-perf-judge-tmp\<読み手>\` の下に置き、終わってから消した（フォルダごと消した・`git status` は空のまま）。読み手 D が 1 度 `target\stderr_d.txt` に書いて直ちに消した（`target\` の下）。

## 2. この範囲がしていること・想定した負荷

- **何をするか**: areka の自走改善ループ（`perf-loop-iteration` スキル）の判定と台帳。PowerShell の道具（範囲の外・`audit-tools-perf-run` の持ち物）が areka を 7 分か 25 分動かして `run.log`・`cpu.csv`・`run-meta.txt` を集め、Python の道具が次をする。
  - `judge-perf.py`: 合否（0 合格・1 不合格・2 判定不能・3 引数の誤り）。判定式⑴コマの間隔・⑵catch-up・⑶確保・⑷a アイドル CPU・⑷b 収束。`--emit-metrics` で `metric=<名前> value=<値>` を出す。§9 の catch-up 突合は兄弟 `judge_perf_catchup.py`。
  - `perf-rank.py`（＋`perf_rank_dump.py`）: 4 段の順位表 `rank.txt`（プロセス・スレッド・関数〔xperf の dump〕・相）。
  - `perf-compare.py`: A1→B1→A2→B2 の 4 本に `judge-perf.py --emit-metrics` を別プロセスで回し、`ADOPTED`／`NO_DIFF`／`WORSE`／`MEASURE_FAILED`。
  - `perf-ledger.py`（＋`perf_ledger_goal.py`）: ループの唯一の記憶 `loop-ledger.md` の読み書き・相の遷移表・STATUS／FINAL 行・目標定義の検査・`/goal` の条件文。
  - `judge-followup.py`: 見た目の追随（クリック透過・ドラッグ・DPI・バルーン追従）の合否。
  - 各道具の `--selftest`（`*_selftest.py` と検体）。標準ライブラリだけ（`tomllib` のため Python 3.11 以上・この機械は 3.13.17）。
- **読む口と書く口**: PowerShell の道具は Python を引数と終了コードで呼び、`key=value` の行（`FOLLOWUP VERDICT`・`metric=` など）と最後の行（`LastLine`）を読む。`perf-loop.common.ps1:464-475` は `judge-perf.py` の `SCRIPT_VERSION` を字面で読む。`judge-followup.py:70` の `CHECK_ALL` は `invoke-followup-checks.ps1:122`・`perf-loop.measure.ps1:84` と対で、**今は 3 か所とも同じ 4 語・同じ順**（読み手 D）。台帳の語と STATUS／FINAL 行はスキルと `/goal` の判定役が読み、`.ps1` は読まない（読み手 E）。
- **想定した負荷**: 開発機 1 台・ループは 1 本ずつ・1 セッションが呼ぶ。`run.log` は 25 分で数十 MB（読み手 A が 40 MB・12.8 万行の擬似ログで `scan_log` を回し 3.8 秒・最大約 200 MB で問題なし）。同じ機械に他のセッションが居ても、この道具を同時には呼ばない。
- **今の自己較正**（要件 4.6 の確かめ・読むだけ）: `pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest` を HEAD `7827ee03` で 1 回回した。**緑**（`SELFTEST RESULT ok=9 ng=0`・43 秒・GPU も実機の走行も無し）。内訳は `judge-perf.py` ok・`perf-rank.py` 9/9・`perf-compare.py` 8/8・`perf-ledger.py` 21/21・`judge-followup.py` 4/4・`invoke-cpu-sample.ps1` ok・`check-quiet.ps1` ok・`invoke-followup-checks.ps1` 4/4・`goal-text-vs-goal-md` ok（1,012 字一致）。回した後の `git status` は空。
  - 一時フォルダ: `perf-loop.ps1` の `goal-text` の検査はワークツリーの `target\perf-loop-tmp\` を使う（後で消す）。**Python の自己較正 4 本は OS の一時フォルダ（`%TEMP%`）を使う**＝要件 7.4 に反する（#1）。
  - 端に出た事実: `judge-perf.py --selftest` の最後の行は日本語で、`perf-loop.ps1:381` がそれを自分の行に埋め込む（Git Bash から回すと化けて見えた）。ほかの 4 本は ASCII の `SELFTEST RESULT ok=N ng=M` で終わる（#24）。

## 3. 指摘の一覧（全 81 件）

組の名は audit の 3 つ: **Must fix（必ず直す）**・**Should fix（直したほうがよい）**・**Nice to have（できれば）**。番号は 3 つの組を通して 1 から（以後の段はこの番号で指す・要件 1.5）。

- 数: **Must fix 4 件・Should fix 37 件・Nice to have 40 件**。
- 「perf-run の口」の欄: 直しが `audit-tools-perf-run` と互いに読む口（引数・終了コード 0〜3・`key=value` の出力の語・`CHECK_ALL`・`SCRIPT_VERSION`）を変えるか（要件 6.1）。**「変わる」は C20 のあいだ申し合わせなしに取れない**（要件 6.3＝議題）。「文面」は `.ps1` が `LastLine` を自分の文に埋めるだけで解析しない所の字面が変わるもの（口の語ではないが、知らせはする）。「—」は変わらない。
- 「範囲外」の欄: 直しが `tools/perf/` の非 `.ps1` の外のファイルに及ぶとき、そのファイル。`.ps1` に及ぶものは要件 6.2 で申し合わせが要る。
- 「残す候補」の欄: 直さずに残してよい 3 種類（⑴ 開発者の裁定・完了した spec の決め／⑵ わざと置いた先送りの印／⑶ SSP・ukadoc の正典に関わる振る舞い）に当たるかもしれないもの。決めるのは要件の討議。「—」は当たらない。**⑵ に当たる印（`shortcut:`・`ponytail:`）は範囲に 1 つも無い**（7 本の読み手が全員 grep で確かめた）。**⑶ に当たるものは無い**（開発の道具で、SSP・ukadoc とは関わらない。要件 3.2 の ukadoc での確かめは要らない）。

### 3.1 Must fix（4 件）

| # | 場所 | 何が起きるか（具体的な場合） | いちばん小さい直し | perf-run の口 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 1 | `judge_followup_selftest.py:130`・`perf_compare_selftest.py:273`・`perf_ledger_selftest.py:222`・`perf-rank.py:862`（どれも `tempfile.TemporaryDirectory(prefix=…)`） | **Python の自己較正 4 本が、検体の写しと出力を OS の一時フォルダ（`%TEMP%\perf-*-selftest-*`）に作る**。`perf-loop.ps1 selftest` と preflight のたびに起きる（今回の緑の走行でも作った）。「一時フォルダはワークツリーの `target\` の下だけ」（要件 7.4・記憶の絶対ルール）に反する。`perf-loop.ps1:260,273` の自前の一時物は `target\perf-loop-tmp\` に置いている | 4 か所とも `dir=<リポジトリ>\target\perf-loop-tmp`（`mkdir(parents=True, exist_ok=True)`）を渡す（各 +2 行）。#41 で支えを 1 つにするなら 1 か所 | — | — | — |
| 2 | `judge-perf.py:568-580`（`J_TALK_START_EVENTS`・`J_TALK_END_EVENTS`）・使う所 `talk_spans`／`talk_peak_cpu_pct` `:1526-1561` | **発話の区切りの語彙が今の kanade と合わず、`talk_peak_cpu_pct`（発話中の CPU の頂）が起動直後の CPU の最大値になる**。起動挨拶は `boot_talk` で始まり `boot_talk_done`（`crates/areka-kanade/src/schedule/boot.rs:330`）で終わるが、終わりの語彙に `boot_talk_done` が無いので、区間が次の `steady_talk_done` まで閉じない（試した例: boot_talk 5 秒・発話 300〜310 秒・CPU 15 秒時点 40%／305 秒時点 4% → 頂 40.0、正しくは 4.0）。実走では毎回この形。ほかに `close_refused` は crates のどこからも出ない（消えた語）、`close_talk_done_quit`（close.rs:132）・`talk_done_break_quit`・`change_talk_done_unload`・始まりの `change_talk_start`・`change_yielded_talk_start`・`sstp_notify_talk` が抜けている。注記の行番号も全部ずれている。値は `metric=talk_peak_cpu_pct` として `perf-compare.py` が `compare.json` へ写す（合否には使わない） | 2 つの語彙を今の kanade の `event=` に合わせる（約 +6 行）。起動の重い区間が頂に入らないことを見る検体を 1 本（`generate.py` は 1,000 行超えなので置き場は設計で決める・§6） | — | 検体（`fixtures/C1_*` の `contains` が `value=4.20` を固定＝直した後も同じ値か確かめる）。kanade の語は読むだけ（要件 5.3＝Rust の書き手の行の読み方が変わる直しとして扱う） | — |
| 3 | `perf_rank_dump.py:249-331`（dump の行を読む所。`Process Name ( PID)` の列を一度も見ない） | **段③（関数別）が areka 以外のプロセスのサンプルまで数える**。xperf の採取（`invoke-cpu-sample.ps1:127-132`）はシステム全体なので、Idle や他のプロセスの `SampledProfile`／`Stack` の行も `samples_total`・自己時間・包含時間・[2b] に入る。静かな 8 コアの機械では自己時間の 1 位が Idle などのカーネル関数になり、areka の関数の割合が数 % に薄まる。検体は全行が `areka.exe (23140)` なので出ない。実機の dump はまだ一度も読ませていない（段③は実走で全周 UNAVAILABLE だった） | `SampledProfile` は処理名の列が `areka.exe (` で始まる行だけを残し、その (時刻, TID) の組に当たる `Stack` 行だけを数える（約 +8 行）。他のプロセスの行を混ぜた検体を 1 本 | — | `README.md:919-920` の一文。`.ps1` の `Measure-ArekaFrames`（`invoke-cpu-sample.ps1:248-317`）にも同じ穴（範囲外の観察・§4.2） | — |
| 4 | `perf-rank.py:473-491`・`:539-540`（段②の差の取り方）。規則の出所 `crates/areka/src/perf_thread_report.rs:225-239` | **TID が使い回されると、段②の合計がプロセスの CPU を超える**。終わったトークのスレッド（前回 5 s）の TID を新しいトークのスレッド（今回 1 s）がもらうと、新しい方は 0 に止められ、名簿外の残りの差に 6 s が載る。プロセスの差が 3 s なのに `unregistered_rest share_pct 200.00` が出た（`target\` の下で再現）。新しいトークの CPU は 0、名簿外には幻の CPU が載る | perf-rank の中だけで: 名簿外の残りを行どうしの差でなく `max(プロセスの差 − 名簿の差の合計, 0)` で出す（約 +2 行）。もう 1 案は (TID, 名前) で照らして名前の変わったものを新しいスレッドとする形で、こちらは Rust の `delta`（規則の正本）とテストも変える | — | 2 つ目の案なら `crates/areka/src/perf_thread_report.rs` と同テスト（`talk-thread-count-growth` の触る所と重なる）。`README.md:878` の「名簿は登録を消しません」 | — |

### 3.2 Should fix（37 件）

**バグと危ない所（#5〜#27）**

| # | 場所 | 何が起きるか | いちばん小さい直し | perf-run の口 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 5 | `judge-perf.py:3439-3440`（`meta_profile is None` だけを見る関門） | `run-meta.txt` の `profile =`（空）や `profile = 25min`（知らない値）で release の verdict を走らせると exit 0。⑷b の「long なのに 20 分に届かないなら判定不能」の関門を、手で作った run-meta がすり抜ける（H11 の写しで確かめた） | 条件を `(meta_profile or "").strip().lower() not in ("short", "long")` に（±0 行）＋検体 1 本 | **文面に近い**: 空・未知の profile のときだけ終了コードが 0→2。`invoke-perf-run.ps1:493-502` は short/long を確かめてから書くので、道具が書いた走行では変わらない（知らせはする） | — | — |
| 6 | `judge-perf.py:4144-4159`（`main` が `JudgeError` しか捕まえない） | 想定外の例外（後の直しで入る `TypeError`・`MemoryError` など）で落ちると Python の既定の exit 1＝契約上「不合格」。`perf-loop.measure.ps1:339` は `verdict FAIL` と記録し、ループは道具の故障を「直すべき欠陥」と読む。実際に落ちる入力は見つけていない（仕組みの指摘） | `except Exception` を 1 つ足し、traceback を stderr に書いて 3 を返す（+4 行・#38 の −6 行で相殺） | **変わる**: 想定外の例外のときの終了コード 1→3（`perf-loop.measure.ps1:339-344` は 3 を計測失敗、`perf-compare.py:419` は判定不能として扱う＝望ましい向き）。要件 6.3 の議題 | — | — |
| 7 | `judge-perf.py:1145-1150`（`read_cpu_csv`）・写し `perf-rank.py:358-393`・受け手 `perf-compare.py:369-376`（`to_number`） | `cpu.csv` の 1 行が `NaN`（`inf` も）だと数として通り、「平均 nan%」で⑷a が不合格・総合 exit 1（判定不能ではない・H11 の写しで確かめた）。`--emit-metrics` も `value=nan` を出し、perf-compare は B2 が nan なら `MEASURE_FAILED` でなく `WORSE` を返して台帳に「試して悪化」と残る（perf-analyze の `already_tried` で候補から外れる）。道具の側は `ToString('F2')` で書くので、カウンタが NaN を返したときだけ起きる | 3 か所で `math.isfinite` を見て読めない行・数の無い値として扱う（各 +1〜2 行） | — | — | — |
| 8 | `judge-perf.py:2473-2524`（判定式⑴の明示指定の関門） | 関門が不合格の確かめより先に判定不能を返す。指定を 2 系列にして A が p95 200ms（上限超え）、B が定常状態に 1 本も出ないと、⑴は不合格でなく判定不能、総合も 1 でなく 2。頭書き（73 行）とバナー（375 行）の「不合格が優先」に反する。今は指定が 1 系列なので起きないが、README §12 の 1 番は走行ごとに指定を見直させる | 関門では未観測を集めるだけにし、`if failures:` の後で未観測があれば判定不能（行の移動だけ・±0） | — | — | — |
| 9 | `judge-perf.py:1638-1640`（`render_metrics` の `catchup_count`）・`:2794`・`:2833-2849` | 時刻の読めない catch-up 行が 1 本あると、⑵は判定不能なのに `metric=catchup_count value=0` を出す。`goals/draw-load-parity.toml:27` の `must_not_regress` に `catchup_count` があるので、A/B の比べは「数えて 0」と読む（道具自身の決め「`-` は測っていない・0 は数えて 0」に反する）。検体に時刻の無い catch-up 行は無い | `undated > 0` なら `-` を出す（±0） | — | — | — |
| 10 | `perf-compare.py:287-292`（`load_goal` の床の検査） | `noise_floor_pct = 0`・`-1`・TOML の `nan`・`true` が通る。床 0 で 4 本とも 6.10 のとき `delta=0.00 ≤ −0.00` で **ADOPTED**（変わっていない変更が採られる・実験で確かめた）。本番の目標定義は 0.30 | `bool` を拒み、`not floor > 0` で拒む（±1 行） | —（拒むときは既存の exit 3） | — | — |
| 11 | `perf-compare.py:362-363`・`:484-489` | 目標定義の副指標の名前を書き違える（例 `frame_interval_p95`＝`_ms` の抜け）と 4 本とも NA になり、p95 が延びた変更でも ADOPTED（実験で確かめた）。表に NA とは出るが判定は止まらない | 数の取れた走行のどれにもその名前の鍵が無ければ exit 3（+4 行）。「鍵はあるが値が `-`」の NA の決めは残す | —（既存の exit 3） | — | — |
| 12 | `perf-compare.py:399-420`（`completed.stderr` を使わない） | 判定スクリプトが失敗した理由を捨てる。`no_show_lines` の検体で judge-perf は stderr に「必要ログ種が欠けています…表示成立点の行が 1 本もありません」と出すが、`compare.txt`・`compare.json`・perf-compare の stderr には「終了コード 2 で終わりました」しか残らない（記録の無い失敗の道＝要件 7.2）。原因を知るには手で回し直すしかない | `compare.json` の各走行に `judge_stderr` を持たせ、計測失敗の stderr に添える（+4 行。`compare.txt` は逐語の自己較正のため変えない） | —（`compare.json` に鍵が 1 つ増えるだけ。スキルが写す 6 つの鍵〔`SKILL.md:201-206`〕は変わらない） | — | — |
| 13 | `judge-followup.py:205-207`・`judge-perf.py:820`・`perf-rank.py:197`（BOM の判定が見えない文字 U+FEFF をそのまま書いた文字列） | 書き込みの道具やエディタがこの見えない 1 字を落とすと `startswith("")` で常に真になり、どのファイルも先頭の 1 字が消える（`probe.log` の 1 行目が `robe: …` になり毎回 exit 3）。記憶「書き込みの道具が逃がしを化かす」と同じ型の壊れ方 | `raw.decode("utf-8-sig")` にして判定の行を消す（各 −2 行。judge-perf.py は行が減る） | — | — | — |
| 14 | `perf-ledger.py:575-578`・`:817`・`perf_ledger_goal.py:242-243` | `verdict` の無い `entry.json` を `append` すると exit 0 で `- verdict: -` が書かれ、その後の `status` は「知らない採否です: -」で exit 3。スキルは相の境目ごとに status を出すので、次の append までずっと止まる（再現した） | `load_entry_json` で verdict が空なら exit 3（+2 行） | —（`.ps1` は append も status も呼ばない） | — | — |
| 15 | `perf-ledger.py:825`（`iteration = … state_int(ledger, "iteration") + 1`）・`:834`・`:956` | `append` の既定の周番号が、スキルの読み方（`SKILL.md:42-46`・`:238-241`＝「iteration は今まわしている周」）と 1 つずれる。周 1 の RECORD で `--iteration` を書き忘れると `## 周 2` で書かれ `results\iter-1` と食い違い、状態の iteration も上書きされる。スキルは太字で「必ず渡す」と注意しているだけ | `--iteration` を必須にし、既定の枝を消す（約 −1 行）。既定に頼る検体の `steps.txt` 5 本に `--iteration N` を足す | —（`.ps1` は append を呼ばない。引数はスキルが読む口） | `SKILL.md:45-46`・`:238-241` の注意書きは短くできる（直さなくても動く） | — |
| 16 | `perf_ledger_goal.py:71`（`SCRIPT_VERSION` を読む正規表現）と `perf-loop.common.ps1:472` | 同じ版を 2 つの正規表現で読み、規則が違う（Python は行頭に縛らず単引用符も可で `x.y.z` の数字だけ、PS は行頭だけ・二重引用符だけ・空白までの全部）。版が `0.5.0rc1 (…)` になると Python は `0.5.0`、PS は `0.5.0rc1` と読み、preflight の 2 回の確かめ（`perf-loop.ps1:472-474` と goal-check）の片方だけが一致になる。今は一致している | Python 側を PS と同じ `(?m)^SCRIPT_VERSION\s*=\s*\(?\s*"([^"\s]+)` に（±0） | —（`.ps1` は変えない。`SCRIPT_VERSION` の字面にも触れない） | — | — |
| 17 | `perf_ledger_goal.py:526-531`（goal-text の「終端と読める行」の検査） | `/goal` の判定役は「`PERF-LOOP FINAL: GOAL_MET run=<token>` で**始まる**行」で判定する（テンプレート自身が :81 でそう書く）のに、検査は STATUS／FINAL の正規表現への完全一致しか拒まない。テンプレートの行頭に `PERF-LOOP FINAL: GOAL_MET run={token} が出たら達成` と書くと検査を通り、条件文を貼った時点で達成と判定されうる。今のテンプレートには無い | `line.startswith((STATUS_PREFIX, FINAL_PREFIX))` で判定（±0） | — | — | — |
| 18 | `perf_ledger_goal.py:542-553`（`_final_release_idle_text`）・`:566` | `summary` の「最終」列の 25 分の値を `verdict.txt` の文面（judge-perf.py:3147 に依存）で拾い、文面が変わる・開けない（`except OSError: return None`）・`results_dir` が `results` 以外のどれでも、黙って最後の周の 7 分の値を出す。コメント :564-565 が塞いだと書く「7 分値 8.91 が 25 分の不合格 22.3% を隠す」穴そのもの。自己較正の `summary_table` に `results/` が無く、この道は一度も試されていない。docstring :559 の「純関数・時計を読まない」も事実と違う | 拾えないとき・OSError のとき stderr へ理由を出す（+3 行）・`--goal` があれば `[goal].results_dir` を使う（+1 行）・検体を 1 本 | — | — | — |
| 19 | `perf-ledger.py:224`（`_TOP_REMAINING_PATTERN = [A-Za-z0-9_.:%+-]+`）・`perf_ledger_goal.py:294` | スキル（`SKILL.md:298-302`）は `--top-remaining` に順位表の最上位を渡す。段③が使える回で最上位が `function:alloc::vec::Vec<T>::push:12.3%` のような Rust の記号名だと exit 3 で FINAL 行を出せず、ループが終われない。実走では段③が全周 UNAVAILABLE で、まだ踏んでいない | `cmd_final` で許さない字を `_` に置き換える（+1 行。山括弧を通さない約束は保たれる） | — | — | — |
| 20 | `perf-ledger.py:813-817` と `.claude/skills/perf-loop-iteration/templates/entry.json` | ひな型の山括弧の値（`"<A1=… B1=…（…）>"`・`"commit": "<採用したときだけ短い SHA。それ以外は null>"`）が、そのまま台帳に入る。IMPLEMENT が BLOCKED の周のように値を埋めない道では、その文が記録され `summary` の表にも出る（append が exit 0 で受けることを確かめた） | `<` で始まり `>` で終わる文字列の値を「ひな型のまま」として exit 3（+2 行） | — | ひな型はそのままでよい | — |
| 21 | `perf-rank.py:428-433`・`:441`・`:451-454`（段①の `talk_peak_pct`） | judge-perf の `talk_peak_cpu_pct`（発話の区間・#2）と別の規則（`apply(ShowSurface)` の前後 10 秒）で「発話の頂」を名乗る。走行は `areka_emo_present=debug` でコマ送りのたびに ShowSurface 行が出るので、瞬きだけで発話 0 回の走行でも `talk_peak_pct == max_pct`（judge は None）。`.claude/agents/perf-measure.md:57` がこれを `talk_peak=` と写す | rank から `talk_peak_pct` と窓の定数・注記を消す（約 −12 行）。期待出力 3 本と perf-measure.md を直す | — | `.claude/agents/perf-measure.md:57`・`README.md:830` | — |
| 22 | `perf-rank.py:559-571`（役割別の足し合わせ） | トークのスレッド名は `sakura-talk-{id}`（`drive.rs:93`）で、役割は `actor:sakura-talk-5` のように 1 本ずつ別になる。窓の中で 4 トークが各 0.2 s 使っても 4 行に分かれ、cursor_monitor の 0.5 s より下に並び、上位 N で切られうる。役割の和（0.8 s）が見えない | `actor:` の役割の末尾の `-<数字>` を `-*` に畳む（+2 行） | — | — | —（`talk-thread-count-growth` が畳み方を決めるなら、そちらへ渡す案もある＝§7） |
| 23 | `perf_rank_dump.py:320-325`（数え方）・`:387`（関門）・`:420` | PDB が無く行が `areka.exe!0x00007ff6…` のとき、その行は未解決と areka のフレームの両方に数えられ、「解決フレーム 0 なら exit 4」の関門を通ってしまい、順位表がアドレスだけで埋まる。実物の未解決行の綴りは確かめていない | `if module == TARGET_MODULE` を解決済みの枝の中へ移す（±0） | — | `.ps1` の `Measure-ArekaFrames:317` も同じ形（範囲外の観察） | — |
| 24 | `judge-perf.py:4026`（自己較正の最後の行）・`:4154`（失敗の締めの行）・`:4054`・`:4120-4123` | 最後の行が日本語（「自己較正は以上。終了コード 0」「部分集計は出していません。終了コード 2」）で、`perf-loop.ps1:381` と `perf-loop.measure.ps1:344` がそれを自分の英語の行へ埋め込む（`[perf-loop] FAILED: no verdict (...): [judge-perf] 部分集計は…`・理由はその 1 行前にあって載らない）。ほかの 4 本は ASCII の `SELFTEST RESULT ok=N ng=M`。README §11（579 行）は「道具が自分で端末へ出す文は ASCII だけ（失敗の文・RESULT の行）」と書く | 4026 を `SELFTEST RESULT ok=… ng=…`、4154 を `JUDGE RESULT code=…` のような ASCII の行に（±0）。レポートの本文は日本語のまま | **文面**: `.ps1` は LastLine を埋めるだけで解析しない（grep で読む所 0）。知らせはする | — | ⑴の可能性（README:583-585 が「写した子の中身」の日本語を認める＝§7 議題 3） |
| 25 | `perf-rank.py:868-883`・`:894`・`:946`・`:949-952`・`RankError` の文言 | `python perf-rank.py --selftest > log` を cp932 で回し、段④の期待出力に食い違いがあると、`_first_difference` の repr に入る `µ`・`—` を書けず `UnicodeEncodeError` の traceback（exit 1）で落ちる。`perf-loop.measure.ps1:427` は日本語の LastLine を英語の停止文へ混ぜる | 入口で `sys.stdout.reconfigure(errors="backslashreplace")`（stderr も・+2 行）。全部を ASCII にするかは議題 3 | **文面**（`measure.ps1:427` の LastLine） | — | ⑴の可能性（議題 3） |
| 26 | `judge-followup.py:793-828`（報告の本文を標準出力へ）・`:870`・`:924-926`・`perf-compare.py:874-876`・`:913`・`:961-964`・両方の selftest の `[selftest] …` 行 | 端末へ出る文に日本語がある。`invoke-followup-checks.ps1:303-306` と perf-loop はこれを端末へそのまま写す。ASCII なのは `FOLLOWUP VERDICT`・`PERF-COMPARE RESULT`・`SELFTEST RESULT` の行だけ | judge-followup は日本語の報告を `followup-verdict.txt` だけに書き、標準出力には ASCII の判定行だけ。stderr の文（約 20 本）を英語に。期待出力 4 本をファイルとの突き合わせに | **文面**（`invoke-followup-checks.ps1:859` はファイルを読み、`perf-loop.measure.ps1:918` は `FOLLOWUP RESULT` 行を見る＝口は変わらない） | `README.md:583-591` | **⑴**: README:583-585 と完了 `areka-P0-perf-tools-console-encoding` の design.md（`judge-followup.py` の日本語の報告を子の中身として写す形を決めた）。議題 3 |
| 27 | `README.md:38`・`:42`・`:46`・`:860`・`:884-886`・`:993`・`:1117-1121` | 早わかり（§1）を写して打つと `-OutDir C:\出力先\perf-001` で C:\ 直下に計測の根ができる。§14・§15・§17 も `C:\出力先`・`C:\出力\cpu.etl`。「実機の根・一時フォルダはワークツリーの `target\` の下だけ（C:\ 直下は不可）」に操作する人を逆らわせる | 例を `<ワークツリー>\target\perf\perf-001` などの置き換え札に。§1 は `-OutDir` を省いて既定に任せる（±0） | — | — | — |

**テストの抜け（#28〜#35）**

| # | 場所 | 何が壊れても緑のままか | いちばん小さい直し | perf-run の口 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 28 | `judge-perf.py:2964-3035`（収束）・`:3165-3235`（⑷b）・`:3075-3109`（dev の⑷適用外）・検体 20 本は全部 `release`・`short`・`cpu.csv` 25 行以下 | 最終判定（25 分・long）の⑷b と収束、dev の経路に赤になる検体が無い。対応表（頭打ち＝合格・単調上昇＝不合格）を入れ替えても、「long なのに 20 分に届かない」を判定不能にする枝を消しても緑。`perf-loop.measure.ps1:587,967`（measure-baseline と final）はこの経路で判定を出している | 検体を 3 本（release・long・平坦 22 分＝0／右肩上がり＝1／long なのに 6 分＝2）、dev を 1 本。`cpu.csv` は 90 行ほど | — | — | — |
| 29 | `judge-perf.py:2565-2572`・`:2701-2722`（明示指定の系列の間隔が 20 本未満） | README §12 の 4 番の約束（20 本未満なら上限内でも判定不能・超えていれば不合格）を、どの検体も踏まない。分岐を消しても緑（メモリの上で 19 本→2・20 本→0・19 本で 1 本超え→1 を確かめた＝今は正しい） | H11 からの検体を 2 本（19 本で 2・20 本で 0） | — | README §12 の 4 番に検体の名を書き足す | — |
| 30 | `judge-perf.py:2275-2294`（`steady_state_status` の 30 回未満・60 秒未満） | 定常状態が「薄いが空ではない」を判定モードで試す検体が無い（空の形は E1）。`B1_baseline_ok` の case.txt は「合否判定なら 2」と書くのに、その入力を判定モードで回す検体が無い。`<` を `<=` にしても緑 | B1 と同じ入力で `mode = verdict`・`exit = 2` の検体を 1 本 | — | — | — |
| 31 | `fixtures-loop/followup/` の 4 ケース・`judge_followup_selftest.py:130-157` | 赤はクリック透過の FAIL と DPI 1 面の INCONCLUSIVE だけ。`judge_drag` の `if not logged: failures.append(…)`（622-623）を消しても 4/4 緑で、「窓は動いたが本番の経路の `kind=write` 行が無い」変更が PASS する。`dpi_not_followed`・`balloon_drifted`・`missing_probe_evidence`・exit 3 も再現しない。しかも selftest は標準出力しか比べず、`.ps1` が実際に読む `followup-verdict.txt` が壊れても緑 | `drag_fail` ケースを 1 本（all_pass の写しから `kind=write` 行を 1 本消す・exit 1）。selftest に `followup-verdict.txt == stdout` の突き合わせを +3 行 | — | — | — |
| 32 | `fixtures-loop/compare/*/case.txt`（A 側の差がどれも 0.10＝床 0.30 より小さい） | `perf-compare.py:559` の `max(spread, floor)` を床だけに書き換えても 8/8 緑（A=9.00/10.00・B=9.00/9.20 は正しくは NO_DIFF、書き換えると WORSE）。`[goal].judge_version` の食い違いを判定不能にする門（429-435）を消しても緑 | 既にある走行だけでケースを 2 本（`no_diff_by_spread`・`judge_version_mismatch`＝exit 4）。コード 0 行 | — | — | — |
| 33 | `perf-ledger.py:343-353`（`write_text`＝台帳を書く唯一の道）・`:273` | 台帳を書く失敗の道（exit 4）を、どのケースも試さない。`write_text` を一時ファイル＋置き換えでなく直書きに戻しても緑 | ケース `write_fails`（普通のファイル `blocker` の下へ init させて exit 4・約 4 行） | — | — | — |
| 34 | `perf-rank.py:696-706`・検体 `no_dump_unavailable/case.txt:3` | 本番（`perf-loop.measure.ps1:421`）は dump が無いとき必ず `--sampling-available false --unavailable-reason <語>` を渡すが、検体は `auto` しか通さない。`reason = unavailable_reason` が壊れて本番に `reason=None` が出ても緑 | その検体の引数に 2 つを足し、期待の 1 行を `reason=not_elevated` に（±0） | — | — | — |
| 35 | `perf-rank.py:268-279`・`:341-345`・`:360-392`・`:677-681` | `run.log`／`cpu.csv` の exit 4 の道を試す検体が無い（exit 4 の 3 本は全部 dump 由来）。`_need_int` が理由を積まずに 0 を返すよう壊れても緑（「部分的な順位表は出さない」約束が守られない） | `cpu_us` の無い `perf(thread)` 行の検体を 1 本（exit 4） | — | — | — |

**要らないコード・写し（#36〜#41）**

| # | 場所 | 何が起きるか | いちばん小さい直し | perf-run の口 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 36 | `judge-perf.py:2526-2552`・`:2576-2588`・`:2604-2605`・`:2611-2629`・`:2724-2758`（判定式⑴の自動選別）・`vanished_series` `:2378-2396`・`judged_interval_pool` `:1571` の `or list(...)` | `FRAME_INTERVAL_JUDGED_SERIES` は空でない定数（`TargetId(0)/1000`）で、関門（2473-2524）は指定の系列が全部そろうときだけ先へ進むので、自動選別の枝（約 110 行）には決して入らない。定数を書き換えたときしか走らず、検体は 1 本も通らない | 枝を消し、指定が空なら起動の時点で止める assert を 1 行（約 −110 行） | —（版を上げるかは別に決める） | バナーの `FRAME_INTERVAL_EXCLUSION_BLOCKS_PASS` の文と README の説明を縮める | 完了 `areka-P0-recompose-budget` の design.md:618 が「自動選別からの切替」を決めている＝消すのは決めに沿う（⑴に当たらない見込み） |
| 37 | `judge-perf.py:2200-2201`・`:2220-2224`（`_STATUS_ORDER`・`worst_status`） | 呼び出しが 0（ツリー全体を grep）。しかも `worst_status(["適用外"])` は「適用外」を返し、`derive_exit_code` の「全部が適用外なら判定不能」と違う規則を持つ | 両方を消す（−8 行） | — | — | — |
| 38 | `judge-perf.py:4041-4042`・`:4071`・`:4140-4142`（`PLANNED_MODES`） | 中身が `{}` で、choices に何も足さず、4140 の枝には決して入らない（参照はこのファイルの中だけ） | 宣言と 2 つの使い所を消す（−6 行） | —（`--mode` の選択肢は同じ） | — | — |
| 39 | 定常の CPU の絞り込み `judge-perf.py:1600`・`:2108`・`:2117`・`:2971`・`:3072`・`:3271`／定常の catch-up `:1639`・`:2057`・`:2792`・`judge_perf_catchup.py:133` | 同じ「定常状態の CPU」「定常状態の catch-up」を 6 か所・4 か所で別々に絞る。perf-compare の採否と GOAL_MET が使う `steady_idle_cpu_mean_pct`（1600）と⑷a の平均（3072）が別の写しなので、片方の境目だけ変えると、ループが下げている数と最終判定が見ている数が黙ってずれる | `aggregate()` で `steady_cpu`・`steady_catchup` を 1 回だけ作って `Aggregation` に持たせる（約 −4 行） | —（数値は変わらない） | — | — |
| 40 | ログ行を読む部品の写し: 元 `judge-perf.py:754-822`（ANSI・時刻・`parse_fields`・`split_after_message`・`read_text_lines`）・`:1122-1164`（`read_cpu_csv`）・`:1209-1213`（`percentile`）／写し `perf-rank.py:146-211`・`:358-393`・`judge-followup.py:132-209`・`perf-compare.py:311-327`・`fixtures/generate.py:260,284` | 規則を 1 か所だけ直すと、残りが黙って古い規則のまま動く（例: #7 の NaN の直しを judge-perf にだけ入れると、同じ `cpu.csv` を perf-rank は NaN のまま順位表に載せる）。写しが元と同じかを見るテストは無い | 判定式も較正値も持たない兄弟モジュール（例 `perf_lines.py`）に 1 度だけ置き、各道具が import（約 −120 行・judge-perf.py は縮む）。残すと決めるなら、写しの正規表現が元と同じことを見る較正を 1 本 | — | Rust の写し（`crates/wintf/src/ecs/window/transition_diag_tests.rs`・`crates/areka/src/placement_tests/transition_judge.rs` ほか 10 ファイル）は「同じ規則」と書くだけで Python に依存しない＝触らない。`perf_thread_report.rs:35`・`perf_thread_report_tests.rs:15` の注記は名前で指す＝ずれない | **⑴の可能性**: `perf-compare.py:57-63` と `perf-rank.py:36-39` が「import しない（同じ周に別の担当が判定スクリプトを触ると 2 つの道具が同時に倒れる）」を完了 `areka-P0-draw-load-parity` の設計 C10 の決めとして書く（要件ではない・共通モジュールは較正値も判定式も持たないので、その心配には当たらない）。議題 4 |
| 41 | 自己較正の支え: `judge_followup_selftest.py:90-165`・`perf_compare_selftest.py:150-295`・`perf_ledger_selftest.py:128,148,222`・`perf-rank.py:727-884` | ケースの読み方・`_invoke`・`_first_difference`・赤の下限の確かめ・一時フォルダが 4 組ほぼ同じ形。#1 の直しを 4 か所で書く必要がある（一緒に変わるべき写し） | 共通の兄弟モジュール 1 本にまとめ、道具ごとの違い（期待出力の突き合わせ）だけを関数で渡す（約 −120〜−200 行） | —（`SELFTEST RESULT` 行と終了コードは同じ。`perf-loop.ps1:399` は自分で数える） | — | — |

### 3.3 Nice to have（40 件）

| # | 場所 | 何が起きるか | いちばん小さい直し | perf-run の口 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 42 | `judge-perf.py:3971-3981`・`:182`・`:725-727`（`SELFTEST_REQUIRED_RED_CODES`） | 赤の関門は期待 1 と 2 しか見ないので、期待 0（合格の陽性対照）と 3 の検体が 0 本でも緑（例: exit=0 の 7 本を `_` で始まる名にすると黙って集められず、それでも exit 0）。README:394 は「合格・不合格・判定不能・引数の誤りも毎回再現」と書く | 4 つの値すべてを求める形にし、定数と assert を消す（約 −8 行） | — | README:397-399 の文 | — |
| 43 | `judge-perf.py:4050-4056`（`Parser.error`）・`:3413-3414`（`--meta` の不在）・`:3432-3437`（build の食い違い） | argparse の誤りの exit 3、`--meta` の不在、run-meta の build の食い違いに検体が無い。Parser の上書きを外すと引数の誤りは argparse の既定の 2（＝判定不能）に変わるが緑。本番は必ず `--meta` を渡す（`perf-loop.measure.ps1:335`）のに、検体は自動で探す道しか通らない | `build = debug` の検体を 1 本（exit 3・コード 0 行） | — | — | — |
| 44 | `judge-perf.py:3582`（集計モードの注意書き）と `:2972`（収束の条件） | 注意書きは全区間の点が 8 未満のときに出るが、収束が求めるのは定常の点 12 以上。全区間 10 点・定常 10 点の走行では注意書きが出ないのに verdict は判定不能 | 3582 を定常の点数と `CONVERGENCE_MIN_SAMPLES_PER_WINDOW * CONVERGENCE_WINDOW_COUNT` で比べる（±0） | — | — | — |
| 45 | `judge-perf.py:3832-3838` | 自己較正が食い違ったとき、子の stderr にあった理由を捨てる（「不一致 X1 期待 3 実際 2」だけ） | stderr の最初の行も添える（+2 行） | — | — | — |
| 46 | `judge-perf.py:3763-3774` | `case.txt` の知らない鍵を黙って無視する（`contain = …`・`emit_metric = yes` と書くと、その検査が知らせなく消える） | 知っている鍵の外なら exit 3（+2 行） | — | — | — |
| 47 | `judge_perf_catchup.py:209-212`・`:141`（`_timedelta_seconds`） | 標準ライブラリの `timedelta(seconds=…)` を包むだけ | 包みを消して直に使う（−4 行） | — | — | — |
| 48 | `judge-perf.py:2509-2771`（判定式⑴の `FormulaResult` 6 か所） | tag・title・applies_to・threshold・calibration・empty_behavior が毎回同じ値で並ぶ。題を変えると 6 か所（#36 の後でも 4 か所）を直す | `functools.partial(FormulaResult, …)` を 1 つ（約 −25 行） | — | — | — |
| 49 | `judge-perf.py:2680-2681` | 観測集合の行が、除外した系列の長い注記を根拠の欄と 2 重に出す（実走 `final-20260823/*/verdict.txt:219` で 9 系列の全文が 1 行に詰まる） | 観測集合は「判定対象から外した系列 N 本（根拠に列挙）」だけに（±0） | — | — | — |
| 50 | `judge-perf.py:1511-1523`（`catchup_analysis` の、agg に属性を後から足すキャッシュ） | 入力が同じなら同じ結果・1 走行 2 回しか呼ばれない。キャッシュは何も足していない | キャッシュを外す（−4 行）。#39 で `Aggregation` に欄を足すならそこへ | — | — | — |
| 51 | `judge-perf.py:1124,1137`・`perf-rank.py:364,373` | 空行を先に捨ててから行番号を振るので、2 行目が空・4 行目が壊れた `cpu.csv` を「3 行目」と報告する（手で直した csv だけ） | 番号を振ってから空行を飛ばす（±0） | — | — | — |
| 52 | `judge-followup.py:75`（`DRAG_DX_PX = 80`）と `invoke-followup-checks.ps1:114` | ドラッグの距離を 2 か所で決め打つ。`.ps1` だけを 120 にすると、全走行が「移動量が 80±2px から外れています」で FAIL。`probe.log` の `step=gesture` 行はもう `dx=` を書いている | 判定側は `probe.log` の `dx`／`dy` を期待値として読み、無ければ判定不能。定数を消す（+2 行） | —（`.ps1` は変えない） | — | — |
| 53 | `judge-followup.py:625`・`:616` | 決して使われない `or` の右（ループを抜けた時点で両方そろう・None を外した後の `or 0`） | `after[...].hwnd` に、ax は 1 度だけ変換（±0〜−1） | — | — | — |
| 54 | `perf-ledger.py:484-488` | `set-phase X --best nan` が通り、`state` が不正な JSON（`NaN`）を出し、`status` は exit 3。今 nan を出す道具は無い | `math.isfinite` で弾く（+2 行） | — | — | — |
| 55 | `perf-ledger.py:651`・`perf_ledger_goal.py:438` | 目標定義の値の型が違う（`spec_dir = 5`・`idle_cpu_release_max_pct = "3.0%"`）と生の traceback で exit 1（この道具の 1 は「自己較正の食い違い」の意味） | 2 か所で型を確かめて exit 3（+3 行） | **変わる**（その場合の終了コード 1→3）。ただし `perf-loop.ps1:523-525` は 0 かどうかしか見ない＝読み方は変わらない。知らせはする | — | — |
| 56 | `perf-ledger.py:336`・`:619-620` | BOM 付きの台帳は「台帳の先頭は '## 状態'…」で exit 3、BOM 付きの TOML は `tomllib` が読めない。PS 側（`Get-Content -Encoding utf8`）は BOM を受けるので、preflight は PS では通り goal-check だけ落ちる | `encoding="utf-8-sig"`（±1 行）。`perf-compare.py:270` も同じ形 | — | — | — |
| 57 | `perf-ledger.py:348-349` | 一時ファイル＋`os.replace` は正しいが、書いた直後に電源が落ちると空の台帳が残りうる。台帳は周ごとにコミットされないので戻す先が無い | flush → `os.fsync` → replace（+3 行） | — | — | — |
| 58 | `fixtures-loop/ledger/{goal_check_missing_key,goal_check_ok,goal_check_threshold_mismatch,goal_check_version_mismatch,goal_text}/judge-stub.py`・`goal_check_ok/goal.toml` と `goal_text/goal.toml` | 同じ中身の偽の判定スクリプトが 5 本（同じ blob）、同じ中身の `goal.toml` が 2 本 | 1 本を `fixtures-loop/ledger/_shared/` へ（`_` で始まるフォルダは `perf_ledger_selftest.py:212` が読み飛ばす）、ハーネスに `@SHARED@` の置き換え（+2 行・約 −56 行と −62 行） | — | — | — |
| 59 | `perf_ledger_goal.py:34-40`（`bind(CORE)`）と `perf_ledger_selftest.py:53-67`＋`perf-ledger.py:878-890`（`SelftestEnv`） | 本体を兄弟に渡すやり方が 2 通り。ハーネスに定数を 1 つ渡すたびにデータクラスと組み立ての 2 か所を直す（`bind` 自体は要る＝本体の名にハイフンがあり、importlib で読み直すと例外のクラスが 2 つになる） | selftest も `bind(core)` に（約 −20 行） | — | — | — |
| 60 | `perf_ledger_goal.py:58`（`formulas`）・`:65`（`main_model_recommended`）・`goals/draw-load-parity.toml:48`・`:118`・検体の `goal.toml` 5 本 | 必須の鍵なのに誰も読まない（`tools/`・`.claude/` を grep して読むのは GOAL_SCHEMA だけ）。`formulas` から `2_catchup_zero` を外しても最終判定は何も変わらないのに、ファイルの上では判定式を選べるように見える | 2 つを GOAL_SCHEMA と toml から消す（約 −14 行） | — | — | 完了 `areka-P0-draw-load-parity` の design.md:439・473 の例に載る（要件・裁定ではない＝⑴に当たらない見込み） |
| 61 | `perf-rank.py:704`・`perf_rank_dump.py:249` | dump を丸ごとメモリへ読み、`replace` と `split` で 3 倍に膨らむ。見積もり（8 論理 CPU × 8.19 kHz × 420 s ＝約 2,750 万行＋Stack 行）で数 GB、MemoryError の traceback（exit 1）。Idle のサンプルが本当に記録されるかは確かめていない | 1 行ずつ読む（約 ±3 行・#3 と同じ所） | — | — | — |
| 62 | `perf-rank.py:687`・`:699`・`:706` | `auto` で 0 バイトの `dump.txt` があると `[3] 関数 UNAVAILABLE reason=no_dump`・exit 0（本来は計測失敗＝README:903-906 で exit 4） | `dump_present = dump_path.is_file()`（±0） | — | — | — |
| 63 | `perf-rank.py:231-241`・`:322-326` | 使っていない `perf(process)` の欄（t_s・wall_ms・kernel_us・user_us・threads）を必須にしている。`talk-thread-count-growth` が `threads=` を変えると、使ってもいない欄のために全行で exit 4 | 5 つの欄と必須の確かめを消す（約 −5 行） | — | — | —（§7 の重なり） |
| 64 | `perf_rank_dump.py:231`・`:247`・`:308-309`・`:342`（`DumpScan.tids`） | 読む所が 0（grep）。`sample_ok_counts/case.txt:6-7` の「ThreadID も数え直す」は rank.txt に出ないので確かめていない。しかもリストの探索が行ごとに走る | 消して case.txt の注記を直す（約 −5 行） | — | — | — |
| 65 | `perf-rank.py:709` | `perf-rank.py .` で回すと `run=` が空になり、同じ走行をフォルダ名で渡すと `run=<名前>`（同じ入力で 1 行目が変わる） | `run_dir.resolve().name`（±0） | — | — | — |
| 66 | `perf-rank.py:498`・`:612`・`:706`（`no_perf_thread_lines`・`no_tick_lines`・`no_dump`・`declared_unavailable`） | rank が自分で出す UNAVAILABLE の理由語がどこにも載っていない（README:901-904 と `perf-loop.ps1:44-50` は段③の `.ps1` の語だけ）。`[tick]` の文言が変わった走行は `[4] 相 UNAVAILABLE reason=no_tick_lines`・exit 0 になり、係が書く形が無い | README に 1 行（エージェント定義にも 1 行ずつ） | — | `.claude/agents/perf-measure.md:56-58`・`perf-analyze.md:68` | — |
| 67 | `perf-rank.py:688-695`・`:911`（`--sampling-available true`） | 使う呼び手がいない（`measure.ps1:417,421` は auto と false だけ）。true を使うのは検体 `unresolved_dump` だけで、auto でも同じ結果 | 選択肢と分岐を消し（約 −7 行）、検体を auto に | **変わる**（引数の選択肢が 1 つ減る。`.ps1` は使っていない）。要件 6.3 の議題 | — | — |
| 68 | `fixtures/generate.py:851-878`・`fixtures/H1b_unpinned_slow_series/`・`README.md:344,411` | H1b は H11 と同じ分岐しか通らない重複の検体（H11 も指定外の系列を上限超えの 180ms で置く。指定外の系列は速さによらず `judge-perf.py:2572` の 1 分岐へ行き、H11 を緑・H1b を赤に分ける誤りは作れない） | `case_H1b` と検体を消し、README の 2 か所を H11 に向ける（約 −29 行・検体約 108 KB 減） | — | — | **⑴の可能性**: docstring 31-43 行が「task 3.2 の裁定」の「既知の代償を記録として固定」を引く。記録の置き場を H11 に移すか。議題 5 |
| 69 | `fixtures/generate.py:656-662`・`:702`・`:732-733` | case の note が事実と合わない（C1「H11 と同じ健全な走行」だが 1001 系列が無い／「無い側は既存の 17 件すべて」は 19 件で C2 は `[tick]` を持つ／C2「catch-up の 3 件だけを移した」は他も落としている／T2 は CPU の山と baseline の mode を書かない） | 3 つの note を直して作り直す（±0） | — | — | — |
| 70 | `fixtures/generate.py:505-509`・`:1238-1239` | 端末へ日本語を出す（Git Bash から回すと cp932 で化けた・SystemExit の文も日本語） | ASCII の英語の文に（±0） | — | — | 議題 3 と同じ扱い |
| 71 | `fixtures/generate.py:1234-1240`・docstring 10-13 行 | CASES から 1 件外して作り直しても古いフォルダは消えず、`git status` もきれいなまま（docstring の「汚れなければ定義どおり」が成り立たない）。`judge-perf.py:3814` は全フォルダを読むので、作り手の無い検体を回し続ける | 作ったもの以外のフォルダがあれば名を出して止める（+3 行・#68 か #72 の削りと相殺） | — | — | — |
| 72 | `fixtures/generate.py:805-812` と `:777-783` | H12c の組み立てが H12 の写し（片方だけ直すと双子の突き合わせが赤になる＝捕まるが手で揃える） | `case_H12()` から作って名前と色付けだけ差し替える（約 −4 行） | — | — | — |
| 73 | `fixtures/generate.py:495-510`（`write()`） | 先にフォルダを消し、色付けの等価の確かめを後にする。確かめが落ちると H12c のフォルダが空のまま残り、selftest が「case.txt がありません」で exit 3（git から戻せる） | 確かめを `rmtree` の前へ（±0） | — | — | — |
| 74 | 古い行番号の参照: `judge-perf.py:40`・`:95-96`・`:503`・`:506`・`:512-513`・`:523-525`・`:569-579`／`judge-followup.py:107`・`:109-110`／`perf-compare.py:79`・`:99`・`:103`／`fixtures/generate.py:195`／範囲外 `crates/areka/src/placement_tests/transition_judge.rs:22`（`judge-perf.py:562,588-596`＝今は 763・789-797）・`perf-loop.common.ps1:188`（`perf-ledger.py:930-932`＝今は 968-970）・`perf-loop.ps1:510`（`perf_ledger_goal.py:430-447`） | 番号をたどった人は別の行に着く（例 `perf-compare.py:79` の `judge-perf.py:715`＝今は 721 の `EXIT_FAIL`）。`judge-followup.py:107` は「変えるときは必ず出所を確かめること」と言いながら指す先が違う。棚卸㉙で直した番号がまたずれている。`perf-loop.common.ps1:465` の `judge-perf.py:150` だけは正しい | 行番号を消し、ファイル名と関数名・定数名・語で指す（±0） | —（`.ps1` の 2 か所はコメントだけ。直すなら要件 6.2 の申し合わせ） | `transition_judge.rs:22`（要件 5.4）・`perf-loop.common.ps1:188`・`perf-loop.ps1:510`（`audit-tools-perf-run` の持ち物） | — |
| 75 | `judge-perf.py:109-111` | 説明文が「`invoke-perf-run.ps1` は `NO_COLOR` を設定しない」と書くが、実際は設定している（`invoke-perf-run.ps1:167,774-780`・README:506-509 もそう書く） | README と同じ趣旨に直す（±0） | — | — | — |
| 76 | `README.md:258` | 「較正値は `SCRIPT_VERSION` から `PERCENTILE_METHOD` まで」と書くが、バナーは `J_DPI_SHOW_FIELDS`（716 行の前）まで続き、`CATCHUP_TICK_LOAD_RATIO_MIN`（644）などを見落とす | 範囲の終わりを直す（±0） | — | — | — |
| 77 | `judge-perf.py:538` | 逐語の見本のコメントが 1 行に潰れ、行の途中に `#:` が混ざる（写すと `J_CATCHUP_LOOP_MESSAGE` に一致しない文になる） | 2 行に分け直すか 608 行を指す（±1） | — | — | — |
| 78 | `README.md:403-423`（§9 の検体の表） | 17 行のままで C1・C2・T2 が無い。`case.txt` の `contains =` と `emit_metrics = yes` の仕組みにも触れない | 表に 3 行と 1 文（+4 行） | — | — | — |
| 79 | `README.md:138-140` | 「バルーンはゴースト一式のルートの下にはなく `<根>\balloon\<名前>` へ分けて展開する」は古い。今の窓口はゴーストのフォルダの中（`crates/sample-ghost-kit/src/lib.rs:79`・spec `areka-P0-ghost-inner-balloon`） | 理由の括弧を「窓口の値をそのまま渡す」に（−1 行） | — | — | — |
| 80 | `README.md:194`・`:1002-1003` | §6 の採取の表は 2 を「`-ConfirmQuiet` 欠落」だけと書くが、`-AutoQuiet` で静かにならなかったときも 2（`invoke-perf-run.ps1:653`）。§15 の「このスクリプト自身は待ちも再試行もしません」は、`check-quiet.ps1:309-324` の 3 回の読み直しと README:93 に食い違う | 表に 1 句・§15 に 1 句（±0〜+1） | — | — | — |
| 81 | 文書の小さな食い違い: `goals/draw-load-parity.toml:80`（「STOPPED reason=max_iterations」＝実際の語は `iteration_cap`。注釈どおり打つと exit 3）・`perf-ledger.py:132`（「init が作るのもこの 10 行」＝今は 13 行）・`:181-182`（「9.4 で追補すること」＝追補済み）・4,000 字の境目（要件 1.6 と `goal.md:36` は「以内」、`perf_ledger_goal.py:521` は `>= 4000` で拒む）・`perf_ledger_goal.py:223`（STOPPED 行が崩れても GOAL_MET の見本を出す）・`:117`（`NOT_AVAILABLE` が `CORE.EMPTY_VALUE` と重複）・`:555-556`（余分な空行）・`judge_perf_catchup.py:5`（「3,500 行」は古い数） | どれも 1 行で直る | 各 1 行（±0） | — | — | — |

- Lean（消せる行の見込み）: **約 −500〜−650 行**（#36 約 −110・#40 約 −120・#41 約 −120〜−200・#58 約 −118・#21・#68 ほか）・**依存 −0**（もともと標準ライブラリだけ）。`judge-perf.py` は差し引きで縮む（#13・#36・#37・#38・#40・#42・#48 で減り、#2・#6・#7 で少し増える）。`fixtures/generate.py` は #2・#28〜#30 の検体を足すと増えるので、置き場は設計で決める（§6・議題 6）。
- Verdict: まず #1（一時フォルダ）・#2（発話の頂の語彙）・#3・#4（順位表の段③と段②の誤った数）。続いて採否を誤らせる入力の穴（#7・#10・#11）と台帳の食い違い（#14・#15）。

## 4. 指摘にしなかったもの（確かめた結果）

### 4.1 判定と台帳の芯

- **終了コードの決め方**（`derive_exit_code`）: 不合格 → 判定不能 → 合格 → 全部が適用外なら判定不能の順。0 を返すのは合格が 1 つ以上あり、不合格も判定不能も無いときだけ。H14 が「不合格が優先」を押さえる（読み手 B）。
- **172.5 の境界**: 実測と上限の両方を小数 3 桁へ丸めてから `<=` で比べ、印字と判定は同じ値から出る。H12／H13 が押さえる。
- **分位点**（`ceil(q·n)`）: q を 7 通り・n を 1〜100,000 で Fraction と突き合わせて食い違い 0。空の列は呼び出し元 7 か所が先に弾く（読み手 A・B）。
- **0 で割る・空の列**: dev の枝は `if steady`、release の枝は `VERDICT_MIN_CPU_SAMPLES=5`、`split_windows` は 12 点以上で空の窓ができない、`check_required` が CPU 0 点を 2 にする（読み手 C）。
- **IDLE の比べ**: `mean < 3.0` は狭義でバナー・README と合う。
- **時刻**: ログも `cpu.csv` も UTC の `Z` 付きで、タイムゾーン付きで比べる。日付をまたいでも問題ない。
- **ANSI の除去**: tracing の色付けは全部落とせる。H12c が双子の比べで本文の一致まで見る。
- **`SCRIPT_VERSION` の字面**: `perf-loop.common.ps1:469-472` の正規表現は `0.4.0` を取り出し、`judge-perf.py:1655` の metric も同じ（#16 は将来の形での食い違い）。
- **バナーの定数**: 全部どこかで使われている（定数ごとに grep）。
- **台帳の書き込みの順と原子性**: 一時ファイルへ書いて `os.replace`・失敗すれば一時ファイルを消して exit 4。set-phase／append／goal-check は検査を全部終えてから最後に 1 回だけ書く。知らない鍵・同じ鍵の二重・同じ周の二重は行番号つきで exit 3。CRLF も読む。実物の台帳（完了 spec の `loop-ledger.md`）で state も status も通る（読み手 E）。
- **トークンの乱数**: `secrets.randbelow`。目的は「見本と実出力を字面で分ける」だけ。`00000000` は作らない。
- **遷移表とスキル**: 相の名（11）・出来事（17）・サブコマンド・引数・FINAL の reason の 4 語は `SKILL.md` と全部一致。STATUS／FINAL 行を読む `.ps1` は無い。
- **`GOAL_TEXT_TEMPLATE` と写し**: `perf-loop.ps1:261-352` が本文と字数（1,012 字）を突き合わせ、自己較正は緑。もう 1 つの写し（`fixtures-loop/ledger/goal_text/expected_stdout.txt`）も検査なので残す。
- **perf-compare の数の端**: 4 本が同じ値なら NO_DIFF。delta がちょうど −noise なら ADOPTED、+noise なら WORSE（docstring どおり）。4 本のうち 1 本が判定不能なら MEASURE_FAILED。perf-compare は run.log／cpu.csv を自分では読まず、judge-perf を別プロセスで呼ぶ（写しは `parse_fields` だけ＝#40）。
- **`CHECK_ALL`・`FOLLOWUP VERDICT`**: 3 か所とも同じ 4 語・同じ順。`FOLLOWUP VERDICT` 行は全部 ASCII で、`invoke-followup-checks.ps1:857-861` がファイルから読み、`perf-loop.measure.ps1:918-920` は中止の形だけを見る。採否の 4 語は `perf-ledger.py:166`・遷移表・`SKILL.md:210`・`templates/entry.json`・`agents/perf-measure.md:71` と綴りまで一致（読み手 D）。
- **thread 行の規則（Rust と照らした）**: 欄の名と並び・空白を `_` に潰す規則・`=` を含む値・`unregistered_rest` の綴り・差の取り方（TID で照らす・新しい TID は全量・消えた TID は落とす・カーネルとユーザーを別々に 0 で止める）は全部一致（#4 は TID の使い回しという別の場合）。段④の `SCHEDULE_NAMES` は `tick_diag.rs:71-85` と同じ。順位表の並べ方は同順位まで鍵を持ち、浮動小数の書式はロケールに依らず、改行は `\n`（#65 を除き決定論）（読み手 F）。
- **自己較正の分け方**: `*_selftest.py` は 120〜250 行の本体を持つ本物の分け方で、素通しの層ではない（本体へ戻すと 1,000 行を超える）。`judge_perf_catchup.py`・`perf_rank_dump.py` も仕事で分かれている。`bind(CORE)` は要る（#59 は selftest 側の重複だけ）。
- **大きい run.log**: 40 MB の擬似ログで 3.8 秒・最大約 200 MB（想定の中・指摘にしない）。
- **検体の決定論**: `generate.py` は乱数・実時刻・辞書の順に頼らず、作り直しでバイトが一致。`.gitattributes` の `* -text` は `git check-attr` で効いている。E3・X1・X2 の run.log はバイトで同じだが試す道が違う（純粋な重複は #68 だけ）。

### 4.2 範囲の外の観察（直す所が範囲の外だけ・番号を付けない）

`.ps1` は `audit-tools-perf-run` の持ち物、`.claude/` と `crates/` は他の見直しの持ち物として、ここには記録だけを残す（要件 5.6）。

- `invoke-cpu-sample.ps1:248-317` の `Measure-ArekaFrames` も、他のプロセスを捨てず（#3）・未解決の areka フレームを数え（#23）・dump を丸ごと読む（#61）。
- `perf-loop.measure.ps1:416-421`: `sampling.txt` が available=true のまま `dump.txt` が消えた走行を `rank` で作り直すと `--unavailable-reason ok` を渡し、`UNAVAILABLE reason=ok` が出る。
- `perf-loop.common.ps1:61` は LOCALAPPDATA が無いと出力先を TEMP へ落とす。
- `.claude/agents/perf-analyze.md:39` のカタログが、順位表に出ない役割名 `taskpool` を引く（TaskPool の CPU は `unregistered_rest` と [2b] にしか出ない）。直す所はエージェント定義だけ。
- `.claude/skills/perf-loop-iteration/SKILL.md:52` は「台帳が無い」を日本語の stderr の語で見分けるので、壊れた台帳でも周 0 へ進む（init が上書きを拒むので記憶は消えない）。
- `crates/areka-placement/src/transition_diag_tests.rs:48-53` は「judge-perf.py と同じ規則」と書きながら `split_whitespace` で切る（今の呼び手は空白を含む値を読まない）。Python の `\s` は全角空白にも当たり Rust の `is_ascii_whitespace` は当たらないが、`crates/` に全角空白の直後に `名前=` が来るログの文は 0 件。
- `perf-loop.ps1:472-474` の PS 側の版の確かめと goal-check の版の確かめが二重（#16 は読み方を揃えるだけ）。
- wpaexporter の道は `wpa\cpu-sampled.wpaProfile` がリポジトリに無いので exit 3 で止まる。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今のコード・道具 | ギャップ |
|---|---|---|
| 1（洗い出し） | 本書 §1〜§4 | 済み（81 件・284 ファイル全部）。1.5 の書き加えは討議の後 |
| 2（全部直す） | 指摘の場所は全部 `tools/perf/` の中（範囲外へ及ぶものは各行の「範囲外」の欄） | **Missing**: 81 件の直し。**Constraint**: #2・#28〜#30 の検体は `generate.py`（1,244 行）を増やす（要件 7.1）＝置き場が要る |
| 3（裁定で残す） | ⑴の候補 #24〜#26・#70（端末の日本語）・#40（import しない決め）・#68（H1b の記録）。⑵ 0 件。⑶ 0 件 | **Missing**: 討議で裁定（§7） |
| 4（直した後の確かめ） | 自己較正 `perf-loop.ps1 selftest`（今 緑 9/9・43 秒）・全体テスト `tools/test-all.ps1`（全体テストは Python の自己較正を回さない） | 直したバグ・危ない所（#1〜#27）に 1 本ずつ、壊れたら落ちる検体か自己較正の確かめを置く。#28〜#35 はテストそのものを足す指摘。かけ直しの audit は検証のタスクで |
| 5（範囲の外） | 範囲の外に及ぶのは #2（kanade の語を読むだけ）・#3（README の文）・#4（2 つ目の案なら Rust）・#15（スキルの注意書き・任意）・#21（`perf-measure.md`）・#26（README）・#40（Rust の注記は名前で指す＝触らない見込み）・#66（エージェント定義 2 本）・#74（`transition_judge.rs:22`・`.ps1` 2 か所） | **Constraint**: `.ps1` の 2 か所（#74）は要件 6.2。Rust の `transition_judge.rs:22`（本体 `crates/areka/` の中）は、そのファイルを受け持つ見直しの spec が C20 で走っているかを設計の段で roadmap の台帳で確かめ、走っていれば申し合わせる（要件 5.2） |
| 6（C20 の並走の約束） | perf-run の口に触るのは #6（終了コード 1→3）・#55（終了コード 1→3・読み手は 0 だけ見る）・#67（引数の選択肢）＝**3 件**。「文面」（LastLine の字面）は #24・#25・#26 と #5（終了コードは道具が書いた走行では変わらない）。`CHECK_ALL`・`SCRIPT_VERSION` の字面を変える指摘は 0 件 | **議題**（要件 6.3・§7 議題 1）。依存は足さない（標準ライブラリだけで全部直る）・`Cargo.lock` は変わらない・`vendors/` は触らない |
| 7（守る決まり） | #1 で一時フォルダを `target\` の下へ。#12・#18・#45 で記録の無い失敗を塞ぐ。#24〜#26・#70 で端末の文 | **Constraint**: `judge-perf.py` と `generate.py` を増やさない（7.1）。`README.md` 1,178 行が 1,000 行を越えている＝文書を数えるかは議題 7。GPU のテストは足さない |
| 8（大きさ） | — | §6 の見込みで 17〜20 本＝上限すれすれ（議題 8） |

## 6. 実装の進め方の案

- **案 A（今のファイルの中で直す）**: 指摘ごとに今のファイルを直し、写し（#40・#41）は写しのまま揃える（揃っていることを見る較正を足す）。検体は手書きの検体として `fixtures/` に足す（`generate.py` を通さない）。
  - ✅ 新しいファイルが無い・読む口が変わらない。❌ #40・#41 の「一緒に変わるべき写し」が残る（Lean の指摘が直らない＝全部直す方針に合わない）。手書きの検体は `generate.py` の「作り直して `git status` が汚れなければ定義どおり」の約束の外になる。
- **案 B（共通の兄弟モジュールを 2 つ置く）**: `perf_lines.py`（行の読み方・`cpu.csv`・分位点）と `perf_selftest_common.py`（自己較正の支え・一時フォルダを `target\perf-loop-tmp\` に置く 1 か所）を足し、5 本の道具と 4 本の自己較正から import する。`generate.py` の検体の組み立ての一部（新しい検体と、long／dev の CPU の作り方）は兄弟の生成モジュール（例 `fixtures/generate_long.py`）へ置き、`generate.py` は呼ぶだけにして行を減らす。
  - ✅ 写しが 0 になる・#1 が 1 か所で直る・`judge-perf.py` と `generate.py` が縮む。❌ ファイルが 2〜3 増える・「import しない」の決め（#40）を討議で覆すかどうかが前提になる。
- **案 C（組ごとに進める）**: 本番の振る舞いに関わる直し（#1〜#27）を先に、テストの足し（#28〜#35）をその後に、要らないコードと写し（#36〜#41・Nice）を最後に。
  - ✅ 判定の振る舞いを変える直しと、寄せ替えが混ざらずレビューしやすい。❌ 寄せ替えの段で、先に足したテストや直しをまた動かすことがある（#40 の後に #7・#13 の直しの所が移る）。
- **規模の見込み**: M（3〜7 日の相当）。タスクは **17〜20 本**の見込み（例: #1＋#41 の支え／#2 の語彙と検体／judge-perf の入力と終了コード〔#5〜#9・#42〜#46〕／judge-perf の要らないコード〔#36〜#39・#47〜#51〕／#40 の共通モジュール〔#7・#13 を含む〕／判定の検体〔#28〜#30・#68〜#73・`generate.py` の分け方〕／端末の文〔#24〜#26・#70〕／perf-compare〔#10〜#12・#32〕／judge-followup〔#31・#52・#53〕／台帳の本体〔#14・#15・#19・#20・#33・#54〜#57〕／台帳の判定面〔#16〜#18・#58〜#60〕／順位表の dump〔#3・#23・#61・#62・#64〕／順位表のスレッドと引数〔#4・#21・#22・#34・#35・#63・#65〜#67〕／文書〔#27・#74〜#81〕／かけ直しの audit／全体テストと自己較正の確かめ）。**20 本の上限すれすれで、かけ直しの audit で指摘が出ると越えうる**（要件 8 の議題）。
- **危うさ**: 中。判定の芯は検体で守られているが、#2・#3・#4 は実走の数字の読み方を変え（過去の `results/` と数が変わる）、#3・#23・#61 は実機の xperf の dump で確かめていない。#40 は 5 本の道具の読み方を一度に動かす。

## 7. 討議に渡す議題の種（要件 2・3・6）

1. **perf-run の口に触る 3 件（要件 6.3）**: #6（想定外の例外の終了コード 1→3）・#55（目標定義の型の誤りの終了コード 1→3・`.ps1` は 0 かどうかしか見ない）・#67（`--sampling-available true` を消す＝引数の選択肢）。「`audit-tools-perf-run` のセッションと申し合わせて C20 のうちに直す」か「あちらの着地の後にこの spec のうちで直す」か。推す案＝申し合わせて C20 のうちに（どれも `.ps1` の側の直しは 0 行で、読み方が変わらないことを伝えるだけ）。あわせて「文面」の #5・#24・#25・#26（`.ps1` が LastLine を埋めるだけ）は知らせるだけでよいか。
2. **#74 の範囲外の行番号**: `perf-loop.common.ps1:188`・`perf-loop.ps1:510` のコメントは `.ps1`（要件 6.2）、`crates/areka/src/placement_tests/transition_judge.rs:22` は Rust（要件 5.4）。こちらの範囲の側だけを直し、`.ps1` の 2 か所は `audit-tools-perf-run` へ伝えて向こうで直してもらう形でよいか。
3. **端末の日本語（#24・#25・#26・#70）は⑴に当たるか**: README §11（579 行）は「道具が自分で端末へ出す文は ASCII だけ」と書く一方、README:583-585 と完了 `areka-P0-perf-tools-console-encoding` の design.md は「写した子の中身（`judge-followup.py` の日本語の報告など）」の日本語を認めている。案 (a) Python の道具の最後の行と stderr の失敗の文だけを ASCII にし、報告の本文は日本語のまま（#24・#25 の最小の直し・#26 は stderr の文だけ）。案 (b) 報告の本文もファイルだけに書き、端末へは ASCII の判定行だけ（#26 の直し）。案 (c) ⑴として残す。推す案＝(a)（記憶「道具が端末へ出す文は ASCII だけ」に合い、完了 spec の決めを覆さない）。
4. **#40（行の読み方の写し）は⑴に当たるか**: `perf-compare.py:57-63`・`perf-rank.py:36-39` の「import しない」は完了 `areka-P0-draw-load-parity` の設計 C10 の決め（要件・裁定ではない）。推す案＝共通モジュール（判定式も較正値も持たない）に寄せる（決めの理由＝「判定スクリプトを触ると 2 つの道具が同時に倒れる」に当たらない）。残すなら、写しの正規表現が元と同じことを見る較正を足す。
5. **#68（H1b の検体）は⑴に当たるか**: `generate.py` の docstring が「task 3.2 の裁定」の「既知の代償を記録として固定」を引く。推す案＝記録は H11 の note と README へ移して H1b を消す（裁定の中身＝「指定外の遅い列は合格を止めない」は H11 で同じく固定される）。残すなら Lean の指摘を 1 件残すことになる。
6. **検体の置き場と `generate.py` の 1,000 行**（要件 7.1）: #2・#28〜#30 の検体を足すと `generate.py`（1,244 行）が増える。案 (a) 新しい検体の組み立てを兄弟の生成モジュールへ置く（`generate.py` は呼ぶだけ・#68・#72 の削りで行を減らす）、案 (b) 手書きの検体（`generate.py` の外）。推す案＝(a)（作り直して一致を確かめる約束を保つ）。
7. **`README.md` の 1,178 行**: 1 ファイル 1,000 行の決まり（要件 7.1）は番人が `.rs` だけを数え、要件の表も越えているファイルを `judge-perf.py` と `generate.py` の 2 本と書く。文書を数えないと読んでよいか（推す案＝数えない・要件の表どおり。分けるなら 13 節以降の自走ループの節を別ファイルへ）。
8. **spec の大きさ（要件 8）**: 見込み 17〜20 本で上限すれすれ。推す案＝1 本のまま進め、タスク生成で 20 本を越えたら Nice（#42〜#81）を同じ範囲の 2 本目へ切る。もう 1 案は今のうちに「順位表と台帳（#3・#4・#14〜#23・#33〜#35・#54〜#67）」を 2 本目へ切る。
9. **他の spec との重なり**: #4・#22・#63 は未着手の `talk-thread-count-growth`（名簿の終わったスレッドの扱い・`threads=`）と重なる。#4 は perf-rank の中だけで直す案（名簿外の残りをプロセスの差から引いて出す）を取れば Rust に触らない。#22 の畳み方をこちらで決めるか、あちらへ渡すか。推す案＝どれもこちらで perf-rank の中だけで直し、あちらの brief に一言残す。
10. **#2 の語彙の足し方**: `sstp_notify_talk` など足す語の始まりと終わりが対になるかは kanade の持ち場の事実（読み手 A が未確認）。設計の段で kanade のソースを読んで決める（kanade の側は触らない）。

## 8. 調べが要るもの（Research Needed）

- **実機の xperf の dump**（#3・#23・#61）: 全プロセスのサンプルが入るか・Idle のサンプルの量・未解決フレームの綴り。手元に実物の `dump.txt` が無い（段③は実走で全周 UNAVAILABLE）。設計の段で `invoke-cpu-sample.ps1` の採取の設定と xperf の `-a dumper` の出力の形を確かめる（昇格した PowerShell で 1 回採れば決まる＝開発者の手が要るか、`target\` の下の既存の採取が無いかを先に探す）。
- **#2 の語彙**: kanade の `schedule/` の `event=` の全一覧（始まりと終わりの対）。`crates/areka-kanade/src/schedule/` を grep して設計で表にする。
- **#4 の直し方**: perf-rank の中だけで直す案が、Rust の `delta` のテスト（規則の正本）と食い違わないか。`perf_thread_report_tests.rs` を設計で照らす。
- **#28 の long の検体**: `cpu.csv` 90 行ほどで収束判定の傾きと窓の条件を満たす形を、設計で数を決める。
- **過去の結果との数の違い**: #2・#3・#4・#21 の直しで、完了 spec の `results/` の数（`talk_peak`・段②③）と今後の数が変わる。過去の記録は書き替えない前提でよいかを設計で確かめる。

### 8.1 要件の討議（2026-11-14）で設計の段へ回した事項

- #1 の一時フォルダの置き場（`target\perf-loop-tmp\` を `perf-loop.ps1` と分け合うか、Python の側の名前を分けるか）と、リポジトリの根の見つけ方（`__file__` から `tools/perf/` の 2 つ上）。
- #40・#41 の共通モジュールの名前と境目（何を置き、何を道具に残すか・判定式と較正値は置かない）と、`generate.py` から分ける兄弟の生成モジュールの形（議題 6）。
- #2 の始まりと終わりの語の対（上の「#2 の語彙」・議題 10）。
- #3・#23・#61 の実機の dump の形（上の「実機の xperf の dump」）。
- #4 を perf-rank の中だけで直す形と、Rust の `delta` のテストとの突き合わせ（上の「#4 の直し方」）。
- #22 の役割名の畳み方（`-<数字>` を `-*` にする規則と、期待出力の直し）。
- #28 の long と dev の検体の数（上の「#28 の long の検体」）。
- タスクの切り方（§6 の 17〜20 本の見込み・案 B と案 C の組み合わせ）。
- `audit-tools-perf-run` から受けた直し（要件の「申し合わせで受けた直し」の 2 件）を、どのタスクに入れるか。

## 9. 設計の段の記録（2026-11-14・`/kiro-spec-design -y`）

- **発見の種類**: light（今の道具の直し。外の新しい依存は無い＝Web の調べは要らない）。読むだけのサブエージェント 4 本（kanade の語彙・順位表の dump とスレッド・判定の検体と `generate.py`・共通モジュールの境目）に分けて読ませ、結論を設計に写した。実験の一時ファイルは `target\audit-perf-judge-research\` の下だけに置き、終わってから消した（`git status` は空のまま）。
- **使った手引き**: スキル `kiro-spec-design`（rules の design-principles・design-discovery-light・design-synthesis・design-review-gate）・形の手本 `.kiro/specs/completed/audit-impl-watch/design.md`・ponytail の「いちばん小さな完全な変更」。

### 9.1 §8・§8.1 の閉じ方（設計の決めの番号は design.md）

| 事項 | 結論 | design |
|---|---|---|
| #1 の一時フォルダ | `<リポジトリ>\target\perf-loop-tmp\<接頭辞><uuid>\`（根は `Path(__file__).resolve().parents[1]`）。`perf-loop.ps1` も同じ根の下に GUID の子を作り子だけを消すので、子の名前で分ければぶつからない（`perf-loop.ps1` の 246・273・350 行を読んだ） | D1 |
| #40 の置き場 | `tools/perf/perf_lines.py`。失敗は呼び手が渡す作り手で起こす（`SelftestEnv.bad_input` と同じ型）。judge-perf は同じ名前で読み込むので、Rust の注記 10 ファイルの「`judge-perf.py` の `parse_fields`」はずれない。置き場を `audit-placement-follow` へ知らせる | D2 |
| #41 の置き場 | `tools/perf/perf_selftest_common.py`。judge-perf の自己較正（その場の検体・一時フォルダ無し・形が違う）は寄せない | D3 |
| #2 の語彙 | 始まり 9 語・終わり 10 語の表。`close_refused` は `crates/` に無いので消す。終わりの印を出さない道（`force_quit`・`shiori_down`・`shiori_failed`・2 つの締め切り）は足さない（すぐ降ろしへ進み、区間はログの最後の行で閉じる） | D4 |
| 検体の置き場 | `tools/perf/fixtures/generate_more.py`。`generate.py` の `main()` が実行時に読む（読み込み時だと `__main__` と循環） | D5 |
| #3・#23・#61 の dump | 1 行ずつ 2 回読む（1 回目に areka の `SampledProfile` の (時刻, TID) を集め、2 回目にその組の `Stack` だけを数える）。列名の行の綴りは `invoke-cpu-sample.ps1` が Toolkit の書式として写した注記と検体で一致。実物の dump はツリーにも `target\` にも無い＝未解決フレームの綴りだけが確かめていない点。昇格の採取は開発者の手なので任意の確かめにした | D6 |
| #4 | perf-rank の中だけで、名簿外の残り＝`max(プロセスの差 − 名簿の差の合計, 0)`。Rust の `delta` のテスト（`perf_thread_report_tests.rs`）は名簿外の残りの差を計算しないので食い違わない。既存の期待出力（`full_ok` 1.6 s・`no_dump_unavailable` 0.5 s）も同じ値。`thread_registry.rs` は同じ TID の登録を置き換える（README の「消しません」は古い） | D7 |
| #22 | `^(actor:.+)-\d+$` → `\1-*`（役割別の表だけ）。今 `-<数字>` で終わる役割名は `sakura-talk-{id}` だけ | D8 |
| #28 の検体の数 | long の平坦 89 点（1,320 秒）→ 0・直線の上り 89 点 → 1・25 点（360 秒）→ 2・dev → 0。境目は 81 点（1,200 秒）で 0・80 点で 2。明示指定は 4 秒周期で 19 本 → 2・20 本 → 0（1 秒周期だと定常が 60 秒に届かない）。どれも `target\` の下で実際に回して確かめた | §Testing Strategy |
| 過去の `results/` | 書き替えない。#2・#3・#4・#21 で今後の数が変わることは design の Overview に書いた | Overview |
| タスクの切り方 | 15 本（案 B＝共通モジュールを先に・案 C の「振る舞いの直しとテストの足し」は道具ごとのタスクの中で同じ順に）。20 本を越えない | §タスクの見取り図 |
| 受けた直し 2 件 | wpaexporter の注釈＝タスク 10（toml）と 13（README）・`no_record`＝タスク 13 | File Structure Plan |

### 9.2 合成（generalization・build vs adopt・simplification）

- **一般化**: 写しの直し（#7 の NaN・#13 の BOM・#51 の行番号）は、#40 の共通モジュールの 1 か所で全部の道具に効く。一時フォルダ（#1）・ASCII の stderr（#25・#26）・BOM の字（#13）・README の例の道筋（#27）の見張りは、共通の自己較正の支えの `check_tree` 1 つにまとめ、4 本の自己較正が毎回回す（どれか 1 本を戻しても残りが赤になる）。
- **作るか使うか**: 全部標準ライブラリ（`unittest.mock.patch.object` で検体の作れない場面を確かめる・`tomllib`・`math.isfinite`）。依存は足さない。
- **削り**: judge-perf の自己較正は共通の支えへ寄せない（形が違い、寄せても縮まない）。`perf_lines` に道具の終了コードや固有の文を持たせない（作り手を渡す）。版は judge-perf だけ上げる（ほかの道具は決まりが無く、期待出力の字面を直すだけになる）。端末の文の「端末の文」タスクは作らず、道具ごとのタスクに畳んだ（同じファイルを 2 度触らない）。

### 9.3 設計の段で見つけた事実

- judge-perf の `--selftest` は argparse の `choices` で `build = debug` を拒むので、#43 の検体はコード 0 行で 3 を固定できる。baseline で run-meta が debug のときは今 0 を返す（指摘には入れない・判定モードの門は build の食い違いで 3 を返す）。
- `talk_peak_cpu_pct` は定常に限らず全区間の CPU を見る（`cpu_max_pct` は定常だけ）。#2 の検体は起動の CPU の山を定常の前に置けば、直す前 9.00・直した後 4.20 で分かれる。
- perf-rank の自己較正の一時フォルダの名はケースのフォルダ名を保つので、#65（`resolve().name`）で期待出力は変わらない。
- `perf-ledger.py` は 996 行で、#14・#15・#19・#20・#54〜#57 を足すと 1,000 行を越えうる。#59 で `SelftestEnv` を消して相殺し、なお越えるなら `entry.json` の検査を `perf_ledger_goal.py` へ置く（design D11）。
