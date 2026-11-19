# かけ直し 5 回目・G3

範囲は `crates/ukadoc-survey/` の `src/check/`・`src/evidence/`・`src/report/` の全ファイル（28 本・8,551 行）。ここは純粋な層で、カタログ・台帳・証拠・報告本文を値として受け取り、食い違いの一覧・証拠の索引・手掛かり候補・報告本文を返します（ファイルには触れません）。負荷は、開発者 1 人が副手続きや整合テストを回すだけと見ました。

読み手としてはファイルを 1 つも変えていません（この記録だけを書いた）。状態を変える git の操作もしていません。1〜4 回目（`round1-G3.md`〜`round4-G3.md`）の指摘は蒸し返していません。4 回目の直し（`2d1e2a247`。G3 の範囲では `finding_tests.rs` のテスト 1 本の削除・`test_support.rs` 1 行目・`domain.rs` 147-150 行の注記）を差分で読み、どれも新しい問題を残していないことを確かめました（消したテストの壊れ方は `the_kind_blocks_come_out_in_the_declared_order` と 80-130 行の 2 本が持つまま。`domain.rs` の直した文は `summary.rs` の `NO_LEDGER` と合っている）。

下の 4 件はどれも完了 spec toolkit（`2350e68a9`）の頃からあるもので、1〜4 回目の直しで入ったものではありません。G3-1 だけが報告の本文（道具の出力）に関わり、残り 3 件はテストと注記です。

## 指摘

| id | 重さ | 種別 | 場所 | 何が悪いか | 直し方 | 道具の振る舞いに触るか | 残す種類の候補 |
|---|---|---|---|---|---|---|---|
| G3-1 | 直したほうがよい | [prod] | `src/report/domain.rs:92-94,140-161`（見出し「ドメイン内で関連が閉じている束」）・`src/report/summary.rs:296-299`（「ドメインの中で閉じている束は、それぞれのドメインの報告に載ります。ここに並ぶのは跨いだものだけです。」） | ドメイン別報告は自分の台帳の `links` だけで束を作るので、**他の台帳からこの台帳の id へ入ってくる関連**が見えない。そのため「閉じている」と書いた束が、全体報告では別の束 id の跨ぐ束の一部として載る。実データで 8 束がそうなっている（例: `shiori.md` は `ukadoc:list_plugin_event:OnInstallComplete:1` を束 id として {`list_plugin_event:OnInstallComplete:1`, `list_shiori_event:OnInstallComplete:1`} を「閉じている」と載せるが、`assets.toml:8212` と `sakura-script.toml:1392,1412` がこの id へ `links` を張っており、`summary.md` では同じ 2 つが束 id `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` の跨ぐ束に入っている。ほかに `sakura-script.md` 4 束・`shiori.md` 3 束）。見出しと全体報告の 1 文が事実と違い、同じ id が 2 つの報告で別の束 id の下に出る | 設計 D-11（ドメイン別報告は台帳 1 本だけの関数）を保つ限り入ってくる関連は見えないので、直すのは文の側。⒜ 出力を変えない直し: `domain.rs`・`bundle.rs:17-19`・`summary.rs` の注記と `doc/ukadoc-coverage/README.md:254,503` に「他の台帳から入る関連は見えず、全体報告ではより大きな跨ぐ束の一部になることがある」と書く。⒝ 出力を直す: 見出しを「この台帳の関連だけで繋がった束」などに、全体報告の 1 文を「ドメイン別報告の束が、ここでより大きな束の一部として現れることがある」に改め、報告 5 本を作り直し、`tests/consistency/documents_checks.rs:593,642` の見出しの綴りを追随させる（⒝ は下の ruling-conflicts を参照） | はい（⒝ は報告 5 本の本文が変わる。⒜ は変わらない） | なし（⑶ の SSP 互換や正典の読み方には当たらない。ただし ⒝ は確定 1 の線を越える） |
| G3-2 | できれば | [test/doc] | `src/report/bundle_tests.rs:140-141` | 「`bundle.rs` の「関連の向きも見ない」という主張はこの 1 本が背負う」は事実と違う。同じファイルの 57-58 行は、向きを見ないことを `two_links_converging_on_the_same_id_merge` と `two_links_leaving_the_same_id_merge` の 2 本が受け持つと書いている。実際、書かれた向きに沿って辿る実装はこのテスト（収束形）で割れ、逆向きに辿る実装は発散形のテストでだけ割れるので、1 本では向きの契約を両側から守れない | 「…という主張は、この 1 本と `two_links_leaving_the_same_id_merge` の 2 本が背負う」に直す | いいえ | なし |
| G3-3 | できれば | [test/doc] | `src/evidence/resolve_tests.rs:322-346`（`commas_and_brackets_inside_a_string_do_not_split`） | 名前は「文字列の中のコンマと角括弧で割らない」と言うが、見本の文字列（`"\\![get]"`・`"選択肢"`）にコンマは無い。文字列の中のコンマを守るテストはこのファイルに 1 本も無い（コンマはコメントの中（444 行）と文字リテラルの中（810 行）にしか無い）。名前が言う守りの半分が無い | 名前を `brackets_inside_a_string_do_not_end_the_table` に変える（テストの中身はそのまま） | いいえ | なし |
| G3-4 | できれば | [test/doc] | `src/evidence/candidates.rs:280`（`has_marker_above` の空行・属性を飛ばす分岐）・`303`（`cfg_test_regions` の空行・属性・コメントを飛ばす分岐） | どちらの分岐にも、壊すと赤になるテストが無い。`candidates_tests.rs` の見本はどれも目印の行の直下に表があり、`#[cfg(test)]` の直下に `mod` がある。例えば 280 行の `|| trimmed.is_empty()` を消すと、`/// ukadoc: <URL>`・空行・表、と並んだ表が候補に出るようになるが、全テストは緑のまま。3 回目の G3-6 で注記をこの振る舞いに合わせたが、振る舞いそのものは釘付けされていない | `candidates_tests.rs` に 1 本足す: 目印の行と表の間に空行と `#[rustfmt::skip]` を挟んだ表は候補にならず、`#[cfg(test)]` と `mod tests {` の間に空行とコメントを挟んだ中の表も候補にならないこと（肯定の側に目印の無い表を 1 つ置く） | いいえ | なし |

## 確かめて指摘にしなかったもの

- G3-1 の 8 束は、報告 5 本の束の表を読み、ドメイン別報告の束の構成 id が全体報告の束の構成 id と重なるものを数えた（`report/*.md` を python で照合）。台帳を変えたわけではない。外へ伸びる関連を持つ束は `closed_bundles` が正しく落としており、入ってくる関連だけが漏れる。
- `bundle.rs:17-19`・`domain.rs:143`（「他ドメインへ伸びた束は丸ごと落とす」）は、外へ伸びる側について正しい。嘘ではないので G3-1 とは別の指摘にしない（G3-1 の ⒜ で書き足す）。
- `tally_tests.rs:164-166` の「`spec_shiori3` は既にあるので、`spec_shiori` のページにアンカー付きの項目が入ればその日に生じる」は仮定の話として正しい（今のカタログには `spec_shiori` のページが無く、ページ名が別のページ名の先頭に重なり次の文字が `:` より小さい組も 0 組。python で数えた）。
- `content_link_tests.rs:565-566`・`content_tests.rs:289-291` の「違反 2 件の間に正しい項目が挟まる」は、`Domain::ALL`（shiori・assets・sakura-script・property）と台帳の id 順で追って正しかった。
- `structure_tests.rs:470-474` の「`<` から `<=` へ緩める」は、実装の `pair[1] > pair[0]`（前 < 後）を前から読んだ形として正しい。
- `freshness_tests.rs` の逆順のテストは、全体報告が台帳の並びに依らないことも同時に守っている（依れば逆順の側だけ `summary.md` の所見が出て赤になる）。
- `resolve.rs:13-14` の「同じ URL を持つ 2 項目は `catalog::read` と `catalog::build` がともに拒む」は `read.rs:178`・`build.rs:81` で確かめた。`resolve.rs:70` の「`check`・`evidence` の副手続き」は `cli/inspect.rs:368` の 1 か所から両方が使う。
- `lex.rs` の生文字列・文字リテラル・ブロックコメントの読み飛ばしと、`read_string` の逃がしの戻し（`\0` も含めて UTF-8 を壊さない）は、具体的な入力で追い、正しかった。
- `resolve_tests.rs:593` の「設計 D-5 規則 2」は、規則 2（1 件に定まったときだけ証拠）が「定まらないものは証拠にしない」をそのまま含むので間違いではない。

## ruling-conflicts

- G3-1 の ⒝（報告の見出しと全体報告の 1 文を直す）は、要件の討議で確定したこと 1「守る線は今の入力から書き出すファイルの中身を 1 バイトも変えない（要件 6.4）」を越える（`shiori.md`・`sakura-script.md`・`assets.md`・`property.md`・`summary.md` の本文が変わる）。設計 D-11 そのものは覆さない（ドメイン別報告は台帳 1 本だけの関数のまま）。⒜（注記と README だけ）は線の内側で直せる。どちらにするかは開発者の裁定が要る。推奨は ⒜ を今入れ、⒝ を議題にすること。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/check/mod.rs | 79 | はい（指摘なし） |
| src/check/finding.rs | 201 | はい（指摘なし） |
| src/check/finding_tests.rs | 184 | はい（指摘なし） |
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
| src/evidence/extract_tests.rs | 313 | はい（指摘なし） |
| src/evidence/lex.rs | 266 | はい（指摘なし） |
| src/evidence/resolve.rs | 309 | はい（指摘なし） |
| src/evidence/resolve_tests.rs | 833 | はい（G3-3） |
| src/evidence/candidates.rs | 385 | はい（G3-4） |
| src/evidence/candidates_tests.rs | 448 | はい（G3-4 の足し先） |
| src/report/mod.rs | 8 | はい（指摘なし） |
| src/report/bundle.rs | 150 | はい（G3-1 の ⒜ の書き足し先） |
| src/report/bundle_tests.rs | 271 | はい（G3-2） |
| src/report/domain.rs | 165 | はい（G3-1） |
| src/report/domain_tests.rs | 396 | はい（指摘なし） |
| src/report/summary.rs | 231 | はい（G3-1） |
| src/report/summary_tests.rs | 388 | はい（指摘なし） |
| src/report/tally.rs | 276 | はい（指摘なし） |
| src/report/tally_tests.rs | 381 | はい（指摘なし） |

Verdict: 必ず直すは 0 件。直したほうがよいが 1 件（G3-1・[prod]。報告の「閉じている束」が実データの 8 束で事実と違う。出力を直す側は確定 1 の線を越えるので裁定が要る）。残る 3 件は「できれば」・[test/doc]（事実と違う注記 1・名前が言う守りの半分が無いテスト 1・テストの無い分岐 1）。
Lean: 増減はほぼ 0（G3-4 でテスト 1 本、約 +25 行）。依存は −0。

確かめていないもの: テストは走らせていない。外のファイルは、指摘を確かめるのに要る所だけ読んだ（`doc/ukadoc-coverage/report/*.md` の束の表・`ledger/*.toml` の該当行・`catalog.toml` のページ名・`README.md:248-262,498-510`・`linkage.md` の該当行・`tests/consistency/documents_checks.rs:30-60,620-660`・`cli/inspect.rs`・`cli/inspect_tests.rs:270-320`・`model.rs` の `ALL`・`catalog/read.rs`・`catalog/build.rs` の該当行・toolkit の requirements.md と design.md の D-5・D-11）。
