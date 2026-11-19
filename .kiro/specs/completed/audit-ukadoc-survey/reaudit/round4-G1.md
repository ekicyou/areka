# かけ直し 4 回目・G1

指摘 5 件（必ず直す 0・直したほうがよい 0・できれば 5）。どれも [test/doc] で、副手続きの出力や振る舞いを変えるもの（[prod]）は 0 件。どれも残す 3 種類に当たらない。読んだファイル 32 本すべて全行。3 回目の直し（G1-1〜G1-6）はどれも直っていることを確かめた（`inspect_tests.rs` の 2 本と `meta` の消去・`apply_over` の絶対パスの確かめ・`generate_tests.rs` の箇条 7 つ・`cli_tests.rs` の 1 本・`sources_tests.rs` の名指し・`assignment_tests.rs` の 1 本と輪）。3 回目の直しから生まれた新しい問題は無い。

| id | 重さ | 種類 | file:line | 何が違うか | 直し方 | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G1-1 | できれば | [test/doc] | src/error.rs:5-6 | 「どの変種も『探した絶対パス』『読めない理由』『形が違う場所』のいずれかを本文に載せる」は偽。`CheckFindings` は件数だけを運ぶ（同じファイルの 57-64 行がそう書く） | 「`CheckFindings` を除くどの変種も」に直す | no | none |
| G1-2 | できれば | [test/doc] | src/diff_tests.rs:45 | id 一覧の表の「11. OnSecondChange 本文は同じ・見出しと分類と版番号**だけ**変わった」は偽。`entry` は URL を見出しから導く（253 行）ので URL も変わり、157-158 行・400-401 行の注記と `sample_shape_premises_hold` の 430 行（`assert_ne!` の url）もそう言う | 「見出しと分類と版番号（したがって URL も）が変わった」に直す | no | none |
| G1-3 | できれば | [test/doc] | src/io/files_tests.rs:64-84（と 18-24 の `own_manifest`） | `read_normalized_returns_a_body_without_carriage_returns` は、同じ `crates/ukadoc-survey/Cargo.toml` を読む 96-127 に含まれる。あちらも「復帰文字が無い」「本文が空でない」を見て、しかも生のバイト列との文字数の差が復帰文字の個数に一致することまで見るので、別のファイルや空の本文を返す壊し方はどちらでも赤になる。片方だけが捕まえる壊し方が無い | 64-84 の 1 本と、それだけが使う `own_manifest` を消す | no | none |
| G1-4 | できれば | [test/doc] | src/cli/generate_tests.rs:121-125 | `a_domain_that_owns_no_entry_still_gets_a_bucket` の「鍵を並べて重複を落として 4 個」は、`BTreeMap` の鍵が初めから重複しないうえ、続く 126-137 の `Domain::ALL` の輪が 4 つの鍵の在りかを 1 つずつ確かめるので、独立して赤になることが無い | 121-125 を消す（126-137 の輪だけを残す） | no | none |
| G1-5 | できれば | [test/doc] | src/io/snapshot.rs:133-136（と snapshot_tests.rs） | `load_with` が `package_info` の成功の値（名前と版）を `SnapshotDoc` へ渡す配線を見るテストが無い。今あるのは `package_fields`・`package_json_path` の単体と、`package.json` が無くて `unknown` に落ちる側（警告 1 行）だけなので、成功の値を捨てて常に `unknown` を入れる壊し方でも、警告は出ないのに緑のまま（要件 1.6）。捕まえるのは実データの通し確認だけ | `files_tests.rs` と同じ置き場（`target\ukadoc-survey-tests\<テスト名>\`）に `package.json` と `data\index.json`（最小の JSON）を置き、`load_with` が警告 0 行で名前と版をそのまま返すことを見る 1 本を足す（冒頭の「ファイルも作らない」の注記もそれに合わせる） | no | none |

## ruling-conflicts

なし。

## 確かめて指摘にしなかったもの

- `model.rs:177` ほかの「ページ名は 2 番目の区切りから取る」: 完了済み spec の設計（`areka-P0-ukadoc-survey-toolkit/design.md` 266・612・964 行）が「区切り」を欄の意で使う決まった言い方で、crate の中の 5 か所も同じ。言い換えの好みなので外した。
- `model_tests.rs:230`・`:271` と `paths_tests.rs:406-410`・`:432-436` の長さの確かめ: 配列の長さは型で決まるが、語彙を 1 つ足して `ALL` の長さを変えると赤になるので、失敗しえないテストではない。
- `inspect_tests.rs:250-272`・`:165-198` の「種類ごと・理由ごとに別の本文」: 逐語のテストが綴りを固定するのは一部の種類・理由だけなので、残りの綴りが互いに潰れる壊し方はこの 2 本だけが捕まえる。
- `generate_tests.rs:494`（書いた本数が 4）: 499 行の場所の並びの等式に含まれるが、同じテストの中の 1 行の前置きなので指摘にしない。
- `sources_tests.rs:72-76`（`/` を含むパスがある）: 他のテストの `starts_with("crates/")` から従うが、別のテストの中の空回り止めなので外した。
- `cli_tests.rs:134-143`: 3 回目の記録で「副手続きの名前を足して見るので写しではない」と決めてある。
- `io/sources.rs:74-81` が記号リンクを黙って飛ばすこと: 「黙って飛ばさない」の約束は列挙・読み込みの失敗についてで、記号リンクは失敗ではない。この repo の `crates/` に記号リンクは無い。
- `generate.rs:120-127` の `ledger_init()` の 1 行が常時テストに届かないこと: 2 回目の記録のとおり理由つきで書いてある。
- `inspect.rs:329` の「形の版だけの違いではハッシュは同じ値」: 3 回目の記録で確かめてある。

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
| src/cli/generate_tests.rs | 618 | はい |
| src/cli/generate_priority_tests.rs | 383 | はい |
| src/cli/inspect.rs | 373 | はい |
| src/cli/inspect_tests.rs | 616 | はい |
| src/io/mod.rs | 15 | はい |
| src/io/files.rs | 83 | はい |
| src/io/files_tests.rs | 280 | はい |
| src/io/paths.rs | 119 | はい |
| src/io/paths_tests.rs | 157 | はい |
| src/io/snapshot.rs | 329 | はい |
| src/io/snapshot_tests.rs | 403 | はい |
| src/io/sources.rs | 97 | はい |
| src/io/sources_tests.rs | 179 | はい |
