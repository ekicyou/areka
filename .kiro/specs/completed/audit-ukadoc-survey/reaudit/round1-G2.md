# かけ直し 1 回目・G2（カタログ・3 文書・台帳）

範囲は src/catalog/・src/documents/・src/ledger/ の 22 ファイル。全部の行を読んだ。指摘 13 件（必ず直す 0・直したほうがよい 5・できれば 8）。どれも残す 3 種類に当たらない（G2-1・3・4・5 は裁定 15・17・19・26・29 と同じ向きで手当てが届いていない所）。裁定とぶつかるもの無し。

| id | 重さ | file:line | 何がおかしいか | 直し方 | 振る舞い |
|---|---|---|---|---|---|
| G2-1 | 直したほうがよい | src/ledger/write.rs:168-204（splice）・src/ledger/patch.rs:70-72・151-157 | 指摘 26 は ledger::read だけを直した。ledger-init は既存本文を merge_initial→blocks::split に直に渡すので、TOML の失敗も LedgerSplitMismatch も置き場が `ledger/<ドメイン>.toml` のまま。priority-apply の replace_priority も並びの崩れ（patch_tests.rs:399 が綴りを釘付け）と DeriveMismatch に `<ドメイン>` を添える | read.rs:99-105 の差し替えを ledger の共有の関数 1 つにして splice でも呼ぶ。replace_priority は置き場を引数で受け取る（呼び手 generate.rs は ledger.domain を持つ）。patch_tests.rs:399 の期待値も直す | はい（崩れた入力のときの標準エラーの文だけ） |
| G2-2 | 直したほうがよい | src/ledger/write.rs:180-191 | 指摘 25 の直しは前の塊の尻尾のコメントにしか効かない。最初の塊の真上のコメントは前置きの尻尾にあるので、最初の塊より小さい id を差し込むとそのコメントの後ろに入る。今の 4 本は最初の項目の真上にコメントが無い | 前置きにも trailing_comments_start を当て、切った尻尾を最初の塊の前の held に入れる。write_tests.rs に例を 1 本 | はい（その形の入力のときだけ） |
| G2-3 | 直したほうがよい | src/ledger/read.rs:89-126 | 根に知らない鍵があっても落とさない（カタログの指摘 19 と同じ穴）。23-26 行の doc が「素通りさせない」と書く形そのもの。今の 4 本の根は ledger と entry だけ | read の頭で reject_unknown_keys(&root, &[LEDGER_TABLE, ENTRY_TABLE], "根", &file)。read_tests.rs に例を 1 本 | はい（崩れた入力を断る） |
| G2-4 | 直したほうがよい | src/documents/parse.rs:342・345・585-668・704-718 | themes・domains の重複と、[[after]]（同じ domain）・[[barrier]]（同じ page）・[[owner_completed]]（同じ spec）・[[spec]]（同じ name）・[[template]]（同じ name）の重複行が黙って通る | 読み手で重複を malformed にする（reject_duplicate_ids と同じ形）。parse_tests の 1 か所ずつ壊す表に行を足す | はい（崩れた入力を断る） |
| G2-5 | 直したほうがよい | src/catalog/read.rs:131-170・73-74 | 指摘 15 は build だけが同じ URL を断る。check・inspect が使う read では同じ URL の 2 項目が通り by_url が片方を黙って捨てる。73-74 の注記は読みの側には言い過ぎ | read_entries でも URL→id の表を持ち 2 度目で malformed（build と同じ文面）。read_tests.rs に例を 1 本。73-74 は「組み立てと読み取りが拒む」に | はい（崩れた入力を断る） |
| G2-6 | できれば | src/catalog/read.rs:87-98（page_urls） | 2 つの別のページが同じページ URL に畳まれると後のページだけが残り、ページ単位の証拠が違うページに付く。build も read も確かめない。今のデータには無い | build と read で「ページ URL→ページ」が 1 つに決まることを確かめ、違えば SnapshotShape／malformed | はい（崩れた入力を断る） |
| G2-7 | できれば | src/ledger/write.rs:210-222 | trailing_comments_start は全部切るか切らないかの二択。`#` で始まる行で閉じる備考の後ろに本物のコメントが続くと 1 行も切らず指摘 25 の付き方に戻る | 末尾から 1 行ずつ切る位置を見て、前半が TOML として読める最後の位置を採る | はい（その形の入力のときだけ） |
| G2-8 | できれば | src/documents/parse_tests.rs:15-17 | 「use super::* は親の私有の輸入までは連れてこない」は誤り。そもそも行が要らない | 注記と use crate::model::EntryId; を消す | いいえ |
| G2-9 | できれば | src/ledger/blocks_tests.rs:4,97,140,204・read_tests.rs:4,151,251,273・write_tests.rs:4,107,264,287,458,565,594 | 完了 spec のタスク番号（「タスク 1.5 の教訓」「テスト 8」など） | 番号を消し中身だけ残す | いいえ |
| G2-10 | できれば | src/ledger/patch_tests.rs:285,362・write_tests.rs:676 | この spec の指摘番号（「指摘 7」など）を節の見出しに書いている | 番号を消し、何を確かめる節かの一言にする | いいえ |
| G2-11 | できれば | src/catalog/write_tests.rs:3-8 | 「守るのは 5 つ」—the_written_key_is_the_table_key_not_the_entry_id はどれでもない | ⑹ を足すか数を書かない形に | いいえ |
| G2-12 | できれば | src/documents/parse.rs:21-26・parse_tests.rs:12・813・821 | parse.rs は「4 つの口」、テストは「3 種の口」と数え方が割れる | 「3 つの口（引用符は toml_blocks＋quoted_ids の 2 関数）」にそろえる | いいえ |
| G2-13 | できれば | src/catalog/write.rs:318-321・src/ledger/write.rs:156-159,173-177 | 型注釈がある所に `.collect::<Vec<…>>()` を重ねている | turbofish か注釈の片方を外す | いいえ |

読んだファイル（22 本すべて全行・はい）: catalog/{mod 108, build 174, build_tests 425, read 203, read_tests 421, write 124, write_tests 349}／documents/{mod 375, fields 321, parse 794, parse_tests 875, derive 269, derive_tests 580}／ledger/{mod 109, blocks 200, blocks_tests 405, read 330, read_tests 615, write 272, write_tests 708, patch 161, patch_tests 404}。
