# かけ直し 4 回目・G4

範囲は tests/ の下の 16 ファイル・7,412 行。全部の行を読んだ。指摘 4 件（必ず直す 0・直したほうがよい 0・できれば 4）。どれも [test/doc] で、道具の振る舞い（副手続きの出力・整合テストの判定）は変えない。どれも残す 3 種類に当たらない（開発者の裁定でも、正典を追う口でも、ukadoc の読み方でもない）。ruling-conflicts 無し。

3 回目の直し（bfbb8b010・99b9b2639）の tests/ の差分（briefing_arms.rs:47・documents_non_vacuity.rs:38-40・242-243・322-323・examples.rs:77・spec_checks.rs:21）も読んだ。直しが生んだ新しい食い違いは無い。

| id | 重さ | 種別 | file:line | 何が悪いか | 直し | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G4-1 | できれば | [test/doc] | tests/consistency/briefing_checks.rs:275-276 | 「段階 A の先頭の行に印を付ける。対象行から外れるので順位の主張は動かないが」は誤り。実データの段階 A の rank 1 は 1 行だけ（briefing.md:376-378）なので、それを対象行から外すと次の行（rank 2）が先頭になり、`dense_order_findings` が「段階 A の先頭の [[rank]] A2 の rank が 1 でない」を出す。テストは 289 行の `contains("insufficient")` で絞るので通るが、注記はその所見が出ないと言い切っている | 「対象行から外れるので先頭の rank の所見も出るが、ここでは insufficient の所見だけを絞って見る。破れるのは『対象行より後』である」の形に直す | いいえ | none |
| G4-2 | できれば | [test/doc] | tests/consistency/documents_checks.rs:403-404 | 「写しの由来の id（`machine` 欄）を 1 文字変える」と書くが、的は `linkage_bundle_ids`（`machine` 欄と表の「…束 id」列の和・259-268 行）から `a_twistable_id` が文字順で選ぶので、表の列にだけ現れる id が選ばれることもある（linkage.md:96 と 4712 に束 id の列がある） | 「写しが引用した機械の束 id（`machine` 欄か表の束 id の列）を 1 文字変える」に直す | いいえ | none |
| G4-3 | できれば | [test/doc] | tests/consistency/documents_checks.rs:348 | 失敗の見出し「3 文書の引用 id が台帳の正典と食い違う」は、突き合わせ相手がカタログ（`catalog.entries`）で台帳ではない。しかも同じ所見の束には裸の id と古い報告名も入る | 「3 文書の引用がカタログ・書き方の決まりと食い違う」の形に直す（所見の各行は既に catalog.toml を名指している） | いいえ | none |
| G4-4 | できれば | [test/doc] | tests/consistency/examples.rs:47-50 | 「**`/kiro-complete` の書き換え対象**である。spec を `completed/` へ移す完了処理は…この綴りを書き換える」は、toolkit spec が完了して綴りが既に `.kiro/specs/completed/…` になった今では起こらない先の話として読める（もう移されない） | 「完了処理（`/kiro-complete`）が `completed/` の綴りへ書き換えた定数である。綴りを 1 つに集めてあるのは、散らすと書き換えの取り残しが…」の過去形に直す | いいえ | none |

確かめて指摘にしなかったもの（数えない）: briefing_arms.rs:46 の「`breakage` は全束とも『黙って壊れる』」は linkage.md の 67 行すべてと一致／values_md.rs:48-52 の「ASCII の角括弧を含む注記は `[変更不可]` の 1 行だけ・ほかの注記に引用符・コンマ・括弧は無い」は shiori_resource.rs のスライス内の注記を数えて一致／examples.rs:59・66 の 4 件・3 件は囲みの中の id を数えて一致／checks.rs:682 の「7 つ」は `Status` の 7 変種／checks.rs:497 の `closed_bundles`・values_md.rs:470 の `match_vocabulary`・:23 の `theme_names`・documents_checks.rs:57 の `push_bundle_list`（domain.rs・summary.rs の両方）・cli_streams.rs:118 の `render_candidates` はどれも実在／non_vacuity.rs:87 の `SELF_CRATE_DIR` は非公開の const として実在／mod.rs:18 の `io::snapshot` は `pub(crate)`／documents_non_vacuity.rs:250-256 の `live ^ completed` は `areka-P0-ukadoc-coverage-roadmap` が今 completed/ にだけ居るので成り立つ／「設計 D-6」が mod.rs（toolkit の design.md:227 の改行コード）と spec_checks.rs（coverage-roadmap の design.md:679 の判定 ⑸）で別の設計の別の決めを指すのは、どちらの注記もその文脈の設計として正しいので指摘にしない／spec_checks.rs:345・478・507 の「1 文字変える」は末尾に `X` を足す形だが、綴りを 1 文字ぶん変える摂動として意味は通るので指摘にしない。

ruling-conflicts: 無し。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| tests/consistency.rs | 13 | はい（全行） |
| tests/cli_streams.rs | 226 | はい（全行） |
| tests/consistency/mod.rs | 205 | はい（全行） |
| tests/consistency/perturb.rs | 243 | はい（全行） |
| tests/consistency/checks.rs | 802 | はい（全行） |
| tests/consistency/documents.rs | 440 | はい（全行） |
| tests/consistency/documents_tools.rs | 313 | はい（全行） |
| tests/consistency/documents_non_vacuity.rs | 403 | はい（全行） |
| tests/consistency/documents_checks.rs | 705 | はい（全行） |
| tests/consistency/spec_checks.rs | 608 | はい（全行） |
| tests/consistency/briefing_arms.rs | 748 | はい（全行） |
| tests/consistency/briefing_checks.rs | 544 | はい（全行） |
| tests/consistency/linkage_checks.rs | 822 | はい（全行） |
| tests/consistency/examples.rs | 327 | はい（全行） |
| tests/consistency/non_vacuity.rs | 417 | はい（全行） |
| tests/consistency/values_md.rs | 596 | はい（全行） |
