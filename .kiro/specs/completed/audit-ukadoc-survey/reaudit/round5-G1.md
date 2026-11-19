# かけ直し 5 回目・G1

指摘 2 件（必ず直す 0・直したほうがよい 0・できれば 2）。どちらも [test/doc] で、副手続きの出力や振る舞いを変えるもの（[prod]）は 0 件。どちらも残す 3 種類に当たらない。読んだファイル 32 本すべて全行。4 回目の直し（G1-1〜G1-5）はどれも直っていることを確かめた（`error.rs` の `CheckFindings` の除き書き・`diff_tests.rs` の 11 行目の URL・`files_tests.rs` の 1 本と `own_manifest` の消去・`generate_tests.rs` の鍵の数え直しの消去・`snapshot_tests.rs` の `load_with` の配線の 1 本と冒頭の注記）。4 回目の直しから生まれた新しい問題は無い（`files_tests.rs:11` の「⑷ の 3 本と `io/snapshot_tests.rs` の 1 本だけ」は crate 全体を grep して合っていた）。

| id | 重さ | 種類 | file:line | 何が違うか | 直し方 | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G1-1 | できれば | [test/doc] | src/io/paths_tests.rs:26 | `workspace_root_holds_this_crate_manifest` の説明「根の直下に自分自身の manifest がある」は偽。見ているのは根の 2 段下の `crates/ukadoc-survey/Cargo.toml` で、根の直下の manifest を見るのは次の `workspace_root_holds_the_workspace_manifest`（40 行）のほう | 「根の下の `crates/ukadoc-survey/` に自分自身の manifest がある」に直す | no | none |
| G1-2 | できれば | [test/doc] | src/cli/cli_tests.rs:14-15 | 「名前と中身の結び付きは、各副手続きの在中テストと `tests/cli_streams.rs` が受け持つ」は言い過ぎ。各副手続きの在中テストは `*_with` や `examine`・`compare` を直に呼ぶので `SUBCOMMANDS` の表を通らず、`cli_streams.rs` が表を通して中身まで走らせるのは `catalog`（失敗の腕）と `candidates` だけ。たとえば `report` と `report-summary` の中身を表の中で入れ替えても、どのテストも赤にならない | 「名前と中身の結び付きを走らせて見るのは `tests/cli_streams.rs` の `catalog`・`candidates` だけで、残る 7 つは表を読んで確かめる（走らせると台帳や報告を書くため）」の形に直す | no | none |

## ruling-conflicts

なし。

## 確かめて指摘にしなかったもの

- `assignment_tests.rs:177-182` の `unassigned_is_empty_for_no_pages_at_all`: 向きを取り違えた実装（表にあって入力に無いページを返す）は直後の `unassigned_is_sorted_and_deduplicated` でも赤になるので重なりは大きいが、空の入力の境目を 1 行で見る形で、片方だけが捕まえる壊し方（空のときだけ別の値を返す）も作れるので外した。
- `inspect_tests.rs:602-616` の `an_empty_diff_still_names_all_four_lists`: 一覧が全部空で注意も無いときだけを見るのはこの 1 本（`compare` の 3 本はどれも空でない一覧か注意を持つ）なので、写しではない。
- `inspect_tests.rs:119-127` の `an_empty_evidence_listing_still_says_how_many_there_were`: 空の `by_id` の版面を逐語で見るのはこの 1 本だけ。
- `inspect.rs:217-227` の `render_evidence` の並べ直しを縛るテストが無いこと: `resolve` が既に順序の決まった集合から作る（`evidence/resolve.rs:88-90`）ので、並べ直しを消しても本番の出力は変わらない。
- `generate.rs:173-187`・`:241-252` の `report()`・`priority_apply()` の 1 行（渡す読み手と書き手）が常時テストに届かないこと: `ledger_init()` と同じ形で、repo を書かずに縛る手が無い（2 回目の記録の `ledger_init()` の扱いと同じ）。G1-2 の直しの文で触れれば足りる。
- `snapshot_tests.rs:406` の「`package.json` と 2 つ下の `data/index.json`」: `snapshot.rs:157`・`:174` の「スナップショットの 2 つ上」と同じ数え方（パスの欄の数）で、食い違っていない。
- `cli_tests.rs:167-178` の `the_usage_text_lists_every_name_in_the_dispatch_table`: 設計の討議（design.md 326・363・421 行）で残すと決めてある。
- 群の外（spec 文書）: `design.md:183`・`:394` の「ファイルを書くのは 2 のテストだけ」は、4 回目の G1-5 の直しで `snapshot_tests.rs` の 1 本が書くようになったので、字面は古くなった。置き場は同じ `target\ukadoc-survey-tests\<テスト名>\` で要件 4.3 は満たしている。コードの群の指摘ではないので表に入れない。

## 読んだファイル

| ファイル（crates/ukadoc-survey/ から） | 行数 | 読んだ |
|---|---|---|
| Cargo.toml | 25 | はい |
| src/main.rs | 26 | はい |
| src/lib.rs | 40 | はい |
| src/lib_test_support.rs | 530 | はい |
| src/error.rs | 69 | はい |
| src/error_tests.rs | 139 | はい |
| src/model.rs | 322 | はい |
| src/model_tests.rs | 304 | はい |
| src/hash.rs | 57 | はい |
| src/hash_tests.rs | 135 | はい |
| src/tomlout.rs | 174 | はい |
| src/tomlout_tests.rs | 339 | はい |
| src/assignment.rs | 149 | はい |
| src/assignment_tests.rs | 238 | はい |
| src/diff.rs | 134 | はい |
| src/diff_tests.rs | 556 | はい |
| src/cli/mod.rs | 208 | はい |
| src/cli/cli_tests.rs | 207 | はい |
| src/cli/generate.rs | 395 | はい |
| src/cli/generate_tests.rs | 613 | はい |
| src/cli/generate_priority_tests.rs | 383 | はい |
| src/cli/inspect.rs | 373 | はい |
| src/cli/inspect_tests.rs | 616 | はい |
| src/io/mod.rs | 15 | はい |
| src/io/files.rs | 84 | はい |
| src/io/files_tests.rs | 250 | はい |
| src/io/paths.rs | 119 | はい |
| src/io/paths_tests.rs | 157 | はい |
| src/io/snapshot.rs | 329 | はい |
| src/io/snapshot_tests.rs | 435 | はい |
| src/io/sources.rs | 97 | はい |
| src/io/sources_tests.rs | 179 | はい |
