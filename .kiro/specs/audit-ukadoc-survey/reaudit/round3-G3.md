# かけ直し 3 回目・G3

範囲は `crates/ukadoc-survey/` の `src/check/`・`src/evidence/`・`src/report/` の全ファイル（28 本・8,577 行）。ここは純粋な層で、カタログ・台帳・証拠・報告本文を値として受け取り、食い違いの一覧・証拠の索引・手掛かり候補・報告本文を返します（ファイルには触れません）。負荷は、開発者 1 人が副手続きや整合テストを回すだけと見ました。

読み手としてはファイルを 1 つも変えていません（この記録だけを書いた）。状態を変える git の操作もしていません。1 回目（`round1-G3.md`）と 2 回目（`round2-G3.md`）の指摘が直っていることを確かめ、蒸し返していません。2 回目の直し（`9b697aeca`・`46d7cdb90`）が新しい問題を残した所は見つかりませんでした。`tally.rs` の `push_overall_table`・`push_theme_table`、`bundle.rs` の `all_links` への寄せ、逐語のテストに含まれる主張の削除はどれも筋が通っています。

下の 7 件はどれも完了 spec toolkit（`2350e68a9`）の頃からある注記で、1・2 回目の直しで入ったものではありません（`git log -S` で確かめた）。7 件とも、事実と違う文か、壊れ方の説明が実装と合わない文です。

## 指摘

| id | 重さ | 種別 | 場所 | 何が悪いか | 直し方 | 道具の振る舞いに触るか | 残す種類の候補 |
|---|---|---|---|---|---|---|---|
| G3-1 | できれば | [test/doc] | `src/evidence/resolve_tests.rs:225` | 注記は「2 番目の文字列（`log_path`）は拾われてはならない」と言うが、見本の `("log_path", Kind::B)` では `log_path` は 2 つ目の要素の**最初の**文字列。テストは 241-244 行で `log_path` が証拠になると主張していて、注記と逆になっている。見本の要素には 2 番目の文字列がそもそも無い | 「各要素の最初の文字列（`version`・`log_path`）を拾う。2 番目の欄は文字列でない（実物の `SetSemantics` の形）」に直す。2 番目の文字列を拾わないことは struct の見本（248 行）が持つ | いいえ | なし |
| G3-2 | できれば | [test/doc] | `src/evidence/resolve_tests.rs:301-302` | 「割ると `Kind::B` 側の断片が新しい要素になり、`log_path` が拾われてしまう」と言うが、この見本に `Kind::B` は無い（上のテストから写した名残）。`log_path` を持つのは `f(1, "log_path")` の断片 | 「割ると `f(1, "log_path")` の後ろの断片が新しい要素になり、`log_path` が拾われてしまう」に直す | いいえ | なし |
| G3-3 | できれば | [test/doc] | `src/report/bundle_tests.rs:53-55` | 「残る 2 対は同じ向きのまま——成分 A の鎖の接ぎ方（後の対が前に接ぐ）を保つためである」は嘘。`shuffled` は `(ALPHA, BRAVO)` を `(BRAVO, CHARLIE)` より先に置くので、後の対は鎖の**後ろ**に接ぐ（`scrambled` とは接ぎ方が逆）。向きは保っているが、並びを入れ替えたので接ぎ方は保てていない | 理由の句を消し、「後の対がどちらの端に接いでも割れないことは `a_chain_merges_no_matter_which_end_the_later_edge_hangs_from` が受け持つ」と書く | いいえ | なし |
| G3-4 | できれば | [test/doc] | `src/report/tally_tests.rs:60-61,267` | 「`世代不明` が最初に現れる」（61 行）、「本文の順でも id の順でも `世代不明` が最初に現れる」（267 行）は、本文の順では嘘。`sample()` が渡す順（＝`file_order`）の先頭は `alpha`（`2.3.53`→`2.3`）で、`世代不明` は 3 件目。そのうえ 37-38 行が言うとおり `tally` は `file_order` を読まない | 「id の順では `世代不明`（`page_one:charlie`）が最初に現れるが、名前順では最後」に直し、「本文の順でも」を消す | いいえ | なし |
| G3-5 | できれば | [test/doc] | `src/report/tally_tests.rs:337-338` | 「前方一致で拾うと 気配り が汚れる」と言うが、`tally` はテーマの欄を `entry.values` の綴りから作るだけで、テーマ名の一覧とは何も突き合わせない（`tally.rs:166-168`）。前方一致が入り込む道筋が無いので、この assert は書かれた壊れ方では赤にならない。接頭辞の守りは `model::parse_theme` にあり、`model_tests::theme_outside_the_vocabulary_is_rejected` が持つ（`content_link_tests.rs:423-424` もそう書いている） | 2 行を消す | いいえ | なし |
| G3-6 | できれば | [test/doc] | `src/evidence/candidates.rs:39-40`（`has_marker_above` は 269-286） | 「直上の**連続した**コメントの塊に `ukadoc: <URL>` の行があれば候補にしない」と言うが、`has_marker_above` は空行も飛ばして遡る（280 行）。例：`// ukadoc: …#OnBoot:1`、空行、`pub const EVENTS: &[&str] = &["OnClose"];` と並ぶと、間に空行があっても `OnClose` は候補から消える。空行を飛ばすのは `resolve::find_slice_start`（目印の後ろの空行を越えて表を探す）と揃っているので、直すのは注記の側 | モジュールの注記と `has_marker_above` の doc を「空行と属性（`#[…]`）を飛ばして遡った直上のコメントの塊」に直す | いいえ（注記だけ） | なし |
| G3-7 | できれば | [test/doc] | `src/evidence/extract_tests.rs:86-87` | 「「`//` を剥がして残りを見る」に変えると、`////` の残りが … `ukadoc: ...` に見えてしまう」は、名指した実装と合わない。`//` を 1 回剥がすと `////` の残りは `// ukadoc: ...` で、`ukadoc:` では始まらない。`////` を通してしまうのは、先頭の `/` を**全部**剥がす形（`trim_start_matches('/')`）である | 「先頭の `/` を全部剥がして残りを見る形に変えると、`////` の残りが `ukadoc: ...` になって通ってしまう」に直す | いいえ | なし |

## 確かめて指摘にしなかったもの

- `content_link_tests.rs:13-15` の「`AliasChain` だけが…並びが結果に漏れうる唯一の判定」は、文脈から `content` の判定の中の話と読める。並べ直しで守る `structure::owning_domains` には、別に逆順のテスト（`structure_tests.rs:703`）がある。
- `summary_tests.rs:273` の「要件 7.3・11.2」は 2 つの spec の番号が並んでいるが、どちらも中身と合っている（7.3 は toolkit、11.2 は統合調査）。間違いではないので指摘にしない。
- `tally.rs:163` の「8 テーマに限っている（要件 6.10）」。toolkit の要件 6.10 には「テーマ名の綴りが違う」ときに赤にすることが入っているので、間違いではない。
- `candidates.rs:156`（`HOLD_FIELDS`）・`171`（`try_register("move", …)`）・`197-198`（`seriko.dpi`・`seriko.zorder`）と `extract_tests.rs:40-42` が名指す実物は、どれも今の木にある（`areka-placement/src/transition_diag.rs:245`・`areka-runtime/src/consumer_ledger.rs:410`・`areka-placement/src/source.rs:46`・`config.rs:139`・`source_tests.rs:291`・`areka-emo-compose/src/fold.rs:434`・`areka-parsers/src/sakura/decode_tests.rs:451`）。
- `freshness.rs:25` の「この repo に `.gitattributes` は無い」。ルートと `doc/` には無い（あるのは `crates/shiori4-testdll/`・`tools/perf/fixtures/`・`vendors/sample_ghost/` の 3 つだけで、報告には効かない）。
- `lex.rs` のバイト位置の進め方、`candidates.rs` の `line[from..]` の切り出し（どれも ASCII の直後で切るので文字の途中にならない）、`bundle.rs` の union-find（5 構成 id の鎖で深さ 4）、`summary.rs` の `crossing_bundles`（台帳の無い id を含む束は 2 ドメイン扱いで載る）、`structure.rs` の前置きの重複の数え方は、具体的な入力で追い、正しかった。

## ruling-conflicts

- なし。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/check/mod.rs | 79 | はい（指摘なし） |
| src/check/finding.rs | 201 | はい（指摘なし） |
| src/check/finding_tests.rs | 212 | はい（指摘なし） |
| src/check/content.rs | 252 | はい（指摘なし） |
| src/check/content_tests.rs | 354 | はい（指摘なし） |
| src/check/content_link_tests.rs | 653 | はい（指摘なし） |
| src/check/freshness.rs | 129 | はい（指摘なし） |
| src/check/freshness_tests.rs | 569 | はい（指摘なし） |
| src/check/structure.rs | 233 | はい（指摘なし） |
| src/check/structure_tests.rs | 738 | はい（指摘なし） |
| src/check/test_support.rs | 118 | はい（指摘なし） |
| src/evidence/mod.rs | 128 | はい（指摘なし） |
| src/evidence/extract.rs | 93 | はい（指摘なし） |
| src/evidence/extract_tests.rs | 313 | はい |
| src/evidence/lex.rs | 266 | はい（指摘なし） |
| src/evidence/resolve.rs | 309 | はい（指摘なし） |
| src/evidence/resolve_tests.rs | 832 | はい |
| src/evidence/candidates.rs | 385 | はい |
| src/evidence/candidates_tests.rs | 448 | はい（指摘なし） |
| src/report/mod.rs | 8 | はい（指摘なし） |
| src/report/bundle.rs | 150 | はい（指摘なし） |
| src/report/bundle_tests.rs | 270 | はい |
| src/report/domain.rs | 164 | はい（指摘なし） |
| src/report/domain_tests.rs | 396 | はい（指摘なし） |
| src/report/summary.rs | 231 | はい（指摘なし） |
| src/report/summary_tests.rs | 388 | はい（指摘なし） |
| src/report/tally.rs | 276 | はい（指摘なし） |
| src/report/tally_tests.rs | 382 | はい |

Verdict: 振る舞いの不具合は見つからなかった。必ず直すは 0 件、直したほうがよいも 0 件。残りは、事実と違う注記や、書かれた壊れ方では赤にならない assert の 7 件（すべて「できれば」・[test/doc]）。
Lean: G3-5 で −2 行。ほかは注記の書き直しだけ。依存は −0。

確かめていないもの: テストは走らせていない（見本の数を主張する assert は CI が守っている前提）。外のファイルは、指摘を確かめるのに要る所だけ読んだ（`catalog/read.rs`・`model.rs`・`io/files.rs`・`io/sources.rs`・`io/paths.rs`・toolkit と統合調査の requirements.md の要件 6・11、注記が名指す実物のソースの該当行）。
