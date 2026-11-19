# かけ直し 3 回目・G2

範囲は src/catalog/・src/documents/・src/ledger/ の 22 ファイル。全部の行を読んだ。指摘 3 件（必ず直す 0・直したほうがよい 0・できれば 3）。2 回目の 8 件の直しは届いている（`at_ledger_file` の `BadEntryId` の腕・`usize_field` の共有・build の doc の数・build_tests の ⑺・要件 1.8 の引用・`writing_twice_is_byte_identical` の削除と番号の詰め・`use` の 2 行・`[[reserved]]`／`supersedes`／`links` の重複）。直しが新しく壊したものは見つからなかった。どれも残す 3 種類に当たらない。

| id | 重さ | file:line | 何がおかしいか | 直し方 | 振る舞い | 残す候補 |
|---|---|---|---|---|---|---|
| G2-1 | できれば [prod] | src/catalog/read.rs:140-141・src/catalog/read_tests.rs:223-233 | カタログの表の鍵が id の形でないと、素の `BadEntryId`（「項目 id の形が違う: ukadoc」）が上がり、`catalog.toml` の名が本文に出ない。2 回目の G2-1 で台帳は置き場つきにしたので、`check`・`inspect` が読む 3 種類（3 文書・台帳・カタログ）のうちカタログだけが置き場を欠く（3 文書は `id_array_field`、台帳は `at_ledger_file` が添える）。read.rs:9-10 の「いずれも**どこが**おかしいかを本文に載せる」とも合わない | read.rs:141 を `EntryId::parse(raw_id).map_err(\|_\| malformed(CATALOG_FILE, format!("項目 id の形が違う: {raw_id}")))?` にし、read_tests.rs:229-232 の期待（`BadEntryId`）を `catalog.toml` を含む `TomlParse` に直す | はい（崩れたカタログを読んだときの標準エラーの文だけ） | なし（カタログに写す正典の読み方は変えない・失敗の文面だけ） |
| G2-2 | できれば [test/doc] | src/documents/parse_tests.rs:7 | 「守るのは次の 4 つと、形の検査の分岐の全部である」は言い過ぎ。根の知らない表（`reject_unknown_keys(&root, &LINKAGE_TABLES…)` ほか 3 文書とも）・`bool_field` の型違い・`array_of_tables` の「配列でない」・`[stage]` の知らない段階・`[tally.singles_by_domain]` の知らない鍵・`[[template]]` の人が読む欄の型違いを試すテストは 1 本も無い（カタログと台帳の根の知らない鍵には在中テストがある） | 「全部」を「主な分岐」に改め、根の知らない表だけは 1 か所ずつ壊す表（parse_tests.rs:743-872）に 1 行足す（`linkage(…)` に `[extra]\na = 1` を混ぜて `"extra"` を期待） | いいえ | なし |
| G2-3 | できれば [test/doc] | src/documents/derive.rs:10-12 | 「束の欄のうち 4 つは人が書かない——テーマの和集合・跨ぐドメイン・資産の広さ・基盤共有度」は誤り。束の欄（`NamedBundle`）にあるのは `themes`・`domains` の 2 つで、`assets`・`shared` は `[[rank]]` の行の欄（`RankRow`）。直後の「判定 ⑷-d と ⑶-e」は正しく分けている | 「束ごとの値のうち 4 つは人が決めない——束の欄の `themes`・`domains` と、順位の行の `assets`・`shared`」にする | いいえ | なし |

## ruling-conflicts

- **2 回目 G2-1 のうち `src/ledger/read.rs:157-158` の部分**: `read_entries` の `EntryId::parse(raw_id).map_err(\|err\| at_ledger_file(err, file))` の失敗の腕には届かない。read.rs:103-104 の `blocks::split_parsed` が先に走り、行頭の見出しの id は `header_id` で 1 つずつ確かめられ（崩れていればそこで `BadEntryId`→`at_ledger_file`）、`calibrate` が「切り分けた id の集合＝`toml` の鍵の集合」を確かめるので、ここまで来た鍵はすべて 2 形のどれかである。read_tests.rs:479-493 `a_malformed_table_key_is_rejected` も 104 行目の経路で緑になっており、157-158 行の腕を外しても赤にならない。害は無いので直さなくてよいが、157 行の注記「綴りが 2 形のどちらでもなければ、置き場つきで落とす」はここで落ちることがあるように読める。直すなら注記を「切り分けの較正を通った鍵なので失敗しない（失敗の文面は 104 行目が決める）」にする程度（2 回目の直しを外す提案ではない）。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/catalog/mod.rs | 108 | はい |
| src/catalog/build.rs | 191 | はい |
| src/catalog/build_tests.rs | 444 | はい |
| src/catalog/read.rs | 219 | はい |
| src/catalog/read_tests.rs | 451 | はい |
| src/catalog/write.rs | 121 | はい |
| src/catalog/write_tests.rs | 337 | はい |
| src/documents/mod.rs | 375 | はい |
| src/documents/fields.rs | 321 | はい |
| src/documents/parse.rs | 847 | はい |
| src/documents/parse_tests.rs | 948 | はい |
| src/documents/derive.rs | 269 | はい |
| src/documents/derive_tests.rs | 580 | はい |
| src/ledger/mod.rs | 127 | はい |
| src/ledger/blocks.rs | 200 | はい |
| src/ledger/blocks_tests.rs | 405 | はい |
| src/ledger/read.rs | 347 | はい |
| src/ledger/read_tests.rs | 678 | はい |
| src/ledger/write.rs | 276 | はい |
| src/ledger/write_tests.rs | 778 | はい |
| src/ledger/patch.rs | 166 | はい |
| src/ledger/patch_tests.rs | 421 | はい |

確かめて指摘にしなかったもの: `[[template]]` の `ids` の重複は拒まないが、読むのは `derive_bundle` だけで集合に畳むので数は変わらない。`[[rank]]` の `singles` の重複は `priority_by_id` が「2 度現れる」で落とす。derive.rs・derive_tests.rs・mod.rs の判定の腕の名（⑷-c・⑷-d・⑷-e・⑶-e・⑶-f・⑸-e）は `tests/consistency/briefing_arms.rs`・`linkage_checks.rs`・`spec_checks.rs` の腕の見出しと合う。derive_tests.rs:10 の「落ちるべき経路の全部」は `priorities` の doc の 6 つの失敗をすべて 1 本ずつ持つので言い過ぎではない。`trailing_comments_start` は doc（末尾から見て、切った前半が読める最も前の位置）どおり。`patch_tests.rs:231` の `contains("2")` は id に 2 を含まないので本数の確かめとして効く。parse.rs:26 の `tests/consistency/examples.rs` は `toml_blocks`・`quoted_ids` を実際に呼ぶ。
