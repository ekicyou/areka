# 実装計画

各指摘の場所（ファイルと行）は `research.md` §3、直し方の決めは `design.md` の「設計の決め（D1〜D12）」と「指摘ごとの直し」の表が正本。指摘の番号（#1〜#81）は `research.md` §3 の番号、T1〜T27 と検体の名（K1・L1 ほか）は `design.md` §Testing Strategy の呼び名。

- 着手の門: C20 は 2 段に分かれ、この spec は後の段。`/kiro-impl` は先の段の 4 本が `main` に着地し、このブランチへ `main` を取り込んで `pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest` が緑であることを確かめてから始める（タスク 1.1 の最初の手）。
- 各タスクの確かめ: 触った道具の `--selftest` が緑。判定の検体を触ったタスクは `python tools/perf/fixtures/generate.py` を回して `git status` が汚れない。消した名前と移した名前がツリー全体の grep（テスト・検体・設定・文字列を含む）で残っていない（5.5）。
- 1,000 行（D11）: `judge-perf.py`（4,159）と `fixtures/generate.py`（1,244）は、どのタスクの終わりでも着手のときの行数を越えない。`perf-ledger.py`（996）・`perf-compare.py`（969）・`perf-rank.py`（957）・`judge-followup.py`（931）も 1,000 行を越えない。各タスクの終わりに行数を数える。
- バグと危ない所を縛るテスト（T1〜T27）は、直しの前か、直しを一時的に戻して 1 度赤になることを確かめる（design の「直しを戻すと」の欄）。今の誤りの値で期待を書かない。
- `README.md` はタスク 13 だけが触る。ほかのタスクで要る README の直しは 13 へ渡す（design の「指摘ごとの直し」の表と §File Structure Plan の README の表が一覧）。
- `tools/perf/*.ps1`・`crates/`・`Cargo.toml`・`Cargo.lock`・`vendors/`・`.claude/skills/` には触らない（6.2・6.4・6.6・設計の申し合わせ）。例外は 14 の議題 1 の当てを使ったときの `SKILL.md` の見分けの語だけ。
- `check_tree` の ⑶（本体の stderr の全行が ASCII）は、その道具の文を英語にするタスク（compare＝7.2・followup＝8・rank＝12.2）で入れる。`perf_selftest_common.py` の ⑶ の本体は 2.2 で置き、道具ごとに有効にする。

## 共通の土台

- [x] 1. 行の読み方を共通の兄弟モジュールへ寄せる（#7・#13・#40・#51）
- [x] 1.1 行の読み方のモジュールを作り、judge-perf をそれへ付け替える（#40・#51・#13・#7 の CSV の分）
  - 着手の門（上）を確かめてから始める
  - 道具に依らない読み方（`parse_fields`・`split_after_message`・CPU の CSV の読み・`percentile` ほか、研究 §3 #40 の 5 か所の写し）を `perf_lines` に置く。判定式・較正値・終了コード・道具に固有の文は置かず、失敗は呼び手が渡す作り手で起こす（D2）
  - judge-perf は同じ名前のまま読み込み、`parse_fields` を名前で指す範囲の外の文がずれないことを grep で確かめる
  - 行番号を振ってから空行を飛ばす（#51）・`decode("utf-8-sig")` で BOM を読み、U+FEFF の字の比べを消す（#13）・CPU の CSV の有限でない値を壊れた行にする（#7）
  - `perf_lines` の振る舞いの変化（#7・#51）は judge-perf の版を上げる扱い（Revalidation Triggers）だが、版上げは 6.3 の 1 回でこの分も含めて行う（1 つの squash PR に入る・D10 の「1 回だけ」と合わせる）
  - 終わった姿: judge-perf の `--selftest` が今ある検体の全部で同じ結果のまま緑で、judge-perf の中に行の読み方の写しが残っていない
  - _Dispatch: standard — 新しい共通モジュールへ写しを寄せ、読み方の振る舞いの直し（#7・#13・#51）を伴う_
  - _Requirements: 2.1, 2.3, 5.1, 5.3, 7.1_
- [x] 1.2 残りの道具を行の読み方のモジュールへ付け替える（#40・#7 の rank の分・#13）
  - perf-rank・`perf_rank_dump`・perf-compare・judge-followup・`fixtures/generate.py` の写しを消して `perf_lines` を読む（`generate.py` は親のフォルダを `sys.path` へ入れる）。perf-compare と perf-rank は判定スクリプトを今までどおり読まない
  - rank のケース `cpu_nan`（有限でない CPU で exit 4）を足す（T7 の rank の分）
  - 終わった姿: 4 本の道具の `--selftest` が今ある全ケースで緑、`cpu_nan` が緑で、`isfinite` を外すと 0 で赤になる。5 か所の写しが grep で 0 件
  - _Dispatch: mechanical — 1.1 で作った形のまま残りの写しを読み込みに替え、ケースを 1 本足す_
  - _Requirements: 2.1, 2.3, 4.7, 4.8, 5.1, 5.3_
- [x] 1.3 新しい判定の検体の置き場を作り、N1・U1 を置く（D5・T7・T13）
  - `fixtures/generate_more.py` を作り、`generate.py` の `main()` が実行時に読み込んで `CASES + generate_more.CASES` を作る形にする（読み込み時の循環を作らない）
  - 判定の検体 `N1_cpu_nan`（exit 2）・`U1_bom_run_log`（exit 0）を `generate_more` に置き、作り直してバイトが一致することを確かめる
  - `audit-placement-follow` への置き場の知らせは要らない（2026-11-14 に着地し、`transition_judge.rs`・`transition_diag_tests.rs` の注記は「tools/perf/ の Python の道具の `parse_fields`」とファイルを名指さない形になった）。`presenter/timing.rs` の `judge-perf.py::parse_fields` の注記は、judge-perf が同じ名前で読み込むので指し先が残る（`crates/` には触らない）
  - 終わった姿: judge-perf の `--selftest` が N1・U1 を含めて緑、`generate.py` を回しても `git status` が汚れず、`generate.py` が着手のときの行数以下
  - _Dispatch: mechanical — 置き場の形（実行時の読み込み）と 2 本の検体の期待値を design が与える_
  - _Requirements: 2.1, 4.7, 4.8, 5.2, 7.1_

- [x] 2. 自己較正の支えを 1 つにまとめ、一時フォルダを `target` の下へ（#1・#41）
- [x] 2.1 自己較正の支えのモジュールを作り、4 組の自己較正を付け替える（D1・D3）
  - ケースのフォルダの並べ方・ケースの鍵の読み（知らない鍵は exit 3）・`invoke`・`first_difference`（違う行は `ascii()`）・赤の下限と `SELFTEST RESULT ok=N ng=M` の行・一時フォルダ `work_dir(prefix)` を `perf_selftest_common` に置く。道具ごとの違い（鍵の一覧・ケースの回し方・期待出力の突き合わせ・赤の値）は道具に残す
  - リポジトリの根は `Path(__file__).resolve().parents[2]` で、読み込みのときに `Cargo.toml` を確かめ、無ければ理由を stderr に出して exit 3。一時フォルダは `<根>\target\perf-loop-tmp\<接頭辞><uuid4>\`、消せなかったら理由を 1 行出して結果は変えない
  - `judge_followup_selftest.py`・`perf_compare_selftest.py`・`perf_ledger_selftest.py`・perf-rank の自己較正の 4 か所の `TemporaryDirectory` を `work_dir` に替える。judge-perf の自己較正は寄せない
  - 終わった姿: 4 本の `--selftest` が今ある全ケースで同じ結果のまま緑で、走らせた後に `target\perf-loop-tmp\` の下に子が残らない
  - _Dispatch: standard — 新しい支えのモジュールへ 4 組の自己較正を付け替え、一時フォルダの扱いを変える_
  - _Requirements: 2.1, 2.3, 5.1, 7.1, 7.2, 7.4_
- [x] 2.2 木全体の見張り `check_tree` の ⑴⑵⑷ を足す（T1・T13・T25 の ⑷）
  - ⑴ `tools/perf/` の下のどの `.py` も `tempfile` を読み込まず（perf-rank.py の 55 行の読み込みも消す）、作った子が、自分のファイルの位置から別に求めた根（`Cargo.toml` と `tools/perf/` があるフォルダ）の `target\perf-loop-tmp\` の下にある。⑵ 範囲のファイルに U+FEFF の字の比べが 0。⑷ `first_difference` の出力が ASCII
  - ⑶（本体の stderr の全行が ASCII）と ⑸（README の例）の確かめの本体も置き、道具の側から有効にする形にする（このタスクではどの道具も有効にしない＝後のタスクが `perf_selftest_common.py` に触らずに済む）
  - 4 本の自己較正の終わりで `check_tree` を回す
  - 終わった姿: 4 本の `--selftest` が緑で、1 本を `TemporaryDirectory` に戻すと ⑴ で、根を `parents[1]` に戻すと ⑴ で、U+FEFF の比べを戻すと ⑵ で赤になる
  - _Dispatch: standard — 木全体を歩く見張りの新しい検査（読み込みの検出・根の別求め）を組む_
  - _Requirements: 4.7, 4.8, 7.4_

## 判定（judge-perf）

- [x] 3. (P) judge-perf の要らないコードを消す（#36〜#39・#47〜#50・#81 の catchup の頭書き）
  - 自動選別の枝を消し、指定が空なら起動の時点で止める assert を置き、バナーの文を縮める（#36・README の文は 13 へ）。`_STATUS_ORDER`・`worst_status`（#37）・`PLANNED_MODES`（#38）・`_timedelta_seconds`（#47）を消す
  - `aggregate()` で `steady_cpu`・`steady_catchup` を 1 回作って持たせ、10 か所と `judge_perf_catchup.py` が読む（#39）。`catchup_analysis` のキャッシュを外す（#50）。判定式⑴の結果を `functools.partial` 1 つに（#48）。観測集合の行を「外した系列 N 本（根拠に列挙）」に（#49）。catchup の頭書きを事実に（#81）
  - 消す名前ごとにツリー全体の grep で使い手が無いことを確かめ、同じ後退で落ちる別のテストが残ることを研究 §3 の根拠のとおり確かめる
  - 終わった姿: judge-perf の `--selftest` が全検体で同じ数のまま緑で、消した名前が grep で 0 件、`judge-perf.py` が着手のときより減っている
  - _Boundary: judge-perf_
  - _Depends: 1.1_
  - _Dispatch: mechanical — 消す名前と `aggregate()` に持たせる形を design と研究 §3 が与える_
  - _Requirements: 2.1, 2.3, 5.5, 7.1_

- [x] 4. 判定の検体を手入れし、足りない場面の検体を足す（#28〜#30・#42・#43・#68〜#73・#74 の `generate.py:195`）
- [x] 4.1 検体の生成器の手入れ（#42・#68〜#74）
  - H1b を消し、「指定の外の遅い系列は合格を止めない」の裁定の記録を H11 の note へ移す（#68・README は 13 へ）。C1・C2・T2 の note を事実に（#69）。端末の文を英語に（#70）。作ったもの以外のフォルダがあれば名を出して止める（#71）。H12c を `case_H12()` から作る（#72）。確かめを `rmtree` の前へ（#73）。`generate.py:195` の行番号を名前へ（#74）
  - 4 つの終了コードを全部求め、定数と assert を消す（#42・judge-perf の数行は同じタスクの削りと差し引く）
  - 終わった姿: `generate.py` を回して検体が H1b を除いてバイトで同じ（H12c は双子の突き合わせが緑）、judge-perf の `--selftest` が緑、H1b の名が grep で 0 件
  - _Dispatch: mechanical — 生成器の注記・文・小さな検査を既存の形に沿って直す_
  - _Requirements: 2.1, 2.3, 5.4, 5.5, 7.1, 7.6_
- [x] 4.2 long・dev・明示指定・薄い定常・build の検体を足す（#28〜#30・#43）
  - `generate_more` に L1_long_flat_pass（0）・L2_long_rising_fail（1）・L3_long_too_short（2）・V1_dev_not_applicable（0）・S1_explicit_19_intervals（2）・S2_explicit_20_intervals（0）・S3_explicit_over_limit（1）・B2_thin_steady_verdict（2・`contains` で薄い定常の文）・X3_build_debug_arg（3）を design の表の形で置く。新しい名は今ある頭の字と重ならない
  - 終わった姿: 9 本が期待の終了コードで judge-perf の `--selftest` に緑で並び、作り直してバイトが一致し、`generate.py` が着手のときの行数以下
  - _Dispatch: mechanical — 9 本の検体の形と期待の終了コードを design の表が与える_
  - _Requirements: 2.1, 4.7, 4.8, 7.1_

- [x] 5. 発話の区切りの語を今の kanade に合わせる（#2・#74 の judge-perf の範囲の中）
  - `J_TALK_START_EVENTS` を 9 語・`J_TALK_END_EVENTS` を 10 語にし（D4 の表）、`close_refused` を消し、足さない語は足さない（議題 7）。`talk_spans` の決めは変えない
  - 語の注記と judge-perf の範囲の中の行番号を、ファイル名と語・関数名で指す形に（#74）
  - 判定の検体 `K1_boot_talk_excluded`（baseline・`emit_metrics = yes`・`talk_peak_cpu_pct value=4.20`）を `generate_more` に置く（T2）
  - 終わった姿: K1 が緑で、終わりの組から `boot_talk_done` を外すと 9.00 で赤。ほかの検体は同じ結果で緑
  - _Dispatch: mechanical — 語の一覧（D4 の表）と検体の期待値を design が与える_
  - _Requirements: 2.1, 4.7, 4.8, 5.3, 5.4_

- [x] 6. judge-perf の入力・終了コード・端末の文を直し、版を上げる（#5・#6・#8・#9・#24・#44〜#46・#75・#77・D10）
- [x] 6.1 入力の関門と数の出し方を直す（#5・#9・#44〜#46・#75・#77）
  - profile が `short`／`long` 以外なら判定不能（#5・検体 M1_profile_empty・M2_profile_unknown＝T5）。時刻の読めない catch-up があれば `catchup_count` は `-`（#9・検体 C3_undated_catchup_metric＝T9）
  - 注意書きを定常の点数と `CONVERGENCE_MIN_SAMPLES_PER_WINDOW * CONVERGENCE_WINDOW_COUNT` で比べる（#44）。食い違いのとき子の stderr の最初の行も添える（#45）。`case.txt` の知らない鍵は exit 3（#46）。`NO_COLOR` の説明を README と同じ趣旨に（#75）。見本のコメントを 2 行に（#77）
  - 終わった姿: M1・M2・C3 が緑で、`is None` に戻すと M1 が 0 で、0 を出す形に戻すと C3 で赤になる
  - _Dispatch: standard — 入力の関門と数の出し方に新しい分岐を足す_
  - _Requirements: 2.1, 4.7, 4.8, 7.1, 7.2_
- [x] 6.2 差し替えの較正を作り、想定外の例外・関門の順・最後の行を直す（#6・#8・#24・D9・D12）
  - `main` に `except Exception` を置き、traceback を stderr に出して終了コード 3（#6・申し合わせ済み）。関門は未観測を集めるだけにし、不合格の確かめの後で判定不能にする（#8）
  - 最後の行を ASCII の決まった形（`SELFTEST RESULT ok=N ng=M`・失敗の締めは `JUDGE-PERF RESULT code=N`）に、枠の行と失敗の理由を英語の ASCII に、`sys.stdout`・`sys.stderr` を `errors="backslashreplace"` に（#24・D9）。報告の本文は日本語のまま
  - judge-perf の読みの失敗の文は、呼び手の文と `perf_lines` の共通の部分の両方を英語の ASCII にする（`perf_lines` の文の持ち主はこのタスク・perf-rank の末尾は 12.2）。判定の検体のうち stderr の文を見る `contains` を `generate.py`／`generate_more.py` の `declare` で直して作り直す（`generate.py` は D11 を守る）
  - `judge_perf_selftest_inproc.py` を作り、judge-perf の `--selftest` から渡された本体の関数と定数を `unittest.mock.patch.object` で差し替えて `main(argv)` を呼ぶ。judge-perf の `--selftest` の終わりに呼び出しを 1 つ足す
  - 終わった姿: T6（`except Exception` を外すと 1 で赤）・T8（S3 の材料で A 上限越え・B 未観測が 1、関門を元の順に戻すと 2 で赤）・T24（最後の行を日本語に戻すと赤）が緑。`main` は今どおり終了コードを返し、終了を早める小細工が無い
  - _Dispatch: standard — 差し替えの較正の新しいモジュールを作り、例外の扱いと関門の順を組み直す_
  - _Requirements: 2.1, 4.7, 4.8, 6.1, 6.3, 7.1, 7.2, 7.3, 7.6_
- [x] 6.3 judge-perf の版を 0.5.0 に上げる（D10・議題 2）
  - `SCRIPT_VERSION` を `0.5.0` にし、版の文の中身を 0.5.0 の変更点の一文に（行は増やさない）。同じコミットで `goals/draw-load-parity.toml` の `judge_version`・`goals/draw-load-parity.goal.md` の条件文の版・`fixtures-loop/compare/goal.toml` と `fixtures-loop/compare/na_secondary/goal.toml` の `judge_version`・`fixtures-loop/compare/*/expected_compare.txt` 8 本の版を上げる。`fixtures-loop/ledger/` と来歴の注記は変えない
  - `audit-tools-perf-run` のセッションへ「judge-perf の版の値が 0.5.0 に変わるだけ・字面の形は同じ」と知らせる
  - 終わった姿: judge-perf と perf-compare の `--selftest` と `perf-loop.ps1 selftest` が緑で、ツリー全体の grep で D10 の一覧に `0.4.0` が残っていない
  - _Dispatch: mechanical — 版の値と直すファイルの一覧を design（D10）が与える_
  - _Requirements: 2.1, 5.1, 5.2, 6.1_

## 比べ・追いの判定・台帳

- [x] 7. perf-compare を直す（#7 の `to_number`・#10〜#12・#26・#32・#56・#74）
- [x] 7.1 床・副指標・判定の理由を直す（#7・#10〜#12）
  - `to_number` が有限でない値を `None` にし、自己較正で `to_number("nan") is None` を確かめる（T7 の compare の分）
  - 床は `bool` と `not floor > 0` を exit 3 で拒む（#10・ケース `floor_zero_rejected`・`floor_bool_rejected`＝T10）。数の取れた走行のどれにも副指標の鍵が無ければ exit 3（#11・ケース `unknown_metric_name`＝T11）。`compare.json` の各走行に `judge_stderr` を残し、計測失敗の stderr に添える（#12・既存の `measure_failed` で確かめる＝T12）
  - 終わった姿: perf-compare の `--selftest` が新しいケースを含めて緑で、検査を戻すと ADOPTED で赤、`judge_stderr` を消すと赤
  - _Dispatch: mechanical — 既存の exit 3 の検査の形に沿って入力の確かめと欄を 1 つ足す_
  - _Requirements: 2.1, 4.7, 4.8, 7.2_
- [x] 7.2 perf-compare の端末の文・ケース・読み方を直す（#26・#32・#56・#74）
  - stderr の失敗の文を英語の ASCII に（#26・D9）。報告の本文は日本語のまま。`fixtures-loop/compare/` の `stderr_substr` のうち当たるものを合わせる。compare の自己較正で `check_tree` ⑶（本体の stderr の全行が ASCII）を入れる（T26 の compare の分）
  - ケース `no_diff_by_spread`・`judge_version_mismatch`（exit 4）を足す（#32）。`encoding="utf-8-sig"`（#56）。範囲の中の行番号を名前へ（#74）
  - 終わった姿: perf-compare の `--selftest` が 2 ケースと ⑶ を含めて緑で、stderr の文を 1 つ日本語に戻すと ⑶ で赤、`perf-compare.py` が 1,000 行以下
  - _Dispatch: mechanical — 端末の文の英語化・ケースの追加・読み込みの文字コードを既存の形で直す_
  - _Requirements: 2.1, 4.8, 5.4, 7.1, 7.6_

- [x] 8. (P) judge-followup を直す（#26・#31・#52・#53・#74）
  - stderr の失敗の文を英語の ASCII に（#26・D9）。標準出力の報告の本文は日本語のまま。`fixtures-loop/followup/` の `stderr_substr` のうち当たるものを合わせ、followup の自己較正で `check_tree` ⑶ を入れる（T26 の followup の分）
  - ケース `drag_fail`（F1）を足し、`followup-verdict.txt` を突き合わせる（#31）。`DRAG_DX_PX` を消して `probe.log` の `dx`・`dy` を期待値に（#52）。使われない `or` の右を消す（#53）。範囲の中の行番号（Rust の行を指す 106〜112 行ほか）を名前へ（#74）
  - 終わった姿: judge-followup の `--selftest` が F1 と ⑶ を含めて緑で（stderr の文を 1 つ日本語に戻すと ⑶ で赤）、`DRAG_DX_PX` が grep で 0 件、`judge-followup.py` が 1,000 行以下
  - _Boundary: judge-followup_
  - _Depends: 2.2_
  - _Dispatch: mechanical — 端末の文の英語化・ケース 1 本・定数の削除を既存の形で直す_
  - _Requirements: 2.1, 2.3, 4.7, 4.8, 5.4, 5.5, 7.1, 7.6_

- [ ] 9. (P) 台帳の本体を直す（#14・#15・#19・#20・#33・#54〜#57・#59・#81 の perf-ledger）
- [ ] 9.1 台帳の書き込みの入力の確かめ（#14・#15・#19・#20）
  - `load_entry_json` で verdict が空なら exit 3（#14・ケース `append_without_verdict`＝T14）。`append` の `--iteration` を必須にして既定の枝を消し、`steps.txt` 5 本を合わせる（#15・ケース `append_without_iteration`＝T15）。`cmd_final` で許さない字を `_` に（#19・ケース `final_top_remaining_symbols`＝T19）。山括弧の値をひな型のままとして exit 3（#20・ケース `append_template_placeholder`＝T20）
  - 終わった姿: perf-ledger の `--selftest` が 4 ケースを含めて緑で、直しを戻すとそれぞれ 0 か 3 で赤
  - _Boundary: perf-ledger_
  - _Depends: 2.2_
  - _Dispatch: mechanical — 既存の exit 3 の検査の形に沿って 4 つの入力の確かめとケースを足す_
  - _Requirements: 2.1, 4.7, 4.8, 7.2_
- [ ] 9.2 台帳の書き込みの安全と支えの片付け（#33・#54〜#57・#59・#81）
  - ケース `write_fails`（普通のファイルの下へ init・exit 4＝W1）を足す（#33）。`--best` を `math.isfinite` に（#54）。目標定義の値の型を確かめて exit 3（#55 の perf-ledger の分・申し合わせ済み）。`encoding="utf-8-sig"`（#56）。flush → `os.fsync` → replace（#57）。selftest も `bind(core)` にして `SelftestEnv` を消す（#59）。perf-ledger の注記 3 か所を事実に（#81）
  - 失敗の文は日本語のまま（議題 1）
  - 終わった姿: perf-ledger の `--selftest` が W1 を含めて緑、`SelftestEnv` が grep で 0 件、`perf-ledger.py` が 1,000 行以下（越えるなら #14・#20 の検査を `perf_ledger_goal.py` へ置く）
  - _Dispatch: standard — 書き込みの順（fsync）と自己較正の支え（`bind(core)`）を組み直す_
  - _Requirements: 2.1, 2.3, 4.8, 5.5, 6.1, 7.1_

- [ ] 10. 台帳の判定面と目標定義を直す（#16〜#18・#55・#58・#60・#81・wpaexporter の注釈）
  - 版の正規表現を PS と同じに（#16・ケース `goal_check_version_suffix`＝T16）。goal-text の検査を前置きの一致に（#17・直の確かめ＝T17）。拾えない・開けないとき理由を stderr へ・`--goal` の `results_dir`・docstring（#18・ケース `summary_final_unreadable`＝T18）
  - 目標定義の値の型の確かめの goal の分（#55）。`_shared/` へ 1 本ずつ・`@SHARED@`（#58）。`formulas`・`main_model_recommended` を `GOAL_SCHEMA`・toml・検体の `goal.toml` 5 本から消す（#60）。`goals/draw-load-parity.toml` の 80 行の語と `perf_ledger_goal.py` の注記を事実に（#81）。#81 のうち 4,000 字の境目は振る舞いの直し（`perf_ledger_goal.py` の `>= 4000` を、要件と `goal.md` の「以内」に合わせて `> 4000` で拒む形に・ちょうど 4,000 字が通ることを ledger の自己較正で確かめる）。STOPPED 行が崩れたら GOAL_MET の見本を出さない（223）・`NOT_AVAILABLE` の重複（117）・余分な空行（555-556）も直す。`[sampling]` の注釈の wpaexporter の行を直す（`[sampling]` と `backend` の鍵は残す）
  - 終わった姿: perf-ledger の `--selftest` と `perf-loop.ps1 selftest` の goal-check が緑で、直しを戻すと T16〜T18 が赤、`formulas`・`main_model_recommended` が grep で 0 件
  - 6.3（`goals/draw-load-parity.toml` の版上げ）と 9（`perf_ledger_selftest.py`・`fixtures-loop/ledger/`）の後に取る
  - _Depends: 6.3_
  - _Dispatch: standard — 判定面の検査・目標定義の鍵・境目の直しが複数のファイルにまたがる_
  - _Requirements: 2.1, 2.3, 4.7, 4.8, 5.5, 6.1, 7.2_

## 順位表（perf-rank）

- [ ] 11. (P) 順位表の dump の読み方を直す（#3・#23・#25 の dump の文・#61・#62・#64）
  - dump を 1 行ずつ 2 回読み、1 回目は `SampledProfile` の `areka.exe (` の行の (時刻, TID) を集めて自己時間を数え、2 回目は当たる `Stack` の行だけを数える。列の確かめは事象ごと。areka の解決フレームは解決済みの枝の中でだけ数える。空の dump の確かめを「空でない行を 1 本も見なかった」に（D6・#3・#23・#61）
  - `dump_present = dump_path.is_file()`（#62）。0 バイトの dump は「在るが空」として計測失敗になるので、rank のケース `empty_dump`（0 バイトの `dump.txt`・exit 4・理由が stderr に出る＝R2）を足す。`DumpScan.tids` を消し、`sample_ok_counts/case.txt` の注記を直す（#64）。`perf_rank_dump` の `RankError` の文を英語の ASCII に（#25 の dump の分）
  - rank のケース `other_process_rows`（T3）・`unresolved_areka_frames`（exit 4＝T23）を足す
  - 終わった姿: perf-rank の `--selftest` が `other_process_rows`・`unresolved_areka_frames`・`empty_dump` を含めて緑で、処理名の絞りを外すと T3 が、数える所を戻すと T23 が、`exists()` に戻すと R2 が赤
  - 実機の確かめ（任意・議題 3）: 開発者が昇格の窓を開けるときだけ `target\perf-judge-dump-check\` へ採取 1 回と `perf-rank.py --stage function` を回し、列名の行・`areka.exe!0x` の綴り・exit を記録に残してフォルダを消す。開けないときは検証の記録に「実機の dump での確かめは未」と書く
  - _Boundary: perf_rank_dump, perf-rank（dump を読む所）, fixtures-loop/rank_
  - _Depends: 2.2_
  - _Dispatch: standard, effort high — dump の 2 回読みと事象ごとの列の確かめは取り違えやすい数え方_
  - _Requirements: 2.1, 2.3, 4.7, 4.8, 5.5, 7.4, 7.6_

- [ ] 12. 順位表のスレッド・引数・端末の文を直す（#4・#21・#22・#25・#34・#35・#63・#65・#66・#67）
- [ ] 12.1 スレッドの数え方を直す（#4・#22・#63・#65）
  - 差の基準で `perf(process)` の差が取れたとき、名簿外の残りの行の `cpu_us` を `max(プロセスの差 − 名簿の行の差の合計, 0)` に（D7・#4・ケース `tid_reused`＝T4）。TID の照らし方は Rust の `delta` と同じまま
  - 役割別の表を作るときだけ `actor:…-<数字>` を `-*` に畳む（D8・#22・ケース `talk_threads_folded`＝T22）。`perf(process)` の欄を `snap`・`cpu_us` だけに（#63）。`run_dir.resolve().name`（#65）
  - Rust（`perf_thread_report.rs` とテスト）には触らない。`talk-thread-count-growth` の brief に、#4・#22・#63 をこの spec で直したことを一言残す（議題 9）
  - 終わった姿: perf-rank の `--selftest` が T4・T22 を含めて緑で、既存の `full_ok`・`no_dump_unavailable` の期待出力が同じ値のまま、行の差に戻すと T4 が `200.00` で赤
  - _Dispatch: standard — スレッドの残りの差（D7）と役割の畳み（D8）の新しい数え方を足す_
  - _Requirements: 2.1, 4.7, 4.8, 5.1, 5.3, 7.1_
- [ ] 12.2 順位表の出力・引数・端末の文を直す（#21・#25・#34・#35・#66・#67）
  - `talk_peak_pct` と窓の定数・注記を消し、期待出力 3 本を直す（#21・T21）。`.claude/agents/perf-measure.md` の当たる行を直す
  - `--sampling-available` から `true` の選択肢と分岐を消し、`unresolved_dump` を `auto` に（#67・申し合わせ済み）。`no_dump_unavailable` に本番の 2 つの引数と期待の 1 行 `reason=not_elevated`（#34）。`cpu_us` の無い `perf(thread)` 行のケース（exit 4＝R1・#35）
  - perf-rank の `RankError` と stderr の文を英語の ASCII に、`errors="backslashreplace"` に（#25・D9）。`perf_lines` の読みの失敗の文に続く perf-rank の末尾も合わせる。`fixtures-loop/rank/` の `stderr_substr` のうち当たるものを合わせ、rank の自己較正で `check_tree` ⑶ を入れる（T25 の ⑶）
  - rank の理由の語を `.claude/agents/perf-measure.md`・`perf-analyze.md` に並べる（#66・perf-loop の語を並べるなら `no_record` を入れる・README は 13 へ）
  - 消す名前（`talk_peak_pct`・`true` の分岐）がツリー全体の grep で `.ps1` を含めて使い手が無いことを確かめる
  - 終わった姿: perf-rank の `--selftest` が R1 と ⑶ を含めて緑で（stderr の文を 1 つ日本語に戻すと ⑶ で赤）、期待出力に `talk_peak_pct` の行が無く、`perf-rank.py` が 1,000 行以下
  - _Dispatch: mechanical — 出力の欄の削除・選択肢の削除・端末の文の英語化を design の形で直す_
  - _Requirements: 2.1, 2.3, 4.7, 4.8, 5.1, 5.2, 5.5, 6.1, 6.3, 7.1, 7.6_

## 文書と仕上げ

- [ ] 13. README を直し、README の見張りを足す（T27）
  - 着手の前に `main` を取り込み、`audit-tools-perf-run` が着地したかを確かめる。着地していればあちらの形と突き合わせて perf-run の文 ①〜④ を書き、着地していなければ今の `.ps1` の形で書いて、後に着地した方が取り込みで合わせる（design の申し合わせ）
  - 指摘の分: #3・#4（段②の TID の使い回しの見え方を 1 文・名簿の登録の文を事実に）・#21・#24〜#27（例を置き換え札に）・#29・#36・#42・#62・#66（理由の語に `no_record`）・#68（H11 の裁定の記録）・#74（行番号を名前へ）・#76・#78〜#80
  - 申し合わせの分: wpaexporter の説明・perf-run の文 ①〜④（555 行・563〜573 行・880〜887 行・902〜903 行の当たる所）
  - 例を直した後に、2.2 で置いた `check_tree` ⑸（README の例の道筋が置き換え札の形）を、4 本の自己較正（perf-rank・judge-followup・perf-compare・perf-ledger の自己較正の所）で有効にする
  - 終わった姿: design の README の表の行が全部入り、⑸ を含む 4 本の `--selftest` が緑で、README の例を `C:\出力先` に戻すと ⑸ で赤、README の中に古い行番号の指し先が無い
  - _Dispatch: mechanical — 直す文の中身を design の README の表と perf-run の design が与える_
  - _Requirements: 2.1, 4.7, 4.8, 5.1, 5.2, 5.4_

- [ ] 14. かけ直しの audit で新しい指摘を 0 件にする
  - 範囲（`tools/perf/` の git が追う `.ps1` でないファイル全部）に、件数の上限なし・全部読む作法で `/ponytail:ponytail-audit` をかける
  - 出た指摘（直しが生んだものを含む）を、番号と 3 つの組を付けて記録し、全部直す。要件 3 の 3 種類に当たるものだけ直さずに開発者へ議題にし、裁定と理由を記録して数から外す。依存が要ると分かったら足さずに議題にする
  - 議題 1 の当て: perf-ledger の失敗の文が `tech.md` の ASCII の決まりに反すると指摘されたら、`audit-tools-perf-run` へ知らせてから perf-ledger の文と `SKILL.md` の見分けの語を重ならない行だけで合わせる
  - 0 件になるまでかけ直す。5 回目でも 0 件にならず、5 回目に「必ず直す」と本番の振る舞いに関わるものが無ければ、開発者に一言知らせて 5 回目の直しで打ち切る。直しでタスクが 20 本を越えそうなら議題にする
  - 終わった姿: `research.md` の検証の記録に、回ごとの組ごとの件数と、0 件または「5 回目で打ち切った・残りは直し済み」の判定が書かれている
  - _Dispatch: standard — かけ直しの audit で出る指摘を読んで直す（中身は前もって決まらない）_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 6.5, 8.1_

- [ ] 15. 自己較正と全体テストを緑にし、差分の約束を確かめる
  - `pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest` と `pwsh -NoProfile -File tools/test-all.ps1` を回す（全体テストは長いので裏で回す）
  - 既定のブランチとの差分で、`tools/perf/*.ps1`・`crates/`・`Cargo.toml`・`Cargo.lock`・`vendors/`・`.claude/skills/` の差分が 0（ただしタスク 14 で議題 1 の当てを使ったときだけ `perf-loop-iteration/SKILL.md` の見分けの語の行を許し、そのことを記録に書く）、Python の道具に標準ライブラリの外の読み込みが 0、範囲の外で触ったのが `.claude/agents/` の 2 本と `talk-thread-count-growth/brief.md` だけであることを確かめる
  - `judge-perf.py` と `fixtures/generate.py` が着手のときの行数以下、ほかのどのファイルも 1,000 行の決まりを守る。GPU を使うテストを足していない（0 件）ことを記録に書く
  - `audit-tools-perf-run` へ py の行が動いたことを知らせる（あちらの 2 か所の行番号の合わせ）
  - 終わった姿: 2 つのコマンドが緑で、上の確かめの結果が検証の記録に書かれている
  - _Dispatch: mechanical — 決まったコマンドを回し、差分の約束を数えて記録する_
  - _Requirements: 4.5, 4.6, 5.2, 5.6, 6.2, 6.4, 6.6, 7.1, 7.5_

## Implementation Notes

- 1.1: `perf_lines.read_cpu_csv` は design の形に加えて `read_fail`（読めないときの作り手）を取り、`header` は tuple。読めない cpu.csv は今どおり exit 3。perf-rank も `read_fail=bad_input` で渡す（1.2）
- 1.2: perf-rank の cpu.csv の行番号は空行を含む実の行番号になった（#51）。perf-rank の注記「較正値（…）も同じ理由で写し」の「も」は先行の写しが無くなったので 14 の audit で拾う候補
- 1.3: U1 は run.log と cpu.csv の両方に BOM。run.log は探して読むので BOM に左右されず、赤を作るのは cpu.csv の見出しの一致。judge-perf の selftest の `contains` は stdout だけを見る（stderr の理由は縛れない）
- 2.1: `perf_selftest_common` の根の確かめは道具の読み込みのたびに走る（.ps1 は `$PSScriptRoot` から呼ぶので本番は壊れない）。`list_case_dirs` は `fail` を、`finish` は `materials`・`case_file` を取る。design の `guard_console` は持ち主のタスクが無く未実装
- 2.2: `check_tree(*, ascii_stderr, readme_examples)`（design の `stderr_texts` の代わりに `invoke` が本体の stderr を集める）。⑶ は 7.2・8・12.2 が自分の selftest の `ascii_stderr=True` で、⑸ は 13 が 4 か所の `readme_examples=True` で有効にする。⑸ は README の code block の中のドライブ文字の絶対パスを全部拾うので、13 は #27 の行に加えて 765 行（`pending_run: C:\...`）と 886 行（`-PdbDir C:\repo\...`）も置き換え札にするか、検査を絞る
- 3: README へ渡す分（13）: 286 行の表の `FRAME_INTERVAL_JUDGED_SERIES` の行に「空にできない（起動で assert）」。`FRAME_INTERVAL_MAX_MEAN_FACTOR` は窓 A の p95 の注記に使うので残す（合否には使わない）
- 4.1: README へ渡す分（13）: 344・411 行の H1b を H11 へ、397〜399 行に「4 つの終了コードは全部要る」（#42）。H1b の裁定の記録は H11 の note
- 4.2: `tools/perf/fixtures/.gitattributes` は `* -text`。fixtures の .py は LF のまま書く（CRLF にすると CRLF で入る）。検体を作り直すのは generate.py（generate_more.py 単独では書かない）
- 5: design T2 の「5 秒の CPU 9.00」は D4 のもとでは満たせない（起動の挨拶も発話の区間）ので、実装に合わせて「12 秒（boot_talk_done の後）」へ直した。K1 が縛るのは「終わりの語が無いと区間が閉じず、挨拶の後の重さを巻き込む」こと
- 6.1: README へ渡す分（13）の候補: profile は short・long 以外で判定不能／時刻の無い catch-up があると `catchup_count` は `-`（978 行あたり）／case.txt の鍵の一覧と知らない鍵は exit 3（389 行あたり）／検体の一覧に M1・M2・C3／baseline の注意書きは定常の点数（12）で比べる
- 6.2: judge-perf の verdict・baseline の標準出力の締めも `JUDGE-PERF RESULT code=N`（「判定は以上」を置き換え）。perf-loop の止まりの文の LastLine はこの行になり、理由は stderr と verdict.txt に残る（`audit-tools-perf-run` へ知らせる・6.3 の知らせと一緒に）。`--emit-metrics` では metric の行が RESULT の後に続く／`SELFTEST RESULT` の ok は検体の数・ng は理由の数／`--selftest` の想定外の例外は今も exit 1（14 の audit で拾う候補）。perf_lines の失敗の文は英語の ASCII
- 6.3: 範囲の外: `doc/COMPAT_ARCHITECTURE.md:629` が「judge-perf.py（版 0.4.0）の較正値に従う」と版を名指していて古くなった（D10 の一覧に無い）。/kiro-complete の棚卸で扱う
- 7.1: 7.2 への申し送り: exit 3 の 3 ケース（floor_*・unknown_metric_name）の `stderr_substr = perf-compare:` は理由を縛らないので、英語化のときに鍵の名（`noise_floor_pct` など）で縛る。bool の床を拒む文「…がありません」は原因（真偽値）とずれるので英語化で合わせる
- 7.2: perf-compare の `main` の `reconfigure(errors="replace")` が残る（D9 は全部 `backslashreplace` にそろえる）。8・12.2 のときに残りの `replace` も合わせる。perf-compare の未決の理由は日本語（報告・compare.json）と英語（stderr）の 2 つを持つ
- 8: `DRAG_DX_PX` は judge-followup の側では 0 件。`invoke-followup-checks.ps1` は作り手の定数として持ち続け、48 行の注記「judge-followup.py と対」は古くなった（`audit-tools-perf-run` へ知らせる）。show.rs の出所の注記の「info!」は実際は `show_line!` の `tracing::event!`（14 の audit で拾う候補）
