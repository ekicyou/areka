# かけ直し 4 回目・G2

範囲は src/catalog/・src/documents/・src/ledger/ の 22 ファイル。全部の行を読んだ。指摘 4 件（必ず直す 0・直したほうがよい 0・できれば 4）。どれも [test/doc] で、サブコマンドの出力にも整合テストの判定にも触らない。3 回目の 3 件の直しは届いている（カタログの表の鍵の失敗が `catalog.toml` つきの `TomlParse` になった・`parse_tests.rs:7` の「主な分岐」と根の知らない表の 1 行・`derive.rs:10-12` の束の欄と順位の行の書き分け）。ruling-conflicts に挙げた `ledger/read.rs:157-160` の注記も直っている。直しが新しく壊したものは見つからなかった。どれも残す 3 種類に当たらない。

3 件（G2-1〜G2-3）は同じ形をしている。落ちる経路を 1 本ずつ試すテストが、**別の確かめでも同じ語を含む失敗が出る**見本を使っているので、試したい確かめを消してもテストは緑のままになる。

| id | 重さ | file:line | 何がおかしいか | 直し方 | 振る舞い | 残す候補 |
|---|---|---|---|---|---|---|
| G2-1 | できれば [test/doc] | src/catalog/build_tests.rs:292-308（守る相手は src/catalog/build.rs:71-75） | `the_same_id_twice_fails_instead_of_being_folded` の 2 件は `raw()` が id から URL を作るので URL も同じになる。build.rs:71-75 の同じ id の確かめを消しても、78-85 行の同じ URL の確かめが `SnapshotShape`（本文に `ukadoc:dev_bind` を含む）で落とすので、このテストは緑のまま | 2 件目の `url` を別の綴りにする（例 `second.url = format!("{}?2", second.url)`）か、`detail.contains("同じ項目 id")` を足す | いいえ | なし |
| G2-2 | できれば [test/doc] | src/documents/derive_tests.rs:389-403（守る相手は src/documents/derive.rs:219-224） | `a_bundle_ranked_twice_fails` は本文に束名 `時刻の刻み` があることしか見ない。derive.rs:219-224 の「[[rank]] に 2 度現れる」を消しても、225-234 行の「2 つの束に属している（後の束は 時刻の刻み）」が束名を含む失敗で落とすので緑のまま。しかもその失敗は `linkage.md` を指し、priority_by_id の doc（189-191 行）が避けたいと言う「直す場所の取り違え」そのものになる | `body.contains("doc/ukadoc-coverage/briefing.md")`（または `"2 度現れる"`）を足す（2 つの束に属する方は `linkage.md` を添えるので見分けられる） | いいえ | なし |
| G2-3 | できれば [test/doc] | src/documents/parse_tests.rs:744（守る相手は src/documents/parse.rs:281-283） | 1 か所ずつ壊す表の「空の members」は `BUNDLE_BLOCK` の `members` だけを空にするので、`hand = ["…OnMinuteChange:1"]` が残る。parse.rs:281-283 の「欄 members が空」を消しても、318-325 行の「欄 hand の … が members に無い」が `members` を含む文で落とすので緑のまま | この行だけ `hand` も空にした見本を使う（`bundle` の差し替えを 2 か所にする）か、期待する語を `"members が空"` にする | いいえ | なし |
| G2-4 | できれば [test/doc] | src/catalog/build_tests.rs:8-9 | 「⑺ 同じ id・同じ URL・同じページ URL の 2 ページが」は誤り。同じ id の 2 件は同じページで、2 ページではない（2 ページなのは同じ URL とページ URL の場合だけ） | 「2 ページ」を「2 項目」にする | いいえ | なし |

## ruling-conflicts

なし。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/catalog/mod.rs | 108 | はい |
| src/catalog/build.rs | 191 | はい |
| src/catalog/build_tests.rs | 444 | はい |
| src/catalog/read.rs | 221 | はい |
| src/catalog/read_tests.rs | 453 | はい |
| src/catalog/write.rs | 121 | はい |
| src/catalog/write_tests.rs | 337 | はい |
| src/documents/mod.rs | 375 | はい |
| src/documents/fields.rs | 321 | はい |
| src/documents/parse.rs | 847 | はい |
| src/documents/parse_tests.rs | 956 | はい |
| src/documents/derive.rs | 269 | はい |
| src/documents/derive_tests.rs | 580 | はい |
| src/ledger/mod.rs | 127 | はい |
| src/ledger/blocks.rs | 200 | はい |
| src/ledger/blocks_tests.rs | 405 | はい |
| src/ledger/read.rs | 349 | はい |
| src/ledger/read_tests.rs | 678 | はい |
| src/ledger/write.rs | 276 | はい |
| src/ledger/write_tests.rs | 778 | はい |
| src/ledger/patch.rs | 166 | はい |
| src/ledger/patch_tests.rs | 421 | はい |

確かめて指摘にしなかったもの: `build` の id の形の失敗は素の `BadEntryId` のまま（置き場なし）だが、`catalog` の副手続きが読む入力はスナップショット 1 つだけで、build.rs:56 と build_tests.rs:279-290 が意図して決めている。1 か所ずつ壊す表のほかの行（`hand`・`machine` の重複、単独項目の 2 つの members、`none` と `bundle` の両方など）は、確かめを消すと見本が読めてしまい赤になるので覆いは効いている。`build_tests` の同じ URL の行は、URL の確かめを消すとページ URL の確かめが落とすが、本文に `ukadoc:dev_nar` が載らないので赤になる。`derive_tests` の順位表に無い束・2 つの束に属す id・単独と名前付きの置き違えは、それぞれ確かめを消すと別の文書名か緑になるので効いている。`patch_tests` の備考に隠れた見出しは、並びの確かめを消すと `DeriveMismatch` になり赤。`write_tests.rs:196-200` の「assets（24 ページ）」は `assignment.rs` の 24 ページと合う。`replace_priority` が `wanted` にあって本文に無い id を黙って飛ばすのは、`priority_apply_with`（cli/generate.rs:299-301）が 4 ドメインぶんの表を 1 本ずつに渡すための意図した形。`SNIPPET_CHARS`・`any_ledger_file` は見える範囲が要るより広いが、害も重複も無い。
