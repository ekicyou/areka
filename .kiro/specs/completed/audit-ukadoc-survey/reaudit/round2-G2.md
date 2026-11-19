# かけ直し 2 回目・G2

範囲は src/catalog/・src/documents/・src/ledger/ の 22 ファイル。全部の行を読んだ。指摘 8 件（必ず直す 0・直したほうがよい 1・できれば 7）。1 回目の 13 件の直しは届いている（G2-1 の置き場の差し替え・G2-2/G2-7 のコメントの切り方・G2-3 の根の鍵・G2-4 の重複・G2-5/G2-6 の 2 つの逆引き・G2-8〜G2-13 の注記）。直しが新しく壊したものは見つからなかった。どれも残す 3 種類に当たらない。裁定とぶつかるもの無し。

| id | 重さ | file:line | 何がおかしいか | 直し方 | 振る舞い | 残す候補 |
|---|---|---|---|---|---|---|
| G2-1 | 直したほうがよい | src/ledger/read.rs:157・src/ledger/blocks.rs:152・src/ledger/mod.rs:61-68（注記は read.rs:10-11,19） | 台帳の表の鍵が id の形でないと素の `BadEntryId`（「項目 id の形が違う: ukadoc」）が上がり、4 本のどの台帳かが本文に出ない。`at_ledger_file` も `BadEntryId` を素通しするので `ledger-init`・`priority-apply` の見出しでも同じ。read.rs:10-11 は「いずれもどのファイルのどの項目かを本文に載せる」と言い、19 行目でこの場合を挙げている | `at_ledger_file` に `BadEntryId { raw } => malformed(file, format!("項目 id の形が違う: {raw}"))` の腕を足し、read.rs:157 も `.map_err(\|err\| at_ledger_file(err, file))` を通す。read_tests.rs:478-490 の期待（`BadEntryId`）を置き場つきの `TomlParse` に直す | はい（崩れた入力のときの標準エラーの文だけ） | なし |
| G2-2 | できれば | src/catalog/read.rs:198-225・src/documents/fields.rs:6-7 | `integer_field`・`count_field` は `documents::fields::usize_field`（整数でない／負を断る）の写しで、負の数の文面だけが違う（「件数にならない」と「が負」）。fields.rs:6-7 の「失敗の文面を直す場所はここ 1 か所きり」と食い違う | `usize_field` を `pub(crate)` にして `count_field` を消して呼ぶ（`integer_field` は `snapshot_version` と `format_field` に残す） | はい（負の件数のときの標準エラーの文だけ） | なし |
| G2-3 | できれば | src/catalog/build.rs:54（`build` の doc） | 「失敗するのは 4 つの場合だけ」と書いて 5 つを並べている（ページ URL の畳み込みを足したときに数が古びた） | 「次の場合に失敗する」と数を書かない形にする | いいえ | なし |
| G2-4 | できれば | src/catalog/build_tests.rs:3-9 | 「守るのは 7 つ」の ⑺ が「同じ id・同じ URL」だけで、`two_pages_folding_onto_one_page_url_fail_and_name_both` が見る「別のページが同じページ URL に畳まれる」が入っていない | ⑺ を「同じ id・同じ URL・同じページ URL の 2 ページが黙って畳まれずに失敗すること」にする | いいえ | なし |
| G2-5 | できれば | src/catalog/build_tests.rs:294-295 | 「要件 1.8 の『黙って失敗しない』」の引用が違う。その語は完了 toolkit の要件 6.12 のもので、1.8 は「スナップショットが読めない・壊れている」 | 「要件 1.8（壊れたスナップショットで失敗する）」か「要件 6.12 の『黙って失敗しない』」のどちらかに直す | いいえ | なし |
| G2-6 | できれば | src/catalog/write_tests.rs:206-215（見出しの ⑵ は 5 行目） | `writing_twice_is_byte_identical` は純粋な `write` を同じ引数で 2 回呼んで比べるだけで、指摘 66 が消した `tomlout_tests::every_function_is_byte_identical_on_a_second_call` と同じ形。中の `contains` の確かめは `entries_are_written_in_id_byte_order` と逐語のテストが持つ | テストを消し、見出しの ⑵ を外して番号を詰める（-10 行） | いいえ | なし |
| G2-7 | できれば | src/catalog/read_tests.rs:18・src/catalog/write_tests.rs:22 | `use super::*` が親の `use crate::model::{EntryId, PageName}` を連れてくるので、`use crate::model::…` の行は要らない（1 回目の G2-8 で parse_tests.rs から消したものと同じ） | 2 行を消す | いいえ | なし |
| G2-8 | できれば | src/documents/parse.rs:767-776・src/ledger/read.rs:236-250,278-305 | 1 回目の G2-4 の直しが届いていない残り。`[[reserved]]` の同じ name の 2 行（束が違ってもどちらが正か分からない）と、台帳の 1 項目の `supersedes` の同じ id・`links` の同じ（種別, 相手）が黙って通る。links の重複は、相手がカタログに無いと同じ `LinkEndpointMissing` が 2 件出る | `[[reserved]]` は `[[spec]]` と同じく `reject_repeated` を通す。`supersedes` と `links` は `read_values` と同じ形で 2 度目を malformed にする。parse_tests の 1 か所ずつ壊す表と read_tests に 1 例ずつ | はい（崩れた入力を断る） | なし（凍結した欄も語彙も変えず、`values` の重複を断るのと同じ読み手の確かめ） |

## ruling-conflicts

なし。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/catalog/mod.rs | 108 | はい |
| src/catalog/build.rs | 191 | はい |
| src/catalog/build_tests.rs | 444 | はい |
| src/catalog/read.rs | 229 | はい |
| src/catalog/read_tests.rs | 451 | はい |
| src/catalog/write.rs | 121 | はい |
| src/catalog/write_tests.rs | 350 | はい |
| src/documents/mod.rs | 375 | はい |
| src/documents/fields.rs | 321 | はい |
| src/documents/parse.rs | 841 | はい |
| src/documents/parse_tests.rs | 936 | はい |
| src/documents/derive.rs | 269 | はい |
| src/documents/derive_tests.rs | 580 | はい |
| src/ledger/mod.rs | 125 | はい |
| src/ledger/blocks.rs | 200 | はい |
| src/ledger/blocks_tests.rs | 405 | はい |
| src/ledger/read.rs | 328 | はい |
| src/ledger/read_tests.rs | 628 | はい |
| src/ledger/write.rs | 276 | はい |
| src/ledger/write_tests.rs | 778 | はい |
| src/ledger/patch.rs | 166 | はい |
| src/ledger/patch_tests.rs | 421 | はい |

確かめて指摘にしなかったもの: `[[rank]]` の `singles` の重複は `priority_by_id` が「2 度現れる」で落とす。`splicing_the_same_body_again_changes_nothing`・`replacing_twice_lands_on_the_same_text` は出力を入力に戻す冪等のテストで、66 の形ではない。`trailing_comments_start` は前置きにも当たり、`#` で閉じる備考の後ろの本物のコメントも切る（1 回目 G2-2・G2-7 の直しどおり）。`replace_priority` の切り分けの失敗に置き場を添える配線にはテストが無いが、`at_ledger_file` 自体は read と write のテストが見ている。
