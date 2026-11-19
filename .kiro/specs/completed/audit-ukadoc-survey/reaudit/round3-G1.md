# かけ直し 3 回目・G1

指摘 6 件（必ず直す 0・直したほうがよい 2・できれば 4）。どれも [test/doc] で、副手続きの出力や振る舞いを変えるもの（[prod]）は 0 件。どれも残す 3 種類に当たらない。読んだファイル 32 本すべて全行。2 回目の直し（G1-1〜G1-10）はどれも直っていることを確かめた。新しく生まれた問題は G1-1 の 1 件（2 回目の G1-4 の直しで足した逐語のテストが、前からある 2 本を含んでしまった）。

| id | 重さ | 種類 | file:line | 何が違うか | 直し方 | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G1-1 | 直したほうがよい | [test/doc] | src/cli/inspect_tests.rs:589-612 | `a_different_catalog_format_draws_a_warning_that_names_both_numbers` と `the_warning_says_which_of_the_two_disagrees` は、2 回目の G1-4 の直しで足した `a_catalog_of_another_shape_version_only_gets_no_claim_that_almost_every_body_changed`（566-583。形の版だけが違うときの注意を「現行 n / 新しい方 n+1」まで逐語で見る）と、算法の違いで両方の綴りと「ほぼ全件」を見る 535-564 に含まれる言い直しになった | 2 本を消す（585-587 の節の見出しは「差分の版面（要件 8.1）」に直す） | no | none |
| G1-2 | 直したほうがよい | [test/doc] | src/cli/generate_priority_tests.rs:295-311（と 159-191 の `apply_over`） | 名前と注記は「探した**絶対**パスを添えて止まる（要件 1.4）」と言うが、確かめるのは本文が `doc/ukadoc-coverage/…` の後ろの綴りを含むことだけ。`priority_apply_with` が `paths::linkage_path()` の代わりに相対の `document_file(...)` を読み手に渡しても、`under_coverage` は相対の綴りをそのまま返すので、このテストも 6 本の読みの並びを見るテストも緑のまま | `apply_over` の読み手で `assert!(path.is_absolute(), ...)` を 1 行足す（6 本とも絶対パスで読みに来ることを見る） | no | none |
| G1-3 | できれば | [test/doc] | src/cli/generate_tests.rs:7-14 | 「このファイルが確かめるのは 6 つ」と箇条 6 つを並べるが、`a_failure_from_the_writer_comes_back_unchanged`（570-606。書き手の失敗を握り潰さず、そのまま返す）が確かめる 7 つ目が抜けている | 「7 つ」に直し、「書き手の失敗をそのまま返すか（[`ledger_init_with`]）」を箇条に足す | no | none |
| G1-4 | できれば | [test/doc] | src/cli/cli_tests.rs:110-124 | `the_notice_for_an_unknown_subcommand_repeats_the_name_that_was_typed` は、直前の `an_unknown_subcommand_asks_for_the_usage_text_and_repeats_the_name`（96-108。`run_reporting_to` を通して断りの本文が打った綴りを含むことを見る）に含まれる。固定の本文では 2 本とも赤になり、片方だけが捕まえる壊し方が無い（151-160 の余計な引数の方は副手続きの名前を足して見るので写しではない） | 110-124 を消す | no | none |
| G1-5 | できれば | [test/doc] | src/io/sources_tests.rs:61-67 | 「このファイル自身が走査に入っていない」の確かめは、直前の 51-59（`crates/ukadoc-survey/` で始まるパスが 1 本も無い）から必ず従うので、独立して赤になることが無い。注記の「除外が『たまたま』でないこと」も 51-59 が既に言う | 61-67 を消す | no | none |
| G1-6 | できれば | [test/doc] | src/assignment_tests.rs:184-193・232-246 | `unassigned_names_the_pages_that_have_no_domain` は、割り当て済みのページを混ぜた入力で名前順・重複無しの結果を逐語で見る `unassigned_is_sorted_and_deduplicated`（196-215）に含まれる。`underscore_names_are_whole_names_not_prefixes` の 232-246（接頭辞の組が表に載っていること）は、同じ `BTreeMap` を逆向きに読むだけで、88-114 の逐語の 4 本が既に言う（1 回目の G1-17 で消した往復と同じ形の残り） | 184-193 の 1 本と 232-246 の輪を消す（224-231 の `memo` の 2 つと 247-263 の切った名前は残す） | no | none |

## ruling-conflicts

なし。

## 確かめて指摘にしなかったもの

- `io/files.rs:46-49` の「書けなかった・置き換えられなかったときは、書きかけの一時ファイルを消して」: 一時ファイルの場所がディレクトリのときは消さない（56 行）が、そのときは「書きかけの一時ファイル」がそもそも無いので文は偽ではない。
- `io/paths.rs:89-92` の `check::freshness` が `DomainReportStale` で全体報告を見ること: `check/freshness.rs:79-85` で確かめた。
- `io/paths.rs:10`・`io/sources.rs:8` の前例 `crates/log-capture-kit/tests/workspace_scan/mod.rs`: 実在する。
- `lib_test_support.rs:57-60` の「6 ページ・6 カテゴリ」「逆斜線を含む見出し 2 件」・381 行の「shiori は 12 ページ」: `CATALOG_ROWS` と `assignment.rs` の表で数え直して合っていた。
- `inspect.rs:329` の「形の版だけの違いではハッシュは同じ値」: 2 回目の G1-4 の直しの注記で、算法が同じなら同じ本文は同じ値になるので正しい。
- `diff_tests.rs:539-556` の `the_review_list_follows_id_order_not_ledger_order`: 前向きの主張は主の等式と重なるが、台帳を逆に渡して答えが変わらないことは他のテストが持たないので写しではない。
- `model_tests.rs:302-303` の `parse_theme("気配")`・`parse_theme("気配り")`: `every_theme_round_trips` と重なるが、接頭辞の組を否定の側と対で並べる 1 行ずつなので指摘にしない。
- `snapshot_tests.rs:400-403` の `unknown_is_spelled_exactly_once`: 名前は大げさだが、定数の綴りを独立した文字列で釘付けにする本体は正しい（言い換えの好みなので外した）。

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
| src/assignment_tests.rs | 264 | はい |
| src/diff.rs | 134 | はい |
| src/diff_tests.rs | 556 | はい |
| src/cli/mod.rs | 208 | はい |
| src/cli/cli_tests.rs | 224 | はい |
| src/cli/generate.rs | 395 | はい |
| src/cli/generate_tests.rs | 617 | はい |
| src/cli/generate_priority_tests.rs | 378 | はい |
| src/cli/inspect.rs | 373 | はい |
| src/cli/inspect_tests.rs | 654 | はい |
| src/io/mod.rs | 15 | はい |
| src/io/files.rs | 83 | はい |
| src/io/files_tests.rs | 280 | はい |
| src/io/paths.rs | 119 | はい |
| src/io/paths_tests.rs | 157 | はい |
| src/io/snapshot.rs | 329 | はい |
| src/io/snapshot_tests.rs | 403 | はい |
| src/io/sources.rs | 97 | はい |
| src/io/sources_tests.rs | 187 | はい |
