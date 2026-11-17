# かけ直し 2 回目・G3

範囲は `crates/ukadoc-survey/` の `src/check/`・`src/evidence/`・`src/report/` の全ファイル（28 本・8,791 行）。カタログ・台帳・証拠・報告本文を値で受け取って、食い違いの一覧・証拠の索引・手掛かり候補・報告本文を返す純粋な層です（ファイルには触れません）。負荷は、開発者 1 人が副手続きや整合テストを回すだけと見ました。

読み手としてはファイルを 1 つも変えていません（この記録だけを書いた）。状態を変える git の操作もしていません。1 回目（`round1-G3.md`）の 16 件は直っていることを確かめ、蒸し返していません。1 回目の直しが残した新しい問題は G3-6 の 1 件です（範囲の外のファイル）。

## 指摘

| id | 重さ | 場所 | 何が悪いか | 直し方 | 道具の振る舞いに触るか | 残す種類の候補 |
|---|---|---|---|---|---|---|
| G3-1 | 直したほうがよい | `src/check/structure_tests.rs:600-613`・`src/check/content_link_tests.rs:655-688`・`src/report/tally_tests.rs:385-398`・`src/report/bundle_tests.rs:302-310` | 純粋な関数を同じ入力で 2 回呼んで比べるだけのテストが 4 本残っている（指摘 66 と同じ種類）。どれも `BTreeMap`／`BTreeSet` だけで組まれた関数で、2 回の結果が食い違う道筋が無いので、決して赤にならない。並びが入力に依らないことは、逆順で通すテスト（`the_order_of_the_ledgers_does_not_change_*`・`shuffling_the_input_*`）が別に守っている | 4 本を消す（約 −55 行） | いいえ | なし。指摘 66 の一覧から漏れていただけで、残す裁定は無い |
| G3-2 | 直したほうがよい | `src/report/domain.rs:63-69,90-97,169-184`・`src/report/summary.rs:117-124,140-148,237-253` | ドメイン別報告と全体報告に、同じ組み立てが 3 か所写してある。⑴「状態 \| 件数」の表と合計行 ⑵ テーマ別の表（`theme_names` → 空の分布 → `push_status_row`）⑶ `links` を辺の列にする関数（`domain_links(ledger)` は `all_links(&[ledger])` と同じ）。片方の版面や辺の規則だけを変えると、2 つの報告が黙ってずれる | ⑴⑵ は `tally.rs` の `push_status_header` の隣に `push_overall_table`・`push_theme_table` として 1 つずつ置き、⑶ は `all_links` を `bundle.rs` へ移して両方から呼ぶ（本文は変わらない＝逐語のテスト 2 本が確かめる） | いいえ（出力のバイトは変わらない） | なし |
| G3-3 | できれば | `src/evidence/candidates_tests.rs:95-103` | 見本の `index()` が `resolve::index_of`（取り出し → 解決）を手で写している。`resolve.rs:68-71` は「`check`・`evidence` の副手続きとテストの見本が同じ経路を通るための 1 か所」と言うが、このテストだけが別の集め方をしている | 中身を `crate::evidence::resolve::index_of(&sources(list), &catalog())` の 1 行にする | いいえ | なし |
| G3-4 | できれば | `src/check/finding_tests.rs:62-84,105-129,236-267` | 逐語のテストに丸ごと含まれる主張が 4 本ある。`the_line_of_a_finding_carries_its_id`（id・場所・詳細が本文にある）と `two_findings_in_the_same_place_share_one_place_line` は 25-60 行の逐語の本文に含まれる。`the_kind_keys_are_pinned_verbatim` は 187-234 行の見出しの並び（`render` が `ALL` の順に `as_key` を書く）と同じ 13 個の綴りを並べ直しただけで、`all_thirteen_kinds_are_declared_once_each` の 13 は配列の型 `[Self; 13]` が、重複無しはその逐語の並びが決める | 4 本を消す（約 −60 行）。README と綴りを結ぶ注記は 187 行のテストへ移す | いいえ | なし |
| G3-5 | できれば | `src/report/tally_tests.rs:155-169`・`src/report/bundle_tests.rs:86-91,228-252` | 言い直しのテストが 5 本ある。`status_counts_over_an_entry_list_matches_the_overall_distribution` は `tally.rs:143` が `overall` を `status_counts` そのもので作るので、同じ関数の出力どうしを比べている。`the_page_distribution_is_keyed_by_page_name_in_name_order` は直後の `a`／`a0` のテストに含まれる。`each_connected_component_becomes_exactly_one_bundle` は 94-107 行が同じ等式を持ち、`the_bundle_id_is_the_smallest_member_not_the_first_one_seen` と `members_are_listed_in_byte_ascending_order` は `expected()` との等式（yankee が先に現れて xray が束 id）に含まれる | 5 本を消す（約 −45 行） | いいえ | なし |
| G3-6 | できれば | `tests/consistency/values_md.rs:48`（G3 の外・G4 の範囲） | 1 回目の G3-15 で字句の読み手を `evidence/lex.rs` へ移したが、注記はまだ `evidence::candidates::slice_element_names` と古い場所を名指している | `evidence::lex::slice_element_names` に直す | いいえ | なし |
| G3-7 | できれば | `src/report/bundle_tests.rs:138-139,157-158` | 「旧い綴り 2 つが同じ正典 id を `alias_of` で指すと、対は (旧1, 新)・(旧2, 新) になる」「`supersedes` する形」と書くが、`alias_of`・`supersedes` の欄は辺にならない（`domain.rs:174`・`summary.rs:32-34`）。辺になるのは `links` に `kind = "alias_of"` などで書いたときだけ | 「`links` に `kind = "alias_of"` で 2 つの旧い綴りから同じ正典 id を指すと」のように、`links` の kind の話だと書く | いいえ | なし |
| G3-8 | できれば | `src/report/domain.rs:174`・`src/report/summary.rs:242` | 「`alias_of` と `supersedes` は辺にしない」は、欄の話か kind の話かを言わない。同じファイルの `summary.rs:32-34` は「欄は辺にしない・kind としてなら `links` に書ける」と直してあり、2 か所の注記だけが古い言い方のまま | 「`alias_of`・`supersedes` の欄は辺にしない（`links` の kind としてなら辺になる）」にそろえる（G3-2 で 1 か所にまとめれば 1 回で済む） | いいえ | なし |
| G3-9 | できれば | `src/evidence/resolve.rs:13-14` | 「同じ URL を持つ 2 項目はカタログの組み立て（`catalog::build`）が拒む」と組み立てだけを名指すが、`check` と `evidence` が `resolve` へ渡すカタログは `catalog.toml` を `catalog::read` で読んだもので、拒んでいるのは `catalog/read.rs:171-187` の方 | 「カタログの読み込み（`catalog::read`）と組み立て（`catalog::build`）がともに拒むので」にする | いいえ | なし |
| G3-10 | できれば | `src/check/mod.rs:1` | 「整合検査の入口（要件 6.3〜6.8・…）」の 6.8（テーマ名の実在）は、今は `check` の判定ではない。`finding.rs:63-65`・`content.rs:11-12` が言うとおり `ledger::read` の段で落ちる | 「6.3〜6.7・6.10〜6.12・7.4・7.5」にする | いいえ | なし |
| G3-11 | できれば | `src/report/tally_tests.rs:177-178` | 「`spec_shiori3` は既にあるので `spec_shiori` が 1 ページ増えればその日に生じる」は言い過ぎ。食い違いが生じるのは新しいページの id がアンカー付き（`ukadoc:spec_shiori:x:1`）のときだけで、アンカー無しの `ukadoc:spec_shiori` は `ukadoc:spec_shiori3` より前に並び、ページ名の順と一致する | 「`spec_shiori` にアンカー付きの項目が増えれば」と条件を書くか、この 2 行を消す | いいえ | なし |

## ruling-conflicts

- なし。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/check/mod.rs | 79 | はい |
| src/check/finding.rs | 201 | はい |
| src/check/finding_tests.rs | 295 | はい |
| src/check/content.rs | 252 | はい |
| src/check/content_tests.rs | 354 | はい |
| src/check/content_link_tests.rs | 688 | はい |
| src/check/freshness.rs | 129 | はい |
| src/check/freshness_tests.rs | 569 | はい |
| src/check/structure.rs | 233 | はい |
| src/check/structure_tests.rs | 753 | はい |
| src/check/test_support.rs | 118 | はい |
| src/evidence/mod.rs | 128 | はい |
| src/evidence/extract.rs | 93 | はい |
| src/evidence/extract_tests.rs | 313 | はい |
| src/evidence/lex.rs | 266 | はい |
| src/evidence/resolve.rs | 309 | はい |
| src/evidence/resolve_tests.rs | 832 | はい |
| src/evidence/candidates.rs | 385 | はい |
| src/evidence/candidates_tests.rs | 454 | はい |
| src/report/mod.rs | 8 | はい（指摘なし） |
| src/report/bundle.rs | 130 | はい（指摘なし） |
| src/report/bundle_tests.rs | 310 | はい |
| src/report/domain.rs | 188 | はい |
| src/report/domain_tests.rs | 396 | はい（指摘なし） |
| src/report/summary.rs | 257 | はい |
| src/report/summary_tests.rs | 388 | はい（指摘なし） |
| src/report/tally.rs | 253 | はい（指摘なし） |
| src/report/tally_tests.rs | 410 | はい |

Verdict: 振る舞いの不具合は見つからなかった（必ず直すは 0 件）。残っているのは、決して赤にならないテスト（G3-1・G3-4・G3-5）、2 つの報告に写した組み立て（G3-2）、古い・言い過ぎの注記（G3-6〜G3-11）。
Lean: G3-1・G3-4・G3-5 で約 −160 行、G3-2 で約 −20 行。依存は −0。

確かめていないもの: テストは走らせていない。外のファイルは、指摘を確かめるのに要る所だけ読んだ（`catalog/read.rs`・`catalog/build.rs`・`ledger/read.rs`・`model.rs`・`io/files.rs`・`lib_test_support.rs`・`cli/inspect.rs`・`tests/consistency/values_md.rs`、注記が名指す実物のソース `areka-placement`・`areka-runtime`・`areka-parsers` の該当行）。字句の読み手の端の場合（`"…\` の行継ぎ・`'\u{…}'`・多バイトの文字リテラル）は追ったが、語彙表の要素名に効く実例は見つからず、指摘にしていない。
