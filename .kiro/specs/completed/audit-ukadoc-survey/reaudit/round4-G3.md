# かけ直し 4 回目・G3

範囲は `crates/ukadoc-survey/` の `src/check/`・`src/evidence/`・`src/report/` の全ファイル（28 本・8,578 行）。ここは純粋な層で、カタログ・台帳・証拠・報告本文を値として受け取り、食い違いの一覧・証拠の索引・手掛かり候補・報告本文を返します（ファイルには触れません）。負荷は、開発者 1 人が副手続きや整合テストを回すだけと見ました。

読み手としてはファイルを 1 つも変えていません（この記録だけを書いた）。状態を変える git の操作もしていません。1〜3 回目（`round1-G3.md`・`round2-G3.md`・`round3-G3.md`）の指摘は蒸し返していません。3 回目の直し（`bfbb8b010`・`99b9b2639`。G3 の範囲では `candidates.rs`・`extract_tests.rs`・`resolve_tests.rs`・`bundle_tests.rs`・`tally_tests.rs` の 5 本）を差分で読み、どれも新しい問題を残していないことを確かめました（`tally_tests.rs` の「id の順では `世代不明` が最初」は、見本の id 順の先頭が `page_one:charlie`（空の登場版）なので正しい。`bundle_tests.rs` の 1 歩しか辿らない実装の説明も、union-find を手で追って正しかった）。

下の 3 件はどれも完了 spec toolkit（`2350e68a9`）の頃からある注記・テストで、1〜3 回目の直しで入ったものではありません。

## 指摘

| id | 重さ | 種別 | 場所 | 何が悪いか | 直し方 | 道具の振る舞いに触るか | 残す種類の候補 |
|---|---|---|---|---|---|---|---|
| G3-1 | できれば | [test/doc] | `src/report/domain.rs:147-149` | 「検査が赤の間だけ、まだ台帳に書かれていない同ドメインの id を指す束がどちらの報告からも落ちる」は嘘。全体報告は、どの台帳にも無い id を含む束を `（台帳に無い）` 付きで載せる（`summary.rs:60-66` の `NO_LEDGER`・`crossing_bundles`）。現に `summary_tests.rs` の見本で、`list_shell:charlie:1` → `list_shell:ghost:1`（同じ assets のページで、どの台帳にも無い id）の束が `assets, （台帳に無い）` として全体報告に出ている。落ちるのはドメイン別報告からだけ | 「……同ドメインの id を指す束はドメイン別報告から落ちる（全体報告は `（台帳に無い）` を添えて載せる）」に直す | いいえ（注記だけ） | なし |
| G3-2 | できれば | [test/doc] | `src/check/finding_tests.rs:186-212`（`the_body_does_not_depend_on_the_input_order`） | 本文が入力の並びに依らないと言うが、見本の 3 件はどれも種類が違うので、逆順にして変わりうるのは塊の並びだけ。それは 137-184 行の `the_kind_blocks_come_out_in_the_declared_order`（13 種を逆順に与えて見出しを逐語で比べる）が丸ごと持っている。塊の中の並べ替え（`sort_by`）を消してもこのテストは緑のままで、そちらは 80-130 行の 2 本が持つ。このテストだけが捕まえる壊れ方が無い（言い直しのテスト。2 回目の G3-4 と同じ種類で、その一覧から漏れていた） | テストを消す（約 −27 行） | いいえ | なし |
| G3-3 | できれば | [test/doc] | `src/check/test_support.rs:1` | 「整合検査（`check/`）の 4 判定のテスト」と言うが、`check/mod.rs:10-14` は「判定は 3 つに分かれ」（構造・内容・新しさ）と言い、出力の整形（`finding`）は判定ではない。3 行目が「構造・内容・鮮度・出力整形」と並べているとおり、4 つ目は判定ではなく整形のテスト | 1 行目を「整合検査（`check/`）の 3 判定と出力の整形のテストが共有する道具」に直す | いいえ | なし |

## 確かめて指摘にしなかったもの

- `content_link_tests.rs` の `the_order_of_the_ledgers_does_not_change_the_content_findings` は、順向きの側が `an_alias_chain_across_two_ledgers_is_a_finding` と同じだが、逆向きの側が「後ろの台帳だけを引く」実装を捕まえる（順向きでは相手の assets が shiori の後ろにあるので、その実装は順向きのテストを両方とも通る）。言い直しではない。
- `resolve_tests.rs` の `the_index_does_not_depend_on_the_order_of_the_hits` は逆順の入力で通すテストで、2 回目の G3-1 が消した「同じ入力を 2 回」の種類ではない。
- `bundle_tests.rs` の 1 歩・k 歩の実装の説明（`scrambled`・`a_chain_merges_…`・`a_five_id_chain_…`）は、`unite`／`find` を手で追って、書かれた向きで割れることを確かめた。
- `content_tests.rs`・`structure_tests.rs` の見本の件数（証拠つき id 4・実装済み 3・登場版 6・版番号つきカタログ 7・証拠の無い項目 8・カタログ 12）は `lib_test_support.rs` の見本と照らして合っていた。
- `resolve.rs:274-276` が名指す `SHIORI_RESOURCE_IDS` の較正は `tests/consistency/values_md.rs` にある。`resolve.rs`・`candidates.rs`・`content.rs`・`freshness.rs` が名指す関数（`io::sources::walk`・`io::files::strip_cr`／`read_normalized`／`write_lf`・`Catalog::by_url`／`page_urls`／`titles_of_page`・`PageAssignment::unassigned`・`ledger_file`）はどれも今の木にある。
- `summary.rs:62-65` が `NO_LEDGER` の id を赤にする担当に要件 6.3 も挙げている点（台帳に無い id には 6.3 は効かない）は、6.4・6.7 が正しく並んでおり読み違えを生まないので指摘にしない。
- `lex.rs` の `find_slice_start` は型注記の `[T; N]` の `;` で止まるが、実物の語彙表にその形は無く、具体的な入力で困る場面が無い。

## ruling-conflicts

- なし。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/check/mod.rs | 79 | はい（指摘なし） |
| src/check/finding.rs | 201 | はい（指摘なし） |
| src/check/finding_tests.rs | 212 | はい |
| src/check/content.rs | 252 | はい（指摘なし） |
| src/check/content_tests.rs | 354 | はい（指摘なし） |
| src/check/content_link_tests.rs | 653 | はい（指摘なし） |
| src/check/freshness.rs | 129 | はい（指摘なし） |
| src/check/freshness_tests.rs | 569 | はい（指摘なし） |
| src/check/structure.rs | 233 | はい（指摘なし） |
| src/check/structure_tests.rs | 738 | はい（指摘なし） |
| src/check/test_support.rs | 118 | はい |
| src/evidence/mod.rs | 128 | はい（指摘なし） |
| src/evidence/extract.rs | 93 | はい（指摘なし） |
| src/evidence/extract_tests.rs | 313 | はい（指摘なし） |
| src/evidence/lex.rs | 266 | はい（指摘なし） |
| src/evidence/resolve.rs | 309 | はい（指摘なし） |
| src/evidence/resolve_tests.rs | 833 | はい（指摘なし） |
| src/evidence/candidates.rs | 385 | はい（指摘なし） |
| src/evidence/candidates_tests.rs | 448 | はい（指摘なし） |
| src/report/mod.rs | 8 | はい（指摘なし） |
| src/report/bundle.rs | 150 | はい（指摘なし） |
| src/report/bundle_tests.rs | 271 | はい（指摘なし） |
| src/report/domain.rs | 164 | はい |
| src/report/domain_tests.rs | 396 | はい（指摘なし） |
| src/report/summary.rs | 231 | はい（指摘なし） |
| src/report/summary_tests.rs | 388 | はい（指摘なし） |
| src/report/tally.rs | 276 | はい（指摘なし） |
| src/report/tally_tests.rs | 381 | はい（指摘なし） |

Verdict: 振る舞いの不具合は見つからなかった。必ず直すは 0 件、直したほうがよいも 0 件。残りは事実と違う注記 2 件（G3-1・G3-3）と言い直しのテスト 1 本（G3-2）で、すべて「できれば」・[test/doc]。
Lean: G3-2 で約 −27 行。ほかは注記の書き直しだけ。依存は −0。

確かめていないもの: テストは走らせていない（見本の数を主張する assert は CI が守っている前提）。外のファイルは、指摘を確かめるのに要る所だけ読んだ（`lib_test_support.rs`・`model.rs` の `Domain::ALL`・`io/files.rs`・`io/sources.rs`・`catalog/read.rs`・`assignment.rs`・`ledger/mod.rs` の該当行、`areka-sylphya/src/vocab/shiori_resource.rs`、`tests/consistency/values_md.rs` の該当行）。
