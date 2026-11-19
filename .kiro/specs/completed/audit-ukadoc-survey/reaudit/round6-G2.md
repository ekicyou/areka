# かけ直し 6 回目・G2

範囲は src/catalog/・src/documents/・src/ledger/ の 22 ファイル。全部の行を読んだ。指摘 4 件（必ず直す 0・直したほうがよい 0・できれば 4）。どれも [test/doc] で、サブコマンドの出力にも整合テストの判定にも触らない。どれも残す 3 種類に当たらない。

5 回目の 3 件の直しは届いている（`ledger/write.rs:27-29`・`:231-233`・`write_tests.rs:567-568` の言い過ぎを「次の整合検査では `check::structure::check_file_order` が同じ形で拾う」に改めた＝`src/check/structure.rs:155-167` が `pair[1] > pair[0]` でない隣り合わせ、つまり等しい隣も拾うことを確かめた・`patch.rs:88` の「priority の欄が無いか文字列でない」と、その腕を通る `patch_tests.rs:337-373` の 1 本・`derive_tests.rs:352` の「を要する」）。足したテストは、欄が無く備考の行頭にだけ当たる本文で 86 行目を通り抜けて 87-88 行目で落ちることをコードで追って確かめた。直しが新しく壊したものは見つからなかった。G2-3 は 5 回目の G2-2 の直しの取り残し（doc の「落ちる場合」に新しい腕が載っていない）。

| id | 重さ | 種類 | file:line | 何がおかしいか | 直し方 | 振る舞い | 残す候補 |
|---|---|---|---|---|---|---|---|
| G2-1 | できれば | [test/doc] | src/documents/parse.rs:23-26・:136 | 「判定 ⑴ が拾う id の口…囲みの中の引用符付き（[`toml_blocks`] ＋ [`quoted_ids`] の 2 関数）」は偽。判定 ⑴（`tests/consistency/documents_checks.rs:292-308` の `citation_findings`）は `documents.rs:148-153` の `cited_ids` を通して `quoted_ids` を**本文全体**へ掛けており、`toml_blocks` は呼ばない（囲みで切ってから掛けるのは `examples.rs:80-83` の方）。地の文の `"ukadoc:…"` も引用符の口が拾い、`bare_id_tokens` が引用符の中を飛ばす（161-164 行）のはそれが前提。136 行の「囲みの中は見ない——あちらは [`quoted_ids`] の持ち場」も、引用符の口が囲みの中だけを見るように読める | 23-26 行を「引用符付き（[`quoted_ids`]。判定 ⑴ は本文全体へ掛け、地の文の引用符付きもここで拾う。`tests/consistency/examples.rs` は [`toml_blocks`] で切った囲みへ掛ける）」に、136 行を「囲みの中は見ない（引用符付きは [`quoted_ids`] の持ち場）」にする。口の数は 3 のまま（1 回目の G2-12 を崩さない） | no | none |
| G2-2 | できれば | [test/doc] | src/documents/parse_tests.rs:907-948（守る相手は src/documents/parse.rs:161-164） | `bare_id_tokens` が地の文の二重引用符の中を飛ばす腕を試すテストが無い。`the_three_id_scanners…` の見本は地の文に引用符を 1 つも持たず、実データの 3 文書も地の文の `"ukadoc:` は 0 件（数えて確かめた）なので、161-164 行を消しても在中テストも整合テストも緑のまま。消すと、次に地の文へ引用符で id を書いた人に判定 ⑴ が誤って「裸」と告げる | 見本の地の文に `"ukadoc:list_shiori_event:OnClose:1"` を引用符で 1 つ足し、`bare_id_tokens(&markdown)` が空のままであることと、`quoted_ids(&markdown)`（本文全体）がその綴りを含むことを並べて見る | no | none |
| G2-3 | できれば | [test/doc] | src/ledger/patch.rs:62-68 | `replace_priority` の「落ちる場合」が、5 回目に直した 87-88 行の腕（欄の `priority` が無いか文字列でない）を挙げていない。列挙は「行頭の `priority = ` が無い・2 本以上・今の値の綴りと違う、または読み直した値が違う」だけで、行は 1 本あるが欄が無い塊の失敗がどれにも当たらない | 66-67 行に「欄の `priority` が無いか文字列でない」を足す（例「塊の中に行頭の `priority = ` が無い・2 本以上ある、欄の `priority` が無いか文字列でない、行が今の値の綴りと違う、または…」） | no | none |
| G2-4 | できれば | [test/doc] | src/ledger/patch_tests.rs:178-191（守る相手は src/ledger/patch.rs:153） | `a_block_without_a_priority_line_fails_naming_the_id` は `reason.contains("priority")` しか見ない。見本 `NO_PRIORITY` は行も欄も無いので、153 行の「priority の行が無い」を消して空の範囲を返すようにしても、87-88 行の「priority の欄が無いか文字列でない」が id と `priority` を含む文で落とし、テストは緑のまま（4 回目の G2-1〜G2-3 と同じ形） | 期待する語を `"priority の行が無い"` にする | no | none |

## ruling-conflicts

なし。

## 読んだファイル

| ファイル（crates/ukadoc-survey/ から） | 行数 | 読んだ |
|---|---|---|
| src/catalog/mod.rs | 108 | はい |
| src/catalog/build.rs | 191 | はい |
| src/catalog/build_tests.rs | 446 | はい |
| src/catalog/read.rs | 221 | はい |
| src/catalog/read_tests.rs | 453 | はい |
| src/catalog/write.rs | 121 | はい |
| src/catalog/write_tests.rs | 337 | はい |
| src/documents/mod.rs | 375 | はい |
| src/documents/fields.rs | 321 | はい |
| src/documents/parse.rs | 847 | はい |
| src/documents/parse_tests.rs | 962 | はい |
| src/documents/derive.rs | 269 | はい |
| src/documents/derive_tests.rs | 586 | はい |
| src/ledger/mod.rs | 127 | はい |
| src/ledger/blocks.rs | 200 | はい |
| src/ledger/blocks_tests.rs | 405 | はい |
| src/ledger/read.rs | 349 | はい |
| src/ledger/read_tests.rs | 678 | はい |
| src/ledger/write.rs | 278 | はい |
| src/ledger/write_tests.rs | 778 | はい |
| src/ledger/patch.rs | 166 | はい |
| src/ledger/patch_tests.rs | 459 | はい |

## 確かめて指摘にしなかったもの

- 言語名が `toml` でない囲み（`briefing.md` の ```` ```sh ```` 2 つ）の中の逆引用符付き・裸の id はどの口にも掛からない。`parse_tests.rs:217-235` が「例の囲み」を地の文から外すことを意図して釘付けしており、実データの `sh` の囲みにも id は無いので指摘にしない。
- `patch_tests.rs:337-373`（5 回目に足した 1 本）は、86 行目で備考の 1 本に当たり、87-88 行目で落ちる。`a_priority_line_found_only_inside_a_note_is_refused` は読み直しの確かめだけが捕まえ、`a_priority_line_with_a_trailing_comment_is_refused` は綴りの照合を消すと置換が通るので赤になる（5 回目の記録どおり）。
- `write_tests.rs:547-548` の「塞ぐのは厳密な昇順の判定だけである」は、差し込みの `check_strictly_ascending` と整合検査の `check_file_order` のどちらも厳密な昇順の判定なので偽ではない。
- `ledger/mod.rs:112-113` の「判定そのものは差し込みと整合検査の担当」は `priority-apply` の頭の確かめを挙げないが、`blocks.rs:37-39` と `write.rs:29-30` が挙げており、担当の言い方として誤りではない。
- `read_tests.rs:320-329` の「別のページが同じページ URL」は、ページの確かめを消すと id の URL が相異なるので通ってしまい赤になる。`the_same_url_on_two_entries_is_rejected` は URL の確かめを消すと同じページなのでページの確かめも通り、赤になる。
- `ledger/write.rs` の `splice` で塊が 0 個のとき、前置きの尻尾のコメントの後ろに差し込む。コメントは次の項目の見出しとして読めるので害は無い。
- `blocks_tests.rs:397-405` の復帰文字は、本文全体の読み取り（`toml` は CRLF を受ける）を通った後に見出し行の `\r` で落ちることを追って確かめた。
