# かけ直し 5 回目・G2

範囲は src/catalog/・src/documents/・src/ledger/ の 22 ファイル。全部の行を読んだ。指摘 3 件（必ず直す 0・直したほうがよい 0・できれば 3）。[prod] は 1 件（G2-2・`priority-apply` が落ちるときの標準エラーの文だけ）、残り 2 件は [test/doc]。整合テストの判定に触るものは 0 件。どれも残す 3 種類に当たらない。

4 回目の 4 件の直しは届いている（`build_tests.rs:302-306` の「同じ項目 id」の文での見分け・`derive_tests.rs:403-408` の「[[rank]] に 2 度現れる」・`parse_tests.rs:744-750` の「members が空」・`build_tests.rs:8-9` の「2 項目」）。直しの注記（「id の確かめを消しても URL の確かめが落とす」「hand が残るので … で落ちる」「2 つの束に属している（linkage.md を指す）でも束名つきで落ちる」）は、どれもコードを追って正しいことを確かめた。直しが新しく壊したものは見つからなかった。

| id | 重さ | 種類 | file:line | 何がおかしいか | 直し方 | 振る舞い | 残す候補 |
|---|---|---|---|---|---|---|---|
| G2-1 | できれば | [test/doc] | src/ledger/write.rs:230-231・src/ledger/write.rs:27-28・src/ledger/write_tests.rs:567-568 | `check_strictly_ascending` の doc の「これが備考の中に隠れた重複を捕まえる唯一の場所である」は偽。整合検査の `check::structure::check_file_order`（src/check/structure.rs:153-167）も `file_order` の等しい隣り合わせを `LedgerOutOfOrder` で拾い（`tests/consistency/checks.rs:375` が試す）、blocks.rs:34-39 自身も「差し込みの `LedgerOutOfOrder` と整合検査の `FindingKind::LedgerOutOfOrder`」の 2 つを挙げている。write.rs:27-28 と write_tests.rs:567-568 の「ここが緩いと盲点は永久に開いたまま」「塞ぐ場所はここしかない」も、次の `check` が拾うので言い過ぎ | write.rs:230-231 を「差し込みと `priority` の置換はここで捕まえる（整合検査では `check::structure::check_file_order` が同じ形で拾う。切り分けの較正は集合で比べるので素通りする）」にし、27-28 行と write_tests.rs:567-568 の「永久に」「ここしかない」を「`ledger-init` は整合検査より先に走るので、ここが緩いと重複した塊のまま差し込みが通る」程度に改める | no | none |
| G2-2 | できれば | [prod] | src/ledger/patch.rs:87-88 | 欄 `priority` が無く、備考の行頭にだけ `priority = "…"` がある塊では、`priority_line` が備考の行を 1 本見つけ、`priority_of` が `None` を返すので「priority の欄が文字列でない」と告げる。欄は文字列でないのではなく**無い**ので、直す人を取り違えさせる。この腕（`ok_or_else`）を通るテストも patch_tests.rs に 1 本も無い（`NO_PRIORITY` は行も欄も無いので 86 行目で先に落ちる） | 文を「priority の欄が無いか文字列でない」にし、patch_tests.rs に `PRIORITY_ONLY_IN_NOTE` から `priority="C1"\n` の行を消した本文で `DeriveMismatch` と id・その文を見る 1 本を足す | yes（`priority-apply` が落ちるときの標準エラーの文だけ） | none |
| G2-3 | できれば | [test/doc] | src/documents/derive_tests.rs:352-353 | 「`linkage.md` を要る跨ぎの 1 項目」は文が壊れている（「を要る」） | 「`linkage.md` を要する跨ぎの 1 項目」にする | no | none |

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
| src/ledger/write.rs | 276 | はい |
| src/ledger/write_tests.rs | 778 | はい |
| src/ledger/patch.rs | 166 | はい |
| src/ledger/patch_tests.rs | 421 | はい |

## 確かめて指摘にしなかったもの

- 1 か所ずつ壊す表（parse_tests.rs:743-886）の全 27 行を、確かめを 1 つずつ消したときに見本が読めてしまうか、別の確かめが同じ語で落とすかで追った。4 回目に直した「空の members」のほかに、別の確かめへ流れて緑のまま残る行は無い（「override の ref が無い」は `string_field` を消せないので対象外）。
- derive_tests.rs の落ちる経路 6 本も同じく追った。`a_bundle_missing_from_the_rank_table…` は 239-246 行を消すと台帳の側の「どの束にも属さない」（`linkage.md`）で落ちるが、テストが `briefing.md` を求めるので赤になる。
- patch_tests.rs の `two_priority_lines…` は 2 本の確かめを消すと綴りの照合で落ちるが、その文に「2」が無いので赤になる。`a_priority_line_found_only_inside_a_note_is_refused` は読み直しの確かめだけが捕まえる（doc のとおり）。`a_priority_line_with_a_trailing_comment_is_refused` は綴りの照合を消すと置換が通るので赤になる。
- catalog/read.rs:73-75 の「ある URL が別の URL の先頭部分になっている例も無い」は、実データの `doc/ukadoc-coverage/catalog.toml` の 1,749 件の URL で、`#` を持たない URL が別の URL の先頭になっている例が 0 件であることを数えて確かめた。
- catalog/read.rs の `integer_field` が `usize_field` と同じ「が整数でない」の文を組むのは、2 回目の G2-2 の直しで `snapshot_version`・`format_field` のために意図して残したもの。
- blocks.rs:47 の「見出し行に復帰文字が残っていれば読めず、その行を挙げて落ちる」は blocks_tests.rs:397-405 が試している。
- mod.rs の `NamedBundle` の欄に ⑹ が無いのは、要件 4.1 ⑹（壊れる振る舞い）が地の文で書く項目で、骨組みの欄ではないため。
- derive_tests.rs:12 と 260 行の「4 台帳の全 id」は見本が 2 台帳だが、本番で渡す 4 台帳の性質を言う言い方で、`priorities` の doc（「渡された台帳の全 id」）と矛盾しない。
