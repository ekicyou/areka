# かけ直し 6 回目・G1

指摘 2 件（必ず直す 0・直したほうがよい 0・できれば 2）。どちらも [test/doc] で、副手続きの出力や振る舞いを変えるもの（[prod]）は 0 件。どちらも残す 3 種類に当たらない。読んだファイル 32 本すべて全行。5 回目の直し（G1-1・G1-2）はどちらも入っていることを確かめた（`paths_tests.rs:26` の説明・`cli_tests.rs:14-17` の言い直し）。ただし G1-2 の直しの文に添えた理由が 7 つのうち 3 つでは事実と違う（下の G1-1。5 回目の推奨文そのものに入っていた言い過ぎで、直しの向き＝「走らせて見るのは `catalog`・`candidates` だけ」は戻さない）。

| id | 重さ | 種類 | file:line | 何が違うか | 直し方 | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G1-1 | できれば | [test/doc] | src/cli/cli_tests.rs:14-17 | 走らせない理由「走らせると台帳や報告を書き出してしまう」「（走らせると台帳や報告を書くため）」は、残る 7 つのうち `check`・`evidence`・`diff` では偽。この 3 つはファイルを 1 つも書かない（`inspect.rs:3`）。走らせないのは、repo の中身（カタログ・台帳・報告）かスナップショットに寄りかかるから（`tests/cli_streams.rs:11-16` の線）。同じ理由が 14 行と 16 行で 2 度書かれてもいる | 14 行の括弧を消し、16 行の括弧を「（`ledger-init`・`report`・`report-summary`・`priority-apply` は台帳や報告を書き、`check`・`evidence`・`diff` は repo の中身かスナップショットに寄りかかる。`tests/cli_streams.rs` 冒頭の線）」に直す | no | none |
| G1-2 | できれば | [test/doc] | src/cli/inspect_tests.rs:4-5 | 「4 つの副手続き本体は repo の木（カタログ・台帳・報告）とスナップショットが要るので、ここでは走らせない」は `candidates` では偽。`candidates` は `crates/**/*.rs` しか読まず（`inspect.rs:115-118`）、`tests/cli_streams.rs:94-106` がまさにその理由で実行ファイルごと走らせている | 「`check`・`evidence`・`diff` は repo の木（カタログ・台帳・報告）かスナップショットが要り、`candidates` は本物の `crates/` を歩いて出力が repo の中身で変わるので、ここでは走らせない（`candidates` は `tests/cli_streams.rs` が走らせる）」に直す | no | none |

## ruling-conflicts

- 5 回目 G1-2（`cli_tests.rs:14-15`）: 推奨文の括弧「（走らせると台帳や報告を書くため）」が 7 つ全部の理由として書かれており、そのまま入った。直しの本体（表を通して走らせるのは `catalog`・`candidates` だけ、と言い切る）は正しいので戻さない。括弧の理由だけを G1-1 で直す。

## 確かめて指摘にしなかったもの

- `snapshot_tests.rs:148-172` の `load_warns_with_an_absolute_path_even_when_given_a_relative_one`: `crates/package.json` が実在しないことに寄りかかるが、今は無いこと（`ls` で確かめた）、置かれれば警告 0 行で赤になり黙って緑にはならないので外した。
- `snapshot.rs:130-138` で提供パッケージの警告がスナップショットの JSON の形の失敗より先に標準エラーへ出ること: 警告と失敗の両方が出るだけで、失敗は失敗として終わる（終了コード 1）。黙る経路ではない。
- `tomlout_tests.rs:7-9` の「必要以上に逃がす書き出し」の例に並べ替えが入っていること: 並べ替えは逃がしではないが、文意（読み戻し一致では捕まらない書き方の違い）は正しく、言い換えの好みなので外した。
- `error_tests.rs` が `DeriveMismatch` の本文を見ないこと: `ledger/patch_tests.rs:184` ほかと `generate_priority_tests.rs:277-297` が本文に id が載ることを見ている。
- `model_tests.rs:102-121`・`:186-204` が `ALL` でなく手書きの並びで往復を見ること: `ALL` に同じ変種を 2 度書いて 1 つ落とす壊し方を捕まえるのは手書きの並びの側なので、写しではない。
- `paths_tests.rs:9` の「存在を確かめる読み取り（`is_file`）だけ」: 50 行の `is_dir` も同じ種類の読み取りで、ファイルを作らない主張は正しい。
- `files.rs:18` の「この repo に `.gitattributes` は無く」: 根に無いことを確かめた。
- `paths.rs:109`・`sources.rs:8` の前例 `crates/log-capture-kit/tests/workspace_scan/mod.rs`、`Cargo.toml:23` の前例 `crates/dola/Cargo.toml`（`serde_json = "1"`）: どちらも実在する。
- `generate.rs:153` の `load_catalog` の共用先 5 つ（`check`・`evidence`・`diff`・`ledger-init`・`report-summary`）: 呼び出しを数えて合っていた。

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
| src/cli/cli_tests.rs | 209 | はい |
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
