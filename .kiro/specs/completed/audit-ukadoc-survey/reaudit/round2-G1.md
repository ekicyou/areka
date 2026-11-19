# かけ直し 2 回目・G1

指摘 10 件（必ず直す 0・直したほうがよい 3・できれば 7）。どれも残す 3 種類に当たらない。読んだファイル 32 本すべて全行。1 回目の直し（G1-1〜G1-20）はどれも直っていることを確かめた。新しく生まれた問題は G1-2（1 回目の G1-1 の直しで足したテストの写し）と G1-6（同じく G1-5 の直しで残った冒頭の文）。

| id | 重さ | file:line | 何が違うか | 直し方 | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|
| G1-1 | 直したほうがよい | src/io/files.rs:54-63 | `write_lf` の「置き換えに失敗したら書きかけの `.tmp` を消す」腕（設計の指摘 2 の直し）を縛るテストが無い。今の 2 本は「一時ファイルが書けない」場合（`.tmp` の場所がディレクトリで消さない側）と成功の側だけで、`.tmp` は書けて `rename` だけが落ちる場合に `.tmp` が残っても緑のまま（要件 4.1） | `files_tests.rs` に 1 本足す: 置き場 `target\ukadoc-survey-tests\<テスト名>\shiori.toml` を中身のあるディレクトリにして `write_lf` を呼び、`Err` が返り `shiori.toml.tmp` が残っていないことを見る | no | none |
| G1-2 | 直したほうがよい | src/cli/generate_tests.rs:567-589 と src/cli/generate_priority_tests.rs:201-222・229-234・355-360・376-381 | 「2 本目の告知で落ちる告げ手」を組む手続きが親（ledger_init の 1 本）では手書き、子では `written_when_the_second_announcement_fails` として別に持たれる。見本の表から読む読み手のクロージャも子の中に 3 つ写しがある | `written_when_the_second_announcement_fails` と「表から読む読み手」を 1 つの関数にして親の `generate_tests.rs` に置き（子は `super::` で引ける）、3 本の告知のテストと `report_over` がそれを呼ぶ | no | none |
| G1-3 | 直したほうがよい | src/cli/inspect_tests.rs:561-570・576-584・586-596・665-676 | `two_catalogs_of_the_same_shape_get_no_warning_in_the_diff_body`・`two_catalogs_of_the_same_shape_draw_no_warning`・`a_different_hash_algorithm_draws_a_warning_that_names_both_spellings`・`the_warning_comes_before_the_four_lists` は、同じ形のカタログの本文を丸ごと等式で見る 462-533 の 3 本と、算法の違いで両方の綴りと注意の位置を見る 536-559 に含まれる言い直し | 4 本を消す | no | none |
| G1-4 | できれば | src/cli/inspect.rs:316-329 | 形の版（`catalog_format`）だけが違うときも「『本文が変わった項目』はほぼ全件に挙がる」と出す。算法が同じならハッシュは同じ値なので、その一文は誤り（読み手が本物の改訂を捨てる） | 「ほぼ全件に挙がる」の文は算法が違うときだけ足す（形の版だけのときは 2 行目までで止める） | yes（形の版だけが違う入力のときの `diff` の標準出力の 1 行だけ） | none |
| G1-5 | できれば | src/cli/generate.rs:194-196 | 「告げるのは 4 本を全部書き終えてから、読み手・書き手・告げ手を引数で受けるのは…同じ理由である」が読点で 2 文をつないだ壊れた文 | 「書き終えてから。」で切る | no | none |
| G1-6 | できれば | src/cli/generate_tests.rs:16-19・29-30 | 「2 つ目と 3 つ目は 4 つ目の再実行の主張も捕まえるが」は主語と目的語が逆（捕まえるのは 4 つ目）。「4 つ目から後の主張はこの読みだけを使う」も、3 つ目（`read_if_present`）が既に repo の台帳を読むので誤り | 「2 つ目と 3 つ目が守る壊れ方は 4 つ目の再実行の主張も捕まえるが」に、「3 つ目から後」に直す | no | none |
| G1-7 | できれば | src/io/paths.rs:90-92・src/cli/generate.rs:226 | 全体報告の突き合わせの両側・入力を「カタログと台帳 4 本だけ」と書くが、テーマ名（`THEMES`）も入る（`check/freshness.rs:18` は「カタログと台帳 4 本とテーマ名」と正しく書く） | 両方に「とテーマ名」を足す | no | none |
| G1-8 | できれば | src/lib_test_support.rs:60 | 「見出しには逆斜線を含むものを 1 件入れてある」—実際は `\e` と `\s[ID]` の 2 件 | 「2 件」に直す | no | none |
| G1-9 | できれば | src/tomlout.rs:60-61 | 「逃がしで伸びる分の余地を少しだけ見ておく」—`value.len() + 2` は前後の引用符 2 つ分だけで、逃がしの余地は無い | 注記を消す（または「前後の引用符の分」に直す） | no | none |
| G1-10 | できれば | src/io/sources_tests.rs:150-158 | 肯定側の `/src/`・`/tests/` の確かめが 43-48 行の写し。しかも注記は「本番・テスト・実行例」と言うが実行例は見ていない | 150-158 行を消す（肯定側は `walk_excludes_the_survey_crate_itself` が持つと 1 行書く） | no | none |

## ruling-conflicts

なし。

## 確かめて指摘にしなかったもの

- `files_tests.rs` が `target\ukadoc-survey-tests\<テスト名>\` にファイルを書くこと: 設計 39・183・394 行（要件 4.3）で決めてある。
- `ledger_init()` の 1 行（渡す取り寄せ）が常時テストに届かないこと: `generate.rs:120-127` に理由つきで書いてあり、repo を書かずに縛る手が無い。
- `Status::ALL` などの配列に変種を足し忘れてもコンパイルが通ること: 語彙の増減は要件の改訂を要する（`model.rs:7`）。往復テストは `ALL` を使わない手書きの並びで `parse`（`ALL` を引く）を通すので、配列からの抜けは赤になる。
- `cli_tests::the_usage_text_lists_every_name_in_the_dispatch_table`: 設計の討議 議題 4 で残すと決めた。
- `.gitattributes` が無く `core.autocrlf` が真であること（`files.rs:3-4`）: 今のワークツリーで確かめた。

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
| src/cli/generate_tests.rs | 589 | はい |
| src/cli/generate_priority_tests.rs | 424 | はい |
| src/cli/inspect.rs | 367 | はい |
| src/cli/inspect_tests.rs | 676 | はい |
| src/io/mod.rs | 15 | はい |
| src/io/files.rs | 83 | はい |
| src/io/files_tests.rs | 247 | はい |
| src/io/paths.rs | 119 | はい |
| src/io/paths_tests.rs | 157 | はい |
| src/io/snapshot.rs | 329 | はい |
| src/io/snapshot_tests.rs | 403 | はい |
| src/io/sources.rs | 97 | はい |
| src/io/sources_tests.rs | 195 | はい |
