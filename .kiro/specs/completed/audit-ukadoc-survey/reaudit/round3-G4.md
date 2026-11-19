# かけ直し 3 回目・G4

範囲は tests/ の下の 16 ファイル・7,410 行。全部の行を読んだ。指摘 5 件（必ず直す 0・直したほうがよい 0・できれば 5）。どれも [test/doc] で、道具の振る舞い（副手続きの出力・整合テストの判定）は変えない。どれも残す 3 種類に当たらない（開発者の裁定でも、正典を追う口でも、ukadoc の読み方でもない）。ruling-conflicts 無し。

2 回目の直し（9b697aeca・78edf300f・46d7cdb90・b042fbf2a・3306c242d）の差分も読んだ。直しが生んだものは G4-2 の 1 件（2 回目 G4-9 の直しが同じ関数の 3 つ目の手書きの綴りを残した）。ほかの直し（`machine_bundle_members` の移し・`values_md.rs` の 2 段の鎖・`perturb.rs` の兄弟の挙げ方・`non_vacuity.rs` の assert の削除・`[[barrier]]` の言い換え）は新しい食い違いを残していない。

| id | 重さ | 種別 | file:line | 何が悪いか | 直し | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G4-1 | できれば | [test/doc] | tests/consistency/documents_non_vacuity.rs:319-321 | 「順位表が空の `briefing.md` は、腕 a〜g のうち a・b・d・e・g が母数 0 でそろって緑」は誤り。`[[rank]]` が空だと腕 a は帰属の束を全部「`[[rank]]` に無い」と挙げて赤（`rank_coverage_findings` は `times` を帰属の束名で 0 から始める）、腕 g は「段階 B に rank 1 の行が無い」で赤（`stage_rule_findings` の `head.is_empty()` の枝）。腕 b も `items` は台帳から数え直すので宣言しだいで赤。母数 0 で緑になるのは d・e だけ | 「順位表が空の `briefing.md` では腕 d・e が**母数 0 で緑**になり」に直す（`[[barrier]]` の後半はそのまま） | いいえ | none |
| G4-2 | できれば | [test/doc] | tests/consistency/documents_non_vacuity.rs:240 | 2 回目 G4-9 の直しで同じ関数の `SPECS_DIR`・`BRIEF_FILE` は定数に替えたが、`!documents.spec_dirs.contains("completed")` の `"completed"` だけ手書きが残る（`documents.rs:29` の `COMPLETED_DIR` と同じ綴り。片方だけ直ると除外の確かめが別の名前を見る） | `COMPLETED_DIR` を `use super::documents::{…}` に足してここで使う（`pub(super)` は既に付いている） | いいえ | none |
| G4-3 | できれば | [test/doc] | tests/consistency/spec_checks.rs:21 | 「（W13「共有ファイル 0」）」は完了した `areka-P0-ukadoc-coverage-roadmap` の設計（design.md:487）にある古いウェーブの決まりの写しで、ここからは読めない。しかもこの監査 spec の design.md では W13 は別の組（差分）を指す | 括弧書きを消す（前の文が理由を言い切っている） | いいえ | none |
| G4-4 | できれば | [test/doc] | tests/consistency/briefing_arms.rs:47 | 「実質の順序は テーマ数 → 資産 → 共有度」に余分な半角空白（2 回目 G4-13 と同じ形） | 「実質の順序はテーマ数 → 資産 → 共有度」に直す | いいえ | none |
| G4-5 | できれば | [test/doc] | tests/consistency/examples.rs:77 | 「3 文書の判定も同じ取り出しを要るので」が文として崩れている | 「同じ取り出しが要るので」に直す | いいえ | none |

確かめて指摘にしなかったもの（数えない）: checks.rs:441-445 の「調査 spec が `areka-kanade/src/schedule/events.rs` へ 1 行置いた」は今も 129 行にある／perturb.rs:42-45 の錨の条件（implemented・`links = []`・`introduced = ""`）は shiori.toml の今の行どおり／checks.rs:649-650 の「8 つの綴り」は `THEMES: [&str; 8]`／values_md.rs:222 の「1 行最大 579 文字」は今の catalog.toml の最長行と一致／documents_non_vacuity.rs:79 の 1,550 は linkage.md の `target = 1550`／documents_non_vacuity.rs:295 の「a・c・e が母数 0 で緑」は正しい（b・f・g は空の帰属を赤にする）／briefing_checks.rs:536-538 の混ぜる相手の選び方は `linkage.md` の `themes` の数で見るが、腕 e（linkage_checks.rs の `derived_findings`）が `themes` を導出と一致させているので実データの上では導出の鍵と同じ／briefing_checks.rs の override と singles の較正は実データの行（override 2 行・singles 行あり）を写しの土台に使うが、作るのは写しの上の形なので briefing_arms.rs:38-42 の注記と食い違わない／documents_tools.rs:5-7 の「較正の側をこちらへ移した」は来歴だが割った理由の説明として正しい。

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
| tests/consistency/documents_non_vacuity.rs | 401 | はい（全行） |
| tests/consistency/documents_checks.rs | 705 | はい（全行） |
| tests/consistency/spec_checks.rs | 608 | はい（全行） |
| tests/consistency/briefing_arms.rs | 748 | はい（全行） |
| tests/consistency/briefing_checks.rs | 544 | はい（全行） |
| tests/consistency/linkage_checks.rs | 822 | はい（全行） |
| tests/consistency/examples.rs | 327 | はい（全行） |
| tests/consistency/non_vacuity.rs | 417 | はい（全行） |
| tests/consistency/values_md.rs | 596 | はい（全行） |
