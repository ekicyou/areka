# 検証の記録（audit-ukadoc-survey）

## 指摘 35: カタログの読み直しの 1 バイトの安定（タスク 1.1）

- 整合テスト `the_catalog_round_trips_byte_for_byte`（`crates/ukadoc-survey/tests/consistency/checks.rs`）を置き、最初にこの 1 本だけを回した。
- 結果: 緑。今の `catalog.toml` は `write(read(t)) == t` を満たす。差は 0 バイトで、6.5 の議題は起きない。

## 所要時間（Performance の「前」）

- `cargo test -p ukadoc-survey`（ビルド済み・排他テストの机を持って 1 回）: 各テスト実行ファイルの所要の合計 約 40.9 秒（単体 601 本 2.56 秒・cli_streams 6 本 2.12 秒・consistency 114 本 36.11 秒・doc-test 5 本 0.06 秒）。全部緑。壁時計の通し値は取り損ねた（計時の道具が無かった）

## かけ直しの結果（タスク 9.1）

- 範囲は `crates/ukadoc-survey/` 全部。4 つの組（G1 入口・入出力・共通／G2 カタログ・3 文書・台帳／G3 判定・証拠・報告／G4 整合テスト）に分け、上限なし・全部読む作法で 6 回かけた。各回の記録は `reaudit/round<回>-G<組>.md`。
- 件数は各回の記録の指摘の表の行を数えたもの。`[prod]`（副手続きの出力や道具の振る舞いに触る指摘）の印は 3 回目から付けた。

| 回 | 件数 | 必ず直す | 直したほうがよい | できれば | [prod] | 直しのコミット |
|---|---|---|---|---|---|---|
| 1 | 68 | 0 | 22 | 46 | （印なし） | `b94b2f76e`・`6713637d4`・`aee1689bd`・`c76d8518a` |
| 2 | 42 | 0 | 6 | 36 | （印なし） | `9b697aeca`（Fable）・`78edf300f`・`46d7cdb90`・`b042fbf2a`・`3306c242d` |
| 3 | 21 | 0 | 2 | 19 | 1 | `bfbb8b010`（Fable）・`99b9b2639` |
| 4 | 16 | 0 | 0 | 16 | 0 | `2d1e2a247` |
| 5 | 11 | 0 | 1 | 10 | 2 | `fea70ff55` |
| 6 | 10 | 0 | 0 | 10 | 1 | `6d594c747`（Fable）・残りの 6 件はこの記録と同じコミット |

- 回をまたいで直しが次の指摘を呼んだ連鎖は、Fable でまとめて矛盾なく直した。
  - 2 回目（`9b697aeca`）: G1-2・G1-6・G3-6・G3-7・G3-8・G3-9・G3-10・G4-1・G4-2・G4-3・G4-12
  - 3 回目（`bfbb8b010`）: G1-1・G1-3・G2-1・G4-2
  - 6 回目（`6d594c747`）: G1-1・G2-1・G2-3・G2-4
- 6 回目の残り 6 件（G1-2・G2-2・G3-1・G3-2・G4-1・G4-2）も直した。G2-2・G3-1・G4-2 は、守る分岐や範囲の終わりを外すと赤になることを確かめてから緑にした。G3-2 は届かない枝を消すもので、副手続きの出力は変わらない。
- **6 回目で打ち切った（必ず直す 0・本番の振る舞いの指摘 0。残りは直し済み）。** 字面の 0 件は、直しが次の言い回しの指摘を呼ぶので収束しない。audit の spec では「必ず直すが 0 件で、本番の振る舞いに触る指摘も 0 件の回が来たら、その回の指摘を直して打ち切ってよい」が開発者の決まりである。6 回目の `[prod]` の 1 件（G3-2）は、副手続きの出力も整合テストの判定も変えない届かない枝なので、本番の振る舞いの指摘には数えない。
- 5 回目 G3-1 の ⒝（ドメイン別報告の見出しと全体報告の 1 文を直し、報告を作り直す）は出力を変える直しなので本 spec では行わず、別 spec で直す（完了の棚卸で `coverage-roadmap-refresh` の brief へ件として申し送った）。申し送りの 1 行は `tasks.md` の Implementation Notes にある。

## `main` との突き合わせ（タスク 9.2）

- 比べた相手は `main` との分かれ目のコミット `5749a4c72`（`git merge-base main HEAD`）。差分は `git diff main...HEAD` で見た。

### 範囲の外の差分

- `crates/ukadoc-survey/` の外で変わったファイルは 3 つだけ: `doc/ukadoc-coverage/README.md`（足し 14・消し 9）・`.claude/skills/kiro-complete/SKILL.md`（消し 5）・`.kiro/specs/audit-ukadoc-survey/` の下（31 ファイル）。
- 次のファイルの差分は 0 行: `Cargo.lock`・`crates/log-capture-kit/`（長さの番人のテスト `tests/file_length_guard_test.rs` を含む）・`doc/ukadoc-coverage/ledger/assets.toml`・`vendors/`・C22 のほかの 5 本が触るファイル（ロードマップの C22 の行の ①〜④⑥ にある crate とファイル: `areka-parsers`・`areka-emo-compose`・`areka-emo-present`・`areka-placement`・`wintf`・`areka-runtime`・`areka`・`areka-actor`・`areka-emo-text`・`real-run-kit`・`sample-ghost-kit`・`temp-path-kit`・`pilot`・`doc/COMPAT_ARCHITECTURE.md`）。

### 副手続きの出力

- 作り方: ワークツリーの `target\` の下に、今の `HEAD` の中身を `git archive` で 2 つ写した（`git worktree add` は使わない）。片方は `crates/ukadoc-survey/` だけを分かれ目の版に替えて道具を作り、作った後で `HEAD` の版へ戻した。根は作った木に埋め込まれるので、2 つの道具は中身が同じ別々の入力を読む。入力が同じことは、2 つの木を `diff -r`（`target` を除く）で比べて確かめた。
- 2 つの木で、同じ順に 9 つの副手続きを回した（`report`・`report-summary`・`ledger-init`・`priority-apply`・`check`・`evidence`・`candidates`・`catalog`・`diff`）。標準出力と標準エラーは、木の根を同じ印に置き換えてから比べた。ワークツリーの `doc/ukadoc-coverage/` には書いていない。
- `catalog`・`diff` はこの機械の既定のスナップショット（`%APPDATA%` の下・`AREKA_UKADOC_SNAPSHOT` は無し）で回した。どちらの道具でも終了コード 1 で、標準エラーは同じ 1 行（`項目 id の形が違う: ukadoc:descript_balloon:charset,文字コード`）。スナップショットの中身が今の道具の読める形でないためで、両方の道具で同じに落ちる。何も書き出さない。

| 副手続き | 終了コード（main・今） | 標準出力 | 標準エラー |
|---|---|---|---|
| `report` | 0・0 | 同じ | 同じ（空） |
| `report-summary` | 0・0 | 同じ | 同じ（空） |
| `ledger-init` | 0・0 | 同じ | 同じ（空） |
| `priority-apply` | 0・0 | 同じ | 同じ（空） |
| `check` | 0・0 | 同じ（食い違い 0 件） | 同じ（空） |
| `evidence` | 0・0 | 同じ | 同じ（空） |
| `candidates` | 0・0 | 違う（下の許された違いだけ） | 同じ（空） |
| `catalog` | 1・1 | 同じ（空） | 同じ |
| `diff` | 1・1 | 同じ（空） | 同じ |

- `candidates` の違いはテストのファイルの 2 つだけ。`crates/areka/src/placement_tests/zorder_branch_coverage_test_support.rs`（候補 2 つ）と `crates/areka/src/runtime_tests/spine_conformance_script.rs`（候補 10 個）の計 12 件が、今の道具では候補から外れる（指摘 3 の `_tests/` の下をテストとして扱う直し・Implementation Notes の 3.4）。そのぶん先頭の合計の行（798 件→786 件）と `[AllowListElement]` の件数の行（639 件→627 件）も 12 減る。ほかの行は同じ。
- 回した後の `doc/ukadoc-coverage/` の木（19 ファイル）は、手引きを含めても 2 つの木でバイトまで同じで、写した元の `HEAD` の中身とも同じ（書き出しはしたが中身が変わらない）。
- **許された違いのほかは 0 件。** 写した 2 つの木は確かめた後に消した。

## 全体テストと所要時間（タスク 9.3）

- `pwsh -NoProfile -File tools/test-all.ps1` を排他テストの机を持って 1 回回した。結果: `all steps green`（終了コード 0）。主な行: x64 workspace tests 808 秒 OK・dola toml feature tests 53 秒 OK・i686 tests (host-32) 43 秒 OK・fmt --check 7 秒 OK・encoding check 25 秒 OK・perf loop selftest 77 秒 OK。ほかの検査も全部 OK。
- 所要時間（Performance の「後」）: 「前」と同じ測り方（テストの実行ファイルを先に作ってから `cargo test -p ukadoc-survey` を回し、各実行ファイルの finished in を足す）で 約 2.9 秒（単体 526 本 0.58 秒・cli_streams 5 本 0.70 秒・consistency 105 本 1.56 秒・doc-test 5 本 0.03 秒・もう 1 本の実行ファイル 0 本 0.00 秒）。全部緑。
- 「前」約 40.9 秒 → 「後」約 2.9 秒。主に consistency が 36.11 秒から 1.56 秒になった。
